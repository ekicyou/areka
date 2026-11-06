//! `reseed` のテストの補助のうち、本体のテストも使う 3 つ（`booted_world`・`new_shell`・
//! `char_positions`）と、それらが中で使う補助（spec: placement-crate-split 設計 TestSupportFeature）。
//! テストだけが使う補助は `reseed_tests.rs` に残す。

use std::collections::BTreeMap;
use std::path::PathBuf;

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use wintf::ecs::{DPI, Point, WindowHandle, WindowPos};

use crate::config::BalloonXMode;
use crate::follow::{MonitorSnapshot, OffsetBase};
use crate::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::source::{DescriptSource, GhostTitles};
use crate::spawn::{GhostWindows, spawn_ghost_windows};

pub(crate) const CHAR0: SizePx = SizePx { w: 434, h: 687 };
pub(crate) const CHAR1: SizePx = SizePx { w: 278, h: 357 };
pub(crate) const BALLOON: SizePx = SizePx { w: 223, h: 158 };

/// 起動時のシェル（Bottom・バルーンは左・ずらし無し）で組んだ 2 スコープの配置。
pub(crate) fn boot_placements() -> Vec<ScopePlacement> {
    let scope = |scope, char_pos: PointPx, char_size, keyword| {
        let offset = PointPx {
            x: -BALLOON.w,
            y: 0,
        };
        ScopePlacement {
            scope,
            char_pos,
            char_size,
            balloon_pos: PointPx {
                x: char_pos.x + offset.x,
                y: char_pos.y,
            },
            balloon_size: BALLOON,
            balloon_offset: offset,
            balloon_offset_base: OffsetBase {
                offset,
                dpi: Some(DPI::from_dpi(96, 96)),
            },
            balloon_limit: true,
            anchor: Anchor::Bottom,
            balloon_keyword_base: keyword,
        }
    };
    vec![
        scope(
            0,
            PointPx { x: 2000, y: 1413 },
            CHAR0,
            Some((BalloonXMode::CenterTop, PointPx { x: 0, y: 0 })),
        ),
        scope(1, PointPx { x: 1500, y: 1743 }, CHAR1, None),
    ]
}

/// 起動の窓を組み、窓書込の口が通るよう偽 HWND と窓の DPI を持たせる。
pub fn booted_world(dpi: u16) -> (World, GhostWindows) {
    world_from(&boot_placements(), dpi)
}

/// `placements` で窓を組む（作業領域は十分に広い 1 枚・全窓の DPI は `dpi`）。
pub(crate) fn world_from(placements: &[ScopePlacement], dpi: u16) -> (World, GhostWindows) {
    let mut world = World::new();
    world.insert_resource(MonitorSnapshot {
        work_areas: vec![RectPx {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2100,
        }],
    });
    let titles = GhostTitles::from_scope_titles([(0, "s".to_string()), (1, "k".to_string())]);
    let windows = spawn_ghost_windows(&mut world, placements, &titles);
    for (i, scope) in windows.scopes().enumerate() {
        let char_window = windows.char_window(scope).unwrap();
        let balloon_window = windows.balloon_window(scope).unwrap();
        world.entity_mut(char_window).insert(fake_handle(0x100 + i));
        world
            .entity_mut(balloon_window)
            .insert(fake_handle(0x200 + i));
    }
    // `WindowHandle` の付与の hook が偽 HWND の DPI（取れず 96）を遅延で書くので、流してから置く。
    world.flush();
    for scope in windows.scopes() {
        for window in [windows.char_window(scope), windows.balloon_window(scope)] {
            world
                .entity_mut(window.unwrap())
                .insert(DPI::from_dpi(dpi, dpi));
        }
    }
    (world, windows)
}

pub(crate) fn fake_handle(raw: usize) -> WindowHandle {
    WindowHandle {
        hwnd: HWND(raw as *mut _),
        instance: HINSTANCE::default(),
    }
}

/// 新しいシェル: 揃え方は free・sakura のバルーンは右で (30,40)・kero は (-7,9)。
/// `char2.` のキーを持つが、設定にスコープ 2 は現れない（spec: areka-P0-emily-ghost-verification
/// 要件 4.7・6.7——`char{n}.`（n≥2）からは scope を足さない）。
pub fn new_shell(extra: &[(&str, &str)]) -> DescriptSource {
    let kv = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    };
    let mut shell_kv = kv(&[
        ("seriko.alignmenttodesktop", "free"),
        ("sakura.balloon.alignment", "right"),
        ("sakura.balloon.offsetx", "30"),
        ("sakura.balloon.offsety", "40"),
        ("kero.balloon.offsetx", "-7"),
        ("kero.balloon.offsety", "9"),
        ("char2.balloon.offsetx", "5"),
    ]);
    shell_kv.extend(kv(extra));
    DescriptSource {
        ghost_kv: kv(&[("kero.name", "k")]),
        shell_kv,
        shell_dir: PathBuf::from("shell/second"),
        titles: GhostTitles::from_scope_titles([(0, "s".to_string()), (1, "k".to_string())]),
    }
}

pub(crate) fn position_of(world: &World, entity: Entity) -> Point {
    world.get::<WindowPos>(entity).unwrap().position.unwrap()
}

pub fn char_positions(world: &World, windows: &GhostWindows) -> Vec<Point> {
    windows
        .scopes()
        .map(|s| position_of(world, windows.char_window(s).unwrap()))
        .collect()
}
