#![allow(deprecated)]

use nix::errno::Errno;
use nix::sys::epoll::{epoll_create1, epoll_ctl};
use nix::sys::epoll::{
    Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollOp,
};

#[test]
pub fn test_deprecated_epoll_errno() {
    let efd = epoll_create1(EpollCreateFlags::empty()).unwrap();
    let result = epoll_ctl(efd, EpollOp::EpollCtlDel, 1, None);
    result.expect_err("assertion failed");
    assert_eq!(result.unwrap_err(), Errno::ENOENT);

    let result = epoll_ctl(efd, EpollOp::EpollCtlAdd, 1, None);
    result.expect_err("assertion failed");
    assert_eq!(result.unwrap_err(), Errno::EINVAL);
}

#[test]
pub fn test_deprecated_epoll_ctl() {
    let efd = epoll_create1(EpollCreateFlags::empty()).unwrap();
    let mut event =
        EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, 1);
    epoll_ctl(efd, EpollOp::EpollCtlAdd, 1, &mut event).unwrap();
    epoll_ctl(efd, EpollOp::EpollCtlDel, 1, None).unwrap();
}

#[test]
pub fn test_epoll_errno() {
    let efd = Epoll::new(EpollCreateFlags::empty()).unwrap();
    let result = efd.epoll_ctl(EpollOp::EpollCtlDel, 1, None);
    assert_eq!(result, Err(Errno::ENOENT));

    let result = efd.epoll_ctl(EpollOp::EpollCtlAdd, 1, None);
    assert_eq!(result, Err(Errno::EFAULT));
}

#[test]
pub fn test_epoll_ctl() {
    let efd = Epoll::new(EpollCreateFlags::empty()).unwrap();
    let mut event =
        EpollEvent::new(EpollFlags::EPOLLIN | EpollFlags::EPOLLERR, 1);
    efd.epoll_ctl(EpollOp::EpollCtlAdd, 1, &mut event).unwrap();
    efd.epoll_ctl(EpollOp::EpollCtlDel, 1, None).unwrap();
}
