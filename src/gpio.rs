use anyhow::Result;
use evdev::Device;
use nix::sys::epoll;
use std::error::Error;

pub struct GPIO {
    device: Device,
    epoll: epoll::Epoll,
}

impl GPIO {
    pub fn new() -> Result<Self> {
        // See https://github.com/emberian/evdev/blob/main/examples/evtest_nonblocking.rs
        let device = Device::open("/dev/input/gamepi13")?;
        device.set_nonblocking(true)?;

        let epoll = epoll::Epoll::new(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        let event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, 0);
        epoll.add(&device, event)?;

        Ok(GPIO { device, epoll })
    }

    pub fn listen(&mut self) -> Result<(), Box<dyn Error>> {
        let mut events = [epoll::EpollEvent::empty(); 2];

        loop {
            match self.device.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        println!("{ev:?}")
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.epoll.wait(&mut events, epoll::EpollTimeout::NONE)?;
                }
                Err(e) => {
                    eprintln!("{e}");
                    break;
                }
            }
        }

        Ok(())
    }
}
