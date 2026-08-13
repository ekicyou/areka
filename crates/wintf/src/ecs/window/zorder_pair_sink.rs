//! 他アプリ活性化時の沈降観測——非活性化の巡には目印を付けるだけに留め、実測と記録は
//! 次の巡で行う（design.md「WM_ACTIVATE 沈降観測（読み取り専用・両案共通・遅延 1 巡）」）。
//!
//! # なぜ遅らせるのか（即時に測ってはならない理由）
//!
//! `WM_ACTIVATE(WA_INACTIVE)` は活性化トランザクションの**途中**に届く。自分が前面から
//! 外れたことは確定しているが、**新しい前面窓の浮上は終わっていないことがある**。その瞬間に
//! 重なりを走査すると、実装に何の欠陥が無くても「前面窓より背面に居る」を満たさず、
//! **偽の失敗**が記録として残る。遅らせた観測はこの偽陽性を断つ。
//!
//! **ここを「即時に測ればよい」と書き換えてはならない。** 非活性化の枝は「前面から外れる
//! 瞬間を確実に知る自窓イベント」としてのみ使い、実測は状態が落ち着いた次の巡で行う、
//! という分業である（research.md §8-10 の解・設計ディスカッション CI3 の裁定）。
//!
//! # 何を測るのか
//!
//! - **ペアの隣接が保たれているか**（要件 4.4）——沈んでもバルーンはキャラのすぐ手前に居るか。
//! - **前面窓より背面に居るか**（要件 4.1／4.2）——ゴーストが前面に居座っていないか。
//!
//! 前面窓との相対を測るのはペアの**手前側の窓**（バルーン窓）である。ペアの中で最も手前に
//! 居る窓が前面窓より背面なら、その背後に居るキャラ窓も当然に背面である——最も強い 1 点を
//! 測れば足りる。
//!
//! # 窓の挙動は一切変えない
//!
//! 本モジュールは読み取りだけを行う。指令を 1 本も出さず、重なり・位置・寸法・窓スタイル・
//! 表示状態のいずれにも書き込まない。要件 4.1 を「沈んだ状態を妨げない」という受動実装で
//! 満たす設計（design.md Requirements Traceability 4.1）に対し、本モジュールはその
//! **証跡だけを残す**役である。
//!
//! 記録を出すマクロは [`zorder_pair`](super::zorder_pair) に置く（`tracing` の出力先は
//! 呼び出し元の module path が既定であり、分けるとサインオフの grep 対象が分裂する）。
//! 本モジュールは記録用の関数を呼ぶだけである。

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;

use super::zorder_pair::{
    FrontScan, PairFixDecision, SkipReason, log_orphan_sink_mark_dropped, log_sink_observed,
    measure_window_below, measure_windows_in_front, record_decision,
};
use super::zorder_pair_maintain::HandleQuery;
use super::{KeepDirectlyAbove, WindowHandle};
use crate::api::get_foreground_window;

// ============================================================================
// SinkObservationPending - 非活性化の巡に付く観測の目印
// ============================================================================

/// 「この巡に非活性化が起きた。次の巡で沈降を実測せよ」の目印（バルーン窓へ付与）。
///
/// 付けるのは `WM_ACTIVATE` の非活性化枝（[`mark_pair_sink_observation`]）、消すのは
/// 次の巡の [`observe_marked_pair_sinks`] であり、**一回限り**である。付けっぱなしにすると
/// 毎巡の走査になり、「トリガ駆動のみ」という維持系の受動性が崩れる。
///
/// 値を持たないのは、観測に要る値がすべて**測る時点**で採れるからである。非活性化の瞬間の
/// 値を持ち越すと、まさにその「途中の値」で判定してしまう。
///
/// 公開到達性を持つのは、維持系（`pub` な system）の引数に現れるためだけである
/// （[`IssuedPairFix`](super::IssuedPairFix) と同じ理由）。
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SinkObservationPending;

