//! 正規化 Surface 定義の公開形: `SurfaceMaster` / `NormalizedElement` / `Transform`。
//!
//! 下流（seriko・collision-geometry）が再パース・再展開なしに消費できる正規化定義を提供する。
//! element（レイヤ・画像パス・座標）に加え collisions・animations・`animation-sort`／
//! `collision-sort` の順序キーを保持する。element 配置は変換行列（`Transform`）で表現し、
//! X,Y のみの平行移動を単位行列の特例として扱う（回転・拡縮は行列表現に予約）。

use crate::method::ComposeMethod;
use bevy_ecs::component::Component;

/// 2D 変換（M1 実挙動は恒等＋平行移動のみ。回転・拡縮は M2 予約の口）。
///
/// 2x2 線形部を整数で保持する。M1 では 2x2 部は常に単位行列に固定され、
/// 平行移動成分 `tx`/`ty` のみが有効値を持つ（転記 x,y の行列表現）。
/// 回転・拡縮のシームは 2x2 部として存在するが M1 では不活性（常に単位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transform {
    /// 2x2 線形部（行優先: [[a, b], [c, d]]）。M1 は単位固定 [[1,0],[0,1]]。
    linear: [[i64; 2]; 2],
    /// 平行移動 X。
    tx: i64,
    /// 平行移動 Y。
    ty: i64,
}

impl Transform {
    /// M1 の単位 2x2 部（恒等線形変換）。
    const IDENTITY_LINEAR: [[i64; 2]; 2] = [[1, 0], [0, 1]];

    /// 恒等変換（平行移動なし＝`translate(0, 0)`）。
    pub fn identity() -> Self {
        Transform::translate(0, 0)
    }

    /// X,Y 平行移動（単位行列＋平行移動＝転記 x,y の特例）。
    pub fn translate(x: i64, y: i64) -> Self {
        Transform {
            linear: Self::IDENTITY_LINEAR,
            tx: x,
            ty: y,
        }
    }

    /// 純平行移動（2x2 部が単位）か。M1 では常に `true`。
    pub fn is_translation(&self) -> bool {
        self.linear == Self::IDENTITY_LINEAR
    }

    /// 平行移動成分 `(x, y)`。
    pub fn offset(&self) -> (i64, i64) {
        (self.tx, self.ty)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Transform::identity()
    }
}

/// 正規化 Surface 定義（公開形・collisions/animations 保持・下流はこれを唯一の正とする）。
///
/// `elements` は layer 昇順・同 layer は登場順（下流が保つ前提）。`animations` は転記層の
/// [`Animation`] を interval/pattern ごとそのまま保持し、seriko が再利用する（1.2/1.3）。
///
/// [`Animation`]: areka_parsers::shell::Animation
///
/// `EmoWorld` の per-ghost `bevy_ecs` World では surface 1件＝entity 1件の component として
/// 常駐する（design「Domain Model」）。`surface(id)` は本型を `&SurfaceMaster` として直接返す。
#[derive(Debug, Clone, PartialEq, Component)]
pub struct SurfaceMaster {
    /// surface id。
    pub id: u32,
    /// layer 昇順・同 layer は登場順の正規化 element 群。
    pub elements: Vec<NormalizedElement>,
    /// 当たり判定領域（転記のまま）。
    ///
    /// 順序不変条件: **登場順**。`surface.append` 由来は末尾へ連結される（`fold.rs:121-122`）。
    /// 画家則（後定義が手前）はこの順序に意味論を載せる（HitCore [`hit_region`] が逆順走査で
    /// 最前面を決める・collision-geometry 要件 2.1）ため、この順序は挙動上の意味を持つ。
    ///
    /// [`hit_region`]: crate::hit::hit_region
    pub collisions: Vec<areka_parsers::shell::Collision>,
    /// SERIKO animation 群（interval/pattern を転記のまま保持）。
    pub animations: Vec<areka_parsers::shell::Animation>,
}

/// 正規化 element（レイヤ・画像パス・配置行列・合成メソッド）。
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedElement {
    /// 合成レイヤ（昇順が下から上）。
    pub layer: u32,
    /// 画像パス（atlas resolve キー・デバッグ）。
    pub path: areka_parsers::shell::ElementPath,
    /// 転記 x,y の行列表現（4.2）。
    pub transform: Transform,
    /// 合成メソッド。M1 は常に [`Overlay`]（転記契約による）。
    ///
    /// [`Overlay`]: crate::method::ComposeMethod::Overlay
    pub method: ComposeMethod,
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_parsers::shell::ElementPath;

    /// `translate` は平行移動成分を保持し、M1 では純平行移動と判定される。
    #[test]
    fn translate_offset_and_is_translation() {
        let t = Transform::translate(3, -4);
        assert_eq!(t.offset(), (3, -4));
        assert!(t.is_translation());
    }

    /// 恒等変換の平行移動成分は (0, 0)。
    #[test]
    fn identity_has_zero_offset() {
        let t = Transform::identity();
        assert_eq!(t.offset(), (0, 0));
        assert!(t.is_translation());
        assert_eq!(t, Transform::default());
    }

    /// `SurfaceMaster` / `NormalizedElement` を構築しフィールドを読み取れる。
    #[test]
    fn surface_master_construct_and_read() {
        let elem = NormalizedElement {
            layer: 0,
            path: ElementPath::new("surface0.png".to_string()),
            transform: Transform::translate(10, 20),
            method: ComposeMethod::Overlay,
        };
        let master = SurfaceMaster {
            id: 1000,
            elements: vec![elem],
            collisions: Vec::new(),
            animations: Vec::new(),
        };
        assert_eq!(master.id, 1000);
        assert_eq!(master.elements.len(), 1);
        assert_eq!(master.elements[0].layer, 0);
        assert_eq!(master.elements[0].path.as_str(), "surface0.png");
        assert_eq!(master.elements[0].transform.offset(), (10, 20));
        assert_eq!(master.elements[0].method, ComposeMethod::Overlay);
    }
}
