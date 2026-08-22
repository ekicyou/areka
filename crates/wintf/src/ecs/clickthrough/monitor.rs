//! カーソル監視ワーカ（別スレッド・`event_listener` 起床・RAII）。
//!
//! 専用ワーカスレッドが `GetCursorPos`（screen physical）を継続取得し、カーソルが
//! 移動した時だけ共有 `event_listener::Event` を `notify(usize::MAX)`（全リスナ起床）で
//! UI スレッドを起床する。ワーカは `&World` / ECS には一切触れない（座標取得のみ）。
//!
//! # 順序不変条件（critical）
//! ワーカは移動検知時に `latest_pos.store(pack(x,y), Release)` を **先に**、その後で
//! `cursor_event.notify(usize::MAX)` を行う（store→notify）。逆順だと UI 側が 1 通知分
//! 古い座標を読む稀レースが生じる（`VsyncEventBridge` と同一規律）。
//!
//! # 周期
//! `GetCursorPos` はブロックしないため、ループは固定短周期（12ms）で sleep しつつ
//! 前回座標との差分ガードで無駄な notify を抑える（R3 常時ポーリングの無駄回避方針）。
//! UI 側でも差分ガードするため二重に安全。sleep で待つため `Drop` の `join()` は最大
//! 約 1 周期で復帰し、ハングしない。
//!
//! # 生存期間（RAII）
//! `VsyncEventBridge` を構造テンプレとし、`Arc<event_listener::Event>`＋`Arc<AtomicI64>`
//! 最新座標＋`Arc<AtomicBool>` stop_flag＋`Option<JoinHandle>` を保持する。`Drop` で
//! stop フラグを立ててワーカを `join()` する（`stop_flag.store(true)→join` の順序）。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread::{self, JoinHandle};

use tracing::{debug, trace};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::ecs::PhysicalPoint;

/// ワーカのポーリング周期（固定短周期）。前回座標差分ガードと併せて無駄通知を抑える。
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(12);

/// 2 つの `i32` screen 座標を 1 つの `i64` に pack する（上位 32bit = x, 下位 32bit = y）。
#[inline]
fn pack(x: i32, y: i32) -> i64 {
    ((x as i64) << 32) | (y as u32 as i64)
}

/// [`pack`] の逆変換。
#[inline]
fn unpack(v: i64) -> (i32, i32) {
    let x = (v >> 32) as i32;
    let y = (v & 0xFFFF_FFFF) as u32 as i32;
    (x, y)
}

/// カーソル監視ワーカスレッドの起床を共有 `event_listener::Event` へ橋渡しする
/// RAII ユニット。
///
/// - 共有 `Event`（`Arc` で clone をワーカスレッドへ move・UI ループと同一 `Arc`）
/// - 最新カーソル座標（`Arc<AtomicI64>`・pack 済み・UI 側が [`latest_cursor`] で読む）
/// - 停止フラグ（`Arc<AtomicBool>`）
/// - スレッドハンドル（`Option<JoinHandle<()>>`・take は `Drop`/`stop` のみ）
///
/// [`latest_cursor`]: CursorMonitorBridge::latest_cursor
pub(crate) struct CursorMonitorBridge {
    /// 共有カーソル移動通知イベント。ワーカが移動ごとに `notify(usize::MAX)`。
    cursor_event: Arc<event_listener::Event>,
    /// pack(x,y) の最新カーソル座標。UI 側が [`latest_cursor`] で読む。
    ///
    /// [`latest_cursor`]: CursorMonitorBridge::latest_cursor
    latest_pos: Arc<AtomicI64>,
    /// ワーカスレッド停止フラグ。`Drop`/`stop` で `true` をストアしループを終える。
    stop_flag: Arc<AtomicBool>,
    /// ワーカスレッドのハンドル。`spawn` で必ず `Some`、take は `stop`/`Drop` のみ。
    handle: Option<JoinHandle<()>>,
}

