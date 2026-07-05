//! helper ライフサイクル管理（要件 R5〜R7）。
//!
//! 本モジュールは helper プロセスの正常終了要求（unload → ループ正常終了 →
//! `exit(0)`）と、その結果の観測・分類を担う。将来的に以下を提供する予定
//! （いずれも後続タスクで実装）:
//! - `HelperLifecycle` — ライフサイクルの状態機械。
//! - `classify_failure` — 終了コード/シグナルから `FailureClass` への分類。
//! - `request_clean_shutdown` — 正常終了（Clean shutdown）要求の発行。
//!
//! 本タスク（1.2）では統一報告語彙・shutdown 失敗語彙の型骨格と定数を定義する:
//! - [`HelperStatus`] — 現在の生存/終了ステータス。
//! - [`FailureClass`] — 死活・request 失敗の突合分類。
//! - [`LifecycleReport`] — 突合結果と原因を束ねる統一報告。
//! - [`ShutdownError`] — shutdown 経路専用の失敗語彙。
//! - [`UNLOAD_ACK_TIMEOUT`] / [`EXIT_OBSERVE_TIMEOUT`] / [`EXIT_POLL_INTERVAL`] — 関連定数。

use std::time::Duration;

use crate::error::RequestError;
use crate::parent_window::SendError;
use crate::process_host::{ExitKind, HelperHandle, poll_exit_kind};

/// 死活監視の観測結果（Send・Copy な所有データ・R2.6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelperStatus {
    /// 稼働中（終了未検出・R1.1）。
    Running,
    /// 終了検出済み（種別付き・R1.2〜1.5）。sticky（以後不変）。
    Exited(ExitKind),
}

/// 統一失敗分類（R2.1〜2.3・Send・Copy）。処分判断は含まない（R2.7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// helper 死亡起因（失敗時点で終了が観測された・R2.1）。保持値は観測された終了種別。
    HelperDown(ExitKind),
    /// helper 生存（終了未検出）だが上限時間内に応答なし（wire timeout・R2.2）。
    Unresponsive,
    /// SHIORI エラー応答（400/500/ErrorLevel・helper は生存・R2.3）。
    ShioriFailure,
    /// helper 生存だが transport 送出失敗（Ipc・死亡未検出の境界異常）。
    Transport,
    /// 未ハンドシェイク（構造上通常起きない・区別のため潰さない・R2.4）。
    Handshake,
}

/// 統一報告（本仕様が正本・kanade は再定義しない・R2.4/2.6）。
/// class（突合結果）と error（原因の RequestError・凍結型を所有内包）の二軸を保持し、
/// 単一の不透明失敗へ潰さない。
#[derive(Debug)]
pub struct LifecycleReport {
    pub class: FailureClass,
    pub error: RequestError,
}

/// UNLOAD ack（unload 完了確認）待機の上限（LOAD_ACK_TIMEOUT と同値 30s）。
pub const UNLOAD_ACK_TIMEOUT: Duration = Duration::from_secs(30);
/// ack 受領後、プロセス終了（ExitKind 観測）までの bounded poll 上限。
pub const EXIT_OBSERVE_TIMEOUT: Duration = Duration::from_secs(10);
/// bounded poll の刻み（実時間 sleep はこれのみ・R7.5）。
pub const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// 突合の正本純関数（R2.1〜2.4・決定的・窓/プロセス非依存で単体テスト可）。
///
/// | observed_exit          | error                  | 結果                    |
/// |------------------------|------------------------|-------------------------|
/// | Some(kind)（終了検出） | （何であれ）           | HelperDown(kind)        |
/// | None（生存）           | RequestError::Timeout  | Unresponsive            |
/// | None                   | RequestError::Ipc      | Transport               |
/// | None                   | RequestError::Shiori   | ShioriFailure           |
/// | None                   | RequestError::Handshake| Handshake               |
///
/// 不変条件: 終了検出は他のすべてに優先する。helper 死亡が観測されていれば
/// 表面の `error` が `Timeout` / `Ipc` であっても `HelperDown(kind)` を返す。
/// `observed_exit == Some(ExitKind::Clean)` でも `HelperDown(Clean)` を返す
/// （死は死・種別は呼び出し側が読む）。実装は `observed_exit` を先に突合し、
/// `None`（生存）の場合にのみ `error` を突合する。
pub fn classify_failure(error: &RequestError, observed_exit: Option<ExitKind>) -> FailureClass {
    // 終了検出は他のすべてに優先する（R2.1）: 死亡が観測されていれば
    // 表面の error が何であれ HelperDown(kind) を返す。
    if let Some(kind) = observed_exit {
        return FailureClass::HelperDown(kind);
    }
    // 生存中（None）の場合のみ error を突合する（R2.2〜2.4）。
    match error {
        RequestError::Timeout => FailureClass::Unresponsive,
        RequestError::Ipc(_) => FailureClass::Transport,
        RequestError::Shiori(_) => FailureClass::ShioriFailure,
        RequestError::Handshake(_) => FailureClass::Handshake,
    }
}

