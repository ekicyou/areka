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

use super::resolver::{Anchor, PointPx, SizePx};

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

/// バルーン相対オフセットのアンカー辺基準点（**char 左上相対**・物理 px・design C1）。
///
/// サーフェス寸法変動に対して不変な基準辺を、char 窓左上を原点とした相対点で返す:
/// - `Bottom`（下端吸着）: `(0, h)` — 下端。左上ではなく下端を基準にすることで、
///   サーフェス高さが変わっても下端からの距離が保たれる（2.2）。
/// - `Top`・`Left`: `(0, 0)` — 左上。
/// - `Right`: `(w, 0)` — 右上（右端）。
/// - `Free`: `(0, 0)` — 左上（縮退・アンカー辺なし。往復恒等のため 0 で固定・檻固定）。
fn anchor_edge_basis(anchor: Anchor, char_size: SizePx) -> PointPx {
    match anchor {
        Anchor::Bottom => PointPx { x: 0, y: char_size.h },
        Anchor::Right => PointPx { x: char_size.w, y: 0 },
        // Top・Left・Free は左上基準（Free は縮退・6.x/2.2）。
        Anchor::Top | Anchor::Left | Anchor::Free => PointPx { x: 0, y: 0 },
    }
}

/// バルーン相対オフセットを**保存方向**へ基準変換する（design C1・純関数・2.2/2.5）。
///
/// `offset_tl` は char 窓左上を基準に採ったセッション内表現。保存はサーフェス寸に
/// 不変なアンカー辺基準へ移す: `persist = offset_tl − 基準点(char 左上相対)`。
/// これにより下端吸着キャラでもサーフェス高さ変動でオフセットがずれない（2.2）。
pub fn balloon_offset_to_persist(anchor: Anchor, offset_tl: PointPx, char_size: SizePx) -> PointPx {
    let basis = anchor_edge_basis(anchor, char_size);
    PointPx {
        x: offset_tl.x - basis.x,
        y: offset_tl.y - basis.y,
    }
}

