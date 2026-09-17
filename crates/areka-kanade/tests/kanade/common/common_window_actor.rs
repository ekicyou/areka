//! 窓を作るテストの共有ヘルパ（窓を作るテストはこの 1 か所から組み立てる）。
//!
//! - [`WINDOW_CREATE_SERIAL`]: 親 message-only 窓の**生成の呼び出しだけ**を直列化するロック。
//!   wintf-winmsg-executor 0.0.5 は同じ瞬間に 2 つの窓を生成すると 2 つ目が
//!   `WindowCreationError` になる。生成後の共存は問題ないため、ロックは生成の一瞬だけを覆う
//!   （`shiori-host32-host` の `lifecycle.rs` にある `WINDOW_TEST_SERIAL` と同じ考え方）。
//! - [`spawn_window_actor`]: 本番の [`spawn_shiori_actor`] を、本番と同じ順序（窓は connect が
//!   アクタースレッド上で作る）で起こす。backend は本番の [`ShioriConnection`] そのもので、
//!   helper は即終了する x64 の stand-in（`cmd.exe /c exit 0`）を公開 API で起こしたもの。
//!   fake backend は置かない（`ShioriConnection` の委譲まで本番経路を踏ませるため）。

use std::process::Command;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use areka_actor::ActorHandle;
use areka_kanade::{KanadeMsg, ShioriBackend, ShioriConnection, ShioriMsg, spawn_shiori_actor};
use shiori_host32_host::process_host::spawn_command;
use shiori_host32_host::{Charset, CharsetNegotiator, HelperLifecycle, ParentMessageWindow};

/// 親 message-only 窓の生成を直列化する共有ロック（生成の呼び出しだけを囲む）。
/// poison は無視する（目的は相互排他だけ）。
pub static WINDOW_CREATE_SERIAL: Mutex<()> = Mutex::new(());

/// connect が HWND を返すまでの待ちの上限。
const HWND_TIMEOUT: Duration = Duration::from_secs(5);

/// [`spawn_window_actor`] が起こした本番の shiori アクター一式。
pub struct WindowActor {
    /// アクターの受信端へ送る送信端（`ShioriMsg::Close` で停止させる）。
    pub shiori_tx: Sender<ShioriMsg>,
    /// アクタースレッドのハンドル（`join_bounded` で有界に待つ）。
    pub handle: ActorHandle,
    /// アクタースレッド上で生成された親 message-only 窓の HWND（`hwnd_u32()`）。
    pub hwnd: u32,
    /// 死活報告（`KanadeMsg::ShioriDown`）の受信端。
    pub down_rx: Receiver<KanadeMsg>,
}

/// 本番の shiori アクターを、実 [`ShioriConnection`]＋x64 stand-in helper で起こす。
///
/// connect はアクタースレッド上で（[`WINDOW_CREATE_SERIAL`] の内側で）親窓を生成し、その HWND を
/// 呼び手へ返してから `ShioriConnection` を backend として返す。HWND が上限内に届かなければ
/// 原因（届いていれば `ShioriDown` の理由）を添えて panic する。
pub fn spawn_window_actor() -> WindowActor {
    let (hwnd_tx, hwnd_rx) = channel::<u32>();
    let (down_tx, down_rx) = channel::<KanadeMsg>();

    let connect = move || -> Result<Box<dyn ShioriBackend>, String> {
        let window = {
            let _guard = WINDOW_CREATE_SERIAL
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            ParentMessageWindow::create()
                .map_err(|e| format!("親 message-only 窓の生成に失敗: {e}"))?
        };
        let mut stand_in = Command::new("cmd.exe");
        stand_in.args(["/c", "exit", "0"]);
        let handle = spawn_command(stand_in)
            .map_err(|e| format!("x64 stand-in helper の起動に失敗: {e}"))?;
        let _ = hwnd_tx.send(window.hwnd_u32());
        Ok(Box::new(ShioriConnection {
            window,
            helper: HelperLifecycle::new(handle),
            // テスト用の既定値（UTF-8・強制なし。stand-in は通信しないので値は結果に影響しない）。
            negotiator: CharsetNegotiator::new(Charset::UTF_8, false),
        }))
    };

    let (shiori_tx, handle) = spawn_shiori_actor(connect, down_tx);
    let hwnd = match hwnd_rx.recv_timeout(HWND_TIMEOUT) {
        Ok(hwnd) => hwnd,
        Err(e) => {
            let down = match down_rx.try_recv() {
                Ok(KanadeMsg::ShioriDown { reason }) => format!("ShioriDown: {reason}"),
                Ok(_) => "ShioriDown 以外の通知".to_string(),
                Err(_) => "なし".to_string(),
            };
            panic!("connect から親窓の HWND が届かなかった（{e:?}）。死活報告: {down}")
        }
    };
    WindowActor {
        shiori_tx,
        handle,
        hwnd,
        down_rx,
    }
}
