//! 高レベルレイアウトコンポーネント
//!
//! BoxStyle統合コンポーネントと関連する値オブジェクト群を提供する。

use bevy_ecs::prelude::*;

use super::dimension::*;

// ===== 高レベルレイアウトコンポーネント（値オブジェクト） =====
// 注: これらの型は以前はComponentでしたが、BoxStyle統合後は値オブジェクトとして使用します。
// Componentとしての利用は廃止され、BoxStyleを使用してください。

/// ボックスサイズ（値オブジェクト）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxSize {
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
}

/// ボックスマージン（値オブジェクト）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxMargin(pub Rect<LengthPercentageAuto>);

/// ボックスパディング（値オブジェクト）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxPadding(pub Rect<LengthPercentage>);

/// ボックス配置タイプ（値オブジェクト）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BoxPosition {
    /// 相対配置（通常のフロー内配置）
    #[default]
    Relative,
    /// 絶対配置（親要素基準の座標指定）
    Absolute,
}

/// 絶対配置のインセット座標（値オブジェクト）
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxInset(pub Rect<LengthPercentageAuto>);

/// Flexコンテナ（値オブジェクト）
///
/// 注: BoxStyleのflex_direction, justify_content, align_itemsを直接使用することを推奨。
/// この型は後方互換性のために維持されています。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlexContainer {
    pub direction: taffy::FlexDirection,
    pub justify_content: Option<taffy::JustifyContent>,
    pub align_items: Option<taffy::AlignItems>,
}

impl Default for FlexContainer {
    fn default() -> Self {
        Self {
            direction: taffy::FlexDirection::Row,
            justify_content: None,
            align_items: None,
        }
    }
}

/// Flexアイテム（値オブジェクト）
///
/// 注: BoxStyleのflex_grow, flex_shrink, flex_basis, align_selfを直接使用することを推奨。
/// この型は後方互換性のために維持されています。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlexItem {
    pub grow: f32,
    pub shrink: f32,
    pub basis: Dimension,
    pub align_self: Option<taffy::AlignSelf>,
}

impl Default for FlexItem {
    fn default() -> Self {
        Self {
            grow: 0.0,
            shrink: 1.0,
            basis: Dimension::Auto,
            align_self: None,
        }
    }
}

// taffy の Flex 関連型を re-export（テストと外部利用のため）
pub use taffy::{AlignContent, AlignItems, AlignSelf, FlexDirection, JustifyContent};

// ===== BoxStyle統合コンポーネント =====

/// 統合レイアウトスタイルコンポーネント
///
/// 全レイアウトプロパティを統合したユーザー向けコンポーネント。
/// `TaffyStyle`と1:1対応し、`build_taffy_styles_system`で変換される。
///
/// # 設計意図
///
/// - Box系5種（size, margin, padding, position, inset）をOption型でネスト構造として含める
/// - Flex系7種（flex_direction, justify_content, align_items, flex_grow, flex_shrink, flex_basis, align_self）
///   をフラットなOption型フィールドとして含める（taffyのStyle構造体と同様のフラット設計）
/// - `None`フィールドはtaffyデフォルト値にマッピング
///
/// # 使用例
///
/// ```rust,ignore
/// use wintf::ecs::layout::*;
///
/// // Flexコンテナーとして使用
/// commands.spawn((
///     BoxStyle {
///         size: Some(BoxSize {
///             width: Some(Dimension::Percent(100.0)),
///             height: Some(Dimension::Percent(100.0)),
///         }),
///         flex_direction: Some(FlexDirection::Row),
///         justify_content: Some(JustifyContent::SPACE_EVENLY),
///         align_items: Some(AlignItems::CENTER),
///         ..Default::default()
///     },
/// ));
///
/// // Flexアイテムとして使用
/// commands.spawn((
///     BoxStyle {
///         size: Some(BoxSize {
///             width: Some(Dimension::Px(200.0)),
///             height: Some(Dimension::Px(100.0)),
///         }),
///         flex_grow: Some(1.0),
///         flex_shrink: Some(1.0),
///         ..Default::default()
///     },
/// ));
/// ```
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct BoxStyle {
    // === Box系プロパティ（ネスト構造） ===
    /// サイズ（width, height）
    pub size: Option<BoxSize>,
    /// 最小サイズ（min_width, min_height）
    pub min_size: Option<BoxSize>,
    /// 最大サイズ（max_width, max_height）
    pub max_size: Option<BoxSize>,
    /// マージン（外側余白）
    pub margin: Option<BoxMargin>,
    /// パディング（内側余白）
    pub padding: Option<BoxPadding>,
    /// 配置タイプ（Relative/Absolute）
    pub position: Option<BoxPosition>,
    /// インセット（絶対配置時の座標）
    pub inset: Option<BoxInset>,

    // === Flex系プロパティ（フラット構造） ===
    /// Flexコンテナーの主軸方向
    pub flex_direction: Option<FlexDirection>,
    /// 主軸方向の子要素配置
    pub justify_content: Option<JustifyContent>,
    /// 交差軸方向の子要素配置
    pub align_items: Option<AlignItems>,
    /// Flexアイテムの伸長率（デフォルト: 0.0）
    /// 注: Noneの場合はtaffyデフォルト値(0.0)を適用
    pub flex_grow: Option<f32>,
    /// Flexアイテムの収縮率（デフォルト: 1.0）
    /// 注: Noneの場合はtaffyデフォルト値(1.0)を適用
    pub flex_shrink: Option<f32>,
    /// Flexアイテムの基準サイズ
    pub flex_basis: Option<Dimension>,
    /// 自身の交差軸配置（親のalign_itemsを上書き）
    pub align_self: Option<AlignSelf>,
}