/// バルーン相対オフセットを**復元方向**へ基準変換する（design C1・純関数・2.2/8.5）。
///
/// [`balloon_offset_to_persist`] の厳密な逆で、**現在の** `char_size` で基準点を
/// 足し戻す: `offset_tl = persisted + 基準点(char 左上相対)`。保存時と復元時で
/// サーフェス高さが異なっても、下端基準の相対関係が保たれる（8.5）。
///
/// 不変条件: `balloon_offset_from_persist(a, balloon_offset_to_persist(a, o, s), s) == o`。
pub fn balloon_offset_from_persist(
    anchor: Anchor,
    persisted: PointPx,
    char_size: SizePx,
) -> PointPx {
    let basis = anchor_edge_basis(anchor, char_size);
    PointPx {
        x: persisted.x + basis.x,
        y: persisted.y + basis.y,
    }
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

    // ------------------------------------------------------------------
    // balloon_offset_to_persist / _from_persist（アンカー辺基準変換・2.2/2.5・design C1）
    //   基準点（char 左上相対）: Bottom=(0,h)・Top/Left=(0,0)・Right=(w,0)・Free=(0,0)
    //   persist = offset_tl − 基準点(相対) ／ from_persist = persisted + 基準点(相対)
    // ------------------------------------------------------------------

    const ALL_ANCHORS: [Anchor; 5] = [
        Anchor::Top,
        Anchor::Bottom,
        Anchor::Left,
        Anchor::Right,
        Anchor::Free,
    ];

    /// 各アンカーで基準点（char 左上相対）が design C1 の規定どおりになる。
    #[test]
    fn to_persist_subtracts_the_correct_anchor_edge_basis() {
        let size = SizePx { w: 300, h: 500 };
        let offset_tl = PointPx { x: 40, y: 70 };

        // Bottom: 基準 (0, h) → persist = (40−0, 70−500) = (40, -430)
        assert_eq!(
            balloon_offset_to_persist(Anchor::Bottom, offset_tl, size),
            PointPx { x: 40, y: -430 }
        );
        // Top: 基準 (0, 0) → persist = offset_tl
        assert_eq!(
            balloon_offset_to_persist(Anchor::Top, offset_tl, size),
            PointPx { x: 40, y: 70 }
        );
        // Left: 基準 (0, 0) → persist = offset_tl
        assert_eq!(
            balloon_offset_to_persist(Anchor::Left, offset_tl, size),
            PointPx { x: 40, y: 70 }
        );
        // Right: 基準 (w, 0) → persist = (40−300, 70−0) = (-260, 70)
        assert_eq!(
            balloon_offset_to_persist(Anchor::Right, offset_tl, size),
            PointPx { x: -260, y: 70 }
        );
        // Free: 基準 (0, 0)（縮退・檻固定）→ persist = offset_tl
        assert_eq!(
            balloon_offset_to_persist(Anchor::Free, offset_tl, size),
            PointPx { x: 40, y: 70 }
        );
    }

    /// from_persist は現在の char_size で基準点を足し戻す（to_persist の逆・現寸使用）。
    #[test]
    fn from_persist_adds_back_the_anchor_edge_basis_with_current_size() {
        let size = SizePx { w: 300, h: 500 };
        // Bottom: persisted (40, -430) + 基準 (0, 500) = (40, 70)
        assert_eq!(
            balloon_offset_from_persist(Anchor::Bottom, PointPx { x: 40, y: -430 }, size),
            PointPx { x: 40, y: 70 }
        );
        // Right: persisted (-260, 70) + 基準 (300, 0) = (40, 70)
        assert_eq!(
            balloon_offset_from_persist(Anchor::Right, PointPx { x: -260, y: 70 }, size),
            PointPx { x: 40, y: 70 }
        );
    }

    /// design C1 の不変条件（Invariant）: 全アンカー × 複数寸法で往復恒等。
    /// `from_persist(a, to_persist(a, o, s), s) == o`。
    #[test]
    fn round_trip_identity_holds_for_all_anchors_and_sizes() {
        let sizes = [
            SizePx { w: 1, h: 1 },
            SizePx { w: 128, h: 256 },
            SizePx { w: 300, h: 500 },
            SizePx { w: 1024, h: 64 },
        ];
        let offsets = [
            PointPx { x: 0, y: 0 },
            PointPx { x: -400, y: 0 },
            PointPx { x: 40, y: 70 },
            PointPx { x: -260, y: -430 },
        ];
        for anchor in ALL_ANCHORS {
            for size in sizes {
                for offset in offsets {
                    let persisted = balloon_offset_to_persist(anchor, offset, size);
                    let restored = balloon_offset_from_persist(anchor, persisted, size);
                    assert_eq!(
                        restored, offset,
                        "往復恒等が破れた: anchor={anchor:?} size={size:?} offset={offset:?}"
                    );
                }
            }
        }
    }

    /// 8.5 サーフェス寸不変性: 高さ h1 で保存したオフセットを、異なる高さ h2 で復元しても
    /// バルーンとアンカー辺（下端）の相対関係が保たれる（＝下端からの距離が不変）。
    ///
    /// Bottom 吸着では、下端からの縦距離 = (char 下端) − (balloon 左上 y)
    ///   = (char.y + h) − (char.y + offset_tl.y) = h − offset_tl.y。
    /// persist 値の y はまさに `offset_tl.y − h`（= −(下端からの距離)）であり寸法非依存。
    /// よって別の高さ h2 で復元した offset_tl'.y は、その h2 に対して同じ「下端からの距離」を与える。
    #[test]
    fn balloon_offset_is_invariant_under_char_surface_height_change_bottom() {
        // 保存時: 高さ h1。ユーザーはバルーンを char 左上から下 70px の位置へ置いた。
        let h1 = 500;
        let size_h1 = SizePx { w: 300, h: h1 };
        let offset_tl_saved = PointPx { x: 40, y: 70 };
        // 保存時の「下端からの距離」（上向き正）: h1 − offset_tl.y。
        let distance_from_bottom_saved = h1 - offset_tl_saved.y; // 430

        let persisted = balloon_offset_to_persist(Anchor::Bottom, offset_tl_saved, size_h1);

        // 復元時: 異なる高さ h2（サーフェスが縮んだ／伸びた）。
        let h2 = 620;
        let size_h2 = SizePx { w: 300, h: h2 };
        let offset_tl_restored =
            balloon_offset_from_persist(Anchor::Bottom, persisted, size_h2);

        // 復元後の「下端からの距離」は保存時と一致する（＝下端基準の関係が保たれた）。
        let distance_from_bottom_restored = h2 - offset_tl_restored.y;
        assert_eq!(
            distance_from_bottom_restored, distance_from_bottom_saved,
            "下端からの距離がサーフェス高さ変動で保たれていない"
        );

        // 反例の檻: もし左上基準（persist=offset_tl そのまま）だったら、
        // 高さが変わっても左上距離を固定してしまい下端距離はずれる——それを排除する。
        let naive_topleft_restored = persisted.y; // 左上基準なら復元 y は persist.y のまま
        assert_ne!(
            h2 - naive_topleft_restored,
            distance_from_bottom_saved,
            "この寸法差では左上基準は下端距離を保てないはず（テスト前提の健全性）"
        );
    }
}
