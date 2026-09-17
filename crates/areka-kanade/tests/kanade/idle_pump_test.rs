//! 窓を作る決定論テスト（要件 1.2・4.1〜4.3・4.6・4.8・4.9・4.11）。
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
use shiori_host32_ipc::{MsgTag, hwnd_from_u32, send_copydata_response};

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
/// 手空きの初回にちょうど 1 通届く（全体 5 秒以内）。
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
    // 1 通目を受けたら、手空きが少なくとも 2 回以上回る長さだけ待って 2 通目が来ないことを見る。
    // 1 通目が来ていなければ待たない（全体 5 秒以内を保つ）。
    let second_down = first_down.is_some()
        && down_before(&actor.down_rx, Instant::now() + 3 * IDLE_INTERVAL).is_some();

    let _ = actor.shiori_tx.send(ShioriMsg::Close);
    join_bounded("idle-pump shiori join", DEFAULT_TIMEOUT, actor.handle)
        .expect("shiori アクターが正常に終わる");

    let delivered = result.is_ok();
    let diag = format!(
        "delivered={delivered} elapsed={elapsed:?} bound={SEND_BOUND:?} \
         uptime_lower_bound={uptime_at_send:?} result={result:?} \
         shiori_down_first={first_down:?} shiori_down_second={second_down} \
         total={:?}",
        started.elapsed()
    );
    assert!(
        delivered && elapsed < SEND_BOUND,
        "待機中のホスト窓への同期送出が上限内に成功復帰する（要件 1.2）: {diag}"
    );
    assert!(
        first_down.is_some() && !second_down,
        "即終了した stand-in の死活報告が手空きの初回にちょうど 1 通届く: {diag}"
    );
}

/// `deadline` までに届いた最初の `ShioriDown` の理由を返す（届かなければ `None`）。
fn down_before(rx: &std::sync::mpsc::Receiver<KanadeMsg>, deadline: Instant) -> Option<String> {
    loop {
        let wait = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(wait) {
            Ok(KanadeMsg::ShioriDown { reason }) => return Some(reason),
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
        }
    }
}
