// Tests for rotation.js, with the layouts seen on the UX8406CA. Run with
// `node --test`.
import assert from 'node:assert/strict';
import {test} from 'node:test';

import {isValid, rotate} from '../extension/rotation.js';
import {find, readState} from '../extension/state.js';

const UPPER = 'eDP-1';
const LOWER = 'eDP-2';

const mode = (current = true) => [
    '2880x1800@120.000',
    2880,
    1800,
    120,
    1.5,
    [1, 1.25],
    current ? {'is-current': true} : {'is-preferred': true},
];
// The compositor marks the panels of the machine as built-in.
const monitor = (connector, current = true) => [
    [connector, 'SDC', '0x419d', '0x0'],
    [mode(current)],
    connector.startsWith('eDP') ? {'color-mode': 0, 'is-builtin': true} : {'color-mode': 0},
];
const logical = (connector, x, y, primary, scale = 1.25) => [
    x,
    y,
    scale,
    0,
    primary,
    [[connector, 'SDC', '0x419d', '0x0']],
    {},
];
const properties = {'layout-mode': 1, 'supports-changing-layout-mode': true};

// Both panels on: the lower one below the upper one, and the primary.
const twoPanels = () =>
    readState([
        7,
        [monitor(UPPER), monitor(LOWER)],
        [logical(UPPER, 0, 0, false), logical(LOWER, 0, 1440, true)],
        properties,
    ]);
// The lower panel switched off; it is still connected.
const lowerOff = () =>
    readState([
        8,
        [monitor(UPPER), monitor(LOWER, false)],
        [logical(UPPER, 0, 0, true)],
        properties,
    ]);

// The laptop on its side. A panel of 2880x1800 at scale 1.25 takes 2304x1440
// in the layout, and 1440x2304 when it is turned.
const place = (layout, connector) => {
    const {x, y, transform} = find(layout, connector);

    return [x, y, transform];
};

test('left side up: both panels turned, the lower one on the left', () => {
    const layout = rotate(twoPanels(), 'left-up');

    assert.deepEqual(place(layout, LOWER), [0, 0, 1]);
    assert.deepEqual(place(layout, UPPER), [1440, 0, 1]);
    assert.equal(find(layout, LOWER).primary, true);
});

test('right side up: both panels turned, the lower one on the right', () => {
    const layout = rotate(twoPanels(), 'right-up');

    assert.deepEqual(place(layout, UPPER), [0, 0, 3]);
    assert.deepEqual(place(layout, LOWER), [1440, 0, 3]);
});

test('upright again: the lower panel below the upper one', () => {
    const turned = twoPanels();

    turned.logical = rotate(turned, 'left-up');

    const layout = rotate(turned, 'normal');

    assert.deepEqual(place(layout, UPPER), [0, 0, 0]);
    assert.deepEqual(place(layout, LOWER), [0, 1440, 0]);
});

test('from one side to the other', () => {
    const turned = twoPanels();

    turned.logical = rotate(turned, 'left-up');

    const layout = rotate(turned, 'right-up');

    assert.deepEqual(place(layout, UPPER), [0, 0, 3]);
    assert.deepEqual(place(layout, LOWER), [1440, 0, 3]);
});

test('with the lower panel off, only the upper one is turned', () => {
    const layout = rotate(lowerOff(), 'left-up');

    assert.equal(layout.length, 1);
    assert.deepEqual(place(layout, UPPER), [0, 0, 1]);
});

test('nothing to do when the layout is that one already', () => {
    assert.equal(rotate(twoPanels(), 'normal'), null);

    const turned = twoPanels();

    turned.logical = rotate(turned, 'right-up');
    assert.equal(rotate(turned, 'right-up'), null);
});

test('upside down and an unknown orientation change nothing', () => {
    assert.equal(rotate(twoPanels(), 'bottom-up'), null);
    assert.equal(rotate(twoPanels(), 'undefined'), null);
});

test('with the built-in panels off, nothing is turned', () => {
    const state = readState([
        3,
        [monitor(UPPER, false), monitor('DP-7')],
        [logical('DP-7', 0, 0, true, 1)],
        properties,
    ]);

    assert.equal(rotate(state, 'left-up'), null);
});

