//! model — 下流共有 I/O 契約型。
//!
//! surfaces.txt を表すシェルサーフェスモデルの正本型を定義する。値は正規化済み
//! （ID/座標は数値・alias 値は数値 ID リスト）であり、下流は再パース不要
//! （要件 1.1/1.2）。意味解釈を下流へ委ねる値（画像パス・alias キー・collision 名）は
//! 不透明 NewType＋read-only アクセサで保持する（要件 1.3）。
//!
//! 設計規律（design.md「State Management」・確定契約スケッチに従う）:
//! - 全公開型に `#[derive(Clone, Debug, PartialEq)]`（`serde` / `Eq` / `Hash` は
//!   sakura 規律に倣い付さず最小派生に留める・要件 1.5）。
//! - 公開 enum（`Interval` / `AppendTarget`）は `#[non_exhaustive]`（要件 1.4）。
//! - opaque NewType（`ElementPath` / `AliasKey` / `CollisionName`）はフィールド非公開・
//!   `new()` コンストラクタ＋read-only accessor（`as_str`）のみ公開（dola `ActorKey` 流儀）。
//! - descript ヘッダ・charset はモデルに保持しない（要件 3.4・2 例目の実需まで追加しない要件 10.5）。
//!
//! 依存方向 `model ← lexer ← decode ← parse` の最上流ゆえ他層に依存しない（std のみ）。

/// surfaces.txt 全体のルート集約（下流共有 I/O 契約）。
/// descript ヘッダ・charset は寛容スキップし保持しない（要件 3）。
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    /// surfaceNNN 定義（出現順保持）。
    pub surfaces: Vec<Surface>,
    /// surface.append 追記定義（ターゲット指定を記述子で保持〔展開しない〕・出現順保持）。
    pub appends: Vec<SurfaceAppend>,
    /// kero.surface.alias 写像（重複キー保持・出現順保持・要件 8.4）。
    pub aliases: Vec<SurfaceAlias>,
}

/// 1 個の surfaceNNN 定義（要件 4.1）。
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    /// surface ID（数値 NNN）。
    pub id: u32,
    /// element overlay 群（レイヤインデックス昇順・要件 4.4）。
    pub elements: Vec<Element>,
    /// collision 矩形群（出現順）。
    pub collisions: Vec<Collision>,
    /// SERIKO animation 群（出現順・ID 順序付け実行は下流・要件 5.6）。
    pub animations: Vec<Animation>,
}

/// element overlay 行 elementN,overlay,PATH,X,Y（要件 4.2）。
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    /// element の N（レイヤインデックス）。
    pub layer: u32,
    /// 無加工画像パス（区切り含む・要件 4.3）。
    pub path: ElementPath,
    /// X 座標。
    pub x: i64,
    /// Y 座標。
    pub y: i64,
}

/// element 画像パスの opaque 中身（読込・検証しない・要件 4.3）。
#[derive(Clone, Debug, PartialEq)]
pub struct ElementPath(String);

impl ElementPath {
    /// 無加工の画像パス文字列を保持する `ElementPath` を構築する（要件 4.3）。
    pub fn new(inner: String) -> Self {
        ElementPath(inner)
    }

    /// 画像パスの opaque 中身を読み取る（改変不可・要件 4.3）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// SERIKO animation（interval＋pattern 群を animation ID で束ねる・要件 5）。
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// animation ID（要件 5.6）。
    pub id: u32,
    /// interval 指定（要件 5.1-5.3）。
    pub interval: Interval,
    /// pattern index を明示保持する pattern 群（疎許容・要件 5.4）。
    pub patterns: Vec<Pattern>,
}

/// interval 3 種（emo2 subset・拡張は non_exhaustive シーム・要件 5.1-5.3/5.7）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Interval {
    /// `interval,bind`（要件 5.1）。
    Bind,
    /// `interval,random,K`（要件 5.2）。
    Random {
        /// 頻度パラメータ K。
        k: u32,
    },
    /// `interval,bind+random,K`（要件 5.3）。
    BindRandom {
        /// 頻度パラメータ K。
        k: u32,
    },
}

