//! dimension.rs の寸法プリミティブ型 → taffy 型変換のギャップテスト
//!
//! 対象: `Dimension` / `LengthPercentageAuto` / `LengthPercentage` / `Rect<T>` と
//! それぞれの taffy 型への From 変換、TaffyZero/TaffyAuto 定数。
//!
//! 注意: `LengthPercentageAuto::Percent` / `LengthPercentage::Percent` の taffy 変換は
//! `Dimension::Percent` と異なり 1/100 正規化を行わない（現状挙動の固定。proposals.md P49 参照）。

use wintf::ecs::layout::{
    Dimension, LengthPercentage, LengthPercentageAuto, Rect, TaffyAuto, TaffyZero,
};

// ===== Dimension → taffy::Dimension =====

/// Dimension::Px は taffy::Dimension::length へそのまま変換される
#[test]
fn test_dimension_px_to_taffy_length() {
    let taffy_dim: taffy::Dimension = Dimension::Px(123.5).into();
    assert_eq!(taffy_dim, taffy::Dimension::length(123.5));
}

/// Dimension::Auto は taffy::Dimension::auto へ変換される
#[test]
fn test_dimension_auto_to_taffy_auto() {
    let taffy_dim: taffy::Dimension = Dimension::Auto.into();
    assert_eq!(taffy_dim, taffy::Dimension::auto());
}

/// Dimension::Percent は 0.0-100.0 → 0.0-1.0 に正規化される（÷100）
#[test]
fn test_dimension_percent_to_taffy_normalized() {
    let half: taffy::Dimension = Dimension::Percent(50.0).into();
    assert_eq!(half, taffy::Dimension::percent(0.5));

    let full: taffy::Dimension = Dimension::Percent(100.0).into();
    assert_eq!(full, taffy::Dimension::percent(1.0));

    let zero: taffy::Dimension = Dimension::Percent(0.0).into();
    assert_eq!(zero, taffy::Dimension::percent(0.0));
}

/// Dimension の Default は Auto、TaffyZero/TaffyAuto 定数が一致する
#[test]
fn test_dimension_default_and_constants() {
    assert_eq!(Dimension::default(), Dimension::Auto);
    assert_eq!(<Dimension as TaffyZero>::ZERO, Dimension::Px(0.0));
    assert_eq!(<Dimension as TaffyAuto>::AUTO, Dimension::Auto);
}

// ===== LengthPercentageAuto → taffy::LengthPercentageAuto =====

/// const コンストラクター・Default・TaffyZero/TaffyAuto 定数の検証
#[test]
fn test_length_percentage_auto_constructors_and_constants() {
    const LPA_LENGTH: LengthPercentageAuto = LengthPercentageAuto::length(10.0);
    const LPA_PERCENT: LengthPercentageAuto = LengthPercentageAuto::percent(50.0);
    const LPA_AUTO: LengthPercentageAuto = LengthPercentageAuto::auto();
    const LPA_ZERO: LengthPercentageAuto = LengthPercentageAuto::zero();

    assert_eq!(LPA_LENGTH, LengthPercentageAuto::Px(10.0));
    assert_eq!(LPA_PERCENT, LengthPercentageAuto::Percent(50.0));
    assert_eq!(LPA_AUTO, LengthPercentageAuto::Auto);
    assert_eq!(LPA_ZERO, LengthPercentageAuto::Px(0.0));

    assert_eq!(LengthPercentageAuto::default(), LengthPercentageAuto::Auto);
    assert_eq!(
        <LengthPercentageAuto as TaffyZero>::ZERO,
        LengthPercentageAuto::Px(0.0)
    );
    assert_eq!(
        <LengthPercentageAuto as TaffyAuto>::AUTO,
        LengthPercentageAuto::Auto
    );
}

/// Px / Auto バリアントの taffy 変換
#[test]
fn test_length_percentage_auto_px_auto_to_taffy() {
    let px: taffy::LengthPercentageAuto = LengthPercentageAuto::Px(42.0).into();
    assert_eq!(px, taffy::LengthPercentageAuto::length(42.0));

    let auto: taffy::LengthPercentageAuto = LengthPercentageAuto::Auto.into();
    assert_eq!(auto, taffy::LengthPercentageAuto::auto());
}

/// 現状仕様の固定: LengthPercentageAuto::Percent は値をそのまま taffy に渡す（÷100 なし）
///
/// ドキュメントコメントは「0.0～100.0 の範囲で指定」と謳うが、taffy 側は 0.0-1.0 を
/// 期待するため、Dimension::Percent（÷100 あり）と整合しない。修正は挙動変更のため
/// 本ループでは現状挙動を固定する（proposals.md P49）。
#[test]
fn test_length_percentage_auto_percent_to_taffy_not_normalized() {
    let pct: taffy::LengthPercentageAuto = LengthPercentageAuto::Percent(50.0).into();
    // ÷100 されず percent(50.0) のまま（= taffy 解釈では 5000%）
    assert_eq!(pct, taffy::LengthPercentageAuto::percent(50.0));
    assert_ne!(pct, taffy::LengthPercentageAuto::percent(0.5));
}

