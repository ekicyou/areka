# 実装依頼書: pasta DSL キューシートモード文法拡張

> **作成日**: 2026-03-03  
> **バージョン**: v1  
> **対象**: pasta_dsl クレート実装者（AI エージェント想定）  
> **参照元仕様**: `areka-wintf/.kiro/specs/dola-cue-pasta-dsl-extension/`

---

## 1. 概要

### 1.1 何を実装するのか

pasta DSL の PEG 文法（`grammar.pest`）を拡張し、**キューシートモード** という新しいシーンモードを追加する。このモードでは、dola クレートの `CueSheet` データモデルをテキストで記述できる。

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

`&type:cuesheet` 属性を持たないシーンは完全に現行 pasta DSL 仕様で動作する。キューシート専用構文（`!` コマンド行、エイリアス定義行）はこのモード内でのみ有効になる。

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
    /// `!keyframe <name>` — 現在時刻に名前を付ける
    KeyframeDecl { name: String },
    /// `!@<name>` / `!@<name>+<offset>` — 基準時刻をキーフレームに設定
    KeyframeRef { name: String, offset: f64 },
    /// Barrier 系（WaitForInput / WaitForChoice / Timeout）
    Barrier(BarrierKind),
    /// `!clear`
    Clear,
    /// `!route_add[target, entity_key]`
    RouteAdd { target: CueTarget, to: EntityKey },
    /// `!route_switch[target, entity_key]`
    RouteSwitch { target: CueTarget, to: EntityKey },
    /// `!route_remove[target]`
    RouteRemove { target: CueTarget },
}
```

### 3.6 CueIrAliasDef

```rust
/// エイリアス定義（シーンスコープ）。
pub struct CueIrAliasDef {
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
│       ├── KeyframeDecl { name }
│       ├── KeyframeRef { name, offset }
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
    (ASCII_EXCL | ZENKAKU_EXCL) ~ SPACE* ~ cue_cmd_body ~ NEWLINE
}

ZENKAKU_EXCL = { "！" }

cue_cmd_body = {
    cue_keyframe_decl
  | cue_keyframe_ref
  | cue_wait_input
  | cue_wait_choice
  | cue_timeout
  | cue_clear
  | cue_route_add
  | cue_route_switch
  | cue_route_remove
}
```

### 4.3 キーフレーム宣言・参照

```peg
// !keyframe <name>  /  ！キーフレーム <name>
cue_keyframe_decl = {
    ("keyframe" | "キーフレーム") ~ SPACE+ ~ cue_ident
}

// !@<name>  /  !@<name>+<offset>
cue_keyframe_ref = {
    "@" ~ cue_ident ~ (SPACE* ~ ("+" | "-") ~ SPACE* ~ float_lit)?
}
```

### 4.4 Barrier 系コマンド

```peg
// !wait_input  /  !wait_input[10.0]  /  ！入力待ち
cue_wait_input = {
    ("wait_input" | "入力待ち") ~ ("[" ~ float_lit ~ "]")?
}

// !wait_choice  /  !wait_choice[30.0]  /  ！選択肢待ち
cue_wait_choice = {
    ("wait_choice" | "選択肢待ち") ~ ("[" ~ float_lit ~ "]")?
}

// !timeout[2.0]  /  ！タイムアウト[2.0]
cue_timeout = {
    ("timeout" | "タイムアウト") ~ "[" ~ float_lit ~ "]"
}
```

### 4.5 Clear コマンド

```peg
// !clear  /  ！クリア
cue_clear = { "clear" | "クリア" }
```

### 4.6 Routing コマンド

```peg
// !route_add[shell, actor:さくら:shell]  /  ！ルート追加[balloon, spot:stage]
cue_route_add = {
    ("route_add" | "ルート追加") ~ "[" ~ cue_target ~ "," ~ SPACE* ~ entity_key ~ "]"
}

// !route_switch[balloon, spot:stage_balloon]  /  ！ルート切替
cue_route_switch = {
    ("route_switch" | "ルート切替") ~ "[" ~ cue_target ~ "," ~ SPACE* ~ entity_key ~ "]"
}

// !route_remove[shell]  /  ！ルート削除[balloon]
cue_route_remove = {
    ("route_remove" | "ルート削除") ~ "[" ~ cue_target ~ "]"
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
cue_ident = { (!(WHITESPACE | "(" | ")" | "[" | "]" | "," | NEWLINE) ~ ANY)+ }

// 非負浮動小数点リテラル
float_lit = { ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT+)? }
```

### 4.10 エイリアス定義行

```peg
// @alias名 = CueCommand(args)  /  @alias名 ＝ CueCommand(args)
alias_def_line = {
    "@" ~ cue_ident ~ SPACE* ~ ("=" | "＝") ~ SPACE* ~ alias_cmd_expr ~ NEWLINE
}

