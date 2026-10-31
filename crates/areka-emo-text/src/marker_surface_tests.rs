//! 印の面（スクロールの矢印）の檻（balloon-scroll-arrows task 4.2・要件 3.4・3.5・5.7・11.1）。
//!
//! 実 GPU（`com_world` の `GraphicsCore`・WUC の Compositor）で面を作り、窓の `Children` の並び・
//! 描いた画素の読み戻し・拡大率 1.5 の位置と大きさ・当たりのマスク・押下の届き方を見る。

use std::sync::Arc;

use areka_emo_present::balloon_marker::MarkerImage;
use areka_sakura::contract::ActorKey;
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use windows_numerics::Matrix3x2;
use wintf::ecs::pointer::build_bubble_path;
use wintf::ecs::widget::bitmap_source::AlphaMask;
use wintf::ecs::{
    AlphaMaskResource, GlobalArrangement, GraphicsCore, HitTest, Point, PointF, Rect, SizeI,
    WindowPos, WucGraphicsResource, hit_test_in_window,
};

use super::super::box_sync_tests::{emote, layout, resolver, select, text};
use super::super::test_support::{com_world, geo_model};
use super::super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use super::*;
use crate::draw::DEFAULT_BALLOON_BACKGROUND;
use crate::place::{PlaceKey, TextPlace};
use crate::state::TextLayerConfig;

/// 全部の文字が見えている時刻。
const LATE: f64 = 10.0;

/// 塗りつぶしの絵（乗算済み BGRA・α は `alpha`・色は白）。
fn solid(w: u32, h: u32, alpha: u8) -> Arc<MarkerImage> {
    Arc::new(MarkerImage {
        width: w,
        height: h,
        bgra: [alpha, alpha, alpha, alpha].repeat((w * h) as usize),
    })
}

fn placed(kind: MarkerKind, at: (i32, i32), image: &Arc<MarkerImage>) -> PlacedMarker {
    PlacedMarker {
        kind,
        at,
        image: Arc::clone(image),
    }
}

fn children_of(world: &World, window: Entity) -> Vec<Entity> {
    world
        .get::<Children>(window)
        .map(|c| c.iter().copied().collect())
        .unwrap_or_default()
}

/// World の資源（GraphicsCore・Compositor）で印の面を挿す（`present_frame` と同じ取り出し方）。
fn attach(
    world: &mut World,
    window: Entity,
    index: usize,
    size: (u32, u32),
    offset: (f32, f32),
) -> MarkerSurface {
    let compositor = world
        .resource::<WucGraphicsResource>()
        .compositor()
        .cloned()
        .expect("Compositor");
    world.resource_scope(|world, core: bevy_ecs::world::Mut<GraphicsCore>| {
        MarkerSurface::attach(world, window, index, &compositor, &core, size, offset)
            .expect("MarkerSurface::attach")
    })
}

fn mask_of(world: &World, surface: &MarkerSurface) -> AlphaMask {
    let entity = surface.window_child().expect("窓の子");
    world
        .get::<AlphaMaskResource>(entity)
        .and_then(AlphaMaskResource::mask)
        .expect("面の大きさのマスクを持つ")
        .clone()
}

fn alpha_at(bytes: &[u8], width: u32, x: u32, y: u32) -> u8 {
    bytes[((y * width + x) * 4 + 3) as usize]
}

