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
use clap::Parser;
use evdev::KeyCode;
use std::sync::mpsc::channel;
use std::{error::Error, path::Path, thread};

use contelia::{Books, Buttons, ControlSettings, Player, Screen, Stage};

fn is_key_enabled(control_settings: &ControlSettings, code: KeyCode) -> bool {
    match code {
        KeyCode::BTN_DPAD_LEFT | KeyCode::BTN_DPAD_RIGHT => control_settings.wheel,
        KeyCode::BTN_DPAD_UP | KeyCode::BTN_DPAD_DOWN => true, // volume
        KeyCode::BTN_SELECT => control_settings.home,
        KeyCode::BTN_START => control_settings.ok,
        KeyCode::BTN_NORTH => control_settings.pause,
        _ => false,
    }
}

/// Process the event and returns true is we want to skip the assets
fn process_event(books: &mut Books, state: &Stage, code: KeyCode, player: &mut Player) -> bool {
    if !is_key_enabled(&state.control_settings, code) {
        return true;
    }
    let Some(book) = books.get() else {
        return true;
    };
    match code {
        KeyCode::BTN_DPAD_LEFT => {
            if state.square_one {
                books.button_wheel_left();
            } else {
                book.button_wheel_left();
            }
            false
        }
        KeyCode::BTN_DPAD_RIGHT => {
            if state.square_one {
                books.button_wheel_right();
            } else {
                book.button_wheel_right();
            }
            false
        }
        KeyCode::BTN_DPAD_UP => (player.volume_up(), true).1,
        KeyCode::BTN_DPAD_DOWN => (player.volume_down(), true).1,
        KeyCode::BTN_SELECT => (book.button_home(), false).1,
        KeyCode::BTN_START => (book.button_ok(), false).1,
        KeyCode::BTN_NORTH => (player.toggle_pause(), true).1,
        _ => true,
    }
}

#[derive(Parser)]
struct Cli {
    /// The path to the books directory
    books: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let path = args.books;
    let (tx, rx) = channel::<(KeyCode, bool)>();
    let mut books = Books::from_dir(&path)?;
    let mut screen = Screen::new(Path::new("/dev/fb2"))?;

    let tx_buttons = tx.clone();
    thread::spawn(move || -> Option<()> {
        let mut buttons = Buttons::new().ok()?;
        loop {
            if let Ok(code) = buttons.listen() {
                println!("{code:?}");
                let _ = tx_buttons.send((code, false));
            }
        }
    });

    let mut player = Player::new()?;
    let mut only_buttons = false;
    loop {
        let Some(book) = books.get() else {
            return Err("No book available".into());
        };
        let Some(state) = book.stage_get() else {
            return Err("Invalid book state".into());
        };

        // Show the image, play the sound and wait on I/O
        println!("{state:?}");

        if !only_buttons {
            match state.image {
                Some(ref image) => {
                    let image = book.path_get().join("assets").join(&image);
                    screen.draw(&image)?;
                }
                None => screen.clear()?,
            }

            match state.audio {
                Some(ref audio) => {
                    let audio = book.path_get().join("assets").join(&audio);
                    let tx_play = tx.clone();
                    player.play(&audio, move || {
                        let code = if state.control_settings.ok {
                            KeyCode::BTN_START
                        } else if state.control_settings.home {
                            KeyCode::BTN_SELECT
                        } else {
                            return;
                        };
                        let _ = tx_play.send((code, true));
                    })?;
                }
                None => {}
            }
        }

        only_buttons = false;
        match rx.recv() {
            Ok((code, eos)) => {
                // Ignore EOS when autoplay is disabled
                if eos && !state.control_settings.autoplay {
                    only_buttons = true; // skip playing, wait only on the buttons
                } else {
                    only_buttons = process_event(&mut books, &state, code, &mut player);
                }
            }
            Err(_) => (),
        };
    }
}
