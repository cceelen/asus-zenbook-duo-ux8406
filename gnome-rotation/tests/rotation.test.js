// Tests for rotation.js, with the layouts seen on the UX8406CA. Run with
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
import {
    buttonsFrom,
    control,
    orientationOf,
    rotate,
    screens,
    turned,
    turnMany,
    turnOne,
    uprightAfter,
} from '../extension/rotation.js';
import {find, isValid, readState} from '../extension/state.js';

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

test('upside down: both panels turned by 180 degrees, the lower one on top', () => {
    const layout = rotate(twoPanels(), 'bottom-up');

    assert.deepEqual(place(layout, LOWER), [0, 0, 2]);
    assert.deepEqual(place(layout, UPPER), [0, 1440, 2]);
});

test('upright again after upside down: the layout from before', () => {
    const upright = twoPanels();
    const turned = {...upright, logical: rotate(upright, 'bottom-up')};
    const layout = rotate(turned, 'normal', uprightAfter(upright, 'bottom-up', null));

    assert.deepEqual(place(layout, UPPER), [0, 0, 0]);
    assert.deepEqual(place(layout, LOWER), [0, 1440, 0]);
});

test('from a side to upside down', () => {
    const turned = twoPanels();

    turned.logical = rotate(turned, 'left-up');

    const layout = rotate(turned, 'bottom-up');

    assert.deepEqual(place(layout, LOWER), [0, 0, 2]);
    assert.deepEqual(place(layout, UPPER), [0, 1440, 2]);
});

test('upside down with the lower panel off: only the upper one is turned', () => {
    const layout = rotate(lowerOff(), 'bottom-up');

    assert.equal(layout.length, 1);
    assert.deepEqual(place(layout, UPPER), [0, 0, 2]);
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

test('an unknown orientation changes nothing', () => {
    assert.equal(rotate(twoPanels(), 'undefined'), null);
    // Not an orientation, but the name of something that each object has.
    assert.equal(rotate(twoPanels(), 'constructor'), null);
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

test('a mirrored panel is left as it is', () => {
    for (const transform of [4, 5, 6, 7]) {
        const state = twoPanels();

        for (const panel of state.logical) panel.transform = transform;
        for (const orientation of ['normal', 'left-up', 'right-up'])
            assert.equal(rotate(state, orientation), null);
    }
});

test('a monitor without a mode: nothing is turned', () => {
    const state = readState([
        16,
        [monitor(UPPER), [['DP-7', 'X', 'y', 'z'], [], {}]],
        [logical(UPPER, 0, 0, true), logical('DP-7', 2304, 0, false, 1)],
        properties,
    ]);

    assert.equal(rotate(state, 'left-up'), null);
});

test('the compositor turned the first panel back by itself: the other one tells the order', () => {
    // Right side up with eDP-2 on the left, as the top panel. Then eDP-1 only
    // is upright again, where it was.
    const state = twoPanels();

    state.logical = [
        {...find(state.logical, UPPER), x: 1440, y: 0, transform: 0},
        {...find(state.logical, LOWER), x: 0, y: 0, transform: 3},
    ];

    const layout = rotate(state, 'right-up');

    assert.deepEqual(place(layout, LOWER), [0, 0, 3]);
    assert.deepEqual(place(layout, UPPER), [1440, 0, 3]);
});

test('a monitor right of the panels stays right of them', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 0, 1440, true),
            logical('DP-7', 2304, 200, false, 1),
        ],
        properties,
    ]);
    const layout = rotate(state, 'left-up');

    // The panels are 2880 wide side by side, 576 more than before. The
    // monitor keeps its height in the layout: it is not put in one row.
    assert.deepEqual(place(layout, 'DP-7'), [2880, 200, 0]);
    assert.deepEqual(place(layout, UPPER), [1440, 0, 1]);
});

