//! Typewriter コンポーネント定義
//!
//! - Typewriter: ウィジェット論理コンポーネント（永続、スタイル設定）
//! - TypewriterTalk: 1回のトーク論理情報（再生中のみ存在）
//! - TypewriterLayoutCache: 描画リソース（システムが自動生成）

use crate::ecs::Visual;
use crate::ecs::widget::text::typewriter_ir::{
    TimelineItem, TypewriterEventKind, TypewriterTimeline, TypewriterToken,
};
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::DeferredWorld;
use tracing::trace;
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;

// re-export TextDirection from label
pub use crate::ecs::widget::text::label::TextDirection;

// ============================================================
// Typewriter - ウィジェット論理コンポーネント（永続）
// ============================================================

/// ウィジェット論理コンポーネント（永続）
/// メモリ戦略: SparseSet（動的追加/削除）
///
/// 色は`Brushes`コンポーネントで指定します。
/// ```ignore
/// world.spawn((
///     Typewriter {
///         font_family: "メイリオ".to_string(),
///         font_size: 18.0,
///         direction: TextDirection::HorizontalLeftToRight,
///         default_char_wait: 0.15,
///     },
///     Brushes::with_colors(fg_color, bg_color),
/// ));
/// ```
#[derive(Component)]
#[component(storage = "SparseSet", on_add = on_typewriter_add, on_remove = on_typewriter_remove)]
pub struct Typewriter {
    // === スタイル設定（Label互換） ===
    pub font_family: String,
    pub font_size: f32,
    pub direction: TextDirection,

    // === デフォルト設定 ===
    /// デフォルト文字間ウェイト（秒）
    pub default_char_wait: f64,
}

impl Default for Typewriter {
    fn default() -> Self {
        Self {
            font_family: "メイリオ".to_string(),
            font_size: 16.0,
            direction: TextDirection::default(),
            default_char_wait: 0.05, // 50ms
        }
    }
}

/// Typewriter追加時のフック: VisualコンポーネントとTyperwriterTalk（空）を自動挿入
fn on_typewriter_add(mut world: DeferredWorld, hook: HookContext) {
    let entity = hook.entity;
    let needs_visual = world.get::<Visual>(entity).is_none();
    let needs_talk = world.get::<TypewriterTalk>(entity).is_none();

    if needs_visual || needs_talk {
        let mut cmds = world.commands();
        let mut entity_cmds = cmds.entity(entity);

        if needs_visual {
            entity_cmds.insert(Visual::default());
        }
        if needs_talk {
            // 空のトークを登録（背景描画のため）
            entity_cmds.insert(TypewriterTalk::new(vec![], 0.0));
        }
    }
}

/// Typewriter削除時のフック
fn on_typewriter_remove(_world: DeferredWorld, hook: HookContext) {
    trace!(entity = ?hook.entity, "[Typewriter] Removed");
}

// ============================================================
// TypewriterTalk - 1回のトーク論理情報（再生中のみ存在）
// ============================================================

/// 再生状態
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TypewriterState {
    #[default]
    Playing,
    Paused,
    Completed,
}

/// 1回のトーク論理情報（再生中のみ存在）
/// トーク完了・クリア時に remove される
/// メモリ戦略: SparseSet（動的追加/削除）
///
/// COMリソース（TextLayout）は TypewriterLayoutCache が保持。
/// このコンポーネントは論理情報のみを保持する。
#[derive(Component, Clone)]
#[component(storage = "SparseSet", on_remove = on_typewriter_talk_remove)]
pub struct TypewriterTalk {
    /// Stage 1 IR トークン列
    tokens: Vec<TypewriterToken>,

    // === 再生状態 ===
    state: TypewriterState,
    /// 再生開始時刻
    start_time: f64,
    /// 一時停止時の経過時間
    paused_elapsed: f64,
    /// 現在の表示クラスタ数
    visible_cluster_count: u32,
    /// 進行度（0.0〜1.0）
    progress: f32,
    /// 次に処理するタイムライン項目インデックス
    next_item_index: usize,
}

impl TypewriterTalk {
    /// トークン列と開始時刻から TypewriterTalk を生成
    pub fn new(tokens: Vec<TypewriterToken>, start_time: f64) -> Self {
        Self {
            tokens,
            state: TypewriterState::Playing,
            start_time,
            paused_elapsed: 0.0,
            visible_cluster_count: 0,
            progress: 0.0,
            next_item_index: 0,
        }
    }

