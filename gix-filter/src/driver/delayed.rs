use bstr::{BStr, BString};

use crate::{
    driver,
    driver::{Operation, State, apply::handle_io_err},
};

///
pub mod list {
    use crate::driver;

    /// The error returned by [State::list_delayed_paths()][super::State::list_delayed_paths()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not get process named '{}' which should be running and tracked", wanted.0)]
        ProcessMissing { wanted: driver::Key },
        #[error("Failed to run 'list_available_blobs' command")]
        ProcessInvoke(#[from] driver::process::client::invoke::without_content::Error),
        #[error("The invoked command 'list_available_blobs' in process indicated an error: {status:?}")]
        ProcessStatus { status: driver::process::Status },
    }
}

///
pub mod fetch {
    use crate::driver;

    /// The error returned by [State::fetch_delayed()][super::State::fetch_delayed()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not get process named '{}' which should be running and tracked", wanted.0)]
        ProcessMissing { wanted: driver::Key },
        #[error("Failed to run '{command}' command")]
        ProcessInvoke {
            command: String,
            source: driver::process::client::invoke::Error,
        },
        #[error("The invoked command '{command}' in process indicated an error: {status:?}")]
        ProcessStatus {
            status: driver::process::Status,
            command: String,
        },
    }
}

/// Operations related to delayed filtering.
impl State {
    /// Return a list of delayed paths for `process` that can then be obtained with [`fetch_delayed()`][Self::fetch_delayed()].
    ///
    /// A process abiding the protocol will eventually list all previously delayed paths for any invoked command, or
    /// signals that it is done with all delayed paths by returning an empty list.
    /// It's up to the caller to validate these assumptions.
    ///
    /// ### Error Handling
    ///
    /// Usually if the process sends the "abort" status, we will not use a certain capability again. Here it's unclear what capability
    /// that is and what to do, so we leave the process running and do nothing else (just like `git`).
    pub fn list_delayed_paths(&mut self, process: &driver::Key) -> Result<Vec<BString>, list::Error> {
        let client = self
            .running
            .get_mut(&process.0)
            .ok_or_else(|| list::Error::ProcessMissing {
                wanted: process.clone(),
            })?;

        let mut out = Vec::new();
        let result = client.invoke_without_content("list_available_blobs", &mut None.into_iter(), &mut |line| {
            if let Some(path) = line.strip_prefix(b"pathname=") {
                out.push(path.into());
            }
        });
        let status = match result {
            Ok(res) => res,
            Err(err) => {
                if let driver::process::client::invoke::without_content::Error::Io(err) = &err {
                    handle_io_err(err, &mut self.running, process.0.as_ref());
                }
                return Err(err.into());
            }
        };

        if status.is_success() {
            Ok(out)
        } else {
            let message = status.message().unwrap_or_default();
            match message {
                "error" | "abort" => {}
                _strange => {
                    let client = self.running.remove(&process.0).expect("we definitely have it");
                    client.into_child().kill().ok();
                }
            }
            Err(list::Error::ProcessStatus { status })
        }
    }

    /// Given a `process` and a `path`  (as previously returned by [list_delayed_paths()][Self::list_delayed_paths()]), return
    /// a reader to stream the filtered result. Note that `operation` must match the original operation that produced the delayed result
    /// or the long-running process might not know the path, depending on its implementation.
    pub fn fetch_delayed(
        &mut self,
        process: &driver::Key,
        path: &BStr,
        operation: Operation,
    ) -> Result<impl std::io::Read + '_, fetch::Error> {
        let client = self
            .running
            .get_mut(&process.0)
            .ok_or_else(|| fetch::Error::ProcessMissing {
                wanted: process.clone(),
            })?;

        let result = client.invoke(
            operation.as_str(),
            &mut [("pathname", path.to_owned())].into_iter(),
            &mut &b""[..],
        );
        let status = match result {
            Ok(status) => status,
            Err(err) => {
                let driver::process::client::invoke::Error::Io(io_err) = &err;
                handle_io_err(io_err, &mut self.running, process.0.as_ref());
                return Err(fetch::Error::ProcessInvoke {
                    command: operation.as_str().into(),
                    source: err,
                });
            }
        };
        if status.is_success() {
            // TODO: find a way to not have to do the 'borrow-dance'.
            let client = self.running.remove(&process.0).expect("present for borrowcheck dance");
            self.running.insert(process.0.clone(), client);
            let client = self.running.get_mut(&process.0).expect("just inserted");

            Ok(client.as_read())
        } else {
            let message = status.message().unwrap_or_default();
            match message {
                "abort" => {
                    client.capabilities_mut().remove(operation.as_str());
                }
                "error" => {}
                _strange => {
                    let client = self.running.remove(&process.0).expect("we definitely have it");
                    client.into_child().kill().ok();
                }
            }
            Err(fetch::Error::ProcessStatus {
                command: operation.as_str().into(),
                status,
            })
        }
    }
}
