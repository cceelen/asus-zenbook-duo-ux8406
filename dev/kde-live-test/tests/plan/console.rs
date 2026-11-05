//! The terminal.

use std::io::Write;
use std::sync::{Arc, Mutex};

use asus_ux8406_kde_live_test::console::{Console, Terminal};

struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn terminal(input: &'static str) -> (Terminal, Arc<Mutex<Vec<u8>>>) {
    let output = Arc::new(Mutex::new(Vec::new()));

    (
        Terminal::new(
            Box::new(input.as_bytes()),
            Box::new(Sink(Arc::clone(&output))),
        ),
        output,
    )
}

#[test]
fn it_says_lines() {
    let (terminal, output) = terminal("");

    terminal.say("hello");
    assert_eq!(output.lock().unwrap().as_slice(), b"hello\n");
}

#[test]
fn enter_and_yes_are_yes_and_everything_else_is_no() {
    let (terminal, output) = terminal("\ny\nyes\ns\nno\n");

    assert!(terminal.confirm("one?"));
    assert!(terminal.confirm("two?"));
    assert!(terminal.confirm("three?"));
    assert!(!terminal.confirm("four?"));
    assert!(!terminal.confirm("five?"));
    // The end of the input is a no.
    assert!(!terminal.confirm("six?"));
    assert!(
        String::from_utf8_lossy(&output.lock().unwrap()).contains("one? [Enter = yes, s = no]")
    );
}

#[test]
fn skip_is_seen_once() {
    let (terminal, _) = terminal("x\nS\n");

    // The reader thread needs a moment.
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);

    while !terminal.skip_requested() {
        assert!(std::time::Instant::now() < end);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!terminal.skip_requested());
}

#[test]
fn nothing_typed_is_no_skip() {
    let (terminal, _) = terminal("");

    assert!(!terminal.skip_requested());
}

#[test]
fn a_terminal_that_is_gone_ends_the_reader() {
    use std::io::BufReader;
    use std::os::unix::net::UnixStream;

    let (reader, mut writer) = UnixStream::pair().unwrap();
    let terminal = Terminal::new(
        Box::new(BufReader::new(reader)),
        Box::new(Sink(Arc::default())),
    );

    drop(terminal);
    // The reader thread wakes with a line and finds no one to give it to.
    writer.write_all(b"late\n").unwrap();
    drop(writer);
    std::thread::sleep(std::time::Duration::from_millis(100));
}
