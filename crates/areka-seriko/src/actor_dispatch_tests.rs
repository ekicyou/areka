use super::test_support::*;
use super::*;
use crate::output::{DisplayCommand, MockSurfaceOutput};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use dola::cue::CueSink;

/// 受信端が生きていれば `emit` が `SerikoMsg::Cue` を inbox へ橋渡しする（R1.1/1.2）。
/// trait 実装＝結線契約（追加の口を設けず単一の出力契約 `CueSink` 経由でのみ届く）。
#[test]
fn emit_forwards_cue_to_inbox() {
    let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
    let mut sink = SerikoSink::new(tx);

    CueSink::emit(&mut sink, emote_cue(1.5, "0", "smile"));

    match rx.recv() {
        Ok(SerikoMsg::Cue(cue)) => {
            assert_eq!(cue.at, 1.5, "at が保たれる");
            assert_eq!(cue.actor, ActorKey::from("0"), "actor が保たれる");
            assert!(
                matches!(cue.command, CueCommand::Emote { ref key } if key == "smile"),
                "Emote{{key}} が保たれる: {:?}",
                cue.command
            );
        }
        other => panic!("Cue が inbox へ届くこと: {other:?}"),
    }
}

/// コンパイル時検証: `SerikoSink` は cue 再生ランタイムへ登録可能な**単一の出力契約**
/// （`dola::cue::CueSink + Clone + Send + 'static`）を満たす（R11.6・task 4.1 の単一トレイト）。
/// トレイトはフルパス束縛で参照する（emo-text 6.1 の `assert_cue_sink_contract` と同型）。
fn assert_cue_sink_contract<T: dola::cue::CueSink + Clone + Send + 'static>() {}

#[test]
fn seriko_sink_satisfies_cue_sink_contract() {
    assert_cue_sink_contract::<SerikoSink>();
}

/// 単一の出力契約 [`dola::cue::CueSink`] の `emit` 経由で発火が `SerikoMsg::Cue` として
/// inbox へ無変形で橋渡しされる（送出本体 `deliver` を用いる・R11.6）。
#[test]
fn cue_sink_emit_forwards_cue_to_inbox() {
    let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
    let mut sink = SerikoSink::new(tx);

    // 担当外の Text cue（broadcast 受信）でも sink は無変形で inbox へ橋渡しする
    // （担当判定は handler の演者側 relevance の領分・sink は選別しない）。
    dola::cue::CueSink::emit(&mut sink, text_cue(2.5, "1", "アヒル"));

    match rx.recv() {
        Ok(SerikoMsg::Cue(cue)) => {
            assert_eq!(cue.at, 2.5, "at が保たれる");
            assert_eq!(cue.actor, ActorKey::from("1"), "actor が保たれる");
            assert!(
                matches!(cue.command, CueCommand::Text(ref t) if t == "アヒル"),
                "Text が保たれる: {:?}",
                cue.command
            );
        }
        other => panic!("CueSink::emit でも Cue が inbox へ届くこと: {other:?}"),
    }
}

/// 受信端（inbox/アクター）消失後に `emit` を呼んでも、`error!` ログを残しつつ
/// panic せず正常に戻る（infallible 契約・send 失敗を黙殺しない・R6.3/6.4）。
#[test]
fn emit_after_receiver_gone_logs_no_panic() {
    let (tx, rx) = std::sync::mpsc::channel::<SerikoMsg>();
    let mut sink = SerikoSink::new(tx);
    // 受信端を drop＝アクター停止後（inbox 全受信端消失）を模す。
    drop(rx);

    // emit は infallible。send 失敗経路（Err）を通り、panic せず戻ること自体が合格条件。
    let logs = capture_logs(|| {
        CueSink::emit(&mut sink, emote_cue(0.0, "0", "smile"));
    });

    // silent failure 禁止（R6.3）: send 失敗が error! として観測できること。
    assert!(
        logs.contains("level=ERROR"),
        "send 失敗が error! ログとして発火すること: {logs}"
    );
    assert!(
        logs.contains("target=areka_seriko"),
        "本クレート target で発火すること: {logs}"
    );
}

