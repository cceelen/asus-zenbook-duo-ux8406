// The layout of the built-in panels when the laptop is turned on its side.
// Made for the two panels of the ASUS Zenbook Duo UX8406; nothing in it is
// specific to that model.
//
// With the built-in panels on and the laptop on its left or right side, the
// panels are turned and put side by side; upright, the layout from before is
// put back. GNOME does this by itself only without a pointer device, and for
// one panel. The layouts are for the session only; monitors.xml is not
// touched. The switch "Auto-rotate" in the quick settings is GNOME's own
// setting orientation-lock.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {QuickToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';

import {applyLayout, currentState} from './display.js';
import {rotate, uprightAfter} from './rotation.js';
import {OrientationSensor} from './sensor.js';

// GNOME's own setting for the rotation of the built-in panel.
const TOUCHSCREEN = 'org.gnome.settings-daemon.peripherals.touchscreen';
const LOCK = 'orientation-lock';
// The laptop must hold an orientation this long before the panels follow.
const SETTLE_MS = 700;

/**
 * The quick settings switch for the rotation, on while it is not locked.
 * GNOME shows its own switch for the same setting while it turns the panel
 * itself (no pointer device); this one is then hidden.
 */
function rotationIndicator(settings) {
    const indicator = new SystemIndicator();
    const toggle = new QuickToggle({
        title: 'Auto-rotate',
        iconName: 'rotation-allowed-symbolic',
        toggleMode: true,
    });

    settings.bind(LOCK, toggle, 'checked', Gio.SettingsBindFlags.INVERT_BOOLEAN);
    // The binding ends when the switch is destroyed.
    global.backend
        .get_monitor_manager()
        .bind_property(
            'panel-orientation-managed',
            toggle,
            'visible',
            GObject.BindingFlags.SYNC_CREATE | GObject.BindingFlags.INVERT_BOOLEAN,
        );
    indicator.quickSettingsItems.push(toggle);
    return indicator;
}

export default class BuiltinScreenRotation extends Extension {
    // The layout from before the panels were turned. It stays when the
    // extension is disabled: the Shell does that at each screen lock, and the
    // laptop can be upright again only after it.
    _upright = null;

    enable() {
        this._settleId = 0;

        const schema = Gio.SettingsSchemaSource.get_default().lookup(TOUCHSCREEN, true);

        if (!schema?.has_key(LOCK)) return;

        this._settings = new Gio.Settings({settings_schema: schema});
        this._lockChanged = this._settings.connect(`changed::${LOCK}`, () => this._watch());
        this._indicator = rotationIndicator(this._settings);
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        // A monitor came or went, as when the keyboard is put on the lower
        // panel: the layout that the compositor then takes is upright.
        this._monitorsChanged = Main.layoutManager.connect('monitors-changed', () =>
            this._settle(),
        );
        this._watch();
    }

    disable() {
        if (this._settleId) GLib.source_remove(this._settleId);
        this._settleId = 0;
        if (this._monitorsChanged) Main.layoutManager.disconnect(this._monitorsChanged);
        this._monitorsChanged = 0;
        this._sensor?.destroy();
        this._sensor = null;
        for (const item of this._indicator?.quickSettingsItems ?? []) item.destroy();
        this._indicator?.destroy();
        this._indicator = null;
        if (this._lockChanged) this._settings.disconnect(this._lockChanged);
        this._lockChanged = 0;
        this._settings = null;
    }

    /**
     * Have the sensor only while the rotation is not locked: the service
     * reads the accelerometer as long as a program has a claim on it. A new
     * sensor tells its orientation, and the panels then follow it.
     */
    _watch() {
        if (this._settings.get_boolean(LOCK)) {
            this._sensor?.destroy();
            this._sensor = null;
        } else if (!this._sensor) {
            this._sensor = new OrientationSensor(() => this._settle());
        }
    }

    /** Follow the orientation once it has held for SETTLE_MS. */
    _settle() {
        if (this._settleId) GLib.source_remove(this._settleId);
        this._settleId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SETTLE_MS, () => {
            this._settleId = 0;
            this._rotate().catch((error) => console.error(`${this.uuid}: ${error.message}`));
            return GLib.SOURCE_REMOVE;
        });
    }

    async _rotate() {
        const sensor = this._sensor;

        // No sensor: the rotation is locked, or the extension is disabled.
        if (!sensor) return;

        const state = await currentState();

        // Locked or disabled while the compositor gave its answer: do no more.
        if (sensor !== this._sensor) return;

        const orientation = sensor.orientation;
        const layout = rotate(state, orientation, this._upright);

        if (!layout) return;
        // The compositor checks a layout before it takes it. One that it
        // does not accept is not applied: the call throws, and the panels
        // stay as they are.
        await applyLayout(state, layout);
        if (sensor === this._sensor)
            this._upright = uprightAfter(state, orientation, this._upright);
    }
}
