//! Command prompt for xi-term. currently this is
//! heavily inspired by vim and is just designed to
//! get a simple base to work off of.

use std::io::Error;
use std::io::Write;
use std::ops::RangeBounds;
use termion::event::{Event, Key};

use core::{Command, ParseCommandError};
use termion::clear::CurrentLine as ClearLine;
use termion::cursor::Goto;

#[derive(Debug, Default)]
pub struct CommandPrompt {
    dex: usize,
    chars: String,
}

impl CommandPrompt {
    /// Process a terminal event for the command prompt.
    pub fn handle_input(&mut self, input: &Event) -> Option<Command> {
        match input {
            Event::Key(Key::Char('\n')) => self.finalize(),
            Event::Key(Key::Backspace) | Event::Key(Key::Ctrl('h')) => self.back(),
			Event::Key(Key::Ctrl('w')) => self.back_word(),
            Event::Key(Key::Delete) => self.delete(),
            Event::Key(Key::Left) => self.left(),
            Event::Key(Key::Right) => self.right(),
            Event::Key(Key::Down) => Some(Command::MoveDown),
            Event::Key(Key::Up) => Some(Command::MoveUp),
            Event::Key(Key::Char(chr)) => self.new_key(*chr),
            _ => None,
        }
    }

    fn left(&mut self) -> Option<Command> {
        if self.dex > 0 {
            self.dex -= 1;
        }
        Some(Command::MoveLeft)
    }

    fn right(&mut self) -> Option<Command> {
        if self.dex < self.chars.len() {
            self.dex += 1;
        }
        Some(Command::MoveRight)
    }

    fn delete(&mut self) -> Option<Command> {
        if self.dex < self.chars.len() {
            self.chars.remove(self.dex);
        }
        None
    }

	fn back_word(&mut self) -> Option<Command> {
		if !self.chars.is_empty() {
			let mut bytes: Vec<u8> = self.chars.bytes().collect();

			let mut removed_chars = 0;

			for i in (0..self.dex).rev() {
				if let Some(ch) = bytes.get(i) {
					if ch == &b' ' {
						bytes.remove(i);
						removed_chars += 1;
					} else {
						break;
					}
				}
			}

			let first_space = bytes
				.drain(..self.dex - removed_chars)
				.rposition(|ch| ch == b' ')
				.map_or(0, |x| x + 1);

			self.chars.replace_range(first_space..self.dex, "");

			self.dex = first_space;
		}
		None
	}

    fn back(&mut self) -> Option<Command> {
        if !self.chars.is_empty() {
            self.dex -= 1;
            self.chars.remove(self.dex);
        }

        None
    }

    /// Gets called when any character is pressed.
    fn new_key(&mut self, chr: char) -> Option<Command> {
        self.chars.insert(self.dex, chr);
        self.dex += 1;
        None
    }

    /// Gets called when return is pressed,
    fn finalize(&mut self) -> Option<Command> {
        Some(Command::Out(self.chars.clone()))
    }

    pub fn render<W: Write>(&mut self, w: &mut W, row: u16) -> Result<(), Error> {
        if let Err(err) = write!(w, "{}{}:{}{}", Goto(1, row), ClearLine, self.chars, Goto(self.dex as u16+2, row)) {
            error!("faile to render status bar: {:?}", err);
        }
        Ok(())
    }
}
