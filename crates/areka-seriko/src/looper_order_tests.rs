//! ループ統括器の抽選の消費順と、scope ごとのバルーン表の引きの決定論テスト。

use super::tests::*;
use super::*;
use crate::resolve::SurfaceTarget;
use areka_parsers::shell::{Animation, Interval};
use dola::time::testing::at_ms;

// ── 固定消費順（scope 昇順→Shell→Balloon→id 昇順） ─────────────────

/// 同一 surface に複数アニメ（id 昇順で消費）。注入列 [1,0] で id 昇順の 2 本目のみ発火することを示し、
/// 消費順が animation id 昇順であることを檻に入れる。
#[test]
fn lottery_consumes_in_animation_id_order() {
    // 同 surface 10 に anim id 5 と anim id 2（宣言順は 5→2＝id 昇順ではない）。
    let s = surface_with(
        10,
        vec![
            Animation {
                id: 5,
                interval: Interval::Random { k: 4 },
                patterns: vec![pat(0, 5000, 0)],
            },
            Animation {
                id: 2,
                interval: Interval::Random { k: 4 },
                patterns: vec![pat(0, 2000, 0)],
            },
        ],
    );
    let table = shell_table(vec![s]);
    // 消費列 [1, 0]: 1 本目（id 昇順の先頭＝id 2）は 1（非発火）、2 本目（id 5）は 0（発火）。
    let (rng, probe) = counting_rng(&[1, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, _scope) = shown_shell(10);

    rt.on_tick(at_ms(0), &mut states);
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2, "2 アニメ分の rng を消費");
    assert_eq!(cmds.len(), 1, "id 5 のみ発火");
    // id 2 は非発火（コマ無し）・id 5 は発火（コマ 5000）＝消費順が id 昇順（2→5）である証拠。
    let p = pattern_of(&cmds[0]);
    assert!(
        p.get(2).is_none(),
        "id 2 は非発火（消費列 1 番目に 1 が割当）"
    );
    assert_eq!(
        p.get(5).unwrap().surface_id,
        5000,
        "id 5 は発火（消費列 2 番目に 0 が割当）"
    );
}

/// scope 昇順で消費する。scope "0"→"1" の順で乱数を消費し、注入列 [1,0] で scope1 のみ発火。
#[test]
fn lottery_consumes_in_scope_ascending_order() {
    // 同一 shell_table 上に surface10（scope0 表示）と surface11（scope1 表示）。
    let s10 = surface_with(
        10,
        vec![Animation {
            id: 0,
            interval: Interval::Random { k: 4 },
            patterns: vec![pat(0, 1000, 0)],
        }],
    );
    let s11 = surface_with(
        11,
        vec![Animation {
            id: 0,
            interval: Interval::Random { k: 4 },
            patterns: vec![pat(0, 1100, 0)],
        }],
    );
    let table = shell_table(vec![s10, s11]);
    // [1, 0]: 先頭消費（scope "0"）は非発火、2 番目（scope "1"）は発火。
    let (rng, probe) = counting_rng(&[1, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let mut states = crate::test_support::empty_states();
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");
    states.apply(&s0, SurfaceTarget::Show(10));
    states.apply(&s1, SurfaceTarget::Show(11));

    rt.on_tick(at_ms(0), &mut states);
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2);
    assert_eq!(cmds.len(), 1, "scope1 のみ発火");
    match &cmds[0] {
        DisplayCommand::Show { scope, pattern, .. } => {
            assert_eq!(
                scope, &s1,
                "発火したのは scope \"1\"（scope 昇順で \"0\" が先に消費された証拠）"
            );
            assert_eq!(pattern.get(0).unwrap().surface_id, 1100);
        }
        other => panic!("Show を期待: {other:?}"),
    }
}

/// Shell を Balloon より先に消費する（同一 scope）。注入列 [0,1] で Shell が発火・Balloon が非発火。
#[test]
fn lottery_consumes_shell_before_balloon() {
    let shell_t = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let balloon_t = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
    // 消費列 [0, 1]: Shell(先) 発火・Balloon(後) 非発火。
    let (rng, probe) = counting_rng(&[0, 1]);
    let scope = ActorKey::from("0");
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: shell_t,
        balloon_tables: BTreeMap::from([(scope.clone(), balloon_t)]),
        rng,
        pick_rng: crate::seeded_rng(0),
        text_routing: crate::TextRouting::default(),
    });
    let mut states = crate::test_support::empty_states();
    states.apply(&scope, SurfaceTarget::Show(10));
    states.apply_balloon(&scope, SurfaceTarget::Show(20));

    rt.on_tick(at_ms(0), &mut states);
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert_eq!(probe.lock().unwrap().calls, 2);
    assert_eq!(cmds.len(), 1, "Shell のみ発火");
    assert!(
        matches!(&cmds[0], DisplayCommand::Show { .. }),
        "発火したのは Shell（Show）＝Shell が Balloon より先に消費（D-7）"
    );
}

