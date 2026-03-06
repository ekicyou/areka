# 実装依頼書: pasta DSL キューコマンド文法拡張

> **作成日**: 2026-03-03  
> **バージョン**: v1  
> **対象**: pasta_dsl クレート実装者（AI エージェント想定）  
> **参照元仕様**: `areka-wintf/.kiro/specs/dola-cue-pasta-dsl-extension/`

---

## 1. 概要

### 1.1 何を実装するのか

pasta DSL の PEG 文法（`grammar.pest`）を拡張し、既存の行指向文法へ **キューコマンド行** と **アクション行内 alias 参照** を埋め込めるようにする。これにより dola クレートの `CueSheet` データモデルをテキストで記述できる。

### 1.2 なぜ必要なのか

dola クレートの `CueSheet` は演出台本（テキスト表示・表情変更・選択肢・バリア・ルーティング）を時系列で管理するデータモデルである。現在は Rust コードでしか構築できないが、ゴーストスクリプト作者がテキストで記述できる必要がある。

### 1.3 アーキテクチャ（境界と責務）

**採用パターン: ハイブリッド（Option C）**

```
.pasta ファイル
    ↓ (PEG パース)
pasta_dsl: CueIrScene（中間表現 — 時刻なし・順序付き）
    ↓ (build)
dola: CueSheetBuilder + DurationResolver + SlotRegistry
    ↓
dola: CueSheet（最終出力 — start_time 確定済み）
```

**pasta_dsl の責務**:
- `.pasta` テキストを PEG で解析し `CueIrScene` を出力する
- 時刻計算は **一切行わない**（行の出現順序と構造のみ）
- `CueIrScene` を dola 側に渡すだけ

**dola の責務**（参考情報 — pasta_dsl 実装者が変更する必要はない）:
- `CueSheetBuilder` が `CueIrScene` を受け取り `CueSheet` に変換
- `DurationResolver` トレイトで各アクション行の所要時間を外部注入
- `SlotRegistry` トレイトでアクター→スロット割り当てを管理

### 1.4 破壊的変更はゼロ

既存の pasta DSL の行種別は維持したまま、拡張対象は次の 3 点に限定する。

- `!` / `！` で始まるキューコマンド行
- アクション行中の `@alias` による CueCommand 参照
- `%actor、actor=N` 形式のスロット指定情報の利用

---

## 2. dola 側ドメイン型（参照用・変更不要）

pasta_dsl が出力する CueIR は、最終的にこれらの dola 型にマッピングされる。pasta_dsl 側が直接これらの型を生成する必要はないが、CueIR の設計を理解するために全型を掲載する。

### 2.1 ActorKey

```rust
/// 演者識別子。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActorKey(String);
```

### 2.2 CueTarget

```rust
/// CueCommand の配送先スロット種別。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CueTarget {
    /// シェル（キャラクター描画）
    Shell,
    /// バルーン（テキスト表示）
    Balloon,
}
```

### 2.3 EntityKey

```rust
/// 配送先キー識別子。
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKey {
    /// アクターの特定スロット
    Actor(ActorKey, CueTarget),
    /// 物理スポットエンティティ
    Spot(String),
    /// 物理バルーンエンティティ
    Balloon(String),
}
```

### 2.4 BarrierKind

```rust
/// バリア種別（3 種）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum BarrierKind {
    /// クリック/キー入力待ち
    WaitForInput { timeout: Option<f64> },
    /// 選択肢待ち
    WaitForChoice { timeout: Option<f64> },
    /// 指定時間経過待ち
    Timeout { duration: f64 },
}
```

### 2.5 RoutingCommand

```rust
/// ルーティングコマンド（3 バリアント）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RoutingCommand {
    /// スロット追加
    RouteAdd { target: CueTarget, to: EntityKey },
    /// スロット切替
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// スロット除去
    RouteRemove { target: CueTarget },
}
```

### 2.6 CueCommand

```rust
/// 演出コマンド（6 バリアント）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CueCommand {
    /// テキスト表示
    Text(String),
    /// コンテンツクリア
    Clear,
    /// 演技発現（表情変更等）
    Emote { key: String },
    /// 選択肢データ
    Choice { id: String, text: String },
    /// ECS エンティティ参照渡し（u64 = Entity::to_bits()）
    EntityRef(u64),
    /// 消費者固有コマンド（DynamicValue は JSON 互換辞書型）
    Custom { command: String, params: DynamicValue },
}
```

### 2.7 CuePayload / Cue（最終出力構造）

```rust
/// 統合ペイロード型。
pub enum CuePayload {
    Command(CueCommand),
    Barrier(BarrierKind),
    Routing(RoutingCommand),
}

/// 個々の演出指示（相対時刻付き）。
pub struct Cue {
    pub actor: ActorKey,
    pub start_time: f64,       // ← dola CueSheetBuilder が算出。pasta_dsl は関与しない
    pub payload: CuePayload,
}
```

### 2.8 データモデル構造図

```
CueSheet
└── Vec<Cue>
    └── Cue
        ├── actor: ActorKey
        ├── start_time: f64          ← CueSheetBuilder が算出
        └── payload: CuePayload
            ├── Command(CueCommand)
            │   ├── Text(String)
            │   ├── Clear
            │   ├── Emote { key }
            │   ├── Choice { id, text }
            │   ├── EntityRef(u64)
            │   └── Custom { command, params }
            ├── Barrier(BarrierKind)
            │   ├── WaitForInput { timeout }
            │   ├── WaitForChoice { timeout }
            │   └── Timeout { duration }
            └── Routing(RoutingCommand)
                ├── RouteAdd { target, to }
                ├── RouteSwitch { target, to }
                └── RouteRemove { target }
```