/// 単発シナリオ（本タスクの主 observable・R1.3/5.3）: 単純な発火 1 件を入力すると、
/// 期待どおりの表示指令 1 件が観測用出力先へちょうど記録される。
///
/// 解決層（2.1）・状態層（2.2）・発行層（2.3）を独立スレッド上で一本の経路で結び、
/// `emit_display` 単一発行点から発行されることを end-to-end で固定する。同期は
/// `Close`→`ActorHandle::join`（Break 後のスレッド終了待ち）で決定論的に行い、sleep を
/// 一切用いない（先に送った Cue は FIFO 単一スレッドゆえ join 復帰時に処理済み）。
#[test]
fn single_cue_emits_one_display_command() {
    use crate::resolve::SurfaceResolver;
    use areka_emo_compose::BindSet;
    use std::collections::BTreeMap;

    // 解決表（"通常"→2100 の 1 件だけ持つ小さな alias 表）と静的 bind 集合。
    let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    aliases.insert("通常".to_string(), vec![2100]);
    let resolver = SurfaceResolver::new(aliases);
    let binds = BindSet::from_ids([1100, 1207]);

    // 観測用出力先。records() ハンドルを move 前に取得しておく。
    let out = MockSurfaceOutput::new();
    let records = out.records();

    // アクター起動→単純な Shell 系 Emote 1 件を emit→Close→join で終了同期。
    let (mut sink, handle) = spawn_seriko(
        resolver,
        binds.clone(),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );
    CueSink::emit(&mut sink, emote_cue(0.0, "0", "2100"));
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    // 表示指令がちょうど 1 件、期待値どおり記録されていること（全値比較）。
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(recorded.len(), 1, "単発発火で表示指令はちょうど 1 件");
    assert_eq!(
        recorded[0],
        DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207]),
            pattern: PatternState::default(),
        },
        "解決→状態確定→単一発行点発行の一本経路の結果が期待どおり"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Task 4.1: 停止経路・異常系の網羅テスト（1.4/2.1/2.4/6.2/6.3/6.4/7.4）。
//
// 設計要点（cross-thread log の決定論化）:
// - handler スレッドで発火する log（Unresolved の error!／非 Shell・EntityRef の warn!）は
//   `spawn_seriko` の別スレッドで出るため、テストスレッドに張った thread-local
//   `capture_logs` では捕捉できない。よってその種の log 表明は `handle_message` を
//   **テストスレッド上で同期呼び出し**して `capture_logs` 直下で発火させる。
// - Close／disconnect／emit-after-stop の lifecycle は `spawn_seriko`＋`join()` を
//   唯一の同期点として決定論化する（sleep/polling 不使用）。emit-after-stop の error! は
//   `join()` 後にテストスレッドで `emit` を呼ぶため thread-local `capture_logs` で捕捉できる。
// ─────────────────────────────────────────────────────────────────────

use areka_emo_compose::{BindSet, PatternState};

/// EntityRef 系の TalkCue（Shell 分類だが M-boot 未対応の防御枝・6.2）を組む。
fn entityref_cue(at: f64, scope: &str, entity: u64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::EntityRef(entity),
        duration: 0.0,
    }
}

/// バルーン面切替の TalkCue（`\b[key]` 相当・Shell 分類だが早期分岐で消費・4.1–4.5）を組む。
fn balloon_cue(at: f64, scope: &str, key: &str) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::BalloonSurface { key: key.into() },
        duration: 0.0,
    }
}

/// 非 Shell（Balloon 担当＝emo-text 行き）の TalkCue（`Text`）を組む——broadcast 受信の担当外例（6.2）。
fn text_cue(at: f64, scope: &str, text: &str) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::Text(text.into()),
        duration: 0.0,
    }
}

/// 純粋な待ち（どの演者の担当でもない・action なし・duration のみ）の TalkCue（`Wait`）を組む。
fn wait_cue(at: f64, scope: &str, duration: f64) -> TalkCue {
    TalkCue {
        at,
        actor: ActorKey::from(scope),
        command: CueCommand::Wait,
        duration,
    }
}

