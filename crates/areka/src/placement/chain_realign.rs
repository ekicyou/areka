//! DPI／拡大率の遷移後に連鎖（隣接ペア）を**一度だけ**解き直す機構
//! （`areka-P0-dpi-transition-atomicity` 設計 D8／C4・要件 6.1／6.2／6.3／6.6）。
//!
//! # なぜ要るか
//!
//! 起動時の連鎖確定（[`super::chain_finalize`]）は「実表示寸が確定した最初のフレームで
//! 一度きり」であり、確定後のサーフェス切替では二度と駆動しない（`scope-chain-gap` 7.4）。
//! この規定は**会話中の表情差替**でキャラが横へ動かないための約束であって、拡大率の遷移は
//! 想定外である。拡大率が変われば全スコープの幅が k 倍に変わり、各窓は下端中央を保ったまま
//! 置き直されるので、隣接していた 2 体のあいだに**幅変化の半分の和**だけ隙間が開く。
//!
//! 実機実測（emo2・200%→100%）: 200% で隙間 0 だった二体が、100% で **359px** 離れた
//! （幅 764→382 と 672→336 の左端差の和 `191 + 168 = 359`）。
//!
//! # 何をするか（一度きりを二段で保つ）
//!
//! 起動時の確定標識 [`ChainFinalized`] は**解除しない**——「起動時の確定は一度きり」という
//! 意味をそのまま残したまま、遷移後の解き直しを別の資源 [`ChainRealignPending`] が担う。
//!
//! 1. **武装**（[`arm_chain_realign`]）: 拡大率の相で、キャラ窓の再射影が**寸変化を伴って**
//!    書込を起こしたときに武装する。表情差替（拡大率不変）では寸が変わっても本経路を通らない
//!    ので武装しない（要件 6.6）。
//! 2. **解決**（[`realign_chain_once_with`]）: 全スコープの窓寸が遷移後の実表示寸へ揃い、かつ
//!    整合待ちの札（[`DpiSyncHold`]）を持つゴースト窓が 1 つも無くなった最初のフレームで
//!    連鎖を解き直し、武装を解く。遷移 1 回につき武装→解決の 1 往復である（要件 6.2）。
//!
//! # 組ごとの決め方（`dpi-realign-remembered-chain` 要件 1・2・3・7）
//!
//! 武装のとき、変わる直前のキャラ窓の矩形を控える（[`ChainRealignPending::before`]）。解決では
//! 組 (n−1, n) ごとに、上から最初に当たった決めを取る。
//!
//! 1. n−1 か n が控えに無い → 動かさない（`unreadable`・理由つきの `warn!`。ほかの組は続ける）
//! 2. n が窓の組に入っている → 動かさない（`sticky-group`）
//! 3. n−1 か n がドラッグの最中 → 動かさない（`dragging`・寄せもしない）
//! 4. n を武装の後に台本が置いた → 動かさない（`script-moved`）
//! 5. 控えの矩形どうしが隣り合っていた → 左／右へ付ける（`adjacent-left`／`adjacent-right`）
//! 6. 離れていた・重なっていた → 動かさない（`apart`／`overlapped`）
//!
//! 決めは揃った後の今の x と幅で [`resolve_chain`] が解き、作業領域の中へ寄せてから書く
//! （y は寄せが縦に収めたときだけ変わる・window-work-area-clamp 要件 2.1）。
//! 既定の位置も台本の印も読まないので、記憶から戻した窓（既定の位置が無い）も、変わる直前に
//! 隣り合っていれば詰まる。組ごとに 1 行、判断の語・動かした窓・量・直前の隙間を `info!` に
//! 残す。詰め直した位置は記憶に書かない（要件 6.1）。
//!
//! # 既定位置は自分で書かない
//!
//! 移動は [`move_window_with_route`] に [`PlacementRoute::ChainRealign`] を持たせて行う。この
//! 経路は**システム由来**（[`PlacementRoute::is_system_reanchor`]）ゆえ、単一の窓書込口が既定
//! 位置を書込先へ運ぶ。本モジュールが重ねて [`GhostWindows::set_default_char_pos`] を呼ぶと
//! 同じ量を 2 人が書くことになるので、**呼ばない**（起動時確定は `MoveCue`＝明示操作の経路で
//! 書くため自分で書く必要があり、そちらとは非対称である）。
//!
//! # 見送りの可観測性
//!
//! 条件未達の見送りは毎フレーム無音のまま数え、有界の待ち
//! （[`CHAIN_FINALIZE_STALL_FRAMES`]）を超えた時点で**一度だけ**理由つきの `warn!` を出す
//! （[`ChainFinalizeStall`] を再利用する）。計数と一発フラグは武装のたびに初期化するので、
//! 2 度目以降の遷移で待ちが生じても警告は各回一度ずつ出る（要件 6.3）。
//!
//! [`CHAIN_FINALIZE_STALL_FRAMES`]: super::chain_finalize::CHAIN_FINALIZE_STALL_FRAMES
//! [`resolve_chain`]: super::chain_finalize::resolve_chain
//! [`GhostWindows::set_default_char_pos`]: super::spawn::GhostWindows::set_default_char_pos

