//! 案 A の owner 確立系——両窓のハンドルが揃った巡に owner を 1 回だけ張る。
//!
//! ペア宣言（[`KeepDirectlyAbove`]）は entity 参照だけの宣言であり、それを Win32 の
//! owner 関係へ落とすのが本モジュールである（design.md「案 A の確立フロー」）。
//! 確立が済めば「バルーンはキャラのすぐ手前」（要件 1.1）の維持は OS の保証へ移り、
//! 以後は再断行の要求（[`ReassertZOrder`]）と実機ゲートの判定だけが残る。
//!
//! 宣言・状態・判断・診断記録の語彙は隣の [`zorder_pair`](super::zorder_pair) が持つ。
//! 本モジュールが足すのは**確立の手順**と、失敗を繰り返さないための印
//! （[`OwnerEstablishFailed`]）だけである。
//!
//! 維持系（トリガ→判断→適用→検証）と破棄時の owner 切離しは後続タスクの担当。

use bevy_ecs::prelude::*;
use bevy_ecs::system::NonSendMarker;
use windows::Win32::Foundation::HWND;

use super::zorder_pair::{
    PairFixDecision, SkipReason, log_owner_establish_failed, log_owner_established,
    measure_window_above, record_decision,
};
use super::{KeepDirectlyAbove, OwnerLink, ReassertZOrder, WindowHandle, ZOrderPairStrategy};
use crate::api::set_window_owner;

// ============================================================================
// OwnerEstablishFailed - 同じハンドルでの再試行を止める印
// ============================================================================

/// owner の確立に失敗したハンドルの組（バルーン窓へ付与）。
///
/// design.md「Error Strategy」の確立失敗は「回復は次の `Added<WindowHandle>` 巡では
/// 行わない——同一 HWND での再試行は同じ失敗を繰り返すのみ・ログ二重化を避ける」と
/// 定める。その「同一 HWND」を判定するために、**試して失敗したハンドルの組そのもの**を
/// 印として残す。窓が作り直されて別のハンドルが付いた場合は組が変わり、確立をもう一度
/// 試みる——抑止しているのは「同じ失敗の繰り返し」であって「二度と試さないこと」ではない。
///
/// [`OwnerLink`] を流用しないのは、あちらが**張れた事実**の記録であり、切離し（要件 5.9）が
/// その有無で対象を選ぶためである。張れなかったペアに [`OwnerLink`] が付くと、
/// 切離しが張っていない owner を外そうとする。
///
/// 公開到達性を持つのは、確立系（`pub` な system）の引数に現れるためだけである。
/// 他クレートから付けることは想定していない（付ける・外すのは本モジュールの手順）。
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerEstablishFailed {
    /// owner を張ろうとした側（バルーン窓）のハンドル
    pub owned_hwnd: HWND,
    /// owner にしようとした側（キャラ窓）のハンドル
    pub owner_hwnd: HWND,
}

// SAFETY: `OwnerEstablishFailed` は 2 つの `HWND`（windows-rs では `*mut c_void` の
// newtype）を保持するため自動では Send/Sync が導出されず、`Component`（ECS は Send+Sync を
// 要求）にするためこの手動 impl は必須である。健全性: 保持するのは**試した組の値の写し**
// にすぎず、窓の所有権も破棄責務も伴わない。この値は比較にしか使わず Win32 へは渡さない。
// [`OwnerLink`] と同根の crate 標準の HWND 取り扱い方針。
unsafe impl Send for OwnerEstablishFailed {}
unsafe impl Sync for OwnerEstablishFailed {}

// ============================================================================
// establish_owner_links - 確立系（Added<WindowHandle> 駆動）
// ============================================================================

