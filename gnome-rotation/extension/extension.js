// The layout of the built-in panels when the laptop is turned on its side.
// Made for the two panels of the ASUS Zenbook Duo UX8406; nothing in it is
// specific to that model.
//
// With the built-in panels on and the laptop on its left or right side, the
// panels are turned and put side by side; upright, the layout from before is
// put back. GNOME does this by itself only without a pointer device, and for
// one panel. The layouts are for the session only; monitors.xml is not
// touched. The switch "Auto-rotate" in the quick settings is GNOME's own
// setting orientation-lock. Where no sensor gives the orientation, two
// buttons turn the panels by hand instead.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {QuickToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';

import {applyLayout, currentState} from './display.js';
import {control, orientationOf, rotate, turned, uprightAfter} from './rotation.js';
import {OrientationPresence, OrientationSensor} from './sensor.js';

// GNOME's own setting for the rotation of the built-in panel.
const TOUCHSCREEN = 'org.gnome.settings-daemon.peripherals.touchscreen';
const LOCK = 'orientation-lock';
// The laptop must hold an orientation this long before the panels follow.
const SETTLE_MS = 700;

/**
 * The quick settings switch for the rotation, on while it is not locked. It
 * is hidden until the extension knows that it can turn a panel
 * (_showControl).
 */
function rotationSwitch(settings) {
    const toggle = new QuickToggle({
        title: 'Auto-rotate',
        iconName: 'rotation-allowed-symbolic',
        toggleMode: true,
        visible: false,
    });

    settings.bind(LOCK, toggle, 'checked', Gio.SettingsBindFlags.INVERT_BOOLEAN);
    return toggle;
}

/**
 * Two buttons in the place of one switch: each turns the panels by 90
 * degrees. `turn(direction)` is called with 'counterclockwise' or
 * 'clockwise'. Hidden, as the switch.
 */
function rotationButtons(turn) {
    const box = new St.BoxLayout({x_expand: true, style: 'spacing: 6px;', visible: false});
    const buttons = {};

    for (const [direction, icon, name] of [
        ['counterclockwise', 'object-rotate-left-symbolic', 'Turn counterclockwise'],
        ['clockwise', 'object-rotate-right-symbolic', 'Turn clockwise'],
    ]) {
        const button = new St.Button({
            style_class: 'quick-toggle',
            x_expand: true,
            can_focus: true,
            accessible_name: name,
            child: new St.Icon({icon_name: icon, style_class: 'quick-toggle-icon'}),
        });

        button.connect('clicked', () => turn(direction));
        box.add_child(button);
        buttons[direction] = button;
    }
    return {box, buttons};
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
        this._indicator = new SystemIndicator();
        this._switch = rotationSwitch(this._settings);
        ({box: this._box, buttons: this._buttons} = rotationButtons((direction) =>
            this._turn(direction).catch((error) => console.error(`${this.uuid}: ${error.message}`)),
        ));
        this._indicator.quickSettingsItems.push(this._switch, this._box);
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        // What decides which control is shown. The sensor is followed without
        // a claim, also while the rotation is locked.
        this._builtin = false;
        this._control = 'none';
        // The orientation that the buttons turned the panels to.
        this._manual = null;
        this._monitorManager = global.backend.get_monitor_manager();
        this._managedChanged = this._monitorManager.connect(
            'notify::panel-orientation-managed',
            () => this._showControl(),
        );
        this._presence = new OrientationPresence(() => this._showControl());
        // A monitor came or went, as when the keyboard is put on the lower
        // panel: the layout that the compositor then takes is upright.
        this._monitorsChanged = Main.layoutManager.connect('monitors-changed', () => {
            this._findBuiltin();
            this._settle();
        });
        this._findBuiltin();
        this._watch();
    }

    disable() {
        if (this._settleId) GLib.source_remove(this._settleId);
        this._settleId = 0;
        if (this._monitorsChanged) Main.layoutManager.disconnect(this._monitorsChanged);
        this._monitorsChanged = 0;
        this._sensor?.destroy();
        this._sensor = null;
        this._presence?.destroy();
        this._presence = null;
        if (this._managedChanged) this._monitorManager.disconnect(this._managedChanged);
        this._managedChanged = 0;
        this._monitorManager = null;
        for (const item of this._indicator?.quickSettingsItems ?? []) item.destroy();
        this._indicator?.destroy();
        this._indicator = null;
        this._switch = null;
        this._box = null;
        this._buttons = null;
        if (this._lockChanged) this._settings.disconnect(this._lockChanged);
        this._lockChanged = 0;
        this._settings = null;
    }

    /**
     * Whether the compositor has a built-in panel, then the control. The
     * buttons start from the orientation that the panels have.
     */
    _findBuiltin() {
        const indicator = this._indicator;

        currentState()
            .then((state) => {
                // Disabled while the compositor gave its answer.
                if (indicator !== this._indicator) return;
                this._builtin = state.builtin.length > 0;
                this._manual ??= orientationOf(state);
                this._showControl();
            })
            .catch((error) => console.error(`${this.uuid}: ${error.message}`));
    }

    _showControl() {
        this._control = control({
            managed: this._monitorManager.panel_orientation_managed,
            sensor: this._presence.present,
            builtin: this._builtin,
        });
        this._switch.visible = this._control === 'switch';
        this._box.visible = this._control === 'buttons';
        // A button that would turn past left side up or right side up does
        // nothing.
        for (const [direction, button] of Object.entries(this._buttons))
            button.reactive = turned(this._manual ?? 'normal', direction) !== null;
    }

    /** Turn the panels by 90 degrees from the orientation of the buttons. */
    async _turn(direction) {
        const orientation = turned(this._manual ?? 'normal', direction);

        if (!orientation) return;
        this._manual = orientation;
        this._showControl();
        await this._rotate();
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

    /**
     * The orientation that the panels follow: the one of the buttons where
     * they are shown, else the one of the sensor. Null for none: the
     * rotation is locked, or the extension is disabled.
     */
    _orientation() {
        if (this._control === 'buttons') return this._manual;
        return this._sensor?.orientation ?? null;
    }

    async _rotate() {
        const [indicator, sensor] = [this._indicator, this._sensor];

        if (!this._orientation()) return;

        const state = await currentState();

        // Locked or disabled while the compositor gave its answer: do no more.
        if (indicator !== this._indicator || sensor !== this._sensor) return;

        const orientation = this._orientation();
        const layout = rotate(state, orientation, this._upright);

        if (!layout) return;
        // The compositor checks a layout before it takes it. One that it
        // does not accept is not applied: the call throws, and the panels
        // stay as they are.
        await applyLayout(state, layout);
        if (indicator === this._indicator && sensor === this._sensor)
            this._upright = uprightAfter(state, orientation, this._upright);
    }
}
