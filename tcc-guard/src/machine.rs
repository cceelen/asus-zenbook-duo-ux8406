//! The machine as sysfs shows it: which model it is, its processor packages,
//! and for each the files of its temperature and its thermal offset.

use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::input::{self, MAX_PACKAGES, Settings};
use crate::sysfs::{self, VALUE_MAX};

/// The offset of the only package on a client platform, below `/sys`.
const TCC_PATH: &str = "/bus/pci/devices/0000:00:04.0/tcc_offset_degree_celsius";
const PRODUCT_NAME: &str = "class/dmi/id/product_name";
/// A product name is at most this long, with its newline.
const PRODUCT_NAME_MAX: usize = 128;
/// hwmon devices are looked for under these numbers.
const HWMON_COUNT: u32 = 256;
const CORETEMP: &str = "coretemp\n";

/// One processor package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// Its number.
    pub id: i32,
    /// The temperature it is limited to at offset 0.
    pub tjmax: i32,
    /// The file of its temperature, in millidegrees.
    pub temp: PathBuf,
    /// The file of its thermal offset.
    pub offset: PathBuf,
}

/// Check that this is the machine the settings are for.
///
/// # Errors
///
/// [`Error::WrongModel`] if it is another one, and what reading the product
/// name gave if that failed. Neither happens with `model = any`.
pub fn check_model(sysroot: &Path, settings: &Settings) -> Result<(), Error> {
    if settings.model == input::ANY_MODEL {
        return Ok(());
    }
    let name = sysfs::read_text(&sysroot.join(PRODUCT_NAME), PRODUCT_NAME_MAX)?;
    let name = name.trim_end_matches('\n');

    if settings.model_matches(name) {
        Ok(())
    } else {
        Err(Error::WrongModel {
            wanted: settings.model.clone(),
            found: name.to_owned(),
        })
    }
}

fn hwmon(sysroot: &Path, index: u32, attribute: &str) -> PathBuf {
    sysroot.join(format!("class/hwmon/hwmon{index}/{attribute}"))
}

/// The package whose sensor a hwmon device is, if it is a coretemp device
/// with a package sensor.
fn package_of(sysroot: &Path, index: u32) -> Option<i32> {
    let name = sysfs::read_text(&hwmon(sysroot, index, "name"), VALUE_MAX).ok()?;

    if name != CORETEMP {
        return None;
    }
    let label = sysfs::read_text(&hwmon(sysroot, index, "temp1_label"), VALUE_MAX).ok()?;

    input::parse_package_label(&label)
}

/// A file below the directory that stands in for `/sys`; `below` starts with
/// a slash.
fn below_sysroot(sysroot: &Path, below: &str) -> PathBuf {
    let mut path = sysroot.as_os_str().to_owned();

    path.push(below);
    path.into()
}

/// Where the offset of a package is written: as configured, or, for a
/// package that is the only one, at the usual place. With more than one
/// package each has to be configured, because a guess could move the wrong
/// one.
#[must_use]
pub fn control(sysroot: &Path, settings: &Settings, package: i32, alone: bool) -> Option<PathBuf> {
    let configured = settings
        .controls
        .iter()
        .find(|control| control.package == package)
        .map(|control| control.path.as_str());

    configured
        .or(alone.then_some(TCC_PATH))
        .map(|below| below_sysroot(sysroot, below))
}

/// Every processor package that has a temperature sensor, without the file
/// of its offset: (hwmon device, package).
fn sensors(sysroot: &Path) -> Result<Vec<(u32, i32)>, Error> {
    let mut found: Vec<(u32, i32)> = Vec::new();

    for index in 0..HWMON_COUNT {
        let Some(package) = package_of(sysroot, index) else {
            continue;
        };
        if found.iter().any(|&(_, known)| known == package) {
            return Err(Error::TwoSensors(package));
        }
        if found.len() >= MAX_PACKAGES {
            return Err(Error::TooManyPackages);
        }
        found.push((index, package));
    }
    if found.is_empty() {
        return Err(Error::NoSensor);
    }
    Ok(found)
}

/// Find the packages, with everything the guard needs of them.
///
/// # Errors
///
/// What stands in the way: no sensor, two for one package, too many
/// packages, an unknown limit, an unknown place for an offset, or a
/// configured package that is not there.
pub fn packages(sysroot: &Path, settings: &Settings) -> Result<Vec<Package>, Error> {
    let sensors = sensors(sysroot)?;
    let mut packages = Vec::with_capacity(sensors.len());

    for &(index, id) in &sensors {
        let tjmax = if settings.tjmax > 0 {
            settings.tjmax
        } else {
            sysfs::read_tjmax(&hwmon(sysroot, index, "temp1_crit"))
                .map_err(|_| Error::NoTjmax(id))?
        };

        packages.push((index, id, tjmax));
    }
    let packages = packages
        .into_iter()
        .map(|(index, id, tjmax)| {
            Ok(Package {
                id,
                tjmax,
                temp: hwmon(sysroot, index, "temp1_input"),
                offset: control(sysroot, settings, id, sensors.len() == 1)
                    .ok_or(Error::NoControl(id))?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    match settings
        .controls
        .iter()
        .find(|control| packages.iter().all(|package| package.id != control.package))
    {
        Some(control) => Err(Error::NotPresent(control.package)),
        None => Ok(packages),
    }
}

/// Check that the temperature and the offset of every package can be read
/// and are plausible, and that the offset can be written.
///
/// # Errors
///
/// What the first file that fails gave.
pub fn check(packages: &[Package]) -> Result<(), Error> {
    for package in packages {
        sysfs::read_temp(&package.temp)?;
        sysfs::read_offset(&package.offset)?;
        sysfs::check_writable(&package.offset)?;
    }
    Ok(())
}
