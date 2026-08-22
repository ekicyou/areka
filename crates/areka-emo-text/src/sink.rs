//! # sink — cue 受信口（結線層）
//!
//! **単一の出力契約** [`dola::cue::CueSink`] を実装する `EmoTextSink` と搬送 envelope
//! `TextMsg` を担う。`emit` は cue 再生ランタイム（`CuePlayer`）／sakura drive の worker
//! スレッド上で呼ばれ、cue を UI ドレインへ非ブロック送出する（`UiSender` が配送口そのもの）。
//!
//! 演者非依存の `CueSink` 1 本へ集約する（旧 `SurfaceSink`/`TextSink` の 2 分割は
//! broadcast＋演者側 relevance ゆえ不要・R11.6）。ghost live-path も `CueSink` 注入で結線する
//! （task 8.1 で旧 `TextSink` 経路を撤去し `CueSink` 一本へ集約）。
//!
//! **層規律**: 結線層。送信失敗（UI アクター停止後）は `tracing::error!` のみ・panic しない
//! （`emit` は infallible 契約・log-first）。

use std::ops::ControlFlow;

use areka_actor::{UiSendError, UiSender};
use areka_sakura::contract::TalkCue;

/// UI ドレインへの搬送 envelope（`Send + 'static`・design.md 結線層 正本）。
///
/// [`EmoTextSink`] が worker スレッドから積み、UI ドレイン（`spawn_ui` の drain タスク）が
/// [`handle_text_msg`] で写像する。cue と終了指示を同一 FIFO 口で運ぶことで、
/// 「Close より前に積まれた cue は必ず先に処理される」順序保証を channel に委ねる。
#[derive(Clone, Debug, PartialEq)]
pub enum TextMsg {
    /// sakura からの cue（後出し優先で即時適用される）。
    Cue(TalkCue),
    /// 終了指示（結線側が talk 経路の畳み込みで送る・`Ok(Break)` 経路）。
    Close,
}

/// **単一の出力契約** [`dola::cue::CueSink`] の実装——broadcast された全 cue を UI ドレインへ
/// 非ブロック送出する受信口（担当外 cue も届き、reveal を汚さず duration を honor する）。
///
/// - `dola::cue::CueSink + Clone + Send + 'static` を満たす（cue 再生ランタイム／`GhostBootOptions.text_sink`
///   へ登録・注入可能な形・注入自体は emo2-boot の責務・R10.1/R11.6）。
/// - `emit` は `CuePlayer`／sakura drive の worker スレッド上で呼ばれる＝受信端はワーカー側
///   （R1.3 前半)。中間アクターは設けない（[`UiSender`] が配送口そのもの——design.md 結線層）。
/// - cue は FIFO で UI ドレインへ到達する（unbounded・非ブロック）。
///
/// # Invariants
///
/// `emit`／`close` はいかなる失敗でも panic しない（送信失敗は `tracing::error!` のみ・R1.5）。
#[derive(Clone)]
pub struct EmoTextSink {
    tx: UiSender<TextMsg>,
}

impl EmoTextSink {
    /// UI ドレインの送信端（`spawn_ui::<TextMsg>` の戻り）から受信口を構築する。
    ///
    /// 正規の構築点は結線 API `spawn_emo_text`（actor.rs・drain 起動と対で取得する）。
    pub fn new(tx: UiSender<TextMsg>) -> Self {
        Self { tx }
    }

    /// 終了指示（`emit` と同じ口・FIFO で cue の後に並ぶ）。
    ///
    /// UI ドレインは [`TextMsg::Close`] 受領で `Ok(Break)`＝error ログなしのクリーン終了
    /// （R1.4 相当）。送信失敗（UI ドレイン停止後）は `tracing::error!` のみ・panic しない。
    pub fn close(&self) {
        if self.tx.send(TextMsg::Close).is_err() {
            tracing::error!(
                "EmoTextSink::close: UI text drain already stopped; close directive dropped"
            );
        }
    }

