// The display layout for an orientation of the laptop. No GNOME imports here,
// so that it can be tested with plain data (rotation.test.js).
import {areaOf, DOWN, find, isRest, isValid, toOrigin} from './state.js';

/**
 * The compositor's transform for each orientation of the sensor: upright, on
 * the left or the right side, and upside down. This is right for
 * a panel that is mounted upright in the machine. For a panel that is mounted
 * turned, the compositor's transform also has the turn of the panel in it,
 * and GetCurrentState does not tell that one.
 */
const TRANSFORM = {normal: 0, 'left-up': 1, 'bottom-up': 2, 'right-up': 3};

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
 * The panels in the order in which they lie on the laptop, from its top to
 * its bottom. Panels that are turned differently (by hand, one by one) have
 * no common turn that tells the order. Then `top` tells it, if it is given
 * and names all panels: the connectors from the top of the laptop to its
 * bottom, the order of the compositor's list of built-in panels. Else
 * `fromTop` decides as above.
 */
function topFirst(panels, top) {
    const alike = panels.every((panel) => panel.transform === panels[0].transform);
    const known = top && panels.every((panel) => top.includes(panel.connectors[0]));

    if (alike || !known) return fromTop(panels);

    const rank = (panel) => top.indexOf(panel.connectors[0]);

    return [...panels].sort((one, other) => rank(one) - rank(other));
}

/**
 * The layouts for a plan, the best one first. The plan says, for the panels:
 * `transformOf(panel)`, the transform that each one has; `across`, whether
 * they lie side by side (else one below the other); `order`, the panels in
 * the order from the top left corner.
 *
 * 1. The panels placed anew from their top left corner; a monitor that lies
 *    beside them, right of them or below them, moves by as much as the panels
 *    grow in that direction.
 * 2. The same, and a monitor that touched an edge of the old block also
 *    touches the widest edge of the new one: it slides along that edge to the
 *    nearest panel that reaches it. The block is not a rectangle when the
 *    panels differ in size.
 * 3. The first one, with the other monitors left where they are.
 * 4. All monitors in one row, in the order in which they are from left to
 *    right, the panels together as the block.
 * 5. The same, with each panel by itself in the row: the block is not a
 *    rectangle when the panels differ in size, and then a monitor beside it
 *    can miss a panel. The compositor can always take this one.
 */
