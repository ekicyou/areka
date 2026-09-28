//! 終了指示の有無による毎フレームの処理の分岐（areka-P0-frame-phases-after-exit タスク 2.1・
//! 要件 1.6／1.7／3.1／3.2／3.4）。
//!
//! 3 つの組み立てとも同じ: 既存の補助で結線を組んで装着済みにし、未登録の対象への `Hide` を
//! 1 件送り、素の World に挿す。差は終了の受け口（`wintf::AppExit`）の有無と、指示済みかだけ。
//! 記録の捕捉に入るのは `emo2_frame_system` の 1 回の呼び出しだけである。
//!
//! - 指示済み: 相を 1 つも回さない＝ERROR 0 件・WARN 0 件・読み飛ばしの記録 1 件（DEBUG）・
//!   結線は World に残り・受信端に指令が 1 件残る。
//! - 未指示と受け口なし: 今日どおり全相を回す＝`apply(Hide)` の ERROR 1 件・読み飛ばしの記録 0 件・
//!   受信端は空・結線は World に残る。WARN の件数と ERROR の総数は確かめない（design D6: 素の
//!   World では作業領域の同期が WARN を出すなど、本件の対象でない記録が混ざる）。

use std::sync::mpsc;

use super::test_support::{capture_logs, count_level, headless_wiring_with, zero_clock};
use super::*;

const SKIP_EVENT: &str = "frame_phases_skipped_after_exit";

/// 終了の受け口の置き方。
enum ExitState {
    /// 受け口を挿し、終了を指示済みにする。
    Requested,
    /// 受け口を挿すが、終了は指示しない。
    NotRequested,
    /// 受け口を挿さない。
    Absent,
}

/// 共通の組み立てで World を作り、`emo2_frame_system` を 1 回だけ捕捉の中で回して、
/// 捕捉した行を返す。
fn run_one_frame(exit: ExitState) -> (World, Vec<String>) {
    let (tx, rx) = mpsc::channel::<PresentCommand>();
    let mut wiring = headless_wiring_with(rx, zero_clock());
    // 装着済みにする（本番は装着の相が立てる）。未装着のままだと指令は取り出されず、
    // 未指示の側で「適用された」ことを観測できない。
    wiring.attached = true;
    tx.send(PresentCommand::Hide {
        target: TargetId(0),
        reply: None,
    })
    .expect("受信端は結線が持っているので送信は成功する");

    let mut world = World::new();
    world.insert_non_send(wiring);
    match exit {
        ExitState::Requested => {
            let app_exit = wintf::AppExit::new();
            // 終了の指示は捕捉の外で行う（捕捉に入るのは毎フレームの処理だけ）。
            app_exit.request_exit();
            world.insert_non_send(app_exit);
        }
        ExitState::NotRequested => world.insert_non_send(wintf::AppExit::new()),
        ExitState::Absent => {}
    }

    let logs = capture_logs(|| emo2_frame_system(&mut world));
    (world, logs)
}

/// 捕捉した行のうち読み飛ばしの記録の行。
fn skip_lines(logs: &[String]) -> Vec<&String> {
    logs.iter().filter(|l| l.contains(SKIP_EVENT)).collect()
}

/// 受信端に残っている指令の件数（確認の最後に 1 回だけ呼ぶ）。
fn remaining_commands(world: &mut World) -> usize {
    world
        .get_non_send_mut::<Emo2Wiring>()
        .expect("結線は World に残っている")
        .drain_received()
        .len()
}

/// 要件 3.1（1.1〜1.4）: 終了が指示済みなら、相を 1 つも回さずに読み飛ばしの記録を 1 行残して戻る。
#[test]
fn frame_phases_are_skipped_once_exit_is_requested() {
    let (mut world, logs) = run_one_frame(ExitState::Requested);

    assert_eq!(
        count_level(&logs, "ERROR"),
        0,
        "指示済みの巡では指令を適用しない＝ERROR 0 件: {logs:?}"
    );
    assert_eq!(
        count_level(&logs, "WARN"),
        0,
        "指示済みの巡では作業領域の同期も走らない＝WARN 0 件: {logs:?}"
    );
    let skips = skip_lines(&logs);
    assert_eq!(skips.len(), 1, "読み飛ばしの記録はちょうど 1 行: {logs:?}");
    assert!(
        skips[0].contains("level=DEBUG"),
        "読み飛ばしの記録は debug の水準: {}",
        skips[0]
    );
    assert!(
        world.get_non_send::<Emo2Wiring>().is_some(),
        "結線は取り除かれず World に残る"
    );
    assert_eq!(
        remaining_commands(&mut world),
        1,
        "受信端から指令を 1 件も取り出さない"
    );
}

/// 要件 3.2（1.6）と 1.7: 未指示のときと受け口が無いときは、今日どおり全相を回す。
#[test]
fn frame_phases_run_as_today_when_exit_is_not_requested_or_absent() {
    for (label, exit) in [
        ("未指示", ExitState::NotRequested),
        ("受け口なし", ExitState::Absent),
    ] {
        let (mut world, logs) = run_one_frame(exit);

        let hide_errors = logs
            .iter()
            .filter(|l| l.contains("level=ERROR") && l.contains("apply(Hide)"))
            .count();
        assert_eq!(
            hide_errors, 1,
            "{label}: 指令が取り出されて適用される＝`apply(Hide)` の ERROR 1 件: {logs:?}"
        );
        assert_eq!(
            skip_lines(&logs).len(),
            0,
            "{label}: 読み飛ばしの記録は 0 行: {logs:?}"
        );
        assert!(
            world.get_non_send::<Emo2Wiring>().is_some(),
            "{label}: 結線は World に戻っている"
        );
        assert_eq!(
            remaining_commands(&mut world),
            0,
            "{label}: 受信端は空（指令は取り出された）"
        );
    }
}
