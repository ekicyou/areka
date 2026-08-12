//! 明示的な非表示指令（`PresentCommand::Hide` ＝ `\b[-1]` 相当）が枠の面と文字層の双方へ
//! 及ぶことの検証（`areka-P0-balloon-visibility` Requirement 1.2・1.7・1.8・6.3・9.1／
//! design §VisualMount 両 entity 化・§Testing Strategy > Unit Tests 4）。
//!
//! `mount_visibility_tests.rs` は [`crate::mount::VisualMount::set_visible`] を直接呼ぶ形で契約を
//! 固定している。本ファイルが足すのは**既存の指令の入口から流した場合**の主張である。
//!
//! 1. 明示的な非表示指令を [`EmoPresenter::apply`] へ流したとき、可視だった両 entity が不可視かつ
//!    ポインタ判定無効へ**遷移**すること。事後状態だけを見る形は恒真になり得る——元から不可視の
//!    target でも同じ事後状態になるため、非表示が効いた証拠にならない。ゆえに非表示の直前に
//!    双方が可視であることを実 component 値で先に主張する。
//! 2. 文字層スロットが**実際に描画内容を持つ**状態でも同じ非表示が及び、内容を壊さずに再表示で
//!    双方が復帰すること。予約スロットへ内容（`VisualGraphics`）を挿す形は emo-text の
//!    `TextSurface::attach`（`crates/areka-emo-text/src/surface.rs:249-262`）と同一である。
//! 3. 外部所有の不可視構築を**指令経由**で行ったとき、`Visual` が可視の値で挿入される瞬間が一度も
//!    無いこと（Requirement 1.2 の「同一フレーム内の往復も禁止」）。装着 API を直接叩く
//!    `mount_visibility_tests.rs` の検査は初期可視性を引数で渡すため、初期可視性を
//!    `presenter/show.rs` が所有権から導出する側の往復は覆えない。

use super::*;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use areka_actor::reply_channel;
use bevy_ecs::prelude::{Insert, On, Query};

use windows::UI::Composition::Visual as WucVisual;
use windows::core::Interface;

use wintf::ecs::{HitTest, HitTestMode, Visual, VisualGraphics};

use super::test_support::{
    build_target_assets, make_world_with_gpu, show_ok, spawn_window_with_dpi,
};

// ── 檻の補助 ────────────────────────────────────────────────────────────────────────────

/// 枠の面と文字層スロットに期待する状態（可視／不可視の 2 値）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layers {
    /// 双方可視。surface は αマスク判定・slot は矩形判定（component 未付与時の既定と同値）。
    Visible,
    /// 双方不可視。surface・slot とも当たり判定停止（ポインタ透過）。
    Hidden,
}

/// 装着済み target の (枠の面 entity, 文字層スロット entity)。
fn mount_entities(presenter: &EmoPresenter, target: TargetId) -> (Entity, Entity) {
    let mount = presenter
        .targets
        .get(&target)
        .expect("装着済み target")
        .mount
        .as_ref()
        .expect("表示確立後は mount が生成済み");
    (mount.surface_entity(), mount.text_slot())
}

/// 枠の面・文字層スロットの実 component 値が `expected` と一致することを主張する。
///
/// presenter の私有状態（`visible`）ではなく **entity の実値**を読む。照会だけが正しく、
/// 画面に載る側の component が取り残されている食い違いを見逃さないためである。
fn assert_layers(
    world: &World,
    presenter: &EmoPresenter,
    target: TargetId,
    expected: Layers,
    ctx: &str,
) {
    let (surface, slot) = mount_entities(presenter, target);
    let (want_visible, want_surface_hit, want_slot_hit) = match expected {
        Layers::Visible => (true, HitTestMode::AlphaMask, HitTestMode::Bounds),
        Layers::Hidden => (false, HitTestMode::None, HitTestMode::None),
    };

    assert_eq!(
        world
            .get::<Visual>(surface)
            .expect("枠の面 entity に Visual が無い")
            .is_visible,
        want_visible,
        "{ctx}: 枠の面 entity の可視状態が期待と違う（{expected:?} を期待）"
    );
    assert_eq!(
        world
            .get::<Visual>(slot)
            .expect("文字層スロット entity に Visual が無い")
            .is_visible,
        want_visible,
        "{ctx}: 文字層スロット entity の可視状態が期待と違う（{expected:?} を期待）"
    );
    assert_eq!(
        world
            .get::<HitTest>(surface)
            .expect("枠の面 entity に HitTest が無い（明示付与が契約）")
            .mode,
        want_surface_hit,
        "{ctx}: 枠の面 entity のポインタ判定が期待と違う（{expected:?} を期待）"
    );
    assert_eq!(
        world
            .get::<HitTest>(slot)
            .expect("文字層スロット entity に HitTest が無い（明示付与が契約）")
            .mode,
        want_slot_hit,
        "{ctx}: 文字層スロット entity のポインタ判定が期待と違う（{expected:?} を期待）"
    );
}

