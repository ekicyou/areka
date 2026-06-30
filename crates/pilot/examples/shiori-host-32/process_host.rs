//! host-32 先進坑: ProcessHost コンポーネント（x64 親に閉じる・design.md §285–327）。
//!
//! i686 helper プロセスを `std::process::Command` で起動し、その生死/終了コードを
//! **IPC レイヤと直交**に監視する（要件 1.1/1.2/1.3/1.4/2.4）。本モジュールは親 x64
//! ターゲットからのみ `#[path = "process_host.rs"] mod process_host;` で取り込み、
//! helper（i686）からは参照しない（プロセス分離・要件 1.5）。
//!
//! 跨ビットネス規約（design.md §339 / ipc.rs §3）に従い、親 HWND は **u32** ワイヤ値
//! として子へ受け渡す（arg/env）。これにより本モジュールは `windows` クレート不要の
//! std-only で完結する（HWND 型を引きずらない）。
//!
//! 本タスク（2.2）のスコープ:
//!   - `spawn(helper_exe, ghostdir, parent_hwnd)`: helper exe を起動し ghostdir/親 HWND を
//!     arg/env で渡す（§305–313 / §324）。
//!   - `poll_exit`: 非ブロッキング生存監視（`try_wait`・要件 1.2/2.4）。
//!   - `wait_clean`: clean shutdown 待ち＋終了コード取得（要件 1.3）。
//!   - 終了コードの clean/異常分類（要件 1.4）。
//! メッセージ窓・WM_COPYDATA 往復・HELLO ハンドシェイクは後続タスク（4.1）であり、
//! ここでは `helper_hwnd` は常に `None`（確定は task 4.1）。

#![allow(dead_code)] // 一部 API（wait_clean 等）は main.rs の 2.2 スライスからのみ使用。

use std::path::Path;
use std::process::{Child, Command};

/// helper exe パスを渡すための環境変数名（design.md §324 確定）。
/// 親はこの環境変数（無ければ第 1 CLI 引数）で helper exe パスを受ける。
pub const HELPER_EXE_ENV: &str = "HELPER_EXE";

/// 子へ親 HWND を渡す環境変数名（u32 LE ワイヤ値・ipc.rs §3 / design.md §339）。
pub const PARENT_HWND_ENV: &str = "PARENT_HWND";

/// 子へ ghostdir を渡す環境変数名（SHIORI `load` に渡す `fixtures/emo2/ghost/master/`）。
pub const GHOSTDIR_ENV: &str = "GHOSTDIR";

/// 起動済 helper プロセスのハンドル（design.md §306–309）。
///
/// `helper_hwnd` は HELLO（task 4.1）受領後に確定する。本タスク（2.2）では常に `None`。
pub struct HelperHandle {
    /// 子プロセスハンドル（終了コード/生死監視の単一ソース）。
    pub child: Child,
    /// helper のメッセージ窓 HWND（u32 ワイヤ値）。HELLO 受領後に確定（task 4.1）。
    pub helper_hwnd: Option<u32>,
}

/// helper の終了種別（要件 1.4: clean / 異常を観測可能な形に分類）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// 終了コード 0 = clean shutdown（要件 1.3）。
    Clean,
    /// 非ゼロ終了コード = 異常終了（要件 1.4）。
    Abnormal(i32),
    /// 終了コードを取得できない異常終了（シグナル/強制終了等）。
    Terminated,
}

impl ExitKind {
    /// 終了コード（`Option<i32>`: None = コード不明）を観測可能な分類へ写像する。
    /// `None`（コードなし＝シグナル等で終了）は `Terminated`（異常）として扱う。
    pub fn classify(code: Option<i32>) -> ExitKind {
        match code {
            Some(0) => ExitKind::Clean,
            Some(c) => ExitKind::Abnormal(c),
            None => ExitKind::Terminated,
        }
    }

    /// clean shutdown か（要件 1.3 の観測点）。
    pub fn is_clean(self) -> bool {
        matches!(self, ExitKind::Clean)
    }
}

