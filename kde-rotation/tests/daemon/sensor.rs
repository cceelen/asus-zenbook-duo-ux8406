//! The claim on the accelerometer, and the switch of the applet.

use crate::{LOWER, Session, panels};

#[test]
fn the_accelerometer_is_claimed_while_the_rotation_is_on() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("the claim", || session.sensor().claims() == (1, 0));
    session.call("SetEnabled", &(false,)).expect("call");
    program.wait_for("the release", || session.sensor().claims() == (1, 1));
    assert!(session.config().contains("enabled = false"));

    // Off: the laptop turns, the screens do not.
    session.sensor().turn("left-up");
    std::thread::sleep(crate::turns::SETTLED);
    assert_eq!(session.place(LOWER), (0, 1440, 0));

    // On again: the screens follow the orientation that the laptop has.
    session.call("SetEnabled", &(true,)).expect("call");
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    assert_eq!(session.sensor().claims(), (2, 1));
}

#[test]
fn switched_off_in_the_configuration_nothing_is_claimed() {
    let session = Session::new(panels(), Some("left-up"));

    std::fs::write(session.file(), "enabled = false\n").expect("configuration");

    let _program = session.start();

    std::thread::sleep(crate::turns::SETTLED);
    assert_eq!(session.sensor().claims(), (0, 0));
    assert_eq!(session.place(LOWER), (0, 1440, 0));
}

#[test]
fn a_sensor_service_that_starts_again_is_claimed_again() {
    let mut session = Session::new(panels(), Some("normal"));
    let program = session.start();

    program.wait_for("the claim", || session.sensor().claims() == (1, 0));
    session.sensor.as_mut().expect("sensor").stop();
    program.wait_for("the control without sensor", || {
        session.control() == "buttons"
    });
    session
        .sensor
        .as_mut()
        .expect("sensor")
        .start(true, "right-up");
    program.wait_for("the new claim", || {
        let (claimed, released) = session.sensor().claims();

        claimed >= 2 && released == 0
    });
    session.wait_places(&program, (0, 0, 3), (1440, 0, 3));
    assert_eq!(session.control(), "switch");
}

#[test]
fn an_accelerometer_that_goes_leaves_the_buttons() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().set_present(false);
    program.wait_for("the buttons", || session.control() == "buttons");
    session.sensor().set_present(true);
    program.wait_for("the switch", || session.control() == "switch");
}

#[test]
fn a_refused_claim_follows_nothing() {
    let session = Session::new(panels(), Some("normal"));

    session.sensor().refuse_claims();

    let program = session.start();

    program.wait_log("iio-sensor-proxy: ");
    session.sensor().turn("left-up");
    std::thread::sleep(crate::turns::SETTLED);
    assert_eq!(session.place(LOWER), (0, 1440, 0));
}

#[test]
fn a_turn_of_the_laptop_keeps_the_claim() {
    let session = Session::new(panels(), Some("normal"));
    let program = session.start();

    session.sensor().turn("left-up");
    session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    assert_eq!(session.sensor().claims(), (1, 0));
}

/// A claim that the service answers late, or never, does not hold up the
/// program.
mod slow_service {
    use std::time::{Duration, Instant};

    use crate::{LOWER, Session, UPPER, panels};

    #[test]
    fn a_claim_that_is_answered_late_does_not_delay_the_start_and_turns_start_after_it() {
        let session = Session::new(panels(), Some("left-up"));

        session.sensor().delay_claims(Duration::from_secs(2));

        let begin = Instant::now();
        let program = session.start();

        assert!(begin.elapsed() < Duration::from_millis(1800));
        program.wait_for("never", || session.compositor.output(LOWER).policy == 0);
        assert_eq!(session.sensor().claims(), (0, 0));
        assert_eq!(session.place(UPPER), (0, 0, 0));
        program.wait_for("the claim", || session.sensor().claims() == (1, 0));
        session.wait_places(&program, (1440, 0, 1), (0, 0, 1));
    }

    #[test]
    fn a_service_that_does_not_answer_is_tried_again_after_a_pause() {
        let session = Session::new(panels(), Some("right-up"));

        session.sensor().fail_claims(1);

        let program = session.start();

        program.wait_log("no answer, the claim is tried again");
        assert_eq!(session.sensor().claims(), (0, 0));
        program.wait_for("the claim", || session.sensor().claims() == (1, 0));
        session.wait_places(&program, (0, 0, 3), (1440, 0, 3));
    }

    #[test]
    fn the_program_answers_while_the_claim_is_pending() {
        let session = Session::new(panels(), Some("normal"));

        session.sensor().delay_claims(Duration::from_secs(60));

        let program = session.start();

        session.call("Toggle", &()).expect("call");
        program.wait_for("the switch off", || {
            !bool::try_from(session.property("Enabled")).expect("flag")
        });
        session.call("Toggle", &()).expect("call");
        session.compositor.set_policy(UPPER, 2);
        program.wait_for("never again", || {
            session.compositor.output(UPPER).policy == 0
        });
        let _ = session.call("Quit", &());

        let (status, _) = program.end();

        assert!(status.success());
    }
}

#[test]
fn without_a_system_bus_the_buttons_are_shown() {
    let session = Session::new(panels(), None);
    let mut command = session.command(&[]);

    command.env("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/nonexistent/bus");

    let program = asus_ux8406_kde_fakes::Program::start(command);

    program.wait_log("ready");
    program.wait_log("no orientation sensor: system bus: ");
    assert_eq!(session.control(), "buttons");
}