use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::{DeferredWorld, World};
use tracing::{debug, info, warn};
use wintf::ecs::WindowPos;
use wintf::ecs::drag::DraggingState;

use super::chain_finalize::{
    Adjacency, ChainDeferReason, ChainFinalizeStall, ChainFinalized, ChainRect, ChainSlot,
    KeepReason, PairDecision, ScopeChainState, judge_adjacency, mark_non_positive_widths,
    note_chain_deferral,
};
use super::diag::PlacementRoute;
use super::dpi_sync::{self, DpiSyncHold};
use super::follow::{ClampScene, move_window_with_route, place_chain_in_work_area};
use super::resolver::PointPx;
use super::spawn::GhostWindows;
use super::sticky_group_ledger::StickyGroupLedger;
use super::transition_diag::{
    self, CHAIN_STAGE_ARMED, CHAIN_STAGE_DEFERRED, CHAIN_STAGE_REALIGNED,
};

/// 遷移後の解き直しが**武装中**であることを表す資源（設計 C4 の状態機械）。
///
/// 状態は `None`（平時）→ `Some{armed_frame, before, script_moved}`（武装）→ `None`（解決）の
/// 1 往復だけを取る。起動時の確定が未了（[`ChainFinalized`] 未挿入）なら武装そのものが起きない。
/// 窓の一式を閉じるとき資源ごと外れるので、控えと記録も一緒に消える
/// （`dpi-realign-remembered-chain` 要件 5.1）。
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct ChainRealignPending {
    /// 武装したフレーム番号（診断で「いつから待っているか」を読むための値）。
    pub armed_frame: u32,
    /// 武装の時点で控えた、変わる直前のキャラ窓の矩形（スコープ, 矩形）。
    /// 控えられなかったスコープは入らない（要件 1.1・7.2）。
    pub before: Vec<(usize, ChainRect)>,
    /// 武装中に台本が置いた窓（同じ窓が何度入ってもよい・要件 3.1）。
    pub script_moved: Vec<Entity>,
}

/// 全キャラ窓の今の矩形を控える（`dpi-realign-remembered-chain` 要件 1.1・7.2）。
///
/// 拡大率の相が最初のキャラ窓を書く直前に 1 度だけ呼ぶ。`WindowPos` の位置と大きさが揃わない
/// 窓は控えに入れない（解決のとき、その窓に触れる組を動かさず理由を `warn!` で記録する）。
/// バルーン窓は詰め直しの組の要素ではないので見ない。台帳が無ければ空を返す。
pub fn snapshot_char_rects(world: &World) -> Vec<(usize, ChainRect)> {
    let Some(ghost_windows) = world.get_resource::<GhostWindows>() else {
        return Vec::new();
    };
    ghost_windows
        .scopes()
        .filter_map(|scope| {
            let rect = ghost_windows
                .char_window(scope)
                .and_then(|window| world.get::<WindowPos>(window))
                .and_then(|wp| wp.position.zip(wp.size))
                .map(|(pos, size)| ChainRect {
                    x: pos.x,
                    y: pos.y,
                    w: size.width,
                    h: size.height,
                });
            if rect.is_none() {
                debug!(
                    scope,
                    "chain_realign: キャラ窓の位置か大きさが無く、変わる直前の矩形を控えられなかった"
                );
            }
            rect.map(|rect| (scope, rect))
        })
        .collect()
}

/// 解き直しの入力——判定へ渡す状態列と、反映へ渡す `(scope, キャラ窓 entity, 現在位置)` の列。
///
/// 起動時確定の走査（`emo2_boot::frame::drain_resnap::collect_chain_states`）の戻り値と
/// **同じ形**である。走査を 2 つ持つと「窓寸が実表示寸へ揃ったか」の判定が 2 実装に割れる
/// ので、解き直しは走査を関数として受け取り、起動時確定とまったく同じものを再利用する。
pub type ChainRealignScan = (Vec<ScopeChainState>, Vec<(usize, Entity, PointPx)>);