/// ProcessHost: helper プロセスの起動と生存監視（design.md §285–327）。
pub struct ProcessHost;

impl ProcessHost {
    /// i686 helper exe を起動する（design.md §313 / §305）。
    ///
    /// `ghostdir`（SHIORI `load` 対象）と `parent_hwnd`（u32 ワイヤ値）を **arg と env の
    /// 両方**で渡す。helper スケルトンは現状これらを無視するが（後続タスクで消費）、
    /// 起動配線そのものが本タスクの観測対象である。
    ///
    /// arg 並び: `argv[1] = ghostdir`, `argv[2] = parent_hwnd(10 進)`。
    /// env: `GHOSTDIR` / `PARENT_HWND`。
    pub fn spawn(
        helper_exe: &Path,
        ghostdir: &Path,
        parent_hwnd: u32,
    ) -> std::io::Result<HelperHandle> {
        let child = Command::new(helper_exe)
            .arg(ghostdir)
            .arg(parent_hwnd.to_string())
            .env(GHOSTDIR_ENV, ghostdir)
            .env(PARENT_HWND_ENV, parent_hwnd.to_string())
            .spawn()?;
        Ok(HelperHandle {
            child,
            helper_hwnd: None,
        })
    }

    /// 子の生死を **非ブロッキング**で確認する（design.md §315・要件 1.2/2.4）。
    ///
    /// `Some(code)` で終了（終了コード確定）・`None` で稼働中。`try_wait` ゆえ IPC レイヤと
    /// 直交した生存監視として何度でも呼べる。終了コードが取得できない異常終了（シグナル等）は
    /// `Some(i32::MIN)` を返さず、本関数では「コードなし」を `None` と区別するため
    /// `poll_exit_kind` を別途用意する。
    pub fn poll_exit(handle: &mut HelperHandle) -> Option<i32> {
        match handle.child.try_wait() {
            // 終了済: ExitStatus からコードを取得（None=シグナル終了は便宜上 -1 とせず
            // poll_exit_kind 側で Terminated と分類）。ここではコードを返す契約ゆえ、
            // コード不明時は None を返さず最も近い観測値として -1 を返す。
            Ok(Some(status)) => Some(status.code().unwrap_or(-1)),
            // 稼働中。
            Ok(None) => None,
            // try_wait 自体の失敗は稼働継続不能だが、観測としては「不明」。-1 を返す。
            Err(_) => Some(-1),
        }
    }

    /// 非ブロッキング生存監視 ＋ 終了分類（要件 1.2/1.4/2.4）。
    /// `None` = 稼働中。`Some(ExitKind)` = 終了済（clean/異常を分類）。
    pub fn poll_exit_kind(handle: &mut HelperHandle) -> Option<ExitKind> {
        match handle.child.try_wait() {
            Ok(Some(status)) => Some(ExitKind::classify(status.code())),
            Ok(None) => None,
            Err(_) => Some(ExitKind::Terminated),
        }
    }

    /// clean shutdown を待つ（design.md §317・要件 1.3）。ブロックして終了コードを返す。
    /// 終了コードが取得できない異常終了（シグナル等）は `-1` を返す。
    pub fn wait_clean(mut handle: HelperHandle) -> std::io::Result<i32> {
        let status = handle.child.wait()?;
        Ok(status.code().unwrap_or(-1))
    }

    /// `wait_clean` の分類版: ブロックして `ExitKind`（clean/異常）を返す（要件 1.3/1.4）。
    pub fn wait_kind(mut handle: HelperHandle) -> std::io::Result<ExitKind> {
        let status = handle.child.wait()?;
        Ok(ExitKind::classify(status.code()))
    }
}

