//! `ViewboxExecutor::render` の**フレーム計画の判定群**（`viewbox_draw.rs` の子モジュール）。
//!
//! [`viewbox_draw`](crate::viewbox_draw) の巨大化を避けるため、計画そのものを組み替える 3 つの純粋
//! 関数——エラー縮退規律（[`degrade_if_needed`]）・全域ダーティ Update の唯一の生成口
//! （[`full_domain_update`]）・想定外不整合の検知（[`plan_inconsistency`]）——をここへ集めた。
//! 3 つとも `pub(super)` で親のファサードからのみ見える。親が `degrade_if_needed` を素の `use` で
//! 再束縛し、`plan_inconsistency` は兄弟檻（`viewbox_draw_frame_render_tests.rs`）が
//! `super::plan_inconsistency` で届くよう `#[cfg(test)]` で再束縛するため、crate 内から見た
//! 呼び出し方は分割前と同じ。

use crate::canvas::ContentCanvas;
use crate::layout::VisibleWindow;
use crate::region::ScaleContract;
use crate::viewbox::{DirtyRect, FramePlan, ScrollPlanner};
use crate::writing::WritingMode;

/// エラー縮退規律の適用（Error Handling）——`plan` を必要なら**全域ダーティ Update** へ縮退する。
///
/// 2 トリガ（正しさ優先・いずれも透明フラッシュを起こす FullClear ではなく全域ダーティ Update）:
/// - **フォント/方向変更**（`rebuilt`＝`ensure_format` が format/行キャッシュを組み直した）: committed
///   ピクセルは旧 format で描かれ前提が崩れるため `debug!`＋全域ダーティへ縮退。
/// - **想定外不整合**（[`plan_inconsistency`] が理由を返す）: `plan` の `Update` が canvas/面寸と矛盾
///   （draw_lines 範囲外・dirty 面寸超過）する場合 `warn!`＋全域ダーティへ縮退（ログ無し失敗経路を
///   作らない・記憶 areka-log-first-no-silent-failure）。
///
/// [`FramePlan::FullClear`] は縮退対象外（Clear は既に全域リセット）。[`FramePlan::NoChange`] は
/// `rebuilt` のときのみ縮退（format 組み直し後の committed ピクセル前提消失を全域再描画で回復）。
pub(super) fn degrade_if_needed(
    plan: FramePlan,
    rebuilt: bool,
    canvas: &ContentCanvas,
    window: &VisibleWindow,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> FramePlan {
    match plan {
        FramePlan::FullClear => FramePlan::FullClear,
        FramePlan::NoChange => {
            if rebuilt {
                tracing::debug!(
                    "フォント/方向変更を検知——committed ピクセル前提消失のため全域ダーティへ縮退"
                );
                full_domain_update(canvas, window, mode, contract, surface_size)
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
                    "フォント/方向変更を検知——format/行キャッシュ組み直し・全域ダーティへ縮退（committed ピクセル前提消失）"
                );
                full_domain_update(canvas, window, mode, contract, surface_size)
            } else if let Some(reason) =
                plan_inconsistency(&dirty, &draw_lines, canvas.residents.len(), surface_size)
            {
                tracing::warn!(
                    reason,
                    "plan と canvas/面寸の想定外不整合——全域ダーティ再描画へ縮退（正しさ優先・最悪でもレガシー全域再描画と等価）"
                );
                full_domain_update(canvas, window, mode, contract, surface_size)
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

/// 全域ダーティ Update（`blit=(0,0)`・dirty=面全域 1 枚・draw_lines=**可視窓の** GlyphRun 住人）を組む。
///
/// [`ScrollPlanner::derive_dirty`] を**空 prev**（`&[]`）で呼び、面全域 1 枚のダーティと
/// **可視窓（`first_visible_line` 以降）の** GlyphRun 住人の描画対象を得る（初回フレームと同一経路）。
/// 描画対象を可視窓で切るのは、全域再描画のオラクル `DrawExecutor::render` が
/// `skip(first_visible_line)` で可視窓より前の行を描かないためで、こちらも同じ結果になる
/// （タスク 3.4 でこの規律を揃えた——derivation-ledger.md「3.5.1 R-2 の決着」）。縮退の唯一の生成口。
pub(super) fn full_domain_update(
    canvas: &ContentCanvas,
    window: &VisibleWindow,
    mode: WritingMode,
    contract: &ScaleContract,
    surface_size: (u32, u32),
) -> FramePlan {
    let (dirty, draw_lines) =
        ScrollPlanner::derive_dirty(canvas, window, mode, contract, (0, 0), surface_size, &[]);
    FramePlan::Update {
        blit: (0, 0),
        dirty,
        draw_lines,
    }
}

/// `plan` の `Update` が canvas/面寸と矛盾していないか検査する（防御的・render の縮退経路が呼ぶ）。
///
/// 返り値 `Some(reason)` は縮退のログ理由・`None` は整合。通常経路（[`ScrollPlanner::derive_dirty`]）は
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
    // index はその矩形で描かれず復元が欠ける）。
    if dirty
        .iter()
        .any(|d| d.lines.iter().any(|i| !draw_lines.contains(i)))
    {
        return Some("dirty 矩形の行が draw_lines の部分集合でない");
    }
    None
}
