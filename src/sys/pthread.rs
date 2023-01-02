//! Low level threading primitives

#[cfg(not(target_os = "redox"))]
use crate::errno::Errno;
#[cfg(not(target_os = "redox"))]
use crate::Result;
use libc::{self, pthread_t};
#[cfg(not(target_os = "redox"))]
use libc::c_int;

#[cfg(target_os = "linux")]
use std::cell::UnsafeCell;

/// Identifies an individual thread.
pub type Pthread = pthread_t;

/// Obtain ID of the calling thread (see
/// [`pthread_self(3)`](https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_self.html)
///
/// The thread ID returned by `pthread_self()` is not the same thing as
/// the kernel thread ID returned by a call to `gettid(2)`.
#[inline]
pub fn pthread_self() -> Pthread {
    unsafe { libc::pthread_self() }
}

feature! {
#![feature = "signal"]

/// Send a signal to a thread (see [`pthread_kill(3)`]).
///
/// If `signal` is `None`, `pthread_kill` will only preform error checking and
/// won't send any signal.
///
/// [`pthread_kill(3)`]: https://pubs.opengroup.org/onlinepubs/9699919799/functions/pthread_kill.html
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[cfg(not(target_os = "redox"))]
pub fn pthread_kill<T>(thread: Pthread, signal: T) -> Result<()>
    where T: Into<Option<crate::sys::signal::Signal>>
{
    let sig = match signal.into() {
        Some(s) => s as c_int,
        None => 0,
    };
    let res = unsafe { libc::pthread_kill(thread, sig) };
    Errno::result(res).map(drop)
}
}

/// Mutex protocol.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Protocol {
    /// [`libc::PTHREAD_PRIO_NONE`]
    None = libc::PTHREAD_PRIO_NONE,
    /// [`libc::PTHREAD_PRIO_INHERIT`]
    Inherit = libc::PTHREAD_PRIO_INHERIT,
    /// [`libc::PTHREAD_PRIO_PROTECT`]
    Protect = libc::PTHREAD_PRIO_PROTECT
}
#[cfg(target_os = "linux")]
impl From<i32> for Protocol {
    fn from(x: i32) -> Self {
        match x {
            libc::PTHREAD_PRIO_NONE => Self::None,
            libc::PTHREAD_PRIO_INHERIT => Self::Inherit,
            libc::PTHREAD_PRIO_PROTECT => Self::Protect,
            _ => unreachable!()
        }
    }
}

/// Mutex attributes.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct MutexAttr(libc::pthread_mutexattr_t);