/// animationN.patternM,overlay,SURFACE_ID,WAIT,X,Y（要件 5.4/5.5）。
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// patternM の M（疎・連番前提を置かない・要件 5.4）。
    pub index: u32,
    /// 参照 surface ID。負値はレイヤクリア/停止センチネル（要件 5.5・下流解釈）。
    pub surface_id: i64,
    /// WAIT（ミリ秒・値保持のみ）。
    pub wait: u32,
    /// X 座標。
    pub x: i64,
    /// Y 座標。
    pub y: i64,
}

/// collisionN,LEFT,TOP,RIGHT,BOTTOM,NAME（矩形・要件 6.1/6.2）。
/// ukadoc の順序 始点X/始点Y/終点X/終点Y = left/top/right/bottom。
#[derive(Clone, Debug, PartialEq)]
pub struct Collision {
    /// collisionN の N。
    pub index: u32,
    /// 矩形 左（始点 X）。
    pub left: i64,
    /// 矩形 上（始点 Y）。
    pub top: i64,
    /// 矩形 右（終点 X）。
    pub right: i64,
    /// 矩形 下（終点 Y）。
    pub bottom: i64,
    /// 領域名 opaque（Head/Bust 等・要件 6.2）。
    pub name: CollisionName,
}

/// collision 領域名の opaque 中身（意味解釈しない・要件 6.2）。
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionName(String);

impl CollisionName {
    /// 領域名文字列を保持する `CollisionName` を構築する（要件 6.2）。
    pub fn new(inner: String) -> Self {
        CollisionName(inner)
    }

    /// 領域名の opaque 中身を読み取る（改変不可・要件 6.2）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// surface.append 追記定義（ターゲット指定は記述子で保持・展開しない・要件 7）。
/// collision/animation は通常 surface と同一型（要件 7.3）。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAppend {
    /// ターゲット指定（ヘッダ数値を第1要素とする単一/範囲の順序付きリスト）。
    /// 範囲の個別 ID 展開・実 surface ツリーへの転記は下流の責務（要件 7.2）。
    pub targets: Vec<AppendTarget>,
    /// 追記 collision 群（通常 surface と同一表現・要件 7.3）。
    pub collisions: Vec<Collision>,
    /// 追記 animation 群（通常 surface と同一表現・要件 7.3）。
    pub animations: Vec<Animation>,
}

/// surface.append のターゲット指定要素（parse 時展開しない・要件 7.2）。
/// `surface.append10,2100-2110` → `[Single(10), Range{start:2100,end:2110}]`。
/// ヘッダ数値も列挙要素と同格の第1要素（カテゴリ番号等の特別扱いはしない）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum AppendTarget {
    /// 単一 surface ID。
    Single(u32),
    /// 範囲指定 `a-b`（両端含む・展開は下流のツリー構築側が担う）。
    Range {
        /// 範囲始点（含む）。
        start: u32,
        /// 範囲終点（含む）。
        end: u32,
    },
}

/// kero.surface.alias の 1 エントリ KEY,[id,...]（要件 8）。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAlias {
    /// alias キー opaque（数値・日本語いずれも・要件 8.2）。
    pub key: AliasKey,
    /// 順序付き数値 ID（要件 8.3）。
    pub ids: Vec<u32>,
}

/// alias キーの opaque 中身（意味解釈しない・要件 8.2）。
#[derive(Clone, Debug, PartialEq)]
pub struct AliasKey(String);

impl AliasKey {
    /// alias キー文字列を保持する `AliasKey` を構築する（要件 8.2）。
    pub fn new(inner: String) -> Self {
        AliasKey(inner)
    }

    /// alias キーの opaque 中身を読み取る（改変不可・要件 8.2）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
