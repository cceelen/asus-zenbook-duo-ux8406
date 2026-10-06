// The display layout for an orientation of the laptop. No GNOME imports here,
// so that it can be tested with plain data (rotation.test.js).
import {find, isRest, isSame, sizeOf, toOrigin} from './state.js';

/**
 * The compositor's transform for each orientation of the sensor that turns
 * the panels: upright, and on the left or the right side.
 */
const TRANSFORM = {normal: 0, 'left-up': 1, 'right-up': 3};

/** The built-in panels that are shown, each by itself. */
export function panelsOf(state) {
    return state.builtin
        .map((connector) => find(state.logical, connector))
        .filter((panel) => panel?.connectors.length === 1);
}

/**
 * The panels in the order in which they lie on the laptop, from its top to
 * its bottom. The layout tells: upright, a panel further down in the layout
 * is further down on the laptop; turned, the layout follows the laptop.
 */
function fromTop(panels) {
    const place = {
        0: (panel) => panel.y,
        1: (panel) => -panel.x,
        2: (panel) => -panel.y,
        3: (panel) => panel.x,
    }[panels[0].transform];

    return [...panels].sort((one, other) => place(one) - place(other));
}

/** The place a monitor of a layout takes up: {x, y, width, height}. */
function areaOf(state, monitor) {
    const mode = state.connected.get(monitor.connectors[0]);

    return {x: monitor.x, y: monitor.y, ...sizeOf(monitor, mode, state.logicalMode)};
}

/**
 * Whether the compositor can take a layout: no monitor lies over another,
 * and each one touches another along an edge, so that the pointer can go
 * from any monitor to any other.
 */
export function isValid(state, layout) {
    const areas = layout.map((monitor) => areaOf(state, monitor));
    const span = (a, b, from, size) =>
        Math.min(a[from] + a[size], b[from] + b[size]) - Math.max(a[from], b[from]);
    const over = (a, b) => span(a, b, 'x', 'width') > 0 && span(a, b, 'y', 'height') > 0;
    const touch = (a, b) =>
        (span(a, b, 'x', 'width') === 0 && span(a, b, 'y', 'height') > 0) ||
        (span(a, b, 'y', 'height') === 0 && span(a, b, 'x', 'width') > 0);
    const reached = new Set([0]);

    for (const [index, area] of areas.entries())
        if (areas.some((other, at) => at < index && over(area, other))) return false;
    for (const from of reached)
        for (const [to, area] of areas.entries()) if (touch(areas[from], area)) reached.add(to);
    return reached.size === areas.length;
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

            places.set(panel, across ? {x: x + width, y, transform} : {x, y: y + height, transform});
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
        x: monitor.x >= right && area.y < down && area.y + area.height > up ? monitor.x + wider : monitor.x,
        y: monitor.y >= down && area.x < right && area.x + area.width > left ? monitor.y + taller : monitor.y,
    });

    const row = [];
    let x = 0;

    for (const monitor of [...state.logical].sort((one, other) => one.x - other.x)) {
        if (!panels.includes(monitor)) {
            row.push({...monitor, x, y: 0});
            x += areaOf(state, monitor).width;
        } else if (monitor === fromLeft(panels)) {
            const together = block(x, 0);

            for (const [panel, place] of together.places) row.push({...panel, ...place});
            x += together.width;
        }
    }
    // The compositor wants the monitors in the order in which it has them.
    row.sort(
        (one, other) =>
            state.logical.indexOf(find(state.logical, one.connectors[0])) -
            state.logical.indexOf(find(state.logical, other.connectors[0])),
    );

    return [withOthers(beside), withOthers(() => ({})), row].map(toOrigin);
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
 * one, no built-in panel is shown by itself, a monitor has no mode, or the
 * panels are turned that way already: the place of the monitors is then not
 * changed, whatever it is.
 */
export function rotate(state, orientation, upright = null) {
    const panels = panelsOf(state);

    if (TRANSFORM[orientation] === undefined || panels.length === 0) return null;
    // Turned as the laptop is: the layout is the user's, or the compositor's
    // own, and is left alone.
    if (panels.every((panel) => panel.transform === TRANSFORM[orientation])) return null;
    if (state.logical.some((monitor) => !state.connected.get(monitor.connectors[0]))) return null;

    const ways = [
        orientation === 'normal' ? restored(state, upright) : null,
        ...placed(state, panels, orientation),
    ];
    const layout = ways.find((way) => way && isValid(state, way));

    return !layout || isSame(layout, state.logical) ? null : layout;
}
