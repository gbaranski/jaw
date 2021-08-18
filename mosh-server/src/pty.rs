use std::{
    ffi::{CStr, OsStr},
    fs::File,
    os::unix::prelude::{AsRawFd, FromRawFd, OsStrExt},
};
use tokio::{
    io,
    process::{Child, Command},
};

pub struct Size {
    pub col: u16,
    pub row: u16,
}

#[derive(Debug)]
pub struct System {
    pub child: Child,
    fd: File,
}

impl System {
    pub fn new(mut command: Command) -> Result<Self, io::Error> {
        let master = unsafe { Self::master()? };
        let slave = unsafe { Self::slave(&master)? };
        let master_fd = master.as_raw_fd();
        command.stdin(slave.try_clone().expect("clone stdin error"));
        command.stdout(slave.try_clone().expect("clone stdout error"));
        // command.stdout(std::process::Stdio::null());
        command.stderr(slave);

        unsafe {
            command.pre_exec(move || {
                // This is OK even though we don't own master since this process is
                // about to become something totally different anyway.
                if libc::close(master_fd) != 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::ioctl(0, libc::TIOCSCTTY.into(), 1) != 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }

        dbg!(&command);
        let child = command.spawn()?;
        dbg!(&child);

        Ok(Self { child, fd: master })
    }

    unsafe fn master() -> Result<File, io::Error> {
        // On MacOS, O_NONBLOCK is not documented as an allowed option to
        // posix_openpt(), but it is in fact allowed and functional, and
        // trying to add it later with fcntl() is forbidden. Meanwhile, on
        // FreeBSD, O_NONBLOCK is *not* an allowed option to
        // posix_openpt(), and the only way to get a nonblocking PTY
        // master is to add the nonblocking flag with fcntl() later. So,
        // we have to jump through some #[cfg()] hoops.

        const APPLY_NONBLOCK_AFTER_OPEN: bool = cfg!(target_os = "freebsd");

        let fd = if APPLY_NONBLOCK_AFTER_OPEN {
            libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY)
        } else {
            libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_NONBLOCK)
        };

        if fd < 0 {
            return Err(io::Error::last_os_error());
        }

        if libc::grantpt(fd) != 0 {
            return Err(io::Error::last_os_error());
        }

        if libc::unlockpt(fd) != 0 {
            return Err(io::Error::last_os_error());
        }

        if APPLY_NONBLOCK_AFTER_OPEN {
            let flags = libc::fcntl(fd, libc::F_GETFL, 0);
            if flags < 0 {
                return Err(io::Error::last_os_error());
            }

            if libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(File::from_raw_fd(fd))
    }

    unsafe fn slave(master: &File) -> Result<File, io::Error> {
        let mut buf: [libc::c_char; 512] = [0; 512];
        let fd = master.as_raw_fd();

        #[cfg(not(any(target_os = "macos", target_os = "freebsd")))]
        {
            if libc::ptsname_r(fd, buf.as_mut_ptr(), buf.len()) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(any(target_os = "macos", target_os = "freebsd"))]
        unsafe {
            let st = libc::ptsname(fd);
            if st.is_null() {
                return Err(io::Error::last_os_error());
            }
            libc::strncpy(buf.as_mut_ptr(), st, buf.len());
        }

        let ptsname = OsStr::from_bytes(CStr::from_ptr(&buf as _).to_bytes());
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(ptsname)
    }

    pub fn resize(&self, size: Size) -> io::Result<()> {
        let fd = self.fd.as_raw_fd();

        let winsz = libc::winsize {
            ws_row: size.row,
            ws_col: size.col,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        if unsafe { libc::ioctl(fd, libc::TIOCSWINSZ.into(), &winsz) } != 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }
}
