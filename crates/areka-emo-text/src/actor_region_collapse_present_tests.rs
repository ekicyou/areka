//! # actor_region_collapse_present_tests — 潰れた場所は提示で描かず、描いた後に潰れたら前の字を片付ける（件 2・実 GPU）
//!
//! 出典 spec: `areka-P0-balloon-text-bugs`（要件 **2.1**・**2.2**・**2.4**・**2.6**〜**2.9**・**4.2**、
//! design「潰れた描画範囲（件 2）」・テストの方針「結合 7」）。
//!
//! 提示（`present_frame`）は、描画範囲の潰れた場所を装着せず描かず（供給面を作らない）、記録も
//! 出さない。描画資源が残っていれば（描いた後の再追従で潰れた）その提示で `TextSurface::release` で
//! 外し、当たり行の写しを外す。潰れた場所は「提示した場所」に数えない（箱の写しに載らない）。
//! 戻れば次の提示で今までどおり装着し、装着の記録の文は変えない。
//!
//! 記録の件数は捕捉窓の中の対照の `error!` 1 件と同時に確かめる（0 件の主張を恒真にしない）。
//! 登録口の記録（潰れの警告・戻りの情報）は捕捉窓の外で済ませ、窓の中は提示のフレームだけにする。

use areka_sakura::contract::{ActorKey, CueCommand};
use bevy_ecs::entity::Entity;
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::name::Name;
use bevy_ecs::prelude::World;
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::{Arrangement, Visual, VisualGraphics};

use super::box_sync_tests::{emote, resolver, text};
use super::region_collapse_tests::{
    COLLAPSED_BALLOON, HEALTHY_BALLOON, IMAGE, SHELL_COLLAPSED, SHELL_HEALTHY, SHELL_IMAGE,
    box_layout, model,
};
use super::test_support::{choice_cue, com_world, cue, geo_model, opaque_count};
use super::{TextLayerRuntime, TextSlotBinding, present_frame};
use crate::place::{PlaceKey, TextPlace};
use crate::state::TextLayerConfig;

/// 装着の記録の文（要件 2.8・1 文字も変えない）。
const ATTACHED: &str =
    "テキスト供給面を予約スロットへ装着した（ActorRender 不在時のみ・以降は Present のみ）";

/// 数十フレーム（0.1 秒刻み）。
const FRAMES: usize = 40;

/// 窓に差し込み口をもう 1 つと枠の面を足す（emo-present の装着と同じく差し込み口が枠より手前）。
fn add_slot(world: &mut World, window: Entity, name: &str) -> Entity {
    let id = world
        .spawn((
            Name::new(name.to_owned()),
            Visual::default(),
            ChildOf(window),
        ))
        .id();
    world.flush();
    id
}

fn children(world: &World, window: Entity) -> Vec<Entity> {
    world
        .get::<Children>(window)
        .map(|c| c.iter().copied().collect())
        .unwrap_or_default()
}

/// 捕捉窓の中で `f` を走らせ、全部の記録を返す。対照の `error!` 1 件だけが ERROR で、WARN が 0 件で
/// あることをここで確かめる（要件 2.2）。
fn quiet(f: impl FnOnce()) -> Vec<CapturedEvent> {
    let ((), events) = capture(|| {
        tracing::error!("捕捉窓が生きていることの対照イベント");
        f();
    });
    let count = |level: tracing::Level| events.iter().filter(|e| e.level == level).count();
    assert_eq!(
        count(tracing::Level::ERROR),
        1,
        "対照の 1 件のほかに ERROR が出た（要件 2.2）: {events:?}"
    );
    assert_eq!(
        count(tracing::Level::WARN),
        0,
        "提示が WARN を出した: {events:?}"
    );
    events
}

fn attach_messages(events: &[CapturedEvent]) -> usize {
    events.iter().filter(|e| e.message() == ATTACHED).count()
}

