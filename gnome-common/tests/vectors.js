// Test cases that both implementations of the display logic must satisfy: the
// JavaScript of the GNOME Shell extensions, and the Rust of the KDE parts. The
// expected results come from the JavaScript, whose own tests check it
// (write-vectors.js writes the cases to vectors/).
//
// A state is plain data in the files:
//   {logicalMode, connected: {connector: {width, height} | null}, builtin: [...],
//    logical: [{x, y, scale, transform, primary, connectors}]}

import {toggle, withBelow, without} from '../../gnome-keys/extension/layout.js';
import {assignments, controllersOf} from '../../gnome-keys/extension/touch.js';
import {
    buttonsFrom,
    control,
    orientationOf,
    rotate,
    turned,
    turnMany,
    turnOne,
    uprightAfter,
} from '../../gnome-rotation/extension/rotation.js';
import {isRest, isValid, readState, shifted, sizeOf, toOrigin} from '../state.js';
import {LOWER, logical, lowerOff, monitor, properties, twoPanels, UPPER} from './fixtures.js';

/** The state of the compositor in the form of the files. */
export function toNeutral(state) {
    return {
        logicalMode: state.logicalMode,
        connected: Object.fromEntries(
            [...state.connected].map(([connector, mode]) => [
                connector,
                mode && {width: mode.width, height: mode.height},
            ]),
        ),
        builtin: [...state.builtin],
        logical: toLayout(state.logical),
    };
}

/** A layout in the form of the files. */
export function toLayout(layout) {
    if (!layout) return null;
    return layout.map(({x, y, scale, transform, primary, connectors}) => ({
        x,
        y,
        scale,
        transform,
        primary,
        connectors: [...connectors],
    }));
}

/** The state of the files in the form the functions work on. */
export function fromNeutral(neutral) {
    const connected = new Map(
        Object.entries(neutral.connected).map(([connector, mode]) => [
            connector,
            mode && {id: 'mode', width: mode.width, height: mode.height},
        ]),
    );

    return {
        logicalMode: neutral.logicalMode,
        connected,
        identities: new Map([...connected.keys()].map((name) => [name, ['SDC', '0x1', '0x0']])),
        builtin: neutral.builtin,
        logical: toLayout(neutral.logical),
    };
}

const SIZE = (connector, width, height) => [
    [connector, 'X', 'y', 'z'],
    [[`${width}x${height}@60.000`, width, height, 60, 1, [1], {'is-current': true}]],
    {},
];
const NO_MODE = (connector) => [[connector, 'X', 'y', 'z'], [], {}];

