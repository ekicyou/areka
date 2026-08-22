use super::test_support::*;
use super::*;
use crate::contract::{CueCommand, TalkCue, TalkId};
use crate::duration::text_playback_duration;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

// ── task 5.2: ResolveChoice ハンドラ＋即時 settle の統合檻（R2.3/2.4/9.8） ──
//
// 共通 fixture: `\s[10]hello\w[2]\q[選択A,targetA]\e`。compile 後（アンカー 0）:
//   ClearAll@0 / Emote{10}@0 / hello@0(D=0.25) / Wait@0.25(0.1) / Choice@0.35(id=targetA) /
//   Barrier@0.35（選択待ち・R2.1/2.2）。占有 horizon=0.35。barrier が**最終 horizon 要素**（menu
//   ケース）ゆえ、Tick(0.5) で barrier 到達後に解決すると、既に current_offset(0.5) ≥ horizon(0.35)
//   で **その場で** 完了する（次 Tick を待たない・settle_after_tick と同型の後始末を共用）。

const MENU_SCRIPT: &str = r"\s[10]hello\w[2]\q[選択A,targetA]\e";

/// Choice の着弾（＝barrier 到達）を決定的に観測するため、記録 sink に加えチャンネル sink を挟む
/// ヘルパ。Tick(0.5) を送り、Choice(id=targetA) cue の着弾を待って返す（この時点で player は
/// `WaitingForChoice`・後続 ResolveChoice は inbox FIFO でこの後に処理される）。
fn drive_menu_to_barrier(handle: &TalkHandle, rx: &mpsc::Receiver<TalkCue>) {
    // 初回 Tick(0.0) 刻印: ClearAll/Emote/hello を配送（barrier 未到達）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    // Tick(0.5): Wait@0.25・Choice@0.35 を配送し barrier@0.35 到達 → WaitingForChoice。
    handle.inbox.send(SakuraMsg::Tick(0.5)).unwrap();
    // Choice cue 着弾を barrier に、barrier 到達（WaitingForChoice 遷移）を決定的に待つ。
    loop {
        let cue = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("Choice cue（barrier 手前）が届くべき");
        if matches!(cue.command, CueCommand::Choice { .. }) {
            break;
        }
    }
}

/// **R2.3（barrier-stop）**: 選択待ち barrier で止まった talk は、horizon 越えまで `Tick` を注入
/// しても**完了として通知されない**（選択未解決の間 `TalkDone` を出さない）。
#[test]
fn menu_barrier_withholds_talkdone_while_choice_unresolved() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: MENU_SCRIPT.to_string(),
        talk_id: TalkId(801),
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    drive_menu_to_barrier(&handle, &rx);

    // horizon(0.35) を遥かに越える Tick を注入しても、選択未解決ゆえ完了しない（R2.3）。
    handle.inbox.send(SakuraMsg::Tick(5.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(50.0)).unwrap();

    // 負の窓: barrier 未解決の間は TalkDone が発火しない（早期完了しない）。
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "選択待ち barrier 未解決の間は horizon 越え Tick でも TalkDone を出さない（R2.3）"
    );
    assert!(
        !handle.actor.is_finished(),
        "barrier 未解決ゆえ talk は駆動継続（完了通知せず）"
    );

    // 片付け: Close で中断 ACK を取り body を畳む（テスト resource の後始末）。
    handle.inbox.send(SakuraMsg::Close).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5)).expect("Close で中断 ACK");
    assert_eq!(done.reason, TalkEndReason::Interrupted);
    handle.actor.join().expect("body は正常終了する");
}

/// **R2.4/9.8（resolve-resume・即時 settle）**: barrier で止まった talk へ有効な選択 id を
/// `SakuraMsg::ResolveChoice` で投入すると、**追加の `Tick` なしに**再開し `TalkDone{Ended}` へ
/// 到達する（menu ケース＝barrier が最終 horizon 要素ゆえその場で完了・R-5 の一 tick 遅延を残さない）。
#[test]
fn resolve_choice_resumes_barrier_stopped_talk_and_settles_immediately() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let talk_id = TalkId(802);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: MENU_SCRIPT.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    drive_menu_to_barrier(&handle, &rx);

    // 有効な選択 id を投入。追加 Tick は**送らない**（即時 settle の弁別）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "targetA".to_string(),
        })
        .unwrap();

    // 追加 Tick なしで自然終端へ到達する（barrier 解決で offset(0.5) ≥ horizon(0.35) ＝即完了）。
    let done = recv_done(&done_rx, Duration::from_secs(5)).expect(
        "ResolveChoice で talk が再開し、追加 Tick なしで TalkDone に到達すべき（R2.4/9.8）",
    );
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(
        done.reason,
        TalkEndReason::Ended,
        "`\\e` 終端の menu talk は解決後 Ended で完了する"
    );
    handle.actor.join().expect("body は正常終了する");
}

