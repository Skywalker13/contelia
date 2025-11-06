use clap::Parser;
use std::error::Error;
use std::path::Path;

use contelia::Book;
use contelia::Books;

#[derive(Parser)]
struct Cli {
    /// The path to the books directory
    books: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let path = args.books;
    let mut books = Books::from_dir(&path)?;

    loop {
        let Some(book) = books.get() else {
            break;
        };

        let Some(state) = book.stage_get() else {
            break;
        };

        // Show the image, play the sound and wait on I/O
        println!("{state:?}");
        break;
    }

    Ok(())
}
