//! The functions that read the text of other programs.

use asus_ux8406_kde_live_test::activate::exec_words;
use asus_ux8406_kde_live_test::auto::{active_keys, strip_ansi, tablet_switches, uuids};
use asus_ux8406_kde_live_test::deps::{parse_provides, parse_requires};
use asus_ux8406_kde_live_test::windows::place_script;

#[test]
fn requirements_are_sorted_into_names_and_conditions() {
    let requires = parse_requires(
        "iio-sensor-proxy\nkwin >= 6.6\nrpmlib(PayloadIsZstd) <= 5.4.18-1\n\n/bin/sh\n(foo if bar)\nlibc.so.6()(64bit)\nkwin\n",
    );

    assert_eq!(
        requires.names.into_iter().collect::<Vec<_>>(),
        ["/bin/sh", "iio-sensor-proxy", "kwin", "libc.so.6()(64bit)"]
    );
    assert_eq!(requires.conditional, ["(foo if bar)"]);
}

#[test]
fn provides_are_names() {
    let provides = parse_provides("config(x) = 1\nplasma-builtin-screen-rotation = 0.5.0-1\n\n");

    assert!(provides.contains("config(x)") && provides.contains("plasma-builtin-screen-rotation"));
}

#[test]
fn an_exec_line_is_split_without_field_codes() {
    assert_eq!(exec_words("/usr/bin/x run"), ["/usr/bin/x", "run"]);
    assert_eq!(
        exec_words("  /usr/bin/x   \"a b\" %U \"\" end\t%f"),
        ["/usr/bin/x", "a b", "", "end"]
    );
    assert_eq!(exec_words(""), Vec::<String>::new());
    assert_eq!(
        exec_words("x \"a\tb c\" d"),
        ["x", "a\tb c", "d"],
        "blanks inside quotes stay"
    );
}

#[test]
fn colour_codes_go() {
    assert_eq!(strip_ansi("\u{1b}[01;32mOutput:\u{1b}[0m 1"), "Output: 1");
    assert_eq!(strip_ansi("plain"), "plain");
}

#[test]
fn outputs_have_uuids() {
    let map = uuids(
        "\u{1b}[0mOutput: 1 eDP-1 aaa-1\n\tenabled\nOutput: 2 eDP-2 bbb-2 extra\nOutput: 3\n",
    );

    assert_eq!(map.get("eDP-1").map(String::as_str), Some("aaa-1"));
    assert_eq!(map.get("eDP-2").map(String::as_str), Some("bbb-2"));
    assert_eq!(map.len(), 2);
}

#[test]
fn the_active_key_is_the_first_field() {
    let keys = active_keys(
        "[kwin]\nA B=Launch (9),Launch (9),Switch\nSwap=F19,none,Swap\nignored line\n[services][x.desktop]\nEmoji=Launch (1)\n",
    );

    assert_eq!(keys[&("[kwin]".to_owned(), "A B".to_owned())], "Launch (9)");
    assert_eq!(keys[&("[kwin]".to_owned(), "Swap".to_owned())], "F19");
    assert_eq!(
        keys[&("[services][x.desktop]".to_owned(), "Emoji".to_owned())],
        "Launch (1)"
    );
}

#[test]
fn the_tablet_mode_switch_is_bit_one() {
    let devices = "I: Bus=0019\nN: Name=\"Lid Switch\"\nB: SW=1\n\nI: Bus=0019\nN: Name=\"Intel HID switches\"\nB: SW=2\n\nI: Bus=0011\nN: Name=\"Keyboard\"\nB: EV=120013\n\nN: Name=\"Odd\"\nB: SW=zz\n";

    assert_eq!(tablet_switches(devices), ["Intel HID switches"]);
    assert_eq!(tablet_switches(""), Vec::<String>::new());
}

#[test]
fn the_place_script_names_process_and_output() {
    let script = place_script(1234, "eDP-2");

    assert!(script.contains("'eDP-2'") && script.contains("=== 1234"));
}

#[test]
fn only_a_new_global_is_announced() {
    use asus_ux8406_kde_live_test::outputs::announced;
    use wayland_client::protocol::wl_registry::Event;

    assert_eq!(
        announced(Event::Global {
            name: 1,
            interface: "wl_compositor".to_owned(),
            version: 6
        }),
        Some(("wl_compositor".to_owned(), 6))
    );
    assert_eq!(announced(Event::GlobalRemove { name: 1 }), None);
}
