# Requirements Document

## Project Description (Input)

dolaクレートの拡張の準備。完了した「wintf-P0-dola-boundary」仕様において、キューシートを設計してもらったが、このキューシートのテキスト表現として、今dolaに組み込んでもらったpasta DSLの文法を拡張してキューシート表現ができるようにpasta DSLの拡張を「設計」する。本仕様はテキスト表現の設計が出来れば完了であり、コード実装は行わない。成果物として「cue.pasta」を実際に作成すること。現行のpasta DSLの文法を拡張するため、成果物の「cue.pasta」はpasta_coreでコンパイルできないことに留意せよ。最終成果物はpasta_coreへの仕様指示の作成。

## Introduction

本ドキュメントは、`dola` クレートの `CueSheet` データモデルを pasta DSL のテキストとして記述可能にするための **pasta DSL 文法拡張** に関する要件を定義する。

対象は実装ではなく **設計と仕様策定** である。成果物は以下の2点：
1. **`cue.pasta`** — 拡張文法の動作サンプルファイル（全コマンド網羅）
2. **`design.md`（pasta_core 実装指示）** — pasta_core パーサーへの変更仕様

本拡張は現行 pasta DSL の後方互換性を維持しつつ、キューシートモード専用の新構文を追加する。

---

## Requirements

### 要件 1: キューシートモード識別

**Objective:** スクリプト作者として、シーン単位でキューシートモードを有効化できる仕組みがほしい。そうすることで、通常の pasta 会話シーンとキューシート演出シーンを同一ファイル内に混在させることができる。

#### Acceptance Criteria

1. When シーン定義の直後にある属性行として `＆type：cuesheet`（または半角 `&type:cuesheet`）が記述された場合、the pasta DSL 拡張パーサー shall そのシーンをキューシートモードとして解釈する。
2. The pasta DSL 拡張パーサー shall `＆type：cuesheet` が存在しないシーンを、現行 pasta 文法として扱い、タイムスタンプ記法・`\cue_*` トークンを有効化しない。
3. When `＆type：cuesheet` 属性がグローバルシーン（`＊`）に付与された場合、the pasta DSL 拡張パーサー shall そのグローバルシーン全体をキューシートモードとして解釈する。
4. Where ローカルシーン（`・`）への `＆type：cuesheet` 付与が仕様範囲に含まれる場合、the pasta DSL 拡張パーサー shall ローカルシーン単位でのモード指定を「将来拡張」として定義し、現バージョンではエラーとする。
5. The pasta DSL 拡張パーサー shall `＆type：cuesheet` を既存の属性行構文規則（シーン定義の直後にのみ配置可能）に従って解析する。

---

### 要件 2: タイムスタンプ記法の文法

**Objective:** スクリプト作者として、各演出コマンドに相対時刻（秒単位）を付与したい。そうすることで、並行演出や細かい時間制御を宣言的に記述できる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall キューシートモードのアクション行において `[<秒>] アクター：コンテンツ` 形式を有効な行記法として認識する。
2. The pasta DSL 拡張パーサー shall タイムスタンプ値として 0.0 以上の浮動小数点リテラル（例：`0.0`, `1.5`, `10`）を受け入れ、対応 `Cue` の `start_time` フィールドに設定する。
3. If タイムスタンプ値が負数の場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
4. The pasta DSL 拡張パーサー shall タイムスタンプと後続のアクション行の間に任意の空白文字（スペース・タブ・全角空白）を許容する。
5. The pasta DSL 拡張パーサー shall 同一タイムスタンプを持つ複数のアクション行を合法と認識し、それぞれ別の `Cue` エントリとして生成する（並行演出表現のため）。

---

### 要件 3: タイムスタンプの継承ルール

**Objective:** スクリプト作者として、同一時刻に連続するコマンドを記述する際にタイムスタンプを繰り返したくない。そうすることで、視認性の高いスクリプトを書ける。

#### Acceptance Criteria

1. When キューシートモードのアクション行でタイムスタンプが省略された場合、the pasta DSL 拡張パーサー shall そのシーン内で直前に指定されたタイムスタンプ値を継承する。
2. The pasta DSL 拡張パーサー shall シーン開始直後でタイムスタンプが一度も指定されていない場合の初期継承値を `0.0` とする。
3. The pasta DSL 拡張パーサー shall タイムスタンプ継承をシーンスコープ内に限定し、別シーンへ跨らせない。
4. While タイムスタンプ省略行において行継続記法（行頭 `：`）が使用された場合、the pasta DSL 拡張パーサー shall 行継続を CueSheet モードでは **非推奨構文** として扱い、警告を出しつつ前行のタイムスタンプ・アクターを継承した独立 `Cue` エントリとして展開する。

> **設計注記**: 行継続（単一 Cue を分割記述）と複数行 Cue（各行が独立エントリ）の意味論的差異は、CueSheet モードでは後者に統一する。

---

### 要件 4: CueCommand 系トークン

