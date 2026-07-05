//! 観測ハーネス基盤（設計「検証層 → 観測ハーネス（tests/kanade/）」）。
//!
//! 後続の統合テスト（4.2〜4.6・5）が個別テストファイルを追加するだけで運行表を
//! 決定的に観測できるよう、以下を提供する:
//!
//! - [`RecordedCall`]: 受理された shiori 呼出（Method・id・References）の記録単位。
//! - [`spawn_mock_shiori`]: real と**同一の [`areka_kanade::ShioriMsg`] 型**を受け、
//!   [`Fixture`] 表に従い同梱 `reply` へ**即時**応答する mock shiori アクター
//!   （trait 不要＝Req 5.1 の型レベル差し替え）。受理呼出を [`RecordedCall`] 列へ蓄積。
//! - [`spawn_mock_sakura`]: [`StartTalk`] を受領・記録し、シナリオ指示（quit true/false・
//!   遅延なし）どおり [`KanadeMsg::TalkDone`] を kanade inbox へ返す mock 再生系宛先。
//! - [`expected_call`]: `areka_kanade::events::*` の [`ShioriCall`] を [`RecordedCall`] へ
//!   変換する導出関数（fixture・検証・実装が単一の正本＝events 表を共有・Req 7.1）。
//! - [`run_bounded`] / [`join_bounded`]: 期限付き待機によるハング検出ヘルパ
//!   （どのテストもハングしない・Req 7.3）。
//! - [`Harness`] / [`spawn_harness`]: mock shiori・mock sakura sink・`spawn_kanade` を
//!   組み立てた駆動ハーネス（4.2〜4.6 が再利用する）。
//!
//! # 未使用許容（プレースホルダ段階）
//! 本モジュールが提供する要素の多くは後続タスク（4.2〜4.6・5）の `#[test]` から
//! 初めて使われる。エントリポイントが全モジュールを先出し宣言する 4.1 時点では
//! 一部が未使用となりコンパイル警告を招くため、モジュール全体に
//! `#![allow(dead_code)]` を付す（プレースホルダ段階に限定した意図的許容）。

#![allow(dead_code)]

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use areka_actor::{ActorError, ActorHandle, spawn_actor};
use areka_kanade::{
    KanadeConfig, KanadeMsg, ShioriCall, ShioriMsg, ShioriOutcome, StartTalk, TalkDone, spawn_kanade,
};

/// 期限付き待機の既定上限（mock は即応ゆえ十分に余裕を持たせた保険値）。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// 固定 boot スクリプト（`OnBoot` GET 200 の fixture 応答）。
pub const FIXED_BOOT_SCRIPT: &str = r"\0\s[0]おはよう\e";

/// 定常運転で散発的に返す fixture スクリプト（`OnSecondChange` GET 200）。
pub const FIXED_STEADY_SCRIPT: &str = r"\0\s[0]ふぅ\e";

/// close talk の fixture スクリプト（`OnClose` GET 200・quit シナリオ）。
pub const FIXED_FAREWELL_SCRIPT: &str = r"\0\s[0]またね\e-";

// ============================================================================
// RecordedCall — 受理された shiori 呼出の記録単位
// ============================================================================

/// shiori 呼出の Method 区別（GET / NOTIFY / Unload）。
///
/// [`ShioriMsg::Request`] の [`ShioriCall`] は GET/NOTIFY を型で区別する。
/// [`ShioriMsg::Unload`] は Method を持たないため、記録上は独立の [`Unload`](CallMethod::Unload)
/// として表す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallMethod {
    /// GET 呼出（`ShioriCall::Get`）。
    Get,
    /// NOTIFY 呼出（`ShioriCall::Notify`）。
    Notify,
    /// 正規終了経路（`ShioriMsg::Unload`）。
    Unload,
}

/// 受理された shiori 呼出 1 件の記録（Method・イベント id・References 構成）。
///
/// fixture・検証・実装が単一の正本（events 表）を共有するため、期待値は
/// [`expected_call`] / [`expected_unload`] で `areka_kanade::events::*` から導出する
/// （ハーネス内に期待 References 定数をハードコードしない・Req 7.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCall {
    /// 呼出の Method（GET / NOTIFY / Unload）。
    pub method: CallMethod,
    /// イベント id（`"OnBoot"` 等・Unload では `"Unload"`）。
    pub id: String,
    /// Reference 構成（順序保持）。
    pub references: Vec<String>,
}