test('a monitor right of the panels stays right of them', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 0, 1440, true),
            logical('DP-7', 2304, 0, false, 1),
        ],
        properties,
    ]);
    const layout = rotate(state, 'left-up');

    // The panels are 2880 wide side by side, 576 more than before.
    assert.deepEqual(place(layout, 'DP-7'), [2880, 0, 0]);
    assert.deepEqual(place(layout, UPPER), [1440, 0, 1]);
});

test('a monitor left of the panels stays where it is', () => {
    const state = readState([
        5,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical('DP-7', 0, 0, false, 1),
            logical(UPPER, 2880, 0, false),
            logical(LOWER, 2880, 1440, true),
        ],
        properties,
    ]);
    const layout = rotate(state, 'right-up');

    assert.deepEqual(place(layout, 'DP-7'), [0, 0, 0]);
    assert.deepEqual(place(layout, UPPER), [2880, 0, 3]);
    assert.deepEqual(place(layout, LOWER), [4320, 0, 3]);
});

// A layout from the hardware: the upper panel below one monitor, another
// monitor at the top right that shares no row with the panel.
const below = () =>
    readState([
        6,
        [monitor(UPPER), monitor('DP-8'), monitor('DP-7')],
        [
            logical(UPPER, 760, 1800, false),
            logical('DP-8', 0, 0, true, 1),
            logical('DP-7', 2880, 0, false, 1),
        ],
        properties,
    ]);

test('a monitor that is not beside the panels stays where it is', () => {
    const turned = below();

    turned.logical = rotate(turned, 'left-up');
    assert.deepEqual(place(turned.logical, 'DP-7'), [2880, 0, 0]);

    const layout = rotate(turned, 'normal');

    assert.deepEqual(place(layout, 'DP-7'), [2880, 0, 0]);
    assert.deepEqual(place(layout, UPPER), [760, 1800, 0]);
});

test('upright again, the layout from before the turn is put back', () => {
    const before = twoPanels();
    const turned = twoPanels();

    turned.logical = rotate(turned, 'left-up');
    // Something moved the upper panel while the laptop lay on its side.
    find(turned.logical, UPPER).y = 100;

    const layout = rotate(turned, 'normal', before.logical);

    assert.deepEqual(layout, before.logical);
    assert.notEqual(layout[0], before.logical[0]);
});

test('the layout from before the turn is not used with other monitors', () => {
    const turned = lowerOff();

    turned.logical = rotate(turned, 'left-up');

    const layout = rotate(turned, 'normal', twoPanels().logical);

    assert.equal(layout.length, 1);
    assert.deepEqual(place(layout, UPPER), [0, 0, 0]);
});

test('upright with the layout from before the turn in place: nothing to do', () => {
    const state = twoPanels();

    assert.equal(rotate(state, 'normal', twoPanels().logical), null);
});

test('the order of the panels comes from the layout, not from their names', () => {
    const state = readState([
        10,
        [monitor(UPPER), monitor(LOWER)],
        [logical(LOWER, 0, 0, false), logical(UPPER, 0, 1440, true)],
        properties,
    ]);
    const layout = rotate(state, 'left-up');

    // eDP-1 is the bottom panel here: it goes to the left.
    assert.deepEqual(place(layout, UPPER), [0, 0, 1]);
    assert.deepEqual(place(layout, LOWER), [1440, 0, 1]);
});

test('a laptop with one panel has that one turned', () => {
    const state = readState([11, [monitor('eDP-3')], [logical('eDP-3', 0, 0, true)], properties]);

    assert.deepEqual(place(rotate(state, 'right-up'), 'eDP-3'), [0, 0, 3]);
});

test('a machine without a built-in panel has nothing turned', () => {
    const state = readState([12, [monitor('DP-7')], [logical('DP-7', 0, 0, true, 1)], properties]);

    assert.equal(rotate(state, 'left-up'), null);
});

