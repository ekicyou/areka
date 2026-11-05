use super::bind_loop_tests::arm_bind_resolver;
use super::test_support::*;
use super::*;
use crate::looper::tests::{
    always_fire, cfg, pat, shell_table, shown_shell_with_binds, surface_with, table_single,
};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use crate::timeline::LoopRng;
use areka_emo_compose::BindSet;
use areka_parsers::shell::{Animation, Interval};

// 刻みの道の役者テスト（spec: audit-seriko 指摘 2・actor_bind_loop_tests.rs から 1,000 行を越えないよう移した）。
// 前半は bind 消費分岐の網羅檻の最後の組（宛名規律）、後半は Tick 経路の網羅檻。

/// ケース23（D8④・宛名規律）: 非正準 params の Custom で宛名が自分（"bind"）＝warn! ；
/// 宛名が他人（"noexist"）＝warn! を出さず debug! 素通し。いずれも発行なし。
#[test]
fn bind_noncanonical_addressee_severity_split() {
    // (a) 宛名 "bind"（自分宛の壊れ物）→ warn!。
    {
        let resolver = tiny_resolver();
        let bind_resolver = arm_bind_resolver();
        let mut states = fresh_states();
        let mut out = MockSurfaceOutput::new();
        let mut loop_runtime = inert_runtime();
        let records = out.records();

        let flow = capture_logs_flow(|| {
            handle_message(
                &resolver,
                &bind_resolver,
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Cue(noncanonical_custom_cue("0", "bind")),
            )
        });

        assert_eq!(flow.1, ControlFlow::Continue(()));
        assert_eq!(
            flow.0.matches("level=WARN").count(),
            1,
            "宛名 bind の非正準 params は warn! を 1 回残す（自分宛の壊れ物・D8④）: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "非正準 params では発行しない"
        );
    }
    // (b) 宛名 "noexist"（他人宛/未知名）→ warn! を出さず debug! 素通し。
    {
        let resolver = tiny_resolver();
        let bind_resolver = arm_bind_resolver();
        let mut states = fresh_states();
        let mut out = MockSurfaceOutput::new();
        let mut loop_runtime = inert_runtime();
        let records = out.records();

        let flow = capture_logs_flow(|| {
            handle_message(
                &resolver,
                &bind_resolver,
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Cue(noncanonical_custom_cue("0", "noexist")),
            )
        });

        assert_eq!(flow.1, ControlFlow::Continue(()));
        assert_eq!(
            flow.0.matches("level=WARN").count(),
            0,
            "他人宛/未知名の非正準 params は warn! を出さない（報告責任は宛名の担当者・D8④）: {}",
            flow.0
        );
        assert!(
            flow.0.contains("level=DEBUG"),
            "他人宛/未知名は良性 debug! として観測できる（D8④）: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "非正準 params では発行しない"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tick 経路（SerikoMsg::Tick・send_tick・handle_message Tick 腕・on_surface_changed
// 連動）の網羅檻（R1.1/6.1/6.3/7.1/7.5/8.3）。
//
// 檻の要点:
// - (A) 表示中 slot が 1 つもない Tick は完全 no-op（無発行）——on_tick が空を返す 2.1 表示中
//       ゲートの自然帰結。
// - (B) live な loop_config（実表＋注入 rng）＋表示中 slot で境界跨ぎ Tick が pattern を載せた
//       Show を**既存 emit_display 単一発行点**から発行する（handle_message→on_tick→emit_display
//       の end-to-end 配線・looper の層で on_tick をじかに呼ぶ檻とは別の配線証明）。
// - (C) アクター停止後の send_tick は debug!（shutdown 期待事象）で panic しない（7.5/6.3）。
// - (D) Emote の Changed で on_surface_changed が呼ばれ、切替後の Tick が旧アニメの残再生を
//       復活させない（playback リセット・R2.3 表示従属）。
//
// すべて Tick 直投函＋注入 rng で決定論（sleep 不使用・7.1）。表構築は looper.rs 檻と同じ
// from_world 実構築パスを通す。
// ─────────────────────────────────────────────────────────────────────

/// 1 度だけ発火し以後発火しない rng（境界 1 回だけ抽選を通す）。
fn fire_once() -> LoopRng {
    let mut calls: u32 = 0;
    Box::new(move |_bound: u32| {
        calls += 1;
        if calls == 1 { 0 } else { 1 }
    })
}

/// (A・R6.1/2.1) 表示中 slot が 1 つもない Tick は完全 no-op（無発行）。
///
/// live な表（常時発火 rng）でも Show 前は表示中 slot ゼロ＝on_tick は評価対象なし＝空を返す。
/// handle_message Tick 腕は空列を回すだけで emit_display を一度も呼ばない（単一発行点の
/// 発火ゼロ＝MockSurfaceOutput 記録ゼロ）。
#[test]
fn tick_with_no_shown_slot_is_complete_no_op() {
    let resolver = tiny_resolver();
    // 表は live（surface 10 に常時発火アニメ）だが、Show していない＝表示中 slot 皆無。
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let mut loop_runtime = LoopRuntime::new(cfg(table, always_fire()));
    let mut states = ScopeStates::new(crate::DefaultBinds::uniform(BindSet::from_ids([
        1100, 1207,
    ]))); // Show なし
    let mut out = MockSurfaceOutput::new();
    let records = out.records();

    // 起動 tick と境界跨ぎ tick の双方が完全 no-op（表示中 slot が無いため）。
    for now in [0_u64, 1000, 5000] {
        let flow = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Tick { now_ms: now },
        );
        assert_eq!(flow, ControlFlow::Continue(()), "Tick は常に処理継続");
    }
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "表示中 slot が 1 つもない Tick は完全 no-op（emit_display 単一発行点の発火ゼロ・R6.1/2.1）"
    );
}

/// (B・R1.1/6.3/7.1) 表示中 slot＋live config で境界跨ぎ Tick が pattern を載せた Show を
/// 既存 emit_display 単一発行点から発行する（handle_message→on_tick→emit_display の end-to-end）。
///
/// looper の層で on_tick をじかに呼ぶ檻と異なり、**Tick メッセージが handle_message を貫通**して発行に至る
/// 配線を固定する。注入 rng（always_fire）＋注入 Tick 列で決定論（sleep 不使用）。
#[test]
fn tick_boundary_cross_emits_show_carrying_pattern() {
    let resolver = tiny_resolver();
    let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
    let mut loop_runtime = LoopRuntime::new(cfg(table, always_fire()));
    let (mut states, scope) = shown_shell_with_binds(10, BindSet::from_ids([1100, 1207]));
    let mut out = MockSurfaceOutput::new();
    let records = out.records();

    // 起動 tick（遅延初期化・非跨ぎ）→ 無発行。
    let _ = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Tick { now_ms: 0 },
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "起動 tick は境界を跨がず無発行"
    );

    // 境界跨ぎ tick（1000）→ 抽選発火＋先頭コマで Show を発行。
    let flow = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Tick { now_ms: 1000 },
    );
    assert_eq!(flow, ControlFlow::Continue(()), "Tick は処理継続");

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(recorded.len(), 1, "境界跨ぎ Tick で Show ちょうど 1 件");
    match &recorded[0] {
        DisplayCommand::Show {
            scope: s,
            surface_id,
            pattern,
            binds,
        } => {
            assert_eq!(s, &scope, "表示中 scope の Show");
            assert_eq!(*surface_id, 10, "表示中 surface は 10");
            assert_eq!(
                binds,
                &BindSet::from_ids([1100, 1207]),
                "刻みの Show は今の着せ替えを載せる"
            );
            let f = pattern.get(0).expect("anim0 の現在コマが pattern に載る");
            assert_eq!(
                f.surface_id, 2106,
                "先頭コマ 2106 が Tick 経由で単一発行点から発行される（配線証明・R6.3）"
            );
        }
        other => panic!("Show を期待（Tick→on_tick→emit_display の end-to-end）: {other:?}"),
    }
}

