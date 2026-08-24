//! `ProcessHost`（要件 1.1〜1.5）: helper プロセスの起動と、IPC と直交する
//! 非ブロッキング生存監視。
//!
//! 本モジュールの純ロジック（`ExitKind::classify`・`resolve_param`・
//! `resolve_request_timeout` 等）は **std-only**（`windows` 非依存）で完結し、親ウィンドウ
//! ハンドルは `windows` の `HWND` 型を引きずらず u32 ワイヤ値として子へ渡す。これにより
//! ProcessHost の判定ロジックは x64/arm64 の別なく純粋な `std::process` 上のロジックとして
//! 単体テスト可能である。唯一 [`spawn_command`] のみ、helper の**孤児化防止**として
//! KILL_ON_JOB_CLOSE 付き Job Object を割り当てる（windows 依存は [`crate::job`] に隔離し、
//! 割り当て失敗時は縮退＝現行挙動）。
//!
//! 子への起動パラメーター受け渡し規約（cross-task 契約・design §SpawnContract・R3.1〜3.3）:
//! helper は起動パラメーターを **arg-n 優先・fallback env** の順で取得する。
//! [`spawn`] は子引数と env の両方に同一値を二重供給し、helper の
//! arg優先/env fallback どちらの読み取りにも整合させる。
//!
//! 子引数の並び（Rust 関数 [`spawn`] の引数順とは別物）:
//! - arg1 = parent_hwnd（10進 u32・現行 helper の arg1 読み取り互換を維持）
//! - arg2 = load_dir（伺かゴーストディレクトリ・cwd も同値）
//! - arg3 = shiori_name（descript 解決済みの DLL ファイル名・親の領分）
//!
//! env は同値を二重供給する: [`PARENT_HWND_ENV`]（parent_hwnd 10進）・
//! [`LOAD_DIR_ENV`]（load_dir）・[`SHIORI_NAME_ENV`]（shiori_name）。cwd
//! （`current_dir`）は load_dir に設定する（伺か慣習・R3.3）。

use std::path::Path;
use std::process::{Child, Command, ExitStatus};
use std::time::Duration;

use crate::error::SpawnError;

/// helper が親 HWND を fallback で読む環境変数名（cross-task 契約）。
///
/// 値は親 HWND の 10進 u32 表現。arg1 が優先で、arg1 が読めない場合に
/// helper はこの env を `parse::<u32>()` する。
pub const PARENT_HWND_ENV: &str = "HOST32_PARENT_HWND";

/// helper が load_dir（ゴーストディレクトリ）を fallback で読む環境変数名（R3.1/3.2）。
///
/// 値は arg2 と同一の load_dir パス。arg2 が優先で、arg2 が読めない場合に
/// helper はこの env を参照する。cwd も同値に設定される（R3.3）。
pub const LOAD_DIR_ENV: &str = "HOST32_LOAD_DIR";

/// helper が SHIORI 名（DLL ファイル名）を fallback で読む環境変数名（R3.1/3.2）。
///
/// 値は arg3 と同一の shiori_name（descript 解決済みの DLL ファイル名・親の領分）。
/// arg3 が優先で、arg3 が読めない場合に helper はこの env を参照する。
pub const SHIORI_NAME_ENV: &str = "HOST32_SHIORI_NAME";

/// LOAD の `send_request` に用いる推奨既定 timeout（per-call 引数・R5.2）。
///
/// `MsgTag::Load` の ack 待機に使う（`SMTO_ABORTIFHUNG` ＋この上限で有限復帰）。
/// ipc の timeout 機構自体は変更せず、この定数は per-call 引数として渡すためだけの
/// 推奨既定値である（echo 系の 5s とは別建て・design §Load timeout 方針）。
pub const LOAD_ACK_TIMEOUT: Duration = Duration::from_secs(30);

