// ASUS Zenbook Duo UX8406: two keys of the detachable keyboard.
//
// - The key right of F12 switches the lower panel on and off, by giving the
//   compositor a layout without it and, at the next press, the layout from
//   before. The layout is for the session only; monitors.xml is not touched.
//   With the keyboard lying on the lower panel the panel is not connected
//   (asus-zenbook-duo-ux8406-second-screen sees to that) and the key does nothing.
// - F8 swaps the windows of the two panels.
//
// The keyboard delivers these keys only with asus-zenbook-duo-ux8406-keyboard-bpf, as F18
// (XF86Launch9) and F19. On any other model the extension binds nothing.
//
// It also tells the compositor which panel each touchscreen and pen belongs
// to, where the user has set nothing: refer to touch.js.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

import {applyLayout, currentState} from './display.js';
import {toggle} from './layout.js';
import {find} from './state.js';
import {assignments, controllersOf} from './touch.js';

const MODEL = 'UX8406';
const UPPER = 'eDP-1';
const LOWER = 'eDP-2';
const KEYS = ['toggle-lower-panel', 'swap-windows'];
const I2C_DEVICES = '/sys/bus/i2c/devices';
// The settings of a touchscreen and of a pen, each at .../<group>/<id>/.
const TOUCH_SETTINGS = [
    ['org.gnome.desktop.peripherals.touchscreen', 'touchscreens'],
    ['org.gnome.desktop.peripherals.tablet', 'tablets'],
];

function isModel() {
    try {
        const [, contents] = GLib.file_get_contents('/sys/class/dmi/id/product_name');

        return new TextDecoder().decode(contents).includes(MODEL);
    } catch {
        return false;
    }
}

/** The names in a directory; none if it cannot be read. */
function namesIn(path) {
    try {
        const entries = Gio.File.new_for_path(path).enumerate_children(
            'standard::name',
            Gio.FileQueryInfoFlags.NONE,
            null,
        );
        const names = [];

        for (let info = entries.next_file(null); info; info = entries.next_file(null))
            names.push(info.get_name());
        return names;
    } catch {
        return [];
    }
}

/** The index the compositor gives the monitor at this place of the layout. */
function monitorAt({x, y}) {
    const display = global.display;

    for (let index = 0; index < display.get_n_monitors(); index++) {
        const geometry = display.get_monitor_geometry(index);

        if (geometry.x === x && geometry.y === y) return index;
    }
    return -1;
}

/** Move a window to another monitor, to the same place on it. */
function moveTo(window, target) {
    const display = global.display;
    const from = display.get_monitor_geometry(window.get_monitor());
    const to = display.get_monitor_geometry(target);
    const frame = window.get_frame_rect();

    if (typeof window.move_to_monitor === 'function') window.move_to_monitor(target);
    else window.move_frame(true, to.x + frame.x - from.x, to.y + frame.y - from.y);
}

export default class ZenbookDuoKeys extends Extension {
    enable() {
        // The layout saved when the key switched the lower panel off.
        this._saved = null;
        this._bound = false;
        if (!isModel()) return;

        this._settings = this.getSettings();
        this._bind('toggle-lower-panel', () => this._toggleLowerPanel());
        this._bind('swap-windows', () => this._swapWindows());
        this._bound = true;

        // The touch controllers do not change while the machine runs.
        this._controllers = controllersOf(
            namesIn(I2C_DEVICES).map((name) => [name, namesIn(`${I2C_DEVICES}/${name}`)]),
        );
        // The lower panel can be away at the start (the keyboard lies on it):
        // look again when a monitor comes.
        this._monitorsChanged = Main.layoutManager.connect('monitors-changed', () =>
            this._assignTouch(),
        );
        this._assignTouch();
    }

    disable() {
        if (this._bound) for (const key of KEYS) Main.wm.removeKeybinding(key);
        this._bound = false;
        if (this._monitorsChanged) Main.layoutManager.disconnect(this._monitorsChanged);
        this._monitorsChanged = 0;
        this._controllers = null;
        this._settings = null;
        this._saved = null;
    }

    _bind(key, action) {
        Main.wm.addKeybinding(
            key,
            this._settings,
            Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
            () =>
                action().catch((error) => console.error(`${this.uuid}: ${key}: ${error.message}`)),
        );
    }

    /**
     * Give each touchscreen and pen its panel, where the user has set
     * nothing. The settings stay when the extension is disabled: they are
     * right for the machine with or without it.
     */
    _assignTouch() {
        if (!this._controllers?.length) return;
        currentState()
            .then((state) => {
                const source = Gio.SettingsSchemaSource.get_default();

                for (const {id, output} of assignments(this._controllers, state, UPPER, LOWER)) {
                    for (const [schemaId, group] of TOUCH_SETTINGS) {
                        const schema = source.lookup(schemaId, true);

                        if (!schema?.has_key('output')) continue;

                        const settings = new Gio.Settings({
                            settings_schema: schema,
                            path: `/org/gnome/desktop/peripherals/${group}/${id}/`,
                        });

                        if (settings.get_user_value('output') === null)
                            settings.set_strv('output', output);
                    }
                }
            })
            .catch((error) => console.error(`${this.uuid}: touch: ${error.message}`));
    }

    async _toggleLowerPanel() {
        const state = await currentState();
        const {apply, save} = toggle(state, this._saved, LOWER, UPPER);

        if (apply) await applyLayout(state, apply);
        this._saved = save;
    }

    async _swapWindows() {
        const {logical} = await currentState();
        const upper = find(logical, UPPER);
        const lower = find(logical, LOWER);

        if (!upper || !lower) return;

        const [above, below] = [monitorAt(upper), monitorAt(lower)];

        if (above < 0 || below < 0) return;

        // Decide first where each window goes: a window moved in the loop
        // would otherwise be found on the other monitor and moved back.
        const moves = global
            .get_window_actors()
            .map((actor) => actor.meta_window)
            .filter((window) => window.get_window_type() === Meta.WindowType.NORMAL)
            .map((window) => [window, window.get_monitor()])
            .filter(([, monitor]) => monitor === above || monitor === below);
        for (const [window, monitor] of moves) moveTo(window, monitor === above ? below : above);
    }
}
