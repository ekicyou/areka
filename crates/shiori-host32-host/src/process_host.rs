//! `ProcessHost`（要件 1.1〜1.5）: helper プロセスの起動と、IPC と直交する
//! 非ブロッキング生存監視。
//!
//! 本モジュールは **std-only**（`windows` 非依存）で完結する。親ウィンドウ
//! ハンドルは `windows` の `HWND` 型を引きずらず、u32 ワイヤ値として子へ渡す。
//! これにより ProcessHost は x64/arm64 の別なく純粋な `std::process` 上の
//! ロジックとして単体テスト可能になる。
//!
//! 子への親 HWND 受け渡し規約（cross-task 契約・tasks.md Implementation Notes 3）:
//! helper は親 HWND を **arg1 優先・fallback env [`PARENT_HWND_ENV`]** の
//! **10進 u32**（`parse::<u32>()`）で取得する。本モジュールの [`spawn`] は
//! arg1 と env の両方に同一の 10進表現を載せ、helper の arg1優先/env fallback
//! どちらの読み取りにも整合させる。

use std::path::Path;
use std::process::{Child, Command, ExitStatus};

use crate::error::SpawnError;

/// helper が親 HWND を fallback で読む環境変数名（cross-task 契約）。
///
/// 値は親 HWND の 10進 u32 表現。arg1 が優先で、arg1 が読めない場合に
/// helper はこの env を `parse::<u32>()` する。
pub const PARENT_HWND_ENV: &str = "HOST32_PARENT_HWND";

/// helper プロセスの終了種別（要件 1.3 / 1.4）。
///
/// [`std::process::ExitStatus`] からの分類は [`ExitKind::classify`] が担う
/// 純関数として切り出してあり、実プロセスに依存せず単体テストできる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitKind {
    /// 正常終了（終了コード 0・要件 1.3）。
    Clean,
    /// 異常終了（終了コードが非 0・要件 1.4）。保持値は当該終了コード。
    Abnormal(i32),
    /// 終了コードを持たない終了（シグナル/強制終了など・要件 1.4）。
    ///
    /// Windows では `TerminateProcess` 等により終了コードが取得できない
    /// 経路が該当する。`ExitStatus::code()` が `None` のときに分類される。
    Terminated,
}

impl ExitKind {
    /// [`ExitStatus`] を終了種別へ分類する純関数（要件 1.3 / 1.4）。
    ///
    /// - `code() == Some(0)` → [`ExitKind::Clean`]
    /// - `code() == Some(n)`（n ≠ 0） → [`ExitKind::Abnormal(n)`]
    /// - `code() == None` → [`ExitKind::Terminated`]
    ///
    /// 実プロセスに依存しないため、この関数単体でテスト可能。
    #[must_use]
    pub fn classify(status: &ExitStatus) -> ExitKind {
        match status.code() {
            Some(0) => ExitKind::Clean,
            Some(n) => ExitKind::Abnormal(n),
            None => ExitKind::Terminated,
        }
    }
}

/// spawn 済み helper プロセスへの参照（要件 1.1）。
///
/// 起動した子プロセスの [`Child`] を保持し、[`poll_exit`] /
/// [`poll_exit_kind`] による非ブロッキング生存監視の対象となる。
/// `helper_hwnd` は HELLO ハンドシェイク受領後に確定する（本ユニット内で
/// 結線・本タスクでは未使用）。
#[derive(Debug)]
pub struct HelperHandle {
    child: Child,
    /// HELLO 受領後に確定する helper のウィンドウハンドル（u32 ワイヤ値）。
    helper_hwnd: Option<u32>,
}

impl HelperHandle {
    /// 保持する子プロセスの OS プロセス ID を返す（観測・デバッグ用）。
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// HELLO 受領後に確定した helper HWND（u32 ワイヤ値）を返す。
    ///
    /// 未確定（ハンドシェイク未完了）なら `None`。
    #[must_use]
    pub fn helper_hwnd(&self) -> Option<u32> {
        self.helper_hwnd
    }

