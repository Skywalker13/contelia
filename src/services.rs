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

    fn exec(&self, args: Vec<&str>) -> io::Result<()> {
        let output = Command::new("sv").args(args).output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr),
            ));
        }

        Ok(())
    }

    pub fn start(&self) -> io::Result<()> {
        self.exec(vec!["up", "wifi", "hostapd", "dnsmasq", "httpd"])?;
        Ok(())
    }

    pub fn stop(&self) -> io::Result<()> {
        self.exec(vec!["down", "wifi", "hostapd", "dnsmasq", "httpd"])?;
        Ok(())
    }
}