    // === 操作 API ===

    /// 一時停止
    pub fn pause(&mut self, current_time: f64) {
        if self.state == TypewriterState::Playing {
            self.paused_elapsed = current_time - self.start_time;
            self.state = TypewriterState::Paused;
        }
    }

    /// 再開
    pub fn resume(&mut self, current_time: f64) {
        if self.state == TypewriterState::Paused {
            self.start_time = current_time - self.paused_elapsed;
            self.state = TypewriterState::Playing;
        }
    }

    /// 全文即時表示（LayoutCache がある場合のみ有効）
    pub fn skip(&mut self, total_cluster_count: u32) {
        self.visible_cluster_count = total_cluster_count;
        self.progress = 1.0;
        self.state = TypewriterState::Completed;
    }

    // === 状態取得 ===

    /// トークン列を取得
    pub fn tokens(&self) -> &[TypewriterToken] {
        &self.tokens
    }

    /// 再生状態を取得
    pub fn state(&self) -> TypewriterState {
        self.state
    }

    /// 進行度を取得 (0.0〜1.0)
    pub fn progress(&self) -> f32 {
        self.progress
    }

    /// 表示クラスタ数を取得
    pub fn visible_cluster_count(&self) -> u32 {
        self.visible_cluster_count
    }

    /// 完了しているかどうか
    pub fn is_completed(&self) -> bool {
        self.state == TypewriterState::Completed
    }

    /// 開始時刻を取得
    pub fn start_time(&self) -> f64 {
        self.start_time
    }

    // === 内部更新（TypewriterLayoutCache と連携） ===

    /// 現在時刻に基づいて状態を更新
    ///
    /// # Arguments
    /// * `current_time` - 現在時刻
    /// * `timeline` - Stage 2 IR タイムライン（LayoutCache から取得）
    ///
    /// # Returns
    /// 発火すべきイベントのリスト
    pub fn update(
        &mut self,
        current_time: f64,
        timeline: &TypewriterTimeline,
    ) -> Vec<(bevy_ecs::entity::Entity, TypewriterEventKind)> {
        if self.state != TypewriterState::Playing {
            return Vec::new();
        }

        let elapsed = current_time - self.start_time;
        let mut events_to_fire = Vec::new();

        // タイムラインを走査して表示状態を更新
        while self.next_item_index < timeline.items.len() {
            let item = &timeline.items[self.next_item_index];
            match item {
                TimelineItem::Glyph { show_at, .. } => {
                    if elapsed >= *show_at {
                        self.visible_cluster_count += 1;
                        self.next_item_index += 1;
                    } else {
                        break;
                    }
                }
                TimelineItem::Wait { start_at, duration } => {
                    if elapsed >= *start_at + *duration {
                        self.next_item_index += 1;
                    } else {
                        break;
                    }
                }
                TimelineItem::FireEvent {
                    target,
                    event,
                    fire_at,
                } => {
                    if elapsed >= *fire_at {
                        events_to_fire.push((*target, event.clone()));
                        self.next_item_index += 1;
                    } else {
                        break;
                    }
                }
            }
        }

        // 進行度を更新
        if timeline.total_cluster_count > 0 {
            self.progress = self.visible_cluster_count as f32 / timeline.total_cluster_count as f32;
        } else {
            self.progress = 1.0;
        }

        // 全クラスタ表示完了で Completed に遷移
        if self.visible_cluster_count >= timeline.total_cluster_count {
            self.state = TypewriterState::Completed;
        }

        events_to_fire
    }
}

fn on_typewriter_talk_remove(_world: DeferredWorld, hook: HookContext) {
    trace!(entity = ?hook.entity, "[TypewriterTalk] Removed");
}

// ============================================================
// TypewriterLayoutCache - 描画リソース（システムが自動生成）
// ============================================================

/// Typewriter 描画リソースキャッシュ
///
/// TypewriterTalk 追加時に描画システムが自動生成する。
/// COMリソース（IDWriteTextLayout）と Stage 2 IR を保持。
#[derive(Component)]
#[component(storage = "SparseSet", on_remove = on_layout_cache_remove)]
pub struct TypewriterLayoutCache {
    /// TextLayout（描画に使用）
    text_layout: IDWriteTextLayout,
    /// Stage 2 IR タイムライン
    timeline: TypewriterTimeline,
}

