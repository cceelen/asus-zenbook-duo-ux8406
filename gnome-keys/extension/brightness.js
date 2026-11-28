// The brightness the lower panel gets from the upper panel. No GNOME imports
// here, so that it can be tested with plain data (brightness.test.js).

/** Whether a backlight is a number with a range of one value or more. */
function valid(backlight) {
    return (
        Number.isFinite(backlight?.value) &&
        Number.isFinite(backlight.min) &&
        Number.isFinite(backlight.max) &&
        backlight.min <= backlight.max
    );
}

/**
 * The brightness to set on the lower panel, or null when nothing is to be set.
 * `upper` and `lower` are {value, min, max}, or null for a panel or a
 * backlight that is not there. The value of the upper panel keeps its place in
 * its range, rounded and held inside the range of the lower panel.
 */
export function lowerBrightness(upper, lower) {
    if (!valid(upper) || !valid(lower)) return null;

    const span = upper.max - upper.min;
    const share = span === 0 ? 0 : (upper.value - upper.min) / span;
    const scaled = Math.round(lower.min + share * (lower.max - lower.min));
    const target = Math.min(lower.max, Math.max(lower.min, scaled));

    return target === lower.value ? null : target;
}