/// 目印の付いた窓を見るためのクエリ。
///
/// 宣言を `Option` で取るのは、目印が付いてから次の巡までに宣言が外れた場合に
/// **目印を確実に外して記録を残す**ためである（記録の無い見送りを作らない）。
pub(crate) type SinkMarkQuery<'w, 's> =
    Query<'w, 's, (Entity, Option<&'static KeepDirectlyAbove>), With<SinkObservationPending>>;

// ============================================================================
// mark_pair_sink_observation - 非活性化の巡に行う唯一の処理
// ============================================================================

/// 非活性化された窓がペアの当事者なら、宣言を持つ側へ観測の目印を付ける。
///
/// 付ける先を**宣言を持つ側（バルーン窓）に揃える**のは、記録の `entity` 欄を他の記録
/// （確立・是正・見送り・検証不一致）と同じ側に揃えるためである。揃っていれば、areka が出す
/// ペア宣言の記録との entity 結合が 1 通りに定まり、スコープの 2 段 grep が成立する
/// （design.md「診断ログ語彙」の結合キーの約束）。
///
/// 非活性化されるのはキャラ窓・バルーン窓のどちらもありうる。前者は宣言を持たない側
/// （`peer` として参照されている側）なので、宣言の側から引き当てる。
///
/// 戻り値は目印を付けた窓（当事者でなければ `None`）。**窓の状態は一切変えない**——
/// 付くのは ECS 上の目印だけであり、Win32 は 1 度も呼ばない。
pub(crate) fn mark_pair_sink_observation(world: &mut World, deactivated: Entity) -> Option<Entity> {
    let holder = if world.get::<KeepDirectlyAbove>(deactivated).is_some() {
        deactivated
    } else {
        let mut declarations = world.query::<(Entity, &KeepDirectlyAbove)>();
        declarations
            .iter(world)
            .find(|(_, declaration)| declaration.peer == deactivated)
            .map(|(entity, _)| entity)?
    };
    world.entity_mut(holder).insert(SinkObservationPending);
    Some(holder)
}

// ============================================================================
// is_behind_foreground - 前面窓との相対を決める純関数
// ============================================================================

/// 対象が前面窓より背面に居るかを、走査結果から決める（Win32 も World も触らない）。
///
/// Win32 には 2 つの窓の前後を直接比べる API が無いため、判定は「対象より手前の列に前面窓が
/// 居るか」で行う。列の組立（実測）は [`measure_windows_in_front`] が、判定は本関数が担う
/// ——この分割が、前面窓が何になるか決まらない環境でも判定を決定論的テストで固定できる根拠
/// である（要件 7.1）。
///
/// | 状況 | 返り値 |
/// |---|---|
/// | 前面窓が取れない | `None`（測れなかった） |
/// | 前面窓が対象自身 | `Some(false)`（自分が前面なら背面ではない） |
/// | 手前の列に前面窓が居る | `Some(true)` |
/// | 最前面まで辿って居ない | `Some(false)` |
/// | 列が不完全なまま見つからない | `None`（測れなかった） |
///
/// 最後の行が肝である——走査が途中で切れたことを「居なかった」と読むと、測れなかっただけの
/// 巡に**偽の失敗**が記録される。取れなかったものは番兵として残す（記録側の
/// `behind_foreground=-`）。
pub(crate) fn is_behind_foreground(
    target: HWND,
    foreground: Option<HWND>,
    scan: &FrontScan,
) -> Option<bool> {
    let foreground = foreground?;
    if foreground == target {
        return Some(false);
    }
    if scan.windows.contains(&foreground) {
        return Some(true);
    }
    // 見つからなかった。列を最前面まで辿れていて初めて「居ない」と言える。
    scan.reached_top.then_some(false)
}

// ============================================================================
// observe_marked_pair_sinks - 遅らせた実測と記録
// ============================================================================

/// 目印の付いたペアを実測し、沈降の記録を残す（維持系の 1 巡の中で走る）。
///
/// 目印は測れても測れなくてもこの巡で外す——一回限りの観測であり、残せば毎巡の走査になる。
/// 測れない巡（相手が消えた・ハンドルが未付与）は理由つきの見送りとして記録する
/// （要件 6.3・黙って諦めない）。相手の消滅は正常系であり error にはしない（要件 1.5）。
///
/// 読み取りだけを行う——指令を積まず、重なりにも位置にも書き込まない。
pub(crate) fn observe_marked_pair_sinks(
    commands: &mut Commands,
    marks: &SinkMarkQuery,
    handles: &HandleQuery,
) {
    for (entity, declaration) in marks.iter() {
        commands.entity(entity).remove::<SinkObservationPending>();

        let Some(declaration) = declaration else {
            // 目印を付けてから測るまでの間に宣言が外れた。観測すべきペアが特定できない。
            log_orphan_sink_mark_dropped(entity);
            continue;
        };
        let peer = declaration.peer;

        let above_hwnd = handles.get(entity).ok().flatten().map(|h| h.hwnd);
        let Ok(peer_handle) = handles.get(peer) else {
            // 相手の実体が既に無い。残る窓には一切書き込まない（要件 1.5）。
            record_decision(entity, peer, PairFixDecision::Skip(SkipReason::PeerMissing));
            continue;
        };
        let (Some(above_hwnd), Some(below_hwnd)) =
            (above_hwnd, peer_handle.map(|h: &WindowHandle| h.hwnd))
        else {
            record_decision(
                entity,
                peer,
                PairFixDecision::Skip(SkipReason::HandleMissing),
            );
            continue;
        };

        // 隣接の正準判定は「手前側の最も近い可視の背後が背後側か」（design.md Invariants）。
        // 維持系の検証と同じ向きで測る——判定の向きが 2 通りあると記録が読めなくなる。
        let adjacency_ok = measure_window_below(above_hwnd) == Some(below_hwnd);

        // 前面窓との相対は手前側の窓で測る（ペアの中で最も手前に居る窓が背面なら、
        // その背後に居る窓も当然に背面である）。
        let foreground = get_foreground_window();
        let scan = measure_windows_in_front(above_hwnd);
        let behind_foreground = is_behind_foreground(above_hwnd, foreground, &scan);

        log_sink_observed(entity, adjacency_ok, foreground, behind_foreground);
    }
}

#[cfg(test)]
#[path = "zorder_pair_sink_tests.rs"]
mod tests;
