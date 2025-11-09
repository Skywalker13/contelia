use anyhow::Result;
use evdev::{Device, KeyCode};
use nix::sys::epoll;
use std::collections::HashSet;
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

    pub fn listen(&mut self) -> Result<KeyCode, Box<dyn Error>> {
        let mut events = [epoll::EpollEvent::empty(); 2];

        let key_codes = HashSet::from([
            KeyCode::KEY_UP,
            KeyCode::KEY_DOWN,
            KeyCode::KEY_LEFT,
            KeyCode::KEY_RIGHT,
            KeyCode::BTN_NORTH,
            KeyCode::BTN_SOUTH,
            KeyCode::BTN_EAST,
            KeyCode::BTN_WEST,
            KeyCode::BTN_SELECT,
            KeyCode::BTN_START,
            KeyCode::BTN_TL,
            KeyCode::BTN_TR,
        ]);

        loop {
            match self.device.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        let code = KeyCode::new(ev.code());
                        let value = ev.value();
                        if key_codes.contains(&code) && value == 1 {
                            return Ok(code);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    self.epoll.wait(&mut events, epoll::EpollTimeout::NONE)?;
                }
                Err(e) => {
                    return Err(Box::new(e));
                }
            };
        }
    }
}
