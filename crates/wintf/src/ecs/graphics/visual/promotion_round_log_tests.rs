//! 1 回の判断の記録のテスト（要件 1.8, 4.9, 7.1〜7.4）。記録は `capture_under_filter` で捕まえる。
//! 上げ下げは移りの回だけ 1 行・見送りは理由が変わるまで 1 度・効かせられない形の `warn!` は続く間 1 度、
//! ログの水準を変えても判断の答えが同じことを確かめる。

use bevy_ecs::prelude::*;

use super::promotion::*;
use super::promotion_round::RoundContext;
use super::promotion_round_tests::{
    ctx, ent, feed, only_updated, ph, ready_surface, round, round_with, world_scan,
};
use super::promotion_shape::Ineligible;
use super::promotion_shape_tests::{node, owner, phased};
use super::world_visual::{ClipClass, ReportedIssues};
use crate::ecs::graphics::components::SurfaceGraphics;
use crate::ecs::test_support::capture_under_filter;
use crate::numerics::Aabb;

const PROMOTED: &str = "[surface_promotion] promoted";
const ACTIVATED: &str = "[surface_promotion] activated";
const DEMOTED: &str = "[surface_promotion] demoted";
const SKIPPED: &str = "[surface_promotion] skipped";
const UNHONORED: &str = "[surface_promotion] 節の下の持ち主に効かせられない形";

fn lines(log: &str, what: &str) -> usize {
    log.lines().filter(|l| l.contains(what)).count()
}

#[test]
fn promotion_and_activation_log_one_line_only_on_the_transition_frame() {
    let mut world = World::new();
    let mut s = world_scan(&mut world, vec![node(1, 10.0, 10.0, 20.0, 20.0)]);
    let a = ent(&s, 0);
    world.entity_mut(a).insert(Name::new("動く節"));
    only_updated(&mut s, &[0]);
    for k in 0..PROMOTE_COUNT + 3 {
        let before = ph(&world, a);
        let log = capture_under_filter("debug", || {
            round(&mut world, &mut s, &ctx(k as f64 * 0.125));
        });
        let moved = ph(&world, a) != before;
        assert_eq!(lines(&log, PROMOTED), usize::from(moved), "回 {k}: {log}");
        if moved {
            assert!(log.contains("動く節") && log.contains("frequent"), "{log}");
        }
    }
    assert_eq!(ph(&world, a), Phase::Preparing);

    // 部分の列を描いた次の回に描画面ができていれば切り替える（1 行）。続く回は残さない。
    let (_resource, surface) = ready_surface();
    world.entity_mut(a).insert(surface);
    world.get_mut::<AutoPromotion>(a).unwrap().built = true;
    let log = capture_under_filter("debug", || {
        round(&mut world, &mut s, &ctx(2.0));
        round(&mut world, &mut s, &ctx(2.125));
    });
    assert_eq!(ph(&world, a), Phase::Active);
    assert_eq!(lines(&log, ACTIVATED), 1, "{log}");
    assert_eq!(lines(&log, "[surface_promotion]"), 1, "{log}");
}