impl RecordedCall {
    /// [`ShioriCall`] を記録単位へ変換する（GET/NOTIFY の別と id・References を写す）。
    fn from_call(call: &ShioriCall) -> Self {
        match call {
            ShioriCall::Get { id, references } => RecordedCall {
                method: CallMethod::Get,
                id: (*id).to_string(),
                references: references.clone(),
            },
            ShioriCall::Notify { id, references } => RecordedCall {
                method: CallMethod::Notify,
                id: (*id).to_string(),
                references: references.clone(),
            },
        }
    }
}

/// `areka_kanade::events::*` の [`ShioriCall`] を期待 [`RecordedCall`] へ導出する。
///
/// fixture・assert・実装が単一の正本（events 表）を共有するための唯一の経路。
/// テストは `expected_call(events::on_boot(&config))` のように書き、期待 References を
/// ハーネス側にハードコードしない（Req 7.1）。
pub fn expected_call(call: ShioriCall) -> RecordedCall {
    RecordedCall::from_call(&call)
}

/// Unload 呼出の期待 [`RecordedCall`]（`ShioriMsg::Unload` に対応）。
///
/// Unload は events 表の対象外（GET/NOTIFY を持たない正規終了経路）ゆえ、id は
/// `"Unload"`・References は空で固定する。
pub fn expected_unload() -> RecordedCall {
    RecordedCall {
        method: CallMethod::Unload,
        id: "Unload".to_string(),
        references: Vec::new(),
    }
}

// ============================================================================
// Fixture — イベント id → 応答の対応（シナリオ構成可能）
// ============================================================================

/// mock shiori の応答表（Req 7.1）。シナリオごとに構成する。
///
/// 基調は fixture 表どおり（OnInitialize→Notified／OnFirstBoot→204／OnBoot→固定 Value／
/// basewareversion→Notified／OnSecondChange→204 基調／Unload→Unloaded）。可変部は
/// 次の 2 点:
///
/// - `steady_value_indices`: `OnSecondChange`（GET・talk 再生可能）呼出のうち Value を
///   返す 0 始まりの出現インデックス集合。含まれない出現は 204（`NoContent`）。
///   NOTIFY で来た `OnSecondChange`（talk 再生不能時）は常に `Notified`（応答は破棄される）。
/// - `close_quits`: `OnClose` を Value（別れの talk・quit シナリオ）で返すなら `true`、
///   無言終了（204）なら `false`。
#[derive(Debug, Clone)]
pub struct Fixture {
    /// `OnBoot` GET 200 の固定スクリプト。
    pub boot_script: String,
    /// `OnSecondChange`（GET）で Value を返す出現インデックス集合（0 始まり）。
    pub steady_value_indices: Vec<usize>,
    /// `OnSecondChange` GET 200 の固定スクリプト。
    pub steady_script: String,
    /// `OnClose` を Value（quit talk）で返すなら true・204（無言終了）なら false。
    pub close_quits: bool,
    /// `OnClose` GET 200（quit シナリオ）の固定スクリプト。
    pub farewell_script: String,
}

impl Default for Fixture {
    /// 既定シナリオ: 散発 Value なし・無言 close（最小の疎通に足る保守的既定）。
    fn default() -> Self {
        Fixture {
            boot_script: FIXED_BOOT_SCRIPT.to_string(),
            steady_value_indices: Vec::new(),
            steady_script: FIXED_STEADY_SCRIPT.to_string(),
            close_quits: false,
            farewell_script: FIXED_FAREWELL_SCRIPT.to_string(),
        }
    }
}

impl Fixture {
    /// quit シナリオ（`OnClose`→別れの talk→終了）の構成を返す。
    pub fn quitting() -> Self {
        Fixture {
            close_quits: true,
            ..Fixture::default()
        }
    }

    /// 指定した `OnSecondChange`（GET）出現で Value を返すよう構成する（連鎖記法）。
    pub fn with_steady_value_indices(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.steady_value_indices = indices.into_iter().collect();
        self
    }
}

