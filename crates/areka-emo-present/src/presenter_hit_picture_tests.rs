//! 描き直さない時点・マスクの控え・土台の面の付け替えの実 GPU のテスト。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, ComposeMethod, EmoWorld, PatternFrame};
use wintf::ecs::GraphicsCore;

use areka_parsers::shell::{
    AppendTarget, Collision, CollisionBounds, CollisionName, CollisionShape, Surface,
};

use super::test_support::{
    CapturedEvent, FIVE_SHAPES, FIVE_SHAPES_MASK, attach_and_show, bake_world, build_balloon,
    capture, elem, five_shapes_assets, frames, gradient, gradient_with_alpha, hit_generation,
    load_hit_shell, make_world_with_gpu, mask_pictures_created, name_at, paint_screen,
    set_window_dpi, shell_of, show_ok, show_with_ok, spawn_window_with_dpi, straight_image,
    warn_count,
};
use super::{EmoPresenter, PatternState, TargetId};

const T: TargetId = TargetId(0);

fn show_binds(
    presenter: &mut EmoPresenter,
    world: &mut bevy_ecs::world::World,
    surface: u32,
    id: u32,
) {
    show_with_ok(
        presenter,
        world,
        T,
        surface,
        BindSet::from_ids(id..=id),
        PatternState::default(),
    );
}

#[test]
fn repeated_checks_new_bind_keys_and_scale_changes_keep_the_generation() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, five_shapes_assets(), 0, 96);
    let generation = hit_generation(&presenter, T).expect("描き先がある");

    for _ in 0..20 {
        assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("R"));
        assert_eq!(name_at(&mut world, &presenter, T, 20, 4), Some("B"));
    }
    assert_eq!(hit_generation(&presenter, T), Some(generation));

    for id in 1..=6 {
        show_binds(&mut presenter, &mut world, 0, id);
    }
    assert_eq!(hit_generation(&presenter, T), Some(generation));

    let window = presenter.targets[&T].window;
    for dpi in [192, 120, 96] {
        let before = presenter.applied_ratio(T);
        set_window_dpi(&mut world, window, dpi);
        show_binds(&mut presenter, &mut world, 0, 7);
        assert_ne!(
            presenter.applied_ratio(T),
            before,
            "dpi {dpi}: 拡大率が変わっていない"
        );
        assert_eq!(hit_generation(&presenter, T), Some(generation), "dpi {dpi}");
    }
}

#[test]
fn frame_advances_keep_the_generation_and_the_name() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let (emo_world, atlas) = build_balloon();
    let steps = [
        frames(&emo_world, Some(0), Some(0)),
        frames(&emo_world, Some(1), Some(1)),
        frames(&emo_world, Some(2), Some(0)),
        frames(&emo_world, Some(2), Some(1)),
    ];
    let window = spawn_window_with_dpi(&mut world, 96);
    presenter.attach_target(T, window, emo_world, atlas, 96);
    show_with_ok(
        &mut presenter,
        &mut world,
        T,
        0,
        BindSet::default(),
        steps[0].clone(),
    );
    let generation = hit_generation(&presenter, T).expect("描き先がある");
    assert_eq!(name_at(&mut world, &presenter, T, 4, 0), Some("Body"));

    for (i, pattern) in steps.iter().enumerate() {
        show_with_ok(
            &mut presenter,
            &mut world,
            T,
            0,
            BindSet::default(),
            pattern.clone(),
        );
        assert_eq!(hit_generation(&presenter, T), Some(generation), "step {i}");
        assert_eq!(
            name_at(&mut world, &presenter, T, 4, 0),
            Some("Body"),
            "step {i}"
        );
    }
}

const TWO_FACES_A: &str = "collisionex0,A,region,ma.png,255,0,0\n";
const TWO_FACES_B: &str = "collisionex0,B,region,mb.png,255,0,0\n";

/// 面 0（名前 A・マスク ma）と面 1（名前 B・マスク mb）。どちらも 16×16 で左 8 列が指定色。
fn two_region_faces() -> (EmoWorld, AtlasTable) {
    let mask = || {
        straight_image(16, 16, |x, _| {
            if x < 8 {
                [0, 0, 255, 255]
            } else {
                [255, 0, 0, 255]
            }
        })
    };
    let body = format!("{TWO_FACES_A}}}\nsurface1\n{{\n{TWO_FACES_B}");
    load_hit_shell_two(&body, mask())
}

