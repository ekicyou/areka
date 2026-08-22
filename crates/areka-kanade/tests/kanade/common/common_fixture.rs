//! mock shiori の応答表（fixture）と、その適用状態。
//!
//! `common/mod.rs`（1,657 行）から責務単位で切り出した子モジュール（タスク 8.2）。
//! 項目は親のファサードから再輸出されるため、消費側の `super::common::X` は不変である。

use std::collections::HashMap;

use areka_kanade::{ShioriCall, ShioriOutcome};

/// 固定 boot スクリプト（`OnBoot` GET 200 の fixture 応答）。
pub const FIXED_BOOT_SCRIPT: &str = r"\0\s[0]おはよう\e";

/// 定常運転で散発的に返す fixture スクリプト（`OnSecondChange` GET 200）。
pub const FIXED_STEADY_SCRIPT: &str = r"\0\s[0]ふぅ\e";

/// close talk の fixture スクリプト（`OnClose` GET 200・quit シナリオ）。
pub const FIXED_FAREWELL_SCRIPT: &str = r"\0\s[0]またね\e-";

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

/// 選択由来 GET（任意名イベント／`OnChoiceSelectEx`／`OnChoiceSelect`／`OnChoiceTimeout`）へ
/// 注入する応答パターン（6.1）。
///
/// [`MouseResponse`] と同型の 2 値語彙だが、注入先が別表（[`Fixture::choice_responses`]）である
/// ことを型で区別する。カスケードの各段はイベント id で識別されるため、段ごとに script／204 を
/// 打ち分けられる（正典形の「Ex が 204 → 無印へ前進」「Ex が Value → 無印を発行しない」の
/// 両分岐を同一器で作れる）。
#[derive(Debug, Clone)]
pub enum ChoiceResponse {
    /// talk スクリプト Value を返す（カスケードを短絡させ StartTalk を起こす）。
    Script(String),
    /// 204 / NoContent を返す（無応答＝次段へ前進、または最終段なら解決のみ）。
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
    /// 選択由来 GET のイベント id → 注入応答の対応（6.1）。
    ///
    /// キーは wire 形のイベント id（`"OnChoiceSelectEx"`／`"OnChoiceSelect"`／`"OnChoiceTimeout"`
    /// および `\q` の `On` 始まり任意名 ID そのもの）。含まれない id は 204（`NoContent`）——
    /// 未注入既定は従来の catch-all（未知 GET＝204）と同値ゆえ additive である。
    pub choice_responses: HashMap<&'static str, ChoiceResponse>,
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
            choice_responses: HashMap::new(),
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

    /// 選択由来 GET のイベント id へ注入応答（script Value ／ 204）を設定する（6.1・連鎖記法）。
    ///
    /// `id` は wire 形のイベント id（任意名 ID・`"OnChoiceSelectEx"`・`"OnChoiceSelect"`・
    /// `"OnChoiceTimeout"`）。同一 id への再指定は後勝ちで上書きする。未設定の id は 204
    /// （`NoContent`）のまま——カスケードの段ごとに応答を打ち分ける唯一の口である。
    pub fn with_choice_response(mut self, id: &'static str, response: ChoiceResponse) -> Self {
        self.choice_responses.insert(id, response);
        self
    }
}

/// fixture 適用の可変状態（`OnSecondChange` GET の出現回数を数える）。
pub(super) struct FixtureState {
    fixture: Fixture,
    second_change_get_seen: usize,
}

impl FixtureState {
    pub(super) fn new(fixture: Fixture) -> Self {
        FixtureState {
            fixture,
            second_change_get_seen: 0,
        }
    }

    /// 1 件の [`ShioriCall`] に対する応答を fixture 表から決定する（即時応答値）。
    pub(super) fn respond(&mut self, call: &ShioriCall) -> ShioriOutcome {
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
                        Some(MouseResponse::Script(script)) => ShioriOutcome::Value(script.clone()),
                        Some(MouseResponse::NoContent) | None => ShioriOutcome::NoContent,
                    }
                }
                // 選択由来 GET は fixture の注入表を引く（未注入は 204・6.1）。任意名イベントは
                // ゴースト作者が書いた名前がそのまま id になるため、固定パターンでは受けられず
                // catch-all の手前で表引きする（表に無い id は従来どおり 204 へ落ちる）。
                other => match self.fixture.choice_responses.get(other) {
                    Some(ChoiceResponse::Script(script)) => ShioriOutcome::Value(script.clone()),
                    // 未注入の選択由来 GET・未知 GET はいずれも 204（保守的既定）。
                    Some(ChoiceResponse::NoContent) | None => ShioriOutcome::NoContent,
                },
            },
        }
    }
}
