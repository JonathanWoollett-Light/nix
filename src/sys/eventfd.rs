use crate::errno::Errno;
use crate::{Result,unistd};
use std::os::unix::io::{FromRawFd, OwnedFd, AsRawFd, AsFd, RawFd, BorrowedFd};

libc_bitflags! {
    pub struct EfdFlags: libc::c_int {
        EFD_CLOEXEC; // Since Linux 2.6.27
        EFD_NONBLOCK; // Since Linux 2.6.27
        EFD_SEMAPHORE; // Since Linux 2.6.30
    }
}

#[deprecated(since = "0.27.0", note = "Use EventFd::value_and_flags() instead")]
pub fn eventfd(initval: libc::c_uint, flags: EfdFlags) -> Result<OwnedFd> {
    let res = unsafe { libc::eventfd(initval, flags.bits()) };

    Errno::result(res).map(|r| unsafe { OwnedFd::from_raw_fd(r) })
}

#[derive(Debug)]
pub struct EventFd(pub OwnedFd);
impl EventFd {
    /// [`EventFd::value_and_flags`] with `init_val = 0` and `flags = EfdFlags::empty()`.
    pub fn new() -> Result<Self> {
        let res = unsafe { libc::eventfd(0, EfdFlags::empty().bits()) };
        Errno::result(res).map(|r| Self(unsafe { OwnedFd::from_raw_fd(r) }))
    }
    /// Constructs [`EventFd`] with the given `init_val` and `flags`.
    /// 
    /// Wrapper around [`libc::eventfd`].
    pub fn value_and_flags(init_val: u32, flags: EfdFlags) -> Result<Self> {
        let res = unsafe { libc::eventfd(init_val, flags.bits()) };
        Errno::result(res).map(|r| Self(unsafe { OwnedFd::from_raw_fd(r) }))
    }
    /// [`EventFd::value_and_flags`] with `init_val = 0` and given `flags`.
    pub fn flags(flags: EfdFlags) -> Result<Self> {
        let res = unsafe { libc::eventfd(0, flags.bits()) };
        Errno::result(res).map(|r| Self(unsafe { OwnedFd::from_raw_fd(r) }))
    }
    /// [`EventFd::value_and_flags`] with given `init_val` and `flags = EfdFlags::empty()`.
    pub fn value(init_val: u32) -> Result<Self> {
        let res = unsafe { libc::eventfd(init_val, EfdFlags::empty().bits()) };
        Errno::result(res).map(|r| Self(unsafe { OwnedFd::from_raw_fd(r) }))
    }
    /// [`EventFd::write`] with `1`.
    pub fn arm(&self) -> Result<usize> {
        unistd::write(self.0.as_raw_fd(),&1u64.to_ne_bytes())
    }
    /// [`EventFd::write`] with `0`.
    pub fn defuse(&self) -> Result<usize> {
        unistd::write(self.0.as_raw_fd(),&0u64.to_ne_bytes())
    }
    /// Writes a given `value` to the file descriptor.
    pub fn write(&self, value: u64) -> Result<usize> {
        unistd::write(self.0.as_raw_fd(),&value.to_ne_bytes())
    }
}
impl AsFd for EventFd {
    fn as_fd(&self) -> BorrowedFd {
        self.0.as_fd()
    }
}
impl AsRawFd for EventFd {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}