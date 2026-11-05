//! The results on the partition: `report.md` with one table row for each
//! check, and the raw logs in `raw/`. Each row is written when the check
//! ends, so that a crash keeps what was done.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The outcome of a check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// As expected.
    Ok,
    /// Not as expected.
    Fail,
    /// Not done: the tester skipped it, or it cannot be done here.
    Skip,
}

impl fmt::Display for Status {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Ok => "OK",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
        })
    }
}

/// Who does the check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The tool alone.
    Auto,
    /// The tester acts, the tool decides.
    Guided,
}

impl fmt::Display for Kind {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::Auto => "automatic",
            Self::Guided => "guided",
        })
    }
}

/// How many checks ended with each status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Checks that were as expected.
    pub ok: usize,
    /// Checks that were not.
    pub fail: usize,
    /// Checks not done.
    pub skip: usize,
}

/// The results directory.
pub struct Report {
    dir: PathBuf,
    counts: Counts,
    /// The first write that failed.
    broken: Option<String>,
}

impl Report {
    /// Start the results in `dir`. A report from an earlier run moves to
    /// `<dir>.previous`.
    ///
    /// # Errors
    ///
    /// The directory cannot be made.
    pub fn open(dir: &Path, title: &str) -> std::io::Result<Self> {
        if dir.join("report.md").exists() {
            let previous = dir.with_file_name("results.previous");

            let _ = fs::remove_dir_all(&previous);
            fs::rename(dir, previous)?;
        }
        fs::create_dir_all(dir.join("raw"))?;

        let mut report = Self {
            dir: dir.to_owned(),
            counts: Counts::default(),
            broken: None,
        };

        report.append("report.md", &format!("# {title}\n"));
        Ok(report)
    }

    fn append(&mut self, name: &str, text: &str) {
        let written = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(name))
            .and_then(|mut file| file.write_all(text.as_bytes()));

        if let Err(error) = written {
            self.broken.get_or_insert(format!("{name}: {error}"));
        }
    }

    /// A heading, and the head of a new table.
    pub fn section(&mut self, title: &str) {
        self.append(
            "report.md",
            &format!(
                "\n## {title}\n\n| Check | Mode | Result | Detail |\n| --- | --- | --- | --- |\n"
            ),
        );
    }

    /// A line of text between the tables.
    pub fn note(&mut self, text: &str) {
        self.append("report.md", &format!("\n{text}\n"));
    }

    /// One row.
    pub fn add(&mut self, check: &str, kind: Kind, status: Status, detail: &str) {
        match status {
            Status::Ok => self.counts.ok += 1,
            Status::Fail => self.counts.fail += 1,
            Status::Skip => self.counts.skip += 1,
        }
        self.append(
            "report.md",
            &format!(
                "| {} | {kind} | {status} | {} |\n",
                cell(check),
                cell(detail)
            ),
        );
    }

    /// Save a raw log under `raw/`.
    pub fn raw(&mut self, name: &str, text: &str) {
        let path = self.dir.join("raw").join(name);
        let written = fs::write(&path, text);

        if let Err(error) = written {
            self.broken
                .get_or_insert(format!("{}: {error}", path.display()));
        }
    }

    /// The directory of the results.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The counts so far.
    #[must_use]
    pub const fn counts(&self) -> Counts {
        self.counts
    }

    /// The first write that failed, if one did.
    #[must_use]
    pub fn broken(&self) -> Option<&str> {
        self.broken.as_deref()
    }
}

/// A text for a table cell: one line, without the column separator.
fn cell(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('|', "/")
}
