//! A live image with the packages installed already: udev, Plasma and KWin
//! started them at the boot and the login. The checks look at this state
//! before the tool changes anything.

use std::path::PathBuf;

use crate::activate::{self, DAEMONS, Daemon, SCRIPT_ID};
use crate::auto::{self, Phase};
use crate::context::Ctx;
use crate::dbus;

/// Whether every package file is installed with the same name, version,
/// release and architecture. No file means no.
#[must_use]
pub fn installed(ctx: &Ctx<'_>, files: &[PathBuf]) -> bool {
    !files.is_empty()
        && files.iter().all(|file| {
            let file = file.to_string_lossy().into_owned();
            let nevra = ctx.run("rpm", &["-qp", "--qf", "%{NEVRA}\\n", &file]);
            let nevra = nevra.stdout.trim();

            nevra_known(nevra) && ctx.run("rpm", &["-q", nevra]).success()
        })
}

fn nevra_known(nevra: &str) -> bool {
    !nevra.is_empty() && !nevra.contains(char::is_whitespace)
}

fn script(ctx: &Ctx<'_>) -> Result<String, String> {
    let bus = ctx.session_bus()?;
    let loaded: bool = dbus::call(
        &bus,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
        "isScriptLoaded",
        &(SCRIPT_ID,),
    )?;

    if loaded {
        Ok(format!("{SCRIPT_ID} is loaded"))
    } else {
        Err(format!("{SCRIPT_ID} is not loaded"))
    }
}

fn unit(ctx: &Ctx<'_>, daemon: &Daemon) -> Result<String, String> {
    let unit = activate::autostart_unit(ctx, daemon.entry)?;
    let active = ctx
        .run("systemctl", &["--user", "is-active", "--quiet", &unit])
        .success();
    let owned = ctx
        .session_bus()
        .and_then(|bus| dbus::has_owner(&bus, daemon.service))?;

    match (active, owned) {
        (true, true) => Ok(format!(
            "{unit} is active and {} has an owner",
            daemon.service
        )),
        (false, _) => Err(format!("{unit} is not active")),
        (true, false) => Err(format!(
            "{unit} is active, but {} has no owner on the session bus",
            daemon.service
        )),
    }
}

/// The state of the session as the image left it. Each row has a name
/// that ends with "(as booted)", or is in the section of these checks.
pub fn checks(ctx: &mut Ctx<'_>) {
    ctx.say("The packages are installed in the image already: the tool looks at the state as booted first.");

    let loaded = script(ctx);

    ctx.auto("KWin loaded the script by itself (as booted)", loaded);
    for daemon in &DAEMONS {
        let started = unit(ctx, daemon);

        ctx.auto(
            &format!("{} started by its autostart unit (as booted)", daemon.entry),
            started,
        );
    }
    auto::keyboard(ctx, Phase::Booted);
    auto::shortcuts(ctx, Phase::Booted);
    auto::policy_booted(ctx);
}
