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

#[cfg(feature = "async")]
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
mod tests {
    use super::*;
    use windows_sys::Win32::{
        Security::{
            ACCESS_ALLOWED_ACE, GetAce, GetSecurityDescriptorDacl, IsValidSecurityDescriptor, IsWellKnownSid,
            WinWorldSid,
        },
        System::SystemServices::ACCESS_ALLOWED_ACE_TYPE,
    };

    #[test]
    fn descriptor_grants_only_gits_non_inheriting_everyone_read_write_policy() -> gix_error::TestResult {
        let security = Security::new()?;
        let attributes = security.attributes();
        assert_eq!(
            attributes.nLength as usize,
            std::mem::size_of::<SECURITY_ATTRIBUTES>(),
            "Windows receives a complete security-attributes structure"
        );
        assert_eq!(
            attributes.bInheritHandle, 0,
            "pipe handles are not inherited by child processes"
        );
        // SAFETY: The descriptor and all referenced allocations remain owned by security.
        assert_ne!(
            unsafe { IsValidSecurityDescriptor(attributes.lpSecurityDescriptor) },
            0,
            "the absolute descriptor satisfies Windows validation"
        );
        let (mut present, mut defaulted, mut acl) = (0, 0, std::ptr::null_mut());
        // SAFETY: The descriptor is valid and output pointers remain live during this call.
        if unsafe { GetSecurityDescriptorDacl(attributes.lpSecurityDescriptor, &mut present, &mut acl, &mut defaulted) }
            == 0
        {
            return Err(io::Error::last_os_error().into());
        }
        assert_ne!(present, 0, "a missing DACL must not fall back to creator defaults");
        assert_eq!(defaulted, 0, "the DACL is explicitly supplied");
        assert!(!acl.is_null(), "a null DACL must not grant unrestricted access");
        // SAFETY: GetSecurityDescriptorDacl returned a live ACL inside the owned descriptor.
        assert_eq!(
            unsafe { (*acl).AceCount },
            1,
            "Git grants access through one Everyone entry"
        );
        let mut entry = std::ptr::null_mut();
        // SAFETY: The ACL has one entry and the output pointer remains live.
        if unsafe { GetAce(acl, 0, &mut entry) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: This valid ACE contains an inline SID. Derive its address from the native
        // allocation rather than from a Rust field reference covering only its first u32.
        let sid = unsafe { entry.byte_add(std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart)) };
        // SAFETY: SetEntriesInAclW created this validated, sufficiently sized access-allowed ACE.
        let entry = unsafe { &*entry.cast::<ACCESS_ALLOWED_ACE>() };
        assert_eq!(
            entry.Header.AceType as u32, ACCESS_ALLOWED_ACE_TYPE,
            "the entry allows access"
        );
        assert_eq!(entry.Header.AceFlags, 0, "the access grant is not inherited");
        assert_eq!(
            entry.Mask,
            GENERIC_READ | GENERIC_WRITE,
            "the access mask matches Git exactly"
        );
        // SAFETY: The inline SID remains within the live ACL allocation.
        assert_ne!(
            unsafe { IsWellKnownSid(sid, WinWorldSid) },
            0,
            "the trustee is Everyone rather than the creator or an administrator group"
        );
        assert!(
            security._descriptor.Owner.is_null(),
            "Git leaves owner selection to Windows"
        );
        assert!(
            security._descriptor.Group.is_null(),
            "Git leaves group selection to Windows"
        );
        assert!(
            security._descriptor.Sacl.is_null(),
            "Git does not add an integrity or audit policy"
        );
        Ok(())
    }
}
