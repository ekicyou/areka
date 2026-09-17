// task 7.2 の檻: バルーン背景色の受け口（[`TextLayerRuntime::set_balloon_background`]／
// `background_of`）と、装着の 1 点（[`TextLayerRuntime::register_actor`]）で 2 層を
// 純粋状態へ差し込む配線（要件 4.6／3.1）。
//
// 檻に入れるのは判断分岐だけ——「背景を受け取っているか」「装着で 2 層が届くか」
// 「再追従が同じ背景で作り直しを起こさないか」の 3 点。COM は要らない
// （`register_actor_binding`／`refresh_actor_binding` は純粋な導出と HashMap 更新のみ）。

use areka_parsers::balloon::{
    BalloonModel, Font, FontColor, Origin, ValidRect, WindowPosition, WordWrapPoint,
};
use areka_sakura::contract::ActorKey;
use bevy_ecs::prelude::World;

use super::test_support::spawn_reserved_slot;
use super::{ResolvedBalloonText, TextLayerRuntime, TextSlotBinding};
use crate::color::mix_disabled;
use crate::draw::DEFAULT_BALLOON_BACKGROUND;
use crate::look::LookLayers;
use crate::state::TextLayerConfig;

/// 代表 native 原寸（`actor_scale_refresh_tests` と同値）。
const NATIVE: (u32, u32) = (400, 224);

/// 既定の 2 層と**必ず異なる**バルーン定義（名前・大きさ・文字色をすべて既定から外す）。
/// これで「装着で 2 層が届く」の述語が恒真にならない——届かなければ状態は
/// [`LookLayers::default`] のまま残り、比較が赤になる。
fn styled_model() -> BalloonModel {
    BalloonModel::new(
        WindowPosition::new(None, None),
        Origin::new(Some(0), Some(0)),
        WordWrapPoint::new(None, None),
        ValidRect::new(None, None, None, None),
        Font::new(
            Some("Meiryo".to_string()),
            Some(20),
            FontColor::new(Some(200), Some(10), Some(20)),
        ),
        None,
        None,
    )
}

/// 背景色として使う非白の代表値（白との違いが混色の各成分に出る値）。
const TINTED: (u8, u8, u8) = (40, 60, 200);

fn binding(world: &mut World) -> (TextSlotBinding, ActorKey) {
    let (window, slot) = spawn_reserved_slot(world);
    (
        TextSlotBinding::new(slot, window, 1.25, (500, 280), NATIVE),
        ActorKey::from("0"),
    )
}

/// R3.1／R4.6: 装着（`register_actor`）の 1 点で解決済みの 2 層が純粋状態へ届く。
///
/// 較正: 「装着で 2 層を差し込まない」誤りを再現すると、状態は `LookLayers::default()`
/// のまま残り本述語が赤になる（`styled_model` が既定と全項目で違うため）。
#[test]
fn attaching_an_actor_delivers_both_look_layers_to_the_pure_state() {
    let mut world = World::new();
    let (bind, actor) = binding(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());

    rt.register_actor_binding(actor.clone(), bind, &styled_model());

    let resolved = &rt.layout_input[&actor];
    let delivered = rt
        .state()
        .actor_state(&actor)
        .expect("装着でスコープが生まれる")
        .look_layers();
    assert_eq!(
        delivered, &resolved.font.looks,
        "装着で解決済みの 2 層がそのまま純粋状態へ届く（要件 3.1）"
    );
    assert_ne!(
        delivered,
        &LookLayers::default(),
        "バルーン定義の値が効いている（既定のままなら差し込みが起きていない）"
    );
}

