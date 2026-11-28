//! クリックの透過と名前の解決が、同じ入れ物の 1 画素を読むことの実 GPU の檻。

use std::path::Path;

use areka_emo_atlas::{
    AlphaParams, AtlasTable, MemoryDecoder, PackConfig, SetId, SurfaceSet, UseSelfAlpha, bake,
};
use areka_emo_compose::{ElementKind, EmoWorld, element_kind};
use areka_parsers::shell::parse;
use bevy_ecs::world::World;
use wintf::ecs::{GraphicsCore, hit_test_entity};

use super::test_support::{
    attach_and_show, capture, make_world_with_gpu, mount_entities, paint_screen, screen_point,
};
use super::{EmoPresenter, TargetId};

const T: TargetId = TargetId(0);
const W: u32 = 30;
const H: u32 = 10;
const OPAQUE: [u8; 4] = [0, 0, 255, 255];
const CLEAR: [u8; 4] = [0; 4];

fn in_middle(x: u32) -> bool {
    (10..20).contains(&x)
}

fn picture(pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .flat_map(|(x, y)| pixel(x, y))
        .collect()
}

/// 面 0 の定義 `elements` と全域の名前 `N` を持つ面を、`(名前, 絵, α を持つか)` の絵で焼いて表示する。
fn shown(elements: &str, pictures: Vec<(&str, Vec<u8>, bool)>) -> (World, EmoPresenter) {
    let shell = parse(&format!(
        "charset,UTF-8\nsurface0\n{{\n{elements}collisionex0,N,rect,0,0,{W},{H}\n}}\n"
    ));
    let base = Path::new("shell/master");
    let mut dec = MemoryDecoder::new();
    for (name, bgra, has_alpha) in pictures {
        dec.insert(base.join(name), W, H, W * 4, bgra, has_alpha);
    }
    let mut surfaces = shell.surfaces.clone();
    for s in &mut surfaces {
        s.elements
            .retain(|e| element_kind(&e.path) == ElementKind::Image);
    }
    let set = SurfaceSet {
        surfaces: &surfaces,
        base_dir: base,
        alpha_params: AlphaParams {
            use_self_alpha: UseSelfAlpha::On,
        },
    };
    let baked = bake(&[set], &dec, PackConfig::default());
    assert!(baked.errors.is_empty(), "{:?}", baked.errors);
    let mut emo = EmoWorld::build(&shell);
    emo.bind_atlas(&baked.table, SetId(0));
    let assets: (EmoWorld, AtlasTable) = (emo, baked.table);

    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, assets, 0, 96);
    paint_screen(&mut world);
    (world, presenter)
}

/// 行 5 の全画素の（クリックが当たるか, 名前）。
fn sweep(world: &World, presenter: &EmoPresenter) -> Vec<(bool, Option<String>)> {
    let surface = mount_entities(presenter, T).0;
    (0..W)
        .map(|x| {
            (
                hit_test_entity(world, surface, screen_point(world, surface, x, 5)),
                presenter
                    .hit_region_client(world, T, x as i64, 5)
                    .region
                    .map(String::from),
            )
        })
        .collect()
}

/// 真ん中だけが透けた面では、そこだけクリックが抜け名前も無く、残りは両方が当たる。
fn assert_middle_is_see_through(what: &str, world: &World, presenter: &EmoPresenter) {
    let got = sweep(world, presenter);
    for (x, (clicked, name)) in got.iter().enumerate() {
        let inside = !in_middle(x as u32);
        assert_eq!(*clicked, inside, "{what}: x={x} のクリックの当たり");
        assert_eq!(
            name.as_deref(),
            inside.then_some("N"),
            "{what}: x={x} の名前"
        );
    }
}

#[test]
fn reduced_replaced_and_keyed_out_points_are_transparent_to_both_paths() {
    let base = || picture(|_, _| OPAQUE);
    let hole = || picture(|x, _| if in_middle(x) { CLEAR } else { OPAQUE });

    let (world, presenter) = shown(
        "element0,overlay,base.png,0,0\nelement1,reduce,hole.png,0,0\n",
        vec![("base.png", base(), true), ("hole.png", hole(), true)],
    );
    assert_middle_is_see_through("reduce", &world, &presenter);

    let (world, presenter) = shown(
        "element0,overlay,base.png,0,0\nelement1,replace,hole.png,0,0\n",
        vec![("base.png", base(), true), ("hole.png", hole(), true)],
    );
    assert_middle_is_see_through("replace", &world, &presenter);

    // 透過色は左上の画素の色。左上と真ん中を同じ色にし、それ以外は別の不透明色にする。
    let key = picture(|x, y| {
        if (x, y) == (0, 0) || in_middle(x) {
            [255, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        }
    });
    let (world, presenter) = shown(
        "element0,overlay,key.png,0,0\n",
        vec![("key.png", key, false)],
    );
    assert_middle_is_see_through("透過色", &world, &presenter);
}

#[test]
fn opaque_points_hit_on_both_paths() {
    let (world, presenter) = shown(
        "element0,overlay,base.png,0,0\n",
        vec![("base.png", picture(|_, _| OPAQUE), true)],
    );
    assert!(
        sweep(&world, &presenter)
            .iter()
            .all(|(clicked, name)| *clicked && name.as_deref() == Some("N"))
    );
}

/// 入れ物が読めない間は、クリックの透過も名前の解決も当たり無しで、`error!` は 2 つ合わせて 1 件。
#[test]
fn unreadable_surface_misses_both_paths_with_one_error() {
    let (mut world, presenter) = shown(
        "element0,overlay,base.png,0,0\n",
        vec![("base.png", picture(|_, _| OPAQUE), true)],
    );
    let core = world
        .remove_resource::<GraphicsCore>()
        .expect("前提: GraphicsCore が載っている");
    let (got, events) = capture(|| sweep(&world, &presenter));
    world.insert_resource(core);

    assert!(got.iter().all(|p| *p == (false, None)), "{got:?}");
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .map(|e| e.message())
        .collect();
    assert_eq!(
        errors,
        ["[hit_test] surface alpha unreadable, no hit"],
        "{events:?}"
    );
}
