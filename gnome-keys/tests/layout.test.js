// Tests for layout.js, with the layouts seen on the UX8406CA. Run with
// `node --test`.
import assert from 'node:assert/strict';
import {test} from 'node:test';

import {
    LOWER,
    logical,
    lowerOff,
    monitor,
    properties,
    twoPanels,
    UPPER,
} from '../../gnome-common/tests/fixtures.js';
import {toggle, withBelow, without} from '../extension/layout.js';
import {find, isValid, readState, toConfig} from '../extension/state.js';

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
        underscanning: undefined,
    });
    assert.deepEqual(find(state.logical, LOWER).connectors, [LOWER]);
    assert.deepEqual(state.leased, []);
});

test('underscanning and the monitors for lease are read, to be kept', () => {
    const tv = [
        ['HDMI-1', 'TV', 'p', 's'],
        [['1920x1080@60.000', 1920, 1080, 60, 1, [1], {'is-current': true}]],
        {'is-underscanning': true},
    ];
    const headset = [['DP-3', 'HMD', 'q', 't'], [], {'is-for-lease': true}];
    const state = readState([
        2,
        [monitor(UPPER), tv, headset],
        [logical(UPPER, 0, 0, true), logical('HDMI-1', 2304, 0, false, 1)],
        properties,
    ]);

    assert.deepEqual(state.leased, [['DP-3', 'HMD', 'q', 't']]);
    assert.deepEqual(toConfig(state.logical, state.connected)[1][5], [
        [
            'HDMI-1',
            '1920x1080@60.000',
            {colorMode: undefined, rgbRange: undefined, underscanning: true},
        ],
    ]);
});

test('a monitor that is off has the mode it prefers', () => {
    assert.equal(lowerOff().connected.get(LOWER).id, '2880x1800@120.000');
});

test('taking the lower panel out leaves the upper one as the primary', () => {
    const rest = without(twoPanels(), LOWER, UPPER);

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

    assert.equal(without(state, LOWER, UPPER)[0].y, 0);
});

test('the only monitor, a monitor not shown and a mirrored one stay', () => {
    const mirrored = [
        {x: 0, y: 0, scale: 1, transform: 0, primary: true, connectors: [UPPER, LOWER]},
        {x: 2880, y: 0, scale: 1, transform: 0, primary: false, connectors: ['DP-7']},
    ];

    assert.equal(without(lowerOff(), UPPER, UPPER), null);
    assert.equal(without(lowerOff(), LOWER, UPPER), null);
    assert.equal(without({...twoPanels(), logical: mirrored}, LOWER, UPPER), null);
});

// With the laptop on its side the panels lie side by side, and a monitor can
// be beside the lower one only.
const besideLower = (transform, lowerX, upperX, monitorX) => {
    const state = readState([
        3,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, upperX, 0, true),
            logical(LOWER, lowerX, 0, false),
            logical('DP-7', monitorX, 0, false, 1),
        ],
        properties,
    ]);

    find(state.logical, UPPER).transform = transform;
    find(state.logical, LOWER).transform = transform;
    return state;
};

test('a monitor beyond the lower panel comes nearer when the panel goes', () => {
    // Right side up: upper, lower, monitor. Left side up: monitor, lower, upper.
    const right = besideLower(3, 1440, 0, 2880);
    const left = besideLower(1, 2880, 4320, 0);

    assert.equal(isValid(right, right.logical), true);
    assert.equal(isValid(left, left.logical), true);
    for (const state of [right, left]) {
        const rest = without(state, LOWER, UPPER);

        assert.equal(rest.length, 2);
        assert.equal(isValid(state, rest), true);
    }
    assert.equal(find(without(right, LOWER, UPPER), 'DP-7').x, 1440);
    assert.equal(find(without(left, LOWER, UPPER), UPPER).x, 2880);
});

test('a monitor below the lower panel comes up when the panel goes', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, true),
            logical(LOWER, 0, 1440, false),
            logical('DP-7', 0, 2880, false, 1),
        ],
        properties,
    ]);

    assert.equal(find(without(state, LOWER, UPPER), 'DP-7').y, 1440);
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

test('with the upper panel turned, the lower one goes where the bottom of the laptop is', () => {
    // A turned panel takes 1440x2304 in the layout.
    const place = (transform) => {
        const state = lowerOff();

        state.logical[0].transform = transform;

        const layout = withBelow(state, LOWER, UPPER);
        const [upper, lower] = [find(layout, UPPER), find(layout, LOWER)];

        assert.equal(isValid(state, layout), true);
        assert.equal(lower.transform, transform);
        return [lower.x - upper.x, lower.y - upper.y];
    };

    assert.deepEqual(place(1), [-1440, 0]);
    assert.deepEqual(place(2), [0, -1440]);
    assert.deepEqual(place(3), [1440, 0]);
});

test('a monitor where the lower panel goes makes room', () => {
    // The laptop on its right side: the upper panel, then a monitor. On its
    // left side: a monitor, then the upper panel.
    const room = (transform, upperX, monitorX) => {
        const state = readState([
            5,
            [monitor(UPPER), monitor(LOWER, false), monitor('DP-7')],
            [logical(UPPER, upperX, 0, true), logical('DP-7', monitorX, 0, false, 1)],
            properties,
        ]);

        state.logical[0].transform = transform;

        const layout = withBelow(state, LOWER, UPPER);

        assert.equal(isValid(state, layout), true);
        return [UPPER, LOWER, 'DP-7'].map((connector) => find(layout, connector).x);
    };

    assert.deepEqual(room(3, 0, 1440), [0, 1440, 2880]);
    assert.deepEqual(room(1, 2880, 0), [4320, 2880, 0]);
});

test('a monitor that is not in the way of the lower panel stays', () => {
    // Right of the upper panel, which is turned: one monitor beside it, and
    // one below that monitor.
    const sized = (connector, width, height) => [
        [connector, 'X', 'y', 'z'],
        [[`${width}x${height}@60.000`, width, height, 60, 1, [1], {'is-current': true}]],
        {},
    ];
    const state = readState([
        6,
        [
            monitor(UPPER),
            monitor(LOWER, false),
            sized('DP-7', 1440, 2304),
            sized('DP-8', 1000, 500),
        ],
        [
            logical(UPPER, 0, 0, true),
            logical('DP-7', 1440, 0, false, 1),
            logical('DP-8', 1440, 2304, false, 1),
        ],
        properties,
    ]);

    state.logical[0].transform = 3;

    const layout = withBelow(state, LOWER, UPPER);

    assert.equal(isValid(state, layout), true);
    assert.equal(find(layout, 'DP-7').x, 2880);
    assert.equal(find(layout, 'DP-8').x, 1440);
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
    const kept = {colorMode: 0, rgbRange: undefined, underscanning: undefined};

    assert.deepEqual(toConfig(state.logical, state.connected), [
        [0, 0, 1.25, 0, false, [[UPPER, '2880x1800@120.000', kept]]],
        [0, 1440, 1.25, 0, true, [[LOWER, '2880x1800@120.000', kept]]],
    ]);
    assert.equal(toConfig(state.logical, new Map()), null);
});
