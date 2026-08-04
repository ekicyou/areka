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
//!
//! # DPI 非参照の宣言（維持・`collision-dpi-hittest` 要件 5.3/6.1）
//!
//! 本モジュールは **DPI を一切参照しない**（この宣言は表示スケール追従の実装後も不変である）。
//! [`hit_region_scaled`] が受け取る `k`（[`ScaleRatio`]）は **DPI そのものではなく表示比**であり、
//! 「窓の実モニタ DPI ÷ 作者基準 DPI」から k を導出する責務は上流（`areka-emo-present` の
//! 表示ターゲット）に留まる。本モジュールは与えられた k を消費するだけで、DPI 値・モニタ・
//! 窓のいずれにも問い合わせない（純関数性の維持）。
//!
//! - [`hit_region`]: サーフェス px の点を照合する**素の純照合関数**（÷k は呼び手責務・契約不変）。
//! - [`hit_region_scaled`]: 窓 client 物理 px の点を k で縮約してから [`hit_region`] へ
//!   **完全委譲**する合成純関数。重なり・反転・閉区間の意味論を再実装しない。

use crate::normalized::SurfaceMaster;
use crate::scale::ScaleRatio;

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
/// `(x, y)` は `master` と同一座標空間（サーフェス px・原点＝サーフェス画像左上）であること。
/// すなわち**呼び手が ÷k 済みの座標を渡す**（表示スケール k の吸収は本関数より上流の責務）。
/// 窓 client 物理 px の点しか持たない呼び手は、代わりに合成純関数 [`hit_region_scaled`] を
/// 使うこと——k の縮約と本関数への委譲をまとめて行う（`collision-dpi-hittest` 要件 5.3/6.1）。
///
/// 本関数自身は k を受け取らず縮約も行わない（DPI・表示比のいずれも参照しない・要件 6.1）。
/// この入出力契約は表示スケール追従の実装後も**不変**である。
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

/// ÷k 縮約済みの照合結果（`collision-dpi-hittest` 要件 1.1/1.8）。
///
/// `region` は `master` 内の collision 名への借用ゆえ、寿命は `master` に従う（割当なし）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaledHit<'a> {
    /// 当たった領域名（[`hit_region`] の戻り値そのもの・無ければ `None`）。
    pub region: Option<&'a str>,
    /// 縮約後のサーフェス px 座標（SHIORI へ配信する「ローカル座標」の正準値・要件 1.8）。
    ///
    /// 正常経路ではこの値が縮約の**唯一の生成点**であり、下流は本値を横流しするのみで
    /// 再縮約してはならない（二重縮約の構造的排除）。
    pub surface_point: (i64, i64),
}

/// 窓 client 物理 px の点を表示スケール `k` で縮約してから [`hit_region`] へ委譲する合成純関数
/// （要件 1.1/1.2/1.3・design DD-2 × DD-6）。
///
/// 「÷k の呼び忘れ」が本増分の欠陥クラスそのものであるため、縮約単体でなく
/// **縮約＋照合の合成**が公開単位（＝檻の最小単位）である。
///
/// - **縮約**: 2 軸それぞれへ [`ScaleRatio::unscale_coord`] を適用する（除算方向の丸め権威は
///   当該メソッド 1 箇所・画素中心逆写像。丸めの式を本関数へ持ち込まない・要件 2.2）。
/// - **照合**: 縮約後の点で [`hit_region`] を呼ぶだけで、閉区間の含端・画家則の重なり優先・
///   反転/退化矩形の扱いを**一切再実装しない**（完全委譲ゆえ意味論は k によらず保存される・
///   要件 2.3/2.4/2.6/6.1）。
/// - **k は DPI ではない**: k は表示比であり、DPI から k を導出する責務は上流にある
///   （モジュール doc の「DPI 非参照」宣言は本関数でも維持される）。
///
/// # Preconditions
/// `(x, y)` は当該表示ターゲットの窓 client 物理 px（k 適用済み空間）の点であること。
/// `k` は当該ターゲットへ**実際に適用中**の表示スケール（表示寸を決めた値と同一の真実源）
/// であること——DPI から独立に再導出した値を渡してはならない（要件 1.4）。
///
/// # Postconditions
/// `k == ScaleRatio::ONE` のとき `region` は `hit_region(master, x, y, priority)` と完全一致し、
/// `surface_point == (x, y)` となる（no-op 保存・要件 1.5/1.9/3.4）。
///
/// # Invariants
/// 同一 `(master, x, y, k, priority)` に対し常に同一結果（決定論・要件 2.1）。`master` を
/// 変更しない不変借用のみ。負値・窓外・i64 極値でもパニックしない（[`ScaleRatio::unscale_coord`]
/// の飽和縮小規約に依拠・要件 2.5）。
pub fn hit_region_scaled<'a>(
    master: &'a SurfaceMaster,
    x: i64,
    y: i64,
    k: ScaleRatio,
    priority: RegionPriority,
) -> ScaledHit<'a> {
    // 2 軸を縮約（丸め規約は unscale_coord の単一権威・ここでは式を持たない）。
    let sx = k.unscale_coord(x);
    let sy = k.unscale_coord(y);
    ScaledHit {
        // 照合は既存純関数へ完全委譲（重なり・反転・閉区間の意味論を再実装しない）。
        region: hit_region(master, sx, sy, priority),
        surface_point: (sx, sy),
    }
}

