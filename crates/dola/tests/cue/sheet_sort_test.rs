//! 台本と予定の並べ替えの比べ方の檻。
//!
//! 台本は開始時刻の昇順に安定ソートされ、同じ瞬間の合図は台本の順のまま予定へ入る。

use dola::cue::{ActorKey, Cue, CueCommand, CuePayload, CueSheet, to_talk_schedule};
use dola::time::{TimePoint, TimeSpan};

fn cue(start_time: TimeSpan, label: &str) -> Cue {
    Cue {
        actor: ActorKey::from("a"),
        start_time,
        payload: CueCommand::Text(label.into()).into(),
        duration: TimeSpan::ZERO,
    }
}

fn label(command: &CueCommand) -> &str {
    match command {
        CueCommand::Text(s) => s,
        other => panic!("unexpected command: {other:?}"),
    }
}

#[test]
fn sort_many_cues_is_ascending_and_stable() {
    // 同じ瞬間の合図を 3 件ずつ、降順で 24 組並べる。
    let mut cues = Vec::new();
    for i in 0..24u32 {
        let t = TimeSpan::try_from_ticks(i64::from(24 - i) * 7).unwrap();
        for k in 0..3 {
            cues.push(cue(t, &format!("{}-{k}", 24 - i)));
        }
    }
    let sheet = CueSheet::new(cues);

    let labels: Vec<&str> = sheet
        .cues()
        .iter()
        .map(|c| match &c.payload {
            CuePayload::Command(command) => label(command),
            other => panic!("unexpected payload: {other:?}"),
        })
        .collect();
    let expected: Vec<String> = (1..=24)
        .flat_map(|t| (0..3).map(move |k| format!("{t}-{k}")))
        .collect();
    assert_eq!(labels, expected);
}

#[test]
fn equal_start_time_keeps_script_order_through_schedule() {
    let sheet = CueSheet::new(vec![
        cue(TimeSpan::from_millis(1), "e"),
        cue(TimeSpan::ZERO, "a"),
        cue(TimeSpan::ZERO, "b"),
        cue(TimeSpan::ZERO, "c"),
        cue(TimeSpan::ZERO, "d"),
    ])
    .with_absolute_start_time(TimePoint::from_ticks(0));

    let mut schedule = to_talk_schedule(&sheet).unwrap();
    schedule.tick(TimePoint::from_ticks(0));
    let ready: Vec<&str> = schedule.ready().iter().map(|c| label(&c.command)).collect();
    assert_eq!(ready, ["a", "b", "c", "d"]);

    schedule.tick(TimePoint::from_ticks(10_000));
    let ready: Vec<&str> = schedule.ready().iter().map(|c| label(&c.command)).collect();
    assert_eq!(ready, ["e"]);
}
