//! events — ukadoc Reference 表の実装正本（イベント発火順序の単一正本）。
//!
//! 本モジュールは boot 系列・定常運転・close の各イベントについて、NOTIFY／GET の別と
//! Reference 構成を設計書「ukadoc Reference 表（運行表の単一正本）」どおりに組み立てる
//! **純粋関数群**を提供する（副作用なし・時刻や config を引数で受け取り [`ShioriCall`] を
//! 返すだけ）。mock fixture の期待応答・状態機械の期待列・観測ハーネス（4.1）の assert は
//! すべて本表から導出され、fixture・検証・実装が単一の正本を共有する（Req 7.1・DD-9）。
//!
//! # 公開面（DD-9 の例外）
//! `schedule/` は `pub(crate)` に閉じるが、本モジュールの Reference 表構成関数のみ例外的に
//! `pub` とする。`tests/` 配下の統合テストクレートは `pub(crate)` を参照できないため、
//! ハーネスが同一の値を fixture の期待値として再利用できるよう、これらの関数だけを
//! クレートの公開面（[`crate::events`] 経由）に露出する。運行状態機械の内部実装
//! （Phase／State／Action／Input／step 本体・boot/steady/close の遷移ロジック）は
//! `pub(crate)` のまま非公開である。
//!
//! # ukadoc Reference 表（本モジュールが唯一の実装点）
//! | イベント | Method | M1 送出値 |
//! |---|---|---|
//! | `OnInitialize` | NOTIFY | References なし（空 Vec・M1 にリロード概念なし） |
//! | `OnFirstBoot` | GET | Ref0=`"0"`（固定・Req 1.6） |
//! | `OnBoot` | GET | Ref0=`config.shell_name`（Ref6/7 省略） |
//! | `basewareversion` | NOTIFY | Ref0=`config.baseware_version`・Ref1=`config.baseware_name`（Ref2 省略） |
//! | `OnSecondChange` | GET（talk 再生可能時）／NOTIFY（talk 再生不能時） | Ref0=`now_ms / 3_600_000` の 10 進文字列・Ref1=`"0"`・Ref2=`"0"`・Ref3=`"1"`(GET)/`"0"`(NOTIFY) |
//! | `OnClose` | GET | Ref0=`reason.as_ref_str()`（"user"/"system"・Ref1/2 省略） |

use crate::msg::{CloseReason, KanadeConfig, MonotonicMs, MouseButton, ShioriCall};
use crate::status::{ExecutionSnapshot, ExecutionStatus};

/// `OnSecondChange` Ref0 の除数（ミリ秒→時。正典: OS 連続起動時間 hour）。
const MS_PER_HOUR: u64 = 3_600_000;

/// Ref1（見切れ）の M1 固定値。
// SEAM(Req1.6): 実測供給時は ExecutionSnapshot の geometry から導出する。
const REF1_OFFSCREEN_M1: &str = "0";
/// Ref2（重なり）の M1 固定値。
// SEAM(Req1.6): 実測供給時は ExecutionSnapshot の geometry から導出する。
const REF2_OVERLAP_M1: &str = "0";

/// OnMouseMove Ref2（ホイール回転量）の M1 固定値＝increment シーム（Req2.4）。
///
/// M1 はホイールイベントを送出しないため常に "0" で構成する。実ホイール量の供給が
/// 実装されたら、この定数の載せ替え（呼び手が実値を渡す形へ）で拡張する。
pub(crate) const REF2_WHEEL_M1: &str = "0";
/// マウス系イベント Ref6（入力デバイス種）の M1 固定値（DD-IE-6）。
///
/// M1 は物理マウスのみを対象とするため常に "mouse"。touch/pen/eraser の区別は
/// M2 のシーム（呼び手がデバイス種を渡す形へ）として残す。
pub(crate) const REF6_DEVICE_MOUSE: &str = "mouse";

