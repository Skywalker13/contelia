mod book;
mod books;

use book::Book;
use books::Books;
use std::path::Path;

fn main() {
    let path = Path::new("./books");
    let mut books = Books::from_dir(path).expect("Load error");

    let path = Path::new("test/story.json");
    let mut book = Book::from_file(path).expect("Erreur lors du chargement");

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
}
