//! 永続値 ⇄ 配置の変換を所有する純関数群（areka-P0-position-persist・design C1）。
//!
//! 統一プロパティシステム（sylphya）が確定した永続 key 族（窓位置・バルーン相対
//! オフセット・起動記録・vanish 回数）を**消費**し、IO・World 変異と分離した
//! 決定論的な純関数として「値を差す／値を書く」変換を提供する。永続ストア実体
//! （形式・原子性・寛容読取・スコープ分離）は sylphya の領分であり、本モジュールは
//! その契約（[`areka_sylphya::PersistKey`]／[`areka_sylphya::Axis`]）を消費するのみ。
//!
//! 本モジュールは永続への書込 API を持たない（保存の投函口は上位の結線層が持つ）。
//! task 1.1（Foundation）で用意するのは決定論的な変換のみ——寛容 parse（[`parse_px`]）
//! と保存 entries 構築（[`char_pos_entries`]／[`balloon_offset_entries`]）。復元 merge・
//! 再射影・バルーン基準変換・`PersistWiring` は後続タスクで本モジュールへ追加する。

use areka_sylphya::{Axis, PersistKey};

use super::resolver::PointPx;

/// 永続値の寛容 parse（design C1・6.1）。
///
/// 数値文字列 → `Some(i32)`。非数値・空文字列 → `None`（＝「値なし」・呼び手が warn＋
/// 既定へ縮退する）。前後空白は許容する（寛容）。決定論（同一入力→同一出力）。
pub fn parse_px(value: &str) -> Option<i32> {
    // 寛容縮退（6.1）: 非数値・空・小数は「値なし」＝None（panic なし）。
    value.trim().parse::<i32>().ok()
}

/// キャラ窓位置の保存 entries を構築する（design C1・純関数・1.6）。
///
/// スコープ別の [`PersistKey::WindowPos`]（X/Y）へ `pos` を i32 の `Display` で
/// 文字列化して載せる。値ドメインは物理 px・仮想スクリーン絶対 i32（負値可）。
pub fn char_pos_entries(scope: u32, pos: PointPx) -> Vec<(PersistKey, String)> {
    vec![
        (PersistKey::WindowPos { scope, axis: Axis::X }, pos.x.to_string()),
        (PersistKey::WindowPos { scope, axis: Axis::Y }, pos.y.to_string()),
    ]
}

/// バルーン相対オフセットの保存 entries を構築する（design C1・純関数・2.5）。
///
/// スコープ別の [`PersistKey::BalloonOffset`]（X/Y）へアンカー辺基準 offset を i32 の
/// `Display` で文字列化して載せる。`offset_persist` は基準変換済み（アンカー辺基準）の
/// 物理 px オフセットであること（変換自体は後続タスクの純関数が担う）。
pub fn balloon_offset_entries(scope: u32, offset_persist: PointPx) -> Vec<(PersistKey, String)> {
    vec![
        (
            PersistKey::BalloonOffset { scope, axis: Axis::X },
            offset_persist.x.to_string(),
        ),
        (
            PersistKey::BalloonOffset { scope, axis: Axis::Y },
            offset_persist.y.to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // parse_px（寛容 parse・6.1）: 数値→Some(i32)・非数値/空→None・決定論
    // ------------------------------------------------------------------

    #[test]
    fn parse_px_numeric_string_yields_some() {
        assert_eq!(parse_px("123"), Some(123));
        assert_eq!(parse_px("0"), Some(0));
        assert_eq!(parse_px("-5"), Some(-5));
        // 仮想スクリーン絶対座標は大きな値・負値もある（1.7）。
        assert_eq!(parse_px("1920"), Some(1920));
        assert_eq!(parse_px("-1080"), Some(-1080));
    }

    #[test]
    fn parse_px_non_numeric_or_empty_yields_none() {
        assert_eq!(parse_px(""), None, "空文字列は値なし");
        assert_eq!(parse_px("abc"), None, "非数値は値なし");
        assert_eq!(parse_px("12.5"), None, "小数は i32 でないため値なし");
        assert_eq!(parse_px("9999999999999999999"), None, "i32 溢れは値なし");
        assert_eq!(parse_px("   "), None, "空白のみは値なし");
    }

    #[test]
    fn parse_px_tolerates_surrounding_whitespace() {
        assert_eq!(parse_px(" 42 "), Some(42));
    }

    #[test]
    fn parse_px_is_deterministic() {
        for s in ["777", "-3", "x", ""] {
            assert_eq!(parse_px(s), parse_px(s), "同一入力→同一出力: {s:?}");
        }
    }

    // ------------------------------------------------------------------
    // char_pos_entries（保存 entries・1.6）: WindowPos X/Y の key/値等価
    // ------------------------------------------------------------------

    #[test]
    fn char_pos_entries_builds_windowpos_x_y_pairs() {
        let entries = char_pos_entries(0, PointPx { x: 1486, y: 353 });
        assert_eq!(
            entries,
            vec![
                (PersistKey::WindowPos { scope: 0, axis: Axis::X }, "1486".to_string()),
                (PersistKey::WindowPos { scope: 0, axis: Axis::Y }, "353".to_string()),
            ]
        );
    }

    #[test]
    fn char_pos_entries_carries_scope_and_negative_values() {
        // スコープ別（1.6）＋負値（仮想デスクトップ左／上のモニタ・1.7）。
        let entries = char_pos_entries(1, PointPx { x: -400, y: -20 });
        assert_eq!(
            entries,
            vec![
                (PersistKey::WindowPos { scope: 1, axis: Axis::X }, "-400".to_string()),
                (PersistKey::WindowPos { scope: 1, axis: Axis::Y }, "-20".to_string()),
            ]
        );
    }

    // ------------------------------------------------------------------
    // balloon_offset_entries（保存 entries・2.5）: BalloonOffset X/Y の key/値等価
    // ------------------------------------------------------------------

    #[test]
    fn balloon_offset_entries_builds_balloonoffset_x_y_pairs() {
        let entries = balloon_offset_entries(0, PointPx { x: -400, y: 0 });
        assert_eq!(
            entries,
            vec![
                (PersistKey::BalloonOffset { scope: 0, axis: Axis::X }, "-400".to_string()),
                (PersistKey::BalloonOffset { scope: 0, axis: Axis::Y }, "0".to_string()),
            ]
        );
    }

    #[test]
    fn balloon_offset_entries_carries_scope() {
        let entries = balloon_offset_entries(2, PointPx { x: 336, y: 12 });
        assert_eq!(
            entries,
            vec![
                (PersistKey::BalloonOffset { scope: 2, axis: Axis::X }, "336".to_string()),
                (PersistKey::BalloonOffset { scope: 2, axis: Axis::Y }, "12".to_string()),
            ]
        );
    }

    /// 保存した値が sylphya の寛容 parse で読み戻せること（key/値等価の往復・8.1 の前哨）。
    /// entries の String は parse_px でそのまま i32 へ戻る（Display ⇄ parse 対称）。
    #[test]
    fn entries_values_round_trip_through_parse_px() {
        let pos = PointPx { x: -7, y: 1040 };
        for (_, v) in char_pos_entries(0, pos) {
            assert!(parse_px(&v).is_some(), "保存値 {v:?} は parse_px で読める");
        }
        assert_eq!(parse_px(&char_pos_entries(0, pos)[0].1), Some(pos.x));
        assert_eq!(parse_px(&char_pos_entries(0, pos)[1].1), Some(pos.y));
    }
}
