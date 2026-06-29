//! 新 runtime レイヤ。UI スレッド基盤の owner となる公開 facade `WinApp` を提供する。
//!
//! `WinApp` は旧 `WinThreadMgr` を置換する新公開 facade であり、プロセス初期化
//! （`CoInitializeEx(COINIT_MULTITHREADED)`・DPI awareness 設定）と `EcsWorld`
//! （`Rc<RefCell<EcsWorld>>`）生成を統括する。クラス登録はライブラリ
//! （`wintf-winmsg-executor`）が担うため、本層では行わない。
//!
//! 依存方向は COM→ECS→Runtime。runtime は ecs に依存してよいが、ecs は runtime に
//! 依存しない。

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use tracing::debug;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::HiDpi::*;
use windows::core::Result;

use crate::ecs::world::EcsWorld;

/// メッセージループ層。ライブラリ（`wintf-winmsg-executor`）の `block_on` /
/// `MessageLoop::run` へ委譲する `MessageLoopDriver` を提供する。
mod message_loop;

/// VSync 起床ブリッジ層。専用スレッドの `DwmFlush` vblank 検出を共有
/// `event_listener::Event` で UI スレッドへ通知する `VsyncEventBridge` を提供する。
mod tick_bridge;

/// ウィンドウ手続きブリッジ層。ライブラリの wndproc クロージャから `dispatch_window_message`
/// 純関数へ Entity 配送を橋渡しする `WndState`/`make_wndproc` を提供する。
mod wndproc_bridge;

/// UI スレッド基盤の owner。旧 `WinThreadMgr` を置換する新公開 facade。
///
/// COM 初期化・DPI awareness 設定・`EcsWorld` 生成を統括し、共有 World ハンドルの
/// 唯一の strong 所有者となる。VSync スレッド・message_window・メッセージループ駆動の
/// 結線は後続タスクで追加する。
pub struct WinApp {
    /// 共有 ECS world。`WinApp` が唯一の strong 所有者（`world()` は strong clone を返す）。
    world: Rc<RefCell<EcsWorld>>,
}

impl WinApp {
    /// COM/DPI 初期化・World 生成を行う。
    ///
    /// COM は `COINIT_MULTITHREADED` で初期化する。既に別モデルで初期化済みの場合
    /// （`S_FALSE` / `RPC_E_CHANGED_MODE`）はレガシー（`WinThreadMgrInner::new`）同様に
    /// 成功扱いとする。DPI awareness は per-monitor v2 を設定する。
    ///
    /// NOTE(W1-V): CoInitializeEx の成功に対し Drop で CoUninitialize を呼ばない現行方針
    /// （P30）を維持する。プロセス常駐の単一インスタンス運用では実害なし。
    pub fn new() -> Result<Self> {
        // SAFETY: Win32 境界。CoInitializeEx はプロセス/スレッドの COM ランタイムを
        // 初期化するのみで、引数 None・COINIT_MULTITHREADED は定数。S_FALSE
        // （同一スレッドで初期化済み）・RPC_E_CHANGED_MODE（別アパートメントモデルで
        // 初期化済み）は成功とみなす（レガシー踏襲）。
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_err() && hr != RPC_E_CHANGED_MODE {
                return Err(hr.into());
            }
        }

        // SAFETY: Win32 境界。SetProcessDpiAwarenessContext はプロセスグローバルな
        // DPI awareness をスレッドセーフに設定する。既に設定済み／不可の場合はエラーを
        // 返すが、レガシー同様に無視する（致命的ではない）。
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }

        let world = Rc::new(RefCell::new(EcsWorld::new()));
        debug!("WinApp initialized (COM/DPI ready, world created)");

        Ok(Self { world })
    }

    /// 共有 ECS world ハンドルを返す（旧 `WinThreadMgr::world` 相当）。
    ///
    /// 返す `Rc` は単一 World（複数ウィンドウで共有）への strong clone。
    pub fn world(&self) -> Rc<RefCell<EcsWorld>> {
        Rc::clone(&self.world)
    }

    /// UI スレッドのメッセージループを開始する（旧 `WinThreadMgr::run` 相当）。
    ///
    /// 最小スタブ。`AsyncTickTask` の spawn・`ShutdownPolicy` future の `block_on` 等の
    /// 完全な結線は後続タスク（run の全結線）で実装する。現状は即時復帰する。
    pub fn run(&self) -> Result<()> {
        Ok(())
    }

    /// UI スレッド単一の async タスクを投入する（旧 `spawn_normal` 相当・`!Send` 可）。
    ///
    /// ライブラリの `spawn_local` へ委譲する。実行にはメッセージループ（`run`/`block_on`）が
    /// 必要だが、投入自体はループ非実行下でも可能。
    pub fn spawn_ui_local<T: 'static>(
        &self,
        fut: impl Future<Output = T> + 'static,
    ) -> wintf_winmsg_executor::JoinHandle<T> {
        wintf_winmsg_executor::spawn_local(fut)
    }
}