/// ケース1（1.4/7.4）: 停止指令 Close による正常終了を独立実行テストで確認する。
///
/// `spawn_seriko`→`close()`→`join()` で `Break` 経由の正常終了（`Ok`）を決定論的に固定する。
/// 同期は `join()` の一点のみ・sleep/polling なし。3.2 の複合シナリオとは別に、
/// 「Close による正常終了」だけを主張する専用テスト。
#[test]
fn close_stops_normally() {
    let out = MockSurfaceOutput::new();
    let (sink, handle) = spawn_seriko(
        tiny_resolver(),
        BindSet::from_ids([1100, 1207]),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );

    sink.close().expect("Close を送れること");
    // Break→スレッド正常終了。panic なし＝Ok。
    assert!(
        handle.join().is_ok(),
        "Close 受領でアクターが正常終了する（Break 経路・1.4）"
    );
}

/// ケース2（1.4）: 送信端が全て失われた場合（disconnect）の正常終了を独立実行テストで確認する。
///
/// `spawn_seriko`→**全 Sender drop**（`SerikoSink` を drop・他に clone を残さない）→`join()`。
/// inbox の `RecvError` で `run_inbox` ループが正常終了することを固定する。Close を送らない
/// ことがこのテストの肝（disconnect 経路のみを主張）。sleep/polling なし。
#[test]
fn disconnect_stops_normally() {
    let out = MockSurfaceOutput::new();
    let (sink, handle) = spawn_seriko(
        tiny_resolver(),
        BindSet::from_ids([1100, 1207]),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );

    // 全送信端（唯一の SerikoSink＝唯一の Sender）を drop。Close は送らない。
    drop(sink);

    // 受信端の RecvError→ループ正常終了（Ok）。
    assert!(
        handle.join().is_ok(),
        "全 Sender drop（disconnect）でアクターが正常終了する（1.4）"
    );
}

/// ケース3（6.2/6.3/7.4）: 分類不能／防御枝の入力が記録を残して読み飛ばされ、処理が継続する。
///
/// (a) 同期 `handle_message`＋`capture_logs`（テストスレッド発火）で EntityRef 防御枝が
///     `warn!` を残し・状態不変・`Continue`（skip して継続）を決定論的に固定する。
/// (b) spawn 経由の observable: EntityRef cue を先に、有効 cue を後に emit→close→join し、
///     mock 記録が有効 Show 1 件のみ＝bad cue は skip され、その後もアクターが処理継続したこと
///     を発行列で示す（cross-thread log には依存しない observable 檻）。
#[test]
fn unclassifiable_input_is_logged_and_skipped_then_continues() {
    // (a) 同期 handler: warn! 発火＋状態不変＋Continue をテストスレッドで捕捉。
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(entityref_cue(0.0, "0", 42)),
        )
    });

    assert_eq!(
        flow.1,
        ControlFlow::Continue(()),
        "分類不能／防御枝は Continue で処理継続する（6.2）"
    );
    assert!(
        flow.0.contains("level=WARN"),
        "EntityRef 防御枝が warn! を残すこと（silent failure 禁止・6.3）: {}",
        flow.0
    );
    assert!(
        flow.0.contains("target=areka_seriko"),
        "本クレート target で発火すること: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "分類不能入力では発行しない（状態不変・skip）"
    );

    // (b) spawn 経由 observable: 悪 cue→有効 cue→close→join、記録は有効 Show のみ。
    let out2 = MockSurfaceOutput::new();
    let records2 = out2.records();
    let (mut sink, handle) = spawn_seriko(
        tiny_resolver(),
        BindSet::from_ids([1100, 1207]),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out2,
    );
    CueSink::emit(&mut sink, entityref_cue(0.0, "0", 7)); // 防御枝＝skip
    CueSink::emit(&mut sink, emote_cue(1.0, "0", "2100")); // 有効
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let recorded = records2.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207]),
            pattern: PatternState::default(),
        }],
        "悪 cue は skip され、後続有効 cue のみ発行＝ループ継続の observable（6.2）"
    );
}

