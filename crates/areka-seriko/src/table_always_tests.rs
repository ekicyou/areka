//! 表が `always` を採る檻（spec: areka-P0-animated-image-playback 要件 4.1・4.6・4.8・4.9・8.1・8.3・8.4）。
//!
//! 入力は `surfaces.txt` の本文そのもの（解析を経る実経路）で組む。

use std::num::NonZeroU64;

use areka_emo_compose::{EmoWorld, PartKey};
use log_capture_kit::{LineFormat, capture_lines};

use super::{AnimationTable, LoopTrigger};

/// 動く絵と手書きの `always` を置いた検体のシェル（中身は同じフォルダの README）。
const FIXTURE: &str =
    include_str!("../../areka-emo-compose/tests/fixtures/animated-playback/shell/surfaces.txt");

/// 本文から表を組み、同じ走行の `tracing` 出力を 1 イベント 1 行で添えて返す。
fn table_and_logs(text: &str) -> (AnimationTable, Vec<String>) {
    let world = EmoWorld::build(&areka_parsers::shell::parse(text));
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&world)
    })
}

/// 間隔の語 1 つだけを差し替えた面 0（待ち 0・50・30）。
fn surface0_with(interval: &str) -> String {
    format!(
        "surface0\n{{\nanimation3.interval,{interval}\n\
         animation3.pattern1,overlay,11,50,0,0\n\
         animation3.pattern0,overlay,10,0,0,0\n\
         animation3.pattern2,overlay,-1,30,0,0\n}}\n"
    )
}

fn period(ms: u64) -> NonZeroU64 {
    NonZeroU64::new(ms).expect("正")
}

fn warns(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains("level=WARN")).collect()
}

/// `always` の単独（小文字の完全一致）だけを、回数なし・周期＝待ちの合計で採る。コマは pattern の
/// 番号の昇順・サーフェスを指す（絵は指さない）。
#[test]
fn exact_always_is_adopted_with_period_and_no_laps() {
    let (table, logs) = table_and_logs(&surface0_with("always"));
    let anims = table.animations(0);
    assert_eq!(anims.len(), 1, "{logs:?}");
    assert_eq!(anims[0].id, 3);
    assert_eq!(
        anims[0].trigger,
        LoopTrigger::Always {
            period_ms: period(80),
            laps: None
        }
    );
    let frames: Vec<(i64, u32, Option<u32>)> = anims[0]
        .frames
        .iter()
        .map(|f| (f.surface_id, f.wait_ms, f.picture))
        .collect();
    assert_eq!(frames, vec![(10, 0, None), (11, 50, None), (-1, 30, None)]);
    assert!(
        warns(&logs).is_empty(),
        "採ったときは warn! を出さない: {logs:?}"
    );
    assert_eq!(table.part_animations(PartKey::Surface(0)), anims);
}

/// 組み合わせ・大文字・ほかの語は今までどおり採らず、元の綴りつきの `debug!` を残す（要件 4.8・4.9）。
#[test]
fn combinations_and_other_words_keep_the_debug_arm() {
    for word in ["bind+always", "always+bind", "Always", "yen-e", "never"] {
        let (table, logs) = table_and_logs(&surface0_with(word));
        assert!(table.animations(0).is_empty(), "{word}: {logs:?}");
        assert!(!table.is_continuous(), "{word}");
        assert!(
            logs.iter()
                .any(|l| l.contains("level=DEBUG") && l.contains(&format!("vocab=\"{word}\""))),
            "{word} は元の綴りつきで debug! に残る: {logs:?}"
        );
        assert!(warns(&logs).is_empty(), "{word}: {logs:?}");
    }
}