// SAFETY: 内包する 2 フィールドはいずれも Send + Sync である:
// - `text_layout: IDWriteTextLayout` … windows-rs（0.62）が当該 COM 型に対し
//   `unsafe impl Send/Sync` を生成済み（DirectWrite のレイアウト系オブジェクトは
//   読み取り中心の利用に対しスレッドアジャイルとして扱われる）。
// - `timeline: TypewriterTimeline` … String/Vec/f64/u32 のみのプレーンデータ。
// したがって本構造体は自動で Send + Sync を導出でき、下記の手動 impl は健全（かつ
// 上記により冗長）である。実利用上も TextLayout への変更系呼び出し（SetDrawingEffect 等、
// typewriter_draw.rs）は Bevy の Draw スケジュール内で当該エンティティを排他参照する
// システムからのみ行われ、跨スレッドの同時アクセスは発生しない。手動 impl は crate 全域の
// COM 保持コンポーネント（GraphicsCommandList 等）と表記を揃えるため明示で残す（撤去は
// S 観点の churn 判断事項）。
unsafe impl Send for TypewriterLayoutCache {}
unsafe impl Sync for TypewriterLayoutCache {}

impl TypewriterLayoutCache {
    /// 新規作成
    pub fn new(text_layout: IDWriteTextLayout, timeline: TypewriterTimeline) -> Self {
        Self {
            text_layout,
            timeline,
        }
    }

    /// TextLayout参照
    pub fn text_layout(&self) -> &IDWriteTextLayout {
        &self.text_layout
    }

    /// タイムライン参照
    pub fn timeline(&self) -> &TypewriterTimeline {
        &self.timeline
    }
}

impl std::fmt::Debug for TypewriterLayoutCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypewriterLayoutCache")
            .field("timeline", &self.timeline)
            .finish_non_exhaustive()
    }
}

