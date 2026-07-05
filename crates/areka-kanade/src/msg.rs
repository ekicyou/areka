//! kanade inbox・shiori 呼出境界の全メッセージ型と運行構成型（`src/msg.rs`）。
//!
//! 本モジュールは kanade アクター inbox（[`KanadeMsg`]）と shiori 呼出境界
//! （[`ShioriMsg`] / [`ShioriCall`] / [`ShioriOutcome`]）、および運行構成
//! （[`KanadeConfig`]）を定義する。すべて `Send + 'static` な所有データのみで
//! 構成される（envelope 規約・Req 3.2）。
//!
//! # 依存規律（Allowed Dependencies）
//! 本ファイルは `std`・[`crate::talk`]・**`areka_actor::ReplySender` のみ**に
//! 依存する。`shiori-host32-host`（`RequestError` 等）は一切 import しない
//! ——[`ShioriFailure`] は host32 非依存の**再表現**（`String` 保持）であり
//! `RequestError` の re-export ではない。host32 型は `shiori/real.rs`（後続
//! タスク）に封じ込め、この境界型は差し替え可能な mock/real の共通面となる
//! （Req 5.1）。
//!
//! # GET／NOTIFY の型区別（Req 5.2）
//! [`ShioriCall`] が GET と NOTIFY を型で区別し、[`ShioriOutcome`] は NOTIFY
//! 完了を [`ShioriOutcome::Notified`] として表す——ここに `Value` を運ぶ経路が
//! 存在しないため、NOTIFY 応答から talk を生成できないことが構造的に保証される。

/// 単調ミリ秒（注入時刻）。本番結線は OS 起動からの経過 ms（GetTickCount64 相当）を
/// 注入する想定（OnSecondChange Ref0 が正典と一致する）。テストは任意の単調値。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonotonicMs(pub u64);

/// close 指示の理由（ukadoc OnClose Ref0 への写像: User→"user"・System→"system"）。
#[derive(Debug, Clone, Copy)]
pub enum CloseReason {
    User,
    System,
}

impl CloseReason {
    /// ukadoc OnClose Ref0 への写像文字列（`User`→`"user"`・`System`→`"system"`）。
    pub fn as_ref_str(self) -> &'static str {
        match self {
            CloseReason::User => "user",
            CloseReason::System => "system",
        }
    }
}

/// kanade アクター inbox（inbox 規約: 1 アクター 1 enum）。
/// shiori 応答は inbox を経由しない（oneshot 往復・DD-2）ため variant を持たない＝
/// 外部から偽の SHIORI 応答を注入できない構造。
pub enum KanadeMsg {
    /// boot 運行の開始指示（Idle 以外では warn!＋無視）。
    Boot,
    /// 1 秒相当の Tick（時刻同梱・DD-3）。pump 駆動と close 期限判定を兼ねる。
    Tick { now: MonotonicMs },
    /// sakura（mock sink 含む）からの再生完了通知。
    TalkDone(crate::talk::TalkDone),
    /// 通常 close 指示（OnClose 握手を開始・終了権限は SHIORI 側）。
    CloseRequest { reason: CloseReason },
    /// 強制終了指示（OS シャットダウン・デバッグ）。quit ゲートを迂回し終了系列へ直行。
    ForceQuit { reason: CloseReason },
    /// SHIORI 死活の暫定 seam（DD-4・lifecycle 正本確定時に実型へ差し替え）。
    ShioriDown { reason: String },
    /// 停止規約の Close（即時停止・非常口。正規終了は運行表経由）。
    Close,
}

/// shiori アクター inbox（real／mock が同一型を受ける＝Req 5.1 の差し替え面）。
/// envelope 規約どおり返信端（oneshot）を同梱する。受信側は 1 度だけ応答を送る。
pub enum ShioriMsg {
    /// GET／NOTIFY の 1 呼出。
    Request {
        call: ShioriCall,
        reply: areka_actor::ReplySender<ShioriOutcome>,
    },
    /// 正規終了経路（unload）の起動。完了で `Unloaded`（失敗は `Failed`）を返す。
    Unload {
        reply: areka_actor::ReplySender<ShioriOutcome>,
    },
    /// 停止規約の Close（即時停止）。
    Close,
}

/// GET と NOTIFY の別を境界越しに保持する（Req 5.2）。
pub enum ShioriCall {
    Get {
        id: &'static str,
        references: Vec<String>,
    },
    Notify {
        id: &'static str,
        references: Vec<String>,
    },
}

/// shiori 呼出の結果（NOTIFY は Value を運べない＝talk 非生成の構造的保証）。
pub enum ShioriOutcome {
    /// GET 200: Value（スクリプト文字列・不透明）。
    Value(String),
    /// GET 204: Value なし。
    NoContent,
    /// NOTIFY 完了（応答は破棄済み）。
    Notified,
    /// Unload 完了。
    Unloaded,
    /// 呼出失敗（区別語彙保持・Req 6.1）。
    Failed(ShioriFailure),
}

/// RequestError の区別語彙の境界写像（host32 非依存の再表現・thiserror）。
#[derive(Debug, thiserror::Error)]
pub enum ShioriFailure {
    /// 接続確立失敗。
    #[error("shiori handshake failure: {0}")]
    Handshake(String),
    /// タイムアウト。
    #[error("shiori request timeout: {0}")]
    Timeout(String),
    /// helper 死活の一態様。
    #[error("shiori ipc failure: {0}")]
    Ipc(String),
    /// SHIORI エラー。
    #[error("shiori error response: {0}")]
    Shiori(String),
}