/// 普通のバルーン: 印の面は差し込み口（文字の面）の直前に挿さり、窓の直接の子・マスクの当たり判定・
/// 面の大きさの空のマスク（何も受けない）を持つ。片付けると窓の子の並びから外れる。
#[test]
fn balloon_marker_is_inserted_right_before_the_slot_with_an_empty_mask() {
    let (mut world, window, slot) = com_world();
    let picture = world.spawn(ChildOf(window)).id();
    world.flush();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let actor = ActorKey::from("0");
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, (200, 100), (200, 100)),
        ResolvedBalloonText::resolve_with_background(
            &geo_model(),
            (200, 100),
            DEFAULT_BALLOON_BACKGROUND,
        ),
    );
    let place = PlaceKey::balloon(&actor);
    let index = rt
        .marker_child_index(&children_of(&world, window), &place)
        .expect("差し込み口の位置");
    assert_eq!(index, 0);

    let markers = attach(&mut world, window, index, (200, 100), (0.0, 0.0));
    let child = markers.window_child().expect("窓の子");
    assert_eq!(
        children_of(&world, window),
        vec![child, slot, picture],
        "印の面 → 文字の面 → 絵（先頭ほど手前）"
    );
    assert_eq!(
        world.get::<ChildOf>(child).map(|c| c.parent()),
        Some(window)
    );
    assert_eq!(
        world.get::<Name>(child).map(|n| n.as_str()),
        Some("emo-text-marker-surface")
    );
    assert_eq!(
        world.get::<HitTest>(child).copied(),
        Some(HitTest::alpha_mask())
    );
    let mask = mask_of(&world, &markers);
    assert_eq!((mask.width(), mask.height()), (200, 100));
    assert!((0..100).all(|y| (0..200).all(|x| !mask.is_hit(x, y))));
    assert_eq!(markers.size(), (200, 100));

    markers.despawn(&mut world);
    assert_eq!(children_of(&world, window), vec![slot, picture]);
    assert!(world.get_entity(child).is_err());
}

/// 箱が複数: 印の面は各箱の文字の面の直前に挿さり、element番号の大きい順（b → a）が崩れない。
/// 箱の面を作り直す挿し位置は、手前の箱の文字の面と印の面の両方を数える。
#[test]
fn box_markers_keep_the_element_order_and_box_child_index_counts_them() {
    let (mut world, window, slot) = com_world();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let layout = layout();
    rt.set_box_layout(&mut world, layout.clone(), resolver(), vec![]);
    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&text("0", "あ"));
    rt.apply_cue(&select("0", "b"));
    rt.apply_cue(&text("0", "い"));
    let shell = TextSlotBinding::new(slot, window, 1.0, (200, 300), (200, 300));
    rt.sync_box_bindings(&mut world, &[(ActorKey::from("0"), Some((shell, 0)))]);
    present_frame(&mut rt, &mut world, LATE).expect("提示フレーム");

    let key = |name: &str| PlaceKey {
        actor: ActorKey::from("0"),
        place: TextPlace::Box(
            layout
                .placements(0)
                .iter()
                .find(|p| p.name.as_str() == name)
                .expect("箱")
                .name
                .clone(),
        ),
    };
    let (a, b) = (key("a"), key("b"));
    let text_of = |rt: &TextLayerRuntime, k: &PlaceKey| {
        rt.surfaces[k].surface.window_child().expect("箱の文字の面")
    };
    let (a_text, b_text) = (text_of(&rt, &a), text_of(&rt, &b));
    assert_eq!(children_of(&world, window), vec![slot, b_text, a_text]);

    let mut markers = Vec::new();
    for k in [&b, &a] {
        let index = rt
            .marker_child_index(&children_of(&world, window), k)
            .expect("箱の文字の面の位置");
        let m = attach(&mut world, window, index, (10, 10), (0.0, 0.0));
        markers.push(m.window_child().expect("窓の子"));
        rt.surfaces.get_mut(k).expect("箱の面").markers = Some(m);
    }
    let (b_mark, a_mark) = (markers[0], markers[1]);
    assert_eq!(
        children_of(&world, window),
        vec![slot, b_mark, b_text, a_mark, a_text],
        "箱ごとに印の面 → 文字の面、element番号の大きい順"
    );

    // 箱 a の面を外して作り直す: 挿し位置は b の文字の面と印の面の後ろ（3）。
    let render = rt.surfaces.remove(&a).expect("a の面");
    render.markers.expect("a の印").despawn(&mut world);
    render.surface.despawn_window_child(&mut world);
    let children = children_of(&world, window);
    assert_eq!(children, vec![slot, b_mark, b_text]);
    assert_eq!(rt.box_child_index(&children, &a), Some(3));
    present_frame(&mut rt, &mut world, LATE).expect("作り直しの提示");
    let a_text = text_of(&rt, &a);
    assert_eq!(
        children_of(&world, window),
        vec![slot, b_mark, b_text, a_text]
    );
}