/// **mismatch**: 未知の選択 id で `ResolveChoice` しても状態は不変（`None` 記録＋継続）で
/// `TalkDone` は出ず、talk は待機継続する。その後**有効な id** で解決すれば完了へ到達し、
/// 誤 id が barrier を壊していない（talk が生存継続していた）ことを示す。
#[test]
fn resolve_choice_with_unknown_id_is_noop_and_talk_continues() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let talk_id = TalkId(803);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: MENU_SCRIPT.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    drive_menu_to_barrier(&handle, &rx);

    // 未知 id: resolve_choice は None → 記録して継続（状態不変・barrier は解けない）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "NO_SUCH_ID".to_string(),
        })
        .unwrap();

    // 負の窓: 誤 id では完了しない（barrier 依然未解決）。
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "未知 id の ResolveChoice では TalkDone を出さない（状態不変・継続）"
    );
    assert!(
        !handle.actor.is_finished(),
        "誤 id は barrier を壊さず talk は待機継続する"
    );

    // 有効 id で解決すれば完了へ到達（barrier が生きていた＝誤 id で壊れていない証）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "targetA".to_string(),
        })
        .unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("有効 id の解決で完了へ到達すべき（誤 id 後も barrier は生存）");
    assert_eq!(done.talk_id, talk_id);
    assert_eq!(done.reason, TalkEndReason::Ended);
    handle.actor.join().expect("body は正常終了する");
}

/// **R2.3/2.4/9.8（full-menu 統合檻・Task 10.2）**: `menu.pasta:15` 相当の**実 3 択メニュー**
/// （`\q \n \q \_l[5em,2lh] \q`）を **`spawn_talk` の actor 境界**（内部で parse→compile を通す）へ
/// 投入し、選択待ち barrier 停止→3 択のうち**実 id の 1 つ**での解決→即時 settle を end-to-end で固定する。
///
/// 5.2 の 3 檻（`menu_barrier_*`/`resolve_choice_*`）は**単一** `\q` の `MENU_SCRIPT` で actor 境界を
/// 覆い、`compile_broadcast_stream_*` は**3 択実 menu** を覆うが**生 `CuePlayer`**（actor 境界を通らない）。
/// 本檻はその両者の交差＝「実 3 択 menu × `spawn_talk` × 実 choice id 解決」を単一の統合檻で立証する
/// （5.2 の単一 `\q` では現れない、複数 Choice がバッグに並ぶ中で**中間 id** を照合して解ける経路）。
///
/// 檻は 3 主張を 1 本の actor フローで固定する:
///  - **R2.3**: barrier 停止後、horizon を遥かに越える `Tick(5.0)/Tick(50.0)` でも `TalkDone` 不送出。
///  - **mismatch**: 未知 id の `ResolveChoice` では状態不変（`TalkDone` 不送出・talk 継続）。
///  - **R2.4/9.8**: 3 択の**中間**実 id（`Onエモの位置調整メニュー`）で解決すると、**追加 `Tick` なしに**
///    再開し `TalkDone{Ended}` へ到達する（barrier は最終 horizon 要素＝その場で settle）。
#[test]
fn full_menu_via_spawn_talk_barrier_stops_and_middle_choice_id_settles_immediately() {
    // menu.pasta:15 の raw さくらスクリプト断片（3 択＋改行＋カーソル指定）。
    // `spawn_talk` へ**生 script として**渡し、parse→compile は actor 内部の実経路を通す
    // （5.2 の単一 `\q` MENU_SCRIPT でも、生 CuePlayer 檻@compile_broadcast_stream_* でもなく、
    //  実 3 択 menu が actor 境界を貫く経路をここで初めて覆う）。
    let script = concat!(
        r"\q[おしゃべり頻度,Onおしゃべり頻度メニュー]",
        r"\n",
        r"\q[エモの位置調整,Onエモの位置調整メニュー]",
        r"\_l[5em,2lh]",
        r"\q[閉じる,Onメニュー閉じる]",
    );
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let talk_id = TalkId(810);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: script.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    // Tick(0.0)/Tick(0.5) で offset 0 群（ClearAll/3 Choice/NewLine/Cursor）を配送し barrier@0 到達を待つ。
    drive_menu_to_barrier(&handle, &rx);

    // R2.3: horizon(=0) を遥かに越える Tick を注入しても、選択未解決ゆえ完了しない。
    handle.inbox.send(SakuraMsg::Tick(5.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(50.0)).unwrap();
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "実 3 択 menu でも barrier 未解決の間は horizon 越え Tick で TalkDone を出さない（R2.3）"
    );
    assert!(
        !handle.actor.is_finished(),
        "barrier 未解決ゆえ talk は駆動継続（早期完了しない・R2.3）"
    );

    // mismatch: 3 択のいずれとも一致しない id では状態不変（`None` 記録＋継続・barrier は解けない）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "NO_SUCH_ID".to_string(),
        })
        .unwrap();
    assert!(
        recv_done(&done_rx, NEG_WINDOW).is_err(),
        "不一致 id の ResolveChoice では TalkDone を出さない（状態不変・複数 Choice バッグは無傷）"
    );
    assert!(
        !handle.actor.is_finished(),
        "不一致 id は barrier を壊さず talk は待機継続する（バッグの他 id は依然解決可能）"
    );

    // R2.4/9.8: 3 択の**中間** id を投入。追加 Tick は**送らない**（即時 settle の弁別）。
    // 中間 id を選ぶことで「先頭/末尾でなくバッグ内の任意 id を照合して解ける」ことも固定する。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "Onエモの位置調整メニュー".to_string(),
        })
        .unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("中間実 id の解決で再開し、追加 Tick なしで TalkDone に到達すべき（R2.4/9.8）");
    assert_eq!(done.talk_id, talk_id, "talk_id エコー");
    assert_eq!(
        done.reason,
        TalkEndReason::Ended,
        "`\\e` 無しの menu 台本は既定 Ended で完了する（compile 既定＝Ended）"
    );
    handle.actor.join().expect("body は正常終了する");
}

