//! 窓を作る決定論テスト（要件 1.2・1.3・4.1〜4.6・4.8・4.9・4.11）。
//!
//! 本番の shiori アクター（実 [`areka_kanade::ShioriConnection`]＋即終了する x64 の stand-in
//! helper）を [`spawn_window_actor`] で起こし、要求の無い待機中のホスト窓へ別スレッド（テスト
//! スレッド）から同期送出して、アクターのスレッドが窓のメッセージを取り出し続けているかを見る。
//! backend に fake を使わないのは、`ShioriConnection::on_idle` から窓の取り出しへ委譲する 1 行
//! まで本番の経路を踏ませるため（fake が同じ部品へ委譲する形では、その 1 行が欠けても緑になる）。

use std::sync::OnceLock;
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

use areka_kanade::shiori::real::IDLE_INTERVAL;
use areka_kanade::{KanadeMsg, ShioriMsg};
use shiori_host32_ipc::{MsgTag, hwnd_from_u32, send_copydata, send_copydata_response};

use super::common::{DEFAULT_TIMEOUT, join_bounded, spawn_window_actor};

/// 速いテストの同期送出の上限。
const SEND_BOUND: Duration = Duration::from_secs(2);
// 上限が保守周期の 4 倍を下回ると、負荷やスケジューラの遅れで間欠的に赤くなりうる。
// 周期を動かしたときに余裕が黙って薄くならないよう、比をコンパイル時に固定する。
const _: () = assert!(SEND_BOUND.as_millis() >= 4 * IDLE_INTERVAL.as_millis());

/// プロセス生存時間（の下限）。このバイナリで最初に呼ばれた時点からの経過を返す。
///
/// OS の応答なし判定はプロセス開始から一定の猶予（20〜30 秒）の後で厳しくなるため、診断文に
/// 載せて「猶予の内か外か」を読めるようにする。std にはプロセス開始時刻を得る API が無く、
/// areka-kanade は Win32 API crate に依存しないため、テストが最初に計時を始めた時点を起点に
/// する（実際の生存時間はこれ以上）。
fn uptime() -> Duration {
    static FIRST_SEEN: OnceLock<Instant> = OnceLock::new();
    FIRST_SEEN.get_or_init(Instant::now).elapsed()
}

/// 待機中のホスト窓へ同期送出すると上限内に成功復帰し、即終了した stand-in の死活報告が
/// 手空きの初回に届く（全体 5 秒以内）。2 通目が来ないことは短い期限では主張しない
/// （要件 4.8）——手空きが回った証拠つきで `real_idle_tests.rs` の Test-B が見張る。
#[test]
fn sync_send_to_idle_window_returns_within_bound() {
    let _ = uptime();
    let started = Instant::now();
    let actor = spawn_window_actor();
    let host = hwnd_from_u32(actor.hwnd);

    // 送出元にも自窓を渡す（受け手は送出元を使わない）。アクターは要求を 1 通も受けていない。
    let send_started = Instant::now();
    let result = send_copydata_response(host, host, MsgTag::Response, b"idle-pump", SEND_BOUND);
    let elapsed = send_started.elapsed();
    let uptime_at_send = uptime();

    // 死活報告は起動から SEND_BOUND の内に届くはず（手空きの初回は IDLE_INTERVAL 後）。
    let deadline = started + SEND_BOUND;
    let first_down = down_before(&actor.down_rx, deadline);

    let _ = actor.shiori_tx.send(ShioriMsg::Close);
    join_bounded("idle-pump shiori join", DEFAULT_TIMEOUT, actor.handle)
        .expect("shiori アクターが正常に終わる");

    let delivered = result.is_ok();
    let diag = format!(
        "delivered={delivered} elapsed={elapsed:?} bound={SEND_BOUND:?} \
         uptime_lower_bound={uptime_at_send:?} result={result:?} \
         shiori_down_first={first_down:?} \
         total={:?}",
        started.elapsed()
    );
    assert!(
        delivered && elapsed < SEND_BOUND,
        "待機中のホスト窓への同期送出が上限内に成功復帰する（要件 1.2）: {diag}"
    );
    assert!(
        first_down.is_some(),
        "即終了した stand-in の死活報告が手空きの初回に届く: {diag}"
    );
}