/// 遷移後の解き直しを**武装**する（設計 C4・要件 6.1）。
///
/// 呼び手は拡大率の相（`emo2_boot::frame::dpi`）だけで、キャラ窓の再射影が**寸変化を
/// 伴って**書込を起こしたときに呼ぶ。判断（寸が変わったか・書込が起きたか）は呼び手が持つ
/// ——本関数は「武装してよい状態か」だけを見る。
///
/// # 何もしない 2 通り
///
/// - [`ChainFinalized`] 未挿入: 起動時の確定がまだ済んでいない。解き直す連鎖がそもそも無く、
///   起動時確定が同じフレームで走る（先に走るのは確定の方である）。
/// - 既に武装済み: 遷移 1 回につき 1 往復ゆえ、`armed_frame`・控え `before`・台本の記録は
///   **最初の**武装を保つ。同一フレームで複数のキャラ窓が寸変化を起こしても、解決の前に
///   2 度目の変化が来ても武装は 1 回である（`dpi-realign-remembered-chain` 要件 1.7）。
pub fn arm_chain_realign(world: &mut World, before: Vec<(usize, ChainRect)>) {
    // 起動時の確定が未了なら解き直す相手が居ない（確定は同一フレームの後段で走る）。
    if world.get_resource::<ChainFinalized>().is_none() {
        return;
    }
    // 既に武装済み——多スコープの遷移で 2 度目の呼出が来ても、待ちの起点は動かさない。
    if world.get_resource::<ChainRealignPending>().is_some() {
        return;
    }
    let armed_frame = dpi_sync::current_frame(world);
    world.insert_resource(ChainRealignPending {
        armed_frame,
        before,
        script_moved: Vec::new(),
    });
    // 停滞診断の初期化（要件 6.3）: 2 度目以降の遷移でも見送りの警告が一度は出るようにする。
    world.init_resource::<ChainFinalizeStall>();
    world.resource_mut::<ChainFinalizeStall>().reset();
    debug!(
        armed_frame,
        "chain_realign: 拡大率の遷移で寸が変わったため連鎖の解き直しを武装（要件 6.1）"
    );
    if transition_diag::is_enabled() {
        let scopes = scope_count(world);
        transition_diag::log_chain(world, CHAIN_STAGE_ARMED, scopes, 0, None);
    }
}

/// `ScriptPlaced` の差し込みの口（`on_insert`・付け直しでも走る）。武装中なら窓を
/// `script_moved` に積む（`dpi-realign-remembered-chain` 要件 3.1）。武装していなければ何もしない。
pub(super) fn note_script_placed_after_arm(mut world: DeferredWorld, hook: HookContext) {
    if let Some(mut pending) = world.get_resource_mut::<ChainRealignPending>() {
        pending.script_moved.push(hook.entity);
    }
}

/// 台本が印を外して窓を置いたとき（`\![execute,resetwindowpos]`＝`reset_pos.rs`）に呼ぶ受け口。
/// 武装中なら窓を `script_moved` に積む（要件 3.1）。武装していなければ何もしない。
pub fn note_script_moved(world: &mut World, char_window: Entity) {
    if let Some(mut pending) = world.get_resource_mut::<ChainRealignPending>() {
        pending.script_moved.push(char_window);
    }
}

