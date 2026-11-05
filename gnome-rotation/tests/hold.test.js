// Tests for hold.js, the hold of the manual turns. Run with `node --test`.
import assert from 'node:assert/strict';
import {test} from 'node:test';

import {Hold} from '../extension/hold.js';

const UPPER = 'eDP-1';
const LOWER = 'eDP-2';
const screens = (upper, lower) => [
    {connector: UPPER, name: 'Upper screen', orientation: upper},
    {connector: LOWER, name: 'Lower screen', orientation: lower},
];
const turn = (hold, list, connector, orientation) =>
    hold.handle({type: 'turn', screens: list, connector, orientation});

test('nothing is held at the start: the switch shows the setting', () => {
    const hold = new Hold();

    assert.equal(hold.active, false);
    assert.equal(hold.top, null);
    assert.deepEqual(hold.held, {});
    assert.equal(hold.isAuto(false), true);
    assert.equal(hold.isAuto(true), false);
});

test('a turn by hand holds all panels: the others as they are', () => {
    const hold = new Hold();

    assert.deepEqual(turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up'), []);
    assert.equal(hold.active, true);
    assert.deepEqual(hold.held, {[UPPER]: 'normal', [LOWER]: 'bottom-up'});
    assert.deepEqual(hold.top, [UPPER, LOWER]);
    assert.equal(hold.isAuto(false), false);
});

test('a mirrored panel has no orientation to hold', () => {
    const hold = new Hold();

    turn(hold, screens(null, 'normal'), LOWER, 'left-up');
    assert.deepEqual(hold.held, {[LOWER]: 'left-up'});
});

test('a second turn changes one panel and keeps the order', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    turn(hold, screens('normal', 'bottom-up').toReversed(), UPPER, 'left-up');
    assert.deepEqual(hold.held, {[UPPER]: 'left-up', [LOWER]: 'bottom-up'});
    assert.deepEqual(hold.top, [UPPER, LOWER]);
});

test('a turn that is refused goes back to what was held', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    turn(hold, screens('normal', 'bottom-up'), UPPER, 'left-up');
    assert.deepEqual(hold.handle({type: 'refused'}), []);
    assert.deepEqual(hold.held, {[UPPER]: 'normal', [LOWER]: 'bottom-up'});
    assert.equal(hold.active, true);
    assert.deepEqual(hold.top, [UPPER, LOWER]);
});

test('the first turn is refused: no hold, no order', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    hold.handle({type: 'refused'});
    assert.equal(hold.active, false);
    assert.equal(hold.top, null);
});

test('the switch: off when on, and on again after a hold or a lock', () => {
    const hold = new Hold();

    assert.deepEqual(hold.handle({type: 'switch', locked: false}), [{do: 'lock', value: true}]);
    // Locked in the setting: on again.
    assert.deepEqual(hold.handle({type: 'switch', locked: true}), [{do: 'lock', value: false}]);
    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    // On hold the switch shows off, whatever the setting says.
    assert.deepEqual(hold.handle({type: 'switch', locked: false}), [{do: 'lock', value: false}]);
    assert.equal(hold.active, false);
    // The order stays for the panels that are turned differently, until they follow.
    assert.deepEqual(hold.top, [UPPER, LOWER]);
    hold.handle({type: 'followed'});
    assert.equal(hold.top, null);
});

test('the panels follow later while a hold is on: the order stays', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    hold.handle({type: 'followed'});
    assert.deepEqual(hold.top, [UPPER, LOWER]);
});

test('the sensor is followed only without a hold', () => {
    const hold = new Hold();

    assert.deepEqual(hold.handle({type: 'sensor'}), [{do: 'follow'}]);
    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    assert.deepEqual(hold.handle({type: 'sensor'}), []);
});

test('monitors changed without a hold: follow the sensor', () => {
    assert.deepEqual(new Hold().handle({type: 'monitors', screens: screens('normal', 'normal')}), [
        {do: 'follow'},
    ]);
});

test('monitors changed on hold: each panel that is there is turned again', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    // The compositor made both upright.
    assert.deepEqual(hold.handle({type: 'monitors', screens: screens('normal', 'normal')}), [
        {do: 'turn', connector: LOWER, orientation: 'bottom-up'},
    ]);
    // Already as held: nothing to do.
    assert.deepEqual(hold.handle({type: 'monitors', screens: screens('normal', 'bottom-up')}), []);
});

test('a panel that is gone keeps its orientation until it is back', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    // The lower panel is off: only the upper one is listed.
    assert.deepEqual(hold.handle({type: 'monitors', screens: [screens('normal', null)[0]]}), []);
    assert.deepEqual(hold.held, {[UPPER]: 'normal', [LOWER]: 'bottom-up'});
    assert.deepEqual(hold.handle({type: 'monitors', screens: screens('normal', 'normal')}), [
        {do: 'turn', connector: LOWER, orientation: 'bottom-up'},
    ]);
});

test('a mirrored panel is not turned again; one that is not held is left alone', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    assert.deepEqual(hold.handle({type: 'monitors', screens: screens('normal', null)}), []);
    assert.deepEqual(
        hold.handle({
            type: 'monitors',
            screens: [
                ...screens('normal', 'bottom-up'),
                {connector: 'eDP-3', orientation: 'left-up'},
            ],
        }),
        [],
    );
});

test('a turn again that is refused drops the hold of that panel', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    assert.deepEqual(hold.handle({type: 'dropped', connector: LOWER}), []);
    assert.deepEqual(hold.held, {[UPPER]: 'normal'});
    assert.equal(hold.active, true);
    // The last one: the sensor is followed again.
    assert.deepEqual(hold.handle({type: 'dropped', connector: UPPER}), [{do: 'follow'}]);
    assert.equal(hold.active, false);
});

test('the buttons turn all panels together: no hold one by one', () => {
    const hold = new Hold();

    turn(hold, screens('normal', 'normal'), LOWER, 'bottom-up');
    assert.deepEqual(hold.handle({type: 'all'}), []);
    assert.equal(hold.active, false);
});

test('an unknown event is an error', () => {
    assert.throws(() => new Hold().handle({type: 'nonsense'}), /unknown event nonsense/);
});
