//! 自動で上げてよい形の純粋な見分け（設計の PromotionShape）。
//!
//! 入力は走査の列と候補の添字と拡大と親の描画面の大きさだけ（ECS に触らない）。

// shortcut: 係から呼ぶのは後のタスク（3.x）。配線が入ったらこの許しを外す。
#![cfg_attr(not(test), allow(dead_code))]

use super::MixMode;
use super::draw::diff::{Parent, PxRect, item_bounds};
use super::promotion::Phase;
use super::world_visual::{ScanEntry, ScanKind, TreeScan};
use windows::Win32::Graphics::Direct2D::D2D1_INTERPOLATION_MODE_LINEAR;
use windows_numerics::Matrix3x2;

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

/// 整数の位置だけを上げる形へ落とすか（研究 R-e が揃わなかったときだけ真にする 1 か所）。
/// 2.4 の WARP の確かめ（`draw/emit_promotion_tests.rs`）で、原点を物理ピクセルで引く部分の列
/// （`TreeFrame::Segment`）なら小数の位置でも ±1 に揃ったので偽（設計の案 b）。
const INTEGER_POSITION_ONLY: bool = false;

/// 形の見分けの入力。
pub(crate) struct ShapeInput<'a> {
    pub scan: &'a TreeScan,
    /// 候補（節）の添字。
    pub idx: usize,
    /// 親の持ち主の拡大（`GlobalArrangement`）。
    pub scale: f32,
    /// 親の描画面の物理の大きさ。
    pub host_px: (u32, u32),
}

/// 自分の描画面の矩形（部分木の描く範囲を物理ピクセルへ外向きに丸め・にじみの余白込み・
/// 親の描画面と祖先の矩形の切り抜きで切ったもの）。原点はその左上。
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct Origin {
    pub px: (i32, i32),
    pub dip: (f32, f32),
    pub size_px: (u32, u32),
}

/// 上げても画面の画素が全部を節にした形と同じになるかを見分け、同じなら自分の描画面の矩形を返す。
///
/// 見る順は 1 後に描く中身との重なり・2 部分木の中の手動の持ち主・3 祖先の層・4 混ぜ方・5 収まり。
/// 手動の持ち主・ほかの自動の持ち主との重なり、回転や拡大、自分の角丸や図形の切り抜きは見ない。
/// 見えない候補と節でない添字は `Hidden`、値が決められない範囲は `OutOfBounds`。
pub(crate) fn check_shape(input: &ShapeInput) -> Result<Origin, Ineligible> {
    let entries = &input.scan.entries;
    let cand = entries
        .get(input.idx)
        .filter(|e| visible_node(e))
        .ok_or(Ineligible::Hidden)?;
    let sub = &entries[input.idx..cand.end];
    let extent = subtree_extent(input).ok_or(Ineligible::OutOfBounds)?;
    if later_overlap(input, cand.end, extent) {
        return Err(Ineligible::LaterContentOverlap);
    }
    if sub[1..]
        .iter()
        .any(|e| matches!(e.kind, ScanKind::Owner { .. }))
    {
        return Err(Ineligible::ContainsOwner);
    }
    // 軸に沿わない矩形の切り抜きも D2D では図形の層で描くので、層と同じく見送る。
    if cand.ancestors.layer || cand.ancestors.rect_clip_rotated {
        return Err(Ineligible::AncestorLayer);
    }
    if sub.iter().any(|e| visible_node(e) && !e.source_over) {
        return Err(Ineligible::NonSourceOver);
    }
    if !fits(extent, input, cand) {
        return Err(Ineligible::OutOfBounds);
    }
    if INTEGER_POSITION_ONLY && fractional_origin(cand, input.scale) {
        return Err(Ineligible::FractionalOrigin);
    }
    Ok(cut(extent, input, cand))
}

/// 上げた後の回: 部分木の描く範囲が `current` の矩形からはみ出したら、新しい範囲で決め直した矩形を返す。
/// 収まっていれば（縮むほうは追わない）、値が決められなければ `None`。
pub(crate) fn regrow(input: &ShapeInput, current: &Origin) -> Option<Origin> {
    let next = surface_rect(input)?;
    let cur = rect_of(current);
    (cur.union(rect_of(&next)) != cur).then_some(next)
}

/// 部分木の描く範囲から決めた自分の描画面の矩形（見分けをしない）。値が決められなければ `None`。
pub(crate) fn surface_rect(input: &ShapeInput) -> Option<Origin> {
    let cand = input.scan.entries.get(input.idx)?;
    Some(cut(subtree_extent(input)?, input, cand))
}

/// 節の下の手動の持ち主 `input.idx` の矩形が、同じ描画面の中で後に描く中身と重なるか
/// （重なりの順が全部を節にした形と違う）。値が決められなければ重なるとみなす。
pub(crate) fn owner_later_overlap(input: &ShapeInput) -> bool {
    let Some(e) = input.scan.entries.get(input.idx) else {
        return false;
    };
    content_px(e, input).is_none_or(|r| !r.is_empty() && later_overlap(input, input.idx + 1, r))
}

