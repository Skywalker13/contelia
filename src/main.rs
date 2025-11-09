use clap::Parser;
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
    let mut gpio = Buttons::new()?;

    loop {
        let Some(book) = books.get() else {
            break;
        };

        let Some(state) = book.stage_get() else {
            break;
        };

        // Show the image, play the sound and wait on I/O
        println!("{state:?}");

        // Test listening on GPIO
        let code = gpio.listen()?;
        println!("{code:?}");
    }

    Ok(())
}
