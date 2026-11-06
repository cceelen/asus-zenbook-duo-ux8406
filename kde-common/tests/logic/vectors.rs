//! The cases of `tests/vectors/`, which the JavaScript of the GNOME
//! Shell extensions must satisfy too.

use std::collections::BTreeMap;
use std::fs;

use asus_ux8406_kde_common::geometry::{
    Area, Monitor, Size, State, is_rest, is_valid, shifted, size_of, to_origin,
};
use asus_ux8406_kde_common::layout::{self, Toggle};
use asus_ux8406_kde_common::rotation::{self, Direction, Orientation};
use asus_ux8406_kde_common::touch::{self, Assignment, Controller};
use rstest::rstest;
use serde_json::Value;

fn int(value: &Value) -> i32 {
    i32::try_from(value.as_i64().expect("an integer")).expect("an i32")
}

fn text(value: &Value) -> &str {
    value.as_str().expect("a string")
}

fn texts(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("a list")
        .iter()
        .map(|item| text(item).to_owned())
        .collect()
}

fn size(value: &Value) -> Option<Size> {
    (!value.is_null()).then(|| Size {
        width: int(&value["width"]),
        height: int(&value["height"]),
    })
}

fn monitor(value: &Value) -> Monitor {
    Monitor {
        x: int(&value["x"]),
        y: int(&value["y"]),
        scale: value["scale"].as_f64().expect("a number"),
        transform: u32::try_from(int(&value["transform"])).expect("a transform"),
        primary: value["primary"].as_bool().expect("a bool"),
        connectors: texts(&value["connectors"]),
    }
}

fn layout(value: &Value) -> Vec<Monitor> {
    value
        .as_array()
        .expect("a layout")
        .iter()
        .map(monitor)
        .collect()
}

fn optional_layout(value: &Value) -> Option<Vec<Monitor>> {
    (!value.is_null()).then(|| layout(value))
}

fn state(value: &Value) -> State {
    State {
        logical_mode: value["logicalMode"].as_bool().expect("a bool"),
        connected: value["connected"]
            .as_object()
            .expect("an object")
            .iter()
            .map(|(connector, mode)| (connector.clone(), size(mode)))
            .collect::<BTreeMap<_, _>>(),
        builtin: texts(&value["builtin"]),
        logical: layout(&value["logical"]),
    }
}

fn area(value: &Value) -> Area {
    Area {
        x: int(&value["x"]),
        y: int(&value["y"]),
        width: int(&value["width"]),
        height: int(&value["height"]),
    }
}

/// The held orientations of a case: an object of connector and orientation.
fn held(value: &Value) -> BTreeMap<String, Orientation> {
    value
        .as_object()
        .expect("an object")
        .iter()
        .map(|(connector, name)| (connector.clone(), Orientation::from_sensor(text(name))))
        .collect()
}

fn orientation(value: &Value) -> Option<Orientation> {
    (!value.is_null()).then(|| Orientation::from_sensor(text(value)))
}

/// Run one case of the layout file.
fn run_layout(entry: &Value) {
    let (call, expect) = (text(&entry["call"]), &entry["expect"]);
    let given = || state(&entry["state"]);

    match call {
        "without" => assert_eq!(
            layout::without(&given(), text(&entry["connector"]), text(&entry["heir"])),
            optional_layout(expect)
        ),
        "withBelow" => assert_eq!(
            layout::with_below(&given(), text(&entry["connector"]), text(&entry["anchor"])),
            optional_layout(expect)
        ),
        "toggle" => {
            let saved = optional_layout(&entry["saved"]);
            let result = layout::toggle(
                &given(),
                saved.as_deref(),
                text(&entry["lower"]),
                text(&entry["upper"]),
            );

            assert_eq!(
                result,
                Toggle {
                    apply: optional_layout(&expect["apply"]),
                    save: optional_layout(&expect["save"]),
                }
            );
        }
        "isValid" => assert_eq!(
            is_valid(&given(), &layout(&entry["layout"])),
            expect.as_bool().expect("a bool")
        ),
        "toOrigin" => assert_eq!(to_origin(&layout(&entry["layout"])), layout(expect)),
        "shifted" => {
            let direction = entry["direction"].as_array().expect("a direction");

            assert_eq!(
                shifted(
                    &given(),
                    &layout(&entry["layout"]),
                    area(&entry["area"]),
                    (int(&direction[0]), int(&direction[1])),
                    int(&entry["steps"]),
                ),
                layout(expect)
            );
        }
        "isRest" => assert_eq!(
            is_rest(
                &layout(&entry["layout"]),
                &layout(&entry["saved"]),
                entry["except"].as_str(),
            ),
            expect.as_bool().expect("a bool")
        ),
        "sizeOf" => assert_eq!(
            size_of(
                &monitor(&entry["monitor"]),
                size(&entry["mode"]).expect("a mode"),
                entry["logicalMode"].as_bool().expect("a bool"),
            ),
            size(expect).expect("a size")
        ),
        other => panic!("unknown call {other}"),
    }
}