/// **defensive（Armed 誤投函）**: 初回 `Tick` 前（`Armed`＝CuePlayer 未構築）に `ResolveChoice` が
/// 届いても warn して継続し（防御枝）、以降の通常 Tick 駆動で talk は正常に終端する。
#[test]
fn resolve_choice_before_playback_armed_is_ignored_and_playback_survives() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(804);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: r"\s[10]hello\w[2]world\e".to_string(),
        talk_id,
    };
    let sink = RecordingSink::new();
    let records = sink.records();
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(sink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // 初回 Tick 前（Armed）に ResolveChoice 誤投函: warn して継続（CuePlayer 未構築ゆえ no-op）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "targetA".to_string(),
        })
        .unwrap();

    // 通常 Tick 列で駆動・終端する（防御枝がループを殺していない証）。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("Armed 誤投函後も通常 Tick で終端するべき");
    assert_eq!(done.reason, TalkEndReason::Ended, "再生は破綻せず Ended");
    handle.actor.join().expect("body は正常終了する");
    assert_eq!(
        records.lock().unwrap().len(),
        5,
        "誤投函で早期全量配信されず、通常 5 cue が届く"
    );
}

// ── task 3.2: 選択待ち成立の通知（`ChoiceWaiting`）檻（R5.2/7.1/7.2・DD-6/DD-7/DD-8） ──
//
// 通知は **talk アクター**（再生層＝真実源）が `WaitingForChoice` 遷移を検出した時点で、
// `TalkDone` と**同一の done ポート**へ送出する（因果順保存・DD-6）。檻は注入 Tick のみで
// 駆動し（実時間待機なし）、次の 4 点を固定する:
//
//  - **一度きり**: barrier 成立の tick でちょうど 1 通。以降 Tick を打ち続けても 2 通目は来ない。
//  - **内容**: 候補 id 列（`pending_choices()` 由来・表示順）／`display_end_elapsed_secs`
//    （占有 horizon＝duration 権威・**tick 時刻ではない**・R7.2）／`timeout_directive_secs`
//    （compile は `None` を書く＝未指定＝下流の既定値へ委譲・DD-8）。
//  - **解決後に再通知しない**: 同一バリアが再成立しない限り 2 通目は出ない（R5.2 の完了へ進む）。
//  - **送出失敗で運行継続**: done 受信端 drop でも `error!` 記録のみで talk は死なない。

