// What to do with the display layout when the lower panel is switched off or
// on by its key. No GNOME imports here, so that it can be tested with plain
// data (layout.test.js).
//
// The data is what org.gnome.Mutter.DisplayConfig.GetCurrentState returns,
// fully unpacked:
//   [serial, monitors, logicalMonitors, properties]
//   monitor:        [[connector, vendor, product, serial], modes, properties]
//   mode:           [id, width, height, refresh, preferredScale, scales, properties]
//   logicalMonitor: [x, y, scale, transform, primary, monitors, properties]

const LAYOUT_MODE_LOGICAL = 1;

/** The mode a monitor is in, or else the one it prefers. */
function modeOf(modes) {
    const current = modes.find((mode) => mode[6]['is-current']);
    const mode = current ?? modes.find((mode) => mode[6]['is-preferred']);

    return mode ? {id: mode[0], width: mode[1], height: mode[2]} : null;
}

/** The compositor's state in the form the functions below work on. */
export function readState([serial, monitors, logicalMonitors, properties]) {
    const connected = new Map();

    for (const [[connector], modes, settings] of monitors) {
        const mode = modeOf(modes);

        // What the monitor is set to besides its mode, to be kept.
        connected.set(
            connector,
            mode && {
                ...mode,
                colorMode: settings['color-mode'],
                rgbRange: settings['rgb-range'],
            },
        );
    }

    return {
        serial,
        logicalMode: (properties['layout-mode'] ?? LAYOUT_MODE_LOGICAL) === LAYOUT_MODE_LOGICAL,
        canSetMode: properties['supports-changing-layout-mode'] === true,
        connected,
        logical: logicalMonitors.map(([x, y, scale, transform, primary, shown]) => ({
            x,
            y,
            scale,
            transform,
            primary,
            connectors: shown.map(([connector]) => connector),
        })),
    };
}

/** The logical monitor that shows this connector, or undefined. */
export function find(logical, connector) {
    return logical.find((monitor) => monitor.connectors.includes(connector));
}

/** Move the layout so that its top left corner is at (0, 0). */
function toOrigin(logical) {
    const left = Math.min(...logical.map((monitor) => monitor.x));
    const top = Math.min(...logical.map((monitor) => monitor.y));

    return logical.map((monitor) => ({...monitor, x: monitor.x - left, y: monitor.y - top}));
}

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

/** The height a logical monitor takes up in the layout. */
function heightOf(monitor, mode, logicalMode) {
    const turned = monitor.transform % 2 === 1;
    const height = turned ? mode.width : mode.height;

    return logicalMode ? Math.round(height / monitor.scale) : height;
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
            y: above.y + heightOf(above, mode, state.logicalMode),
            scale: above.scale,
            transform: above.transform,
            primary: false,
            connectors: [connector],
        },
    ]);
}

/** Whether `logical` shows exactly the connectors of `saved` but `connector`. */
function isRest(logical, saved, connector) {
    const now = logical.flatMap((monitor) => monitor.connectors).sort();
    const before = saved
        .flatMap((monitor) => monitor.connectors)
        .filter((name) => name !== connector)
        .sort();

    return now.length === before.length && now.every((name, index) => name === before[index]);
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

/**
 * A layout as the argument list of ApplyMonitorsConfig, without the variants
 * around the property dictionaries: [x, y, scale, transform, primary,
 * [[connector, modeId, {colorMode, rgbRange}]]]. Null if a monitor in it has
 * no mode.
 */
export function toConfig(logical, connected) {
    const config = logical.map((monitor) => [
        monitor.x,
        monitor.y,
        monitor.scale,
        monitor.transform,
        monitor.primary,
        monitor.connectors.map((connector) => {
            const {id, colorMode, rgbRange} = connected.get(connector) ?? {};

            return [connector, id, {colorMode, rgbRange}];
        }),
    ]);

    return config.every((monitor) => monitor[5].every(([, mode]) => mode)) ? config : null;
}
