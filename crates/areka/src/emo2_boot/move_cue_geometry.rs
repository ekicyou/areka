//! `\![move]` の移動先の計算（design.md「MoveGeometry」・要件 1.3・1.5・2.1〜2.4・2.6・3.4・4.1〜4.3・6.1・6.2）。
//!
//! 基準の枠（[`BaseFrame`]）・動かす窓の矩形と基準位置・指令・拡大率 k から、動かす窓の
//! 左上（吸い付きの前・物理 px）を求める。基準を解く（窓・作業領域・原点を World から選ぶ）
//! のは適用の側（`move_cue_place`）で、本ファイルは純粋（`World`・`tracing`・wintf の窓を
//! 綴らない）・決定論・panic しない（`saturating_*`）。
//!
//! # 式（軸ごと）
//!
//! - `Fix` → 動かす窓の今の左端（上端）。
//! - `Px(d)` → `枠の上の点 + scale_signed(d, k) − 動かす窓の中の点`。
//!
//! 枠の上の点は `left`＝左端・`right`＝右端（排他側＝左端＋幅）・`center`＝左端＋幅 / 2・
//! `base`＝左端＋basepos.x（縦も同じ）。幅・高さの半分は整数の割り算で切り捨てる。
//!
//! # 置き換え（[`FrameNote`]）
//!
//! - 作業領域（`screen`・`primaryscreen`）の基準の参照点 `base` は軸ごとに `center` へ（要件 4.1）。
//! - 原点（`global`）の基準の参照点が `left.top` 以外なら使わずに原点そのもの（要件 4.2）。
//! - 動かす窓の参照点には置き換えを当てない（要件 4.3）。
//!
//! 基準位置の解き手（[`BaseposResolver`]／[`CanonDefaultBasepos`]）も、`move_cue.rs` から
//! 中身を変えずにここへ移した。

use wintf::ecs::SizeI;

use areka_emo_compose::ScaleRatio;

use super::{Axis, AxisSpec, MoveDirective, RefPoint, RefX, RefY};
use crate::placement::resolver::{PointPx, RectPx};
use crate::placement::scale_signed;

// =============================================================================
// basepos 型シーム（task 7.2・R5.2・全て物理 px・R-6 対策）
// =============================================================================

/// basepos（基準位置）解決の**型シーム**（R5.2・design「BaseposResolver」）。
///
/// M1 実導出は [`CanonDefaultBasepos`]（正典既定＝x=サーフェス幅÷2・y=下端）のみ。宣言
/// `point.basepos` の実導出は本 spec の範囲外であり、このトレイトは追跡 spec
/// `areka-P0-surfaces-basepos` が別実装（宣言値を差す resolver）を差し込むための**差替点**
/// として型のみを予約する（doc/COMPAT_ARCHITECTURE.md §8 対応表に登記）。
///
/// 入力は窓寸法（物理 px・`WindowPos.size` 由来）のみ——論理 px 系（BoxStyle）を経由しない
/// ことで 2026-07-05 の二重スケール欠陥（R-6）を型で構造遮断する。
pub trait BaseposResolver {
    /// 窓寸法（物理 px）に対する basepos（窓左上原点相対・物理 px）を返す。
    fn basepos(&self, window_size: SizeI) -> PointPx;
}

/// 正典既定の basepos（x=サーフェス幅÷2・y=下端＝height・R5.2・A-1 裁定）。
///
/// emo2 は `point.basepos` を宣言せず、この正典既定がそのまま適用される正規経路。y の「下端」は
/// 窓左上原点からの相対＝サーフェス高さ（`height`）そのもの。奇数幅は整数除算で切り捨てる。
pub struct CanonDefaultBasepos;

impl BaseposResolver for CanonDefaultBasepos {
    fn basepos(&self, window_size: SizeI) -> PointPx {
        PointPx {
            x: window_size.width / 2,
            y: window_size.height,
        }
    }
}

// =============================================================================
// 基準の枠と移動先の計算
// =============================================================================

/// 基準の枠の 3 形（design.md「MoveGeometry」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseFrame {
    /// 窓（スコープ・`me`）。4 語すべてが使える。`basepos` は窓の左上からの基準位置（物理 px）。
    Window { rect: RectPx, basepos: PointPx },
    /// 作業領域（`screen`・`primaryscreen`）。`base` は軸ごとに `center` へ置き換える（要件 4.1）。
    Area(RectPx),
    /// 原点（`global`・主モニタの左上 (0,0)・要件 3.4）。参照点は使わない（要件 4.2）。
    Origin,
}

