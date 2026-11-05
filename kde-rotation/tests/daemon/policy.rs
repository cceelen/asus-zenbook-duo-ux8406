//! KWin's own auto-rotate is off on the built-in screens, and the policy
//! from before is kept.

use asus_ux8406_kde_fakes::{Output, Style};

use crate::{LOWER, Session, UPPER, panels};

#[test]
fn at_the_start_kwin_turns_no_built_in_screen() {
    let mut outputs = panels();

    outputs.push(Output {
        policy: 2,
        ..Output::monitor("DP-1", 2560, 1440, 2304, 0)
    });

    let session = Session::new(outputs, Some("normal"));
    let program = session.start();

    program.wait_for("the policies", || {
        session.compositor.output(UPPER).policy == 0 && session.compositor.output(LOWER).policy == 0
    });
    // An external monitor keeps its policy.
    assert_eq!(session.compositor.output("DP-1").policy, 2);
    assert!(
        session.config().contains("eDP-1 = \"in-tablet-mode\""),
        "{}",
        session.config()
    );
    assert!(session.config().contains("eDP-2 = \"in-tablet-mode\""));
}

#[test]
fn a_policy_that_the_user_sets_is_kept_and_set_back() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("never", || session.compositor.output(UPPER).policy == 0);
    session.compositor.set_policy(UPPER, 2);
    program.wait_for("the policy of the user kept", || {
        session.config().contains("eDP-1 = \"always\"")
    });
    program.wait_for("never again", || {
        session.compositor.output(UPPER).policy == 0
    });
}

#[test]
fn a_kwin_without_the_policy_is_left_alone() {
    let session = Session::of_style(Style::Old, panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("left-up");
    program.wait_log("turned to left-up");
    assert!(
        !session
            .compositor
            .requests()
            .iter()
            .any(|request| request.contains("policy"))
    );
}

#[test]
fn a_configuration_that_cannot_be_written_is_said() {
    let session = Session::new(panels(), Some("normal"));
    let file = session.file();

    // A directory in the place of the file.
    std::fs::create_dir(&file).expect("directory");

    let (status, log) = asus_ux8406_kde_fakes::Program::start(session.command(&[])).end();

    assert_eq!(status.code(), Some(1));
    assert!(
        log.iter()
            .any(|line| line.contains("plasma-builtin-screen-rotation.toml")),
        "{log:?}"
    );
}