/// fixture 適用の可変状態（`OnSecondChange` GET の出現回数を数える）。
struct FixtureState {
    fixture: Fixture,
    second_change_get_seen: usize,
}

impl FixtureState {
    fn new(fixture: Fixture) -> Self {
        FixtureState {
            fixture,
            second_change_get_seen: 0,
        }
    }

    /// 1 件の [`ShioriCall`] に対する応答を fixture 表から決定する（即時応答値）。
    fn respond(&mut self, call: &ShioriCall) -> ShioriOutcome {
        match call {
            ShioriCall::Notify { .. } => {
                // NOTIFY は完了応答のみ（Value を運ばない＝talk 非生成の構造保証）。
                ShioriOutcome::Notified
            }
            ShioriCall::Get { id, .. } => match *id {
                "OnFirstBoot" => ShioriOutcome::NoContent,
                "OnBoot" => ShioriOutcome::Value(self.fixture.boot_script.clone()),
                "OnSecondChange" => {
                    let index = self.second_change_get_seen;
                    self.second_change_get_seen += 1;
                    if self.fixture.steady_value_indices.contains(&index) {
                        ShioriOutcome::Value(self.fixture.steady_script.clone())
                    } else {
                        ShioriOutcome::NoContent
                    }
                }
                "OnClose" => {
                    if self.fixture.close_quits {
                        ShioriOutcome::Value(self.fixture.farewell_script.clone())
                    } else {
                        ShioriOutcome::NoContent
                    }
                }
                // 未知 GET は 204（保守的既定・M1 の対象イベントは上記で網羅）。
                _ => ShioriOutcome::NoContent,
            },
        }
    }
}

// ============================================================================
// mock shiori アクター
// ============================================================================

/// mock shiori アクターのハンドル群（送信端・join ハンドル・記録アクセサ）。
pub struct MockShiori {
    /// kanade へ渡す shiori inbox の送信端（real と同一の [`ShioriMsg`] 型）。
    pub sender: Sender<ShioriMsg>,
    /// アクタースレッドの join ハンドル。
    pub handle: ActorHandle,
    /// 受理した呼出の記録列（`(method, id, references)`）。
    calls: Arc<Mutex<Vec<RecordedCall>>>,
}

impl MockShiori {
    /// 受理済み呼出の記録スナップショットを返す。
    pub fn recorded(&self) -> Vec<RecordedCall> {
        self.calls.lock().expect("mock shiori calls mutex").clone()
    }
}

/// mock shiori アクターを起動する。
///
/// real と同一の [`ShioriMsg`] 型を受け、[`Fixture`] 表に従い同梱 `reply` へ即時応答する。
/// 各 [`ShioriMsg::Request`] は `(method, id, references)` を記録列へ蓄積し、fixture の
/// 応答を `reply.send` で返す。[`ShioriMsg::Unload`] は記録の上 `Unloaded` を返す。
/// [`ShioriMsg::Close`] で受信ループを終了する。
pub fn spawn_mock_shiori(fixture: Fixture) -> MockShiori {
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_body = Arc::clone(&calls);

    let (sender, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori", move |rx| {
        let mut state = FixtureState::new(fixture);
        // 受信ループ: 即時応答（sleep なし・決定的）。
        while let Ok(msg) = rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(RecordedCall::from_call(&call));
                    let outcome = state.respond(&call);
                    let _ = reply.send(outcome);
                }
                ShioriMsg::Unload { reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(expected_unload());
                    let _ = reply.send(ShioriOutcome::Unloaded);
                }
                ShioriMsg::Close => break,
            }
        }
    });

    MockShiori {
        sender,
        handle,
        calls,
    }
}

// ============================================================================
// mock sakura sink
// ============================================================================

/// mock sakura sink のハンドル群（join ハンドル・受領 talk のアクセサ）。
///
/// sink は別スレッドで [`StartTalk`] を読み、受領を記録した上でシナリオ指示（quit 真偽）に
/// 応じて [`KanadeMsg::TalkDone`] を kanade inbox へ返す。sink スレッドは `StartTalk` の
/// 全 Sender drop（＝kanade 停止）で自然終了する。
pub struct MockSakura {
    join: thread::JoinHandle<()>,
    started: Arc<Mutex<Vec<StartTalk>>>,
}

