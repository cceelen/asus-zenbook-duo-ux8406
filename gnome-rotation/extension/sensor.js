// The orientation of the laptop, from iio-sensor-proxy (net.hadess.SensorProxy
// on the system bus). No GNOME Shell imports here.
import Gio from 'gi://Gio';

const NAME = 'net.hadess.SensorProxy';
const PATH = '/net/hadess/SensorProxy';

/**
 * Claims the accelerometer and calls `changed()` each time `orientation` is
 * another one: 'normal', 'bottom-up', 'left-up', 'right-up' or 'undefined'.
 *
 * The service keeps one claim for each connection, whatever the number of
 * its users, and tells the orientation only to a connection that has one. In
 * GNOME Shell the compositor uses the connection of the process, and gives
 * its claim back while it does not turn the panel itself. Thus this class
 * has a connection of its own: the compositor cannot take its claim, and it
 * cannot take the compositor's.
 */
export class OrientationSensor {
    constructor(changed) {
        this._changed = changed;
        this._cancellable = new Gio.Cancellable();
        this._connection = null;
        this._proxy = null;
        this._signals = [];
        // The orientation at the last call of `changed`; null before the first.
        this._reported = null;
        Gio.DBusConnection.new_for_address(
            Gio.dbus_address_get_for_bus_sync(Gio.BusType.SYSTEM, null),
            Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT |
                Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION,
            null,
            this._cancellable,
            (_source, result) => {
                try {
                    this._connection = Gio.DBusConnection.new_for_address_finish(result);
                } catch {
                    // Cancelled, or there is no system bus.
                    return;
                }
                this._watch();
            },
        );
    }

    /** The orientation now; 'undefined' while the sensor has none. */
    get orientation() {
        const value = this._proxy?.get_cached_property('AccelerometerOrientation');

        return value ? value.unpack() : 'undefined';
    }

    _report() {
        const orientation = this.orientation;

        // The service tells the tilt of the laptop too: that is no change.
        if (orientation === this._reported) return;
        this._reported = orientation;
        this._changed();
    }

    /** Follow the service on the connection, and claim the accelerometer. */
    _watch() {
        Gio.DBusProxy.new(
            this._connection,
            Gio.DBusProxyFlags.NONE,
            null,
            NAME,
            PATH,
            NAME,
            this._cancellable,
            (_source, result) => {
                try {
                    this._proxy = Gio.DBusProxy.new_finish(result);
                } catch {
                    // Cancelled, or the connection closed.
                    return;
                }
                this._signals = [
                    this._proxy.connect('g-properties-changed', () => this._report()),
                    // The service started or started again: claim once more.
                    this._proxy.connect('notify::g-name-owner', () => this._claim()),
                ];
                this._claim();
                // The orientation that the service has now. The answer to
                // the claim can come much later: the service holds it back
                // until the sensor gives a value.
                this._report();
            },
        );
    }

    _claim() {
        if (!this._proxy.g_name_owner) return;
        this._proxy.call(
            'ClaimAccelerometer',
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            this._cancellable,
            (proxy, result) => {
                try {
                    proxy.call_finish(result);
                } catch {
                    // Cancelled, not permitted, or no answer in time. In the
                    // last case the service has the claim all the same.
                    return;
                }
                this._report();
            },
        );
    }

    /** Give the accelerometer back and call `changed` no more. */
    destroy() {
        this._cancellable.cancel();
        for (const signal of this._signals) this._proxy.disconnect(signal);
        this._signals = [];
        this._proxy = null;
        // The service takes the claim back from a connection that closes.
        this._connection?.close(null, null);
        this._connection = null;
    }
}