fn on_layout_cache_remove(_world: DeferredWorld, hook: HookContext) {
    trace!(entity = ?hook.entity, "[TypewriterLayoutCache] Removed - COM resources released");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync`（typewriter.rs）の
    /// 健全性を固定する特性化テスト。内包フィールド（IDWriteTextLayout は windows-rs が
    /// Send+Sync 付与済み・TypewriterTimeline はプレーンデータ）により本構造体は本来
    /// 自動で Send+Sync を導出できる＝手動 impl は冗長だが健全、という不変条件をコンパイル時に
    /// 検証する。将来 !Send なフィールドが追加された場合は（手動 impl があっても）型として
    /// 不健全な状態をここで検出できないため、この境界の更新時は手動 impl の妥当性を再点検すること。
    #[test]
    fn test_typewriter_layout_cache_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TypewriterLayoutCache>();
    }

    #[test]
    fn test_typewriter_default() {
        let tw = Typewriter::default();
        assert_eq!(tw.font_family, "メイリオ");
        assert_eq!(tw.font_size, 16.0);
        assert!((tw.default_char_wait - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_typewriter_state_default() {
        let state = TypewriterState::default();
        assert_eq!(state, TypewriterState::Playing);
    }

    #[test]
    fn test_typewriter_state_transitions() {
        assert_eq!(TypewriterState::Playing, TypewriterState::Playing);
        assert_eq!(TypewriterState::Paused, TypewriterState::Paused);
        assert_eq!(TypewriterState::Completed, TypewriterState::Completed);
        assert_ne!(TypewriterState::Playing, TypewriterState::Paused);
    }

    // ============================================================
    // TypewriterTalk — デバイス非依存な再生状態マシンの特性化
    // （pause/resume/skip/update のロジックは DirectWrite 非依存。
    //  timeline は plain data 構造体として手組みできる）
    // ============================================================

    /// グリフ N 個を等間隔 `step` 秒で表示するだけの timeline を組み立てる。
    /// total_cluster_count = glyph_count。convert_to_timeline を経由せず
    /// 直接構築する（DirectWrite 非依存）。
    fn make_glyph_timeline(glyph_count: u32, step: f64) -> TypewriterTimeline {
        let mut items = Vec::new();
        let mut t = 0.0;
        for i in 0..glyph_count {
            t += step;
            items.push(TimelineItem::Glyph {
                cluster_index: i,
                show_at: t,
            });
        }
        TypewriterTimeline {
            full_text: "x".repeat(glyph_count as usize),
            items,
            total_duration: t,
            total_cluster_count: glyph_count,
        }
    }

    #[test]
    fn test_typewriter_talk_new_initial_state() {
        let talk = TypewriterTalk::new(vec![TypewriterToken::Text("ab".into())], 10.0);
        assert_eq!(talk.state(), TypewriterState::Playing);
        assert_eq!(talk.start_time(), 10.0);
        assert_eq!(talk.visible_cluster_count(), 0);
        assert_eq!(talk.progress(), 0.0);
        assert!(!talk.is_completed());
        assert_eq!(talk.tokens().len(), 1);
    }

    #[test]
    fn test_typewriter_talk_pause_records_elapsed_and_changes_state() {
        let mut talk = TypewriterTalk::new(vec![], 100.0);
        talk.pause(105.0);
        assert_eq!(talk.state(), TypewriterState::Paused);
        // resume で paused_elapsed(=5.0) を使って start_time を再計算するため、
        // resume(200.0) 後の start_time は 200 - 5 = 195 になる。
        talk.resume(200.0);
        assert_eq!(talk.state(), TypewriterState::Playing);
        assert_eq!(talk.start_time(), 195.0);
    }

    #[test]
    fn test_typewriter_talk_pause_is_noop_when_not_playing() {
        let mut talk = TypewriterTalk::new(vec![], 0.0);
        talk.pause(5.0);
        assert_eq!(talk.state(), TypewriterState::Paused);
        let start_after_first_pause = talk.start_time();
        // 既に Paused のとき pause を再呼び出ししても状態・start_time は変化しない。
        talk.pause(50.0);
        assert_eq!(talk.state(), TypewriterState::Paused);
        assert_eq!(talk.start_time(), start_after_first_pause);
    }

    #[test]
    fn test_typewriter_talk_resume_is_noop_when_not_paused() {
        let mut talk = TypewriterTalk::new(vec![], 100.0);
        // Playing 状態での resume は何もしない（start_time 不変）。
        talk.resume(999.0);
        assert_eq!(talk.state(), TypewriterState::Playing);
        assert_eq!(talk.start_time(), 100.0);
    }

    #[test]
    fn test_typewriter_talk_skip_forces_complete() {
        let mut talk = TypewriterTalk::new(vec![], 0.0);
        talk.skip(42);
        assert_eq!(talk.state(), TypewriterState::Completed);
        assert_eq!(talk.visible_cluster_count(), 42);
        assert_eq!(talk.progress(), 1.0);
        assert!(talk.is_completed());
    }

    #[test]
    fn test_typewriter_talk_update_returns_empty_when_not_playing() {
        let timeline = make_glyph_timeline(3, 1.0);
        let mut talk = TypewriterTalk::new(vec![], 0.0);
        talk.pause(0.0);
        // Paused 中は update が時刻に関わらず何も進めず空イベントを返す。
        let events = talk.update(1000.0, &timeline);
        assert!(events.is_empty());
        assert_eq!(talk.visible_cluster_count(), 0);
        assert_eq!(talk.state(), TypewriterState::Paused);
    }

    #[test]
    fn test_typewriter_talk_update_reveals_glyphs_up_to_elapsed() {
        // step=1.0 で 3 グリフ（show_at = 1,2,3）。start_time=0。
        let timeline = make_glyph_timeline(3, 1.0);
        let mut talk = TypewriterTalk::new(vec![], 0.0);

        // elapsed=0: まだどのグリフも show_at(>=1) に達していない。
        talk.update(0.0, &timeline);
        assert_eq!(talk.visible_cluster_count(), 0);
        assert_eq!(talk.state(), TypewriterState::Playing);

        // elapsed=1.5: show_at<=1.5 のグリフ1個のみ表示。
        talk.update(1.5, &timeline);
        assert_eq!(talk.visible_cluster_count(), 1);
        assert!((talk.progress() - (1.0 / 3.0)).abs() < f32::EPSILON);
        assert_eq!(talk.state(), TypewriterState::Playing);
    }

    #[test]
    fn test_typewriter_talk_update_completes_when_all_glyphs_visible() {
        let timeline = make_glyph_timeline(3, 1.0);
        let mut talk = TypewriterTalk::new(vec![], 0.0);
        // elapsed=10 で全グリフ（show_at<=3）が表示され Completed へ遷移。
        let events = talk.update(10.0, &timeline);
        assert!(events.is_empty());
        assert_eq!(talk.visible_cluster_count(), 3);
        assert_eq!(talk.progress(), 1.0);
        assert_eq!(talk.state(), TypewriterState::Completed);
        assert!(talk.is_completed());
    }

    #[test]
    fn test_typewriter_talk_update_zero_clusters_completes_immediately() {
        // グリフを持たない timeline（total_cluster_count=0）は
        // progress=1.0・即 Completed（0 >= 0）になる退化ケース。
        let timeline = TypewriterTimeline::empty();
        let mut talk = TypewriterTalk::new(vec![], 0.0);
        talk.update(0.0, &timeline);
        assert_eq!(talk.progress(), 1.0);
        assert_eq!(talk.state(), TypewriterState::Completed);
    }

    #[test]
    fn test_typewriter_talk_update_wait_gates_following_glyph() {
        // Wait(duration=5, start_at=0) の後に Glyph(show_at=5)。
        // next_item_index は Wait を「elapsed >= start_at+duration」まで通過しない。
        let timeline = TypewriterTimeline {
            full_text: "a".into(),
            items: vec![
                TimelineItem::Wait {
                    duration: 5.0,
                    start_at: 0.0,
                },
                TimelineItem::Glyph {
                    cluster_index: 0,
                    show_at: 5.0,
                },
            ],
            total_duration: 5.0,
            total_cluster_count: 1,
        };
        let mut talk = TypewriterTalk::new(vec![], 0.0);

        // elapsed=3: Wait 未満で break。グリフ未到達。
        talk.update(3.0, &timeline);
        assert_eq!(talk.visible_cluster_count(), 0);
        assert_eq!(talk.state(), TypewriterState::Playing);

        // elapsed=6: Wait 通過 → グリフ表示 → 全数表示で Completed。
        talk.update(6.0, &timeline);
        assert_eq!(talk.visible_cluster_count(), 1);
        assert_eq!(talk.state(), TypewriterState::Completed);
    }

    #[test]
    fn test_typewriter_talk_update_fires_event_at_threshold() {
        let target = bevy_ecs::entity::Entity::from_raw_u32(1).unwrap();
        // FireEvent(fire_at=2) を含み、その後 Glyph(show_at=3) で完了する timeline。
        let timeline = TypewriterTimeline {
            full_text: "a".into(),
            items: vec![
                TimelineItem::FireEvent {
                    target,
                    event: TypewriterEventKind::Paused,
                    fire_at: 2.0,
                },
                TimelineItem::Glyph {
                    cluster_index: 0,
                    show_at: 3.0,
                },
            ],
            total_duration: 3.0,
            total_cluster_count: 1,
        };
        let mut talk = TypewriterTalk::new(vec![], 0.0);

        // elapsed=1: fire_at(2) 未満 → イベント未発火・グリフ未到達。
        let events = talk.update(1.0, &timeline);
        assert!(events.is_empty());
        assert_eq!(talk.visible_cluster_count(), 0);

        // elapsed=2.5: FireEvent 通過（発火）するがグリフ(show_at=3)は未達で break。
        let events = talk.update(2.5, &timeline);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, target);
        assert_eq!(events[0].1, TypewriterEventKind::Paused);
        assert_eq!(talk.visible_cluster_count(), 0);
        assert_eq!(talk.state(), TypewriterState::Playing);
    }

    #[test]
    fn test_typewriter_talk_update_does_not_refire_event_on_second_call() {
        let target = bevy_ecs::entity::Entity::from_raw_u32(7).unwrap();
        let timeline = TypewriterTimeline {
            full_text: String::new(),
            items: vec![TimelineItem::FireEvent {
                target,
                event: TypewriterEventKind::Complete,
                fire_at: 1.0,
            }],
            total_duration: 1.0,
            total_cluster_count: 0,
        };
        let mut talk = TypewriterTalk::new(vec![], 0.0);

        // 1回目: イベント発火（next_item_index が進む）。
        // total_cluster_count=0 のため同時に Completed へ遷移する。
        let first = talk.update(5.0, &timeline);
        assert_eq!(first.len(), 1);

        // 2回目: 既に Completed なので update は早期 return し、再発火しない。
        let second = talk.update(6.0, &timeline);
        assert!(second.is_empty());
    }
}
