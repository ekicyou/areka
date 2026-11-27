//! 姿を変える指令（`\s`・`\b`・着せ替え）と窓の知らせの、出来事の時刻。
//!
//! 出来事の時刻は合図の発火時刻（窓の知らせは渡した時刻）なので、合図を処理したのが発火時刻の直後でも、
//! 何刻みか後でも、その後の刻みの絵は同じになる。前半の檻は 2 つの回を比べる: 刻み 1000 の直後に
//! 処理した回と、刻み 1016・1032・1048 を済ませてから処理した回。刻みは 16 ms ごと、発火時刻は 1020。
//! 後半の檻は、遅れて処理した合図が今あるべき絵を出す事と、発火時刻より前の止めが替える前に当たる事。

use super::test_support::*;
use super::*;
use crate::looper::tests::{always_fire, cfg};
use crate::output::DisplayCommand;
use crate::test_support::{capture_text, part_frames, table_of, text};
use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::{ActorKey, CueCommand};
use dola::TimeSpan;
use dola::time::testing::{at_ms, ms};
use std::collections::BTreeMap;

/// 姿を変える合図の発火時刻。
const FIRE: u32 = 1020;

/// 一番上 0 の `always`: 101（待ち 0）→ 100ms で 102 → 100ms で 103（周期 200）。
const TOP_ALWAYS: &str = "surface0\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

/// 一番上 0 の `runonce`: 50ms で 201 → 50ms で 202 → 50ms で `-1`。
const TOP_RUNONCE: &str = "surface0\n{\nanimation0.interval,runonce\n\
    animation0.pattern0,overlay,201,50,0,0\n\
    animation0.pattern1,overlay,202,50,0,0\n\
    animation0.pattern2,overlay,-1,50,0,0\n}\n\
    surface201\n{\n}\nsurface202\n{\n}\n";

/// 一番上 0 は着せ替え 5 で子 100 を出す。100 は [`TOP_ALWAYS`] と同じ `always` を持つ。
const BIND_ALWAYS: &str = "surface0\n{\nanimation5.interval,bind\n\
    animation5.pattern0,overlay,100,0,0,0\n}\n\
    surface100\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\n";

/// 面 0（口は animation 0）と面 5（口は animation 1）の一番上に `talk,1` の口（601 を 60ms 出して `-1`）。
const MOUTHS: &str = "surface0\n{\nanimation0.interval,talk,1\n\
    animation0.pattern0,overlay,601,0,0,0\n\
    animation0.pattern1,overlay,-1,60,0,0\n}\n\
    surface5\n{\nanimation1.interval,talk,1\n\
    animation1.pattern0,overlay,601,0,0,0\n\
    animation1.pattern1,overlay,-1,60,0,0\n}\n\
    surface601\n{\n}\n";

/// この檻の足場（本体は兄弟の足場 `ActorRig`・着せ替えの名前は (目, 閉) → 5）。
fn rig(config: SerikoLoopConfig) -> ActorRig {
    ActorRig::configured(config, eye_resolver(), BindSet::from_ids([]))
}

/// 刻み 1000〜1300 を回し、`late` でなければ刻み 1000 の直後に、`late` なら刻み 1048 の後に `shape`
/// を処理する（`before` は刻み 1000 の前、`between` は刻み 1000 の直後で `shape` より前に渡す）。刻み
/// 1064 から後の各刻みの後の `slot` の欄を返す。
fn drive(
    rig: &mut ActorRig,
    late: bool,
    slot: Slot,
    shape: impl FnOnce(&mut ActorRig) -> Vec<DisplayCommand>,
    mut at_tick: impl FnMut(&mut ActorRig, u32),
) -> Vec<(u32, PatternState)> {
    let mut shape = Some(shape);
    let mut seen = Vec::new();
    for t in (1000..=1300).step_by(16) {
        rig.tick(at_ms(t.into()));
        at_tick(rig, t);
        if t == if late { 1048 } else { 1000 }
            && let Some(shape) = shape.take()
        {
            shape(rig);
        }
        if t >= 1064 {
            seen.push((
                t,
                rig.states
                    .current_pattern(&ActorKey::from("0"), slot)
                    .clone(),
            ));
        }
    }
    seen
}