    /// 1 発火を UI ドレインへ積む（unbounded・非ブロック・R1.1）。単一の出力契約
    /// [`dola::cue::CueSink`] の `emit` が用いる送出本体。
    ///
    /// 送信失敗（UI ドレイン停止後）は失われた cue の文脈付き `tracing::error!` のみで、
    /// panic せず infallible 契約に従い戻り値なし（R1.5・log-first）。後続の `emit` も
    /// 引き続き受理される（受信口は失敗で壊れない）。
    fn deliver(&mut self, cue: TalkCue) {
        if let Err(UiSendError(TextMsg::Cue(lost))) = self.tx.send(TextMsg::Cue(cue)) {
            tracing::error!(
                at = lost.at,
                actor = ?lost.actor,
                command = ?lost.command,
                "EmoTextSink::emit: cue delivery failed (UI text drain stopped); \
                 dropping cue and continuing"
            );
        }
    }
}

/// **単一の出力契約**（R11.6・task 4.1）: 演者非依存の [`dola::cue::CueSink`] を実装する。
///
/// `CuePlayer` は登録された全 sink へ全 cue を broadcast し、emo-text は担当（Balloon）か否かを
/// 演者側 relevance（`cue_target_of`）で選別する——action を無視しても duration は honor する
/// （担当外 cue も本 sink 経由で純粋状態機械へ届き、reveal を汚さない・R2.2/R2.3）。配送先スロットの
/// 2 分割（旧 SurfaceSink/TextSink）は broadcast＋演者側 relevance ゆえ廃した（task 8.1 で暫定並存の
/// `TextSink` 実装を撤去し `CueSink` 一本へ集約）。
impl dola::cue::CueSink for EmoTextSink {
    fn emit(&mut self, cue: TalkCue) {
        self.deliver(cue);
    }
}

