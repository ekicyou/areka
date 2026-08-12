//! `VisualMount` の非表示契約（枠の面＋文字層スロットの両 entity）を検証する。
//!
//! 対象 Acceptance Criteria（`areka-P0-balloon-visibility` requirements.md）:
//! - 1.7: 非表示は枠の面と文字層の双方へ及ぶ（枠だけ消えて文字が残らない）。
//! - 1.8: 不可視時のポインタ透過も双方へ及ぶ（文字層だけ判定が残らない）。
//! - 1.4: 不可視時の枠のポインタ透過（従来挙動の維持）。
//! - 1.2: 不可視構築は可視状態を一度も経由しない（構造保証——フレーム遅延や合成の
//!   タイミングに成否を委ねない）。
//!
//! 文字層スロットが従来 `HitTest` component を持たなかったことは、`wintf` 側で
//! 「未付与＝既定 `HitTestMode::Bounds`」（`ecs/layout/hit_test/mod.rs:367-371`）と扱われ、
//! `Bounds` 判定の合成 α は `Visual::clamped_opacity()`（`ecs/graphics/visual.rs:140-142`）＝
//! `is_visible` を見ないため、可視性を落としてもポインタが透過しない。ゆえに本層で
//! スロットへも明示的に `HitTest` を付与して切り替える。

use super::*;

use std::sync::{Arc, Mutex};

use wintf::ecs::HitTestMode;

use super::test_support::{attach_fixture, attach_fixture_with_visibility};

/// entity の `Visual::is_visible` を取り出す（未装着は panic＝構造前提の破綻）。
fn is_visible(world: &World, entity: Entity, what: &str) -> bool {
    world
        .get::<Visual>(entity)
        .unwrap_or_else(|| panic!("{what} に Visual が無い"))
        .is_visible
}

/// entity の `HitTest::mode` を取り出す（未付与は panic＝明示付与が契約）。
fn hit_mode(world: &World, entity: Entity, what: &str) -> HitTestMode {
    world
        .get::<HitTest>(entity)
        .unwrap_or_else(|| panic!("{what} に HitTest が無い（明示付与が契約）"))
        .mode
}

/// 1.7 / 1.8 / 1.4: `set_visible(false)` が枠の面と文字層スロットの双方を不可視かつ
/// ポインタ透過（`HitTest::None`）にし、`set_visible(true)` で双方が復帰すること。
///
/// 可視時のスロットは `HitTestMode::Bounds`＝従来の「component 不在＝既定 Bounds」と同値。
#[test]
fn hide_and_show_cover_both_surface_and_text_slot() {
    let (mut world, _window, mount, _g) = attach_fixture(4, 4);
    let surface = mount.surface_entity();
    let slot = mount.text_slot();

    // 装着直後（可視構築）: 双方可視・surface＝αマスク・slot＝Bounds（従来挙動と同値）。
    assert!(
        is_visible(&world, surface, "surface entity"),
        "装着直後は可視"
    );
    assert!(
        is_visible(&world, slot, "text-layer slot"),
        "装着直後は可視"
    );
    assert_eq!(
        hit_mode(&world, surface, "surface entity"),
        HitTestMode::AlphaMask
    );
    assert_eq!(
        hit_mode(&world, slot, "text-layer slot"),
        HitTestMode::Bounds,
        "可視時のスロットは既定 Bounds と同値の明示付与"
    );

    // 非表示へ: 双方が不可視 ∧ 双方が HitTest::None。
    mount.set_visible(&mut world, false);
    assert!(
        !is_visible(&world, surface, "surface entity"),
        "非表示後は枠の面が不可視（1.7）"
    );
    assert!(
        !is_visible(&world, slot, "text-layer slot"),
        "非表示後は文字層スロットも不可視（1.7——枠だけ消えて文字が残ってはならない）"
    );
    assert_eq!(
        hit_mode(&world, surface, "surface entity"),
        HitTestMode::None,
        "非表示後は枠のポインタ判定が停止（1.4）"
    );
    assert_eq!(
        hit_mode(&world, slot, "text-layer slot"),
        HitTestMode::None,
        "非表示後は文字層スロットのポインタ判定も停止（1.8——既定 Bounds のまま残ってはならない）"
    );

    // 再表示へ: 双方が復帰。
    mount.set_visible(&mut world, true);
    assert!(
        is_visible(&world, surface, "surface entity"),
        "再表示で枠が復帰"
    );
    assert!(
        is_visible(&world, slot, "text-layer slot"),
        "再表示で文字層スロットも復帰"
    );
    assert_eq!(
        hit_mode(&world, surface, "surface entity"),
        HitTestMode::AlphaMask,
        "再表示で枠は αマスク判定へ復帰"
    );
    assert_eq!(
        hit_mode(&world, slot, "text-layer slot"),
        HitTestMode::Bounds,
        "再表示で文字層スロットは Bounds 判定へ復帰（従来同値）"
    );
}

/// 1.2: 不可視構築（`initially_visible=false`）では、surface entity・text-layer slot の
/// いずれの `Visual` も **可視の値で挿入されない**こと。
///
/// 「装着直後に不可視であること」だけでは「可視で作ってから消した」経路を排除できないため、
/// `Insert` observer で `Visual` の**挿入時の値**を全件記録し、`is_visible == true` の挿入が
/// 一度も起きないことを検査する（構造保証——同一フレーム内の往復という抜け道を塞ぐ）。
#[test]
fn invisible_construction_never_inserts_a_visible_visual() {
    let inserted: Arc<Mutex<Vec<(Entity, bool)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&inserted);

    let (world, _window, mount, _g) =
        attach_fixture_with_visibility(4, 4, false, move |world: &mut World| {
            world.add_observer(move |on: On<Insert, Visual>, q: Query<&Visual>| {
                let entity = on.entity;
                if let Ok(v) = q.get(entity) {
                    sink.lock()
                        .expect("Visual 挿入記録の Mutex")
                        .push((entity, v.is_visible));
                }
            });
        });

    let surface = mount.surface_entity();
    let slot = mount.text_slot();

    let records = inserted.lock().expect("Visual 挿入記録の Mutex").clone();
    // 観測器そのものの較正: 両 entity の Visual 挿入を実際に捉えていること
    // （捉えていなければ「可視の挿入なし」は恒真になる）。
    assert!(
        records.iter().any(|(e, _)| *e == surface),
        "surface entity の Visual 挿入が観測されていない（観測器が機能していない）: {records:?}"
    );
    assert!(
        records.iter().any(|(e, _)| *e == slot),
        "text-layer slot の Visual 挿入が観測されていない（観測器が機能していない）: {records:?}"
    );
    // 本題: 可視の値で挿入された Visual が一度も無いこと。
    assert!(
        records.iter().all(|(_, visible)| !*visible),
        "不可視構築では Visual が可視の値で挿入されてはならない（1.2）: {records:?}"
    );

    // 装着完了時点でも双方が不可視 ∧ ポインタ透過。
    assert!(
        !is_visible(&world, surface, "surface entity"),
        "不可視構築の枠は不可視"
    );
    assert!(
        !is_visible(&world, slot, "text-layer slot"),
        "不可視構築の文字層スロットも不可視"
    );
    assert_eq!(
        hit_mode(&world, surface, "surface entity"),
        HitTestMode::None,
        "不可視構築の枠はポインタ透過（1.4）"
    );
    assert_eq!(
        hit_mode(&world, slot, "text-layer slot"),
        HitTestMode::None,
        "不可視構築の文字層スロットもポインタ透過（1.8）"
    );
}
