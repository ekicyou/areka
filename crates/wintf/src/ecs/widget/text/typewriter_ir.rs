//! Typewriter IR (Intermediate Representation) 型定義
//!
//! 2段階IR設計:
//! - Stage 1 IR (TypewriterToken): 外部インターフェース（Script Engine から受け取る形式）
//! - Stage 2 IR (TimelineItem): 内部タイムライン（グリフ単位に分解済み）

use bevy_ecs::prelude::*;
use dola::TimeSpan;

// ============================================================
// Stage 1 IR - 外部インターフェース
// ============================================================

/// Stage 1 IR - 外部インターフェース
/// Script Engine から受け取る形式
#[derive(Debug, Clone)]
pub enum TypewriterToken {
    /// 表示するテキスト
    Text(String),
    /// ウェイト（作者が書く秒。時間表を組む時に長さへ変える）
    Wait(f64),
    /// イベント発火（対象エンティティの TypewriterEvent を設定）
    FireEvent {
        target: Entity,
        event: TypewriterEventKind,
    },
}

/// イベント通知用 enum Component
/// Changed<TypewriterEvent> で検出、処理後に None へ戻す（set パターン）
/// メモリ戦略: SparseSet（動的変更）
#[derive(Component, Debug, Clone, Default, PartialEq)]
#[component(storage = "SparseSet")]
pub enum TypewriterEvent {
    #[default]
    None,
    /// 表示完了
    Complete,
    /// 一時停止
    Paused,
    /// 再開
    Resumed,
}

/// FireEvent で使用するイベント種別
/// TypewriterEvent との違い: Component ではない純粋なデータ型
#[derive(Debug, Clone, PartialEq)]
pub enum TypewriterEventKind {
    /// 表示完了
    Complete,
    /// 一時停止
    Paused,
    /// 再開
    Resumed,
}

impl From<TypewriterEventKind> for TypewriterEvent {
    fn from(kind: TypewriterEventKind) -> Self {
        match kind {
            TypewriterEventKind::Complete => TypewriterEvent::Complete,
            TypewriterEventKind::Paused => TypewriterEvent::Paused,
            TypewriterEventKind::Resumed => TypewriterEvent::Resumed,
        }
    }
}

// ============================================================
// Stage 2 IR - 内部タイムライン
// ============================================================

/// Stage 2 IR - 内部タイムライン
/// DirectWriteでグリフ単位に分解後の形式
#[derive(Debug, Clone)]
pub enum TimelineItem {
    /// グリフ表示（TextLayout内のクラスタ番号）
    Glyph {
        /// クラスタインデックス
        cluster_index: u32,
        /// 開始から現れるまでの長さ
        show_at: TimeSpan,
    },
    /// ウェイト
    Wait {
        /// ウェイトの長さ
        duration: TimeSpan,
        /// 開始からウェイトの始まりまでの長さ
        start_at: TimeSpan,
    },
    /// イベント発火
    FireEvent {
        /// 対象エンティティ
        target: Entity,
        /// 発火するイベント
        event: TypewriterEventKind,
        /// 開始から発火までの長さ
        fire_at: TimeSpan,
    },
}

/// Typewriter タイムライン全体
#[derive(Debug, Clone)]
pub struct TypewriterTimeline {
    /// 全文テキスト
    pub full_text: String,
    /// タイムライン項目
    pub items: Vec<TimelineItem>,
    /// 総クラスタ数
    pub total_cluster_count: u32,
}

impl TypewriterTimeline {
    /// 行程の終わり（最後に済む項目の、開始からの長さ）。項目が無ければ 0。
    pub fn end(&self) -> TimeSpan {
        self.items
            .iter()
            .map(|item| match item {
                TimelineItem::Glyph { show_at, .. } => *show_at,
                TimelineItem::Wait { start_at, duration } => *start_at + *duration,
                TimelineItem::FireEvent { fire_at, .. } => *fire_at,
            })
            .max()
            .unwrap_or(TimeSpan::ZERO)
    }
}
