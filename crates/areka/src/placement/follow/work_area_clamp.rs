//! キャラ窓を作業領域へ寄せる判定と記録（window-work-area-clamp・設計
//! 「配置の層: placement/follow/work_area_clamp.rs」）。
//!
//! - 場面の語 [`ClampScene`] と記録の語（`[work-area-clamp] Clamp`／`Keep`／`Unresolved`）を
//!   1 か所に置く（要件 9.1・9.2・7.4）
//! - [`resolve_chain_in_work_area`] は組ごとの決めで解いた鎖を、前の窓の置いた後の矩形が属する
//!   モニタの作業領域の中に置く純関数の核（要件 1・2・7）
//! - [`place_chain_in_work_area`] はその World の口。窓の高さと作業領域を World から読んで核を呼び、
//!   寄せた・寄せなかった・取れなかったを記録する（要件 7.4・9.1・9.2）
//! - [`clamp_grown_free_default`] は既定の位置にいる `free` のキャラ窓が大きくなってはみ出したら、
//!   はみ出した向きにだけ収める判定（要件 3・7）
//! - [`keep_restored_groups`] は記憶から戻した組の窓を同じ量だけ動かす判定（要件 4・7）
//!
//! 寄せの式は `balloon_limit::clamp_rect_to_work_area`（左辺・上辺を優先）だけを使い、
//! 隣の位置は `chain_finalize::resolve_chain` に 2 スロットを渡して得る（式を写さない）。
//! 判定の入力は窓の幾何と組の決めだけで、ゴーストや初回起動を名指ししない（要件 7.8）。
//!
//! examples（`collision-probe`・`window-placement`）は `#[path]` で配置の層を取り込むが
//! 呼び手（起動の並べ直し・詰め直し）を持たないので、そちらのビルドでは本ファイルの項目が
//! 未使用に見える。ゆえに項目ごとに `#[allow(dead_code)]` を付ける。

use bevy_ecs::prelude::{Entity, World};
use tracing::{debug, info, warn};
use wintf::ecs::WindowPos;
use wintf::ecs::drag::DraggingState;

use super::super::balloon_limit::clamp_rect_to_work_area;
use super::super::chain_finalize::{
    Adjacency, AdjacencyMark, ChainRect, ChainSlot, KeepReason, PairDecision, ScriptPlaced,
    judge_adjacency, resolve_chain,
};
use super::{
    Anchor, CharWindowMarker, GhostWindows, MonitorSnapshot, PlacementRoute, PointPx, RectPx,
    SizePx, limit_correction, rect_at, work_area_for_window,
};

/// 寄せて動かした（info!）。
#[allow(dead_code)] // 出す口は task 1.2 の World の口・examples には呼び手が無い
pub(crate) const WORK_AREA_CLAMP_TAG: &str = "[work-area-clamp] Clamp";
/// 判定したが動かさなかった（debug!）。
#[allow(dead_code)] // 同上
pub(crate) const WORK_AREA_KEEP_TAG: &str = "[work-area-clamp] Keep";
/// 作業領域が取れず寄せなかった（warn!）。
#[allow(dead_code)] // 同上
pub(crate) const WORK_AREA_UNRESOLVED_TAG: &str = "[work-area-clamp] Unresolved";

/// 寄せの場面（記録の語彙・判断は変えない）。
#[allow(dead_code)] // 場面を選ぶ呼び手は起動の並べ直しと詰め直し・examples には無い
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClampScene {
    /// 起動の並べ直し（記憶に位置の無い窓）。
    BootRepack,
    /// 記憶からの隣り合わせ。
    RememberedJoin,
    /// 拡大率の後の詰め直し。
    DpiRealign,
    /// 面の切り替え。
    SurfaceGrow,
    /// 記憶からの組の戻し。
    RestoreGroup,
}

#[allow(dead_code)] // 同上
impl ClampScene {
    /// 記録に載せる場面の語。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::BootRepack => "boot-repack",
            Self::RememberedJoin => "remembered-join",
            Self::DpiRealign => "dpi-realign",
            Self::SurfaceGrow => "surface-grow",
            Self::RestoreGroup => "restore-group",
        }
    }
}

/// 鎖の 1 スロットをどう置いたか。
#[allow(dead_code)] // 読む呼び手は task 1.2 以降・examples には無い
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChainClampOutcome {
    /// 並べた位置のまま（y だけ収めた場合を含む）。
    Fits,
    /// 反対の側へ並べ替えた。
    Flipped,
    /// どちらの側にも入らず端へ寄せた（基準の窓と重なってよい）。
    Edge,
    /// 組の決めが Keep＝寄せの対象外（今の位置のまま）。
    Kept(KeepReason),
    /// 作業領域が取れず、並べた位置のまま。
    Unresolved,
}

