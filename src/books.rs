use crate::book::Book;
use anyhow::{Result, bail};
use std::{fs, path::Path};

const STORY_JSON: &str = "story.json";

pub struct Books {
    books: Vec<Book>,

    current_book: usize,
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

            let story_path = path.join(STORY_JSON);
            match Book::from_file(&story_path) {
                Ok(book) => books.push(book),
                Err(e) => eprintln!("Cannot load the book {:?}: {}", story_path, e),
            }
        }

        if books.is_empty() {
            bail!("No book found");
        }

        let current_book = 0;

        Ok(Books {
            books,
            current_book,
        })
    }
}
