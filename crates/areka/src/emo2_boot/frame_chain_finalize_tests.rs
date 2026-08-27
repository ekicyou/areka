// =============================================================================
// 初期配置の確定（finalize_chain_once）の結線檻（scg 要件 7.1/7.3/7.4・design C6）
//
// 判定そのものは placement::chain_finalize の純関数檻が全網羅する。ここで固定するのは
// **結線の振る舞い**——いつ駆動し、いつ見送り、二度目に駆動しないこと。
//
// 偽装境界: resnap 檻と同じ headless World（`resnap_world`＝実 spawn_ghost_windows で
// 2 スコープの char/balloon 窓＋偽 WindowHandle＋MonitorSnapshot）に、scope ごとに寸を
// 作り分ける `PhysicalSizeSource` の fake を組み合わせる。
// =============================================================================

use super::test_support::{
    PerTargetSizes, SPAWN_SIZE_0, SPAWN_SIZE_1, pos_of, resnap_world, settled_sizes, size_of,
};
use super::*;

use crate::placement::chain_finalize::{CHAIN_FINALIZE_STALL_FRAMES, ChainFinalized};

use log_capture_kit::{LineFormat, capture_lines};
use wintf::ecs::{Point, SizeI};

// `PerTargetSizes`／`SPAWN_SIZE_*`／`settled_sizes` は多フレーム駆動ハーネス（task 3.3）と
// 共有するため `frame_test_support.rs` へ集約した（テーマ間で共有するヘルパは
// `<stem>_test_support.rs` へ・複製すると本文の同一性が壊れる）。

// -----------------------------------------------------------------------------
// 駆動して隣接を回復する
// -----------------------------------------------------------------------------

/// 実表示寸が確定したフレームで連鎖を解き直し、後続スコープだけを動かして隙間 0 にする。
///
/// フィクスチャの spawn 位置は scope0 `[1483, 1917]`・scope1 `[1049, 1327]` で 156px 空いて
/// いる。確定後は scope1 の右端が scope0 の左端に一致する。
#[test]
fn finalize_closes_the_gap_by_moving_the_follower_only() {
    let (mut world, gw) = resnap_world();
    let char0 = gw.char_window(0).unwrap();
    let char1 = gw.char_window(1).unwrap();
    assert_eq!(
        pos_of(&world, char0).map(|p| p.x),
        Some(1483),
        "前提: scope0 の spawn 位置"
    );
    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(1049),
        "前提: scope1 の spawn 位置（隣接していない）"
    );
    let y1_before = pos_of(&world, char1).map(|p| p.y);

    finalize_chain_once_with(&settled_sizes(), &mut world);

    assert_eq!(
        pos_of(&world, char0).map(|p| p.x),
        Some(1483),
        "起点スコープは動かさない（接地点は不変・7.2）"
    );
    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(1205),
        "new_x = 1483 − 278（前スコープ左端 − 自スコープ幅）"
    );
    assert_eq!(
        pos_of(&world, char1).map(|p| p.y),
        y1_before,
        "Y は動かさない（下端吸着は再アンカーが保つ・7.2）"
    );
    // 隣接不変量: scope1 の右端＝scope0 の左端。
    assert_eq!(1483 - (1205 + 278), 0, "確定後は隙間 0（scg 7.1）");
    assert!(
        world.get_resource::<ChainFinalized>().is_some(),
        "確定標識が立つ（7.4）"
    );
}

/// 寸は変えない（確定は位置だけを直す）。
#[test]
fn finalize_does_not_touch_window_sizes() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();

    finalize_chain_once_with(&settled_sizes(), &mut world);

    assert_eq!(
        size_of(&world, char1),
        Some(SizeI::new(278, 357)),
        "寸は resnap の領分ゆえ確定は触らない"
    );
}

// -----------------------------------------------------------------------------
// 一度きり（7.4）
// -----------------------------------------------------------------------------