/// 鎖の 1 スロットの置き場（呼び手が書く値と記録の値）。
#[allow(dead_code)] // 同上
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChainPlacement {
    pub scope: usize,
    /// 今の左上。
    pub from: PointPx,
    /// 並べた位置（寄せる前）。
    pub joined: PointPx,
    /// 置く左上。
    pub to: PointPx,
    pub outcome: ChainClampOutcome,
}

/// 純関数の核。`slots` と `rects` は同じ長さ・同じ順（スコープ昇順）で、`rects[i]` は
/// `slots[i]` の今の矩形（`x == current_x`・`w == width`）。先頭を除く各スロットの置き場を返す。
///
/// 2 番目から昇順に置き、基準は前の窓の**置いた後**の矩形とそのモニタの作業領域
/// （要件 1.1・1.5・2.1）。並べた位置が横に入らなければ反対の側へ並べ替え、それも入らなければ
/// 並べた位置を作業領域の端へ寄せる（要件 1.2）。Keep は今の位置のまま（要件 1.4・2.3）、
/// 作業領域が取れなければ並べた位置のまま `Unresolved`（要件 7.4）。
///
/// 事前条件: `mark_non_positive_widths` を通した後の `slots`。長さが違えば短い方に揃える。
#[allow(dead_code)] // 呼び手は task 1.2 の World の口・examples には無い
pub(crate) fn resolve_chain_in_work_area(
    slots: &[ChainSlot],
    rects: &[ChainRect],
    snapshot: &MonitorSnapshot,
) -> Vec<ChainPlacement> {
    resolve_chain_with(slots, rects, snapshot, |_| true)
}

/// 核の本体。`height_known(i)` が偽のスロットは作業領域を引かず `Unresolved` で並べた位置に置く
/// （窓の高さが読めない・World の口だけが使う）。
fn resolve_chain_with(
    slots: &[ChainSlot],
    rects: &[ChainRect],
    snapshot: &MonitorSnapshot,
    height_known: impl Fn(usize) -> bool,
) -> Vec<ChainPlacement> {
    let n = slots.len().min(rects.len());
    let mut out = Vec::with_capacity(n.saturating_sub(1));
    // 前の窓の置いた後の矩形（先頭は今の矩形のまま・要件 1.3）。
    let mut base = match rects.first() {
        Some(r) => *r,
        None => return out,
    };
    for i in 1..n {
        let (s, r) = (slots[i], rects[i]);
        let from = PointPx { x: r.x, y: r.y };
        let size = SizePx { w: r.w, h: r.h };
        let placed = match s.decision {
            PairDecision::Keep(reason) => (from, from, ChainClampOutcome::Kept(reason)),
            decision => {
                let joined = PointPx {
                    x: neighbor_x(slots[i - 1], base, s, decision),
                    y: r.y,
                };
                let area = height_known(i)
                    .then(|| work_area_for_window(snapshot, rect_of(base)))
                    .flatten();
                match area {
                    None => (joined, joined, ChainClampOutcome::Unresolved),
                    Some(area) => {
                        // 横に入るか＝式を当てて x が変わらないか（式は 1 本）。入れば y だけ収まる。
                        let fit = |pos: PointPx| {
                            let c = clamp_rect_to_work_area(pos, size, area);
                            (c.x == pos.x).then_some(c)
                        };
                        let flipped = PointPx {
                            x: neighbor_x(slots[i - 1], base, s, opposite(decision)),
                            y: r.y,
                        };
                        if let Some(to) = fit(joined) {
                            (joined, to, ChainClampOutcome::Fits)
                        } else if let Some(to) = fit(flipped) {
                            (joined, to, ChainClampOutcome::Flipped)
                        } else {
                            let to = clamp_rect_to_work_area(joined, size, area);
                            (joined, to, ChainClampOutcome::Edge)
                        }
                    }
                }
            }
        };
        let (joined, to, outcome) = placed;
        out.push(ChainPlacement {
            scope: s.scope,
            from,
            joined,
            to,
            outcome,
        });
        base = ChainRect {
            x: to.x,
            y: to.y,
            ..r
        };
    }
    out
}

