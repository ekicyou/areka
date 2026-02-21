//! コンパイル済みデータ構造体群
//!
//! ランタイムが直接消費する CompiledStoryboard / CompiledVariableTimeline /
//! CompiledSegment / VariableTypeHint / CompiledTrigger を定義する。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::easing::EasingFunction;
use crate::storyboard::{InterruptionPolicy, LoopOffset};
use crate::transition::TransitionValue;

/// コンパイル済みストーリーボードのルート構造体
///
/// 変数名をキーとしたタイムラインマップと、ランタイム用メタ情報を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledStoryboard {
    /// 元のストーリーボード名
    pub storyboard_name: String,
    /// コンパイル起点の開始時刻（f64秒）
    pub start_time: f64,
    /// 変数名 → コンパイル済みタイムライン
    pub timelines: BTreeMap<String, CompiledVariableTimeline>,
    /// 再生速度倍率（ランタイム適用、事前適用なし）
    pub time_scale: f64,
    /// ループ回数（1 = 1回再生、n≥2 = n回再生、-1 = 無限ループ）
    pub loop_count: i32,
    /// 割り込み終了戦略
    pub interruption_policy: InterruptionPolicy,
    /// ループオフセット定義（省略可能）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_offset: Option<LoopOffset>,
    /// ベース合計再生時間 time_scale未適用 全タイムラインの最大値
    pub total_base_duration: f64,
    /// トリガーリスト（fire_time 順ソート済み）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triggers: Vec<CompiledTrigger>,
}

/// 変数ごとのセグメント列とランタイムヒント
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledVariableTimeline {
    /// 変数型情報
    pub variable_type: VariableTypeHint,
    /// セグメント配列（時刻順ソート済み、重複なし）
    pub segments: Vec<CompiledSegment>,
    /// このタイムラインのベース再生時間（最終セグメント end_time - start_time）
    pub base_duration: f64,
    /// 値域下限 f64/i64のみ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    /// 値域上限 f64/i64のみ
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
}

/// 単一トランジションセグメントの全情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledSegment {
    /// セグメント開始時刻（絶対時刻、f64秒）
    pub start_time: f64,
    /// セグメント終了時刻（絶対時刻、f64秒）
    /// 即時遷移の場合は start_time と等しい
    pub end_time: f64,
    /// 開始値
    pub from_value: TransitionValue,
    /// 終了値
    pub to_value: TransitionValue,
    /// イージング関数
    /// None = 線形補間 または Object型即時切り替え
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub easing: Option<EasingFunction>,
}

/// ランタイムに変数型固有の処理方法を伝達する enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VariableTypeHint {
    /// f64連続値（補間対応）
    Float,
    /// i64離散値（補間後の丸め処理が必要）
    Integer {
        /// タイプライター文字列
        #[serde(default, skip_serializing_if = "Option::is_none")]
        typewriter: Option<String>,
    },
    /// Object型（即時切り替えのみ）
    Object,
}

/// コンパイル済みトリガー情報
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledTrigger {
    /// トリガー発火時刻（絶対時刻、f64秒）
    pub fire_time: f64,
    /// 対象ストーリーボード名
    pub target_storyboard: String,
    /// 開始オフセット（fire_time + start_offset = 子SBの start_time）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_offset: Option<f64>,
    /// 元エントリのインデックス（デバッグ用）
    pub source_entry_index: usize,
}
