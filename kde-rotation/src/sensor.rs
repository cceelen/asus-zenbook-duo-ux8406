//! The orientation of the laptop, from iio-sensor-proxy
//! (`net.hadess.SensorProxy` on the system bus).
//!
//! The service reads the accelerometer while a program has a claim on it,
//! and tells the orientation only to a connection with a claim. It keeps
//! one claim for each connection and takes it back when the connection
//! closes. The orientation comes from the accelerometer only, as KWin's
//! own rotation does: it tells where gravity is.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

use asus_ux8406_kde_common::rotation::Orientation;
use log::warn;
use zbus::MatchRule;
use zbus::blocking::{Connection, MessageIterator};
use zbus::message::Type;
use zbus::zvariant::OwnedValue;

/// The service, its interface and its object.
const SERVICE: &str = "net.hadess.SensorProxy";
const PATH: &str = "/net/hadess/SensorProxy";
const PROPERTIES: &str = "org.freedesktop.DBus.Properties";

/// The errors of the bus that say the service did not answer in time: the
/// claim is tried again.
const NO_ANSWER: [&str; 3] = [
    "org.freedesktop.DBus.Error.NoReply",
    "org.freedesktop.DBus.Error.Timeout",
    "org.freedesktop.DBus.Error.TimedOut",
];

/// How long the worker waits for a job when it has no claim to try again.
const IDLE: Duration = Duration::from_secs(86_400);

/// The service on the system bus.
#[derive(Clone)]
pub struct Sensor {
    connection: Connection,
}

/// What changed at the service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// A property: the orientation, or whether there is an accelerometer.
    Property,
    /// The service started, or started again, or stopped.
    Owner,
}

impl Sensor {
    /// The service on the system bus; it need not run yet.
    ///
    /// # Errors
    ///
    /// There is no system bus.
    pub fn connect() -> zbus::Result<Self> {
        Connection::system().map(Self::on)
    }

    /// The service on this connection.
    #[must_use]
    pub fn on(connection: Connection) -> Self {
        Self { connection }
    }

    fn property(&self, name: &str) -> zbus::Result<OwnedValue> {
        self.connection
            .call_method(
                Some(SERVICE),
                PATH,
                Some(PROPERTIES),
                "Get",
                &(SERVICE, name),
            )
            .and_then(|reply| reply.body().deserialize::<OwnedValue>())
    }

    /// Whether the service runs and has an accelerometer. The service is
    /// not started for this: it starts by itself when the kernel has a
    /// sensor.
    #[must_use]
    pub fn present(&self) -> bool {
        self.connection
            .call_method(
                Some("org.freedesktop.DBus"),
                "/org/freedesktop/DBus",
                Some("org.freedesktop.DBus"),
                "NameHasOwner",
                &(SERVICE,),
            )
            .and_then(|reply| reply.body().deserialize::<bool>())
            .is_ok_and(|running| running)
            // No answer, or one of another type, means no accelerometer: the
            // service has none to claim.
            && self
                .property("HasAccelerometer")
                .is_ok_and(|value| bool::try_from(value).unwrap_or_default())
    }

    /// Whether the service runs and has an accelerometer, and the
    /// orientation. Each call waits for the service.
    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            present: self.present(),
            orientation: self.orientation(),
        }
    }

    /// The orientation now; `undefined` while the sensor gives none.
    #[must_use]
    pub fn orientation(&self) -> Orientation {
        // No answer is the documented `undefined`: the service does not run, or
        // the sensor gives none.
        self.property("AccelerometerOrientation")
            .ok()
            .and_then(|value| String::try_from(value).ok())
            .map_or(Orientation::Undefined, |text| {
                Orientation::from_sensor(&text)
            })
    }

    /// Claim the accelerometer, or give the claim back.
    ///
    /// # Errors
    ///
    /// The service did not take the call.
    pub fn claim(&self, claim: bool) -> zbus::Result<()> {
        let method = if claim {
            "ClaimAccelerometer"
        } else {
            "ReleaseAccelerometer"
        };

        self.connection
            .call_method(Some(SERVICE), PATH, Some(SERVICE), method, &())
            .map(drop)
    }

    /// Call `changed` in a thread of its own for each change at the
    /// service.
    ///
    /// # Errors
    ///
    /// The changes cannot be followed.
    pub fn on_change(&self, changed: impl Fn(Change) + Send + Clone + 'static) -> zbus::Result<()> {
        let properties = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(SERVICE)
            .and_then(|rule| rule.interface(PROPERTIES))
            .and_then(|rule| rule.member("PropertiesChanged"))
            .and_then(|rule| rule.path(PATH))
            .map(zbus::match_rule::Builder::build);
        let owner = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender("org.freedesktop.DBus")
            .and_then(|rule| rule.member("NameOwnerChanged"))
            .and_then(|rule| rule.add_arg(SERVICE))
            .map(zbus::match_rule::Builder::build);

        [(properties, Change::Property), (owner, Change::Owner)]
            .into_iter()
            .try_for_each(|(rule, change)| {
                let changed = changed.clone();

                rule.and_then(|rule| MessageIterator::for_match_rule(rule, &self.connection, None))
                    .map(|messages| {
                        std::thread::spawn(move || messages.for_each(|_| changed(change)));
                    })
            })
    }
}

