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

import {applyLayout, currentState, Method} from './display.js';
import {panelsOf, rotate} from './rotation.js';
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
const RotationIndicator = GObject.registerClass(
    class RotationIndicator extends SystemIndicator {
        constructor(settings) {
            super();
            this._toggle = new QuickToggle({
                title: 'Auto-rotate',
                iconName: 'rotation-allowed-symbolic',
                toggleMode: true,
            });
            settings.bind(LOCK, this._toggle, 'checked', Gio.SettingsBindFlags.INVERT_BOOLEAN);
            this.quickSettingsItems.push(this._toggle);

            this._monitors = global.backend.get_monitor_manager();
            this._managed = this._monitors.connect('notify::panel-orientation-managed', () =>
                this._sync(),
            );
            this._sync();
        }

        _sync() {
            this._toggle.visible = !this._monitors.get_panel_orientation_managed();
        }

        destroy() {
            this._monitors.disconnect(this._managed);
            for (const item of this.quickSettingsItems) item.destroy();
            super.destroy();
        }
    },
);

export default class BuiltinScreenRotation extends Extension {
    enable() {
        // The layout from before the panels were turned.
        this._upright = null;
        this._settleId = 0;

        const schema = Gio.SettingsSchemaSource.get_default().lookup(TOUCHSCREEN, true);

        if (!schema?.has_key(LOCK)) return;

        this._settings = new Gio.Settings({settings_schema: schema});
        this._lockChanged = this._settings.connect(`changed::${LOCK}`, () => this._settle());
        this._indicator = new RotationIndicator(this._settings);
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        this._sensor = new OrientationSensor(() => this._settle());
        // A monitor came or went, as when the keyboard is put on the lower
        // panel: the layout that the compositor then takes is upright.
        this._monitorsChanged = Main.layoutManager.connect('monitors-changed', () =>
            this._settle(),
        );
    }

    disable() {
        if (this._settleId) GLib.source_remove(this._settleId);
        this._settleId = 0;
        if (this._monitorsChanged) Main.layoutManager.disconnect(this._monitorsChanged);
        this._monitorsChanged = 0;
        this._sensor?.destroy();
        this._sensor = null;
        this._indicator?.destroy();
        this._indicator = null;
        if (this._lockChanged) this._settings.disconnect(this._lockChanged);
        this._lockChanged = 0;
        this._settings = null;
        this._upright = null;
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
        if (!this._sensor || this._settings.get_boolean(LOCK)) return;

        const state = await currentState();
        const orientation = this._sensor.orientation;
        const layout = rotate(state, orientation, this._upright);

        if (!layout) return;
        // A layout that the compositor does not accept is not applied: the
        // check throws, and the panels stay as they are.
        await applyLayout(state, layout, Method.VERIFY);
        await applyLayout(state, layout);
        // Turned from upright: remember the layout, to put it back as it was.
        if (orientation === 'normal') this._upright = null;
        else if (panelsOf(state).every((panel) => panel.transform === 0))
            this._upright = state.logical;
    }
}