#[test]
fn demotion_logs_one_line_with_the_reason_word() {
    // 上げたは 1.0 秒に切り替えた（最後の更新 0.875 秒）。落ち着きの戻しは 4.0 秒。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active)],
    );
    let log = capture_under_filter("debug", || {
        for k in 0..10 {
            round(&mut world, &mut s, &ctx(1.5 + k as f64 * 0.5));
        }
    });
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Node);
    assert_eq!(lines(&log, "[surface_promotion]"), 1, "{log}");
    assert!(lines(&log, DEMOTED) == 1 && log.contains("quiet"), "{log}");

    // 見えなくなった・後の中身と重なった・描画面を作れなかった。
    let mut hidden = phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active);
    hidden.visible = false;
    let cases = [
        (vec![hidden], false, "hidden"),
        (
            vec![
                phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Active),
                node(1, 15.0, 15.0, 25.0, 25.0),
            ],
            false,
            "later_content_overlap",
        ),
        (
            vec![phased(node(1, 10.0, 10.0, 20.0, 20.0), Phase::Preparing)],
            true,
            "surface_failed",
        ),
    ];
    for (entries, failed_surface, word) in cases {
        let mut world = World::new();
        let mut s = world_scan(&mut world, entries);
        if failed_surface {
            let a = ent(&s, 0);
            world.entity_mut(a).insert(SurfaceGraphics::default());
            world.get_mut::<AutoPromotion>(a).unwrap().built = true;
        }
        let log = capture_under_filter("debug", || {
            round(&mut world, &mut s, &ctx(1.5));
            round(&mut world, &mut s, &ctx(1.625));
        });
        assert_eq!(ph(&world, ent(&s, 0)), Phase::Node, "{word}");
        assert_eq!(lines(&log, DEMOTED), 1, "{word}: {log}");
        assert!(log.contains(word), "{word}: {log}");
    }

    // 祖先が上がった回に吸い込まれた。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            node(1, 10.0, 10.0, 30.0, 30.0),
            phased(node(2, 12.0, 12.0, 16.0, 16.0), Phase::Active),
        ],
    );
    let log = capture_under_filter("debug", || {
        feed(&mut world, &mut s, &[0], 1.0, PROMOTE_COUNT);
    });
    assert_eq!(ph(&world, ent(&s, 1)), Phase::Node);
    assert_eq!(lines(&log, PROMOTED), 1, "{log}");
    assert!(
        lines(&log, DEMOTED) == 1 && log.contains("absorbed"),
        "{log}"
    );
}

#[test]
fn skip_logs_once_while_the_same_reason_continues() {
    // 後に描く節と重なるので、頻繁になった回から毎回見送る。
    let mut world = World::new();
    let mut s = world_scan(
        &mut world,
        vec![
            node(1, 10.0, 10.0, 20.0, 20.0),
            node(1, 15.0, 15.0, 25.0, 25.0),
        ],
    );
    let log = capture_under_filter("debug", || {
        feed(&mut world, &mut s, &[0], 0.0, PROMOTE_COUNT + 6);
    });
    assert_eq!(ph(&world, ent(&s, 0)), Phase::Node);
    assert_eq!(lines(&log, SKIPPED), 1, "{log}");
    assert!(log.contains("later_content_overlap"), "{log}");

    // 頻繁なまま理由が変わった回にだけまた残す（装置が無い → 重なりへ戻る）。
    let log = capture_under_filter("debug", || {
        for k in 0..3 {
            let c = RoundContext {
                device_ready: false,
                ..ctx(1.75 + k as f64 * 0.125)
            };
            round(&mut world, &mut s, &c);
        }
        feed(&mut world, &mut s, &[0], 2.125, 3);
    });
    assert_eq!(lines(&log, SKIPPED), 2, "{log}");
    assert_eq!(lines(&log, "no_device"), 1, "{log}");
    assert_eq!(lines(&log, "later_content_overlap"), 1, "{log}");
}