/// ケース4（2.1/2.4/7.4）: 数値解釈の境界値が解決不能として扱われる（handler 経路＋解決失敗ログ）。
///
/// 非表示指定以外の負値（`"-2"`）と許容範囲超（`"4294967296"`＝`u32::MAX+1`）を
/// 同期 `handle_message`＋`capture_logs`（テストスレッド発火）で流し、`Unresolved` の
/// `error!` が残り・状態不変・発行なし・`Continue` を決定論的に固定する。解決層単体は
/// resolve.rs の 2.1 で檻済み。ここは **アクター handler 経路の解決失敗ログ**（7.4）を実行檻化。
#[test]
fn numeric_boundary_is_unresolved() {
    for bad in ["-2", "4294967296"] {
        let resolver = tiny_resolver();
        let mut states = fresh_states();
        let mut out = MockSurfaceOutput::new();
        let mut loop_runtime = inert_runtime();
        let records = out.records();

        let flow = capture_logs_flow(|| {
            handle_message(
                &resolver,
                &BindResolver::empty(),
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Cue(emote_cue(0.0, "0", bad)),
            )
        });

        assert_eq!(
            flow.1,
            ControlFlow::Continue(()),
            "境界値 {bad:?} は解決不能として skip し処理継続（2.4/6.1）"
        );
        assert!(
            flow.0.contains("level=ERROR"),
            "境界値 {bad:?} の解決失敗が error! を残すこと（7.4）: {}",
            flow.0
        );
        assert!(
            flow.0.contains("target=areka_seriko"),
            "本クレート target で発火すること: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "境界値 {bad:?} では発行しない（状態不変）"
        );
    }
}

/// ケース5（6.3/6.4）: アクター停止後の emit がログのみ残し panic せず全体を異常終了させない。
///
/// design の emit-after-stop シナリオ（`emit_after_receiver_gone` を **実アクター停止**の
/// setup で行う）: `spawn_seriko`→`close()`→`join()`（アクター完全停止＝inbox 受信端消失）→
/// その後テストスレッドで `emit` を `capture_logs` 直下で呼ぶ。send 失敗が `error!` として
/// 観測でき、かつ `emit` が panic せず戻る（＝処理系全体が異常終了しない）ことを固定する。
/// emit-after-stop の error! はテストスレッドで発火するため thread-local `capture_logs` で捕捉可。
#[test]
fn emit_after_actor_stopped_logs_no_panic() {
    let out = MockSurfaceOutput::new();
    let (mut sink, handle) = spawn_seriko(
        tiny_resolver(),
        BindSet::from_ids([1100, 1207]),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );

    // アクターを完全停止させる（Break→スレッド終了→inbox 受信端消失）。
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    // 停止後の emit: send は Err だが panic せず error! を残して戻る（infallible 契約・6.3/6.4）。
    let logs = capture_logs(|| {
        CueSink::emit(&mut sink, emote_cue(0.0, "0", "2100"));
    });

    assert!(
        logs.contains("level=ERROR"),
        "停止後の emit が send 失敗を error! として残すこと（silent failure 禁止・6.3）: {logs}"
    );
    assert!(
        logs.contains("target=areka_seriko"),
        "本クレート target で発火すること: {logs}"
    );
    // panic せず本行へ到達したこと自体が「処理系全体が異常終了しない」証跡（6.4）。
}

// ─────────────────────────────────────────────────────────────────────
// Task 4.4: バルーン面切替コマンドの消費経路（早期分岐）の網羅テスト（4.1/4.2/4.3/4.5/4.6）。
//
// 早期分岐（key 抽出 match の前段）は解決→適用→発行を arm 内で完結し値を返さないため、
// 同期 `handle_message`＋`MockSurfaceOutput` の records 照合で発行を、`capture_logs_flow` で
// NameForm=warn!／Invalid=error! の severity split を、いずれもテストスレッド上で決定論的に
// 檻化する（cross-thread log 問題を回避）。既存 `emote_cue` 経路（シェル面）は無改変。
// ─────────────────────────────────────────────────────────────────────

