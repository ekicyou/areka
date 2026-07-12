//! 命令モデル（下流共有 I/O 契約）。
//!
//! さくらスクリプトの 1 命令を表すフラットな単一 enum `Instruction` と、
//! 付随する値型（`SurfaceArg` / `NewLineRatio` / `Choice` / `MoveArgs`）を定義する。
//! これがクロスエンジン I/O 契約の片側であり、本パーサが生成者、
//! 下流 `areka-P0-sakura-engine` が消費者となる（型の正本は本クレートが所有）。
//!
//! 設計規律（design.md「State Management」）:
//! - フラット表現（意味の入れ子／木は持たない）。
//! - 派生は `Clone` / `Debug` / `PartialEq` のみ（`f32`/`Duration` を含むため
//!   `Eq`/`Hash` は付さない・`serde` も付さない）。
//! - `#[non_exhaustive]` により variant 追加は後方互換（要件 11.1）。
//! - 不透明 NewType（`SurfaceArg` / `NewLineRatio`）はフィールドを非公開とし、
//!   read-only アクセサ（`as_str` / `ratio`）で中身を公開する（dola `ActorKey` 流儀）。

use std::time::Duration;

/// さくらスクリプトの 1 命令（フラット・拡張に開く）。
///
/// 下流 `areka-P0-sakura-engine` と共有する I/O 契約の片側。各 variant は
/// 値正規化済み（待ち時間 = `Duration`、改行 = 比率、Choice = disp/target 分離）であり、
/// 下流は再パース不要（要件 1.2）。
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum Instruction {
    /// タグ間プレーンテキスト（要件 9）。
    Text(String),
    /// 話者スコープ `\p[n]`（要件 2.1）。
    SpeakerScope { n: u32 },
    /// サーフェス `\s[...]`（中身は不透明文字列・無加工。要件 2.2/2.3）。
    Surface(SurfaceArg),
    /// バルーン面切替 `\b[...]` / 裸形 `\bN`（`\s` と完全対称の第一級命令）。
    ///
    /// 面引数は数値形（`\b[10]`）・名前形（`\b[バルーン１]`）・非表示センチネル
    /// （`\b[-1]`）を区別せず、`Surface` と同じ不透明 `SurfaceArg` で無加工保持する。
    /// 数値化・範囲展開・alias 解決・`-1` の解釈はいずれも本層で行わず、消費側
    /// （seriko）の下流責務とする（転記層の規律・balloon-face-cue R1.1/1.4/1.5）。
    BalloonSurface(SurfaceArg),
    /// 待ち時間 `\w[n]` / `\wN` / `\_w[ms]` を統一（要件 3）。
    Wait(Duration),
    /// 改行 `\n[percent]` / `\n`（比率。要件 4）。
    NewLine(NewLineRatio),
    /// 選択肢 `\q[disp,target,...]`（要件 5）。
    Choice(Choice),
    /// カーソル絶対位置 `\_l[x,y]`（要件 6.1）。
    Cursor { x: String, y: String },
    /// 終端 `\e`（要件 6.2）。
    End,
    /// クリア `\c`（要件 6.3）。
    Clear,
    /// 終了 `\-`（要件 6.4）。
    Quit,
    /// キャラ移動 `\![move,...]`（引数 decode 済み。要件 7.1）。
    Move(MoveArgs),
    /// システム変数 `%username`（展開なしトークン。要件 8）。
    SystemVar(String),
    /// 汎用 `\!` コマンド（move 以外・種別＋生引数。要件 7.2/7.3/10）。
    GenericCommand { name: String, raw_args: Vec<String> },
    /// 寛容パススルー: 構文区切りできたが意味未対応／不正の生保持（要件 10/13.8）。
    Raw(String),
}

/// `\s[...]` の不透明中身（NewType・surface 層が解釈）。要件 2.2/2.3。
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceArg(String);

impl SurfaceArg {
    /// 不透明中身を保持する `SurfaceArg` を構築する（無加工保持・要件 2.3）。
    pub fn new(inner: String) -> Self {
        SurfaceArg(inner)
    }

    /// `\s[...]` の不透明中身を読み取る（改変不可・要件 2.3）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `\n` の比率（150 → 1.5）。素の `\n` は既定比率（1.0）。要件 4。
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

/// `\q[disp,target,refs...]` の分離保持。要件 5.1/5.2。
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// 表示文字列（disp）。
    pub disp: String,
    /// 選択 ID（target）。
    pub target: String,
    /// 第 3 引数以降の追加 Reference（順序保持・要件 5.2）。
    pub references: Vec<String>,
}

/// `\![move,...]` の decode 済み引数。要件 7.1。
///
/// 「decode」= 構文区切り＋引数分割であり、dx/dy/base の意味割当は
/// window-placement の責務（design「Out of Boundary」）。生引数列を保持する。
#[derive(Clone, Debug, PartialEq)]
pub struct MoveArgs {
    /// move コマンドの生引数列（意味割当は下流 window-placement の責務）。
    pub args: Vec<String>,
}
