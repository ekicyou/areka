use areka_parsers::shell::{Animation, AppendTarget, DrawMethod, Interval, Pattern};

use crate::bind::BindSet;
use crate::fold::fold_shell_with;
use crate::method::{BlendKind, BlendMode, ComposeMethod};
use crate::normalized::SurfaceMaster;
use crate::pattern::PatternState;
use crate::plan::{Extent, compute_extent};
use crate::plan_test_support::{
    append_def, bake_gray, bound_world, component_of, elem, fresh_world, ops, pattern, shell_mixed,
    shell_of, surface, surface_with_anims,
};
use crate::world::Records;
use areka_parsers::shell::{DefRef, Element, Surface};

fn elem_with(method: &str) -> Element {
    Element {
        method: DrawMethod::new(method.to_string()),
        ..elem(0, "a.png", 3, 4)
    }
}

fn at(p: Pattern, x: i64, y: i64) -> Pattern {
    Pattern { x, y, ..p }
}

fn anim(id: u32, interval: Interval, patterns: Vec<Pattern>) -> Animation {
    Animation {
        id,
        interval,
        patterns,
    }
}

fn folded(surfaces: Vec<Surface>, id: u32) -> SurfaceMaster {
    let mut world = fresh_world();
    fold_shell_with(&mut world, &shell_of(surfaces), Records::Quiet);
    component_of::<SurfaceMaster>(&world, id)
        .expect("面がある")
        .clone()
}

/// (語, X, Y) の並び（番号の順）。
fn words(master: &SurfaceMaster, anim_id: u32) -> Vec<(String, i64, i64)> {
    let a = master
        .animations
        .iter()
        .find(|a| a.id == anim_id)
        .expect("animation がある");
    a.patterns
        .iter()
        .map(|p| (p.method.as_str().to_string(), p.x, p.y))
        .collect()
}

#[test]
fn element_word_maps_to_its_compose_method_and_base_is_overlay() {
    let cases = [
        ("overlay", ComposeMethod::Overlay),
        ("base", ComposeMethod::Overlay),
        ("overlay-fast", ComposeMethod::OverlayFast),
        ("overlayfast", ComposeMethod::OverlayFast),
        ("interpolate", ComposeMethod::Interpolate),
        ("replace", ComposeMethod::Replace),
        ("asis", ComposeMethod::Asis),
        ("reduce", ComposeMethod::Reduce),
        (
            "blend-multiply",
            ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply)),
        ),
    ];
    for (word, want) in cases {
        let master = folded(vec![surface(10, vec![elem_with(word)])], 10);
        assert_eq!(master.elements[0].method, want, "{word}");
        assert_eq!(
            master.elements[0].transform.offset(),
            (3, 4),
            "element定義の X,Y は位置のまま: {word}"
        );
    }
}

#[test]
fn append_element_word_maps_the_same_way() {
    let shell = shell_mixed(
        vec![surface(10, vec![elem(0, "a.png", 0, 0)])],
        vec![append_def(
            vec![AppendTarget::Single(10)],
            vec![Element {
                layer: 1,
                ..elem_with("replace")
            }],
            Vec::new(),
            Vec::new(),
        )],
        vec![DefRef::Surface(0), DefRef::Append(0)],
    );
    let mut world = fresh_world();
    fold_shell_with(&mut world, &shell, Records::Quiet);
    let master = component_of::<SurfaceMaster>(&world, 10).expect("面がある");
    assert_eq!(master.elements[1].method, ComposeMethod::Replace);
}

#[test]
fn base_frame_of_a_plain_animation_loses_its_xy_and_keeps_the_word() {
    let master = folded(
        vec![surface_with_anims(
            10,
            vec![elem(0, "a.png", 0, 0)],
            vec![anim(
                1,
                Interval::Random { k: 2 },
                vec![
                    at(pattern(0, "base", 20), 7, 8),
                    at(pattern(1, "overlay", 21), 7, 8),
                    at(pattern(2, "move", -1), 5, 6),
                    at(pattern(3, "Base", 22), 9, 9),
                ],
            )],
        )],
        10,
    );
    assert_eq!(
        words(&master, 1),
        vec![
            ("base".into(), 0, 0),
            ("overlay".into(), 7, 8),
            ("move".into(), 5, 6),
            ("Base".into(), 0, 0),
        ]
    );
}

