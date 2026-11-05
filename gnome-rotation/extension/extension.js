// The layout of the built-in panels when the laptop is turned on its side.
// Made for the two panels of the ASUS Zenbook Duo UX8406; nothing in it is
// specific to that model.
//
// With the built-in panels on and the laptop on its left or right side, the
// panels are turned and put side by side; upright, the layout from before is
// put back. GNOME does this by itself only without a pointer device, and for
// one panel. The layouts are for the session only; monitors.xml is not
// touched. The switch "Auto-rotate" in the quick settings is GNOME's own
// setting orientation-lock. Its menu turns each panel by hand, to any of the
// four orientations; that switches Auto-rotate off for the session, and the
// setting is not changed. Where no sensor gives the orientation, two buttons
// turn all panels by 90 degrees instead.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';

import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {QuickMenuToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';

import {applyLayout, currentState} from './display.js';
import {Hold} from './hold.js';
import {
    buttonsFrom,
    control,
    orientationOf,
    rotate,
    screens,
    turned,
    turnOne,
    uprightAfter,
} from './rotation.js';
import {OrientationPresence, OrientationSensor} from './sensor.js';

// GNOME's own setting for the rotation of the built-in panel.
const TOUCHSCREEN = 'org.gnome.settings-daemon.peripherals.touchscreen';
const LOCK = 'orientation-lock';
// The laptop must hold an orientation this long before the panels follow.
const SETTLE_MS = 700;

// The four orientations of a screen in its menu: the orientation, the icon,
// the name. The arrow points to the side of the screen that is up.
const CHOICES = [
    ['normal', 'go-up-symbolic', 'Upright'],
    ['left-up', 'go-previous-symbolic', 'Left side up'],
    ['right-up', 'go-next-symbolic', 'Right side up'],
    ['bottom-up', 'go-down-symbolic', 'Upside down'],
];
const TOOLTIP_MS = 500;

/**
 * A tooltip for a button: the Shell has none for its own buttons. It shows
 * below the button after TOOLTIP_MS of hover, and goes with the hover, the
 * menu or the button.
 */
function addTooltip(button, text) {
    let label = null;
    let waiting = 0;
    const hide = () => {
        if (waiting) GLib.source_remove(waiting);
        waiting = 0;
        label?.destroy();
        label = null;
    };

    button.connect('notify::hover', () => {
        hide();
        if (!button.hover) return;
        waiting = GLib.timeout_add(GLib.PRIORITY_DEFAULT, TOOLTIP_MS, () => {
            waiting = 0;
            label = new St.Label({text, style_class: 'dash-label'});
            Main.layoutManager.uiGroup.add_child(label);

            const [x, y] = button.get_transformed_position();

            label.set_position(
                Math.max(0, Math.round(x + (button.width - label.width) / 2)),
                Math.round(y + button.height + 4),
            );
            return GLib.SOURCE_REMOVE;
        });
    });
    button.connect('notify::mapped', () => {
        if (!button.mapped) hide();
    });
    button.connect('destroy', hide);
}

/**
 * The quick settings item for the rotation: the switch "Auto-rotate", and
 * the menu with a section for each built-in screen. Hidden until the
 * extension knows that it can turn a panel (_showControl).
 */
function rotationToggle() {
    const toggle = new QuickMenuToggle({
        title: 'Auto-rotate',
        iconName: 'rotation-allowed-symbolic',
        toggleMode: true,
        visible: false,
    });

    toggle.menu.setHeader('rotation-allowed-symbolic', 'Rotate screens');
    return toggle;
}

/**
 * The sections of the menu: for each screen, its name and four buttons.
 * `turn(connector, orientation)` is called for a button. `update(list)` shows
 * the screens of `screens()` and marks the orientation of each.
 */
function screenSections(menu, turn) {
    const section = new PopupMenu.PopupMenuSection();
    let shown = '';
    let buttons = new Map();

    menu.addMenuItem(section);

    const build = ({connector, name}) => {
        const box = new St.BoxLayout({x_expand: true, style: 'spacing: 6px;'});
        const item = new PopupMenu.PopupBaseMenuItem({activate: false, can_focus: false});

        for (const [orientation, icon, label] of CHOICES) {
            const button = new St.Button({
                style_class: 'icon-button',
                x_expand: true,
                can_focus: true,
                accessible_name: `${name}: ${label}`,
                child: new St.Icon({icon_name: icon}),
            });

            button.connect('clicked', () => turn(connector, orientation));
            addTooltip(button, label);
            box.add_child(button);
            buttons.set(`${connector} ${orientation}`, button);
        }
        item.add_child(box);
        section.addMenuItem(new PopupMenu.PopupSeparatorMenuItem(name));
        section.addMenuItem(item);
    };

    return {
        section,
        update(list) {
            const key = JSON.stringify(list.map(({connector, name}) => [connector, name]));

            if (key !== shown) {
                shown = key;
                section.removeAll();
                buttons = new Map();
                for (const screen of list) build(screen);
            }
            for (const {connector, orientation} of list)
                for (const [choice] of CHOICES)
                    buttons.get(`${connector} ${choice}`).checked = choice === orientation;
        },
    };
}

/**
 * Two buttons in the place of one switch: each turns the panels by 90
 * degrees. `turn(direction)` is called with 'counterclockwise' or
 * 'clockwise'. Hidden, as the switch.
 */
function rotationButtons(turn) {
    const box = new St.BoxLayout({x_expand: true, style: 'spacing: 6px;', visible: false});
    const buttons = {};

    for (const [direction, icon, name] of [
        ['counterclockwise', 'object-rotate-left-symbolic', 'Turn counterclockwise'],
        ['clockwise', 'object-rotate-right-symbolic', 'Turn clockwise'],
    ]) {
        const button = new St.Button({
            style_class: 'quick-toggle',
            x_expand: true,
            can_focus: true,
            accessible_name: name,
            child: new St.Icon({icon_name: icon, style_class: 'quick-toggle-icon'}),
        });

        button.connect('clicked', () => turn(direction));
        box.add_child(button);
        buttons[direction] = button;
    }
    return {box, buttons};
}

export default class BuiltinScreenRotation extends Extension {
    // The layout from before the panels were turned. It stays when the
    // extension is disabled: the Shell does that at each screen lock, and the
    // laptop can be upright again only after it.
    _upright = null;
    // The panels that were turned by hand (hold.js): the sensor is not
    // followed until the switch is turned on again. Like `_upright`, it stays
    // at a screen lock; a new login starts with none. The setting
    // orientation-lock is not touched by it.
    _hold = new Hold();
    // The turns of held panels that wait their turn, one at a time.
    _queue = Promise.resolve();

    enable() {
        this._settleId = 0;

        const schema = Gio.SettingsSchemaSource.get_default().lookup(TOUCHSCREEN, true);

        if (!schema?.has_key(LOCK)) return;

        // GNOME's own schema: the extension ships none, it has no settings.
        this._settings = Gio.Settings.new_full(schema, null, null);
        this._lockChanged = this._settings.connect(`changed::${LOCK}`, () => {
            this._watch();
            this._showSwitch();
        });
        this._indicator = new SystemIndicator();
        this._switch = rotationToggle();
        this._clickedId = this._switch.connect('clicked', () => this._clicked());
        this._screens = screenSections(this._switch.menu, (connector, orientation) =>
            this._turnOne(connector, orientation).catch((error) => this._logError(error)),
        );
        ({box: this._box, buttons: this._buttons} = rotationButtons((direction) =>
            this._turn(direction).catch((error) => this._logError(error)),
        ));
        this._indicator.quickSettingsItems.push(this._switch, this._box);
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        // What decides which control is shown. The sensor is followed without
        // a claim, also while the rotation is locked.
        this._builtin = false;
        this._control = 'none';
        // The orientation that the buttons turned the panels to.
        this._manual = null;
        this._monitorManager = global.backend.get_monitor_manager();
        this._managedChanged = this._monitorManager.connect(
            'notify::panel-orientation-managed',
            () => this._showControl(),
        );
        this._presence = new OrientationPresence(() => this._showControl());
        // A monitor came or went, as when the keyboard is put on the lower
        // panel: the layout that the compositor then takes is upright.
        this._monitorsChanged = Main.layoutManager.connect('monitors-changed', () =>
            this._findBuiltin(),
        );
        this._findBuiltin();
        this._watch();
    }

    disable() {
        if (this._settleId) GLib.source_remove(this._settleId);
        this._settleId = 0;
        if (this._monitorsChanged) Main.layoutManager.disconnect(this._monitorsChanged);
        this._monitorsChanged = 0;
        this._sensor?.destroy();
        this._sensor = null;
        this._presence?.destroy();
        this._presence = null;
        if (this._managedChanged) this._monitorManager.disconnect(this._managedChanged);
        this._managedChanged = 0;
        this._monitorManager = null;
        if (this._clickedId) this._switch.disconnect(this._clickedId);
        this._clickedId = 0;
        for (const item of this._indicator?.quickSettingsItems ?? []) item.destroy();
        this._indicator?.destroy();
        this._indicator = null;
        // A turn that still waits finds no indicator and is dropped.
        this._queue = Promise.resolve();
        this._switch = null;
        this._screens = null;
        this._box = null;
        this._buttons = null;
        if (this._lockChanged) this._settings.disconnect(this._lockChanged);
        this._lockChanged = 0;
        this._settings = null;
    }

    /**
     * Whether the compositor has a built-in panel, then the control. The
     * buttons start from the orientation that the panels have, and the menu
     * shows the orientation of each panel. The hold decides what a change of
     * the monitors means: the panels follow the sensor, or the panels that
     * are held are turned again.
     */
    _findBuiltin() {
        const indicator = this._indicator;

        if (!indicator) return;
        currentState()
            .then((state) => {
                // Disabled while the compositor gave its answer.
                if (indicator !== this._indicator) return;
                this._builtin = state.builtin.length > 0;
                this._manual ??= orientationOf(state);

                const list = screens(state);

                this._screens.update(list);
                this._switch.menuEnabled = list.length > 0;
                this._showControl();
                this._act(this._hold.handle({type: 'monitors', screens: list}));
            })
            .catch((error) => this._logError(error));
    }

    /** Do what the hold asks. */
    _act(actions) {
        for (const action of actions) {
            if (action.do === 'lock') {
                this._settings.set_boolean(LOCK, action.value);
            } else if (action.do === 'turn') {
                // One at a time: each turn starts from the state that the one
                // before made.
                // A turn that waits when the extension is disabled is
                // dropped: it belongs to the indicator of its time.
                const indicator = this._indicator;

                this._queue = this._queue
                    .then(() => this._turnAgain(action.connector, action.orientation, indicator))
                    .catch((error) => this._logError(error));
            } else {
                this._settle();
            }
        }
    }

    _showControl() {
        this._control = control({
            managed: this._monitorManager.panel_orientation_managed,
            sensor: this._presence.present,
            builtin: this._builtin,
        });
        // The menu with the switch where the sensor gives the orientation;
        // without it, the menu alone, next to the two buttons.
        this._switch.visible = this._control !== 'none';
        this._switch.toggleMode = this._control === 'switch';
        this._switch.title = this._control === 'switch' ? 'Auto-rotate' : 'Rotate screens';
        this._showSwitch();
        this._box.visible = this._control === 'buttons';
        // A button that would turn past left side up or right side up does
        // nothing.
        for (const [direction, button] of Object.entries(this._buttons))
            button.reactive = turned(buttonsFrom(this._manual), direction) !== null;
    }

    /**
     * The switch is on while the rotation is not locked and no panel was
     * turned by hand. A turn by hand does not change the setting: the switch
     * only shows it.
     */
    _showSwitch() {
        const auto = this._hold.isAuto(this._settings.get_boolean(LOCK));

        this._switch.checked = this._control === 'switch' && auto;
        this._switch.subtitle =
            this._control === 'switch' && this._hold.active ? 'Turned by hand' : null;
    }

    /** The switch or the button of the item was clicked. */
    _clicked() {
        // Without a sensor the item is no switch: it opens the menu.
        if (this._control !== 'switch') {
            this._switch.menu.open();
            return;
        }

        // Off, or on again: all panels go back under the sensor, which tells
        // the orientation of the laptop as soon as it is there again.
        this._act(this._hold.handle({type: 'switch', locked: this._settings.get_boolean(LOCK)}));
        this._watch();
        this._showSwitch();
    }

    /** Turn the panels by 90 degrees from the orientation of the buttons. */
    async _turn(direction) {
        const orientation = turned(buttonsFrom(this._manual), direction);

        if (!orientation) return;
        // All panels together: none is held one by one any more.
        this._hold.handle({type: 'all'});
        this._watch();
        this._manual = orientation;
        this._showControl();
        await this._rotate();
    }

    /**
     * Turn one panel to an orientation, from its menu. The panels are on hold
     * from then on: the sensor is not followed until the switch is on again,
     * it would undo the turn.
     */
    async _turnOne(connector, orientation) {
        const indicator = this._indicator;

        if (!indicator) return;

        const state = await currentState();

        if (indicator !== this._indicator) return;

        const layout = turnOne(state, connector, orientation);

        if (!layout) return;
        this._hold.handle({type: 'turn', screens: screens(state), connector, orientation});
        this._holdChanged();
        try {
            await applyLayout(state, layout);
        } catch (error) {
            // The compositor did not take the layout: the panels are as they
            // were, and so is the hold.
            if (indicator === this._indicator) {
                this._hold.handle({type: 'refused'});
                this._holdChanged();
            }
            throw error;
        }
        if (indicator === this._indicator) this._applied(state, layout);
    }

    /**
     * A held panel is turned again after the compositor changed the layout. If
     * the compositor does not take it, the panel is not held any more.
     */
    async _turnAgain(connector, orientation, indicator) {
        if (!indicator || indicator !== this._indicator) return;

        const state = await currentState();

        if (indicator !== this._indicator) return;

        const layout = turnOne(state, connector, orientation);

        if (!layout) return;
        try {
            await applyLayout(state, layout);
        } catch (error) {
            if (indicator !== this._indicator) return;
            this._logError(error, `${connector} is not held any more: `);
            this._act(this._hold.handle({type: 'dropped', connector}));
            this._holdChanged();
            return;
        }
        if (indicator === this._indicator) this._applied(state, layout);
    }

    /** A layout of a held panel was taken. */
    _applied(state, layout) {
        const now = {...state, logical: layout};

        this._upright = uprightAfter(state, orientationOf(now), this._upright);
        // The buttons go on from the orientation of the panels, if they have one.
        this._manual = orientationOf(now);
        this._findBuiltin();
    }

    /** The hold changed: the sensor and the switch follow it. */
    _holdChanged() {
        if (this._hold.active && this._settleId) {
            GLib.source_remove(this._settleId);
            this._settleId = 0;
        }
        this._watch();
        this._showSwitch();
    }

    /**
     * Have the sensor only while the rotation is not locked and no panel was
     * turned by hand: the service reads the accelerometer as long as a
     * program has a claim on it. A new sensor tells its orientation, and the
     * panels then follow it.
     */
    _watch() {
        if (this._settings.get_boolean(LOCK) || this._hold.active) {
            this._sensor?.destroy();
            this._sensor = null;
        } else if (!this._sensor) {
            this._sensor = new OrientationSensor(() =>
                this._act(this._hold.handle({type: 'sensor'})),
            );
        }
    }

    /** Write an error to the journal, with the name of the extension. */
    _logError(error, context = '') {
        console.error(`${this.uuid}: ${context}${error.message}`);
    }

    /** Follow the orientation once it has held for SETTLE_MS. */
    _settle() {
        if (this._settleId) GLib.source_remove(this._settleId);
        this._settleId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, SETTLE_MS, () => {
            this._settleId = 0;
            this._rotate().catch((error) => this._logError(error));
            return GLib.SOURCE_REMOVE;
        });
    }

    /**
     * The orientation that the panels follow: the one of the buttons where
     * they are shown, else the one of the sensor. Null for none: the
     * rotation is locked, or the extension is disabled.
     */
    _orientation() {
        if (this._control === 'buttons') return this._manual;
        return this._sensor?.orientation ?? null;
    }

    async _rotate() {
        const [indicator, sensor] = [this._indicator, this._sensor];

        if (!this._orientation()) return;

        const state = await currentState();

        // Locked or disabled while the compositor gave its answer: do no more.
        if (indicator !== this._indicator || sensor !== this._sensor) return;

        const orientation = this._orientation();
        const layout = rotate(state, orientation, this._upright, this._hold.top);

        if (!layout) {
            this._hold.handle({type: 'followed'});
            return;
        }
        // The compositor checks a layout before it takes it. One that it
        // does not accept is not applied: the call throws, and the panels
        // stay as they are.
        await applyLayout(state, layout);
        this._hold.handle({type: 'followed'});
        if (indicator === this._indicator && sensor === this._sensor)
            this._upright = uprightAfter(state, orientation, this._upright);
    }
}
