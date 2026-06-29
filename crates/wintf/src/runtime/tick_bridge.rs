//! VSync 起床ブリッジ。
//!
//! 専用 VSync スレッドが `DwmFlush()` で vblank を検出し、共有
//! `event_listener::Event` を `notify(usize::MAX)`（全リスナ起床）で notify する。
//! UI スレッド側の待機タスク（後続タスク 2.3 の `AsyncTickTask`）は
//! `bridge.event().listen().await` で 1 フレームごとに起床できる。
//!
//! # 周期
//! `DwmFlush()` は次の vblank までブロックするため、ループは固定 16.67ms の
//! sleep を持たずモニターのリフレッシュレートに自然追従する（実 vblank 同期・
//! 要件 4.4）。旧 `PostMessageW(WM_VSYNC)` 経路は本ブリッジでは持たない（要件 4.1）。
//!
//! # 生存期間（RAII）
//! 本ユニットは自己完結の RAII で、スレッドの生存期間を `stop_flag → join` の
//! 順序規律で管理する（旧 `WinThreadMgrInner::drop` の規律を継承）。`Drop` で
//! stop フラグを立て、スレッドを `join()` する。
//!
//! 最終的にはこのインスタンスを `WinApp` が所有する（設計: 「`Event` は `WinApp`
//! が所有」）。`WinApp::run` の全結線（後続タスク 4.3）でフィールドとして保持され、
//! `WinApp` の drop に生存期間が委譲される。本タスクの時点では結線は行わず、
//! ブリッジ自身が自前のスレッドを所有し Drop で join する。

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};

use tracing::{debug, trace};
use windows::Win32::Graphics::Dwm::DwmFlush;

/// VSync 専用スレッドの vblank 検出を共有 `event_listener::Event` へ橋渡しする
/// RAII ユニット。
///
/// - 共有 `Event`（`Arc` で clone を VSync スレッドへ move）
/// - 停止フラグ（`Arc<AtomicBool>`）
/// - スレッドハンドル（`Option<JoinHandle<()>>`・take は `Drop`/`stop` のみ）
///
/// リスナは [`VsyncEventBridge::event`] で `Arc<Event>` を取得し `listen()` できる。
pub(crate) struct VsyncEventBridge {
    /// 共有 vblank 通知イベント。VSync スレッドが vblank ごとに `notify(usize::MAX)`。
    vblank_event: Arc<event_listener::Event>,
    /// VSync スレッド停止フラグ。`Drop`/`stop` で `true` をストアしループを終える。
    stop_flag: Arc<AtomicBool>,
    /// VSync スレッドのハンドル。`new` で必ず `Some`、take は `stop`/`Drop` のみ。
    handle: Option<JoinHandle<()>>,
}

impl VsyncEventBridge {
    /// VSync スレッドを起動してブリッジを生成する。
    ///
    /// スレッドは `while !stop { DwmFlush(); event.notify(usize::MAX); }` をループし、
    /// vblank ごとに全リスナを起床する。
    pub(crate) fn new() -> Self {
        let vblank_event = Arc::new(event_listener::Event::new());
        let stop_flag = Arc::new(AtomicBool::new(false));

        // clone を VSync スレッドへ move（Event/stop_flag とも Arc で共有）。
        let thread_event = Arc::clone(&vblank_event);
        let thread_stop = Arc::clone(&stop_flag);

        let handle = thread::Builder::new()
            .name("wintf-vsync".to_owned())
            .spawn(move || vsync_loop(thread_event, thread_stop))
            .expect("failed to spawn wintf-vsync thread");

        debug!("VsyncEventBridge started (wintf-vsync thread spawned)");

        Self {
            vblank_event,
            stop_flag,
            handle: Some(handle),
        }
    }

    /// 共有 vblank 通知イベントへの参照を返す。
    ///
    /// 呼び出し側は `bridge.event().listen()` で `EventListener` を arm できる
    /// （`AsyncTickTask`・テスト用）。返す `Arc` を clone して保持してもよい。
    pub(crate) fn event(&self) -> &Arc<event_listener::Event> {
        &self.vblank_event
    }

    /// VSync スレッドを停止し join する（`stop_flag → join` の順序）。
    ///
    /// 冪等。`Drop` からも呼ばれる。
    ///
    /// # Edge
    /// stop フラグを立てた時点でスレッドが `DwmFlush()` 内で次の vblank まで
    /// park していることがあるため、`join()` は最大 1 フレーム（~リフレッシュ
    /// 周期）待つ。これは許容範囲。
    fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop_flag.store(true, Ordering::Release);
            if handle.join().is_err() {
                trace!("wintf-vsync thread join returned Err (thread panicked)");
            }
            debug!("VsyncEventBridge stopped (wintf-vsync thread joined)");
        }
    }
}

impl Drop for VsyncEventBridge {
    fn drop(&mut self) {
        self.stop();
    }
}

/// VSync スレッド本体。`DwmFlush()` で vblank を待ち、毎回 `Event` を全リスナ起床
/// で notify する。`stop` が立つまでループする。
fn vsync_loop(event: Arc<event_listener::Event>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Acquire) {
        // SAFETY: Win32 境界。`DwmFlush` は引数を取らず、DWM 合成の次フレーム
        // （vblank）まで現スレッドをブロックして `windows::core::Result<()>` を返す。
        // 副作用は本スレッドのブロックのみで、共有状態には触れない。
        let flushed = unsafe { DwmFlush() };

        match flushed {
            Ok(()) => {
                // vblank 到来 — 全リスナを起床（複数 tick タスク対応・
                // データは搬送しない起床通知のみ）。
                event.notify(usize::MAX);
            }
            Err(_) => {
                // DWM 無効など稀な失敗。スピンホットを避けるためおおよそ 60Hz
                // 相当だけ待ってからリトライする（本環境では通常到達しない）。
                thread::sleep(std::time::Duration::from_millis(15));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_listener::Listener;
    use std::time::Duration;

    /// 完了状態の検証: vblank ごとに Event が notify され、待機リスナを起床できる。
    /// さらに drop でスレッドが clean に join することを確認する（ハングしない）。
    #[test]
    fn vblank_notifies_listener_then_joins_on_drop() {
        let bridge = VsyncEventBridge::new();

        // notify/listen レースを避けるため、待機前に listen() を arm する。
        let listener = bridge.event().listen();

        // 500ms 以内に vblank notify が届けば Some（通常 Win10/11 で DWM 常時 ON）。
        let got = listener.wait_timeout(Duration::from_millis(500));
        assert!(
            got.is_some(),
            "expected a vblank notify within 500ms (is DWM running?)"
        );

        // drop で stop→join。スレッドが clean に終われば本テストはハングせず復帰する。
        drop(bridge);
    }
}