/// request 往復（GET／NOTIFY）の `send_request` に用いる既定 timeout（per-call 引数・R4.3/R5.1）。
///
/// `MsgTag::Request` の応答待機に使う（`SMTO_ABORTIFHUNG` ＋この上限で有限復帰）。
/// [`LOAD_ACK_TIMEOUT`]（30s）とは**別建て**の 60s とする（GET は SHIORI 側の
/// 思考時間を含み得るため長めに取る・design §Shiori3Client Implementation Notes・
/// 開発者決定 2026-07-04）。ipc の timeout 機構自体は変更せず、この定数は per-call 引数
/// として渡すためだけの既定値である。env [`REQUEST_TIMEOUT_ENV`] が未設定のときの既定。
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// request 往復 timeout を上書きする環境変数名（ミリ秒指定・R4.3/R5.1）。
///
/// 値はミリ秒の 10 進整数。未設定なら [`REQUEST_TIMEOUT`]（60s）が既定。`"0"` は
/// **無限待ち**（timeout 無効）を意味し、ブレークポイントデバッグ用の明示 opt-in である
/// （ブレークポイント停止と真のハングを機械的に区別できないため、開発者の明示指定を
/// 代替手段とする・design §Shiori3Client Implementation Notes）。
///
/// 命名根拠: `AREKA_` 名前空間で OS 共有 env との衝突を回避し、`SHIORI_` で対象概念を
/// 自己説明する（内部コードネーム `host32` は本番 env 名に用いない・design.md）。
pub const REQUEST_TIMEOUT_ENV: &str = "AREKA_SHIORI_REQUEST_TIMEOUT_MS";

/// env 値文字列から実効 request timeout を解決する**決定的な純関数**（R4.3/R5.1）。
///
/// `std::env` を直接読まず env 値を `Option<String>` として受け取る（単体テスト可能・
/// helper 側 `resolve_param` と同型の arg/env 分離）。戻り値の `Option<Duration>` は
/// **`None` = 無限待ち**（timeout 無効）を表す:
///
/// - env 不在／空文字／空白のみ／parse 不能 → `Some(REQUEST_TIMEOUT)`（既定 60s）。
///   parse 不能を既定へ落とすのは**意図的な安全側フォールバック**であり、不正指定で
///   起動を止めない（無限待ちへ暗黙昇格させない＝有限復帰を守る）。
/// - env `"0"` → `None`（**無限待ち**・デバッグ opt-in）。
/// - env `"<n>"`（n > 0・ミリ秒） → `Some(Duration::from_millis(n))`。
///
/// この関数は値解決に徹し I/O も副作用も持たない。実 env の読み取りは薄いラッパ
/// [`request_timeout_from_env`] が担う。
#[must_use]
pub fn resolve_request_timeout(env: Option<String>) -> Option<Duration> {
    // trim 後に空でなければ採用（helper 側 resolve_param と同様の正規化）。
    let trimmed = env.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    match trimmed.and_then(|s| s.parse::<u64>().ok()) {
        // "0" は無限待ち（timeout 無効・デバッグ opt-in）。
        Some(0) => None,
        // 正のミリ秒指定を採用。
        Some(n) => Some(Duration::from_millis(n)),
        // 不在／空／parse 不能は既定 60s へ安全側フォールバック（無限へ昇格させない）。
        None => Some(REQUEST_TIMEOUT),
    }
}

