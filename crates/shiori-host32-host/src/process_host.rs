//! `ProcessHost`（要件 1.1〜1.5）: helper プロセスの起動と、IPC と直交する
//! 非ブロッキング生存監視。
//!
//! 判定の純ロジック（`ExitKind::classify`・`resolve_request_timeout` 等）は
//! `std::process` の上だけで組み、実プロセスなしで単体テストできる。親ウィンドウのハンドルは
//! `HWND` 型を引きずらず u32 ワイヤ値として子へ渡す。`windows` を使うのは起こし方と待ち方の所
//! （`CREATE_NO_WINDOW`・終わりの待ち `WaitForSingleObject`・標準エラーの ANSI 読み
//! `MultiByteToWideChar`）と、[`spawn_command`] が helper の**孤児化防止**として割り当てる
//! KILL_ON_JOB_CLOSE 付き Job Object（[`crate::job`] にまとめ、割り当て失敗時は縮退＝現行挙動）。
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
//!
//! helper の向きの確かめ（dev-helper-x64-clobber 要件 3.1〜3.6）: [`spawn`] は起こす前に、
//! 渡された helper の実行ファイルの見出し（先頭の最大 4 KiB）を読み、32bit（i686）版で
//! なければ理由と直し方の入ったエラーの 1 行（`helper_not_i686`）を、確かめられなければ
//! 警告の 1 行（`helper_image_unverified`）を残す。記録だけで、`spawn` の結果も起こし方も
//! 変えない。見るのは渡されたパスだけ（探さない）。[`spawn_command`] では確かめない
//! （helper でない子を起こすテストが記録を出さない）。

use std::io::BufRead;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;

use windows::Win32::System::Threading::CREATE_NO_WINDOW;

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
    /// 終了コードを持たない終了（`ExitStatus::code()` が `None`・要件 1.4）。
    ///
    /// Windows の `ExitStatus::code()` はいつも値を持つので、この腕は Windows では起きない
    /// （`TerminateProcess(..,1)` は [`ExitKind::Abnormal(1)`]）。公開の形を変えないために残す。
    Terminated,
}

impl ExitKind {
    /// [`ExitStatus`] を終了種別へ分類する純関数（要件 1.3 / 1.4）。
    ///
    /// - `code() == Some(0)` → [`ExitKind::Clean`]
    /// - `code() == Some(n)`（n ≠ 0） → [`ExitKind::Abnormal(n)`]
    /// - `code() == None` → [`ExitKind::Terminated`]（Windows では起きない）
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
/// 起動した子プロセスの [`Child`] を保持し、[`poll_exit_kind`] による非ブロッキング生存監視の
/// 対象となる。
///
/// **孤児化防止**: spawn 時に helper を `KILL_ON_JOB_CLOSE` 付き Job Object へ割り当て、
/// その job ハンドルを `_job` として保持する（[`crate::job`]）。本ハンドルを保持する
/// 親プロセスが終了（正常・異常・UNLOAD 未送出のいずれも）すると OS が job を閉じ、
/// helper を確実に道連れにする。helper は親の死を自分では検知しないため、親が
/// UNLOAD を送らずに終わったときの安全網はこれだけ。job 割り当てに失敗した場合は `None`（縮退・現行挙動）。
#[derive(Debug)]
pub struct HelperHandle {
    child: Child,
    /// KILL_ON_JOB_CLOSE 付き Job Object の RAII ハンドル（孤児化防止）。`Drop` で
    /// `CloseHandle`→job クローズ→helper kill。割り当て失敗時は `None`（縮退）。
    _job: Option<std::os::windows::io::OwnedHandle>,
    /// helper の標準エラーの読み手のスレッド（標準エラーが管のときだけ `Some`）。管が閉じれば
    /// （helper とその書き口を受け継いだ子がみな終われば）自分で終わる。`HelperHandle` の Drop が
    /// helper の終わりの後にこれを 1 秒まで待つ（U7）ので、ここに持っておく。
    stderr_reader: Option<JoinHandle<()>>,
}

impl HelperHandle {
    /// 保持する子プロセスの OS プロセス ID を返す（観測・デバッグ用）。
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// helper を強制終了する（本番の kill の経路: 本型の `Drop` と `HelperLifecycle::terminate` が使う）。
    ///
    /// 既に終了済みの場合も `Ok(())` として扱う（冪等）。強制終了後の
    /// 終了種別は [`poll_exit_kind`] で観測できる。
    ///
    /// # Errors
    /// `TerminateProcess` 相当の I/O 失敗時に [`std::io::Error`] を返す。
    pub fn terminate(&mut self) -> std::io::Result<()> {
        // Windows の std の kill は、もう終わったプロセスにも Ok を返す（冪等）。
        self.child.kill()
    }