#[cfg(target_os = "linux")]
impl MutexAttr {
    /// Wraps [`libc::pthread_mutexattr_init`].
    pub fn new() -> Result<Self> {
        let attr = unsafe {
            let mut uninit = std::mem::MaybeUninit::<libc::pthread_mutexattr_t>::uninit();
            Errno::result(libc::pthread_mutexattr_init(uninit.as_mut_ptr()))?;
            uninit.assume_init()
        };
        Ok(Self(attr))
    }
    /// Wraps [`libc::pthread_mutexattr_getpshared`].
    pub fn get_shared(&self) -> Result<bool> {
        let init = unsafe {
            let mut uninit = std::mem::MaybeUninit::uninit();
            Errno::result(libc::pthread_mutexattr_getpshared(&self.0,uninit.as_mut_ptr()))?;
            uninit.assume_init()
        };
        Ok(init == libc::PTHREAD_PROCESS_SHARED)
    }
    /// Wraps [`libc::pthread_mutexattr_setpshared`].
    pub fn set_shared(&mut self, shared: bool) -> Result<()> {
        let shared = if shared { libc::PTHREAD_PROCESS_SHARED} else { libc::PTHREAD_PROCESS_PRIVATE };
        unsafe {
            Errno::result(libc::pthread_mutexattr_setpshared(&mut self.0,shared)).map(drop)
        }
    }
    /// Wraps [`libc::pthread_mutexattr_getrobust`].
    pub fn get_robust(&self) -> Result<bool> {
        let init = unsafe {
            let mut uninit = std::mem::MaybeUninit::uninit();
            Errno::result(libc::pthread_mutexattr_getrobust(&self.0,uninit.as_mut_ptr()))?;
            uninit.assume_init()
        };
        Ok(init == libc::PTHREAD_MUTEX_ROBUST)
    }
    /// Wraps [`libc::pthread_mutexattr_setrobust`].
    pub fn set_robust(&mut self, robust: bool) -> Result<()> {
        let robust = if robust { libc::PTHREAD_MUTEX_ROBUST} else { libc::PTHREAD_MUTEX_STALLED };
        unsafe {
            Errno::result(libc::pthread_mutexattr_setrobust(&mut self.0,robust)).map(drop)
        }
    }
    /// Wraps [`libc::pthread_mutexattr_getprotocol`].
    pub fn get_protocol(&self) -> Result<Protocol> {
        let init = unsafe {
            let mut uninit = std::mem::MaybeUninit::uninit();
            Errno::result(libc::pthread_mutexattr_getprotocol(&self.0,uninit.as_mut_ptr()))?;
            uninit.assume_init()
        };
        Ok(Protocol::from(init))
    }
    /// Wraps [`libc::pthread_mutexattr_setprotocol`].
    pub fn set_protocol(&mut self, protocol: Protocol) -> Result<()> {
        unsafe {
            Errno::result(libc::pthread_mutexattr_setprotocol(&mut self.0,protocol as i32)).map(drop)
        }
    }
}
#[cfg(target_os = "linux")]
impl std::default::Default for MutexAttr {
    fn default() -> Self {
        let mutex_attr = Self::new().unwrap();
        debug_assert_eq!(mutex_attr.get_shared(),Ok(true));
        mutex_attr
    }
}
#[cfg(target_os = "linux")]
impl std::ops::Drop for MutexAttr {
    /// Wraps [`libc::pthread_mutexattr_destroy`].
    fn drop(&mut self) {
        unsafe {
            Errno::result(libc::pthread_mutexattr_destroy(&mut self.0)).unwrap();
        }
    }
}

