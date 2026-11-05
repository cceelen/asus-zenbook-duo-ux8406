//! A fake of iio-sensor-proxy (`net.hadess.SensorProxy` on the system bus):
//! the orientation of the laptop, and the claims on the accelerometer.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use zbus::blocking::Connection;
use zbus::interface;

/// The service and its interface.
pub const SERVICE: &str = "net.hadess.SensorProxy";
/// Its object.
pub const PATH: &str = "/net/hadess/SensorProxy";

#[derive(Debug, Default)]
struct Counts {
    claimed: u32,
    released: u32,
    refuse: bool,
    /// How many of the next claims get no answer but an error of the bus.
    fail: u32,
    /// How long each claim waits before it is answered.
    delay: Duration,
}

struct Proxy {
    present: bool,
    orientation: String,
    claims: Arc<Mutex<Counts>>,
}

#[interface(name = "net.hadess.SensorProxy")]
impl Proxy {
    async fn claim_accelerometer(&self) -> zbus::fdo::Result<()> {
        let delay = self
            .claims
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .delay;

        async_io::Timer::after(delay).await;

        let mut claims = self.claims.lock().unwrap_or_else(PoisonError::into_inner);

        if claims.fail > 0 {
            claims.fail -= 1;
            return Err(zbus::fdo::Error::NoReply("no answer".to_owned()));
        }
        if claims.refuse {
            return Err(zbus::fdo::Error::AccessDenied("not permitted".to_owned()));
        }
        claims.claimed += 1;
        Ok(())
    }

    fn release_accelerometer(&self) {
        self.claims
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .released += 1;
    }

    #[zbus(property)]
    fn has_accelerometer(&self) -> bool {
        self.present
    }

    #[zbus(property)]
    fn accelerometer_orientation(&self) -> String {
        self.orientation.clone()
    }
}

/// The fake service.
pub struct Sensor {
    address: String,
    connection: Option<Connection>,
    claims: Arc<Mutex<Counts>>,
}

impl Sensor {
    /// A sensor that gives this orientation: `normal`, `left-up`,
    /// `right-up`, `bottom-up` or `undefined`.
    #[must_use]
    pub fn new(address: &str, orientation: &str) -> Self {
        let mut sensor = Self {
            address: address.to_owned(),
            connection: None,
            claims: Arc::new(Mutex::new(Counts::default())),
        };

        sensor.start(true, orientation);
        sensor
    }

    /// Start the service (again), with or without an accelerometer.
    ///
    /// # Panics
    ///
    /// If the bus does not take the service.
    pub fn start(&mut self, present: bool, orientation: &str) {
        self.connection = None;
        self.connection = Some(
            zbus::blocking::connection::Builder::address(self.address.as_str())
                .and_then(|builder| {
                    builder.serve_at(
                        PATH,
                        Proxy {
                            present,
                            orientation: orientation.to_owned(),
                            claims: Arc::clone(&self.claims),
                        },
                    )
                })
                .and_then(|builder| builder.name(SERVICE))
                .and_then(zbus::blocking::connection::Builder::build)
                .expect("sensor service"),
        );
    }

    /// Stop the service.
    pub fn stop(&mut self) {
        self.connection = None;
    }

    fn proxy(&self) -> zbus::blocking::object_server::InterfaceRef<Proxy> {
        self.connection
            .as_ref()
            .expect("a running service")
            .object_server()
            .interface::<_, Proxy>(PATH)
            .expect("the sensor object")
    }

    /// Turn the laptop: the service tells the new orientation.
    ///
    /// # Panics
    ///
    /// If the service is not running or the signal cannot be sent.
    pub fn turn(&self, orientation: &str) {
        let proxy = self.proxy();
        let mut object = proxy.get_mut();

        orientation.clone_into(&mut object.orientation);
        zbus::block_on(object.accelerometer_orientation_changed(proxy.signal_emitter()))
            .expect("signal");
    }

    /// The accelerometer comes or goes, as when the kernel driver loads.
    ///
    /// # Panics
    ///
    /// If the service is not running or the signal cannot be sent.
    pub fn set_present(&self, present: bool) {
        let proxy = self.proxy();
        let mut object = proxy.get_mut();

        object.present = present;
        zbus::block_on(object.has_accelerometer_changed(proxy.signal_emitter())).expect("signal");
    }

    /// Refuse each claim from now on, as when polkit does not permit it.
    pub fn refuse_claims(&self) {
        self.claims
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .refuse = true;
    }

    /// Answer each claim only after this time.
    pub fn delay_claims(&self, delay: Duration) {
        self.claims
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .delay = delay;
    }

    /// Answer the next `count` claims with the error that the bus gives when
    /// a service does not answer in time.
    pub fn fail_claims(&self, count: u32) {
        self.claims
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .fail = count;
    }

    /// The claims and releases so far.
    #[must_use]
    pub fn claims(&self) -> (u32, u32) {
        let claims = self.claims.lock().unwrap_or_else(PoisonError::into_inner);

        (claims.claimed, claims.released)
    }
}
