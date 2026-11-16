//! 1 回の判断（設計の PromotionRound）: 1 つの手動の持ち主の走査の列から、帰属・祖先の優先・相の移り・
//! 置き場の書き込み・描く列の分け方を決める。使い手の部品（`OwnSurface`・`GraphicsCommandList`・
//! `VisualDrawContent`・`VisualTransform`）には触らず、`AutoPromotion` と `SurfacePlacement` だけを書く。

// shortcut: 係から呼ぶのは 3.4。配線が入ったらこの許しを外す。
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashSet;
use std::iter::successors;

use bevy_ecs::prelude::*;

use super::placement::{PlacementClip, PlacementKind, SurfacePlacement};
use super::promotion::{AutoPromotion, DemoteReason, Phase, SurfaceReadiness, Verdict};
use super::promotion_shape::{
    Ineligible, Origin, ShapeInput, check_shape, owner_later_overlap, regrow, surface_rect,
};
use super::world_visual::{ClipClass, ReportedIssues, ScanEntry, ScanKind, TreeIssue, TreeScan};
use crate::ecs::graphics::components::SurfaceGraphics;
use crate::ecs::graphics::systems::format_entity_name;
use tracing::{debug, warn};

/// 1 回の判断が読み書きする部品（節は `AutoPromotion` を必ず持つ・手動の持ち主は持たない）。
pub(crate) type PromotionData = (
    Option<&'static mut AutoPromotion>,
    Option<&'static mut SurfacePlacement>,
    Option<&'static SurfaceGraphics>,
    Option<&'static Name>,
);

/// 1 回の判断の入力（1 つの手動の持ち主の回ぶん）。
pub(crate) struct RoundContext {
    /// フレームの時刻。無ければ判断しない（相を進めない）。
    pub now: Option<f64>,
    /// 装置と WUC の資源が在るか。
    pub device_ready: bool,
    /// 親の持ち主の拡大（`GlobalArrangement`）。
    pub scale: f32,
    /// 親の描画面の物理の大きさ。
    pub host_px: (u32, u32),
}

/// 自動の持ち主 1 つの部分の列。
pub(crate) struct Segment {
    pub owner: Entity,
    /// 部分の見える節（前順・深さは付け直さない）。
    pub visited: Vec<(Entity, usize)>,
    pub origin: Origin,
}

/// 描く列の分け方（1 つの手動の持ち主の回ぶん）。
pub(crate) struct RoundPlan {
    /// 親の描く列に入れる見える節（前順・深さつき）。Active の自動の持ち主の部分を除いたもの。
    pub host_visited: Vec<(Entity, usize)>,
    /// 部分の列を描く自動の持ち主（Preparing の 3 部品つき・Active）。
    pub segments: Vec<Segment>,
}