impl BoxStyle {
    /// 新しいBoxStyleを作成
    pub fn new() -> Self {
        Self::default()
    }
}

/// BoxSizeの指定軸のみをtaffyのサイズへ反映する（未指定軸はtaffyデフォルトを維持）
fn apply_box_size(target: &mut taffy::Size<taffy::Dimension>, src: &BoxSize) {
    if let Some(w) = src.width {
        target.width = w.into();
    }
    if let Some(h) = src.height {
        target.height = h.into();
    }
}

/// BoxStyleからtaffy::Styleへの変換
impl From<&BoxStyle> for taffy::Style {
    fn from(style: &BoxStyle) -> Self {
        let mut taffy_style = taffy::Style::default();

        // Box系プロパティ変換
        if let Some(size) = &style.size {
            apply_box_size(&mut taffy_style.size, size);
        }
        if let Some(min_size) = &style.min_size {
            apply_box_size(&mut taffy_style.min_size, min_size);
        }
        if let Some(max_size) = &style.max_size {
            apply_box_size(&mut taffy_style.max_size, max_size);
        }
        if let Some(margin) = &style.margin {
            taffy_style.margin = margin.0.into();
        }
        if let Some(padding) = &style.padding {
            taffy_style.padding = padding.0.into();
        }
        if let Some(position) = &style.position {
            taffy_style.position = match position {
                BoxPosition::Relative => taffy::Position::Relative,
                BoxPosition::Absolute => taffy::Position::Absolute,
            };
        }
        if let Some(inset) = &style.inset {
            taffy_style.inset = inset.0.into();
        }

        // Flex系プロパティ変換
        // コンテナープロパティ設定時にdisplay: Flexを自動設定
        if style.flex_direction.is_some()
            || style.justify_content.is_some()
            || style.align_items.is_some()
        {
            taffy_style.display = taffy::Display::Flex;
        }
        if let Some(dir) = style.flex_direction {
            taffy_style.flex_direction = dir;
        }
        if let Some(jc) = style.justify_content {
            taffy_style.justify_content = Some(jc);
        }
        if let Some(ai) = style.align_items {
            taffy_style.align_items = Some(ai);
        }

        // flex_grow/flex_shrinkはNone時にtaffyデフォルト値を適用
        taffy_style.flex_grow = style.flex_grow.unwrap_or(0.0);
        taffy_style.flex_shrink = style.flex_shrink.unwrap_or(1.0);

        if let Some(basis) = style.flex_basis {
            taffy_style.flex_basis = basis.into();
        }
        if let Some(align_self) = style.align_self {
            taffy_style.align_self = Some(align_self);
        }

        taffy_style
    }
}
