// The shared test cases (refer to gnome-common/tests/vectors.js), run against
// the JavaScript of this extension.
// Run with `node --test`.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';

import {build, fromNeutral, toLayout} from '../../gnome-common/tests/vectors.js';
import {toggle, withBelow, without} from '../extension/layout.js';
import {isRest, isValid, shifted, sizeOf, toOrigin} from '../extension/state.js';
import {assignments, controllersOf} from '../extension/touch.js';

const read = (name) =>
    JSON.parse(
        readFileSync(new URL(`../../kde-common/tests/vectors/${name}.json`, import.meta.url)),
    );

const layout = (entry) => {
    const state = entry.state && fromNeutral(entry.state);

    switch (entry.call) {
        case 'without':
            return toLayout(without(state, entry.connector, entry.heir));
        case 'withBelow':
            return toLayout(withBelow(state, entry.connector, entry.anchor));
        case 'toggle': {
            const {apply, save} = toggle(state, entry.saved, entry.lower, entry.upper);

            return {apply: toLayout(apply), save: toLayout(save)};
        }
        case 'isValid':
            return isValid(state, entry.layout);
        case 'toOrigin':
            return toLayout(toOrigin(entry.layout));
        case 'shifted':
            return toLayout(shifted(state, entry.layout, entry.area, entry.direction, entry.steps));
        case 'isRest':
            return isRest(entry.layout, entry.saved, entry.except);
        case 'sizeOf': {
            const {width, height} = sizeOf(entry.monitor, entry.mode, entry.logicalMode);

            return {width, height};
        }
        default:
            throw new Error(`unknown call ${entry.call}`);
    }
};

const touch = (entry) => {
    switch (entry.call) {
        case 'controllersOf':
            return controllersOf(entry.devices);
        case 'assignments': {
            const state = fromNeutral(entry.state);
            const found = controllersOf(entry.devices);

            return assignments(found, state, entry.upper, entry.lower).map(({id, output}) => ({
                id,
                connector: output[3],
            }));
        }
        default:
            throw new Error(`unknown call ${entry.call}`);
    }
};

test('the files hold the cases of vectors.js', () => {
    const cases = build();

    assert.deepEqual(read('layout'), cases.layout);
    assert.deepEqual(read('touch'), cases.touch);
});

for (const [name, run] of [
    ['layout', layout],
    ['touch', touch],
]) {
    for (const entry of read(name)) {
        test(`${name}: ${entry.name}`, () => {
            assert.deepEqual(run(entry), entry.expect);
        });
    }
}