/// 案 A: 両窓の OS ハンドルが揃った巡に owner を 1 回だけ張る（冪等）。
///
/// # 何を契機に動くか
///
/// 契機は [`WindowHandle`] の**付与**である（`Added<WindowHandle>` パターン・
/// `register_ghost_windows_click_through` の先例）。ペアの 2 窓へのハンドル付与は同じ巡とは
/// 限らない（生成順はバルーン → キャラ）ため、**宣言を持つ窓を毎巡走査し、その巡に
/// どちらかのハンドルが付いた場合だけ**手順へ入る。付与が無い巡は判断そのものを行わない
/// ——これが「毎フレーム巡回では動かない」（維持系と共通の受動性）の実装である。
///
/// # 手順（design.md「案 A の確立フロー」2）
///
/// 1. 揃っていなければ理由つきで見送る（要件 6.3）。理由は 3 通り——相手の実体が既に
///    無い（`PeerMissing`・要件 1.5）／まだハンドルが付いていない（`HandleMissing`）／
///    現在のストラテジが案 B で owner を張らない構成である（`StrategyDisabled`）。
/// 2. 揃っていれば `set_window_owner(balloon_hwnd, char_hwnd)` を呼ぶ。
/// 3. 成功: [`OwnerLink`] を付け、初期隣接を確定させるため [`ReassertZOrder`] を 1 発入れ、
///    確立の記録（info・要件 6.1）を出す。記録には確立直後に採った「キャラ窓の 1 つ手前」の
///    実測を載せる——ここにバルーン窓が現れることが案 A の中核保証（ゲート G6）の証跡になる。
/// 4. 失敗: error 水準で記録し（要件 6.2）、同じハンドルの組では再試行しないよう
///    [`OwnerEstablishFailed`] を付ける。窓は現行どおり表示され続ける（要件 6.4）。
///
/// # 二度張らない仕掛け
///
/// 確立済み（[`OwnerLink`] 保持）のペアはクエリの絞り込みで外れるため、以後どれだけ
/// ハンドル付与の巡が来ても手順へ入らない。
///
/// # UI スレッド固定
///
/// [`NonSendMarker`] を取るのは Win32 を呼ぶためである（design.md「Responsibilities &
/// Constraints」）。この印が付いた system をスケジュール実行器はメインスレッド以外で
/// 走らせない——窓を作ったスレッド以外から窓を触らせないための固定である。
pub fn establish_owner_links(
    _ui_thread: NonSendMarker,
    mut commands: Commands,
    strategy: Option<Res<ZOrderPairStrategy>>,
    declarations: Query<
        (Entity, &KeepDirectlyAbove, Option<&OwnerEstablishFailed>),
        Without<OwnerLink>,
    >,
    // すべての entity に当たるクエリ。`Err` は「その実体がもう無い」、`Ok(None)` は
    // 「実体はあるがハンドルがまだ無い」——見送りの理由を分けるためこの形にする。
    handles: Query<Option<Ref<WindowHandle>>>,
) {
    // ストラテジ未挿入は結線前の状態。既定（案 A）で動く（design.md「State」の既定値）。
    let strategy = strategy.map(|s| *s).unwrap_or_default();

    for (entity, declaration, failed) in declarations.iter() {
        let peer = declaration.peer;
        let owned = handles.get(entity).ok().flatten();
        let peer_lookup = handles.get(peer);

        // この巡でどちらかにハンドルが付いたか（付与が無い巡は判断そのものを行わない）。
        let handle_added = owned.as_ref().is_some_and(Ref::is_added)
            || peer_lookup
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .is_some_and(Ref::is_added);
        if !handle_added {
            continue;
        }

        // ストラテジの判定を生存・ハンドルより先に置くのは、案 B では**そもそも owner を
        // 張らない**ためである（[`decide_pair_fix`](super::zorder_pair::decide_pair_fix) は
        // 生存を先に見るが、あちらは是正の判断であり順序の意味が違う）。案 B で相手の不在を
        // 理由に挙げると、owner を張らない本当の理由が記録から消える。
        if matches!(strategy, ZOrderPairStrategy::ExplicitMaintenance) {
            record_decision(
                entity,
                peer,
                PairFixDecision::Skip(SkipReason::StrategyDisabled),
            );
            continue;
        }

        let Ok(owner) = peer_lookup else {
            // 相手の実体が既に無い。残る窓には一切書き込まない（要件 1.5）。
            record_decision(entity, peer, PairFixDecision::Skip(SkipReason::PeerMissing));
            continue;
        };
        let (Some(owned), Some(owner)) = (owned, owner) else {
            // 両方生きているが、まだ片方にハンドルが付いていない。揃った巡で改めて張る。
            record_decision(
                entity,
                peer,
                PairFixDecision::Skip(SkipReason::HandleMissing),
            );
            continue;
        };
        let (owned_hwnd, owner_hwnd) = (owned.hwnd, owner.hwnd);

        // 同じハンドルの組で既に失敗している場合は、もう試さない（design.md
        // 「Error Strategy」）。記録も足さない——error 水準の失敗記録は同じ entity に
        // 対して既に出ており、ここで見送りを重ねるとログの二重化を避けた意味が消える。
        if failed
            == Some(&OwnerEstablishFailed {
                owned_hwnd,
                owner_hwnd,
            })
        {
            continue;
        }

        match set_window_owner(owned_hwnd, owner_hwnd) {
            Ok(()) => {
                // 確立直後の「キャラ窓の 1 つ手前」——ここにバルーン窓が居ることが
                // 案 A の中核保証の証跡（ゲート G6）。実測できなくても確立は成立する。
                let measured_prev = measure_window_above(owner_hwnd);
                commands.entity(entity).insert((
                    OwnerLink { owner_hwnd },
                    // 初期隣接の確定（維持系が消費する一回限りの要求）。確立は窓の
                    // 生成直後・ペアにつき 1 度きりであり、他の挿入元（要件 2.6 の再表示・
                    // z 変化検知）はこれより後にしか動かない——ゆえにここでの挿入が
                    // 検証待ちの要求を踏み潰すことはない。
                    ReassertZOrder::default(),
                ));
                log_owner_established(entity, peer, owned_hwnd, owner_hwnd, measured_prev);
            }
            Err(err) => {
                log_owner_establish_failed(entity, &err);
                commands.entity(entity).insert(OwnerEstablishFailed {
                    owned_hwnd,
                    owner_hwnd,
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "zorder_pair_establish_tests.rs"]
mod tests;
