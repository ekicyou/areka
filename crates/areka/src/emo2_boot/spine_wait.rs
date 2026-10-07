//! spine の有界待機の部品（締切・空回しの予算・休み・`spin_wait_until`・`run_bounded`・`join_bounded`）。
//!
//! `spine.rs` から中身を変えずに移した。親の `spine.rs` が同じ名前で出し直すので、兄弟のテストは
//! `super::SPIN_WAIT` などの呼び名のまま使える。

use std::sync::mpsc;
use std::time::{Duration, Instant};

use areka_actor::{ActorError, ActorHandle};

/// **別スレッドの進行を待つ**有界スピンの猶予（sleep 不使用・`yield_now` のみで回す協調ループ用）。
///
/// # 反復回数で打ち切ってはならない
///
/// `yield_now()` のビジーウェイトでは **反復回数が経過時間の代理にならない**。CPU 競合下
/// （`cargo test --workspace` の並行実行・ウイルス対策の再スキャン等）では、待っている相手スレッドが
/// 一度も走らないまま数十万回の yield が尽きうる（steering
/// `areka-defender-rescan-starves-cooperative-test-loops`）。実測では旧 `for _ in 0..100_000u32` 形が
/// 並行実行時に**約 6%**（単独実行 30 回中 2 回）で待機に失敗し、`boot_calls` が空のまま照合へ落ちていた。
///
/// # 適用範囲
///
/// 分類の軸は「Tick を注入するか」ではなく **「別スレッドの進行を待っているか」** である。
///
/// - **対象（[`spin_wait_until`] を使う）**: 待機対象が別スレッドの進行であり、各反復が**何も進めない**
///   純粋なポーリング（`non_status_calls()` / `drain_received()` を読むだけのループ）。
/// - **対象（本猶予の期限だけを借りる。ただし期限は十分条件ではない）**: 各反復で
///   `inject_dispatcher_tick` により**系を進めつつ**、同じ反復で `drain_received()` 等により
///   **別スレッドの結果も待つ**ハイブリッドのループ（[`spin_wait_until`] は純粋ポーリング専用ゆえ
///   流用しない）。ここでは打ち切りを時刻期限にするだけでは足りず、**注入する simulated time が
///   待っている観測を追い越さない**ことを構造で保証しなければならない。追い越しうる時刻には必ず
///   上限（頭打ち）を置くこと。実測: S2 Phase 1 は毎反復 `now += 5` が 210 反復（実時間 ~0.6 秒）で
///   Clear cue の時刻を跨ぎ、リビール観測が間に合わないと**待っている条件そのものが破壊**されて
///   永久に不成立になる——並行実行時に約 2% 失敗し、期限を 30 秒に延ばしても 50 回中 3 回失敗した。
///   期限は「壊れていない条件を待つ」ためのものであり、条件が壊れるレースは期限では直らない。
/// - **非対象**: 別スレッドの進行を待たず、注入 Tick 列そのものが仕事量であるループ。時刻で打ち切ると
///   注入列が短くなり意味が変わる。
///
/// 猶予は通常経路（マイクロ秒〜ミリ秒）に対して桁違いに大きく取る。期限切れは呼び手の assert が
/// 落として原因を名指しするので hang しない。
pub(super) const SPIN_WAIT: Duration = Duration::from_secs(30);

/// `yield_now()` の密スピンを続ける上限反復数。これを超えたら [`BACKOFF_SLEEP`] へ落とす。
///
/// # なぜ純 yield のままではいけないか
/// `yield_now()` の密ループは **1 コアを占有し続ける**。反復上限だけで打ち切っていた旧実装は
/// 早々に諦めるためこれが顕在化しなかったが、時刻期限（[`SPIN_WAIT`]）へ変えると失敗経路が
/// 数十秒フルにコアを焼き、**同一バイナリで並走する他テストを飢餓させて別の flake を生む**
/// （実測: 純 yield ＋ 30 秒期限で 50 回中 5 回・無関係な 3 テストが巻き添えで失敗し、総所要が
/// 230 秒→1490 秒へ悪化した）。待機は「速い経路を邪魔しない」と同時に「長引いたら CPU を返す」
/// 必要がある。
///
/// # 予算を旧実装の上限に揃える理由
/// 予算を小さく取る（実測: 10_000）と、**正常でも数百 ms 待つ呼出点**が [`BACKOFF_SLEEP`] の
/// 1ms 粒度に律速され、通常経路が 1 回 4.4 秒 → 10.3 秒へ倍増した。旧実装の最大予算（1_000_000）
/// をそのまま踏襲すれば、**成功する待機は旧実装と完全に同じ密スピンで完了**し、予算を使い切った
/// ——旧実装なら諦めて assert を落としていた——場合にのみ sleep へ落ちる。すなわち本ヘルパは
/// 「旧挙動 ＋ 諦めずに時刻期限まで CPU を返しながら待つ」の純増であり、通常経路を一切遅くしない。
pub(super) const SPIN_YIELD_BUDGET: u32 = 1_000_000;

/// 密スピンを使い切った後の 1 回あたり待機。CPU を明け渡し、相手スレッドに実行機会を与える。
pub(super) const BACKOFF_SLEEP: Duration = Duration::from_millis(1);

/// `cond` が真になるまで [`SPIN_WAIT`] の範囲で待つ。真になったら `true`、期限切れなら `false`。
///
/// 速い経路（通常はマイクロ秒）は `yield_now()` の密スピンで待ち time-to-detect を犠牲にしない。
/// [`SPIN_YIELD_BUDGET`] を超えたら [`BACKOFF_SLEEP`] の短い sleep へ落として**コア占有をやめる**。
/// 本ファイルの「sleep 不使用」規律は *系を進める* Tick 注入ループの決定論を守るためのものであり、
/// 別スレッドの進行を待つだけの本ヘルパには当たらない（待機は観測内容を変えない）。
pub(crate) fn spin_wait_until(mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + SPIN_WAIT;
    let mut spun = 0u32;
    loop {
        if cond() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        if spun < SPIN_YIELD_BUDGET {
            spun += 1;
            std::thread::yield_now();
        } else {
            std::thread::sleep(BACKOFF_SLEEP);
        }
    }
}

/// クロージャ `f` を別スレッドで実行し有界時間で完了を観測する（ghost spine の `run_bounded` 同旨）。
pub(crate) fn run_bounded<F: FnOnce() + Send + 'static>(what: &str, timeout: Duration, f: F) {
    let (done_tx, done_rx) = mpsc::sync_channel::<()>(0);
    std::thread::spawn(move || {
        f();
        let _ = done_tx.send(());
    });
    assert!(
        done_rx.recv_timeout(timeout).is_ok(),
        "'{what}' did not complete within {timeout:?} (possible hang)"
    );
}

/// `ActorHandle::join` を有界時間で観測する（ghost spine の `join_bounded` 同旨）。
pub(super) fn join_bounded(
    what: &str,
    timeout: Duration,
    handle: ActorHandle,
) -> Result<(), ActorError> {
    let (res_tx, res_rx) = mpsc::sync_channel::<Result<(), ActorError>>(0);
    std::thread::spawn(move || {
        let _ = res_tx.send(handle.join());
    });
    match res_rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => panic!("'{what}' join did not complete within {timeout:?} (possible hang)"),
    }
}
