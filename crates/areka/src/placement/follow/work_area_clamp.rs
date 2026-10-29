//! キャラ窓を作業領域へ寄せる判定と記録（window-work-area-clamp・設計
//! 「配置の層: placement/follow/work_area_clamp.rs」）。
//!
//! - 場面の語 [`ClampScene`] と記録の語（`[work-area-clamp] Clamp`／`Keep`／`Unresolved`）を
//!   1 か所に置く（要件 9.1・9.2・7.4）
//! - [`resolve_chain_in_work_area`] は組ごとの決めで解いた鎖を、前の窓の置いた後の矩形が属する
//!   モニタの作業領域の中に置く純関数の核（要件 1・2・7）
//! - [`place_chain_in_work_area`] はその World の口。窓の高さと作業領域を World から読んで核を呼び、
//!   寄せた・寄せなかった・取れなかったを記録する（要件 7.4・9.1・9.2）
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

use super::super::balloon_limit::clamp_rect_to_work_area;
use super::super::chain_finalize::{ChainRect, ChainSlot, KeepReason, PairDecision, resolve_chain};
use super::{MonitorSnapshot, PointPx, RectPx, SizePx, work_area_for_window};

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