function placed(state, panels, {transformOf, across, order}) {
    const turned = (panel) => areaOf(state, {...panel, transform: transformOf(panel)});

    /** The panels from (x, y) on; the size of the whole is returned too. */
    const block = (x, y) => {
        const places = new Map();
        let [width, height] = [0, 0];

        for (const panel of order) {
            const area = turned(panel);

            const transform = transformOf(panel);

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
                : {...monitor, ...move?.(monitor, areaOf(state, monitor))},
        );
    const sides = (monitor, area) => ({
        // Beside the panels: it shares rows, or columns, with them.
        right: monitor.x >= right && area.y < down && area.y + area.height > up,
        below: monitor.y >= down && area.x < right && area.x + area.width > left,
    });
    const beside = (monitor, area) => {
        const side = sides(monitor, area);

        return {
            x: side.right ? monitor.x + wider : monitor.x,
            y: side.below ? monitor.y + taller : monitor.y,
        };
    };
    // The panels of the new block, and those that reach its right and its
    // bottom edge.
    const fresh = [...anew.places].map(([panel, place]) => ({...turned(panel), ...place}));
    const reaching = (from, size, end) => fresh.filter((area) => area[from] + area[size] === end);
    const reachRight = reaching('x', 'width', left + anew.width);
    const reachBottom = reaching('y', 'height', up + anew.height);
    // `start` of a monitor of `length` along an edge, moved to the nearest
    // panel of `edge` if it misses all of them.
    const slide = (start, length, edge, from, size) => {
        const gap = (area) =>
            Math.max(area[from] - (start + length), start - (area[from] + area[size]), 0);

        if (
            edge.some(
                (area) =>
                    gap(area) === 0 &&
                    start < area[from] + area[size] &&
                    start + length > area[from],
            )
        )
            return start;

        const [near] = [...edge].sort((one, other) => gap(one) - gap(other));

        return start + length <= near[from] ? near[from] : near[from] + near[size] - length;
    };
    const fitted = (monitor, area) => {
        const side = sides(monitor, area);
        const moved = beside(monitor, area);

        return {
            x:
                side.below && monitor.y === down
                    ? slide(moved.x, area.width, reachBottom, 'x', 'width')
                    : moved.x,
            y:
                side.right && monitor.x === right
                    ? slide(moved.y, area.height, reachRight, 'y', 'height')
                    : moved.y,
        };
    };

    // All monitors in one row. `together`: the panels as the block, else
    // each by itself, in the order of the plan.
    const inRow = (together) => {
        const row = new Map();
        const waiting = [...order];
        let x = 0;

        for (const monitor of [...state.logical].sort((one, other) => one.x - other.x)) {
            if (!panels.includes(monitor)) {
                row.set(monitor, {x, y: 0});
                x += areaOf(state, monitor).width;
            } else if (!together) {
                const panel = waiting.shift();

                row.set(panel, {x, y: 0, transform: transformOf(panel)});
                x += turned(panel).width;
            } else if (waiting.length > 0) {
                // The first panel from the left brings the others with it.
                const whole = block(x, 0);

                waiting.length = 0;
                for (const [panel, place] of whole.places) row.set(panel, place);
                x += whole.width;
            }
        }
        // The compositor wants the monitors in the order in which it has them.
        return state.logical.map((monitor) => ({...monitor, ...row.get(monitor)}));
    };

    return [withOthers(beside), withOthers(fitted), withOthers(), inRow(true), inRow(false)].map(
        toOrigin,
    );
}

/**
 * The layout for an orientation of the laptop; null for nothing to do.
 *
 * - Upright: the built-in panels one below the other.
 * - Left side up: the panels turned and side by side, the bottom one on the
 *   left.
 * - Right side up: turned the other way, the bottom one on the right.
 * - Upside down: the panels turned by 180 degrees and one below the other,
 *   the bottom one on top.
 *
 * A laptop with one panel has that one turned. `upright` is the layout from
 * before the panels were turned: upright again, it is put back as it was if
 * it still fits. Else the panels are placed anew, in the first way that the
 * compositor can take (refer to `placed`). `top` is the order of the panels
 * from the top of the laptop, for panels that are turned differently, as in
 * `topFirst`. Null if the orientation is another
 * one, no built-in panel is shown by itself, a panel is mirrored, a monitor
 * has no mode, or the panels are turned that way already: the place of the
 * monitors is then not changed, whatever it is.
 */
export function rotate(state, orientation, upright = null, top = null) {
    const panels = panelsOf(state);

    if (!Object.hasOwn(TRANSFORM, orientation)) return null;
    if (unfit(state, panels)) return null;
    // Turned as the laptop is: the layout is the user's, or the compositor's
    // own, and is left alone. Also with no panel shown by itself: all of none.
    if (panels.every((panel) => panel.transform === TRANSFORM[orientation])) return null;

    // The panels in the order they lie from the top left corner: the bottom
    // one first where the bottom of the laptop is at the left or at the top.
    const bottomFirst = orientation === 'left-up' || orientation === 'bottom-up';
    const topmost = topFirst(panels, top);
    const ways = [
        orientation === 'normal' ? restored(state, upright) : null,
        ...placed(state, panels, {
            transformOf: () => TRANSFORM[orientation],
            across: orientation === 'left-up' || orientation === 'right-up',
            order: bottomFirst ? topmost.reverse() : topmost,
        }),
    ];

    // Each of them turns a panel, so that none is the layout of now. The last
    // one, each monitor by itself in one row, is always valid: each has a mode,
    // so each has a size, and in one row they touch without overlap.
    return ways.find((way) => way && isValid(state, way));
}

/**
 * Whether the panels cannot be turned: one is mirrored (transforms 4 to 7),
 * which is the user's own setting that a turn would take away, or a monitor
 * has no mode, so that it has no size.
 */
function unfit(state, panels) {
    return (
        panels.some((panel) => panel.transform > 3) ||
        state.logical.some((monitor) => !state.connected.get(monitor.connectors[0]))
    );
}

/**
 * How the panels lie in the layout now: `across` if they are side by side,
 * else one below the other, and the panels `order`ed from the left or from
 * the top. It follows the middle of each panel: a panel that is turned has
 * another size than the one beside it, and still lies where it lies.
 */
function arrangement(state, panels) {
    const middles = panels.map((panel) => {
        const area = areaOf(state, panel);

        return [2 * area.x + area.width, 2 * area.y + area.height];
    });
    const spread = (axis) =>
        Math.max(...middles.map((middle) => middle[axis])) -
        Math.min(...middles.map((middle) => middle[axis]));
    const across = spread(0) > spread(1);
    const place = (panel) => (across ? panel.x : panel.y);

    return {across, order: [...panels].sort((one, other) => place(one) - place(other))};
}

/**
 * The layout for built-in panels turned to orientations, the other panels as
 * they are: `held` is an object of connector and orientation. Null for
 * nothing to do. The panels stay as they lie: one below the other, or side by
 * side, in the same order. They are placed anew from their top left corner
 * with their new sizes, and the monitors beside them move as in `rotate`. An
 * entry for a connector that is not a built-in panel shown by itself (a panel
 * that is off or gone) is left out.
 *
 * Null if an orientation is unknown, a panel is mirrored, a monitor has no
 * mode, or each panel is in its orientation already.
 */
export function turnMany(state, held) {
    const panels = panelsOf(state);
    const target = (panel) =>
        Object.hasOwn(held, panel.connectors[0])
            ? TRANSFORM[held[panel.connectors[0]]]
            : panel.transform;

    if (Object.values(held).some((orientation) => !Object.hasOwn(TRANSFORM, orientation)))
        return null;
    if (unfit(state, panels) || panels.every((panel) => target(panel) === panel.transform))
        return null;

    const ways = placed(state, panels, {transformOf: target, ...arrangement(state, panels)});

    // The last way is always valid, as in `rotate`.
    return ways.find((way) => isValid(state, way));
}

/**
 * The layout for one built-in panel turned to an orientation (`turnMany` for
 * one panel). Null also for a connector that is not a built-in panel shown by
 * itself.
 */
export function turnOne(state, connector, orientation) {
    return turnMany(state, {[connector]: orientation});
}

/**
 * The built-in panels that can be turned one by one, in the order of the
 * compositor's list of built-in panels: `{connector, name, orientation}`.
 * The first of two is the upper screen and the second the lower one, as on
 * the UX8406. The orientation is null for a mirrored panel.
 */
export function screens(state) {
    const names =
        state.builtin.length === 2
            ? ['Upper screen', 'Lower screen']
            : state.builtin.map((_, index) =>
                  state.builtin.length === 1 ? 'Built-in screen' : `Built-in screen ${index + 1}`,
              );

    const panels = panelsOf(state);

    return state.builtin.flatMap((connector, index) => {
        const panel = panels.find((it) => it.connectors[0] === connector);

        if (!panel) return [];

        const orientation = Object.keys(TRANSFORM).find(
            (key) => TRANSFORM[key] === panel.transform,
        );

        return [{connector, name: names[index], orientation: orientation ?? null}];
    });
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
 * The orientation that the buttons start from: the orientation of the
 * panels when the buttons go through it, else upright. Upside down, or
 * with the panels turned differently (null), the buttons start from
 * upright, so that both of them stay usable.
 */
export function buttonsFrom(orientation) {
    return ORDER.includes(orientation) ? orientation : 'normal';
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
