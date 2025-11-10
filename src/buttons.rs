use anyhow::Result;
use evdev::{Device, KeyCode};
use nix::sys::epoll;
use std::collections::HashSet;
use std::error::Error;

const DEV_INPUT: &str = "/dev/input/gamepi13";

pub struct Buttons {
    device: Device,
    epoll: epoll::Epoll,
}

impl Buttons {
    pub fn new() -> Result<Self> {
        // See https://github.com/emberian/evdev/blob/main/examples/evtest_nonblocking.rs
        let device = Device::open(DEV_INPUT)?;
        device.set_nonblocking(true)?;

        let epoll = epoll::Epoll::new(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        let event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, 0);
        epoll.add(&device, event)?;

        Ok(Buttons { device, epoll })
    }

    pub fn listen(&mut self) -> Result<KeyCode, Box<dyn Error>> {
        let mut events = [epoll::EpollEvent::empty(); 2];

        let key_codes = HashSet::from([
            KeyCode::BTN_DPAD_UP,
            KeyCode::BTN_DPAD_DOWN,
            KeyCode::BTN_DPAD_LEFT,
            KeyCode::BTN_DPAD_RIGHT,
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
