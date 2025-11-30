/* Contelia
 * Copyright (C) 2025  Mathieu Schroeter <mathieu@schroetersa.ch>
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

use crate::book::{Book, book::Source};
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

            let source;

            if Book::is_story_json(&path) {
                source = Source::StoryJson(&path);
            } else if Book::is_story_pack(&path) {
                source = Source::StoryPack(&path);
            } else {
                continue;
            }

            match Book::from_source(source) {
                Ok(book) => books.push(book),
                Err(e) => eprintln!("Cannot load the book {:?}: {}", path, e),
            }
        }

        if books.is_empty() {
            bail!("No book found");
        }

        println!("Loaded {} books", books.len());

        let current_book_index = 0;

        Ok(Self {
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
