//! コンパイル済みデータ構造体群
//!
//! ランタイムが直接消費する CompiledStoryboard / CompiledVariableTimeline /
//! CompiledSegment / VariableTypeHint / CompiledTrigger を定義する。

use std::collections::BTreeMap;

use crate::easing::EasingFunction;
use crate::storyboard::{InterruptionPolicy, LoopOffset};
use crate::time::{TimeDelta, TimeScale, TimeSpan};
use crate::transition::TransitionValue;

/// コンパイル済みストーリーボードのルート構造体
///
/// 変数名をキーとしたタイムラインマップと、ランタイム用メタ情報を保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledStoryboard {
    /// 元のストーリーボード名
    pub storyboard_name: String,
    /// 変数名 → コンパイル済みタイムライン
    pub timelines: BTreeMap<String, CompiledVariableTimeline>,
    /// 再生速度倍率（ランタイム適用、事前適用なし）
    pub time_scale: TimeScale,
    /// ループ回数（1 = 1回再生、n≥2 = n回再生、-1 = 無限ループ）
    pub loop_count: i32,
    /// 割り込み終了戦略
    pub interruption_policy: InterruptionPolicy,
    /// ループオフセット定義（省略可能）
    pub loop_offset: Option<LoopOffset>,
    /// ベース合計再生時間 time_scale未適用 全タイムラインの最大値
    pub total_base_duration: TimeSpan,
    /// トリガーリスト（fire_time 順ソート済み）
    pub triggers: Vec<CompiledTrigger>,
}

impl CompiledStoryboard {
    /// 1 周の経った長さ（倍率の写しで全体の長さに届く最初の長さ）。表せなければ `None`。
    pub fn lap_length(&self) -> Option<TimeSpan> {
        self.time_scale
            .first_reaching(self.total_base_duration.into())
            .and_then(TimeDelta::to_span)
    }
}

/// 変数ごとのセグメント列とランタイムヒント
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledVariableTimeline {
    /// 変数型情報
    pub variable_type: VariableTypeHint,
    /// セグメント配列（時刻順ソート済み、重複なし）
    pub segments: Vec<CompiledSegment>,
    /// このタイムラインのベース再生時間（最終セグメントの end_time。storyboard の頭からの隔たりで、先頭の遅れを含む）
    pub base_duration: TimeDelta,
    /// 値域下限 f64/i64のみ
    pub min_value: Option<f64>,
    /// 値域上限 f64/i64のみ
    pub max_value: Option<f64>,
}

/// 単一トランジションセグメントの全情報
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledSegment {
    /// セグメント開始時刻（storyboard の頭からの隔たり。頭より前にも置ける）
    pub start_time: TimeDelta,
    /// セグメント終了時刻（storyboard の頭からの隔たり）
    /// 即時遷移の場合は start_time と等しい
    pub end_time: TimeDelta,
    /// 開始値
    pub from_value: TransitionValue,
    /// 終了値
    pub to_value: TransitionValue,
    /// イージング関数
    /// None = 線形補間 または Object型即時切り替え
    pub easing: Option<EasingFunction>,
}

/// ランタイムに変数型固有の処理方法を伝達する enum
#[derive(Debug, Clone, PartialEq)]
pub enum VariableTypeHint {
    /// f64連続値（補間対応）
    Float,
    /// i64離散値（補間後の丸め処理が必要）
    Integer,
    /// Object型（即時切り替えのみ）
    Object,
}

/// コンパイル済みトリガー情報
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledTrigger {
    /// トリガー発火時刻（storyboard の頭からの隔たり）
    pub fire_time: TimeDelta,
    /// 対象ストーリーボード名
    pub target_storyboard: String,
    /// 開始オフセット（発火の時刻 + start_offset = 子SBを始める時刻）
    pub start_offset: Option<TimeDelta>,
    /// 元エントリのインデックス（デバッグ用）
    pub source_entry_index: usize,
}
