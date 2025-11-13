use clap::Parser;
use evdev::KeyCode;
use std::sync::mpsc::channel;
use std::{error::Error, path::Path, thread};

use contelia::{Books, Buttons, Player, Renderer};

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
    let mut renderer = Renderer::new(Path::new("/dev/fb2"))?;

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
            return Ok(());
        };
        let Some(state) = book.stage_get() else {
            return Ok(());
        };

        // Show the image, play the sound and wait on I/O
        println!("{state:?}");

        if !only_buttons {
            match state.image {
                Some(image) => {
                    let image = book.path_get().join("assets").join(&image);
                    renderer.blit(&image)?;
                }
                None => renderer.clear()?,
            }

            match state.audio {
                Some(audio) => {
                    let audio = book.path_get().join("assets").join(&audio);
                    let tx_play = tx.clone();
                    player.play(&audio, move |code| {
                        let _ = tx_play.send((code, true));
                    })?;
                }
                None => {}
            }
        }

        only_buttons = false;
        let _result = match rx.recv() {
            Ok((code, eos)) => {
                // Ignore EOS when autoplay is disabled
                if eos && !state.control_settings.autoplay {
                    only_buttons = true; // skip playing, wait only on the buttons
                    continue;
                }
                match code {
                    KeyCode::BTN_DPAD_LEFT => {
                        if state.control_settings.wheel {
                            book.button_wheel_left();
                        } else {
                            only_buttons = true;
                        }
                    }
                    KeyCode::BTN_DPAD_RIGHT => {
                        if state.control_settings.wheel {
                            book.button_wheel_right();
                        } else {
                            only_buttons = true;
                        }
                    }
                    KeyCode::BTN_SELECT => {
                        if state.control_settings.home {
                            book.button_home();
                        } else {
                            only_buttons = true;
                        }
                    }
                    KeyCode::BTN_START => {
                        if state.control_settings.ok {
                            book.button_ok();
                        } else {
                            only_buttons = true;
                        }
                    }
                    KeyCode::BTN_NORTH => {
                        if state.control_settings.pause {
                            Some(player.toggle_pause());
                        }
                        only_buttons = true;
                    }
                    _ => (),
                };
            }
            Err(_) => (),
        };
    }
}
