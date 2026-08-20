/// The path to the default TTY on linux
pub const TTY_PATH: &str = "/dev/tty";

#[cfg(unix)]
pub(crate) mod imp {
    use std::{
        fs::File,
        io,
        io::{BufRead, Read, Write},
    };

    use gix_error::{ErrorExt, Message, ResultExt, message};
    use parking_lot::{Mutex, RawMutex, const_mutex, lock_api::MutexGuard};
    use rustix::termios::{self, Termios};

    use crate::{Error, Mode, Options, unix::TTY_PATH};

    static TERM_STATE: Mutex<Option<Termios>> = const_mutex(None);

    fn tty_io() -> Message {
        gix_error::message!("Failed to open terminal at {TTY_PATH:?} for writing prompt, or to write it")
    }

    fn terminal_configuration() -> Message {
        message("Failed to obtain or set terminal configuration")
    }

    /// Ask the user given a `prompt`, returning the result.
    pub(crate) fn ask(prompt: &str, Options { mode, .. }: &Options) -> Result<String, Error> {
        match mode {
            Mode::Disable => Err(message("Terminal prompts are disabled").raise()),
            Mode::Hidden => {
                let state = TERM_STATE.lock();
                let mut in_out = save_term_state_and_disable_echo(
                    state,
                    std::fs::OpenOptions::new()
                        .write(true)
                        .read(true)
                        .open(TTY_PATH)
                        .or_raise(tty_io)?,
                )?;
                in_out.write_all(prompt.as_bytes()).or_raise(tty_io)?;

                let mut buf_read = std::io::BufReader::with_capacity(64, in_out);
                let mut out = String::with_capacity(64);
                buf_read.read_line(&mut out).or_raise(tty_io)?;

                out.pop();
                if out.ends_with('\r') {
                    out.pop();
                }
                buf_read.into_inner().restore_term_state()?;
                Ok(out)
            }
            Mode::Visible => {
                let mut in_out = std::fs::OpenOptions::new()
                    .write(true)
                    .read(true)
                    .open(TTY_PATH)
                    .or_raise(tty_io)?;
                in_out.write_all(prompt.as_bytes()).or_raise(tty_io)?;

                let mut buf_read = std::io::BufReader::with_capacity(64, in_out);
                let mut out = String::with_capacity(64);
                buf_read.read_line(&mut out).or_raise(tty_io)?;
                Ok(out.trim_end().to_owned())
            }
        }
    }

    type TermiosGuard<'a> = MutexGuard<'a, RawMutex, Option<Termios>>;

    struct RestoreTerminalStateOnDrop<'a> {
        state: TermiosGuard<'a>,
        fd: File,
    }

    impl Read for RestoreTerminalStateOnDrop<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.fd.read(buf)
        }

        fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
            self.fd.read_vectored(bufs)
        }
    }

    impl Write for RestoreTerminalStateOnDrop<'_> {
        #[inline(always)]
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.fd.write(buf)
        }

        #[inline(always)]
        fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
            self.fd.write_vectored(bufs)
        }

        #[inline(always)]
        fn flush(&mut self) -> io::Result<()> {
            self.fd.flush()
        }
    }

    impl RestoreTerminalStateOnDrop<'_> {
        fn restore_term_state(mut self) -> Result<(), Error> {
            let state = self.state.take().expect("BUG: we exist only if something is saved");
            termios::tcsetattr(&self.fd, termios::OptionalActions::Flush, &state).or_raise(terminal_configuration)?;
            Ok(())
        }
    }

    impl Drop for RestoreTerminalStateOnDrop<'_> {
        fn drop(&mut self) {
            if let Some(state) = self.state.take() {
                termios::tcsetattr(&self.fd, termios::OptionalActions::Flush, &state).ok();
            }
        }
    }

    fn save_term_state_and_disable_echo(
        mut state: TermiosGuard<'_>,
        fd: File,
    ) -> Result<RestoreTerminalStateOnDrop<'_>, Error> {
        assert!(
            state.is_none(),
            "BUG: recursive calls are not possible and we restore afterwards"
        );

        let prev = termios::tcgetattr(&fd).or_raise(terminal_configuration)?;
        let mut new = prev.clone();
        *state = prev.into();

        new.local_modes &= !termios::LocalModes::ECHO;
        new.local_modes |= termios::LocalModes::ECHONL;
        termios::tcsetattr(&fd, termios::OptionalActions::Flush, &new).or_raise(terminal_configuration)?;

        Ok(RestoreTerminalStateOnDrop { fd, state })
    }
}