/// Mutex.
/// ```
/// # use std::{
/// #   sync::Arc,
/// #   time::{Instant, Duration},
/// #   thread::{sleep, spawn},
/// #   mem::size_of,
/// #   num::NonZeroUsize,
/// #   os::unix::io::OwnedFd
/// # };
/// # use nix::{
/// #   sys::{pthread::{Mutex, MutexAttr}, mman::{mmap, MapFlags, ProtFlags}},
/// #   unistd::{fork,ForkResult},
/// # };
/// const TIMEOUT: Duration = Duration::from_millis(500);
/// const DELTA: Duration = Duration::from_millis(100);
/// # fn main() -> nix::Result<()> {
/// let mutex = Mutex::default();
/// 
/// // The mutex is initialized unlocked, so an attempt to unlock it should
/// // return immediately.
/// assert_eq!(mutex.unlock(), Ok(()));
/// // The mutex is unlocked, so `try_lock` will lock.
/// assert_eq!(mutex.try_lock(), Ok(true));
/// // Unlock the mutex.
/// assert_eq!(mutex.unlock(), Ok(()));
/// // The mutex is unlocked, so `lock` will lock and exit immediately.
/// assert_eq!(mutex.lock(), Ok(()));
/// // Unlock the mutex.
/// assert_eq!(mutex.unlock(), Ok(()));
/// 
/// // Test across threads
/// // -------------------------------------------------------------------------
/// 
/// let mutex = Arc::new(mutex);
/// let mutex_clone = mutex.clone();
/// let instant = Instant::now();
/// spawn(move || {
///     assert_eq!(mutex_clone.lock(), Ok(()));
///     sleep(TIMEOUT);
///     assert_eq!(mutex_clone.unlock(), Ok(()));
/// });
/// sleep(DELTA);
/// assert_eq!(mutex.lock(), Ok(()));
/// assert!(instant.elapsed() > TIMEOUT && instant.elapsed() < TIMEOUT + DELTA);
/// 
/// // Test across processes
/// // -------------------------------------------------------------------------
///
/// let shared_memory = unsafe { mmap::<OwnedFd>(
///     None,
///     NonZeroUsize::new_unchecked(size_of::<Mutex>()),
///     ProtFlags::PROT_WRITE | ProtFlags::PROT_READ,
///     MapFlags::MAP_SHARED | MapFlags::MAP_ANONYMOUS,
///     None,
///     0
/// )? };
/// let mutex_ptr = shared_memory.cast::<Mutex>();
/// let mutex = unsafe { &*mutex_ptr };
/// 
/// // If transmute or cast into a mutex, you must ensure it is initialized.
/// // By default mutex's are process private, so we need to initialize with the `MutexAttr` with
/// // shared.
/// let mut mutex_attr = MutexAttr::new()?;
/// mutex_attr.set_shared(true)?;
/// mutex.init(Some(mutex_attr))?;
/// 
/// match unsafe { fork()? } {
///     ForkResult::Parent { child } => {
///         assert_eq!(mutex.lock(), Ok(()));
///         sleep(TIMEOUT);
///         assert_eq!(mutex.unlock(), Ok(()));
///         // Wait for child process to exit
///         unsafe {
///             assert_eq!(libc::waitpid(child.as_raw(),std::ptr::null_mut(),0),child.as_raw());
///         }
///     },
///     ForkResult::Child => {
///         let now = Instant::now();
///         sleep(DELTA);
///         assert_eq!(mutex.lock(), Ok(()));
///         assert!(now.elapsed() > TIMEOUT && now.elapsed() < TIMEOUT + DELTA);
///     }
/// }
/// 
/// # Ok(())
/// # }
/// ```
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct Mutex(UnsafeCell<libc::pthread_mutex_t>);
#[cfg(target_os = "linux")]
impl Mutex {
    /// Wraps [`libc::pthread_mutex_init`].
    pub fn init(&self, attr: Option<MutexAttr>) -> Result<()> {
        let attr = match attr {
            Some(mut x) => &mut x.0,
            None => std::ptr::null_mut()
        };
        unsafe {
            Errno::result(libc::pthread_mutex_init(self.0.get(),attr))?;
        }
        Ok(())
    }
    /// Wraps [`libc::pthread_mutex_init`].
    pub fn new(attr: Option<MutexAttr>) -> Result<Self> {
        let attr = match attr {
            Some(mut x) => &mut x.0,
            None => std::ptr::null_mut()
        };
        let init = unsafe {
            let mut uninit = std::mem::MaybeUninit::<libc::pthread_mutex_t>::uninit();
            Errno::result(libc::pthread_mutex_init(uninit.as_mut_ptr(),attr))?;
            uninit.assume_init()
        };
        Ok(Self(UnsafeCell::new(init)))
    }
    /// Wraps [`libc::pthread_mutex_lock`].
    /// 
    /// <https://man7.org/linux/man-pages/man3/pthread_mutex_lock.3p.html>
    pub fn lock(&self) -> Result<()> {
        unsafe {
            Errno::result(libc::pthread_mutex_lock(self.0.get())).map(drop)
        }
    }
    /// Wraps [`libc::pthread_mutex_trylock`].
    /// 
    /// <https://man7.org/linux/man-pages/man3/pthread_mutex_lock.3p.html>
    pub fn try_lock(&self) -> Result<bool> {
        unsafe {
            match Errno::result(libc::pthread_mutex_trylock(self.0.get())) {
                Ok(_) => Ok(true),
                Err(Errno::EBUSY) => Ok(false),
                Err(err) => Err(err)
            }
            
        }
    }
    /// Wraps [`libc::pthread_mutex_unlock`].
    /// 
    /// <https://man7.org/linux/man-pages/man3/pthread_mutex_lock.3p.html>
    pub fn unlock(&self) -> Result<()> {
        unsafe {
            Errno::result(libc::pthread_mutex_unlock(self.0.get())).map(drop)
        }
    }
}
#[cfg(target_os = "linux")]
unsafe impl Sync for Mutex {}
#[cfg(target_os = "linux")]
impl std::default::Default for Mutex {
    fn default() -> Self {
        Self::new(None).unwrap()
    }
}
#[cfg(target_os = "linux")]
impl std::ops::Drop for Mutex {
    /// Wraps [`libc::pthread_mutex_destroy`].
    fn drop(&mut self) {
        unsafe {
            Errno::result(libc::pthread_mutex_destroy(self.0.get())).unwrap();
        }
    }
}