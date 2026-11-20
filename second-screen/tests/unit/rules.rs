use asus_ux8406_second_screen::Error;

const RULES: &str = include_str!("../../udev/90-asus-ux8406-second-screen.rules.in");

#[test]
fn the_rule_names_the_models_of_the_program() {
    let gate = RULES
        .lines()
        .find_map(|line| line.strip_prefix("ATTR{[dmi/id]product_name}!=\""))
        .and_then(|rest| rest.split_once('"'))
        .map(|(patterns, _)| patterns)
        .expect("the model gate of the rule");
    let named: Vec<&str> = gate
        .split('|')
        .map(|pattern| pattern.trim_matches('*'))
        .collect();
    let message = Error::UnsupportedModel.to_string();
    let known: Vec<&str> = message
        .strip_prefix("this is not a Zenbook Duo ")
        .and_then(|rest| rest.strip_suffix("; nothing done"))
        .expect("the models in the message")
        .split(" or ")
        .collect();

    assert_eq!(named, known);
}
