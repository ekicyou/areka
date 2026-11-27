//! model — 下流共有 I/O 契約型。
//!
//! surfaces.txt を表すシェルサーフェスモデルの正本型を定義する。値は正規化済み
//! （ID/座標は数値・alias 値は数値 ID リスト）であり、下流は再パース不要。
//! 意味解釈を下流へ委ねる値（画像パス・alias キー・collision 名）は
//! 不透明 NewType＋read-only アクセサで保持する。
//!
//! - 公開型の派生は `Clone, Debug, PartialEq` に留める（`serde` / `Eq` / `Hash` は付けない）。
//! - 公開 enum（`Interval` / `AppendTarget`）は `#[non_exhaustive]`。
//! - opaque NewType（`ElementPath` / `AliasKey` / `CollisionName`）はフィールド非公開・
//!   `new()` と `as_str` のみ公開。
//! - descript ヘッダ・charset はモデルに保持しない。
//!
//! 依存方向 `model ← lexer ← decode ← parse` の最上流ゆえ他層に依存しない（std と、崩れの記録の重さ `tracing::Level` だけ）。

/// surfaces.txt 全体のルート集約（下流共有 I/O 契約）。
/// descript ヘッダ・charset は保持しない。
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    /// surfaceNNN 定義（出現順保持）。
    pub surfaces: Vec<Surface>,
    /// surface.append 追記定義（ターゲット指定を記述子で保持〔展開しない〕・出現順保持）。
    pub appends: Vec<SurfaceAppend>,
    /// kero.surface.alias 写像（重複キー保持・出現順保持）。
    pub aliases: Vec<SurfaceAlias>,
    /// `animation-sort` の値（記述のまま。未指定は None・既定解釈は下流）。
    pub animation_sort: Option<SortOrder>,
    /// `collision-sort` の値（同上・未指定は None・none 既定の解釈も下流）。
    pub collision_sort: Option<SortOrder>,
    /// 登場順の単一定義ストリーム（3 Vec への index 参照・種別間の登場順を保持・データ重複なし）。
    pub definitions: Vec<DefRef>,
}

/// ソート順の値（`animation-sort`／`collision-sort` の記述のまま。既定の適用は下流）。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortOrder {
    /// 昇順（`ascend`）。
    Ascend,
    /// 降順（`descend`）。
    Descend,
    /// 上の 2 つでない空でない値の原文（転記層は黙って落とさない・扱いと知らせは下流）。
    Other(Box<str>),
}

/// 登場順参照ストリームの要素。各 variant の `usize` は対応する Vec 内の位置。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefRef {
    /// `Shell.surfaces` の index を指す。
    Surface(usize),
    /// `Shell.appends` の index を指す。
    Append(usize),
    /// `Shell.aliases` の index を指す。
    Alias(usize),
}

/// 1 個の surfaceNNN 定義。
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    /// ヘッダ記述子の転記（単一形は `[Single(id)]`・多 id 形は列挙/範囲を保持）。
    /// 範囲展開・実 surface ツリーへの転記は下流の責務。
    pub targets: Vec<AppendTarget>,
    /// element定義の `overlay` と `base` の行（どちらも同じ値・レイヤインデックス昇順）。
    pub elements: Vec<Element>,
    /// collision 矩形群（出現順）。
    pub collisions: Vec<Collision>,
    /// SERIKO animation 群（出現順・ID 順序付け実行は下流）。
    pub animations: Vec<Animation>,
}

/// element定義の画像の行 elementN,語,PATH,X,Y[,オプション…]。
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    /// element の N（レイヤインデックス）。
    pub layer: u32,
    /// 無加工画像パス（区切り含む）。
    pub path: ElementPath,
    /// X 座標。
    pub x: i64,
    /// Y 座標。
    pub y: i64,
    /// 書かれた描画メソッドの語の原文。
    pub method: DrawMethod,
    /// 座標の後の並びの原文（無ければ空）。
    pub options: ElementOptions,
}

/// element定義の座標の後の並びの原文（意味は下流が読む）。
#[derive(Clone, Debug, PartialEq)]
pub struct ElementOptions(String);

impl ElementOptions {
    /// 無加工のオプションの並びを保持する `ElementOptions` を構築する。
    pub fn new(inner: String) -> Self {
        ElementOptions(inner)
    }

