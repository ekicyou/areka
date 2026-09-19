//! resources — SHIORI Resource 照会の許可集合（イベント檻とは別族の単一正本）。
//!
//! 本モジュールは kanade の **SHIORI リソース照会増分**（design「kanade（リソース照会増分）」）
//! の許可語彙 [`ALLOWED_RESOURCE_IDS`] とその判定 [`is_allowed_resource_id`] を提供する。
//! イベント発火の許可集合（[`crate::schedule::events::ALLOWED_EVENT_IDS`]）とは**別族**であり、
//! egress チョークポイント（`actor.rs` の `round_trip_request`）の submit ガードは
//! 「`is_allowed_event_id(id)` ∨ `is_allowed_resource_id(id)`」へ拡張される（既存イベント檻は
//! 無改変・許可外は従来どおり `ShioriFailure::Internal`・Req4.1）。
//!
//! # 別族である理由（design 論点1・Boundary Commitments）
//! リソース照会は「イベント発火」ではなく「値源への問い合わせ」であり、`OnTalk`/`OnHour` を
//! 恒久禁止するイベント檻の語彙とは意味論が異なる。両者を混ぜず別集合として保つことで、
//! イベント語彙の不変量を保存したままリソース ID を additive に増分する。
//!
//! # 範囲
//! 許可集合と判定（[`ALLOWED_RESOURCE_IDS`]／[`is_allowed_resource_id`]）・照会構築関数
//! （汎用の [`resource_get`] とその上に畳まれた [`resource_username`]）・結果語彙
//! [`ResourceOutcome`]・注入シーム [`ResourceSink`]（boot 系列 prefetch から呼ばれる）を持つ。
//! prefetch 段そのものの挿入は `boot.rs`（OnInitialize 後・OnFirstBoot 前）が、メニューからの
//! 任意時点の照会は `actor_resources.rs` が担う。

use crate::msg::{EventId, ShioriCall};
use crate::status::{ExecutionSnapshot, ExecutionStatus};

/// SHIORI Resource 照会で送出し得るリソース ID の確定ホワイトリスト（利用者名＋メニュー 9 名・Req4.1）。
///
/// イベント発火の許可集合（[`crate::schedule::events::ALLOWED_EVENT_IDS`]）とは**別族**である。
/// egress ガードは「イベント許可 ∨ リソース許可」で判定するため、本集合の要素は許可外拒否を
/// 免れて送出される（ただし送出経路・往復規律はイベントと共通）。
///
/// 内訳は利用者名（boot prefetch）・メニュー枠 7 種の項目名（要件 3.1）・本体側／相方側の
/// メニュー表示可否（要件 3.6）である。`popupmenu.type` 系は**問い合わせない**裁定なので載せない
/// （要件 3.8）。n≧2 の `char*.popupmenu.visible` も α に窓が無いので載せない（要件 3.6）。
///
/// SEAM(M2・159 項目汎用化): SHIORI Resource は正典で 159 項目ある。語彙拡張は本集合への
/// ID 追加（additive）で行い、判定側と構築側（[`resource_get`]）は無改変で追随する。
pub const ALLOWED_RESOURCE_IDS: &[&str] = &[
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#username:1
    "username",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#ghostrootbutton.caption:1
    "ghostrootbutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#shellrootbutton.caption:1
    "shellrootbutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#balloonrootbutton.caption:1
    "balloonrootbutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebutton.caption:1
    "updatebutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#ghostinstallbutton.caption:1
    "ghostinstallbutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#readmebutton.caption:1
    "readmebutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#closebutton.caption:1
    "closebutton.caption",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#sakura.popupmenu.visible:1
    "sakura.popupmenu.visible",
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#kero.popupmenu.visible:1
    "kero.popupmenu.visible",
];

