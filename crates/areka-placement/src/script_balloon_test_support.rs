//! 台本のバルーン位置のテストが共に使う headless の World の組み立て。

use std::path::PathBuf;

use bevy_ecs::prelude::*;
use wintf::ecs::DPI;

use crate::follow::{MonitorSnapshot, OffsetBase};
use crate::reseed::test_support::world_from;
use crate::reseed::{BalloonPlacementInputs, resolve_standard_scopes};
use crate::reset_pos::DefaultPlacementBasis;
use crate::resolver::{Anchor, PointPx, RectPx, ScopePlacement, SizePx};
use crate::source::{DescriptSource, GhostTitles};
use crate::spawn::GhostWindows;
use crate::test_support::kv;

/// 列挙順の画面の作業領域と DPI。左の画面だけ DPI が違い、窓は右の画面に居る。
pub(super) const MONITORS: [(RectPx, u16); 2] = [
    (
        RectPx {
            left: -1920,
            top: 0,
            right: 0,
            bottom: 1040,
        },
        144,
    ),
    (
        RectPx {
            left: 0,
            top: 0,
            right: 3840,
            bottom: 2100,
        },
        96,
    ),
];

pub(super) const CHAR_SIZES: [SizePx; 2] = [SizePx { w: 434, h: 687 }, SizePx { w: 278, h: 357 }];
pub(super) const BALLOON_SIZE: SizePx = SizePx { w: 223, h: 158 };

pub(super) struct ScriptWorld {
    pub world: World,
    pub windows: GhostWindows,
}

/// 右の画面の下端に sakura と kero の窓を置き、既定の配置の材料（シェルの descript に `extra` を足す）を入れた World。
pub(super) fn script_world(extra: &[(&str, &str)]) -> ScriptWorld {
    let (right, dpi) = MONITORS[1];
    let placements: Vec<ScopePlacement> = [(0, 2000), (1, 1500)]
        .into_iter()
        .map(|(scope, x)| {
            let char_size = CHAR_SIZES[scope];
            let char_pos = PointPx {
                x,
                y: right.bottom - char_size.h,
            };
            let offset = PointPx {
                x: -BALLOON_SIZE.w,
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
                balloon_size: BALLOON_SIZE,
                balloon_offset: offset,
                balloon_offset_base: OffsetBase {
                    offset,
                    dpi: Some(DPI::from_dpi(dpi, dpi)),
                },
                balloon_limit: true,
                anchor: Anchor::Bottom,
                balloon_keyword_base: None,
            }
        })
        .collect();
    let (mut world, windows) = world_from(&placements, dpi);
    world.insert_resource(MonitorSnapshot {
        work_areas: MONITORS.iter().map(|(area, _)| *area).collect(),
    });
    let mut shell_kv = kv(&[("seriko.alignmenttodesktop", "bottom")]);
    shell_kv.extend(kv(extra));
    world.insert_resource(DefaultPlacementBasis {
        source: DescriptSource {
            ghost_kv: kv(&[("kero.name", "k")]),
            shell_kv,
            shell_dir: PathBuf::from("shell/master"),
            titles: GhostTitles::from_scope_titles([(0, "s".to_string()), (1, "k".to_string())]),
        },
        balloon: BalloonPlacementInputs {
            author_dpi: 96,
            windowpositions: Default::default(),
        },
    });
    ScriptWorld { world, windows }
}

#[test]
fn standard_resolve_runs_in_the_two_monitor_world() {
    let w = script_world(&[]);
    let (right, _) = MONITORS[1];

    let basis = w
        .world
        .get_resource::<DefaultPlacementBasis>()
        .expect("既定の配置の材料が World に居る");

    let solved = resolve_standard_scopes(&w.world, &w.windows, &basis.source, &basis.balloon);

    let scopes: Vec<usize> = solved.iter().map(|s| s.placement.scope).collect();
    assert_eq!(scopes, vec![0, 1]);
    for s in &solved {
        assert_eq!(
            s.placement.char_pos.y + s.placement.char_size.h,
            right.bottom
        );
    }
    assert_eq!(
        solved[0].placement.balloon_pos,
        PointPx {
            x: right.right - CHAR_SIZES[0].w - BALLOON_SIZE.w,
            y: right.bottom - CHAR_SIZES[0].h,
        }
    );
}
