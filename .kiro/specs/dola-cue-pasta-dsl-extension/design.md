# 設計書：pasta DSL キューシート拡張

## 概要

dola の `CueSheet`（`crates/dola/src/cue/`）をテキストで記述するための pasta DSL 文法拡張を定義する。
本設計書は pasta_core への実装指示として位置づける。

---

## 背景と制約

### dola CueSheet データモデル（実装済み）

```
CueSheet
└── Vec<Cue>
    └── Cue
        ├── actor: ActorKey          // 演者識別子
        ├── start_time: f64          // 相対開始秒
        └── payload: CuePayload
            ├── Command(CueCommand)
            │   ├── Text(String)
            │   ├── Clear
            │   ├── Emote { key: String }
            │   ├── Choice { id: String, text: String }
            │   ├── EntityRef(u64)
            │   └── Custom { command: String, params: DynamicValue }
            ├── Barrier(BarrierKind)
            │   ├── WaitForInput { timeout: Option<f64> }
            │   ├── WaitForChoice { timeout: Option<f64> }
            │   └── Timeout { duration: f64 }
            └── Routing(RoutingCommand)
                ├── RouteAdd { target: CueTarget, to: EntityKey }
                ├── RouteSwitch { target: CueTarget, to: EntityKey }
                └── RouteRemove { target: CueTarget }
```

### 設計方針

1. **既存文法の最小拡張** — 現行 pasta DSL の行指向モデルを維持する
2. **sakura スクリプト拡張** — コマンド記法は既存の `\token[bracket]` パターンを活用する
3. **名前空間プレフィックス `cue_`** — 既存 sakura スクリプトとの衝突を防ぐ
4. **後方互換性** — `＆type：cuesheet` がない通常のシーンは現行どおり動作する

---

## 拡張 1：タイムスタンプ記法

### 文法定義

```ebnf
timed_action_line ::= "[" time_value "]" SP+ action_line
time_value        ::= float_literal       (* 秒単位、0.0 以上 *)
```

### セマンティクス

- 行頭の `[<秒>]` が対応 Cue の `start_time` を指定する
- タイムスタンプ省略時：シーン内で最後に指定した値を継承（初期値 `0.0`）
- 複数行が同じタイムスタンプを持つことは合法（並行演出を表現）

### 例

```pasta
[0.5] さくら：こんにちは
[0.5] うにゅう：こんにちは  ＃ 同時刻で別アクター
[1.0] さくら：いい天気だね  ＃ 時刻進む
      うにゅう：そうですわね ＃ 省略 → t=1.0 継承
```

---

## 拡張 2：cue コマンド記法（sakura スクリプト拡張トークン）

アクション行の本文コンテンツとして、以下の `\cue_*` トークンを新たに認識する。
アクション行 1 行が **1 Cue** にマッピングされる（`actor` と `start_time` はその行で確定）。

> テキストを含む通常行（`actor：テキスト`）は `CueCommand::Text` として解釈する。

### 2-1. CueCommand 系

| pasta 記法                         | dola マッピング                                      |
| ---------------------------------- | ---------------------------------------------------- |
| `actor：テキスト`                  | `CueCommand::Text("テキスト")`                       |
| `actor：\cue_clear`                | `CueCommand::Clear`                                  |
| `actor：\cue_emote[<key>]`         | `CueCommand::Emote { key }`                          |
| `actor：\cue_choice[<id>, <text>]` | `CueCommand::Choice { id, text }`                    |
| `actor：\cue_entity[<u64>]`        | `CueCommand::EntityRef(u64)`                         |
| `actor：\cue_cmd[<name>, <json>]`  | `CueCommand::Custom { command: name, params: json }` |

**注意事項**
- `\cue_choice` は `\cue_wait_choice` の直前に連続投入すること（先積みプロトコル）
- `\cue_cmd` の `<json>` は DynamicValue に対応する JSON オブジェクト文字列

### 2-2. BarrierKind 系

| pasta 記法                       | dola マッピング                                     |
| -------------------------------- | --------------------------------------------------- |
| `actor：\cue_wait_input`         | `BarrierKind::WaitForInput { timeout: None }`       |
| `actor：\cue_wait_input[<sec>]`  | `BarrierKind::WaitForInput { timeout: Some(sec) }`  |
| `actor：\cue_wait_choice`        | `BarrierKind::WaitForChoice { timeout: None }`      |
| `actor：\cue_wait_choice[<sec>]` | `BarrierKind::WaitForChoice { timeout: Some(sec) }` |
| `actor：\cue_timeout[<sec>]`     | `BarrierKind::Timeout { duration: sec }`            |