/// 描いた矢印の画素が読み戻しに出て、マスクは出ている矢印の α ≥ 0.5 の画素だけを持つ（拡大率 1）。
/// 出す矢印を空にして描き直すと、面は透明・マスクは空に戻る。
#[test]
fn drawn_arrow_shows_in_read_back_and_mask_holds_only_shown_opaque_pixels() {
    let (mut world, window, _slot) = com_world();
    let mut markers = attach(&mut world, window, 0, (40, 30), (0.0, 0.0));
    // 左の 1 列だけ α 100（< 0.5）、残りは α 200（≥ 0.5）。
    let mut bgra = [200u8; 4].repeat(4 * 3);
    for y in 0..3 {
        bgra[y * 16..y * 16 + 4].copy_from_slice(&[100; 4]);
    }
    let up = Arc::new(MarkerImage {
        width: 4,
        height: 3,
        bgra,
    });
    let scale = ScaleContract::new(1.0, None);
    markers
        .draw(&mut world, &[placed(MarkerKind::Up, (2, 5), &up)], &scale)
        .expect("描く");

    let bytes = markers.read_back().expect("読み戻し");
    assert_eq!(alpha_at(&bytes, 40, 3, 6), 200, "矢印の画素が出る");
    assert_eq!(alpha_at(&bytes, 40, 2, 5), 100);
    assert_eq!(alpha_at(&bytes, 40, 1, 5), 0, "矢印の外は透明");
    assert_eq!(
        alpha_at(&bytes, 40, 30, 20),
        0,
        "出ていない下の矢印の所は透明"
    );

    let mask = mask_of(&world, &markers);
    assert!(mask.is_hit(3, 6) && mask.is_hit(5, 7));
    assert!(!mask.is_hit(2, 6), "α < 0.5 の画素は当たらない");
    assert!(!mask.is_hit(6, 6) && !mask.is_hit(3, 8), "矢印の外");
    let hits = (0..30)
        .flat_map(|y| (0..40).map(move |x| (x, y)))
        .filter(|&(x, y)| mask.is_hit(x, y))
        .count();
    assert_eq!(hits, 3 * 3, "出ている矢印の不透明な画素だけ");

    markers.draw(&mut world, &[], &scale).expect("空で描く");
    assert!(
        markers
            .read_back()
            .expect("読み戻し")
            .iter()
            .skip(3)
            .step_by(4)
            .all(|&a| a == 0),
        "空で描き直すと面は透明"
    );
    let mask = mask_of(&world, &markers);
    assert_eq!((mask.width(), mask.height()), (40, 30));
    assert!((0..30).all(|y| (0..40).all(|x| !mask.is_hit(x, y))));
}

/// 拡大率 1.5: 左上 (10, 6)・4×4 の矢印は物理 px の [15, 21) × [9, 15) に描かれ、当たりも同じ所。
/// 2 つの矢印は両方とも出る（面の外へはみ出す分は面の端で切れる）。
#[test]
fn scale_one_and_a_half_places_and_sizes_the_arrows() {
    let (mut world, window, _slot) = com_world();
    let mut markers = attach(&mut world, window, 0, (60, 40), (0.0, 0.0));
    let image = solid(4, 4, 255);
    let scale = ScaleContract::new(1.5, None);
    markers
        .draw(
            &mut world,
            &[
                placed(MarkerKind::Up, (10, 6), &image),
                placed(MarkerKind::Down, (38, 24), &image),
            ],
            &scale,
        )
        .expect("描く");

    let bytes = markers.read_back().expect("読み戻し");
    for (x, y) in [(15, 9), (20, 14), (17, 12)] {
        assert_eq!(alpha_at(&bytes, 60, x, y), 255, "({x},{y}) は矢印の内");
    }
    for (x, y) in [(14, 12), (21, 12), (17, 8), (17, 15)] {
        assert_eq!(alpha_at(&bytes, 60, x, y), 0, "({x},{y}) は矢印の外");
    }
    // 下の矢印は [57, 63) × [36, 42) で、面（60×40）の端で切れる。
    assert_eq!(alpha_at(&bytes, 60, 59, 39), 255);
    assert_eq!(alpha_at(&bytes, 60, 56, 37), 0);

    let mask = mask_of(&world, &markers);
    let hits: Vec<(u32, u32)> = (0..40)
        .flat_map(|y| (0..60).map(move |x| (x, y)))
        .filter(|&(x, y)| mask.is_hit(x, y))
        .collect();
    let expected: Vec<(u32, u32)> = (0..40)
        .flat_map(|y| (0..60).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            ((15..21).contains(&x) && (9..15).contains(&y))
                || ((57..60).contains(&x) && (36..40).contains(&y))
        })
        .collect();
    assert_eq!(hits, expected, "当たりは描いた所と同じ（面の端で切れる）");
}

