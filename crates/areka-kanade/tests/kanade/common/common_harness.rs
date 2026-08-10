//! 駆動ハーネスの組み立て（mock shiori・mock sakura sink・`spawn_kanade` の結線）。
//!
//! `common/mod.rs`（1,657 行）から責務単位で切り出した子モジュール（タスク 8.2）。
//! 項目は親のファサードから再輸出されるため、消費側の `super::common::X` は不変である。

use std::sync::mpsc::{Receiver, Sender};

use areka_actor::ActorHandle;
use areka_kanade::{KanadeConfig, KanadeMsg, TalkCommand, spawn_kanade};

use super::{
    BlockOn, FailOn, Fixture, MockSakura, MockShiori, QuitPolicy, SakuraGate, ShioriGate,
    spawn_mock_sakura, spawn_mock_sakura_gated, spawn_mock_shiori, spawn_mock_shiori_blocking,
    spawn_mock_shiori_failing,
};

// ============================================================================
// 駆動ハーネス（4.2〜4.6 が再利用）
// ============================================================================

/// {mock shiori・mock sakura sink・spawn_kanade} を組み立てた駆動ハーネス。
///
/// テストは [`sender`](Harness::sender) 経由で kanade inbox へ [`KanadeMsg`] を注入し、
/// [`shiori`](Harness::shiori)/[`sakura`](Harness::sakura) の記録アクセサで観測する。
/// 停止は kanade へ [`KanadeMsg::Close`] を送るか、全 Sender を drop すればよい。
pub struct Harness {
    /// kanade inbox の送信端（Boot/Tick/CloseRequest 等を注入）。
    pub sender: Sender<KanadeMsg>,
    /// kanade アクタースレッドの join ハンドル。
    pub kanade: ActorHandle,
    /// mock shiori（記録アクセサ・停止用送信端）。
    pub shiori: MockShiori,
    /// mock sakura sink（受領 talk アクセサ）。
    pub sakura: MockSakura,
}

/// 駆動ハーネスを組み立てる。
///
/// - `config`: kanade 運行構成（`shell_name`/`baseware_version` 等）。
/// - `fixture`: mock shiori の応答表（シナリオ構成）。
/// - `quit_policy`: mock sakura sink の TalkDone quit 方針。
///
/// 返す [`Harness`] は kanade 送信端・各 mock の記録アクセサ・join ハンドルを保持する。
pub fn spawn_harness(config: KanadeConfig, fixture: Fixture, quit_policy: QuitPolicy) -> Harness {
    let shiori = spawn_mock_shiori(fixture);

    // kanade→sakura の TalkCommand チャンネルを 1 本張る（DD-5・起動と選択解決が同一チャンネル）。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();

    // kanade を起動（inbox 送信端を得る）。boot prefetch（username 照会・R4.1）は駆動されるが、
    // ハーネスは照会結果を消費しないため no-op sink を注入する（Implementation Notes）。
    let (kanade_tx, kanade_handle) =
        spawn_kanade(config, shiori.sender.clone(), talk_tx, Box::new(|_, _| {}));

    // sink には TalkDone 返送用に kanade inbox 送信端のクローンを渡す。
    let sakura = spawn_mock_sakura(talk_rx, kanade_tx.clone(), quit_policy);

    Harness {
        sender: kanade_tx,
        kanade: kanade_handle,
        shiori,
        sakura,
    }
}

/// 保留機能付き駆動ハーネスを組み立てる（4.4 pattern 3 専用・[`spawn_harness`] の派生）。
///
/// [`spawn_harness`] と同一の結線だが、mock sakura sink を [`spawn_mock_sakura_gated`] で
/// 起動し、`hold_indices` に含まれる受領インデックスの talk の [`TalkDone`] を保留する。
/// 返す [`SakuraGate`] の [`release_all`](SakuraGate::release_all) で保留を解放できる。
///
/// これにより「talk を 1 本 active に保ったまま Tick を挟む」窓を決定的に作れる——保留 talk の
/// TalkDone は inbox へ届かないため、次 Tick は必ず `Steady{Some}` から処理され NOTIFY（Ref3=0）を
/// 発行する（DD-6）。sleep も wall-clock も用いない（メッセージ順序と有界条件のみ）。
pub fn spawn_harness_gated(
    config: KanadeConfig,
    fixture: Fixture,
    quit_policy: QuitPolicy,
    hold_indices: Vec<usize>,
) -> (Harness, SakuraGate) {
    let shiori = spawn_mock_shiori(fixture);

    // kanade→sakura の TalkCommand チャンネルを 1 本張る（DD-5・起動と選択解決が同一チャンネル）。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();

    // kanade を起動（inbox 送信端を得る）。boot prefetch（username 照会・R4.1）は駆動されるが、
    // ハーネスは照会結果を消費しないため no-op sink を注入する（Implementation Notes）。
    let (kanade_tx, kanade_handle) =
        spawn_kanade(config, shiori.sender.clone(), talk_tx, Box::new(|_, _| {}));

    // sink には TalkDone 返送用に kanade inbox 送信端のクローンを渡す（保留機能付き）。
    let (sakura, gate) =
        spawn_mock_sakura_gated(talk_rx, kanade_tx.clone(), quit_policy, hold_indices);

    (
        Harness {
            sender: kanade_tx,
            kanade: kanade_handle,
            shiori,
            sakura,
        },
        gate,
    )
}