/// ケース6（4.1）: 数値 key はバルーン面表示指令を単一発行点から発行する。
///
/// `BalloonSurface{key:"2"}` を同期 `handle_message` へ流すと、`ShowBalloon{scope, surface_id:2}`
/// がちょうど 1 件記録される（binds なし＝M-boot にバルーン着せ替え無し）。RED では BalloonSurface
/// が key 抽出 match の catch-all（warn!＋skip）へ落ち発行されないため本 assert が落ちる。
#[test]
fn balloon_numeric_key_emits_showballoon() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(balloon_cue(0.0, "0", "2")),
    );

    assert_eq!(flow, ControlFlow::Continue(()), "消費後も処理継続（4.1）");
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "数値 key は ShowBalloon をちょうど 1 件発行する（単一発行点共用・4.1）"
    );
}

/// ケース7（4.2）: `-1` はバルーン非表示指令を発行する。
#[test]
fn balloon_minus_one_emits_hideballoon() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(balloon_cue(0.0, "0", "-1")),
    );

    assert_eq!(flow, ControlFlow::Continue(()), "消費後も処理継続（4.2）");
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[DisplayCommand::HideBalloon {
            scope: ActorKey::from("0"),
        }],
        "-1 は HideBalloon をちょうど 1 件発行する（非表示センチネル・4.2）"
    );
}

/// ケース8（4.5・severity split）: 名前形 key は warn! を 1 回だけ残し発行しない。
///
/// `\b[バルーン１]` 相当の非数値 key は M-boot 未対応の**正当構文**＝`NameForm`。同期
/// `handle_message`＋`capture_logs_flow` で warn! が**1 回だけ**・error! は**0 回**・発行なし・
/// `Continue` を固定する。RED では catch-all の別文言 warn! に落ちるため文言 assert が落ちる。
#[test]
fn balloon_name_form_warns_once_no_emit() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(balloon_cue(0.0, "0", "バルーン１")),
        )
    });

    assert_eq!(
        flow.1,
        ControlFlow::Continue(()),
        "名前形は skip して処理継続（正当構文・4.5）"
    );
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        1,
        "名前形は warn! を 1 回だけ残す（4.5）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=ERROR").count(),
        0,
        "名前形は error! を残さない（NameForm=warn の severity split・4.5）: {}",
        flow.0
    );
    assert!(
        flow.0.contains("名前解決できず"),
        "バルーン面 key 固有の名前形メッセージであること（catch-all 汎用文言でない）: {}",
        flow.0
    );
    assert!(
        flow.0.contains("target=areka_seriko"),
        "本クレート target で発火すること: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "名前形では発行しない（発行なし・4.5）"
    );
}

/// ケース9（4.5・severity split）: 不正な数値 key は error! を 1 回だけ残し発行しない。
///
/// 負の非 `-1`（`-2`）と `u32` 超過（`4294967296`）は破損入力＝`Invalid`。シェル経路の
/// `Unresolved`→error! と同水準で、error! が**1 回だけ**・warn! は**0 回**・発行なし・`Continue`。
/// RED では catch-all の warn!（error! でない）に落ちるため error! カウント assert が落ちる。
#[test]
fn balloon_invalid_numeric_errors_once_no_emit() {
    for bad in ["-2", "4294967296"] {
        let resolver = tiny_resolver();
        let mut states = fresh_states();
        let mut out = MockSurfaceOutput::new();
        let mut loop_runtime = inert_runtime();
        let records = out.records();

        let flow = capture_logs_flow(|| {
            handle_message(
                &resolver,
                &BindResolver::empty(),
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Cue(balloon_cue(0.0, "0", bad)),
            )
        });

        assert_eq!(
            flow.1,
            ControlFlow::Continue(()),
            "不正数値 {bad:?} は skip して処理継続（破損入力・4.5）"
        );
        assert_eq!(
            flow.0.matches("level=ERROR").count(),
            1,
            "不正数値 {bad:?} は error! を 1 回だけ残す（シェル経路と同水準・4.5）: {}",
            flow.0
        );
        assert_eq!(
            flow.0.matches("level=WARN").count(),
            0,
            "不正数値 {bad:?} は warn! を残さない（Invalid=error の severity split・4.5）: {}",
            flow.0
        );
        assert!(
            flow.0.contains("不正な数値"),
            "破損数値固有のメッセージであること: {}",
            flow.0
        );
        assert!(
            flow.0.contains("target=areka_seriko"),
            "本クレート target で発火すること: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "不正数値 {bad:?} では発行しない（発行なし・4.5）"
        );
    }
}

