// Tests for layout.js, with the layouts seen on the UX8406CA. Run with
// `node --test`.
import assert from 'node:assert/strict';
import {test} from 'node:test';

import {find, readState, toConfig, toggle, withBelow, without} from '../extension/layout.js';

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
const monitor = (connector, current = true) => [
    [connector, 'SDC', '0x419d', '0x0'],
    [mode(current)],
    {'color-mode': 0},
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
// The keyboard lies on the lower panel: it is not connected.
const docked = () => readState([9, [monitor(UPPER)], [logical(UPPER, 0, 0, true)], properties]);

test('the state is read with modes and connectors', () => {
    const state = twoPanels();

    assert.equal(state.serial, 7);
    assert.equal(state.logicalMode, true);
    assert.equal(state.canSetMode, true);
    assert.deepEqual(state.connected.get(LOWER), {
        id: '2880x1800@120.000',
        width: 2880,
        height: 1800,
        colorMode: 0,
        rgbRange: undefined,
    });
    assert.deepEqual(find(state.logical, LOWER).connectors, [LOWER]);
});

test('a monitor that is off has the mode it prefers', () => {
    assert.equal(lowerOff().connected.get(LOWER).id, '2880x1800@120.000');
});

test('taking the lower panel out leaves the upper one as the primary', () => {
    const rest = without(twoPanels().logical, LOWER, UPPER);

    assert.equal(rest.length, 1);
    assert.deepEqual(rest[0].connectors, [UPPER]);
    assert.equal(rest[0].primary, true);
    assert.equal(rest[0].y, 0);
});

test('what is left moves to the origin', () => {
    const state = readState([
        1,
        [monitor(UPPER), monitor(LOWER)],
        [logical(LOWER, 0, 0, true), logical(UPPER, 0, 1440, false)],
        properties,
    ]);

    assert.equal(without(state.logical, LOWER, UPPER)[0].y, 0);
});

test('the only monitor, a monitor not shown and a mirrored one stay', () => {
    const mirrored = [
        {x: 0, y: 0, scale: 1, transform: 0, primary: true, connectors: [UPPER, LOWER]},
        {x: 2880, y: 0, scale: 1, transform: 0, primary: false, connectors: ['DP-7']},
    ];

    assert.equal(without(lowerOff().logical, UPPER, UPPER), null);
    assert.equal(without(lowerOff().logical, LOWER, UPPER), null);
    assert.equal(without(mirrored, LOWER, UPPER), null);
});

test('the lower panel goes below the upper one, at its scale', () => {
    const layout = withBelow(lowerOff(), LOWER, UPPER);

    assert.equal(layout.length, 2);
    assert.deepEqual(
        {x: layout[1].x, y: layout[1].y, scale: layout[1].scale, primary: layout[1].primary},
        {x: 0, y: 1440, scale: 1.25, primary: false},
    );
});

test('in panel pixels the lower panel is 1800 below', () => {
    const state = lowerOff();

    state.logicalMode = false;
    assert.equal(withBelow(state, LOWER, UPPER)[1].y, 1800);
});

test('pressing the key with both panels on hides the lower one and remembers', () => {
    const state = twoPanels();
    const {apply, save} = toggle(state, null, LOWER, UPPER);

    assert.equal(apply.length, 1);
    assert.equal(save, state.logical);
});

test('pressing it again puts the remembered layout back', () => {
    const before = twoPanels().logical;
    const {apply, save} = toggle(lowerOff(), before, LOWER, UPPER);

    assert.equal(apply, before);
    assert.equal(save, null);
});

test('without a remembered layout the panel goes below the upper one', () => {
    const {apply} = toggle(lowerOff(), null, LOWER, UPPER);

    assert.equal(apply.length, 2);
    assert.equal(apply[1].y, 1440);
});

test('a remembered layout for other monitors is not put back', () => {
    const other = [
        {x: 0, y: 0, scale: 1, transform: 0, primary: true, connectors: ['DP-7']},
        {x: 2560, y: 0, scale: 1, transform: 0, primary: false, connectors: [LOWER]},
    ];
    const {apply} = toggle(lowerOff(), other, LOWER, UPPER);

    assert.equal(apply[1].y, 1440);
});

test('with the keyboard on the panel the key does nothing', () => {
    const saved = twoPanels().logical;
    const {apply, save} = toggle(docked(), saved, LOWER, UPPER);

    assert.equal(apply, null);
    assert.equal(save, saved);
});

test('a layout becomes the arguments of ApplyMonitorsConfig', () => {
    const state = twoPanels();

    assert.deepEqual(toConfig(state.logical, state.connected), [
        [0, 0, 1.25, 0, false, [[UPPER, '2880x1800@120.000', {colorMode: 0, rgbRange: undefined}]]],
        [
            0,
            1440,
            1.25,
            0,
            true,
            [[LOWER, '2880x1800@120.000', {colorMode: 0, rgbRange: undefined}]],
        ],
    ]);
    assert.equal(toConfig(state.logical, new Map()), null);
});
