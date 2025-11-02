mod book;

use book::Book;
use std::path::Path;

fn main() {
    let path = Path::new("./story.json");

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
