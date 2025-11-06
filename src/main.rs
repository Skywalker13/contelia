mod book;
mod books;

use book::Book;
use books::Books;
use clap::Parser;
use std::error::Error;
use std::path::Path;

#[derive(Parser)]
struct Cli {
    /// The path to the books directory
    books: std::path::PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    let path = args.books;
    let mut books = Books::from_dir(&path)?;

    let path = Path::new("test/story.json");
    let mut book = Book::from_file(path)?;

    let state = book.stage_get();
    println!("State : {state:?}");
    //println!("Book  : {book:?}");

    book.button_home();
    book.button_ok();

    let state = book.stage_get();
    println!("State: {state:?}");
    //println!("Book  : {book:?}");

    book.button_wheel_right();

    let state = book.stage_get();
    println!("State: {state:?}");
    //println!("Book  : {book:?}");

    Ok(())
}