/// `anim` の欄が絵 `surface_id` だった刻み。
fn ticks_with(seen: &[(u32, PatternState)], anim: u32, surface_id: u32) -> Vec<u32> {
    seen.iter()
        .filter(|(_, p)| p.get(anim).is_some_and(|f| f.surface_id == surface_id))
        .map(|&(t, _)| t)
        .collect()
}

/// 部品 `part` の欄に絵 `surface_id` が載っていた刻み。
fn part_ticks_with(seen: &[(u32, PatternState)], part: u32, surface_id: u32) -> Vec<u32> {
    seen.iter()
        .filter(|(_, p)| part_frames(p, part).iter().any(|&(_, s)| s == surface_id))
        .map(|&(t, _)| t)
        .collect()
}

/// `from` から `to` までの刻み（16 ms ごと）。
fn ticks(from: u32, to: u32) -> Vec<u32> {
    (from..=to).step_by(16).collect()
}

/// 発火時刻 1020 の台本の合図。
fn script_cue(command: CueCommand) -> TalkCue {
    cue(ms(FIRE), "0", command, TimeSpan::ZERO)
}

fn emote(key: &str) -> CueCommand {
    CueCommand::Emote { key: key.into() }
}

/// 面の切り替えは、処理した時に依らず `always` を発火時刻 1020 で始める（102 は 1120 から 1220 まで）。
#[test]
fn surface_cue_starts_always_at_its_fire_time_however_late_it_is_processed() {
    let run = |late| {
        let mut rig = rig(cfg(table_of(TOP_ALWAYS), always_fire()));
        drive(
            &mut rig,
            late,
            Slot::Shell,
            |b| b.hear(script_cue(emote("0"))),
            |_, _| {},
        )
    };
    let soon = run(false);
    assert_eq!(ticks_with(&soon, 0, 102), ticks(1128, 1208), "{soon:?}");
    assert_eq!(run(true), soon, "何刻みか後に処理しても同じ");
}

/// 面の切り替えは、処理した時に依らず `runonce` を発火時刻 1020 で鳴らす（201 は 1070・202 は 1120・
/// 1170 で止まる）。
#[test]
fn surface_cue_rings_runonce_at_its_fire_time_however_late_it_is_processed() {
    let run = |late| {
        let mut rig = rig(cfg(table_of(TOP_RUNONCE), always_fire()));
        drive(
            &mut rig,
            late,
            Slot::Shell,
            |b| b.hear(script_cue(emote("0"))),
            |_, _| {},
        )
    };
    let soon = run(false);
    assert_eq!(ticks_with(&soon, 0, 201), ticks(1080, 1112), "{soon:?}");
    assert_eq!(ticks_with(&soon, 0, 202), ticks(1128, 1160), "{soon:?}");
    assert_eq!(run(true), soon, "何刻みか後に処理しても同じ");
}

/// 1010 に現れた字（発火時刻 1020 より前）の区切りは替える前の面 0 で判定され（口 0 が 1010 に 1 回
/// 鳴る）、面 5 の口の数え始めはその字の次になる: 面 5 の口は 1100 の字で初めて動く。
#[test]
fn glyphs_up_to_the_fire_time_are_judged_on_the_surface_before_the_switch() {
    let run = |late| {
        let mut rig = rig(cfg(table_of(MOUTHS), always_fire()));
        let mut seen = Vec::new();
        let logs = capture_text(|| {
            rig.hear(cue(ms(0), "0", emote("0"), TimeSpan::ZERO));
            seen = drive(
                &mut rig,
                late,
                Slot::Shell,
                |b| b.hear(script_cue(emote("5"))),
                |b, t| match t {
                    1000 => {
                        b.hear(text(ms(1010), "0", "あ"));
                    }
                    1096 => {
                        b.hear(text(ms(1100), "0", "い"));
                    }
                    _ => {}
                },
            );
        });
        let rang = logs
            .lines()
            .filter(|l| {
                l.contains("animation_id=0") && l.contains(&format!("started_at={}", at_ms(1010)))
            })
            .count();
        (seen, rang)
    };
    let (soon, rang) = run(false);
    assert_eq!(rang, 1, "面 0 の口が 1010 に 1 回鳴る");
    assert_eq!(ticks_with(&soon, 1, 601), ticks(1112, 1144), "{soon:?}");
    assert_eq!(run(true), (soon, 1), "何刻みか後に処理しても同じ");
}