**Objective:** スクリプト作者として、演者に割り当てる6種の演出コマンドを pasta テキスト内で表現したい。そうすることで、dola の `CueCommand` 全バリアントをテキスト記法で完全に記述できる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall アクション行の本文が通常テキスト（`\cue_*` トークンなし）の場合、`CueCommand::Text(content)` にマッピングする。
2. The pasta DSL 拡張パーサー shall `\cue_clear` トークンを `CueCommand::Clear` にマッピングする。
3. The pasta DSL 拡張パーサー shall `\cue_emote[<key>]` トークンを `CueCommand::Emote { key }` にマッピングし、`<key>` を空でない任意の文字列として受け入れる。
4. The pasta DSL 拡張パーサー shall `\cue_choice[<id>, <text>]` トークンを `CueCommand::Choice { id, text }` にマッピングする。`id` は最初のカンマ+空白区切りの前まで、`text` は残り全体（日本語読点・記念符号を含む）とする。
5. The pasta DSL 拡張パーサー shall `\cue_entity[<u64>]` トークンを `CueCommand::EntityRef(u64)` にマッピングし、`<u64>` を 0 以上 u64::MAX 以下の整数リテラルとして検証する。
6. The pasta DSL 拡張パーサー shall `\cue_cmd[<name>, <json>]` トークンを `CueCommand::Custom { command: name, params: json }` にマッピングする。JSON 引数内に `]` 文字が含まれるケース（配列リテラル等）への対処方針は設計フェーズで決定する。
7. If `\cue_choice` の `id` または `text` が空文字列の場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
8. If `\cue_cmd` の `<json>` が有効な JSON オブジェクト形式（`{...}`）でない場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
9. The pasta DSL 拡張パーサー shall 1 つのアクション行に含める `\cue_*` トークンを 1 つに制限し、複数トークンが含まれる場合はパースエラーを報告する。

---

### 要件 5: BarrierKind 系トークン

**Objective:** スクリプト作者として、演出の進行停止点（入力待ち・選択待ち・タイムアウト）を宣言したい。そうすることで、インタラクティブな演出フローをキューシートとして記述できる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall `\cue_wait_input` トークンを `BarrierKind::WaitForInput { timeout: None }` にマッピングする。
2. The pasta DSL 拡張パーサー shall `\cue_wait_input[<sec>]` トークンを `BarrierKind::WaitForInput { timeout: Some(sec) }` にマッピングし、`<sec>` を正の浮動小数点数として検証する。
3. The pasta DSL 拡張パーサー shall `\cue_wait_choice` トークンを `BarrierKind::WaitForChoice { timeout: None }` にマッピングする。
4. The pasta DSL 拡張パーサー shall `\cue_wait_choice[<sec>]` トークンを `BarrierKind::WaitForChoice { timeout: Some(sec) }` にマッピングする。
5. The pasta DSL 拡張パーサー shall `\cue_timeout[<sec>]` トークンを `BarrierKind::Timeout { duration: sec }` にマッピングし、`<sec>` を正値（> 0.0）の浮動小数点数として検証する。
6. If `\cue_wait_input` または `\cue_wait_choice` の引数（`<sec>`）に 0 以下の値が指定された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
7. If `\cue_timeout` の `<sec>` 引数が 0 以下の場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。

---

### 要件 6: RoutingCommand 系トークン

**Objective:** スクリプト作者として、CueCommand の配送先（エンティティ）を動的に設定・切替・解除できるよう宣言したい。そうすることで、ECS エンティティへの演出ルーティングをキューシート内で完結させることができる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall `\cue_route_add[<target>, <entity>]` トークンを `RoutingCommand::RouteAdd { target, to: entity }` にマッピングする。
2. The pasta DSL 拡張パーサー shall `\cue_route_switch[<target>, <entity>]` トークンを `RoutingCommand::RouteSwitch { target, to: entity }` にマッピングする。
3. The pasta DSL 拡張パーサー shall `\cue_route_remove[<target>]` トークンを `RoutingCommand::RouteRemove { target }` にマッピングする。
4. If `<target>` が `shell` でも `balloon` でもない文字列の場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
5. If `<entity>` が要件 7 で定義するいずれの EntityKey 記法にも一致しない場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。

---

### 要件 7: 引数エンコーディング規則

**Objective:** スクリプト作者として、CueTarget・EntityKey・JSON を一貫した文字列記法でトークン引数に記述したい。そうすることで、パースが決定的かつエラーなく機能する。

#### CueTarget 記法

| テキスト記法 | dola 型              |
| ------------ | -------------------- |
| `shell`      | `CueTarget::Shell`   |
| `balloon`    | `CueTarget::Balloon` |

#### EntityKey 記法