/// 二度目以降は駆動しない。確定後にサーフェス寸が変わっても位置は動かない
/// （会話中の表情差替でキャラが横滑りしない）。
#[test]
fn finalize_runs_only_once_even_if_sizes_change_again() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();

    finalize_chain_once_with(&settled_sizes(), &mut world);
    let after_first = pos_of(&world, char1);
    assert_eq!(after_first.map(|p| p.x), Some(1205), "一度目は動く");

    // 二度目: 同じ入力でも動かない（べき等以前に駆動自体がない）。
    finalize_chain_once_with(&settled_sizes(), &mut world);
    assert_eq!(pos_of(&world, char1), after_first, "二度目は駆動しない");

    // 確定後に scope0 の窓を別寸へ変えても連鎖は解き直されない（7.4）。
    resnap_from_sizes(
        &mut world,
        [(
            0usize,
            crate::placement::resolver::SizePx { w: 500, h: 687 },
        )]
        .into_iter(),
    );
    let moved_origin_x = pos_of(&world, gw.char_window(0).unwrap()).map(|p| p.x);
    finalize_chain_once_with(
        &PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))]),
        &mut world,
    );
    assert_eq!(
        pos_of(&world, char1),
        after_first,
        "確定後のサーフェス切替では後続スコープを動かさない（7.4）"
    );
    assert!(
        moved_origin_x.is_some(),
        "前提: 起点スコープ側は resnap で再アンカーされている（観測の退化防止）"
    );
}

// -----------------------------------------------------------------------------
// 見送り条件（部分適用しない）
// -----------------------------------------------------------------------------

/// 表示未成立のスコープが 1 つでもあれば確定しない（次フレームへ送る）。
#[test]
fn finalize_defers_while_any_scope_has_not_shown_yet() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();
    let before = pos_of(&world, char1);

    finalize_chain_once_with(
        &PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, None)]),
        &mut world,
    );

    assert_eq!(pos_of(&world, char1), before, "未表示があれば動かさない");
    assert!(
        world.get_resource::<ChainFinalized>().is_none(),
        "確定させない（次フレームで再挑戦する）"
    );

    // 表示が成立したら確定する。
    finalize_chain_once_with(&settled_sizes(), &mut world);
    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(1205),
        "全スコープが揃った時点で確定する"
    );
}

/// 実表示寸と窓の寸が食い違う間は確定しない（再アンカーが未 landing＝位置が未確定）。
#[test]
fn finalize_defers_until_resnap_has_landed() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();
    let before = pos_of(&world, char1);

    // scope0 の実表示寸だけが窓の寸（434x687）と食い違う状態。
    finalize_chain_once_with(
        &PerTargetSizes::new([(0, Some((500, 687))), (1, Some(SPAWN_SIZE_1))]),
        &mut world,
    );

    assert_eq!(pos_of(&world, char1), before, "寸が揃うまで動かさない");
    assert!(
        world.get_resource::<ChainFinalized>().is_none(),
        "確定させない（古い位置で連鎖を解く取り違えを構造的に避ける）"
    );
}

// -----------------------------------------------------------------------------
// 明示的な再配置の尊重（7.3）
// -----------------------------------------------------------------------------

/// 台本の移動指令などで既定位置から動かされたスコープは引き戻さない。
#[test]
fn finalize_does_not_pull_back_an_explicitly_moved_scope() {
    let (mut world, gw) = resnap_world();
    let char1 = gw.char_window(1).unwrap();

    // 明示的な再配置（唯一の位置ライター経由）。以後 current_x != default_x になる。
    crate::placement::follow::move_window_to(&mut world, char1, 800, 1087);
    assert_eq!(pos_of(&world, char1), Some(Point { x: 800, y: 1087 }));

    finalize_chain_once_with(&settled_sizes(), &mut world);

    assert_eq!(
        pos_of(&world, char1).map(|p| p.x),
        Some(800),
        "明示的に動かされたスコープは引き戻さない（7.3）"
    );
}

// -----------------------------------------------------------------------------
// 確定が見送られ続けたときの一発診断（scg 6.5・design C6）
//
// 定石は `move_cue_move_severity_log_tests.rs` と同じ——硬化機構の唯一の定義元
// `log-capture-kit` の捕捉窓へ委譲する。判定そのものの回数は純関数檻が固定しており、ここで
// 確かめるのは**本番経路が実際に 1 行だけ出す／正常起動では 1 行も出さない**こと。
//
// 「`with_default` はスレッドローカルゆえ並行実行でも干渉しない」は**誤り**である。差し替わる
// のはスレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
// interest キャッシュは**プロセス全体で 1 つ**しかなく、その発行点を最初に踏んだスレッドの判定が
// 焼き付く（先着が勝つ）。捕捉窓を持たないスレッドの既定は `NoSubscriber` で判定は「不要」ゆえ、
// 先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。
// 共有機構は ⑴ プロセス寿命の probe 常駐 ⑵ 捕捉窓の内側での interest 再計算 ⑶ 番兵イベントに
// よる空振り検出 の 3 点でこれを塞ぐ（機序の逐条解説は `log_capture_kit` の crate doc と
// 同 crate の `src/probe.rs`）。
// -----------------------------------------------------------------------------

