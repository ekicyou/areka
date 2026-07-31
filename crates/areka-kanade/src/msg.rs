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

use crate::status::ExecutionStatus;
use crate::talk::EpilogueCommand;

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

/// マウス入力（UI 配線層 → kanade の境界メッセージ・DD-IE 系）。
///
/// UI 配線層が collision resolver の `HitRegion { scope, region }` を destructure して詰める
/// 値オブジェクト（同一性なし）。kanade は `region` を意味解釈せず不透明転写する
/// （[[areka-surface-args-opaque-string-downstream-resolve]] と同精神）。座標は窓 client 物理 px。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseInput {
    /// 対象スコープ（本体 0／相方 1・Ref3 へ転写）。
    pub scope: u32,
    /// ローカル x 座標（Ref0・窓 client 物理 px）。
    pub x: i64,
    /// ローカル y 座標（Ref1・同上）。
    pub y: i64,
    /// 当たり判定名（Ref4・不透明転写・`None`＝判定外）。
    pub region: Option<String>,
    /// イベント種別（移動／ダブルクリック）。
    pub kind: MouseEventKind,
}

/// マウスイベント種別（OnMouseMove／OnMouseDoubleClick に対応）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventKind {
    /// マウス移動（OnMouseMove・Ref5 は常に "0"）。
    Move,
    /// ダブルクリック（OnMouseDoubleClick・Ref5 は左右で分岐）。
    DoubleClick { button: MouseButton },
}

/// マウスボタン識別（Ref5: 左 "0"／右 "1"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
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
    /// マウス入力（移動／ダブルクリック）。Steady でのみ受理され、他フェーズでは安全に
    /// 無視される（横断ルーティングは schedule 層・DD-IE-8）。additive 増分（Req 4.4）。
    Mouse(MouseInput),
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

/// 送出イベント ID（出所カテゴリを型で保持・DD-1）。
///
/// SHIORI へ渡る文字列表現（wire 形）は [`EventId::as_str`]（と同値の [`std::fmt::Display`]）
/// のみが与え、出所カテゴリによって変わらない——カテゴリは egress チョークポイントが
/// **出所別の受理規則**を適用するためだけに存在する。
///
/// - [`EventId::Static`]: スケジューラ起源の固定 ID。`schedule/events.rs`／`schedule/resources.rs`
///   の構築関数のみが構成し、固定表（`ALLOWED_EVENT_IDS`／`ALLOWED_RESOURCE_IDS`）で検証される。
/// - [`EventId::Choice`]: 選択起源の任意名イベント（`\q` の `On` 始まり ID）。ゴースト作者が
///   書いた名前を逐語で運ぶ（事前の固定登録を要さない・Req2.6）。
///
/// # 不変条件
/// [`EventId::Choice`] は選択カスケードの planner のみが構成する（`On` 始まり保証の発生源を
/// 1 点に閉じる）。スケジューラ起源の経路が `Choice` を作ることはない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventId {
    /// スケジューラ起源の固定 ID（構築関数のみが構成・固定表で検証）。
    Static(&'static str),
    /// 選択起源の任意名イベント（逐語・カテゴリ規則で検証）。
    Choice(String),
}

impl EventId {
    /// wire 形（SHIORI へ渡る文字列）を返す。出所カテゴリで表現は変わらない。
    pub fn as_str(&self) -> &str {
        match self {
            EventId::Static(id) => id,
            EventId::Choice(name) => name.as_str(),
        }
    }
}

impl std::fmt::Display for EventId {
    /// [`EventId::as_str`] と同一の逐語表現（wire 形）を書き出す。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// GET と NOTIFY の別を境界越しに保持する（Req 5.2）。
///
/// `status` は全構築点が明示する共通ヘッダ（`Status` 実行状態集合の送出値）である。
/// 構築点が Status を忘れられない構造（events.rs の各構築関数が snapshot から自ら導出する）
/// にすることで、Ref3 と `Status` の不整合を発生源で排除する（DD-IT-3）。
///
/// `id` は出所カテゴリを型で保持する [`EventId`]（DD-1）。SHIORI へ渡る文字列は
/// [`EventId::as_str`] が与える逐語表現のみで、カテゴリは送出可否の判定にのみ用いる。
pub enum ShioriCall {
    Get {
        id: EventId,
        references: Vec<String>,
        status: ExecutionStatus,
    },
    Notify {
        id: EventId,
        references: Vec<String>,
        status: ExecutionStatus,
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

/// 呼出失敗の区別語彙（host32 非依存の再表現・thiserror）。
///
/// 既存4語彙（`Handshake`／`Timeout`／`Ipc`／`Shiori`）は `RequestError` の**境界写像**であり、
/// shiori 境界（`shiori/real.rs` の `map_error`）でのみ構成される。`Internal` は境界写像から
/// **決して生成されない**——kanade 内部で検出した内部規律違反（許可集合外 ID の送出企図等）に
/// 対してのみ kanade 自身が構成する（DD-IT-11）。
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
    /// kanade 内部規律違反（境界写像では生成されない・kanade 内部でのみ構成・DD-IT-11）。
    #[error("kanade internal violation: {0}")]
    Internal(String),
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
    /// 初回起動ゲート（既定 true＝現行挙動不変・値源は ghost boot の BootCount 存在判定）。
    pub first_boot: bool,
    /// OnFirstBoot Reference0 に渡す永続 vanish 回数（既定 0）。
    pub vanish_count: u32,
    /// 初回挨拶トーク末尾へ添付する汎用 epilogue（既定空＝何も添付しない）。
    /// kanade は内容を解釈しない（不透明搬送・sylphya 非依存の維持）。
    pub first_boot_epilogue: Vec<EpilogueCommand>,
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
            first_boot: true,
            vanish_count: 0,
            first_boot_epilogue: Vec::new(),
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
        // マウス入力契約型（Task 1・DD-IE 系）も Send + 'static な所有データのみ。
        assert_send_static::<MouseInput>();
        assert_send_static::<MouseEventKind>();
        assert_send_static::<MouseButton>();
        // 送出イベント ID の出所カテゴリ型（DD-1）も同様。
        assert_send_static::<EventId>();
    }