test('a monitor below the panels stays below them', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 0, 1440, true),
            logical('DP-7', 300, 2880, false, 1),
        ],
        properties,
    ]);
    const layout = rotate(state, 'left-up');

    // The turned panels are 2304 high, 576 less than the two before.
    assert.deepEqual(place(layout, 'DP-7'), [300, 2304, 0]);
    assert.deepEqual(place(layout, LOWER), [0, 0, 1]);
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
    // The lower panel is not exactly below the upper one: a new placement
    // does not give this layout.
    const before = twoPanels();

    find(before.logical, LOWER).x = 300;

    const turned = {...before, logical: rotate(before, 'left-up')};
    const layout = rotate(turned, 'normal', before.logical);

    assert.deepEqual(layout, before.logical);
    assert.notEqual(layout[0], before.logical[0]);
    assert.deepEqual(place(rotate(turned, 'normal'), LOWER), [0, 1440, 0]);
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
    // A monitor below the panel, and one that touches that monitor only. The
    // turned panel is higher: the monitor below must go down, and the other
    // one is then apart; left where it is, the monitor below is under the
    // panel.
    const state = readState([
        14,
        [
            monitor(UPPER),
            [['DP-1', 'A', 'a', '1'], [size(2304, 600)], {}],
            [['DP-2', 'B', 'b', '2'], [size(1000, 500)], {}],
        ],
        [
            logical('DP-2', 2304, 1440, false, 1),
            logical(UPPER, 0, 0, true),
            logical('DP-1', 0, 1440, false, 1),
        ],
        properties,
    ]);
    const layout = rotate(state, 'left-up');

    assert.equal(isValid(state, layout), true);
    // From left to right as they were, each at the top.
    assert.deepEqual(place(layout, UPPER), [0, 0, 1]);
    assert.deepEqual(place(layout, 'DP-1'), [1440, 0, 0]);
    assert.deepEqual(place(layout, 'DP-2'), [3744, 0, 0]);
    // The compositor's order is kept.
    assert.deepEqual(
        layout.map((monitor) => monitor.connectors[0]),
        ['DP-2', UPPER, 'DP-1'],
    );
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

test('the layout to remember: the one from before the first turn', () => {
    const upright = twoPanels();
    const turned = {...upright, logical: rotate(upright, 'left-up')};

    // Turned away from upright: this layout is the one to put back.
    assert.equal(uprightAfter(upright, 'left-up', null), upright.logical);
    // From one side to the other: still the one from before the first turn.
    assert.equal(uprightAfter(turned, 'right-up', upright.logical), upright.logical);
    // Upright again: nothing to put back.
    assert.equal(uprightAfter(turned, 'normal', upright.logical), null);
});

test('the switch where a sensor gives the orientation, else the buttons', () => {
    const can = {managed: false, sensor: true, builtin: true};

    assert.equal(control(can), 'switch');
    // No sensor gives the orientation: the panels are turned by hand.
    assert.equal(control({...can, sensor: false}), 'buttons');
    // No built-in panel: nothing to turn.
    assert.equal(control({...can, builtin: false}), 'none');
    assert.equal(control({...can, sensor: false, builtin: false}), 'none');
    // GNOME turns the panel itself and shows its own switch.
    assert.equal(control({...can, managed: true}), 'none');
    assert.equal(control({...can, managed: true, sensor: false}), 'none');
});

test('the buttons turn by 90 degrees, from left side up to right side up', () => {
    assert.equal(turned('normal', 'clockwise'), 'right-up');
    assert.equal(turned('normal', 'counterclockwise'), 'left-up');
    assert.equal(turned('left-up', 'clockwise'), 'normal');
    assert.equal(turned('right-up', 'counterclockwise'), 'normal');
    // Upside down is not a position.
    assert.equal(turned('right-up', 'clockwise'), null);
    assert.equal(turned('left-up', 'counterclockwise'), null);
    assert.equal(turned('bottom-up', 'clockwise'), null);
});

