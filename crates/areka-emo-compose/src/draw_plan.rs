//! 描く計画 `DrawPlan` と、語 → 重ね方 `Mix` の写し（どちらも純粋な値）。

use areka_emo_atlas::ElementId;

use crate::method::ComposeMethod;
use crate::normalized::Transform;
use crate::plan::{BlitOp, Extent};

/// 重ね方（D2D の合成モードへの写しは emo-present の 1 か所）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mix {
    /// 普通に重ねる。
    SourceOver,
    /// 合成の後の α をベースのままにして重ねる。
    SourceAtop,
    /// ベースの下へ差し込む。
    DestinationOver,
    /// 範囲の中を透過も含めて置き換える。
    Copy,
    /// ベースの α に掛ける（色はベースのまま）。
    DestinationIn,
}

/// 整数の矩形（左上と大きさ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IRect {
    /// 左端。
    pub x: i64,
    /// 上端。
    pub y: i64,
    /// 幅。
    pub w: u32,
    /// 高さ。
    pub h: u32,
}

/// 計画の 1 項。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanItem {
    /// 絵 1 枚（`range` は `replace` の範囲）。
    Image {
        /// 絵の ID（通常の絵か `asis` 用の絵）。
        element: ElementId,
        /// 置く位置。
        transform: Transform,
        /// 重ね方。
        mix: Mix,
        /// `replace` の置き換える範囲。
        range: Option<IRect>,
    },
    /// 黒の不透明の塗り（入れ子の `asis` の下地）。
    Fill {
        /// 塗る範囲。
        rect: IRect,
    },
    /// 透明から始まる 1 枚のまとめを開く。
    GroupBegin {
        /// まとめを置く位置。
        transform: Transform,
        /// まとめ全体の重ね方。
        mix: Mix,
    },
    /// まとめを閉じる。
    GroupEnd,
}

/// 面 1 つの「描く順の項の列」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawPlan {
    /// 描く面。
    pub surface_id: u32,
    /// 今の土台の面。
    pub base_surface: u32,
    /// 外形。
    pub extent: Extent,
    /// 描く順の項。
    pub items: Vec<PlanItem>,
}

/// 語 → 重ね方の 1 か所の写し。描けない語は `None`。
pub(crate) fn mix_of(method: &ComposeMethod) -> Option<Mix> {
    match method {
        ComposeMethod::Overlay | ComposeMethod::Asis | ComposeMethod::Base => Some(Mix::SourceOver),
        ComposeMethod::OverlayFast => Some(Mix::SourceAtop),
        ComposeMethod::Interpolate => Some(Mix::DestinationOver),
        ComposeMethod::Replace => Some(Mix::Copy),
        ComposeMethod::Reduce => Some(Mix::DestinationIn),
        _ => None,
    }
}

/// 命令の列を計画の絵の項へ順に写す（描けない語の命令は入れない）。
pub(crate) fn push_images(items: &mut Vec<PlanItem>, ops: &[BlitOp]) {
    items.extend(ops.iter().filter_map(|op| {
        Some(PlanItem::Image {
            element: op.element,
            transform: op.transform,
            mix: mix_of(&op.method)?,
            range: None,
        })
    }));
}

#[cfg(test)]
#[path = "draw_plan_tests.rs"]
mod plan_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use areka_parsers::shell::BLEND_ELEMENT_WORDS;

    #[test]
    fn mix_table_per_word() {
        let t = |w: &str| mix_of(&ComposeMethod::from_name(w));
        assert_eq!(t("overlay"), Some(Mix::SourceOver));
        assert_eq!(t("add"), Some(Mix::SourceOver));
        assert_eq!(t("bind"), Some(Mix::SourceOver));
        assert_eq!(t("overlay-fast"), Some(Mix::SourceAtop));
        assert_eq!(t("overlayfast"), Some(Mix::SourceAtop));
        assert_eq!(t("interpolate"), Some(Mix::DestinationOver));
        assert_eq!(t("replace"), Some(Mix::Copy));
        assert_eq!(t("asis"), Some(Mix::SourceOver));
        assert_eq!(t("base"), Some(Mix::SourceOver));
        assert_eq!(t("reduce"), Some(Mix::DestinationIn));
        assert_eq!(t("auto"), None);
        assert_eq!(t("nonsense"), None);
    }

    #[test]
    fn blend_spellings_stay_undrawable() {
        for w in BLEND_ELEMENT_WORDS {
            let m = ComposeMethod::from_name(w);
            assert_eq!(mix_of(&m), None, "{w}: {m:?}");
            assert!(!m.is_implemented(), "{w}");
            assert!(!crate::method::is_implemented_name(w), "{w}");
        }
    }

    #[test]
    fn mix_exists_exactly_when_drawable() {
        for w in [
            "overlay",
            "replace",
            "asis",
            "base",
            "reduce",
            "auto",
            "blend-add",
            "x",
        ] {
            let m = ComposeMethod::from_name(w);
            assert_eq!(mix_of(&m).is_some(), m.is_implemented(), "{w}");
        }
    }
}
