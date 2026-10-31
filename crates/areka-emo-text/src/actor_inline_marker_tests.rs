// script-small-tags task 6.3 の檻: アクターごとの印の見た目（[`TextLayerRuntime::set_inline_marker`]）を
// 解いた直後に押す 3 か所（装着・再追従・箱の解決）の判断分岐（要件 5.1）。
//
// COM は要らない（登録と再追従は純粋な導出と HashMap の更新だけ）。

use std::sync::Arc;

use areka_emo_present::balloon_marker::MarkerImage;
use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::ActorKey;
use bevy_ecs::prelude::World;

use super::TextLayerRuntime;
use super::TextSlotBinding;
use super::box_sync_tests::{fixture, text};
use super::test_support::spawn_reserved_slot;
use crate::inline_marker::InlineMarkerLook;
use crate::place::PlaceKey;
use crate::state::TextLayerConfig;

const NATIVE: (u32, u32) = (400, 224);

fn model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(None, None),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(None, None, FontColor::new(None, None, None)),
        None,
        None,
    )
}

fn image() -> Arc<MarkerImage> {
    Arc::new(MarkerImage {
        width: 2,
        height: 1,
        bgra: vec![1, 2, 3, 255, 4, 5, 6, 255],
    })
}

/// [`image`] と中身の違う絵（スコープを取り違えたら比較が赤になる）。
fn other_image() -> Arc<MarkerImage> {
    Arc::new(MarkerImage {
        width: 1,
        height: 1,
        bgra: vec![9, 9, 9, 255],
    })
}

fn attached() -> (TextLayerRuntime, TextSlotBinding, ActorKey) {
    let mut world = World::new();
    let (window, slot) = spawn_reserved_slot(&mut world);
    let bind = TextSlotBinding::new(slot, window, 1.0, NATIVE, NATIVE);
    (
        TextLayerRuntime::new(TextLayerConfig::default()),
        bind,
        ActorKey::from("0"),
    )
}

fn look_at(rt: &TextLayerRuntime, key: &PlaceKey) -> InlineMarkerLook {
    rt.layout_input[key].font.inline_marker.clone()
}

/// 絵の無いアクターは既定の丸。
#[test]
fn an_actor_without_an_image_gets_the_default_dot() {
    let (mut rt, bind, actor) = attached();
    rt.register_actor_binding(actor.clone(), bind, &model());
    assert_eq!(
        look_at(&rt, &PlaceKey::balloon(&actor)),
        InlineMarkerLook::Dot
    );
}

/// 装着の前に覚えた絵は、装着で解いた直後に押される。
#[test]
fn attaching_stamps_the_remembered_image() {
    let (mut rt, bind, actor) = attached();
    let img = image();
    rt.set_inline_marker(actor.clone(), Some(Arc::clone(&img)));
    rt.register_actor_binding(actor.clone(), bind, &model());
    assert_eq!(
        look_at(&rt, &PlaceKey::balloon(&actor)),
        InlineMarkerLook::Image(img)
    );
}

/// 絵を変えても今の場所の値には押し直さず、次の再追従で 1 度だけ組み直し、以降は静穏。
///
/// 較正: 再追従の比較の前で押さないと、入力が前と同じなので 0 回になり赤になる。
#[test]
fn changing_the_image_rebuilds_once_on_the_next_refresh() {
    let (mut rt, bind, actor) = attached();
    rt.register_actor_binding(actor.clone(), bind, &model());
    let place = PlaceKey::balloon(&actor);

    let img = image();
    rt.set_inline_marker(actor.clone(), Some(Arc::clone(&img)));
    assert_eq!(
        look_at(&rt, &place),
        InlineMarkerLook::Dot,
        "覚えるだけで今の値には押し直さない"
    );

    let rebuilds = (0..3)
        .filter(|_| rt.refresh_actor_binding(&actor, bind, &model()))
        .count();
    assert_eq!(rebuilds, 1, "絵の変化は 1 度だけ組み直しを起こす");
    assert_eq!(look_at(&rt, &place), InlineMarkerLook::Image(img));

    rt.set_inline_marker(actor.clone(), None);
    assert!(
        rt.refresh_actor_binding(&actor, bind, &model()),
        "絵を外すと次の再追従で丸へ組み直す"
    );
    assert_eq!(look_at(&rt, &place), InlineMarkerLook::Dot);
}

/// 箱はそのスコープの普通のバルーンの印を使う（ほかのスコープの絵は効かない）。
#[test]
fn a_box_uses_the_marker_of_its_scope() {
    let img = image();
    let mut f = fixture();
    f.rt.set_inline_marker(ActorKey::from("1"), Some(other_image()));
    f.rt.set_inline_marker(ActorKey::from("0"), Some(Arc::clone(&img)));
    f.show("0");
    f.rt.apply_cue(&text("0", "あ"));
    f.sync(Some(1.0));
    assert_eq!(look_at(&f.rt, &f.key(0, "a")), InlineMarkerLook::Image(img));

    let mut g = fixture();
    g.rt.set_inline_marker(ActorKey::from("1"), Some(other_image()));
    g.show("0");
    g.rt.apply_cue(&text("0", "あ"));
    g.sync(Some(1.0));
    assert_eq!(
        look_at(&g.rt, &g.key(0, "a")),
        InlineMarkerLook::Dot,
        "ほかのスコープの絵は箱に効かない"
    );
}
