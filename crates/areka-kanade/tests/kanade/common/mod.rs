//! 観測ハーネス基盤（設計「検証層 → 観測ハーネス（tests/kanade/）」）。
//!
//! 後続の統合テスト（4.2〜4.6・5）が個別テストファイルを追加するだけで運行表を
//! 決定的に観測できるよう、以下を提供する:
//!
//! - [`RecordedCall`]: 受理された shiori 呼出（Method・id・References）の記録単位。
//! - [`spawn_mock_shiori`]: real と**同一の [`areka_kanade::ShioriMsg`] 型**を受け、
//!   [`Fixture`] 表に従い同梱 `reply` へ**即時**応答する mock shiori アクター
//!   （trait 不要＝Req 5.1 の型レベル差し替え）。受理呼出を [`RecordedCall`] 列へ蓄積。
//! - [`spawn_mock_sakura`]: [`TalkCommand`] を到着順に受領・記録し、`Start` についてシナリオ指示（quit true/false・
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

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use areka_actor::{ActorError, ActorHandle, spawn_actor};
use areka_kanade::{
    KanadeConfig, KanadeMsg, MonotonicMs, ShioriCall, ShioriFailure, ShioriMsg, ShioriOutcome,
    StartTalk, TalkCommand, TalkDone, TalkEndReason, spawn_kanade,
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
    /// 送出時の `Status` 実行状態集合の wire 値（`None` ⇔ ヘッダ行なし・5.1）。
    ///
    /// `ExecutionStatus::render()` の結果を写す。これにより mock が記録した呼出と
    /// [`expected_call`] 導出の期待値が Status ヘッダまで含めて突合される
    /// （Testing Strategy #15・DD-IT-3/DD-IT-5）。Unload は Status を持たないため `None`。
    pub status: Option<String>,
}

