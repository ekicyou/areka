//! 台本と予定の並べ替えの比べ方の檻（#10）。
//!
//! 比べは −0.0 を +0.0 にそろえたうえでの全順序（`total_cmp`）。NaN を多く含んでも
//! 全順序どおりに並び（符号で両端に分かれる）、−0.0 と +0.0 は同じ瞬間として台本の順を保つ。

use std::cmp::Ordering;
use std::hint::black_box;

use dola::cue::{
    ActorKey, Cue, CueCommand, CuePayload, CueSheet, Entry, TimedSchedule, to_talk_schedule,
};

fn cue(start_time: f64, label: &str) -> Cue {
    Cue {
        actor: ActorKey::from("a"),
        start_time,
        payload: CueCommand::Text(label.into()).into(),
        duration: 0.0,
    }
}

fn label(command: &CueCommand) -> &str {
    match command {
        CueCommand::Text(s) => s,
        other => panic!("unexpected command: {other:?}"),
    }
}

/// 並べ替えの決まりと同じ比べ（−0.0 を +0.0 にそろえた全順序）で、昇順になっているか。
fn assert_total_order(times: &[f64]) {
    for (i, w) in times.windows(2).enumerate() {
        assert_ne!(
            (w[0] + 0.0).total_cmp(&(w[1] + 0.0)),
            Ordering::Greater,
            "index {i}: {:?} の後に {:?}（全体 {times:?}）",
            w[0],
            w[1]
        );
    }
}

#[test]
fn sort_with_many_nan_follows_total_order() {
    // 計算で生まれる NaN（x86 では符号の桁が立つ）と符号なしの NaN を合わせて 24 件、有限の時刻と交互に。
    let computed_nan = black_box(f64::INFINITY) - f64::INFINITY;
    let mut cues = Vec::new();
    for i in 0..24 {
        let nan = if i % 2 == 0 { f64::NAN } else { computed_nan };
        cues.push(cue(nan, &format!("nan{i}")));
        cues.push(cue(f64::from(24 - i), &format!("t{i}")));
    }

    let offsets: Vec<f64> = cues.iter().map(|c| c.start_time).collect();
    let sheet = CueSheet::new(cues);
    let times: Vec<f64> = sheet.cues().iter().map(|c| c.start_time).collect();
    assert_eq!(times.len(), 48);
    assert_eq!(times.iter().filter(|t| t.is_nan()).count(), 24);
    assert_total_order(&times);

    // この半分は release でだけ走る（debug では予定の挿入の debug_assert が NaN のオフセットを拒む）。
    // 台本で並べ直す前の並びのまま渡し、予定自身の並べ替えを見る。
    if !cfg!(debug_assertions) {
        let mut schedule = TimedSchedule::<f64>::with_horizon(0.0, 0.0);
        schedule.insert_each(offsets.iter().map(|&t| Entry::Payload(t, t)));
        schedule.tick(1.0e9);
        let delivered: Vec<f64> = schedule.ready().to_vec();
        assert_eq!(delivered.len(), 48);
        assert_total_order(&delivered);
    }
}

#[test]
fn negative_zero_keeps_script_order() {
    let sheet = CueSheet::new(vec![
        cue(1.0, "e"),
        cue(0.0, "a"),
        cue(-0.0, "b"),
        cue(0.0, "c"),
        cue(-0.0, "d"),
    ]);
    let labels: Vec<&str> = sheet
        .cues()
        .iter()
        .map(|c| match &c.payload {
            CuePayload::Command(command) => label(command),
            other => panic!("unexpected payload: {other:?}"),
        })
        .collect();
    assert_eq!(labels, ["a", "b", "c", "d", "e"]);

    let mut schedule = to_talk_schedule(&sheet);
    schedule.tick(0.0);
    let ready: Vec<&str> = schedule.ready().iter().map(|c| label(&c.command)).collect();
    assert_eq!(ready, ["a", "b", "c", "d"]);
}
