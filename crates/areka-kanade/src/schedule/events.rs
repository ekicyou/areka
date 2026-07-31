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
//! | `OnChoiceSelectEx` | GET | Ref0=ラベル・Ref1=選択肢 ID・Ref2 以降=付随参照列（空なら位置なし・Req3.1/3.5） |
//! | `OnChoiceSelect` | GET | Ref0=選択肢 ID（Req3.2） |
//! | 任意名（`\q` の `On` 始まり ID） | GET | Ref0 以降=付随参照列のみ（空なら References なし・Req3.3/3.5） |
//! | `OnChoiceTimeout` | GET | Ref0=タイムアウトした選択肢を含むトークの起動スクリプト（Req3.4） |

use crate::msg::{CloseReason, EventId, KanadeConfig, MonotonicMs, MouseButton, ShioriCall};
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

/// **スケジューラ起源**（[`crate::msg::EventId::Static`]）で送出し得るイベント ID の
/// 確定ホワイトリスト（Req3.1）。
///
/// 本表は**正典（ukadoc）固定 ID の部分集合**である——載る ID はすべて正典に実在する固定名で、
/// 実行時に生成される任意名は 1 つも載らない（この性質は表への追加が起きても不変）。
/// `OnTalk`／`OnHour` は emo2 が OnSecondChange 内部で自発生成するため**恒久的に含めない**（Req3.2）。
///
/// 選択関連の固定 3 ID（`OnChoiceSelectEx`／`OnChoiceSelect`／`OnChoiceTimeout`）は
/// choice-select-events（DD-2）で additive 追加した——いずれも正典固定 ID であり、マウス系 2 種の
/// 追加と同じ前例に従う。一方 `\q[タイトル,OnID]` 由来の**任意名イベント**（emo2 唯一の依存形・
/// menu.pasta:15 実物）は固定 const 表には載らず、[`is_allowed_choice_event`] という
/// **出所別の受理規則**で検証する（DD-1／DD-2・id 型は [`crate::msg::EventId::Choice`] が
/// 任意名を逐語で運ぶ）。ゆえに本表の恒久禁止（`OnTalk`／`OnHour`）は選択起源へ波及しない
/// （禁止の根拠＝自発生成との二重駆動は、作者が明示的に書いた 1 クリック 1 回の発火に該当しない・
/// Req2.9・裁定 8）。
pub const ALLOWED_EVENT_IDS: &[&str] = &[
    "OnInitialize",
    "OnFirstBoot",
    "OnBoot",
    "basewareversion",
    "OnSecondChange",
    "OnClose",
    "OnMouseMove",
    "OnMouseDoubleClick",
    "OnChoiceSelectEx",
    "OnChoiceSelect",
    "OnChoiceTimeout",
];

/// `id` が送出許可集合（[`ALLOWED_EVENT_IDS`]）に属するかを判定する（Req3.1）。
///
/// **スケジューラ起源専用**の判定である。選択起源（[`crate::msg::EventId::Choice`]）は
/// 本判定ではなく [`is_allowed_choice_event`] を用いる（DD-2）。
pub fn is_allowed_event_id(id: &str) -> bool {
    ALLOWED_EVENT_IDS.contains(&id)
}

/// **選択起源**（[`crate::msg::EventId::Choice`]）の任意名イベント受理規則（Req2.6／2.9・DD-2）。
///
/// ゴースト作者が `\q` の ID に書いた名前を**事前の固定登録なしに逐語で**発火するため、判定は
/// 「`On` 接頭であること」ただ 1 条件とする（[`ALLOWED_EVENT_IDS`] への登録は要求しない）。
///
/// スケジューラ起源の恒久禁止（`OnTalk`／`OnHour`）は本規則へ**適用しない**——禁止の根拠は
/// 「ベースウェアが自発的に周期発火すると消費側ゴーストの自発生成と二重駆動する」ことであり、
/// 作者が選択肢へ明示的に書いた 1 クリック = 1 回の発火はこの根拠に該当しないため、
/// `OnTalk` も選択起源なら発火できる（Req2.9・裁定 8）。
///
/// 大小文字の揺れ（`on`／`ONMENU`）は正典の書式ではないため補正せず拒否する（逐語判定）。
pub fn is_allowed_choice_event(id: &str) -> bool {
    id.starts_with("On")
}

