// What to do with the display layout when the lower panel is switched off or
// on by its key. No GNOME imports here, so that it can be tested with plain
// data (layout.test.js).
import {areaOf, DOWN, find, isRest, isValid, shifted, sizeOf, toOrigin} from './state.js';

/**
 * The layout without the logical monitor that shows `connector`; null if it
 * is not shown, is mirrored with another monitor, or nothing would be left.
 * If it was the primary, `heir` becomes the primary, or else the first.
 *
 * Where the monitors do not hold together without it, as when it lies
 * between two of them, those right of it or below it come nearer by its
 * size.
 */
export function without(state, connector, heir) {
    const gone = find(state.logical, connector);

    if (gone?.connectors.length !== 1 || state.logical.length < 2) return null;

    const rest = state.logical
        .filter((monitor) => monitor !== gone)
        .map((monitor) => ({...monitor}));

    if (gone.primary) (find(rest, heir) ?? rest[0]).primary = true;

    const hole = areaOf(state, gone);
    const ways = [
        rest,
        shifted(state, rest, hole, [1, 0], -1),
        shifted(state, rest, hole, [0, 1], -1),
    ];

    return toOrigin(ways.find((way) => isValid(state, way)) ?? rest);
}

/**
 * The layout with `connector` at the bottom of the logical monitor that shows
 * `anchor`, at its scale and turned the same way: directly below it, or, with
 * the anchor turned, on the side where the bottom of the laptop then is. A
 * monitor that lies there moves on to make room. Null if the anchor is not
 * shown by itself or one of the two has no mode.
 */
export function withBelow(state, connector, anchor) {
    const above = find(state.logical, anchor);
    const mode = state.connected.get(anchor);
    const added = state.connected.get(connector);

    if (above?.connectors.length !== 1 || !mode || !added) return null;

    const down = DOWN[above.transform % 4];
    const [dx, dy] = down;
    const upper = sizeOf(above, mode, state.logicalMode);
    const panel = {
        x: above.x,
        y: above.y,
        scale: above.scale,
        transform: above.transform,
        primary: false,
        connectors: [connector],
    };
    const lower = sizeOf(panel, added, state.logicalMode);

    panel.x += dx > 0 ? upper.width : dx * lower.width;
    panel.y += dy > 0 ? upper.height : dy * lower.height;

    const place = {x: panel.x, y: panel.y, ...lower};
    const ways = [
        state.logical.map((monitor) => ({...monitor})),
        shifted(state, state.logical, place, down, 1),
    ].map((way) => [...way, panel]);

    return toOrigin(ways.find((way) => isValid(state, way)) ?? ways[0]);
}

/**
 * What the key does: {apply, save}. `apply` is the layout to give to the
 * compositor, or null for nothing; `save` is the layout to remember for the
 * next press, or null to forget.
 *
 * - The lower panel is shown: take it out and remember the layout.
 * - It is not shown but connected: put the remembered layout back if the
 *   other monitors are still the same and the upper panel is turned the same
 *   way, else put it below the upper panel.
 * - It is not connected (the keyboard lies on it): nothing.
 */
export function toggle(state, saved, lower, upper) {
    if (find(state.logical, lower)) {
        const apply = without(state, lower, upper);

        return {apply, save: apply ? state.logical : saved};
    }
    if (!state.connected.has(lower)) return {apply: null, save: saved};
    if (
        saved &&
        isRest(state.logical, saved, lower) &&
        find(saved, upper)?.transform === find(state.logical, upper)?.transform
    )
        return {apply: saved, save: null};
    return {apply: withBelow(state, lower, upper), save: null};
}