/// 着せ替えは、処理した時に依らず、見えた部品の `always` を発火時刻 1020 で始める。
#[test]
fn bind_cue_starts_part_always_at_its_fire_time_however_late_it_is_processed() {
    let run = |late| {
        let mut rig = rig(cfg(table_of(BIND_ALWAYS), always_fire()));
        rig.hear(cue(ms(0), "0", emote("0"), TimeSpan::ZERO));
        drive(
            &mut rig,
            late,
            Slot::Shell,
            |b| {
                b.hear(fired(
                    at_ms(FIRE.into()),
                    carrier_cue("0", "bind", &["目", "閉", "1"]),
                ))
            },
            |_, _| {},
        )
    };
    let soon = run(false);
    assert_eq!(
        part_ticks_with(&soon, 100, 102),
        ticks(1128, 1208),
        "{soon:?}"
    );
    assert_eq!(run(true), soon, "何刻みか後に処理しても同じ");
}

/// バルーンの面の切り替えも、処理した時に依らず `always` を発火時刻 1020 で始める。
#[test]
fn balloon_cue_starts_always_at_its_fire_time_however_late_it_is_processed() {
    let run = |late| {
        let mut rig = rig(SerikoLoopConfig {
            shell_table: AnimationTable::empty(),
            balloon_tables: BTreeMap::from([(ActorKey::from("0"), table_of(TOP_ALWAYS))]),
            rng: always_fire(),
            pick_rng: crate::seeded_rng(0),
        });
        drive(
            &mut rig,
            late,
            Slot::Balloon,
            |b| b.hear(script_cue(CueCommand::BalloonSurface { key: "0".into() })),
            |_, _| {},
        )
    };
    let soon = run(false);
    assert_eq!(ticks_with(&soon, 0, 102), ticks(1128, 1208), "{soon:?}");
    assert_eq!(run(true), soon, "何刻みか後に処理しても同じ");
}

/// 手で組んだ合図（起点がフレームの時刻・台本の 0）も、その起点が出来事の時刻になる。
#[test]
fn hand_built_cue_uses_its_origin_as_the_event_time() {
    let run = |late| {
        let mut rig = rig(cfg(table_of(TOP_ALWAYS), always_fire()));
        drive(
            &mut rig,
            late,
            Slot::Shell,
            |b| {
                b.hear(TalkCue::scheduled(
                    at_ms(FIRE.into()),
                    TimeSpan::ZERO,
                    ActorKey::from("0"),
                    emote("0"),
                    TimeSpan::ZERO,
                ))
            },
            |_, _| {},
        )
    };
    let soon = run(false);
    assert_eq!(ticks_with(&soon, 0, 102), ticks(1128, 1208), "{soon:?}");
    assert_eq!(run(true), soon, "何刻みか後に処理しても同じ");
}

