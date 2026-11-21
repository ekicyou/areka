//! 命令モデル（下流共有 I/O 契約）。
//!
//! さくらスクリプトの 1 命令を表すフラットな単一 enum `Instruction` と、
//! 付随する値型（`SurfaceArg` / `NewLineRatio` / `Choice` / `Anchor`）を定義する。
//! これがクロスエンジン I/O 契約の片側であり、本パーサが生成者、
//! 下流 `areka-P0-sakura-engine` が消費者となる（型の正本は本クレートが所有）。
//!
//! - フラット表現（意味の入れ子／木は持たない）。
//! - 派生は `Clone` / `Debug` / `PartialEq` のみ（`f32`/`Duration` を含むため
//!   `Eq`/`Hash` は付さない・`serde` も付さない）。
//! - 不透明 NewType（`SurfaceArg` / `NewLineRatio`）はフィールドを非公開とし、
//!   read-only アクセサ（`as_str` / `ratio`）で中身を公開する（dola `ActorKey` 流儀）。

use std::time::Duration;

/// さくらスクリプトの 1 命令（フラット・拡張に開く）。
///
/// 下流 `areka-P0-sakura-engine` と共有する I/O 契約の片側。各 variant は
/// 値正規化済み（待ち時間 = `Duration`、改行 = 比率、Choice = disp/target 分離）であり、
/// 下流は再パース不要。
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// タグ間プレーンテキスト。
    Text(String),
    /// 話者スコープ `\p[n]`。
    SpeakerScope { n: u32 },
    /// サーフェス `\s[...]`（中身は不透明文字列・無加工）。
    Surface(SurfaceArg),
    /// バルーン面切替 `\b[...]` / 裸形 `\bN`（`\s` と完全対称の第一級命令）。
    ///
    /// 面引数は数値形（`\b[10]`）・名前形（`\b[バルーン１]`）・非表示センチネル
    /// （`\b[-1]`）を区別せず、`Surface` と同じ不透明 `SurfaceArg` で無加工保持する。
    /// 数値化・範囲展開・alias 解決・`-1` の解釈はいずれも本層で行わず、消費側
    /// （seriko）の下流責務とする（転記層の規律）。
    BalloonSurface(SurfaceArg),
    /// 待ち時間 `\wN` / `\_w[ms]` を統一。
    Wait(Duration),
    /// 改行 `\n[percent]` / `\n`（比率）。
    NewLine(NewLineRatio),
    /// 選択肢 `\q[disp,target,...]`。
    Choice(Choice),
    /// カーソル絶対位置 `\_l[x,y]`。
    Cursor { x: String, y: String },
    /// 終端 `\e`。
    End,
    /// クリア `\c`。
    Clear,
    /// 終了 `\-`。
    Quit,
    /// システム変数 `%username`（展開なしトークン）。
    SystemVar(String),
    /// 汎用 `\!` コマンド（`move` も含む・種別＋生引数）。
    GenericCommand { name: String, raw_args: Vec<String> },
    /// 文字装飾 `\f[key,args...]`（転記のみ）。
    ///
    /// `args` は角括弧の中を `,` で割った列そのもの（`args[0]` がキー・以降が値の列）。
    /// 意味は一切読まず、記述順のまま保持し、空のトークンも潰さない。引数の形は 4 通り:
    /// `\f[]` と裸の `\f` はどちらも `[]`、`\f[""]` だけが `[""]`、`\f[bold,]` は
    /// `["bold",""]`。キーの意味付け（43 形のうち意味を与える 12 項目とそれ以外の
    /// 振り分け）は消費側（文字レンダリング層）の責務であり、本層は転記に徹する。
    Font { args: Vec<String> },
    /// アンカーの開き `\_a[ID,…]`（転記のみ）。
    Anchor(Anchor),
    /// アンカーの閉じ（角括弧の無い `\_a`）。
    ///
    /// 開きとの対応（閉じ無し・重なり・開いていない閉じ）は本層では見ない。
    AnchorEnd,
    /// 寛容パススルー: 構文区切りできたが意味未対応／不正の生保持。
    ///
    /// 区切れたタグの字は元の綴りでなく、おおよその組み直し（`\i["a,b"]` → `\i[a,b]`）。
    Raw(String),
    /// 出せない・読めない文字の番号を捨てた（`\_u[…]`・`\_m[…]`）。
    ///
    /// `raw` はタグのおおよその組み直し（元の綴りそのものではない・`\_u[]` → `\_u`）。
    /// 警告を出すかどうかは消費側の責務（本層は記録を出さない）。
    CharCodeDropped { raw: String, reason: CharCodeDrop },
}