/// 1 つの手動の持ち主の走査の列 `scan`（同じ回に `fill_world_visuals` が埋めたもの）から、相を進め、
/// 置き場を値が変わったときだけ書き（書いた実体は `written` に記す・外すのは全部の根を回した後）、
/// 描く列の分け方を返す。
///
/// - 帰属: 更新のあった見える節を、自分か祖先のいちばん近い自動の持ち主（用意中・上げた）へ、
///   無ければ自分へ数える。帰属先ごとにこの回 1 回の更新。
/// - 判断するのは、この回に更新のあった帰属先とこの走査の列に居る自動の持ち主だけ（時刻が無い回はしない）。
/// - 前順に判断し、上げると答えた候補の部分木の中の節は上げず、自動の持ち主は同じ回に吸い込みで戻す。
/// - 置き場は節の下の手動の持ち主（`Embedded`）と自動の持ち主（`Auto`）に書く。
/// - 記録: 相の移りの回だけ `debug!` を 1 行、見送りは理由が前の見送りと違うときだけ。節の下の手動の
///   持ち主の効かせられない形は `reported` で種類ごとに続く間 1 度 `warn!`。答えはログの水準で変えない。
/// - 部分の列を描くのは Preparing の 3 部品つき・Active の自動の持ち主（判断の回は親の列にだけ残す）。
/// - 事後: 上げた持ち主の部分は親の列に無く部分の列に在る。3 部品つきの用意中の部分は両方に在る。
pub(crate) fn run_round(
    ctx: &RoundContext,
    scan: &TreeScan,
    states: &mut Query<PromotionData>,
    commands: &mut Commands,
    written: &mut HashSet<Entity>,
    reported: &mut ReportedIssues,
) -> RoundPlan {
    let entries = &scan.entries;
    let input = |idx| ShapeInput {
        scan,
        idx,
        scale: ctx.scale,
        host_px: ctx.host_px,
    };
    let mut phases: Vec<Phase> = entries.iter().map(phase_of).collect();
    if let Some(now) = ctx.now {
        decide(ctx, now, scan, &mut phases, states);
    }
    let mut origins: Vec<Option<Origin>> = vec![None; entries.len()];
    for (i, e) in entries.iter().enumerate() {
        let kind = match e.kind {
            ScanKind::Owner { parent_world } if e.parent.is_some() => {
                let clip = placement_clip(entries, i);
                let name = states.get(e.entity).ok().and_then(|d| d.3);
                report_unhonored(&input(i), &clip, name, reported);
                PlacementKind::Embedded {
                    parent_world,
                    opacity: e.ancestors.opacity,
                    visible: e.visible,
                    clip,
                }
            }
            ScanKind::Node { .. } if phases[i] != Phase::Node => {
                let stored = match states.get(e.entity) {
                    Ok((
                        _,
                        Some(SurfacePlacement(PlacementKind::Auto { origin_px, size_px })),
                        _,
                        _,
                    )) => Some(Origin {
                        px: *origin_px,
                        dip: (
                            origin_px.0 as f32 / ctx.scale,
                            origin_px.1 as f32 / ctx.scale,
                        ),
                        size_px: *size_px,
                    }),
                    _ => None,
                };
                // 上げた持ち主の矩形ははみ出したときだけ決め直す。用意中は毎回この回の範囲から。
                let origin = match (phases[i], stored) {
                    (Phase::Active, Some(cur)) => Some(regrow(&input(i), &cur).unwrap_or(cur)),
                    _ => surface_rect(&input(i)),
                };
                // 0×0 の描画面は作らない（部分の列も描かない）。
                let Some(o) = origin.filter(|o| o.size_px.0 > 0 && o.size_px.1 > 0) else {
                    continue;
                };
                origins[i] = Some(o);
                PlacementKind::Auto {
                    origin_px: o.px,
                    size_px: o.size_px,
                }
            }
            _ => continue,
        };
        written.insert(e.entity);
        match states.get_mut(e.entity) {
            Ok((_, Some(mut p), _, _)) => {
                p.set_if_neq(SurfacePlacement(kind));
            }
            Ok((_, None, _, _)) => {
                commands.entity(e.entity).insert(SurfacePlacement(kind));
            }
            Err(_) => {} // 消えた実体
        }
    }
    let cut = |j: usize| phases[j] == Phase::Active && origins[j].is_some();
    // 3 部品の無い用意中の部分を書けば、built が描画面の作成より先に立ち、作る前の無効な描画面で戻してしまう。
    let draws = |i: usize| {
        phases[i] == Phase::Active
            || matches!(states.get(entries[i].entity), Ok((_, _, Some(_), _)))
    };
    RoundPlan {
        host_visited: drawn(entries, 0, entries.len(), cut),
        segments: origins
            .iter()
            .enumerate()
            .filter(|&(i, _)| draws(i))
            .filter_map(|(i, o)| {
                o.map(|origin| Segment {
                    owner: entries[i].entity,
                    visited: drawn(entries, i, entries[i].end, |j| j != i && cut(j)),
                    origin,
                })
            })
            .collect(),
    }
}

