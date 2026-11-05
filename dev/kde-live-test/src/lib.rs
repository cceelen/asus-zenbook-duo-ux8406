//! Tests the KDE packages in a Fedora KDE live session on the ASUS Zenbook
//! Duo UX8406. The tester runs one command from the data partition of the
//! stick. The tool installs the packages from `rpms/`, activates them without
//! a logout (the programs start through systemd, as Plasma starts them), checks what a machine can check, asks the tester for the
//! physical actions that remain (a key, the keyboard on the screen, a turn of
//! the laptop, a touch) and decides by itself from the events of KWin and the
//! kernel whether the action had the right result. It writes `report.md` and
//! the raw logs to `results/` next to itself.
//!
//! This is a development tool: it is not packaged and not installed.

pub mod activate;
pub mod auto;
pub mod booted;
pub mod cli;
pub mod collect;
pub mod console;
pub mod context;
pub mod dbus;
pub mod deps;
pub mod guided;
pub mod hand;
pub mod install;
pub mod keepalive;
pub mod live;
pub mod outputs;
pub mod preflight;
pub mod report;
pub mod runner;
pub mod windows;

use context::Ctx;
use report::Counts;

/// Why the tool stopped before its end.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The system is not the live image.
    #[error("this is not a live session ({0}): the tool never changes an installed system")]
    NotLive(String),
    /// The results directory cannot be made.
    #[error("results directory: {0}")]
    Results(#[from] std::io::Error),
    /// A file of the results cannot be written.
    #[error("cannot write the results: {0}")]
    Write(String),
}

/// Run the whole test. The result is how many checks ended with each status.
///
/// # Errors
///
/// Refer to [`Error`].
pub fn run(ctx: &mut Ctx<'_>) -> Result<Counts, Error> {
    let runner = ctx.runner;
    let interval = ctx.timing.keepalive;
    let counts = keepalive::keep(runner, interval, || full(ctx));

    ctx.report
        .broken()
        .map_or(Ok(counts), |reason| Err(Error::Write(reason.to_owned())))
}

fn full(ctx: &mut Ctx<'_>) -> Counts {
    ctx.report.section("Preflight");
    if preflight::run(ctx) {
        let booted = booted::installed(ctx, &deps::rpm_files(ctx));

        if booted {
            ctx.report.section("As booted");
            booted::checks(ctx);
        }
        ctx.report.section("Dependencies and installation");
        if install::run(ctx, booted) {
            ctx.report.section("Activation");
            activate::run(ctx, booted);
            ctx.report.section("Runtime dependencies");
            deps::runtime(ctx);
            collect::run(ctx, "before");
            ctx.report.section("Automatic checks");
            auto::run(ctx, booted);
            ctx.report.section("Guided checks");
            guided::run(ctx);
            collect::run(ctx, "after");
        }
    }
    finish(ctx)
}

fn finish(ctx: &mut Ctx<'_>) -> Counts {
    let counts = ctx.report.counts();

    ctx.say(&format!(
        "Done: {} OK, {} FAIL, {} SKIP. Report: {}",
        counts.ok,
        counts.fail,
        counts.skip,
        ctx.report.dir().join("report.md").display()
    ));
    ctx.say("Plug the stick back into the work machine.");
    counts
}
