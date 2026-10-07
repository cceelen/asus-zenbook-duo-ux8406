// Reading the display layout from the compositor and giving it one, through
// org.gnome.Mutter.DisplayConfig. No GNOME Shell imports here, so that it can
// be tried from the command line (the tools in the tests/ directories).
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {readState, toConfig} from './state.js';

const NAME = 'org.gnome.Mutter.DisplayConfig';
const PATH = '/org/gnome/Mutter/DisplayConfig';
const LAYOUT_MODE = {logical: 1, physical: 2};

/** How ApplyMonitorsConfig is to treat a layout. */
export const Method = {VERIFY: 0, TEMPORARY: 1};

function call(method, parameters) {
    return new Promise((resolve, reject) => {
        Gio.DBus.session.call(
            NAME,
            PATH,
            NAME,
            method,
            parameters,
            null,
            Gio.DBusCallFlags.NONE,
            -1,
            null,
            (connection, result) => {
                try {
                    resolve(connection.call_finish(result));
                } catch (error) {
                    reject(error);
                }
            },
        );
    });
}

/** The compositor's state, as state.js reads it. */
export async function currentState() {
    const reply = await call('GetCurrentState', null);

    return readState(reply.recursiveUnpack());
}

/** The settings of one monitor that are kept, as ApplyMonitorsConfig takes them. */
function monitorSettings({colorMode, rgbRange}) {
    const settings = {};

    if (colorMode !== undefined) settings['color-mode'] = new GLib.Variant('u', colorMode);
    if (rgbRange !== undefined) settings['rgb-range'] = new GLib.Variant('u', rgbRange);
    return settings;
}

/**
 * Give the compositor a layout: for this session only (it does not touch the
 * saved monitors.xml), or just to have it checked.
 */
export async function applyLayout(state, logical, method = Method.TEMPORARY) {
    const config = toConfig(logical, state.connected);

    if (!config) throw new Error('a monitor of the layout has no mode');

    const monitors = config.map(([x, y, scale, transform, primary, shown]) => [
        x,
        y,
        scale,
        transform,
        primary,
        shown.map(([connector, mode, settings]) => [connector, mode, monitorSettings(settings)]),
    ]);
    const properties = {};

    if (state.canSetMode) {
        properties['layout-mode'] = new GLib.Variant(
            'u',
            state.logicalMode ? LAYOUT_MODE.logical : LAYOUT_MODE.physical,
        );
    }
    await call(
        'ApplyMonitorsConfig',
        new GLib.Variant('(uua(iiduba(ssa{sv}))a{sv})', [
            state.serial,
            method,
            monitors,
            properties,
        ]),
    );
}