/// 明示的な非表示指令を指令の入口（[`EmoPresenter::apply`]）へ流し、応答が `Ok(())` であることを見る。
fn hide_via_command(presenter: &mut EmoPresenter, world: &mut World, target: TargetId) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::Hide {
            target,
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "明示的な非表示指令の応答が Ok でない"
    );
}

/// 既定の可視性所有（＝従来の指令駆動）の target を 1 つ組み、初回の表示指令まで済ませて返す。
///
/// 所有権を設定しないことが要点である——本ファイルが対象とするのは本仕様が新設した経路ではなく、
/// 従来から存在する明示的な非表示指令の経路そのものだからである。
fn command_driven_target_after_first_show(salt: u8) -> (World, EmoPresenter, TargetId) {
    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, salt);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    (world, presenter, TargetId(0))
}

/// 予約スロットへ描画内容（`VisualGraphics`）を装着する（emo-text の
/// `TextSurface::attach`（`crates/areka-emo-text/src/surface.rs:249-262`）と同じ形）。
///
/// 本 crate は emo-text へ依存できない（依存方向は emo-compose → emo-present）ため、装着の形だけを
/// 同型に再現する。文字の**絵**は要らない——ここで問うのは「内容を持つスロットにも非表示が及ぶか」
/// であって描かれた画素の内容ではない。
fn attach_content_to_slot(world: &mut World, slot: Entity) {
    let compositor = world
        .get_resource::<WucGraphicsResource>()
        .and_then(|r| r.compositor().cloned())
        .expect("テスト World には WucGraphicsResource がある");
    let sprite = compositor
        .CreateSpriteVisual()
        .expect("CreateSpriteVisual 失敗");
    let wuc_visual: WucVisual = sprite.cast().expect("SpriteVisual->Visual cast 失敗");
    world
        .entity_mut(slot)
        .insert(VisualGraphics::new(wuc_visual));
    // 挿入フックの遅延コマンドを確定させる（mount.rs / emo-text surface.rs と同じ規律）。
    world.flush();
}

// ── ⑴ 指令経由の非表示が双方へ及ぶ（Requirement 1.7/1.8/6.3）─────────────────────────────

/// Requirement 1.7／1.8／6.3: 既存の明示的な非表示指令を指令の入口へ流すと、可視だった枠の面と
/// 文字層スロットが**双方とも**不可視かつポインタ判定停止へ遷移する。
///
/// 非表示の直前に双方が可視であることを先に主張しているのは、事後状態だけの検査が恒真になるためで
/// ある——不可視のまま確立した target でも事後状態は同一で、「非表示が効いた」ことを何も語らない。
#[test]
fn explicit_hide_command_transitions_both_layers_from_visible_to_hidden() {
    let (mut world, mut presenter, target) = command_driven_target_after_first_show(0x41);

    assert_layers(
        &world,
        &presenter,
        target,
        Layers::Visible,
        "明示的な非表示指令の直前",
    );

    hide_via_command(&mut presenter, &mut world, target);

    assert_layers(
        &world,
        &presenter,
        target,
        Layers::Hidden,
        "明示的な非表示指令の直後",
    );
    assert_eq!(
        presenter.target_visible(target),
        Some(false),
        "明示的な非表示指令の後に可視状態の照会が真のまま"
    );
}

// ── ⑵ 内容を持つ文字層でも同じ（完了状態「文字が残らない」）───────────────────────────────