/// 失敗注入付き駆動ハーネスを組み立てる（4.6 case 1 専用・[`spawn_harness`] の派生）。
///
/// [`spawn_harness`] と同一の結線だが、mock shiori を [`spawn_mock_shiori_failing`] で起動し、
/// `fail_on` が指す最初の呼出を指定語彙で失敗させる。これにより「区別語彙ごとの呼出失敗 →
/// Unloading{Fault}→Unload→Stopped（観測可能な終了）」を統合層で駆動できる（Req 6.1）。
pub fn spawn_harness_failing(
    config: KanadeConfig,
    fixture: Fixture,
    quit_policy: QuitPolicy,
    fail_on: FailOn,
) -> Harness {
    let shiori = spawn_mock_shiori_failing(fixture, fail_on);

    // kanade→sakura の TalkCommand チャンネルを 1 本張る（DD-5・起動と選択解決が同一チャンネル）。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();

    // kanade を起動（inbox 送信端を得る）。boot prefetch（username 照会・R4.1）は駆動されるが、
    // ハーネスは照会結果を消費しないため no-op sink を注入する（Implementation Notes）。
    let (kanade_tx, kanade_handle) =
        spawn_kanade(config, shiori.sender.clone(), talk_tx, Box::new(|_, _| {}));

    // sink には TalkDone 返送用に kanade inbox 送信端のクローンを渡す。
    let sakura = spawn_mock_sakura(talk_rx, kanade_tx.clone(), quit_policy);

    Harness {
        sender: kanade_tx,
        kanade: kanade_handle,
        shiori,
        sakura,
    }
}

/// 保留 sakura ＋失敗注入 shiori の駆動ハーネスを組み立てる（6.1 専用・両派生の合成）。
///
/// [`spawn_harness_gated`]（保留機能付き sink）と [`spawn_harness_failing`]（失敗注入 shiori）を
/// 同時に効かせる。選択系の失敗経路（DD-12）を統合層で踏むには**両方**が要る:
///
/// - 選択待ち帳簿は「現行 talk と一致する `ChoiceWaiting`」でしか確立しないため、talk を active に
///   保つ保留窓（`hold_indices`）が要る（即応 sink では TalkDone が先着し `Steady{None}` に落ちる）。
/// - その窓で発行される選択由来 GET を語彙付きで失敗させるために失敗注入 shiori が要る。
///
/// `fail_on` の意味論は [`spawn_mock_shiori_failing`] と同一（一致する**最初の**呼出のみ失敗・
/// 以降は良性応答表へ戻る）。
pub fn spawn_harness_gated_failing(
    config: KanadeConfig,
    fixture: Fixture,
    quit_policy: QuitPolicy,
    hold_indices: Vec<usize>,
    fail_on: FailOn,
) -> (Harness, SakuraGate) {
    let shiori = spawn_mock_shiori_failing(fixture, fail_on);

    // kanade→sakura の TalkCommand チャンネルを 1 本張る（DD-5・起動と選択解決が同一チャンネル）。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();

    // kanade を起動（inbox 送信端を得る）。boot prefetch（username 照会・R4.1）は駆動されるが、
    // ハーネスは照会結果を消費しないため no-op sink を注入する（Implementation Notes）。
    let (kanade_tx, kanade_handle) =
        spawn_kanade(config, shiori.sender.clone(), talk_tx, Box::new(|_, _| {}));

    // sink には TalkDone 返送用に kanade inbox 送信端のクローンを渡す（保留機能付き）。
    let (sakura, gate) =
        spawn_mock_sakura_gated(talk_rx, kanade_tx.clone(), quit_policy, hold_indices);

    (
        Harness {
            sender: kanade_tx,
            kanade: kanade_handle,
            shiori,
            sakura,
        },
        gate,
    )
}