/// 窓の知らせは、渡した時刻 1020 を出来事の時刻にする（閉じた窓では始まらなかった `always` が 1020 で
/// 始まる）。
#[test]
fn window_notice_uses_the_time_it_carries() {
    let run = |late| {
        let mut rig = rig(SerikoLoopConfig {
            shell_table: AnimationTable::empty(),
            balloon_tables: BTreeMap::from([(ActorKey::from("0"), table_of(TOP_ALWAYS))]),
            rng: always_fire(),
            pick_rng: crate::seeded_rng(0),
        });
        let note = |open| StageNote::Balloon {
            scope: ActorKey::from("0"),
            open,
            face: 0,
            generation: 1,
        };
        rig.send(SerikoMsg::Stage {
            note: note(false),
            at: at_ms(0),
        });
        rig.hear(cue(
            ms(0),
            "0",
            CueCommand::BalloonSurface { key: "0".into() },
            TimeSpan::ZERO,
        ));
        drive(
            &mut rig,
            late,
            Slot::Balloon,
            |b| {
                b.send(SerikoMsg::Stage {
                    note: note(true),
                    at: at_ms(FIRE.into()),
                })
            },
            |_, _| {},
        )
    };
    let soon = run(false);
    assert_eq!(ticks_with(&soon, 0, 102), ticks(1128, 1208), "{soon:?}");
    assert_eq!(run(true), soon, "何刻みか後に処理しても同じ");
}

/// 一番上 0 の `always`（101 → 20ms で 102 → 20ms で 103 → 20ms で 104・周期 60）と、着せ替え 5 で 130 を
/// 出す animation5。
const FAST_ALWAYS: &str = "surface0\n{\nanimation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,20,0,0\n\
    animation0.pattern2,overlay,103,20,0,0\n\
    animation0.pattern3,overlay,104,20,0,0\n\
    animation5.interval,bind\n\
    animation5.pattern0,overlay,130,0,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\nsurface104\n{\n}\nsurface130\n{\n}\n";

/// 一番上の animation `anim` の欄の絵。
fn top_cell(pattern: &PatternState, anim: u32) -> Option<u32> {
    pattern.get(anim).map(|f| f.surface_id)
}

/// 遅れて処理した面の切り替えは、処理した時に今あるべき絵を出す: 刻み 1048 の後に処理した合図の指令の
/// 欄は、刻み 1000 の直後に処理して刻み 1048 まで進めた欄と同じ（発火時刻 1020 から 28ms＝102）。
#[test]
fn late_surface_cue_shows_the_picture_due_at_processing() {
    let config = || cfg(table_of(FAST_ALWAYS), always_fire());
    let mut soon = rig(config());
    soon.tick(at_ms(1000));
    soon.hear(script_cue(emote("0")));
    for t in [1016, 1032, 1048] {
        soon.tick(at_ms(t));
    }
    let due = soon
        .states
        .current_pattern(&ActorKey::from("0"), Slot::Shell)
        .clone();
    assert_eq!(top_cell(&due, 0), Some(102), "前提: {due:?}");

    let mut late = rig(config());
    for t in [1000, 1016, 1032, 1048] {
        late.tick(at_ms(t));
    }
    let (_, _, shown) = single_show(late.hear(script_cue(emote("0"))));
    assert_eq!(shown, due, "処理した時の指令は今あるべき絵");
}

/// 遅れて処理した着せ替えは、動いている `always` を発火時刻の絵へ戻さない: 刻み 1048 の後に処理した
/// 発火時刻 1030 の着せ替えの指令は、`always` が刻み 1048 の絵（0 から 1048 ms＝102）のまま。
#[test]
fn late_bind_cue_does_not_step_a_running_always_back() {
    let mut rig = rig(cfg(table_of(FAST_ALWAYS), always_fire()));
    rig.hear(cue(ms(0), "0", emote("0"), TimeSpan::ZERO));
    for t in (0..=1048).step_by(16) {
        rig.tick(at_ms(t));
    }
    let scope = ActorKey::from("0");
    assert_eq!(
        top_cell(rig.states.current_pattern(&scope, Slot::Shell), 0),
        Some(102),
        "前提"
    );
    let (_, binds, shown) = single_show(rig.hear(fired(
        at_ms(1030),
        carrier_cue("0", "bind", &["目", "閉", "1"]),
    )));
    assert_eq!(top_cell(&shown, 0), Some(102), "{shown:?}");
    assert!(binds.contains(5), "着せ替えは当たる");
}