/// Requirement 1.7／6.3: 文字層スロットが**実際に描画内容を持つ**状態でも、明示的な非表示指令は
/// スロットを不可視かつポインタ判定停止にする（枠だけ消えて文字が残らない）。
///
/// 併せて、非表示が内容そのものを破棄しないこと（再表示に再描画を要さない＝供給面・キャッシュを
/// 保持する既存契約と対）と、指令経由の再表示で双方が復帰することを見る。
#[test]
fn explicit_hide_command_covers_a_text_layer_that_holds_drawn_content() {
    let (mut world, mut presenter, target) = command_driven_target_after_first_show(0x42);
    let (_surface, slot) = mount_entities(&presenter, target);

    attach_content_to_slot(&mut world, slot);
    assert!(
        world.get::<VisualGraphics>(slot).is_some(),
        "前提: 文字層スロットへ描画内容が装着されている"
    );
    // 内容の装着は可視性を書き換えない（ここが崩れると以降の遷移の意味が変わる）。
    assert_layers(
        &world,
        &presenter,
        target,
        Layers::Visible,
        "文字層へ内容を装着した直後",
    );

    hide_via_command(&mut presenter, &mut world, target);

    assert_layers(
        &world,
        &presenter,
        target,
        Layers::Hidden,
        "内容を持つ文字層への明示的な非表示指令の直後",
    );
    assert!(
        world.get::<VisualGraphics>(slot).is_some(),
        "非表示は文字層の内容そのものを破棄してはならない（再表示は再描画を要さない）"
    );

    // 指令経由の再表示（同一面の再指令）で双方が復帰する。
    show_ok(&mut presenter, &mut world, target, 1000);
    assert_layers(
        &world,
        &presenter,
        target,
        Layers::Visible,
        "再表示指令の直後",
    );
    assert!(
        world.get::<VisualGraphics>(slot).is_some(),
        "再表示で文字層の内容が失われている"
    );
}

// ── ⑶ 指令経由の不可視構築は可視状態を経由しない（Requirement 1.2）───────────────────────

/// Requirement 1.2: 外部所有の target へ**指令経由**で初回の表示を確立させたとき、枠の面・文字層
/// スロットのいずれの `Visual` も可視の値で挿入されない。
///
/// 事後に不可視であることだけでは「可視で作ってから同一フレーム内で消した」経路を排除できない。
/// `Insert` observer で挿入時の値を全件記録し、可視の挿入が一度も起きないことを検査する。
/// `mount_visibility_tests.rs` の同種の検査は初期可視性を装着 API へ**引数で直接渡す**ため、
/// 初期可視性を所有権から導出する `presenter/show.rs` 側の往復はそちらでは覆えない。
#[test]
fn external_construction_through_the_command_funnel_never_inserts_a_visible_visual() {
    let inserted: Arc<Mutex<Vec<(Entity, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&inserted);

    let mut world = make_world_with_gpu();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(3, 2, 0x43);

    let mut presenter = EmoPresenter::new();
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    presenter
        .set_visibility_ownership(TargetId(0), VisibilityOwnership::External)
        .expect("装着済み target への所有権設定は Ok");

    world.add_observer(move |on: On<Insert, Visual>, q: Query<&Visual>| {
        let entity = on.entity;
        if let Ok(v) = q.get(entity) {
            sink.lock()
                .expect("Visual 挿入記録の Mutex")
                .push((entity, v.is_visible));
        }
    });

    // 表示の確立（装着の遅延生成はこの中で起きる）。
    show_ok(&mut presenter, &mut world, TargetId(0), 1000);

    let (surface, slot) = mount_entities(&presenter, TargetId(0));
    let records = inserted.lock().expect("Visual 挿入記録の Mutex").clone();

    // 観測器そのものの較正: 両 entity の Visual 挿入を実際に捉えていること
    // （捉えていなければ「可視の挿入なし」は恒真になる）。
    assert!(
        records.iter().any(|(e, _)| *e == surface),
        "枠の面 entity の Visual 挿入が観測されていない（観測器が機能していない）: {records:?}"
    );
    assert!(
        records.iter().any(|(e, _)| *e == slot),
        "文字層スロット entity の Visual 挿入が観測されていない（観測器が機能していない）: {records:?}"
    );
    // 本題: 可視の値で挿入された Visual が一度も無いこと。
    assert!(
        records.iter().all(|(_, visible)| !*visible),
        "指令経由の不可視構築で Visual が可視の値で挿入された（Requirement 1.2）: {records:?}"
    );

    assert_layers(
        &world,
        &presenter,
        TargetId(0),
        Layers::Hidden,
        "外部所有の初回表示指令の直後",
    );
}
