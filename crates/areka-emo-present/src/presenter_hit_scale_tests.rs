//! 表示の拡大率が 1 でないときの当たりの実 GPU のテスト。

use areka_emo_atlas::AtlasTable;
use areka_emo_compose::EmoWorld;
use bevy_ecs::world::World;

use super::test_support::{
    attach_and_show_hit_target, five_shapes_assets, gradient, load_hit_shell, make_world_with_gpu,
    name_at, show_ok, spawn_window_with_dpi,
};
use super::{EmoPresenter, ScaleRatio, TargetId};

const T: TargetId = TargetId(0);

/// (窓の DPI, 作者の DPI, 分子, 分母)。比は窓の DPI ÷ 作者の DPI。
const SCALES: [(u16, u16, u32, u32); 3] = [(192, 96, 2, 1), (120, 96, 5, 4), (132, 120, 11, 10)];

fn shown_at(assets: (EmoWorld, AtlasTable), dpi: u16, author_dpi: u16) -> (World, EmoPresenter) {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, dpi);
    let (emo_world, atlas) = assets;
    presenter.attach_target(T, window, emo_world, atlas, author_dpi);
    show_ok(&mut presenter, &mut world, T, 0);
    (world, presenter)
}

fn rect_edges() -> (EmoWorld, AtlasTable) {
    load_hit_shell(
        "collisionex0,Rc,rect,10,10,20,20\n",
        (40, 40, gradient(40, 40, 0x15)),
        Vec::new(),
    )
}

/// サーフェスの 1 画素 `s` に画素の中心が入る画面の画素の範囲。
fn client_span(s: i64, num: u32, den: u32) -> std::ops::Range<i64> {
    let (num, den) = (i64::from(num), i64::from(den));
    let covers = |v: i64| (2 * v + 1) * den >= 2 * s * num;
    let first = (0..).find(|&v| covers(v)).expect("範囲は有限");
    let end = (first..)
        .find(|&v| (2 * v + 1) * den >= 2 * (s + 1) * num)
        .expect("範囲は有限");
    first..end
}

fn walk_names(
    presenter: &EmoPresenter,
    (w, h): (i64, i64),
    (num, den): (u32, u32),
) -> Vec<Option<String>> {
    let mut names = Vec::new();
    for sy in 0..h {
        for sx in 0..w {
            let (xs, ys) = (client_span(sx, num, den), client_span(sy, num, den));
            assert!(!xs.is_empty() && !ys.is_empty(), "({sx},{sy})");
            let first = presenter.hit_region_client(T, xs.start, ys.start);
            let want = first.region.map(str::to_owned);
            for cy in ys.clone() {
                for cx in xs.clone() {
                    let hit = presenter.hit_region_client(T, cx, cy);
                    assert_eq!(hit.surface_point, (sx, sy), "client ({cx},{cy})");
                    assert_eq!(
                        hit.region.map(str::to_owned),
                        want,
                        "surface ({sx},{sy}) client ({cx},{cy})"
                    );
                }
            }
            names.push(want);
        }
    }
    names
}

fn identity_names(assets: (EmoWorld, AtlasTable), size: (i64, i64)) -> Vec<Option<String>> {
    let (_world, presenter) = shown_at(assets, 96, 96);
    let mut names = Vec::new();
    for y in 0..size.1 {
        for x in 0..size.0 {
            names.push(name_at(&presenter, T, x, y).map(str::to_owned));
        }
    }
    names
}

#[test]
fn every_screen_pixel_of_a_surface_pixel_names_the_same_shape() {
    let want = identity_names(five_shapes_assets(), (80, 16));
    for name in ["R", "B", "E", "C", "P"] {
        assert!(want.iter().any(|n| n.as_deref() == Some(name)), "{name}");
    }
    for (dpi, author, num, den) in SCALES {
        let (_world, presenter) = shown_at(five_shapes_assets(), dpi, author);
        assert_eq!(presenter.applied_ratio(T), ScaleRatio::new(num, den));
        assert_eq!(
            walk_names(&presenter, (80, 16), (num, den)),
            want,
            "scale {num}/{den}"
        );
    }
}

#[test]
fn rect_edge_stays_left_top_inclusive_right_bottom_exclusive_at_every_scale() {
    let edge_cases = [
        ((10, 15), Some("Rc")),
        ((9, 15), None),
        ((19, 15), Some("Rc")),
        ((20, 15), None),
        ((15, 10), Some("Rc")),
        ((15, 9), None),
        ((15, 19), Some("Rc")),
        ((15, 20), None),
    ];
    for (dpi, author, num, den) in [(96, 96, 1, 1)].into_iter().chain(SCALES) {
        let (_world, presenter) = shown_at(rect_edges(), dpi, author);
        let names = walk_names(&presenter, (40, 40), (num, den));
        for ((sx, sy), want) in edge_cases {
            assert_eq!(
                names[(sy * 40 + sx) as usize].as_deref(),
                want,
                "scale {num}/{den} surface ({sx},{sy})"
            );
        }
    }
}

#[test]
fn visible_surface_hit_uses_applied_scale_at_k2() {
    let mut world = make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach_and_show_hit_target(&mut presenter, &mut world, T, 192);
    assert_eq!(presenter.current_surface_id(T), Some(1000));
    assert_eq!(presenter.applied_ratio(T), ScaleRatio::new(2, 1));

    for (client, want_region, want_point) in [
        ((360, 192), Some("Head"), (180, 96)),
        ((420, 620), Some("Arm"), (210, 310)),
        ((210, 310), None, (105, 155)),
    ] {
        let hit = presenter.hit_region_client(T, client.0, client.1);
        assert_eq!(hit.surface_point, want_point, "client {client:?}");
        assert_eq!(hit.region, want_region, "client {client:?}");
    }
}