    /// オプションの並びの原文を読み取る。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// element 画像パスの opaque 中身（読込・検証しない）。
#[derive(Clone, Debug, PartialEq)]
pub struct ElementPath(String);

impl ElementPath {
    /// 無加工の画像パス文字列を保持する `ElementPath` を構築する。
    pub fn new(inner: String) -> Self {
        ElementPath(inner)
    }

    /// 画像パスの opaque 中身を読み取る（改変不可）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// SERIKO animation（interval＋pattern 群を animation ID で束ねる）。
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// animation ID。
    pub id: u32,
    /// interval 指定。interval 行が無いときは既定 `Interval::Bind`。
    pub interval: Interval,
    /// pattern index を明示保持する pattern 群（疎許容）。
    pub patterns: Vec<Pattern>,
}

/// interval 語彙（emo2 subset は 3 種を駆動・拡張は non_exhaustive シーム）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Interval {
    /// `interval,bind`。
    Bind,
    /// `interval,random,K`。
    Random {
        /// 頻度パラメータ K。
        k: u32,
    },
    /// `interval,bind+random,K`。
    BindRandom {
        /// 頻度パラメータ K。
        k: u32,
    },
    /// `interval,runonce`（第 3 欄以降は読まない）。
    Runonce,
    /// `interval,periodic,秒`（`secs >= 1` を構築時に保証する）。
    Periodic {
        /// 周期の秒数（1 以上）。
        secs: u32,
    },
    /// `interval,talk,文字数`（`n >= 1` を構築時に保証する）。
    Talk {
        /// 何文字ごとに鳴らすか（1 以上）。
        n: u32,
    },
    /// 未認識 interval 語彙の原文（sometimes/rarely/always 等）。駆動しないが値としては保持する。
    /// `talk`／`periodic` で数値が 1 以上の整数として読めないとき（欠落・0・非数値）は
    /// 第 2 欄以降を `,` で繋いだ原文（例 `talk,abc`・`periodic`）を運ぶ。
    Other(Box<str>),
}

/// animationN.patternM,描画メソッド,SURFACE_ID,WAIT,X,Y。
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// patternM の M（疎・連番前提を置かない）。
    pub index: u32,
    /// 描画メソッド（method）の原文（意味解釈は下流・`DrawMethod` 参照）。
    pub method: DrawMethod,
    /// 参照 surface ID。負値はレイヤクリア/停止センチネル（下流解釈）。
    pub surface_id: i64,
    /// 欄 2 の原文（`(` で始まる並びは `)` まで継ぎ直した原文）。8 語の相手の欄として下流が読む。
    pub target: String,
    /// WAIT（ミリ秒・値保持のみ）。
    pub wait: u32,
    /// X 座標。
    pub x: i64,
    /// Y 座標。
    pub y: i64,
}

/// コマ描画メソッドの opaque 中身（意味解釈しない）。
///
/// - **新形式** `animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X,Y`
///   ＝method は第 1 位置（現行 lexer の対象）。
/// - **旧形式**（SERIKO 1.x）＝method は第 3 位置。旧形式キー行の字句解析は emo2 subset 外。
///
/// 語彙（overlay/base/move/scaling/start/stop/alternativestart/alternativestop/
/// parallelstart/parallelstop/insert/auto 等）は全て原文のまま保持する。
/// 同義写像・実装可否の判定は下流の責務。
#[derive(Clone, Debug, PartialEq)]
pub struct DrawMethod(String);

impl DrawMethod {
    /// 無加工の描画メソッド文字列を保持する `DrawMethod` を構築する。
    pub fn new(inner: String) -> Self {
        DrawMethod(inner)
    }