---

## 3. pasta_dsl が出力すべき CueIR 型定義

以下の型を `pasta_dsl::cue_ir` モジュール（推奨）に配置する。これらは PEG パースの結果として生成し、dola の `CueSheetBuilder` に渡す中間表現である。

### 3.1 CueIrScene

```rust
/// キューシートモードのシーン中間表現。
/// start_time は含まない（dola CueSheetBuilder が算出する）。
pub struct CueIrScene {
    /// シーン名
    pub name: String,
    /// エントリの有順序リスト（出現順 = タイムライン順序）
    pub entries: Vec<CueIrEntry>,
    /// シーンスコープのエイリアス定義（エントリより前に処理される）
    pub alias_defs: Vec<CueIrAliasDef>,
}
```

### 3.2 CueIrEntry

```rust
/// CueIR エントリ（1 行 または 継続行を含む 1 論理ブロック）。
pub enum CueIrEntry {
    /// アクション行（actor:content + @command フラグメント）
    Action(CueIrAction),
    /// `!` コマンド行
    Command(CueIrCommand),
}
```

### 3.3 CueIrAction

```rust
/// アクション行の中間表現。
pub struct CueIrAction {
    /// アクター識別子
    pub actor: ActorKey,
    /// 行内フラグメントのリスト（テキスト断片 + エイリアス参照が交互に並ぶ）
    pub fragments: Vec<CueIrFragment>,
    /// ソース行番号（エラーレポート用）
    pub source_line: u32,
}
```

### 3.4 CueIrFragment

```rust
/// アクション行内の最小単位。
pub enum CueIrFragment {
    /// テキスト断片（継続行は `\n` 結合済み）
    Text(String),
    /// `@name` 参照（エイリアス解決前の名前）
    AliasRef(String),
}
```

### 3.5 CueIrCommand

```rust
/// `!` コマンド行の中間表現（7 バリアント）。
pub enum CueIrCommand {
    /// `!mark@名前` — 現在時刻に名前を付ける（マーカー登録）
    Mark { name: String },
    /// `!seek(@名前)` / `!seek(@名前, offset)` — 基準時刻をマーカーに設定
    Seek { name: String, offset: f64 },
    /// Barrier 系（WaitForInput / WaitForChoice / Timeout）
    Barrier(BarrierKind),
    /// `!clear`
    Clear,
    /// `!route_add(target, entity_key)`
    RouteAdd { target: CueTarget, to: EntityKey },
    /// `!route_switch(target, entity_key)`
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// `!route_remove(target)`
    RouteRemove { target: CueTarget },
}
```

### 3.6 CueIrAliasDef

```rust
/// 名前付きコマンド定義（シーンスコープ）。
pub struct CueIrAliasDef {
    /// actor ローカル定義なら Some(actor)、グローバル定義なら None
    pub scope_actor: Option<ActorKey>,
    /// エイリアス名（`@` 抜き）
    pub name: String,
    /// 対応する CueCommand
    pub command: CueCommand,
    /// ソース行番号
    pub source_line: u32,
}
```

### 3.7 型の依存関係

```
CueIrScene
├── Vec<CueIrEntry>
│   ├── Action(CueIrAction)
│   │   ├── actor: ActorKey          ← dola 型を再利用
│   │   └── fragments: Vec<CueIrFragment>
│   │       ├── Text(String)
│   │       └── AliasRef(String)
│   └── Command(CueIrCommand)
│       ├── Mark { name }
│       ├── Seek { name, offset }
│       ├── Barrier(BarrierKind)     ← dola 型を直接使用
│       ├── Clear
│       ├── RouteAdd { target: CueTarget, to: EntityKey }  ← dola 型
│       ├── RouteSwitch { target: CueTarget, to: EntityKey }
│       └── RouteRemove { target: CueTarget }
└── Vec<CueIrAliasDef>
    ├── name: String
    └── command: CueCommand          ← dola 型を直接使用
```

**重要**: `CueIrCommand` の `Barrier`, `RouteAdd`, `RouteSwitch`, `RouteRemove` バリアントは dola のドメイン型（`BarrierKind`, `CueTarget`, `EntityKey`）を直接使用する。`CueIrAliasDef` の `command` フィールドも dola の `CueCommand` を直接使用する。pasta_dsl は dola クレートに依存する。

---

## 4. PEG 文法ルール（grammar.pest への追加内容）

### 4.1 モード判定

```peg
// &type:cuesheet / ＆type：cuesheet に対応
// 既存の attr_line ルールのセマンティクス拡張として実装。
// key="type", value="cuesheet" のときシーンを CueSheet モードとしてマーク。
```

既存の属性行パースで `key=type`, `value=cuesheet` を検出したらシーンフラグを立てる。新規 PEG ルールの追加は不要で、パーサーのセマンティクス処理（AST 構築時）で分岐する。

### 4.2 `!` キューコマンド行（全角・半角両対応）

```peg
// キューコマンド行（cuesheet モード内のみ有効）
cue_cmd_line = {
    cue_cmd_marker ~ cue_cmd_body ~ NEWLINE
}

cue_cmd_marker = _{ "!" | "！" }

cue_cmd_body = {
    cue_mark
    | cue_emote_def
    | cue_choice_def
    | cue_custom_def
  | cue_seek
  | cue_yield
  | cue_select
  | cue_wait
  | cue_clear
  | cue_route_add
  | cue_route_switch
  | cue_route_remove
}
```

### 4.3 名前付き定義 + Mark ・ Seek（キーフレーム制御）