// Seen on the hardware: the compositor took a saved layout with the panels
// turned while the laptop stood upright. The big monitor is beside the panels
// by a few rows only, and right of the other monitor.
const size = (width, height) => [
    `${width}x${height}@60.000`,
    width,
    height,
    60,
    1,
    [1],
    {'is-current': true},
];
const turnedAtTheDock = () => {
    const state = readState([
        13,
        [
            monitor(UPPER),
            monitor(LOWER),
            [['DP-8', 'DEL', 'U2713HM', '1'], [size(2560, 1440)], {}],
            [['DP-7', 'SAM', 'G75F', '2'], [size(5120, 2160)], {}],
        ],
        [
            logical('DP-8', 1040, 0, false, 1),
            logical(LOWER, 0, 1440, false, 1),
            logical('DP-7', 3600, 0, true, 1),
            logical(UPPER, 1800, 1440, false, 1),
        ],
        properties,
    ]);

    find(state.logical, UPPER).transform = 1;
    find(state.logical, LOWER).transform = 1;
    return state;
};

test('the layout seen on the hardware is one the compositor can take', () => {
    assert.equal(isValid(turnedAtTheDock(), turnedAtTheDock().logical), true);
});

test('upright from a saved turned layout: no monitor lies over another', () => {
    const state = turnedAtTheDock();
    const layout = rotate(state, 'normal');

    assert.equal(isValid(state, layout), true);
    assert.equal(find(layout, UPPER).transform, 0);
    assert.equal(find(layout, LOWER).transform, 0);
    // The upper panel was on the right when turned left side up: it is on top.
    assert.ok(find(layout, UPPER).y < find(layout, LOWER).y);
    // The monitors stay where they are: moving the big one would put it over
    // the other.
    assert.deepEqual(place(layout, 'DP-7'), [3600, 0, 0]);
    assert.deepEqual(place(layout, 'DP-8'), [1040, 0, 0]);
});

test('a layout with a monitor over another is not one the compositor can take', () => {
    const state = twoPanels();

    find(state.logical, LOWER).y = 100;
    assert.equal(isValid(state, state.logical), false);
});

test('a layout with a monitor apart from the others is not one either', () => {
    const state = twoPanels();

    find(state.logical, LOWER).y = 2000;
    assert.equal(isValid(state, state.logical), false);
});

test('the layout from before the turn is not used at another scale', () => {
    const before = twoPanels();
    const turned = twoPanels();

    turned.logical = rotate(turned, 'left-up');
    for (const monitor of turned.logical) monitor.scale = 1;

    const layout = rotate(turned, 'normal', before.logical);

    assert.equal(isValid(turned, layout), true);
    assert.deepEqual(place(layout, LOWER), [0, 1800, 0]);
});

test('when nothing else fits, the monitors are put in one row', () => {
    // The panels are enclosed: a monitor right of them and one below them,
    // and those two touch only through the panels.
    const state = readState([
        14,
        [
            monitor(UPPER),
            [['DP-1', 'A', 'a', '1'], [size(1000, 1440)], {}],
            [['DP-2', 'B', 'b', '2'], [size(2304, 500)], {}],
            [['DP-3', 'C', 'c', '3'], [size(1000, 500)], {}],
        ],
        [
            logical(UPPER, 0, 0, true),
            logical('DP-1', 2304, 0, false, 1),
            logical('DP-2', 0, 1440, false, 1),
            logical('DP-3', 2304, 1440, false, 1),
        ],
        properties,
    ]);
    const layout = rotate(state, 'left-up');

    assert.equal(isValid(state, layout), true);
    assert.equal(find(layout, UPPER).transform, 1);
});

test('panels that are upright with the laptop upright are left where they are', () => {
    // Side by side, as a user can put them: not the layout that a turn gives.
    const state = readState([
        15,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(LOWER, 0, 0, false),
            logical(UPPER, 2304, 0, true),
            logical('DP-7', 4608, 0, false, 1),
        ],
        properties,
    ]);

    assert.equal(rotate(state, 'normal'), null);
    assert.equal(rotate(state, 'normal', twoPanels().logical), null);
});

test('panels that are turned as the laptop is are left where they are', () => {
    const state = twoPanels();

    state.logical = rotate(state, 'left-up');
    // The user swapped the two turned panels.
    [find(state.logical, UPPER).x, find(state.logical, LOWER).x] = [0, 1440];
    assert.equal(rotate(state, 'left-up'), null);
});