alias_cmd_expr = {
    alias_choice_cmd
  | alias_emote_cmd
  | alias_custom_cmd
}

// Choice(id, "表示テキスト")
alias_choice_cmd = {
    ("Choice" | "選択肢") ~ "(" ~ SPACE* ~ cue_ident ~ SPACE* ~ "," ~ SPACE* ~ quoted_string ~ SPACE* ~ ")"
}

// Emote(key)
alias_emote_cmd = {
    ("Emote" | "表情") ~ "(" ~ SPACE* ~ cue_ident ~ SPACE* ~ ")"
}

// Custom("command_name", {json})
alias_custom_cmd = {
    ("Custom" | "カスタム") ~ "(" ~ SPACE* ~ quoted_string ~ SPACE* ~ "," ~ SPACE* ~ json_object ~ SPACE* ~ ")"
}
```

---

## 5. コマンドキーワード英日対応表

| 英語キーワード | 日本語キーワード | CueIR 出力型 |
|-------------|---------------|-------------|
| `keyframe` | `キーフレーム` | `CueIrCommand::KeyframeDecl` |
| `wait_input` | `入力待ち` | `CueIrCommand::Barrier(WaitForInput)` |
| `wait_choice` | `選択肢待ち` | `CueIrCommand::Barrier(WaitForChoice)` |
| `timeout` | `タイムアウト` | `CueIrCommand::Barrier(Timeout)` |
| `clear` | `クリア` | `CueIrCommand::Clear` |
| `route_add` | `ルート追加` | `CueIrCommand::RouteAdd` |
| `route_switch` | `ルート切替` | `CueIrCommand::RouteSwitch` |
| `route_remove` | `ルート削除` | `CueIrCommand::RouteRemove` |
| `shell` | `シェル` | `CueTarget::Shell` |
| `balloon` | `バルーン` | `CueTarget::Balloon` |
| `Choice` | `選択肢` | `CueCommand::Choice` |
| `Emote` | `表情` | `CueCommand::Emote` |
| `Custom` | `カスタム` | `CueCommand::Custom` |

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
| `!keyframe 名前` / `！キーフレーム 名前` | `Command(KeyframeDecl { name: "名前" })` |
| `!@名前` | `Command(KeyframeRef { name: "名前", offset: 0.0 })` |
| `!@名前+0.5` | `Command(KeyframeRef { name: "名前", offset: 0.5 })` |
| `!wait_input` | `Command(Barrier(WaitForInput { timeout: None }))` |
| `!wait_input[10.0]` | `Command(Barrier(WaitForInput { timeout: Some(10.0) }))` |
| `!wait_choice[30.0]` | `Command(Barrier(WaitForChoice { timeout: Some(30.0) }))` |
| `!timeout[2.0]` | `Command(Barrier(Timeout { duration: 2.0 }))` |
| `!clear` / `！クリア` | `Command(Clear)` |
| `!route_add[shell, actor:さくら:shell]` | `Command(RouteAdd { target: Shell, to: Actor(さくら, Shell) })` |
| `!route_add[balloon, spot:stage]` | `Command(RouteAdd { target: Balloon, to: Spot("stage") })` |
| `!route_switch[balloon, spot:stage]` | `Command(RouteSwitch { target: Balloon, to: Spot("stage") })` |
| `!route_remove[shell]` | `Command(RouteRemove { target: Shell })` |

### 6.3 エイリアス定義行

| DSL 記法 | CueIR 出力 |
|---------|-----------|
| `@えもじ = Emote(smile)` | `CueIrAliasDef { name: "えもじ", command: Emote { key: "smile" } }` |
| `@はい = Choice(yes, "はい！")` | `CueIrAliasDef { name: "はい", command: Choice { id: "yes", text: "はい！" } }` |
| `@func = Custom("bell", {})` | `CueIrAliasDef { name: "func", command: Custom { command: "bell", params: {} } }` |

### 6.4 アクター配置行

| DSL 記法 | 処理 |
|---------|------|
| `%さくら=0` | アクター "さくら" をスロット 0 に明示割り当て。CueIrScene のメタデータとして保持 |

---

## 7. エラー型定義

### 7.1 CueParseError（PEG パース時のエラー）

```rust
/// pasta DSL 文法解析エラー（行番号・カラム番号付き）
#[derive(Debug, thiserror::Error)]
pub enum CueParseError {
    #[error("行 {line}:{col}: 不明なキューコマンド '{cmd}' — `!keyframe`, `!wait_input` 等を確認してください")]
    UnknownCommand { line: u32, col: u32, cmd: String },

