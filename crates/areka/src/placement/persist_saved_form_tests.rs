//! 記憶へ書く形（[`saved_form_anchor`]）のテスト（areka-P0-window-link-directives 要件 4.8・
//! 設計の決め 6・design Testing Strategy「`saved_form_anchor` と書く形」）。
//!
//! 次の起動は窓を descript の向きで読み戻すので、台本で向きを変えた窓も descript の向きの形で
//! 書けば、読み戻しで x が書いた位置に戻る。

use bevy_ecs::prelude::*;

use super::{char_pos_from_origin_x, char_pos_to_origin_x, project_restore, saved_form_anchor};
use crate::placement::follow::{AnchorSource, Anchored, MonitorSnapshot};
use crate::placement::resolver::{Anchor, PointPx, RectPx, SizePx};

/// 形は `AnchorSource.descript` で決まり、今の向き（`Anchored`）は使わない。
#[test]
fn saved_form_uses_descript_anchor_over_current_anchor() {
    let mut world = World::new();
    let window = world
        .spawn((
            Anchored(Anchor::Free),
            AnchorSource {
                descript: Anchor::Bottom,
                by_script: true,
            },
        ))
        .id();
    assert_eq!(saved_form_anchor(&world, window), Some(Anchor::Bottom));
}

/// `AnchorSource` の無い窓は `Anchored` の形で書き、どちらも無ければ形は無い。
#[test]
fn saved_form_falls_back_to_anchored_then_none() {
    let mut world = World::new();
    let anchored_only = world.spawn(Anchored(Anchor::Left)).id();
    let bare = world.spawn_empty().id();
    assert_eq!(saved_form_anchor(&world, anchored_only), Some(Anchor::Left));
    assert_eq!(saved_form_anchor(&world, bare), None);
}

/// 設計の例: descript が下・台本で `free` にした窓（幅 200）を左上 (100, 300) で書くと、次の
/// 起動の読み戻し（下の向き）で x が 100 に戻る（今の向きの形で書くと 0 へずれる）。
#[test]
fn descript_bottom_free_window_restores_to_saved_x_on_next_boot() {
    let mut world = World::new();
    let window = world
        .spawn((
            Anchored(Anchor::Free),
            AnchorSource {
                descript: Anchor::Bottom,
                by_script: true,
            },
        ))
        .id();
    let pos = PointPx { x: 100, y: 300 };
    let size = SizePx { w: 200, h: 400 };
    let snapshot = MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1040,
        }],
    };

    let form = saved_form_anchor(&world, window).expect("形がある");
    let saved = char_pos_to_origin_x(form, pos, size);
    assert_eq!(
        saved,
        PointPx { x: 200, y: 300 },
        "下の形（下端の中央の x）で書く"
    );

    // 次の起動の読み戻し: descript の向き（下）で原点から左上へ戻し、作業領域へ射影する。
    let restored = project_restore(
        Anchor::Bottom,
        char_pos_from_origin_x(Anchor::Bottom, saved, size),
        size,
        &snapshot,
    );
    assert_eq!(restored.x, 100, "x は書いた位置に戻る");
    assert_eq!(restored.y, 1040 - 400, "y は descript の向きで下端へ寄る");
}
