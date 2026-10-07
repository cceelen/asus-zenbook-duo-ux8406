// The display layout of the compositor as plain data, and what the two
// extensions for the Zenbook Duo UX8406 both do with it. No GNOME imports
// here, so that it can be tested with plain data.
//
// The data is what org.gnome.Mutter.DisplayConfig.GetCurrentState returns,
// fully unpacked:
//   [serial, monitors, logicalMonitors, properties]
//   monitor:        [[connector, vendor, product, serial], modes, properties]
//   mode:           [id, width, height, refresh, preferredScale, scales, properties]
//   logicalMonitor: [x, y, scale, transform, primary, monitors, properties]

const LAYOUT_MODE_LOGICAL = 1;

/**
 * The direction in the layout that goes down the laptop, from its top to its
 * bottom, for each transform of a panel, as [x, y]: down when it is upright,
 * to the left with the left side up, up when it is upside down, to the right
 * with the right side up.
 */
export const DOWN = [
    [0, 1],
    [-1, 0],
    [0, -1],
    [1, 0],
];

/** The mode a monitor is in, or else the one it prefers. */
function modeOf(modes) {
    const current = modes.find((mode) => mode[6]['is-current']);
    const mode = current ?? modes.find((mode) => mode[6]['is-preferred']);

    return mode ? {id: mode[0], width: mode[1], height: mode[2]} : null;
}

/** The compositor's state in the form the functions below work on. */
export function readState([serial, monitors, logicalMonitors, properties]) {
    const connected = new Map();
    const builtin = [];

    for (const [[connector], modes, settings] of monitors) {
        const mode = modeOf(modes);

        if (settings['is-builtin'] === true) builtin.push(connector);

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
        // The connectors of the panels that are part of the machine.
        builtin,
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
export function toOrigin(logical) {
    const left = Math.min(...logical.map((monitor) => monitor.x));
    const top = Math.min(...logical.map((monitor) => monitor.y));

    return logical.map((monitor) => ({...monitor, x: monitor.x - left, y: monitor.y - top}));
}

/** The size a logical monitor takes up in the layout. */
export function sizeOf(monitor, mode, logicalMode) {
    const turned = monitor.transform % 2 === 1;
    const [width, height] = turned ? [mode.height, mode.width] : [mode.width, mode.height];

    return logicalMode
        ? {width: Math.round(width / monitor.scale), height: Math.round(height / monitor.scale)}
        : {width, height};
}

/** The place a monitor of a layout takes up: {x, y, width, height}. */
export function areaOf(state, monitor) {
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

/** Whether `logical` shows exactly the connectors of `saved`, without `except`. */
export function isRest(logical, saved, except = null) {
    const now = logical.flatMap((monitor) => monitor.connectors).sort();
    const before = saved
        .flatMap((monitor) => monitor.connectors)
        .filter((name) => name !== except)
        .sort();

    return now.length === before.length && now.every((name, index) => name === before[index]);
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
