// Tests for touch.js, with the devices seen on the UX8406CA. Run with
// `node --test`.
import assert from 'node:assert/strict';
import {test} from 'node:test';

import {
    LOWER,
    logical,
    monitor,
    properties,
    twoPanels,
    UPPER,
} from '../../gnome-common/tests/fixtures.js';
import {readState} from '../extension/state.js';
import {assignments, controllersOf} from '../extension/touch.js';

// What /sys/bus/i2c/devices holds on the UX8406CA, in part.
const devices = [
    ['i2c-0', ['name', 'subsystem']],
    ['i2c-ELAN9008:00', ['0018:04F3:4447.0006', 'driver', 'modalias', 'power']],
    ['i2c-ELAN9009:00', ['0018:04F3:4448.0005', 'driver', 'modalias', 'power']],
    ['i2c-ASUE1234:00', ['0018:0B05:1234.0001', 'driver']],
];

test('the two touch controllers are found with their ids in small letters', () => {
    assert.deepEqual(controllersOf(devices), [
        {controller: 'ELAN9008', id: '04f3:4447'},
        {controller: 'ELAN9009', id: '04f3:4448'},
    ]);
});

test('the ids are read from the machine, not written down', () => {
    // A UX8406MA, as its owners report it.
    const other = [
        ['i2c-ELAN9008:00', ['0018:04F3:425B.0007']],
        ['i2c-ELAN9009:00', ['0018:04F3:425A.0008']],
    ];

    assert.deepEqual(
        controllersOf(other).map(({id}) => id),
        ['04f3:425b', '04f3:425a'],
    );
});

test('a controller without a HID device, or with two, is left out', () => {
    assert.deepEqual(controllersOf([['i2c-ELAN9008:00', ['driver']]]), []);
    assert.deepEqual(
        controllersOf([['i2c-ELAN9009:00', ['0018:04F3:4448.0005', '0018:04F3:4449.0006']]]),
        [],
    );
});

test('a machine without these controllers has none', () => {
    assert.deepEqual(controllersOf([['i2c-SYNA0001:00', ['0018:06CB:0001.0001']]]), []);
    assert.deepEqual(controllersOf([]), []);
});

test('each controller gets its panel, named with the connector', () => {
    assert.deepEqual(assignments(controllersOf(devices), twoPanels(), UPPER, LOWER), [
        {id: '04f3:4447', output: ['SDC', '0x419d', '0x0', UPPER]},
        {id: '04f3:4448', output: ['SDC', '0x419d', '0x0', LOWER]},
    ]);
});

test('with the keyboard on the lower panel, only the upper one is assigned', () => {
    const docked = readState([9, [monitor(UPPER)], [logical(UPPER, 0, 0, true)], properties]);

    assert.deepEqual(assignments(controllersOf(devices), docked, UPPER, LOWER), [
        {id: '04f3:4447', output: ['SDC', '0x419d', '0x0', UPPER]},
    ]);
});

test('a panel that is connected but switched off is assigned too', () => {
    const state = readState([
        8,
        [monitor(UPPER), monitor(LOWER, false)],
        [logical(UPPER, 0, 0, true)],
        properties,
    ]);

    assert.equal(assignments(controllersOf(devices), state, UPPER, LOWER).length, 2);
});