### 2-3. RoutingCommand 系

| pasta 記法                                     | dola マッピング                                      |
| ---------------------------------------------- | ---------------------------------------------------- |
| `actor：\cue_route_add[<target>, <entity>]`    | `RoutingCommand::RouteAdd { target, to: entity }`    |
| `actor：\cue_route_switch[<target>, <entity>]` | `RoutingCommand::RouteSwitch { target, to: entity }` |
| `actor：\cue_route_remove[<target>]`           | `RoutingCommand::RouteRemove { target }`             |

---

## 拡張 3：引数エンコーディング規則

### CueTarget 記法

| 記法      | dola 型              |
| --------- | -------------------- |
| `shell`   | `CueTarget::Shell`   |
| `balloon` | `CueTarget::Balloon` |

### EntityKey 記法

| 記法                   | dola 型                                                  |
| ---------------------- | -------------------------------------------------------- |
| `actor:<name>:shell`   | `EntityKey::Actor(ActorKey(<name>), CueTarget::Shell)`   |
| `actor:<name>:balloon` | `EntityKey::Actor(ActorKey(<name>), CueTarget::Balloon)` |
| `spot:<name>`          | `EntityKey::Spot(<name>)`                                |
| `balloon:<name>`       | `EntityKey::Balloon(<name>)`                             |

---

## 拡張 4：シーン属性 `＆type：cuesheet`

```pasta
＊演出名
    ＆type：cuesheet
```

- **意味**：このシーンを CueSheet モードで解釈するマーク
- **パーサーへの影響**：タイムスタンプ記法と `\cue_*` トークンを有効化
- **省略時**：通常の pasta シーンとして扱われ、拡張文法は無効

---

## pasta_core への実装指示

### 変更対象ファイル（推定）

| ファイル                                      | 変更内容                                                      |
| --------------------------------------------- | ------------------------------------------------------------- |
| `crates/pasta_dsl/src/parser/grammar.pest`    | タイムスタンプ記法・`\cue_*` トークンの文法ルール追加         |
| `crates/pasta_dsl/src/parser/ast/*.rs`        | `TimedActionLine`, `CueDirective` AST ノード追加              |
| `crates/pasta_dsl/src/parser/parse_action.rs` | 新ルールに対応したパーサーロジック追加                        |
| `crates/pasta_dsl/src/parser/parse_scene.rs`  | CueSheet シーンスコープの処理追加                             |
| *(IR 層は pasta_lua 等の消費者側)*            | CueSheet 向けの IR 出力形式追加（対象クレートは設計時に確定） |

### 実装フェーズ提案

1. **フェーズ A（最小 MVP）**：タイムスタンプ記法 + Text/Clear/Emote の 3 コマンド
2. **フェーズ B**：Choice/WaitForChoice + ルーティングコマンド
3. **フェーズ C**：Custom/EntityRef + `\cue_cmd` の JSON パース

### 非実装事項（本設計の対象外）

- **CueSheet ↔ Storyboard 連携**（将来拡張）
  - CueSheet から Storyboard（連続値アニメーション）を起動する記法
  - Storyboard キーフレームを CueSheet 側の同期点として使用する記法
  - `start_time: f64` と `KeyframeRef` の相互変換
- タイムスタンプなし行の自動時刻割り当て戦略の最終決定（継承 vs. エラーは実装時に確定）
- `\cue_entity` の運用方法（ECS Entity bits の流通経路）は areka 側設計と調整が必要
- Lua ブロックからのキュー操作 API（将来拡張）

---

## 参照成果物

- [`cue.pasta`](cue.pasta) — 本拡張文法の動作サンプル（全コマンドを網羅）
- [`crates/dola/src/cue/command.rs`](../../../../crates/dola/src/cue/command.rs) — dola 側コマンド定義
- [`crates/dola/src/cue/sheet.rs`](../../../../crates/dola/src/cue/sheet.rs) — CueSheet 構造体
- [`vendors/pasta/GRAMMAR.md`](../../../../vendors/pasta/GRAMMAR.md) — 現行 pasta DSL 文法
