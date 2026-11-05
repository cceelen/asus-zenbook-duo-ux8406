// The applet "Auto-rotate" in the system tray, for the program
// plasma-builtin-screen-rotation. It has no logic of its own: it shows the
// properties of the program's service on the session bus, and calls its
// methods. The system tray shows it while the service is there.
//
// - A left click on the icon opens the popup: the switch "Auto-rotate" and, for
//   each built-in screen, four buttons that turn it to upright, left side up,
//   right side up or upside down, the present one marked. A turn by hand puts
//   Auto-rotate off for the session (the program holds the screens); the
//   switch on hands all screens back to the sensor.
// - A middle click on the icon switches the rotation on and off: the program
//   flips its own state (method Toggle). So does the global shortcut "Toggle
//   auto-rotate" of the program, which the user sets in System Settings,
//   Shortcuts. The keyboard shortcut of the applet, and the keys Space and Enter
//   on its icon, open the popup.
// - Without a sensor the popup has no switch, but the buttons of each screen
//   and two buttons that turn all screens by 90 degrees.
// - Control "none" (no built-in screen): hidden.
// - Every button can be reached and pressed with the keyboard, and has an
//   accessible name.

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.plasma.workspace.dbus as DBus

PlasmoidItem {
    id: root

    readonly property string service: "io.github.cceelen.BuiltinScreenRotation"
    readonly property string path: "/io/github/cceelen/BuiltinScreenRotation"
    readonly property var status: properties.properties
    readonly property string control: status.Control ?? "none"
    readonly property bool rotating: status.Enabled === true
    // The built-in screens: each is [connector, name, orientation].
    readonly property var screens: Array.isArray(status.Screens) ? status.Screens : []
    readonly property var choices: [
        {"orientation": "normal", "icon": "go-up-symbolic", "label": i18n("Upright")},
        {"orientation": "left-up", "icon": "go-previous-symbolic", "label": i18n("Left side up")},
        {"orientation": "right-up", "icon": "go-next-symbolic", "label": i18n("Right side up")},
        {"orientation": "bottom-up", "icon": "go-down-symbolic", "label": i18n("Upside down")},
    ]

    function call(member: string, signature: string, values: var): void {
        DBus.SessionBus.asyncCall({
            "service": root.service,
            "path": root.path,
            "iface": root.service,
            "member": member,
            "signature": signature,
            "arguments": values,
        });
    }

    function setRotating(on: bool): void {
        call("SetEnabled", "b", [new DBus.bool(on)]);
    }

    function toggle(): void {
        call("Toggle", "", []);
    }

    function turnScreen(connector: string, orientation: string): void {
        call("TurnScreen", "ss", [new DBus.string(connector), new DBus.string(orientation)]);
    }

    function turn(direction: string): void {
        call("Turn", "s", [new DBus.string(direction)]);
    }

    DBus.Properties {
        id: properties
        busType: DBus.BusType.Session
        service: root.service
        path: root.path
        iface: root.service
    }

    // The shortcut of the applet, and the keys of the keyboard on the icon in
    // the system tray (Space, Enter), open and close the popup, as for any
    // applet. The global shortcut "Toggle auto-rotate" is the program's own.
    Plasmoid.onActivated: root.expanded = !root.expanded

    Plasmoid.icon: control === "switch" && !rotating ? "rotation-locked-symbolic" : "rotation-allowed-symbolic"
    Plasmoid.status: control === "none" ? PlasmaCore.Types.HiddenStatus : PlasmaCore.Types.ActiveStatus
    toolTipMainText: i18n("Auto-rotate")
    toolTipSubText: control === "buttons" ? i18n("No sensor: turn the screens by hand")
        : rotating ? i18n("The screens turn with the laptop") : i18n("The screens do not turn with the laptop")

    compactRepresentation: Kirigami.Icon {
        source: Plasmoid.icon
        active: mouse.containsMouse

        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            acceptedButtons: Qt.LeftButton | Qt.MiddleButton
            onClicked: mouse => {
                if (mouse.button === Qt.MiddleButton) {
                    if (root.control === "switch") {
                        root.toggle();
                    }
                } else {
                    root.expanded = !root.expanded;
                }
            }
        }
    }

    fullRepresentation: ColumnLayout {
        PlasmaComponents.Switch {
            visible: root.control === "switch"
            text: i18n("Auto-rotate")
            checked: root.rotating
            onToggled: root.setRotating(checked)
            Accessible.name: i18n("Auto-rotate")
        }

        Repeater {
            model: root.screens

            delegate: ColumnLayout {
                id: section

                required property var modelData
                readonly property string connector: String(modelData[0] ?? "")
                readonly property string screenName: String(modelData[1] ?? "")
                readonly property string current: String(modelData[2] ?? "")

                PlasmaComponents.Label {
                    text: section.screenName
                    font.bold: true
                }

                RowLayout {
                    Repeater {
                        model: root.choices

                        delegate: PlasmaComponents.Button {
                            id: choice

                            required property var modelData
                            readonly property bool present: section.current === choice.modelData.orientation

                            icon.name: choice.modelData.icon
                            display: PlasmaComponents.AbstractButton.IconOnly
                            highlighted: choice.present
                            focusPolicy: Qt.StrongFocus
                            Accessible.name: choice.present
                                ? i18n("%1: %2, present", section.screenName, choice.modelData.label)
                                : i18n("%1: %2", section.screenName, choice.modelData.label)
                            PlasmaComponents.ToolTip.text: choice.modelData.label
                            PlasmaComponents.ToolTip.visible: choice.hovered || choice.activeFocus
                            onClicked: root.turnScreen(section.connector, choice.modelData.orientation)
                        }
                    }
                }
            }
        }

        RowLayout {
            visible: root.control === "buttons"

            PlasmaComponents.Button {
                icon.name: "object-rotate-left-symbolic"
                text: i18n("Turn counterclockwise")
                enabled: root.status.CanTurnCounterclockwise === true
                Accessible.name: i18n("Turn all screens counterclockwise")
                onClicked: root.turn("counterclockwise")
            }

            PlasmaComponents.Button {
                icon.name: "object-rotate-right-symbolic"
                text: i18n("Turn clockwise")
                enabled: root.status.CanTurnClockwise === true
                Accessible.name: i18n("Turn all screens clockwise")
                onClicked: root.turn("clockwise")
            }
        }
    }
}