/// World の口。`targets` は走査（`collect_chain_states`）の `(scope, char 窓, 今の位置)` で、
/// `slots` と同じ順。窓の高さを `WindowPos` から、作業領域を `MonitorSnapshot` 資源から読んで
/// 核を呼び、スロットごとに記録を 1 行出す: 置いた位置が並べた位置と違えば `Clamp`（info!）、
/// 収まっていた・動かさない窓は `Keep`（debug!）、モニタの写しが無い・窓の高さが読めない・
/// 作業領域が引けないときは `Unresolved`（warn!・ほかのスロットは続ける・要件 7.4）。
#[allow(dead_code)] // 呼び手は起動の並べ直しと詰め直し（task 2.x）・examples には無い
pub(crate) fn place_chain_in_work_area(
    world: &World,
    slots: &[ChainSlot],
    targets: &[(usize, Entity, PointPx)],
    scene_of: impl Fn(usize) -> ClampScene,
) -> Vec<ChainPlacement> {
    let heights: Vec<Option<i32>> = targets
        .iter()
        .map(|&(_, e, _)| {
            world
                .get::<WindowPos>(e)
                .and_then(|wp| wp.size)
                .map(|s| s.height)
        })
        .collect();
    let rects: Vec<ChainRect> = slots
        .iter()
        .zip(targets)
        .zip(&heights)
        .map(|((s, &(_, _, pos)), h)| ChainRect {
            x: s.current_x,
            y: pos.y,
            w: s.width,
            h: h.unwrap_or(0),
        })
        .collect();
    let empty = MonitorSnapshot { work_areas: vec![] };
    let snapshot = world.get_resource::<MonitorSnapshot>();
    let out = resolve_chain_with(slots, &rects, snapshot.unwrap_or(&empty), |i| {
        heights[i].is_some()
    });

    let mut base = match rects.first() {
        Some(r) => *r,
        None => return out,
    };
    for (i, (p, r)) in out.iter().zip(&rects[1..]).enumerate() {
        let scene = scene_of(p.scope).as_str();
        let (scope, from, to) = (p.scope, p.from, p.to);
        let how = match p.outcome {
            ChainClampOutcome::Fits => "fits",
            ChainClampOutcome::Flipped => "flipped",
            ChainClampOutcome::Edge => "edge",
            ChainClampOutcome::Kept(reason) => {
                debug!(scene, scope, reason = reason.as_str(), at = ?from, "{WORK_AREA_KEEP_TAG} 組の決めで動かさない窓");
                base = ChainRect {
                    x: to.x,
                    y: to.y,
                    ..*r
                };
                continue;
            }
            ChainClampOutcome::Unresolved => {
                let reason = if snapshot.is_none() {
                    "no-monitor-snapshot"
                } else if heights[i + 1].is_none() {
                    "no-window-height"
                } else {
                    "no-work-area"
                };
                warn!(scene, scope, reason, at = ?to, "{WORK_AREA_UNRESOLVED_TAG} 作業領域が取れず寄せずに並べた位置へ置く");
                base = ChainRect {
                    x: to.x,
                    y: to.y,
                    ..*r
                };
                continue;
            }
        };
        if to != p.joined {
            let work_area = work_area_for_window(snapshot.unwrap_or(&empty), rect_of(base));
            info!(scene, scope, how, joined = ?p.joined, from = ?from, to = ?to, work_area = ?work_area, "{WORK_AREA_CLAMP_TAG} キャラ窓を作業領域の中へ寄せた");
        } else {
            debug!(scene, scope, reason = "fits", at = ?to, "{WORK_AREA_KEEP_TAG} 並べた位置が作業領域に収まっている");
        }
        base = ChainRect {
            x: to.x,
            y: to.y,
            ..*r
        };
    }
    out
}

/// 鎖の矩形を帰属判定の矩形へ（飽和演算）。
fn rect_of(r: ChainRect) -> RectPx {
    RectPx {
        left: r.x,
        top: r.y,
        right: r.x.saturating_add(r.w),
        bottom: r.y.saturating_add(r.h),
    }
}

/// 隣の x を今の隣り合いの解き（`resolve_chain`）に 2 スロットを渡して得る（式を写さない）。
/// 前の窓は置いた後の x と幅で渡す。動かさない解き（非正寸）は今の x のまま。
fn neighbor_x(prev: ChainSlot, base: ChainRect, cur: ChainSlot, decision: PairDecision) -> i32 {
    let pair = [
        ChainSlot {
            current_x: base.x,
            width: base.w,
            ..prev
        },
        ChainSlot { decision, ..cur },
    ];
    resolve_chain(&pair)
        .first()
        .map_or(cur.current_x, |m| m.new_x)
}

/// 反対の側の決め。
fn opposite(decision: PairDecision) -> PairDecision {
    match decision {
        PairDecision::JoinLeft => PairDecision::JoinRight,
        PairDecision::JoinRight => PairDecision::JoinLeft,
        keep => keep,
    }
}

