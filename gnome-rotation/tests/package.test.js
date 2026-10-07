// The extension's metadata must agree with the directory the build installs
// it under (meson.build passes that uuid as EXTENSION_UUID).
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

test('the extension has no settings of its own', () => {
    assert.equal(metadata['settings-schema'], undefined);
});
