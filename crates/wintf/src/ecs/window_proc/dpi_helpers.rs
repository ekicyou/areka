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
        None | Some(DpiSuggestedRectPolicy::ApplyPosition) => {
            Some((suggested.left, suggested.top))
        }
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
mod tests {
    use super::*;
    use crate::ecs::layout::{BoxSize, BoxStyle, Dimension};
    use crate::ecs::window::DPI;
    use crate::ecs::{Point, SizeI};

    // ================================================================
    // calculate_center_correction tests
    // ================================================================

    #[test]
    fn test_center_correction_size_decrease() {
        // 200% → 125%: 800×600 → 500×375
        let old = SizeI { width: 800, height: 600 };
        let new = SizeI { width: 500, height: 375 };
        let (dx, dy) = calculate_center_correction(old, new);
        assert_eq!(dx, 150);
        assert_eq!(dy, 112); // (600 - 375) / 2 = 112 (integer division)
    }

    #[test]
    fn test_center_correction_size_increase() {
        // 125% → 200%: 500×375 → 800×600
        let old = SizeI { width: 500, height: 375 };
        let new = SizeI { width: 800, height: 600 };
        let (dx, dy) = calculate_center_correction(old, new);
        assert_eq!(dx, -150);
        assert_eq!(dy, -112);
    }

    #[test]
    fn test_center_correction_same_size() {
        let size = SizeI { width: 600, height: 400 };
        let (dx, dy) = calculate_center_correction(size, size);
        assert_eq!(dx, 0);
        assert_eq!(dy, 0);
    }