impl MockSakura {
    /// 受領した [`StartTalk`] の記録スナップショットを返す。
    pub fn started(&self) -> Vec<StartTalk> {
        self.started.lock().expect("mock sakura mutex").clone()
    }

    /// sink スレッドの終了を待つ（`StartTalk` 送信端が全て drop された後に完了する）。
    pub fn join_bounded(self, what: &str, timeout: Duration) {
        let MockSakura { join, .. } = self;
        run_join_bounded(what, timeout, move || {
            let _ = join.join();
        });
    }
}

/// quit フラグの決定方式（シナリオ指示）。
#[derive(Debug, Clone)]
pub enum QuitPolicy {
    /// 受領した全 talk で quit フラグを固定値にする。
    Fixed(bool),
    /// n 番目（0 始まり）の受領 talk の quit フラグを個別指定する（範囲外は false）。
    PerTalk(Vec<bool>),
}

impl QuitPolicy {
    /// n 番目（0 始まり）の talk に対する quit フラグを返す。
    fn quit_for(&self, index: usize) -> bool {
        match self {
            QuitPolicy::Fixed(q) => *q,
            QuitPolicy::PerTalk(flags) => flags.get(index).copied().unwrap_or(false),
        }
    }
}

/// mock sakura sink を起動する。
///
/// `talk_rx`（kanade→sakura の [`StartTalk`] 受信端）を別スレッドで読み、各受領を記録した
/// 上で `quit_policy` に従った [`TalkDone`] を `kanade_tx` 経由で kanade inbox へ返す
/// （遅延なし・即時）。`kanade_tx` は TalkDone 返送のためだけに保持するクローンでよい。
pub fn spawn_mock_sakura(
    talk_rx: Receiver<StartTalk>,
    kanade_tx: Sender<KanadeMsg>,
    quit_policy: QuitPolicy,
) -> MockSakura {
    let started: Arc<Mutex<Vec<StartTalk>>> = Arc::new(Mutex::new(Vec::new()));
    let started_body = Arc::clone(&started);

    let join = thread::Builder::new()
        .name("mock-sakura".to_string())
        .spawn(move || {
            let mut index = 0usize;
            // 全 StartTalk Sender drop（kanade 停止）で recv が Err→ループ終了。
            while let Ok(start) = talk_rx.recv() {
                let talk_id = start.talk_id;
                started_body
                    .lock()
                    .expect("mock sakura mutex")
                    .push(start);
                let quit = quit_policy.quit_for(index);
                index += 1;
                // TalkDone 返送。kanade 停止済みで送れなくても無害（続行）。
                let _ = kanade_tx.send(KanadeMsg::TalkDone(TalkDone { talk_id, quit }));
            }
        })
        .expect("spawn mock-sakura thread");

    MockSakura { join, started }
}

// ============================================================================
// 保留付き mock sakura sink（active talk 窓を決定的に作る・4.4 pattern 3）
// ============================================================================

/// park した [`TalkDone`] の共有状態（保留分の待ち行列＋解放フラグ・Condvar）。
///
/// sakura の recv ループスレッドが park を積み、[`SakuraGate::release_all`] が flag を立て、
/// 専用の releaser スレッドが flag 成立を待って一斉送出する。この 3 者を繋ぐ共有点。
struct GateShared {
    /// (解放フラグ＋park 待ち行列, 起床用 Condvar)。
    ///
    /// bool=解放許可・`Vec<TalkDone>`=まだ送っていない保留 talk（recv ループが積む）。
    inner: Mutex<GateInner>,
    cvar: std::sync::Condvar,
}

/// [`GateShared`] の Mutex 保護部（解放フラグと park 待ち行列）。
struct GateInner {
    /// テストが release_all を呼んだら true（一度立てたら以後 true のまま）。
    released: bool,
    /// recv ループが積んだ保留 [`TalkDone`]（releaser が解放時に drain して送る）。
    parked: Vec<TalkDone>,
    /// recv ループが終了（全 StartTalk Sender drop）したら true。releaser の終了条件。
    recv_closed: bool,
    /// 解放時に送るべき保留 talk の総数（`hold_indices` の要素数）。releaser は
    /// 「解放済み **かつ** `parked.len()` がこの数に達した」時点で初めて drain する。
    ///
    /// これがないと、`release_all` が「保留 talk がまだ park される前」に呼ばれた場合
    /// （kanade が Tick を非同期処理する以上あり得る）、releaser が空の `parked` を drain して
    /// 終了し、後から park された TalkDone が永久に送られず kanade が `Steady{Some}` で
    /// 宙吊りになる（決定性を壊す race）。expected_holds を待つことでこの race を閉じる。
    expected_holds: usize,
}