test('after a turn upside down, both buttons turn the panels again', () => {
    // The menu turned the panels upside down; the buttons start from upright.
    assert.equal(buttonsFrom('bottom-up'), 'normal');
    assert.equal(turned(buttonsFrom('bottom-up'), 'clockwise'), 'right-up');
    assert.equal(turned(buttonsFrom('bottom-up'), 'counterclockwise'), 'left-up');
    // Panels turned differently, or not known: from upright as well.
    assert.equal(buttonsFrom(null), 'normal');
    // A position of the buttons stays as it is.
    for (const orientation of ['left-up', 'normal', 'right-up'])
        assert.equal(buttonsFrom(orientation), orientation);
});

test('the orientation comes from the turn of the built-in panels', () => {
    const upright = twoPanels();

    assert.equal(orientationOf(upright), 'normal');
    assert.equal(orientationOf(lowerOff()), 'normal');
    for (const orientation of ['left-up', 'right-up', 'bottom-up'])
        assert.equal(
            orientationOf({...upright, logical: rotate(upright, orientation)}),
            orientation,
        );
    // The panels turned differently: no orientation.
    const [upper, lower] = upright.logical;

    assert.equal(orientationOf({...upright, logical: [{...upper, transform: 1}, lower]}), null);
    assert.equal(
        orientationOf({
            ...upright,
            logical: [
                {...upper, transform: 2},
                {...lower, transform: 0},
            ],
        }),
        null,
    );
});

// One panel turned by itself.
const withTurn = (state, connector, orientation) => ({
    ...state,
    logical: turnOne(state, connector, orientation),
});

test('one panel by 180 degrees: the panels face each other, nothing else moves', () => {
    const layout = turnOne(twoPanels(), LOWER, 'bottom-up');

    assert.deepEqual(place(layout, UPPER), [0, 0, 0]);
    assert.deepEqual(place(layout, LOWER), [0, 1440, 2]);
});

test('one panel by 90 degrees: the other one is placed below it, from the same corner', () => {
    const layout = turnOne(twoPanels(), UPPER, 'left-up');

    assert.deepEqual(place(layout, UPPER), [0, 0, 1]);
    assert.deepEqual(place(layout, LOWER), [0, 2304, 0]);
});

test('each panel in its own orientation, one after the other', () => {
    const one = withTurn(twoPanels(), LOWER, 'bottom-up');
    const layout = turnOne(one, UPPER, 'right-up');

    assert.deepEqual(place(layout, UPPER), [0, 0, 3]);
    assert.deepEqual(place(layout, LOWER), [0, 2304, 2]);
    assert.deepEqual(
        orientationOf({...one, logical: layout}),
        null,
        'two orientations are no orientation of the laptop',
    );
});

test('side by side stays side by side, in the same order', () => {
    const side = {...twoPanels(), logical: rotate(twoPanels(), 'right-up')};
    const layout = turnOne(side, UPPER, 'normal');

    assert.deepEqual(place(layout, UPPER), [0, 0, 0]);
    assert.deepEqual(place(layout, LOWER), [2304, 0, 3]);
});

test('the order of the panels is the order in the layout, not the names', () => {
    const state = readState([
        10,
        [monitor(UPPER), monitor(LOWER)],
        [logical(LOWER, 0, 0, false), logical(UPPER, 0, 1440, true)],
        properties,
    ]);
    const layout = turnOne(state, LOWER, 'left-up');

    assert.deepEqual(place(layout, LOWER), [0, 0, 1]);
    assert.deepEqual(place(layout, UPPER), [0, 2304, 0]);
});

test('a monitor below the panels moves by as much as they grow', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 0, 1440, true),
            logical('DP-7', 300, 2880, false, 1),
        ],
        properties,
    ]);

    assert.deepEqual(place(turnOne(state, UPPER, 'left-up'), 'DP-7'), [300, 3744, 0]);
});