    #[error("行 {line}:{col}: 負のオフセット秒数 '{value}' — 0.0 以上の値を指定してください")]
    NegativeFloat { line: u32, col: u32, value: f64 },

    #[error("行 {line}:{col}: エイリアス定義の構文エラー — `@名前 = Emote(key)` 形式を確認してください")]
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
    #[error("シーン '{scene}' でキーフレーム名 '{name}' が重複しています")]
    DuplicateKeyframe { scene: String, name: String },

    #[error("シーン '{scene}' でキーフレーム '{name}' は未宣言です — `!keyframe {name}` を事前に記述してください")]
    UnknownKeyframe { scene: String, name: String },

    #[error("キーフレーム参照のオフセット '{value}' が負数です")]
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

1. **KeyframeDecl**: `current_time` をキーフレームテーブルに名前付きで登録
2. **KeyframeRef**: `current_time = keyframe_table[name] + offset`
3. **Barrier / Clear / RouteAdd / RouteSwitch / RouteRemove**: SYSTEM_ACTOR の Cue として生成
4. **Action**: 
   - アクター初出現時は RouteAdd を自動生成（Shell + Balloon 両方）
   - fragments を順に処理: `Text` → `CueCommand::Text`, `AliasRef` → エイリアステーブル参照 → 未定義なら `Emote` フォールバック
   - `DurationResolver.resolve_duration()` で所要時間を取得し `current_time` を前進

---

## 9. 変更対象ファイル（推定）

| ファイル | 変更内容 |
|---------|---------|
| `grammar.pest` | `cue_cmd_line`・`cue_cmd_body`・各コマンドルール・`alias_def_line`・`cue_target`・`entity_key`・`float_lit`・`cue_ident` の追加 |
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

### フェーズ B: Barrier + キーフレーム制御

**スコープ**: `!` コマンド行（KeyframeDecl / KeyframeRef / Barrier / Clear）

| 対象 | 変更内容 |
|------|---------|
| `grammar.pest` | `cue_cmd_line` + 各 `cue_cmd_body` バリアント |
| `ast/` | `CueIrCommand` enum（KeyframeDecl, KeyframeRef, Barrier, Clear） |
| テスト | `!keyframe`, `!@name+offset`, `!wait_input[10.0]`, `!clear` のパース確認 |

### フェーズ C: エイリアス定義 + Routing コマンド

**スコープ**: `alias_def_line` + `!route_add` / `!route_switch` / `!route_remove`

| 対象 | 変更内容 |
|------|---------|
| `grammar.pest` | `alias_def_line` + `alias_cmd_expr`、`cue_route_add`・`cue_route_switch`・`cue_route_remove` + `entity_key` |
| `ast/` | `CueIrAliasDef`、`CueIrCommand::RouteAdd` / `RouteSwitch` / `RouteRemove` |
| テスト | エイリアス定義パース、`!route_add[shell, actor:さくら:shell]` パース確認 |

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
| KeyframeDecl | `!keyframe 挨拶後` | `Command(KeyframeDecl { name: "挨拶後" })` |
| KeyframeRef + offset | `!@挨拶後+1.0` | `Command(KeyframeRef { name: "挨拶後", offset: 1.0 })` |
| WaitForInput | `!wait_input[10.0]` | `Command(Barrier(WaitForInput { timeout: Some(10.0) }))` |
| WaitForInput (no timeout) | `!wait_input` | `Command(Barrier(WaitForInput { timeout: None }))` |
| Timeout | `!timeout[2.0]` | `Command(Barrier(Timeout { duration: 2.0 }))` |
| Clear | `!clear` | `Command(Clear)` |
| Clear (日本語) | `！クリア` | `Command(Clear)` |
| RouteAdd | `!route_add[shell, actor:さくら:shell]` | `Command(RouteAdd { target: Shell, to: Actor("さくら", Shell) })` |
| RouteSwitch | `!route_switch[balloon, spot:stage]` | `Command(RouteSwitch { target: Balloon, to: Spot("stage") })` |
| RouteRemove | `!route_remove[balloon]` | `Command(RouteRemove { target: Balloon })` |
| エイリアス定義 | `@笑顔 = Emote(smile)` | `CueIrAliasDef { name: "笑顔", command: Emote { key: "smile" } }` |
| 選択肢エイリアス | `@はい = Choice(yes, "はい！")` | `CueIrAliasDef { name: "はい", command: Choice { id: "yes", text: "はい！" } }` |

### 11.2 後方互換性テスト

- `&type:cuesheet` なしのシーンで `!keyframe` が通常テキスト行として扱われること
- `&type:cuesheet` なしのシーンで `@alias` が通常の pasta ランダムワード参照として動作すること

### 11.3 インテグレーションテスト

- 付属サンプルファイル `cue.pasta`（セクション 12 参照）の全シーンがパースエラーなく `CueIrScene` に変換されること

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
#   1. キューシートモード  &type:cuesheet — シーン単位で拡張構文を有効化
#   2. ! キューコマンド行  !keyframe / !wait_input 等 — 時系列・バリア制御
#   3. エイリアス定義行   @alias = Emote(key) 等 — CueCommand の名前付き定義
#
# ルーティング（RouteAdd）はアクション行の初出現から自動生成されます。
# スロット割り当ては %actor=slot で明示でき、省略時は自動採番されます。
# ===========================================================================

# ---------------------------------------------------------------------------
# シーン 1: 基本キューシート
# ---------------------------------------------------------------------------

＊起動挨拶
    ＆type：cuesheet