/// ケース10（4.3・冪等）: 同一バルーン面への再指定は指令を再発行しない。
///
/// `\b[2]` を 2 回流すと、2 回目は状態不変ゆえ `ShowBalloon` が 1 件のみ記録される
/// （既存 per-scope 冪等ガードと同一挙動・単一発行点の冪等抑止）。
#[test]
fn balloon_same_surface_twice_emits_once() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let f1 = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(balloon_cue(0.0, "0", "2")),
    );
    let f2 = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(balloon_cue(1.0, "0", "2")),
    );

    assert_eq!(f1, ControlFlow::Continue(()));
    assert_eq!(f2, ControlFlow::Continue(()));
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[DisplayCommand::ShowBalloon {
            scope: ActorKey::from("0"),
            surface_id: 2,
            pattern: PatternState::default(),
        }],
        "同一面の再指定は再発行しない（冪等・4.3）"
    );
}

/// ケース11（4.6・独立性）: シェル面切替とバルーン面切替の混在で双方が独立に記録される。
///
/// `Emote{key:"2100"}`（シェル）と `BalloonSurface{key:"2"}`（バルーン）を同一 scope・同一
/// states へ順に流すと、`Show{..,binds}`（シェル・binds 同梱）と `ShowBalloon{2}`（バルーン・
/// binds なし）が互いに干渉せず送信順どおりに記録される（別 map・別発行・4.6）。RED では
/// バルーン cue が catch-all へ落ち ShowBalloon が欠落するため列照合が落ちる。
#[test]
fn shell_and_balloon_recorded_independently() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    // シェル面（Emote）→バルーン面（BalloonSurface）を同一 scope へ順に流す。
    let shell = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(emote_cue(0.0, "0", "2100")),
    );
    let balloon = handle_message(
        &resolver,
        &BindResolver::empty(),
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(balloon_cue(1.0, "0", "2")),
    );

    assert_eq!(shell, ControlFlow::Continue(()));
    assert_eq!(balloon, ControlFlow::Continue(()));
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[
            DisplayCommand::Show {
                scope: ActorKey::from("0"),
                surface_id: 2100,
                binds: BindSet::from_ids([1100, 1207]),
                pattern: PatternState::default(),
            },
            DisplayCommand::ShowBalloon {
                scope: ActorKey::from("0"),
                surface_id: 2,
                pattern: PatternState::default(),
            },
        ],
        "シェル面（Show+binds）とバルーン面（ShowBalloon）が独立に記録される（4.6）"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Task 6.2: broadcast honor 契約——担当外 cue の受信は正常経路（良性 debug!）（2.2/2.3/5.4/11.6）。
//
// broadcast（D4）で seriko は全 cue を受け取る。Balloon 系（Text/NewLine/Clear/ClearAll/Choice）は
// emo-text の担当、Wait はどの演者の担当でもない純粋な待ち。いずれも seriko にとって「異常」でなく
// 「正常な担当外受信」であり、action を無視しつつ duration を honor（＝新ローカル遅延を生まない・
// seriko は自前 reveal/timeline を持たないので skip で自明に満たす）。ログは良性 debug! へ格下げ
// （warn/error を出さない＝実害ない水準）。genuine anomaly（Unresolved=error/Invalid=error/
// NameForm=warn/EntityRef=warn）は据え置き——本タスクで格下げしない。
// ─────────────────────────────────────────────────────────────────────

