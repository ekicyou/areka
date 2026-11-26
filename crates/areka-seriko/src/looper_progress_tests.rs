//! ループ統括器の進行相の bind 判定（再生中に外れた ID の停止）の決定論テスト。

use super::tests::*;
use super::*;
use crate::test_support::capture_text;
use areka_emo_compose::BindSet;
use areka_parsers::shell::{Animation, Interval};
use dola::time::testing::at_ms;

// ── 進行相の bind 判定（再生中に外れた ID の停止） ──

/// 再生**途中**（末尾未到達）に bind から外した ID は、次の評価でコマも再生も消え、以後**復活しない**。
///
/// 実機の固着そのものの機構: 状態側の除去（`commit_bind` の `drop_residual_frames`）だけでは
/// 「再生中に外れた」場合を塞げない——playback が残る限り次 tick の進行相が `frame_at` の結果でコマを
/// 置き直すため。進行相にも bind 判定が要る。
#[test]
fn playing_bind_anim_removed_from_binds_stops_and_does_not_revive() {
    // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。elapsed<150 は Active（末尾未到達＝再生途中）。
    let table = table_single(
        10,
        7,
        Interval::BindRandom { k: 4 },
        &[(1412, 0), (1411, 150), (1410, 22)],
    );
    let (rng, probe) = counting_rng(&[0, 0]);
    let mut rt = LoopRuntime::new(cfg(table, rng));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(at_ms(0), &mut states);
    let c1 = rt.on_tick(at_ms(1000), &mut states); // 発火 → 先頭コマ 1412（再生中）
    assert_eq!(
        pattern_of(&c1[0]).get(7).unwrap().surface_id,
        1412,
        "再生中の先頭コマ"
    );

    // 再生途中で bind から外す（状態側の除去が保持コマをここで取り除く）。
    states.apply_bind(&scope, 7, false);
    assert!(
        states.current_pattern(&scope, Slot::Shell).get(7).is_none(),
        "状態側の除去（bindopt 7.1）で保持コマは一旦消える"
    );

    // 次の評価（境界を跨がない tick でも進行相は走る）。進行相に bind 判定が無いとここで復活する。
    let c2 = rt.on_tick(at_ms(1100), &mut states);
    assert!(
        states.current_pattern(&scope, Slot::Shell).get(7).is_none(),
        "bind から外れた ID のコマは次の評価で復活しない（bindopt 7.3）"
    );
    assert!(
        c2.is_empty(),
        "復活しない＝pattern 不変ゆえ無発行（要件 6.2）"
    );
    assert!(
        rt.playback
            .get(&(scope.clone(), Slot::Shell))
            .is_none_or(|pb| !pb.contains_key(&7)),
        "bind から外れた ID の再生も捨てる（bindopt 7.3）"
    );

    // bind を戻すと次の境界で再び抽選の対象になる（2000 では経過 1000 で末尾を過ぎるので、
    // 再生が残っていても引く。再生を捨てたことは上の確かめが見る）。
    states.apply_bind(&scope, 7, true);
    rt.on_tick(at_ms(2000), &mut states);
    assert_eq!(
        probe.lock().unwrap().calls,
        2,
        "bind を戻すと再抽選の対象へ戻る（bindopt 7.3）"
    );
}