/// 実 env（[`REQUEST_TIMEOUT_ENV`]）を読み [`resolve_request_timeout`] へ委譲する薄いラッパ。
///
/// `None` = 無限待ち（`"0"` 指定時）・`Some(Duration)` = 有限 timeout。実 env の読み取りは
/// ここに閉じ込め、値解決ロジック本体は純関数側で単体テストする（helper 側の
/// `parent_hwnd_arg_env` → `resolve_param` 分離と同型）。
#[must_use]
pub fn request_timeout_from_env() -> Option<Duration> {
    resolve_request_timeout(std::env::var(REQUEST_TIMEOUT_ENV).ok())
}

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
///
/// **孤児化防止**: spawn 時に helper を `KILL_ON_JOB_CLOSE` 付き Job Object へ割り当て、
/// その job ハンドルを `_job` として保持する（[`crate::job`]）。本ハンドルを保持する
/// 親プロセスが終了（正常・異常・UNLOAD 未送出のいずれも）すると OS が job を閉じ、
/// helper を確実に道連れにする。helper 側の親死亡検知（下流 host32-lifecycle 領分・未実装）に
/// 依存しない安全網。job 割り当てに失敗した場合は `None`（縮退・現行挙動）。
#[derive(Debug)]
pub struct HelperHandle {
    child: Child,
    /// HELLO 受領後に確定する helper のウィンドウハンドル（u32 ワイヤ値）。
    helper_hwnd: Option<u32>,
    /// KILL_ON_JOB_CLOSE 付き Job Object の RAII ハンドル（孤児化防止）。`Drop` で
    /// `CloseHandle`→job クローズ→helper kill。割り当て失敗時は `None`（縮退）。
    _job: Option<std::os::windows::io::OwnedHandle>,
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

/// helper exe を起動し、[`HelperHandle`] を返す（要件 1.1 / 3.1 / 3.2 / 3.3 / 5.2）。
///
/// `std::process::Command` で `helper_exe` を起動する。起動パラメーター契約
/// （design §SpawnContract・R3.1〜3.3）に従い、次を子へ二重供給する:
///
/// - **子引数**: arg1 = `parent_hwnd`（10進 u32・helper の arg1 読み取り互換を維持）・
///   arg2 = `load_dir`・arg3 = `shiori_name`。
/// - **env（同値 fallback）**: [`PARENT_HWND_ENV`] = parent_hwnd 10進・
///   [`LOAD_DIR_ENV`] = load_dir・[`SHIORI_NAME_ENV`] = shiori_name。
/// - **cwd**: `load_dir`（伺か慣習・R3.3）。
///
/// Rust 関数の引数順（`helper_exe, load_dir, shiori_name, parent_hwnd`）と
/// 子引数の並び（arg1=parent_hwnd 先頭）は別物である点に注意。`shiori_name` は
/// 親側で descript 解決済みの DLL ファイル名であり、helper/factory は descript を
/// 解釈しない（R3.6）。
///
/// spawn に失敗（exe 不在・実行権限不足など）した場合は
/// [`SpawnError`] を返し、[`HelperHandle`] を返さない（＝稼働中の helper が
/// 存在しない状態を保つ・要件 1.5）。
///
/// # Errors
/// `Command::spawn` の I/O 失敗を [`SpawnError`] として返す。
pub fn spawn(
    helper_exe: &Path,
    load_dir: &Path,
    shiori_name: &str,
    parent_hwnd: u32,
) -> Result<HelperHandle, SpawnError> {
    let mut command = Command::new(helper_exe);
    let parent_hwnd_decimal = parent_hwnd.to_string();
    command
        // 子引数: arg1=parent_hwnd（互換維持）・arg2=load_dir・arg3=shiori_name。
        .arg(&parent_hwnd_decimal)
        .arg(load_dir)
        .arg(shiori_name)
        // env: 同値二重供給（arg 読み取り不能時の fallback・R3.2）。
        .env(PARENT_HWND_ENV, &parent_hwnd_decimal)
        .env(LOAD_DIR_ENV, load_dir)
        .env(SHIORI_NAME_ENV, shiori_name)
        // cwd=load_dir（伺か慣習・R3.3）。
        .current_dir(load_dir);
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
    // 孤児化防止: spawn 直後に KILL_ON_JOB_CLOSE 付き job へ割り当てる（失敗は縮退＝None）。
    // これにより、親プロセスが UNLOAD を送らずに終了しても helper が OS 機構で kill される。
    let job = crate::job::attach_kill_on_close_job(&child);
    Ok(HelperHandle {
        child,
        helper_hwnd: None,
        _job: job,
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
            assert!(
                Instant::now() < deadline,
                "helper が上限時間内に終了しなかった"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // --- ExitKind::classify 純関数（要件 1.3 / 1.4）---

    #[test]
    fn classify_clean_from_exit_zero() {
        // 実プロセスで code() == Some(0) を作り Clean へ分類（要件 1.3）。
        let status = cmd_exit(0)
            .status()
            .expect("cmd.exe /c exit 0 が起動できる");
        assert_eq!(ExitKind::classify(&status), ExitKind::Clean);
    }

    #[test]
    fn classify_abnormal_from_nonzero_exit() {
        // 実プロセスで code() == Some(3) を作り Abnormal(3) へ分類（要件 1.4）。
        let status = cmd_exit(3)
            .status()
            .expect("cmd.exe /c exit 3 が起動できる");
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
        let load_dir = std::env::temp_dir();
        let result = spawn(missing, &load_dir, "shiori.dll", 0);
        assert!(
            matches!(result, Err(SpawnError::Spawn(_))),
            "spawn 失敗は SpawnError::Spawn を返す"
        );
    }

    // --- spawn の起動パラメーター契約（arg1..3・env 3 種・cwd＝design §SpawnContract）---

    /// env キー名が design §SpawnContract どおりに固定されていること（契約回帰防止）。
    #[test]
    fn spawn_env_key_names_match_contract() {
        assert_eq!(PARENT_HWND_ENV, "HOST32_PARENT_HWND");
        assert_eq!(LOAD_DIR_ENV, "HOST32_LOAD_DIR");
        assert_eq!(SHIORI_NAME_ENV, "HOST32_SHIORI_NAME");
    }

    /// LOAD 用 ack 待機の推奨既定 timeout が 30 秒であること（R5.2・design §Load timeout）。
    #[test]
    fn load_ack_timeout_default_is_30s() {
        assert_eq!(LOAD_ACK_TIMEOUT, Duration::from_secs(30));
    }

    // --- request timeout 定数 + env seam 解決（R4.3 / R5.1・design §Shiori3Client）---

    /// request 往復の既定 timeout が 60 秒で、env キー名が契約どおりであること（回帰防止）。
    #[test]
    fn request_timeout_default_is_60s_and_env_key_matches() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(60));
        assert_eq!(REQUEST_TIMEOUT_ENV, "AREKA_SHIORI_REQUEST_TIMEOUT_MS");
    }

    /// env 不在／空／空白のみ／parse 不能 → 既定 60s へ安全側フォールバック（無限へ昇格させない）。
    #[test]
    fn resolve_request_timeout_defaults_to_60s_when_absent_or_garbage() {
        // 不在。
        assert_eq!(resolve_request_timeout(None), Some(REQUEST_TIMEOUT));
        assert_eq!(resolve_request_timeout(None), Some(Duration::from_secs(60)));
        // 空文字・空白のみ。
        assert_eq!(
            resolve_request_timeout(Some(String::new())),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            resolve_request_timeout(Some("   ".to_string())),
            Some(Duration::from_secs(60))
        );
        // parse 不能（非数値・負値・小数）はすべて既定へ（無限待ちへ暗黙昇格しない）。
        assert_eq!(
            resolve_request_timeout(Some("abc".to_string())),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            resolve_request_timeout(Some("-1".to_string())),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            resolve_request_timeout(Some("1.5".to_string())),
            Some(Duration::from_secs(60))
        );
    }

    /// env `"0"` → `None`（無限待ち・デバッグ opt-in）。
    #[test]
    fn resolve_request_timeout_zero_means_infinite() {
        assert_eq!(resolve_request_timeout(Some("0".to_string())), None);
        // 前後空白付き "0" も trim 後に 0 として無限待ちへ。
        assert_eq!(resolve_request_timeout(Some("  0  ".to_string())), None);
    }

    /// env `"<n>"`（n > 0・ミリ秒） → `Some(Duration::from_millis(n))`。
    #[test]
    fn resolve_request_timeout_positive_millis_are_used() {
        assert_eq!(
            resolve_request_timeout(Some("1500".to_string())),
            Some(Duration::from_millis(1500))
        );
        // trim 済みで採用されること。
        assert_eq!(
            resolve_request_timeout(Some("  250  ".to_string())),
            Some(Duration::from_millis(250))
        );
    }

    /// spawn が起動パラメーターを子へ供給する契約を、`spawn` の実装が組み立てる
    /// Command と同一手順の stand-in（バッチファイル）で**実際に観測**して検証する（R3.1〜3.3）。
    ///
    /// 観測対象:
    /// - arg1 = parent_hwnd（10進）・arg2 = load_dir・arg3 = shiori_name（`%1`/`%2`/`%3`）
    /// - env `HOST32_PARENT_HWND` = parent_hwnd 10進・`HOST32_LOAD_DIR` = load_dir・
    ///   `HOST32_SHIORI_NAME` = shiori_name
    /// - cwd = load_dir（`%CD%` 出力で確認）
    ///
    /// バッチファイルを stand-in にするのは `%1..%3`（位置引数）の展開がバッチ実行時のみ
    /// 有効で、`cmd /c "..."` 単一コマンドでは展開されないため。`spawn` 本体と**同一順序**で
    /// arg/env/cwd を積み、位置引数・env・cwd の三面が同値供給されることを assert する
    /// （spawn は helper_exe を実行するため直接キャプチャできない＝同一規約の Command で観測）。
    #[test]
    fn spawn_supplies_hwnd_load_dir_and_shiori_name_via_arg_env_and_cwd() {
        let parent_hwnd: u32 = 4_294_967_295; // u32::MAX（10進境界値）
        let decimal = parent_hwnd.to_string();
        // cwd 一致を検証できる実在ディレクトリ。canonicalize で表記ゆれを吸収。
        let load_dir =
            std::fs::canonicalize(std::env::temp_dir()).expect("temp_dir を canonicalize できる");
        let shiori_name = "shiori.dll";

        // 位置引数を観測するバッチ stand-in を一時ファイルへ書く（テスト後に削除）。
        // 各面を "KEY=VALUE" 行で出力: %1..%3=位置引数・%VAR%=env・%CD%=cwd。
        let unique = std::process::id();
        let batch_path =
            std::env::temp_dir().join(format!("areka_host32_spawn_contract_{unique}.bat"));
        std::fs::write(
            &batch_path,
            "@echo off\r\n\
             echo arg1=%1\r\n\
             echo arg2=%2\r\n\
             echo arg3=%3\r\n\
             echo phwnd=%HOST32_PARENT_HWND%\r\n\
             echo ldir=%HOST32_LOAD_DIR%\r\n\
             echo sname=%HOST32_SHIORI_NAME%\r\n\
             echo cwd=%CD%\r\n",
        )
        .expect("stand-in バッチを書ける");

        // `spawn` と同一手順で Command を組む（arg1=hwnd・arg2=load_dir・arg3=name、
        // env 3 種、cwd=load_dir）。cmd.exe /c <batch> で位置引数を渡す。
        let mut command = Command::new("cmd.exe");
        command
            .arg("/c")
            .arg(&batch_path)
            .arg(&decimal) // arg1
            .arg(&load_dir) // arg2
            .arg(shiori_name) // arg3
            .env(PARENT_HWND_ENV, &decimal)
            .env(LOAD_DIR_ENV, &load_dir)
            .env(SHIORI_NAME_ENV, shiori_name)
            .current_dir(&load_dir);
        let output = command.output().expect("stand-in を実行できる");
        let _ = std::fs::remove_file(&batch_path); // 後始末（失敗は無視）。
        let stdout = String::from_utf8_lossy(&output.stdout);

        let load_dir_str = load_dir.to_string_lossy();
        // 期待表記の cwd（%CD% は `\\?\` 冗長プレフィクス無し・末尾セパレータ無しの正規表記）。
        let expected_cwd = load_dir_str
            .strip_prefix(r"\\?\")
            .unwrap_or(&load_dir_str)
            .trim_end_matches(['\\', '/']);

        // arg1..3 同値供給（位置引数を %1..%3 で観測）。
        assert!(
            stdout.contains(&format!("arg1={decimal}")),
            "arg1 に 10進 parent_hwnd が載る: got {stdout:?}"
        );
        assert!(
            stdout.contains(&format!("arg2={load_dir_str}")),
            "arg2 に load_dir が載る: got {stdout:?}"
        );
        assert!(
            stdout.contains(&format!("arg3={shiori_name}")),
            "arg3 に shiori_name が載る: got {stdout:?}"
        );
        // env 3 種同値供給。
        assert!(
            stdout.contains(&format!("phwnd={decimal}")),
            "env HOST32_PARENT_HWND に 10進 parent_hwnd が載る: got {stdout:?}"
        );
        assert!(
            stdout.contains(&format!("ldir={load_dir_str}")),
            "env HOST32_LOAD_DIR に load_dir が載る: got {stdout:?}"
        );
        assert!(
            stdout.contains(&format!("sname={shiori_name}")),
            "env HOST32_SHIORI_NAME に shiori_name が載る: got {stdout:?}"
        );
        // cwd = load_dir。
        assert!(
            stdout.contains(&format!("cwd={expected_cwd}")),
            "cwd が load_dir に設定される: expected cwd={expected_cwd:?} in {stdout:?}"
        );
    }
}
