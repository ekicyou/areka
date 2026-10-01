//! ループの表の差し替え（`replace_shell_table`／`replace_balloon_tables`）の決定論テスト
//! （要件 2.8・3.5）。

use super::tests::{always_fire, cfg, pattern_of, table_single};
use super::*;
use crate::resolve::SurfaceTarget;
use areka_emo_compose::BindSet;
use areka_parsers::shell::Interval;

/// シェルの表の差し替え: 再生中のループを捨て、古い表のコマは以後 0 件。次の抽選から
/// 新しい表で始め直す（要件 2.8）。
#[test]
fn replace_shell_table_drops_playing_loop_and_restarts_from_new_table() {
    // 古い表: 2106 → 150ms 後に 2110（再生が続く）。
    let old = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
    let mut rt = LoopRuntime::new(cfg(old, always_fire()));
    let mut states = ScopeStates::new(BindSet::from_ids([]));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(10));

    rt.on_tick(0, &mut states);
    let c1 = rt.on_tick(1000, &mut states); // 古い表で発火・先頭コマ 2106
    assert_eq!(pattern_of(&c1[0]).get(0).unwrap().surface_id, 2106);

    // 差し替え（アクターの順: 状態 → 表）。
    states.rebase_shell(BindSet::from_ids([]));
    let new = table_single(10, 0, Interval::Random { k: 4 }, &[(3000, 0)]);
    rt.replace_shell_table(new);

    // 古い表なら 2110 が出る時刻でも、何も出ない。
    assert!(
        rt.on_tick(1200, &mut states).is_empty(),
        "古い表のループの指令は差し替えの後に 0 件"
    );
    // 次の境界で新しい表から始め直す。
    let c2 = rt.on_tick(2000, &mut states);
    assert_eq!(c2.len(), 1, "新しい表で発火");
    let p = pattern_of(&c2[0]);
    assert_eq!(p.get(0).unwrap().surface_id, 3000, "新しい表のコマ");
}

/// バルーンの表の差し替え: バルーン側の再生だけを捨て、シェル側のループは続く（要件 3.5）。
#[test]
fn replace_balloon_tables_keeps_shell_loop() {
    let shell = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0), (2110, 150)]);
    let balloon = table_single(20, 1, Interval::Random { k: 4 }, &[(30, 0), (31, 150)]);
    let scope = ActorKey::from("0");
    let mut rt = LoopRuntime::new(SerikoLoopConfig {
        shell_table: shell,
        balloon_tables: BTreeMap::from([(scope.clone(), balloon)]),
        rng: always_fire(),
    });
    let mut states = ScopeStates::new(BindSet::from_ids([]));
    states.apply(&scope, SurfaceTarget::Show(10));
    states.apply_balloon(&scope, SurfaceTarget::Show(20));

    rt.on_tick(0, &mut states);
    assert_eq!(rt.on_tick(1000, &mut states).len(), 2, "両 slot が発火");

    // 新しいバルーンの表: 同じ scope・同じ面 20・同じ anim id 1 でコマだけが違う。古い再生が
    // 残っていれば、同じ id の新しいコマ 50 が 1200 の時点で出てしまう（再生を捨てたことの檻）。
    states.rebase_balloon();
    let new_balloon = table_single(20, 1, Interval::Random { k: 4 }, &[(50, 0)]);
    rt.replace_balloon_tables(BTreeMap::from([(scope.clone(), new_balloon)]));

    // (a) 境界の前: シェルだけが次のコマ 2110 へ進み、バルーンからは何も出ない。
    let cmds = rt.on_tick(1200, &mut states);
    assert_eq!(cmds.len(), 1, "シェル側だけが進む: {cmds:?}");
    match &cmds[0] {
        DisplayCommand::Show { pattern, .. } => {
            assert_eq!(pattern.get(0).unwrap().surface_id, 2110);
        }
        other => panic!("シェルの Show を期待: {other:?}"),
    }

    // (b)(c) 次の境界: シェルは今までどおり再抽選で 2106 から、バルーンは新しい表の 50 で始まる。
    let cmds = rt.on_tick(2000, &mut states);
    assert_eq!(cmds.len(), 2, "シェルとバルーンが発火: {cmds:?}");
    match &cmds[0] {
        DisplayCommand::Show { pattern, .. } => {
            assert_eq!(
                pattern.get(0).unwrap().surface_id,
                2106,
                "シェルのループは続く"
            );
        }
        other => panic!("シェルの Show を期待: {other:?}"),
    }
    match &cmds[1] {
        DisplayCommand::ShowBalloon { pattern, .. } => {
            assert_eq!(
                pattern.get(1).unwrap().surface_id,
                50,
                "新しいバルーンの表のコマ"
            );
        }
        other => panic!("バルーンの ShowBalloon を期待: {other:?}"),
    }
}