/// 送出し得るイベント ID の確定ホワイトリスト（Req3.1）。
/// `OnTalk`／`OnHour` は emo2 が OnSecondChange 内部で自発生成するため**恒久的に含めない**（Req3.2）。
///
/// SEAM(W5・choice-select-events): `\q[タイトル,OnID]` は実行時スクリプト由来の**任意名イベント**を
/// 発火する（emo2 唯一の依存形・menu.pasta:15 実物）＝固定 const 表にも `&'static str` の id にも載らない。
/// W5 での拡張は本表への ID 追加ではなく**受理規則へのカテゴリ追加**（additive）で行う——チョークポイント・
/// 本表（正典固定 ID の部分集合）・`OnTalk`/`OnHour` 恒久禁止は不変。任意名カテゴリと Req3.2 恒久禁止の
/// 交差（`\q[x,OnTalk]` を書くゴーストの扱い）は choice-select-events の要件フェーズで決着（research §16 申し送り）。
pub const ALLOWED_EVENT_IDS: &[&str] = &[
    "OnInitialize",
    "OnFirstBoot",
    "OnBoot",
    "basewareversion",
    "OnSecondChange",
    "OnClose",
    "OnMouseMove",
    "OnMouseDoubleClick",
];

/// `id` が送出許可集合（[`ALLOWED_EVENT_IDS`]）に属するかを判定する（Req3.1）。
pub fn is_allowed_event_id(id: &str) -> bool {
    ALLOWED_EVENT_IDS.contains(&id)
}