/// クロージャ `f` 実行中に**現在のスレッド**で発火した tracing イベントを 1 行 1 件で返す。
fn capture_logs<F: FnOnce()>(f: F) -> Vec<String> {
    let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
    lines
}

/// 捕捉行のうち指定 level の行だけを抜く。
fn lines_of_level(logs: &[String], level: &str) -> Vec<String> {
    let needle = format!("level={level}");
    logs.iter()
        .filter(|l| l.contains(&needle))
        .cloned()
        .collect()
}

/// 失敗時に貼る要約（退行注入では数千行出るため、件数＋先頭 1 行に畳む）。
fn digest(lines: &[String]) -> String {
    format!(
        "{} 行・先頭: {}",
        lines.len(),
        lines.first().map(String::as_str).unwrap_or("(無し)")
    )
}

/// 表示が永久に成立しない停滞では、閾値を超えた時点で診断が**ちょうど 1 回**出る。
///
/// 閾値の手前は 1 行も出ない（毎フレームの見送りは無音）ことも同じ檻で押さえる——ここが緩むと
/// 「起動中の正常な待ち」で毎フレーム警告が出る形へ退行する。
#[test]
fn stalled_finalize_reports_the_reason_exactly_once() {
    let (mut world, _gw) = resnap_world();
    // scope1 が永久に表示されない（初回 ShowSurface が来ない）状態。
    let stuck = PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, None)]);

    // 閾値の手前まで: 無音。
    let quiet = capture_logs(|| {
        for _ in 0..(CHAIN_FINALIZE_STALL_FRAMES - 1) {
            finalize_chain_once_with(&stuck, &mut world);
        }
    });
    assert!(
        quiet.is_empty(),
        "閾値内の見送りは無音であるべき（実際: {}）",
        digest(&quiet)
    );

    // 閾値到達以降を大きく超えて回しても、診断は 1 行だけ。
    let logs = capture_logs(|| {
        for _ in 0..(CHAIN_FINALIZE_STALL_FRAMES * 2) {
            finalize_chain_once_with(&stuck, &mut world);
        }
    });
    let warns = lines_of_level(&logs, "WARN");
    assert_eq!(
        warns.len(),
        1,
        "診断はちょうど 1 回（実際: {}）",
        digest(&warns)
    );
    let diag = &warns[0];
    assert!(
        diag.contains("scope 1"),
        "見送られたスコープを名指しする: {diag}"
    );
    assert!(
        diag.contains("初回表示が未成立"),
        "見送りの条件を名指しする: {diag}"
    );
    assert!(
        world.get_resource::<ChainFinalized>().is_none(),
        "診断は確定させない（条件が揃えば通常どおり確定できる）"
    );
}

/// 閾値内に確定した起動では診断を出さない（正常系のログ量を増やさない）。
///
/// 確定後は駆動自体が打ち切られるため、そのまま長時間回しても警告へ転じない。
#[test]
fn finalize_within_the_bounded_wait_emits_no_diagnostic() {
    let (mut world, gw) = resnap_world();
    let stuck = PerTargetSizes::new([(0, Some(SPAWN_SIZE_0)), (1, None)]);

    let logs = capture_logs(|| {
        // 閾値の手前まで待たされてから表示が揃う（＝遅い起動）。
        for _ in 0..(CHAIN_FINALIZE_STALL_FRAMES - 1) {
            finalize_chain_once_with(&stuck, &mut world);
        }
        finalize_chain_once_with(&settled_sizes(), &mut world);
        // 確定後は何フレーム回しても駆動しない（7.4）。停滞の診断へ転じないことの確認。
        for _ in 0..(CHAIN_FINALIZE_STALL_FRAMES * 2) {
            finalize_chain_once_with(&settled_sizes(), &mut world);
        }
    });

    let warns = lines_of_level(&logs, "WARN");
    assert!(
        warns.is_empty(),
        "確定した起動では診断を出さない（実際: {}）",
        digest(&warns)
    );
    assert!(
        world.get_resource::<ChainFinalized>().is_some(),
        "前提: 閾値内に確定している（観測の退化防止）"
    );
    assert_eq!(
        pos_of(&world, gw.char_window(1).unwrap()).map(|p| p.x),
        Some(1205),
        "前提: 確定は通常どおり隣接を回復している"
    );
}
