//! 面の拡大の寄せの檻（window-work-area-clamp・task 1.3・要件 3.1・3.2・3.5・7.1・7.2・7.4・7.6・
//! 10.1 ⑶⑸⑹）。
//!
//! 数値は開発者の机の形の合成: 主の画面（200%）の作業領域 `0..2880 × 0..1704`。既定の位置の
//! `free` の窓が y=864 に 640×840（下端 1704）で立ち、面の切り替えで 1108×1368 へ大きくなる。

use bevy_ecs::prelude::*;

use super::super::test_support::{ExpectField, LogEvent, capture_logs, expect_one};
use super::test_support::{dragging_state, point, rect};
use super::{
    MonitorSnapshot, PlacementRoute, WORK_AREA_CLAMP_TAG, WORK_AREA_KEEP_TAG,
    WORK_AREA_UNRESOLVED_TAG, clamp_grown_free_default,
};
use crate::placement::chain_finalize::ScriptPlaced;
use crate::placement::follow::OffsetBase;
use crate::placement::resolver::{Anchor, PointPx, ScopePlacement, SizePx};
use crate::placement::source::GhostTitles;
use crate::placement::spawn::{GhostWindows, spawn_ghost_windows};

const SCOPE: usize = 0;
/// 既定の位置（下端 864+840=1704＝作業領域の下端）。
const DEFAULT_POS: PointPx = PointPx { x: 1000, y: 864 };
/// 大きくなった面。
const GROWN: SizePx = SizePx { w: 1108, h: 1368 };

fn desk() -> MonitorSnapshot {
    MonitorSnapshot {
        work_areas: vec![rect(0, 0, 2880, 1704)],
    }
}

/// `free` のキャラ窓 1 枚のスコープを本番の生成口で立てる（既定の位置の台帳も本番の経路）。
fn world_free(snap: Option<MonitorSnapshot>) -> (World, Entity) {
    let placements = vec![ScopePlacement {
        scope: SCOPE,
        char_pos: DEFAULT_POS,
        char_size: SizePx { w: 640, h: 840 },
        balloon_pos: PointPx { x: 600, y: 800 },
        balloon_size: SizePx { w: 223, h: 158 },
        balloon_offset: PointPx { x: -400, y: -64 },
        balloon_offset_base: OffsetBase::unpinned(PointPx { x: -400, y: -64 }),
        balloon_limit: false,
        anchor: Anchor::Free,
        balloon_keyword_base: None,
    }];
    let mut world = World::new();
    let gw = spawn_ghost_windows(
        &mut world,
        &placements,
        &GhostTitles::from_scope_titles([(SCOPE, "a".to_string())]),
    );
    let char_window = gw.char_window(SCOPE).expect("char 窓がある");
    if let Some(snap) = snap {
        world.insert_resource(snap);
    }
    (world, char_window)
}

fn grow(world: &World, window: Entity, route: PlacementRoute, size: SizePx) -> PointPx {
    clamp_grown_free_default(world, window, Anchor::Free, route, DEFAULT_POS, size)
}

fn tagged<'a>(events: &'a [LogEvent], tag: &str) -> Vec<&'a LogEvent> {
    events
        .iter()
        .filter(|e| e.message().contains(tag))
        .collect()
}

/// ⑶ 200% で y=864 の 640×840 が 1108×1368 になると、下端が 1704 以下へ収まり x は変わらない
/// （はみ出した向きにだけ動かす・要件 3.1・3.2）。`Clamp`（info・how=grown）が残る。
/// 面の再スナップと窓寸の照合の両経路で同じ。
#[test]
fn grown_default_free_window_is_lifted_only_vertically() {
    let (world, window) = world_free(Some(desk()));
    for route in [
        PlacementRoute::Resnap,
        PlacementRoute::ReportedSizeReconcile,
    ] {
        let (to, events) = capture_logs(|| grow(&world, window, route, GROWN));

        assert_eq!(to, point(1000, 1704 - 1368), "{route:?}");
        assert!(to.y + GROWN.h <= 1704);
        let clamp = expect_one(&events, WORK_AREA_CLAMP_TAG);
        assert_eq!(clamp.level, tracing::Level::INFO);
        assert_eq!(clamp.expect_field("scene"), "\"surface-grow\"");
        assert_eq!(clamp.expect_field("scope"), "0");
        assert_eq!(clamp.expect_field("how"), "\"grown\"");
        assert_eq!(clamp.expect_field("from"), format!("{DEFAULT_POS:?}"));
        assert_eq!(clamp.expect_field("to"), format!("{to:?}"));
        assert_eq!(
            clamp.expect_field("work_area"),
            format!("{:?}", Some(rect(0, 0, 2880, 1704)))
        );
    }
}

