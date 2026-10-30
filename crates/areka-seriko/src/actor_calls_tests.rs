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