    /// 描画メソッドの opaque 中身を読み取る（改変不可）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 当たり判定 1 件（番号・形・名前）。`collision*` と `collisionex*` を同じ型で持つ。
#[derive(Clone, Debug, PartialEq)]
pub struct Collision {
    /// collisionN・collisionexN の N。
    pub index: u32,
    /// 形（collision* は常に矩形）。
    pub shape: CollisionShape,
    /// 領域名 opaque（Head/Bust 等）。
    pub name: CollisionName,
}

/// 当たり判定の形。ukadoc が閉じて定める 5 つの集合なので `#[non_exhaustive]` を付けない
/// （使い手の match をワイルドカードなしの網羅に保つ）。
#[derive(Clone, Debug, PartialEq)]
pub enum CollisionShape {
    /// 矩形。
    Rect(CollisionBounds),
    /// 楕円（囲みの長方形）。
    Ellipse(CollisionBounds),
    /// 円（中心と半径・負も運ぶ）。
    Circle { x: i64, y: i64, radius: i64 },
    /// 多角形（書いた順の頂点）。
    Polygon(Vec<(i64, i64)>),
    /// 画像で塗った領域。
    Region(CollisionRegion),
}

/// 囲みの長方形（ukadoc の順序 始点X/始点Y/終点X/終点Y = left/top/right/bottom・並べ直さない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionBounds {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

/// `region` の欄と画像を置く位置。
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionRegion {
    /// 書いたまま（区切りも大小文字も変えない・開かない）。
    pub file: String,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    /// 領域反転（欄が無ければ false）。
    pub invert: bool,
    /// 画像を置く位置（読みでは 0,0・持ち込みでずれる）。
    pub x: i64,
    pub y: i64,
}

/// collision 領域名の opaque 中身（意味解釈しない）。
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionName(String);

impl CollisionName {
    /// 領域名文字列を保持する `CollisionName` を構築する。
    pub fn new(inner: String) -> Self {
        CollisionName(inner)
    }

    /// 領域名の opaque 中身を読み取る（改変不可）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// surface.append 追記定義（ターゲット指定は記述子で保持・展開しない）。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAppend {
    /// ターゲット指定（ヘッダ数値を第1要素とする単一/範囲の順序付きリスト）。
    /// 範囲の個別 ID 展開・実 surface ツリーへの転記は下流の責務。
    pub targets: Vec<AppendTarget>,
    /// 追記 element 群（レイヤ昇順）。
    pub elements: Vec<Element>,
    /// 追記 collision 群（通常 surface と同一表現）。
    pub collisions: Vec<Collision>,
    /// 追記 animation 群（通常 surface と同一表現）。
    pub animations: Vec<Animation>,
}

/// surface.append のターゲット指定要素（parse 時展開しない）。
/// `surface.append10,2100-2110` → `[Single(10), Range{start:2100,end:2110}]`。
/// ヘッダ数値も列挙要素と同格の第1要素。
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
    /// 除外指定 `!N`（記述子のまま保持・減算適用は下流の展開時）。
    Exclude(u32),
    /// 除外範囲指定 `!a-b`（両端含む・記述子のまま保持・減算適用は下流）。
    ExcludeRange {
        /// 除外範囲始点（含む）。
        start: u32,
        /// 除外範囲終点（含む）。
        end: u32,
    },
}

/// kero.surface.alias の 1 エントリ KEY,[id,...]。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceAlias {
    /// alias キー opaque（数値・日本語いずれも）。
    pub key: AliasKey,
    /// 順序付き数値 ID。
    pub ids: Vec<u32>,
}

/// alias キーの opaque 中身（意味解釈しない）。
#[derive(Clone, Debug, PartialEq)]
pub struct AliasKey(String);

impl AliasKey {
    /// alias キー文字列を保持する `AliasKey` を構築する。
    pub fn new(inner: String) -> Self {
        AliasKey(inner)
    }

