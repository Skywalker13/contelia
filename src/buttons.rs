/* Contelia
 * Copyright (C) 2025  Mathieu Schroeter <mathieu@schroetersa.ch>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use anyhow::Result;
use evdev::{Device, KeyCode};
use nix::sys::epoll;
use std::{error::Error, path::Path};

pub struct Buttons {
    device: Device,
    epoll: epoll::Epoll,
    status: Status,
}

#[derive(Debug)]
pub struct Status {
    dpad_left: bool,
    dpad_right: bool,
    dpad_up: bool,
    dpad_down: bool,
    start: bool,
    select: bool,
}

impl Buttons {
    pub fn new(input: &Path) -> Result<Self> {
        // See https://github.com/emberian/evdev/blob/main/examples/evtest_nonblocking.rs
        let device = Device::open(input)?;
        device.set_nonblocking(true)?;

        let epoll = epoll::Epoll::new(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        let event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, 0);
        epoll.add(&device, event)?;

        let status = Status {
            dpad_left: false,
            dpad_right: false,
            dpad_up: false,
            dpad_down: false,
            start: false,
            select: false,
        };

        Ok(Self {
            device,
            epoll,
            status,
        })
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn listen(&mut self) -> Result<KeyCode, Box<dyn Error>> {
        let mut events = [epoll::EpollEvent::empty(); 2];

        loop {
            match self.device.fetch_events() {
                Ok(events) => {
                    for ev in events {
                        let value = ev.value();
                        let code = KeyCode::new(ev.code());
                        if match code {
                            KeyCode::BTN_DPAD_LEFT => {
                                self.status.dpad_left = value == 1;
                                self.status.dpad_left
                            }
                            KeyCode::BTN_DPAD_RIGHT => {
                                self.status.dpad_right = value == 1;
                                self.status.dpad_right
                            }
                            KeyCode::BTN_DPAD_UP => {
                                self.status.dpad_up = value == 1;
                                self.status.dpad_up
                            }
                            KeyCode::BTN_DPAD_DOWN => {
                                self.status.dpad_down = value == 1;
                                self.status.dpad_down
                            }
                            KeyCode::BTN_START => {
                                self.status.start = value == 1;
                                self.status.start
                            }
                            KeyCode::BTN_SELECT => {
                                self.status.select = value == 1;
                                self.status.select
                            }
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