/// 保留 talk の解放を sakura の releaser スレッドへ通知するゲート（4.4 pattern 3 専用）。
///
/// [`spawn_mock_sakura_gated`] / [`spawn_harness_gated`] が返す。テストは
/// [`release_all`](SakuraGate::release_all) を呼んで、保留していた全 [`TalkDone`] を
/// （各 talk の per-policy quit フラグ付きで）kanade inbox へ返送させる。
///
/// # 決定性（sleep 不要・race-free）
/// 「保留」は、当該 talk の [`TalkDone`] を kanade inbox へ**送らない**ことで実現する。
/// TalkDone は「二つの Tick の間に割り込み得る唯一のメッセージ」であり、これを送らない限り
/// active talk 窓（`Steady{Some}`）は次 Tick まで確実に維持される（interleaving が起きない）。
/// 解放シグナルは `Mutex<bool>`＋`Condvar` で伝える——flag を Mutex 下で立ててから notify する
/// ため lost wakeup は起きない。
pub struct SakuraGate {
    shared: Arc<GateShared>,
}

impl SakuraGate {
    /// 保留中の全 [`TalkDone`] の解放を releaser スレッドへ通知する（sleep なし・確実に起床）。
    ///
    /// flag を Mutex 下で `true` にしてから `notify_all` するため、releaser がまだ wait に
    /// 入る前に呼ばれても取りこぼさない（lost wakeup 対策）。解放後に積まれる park は無い
    /// （テストは全保留 talk 起動後に本メソッドを呼ぶ契約）。
    pub fn release_all(&self) {
        let mut inner = self.shared.inner.lock().expect("sakura gate mutex");
        inner.released = true;
        self.shared.cvar.notify_all();
    }
}