/// 通知の**捕捉時点**を弁別するための算術（MENU_SCRIPT の相対占有 horizon）。
///
/// `\s[10]hello\w[2]\q[選択A,targetA]\e`: hello の D(0.25) ＋ `\w[2]`(0.1) ＝ 0.35。
/// 期待値は本番と同一算術で導く（10 進直書きの表現誤差を排除）。
fn menu_relative_horizon() -> f64 {
    text_playback_duration("hello") + Duration::from_millis(100).as_secs_f64()
}

/// **一度きり検出＋通知内容（R7.1/7.2・DD-6/DD-7/DD-8）**: 選択肢を含む台本を注入 Tick で
/// 駆動すると、選択待ちバリア成立の tick で `ChoiceWaiting` が**ちょうど 1 回**届き、
/// 候補 id 列・占有 horizon・タイムアウト指令を同梱する。以降 Tick を打ち続けても再通知されない。
///
/// **弁別**: `display_end_elapsed_secs` は **注入 Tick 時刻（0.5）ではなく** 占有 horizon
/// （0.35＝台本由来の duration 権威）である。tick 時刻を載せる実装ならこの assert が落ちる。
#[test]
fn choice_waiting_notifies_exactly_once_with_ids_horizon_and_timeout() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let talk_id = TalkId(820);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: MENU_SCRIPT.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    // Tick(0.0) 刻印（アンカー 0.0）→ Tick(0.5) で barrier@0.35 到達＝WaitingForChoice。
    drive_menu_to_barrier(&handle, &rx);

    // 通知は barrier 成立の tick で送出される（**追加 Tick を要さない**）。
    let notice = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("選択待ちバリア成立で ChoiceWaiting が done ポートへ届くべき（DD-6・R7.1）");

    // 期待 horizon: 台本由来の占有 horizon（duration 権威）＝アンカー(0.0)＋相対 0.35。
    let compiled = compile(
        &areka_parsers::sakura::parse(MENU_SCRIPT),
        &SystemVarSnapshot::default(),
    );
    let expected_horizon = compiled.sheet.absolute_end_time();
    assert_eq!(
        expected_horizon,
        menu_relative_horizon(),
        "台本由来 horizon は hello の D ＋ \\w[2]（本番と同一算術で導いた 0.35）"
    );

    assert_eq!(
        notice,
        TalkNotice::ChoiceWaiting(ChoiceWaiting {
            talk_id,
            choice_ids: vec!["targetA".to_string()],
            display_end_elapsed_secs: expected_horizon,
            timeout_directive_secs: None,
        }),
        "通知は talk_id エコー＋候補 id 列＋占有 horizon（tick 時刻 0.5 ではない・R7.2）＋\
         タイムアウト未指定（compile は `None` を書く＝下流の既定値へ委譲・DD-8）を同梱する"
    );

    // 一度きり: barrier は解けていないため以降の Tick でも 2 通目は出ない（検出フラグ・DD-6）。
    handle.inbox.send(SakuraMsg::Tick(5.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(50.0)).unwrap();
    assert_eq!(
        done_rx.recv_timeout(NEG_WINDOW).unwrap_err(),
        RecvTimeoutError::Timeout,
        "選択待ち継続中に Tick を重ねても ChoiceWaiting は再送されない（一度きり検出）"
    );

    // 通知後の Close: 中断 ACK が**通知の後**に同一ポートを流れる（因果順保存・DD-6）。
    handle.inbox.send(SakuraMsg::Close).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5))
        .expect("通知後の Close でも中断 ACK（TalkDone{Interrupted}）が返るべき");
    assert_eq!(
        done,
        TalkDone {
            talk_id,
            reason: TalkEndReason::Interrupted
        },
        "Close は既存どおり Interrupted ACK（通知の追加で中断経路は変わらない）"
    );
    handle.actor.join().expect("body は正常終了する");
}

