use areka_emo_compose::ScaleRatio;
use areka_parsers::balloon::{WindowPosition, WindowPositionRaw};

use super::*;
use crate::config::{BalloonSide, BalloonXMode, PlacementConfig, ScopeConfig};
use crate::shared_test_support::{cfg_of, k};

fn abs(x: i32, y: i32) -> ScriptOffset {
    ScriptOffset {
        x: OffsetAxis::Absolute(x),
        y: OffsetAxis::Absolute(y),
    }
}

fn rel(x: i32, y: i32) -> ScriptOffset {
    ScriptOffset {
        x: OffsetAxis::Relative(x),
        y: OffsetAxis::Relative(y),
    }
}

fn scope(offset: Option<(i32, i32)>) -> ScopeConfig {
    ScopeConfig {
        balloon_offset: offset,
        balloon_offset_descript_x: offset.map_or(0, |o| o.0),
        ..ScopeConfig::default()
    }
}

fn merged(
    offset: Option<(i32, i32)>,
    script: Option<ScriptOffset>,
    wp_x: Option<&str>,
    wp_y: Option<i32>,
    k_shell: ScaleRatio,
) -> PlacementConfig {
    let mut cfg = cfg_of(vec![(0, scope(offset))]);
    if let Some(script) = script {
        overlay_offset(cfg.scopes.get_mut(&0).unwrap(), &script);
    }
    let wps = [(
        0,
        (
            WindowPosition::new(None, wp_y),
            WindowPositionRaw::new(wp_x.map(str::to_string), None),
        ),
    )]
    .into();
    crate::scale_then_merge(&mut cfg, &wps, &[0], k_shell, ScaleRatio::ONE);
    cfg
}

#[test]
fn absolute_replaces_descript_offset() {
    let mut sc = scope(Some((476, 10)));
    overlay_offset(&mut sc, &abs(100, 0));
    assert_eq!(sc.balloon_offset, Some((100, 0)));
    assert_eq!(sc.balloon_offset_descript_x, 100);
}

#[test]
fn relative_adds_to_descript_offset() {
    let mut sc = scope(Some((476, 10)));
    overlay_offset(&mut sc, &rel(-10, 5));
    assert_eq!(sc.balloon_offset, Some((466, 15)));
    assert_eq!(sc.balloon_offset_descript_x, 466);
}

#[test]
fn relative_without_descript_starts_from_zero() {
    let mut sc = scope(None);
    overlay_offset(&mut sc, &rel(5, 5));
    assert_eq!(sc.balloon_offset, Some((5, 5)));
}

#[test]
fn axes_are_independent() {
    let mut sc = scope(Some((476, 10)));
    overlay_offset(
        &mut sc,
        &ScriptOffset {
            x: OffsetAxis::Absolute(1),
            y: OffsetAxis::Relative(2),
        },
    );
    assert_eq!(sc.balloon_offset, Some((1, 12)));
}

#[test]
fn relative_saturates() {
    let mut sc = scope(Some((i32::MAX, i32::MIN)));
    overlay_offset(&mut sc, &rel(1, -1));
    assert_eq!(sc.balloon_offset, Some((i32::MAX, i32::MIN)));
}

#[test]
fn overlaid_value_is_scaled_like_descript() {
    let cfg = merged(Some((476, 0)), Some(rel(-10, 0)), None, None, k(3, 2));
    assert_eq!(cfg.scopes[&0].balloon_offset, Some((699, 0)));
    let cfg = merged(Some((476, 0)), Some(abs(100, 0)), None, None, k(3, 2));
    assert_eq!(cfg.scopes[&0].balloon_offset, Some((150, 0)));
}

#[test]
fn windowposition_adjust_survives_offset_overlay() {
    let cfg = merged(
        Some((476, 0)),
        Some(abs(100, 7)),
        None,
        Some(20),
        ScaleRatio::ONE,
    );
    assert_eq!(cfg.scopes[&0].balloon_offset, Some((100, 27)));
}

#[test]
fn align_before_merge_is_overwritten_after_is_not() {
    let wps = [(
        0,
        (
            WindowPosition::new(None, None),
            WindowPositionRaw::new(Some("center".into()), None),
        ),
    )]
    .into();

    let mut before = cfg_of(vec![(0, ScopeConfig::default())]);
    overlay_align(before.scopes.get_mut(&0).unwrap(), &ScriptAlign::Left);
    crate::scale_then_merge(&mut before, &wps, &[0], ScaleRatio::ONE, ScaleRatio::ONE);
    assert_eq!(before.scopes[&0].balloon_x_mode, BalloonXMode::CenterTop);

    let mut after = cfg_of(vec![(0, ScopeConfig::default())]);
    crate::scale_then_merge(&mut after, &wps, &[0], ScaleRatio::ONE, ScaleRatio::ONE);
    assert_eq!(after.scopes[&0].balloon_x_mode, BalloonXMode::CenterTop);
    overlay_align(after.scopes.get_mut(&0).unwrap(), &ScriptAlign::Left);
    assert_eq!(after.scopes[&0].balloon_x_mode, BalloonXMode::Side);
    assert_eq!(after.scopes[&0].balloon_alignment, BalloonSide::Left);
}

fn aligned(align: ScriptAlign) -> ScopeConfig {
    let mut sc = ScopeConfig {
        balloon_alignment: BalloonSide::Right,
        ..ScopeConfig::default()
    };
    overlay_align(&mut sc, &align);
    sc
}

#[test]
fn align_words_map_to_side_and_mode() {
    let cases = [
        (ScriptAlign::Left, BalloonSide::Left, BalloonXMode::Side),
        (ScriptAlign::Right, BalloonSide::Right, BalloonXMode::Side),
        (
            ScriptAlign::Center,
            BalloonSide::Right,
            BalloonXMode::CenterTop,
        ),
        (
            ScriptAlign::Top,
            BalloonSide::Right,
            BalloonXMode::CenterTop,
        ),
        (
            ScriptAlign::Bottom,
            BalloonSide::Right,
            BalloonXMode::CenterBottom,
        ),
        (ScriptAlign::None, BalloonSide::Auto, BalloonXMode::Side),
        (
            ScriptAlign::Unknown("up".into()),
            BalloonSide::Auto,
            BalloonXMode::Side,
        ),
    ];
    for (align, side, mode) in cases {
        let sc = aligned(align.clone());
        assert_eq!(
            (sc.balloon_alignment, sc.balloon_x_mode),
            (side, mode),
            "{align:?}"
        );
    }
}

#[test]
fn script_placed_follows_component() {
    let mut world = World::new();
    let e = world.spawn_empty().id();
    assert!(!is_script_placed(&world, e));
    world.entity_mut(e).insert(ScriptBalloonPlacement {
        scope: 0,
        offset: None,
        align: None,
    });
    assert!(is_script_placed(&world, e));
    world.entity_mut(e).remove::<ScriptBalloonPlacement>();
    assert!(!is_script_placed(&world, e));
}