// ============================================================================
// 単体テスト（観測可能な受け入れ基準・task 2.2）
//
// 注意: 本テストは i686 helper exe の有無に依存しない自己完結テストである。
// ProcessHost の責務は「子プロセスの起動 → 終了コード/生死の監視」であり、どの exe を
// 起動するかは付随的。ゆえに決定的・可搬な stand-in プロセス（`cmd.exe /c exit N`）で
// その配線を検証する。REAL helper の起動は main.rs の 2.2 スライス（手動実行）で実証する。
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// テスト用 stand-in: `cmd.exe /c exit <code>` をハンドル化する。
    /// ProcessHost::spawn は helper exe を取るが、ここでは終了コード制御のため
    /// Command を直接組み、HelperHandle に包む（spawn と同じ Child 監視経路を通す）。
    fn spawn_exit(code: i32) -> HelperHandle {
        let child = Command::new("cmd.exe")
            .args(["/c", &format!("exit {code}")])
            .spawn()
            .expect("cmd.exe must spawn on Windows");
        HelperHandle {
            child,
            helper_hwnd: None,
        }
    }

    /// 要件 1.3: clean shutdown（終了コード 0）を `wait_clean` で取得し、clean と分類する。
    #[test]
    fn wait_clean_captures_zero_and_classifies_clean() {
        let handle = spawn_exit(0);
        let code = ProcessHost::wait_clean(handle).expect("wait must succeed");
        assert_eq!(code, 0, "clean exit must report code 0");
        assert_eq!(ExitKind::classify(Some(code)), ExitKind::Clean);
        assert!(ExitKind::classify(Some(code)).is_clean());
    }

    /// 要件 1.4: 非ゼロ終了コードを捕捉し、異常（Abnormal）と分類する。
    #[test]
    fn wait_captures_nonzero_and_classifies_abnormal() {
        let handle = spawn_exit(3);
        let kind = ProcessHost::wait_kind(handle).expect("wait must succeed");
        assert_eq!(kind, ExitKind::Abnormal(3), "non-zero exit must be Abnormal(3)");
        assert!(!kind.is_clean());
    }

    /// 要件 1.2/2.4: 非ブロッキング生存監視。稼働中は `None`、終了後に `Some(code)`。
    #[test]
    fn poll_exit_is_none_while_running_then_some_after_exit() {
        // 稼働中を観測するため、短時間スリープしてから 0 終了するプロセスを起動。
        // `ping -n 2 127.0.0.1` は約 1 秒待つ（cmd の timeout は対話端末で不安定なため ping を用いる）。
        let child = Command::new("cmd.exe")
            .args(["/c", "ping -n 2 127.0.0.1 >NUL & exit 0"])
            .spawn()
            .expect("cmd.exe must spawn");
        let mut handle = HelperHandle {
            child,
            helper_hwnd: None,
        };

        // 起動直後は稼働中（None）であることを観測（非ブロッキング）。
        assert_eq!(
            ProcessHost::poll_exit(&mut handle),
            None,
            "poll_exit must be None while the child is still running"
        );

        // 終了を待つ（ポーリングして Some に変わるまで・上限ガード付き）。
        let mut observed: Option<i32> = None;
        for _ in 0..200 {
            if let Some(code) = ProcessHost::poll_exit(&mut handle) {
                observed = Some(code);
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            observed,
            Some(0),
            "poll_exit must become Some(0) after the child exits cleanly"
        );

        // 分類版でも clean を観測（終了後は何度呼んでも Some）。
        // 既に try_wait で回収済みでも try_wait は終了ステータスを返し続ける。
        match ProcessHost::poll_exit_kind(&mut handle) {
            Some(kind) => assert_eq!(kind, ExitKind::Clean),
            None => panic!("poll_exit_kind must report Some after exit"),
        }
    }

    /// ExitKind 分類表（要件 1.3/1.4）: 0=Clean, 非0=Abnormal, None=Terminated。
    #[test]
    fn exit_kind_classification_table() {
        assert_eq!(ExitKind::classify(Some(0)), ExitKind::Clean);
        assert_eq!(ExitKind::classify(Some(1)), ExitKind::Abnormal(1));
        assert_eq!(ExitKind::classify(Some(255)), ExitKind::Abnormal(255));
        assert_eq!(ExitKind::classify(None), ExitKind::Terminated);
        assert!(ExitKind::classify(Some(0)).is_clean());
        assert!(!ExitKind::classify(Some(2)).is_clean());
        assert!(!ExitKind::classify(None).is_clean());
    }
}
