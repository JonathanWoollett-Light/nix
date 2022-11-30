use nix::sys::{epoll::{Epoll, EpollEvent, EpollFlags, EpollCreateFlags}, eventfd::{eventfd, EfdFlags}};
use nix::unistd::write;
use std::os::unix::io::{OwnedFd, FromRawFd, AsRawFd, AsFd};
use std::time::{Instant, Duration};

const DATA: u64 = 17;
const MILLIS: u64 = 100;

fn nix_epoll_create_test() -> std::result::Result<(),i32> {
    let _epoll = Epoll::new_test(EpollCreateFlags::empty())?;
    Ok(())
}
fn nix_epoll_create() -> nix::Result<()> {
    let _epoll = Epoll::new(EpollCreateFlags::empty())?;
    Ok(())
}
fn libc_epoll_create() -> Result<(),i32> {
    unsafe {
        let epoll = libc::epoll_create1(0);
        if epoll == -1 {
            return Err(*libc::__errno_location());
        }
        libc::close(epoll);
        Ok(())
    }
}

fn nix_epoll() -> nix::Result<()> {
    // Create epoll
    let epoll = Epoll::new(EpollCreateFlags::empty()).unwrap();

    // Create eventfd & Add event
    let eventfd = unsafe { OwnedFd::from_raw_fd(eventfd(0, EfdFlags::empty())?) };
    epoll.add(eventfd.as_fd(), EpollEvent::new(EpollFlags::EPOLLIN,DATA))?;

    // Arm eventfd & Time wait
    write(eventfd.as_raw_fd(), &1u64.to_ne_bytes())?;
    let now = Instant::now();

    // Wait on event
    let mut events = [EpollEvent::empty()];
    epoll.wait(&mut events, MILLIS as isize)?;

    // Assert data correct & timeout didn't occur
    assert_eq!(events[0].data(), DATA);
    assert!(now.elapsed() < Duration::from_millis(MILLIS));
    Ok(())
}
fn libc_epoll() {
    unsafe {
        // Create epoll
        let epoll = libc::epoll_create1(0);
    
        // Create eventfd & Add event
        let eventfd = libc::eventfd(0, 0);
        let mut epoll_event = libc::epoll_event {
            events: libc::EPOLLIN as u32,
            r#u64: DATA,
        };
        libc::epoll_ctl(epoll, libc::EPOLL_CTL_ADD, eventfd, &mut epoll_event);
    
        // Arm eventfd & Time wait
        let buf = 1u64.to_ne_bytes();
        libc::write(eventfd, (&buf as *const [u8; 8]).cast(), std::mem::size_of::<u64>());
        let now = Instant::now();
    
        // Wait on event
        let mut events = std::mem::MaybeUninit::uninit();
        libc::epoll_wait(epoll, events.as_mut_ptr(), 1, MILLIS as i32);
    
        // Assert data correct & timeout didn't occur
        let event = events.assume_init();
        let data = event.r#u64;
        assert_eq!(data, DATA);
        assert!(now.elapsed() < Duration::from_millis(MILLIS));

        libc::close(eventfd);
        libc::close(epoll);
    }
}


// iai::main!(nix_epoll,libc_epoll,nix_epoll_create,libc_epoll_create);
iai::main!(nix_epoll_create,libc_epoll_create,nix_epoll_create_test);