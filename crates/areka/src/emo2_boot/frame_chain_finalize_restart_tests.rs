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
    PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, capture_logs, count_level, pos_of, resnap_world,
    settled_sizes, spawn_resnap_windows,
};
use super::*;

use crate::app_exit::{WindowsEpoch, close_windows_for_restart};
use crate::placement::chain_finalize::{
    CHAIN_FINALIZE_STALL_FRAMES, ChainFinalizeStall, ChainFinalized,
};
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

// -----------------------------------------------------------------------------
// ゴースト待ちは見送りに数えない（areka-P0-restart-chain-finalize-stall 要件 1・2・3）
//
// 窓の一式が無い巡（閉じた後）と、窓は在るが相方のシェルがまだ一度も表示されていない巡は、
// ゴーストの台本しだいで長さの決まる正常な待ちなので数えない。全スコープが表示された後に
// areka 自身の理由で見送る巡だけを数え、今日と同じ 600 回目に WARN を 1 回出す。
// -----------------------------------------------------------------------------

/// 捕捉行のうち WARN の先頭 1 行（失敗のときに貼る）。
fn first_warn(logs: &[String]) -> &str {
    logs.iter()
        .find(|l| l.contains("level=WARN"))
        .map(String::as_str)
        .unwrap_or("(無し)")
}

/// 窓を全部閉じた後の窓なしの巡は数えない。何巡回しても WARN は出ず、外した見送りの数の記録も
/// 作り直さない（要件 1.1・1.3・3.1）。
#[test]
fn no_window_frames_after_close_are_not_counted() {
    let (mut world, _gw) = resnap_world();
    let _ = close_windows_for_restart(&mut world);

    let logs = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            finalize_chain_once_with(&settled_sizes(), &mut world);
        }
    });

    assert_eq!(
        (
            count_level(&logs, "WARN"),
            world
                .get_resource::<ChainFinalizeStall>()
                .map(|s| (s.deferrals, s.reported)),
            world.contains_resource::<ChainFinalized>(),
        ),
        (0, None, false),
        "(窓なしで {CHAIN_FINALIZE_STALL_FRAMES} 巡回した後の WARN の件数, \
         見送りの数の記録 (数, 報告済み), 確定の印)・WARN の先頭: {}",
        first_warn(&logs)
    );
}

/// 閉じる → 窓なし → 新しい一式で相方が未表示 → 全スコープ表示済みで areka 自身の理由の見送り、
/// を 1 本で歩く。数え始めは表示が揃ってからで、そこから 600 回目でちょうど 1 件の WARN が出て、
/// それより前と後には出ない（要件 1.4・2.1〜2.3・3.5）。
#[test]
fn next_window_set_counts_only_areka_deferrals_after_all_shown() {
    let (mut world, _gw) = resnap_world();
    let _ = close_windows_for_restart(&mut world);

    // 窓なしの巡（起こし直しで窓を外した直後）。
    let no_window = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            finalize_chain_once_with(&settled_sizes(), &mut world);
        }
    });

    // 新しい一式。scope 0 は窓の寸どおりに表示済み・相方の scope 1 だけがまだ一度も表示されて
    // いない（ゴーストが scope 1 へ最初の `\s` を出していない）。scope 0 の寸を食い違わせると
    // 走査が scope 0 で先に止まり、この区間が areka 自身の待ちとして数えられてしまう。
    let _gw2 = spawn_resnap_windows(&mut world);
    let unshown = PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, None)]);
    let partner_unshown = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            finalize_chain_once_with(&unshown, &mut world);
        }
    });

    // 全スコープが表示された後、scope 0 の実表示寸（500x687）が窓の寸（434x687）と食い違った
    // まま＝再アンカーが未 landing（areka 自身の待ち）。確定の処理は再スナップを呼ばないので、
    // この食い違いは何巡回しても解けない。
    let not_landed = PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))]);
    let before_threshold = capture_logs(|| {
        for _ in 0..(CHAIN_FINALIZE_STALL_FRAMES - 1) {
            finalize_chain_once_with(&not_landed, &mut world);
        }
    });
    let at_threshold = capture_logs(|| finalize_chain_once_with(&not_landed, &mut world));
    let after_threshold = capture_logs(|| {
        for _ in 0..CHAIN_FINALIZE_STALL_FRAMES {
            finalize_chain_once_with(&not_landed, &mut world);
        }
    });

    assert_eq!(
        (
            count_level(&no_window, "WARN"),
            count_level(&partner_unshown, "WARN"),
            count_level(&before_threshold, "WARN"),
            count_level(&at_threshold, "WARN"),
            count_level(&after_threshold, "WARN"),
        ),
        (0, 0, 0, 1, 0),
        "WARN の件数 (窓なしの {n} 巡, 相方が未表示の {n} 巡, 表示が揃ってから {m} 巡, \
         表示が揃ってから {n} 回目, その後の {n} 巡)・最初の WARN: {}",
        [
            &no_window,
            &partner_unshown,
            &before_threshold,
            &at_threshold,
            &after_threshold
        ]
        .into_iter()
        .map(|l| first_warn(l))
        .find(|w| *w != "(無し)")
        .unwrap_or("(無し)"),
        n = CHAIN_FINALIZE_STALL_FRAMES,
        m = CHAIN_FINALIZE_STALL_FRAMES - 1,
    );
    let diag = first_warn(&at_threshold);
    assert!(
        diag.contains("scope 0") && diag.contains("再アンカーが未 landing"),
        "WARN は areka 自身の理由（scope 0 の再アンカーが未 landing）を名指しする: {diag}"
    );
    assert!(
        !world.contains_resource::<ChainFinalized>(),
        "停滞の間は確定させない"
    );
}