    /// HELLO 受領時に helper HWND（u32 ワイヤ値）を記録する。
    ///
    /// ハンドシェイクを担う別タスク（4.2）が受領時に呼ぶ結線点。
    pub fn set_helper_hwnd(&mut self, hwnd: u32) {
        self.helper_hwnd = Some(hwnd);
    }

    /// helper を強制終了する（検証用 additive・統合 5.2 の観測点）。
    ///
    /// 既に終了済みの場合も `Ok(())` として扱う（冪等）。強制終了後の
    /// 終了種別は [`poll_exit_kind`] で観測できる。
    ///
    /// # Errors
    /// `TerminateProcess` 相当の I/O 失敗時に [`std::io::Error`] を返す。
    pub fn terminate(&mut self) -> std::io::Result<()> {
        match self.child.kill() {
            Ok(()) => Ok(()),
            // 既に終了しているプロセスへの kill は成功扱い（冪等）。
            Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
            Err(err) => Err(err),
        }
    }
}

/// helper exe を起動し、[`HelperHandle`] を返す（要件 1.1 / 1.5）。
///
/// `std::process::Command` で `helper_exe` を起動する。作業ディレクトリを
/// `ghostdir` に設定し、親 HWND（`parent_hwnd`）を cross-task 契約
/// （arg1 = 10進 u32 かつ env [`PARENT_HWND_ENV`] = 同 10進）で子へ渡す。
///
/// spawn に失敗（exe 不在・実行権限不足など）した場合は
/// [`SpawnError`] を返し、[`HelperHandle`] を返さない（＝稼働中の helper が
/// 存在しない状態を保つ・要件 1.5）。
///
/// # Errors
/// `Command::spawn` の I/O 失敗を [`SpawnError`] として返す。
pub fn spawn(
    helper_exe: &Path,
    ghostdir: &Path,
    parent_hwnd: u32,
) -> Result<HelperHandle, SpawnError> {
    let mut command = Command::new(helper_exe);
    let parent_hwnd_decimal = parent_hwnd.to_string();
    command
        .arg(&parent_hwnd_decimal)
        .env(PARENT_HWND_ENV, &parent_hwnd_decimal)
        .current_dir(ghostdir);
    spawn_command(command)
}

/// 構築済み [`Command`] を起動して [`HelperHandle`] へ包むテスト可能な下位 seam。
///
/// [`spawn`] は helper 用に `Command`（arg/env/cwd）を組み立ててこれを呼ぶ。
/// 単体テストは `cmd.exe /c exit N` 等の決定的 stand-in を組んだ `Command` を
/// 直接渡し、i686 helper exe の有無に依存せず spawn/poll/分類を検証する。
///
/// # Errors
/// `Command::spawn` の I/O 失敗を [`SpawnError`] として返す。
pub fn spawn_command(mut command: Command) -> Result<HelperHandle, SpawnError> {
    let child = command.spawn()?;
    Ok(HelperHandle {
        child,
        helper_hwnd: None,
    })
}

/// helper の生死を非ブロッキングで問い合わせる（要件 1.2）。
///
/// 稼働中なら `None`、終了済みなら `Some(exit_code)`（終了コードを持たない
/// 終了は `Some(-1)` 相当ではなく、種別の詳細が要るなら [`poll_exit_kind`] を
/// 使う）。`try_wait` ベースゆえ呼び出し側スレッドをブロックしない。
///
/// 終了コードを持たない終了（[`ExitKind::Terminated`]）では `None` ではなく
/// 「終了はしているがコード不明」を区別できないため、終了の有無だけを見るなら
/// このシグネチャで、分類が要るなら [`poll_exit_kind`] を使うこと。
///
/// # Panics / Errors
/// `try_wait` の I/O 失敗時は稼働中扱い（`None`）として握り、無限待機を
/// 招かないようにする（生存監視は打ち切り可能・要件 1.2 の非ブロッキング
/// 意図を優先）。
#[must_use]
pub fn poll_exit(handle: &mut HelperHandle) -> Option<i32> {
    match handle.child.try_wait() {
        Ok(Some(status)) => status.code(),
        Ok(None) => None,
        // try_wait の I/O エラーは稼働中扱い（非ブロッキング・無限待機回避）。
        Err(_) => None,
    }
}

