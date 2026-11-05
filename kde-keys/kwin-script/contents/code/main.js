// ASUS Zenbook Duo UX8406 in KDE Plasma: two keys of the detachable keyboard.
//
// - The key right of F12 switches the lower screen on and off. A KWin script
//   cannot change the outputs: the program asus-ux8406-kde-keys does it, and
//   this script calls it over D-Bus. With the keyboard on the lower screen,
//   the program does nothing.
// - F8 swaps the windows of the two screens.
//
// The keyboard sends these keys only with asus-zenbook-duo-ux8406-keyboard-bpf,
// as KEY_F18 (XF86Launch9, for Qt "Launch (9)") and KEY_F19 (F19). The user can
// give the two actions other keys in System Settings, Shortcuts, KWin.

const UPPER = 'eDP-1';
const LOWER = 'eDP-2';
const SERVICE = 'io.github.cceelen.AsusUX8406.Keys';
const PATH = '/io/github/cceelen/AsusUX8406/Keys';

function toggleLowerScreen() {
    callDBus(SERVICE, PATH, SERVICE, 'ToggleLowerScreen');
}

// Decide first where each window goes, then move them: a window moved in the
// loop would otherwise be found on the other screen and moved back. KWin keeps
// the place, the size and the tiling of a window on the screen it goes to.
function swapWindows() {
    const screens = Array.from(workspace.screens);
    const upper = screens.find((screen) => screen.name === UPPER);
    const lower = screens.find((screen) => screen.name === LOWER);

    if (!upper || !lower) return;

    const moves = Array.from(workspace.windowList())
        .filter((window) => window.normalWindow && !window.deleted)
        .filter((window) => [UPPER, LOWER].includes(window.output?.name))
        .map((window) => [window, window.output.name === UPPER ? lower : upper]);

    for (const [window, screen] of moves) workspace.sendClientToScreen(window, screen);
}

registerShortcut(
    'ASUS UX8406 Toggle Lower Screen',
    'Switch the lower screen on or off',
    'Launch (9)',
    toggleLowerScreen,
);
registerShortcut(
    'ASUS UX8406 Swap Windows',
    'Swap the windows of the two screens',
    'F19',
    swapWindows,
);
