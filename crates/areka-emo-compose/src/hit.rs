//! HitCore — 座標→当たり判定領域名を決定論で解く純関数コア（wintf 非依存・`std` のみ）。
//!
//! `SurfaceMaster.collisions`（`areka_parsers::shell::Collision` の転記列）に対し、
//! サーフェス px 座標 `(x, y)` が属する領域名を返す。α（`AlphaMask`）・`collision-sort`・
//! DPI・wintf 型を一切参照しない（design「正典確定表」C3/C8・要件 2.2/6.1/6.2）。
//!
//! - 含端規則は**閉区間**（4辺すべて含端・C2・要件 1.4）。
//! - 重なりは**画家のアルゴリズム**（後に定義された矩形が手前・C3・要件 2.1）で、
//!   `collisions` を**逆順**に走査し最初に当たった領域を返す（`blit.rs:83` の下層→上層
//!   合成と一貫）。上限 256 矩形（C6）ゆえ索引構造を持たない線形走査で足りる。

use crate::normalized::SurfaceMaster;

/// 当たり判定の重なり解決規則（型シーム・要件 2.3）。
///
/// 本 spec は `Painter` のみを実装する。SSP `collision-sort`（none/ascend/descend）の
/// 忠実解決は行わない（要件 2.2・正典確定表 C3 の意図的逸脱）。
///
/// **シームの機序**: variant を追加すると [`hit_region`] 内の**網羅 match がコンパイルエラー**
/// となり実装漏れを機械的に検出する。これを成立させるため [`hit_region`] の match に
/// `_`（ワイルドカード）アームを置いてはならない（実装制約＝レビュー担保・design
/// 「Testing Strategy」の注記どおりテストでは担保できない唯一の口）。`#[non_exhaustive]` は
/// 定義 crate 内では効かないため検出機序ではない（下流 crate に wildcard を強制する副作用が
/// あるが、現状 `Painter` 決め打ちゆえ無害）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegionPriority {
    /// 画家のアルゴリズム: 後に定義された矩形が手前（emo 合成規約 `blit.rs:83` と一貫）。
    #[default]
    Painter,
}