/// 既定の位置と違う窓（ドラッグの後）・台本の印の窓・ドラッグ中の窓・既定の位置の無い窓
/// （記憶から戻した窓）は動かさず、`Keep`（debug・理由の語）を残す（要件 3.5・7.6）。
/// 各形は同じ World の 1 か所だけを変えて作り、変える前は寄せられることを同じ捕捉で確かめる。
#[test]
fn explicitly_placed_or_remembered_windows_are_kept() {
    type Tweak = fn(&mut World, Entity);
    let cases: [(&str, Tweak); 4] = [
        ("not-default-pos", |w, _| {
            w.resource_mut::<GhostWindows>()
                .set_default_char_pos(SCOPE, point(900, 864));
        }),
        ("script-placed", |w, e| {
            w.entity_mut(e).insert(ScriptPlaced);
        }),
        ("dragging", |w, e| {
            w.entity_mut(e)
                .insert(dragging_state((1000, 864), (1200, 900)));
        }),
        ("not-default-pos", |w, _| {
            w.resource_mut::<GhostWindows>()
                .clear_default_char_pos(SCOPE);
        }),
    ];
    for (reason, tweak) in cases {
        let (mut world, window) = world_free(Some(desk()));
        let (before, _) = capture_logs(|| grow(&world, window, PlacementRoute::Resnap, GROWN));
        assert_ne!(
            before, DEFAULT_POS,
            "対照: 変える前は寄せられる（{reason}）"
        );
        tweak(&mut world, window);

        let (to, events) = capture_logs(|| grow(&world, window, PlacementRoute::Resnap, GROWN));

        assert_eq!(to, DEFAULT_POS, "{reason}");
        let keep = expect_one(&events, WORK_AREA_KEEP_TAG);
        assert_eq!(keep.level, tracing::Level::DEBUG);
        assert_eq!(keep.expect_field("reason"), format!("\"{reason}\""));
        assert!(tagged(&events, WORK_AREA_CLAMP_TAG).is_empty());
    }
}

/// 拡大率・吸着・作業領域の変化の経路と、`free` でない吸着・キャラ窓でない窓には当てず、記録もしない。
#[test]
fn other_routes_anchors_and_non_char_windows_are_untouched_and_silent() {
    let (mut world, window) = world_free(Some(desk()));
    let stray = world.spawn_empty().id();
    let (outs, events) = capture_logs(|| {
        let mut outs = Vec::new();
        for route in [
            PlacementRoute::DpiReproject,
            PlacementRoute::AnchorChange,
            PlacementRoute::WorkAreaResnap,
        ] {
            outs.push(grow(&world, window, route, GROWN));
        }
        outs.push(clamp_grown_free_default(
            &world,
            window,
            Anchor::Bottom,
            PlacementRoute::Resnap,
            DEFAULT_POS,
            GROWN,
        ));
        outs.push(grow(&world, stray, PlacementRoute::Resnap, GROWN));
        outs
    });
    assert!(outs.iter().all(|p| *p == DEFAULT_POS), "{outs:?}");
    assert!(
        tagged(&events, "[work-area-clamp]").is_empty(),
        "{:?}",
        events.iter().map(|e| e.message()).collect::<Vec<_>>()
    );
    // 対照: 同じ World で当てる経路なら寄せる（捕捉が生きている）。
    assert_ne!(
        grow(&world, window, PlacementRoute::Resnap, GROWN),
        DEFAULT_POS
    );
}

/// ⑸ 作業領域より大きい窓は左辺と上辺が作業領域の左辺と上辺に合う（要件 7.2）。
#[test]
fn oversized_window_aligns_left_and_top() {
    let (world, window) = world_free(Some(desk()));
    let to = grow(
        &world,
        window,
        PlacementRoute::Resnap,
        SizePx { w: 3000, h: 2000 },
    );
    assert_eq!(to, point(0, 0));
}

/// ⑺ 大きくなっても収まっていれば動かさず、`Keep`（reason=fits）を残す。
#[test]
fn grown_window_that_still_fits_is_kept() {
    let (world, window) = world_free(Some(desk()));
    let (to, events) = capture_logs(|| {
        grow(
            &world,
            window,
            PlacementRoute::Resnap,
            SizePx { w: 700, h: 840 },
        )
    });
    assert_eq!(to, DEFAULT_POS);
    assert_eq!(
        expect_one(&events, WORK_AREA_KEEP_TAG).expect_field("reason"),
        "\"fits\""
    );
    assert!(tagged(&events, WORK_AREA_CLAMP_TAG).is_empty());
}

/// ⑹ モニタの写しが無いと動かさず `Unresolved` の warn が出る（要件 7.4）。空の写しも同じ
/// （reason=no-work-area）。対照: 同じ捕捉窓で写しの有る World は `Clamp` を出す。
#[test]
fn missing_snapshot_warns_unresolved_with_contrast() {
    let (bare, w1) = world_free(None);
    let (empty, w2) = world_free(Some(MonitorSnapshot { work_areas: vec![] }));
    let (with_snap, w3) = world_free(Some(desk()));

    let ((a, b, c), events) = capture_logs(|| {
        (
            grow(&bare, w1, PlacementRoute::Resnap, GROWN),
            grow(&empty, w2, PlacementRoute::Resnap, GROWN),
            grow(&with_snap, w3, PlacementRoute::Resnap, GROWN),
        )
    });

    assert_eq!((a, b), (DEFAULT_POS, DEFAULT_POS));
    assert_ne!(c, DEFAULT_POS);
    let warns = tagged(&events, WORK_AREA_UNRESOLVED_TAG);
    assert_eq!(warns.len(), 2);
    assert!(warns.iter().all(|w| w.level == tracing::Level::WARN));
    assert_eq!(warns[0].expect_field("scene"), "\"surface-grow\"");
    assert_eq!(warns[0].expect_field("reason"), "\"no-monitor-snapshot\"");
    assert_eq!(warns[1].expect_field("reason"), "\"no-work-area\"");
    expect_one(&events, WORK_AREA_CLAMP_TAG);
}