    %さくら=0

    @普通 = Emote(normal)
    @笑顔 = Emote(smile)
    @はい = Choice(yes, "はい、行きましょう！")
    @いいえ = Choice(no, "今日は遠慮します")

    さくら：@普通
    さくら：こんにちは！今日はいい天気ですね。
    ：お散歩の季節ですね。

    !keyframe 挨拶後

    さくら：@笑顔
    さくら：お散歩でも行きませんか？

    !wait_input[10.0]

    !clear
    さくら：@はい
    さくら：@いいえ
    !wait_choice[30.0]

    さくら：@happy よかった！

# ---------------------------------------------------------------------------
# シーン 2: 並列演出
# ---------------------------------------------------------------------------

＊デュエット挨拶
    ＆type：cuesheet

    %さくら=0
    %うにゅう=1

    @うれしい = Emote(happy)
    @はにかみ = Emote(shy)

    さくら：こんにちはー！
    うにゅう：ごきげんよう！

    !keyframe 会話開始

    !@会話開始
    さくら：ねえ、何か食べに行かない？

    !@会話開始
    うにゅう：いいですわね、ケーキはいかが？

    !@会話開始+1.0
    さくら：@うれしい
    さくら：決まりだね！

    !@会話開始+1.0
    うにゅう：@はにかみ
    うにゅう：では参りましょう。

    !timeout[2.0]
    !route_remove[balloon]

# ---------------------------------------------------------------------------
# シーン 3: 1 行内複数 @command
# ---------------------------------------------------------------------------

＊表情豊かな発話
    ＆type：cuesheet

    %さくら=0

    @驚き = Emote(surprised)
    @笑顔 = Emote(smile)

    さくら：ふふーんいいでしょ。@笑顔　あ！@驚き

# ---------------------------------------------------------------------------
# シーン 5: 明示 !route_add / !route_switch
# ---------------------------------------------------------------------------

＊舞台演出
    ＆type：cuesheet

    !route_add[shell, actor:さくら:shell]
    !route_add[balloon, spot:stage_balloon]

    さくら：ここは舞台の上ですわ！

    !route_switch[balloon, actor:さくら:balloon]

    さくら：通常バルーンに戻りましたわ。

# ---------------------------------------------------------------------------
# 通常 pasta シーン（後方互換性確認）
# ---------------------------------------------------------------------------

＊通常会話
    さくら：こちらはキューシートモードではありません。
    さくら：!keyframe のような行も通常テキストとして扱われます。
    さくら：@happy も通常の pasta ランダムワード参照として動作します。
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
4. **明示 RouteAdd/RouteSwitch は pasta_dsl がパースする**: `!route_add[target, entity_key]` / `!route_switch[target, entity_key]` は PEG で解析し `CueIrCommand::RouteAdd` / `RouteSwitch` として出力する。
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
| 暗黙キーフレーム | アクション行の終了時点に自動的に設定される基準時刻の進行点 |
| `%` 行 | アクター配置行。`%actor=slot_id` 形式でスロットを明示割り当て |
| pasta DSL | 行指向のドメイン固有言語。ゴーストスクリプトの記述に使用 |
| 伺か | デスクトップマスコットアプリのプラットフォーム。本プロジェクトの文脈基盤 |