/// 面の拡大の寄せ（設計 GrowClamp）。`resize_window_to` の可視性の関門の後の提案の位置を受け、
/// 収めた位置か提案のままを返す。World は読むだけ。
///
/// 当てるのは全部を満たす窓だけ: 吸着が `Free`・経路が `Resnap` か `ReportedSizeReconcile`・
/// キャラ窓・既定の位置が有り今の位置と同じ・`ScriptPlaced` が無い・`DraggingState` が無い
/// （要件 3.1・3.5・7.6）。収め方は今ある「はみ出した分だけの補正」（`limit_correction`）で、
/// はみ出していない向きの座標は変えない（要件 3.2）。作業領域は提案の位置に置いた窓のモニタ
/// （要件 7.1）、大きすぎる軸は左辺・上辺に合う（要件 7.2）。
///
/// 記録: 吸着が `Free` でない・経路が違う・キャラ窓でない窓は記録しない。`Free` のキャラ窓で
/// 動かさなかったものは `Keep`（debug!）、作業領域が取れなければ `Unresolved`（warn!・要件 7.4）、
/// 収めたら `Clamp`（info!・how=grown）。
#[allow(dead_code)] // 呼び手は `resize_window_to`（task 2.3）・examples には無い
pub(crate) fn clamp_grown_free_default(
    world: &World,
    char_window: Entity,
    anchor: Anchor,
    route: PlacementRoute,
    proposed: PointPx,
    new_size: SizePx,
) -> PointPx {
    // 拡大率・吸着・作業領域の変化の経路はそれぞれの持ち主が置き直す（設計 GrowClamp）。
    let grows = matches!(
        route,
        PlacementRoute::Resnap | PlacementRoute::ReportedSizeReconcile
    );
    if anchor != Anchor::Free || !grows {
        return proposed;
    }
    let Some(scope) = world.get::<CharWindowMarker>(char_window).map(|m| m.scope) else {
        return proposed;
    };
    let scene = ClampScene::SurfaceGrow.as_str();
    let current = world
        .get::<WindowPos>(char_window)
        .and_then(|wp| wp.position)
        .map(|p| PointPx { x: p.x, y: p.y });
    let default_pos = world
        .get_resource::<GhostWindows>()
        .and_then(|gw| gw.default_char_pos(scope));
    let keep = if current.is_none() || current != default_pos {
        Some("not-default-pos")
    } else if world.get::<ScriptPlaced>(char_window).is_some() {
        Some("script-placed")
    } else if world.get::<DraggingState>(char_window).is_some() {
        Some("dragging")
    } else {
        None
    };
    if let Some(reason) = keep {
        debug!(scene, scope, reason, at = ?proposed, "{WORK_AREA_KEEP_TAG} 既定の位置にいない・明示の操作中の窓は動かさない");
        return proposed;
    }
    let snapshot = world.get_resource::<MonitorSnapshot>();
    let work_area = snapshot.and_then(|s| work_area_for_window(s, rect_at(proposed, new_size)));
    let Some(area) = work_area else {
        let reason = if snapshot.is_none() {
            "no-monitor-snapshot"
        } else {
            "no-work-area"
        };
        warn!(scene, scope, reason, at = ?proposed, "{WORK_AREA_UNRESOLVED_TAG} 作業領域が取れず寄せずに提案の位置へ置く");
        return proposed;
    };
    match limit_correction(proposed, new_size, area) {
        Some(to) => {
            info!(scene, scope, how = "grown", from = ?proposed, to = ?to, work_area = ?work_area, "{WORK_AREA_CLAMP_TAG} 大きくなった窓を作業領域の中へ寄せた");
            to
        }
        None => {
            debug!(scene, scope, reason = "fits", at = ?proposed, "{WORK_AREA_KEEP_TAG} 大きくなった窓が作業領域に収まっている");
            proposed
        }
    }
}

/// 記憶から位置を戻した 1 窓（組の寄せの入力・設計 GroupClamp）。
#[allow(dead_code)] // 組む呼び手は記憶の戻し（task 2.4）・examples には無い
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RestoredWindow {
    pub scope: usize,
    /// 記憶の位置を今の大きさで置いた矩形（吸着の当て直しの前）。
    pub remembered: ChainRect,
    /// 吸着の当て直しだけをした左上（窓ごとの寄せの前）。
    pub projected: PointPx,
    /// 今の窓ごとの判定の結果（merge の `char_pos`）。
    pub shown: PointPx,
    /// 組 (scope−1, scope) の覚えた印。
    pub mark: Option<AdjacencyMark>,
}

