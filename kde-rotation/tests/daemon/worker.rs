//! The worker for the sensor service: what it reports, and how it claims.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use asus_ux8406_kde_fakes::{Bus, Sensor as Fake};
use asus_ux8406_kde_rotation::sensor::{Backoff, Change, Report, Sensor, Worker};

const PATIENCE: Duration = Duration::from_secs(10);
const BACKOFF: Backoff = Backoff {
    first: Duration::from_millis(40),
    last: Duration::from_millis(100),
};

fn start(bus: &Bus) -> (Worker, Receiver<Report>) {
    let (sender, reports) = mpsc::channel();
    let worker = Worker::spawn(Sensor::on(bus.connect()), BACKOFF, move |report| {
        let _ = sender.send(report);
    })
    .expect("worker");

    (worker, reports)
}

fn next(reports: &Receiver<Report>) -> Report {
    reports.recv_timeout(PATIENCE).expect("a report")
}

#[test]
fn a_look_reports_the_service_as_it_is() {
    let bus = Bus::new();
    let fake = Fake::new(bus.address(), "left-up");
    let (worker, reports) = start(&bus);

    worker.refresh(Change::Owner);

    let Report::State { change, snapshot } = next(&reports) else {
        panic!("a state");
    };

    assert_eq!(change, Change::Owner);
    assert!(snapshot.present);
    fake.turn("right-up");
    assert!(matches!(
        next(&reports),
        Report::State {
            change: Change::Property,
            ..
        }
    ));
}

#[test]
fn a_claim_is_reported_and_followed_by_a_look() {
    let bus = Bus::new();
    let fake = Fake::new(bus.address(), "normal");
    let (worker, reports) = start(&bus);

    worker.claim(true);
    assert_eq!(next(&reports), Report::Claimed(true));
    assert!(matches!(next(&reports), Report::State { .. }));
    assert_eq!(fake.claims(), (1, 0));
    worker.claim(false);
    assert_eq!(next(&reports), Report::Claimed(false));
    assert_eq!(fake.claims(), (1, 1));
}

#[test]
fn a_claim_without_an_answer_is_tried_again_with_longer_pauses() {
    let bus = Bus::new();
    let fake = Fake::new(bus.address(), "normal");
    let (worker, reports) = start(&bus);

    fake.fail_claims(3);
    worker.claim(true);
    assert_eq!(next(&reports), Report::Claimed(true));
    assert_eq!(fake.claims(), (1, 0));
}

#[test]
fn a_refused_claim_is_not_tried_again() {
    let bus = Bus::new();
    let fake = Fake::new(bus.address(), "normal");
    let (worker, reports) = start(&bus);

    fake.refuse_claims();
    worker.claim(true);
    assert_eq!(next(&reports), Report::Claimed(false));
    std::thread::sleep(Duration::from_millis(300));
    assert!(reports.try_recv().is_err());
    assert_eq!(fake.claims(), (0, 0));
}

#[test]
fn a_new_job_replaces_a_claim_that_waits_to_be_tried_again() {
    let bus = Bus::new();
    let fake = Fake::new(bus.address(), "normal");
    let (worker, reports) = start(&bus);

    fake.fail_claims(100);
    worker.claim(true);
    std::thread::sleep(Duration::from_millis(300));
    worker.claim(false);
    fake.fail_claims(0);
    assert_eq!(next(&reports), Report::Claimed(false));
}

#[test]
fn a_worker_that_is_let_go_ends() {
    let bus = Bus::new();
    let _fake = Fake::new(bus.address(), "normal");
    let (worker, reports) = start(&bus);

    drop(worker);
    std::thread::sleep(Duration::from_millis(200));
    assert!(reports.try_recv().is_err());
}