| テキスト記法           | dola 型                                                  |
| ---------------------- | -------------------------------------------------------- |
| `actor:<name>:shell`   | `EntityKey::Actor(ActorKey(<name>), CueTarget::Shell)`   |
| `actor:<name>:balloon` | `EntityKey::Actor(ActorKey(<name>), CueTarget::Balloon)` |
| `spot:<name>`          | `EntityKey::Spot(<name>)`                                |
| `balloon:<name>`       | `EntityKey::Balloon(<name>)`                             |

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall `shell` 文字列を `CueTarget::Shell`、`balloon` 文字列を `CueTarget::Balloon` に厳密マッチングで変換する（大文字小文字を区別する）。
2. The pasta DSL 拡張パーサー shall `actor:<name>:shell` および `actor:<name>:balloon` 記法を `EntityKey::Actor` にマッピングし、`<name>` として空でない任意の文字列（コロン `:`・`]` を除く）を許容する。
3. The pasta DSL 拡張パーサー shall `spot:<name>` を `EntityKey::Spot`、`balloon:<name>` を `EntityKey::Balloon` にマッピングする。
4. The pasta DSL 拡張パーサー shall `\cue_cmd` の JSON 引数について、JSON 内部に `]` が含まれるケースでも正しく引数終端を識別できるパース規則を適用する。具体的なパース戦略は設計フェーズで決定する。
5. The pasta DSL 拡張パーサー shall `\cue_choice` の第1引数（id）をカンマ区切りの第1トークンとし、第2引数（text）を**最初の `, `（カンマ＋スペースまたはカンマ＋全角スペース）以降の全文字列**として解釈する。
6. The pasta DSL 拡張パーサー shall `\cue_choice` における text の中に日本語読点（`、`）が含まれる場合も正常に受け入れる。

---

### 要件 8: 後方互換性と既存文法との共存

**Objective:** プロジェクト関係者として、既存の pasta スクリプトが変更なく動作し続けることを保証したい。そうすることで、拡張導入による既存コンテンツの破壊リスクをゼロにできる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall `＆type：cuesheet` を持たないシーンを完全に現行 pasta DSL 仕様で処理し、タイムスタンプ記法・`\cue_*` トークンを解釈しない。
2. While キューシートモードが無効な通常シーンにおいて、`[0.5]` のような文字列が本文に含まれる場合、the pasta DSL 拡張パーサー shall その文字列を通常テキストとして扱う。
3. The pasta DSL 拡張パーサー shall `\cue_*` プレフィックスのトークンをキューシートモード外では未知の Sakura スクリプトとして現行どおり処理する（通常版の pasta_core の挙動に準拠）。
4. The pasta DSL 拡張パーサー shall `＆type` 属性の Pasta 既存属性構文（シーン定義直後のみ配置可能、アクション行の後には配置不可）を変更しない。

---

### 要件 9: エラーハンドリング

**Objective:** スクリプト作者として、文法エラーの箇所が特定できるエラーメッセージを受け取りたい。そうすることで、キューシート記述の誤りを迅速に修正できる。

#### Acceptance Criteria

1. If タイムスタンプが不正なリテラル（例：`[abc]`, `[-1.0]`）の場合、the pasta DSL 拡張パーサー shall 行番号・カラム番号・エラー種別を含むパースエラーを報告する。
2. If `\cue_*` トークンのトークン名が既知バリアントに一致しない場合（例：`\cue_unknown`）、the pasta DSL 拡張パーサー shall 未知トークンとしてエラーを報告する。
3. If `\cue_entity` の引数が u64 範囲外の場合、the pasta DSL 拡張パーサー shall 値範囲エラーを報告する。
4. If `\cue_cmd` の JSON 引数が `{` で始まらない、または対応する `}` がない場合、the pasta DSL 拡張パーサー shall JSON 構文エラーを報告する。
5. The pasta DSL 拡張パーサー shall `\cue_wait_choice` が当該タイムスタンプ内に `\cue_choice` の先積みなく出現した場合、**実装段階で検討すべき警告候補**としてパーサー仕様に記録する（パースエラーではなく意味論的警告扱い）。

---

### 要件 10: 設計成果物要件

**Objective:** プロジェクト関係者として、本仕様を元に pasta_core の実装者が文法拡張を実施できる状態の設計書と動作サンプルファイルを得たい。そうすることで、コード実装フェーズに確実に移行できる。

#### Acceptance Criteria

1. The pasta DSL 拡張仕様 shall 全コマンドバリアント（CueCommand 6種・BarrierKind 3種・RoutingCommand 3種）を網羅したサンプルシーンを含む `cue.pasta` ファイルを成果物として提供する。
2. The pasta DSL 拡張仕様 shall 並行演出（複数アクターの同一タイムスタンプ）を示すシーンを `cue.pasta` に含める。
3. The pasta DSL 拡張仕様 shall `design.md` に要件 1～9 の全拡張を実現するために必要な pasta_core 変更対象ファイル（`.pest` 文法ファイル・AST・パーサー・IR）と変更内容の指針を記載する。
4. The pasta DSL 拡張仕様 shall `design.md` に実装フェーズ計画（段階的 MVP）を記載し、最小実装（MVP）から段階的に拡張できる構成とする。
5. The pasta DSL 拡張仕様 shall `cue.pasta` を現行の pasta_core ではコンパイルできないことを明記した免責コメントを当ファイル冒頭に記載する。
6. The pasta DSL 拡張仕様 shall 引数エンコーディング規則・CueTarget 記法・EntityKey 記法を `design.md` に表形式で明示し、実装者が正確に参照できるようにする。
