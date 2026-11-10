use crate::ecs::Visual;
use crate::ecs::graphics::GraphicsCommandList;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::component::Component;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::world::DeferredWorld;
use windows::Win32::Graphics::DirectWrite::*;

/// テキストの方向
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextDirection {
    #[default]
    HorizontalLeftToRight, // writing-mode: horizontal-tb, direction: ltr
    HorizontalRightToLeft, // writing-mode: horizontal-tb, direction: rtl
    VerticalRightToLeft,   // writing-mode: vertical-rl (Japanese)
    VerticalLeftToRight,   // writing-mode: vertical-lr
}

/// Labelコンポーネント: テキスト表示ウィジット
///
/// 色は`Brushes`コンポーネントで指定します。
/// ```ignore
/// world.spawn((
///     Label {
///         text: "Hello".to_string(),
///         font_family: "メイリオ".to_string(),
///         font_size: 16.0,
///         direction: TextDirection::default(),
///     },
///     Brushes::with_foreground(Brush::BLACK.as_color().unwrap()),
/// ));
/// ```
///
/// # フィールド
/// - `text`: 表示するテキスト (UTF-8)
/// - `font_family`: フォントファミリー名 (例: "メイリオ", "Arial")
/// - `font_size`: フォントサイズ（DirectWrite の DIP。範囲は確かめない）
/// - `direction`: テキストの方向
#[derive(Component)]
#[component(storage = "SparseSet", on_add = on_label_add, on_remove = on_label_remove)]
pub struct Label {
    pub text: String,
    pub font_family: String,
    pub font_size: f32,
    pub direction: TextDirection,
}

/// Label追加時のフック: Visualコンポーネントを自動挿入 (R4)
fn on_label_add(mut world: DeferredWorld, hook: HookContext) {
    // 既にVisualを持っている場合はスキップ
    if world.get::<Visual>(hook.entity).is_some() {
        return;
    }
    world
        .commands()
        .entity(hook.entity)
        .insert(Visual::default());
}

impl Default for Label {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_family: "メイリオ".to_string(),
            font_size: 16.0,
            direction: Default::default(),
        }
    }
}

/// Labelコンポーネント削除時のフック
/// GraphicsCommandListをクリアしてChanged検出に対応
fn on_label_remove(mut world: DeferredWorld, hook: HookContext) {
    let entity = hook.entity;
    // GraphicsCommandListを取得して中身をクリア(Changed検出のため)
    if let Some(mut cmd_list) = world.get_mut::<GraphicsCommandList>(entity) {
        cmd_list.set_if_neq(GraphicsCommandList::empty());
    }
}

/// 読む向きと行の向きを書式へ設定する。DirectWrite は組み合わせを配置を使う所で確かめる
/// （research.md 8.3）ので、設定の順はこのままでよい。
pub(crate) fn apply_direction(
    format: &IDWriteTextFormat,
    direction: TextDirection,
) -> windows::core::Result<()> {
    let (reading, flow) = match direction {
        TextDirection::HorizontalLeftToRight => (
            DWRITE_READING_DIRECTION_LEFT_TO_RIGHT,
            DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
        ),
        TextDirection::HorizontalRightToLeft => (
            DWRITE_READING_DIRECTION_RIGHT_TO_LEFT,
            DWRITE_FLOW_DIRECTION_TOP_TO_BOTTOM,
        ),
        TextDirection::VerticalRightToLeft => (
            DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
            DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT,
        ),
        TextDirection::VerticalLeftToRight => (
            DWRITE_READING_DIRECTION_TOP_TO_BOTTOM,
            DWRITE_FLOW_DIRECTION_LEFT_TO_RIGHT,
        ),
    };
    unsafe {
        format.SetReadingDirection(reading)?;
        format.SetFlowDirection(flow)
    }
}

#[cfg(test)]
#[path = "label_tests.rs"]
mod label_tests;