fn load_hit_shell_two(body: &str, mask: areka_emo_atlas::DecodedImage) -> (EmoWorld, AtlasTable) {
    let dir = temp_path_kit::TempPath::new("presenter-hit-two-faces");
    std::fs::write(
        dir.path().join("surfaces.txt"),
        format!("charset,UTF-8\nsurface0\n{{\n{body}}}\n"),
    )
    .expect("surfaces.txt 作成");
    std::fs::write(
        dir.path().join("descript.txt"),
        "charset,UTF-8\nseriko.use_self_alpha,1\n",
    )
    .expect("descript.txt 作成");
    let mut dec = areka_emo_atlas::MemoryDecoder::new();
    for (i, salt) in [(0, 0x15), (1, 0x65)] {
        let name = format!("surface{i}.png");
        std::fs::File::create(dir.path().join(&name)).expect("面の画像作成");
        dec.insert(
            dir.path().join(&name),
            16,
            16,
            64,
            gradient(16, 16, salt),
            true,
        );
    }
    for name in ["ma.png", "mb.png"] {
        dec.insert_unmultiplied(dir.path().join(name), mask.clone());
    }
    let mut target =
        crate::shell_target::load_shell_target(dir.path(), &dec).expect("シェルは読める");
    (target.take_world(), target.atlas().clone())
}

#[test]
fn switching_surfaces_names_the_new_one_and_keeps_mask_pictures_until_a_device_change() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, two_region_faces(), 0, 96);
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("A"));

    show_binds(&mut presenter, &mut world, 1, 1);
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("B"));
    let kinds = mask_pictures_created(&presenter, T);
    assert_eq!(kinds, 2, "マスクは 2 種類");

    for round in 0..6 {
        show_binds(&mut presenter, &mut world, 0, 2 + round);
        assert_eq!(
            name_at(&mut world, &presenter, T, 2, 2),
            Some("A"),
            "round {round}"
        );
        show_binds(&mut presenter, &mut world, 1, 2 + round);
        assert_eq!(
            name_at(&mut world, &presenter, T, 2, 2),
            Some("B"),
            "round {round}"
        );
    }
    assert_eq!(mask_pictures_created(&presenter, T), kinds);

    presenter.refresh_after_device_change(&mut world);
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore::new 失敗"));
    presenter.refresh_after_device_change(&mut world);
    // 描画面は装置の作り直しを追う回の次の回に新しい装置で描き直される。
    paint_screen(&mut world);
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("B"));
    let after_follow = mask_pictures_created(&presenter, T);
    assert_eq!(
        after_follow, 1,
        "追従で捨てた後、表示中の面のぶんだけ作り直された"
    );
    show_binds(&mut presenter, &mut world, 0, 30);
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("A"));
    assert_eq!(mask_pictures_created(&presenter, T), kinds);
}

/// 面 0 は左半分が `A`・面 1 は右半分が `B`。面 0 の動き 10 は `base` で面 1 を土台にする。
fn base_replacing_faces() -> (EmoWorld, AtlasTable) {
    let body = [
        "collisionex0,A,rect,0,0,8,16",
        "animation10.interval,never",
        "animation10.pattern0,base,1,0,0,0",
        "}",
        "surface1",
        "{",
        "collisionex0,B,rect,8,0,16,16",
        "",
    ]
    .join("\n");
    load_hit_shell_two(&body, straight_image(1, 1, |_, _| [0, 0, 0, 255]))
}

#[test]
fn a_base_frame_moves_the_regions_and_the_picture_to_the_replacing_surface_and_back() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, base_replacing_faces(), 0, 96);
    let picture = |p: &EmoPresenter| {
        let pic = p.targets[&T].hit_picture.as_ref().expect("描き先がある");
        (pic.surface_id, pic.base_surface)
    };
    assert_eq!(picture(&presenter), (0, 0));
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("A"));
    assert_eq!(name_at(&mut world, &presenter, T, 12, 2), None);

    let mut based = PatternState::default();
    based.set(
        10,
        PatternFrame {
            surface_id: 1,
            method: ComposeMethod::Base,
            x: 0,
            y: 0,
        },
    );
    show_with_ok(&mut presenter, &mut world, T, 0, BindSet::default(), based);
    assert_eq!(presenter.current_surface_id(T), Some(0));
    assert_eq!(
        picture(&presenter),
        (0, 1),
        "描き先は置き換え先の面の領域で塗る"
    );
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), None);
    assert_eq!(name_at(&mut world, &presenter, T, 12, 2), Some("B"));

    show_with_ok(
        &mut presenter,
        &mut world,
        T,
        0,
        BindSet::default(),
        PatternState::default(),
    );
    assert_eq!(picture(&presenter), (0, 0), "コマが終われば元の面へ戻る");
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("A"));
    assert_eq!(name_at(&mut world, &presenter, T, 12, 2), None);
}

fn problems(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN || e.level == tracing::Level::ERROR)
        .count()
}

fn warns_containing(events: &[CapturedEvent], text: &str) -> usize {
    events
        .iter()
        .filter(|e| {
            e.level == tracing::Level::WARN && e.field("message").is_some_and(|m| m.contains(text))
        })
        .count()
}

