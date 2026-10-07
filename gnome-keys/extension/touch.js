// Which panel each touchscreen and pen of the UX8406 belongs to. No GNOME
// imports here, so that it can be tested with plain data (touch.test.js).
//
// The compositor gives a touch device the monitor of its size. The two panels
// have one size and say that they are the same monitor (vendor, product and
// serial are equal), so without a setting both touchscreens get the first
// panel, and a touch on the lower panel acts on the upper one. The setting
// `output` of a touch device can name the monitor with a fourth value, the
// connector, which the compositor reads when two monitors are twins (mutter,
// src/backends/meta-input-mapper.c, match_config).

/**
 * The panel of each touch controller. Seen on the UX8406CA, and reported the
 * same for the UX8406MA: the controller's name tells the panel, its product
 * id does not (the ids are not the same on all units).
 */
const PANEL = {ELAN9008: 'upper', ELAN9009: 'lower'};

/**
 * The touch controllers among the i2c devices of the machine, as
 * [{controller, id}]. `devices` is a list of [name, children]: the name of a
 * directory in /sys/bus/i2c/devices and the names in it. The HID device in
 * it, bus:vendor:product.instance, gives the id as the compositor writes it,
 * vendor:product in small letters.
 */
export function controllersOf(devices) {
    const found = [];

    for (const [name, children] of devices) {
        const controller = /^i2c-([A-Z0-9]+):/.exec(name)?.[1];
        const ids = children
            .map((child) => /^[0-9A-F]{4}:([0-9A-F]{4}):([0-9A-F]{4})\.[0-9A-F]+$/i.exec(child))
            .filter(Boolean);

        if (Object.hasOwn(PANEL, controller ?? '') && ids.length === 1)
            found.push({controller, id: `${ids[0][1]}:${ids[0][2]}`.toLowerCase()});
    }
    return found;
}

/**
 * What to set for each controller whose panel the compositor knows:
 * [{id, output}], `output` being [vendor, product, serial, connector] of the
 * panel. A panel that is not connected (the keyboard lies on it) has no entry
 * yet.
 */
export function assignments(controllers, state, upper, lower) {
    const connector = {upper, lower};

    return controllers
        .map(({controller, id}) => [id, connector[PANEL[controller]]])
        .filter(([, panel]) => state.identities.has(panel))
        .map(([id, panel]) => ({id, output: [...state.identities.get(panel), panel]}));
}
