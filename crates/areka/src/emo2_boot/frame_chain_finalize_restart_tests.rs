// =============================================================================
// 窓を全部閉じたあとの次の窓の一式でも、初期配置の確定が初回と同じく走る
// （areka-P0-ghost-shell-balloon-switch 要件 4.10・3.1・タスク 11.9）
//
// 実機の欠陥: 確定の印（`ChainFinalized`）・見送りの回数（`ChainFinalizeStall`）・遷移後の
// 解き直しの待ち（`ChainRealignPending`）が World 全体に 1 つずつ残り、切替や既定への戻しで
// 作り直した窓では確定が一度も走らなかった（emo2 の 2 人目が x=1340 のまま・初回は 1392）。
//
// 窓は確定の既存テストと同じ `resnap_world`（実 `spawn_ghost_windows`・2 スコープ）で生やし、
// 閉じるのは本番と同じ `close_windows_for_restart`、次の一式は同じ形を同じ World へ生やす。
// =============================================================================

use super::test_support::{
    capture_logs, pos_of, resnap_world, settled_sizes, spawn_resnap_windows,
};
use super::*;

use crate::app_exit::{WindowsEpoch, close_windows_for_restart};
use crate::placement::chain_finalize::{ChainFinalizeStall, ChainFinalized};
use crate::placement::chain_realign::ChainRealignPending;
use crate::placement::follow::MonitorSnapshot;
use wintf::ecs::window::ZOrderChainPlan;

/// 確定が連鎖を解き直したときの記録の本文（`finalize_chain_once_with` の info）。
const RESOLVED: &str = "実表示寸で連鎖を再解決";

fn resolved_lines(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains(RESOLVED)).collect()
}

/// 閉じたら窓の一式に属する資源（確定の印・見送りの回数・解き直しの待ち・重なりの鎖の受け口と
/// 不在の報告の控え）は無く、プロセスに属する資源（作業領域の表・閉じた回数）は残る。
#[test]
fn close_windows_for_restart_forgets_window_set_resources() {
    let (mut world, _gw) = resnap_world();
    finalize_chain_once_with(&settled_sizes(), &mut world);
    world.insert_resource(ChainFinalizeStall {
        deferrals: 599,
        reported: false,
    });
    world.insert_resource(ChainRealignPending { armed_frame: 7 });
    world.insert_resource(ZOrderChainPlan::default());
    world.insert_resource(ZOrderAbsentReports::default());
    let present = |world: &World| {
        (
            world.contains_resource::<ChainFinalized>(),
            world.contains_resource::<ChainFinalizeStall>(),
            world.contains_resource::<ChainRealignPending>(),
            world.contains_resource::<ZOrderChainPlan>(),
            world.contains_resource::<ZOrderAbsentReports>(),
        )
    };
    assert_eq!(
        present(&world),
        (true, true, true, true, true),
        "前提: 閉じる前は 5 つとも在る（無い World では判定が空になる）"
    );

    let _ = close_windows_for_restart(&mut world);

    assert_eq!(
        (
            present(&world),
            world.contains_resource::<MonitorSnapshot>(),
            world.get_resource::<WindowsEpoch>().map(|e| e.0),
        ),
        ((false, false, false, false, false), true, Some(1)),
        "((確定の印, 見送りの回数, 解き直しの待ち, 鎖の受け口, 不在の報告の控え), 作業領域の表, 閉じた回数)"
    );
}

/// 次の窓の一式では、確定が初回と同じ入力で同じ移動（from_x→to_x）を 1 回だけ行う。
#[test]
fn next_window_set_finalizes_once_like_first_boot() {
    let (mut world, gw) = resnap_world();
    let first = capture_logs(|| finalize_chain_once_with(&settled_sizes(), &mut world));
    let first_x = pos_of(&world, gw.char_window(1).unwrap()).map(|p| p.x);

    let _ = close_windows_for_restart(&mut world);
    let gw2 = spawn_resnap_windows(&mut world);
    let char1 = gw2.char_window(1).unwrap();
    let spawned_x = pos_of(&world, char1).map(|p| p.x);
    let second = capture_logs(|| {
        // 2 回呼ぶ: 1 回目で確定し、2 回目は一度きり（scg 7.4）で駆動しない。
        finalize_chain_once_with(&settled_sizes(), &mut world);
        finalize_chain_once_with(&settled_sizes(), &mut world);
    });

    assert_eq!(
        (
            resolved_lines(&first).len(),
            first_x,
            spawned_x,
            resolved_lines(&second).len(),
            pos_of(&world, char1).map(|p| p.x),
            world.contains_resource::<ChainFinalized>(),
        ),
        (1, Some(1205), Some(1049), 1, Some(1205), true),
        "(初回の再解決の件数, 初回の確定後の x, 次の一式の生えた x, 次の一式の再解決の件数, 次の一式の確定後の x, 確定の印)"
    );
    assert_eq!(
        resolved_lines(&first),
        resolved_lines(&second),
        "次の一式の再解決は初回と同じ from_x→to_x"
    );
}
