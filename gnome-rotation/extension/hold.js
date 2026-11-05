// The hold of the manual turns: what the extension remembers while panels are
// turned by hand, and what it does when something happens. No GNOME imports
// here, so that it can be tested with plain data (hold.test.js).
//
// A turn by hand puts the screens on hold: the sensor is not followed, so that
// it cannot undo the turn, and each panel has the orientation that it was
// turned to. The hold is for the session. The setting orientation-lock is not
// part of it: `lock` actions come only from the switch of the user.
//
// The input is an event, the output a list of actions for the extension:
//
// - `{type: 'turn', screens, connector, orientation}`: a panel is to be turned
//   by hand. `screens` is the list of `screens()` of rotation.js.
// - `{type: 'refused'}`: the compositor did not take that turn.
// - `{type: 'all'}`: all panels were turned together (the buttons): there is
//   no hold on one by one any more.
// - `{type: 'switch', locked}`: the user used the switch; `locked` is the
//   setting orientation-lock.
// - `{type: 'monitors', screens}`: the monitors changed.
// - `{type: 'dropped', connector}`: the compositor did not take the held
//   orientation of that panel again.
// - `{type: 'sensor'}`: the sensor gave an orientation.
// - `{type: 'followed'}`: the panels were brought to the orientation of the
//   sensor, or were there already.
//
// The actions: `{do: 'lock', value}` sets orientation-lock; `{do: 'turn',
// connector, orientation}` turns a panel to its held orientation; `{do:
// 'follow'}` brings the panels to the orientation that is followed.

export class Hold {
    // The orientation of each panel that was turned by hand, by connector. A
    // panel that is gone keeps its place until it comes back.
    #held = new Map();
    // The same before the last turn, to put back if it is refused.
    #before = new Map();
    // The connectors from the top of the laptop to its bottom: the order of
    // the compositor's list of built-in panels. It stays until the panels
    // follow the sensor again, which needs it for the panels that are turned
    // differently.
    #top = null;

    /** Whether panels are on hold: the sensor is not followed. */
    get active() {
        return this.#held.size > 0;
    }

    /** The order of the panels, or null when it is not needed. */
    get top() {
        return this.#top;
    }

    /** The held orientations as an object, for `turnMany`. */
    get held() {
        return Object.fromEntries(this.#held);
    }

    /** Whether the switch shows on: not locked and not on hold. */
    isAuto(locked) {
        return !locked && !this.active;
    }

    /** The actions to take for an event. */
    handle(event) {
        switch (event.type) {
            case 'turn':
                return this.#turn(event);
            case 'refused':
                this.#held = new Map(this.#before);
                if (!this.active) this.#top = null;
                return [];
            case 'all':
                this.#held.clear();
                return [];
            case 'switch':
                return this.#switch(event.locked);
            case 'monitors':
                return this.#monitors(event.screens);
            case 'dropped':
                this.#held.delete(event.connector);
                return this.active ? [] : [{do: 'follow'}];
            case 'sensor':
                return this.active ? [] : [{do: 'follow'}];
            case 'followed':
                if (!this.active) this.#top = null;
                return [];
            default:
                throw new Error(`unknown event ${event.type}`);
        }
    }

    #turn({screens, connector, orientation}) {
        this.#before = new Map(this.#held);
        if (!this.active) {
            // The other panels keep the orientation that they have now.
            this.#held = new Map(
                screens
                    .filter((screen) => screen.orientation !== null)
                    .map((screen) => [screen.connector, screen.orientation]),
            );
            this.#top = screens.map((screen) => screen.connector);
        }
        this.#held.set(connector, orientation);
        return [];
    }

    #switch(locked) {
        if (this.isAuto(locked)) return [{do: 'lock', value: true}];
        // On again: the panels go back under the sensor.
        this.#held.clear();
        return [{do: 'lock', value: false}];
    }

    #monitors(screens) {
        if (!this.active) return [{do: 'follow'}];
        // The compositor changed the layout: each panel that is there and has
        // another orientation than the held one is turned again. A mirrored
        // panel (no orientation) is the user's own setting.
        return screens
            .filter(
                (screen) =>
                    this.#held.has(screen.connector) &&
                    screen.orientation !== null &&
                    screen.orientation !== this.#held.get(screen.connector),
            )
            .map((screen) => ({
                do: 'turn',
                connector: screen.connector,
                orientation: this.#held.get(screen.connector),
            }));
    }
}
