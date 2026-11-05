//! `plasma-builtin-screen-rotation restore`: KWin's own policies back, before
//! the package is removed.

use asus_ux8406_kde_fakes::{Bus, Program, Style, program};

use crate::{LOWER, PROGRAM, Session, UPPER, panels};

fn restore(session: &Session) -> (Option<i32>, Vec<String>) {
    let (status, log) = Program::start(session.command(&["restore"])).end();

    (status.code(), log)
}

#[test]
fn restore_ends_the_program_and_gives_the_policies_back() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("never", || session.compositor.output(LOWER).policy == 0);

    let (code, log) = restore(&session);

    assert_eq!(code, Some(0), "{log:?}");
    assert!(program.end().0.success());
    assert_eq!(session.compositor.output(UPPER).policy, 1);
    assert_eq!(session.compositor.output(LOWER).policy, 1);
    assert!(
        !session.config().contains("policies.eDP"),
        "{}",
        session.config()
    );
    assert!(
        !session.config().contains("in-tablet-mode"),
        "{}",
        session.config()
    );
}

#[test]
fn a_screen_that_is_not_connected_keeps_its_saved_policy() {
    let session = Session::new(vec![crate::panels().remove(0)], Some("normal"));

    std::fs::write(
        session.file(),
        "[policies]\neDP-1 = \"always\"\neDP-2 = \"in-tablet-mode\"\nDP-9 = \"nonsense\"\n",
    )
    .expect("configuration");

    let (code, _) = restore(&session);

    assert_eq!(code, Some(0));
    assert_eq!(session.compositor.output(UPPER).policy, 2);
    assert!(session.config().contains("eDP-2 = \"in-tablet-mode\""));
}

#[test]
fn without_a_session_bus_restore_still_gives_the_policies_back() {
    let session = Session::new(panels(), None);
    let mut command = program::command(PROGRAM, Some(&session.compositor), None);

    std::fs::write(session.file(), "[policies]\neDP-1 = \"always\"\n").expect("configuration");
    command
        .env("XDG_CONFIG_HOME", session.config.path())
        .arg("restore");

    let (status, log) = Program::start(command).end();

    assert!(status.success(), "{log:?}");
    assert_eq!(session.compositor.output(UPPER).policy, 2);
}

#[test]
fn a_refusal_of_kwin_is_an_error() {
    let session = Session::new(panels(), None);

    std::fs::write(session.file(), "[policies]\neDP-1 = \"always\"\n").expect("configuration");
    session.compositor.refuse_next("no");

    let (code, log) = restore(&session);

    assert_eq!(code, Some(1));
    assert!(
        log.iter()
            .any(|line| line.contains("KWin did not take the policies: no")),
        "{log:?}"
    );
    assert!(session.config().contains("always"));
}

#[test]
fn kwin_that_ends_meanwhile_is_an_error() {
    let session = Session::new(panels(), None);

    std::fs::write(session.file(), "[policies]\neDP-1 = \"always\"\n").expect("configuration");
    session.compositor.stop_on_next_apply();

    let (code, log) = restore(&session);

    assert_eq!(code, Some(1));
    assert!(log.iter().any(|line| line.contains("Wayland")), "{log:?}");
}

#[test]
fn an_old_kwin_has_nothing_to_restore() {
    let session = Session::of_style(Style::Old, panels(), None);

    std::fs::write(session.file(), "[policies]\neDP-1 = \"always\"\n").expect("configuration");

    let (code, _) = restore(&session);

    assert_eq!(code, Some(0));
    assert_eq!(session.compositor.requests(), Vec::<String>::new());
}

#[test]
fn a_configuration_that_is_not_one_stops_restore() {
    let session = Session::new(panels(), None);

    std::fs::write(session.file(), "policies = 3\n").expect("configuration");

    let (code, log) = restore(&session);

    assert_eq!(code, Some(1));
    assert!(
        log.iter()
            .any(|line| line.contains("plasma-builtin-screen-rotation.toml")),
        "{log:?}"
    );
}

#[test]
fn restore_needs_kwin() {
    let session = Session::new(panels(), None);
    let mut command = program::command(PROGRAM, None, Some(&session.bus));

    command
        .env("XDG_CONFIG_HOME", session.config.path())
        .arg("restore");

    let (status, log) = Program::start(command).end();

    assert_eq!(status.code(), Some(1));
    assert!(
        log.iter().any(|line| line.contains("no Wayland session")),
        "{log:?}"
    );
}

#[test]
fn a_configuration_that_cannot_be_written_stops_restore() {
    use std::os::unix::fs::PermissionsExt;

    let session = Session::new(panels(), None);

    std::fs::write(session.file(), "[policies]\neDP-1 = \"always\"\n").expect("configuration");
    std::fs::set_permissions(
        session.config.path(),
        std::fs::Permissions::from_mode(0o555),
    )
    .expect("read only");

    let (code, log) = restore(&session);

    std::fs::set_permissions(
        session.config.path(),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("writable");
    assert_eq!(code, Some(1));
    assert!(
        log.iter()
            .any(|line| line.contains("plasma-builtin-screen-rotation.toml: ")),
        "{log:?}"
    );
    // KWin has the policy back all the same.
    assert_eq!(session.compositor.output(UPPER).policy, 2);
}

#[test]
fn a_program_that_does_not_end_is_said() {
    let session = Session::new(panels(), None);
    // A connection of this process owns the name and answers no `Quit`.
    // It lets the name go soon, but its process, this one, does not end.
    let owner = session.bus.connect();

    owner.request_name(crate::SERVICE).expect("name");
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        owner.release_name(crate::SERVICE).expect("release");
    });

    let (code, log) = restore(&session);

    assert_eq!(code, Some(0), "{log:?}");
    assert!(
        log.iter()
            .any(|line| line
                .contains("did not end within 5 s (name free: true, process gone: false)")),
        "{log:?}"
    );
}

#[test]
fn a_bus_that_takes_no_match_rule_is_said_and_restore_goes_on() {
    let session = Session::on(Bus::with_match_rules(0), Style::Registry, panels(), None);
    // A connection of this process owns the name and answers no `Quit`.
    let owner = session.bus.connect();

    // The plain call: the helper of the library follows the answer with a
    // match rule, which this bus refuses.
    owner
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "RequestName",
            &(crate::SERVICE, 0_u32),
        )
        .expect("name");
    // The owner answers no `Quit`: it lets the name go soon.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        owner
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "ReleaseName",
                &(crate::SERVICE,),
            )
            .expect("release");
    });

    let (code, log) = restore(&session);

    assert_eq!(code, Some(0), "{log:?}");
    assert!(
        log.iter()
            .any(|line| line.contains("the end of the program is not followed")),
        "{log:?}"
    );
    assert_eq!(session.compositor.output(UPPER).policy, 1);
}
