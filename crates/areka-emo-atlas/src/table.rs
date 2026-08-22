//! 成果物契約型（正本）: `AtlasTable` / `AtlasEntry` / `AtlasKey` / `AtlasPage`。
//!
//! 設計決定 **D3**（要件 **R6**）。
//!
//! 本層が emo-compose と共有する成果物契約を**正本として定義**する。識別子は二層で、
//! ランタイムキー＝`ElementId(u32)`（密 index・決定的採番・毎フレーム O(1) 引き）／
//! ソースキー＝`AtlasKey{ set, rel_path }`（無改変相対パス・重複排除／golden／
//! デバッグ逆引き用にテーブル保持）。空エントリは
//! `AtlasEntry.placement: Option<Placement>`（`None`＝転写スキップ）で表現する。
//! 頁バッファは `AtlasPage{ bytes: Arc<[u8]>, width, height, stride }`
//! （premultiplied BGRA・stride 明示）。
//!
use std::collections::HashMap;
use std::sync::Arc;

/// ランタイムキー＝密 index（ECS エンティティ ID と同じ発想）。
/// (SetId 昇順, 相対パス昇順) で bake 時に決定的採番（D7・golden 安定）。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ElementId(pub u32);

/// SurfaceSet の序数（出所の識別・重複排除キーの一部）。
/// （設計では ManifestDeriver 節に記載されるが、`AtlasKey` が含むため契約層 table.rs に
///  定義し、manifest.rs〔task 2.1〕はここから import する。）
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SetId(pub u32);

/// ソースキー＝出所セット＋無改変相対パス（1.5・環境非依存・デバッグ逆引き用に保持）。
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct AtlasKey {
    /// shell / balloon 等の出所（同名ファイル誤同一視を防ぐ）。
    pub set: SetId,
    /// ElementPath 無改変（サブディレクトリ含む）。
    pub rel_path: String,
}

/// 幾何プリミティブ（wintf::types と別定義・純粋層自前）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

/// 幾何プリミティブ（寸法）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

/// 幾何プリミティブ（矩形）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// アトラス 1 エントリ。`placement=None` は転写スキップ（全透明・6.2/4.4）。
#[derive(Clone, Debug)]
pub struct AtlasEntry {
    /// 原寸（4.2/4.5）。
    pub original: Size,
    /// `None`＝空エントリ（転写スキップ）。
    pub placement: Option<Placement>,
}

/// アトラス頁への配置情報。
#[derive(Clone, Debug)]
pub struct Placement {
    /// 頁番号（6.1）。
    pub page: u32,
    /// 頁内 UV（padding 非包含・5.3）。
    pub uv_rect: Rect,
    /// 原画像内 bbox 左上（4.2/4.5）。
    pub trim_offset: Point,
}

/// 頁バッファ（premultiplied BGRA・stride 明示・`Arc` 共有・6.3/6.4）。
#[derive(Clone, Debug)]
pub struct AtlasPage {
    pub width: u32,
    pub height: u32,
    /// stride を明示（6.3）。
    pub stride: u32,
    /// premultiplied BGRA・共有参照（6.4）。
    pub bytes: Arc<[u8]>,
}

/// 索引表＋頁群（bake の成果物・channel 非依存・6.5）。
///
/// `entries`/`keys` は `ElementId`（= index）で整列した密 `Vec`（毎フレーム O(1)）。
/// `bake` が一度構築し以後不変（immutable snapshot）。`Arc` 共有ゆえ `Clone` は安価。
#[derive(Clone, Debug)]
pub struct AtlasTable {
    /// index == ElementId.0（デバッグ逆引き・ツリーダンプ用）。
    keys: Arc<[AtlasKey]>,
    /// index == ElementId.0（ランタイム正準参照）。
    entries: Arc<[AtlasEntry]>,
    /// 構築時の一度きり用（(set, rel_path)→ElementId）。
    resolve: HashMap<AtlasKey, ElementId>,
    pages: Arc<[AtlasPage]>,
}

