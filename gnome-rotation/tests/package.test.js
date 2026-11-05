// The extension's metadata must be what extensions.gnome.org takes.
// In this tree it must also carry the uuid that the build installs it under
// (meson.build passes it as EXTENSION_UUID).
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';

const metadata = JSON.parse(
    readFileSync(new URL('../extension/metadata.json', import.meta.url), 'utf8'),
);

test('metadata.json carries the uuid the build installs under', () => {
    const uuid = process.env.EXTENSION_UUID;
    assert.ok(uuid, 'EXTENSION_UUID is not set; run the tests with `meson test`');
    assert.equal(metadata.uuid, uuid);
});

test('metadata.json links the repository for problem reports', () => {
    assert.equal(metadata.url, 'https://github.com/cceelen/gnome-extension');
});

test('metadata.json has no version: extensions.gnome.org sets it', () => {
    assert.equal(metadata.version, undefined);
    assert.match(metadata['version-name'], /^[0-9]+\.[0-9]+\.[0-9]+$/);
});

test('shell-version holds plain major versions as strings', () => {
    assert.ok(Array.isArray(metadata['shell-version']));
    assert.ok(metadata['shell-version'].length > 0);
    for (const version of metadata['shell-version'])
        assert.match(
            version,
            /^[1-9][0-9]*$/,
            `not a plain major version: ${JSON.stringify(version)}`,
        );
});

test('shell-version declares GNOME Shell 45 to 51 without a gap', () => {
    assert.deepEqual(metadata['shell-version'], ['45', '46', '47', '48', '49', '50', '51']);
});

test('the extension has no settings of its own', () => {
    assert.equal(metadata['settings-schema'], undefined);
});