```peg
// !mark@名前  /  ！マーク＠名前
cue_mark = {
    ("mark" | "マーク") ~ at_marker ~ cue_ident
}

// !emote@名前(key)  /  ！表情＠名前（key）
cue_emote_def = {
    ("emote" | "表情") ~ at_marker ~ cue_ident ~ paren_open ~ SPACE* ~ cue_ident ~ SPACE* ~ paren_close
}

// !choice@名前(id, 「表示テキスト」)  /  ！選択肢＠名前（id, 「表示テキスト」）
cue_choice_def = {
    ("choice" | "選択肢") ~ at_marker ~ cue_ident ~ paren_open ~ SPACE* ~ cue_ident ~ SPACE* ~ "," ~ SPACE* ~ string_literal ~ SPACE* ~ paren_close
}

// !custom@名前(「command_name」, {json})  /  ！カスタム＠名前（「command_name」, {json}）
cue_custom_def = {
    ("custom" | "カスタム") ~ at_marker ~ cue_ident ~ paren_open ~ SPACE* ~ string_literal ~ SPACE* ~ "," ~ SPACE* ~ json_object ~ SPACE* ~ paren_close
}

// !seek(@名前)  /  !seek(@名前, offset)  /  ！シーク（＠名前, 0.5）
cue_seek = {
    ("seek" | "シーク") ~ paren_open ~ SPACE* ~ at_marker ~ cue_ident ~ (SPACE* ~ "," ~ SPACE* ~ float_lit)? ~ SPACE* ~ paren_close
}

// 共有プリミティブ
at_marker = _{ "@" | "＠" }
paren_open = _{ "(" | "（" }
paren_close = _{ ")" | "）" }
```

### 4.4 Barrier 系コマンド

```peg
// !yield  /  !yield(10.0)  /  ！区切り
cue_yield = {
    ("yield" | "区切り") ~ (paren_open ~ float_lit ~ paren_close)?
}

// !select  /  !select(30.0)  /  ！選択待ち
cue_select = {
    ("select" | "選択待ち") ~ (paren_open ~ float_lit ~ paren_close)?
}

// !wait(2.0)  /  ！待機（2.0）
cue_wait = {
    ("wait" | "待機") ~ paren_open ~ float_lit ~ paren_close
```peg
// !clear  /  ！クリア
cue_clear = { "clear" | "クリア" }
```

### 4.6 Routing コマンド

```peg
// !route_add(shell, actor:さくら:shell)  /  ！ルート追加（balloon, spot:stage）
cue_route_add = {
    ("route_add" | "ルート追加") ~ paren_open ~ cue_target ~ "," ~ SPACE* ~ entity_key ~ paren_close
}

// !route_switch(balloon, spot:stage_balloon)  /  ！ルート切替
cue_route_switch = {
    ("route_switch" | "ルート切替") ~ paren_open ~ cue_target ~ "," ~ SPACE* ~ entity_key ~ paren_close
}

// !route_remove(shell)  /  ！ルート削除（balloon）
cue_route_remove = {
    ("route_remove" | "ルート削除") ~ paren_open ~ cue_target ~ paren_close
}
```

### 4.7 EntityKey 記法

```peg
// EntityKey 記法（RouteAdd / RouteSwitch の to 引数）
entity_key = { entity_key_actor | entity_key_spot | entity_key_balloon }
entity_key_actor   = { "actor:" ~ cue_ident ~ ":" ~ cue_target }
entity_key_spot    = { "spot:"    ~ cue_ident }
entity_key_balloon = { "balloon:" ~ cue_ident }
```

**EntityKey 文字列形式の例**:

| 文字列 | 解釈 |
|--------|------|
| `actor:さくら:shell` | `EntityKey::Actor(ActorKey("さくら"), CueTarget::Shell)` |
| `actor:うにゅう:balloon` | `EntityKey::Actor(ActorKey("うにゅう"), CueTarget::Balloon)` |
| `spot:stage` | `EntityKey::Spot("stage")` |
| `balloon:special_balloon` | `EntityKey::Balloon("special_balloon")` |

### 4.8 CueTarget 識別子

```peg
// ターゲット識別子
cue_target = { "shell" | "balloon" | "シェル" | "バルーン" }
```

### 4.9 共通プリミティブ

```peg
// 識別子（スペース・括弧・カンマ・改行以外の文字列）
cue_ident = { (!(WHITESPACE | "(" | ")" | "（" | "）" | "," | NEWLINE) ~ ANY)+ }

