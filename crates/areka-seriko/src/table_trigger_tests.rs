//! 表が `runonce`・`periodic,数値`・`talk,数値` を採る檻
//! （spec: areka-P0-seriko-trigger-intervals 要件 1.5・7.1・7.2・7.4・8.1）。
//!
//! 入力は `surfaces.txt` の本文そのもの（解析を経る実経路）で組む。

use std::num::{NonZeroU32, NonZeroU64};

use areka_emo_compose::EmoWorld;
use log_capture_kit::{LineFormat, capture_lines};

use super::{AnimationTable, LoopTrigger};

/// 本文から表を組み、同じ走行の `tracing` 出力を 1 イベント 1 行で添えて返す。
fn table_and_logs(text: &str) -> (AnimationTable, Vec<String>) {
    let world = EmoWorld::build(&areka_parsers::shell::parse(text));
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&world)
    })
}

/// 間隔の語 1 つだけを差し替えた面 0 の animation 3（コマ 2 本）。
fn surface0_with(interval: &str) -> String {
    format!(
        "surface0\n{{\nanimation3.interval,{interval}\n\
         animation3.pattern0,overlay,10,0,0,0\n\
         animation3.pattern1,overlay,-1,50,0,0\n}}\n"
    )
}

fn warns(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("level=WARN")).collect()
}

/// 採録の記録（表を組むときの `debug!`）。
fn adoptions(logs: &[String]) -> Vec<&String> {
    logs.iter()
        .filter(|l| l.contains("level=DEBUG") && l.contains("引き金の語を採録"))
        .collect()
}

/// 3 語はそれぞれの引き金で採り、面の番号・animation の番号・語・数値つきの `debug!` をちょうど
/// 1 回残す（`warn!` は出さない）。採った表は門を通り、`talk` の印は `talk` を採ったときだけ真。
#[test]
fn three_words_are_adopted_and_debug_logged_once() {
    let cases = [
        ("runonce", LoopTrigger::Runonce, "runonce", None),
        (
            "periodic,3",
            LoopTrigger::Periodic {
                period_ms: NonZeroU64::new(3000).expect("正"),
            },
            "periodic",
            Some("value=3"),
        ),
        (
            "talk,2",
            LoopTrigger::Talk {
                every: NonZeroU32::new(2).expect("正"),
            },
            "talk",
            Some("value=2"),
        ),
    ];
    for (interval, trigger, vocab, value) in cases {
        let (table, logs) = table_and_logs(&surface0_with(interval));
        let anims = table.animations(0);
        assert_eq!(anims.len(), 1, "{interval}: {logs:?}");
        assert_eq!(anims[0].id, 3, "{interval}");
        assert_eq!(anims[0].trigger, trigger, "{interval}");
        let targets: Vec<i64> = anims[0].frames.iter().map(|f| f.surface_id).collect();
        assert_eq!(targets, vec![10, -1], "{interval}");

        let adopted = adoptions(&logs);
        assert_eq!(adopted.len(), 1, "{interval} の採録は 1 回: {logs:?}");
        let line = adopted[0];
        assert!(
            line.contains("surface_id=0")
                && line.contains("animation_id=3")
                && line.contains(&format!("vocab=\"{vocab}\"")),
            "{interval}: {line}"
        );
        match value {
            Some(value) => assert!(line.contains(value), "{interval}: {line}"),
            None => assert!(!line.contains("value="), "{interval}: {line}"),
        }
        assert!(warns(&logs).is_empty(), "{interval}: {logs:?}");

        assert!(table.has_triggers, "{interval}");
        assert!(table.is_continuous(), "{interval} を採ったので門を通る");
        assert_eq!(table.has_talk(), vocab == "talk", "{interval}");
    }
}