/// 死活監視の常設の器（R1.1〜1.6, 4.1, 5.2, 7.7）。
///
/// spawn 済み [`HelperHandle`] を**単独所有**し、非ブロッキング・sticky な死活問い合わせと
/// 冪等な後始末を提供する。request 経路（`Shiori3Client`＝窓借用）とは別オブジェクトであり、
/// 包まない・仲介しない（design §HelperLifecycle Responsibilities）。
///
/// `Child` は `Send` ゆえ本型も `Send`（将来の shiori アクター inbox から移送可能・R7.7）。
/// 窓（`!Send`）は保持しない。
///
/// `report_failure`（R2.1〜2.5・突合）は本タスク（2.3）で追加済み。
/// `request_clean_shutdown`（R5.1・正常終了経路）は後続タスク（2.4）で追加する。
pub struct HelperLifecycle {
    handle: HelperHandle,
    /// sticky 終了キャッシュ（一度 `Some` になったら以後 poll しない・R1.2）。
    /// 不変条件: `None → Some` の一方向のみ（終了は終端状態）。
    last_exit: Option<ExitKind>,
}

impl HelperLifecycle {
    /// spawn 済み handle を監視の器に載せる（所有移転・R1.6）。
    #[must_use]
    pub fn new(handle: HelperHandle) -> Self {
        Self {
            handle,
            last_exit: None,
        }
    }

    /// 非ブロッキング死活問い合わせ（R1.1/1.2）。`try_wait` ベースで呼び手を止めない。
    ///
    /// 終了検出後は **sticky**: 一度 [`HelperStatus::Exited`] を観測したら以後は再 poll せず
    /// 同じ [`ExitKind`] を返し続ける（終了は終端状態・観測の決定性）。
    pub fn status(&mut self) -> HelperStatus {
        // sticky: 一度 Exited を観測したら再 poll しない（`std` の try_wait の
        // reap 後挙動に依存せず観測を型内で決定化する）。
        if let Some(kind) = self.last_exit {
            return HelperStatus::Exited(kind);
        }
        match poll_exit_kind(&mut self.handle) {
            Some(kind) => {
                self.last_exit = Some(kind);
                HelperStatus::Exited(kind)
            }
            None => HelperStatus::Running,
        }
    }

    /// 強制終了（冪等・二重 kill 安全・R5.2）。既存 [`HelperHandle::terminate`] へ委譲する。
    ///
    /// `HelperHandle::terminate` は既終了プロセスへの kill（`InvalidInput`）を `Ok` へ畳むため、
    /// 二重呼び出しでも成功扱いになる。`Err` は握り潰さず `error!` ＋そのまま返す（R4.4/R7.6）。
    ///
    /// # Errors
    /// `TerminateProcess` 相当の I/O 失敗時に [`std::io::Error`] を返す。
    pub fn terminate(&mut self) -> std::io::Result<()> {
        match self.handle.terminate() {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!(error = ?e, "[HelperLifecycle::terminate] helper terminate failed");
                Err(e)
            }
        }
    }

    /// request 失敗と死活を突合し統一報告を作る（R2.1〜2.5）。
    ///
    /// 内部で [`Self::status`]（非ブロッキング・sticky・task 2.2）を採り、その観測結果を
    /// [`classify_failure`] へ渡す。終了検出は他のすべてに優先するため、表面の `error` が
    /// `Timeout` / `Ipc` であっても helper 死亡が観測されていれば `HelperDown(kind)` を返す。
    ///
    /// 死活起因（[`FailureClass::HelperDown`]）と判定された時のみ `error!` を発行する
    /// （log-first・R4.4/R7.6）。他の分類（`Unresponsive` / `ShioriFailure` / `Transport`
    /// / `Handshake`）はここではログしない（処分判断は呼び手・R2.7）。
    ///
    /// `RequestError` は `Clone` を持たないため、報告は原因を**移動**で内包する（R2.4）。
    /// `error!` はこの move の**前**に参照でログするので、`class` と原本 `error` の双方が
    /// 潰されずに [`LifecycleReport`] として返る。
    pub fn report_failure(&mut self, error: RequestError) -> LifecycleReport {
        // status() は sticky・非ブロッキング（task 2.2）。Exited(kind)→Some(kind) / Running→None。
        let observed_exit = match self.status() {
            HelperStatus::Exited(kind) => Some(kind),
            HelperStatus::Running => None,
        };
        let class = classify_failure(&error, observed_exit);
        // 死活起因（HelperDown）検出時は log-first で surface（R4.4/R7.6）。
        // error は下で LifecycleReport へ move するため、ここで参照して先にログする。
        if let FailureClass::HelperDown(kind) = class {
            tracing::error!(
                exit = ?kind,
                error = ?error,
                "[HelperLifecycle::report_failure] request failed due to helper death"
            );
        }
        LifecycleReport { class, error }
    }

    /// 観測用: OS プロセス ID。
    #[must_use]
    pub fn pid(&self) -> u32 {
        self.handle.pid()
    }
}

