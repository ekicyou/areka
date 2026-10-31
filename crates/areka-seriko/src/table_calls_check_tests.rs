//! 8 語の誤りを読み込みのときに 1 度だけ記録する檻
//! （spec: seriko-pattern-calls 要件 2.5・5.1・5.3・5.4・6.4〜6.6・8.3）。
//!
//! 入力は `surfaces.txt` の本文そのもの（解析を経る実経路）で組む。

use std::collections::BTreeMap;

use areka_emo_compose::{BindSet, EmoWorld};
use log_capture_kit::{LineFormat, capture_lines};

use super::FrameAct;
use crate::looper::{LoopRuntime, SerikoLoopConfig};
use crate::resolve::SurfaceTarget;
use crate::state::{ScopeStates, Slot};
use crate::table::AnimationTable;

/// 6 種の誤りを 1 件ずつ持つ面 10。
const SIX_KINDS: &str = "surface10\n{\n\
    animation0.interval,random,2\n\
    animation0.pattern0,overlay,100,50,0,0\n\
    animation0.pattern1,start,abc\n\
    animation0.pattern2,parallelstart,(1,7)\n\
    animation0.pattern3,insert,1\n\
    animation1.interval,bind\n\
    animation1.pattern0,overlay,101,0,0,0\n\
    animation1.pattern1,move,0,10,3,5\n\
    animation1.pattern2,insert,9\n\
    animation2.interval,bind\n\
    animation2.pattern0,overlay,102,0,0,0\n\
    animation2.pattern1,insert,2\n}\n";

/// 6 種の記録の目印（要件の番号）。
const KINDS: [&str; 6] = [
    "要件 2.5",
    "要件 5.1",
    "要件 5.3",
    "要件 6.6",
    "要件 6.5",
    "要件 6.4",
];

fn table_and_logs(text: &str) -> (AnimationTable, Vec<String>) {
    let world = EmoWorld::build(&areka_parsers::shell::parse(text));
    capture_lines(LineFormat::LevelTargetFields, || {
        AnimationTable::from_world(&world)
    })
}

fn count(logs: &[String], needle: &str) -> usize {
    logs.iter().filter(|l| l.contains(needle)).count()
}

/// 6 種がそれぞれ 1 度だけ、面・animation・pattern・語・原文の欄つきで出る。
#[test]
fn each_kind_is_warned_once_with_its_place() {
    let (_, logs) = table_and_logs(SIX_KINDS);
    for kind in KINDS {
        assert_eq!(count(&logs, kind), 1, "{kind}: {logs:#?}");
    }
    let warns: Vec<&String> = logs.iter().filter(|l| l.contains("WARN")).collect();
    assert_eq!(warns.len(), 6, "ほかの WARN は無い: {warns:#?}");
    let unreadable = warns.iter().find(|l| l.contains("要件 2.5")).unwrap();
    for field in [
        "surface_id=10",
        "animation_id=0",
        "pattern=1",
        "word=\"start\"",
        "target=\"abc\"",
    ] {
        assert!(unreadable.contains(field), "{field}: {unreadable}");
    }
    let missing = warns.iter().find(|l| l.contains("要件 5.1")).unwrap();
    assert!(missing.contains("missing=7"), "{missing}");
}

/// 純 `bind` の `move` は何もしないコマになる（待ち・X・Y も 0）。
#[test]
fn move_in_pure_bind_becomes_inert() {
    let (table, _) = table_and_logs(SIX_KINDS);
    let anim = table.animations(10).iter().find(|a| a.id == 1).unwrap();
    let f = &anim.frames[1];
    assert_eq!((&f.act, f.wait_ms, f.x, f.y), (&FrameAct::Inert, 0, 0, 0));
    // 抽選で動く animation の呼び出しはそのまま働く（相手の欠けは実行時にその相手だけ何もしない）。
    let caller = table.animations(10).iter().find(|a| a.id == 0).unwrap();
    assert_eq!(
        caller.frames[2].act,
        FrameAct::Start {
            targets: [1, 7].into(),
            pick_one: false
        }
    );
}

/// 誤りの無い呼び出しと差し込みは何も記録しない。
#[test]
fn sound_calls_log_nothing() {
    let text = "surface10\n{\n\
        animation0.interval,random,2\n\
        animation0.pattern0,alternativestart,(1,2)\n\
        animation0.pattern1,move,-1,10,0,5\n\
        animation1.interval,bind\n\
        animation1.pattern0,insert,3\n\
        animation2.interval,never\n\
        animation2.pattern0,overlay,102,10,0,0\n\
        animation3.interval,bind\n\
        animation3.pattern0,overlay,103,0,0,0\n}\n";
    let (_, logs) = table_and_logs(text);
    assert!(logs.iter().all(|l| !l.contains("WARN")), "{logs:#?}");
}

/// 表を作るたびに 1 度で、表を使って刻みを回しても増えない。
#[test]
fn ticks_add_no_warn() {
    let (table, logs) = table_and_logs(SIX_KINDS);
    assert_eq!(count(&logs, "WARN"), 6);
    let (_, ticking) = capture_lines(LineFormat::LevelTargetFields, || {
        let mut rt = LoopRuntime::new(SerikoLoopConfig {
            shell_table: table,
            balloon_tables: BTreeMap::new(),
            rng: Box::new(|_| 0),
            pick_rng: crate::seeded_rng(0),
        });
        let mut states = ScopeStates::new(crate::DefaultBinds::uniform(BindSet::from_ids([1, 2])));
        let scope = areka_sakura::ActorKey::from("0");
        states.apply(&scope, SurfaceTarget::Show(10));
        rt.on_surface_changed(&scope, Slot::Shell);
        rt.refresh(&scope, Slot::Shell, Some(0), &mut states);
        for now in (0..10_000).step_by(100) {
            rt.on_tick(now, &mut states);
        }
    });
    for kind in KINDS {
        assert_eq!(count(&ticking, kind), 0, "{kind}: {ticking:#?}");
    }
}
