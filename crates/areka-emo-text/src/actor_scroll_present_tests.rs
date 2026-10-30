//! 毎フレームの矢印の出し分けと当たりの写しの檻（balloon-scroll-arrows task 4.3・要件 3.1〜3.4・
//! 3.8・5.7・7.3・8.6・11.1）。
//!
//! 実 GPU で提示し、矢印が出ているかを公開の当たりの口（`marker_at_point`）で読む: 押し出された行の
//! 有無が変わった次の提示で矢印が合うこと、モデルを替えると置き場が替わり古い印の面が窓の子から
//! 消えること、α < 0.5 の画素で当たらないこと。箱の足りない番号の埋め方は純粋な引き当てで見る。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use areka_emo_present::balloon_marker::{MarkerImage, MarkerPair};
use areka_parsers::balloon::{ArrowKeys, MarkerCoord, MarkerPoint};
use areka_sakura::contract::{ActorKey, CueCommand};
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::Children;
use bevy_ecs::prelude::World;
use wintf::ecs::WucGraphicsResource;

use super::box_sync_tests::{emote, layout, select, text};
use super::region_collapse_tests::model as parse_model;
use super::scroll::placed_markers;
use super::test_support::{com_world, cue, geo_model};
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding, present_frame};
use crate::place::{PlaceKey, TextPlace};
use crate::scroll_marker::{MarkerKind, MarkerShown, ResolvedMarkerPlacement, ScrollDir};
use crate::state::TextLayerConfig;

/// 小さいバルーン（既定の 12px の字で 2〜3 行しか入らない）。
const IMAGE: (u32, u32) = (120, 40);
/// 全部の字が見えている時刻。
const DONE: f64 = 100.0;

/// 塗りつぶしの絵（乗算済み BGRA）。
fn solid(w: u32, h: u32, alpha: u8) -> Arc<MarkerImage> {
    Arc::new(MarkerImage {
        width: w,
        height: h,
        bgra: [alpha; 4].repeat((w * h) as usize),
    })
}

/// 10×5 の矢印。左の 2 列だけ α 100（< 0.5）、残りは α 255。
fn arrow() -> Arc<MarkerImage> {
    let mut bgra = [255u8; 4].repeat(10 * 5);
    for y in 0..5 {
        bgra[y * 40..y * 40 + 8].copy_from_slice(&[100; 8]);
    }
    Arc::new(MarkerImage {
        width: 10,
        height: 5,
        bgra,
    })
}

fn pair() -> MarkerPair {
    MarkerPair {
        up: Some(arrow()),
        down: Some(arrow()),
    }
}

fn binding(slot: Entity, window: Entity) -> TextSlotBinding {
    TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE)
}

/// `lines` 行の台本（改行で区切る）を流した実行時。矢印の絵は提示の前に入れる。
fn runtime(slot: Entity, window: Entity, lines: usize) -> (TextLayerRuntime, ActorKey) {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let actor = ActorKey::from("0");
    rt.set_balloon_markers(actor.clone(), pair());
    rt.register_actor(
        actor.clone(),
        binding(slot, window),
        ResolvedBalloonText::resolve(&geo_model(), IMAGE),
    );
    for i in 0..lines {
        rt.apply_cue(&cue("0", i as f64, CueCommand::Text("あ".into())));
        if i + 1 < lines {
            rt.apply_cue(&cue(
                "0",
                i as f64 + 0.5,
                CueCommand::NewLine { ratio: 1.0 },
            ));
        }
    }
    (rt, actor)
}

/// 既定の置き場（座標なし）: 上は (幅 − 20, 15)、下は (幅 − 20, 高さ − 15)。点は矢印の不透明な所。
const UP: (f32, f32) = (105.0, 17.0);
const DOWN: (f32, f32) = (105.0, 27.0);

fn shown(rt: &TextLayerRuntime, place: &PlaceKey) -> (bool, bool) {
    (
        rt.marker_at_point(place, UP) == Some(MarkerKind::Up),
        rt.marker_at_point(place, DOWN) == Some(MarkerKind::Down),
    )
}

fn children_of(world: &World, window: Entity) -> Vec<Entity> {
    world
        .get::<Children>(window)
        .map(|c| c.iter().copied().collect())
        .unwrap_or_default()
}