/// 候補の世界の位置（物理）に端数が在るか（整数の位置だけを上げる形へ落とす条件）。
pub(crate) fn fractional_origin(entry: &ScanEntry, scale: f32) -> bool {
    let m = entry.world.world_mat;
    (m.M31 * scale).fract() != 0.0 || (m.M32 * scale).fract() != 0.0
}

fn visible_node(e: &ScanEntry) -> bool {
    e.visible && matches!(e.kind, ScanKind::Node { .. })
}

/// 自動の持ち主（用意中・上げた）の節。部分の中は別の描画面に描く。
fn auto_owner(e: &ScanEntry) -> bool {
    matches!(
        e.kind,
        ScanKind::Node {
            phase: Phase::Preparing | Phase::Active
        }
    )
}

/// 節 1 つの中身の描く範囲（物理・外向き・にじみの余白込み）。中身が無ければ空、値が決められなければ `None`。
fn content_px(e: &ScanEntry, input: &ShapeInput) -> Option<PxRect> {
    let aabb = e.world.content_world_aabb;
    if aabb.is_empty() {
        return Some(PxRect::default());
    }
    let s = Matrix3x2::scale(input.scale, input.scale);
    let b = aabb.transform(s).0;
    let m = e.world.world_mat * s;
    // 中の命令の補間は走査の列に無い（D2D は中の命令の補間で描く）ので、最近傍でない余白をいつも足す。
    item_bounds(
        [b.left, b.top, b.right, b.bottom],
        [m.M11, m.M12, m.M21, m.M22],
        D2D1_INTERPOLATION_MODE_LINEAR,
        MixMode::default(),
        &Parent::default(),
        input.host_px,
    )
}

/// 候補の部分木の見える節の描く範囲の和（部分木の中の自動の持ち主も、上げれば吸い込むので含める）。
fn subtree_extent(input: &ShapeInput) -> Option<PxRect> {
    let entries = &input.scan.entries;
    let end = entries.get(input.idx)?.end;
    entries[input.idx..end]
        .iter()
        .filter(|e| visible_node(e))
        .try_fold(PxRect::default(), |acc, e| {
            Some(acc.union(content_px(e, input)?))
        })
}

/// 候補の部分より後ろの見える節の中身が `extent` と重なるか。ほかの自動の持ち主の部分は飛ばす。
/// 値が決められない中身と SOURCE_OVER でない中身（D2D は軸に沿う切り抜きか描画面の全体まで写す）は
/// 重なるとみなす。
fn later_overlap(input: &ShapeInput, from: usize, extent: PxRect) -> bool {
    let entries = &input.scan.entries;
    let mut j = from;
    while j < entries.len() {
        let e = &entries[j];
        if visible_node(e) && auto_owner(e) {
            j = e.end;
            continue;
        }
        if visible_node(e)
            && (!e.source_over
                || content_px(e, input).is_none_or(|r| !r.intersect(extent).is_empty()))
        {
            return true;
        }
        j += 1;
    }
    false
}

fn host_rect(input: &ShapeInput) -> PxRect {
    PxRect {
        left: 0,
        top: 0,
        right: i32::try_from(input.host_px.0).unwrap_or(i32::MAX),
        bottom: i32::try_from(input.host_px.1).unwrap_or(i32::MAX),
    }
}

/// 範囲が親の描画面と祖先の矩形の切り抜き（物理・丸めない）の交わりに収まるか。
fn fits(extent: PxRect, input: &ShapeInput, cand: &ScanEntry) -> bool {
    if extent.is_empty() {
        return true;
    }
    if extent.union(host_rect(input)) != host_rect(input) {
        return false;
    }
    cand.ancestors.rect_clip.is_none_or(|c| {
        let c = c.transform(Matrix3x2::scale(input.scale, input.scale)).0;
        extent.left as f32 >= c.left
            && extent.top as f32 >= c.top
            && extent.right as f32 <= c.right
            && extent.bottom as f32 <= c.bottom
    })
}

/// 範囲を親の描画面と祖先の矩形の切り抜き（物理へ外向きに丸めたもの）で切り、原点を左上に置く。
fn cut(extent: PxRect, input: &ShapeInput, cand: &ScanEntry) -> Origin {
    let mut r = extent.intersect(host_rect(input));
    if let Some(c) = cand.ancestors.rect_clip {
        let c = c.transform(Matrix3x2::scale(input.scale, input.scale)).0;
        let clip = PxRect {
            left: c.left.floor() as i32,
            top: c.top.floor() as i32,
            right: c.right.ceil() as i32,
            bottom: c.bottom.ceil() as i32,
        };
        r = r.intersect(clip);
    }
    Origin {
        px: (r.left, r.top),
        dip: (r.left as f32 / input.scale, r.top as f32 / input.scale),
        size_px: ((r.right - r.left) as u32, (r.bottom - r.top) as u32),
    }
}

fn rect_of(o: &Origin) -> PxRect {
    PxRect {
        left: o.px.0,
        top: o.px.1,
        right: o.px.0.saturating_add(o.size_px.0 as i32),
        bottom: o.px.1.saturating_add(o.size_px.1 as i32),
    }
}
