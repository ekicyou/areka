//! 面を出す合図で止めた `always` の印を消す配線のテスト（spec: seriko-pattern-calls 要件 3.5・M2）。

use super::test_support::*;
use super::*;
use crate::looper::tests::counting_rng;
use crate::table::AnimationTable;
use areka_emo_compose::EmoWorld;
use std::collections::BTreeMap;

/// 面 0: animation0 は `always`（101→102→103・周期 200）、animation1 は抽選で動き 100 ms 後に animation0 を止める。
const STOP_ALWAYS: &str = "surface0\n{\n\
    animation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n\
    animation1.interval,random,2\n\
    animation1.pattern0,overlay,120,100,0,0\n\
    animation1.pattern1,stop,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\nsurface120\n{\n}\n";

/// 同じ面の出し直し（状態は変わらない）でも印を消し、合図の時刻で `always` を始め直す。
#[test]
fn same_surface_reshow_restarts_a_stopped_always_at_the_event_time() {
    let (rng, _probe) = counting_rng(&[0]);
    let table =
        AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(STOP_ALWAYS)));
    let mut rig = ClockRig::with_config(SerikoLoopConfig {
        shell_table: table,
        balloon_tables: BTreeMap::new(),
        rng,
        pick_rng: crate::seeded_rng(0),
    });
    rig.hear(0, emote_cue(0.0, "0", "0"));
    rig.tick(10);
    rig.tick(1000); // animation1 が始まる。止めは 1100。
    assert_eq!(frames(&rig.tick(1100), 0), vec![("0", None)]);
    assert!(
        rig.tick(2000).is_empty(),
        "止めた always は刻みで始め直さない"
    );

    // 始めの絵は経過 0 の絵と同じなので、合図では何も出ない。
    assert!(rig.hear(2500, emote_cue(2.5, "0", "0")).is_empty());
    // 合図の時刻 2500 から 150 ms＝2 枚目（刻みの時刻で始めたなら経過 0 で何も出ない）。
    assert_eq!(frames(&rig.tick(2650), 0), vec![("0", Some(102))]);
}

/// 面 0 の `runonce` が頭で `move` を当てる。
const RUNONCE_MOVE: &str = "surface0\n{\n\
    animation0.interval,runonce\n\
    animation0.pattern0,move,0,0,2,4\n\
    animation0.pattern1,overlay,120,100,0,0\n}\n\
    surface120\n{\n}\n";

fn shift(x: i64, y: i64) -> DisplayCommand {
    DisplayCommand::MoveShift {
        scope: ActorKey::from("0"),
        x,
        y,
    }
}

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

/// 面を出す合図で鳴った `move` は同じ合図の `Show` の後に出て、同じ面の出し直しでは (0, 0) が出る
/// （spec: seriko-pattern-calls 7.1・7.6・M3）。
#[test]
fn surface_cue_emits_the_shift_after_the_show_and_reshow_resets_it() {
    let (rng, _probe) = counting_rng(&[]);
    let mut rig = ClockRig::with_config(SerikoLoopConfig {
        shell_table: table_of(RUNONCE_MOVE),
        balloon_tables: BTreeMap::new(),
        rng,
        pick_rng: crate::seeded_rng(0),
    });
    rig.tick(0);
    let cmds = rig.hear(10, emote_cue(0.0, "0", "0"));
    assert_eq!(cmds.len(), 2, "{cmds:?}");
    assert!(matches!(
        cmds[0],
        DisplayCommand::Show { surface_id: 0, .. }
    ));
    assert_eq!(cmds[1], shift(2, 4));
    let later = rig.tick(200); // 2 枚目の絵（120）だけ
    assert!(
        later
            .iter()
            .all(|c| matches!(c, DisplayCommand::Show { .. })),
        "同じ値は繰り返さない: {later:?}"
    );
    assert_eq!(rig.hear(300, emote_cue(0.3, "0", "0")), vec![shift(0, 0)]);
}

/// シェルの差し替え（`Rebased`）も面を出す合図: 全スコープのずれを 0 へ戻し、合図の後に (0, 0) を出す
/// （spec: seriko-pattern-calls 3.5・7.6）。
#[test]
fn shell_replace_resets_the_shift_after_the_rebased() {
    const MOVE: &str = "surface0\n{\n\
        animation0.interval,random,2\n\
        animation0.pattern0,overlay,120,0,0,0\n\
        animation0.pattern1,move,0,0,3,5\n}\n\
        surface120\n{\n}\n";
    let no_binds = || crate::DefaultBinds::uniform(areka_emo_compose::BindSet::default());
    let out = crate::MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        SurfaceResolver::new(BTreeMap::new()),
        no_binds(),
        BindResolver::empty(),
        crate::looper::tests::cfg(table_of(MOVE), crate::looper::tests::always_fire()),
        out,
    );
    dola::cue::CueSink::emit(&mut sink, emote_cue(0.0, "0", "0"));
    sink.send_tick(0);
    sink.send_tick(1000);
    assert!(sink.send_replace(
        SerikoReplace::Shell {
            epoch: 3,
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: no_binds(),
            bind_resolver: BindResolver::empty(),
            shell_table: table_of(MOVE),
            fallback: crate::ShellSurfaceFallback::default(),
        },
        crate::SkinFacts::new(BTreeMap::new(), BTreeMap::new(), BTreeMap::new())
    ));
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let recorded = records.lock().unwrap().clone();
    let at = recorded
        .iter()
        .position(|c| matches!(c, DisplayCommand::Rebased { .. }))
        .expect("Rebased");
    assert!(recorded[..at].contains(&shift(3, 5)), "{recorded:?}");
    assert_eq!(recorded[at + 1..], [shift(0, 0)], "{recorded:?}");
}