/// 運行構成（結線側が供給。既定値は [`KanadeConfig::new`] で提供）。
pub struct KanadeConfig {
    /// OnBoot Ref0（package-mount 由来・ハーネスは "master"）。
    pub shell_name: String,
    /// basewareversion Ref0。
    pub baseware_version: String,
    /// basewareversion Ref1（既定 "areka"）。
    pub baseware_name: String,
    /// close talk 再生完了待ち上限（既定 30_000・DD 表参照）。
    pub close_talk_deadline_ms: u64,
}

impl KanadeConfig {
    /// 既定値つき構成を生成する。
    ///
    /// `baseware_name` は `"areka"`・`close_talk_deadline_ms` は `30_000`（DD 表）。
    /// `shell_name` / `baseware_version` は結線側固有ゆえ引数で受ける。
    pub fn new(shell_name: impl Into<String>, baseware_version: impl Into<String>) -> Self {
        KanadeConfig {
            shell_name: shell_name.into(),
            baseware_version: baseware_version.into(),
            baseware_name: "areka".to_string(),
            close_talk_deadline_ms: 30_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Send + 'static` を型検査で強制する静的アサーション。
    fn assert_send_static<T: Send + 'static>() {}

    #[test]
    fn kanade_and_shiori_msgs_are_send_static() {
        // 観測可能な完了条件: メッセージ型が Send + 'static な所有データのみで構成される。
        assert_send_static::<KanadeMsg>();
        assert_send_static::<ShioriMsg>();
        assert_send_static::<ShioriCall>();
        assert_send_static::<ShioriOutcome>();
        assert_send_static::<ShioriFailure>();
        assert_send_static::<KanadeConfig>();
        assert_send_static::<CloseReason>();
        assert_send_static::<MonotonicMs>();
    }

    #[test]
    fn shiori_failure_display_matches_vocabulary() {
        // 区別語彙（接続確立失敗／タイムアウト／helper 死活／SHIORI エラー）の Display。
        assert_eq!(
            ShioriFailure::Handshake("boom".to_string()).to_string(),
            "shiori handshake failure: boom"
        );
        assert_eq!(
            ShioriFailure::Timeout("30s".to_string()).to_string(),
            "shiori request timeout: 30s"
        );
        assert_eq!(
            ShioriFailure::Ipc("pipe closed".to_string()).to_string(),
            "shiori ipc failure: pipe closed"
        );
        assert_eq!(
            ShioriFailure::Shiori("400".to_string()).to_string(),
            "shiori error response: 400"
        );
    }

    #[test]
    fn close_reason_maps_to_onclose_ref0() {
        assert_eq!(CloseReason::User.as_ref_str(), "user");
        assert_eq!(CloseReason::System.as_ref_str(), "system");
    }

    #[test]
    fn kanade_config_new_supplies_documented_defaults() {
        let config = KanadeConfig::new("master", "1.0.0");
        assert_eq!(config.shell_name, "master");
        assert_eq!(config.baseware_version, "1.0.0");
        assert_eq!(config.baseware_name, "areka");
        assert_eq!(config.close_talk_deadline_ms, 30_000);
    }

    #[test]
    fn monotonic_ms_orders_by_inner_value() {
        assert!(MonotonicMs(1) < MonotonicMs(2));
        assert_eq!(MonotonicMs(5), MonotonicMs(5));
    }

    #[test]
    fn shiori_request_envelope_wires_and_outcome_variants_exist() {
        // envelope 結線（reply_channel）が型検査を通り、応答端で全 outcome を送れる。
        let (reply, receiver) = areka_actor::reply_channel::<ShioriOutcome>();
        let msg = ShioriMsg::Request {
            call: ShioriCall::Get {
                id: "OnBoot",
                references: vec!["master".to_string()],
            },
            reply,
        };
        // Request の reply 端を取り出して 1 度だけ応答を送る。
        match msg {
            ShioriMsg::Request { call, reply } => {
                match call {
                    ShioriCall::Get { id, references } => {
                        assert_eq!(id, "OnBoot");
                        assert_eq!(references, vec!["master".to_string()]);
                    }
                    ShioriCall::Notify { .. } => unreachable!("constructed Get"),
                }
                let _ = reply.send(ShioriOutcome::Value(r"\0hi\e".to_string()));
            }
            _ => unreachable!("constructed Request"),
        }
        // 受信側で Value を観測できる。
        match receiver.recv() {
            Ok(ShioriOutcome::Value(script)) => assert_eq!(script, r"\0hi\e"),
            Ok(_) => unreachable!("expected Value outcome"),
            Err(_) => unreachable!("reply should have been sent"),
        }

        // 残りの outcome variant が構築可能であることを型検査で確認。
        let _ = ShioriOutcome::NoContent;
        let _ = ShioriOutcome::Notified;
        let _ = ShioriOutcome::Unloaded;
        let _ = ShioriOutcome::Failed(ShioriFailure::Timeout("x".to_string()));

        // Notify 呼出も構築できる。
        let _notify = ShioriCall::Notify {
            id: "OnInitialize",
            references: Vec::new(),
        };
    }
}
