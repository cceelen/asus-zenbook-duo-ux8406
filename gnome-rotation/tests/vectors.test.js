// The shared test cases (refer to gnome-common/tests/vectors.js), run against
// the JavaScript of this extension.
// Run with `node --test`.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';

import {build, fromNeutral, toLayout} from '../../gnome-common/tests/vectors.js';
import {
    buttonsFrom,
    control,
    orientationOf,
    rotate,
    turned,
    turnMany,
    turnOne,
    uprightAfter,
} from '../extension/rotation.js';

const read = (name) =>
    JSON.parse(
        readFileSync(new URL(`../../gnome-common/tests/vectors/${name}.json`, import.meta.url)),
    );

const run = (entry) => {
    const state = entry.state && fromNeutral(entry.state);

    switch (entry.call) {
        case 'rotate':
            return toLayout(rotate(state, entry.orientation, entry.upright, entry.top ?? null));
        case 'turnMany':
            return toLayout(turnMany(state, entry.held));
        case 'turnOne':
            return toLayout(turnOne(state, entry.connector, entry.orientation));
        case 'uprightAfter':
            return toLayout(uprightAfter(state, entry.orientation, entry.upright));
        case 'orientationOf':
            return orientationOf(state);
        case 'turned':
            return turned(entry.orientation, entry.direction);
        case 'buttonsFrom':
            return buttonsFrom(entry.orientation);
        case 'control':
            return control({managed: entry.managed, sensor: entry.sensor, builtin: entry.builtin});
        default:
            throw new Error(`unknown call ${entry.call}`);
    }
};

test('the file holds the cases of vectors.js', () => {
    assert.deepEqual(read('rotation'), build().rotation);
});

for (const entry of read('rotation')) {
    test(entry.name, () => {
        assert.deepEqual(run(entry), entry.expect);
    });
}