fn frames(rt: &mut TextLayerRuntime, world: &mut World) {
    for i in 0..FRAMES {
        present_frame(rt, world, 0.1 * (i + 1) as f64).expect("潰れた場所があっても提示は Ok");
    }
}

/// 潰れたスコープ 1 に文字があっても、数十フレームで ERROR・WARN 0 行・面は作られず差し込み口に
/// 文字の部品が付かない。隣のスコープ 0 は描かれる（要件 2.1・2.2・2.6）。
#[test]
fn a_collapsed_balloon_is_never_attached_and_the_neighbour_is_drawn() {
    let (mut world, window, slot0) = com_world();
    let slot1 = add_slot(&mut world, window, "emo-text-layer-slot-1");
    let (small, other) = (ActorKey::from("1"), ActorKey::from("0"));
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&cue("1", 0.0, CueCommand::Text("あいう".into())));
    rt.apply_cue(&cue("0", 0.0, CueCommand::Text("えお".into())));
    rt.register_actor_binding(
        small.clone(),
        TextSlotBinding::new(slot1, window, 1.0, IMAGE, IMAGE),
        &model(COLLAPSED_BALLOON),
    );
    rt.register_actor_binding(
        other.clone(),
        TextSlotBinding::new(slot0, window, 1.0, (120, 60), (120, 60)),
        &geo_model(),
    );

    let events = quiet(|| frames(&mut rt, &mut world));

    assert!(
        !rt.is_attached(&small),
        "潰れた場所に面を作った（要件 2.1）"
    );
    assert!(world.get::<VisualGraphics>(slot1).is_none());
    assert!(rt.choice_hit_rows(&small).is_empty());
    assert!(
        rt.is_attached(&other),
        "隣のスコープが装着されない（要件 2.6）"
    );
    let bytes = rt
        .surface(&other)
        .expect("隣の面")
        .read_back()
        .expect("read_back");
    assert!(opaque_count(&bytes) > 0, "隣のスコープの字が描かれない");
    assert_eq!(attach_messages(&events), 1, "装着は隣のスコープの 1 回だけ");
}

/// 描いた後に潰れると、次の提示で差し込み口から文字の部品が外れ当たり行が空になる（要件 2.9）。
/// 戻すと次のフレームで装着し、記録の文は同じで、差し込み口は枠の面より手前に居る（要件 2.4・2.8）。
#[test]
fn collapsing_after_drawing_releases_the_surface_and_recovery_reattaches() {
    let (mut world, window, slot) = com_world();
    let frame = add_slot(&mut world, window, "emo-surface");
    let actor = ActorKey::from("1");
    let binding = TextSlotBinding::new(slot, window, 1.0, IMAGE, IMAGE);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("1", 0.0, "OnYes", "はい", &["r0"]));
    rt.register_actor_binding(actor.clone(), binding, &model(HEALTHY_BALLOON));
    present_frame(&mut rt, &mut world, 1.0).expect("描く");
    assert!(world.get::<VisualGraphics>(slot).is_some(), "描いた前提");
    assert!(!rt.choice_hit_rows(&actor).is_empty(), "当たり行がある前提");

    assert!(rt.refresh_actor_binding(&actor, binding, &model(COLLAPSED_BALLOON)));
    quiet(|| frames(&mut rt, &mut world));
    assert!(!rt.is_attached(&actor));
    assert!(
        world.get::<VisualGraphics>(slot).is_none() && world.get::<Arrangement>(slot).is_none(),
        "前の字の部品が差し込み口に残っている（要件 2.9）"
    );
    assert!(
        rt.choice_hit_rows(&actor).is_empty(),
        "当たり行が残っている"
    );
    assert_eq!(children(&world, window), vec![slot, frame]);

    assert!(rt.refresh_actor_binding(&actor, binding, &model(HEALTHY_BALLOON)));
    let events = quiet(|| {
        present_frame(&mut rt, &mut world, 5.0).expect("戻った次のフレーム");
    });
    assert_eq!(
        attach_messages(&events),
        1,
        "戻った次のフレームで装着（要件 2.4・2.8）"
    );
    assert!(rt.is_attached(&actor));
    assert!(!rt.choice_hit_rows(&actor).is_empty(), "当たり行が戻る");
    let vg = world.get::<VisualGraphics>(slot).expect("付け直しの絵");
    assert!(
        vg.parent_visual().is_none(),
        "付け直しの絵は未同期＝visual_sync が Children の順に入れ直す"
    );
    assert_eq!(
        children(&world, window),
        vec![slot, frame],
        "差し込み口は枠より手前"
    );
}