/// ブロッキング mock shiori 付き駆動ハーネスを組み立てる（6.3 専用・[`spawn_harness`] の派生）。
///
/// [`spawn_harness`] と同一の結線だが、mock shiori を [`spawn_mock_shiori_blocking`] で起動し、
/// `block_on` が指す最初の呼出を明示解放まで握る。返す [`ShioriGate`] の
/// [`wait_until_blocked`](ShioriGate::wait_until_blocked) で「kanade が round-trip でブロック中」を
/// 確認し、[`release`](ShioriGate::release) で catch-up を解禁できる。これにより「呼出ブロック中に
/// 溜まった Tick が解除後に順次処理される（catch-up・in-flight ≤ 1）」を統合層で観測できる
/// （Req 3.1/3.2・DD-2）。
pub fn spawn_harness_blocking(
    config: KanadeConfig,
    fixture: Fixture,
    quit_policy: QuitPolicy,
    block_on: BlockOn,
) -> (Harness, ShioriGate) {
    let (shiori, gate) = spawn_mock_shiori_blocking(fixture, block_on);

    // kanade→sakura の TalkCommand チャンネルを 1 本張る（DD-5・起動と選択解決が同一チャンネル）。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();

    // kanade を起動（inbox 送信端を得る）。boot prefetch（username 照会・R4.1）は駆動されるが、
    // ハーネスは照会結果を消費しないため no-op sink を注入する（Implementation Notes）。
    let (kanade_tx, kanade_handle) =
        spawn_kanade(config, shiori.sender.clone(), talk_tx, Box::new(|_, _| {}));

    // sink には TalkDone 返送用に kanade inbox 送信端のクローンを渡す。
    let sakura = spawn_mock_sakura(talk_rx, kanade_tx.clone(), quit_policy);

    (
        Harness {
            sender: kanade_tx,
            kanade: kanade_handle,
            shiori,
            sakura,
        },
        gate,
    )
}

/// sakura sink を持たない駆動ハーネス（4.6 case 4 専用・全 Sender drop 経路の観測用）。
///
/// [`spawn_harness`] の sink は TalkDone 返送のため kanade inbox 送信端のクローンを**恒久的に
/// 保持する**。その結果、通常ハーネスでは `Harness.sender` を drop しても kanade inbox は
/// sink のクローン越しに生き続け、kanade は受信待ちのまま止まらない（sink は kanade の
/// StartTalk 送信端 drop＝kanade 停止でしか閉じないため相互待ちになる）。Req 4.9（全 Sender
/// drop → 正常終了）を統合層で観測するには、**kanade inbox のクローンを誰も保持しない**結線が
/// 要る。本ビルダは mock sakura を起動せず、StartTalk 受信端を保持する（drop はしない——drop
/// すると StartTalk 送出が失敗して error! 経路になるだけで停止観測には無関係だが、受信端を
/// 生かしておけば kanade は StartTalk 送出に成功しつつ待機できる）。返す [`sender`](Self::sender)
/// が kanade inbox の**唯一の**送信端であり、これを drop すれば inbox が完全に切断され
/// `run_inbox` が正常終了する（Req 4.9 の構造保証）。
pub struct SinklessHarness {
    /// kanade inbox の**唯一の**送信端（drop すれば inbox が完全切断される）。
    pub sender: Sender<KanadeMsg>,
    /// kanade アクタースレッドの join ハンドル。
    pub kanade: ActorHandle,
    /// mock shiori（停止用送信端・記録アクセサ）。
    pub shiori: MockShiori,
    /// kanade→sakura の [`TalkCommand`] 受信端（保持のみ・sink スレッドは起動しない）。
    /// kanade inbox のクローンを一切生まないため、`sender` drop で inbox を切断できる。
    pub talk_rx: Receiver<TalkCommand>,
}

/// sakura sink を持たない駆動ハーネスを組み立てる（4.6 case 4 専用）。
///
/// mock sakura を起動しないため kanade inbox 送信端のクローンは生じない。返す
/// [`SinklessHarness::sender`] が唯一の inbox 送信端であり、これを drop すれば inbox が完全に
/// 切断され kanade は正常終了する（Req 4.9）。StartTalk 受信端は [`SinklessHarness::talk_rx`]
/// として保持して返す（kanade の StartTalk 送出を失敗させないため）。
pub fn spawn_harness_no_sink(config: KanadeConfig, fixture: Fixture) -> SinklessHarness {
    let shiori = spawn_mock_shiori(fixture);

    // kanade→sakura の TalkCommand チャンネルを 1 本張る（DD-5・起動と選択解決が同一チャンネル。
    // 受信端は sink を起動せず保持する）。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();

    // kanade を起動（inbox 送信端を得る・クローンは作らない）。boot prefetch（R4.1）は駆動されるが
    // 照会結果を消費しないため no-op sink を注入する。
    let (kanade_tx, kanade_handle) =
        spawn_kanade(config, shiori.sender.clone(), talk_tx, Box::new(|_, _| {}));

    SinklessHarness {
        sender: kanade_tx,
        kanade: kanade_handle,
        shiori,
        talk_rx,
    }
}
