//! I1 canonical 一周 e2e（task 5.1・design.md「inproc 決定論 e2e（I1〜I3）」シナリオ I1）。
//!
//! spine の S1（`spine_e2e_test.rs` の `s1_boot_success`）を **実 InProc DLL 境界越し**で再演する。
//! S1 は台本 fake backend（`ScriptedShioriBackend`）が `\s[0]hello\e` を返すのに対し、本 I1 は
//! `ShioriWiring::InProc` で実ビルド済み `shiori4_testdll.dll` をロードし、その OnBoot 応答（凍結
//! スナップショット由来の起動挨拶）が kanade→start-relay→dispatcher→sakura→`RecordingSink` へ
//! 届くまでを **注入時刻のみ**（時刻前進＝talk 進行は単調増加する `DispatcherMsg::Tick` の `now` 注入のみ・
//! 実時計は talk を進めない）で駆動し、演出 cue 列を凍結台本由来の期待列と **全順序**で照合する
//! （要件 5.1/5.5・design.md I1）。駆動ループは反復回数境界でなく **壁時計 deadline 境界**（task 2.4 方式）で
//! 括り、poll 周期は `yield_now()`（sleep 不使用・要件 1.3）——deadline は宙吊り防止の上限にすぎず sim 時刻は
//! 注入 Tick の `now` のみが進める。
//!
//! さらに凍結応答が差し替えられた際の検出漏れを塞ぐため、期待列と凍結スナップショット
//! （`shiori4_testdll::snapshot_for("OnBoot")`）の整合を **ドリフト検出 assert** で固定する（要件 5.2）。
//! 正常終了の握手は `shutdown(CloseReason::System)==Ok(())` で観測する（要件 5.3）。観測は演出配送の
//! **受領レベル**（`RecordingSink`）に留め、実描画（サーフェス合成・画素読戻し）は要求しない（要件 5.5）。
//!
//! # 兄弟 spine テストとの共存（tasks.md「[4.2 重大・5.1/5.2必読]」＋ task 5.0）
//! 本テストは同一 `ghost` テストバイナリ内で spine の協調ループと並走する。自前 assemble をせず
//! `inproc_fixture::shared_test_ghost()`（OnceLock・assemble 一度・hardlink 優先）を再利用して新規バイト
//! 書き込み由来の Defender 再スキャン＝spine starvation を避け、駆動ループは反復回数境界でなく **壁時計
//! deadline 境界**（task 2.4 の runtime.rs InProc 統合テスト方式）で括る。spine のプローブループは task 5.0 で
//! 同じ壁時計 deadline へ硬化済みゆえ、本テストの並走による CPU 競合は spine の cue 到達を deadline 内で遅らせる
//! だけで偽陽性を生まない（5.0 前は spine が反復回数境界の空 spin で早合点し飢餓したため 2ms sleep で緩和して
//! いたが、その応急措置は 5.0 完了で撤去した）。残る低頻度残存はこの並列マルチエージェント build セッション
//! 特有の兄弟コンパイル burst 下でのみ稀に spine settle ループが飢餓するもので、本テスト自体は無関係。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use areka_ghost::dispatcher::DispatcherMsg;
use areka_ghost::{GhostBootOptions, ShioriWiring, TickerMode, boot, inproc_connect};
use areka_kanade::{CloseReason, MonotonicMs, ShioriBackend};
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package;
use areka_parsers::package::ShioriMount;
use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

use shiori_host32_host::{ExitKind, HelperStatus, RequestError, ShutdownError};

use crate::inproc_fixture::shared_test_ghost;
use crate::recorder::{ExchangeKind, ExchangeOutcome, ExchangeRecord, Recorder, RecorderHandle};
use crate::spine_e2e_test::RecordingSink;

