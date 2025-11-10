use anyhow::Result;
use evdev::{Device, KeyCode};
use nix::sys::epoll;
use std::error::Error;

use crate::book::ControlSettings;

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

    pub fn listen(
        &mut self,
        control_settings: &ControlSettings,
    ) -> Result<KeyCode, Box<dyn Error>> {
        let mut events = [epoll::EpollEvent::empty(); 2];

        loop {
            match self.device.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        let value = ev.value();
                        if value != 1 {
                            continue;
                        }

                        let code = KeyCode::new(ev.code());
                        if match code {
                            KeyCode::BTN_DPAD_LEFT => control_settings.wheel,
                            KeyCode::BTN_DPAD_RIGHT => control_settings.wheel,
                            KeyCode::BTN_NORTH => control_settings.pause,
                            KeyCode::BTN_SELECT => control_settings.home,
                            KeyCode::BTN_START => control_settings.ok,
                            _ => false,
                        } {
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