/// 基準を解いた後の置き換えの記録（適用の側が `move_frame_substituted` の `warn!` で出す）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameNote {
    /// 基準のスコープの窓が無く `screen` として解き直した（適用の側が作る・要件 3.7）。
    MissingBaseScope { scope: u32 },
    /// 作業領域の基準の参照点 `base` を `center` として扱った（要件 4.1）。
    AreaBaseToCenter { axis: Axis },
    /// 原点の基準の参照点（`left.top` 以外）を使わずに原点とした（要件 4.2）。
    GlobalOffsetIgnored { offset: RefPoint },
}

/// 計算の結果（design.md Service Interface）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveTarget {
    /// 吸い付きの前の左上（物理 px）。
    pub position: PointPx,
    /// 置き換えの後に実際に使った基準の参照点（記録用）。
    pub base_offset_used: RefPoint,
    /// 置き換えの記録（出現順）。空なら台本どおり。
    pub notes: Vec<FrameNote>,
}

/// 基準の枠・動かす窓の矩形と基準位置・指令・k から、動かす窓の左上（吸い付きの前）を求める。
///
/// `target_rect` は正の大きさであること（呼び手が確かめる）。両軸 `Fix` なら `target_rect` の
/// 左上を返す。`k` は台本の X・Y（作者基準）を物理 px へ写すのにだけ使う（要件 2.4）。
pub fn compute_move_target(
    frame: &BaseFrame,
    target_rect: RectPx,
    target_basepos: PointPx,
    directive: &MoveDirective,
    k: ScaleRatio,
) -> MoveTarget {
    let mut notes = Vec::new();
    let offset = directive.base_offset;

    // 枠の上の点（軸ごと）と、置き換えの後に実際に使った参照点。
    let (frame_x, frame_y, used) = match *frame {
        BaseFrame::Window { rect, basepos } => (
            point_x(rect, basepos.x, offset.x),
            point_y(rect, basepos.y, offset.y),
            offset,
        ),
        BaseFrame::Area(rect) => {
            let x = if offset.x == RefX::Base {
                notes.push(FrameNote::AreaBaseToCenter { axis: Axis::X });
                RefX::Center
            } else {
                offset.x
            };
            let y = if offset.y == RefY::Base {
                notes.push(FrameNote::AreaBaseToCenter { axis: Axis::Y });
                RefY::Center
            } else {
                offset.y
            };
            // 置き換えの後に `base` は残らないので basepos は読まれない（0 を渡す）。
            (point_x(rect, 0, x), point_y(rect, 0, y), RefPoint { x, y })
        }
        BaseFrame::Origin => {
            if offset != RefPoint::LEFT_TOP {
                notes.push(FrameNote::GlobalOffsetIgnored { offset });
            }
            (0, 0, RefPoint::LEFT_TOP)
        }
    };

    // 動かす窓の中の点（左上からのずれ）。置き換えは当てない（要件 4.3）。
    let mv = directive.move_offset;
    let inner_x = point_x(target_rect, target_basepos.x, mv.x).saturating_sub(target_rect.left);
    let inner_y = point_y(target_rect, target_basepos.y, mv.y).saturating_sub(target_rect.top);

    let axis = |spec: AxisSpec, current: i32, on_frame: i32, inner: i32| match spec {
        AxisSpec::Fix => current,
        AxisSpec::Px(d) => on_frame
            .saturating_add(scale_signed(d, k))
            .saturating_sub(inner),
    };

    MoveTarget {
        position: PointPx {
            x: axis(directive.x, target_rect.left, frame_x, inner_x),
            y: axis(directive.y, target_rect.top, frame_y, inner_y),
        },
        base_offset_used: used,
        notes,
    }
}

/// 矩形の横の参照点の画面座標（`base` は左端＋`base_x`）。半分は切り捨て。
fn point_x(rect: RectPx, base_x: i32, which: RefX) -> i32 {
    let w = rect.right.saturating_sub(rect.left);
    match which {
        RefX::Left => rect.left,
        RefX::Right => rect.right,
        RefX::Center => rect.left.saturating_add(w / 2),
        RefX::Base => rect.left.saturating_add(base_x),
    }
}

/// 矩形の縦の参照点の画面座標（`base` は上端＋`base_y`）。半分は切り捨て。
fn point_y(rect: RectPx, base_y: i32, which: RefY) -> i32 {
    let h = rect.bottom.saturating_sub(rect.top);
    match which {
        RefY::Top => rect.top,
        RefY::Bottom => rect.bottom,
        RefY::Center => rect.top.saturating_add(h / 2),
        RefY::Base => rect.top.saturating_add(base_y),
    }
}

#[cfg(test)]
#[path = "move_cue_geometry_tests.rs"]
mod tests;