// ===== LengthPercentage → taffy::LengthPercentage =====

/// const コンストラクター・Default・TaffyZero 定数の検証
/// （LengthPercentage は Auto を持たないため Default は Px(0.0)）
#[test]
fn test_length_percentage_constructors_and_constants() {
    const LP_LENGTH: LengthPercentage = LengthPercentage::length(7.5);
    const LP_PERCENT: LengthPercentage = LengthPercentage::percent(25.0);
    const LP_ZERO: LengthPercentage = LengthPercentage::zero();

    assert_eq!(LP_LENGTH, LengthPercentage::Px(7.5));
    assert_eq!(LP_PERCENT, LengthPercentage::Percent(25.0));
    assert_eq!(LP_ZERO, LengthPercentage::Px(0.0));

    assert_eq!(LengthPercentage::default(), LengthPercentage::Px(0.0));
    assert_eq!(
        <LengthPercentage as TaffyZero>::ZERO,
        LengthPercentage::Px(0.0)
    );
}

/// Px バリアントの taffy 変換と、Percent の現状仕様固定（÷100 なし。P49 参照）
#[test]
fn test_length_percentage_to_taffy() {
    let px: taffy::LengthPercentage = LengthPercentage::Px(3.0).into();
    assert_eq!(px, taffy::LengthPercentage::length(3.0));

    let pct: taffy::LengthPercentage = LengthPercentage::Percent(25.0).into();
    // ÷100 されず percent(25.0) のまま（= taffy 解釈では 2500%）
    assert_eq!(pct, taffy::LengthPercentage::percent(25.0));
    assert_ne!(pct, taffy::LengthPercentage::percent(0.25));
}

// ===== Rect<T> =====

/// Rect::zero() / Rect::auto() / Rect::default() の各辺が期待値になる
#[test]
fn test_rect_zero_auto_default() {
    let zero: Rect<LengthPercentageAuto> = Rect::zero();
    assert_eq!(zero.left, LengthPercentageAuto::Px(0.0));
    assert_eq!(zero.right, LengthPercentageAuto::Px(0.0));
    assert_eq!(zero.top, LengthPercentageAuto::Px(0.0));
    assert_eq!(zero.bottom, LengthPercentageAuto::Px(0.0));

    let auto: Rect<LengthPercentageAuto> = Rect::auto();
    assert_eq!(auto.left, LengthPercentageAuto::Auto);
    assert_eq!(auto.right, LengthPercentageAuto::Auto);
    assert_eq!(auto.top, LengthPercentageAuto::Auto);
    assert_eq!(auto.bottom, LengthPercentageAuto::Auto);

    // Default は T::default() を各辺に適用する
    let default_lpa: Rect<LengthPercentageAuto> = Rect::default();
    assert_eq!(default_lpa.left, LengthPercentageAuto::Auto);
    let default_lp: Rect<LengthPercentage> = Rect::default();
    assert_eq!(default_lp.left, LengthPercentage::Px(0.0));
}

/// Rect<LengthPercentageAuto> → taffy::Rect 変換（バリアント混在で各辺独立に変換される）
#[test]
fn test_rect_lpa_to_taffy_mixed_variants() {
    let rect = Rect {
        left: LengthPercentageAuto::Px(10.0),
        right: LengthPercentageAuto::Auto,
        top: LengthPercentageAuto::Px(0.0),
        bottom: LengthPercentageAuto::Px(40.0),
    };
    let taffy_rect: taffy::Rect<taffy::LengthPercentageAuto> = rect.into();
    assert_eq!(taffy_rect.left, taffy::LengthPercentageAuto::length(10.0));
    assert_eq!(taffy_rect.right, taffy::LengthPercentageAuto::auto());
    assert_eq!(taffy_rect.top, taffy::LengthPercentageAuto::length(0.0));
    assert_eq!(taffy_rect.bottom, taffy::LengthPercentageAuto::length(40.0));
}

/// Rect<LengthPercentage> → taffy::Rect 変換
#[test]
fn test_rect_lp_to_taffy() {
    let rect = Rect {
        left: LengthPercentage::Px(1.0),
        right: LengthPercentage::Px(2.0),
        top: LengthPercentage::Px(3.0),
        bottom: LengthPercentage::Px(4.0),
    };
    let taffy_rect: taffy::Rect<taffy::LengthPercentage> = rect.into();
    assert_eq!(taffy_rect.left, taffy::LengthPercentage::length(1.0));
    assert_eq!(taffy_rect.right, taffy::LengthPercentage::length(2.0));
    assert_eq!(taffy_rect.top, taffy::LengthPercentage::length(3.0));
    assert_eq!(taffy_rect.bottom, taffy::LengthPercentage::length(4.0));
}