test('a monitor below panels that shrink slides to the nearest of two panels', () => {
    const state = readState([
        41,
        [
            monitor(UPPER),
            monitor(LOWER),
            [['DP-7', 'X', 'y', 'z'], [['m', 100, 50, 60, 1, [1], {'is-current': true}]], {}],
        ],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 2304, 0, true),
            logical('DP-7', 4000, 1440, false, 1),
        ],
        properties,
    ]);
    const layout = turnMany(state, {[UPPER]: 'left-up', [LOWER]: 'left-up'});

    // The panels are 1440 wide each and 2304 high: the monitor ends at the
    // right edge of the right one.
    assert.ok(isValid(state, layout));
    assert.deepEqual(place(layout, 'DP-7'), [2780, 2304, 0]);
});

test('one panel alone is turned', () => {
    const layout = turnOne(lowerOff(), UPPER, 'bottom-up');

    assert.equal(layout.length, 1);
    assert.deepEqual(place(layout, UPPER), [0, 0, 2]);
});

test('turning one panel changes nothing where it cannot be done', () => {
    const state = twoPanels();
    const mirrored = twoPanels();

    mirrored.logical[1].transform = 4;

    const noMode = readState([
        16,
        [monitor(UPPER), [['DP-7', 'X', 'y', 'z'], [], {}]],
        [logical(UPPER, 0, 0, true), logical('DP-7', 2304, 0, false, 1)],
        properties,
    ]);

    assert.equal(turnOne(state, UPPER, 'undefined'), null);
    assert.equal(turnOne(state, UPPER, 'constructor'), null);
    assert.equal(turnOne(state, 'DP-9', 'left-up'), null);
    assert.equal(turnOne(lowerOff(), LOWER, 'left-up'), null);
    assert.equal(turnOne(state, UPPER, 'normal'), null);
    assert.equal(turnOne(mirrored, UPPER, 'left-up'), null);
    assert.equal(turnOne(mirrored, LOWER, 'normal'), null);
    assert.equal(turnOne(noMode, UPPER, 'left-up'), null);
});

test('a monitor that is not a built-in panel is not turned', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor('DP-7')],
        [logical(UPPER, 0, 0, true), logical('DP-7', 2304, 0, false, 1)],
        properties,
    ]);

    assert.equal(turnOne(state, 'DP-7', 'left-up'), null);
});

test('auto-rotate again after panels were turned by hand: the layout from before', () => {
    const upright = twoPanels();
    const byHand = withTurn(upright, LOWER, 'bottom-up');
    const layout = rotate(byHand, 'normal', upright.logical);

    assert.deepEqual(place(layout, UPPER), [0, 0, 0]);
    assert.deepEqual(place(layout, LOWER), [0, 1440, 0]);
});

test('auto-rotate again: the panels keep the order of the list of built-in panels', () => {
    const upright = twoPanels();
    const byHand = withTurn(upright, LOWER, 'bottom-up');
    const top = [UPPER, LOWER];
    const side = rotate(byHand, 'left-up', null, top);

    // Left side up: the lower panel is on the left.
    assert.deepEqual(place(side, LOWER), [0, 0, 1]);
    assert.deepEqual(place(side, UPPER), [1440, 0, 1]);

    const flat = rotate(byHand, 'normal', null, top);

    assert.deepEqual(place(flat, UPPER), [0, 0, 0]);
    assert.deepEqual(place(flat, LOWER), [0, 1440, 0]);
    // The other way round in the list: the other panel is on top.
    assert.deepEqual(place(rotate(byHand, 'normal', null, [LOWER, UPPER]), LOWER), [0, 0, 0]);
});

test('a list of panels that misses one is not used; panels turned alike ignore it', () => {
    const byHand = withTurn(twoPanels(), LOWER, 'bottom-up');

    assert.deepEqual(rotate(byHand, 'normal', null, [UPPER]), rotate(byHand, 'normal'));
    assert.deepEqual(
        rotate(twoPanels(), 'left-up', null, [LOWER, UPPER]),
        rotate(twoPanels(), 'left-up'),
    );
});

