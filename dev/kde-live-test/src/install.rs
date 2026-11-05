//! The installation: the tools of the checks, then the packages from
//! `rpms/` with dnf, and the checks of their dependencies around it.

use crate::context::Ctx;
use crate::deps::{self, Packages};
use crate::preflight::TOOLS;
use crate::report::{Kind, Status};

/// Install the tools that the checks use and the live image may lack. dnf
/// names the package by the path of the program.
fn tools(ctx: &mut Ctx<'_>) {
    let missing: Vec<String> = TOOLS
        .iter()
        .filter(|program| ctx.which(program).is_none())
        .map(|program| format!("/usr/bin/{program}"))
        .collect();

    if !missing.is_empty() && !deps::network(ctx) {
        ctx.record(
            "test tools install",
            Kind::Auto,
            Status::Skip,
            &format!(
                "no network, and only a repository has these programs: {}",
                missing.join(" ")
            ),
        );
    } else if !missing.is_empty() {
        ctx.say(&format!(
            "The live image lacks some tools. dnf installs the packages of: {}",
            missing.join(" ")
        ));

        let mut args = vec!["install", "-y"];

        args.extend(missing.iter().map(String::as_str));

        let output = ctx.sudo("dnf", &args);

        ctx.raw("dnf-tools.txt", &output.text());
    }

    let still: Vec<&str> = TOOLS
        .iter()
        .copied()
        .filter(|program| ctx.which(program).is_none())
        .collect();

    ctx.auto(
        "test tools present",
        if still.is_empty() {
            Ok(TOOLS.join(", "))
        } else {
            Err(format!(
                "missing, and dnf could not install them: {}",
                still.join(", ")
            ))
        },
    );
}

/// Install the packages; whether the test can go on. `booted` says that the
/// image has them already.
pub fn run(ctx: &mut Ctx<'_>, booted: bool) -> bool {
    let files = deps::rpm_files(ctx);

    if files.is_empty() {
        ctx.auto(
            "packages in rpms/",
            Err(format!(
                "no .rpm file in {}",
                ctx.dir.join("rpms").display()
            )),
        );
        return false;
    }

    let Some(Packages {
        names: packages,
        fetch,
    }) = deps::before(ctx, &files)
    else {
        return false;
    };

    if booted {
        ctx.auto(
            "packages already in the image",
            Ok(format!(
                "{} packages with the version of rpms/: {}",
                packages.len(),
                packages.join(", ")
            )),
        );
    } else if !dnf(ctx, &files, &packages, fetch) {
        return false;
    }
    deps::after(ctx, &packages);
    tools(ctx);
    true
}

/// Install the files with dnf; whether it worked.
fn dnf(ctx: &mut Ctx<'_>, files: &[std::path::PathBuf], packages: &[String], fetch: bool) -> bool {
    ctx.say("Installing the packages with dnf. This can take a minute.");

    let paths: Vec<String> = files
        .iter()
        .map(|file| file.to_string_lossy().into_owned())
        .collect();
    let mut args = vec!["install", "-y"];

    // Nothing to fetch: dnf must not load the metadata of the repositories,
    // because the live session may have no network.
    if !fetch {
        args.push("--disablerepo=*");
    }
    args.extend(paths.iter().map(String::as_str));

    let output = ctx.sudo("dnf", &args);

    ctx.raw("dnf-install.txt", &output.text());
    ctx.auto(
        "dnf install of the packages",
        if output.success() {
            Ok(format!(
                "{} packages: {}",
                packages.len(),
                packages.join(", ")
            ))
        } else {
            Err(format!(
                "dnf ended with {:?}: {}",
                output.code,
                output.text().lines().last().unwrap_or_default()
            ))
        },
    );
    output.success()
}
