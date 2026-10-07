// The display layout for an orientation of the laptop. No GNOME imports here,
// so that it can be tested with plain data (rotation.test.js).
import {areaOf, DOWN, find, isRest, isValid, toOrigin} from './state.js';

/**
 * The compositor's transform for each orientation of the sensor that turns
 * the panels: upright, and on the left or the right side. This is right for
 * a panel that is mounted upright in the machine. For a panel that is mounted
 * turned, the compositor's transform also has the turn of the panel in it,
 * and GetCurrentState does not tell that one.
 */
const TRANSFORM = {normal: 0, 'left-up': 1, 'right-up': 3};

/** The built-in panels that are shown, each by itself. */
function panelsOf(state) {
    return state.builtin
        .map((connector) => find(state.logical, connector))
        .filter((panel) => panel?.connectors.length === 1);
}

/**
 * The panels in the order in which they lie on the laptop, from its top to
 * its bottom. The layout tells: upright, a panel further down in the layout
 * is further down on the laptop; turned, the layout follows the laptop.
 *
 * The last panel tells how the layout is turned: the compositor turns the
 * first built-in panel by itself where it follows the orientation, and that
 * one only. The layout is then still the one of the other panels.
 */
function fromTop(panels) {
    const [dx, dy] = DOWN[panels.at(-1).transform];
    const place = (panel) => panel.x * dx + panel.y * dy;

    return [...panels].sort((one, other) => place(one) - place(other));
}

/**
 * The layout from before the turn, if it still fits: the same monitors, each
 * at the same scale. Else null.
 */
function restored(state, upright) {
    if (!upright || !isRest(state.logical, upright)) return null;

    const fits = upright.every(
        (monitor) => find(state.logical, monitor.connectors[0]).scale === monitor.scale,
    );

    return fits ? upright.map((monitor) => ({...monitor})) : null;
}

/**
 * The layouts for an orientation, the best one first:
 *
 * 1. The panels placed anew from their top left corner; a monitor that lies
 *    beside them, right of them or below them, moves by as much as the panels
 *    grow in that direction.
 * 2. The same, with the other monitors left where they are.
 * 3. All monitors in one row, in the order in which they are from left to
 *    right, the panels together. The compositor can always take this one.
 */
function placed(state, panels, orientation) {
    const transform = TRANSFORM[orientation];
    const across = orientation !== 'normal';
    // The panels in the order they lie from the top left corner.
    const order = orientation === 'left-up' ? fromTop(panels).reverse() : fromTop(panels);
    const turned = (panel) => areaOf(state, {...panel, transform});

    /** The panels from (x, y) on; the size of the whole is returned too. */
    const block = (x, y) => {
        const places = new Map();
        let [width, height] = [0, 0];

        for (const panel of order) {
            const area = turned(panel);

            places.set(
                panel,
                across ? {x: x + width, y, transform} : {x, y: y + height, transform},
            );
            width = across ? width + area.width : Math.max(width, area.width);
            height = across ? Math.max(height, area.height) : height + area.height;
        }
        return {places, width, height};
    };
    const before = panels.map((panel) => areaOf(state, panel));
    const left = Math.min(...before.map((area) => area.x));
    const up = Math.min(...before.map((area) => area.y));
    const right = Math.max(...before.map((area) => area.x + area.width));
    const down = Math.max(...before.map((area) => area.y + area.height));
    const anew = block(left, up);
    const [wider, taller] = [anew.width - (right - left), anew.height - (down - up)];
    const withOthers = (move) =>
        state.logical.map((monitor) =>
            anew.places.has(monitor)
                ? {...monitor, ...anew.places.get(monitor)}
                : {...monitor, ...move(monitor, areaOf(state, monitor))},
        );
    const beside = (monitor, area) => ({
        // Beside the panels: it shares rows, or columns, with them.
        x:
            monitor.x >= right && area.y < down && area.y + area.height > up
                ? monitor.x + wider
                : monitor.x,
        y:
            monitor.y >= down && area.x < right && area.x + area.width > left
                ? monitor.y + taller
                : monitor.y,
    });

    // The place of each monitor when all lie in one row, the panels together.
    const row = new Map();
    const first = fromLeft(panels);
    let x = 0;

    for (const monitor of [...state.logical].sort((one, other) => one.x - other.x)) {
        if (!panels.includes(monitor)) {
            row.set(monitor, {x, y: 0});
            x += areaOf(state, monitor).width;
        } else if (monitor === first) {
            const together = block(x, 0);

            for (const [panel, place] of together.places) row.set(panel, place);
            x += together.width;
        }
    }
    // The compositor wants the monitors in the order in which it has them.
    const inOneRow = state.logical.map((monitor) => ({...monitor, ...row.get(monitor)}));

    return [withOthers(beside), withOthers(() => ({})), inOneRow].map(toOrigin);
}