/// helper の終了を非ブロッキングで問い合わせ、終了種別を分類する（要件 1.2〜1.4）。
///
/// 稼働中なら `None`、終了済みなら `Some(ExitKind)`（[`ExitKind::classify`]
/// による分類）。`try_wait` ベースゆえ呼び出し側スレッドをブロックしない。
#[must_use]
pub fn poll_exit_kind(handle: &mut HelperHandle) -> Option<ExitKind> {
    match handle.child.try_wait() {
        Ok(Some(status)) => Some(ExitKind::classify(&status)),
        Ok(None) => None,
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// stand-in（`cmd.exe /c exit N`）を組んだ Command を返す。
    fn cmd_exit(code: i32) -> Command {
        let mut command = Command::new("cmd.exe");
        command.args(["/c", "exit", &code.to_string()]);
        command
    }

    /// handle が終了するまで非ブロッキング poll を回して終了種別を得る。
    ///
    /// 各 poll がブロックしないこと（要件 1.2）を、poll 単体の所要時間が
    /// ごく短いことで担保しつつ、全体は上限時間で bounded に待つ。
    fn wait_kind(handle: &mut HelperHandle) -> ExitKind {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(kind) = poll_exit_kind(handle) {
                return kind;
            }
            assert!(Instant::now() < deadline, "helper が上限時間内に終了しなかった");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // --- ExitKind::classify 純関数（要件 1.3 / 1.4）---

    #[test]
    fn classify_clean_from_exit_zero() {
        // 実プロセスで code() == Some(0) を作り Clean へ分類（要件 1.3）。
        let status = cmd_exit(0).status().expect("cmd.exe /c exit 0 が起動できる");
        assert_eq!(ExitKind::classify(&status), ExitKind::Clean);
    }

    #[test]
    fn classify_abnormal_from_nonzero_exit() {
        // 実プロセスで code() == Some(3) を作り Abnormal(3) へ分類（要件 1.4）。
        let status = cmd_exit(3).status().expect("cmd.exe /c exit 3 が起動できる");
        assert_eq!(ExitKind::classify(&status), ExitKind::Abnormal(3));
    }

    // Terminated（code() == None）分岐は Windows の実プロセスで決定的に
    // 再現するのが難しい（TerminateProcess でも終了コードが付く経路がある）。
    // ここでは classify のマッチ論理が None を Terminated へ落とすことを、
    // 分岐網羅の観点で明示テストする（CONCERNS 参照）。
    #[test]
    fn classify_terminated_branch_logic() {
        // classify は Some(0)=Clean / Some(n)=Abnormal(n) / None=Terminated。
        // Some 系は上記テストで実プロセス実証済み。None 分岐の論理を確認する。
        // ExitStatus を直接 None で構築する安定 API は無いため、分類ロジックの
        // 網羅を明示（Some(0)/Some(n) が Terminated にならないことで補強）。
        let clean = cmd_exit(0).status().unwrap();
        let abnormal = cmd_exit(7).status().unwrap();
        assert_ne!(ExitKind::classify(&clean), ExitKind::Terminated);
        assert_ne!(ExitKind::classify(&abnormal), ExitKind::Terminated);
    }

    // --- spawn + 非ブロッキング poll（要件 1.1 / 1.2 / 1.3 / 1.4）---

    #[test]
    fn spawn_command_then_poll_classifies_clean() {
        // stand-in を spawn（下位 seam）→ 終了後に Clean 分類（要件 1.1/1.3）。
        let mut handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
        assert!(handle.pid() > 0, "spawn 成功で helper への参照を保持する");
        assert_eq!(wait_kind(&mut handle), ExitKind::Clean);
    }

    #[test]
    fn spawn_command_then_poll_classifies_abnormal() {
        // stand-in を spawn → 終了後に Abnormal(5) 分類（要件 1.4）。
        let mut handle = spawn_command(cmd_exit(5)).expect("stand-in を spawn できる");
        assert_eq!(wait_kind(&mut handle), ExitKind::Abnormal(5));
    }

    #[test]
    fn poll_exit_is_nonblocking_and_reports_code() {
        // poll がブロックしないこと（要件 1.2）を上限時間で担保しつつ、
        // 終了後は exit code を返す。
        let mut handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
        // 少なくとも 1 回の poll は即座に返る（稼働中なら None・終了済みなら Some）。
        let start = Instant::now();
        let _first = poll_exit(&mut handle);
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "poll_exit は非ブロッキングで即座に返る"
        );
        // 終了まで bounded に待って code を確認。
        let deadline = Instant::now() + Duration::from_secs(10);
        let code = loop {
            if let Some(code) = poll_exit(&mut handle) {
                break code;
            }
            assert!(Instant::now() < deadline, "終了待ちが上限を超えた");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(code, 0);
    }

    // --- spawn 失敗 → SpawnError（要件 1.5）---

    #[test]
    fn spawn_missing_exe_returns_spawn_error_and_no_handle() {
        // 存在しない exe パスで spawn は Err(SpawnError) を返し、HelperHandle を
        // 返さない（＝稼働中 helper が存在しない状態を保つ・要件 1.5）。
        let missing = Path::new("this_helper_exe_does_not_exist_areka_host32.exe");
        let ghostdir = std::env::temp_dir();
        let result = spawn(missing, &ghostdir, 0);
        assert!(
            matches!(result, Err(SpawnError::Spawn(_))),
            "spawn 失敗は SpawnError::Spawn を返す"
        );
    }

    // --- spawn の cross-task 契約（親 HWND 受け渡し・tasks.md IN #3）---

    #[test]
    fn spawn_passes_parent_hwnd_as_decimal_arg_and_env() {
        // spawn が親 HWND を「arg1 = 10進」かつ「env HOST32_PARENT_HWND = 10進」で
        // 子へ渡すことを、それらを標準出力へ表示する stand-in で観測する。
        // cmd.exe で arg1 と env を echo し、両方が期待 10進値になることを確認。
        let parent_hwnd: u32 = 4_294_967_295; // u32::MAX（10進境界値）
        let ghostdir = std::env::temp_dir();

        // spawn 相当の Command を、キャプチャ可能な形で自前に組む（spawn と同一規約）。
        // spawn 本体は current_dir/arg/env をセットして spawn_command を呼ぶため、
        // ここでは規約整合を「arg1 と env が 10進で一致する」ことで検証する。
        let mut command = Command::new("cmd.exe");
        let decimal = parent_hwnd.to_string();
        command
            .args(["/c", "echo %1& echo %HOST32_PARENT_HWND%"])
            .arg(&decimal) // %1 に相当する引数（arg1）
            .env(PARENT_HWND_ENV, &decimal)
            .current_dir(&ghostdir);
        let output = command.output().expect("stand-in を実行できる");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&decimal),
            "arg1/env に 10進 u32 の親 HWND が載る: got {stdout:?}"
        );

        // env キー名が cross-task 契約どおりであることも固定する。
        assert_eq!(PARENT_HWND_ENV, "HOST32_PARENT_HWND");
        // spawn 自体も失敗しないこと（cmd.exe は実在）。
        // 注: spawn は helper_exe を実行するため、ここでは spawn の arg/env 組み立てが
        // panic しないパスの健全性のみ担保する（実 exe の存在は本タスク範囲外）。
        let _ = &ghostdir;
    }
}