    /// alias キーの opaque 中身を読み取る（改変不可）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 範囲の幅の合計の上限（シェル全体）。ukadoc が MATERIA の制限として載せる番号の数（0〜8192）を借りた。
/// 番号の値にはかけない。emo-compose の fold の範囲の展開はこの上限だけに頼るので、緩めるときは先に伝える。
pub const RANGE_WIDTH_CAP: u64 = 8193;

/// 読みの崩れ 1 件。見出しは原文、行の番号は見出し行の 1 始まりの番号。
#[derive(Clone, Debug, PartialEq)] // #[non_exhaustive] を付けない
pub enum ShellIssue {
    /// 終わりで閉じていないブレス（中身は落とした）。
    UnclosedAtEnd { heading: String, line: usize },
    /// `}` が抜け、次の `{` で区切り直したブレス（中身は残した）。
    ClosedByNextBrace { heading: String, line: usize },
    /// 読めない数の記述子・項目を飛ばした（0 にしない）。`text` は飛ばした行の欄を `,` で繋いだもの。
    UnreadableNumber { heading: String, text: String },
    /// シェル全体の範囲の幅の合計が上限を越える範囲の記述子を飛ばした。
    RangeTooWide { heading: String, text: String },
    /// ウェイトの `最小-最大` の最小を待ちにした（ランダムに選ぶのは未対応）。
    WaitRange { heading: String, text: String },
    /// ブレスの外の `}` だけの行・見出しの無い `{` だけの行を読み飛ばした。1 始まりの行の番号。
    StrayBrace { line: usize },
    /// collisionex の行を使わなかった、または前から求める数だけを使った（理由つき）。`text` は行の欄を `,` で繋いだもの。
    CollisionEx {
        heading: String,
        text: String,
        fault: CollisionExFault,
    },
}

/// `collisionex` の行を使わなかった（または一部だけ使った）理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq)] // #[non_exhaustive] を付けない（ShellIssue と同じ）
pub enum CollisionExFault {
    UnknownShape,
    MissingCoordinates,
    OddCoordinates,
    NotInteger,
    BadRegion,
    /// この理由だけは行を使う（前から求める数だけ）。
    ExtraFields,
}

impl ShellIssue {
    /// 記録の重さ（`RangeTooWide` は error、ほかは warn）。
    pub fn severity(&self) -> tracing::Level {
        match self {
            ShellIssue::RangeTooWide { .. } => tracing::Level::ERROR,
            ShellIssue::UnclosedAtEnd { .. }
            | ShellIssue::ClosedByNextBrace { .. }
            | ShellIssue::UnreadableNumber { .. }
            | ShellIssue::WaitRange { .. }
            | ShellIssue::StrayBrace { .. }
            | ShellIssue::CollisionEx { .. } => tracing::Level::WARN,
        }
    }

    /// 記録の文言（見出しと、行の番号または欄の原文を名指す）。
    pub fn message(&self) -> String {
        match self {
            ShellIssue::UnclosedAtEnd { heading, line } => format!(
                "shell: surfaces.txt の {line} 行目のブレス `{heading}` が終わりまで閉じていないので中身を読まない"
            ),
            ShellIssue::ClosedByNextBrace { heading, line } => format!(
                "shell: surfaces.txt の {line} 行目のブレス `{heading}` の `}}` が抜けているので、次の `{{` の手前で閉じた"
            ),
            ShellIssue::UnreadableNumber { heading, text } => format!(
                "shell: surfaces.txt のブレス `{heading}` の `{text}` は番号が読めないので飛ばした"
            ),
            ShellIssue::RangeTooWide { heading, text } => format!(
                "shell: surfaces.txt のブレス `{heading}` の範囲 `{text}` を足すとシェル全体の範囲の幅の合計が上限 {RANGE_WIDTH_CAP} を越えるので飛ばした"
            ),
            ShellIssue::WaitRange { heading, text } => format!(
                "shell: surfaces.txt のブレス `{heading}` のウェイト `{text}` から選ぶのは未対応なので最小を待ちにした"
            ),
            ShellIssue::StrayBrace { line } => format!(
                "shell: surfaces.txt の {line} 行目の括弧はブレスの見出しに続かないので読み飛ばした"
            ),
            ShellIssue::CollisionEx {
                heading,
                text,
                fault,
            } => {
                let (reason, effect) = match fault {
                    CollisionExFault::UnknownShape => ("形の名前を知らない", "当たり判定にしない"),
                    CollisionExFault::MissingCoordinates => {
                        ("座標が足りない", "当たり判定にしない")
                    }
                    CollisionExFault::OddCoordinates => {
                        ("多角形の座標の数が奇数", "当たり判定にしない")
                    }
                    CollisionExFault::NotInteger => {
                        ("整数として読めない欄がある", "当たり判定にしない")
                    }
                    CollisionExFault::BadRegion => ("region の欄が不正", "当たり判定にしない"),
                    CollisionExFault::ExtraFields => ("欄が多い", "前から求める数だけを使った"),
                };
                format!(
                    "shell: surfaces.txt のブレス `{heading}` の collisionex の行 `{text}` は{reason}ので{effect}"
                )
            }
        }
    }
}