fn faint_five_shapes() -> (EmoWorld, AtlasTable) {
    let mask = straight_image(16, 16, |x, _| {
        if x < 8 {
            [0, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        }
    });
    let alpha = |x: u32, _| match x {
        20 => 127,
        21 => 128,
        _ => 255,
    };
    load_hit_shell(
        FIVE_SHAPES,
        (80, 16, gradient_with_alpha(80, 16, 0x15, alpha)),
        vec![(FIVE_SHAPES_MASK, mask)],
    )
}

#[test]
fn the_color_picture_alpha_decides_inside_a_shape() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, faint_five_shapes(), 0, 96);

    assert_eq!(name_at(&mut world, &presenter, T, 19, 4), Some("B"));
    assert_eq!(
        name_at(&mut world, &presenter, T, 20, 4),
        None,
        "α 127 は 0.5 未満"
    );
    assert_eq!(
        name_at(&mut world, &presenter, T, 21, 4),
        Some("B"),
        "α 128 は 0.5 以上"
    );
    assert_eq!(name_at(&mut world, &presenter, T, 22, 4), Some("B"));
}

#[test]
fn outside_the_face_and_nothing_shown_hit_nothing_and_record_nothing() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, five_shapes_assets(), 0, 96);
    let (names, events) = capture(|| {
        [(-1, 4), (4, -1), (80, 4), (4, 16), (1000, 1000)]
            .map(|(x, y)| name_at(&mut world, &presenter, T, x, y))
    });
    assert_eq!(names, [None; 5]);
    assert_eq!(problems(&events), 0, "{events:?}");

    let mut idle = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = five_shapes_assets();
    idle.attach_target(T, window, emo_world, atlas, 96);
    let (name, events) = capture(|| name_at(&mut world, &idle, T, 2, 2).map(str::to_owned));
    assert_eq!(name, None);
    assert_eq!(problems(&events), 0, "{events:?}");
}

#[test]
fn a_discarded_target_warns_once_and_the_next_commit_brings_the_name_back() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, five_shapes_assets(), 0, 96);
    presenter.targets.get_mut(&T).unwrap().hit_picture = None;

    let (names, events) = capture(|| {
        [(2, 2), (2, 2), (20, 4)]
            .map(|(x, y)| name_at(&mut world, &presenter, T, x, y).map(str::to_owned))
    });
    assert_eq!(names, [None, None, None]);
    assert_eq!(warn_count(&events), 1, "{events:?}");
    assert_eq!(problems(&events), 1, "{events:?}");

    show_binds(&mut presenter, &mut world, 0, 1);
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("R"));
    assert_eq!(name_at(&mut world, &presenter, T, 20, 4), Some("B"));
}

#[test]
fn a_device_change_follow_up_repaints_and_the_answer_returns() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, five_shapes_assets(), 0, 96);
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("R"));

    presenter.refresh_after_device_change(&mut world);
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore::new 失敗"));
    presenter.refresh_after_device_change(&mut world);
    // 描画面は装置の作り直しを追う回の次の回に新しい装置で描き直される。
    paint_screen(&mut world);
    assert_eq!(hit_generation(&presenter, T), Some(1), "塗り直された");
    assert_eq!(name_at(&mut world, &presenter, T, 2, 2), Some("R"));
    assert_eq!(name_at(&mut world, &presenter, T, 20, 4), Some("B"));
}

#[test]
fn one_over_the_bound_warns_once_and_drops_the_head_side() {
    let count = crate::hit_shapes::HIT_SHAPE_LIMIT + 1;
    let rect = |index: usize, name: &str, (left, right): (i64, i64)| Collision {
        index: index as u32,
        shape: CollisionShape::Rect(CollisionBounds {
            left,
            top: 0,
            right,
            bottom: 16,
        }),
        name: CollisionName::new(name.to_string()),
    };
    let mut collisions = vec![rect(0, "head", (0, 16))];
    collisions.push(rect(1, "second", (16, 24)));
    collisions.extend((2..count - 1).map(|i| rect(i, "far", (100, 101))));
    collisions.push(rect(count - 1, "front", (8, 16)));
    let (emo_world, atlas) = bake_world(
        shell_of(vec![Surface {
            targets: vec![AppendTarget::Single(0)],
            elements: vec![elem("a.png", 0, 0)],
            collisions,
            animations: Vec::new(),
        }]),
        vec![("a.png", 80, 16, gradient(80, 16, 0x15))],
    );
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    presenter.attach_target(T, window, emo_world, atlas, 96);
    let ((), events) = capture(|| show_ok(&mut presenter, &mut world, T, 0));
    assert_eq!(warn_count(&events), 1, "{events:?}");
    assert_eq!(warns_containing(&events, "上限"), 1, "{events:?}");

    assert_eq!(name_at(&mut world, &presenter, T, 10, 4), Some("front"));
    assert_eq!(name_at(&mut world, &presenter, T, 20, 4), Some("second"));
    assert_eq!(
        name_at(&mut world, &presenter, T, 2, 2),
        None,
        "越えた先頭の側は当たらない"
    );

    let ((), again) = capture(|| show_ok(&mut presenter, &mut world, T, 0));
    assert_eq!(warn_count(&again), 0, "{again:?}");
}
