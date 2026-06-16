//! metrics.rs 系コンポーネントのギャップテスト (W4b-T)
//!
//! `LayoutScale` / `Opacity` / `TextLayoutMetrics` の Default・バリデーション・
//! クランプといった純粋ロジックを特性化する。これらはデバイス非依存で、
//! 既存テストでは `Opacity::clamped()`（visual/component_test）と
//! `TextLayoutMetrics` の構築（widget/vertical_text_layout_test）のみが
//! 触れられており、各 Default 値と validate() の警告経路は未検証だった。
//!
//! 対象: crates/wintf/src/ecs/layout/metrics.rs

#![allow(deprecated)] // Opacity は deprecated だが利用箇所があるため挙動を固定する

use wintf::ecs::layout::{LayoutScale, Opacity, TextLayoutMetrics};

// ========================================================================
// LayoutScale
// ========================================================================

/// LayoutScale::default() は恒等スケール (1.0, 1.0) を返す
#[test]
fn test_layout_scale_default_is_identity() {
    let scale = LayoutScale::default();
    assert_eq!(scale.x, 1.0, "default scale.x should be 1.0");
    assert_eq!(scale.y, 1.0, "default scale.y should be 1.0");
}

/// validate() は非ゼロスケールでは何もしない（パニックせず完走）
///
/// validate は警告ログのみで実行時エラーとはしない設計（mayレベル推奨）。
/// 正常値ではログも出ず、副作用なく戻ることを特性化する。
#[test]
fn test_layout_scale_validate_nonzero_is_noop() {
    let scale = LayoutScale { x: 2.0, y: 1.5 };
    scale.validate(); // 警告なし・パニックなしで戻る
}

/// validate() はゼロスケールでも警告ログのみでパニックしない
///
/// x=0 / y=0 双方のゼロ検出経路を通すが、戻り値も副作用（値の変更）もない。
#[test]
fn test_layout_scale_validate_zero_does_not_panic() {
    let zero_x = LayoutScale { x: 0.0, y: 1.0 };
    zero_x.validate(); // warn が出るがパニックしない

    let zero_y = LayoutScale { x: 1.0, y: 0.0 };
    zero_y.validate();

    let zero_both = LayoutScale { x: 0.0, y: 0.0 };
    zero_both.validate();
}

// ========================================================================
// Opacity (deprecated)
// ========================================================================

/// Opacity::default() は完全不透明 1.0 を返す
#[test]
fn test_opacity_default_is_fully_opaque() {
    let opacity = Opacity::default();
    assert_eq!(opacity.0, 1.0, "default opacity should be 1.0 (opaque)");
}

/// clamped() は範囲内の値をそのまま返す
#[test]
fn test_opacity_clamped_within_range() {
    assert_eq!(Opacity(0.5).clamped(), 0.5);
    assert_eq!(Opacity(0.0).clamped(), 0.0);
    assert_eq!(Opacity(1.0).clamped(), 1.0);
}

/// clamped() は範囲外の値を [0.0, 1.0] に丸める
#[test]
fn test_opacity_clamped_out_of_range() {
    assert_eq!(Opacity(-0.5).clamped(), 0.0, "負値は 0.0 に丸める");
    assert_eq!(Opacity(2.5).clamped(), 1.0, "1.0 超は 1.0 に丸める");
}

/// validate() は範囲内・範囲外いずれでも警告ログのみでパニックしない
///
/// validate は範囲外を warn するのみで値は変更しない（clamped とは独立）。
#[test]
fn test_opacity_validate_does_not_panic_or_mutate() {
    Opacity(0.5).validate(); // 範囲内: 警告なし
    Opacity(-1.0).validate(); // 範囲外: warn のみ
    Opacity(5.0).validate(); // 範囲外: warn のみ

    // validate は値を変更しない（clamp しない）ことを特性化
    let op = Opacity(5.0);
    op.validate();
    assert_eq!(op.0, 5.0, "validate は値を変更しない");
}

// ========================================================================
// TextLayoutMetrics
// ========================================================================

/// TextLayoutMetrics::default() は (0.0, 0.0)
#[test]
fn test_text_layout_metrics_default_is_zero() {
    let metrics = TextLayoutMetrics::default();
    assert_eq!(metrics.width, 0.0);
    assert_eq!(metrics.height, 0.0);
}

/// TextLayoutMetrics は PartialEq で値比較できる（変更検知の基礎）
#[test]
fn test_text_layout_metrics_partial_eq() {
    let a = TextLayoutMetrics {
        width: 100.0,
        height: 50.0,
    };
    let b = TextLayoutMetrics {
        width: 100.0,
        height: 50.0,
    };
    let c = TextLayoutMetrics {
        width: 100.0,
        height: 60.0,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}