/// bind 種でないアニメ（純 `Random`）は進行相の bind 判定の影響を受けない。
///
/// 同一 surface に bind 種（id 7・BindRandom）と非 bind 種（id 9000・Random）を並べ、id 7 だけを
/// bind から外す。id 7 は停止するが、id 9000 は再生を続け末尾残留コマまで到達する。
#[test]
fn non_bind_anim_is_unaffected_by_progress_phase_bind_check() {
    let s = surface_with(
        10,
        vec![
            Animation {
                id: 7,
                interval: Interval::BindRandom { k: 4 },
                // 1412(w0)/1411(w150)/1410(w22) ⇒ t=[0,150,172]。
                patterns: vec![pat(0, 1412, 0), pat(1, 1411, 150), pat(2, 1410, 22)],
            },
            Animation {
                id: 9000,
                interval: Interval::Random { k: 4 },
                // 2106(w0)/2110(w150) ⇒ t=[0,150]。末尾非負＝到達後も残留する。
                patterns: vec![pat(0, 2106, 0), pat(1, 2110, 150)],
            },
        ],
    );
    let mut rt = LoopRuntime::new(cfg(shell_table(vec![s]), always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(at_ms(0), &mut states);
    let c1 = rt.on_tick(at_ms(1000), &mut states); // 両方発火（id 7 は bind ON・id 9000 は無条件）
    let p1 = pattern_of(&c1[0]);
    assert_eq!(p1.get(7).unwrap().surface_id, 1412);
    assert_eq!(p1.get(9000).unwrap().surface_id, 2106);

    // bind 種の id 7 だけを外す（id 9000 は bind 集合に属さないアニメ）。
    states.apply_bind(&scope, 7, false);

    // 経過 200ms（1200）: id 7 は停止、id 9000 は末尾 2110 へ進み残留する。
    let c2 = rt.on_tick(at_ms(1200), &mut states);
    assert_eq!(c2.len(), 1, "id 9000 のコマ進行で 1 指令");
    let p2 = pattern_of(&c2[0]);
    assert!(
        p2.get(7).is_none(),
        "bind から外れた bind 種は復活しない（bindopt 7.3）"
    );
    assert_eq!(
        p2.get(9000).unwrap().surface_id,
        2110,
        "bind 非所属アニメは進行相の bind 判定に影響されない（bindopt 7.4）"
    );

    // さらに時間が進んでも id 9000 の残留は保たれる（末尾非負＝IdleResidual）。
    assert!(
        rt.on_tick(at_ms(1500), &mut states).is_empty(),
        "残留は恒久＝無発行"
    );
    assert_eq!(
        states
            .current_pattern(&scope, Slot::Shell)
            .get(9000)
            .unwrap()
            .surface_id,
        2110,
        "bind 非所属アニメの保持コマは除去されない（bindopt 7.4）"
    );
}

/// 進行相の停止は `info!` で痕跡を残す（無言の状態変更を作らない）。文言と水準を固定する。
#[test]
fn progress_phase_bind_drop_emits_info_marker() {
    let table = table_single(
        10,
        7,
        Interval::BindRandom { k: 4 },
        &[(1412, 0), (1411, 150), (1410, 22)],
    );
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(at_ms(0), &mut states);
    rt.on_tick(at_ms(1000), &mut states); // 発火・再生中
    states.apply_bind(&scope, 7, false);

    let logs = capture_text(|| {
        rt.on_tick(at_ms(1100), &mut states);
    });

    assert!(
        logs.contains("level=INFO")
            && logs.contains("seriko: loop bind から外れた ID の再生を停止"),
        "進行相の停止は実機の既定ログ水準（info）で grep 可能な固定文言を残す（bindopt 7.5）: {logs}"
    );
    assert!(
        logs.contains("animation_id=7"),
        "停止した ID を痕跡に載せる（bindopt 7.5）: {logs}"
    );
}

/// bind 種でも bind 集合に属したまま再生が続く間は停止しない（誤検出の不在）。
///
/// **不在主張には同一捕捉内の陽性対照を併置する**（`actor_dispatch_tests` と同じ形）。捕捉区間に
/// 抽選境界を跨ぐ tick を含め、健全系では `loop 抽選発火` の痕跡が**必ず出る**ことを同時に主張する
/// ——捕捉が空でも通ってしまう恒真の檻にしないため。あわせて状態側（保持コマの残存）でも反証し、
/// ログ経路が壊れても判別能力が残るようにする。進行相のガード条件を反転させる変異
/// （`!contains` → `contains`）では、発火直後の進行相で id 7 が落とされ、停止痕跡の出現と
/// コマの消失の**両方**でこの檻が赤になる。
#[test]
fn progress_phase_emits_no_drop_marker_while_bind_holds() {
    let table = table_single(
        10,
        7,
        Interval::BindRandom { k: 4 },
        &[(1412, 0), (1411, 150), (1410, 22)],
    );
    let mut rt = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([7]));

    rt.on_tick(at_ms(0), &mut states);
    // 捕捉区間に境界跨ぎ（＝発火）を含める。bind は終始 ON のまま。
    let logs = capture_text(|| {
        rt.on_tick(at_ms(1000), &mut states); // 抽選発火 → 進行相（bind 所属のまま）
        rt.on_tick(at_ms(1100), &mut states); // 再生継続（Active・末尾未到達）
    });

    // 陽性対照: この捕捉には現に痕跡が出ている（空捕捉に対する恒真の不在主張を禁じる）。
    assert!(
        logs.contains("seriko: loop 抽選発火"),
        "陽性対照: bind ON の発火痕跡が同一捕捉に現れる（捕捉が空でないことの反証）: {logs}"
    );
    // 不在主張: bind 所属のまま再生中の ID は停止相当へ落ちない。
    assert!(
        !logs.contains("seriko: loop bind から外れた ID の再生を停止"),
        "bind 所属のまま再生中の ID は停止しない（bindopt 7.3 の裏側）: {logs}"
    );
    // 状態側の反証（ログ経路に依らない判別）: 再生中のコマが保たれている。
    assert_eq!(
        states
            .current_pattern(&scope, Slot::Shell)
            .get(7)
            .map(|f| f.surface_id),
        Some(1412),
        "bind 所属のまま再生は継続し保持コマも保たれる（bindopt 7.3 の裏側）"
    );
}