impl RecordedCall {
    /// [`ShioriCall`] を記録単位へ変換する（GET/NOTIFY の別と id・References・Status を写す）。
    fn from_call(call: &ShioriCall) -> Self {
        match call {
            ShioriCall::Get {
                id,
                references,
                status,
            } => RecordedCall {
                method: CallMethod::Get,
                id: id.as_str().to_string(),
                references: references.clone(),
                status: status.render(),
            },
            ShioriCall::Notify {
                id,
                references,
                status,
            } => RecordedCall {
                method: CallMethod::Notify,
                id: id.as_str().to_string(),
                references: references.clone(),
                status: status.render(),
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
        // Unload は Status ヘッダを持たない正規終了経路（5.1）。
        status: None,
    }
}

// ============================================================================
// Fixture — イベント id → 応答の対応（シナリオ構成可能）
// ============================================================================

/// マウス GET（`OnMouseMove`／`OnMouseDoubleClick`）へ注入する応答パターン（4.1）。
///
/// 4.2／4.3 の檻がマウス GET へ「talk スクリプト応答」または「無応答（204）」を任意に
/// 注入するための語彙。`Fixture` のマウス応答表（[`Fixture::mouse_responses`]）の値として
/// 用いる。既存の boot／steady／close 応答（それぞれ専用フィールド）と同じく、mock は
/// この記述子どおりの [`ShioriOutcome`] を即応する（設計「Fixture へマウス応答（script／204）の
/// additive 拡張」）。
#[derive(Debug, Clone)]
pub enum MouseResponse {
    /// talk スクリプト Value を返す（`Steady{None}` から StartTalk を起こす・Req 8.1(c)）。
    Script(String),
    /// 204 / NoContent を返す（無応答・StartTalk 不発・Req 8.1(d)）。
    NoContent,
}

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
    /// `OnBoot` が起動挨拶 Value を返すか（DD-IT-12）。`true`＝固定スクリプトの Value（挨拶 talk を
    /// 起こし `Steady{talk: Some(_)}` へ完了）。`false`＝204（挨拶なし・`Steady{talk: None}` へ直行）。
    ///
    /// DD-IT-12 で boot は挨拶 talk を正規追跡するようになった。挨拶 talk の TalkDone は mock sakura
    /// が別スレッドから返すため、その到着は後続 Tick と inbox 上で競合する（GET/NOTIFY・Ref3 が
    /// 非決定になる）。定常 pump（`Steady{None}` の GET・Req 2.1/2.3/3.3）を**決定的に**観測する
    /// テストは、この挨拶を出さない（`false`）ことで boot→`Steady{None}` へ直行させ競合を発生源から
    /// 断つ（設計 Testing Strategy「boot が 204 を返す fixture」）。挨拶 boot 自体の観測は
    /// boot_test／full_run_test が担う。
    pub boot_greets: bool,
    /// `OnSecondChange`（GET）で Value を返す出現インデックス集合（0 始まり）。
    pub steady_value_indices: Vec<usize>,
    /// `OnSecondChange` GET 200 の固定スクリプト。
    pub steady_script: String,
    /// `OnClose` を Value（quit talk）で返すなら true・204（無言終了）なら false。
    pub close_quits: bool,
    /// `OnClose` GET 200（quit シナリオ）の固定スクリプト。
    pub farewell_script: String,
    /// マウス GET id（`"OnMouseMove"`／`"OnMouseDoubleClick"`）→ 注入応答の対応（4.1）。
    ///
    /// 含まれない mouse id は 204（`NoContent`）——未注入既定は従来の catch-all（未知 GET＝204）と
    /// 同値ゆえ additive（既存 consumer は mouse GET を発しないので無影響）。4.2／4.3 の檻が
    /// [`Fixture::with_mouse_response`] でイベント別に script／204 を注入する。
    pub mouse_responses: HashMap<&'static str, MouseResponse>,
}

impl Default for Fixture {
    /// 既定シナリオ: 起動挨拶あり・散発 Value なし・無言 close（最小の疎通に足る保守的既定）。
    fn default() -> Self {
        Fixture {
            boot_script: FIXED_BOOT_SCRIPT.to_string(),
            boot_greets: true,
            steady_value_indices: Vec::new(),
            steady_script: FIXED_STEADY_SCRIPT.to_string(),
            close_quits: false,
            farewell_script: FIXED_FAREWELL_SCRIPT.to_string(),
            mouse_responses: HashMap::new(),
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

    /// 起動挨拶（`OnBoot` Value）を出さない構成にする（`OnBoot`→204・DD-IT-12・連鎖記法）。
    ///
    /// boot→`Steady{talk: None}` へ直行させ、挨拶 talk の TalkDone と後続 Tick の競合を発生源から
    /// 断つ。定常 pump（`Steady{None}` の GET）を決定的に観測するテスト専用（設計 Testing Strategy）。
    pub fn without_boot_greeting(mut self) -> Self {
        self.boot_greets = false;
        self
    }

    /// マウス GET id へ注入応答（script Value ／ 204）を設定する（4.1・連鎖記法）。
    ///
    /// `id` は `"OnMouseMove"` ／ `"OnMouseDoubleClick"`。同一 id への再指定は後勝ちで上書きする。
    /// 未設定の mouse id は 204（`NoContent`）のまま——4.2／4.3 が「talk 応答」「無応答」の両
    /// パターンをイベント別に注入するための唯一の口（設計 Testing Strategy #4「204→無動作」／
    /// Integration #1「Value→StartTalk」）。
    pub fn with_mouse_response(mut self, id: &'static str, response: MouseResponse) -> Self {
        self.mouse_responses.insert(id, response);
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
            ShioriCall::Get { id, .. } => match id.as_str() {
                "OnFirstBoot" => ShioriOutcome::NoContent,
                // DD-IT-12: 挨拶ありは固定 Value（`Steady{Some}` 完了）、なしは 204（`Steady{None}` 直行）。
                "OnBoot" => {
                    if self.fixture.boot_greets {
                        ShioriOutcome::Value(self.fixture.boot_script.clone())
                    } else {
                        ShioriOutcome::NoContent
                    }
                }
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
                // マウス GET は fixture の注入表を引く（未注入は 204・4.1）。script Value は
                // 既存 talk 起動棚（Steady の StartTalk）へそのまま載る（Req 8.1(c)/(d)）。
                mouse_id @ ("OnMouseMove" | "OnMouseDoubleClick") => {
                    match self.fixture.mouse_responses.get(mouse_id) {
                        Some(MouseResponse::Script(script)) => {
                            ShioriOutcome::Value(script.clone())
                        }
                        Some(MouseResponse::NoContent) | None => ShioriOutcome::NoContent,
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
// 失敗注入付き mock shiori（4.6 専用・区別語彙ごとの呼出失敗を観測する）
// ============================================================================

/// 失敗注入の区別語彙（[`ShioriFailure`] の 5 種に 1:1 対応する COPYABLE 記述子）。
///
/// [`ShioriFailure`] 自体は `Clone` を実装しないため（かつ `Fixture` は `Clone` 派生ゆえ
/// 中に持てない）、fixture 側にはこの Copy な種別のみを持たせ、実 `ShioriFailure` は mock の
/// `respond` 内でその都度 fresh に構築する（設計「Error Categories」の 5 語彙）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailKind {
    /// [`ShioriFailure::Handshake`]（接続確立失敗）。
    Handshake,
    /// [`ShioriFailure::Timeout`]（タイムアウト）。
    Timeout,
    /// [`ShioriFailure::Ipc`]（helper 死活の一態様）。
    Ipc,
    /// [`ShioriFailure::Shiori`]（SHIORI エラー応答）。
    Shiori,
    /// [`ShioriFailure::Internal`]（kanade 内部規律違反・境界写像では生成されない・DD-IT-11）。
    /// 5.1 は記述子のみ追加し語彙を完全化する（実掃引は 5.2 の担当）。
    Internal,
}

impl FailKind {
    /// この種別に対応する fresh な [`ShioriFailure`] を構築する（`Clone` 不要ゆえ都度生成）。
    fn make(self) -> ShioriFailure {
        match self {
            FailKind::Handshake => ShioriFailure::Handshake("injected handshake failure".into()),
            FailKind::Timeout => ShioriFailure::Timeout("injected timeout".into()),
            FailKind::Ipc => ShioriFailure::Ipc("injected ipc failure".into()),
            FailKind::Shiori => ShioriFailure::Shiori("injected shiori error".into()),
            FailKind::Internal => ShioriFailure::Internal("injected internal violation".into()),
        }
    }
}

/// 失敗注入 mock shiori の記述子（どのイベント id の呼出をどの語彙で落とすか）。
///
/// `fail_on_id` に一致する **最初の** GET／NOTIFY 呼出で [`ShioriOutcome::Failed`] を返し、
/// それ以外の呼出は良性応答（[`FixtureState::respond`] と同一の既定表）を返す。全て Copy／
/// `&'static str` で構成され `ShioriFailure`（非 Clone）を保持しないため、`Fixture` 同様に
/// 気軽に複製できる。
#[derive(Debug, Clone, Copy)]
pub struct FailOn {
    /// 落とす対象のイベント id（例: `"OnInitialize"`）。
    pub id: &'static str,
    /// 返す失敗の区別語彙。
    pub kind: FailKind,
}

impl FailOn {
    /// `OnInitialize`（boot 最初の NOTIFY）を指定語彙で落とす記述子を返す。
    ///
    /// boot 系列の最初の呼出（`OnInitialize` NOTIFY・Req 1.1）を対象にすることで、Boot 駆動が
    /// 確実にこの呼出へ到達し、失敗が boot 応答待ち（`awaits_reply`）で受領され Unloading{Fault}
    /// へ倒れる経路を確実に踏む（設計「Error Categories」ShioriFailure 行）。
    pub fn on_initialize(kind: FailKind) -> Self {
        FailOn {
            id: "OnInitialize",
            kind,
        }
    }
}

/// 失敗注入付き mock shiori アクターを起動する（[`spawn_mock_shiori`] の派生・4.6 専用）。
///
/// `fixture` の良性応答表に加え、`fail_on.id` に一致する最初の呼出で
/// [`ShioriOutcome::Failed`]（`fail_on.kind` に対応する fresh な [`ShioriFailure`]）を返す。
/// 呼出の記録・[`ShioriMsg::Unload`]／[`ShioriMsg::Close`] の扱いは [`spawn_mock_shiori`] と同一
/// （Unload は best-effort で `Unloaded` を返し記録する）。これにより「区別語彙ごとの呼出失敗
/// → 観測可能な終了（Unloading{Fault}→Unload→Stopped）」を統合層で観測できる（Req 6.1）。
pub fn spawn_mock_shiori_failing(fixture: Fixture, fail_on: FailOn) -> MockShiori {
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_body = Arc::clone(&calls);

    let (sender, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori-failing", move |rx| {
        let mut state = FixtureState::new(fixture);
        let mut failed_once = false;
        while let Ok(msg) = rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(RecordedCall::from_call(&call));
                    // 対象イベントの最初の呼出のみ失敗させる（以降は良性表へ戻す）。
                    let outcome = if !failed_once && call_id(&call) == fail_on.id {
                        failed_once = true;
                        ShioriOutcome::Failed(fail_on.kind.make())
                    } else {
                        state.respond(&call)
                    };
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

/// [`ShioriCall`] のイベント id の wire 形を取り出す（GET／NOTIFY 共通・失敗注入の突合用）。
fn call_id(call: &ShioriCall) -> &str {
    match call {
        ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id.as_str(),
    }
}

// ============================================================================
// ブロッキング mock shiori（6.3 専用・呼出ブロック中の Tick catch-up を観測する）
// ============================================================================

/// ブロック対象の記述子（どの呼出を握り続けるか・COPYABLE）。
///
/// 実経路の SHIORI 呼出は本質的にブロックし得る（別プロセス往復）。mock は既定で即応するため
/// その窓が存在しない。本記述子で「特定の 1 呼出」を選び、その往復を明示的な解放まで握って
/// kanade を drive ループの `ReplyReceiver::recv` で止め、以降の Tick が inbox に溜まる catch-up
/// 窓を決定的に作る。ブロックするのは各種別で**最初の**一致呼出のみ（以降は即応へ戻す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOn {
    /// 指定 id の最初の GET 呼出をブロックする（例: `"OnSecondChange"`）。
    Get(&'static str),
    /// 指定 id の最初の NOTIFY 呼出をブロックする。
    Notify(&'static str),
}

impl BlockOn {
    /// この呼出が本記述子の対象か（method＋id 一致）を判定する。
    fn matches(&self, call: &ShioriCall) -> bool {
        match (self, call) {
            (BlockOn::Get(id), ShioriCall::Get { id: cid, .. }) => cid.as_str() == *id,
            (BlockOn::Notify(id), ShioriCall::Notify { id: cid, .. }) => cid.as_str() == *id,
            _ => false,
        }
    }
}

/// ブロック解放を mock shiori の受信ループへ伝えるゲートの共有部（Mutex＋Condvar）。
struct ShioriGateShared {
    inner: Mutex<ShioriGateInner>,
    cvar: std::sync::Condvar,
}

/// [`ShioriGateShared`] の Mutex 保護部（受領フラグ・解放フラグ）。
struct ShioriGateInner {
    /// 対象呼出を受領し、往復を握った（＝kanade が round-trip でブロック中）なら true。
    /// `wait_until_blocked` はこの成立を待つ（決定的バリア）。
    blocked_arrived: bool,
    /// テストが `release()` を呼んだら true（対象呼出の応答を送ってよい合図）。
    /// flag は Mutex 下で立ててから `notify_all`（lost wakeup 対策・SakuraGate と同一規律）。
    released: bool,
}

/// ブロッキング mock shiori のゲート（6.3 専用）。
///
/// [`wait_until_blocked`](ShioriGate::wait_until_blocked) で「kanade が対象呼出の往復で
/// ブロック中」であることを有界に確認し、[`release`](ShioriGate::release) でその往復に
/// fixture 既定の応答を送らせる。sleep も wall-clock も用いない（Mutex＋Condvar のみ）。
pub struct ShioriGate {
    shared: Arc<ShioriGateShared>,
}

impl ShioriGate {
    /// 対象呼出が mock へ到達し握られる（＝kanade が round-trip でブロック済み）まで有界に待つ。
    ///
    /// 期限内に成立しなければ `false`（テストは assert で失敗にする）。成立すれば `true`。
    /// flag は受信ループが Mutex 下で立ててから notify するため、本メソッドが wait に入る前に
    /// 到達しても取りこぼさない（lost wakeup なし）。
    pub fn wait_until_blocked(&self, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        let mut inner = self.shared.inner.lock().expect("shiori gate mutex");
        while !inner.blocked_arrived {
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            let (guard, res) = self
                .shared
                .cvar
                .wait_timeout(inner, deadline - now)
                .expect("shiori gate condvar wait_timeout");
            inner = guard;
            if res.timed_out() && !inner.blocked_arrived {
                return false;
            }
        }
        true
    }

    /// 握られている対象呼出に fixture 既定の応答を送らせる（catch-up を解禁する）。
    ///
    /// flag を Mutex 下で立ててから `notify_all` するため、受信ループがまだ wait に入る前に
    /// 呼ばれても取りこぼさない。以降の（非対象）呼出は即応へ戻る。
    pub fn release(&self) {
        let mut inner = self.shared.inner.lock().expect("shiori gate mutex");
        inner.released = true;
        self.shared.cvar.notify_all();
    }
}

/// ブロッキング mock shiori アクターを起動する（[`spawn_mock_shiori`] の派生・6.3 専用）。
///
/// `block_on` が指す**最初の**呼出を受領したとき、記録した上で応答を送らず、受信ループ内で
/// [`ShioriGate::release`] まで Condvar で待つ（＝kanade の drive ループが `ReplyReceiver::recv`
/// で同期ブロックする窓を作る）。ブロック中に kanade inbox へ溜まった Tick は、解放後に順次
/// 処理される（catch-up・in-flight ≤ 1）。解放後の応答は当該呼出に対する fixture 既定
/// （[`FixtureState::respond`]）を用いる——非ブロック呼出（boot 各種・後続 OnSecondChange・
/// OnClose・Unload）は従来どおり即応する。
///
/// # デッドロック安全
/// 受信ループ内の Condvar 待ちは `release()` で解ける。テストが `release()` を呼ばずに終えても、
/// StartTalk 送信端 drop 等で kanade 側 round-trip が畳まれるだけでは本スレッドは解けないが、
/// テストは必ず `release()` を呼ぶ契約であり、全 join はテストの [`DEFAULT_TIMEOUT`] で有界。
pub fn spawn_mock_shiori_blocking(fixture: Fixture, block_on: BlockOn) -> (MockShiori, ShioriGate) {
    let calls: Arc<Mutex<Vec<RecordedCall>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_body = Arc::clone(&calls);

    let shared = Arc::new(ShioriGateShared {
        inner: Mutex::new(ShioriGateInner {
            blocked_arrived: false,
            released: false,
        }),
        cvar: std::sync::Condvar::new(),
    });
    let shared_body = Arc::clone(&shared);

    let (sender, handle) = spawn_actor::<ShioriMsg, _>("mock-shiori-blocking", move |rx| {
        let mut state = FixtureState::new(fixture);
        let mut blocked_once = false;
        while let Ok(msg) = rx.recv() {
            match msg {
                ShioriMsg::Request { call, reply } => {
                    calls_body
                        .lock()
                        .expect("mock shiori calls mutex")
                        .push(RecordedCall::from_call(&call));
                    // 対象の最初の呼出のみ握る（応答は release まで送らない）。
                    if !blocked_once && block_on.matches(&call) {
                        blocked_once = true;
                        // fixture 既定の応答を今のうちに決めておく（respond の出現カウンタは
                        // 呼出順に前進させたいので、ここで消費する＝実経路の 1 呼出＝1 応答）。
                        let outcome = state.respond(&call);
                        // 受領を通知（wait_until_blocked を起こす）→ release まで待つ。
                        {
                            let mut inner =
                                shared_body.inner.lock().expect("shiori gate mutex");
                            inner.blocked_arrived = true;
                            shared_body.cvar.notify_all();
                            while !inner.released {
                                inner = shared_body
                                    .cvar
                                    .wait(inner)
                                    .expect("shiori gate condvar wait");
                            }
                        }
                        let _ = reply.send(outcome);
                    } else {
                        let outcome = state.respond(&call);
                        let _ = reply.send(outcome);
                    }
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

    (
        MockShiori {
            sender,
            handle,
            calls,
        },
        ShioriGate { shared },
    )
}

// ============================================================================
// mock sakura sink
// ============================================================================

/// mock sakura sink のハンドル群（join ハンドル・受領 talk 指示のアクセサ）。
///
/// sink は別スレッドで [`TalkCommand`] を読み、**到着順**に記録した上で、`Start` についてのみ
/// シナリオ指示（quit 真偽）に応じた [`KanadeMsg::TalkDone`] を kanade inbox へ返す。sink
/// スレッドは `TalkCommand` の全 Sender drop（＝kanade 停止）で自然終了する。
///
/// # 観測面（design C7 Ordering / delivery・Testing Strategy）
/// - [`commands`](Self::commands): 3 形（`Start`/`ResolveChoice`/`CancelChoice`）の**到着順**の
///   記録列。`TalkCommand` が単一チャンネルを流れることによる FIFO 順序保存（DD-4 の前提）を
///   観測する面である。
/// - [`started`](Self::started): 記録列のうち `TalkCommand::Start` の射影。既存の起動系檻は
///   本アクセサを従来どおり使い続け、意味は不変である。
pub struct MockSakura {
    join: thread::JoinHandle<()>,
    commands: Arc<Mutex<Vec<TalkCommand>>>,
}

impl MockSakura {
    /// 受領した [`TalkCommand`] の記録スナップショットを**到着順**で返す。
    pub fn commands(&self) -> Vec<TalkCommand> {
        self.commands.lock().expect("mock sakura mutex").clone()
    }

    /// 受領した [`StartTalk`] の記録スナップショットを返す（記録列の `Start` 射影）。
    pub fn started(&self) -> Vec<StartTalk> {
        self.commands
            .lock()
            .expect("mock sakura mutex")
            .iter()
            .filter_map(|command| match command {
                TalkCommand::Start(start) => Some(start.clone()),
                TalkCommand::ResolveChoice { .. } | TalkCommand::CancelChoice { .. } => None,
            })
            .collect()
    }

    /// sink スレッドの終了を待つ（`TalkCommand` 送信端が全て drop された後に完了する）。
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

    /// n 番目（0 始まり）の talk に対する [`TalkEndReason`] を返す（機械的置換:
    /// quit:true → `Quit`・quit:false → `Ended`）。ハーネスのシナリオ指示は quit 真偽の
    /// ままとし、契約型への変換のみを本メソッドに閉じる。
    fn reason_for(&self, index: usize) -> TalkEndReason {
        if self.quit_for(index) {
            TalkEndReason::Quit
        } else {
            TalkEndReason::Ended
        }
    }
}

/// mock sakura sink を起動する。
///
/// `talk_rx`（kanade→sakura の [`TalkCommand`] 受信端）を別スレッドで読み、各受領を**到着順**に
/// 記録した上で、`Start` については `quit_policy` に従った [`TalkDone`] を `kanade_tx` 経由で
/// kanade inbox へ返す（遅延なし・即時）。`kanade_tx` は TalkDone 返送のためだけに保持する
/// クローンでよい。
///
/// `ResolveChoice`/`CancelChoice` は**記録のみ**行う——本 mock は再生層を持たないため解決も
/// 中断も起こせず、`quit_policy` の index（＝何本目の talk か）も前進させない。到着順の記録は
/// [`MockSakura::commands`] で観測でき、起動系檻が使う [`MockSakura::started`] の意味は不変。
pub fn spawn_mock_sakura(
    talk_rx: Receiver<TalkCommand>,
    kanade_tx: Sender<KanadeMsg>,
    quit_policy: QuitPolicy,
) -> MockSakura {
    let commands: Arc<Mutex<Vec<TalkCommand>>> = Arc::new(Mutex::new(Vec::new()));
    let commands_body = Arc::clone(&commands);

    let join = thread::Builder::new()
        .name("mock-sakura".to_string())
        .spawn(move || {
            let mut index = 0usize;
            // 全 TalkCommand Sender drop（kanade 停止）で recv が Err→ループ終了。
            while let Ok(command) = talk_rx.recv() {
                // 到着順の記録は 3 形共通（記録より先に副作用を起こさない）。
                let start_talk_id = match &command {
                    TalkCommand::Start(start) => Some(start.talk_id),
                    TalkCommand::ResolveChoice { .. } | TalkCommand::CancelChoice { .. } => None,
                };
                commands_body
                    .lock()
                    .expect("mock sakura mutex")
                    .push(command);
                let Some(talk_id) = start_talk_id else {
                    // 選択解決／解除は記録のみ（再生層を持たない mock は TalkDone を作れない）。
                    continue;
                };
                let reason = quit_policy.reason_for(index);
                index += 1;
                // TalkDone 返送。kanade 停止済みで送れなくても無害（続行）。
                let _ = kanade_tx.send(KanadeMsg::TalkDone(TalkDone { talk_id, reason }));
            }
        })
        .expect("spawn mock-sakura thread");

    MockSakura { join, commands }
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
    talk_rx: Receiver<TalkCommand>,
    kanade_tx: Sender<KanadeMsg>,
    quit_policy: QuitPolicy,
    hold_indices: Vec<usize>,
) -> (MockSakura, SakuraGate) {
    let commands: Arc<Mutex<Vec<TalkCommand>>> = Arc::new(Mutex::new(Vec::new()));
    let commands_body = Arc::clone(&commands);

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
            // 全 TalkCommand Sender drop（kanade 停止）で recv が Err→ループ終了。
            while let Ok(command) = talk_rx.recv() {
                // 到着順の記録は 3 形共通（記録より先に副作用を起こさない）。
                let start_talk_id = match &command {
                    TalkCommand::Start(start) => Some(start.talk_id),
                    TalkCommand::ResolveChoice { .. } | TalkCommand::CancelChoice { .. } => None,
                };
                commands_body
                    .lock()
                    .expect("mock sakura mutex")
                    .push(command);
                let Some(talk_id) = start_talk_id else {
                    // 選択解決／解除は記録のみ（保留対象でもない＝index を前進させない）。
                    continue;
                };
                let reason = quit_policy.reason_for(index);
                let this_index = index;
                index += 1;
                let done = TalkDone { talk_id, reason };
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

    (MockSakura { join, commands }, SakuraGate { shared })
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

/// Tick を 1 秒刻みで送り続け、kanade の終了（inbox 切断＝send Err）で戻る。
/// quit:true talk の帰結として終了が必然の台本でのみ使うこと。
/// kanade が終了しない（欠陥）場合は DEFAULT_TIMEOUT の壁時計 deadline で
/// ハングでなく panic（失敗）として検出する。
///
/// 復帰駆動の完了バリア（R7' 新構造・7.1/7.3/8.5）。`first_tick_second` から 1 秒刻みで
/// 単調増加する `now`（＝`MonotonicMs(second * 1_000)`）を持つ [`KanadeMsg::Tick`] を、
/// `sender.send` が `Err`（Receiver drop＝kanade スレッド終了＝inbox 切断）を返すまで
/// 反復送出する（反復回数の上限は持たない・上限非依存の完了バリア）。
///
/// 供給ペーシングは送出ごとの [`std::thread::yield_now`] 1 回で足る（kanade へ処理を譲る）。
/// 滞留した Tick は切断時に破棄され意味論に影響しない（設計 Implementation Notes）。
///
/// # 非空虚性（ハング→失敗変換・7.3）
/// kanade が終了しない欠陥時は send が成功し続けるが、`DEFAULT_TIMEOUT` の
/// [`std::time::Instant`] deadline をループ内で毎回判定し、超過したら `what` を含む
/// 説明的メッセージで [`panic!`] する（silent hang を作らない）。
pub fn drive_ticks_until_disconnect(
    sender: &Sender<KanadeMsg>,
    first_tick_second: u64,
    what: &str,
) {
    let deadline = std::time::Instant::now() + DEFAULT_TIMEOUT;
    let mut second = first_tick_second;
    loop {
        // 切断（Receiver drop＝kanade 終了）で戻る＝完了バリア。上限回数は持たない。
        if sender
            .send(KanadeMsg::Tick {
                now: MonotonicMs(second * 1_000),
            })
            .is_err()
        {
            return;
        }
        // 供給ペーシング: 送出ごとに短い backoff sleep で kanade ワーカースレッドへ CPU を
        // 明け渡す。`yield_now()`（Windows: `SwitchToThread`＝同一プロセッサの ready スレッドのみに
        // 譲る）は、`cargo test --workspace` の並列実行でコア数を超えるスレッド（多数の協調ループ檻
        // ＋各 kanade ワーカー）が走る飽和下では kanade ワーカーへ確実に譲れず、producer の busy-spin が
        // worker を CPU 飢餓させて `DEFAULT_TIMEOUT` を偽陽性で踏む（実ハングではなく競合飢餓＝単独/
        // 直列では緑・並列で赤・失敗数も負荷依存で変動する非決定 flake）。短い sleep はスレッドを実際に
        // deschedule するため飽和下でも worker が確実に前進でき、本ループの終了は少数の論理 tick で必然
        // ゆえ総遅延は無視できる（deadline には到達しない）。
        std::thread::sleep(Duration::from_micros(200));
        // ハング検出: deadline 超過は必ず panic（kanade 非終了の欠陥を失敗へ変換）。
        if std::time::Instant::now() >= deadline {
            panic!(
                "'{what}' did not disconnect within {DEFAULT_TIMEOUT:?} \
                 (kanade failed to terminate; possible hang)"
            );
        }
        second += 1;
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

// ============================================================================
// 疎通テスト（観測可能な完了条件）
// ============================================================================

#[cfg(test)]
mod smoke {
    use super::*;
    use areka_actor::reply_channel;
    use areka_kanade::{ExecutionSnapshot, MouseButton, TalkId, events};

    /// mock shiori 単独駆動: `OnBoot` GET へ即時に固定 Value を返し、記録が
    /// events 表から導出した期待値と一致する（fixture・assert・実装の三点一正本）。
    #[test]
    fn mock_shiori_replies_and_records_onboot() {
        let config = KanadeConfig::new("master", "1.0.0");
        let shiori = spawn_mock_shiori(Fixture::default());

        // events 表から GET OnBoot の呼出を導出して送る（ハードコードしない）。boot 系列は
        // talk 非アクティブ（INACTIVE スナップショット・DD-IT-4）ゆえ Status ヘッダは出ない。
        let call = events::on_boot(&config, &ExecutionSnapshot::INACTIVE);
        let expected = expected_call(events::on_boot(&config, &ExecutionSnapshot::INACTIVE));

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

    /// mock sakura sink 単独駆動: [`TalkCommand::Start`] を受領・記録し、シナリオ指示どおり
    /// [`KanadeMsg::TalkDone`] を返す。
    #[test]
    fn mock_sakura_records_and_returns_talkdone() {
        use areka_kanade::talk::TalkCommand;

        let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();
        // sink → TalkDone を受け取るための疑似 kanade inbox。
        let (kanade_tx, kanade_rx) = std::sync::mpsc::channel::<KanadeMsg>();

        let sakura = spawn_mock_sakura(talk_rx, kanade_tx, QuitPolicy::Fixed(true));

        talk_tx
            .send(TalkCommand::Start(StartTalk {
                epilogue: Vec::new(),
                talk_id: TalkId(1),
                script: FIXED_BOOT_SCRIPT.to_string(),
            }))
            .expect("send TalkCommand::Start to mock sakura");

        // TalkDone が即時に返る（期限付き・ハングしない）。
        let msg = kanade_rx
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("mock sakura should emit TalkDone immediately");
        match msg {
            KanadeMsg::TalkDone(done) => {
                assert_eq!(done.talk_id, TalkId(1));
                assert_eq!(
                    done.reason,
                    TalkEndReason::Quit,
                    "QuitPolicy::Fixed(true) should set reason=Quit"
                );
            }
            _ => panic!("expected TalkDone from mock sakura"),
        }

        // 記録に受領 talk が残る。
        let started = sakura.started();
        assert_eq!(started.len(), 1);
        assert_eq!(started[0].talk_id, TalkId(1));
        assert_eq!(started[0].script, FIXED_BOOT_SCRIPT);

        // TalkCommand 送信端を drop → sink スレッドは自然終了（期限付き join）。
        drop(talk_tx);
        sakura.join_bounded("mock-sakura join", DEFAULT_TIMEOUT);
    }

    /// task 1.4（design Testing Strategy 冒頭・C7 Ordering / delivery）: mock sakura sink が
    /// [`TalkCommand`] 3 形の**到着順**を記録する。
    ///
    /// `TalkCommand` は単一チャンネルを流れることで FIFO 順序保存が契約（DD-4 の前提）であり、
    /// その観測面が本記録列である。ここでは Start→Resolve→Cancel→Start を投函し、記録列が
    /// **投函順そのまま**であること（＝解決系と起動系が別扱いされず 1 本の順序に載ること）を固定する。
    /// `started()` は `TalkCommand::Start` の射影であり、既存檻の意味は不変であることも併せて確認する。
    #[test]
    fn mock_sakura_records_talk_command_arrival_order() {
        use areka_kanade::talk::TalkCommand;

        let (talk_tx, talk_rx) = std::sync::mpsc::channel::<TalkCommand>();
        let (kanade_tx, kanade_rx) = std::sync::mpsc::channel::<KanadeMsg>();

        let sakura = spawn_mock_sakura(talk_rx, kanade_tx, QuitPolicy::Fixed(false));

        talk_tx
            .send(TalkCommand::Start(StartTalk::new(TalkId(1), "first")))
            .expect("send Start(1)");
        // Start の TalkDone を受けてから次を送る（記録スレッドの前進を確定させる barrier）。
        match kanade_rx
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("Start(1) の TalkDone が返るはず")
        {
            KanadeMsg::TalkDone(done) => assert_eq!(done.talk_id, TalkId(1)),
            _ => panic!("expected TalkDone from mock sakura"),
        }

        talk_tx
            .send(TalkCommand::ResolveChoice {
                talk_id: TalkId(1),
                id: "pick".to_string(),
            })
            .expect("send ResolveChoice");
        talk_tx
            .send(TalkCommand::CancelChoice {
                talk_id: TalkId(1),
            })
            .expect("send CancelChoice");
        talk_tx
            .send(TalkCommand::Start(StartTalk::new(TalkId(2), "second")))
            .expect("send Start(2)");
        match kanade_rx
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("Start(2) の TalkDone が返るはず")
        {
            KanadeMsg::TalkDone(done) => assert_eq!(done.talk_id, TalkId(2)),
            _ => panic!("expected TalkDone from mock sakura"),
        }

        // 到着順の記録（投函順そのまま・解決系と起動系が同一の順序列に載る）。
        let commands = sakura.commands();
        assert_eq!(commands.len(), 4, "4 件すべてが記録されるべき: {commands:?}");
        match &commands[0] {
            TalkCommand::Start(s) => {
                assert_eq!(s.talk_id, TalkId(1));
                assert_eq!(s.script, "first");
            }
            other => panic!("commands[0] は Start(1) のはず: {other:?}"),
        }
        match &commands[1] {
            TalkCommand::ResolveChoice { talk_id, id } => {
                assert_eq!(*talk_id, TalkId(1));
                assert_eq!(id, "pick");
            }
            other => panic!("commands[1] は ResolveChoice のはず: {other:?}"),
        }
        match &commands[2] {
            TalkCommand::CancelChoice { talk_id } => assert_eq!(*talk_id, TalkId(1)),
            other => panic!("commands[2] は CancelChoice のはず: {other:?}"),
        }
        match &commands[3] {
            TalkCommand::Start(s) => {
                assert_eq!(s.talk_id, TalkId(2));
                assert_eq!(s.script, "second");
            }
            other => panic!("commands[3] は Start(2) のはず: {other:?}"),
        }

        // `started()` は Start の射影（既存檻の意味不変・Resolve/Cancel は現れない）。
        let started = sakura.started();
        assert_eq!(started.len(), 2);
        assert_eq!(started[0].talk_id, TalkId(1));
        assert_eq!(started[1].talk_id, TalkId(2));

        drop(talk_tx);
        sakura.join_bounded("mock-sakura join", DEFAULT_TIMEOUT);
    }

    /// 4.1 配管の疎通: マウス GET へ script Value と 204 を任意注入でき、mock がそれを返す。
    ///
    /// `Fixture::with_mouse_response` で `OnMouseMove` に script を注入・`OnMouseDoubleClick` は
    /// 未注入（既定 204）とし、events 表から導出した両 GET を送って注入どおりの [`ShioriOutcome`]
    /// が返ることを確認する（4.2／4.3 の (c)/(d) 檻の土台＝注入点が実在することの証明）。
    #[test]
    fn mock_shiori_injects_mouse_response() {
        const MOUSE_SCRIPT: &str = r"\0\s[0]なでなで\e";
        // OnMouseMove へ script を注入・OnMouseDoubleClick は未注入（既定 204）。
        let fixture = Fixture::default()
            .with_mouse_response("OnMouseMove", MouseResponse::Script(MOUSE_SCRIPT.to_string()));
        let shiori = spawn_mock_shiori(fixture);

        // (c) script 注入: OnMouseMove GET → Value（events 表から導出・ハードコードしない）。
        let move_call = events::on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot::INACTIVE);
        let (reply, receiver) = reply_channel::<ShioriOutcome>();
        shiori
            .sender
            .send(ShioriMsg::Request {
                call: move_call,
                reply,
            })
            .expect("send OnMouseMove to mock shiori");
        let outcome = receiver
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("mock shiori should reply immediately");
        match outcome {
            ShioriOutcome::Value(script) => assert_eq!(script, MOUSE_SCRIPT),
            _ => panic!("expected injected Value(script) for OnMouseMove"),
        }

        // (d) 未注入: OnMouseDoubleClick GET → 204（NoContent・StartTalk 不発の源）。
        let dbl_call = events::on_mouse_double_click(
            10,
            20,
            0,
            Some("Head"),
            MouseButton::Left,
            &ExecutionSnapshot::INACTIVE,
        );
        let (reply, receiver) = reply_channel::<ShioriOutcome>();
        shiori
            .sender
            .send(ShioriMsg::Request {
                call: dbl_call,
                reply,
            })
            .expect("send OnMouseDoubleClick to mock shiori");
        let outcome = receiver
            .recv_timeout(DEFAULT_TIMEOUT)
            .expect("mock shiori should reply immediately");
        match outcome {
            ShioriOutcome::NoContent => {}
            _ => panic!("expected 204 (NoContent) for un-injected OnMouseDoubleClick"),
        }

        // 両 GET が events 導出の期待値どおり記録される（Ref layout 込みの突合は 4.2／4.3 が担う）。
        let recorded = shiori.recorded();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0].id, "OnMouseMove");
        assert_eq!(recorded[1].id, "OnMouseDoubleClick");

        shiori
            .sender
            .send(ShioriMsg::Close)
            .expect("send close to mock shiori");
        join_bounded("mock-shiori join", DEFAULT_TIMEOUT, shiori.handle)
            .expect("mock shiori body completes normally");
    }
}
