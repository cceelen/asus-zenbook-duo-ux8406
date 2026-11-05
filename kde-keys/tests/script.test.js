// Tests for the KWin script, in a stand-in for KWin's scripting API. Run
// with `node --test`.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import {fileURLToPath} from 'node:url';
import vm from 'node:vm';

const MAIN = fileURLToPath(new URL('../kwin-script/contents/code/main.js', import.meta.url));

/** A screen and a window as KWin's scripting API has them. */
const screen = (name) => ({name});
const window = (name, output, more = {}) => ({
    name,
    output,
    normalWindow: true,
    deleted: false,
    ...more,
});

/** Run the script in a stand-in for KWin: what it registered and called. */
function load(screens, windows) {
    const shortcuts = new Map();
    const calls = [];
    const moved = [];
    const kwin = {
        registerShortcut: (name, text, keys, action) => shortcuts.set(name, {text, keys, action}),
        callDBus: (...args) => calls.push(args),
        workspace: {
            screens,
            windowList: () => windows,
            sendClientToScreen: (client, output) => {
                moved.push([client.name, output.name]);
                client.output = output;
            },
        },
    };

    vm.runInNewContext(readFileSync(MAIN, 'utf8'), kwin, {filename: MAIN});
    return {shortcuts, calls, moved};
}

const [upper, lower, external] = [screen('eDP-1'), screen('eDP-2'), screen('DP-1')];

test('two shortcuts on the keys that the keyboard program sends', () => {
    const {shortcuts} = load([upper, lower], []);

    assert.deepEqual(
        [...shortcuts.values()].map(({keys}) => keys),
        ['Launch (9)', 'F19'],
    );
    for (const {text} of shortcuts.values()) assert.ok(text.length > 0);
});

test('the key right of F12 asks the program to switch the lower screen', () => {
    const {shortcuts, calls} = load([upper, lower], []);

    shortcuts.get('ASUS UX8406 Toggle Lower Screen').action();
    assert.deepEqual(calls, [
        [
            'io.github.cceelen.AsusUX8406.Keys',
            '/io/github/cceelen/AsusUX8406/Keys',
            'io.github.cceelen.AsusUX8406.Keys',
            'ToggleLowerScreen',
        ],
    ]);
});

test('F8 swaps the windows of the two panels, each one time', () => {
    const windows = [
        window('editor', upper),
        window('terminal', lower),
        window('browser', upper),
        window('video', external),
    ];
    const {shortcuts, moved} = load([upper, lower, external], windows);

    shortcuts.get('ASUS UX8406 Swap Windows').action();
    assert.deepEqual(moved, [
        ['editor', 'eDP-2'],
        ['terminal', 'eDP-1'],
        ['browser', 'eDP-2'],
    ]);
});

test('panels, docks, closed windows and windows without a screen stay', () => {
    const windows = [
        window('panel', lower, {normalWindow: false}),
        window('closing', upper, {deleted: true}),
        window('nowhere', null),
    ];
    const {shortcuts, moved} = load([upper, lower], windows);

    shortcuts.get('ASUS UX8406 Swap Windows').action();
    assert.deepEqual(moved, []);
});

test('with one panel only, F8 does nothing', () => {
    for (const screens of [[upper], [lower], [upper, external], []]) {
        const {shortcuts, moved} = load(screens, [window('editor', screens[0] ?? null)]);

        shortcuts.get('ASUS UX8406 Swap Windows').action();
        assert.deepEqual(moved, []);
    }
});