/// `OnInitialize`（NOTIFY・References なし）。
///
/// M1 にリロード概念がないため Ref0（正典: リロード時 `reload`）は送出せず空 Vec とする。
pub fn on_initialize(snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Notify {
        id: "OnInitialize",
        references: Vec::new(),
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnFirstBoot`（GET・Ref0=`"0"` 固定）。
///
/// M1 は vanish count 等の永続値を持たない（永続化は position-persist の領分）ため、
/// 起動種別イベントは毎回同一の運行として Ref0 を固定値 `"0"` で構成する（Req 1.6）。
/// この応答が 204 であれば呼び手はフォールスルーして [`on_boot`] へ進む。
pub fn on_first_boot(snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: "OnFirstBoot",
        references: vec!["0".to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnBoot`（GET・Ref0=`config.shell_name`）。
///
/// Ref6/7（前回 crash 情報・MATERIA/SSP）は crash 情報を持たない M1 では省略する。
pub fn on_boot(config: &KanadeConfig, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: "OnBoot",
        references: vec![config.shell_name.clone()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `basewareversion`（NOTIFY・Ref0=バージョン／Ref1=本体識別）。
///
/// Ref2（詳細数値・SSP のみ）は省略する。
pub fn baseware_version(config: &KanadeConfig, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Notify {
        id: "basewareversion",
        references: vec![
            config.baseware_version.clone(),
            config.baseware_name.clone(),
        ],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnSecondChange`（talk 再生可能なら GET・不能なら NOTIFY）。
///
/// GET/NOTIFY の別・Ref3・`Status.talking` を**単一の [`ExecutionSnapshot`] から**導出する
/// （DD-IT-3）。`talk_playable = !snapshot.talk_active` ゆえ「Ref3=`"1"` かつ `Status: talking`」
/// という不整合の組み合わせは構造的に発生しない。
///
/// - Ref0: `now_ms / 3_600_000` の 10 進文字列（正典: OS 連続起動時間 hour）。
/// - Ref1: [`REF1_OFFSCREEN_M1`]（見切れ・emo 領分ゆえ M1 固定）。
/// - Ref2: [`REF2_OVERLAP_M1`]（重なり・同上）。
/// - Ref3: talk 再生可否——`talk_playable==true` なら `"1"`（GET・status 空）、`false` なら
///   `"0"`（NOTIFY・status `talking`）。
///
/// DD-6: talk 再生中（再生不能時）は NOTIFY（Ref3=0）で発行し、返却スクリプトは構造的に
/// 破棄される（active talk 中に Value が届く経路を発生源から塞ぐ）。
pub fn on_second_change(now: MonotonicMs, snapshot: &ExecutionSnapshot) -> ShioriCall {
    let talk_playable = !snapshot.talk_active;
    let hours = (now.0 / MS_PER_HOUR).to_string();
    let ref3 = if talk_playable { "1" } else { "0" };
    let references = vec![
        hours,
        REF1_OFFSCREEN_M1.to_string(),
        REF2_OVERLAP_M1.to_string(),
        ref3.to_string(),
    ];
    let status = ExecutionStatus::derive(snapshot);
    if talk_playable {
        ShioriCall::Get {
            id: "OnSecondChange",
            references,
            status,
        }
    } else {
        ShioriCall::Notify {
            id: "OnSecondChange",
            references,
            status,
        }
    }
}

/// `OnClose`（GET・Ref0=`reason.as_ref_str()`）。
///
/// Ref1/2（スコープ番号・SSP）は単一スコープの M1 では省略する。
pub fn on_close(reason: CloseReason, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: "OnClose",
        references: vec![reason.as_ref_str().to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnClose`（**NOTIFY**・ForceQuit の best-effort 通知・Ref0=`reason.as_ref_str()`）。
///
/// DD-IT-8: `mod.rs` の `force_quit` が inline 構築していた退化 NOTIFY を置き換え、events.rs を
/// `ShioriCall` 構築の単一列挙点へ回復する。通常握手の [`on_close`] は **GET** を返すため
/// force_quit には流用できず（force_quit は NOTIFY を要する）、NOTIFY 版を別に増設する。
/// snapshot は Unloading へ遷移後の [`ExecutionSnapshot::INACTIVE`] を渡す（DD-IT-4）。
pub fn on_close_notify(reason: CloseReason, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Notify {
        id: "OnClose",
        references: vec![reason.as_ref_str().to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnMouseMove`（GET・Ref0..6 正典 layout）。
///
/// 撫で入力を SSP/NINIX 準拠の正典 Reference layout で組み立てる純粋関数（副作用なし）。
/// Reference 数は常に 7。
///
/// - Ref0=`x`（ローカル x 座標・窓 client 物理 px）。
/// - Ref1=`y`（ローカル y 座標・同上）。
/// - Ref2=[`REF2_WHEEL_M1`]（ホイール回転量・M1 固定 "0"・Req2.4）。
/// - Ref3=`scope`（対象スコープ・本体 0／相方 1）。
/// - Ref4=`region`（当たり判定の識別子・不透明転写・`None`→空文字・Req2.3/DD-IE-6）。
/// - Ref5=`"0"`（移動はボタン押下を伴わないため常に "0"・Req2.5）。
/// - Ref6=[`REF6_DEVICE_MOUSE`]（入力デバイス種・M1 固定 "mouse"・DD-IE-6）。
///
/// `region` は collision resolver 由来の領域名を意味解釈せず不透明転写する（kanade は
/// 当たり判定名を解釈しない・[[areka-surface-args-opaque-string-downstream-resolve]] と同精神）。
pub fn on_mouse_move(
    x: i64,
    y: i64,
    scope: u32,
    region: Option<&str>,
    snapshot: &ExecutionSnapshot,
) -> ShioriCall {
    ShioriCall::Get {
        id: "OnMouseMove",
        references: vec![
            x.to_string(),
            y.to_string(),
            REF2_WHEEL_M1.to_string(),
            scope.to_string(),
            region.unwrap_or("").to_string(),
            "0".to_string(),
            REF6_DEVICE_MOUSE.to_string(),
        ],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnMouseDoubleClick`（GET・Ref0..6 正典 layout）。
///
/// ダブルクリック入力を SSP/NINIX 準拠の正典 Reference layout で組み立てる純粋関数
/// （副作用なし）。Reference 数は常に 7。[`on_mouse_move`] と同一の Reference 構造で、
/// Ref2 が常に "0"・Ref5 がボタン識別（左 "0"／右 "1"・Req3.3）である点のみ異なる。
///
/// - Ref0=`x`／Ref1=`y`（座標・窓 client 物理 px）。
/// - Ref2=`"0"`（正典で常に "0"・Req3.2）。
/// - Ref3=`scope`（対象スコープ・本体 0／相方 1）。
/// - Ref4=`region`（当たり判定の識別子・不透明転写・`None`→空文字・Req3.4/DD-IE-6）。
/// - Ref5=`button`（左 [`MouseButton::Left`]→"0"／右 [`MouseButton::Right`]→"1"・Req3.3）。
/// - Ref6=[`REF6_DEVICE_MOUSE`]（入力デバイス種・M1 固定 "mouse"・DD-IE-6）。
pub fn on_mouse_double_click(
    x: i64,
    y: i64,
    scope: u32,
    region: Option<&str>,
    button: MouseButton,
    snapshot: &ExecutionSnapshot,
) -> ShioriCall {
    let button_ref5 = match button {
        MouseButton::Left => "0",
        MouseButton::Right => "1",
    };
    ShioriCall::Get {
        id: "OnMouseDoubleClick",
        references: vec![
            x.to_string(),
            y.to_string(),
            "0".to_string(),
            scope.to_string(),
            region.unwrap_or("").to_string(),
            button_ref5.to_string(),
            REF6_DEVICE_MOUSE.to_string(),
        ],
        status: ExecutionStatus::derive(snapshot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// GET variant を分解して (id, references) を取り出す（Notify なら panic）。
    fn expect_get(call: ShioriCall) -> (&'static str, Vec<String>) {
        match call {
            ShioriCall::Get { id, references, .. } => (id, references),
            ShioriCall::Notify { .. } => panic!("expected GET, got NOTIFY"),
        }
    }

    /// NOTIFY variant を分解して (id, references) を取り出す（Get なら panic）。
    fn expect_notify(call: ShioriCall) -> (&'static str, Vec<String>) {
        match call {
            ShioriCall::Notify { id, references, .. } => (id, references),
            ShioriCall::Get { .. } => panic!("expected NOTIFY, got GET"),
        }
    }

    /// 呼出の `id` を GET/NOTIFY 不問で取り出す（許可集合檻の被覆確認用）。
    fn call_id(call: &ShioriCall) -> &'static str {
        match call {
            ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id,
        }
    }

    /// 呼出の `status` を render した wire 値（`None` ⇔ ヘッダ行なし）を取り出す。
    fn call_status(call: &ShioriCall) -> Option<String> {
        match call {
            ShioriCall::Get { status, .. } | ShioriCall::Notify { status, .. } => status.render(),
        }
    }

    #[test]
    fn on_initialize_is_notify_with_empty_references() {
        let (id, references) = expect_notify(on_initialize(&ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnInitialize");
        assert!(references.is_empty());
    }

    #[test]
    fn on_first_boot_is_get_with_fixed_zero_ref0() {
        let (id, references) = expect_get(on_first_boot(&ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnFirstBoot");
        assert_eq!(references, vec!["0".to_string()]);
    }

    #[test]
    fn on_boot_is_get_with_shell_name_ref0() {
        let (id, references) = expect_get(on_boot(&config(), &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnBoot");
        assert_eq!(references, vec!["master".to_string()]);
    }

    #[test]
    fn baseware_version_is_notify_with_version_and_name() {
        let (id, references) =
            expect_notify(baseware_version(&config(), &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "basewareversion");
        assert_eq!(references, vec!["1.0.0".to_string(), "areka".to_string()]);
    }

    #[test]
    fn on_second_change_playable_is_get_ref3_one() {
        // 7_200_000 ms = 2 hours。talk_active=false（再生可能）→ GET・Ref3=1・status 空（DD-IT-3）。
        let call = on_second_change(MonotonicMs(7_200_000), &ExecutionSnapshot { talk_active: false });
        assert_eq!(
            call_status(&call),
            None,
            "再生可能時（talk 非アクティブ）は Status ヘッダを出さない（DD-IT-3/DD-IT-5）"
        );
        let (id, references) = expect_get(call);
        assert_eq!(id, "OnSecondChange");
        assert_eq!(
            references,
            vec![
                "2".to_string(),
                "0".to_string(),
                "0".to_string(),
                "1".to_string(),
            ]
        );
    }

    #[test]
    fn on_second_change_not_playable_is_notify_ref3_zero() {
        // 3_600_000 ms = 1 hour。talk_active=true（再生中）→ NOTIFY・Ref3=0・status talking（DD-IT-3）。
        let call = on_second_change(MonotonicMs(3_600_000), &ExecutionSnapshot { talk_active: true });
        assert_eq!(
            call_status(&call),
            Some("talking".to_string()),
            "再生中は Ref3=0 と Status: talking が同一スナップショットから出る（DD-IT-3）"
        );
        let (id, references) = expect_notify(call);
        assert_eq!(id, "OnSecondChange");
        assert_eq!(
            references,
            vec![
                "1".to_string(),
                "0".to_string(),
                "0".to_string(),
                "0".to_string(),
            ]
        );
    }

    #[test]
    fn on_second_change_ref0_truncates_toward_zero() {
        // 端数（1 時間未満）は切り捨てて "0"（3_599_999 ms < 1 hour）。
        let (_, references) = expect_get(on_second_change(
            MonotonicMs(3_599_999),
            &ExecutionSnapshot { talk_active: false },
        ));
        assert_eq!(references[0], "0");
    }

    #[test]
    fn on_close_user_maps_to_user() {
        let (id, references) = expect_get(on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnClose");
        assert_eq!(references, vec!["user".to_string()]);
    }

    #[test]
    fn on_close_system_maps_to_system() {
        let (id, references) =
            expect_get(on_close(CloseReason::System, &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnClose");
        assert_eq!(references, vec!["system".to_string()]);
    }

    /// `on_close_notify`（DD-IT-8）: NOTIFY・Ref0=reason・status は snapshot 由来（INACTIVE→None）。
    #[test]
    fn on_close_notify_is_notify_with_reason_and_derived_status() {
        let call = on_close_notify(CloseReason::System, &ExecutionSnapshot::INACTIVE);
        assert_eq!(
            call_status(&call),
            None,
            "INACTIVE スナップショット（Unloading 遷移後・DD-IT-4）は Status ヘッダを出さない"
        );
        let (id, references) = expect_notify(call);
        assert_eq!(id, "OnClose");
        assert_eq!(references, vec!["system".to_string()]);
    }

    /// 許可 ID 檻（Req3.1/3.2/7.1・DD-IT-8・DD-IE-11）: 表が期待8集合と完全一致し
    /// `OnTalk`/`OnHour` を含まない。マウス系2種（OnMouseMove/OnMouseDoubleClick）は
    /// Task 2.1 で additive 追加された（whitelist が意図的に6→8へ増えたための更新）。
    #[test]
    fn allowed_event_ids_are_exactly_the_eight_and_exclude_ontalk_onhour() {
        assert_eq!(
            ALLOWED_EVENT_IDS,
            &[
                "OnInitialize",
                "OnFirstBoot",
                "OnBoot",
                "basewareversion",
                "OnSecondChange",
                "OnClose",
                "OnMouseMove",
                "OnMouseDoubleClick",
            ]
        );
        assert!(is_allowed_event_id("OnMouseMove"), "OnMouseMove は許可集合に属する（Req7.1）");
        assert!(
            is_allowed_event_id("OnMouseDoubleClick"),
            "OnMouseDoubleClick は許可集合に属する（Req7.1）"
        );
        assert!(!is_allowed_event_id("OnTalk"), "OnTalk は恒久的に許可しない（Req3.2）");
        assert!(!is_allowed_event_id("OnHour"), "OnHour は恒久的に許可しない（Req3.2）");
        // 表の全要素が許可判定を通ること。
        for id in ALLOWED_EVENT_IDS {
            assert!(is_allowed_event_id(id), "{id} は表にあるのに許可されない");
        }
    }

    /// OnMouseMove 正典 layout（Req2.1/2.2/2.5・DD-IE-6）: References が期待7並びと完全一致。
    #[test]
    fn on_mouse_move_builds_canonical_seven_reference_layout() {
        let call = on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot::INACTIVE);
        assert_eq!(
            call_status(&call),
            None,
            "INACTIVE スナップショットは Status ヘッダを出さない"
        );
        let (id, references) = expect_get(call);
        assert_eq!(id, "OnMouseMove");
        assert_eq!(
            references,
            vec![
                "10".to_string(),   // Ref0=x
                "20".to_string(),   // Ref1=y
                "0".to_string(),    // Ref2=wheel（M1 固定・Req2.4）
                "0".to_string(),    // Ref3=scope（本体0）
                "Head".to_string(), // Ref4=region（不透明転写）
                "0".to_string(),    // Ref5=移動は常に "0"（Req2.5）
                "mouse".to_string(),// Ref6=デバイス種（DD-IE-6）
            ]
        );
        assert_eq!(references.len(), 7, "Reference 数は常に 7");
    }

    /// Ref4 の None→空文字転写（Req2.3・DD-IE-6）: 位置は保持され Vec 長は 7 のまま。
    #[test]
    fn on_mouse_move_region_none_transcribes_to_empty_ref4() {
        let (_, references) = expect_get(on_mouse_move(1, 2, 1, None, &ExecutionSnapshot::INACTIVE));
        assert_eq!(references[4], "", "None は空文字へ転写（省略ではない）");
        assert_eq!(references[3], "1", "Ref3=scope 相方は 1");
        assert_eq!(references.len(), 7, "None でも Reference 数は 7 のまま");
    }

    /// OnMouseDoubleClick 正典 layout・左ボタン（Req3.1/3.2/3.3）: Ref5="0"・Ref2="0"・Ref6="mouse"。
    #[test]
    fn on_mouse_double_click_left_builds_ref5_zero() {
        let call = on_mouse_double_click(
            10,
            20,
            0,
            Some("Bust"),
            MouseButton::Left,
            &ExecutionSnapshot::INACTIVE,
        );
        let (id, references) = expect_get(call);
        assert_eq!(id, "OnMouseDoubleClick");
        assert_eq!(
            references,
            vec![
                "10".to_string(),
                "20".to_string(),
                "0".to_string(), // Ref2=常に "0"（Req3.2）
                "0".to_string(),
                "Bust".to_string(),
                "0".to_string(), // Ref5=左 "0"（Req3.3）
                "mouse".to_string(),
            ]
        );
    }

    /// OnMouseDoubleClick 右ボタン（Req3.3）: Ref5="1"。
    #[test]
    fn on_mouse_double_click_right_builds_ref5_one() {
        let (_, references) = expect_get(on_mouse_double_click(
            -5,
            0,
            1,
            None,
            MouseButton::Right,
            &ExecutionSnapshot::INACTIVE,
        ));
        assert_eq!(references[5], "1", "右ボタンは Ref5 \"1\"（Req3.3）");
        assert_eq!(references[2], "0", "Ref2 は常に \"0\"（Req3.2）");
        assert_eq!(references[4], "", "Ref4 None→空文字（Req3.4）");
        assert_eq!(references[6], "mouse", "Ref6=デバイス種");
        assert_eq!(references.len(), 7);
    }

    /// talk_active=true では両構築子が `Status: talking` を snapshot から導出する（DD-IT-3）。
    #[test]
    fn mouse_constructors_carry_talking_status_when_active() {
        let active = ExecutionSnapshot { talk_active: true };
        let mv = on_mouse_move(0, 0, 0, Some("Head"), &active);
        assert_eq!(call_status(&mv), Some("talking".to_string()));
        let dbl = on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &active);
        assert_eq!(call_status(&dbl), Some("talking".to_string()));
    }

    /// 全構築関数の返す `id` が許可集合の要素であること（Service Interface Postcondition）。
    #[test]
    fn every_construction_function_returns_an_allowed_id() {
        let cfg = config();
        let snap = ExecutionSnapshot::INACTIVE;
        let calls = [
            on_initialize(&snap),
            on_first_boot(&snap),
            on_boot(&cfg, &snap),
            baseware_version(&cfg, &snap),
            on_second_change(MonotonicMs(0), &snap),
            on_second_change(MonotonicMs(0), &ExecutionSnapshot { talk_active: true }),
            on_close(CloseReason::User, &snap),
            on_close_notify(CloseReason::System, &snap),
            on_mouse_move(0, 0, 0, Some("Head"), &snap),
            on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &snap),
        ];
        for call in &calls {
            let id = call_id(call);
            assert!(
                is_allowed_event_id(id),
                "構築関数が許可集合外の id={id} を返した"
            );
        }
    }
}