/// `id` がリソース送出許可集合（[`ALLOWED_RESOURCE_IDS`]）に属するかを判定する（Req4.1）。
///
/// イベント許可判定（[`crate::schedule::events::is_allowed_event_id`]）とは独立した別族の判定で
/// あり、submit ガードは両者の論理和で送出可否を決める。
pub fn is_allowed_resource_id(id: &str) -> bool {
    ALLOWED_RESOURCE_IDS.contains(&id)
}

/// `username` リソース照会（GET・References なし・Status は既存イベント同様 snapshot から導出）。
///
/// SHIORI Resource `username` への GET 照会を組み立てる純粋関数（副作用なし・Req4.1）。既存イベント
/// 構築関数（`events::*`）と同じく `snapshot` から [`ExecutionStatus`] を導出し、送出時点の実行状態を
/// wire 証跡へ載せる。boot prefetch 段は talk 非アクティブ（[`ExecutionSnapshot::INACTIVE`]）で呼ぶ。
///
/// References は持たない（正典 Resource GET は Reference を要しない）。応答（200 Value／204／失敗）の
/// [`ResourceOutcome`] への写像・[`ResourceSink`] 呼出・完了固定ログは prefetch 段（`boot.rs`）が担う。
///
/// 本体は汎用の [`resource_get`] に `"username"` を渡すだけの薄い呼び出しであり、組み上がる照会は
/// 従来と同一である（boot 系列の既存呼び手は無改変）。
pub fn resource_username(snapshot: &ExecutionSnapshot) -> ShioriCall {
    resource_get("username", snapshot)
}

/// 任意のリソース ID への照会（GET・References なし・Status は snapshot から導出）を組み立てる
/// 純粋関数（副作用なし・要件 3.2）。
///
/// `id` は [`ALLOWED_RESOURCE_IDS`] の要素であることを**求めない**——送出可否の判定は egress
/// チョークポイント（`actor.rs` の `round_trip_request`）が [`is_allowed_resource_id`] で行い、
/// 許可外は従来どおり `ShioriFailure::Internal` で拒否される（檻は 1 か所・Req4.1）。
///
/// `&'static str` に据えるのは [`ShioriCall`] の `id: EventId::Static(&'static str)` 契約
/// （スケジューラ起源・DD-1）に従うためで、呼び手は許可表の要素そのものを渡す。
pub fn resource_get(id: &'static str, snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static(id),
        references: Vec::new(),
        status: ExecutionStatus::derive(snapshot),
    }
}

/// リソース照会の結果語彙（prefetch 段が SHIORI 応答から写像し [`ResourceSink`] へ渡す）。
///
/// - [`Value`](ResourceOutcome::Value): 200 応答の値（不透明文字列）。
/// - [`NoContent`](ResourceOutcome::NoContent): 204 / 空値（既定値縮退は消費側 sakura の責務・R4.2）。
/// - [`Failed`](ResourceOutcome::Failed): タイムアウト・IPC 断等の照会失敗（理由文字列を保持）。
///   失敗でも boot は殺さず続行する（prefetch 段が warn＋本語彙を sink へ渡す・R4.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceOutcome {
    /// 200 応答の値（不透明文字列）。
    Value(String),
    /// 204 / 空値（既定値縮退は消費側 sakura に残置・R4.2）。
    NoContent,
    /// 照会失敗（タイムアウト・IPC 断等・理由文字列を保持）。
    Failed(String),
}

