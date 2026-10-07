// The compositor's state as GetCurrentState gives it, with the layouts seen
// on the UX8406CA: what the tests of the two extensions build their cases
// from.
import {readState} from '../state.js';

export const UPPER = 'eDP-1';
export const LOWER = 'eDP-2';

export const mode = (current = true) => [
    '2880x1800@120.000',
    2880,
    1800,
    120,
    1.5,
    [1, 1.25],
    current ? {'is-current': true} : {'is-preferred': true},
];
// The compositor marks the panels of the machine as built-in.
export const monitor = (connector, current = true) => [
    [connector, 'SDC', '0x419d', '0x0'],
    [mode(current)],
    connector.startsWith('eDP') ? {'color-mode': 0, 'is-builtin': true} : {'color-mode': 0},
];
export const logical = (connector, x, y, primary, scale = 1.25) => [
    x,
    y,
    scale,
    0,
    primary,
    [[connector, 'SDC', '0x419d', '0x0']],
    {},
];
export const properties = {'layout-mode': 1, 'supports-changing-layout-mode': true};

// Both panels on: the lower one below the upper one, and the primary.
export const twoPanels = () =>
    readState([
        7,
        [monitor(UPPER), monitor(LOWER)],
        [logical(UPPER, 0, 0, false), logical(LOWER, 0, 1440, true)],
        properties,
    ]);
// The lower panel switched off; it is still connected.
export const lowerOff = () =>
    readState([
        8,
        [monitor(UPPER), monitor(LOWER, false)],
        [logical(UPPER, 0, 0, true)],
        properties,
    ]);