/// 押し出された行が無いうちは印の面を作らず、あふれた次の提示で上の矢印が出る。手動で戻すと
/// 次の提示で下の矢印も出て、自動へ帰ると下の矢印が消える（要件 3.1〜3.3・3.8）。
#[test]
fn arrows_follow_the_pushed_out_lines_on_the_next_present() {
    let (mut world, window, slot) = com_world();
    let (mut rt, actor) = runtime(slot, window, 1);
    let place = PlaceKey::balloon(&actor);
    present_frame(&mut rt, &mut world, DONE).expect("提示");
    assert_eq!(shown(&rt, &place), (false, false));
    assert!(
        rt.marker_surface(&actor).is_none(),
        "矢印が無いうちは面を作らない"
    );

    for i in 1..8 {
        let at = i as f64;
        rt.apply_cue(&cue("0", at, CueCommand::NewLine { ratio: 1.0 }));
        rt.apply_cue(&cue("0", at + 0.5, CueCommand::Text("い".into())));
    }
    present_frame(&mut rt, &mut world, DONE).expect("あふれの提示");
    assert_eq!(shown(&rt, &place), (true, false), "古い側に隠れた行");
    let surface = rt.marker_surface(&actor).expect("印の面");
    let child = surface.window_child().expect("窓の子");
    assert_eq!(children_of(&world, window)[0], child, "文字の面の直前");

    rt.scroll_manual_at(&place, ScrollDir::Older, 2);
    assert_eq!(
        shown(&rt, &place),
        (true, false),
        "送りだけでは描き直さない"
    );
    present_frame(&mut rt, &mut world, DONE).expect("手動の提示");
    assert_eq!(shown(&rt, &place), (true, true));

    rt.scroll_manual_at(&place, ScrollDir::Newer, 2);
    present_frame(&mut rt, &mut world, DONE).expect("自動へ帰る提示");
    assert_eq!(shown(&rt, &place), (true, false));
}

/// 当たりは矢印の絵の α ≥ 0.5 の画素だけ（要件 5.7）。矢印の外も当たらない。
#[test]
fn transparent_arrow_pixels_and_outside_do_not_hit() {
    let (mut world, window, slot) = com_world();
    let (mut rt, actor) = runtime(slot, window, 8);
    let place = PlaceKey::balloon(&actor);
    present_frame(&mut rt, &mut world, DONE).expect("提示");
    assert_eq!(
        rt.marker_at_point(&place, (103.0, 17.0)),
        Some(MarkerKind::Up)
    );
    assert_eq!(
        rt.marker_at_point(&place, (101.0, 17.0)),
        None,
        "α 100 の列"
    );
    assert_eq!(
        rt.marker_at_point(&place, (111.0, 17.0)),
        None,
        "矢印の右の外"
    );
    assert_eq!(
        rt.marker_at_point(&place, (105.0, 21.0)),
        None,
        "矢印の下の外"
    );
    assert_eq!(rt.marker_at_point(&place, (-1.0, -1.0)), None);
}

/// 再追従でモデルの矢印の鍵が替わると、置き場が替わり（要件 3.4・8.6）、古い拡大率の印の面は
/// 次の提示で窓の子から消えて新しい面に替わる（窓の子を残さない）。
#[test]
fn model_change_moves_the_arrow_and_drops_the_old_marker_surface() {
    let (mut world, window, slot) = com_world();
    let (mut rt, actor) = runtime(slot, window, 8);
    let place = PlaceKey::balloon(&actor);
    present_frame(&mut rt, &mut world, DONE).expect("提示");
    assert!(shown(&rt, &place).0);
    let old = rt
        .marker_surface(&actor)
        .and_then(|s| s.window_child())
        .expect("古い印の面");

    let at = |x, y| MarkerPoint {
        x: Some(MarkerCoord::FromStart(x)),
        y: Some(MarkerCoord::FromStart(y)),
    };
    let model = geo_model().with_arrow_keys(ArrowKeys {
        arrow0: at(4, 2),
        arrow1: at(4, 30),
        ..ArrowKeys::default()
    });
    assert!(rt.refresh_actor_binding(&actor, binding(slot, window), &model));
    assert_eq!(rt.layout_input[&place].markers.up, (4, 2));
    present_frame(&mut rt, &mut world, DONE).expect("作り直しの提示");

    assert!(world.get_entity(old).is_err(), "古い印の面は消える");
    let new = rt
        .marker_surface(&actor)
        .and_then(|s| s.window_child())
        .expect("新しい印の面");
    let children = children_of(&world, window);
    assert_eq!(children.iter().filter(|c| **c == new).count(), 1);
    assert!(!children.contains(&old));
    assert_eq!(
        rt.marker_at_point(&place, UP),
        None,
        "前の置き場では当たらない"
    );
    assert_eq!(rt.marker_at_point(&place, (9.0, 4.0)), Some(MarkerKind::Up));
}