/// 待ち時間の合計が 0 の `always` は採らず、サーフェスの番号・animation の番号・理由を `warn!` で
/// ちょうど 1 回出す（要件 4.6・8.4）。
#[test]
fn zero_total_always_is_not_adopted_and_warns_once() {
    let text = "surface7\n{\nanimation2.interval,always\n\
                animation2.pattern0,overlay,30,0,0,0\n\
                animation2.pattern1,overlay,31,0,0,0\n}\n";
    let (table, logs) = table_and_logs(text);
    assert!(table.animations(7).is_empty());
    assert!(!table.is_continuous());
    let w = warns(&logs);
    assert_eq!(w.len(), 1, "warn! はちょうど 1 回: {logs:?}");
    assert!(
        w[0].contains("surface_id=7") && w[0].contains("animation_id=2"),
        "サーフェスの番号と animation の番号が載る: {}",
        w[0]
    );
    assert!(w[0].contains("待ち時間の合計が 0"), "理由が載る: {}", w[0]);
}

/// コマが空の `always` は今までの空の検査（`warn!`）に落ち、合計 0 の `warn!` は重ねない。
#[test]
fn empty_always_keeps_the_existing_empty_warn() {
    let (table, logs) = table_and_logs("surface7\n{\nanimation2.interval,always\n}\n");
    assert!(table.is_empty());
    let w = warns(&logs);
    assert_eq!(w.len(), 1, "{logs:?}");
    assert!(w[0].contains("コマ列が空のアニメは非採録"), "{}", w[0]);
}

/// 門 `is_continuous()`: 一番上の `random` だけの表は偽、`always` を 1 本採れば真。
#[test]
fn is_continuous_is_true_only_when_always_or_animated_parts_exist() {
    let (random_only, _) = table_and_logs(&surface0_with("random,2"));
    assert!(!random_only.has_animated_parts());
    assert!(!random_only.is_continuous());

    let (always, _) = table_and_logs(&surface0_with("always"));
    assert!(!always.has_animated_parts(), "部品は無い");
    assert!(always.is_continuous(), "always を採ったので真");
}

/// 完了の姿: 検体の表に手書きの `always`（0・20）が載り、合計 0（10）と `bind+always`（11）は載らない。
/// 記録は表を作るときだけ、合計 0 の 1 件が 1 回。
#[test]
fn fixture_table_carries_hand_written_always() {
    let (table, logs) = table_and_logs(FIXTURE);
    let always_of = |s: u32| -> Vec<(u32, LoopTrigger, Vec<i64>)> {
        table
            .animations(s)
            .iter()
            .map(|a| {
                (
                    a.id,
                    a.trigger,
                    a.frames.iter().map(|f| f.surface_id).collect(),
                )
            })
            .collect()
    };
    let endless = |ms| LoopTrigger::Always {
        period_ms: period(ms),
        laps: None,
    };
    assert_eq!(always_of(0), vec![(0, endless(200), vec![30, 31])]);
    assert_eq!(always_of(20), vec![(0, endless(100), vec![30, 31])]);
    assert!(always_of(10).is_empty(), "合計 0 は採らない");
    assert!(always_of(11).is_empty(), "bind+always は採らない");
    assert!(table.is_continuous());

    let w = warns(&logs);
    assert_eq!(w.len(), 1, "原因は合計 0 の 1 件だけ: {logs:?}");
    assert!(w[0].contains("surface_id=10") && w[0].contains("animation_id=0"));
}

/// 実物の emo2 の表: `always` を持たず、門 `is_continuous()` は今の門
/// （`has_animated_parts` ＝ 2110／2210 が自分のアニメーションを持つので真）と同じ値。
#[test]
fn emo2_table_is_continuous_equals_current_gate() {
    let path = crate::sample_test_support::emo2_root()
        .join("shell")
        .join("master")
        .join("surfaces.txt");
    let content = std::fs::read_to_string(&path).expect("emo2 surfaces.txt を読める");
    let world = EmoWorld::build(&areka_parsers::shell::parse(&content));
    let table = AnimationTable::from_world(&world);

    assert!(table.has_animated_parts(), "本 spec の前と同じ（真）");
    assert_eq!(table.is_continuous(), table.has_animated_parts());
    for id in world.surface_ids() {
        assert!(
            table
                .animations(id)
                .iter()
                .all(|a| !matches!(a.trigger, LoopTrigger::Always { .. })),
            "emo2 の面 {id} に always は無い"
        );
    }
}
