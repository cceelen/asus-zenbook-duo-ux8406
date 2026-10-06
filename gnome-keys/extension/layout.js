// What to do with the display layout when the lower panel is switched off or
// on by its key. No GNOME imports here, so that it can be tested with plain
// data (layout.test.js).
import {find, isRest, sizeOf, toOrigin} from './state.js';

/**
 * The layout without the logical monitor that shows `connector`; null if it
 * is not shown, is mirrored with another monitor, or nothing would be left.
 * If it was the primary, `heir` becomes the primary, or else the first.
 */
export function without(logical, connector, heir) {
    const gone = find(logical, connector);

    if (gone?.connectors.length !== 1 || logical.length < 2) return null;

    const rest = logical.filter((monitor) => monitor !== gone).map((monitor) => ({...monitor}));

    if (gone.primary) (find(rest, heir) ?? rest[0]).primary = true;
    return toOrigin(rest);
}

/**
 * The layout with `connector` directly below the logical monitor that shows
 * `anchor`, at its scale and turned the same way; null if the anchor is not
 * shown by itself or one of the two has no mode.
 */
export function withBelow(state, connector, anchor) {
    const above = find(state.logical, anchor);
    const mode = state.connected.get(anchor);

    if (above?.connectors.length !== 1 || !mode || !state.connected.get(connector)) return null;

    return toOrigin([
        ...state.logical.map((monitor) => ({...monitor})),
        {
            x: above.x,
            y: above.y + sizeOf(above, mode, state.logicalMode).height,
            scale: above.scale,
            transform: above.transform,
            primary: false,
            connectors: [connector],
        },
    ]);
}

/**
 * What the key does: {apply, save}. `apply` is the layout to give to the
 * compositor, or null for nothing; `save` is the layout to remember for the
 * next press, or null to forget.
 *
 * - The lower panel is shown: take it out and remember the layout.
 * - It is not shown but connected: put the remembered layout back if the
 *   other monitors are still the same, else put it below the upper panel.
 * - It is not connected (the keyboard lies on it): nothing.
 */
export function toggle(state, saved, lower, upper) {
    if (find(state.logical, lower)) {
        const apply = without(state.logical, lower, upper);

        return {apply, save: apply ? state.logical : saved};
    }
    if (!state.connected.has(lower)) return {apply: null, save: saved};
    if (saved && isRest(state.logical, saved, lower)) return {apply: saved, save: null};
    return {apply: withBelow(state, lower, upper), save: null};
}