/// Run one case of the rotation file.
fn run_rotation(entry: &Value) {
    let (call, expect) = (text(&entry["call"]), &entry["expect"]);
    let given = || state(&entry["state"]);
    let sensor = || Orientation::from_sensor(text(&entry["orientation"]));
    let upright = optional_layout(&entry["upright"]);
    // The order of the panels from the top of the laptop, if the case gives it.
    let top = entry["top"].as_array().map(|_| texts(&entry["top"]));

    match call {
        "rotate" => assert_eq!(
            rotation::rotate(&given(), sensor(), upright.as_deref(), top.as_deref()),
            optional_layout(expect)
        ),
        "turnMany" => assert_eq!(
            rotation::turn_many(&given(), &held(&entry["held"])),
            optional_layout(expect)
        ),
        "turnOne" => assert_eq!(
            rotation::turn_one(
                &given(),
                text(&entry["connector"]),
                Orientation::from_sensor(text(&entry["orientation"])),
            ),
            optional_layout(expect)
        ),
        "uprightAfter" => assert_eq!(
            rotation::upright_after(&given(), sensor(), upright.as_deref()),
            optional_layout(expect)
        ),
        "buttonsFrom" => assert_eq!(
            rotation::buttons_from(orientation(&entry["orientation"])).as_str(),
            text(expect)
        ),
        "orientationOf" => assert_eq!(rotation::orientation_of(&given()), orientation(expect)),
        "turned" => assert_eq!(
            rotation::turned(
                sensor(),
                Direction::parse(text(&entry["direction"])).expect("a direction"),
            ),
            orientation(expect)
        ),
        "control" => assert_eq!(
            rotation::control(
                entry["managed"].as_bool().expect("a bool"),
                entry["sensor"].as_bool().expect("a bool"),
                entry["builtin"].as_bool().expect("a bool"),
            )
            .as_str(),
            text(expect)
        ),
        other => panic!("unknown call {other}"),
    }
}

/// Run one case of the touch file.
fn run_touch(entry: &Value) {
    let (call, expect) = (text(&entry["call"]), &entry["expect"]);
    let devices: Vec<(String, Vec<String>)> = entry["devices"]
        .as_array()
        .expect("devices")
        .iter()
        .map(|device| (text(&device[0]).to_owned(), texts(&device[1])))
        .collect();
    let found = touch::controllers_of(&devices);

    match call {
        "controllersOf" => {
            let wanted: Vec<Controller> = expect
                .as_array()
                .expect("controllers")
                .iter()
                .map(|item| Controller {
                    controller: text(&item["controller"]).to_owned(),
                    id: text(&item["id"]).to_owned(),
                })
                .collect();

            assert_eq!(found, wanted);
        }
        "assignments" => {
            let wanted: Vec<Assignment> = expect
                .as_array()
                .expect("assignments")
                .iter()
                .map(|item| Assignment {
                    id: text(&item["id"]).to_owned(),
                    connector: text(&item["connector"]).to_owned(),
                })
                .collect();

            assert_eq!(
                touch::assignments(
                    &found,
                    &state(&entry["state"]),
                    text(&entry["upper"]),
                    text(&entry["lower"]),
                ),
                wanted
            );
        }
        other => panic!("unknown call {other}"),
    }
}

/// Every case of a file gives the expected result. The failure names the case.
#[rstest]
#[case::layout("layout", run_layout)]
#[case::rotation("rotation", run_rotation)]
#[case::touch("touch", run_touch)]
fn every_case_of_the_file_gives_the_expected_result(#[case] file: &str, #[case] run: fn(&Value)) {
    let path = format!("{}/tests/vectors/{file}.json", env!("CARGO_MANIFEST_DIR"));
    let cases: Value = serde_json::from_str(&fs::read_to_string(&path).expect("the file"))
        .expect("the file is JSON");
    let cases = cases.as_array().expect("a list of cases");

    assert!(!cases.is_empty(), "{file} has no case");
    for entry in cases {
        let name = text(&entry["name"]);

        if let Err(cause) = std::panic::catch_unwind(|| run(entry)) {
            let reason = cause
                .downcast_ref::<String>()
                .map_or("a panic", String::as_str);

            panic!("{file}: {name}: {reason}");
        }
    }
}