impl Drop for HelperLifecycle {
    /// 冪等 `terminate`（panic 経路のプロセスリーク防止・HelperGuard パターンの昇格）。
    ///
    /// `Err` は `error!` のみ（Drop で panic しない・R7.6）。
    fn drop(&mut self) {
        if let Err(e) = self.handle.terminate() {
            tracing::error!(error = ?e, "[HelperLifecycle::drop] helper terminate on drop failed");
        }
    }
}

/// shutdown 経路の失敗語彙（R5.5・error! と対で surface）。
#[derive(thiserror::Error, Debug)]
pub enum ShutdownError {
    /// UNLOAD の送出／ack 往復が失敗（SendError を所有内包・凍結型不改変）。
    #[error("unload round-trip failed: {0}")]
    Unload(#[from] SendError),
    /// ack が厳密 1 byte [1] でない（契約違反の観測）。
    #[error("unexpected unload ack: {0:?}")]
    UnexpectedAck(Vec<u8>),
    /// ack 受領後、上限時間内にプロセス終了が観測できない。
    #[error("helper did not exit within bound after unload ack")]
    ExitTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `T: Send` を静的に固定する（R2.6/7.7・報告データは Send な所有データ）。
    /// 本関数が対象型で単相化できること自体が Send の証明であり、
    /// コンパイルが通れば当該型は Send（通らなければ型が Send でない）。
    const fn assert_send<T: Send>() {}

    #[test]
    fn lifecycle_report_types_are_send() {
        assert_send::<HelperStatus>();
        assert_send::<FailureClass>();
        assert_send::<LifecycleReport>();
        assert_send::<ShutdownError>();
        // HelperLifecycle も Send（Child は Send・R7.7・design §395）。
        assert_send::<HelperLifecycle>();
    }

    use crate::error::{HandshakeError, ShioriError};
    use shiori_host32_ipc::IpcError;

    // --- 突合表 5 パターン（R2.1〜2.4）---

    /// 行1（死優先）: `Some(Clean)` × `Timeout` → `HelperDown(Clean)`。
    /// 表面の error が Timeout でも終了検出が優先することを証明する。
    #[test]
    fn classify_death_precedes_timeout() {
        let got = classify_failure(&RequestError::Timeout, Some(ExitKind::Clean));
        assert_eq!(got, FailureClass::HelperDown(ExitKind::Clean));
    }

    /// 行1（死優先・種別保持）: `Some(Abnormal(5))` × `Ipc` → `HelperDown(Abnormal(5))`。
    #[test]
    fn classify_death_precedes_ipc_kind_preserved() {
        let got = classify_failure(
            &RequestError::Ipc(IpcError::SendFailed),
            Some(ExitKind::Abnormal(5)),
        );
        assert_eq!(got, FailureClass::HelperDown(ExitKind::Abnormal(5)));
    }

    /// 行1（死優先・種別保持）: `Some(Terminated)` × `Shiori` → `HelperDown(Terminated)`。
    #[test]
    fn classify_death_precedes_shiori_kind_preserved() {
        let got = classify_failure(
            &RequestError::Shiori(ShioriError::Parse),
            Some(ExitKind::Terminated),
        );
        assert_eq!(got, FailureClass::HelperDown(ExitKind::Terminated));
    }

    /// 行2（生存）: `None` × `Timeout` → `Unresponsive`。
    #[test]
    fn classify_alive_timeout_is_unresponsive() {
        let got = classify_failure(&RequestError::Timeout, None);
        assert_eq!(got, FailureClass::Unresponsive);
    }

    /// 行3（生存）: `None` × `Ipc` → `Transport`。
    #[test]
    fn classify_alive_ipc_is_transport() {
        let got = classify_failure(&RequestError::Ipc(IpcError::SendFailed), None);
        assert_eq!(got, FailureClass::Transport);
    }

    /// 行4（生存）: `None` × `Shiori` → `ShioriFailure`。
    #[test]
    fn classify_alive_shiori_is_shiori_failure() {
        let got = classify_failure(&RequestError::Shiori(ShioriError::Parse), None);
        assert_eq!(got, FailureClass::ShioriFailure);
    }

    /// 行5（生存）: `None` × `Handshake` → `Handshake`。
    #[test]
    fn classify_alive_handshake_is_handshake() {
        let got = classify_failure(
            &RequestError::Handshake(HandshakeError::Incomplete),
            None,
        );
        assert_eq!(got, FailureClass::Handshake);
    }

    // --- HelperLifecycle: 非ブロッキング・分類・sticky・冪等後始末（R1.1〜1.6, 5.2）---

    use crate::process_host::spawn_command;
    use std::process::Command;
    use std::time::Instant;

    /// stand-in（`cmd.exe /c exit N`）を組んだ Command を返す（process_host tests と同意匠）。
    fn cmd_exit(code: i32) -> Command {
        let mut command = Command::new("cmd.exe");
        command.args(["/c", "exit", &code.to_string()]);
        command
    }

    /// stand-in が終了するまで非ブロッキング `status()` を bounded に回して `Exited` を得る。
    ///
    /// 各 `status()` はブロックしない（R1.1/1.2）。全体は上限 10s・刻み 5ms で待つ。
    fn wait_exited(lc: &mut HelperLifecycle) -> ExitKind {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let HelperStatus::Exited(kind) = lc.status() {
                return kind;
            }
            assert!(Instant::now() < deadline, "stand-in が上限時間内に終了しなかった");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// 性質①（非ブロッキング・R1.1/1.2）: 単一 `status()` 呼び出しが 1 秒未満で返る。
    #[test]
    fn status_is_nonblocking_returns_under_one_second() {
        // まだ終了していないかもしれない stand-in でも、status() は try_wait ベースで
        // 即座に返る（稼働中なら Running・終了済みなら Exited のどちらでも可）。
        let handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        let start = Instant::now();
        let _status = lc.status();
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "status() は非ブロッキングで即座に返る（{:?}）",
            start.elapsed()
        );
    }

    /// 分類（Clean・R1.3）: stand-in `cmd /c exit 0` → `Exited(ExitKind::Clean)`。
    #[test]
    fn status_classifies_clean_exit() {
        let handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        assert_eq!(wait_exited(&mut lc), ExitKind::Clean);
    }

    /// 性質②（分類 Abnormal ＋ sticky・R1.4/1.2）: stand-in `cmd /c exit 5` →
    /// `Exited(Abnormal(5))`、以後の再問い合わせも**同値**（再 poll しない sticky）。
    #[test]
    fn status_classifies_abnormal_and_is_sticky() {
        let handle = spawn_command(cmd_exit(5)).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        // 初回検出。
        assert_eq!(wait_exited(&mut lc), ExitKind::Abnormal(5));
        // sticky: 終了検出後の再問い合わせは同じ ExitKind を返し続ける。
        assert_eq!(lc.status(), HelperStatus::Exited(ExitKind::Abnormal(5)));
        assert_eq!(lc.status(), HelperStatus::Exited(ExitKind::Abnormal(5)));
    }

    /// 性質③（冪等後始末・R5.2）: 終了済み stand-in への二重 `terminate()` が両方 `Ok`。
    #[test]
    fn terminate_is_idempotent_after_exit() {
        let handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        // 先に終了を観測してから二重 terminate（既存 HelperHandle::terminate が冪等）。
        let _ = wait_exited(&mut lc);
        assert!(lc.terminate().is_ok(), "1 回目の terminate は Ok");
        assert!(lc.terminate().is_ok(), "2 回目の terminate も Ok（冪等・R5.2）");
    }

    // --- report_failure: request 失敗と死活の突合・統一報告（R2.1〜2.5, 4.4, 7.6）---

    /// 生存中の stand-in を組む（`ping -n 3 127.0.0.1 >NUL` で ~2s 生存）。
    ///
    /// ALIVE 経路（`status() == Running` / `observed_exit == None`）を決定的に行使するため、
    /// `cmd /c exit 0`（spawn 直後に既に死んでいる可能性がある）ではなく十分に長命な
    /// stand-in を使う。`ping` は 1 秒間隔で 3 回試行し、その間プロセスは生存する。
    fn cmd_alive() -> Command {
        let mut command = Command::new("cmd.exe");
        command.args(["/c", "ping", "-n", "3", "127.0.0.1", ">NUL"]);
        command
    }

    /// 性質（生存 → 非死活報告・R2.2/R2.4）: 生存中に `report_failure(Timeout)` を突合すると
    /// `class == Unresponsive`（死活起因でない）となり、原本 `error` が報告に保持される。
    ///
    /// ALIVE 経路の決定性: 長命 stand-in を spawn 直後に突合し、`status()` が `Running`
    /// （`observed_exit == None`）である間に `report_failure` を呼ぶ。生存確認を assert で固定する。
    #[test]
    fn report_failure_alive_timeout_is_unresponsive_and_retains_error() {
        let handle = spawn_command(cmd_alive()).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        // 生存確認: spawn 直後の長命 stand-in はまだ稼働中である（決定的）。
        assert_eq!(lc.status(), HelperStatus::Running, "長命 stand-in は突合時点で生存");

        let report = lc.report_failure(RequestError::Timeout);
        // 生存中の timeout は死活起因でなく Unresponsive（R2.2）。
        assert_eq!(report.class, FailureClass::Unresponsive);
        // 原本 error は単一不透明へ潰さず報告に保持される（R2.4）。
        assert!(
            matches!(report.error, RequestError::Timeout),
            "report は原本 RequestError::Timeout を保持する"
        );

        // 後始末: 長命 stand-in を kill（Drop でも走るが明示）。
        let _ = lc.terminate();
    }

    /// 性質（死優先 → HelperDown・ログ発行経路・R2.1/R2.5/R4.4/R7.6）:
    /// 終了済み stand-in `cmd /c exit 5` に対し `report_failure(Timeout)` を突合すると、
    /// 表面が `Timeout` でも終了検出が優先して `class == HelperDown(Abnormal(5))` となる。
    /// これは `error!` を発行する唯一の分岐であり（death precedence）、原本 error も保持される。
    #[test]
    fn report_failure_dead_abnormal_is_helper_down_precedes_timeout() {
        let handle = spawn_command(cmd_exit(5)).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        // 終了を bounded-poll で観測してから突合（sticky・R1.2）。
        assert_eq!(wait_exited(&mut lc), ExitKind::Abnormal(5));

        let report = lc.report_failure(RequestError::Timeout);
        // 死は死: 表面 Timeout でも HelperDown(Abnormal(5))（R2.1・death precedence）。
        // この分岐が error! を発行する（R4.4/R7.6・戻り値検証で構造的に担保）。
        assert_eq!(report.class, FailureClass::HelperDown(ExitKind::Abnormal(5)));
        // 原本 error は保持（R2.4/R2.5）。
        assert!(
            matches!(report.error, RequestError::Timeout),
            "report は原本 RequestError::Timeout を保持する（死活検出でも潰さない）"
        );
    }

    /// 性質（正常終了死も HelperDown・R2.1）: `cmd /c exit 0`（Clean 終了）に対し
    /// `report_failure(Ipc(..))` を突合すると `HelperDown(Clean)` となる（死は種別を問わず死）。
    #[test]
    fn report_failure_dead_clean_is_helper_down() {
        let handle = spawn_command(cmd_exit(0)).expect("stand-in を spawn できる");
        let mut lc = HelperLifecycle::new(handle);
        assert_eq!(wait_exited(&mut lc), ExitKind::Clean);

        let report = lc.report_failure(RequestError::Ipc(IpcError::SendFailed));
        assert_eq!(report.class, FailureClass::HelperDown(ExitKind::Clean));
        assert!(
            matches!(report.error, RequestError::Ipc(_)),
            "report は原本 RequestError::Ipc を保持する"
        );
    }
}