/// 文字の番号を捨てた理由。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharCodeDrop {
    /// 番号 0。
    Zero,
    /// 制御の文字（U+0001〜U+001F・U+007F〜U+009F）。
    Control,
    /// 組にならない片割れのサロゲート。
    LoneSurrogate,
    /// U+10FFFF より上（32 ビットに入らない桁も含む）。
    OutOfRange,
    /// 番号として読めない綴り（空・`0x` だけ・符号・ほかの字・引数なし）。
    Unreadable,
}

/// 選択肢の印 `\![*]`・`%*` を載せる汎用コマンドの名前。
pub const MARKER_COMMAND: &str = "*";

/// `\j[ID]`（ジャンプ）を載せる汎用コマンドの運搬名。
///
/// `\j[…]` は [`Instruction::GenericCommand`] の `name` にこの値を入れ、引数列を記述順の
/// まま `raw_args` へ運ぶ。`\f` の運搬名（`"\\f"`）と同じ流儀で、バックスラッシュで始まる
/// ため `\!` のコマンド名とは衝突しない。ID の意味付けは消費側の責務。
pub const JUMP_TAG_CARRIER: &str = "\\j";

/// 裸の `\v`（手前へ出す）を載せる汎用コマンドの運搬名。
///
/// [`JUMP_TAG_CARRIER`] と同じ流儀で、台本から `\![…]` では書けない綴りにする。
/// `\v` は引数を持たないので `raw_args` は空。窓をどう動かすかは消費側の責務。
pub const RAISE_TAG_CARRIER: &str = "\\v";

/// `\s[...]` の不透明中身（NewType・surface 層が解釈）。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceArg(String);

impl SurfaceArg {
    /// 不透明中身を保持する `SurfaceArg` を構築する（無加工保持）。
    pub fn new(inner: String) -> Self {
        SurfaceArg(inner)
    }

    /// `\s[...]` の不透明中身を読み取る（改変不可）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `\n` の比率（150 → 1.5）。素の `\n` は既定比率（1.0）。
#[derive(Clone, Debug, PartialEq)]
pub struct NewLineRatio(f32);

impl NewLineRatio {
    /// 比率値を保持する `NewLineRatio` を構築する（負値も符号付きで保持）。
    pub fn new(ratio: f32) -> Self {
        NewLineRatio(ratio)
    }

    /// `\n` の比率（150 → 1.5）を読み取る。
    pub fn ratio(&self) -> f32 {
        self.0
    }
}

/// `\q[disp,target,refs...]` の分離保持。
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// 表示文字列（disp）。
    pub disp: String,
    /// 選択 ID（target）。
    pub target: String,
    /// 第 3 引数以降の追加 Reference（順序保持）。
    pub references: Vec<String>,
}

/// `\_a[ID,r…]`（アンカーの開き）の分離保持。
///
/// 意味は読まない: ID が `On` で始まるかどうかも、ID が空かどうかも本層では区別しない。
#[derive(Clone, Debug, PartialEq)]
pub struct Anchor {
    /// 第 1 引数（ID）。`\_a[]` のように無ければ空の文字列。
    pub id: String,
    /// 第 2 引数以降（記述順・空のトークンも潰さない）。
    pub references: Vec<String>,
}

/// 台本を読む段が、命令 1 つについて自分で下した扱いの印。
///
/// 印は「読む段が自分でしたこと」だけを言う（意味づけ・シェルの事実・`\!` の行き先は知らない）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadNote {
    /// 腕の無い綴りなので `Raw` にした（知らないタグ）。
    UnknownTag,
    /// 閉じていない `[`／`"` なので、そこから入力の末尾までを `Raw` にした。
    Unclosed,
    /// 引数が無い・読めないので、既定の値へ落とした。
    ArgumentDefaulted,
}

/// 命令 1 つと、その台本の中の位置・印（`parse_noted` の 1 要素）。
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub instruction: Instruction,
    /// 台本の中のバイト範囲（先頭を含み末尾を含まない）。`&input[span]` が該当する綴り。
    pub span: std::ops::Range<usize>,
    /// 無ければ空。
    pub notes: Vec<ReadNote>,
}
