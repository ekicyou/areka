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
//! # M1 の範囲（本タスク 6.1）
//! 本モジュールは許可集合と判定のみを持つ。`resource_username` 構築関数・`ResourceOutcome`／
//! `ResourceSink` 型・boot 系列 prefetch はタスク 6.2 の担当であり本タスクでは実装しない。

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
}