/// R4.6: 背景の受け口が未設定なら白（[`DEFAULT_BALLOON_BACKGROUND`]）。
#[test]
fn balloon_background_is_white_until_it_is_set() {
    let mut world = World::new();
    let (bind, actor) = binding(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    assert_eq!(
        rt.background_of(&actor),
        DEFAULT_BALLOON_BACKGROUND,
        "未設定の背景は白"
    );

    rt.register_actor_binding(actor.clone(), bind, &styled_model());

    let text = rt.layout_input[&actor].font.color;
    assert_eq!(
        rt.state()
            .actor_state(&actor)
            .unwrap()
            .look_layers()
            .disable
            .color,
        mix_disabled(text, DEFAULT_BALLOON_BACKGROUND),
        "背景未設定のときの無効表示の色は白との混色"
    );
}

/// R4.6: 背景を設定してから装着すると、無効表示の色がその背景との混色になる。
///
/// 較正: 「背景を受け取らず常に白として 2 層を組む」誤りを再現すると、
/// 白との混色と一致してしまい `assert_ne!` の側が赤になる。
#[test]
fn the_disabled_look_mixes_with_the_configured_background() {
    let mut world = World::new();
    let (bind, actor) = binding(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());

    rt.set_balloon_background(actor.clone(), TINTED);
    rt.register_actor_binding(actor.clone(), bind, &styled_model());

    let text = rt.layout_input[&actor].font.color;
    let disable = rt
        .state()
        .actor_state(&actor)
        .unwrap()
        .look_layers()
        .disable
        .color;
    assert_eq!(
        disable,
        mix_disabled(text, TINTED),
        "無効表示の色は設定した背景との混色（要件 4.6）"
    );
    assert_ne!(
        disable,
        mix_disabled(text, DEFAULT_BALLOON_BACKGROUND),
        "白との混色とは違う——背景が実際に導出へ入っている"
    );
}

/// R4.6: 装着と再追従が**同じ導出**を通る——同じ背景での再追従は churn ガードに掛かり、
/// 作り直しの実回数が 0 になる。
///
/// 較正: 「再追従で同じ背景でも作り直す」誤り（＝再追従だけが白で解き直す／装着だけが
/// 背景付きで解く）を再現すると作り直し回数が **1** になる（実測——1 度目の再追従が白の
/// 値を `layout_input` へ書き込むので 2 度目以降は一致してしまう。それでも本番では
/// 「装着直後の 1 フレームで供給面を捨てて全再描画する」退行なので赤にする価値がある）。
/// 数えているのは保持庫の要素数ではなく **`refresh_actor_binding` が再構築を行った実回数**。
#[test]
fn refreshing_with_the_same_background_rebuilds_zero_times() {
    let mut world = World::new();
    let (bind, actor) = binding(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_balloon_background(actor.clone(), TINTED);
    rt.register_actor_binding(actor.clone(), bind, &styled_model());

    let rebuilds = (0..3)
        .filter(|_| rt.refresh_actor_binding(&actor, bind, &styled_model()))
        .count();

    assert_eq!(
        rebuilds, 0,
        "同じ背景・同じ binding の再追従は 1 度も作り直さない（装着と同じ導出）"
    );
}

/// R4.6: 背景が変われば再追従は作り直す（上のテストが「常に false」で恒真になっていない
/// ことの対照——判定キーに背景由来の値が実際に載っている）。
#[test]
fn refreshing_after_the_background_changed_rebuilds_once() {
    let mut world = World::new();
    let (bind, actor) = binding(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.register_actor_binding(actor.clone(), bind, &styled_model());

    rt.set_balloon_background(actor.clone(), TINTED);
    let rebuilds = (0..3)
        .filter(|_| rt.refresh_actor_binding(&actor, bind, &styled_model()))
        .count();

    assert_eq!(
        rebuilds, 1,
        "背景の変化は 1 度だけ作り直しを起こし、以降は再び静穏になる"
    );
    assert_eq!(
        rt.state()
            .actor_state(&actor)
            .unwrap()
            .look_layers()
            .disable
            .color,
        mix_disabled(rt.layout_input[&actor].font.color, TINTED),
        "作り直しの後も 2 層は新しい背景で組まれている"
    );
}

/// R4.6: 背景の受け口は actor ごとに独立する（本体側の背景が相方側に効かない）。
#[test]
fn the_background_slot_is_per_actor() {
    let mut world = World::new();
    let (_, _) = spawn_reserved_slot(&mut world);
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.set_balloon_background(ActorKey::from("0"), TINTED);

    assert_eq!(rt.background_of(&ActorKey::from("0")), TINTED);
    assert_eq!(
        rt.background_of(&ActorKey::from("1")),
        DEFAULT_BALLOON_BACKGROUND,
        "相方側は未設定のまま白"
    );
}

/// R4.6: `ResolvedBalloonText::resolve` は背景 白の
/// [`ResolvedBalloonText::resolve_with_background`]（既存の呼び手を 1 バイトも変えない）。
#[test]
fn resolve_is_resolve_with_a_white_background() {
    let model = styled_model();
    assert_eq!(
        ResolvedBalloonText::resolve(&model, NATIVE),
        ResolvedBalloonText::resolve_with_background(&model, NATIVE, DEFAULT_BALLOON_BACKGROUND),
        "背景を知らない呼び手は白で解決する"
    );
}