/// 面 0: animation0 は `always`（101→102→103・周期 200）、animation1 は抽選で動き 100 ms 後に animation0 を
/// 止める。animation5 は着せ替え 5 で 130 を出す。
const STOP_ALWAYS: &str = "surface0\n{\n\
    animation0.interval,always\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,100,0,0\n\
    animation0.pattern2,overlay,103,100,0,0\n\
    animation1.interval,random,2\n\
    animation1.pattern0,overlay,120,100,0,0\n\
    animation1.pattern1,stop,0\n\
    animation5.interval,bind\n\
    animation5.pattern0,overlay,130,0,0,0\n}\n\
    surface101\n{\n}\nsurface102\n{\n}\nsurface103\n{\n}\nsurface120\n{\n}\nsurface130\n{\n}\n";

/// 面 0 を 950 に出し、刻み 960・1000（animation1 が始まり、止めは 1100）・1090（`always` は 102）を
/// 済ませた足場。
fn stop_pending() -> ActorRig {
    let (rng, _probe) = crate::looper::tests::counting_rng(&[0]);
    let mut rig = rig(SerikoLoopConfig {
        shell_table: table_of(STOP_ALWAYS),
        balloon_tables: BTreeMap::new(),
        rng,
        pick_rng: crate::seeded_rng(0),
    });
    rig.hear(cue(ms(950), "0", emote("0"), TimeSpan::ZERO));
    for t in [960, 1000, 1090] {
        rig.tick(at_ms(t));
    }
    assert_eq!(
        top_cell(
            rig.states
                .current_pattern(&ActorKey::from("0"), Slot::Shell),
            0
        ),
        Some(102),
        "前提"
    );
    rig
}

/// 直前の刻み 1090 と発火時刻 1110 の間に在る止めは、着せ替えを当てる前に当たり、止めた `always` の絵は
/// 欄から消えたまま戻らない。
#[test]
fn stop_before_the_fire_time_clears_the_always_on_a_bind_cue() {
    let mut rig = stop_pending();
    let (_, _, shown) = single_show(rig.hear(fired(
        at_ms(1110),
        carrier_cue("0", "bind", &["目", "閉", "1"]),
    )));
    assert_eq!(top_cell(&shown, 0), None, "{shown:?}");
    rig.tick(at_ms(1200));
    assert_eq!(
        top_cell(
            rig.states
                .current_pattern(&ActorKey::from("0"), Slot::Shell),
            0
        ),
        None
    );
}

/// 同じ面の出し直しでも、発火時刻 1110 より前の止めが先に当たる（止めた `always` は出し直しの時刻 1110
/// から始め直し、前の 102 は残らない）。
#[test]
fn stop_before_the_fire_time_clears_the_always_on_a_same_surface_reshow() {
    let mut rig = stop_pending();
    rig.hear(cue(ms(1110), "0", emote("0"), TimeSpan::ZERO));
    let scope = ActorKey::from("0");
    assert_eq!(
        top_cell(rig.states.current_pattern(&scope, Slot::Shell), 0),
        None
    );
    rig.tick(at_ms(1200));
    assert_eq!(
        top_cell(rig.states.current_pattern(&scope, Slot::Shell), 0),
        None,
        "1110 から 90ms は 1 枚目"
    );
    rig.tick(at_ms(1220));
    assert_eq!(
        top_cell(rig.states.current_pattern(&scope, Slot::Shell), 0),
        Some(102),
        "1110 から 110ms"
    );
}

/// 着せ替えが変わらない合図（もう脱いでいる 5 を脱ぐ）でも、発火時刻 1110 より前の止めが当たった絵を出す。
#[test]
fn stop_before_the_fire_time_is_shown_on_an_unchanged_bind_cue() {
    let mut rig = stop_pending();
    let (_, _, shown) = single_show(rig.hear(fired(
        at_ms(1110),
        carrier_cue("0", "bind", &["目", "閉", "0"]),
    )));
    assert_eq!(top_cell(&shown, 0), None, "{shown:?}");
}
