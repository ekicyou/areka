//! 刻みと表の差し替えに部品の時計をつなぐ檻（spec: areka-P0-surface-element-nesting 要件 5.6・5.8・
//! 7.2・7.3・8.3・tasks.md 7.4）。
//!
//! 表は `surfaces.txt` の本文から解析 → 畳み込み → [`AnimationTable::from_world`] の実経路で組む。
//! 刻みは `on_tick(now_ms)`、乱数は注入列で、どちらも決定論。

use std::sync::{Arc, Mutex};

use areka_emo_compose::{BindSet, EmoWorld, PatternState};

use super::tests::{always_fire, cfg, counting_rng};
use super::*;
use crate::resolve::SurfaceTarget;

fn table_of(text: &str) -> AnimationTable {
    AnimationTable::from_world(&EmoWorld::build(&areka_parsers::shell::parse(text)))
}

fn no_binds() -> BindSet {
    BindSet::from_ids([])
}

/// 部品の欄を外した写しと等しい＝部品の欄が空。
fn has_no_parts(pattern: &PatternState) -> bool {
    let mut stripped = pattern.clone();
    stripped.clear_parts();
    stripped == *pattern
}

/// `Show` の (面, 運んだコマの状態)。
fn show_of(cmd: &DisplayCommand) -> (u32, &PatternState) {
    match cmd {
        DisplayCommand::Show {
            surface_id,
            pattern,
            ..
        } => (*surface_id, pattern),
        other => panic!("Show を期待: {other:?}"),
    }
}

/// 部品 `part` の欄を (animation の番号, コマの番号) の列で読む。
fn part_frames(pattern: &PatternState, part: u32) -> Vec<(u32, u32)> {
    pattern
        .part(part)
        .map(|(id, f)| (id, f.surface_id))
        .collect()
}

/// 一番上 0 が子 100 を置き、1 は何も置かない。100 の animation0 は 101（すぐ）→ 102（500ms 後）
/// → `-1`（さらに 500ms 後）。
const CHILD: &str = "surface0\n{\nelement0,overlay,100,0,0\n}\n\
    surface1\n{\n}\n\
    surface100\n{\nanimation0.interval,random,2\n\
    animation0.pattern0,overlay,101,0,0,0\n\
    animation0.pattern1,overlay,102,500,0,0\n\
    animation0.pattern2,overlay,-1,500,0,0\n}\n\
    surface101\n{\n}\n\
    surface102\n{\n}\n";

/// シェルの表の差し替えで部品の時計を捨てる。面の切り替え（一番上の再生を捨てる口）は
/// 部品の時計に触らない（要件 5.8・5.9）。
#[test]
fn shell_table_replacement_discards_part_clocks_but_surface_switch_does_not() {
    let mut rt = LoopRuntime::new(cfg(table_of(CHILD), always_fire()));
    let mut states = ScopeStates::new(no_binds());
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(0));

    rt.on_tick(0, &mut states);
    let cmds = rt.on_tick(1000, &mut states);
    let playing = Some(crate::parts::PartAnim::Playing {
        started_at_ms: 1000,
    });
    assert_eq!(rt.parts.clock(&scope, 100, 0), playing, "境界で部品が発火");
    assert_eq!(cmds.len(), 1, "部品のコマが載った Show が 1 件");
    assert_eq!(part_frames(show_of(&cmds[0]).1, 100), vec![(0, 101)]);

    // 面の切り替え: 一番上の再生だけ捨て、部品の時計はそのまま。
    states.apply(&scope, SurfaceTarget::Show(1));
    rt.on_surface_changed(&scope, Slot::Shell);
    assert_eq!(rt.parts.clock(&scope, 100, 0), playing);

    // シェルの表の差し替え: 部品の時計を捨てる。
    states.rebase_shell(no_binds());
    rt.replace_shell_table(table_of(CHILD));
    assert!(rt.parts.is_empty(), "差し替えの後に部品の時計が空");
}

/// `refresh`: 抽選の時計も乱数も触らずに、直前の刻みの時刻の部品のコマを載せた `Show` を返す。
/// 刻みが 1 度も来ていない・シェル面が表示中でない・子を置いていない面では何も返さない（要件 5.6）。
#[test]
fn refresh_reloads_part_frames_at_last_tick_time() {
    let (rng, probe) = counting_rng(&[0]);
    let mut rt = LoopRuntime::new(cfg(table_of(CHILD), rng));
    let mut states = ScopeStates::new(no_binds());
    let scope = ActorKey::from("0");

    assert_eq!(
        rt.refresh(&scope, Slot::Shell, None, &mut states),
        None,
        "表示中でない"
    );
    states.apply(&scope, SurfaceTarget::Show(0));
    assert_eq!(
        rt.refresh(&scope, Slot::Shell, None, &mut states),
        None,
        "刻みが来ていない"
    );

    rt.on_tick(0, &mut states);
    rt.on_tick(1000, &mut states);
    rt.on_tick(1600, &mut states); // 経過 600 → 102
    assert_eq!(probe.lock().unwrap().calls, 1);

    // 子を置いていない面へ: 載せる部品が無い。
    states.apply(&scope, SurfaceTarget::Show(1));
    rt.on_surface_changed(&scope, Slot::Shell);
    assert_eq!(rt.refresh(&scope, Slot::Shell, None, &mut states), None);

    // 戻る: 切り替えの Show は空のコマ。refresh が 1600 の時点の 102 を載せた Show を返す。
    states.apply(&scope, SurfaceTarget::Show(0));
    rt.on_surface_changed(&scope, Slot::Shell);
    let cmd = rt
        .refresh(&scope, Slot::Shell, None, &mut states)
        .expect("部品のコマを載せた Show");
    let (sid, pattern) = show_of(&cmd);
    assert_eq!(sid, 0);
    assert_eq!(part_frames(pattern, 100), vec![(0, 102)]);
    assert_eq!(
        rt.parts.clock(&scope, 100, 0),
        Some(crate::parts::PartAnim::Playing {
            started_at_ms: 1000
        }),
        "時計は書き換えない"
    );
    assert_eq!(probe.lock().unwrap().calls, 1, "乱数を呼ばない");
    assert_eq!(
        rt.refresh(&scope, Slot::Shell, None, &mut states),
        None,
        "同じ内容は二度出さない"
    );
}

