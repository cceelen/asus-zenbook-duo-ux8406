//! The auto-rotate policy of the built-in outputs.

use asus_ux8406_kde_common::output::Policy;
use asus_ux8406_kde_fakes::{Compositor, Output, Style};
use rstest::rstest;

use crate::{LOWER, UPPER, session, until};

#[rstest]
#[case(0, Policy::Never)]
#[case(1, Policy::InTabletMode)]
#[case(2, Policy::Always)]
fn the_policy_of_each_panel_is_read(#[case] value: u32, #[case] policy: Policy) {
    let compositor = Compositor::new(
        Style::Registry,
        vec![
            Output {
                policy: value,
                ..Output::panel(UPPER, 0, 0)
            },
            Output::monitor("DP-1", 1920, 1080, 2304, 0),
        ],
    );

    // An external monitor has a policy too, but it does not count.
    assert_eq!(
        session(&compositor).outputs.policies(),
        [(UPPER.to_owned(), policy)]
    );
}

#[rstest]
#[case(Policy::Never, 0)]
#[case(Policy::InTabletMode, 1)]
#[case(Policy::Always, 2)]
fn a_policy_is_set_on_the_outputs_named(#[case] policy: Policy, #[case] value: u32) {
    let compositor = Compositor::new(
        Style::Globals,
        vec![Output::panel(UPPER, 0, 0), Output::panel(LOWER, 0, 1440)],
    );
    let mut session = session(&compositor);
    let number = session
        .set_policies(&[(UPPER.to_owned(), policy)])
        .expect("KWin 6.6 has the policy");

    session.flush().expect("flush");
    assert_eq!(crate::wait(&mut session, number).expect("answer"), Ok(()));
    assert_eq!(
        compositor.requests(),
        [format!("{UPPER} policy {value}"), "apply".to_owned()]
    );
    until(&mut session, "the new policy", |s| {
        s.outputs.policies()[0] == (UPPER.to_owned(), policy)
    });
    assert_eq!(compositor.output(LOWER).policy, 1);
}

#[test]
fn an_old_kwin_has_no_policy() {
    let compositor = Compositor::new(Style::Old, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);

    assert_eq!(session.outputs.policies(), []);
    assert_eq!(
        session.set_policies(&[(UPPER.to_owned(), Policy::Never)]),
        None
    );
}

#[test]
fn the_first_kwin_with_the_policy_sets_it() {
    let compositor = Compositor::new(Style::FirstPolicy, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);
    let number = session
        .set_policies(&[(UPPER.to_owned(), Policy::Never)])
        .expect("management version 5 has the policy");

    session.flush().expect("flush");
    assert_eq!(crate::wait(&mut session, number).expect("answer"), Ok(()));
    assert_eq!(
        compositor.requests(),
        [format!("{UPPER} policy 0"), "apply".to_owned()]
    );
}

#[test]
fn a_change_in_the_settings_is_seen() {
    let compositor = Compositor::new(Style::Registry, vec![Output::panel(UPPER, 0, 0)]);
    let mut session = session(&compositor);

    compositor.set_policy(UPPER, 2);
    until(&mut session, "the policy of the user", |s| {
        s.outputs.policies() == [(UPPER.to_owned(), Policy::Always)]
    });
}

#[rstest]
#[case("never", Some(Policy::Never))]
#[case("in-tablet-mode", Some(Policy::InTabletMode))]
#[case("always", Some(Policy::Always))]
#[case("InTabletMode", None)]
fn policy_names(#[case] text: &str, #[case] policy: Option<Policy>) {
    assert_eq!(Policy::parse(text), policy);
    if let Some(policy) = policy {
        assert_eq!(policy.as_str(), text);
    }
}