/// 帰属・判断・祖先の優先と吸い込み。`phases` を判断の後の相へ書き換える。
fn decide(
    ctx: &RoundContext,
    now: f64,
    scan: &TreeScan,
    phases: &mut [Phase],
    states: &mut Query<PromotionData>,
) {
    let entries = &scan.entries;
    let mut updated = vec![false; entries.len()];
    for i in (0..entries.len()).filter(|&i| entries[i].updated) {
        let to = successors(Some(i), |&k| entries[k].parent)
            .find(|&k| phases[k] != Phase::Node)
            .unwrap_or(i);
        updated[to] = true;
    }
    // 前順なので祖先が先。新しく上げた候補の部分木 [候補, promoted_end) の中は上げない。
    let mut promoted_end = 0;
    for (i, e) in entries.iter().enumerate() {
        let ScanKind::Node { phase } = e.kind else {
            continue;
        };
        if !updated[i] && phase == Phase::Node {
            continue;
        }
        let Ok((Some(mut auto), _, surface, name)) = states.get_mut(e.entity) else {
            continue; // 消えた実体
        };
        // 履歴だけの書き込みは変化の印を立てない。相が変わった回だけ立てる。
        let state = auto.bypass_change_detection();
        let before = state.rule.phase();
        let verdict = if i < promoted_end {
            state.rule.absorb(now)
        } else {
            let readiness = readiness(ctx.device_ready, state.built, surface);
            let input = ShapeInput {
                scan,
                idx: i,
                scale: ctx.scale,
                host_px: ctx.host_px,
            };
            let shape = || {
                if before == Phase::Node && !ctx.device_ready {
                    return Err(Ineligible::NoDevice);
                }
                // 描く中身の無い部分木は描画面にしない（0×0 の描画面を作らない）。
                check_shape(&input).and_then(|o| {
                    if o.size_px.0 == 0 || o.size_px.1 == 0 {
                        Err(Ineligible::Hidden)
                    } else {
                        Ok(())
                    }
                })
            };
            state.rule.step(now, updated[i], shape, readiness)
        };
        if verdict == Verdict::Prepare {
            promoted_end = e.end;
        }
        record(e.entity, name, verdict, &mut state.skip_memo);
        let after = state.rule.phase();
        if after != before {
            state.built = false;
            auto.set_changed();
        }
        phases[i] = after;
    }
}

/// 用意中の描画面の様子: 装置が無ければ作れない。3 部品がまだ無いか部分の列をまだ描いていなければ待つ。
/// 描いた次の回に描画面が有効ならできた、無効なら作れなかった。
fn readiness(
    device_ready: bool,
    built: bool,
    surface: Option<&SurfaceGraphics>,
) -> SurfaceReadiness {
    match surface {
        _ if !device_ready => SurfaceReadiness::Failed,
        Some(s) if built => {
            if s.is_valid() {
                SurfaceReadiness::Ready
            } else {
                SurfaceReadiness::Failed
            }
        }
        _ => SurfaceReadiness::Pending,
    }
}

/// 相の移りの回だけ 1 行、見送りは理由が前の見送りと違うときだけ残す（要件 7.1〜7.3）。
/// `skip_memo` は水準によらず進める（答えをログの水準で変えない・要件 1.8）。
fn record(e: Entity, name: Option<&Name>, verdict: Verdict, skip_memo: &mut Option<Ineligible>) {
    let entity = || format_entity_name(e, name);
    match verdict {
        Verdict::Prepare => {
            *skip_memo = None;
            debug!(entity = %entity(), reason = "frequent", "[surface_promotion] promoted");
        }
        Verdict::Activate => debug!(entity = %entity(), "[surface_promotion] activated"),
        Verdict::Demote(DemoteReason::Shape(s)) if s != Ineligible::Hidden => debug!(
            entity = %entity(),
            reason = "shape",
            shape = s.as_str(),
            "[surface_promotion] demoted"
        ),
        Verdict::Demote(r) => {
            let reason = match r {
                DemoteReason::Quiet => "quiet",
                DemoteReason::SurfaceFailed => "surface_failed",
                DemoteReason::Absorbed => "absorbed",
                DemoteReason::Shape(_) => "hidden",
            };
            debug!(entity = %entity(), reason, "[surface_promotion] demoted");
        }
        Verdict::Skip(s) if *skip_memo != Some(s) => {
            *skip_memo = Some(s);
            debug!(entity = %entity(), shape = s.as_str(), "[surface_promotion] skipped");
        }
        Verdict::Skip(_) | Verdict::Stay => {}
    }
}

