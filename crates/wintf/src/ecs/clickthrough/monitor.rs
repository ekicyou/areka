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
//! 前回座標との差分ガードで無駄な notify を抑える。
//! UI 側でも差分ガードするため二重に安全。sleep で待つため `Drop` の `join()` は最大
//! 約 1 周期で復帰し、ハングしない。
//!
//! # 生存期間（RAII）
//! `VsyncEventBridge` を構造テンプレとし、`Arc<AtomicI64>` 最新座標＋`Arc<AtomicBool>`
//! stop_flag＋`Option<JoinHandle>` を保持する（`Arc<event_listener::Event>` はワーカへ渡すだけ）。`Drop` で
//! stop フラグを立ててワーカを `join()` する（`stop_flag.store(true)→join` の順序）。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread::{self, JoinHandle};

use tracing::{debug, error, trace};
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
/// - 共有 `Event`（ワーカスレッドへ move するだけで持たない・UI ループと同一 `Arc`）
/// - 最新カーソル座標（`Arc<AtomicI64>`・pack 済み・UI 側が [`latest_cursor`] で読む）
/// - 停止フラグ（`Arc<AtomicBool>`）
/// - スレッドハンドル（`Option<JoinHandle<()>>`・take は `Drop`/`stop` のみ）
///
/// [`latest_cursor`]: CursorMonitorBridge::latest_cursor
pub(crate) struct CursorMonitorBridge {
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
    /// `cursor_event` は UI ループ（[`super::start_click_through`] が回す判定ループの listen 対象）と共有する
    /// 同一 `Arc`。
    pub(crate) fn spawn(cursor_event: Arc<event_listener::Event>) -> Self {
        let latest_pos = Arc::new(AtomicI64::new(0));
        let stop_flag = Arc::new(AtomicBool::new(false));

        let thread_latest = Arc::clone(&latest_pos);
        let thread_stop = Arc::clone(&stop_flag);

        let handle = thread::Builder::new()
            .name("wintf-cursor-monitor".to_owned())
            .spawn(move || cursor_loop(cursor_event, thread_latest, thread_stop))
            .expect("failed to spawn wintf-cursor-monitor thread");

        debug!("CursorMonitorBridge started (wintf-cursor-monitor thread spawned)");

        Self {
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
    // 走り始めに 1 度だけ、自分の役割を宣言してスレッド名簿へ載せる。
    // 失敗しても監視は続ける——このスレッドの CPU が `unregistered_rest` へ回るだけで、
    // 観測の道具の不調が観測対象を止める理由にはならない。
    if let Err(e) = crate::ecs::world::thread_registry::register_current_thread(
        crate::ecs::world::thread_registry::ROLE_CURSOR_MONITOR,
    ) {
        error!(error = %e, "[cursor_loop] スレッド名簿への登録に失敗した（カーソル監視は継続）");
    }

    // 初回は必ず notify するよう `None` から始める。
    let mut prev: Option<(i32, i32)> = None;

    while !stop.load(Ordering::Acquire) {
        poll_once(read_cursor, &mut prev, &latest, || {
            event.notify(usize::MAX);
        });

        thread::sleep(POLL_INTERVAL);
    }
}

/// 本物のカーソル位置（screen physical）を 1 回読む。失敗したら `None`。
fn read_cursor() -> Option<(i32, i32)> {
    let mut pt = POINT::default();

    // SAFETY: Win32 境界。`pt` は本スレッドのスタック変数で、`GetCursorPos` の書き込み先として
    // 有効な `POINT` を指す。共有状態には触れない。
    match unsafe { GetCursorPos(&mut pt) } {
        Ok(()) => Some((pt.x, pt.y)),
        Err(_) => {
            // 稀な失敗。当該反復の notify を飛ばして継続する。
            trace!("GetCursorPos failed; skipping this iteration");
            None
        }
    }
}

/// カーソルを 1 回読み、前と違うときだけ最新の位置へ書いてから `notify` を呼ぶ。知らせたら `true`。
/// `read` が `None`（`GetCursorPos` の失敗）なら何もしない。
fn poll_once(
    read: impl FnOnce() -> Option<(i32, i32)>,
    prev: &mut Option<(i32, i32)>,
    latest: &AtomicI64,
    notify: impl FnOnce(),
) -> bool {
    let Some(cur) = read() else {
        return false;
    };
    if *prev == Some(cur) {
        return false;
    }
    // 順序不変条件: store（Release）→ notify。逆順だと UI 側が 1 通知分古い座標を読む。
    latest.store(pack(cur.0, cur.1), Ordering::Release);
    notify();
    *prev = Some(cur);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::world::thread_registry::{self, ROLE_CURSOR_MONITOR, ThreadEntry};
    use event_listener::Event;
    use std::time::Duration;

    /// 名簿に当該役割名の項目が現れるまで待ち、現れた項目を返す（現れなければ `None`）。
    ///
    /// 待ち時間は合否の対象ではなく、登録が起きなかったときに無限に待たないための上限で
    /// ある（実時間の閾値による判定ではない）。
    fn wait_for_role(role: &str, limit: Duration) -> Option<ThreadEntry> {
        let deadline = std::time::Instant::now() + limit;
        loop {
            let found = thread_registry::snapshot()
                .into_iter()
                .find(|entry| entry.role == role);
            if found.is_some() {
                return found;
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// カーソル監視ワーカは走り始めに自分をスレッド名簿へ登録する
    /// （役割名 `cursor_monitor`）。
    ///
    /// 名簿はプロセス共有で、テストは並列に走る——**全体の件数は当てにせず**、登録された
    /// 役割名の項目だけを検査する。観測はワーカを生かしたまま行う（終了した TID が別の
    /// スレッドへ再利用されて項目が置き換わる余地を残さないため）。
    #[test]
    fn cursor_monitor_thread_registers_itself_with_the_cursor_monitor_role() {
        let bridge = CursorMonitorBridge::spawn(Arc::new(Event::new()));

        let found = wait_for_role(ROLE_CURSOR_MONITOR, Duration::from_secs(5))
            .expect("wintf-cursor-monitor スレッドは走り始めに名簿へ載るはず");
        assert_eq!(
            found.name.as_deref(),
            Some("wintf-cursor-monitor"),
            "名簿の OS 名は生成時に付けたスレッド名であるはず"
        );
        assert!(
            thread_registry::is_known_role(ROLE_CURSOR_MONITOR),
            "登録した役割名は固定語彙に含まれるはず"
        );

        drop(bridge);
    }

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

    /// `spawn`→`drop` でワーカが確実に stop/join され、ハングしないこと。
    #[test]
    fn spawn_then_joins_on_drop() {
        let bridge = CursorMonitorBridge::spawn(Arc::new(Event::new()));
        drop(bridge);
    }

    /// 偽のカーソルが同じ位置を 2 回返すと、知らせは 1 回だけ（差分の守り）。
    #[test]
    fn poll_once_notifies_once_for_the_same_position_twice() {
        let latest = AtomicI64::new(0);
        let mut prev = None;
        let mut notified = 0;

        let first = poll_once(|| Some((10, 20)), &mut prev, &latest, || notified += 1);
        let second = poll_once(|| Some((10, 20)), &mut prev, &latest, || notified += 1);

        assert!(first, "初めの位置は知らせるはず");
        assert!(!second, "同じ位置の 2 回目は知らせないはず");
        assert_eq!(notified, 1, "同じ位置を 2 回読んでも知らせは 1 回のはず");
    }

    /// 知らせの中で最新の位置を読むと、もう新しい位置が入っている（書いてから知らせる順）。
    #[test]
    fn poll_once_stores_the_position_before_notifying() {
        let latest = AtomicI64::new(pack(1, 2));
        let mut prev = Some((1, 2));
        let mut seen = None;

        poll_once(
            || Some((-30, 40)),
            &mut prev,
            &latest,
            || {
                seen = Some(unpack(latest.load(Ordering::Acquire)));
            },
        );

        assert_eq!(
            seen,
            Some((-30, 40)),
            "知らせの中では新しい位置が読めるはず"
        );
    }
}