/// `inproc_connect` が返す `Box<dyn ShioriBackend>` を、具体型境界 `Recorder<B: ShioriBackend>` へ
/// 渡せるようにする薄い委譲アダプタ（`Box<dyn ShioriBackend>` は `ShioriBackend` を実装せず、孤児則で
/// blanket impl も足せないため newtype で橋渡しする）。全呼出を内側の実 InProc backend へ素通しする
/// ——記録は外側の `Recorder` が担い、本アダプタは駆動対象非依存の中継に徹する（D-3「同一手口」）。
struct BoxedBackend(Box<dyn ShioriBackend>);

impl ShioriBackend for BoxedBackend {
    fn get(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<Option<String>, RequestError> {
        self.0.get(id, references, status)
    }

    fn notify(
        &mut self,
        id: &str,
        references: &[String],
        status: Option<&str>,
    ) -> Result<(), RequestError> {
        self.0.notify(id, references, status)
    }

    fn unload(&mut self) -> Result<ExitKind, ShutdownError> {
        self.0.unload()
    }

    fn status(&mut self) -> HelperStatus {
        self.0.status()
    }
}

/// 凍結 OnBoot スナップショットの Value（起動挨拶さくらスクリプト・tasks.md「[1.3 提供データ]」）。
///
/// 期待 cue 列（下記）とドリフト検出 assert の **単一の値源**。task 6.2 が実採取で PROVISIONAL を
/// 置換する際は、この定数と期待列を凍結後の実データへ更新する（tasks.md 6.2・要件 2.6）。
const EXPECTED_ONBOOT_VALUE: &str = r"\0\s[0]おはようございますわ（暫定）\e";

/// 起動挨拶のテキスト本文（`EXPECTED_ONBOOT_VALUE` の `\s[0]` と `\e` に挟まれた表示文字列）。
///
/// 期待 Text cue の内容をこの定数から導出し、値源との連結を明示する（task 5.1）。
const EXPECTED_GREETING_TEXT: &str = "おはようございますわ（暫定）";

/// 有界待機ヘルパ（spine S1／runtime.rs の `run_bounded` と同旨のローカルコピー）。`shutdown` を
/// 別スレッドへ逃がし `recv_timeout` で宙吊りを防ぐ（`ActorHandle::join` 由来のブロックを括る）。
fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// I1: 起動から終話までの一周を注入時刻のみで駆動し、起動挨拶の演出列が凍結台本由来の期待列と
/// 全順序で一致することを照合する（＋ドリフト検出＋正常終了握手・要件 5.1/5.2/5.3/5.4/5.5/1.1/1.3）。
#[test]
fn i1_inproc_one_lap_records_frozen_onboot_greeting_and_closes_cleanly() {
    // ---- ドリフト検出（要件 5.2）: 期待列の値源（EXPECTED_ONBOOT_VALUE）が実 DLL の凍結
    //      スナップショットと一致することを先に固定する。凍結応答が task 6.2 等で差し替えられた
    //      のに期待列が追随していない場合、この assert（および後段の cue 列 assert）が確実に fail し、
    //      検出漏れ（silent drift）を塞ぐ。加えて期待 Text 本文が値源に含まれることも連結固定する。 ----
    let snapshot = shiori4_testdll::snapshot_for("OnBoot")
        .expect("OnBoot は shiori4-testdll のスナップショット表に収載されていること（narrowing: GET）");
    assert!(
        snapshot.contains(EXPECTED_ONBOOT_VALUE),
        "ドリフト検出: 凍結 OnBoot スナップショットが期待 Value を含まない。\
         スナップショットが差し替えられたら期待列（EXPECTED_ONBOOT_VALUE）も更新すること（要件 5.2・tasks.md 6.2）。\n\
         expected Value = {EXPECTED_ONBOOT_VALUE:?}\n\
         snapshot       = {snapshot:?}"
    );
    assert!(
        EXPECTED_ONBOOT_VALUE.contains(EXPECTED_GREETING_TEXT),
        "期待 Text 本文（EXPECTED_GREETING_TEXT）は値源 EXPECTED_ONBOOT_VALUE から導出される連結であること"
    );

    // ---- 実 InProc 一周の駆動 ----
    // 自前 assemble せず共有 fixture を再利用（tasks.md「[4.2 重大]」・spine starvation 回避）。
    let ghost_root = shared_test_ghost().to_path_buf();

    // broadcast ゆえ surface/text の両 sink が同一の全 cue を受ける（S1 と同律）。boot 前に records
    // ハンドルを取得しておく。
    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

    let options = GhostBootOptions {
        ghost_root,
        default_encoding: DefaultEncoding::Utf8,
        shiori: ShioriWiring::InProc,
        surface_sink,
        text_sink,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed through ShioriWiring::InProc（実テスト DLL）");

    // 期待 broadcast 列（S1 と同構造）: `\0\s[0]<greeting>\e` は
    //   ClearAll@0（#6 全消去・talk 冒頭前置）/ Emote{key:"0"}@0（\s[0]）/ Text(<greeting>)@0
    // へコンパイルされる（`\0` は既定 scope 0 の明示ゆえ actor は "0" のまま・後続 cue が無く先頭群に
    // 留まる）。値源 EXPECTED_GREETING_TEXT から Text 内容を導出する。
    let expected: Vec<CueCommand> = vec![
        CueCommand::ClearAll,
        CueCommand::Emote {
            key: "0".to_string(),
        },
        CueCommand::Text(EXPECTED_GREETING_TEXT.to_string()),
    ];

    // 駆動: dispatcher へ Tick を注入し、両 sink が期待列長へ整定するまで待つ。反復回数ではなく
    // **壁時計デッドライン**で括る（task 2.4 方式）——InProc 経路は最初の SHIORI 呼出で実 DLL の
    // `LoadLibraryW`＋`CreateInstance` を初めて走らせ数十 ms を要するため。デッドラインは宙吊り防止の上限に
    // すぎず、talk timeline の前進は依然として注入 Tick の `now` のみ（実時計で進めない・要件 5.1）。DLL
    // ロード完了後は最初の Tick で挨拶が全 broadcast されるため、捕捉の有無・cue 列・順序は決定論的。
    //
    // poll 周期は `yield_now()`（task 2.4 先例と同形・sleep 不使用）。兄弟 spine のプローブループは task 5.0 で
    // 壁時計 deadline へ硬化済みゆえ、本テストの並走による CPU 競合は spine の cue 到達を deadline 内で遅らせる
    // だけで偽陽性を生まない（5.0 前は spine が反復回数境界の空 spin で早合点し飢餓したため 2ms sleep で緩和して
    // いたが、その応急措置は 5.0 完了で不要）。sim 時刻は依然として注入 Tick の `now` のみが進める。
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut now: u64 = 1;
    let mut captured = false;
    while Instant::now() < deadline {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive while probing for the boot talk");
        now += 1;
        let s = surface_records.lock().expect("records mutex poisoned").len();
        let t = text_records.lock().expect("records mutex poisoned").len();
        if s >= expected.len() && t >= expected.len() {
            captured = true;
            break;
        }
        // hang-guard の poll 周期（talk pacing ではない）。sim 時刻は注入 Tick の `now` のみが進める。
        std::thread::yield_now();
    }
    assert!(
        captured,
        "I1: 起動挨拶の演出列が壁時計デッドライン内に両 sink へ届かなかった——実 InProc DLL 境界を \
         横断した一周が成立していない（hang guard）"
    );

    // ---- 演出列の全順序照合（要件 5.1・broadcast・at 昇順・内容一致）----
    let surface = surface_records
        .lock()
        .expect("records mutex poisoned")
        .clone();
    let text = text_records.lock().expect("records mutex poisoned").clone();

    let assert_broadcast = |cues: &[TalkCue], who: &str| {
        let commands: Vec<CueCommand> = cues.iter().map(|c| c.command.clone()).collect();
        assert_eq!(
            commands, expected,
            "{who} sink は broadcast で ClearAll/Emote{{0}}/Text(挨拶) を全順序で受けること \
             （partition は演者側 relevance・凍結台本由来）: {cues:?}"
        );
        for cue in cues {
            assert_eq!(cue.at, 0.0, "{who} 発火は全て at=0.0（先頭群・後続 cue なし）");
            assert_eq!(
                cue.actor,
                ActorKey::from("0"),
                "{who} 発火 actor は \\0 が明示する既定 scope 0"
            );
        }
        for pair in cues.windows(2) {
            assert!(pair[0].at <= pair[1].at, "{who} 発火列は at 昇順であるべき");
        }
    };
    assert_broadcast(&surface, "surface");
    assert_broadcast(&text, "text");

    // ---- 正常終了の握手（要件 5.3）----
    // shutdown（ForceQuit→OnClose NOTIFY→Unload）が clean に完走し `Ok(())` を返すこと。有界待機で
    // 宙吊りを防ぐ（`shutdown` 内部の全 actor join を括る）。
    run_bounded(
        "shutdown after I1 boot talk completion",
        Duration::from_secs(15),
        move || {
            let result = runtime.shutdown(CloseReason::System);
            assert!(
                result.is_ok(),
                "正常終了の握手: shutdown は I1 一周後に Ok(()) を返すこと（clean close）, got {result:?}"
            );
        },
    );

    // 共有 fixture（shared_test_ghost）はプロセス寿命 leak ゆえ本テストでは削除しない（意図的・
    // inproc_fixture.rs のドキュメント参照）。
}

/// 起動挨拶の broadcast cue 列を全順序照合する共有ヘルパ（I1 の局所クロージャと同律・
/// I2 が同一の演出出力 assert を「同じ手口」で再演するために free 関数へ括り出す）。
///
/// 期待列は値源 `EXPECTED_GREETING_TEXT`（=`EXPECTED_ONBOOT_VALUE` の表示本文）から導出した
/// `[ClearAll, Emote{key:"0"}, Text(<greeting>)]`。broadcast ゆえ surface/text いずれの sink も
/// 同一の全順序でこれを受ける（partition は演者側 relevance・凍結台本由来）。加えて全 cue が
/// `at=0.0`・actor `"0"`（`\0` が明示する既定 scope）で、発火列が at 昇順であることを固定する。
fn assert_greeting_broadcast(cues: &[TalkCue], who: &str) {
    let expected: Vec<CueCommand> = vec![
        CueCommand::ClearAll,
        CueCommand::Emote {
            key: "0".to_string(),
        },
        CueCommand::Text(EXPECTED_GREETING_TEXT.to_string()),
    ];
    let commands: Vec<CueCommand> = cues.iter().map(|c| c.command.clone()).collect();
    assert_eq!(
        commands, expected,
        "{who} sink は broadcast で ClearAll/Emote{{0}}/Text(挨拶) を全順序で受けること \
         （partition は演者側 relevance・凍結台本由来）: {cues:?}"
    );
    for cue in cues {
        assert_eq!(cue.at, 0.0, "{who} 発火は全て at=0.0（先頭群・後続 cue なし）");
        assert_eq!(
            cue.actor,
            ActorKey::from("0"),
            "{who} 発火 actor は \\0 が明示する既定 scope 0"
        );
    }
    for pair in cues.windows(2) {
        assert!(pair[0].at <= pair[1].at, "{who} 発火列は at 昇順であるべき");
    }
}

/// I2: 実 InProc DLL を駆動しながら交信記録デコレータ `Recorder` を被せ、**交信列**（発行イベントの
/// 種別・ID・順序・結果）と**演出出力列**（cue）の**双方**を同一の一周から同一手口で照合する
/// （要件 1.4「二記録装置」・design.md「inproc 決定論 e2e」シナリオ I2・D-3）。
///
/// I1 が演出出力（`RecordingSink`）のみを見るのに対し、本 I2 は同じ DLL 境界に対し `ShioriBackend`
/// seam へ `Recorder` を合成（`Custom(Recorder(inproc_connect(..)))`・D-3）して**交信列**も同時に
/// 捕捉する。boot が resolve を内部で行うのと同型に、ここでは mount を**自前で解決**して
/// `mount.shiori` を `inproc_connect` へ渡し（`boot()` の InProc arm が `inproc_connect(mount.shiori.clone())`
/// を呼ぶのと同じ導出）、その実 InProc backend を `Recorder` で包んで `Custom` wiring として注入する。
/// connect closure は shiori アクタースレッド上で spawn 時に一度だけ走り、`RecorderHandle` を
/// 共有スロット経由でテストスレッドへ逃がす（駆動後に読む）。
///
/// # 交信列の決定論（ticker 無効＋Tick は dispatcher 止まり）
/// `TickerMode::Disabled` かつ dispatcher は `Tick` を kanade へ中継しない（`TalkDone` のみ中継）ため、
/// kanade は駆動ループ中に Tick を一切受けず、定常運転由来の追加交信を発行しない。したがって交信列は
/// boot 系列（`OnInitialize` NOTIFY → `OnFirstBoot` GET(204) → `OnBoot` GET(Value) → `basewareversion`
/// NOTIFY）に続き、shutdown（ForceQuit）が起こす close 系列（`OnClose` NOTIFY → `Unload` Clean）で
/// 閉じる——全順序が決定論的。記録は shutdown 完了後に読む（`OnClose`/`Unload` は shutdown が起こす）。
#[test]
fn i2_inproc_one_lap_records_both_exchange_sequence_and_greeting_cues() {
    // ---- 実 InProc backend へ Recorder を合成した Custom wiring を組む（D-3）----
    // 自前 assemble せず共有 fixture を再利用（tasks.md「[4.2 重大]」・spine starvation 回避）。
    let ghost_root = shared_test_ghost();

    // boot が内部で通すのと同一契約で mount を自前解決し、`mount.shiori` を inproc_connect へ渡す
    // （`boot()` の InProc arm が `inproc_connect(mount.shiori.clone())` を呼ぶのと同じ導出）。
    let mount = package::resolve(ghost_root, DefaultEncoding::Utf8)
        .expect("共有テストゴーストは実マウント解決を成功裏に通過すべき（boot 内部と同一契約）");

    // Recorder ハンドルを spawn 後のアクタースレッドからテストスレッドへ逃がす共有スロット。
    let handle_slot: Arc<Mutex<Option<RecorderHandle>>> = Arc::new(Mutex::new(None));
    let slot2 = Arc::clone(&handle_slot);
    let connect = inproc_connect(mount.shiori.clone());
    let wiring = ShioriWiring::Custom(Box::new(move || {
        // 実 InProc backend を確立（このアクタースレッド上で実 DLL を LoadLibrary する）。
        let inner = connect()?;
        let (recorder, handle) = Recorder::new(BoxedBackend(inner));
        *slot2.lock().expect("recorder handle slot poisoned") = Some(handle);
        Ok(Box::new(recorder) as Box<dyn ShioriBackend>)
    }));

    // broadcast ゆえ surface/text の両 sink が同一の全 cue を受ける（I1 と同律）。
    let surface_sink = RecordingSink::new();
    let text_sink = RecordingSink::new();
    let surface_records = surface_sink.records();
    let text_records = text_sink.records();

    let options = GhostBootOptions {
        ghost_root: ghost_root.to_path_buf(),
        default_encoding: DefaultEncoding::Utf8,
        shiori: wiring,
        surface_sink,
        text_sink,
        ticker: TickerMode::Disabled,
    };

    let runtime = boot(options).expect("boot should succeed through Custom(Recorder(inproc_connect))");

    // 期待演出列長（I1 と同構造の `[ClearAll, Emote{0}, Text(挨拶)]` = 3）。
    let expected_cue_len = 3usize;

    // 駆動: dispatcher へ Tick を注入し、両 sink が期待列長へ整定するまで**壁時計デッドライン**で
    // 括って待つ（I1・task 2.4 方式・sleep 不使用・`yield_now` poll）。sim 時刻は注入 Tick の `now`
    // のみが進める。DLL ロード完了後は最初の Tick で挨拶が全 broadcast されるため決定論的。
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut now: u64 = 1;
    let mut captured = false;
    while Instant::now() < deadline {
        runtime
            .dispatcher()
            .send(DispatcherMsg::Tick {
                now: MonotonicMs(now),
            })
            .expect("dispatcher actor should still be alive while probing for the boot talk");
        now += 1;
        let s = surface_records.lock().expect("records mutex poisoned").len();
        let t = text_records.lock().expect("records mutex poisoned").len();
        if s >= expected_cue_len && t >= expected_cue_len {
            captured = true;
            break;
        }
        // hang-guard の poll 周期（talk pacing ではない）。sim 時刻は注入 Tick の `now` のみが進める。
        std::thread::yield_now();
    }
    assert!(
        captured,
        "I2: 起動挨拶の演出列が壁時計デッドライン内に両 sink へ届かなかった——実 InProc DLL 境界を \
         横断した一周が成立していない（hang guard）"
    );

    // ---- 正常終了の握手（要件 5.3）: shutdown（ForceQuit→OnClose NOTIFY→Unload）を完走させる。----
    // 交信列の close 系列（OnClose/Unload）は shutdown が起こすため、記録は shutdown 完了後に読む。
    run_bounded(
        "shutdown after I2 boot talk completion",
        Duration::from_secs(15),
        move || {
            let result = runtime.shutdown(CloseReason::System);
            assert!(
                result.is_ok(),
                "正常終了の握手: shutdown は I2 一周後に Ok(()) を返すこと（clean close）, got {result:?}"
            );
        },
    );

    // ================= 記録装置 (1): 交信列（Recorder） =================
    // shutdown 完了後に交信記録を読む（OnClose NOTIFY→Unload まで確定済み）。
    let records: Vec<ExchangeRecord> = handle_slot
        .lock()
        .expect("recorder handle slot poisoned")
        .as_ref()
        .expect("connect closure が RecorderHandle をスロットへ格納しているはず（shiori actor spawn 済み）")
        .records();

    // 観測を可視化（RED phase の実列採取・回帰時の突合材料）。
    eprintln!("I2 observed exchange records ({} 件):", records.len());
    for (i, r) in records.iter().enumerate() {
        eprintln!(
            "  [{i}] kind={:?} id={:?} references={:?} status={:?} outcome={:?}",
            r.kind, r.id, r.references, r.status, r.outcome
        );
    }

    // 交信列を (kind, id, outcome) の全順序で照合する（status() は Recorder 非記録・status フィールドは
    // 定常 nuance を持つため assert 対象外）。OnBoot の Value 結果は演出列の値源 EXPECTED_ONBOOT_VALUE と
    // 同一（`InProcBackend::get("OnBoot")` は Value 行 payload を返す）——二記録装置の連結固定。
    let observed: Vec<(ExchangeKind, Option<String>, ExchangeOutcome)> = records
        .iter()
        .map(|r| (r.kind.clone(), r.id.clone(), r.outcome.clone()))
        .collect();
    let expected_exchanges: Vec<(ExchangeKind, Option<String>, ExchangeOutcome)> = vec![
        // boot 系列（KanadeMsg::Boot 起点・ticker 無効ゆえ定常追加交信なし）。
        (
            ExchangeKind::Notify,
            Some("OnInitialize".to_string()),
            ExchangeOutcome::NotifyOk,
        ),
        (
            ExchangeKind::Get,
            Some("OnFirstBoot".to_string()),
            ExchangeOutcome::NoContent,
        ),
        (
            ExchangeKind::Get,
            Some("OnBoot".to_string()),
            ExchangeOutcome::Value(EXPECTED_ONBOOT_VALUE.to_string()),
        ),
        (
            ExchangeKind::Notify,
            Some("basewareversion".to_string()),
            ExchangeOutcome::NotifyOk,
        ),
        // close 系列（shutdown=ForceQuit が起こす・OnClose は best-effort NOTIFY→Unload Clean）。
        (
            ExchangeKind::Notify,
            Some("OnClose".to_string()),
            ExchangeOutcome::NotifyOk,
        ),
        (
            ExchangeKind::Unload,
            None,
            ExchangeOutcome::Unloaded("Ok(Clean)".to_string()),
        ),
    ];
    assert_eq!(
        observed, expected_exchanges,
        "I2 交信列: 発行イベントの種別・ID・順序・結果が boot 系列→close 系列の決定論列と一致すること \
         （要件 1.4・同一手口の記録装置(1)）"
    );

    // ================= 記録装置 (2): 演出出力列（RecordingSink） =================
    // 同一の一周から、起動挨拶 cue 列を I1 と同一手口で全順序照合する（broadcast・at 昇順・内容一致）。
    let surface = surface_records
        .lock()
        .expect("records mutex poisoned")
        .clone();
    let text = text_records.lock().expect("records mutex poisoned").clone();
    assert_greeting_broadcast(&surface, "surface");
    assert_greeting_broadcast(&text, "text");

    // 共有 fixture（shared_test_ghost）はプロセス寿命 leak ゆえ本テストでは削除しない（意図的）。
}

// ============================================================================
// I3: ロード失敗の主要態様を決定論的に検証する（task 5.3・design.md「inproc 決定論 e2e」
//     シナリオ I3・Error Handling「connect 失敗」行・要件 3.5）。
//
// I1/I2 が実 InProc DLL 越しの一周（boot→talk→shutdown）を協調ループで駆動するのに対し、
// I3 は **boot しない**——`inproc_connect(shiori)()` を直接（unit-style）呼び、ロード確立の
// 3 態様（参照先未指定 / DLL 欠落 / 不正イメージ）がいずれも **panic せず** `Err`（log-first・
// 要件 3.5）として顕在化することのみを軽量・決定論的に確かめる。アクタースレッド・drive ループ・
// 共有 fixture・実ビルド済み DLL のいずれも要さない（3 態様とも「失敗」検証ゆえ成功ロードが不要）。
// ============================================================================

/// I3 用フィクスチャ組み立てヘルパ（`shiori_wiring.rs::build_shiori_mount` を鏡写しにしたもの）。
///
/// 一意な一時ゴースト（`ghost/master/descript.txt`＋`shell/master/` dir）を作り、本番と同じ
/// 経路（`package::resolve`）で `ShioriMount` を得る（`ShioriMount` は `#[non_exhaustive]` ゆえ
/// struct リテラルで組めない・実フィクスチャの resolve が唯一の入手経路）。`shiori_line` が
/// `Some(name)` なら descript に `shiori,<name>` 行を書き `file: Some(name)`、`None` なら行を
/// 書かず `file: None`（resolve は推測しない）。掃除のため root パスも返す（呼び出し側が
/// best-effort で削除する）。`dll_bytes` が `Some` なら resolve 後の `mount.shiori.dir`
/// （=`ghost/master`・`inproc_connect` が `dir.join(file)` で組むロード元）へ指定バイト列を
/// `shiori_line` の名前で書き出す（不正イメージ態様のため）。
fn build_i3_mount(
    shiori_line: Option<&str>,
    dll_bytes: Option<&[u8]>,
) -> (ShioriMount, PathBuf) {
    let unique = format!(
        "areka-ghost-i3-load-failure-{}-{:?}-{}",
        std::process::id(),
        std::thread::current().id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );
    let root = std::env::temp_dir().join(unique);
    let master_dir = root.join("ghost").join("master");
    std::fs::create_dir_all(&master_dir).expect("fixture master dir 作成に失敗");
    std::fs::create_dir_all(root.join("shell").join("master")).expect("fixture shell dir 作成に失敗");

    let mut descript = String::new();
    if let Some(name) = shiori_line {
        descript.push_str(&format!("shiori,{name}\n"));
    }
    std::fs::write(master_dir.join("descript.txt"), descript)
        .expect("fixture descript.txt 書き込みに失敗");

    let mount = package::resolve(&root, DefaultEncoding::Ansi).expect("fixture ghost_root の resolve に失敗");
    let shiori = mount.shiori;

    // 不正イメージ態様: resolve 済み shiori.dir（ロード元 dir）へ非 PE バイト列を DLL 名で置く。
    if let Some(bytes) = dll_bytes {
        let name = shiori_line.expect("dll_bytes を書くには shiori_line（DLL 名）が要る");
        std::fs::write(shiori.dir.join(name), bytes).expect("非 PE テキストの書き出しに失敗");
    }

    (shiori, root)
}

/// I3-① **参照先未指定**（`file: None`）: descript に `shiori,` 行が無ければ `file: None` になり、
/// `inproc_connect(shiori)()` は DLL ロードを試みず即座に `Err`（推測しない・log-first・要件 3.5）。
/// 偽陽性（別理由の失敗）を避けるため、エラー文言が DLL ファイル名未解決由来であること
/// （`inproc_connect` の `file: None` 枝の文言に「ファイル名」を含む）まで確認する。
#[test]
fn i3_load_failure_missing_reference_returns_err() {
    let (shiori, root) = build_i3_mount(None, None);
    assert_eq!(shiori.file, None, "fixture は shiori 行なし＝file:None のはず");

    let connect = inproc_connect(shiori);
    let result = connect();

    // 後始末（best-effort）: assert より先に一時ディレクトリを掃除する。
    let _ = std::fs::remove_dir_all(&root);

    match result {
        Err(err) => assert!(
            err.contains("ファイル名"),
            "参照先未指定はファイル名未解決として顕在化すること（別理由の失敗でない）: {err}"
        ),
        Ok(_) => panic!("参照先未指定（file:None）はロード失敗になるはず（inproc_connect は panic せず Err）"),
    }
}

/// I3-② **DLL 欠落**: `shiori,someMissing.dll` 行はあるが `ghost/master/` に当該 DLL が存在しない。
/// `inproc_connect` は `LoadLibraryW` を不在パスへ試み、失敗を `error!` 済み `Err` へ写す（panic せず・
/// 要件 3.5）。
#[test]
fn i3_load_failure_missing_dll_returns_err() {
    let (shiori, root) = build_i3_mount(Some("someMissing.dll"), None);
    assert_eq!(
        shiori.file,
        Some("someMissing.dll".to_string()),
        "fixture は shiori 行あり＝file:Some のはず"
    );
    // DLL 実体は書いていない＝ロード元に不在（LoadLibraryW が失敗する前提）。
    assert!(
        !shiori.dir.join("someMissing.dll").exists(),
        "態様前提: DLL は存在しないこと"
    );

    let connect = inproc_connect(shiori);
    let result = connect();

    let _ = std::fs::remove_dir_all(&root);

    match result {
        Err(_) => {}
        Ok(_) => panic!("DLL 欠落はロード失敗になるはず（inproc_connect は panic せず Err）"),
    }
}

/// I3-③ **不正イメージ**: `shiori,bogus.dll` 行があり `ghost/master/bogus.dll` は非 PE テキスト。
/// `LoadLibraryW` は不正イメージを拒否し、`inproc_connect` は失敗を `error!` 済み `Err` へ写す
/// （panic せず・要件 3.5）。
#[test]
fn i3_load_failure_invalid_image_returns_err() {
    let (shiori, root) = build_i3_mount(Some("bogus.dll"), Some(b"this is not a valid PE image"));
    // 不正イメージ実体がロード元に存在すること（LoadLibraryW がイメージ検証で拒否する前提）。
    assert!(
        shiori.dir.join("bogus.dll").exists(),
        "態様前提: 非 PE テキストの bogus.dll が存在すること"
    );

    let connect = inproc_connect(shiori);
    let result = connect();

    let _ = std::fs::remove_dir_all(&root);

    match result {
        Err(_) => {}
        Ok(_) => panic!("不正イメージはロード失敗になるはず（inproc_connect は panic せず Err）"),
    }
}
