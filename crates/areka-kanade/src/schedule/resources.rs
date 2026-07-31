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
//! イベント語彙 8 ID の不変量を保存したままリソース ID（M1: `"username"`）を additive に増分する。
//!
//! # M1 の範囲（タスク 6.1／6.2）
//! タスク 6.1 が許可集合と判定（[`ALLOWED_RESOURCE_IDS`]／[`is_allowed_resource_id`]）を、
//! タスク 6.2 が照会構築関数 [`resource_username`]・結果語彙 [`ResourceOutcome`]・注入シーム
//! [`ResourceSink`]（boot 系列 prefetch から呼ばれる）を持つ。prefetch 段そのものの挿入は
//! `boot.rs`（OnInitialize 後・OnFirstBoot 前）が担う。

use crate::msg::{EventId, ShioriCall};
use crate::status::{ExecutionSnapshot, ExecutionStatus};

/// SHIORI Resource 照会で送出し得るリソース ID の確定ホワイトリスト（M1: `username` 1 件・Req4.1）。
///
/// イベント発火の許可集合（[`crate::schedule::events::ALLOWED_EVENT_IDS`]）とは**別族**である。
/// egress ガードは「イベント許可 ∨ リソース許可」で判定するため、本集合の要素は許可外拒否を
/// 免れて送出される（ただし送出経路・往復規律はイベントと共通）。
///
/// SEAM(M2・159 項目汎用化): SHIORI Resource は正典で 159 項目あるが、M1 は源のある `username`
/// のみを実導出する。語彙拡張は本集合への ID 追加（additive）で行い、判定側は無改変で追随する。
pub const ALLOWED_RESOURCE_IDS: &[&str] = &["username"];

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
/// SEAM(M2・159 項目汎用化): `id` は M1 ではリテラル `&'static str` の `"username"` に据え置く。
/// SHIORI Resource は正典で 159 項目あるが、159 項目の String 汎用化（任意リソース名への一般化）は
/// M2 のシームであり、[`ShioriCall`] の `id: EventId::Static(&'static str)` 契約（スケジューラ起源・
/// DD-1）と `ALLOWED_RESOURCE_IDS` への ID 追加（additive）で段階的に拡張する。
pub fn resource_username(snapshot: &ExecutionSnapshot) -> ShioriCall {
    ShioriCall::Get {
        id: EventId::Static("username"),
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

    /// リソース許可集合は M1 では厳密に `["username"]` の 1 件（語彙の凍結檻・Req4.1）。
    #[test]
    fn allowed_resource_ids_are_exactly_username() {
        assert_eq!(
            ALLOWED_RESOURCE_IDS,
            &["username"],
            "M1 のリソース許可集合は username 1 件のみ"
        );
        for id in ALLOWED_RESOURCE_IDS {
            assert!(is_allowed_resource_id(id), "{id} は集合にあるのに許可されない");
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
                assert!(references.is_empty(), "Resource GET は References を持たない");
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

    /// talk_active=true では Status: talking を snapshot から導出する（既存イベント構築子と同一規律）。
    #[test]
    fn resource_username_carries_talking_status_when_active() {
        let call = resource_username(&ExecutionSnapshot { talk_active: true });
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
        assert_ne!(ResourceOutcome::NoContent, ResourceOutcome::Value("x".to_string()));
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
        let seen: Arc<Mutex<Vec<(&'static str, ResourceOutcome)>>> = Arc::new(Mutex::new(Vec::new()));
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
