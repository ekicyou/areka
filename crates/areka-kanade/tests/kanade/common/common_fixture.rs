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

/// 表に載せた GET（マウス・選択由来・リソース）へ返す応答。
///
/// `Script(String::new())` は **200 の空文字**であり 204 とは別物である（リソースの要件 3.3／3.4 の
/// 「空を返す」と「値なしを返す」の区別はここで作る）。
#[derive(Debug, Clone)]
pub enum Response {
    /// 200 の Value を返す（talk の台本なら StartTalk を起こす）。
    Script(String),
    /// 204 / NoContent を返す。
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
    /// `OnBoot` が起動挨拶 Value を返すか（DD-IT-12）。`true`＝固定スクリプトの Value（挨拶 talk を
    /// 起こし `Steady{talk: Some(_)}` へ完了）。`false`＝204（挨拶なし・`Steady{talk: None}` へ直行）。
    ///
    /// DD-IT-12 で boot は挨拶 talk を正規追跡するようになった。挨拶 talk の TalkDone は mock sakura
    /// が別スレッドから返すため、その到着は後続 Tick と inbox 上で競合する（GET/NOTIFY・Ref3 が
    /// 非決定になる）。定常 pump（`Steady{None}` の GET・Req 2.1/2.3/3.3）を**決定的に**観測する
    /// テストは、この挨拶を出さない（`false`）ことで boot→`Steady{None}` へ直行させ競合を発生源から
    /// 断つ（設計 Testing Strategy「boot が 204 を返す fixture」）。挨拶 boot 自体の観測は
    /// full_run_test が担う。
    pub boot_greets: bool,
    /// `OnSecondChange`（GET）で Value を返す出現インデックス集合（0 始まり）。
    pub steady_value_indices: Vec<usize>,
    /// `OnClose` を Value（quit talk）で返すなら true・204（無言終了）なら false。
    pub close_quits: bool,
    /// 表に載らない GET（マウス・選択由来・リソース）の id → 応答の対応。
    ///
    /// キーは wire 形のイベント id（`"OnMouseMove"`・`"OnChoiceSelectEx"`・`\q` の任意名 ID）または
    /// SHIORI リソース名（`"readmebutton.caption"` 等）。含まれない id は 204（未知 GET と同じ）。
    pub responses: HashMap<&'static str, Response>,
}

impl Default for Fixture {
    /// 既定シナリオ: 起動挨拶あり・散発 Value なし・無言 close（最小の疎通に足る保守的既定）。
    fn default() -> Self {
        Fixture {
            boot_greets: true,
            steady_value_indices: Vec::new(),
            close_quits: false,
            responses: HashMap::new(),
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

    /// GET の id（マウス・選択由来・リソース）へ応答を設定する（連鎖記法）。
    ///
    /// 同一 id への再指定は後勝ちで上書きする。未設定の id は 204 のまま。
    pub fn with_response(mut self, id: &'static str, response: Response) -> Self {
        self.responses.insert(id, response);
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
                        ShioriOutcome::Value(
                            FIXED_BOOT_SCRIPT.to_string(),
                            areka_talk::SecurityLevel::Local,
                        )
                    } else {
                        ShioriOutcome::NoContent
                    }
                }
                "OnSecondChange" => {
                    let index = self.second_change_get_seen;
                    self.second_change_get_seen += 1;
                    if self.fixture.steady_value_indices.contains(&index) {
                        ShioriOutcome::Value(
                            FIXED_STEADY_SCRIPT.to_string(),
                            areka_talk::SecurityLevel::Local,
                        )
                    } else {
                        ShioriOutcome::NoContent
                    }
                }
                "OnClose" => {
                    if self.fixture.close_quits {
                        ShioriOutcome::Value(
                            FIXED_FAREWELL_SCRIPT.to_string(),
                            areka_talk::SecurityLevel::Local,
                        )
                    } else {
                        ShioriOutcome::NoContent
                    }
                }
                // 表に載せた GET（マウス・選択由来・リソース）。どれも語彙が交わらないので表は 1 本。
                other => match self.fixture.responses.get(other) {
                    Some(Response::Script(value)) => {
                        ShioriOutcome::Value(value.clone(), areka_talk::SecurityLevel::Local)
                    }
                    // 未設定の id・未知 GET はいずれも 204（保守的既定）。
                    Some(Response::NoContent) | None => ShioriOutcome::NoContent,
                },
            },
        }
    }
}