/** The panel that is furthest to the left in the layout. */
function fromLeft(panels) {
    return panels.reduce((one, other) => (other.x < one.x ? other : one));
}

/**
 * The layout for an orientation of the laptop; null for nothing to do.
 *
 * - Upright: the built-in panels one below the other.
 * - Left side up: the panels turned and side by side, the bottom one on the
 *   left.
 * - Right side up: turned the other way, the bottom one on the right.
 *
 * A laptop with one panel has that one turned. `upright` is the layout from
 * before the panels were turned: upright again, it is put back as it was if
 * it still fits. Else the panels are placed anew, in the first way that the
 * compositor can take (refer to `placed`). Null if the orientation is another
 * one, no built-in panel is shown by itself, a panel is mirrored, a monitor
 * has no mode, or the panels are turned that way already: the place of the
 * monitors is then not changed, whatever it is.
 */
export function rotate(state, orientation, upright = null) {
    const panels = panelsOf(state);

    if (!Object.hasOwn(TRANSFORM, orientation) || panels.length === 0) return null;
    // A mirrored panel (transforms 4 to 7) is the user's own setting: a turn
    // would take the mirror away.
    if (panels.some((panel) => panel.transform > 3)) return null;
    // Turned as the laptop is: the layout is the user's, or the compositor's
    // own, and is left alone.
    if (panels.every((panel) => panel.transform === TRANSFORM[orientation])) return null;
    if (state.logical.some((monitor) => !state.connected.get(monitor.connectors[0]))) return null;

    const ways = [
        orientation === 'normal' ? restored(state, upright) : null,
        ...placed(state, panels, orientation),
    ];

    // Each of them turns a panel, so that none is the layout of now.
    return ways.find((way) => way && isValid(state, way)) ?? null;
}

/**
 * The layout to remember after `rotate` gave a layout for `state` and the
 * compositor took it: `state.logical` if that turn was away from upright,
 * to put it back as it was; nothing when the laptop is upright again; else
 * the one from before.
 */
export function uprightAfter(state, orientation, upright) {
    if (orientation === 'normal') return null;
    return panelsOf(state).every((panel) => panel.transform === 0) ? state.logical : upright;
}

/**
 * The control of this extension in the quick settings:
 *
 * - 'none': GNOME turns the panel itself and shows its own switch for the
 *   same setting, or the compositor has no built-in panel.
 * - 'switch': the switch "Auto-rotate", where iio-sensor-proxy gives the
 *   orientation of the laptop.
 * - 'buttons': two buttons that turn the panels, where no sensor gives it.
 */
export function control({managed, sensor, builtin}) {
    if (managed || !builtin) return 'none';
    return sensor ? 'switch' : 'buttons';
}

/** The orientations that the buttons go through, counterclockwise first. */
const ORDER = ['left-up', 'normal', 'right-up'];

/**
 * The orientation after one turn of the picture by 90 degrees, 'clockwise'
 * or 'counterclockwise'. Null where the turn goes past left side up or right
 * side up, or for another orientation.
 */
export function turned(orientation, direction) {
    const index = ORDER.indexOf(orientation);

    if (index < 0) return null;
    return ORDER[index + (direction === 'clockwise' ? 1 : -1)] ?? null;
}

/**
 * The orientation that the built-in panels are turned to; null if no panel
 * is shown by itself, or the panels are turned differently or another way.
 */
export function orientationOf(state) {
    const transforms = new Set(panelsOf(state).map((panel) => panel.transform));

    if (transforms.size !== 1) return null;

    const [transform] = transforms;

    return Object.keys(TRANSFORM).find((key) => TRANSFORM[key] === transform) ?? null;
}