/// What the service says now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    /// Whether the service runs and has an accelerometer.
    pub present: bool,
    /// The orientation; `undefined` without a claim.
    pub orientation: Orientation,
}

/// What the worker tells the program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Report {
    /// The service as it is now, after this change.
    State {
        /// What made the worker look.
        change: Change,
        /// What it found.
        snapshot: Snapshot,
    },
    /// The claim is as wanted (`true`: held), or the service refused the
    /// call (`false`) and the worker gave up.
    Claimed(bool),
}

/// The pause between two tries of a claim: it doubles up to `last`.
#[derive(Clone, Copy, Debug)]
pub struct Backoff {
    /// The first pause.
    pub first: Duration,
    /// The longest pause.
    pub last: Duration,
}

impl Backoff {
    /// The pauses of the program.
    pub const DEFAULT: Self = Self {
        first: Duration::from_secs(2),
        last: Duration::from_secs(60),
    };
}

enum Job {
    Refresh(Change),
    Claim(bool),
}

/// A thread that talks to the service, so that the program never waits for
/// it: a service that answers late, or not at all, delays only the reports.
/// A claim that gets no answer is tried again, after a pause that grows.
pub struct Worker {
    jobs: mpsc::Sender<Job>,
}

impl Worker {
    /// Start the thread, and follow the changes at the service: each one
    /// makes the worker look and report.
    ///
    /// # Errors
    ///
    /// The changes cannot be followed.
    pub fn spawn(
        sensor: Sensor,
        backoff: Backoff,
        report: impl Fn(Report) + Send + 'static,
    ) -> zbus::Result<Self> {
        let (jobs, received) = mpsc::channel();
        let follower = jobs.clone();

        sensor
            .on_change(move |change| {
                // The worker ends with the program: a change is of no use then.
                let _ = follower.send(Job::Refresh(change));
            })
            .map(|()| {
                std::thread::spawn(move || work(&sensor, backoff, &received, &report));
                Self { jobs }
            })
    }

    /// Look at the service, and report.
    pub fn refresh(&self, change: Change) {
        // The worker ends with the program: a request is of no use then.
        let _ = self.jobs.send(Job::Refresh(change));
    }

    /// Claim the accelerometer, or give the claim back, and report.
    pub fn claim(&self, claim: bool) {
        // The worker ends with the program: a request is of no use then.
        let _ = self.jobs.send(Job::Claim(claim));
    }
}

fn work(sensor: &Sensor, backoff: Backoff, jobs: &mpsc::Receiver<Job>, report: &impl Fn(Report)) {
    let mut wish: Option<bool> = None;
    let mut pause = backoff.first;

    loop {
        let due = match jobs.recv_timeout(wish.map_or(IDLE, |_| pause)) {
            Ok(Job::Refresh(change)) => {
                report(Report::State {
                    change,
                    snapshot: sensor.snapshot(),
                });
                false
            }
            Ok(Job::Claim(claim)) => {
                wish = Some(claim);
                pause = backoff.first;
                true
            }
            Err(RecvTimeoutError::Timeout) => true,
            Err(RecvTimeoutError::Disconnected) => return,
        };

        if let Some(claim) = wish.filter(|_| due) {
            match sensor.claim(claim) {
                Ok(()) => {
                    wish = None;
                    report(Report::Claimed(claim));
                    report(Report::State {
                        change: Change::Property,
                        snapshot: sensor.snapshot(),
                    });
                }
                Err(zbus::Error::MethodError(name, ..)) if NO_ANSWER.contains(&name.as_str()) => {
                    warn!("iio-sensor-proxy: no answer, the claim is tried again in {pause:?}");
                    pause = (pause * 2).min(backoff.last);
                }
                Err(error) => {
                    warn!("iio-sensor-proxy: {error}");
                    wish = None;
                    report(Report::Claimed(false));
                }
            }
        }
    }
}