/// 武装していれば、条件が揃った最初のフレームで連鎖を**一度だけ**解き直す（要件 6.2／6.6）。
///
/// `collect` は起動時確定と同一の走査（全スコープの実表示寸と窓寸の一致を含む）であり、
/// 呼び手が表示側の寸の引き口を束ねて渡す。本モジュールは表示層を知らない。
///
/// # 解決の条件（すべて満たしたフレームで 1 度だけ走る）
///
/// 1. 武装中である（[`ChainRealignPending`] が在る）。
/// 2. 整合待ちの札（[`DpiSyncHold`]）を持つゴースト窓が 1 つも無い。
/// 3. `collect` が成功する＝全スコープで実表示寸が引け、窓寸がそれと一致している。
///
/// 2 を 3 より**先**に見るのは診断のためである——待ち札のある窓は窓書込を見送られており、
/// 窓寸が実表示寸に追いつかないのは*結果*でしかない。順序を逆にすると、待ちが原因の見送りが
/// すべて `resnap-not-landed` として記録され、本当の理由（拡大率と表が未整合）が消える。
/// 判定そのものは 2 と 3 の論理積ゆえ、順序を変えても解決の可否は 1 bit も変わらない。
pub fn realign_chain_once_with<F>(world: &mut World, collect: F)
where
    F: FnOnce(&World) -> Result<ChainRealignScan, ChainDeferReason>,
{
    // 武装していないフレームは**何も読まない**（定常フレームの無操作＝要件 4.7）。
    if world.get_resource::<ChainRealignPending>().is_none() {
        return;
    }
    if let Some(scope) = held_ghost_window_scope(world) {
        defer_chain_realign(world, ChainDeferReason::DpiSyncHeld { scope });
        return;
    }
    let (states, targets) = match collect(world) {
        Ok(scan) => scan,
        Err(reason) => {
            defer_chain_realign(world, reason);
            return;
        }
    };

    // 武装を解く（遷移 1 回につき 1 往復・要件 6.6）。控えと記録はここで受け取り、以後の
    // 台本の差し込みは記録されない（要件 3.2）。以後のフレームは早期 return で抜ける。
    let Some(pending) = world.remove_resource::<ChainRealignPending>() else {
        return;
    };
    let ledger = world.get_resource::<StickyGroupLedger>();
    let entity_of = |scope: usize| {
        targets
            .iter()
            .find(|(s, _, _)| *s == scope)
            .map(|&(_, entity, _)| entity)
    };
    let before_of = |scope: usize| {
        pending
            .before
            .iter()
            .find(|(s, _)| *s == scope)
            .map(|&(_, rect)| rect)
    };
    let dragging =
        |scope: usize| entity_of(scope).is_some_and(|e| world.get::<DraggingState>(e).is_some());

    // 組ごとの決めと、控えで判定したときの直前の隙間（1〜4 の理由で外した組は判定しない）。
    let mut slots: Vec<ChainSlot> = Vec::with_capacity(states.len());
    let mut before_gaps: Vec<Option<i32>> = Vec::with_capacity(states.len());
    for (i, s) in states.iter().enumerate() {
        let (decision, before_gap) = match i.checked_sub(1).map(|p| states[p].scope) {
            // 先頭（本体）は組を持たない（解きは決めを読まない）。
            None => (PairDecision::Keep(KeepReason::ExplicitlyPlaced), None),
            Some(prev) => match (before_of(prev), before_of(s.scope)) {
                (Some(prev_rect), Some(rect)) => {
                    if ledger.is_some_and(|l| l.group_of(s.scope as u32).is_some()) {
                        (
                            PairDecision::Keep(KeepReason::StickyGroup { scope: s.scope }),
                            None,
                        )
                    } else if let Some(scope) = [prev, s.scope].into_iter().find(|&sc| dragging(sc))
                    {
                        (PairDecision::Keep(KeepReason::Dragging { scope }), None)
                    } else if entity_of(s.scope).is_some_and(|e| pending.script_moved.contains(&e))
                    {
                        (
                            PairDecision::Keep(KeepReason::ScriptMovedAfterArm { scope: s.scope }),
                            None,
                        )
                    } else {
                        match judge_adjacency(prev_rect, rect) {
                            Adjacency::Left { gap } => (PairDecision::JoinLeft, Some(gap)),
                            Adjacency::Right { gap } => (PairDecision::JoinRight, Some(gap)),
                            Adjacency::Apart { gap } => {
                                (PairDecision::Keep(KeepReason::Apart { gap }), Some(gap))
                            }
                            Adjacency::Overlapped { gap } => (
                                PairDecision::Keep(KeepReason::Overlapped { gap }),
                                Some(gap),
                            ),
                        }
                    }
                }
                (prev_rect, _) => {
                    let missing = if prev_rect.is_none() { prev } else { s.scope };
                    warn!(
                        prev_scope = prev,
                        scope = s.scope,
                        missing_scope = missing,
                        "chain_realign: 変わる直前の位置か大きさを控えられなかった窓の組は動かさない（ほかの組は続ける・要件 7.2）"
                    );
                    (
                        PairDecision::Keep(KeepReason::Unreadable { scope: missing }),
                        None,
                    )
                }
            },
        };
        slots.push(ChainSlot {
            scope: s.scope,
            current_x: s.current_x,
            width: s.width,
            decision,
        });
        before_gaps.push(before_gap);
    }

    mark_non_positive_widths(&mut slots);
    // 解いた鎖を基準の窓のモニタの作業領域の中に置く（window-work-area-clamp 要件 2.1・2.3）。
    let moves: Vec<_> =
        place_chain_in_work_area(world, &slots, &targets, |_| ClampScene::DpiRealign)
            .into_iter()
            .filter(|p| p.to != p.from)
            .collect();
    let mut applied: Vec<(Entity, i32, i32)> = Vec::new();
    for (i, slot) in slots.iter().enumerate().skip(1) {
        let decision = slot.decision.as_str();
        let to = moves.iter().find(|m| m.scope == slot.scope).map(|m| m.to);
        let to_x = to.map_or(slot.current_x, |p| p.x);
        info!(
            prev_scope = slots[i - 1].scope,
            scope = slot.scope,
            decision,
            before_gap = before_gaps[i],
            from_x = slot.current_x,
            to_x,
            dx = to_x.saturating_sub(slot.current_x),
            "chain_realign: 拡大率の後の詰め直しで組の判断を下した（要件 7.1）"
        );
        if let Some(to) = to
            && let Some(&(_, entity, _)) = targets.iter().find(|(s, _, _)| *s == slot.scope)
        {
            applied.push((entity, to.x, to.y));
        }
    }
    // 経路は `ChainRealign`＝**システム由来**（D9／D16）。既定位置は単一の窓書込口が
    // 同じ量を運ぶので、ここで `set_default_char_pos` を重ねて呼ばない。
    // Y は寄せが縦に収めたときだけ変わる（それ以外は今の Y＝下端吸着は各窓の再射影が保つ・要件 1.4）。
    for (entity, x, y) in applied {
        move_window_with_route(world, entity, x, y, PlacementRoute::ChainRealign);
    }

    debug!(
        scopes = states.len(),
        moved = moves.len(),
        "chain_realign: 遷移後の連鎖を解き直して武装を解いた（次の遷移まで駆動しない）"
    );
    if transition_diag::is_enabled() {
        transition_diag::log_chain(
            world,
            CHAIN_STAGE_REALIGNED,
            states.len(),
            moves.len(),
            None,
        );
    }
}