/// 保留機能付き mock sakura sink を起動する（4.4 pattern 3 専用・[`spawn_mock_sakura`] の派生）。
///
/// `hold_indices` に含まれる受領インデックス（0 始まり）の [`StartTalk`] は、記録はするが
/// その [`TalkDone`] を**即座には返さない**（park する）。[`SakuraGate::release_all`] が
/// 呼ばれると、park した全 TalkDone を per-policy quit フラグ付きで返送する。`hold_indices` に
/// 含まれない talk は従来どおり即応する。
///
/// # スレッド構成
/// recv ループスレッド（[`MockSakura::join`] が待つ本体）は従来同様 `talk_rx.recv()` を回して
/// 記録・即応・park を行う。park の**送出**は別の releaser スレッドが担う——recv ループは
/// `recv` で恒常的にブロックし得るため、release_all を実行中の recv ループ内で拾えないからで
/// ある。releaser は「解放フラグ成立」または「recv ループ終了」で起床し、park を送って自然終了
/// する。テストは Sender drop の前に `release_all` を呼ぶ契約（それにより pattern 3 の
/// `Steady{None}` 復帰→close 握手が駆動される）。
pub fn spawn_mock_sakura_gated(
    talk_rx: Receiver<StartTalk>,
    kanade_tx: Sender<KanadeMsg>,
    quit_policy: QuitPolicy,
    hold_indices: Vec<usize>,
) -> (MockSakura, SakuraGate) {
    let started: Arc<Mutex<Vec<StartTalk>>> = Arc::new(Mutex::new(Vec::new()));
    let started_body = Arc::clone(&started);

    let expected_holds = hold_indices.len();
    let shared = Arc::new(GateShared {
        inner: Mutex::new(GateInner {
            released: false,
            parked: Vec::new(),
            recv_closed: false,
            expected_holds,
        }),
        cvar: std::sync::Condvar::new(),
    });
    let shared_recv = Arc::clone(&shared);
    let shared_releaser = Arc::clone(&shared);

    // releaser: 「解放済み かつ 保留 talk が全て park された」まで待ち、park された TalkDone を
    // 送出して終了する。recv ループ終了（recv_closed）でも起床する（安全弁: 保留が揃わないまま
    // kanade が停止した誤用時に宙吊りしないため）。
    let releaser_kanade_tx = kanade_tx.clone();
    let releaser = thread::Builder::new()
        .name("mock-sakura-releaser".to_string())
        .spawn(move || {
            let mut inner = shared_releaser
                .inner
                .lock()
                .expect("sakura gate mutex");
            // 起床条件:
            //   (a) 解放済み かつ 全保留 talk が park された（＝正規の解放点）、または
            //   (b) recv ループ終了（安全弁・kanade 停止で二度と park は増えない）。
            while !(inner.released && inner.parked.len() >= inner.expected_holds)
                && !inner.recv_closed
            {
                inner = shared_releaser
                    .cvar
                    .wait(inner)
                    .expect("sakura gate condvar wait");
            }
            let to_send: Vec<TalkDone> = inner.parked.drain(..).collect();
            drop(inner);
            for done in to_send {
                // kanade 停止済みで送れなくても無害（続行）。
                let _ = releaser_kanade_tx.send(KanadeMsg::TalkDone(done));
            }
        })
        .expect("spawn mock-sakura-releaser thread");

    let join = thread::Builder::new()
        .name("mock-sakura-gated".to_string())
        .spawn(move || {
            let mut index = 0usize;
            // 全 StartTalk Sender drop（kanade 停止）で recv が Err→ループ終了。
            while let Ok(start) = talk_rx.recv() {
                let talk_id = start.talk_id;
                started_body
                    .lock()
                    .expect("mock sakura mutex")
                    .push(start);
                let quit = quit_policy.quit_for(index);
                let this_index = index;
                index += 1;
                let done = TalkDone { talk_id, quit };
                if hold_indices.contains(&this_index) {
                    // 保留: TalkDone を park し、解放シグナルまで送らない（active talk 窓を作る）。
                    // park のたびに releaser を起こす（「解放済み かつ 全 park 到着」を再評価させる・
                    // release_all が park より先行しても取りこぼさない）。
                    let mut inner = shared_recv.inner.lock().expect("sakura gate mutex");
                    inner.parked.push(done);
                    shared_recv.cvar.notify_all();
                } else {
                    // 非保留: 従来どおり即応。kanade 停止済みで送れなくても無害（続行）。
                    let _ = kanade_tx.send(KanadeMsg::TalkDone(done));
                }
            }
            // recv 終了を releaser へ通知（park が残っていて未解放でも宙吊りにしない）。
            {
                let mut inner = shared_recv.inner.lock().expect("sakura gate mutex");
                inner.recv_closed = true;
                shared_recv.cvar.notify_all();
            }
            // releaser の後始末（park 送出）を見届けてから本体スレッドを終える。
            let _ = releaser.join();
        })
        .expect("spawn mock-sakura-gated thread");

    (MockSakura { join, started }, SakuraGate { shared })
}

// ============================================================================
// ハング検出ヘルパ
// ============================================================================

/// 期限付き待機ヘルパ: 別スレッドで `f` を走らせ、期限内に完了しなければテストを
/// 失敗させる（どのテストもハングしない・Req 7.3）。areka-actor のテスト慣行に倣う。
pub fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    run_join_bounded(what, timeout, f);
}

/// 内部: クロージャを別スレッドで走らせ、`recv_timeout` の期限で完了を判定する。
fn run_join_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    use std::sync::mpsc::sync_channel;
    let (done_tx, done_rx) = sync_channel::<()>(0);
    thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// [`ActorHandle`] を期限付きで join する（ハングせず結果を返す）。
///
/// 停止駆動（Close 送信／全 Sender drop）を先に済ませてから呼ぶこと。期限内に join が
/// 完了しなければテストを失敗させる。
pub fn join_bounded(what: &str, timeout: Duration, handle: ActorHandle) -> Result<(), ActorError> {
    use std::sync::mpsc::sync_channel;
    let (res_tx, res_rx) = sync_channel::<Result<(), ActorError>>(0);
    thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
    }
}

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

    // kanade→sakura の StartTalk チャンネルを 1 本張る。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<StartTalk>();

    // kanade を起動（inbox 送信端を得る）。
    let (kanade_tx, kanade_handle) = spawn_kanade(config, shiori.sender.clone(), talk_tx);

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

    // kanade→sakura の StartTalk チャンネルを 1 本張る。
    let (talk_tx, talk_rx) = std::sync::mpsc::channel::<StartTalk>();

    // kanade を起動（inbox 送信端を得る）。
    let (kanade_tx, kanade_handle) = spawn_kanade(config, shiori.sender.clone(), talk_tx);

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