/// サーフェス px 座標 `(x, y)` が属する当たり判定の領域名を返す。
///
/// - 含端規則: 閉区間（`left <= x <= right && top <= y <= bottom`・正典確定表 C2・要件 1.4）。
///   反転/退化矩形（`left > right` 等）は正規化せず、そのまま比較する結果として何にも当たらない
///   （決定論的縮退・正典に正規化の規定は無いため発明しない）。
/// - 重なり: `priority` に従う（`Painter` = 後定義が手前）。`collisions` を逆順走査し最初に
///   当たった領域を返す＝画家則の等価かつ決定論的実装（要件 2.1）。
/// - α（透明画素）・`collision-sort`・DPI は参照しない（要件 6.1/6.2/2.2）。
///
/// # Preconditions
/// `(x, y)` は `master` と同一座標空間（サーフェス px・原点＝サーフェス画像左上）であること
/// （呼び手が k=1.0 契約を保証する・要件 4.3）。
///
/// # Postconditions
/// 当たりがあれば最前面矩形の [`CollisionName::as_str`] を返す。無ければ `None`（要件 1.2/1.3）。
/// 戻り値の寿命は `master` に従う（割当なし）。
///
/// # Invariants
/// 同一 `(master, x, y, priority)` に対し常に同一結果（要件 1.5）。`master` を変更しない。
///
/// 戻り値の寿命は `master` に従う（借用の生存＝`master` の生存・lifetime elision で自動導出。
/// design Service Interface が示す `<'a>` 明示形と同一契約）。
///
/// [`CollisionName::as_str`]: areka_parsers::shell::CollisionName::as_str
pub fn hit_region(
    master: &SurfaceMaster,
    x: i64,
    y: i64,
    priority: RegionPriority,
) -> Option<&str> {
    // NOTE(要件 2.3): この match に `_`（ワイルドカード）アームを置いてはならない。
    // variant 追加時に網羅漏れをコンパイルエラーで検出する唯一の機序であり、テストでは
    // 担保できない（レビュー担保・RegionPriority の doc 参照）。
    match priority {
        // 画家則: 逆順走査で最初に当たった領域を返す（後定義が手前・要件 2.1）。
        RegionPriority::Painter => master
            .collisions
            .iter()
            .rev()
            .find(|c| c.left <= x && x <= c.right && c.top <= y && y <= c.bottom)
            .map(|c| c.name.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::{hit_region, RegionPriority};
    use crate::normalized::SurfaceMaster;
    use areka_parsers::shell::{Collision, CollisionName};

    fn coll(index: u32, left: i64, top: i64, right: i64, bottom: i64, name: &str) -> Collision {
        Collision {
            index,
            left,
            top,
            right,
            bottom,
            name: CollisionName::new(name.to_string()),
        }
    }

    fn master_with(collisions: Vec<Collision>) -> SurfaceMaster {
        SurfaceMaster {
            id: 1000,
            elements: Vec::new(),
            collisions,
            animations: Vec::new(),
        }
    }

    #[test]
    fn point_inside_returns_region_name() {
        let m = master_with(vec![
            coll(0, 93, 62, 271, 130, "Head"),
            coll(1, 133, 270, 229, 326, "Bust"),
        ]);
        assert_eq!(hit_region(&m, 180, 96, RegionPriority::Painter), Some("Head"));
        assert_eq!(hit_region(&m, 180, 300, RegionPriority::Painter), Some("Bust"));
    }

    #[test]
    fn point_outside_returns_none() {
        let m = master_with(vec![
            coll(0, 93, 62, 271, 130, "Head"),
            coll(1, 133, 270, 229, 326, "Bust"),
        ]);
        assert_eq!(hit_region(&m, 0, 0, RegionPriority::Painter), None);
        assert_eq!(hit_region(&m, 500, 500, RegionPriority::Painter), None);
    }

    #[test]
    fn no_collisions_returns_none() {
        let m = master_with(Vec::new());
        assert_eq!(hit_region(&m, 100, 100, RegionPriority::Painter), None);
    }

    #[test]
    fn closed_interval_edges_and_corners() {
        let m = master_with(vec![coll(0, 93, 62, 271, 130, "Head")]);
        for (x, y) in [(93, 62), (271, 62), (93, 130), (271, 130)] {
            assert_eq!(hit_region(&m, x, y, RegionPriority::Painter), Some("Head"));
        }
        for (x, y) in [(180, 62), (180, 130), (93, 96), (271, 96)] {
            assert_eq!(hit_region(&m, x, y, RegionPriority::Painter), Some("Head"));
        }
        for (x, y) in [(92, 96), (272, 96), (180, 61), (180, 131), (92, 61)] {
            assert_eq!(hit_region(&m, x, y, RegionPriority::Painter), None);
        }
    }

    #[test]
    fn overlap_later_defined_wins() {
        let m = master_with(vec![
            coll(1, 0, 0, 100, 100, "A"),
            coll(2, 50, 50, 150, 150, "B"),
        ]);
        assert_eq!(hit_region(&m, 75, 75, RegionPriority::Painter), Some("B"));
    }

    #[test]
    fn inverted_or_degenerate_rect_hits_nothing() {
        let m = master_with(vec![
            coll(0, 100, 100, 0, 0, "InvBoth"),
            coll(1, 0, 100, 100, 0, "InvY"),
        ]);
        assert_eq!(hit_region(&m, 50, 50, RegionPriority::Painter), None);
    }

    #[test]
    fn duplicate_name_returns_frontmost() {
        let m = master_with(vec![
            coll(1, 0, 0, 100, 100, "Hand"),
            coll(2, 50, 50, 150, 150, "Hand"),
        ]);
        assert_eq!(hit_region(&m, 75, 75, RegionPriority::Painter), Some("Hand"));
    }

    #[test]
    fn fold_output_append_wins_painter() {
        use areka_parsers::shell::{AppendTarget, DefRef, Shell, Surface, SurfaceAppend};

        let base = Surface {
            id: 1000,
            targets: vec![AppendTarget::Single(1000)],
            elements: Vec::new(),
            collisions: vec![coll(0, 0, 0, 100, 100, "Base")],
            animations: Vec::new(),
        };
        let ap = SurfaceAppend {
            targets: vec![AppendTarget::Single(1000)],
            elements: Vec::new(),
            collisions: vec![coll(1, 50, 50, 150, 150, "Appended")],
            animations: Vec::new(),
        };
        let shell = Shell {
            surfaces: vec![base],
            appends: vec![ap],
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions: vec![DefRef::Surface(0), DefRef::Append(0)],
        };
        let world = crate::world::EmoWorld::build(&shell);
        let master = world.surface(1000).expect("surface 1000 は fold 済み");
        assert_eq!(
            hit_region(master, 75, 75, RegionPriority::Painter),
            Some("Appended")
        );
    }

    #[test]
    fn deterministic_repeated_calls() {
        let m = master_with(vec![coll(0, 93, 62, 271, 130, "Head")]);
        let first = hit_region(&m, 180, 96, RegionPriority::Painter);
        for _ in 0..5 {
            assert_eq!(hit_region(&m, 180, 96, RegionPriority::Painter), first);
        }
        assert_eq!(first, Some("Head"));
    }
}