const state = (serial, monitors, logicals) => readState([serial, monitors, logicals, properties]);
const at = (layout, connector) => layout.find((item) => item.connectors.includes(connector));
const withTransform = (st, transform, ...connectors) => {
    for (const connector of connectors) at(st.logical, connector).transform = transform;
    return st;
};
// The keyboard lies on the lower panel: it is not connected.
const docked = () => state(9, [monitor(UPPER)], [logical(UPPER, 0, 0, true)]);
const mirrored = () => [
    {x: 0, y: 0, scale: 1, transform: 0, primary: true, connectors: [UPPER, LOWER]},
    {x: 2880, y: 0, scale: 1, transform: 0, primary: false, connectors: ['DP-7']},
];
const sideBySide = (transform, lowerX, upperX, monitorX) =>
    withTransform(
        state(
            3,
            [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
            [
                logical(UPPER, upperX, 0, true),
                logical(LOWER, lowerX, 0, false),
                logical('DP-7', monitorX, 0, false, 1),
            ],
        ),
        transform,
        UPPER,
        LOWER,
    );
const mirroredState = () => ({...twoPanels(), logical: toLayout(mirrored())});

/** The cases of a file, each with the result of the reference. */
function collect(run) {
    const cases = [];
    const add = (name, call, inputs) => {
        const entry = {name, call, ...inputs};

        cases.push({...entry, expect: run(entry)});
    };

    return {cases, add};
}

const prepared = (entry) => ({
    ...entry,
    state: entry.state && fromNeutral(entry.state),
});

/** The result of a layout case, from the files' form of the inputs. */
function runLayout(entry) {
    const {call, state: st, layout: lay, saved} = prepared(entry);

    switch (call) {
        case 'without':
            return toLayout(without(st, entry.connector, entry.heir));
        case 'withBelow':
            return toLayout(withBelow(st, entry.connector, entry.anchor));
        case 'toggle': {
            const {apply, save} = toggle(st, saved, entry.lower, entry.upper);

            return {apply: toLayout(apply), save: toLayout(save)};
        }
        case 'isValid':
            return isValid(st, lay);
        case 'toOrigin':
            return toLayout(toOrigin(lay));
        case 'shifted':
            return toLayout(shifted(st, lay, entry.area, entry.direction, entry.steps));
        case 'isRest':
            return isRest(lay, saved, entry.except);
        case 'sizeOf': {
            const {width, height} = sizeOf(entry.monitor, entry.mode, entry.logicalMode);

            return {width, height};
        }
        default:
            throw new Error(`unknown call ${call}`);
    }
}

function runRotation(entry) {
    const {call, state: st, upright} = prepared(entry);

    switch (call) {
        case 'rotate':
            return toLayout(rotate(st, entry.orientation, upright, entry.top ?? null));
        case 'turnMany':
            return toLayout(turnMany(st, entry.held));
        case 'turnOne':
            return toLayout(turnOne(st, entry.connector, entry.orientation));
        case 'uprightAfter':
            return toLayout(uprightAfter(st, entry.orientation, upright));
        case 'orientationOf':
            return orientationOf(st);
        case 'turned':
            return turned(entry.orientation, entry.direction);
        case 'buttonsFrom':
            return buttonsFrom(entry.orientation);
        case 'control':
            return control({managed: entry.managed, sensor: entry.sensor, builtin: entry.builtin});
        default:
            throw new Error(`unknown call ${call}`);
    }
}

function runTouch(entry) {
    const {call, state: st} = prepared(entry);

    switch (call) {
        case 'controllersOf':
            return controllersOf(entry.devices);
        case 'assignments':
            return assignments(controllersOf(entry.devices), st, entry.upper, entry.lower).map(
                ({id, output}) => ({id, connector: output[3]}),
            );
        default:
            throw new Error(`unknown call ${call}`);
    }
}

const N = toNeutral;
const L = toLayout;

function layoutCases() {
    const {cases, add} = collect(runLayout);
    const two = twoPanels();
    const swapped = state(
        1,
        [monitor(UPPER), monitor(LOWER)],
        [logical(LOWER, 0, 0, true), logical(UPPER, 0, 1440, false)],
    );
    const below = state(
        4,
        [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
        [
            logical(UPPER, 0, 0, true),
            logical(LOWER, 0, 1440, false),
            logical('DP-7', 0, 2880, false, 1),
        ],
    );
    const right = sideBySide(3, 1440, 0, 2880);
    const left = sideBySide(1, 2880, 4320, 0);

    add('taking the lower panel out leaves the upper one as the primary', 'without', {
        state: N(two),
        connector: LOWER,
        heir: UPPER,
    });
    add('the upper panel out: the lower one is the primary', 'without', {
        state: N(two),
        connector: UPPER,
        heir: LOWER,
    });
    add('an unknown heir: the first monitor is the primary', 'without', {
        state: N(two),
        connector: LOWER,
        heir: 'DP-9',
    });
    add('what is left moves to the origin', 'without', {
        state: N(swapped),
        connector: LOWER,
        heir: UPPER,
    });
    add('the primary goes to the heir, not to the first monitor', 'without', {
        state: N(
            state(
                40,
                [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
                [
                    logical(UPPER, 0, 0, false),
                    logical(LOWER, 0, 1440, true),
                    logical('DP-7', 0, 2880, false, 1),
                ],
            ),
        ),
        connector: LOWER,
        heir: 'DP-7',
    });
    add('a monitor that is not the primary leaves the primary as it is', 'without', {
        state: N(below),
        connector: LOWER,
        heir: 'DP-7',
    });
    add('the only monitor stays', 'without', {state: N(lowerOff()), connector: UPPER, heir: UPPER});
    add('a monitor not shown stays', 'without', {
        state: N(lowerOff()),
        connector: LOWER,
        heir: UPPER,
    });
    add('a mirrored monitor stays', 'without', {
        state: N(mirroredState()),
        connector: LOWER,
        heir: UPPER,
    });
    add('a monitor beyond the lower panel (right side up) comes nearer', 'without', {
        state: N(right),
        connector: LOWER,
        heir: UPPER,
    });
    add('a monitor beyond the lower panel (left side up) comes nearer', 'without', {
        state: N(left),
        connector: LOWER,
        heir: UPPER,
    });
    add('a monitor over the lower panel and the one beside it stays where it is', 'without', {
        state: N(
            withTransform(
                state(
                    42,
                    [monitor(UPPER), monitor(LOWER), monitor('DP-7'), SIZE('DP-8', 2880, 1000)],
                    [
                        logical(UPPER, 0, 0, true),
                        logical(LOWER, 1440, 0, false),
                        logical('DP-7', 2880, 0, false, 1),
                        logical('DP-8', 0, -1000, false, 1),
                    ],
                ),
                3,
                UPPER,
                LOWER,
            ),
        ),
        connector: LOWER,
        heir: UPPER,
    });
    add('a monitor below the lower panel comes up', 'without', {
        state: N(below),
        connector: LOWER,
        heir: UPPER,
    });
    add('a monitor that holds the others together takes its place', 'without', {
        state: N(below),
        connector: UPPER,
        heir: LOWER,
    });
    // No way is valid: the monitor far away touches nothing, whatever is shifted.
    add('no shift makes a valid layout: the rest as it is', 'without', {
        state: N(
            state(
                20,
                [monitor(UPPER), monitor(LOWER), SIZE('DP-7', 1000, 500)],
                [
                    logical(UPPER, 0, 0, true),
                    logical(LOWER, 0, 1440, false),
                    logical('DP-7', 10000, 0, false, 1),
                ],
            ),
        ),
        connector: LOWER,
        heir: UPPER,
    });
    add('no shift makes a valid layout: the panel is added as it is', 'withBelow', {
        state: N(
            withTransform(
                state(
                    23,
                    [monitor(UPPER), monitor(LOWER, false), SIZE('DP-7', 100, 100)],
                    [logical(UPPER, 0, 0, true), logical('DP-7', 5000, 5000, false, 1)],
                ),
                3,
                UPPER,
            ),
        ),
        connector: LOWER,
        anchor: UPPER,
    });

    add('the lower panel goes below the upper one, at its scale', 'withBelow', {
        state: N(lowerOff()),
        connector: LOWER,
        anchor: UPPER,
    });
    add('in panel pixels the lower panel is 1800 below', 'withBelow', {
        state: N({...lowerOff(), logicalMode: false}),
        connector: LOWER,
        anchor: UPPER,
    });
    for (const transform of [1, 2, 3])
        add(`turned to ${transform}: the lower panel goes to the bottom`, 'withBelow', {
            state: N(withTransform(lowerOff(), transform, UPPER)),
            connector: LOWER,
            anchor: UPPER,
        });
    // The panel to add has a mode of another size than the anchor.
    for (const transform of [0, 1, 2, 3])
        add(`a panel of another size goes beside the anchor, turned to ${transform}`, 'withBelow', {
            state: N(
                withTransform(
                    state(
                        41,
                        [monitor(UPPER), SIZE(LOWER, 1920, 1080)],
                        [logical(UPPER, 0, 0, true)],
                    ),
                    transform,
                    UPPER,
                ),
            ),
            connector: LOWER,
            anchor: UPPER,
        });
    add('an anchor that is mirrored gives nothing', 'withBelow', {
        state: N({...mirroredState(), connected: lowerOff().connected}),
        connector: LOWER,
        anchor: UPPER,
    });
    add('an anchor that is not shown gives nothing', 'withBelow', {
        state: N(lowerOff()),
        connector: UPPER,
        anchor: LOWER,
    });
    add('a connector that is not connected gives nothing', 'withBelow', {
        state: N(docked()),
        connector: LOWER,
        anchor: UPPER,
    });
    add('an anchor without a mode gives nothing', 'withBelow', {
        state: N(state(21, [NO_MODE(UPPER), monitor(LOWER, false)], [logical(UPPER, 0, 0, true)])),
        connector: LOWER,
        anchor: UPPER,
    });
    for (const [transform, upperX, monitorX] of [
        [3, 0, 1440],
        [1, 2880, 0],
    ])
        add(`a monitor in the way makes room (turned to ${transform})`, 'withBelow', {
            state: N(
                withTransform(
                    state(
                        5,
                        [monitor(UPPER), monitor(LOWER, false), monitor('DP-7')],
                        [logical(UPPER, upperX, 0, true), logical('DP-7', monitorX, 0, false, 1)],
                    ),
                    transform,
                    UPPER,
                ),
            ),
            connector: LOWER,
            anchor: UPPER,
        });
    add('a monitor that is not in the way of the lower panel stays', 'withBelow', {
        state: N(
            withTransform(
                state(
                    6,
                    [
                        monitor(UPPER),
                        monitor(LOWER, false),
                        SIZE('DP-7', 1440, 2304),
                        SIZE('DP-8', 1000, 500),
                    ],
                    [
                        logical(UPPER, 0, 0, true),
                        logical('DP-7', 1440, 0, false, 1),
                        logical('DP-8', 1440, 2304, false, 1),
                    ],
                ),
                3,
                UPPER,
            ),
        ),
        connector: LOWER,
        anchor: UPPER,
    });

    const saved = two.logical;

    add('the key with both panels on hides the lower one and remembers', 'toggle', {
        state: N(two),
        saved: null,
        lower: LOWER,
        upper: UPPER,
    });
    add('the key with both panels on keeps the saved layout if nothing can be hidden', 'toggle', {
        state: N(mirroredState()),
        saved: L(saved),
        lower: LOWER,
        upper: UPPER,
    });
    add('the key again puts the remembered layout back', 'toggle', {
        state: N(lowerOff()),
        saved: L(saved),
        lower: LOWER,
        upper: UPPER,
    });
    add('after a turn of the laptop the remembered layout is not put back', 'toggle', {
        state: N(withTransform(lowerOff(), 1, UPPER)),
        saved: L(saved),
        lower: LOWER,
        upper: UPPER,
    });
    add('without a remembered layout the panel goes below the upper one', 'toggle', {
        state: N(lowerOff()),
        saved: null,
        lower: LOWER,
        upper: UPPER,
    });
    add('a remembered layout for other monitors is not put back', 'toggle', {
        state: N(lowerOff()),
        saved: [
            {x: 0, y: 0, scale: 1, transform: 0, primary: true, connectors: ['DP-7']},
            {x: 2560, y: 0, scale: 1, transform: 0, primary: false, connectors: [LOWER]},
        ],
        lower: LOWER,
        upper: UPPER,
    });
    add('no upper panel in both layouts: the turn is the same', 'toggle', {
        state: N(
            state(22, [monitor('DP-7'), monitor(LOWER, false)], [logical('DP-7', 0, 0, true, 1)]),
        ),
        saved: [{x: 0, y: 0, scale: 1, transform: 0, primary: true, connectors: ['DP-7']}],
        lower: LOWER,
        upper: 'DP-9',
    });
    add('with the keyboard on the panel the key does nothing', 'toggle', {
        state: N(docked()),
        saved: L(saved),
        lower: LOWER,
        upper: UPPER,
    });
    add('with the keyboard on the panel and nothing saved', 'toggle', {
        state: N(docked()),
        saved: null,
        lower: LOWER,
        upper: UPPER,
    });
    add('the lower panel shown, nothing to hide: the saved layout stays', 'toggle', {
        state: N(state(1, [monitor(UPPER), monitor(LOWER)], [logical(LOWER, 0, 0, true)])),
        saved: L(saved),
        lower: LOWER,
        upper: UPPER,
    });

    // Geometry.
    const valid = (name, st, lay = st.logical) =>
        add(name, 'isValid', {state: N(st), layout: L(lay)});
    const moved = (x, y) => {
        const st = twoPanels();

        Object.assign(at(st.logical, LOWER), {x, y});
        return st;
    };

    valid('two panels, one below the other, are valid', two);
    valid('a monitor over another is not valid', moved(0, 100));
    valid('a monitor apart from the others is not valid', moved(0, 2000));
    valid('monitors that touch at a corner only are not valid', moved(2304, 1440));
    valid('monitors side by side are valid', moved(2304, 0));
    valid('monitors side by side, shifted, are valid', moved(2304, 700));
    valid('monitors with a gap on a side are not valid', moved(2305, 0));
    valid('a monitor that touches the others through a third is valid', below);
    valid('one monitor is valid', lowerOff());
    valid('an empty layout is not valid', two, []);
    valid('a layout in panel pixels', {...two, logicalMode: false});
    valid('a layout in panel pixels, side by side', {...two, logicalMode: false}, [
        {...two.logical[0]},
        {...two.logical[1], y: 0, x: 2880},
    ]);

    add('the layout moves to the origin', 'toOrigin', {
        layout: L([
            {...two.logical[0], x: 100, y: 50},
            {...two.logical[1], x: 100, y: 1490},
        ]),
    });
    add('the empty layout stays empty', 'toOrigin', {layout: []});
    add('negative places move to the origin', 'toOrigin', {
        layout: L([
            {...two.logical[0], x: -300, y: -20},
            {...two.logical[1], x: 5, y: 7},
        ]),
    });

    const hole = {x: 0, y: 1440, width: 2304, height: 1440};
    for (const [direction, area] of [
        [[0, 1], hole],
        [[0, -1], hole],
        [[1, 0], {x: 0, y: 0, width: 1000, height: 3000}],
        [[-1, 0], {x: 2304, y: 0, width: 1000, height: 3000}],
        [[1, 0], {x: 0, y: 5000, width: 1000, height: 100}],
    ])
        for (const steps of [1, -1])
            add(`shifted by ${steps} to ${direction} from ${JSON.stringify(area)}`, 'shifted', {
                state: N(below),
                layout: L(below.logical),
                area,
                direction,
                steps,
            });
    add('shifted in panel pixels', 'shifted', {
        state: N({...below, logicalMode: false}),
        layout: L(below.logical),
        area: hole,
        direction: [0, 1],
        steps: 1,
    });

    // A monitor that touches the area across its direction is not in line with it.
    const neighbour = (x, y) => [...below.logical.slice(0, 2), {...below.logical[2], x, y}];

    add('shifted: a monitor that ends where the area begins is not in line', 'shifted', {
        state: N(below),
        layout: L(neighbour(-2880, 2880)),
        area: hole,
        direction: [0, 1],
        steps: 1,
    });
    add('shifted: a monitor that begins where the area ends is not in line', 'shifted', {
        state: N(below),
        layout: L(neighbour(2304, 2880)),
        area: hole,
        direction: [0, 1],
        steps: 1,
    });
    add('shifted: a monitor above that ends where the area begins is not in line', 'shifted', {
        state: N(below),
        layout: L(neighbour(2304, -1800)),
        area: {x: 0, y: 0, width: 1000, height: 3000},
        direction: [1, 0],
        steps: 1,
    });

    const sameLayout = [
        ['the same connectors', two.logical, two.logical, null],
        ['without the connector of the exception', two.logical, lowerOff().logical, LOWER],
        ['the exception is not in the saved layout', two.logical, two.logical, 'DP-9'],
        ['other connectors', two.logical, L([{...two.logical[0], connectors: ['DP-7']}]), null],
        ['a different count', two.logical, lowerOff().logical, null],
        [
            'the same connectors in another order',
            two.logical,
            [two.logical[1], two.logical[0]],
            null,
        ],
        [
            'the connectors of the layout in another order than the saved ones',
            [two.logical[1], two.logical[0]],
            two.logical,
            null,
        ],
        ['nothing and nothing', [], [], null],
        [
            'the same count, other names',
            two.logical,
            L([two.logical[0], {...two.logical[1], connectors: ['DP-7']}]),
            null,
        ],
    ];
    for (const [name, now, before, except] of sameLayout)
        add(`isRest: ${name}`, 'isRest', {layout: L(now), saved: L(before), except});

    for (const [transform, scale, logicalMode, width, height] of [
        [0, 1.25, true, 2880, 1800],
        [1, 1.25, true, 2880, 1800],
        [2, 1.5, true, 2880, 1800],
        [3, 1, false, 2880, 1800],
        [0, 1.75, true, 1000, 1000],
        [0, 1.5, true, 1001, 1001],
        [1, 2, true, 3, 5],
        [0, 2, true, 5, 3],
        [0, 3, true, 1, 1],
        [0, 3, true, 2, 4],
        [4, 1, true, 100, 200],
        [5, 1, true, 100, 200],
    ])
        add(`sizeOf ${transform} ${scale} ${logicalMode} ${width}x${height}`, 'sizeOf', {
            monitor: {x: 0, y: 0, scale, transform, primary: false, connectors: [UPPER]},
            mode: {width, height},
            logicalMode,
        });

    return cases;
}

function rotationCases() {
    const {cases, add} = collect(runRotation);
    const two = twoPanels();
    const turnedTo = (st, orientation, upright = null) => ({
        ...st,
        logical: rotate(st, orientation, upright),
    });
    const named = (name, st, orientation, upright = null, top = null) =>
        add(name, 'rotate', {state: N(st), orientation, upright: L(upright), ...(top && {top})});
    const leftUp = turnedTo(two, 'left-up');

    named('left side up: both panels turned, the lower one on the left', two, 'left-up');
    named('right side up: both panels turned, the lower one on the right', two, 'right-up');
    named('upright again: the lower panel below the upper one', leftUp, 'normal');
    named('from one side to the other', leftUp, 'right-up');
    named('with the lower panel off, only the upper one is turned', lowerOff(), 'left-up');
    named('nothing to do when the layout is upright already', two, 'normal');
    named(
        'nothing to do when the layout is that one already',
        turnedTo(two, 'right-up'),
        'right-up',
    );
    named('upside down: both panels turned by 180 degrees, the lower one on top', two, 'bottom-up');
    named('from a side to upside down', leftUp, 'bottom-up');
    named('upside down with the lower panel off', lowerOff(), 'bottom-up');
    named(
        'upright again after upside down: the layout from before',
        turnedTo(two, 'bottom-up'),
        'normal',
        two.logical,
    );
    named('an undefined orientation changes nothing', two, 'undefined');
    named('the name of something that each object has changes nothing', two, 'constructor');
    named('an empty orientation changes nothing', two, '');
    named(
        'with the built-in panels off, nothing is turned',
        state(3, [monitor(UPPER, false), monitor('DP-7')], [logical('DP-7', 0, 0, true, 1)]),
        'left-up',
    );
    for (const transform of [4, 5, 6, 7])
        for (const orientation of ['normal', 'left-up', 'right-up', 'bottom-up']) {
            const st = twoPanels();

            for (const panel of st.logical) panel.transform = transform;
            named(
                `a mirrored panel (${transform}) is left as it is: ${orientation}`,
                st,
                orientation,
            );
        }
    named(
        'a monitor without a mode: nothing is turned',
        state(
            16,
            [monitor(UPPER), NO_MODE('DP-7')],
            [logical(UPPER, 0, 0, true), logical('DP-7', 2304, 0, false, 1)],
        ),
        'left-up',
    );
    named(
        'the compositor turned the first panel back by itself: the other one tells the order',
        {
            ...two,
            logical: [
                {...at(two.logical, UPPER), x: 1440, y: 0, transform: 0},
                {...at(two.logical, LOWER), x: 0, y: 0, transform: 3},
            ],
        },
        'right-up',
    );

    const beside = (x, y) =>
        state(
            4,
            [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
            [
                logical(UPPER, 0, 0, false),
                logical(LOWER, 0, 1440, true),
                logical('DP-7', x, y, false, 1),
            ],
        );

    named('a monitor right of the panels stays right of them', beside(2304, 200), 'left-up');
    named('a monitor right of the panels, right side up', beside(2304, 200), 'right-up');
    named('a monitor below the panels stays below them', beside(300, 2880), 'left-up');
    named(
        'a monitor left of the panels stays where it is',
        state(
            5,
            [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
            [
                logical('DP-7', 0, 0, false, 1),
                logical(UPPER, 2880, 0, false),
                logical(LOWER, 2880, 1440, true),
            ],
        ),
        'right-up',
    );

    const away = () =>
        state(
            6,
            [monitor(UPPER), monitor('DP-8'), monitor('DP-7')],
            [
                logical(UPPER, 760, 1800, false),
                logical('DP-8', 0, 0, true, 1),
                logical('DP-7', 2880, 0, false, 1),
            ],
        );
    const awayTurned = turnedTo(away(), 'left-up');

    named('a monitor that is not beside the panels stays: left side up', away(), 'left-up');
    named('a monitor that is not beside the panels stays: upright again', awayTurned, 'normal');

    const bent = twoPanels();

    at(bent.logical, LOWER).x = 300;

    const bentTurned = turnedTo(bent, 'left-up');

    named(
        'upright again, the layout from before the turn is put back',
        bentTurned,
        'normal',
        bent.logical,
    );
    named('upright again, no layout from before: placed anew', bentTurned, 'normal');
    named(
        'the layout from before the turn is not used with other monitors',
        turnedTo(lowerOff(), 'left-up'),
        'normal',
        two.logical,
    );
    named(
        'upright with the layout from before in place: nothing to do',
        two,
        'normal',
        twoPanels().logical,
    );

    const swappedRows = state(
        10,
        [monitor(UPPER), monitor(LOWER)],
        [logical(LOWER, 0, 0, false), logical(UPPER, 0, 1440, true)],
    );

    named(
        'the order of the panels comes from the layout, not from their names',
        swappedRows,
        'left-up',
    );
    named('the same, right side up', swappedRows, 'right-up');
    named(
        'a laptop with one panel has that one turned',
        state(11, [monitor('eDP-3')], [logical('eDP-3', 0, 0, true)]),
        'right-up',
    );
    named(
        'a machine without a built-in panel has nothing turned',
        state(12, [monitor('DP-7')], [logical('DP-7', 0, 0, true, 1)]),
        'left-up',
    );

    const dock = () =>
        withTransform(
            state(
                13,
                [
                    monitor(UPPER),
                    monitor(LOWER),
                    [['DP-8', 'DEL', 'U2713HM', '1'], [SIZE('', 2560, 1440)[1][0]], {}],
                    [['DP-7', 'SAM', 'G75F', '2'], [SIZE('', 5120, 2160)[1][0]], {}],
                ],
                [
                    logical('DP-8', 1040, 0, false, 1),
                    logical(LOWER, 0, 1440, false, 1),
                    logical('DP-7', 3600, 0, true, 1),
                    logical(UPPER, 1800, 1440, false, 1),
                ],
            ),
            1,
            UPPER,
            LOWER,
        );

    named('upright from a saved turned layout: no monitor lies over another', dock(), 'normal');
    named('turned from a saved turned layout, to the other side', dock(), 'right-up');

    const rescaled = turnedTo(twoPanels(), 'left-up');

    for (const panel of rescaled.logical) panel.scale = 1;
    named(
        'the layout from before the turn is not used at another scale',
        rescaled,
        'normal',
        two.logical,
    );

    named(
        'when nothing else fits, the monitors are put in one row',
        state(
            14,
            [monitor(UPPER), SIZE('DP-1', 2304, 600), SIZE('DP-2', 1000, 500)],
            [
                logical('DP-2', 2304, 1440, false, 1),
                logical(UPPER, 0, 0, true),
                logical('DP-1', 0, 1440, false, 1),
            ],
        ),
        'left-up',
    );
    named(
        'panels that are upright with the laptop upright are left where they are',
        state(
            15,
            [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
            [
                logical(LOWER, 0, 0, false),
                logical(UPPER, 2304, 0, true),
                logical('DP-7', 4608, 0, false, 1),
            ],
        ),
        'normal',
    );
    named(
        'the same with the layout from before',
        state(
            15,
            [monitor(UPPER), monitor(LOWER), monitor('DP-7')],
            [
                logical(LOWER, 0, 0, false),
                logical(UPPER, 2304, 0, true),
                logical('DP-7', 4608, 0, false, 1),
            ],
        ),
        'normal',
        two.logical,
    );

    const swappedTurned = turnedTo(two, 'left-up');

    [at(swappedTurned.logical, UPPER).x, at(swappedTurned.logical, LOWER).x] = [0, 1440];
    named(
        'panels that are turned as the laptop is are left where they are',
        swappedTurned,
        'left-up',
    );

    const rightTurned = turnedTo(two, 'right-up');
    const mixed = twoPanels();

    mixed.logical[0].transform = 1;
    named('panels turned differently: the first one tells the way', mixed, 'right-up');
    named('panels turned differently: to upright', mixed, 'normal');
    named('panels turned differently: to the way of one', mixed, 'left-up');
    named('right side up, upright again', rightTurned, 'normal');

    // The monitors around the panels, with the panels of another scale than the
    // upper one.
    const around = (serial, transform, [upperAt, lowerAt], lowerScale, others) =>
        withTransform(
            state(
                serial,
                [
                    monitor(UPPER),
                    monitor(LOWER),
                    ...others.map(([connector, width, height]) => SIZE(connector, width, height)),
                ],
                [
                    logical(UPPER, ...upperAt, true),
                    logical(LOWER, ...lowerAt, false, lowerScale),
                    ...others.map(([connector, , , x, y]) => logical(connector, x, y, false, 1)),
                ],
            ),
            transform,
            UPPER,
            LOWER,
        );

    named(
        'the layout from before the turn is not used when one panel is at another scale',
        (() => {
            const st = turnedTo(twoPanels(), 'left-up');

            at(st.logical, UPPER).scale = 1;
            return st;
        })(),
        'normal',
        two.logical,
    );
    named(
        'the layout from before the turn is used only when the laptop is upright again',
        two,
        'left-up',
        two.logical,
    );
    named(
        'panels of two scales, upright again: the monitor beside them comes nearer',
        around(
            50,
            1,
            [
                [1200, 0],
                [0, 0],
            ],
            1.5,
            [['DP-7', 1920, 1080, 2640, 0]],
        ),
        'normal',
    );
    named(
        'panels of two scales, turned: the monitor beside them keeps up with the widest',
        around(
            51,
            0,
            [
                [0, 0],
                [0, 1440],
            ],
            1.5,
            [['DP-7', 1920, 1080, 2304, 0]],
        ),
        'left-up',
    );
    named(
        'panels away from the origin: the monitor beside them moves as far as they grow',
        around(
            52,
            0,
            [
                [720, 360],
                [720, 1800],
            ],
            1.25,
            [['DP-7', 2880, 1800, 3024, 560]],
        ),
        'left-up',
    );
    named(
        'panels away from the origin: the monitor below them moves as far as they shrink',
        around(
            53,
            0,
            [
                [720, 360],
                [720, 1800],
            ],
            1.25,
            [['DP-7', 2880, 1800, 720, 3240]],
        ),
        'left-up',
    );
    named(
        'a monitor right of the panels but above them does not move with them',
        around(
            54,
            0,
            [
                [0, 720],
                [0, 2160],
            ],
            1.25,
            [
                ['DP-7', 1920, 1080, 2304, 720],
                ['DP-8', 1000, 500, 2304, 220],
            ],
        ),
        'left-up',
    );
    named(
        'a monitor below the panels but left of them does not move with them',
        around(
            55,
            0,
            [
                [720, 0],
                [720, 1440],
            ],
            1.25,
            [
                ['DP-7', 2304, 1080, 720, 2880],
                ['DP-8', 1920, 1080, -1200, 2880],
            ],
        ),
        'left-up',
    );
    named(
        'a monitor below the panels but right of them does not move with them',
        around(
            56,
            0,
            [
                [720, 0],
                [720, 1440],
            ],
            1.25,
            [
                ['DP-7', 2304, 1080, 720, 2880],
                ['DP-8', 1000, 500, 3024, 2880],
            ],
        ),
        'left-up',
    );
    named(
        'a monitor left of the panels stays when the panels get narrower',
        around(
            57,
            1,
            [
                [3360, 0],
                [1920, 0],
            ],
            1.25,
            [
                ['DP-7', 1920, 1080, 0, 0],
                ['DP-8', 5280, 1000, 0, 2304],
            ],
        ),
        'normal',
    );
    named(
        'only one panel mirrored: the layout is left as it is',
        (() => {
            const st = twoPanels();

            at(st.logical, UPPER).transform = 4;
            return st;
        })(),
        'left-up',
    );
    named(
        'a monitor between the panels in the row, and the panels side by side',
        state(
            58,
            [monitor(UPPER), monitor(LOWER), SIZE('DP-7', 1000, 500)],
            [
                logical(UPPER, 720, 0, true),
                logical(LOWER, 3024, -720, false),
                logical('DP-7', 1080, -500, false, 1),
            ],
        ),
        'left-up',
    );
    named(
        'the same, the panels at two scales, with a monitor that lies above',
        state(
            59,
            [monitor(UPPER), monitor(LOWER), SIZE('DP-7', 1920, 1080)],
            [
                logical('DP-7', 0, -1080, false, 1),
                logical(LOWER, -2304, -360, false),
                logical(UPPER, 0, 0, true, 1.5),
            ],
        ),
        'left-up',
    );

    const after = (name, st, orientation, upright) =>
        add(name, 'uprightAfter', {state: N(st), orientation, upright: L(upright)});

    after('the layout to remember: away from upright', two, 'left-up', null);
    after('the layout to remember: from one side to the other', leftUp, 'right-up', two.logical);
    after('the layout to remember: upright again', leftUp, 'normal', two.logical);
    after(
        'the layout to remember: upside down keeps the one from before',
        leftUp,
        'bottom-up',
        two.logical,
    );
    after('the layout to remember: upside down, nothing before', leftUp, 'bottom-up', null);
    after('the layout to remember: upright panels, upside down', two, 'bottom-up', null);
    after(
        'the layout to remember: no built-in panel shown',
        state(12, [monitor('DP-7')], [logical('DP-7', 0, 0, true, 1)]),
        'left-up',
        null,
    );

    for (const managed of [false, true])
        for (const sensor of [false, true])
            for (const builtin of [false, true])
                add(`control managed ${managed} sensor ${sensor} builtin ${builtin}`, 'control', {
                    managed,
                    sensor,
                    builtin,
                });
    const names = ['normal', 'left-up', 'right-up', 'bottom-up', 'undefined', 'constructor'];

    for (const orientation of names)
        for (const direction of ['clockwise', 'counterclockwise'])
            add(`turned ${orientation} ${direction}`, 'turned', {orientation, direction});
    for (const orientation of [...names, null])
        add(`the buttons start from ${orientation}`, 'buttonsFrom', {orientation});

    const of = (name, st) => add(name, 'orientationOf', {state: N(st)});

    of('the orientation of upright panels', two);
    of('the orientation with the lower panel off', lowerOff());
    of('the orientation of panels turned left side up', leftUp);
    of('the orientation of panels turned right side up', turnedTo(two, 'right-up'));
    of('the orientation of panels turned differently', mixed);
    of('the orientation of panels upside down', withTransform(twoPanels(), 2, UPPER, LOWER));
    of('the orientation of mirrored panels', withTransform(twoPanels(), 4, UPPER, LOWER));
    of(
        'the orientation without a built-in panel',
        state(12, [monitor('DP-7')], [logical('DP-7', 0, 0, true, 1)]),
    );
    of('the orientation with the panels mirrored together', mirroredState());

    // One panel turned by itself: the other panels stay as they are.
    const one = (name, st, connector, orientation) =>
        add(name, 'turnOne', {state: N(st), connector, orientation});
    const faceToFace = {
        ...two,
        logical: [{...at(two.logical, UPPER)}, {...at(two.logical, LOWER), transform: 2}],
    };

    one('one panel by itself, face to face: the lower one by 180 degrees', two, LOWER, 'bottom-up');
    one('one panel by itself: the upper one by 180 degrees', two, UPPER, 'bottom-up');
    one('one panel by itself, 90 degrees: the upper one left side up', two, UPPER, 'left-up');
    one('one panel by itself, 90 degrees: the lower one right side up', two, LOWER, 'right-up');
    one(
        'both panels turned differently: the upper one right side up',
        faceToFace,
        UPPER,
        'right-up',
    );
    one('both panels turned differently: the lower one back upright', faceToFace, LOWER, 'normal');
    one(
        'side by side: the lower panel upright, the upper one stays turned',
        leftUp,
        LOWER,
        'normal',
    );
    one('side by side: the upper panel by 180 degrees', leftUp, UPPER, 'bottom-up');
    one('side by side, right side up: the upper panel back upright', rightTurned, UPPER, 'normal');
    one(
        'the order of the panels comes from the layout, not from their names',
        swappedRows,
        UPPER,
        'bottom-up',
    );
    one(
        'a panel on top of the layout stays on top when it is turned',
        swappedRows,
        LOWER,
        'left-up',
    );
    one('a laptop with one panel has that one turned', lowerOff(), UPPER, 'bottom-up');
    one(
        'a monitor right of the panels moves by as much as they grow',
        beside(2304, 200),
        UPPER,
        'left-up',
    );
    one(
        'a monitor below the panels moves by as much as they grow',
        beside(300, 2880),
        UPPER,
        'left-up',
    );
    one(
        'a monitor below the panels does not move when they stay as high',
        beside(300, 2880),
        LOWER,
        'bottom-up',
    );
    one('a monitor that is not beside the panels stays', away(), UPPER, 'left-up');
    one(
        'when nothing else fits, the monitors are put in one row',
        state(
            14,
            [monitor(UPPER), SIZE('DP-1', 2304, 600), SIZE('DP-2', 1000, 500)],
            [
                logical('DP-2', 2304, 1440, false, 1),
                logical(UPPER, 0, 0, true),
                logical('DP-1', 0, 1440, false, 1),
            ],
        ),
        UPPER,
        'left-up',
    );

    one('an unknown orientation changes nothing', two, UPPER, 'undefined');
    one('the name of something that each object has changes nothing', two, UPPER, 'constructor');
    one('an empty orientation changes nothing', two, UPPER, '');
    one(
        'a monitor that is not a built-in panel changes nothing',
        beside(2304, 200),
        'DP-7',
        'left-up',
    );
    one('a connector that is not there changes nothing', two, 'DP-9', 'left-up');
    one('a panel that is connected but off changes nothing', lowerOff(), LOWER, 'left-up');
    one('a panel in the orientation that is asked for changes nothing', two, UPPER, 'normal');
    one(
        'a turned panel in the orientation that is asked for changes nothing',
        faceToFace,
        LOWER,
        'bottom-up',
    );
    one('panels that are mirrored together change nothing', mirroredState(), UPPER, 'left-up');
    for (const transform of [4, 5, 6, 7]) {
        const st = twoPanels();

        at(st.logical, LOWER).transform = transform;
        one(`the other panel is mirrored (${transform}): nothing is turned`, st, UPPER, 'left-up');
        one(`the panel is mirrored (${transform}): nothing is turned`, st, LOWER, 'normal');
    }
    one(
        'a monitor without a mode changes nothing',
        state(
            16,
            [monitor(UPPER), NO_MODE('DP-7')],
            [logical(UPPER, 0, 0, true), logical('DP-7', 2304, 0, false, 1)],
        ),
        UPPER,
        'left-up',
    );

    // Auto-rotate on again: the panels that were turned by hand follow the
    // laptop. `top` is the order of the compositor's list, upper panel first.
    const upperFirst = [UPPER, LOWER];

    named(
        'from face to face to upright: the layout from before',
        faceToFace,
        'normal',
        two.logical,
    );
    named('from face to face to upright: no layout from before', faceToFace, 'normal');
    named(
        'from face to face to upright: the order of the list',
        faceToFace,
        'normal',
        null,
        upperFirst,
    );
    named(
        'from face to face to left side up: the order of the list',
        faceToFace,
        'left-up',
        null,
        upperFirst,
    );
    named(
        'from face to face to right side up: the order of the list',
        faceToFace,
        'right-up',
        null,
        upperFirst,
    );
    named(
        'from face to face to upside down: the order of the list',
        faceToFace,
        'bottom-up',
        null,
        upperFirst,
    );
    named('turned differently, the lower panel first in the list', faceToFace, 'left-up', null, [
        LOWER,
        UPPER,
    ]);
    named(
        'turned differently, a list that misses a panel is not used',
        faceToFace,
        'left-up',
        null,
        [UPPER],
    );
    named('panels turned the same way ignore the list', two, 'left-up', null, [LOWER, UPPER]);

    // Panels put back to their orientations, for instance after a monitor change.
    const many = (name, st, held) => add(name, 'turnMany', {state: N(st), held});

    many('both panels put back: upper upright, lower upside down', twoPanels(), {
        [UPPER]: 'normal',
        [LOWER]: 'bottom-up',
    });
    many('both panels put back: each turned', twoPanels(), {
        [UPPER]: 'left-up',
        [LOWER]: 'right-up',
    });
    many('a panel that is gone keeps no place: only the upper one is turned', lowerOff(), {
        [UPPER]: 'bottom-up',
        [LOWER]: 'left-up',
    });
    many('panels that have their orientations already', faceToFace, {
        [UPPER]: 'normal',
        [LOWER]: 'bottom-up',
    });
    many('no orientations', two, {});
    many('an unknown orientation among them', two, {[UPPER]: 'left-up', [LOWER]: 'undefined'});
    many('a mirrored panel', mirroredState(), {[UPPER]: 'left-up'});

    // A monitor that touched the old block touches the widest edge of the new one.
    const besideTall = () =>
        withTransform(
            state(
                40,
                [monitor(UPPER), monitor(LOWER), SIZE('DP-7', 1000, 500)],
                [
                    logical(UPPER, 0, 0, false),
                    logical(LOWER, 1440, 0, true),
                    logical('DP-7', 100, 2304, false, 1),
                ],
            ),
            3,
            UPPER,
            LOWER,
        );

    one(
        'a monitor right of the panels, the other panel turned',
        beside(2304, 2000),
        LOWER,
        'right-up',
    );
    one(
        'a monitor below side by side panels slides to the panel that reaches the edge',
        besideTall(),
        UPPER,
        'normal',
    );

    return cases;
}

function touchCases() {
    const {cases, add} = collect(runTouch);
    const devices = [
        ['i2c-0', ['name', 'subsystem']],
        ['i2c-ELAN9008:00', ['0018:04F3:4447.0006', 'driver', 'modalias', 'power']],
        ['i2c-ELAN9009:00', ['0018:04F3:4448.0005', 'driver', 'modalias', 'power']],
        ['i2c-ASUE1234:00', ['0018:0B05:1234.0001', 'driver']],
    ];
    const found = (name, list) => add(name, 'controllersOf', {devices: list});

    found('the two touch controllers are found with their ids in small letters', devices);
    found('the ids are read from the machine, not written down', [
        ['i2c-ELAN9008:00', ['0018:04F3:425B.0007']],
        ['i2c-ELAN9009:00', ['0018:04F3:425A.0008']],
    ]);
    found('a controller without a HID device is left out', [['i2c-ELAN9008:00', ['driver']]]);
    found('a controller with two HID devices is left out', [
        ['i2c-ELAN9009:00', ['0018:04F3:4448.0005', '0018:04F3:4449.0006']],
    ]);
    found('a machine without these controllers has none', [
        ['i2c-SYNA0001:00', ['0018:06CB:0001.0001']],
    ]);
    found('no devices', []);
    found('the case of the hex digits does not matter', [
        ['i2c-ELAN9008:00', ['0018:04f3:44Ab.0006']],
    ]);
    found('a name in small letters is not a controller', [
        ['i2c-elan9008:00', ['0018:04F3:4447.0006']],
    ]);
    found('a name without a colon is not a controller', [
        ['i2c-ELAN9008', ['0018:04F3:4447.0006']],
        ['ELAN9009:00', ['0018:04F3:4448.0005']],
    ]);
    found('a name with more after the colon is a controller', [
        ['i2c-ELAN9008:00:more', ['0018:04F3:4447.0006']],
    ]);
    found('a name with something before the controller is not one', [
        ['i2c-x-ELAN9008:00', ['0018:04F3:4447.0006']],
    ]);
    found('a name that holds the name of a controller after something else is not one', [
        ['x-i2c-ELAN9008:00', ['0018:04F3:4447.0006']],
    ]);
    for (const child of [
        '0018:04F3:4447.',
        '0018:04F3:4447',
        '0018:04F3:4447:0006',
        '0018-04F3-4447.0006',
        '018:04F3:4447.0006',
        '0018:04F3:447.0006',
        '0018:04F3:44470.0006',
        '0018:04F3:4447.000G',
        '0018:04F3:4447.0006\n',
        ' 0018:04F3:4447.0006',
        '0018:04F3:4447.00.06',
        '0018:04F3:444é.0006',
        '00G8:04F3:4447.0006',
        '0018:04F3:4447.0',
        '',
    ])
        found(`the HID name ${JSON.stringify(child)}`, [['i2c-ELAN9008:00', [child]]]);
    found('a HID device among other names with the same shape of none', [
        ['i2c-ELAN9008:00', ['driver', '0018:04F3:4447.0006', 'power']],
    ]);

    const one = (name, st, list = devices) =>
        add(name, 'assignments', {devices: list, state: N(st), upper: UPPER, lower: LOWER});

    one('each controller gets its panel, named with the connector', twoPanels());
    one('with the keyboard on the lower panel, only the upper one is assigned', docked());
    one('a panel that is connected but switched off is assigned too', lowerOff());
    one('no panel is connected', state(30, [monitor('DP-7')], [logical('DP-7', 0, 0, true, 1)]));
    one(
        'a panel without a mode is known',
        state(31, [NO_MODE(UPPER), monitor(LOWER)], [logical(LOWER, 0, 0, true)]),
    );
    one('no controllers', twoPanels(), []);
    add('other connector names', 'assignments', {
        devices,
        state: N(state(32, [monitor('DP-1'), monitor('DP-2')], [logical('DP-1', 0, 0, true)])),
        upper: 'DP-2',
        lower: 'DP-1',
    });

    return cases;
}

/** All cases, by the file they go to. */
export function build() {
    return {layout: layoutCases(), rotation: rotationCases(), touch: touchCases()};
}
