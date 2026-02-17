// TODO: Implement Storyboard, StoryboardEntry, KeyframeRef, KeyframeNames, BetweenKeyframes, InterruptionPolicy
use serde::{Deserialize, Serialize};

use crate::easing::{EasingFunction, EasingName};
use crate::transition::TransitionRef;

/// デフォルトのイージング関数（Linear）を返すヘルパー
fn default_easing_linear() -> EasingFunction {
    EasingFunction::Named(EasingName::Linear)
}

/// ループオフセット定義（短縮形 / オブジェクト形式）
///
/// ストーリーボードの各ループ周回間にランダムな待機時間を挿入するためのパラメータ。
/// - 短縮形: 数値 → `max` として解釈（`min=0.0`, `easing=linear`）
/// - オブジェクト形式: `{ min, max, easing }` で詳細指定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LoopOffset {
    /// 短縮形: 数値 → max として解釈（min=0.0, easing=linear）
    Scalar(f64),
    /// オブジェクト形式: { min, max, easing }
    Range(LoopOffsetRange),
}

/// ループオフセット範囲定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopOffsetRange {
    /// 最小待機時間（f64秒、デフォルト 0.0）
    #[serde(default)]
    pub min: f64,
    /// 最大待機時間（f64秒、必須）
    pub max: f64,
    /// イージング関数（デフォルト: linear）
    #[serde(default = "default_easing_linear")]
    pub easing: EasingFunction,
}

/// 割り込み終了戦略（ストーリーボード競合時の自己申告方針）
///
/// マルチプロセス協調アニメーション環境において、各ストーリーボードは
/// 「自分が中断されたらどう振る舞うか」を宣言的に自己申告する。
/// オーケストレーション側の解決ロジックはこの情報を参照して適切な終了処理を実行する。
/// priority（競争的優先度）は採用せず、協調的な自己申告のみとする。
/// (research.md Decision 10 参照)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionPolicy {
    /// 即座に破棄。変数値はその瞬間で凍結（WAM: Abandon 相当）
    Cancel,
    /// 現在のトランジションを最終値へジャンプさせて完了（デフォルト）
    Conclude,
    /// 割り込み開始時点まで再生して切断
    Trim,
    /// 残りを圧縮（高速再生）して完了
    Compress,
    /// 中断不可。このストーリーボードが未完了なら新ストーリーボードの開始を待機
    Never,
}

fn default_time_scale() -> f64 {
    1.0
}

fn default_loop_count() -> i32 {
    1
}

fn default_interruption_policy() -> InterruptionPolicy {
    InterruptionPolicy::Conclude
}

/// ストーリーボード（メタ情報 + エントリ配列）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Storyboard {
    /// 再生速度倍率（デフォルト 1.0）
    #[serde(default = "default_time_scale")]
    pub time_scale: f64,
    /// ループ回数（1 = 1回再生、n≥2 = n回再生、-1 = 無限ループ、0以下 = エラー）
    #[serde(default = "default_loop_count")]
    pub loop_count: i32,
    /// 割り込み終了戦略（デフォルト: Conclude）
    #[serde(default = "default_interruption_policy")]
    pub interruption_policy: InterruptionPolicy,
    /// ループオフセット定義（省略可能 — 省略時は遅延なし）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_offset: Option<LoopOffset>,
    /// エントリ配列
    #[serde(default)]
    pub entry: Vec<StoryboardEntry>,
}

/// ストーリーボードエントリ（配置 + KF 定義の統合単位）
///
/// 5配置パターン:
/// - 前エントリ連結: variable + transition（at/between なし）
/// - KF起点: variable + transition + at
/// - KF間: variable + transition + between
/// - 純粋KF: keyframe のみ
/// - トリガー: trigger_storyboard（他SB起動）
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct StoryboardEntry {
    /// 対象変数名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variable: Option<String>,
    /// トランジション参照（名前 or インライン）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<TransitionRef>,
    /// 開始キーフレーム指定（文字列/配列/オフセット付きオブジェクト）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<KeyframeRef>,
    /// キーフレーム間配置
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub between: Option<BetweenKeyframes>,
    /// このエントリ終了時点のキーフレーム名（省略時は暗黙的KFが生成される: Req 3.6）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyframe: Option<String>,
    /// トリガー対象ストーリーボード名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_storyboard: Option<String>,
    /// トリガー開始オフセット（トリガー発火時刻 + offset = 子SBの start_time）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger_start_offset: Option<f64>,
}

/// キーフレーム起点指定（`at` フィールド用）
/// 4つの表現形式をサポート:
///   at = "visible"                                           → Single
///   at = ["visible", "audio_done"]                           → Multiple
///   at = { keyframes = "visible", offset = 0.5 }            → WithOffset (single)
///   at = { keyframes = ["visible", "done"], offset = 0.5 }  → WithOffset (multiple)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyframeRef {
    /// 単一キーフレーム名（文字列短縮形）
    Single(String),
    /// 複数キーフレーム名（配列形式、全KF完了待機）
    Multiple(Vec<String>),
    /// オフセット付き指定（オブジェクト形式）
    WithOffset {
        /// キーフレーム名指定（文字列または配列）
        keyframes: KeyframeNames,
        /// キーフレーム時刻からの時間オフセット（f64秒、デフォルト 0.0）
        #[serde(default)]
        offset: f64,
    },
}

/// キーフレーム名指定（単一または複数）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KeyframeNames {
    /// 単一キーフレーム名
    Single(String),
    /// 複数キーフレーム名（全KF完了待機）
    Multiple(Vec<String>),
}

/// キーフレーム間配置指定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BetweenKeyframes {
    /// 開始キーフレーム名
    pub from: String,
    /// 終了キーフレーム名
    pub to: String,
}