/// `talk`／`periodic` の数値が正の整数として読めないときは採らず、面の番号・animation の番号・
/// 元の綴りつきの `warn!` をちょうど 1 回残す（印は偽のまま）。
#[test]
fn invalid_number_is_not_adopted_and_warns_once() {
    for interval in ["talk", "talk,abc", "periodic", "periodic,0"] {
        let (table, logs) = table_and_logs(&surface0_with(interval));
        assert!(table.animations(0).is_empty(), "{interval}: {logs:?}");
        let w = warns(&logs);
        assert_eq!(w.len(), 1, "{interval} の warn! は 1 回: {logs:?}");
        assert!(
            w[0].contains("surface_id=0")
                && w[0].contains("animation_id=3")
                && w[0].contains(&format!("vocab=\"{interval}\""))
                && w[0].contains("数値が無効"),
            "{interval}: {}",
            w[0]
        );
        assert!(adoptions(&logs).is_empty(), "{interval}: {logs:?}");
        assert!(!table.has_triggers && !table.has_talk(), "{interval}");
        assert!(!table.is_continuous(), "{interval}");
    }
}

/// コマ列が空の 3 語は今までの空の検査（`warn!`）で採らず、採録の記録も残さない。
#[test]
fn empty_frames_keep_the_existing_empty_warn() {
    for interval in ["runonce", "periodic,3", "talk,2"] {
        let text = format!("surface7\n{{\nanimation2.interval,{interval}\n}}\n");
        let (table, logs) = table_and_logs(&text);
        assert!(table.is_empty(), "{interval}");
        let w = warns(&logs);
        assert_eq!(w.len(), 1, "{interval}: {logs:?}");
        assert!(
            w[0].contains("コマ列が空のアニメは非採録")
                && w[0].contains("surface_id=7")
                && w[0].contains("animation_id=2"),
            "{interval}: {}",
            w[0]
        );
        assert!(adoptions(&logs).is_empty(), "{interval}: {logs:?}");
        assert!(!table.has_triggers && !table.has_talk(), "{interval}");
        assert!(!table.is_continuous(), "{interval}");
    }
}

/// `+` の組み合わせ・大文字混じり・範囲外の語は今までどおり採らず、元の綴りつきの `debug!` だけを
/// 残す（`warn!` は出さない。先頭の語が `talk`／`periodic` そのものでなければ無効の数値の扱いにしない）。
#[test]
fn combinations_and_out_of_scope_words_keep_the_debug_arm() {
    for (interval, vocab) in [
        ("bind+runonce", "bind+runonce"),
        ("talk+bind,2", "talk+bind"),
        ("RunOnce", "RunOnce"),
        ("Talk,2", "Talk"),
        ("yen-e", "yen-e"),
    ] {
        let (table, logs) = table_and_logs(&surface0_with(interval));
        assert!(table.animations(0).is_empty(), "{interval}: {logs:?}");
        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains(&format!("vocab=\"{vocab}\""))),
            "{interval} は元の綴りつきで debug! に残る: {logs:?}"
        );
        assert!(warns(&logs).is_empty(), "{interval}: {logs:?}");
        assert!(!table.has_triggers && !table.has_talk(), "{interval}");
        assert!(!table.is_continuous(), "{interval}");
    }
}

/// 3 語を書いていない表では印が偽（`random` だけの表は門も偽・`always` の表は門だけ真）。
#[test]
fn tables_without_the_three_words_have_false_marks() {
    let empty = AnimationTable::empty();
    assert!(!empty.has_triggers && !empty.has_talk() && !empty.is_continuous());

    let (random_only, logs) = table_and_logs(&surface0_with("random,2"));
    assert_eq!(random_only.animations(0).len(), 1);
    assert!(!random_only.has_triggers && !random_only.has_talk());
    assert!(!random_only.is_continuous());
    assert!(adoptions(&logs).is_empty(), "{logs:?}");

    let (always, _) = table_and_logs(&surface0_with("always"));
    assert!(!always.has_triggers && !always.has_talk());
    assert!(always.is_continuous(), "always の門は今までどおり");
}
