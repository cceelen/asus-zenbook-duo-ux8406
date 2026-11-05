//! The dependencies of the packages: what rpm expresses before and after
//! `dnf`, and what the programs need at run time that rpm does not say.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::context::Ctx;
use crate::dbus;
use crate::outputs;

/// The requirements of a package, sorted into the kinds that matter here.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Requirements {
    /// Names that a package or a file has to provide.
    pub names: BTreeSet<String>,
    /// Conditional requirements: only dnf can tell.
    pub conditional: Vec<String>,
}

/// Sort the output of `rpm --requires`. A line is a name, optionally with a
/// comparison and a version; `rpmlib(...)` is rpm itself; a line in
/// parentheses is a conditional requirement.
#[must_use]
pub fn parse_requires(text: &str) -> Requirements {
    let mut result = Requirements::default();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.starts_with("rpmlib(") {
            continue;
        }
        if line.starts_with('(') {
            result.conditional.push(line.to_owned());
        } else {
            result
                .names
                .insert(line.split_whitespace().next().unwrap_or(line).to_owned());
        }
    }
    result
}

/// The names that the output of `rpm --provides` lists.
#[must_use]
pub fn parse_provides(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

/// The package files in `dir/rpms`, sorted.
#[must_use]
pub fn rpm_files(ctx: &Ctx<'_>) -> Vec<PathBuf> {
    let dir = ctx.dir.join("rpms");

    std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rpm"))
        .fold(BTreeSet::new(), |mut set, path| {
            set.insert(path);
            set
        })
        .into_iter()
        .collect()
}

/// Whether something installed provides a name.
fn provided(ctx: &Ctx<'_>, name: &str) -> bool {
    ctx.run("rpm", &["-q", "--whatprovides", name]).success()
}

/// The packages in the files, and whether dnf has to fetch more.
#[derive(Debug, PartialEq, Eq)]
pub struct Packages {
    /// The names of the packages, sorted.
    pub names: Vec<String>,
    /// Whether a requirement needs a package that is not present.
    pub fetch: bool,
}

/// Whether the machine reaches the Fedora mirrors.
#[must_use]
pub fn network(ctx: &Ctx<'_>) -> bool {
    ctx.run("getent", &["ahosts", "mirrors.fedoraproject.org"])
        .success()
}

/// The names of the packages in the files, and what dnf has to fetch for
/// them. Reports the result as one row; none if dnf has to fetch and there is
/// no network.
pub fn before(ctx: &mut Ctx<'_>, files: &[PathBuf]) -> Option<Packages> {
    let mut packages = Vec::new();
    let mut required = BTreeSet::new();
    let mut conditional = 0;
    let mut offered = BTreeSet::new();

    for file in files {
        let file = file.to_string_lossy().into_owned();
        let name = ctx.run("rpm", &["-qp", "--qf", "%{NAME}\\n", &file]);
        let wanted = parse_requires(&ctx.run("rpm", &["-qp", "--requires", &file]).stdout);

        packages.push(name.stdout.trim().to_owned());
        offered.extend(parse_provides(
            &ctx.run("rpm", &["-qp", "--provides", &file]).stdout,
        ));
        conditional += wanted.conditional.len();
        required.extend(wanted.names);
    }
    packages.sort();

    let own: Vec<&String> = required
        .iter()
        .filter(|name| offered.contains(*name))
        .collect();
    let others: Vec<&String> = required
        .iter()
        .filter(|name| !offered.contains(*name))
        .collect();
    let (present, fetch): (Vec<&String>, Vec<&String>) =
        others.iter().partition(|name| provided(ctx, name));
    let network = network(ctx);
    let summary = format!(
        "{} packages, {} requirements: {} present in the live image, {} from the stick, {} for dnf to fetch{}; {} conditional left to dnf; network {}",
        packages.len(),
        required.len(),
        present.len(),
        own.len(),
        fetch.len(),
        if fetch.is_empty() {
            String::new()
        } else {
            format!(
                " ({})",
                fetch
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        conditional,
        if network { "yes" } else { "no" },
    );
    let missing_network = !fetch.is_empty() && !network;

    if missing_network {
        ctx.auto(
            "requirements before dnf",
            Err(format!("no network, and dnf has to fetch: {summary}")),
        );
        None
    } else {
        ctx.auto("requirements before dnf", Ok(summary));
        Some(Packages {
            names: packages,
            fetch: !fetch.is_empty(),
        })
    }
}

/// After dnf: each package is installed, each requirement is provided, and
/// `rpm -V` finds nothing changed.
pub fn after(ctx: &mut Ctx<'_>, packages: &[String]) {
    let names: Vec<&str> = packages.iter().map(String::as_str).collect();
    let mut query = vec!["-q"];

    query.extend(&names);

    let installed = ctx.run("rpm", &query);

    ctx.auto(
        "packages installed",
        if installed.success() {
            Ok(installed.stdout.lines().collect::<Vec<_>>().join(", "))
        } else {
            Err(installed.text())
        },
    );

    let mut missing = BTreeSet::new();
    let mut total = 0;

    for name in &names {
        let requires = parse_requires(&ctx.run("rpm", &["-q", "--requires", name]).stdout);

        total += requires.names.len();
        missing.extend(
            requires
                .names
                .into_iter()
                .filter(|requirement| !provided(ctx, requirement)),
        );
    }
    ctx.auto(
        "requirements after dnf",
        if missing.is_empty() {
            Ok(format!("{total} requirements, all provided"))
        } else {
            Err(format!(
                "nothing provides: {}",
                missing.into_iter().collect::<Vec<_>>().join(", ")
            ))
        },
    );

    let mut verify = vec!["-V"];

    verify.extend(&names);

    let verified = ctx.run("rpm", &verify);

    ctx.auto(
        "files verified (rpm -V)",
        if verified.success() && verified.stdout.trim().is_empty() {
            Ok("no file differs from its package".to_owned())
        } else {
            Err(format!("rpm -V: {}", verified.text()))
        },
    );
}

/// The path of the object that the hwdb names for the keyboard.
const BPF_OBJECT: &str = "0010-ASUS__Zenbook-Duo-UX8406-Keyboard.bpf.o";
/// The modalias of the keyboard on USB, as the hwdb has it.
const KEYBOARD_MODALIAS: &str = "hid-bpf:hid:b0003g0001v00000B05p00001BF2";
/// The oldest version of the output management that sets the rotation
/// policy; the two programs need it.
const MANAGEMENT_MINIMUM: u32 = 5;

/// The runtime dependencies that rpm does not say. One row for each.
pub fn runtime(ctx: &mut Ctx<'_>) {
    let hid_bpf = hid_bpf(ctx);
    ctx.auto("udev-hid-bpf can load the keyboard object", hid_bpf);

    let sensor = sensor_proxy(ctx);
    ctx.auto("iio-sensor-proxy on the system bus", sensor);

    let accel = ctx
        .session_bus()
        .and_then(|bus| dbus::has_owner(&bus, "org.kde.kglobalaccel"))
        .and_then(|owned| {
            if owned {
                Ok("org.kde.kglobalaccel has an owner".to_owned())
            } else {
                Err("kglobalacceld is not on the session bus".to_owned())
            }
        });
    ctx.auto("kglobalacceld on the session bus", accel);

    let kwin = kwin_interfaces(ctx);
    ctx.auto("KWin scripting and input device interfaces", kwin);

    let globals = wayland_globals(ctx);
    ctx.auto("KWin Wayland output protocols", globals);

    let qml = qml_modules(ctx);
    ctx.auto("Plasma QML modules of the applet", qml);
}

fn hid_bpf(ctx: &Ctx<'_>) -> Result<String, String> {
    let tool = ctx.run("udev-hid-bpf", &["--version"]);
    let object = ["/usr/lib/firmware/hid/bpf", "/lib/firmware/hid/bpf"]
        .iter()
        .map(|dir| ctx.path(&format!("{dir}/{BPF_OBJECT}")))
        .find(|path| path.is_file());
    let hwdb = ctx.run("systemd-hwdb", &["query", KEYBOARD_MODALIAS]);
    let btf = ctx.path("/sys/kernel/btf/vmlinux").is_file();
    let mut missing = Vec::new();

    if !tool.success() {
        missing.push("the program udev-hid-bpf does not run".to_owned());
    }
    if object.is_none() {
        missing.push(format!(
            "the object {BPF_OBJECT} is not in firmware/hid/bpf"
        ));
    }
    if !hwdb.stdout.contains(BPF_OBJECT) {
        missing.push("the hwdb does not name the object for the keyboard".to_owned());
    }
    if !btf {
        missing.push("the kernel has no /sys/kernel/btf/vmlinux".to_owned());
    }
    if missing.is_empty() {
        Ok(format!(
            "{}; object {}; hwdb names it; kernel BTF present",
            tool.stdout.trim(),
            object
                .map(|path| path.display().to_string())
                .unwrap_or_default()
        ))
    } else {
        Err(missing.join("; "))
    }
}

fn sensor_proxy(ctx: &Ctx<'_>) -> Result<String, String> {
    let bus = ctx.system_bus()?;
    let name = "net.hadess.SensorProxy";

    // The service starts when something asks for it; if it runs already,
    // the bus says so.
    let _: Result<u32, String> = dbus::call(
        &bus,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "StartServiceByName",
        &(name, 0_u32),
    );
    if !dbus::wait_owner(&bus, name, ctx.timing.settle, ctx.timing.poll) {
        return Err(format!(
            "{name} has no owner on the system bus: iio-sensor-proxy does not run"
        ));
    }

    dbus::property::<bool>(
        &bus,
        name,
        "/net/hadess/SensorProxy",
        name,
        "HasAccelerometer",
    )
    .and_then(|accelerometer| {
        if accelerometer {
            Ok("running, has an accelerometer".to_owned())
        } else {
            Err("running, but HasAccelerometer is false".to_owned())
        }
    })
}

fn kwin_interfaces(ctx: &Ctx<'_>) -> Result<String, String> {
    let bus = ctx.session_bus()?;
    let loaded: bool = dbus::call(
        &bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "isScriptLoaded",
        &("asus-zenbook-duo-ux8406-keys",),
    )
    .map_err(|error| format!("scripting interface: {error}"))?;
    let devices = asus_ux8406_kde_keys::input::devices(&bus)
        .map_err(|error| format!("input device interface: {error}"))?;

    Ok(format!(
        "scripting answers (script loaded: {loaded}); {} input devices",
        devices.len()
    ))
}

fn wayland_globals(ctx: &Ctx<'_>) -> Result<String, String> {
    let globals = ctx
        .wayland()
        .and_then(|connection| outputs::globals(&connection))?;
    let version = |name: &str| {
        globals
            .iter()
            .find(|(known, _)| known == name)
            .map(|(_, v)| *v)
    };
    let management = version("kde_output_management_v2")
        .ok_or("the compositor has no kde_output_management_v2: it is not KWin 6.x")?;
    let device = version("kde_output_device_registry_v2")
        .map(|v| format!("kde_output_device_registry_v2 version {v}"))
        .or_else(|| {
            version("kde_output_device_v2").map(|v| format!("kde_output_device_v2 version {v}"))
        })
        .ok_or(
            "the compositor has neither kde_output_device_registry_v2 nor kde_output_device_v2",
        )?;

    if management < MANAGEMENT_MINIMUM {
        return Err(format!(
            "kde_output_management_v2 is version {management}; the rotation program needs {MANAGEMENT_MINIMUM} (auto-rotate policy)"
        ));
    }
    Ok(format!(
        "kde_output_management_v2 version {management}; {device}"
    ))
}

fn qml_modules(ctx: &Ctx<'_>) -> Result<String, String> {
    let applet = "/usr/share/plasma/plasmoids/io.github.cceelen.builtin-screen-rotation/contents/ui/main.qml";
    let source = ctx
        .read(applet)
        .ok_or_else(|| format!("{applet} is not installed"))?;
    let modules: BTreeSet<&str> = source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("import "))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter(|module| module.starts_with("org.kde."))
        .collect();
    let missing: Vec<&str> = modules
        .iter()
        .copied()
        .filter(|module| {
            let path = module.replace('.', "/");

            ![
                "/usr/lib64/qt6/qml",
                "/usr/lib/qt6/qml",
                "/usr/lib/x86_64-linux-gnu/qt6/qml",
            ]
            .iter()
            .any(|root| ctx.path(&format!("{root}/{path}/qmldir")).is_file())
        })
        .collect();

    if missing.is_empty() {
        Ok(format!(
            "{} modules found: {}",
            modules.len(),
            modules.into_iter().collect::<Vec<_>>().join(", ")
        ))
    } else {
        Err(format!("QML modules missing: {}", missing.join(", ")))
    }
}