#[cfg(test)]
mod tests {
    use super::{hit_region, hit_region_scaled, RegionPriority};
    use crate::normalized::SurfaceMaster;
    use crate::scale::ScaleRatio;
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

    /// 要件 1.5/3.4（no-op 保存の檻・R3.4 の正本）: k=1.0 では [`hit_region_scaled`] が
    /// 既存純関数 [`hit_region`] と `region` 完全一致し、`surface_point` が入力そのものになる。
    ///
    /// 点は「領域内」「別領域内」「背景」「閉区間の端（内側/外側 1px）」「負値・窓外」を含み、
    /// 縮約が恒等でなくなる改変（丸め持ち込み・軸取り違え・×k の誤挿入）を全て落とす。
    /// 任意 k の網羅檻は本関数の射程外（Task 2.2 が担う）。
    #[test]
    fn scaled_identity_matches_hit_region_exactly() {
        let m = master_with(vec![
            coll(0, 93, 62, 271, 130, "Head"),
            coll(1, 133, 270, 229, 326, "Bust"),
        ]);
        // 96/96 のように既約化で 1/1 になる比も同じ恒等経路を通る（k の表現に依存しない）。
        for k in [ScaleRatio::ONE, ScaleRatio::new(96, 96).unwrap()] {
            for (x, y, what) in [
                (180, 96, "Head 内"),
                (180, 300, "Bust 内"),
                (0, 0, "背景"),
                (500, 500, "背景（窓外相当）"),
                (93, 62, "閉区間の左上隅"),
                (271, 130, "閉区間の右下隅"),
                (92, 61, "外側 1px"),
                (272, 131, "外側 1px"),
                (94, 63, "内側 1px"),
                (-7, -13, "負値（panic なし・R2.5）"),
                (i64::MIN, 0, "i64 極値（飽和経路でも panic しない）"),
            ] {
                let got = hit_region_scaled(&m, x, y, k, RegionPriority::Painter);
                assert_eq!(
                    got.region,
                    hit_region(&m, x, y, RegionPriority::Painter),
                    "k=1.0 の region は無縮約と完全一致すること: {what} ({x}, {y})"
                );
                assert_eq!(
                    got.surface_point,
                    (x, y),
                    "k=1.0 の surface_point は入力素通しであること: {what} ({x}, {y})"
                );
            }
        }
        // 非空虚性: 上の一致検証が「両方 None」で成立しているのではないことを固定する。
        let hit = hit_region_scaled(&m, 180, 96, ScaleRatio::ONE, RegionPriority::Painter);
        assert_eq!(hit.region, Some("Head"));
        assert_eq!(hit.surface_point, (180, 96));
    }
}
