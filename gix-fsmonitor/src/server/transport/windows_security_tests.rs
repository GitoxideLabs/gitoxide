use std::{
    fs::File,
    io::{self, Read, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use windows_sys::Win32::{
    Foundation::{ERROR_ACCESS_DENIED, INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE,
        Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT},
        CreateRestrictedToken, CreateWellKnownSid, DACL_SECURITY_INFORMATION, GENERIC_MAPPING, GetAce,
        ImpersonateLoggedOnUser, IsWellKnownSid, MapGenericMask, RevertToSelf, SID_AND_ATTRIBUTES, TOKEN_DUPLICATE,
        TOKEN_QUERY, WinWorldSid,
    },
    Storage::FileSystem::{
        CreateFileW, FILE_ALL_ACCESS, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ,
        FILE_GENERIC_WRITE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    },
    System::{
        Pipes::{CreateNamedPipeW, PIPE_NOWAIT, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE},
        SystemServices::ACCESS_ALLOWED_ACE_TYPE,
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

use super::windows::{Listener, Wake};

struct Descriptor(*mut std::ffi::c_void);

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: GetSecurityInfo allocates its descriptor with LocalAlloc.
        unsafe { LocalFree(self.0) };
    }
}

struct Impersonation;

impl Drop for Impersonation {
    fn drop(&mut self) {
        // SAFETY: This guard stays on the private worker thread that began impersonation.
        assert_ne!(
            unsafe { RevertToSelf() },
            0,
            "the test restores its worker's original token"
        );
    }
}

fn with_everyone_restricted_token<T>(access: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    let mut token = std::ptr::null_mut();
    // SAFETY: The process pseudo-handle is valid and the output transfers a new token handle.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY | TOKEN_DUPLICATE, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: OpenProcessToken returned this uniquely owned handle.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut world = [0u32; 17];
    let mut size = std::mem::size_of_val(&world) as u32;
    // SAFETY: This aligned buffer fits SECURITY_MAX_SID_SIZE; the well-known SID needs no domain.
    if unsafe { CreateWellKnownSid(WinWorldSid, std::ptr::null_mut(), world.as_mut_ptr().cast(), &mut size) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let restricting = SID_AND_ATTRIBUTES {
        Sid: world.as_mut_ptr().cast(),
        Attributes: 0,
    };
    let mut restricted = std::ptr::null_mut();
    // SAFETY: Windows copies the live restricting SID into a new token, leaving the process token intact.
    if unsafe {
        CreateRestrictedToken(
            token.as_raw_handle(),
            0,
            0,
            std::ptr::null(),
            0,
            std::ptr::null(),
            1,
            &restricting,
            &mut restricted,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: CreateRestrictedToken returned this uniquely owned handle.
    let restricted = unsafe { OwnedHandle::from_raw_handle(restricted) };
    // SAFETY: This restricted token belongs to the current account and permits impersonation.
    if unsafe { ImpersonateLoggedOnUser(restricted.as_raw_handle()) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let _impersonation = Impersonation;
    access()
}

fn name(path: &Path) -> Vec<u16> {
    super::pipe_name(path.as_os_str().encode_wide().collect())
}

fn open_restricted_client(path: &Path) -> io::Result<File> {
    let path = path.to_owned();
    std::thread::spawn(move || {
        with_everyone_restricted_token(|| {
            let name = name(&path);
            // SAFETY: The terminated name is live; successful opening transfers a new pipe handle.
            let handle = unsafe {
                CreateFileW(
                    name.as_ptr(),
                    FILE_GENERIC_READ | FILE_GENERIC_WRITE,
                    0,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: CreateFileW returned this uniquely owned handle, transferable after impersonation ends.
            Ok(unsafe { File::from_raw_handle(handle) })
        })
    })
    .join()
    .map_err(|_| io::Error::other("restricted client thread panicked"))?
}

fn exchange(path: &Path) -> gix_testtools::TestResult {
    let mut client = open_restricted_client(path)?;
    assert_native_read_write_policy(&client)?;
    client.write_all(b"request")?;
    let mut response = [0; 8];
    client.read_exact(&mut response)?;
    assert_eq!(
        &response, b"response",
        "the restricted client receives the complete server response"
    );
    Ok(())
}

#[test]
fn native_clients_need_gits_everyone_read_write_policy() -> gix_testtools::TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    let control_path = root.join("default-ipc");
    let control_name = name(&control_path);
    // The restricting-SID pass requires permission through Everyone, independently of the user's
    // owner/admin grants. It exposes the default Everyone read-only ACL without an elevated login.
    // SAFETY: The terminated name is valid; null attributes select Windows' default pipe security.
    let control = unsafe {
        CreateNamedPipeW(
            control_name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            1024,
            1024,
            5000,
            std::ptr::null(),
        )
    };
    if control == INVALID_HANDLE_VALUE {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: CreateNamedPipeW returned this uniquely owned handle.
    let control = unsafe { File::from_raw_handle(control) };
    let error = open_restricted_client(&control_path)
        .expect_err("the default Everyone read-only grant cannot open a duplex client");
    assert_eq!(
        error.raw_os_error(),
        Some(ERROR_ACCESS_DENIED as i32),
        "the negative control fails native access checking"
    );
    drop(control);

    let path = root.join("explicit-ipc");
    let worker_path = path.clone();
    let (ready, receive) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || -> io::Result<()> {
        let mut listener = Listener::bind(&worker_path)?;
        ready.send(()).map_err(io::Error::other)?;
        let wake = Wake::new()?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            if let Some(stream) = listener.accept()? {
                break stream;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "the restricted client did not connect",
                ));
            }
            listener.wait(&wake, remaining)?;
        };
        let mut request = [0; 7];
        stream.read_exact(&mut request)?;
        assert_eq!(
            &request, b"request",
            "the restricted client can write to the explicitly secured pipe"
        );
        stream.write_all(b"response")?;
        stream.finish()
    });
    let result = (|| -> gix_testtools::TestResult {
        receive.recv_timeout(Duration::from_secs(5))?;
        exchange(&path)
    })();
    let worker_result = worker.join().map_err(|_| "explicit-security server thread panicked")?;
    result?;
    worker_result?;
    Ok(())
}

#[cfg(feature = "async")]
#[test]
fn native_async_clients_need_gits_everyone_read_write_policy() -> gix_testtools::TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let path = directory.path().canonicalize()?.join("async-ipc");
    let worker_path = path.clone();
    let (ready, receive) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || -> io::Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let pipe_name = crate::client::daemon::pipe_name(&worker_path)?;
        let mut stream = {
            let _entered = runtime.enter();
            super::create_async_pipe(&pipe_name)?
        };
        ready.send(()).map_err(io::Error::other)?;
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), async {
                stream.connect().await?;
                let mut request = [0; 7];
                stream.read_exact(&mut request).await?;
                assert_eq!(
                    &request, b"request",
                    "the restricted client can write to the Tokio pipe"
                );
                stream.write_all(b"response").await?;
                match stream.read(&mut [0]).await {
                    Ok(0) => Ok(()),
                    Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
                    Err(error) => Err(error),
                    Ok(_) => Err(io::Error::other("the client sent unexpected data after its request")),
                }
            })
            .await
            .map_err(io::Error::other)?
        })
    });
    let result = (|| -> gix_testtools::TestResult {
        receive.recv_timeout(Duration::from_secs(5))?;
        exchange(&path)
    })();
    let worker_result = worker
        .join()
        .map_err(|_| "explicit-security Tokio server thread panicked")?;
    result?;
    worker_result?;
    Ok(())
}

