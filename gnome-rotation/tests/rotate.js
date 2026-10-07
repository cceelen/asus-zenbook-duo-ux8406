// Follows the orientation of the laptop from outside GNOME Shell, with the
// code of the extension: for a trial on the hardware without a new login.
// It prints each orientation and the layout it applies. The layouts are for
// the session only. Manual tool, not part of `meson test`: it needs a running
// GNOME session and iio-sensor-proxy. Disable the extension first, or the two
// act at the same time. It is not the extension in two points: it follows an
// orientation at once, not after the time that the extension waits, and it
// does not look at the setting orientation-lock. Run it from the
// gnome-rotation directory, with the number of seconds to run (default 60):
//   gjs -m tests/rotate.js 120
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

import {applyLayout, currentState} from '../extension/display.js';
import {rotate, uprightAfter} from '../extension/rotation.js';
import {OrientationSensor} from '../extension/sensor.js';

const NAME = 'org.gnome.Mutter.DisplayConfig';
const seconds = Number(ARGV[0] ?? 60);

if (!Number.isInteger(seconds) || seconds < 1) {
    printerr('usage: gjs -m tests/rotate.js [seconds]');
    imports.system.exit(2);
}

const loop = new GLib.MainLoop(null, false);
// The layout from before the panels were turned, as the extension keeps it.
let upright = null;
const show = (logical) =>
    logical.map((m) => `${m.connectors}@${m.x},${m.y} transform ${m.transform}`).join('; ');

/** Bring the layout in line with the orientation, as the extension does. */
async function follow(why) {
    try {
        const state = await currentState();
        const orientation = sensor.orientation;
        const layout = rotate(state, orientation, upright);

        print(`${why}, ${orientation}: now ${show(state.logical)}`);
        if (!layout) {
            print('  nothing to do');
            return;
        }
        await applyLayout(state, layout);
        upright = uprightAfter(state, orientation, upright);
        print(`  applied ${show(layout)}`);
    } catch (error) {
        printerr(`  failed: ${error.message}`);
    }
}

const sensor = new OrientationSensor(() => follow('orientation'));
// A monitor came or went, as when the keyboard is put on the lower panel.
const changed = Gio.DBus.session.signal_subscribe(
    NAME,
    NAME,
    'MonitorsChanged',
    '/org/gnome/Mutter/DisplayConfig',
    null,
    Gio.DBusSignalFlags.NONE,
    () => follow('monitors'),
);

GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, seconds, () => {
    Gio.DBus.session.signal_unsubscribe(changed);
    sensor.destroy();
    loop.quit();
    return GLib.SOURCE_REMOVE;
});
loop.run();
