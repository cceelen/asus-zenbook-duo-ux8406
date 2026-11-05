//! The tester at the terminal: what the tool says, and the two answers it
//! reads, Enter and skip.

use std::io::{BufRead, Write};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Mutex, PoisonError};

/// How the tool talks to the tester.
pub trait Console {
    /// Say a line.
    fn say(&self, text: &str);

    /// Ask a question; the tester answers with Enter (yes) or `s` (no).
    fn confirm(&self, question: &str) -> bool;

    /// Whether the tester typed `s` and Enter since the last call.
    fn skip_requested(&self) -> bool;
}

/// Whether a line of input means yes.
fn is_yes(line: &str) -> bool {
    matches!(line.trim(), "" | "y" | "Y" | "yes")
}

/// Whether a line of input means skip.
fn is_skip(line: &str) -> bool {
    line.trim().eq_ignore_ascii_case("s")
}

/// A terminal: lines in, lines out. A thread reads the input, so that the
/// tool can look for `s` while it waits for the machine.
pub struct Terminal {
    input: Mutex<Receiver<String>>,
    output: Mutex<Box<dyn Write + Send>>,
}

impl Terminal {
    /// A terminal that reads `input` and writes `output`.
    #[must_use]
    pub fn new(input: Box<dyn BufRead + Send>, output: Box<dyn Write + Send>) -> Self {
        let (sender, receiver) = channel();

        std::thread::spawn(move || {
            for line in input.lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            input: Mutex::new(receiver),
            output: Mutex::new(output),
        }
    }
}

impl Console for Terminal {
    fn say(&self, text: &str) {
        let mut output = self.output.lock().unwrap_or_else(PoisonError::into_inner);

        // A terminal that cannot be written to has no one to tell.
        let _ = writeln!(output, "{text}").and_then(|()| output.flush());
    }

    fn confirm(&self, question: &str) -> bool {
        self.say(&format!("{question} [Enter = yes, s = no]"));
        // The end of the input is a no.
        self.input
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .recv()
            .is_ok_and(|line| is_yes(&line))
    }

    fn skip_requested(&self) -> bool {
        let input = self.input.lock().unwrap_or_else(PoisonError::into_inner);
        let mut skip = false;

        while let Ok(line) = input.try_recv() {
            skip |= is_skip(&line);
        }
        skip
    }
}
