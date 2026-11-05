//! The raw state of the session, saved to `results/raw/` at two points.

use crate::context::Ctx;

/// A file of the home directory.
fn home_file(ctx: &Ctx<'_>, name: &str) -> String {
    std::fs::read_to_string(format!("{}/{name}", ctx.var("HOME").unwrap_or_default()))
        .unwrap_or_default()
}

/// Save the state under names that start with `stage`.
pub fn run(ctx: &mut Ctx<'_>, stage: &str) {
    let journal_user = ctx.run("journalctl", &["--user", "-b", "--no-pager"]);
    let journal_kernel = ctx.run("journalctl", &["-b", "-k", "--no-pager"]);
    let outputs = ctx.run("kscreen-doctor", &["-o"]);
    let devices = ctx.run("libinput", &["list-devices"]);
    let packages = ctx.run("rpm", &["-qa", "--last"]);

    ctx.raw(&format!("{stage}-journal-user.txt"), &journal_user.text());
    ctx.raw(
        &format!("{stage}-journal-kernel.txt"),
        &journal_kernel.text(),
    );
    ctx.raw(&format!("{stage}-kscreen-doctor.txt"), &outputs.text());
    ctx.raw(
        &format!("{stage}-libinput-list-devices.txt"),
        &devices.text(),
    );
    ctx.raw(&format!("{stage}-rpm-qa.txt"), &packages.text());
    ctx.raw(
        &format!("{stage}-kwinoutputconfig.json"),
        &home_file(ctx, ".config/kwinoutputconfig.json"),
    );
    ctx.raw(
        &format!("{stage}-kcminputrc"),
        &home_file(ctx, ".config/kcminputrc"),
    );
    ctx.raw(
        &format!("{stage}-kglobalshortcutsrc"),
        &home_file(ctx, ".config/kglobalshortcutsrc"),
    );
}