/// 整合待ちの札を持つゴースト窓のスコープ（無ければ `None`）。
///
/// 対象は台帳（[`GhostWindows`]）が持つキャラ窓・バルーン窓の**両方**である。バルーンだけが
/// 待っている状態でも連鎖を解いてはならない——キャラ窓を動かせば随伴でバルーンも動き、
/// 待ちが解けたフレームにもう一度書き直すことになる（要件 5.8 が禁じる 2 段書込）。
fn held_ghost_window_scope(world: &World) -> Option<usize> {
    let ghost_windows = world.get_resource::<GhostWindows>()?;
    ghost_windows.scopes().find(|&scope| {
        [
            ghost_windows.char_window(scope),
            ghost_windows.balloon_window(scope),
        ]
        .into_iter()
        .flatten()
        .any(|window| world.get::<DpiSyncHold>(window).is_some())
    })
}

/// 台帳が持つスコープ数（台帳が無ければ 0＝架空の件数を発明しない）。
fn scope_count(world: &World) -> usize {
    world
        .get_resource::<GhostWindows>()
        .map_or(0, |ghost_windows| ghost_windows.scopes().count())
}

/// 見送りを 1 フレームぶん記録し、有界の待ちを超えていれば**一度だけ**診断を出す（要件 6.3）。
///
/// 計数は武装のたびに [`ChainFinalizeStall::reset`] で初期化されるので、2 度目以降の遷移でも
/// 警告は各回一度ずつ出る（起動時確定の一発フラグを共有しつつ、寿命だけを遷移ごとに切る）。
fn defer_chain_realign(world: &mut World, reason: ChainDeferReason) {
    if transition_diag::is_enabled() {
        let scopes = scope_count(world);
        transition_diag::log_chain(
            world,
            CHAIN_STAGE_DEFERRED,
            scopes,
            0,
            Some(reason.as_str()),
        );
    }
    world.init_resource::<ChainFinalizeStall>();
    let mut stall = world.resource_mut::<ChainFinalizeStall>();
    if !note_chain_deferral(&mut stall) {
        return;
    }
    let deferrals = stall.deferrals;
    warn!(
        deferrals,
        // 構造化フィールドの `scope`／`reason` は grep 用（本文の Display は人が読む側）。
        scope = ?reason.scope(),
        reason = reason.as_str(),
        detail = %reason,
        "chain_realign: 遷移後の連鎖の解き直しが続けて見送られている（隣接が崩れたままの可能性・要件 6.3）"
    );
}

#[cfg(test)]
#[path = "chain_realign_script_moved_tests.rs"]
mod chain_realign_script_moved_tests;
