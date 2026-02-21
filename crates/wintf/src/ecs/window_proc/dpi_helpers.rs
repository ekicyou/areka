//! DPI 変更時の中心保持補正ヘルパー関数
//!
//! `calculate_physical_size_from_box_style`, `calculate_center_correction`,
//! `correct_position_for_dpi_center_preserve` の純粋関数群。

use tracing::{debug, trace, warn};
use windows::Win32::Foundation::*;

/// BoxStyle.size と DPI スケールから物理ピクセルサイズを計算する。
///
/// `window_pos_sync_system` と同一の ceiling 変換ロジックを使用し、
/// 計算結果の一致を保証する。
///
/// Returns: 物理サイズ `SIZE`。`BoxStyle.size` が `None` または `Dimension::Px` 以外の場合は `None`。
fn calculate_physical_size_from_box_style(
    box_style: &crate::ecs::layout::BoxStyle,
    dpi: &crate::ecs::window::DPI,
) -> Option<SIZE> {
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

    Some(SIZE {
        cx: (width * dpi.scale_x()).ceil() as i32,
        cy: (height * dpi.scale_y()).ceil() as i32,
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
fn calculate_center_correction(old_physical_size: SIZE, new_physical_size: SIZE) -> (i32, i32) {
    (
        (old_physical_size.cx - new_physical_size.cx) / 2,
        (old_physical_size.cy - new_physical_size.cy) / 2,
    )
}

/// WM_WINDOWPOSCHANGED ハンドラ内で呼び出す中心保持補正のエントリポイント。
///
/// `dpi_context` が存在する場合にのみ補正を適用する。
/// `dpi_context` が `None` の場合、`client_pos` をそのまま返す。
pub(super) fn correct_position_for_dpi_center_preserve(
    client_pos: POINT,
    client_size: SIZE,
    dpi_context: &Option<crate::ecs::window::DpiChangeContext>,
    box_style: Option<&crate::ecs::layout::BoxStyle>,
    dpi: &crate::ecs::window::DPI,
) -> POINT {
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

    let corrected = POINT {
        x: client_pos.x + dx,
        y: client_pos.y + dy,
    };

    debug!(
        old_pos_x = client_pos.x,
        old_pos_y = client_pos.y,
        corrected_pos_x = corrected.x,
        corrected_pos_y = corrected.y,
        old_size_cx = client_size.cx,
        old_size_cy = client_size.cy,
        new_size_cx = new_physical_size.cx,
        new_size_cy = new_physical_size.cy,
        correction_dx = dx,
        correction_dy = dy,
        "[WM_WINDOWPOSCHANGED] DPI center correction applied"
    );

    corrected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::layout::{BoxSize, BoxStyle, Dimension};
    use crate::ecs::window::DPI;
    use windows::Win32::Foundation::{POINT, SIZE};

    // ================================================================
    // calculate_center_correction tests
    // ================================================================

    #[test]
    fn test_center_correction_size_decrease() {
        // 200% → 125%: 800×600 → 500×375
        let old = SIZE { cx: 800, cy: 600 };
        let new = SIZE { cx: 500, cy: 375 };
        let (dx, dy) = calculate_center_correction(old, new);
        assert_eq!(dx, 150);
        assert_eq!(dy, 112); // (600 - 375) / 2 = 112 (integer division)
    }

    #[test]
    fn test_center_correction_size_increase() {
        // 125% → 200%: 500×375 → 800×600
        let old = SIZE { cx: 500, cy: 375 };
        let new = SIZE { cx: 800, cy: 600 };
        let (dx, dy) = calculate_center_correction(old, new);
        assert_eq!(dx, -150);
        assert_eq!(dy, -112);
    }

    #[test]
    fn test_center_correction_same_size() {
        let size = SIZE { cx: 600, cy: 400 };
        let (dx, dy) = calculate_center_correction(size, size);
        assert_eq!(dx, 0);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_center_correction_preserves_center() {
        // 任意のケース: old_center ≈ new_center を数値で検証
        // 整数除算のため最大 1px の丸め誤差が生じうる（design.md 記載済み）
        let old_pos = POINT { x: 100, y: 200 };
        let old_size = SIZE { cx: 800, cy: 600 };
        let new_size = SIZE { cx: 500, cy: 375 };

        let (dx, dy) = calculate_center_correction(old_size, new_size);
        let new_pos = POINT {
            x: old_pos.x + dx,
            y: old_pos.y + dy,
        };

        let old_center_x = old_pos.x + old_size.cx / 2;
        let old_center_y = old_pos.y + old_size.cy / 2;
        let new_center_x = new_pos.x + new_size.cx / 2;
        let new_center_y = new_pos.y + new_size.cy / 2;

        assert_eq!(old_center_x, new_center_x);
        // 整数除算丸め: (600-375)/2=112, 375/2=187 → 112+187=299 ≠ 300
        // 許容誤差 ≤ 1px
        assert!(
            (old_center_y - new_center_y).abs() <= 1,
            "center Y drift exceeds 1px: old={}, new={}",
            old_center_y,
            new_center_y
        );
    }

    // ================================================================
    // calculate_physical_size_from_box_style tests
    // ================================================================

    #[test]
    fn test_physical_size_from_box_style_125pct() {
        // 400×300 logical @ 125% (DPI 120) → 500×375 physical
        let bs = BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(400.0)),
                height: Some(Dimension::Px(300.0)),
            }),
            ..Default::default()
        };
        let dpi = DPI::from_dpi(120, 120);
        let result = calculate_physical_size_from_box_style(&bs, &dpi).unwrap();
        assert_eq!(result.cx, 500);
        assert_eq!(result.cy, 375);
    }

    #[test]
    fn test_physical_size_from_box_style_200pct() {
        // 400×300 logical @ 200% (DPI 192) → 800×600 physical
        let bs = BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(400.0)),
                height: Some(Dimension::Px(300.0)),
            }),
            ..Default::default()
        };
        let dpi = DPI::from_dpi(192, 192);
        let result = calculate_physical_size_from_box_style(&bs, &dpi).unwrap();
        assert_eq!(result.cx, 800);
        assert_eq!(result.cy, 600);
    }

    #[test]
    fn test_physical_size_from_box_style_none() {
        let bs = BoxStyle {
            size: None,
            ..Default::default()
        };
        let dpi = DPI::from_dpi(120, 120);
        assert!(calculate_physical_size_from_box_style(&bs, &dpi).is_none());
    }

    #[test]
    fn test_physical_size_from_box_style_ceiling() {
        // ceiling 境界値: 333.0 * 1.5 = 499.5 → ceil → 500
        let bs = BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(333.0)),
                height: Some(Dimension::Px(250.0)),
            }),
            ..Default::default()
        };
        let dpi = DPI::from_dpi(144, 144); // 150%
        let result = calculate_physical_size_from_box_style(&bs, &dpi).unwrap();
        assert_eq!(result.cx, (333.0_f32 * 1.5).ceil() as i32); // 500
        assert_eq!(result.cy, (250.0_f32 * 1.5).ceil() as i32); // 375
    }

    #[test]
    fn test_physical_size_from_box_style_non_px() {
        // Dimension::Percent → None
        let bs = BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Percent(100.0)),
                height: Some(Dimension::Px(300.0)),
            }),
            ..Default::default()
        };
        let dpi = DPI::from_dpi(120, 120);
        assert!(calculate_physical_size_from_box_style(&bs, &dpi).is_none());
    }
}