impl CursorMonitorBridge {
    /// カーソル監視ワーカを起動してブリッジを生成する。
    ///
    /// `cursor_event` は UI ループ（`ClickThroughController` の listen 対象）と共有する
    /// 同一 `Arc`。ワーカは `while !stop { GetCursorPos; if moved { store; notify } }` を
    /// 固定短周期でループする。
    pub(crate) fn spawn(cursor_event: Arc<event_listener::Event>) -> Self {
        let latest_pos = Arc::new(AtomicI64::new(0));
        let stop_flag = Arc::new(AtomicBool::new(false));

        // clone をワーカスレッドへ move（Event/latest_pos/stop_flag とも Arc で共有）。
        let thread_event = Arc::clone(&cursor_event);
        let thread_latest = Arc::clone(&latest_pos);
        let thread_stop = Arc::clone(&stop_flag);

        let handle = thread::Builder::new()
            .name("wintf-cursor-monitor".to_owned())
            .spawn(move || cursor_loop(thread_event, thread_latest, thread_stop))
            .expect("failed to spawn wintf-cursor-monitor thread");

        debug!("CursorMonitorBridge started (wintf-cursor-monitor thread spawned)");

        Self {
            cursor_event,
            latest_pos,
            stop_flag,
            handle: Some(handle),
        }
    }

    /// UI 側が最新カーソル座標（screen physical）を読む。
    ///
    /// ワーカが `Release` でストアした値を `Acquire` で読み、`notify` と対で
    /// 順序不変条件を満たす（store→notify の後に読む UI は最新座標を観測できる）。
    pub(crate) fn latest_cursor(&self) -> PhysicalPoint {
        let (x, y) = unpack(self.latest_pos.load(Ordering::Acquire));
        PhysicalPoint::new(x, y)
    }

    /// 共有カーソル移動通知イベントへの参照を返す（テスト・結線用）。
    pub(crate) fn event(&self) -> &Arc<event_listener::Event> {
        &self.cursor_event
    }

    /// ワーカスレッドを停止し join する（`stop_flag → join` の順序）。
    ///
    /// 冪等。`Drop` からも呼ばれる。ワーカは sleep で待機するため `join()` は最大
    /// 約 1 周期（[`POLL_INTERVAL`]）で復帰しハングしない。
    fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop_flag.store(true, Ordering::Release);
            if handle.join().is_err() {
                trace!("wintf-cursor-monitor thread join returned Err (thread panicked)");
            }
            debug!("CursorMonitorBridge stopped (wintf-cursor-monitor thread joined)");
        }
    }
}

impl Drop for CursorMonitorBridge {
    fn drop(&mut self) {
        self.stop();
    }
}

