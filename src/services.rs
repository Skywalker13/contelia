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
use std::{io, process::Command};

pub struct Services {}

impl Services {
    pub fn new() -> Result<Self> {
        let services = Self {};

        Ok(services)
    }

    fn exec(&self, args: [&str; 2]) -> io::Result<()> {
        let output = Command::new("sv").args(args).output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr),
            ));
        }

        Ok(())
    }

    fn stop_service(&self, name: &str) -> io::Result<()> {
        self.exec(["down", name])
    }

    fn start_service(&self, name: &str) -> io::Result<()> {
        self.exec(["up", name])
    }

    pub fn start(&self) -> io::Result<()> {
        self.start_service("wifi")?;
        self.start_service("hostapd")?;
        self.start_service("dnsmasq")?;
        self.start_service("httpd")?;
        Ok(())
    }

    pub fn stop(&self) -> io::Result<()> {
        self.stop_service("httpd")?;
        self.stop_service("dnsmasq")?;
        self.stop_service("hostapd")?;
        self.stop_service("wifi")?;
        Ok(())
    }
}
