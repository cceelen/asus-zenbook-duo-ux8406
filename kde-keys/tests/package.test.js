// The metadata of the KWin script must agree with the directory the build
// installs it under (meson.build passes that id as SCRIPT_ID), and have KWin
// load it without a setting.
import assert from 'node:assert/strict';
import {existsSync, readFileSync} from 'node:fs';
import {test} from 'node:test';

const metadata = JSON.parse(
    readFileSync(new URL('../kwin-script/metadata.json', import.meta.url), 'utf8'),
);

test('metadata.json carries the id the build installs under', () => {
    const id = process.env.SCRIPT_ID;
    assert.ok(id, 'SCRIPT_ID is not set; run the tests with `meson test`');
    assert.equal(metadata.KPlugin.Id, id);
});

test('a KWin script in JavaScript, enabled without a setting', () => {
    assert.equal(metadata.KPackageStructure, 'KWin/Script');
    assert.equal(metadata['X-Plasma-API'], 'javascript');
    assert.equal(metadata.KPlugin.EnabledByDefault, true);
});

test('the main script is where the metadata says', () => {
    const main = new URL(
        `../kwin-script/contents/${metadata['X-Plasma-MainScript']}`,
        import.meta.url,
    );

    assert.ok(existsSync(main));
});
