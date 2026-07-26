/* Contelia
 * Copyright (C) 2025-2026  Mathieu Schroeter <mathieu@schroetersa.ch>
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
use futures_lite::future::block_on;
use signal_hook::{consts::*, iterator::Signals};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::channel;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use std::{error::Error, thread};

use contelia::{
    Bluez, Books, Buttons, ControlSettings, Device, DeviceKind, Player, Screen, Services, Stage,
    Status, Timeout,
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

    AccessPoint,

    BluetoothStart,
    BluetoothStop,
    BluetoothScan,
    BluetoothSelect,
    BluetoothOk,

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
        KeyCode::BTN_DPAD_UP => {
            player.volume_up();
            Next::Volume
        }
        KeyCode::BTN_DPAD_DOWN => {
            player.volume_down();
            Next::Volume
        }
        KeyCode::BTN_SELECT => {
            if state.square_one {
                Next::None
            } else {
                book.button_home();
                Next::Normal
            }
        }
        KeyCode::BTN_START => {
            book.button_ok();
            Next::Normal
        }
        _ => Next::Timeout,
    }
}

fn bt_new_scan_spawn(
    tx: std::sync::mpsc::Sender<(KeyCode, Option<Status>, bool, Option<Device>)>,
    cancel: Arc<AtomicBool>,
) {
    cancel.store(false, Ordering::Relaxed);

    println!("Start bluetooth scanning");
    thread::spawn(move || {
        loop {
            let result = block_on(async {
                let bluez = Bluez::new().await?;
                /* Remove all devices before a new scan (reset) */
                bluez.remove_all_devices().await?;
                bluez
                    .scan_audio_devices(tx.clone(), Arc::clone(&cancel))
                    .await
            });

            if cancel.load(Ordering::Relaxed) {
                println!("Stop bluetooth scanning");
                break;
            }

            if let Err(e) = result {
                eprintln!("Bluetooth scan error, retry: {}", e);
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                println!("Stop bluetooth scanning");
                break;
            }
        }
    });
}

#[derive(Parser)]
struct Cli {
    /// Framebuffer device
    #[arg(short, long, default_value = "/dev/fb2")]
    fb: PathBuf,

    /// Main buttons input device
    #[arg(short, long, default_value = "/dev/input/tftbonnet13")]
    input: PathBuf,

    /// Timeout before poweroff
    #[arg(short, long, default_value = "20")]
    timeout: u64,

    /// The path to the books directory
    books: std::path::PathBuf,
}