// ============================================================================
// 疎通テスト（観測可能な完了条件）
// ============================================================================

#[cfg(test)]
mod smoke {
    use super::*;
    use areka_actor::reply_channel;
    use areka_kanade::{TalkId, events};

    /// mock shiori 単独駆動: `OnBoot` GET へ即時に固定 Value を返し、記録が
    /// events 表から導出した期待値と一致する（fixture・assert・実装の三点一正本）。
    #[test]
    fn mock_shiori_replies_and_records_onboot() {
        let config = KanadeConfig::new("master", "1.0.0");
        let shiori = spawn_mock_shiori(Fixture::default());

        // events 表から GET OnBoot の呼出を導出して送る（ハードコードしない）。
        let call = events::on_boot(&config);
        let expected = expected_call(events::on_boot(&config));

        let (reply, receiver) = reply_channel::<ShioriOutcome>();
        shiori
            .sender
            .send(ShioriMsg::Request { call, reply })
            .expect("send request to mock shiori");

        // 応答は即時（期限付きで受ける・ハングしない）。
        let outcome = receiver
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("mock shiori should reply immediately");
        match outcome {
            ShioriOutcome::Value(script) => assert_eq!(script, FIXED_BOOT_SCRIPT),
            // ShioriOutcome は Debug 非実装ゆえ、非 Value は総称メッセージで報告する。
            _ => panic!("expected Value(boot script) from mock shiori OnBoot"),
        }

        // 記録列が events 導出の期待値と一致する。
        let recorded = shiori.recorded();
        assert_eq!(recorded, vec![expected.clone()]);
        assert_eq!(recorded[0].method, CallMethod::Get);
        assert_eq!(recorded[0].id, "OnBoot");
        assert_eq!(recorded[0].references, vec!["master".to_string()]);

        // 停止駆動（Close）→期限付き join（ハングしない）。
        shiori
            .sender
            .send(ShioriMsg::Close)
            .expect("send close to mock shiori");
        join_bounded("mock-shiori join", DEFAULT_TIMEOUT, shiori.handle)
            .expect("mock shiori body completes normally");
    }

    /// mock sakura sink 単独駆動: [`StartTalk`] を受領・記録し、シナリオ指示どおり
    /// [`KanadeMsg::TalkDone`] を返す。
    #[test]
    fn mock_sakura_records_and_returns_talkdone() {
        let (talk_tx, talk_rx) = std::sync::mpsc::channel::<StartTalk>();
        // sink → TalkDone を受け取るための疑似 kanade inbox。
        let (kanade_tx, kanade_rx) = std::sync::mpsc::channel::<KanadeMsg>();

        let sakura = spawn_mock_sakura(talk_rx, kanade_tx, QuitPolicy::Fixed(true));

        talk_tx
            .send(StartTalk {
                talk_id: TalkId(1),
                script: FIXED_BOOT_SCRIPT.to_string(),
            })
            .expect("send StartTalk to mock sakura");

        // TalkDone が即時に返る（期限付き・ハングしない）。
        let msg = kanade_rx
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("mock sakura should emit TalkDone immediately");
        match msg {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(done.talk_id, TalkId(1));
                assert!(done.quit, "QuitPolicy::Fixed(true) should set quit");
            }
            _ => panic!("expected TalkDone from mock sakura"),
        }

        // 記録に受領 talk が残る。
        let started = sakura.started();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].talk_id, TalkId(1));
        assert_eq!(started[0].script, FIXED_BOOT_SCRIPT);

        // StartTalk 送信端を drop → sink スレッドは自然終了（期限付き join）。
        drop(talk_tx);
        sakura.join_bounded("mock-sakura join", DEFAULT_TIMEOUT);
    }
}