    /// helper の終わりを `timeout` まで待つ（`WaitForSingleObject` 1 回）。終わっていれば終了種別、
    /// 待ちきれなければ `None`。待つ長さはミリ秒へ切り上げ（短く待たない）、`u32::MAX - 1` で抑える
    /// （`INFINITE` にしない）。
    pub(crate) fn wait_for_exit(&mut self, timeout: Duration) -> Option<ExitKind> {
        wait_handle(self.child.as_raw_handle(), timeout);
        poll_exit_kind(self)
    }

    /// `Child` を持たないスレッドから helper を終わらせる取っ手を作る（プロセスの取っ手の複製）。
    ///
    /// # Errors
    /// 取っ手の複製（`DuplicateHandle`）の失敗を [`std::io::Error`] として返す。
    pub fn terminator(&self) -> std::io::Result<crate::HelperTerminator> {
        crate::HelperTerminator::from_child(&self.child)
    }
}

/// Drop が helper の終わりを待つ上限。
const DROP_EXIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Drop が helper の終わりの後に標準エラーの読み手の終わりを待つ上限。
const DROP_READER_TIMEOUT: Duration = Duration::from_secs(1);

/// 取っ手（プロセス・スレッド）がシグナル状態になるまで `timeout` まで待ち、なったら `true`。
fn wait_handle(handle: std::os::windows::io::RawHandle, timeout: Duration) -> bool {
    use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::WaitForSingleObject;
    let millis = timeout
        .as_nanos()
        .div_ceil(1_000_000)
        .min(u128::from(u32::MAX - 1)) as u32;
    // SAFETY: `handle` は呼び手が持つ生きた取っ手（`Child`／`JoinHandle` が借り元）。借りるだけ。
    unsafe { WaitForSingleObject(HANDLE(handle), millis) == WAIT_OBJECT_0 }
}

/// 包む前に捨てられた helper も、job を持たない helper も残さない（指摘 14）。
impl Drop for HelperHandle {
    fn drop(&mut self) {
        let pid = self.pid();
        // 終わっていれば `terminate` は `Ok`（冪等）。
        if let Err(error) = self.terminate() {
            tracing::error!(
                target: LOG_TARGET,
                event = "helper_kill_on_drop_failed",
                pid,
                %error,
                "helper を終わらせられなかった"
            );
        }
        if self.wait_for_exit(DROP_EXIT_TIMEOUT).is_none() {
            tracing::error!(
                target: LOG_TARGET,
                event = "helper_exit_timeout_on_drop",
                pid,
                "helper が上限時間内に終わらなかった"
            );
        }
        // 終わった helper の最後の行を記録へ入れきってから戻る。書き口を受け継いだ孫が残れば管は
        // 閉じないので、待ちは 1 秒で切り上げる。
        if let Some(reader) = self.stderr_reader.take() {
            if wait_handle(reader.as_raw_handle(), DROP_READER_TIMEOUT) {
                let _ = reader.join();
            } else {
                tracing::debug!(
                    target: LOG_TARGET,
                    event = "helper_stderr_reader_detached",
                    pid,
                    "helper の標準エラーの読み手が 1 秒で終わらず、切り離した"
                );
            }
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
/// - **`PASTA_DEBUG=0`**: 親の値を受け継がせない（[`helper_command`]）。
/// - **標準エラー**: 管にし、行ごとに `warn`（`event = "helper_stderr"`）で記録へ回す。
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
    check_helper_image(helper_exe);
    spawn_command(helper_command(
        helper_exe,
        load_dir,
        shiori_name,
        parent_hwnd,
    ))
}

/// helper を起こす [`Command`] を 1 か所で組む（引数・環境・作業フォルダ・標準エラーの管）。中身はテストが
/// `get_args`／`get_envs`／`get_current_dir` で読む。
///
/// `PASTA_DEBUG` は常に `0`（開ける口は無い＝要件 8.1・8.2・10.4）。親（areka）に立っていても
/// helper の下の SHIORI（pasta）へは受け継がせない。
fn helper_command(
    helper_exe: &Path,
    load_dir: &Path,
    shiori_name: &str,
    parent_hwnd: u32,
) -> Command {
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
        .env("PASTA_DEBUG", "0")
        // cwd=load_dir（伺か慣習・R3.3）。
        .current_dir(load_dir);
    // helper の `[helper]` の行を areka の記録へ回す（[`spawn_command`] が読み手を起こす）。
    command.stderr(Stdio::piped());
    // 窓だけの areka には標準の取っ手が無く、受け継ぐと helper が毎回「取っ手が無い」を書くので、
    // helper が使わない標準入力・標準出力は NUL を渡す（`Command` は stdio の設定を読み返せず、テストでは縛れない）。
    command.stdin(Stdio::null()).stdout(Stdio::null());
    command
}

/// 構築済み [`Command`] を起動して [`HelperHandle`] へ包むテスト可能な下位 seam。
///
/// [`spawn`] は helper 用に `Command`（arg/env/cwd）を組み立ててこれを呼ぶ。
/// 単体テストは `cmd.exe /c exit N` 等の決定的 stand-in を組んだ `Command` を
/// 直接渡し、i686 helper exe の有無に依存せず spawn/poll/分類を検証する。
///
/// 補助プロセスはコンソールの実行体なので、ここで [`CREATE_NO_WINDOW`] を付けて起こす
/// （GUI の `areka.exe` から窓の指定なしで起こすと新しいコンソール窓が開き、利用者が閉じると
/// 補助プロセスが殺される）。`HelperHandle` に包む子の起こし方の決まり（job への割り当てと同じ
/// 置き場）なので、`arg`／`env`／`cwd` の契約を組む [`spawn`] ではなくこの seam に置く。
/// 標準入出力の扱いは変えない（受け取った `Command` の指定のまま）。標準エラーが管なら、
/// 行を `warn` で記録へ回す読み手のスレッドを起こす（[`log_helper_line`]）。
///
/// # Errors
/// `Command::spawn` の I/O 失敗を [`SpawnError`] として返す。
pub fn spawn_command(command: Command) -> Result<HelperHandle, SpawnError> {
    spawn_command_with(command, log_helper_line)
}

/// [`spawn_command`] の中身。標準エラーの行を渡す先 `sink(pid, 行)` を差し替えられる（テスト用）。
pub(crate) fn spawn_command_with(
    mut command: Command,
    mut sink: impl FnMut(u32, &str) + Send + 'static,
) -> Result<HelperHandle, SpawnError> {
    let mut child = command.creation_flags(CREATE_NO_WINDOW.0).spawn()?;
    // 孤児化防止: spawn 直後に KILL_ON_JOB_CLOSE 付き job へ割り当てる（失敗は縮退＝None）。
    // これにより、親プロセスが UNLOAD を送らずに終了しても helper が OS 機構で kill される。
    // shortcut: 起こしてから job に入れるまでのあいだに areka が落ちると helper が残る。閉じるには CREATE_SUSPENDED か PROC_THREAD_ATTRIBUTE_JOB_LIST（std の Command の外）が要る。残った helper が実際に見つかったら上げる
    let job = crate::job::attach_kill_on_close_job(&child);
    let pid = child.id();
    let stderr_reader = child.stderr.take().and_then(|stderr| {
        std::thread::Builder::new()
            .name("shiori-host32-stderr".into())
            .spawn(move || {
                if let Err(e) =
                    forward_helper_stderr(std::io::BufReader::new(stderr), |line| sink(pid, line))
                {
                    tracing::error!(
                        target: LOG_TARGET,
                        event = "helper_stderr_read_failed",
                        pid,
                        error = %e,
                        "helper の標準エラーを読めなかった——ここから先の行は記録へ回らない"
                    );
                }
            })
            .map_err(|e| {
                tracing::error!(
                    target: LOG_TARGET,
                    event = "helper_stderr_reader_spawn_failed",
                    pid,
                    error = %e,
                    "helper の標準エラーの読み手を起こせなかった——helper の行は記録へ回らない"
                );
            })
            .ok()
    });
    Ok(HelperHandle {
        child,
        _job: job,
        stderr_reader,
    })
}

/// helper の標準エラーを LF で切り（行末の CR を落とす）、行ごとに `sink` を呼ぶ。管の終わりで
/// `Ok`、読みの失敗で `Err` を返す（呼び手が記録して終わる）。
///
/// 行は UTF-8 で読み、UTF-8 でない行は OS の ANSI コードページで読む（文字化けは読む側で直す）。
fn forward_helper_stderr(
    mut reader: impl BufRead,
    mut sink: impl FnMut(&str),
) -> std::io::Result<()> {
    let mut buf = Vec::new();
    loop {
        buf.clear();
        // shortcut: 1 行の長さに上限なし（改行の無い大きな書き込みは丸ごと貯まる）、実害が出たら上限で切る
        if reader.read_until(b'\n', &mut buf)? == 0 {
            return Ok(());
        }
        let mut line = buf.as_slice();
        line = line.strip_suffix(b"\n").unwrap_or(line);
        line = line.strip_suffix(b"\r").unwrap_or(line);
        match std::str::from_utf8(line) {
            Ok(text) => sink(text),
            Err(_) => sink(&decode_ansi(line)),
        }
    }
}

/// ANSI コードページ（`CP_ACP`）のバイト列を読む。読めなければ置き換えの字で読む（行は落とさない）。
fn decode_ansi(bytes: &[u8]) -> String {
    use windows::Win32::Globalization::{
        CP_ACP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS, MultiByteToWideChar,
    };
    // SAFETY: `bytes` は有効なバイト列。出力 None＝長さの問い合わせ。既定のフラグ(0)。
    let len = unsafe { MultiByteToWideChar(CP_ACP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0), bytes, None) };
    if len > 0 {
        let mut wide = vec![0u16; len as usize];
        // SAFETY: `wide` は `len` 要素の可変領域。`bytes` は有効なバイト列。既定のフラグ(0)。
        let got = unsafe {
            MultiByteToWideChar(
                CP_ACP,
                MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0),
                bytes,
                Some(&mut wide),
            )
        };
        if got > 0 {
            return String::from_utf16_lossy(&wide[..got as usize]);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// 本番の行の渡し先: helper の 1 行を文を変えずに `warn` で書く（Display＝逃がしの字を入れない）。
fn log_helper_line(pid: u32, line: &str) {
    tracing::warn!(target: LOG_TARGET, event = "helper_stderr", pid, "{line}");
}

/// helper の終了を非ブロッキングで問い合わせ、終了種別を分類する（要件 1.2〜1.4）。
///
/// 稼働中なら `None`、終了済みなら `Some(ExitKind)`（[`ExitKind::classify`]
/// による分類）。`try_wait` ベースゆえ呼び出し側スレッドをブロックしない。
#[must_use]
pub fn poll_exit_kind(handle: &mut HelperHandle) -> Option<ExitKind> {
    note_try_wait(handle.child.try_wait())
}

/// `try_wait` の結果を終了種別へ写す。失敗は `error!` を書いてから稼働中（`None`）と
/// みなす（非ブロッキングのまま・黙って握らない）。
fn note_try_wait(result: std::io::Result<Option<ExitStatus>>) -> Option<ExitKind> {
    match result {
        Ok(Some(status)) => Some(ExitKind::classify(&status)),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(
                target: LOG_TARGET,
                event = "helper_try_wait_failed",
                %error,
                "helper の終了を問い合わせられなかった——動いているとみなす"
            );
            None
        }
    }
}

/// helper の判定の記録の宛先（出す側＝`shiori-host32-host` の名前）。
///
/// `RUST_LOG` は **target 名**で指定する（`shiori-host32=debug`）。
pub(crate) const LOG_TARGET: &str = "shiori-host32";

/// PE の機械の種類 i386（32bit の SHIORI を読める helper の印）。
const IMAGE_FILE_MACHINE_I386: u16 = 0x014C;

/// helper の見出しとして読む先頭の上限。
// ponytail: 先頭 4 KiB だけを見る。PE の見出しをこれより後ろに置く実行ファイルは `NotPe`
// （警告）になる。MSVC の linker が作る helper の `e_lfanew` は 0x100 前後なので当たらない。
// 当たったら `e_lfanew` へ seek して読む形へ広げる。
const HELPER_HEAD_LIMIT: u64 = 4096;

/// helper の実行ファイルの見出しが示す形（判定の結果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelperImage {
    /// `MZ`・`PE\0\0`・機械の種類 0x014c（i386）。32bit の SHIORI を読める。
    I686,
    /// PE だが機械の種類が i386 でない（例: 0x8664＝x64・0xAA64＝arm64）。
    OtherMachine(u16),
    /// `MZ` が無い・`e_lfanew` が読んだ範囲の外・`PE\0\0` が無い。
    NotPe,
}

/// 見出しのバイト列だけから判定する純粋な関数（I/O なし・決定論）。
fn classify_helper_image(head: &[u8]) -> HelperImage {
    let machine = (|| {
        if head.get(0..2)? != b"MZ" {
            return None;
        }
        let e_lfanew =
            usize::try_from(u32::from_le_bytes(head.get(0x3C..0x40)?.try_into().ok()?)).ok()?;
        if head.get(e_lfanew..e_lfanew.checked_add(4)?)? != b"PE\0\0" {
            return None;
        }
        let at = e_lfanew + 4;
        Some(u16::from_le_bytes(head.get(at..at + 2)?.try_into().ok()?))
    })();
    match machine {
        Some(IMAGE_FILE_MACHINE_I386) => HelperImage::I686,
        Some(m) => HelperImage::OtherMachine(m),
        None => HelperImage::NotPe,
    }
}

/// 判定の結果を記録する（`I686` は何も出さない・ほかはどれも 1 行）。
///
/// 記録だけを行い、helper の起こし方は変えない（判定は助言であり、失敗の道を作らない）。
fn report_helper_image(helper_exe: &Path, image: std::io::Result<HelperImage>) {
    match image {
        Ok(HelperImage::I686) => {}
        Ok(HelperImage::OtherMachine(machine)) => tracing::error!(
            target: LOG_TARGET,
            event = "helper_not_i686",
            path = %helper_exe.display(),
            machine = %format_args!("{machine:#06x}"),
            "helper が 32bit（i686）版でないので 32bit の SHIORI を読めない——このパスへ i686 版の \
             shiori-host32-helper.exe を置くこと"
        ),
        Ok(HelperImage::NotPe) => tracing::warn!(
            target: LOG_TARGET,
            event = "helper_image_unverified",
            path = %helper_exe.display(),
            reason = "not a PE image",
            "helper の向きを確かめられなかった（実行ファイルの形をしていない）——起こし方は変えずに続ける"
        ),
        Err(e) => tracing::warn!(
            target: LOG_TARGET,
            event = "helper_image_unverified",
            path = %helper_exe.display(),
            error = %e,
            "helper の向きを確かめられなかった（読めない・無い）——起こし方は変えずに続ける"
        ),
    }
}

/// 先頭の最大 [`HELPER_HEAD_LIMIT`] バイトを読み、判定して記録する。
///
/// 読み終えたら閉じる（ロックを持ち続けない）。見るのは渡されたパスだけ（探さない）。
fn check_helper_image(helper_exe: &Path) {
    use std::io::Read;
    let image = std::fs::File::open(helper_exe).and_then(|file| {
        let mut head = Vec::new();
        file.take(HELPER_HEAD_LIMIT).read_to_end(&mut head)?;
        Ok(classify_helper_image(&head))
    });
    report_helper_image(helper_exe, image);
}

#[cfg(test)]
#[path = "process_host_console_tests.rs"]
mod console_tests;

#[cfg(test)]
#[path = "process_host_image_tests.rs"]
mod image_tests;

#[cfg(test)]
#[path = "process_host_stderr_tests.rs"]
mod stderr_tests;

#[cfg(test)]
#[path = "process_host_tests.rs"]
pub(crate) mod tests;