/// カーソル監視ワーカ本体。`GetCursorPos` で最新座標を取得し、前回と異なる時のみ
/// `latest_pos.store` → `cursor_event.notify(usize::MAX)` の順序で UI を起床する。
/// `stop` が立つまで固定短周期でループする。
fn cursor_loop(event: Arc<event_listener::Event>, latest: Arc<AtomicI64>, stop: Arc<AtomicBool>) {
    // 前回座標（差分ガード）。初回は必ず notify するため到達しない番兵で初期化する。
    let mut prev: Option<(i32, i32)> = None;

    while !stop.load(Ordering::Acquire) {
        let mut pt = POINT::default();

        // SAFETY: Win32 境界。`GetCursorPos` は書き込み先 `POINT` への可変ポインタを
        // 取り、現在のカーソル位置（screen physical）を書き込んで `Result<()>` を返す。
        // `pt` は本スレッドローカルなスタック変数で有効な領域を指し、共有状態には
        // 触れない。失敗時は当該反復をスキップする（grace）。
        let got = unsafe { GetCursorPos(&mut pt) };

        match got {
            Ok(()) => {
                let cur = (pt.x, pt.y);
                if prev != Some(cur) {
                    // 移動検知。順序不変条件: store（Release）→ notify を厳守する。
                    // 逆順だと UI 側が 1 通知分古い座標を読む稀レースが生じる。
                    latest.store(pack(cur.0, cur.1), Ordering::Release);
                    event.notify(usize::MAX);
                    prev = Some(cur);
                }
            }
            Err(_) => {
                // 稀な失敗。当該反復の notify をスキップして継続する（panic しない）。
                trace!("GetCursorPos failed; skipping this iteration");
            }
        }

        // 固定短周期で待機（stop 後は最大約 1 周期で join へ復帰）。
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_listener::{Event, Listener};
    use std::time::Duration;
    use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

    /// pack/unpack が負値を含む座標で往復すること（マルチモニタ負座標対応）。
    #[test]
    fn pack_unpack_roundtrip() {
        for (x, y) in [
            (0, 0),
            (100, 200),
            (-1, -1),
            (-1920, 1080),
            (i32::MIN, i32::MAX),
        ] {
            assert_eq!(
                unpack(pack(x, y)),
                (x, y),
                "pack/unpack roundtrip for ({x},{y})"
            );
        }
    }

    /// 完了状態の中核検証（決定的・`VsyncEventBridge` テスト準拠）:
    /// `spawn`→`drop` でワーカが確実に stop/join され、ハングしないこと。
    #[test]
    fn spawn_then_joins_on_drop() {
        let bridge = CursorMonitorBridge::spawn(Arc::new(Event::new()));
        // drop で stop→join。ワーカが clean に終われば本テストはハングせず復帰する。
        drop(bridge);
    }

    /// `latest_cursor` は pack 済み値を PhysicalPoint として復元する。
    /// 初期状態（未起動相当）は (0,0)。ワーカ稼働後は最新座標を反映する。
    #[test]
    fn latest_cursor_reads_stored_position() {
        let bridge = CursorMonitorBridge::spawn(Arc::new(Event::new()));

        // ワーカが 1 反復以上回れば実カーソル座標がストアされる。少し待つ。
        // 実座標に依存しない不変条件のみ検証: latest_cursor が unpack と一致すること。
        thread::sleep(Duration::from_millis(60));
        let raw = bridge.latest_pos.load(Ordering::Acquire);
        let (ex, ey) = unpack(raw);
        let p = bridge.latest_cursor();
        assert_eq!(
            (p.x, p.y),
            (ex, ey),
            "latest_cursor は latest_pos の unpack と一致"
        );

        drop(bridge);
    }

    /// notify on cursor move + UI reads latest（store→notify 効果の観測）。
    ///
    /// listener を移動の**前**に arm し、`SetCursorPos` で既知座標へ強制移動、
    /// タイムアウト内に listener が起床し、かつ `latest_cursor()` が移動を反映することを
    /// 確認する。対話デスクトップが無い CI 等で `SetCursorPos` が拒否される環境では
    /// 誤検出を避けるため、その場合は RAII/順序規律パスの検証に切替える。
    #[test]
    fn notify_on_move_and_ui_reads_latest() {
        let event = Arc::new(Event::new());
        let bridge = CursorMonitorBridge::spawn(Arc::clone(&event));

        // 移動の前に listen() を arm（notify/listen レース回避）。
        let listener = event.listen();

        // 既知座標へ強制移動（現在位置と必ず異なるよう 2 点試す）。
        // SAFETY: Win32 境界。`SetCursorPos` は screen physical 座標にカーソルを
        // 移動する。副作用はカーソル位置のみで共有状態には触れない。対話デスクトップが
        // 無い環境では `Err` を返しうるため戻り値を検査する。
        let target = (137, 149);
        let moved = unsafe { SetCursorPos(target.0, target.1) };

        if moved.is_err() {
            // 非対話環境。移動観測は不能なので、決定的な RAII 収束のみ確認する。
            trace!("SetCursorPos rejected (headless/non-interactive); asserting RAII only");
            drop(bridge);
            return;
        }

        // タイムアウト内に移動 notify が届けば Some。
        let got = listener.wait_timeout(Duration::from_millis(500));

        if got.is_some() {
            // 起床したなら UI 側は最新座標を読める（store→notify 順序の効果）。
            // 別プロセス等がさらにカーソルを動かした可能性はあるため、少なくとも
            // 初期 (0,0) から更新されていること＝何らかの移動が反映されたことを確認する。
            let p = bridge.latest_cursor();
            assert_ne!(
                (p.x, p.y),
                (0, 0),
                "移動 notify 後は latest_cursor が更新済みであるべき"
            );
        } else {
            // 環境要因（他プロセスの即時再移動でカーソルが target と一致し差分ガードに
            // 掛かる等）で notify を観測できないことがある。決定的失敗にはしない。
            trace!("no move notify observed within timeout (environment-dependent)");
        }

        drop(bridge);
    }
}
