use crate::book::Book;
use anyhow::{Result, bail};
use std::{fs, path::Path};

pub struct Books {
    books: Vec<Book>,
    current_book_index: usize,
}

impl Books {
    pub fn from_dir(path: &Path) -> Result<Self> {
        let mut books = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            match Book::from_file(&path) {
                Ok(book) => books.push(book),
                Err(e) => eprintln!("Cannot load the book {:?}: {}", path, e),
            }
        }

        if books.is_empty() {
            bail!("No book found");
        }

        println!("Loaded {} books", books.len());

        let current_book_index = 0;

        Ok(Books {
            books,
            current_book_index,
        })
    }

    pub fn get(&mut self) -> Option<&mut Book> {
        self.books.get_mut(self.current_book_index)
    }

    pub fn button_wheel_right(&mut self) {
        let mut book_index = self.current_book_index as isize;
        book_index = book_index + 1;
        if book_index >= self.books.len() as isize {
            self.current_book_index = 0;
        } else {
            self.current_book_index = book_index as usize;
        }
    }

    pub fn button_wheel_left(&mut self) {
        let mut book_index = self.current_book_index as isize;
        book_index = book_index - 1;
        if book_index < 0 {
            self.current_book_index = self.books.len() - 1;
        } else {
            self.current_book_index = book_index as usize;
        }
    }
}