fn assert_native_read_write_policy(client: &File) -> gix_testtools::TestResult {
    let mut descriptor = Descriptor(std::ptr::null_mut());
    let mut acl = std::ptr::null_mut();
    // SAFETY: The connected pipe handle permits READ_CONTROL and all outputs remain live.
    let status = unsafe {
        GetSecurityInfo(
            client.as_raw_handle(),
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut acl,
            std::ptr::null_mut(),
            &mut descriptor.0,
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32).into());
    }
    assert!(
        !acl.is_null(),
        "the native pipe has a restrictive DACL rather than a null DACL"
    );
    // SAFETY: The returned descriptor owns a validated ACL until the function returns.
    assert_eq!(
        unsafe { (*acl).AceCount },
        1,
        "the native pipe retains Git's single Everyone grant"
    );
    let mut entry = std::ptr::null_mut();
    // SAFETY: The ACL has one entry and its output pointer remains live.
    if unsafe { GetAce(acl, 0, &mut entry) } == 0 {
        return Err(io::Error::last_os_error().into());
    }
    // SAFETY: The native ACL has a validated ACE, whose header is checked before reading its payload.
    let header = unsafe { &*entry.cast::<windows_sys::Win32::Security::ACE_HEADER>() };
    assert_eq!(
        header.AceType as u32, ACCESS_ALLOWED_ACE_TYPE,
        "the native grant is an access-allowed ACE"
    );
    // SAFETY: This raw allocation contains the entire access-allowed ACE and its variable-length SID.
    let sid = unsafe { entry.byte_add(std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart)) };
    assert_ne!(
        unsafe { IsWellKnownSid(sid, WinWorldSid) },
        0,
        "the native policy grants Everyone rather than relying on the creator token"
    );
    // SAFETY: The validated access-allowed ACE includes the complete fixed-size mask structure.
    let entry = unsafe { &*entry.cast::<ACCESS_ALLOWED_ACE>() };
    let mapping = GENERIC_MAPPING {
        GenericRead: FILE_GENERIC_READ,
        GenericWrite: FILE_GENERIC_WRITE,
        GenericExecute: FILE_GENERIC_EXECUTE,
        GenericAll: FILE_ALL_ACCESS,
    };
    let mut mask = entry.Mask;
    // SAFETY: Both mapping and access-mask pointers are live for the synchronous mapping.
    unsafe { MapGenericMask(&mut mask, &mapping) };
    assert_eq!(
        mask,
        FILE_GENERIC_READ | FILE_GENERIC_WRITE,
        "Windows grants exactly Git's mapped read/write permissions"
    );
    Ok(())
}
