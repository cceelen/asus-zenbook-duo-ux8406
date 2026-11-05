// The applet in the system tray, without a Plasma session: its metadata,
// and the names in its QML, which must be those of the program's service.
// Run with `node --test`; meson.build passes the id as APPLET_ID.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const metadata = JSON.parse(read('../applet/metadata.json'));
const qml = read('../applet/contents/ui/main.qml');
const service = read('../src/service.rs');
const constant = (name) => new RegExp(`pub const ${name}: &str = "([^"]+)";`).exec(service)[1];

test('metadata.json carries the id the build installs under', () => {
    const id = process.env.APPLET_ID;
    assert.ok(id, 'APPLET_ID is not set; run the tests with `meson test`');
    assert.equal(metadata.KPlugin.Id, id);
    assert.equal(metadata.KPackageStructure, 'Plasma/Applet');
});

test('the system tray shows it by itself, while the service is there', () => {
    assert.equal(metadata.KPlugin.EnabledByDefault, true);
    assert.equal(metadata['X-Plasma-NotificationAreaCategory'], 'Hardware');
    assert.equal(metadata['X-Plasma-DBusActivationService'], constant('NAME'));
});

test('the QML names the service, its object and what it has', () => {
    assert.ok(qml.includes(`"${constant('NAME')}"`));
    assert.ok(qml.includes(`"${constant('PATH')}"`));
    for (const name of [
        'SetEnabled',
        'Toggle',
        'Turn',
        'TurnScreen',
        'Screens',
        'Control',
        'Enabled',
        'CanTurnClockwise',
        'CanTurnCounterclockwise',
    ])
        assert.ok(qml.includes(name), name);
    for (const control of ['"switch"', '"buttons"', '"none"']) assert.ok(qml.includes(control));
});

test('a middle click calls Toggle, and the state is a real boolean', () => {
    assert.match(qml, /call\("Toggle", "", \[\]\)/);
    assert.match(
        qml,
        /Qt\.MiddleButton\) \{\s*if \(root\.control === "switch"\) \{\s*root\.toggle\(\)/,
    );
    assert.ok(!qml.includes('setRotating(!'));
    assert.match(qml, /rotating: status\.Enabled === true/);
});

test('a left click, the shortcut of the applet and the keyboard open the popup', () => {
    assert.match(qml, /root\.expanded = !root\.expanded;/);
    assert.match(qml, /acceptedButtons: Qt\.LeftButton \| Qt\.MiddleButton/);
    // Plasma gives `activated` to the keyboard: it must not toggle.
    assert.match(qml, /Plasmoid\.onActivated: root\.expanded = !root\.expanded/);
    assert.ok(!qml.includes('onSecondaryActivated'));
});

test('the popup turns each screen to four orientations, and the keyboard reaches them', () => {
    assert.match(qml, /call\("TurnScreen", "ss"/);
    for (const orientation of ['normal', 'left-up', 'right-up', 'bottom-up'])
        assert.ok(qml.includes(`"orientation": "${orientation}"`), orientation);
    assert.match(qml, /focusPolicy: Qt\.StrongFocus/);
    assert.match(qml, /highlighted: choice\.present/);
});

test('every button has an accessible name', () => {
    const buttons = qml.match(/PlasmaComponents\.(Button|Switch) \{/g).length;
    const names = qml.match(/Accessible\.name:/g).length;

    assert.equal(buttons, names);
});
