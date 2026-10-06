// The orientation of the laptop, from iio-sensor-proxy (net.hadess.SensorProxy
// on the system bus). No GNOME Shell imports here.
import Gio from 'gi://Gio';

const NAME = 'net.hadess.SensorProxy';
const PATH = '/net/hadess/SensorProxy';

/**
 * Claims the accelerometer and calls `changed(orientation)` at each change:
 * 'normal', 'bottom-up', 'left-up', 'right-up' or 'undefined'.
 */
export class OrientationSensor {
    constructor(changed) {
        this._changed = changed;
        this._cancellable = new Gio.Cancellable();
        this._proxy = null;
        this._signals = [];
        Gio.DBusProxy.new_for_bus(
            Gio.BusType.SYSTEM,
            Gio.DBusProxyFlags.NONE,
            null,
            NAME,
            PATH,
            NAME,
            this._cancellable,
            (_source, result) => {
                try {
                    this._proxy = Gio.DBusProxy.new_for_bus_finish(result);
                } catch {
                    // Cancelled, or there is no system bus.
                    return;
                }
                this._signals = [
                    this._proxy.connect('g-properties-changed', () => this._report()),
                    // The service started or started again: claim once more.
                    this._proxy.connect('notify::g-name-owner', () => this._claim()),
                ];
                this._claim();
            },
        );
    }

    /** The orientation now; 'undefined' while the sensor has none. */
    get orientation() {
        const value = this._proxy?.get_cached_property('AccelerometerOrientation');

        return value ? value.unpack() : 'undefined';
    }

    _report() {
        this._changed(this.orientation);
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
                    // Cancelled, or the machine has no accelerometer.
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
        if (this._proxy?.g_name_owner) {
            this._proxy.call(
                'ReleaseAccelerometer',
                null,
                Gio.DBusCallFlags.NONE,
                -1,
                null,
                null,
            );
        }
        this._proxy = null;
    }
}
