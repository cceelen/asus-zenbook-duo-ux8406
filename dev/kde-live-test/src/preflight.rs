//! What must be true before the test starts, and the versions to record.

use crate::context::Ctx;
use crate::report::{Kind, Status};

/// The programs that the checks use and the live image may lack. dnf
/// finds the package of each by the path of the program.
pub const TOOLS: [&str; 7] = [
    "kscreen-doctor",
    "libinput",
    "konsole",
    "journalctl",
    "systemd-hwdb",
    "pgrep",
    "stdbuf",
];

/// The first line of the output of a command that prints a version.
fn version(ctx: &Ctx<'_>, program: &str, args: &[&str]) -> Result<String, String> {
    let output = ctx.run(program, args);

    if output.success() {
        Ok(output.stdout.lines().next().unwrap_or_default().to_owned())
    } else {
        Err(format!("{program} failed: {}", output.text().trim()))
    }
}

/// Run the preflight; whether the test can go on.
pub fn run(ctx: &mut Ctx<'_>) -> bool {
    let product = ctx
        .read("/sys/class/dmi/id/product_name")
        .unwrap_or_default();
    let product = product.trim();
    let model = if product.contains("UX8406") {
        Ok(product.to_owned())
    } else {
        Err(format!("the product name is {product:?}, not a UX8406"))
    };
    let model_ok = model.is_ok();

    ctx.auto("product name contains UX8406", model);

    let session = ctx.var("XDG_SESSION_TYPE").unwrap_or_default().to_owned();
    let socket = ctx.wayland_socket();
    let wayland = match (session.as_str(), socket) {
        ("wayland", Some(socket)) => {
            Ok(format!("session type wayland, socket {}", socket.display()))
        }
        (other, _) => Err(format!(
            "session type {other:?} or no WAYLAND_DISPLAY: the test needs the Wayland session of Plasma"
        )),
    };
    let wayland_ok = wayland.is_ok();

    ctx.auto("Wayland session", wayland);

    let plasma = version(ctx, "plasmashell", &["--version"]);
    let kwin = version(ctx, "kwin_wayland", &["--version"]);
    let kernel = version(ctx, "uname", &["-r"]);

    ctx.raw(
        "versions.txt",
        &format!(
            "plasmashell: {plasma:?}\nkwin_wayland: {kwin:?}\nkernel: {kernel:?}\nos-release:\n{}\n",
            ctx.read("/etc/os-release").unwrap_or_default()
        ),
    );
    ctx.auto(
        "Plasma and KWin versions",
        plasma.and_then(|plasma| kwin.map(|kwin| format!("{plasma}; {kwin}"))),
    );
    ctx.auto("kernel version", kernel);

    let network = ctx.run("getent", &["ahosts", "mirrors.fedoraproject.org"]);

    ctx.record(
        "network for dnf",
        Kind::Auto,
        if network.success() {
            Status::Ok
        } else {
            Status::Skip
        },
        if network.success() {
            "mirrors.fedoraproject.org resolves"
        } else {
            "no network: enough if dnf has nothing to fetch"
        },
    );

    let sudo_ok = sudo(ctx);

    model_ok && wayland_ok && sudo_ok
}

/// Make sure that `sudo -n` works. The live user has no password, so sudo
/// asks for one and Enter is the answer; the tester sees the question.
pub fn sudo(ctx: &mut Ctx<'_>) -> bool {
    let first = ctx.run("sudo", &["-n", "true"]);

    if first.success() {
        ctx.auto("sudo", Ok("sudo -n works".to_owned()));
        return true;
    }

    if !ctx.prompt {
        ctx.auto(
            "sudo",
            Err(format!(
                "sudo -n true failed, and the tool may not ask: {}",
                first.text().trim()
            )),
        );
        return false;
    }

    ctx.say("sudo asks for the password of liveuser. That user has no password: press Enter.");

    let asked = ctx.run_attached("sudo", &["-v"]);
    let second = ctx.run("sudo", &["-n", "true"]);
    let result = if second.success() {
        Ok("sudo -v worked and sudo -n works".to_owned())
    } else if asked.success() {
        Err(format!(
            "sudo -v worked, but sudo -n true failed: {}",
            second.text().trim()
        ))
    } else {
        Err(format!(
            "sudo -v ended with {:?}, and sudo -n true failed: {}",
            asked.code,
            second.text().trim()
        ))
    };
    let ok = result.is_ok();

    ctx.auto("sudo", result);
    ok
}
