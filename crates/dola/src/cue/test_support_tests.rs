use super::*;

#[test]
fn carrier_cue_builds_canonical_carrier_at_zero() {
    let cue = carrier_cue("1", "move", &["-353", "40"]);
    assert_eq!(cue.at, 0.0);
    assert_eq!(cue.duration, 0.0);
    assert_eq!(cue.actor, ActorKey::from("1"));
    assert_eq!(
        cue.command,
        CueCommand::command_carrier("move", vec!["-353".into(), "40".into()])
    );
}

#[test]
fn carrier_cue_keeps_empty_and_equals_tokens() {
    let cue = carrier_cue("0", "set", &["", "--key=value", "b", ""]);
    assert_eq!(
        cue.command.as_command_carrier(),
        Some(("set", vec!["", "--key=value", "b", ""]))
    );
}