    /// wire 形（SHIORI へ渡る文字列）は出所カテゴリに依らず逐語であること（DD-1・Req2.6/3.6）。
    ///
    /// `EventId` はチョークポイントが出所別の受理規則を適用するためだけの区別であり、
    /// SHIORI へ出る文字列は `as_str()`（および `Display`）が与える逐語表現のみである。
    #[test]
    fn event_id_as_str_is_verbatim_wire_form_for_both_origins() {
        assert_eq!(EventId::Static("OnBoot").as_str(), "OnBoot");
        assert_eq!(EventId::Static("basewareversion").as_str(), "basewareversion");
        // 選択起源は任意名を逐語で運ぶ（事前登録不要・Req2.6）。
        assert_eq!(
            EventId::Choice("OnMenuBack".to_string()).as_str(),
            "OnMenuBack"
        );
        // 同一綴りなら出所が違っても wire 形は同一（表現の一字一句同一性）。
        assert_eq!(
            EventId::Choice("OnBoot".to_string()).as_str(),
            EventId::Static("OnBoot").as_str()
        );
        // Display は as_str と一致する（`id.to_string()` の既存呼出面が wire 形を保つ）。
        assert_eq!(EventId::Static("OnClose").to_string(), "OnClose");
        assert_eq!(
            EventId::Choice("OnMenuBack".to_string()).to_string(),
            "OnMenuBack"
        );
    }

    /// `ShioriCall` の `id` が [`EventId`] を運び、GET/NOTIFY 双方で wire 形を取り出せる。
    #[test]
    fn shiori_call_carries_event_id_for_both_methods() {
        let get = ShioriCall::Get {
            id: EventId::Static("OnBoot"),
            references: Vec::new(),
            status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
        };
        let notify = ShioriCall::Notify {
            id: EventId::Choice("OnMenuBack".to_string()),
            references: Vec::new(),
            status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
        };
        let wire = |call: &ShioriCall| match call {
            ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id.as_str().to_string(),
        };
        assert_eq!(wire(&get), "OnBoot");
        assert_eq!(wire(&notify), "OnMenuBack");
    }

    #[test]
    fn mouse_input_types_construct_and_compare() {
        // 境界メッセージの形（scope・座標・当たり判定名・種別・ボタン）が構築でき、
        // 値の同一性（PartialEq/Eq）で観測できる。
        let mv = MouseInput {
            scope: 0,
            x: 12,
            y: 34,
            region: Some("head".to_string()),
            kind: MouseEventKind::Move,
        };
        assert_eq!(mv.kind, MouseEventKind::Move);
        assert_eq!(mv.region.as_deref(), Some("head"));

        let dbl_left = MouseInput {
            scope: 1,
            x: -5,
            y: 0,
            region: None,
            kind: MouseEventKind::DoubleClick {
                button: MouseButton::Left,
            },
        };
        assert_eq!(
            dbl_left.kind,
            MouseEventKind::DoubleClick {
                button: MouseButton::Left
            }
        );
        assert_ne!(
            MouseEventKind::DoubleClick {
                button: MouseButton::Left
            },
            MouseEventKind::DoubleClick {
                button: MouseButton::Right
            }
        );
        // KanadeMsg::Mouse variant が MouseInput を運ぶ（additive・Req 4.4）。
        let _ = KanadeMsg::Mouse(mv);
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
        // Internal は境界写像でなく kanade 内部で構成される内部規律違反語彙（DD-IT-11）。
        assert_eq!(
            ShioriFailure::Internal("event_id_not_allowed: OnTalk".to_string()).to_string(),
            "kanade internal violation: event_id_not_allowed: OnTalk"
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
    fn kanade_config_new_defaults_additive_fields() {
        // Task 5.1 / design C8: 追加3フィールドの既定は true / 0 / 空。
        // 既定により既存 boot happy-path 檻は意味論無改変で緑（3.1）。
        let config = KanadeConfig::new("master", "1.0.0");
        assert!(config.first_boot, "first_boot default must be true (現行挙動不変)");
        assert_eq!(config.vanish_count, 0, "vanish_count default must be 0 (OnFirstBoot Ref0)");
        assert!(
            config.first_boot_epilogue.is_empty(),
            "first_boot_epilogue default must be empty (何も添付しない)"
        );
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
                id: EventId::Static("OnBoot"),
                references: vec!["master".to_string()],
                status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
            },
            reply,
        };
        // Request の reply 端を取り出して 1 度だけ応答を送る。
        match msg {
            ShioriMsg::Request { call, reply } => {
                match call {
                    ShioriCall::Get { id, references, .. } => {
                        assert_eq!(id.as_str(), "OnBoot");
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
            id: EventId::Static("OnInitialize"),
            references: Vec::new(),
            status: ExecutionStatus::derive(&crate::status::ExecutionSnapshot::INACTIVE),
        };
    }
}
