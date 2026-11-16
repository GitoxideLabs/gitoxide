use std::io;

use gix_error::{Result, ResultExt, message};
use windows_sys::Win32::{
    Foundation::{ERROR_SUCCESS, GENERIC_READ, GENERIC_WRITE, LocalFree},
    Security::{
        ACL, AllocateAndInitializeSid,
        Authorization::{
            EXPLICIT_ACCESS_W, NO_MULTIPLE_TRUSTEE, SET_ACCESS, SetEntriesInAclW, TRUSTEE_IS_SID,
            TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
        },
        FreeSid, InitializeSecurityDescriptor, NO_INHERITANCE, PSID, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR,
        SECURITY_WORLD_SID_AUTHORITY, SetSecurityDescriptorDacl,
    },
    System::SystemServices::{SECURITY_DESCRIPTOR_REVISION, SECURITY_WORLD_RID},
};

struct Sid(PSID);

impl Drop for Sid {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: AllocateAndInitializeSid transferred ownership of this allocation.
            unsafe { FreeSid(self.0) };
        }
    }
}

struct Acl(*mut ACL);

impl Drop for Acl {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: SetEntriesInAclW allocates its result with LocalAlloc.
            unsafe { LocalFree(self.0.cast()) };
        }
    }
}

/// Git's first pipe instance grants local Everyone clients generic read/write access, allowing
/// ordinary commands to use a monitor created by an elevated command. Remote clients are rejected
/// separately by the pipe mode. Keep the descriptor and its referenced allocations alive together.
pub(super) struct Security {
    attributes: SECURITY_ATTRIBUTES,
    _descriptor: Box<SECURITY_DESCRIPTOR>,
    _acl: Acl,
    _sid: Sid,
}

impl Security {
    pub(super) fn new() -> Result<Self> {
        let mut sid = Sid(std::ptr::null_mut());
        // SAFETY: The well-known Everyone authority and subauthority are valid; the output is owned.
        if unsafe {
            AllocateAndInitializeSid(
                &SECURITY_WORLD_SID_AUTHORITY,
                1,
                SECURITY_WORLD_RID as u32,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                &mut sid.0,
            )
        } == 0
        {
            return Err(io::Error::last_os_error())
                .or_raise(|| message("allocating the Everyone SID for filesystem monitor IPC"));
        }
        let entry = EXPLICIT_ACCESS_W {
            grfAccessPermissions: GENERIC_READ | GENERIC_WRITE,
            grfAccessMode: SET_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: TRUSTEE_W {
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
                ptstrName: sid.0.cast(),
                ..TRUSTEE_W::default()
            },
        };
        let mut acl = Acl(std::ptr::null_mut());
        // SAFETY: The single entry refers to the live SID; the new ACL's allocation is owned here.
        let status = unsafe { SetEntriesInAclW(1, &entry, std::ptr::null(), &mut acl.0) };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status as i32))
                .or_raise(|| message("granting Everyone read/write access to filesystem monitor IPC"));
        }
        let mut descriptor = Box::new(SECURITY_DESCRIPTOR::default());
        let descriptor_ptr = std::ptr::from_mut(&mut *descriptor).cast();
        // SAFETY: This stable allocation has the size and alignment of an absolute descriptor.
        if unsafe { InitializeSecurityDescriptor(descriptor_ptr, SECURITY_DESCRIPTOR_REVISION) } == 0 {
            return Err(io::Error::last_os_error())
                .or_raise(|| message("initializing filesystem monitor IPC security"));
        }
        // SAFETY: The initialized descriptor and its owned ACL remain alive together.
        if unsafe { SetSecurityDescriptorDacl(descriptor_ptr, 1, acl.0, 0) } == 0 {
            return Err(io::Error::last_os_error())
                .or_raise(|| message("setting the filesystem monitor IPC access policy"));
        }
        Ok(Self {
            attributes: SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor_ptr,
                bInheritHandle: 0,
            },
            _descriptor: descriptor,
            _acl: acl,
            _sid: sid,
        })
    }

    pub(super) fn attributes(&self) -> &SECURITY_ATTRIBUTES {
        &self.attributes
    }
}

#[cfg(feature = "async-tokio")]
pub(in crate::server) fn create_async_pipe(
    name: &std::ffi::OsStr,
) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    use tokio::net::windows::named_pipe::ServerOptions;

    let security = Security::new().map_err(io::Error::other)?;
    let attributes = std::ptr::from_ref(security.attributes());
    // SAFETY: Tokio forwards these live attributes to CreateNamedPipeW, which does not mutate them.
    // The descriptor and its allocations remain owned through creation and are never held across await.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(true)
            .max_instances(1)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(name, attributes.cast_mut().cast())
    }
}

#[cfg(test)]
mod tests;