/// ケース12（6.2/2.3・broadcast honor）: 担当外（非 Shell＝Balloon 担当）の cue を broadcast
/// 受信すると、異常でなく正常経路として skip され、ログは良性 `debug!`（WARN/ERROR を出さない）。
///
/// 同期 `handle_message`＋`capture_logs_flow` で、WARN も ERROR も 0 件・DEBUG は観測可能
/// （silent 化でなく格下げ）・状態不変・`Continue` を固定する。RED では `Some(other)` 腕が
/// `warn!` を出すため WARN=0 の assert が落ちる。
#[test]
fn non_shell_broadcast_reception_is_benign_debug_no_warn_error() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(text_cue(0.0, "0", "アヒルやアヒル")),
        )
    });

    assert_eq!(
        flow.1,
        ControlFlow::Continue(()),
        "担当外 cue も正常経路として処理継続する（6.2）"
    );
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        0,
        "担当外 broadcast 受信は WARN を出さない（異常でなく正常経路・実害ない水準・R2.2/2.3）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=ERROR").count(),
        0,
        "担当外 broadcast 受信は ERROR を出さない: {}",
        flow.0
    );
    assert!(
        flow.0.contains("level=DEBUG"),
        "担当外 broadcast 受信は良性 debug! として観測できる（silent 化でなく格下げ・log-first）: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "担当外 cue では発行しない（action 無視・状態不変・skip）"
    );
}

/// ケース13（6.2/5.4・純粋 Wait honor）: Wait cue（どの演者の担当でもない純粋な待ち）を
/// broadcast 受信すると、action なしで skip され、ログは良性 `debug!`（WARN/ERROR なし）。
///
/// `cue_target_of(Wait)==None` だが「分類不能」ではなく「担当演者なし」——5.2 が Wait cue を
/// 発行し始めた今、`warn!("分類できない")` は意味的に不正確なノイズ（申し送り）。debug! へ格下げし
/// 文言も是正したことを固定する。seriko は自前 timeline を持たず skip が新ローカル遅延を生まない
/// （duration honor＝否定的 no-op・R5.4）。RED では `None` 腕が `warn!` を出すため WARN=0 が落ちる。
#[test]
fn wait_broadcast_reception_is_benign_debug_no_warn_error() {
    let resolver = tiny_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(wait_cue(0.0, "0", 0.5)),
        )
    });

    assert_eq!(
        flow.1,
        ControlFlow::Continue(()),
        "Wait も正常経路として処理継続する（5.4）"
    );
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        0,
        "Wait 受信は WARN を出さない（分類不能でなく担当演者なし・正常経路）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=ERROR").count(),
        0,
        "Wait 受信は ERROR を出さない: {}",
        flow.0
    );
    assert!(
        flow.0.contains("level=DEBUG"),
        "Wait 受信は良性 debug! として観測できる: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "Wait では発行しない（action なし・状態不変）"
    );
}

/// ケース14（2.2/2.3・honor＝否定的 no-op の observable）: 担当外 cue（非 Shell・Wait）を
/// 挟んでも、後続の担当 cue（Emote）は焼き込み絶対時刻どおりに発行され、担当外 cue は状態も
/// タイミングも乱さない（新たなローカル遅延なし・二重待ちなし）。spawn 経由 observable ゆえ
/// cross-thread log には依存せず、発行列でスキップの純粋 no-op 性を示す。
#[test]
fn non_shell_and_wait_are_no_op_then_shell_cue_still_emits() {
    let out = MockSurfaceOutput::new();
    let records = out.records();
    let (mut sink, handle) = spawn_seriko(
        tiny_resolver(),
        BindSet::from_ids([1100, 1207]),
        BindResolver::empty(),
        SerikoLoopConfig::disabled(),
        out,
    );

    CueSink::emit(&mut sink, text_cue(0.0, "0", "担当外テキスト")); // 非 Shell＝skip
    CueSink::emit(&mut sink, wait_cue(0.5, "0", 1.0)); // Wait＝skip
    CueSink::emit(&mut sink, emote_cue(1.5, "0", "2100")); // 担当＝発行
    sink.close().expect("Close を送れること");
    handle.join().expect("Close で正常終了する");

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[DisplayCommand::Show {
            scope: ActorKey::from("0"),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207]),
            pattern: PatternState::default(),
        }],
        "担当外（非 Shell/Wait）は skip され、担当 Emote のみ発行＝状態/タイミング不変（honor 否定的 no-op・2.2/2.3）"
    );
}