impl AtlasTable {
    /// 索引表を構築する（`bake` とテストの入口）。
    ///
    /// `keys` と `entries` はともに `ElementId`（= index）で整列した密 `Vec` であり、
    /// `keys.len() == entries.len()` を前提とする（両者は同一 ElementId で対応）。
    /// `resolve` 逆引き表は `keys[i].clone() → ElementId(i)` として内部で構築する。
    ///
    /// # Panics
    /// `keys.len() != entries.len()` のとき panic する（契約違反・呼び出し側のバグ）。
    pub fn new(keys: Vec<AtlasKey>, entries: Vec<AtlasEntry>, pages: Vec<AtlasPage>) -> Self {
        assert_eq!(
            keys.len(),
            entries.len(),
            "AtlasTable::new: keys.len() ({}) != entries.len() ({}) — both are indexed by ElementId",
            keys.len(),
            entries.len(),
        );
        let resolve = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), ElementId(i as u32)))
            .collect();
        Self {
            keys: keys.into(),
            entries: entries.into(),
            resolve,
            pages: pages.into(),
        }
    }

    /// 【ランタイム正準・毎フレーム】`ElementId`→エントリ（O(1) Vec index・6.1）。
    ///
    /// `id` は本テーブルの `resolve`／列挙由来のみを前提とする（密 index の契約）。
    ///
    /// # Panics
    /// 範囲外 `id`（本テーブル由来でない）のとき panic する（契約不変違反）。
    pub fn entry(&self, id: ElementId) -> &AtlasEntry {
        &self.entries[id.0 as usize]
    }

    /// 【構築時のみ】(set, 相対パス)→`ElementId`（emo-compose が自ツリー構築時に一度
    /// resolve し、以後 `ElementId` を保持）。既知は `Some`・未知は `None`。
    pub fn resolve(&self, set: SetId, rel_path: &str) -> Option<ElementId> {
        // AtlasKey は借用キーで引けないため一時キーを組む（構築時の一度きりゆえ許容）。
        let key = AtlasKey {
            set,
            rel_path: rel_path.to_owned(),
        };
        self.resolve.get(&key).copied()
    }

    /// 【デバッグ】`ElementId`→ソースキー逆引き（画像生成ツリーのダンプ・
    /// 「頁の矩形→元画像」追跡）。
    ///
    /// # Panics
    /// 範囲外 `id`（本テーブル由来でない）のとき panic する（契約不変違反）。
    pub fn key(&self, id: ElementId) -> &AtlasKey {
        &self.keys[id.0 as usize]
    }

    /// エントリ数（== keys 数）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// エントリが空か。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 頁バッファ（6.3）。範囲外は `None`。
    pub fn page(&self, index: u32) -> Option<&AtlasPage> {
        self.pages.get(index as usize)
    }

    /// 全頁スライス。
    pub fn pages(&self) -> &[AtlasPage] {
        &self.pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 頁バッファを手組みで生成（premultiplied BGRA・4byte/px）。
    fn page(width: u32, height: u32) -> AtlasPage {
        let stride = width * 4;
        AtlasPage {
            width,
            height,
            stride,
            bytes: Arc::from(vec![0u8; (stride * height) as usize]),
        }
    }

    /// set 0 に 2 エントリ（1 つは placement あり・1 つは空）を持つ小テーブル。
    fn sample_table() -> AtlasTable {
        let keys = vec![
            AtlasKey {
                set: SetId(0),
                rel_path: "surface0.png".into(),
            },
            AtlasKey {
                set: SetId(0),
                rel_path: "surface1.png".into(),
            },
        ];
        let entries = vec![
            AtlasEntry {
                original: Size { w: 100, h: 80 },
                placement: Some(Placement {
                    page: 0,
                    uv_rect: Rect {
                        x: 4,
                        y: 4,
                        w: 60,
                        h: 50,
                    },
                    trim_offset: Point { x: 10, y: 12 },
                }),
            },
            AtlasEntry {
                // 全透明＝空エントリ（転写スキップ・6.2/4.4）
                original: Size { w: 32, h: 32 },
                placement: None,
            },
        ];
        AtlasTable::new(keys, entries, vec![page(64, 64)])
    }

    /// 6.1: 既知 path→Some・未知 path→None・別 SetId→None（set がキーの一部）。
    #[test]
    fn resolve_known_unknown_and_set_scoped() {
        let t = sample_table();
        assert_eq!(t.resolve(SetId(0), "surface0.png"), Some(ElementId(0)));
        assert_eq!(t.resolve(SetId(0), "surface1.png"), Some(ElementId(1)));
        // 未知の相対パス
        assert_eq!(t.resolve(SetId(0), "missing.png"), None);
        // 同名だが別 SetId → None（set がキーに含まれる証明）
        assert_eq!(t.resolve(SetId(1), "surface0.png"), None);
    }

    /// 6.1/6.2: known-with-placement / known-empty / unknown の 3 分岐。
    #[test]
    fn entry_placement_three_way() {
        let t = sample_table();

        // known-with-placement
        let id0 = t.resolve(SetId(0), "surface0.png").unwrap();
        let e0 = t.entry(id0);
        assert_eq!(e0.original, Size { w: 100, h: 80 });
        let p = e0.placement.as_ref().expect("placement present");
        assert_eq!(p.page, 0);
        assert_eq!(
            p.uv_rect,
            Rect {
                x: 4,
                y: 4,
                w: 60,
                h: 50
            }
        );
        assert_eq!(p.trim_offset, Point { x: 10, y: 12 });

        // known-empty（全透明・転写スキップ）
        let id1 = t.resolve(SetId(0), "surface1.png").unwrap();
        assert!(t.entry(id1).placement.is_none());

        // unknown（resolve が None）
        assert_eq!(t.resolve(SetId(0), "nope.png"), None);
    }

    /// D3: key(id) 逆引きが元の AtlasKey を返す（デバッグツリーダンプ）。
    #[test]
    fn key_reverse_lookup_round_trip() {
        let t = sample_table();
        let id = t.resolve(SetId(0), "surface1.png").unwrap();
        assert_eq!(
            t.key(id),
            &AtlasKey {
                set: SetId(0),
                rel_path: "surface1.png".into()
            }
        );
        assert_eq!(t.len(), 2);
        assert!(!t.is_empty());
    }

    /// 6.4: AtlasTable / AtlasPage が Send + Sync（スレッド間安全手渡し）。
    #[test]
    fn table_is_send_sync_and_cheap_clone() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AtlasTable>();
        assert_send_sync::<AtlasPage>();

        // Arc 共有ゆえ clone が安価・両方使える。
        let t = sample_table();
        let t2 = t.clone();
        assert_eq!(t.len(), t2.len());
        assert_eq!(
            t.resolve(SetId(0), "surface0.png"),
            t2.resolve(SetId(0), "surface0.png")
        );
    }

    /// 6.3: page 取得・stride 明示・範囲外は None・pages() スライス長一致。
    #[test]
    fn page_accessors() {
        let t = sample_table();
        let p = t.page(0).expect("page 0 present");
        assert_eq!(p.width, 64);
        assert_eq!(p.height, 64);
        assert_eq!(p.stride, 64 * 4);
        assert_eq!(p.bytes.len(), (p.stride * p.height) as usize);

        assert!(t.page(1).is_none());
        assert_eq!(t.pages().len(), 1);
    }
}