/// **候補 id 列は表示順・horizon はアンカー込み（DD-7・R7.2）**: 実 3 択メニューを **0 以外の
/// アンカー**（7.0）で駆動し、通知が (1) 3 択の id を**表示順のまま**運び、(2)
/// `display_end_elapsed_secs` に**アンカーを含む**占有 horizon を載せることを固定する。
///
/// **弁別**: この台本は全内容が `at=0`（相対 horizon＝0）ゆえ、期待値は **アンカー 7.0 そのもの**。
/// アンカーを無視して相対 horizon だけを載せる実装なら 0.0 が届いて落ちる。
/// なお barrier は初回 `Tick(7.0)` で成立するため、本檻では注入 Tick 時刻と horizon が
/// 一致し **tick 時刻実装は弁別できない**——その弁別は
/// `choice_waiting_notifies_exactly_once_with_ids_horizon_and_timeout`
/// （horizon 0.35 vs tick 0.5）が担う。
#[test]
fn choice_waiting_carries_candidate_ids_in_display_order_with_anchored_horizon() {
    let script = concat!(
        r"\q[おしゃべり頻度,Onおしゃべり頻度メニュー]",
        r"\n",
        r"\q[エモの位置調整,Onエモの位置調整メニュー]",
        r"\_l[5em,2lh]",
        r"\q[閉じる,Onメニュー閉じる]",
    );
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let talk_id = TalkId(821);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: script.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    // アンカー 7.0 で刻印し、7.5 で barrier@0（相対）到達まで進める。
    const ANCHOR: f64 = 7.0;
    handle.inbox.send(SakuraMsg::Tick(ANCHOR)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(ANCHOR + 0.5)).unwrap();
    loop {
        let cue = rx
            .recv_timeout(Duration::from_secs(5))
            .expect("Choice cue（barrier 手前）が届くべき");
        if matches!(cue.command, CueCommand::Choice { .. }) {
            break;
        }
    }

    // 弁別の前提: この台本の相対占有 horizon は 0.0（全内容が at=0・duration 0）。
    let compiled = compile(
        &areka_parsers::sakura::parse(script),
        &SystemVarSnapshot::default(),
    );
    assert_eq!(
        compiled.sheet.absolute_end_time(),
        0.0,
        "3 択 menu は全内容 at=0 ゆえ相対占有 horizon は 0.0（アンカー弁別の前提）"
    );

    let notice = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("3 択 menu でも barrier 成立で ChoiceWaiting が届くべき");
    assert_eq!(
        notice,
        TalkNotice::ChoiceWaiting(ChoiceWaiting {
            talk_id,
            choice_ids: vec![
                "Onおしゃべり頻度メニュー".to_string(),
                "Onエモの位置調整メニュー".to_string(),
                "Onメニュー閉じる".to_string(),
            ],
            display_end_elapsed_secs: ANCHOR,
            timeout_directive_secs: None,
        }),
        "候補 id は表示順（先頭/中間/末尾）で運ばれ、horizon はアンカー込み（7.0・相対 0.0 ではない）"
    );

    // 片付け: Close で中断 ACK を取り body を畳む。
    handle.inbox.send(SakuraMsg::Close).unwrap();
    let done = recv_done(&done_rx, Duration::from_secs(5)).expect("Close で中断 ACK");
    assert_eq!(done.reason, TalkEndReason::Interrupted);
    handle.actor.join().expect("body は正常終了する");
}

/// **解決後に再通知されない（R5.2）**: 通知 → 有効 id の `ResolveChoice` → `TalkDone{Ended}` の
/// **ちょうど 2 通**が同一ポートを因果順で流れ、3 通目は存在しない（ポート disconnect で確定）。
///
/// 既存挙動の保存も同時に固定する: 残台本なしのメニュー形は解決の**その場で** settle し、
/// 追加 `Tick` なしに `Ended` へ到達する（`on_resolve_choice` の即 settle は無改変）。
#[test]
fn choice_waiting_is_not_renotified_after_resolve_and_done_follows() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let talk_id = TalkId(822);
    let start = StartTalk {
        epilogue: Vec::new(),
        script: MENU_SCRIPT.to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    drive_menu_to_barrier(&handle, &rx);

    // 1 通目: 選択待ち成立通知。
    let first = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("1 通目は ChoiceWaiting");
    assert_eq!(
        first,
        TalkNotice::ChoiceWaiting(ChoiceWaiting {
            talk_id,
            choice_ids: vec!["targetA".to_string()],
            display_end_elapsed_secs: menu_relative_horizon(),
            timeout_directive_secs: None,
        }),
        "1 通目は選択待ち成立通知（因果順の先）"
    );

    // 有効 id で解決。追加 Tick は**送らない**（即時 settle の保存＝既存挙動無改変）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "targetA".to_string(),
        })
        .unwrap();

    // 2 通目: 再生完了。ChoiceWaiting の再送ではない。
    let second = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("2 通目は TalkDone（解決で即時 settle・追加 Tick 不要）");
    assert_eq!(
        second,
        TalkNotice::Done(TalkDone {
            talk_id,
            reason: TalkEndReason::Ended
        }),
        "2 通目は TalkDone{{Ended}}（解決後に ChoiceWaiting が再送されない）"
    );

    // 3 通目は存在しない: talk スレッド終了で送信端が drop され、ポートは Disconnected になる。
    handle.actor.join().expect("body は正常終了する");
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap_err(),
        RecvTimeoutError::Disconnected,
        "通知は通算 2 通のみ（解決後の再通知も二重完了も無い）"
    );
}