    #[test]
    fn test_center_correction_preserves_center() {
        // 任意のケース: old_center ≈ new_center を数値で検証
        // 整数除算のため最大 1px の丸め誤差が生じうる（design.md 記載済み）
        let old_pos = Point { x: 100, y: 200 };
        let old_size = SizeI { width: 800, height: 600 };
        let new_size = SizeI { width: 500, height: 375 };

        let (dx, dy) = calculate_center_correction(old_size, new_size);
        let new_pos = Point {
            x: old_pos.x + dx,
            y: old_pos.y + dy,
        };

        let old_center_x = old_pos.x + old_size.width / 2;
        let old_center_y = old_pos.y + old_size.height / 2;
        let new_center_x = new_pos.x + new_size.width / 2;
        let new_center_y = new_pos.y + new_size.height / 2;

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
        assert_eq!(result.width, 500);
        assert_eq!(result.height, 375);
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
        assert_eq!(result.width, 800);
        assert_eq!(result.height, 600);
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
        assert_eq!(result.width, (333.0_f32 * 1.5).ceil() as i32); // 500
        assert_eq!(result.height, (250.0_f32 * 1.5).ceil() as i32); // 375
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

    // ================================================================
    // correct_position_for_dpi_center_preserve tests
    // （純粋エントリポイント。dpi_context / box_style / サイズ計算の各
    //   フォールバック分岐と、実補正適用の writethrough を特性化する）
    // ================================================================

    use crate::ecs::window::DpiChangeContext;
    // `RECT` は `use super::*` 経由で可視（本体が import 済み）。

    fn make_box_style_px(w: f32, h: f32) -> BoxStyle {
        BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Px(w)),
                height: Some(Dimension::Px(h)),
            }),
            ..Default::default()
        }
    }

    fn make_dpi_context(dpi: DPI) -> DpiChangeContext {
        DpiChangeContext::new(dpi, RECT::default())
    }

    #[test]
    fn test_correct_position_returns_input_when_dpi_context_none() {
        // dpi_context が None（DPI 変更なし）→ client_pos をそのまま返す（補正不要）
        let client_pos = Point { x: 100, y: 200 };
        let client_size = SizeI { width: 800, height: 600 };
        let bs = make_box_style_px(400.0, 300.0);
        let dpi = DPI::from_dpi(120, 120);

        let result = correct_position_for_dpi_center_preserve(
            client_pos,
            client_size,
            &None,
            Some(&bs),
            &dpi,
        );
        assert_eq!(result.x, 100);
        assert_eq!(result.y, 200);
    }

    #[test]
    fn test_correct_position_returns_input_when_box_style_none() {
        // dpi_context は Some だが box_style が None → フォールバックで client_pos 素通し
        let client_pos = Point { x: 50, y: 75 };
        let client_size = SizeI { width: 800, height: 600 };
        let dpi = DPI::from_dpi(192, 192);
        let ctx = Some(make_dpi_context(dpi));

        let result =
            correct_position_for_dpi_center_preserve(client_pos, client_size, &ctx, None, &dpi);
        assert_eq!(result.x, 50);
        assert_eq!(result.y, 75);
    }

    #[test]
    fn test_correct_position_returns_input_when_size_not_px() {
        // dpi_context は Some だが BoxStyle.size が非 Px → 物理サイズ計算不可でフォールバック
        let client_pos = Point { x: 10, y: 20 };
        let client_size = SizeI { width: 800, height: 600 };
        let dpi = DPI::from_dpi(192, 192);
        let ctx = Some(make_dpi_context(dpi));
        let bs = BoxStyle {
            size: Some(BoxSize {
                width: Some(Dimension::Percent(100.0)),
                height: Some(Dimension::Px(300.0)),
            }),
            ..Default::default()
        };

        let result = correct_position_for_dpi_center_preserve(
            client_pos,
            client_size,
            &ctx,
            Some(&bs),
            &dpi,
        );
        assert_eq!(result.x, 10);
        assert_eq!(result.y, 20);
    }

    #[test]
    fn test_correct_position_returns_input_when_correction_is_zero() {
        // 新旧サイズが一致（補正量 = (0,0)）→ client_pos 素通し
        // BoxStyle 400×300 @ 192dpi(200%) → 物理 800×600。client_size も 800×600 で一致。
        let client_pos = Point { x: 100, y: 200 };
        let client_size = SizeI { width: 800, height: 600 };
        let dpi = DPI::from_dpi(192, 192);
        let ctx = Some(make_dpi_context(dpi));
        let bs = make_box_style_px(400.0, 300.0);

        let result = correct_position_for_dpi_center_preserve(
            client_pos,
            client_size,
            &ctx,
            Some(&bs),
            &dpi,
        );
        assert_eq!(result.x, 100);
        assert_eq!(result.y, 200);
    }

    #[test]
    fn test_correct_position_applies_center_preserving_correction() {
        // 実補正: 旧サイズ(client_size) 800×600 → 新サイズ(BoxStyle由来) 500×375。
        // correction = ((800-500)/2, (600-375)/2) = (150, 112)。
        // corrected = (100+150, 200+112) = (250, 312)。
        // 新サイズ 500×375 は BoxStyle 400×300 @ 120dpi(125%) で導出される。
        let client_pos = Point { x: 100, y: 200 };
        let client_size = SizeI { width: 800, height: 600 };
        let dpi = DPI::from_dpi(120, 120);
        let ctx = Some(make_dpi_context(dpi));
        let bs = make_box_style_px(400.0, 300.0);

        let result = correct_position_for_dpi_center_preserve(
            client_pos,
            client_size,
            &ctx,
            Some(&bs),
            &dpi,
        );
        assert_eq!(result.x, 250);
        assert_eq!(result.y, 312);

        // 中心保持の検証: 旧中心 ≈ 新中心（整数除算で Y は最大1px誤差）
        let old_center_x = client_pos.x + client_size.width / 2;
        let new_center_x = result.x + 500 / 2;
        assert_eq!(old_center_x, new_center_x);
    }

    // ================================================================
    // dpi_suggested_position_decision tests
    //
    // 位置づけ（design.md「Testing Strategy」1.）: **分岐網羅の補助**。
    // S1（OS 提案位置の素通し）の赤→緑の正証跡は `window_proc/mod.rs` の
    // dispatch 檻（タスク 4.3）が担う——是正前の欠陥は wndproc の無条件
    // 書込＝実配線に在るため、新設純関数上の模擬では証明力が足りない。
    // ここで固定するのは「判断関数の 3 分岐が仕様どおりの答えを返すこと」と
    // 「dpi=96 では 3 分岐の区別が消える（＝96 が欠陥を隠す性質）」の 2 点。
    // ================================================================

    // `DpiSuggestedRectPolicy` は `use super::*` 経由で可視（本体が import 済み）。

    /// 提案矩形を左上 `(left, top)`・寸 `(w, h)` で組む。
    fn make_suggested_rect(left: i32, top: i32, w: i32, h: i32) -> RECT {
        RECT {
            left,
            top,
            right: left + w,
            bottom: top + h,
        }
    }

    /// 判断結果を「最終位置」へ畳む消費側の模擬。
    /// `Some((x, y))` は提案位置を書く＝最終位置が提案位置になる。
    /// `None` は書かない＝現位置が最終位置として残る（外部権威が後で決める）。
    fn final_position(decision: Option<(i32, i32)>, current: (i32, i32)) -> (i32, i32) {
        decision.unwrap_or(current)
    }

    #[test]
    fn test_dpi_decision_absent_policy_applies_suggested_position() {
        // component 未付与 = 後方互換の既定値 = 従来どおり提案位置を書く
        let suggested = make_suggested_rect(1920, 240, 400, 300);
        let decision = dpi_suggested_position_decision(None, &suggested);
        assert_eq!(decision, Some((1920, 240)));
    }

    #[test]
    fn test_dpi_decision_apply_position_applies_suggested_position() {
        // 明示 ApplyPosition = 未付与と同一の答え
        let suggested = make_suggested_rect(1920, 240, 400, 300);
        let decision =
            dpi_suggested_position_decision(Some(&DpiSuggestedRectPolicy::ApplyPosition), &suggested);
        assert_eq!(decision, Some((1920, 240)));
    }

    #[test]
    fn test_dpi_decision_external_authority_does_not_write() {
        // 位置権威が外部（areka 配置システム）にある窓 = 書かない
        // 「書かない」は番兵座標ではなく型上の別値（None）で表現される。
        let suggested = make_suggested_rect(1920, 240, 400, 300);
        let decision = dpi_suggested_position_decision(
            Some(&DpiSuggestedRectPolicy::ExternalAuthority),
            &suggested,
        );
        assert_eq!(decision, None);
    }

    #[test]
    fn test_dpi_decision_default_policy_equals_absent_policy() {
        // 既定値が ApplyPosition であること（＝未付与と付与既定が同義）を固定する。
        // これが崩れると「未付与＝従来どおり」という後方互換の非退行保証が消える。
        assert_eq!(
            DpiSuggestedRectPolicy::default(),
            DpiSuggestedRectPolicy::ApplyPosition
        );

        let suggested = make_suggested_rect(-1024, 96, 400, 300);
        assert_eq!(
            dpi_suggested_position_decision(Some(&DpiSuggestedRectPolicy::default()), &suggested),
            dpi_suggested_position_decision(None, &suggested)
        );
    }

    #[test]
    fn test_dpi_decision_uses_rect_origin_not_extent() {
        // 書込先は提案矩形の左上であり、右下・寸ではない。
        // 同一原点で寸だけ違う 2 矩形が同じ答えを返すことで固定する
        // （寸は ECS レイアウトパイプラインの所管＝SWP_NOSIZE）。
        let small = make_suggested_rect(300, 400, 10, 10);
        let large = make_suggested_rect(300, 400, 5000, 5000);
        assert_eq!(
            dpi_suggested_position_decision(None, &small),
            Some((300, 400))
        );
        assert_eq!(
            dpi_suggested_position_decision(None, &large),
            dpi_suggested_position_decision(None, &small)
        );
    }

    #[test]
    fn test_dpi_decision_at_96_hides_the_branch_difference() {
        // Req 5.1/5.4 の「96 の自己整合が欠陥を隠す」性質を檻として明示する。
        // dpi=96 では提案位置 = 現位置（比 1.0）ゆえ、3 分岐すべてが
        // 現接地点 X を保存してしまい、分岐の区別が観測できない。
        let current = (1200, 400);
        let ratio = 96.0_f32 / 96.0;
        let suggested = make_suggested_rect(
            (current.0 as f32 * ratio).round() as i32,
            (current.1 as f32 * ratio).round() as i32,
            400,
            300,
        );

        for policy in [
            None,
            Some(&DpiSuggestedRectPolicy::ApplyPosition),
            Some(&DpiSuggestedRectPolicy::ExternalAuthority),
        ] {
            let final_pos = final_position(
                dpi_suggested_position_decision(policy, &suggested),
                current,
            );
            assert_eq!(
                final_pos, current,
                "dpi=96 では policy={policy:?} でも現位置が保存されるはず"
            );
        }
    }

    #[test]
    fn test_dpi_decision_at_120_and_192_separates_apply_from_external_authority() {
        // 96 以外の水準（120・192）でモニタ跨ぎ相当の提案シフトを注入すると、
        // 「提案位置を書く」分岐は現接地点 X を破壊し、
        // `ExternalAuthority` だけが現接地点 X を保存する。
        // 判定は絶対 px ではなく DPI 水準に対する比で表現する（Req 5.6）。
        let current = (1200, 400);

        for dpi in [120_u16, 192_u16] {
            let ratio = dpi as f32 / 96.0;
            let shifted_x = (current.0 as f32 * ratio).round() as i32;
            let shifted_y = (current.1 as f32 * ratio).round() as i32;
            let suggested = make_suggested_rect(shifted_x, shifted_y, 400, 300);
            assert_ne!(
                shifted_x, current.0,
                "dpi={dpi} では提案 X が現位置から動いている前提"
            );

            // 未付与・ApplyPosition: 提案位置を書く = 接地点 X が保存されない
            for policy in [None, Some(&DpiSuggestedRectPolicy::ApplyPosition)] {
                let final_pos = final_position(
                    dpi_suggested_position_decision(policy, &suggested),
                    current,
                );
                assert_eq!(
                    final_pos,
                    (shifted_x, shifted_y),
                    "dpi={dpi} policy={policy:?}: 提案位置が最終位置になるはず"
                );
                assert_ne!(
                    final_pos.0, current.0,
                    "dpi={dpi} policy={policy:?}: 接地点 X が破壊されることを明示する"
                );
            }

            // ExternalAuthority: 書かない = 現接地点 X が保存される
            let final_pos = final_position(
                dpi_suggested_position_decision(
                    Some(&DpiSuggestedRectPolicy::ExternalAuthority),
                    &suggested,
                ),
                current,
            );
            assert_eq!(
                final_pos, current,
                "dpi={dpi}: ExternalAuthority は現接地点を保存するはず"
            );
        }
    }
}