// 非負浮動小数点リテラル
float_lit = { ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
```

### 4.10 名前付きコマンド定義

```peg
// 名前付き CueCommand 定義は !command@name(...) 形式で cue_cmd_body に含める
// `string_literal` は pasta 既存のルールを再利用。`「」` がプライマリ、`""` がオルタナティブ。
```

---

## 5. コマンドキーワード正式対照表

> **方針**: 英語キーワードを正規名、日本語キーワードを任意 alias とする。舞台用語は仕様理解を助ける注釈であり、実装・検索・レビューでは英語正規名を優先する。

| 正規名 | 日本語 alias | 舞台用語メモ | CueIR 出力型 |
|-------|-------------|--------------|-------------|
| `mark` | `マーク` | きっかけ点 | `CueIrCommand::Mark` |
| `seek` | `頭出し` | 頭出し | `CueIrCommand::Seek` |
| `emote` | `表情` | 表情替え | `CueIrAliasDef` |
| `choice` | `選択肢` | 応答候補 | `CueIrAliasDef` |
| `custom` | `カスタム` | 特殊キュー | `CueIrAliasDef` |
| `yield` | `入力待ち` | 客待ちに近い停止点 | `CueIrCommand::Barrier(WaitForInput)` |
| `select` | `選択待ち` | 応答待ち | `CueIrCommand::Barrier(WaitForChoice)` |
| `wait` | `待機` | 間を置く | `CueIrCommand::Barrier(Timeout)` |
| `clear` | `クリア` | 表示整理 | `CueIrCommand::Clear` |
| `route_add` | `ルート追加` | 配送先を立てる | `CueIrCommand::RouteAdd` |
| `route_switch` | `ルート切替` | 配送先転換 | `CueIrCommand::RouteSwitch` |
| `route_remove` | `ルート削除` | 配送先を外す | `CueIrCommand::RouteRemove` |
| `shell` | `シェル` | 見た目の出し先 | `CueTarget::Shell` |
| `balloon` | `バルーン` | 台詞の出し先 | `CueTarget::Balloon` |
| `Choice` | `選択肢` | 応答候補 | `CueCommand::Choice` |
| `Emote` | `表情` | 表情替え | `CueCommand::Emote` |
| `Custom` | `カスタム` | 特殊キュー | `CueCommand::Custom` |

---

## 6. DSL 記法 → CueIR マッピング対応表

### 6.1 アクション行

| DSL 記法 | CueIR 出力 | 備考 |
|---------|-----------|------|
| `actor：テキスト` | `CueIrEntry::Action { actor, fragments: [Text("テキスト")] }` | 基本パターン |
| `：継続テキスト` | 前行 Action の fragments 末尾 Text に `\n` 結合 | 継続行 |
| `actor：@alias名` | `Action { fragments: [AliasRef("alias名")] }` | エイリアス参照 |
| `actor：@unknown` | `Action { fragments: [AliasRef("unknown")] }` | 未定義でも AliasRef として出力（解決は dola 側） |
| `actor：テキスト@cmd テキスト2` | `Action { fragments: [Text("テキスト"), AliasRef("cmd"), Text(" テキスト2")] }` | 1 行内複数 @command |

### 6.2 `!` コマンド行

| DSL 記法 | CueIR 出力 |
|---------|-----------|
| `!mark@名前` / `！マーク＠名前` | `Command(Mark { name: "名前" })` |
| `!seek(@名前)` | `Command(Seek { name: "名前", offset: 0.0 })` |
| `!seek(@名前, 0.5)` | `Command(Seek { name: "名前", offset: 0.5 })` |
| `!emote@えもじ(smile)` | `CueIrAliasDef { name: "えもじ", command: Emote { key: "smile" } }` |
| `!choice@はい(yes, 「はい！」)` | `CueIrAliasDef { name: "はい", command: Choice { id: "yes", text: "はい！" } }` |
| `!custom@func(「bell」, {})` | `CueIrAliasDef { name: "func", command: Custom { command: "bell", params: {} } }` |
| `!yield` / `！区切り` | `Command(Barrier(WaitForInput { timeout: None }))` |
| `!yield(10.0)` | `Command(Barrier(WaitForInput { timeout: Some(10.0) }))` |
| `!select(30.0)` / `！選択（30.0）` | `Command(Barrier(WaitForChoice { timeout: Some(30.0) }))` |
| `!wait(2.0)` / `！待機（2.0）` | `Command(Barrier(Timeout { duration: 2.0 }))` |
| `!clear` / `！クリア` | `Command(Clear)` |
| `!route_add(shell, actor:さくら:shell)` | `Command(RouteAdd { target: Shell, to: Actor(さくら, Shell) })` |
| `!route_add(balloon, spot:stage)` | `Command(RouteAdd { target: Balloon, to: Spot("stage") })` |
| `!route_switch(balloon, spot:stage)` | `Command(RouteSwitch { target: Balloon, to: Spot("stage") })` |
| `!route_remove(shell)` | `Command(RouteRemove { target: Shell })` |

### 6.3 名前付きコマンド定義

| DSL 記法 | CueIR 出力 |
|---------|-----------|
| `!emote@えもじ(smile)` | `CueIrAliasDef { name: "えもじ", command: Emote { key: "smile" } }` |
| `!choice@はい(yes, 「はい！」)` | `CueIrAliasDef { name: "はい", command: Choice { id: "yes", text: "はい！" } }` |
| `!custom@func(「bell」, {})` | `CueIrAliasDef { name: "func", command: Custom { command: "bell", params: {} } }` |

> **文字列リテラル**: `「」`（プライマリ）および `""` （オルタナティブ）。pasta 既存の `string_literal` ルールを再利用する。

### 6.4 アクター配置行

| DSL 記法 | 処理 |
|---------|------|
| `%さくら` | アクター "さくら" にスロット 0 を自動割り当て。CueIrScene のメタデータとして保持 |
| `%さくら、うにゅう＝２` | カンマ区切りで複数アクター指定。C# enum 式自動番号付け（さくら=0, うにゅう=2） |
| `%さくら、うにゅう、まりか` | さくら=0, うにゅう=1, まりか=2 と順番に自動採番 |

> **`%` 行記法ルール**: C# enum 式自動番号付け。値は `u32`。全角数字は半角に正規化。詳細は design.md 参照。

---

## 7. エラー型定義

### 7.1 CueParseError（PEG パース時のエラー）

```rust
/// pasta DSL 文法解析エラー（行番号・カラム番号付き）
#[derive(Debug, thiserror::Error)]
pub enum CueParseError {
    #[error("行 {line}:{col}: 不明なキューコマンド '{cmd}' — `!mark@`, `!yield` 等を確認してください")]
    UnknownCommand { line: u32, col: u32, cmd: String },

    #[error("行 {line}:{col}: 負のオフセット秒数 '{value}' — 0.0 以上の値を指定してください")]
    NegativeFloat { line: u32, col: u32, value: f64 },

    #[error("行 {line}:{col}: 名前付きコマンド定義の構文エラー — `!emote@名前(key)` 形式を確認してください")]
    InvalidAliasSyntax { line: u32, col: u32 },

    #[error("行 {line}:{col}: 不正なスロット番号 '{value}' — 0 以上の整数を指定してください")]
    InvalidSlotNumber { line: u32, col: u32, value: String },

    #[error("行 {line}:{col}: 継続行に @command が含まれています — 継続行内 @command は不許可")]
    AtCommandInContinuation { line: u32, col: u32 },
}
```

### 7.2 CueBuildError（dola 側 — 参考情報）

```rust
/// CueSheet 構築エラー（セマンティクス）— dola CueSheetBuilder が使用
#[derive(Debug, thiserror::Error)]
pub enum CueBuildError {
    #[error("シーン '{scene}' でマーク名 '{name}' が重複しています")]
    DuplicateMark { scene: String, name: String },

    #[error("シーン '{scene}' でマーク '{name}' は未登録です — `!mark@{name}` を事前に記述してください")]
    UnknownMark { scene: String, name: String },

    #[error("シーン '{scene}' でマーカー名 '{name}' はエイリアスと同名です — マーカー名とエイリアス名は差別化が必要です")]
    MarkAliasConflict { scene: String, name: String },

    #[error("シーン '{scene}' でマーク '{name}' に actor 指定はできません — mark はグローバルなタイムライン参照点です")]
    ActorScopedMarkUnsupported { scene: String, name: String },

    #[error("シーン '{scene}' でマーク '{name}' は 2 回以上使用されています — 1 つの mark は 1 回だけ刻印可能です")]
    MarkUsedMultipleTimes { scene: String, name: String },

    #[error("マーク参照のオフセット '{value}' が負数です")]
    NegativeOffset { value: f64 },
}
```

> **注意**: `CueBuildError` は dola 側に定義される。pasta_dsl 実装者が定義するのは `CueParseError` のみ。

---

## 8. dola 側インターフェース（参考情報）

pasta_dsl が出力した `CueIrScene` を dola がどう消費するかの参考情報。pasta_dsl 側はこれらを実装する必要はないが、CueIR の設計意図を理解するために掲載する。

### 8.1 DurationResolver トレイト

```rust
/// アクション行の所要時間を解決するトレイト。
pub trait DurationResolver {
    /// 指定アクションの所要時間（秒）を返す。
    fn resolve_duration(&self, actor: &ActorKey, action: &CueIrAction) -> f64;
}

/// 全アクションに固定時間を返す（テスト・プロトタイプ用）。
pub struct FixedDurationResolver {
    pub default_seconds: f64,
}
```

### 8.2 SlotRegistry トレイト

```rust
/// アクター→スロット割り当てを管理するトレイト。
pub trait SlotRegistry {
    fn get_slot_assignment(&self, actor: &ActorKey) -> Option<SlotId>;
    fn assign_explicit(&mut self, actor: ActorKey, slot: SlotId);
    fn next_available_slot(&self) -> SlotId;
    fn auto_assign(&mut self, actor: ActorKey) -> SlotId;
}
```

### 8.3 CueSheetBuilder

```rust
/// CueIrScene を CueSheet へ変換するビルダー。
pub struct CueSheetBuilder<R: DurationResolver, S: SlotRegistry> {
    resolver: R,
    slot_registry: S,
}

impl<R: DurationResolver, S: SlotRegistry> CueSheetBuilder<R, S> {
    pub fn new(resolver: R, slot_registry: S) -> Self;
    pub fn build(&mut self, scene: CueIrScene) -> Result<CueSheet, CueBuildError>;
}
```

### 8.4 Builder アルゴリズム概要

Builder は `CueIrScene.entries` を順番に処理する:

1. **Mark**: `current_time` をマークテーブルに名前付きで登録（MarkerRegistered 状態）。**エイリアステーブルに同名が存在する場合は `MarkAliasConflict` エラー。** actor 修飾付き名（例: `さくら:かぶせ`）は `ActorScopedMarkUnsupported` エラー。重複登録は `DuplicateMark` エラー。
2. **Seek**: `current_time = mark_table[name] + offset`。マーカーが Stamped 済みの場合はその時刻を使用
3. **Barrier / Clear / RouteAdd / RouteSwitch / RouteRemove**: SYSTEM_ACTOR の Cue として生成
4. **Action**: 
   - アクター初出現時は RouteAdd を自動生成（Shell + Balloon 両方）
    - fragments を順に処理: `Text` → `CueCommand::Text`, `AliasRef` → マークテーブル確認（MarkerRegistered なら Stamped に更新、Stamped 済みなら `MarkUsedMultipleTimes` エラー）→ それ以外なら actor ローカル定義 → グローバル定義の順でエイリアステーブル参照 → 未定義なら `Emote` フォールバック
   - `DurationResolver.resolve_duration()` で所要時間を取得し `current_time` を前進

---

## 9. 変更対象ファイル（推定）

| ファイル | 変更内容 |
|---------|---------|
| `grammar.pest` | `cue_cmd_line`・`cue_cmd_body`・各コマンドルール・`cue_emote_def`・`cue_choice_def`・`cue_custom_def`・`cue_target`・`entity_key`・`float_lit`・`cue_ident` の追加 |
| `ast/*.rs` | `CueIrScene`, `CueIrEntry`, `CueIrAction`, `CueIrFragment`, `CueIrCommand`, `CueIrAliasDef` ノードの追加 |
| `parse_scene.rs` | `&type:cuesheet` モードスコープ処理の追加 |
| `parse_action.rs` | `@fragment` 分割ロジック、継続行 `\n` 結合処理の追加 |
| `error.rs` | `CueParseError` 型の追加 |

> 実際のファイル構成は pasta_dsl のリポジトリ構造に依存する。上記は推定。

---

## 10. 実装フェーズ計画（段階的 MVP）

### フェーズ A: 最小 MVP

**スコープ**: `&type:cuesheet` 認識 + アクション行（Text/Emote）+ 暗黙キーフレーム

| 対象 | 変更内容 |
|------|---------|
| `grammar.pest` | `cue_mode_attr` ルール（or 属性セマンティクス）, アクション行 `@cmd` フラグメント分割 |
| `ast/` | `CueIrScene`, `CueIrAction`, `CueIrFragment` |
| テスト | 基本的なアクション行パース → CueIrScene 生成の確認 |

**検証ポイント**:
- `&type:cuesheet` 付きシーンで `actor：content` が `CueIrAction` に変換される
- `@alias` が `AliasRef` フラグメントとして出力される
- 継続行が前行に `\n` 結合される
- 継続行内 `@command` でパースエラーが出る

### フェーズ B: Barrier + Mark/Seek 制御

**スコープ**: `!` コマンド行（Mark / Seek / Barrier / Clear）

| 対象 | 変更内容 |
|------|---------|
| `grammar.pest` | `cue_cmd_line` + 各 `cue_cmd_body` バリアント（mark/seek/yield/select/wait/clear） |
| `ast/` | `CueIrCommand` enum（Mark, Seek, Barrier, Clear） |
| テスト | `!mark@名前`, `!seek(@名前, offset)`, `!yield(10.0)`, `!clear` のパース確認 |

### フェーズ C: 名前付きコマンド定義 + Routing コマンド

**スコープ**: `!emote@...` / `!choice@...` / `!custom@...` + `!route_add` / `!route_switch` / `!route_remove`

| 対象 | 変更内容 |
|------|---------|
| `grammar.pest` | `cue_emote_def`・`cue_choice_def`・`cue_custom_def`、`cue_route_add`・`cue_route_switch`・`cue_route_remove` + `entity_key` |
| `ast/` | `CueIrAliasDef`、`CueIrCommand::RouteAdd` / `RouteSwitch` / `RouteRemove` |
| テスト | エイリアス定義パース、`!route_add(shell, actor:さくら:shell)` パース確認 |

### フェーズ D: 完全機能（Custom / EntityRef）

**スコープ**: `alias_custom_cmd` + `%` 行スロット明示割り当てのメタデータ出力

---

## 11. テスト戦略

### 11.1 ユニットテスト（pasta_dsl 側）

各 PEG ルールが期待通りの CueIR を生成することを確認:

| テストケース | 入力 | 期待出力 |
|------------|------|---------|
| 基本アクション行 | `さくら：こんにちは` | `Action { actor: "さくら", fragments: [Text("こんにちは")] }` |
| @command 付き | `さくら：@笑顔` | `Action { fragments: [AliasRef("笑顔")] }` |
| 1行内複数 @command | `さくら：ふふ@笑顔　あ！@驚き` | `Action { fragments: [Text("ふふ"), AliasRef("笑顔"), Text("　あ！"), AliasRef("驚き")] }` |
| 継続行 | `:続き` (前行あり) | 前行の fragments 末尾 Text に `\n続き` を結合 |
| 継続行 @command エラー | `：@cmd` | `CueParseError::AtCommandInContinuation` |
| Mark | `!mark@挨拶後` | `Command(Mark { name: "挨拶後" })` |
| Seek + offset | `!seek(@挨拶後, 1.0)` | `Command(Seek { name: "挨拶後", offset: 1.0 })` |
| WaitForInput | `!yield(10.0)` | `Command(Barrier(WaitForInput { timeout: Some(10.0) }))` |
| WaitForInput (no timeout) | `!yield` | `Command(Barrier(WaitForInput { timeout: None }))` |
| WaitForChoice | `!select(30.0)` | `Command(Barrier(WaitForChoice { timeout: Some(30.0) }))` |
| Timeout | `!wait(2.0)` | `Command(Barrier(Timeout { duration: 2.0 }))` |
| Clear | `!clear` | `Command(Clear)` |
| Clear (日本語) | `！クリア` | `Command(Clear)` |
| RouteAdd | `!route_add(shell, actor:さくら:shell)` | `Command(RouteAdd { target: Shell, to: Actor("さくら", Shell) })` |
| RouteSwitch | `!route_switch(balloon, spot:stage)` | `Command(RouteSwitch { target: Balloon, to: Spot("stage") })` |
| RouteRemove | `!route_remove(balloon)` | `Command(RouteRemove { target: Balloon })` |
| 名前付き表情定義 | `!emote@笑顔(smile)` | `CueIrAliasDef { name: "笑顔", command: Emote { key: "smile" } }` |
| actor ローカル表情定義 | `!emote@さくら:笑顔(sakura_smile)` | `CueIrAliasDef { scope_actor: Some("さくら"), name: "笑顔", command: Emote { key: "sakura_smile" } }` |
| 名前付き選択肢定義 | `!choice@はい(yes, 「はい！」)` | `CueIrAliasDef { name: "はい", command: Choice { id: "yes", text: "はい！" } }` |

### 11.2 Builder セマンティクステスト（dola 側）

構文が正しくパースされた後、意味解決で検証すべき主要観点を以下に整理する。

| 観点 | 入力 / 前提 | 期待結果 |
|------|-------------|----------|
| mark 名とエイリアス名の衝突 | `!emote@笑顔(smile)` 済みシーンで `!mark@笑顔` | `CueBuildError::MarkAliasConflict` |
| actor 修飾付き mark 拒否 | `!mark@さくら:転換点` | `CueBuildError::ActorScopedMarkUnsupported` |
| mark 重複登録 | `!mark@転換点` を同一シーンで 2 回定義 | `CueBuildError::DuplicateMark` |
| mark 単回使用の正常系 | `!mark@転換点` 後にアクション行で `@転換点` を 1 回だけ使用 | 最初の使用位置に刻印され、ビルド成功 |
| mark 多重使用拒否 | `!mark@転換点` 後にアクション行で `@転換点` を 2 回使用 | 2 回目で `CueBuildError::MarkUsedMultipleTimes` |
| actor ローカル優先解決 | `!emote@笑顔(common)` と `!emote@さくら:笑顔(sakura_smile)` が共存し、`さくら：@笑顔` | `Emote { key: "sakura_smile" }` を採用 |
| グローバル定義へのフォールバック | `!emote@会釈(common_bow)` のみ定義済みで `うにゅう：@会釈` | `Emote { key: "common_bow" }` を採用 |
| 未定義 alias の Emote フォールバック | alias 未定義で `さくら：@happy` | `Emote { key: "happy" }` を生成 |
| 未登録 mark 参照 | `!seek(@未登録)` | `CueBuildError::UnknownMark` |

### 11.3 後方互換性テスト

- `&type:cuesheet` なしのシーンで `!mark@name` が通常テキスト行として扱われること
- `&type:cuesheet` なしのシーンで `@alias` が通常の pasta ランダムワード参照として動作すること

### 11.4 インテグレーションテスト

| サンプルシーン | 主観点 | 期待結果 |
|--------------|--------|----------|
| `起動挨拶` | 基本アクション行、継続行、`!yield`、`!select`、`!clear`、未定義 alias フォールバック | `CueIrScene` 生成成功 |
| `デュエット挨拶` | `!mark`、`!seek(@name, offset)`、並列演出、`!wait` | `CueIrScene` 生成成功 |
| `表情豊かな発話` | 1 行内複数 `@command` | fragment 順序が保持される |
| `ローカル優先の表情` | actor ローカル alias 優先、グローバル fallback、mark 単回使用 | `CueIrScene` 生成成功、builder 正常系 fixture に流用可能 |
| `舞台演出` | `!route_add` / `!route_switch` / `!route_remove` | `EntityKey` を保持した `CueIrCommand` 生成 |
| `通常会話` | cuesheet モード外後方互換 | 拡張構文として扱わない |

### 11.5 非受理サンプル観点

`cue.pasta` は有効サンプルのみを保持し、拒否ケースは個別 fixture で管理する。

| fixture 名の例 | 入力例 | 期待結果 |
|---------------|--------|----------|
| `invalid_actor_scoped_mark.pasta` | `!mark@さくら:転換点` | builder で `CueBuildError::ActorScopedMarkUnsupported` |
| `invalid_mark_reuse.pasta` | `!mark@転換点` + `@転換点` を 2 回使用 | builder で `CueBuildError::MarkUsedMultipleTimes` |
| `invalid_unknown_mark_seek.pasta` | `!seek(@未登録)` | builder で `CueBuildError::UnknownMark` |

---

## 12. サンプルファイル

以下は全機能を網羅したサンプルファイルである（`cue.pasta` として提供済み）。

```pasta
# dola キューシート — pasta DSL 拡張設計仕様 (v3)
# ===========================================================================
# 【免責】本ファイルは pasta DSL 拡張仕様（dola-cue-pasta-dsl-extension）の
#          全機能網羅サンプルです。
#          現行の pasta_core ではコンパイルできません（拡張実装が完了してから
#          使用可能になります）。
#
# 拡張文法の 3 つの柱：
#   1. ! キューコマンド行  !mark@ / !seek(@...) / !yield 等 — 時系列・バリア制御
#   2. アクション行中の @alias 参照 — インラインの CueCommand 挿入
#   3. % 行のスロット指定  %さくら、うにゅう=1 等 — 既存配置文法の拡張利用
#
# ルーティング（RouteAdd）はアクション行の初出現から自動生成されます。
# スロット割り当ては % 行でカンマ区切り指定でき、省略時は自動採番されます。
# actor 修飾付き alias は `!command@actor:alias(...)` で定義できますが、mark は常にグローバル専用です。
# ===========================================================================

# ---------------------------------------------------------------------------
# シーン 1: 基本キューシート
# ---------------------------------------------------------------------------

＊起動挨拶

    %さくら

    !emote@普通(normal)
    !emote@笑顔(smile)
    !choice@はい(yes, 「はい、行きましょう！」)
    !choice@いいえ(no, 「今日は遠慮します」)

    さくら：@普通
    さくら：こんにちは！今日はいい天気ですね。
    ：お散歩の季節ですね。

    !mark@挨拶後

    さくら：@笑顔
    さくら：お散歩でも行きませんか？

    !yield(10.0)

    !clear
    さくら：@はい
    さくら：@いいえ
    !select(30.0)

    さくら：@happy よかった！

# ---------------------------------------------------------------------------
# シーン 2: 並列演出（かぶせ）
# ---------------------------------------------------------------------------

＊デュエット挨拶

    %さくら、うにゅう

    !emote@うれしい(happy)
    !emote@はにかみ(shy)

    さくら：こんにちはー！
    うにゅう：ごきげんよう！

    !mark@会話開始

    !seek(@会話開始)
    さくら：ねえ、何か食べに行かない？

    !seek(@会話開始)
    うにゅう：いいですわね、ケーキはいかが？

    !seek(@会話開始, 1.0)
    さくら：@うれしい
    さくら：決まりだね！

    !seek(@会話開始, 1.0)
    うにゅう：@はにかみ
    うにゅう：では参りましょう。

    !wait(2.0)
    !route_remove(balloon)

# ---------------------------------------------------------------------------
# シーン 3: 1 行内複数 @command
# ---------------------------------------------------------------------------

＊表情豊かな発話

    %さくら

    !emote@驚き(surprised)
    !emote@笑顔(smile)

    さくら：ふふーんいいでしょ。@笑顔　あ！@驚き

# ---------------------------------------------------------------------------
# シーン 4: actor ローカル alias 解決 + mark 単回使用
# ---------------------------------------------------------------------------

＊ローカル優先の表情
    ＆type：cuesheet

    %さくら、うにゅう

    !emote@会釈(common_bow)
    !emote@笑顔(common_smile)
    !emote@さくら:笑顔(sakura_smile)
    !emote@うにゅう:笑顔(unyuu_smile)
    !choice@さくら:承知(ok, 「もちろんです！」)

    さくら：@笑顔 わたし専用の笑顔です。
    うにゅう：@笑顔 わたくし専用の笑顔ですわ。
    うにゅう：@会釈 グローバル定義にも戻れますの。

    !mark@転換点
    さくら：ここで@転換点 場面転換です。
    !seek(@転換点, 0.5)
    さくら：@承知 了解しました。

＊ローカル優先の表情

    %さくら、うにゅう

    !emote@会釈(common_bow)
    !emote@笑顔(common_smile)
    !emote@さくら:笑顔(sakura_smile)
    !emote@うにゅう:笑顔(unyuu_smile)
    !choice@さくら:承知(ok, 「もちろんです！」)

    さくら：@笑顔 わたし専用の笑顔です。
    うにゅう：@笑顔 わたくし専用の笑顔ですわ。
    うにゅう：@会釈 グローバル定義にも戻れますの。

    !mark@転換点
    さくら：ここで@転換点 場面転換です。
    !seek(@転換点, 0.5)
    さくら：@承知 了解しました。

# ---------------------------------------------------------------------------
# シーン 5: 明示 !route_add / !route_switch
# ---------------------------------------------------------------------------

＊舞台演出

    !route_add(shell, actor:さくら:shell)
    !route_add(balloon, spot:stage_balloon)

    さくら：ここは舞台の上ですわ！

    !route_switch(balloon, actor:さくら:balloon)

    さくら：通常バルーンに戻りましたわ。

# ---------------------------------------------------------------------------
# 通常 pasta シーン（拡張構文を使わない例）
# ---------------------------------------------------------------------------

＊通常会話
    さくら：こちらは拡張構文を使わない通常会話です。
    さくら：既存の会話行はそのまま書けます。
    ：継続行も従来どおり使えます。
```

---

## 13. 制約と前提

### 13.1 依存クレート

| クレート | バージョン | 用途 |
|---------|----------|------|
| `pest` | 2.x | PEG パーサー生成 |
| `thiserror` | 2 | エラー型定義 |
| `dola` | （ワークスペース内） | `ActorKey`, `CueTarget`, `EntityKey`, `BarrierKind`, `CueCommand` 等のドメイン型 |

### 13.2 Rust Edition

Rust 2024 Edition を使用。

### 13.3 重要な制約

1. **pasta_dsl は時刻計算を行わない**: `CueIrScene` に `start_time` フィールドは存在しない。行の出現順序と構造のみを出力する。
2. **エイリアス解決は dola CueSheetBuilder の責務**: pasta_dsl は `AliasRef(name)` をそのまま出力する。エイリアステーブルの構築と解決、未定義時の `Emote` フォールバックは dola 側で実行する。
3. **RouteAdd 自動生成は dola CueSheetBuilder の責務**: pasta_dsl はアクション行の actor を `CueIrAction.actor` として出力するだけ。スロット割り当て判定と RouteAdd 自動生成は dola 側で実行する。
4. **明示 RouteAdd/RouteSwitch は pasta_dsl がパースする**: `!route_add(target, entity_key)` / `!route_switch(target, entity_key)` は PEG で解析し `CueIrCommand::RouteAdd` / `RouteSwitch` として出力する。
5. **後方互換性は絶対**: `&type:cuesheet` を持たないシーンの挙動は一切変更しない。

---

## 14. 用語集

| 用語 | 意味 |
|------|------|
| CueSheet | dola の演出台本データモデル。時刻付き `Cue` のリスト |
| Cue | 個々の演出指示。`actor` + `start_time` + `payload` |
| CueIR | pasta_dsl が出力する中間表現。時刻なし。CueSheet への変換は dola の CueSheetBuilder が行う |
| ActorKey | 演者識別子（文字列ベース）。さくらスクリプトの `\0` / `\1` に相当 |
| CueTarget | 配送先種別: `Shell`（キャラクター描画）/ `Balloon`（テキスト表示）|
| EntityKey | 配送先エンティティの識別子: `Actor(name, target)` / `Spot(name)` / `Balloon(name)` |
| BarrierKind | 進行停止点の種別: 入力待ち / 選択肢待ち / タイムアウト |
| RoutingCommand | 配送制御: RouteAdd（追加）/ RouteSwitch（切替）/ RouteRemove（除去）|
| SlotRegistry | アクター→スロット割り当て管理 API（dola 側トレイト）|
| DurationResolver | アクション行の所要時間を外部注入するインターフェース（dola 側トレイト）|
| 暗黙マーク | アクション行の終了時点に自動的に設定される基準時刻の進行点 |
| `%` 行 | アクター配置行。`%actor、actor＝N` 形式でスロットを割り当て（C# enum 式自動番号付け） |
| pasta DSL | 行指向のドメイン固有言語。ゴーストスクリプトの記述に使用 |
| 伺か | デスクトップマスコットアプリのプラットフォーム。本プロジェクトの文脈基盤 |
