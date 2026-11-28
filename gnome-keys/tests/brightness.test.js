// Tests for brightness.js. Run with `node --test`.
import assert from 'node:assert/strict';
import {test} from 'node:test';

import {lowerBrightness} from '../extension/brightness.js';

const panel = (value, min = 4, max = 400) => ({value, min, max});

test('the value of the upper panel is copied when the ranges are equal', () => {
    assert.equal(lowerBrightness(panel(23), panel(61)), 23);
});

test('nothing is set when the lower panel has the value already', () => {
    assert.equal(lowerBrightness(panel(23), panel(23)), null);
});

test('the value keeps its place in the range of the lower panel', () => {
    assert.equal(lowerBrightness(panel(202), panel(0, 0, 100)), 50);
    assert.equal(lowerBrightness(panel(50, 0, 100), panel(0, 4, 400)), 202);
});

test('the value is rounded to a whole number', () => {
    assert.equal(lowerBrightness(panel(1, 0, 3), panel(0, 0, 10)), 3);
    assert.equal(lowerBrightness(panel(2, 0, 3), panel(0, 0, 10)), 7);
});

test('the bounds of the range give the bounds of the other range', () => {
    assert.equal(lowerBrightness(panel(4), panel(50, 0, 100)), 0);
    assert.equal(lowerBrightness(panel(400), panel(50, 0, 100)), 100);
});

test('a value outside the range is held inside the range of the lower panel', () => {
    assert.equal(lowerBrightness(panel(1000), panel(50)), 400);
    assert.equal(lowerBrightness(panel(-5), panel(50)), 4);
});

test('an upper range of one value gives the lowest value', () => {
    assert.equal(lowerBrightness(panel(7, 7, 7), panel(50)), 4);
});

test('nothing is set when a panel or a backlight is missing', () => {
    assert.equal(lowerBrightness(null, panel(5)), null);
    assert.equal(lowerBrightness(panel(5), null), null);
    assert.equal(lowerBrightness(undefined, undefined), null);
});

test('nothing is set when a number is missing or a range is wrong', () => {
    assert.equal(lowerBrightness({value: 5, min: 4}, panel(5)), null);
    assert.equal(lowerBrightness(panel(5), {value: 5, min: 4}), null);
    assert.equal(lowerBrightness({min: 4, max: 400}, panel(5)), null);
    assert.equal(lowerBrightness(panel(Number.NaN), panel(5)), null);
    assert.equal(lowerBrightness(panel(5, 400, 4), panel(5)), null);
    assert.equal(lowerBrightness(panel(5), panel(5, 400, 4)), null);
});
