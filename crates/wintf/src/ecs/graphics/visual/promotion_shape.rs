//! 自動で上げてよい形の純粋な見分け（設計の PromotionShape）。
//!
//! 見送りの理由の型だけを先に置く。見分けの中身は後のタスクで足す。

// shortcut: 係から呼ぶのは後のタスク（3.x）。配線が入ったらこの許しを外す。
#![cfg_attr(not(test), allow(dead_code))]

/// 節を自動で上げない（上げた持ち主を戻す）理由。語は記録の行に添える。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) enum Ineligible {
    /// 同じ描画面の中で後に描く中身と重なる。
    LaterContentOverlap,
    /// 部分木の中に手動の持ち主が在る。
    ContainsOwner,
    /// 祖先の節の不透明度の層（1 未満）か図形の切り抜き（角丸・任意の図形）の中。
    AncestorLayer,
    /// 部分木に `SOURCE_OVER` でない混ぜ方の件が在る。
    NonSourceOver,
    /// 描く範囲が親の描画面・祖先の矩形の切り抜きに収まらない。
    OutOfBounds,
    /// 原点の端数が 0 でない（研究 R-e が揃わなかったときだけ使う落とし）。
    FractionalOrigin,
    /// 見えない（自動の持ち主の即時の戻しに使う）。
    Hidden,
    /// 装置か WUC の資源が無い。
    NoDevice,
}

impl Ineligible {
    /// 記録の行に添える語。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LaterContentOverlap => "later_content_overlap",
            Self::ContainsOwner => "contains_owner",
            Self::AncestorLayer => "ancestor_layer",
            Self::NonSourceOver => "non_source_over",
            Self::OutOfBounds => "out_of_bounds",
            Self::FractionalOrigin => "fractional_origin",
            Self::Hidden => "hidden",
            Self::NoDevice => "no_device",
        }
    }
}