test('panels put back to their orientations, also after the compositor made them upright', () => {
    const layout = turnMany(twoPanels(), {[UPPER]: 'normal', [LOWER]: 'bottom-up'});

    assert.deepEqual(place(layout, LOWER), [0, 1440, 2]);

    const both = turnMany(twoPanels(), {[UPPER]: 'left-up', [LOWER]: 'right-up'});

    assert.deepEqual(place(both, UPPER), [0, 0, 1]);
    assert.deepEqual(place(both, LOWER), [0, 2304, 3]);
});

test('panels put back: a panel that is gone is left out; nothing to do gives null', () => {
    const layout = turnMany(lowerOff(), {[UPPER]: 'bottom-up', [LOWER]: 'left-up'});

    assert.equal(layout.length, 1);
    assert.deepEqual(place(layout, UPPER), [0, 0, 2]);
    assert.equal(turnMany(twoPanels(), {}), null);
    assert.equal(turnMany(twoPanels(), {[UPPER]: 'normal'}), null);
    assert.equal(turnMany(twoPanels(), {[UPPER]: 'undefined'}), null);
});

test('a monitor right of the panels slides to the panel that reaches the new edge', () => {
    const state = readState([
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 0, 1440, true),
            logical('DP-7', 2304, 200, false, 1),
        ],
        properties,
    ]);

    // The widest edge is the lower panel's, 2304 down: the monitor is level
    // with its top.
    assert.deepEqual(place(turnOne(state, UPPER, 'left-up'), 'DP-7'), [2304, 2304, 0]);
});

test('a monitor below side by side panels slides to the panel that reaches the new edge', () => {
    const side = readState([
        40,
        [
            monitor(UPPER),
            monitor(LOWER),
            [['DP-7', 'X', 'y', 'z'], [['m', 1000, 500, 60, 1, [1], {'is-current': true}]], {}],
        ],
        [
            logical(UPPER, 0, 0, false),
            logical(LOWER, 1440, 0, true),
            logical('DP-7', 100, 2304, false, 1),
        ],
        properties,
    ]);

    side.logical[0].transform = 3;
    side.logical[1].transform = 3;

    const layout = turnOne(side, UPPER, 'normal');

    assert.ok(isValid(side, layout));
    assert.deepEqual(place(layout, 'DP-7'), [2304, 2304, 0]);
});

test('the screens: two panels are the upper and the lower screen', () => {
    const state = twoPanels();

    assert.deepEqual(screens(state), [
        {connector: UPPER, name: 'Upper screen', orientation: 'normal'},
        {connector: LOWER, name: 'Lower screen', orientation: 'normal'},
    ]);
    assert.deepEqual(
        screens(withTurn(state, LOWER, 'bottom-up')).map((screen) => screen.orientation),
        ['normal', 'bottom-up'],
    );
});

test('the screens: one panel is the built-in screen; a panel that is off is left out', () => {
    assert.deepEqual(screens(lowerOff()), [
        {connector: UPPER, name: 'Upper screen', orientation: 'normal'},
    ]);

    const docked = readState([9, [monitor(UPPER)], [logical(UPPER, 0, 0, true)], properties]);

    assert.deepEqual(screens(docked), [
        {connector: UPPER, name: 'Built-in screen', orientation: 'normal'},
    ]);
});

test('the screens: more than two panels are numbered; a mirrored panel has no orientation', () => {
    const state = readState([
        9,
        [monitor(UPPER), monitor(LOWER), monitor('eDP-3')],
        [
            logical(UPPER, 0, 0, true),
            logical(LOWER, 0, 1440, false),
            logical('eDP-3', 0, 2880, false),
        ],
        properties,
    ]);

    state.logical[2].transform = 5;
    assert.deepEqual(
        screens(state).map(({name, orientation}) => [name, orientation]),
        [
            ['Built-in screen 1', 'normal'],
            ['Built-in screen 2', 'normal'],
            ['Built-in screen 3', null],
        ],
    );
});