#[test]
fn unhonored_shape_under_a_node_warns_once_while_it_persists() {
    let plain = || node(1, 0.0, 0.0, 50.0, 50.0);
    let below = || owner(2, 10.0, 10.0, 20.0, 20.0);
    let mut rounded_parent = plain();
    rounded_parent.own_clip = ClipClass::Rounded;
    let mut rotated = below();
    rotated.ancestors.rect_clip_rotated = true;
    let mut blend = below();
    blend.ancestors.blend = true;
    let mut combo = below();
    combo.own_clip = ClipClass::Rounded;
    combo.ancestors.rect_clip = Some(Aabb::new(0.0, 0.0, 15.0, 15.0));
    let cases = [
        (vec![rounded_parent, below()], "geometry_clip"),
        (vec![plain(), rotated], "rotated_clip"),
        (vec![plain(), blend], "blend"),
        (
            vec![plain(), below(), node(1, 15.0, 15.0, 25.0, 25.0)],
            "later_overlap",
        ),
        (vec![plain(), combo], "clip_combo"),
    ];
    for (entries, word) in cases {
        let mut world = World::new();
        let mut s = world_scan(&mut world, entries);
        world.entity_mut(ent(&s, 1)).insert(Name::new("下の持ち主"));
        let mut reported = ReportedIssues::default();
        let log = capture_under_filter("warn", || {
            for k in 0..3 {
                round_with(&mut world, &mut s, &ctx(k as f64), &mut reported);
            }
        });
        assert_eq!(lines(&log, UNHONORED), 1, "{word}: {log}");
        assert!(log.contains(word) && log.contains("下の持ち主"), "{log}");

        // 形が消えた回の後にまた現れたら、もう 1 度残す。
        let kept = s.entries.clone();
        s.entries[0].own_clip = ClipClass::None;
        s.entries[1].own_clip = ClipClass::None;
        s.entries[1].ancestors = below().ancestors;
        s.entries.truncate(2);
        let log = capture_under_filter("warn", || {
            round_with(&mut world, &mut s, &ctx(3.0), &mut reported);
        });
        assert_eq!(lines(&log, UNHONORED), 0, "{word}: {log}");
        s.entries = kept;
        let log = capture_under_filter("warn", || {
            round_with(&mut world, &mut s, &ctx(4.0), &mut reported);
        });
        assert_eq!(lines(&log, UNHONORED), 1, "{word}: {log}");
    }
}

#[test]
fn plain_or_root_owners_do_not_warn() {
    let mut contained = owner(2, 10.0, 10.0, 20.0, 20.0);
    contained.own_clip = ClipClass::Rounded;
    contained.ancestors.rect_clip = Some(Aabb::new(0.0, 0.0, 50.0, 50.0));
    let mut root_blend = owner(1, 10.0, 10.0, 20.0, 20.0);
    root_blend.ancestors.blend = true;
    for entries in [
        vec![
            node(1, 0.0, 0.0, 50.0, 50.0),
            owner(2, 10.0, 10.0, 20.0, 20.0),
        ],
        vec![node(1, 0.0, 0.0, 50.0, 50.0), contained],
        vec![root_blend, node(1, 15.0, 15.0, 25.0, 25.0)],
    ] {
        let mut world = World::new();
        let mut s = world_scan(&mut world, entries);
        let log = capture_under_filter("warn", || {
            round(&mut world, &mut s, &ctx(0.0));
        });
        assert_eq!(lines(&log, UNHONORED), 0, "{log}");
    }
}

#[test]
fn answers_do_not_depend_on_the_log_level() {
    // 見送り・上げ・吸い込みを含む並びを、水準を変えて同じに回す。
    let run = |filter: &str| {
        let mut world = World::new();
        let mut s = world_scan(
            &mut world,
            vec![
                node(1, 10.0, 10.0, 30.0, 30.0),
                phased(node(2, 12.0, 12.0, 16.0, 16.0), Phase::Active),
                node(1, 40.0, 40.0, 50.0, 50.0),
                node(1, 45.0, 45.0, 55.0, 55.0),
            ],
        );
        let mut trace = Vec::new();
        capture_under_filter(filter, || {
            only_updated(&mut s, &[0, 2]);
            for k in 0..PROMOTE_COUNT + 4 {
                let (plan, written) = round(&mut world, &mut s, &ctx(1.0 + k as f64 * 0.125));
                let phases: Vec<_> = (0..4).map(|i| ph(&world, ent(&s, i))).collect();
                let memo: Vec<_> = (0..4)
                    .map(|i| world.get::<AutoPromotion>(ent(&s, i)).unwrap().skip_memo)
                    .collect();
                trace.push((phases, memo, plan.host_visited, written.len()));
            }
        });
        trace
    };
    let quiet = run("off");
    assert_eq!(quiet, run("trace"));
    let last = quiet.last().unwrap();
    assert_eq!(last.0[0], Phase::Preparing);
    assert_eq!(last.0[1], Phase::Node, "吸い込み");
    assert_eq!(last.1[2], Some(Ineligible::LaterContentOverlap));
}