/// 箱の矢印の足りない番号は、そのスコープの普通のバルーンの絵で埋める（要件 7.3）。出していない
/// 矢印と、どこにも絵の無い矢印は並べない。
#[test]
fn box_missing_arrow_is_filled_from_the_scope_balloon() {
    let (scope_up, scope_down, box_up) = (solid(1, 1, 255), solid(2, 2, 255), solid(3, 3, 255));
    let actor = ActorKey::from("0");
    let balloon = HashMap::from([(
        actor.clone(),
        MarkerPair {
            up: Some(Arc::clone(&scope_up)),
            down: Some(Arc::clone(&scope_down)),
        },
    )]);
    let name = layout().placements(0)[0].name.clone();
    let boxes = BTreeMap::from([(
        name.clone(),
        MarkerPair {
            up: Some(Arc::clone(&box_up)),
            down: None,
        },
    )]);
    let at = ResolvedMarkerPlacement {
        up: (1, 2),
        down: (3, 4),
    };
    let both = MarkerShown {
        up: true,
        down: true,
    };
    let box_place = PlaceKey {
        actor: actor.clone(),
        place: TextPlace::Box(name),
    };
    let got = placed_markers(&balloon, &boxes, &box_place, both, at);
    assert_eq!(got.len(), 2);
    assert!(Arc::ptr_eq(&got[0].image, &box_up), "箱の絵が先");
    assert_eq!((got[0].kind, got[0].at), (MarkerKind::Up, (1, 2)));
    assert!(
        Arc::ptr_eq(&got[1].image, &scope_down),
        "足りない番号はスコープの絵"
    );
    assert_eq!((got[1].kind, got[1].at), (MarkerKind::Down, (3, 4)));

    let balloon_place = PlaceKey::balloon(&actor);
    let got = placed_markers(&balloon, &boxes, &balloon_place, both, at);
    assert!(
        Arc::ptr_eq(&got[0].image, &scope_up),
        "普通のバルーンは箱の絵を見ない"
    );

    let up_only = MarkerShown {
        up: true,
        down: false,
    };
    assert_eq!(
        placed_markers(&balloon, &boxes, &box_place, up_only, at).len(),
        1
    );
    let other = PlaceKey::balloon(&ActorKey::from("1"));
    assert!(
        placed_markers(&balloon, &boxes, &other, both, at).is_empty(),
        "絵が無い"
    );
}

/// 描画範囲が潰れた再追従の次の提示で、印の面も窓の子ごと外れる（文字の面と一緒に片付ける）。
#[test]
fn collapse_drops_the_marker_surface_with_the_text_surface() {
    let (mut world, window, slot) = com_world();
    let (mut rt, actor) = runtime(slot, window, 8);
    let place = PlaceKey::balloon(&actor);
    present_frame(&mut rt, &mut world, DONE).expect("提示");
    let marker = rt
        .marker_surface(&actor)
        .and_then(|s| s.window_child())
        .expect("印の面");

    // 左 80・右 120 − 60＝60 で幅が負に潰れる。
    let collapsed = parse_model(concat!("validrect.left,80\n", "validrect.right,-60\n"));
    assert!(rt.refresh_actor_binding(&actor, binding(slot, window), &collapsed));
    assert!(rt.layout_input[&place].region.is_collapsed());
    present_frame(&mut rt, &mut world, DONE).expect("潰れた提示");

    assert!(
        world.get_entity(marker).is_err(),
        "印の面の entity が残っている"
    );
    assert!(!children_of(&world, window).contains(&marker));
    assert!(rt.marker_surface(&actor).is_none());
    assert_eq!(rt.marker_at_point(&place, UP), None);
}

/// 印の面だけが失敗しても（Compositor が無く挿せない）、箱は提示した場所に数えられ、文字の出ている
/// 箱の写しに残る（その場所の矢印だけが出ない）。失敗は `present_frame` の `Err` で返る。
#[test]
fn marker_failure_keeps_the_box_in_the_shown_copy() {
    let (mut world, window, slot) = com_world();
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let actor = ActorKey::from("0");
    rt.set_box_layout(
        &mut world,
        layout(),
        super::box_sync_tests::resolver(),
        vec![],
    );
    rt.set_balloon_markers(actor.clone(), pair());
    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&select("0", "a"));
    rt.apply_cue(&text("0", "あ"));
    let shell = TextSlotBinding::new(slot, window, 1.0, (200, 300), (200, 300));
    rt.sync_box_bindings(&mut world, &[(actor.clone(), Some((shell, 0)))]);
    present_frame(&mut rt, &mut world, DONE).expect("あふれる前の提示");
    assert_eq!(rt.shown_boxes(&actor).len(), 1, "箱の文字が出ている前提");

    world.remove_resource::<WucGraphicsResource>();
    for i in 0..8 {
        rt.apply_cue(&cue(
            "0",
            1.0 + i as f64,
            CueCommand::NewLine { ratio: 1.0 },
        ));
        rt.apply_cue(&text("0", "い"));
    }
    assert!(
        present_frame(&mut rt, &mut world, DONE).is_err(),
        "印の面の失敗は返る"
    );
    let shown = rt.shown_boxes(&actor);
    assert_eq!(shown.len(), 1, "印の面の失敗で箱が写しから落ちた");
    let place = PlaceKey {
        actor: actor.clone(),
        place: TextPlace::Box(layout().placements(0)[0].name.clone()),
    };
    assert!(rt.has_hidden_lines_at(&place), "あふれている前提");
    assert!(
        rt.surfaces[&place].markers.is_none(),
        "印の面は挿せていない"
    );
}
