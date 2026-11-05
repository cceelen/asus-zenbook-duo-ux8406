//! The file of the program in the configuration directory of the user:
//! whether it turns the screens, and KWin's auto-rotate policy of each
//! built-in screen from before the program set it to `never`.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The name of the file.
const NAME: &str = "plasma-builtin-screen-rotation.toml";

/// Why the file could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The file could not be read or written.
    #[error("{}: {source}", path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// What the system said.
        source: std::io::Error,
    },
    /// The file is not a configuration of this program.
    #[error("{}: {source}", path.display())]
    Format {
        /// The file.
        path: PathBuf,
        /// What is wrong in it.
        source: toml::de::Error,
    },
    /// There is no configuration directory.
    #[error("neither XDG_CONFIG_HOME nor HOME is set")]
    NoDirectory,
}

fn enabled() -> bool {
    true
}

/// What the file holds.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Whether the screens follow the orientation of the laptop.
    #[serde(default = "enabled")]
    pub enabled: bool,
    /// KWin's auto-rotate policy of each built-in screen from before the
    /// program set it to `never`, by connector.
    #[serde(default)]
    pub policies: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            policies: BTreeMap::new(),
        }
    }
}

/// Where the file is: in `$XDG_CONFIG_HOME`, else in `$HOME/.config`.
///
/// # Errors
///
/// [`Error::NoDirectory`] where neither is set.
pub fn path() -> Result<PathBuf, Error> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))
        .map(|dir| dir.join(NAME))
        .ok_or(Error::NoDirectory)
}

/// The configuration in the file; the defaults where there is no file.
///
/// # Errors
///
/// The file cannot be read, or it is not a configuration of this program.
pub fn load(path: &Path) -> Result<Config, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => toml::from_str(&text).map_err(|source| Error::Format {
            path: path.to_owned(),
            source,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(source) => Err(Error::Io {
            path: path.to_owned(),
            source,
        }),
    }
}

/// Write the configuration: to a new file next to it first, which then takes
/// its place. The directory is made if need be.
///
/// # Errors
///
/// The file cannot be written.
pub fn save(path: &Path, config: &Config) -> Result<(), Error> {
    // The path ends in the name of the file, so it has a directory; a bare
    // name is in the current one.
    let dir = path.parent().unwrap_or(Path::new("."));
    let io = |source| Error::Io {
        path: path.to_owned(),
        source,
    };
    // The serialisation cannot fail for these types (a flag and a table of
    // strings), so the default is never the text that gets written.
    let text = toml::to_string(config).unwrap_or_default();

    std::fs::create_dir_all(dir)
        .and_then(|()| tempfile::NamedTempFile::new_in(dir))
        .and_then(|mut file| file.write_all(text.as_bytes()).map(|()| file))
        .and_then(|file| file.persist(path).map_err(std::io::Error::from))
        .map(drop)
        .map_err(io)
}
