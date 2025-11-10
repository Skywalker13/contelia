use clap::Parser;
use evdev::KeyCode;
use std::error::Error;

use contelia::{Books, Buttons};

#[derive(Parser)]
struct Cli {
    /// The path to the books directory
    books: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let path = args.books;
    let mut books = Books::from_dir(&path)?;
    let mut buttons = Buttons::new()?;

    loop {
        let Some(book) = books.get() else {
            break;
        };

        let Some(state) = book.stage_get() else {
            break;
        };

        // Show the image, play the sound and wait on I/O
        println!("{state:?}");

        let code = buttons.listen(&state.control_settings)?;
        println!("{code:?}");

        let _result = match code {
            KeyCode::BTN_DPAD_LEFT => book.button_wheel_left(),
            KeyCode::BTN_DPAD_RIGHT => book.button_wheel_right(),
            // KeyCode::BTN_NORTH => ,
            KeyCode::BTN_SELECT => book.button_home(),
            KeyCode::BTN_START => book.button_ok(),
            _ => None,
        };
    }

    Ok(())
}
