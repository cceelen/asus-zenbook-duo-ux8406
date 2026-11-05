"""Checks of the shortcuts file for kglobalacceld, without a KDE session.

Each action named in Actions= has its section, a program to run and a key;
each key is one that the keyboard program sends: the emoji key is KEY_PROG1,
which xkeyboard-config makes XF86Launch1 and Qt calls "Launch (1)".
"""

import configparser
import pathlib
import sys
import unittest

FILE = (
    pathlib.Path(__file__).parent.parent / "asus-zenbook-duo-ux8406-shortcuts.desktop"
)
# The keys of the keyboard program with their Qt names.
KEYS = {"Launch (1)": "F11, the emoji key (KEY_PROG1)"}


def read():
    parser = configparser.ConfigParser(interpolation=None, strict=True)
    parser.optionxform = str
    parser.read(FILE, encoding="utf-8")
    return parser


class Shortcuts(unittest.TestCase):
    def test_entry_is_a_service_that_kglobalacceld_reads(self):
        entry = read()["Desktop Entry"]
        self.assertEqual(entry["Type"], "Application")
        self.assertEqual(entry["X-KDE-GlobalShortcutType"], "Service")
        self.assertNotEqual(
            entry.get("NoDisplay", "false").lower(),
            "true",
            "kglobalacceld skips a file in kglobalaccel/ with NoDisplay=true",
        )
        self.assertTrue(
            entry.get("Exec", "").split(),
            "kglobalacceld skips a file that is not a valid application: "
            "the entry needs an Exec",
        )

    def test_each_action_has_a_program_and_a_known_key(self):
        parser = read()
        actions = [a for a in parser["Desktop Entry"]["Actions"].split(";") if a]
        self.assertTrue(actions)
        for action in actions:
            section = parser[f"Desktop Action {action}"]
            self.assertTrue(section["Name"])
            self.assertTrue(section["Exec"].split()[0])
            for key in section["X-KDE-Shortcuts"].split(","):
                self.assertIn(key, KEYS)

    def test_no_section_without_an_action(self):
        parser = read()
        actions = set(parser["Desktop Entry"]["Actions"].split(";")) - {""}
        for section in parser.sections():
            if section != "Desktop Entry":
                self.assertIn(section.removeprefix("Desktop Action "), actions)


if __name__ == "__main__":
    sys.exit(unittest.main())
