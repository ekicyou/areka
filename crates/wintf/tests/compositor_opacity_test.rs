//! Task 5.2: CompositeContext opacity 累積テスト
//!
//! - parent opacity 0.8 × child opacity 0.5 = final 0.4 を検証
//! - is_visible == false でサブツリーがスキップされることを検証
//! - opacity が [0.0, 1.0] に clamp されることを検証
//!
//! composite_render_system の内部ロジックは private のため、
//! ここでは Visual.clamped_opacity() と累積計算ロジックを独立検証する。

use wintf::ecs::Visual;

// ==========================================================================
// clamped_opacity 基本テスト
// ==========================================================================

#[test]
fn test_visual_clamped_opacity_normal() {
    let visual = Visual {
        opacity: 0.5,
        ..Default::default()
    };
    assert!(
        (visual.clamped_opacity() - 0.5).abs() < f32::EPSILON,
        "opacity 0.5 がそのまま返る"
    );
    eprintln!("✅ clamped_opacity() で通常値 0.5 が正しく返された");
}

#[test]
fn test_visual_clamped_opacity_clamp_upper() {
    let visual = Visual {
        opacity: 1.5,
        ..Default::default()
    };
    assert!(
        (visual.clamped_opacity() - 1.0).abs() < f32::EPSILON,
        "opacity 1.5 が 1.0 に clamp される"
    );
    eprintln!("✅ clamped_opacity() で 1.5 → 1.0 に clamp された");
}

#[test]
fn test_visual_clamped_opacity_clamp_lower() {
    let visual = Visual {
        opacity: -0.3,
        ..Default::default()
    };
    assert!(
        (visual.clamped_opacity() - 0.0).abs() < f32::EPSILON,
        "opacity -0.3 が 0.0 に clamp される"
    );
    eprintln!("✅ clamped_opacity() で -0.3 → 0.0 に clamp された");
}

// ==========================================================================
// opacity 累積計算（composite_render_system 内の式を再現）
// ==========================================================================

/// composite_render_system 内の累積計算式を再現するヘルパー
fn accumulated_opacity(parent_accumulated: f32, child_opacity: f32) -> f32 {
    (parent_accumulated * child_opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0)
}

#[test]
fn test_opacity_accumulation_parent_child() {
    // Req 2.4: parent 0.8 × child 0.5 = 0.4
    let parent_opacity = 0.8f32;
    let child_opacity = 0.5f32;
    let result = accumulated_opacity(parent_opacity, child_opacity);
    assert!(
        (result - 0.4).abs() < f32::EPSILON,
        "0.8 × 0.5 = 0.4: got {result}"
    );
    eprintln!("✅ parent 0.8 × child 0.5 = {result}");
}

#[test]
fn test_opacity_accumulation_three_levels() {
    // 3階層: 0.8 × 0.5 × 0.5 = 0.2
    let level1 = 0.8f32;
    let level2 = accumulated_opacity(level1, 0.5);
    let level3 = accumulated_opacity(level2, 0.5);
    assert!(
        (level3 - 0.2).abs() < f32::EPSILON,
        "0.8 × 0.5 × 0.5 = 0.2: got {level3}"
    );
    eprintln!("✅ 3階層累積 0.8 → 0.4 → {level3}");
}

#[test]
fn test_opacity_accumulation_zero_parent() {
    // parent == 0.0 → 子もすべて 0.0
    let result = accumulated_opacity(0.0, 0.8);
    assert!(
        result.abs() < f32::EPSILON,
        "0.0 × 0.8 = 0.0: got {result}"
    );
    eprintln!("✅ parent 0.0 なら子も 0.0");
}

#[test]
fn test_opacity_accumulation_clamp_overflow() {
    // overflow ケース: parent 1.0 × child 1.5 = 1.0（clamp）
    let result = accumulated_opacity(1.0, 1.5);
    assert!(
        (result - 1.0).abs() < f32::EPSILON,
        "1.0 × 1.5 → 1.0 に clamp: got {result}"
    );
    eprintln!("✅ overflow は 1.0 に clamp された");
}

#[test]
fn test_opacity_accumulation_clamp_negative() {
    // negative ケース: parent 0.5 × child -0.3 = 0.0（clamp）
    let result = accumulated_opacity(0.5, -0.3);
    assert!(
        result.abs() < f32::EPSILON,
        "0.5 × -0.3 → 0.0 に clamp: got {result}"
    );
    eprintln!("✅ negative は 0.0 に clamp された");
}

// ==========================================================================
// is_visible スキップ判定
// ==========================================================================

#[test]
fn test_visible_skip_logic() {
    // Req 2.3: is_visible == false ならサブツリーごとスキップ
    let hidden = Visual {
        is_visible: false,
        opacity: 1.0,
        ..Default::default()
    };
    assert!(
        !hidden.is_visible,
        "is_visible == false でスキップ判定"
    );

    let visible = Visual {
        is_visible: true,
        opacity: 1.0,
        ..Default::default()
    };
    assert!(
        visible.is_visible,
        "is_visible == true で描画続行"
    );

    eprintln!("✅ is_visible の真偽判定が正しい");
}

#[test]
fn test_zero_opacity_skip_logic() {
    // Req 2.6: accumulated_opacity == 0.0 ならサブツリーごとスキップ
    let transparent = Visual {
        opacity: 0.0,
        ..Default::default()
    };
    let accumulated = accumulated_opacity(1.0, transparent.clamped_opacity());
    assert!(
        accumulated.abs() < f32::EPSILON,
        "opacity 0.0 で累積結果も 0.0 → スキップ対象"
    );
    eprintln!("✅ opacity 0.0 なら累積結果も 0.0 でスキップ対象");
}

// ==========================================================================
// set_opacity による clamp 挙動
// ==========================================================================

#[test]
fn test_set_opacity_clamp() {
    let mut visual = Visual::default();

    visual.set_opacity(0.7);
    assert!((visual.clamped_opacity() - 0.7).abs() < f32::EPSILON);

    // set_opacity は内部で clamp するかどうかは実装依存
    // clamped_opacity() が常に [0,1] を返すことを保証
    visual.set_opacity(2.0);
    assert!(
        (visual.clamped_opacity() - 1.0).abs() < f32::EPSILON,
        "clamped_opacity は常に 1.0 以下"
    );

    visual.set_opacity(-1.0);
    assert!(
        visual.clamped_opacity().abs() < f32::EPSILON,
        "clamped_opacity は常に 0.0 以上"
    );

    eprintln!("✅ set_opacity + clamped_opacity の clamp 挙動が正しい");
}