/// **送出失敗でも運行継続（ログ無し失敗経路の禁止・TalkDone 送出と同規律）**: done 受信端を
/// 通知前に drop すると `ChoiceWaiting` の送出は `Err` になるが、talk は死なず選択待ちを継続し、
/// その後の有効 id 解決で通常どおり終端して body が panic せず畳まれる。
#[test]
fn choice_waiting_send_failure_is_tolerated_and_playback_continues() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let (tx, rx) = mpsc::channel::<TalkCue>();
    let start = StartTalk {
        epilogue: Vec::new(),
        script: MENU_SCRIPT.to_string(),
        talk_id: TalkId(823),
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(ChannelSink { tx }, NoopSink),
        SystemVarSnapshot::default(),
    );

    // 通知送出**前**に受信端を drop（以降の送出は全て Err＝`error!` 記録のみ）。
    drop(done_rx);

    drive_menu_to_barrier(&handle, &rx);
    assert!(
        !handle.actor.is_finished(),
        "通知の送出失敗は talk を終端させない（選択待ちを継続する）"
    );

    // 送出失敗後も inbox は生きており、解決入力を通常どおり処理して終端する
    // （＝失敗経路が受信ループを殺していないことの決定的証拠）。
    handle
        .inbox
        .send(SakuraMsg::ResolveChoice {
            id: "targetA".to_string(),
        })
        .expect("通知の送出失敗後も inbox は生きている");
    handle
        .actor
        .join()
        .expect("done 受信端 drop でも body は panic せず正常終了する");
}

/// **通知の負条件（DD-6・R5.2）**: `\q` を含まない台本は選択待ちへ入らないため
/// `ChoiceWaiting` を**一切**送出せず、done ポートを流れる通知は `TalkDone{Ended}` の
/// **ちょうど 1 通**である。
///
/// **弁別**: 通知条件から `WaitingForChoice` 判定を落とし settle ごとに通知する実装なら、
/// 1 通目が `ChoiceWaiting` になってこの assert が落ちる。通算 1 通であることは
/// talk スレッド終了後の `Disconnected` で確定させる（負の時間窓に依存しない）。
#[test]
fn script_without_choices_never_notifies_choice_waiting() {
    let (done_tx, done_rx) = mpsc::channel::<TalkNotice>();
    let talk_id = TalkId(824);
    let start = StartTalk {
        epilogue: Vec::new(),
        // `\q` 無し（＝compile は選択待ち barrier を発行しない・R2.5）。
        script: r"\s[10]hello\e".to_string(),
        talk_id,
    };
    let handle = spawn_talk(
        start,
        done_tx,
        two_sinks(NoopSink, NoopSink),
        SystemVarSnapshot::default(),
    );

    // Tick(0.0) でアンカー刻印、Tick(1.0) で占有 horizon（hello の D＝0.25）を跨いで自然終端。
    handle.inbox.send(SakuraMsg::Tick(0.0)).unwrap();
    handle.inbox.send(SakuraMsg::Tick(1.0)).unwrap();

    // 1 通目は `TalkDone{Ended}`（`recv_done` の読み飛ばしを使わず**生の 1 通目**を突合する）。
    let first = done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("自然終端で通知が 1 通届く");
    assert_eq!(
        first,
        TalkNotice::Done(TalkDone {
            talk_id,
            reason: TalkEndReason::Ended
        }),
        "選択肢の無い台本の 1 通目は TalkDone{{Ended}}（ChoiceWaiting は出ない）"
    );

    // 2 通目は存在しない: talk スレッド終了で送信端が drop され、ポートは Disconnected になる。
    handle.actor.join().expect("body は正常終了する");
    assert_eq!(
        done_rx.recv_timeout(Duration::from_secs(5)).unwrap_err(),
        RecvTimeoutError::Disconnected,
        "通知は通算 1 通のみ（選択待ちに入らない talk は ChoiceWaiting を送出しない）"
    );
}
