// Has the running compositor check the current layout and the layout the key
// would ask for. It uses Method.VERIFY, so nothing is applied. Manual tool, not
// part of `meson test`: it needs a running GNOME session (D-Bus session bus and
// Mutter). Run it from the gnome-keys directory:
//   gjs -m tests/verify.js
import GLib from 'gi://GLib';

import {applyLayout, currentState, Method} from '../extension/display.js';
import {toggle} from '../extension/layout.js';

const loop = new GLib.MainLoop(null, false);

try {
    const state = await currentState();
    const {apply} = toggle(state, null, 'eDP-2', 'eDP-1');

    print(
        `now: ${state.logical.map((m) => `${m.connectors}@${m.x},${m.y}${m.primary ? ' primary' : ''}`).join('; ')}`,
    );
    await applyLayout(state, state.logical, Method.VERIFY);
    print('the current layout is accepted');
    if (apply) {
        print(
            `key: ${apply.map((m) => `${m.connectors}@${m.x},${m.y}${m.primary ? ' primary' : ''}`).join('; ')}`,
        );
        await applyLayout(state, apply, Method.VERIFY);
        print('the layout the key would ask for is accepted');
    } else {
        print('the key would do nothing right now');
    }
} catch (error) {
    printerr(`failed: ${error.message}`);
    imports.system.exit(1);
}
loop.quit();
