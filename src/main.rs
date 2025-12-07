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
use signal_hook::{consts::*, iterator::Signals};
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{error::Error, thread};

use contelia::{
    Books, Buttons, ControlSettings, FileReader, Player, Screen, Stage, Status, Timeout,
};

#[derive(Debug, PartialEq)]
enum Next {
    None,
    Normal,
    Image,
    Audio,
    Volume,
    Pause,
    Play,
    Timeout,
    Settings,
    Shutdown,
}

fn is_key_enabled(control_settings: &ControlSettings, code: KeyCode) -> bool {
    match code {
        KeyCode::BTN_DPAD_LEFT => control_settings.wheel,
        KeyCode::BTN_DPAD_RIGHT => control_settings.wheel || control_settings.pause,
        KeyCode::BTN_DPAD_UP | KeyCode::BTN_DPAD_DOWN => true, // volume
        KeyCode::BTN_SELECT => control_settings.home,
        KeyCode::BTN_START => control_settings.ok,
        _ => false,
    }
}

/// Process the event and returns true is we want to skip the assets
fn process_event(
    books: &mut Books,
    player: &mut Player,
    state: &Stage,
    code: KeyCode,
    autoplay: bool,
    status: Option<&Status>,
) -> Next {
    /* In case of autoplay or square_one, we ignore the button settings */
    if !autoplay && !state.square_one && !is_key_enabled(&state.control_settings, code) {
        return Next::Timeout;
    }
    let Some(book) = books.get() else {
        return Next::Timeout;
    };
    match code {
        KeyCode::BTN_DPAD_LEFT => {
            if state.square_one {
                books.button_wheel_left();
            } else {
                book.button_wheel_left();
            }
            Next::Normal
        }
        KeyCode::BTN_DPAD_RIGHT => {
            if state.square_one {
                books.button_wheel_right();
                return Next::Normal;
            }
            if state.control_settings.wheel {
                book.button_wheel_right();
                return Next::Normal;
            }
            if state.control_settings.pause {
                player.toggle_pause();
                if player.is_paused() {
                    return Next::Pause;
                }
                return Next::Play;
            }
            Next::Normal
        }
        KeyCode::BTN_DPAD_UP => (player.volume_up(), Next::Volume).1,
        KeyCode::BTN_DPAD_DOWN => {
            match status {
                Some(status) => {
                    if status.select {
                        return Next::Settings;
                    }
                }
                None => {}
            }

            player.volume_down();
            Next::Volume
        }
        KeyCode::BTN_SELECT => {
            if state.square_one {
                return Next::None;
            }
            book.button_home();
            Next::Normal
        }
        KeyCode::BTN_START => {
            book.button_ok();
            Next::Normal
        }
        _ => Next::Timeout,
    }
}

#[derive(Parser)]
struct Cli {
    /// Framebuffer device
    #[arg(short, long, default_value = "/dev/fb2")]
    fb: PathBuf,

    /// Input device
    #[arg(short, long, default_value = "/dev/input/tftbonnet13")]
    input: PathBuf,

    /// The path to the books directory
    books: std::path::PathBuf,
}