/// 終了規律の正準写像（design.md 結線層——drain handler の器）。
///
/// UI ドレインの handler は本関数へ委譲し、cue の処理（状態機械適用等）だけを `on_cue` で
/// 差し込む。終了経路はちょうど 2 つ（R1.4 相当）:
///
/// - [`TextMsg::Close`] 受領 → `Ok(ControlFlow::Break(()))`（`on_cue` は呼ばれない・
///   基盤 `spawn_ui` が error ログなしで drain を終了する）。
/// - 全 [`UiSender`]（＝全 [`EmoTextSink`] クローン）drop → drain の `recv` が閉じて正常終了
///   （本関数の外・基盤の規約）。
///
/// 個別 cue の処理失敗は `Err` として返し、基盤が `error!`＋継続する（R1.5——
/// **失敗は終了経路ではない**・ループを殺さない）。
pub fn handle_text_msg<E>(
    msg: TextMsg,
    on_cue: impl FnOnce(TalkCue) -> Result<(), E>,
) -> Result<ControlFlow<()>, E> {
    match msg {
        TextMsg::Close => Ok(ControlFlow::Break(())),
        TextMsg::Cue(cue) => on_cue(cue).map(|()| ControlFlow::Continue(())),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use areka_sakura::contract::{ActorKey, CueCommand, CueSink, TalkCue};
    use windows::Win32::UI::WindowsAndMessaging::PostQuitMessage;
    use wintf_winmsg_executor::{FilterResult, MessageLoop};

    use super::{EmoTextSink, TextMsg, handle_text_msg};

    // ── ログ檻（WARN/ERROR 件数を数える最小 Subscriber・draw.rs の檻パターン踏襲） ──

    struct LevelCounter {
        warns: Arc<AtomicUsize>,
        errors: Arc<AtomicUsize>,
    }

    impl tracing::Subscriber for LevelCounter {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            match *event.metadata().level() {
                tracing::Level::WARN => {
                    self.warns.fetch_add(1, Ordering::SeqCst);
                }
                tracing::Level::ERROR => {
                    self.errors.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    /// クロージャをログ檻の中で実行し、（結果, WARN 件数, ERROR 件数）を返す。
    ///
    /// `with_default` はスレッドスコープ。drain タスクは本テストスレッド（＝pump スレッド）
    /// 上で走るため、handler／基盤 drain ループ／`emit` 失敗経路の全ログが檻に入る。
    fn with_log_cage<T>(f: impl FnOnce() -> T) -> (T, usize, usize) {
        let warns = Arc::new(AtomicUsize::new(0));
        let errors = Arc::new(AtomicUsize::new(0));
        let subscriber = LevelCounter {
            warns: Arc::clone(&warns),
            errors: Arc::clone(&errors),
        };
        let out = tracing::subscriber::with_default(subscriber, f);
        (
            out,
            warns.load(Ordering::SeqCst),
            errors.load(Ordering::SeqCst),
        )
    }

    // ── テスト土台（実 channel・実 spawn_ui・bounded pump） ──

    /// テスト用 cue（state.rs の cue ヘルパと同型）。
    fn cue(actor: &str, at: f64, command: CueCommand) -> TalkCue {
        TalkCue {
            at,
            actor: ActorKey::from(actor),
            command,
            duration: 0.0,
        }
    }

    /// drain タスク終了の観測用センチネル。handler クロージャへ move し、drain future が
    /// drop された時（`Ok(Break)` 終了・全送信元切断終了のいずれも）に flag が立つ。
    /// async_task は future 完走時に future を即時 drop するため、pump 内で同期的に観測できる。
    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    /// 記録 handler で `spawn_ui` し、`(sink, terminated, records)` を返す。
    ///
    /// - handler は [`handle_text_msg`]（終了規律の正準写像）へ委譲し、cue を `records` へ
    ///   push する（＝「描画を担う側」への配送先スタンドイン。状態機械適用は task 7.2 の領分）。
    /// - `poison` が指す本文の Text cue は `Err` を返す（個別配送失敗の再現・R1.5）。
    /// - 本テストスレッドが pump スレッドを演じる（`spawn_ui` と pump は同一スレッド必須）。
    fn spawn_recording_drain(
        poison: Option<&'static str>,
    ) -> (EmoTextSink, Arc<AtomicBool>, Arc<Mutex<Vec<TalkCue>>>) {
        let terminated = Arc::new(AtomicBool::new(false));
        let records = Arc::new(Mutex::new(Vec::new()));
        let sentinel = DropFlag(Arc::clone(&terminated));
        let recs = Arc::clone(&records);
        let (tx, _detached_drain) = areka_actor::spawn_ui(
            "emo-text-drain-test",
            move |msg: TextMsg| -> Result<ControlFlow<()>, String> {
                // センチネルを handler クロージャへ捕捉（drain future の drop で flag が立つ）。
                let _hold = &sentinel;
                handle_text_msg(msg, |cue| {
                    if let (Some(p), CueCommand::Text(text)) = (poison, &cue.command) {
                        if text == p {
                            return Err(format!("poison cue rejected: {text}"));
                        }
                    }
                    recs.lock().expect("records lock").push(cue);
                    Ok(())
                })
            },
        )
        .expect("spawn_ui on the test (pump) thread must succeed");
        (EmoTextSink::new(tx), terminated, records)
    }

    /// 事前 queue 済みメッセージを全て処理し、queue が空になった時点で抜ける bounded pump。
    ///
    /// `PostQuitMessage` の `WM_QUIT` は posted メッセージ（spawn_local の wake 含む）が
    /// 尽きた後にのみ配送されるため、drain タスクは queue 済み cue を全て処理してから
    /// pump が復帰する（heartbeat・deadline・sleep 不要の決定論・ハングしない）。
    fn pump_until_idle() {
        // SAFETY: PostQuitMessage は現スレッドの message queue へ quit 要求を積むだけの
        // 無害な Win32 呼び出し（テストスレッド＝pump スレッド）。
        unsafe { PostQuitMessage(0) };
        MessageLoop::run(|_, _| FilterResult::Forward);
    }

    // ── 注入可能形（単一の出力契約 dola::cue::CueSink + Clone + Send + 'static） ──

    /// 単一出力契約（`dola::cue::CueSink`）への登録可能性を型で検証する（トレイトはフルパス束縛で参照）。
    fn assert_cue_sink_contract<T: dola::cue::CueSink + Clone + Send + 'static>() {}

    /// コンパイル時検証: `EmoTextSink` は cue 再生ランタイムへの登録・`GhostBootOptions.text_sink` への
    /// 注入が可能な**単一の出力契約**（`dola::cue::CueSink + Clone + Send + 'static`）を満たす
    /// （R10.1/R11.6・task 4.1 の単一トレイト・注入自体は emo2-boot の責務）。
    #[test]
    fn emo_text_sink_satisfies_cue_sink_contract() {
        assert_cue_sink_contract::<EmoTextSink>();
    }

    // ── 終了規律の正準写像（handle_text_msg・純粋部） ──

    /// `TextMsg::Close` → `Ok(Break)`（終了指示＝クリーン終了経路・R1.4 相当の写像）。
    /// on_cue は呼ばれない。
    #[test]
    fn handle_text_msg_maps_close_to_break() {
        let result = handle_text_msg::<String>(TextMsg::Close, |_| {
            panic!("on_cue must not be called for Close")
        });
        assert_eq!(result, Ok(ControlFlow::Break(())));
    }

    /// `TextMsg::Cue` → 配送成功なら `Ok(Continue)`（受理継続）。cue は無変形で配送先へ渡る。
    #[test]
    fn handle_text_msg_delivers_cue_and_continues() {
        let delivered = std::cell::RefCell::new(None);
        let sent = cue("0", 1.5, CueCommand::Text("アヒル".into()));
        let result = handle_text_msg::<String>(TextMsg::Cue(sent.clone()), |c| {
            *delivered.borrow_mut() = Some(c);
            Ok(())
        });
        assert_eq!(result, Ok(ControlFlow::Continue(())));
        assert_eq!(
            delivered.into_inner(),
            Some(sent),
            "cue は無変形で配送される"
        );
    }

    /// 個別配送失敗は `Err` として基盤へ返す（基盤が error!＋継続する・R1.5——
    /// Break にはしない＝失敗は終了経路ではない）。
    #[test]
    fn handle_text_msg_propagates_cue_failure_as_err() {
        let result = handle_text_msg(
            TextMsg::Cue(cue("0", 0.0, CueCommand::Clear)),
            |_| -> Result<(), String> { Err("delivery failed".into()) },
        );
        assert_eq!(result, Err("delivery failed".to_string()));
    }

    // ── 3 ケースの実 channel 再現（実 spawn_ui drain・bounded pump） ──

    /// ケース1: 終了指示（`close()`）——error ログを伴わずクリーンに終了する（R1.4 相当）。
    /// あわせて疎通の正準観測: clone 間で interleave した emit が FIFO で配送される（R1.1）。
    #[test]
    fn close_directive_terminates_drain_cleanly_after_fifo_delivery() {
        let ((), _warns, errors) = with_log_cage(|| {
            let (mut sink, terminated, records) = spawn_recording_drain(None);
            let mut sink2 = sink.clone();

            let c0 = cue("0", 0.0, CueCommand::Text("アヒルや".into()));
            let c1 = cue("1", 0.1, CueCommand::NewLine { ratio: 1.0 });
            let c2 = cue("0", 0.2, CueCommand::Clear);
            sink.emit(c0.clone());
            sink2.emit(c1.clone());
            sink.emit(c2.clone());
            sink.close();

            pump_until_idle();

            // 全送信元（sink/sink2）は生存したまま＝終了は Close（Ok(Break)）経路のみでありうる。
            assert!(
                terminated.load(Ordering::SeqCst),
                "Close 受領で drain はクリーンに終了する"
            );
            assert_eq!(
                *records.lock().expect("records lock"),
                vec![c0, c1, c2],
                "clone 間で interleave した cue が FIFO・無変形で配送される"
            );
            drop(sink);
            drop(sink2);
        });
        assert_eq!(
            errors, 0,
            "終了指示によるクリーン終了は error ログを伴わない"
        );
    }

    /// ケース2: 全送信元切断（全 `EmoTextSink` clone drop）——queue 済み cue を届け切った上で
    /// error ログを伴わずクリーンに終了する（R1.4 相当）。
    #[test]
    fn dropping_all_sinks_terminates_drain_cleanly() {
        let ((), _warns, errors) = with_log_cage(|| {
            let (mut sink, terminated, records) = spawn_recording_drain(None);
            let sink2 = sink.clone();

            let c0 = cue("0", 0.0, CueCommand::Text("残置 cue".into()));
            sink.emit(c0.clone());

            // Close は送らない＝終了は全送信元切断（channel closed）経路のみでありうる。
            drop(sink);
            drop(sink2);

            pump_until_idle();

            assert!(
                terminated.load(Ordering::SeqCst),
                "全送信元切断で drain はクリーンに終了する"
            );
            assert_eq!(
                *records.lock().expect("records lock"),
                vec![c0],
                "切断前に queue 済みの cue は破棄されず届く"
            );
        });
        assert_eq!(
            errors, 0,
            "全送信元切断によるクリーン終了は error ログを伴わない"
        );
    }

    /// ケース3a: 個別配送失敗（handler の `Err`）——panic せず error ログ 1 件を記録した上で、
    /// 後続 cue の受理を継続する（R1.5・失敗はループを殺さない）。
    #[test]
    fn delivery_failure_is_logged_and_subsequent_cues_processed() {
        let ((), _warns, errors) = with_log_cage(|| {
            let (mut sink, terminated, records) = spawn_recording_drain(Some("poison"));

            let before = cue("0", 0.0, CueCommand::Text("前".into()));
            let bad = cue("0", 0.1, CueCommand::Text("poison".into()));
            let after = cue("0", 0.2, CueCommand::Text("後".into()));
            sink.emit(before.clone());
            sink.emit(bad);
            sink.emit(after.clone());
            sink.close();

            pump_until_idle();

            assert!(
                terminated.load(Ordering::SeqCst),
                "失敗後も drain は生き続け、Close で終了する（＝後続 cue まで処理が届いた）"
            );
            assert_eq!(
                *records.lock().expect("records lock"),
                vec![before, after],
                "失敗 cue の後続 cue も受理・配送される（失敗は継続・R1.5）"
            );
        });
        assert_eq!(
            errors, 1,
            "個別配送失敗はちょうど 1 件の error ログとして記録される"
        );
    }

    /// ケース3b: 送信側の配送失敗（UI drain 停止後の `emit`／`close`）——panic せず
    /// 1 呼び出しにつき 1 件の error ログのみで、後続の `emit` 受理（infallible 契約）を
    /// 継続する（R1.5・design「送信失敗は error! のみ」）。
    #[test]
    fn emit_after_drain_stop_logs_error_without_panic_and_stays_callable() {
        // まず drain を正規経路（Close）で停止させる（channel closed 状態を作る）。
        let (mut sink, terminated, _records) = spawn_recording_drain(None);
        sink.close();
        pump_until_idle();
        assert!(terminated.load(Ordering::SeqCst), "前提: drain は停止済み");

        // 停止後の emit×2＋close×1: panic せず、各呼び出しが error 1 件のみ。
        let ((), _warns, errors) = with_log_cage(|| {
            sink.emit(cue("0", 1.0, CueCommand::Text("届かない".into())));
            sink.emit(cue("0", 1.1, CueCommand::Clear));
            sink.close();
        });
        assert_eq!(
            errors, 3,
            "停止後の emit/close は 1 呼び出しにつき error 1 件のみ（panic なし・受理継続）"
        );
    }
}
