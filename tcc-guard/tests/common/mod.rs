//! What the tests share: a machine in a temporary directory, copied from
//! `tests/fixtures/`.

// Each test program uses its own part of this.
#![allow(dead_code)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use assert_fs::TempDir;
use assert_fs::prelude::*;
use asus_ux8406_tcc_guard::paths::Paths;
use rstest::fixture;
use rustix::fs::{CWD, FileType, Mode, mknodat};

/// The offset of the first package, below the root.
pub const OFFSET: &str = "sys/bus/pci/devices/0000:00:04.0/tcc_offset_degree_celsius";
/// The offset of the second package.
pub const OFFSET_TWO: &str = "sys/bus/pci/devices/0000:80:04.0/tcc_offset_degree_celsius";
/// The temperature of the first package.
pub const TEMP: &str = "sys/class/hwmon/hwmon1/temp1_input";
/// The temperature of the second package.
pub const TEMP_TWO: &str = "sys/class/hwmon/hwmon5/temp1_input";
/// The hwmon device of the first package.
pub const HWMON: &str = "sys/class/hwmon/hwmon1";
pub const PRODUCT_NAME: &str = "sys/class/dmi/id/product_name";
pub const CONFIG: &str = "etc/asus-ux8406-tcc-guard.toml";
pub const KMSG: &str = "dev/kmsg";
pub const STATE: &str = "run/asus-ux8406-tcc-guard";
/// The lines that tell a machine with two packages where their offsets are.
pub const CONFIG_TWO: &str = "\n[[package]]\nid = 0\n\
     offset = \"/sys/bus/pci/devices/0000:00:04.0/tcc_offset_degree_celsius\"\n\
     [[package]]\nid = 1\n\
     offset = \"/sys/bus/pci/devices/0000:80:04.0/tcc_offset_degree_celsius\"\n";

/// A system below a temporary root, gone again when the test is over.
pub struct Machine {
    root: TempDir,
}

/// The UX8406CA of `tests/fixtures/`.
#[fixture]
pub fn machine() -> Machine {
    Machine::from_fixtures(&["ux8406ca"])
}

/// The same with a second processor package, configured.
#[fixture]
pub fn two_packages() -> Machine {
    let machine = Machine::from_fixtures(&["ux8406ca", "second-package"]);

    machine.append(CONFIG, CONFIG_TWO);
    machine
}

/// A root with nothing in it.
#[fixture]
pub fn empty() -> Machine {
    Machine::from_fixtures(&[])
}

impl Machine {
    fn from_fixtures(names: &[&str]) -> Self {
        let root = TempDir::new().unwrap();
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

        for name in names {
            root.copy_from(fixtures.join(name), &["**"]).unwrap();
        }
        root.child("run").create_dir_all().unwrap();
        // Whatever the umask of the one who runs the tests: the guard
        // refuses what others may write.
        chmod(root.path(), 0o700);
        if root.child(CONFIG).exists() {
            chmod(&root.join(CONFIG), 0o600);
        }
        Self { root }
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn join(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn paths(&self) -> Paths {
        Paths::below(self.root())
    }

    pub fn sys(&self) -> PathBuf {
        self.join("sys")
    }

    /// Replace a file in one step, as a sysfs attribute changes: a guard
    /// that runs meanwhile must never find it half written.
    pub fn put(&self, relative: &str, text: impl AsRef<[u8]>) -> PathBuf {
        let path = self.join(relative);
        let mut fresh = path.clone().into_os_string();

        fresh.push(".new");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&fresh, text).unwrap();
        chmod(Path::new(&fresh), 0o600);
        fs::rename(&fresh, &path).unwrap();
        path
    }

    pub fn append(&self, relative: &str, text: &str) {
        use std::io::Write;

        fs::OpenOptions::new()
            .append(true)
            .open(self.join(relative))
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }

    pub fn remove(&self, relative: &str) {
        fs::remove_file(self.join(relative)).unwrap();
    }

    pub fn chmod(&self, relative: &str, mode: u32) {
        chmod(&self.join(relative), mode);
    }

    /// The number in a file, if there is one.
    pub fn number(&self, relative: &str) -> Option<i32> {
        fs::read_to_string(self.join(relative))
            .ok()?
            .trim_end()
            .parse()
            .ok()
    }

    /// A coretemp device with the sensor of a package, limit 105 C.
    pub fn sensor(&self, hwmon: u32, package: i32, millidegrees: i32) {
        let dir = format!("sys/class/hwmon/hwmon{hwmon}");

        self.put(&format!("{dir}/name"), "coretemp\n");
        self.put(
            &format!("{dir}/temp1_label"),
            format!("Package id {package}\n"),
        );
        self.put(&format!("{dir}/temp1_crit"), "105000\n");
        self.put(&format!("{dir}/temp1_input"), format!("{millidegrees}\n"));
    }

    /// A pipe in place of a file.
    pub fn fifo(&self, relative: &str) {
        let path = self.join(relative);

        let _ = fs::remove_file(&path);
        mknodat(CWD, &path, FileType::Fifo, Mode::RUSR | Mode::WUSR, 0).unwrap();
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        // A directory that a test has closed cannot be emptied.
        for closed in [STATE, "run"] {
            let _ = fs::set_permissions(self.join(closed), fs::Permissions::from_mode(0o700));
        }
    }
}

pub fn chmod(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

/// Stop a test that cannot show anything as root: root reads and writes
/// whatever the permissions say.
pub fn ordinary_user() {
    assert!(
        !rustix::process::geteuid().is_root(),
        "run these tests as an ordinary user"
    );
}