fn run() -> Result<u8, Box<dyn Error>> {
    let args = Cli::parse();

    let path = args.books;
    let fb = args.fb;
    let input = args.input;

    let (tx, rx) = channel::<(KeyCode, Option<Status>, bool)>();

    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;
    let tx_sig = tx.clone();
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGTERM | SIGINT => {
                    println!("{sig:?}");
                    let _ = tx_sig.send((KeyCode::KEY_END, None, false));
                }
                _ => unreachable!(),
            }
        }
    });

    let mut books = Books::from_dir(&path)?;
    let mut screen = Screen::new(fb.as_path())?;

    let tx_buttons = tx.clone();
    thread::spawn(move || -> Option<()> {
        let mut buttons = Buttons::new(input.as_path()).ok()?;
        loop {
            if let Ok(code) = buttons.listen() {
                let status = buttons.status().clone();
                println!("{code:?}: {:?}", status);
                let _ = tx_buttons.send((code, Some(status), false));
            }
        }
    });

    let mut player = Player::new()?;
    let mut next = Next::Normal;
    let mut timeout: Option<Timeout> = None;
    let mut settings = false;

    while next != Next::Shutdown {
        let Some(book) = books.get() else {
            return Err("No book available".into());
        };
        let Some(state) = book.stage_get() else {
            return Err("Invalid book state".into());
        };

        if next != Next::Timeout {
            if let Some(ref mut timeout) = timeout {
                timeout.clear();
            }
        }

        // Show the image, play the sound and wait on I/O
        println!("{state:?}");
        println!("{next:?}");

        if next == Next::Normal || next == Next::Image {
            match state.image {
                Some(ref image) => {
                    let (mut image, format) = book.images_file_get(&image)?;
                    screen.draw(&mut image, format)?;
                    screen.on()?;
                }
                None => {
                    screen.off()?;
                    screen.clear()?;
                }
            }
        }
        if next == Next::Normal || next == Next::Audio {
            match state.audio {
                Some(ref audio) => {
                    let audio = book.audio_file_get(&audio)?;
                    let tx_play = tx.clone();
                    player.play(audio, move || {
                        let code = if state.control_settings.ok || state.control_settings.autoplay {
                            KeyCode::BTN_START
                        } else if state.control_settings.home {
                            KeyCode::BTN_SELECT
                        } else {
                            return;
                        };
                        let _ = tx_play.send((code, None, true));
                    })?;
                }
                None => {}
            }
        }
        if next == Next::Volume {
            let volume = player.get_volume();
            let mut image = env::current_exe()?;
            image.pop();
            image.pop();
            let image = image
                .join("share/contelia/assets")
                .join(format!("volume{:0>2}.png", volume));

            let path = Path::new(&image);
            println!("volume image: {}", path.to_string_lossy().to_string());
            let mut file = FileReader::Plain(File::open(path)?);
            screen.draw(&mut file, image::ImageFormat::Png)?;
            screen.on()?;

            let tx_timeout = tx.clone();
            timeout = Some(Timeout::set(Duration::from_millis(800), move || {
                let _ = tx_timeout.send((KeyCode::KEY_TIME, None, true));
            }));
        }
        if next == Next::Pause || next == Next::Play {
            let mut image = env::current_exe()?;
            image.pop();
            image.pop();
            image = image.join("share/contelia/assets");
            if next == Next::Play {
                image = image.join("play.png");
            } else {
                image = image.join("pause.png");
            }

            let path = Path::new(&image);
            println!("play/pause image: {}", path.to_string_lossy().to_string());
            let mut file = FileReader::Plain(File::open(path)?);
            screen.draw(&mut file, image::ImageFormat::Png)?;
            screen.on()?;

            let tx_timeout = tx.clone();
            timeout = Some(Timeout::set(Duration::from_millis(800), move || {
                let _ = tx_timeout.send((KeyCode::KEY_TIME, None, true));
            }));
        }
        if next == Next::Settings {
            settings = true;
            player.stop();

            let mut image = env::current_exe()?;
            image.pop();
            image.pop();
            image = image.join("share/contelia/assets");
            image = image.join("settings.png");

            let path = Path::new(&image);
            println!("settings image: {}", path.to_string_lossy().to_string());
            let mut file = FileReader::Plain(File::open(path)?);
            screen.draw(&mut file, image::ImageFormat::Png)?;
            screen.on()?;
        }

        next = Next::Normal;
        match rx.recv() {
            Ok((code, status, eos)) => {
                if code == KeyCode::KEY_END {
                    next = Next::Shutdown; // Clean shutdown
                } else if settings == true {
                    next = Next::None;
                } else if code == KeyCode::KEY_TIME {
                    next = Next::Image; // Restore screen
                } else if eos && !state.control_settings.autoplay {
                    // Ignore EOS when autoplay is disabled
                    if timeout.is_none() {
                        next = Next::Image;
                    } else {
                        next = Next::Timeout;
                    }
                } else {
                    next =
                        process_event(&mut books, &mut player, &state, code, eos, status.as_ref());
                }
            }
            Err(_) => (),
        };
    }

    if next == Next::Shutdown {
        screen.off()?;
        screen.clear()?;
    }

    Ok(0)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("Error : {}", e);
            ExitCode::FAILURE
        }
    }
}