/// リソース照会結果の注入シンク（kanade 構築時に注入・prefetch 段が**同期的に**呼ぶ）。
///
/// `SystemVarSource` と同型の疎結合シームであり、kanade は結果の消費側（ghost/sylphya）へ**依存しない**
/// （sink は素のクロージャ）。prefetch 段は `(id, outcome)` を渡して sink が**返るまで boot を進めない**
/// （ghost が据える sink 内の publish＋barrier により初回 talk までの反映が決定論化する・研究 §12-1）。
///
/// # 契約（design Preconditions）
/// sink は `Send`（別スレッドの kanade アクターから呼ばれる）であり、呼出で panic しない（ghost 結線の
/// 責務）。既存テストのように結果を使わない構成では no-op sink（`Box::new(|_, _| {})`）を注入してよい。
pub type ResourceSink = Box<dyn Fn(&'static str, ResourceOutcome) + Send>;

#[cfg(test)]
mod tests {
    use super::*;

    /// GET 照会の観測できる形（id・References・Status）だけを取り出す。`ShioriCall` は
    /// 比較を derive していないので、比較のためにこの組へ落とす。
    fn shape_of(call: ShioriCall) -> (String, Vec<String>, Option<String>) {
        match call {
            ShioriCall::Get {
                id,
                references,
                status,
            } => (id.as_str().to_string(), references, status.render()),
            ShioriCall::Notify { .. } => panic!("GET を期待している"),
        }
    }

    /// 許可リソース ID（`username`）は判定を通る（Req4.1）。
    #[test]
    fn username_is_allowed_resource_id() {
        assert!(
            is_allowed_resource_id("username"),
            "username はリソース許可集合に属する（Req4.1）"
        );
    }

    /// 許可集合外のリソース ID は判定を通らない（別族の否定側檻）。
    #[test]
    fn non_resource_id_is_not_allowed() {
        assert!(
            !is_allowed_resource_id("notaresource"),
            "許可集合外のリソース ID は拒否される"
        );
    }

    /// リソース許可集合は厳密にこの 10 名（語彙の凍結檻・Req4.1／要件 3.2）。
    #[test]
    fn allowed_resource_ids_are_exactly_the_ten_names() {
        assert_eq!(
            ALLOWED_RESOURCE_IDS,
            &[
                "username",
                "ghostrootbutton.caption",
                "shellrootbutton.caption",
                "balloonrootbutton.caption",
                "updatebutton.caption",
                "ghostinstallbutton.caption",
                "readmebutton.caption",
                "closebutton.caption",
                "sakura.popupmenu.visible",
                "kero.popupmenu.visible",
            ],
            "リソース許可集合は利用者名＋枠 7 種の項目名＋本体側／相方側の表示可否の 10 名"
        );
        for id in ALLOWED_RESOURCE_IDS {
            assert!(
                is_allowed_resource_id(id),
                "{id} は集合にあるのに許可されない"
            );
        }
    }

    /// 問い合わせない裁定の名前（`popupmenu.type` 系・要件 3.8）と n≧2 の表示可否は
    /// 許可集合に**入れない**——従来どおり拒否される。
    #[test]
    fn unqueried_menu_names_are_not_allowed() {
        for id in [
            "sakura.popupmenu.type",
            "kero.popupmenu.type",
            "char2.popupmenu.type",
            "char2.popupmenu.visible",
        ] {
            assert!(
                !is_allowed_resource_id(id),
                "{id} は問い合わせない名前なので許可集合に属してはならない（要件 3.8／3.6）"
            );
        }
    }

    /// イベント許可集合とは**別族**であること: イベント ID はリソース判定を通らない
    /// （族の分離檻・design 論点1／Boundary Commitments）。
    #[test]
    fn event_ids_are_not_resource_ids() {
        for ev in crate::schedule::events::ALLOWED_EVENT_IDS {
            assert!(
                !is_allowed_resource_id(ev),
                "イベント ID {ev} はリソース許可集合に属してはならない（別族）"
            );
        }
    }

    /// `resource_username` は GET・id=`"username"`・References なし・Status は snapshot 由来
    /// （INACTIVE→ヘッダ行なし・Req4.1）。id は許可集合を通る。
    #[test]
    fn resource_username_is_get_with_no_references() {
        let call = resource_username(&ExecutionSnapshot::INACTIVE);
        match call {
            ShioriCall::Get {
                id,
                references,
                status,
            } => {
                assert_eq!(
                    id,
                    EventId::Static("username"),
                    "リソース照会 id はスケジューラ起源の username リテラル（M1・DD-1）"
                );
                assert!(
                    references.is_empty(),
                    "Resource GET は References を持たない"
                );
                assert_eq!(
                    status.render(),
                    None,
                    "INACTIVE スナップショットは Status ヘッダを出さない"
                );
                assert!(
                    is_allowed_resource_id(id.as_str()),
                    "username は送出許可集合を通る"
                );
            }
            ShioriCall::Notify { .. } => panic!("resource_username は GET を返すべき"),
        }
    }

    /// `resource_get` は任意の許可名について `resource_username` と同じ形の GET を組む
    /// （id だけが違う・References なし・Status は snapshot 由来・要件 3.2）。
    #[test]
    fn resource_get_builds_the_same_shape_for_any_allowed_id() {
        let call = resource_get("readmebutton.caption", &ExecutionSnapshot::INACTIVE);
        match call {
            ShioriCall::Get {
                id,
                references,
                status,
            } => {
                assert_eq!(
                    id,
                    EventId::Static("readmebutton.caption"),
                    "渡した id がそのまま照会 id になる"
                );
                assert!(
                    references.is_empty(),
                    "Resource GET は References を持たない"
                );
                assert_eq!(
                    status.render(),
                    None,
                    "INACTIVE スナップショットは Status ヘッダを出さない"
                );
                assert!(
                    is_allowed_resource_id(id.as_str()),
                    "readmebutton.caption は送出許可集合を通る"
                );
            }
            ShioriCall::Notify { .. } => panic!("resource_get は GET を返すべき"),
        }
        // `resource_username` は同じ汎用関数の上に畳まれている（出力が一致する）。
        let snapshot = ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
        };
        let folded = shape_of(resource_username(&snapshot));
        assert_eq!(
            folded,
            shape_of(resource_get("username", &snapshot)),
            "resource_username は resource_get(\"username\", ..) と同じ照会を組む"
        );
    }

    /// talk_active=true では Status: talking を snapshot から導出する（既存イベント構築子と同一規律）。
    #[test]
    fn resource_username_carries_talking_status_when_active() {
        let call = resource_username(&ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
        });
        let status = match call {
            ShioriCall::Get { status, .. } => status.render(),
            ShioriCall::Notify { .. } => panic!("expected GET"),
        };
        assert_eq!(status, Some("talking".to_string()));
    }

    /// `ResourceOutcome` の 3 語彙が構築でき、値の同一性（PartialEq/Eq）で観測できる。
    #[test]
    fn resource_outcome_variants_construct_and_compare() {
        assert_eq!(
            ResourceOutcome::Value("bob".to_string()),
            ResourceOutcome::Value("bob".to_string())
        );
        assert_ne!(
            ResourceOutcome::NoContent,
            ResourceOutcome::Value("x".to_string())
        );
        assert_ne!(
            ResourceOutcome::Failed("timeout".to_string()),
            ResourceOutcome::NoContent
        );
    }

    /// `ResourceSink` は素のクロージャシームであり、`(&'static str, ResourceOutcome)` で呼べる。
    /// 既存テスト向けの no-op sink（`Box::new(|_, _| {})`）が型検査を通る（Implementation Notes）。
    #[test]
    fn resource_sink_is_a_plain_closure_seam() {
        use std::sync::{Arc, Mutex};
        let seen: Arc<Mutex<Vec<(&'static str, ResourceOutcome)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let seen_body = Arc::clone(&seen);
        let sink: ResourceSink =
            Box::new(move |id, outcome| seen_body.lock().unwrap().push((id, outcome)));
        sink("username", ResourceOutcome::Value("bob".to_string()));
        assert_eq!(
            seen.lock().unwrap().as_slice(),
            &[("username", ResourceOutcome::Value("bob".to_string()))]
        );
        // no-op sink も同じ型で構築できる（既存呼び手の無害注入・Implementation Notes）。
        let _noop: ResourceSink = Box::new(|_, _| {});
    }
}
