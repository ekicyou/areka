//! `ViewboxExecutor::render` の**フレーム計画の判定群**（`viewbox_draw.rs` の子モジュール）。
//!
//! [`viewbox_draw`](crate::viewbox_draw) の巨大化を避けるため、計画そのものを組み替える 2 つの純粋
//! 関数——エラー縮退規律（[`degrade_if_needed`]・縮退先は計画者の全域の道
//! [`ScrollPlanner::full_domain_plan`]）・想定外不整合の検知（[`plan_inconsistency`]）——をここへ集めた。
//! 2 つとも `pub(super)` で親（`viewbox_draw`）とその子からだけ見える。描画の子（`render`）と
//! 兄弟檻（`viewbox_draw_frame_render_tests.rs`）は `super::plan::..` で引く。

use crate::canvas::ContentCanvas;
use crate::layout::VisibleWindow;
use crate::region::ScaleContract;
use crate::viewbox::{DirtyRect, FramePlan, LineOverhang, ScrollPlanner};
use crate::writing::WritingMode;

/// エラー縮退規律の適用（Error Handling）——`plan` を必要なら**全域ダーティ Update** へ縮退する。
///
/// 2 トリガ（正しさ優先・いずれも透明フラッシュを起こす FullClear ではなく全域ダーティ Update）:
/// - **既定の見た目/方向の変更**（`rebuilt`＝`ensure_format` が format/行キャッシュを組み直した・
///   書式のほか既定の線と色の変更も含む）: committed ピクセルは古い見た目で描かれ前提が崩れるため
///   `debug!`＋全域ダーティへ縮退。
/// - **想定外不整合**（[`plan_inconsistency`] が理由を返す）: `plan` の `Update` が canvas/面寸と矛盾
///   （draw_lines 範囲外・dirty 面寸超過・矩形の行が draw_lines の部分集合でない、の 3 つ）する場合
///   `warn!`＋全域ダーティへ縮退（記録の無い縮退を作らない）。
///
/// [`FramePlan::FullClear`] は縮退対象外（Clear は既に全域リセット）。[`FramePlan::NoChange`] は
/// `rebuilt` のときのみ縮退（format 組み直し後の committed ピクセル前提消失を全域再描画で回復）。
#[allow(clippy::too_many_arguments)]
pub(super) fn degrade_if_needed(
    plan: FramePlan,
    rebuilt: bool,
    canvas: &ContentCanvas,
    window: &VisibleWindow,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
    overhangs: &[LineOverhang],
) -> FramePlan {
    match plan {
        FramePlan::FullClear => FramePlan::FullClear,
        FramePlan::NoChange => {
            if rebuilt {
                tracing::debug!(
                    "既定の見た目/方向の変更を検知——committed ピクセル前提消失のため全域ダーティへ縮退"
                );
                ScrollPlanner::full_domain_plan(
                    canvas,
                    window,
                    mode,
                    contract,
                    surface_size,
                    overhangs,
                )
            } else {
                FramePlan::NoChange
            }
        }
        FramePlan::Update {
            blit,
            dirty,
            draw_lines,
        } => {
            if rebuilt {
                tracing::debug!(
                    "既定の見た目/方向の変更を検知——format/行キャッシュ組み直し・全域ダーティへ縮退（committed ピクセル前提消失）"
                );
                ScrollPlanner::full_domain_plan(
                    canvas,
                    window,
                    mode,
                    contract,
                    surface_size,
                    overhangs,
                )
            } else if let Some(reason) =
                plan_inconsistency(&dirty, &draw_lines, canvas.residents.len(), surface_size)
            {
                tracing::warn!(
                    reason,
                    "plan と canvas/面寸の想定外不整合——全域ダーティ再描画へ縮退（正しさ優先・最悪でもレガシー全域再描画と等価）"
                );
                ScrollPlanner::full_domain_plan(
                    canvas,
                    window,
                    mode,
                    contract,
                    surface_size,
                    overhangs,
                )
            } else {
                FramePlan::Update {
                    blit,
                    dirty,
                    draw_lines,
                }
            }
        }
    }
}

/// `plan` の `Update` が canvas/面寸と矛盾していないか検査する（防御的・render の縮退経路が呼ぶ）。
///
/// 返り値 `Some(reason)` は縮退のログ理由・`None` は整合。通常経路（[`ScrollPlanner::derive_dirty_with_overhangs`]）は
/// 面寸クランプ済み・住人範囲内の index のみを返すため不発だが、行指紋と内容キャンバスの想定外不整合
/// （範囲外 index・面寸超過矩形・矩形の行が描画対象行に無い）を検知したら全域ダーティへ縮退させる
/// （ログ無し失敗経路を作らない）。
pub(super) fn plan_inconsistency(
    dirty: &[DirtyRect],
    draw_lines: &[usize],
    residents_len: usize,
    surface_size: (u32, u32),
) -> Option<&'static str> {
    if draw_lines.iter().any(|&i| i >= residents_len) {
        return Some("draw_lines に canvas 住人範囲外の index が含まれる");
    }
    let (w, h) = (surface_size.0 as u64, surface_size.1 as u64);
    if dirty
        .iter()
        .any(|d| d.rect.x as u64 + d.rect.w as u64 > w || d.rect.y as u64 + d.rect.h as u64 > h)
    {
        return Some("dirty 矩形が面寸を超える");
    }
    // 各矩形の行は描画対象行の部分集合（Phase 1 が draw_lines の資源しか組まないため、外れた
    // index はその矩形で描かれず復元が欠ける）。draw_lines は昇順（計画者の不変条件）ゆえ二分探索。
    if dirty
        .iter()
        .any(|d| d.lines.iter().any(|i| draw_lines.binary_search(i).is_err()))
    {
        return Some("dirty 矩形の行が draw_lines の部分集合でない");
    }
    None
}