/// 入れ子も動く部品も無い表: 刻みと切り替えを回しても部品の時計は空・部品の欄は空・乱数は一番上の
/// 抽選の分だけ（要件 7.2・7.3）。
#[test]
fn table_without_animated_parts_never_enters_part_path() {
    // 一番上のコマが指す 100 は自分の animation を持たない。
    let table = table_of(
        "surface0\n{\nanimation0.interval,random,2\n\
         animation0.pattern0,overlay,100,50,0,0\n\
         animation0.pattern1,overlay,-1,50,0,0\n}\n\
         surface1\n{\nanimation3.interval,random,3\n\
         animation3.pattern0,overlay,100,50,0,0\n}\n\
         surface100\n{\n}\n",
    );
    assert!(!table.has_animated_parts(), "前提: 動く部品が無い表");
    let (rng, probe) = counting_rng(&[0; 16]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let mut states = ScopeStates::new(no_binds());
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(0));

    let mut shows = 0;
    let mut now = 0;
    while now <= 6000 {
        if now == 3050 {
            states.apply(&scope, SurfaceTarget::Show(1));
            rt.on_surface_changed(&scope, Slot::Shell);
            assert_eq!(rt.refresh(&scope, Slot::Shell, None, &mut states), None);
        }
        for cmd in rt.on_tick(now, &mut states) {
            assert!(
                has_no_parts(show_of(&cmd).1),
                "{now}: Show に部品の欄が無い"
            );
            shows += 1;
        }
        assert!(has_no_parts(states.current_pattern(&scope, Slot::Shell)));
        now += 50;
    }

    assert!(rt.parts.is_empty(), "部品の時計に 1 度も触らない");
    // 一番上の規則だけで数えた回数: 面 0 の境界 1000・2000・3000（`-1` で止まって再抽選）と、
    // 面 1 の境界 4000・5000・6000（末尾で保つ間も抽選の対象）で 1 回ずつ。
    assert_eq!(probe.lock().unwrap().calls, 6, "乱数は一番上の抽選の分だけ");
    assert!(shows > 0);
}

/// 部品の抽選は、一番上の抽選を全スコープぶん引き終えた後に、スコープの昇順 → 繰り返しの回 →
/// 部品の番号の昇順 → animation の番号の昇順で引かれる（tasks.md 7.4・design.md「抽選の消費順」）。
///
/// K の値で誰が引いたかを見分ける: 一番上 2・部品 100 の animation1 が 3・animation2 が 4・
/// 部品 50（100 のコマが指して 2 回目の回で見える）が 5。50 は 100 より番号が小さくても後に引かれる。
#[test]
fn part_draws_follow_all_top_draws_in_fixed_order() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,100,0,0\n\
         animation9.interval,random,2\n\
         animation9.pattern0,overlay,200,0,0,0\n}\n\
         surface100\n{\nanimation1.interval,random,3\n\
         animation1.pattern0,overlay,50,0,0,0\n\
         animation2.interval,random,4\n\
         animation2.pattern0,overlay,201,0,0,0\n}\n\
         surface50\n{\nanimation0.interval,random,5\n\
         animation0.pattern0,overlay,202,0,0,0\n}\n\
         surface200\n{\n}\nsurface201\n{\n}\nsurface202\n{\n}\n",
    );
    let bounds = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&bounds);
    let rng: LoopRng = Box::new(move |k| {
        log.lock().unwrap().push(k);
        0
    });
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let mut states = ScopeStates::new(no_binds());
    let sakura = ActorKey::from("0");
    let kero = ActorKey::from("1");
    states.apply(&kero, SurfaceTarget::Show(0));
    states.apply(&sakura, SurfaceTarget::Show(0));

    rt.on_tick(0, &mut states);
    assert!(bounds.lock().unwrap().is_empty(), "境界の前は引かない");
    let cmds = rt.on_tick(1000, &mut states);

    assert_eq!(*bounds.lock().unwrap(), vec![2, 2, 3, 4, 5, 3, 4, 5]);
    assert_eq!(cmds.len(), 2);
    for cmd in &cmds {
        let (_, pattern) = show_of(cmd);
        assert_eq!(part_frames(pattern, 100), vec![(1, 50), (2, 201)]);
        assert_eq!(part_frames(pattern, 50), vec![(0, 202)]);
    }
}
