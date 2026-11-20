//! 描き直さない時点とマスクの控えの実 GPU のテスト。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::{BindSet, EmoWorld};
use wintf::ecs::GraphicsCore;

use super::test_support::{
    attach_and_show, build_balloon, five_shapes_assets, frames, gradient, hit_generation,
    load_hit_shell, make_world_with_gpu, mask_pictures_created, name_at, set_window_dpi,
    show_with_ok, spawn_window_with_dpi, straight_image,
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
        assert_eq!(name_at(&presenter, T, 2, 2), Some("R"));
        assert_eq!(name_at(&presenter, T, 20, 4), Some("B"));
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
    assert_eq!(name_at(&presenter, T, 4, 0), Some("Body"));

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
        assert_eq!(name_at(&presenter, T, 4, 0), Some("Body"), "step {i}");
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
    assert_eq!(name_at(&presenter, T, 2, 2), Some("A"));

    show_binds(&mut presenter, &mut world, 1, 1);
    assert_eq!(name_at(&presenter, T, 2, 2), Some("B"));
    let kinds = mask_pictures_created(&presenter, T);
    assert_eq!(kinds, 2, "マスクは 2 種類");

    for round in 0..6 {
        show_binds(&mut presenter, &mut world, 0, 2 + round);
        assert_eq!(name_at(&presenter, T, 2, 2), Some("A"), "round {round}");
        show_binds(&mut presenter, &mut world, 1, 2 + round);
        assert_eq!(name_at(&presenter, T, 2, 2), Some("B"), "round {round}");
    }
    assert_eq!(mask_pictures_created(&presenter, T), kinds);

    presenter.refresh_after_device_change(&mut world);
    world.insert_resource(GraphicsCore::new().expect("GraphicsCore::new 失敗"));
    presenter.refresh_after_device_change(&mut world);
    assert_eq!(name_at(&presenter, T, 2, 2), Some("B"));
    let after_follow = mask_pictures_created(&presenter, T);
    assert_eq!(
        after_follow, 1,
        "追従で捨てた後、表示中の面のぶんだけ作り直された"
    );
    show_binds(&mut presenter, &mut world, 0, 30);
    assert_eq!(name_at(&presenter, T, 2, 2), Some("A"));
    assert_eq!(mask_pictures_created(&presenter, T), kinds);
}

/// 同じ寸法の面を巡回して表示し、暖機の後に α マスクの確保が増えるかを数える。
fn mask_allocs_after_warm_up(assets: (EmoWorld, AtlasTable), surface: u32) -> (u64, u64) {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas) = assets;
    presenter.attach_target(T, window, emo_world, atlas, 96);
    let warm = crate::cache::CAPACITY + 1;
    for i in 0..warm {
        show_binds(&mut presenter, &mut world, surface, 1 + i as u32);
    }
    let warmed = presenter.targets[&T].budget.cumulative().alloc_mask;
    for i in 0..8 {
        show_binds(&mut presenter, &mut world, surface, 1 + (warm + i) as u32);
    }
    let settled = presenter.targets[&T].budget.cumulative().alloc_mask;
    (warmed, settled)
}

#[test]
fn the_shown_mask_copy_does_not_add_mask_allocations() {
    let (warmed, settled) = mask_allocs_after_warm_up(five_shapes_assets(), 0);
    assert_eq!(settled, warmed, "暖機の後に α マスクの確保が増えた");

    let plain = load_hit_shell("", (80, 16, gradient(80, 16, 0x15)), Vec::new());
    assert_eq!(
        mask_allocs_after_warm_up(plain, 0),
        (warmed, settled),
        "当たり判定の有無で α マスクの確保の数が違う"
    );
}
