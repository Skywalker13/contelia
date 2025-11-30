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

use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicBool};
use std::thread;
use std::time::Duration;

pub struct Timeout {
    abort: Arc<AtomicBool>,
}

impl Timeout {
    pub fn set<F>(delay: Duration, callback: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let abort = Arc::new(AtomicBool::new(false));
        let abort_clone = abort.clone();

        thread::spawn(move || {
            thread::sleep(delay);
            if !abort_clone.load(Ordering::Relaxed) {
                callback();
            }
        });

        Self { abort }
    }

    pub fn clear(&mut self) {
        self.abort.store(true, Ordering::Relaxed);
    }
}