// ── scope キー表引き（バルーン表） ──────────────────────

/// バルーン表は scope ごとに独立に引かれる。同一 surface id・同一 animation id でも、
/// scope 0 は自 scope の表のコマ、scope 1 は自 scope の表のコマで駆動される
/// （ある scope が別 scope の系列由来の定義で駆動されないことの反証）。
#[test]
fn balloon_tables_are_looked_up_per_scope() {
    // 同じ surface 20・同じ anim id 1 でありながら、コマの surface_id が表ごとに異なる。
    let t0 = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
    let t1 = table_single(20, 1, Interval::Random { k: 4 }, &[(4000, 0)]);
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(ActorKey::from("0"), t0), (ActorKey::from("1"), t1)]),
        rng: always_fire(),
        pick_rng: crate::seeded_rng(0),
        text_routing: crate::TextRouting::default(),
    });
    let mut states = crate::test_support::empty_states();
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");
    states.apply_balloon(&s0, SurfaceTarget::Show(20));
    states.apply_balloon(&s1, SurfaceTarget::Show(20));

    rt.on_tick(at_ms(0), &mut states);
    let cmds = rt.on_tick(at_ms(1000), &mut states);
    assert_eq!(
        cmds.len(),
        2,
        "両 scope が自 scope の表で発火（scope 昇順）"
    );
    match &cmds[0] {
        DisplayCommand::ShowBalloon { scope, pattern, .. } => {
            assert_eq!(scope, &s0);
            assert_eq!(
                pattern.get(1).expect("scope0 anim1 の現在コマ").surface_id,
                3000,
                "scope0 は scope0 の表のコマ（要件 5.6）"
            );
        }
        other => panic!("ShowBalloon を期待: {other:?}"),
    }
    match &cmds[1] {
        DisplayCommand::ShowBalloon { scope, pattern, .. } => {
            assert_eq!(scope, &s1);
            assert_eq!(
                pattern.get(1).expect("scope1 anim1 の現在コマ").surface_id,
                4000,
                "scope1 は scope1 の表のコマ＝scope0 の表とは独立に引かれる（要件 5.6）"
            );
        }
        other => panic!("ShowBalloon を期待: {other:?}"),
    }
}

/// 表を持たない scope のバルーンは**空表意味論＝不活性**（抽選対象ゼロ・**乱数非消費**・panic なし）。他 scope が表を持っていても、その表が不在 scope へ流用されることはない。
#[test]
fn absent_scope_balloon_table_is_inert() {
    // 表は scope "0" だけが持ち、表示するのは表を持たない scope "1" のバルーンのみ。
    let t0 = table_single(20, 1, Interval::Random { k: 4 }, &[(3000, 0)]);
    let (rng, probe) = counting_rng(&[0, 0]); // もし引かれれば発火してしまう値
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: AnimationTable::empty(),
        balloon_tables: BTreeMap::from([(ActorKey::from("0"), t0)]),
        rng,
        pick_rng: crate::seeded_rng(0),
        text_routing: crate::TextRouting::default(),
    });
    let mut states = crate::test_support::empty_states();
    let s1 = ActorKey::from("1");
    states.apply_balloon(&s1, SurfaceTarget::Show(20));

    rt.on_tick(at_ms(0), &mut states);
    assert!(
        rt.on_tick(at_ms(1000), &mut states).is_empty(),
        "不在 scope は抽選対象ゼロ＝無発行（要件 5.6）"
    );
    assert_eq!(
        probe.lock().unwrap().calls,
        0,
        "不在 scope は乱数を消費しない（空表意味論・要件 5.6）"
    );
    // さらに境界を跨いでも不活性のまま（panic せず無発行・非消費）。
    assert!(rt.on_tick(at_ms(2000), &mut states).is_empty());
    assert_eq!(probe.lock().unwrap().calls, 0);
}