/// 押下の届き方（6.2・6.3 の受け口の前提）: 印の面は窓の直接の子なので、泡立ちの道は「送り手 → 窓」で、
/// 押下は窓の `OnPointerPressed`（普通のバルーンはバルーンの窓・箱はシェルの窓）が受ける。wintf は
/// 子を末尾から当てるので、矢印の下の絵が不透明ならその絵が送り手になり、絵が透けていれば印の面が
/// 送り手になる。矢印の透けた画素は（絵も透けていれば）何も受けない。
#[test]
fn press_on_the_marker_mask_bubbles_to_the_owning_window() {
    let (mut world, _w, _slot) = com_world();
    const WINDOW: (u32, u32) = (100, 50);
    let arranged = |left: f32, top: f32, size: (u32, u32)| GlobalArrangement {
        transform: Matrix3x2::translation(left, top),
        bounds: Rect {
            left,
            top,
            right: left + size.0 as f32,
            bottom: top + size.1 as f32,
        },
    };
    let window = world
        .spawn((
            arranged(0.0, 0.0, WINDOW),
            HitTest::none(),
            WindowPos {
                position: Some(Point { x: 0, y: 0 }),
                size: Some(SizeI {
                    width: WINDOW.0 as i32,
                    height: WINDOW.1 as i32,
                }),
                ..Default::default()
            },
        ))
        .id();
    // 絵: x ≥ 50 が不透明。製品と同じく文字の面と印の面より後ろ（末尾）に並ぶ。
    let mut px = vec![0u8; (WINDOW.0 * WINDOW.1 * 4) as usize];
    for y in 0..WINDOW.1 {
        for x in 50..WINDOW.0 {
            px[((y * WINDOW.0 + x) * 4 + 3) as usize] = 255;
        }
    }
    let mut picture_mask = AlphaMaskResource::new();
    picture_mask.set(AlphaMask::from_pbgra32(
        &px,
        WINDOW.0,
        WINDOW.1,
        WINDOW.0 * 4,
    ));
    let picture = world
        .spawn((
            arranged(0.0, 0.0, WINDOW),
            HitTest::alpha_mask(),
            picture_mask,
            ChildOf(window),
        ))
        .id();
    world.flush();

    let mut markers = attach(&mut world, window, 0, WINDOW, (0.0, 0.0));
    let marker = markers.window_child().expect("窓の子");
    world.entity_mut(marker).insert(arranged(0.0, 0.0, WINDOW));
    assert_eq!(children_of(&world, window), vec![marker, picture]);
    // 左の矢印は透けた絵の上、右の矢印は不透明な絵の上。左の矢印の 1 列目は透明（α 0）。
    let mut bgra = [255u8; 4].repeat(4 * 4);
    for y in 0..4 {
        bgra[y * 16..y * 16 + 4].copy_from_slice(&[0; 4]);
    }
    let left = Arc::new(MarkerImage {
        width: 4,
        height: 4,
        bgra,
    });
    markers
        .draw(
            &mut world,
            &[
                placed(MarkerKind::Up, (10, 10), &left),
                placed(MarkerKind::Down, (70, 10), &solid(4, 4, 255)),
            ],
            &ScaleContract::new(1.0, None),
        )
        .expect("描く");

    let hit = |x: f32, y: f32| hit_test_in_window(&world, window, PointF::new(x, y));
    assert_eq!(
        hit(12.0, 12.0),
        Some(marker),
        "透けた絵の上の矢印は印の面が受ける"
    );
    assert_eq!(hit(10.0, 12.0), None, "矢印の透けた画素は通す");
    assert_eq!(
        hit(72.0, 12.0),
        Some(picture),
        "不透明な絵の上では末尾の絵が先に当たる（送り手は絵）"
    );
    assert_eq!(
        build_bubble_path(&world, marker),
        vec![marker, window],
        "印の面の押下は持ち主の窓へ泡立つ"
    );
    assert_eq!(build_bubble_path(&world, picture), vec![picture, window]);
}