fn run() -> Result<u8, Box<dyn Error>> {
    let args = Cli::parse();
    let (tx, rx) = channel::<(KeyCode, Option<Status>, bool, Option<Device>)>();

    //// Listen for signals ////////////////////////////////////////////////////
    let mut signals = Signals::new(&[SIGTERM, SIGINT])?;
    let tx_sig = tx.clone();
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGTERM | SIGINT => {
                    println!("{sig:?}");
                    let _ = tx_sig.send((KeyCode::KEY_END, None, false, None));
                }
                _ => unreachable!(),
            }
        }
    });

    //// Listen for main buttons ///////////////////////////////////////////////
    let input = args.input;
    let tx_main_buttons = tx.clone();
    thread::spawn(move || -> Option<()> {
        let mut buttons = Buttons::new(input.as_path()).ok()?;
        loop {
            if let Ok(code) = buttons.listen() {
                let status = buttons.status().clone();
                println!("{code:?}: {:?}", status);
                let _ = tx_main_buttons.send((code, Some(status), false, None));
            }
        }
    });

    //// Check if a bluetooth device is already paired /////////////////////////
    // TODO: check in /usr/var/lib/bluetooth/*/*/info

    let path = args.books;
    let fb = args.fb;
    let mut books = Books::from_dir(&path)?;
    let mut screen = Screen::new(fb.as_path())?;
    let mut player = Player::new()?;
    let mut device: Option<Device> = None;
    let mut next = Next::Normal;
    let mut timeout: Option<Timeout> = None;
    let mut access_point = false;
    let mut bluetooth = false;
    let mut player_reload = true;
    let mut status_code = 0;
    let mut inactivity: Option<Timeout> = None;
    let mut bt_connected = false;
    let bt_cancel = Arc::new(AtomicBool::new(false));

    let mut assets_dir = env::current_exe()?;
    assets_dir.pop();
    assets_dir.pop();
    assets_dir = assets_dir.join("share/contelia/assets");

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

        if next == Next::Audio {
            if !player_reload {
                let mut connected = false;
                let running = Services::status_bluez()?;
                if running {
                    let is_connected: Result<bool, Box<dyn std::error::Error>> = block_on(async {
                        let bluez = Bluez::new().await?;
                        bluez.device_is_connected().await
                    });
                    if let Ok(is_connected) = is_connected {
                        connected = is_connected;
                    }

                    if connected != bt_connected {
                        bt_connected = connected;
                        player_reload = true;
                    }

                    /* Stop the bluetooth when not connected anymore */
                    if !bt_connected {
                        let _ = block_on(async {
                            let bluez = Bluez::new().await?;
                            bluez.set_powered(false).await
                        });
                        Services::down_bluez()?;
                    }
                } else if bt_connected {
                    bt_connected = false;
                    player_reload = true;
                }
            }
        }

        if next == Next::Normal || next == Next::Audio {
            match state.audio {
                Some(ref audio) => {
                    let audio = book.audio_file_get(&audio)?;
                    let tx_play = tx.clone();
                    player.play(audio, player_reload, move || {
                        let code = if state.control_settings.ok || state.control_settings.autoplay {
                            KeyCode::BTN_START
                        } else if state.control_settings.home {
                            KeyCode::BTN_SELECT
                        } else {
                            return;
                        };
                        let _ = tx_play.send((code, None, true, None));
                    })?;
                    player_reload = false;
                }
                None => {}
            }
        }

        //// Access Point //////////////////////////////////////////////////////

        if next == Next::AccessPoint && access_point {
            if Services::down_ap().is_ok() {
                access_point = false;
                books.reload();
                next = Next::Normal;
                continue; /* Restore image and/or audio */
            }
        }

        if next == Next::AccessPoint {
            player.stop();

            if Services::up_ap().is_ok() {
                access_point = true;
                screen.draw_file(assets_dir.join("settings.png"))?;
            }
        }

        //// Bluetooth /////////////////////////////////////////////////////////

        if next == Next::BluetoothStop {
            bt_cancel.store(true, Ordering::Relaxed);

            let _ = block_on(async {
                let bluez = Bluez::new().await?;
                bluez.remove_all_devices().await
            });

            Services::down_bluez()?;

            bluetooth = false;
            next = Next::Normal;
            continue; /* Restore image and/or audio */
        }

        if next == Next::BluetoothStart {
            player.stop();
            bluetooth = true;
            next = Next::BluetoothScan;
            continue;
        }

        if next == Next::BluetoothScan {
            bt_cancel.store(true, Ordering::Relaxed);

            screen.draw_file(assets_dir.join("wait.png"))?;
            Services::start_bluez()?;

            screen.draw_file(assets_dir.join("bt_scan.png"))?;
            let tx_bluetooth = tx.clone();
            bt_new_scan_spawn(tx_bluetooth, Arc::clone(&bt_cancel));
        }

        if next == Next::BluetoothSelect {
            match device {
                Some(ref device) => match device.kind {
                    DeviceKind::Headset => {
                        screen.draw_file(assets_dir.join("bt_headset.png"))?;
                    }
                    DeviceKind::Headphones => {
                        screen.draw_file(assets_dir.join("bt_headphone.png"))?;
                    }
                    DeviceKind::Speaker | DeviceKind::Portable => {
                        screen.draw_file(assets_dir.join("bt_speaker.png"))?;
                    }
                    DeviceKind::Car => {
                        screen.draw_file(assets_dir.join("bt_car.png"))?;
                    }
                    DeviceKind::Unknown => {
                        screen.draw_file(assets_dir.join("bt_unknown.png"))?;
                    }
                },
                None => {}
            }
        }

        if next == Next::BluetoothOk {
            match device {
                Some(ref device) => {
                    screen.draw_file(assets_dir.join("wait.png"))?;
                    bt_cancel.store(true, Ordering::Relaxed);

                    println!("Connect to {}", device.name);
                    let result = block_on(async move {
                        let bluez = Bluez::new().await?;
                        bluez.connect(&device.path).await
                    });

                    if let Err(e) = result {
                        println!("Connection error with {}: {}", device.name, e);

                        next = Next::BluetoothScan;
                        continue;
                    } else {
                        println!("Connected to {}", device.name);

                        bt_cancel.store(true, Ordering::Relaxed);
                        bluetooth = false;
                        player_reload = true;
                        next = Next::Normal;
                        continue;
                    }
                }
                None => {}
            }
        }

        //// Volume ////////////////////////////////////////////////////////////

        if next == Next::Volume {
            let volume = player.get_volume();

            if volume > 0 {
                screen.draw_file(assets_dir.join(format!("volume{:0>2}.png", volume)))?;
            }

            let tx_timeout = tx.clone();
            timeout = Some(Timeout::set(Duration::from_millis(800), move || {
                let _ = tx_timeout.send((KeyCode::KEY_TIME, None, true, None));
            }));
        }

        //// Pause - Play //////////////////////////////////////////////////////

        if next == Next::Pause || next == Next::Play {
            let image = if next == Next::Play {
                assets_dir.join("play.png")
            } else {
                assets_dir.join("pause.png")
            };
            screen.draw_file(image)?;

            let tx_timeout = tx.clone();
            timeout = Some(Timeout::set(Duration::from_millis(800), move || {
                let _ = tx_timeout.send((KeyCode::KEY_TIME, None, true, None));
            }));
        }

        //// Inactivity - Timeout //////////////////////////////////////////////

        /* When the screen is not cleared by using the menu excepted for the
         * settings, then a timeout of 20s is started and the system is
         * poweroff when the timeout is reached (status code 42).
         */
        if !screen.is_cleared() && !bluetooth && !access_point {
            println!("Start inactivity timeout for {}s", args.timeout);
            if let Some(ref mut inactivity) = inactivity {
                inactivity.clear();
            }
            let tx_poweroff = tx.clone();
            inactivity = Some(Timeout::set(Duration::from_secs(args.timeout), move || {
                let _ = tx_poweroff.send((KeyCode::KEY_POWER, None, true, None));
            }))
        } else if inactivity.is_some() {
            if let Some(ref mut inactivity) = inactivity {
                println!("Stop inactivity timeout");
                inactivity.clear();
            }
            inactivity = None;
        }

        //// Main events loop //////////////////////////////////////////////////

        next = Next::Normal;
        match rx.recv() {
            Ok((code, status, eos, bt_devices)) => {
                if let Some(status) = status {
                    if status.dpad_down && status.select && status.start && !bluetooth {
                        next = Next::AccessPoint;
                        continue;
                    }
                    if status.dpad_up && status.select && status.start && !access_point {
                        next = if bluetooth {
                            Next::BluetoothStop
                        } else {
                            Next::BluetoothStart
                        };
                        continue;
                    }
                    let just_one =
                        (status.select && !status.start) || (!status.select && status.start);
                    if (status.select && status.start)
                        /* Ignore to prevent menu navigation */
                        || (status.dpad_down && just_one)
                        || (status.dpad_up && just_one)
                    {
                        next = Next::None;
                        continue;
                    }
                }

                if code == KeyCode::KEY_END {
                    next = Next::Shutdown; /* Clean shutdown */
                } else if code == KeyCode::KEY_POWER {
                    next = Next::Shutdown;
                    status_code = 42; /* Poweroff */
                } else if bluetooth == true && code == KeyCode::KEY_BLUETOOTH {
                    match bt_devices {
                        Some(dev) => {
                            device = Some(dev);
                            next = Next::BluetoothSelect;
                        }
                        None => {
                            next = Next::None;
                            continue;
                        }
                    }
                } else if access_point == true {
                    next = Next::None;
                } else if bluetooth == true {
                    if code == KeyCode::BTN_START && next != Next::BluetoothOk {
                        next = Next::BluetoothOk;
                    } else {
                        next = Next::None;
                    }
                } else if code == KeyCode::KEY_TIME {
                    next = Next::Image; /* Restore screen */
                } else if eos && !state.control_settings.autoplay {
                    /* Ignore EOS when autoplay is disabled */
                    next = if timeout.is_none() {
                        Next::Image
                    } else {
                        Next::Timeout
                    };
                } else {
                    next = process_event(&mut books, &mut player, &state, code, eos);
                }
            }
            Err(_) => (),
        };
    }

    if next == Next::Shutdown {
        screen.off()?;
        screen.clear()?;
    }

    Ok(status_code)
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