/// `OnInitialize`（NOTIFY・References なし）。
///
/// M1 にリロード概念がないため Ref0（正典: リロード時 `reload`）は送出せず空 Vec とする。
pub fn on_initialize(snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Notify {
        id: EventId::Static("OnInitialize"),
        references: Vec::new(),
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnFirstBoot`（GET・Ref0=`vanish_count` 由来）。
///
/// Ref0 は呼び手が渡す vanish 回数（永続状態由来・値なしは `0`）をそのまま 10 進文字列化して
/// 構成する（Req 4.1／4.2）。永続状態に記録が無い場合や M1（vanish 発生源が未実装）では
/// 呼び手が `0` を渡すため、従来の固定値 `"0"` と同値の運行に縮退する。
/// この応答が 204 であれば呼び手はフォールスルーして [`on_boot`] へ進む。
pub fn on_first_boot(snapshot: &ExecutionSnapshot, vanish_count: u32) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("OnFirstBoot"),
        references: vec![vanish_count.to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnBoot`（GET・Ref0=`config.shell_name`）。
///
/// Ref6/7（前回 crash 情報・MATERIA/SSP）は crash 情報を持たない M1 では省略する。
pub fn on_boot(config: &KanadeConfig, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("OnBoot"),
        references: vec![config.shell_name.clone()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `basewareversion`（NOTIFY・Ref0=バージョン／Ref1=本体識別）。
///
/// Ref2（詳細数値・SSP のみ）は省略する。
pub fn baseware_version(config: &KanadeConfig, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Notify {
        id: EventId::Static("basewareversion"),
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
            id: EventId::Static("OnSecondChange"),
            references,
            status,
        }
    } else {
        ShioriCall::Notify {
            id: EventId::Static("OnSecondChange"),
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
        id: EventId::Static("OnClose"),
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
        id: EventId::Static("OnClose"),
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
        id: EventId::Static("OnMouseMove"),
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
        id: EventId::Static("OnMouseDoubleClick"),
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

// ---------------------------------------------------------------------------
// 選択関連イベント（`\q` 選択確定の 4 構築関数・設計 C3・Req3.1〜3.6）
// ---------------------------------------------------------------------------
//
// # 空参照列の規約（マウス系とは**非対称**・Req3.5）
//
// 付随参照列が空のときは、対応する Reference **位置そのものを作らない**（空文字で埋めない）。
// これは [`on_mouse_move`]／[`on_mouse_double_click`] の Ref4（`region: None` → `""` と
// 転写し**位置は保持**する）とは**逆**の規約である。マウス系は正典 layout が固定長で後続
// Reference（Ref5/Ref6）が続くため位置を維持せねばならないのに対し、選択関連の付随参照列は
// 末尾可変長であり、正典では「位置ごと存在しない」形が定義であるため（Req3.5）。
// 両者が同一ファイルに同居する以上、この非対称は意図であって漏れではない。
//
// # 共通リクエストヘッダ（Req3.6）
//
// 4 関数すべてが `snapshot: &ExecutionSnapshot` を**必須引数**として受け取り、
// [`ExecutionStatus::derive`] で自ら共通ヘッダを構成する（既存構築関数と同一規律・DD-IT-3）。
// ヘッダ欠落は「引数を渡さない」という書き方が存在しないため構造上起こらない。

/// `OnChoiceSelectEx`（GET・Ref0=ラベル／Ref1=ID／Ref2 以降＝付随参照列）。
///
/// 正典形（`On` 始まりでない選択肢 ID）カスケードの**先行段**（設計 C3・裁定 2）。
///
/// - Ref0=`label`（表示ラベル・不透明転写・Req3.1）。
/// - Ref1=`id`（選択肢 ID・不透明転写・Req3.1）。
/// - Ref2..=`references`（付随参照列を**記述順**のまま・Req3.1）。
///
/// `references` が空なら Ref2 以降の位置を作らず、Reference は Ref0/Ref1 の 2 個で終わる
/// （空文字で埋めない・Req3.5。上記「空参照列の規約」を参照）。
///
/// ラベル・ID・参照列はいずれも意味解釈せず逐語で転写する（トリム・正規化・空要素除去を
/// しない・Req1.5／[[areka-surface-args-opaque-string-downstream-resolve]] と同精神）。
pub fn on_choice_select_ex(
    label: &str,
    id: &str,
    references: &[String],
    snapshot: &ExecutionSnapshot,
) -> ShioriCall {
    let mut refs = Vec::with_capacity(2 + references.len());
    refs.push(label.to_string());
    refs.push(id.to_string());
    // 空参照列なら extend は 1 要素も足さない＝Ref2 以降の位置が生えない（Req3.5）。
    refs.extend(references.iter().cloned());
    ShioriCall::Get {
        id: EventId::Static("OnChoiceSelectEx"),
        references: refs,
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnChoiceSelect`（GET・Ref0=選択肢 ID のみ）。
///
/// 正典形カスケードの**後続段**（先行 `OnChoiceSelectEx` が 204 のときのみ発行・裁定 2）。
/// Reference は常に 1 個で、付随参照列・表示ラベルは載せない（Req3.2）。
pub fn on_choice_select(id: &str, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("OnChoiceSelect"),
        references: vec![id.to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

/// 任意名イベント（GET・イベント名＝選択肢 ID 逐語・Ref0 以降＝付随参照列のみ）。
///
/// `On` 始まり選択肢 ID の**直接発火 1 段のみ**（先行 Ex／無印を発行しない・裁定 1）。
/// `id` は [`EventId::Choice`] として運ぶ——ゴースト作者が書いた名前を事前登録なしで逐語発火
/// する選択起源カテゴリであり、スケジューラ起源の固定表（[`ALLOWED_EVENT_IDS`]）には載らない
/// （DD-1・Req2.6）。本関数は `EventId::Choice` を構成する events.rs 側の唯一点である。
///
/// - Ref0..=`references`（付随参照列を**記述順**のまま・Req3.3）。
/// - 表示ラベルと選択肢 ID は Reference に**含めない**（Req3.3。ID はイベント名側が運ぶ）。
///
/// `references` が空なら Reference を 1 個も作らない（空 Vec・Req3.5）。
pub fn on_choice_named(
    id: String,
    references: &[String],
    snapshot: &ExecutionSnapshot,
) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Choice(id),
        // 空参照列なら空 Vec のまま＝Reference 位置を 1 個も作らない（Req3.5）。
        references: references.to_vec(),
        status: ExecutionStatus::derive(snapshot),
    }
}

/// `OnChoiceTimeout`（GET・Ref0=タイムアウトした選択肢を含むトークの起動スクリプト）。
///
/// Reference は常に 1 個。`script` は選択肢を含むトークの起動スクリプト（`ActiveTalk.script`・
/// DD-10）を不透明転写する（Req3.4）。
// C4（タスク 4.x）が steady 調停から本構築を呼ぶまでは非テストビルドで未使用となる。
#[allow(dead_code)]
pub fn on_choice_timeout(script: &str, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("OnChoiceTimeout"),
        references: vec![script.to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> KanadeConfig {
        KanadeConfig::new("master", "1.0.0")
    }

    /// GET variant を分解して (id の wire 形, references) を取り出す（Notify なら panic）。
    fn expect_get(call: ShioriCall) -> (String, Vec<String>) {
        match call {
            ShioriCall::Get { id, references, .. } => (id.as_str().to_string(), references),
            ShioriCall::Notify { .. } => panic!("expected GET, got NOTIFY"),
        }
    }

    /// NOTIFY variant を分解して (id の wire 形, references) を取り出す（Get なら panic）。
    fn expect_notify(call: ShioriCall) -> (String, Vec<String>) {
        match call {
            ShioriCall::Notify { id, references, .. } => (id.as_str().to_string(), references),
            ShioriCall::Get { .. } => panic!("expected NOTIFY, got GET"),
        }
    }

    /// 呼出の `id`（出所カテゴリ込み）を GET/NOTIFY 不問で取り出す（許可集合檻の被覆確認用）。
    fn event_id(call: &ShioriCall) -> &EventId {
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

    /// OnFirstBoot Ref0 は vanish 引数由来（Req 4.1／4.2）: `0` で従来の固定値 `"0"` と同値・
    /// 非ゼロ（`7`）はそのまま Reference0 に載る（値源が呼び手へ移ったことの檻）。
    #[test]
    fn on_first_boot_ref0_is_vanish_count_argument() {
        // vanish_count=0 → 従来値 "0" と同値（既存全サイトはこの経路で挙動不変）。
        let (id, references) = expect_get(on_first_boot(&ExecutionSnapshot::INACTIVE, 0));
        assert_eq!(id, "OnFirstBoot");
        assert_eq!(references, vec!["0".to_string()]);

        // vanish_count=7 → Reference0 は "7"（Ref0 の値源が呼び手引数であることを固定）。
        let (id, references) = expect_get(on_first_boot(&ExecutionSnapshot::INACTIVE, 7));
        assert_eq!(id, "OnFirstBoot");
        assert_eq!(references, vec!["7".to_string()]);
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
        let call = on_second_change(MonotonicMs(7_200_000), &ExecutionSnapshot { talk_active: false, choice_active: false });
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
        let call = on_second_change(MonotonicMs(3_600_000), &ExecutionSnapshot { talk_active: true, choice_active: false });
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
            &ExecutionSnapshot { talk_active: false, choice_active: false },
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

    /// 許可 ID 檻（Req3.1/3.2/7.1・DD-IT-8・DD-IE-11・DD-2）: 表が期待11集合と完全一致し
    /// `OnTalk`/`OnHour` を含まない。マウス系2種（OnMouseMove/OnMouseDoubleClick）は
    /// Task 2.1 で additive 追加され、選択関連の固定 3 ID（OnChoiceSelectEx/OnChoiceSelect/
    /// OnChoiceTimeout）は choice-select-events 2.3 で同じ前例に倣い additive 追加された
    /// （whitelist が意図的に 6→8→11 へ増えたための更新）。3 ID はいずれも正典（ukadoc）の
    /// 固定イベント ID であり、「表＝正典固定 ID の部分集合」の性質は保たれる。
    #[test]
    fn allowed_event_ids_are_exactly_the_eleven_and_exclude_ontalk_onhour() {
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
                "OnChoiceSelectEx",
                "OnChoiceSelect",
                "OnChoiceTimeout",
            ]
        );
        assert!(is_allowed_event_id("OnMouseMove"), "OnMouseMove は許可集合に属する（Req7.1）");
        assert!(
            is_allowed_event_id("OnMouseDoubleClick"),
            "OnMouseDoubleClick は許可集合に属する（Req7.1）"
        );
        for id in ["OnChoiceSelectEx", "OnChoiceSelect", "OnChoiceTimeout"] {
            assert!(
                is_allowed_event_id(id),
                "{id} は選択関連の正典固定 ID ゆえ許可集合に属する（DD-2）"
            );
        }
        assert!(!is_allowed_event_id("OnTalk"), "OnTalk は恒久的に許可しない（Req3.2）");
        assert!(!is_allowed_event_id("OnHour"), "OnHour は恒久的に許可しない（Req3.2）");
        // 表の全要素が許可判定を通ること。
        for id in ALLOWED_EVENT_IDS {
            assert!(is_allowed_event_id(id), "{id} は表にあるのに許可されない");
        }
    }

    /// 選択起源の受理規則（Req2.6・DD-2）: `On` 接頭のみを受理し、事前の固定登録を要さない。
    ///
    /// 判定は接頭辞ただ 1 条件——ゴースト作者が `\q` の ID に書いた名前を逐語で受理するため、
    /// 固定表（[`ALLOWED_EVENT_IDS`]）への登録有無・大小文字の揺れの補正はいずれも行わない。
    #[test]
    fn is_allowed_choice_event_accepts_only_on_prefixed_names_verbatim() {
        // 受理: 未登録の任意名・境界入力（"On" 単独）・正典固定 ID の逐語形。
        for id in [
            "On",
            "OnMenu",
            "Onおしゃべり頻度メニュー",
            "OnChoiceSelect",
            "On ",
        ] {
            assert!(
                is_allowed_choice_event(id),
                "{id} は On 接頭ゆえ選択起源として受理される（Req2.6）"
            );
        }
        // 拒否: On 接頭でない形（空文字・小文字・大文字・部分一致・接頭でない位置）。
        for id in ["", "foo", "on", "onMenu", "ONMENU", "MenuOn", " OnMenu"] {
            assert!(
                !is_allowed_choice_event(id),
                "{id:?} は On 接頭でないゆえ選択起源として受理されない（Req2.6）"
            );
        }
    }

    /// 裁定 8（Req2.9）: スケジューラ起源の恒久禁止と choice 起源の逐語発火が**交差しない**。
    ///
    /// `OnTalk`／`OnHour` は「ベースウェアが自発的に周期発火すると消費側ゴーストの自発生成と
    /// 二重駆動する」ことを根拠に固定表から恒久的に除外されるが、この根拠はゴースト作者が
    /// 選択肢へ明示的に書いた 1 クリック = 1 回の発火には該当しない。ゆえに同じ ID が
    /// スケジューラ起源では拒否・選択起源では受理される（両方向をこの 1 檻で固定する）。
    #[test]
    fn scheduler_forbidden_ids_are_still_fireable_from_choice_origin() {
        for id in ["OnTalk", "OnHour"] {
            assert!(
                !is_allowed_event_id(id),
                "{id} はスケジューラ起源では恒久的に禁止（Req3.2・自発生成との二重駆動）"
            );
            assert!(
                is_allowed_choice_event(id),
                "{id} は選択起源なら逐語で発火できる（Req2.9・恒久禁止を適用しない）"
            );
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
        let active = ExecutionSnapshot { talk_active: true, choice_active: false };
        let mv = on_mouse_move(0, 0, 0, Some("Head"), &active);
        assert_eq!(call_status(&mv), Some("talking".to_string()));
        let dbl = on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &active);
        assert_eq!(call_status(&dbl), Some("talking".to_string()));
    }

    /// 全構築関数の返す `id` が**スケジューラ起源**（[`EventId::Static`]）であること（DD-1）。
    ///
    /// 選択起源（[`EventId::Choice`]）はカスケード planner のみが構成する不変条件を、構築関数側から
    /// 固定する檻——events.rs の構築関数が任意名を作り得ないことを型の実値で観測する。
    ///
    /// 対象は**スケジューラ起源**の構築関数のみ。選択起源の [`on_choice_named`] は設計どおり
    /// [`EventId::Choice`] を返すため本檻の被覆対象ではなく、
    /// `choice_constructors_split_event_id_category_by_origin` が別途カテゴリを固定する。
    #[test]
    fn every_construction_function_returns_static_event_id() {
        let cfg = config();
        let snap = ExecutionSnapshot::INACTIVE;
        let calls = [
            on_initialize(&snap),
            on_first_boot(&snap, 0),
            on_boot(&cfg, &snap),
            baseware_version(&cfg, &snap),
            on_second_change(MonotonicMs(0), &snap),
            on_second_change(MonotonicMs(0), &ExecutionSnapshot { talk_active: true, choice_active: false }),
            on_close(CloseReason::User, &snap),
            on_close_notify(CloseReason::System, &snap),
            on_mouse_move(0, 0, 0, Some("Head"), &snap),
            on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &snap),
        ];
        for call in &calls {
            let id = event_id(call);
            assert!(
                matches!(id, EventId::Static(_)),
                "構築関数がスケジューラ起源でない id={} を返した",
                id.as_str()
            );
        }
    }

    /// 全構築関数の返す `id` が許可集合の要素であること（Service Interface Postcondition）。
    ///
    /// 対象は [`EventId::Static`] を返す構築関数——固定 3 ID を許可表へ載せた（DD-2）ことにより
    /// `on_choice_select_ex`／`on_choice_select`／`on_choice_timeout` も本檻の被覆対象に入る。
    /// 選択起源の任意名 `on_choice_named`（[`EventId::Choice`]）だけは固定表ではなく出所別の
    /// 受理規則（`is_allowed_choice_event`）で検証されるため、本檻の被覆対象外である。
    #[test]
    fn every_construction_function_returns_an_allowed_id() {
        let cfg = config();
        let snap = ExecutionSnapshot::INACTIVE;
        let calls = [
            on_initialize(&snap),
            on_first_boot(&snap, 0),
            on_boot(&cfg, &snap),
            baseware_version(&cfg, &snap),
            on_second_change(MonotonicMs(0), &snap),
            on_second_change(MonotonicMs(0), &ExecutionSnapshot { talk_active: true, choice_active: false }),
            on_close(CloseReason::User, &snap),
            on_close_notify(CloseReason::System, &snap),
            on_mouse_move(0, 0, 0, Some("Head"), &snap),
            on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &snap),
            on_choice_select_ex("ラベル", "ID", &[], &snap),
            on_choice_select("ID", &snap),
            on_choice_timeout("\\e", &snap),
        ];
        for call in &calls {
            let id = event_id(call).as_str();
            assert!(
                is_allowed_event_id(id),
                "構築関数が許可集合外の id={id} を返した"
            );
        }
    }

    /// テスト用の付随参照列（不透明転写の檻に使う「加工されたら壊れる」値の並び）。
    ///
    /// 非 ASCII・前後空白・空文字要素・記号（カンマ／バックスラッシュ）を含み、トリム・
    /// 正規化・空要素除去のいずれかが混入すれば必ず不一致になる。
    fn opaque_references() -> Vec<String> {
        vec![
            " 頻度  ".to_string(),
            String::new(),
            "a,b".to_string(),
            "\\q[x,y]".to_string(),
        ]
    }

    /// `OnChoiceSelectEx` 正典 layout（Req3.1）:
    /// Ref0=ラベル／Ref1=ID／Ref2 以降が付随参照列の記述順であること（位置と値の実値突合）。
    #[test]
    fn on_choice_select_ex_builds_label_id_then_references() {
        let references = opaque_references();
        let call = on_choice_select_ex(
            "おしゃべり頻度",
            "Choice頻度",
            &references,
            &ExecutionSnapshot::INACTIVE,
        );
        let (id, refs) = expect_get(call);
        assert_eq!(id, "OnChoiceSelectEx");
        assert_eq!(
            refs,
            vec![
                "おしゃべり頻度".to_string(), // Ref0=表示ラベル（Req3.1）
                "Choice頻度".to_string(),     // Ref1=選択肢 ID（Req3.1）
                " 頻度  ".to_string(),        // Ref2 以降＝付随参照列を記述順（不透明転写）
                String::new(),
                "a,b".to_string(),
                "\\q[x,y]".to_string(),
            ]
        );
        assert_eq!(refs.len(), 2 + references.len(), "Reference 数は 2＋付随参照列長");
    }

    /// 空参照列で Ref2 以降の位置が生えないこと（Req3.5）: Reference は Ref0/Ref1 の**2 個のみ**。
    ///
    /// 既存マウス系の `None→""`（位置保持）とは**非対称**な規約であることを実値で固定する。
    #[test]
    fn on_choice_select_ex_with_empty_references_stops_at_ref1() {
        let (id, refs) = expect_get(on_choice_select_ex(
            "ラベル",
            "ID",
            &[],
            &ExecutionSnapshot::INACTIVE,
        ));
        assert_eq!(id, "OnChoiceSelectEx");
        assert_eq!(refs, vec!["ラベル".to_string(), "ID".to_string()]);
        assert_eq!(refs.len(), 2, "空参照列は Ref2 以降の位置を作らない（空文字で埋めない）");
    }

    /// `OnChoiceSelect` 正典 layout（Req3.2）: Ref0=選択肢 ID の**1 個のみ**。
    #[test]
    fn on_choice_select_builds_id_only_ref0() {
        let (id, refs) = expect_get(on_choice_select("Choice頻度", &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnChoiceSelect");
        assert_eq!(refs, vec!["Choice頻度".to_string()]);
        assert_eq!(refs.len(), 1, "無印は常に Ref0=ID の 1 個のみ");
    }

    /// 任意名イベント正典 layout（Req3.3）: Ref0 以降が付随参照列のみで、
    /// 表示ラベルと選択肢 ID を Reference に**含めない**こと。
    #[test]
    fn on_choice_named_builds_references_from_ref0_without_label_or_id() {
        let references = opaque_references();
        let call = on_choice_named(
            "Onおしゃべり頻度メニュー".to_string(),
            &references,
            &ExecutionSnapshot::INACTIVE,
        );
        let (id, refs) = expect_get(call);
        assert_eq!(id, "Onおしゃべり頻度メニュー", "任意名は逐語で wire へ載る");
        assert_eq!(refs, references, "Ref0 以降＝付随参照列そのもの（記述順・不透明転写）");
        assert!(
            !refs.contains(&"Onおしゃべり頻度メニュー".to_string()),
            "任意名イベントの Reference に選択肢 ID を含めない（Req3.3）"
        );
    }

    /// 空参照列で Reference が 1 個も生えないこと（Req3.3/3.5）: References は空 Vec。
    #[test]
    fn on_choice_named_with_empty_references_builds_no_reference() {
        let (id, refs) = expect_get(on_choice_named(
            "OnMenu".to_string(),
            &[],
            &ExecutionSnapshot::INACTIVE,
        ));
        assert_eq!(id, "OnMenu");
        assert!(
            refs.is_empty(),
            "空参照列は Reference 位置を 1 個も作らない（空文字で埋めない・Req3.5）"
        );
    }

    /// `OnChoiceTimeout` 正典 layout（Req3.4）: Ref0=起動スクリプトの**1 個のみ**・不透明転写。
    #[test]
    fn on_choice_timeout_builds_script_ref0() {
        let script = "\\0\\s[0]選んで\\q[はい,Onはい]\\q[いいえ,Onいいえ]\\e";
        let (id, refs) = expect_get(on_choice_timeout(script, &ExecutionSnapshot::INACTIVE));
        assert_eq!(id, "OnChoiceTimeout");
        assert_eq!(refs, vec![script.to_string()]);
        assert_eq!(refs.len(), 1, "Timeout は常に Ref0=script の 1 個のみ");
    }

    /// 共通リクエストヘッダ（実行状態スナップショット）が 4 構築関数すべてに載ること（Req3.6）。
    ///
    /// snapshot が必須引数であるため欠落は構造上起こらない。実値としても、
    /// INACTIVE→ヘッダ行なし（`None`）／talk_active=true→`talking` の双方向を固定する。
    #[test]
    fn choice_constructors_carry_the_common_request_header() {
        let refs = opaque_references();
        let active = ExecutionSnapshot { talk_active: true, choice_active: false };
        let idle = ExecutionSnapshot::INACTIVE;

        let active_calls = [
            on_choice_select_ex("ラベル", "ID", &refs, &active),
            on_choice_select("ID", &active),
            on_choice_named("OnMenu".to_string(), &refs, &active),
            on_choice_timeout("\\e", &active),
        ];
        for call in &active_calls {
            assert_eq!(
                call_status(call),
                Some("talking".to_string()),
                "選択関連イベントも共通ヘッダを snapshot から導出する（Req3.6）: id={}",
                event_id(call).as_str()
            );
        }

        let idle_calls = [
            on_choice_select_ex("ラベル", "ID", &refs, &idle),
            on_choice_select("ID", &idle),
            on_choice_named("OnMenu".to_string(), &refs, &idle),
            on_choice_timeout("\\e", &idle),
        ];
        for call in &idle_calls {
            assert_eq!(
                call_status(call),
                None,
                "非アクティブ snapshot は Status ヘッダ行を出さない: id={}",
                event_id(call).as_str()
            );
        }
    }

    /// 出所カテゴリの型分離（DD-1）: 任意名イベントのみ [`EventId::Choice`]、
    /// 固定 ID 3 種は [`EventId::Static`]。
    #[test]
    fn choice_constructors_split_event_id_category_by_origin() {
        let snap = ExecutionSnapshot::INACTIVE;
        let named = on_choice_named("OnMenu".to_string(), &[], &snap);
        assert!(
            matches!(event_id(&named), EventId::Choice(_)),
            "任意名イベントは選択起源（EventId::Choice）"
        );

        let statics = [
            on_choice_select_ex("ラベル", "ID", &[], &snap),
            on_choice_select("ID", &snap),
            on_choice_timeout("\\e", &snap),
        ];
        for call in &statics {
            let id = event_id(call);
            assert!(
                matches!(id, EventId::Static(_)),
                "固定 ID の選択関連イベントはスケジューラ起源の型を保つ id={}",
                id.as_str()
            );
        }
    }
}
