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
    /// **軸の取り違え**（非対称点ゆえ `surface_point` が `(y, x)` になり検出できる）を落とす。
    ///
    /// 一方、**×k の誤挿入・素の floor 丸めの持ち込みは k=1.0 では検出できない**——どちらも
    /// k=1.0 では恒等写像へ退化するためである（本檻の射程外）。それらの誤実装を殺すのは
    /// 任意 k（2.0・1.25）の網羅檻の責務であり、本ファイルの `scaled_five_branches_at_k2` ／
    /// `scaled_five_branches_at_k125` 以下が担う。
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

    // ------------------------------------------------------------------
    // 任意 k（k≠1.0）の決定論檻（要件 3.1/3.2/3.3/3.5・2.1/2.3/2.4/2.5/2.6）
    //
    // 丸め規約は DD-1（画素中心逆写像）: `s(v) = ⌊((2v + 1)·den) / (2·num)⌋`。
    // 本節の期待値は**全てハードコード定数**であり、実装式を期待値側で再計算しない
    // （トートロジー禁止）。各値は手計算で導出済みで、代表値は DD-1 の
    // 「k=2: 100→50・101→50／k=5/4: 1→1・6→5」と一致する。
    // ------------------------------------------------------------------

    /// 作者基準 DPI（ukadoc 正典既定）。k を「実 DPI ÷ 作者基準 DPI」として組む分母。
    const AUTHOR_DPI: u32 = 96;

    /// k=2.0（192dpi / 96dpi → 既約 2/1）。`s(v) = ⌊(2v+1)/4⌋`（＝`⌊v/2⌋`）。
    fn k2() -> ScaleRatio {
        ScaleRatio::new(192, AUTHOR_DPI).expect("192/96 は構築可能")
    }

    /// k=1.25（120dpi / 96dpi → 既約 5/4）。`s(v) = ⌊(8v+4)/10⌋`（割り切れない縮約）。
    fn k125() -> ScaleRatio {
        ScaleRatio::new(120, AUTHOR_DPI).expect("120/96 は構築可能")
    }

    /// 既存 5 分岐檻（`point_inside_returns_region_name`／`closed_interval_edges_and_corners`）と
    /// 同一の fixture。k 空間版はこの矩形の逆像を物理 px で狙う。
    fn head_and_bust() -> SurfaceMaster {
        master_with(vec![
            coll(0, 93, 62, 271, 130, "Head"),
            coll(1, 133, 270, 229, 326, "Bust"),
        ])
    }

    /// 期待値表 1 件の検証補助（`surface_point` と `region` を同時に固定する）。
    ///
    /// `surface_point` を毎件固定することで、「region は偶然一致するが縮約値が違う」
    /// 誤実装（素の floor 丸めなど）も逃さない。
    fn assert_scaled(
        m: &SurfaceMaster,
        k: ScaleRatio,
        client: (i64, i64),
        want_surface: (i64, i64),
        want_region: Option<&str>,
        what: &str,
    ) {
        let (x, y) = client;
        let got = hit_region_scaled(m, x, y, k, RegionPriority::Painter);
        assert_eq!(
            got.surface_point, want_surface,
            "surface_point: {what} / client=({x},{y})"
        );
        assert_eq!(
            got.region, want_region,
            "region: {what} / client=({x},{y}) → surface={:?}",
            got.surface_point
        );
    }

    /// 要件 3.3（5 分岐 × k=2.0）＋ 3.1/1.1/1.2/1.3/2.3: 領域内／別領域内／背景／
    /// 矩形境界の内側 1px／外側 1px を、窓 client 物理 px の点で網羅する。
    ///
    /// 境界画素は「縮約後に境界へ**乗る**／**外れる**」物理点を明示的に選んである
    /// （design Testing Strategy 項目 1 の但し書き）——k=2.0 では物理 x=186・187 が共に
    /// surface 93（Head 左端＝当たり）へ落ち、x=185 が surface 92（外れ）へ落ちる。
    /// これにより閉区間の含端が k 空間でも保存されていることが 1px 精度で固定される。
    #[test]
    fn scaled_five_branches_at_k2() {
        let m = head_and_bust();
        let k = k2();
        for (client, surface, region, what) in [
            // (1) 領域内: Head の内部。
            ((360, 192), (180, 96), Some("Head"), "領域内（Head）"),
            // (2) 別領域内: Bust の内部（同一 x 列でも y で別領域になる）。
            ((360, 600), (180, 300), Some("Bust"), "別領域内（Bust）"),
            // (3) 背景: Head と Bust の隙間／原点。
            ((360, 400), (180, 200), None, "背景（Head と Bust の間）"),
            ((0, 0), (0, 0), None, "背景（原点）"),
            // (4) 矩形境界（縮約後に境界へ乗る＝当たり）。
            ((186, 124), (93, 62), Some("Head"), "境界に乗る（左上隅）"),
            ((187, 125), (93, 62), Some("Head"), "境界に乗る（同一元画素の別物理点）"),
            ((543, 261), (271, 130), Some("Head"), "境界に乗る（右下隅）"),
            // (4') 境界の内側 1px。
            ((188, 126), (94, 63), Some("Head"), "内側 1px（左上）"),
            ((541, 259), (270, 129), Some("Head"), "内側 1px（右下）"),
            // (5) 境界の外側 1px（縮約後に境界を外れる）。
            ((185, 124), (92, 62), None, "外側 1px（左端の 1 つ外・x）"),
            ((186, 123), (93, 61), None, "外側 1px（上端の 1 つ外・y）"),
            ((544, 261), (272, 130), None, "外側 1px（右端の 1 つ外・x）"),
            ((543, 262), (271, 131), None, "外側 1px（下端の 1 つ外・y）"),
        ] {
            assert_scaled(&m, k, client, surface, region, what);
        }
    }

    /// 要件 3.3/3.5（5 分岐 × k=1.25）: 割り切れない縮約（5/4）でも 5 分岐が同一の意味を保つ。
    ///
    /// k=5/4 では 1 元画素へ落ちる物理画素が 1〜2 列で不均等になるため、境界の内外が
    /// 物理空間で非自明になる——例えば surface 93（Head 左端）へ落ちる物理列は x=116 **のみ**で、
    /// x=115 は surface 92（外れ）、x=117 は surface 94（内側 1px）である。素の floor 丸め
    /// （`⌊v·den/num⌋`）ならば x=116 は 92 となり本檻は落ちる（DD-1 の +1/2 中心補正の檻）。
    #[test]
    fn scaled_five_branches_at_k125() {
        let m = head_and_bust();
        let k = k125();
        for (client, surface, region, what) in [
            // (1) 領域内。
            ((120, 90), (96, 72), Some("Head"), "領域内（Head）"),
            // (2) 別領域内。
            ((225, 375), (180, 300), Some("Bust"), "別領域内（Bust）"),
            // (3) 背景。
            ((225, 250), (180, 200), None, "背景（Head と Bust の間）"),
            ((0, 0), (0, 0), None, "背景（原点）"),
            // (4) 矩形境界（縮約後に境界へ乗る）。
            ((116, 77), (93, 62), Some("Head"), "境界に乗る（左上隅）"),
            ((116, 78), (93, 62), Some("Head"), "境界に乗る（y=77/78 が同一元画素 62）"),
            ((339, 163), (271, 130), Some("Head"), "境界に乗る（右下隅）"),
            // (4') 境界の内側 1px。
            ((117, 79), (94, 63), Some("Head"), "内側 1px（左上）"),
            ((338, 161), (270, 129), Some("Head"), "内側 1px（右下）"),
            // (5) 境界の外側 1px。
            ((115, 77), (92, 62), None, "外側 1px（左端の 1 つ外・x）"),
            ((116, 76), (93, 61), None, "外側 1px（上端の 1 つ外・y）"),
            ((340, 163), (272, 130), None, "外側 1px（右端の 1 つ外・x）"),
            ((339, 164), (271, 131), None, "外側 1px（下端の 1 つ外・y）"),
        ] {
            assert_scaled(&m, k, client, surface, region, what);
        }
    }

    /// 要件 3.2（正本檻）: k=2.0・client 点 (100,100) は **surface (50,50) として照合**される。
    ///
    /// 矩形は (30,30)-(70,70) ゆえ、縮約を省いた素通し（(100,100)）は領域外＝`None` へ落ちる。
    /// 一方 ×k の誤挿入（(200,200)）は `None` ではなく**別の領域 `Some("Foot")`** へ解決する
    /// ——第 2 矩形 `Foot` (200,200)-(260,260) は、×k mutant に「該当なし」ではなく
    /// **区別可能な誤答**を出させるために fixture へ置いてある（誤答が `None` だと、単に
    /// 外れただけの他の誤実装と見分けが付かない）。いずれの誤実装でも本檻の
    /// `Some("Torso")` 期待は破れる＝非トートロジー。
    #[test]
    fn scaled_k2_client_100_100_equals_surface_50_50() {
        let m = master_with(vec![
            coll(0, 30, 30, 70, 70, "Torso"),
            coll(1, 200, 200, 260, 260, "Foot"),
        ]);
        let got = hit_region_scaled(&m, 100, 100, k2(), RegionPriority::Painter);
        assert_eq!(got.surface_point, (50, 50), "R3.2: (100,100) ÷ 2 = (50,50)");
        assert_eq!(got.region, Some("Torso"), "R3.2: surface (50,50) の領域");
        assert_eq!(
            got.region,
            hit_region(&m, 50, 50, RegionPriority::Painter),
            "R3.2: k=2.0 の client (100,100) は surface (50,50) の無縮約照合と同一結果"
        );
        // 反トートロジーの明示: 素通し (100,100) は `None`・×k (200,200) は別領域
        // `Some("Foot")` となり、いずれも上の `Some("Torso")` 期待とは一致しない。
        assert_eq!(hit_region(&m, 100, 100, RegionPriority::Painter), None);
        assert_eq!(hit_region(&m, 200, 200, RegionPriority::Painter), Some("Foot"));
    }

    /// 要件 3.5（丸め期待値の固定・DD-1 代表値）: 割り切れない縮約と、整数倍 k で
    /// 画素の中間に落ちる奇数座標の丸め結果を期待値として固定する。
    ///
    /// - k=2.0: 100→50・**101→50**（奇数 client 座標が同一元画素へ落ちる）
    /// - k=5/4: **1→1**・**6→5**（素の floor なら 0・4 になる値）
    ///
    /// 矩形は「元画素 50 のみ」「元画素 1 のみ」「元画素 5 のみ」を含む 1px 幅の帯にしてあり、
    /// 丸め結果が 1 でもずれれば当たり／外れが反転する。
    #[test]
    fn scaled_rounding_expectations_are_pinned() {
        // 元画素 x=50 のみを含む 1px 幅の縦帯（y は十分広く取る）。
        let band50 = master_with(vec![coll(0, 50, 0, 50, 1000, "Band50")]);
        let k = k2();
        assert_scaled(&band50, k, (100, 0), (50, 0), Some("Band50"), "k=2: 100→50");
        assert_scaled(&band50, k, (101, 0), (50, 0), Some("Band50"), "k=2: 101→50");
        // 直前・直後の物理列は別の元画素（49・51）へ落ちる＝帯の外。
        assert_scaled(&band50, k, (99, 0), (49, 0), None, "k=2: 99→49");
        assert_scaled(&band50, k, (102, 0), (51, 0), None, "k=2: 102→51");

        // k=5/4 の代表値: 1→1（素の floor なら 0）・6→5（素の floor なら 4）。
        let band1 = master_with(vec![coll(0, 1, 0, 1, 1000, "Band1")]);
        let band5 = master_with(vec![coll(0, 5, 0, 5, 1000, "Band5")]);
        let q = k125();
        assert_scaled(&band1, q, (1, 0), (1, 0), Some("Band1"), "k=5/4: 1→1");
        assert_scaled(&band1, q, (0, 0), (0, 0), None, "k=5/4: 0→0");
        assert_scaled(&band5, q, (6, 0), (5, 0), Some("Band5"), "k=5/4: 6→5");
        assert_scaled(&band5, q, (7, 0), (6, 0), None, "k=5/4: 7→6");
        // 素の floor（⌊v·4/5⌋）ならば 1→0・6→4 となり、上の 2 件が外れる。
        assert_scaled(&band1, q, (2, 0), (2, 0), None, "k=5/4: 2→2");
    }

    /// 要件 2.4（重なり優先の k 非依存）: k≠1.0 でも画家則（後定義が手前）が保存される。
    ///
    /// [`hit_region_scaled`] は照合を [`hit_region`] へ完全委譲するため構造的に保存されるが、
    /// 「縮約後の点が重なり域へ落ちる」ことを含めて 1 本の檻で固定する。
    #[test]
    fn scaled_overlap_prefers_later_defined_at_nonidentity_k() {
        let m = master_with(vec![
            coll(1, 0, 0, 100, 100, "A"),
            coll(2, 50, 50, 150, 150, "B"),
        ]);
        // k=2.0: 重なり域（surface 75,75）は後定義 B。
        assert_scaled(&m, k2(), (150, 150), (75, 75), Some("B"), "k=2: 重なり域は B");
        assert_scaled(&m, k2(), (151, 151), (75, 75), Some("B"), "k=2: 奇数座標も同一元画素");
        // 重なりの外は各々の領域（重なり優先が「常に B」に潰れていないことの非空虚性）。
        assert_scaled(&m, k2(), (40, 40), (20, 20), Some("A"), "k=2: A 単独域");
        assert_scaled(&m, k2(), (280, 280), (140, 140), Some("B"), "k=2: B 単独域");
        // k=1.25 でも同一の重なり域へ落ちる（(8·94+4)/10 = 75）。
        assert_scaled(&m, k125(), (94, 94), (75, 75), Some("B"), "k=5/4: 重なり域は B");
        assert_scaled(&m, k125(), (25, 25), (20, 20), Some("A"), "k=5/4: A 単独域");
    }

    /// 要件 2.6（反転/退化矩形 × k≠1.0）: 反転矩形は k によらず何にも当たらない。
    ///
    /// 非空虚性のため、同じ物理点が**正立矩形なら当たる**ことを対照で固定する
    /// （「縮約がおかしくて外れている」のではないことの証拠）。
    #[test]
    fn scaled_inverted_rect_hits_nothing_at_nonidentity_k() {
        let inverted = master_with(vec![
            coll(0, 100, 100, 0, 0, "InvBoth"),
            coll(1, 0, 100, 100, 0, "InvY"),
        ]);
        let upright = master_with(vec![coll(0, 0, 0, 100, 100, "Ok")]);

        // k=2.0: client (100,100) → surface (50,50)。
        assert_scaled(&inverted, k2(), (100, 100), (50, 50), None, "k=2: 反転は当たらない");
        assert_scaled(&upright, k2(), (100, 100), (50, 50), Some("Ok"), "対照: 正立なら当たる");
        // k=1.25: client (63,63) → surface (50,50)（(8·63+4)/10 = 50）。
        assert_scaled(&inverted, k125(), (63, 63), (50, 50), None, "k=5/4: 反転は当たらない");
        assert_scaled(&upright, k125(), (63, 63), (50, 50), Some("Ok"), "対照: 正立なら当たる");
    }

    /// 要件 2.5（負値・窓外・i64 極値 × k≠1.0）: panic せず定義された結果を返す。
    ///
    /// 負値の縮約は Euclid 除算＝**床方向**である。2 つの k の負値点は**別々の誤実装を殺す**
    /// ——実測どおりの帰属を以下に記す（片方を「冗長」として刈り取ると当該 mutant が黙って通る）。
    ///
    /// - **0 方向切り捨て（Euclid でなく素の `/`）の判別**は **k=2 の `(-7, -13)`** が担う:
    ///   DD-1 は `(-4, -7)`、切り捨てなら `(-3, -6)` になる。k=5/4 では x=-7（DD-1 -6・
    ///   切り捨て -5）が判別する一方、**y=-13 は切り捨てを判別しない**——分子
    ///   `(2·(-13)+1)·4 = -100` が `2·num = 10` で割り切れ、Euclid と切り捨てがともに -10 に
    ///   なるためである。
    /// - **素の floor 丸め `⌊v·den/num⌋`（DD-1 の +1/2 中心補正を落とす mutant）の判別**は
    ///   逆に **k=5/4 の y=-13** が担う: 同 mutant は `⌊-52/5⌋ = -11` を返し、DD-1 の -10 と
    ///   食い違う。k=2 の負値点はこの mutant を判別しない（k=2 では両者が一致する）。
    ///
    /// i64 極値は `unscale_coord` の i128 中間＋飽和縮小の経路を通る（本 fixture では
    /// 飽和は起きない領域）。
    #[test]
    fn scaled_negative_and_out_of_window_points_are_defined_at_nonidentity_k() {
        let m = head_and_bust();
        // k=2.0。
        assert_scaled(&m, k2(), (-7, -13), (-4, -7), None, "k=2: 負値（床方向）");
        assert_scaled(&m, k2(), (5000, 5000), (2500, 2500), None, "k=2: 窓外");
        assert_scaled(
            &m,
            k2(),
            (i64::MIN, 0),
            (-4_611_686_018_427_387_904, 0),
            None,
            "k=2: i64 極値（panic なし）",
        );
        // k=1.25。
        assert_scaled(&m, k125(), (-7, -13), (-6, -10), None, "k=5/4: 負値（床方向）");
        assert_scaled(&m, k125(), (5000, 5000), (4000, 4000), None, "k=5/4: 窓外");
        assert_scaled(
            &m,
            k125(),
            (i64::MIN, 0),
            (-7_378_697_629_483_820_646, 0),
            None,
            "k=5/4: i64 極値（panic なし）",
        );
    }

    /// 要件 2.1（決定性の k≠1.0 版・既存 `deterministic_repeated_calls` の流儀）:
    /// 同一 `(master, x, y, k, priority)` の反復呼出は常に同一の `ScaledHit` を返す。
    #[test]
    fn scaled_deterministic_repeated_calls_at_nonidentity_k() {
        let m = head_and_bust();
        for (k, client, surface, region) in [
            (k2(), (360, 192), (180, 96), Some("Head")),
            (k2(), (360, 600), (180, 300), Some("Bust")),
            (k2(), (186, 124), (93, 62), Some("Head")),
            (k125(), (120, 90), (96, 72), Some("Head")),
            (k125(), (116, 77), (93, 62), Some("Head")),
            (k125(), (225, 250), (180, 200), None),
        ] {
            let (x, y) = client;
            let first = hit_region_scaled(&m, x, y, k, RegionPriority::Painter);
            for _ in 0..5 {
                assert_eq!(
                    hit_region_scaled(&m, x, y, k, RegionPriority::Painter),
                    first,
                    "反復呼出で結果が変わってはならない: client=({x},{y})"
                );
            }
            assert_eq!(first.surface_point, surface, "client=({x},{y})");
            assert_eq!(first.region, region, "client=({x},{y})");
        }
    }
}
