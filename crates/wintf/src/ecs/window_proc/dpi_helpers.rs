//! DPI 変更時の純粋ヘルパー関数
//!
//! - 中心保持補正: `calculate_physical_size_from_box_style`,
//!   `calculate_center_correction`, `correct_position_for_dpi_center_preserve`
//! - 提案位置の採否: `dpi_suggested_position_decision`

use tracing::{debug, trace, warn};
use windows::Win32::Foundation::RECT;

use crate::ecs::window::DpiSuggestedRectPolicy;
use crate::ecs::{Point, SizeI};

/// `WM_DPICHANGED` で OS 提案位置を採用するかの純判断。
///
/// - `Some((x, y))` = 提案位置を書く（`DpiChangeContext` も set する）。
/// - `None` = 書かない（`DpiChangeContext` も **set しない**）。
///
/// 「書かない」は番兵座標ではなく型上の別値で表現する。呼出側は
/// `DpiChangeContext::set` と `guarded_set_window_pos` を戻り値でまとめて分岐でき、
/// 残置コンテキストを後続の `WM_WINDOWPOSCHANGED` が DPI echo と誤認する競合を封じられる。
///
/// # Arguments
/// * `policy` - 当該窓の [`DpiSuggestedRectPolicy`]。component 未付与は `None` で表現する。
/// * `suggested` - `WM_DPICHANGED` の LPARAM が指す OS 提案矩形。
///
/// 寸は使わない（サイズは ECS レイアウトパイプラインの所管＝`SWP_NOSIZE`）ため、
/// 参照するのは矩形の左上のみである。
//
// 配線済み（タスク 5.1・Phase C）: `window_pos.rs` の `WM_DPICHANGED` が本関数の
// 戻り値で `DpiChangeContext::set` と `guarded_set_window_pos` をまとめて分岐する。
pub(super) fn dpi_suggested_position_decision(
    policy: Option<&DpiSuggestedRectPolicy>,
    suggested: &RECT,
) -> Option<(i32, i32)> {
    match policy {
        // 未付与 = 既定 = 従来挙動
        None | Some(DpiSuggestedRectPolicy::ApplyPosition) => Some((suggested.left, suggested.top)),
        Some(DpiSuggestedRectPolicy::ExternalAuthority) => None,
    }
}

/// BoxStyle.size と DPI スケールから物理ピクセルサイズを計算する。
///
/// `window_pos_sync_system` と同一の ceiling 変換ロジックを使用し、
/// 計算結果の一致を保証する。
///
/// Returns: 物理サイズ `SizeI`。`BoxStyle.size` が `None` または `Dimension::Px` 以外の場合は `None`。
fn calculate_physical_size_from_box_style(
    box_style: &crate::ecs::layout::BoxStyle,
    dpi: &crate::ecs::window::DPI,
) -> Option<SizeI> {
    use crate::ecs::layout::Dimension;

    let size = box_style.size.as_ref()?;
    let width = match size.width? {
        Dimension::Px(w) => w,
        _ => return None,
    };
    let height = match size.height? {
        Dimension::Px(h) => h,
        _ => return None,
    };

    Some(SizeI {
        width: (width * dpi.scale_x()).ceil() as i32,
        height: (height * dpi.scale_y()).ceil() as i32,
    })
}

/// 旧物理サイズと新物理サイズから中心保持補正量を算出する。
///
/// 補正量 `(dx, dy)` を `client_pos` に加算すると、
/// ウィンドウ中心座標がサイズ変更前後で不変となる。
///
/// 数学的証明:
/// ```text
/// old_center = pos + old_size / 2
/// new_center = (pos + correction) + new_size / 2
///            = pos + (old_size - new_size) / 2 + new_size / 2
///            = pos + old_size / 2
///            = old_center  ✓
/// ```
fn calculate_center_correction(old_physical_size: SizeI, new_physical_size: SizeI) -> (i32, i32) {
    (
        (old_physical_size.width - new_physical_size.width) / 2,
        (old_physical_size.height - new_physical_size.height) / 2,
    )
}

/// WM_WINDOWPOSCHANGED ハンドラ内で呼び出す中心保持補正のエントリポイント。
///
/// `dpi_context` が存在する場合にのみ補正を適用する。
/// `dpi_context` が `None` の場合、`client_pos` をそのまま返す。
pub(super) fn correct_position_for_dpi_center_preserve(
    client_pos: Point,
    client_size: SizeI,
    dpi_context: &Option<crate::ecs::window::DpiChangeContext>,
    box_style: Option<&crate::ecs::layout::BoxStyle>,
    dpi: &crate::ecs::window::DPI,
) -> Point {
    // DPI 変更なし → 補正不要
    let Some(_ctx) = dpi_context else {
        return client_pos;
    };

    // BoxStyle が取得できない → フォールバック
    let Some(bs) = box_style else {
        warn!("[WM_WINDOWPOSCHANGED] DPI center correction skipped: BoxStyle not found");
        return client_pos;
    };

    // 物理サイズ計算不可（Dimension::Px 以外等）→ フォールバック
    let Some(new_physical_size) = calculate_physical_size_from_box_style(bs, dpi) else {
        trace!("[WM_WINDOWPOSCHANGED] DPI center correction skipped: BoxStyle.size not Px");
        return client_pos;
    };

    let (dx, dy) = calculate_center_correction(client_size, new_physical_size);

    if dx == 0 && dy == 0 {
        trace!("[WM_WINDOWPOSCHANGED] DPI center correction: no size change, correction = (0, 0)");
        return client_pos;
    }

    let corrected = Point {
        x: client_pos.x + dx,
        y: client_pos.y + dy,
    };

    debug!(
        old_pos_x = client_pos.x,
        old_pos_y = client_pos.y,
        corrected_pos_x = corrected.x,
        corrected_pos_y = corrected.y,
        old_size_cx = client_size.width,
        old_size_cy = client_size.height,
        new_size_cx = new_physical_size.width,
        new_size_cy = new_physical_size.height,
        correction_dx = dx,
        correction_dy = dy,
        "[WM_WINDOWPOSCHANGED] DPI center correction applied"
    );

    corrected
}

#[cfg(test)]
#[path = "dpi_helpers_tests.rs"]
mod tests;