/// (C・R7.5/6.3) アクター停止後の send_tick は debug! を残し panic しない（shutdown 期待事象）。
///
/// spawn_seriko→close→join でアクター完全停止（inbox 受信端消失）→その後テストスレッドで
/// send_tick を capture_text 直下で呼ぶ。send 失敗が debug!（error! でない）として観測でき、
/// send_tick が panic せず戻る（本行到達＝処理系が異常終了しない）ことを固定する。
#[test]
fn send_tick_after_actor_stopped_logs_debug_no_panic() {
    let out = MockSurfaceOutput::new();
    let (sink, handle) = spawn_seriko(
        tiny_resolver(),
        crate::DefaultBinds::uniform(BindSet::from_ids([1100, 1207])),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let logs = capture_text(|| {
        sink.send_tick(1234); // 停止後の tick 送出。
    });
    assert!(
        logs.contains("level=DEBUG"),
        "停止後の send_tick が送出失敗を debug! で残すこと（shutdown 期待事象・R7.5）: {logs}"
    );
    assert_eq!(
        logs.matches("level=ERROR").count(),
        0,
        "停止後の send_tick は error! を出さない（PresentBridge 先例・shutdown 期待事象）: {logs}"
    );
    assert!(
        logs.contains("target=areka_seriko"),
        "本クレート target で発火すること: {logs}"
    );
    // panic せず本行へ到達したこと自体が「処理系が異常終了しない」証跡（R6.3）。
}

/// (D・R2.3・8.3) Emote の Changed で on_surface_changed が呼ばれ、面切替後の Tick が旧アニメの
/// 残再生を復活させない（playback リセット）。
///
/// 同一 anim id 0 を surface10（コマ 500/501）と surface20（コマ 700/701）が持つ表で、
/// surface10 で再生開始（started_at=1000）→ Emote で surface20 へ切替 → 境界を跨がない後続 Tick。
/// on_surface_changed が playback を除去するため、後続 Tick は「継続再生」を surface20 上へ
/// 復活させず無発行。**リセットが無ければ** elapsed 継続で surface20 のコマ 700 が発行されてしまう
/// （本テストはその差分を檻に入れる）。
#[test]
fn emote_surface_change_resets_loop_playback() {
    let resolver = tiny_resolver(); // 数値 key は直接解決（"20"→Show(20)）。
    let table = shell_table(vec![
        surface_with(
            10,
            vec![Animation {
                id: 0,
                interval: Interval::Random { k: 4 },
                interval_written: true,
                patterns: vec![pat(0, 500, 0), pat(1, 501, 100)],
            }],
        ),
        surface_with(
            20,
            vec![Animation {
                id: 0,
                interval: Interval::Random { k: 4 },
                interval_written: true,
                patterns: vec![pat(0, 700, 0), pat(1, 701, 100)],
            }],
        ),
    ]);
    // 境界 1 回だけ発火（切替後の非跨ぎ Tick では再抽選しない）。
    let mut loop_runtime = LoopRuntime::new(cfg(table, fire_once()));
    let (mut states, _scope) = shown_shell_with_binds(10, BindSet::from_ids([1100, 1207]));
    let mut out = MockSurfaceOutput::new();
    let records = out.records();

    // 起動 tick → 境界跨ぎ tick（1000）で surface10 の anim0 が発火・elapsed0→コマ 500。
    let _ = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Tick { now_ms: 0 },
    );
    let _ = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Tick { now_ms: 1000 },
    );

    // Emote で surface20 へ切替（apply Changed → emit Show{20,空} → on_surface_changed で playback 除去）。
    let _ = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(emote_cue(1.0, "0", "20")),
    );

    // 境界を跨がない後続 tick（1050・次境界は 2000）。playback がリセット済みなら無発行。
    let _ = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Tick { now_ms: 1050 },
    );

    let recorded = records.lock().expect("records mutex poisoned");
    // 発行列: [Show{10,コマ500}, Show{20,空}]。切替後 Tick は無発行（旧再生を復活させない）。
    assert_eq!(
        recorded.len(),
        2,
        "切替後の非跨ぎ Tick は旧アニメの残再生を復活させない（playback リセット・R2.3）: {recorded:?}"
    );
    match &recorded[0] {
        DisplayCommand::Show {
            surface_id,
            pattern,
            ..
        } => {
            assert_eq!(*surface_id, 10, "1 件目は surface10 の発火");
            assert_eq!(
                pattern.get(0).expect("コマ").surface_id,
                500,
                "surface10 の先頭コマ 500"
            );
        }
        other => panic!("Show を期待: {other:?}"),
    }
    match &recorded[1] {
        DisplayCommand::Show {
            surface_id,
            pattern,
            ..
        } => {
            assert_eq!(*surface_id, 20, "2 件目は surface20 への切替");
            assert!(
                pattern.get(0).is_none(),
                "面切替の Show は空 pattern（旧コマも新残再生も載らない・R2.3/8.3）"
            );
        }
        other => panic!("Show を期待: {other:?}"),
    }
}