/// 遅いテストの各段の待ち（要求を 1 通も送らない時間）。スレッドが応答なしと見なされる 14 秒を
/// 確実に超える最小の切りの良い値（`shiori-host32-helper` の同型テストと同値）。
const SLOW_IDLE: Duration = Duration::from_secs(20);
/// 遅いテストの同期送出の上限（helper 本体の `REPLY_TIMEOUT` と同値）。
const SLOW_SEND_BOUND: Duration = Duration::from_secs(5);
/// 遅いテスト全体の上限。構造上の最悪でも 起動待ち 5＋20＋5＋20＋5＋join＝60 秒程度。
const SLOW_CAGE_BOUND: Duration = Duration::from_secs(90);

/// 要求の無い待機中のホスト窓へ、相手が応答なしなら待たずに打ち切る送り方（`SMTO_ABORTIFHUNG`）で
/// 20 秒待ち → 送出① → 20 秒待ち → 送出② と送り、どちらも届くこと（要件 1.3・4.4・4.5）。
///
/// OS の打ち切りは「プロセスが起きてから概ね 20〜30 秒を過ぎた」かつ「宛先の窓のスレッドが 14 秒
/// 以上メッセージを取り出していない」ときに起きる。送出②の時点ではプロセスの生存は 40 秒を超え、
/// アクターは 20 秒要求を受けていないので両方の条件が揃う。待機中に窓のメッセージを取り出さない
/// 受信ループでは、① が上限まで待って失敗し ② が即座に失敗する（② の即時失敗が打ち切りの署名）。
#[test]
fn abortifhung_send_reaches_window_idle_for_twenty_seconds() {
    let _ = uptime();
    let started = Instant::now();
    let actor = spawn_window_actor();
    let host = hwnd_from_u32(actor.hwnd);

    std::thread::sleep(SLOW_IDLE);
    let uptime_at_first = uptime();
    let first_started = Instant::now();
    let first = send_copydata(
        host,
        host,
        MsgTag::Response,
        b"hung-cage-1",
        SLOW_SEND_BOUND,
    );
    let first_elapsed = first_started.elapsed();

    std::thread::sleep(SLOW_IDLE);
    let uptime_at_second = uptime();
    let second_started = Instant::now();
    let second = send_copydata(
        host,
        host,
        MsgTag::Response,
        b"hung-cage-2",
        SLOW_SEND_BOUND,
    );
    let second_elapsed = second_started.elapsed();

    let _ = actor.shiori_tx.send(ShioriMsg::Close);
    let joined = join_bounded("idle-pump-slow shiori join", DEFAULT_TIMEOUT, actor.handle);

    let send_failures = [first.is_err(), second.is_err()]
        .iter()
        .filter(|failed| **failed)
        .count();
    let total = started.elapsed();
    let diag = format!(
        "first={first:?} first_elapsed={first_elapsed:?} \
         second={second:?} second_elapsed={second_elapsed:?} \
         idle={SLOW_IDLE:?} send_bound={SLOW_SEND_BOUND:?} \
         uptime_lower_bound_at_first={uptime_at_first:?} \
         uptime_lower_bound_at_second={uptime_at_second:?} \
         send_failures={send_failures} total={total:?}"
    );
    joined.expect("shiori アクターが正常に終わる");
    assert!(
        first.is_ok(),
        "送出①が届く（20 秒の待機後・要件 1.3）: {diag}"
    );
    assert!(
        second.is_ok(),
        "送出②が届く＝待機中のホスト窓が応答なし判定に落ちていない（要件 1.3・4.4）: {diag}"
    );
    assert_eq!(send_failures, 0, "送出の失敗が 0 回である: {diag}");
    assert!(
        total < SLOW_CAGE_BOUND,
        "全体が上限 {SLOW_CAGE_BOUND:?} の内で終わる（要件 4.5）: {diag}"
    );
}

/// `deadline` までに届いた最初の `ShioriDown` の理由を返す（届かなければ `None`）。
fn down_before(rx: &std::sync::mpsc::Receiver<KanadeMsg>, deadline: Instant) -> Option<String> {
    loop {
        let wait = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(KanadeMsg::ShioriDown { reason, .. }) => return Some(reason),
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
        }
    }
}