#[test]
fn bind_pattern0_base_replaces_and_later_base_is_overlay_at_its_xy() {
    for interval in [Interval::Bind, Interval::BindRandom { k: 3 }] {
        let master = folded(
            vec![surface_with_anims(
                10,
                vec![elem(0, "a.png", 0, 0)],
                vec![anim(
                    1,
                    interval.clone(),
                    vec![
                        at(pattern(2, "base", 21), 4, 5),
                        at(pattern(0, "base", 20), 7, 8),
                        at(pattern(3, "move", -1), 1, 1),
                    ],
                )],
            )],
            10,
        );
        assert_eq!(
            words(&master, 1),
            vec![
                ("overlay".into(), 4, 5),
                ("base".into(), 0, 0),
                ("move".into(), 1, 1),
            ],
            "{interval:?}"
        );
    }
}

#[test]
fn append_animation_is_normalized_too() {
    let shell = shell_mixed(
        vec![surface(10, vec![elem(0, "a.png", 0, 0)])],
        vec![append_def(
            vec![AppendTarget::Single(10)],
            Vec::new(),
            Vec::new(),
            vec![anim(
                4,
                Interval::Other("always".into()),
                vec![at(pattern(0, "base", 20), 7, 8)],
            )],
        )],
        vec![DefRef::Surface(0), DefRef::Append(0)],
    );
    let mut world = fresh_world();
    fold_shell_with(&mut world, &shell, Records::Quiet);
    let master = component_of::<SurfaceMaster>(&world, 10).expect("面がある");
    assert_eq!(words(master, 4), vec![("base".into(), 0, 0)]);
}

/// `base` のコマの外形は置き換え先の面（XY を足さない）。後ろの `base` は overlay なので XY に置く。
#[test]
fn extent_of_a_base_frame_is_the_target_surface() {
    let atlas = bake_gray(&[("base.png", (40, 30)), ("big.png", (60, 50))]);
    let target = surface(20, vec![elem(0, "big.png", 0, 0)]);
    let host =
        |anim: Animation| surface_with_anims(10, vec![elem(0, "base.png", 0, 0)], vec![anim]);

    let always = anim(
        1,
        Interval::Other("always".into()),
        vec![at(pattern(0, "base", 20), 100, 100)],
    );
    let world = bound_world(&shell_of(vec![host(always), target.clone()]), &[], &atlas);
    assert_eq!(compute_extent(&world, &atlas, 10), Extent { w: 60, h: 50 });

    let bind = anim(
        1,
        Interval::Bind,
        vec![
            at(pattern(0, "base", 20), 100, 100),
            at(pattern(1, "base", 20), 10, 0),
        ],
    );
    let world = bound_world(&shell_of(vec![host(bind), target]), &[], &atlas);
    assert_eq!(compute_extent(&world, &atlas, 10), Extent { w: 70, h: 50 });
}

/// 着せ替えの pattern0 の `base` は層 (i) を (0,0) で置き換え、pattern1 の `base` は書いた XY に積まれる。
#[test]
fn bind_steps_read_the_normalized_patterns() {
    let atlas = bake_gray(&[("base.png", (40, 30)), ("big.png", (60, 50))]);
    let bind = anim(
        1,
        Interval::Bind,
        vec![
            at(pattern(0, "base", 20), 100, 100),
            at(pattern(1, "base", 20), 10, 0),
        ],
    );
    let shell = shell_of(vec![
        surface_with_anims(10, vec![elem(0, "base.png", 0, 0)], vec![bind]),
        surface(20, vec![elem(0, "big.png", 0, 0)]),
    ]);
    let world = bound_world(&shell, &[], &atlas);
    let offsets: Vec<_> = ops(
        &world,
        &atlas,
        10,
        &BindSet::from_ids([1]),
        &PatternState::default(),
    )
    .iter()
    .map(|op| op.transform.offset())
    .collect();
    assert_eq!(offsets, vec![(0, 0), (10, 0)]);
}