/// 節の下の手動の持ち主 `input.idx` の効かせられない形を、種類ごとに続く間 1 度 `warn!` に残す（要件 4.9）。
fn report_unhonored(
    input: &ShapeInput,
    clip: &PlacementClip,
    name: Option<&Name>,
    reported: &mut ReportedIssues,
) {
    let entries = &input.scan.entries;
    let e = &entries[input.idx];
    if !e.visible {
        return;
    }
    let kinds = [
        ("geometry_clip", ancestor_shaped(entries, input.idx)),
        ("rotated_clip", e.ancestors.rect_clip_rotated),
        ("blend", e.ancestors.blend),
        ("later_overlap", owner_later_overlap(input)),
        ("clip_combo", clip_combo(e, clip)),
    ];
    for (kind, _) in kinds.into_iter().filter(|k| k.1) {
        if reported.first_seen(e.entity, TreeIssue::Unhonored(kind)) {
            warn!(
                entity = %format_entity_name(e.entity, name),
                kind,
                "[surface_promotion] 節の下の持ち主に効かせられない形"
            );
        }
    }
}

/// 自分の切り抜きと置き場の矩形の切り抜きを 1 つに合わせられないか（`placement::merged_clip` が
/// `Unsupported` を返す条件と同じ。`content_world_aabb` は局所の矩形を親の持ち主の DIP へ移したもの）。
fn clip_combo(e: &ScanEntry, clip: &PlacementClip) -> bool {
    let PlacementClip::Rect(host) = clip else {
        return false;
    };
    let m = e.world.world_mat;
    let det = m.M11 * m.M22 - m.M12 * m.M21;
    if e.own_clip == ClipClass::None || !det.is_finite() || det.abs() < 1e-12 {
        return false;
    }
    let b = e.world.content_world_aabb;
    let kept = matches!(e.own_clip, ClipClass::Rect | ClipClass::Rounded)
        && host.left() <= b.left()
        && host.top() <= b.top()
        && host.right() >= b.right()
        && host.bottom() >= b.bottom();
    !kept && (!e.world.axis_aligned || e.own_clip != ClipClass::Rect)
}

fn phase_of(e: &ScanEntry) -> Phase {
    match e.kind {
        ScanKind::Node { phase } => phase,
        ScanKind::Owner { .. } => Phase::Node,
    }
}

/// 節の下の手動の持ち主の祖先の切り抜き（祖先の角丸・図形・軸に沿わない矩形は合わせられない）。
fn placement_clip(entries: &[ScanEntry], i: usize) -> PlacementClip {
    let acc = entries[i].ancestors;
    if ancestor_shaped(entries, i) || acc.rect_clip_rotated {
        PlacementClip::Unsupported
    } else {
        acc.rect_clip
            .map_or(PlacementClip::None, PlacementClip::Rect)
    }
}

/// 祖先の節に角丸か任意の図形の切り抜きが在るか。
fn ancestor_shaped(entries: &[ScanEntry], i: usize) -> bool {
    successors(entries[i].parent, |&p| entries[p].parent).any(|p| {
        matches!(
            entries[p].own_clip,
            ClipClass::Rounded | ClipClass::Geometry
        )
    })
}

/// `[from, to)` の見える節（前順・深さつき）。`cut` が真の項の部分木は飛ばす。
fn drawn(
    entries: &[ScanEntry],
    from: usize,
    to: usize,
    cut: impl Fn(usize) -> bool,
) -> Vec<(Entity, usize)> {
    let mut out = Vec::new();
    let mut j = from;
    while j < to {
        if cut(j) {
            j = entries[j].end;
            continue;
        }
        if entries[j].is_visible_node() {
            out.push((entries[j].entity, entries[j].depth));
        }
        j += 1;
    }
    out
}