/// 組の寄せで置き直す窓（`shown` と違うものだけ）。
#[allow(dead_code)] // 同上
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GroupShift {
    pub scope: usize,
    /// 置く左上。
    pub to: PointPx,
}

/// 組の寄せ（設計 GroupClamp）。`windows` は記憶から位置を戻したスコープだけ（スコープ昇順）。
///
/// 組は隣り合う番号の対 (n−1, n) が、印が `left`／`right` か、記憶の矩形どうしの
/// `judge_adjacency` が左・右・重なりのとき繋がる（要件 4.1・4.2）。離れていれば窓ごと（要件 4.4）。
/// 組の全員が今の判定で寄せられている（`shown != projected`）ときだけ、`projected` の和の矩形を
/// 和の矩形のモニタの作業領域へ収めの式で収め、その差を全員に足す（要件 4.1・7.1・7.2）。
/// 見えている窓のある組は動かさない（要件 4.1 の改め・4.3）。作業領域が取れない組は返さず
/// `Unresolved`（warn!・要件 7.4）。
#[allow(dead_code)] // 呼び手は記憶の戻し（task 2.4）・examples には無い
pub(crate) fn keep_restored_groups(
    windows: &[RestoredWindow],
    snapshot: &MonitorSnapshot,
) -> Vec<GroupShift> {
    let scene = ClampScene::RestoreGroup.as_str();
    let mut out = Vec::new();
    let mut start = 0;
    while start < windows.len() {
        let mut end = start + 1;
        while end < windows.len() && joined_with_prev(windows[end - 1], windows[end]) {
            end += 1;
        }
        let group = &windows[start..end];
        start = end;
        if group.len() < 2 {
            continue; // 1 窓の組は窓ごとの結果のまま（要件 4.4）。
        }
        let scopes: Vec<usize> = group.iter().map(|w| w.scope).collect();
        let clamped = |w: &RestoredWindow| w.shown != w.projected;
        if !group.iter().any(clamped) {
            debug!(scene, scopes = ?scopes, reason = "remembered-visible", "{WORK_AREA_KEEP_TAG} 記憶の組がどれも見えているので動かさない");
            continue;
        }
        if !group.iter().all(clamped) {
            for w in group.iter().filter(|w| clamped(w)) {
                debug!(scene, scope = w.scope, reason = "partner-visible", at = ?w.shown, "{WORK_AREA_KEEP_TAG} 組に見えている窓があるので窓ごとの結果のまま");
            }
            continue;
        }
        let union = group
            .iter()
            .map(|w| {
                rect_of(ChainRect {
                    x: w.projected.x,
                    y: w.projected.y,
                    ..w.remembered
                })
            })
            .reduce(|a, b| RectPx {
                left: a.left.min(b.left),
                top: a.top.min(b.top),
                right: a.right.max(b.right),
                bottom: a.bottom.max(b.bottom),
            })
            .expect("組は 2 窓以上");
        let Some(area) = work_area_for_window(snapshot, union) else {
            warn!(scene, scopes = ?scopes, reason = "no-work-area", "{WORK_AREA_UNRESOLVED_TAG} 作業領域が取れず組を窓ごとの結果のまま置く");
            continue;
        };
        let origin = PointPx {
            x: union.left,
            y: union.top,
        };
        let size = SizePx {
            w: union.right.saturating_sub(union.left),
            h: union.bottom.saturating_sub(union.top),
        };
        let moved = clamp_rect_to_work_area(origin, size, area);
        let (dx, dy) = (
            moved.x.saturating_sub(origin.x),
            moved.y.saturating_sub(origin.y),
        );
        for w in group {
            let to = PointPx {
                x: w.projected.x.saturating_add(dx),
                y: w.projected.y.saturating_add(dy),
            };
            if to != w.shown {
                info!(scene, scope = w.scope, how = "shifted", from = ?w.shown, to = ?to, work_area = ?Some(area), "{WORK_AREA_CLAMP_TAG} 記憶の組を同じ量だけ作業領域の中へ寄せた");
                out.push(GroupShift { scope: w.scope, to });
            }
        }
    }
    out
}

/// 組 (prev, cur) が繋がるか: 隣り合う番号で、印が左右か、記憶の矩形が左・右・重なり。
fn joined_with_prev(prev: RestoredWindow, cur: RestoredWindow) -> bool {
    prev.scope.checked_add(1) == Some(cur.scope)
        && (matches!(cur.mark, Some(AdjacencyMark::Left | AdjacencyMark::Right))
            || !matches!(
                judge_adjacency(prev.remembered, cur.remembered),
                Adjacency::Apart { .. }
            ))
}
