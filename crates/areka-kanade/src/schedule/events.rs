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
/// - Ref0=`x`（ローカル x 座標・**縮約後サーフェス px**・下記「座標空間」節）。
/// - Ref1=`y`（ローカル y 座標・同上）。
/// - Ref2=[`REF2_WHEEL_M1`]（ホイール回転量・M1 固定 "0"・Req2.4）。
/// - Ref3=`scope`（対象スコープ・本体 0／相方 1）。
/// - Ref4=`region`（当たり判定の識別子・不透明転写・`None`→空文字・Req2.3/DD-IE-6）。
/// - Ref5=`"0"`（移動はボタン押下を伴わないため常に "0"・Req2.5）。
/// - Ref6=[`REF6_DEVICE_MOUSE`]（入力デバイス種・M1 固定 "mouse"・DD-IE-6）。
///
/// `region` は collision resolver 由来の領域名を意味解釈せず不透明転写する（kanade は
/// 当たり判定名を解釈しない・[[areka-surface-args-opaque-string-downstream-resolve]] と同精神）。
///
/// # 座標空間（areka-P0-collision-dpi-hittest・R1.8/R5.3）
///
/// Ref0/Ref1 の値は **縮約後のサーフェス px**（作者定義の合成座標系）であり、Ref4 の当たり判定
/// 識別子が解決された空間と**同一**である。窓 client 物理 px ではない——÷k（実適用の表示スケール）は
/// `areka-emo-present` の `EmoPresenter::hit_region_client` ただ 1 箇所が吸収し、本関数は呼び手
/// （[`crate::msg::MouseInput`] の `x`/`y`）から受けた値を**無変換で** Reference へ載せるだけである。座標契約の
/// 正本は `areka` bin の `emo2_boot::hit_region` モジュール冒頭 doc。正典（ukadoc）の `OnMouseMove`
/// Reference0/1 は「ローカル座標」としか規定せず空間を定義していないため、サーフェス px を採ることは
/// areka 側の裁定である（R1.8）。k=1.0 では縮約が恒等ゆえ従前の配信値と完全に一致する。
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
/// - Ref0=`x`／Ref1=`y`（座標・**縮約後サーフェス px**・[`on_mouse_move`] の「座標空間」節と同一契約）。
/// - Ref2=`"0"`（正典で常に "0"・Req3.2）。
/// - Ref3=`scope`（対象スコープ・本体 0／相方 1）。
/// - Ref4=`region`（当たり判定の識別子・不透明転写・`None`→空文字・Req3.4/DD-IE-6）。
/// - Ref5=`button`（左 [`MouseButton::Left`]→"0"／右 [`MouseButton::Right`]→"1"・Req3.3）。
/// - Ref6=[`REF6_DEVICE_MOUSE`]（入力デバイス種・M1 固定 "mouse"・DD-IE-6）。
///
/// 座標は Ref4 の当たり判定識別子と同一空間（作者定義のサーフェス px）に揃う。÷k は
/// `areka-emo-present` の `EmoPresenter::hit_region_client` が吸収済みで、本関数は無変換で転写する
/// （正本＝`areka` bin の `emo2_boot::hit_region` モジュール冒頭 doc・areka 側の裁定＝R1.8）。
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
// 消費点は steady 調停の期限到達アーム（`fire_choice_timeout_if_due`・C4 規則 5）。
pub fn on_choice_timeout(script: &str, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("OnChoiceTimeout"),
        references: vec![script.to_string()],
        status: ExecutionStatus::derive(snapshot),
    }
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod tests;
