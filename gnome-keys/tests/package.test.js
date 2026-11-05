// The extension's metadata must be what extensions.gnome.org takes and agree
// with the GSettings schema it ships.
// In this tree it must also carry the uuid that the build installs it under
// (meson.build passes it as EXTENSION_UUID).
import assert from 'node:assert/strict';
import {readdirSync, readFileSync} from 'node:fs';
import {test} from 'node:test';

const root = new URL('../', import.meta.url);
const read = (path) => readFileSync(new URL(path, root), 'utf8');
const metadata = JSON.parse(read('extension/metadata.json'));

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

test('metadata.json lists the supported shell versions', () => {
    assert.ok(Array.isArray(metadata['shell-version']));
    assert.ok(metadata['shell-version'].length > 0);
});

test('shell-version holds plain major versions as strings', () => {
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

test('the schema id and path match metadata.json and extension.js', () => {
    const dir = new URL('extension/schemas/', root);
    const files = readdirSync(dir).filter((f) => f.endsWith('.gschema.xml'));
    assert.equal(files.length, 1);
    const xml = readFileSync(new URL(files[0], dir), 'utf8');
    const id = /<schema\s[^>]*id="([^"]+)"/.exec(xml)?.[1];
    const path = /<schema\s[^>]*path="([^"]+)"/.exec(xml)?.[1];
    assert.equal(metadata['settings-schema'], id);
    assert.equal(files[0], `${id}.gschema.xml`);
    assert.equal(path, `/${id.replaceAll('.', '/')}/`);
    // extension.js asks for its settings with getSettings(); that call reads
    // settings-schema from metadata.json, so it must not name another id.
    const code = read('extension/extension.js');
    for (const [, named] of code.matchAll(/getSettings\(\s*['"]([^'"]+)['"]/g))
        assert.equal(named, id);
    for (const [, named] of code.matchAll(/org\.gnome\.shell\.extensions\.[\w.-]+/g))
        assert.equal(named, id);
});