/// 箱でも同じ（要件 2.7）: 描いた箱の束を潰れた定義へ差し替えると面が外れ、数十フレームで ERROR・
/// WARN 0 行・面は作られず写しに載らない。隣のスコープは描かれる。戻すと次のフレームで装着する。
#[test]
fn a_collapsed_box_is_not_drawn_nor_shown_and_recovery_reattaches() {
    let (mut world, window, slot) = com_world();
    let slot1 = add_slot(&mut world, window, "emo-text-layer-slot-1");
    let (scope0, scope1) = (ActorKey::from("0"), ActorKey::from("1"));
    let shell = TextSlotBinding::new(slot, window, 1.0, SHELL_IMAGE, SHELL_IMAGE);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let speak = |rt: &mut TextLayerRuntime, world: &mut World, source: &str| {
        rt.set_box_layout(world, box_layout(source), resolver(), vec![]);
        rt.apply_cue(&emote("0", "0"));
        rt.apply_cue(&text("0", "あ"));
        rt.sync_box_bindings(world, &[(ActorKey::from("0"), Some((shell, 0)))]);
    };
    rt.apply_cue(&cue("1", 0.0, CueCommand::Text("えお".into())));
    rt.register_actor_binding(
        scope1.clone(),
        TextSlotBinding::new(slot1, window, 1.0, (120, 60), (120, 60)),
        &geo_model(),
    );
    let box_key = |rt: &TextLayerRuntime| -> PlaceKey {
        rt.box_sites
            .keys()
            .find(|k| k.place != TextPlace::Balloon)
            .cloned()
            .expect("箱 c が登録されている")
    };

    speak(&mut rt, &mut world, SHELL_HEALTHY);
    present_frame(&mut rt, &mut world, 1.0).expect("描く");
    let drawn = box_key(&rt);
    let child = rt
        .surface_at(&drawn)
        .and_then(|s| s.window_child())
        .expect("箱の面を描いた前提");
    assert_eq!(rt.shown_boxes(&scope0).len(), 1, "写しに載る前提");

    speak(&mut rt, &mut world, SHELL_COLLAPSED);
    quiet(|| frames(&mut rt, &mut world));
    let collapsed = box_key(&rt);
    assert!(rt.surface_at(&collapsed).is_none(), "潰れた箱に面を作った");
    assert!(
        world.get_entity(child).is_err(),
        "前の字の面が残っている（要件 2.9）"
    );
    assert!(rt.choice_hit_rows_at(&collapsed).is_empty());
    assert!(rt.shown_boxes(&scope0).is_empty(), "潰れた箱を写しに載せた");
    assert_eq!(children(&world, window), vec![slot, slot1]);
    assert!(
        rt.is_attached(&scope1),
        "隣のスコープが描かれない（要件 2.6）"
    );

    speak(&mut rt, &mut world, SHELL_HEALTHY);
    let events = quiet(|| {
        present_frame(&mut rt, &mut world, 5.0).expect("戻った次のフレーム");
    });
    assert_eq!(
        attach_messages(&events),
        1,
        "戻った次のフレームで装着（要件 2.8）"
    );
    assert!(rt.surface_at(&box_key(&rt)).is_some());
    assert_eq!(rt.shown_boxes(&scope0).len(), 1);
}
