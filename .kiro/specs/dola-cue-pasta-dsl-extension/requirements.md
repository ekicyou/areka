# Requirements Document

## Project Description (Input)

dolaクレートの拡張の準備。完了した「wintf-P0-dola-boundary」仕様において、キューシートを設計してもらったが、このキューシートのテキスト表現として、今dolaに組み込んでもらったpasta DSLの文法を拡張してキューシート表現ができるようにpasta DSLの拡張を「設計」する。本仕様はテキスト表現の設計が出来れば完了であり、コード実装は行わない。成果物として「cue.pasta」を実際に作成すること。現行のpasta DSLの文法を拡張するため、成果物の「cue.pasta」はpasta_coreでコンパイルできないことに留意せよ。最終成果物はpasta_coreへの仕様指示の作成。

## Introduction

本ドキュメントは、`dola` クレートの `CueSheet` データモデルを pasta DSL のテキストとして記述可能にするための **pasta DSL 文法拡張** に関する要件を定義する。

対象は実装ではなく **設計と仕様策定** である。成果物は以下の2点：
1. **`cue.pasta`** — 拡張文法の動作サンプルファイル（全機能網羅）
2. **`design.md`（pasta_dsl 実装指示）** — pasta_dsl パーサーへの変更仕様

本拡張は現行 pasta DSL の **行単位の役割分担** という設計原則を維持しつつ、キューシートモード専用の新しい行種別を追加する。

### 設計方針

- **行指向文法の維持**: pasta DSL の基本原則である「1行＝1役割」を踏襲
- **暗黙キーフレーム**: 各アクション行の終了時点で自動的にキーフレームを生成し、次の要素の基準時刻とする
- **宣言的時系列制御**: キーフレーム名 + オフセット秒数による並列演出の記述
- **既存構造の活用**: アクター指定（`actor：content`）、属性（`&key：value`）、アクター配置（`％`行）など既存構文を可能な限り再利用

### スコープ

本仕様は **CueSheet 単体のテキスト表現** に集中する。以下は明示的にスコープ外とする：

- **CueSheet → Storyboard 起動**: CueSheet から連続値アニメーション（Storyboard）を起動する記法
- **Storyboard キーフレーム同期**: Storyboard のキーフレームを CueSheet 側の同期点として使用する記法
- **時刻・キーフレーム相互変換**: CueSheet の `start_time: f64` と Storyboard の `KeyframeRef` の連携

これらの連携機能は将来拡張として、専用の統合仕様で扱う。

---

## Requirements

### 要件 1: キューシートモード識別

**Objective:** スクリプト作者として、シーン単位でキューシートモードを有効化できる仕組みがほしい。そうすることで、通常の pasta 会話シーンとキューシート演出シーンを同一ファイル内に混在させることができる。

#### Acceptance Criteria

1. When シーン定義の直後にある属性行として `＆type：cuesheet`（または半角 `&type:cuesheet`）が記述された場合、the pasta DSL 拡張パーサー shall そのシーンをキューシートモードとして解釈する。
2. The pasta DSL 拡張パーサー shall `＆type：cuesheet` が存在しないシーンを、現行 pasta 文法として扱い、キューシート専用構文を有効化しない。
3. When `＆type：cuesheet` 属性がグローバルシーン（`＊`）に付与された場合、the pasta DSL 拡張パーサー shall そのグローバルシーン全体をキューシートモードとして解釈する。
4. The pasta DSL 拡張パーサー shall `＆type：cuesheet` を既存の属性行構文規則（シーン定義の直後にのみ配置可能）に従って解析する。

---

### 要件 2: 暗黙キーフレームの自動生成

**Objective:** スクリプト作者として、各アクション行の終了時点が自動的にキーフレームとして機能してほしい。そうすることで、明示的な時刻指定なしにシーケンシャルな会話フローを記述できる。

#### Acceptance Criteria

1. When キューシートモードでアクション行（`actor：content` 形式）が記述された場合、the pasta DSL 拡張パーサー shall その行の終了時点に暗黙的なキーフレームを生成する。
2. The pasta DSL 拡張パーサー shall 次のアクション行が時刻指定なしで記述された場合、直前の暗黙キーフレームを基準時刻（t=0.0）として扱う。
3. The pasta DSL 拡張パーサー shall シーン開始時の初期基準時刻を `0.0` とする。
4. The pasta DSL 拡張パーサー shall 暗黙キーフレームの時刻計算をシーンスコープ内に限定し、別シーンへ跨らせない。

---

### 要件 3: キューコマンド行（`!` 行）

**Objective:** スクリプト作者として、キーフレーム制御・Barrier指定などの演出制御コマンドを統一的な記法で記述したい。そうすることで、時系列制御とインタラクティブな演出フローを宣言的に表現できる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall キューシートモードで `!` または `！` で始まる行をキューコマンド行として認識する。
2. The pasta DSL 拡張パーサー shall キューコマンド行をシーンスコープ内の行種別として扱い、アクション行の本文に含めない。
3. The pasta DSL 拡張パーサー shall キューコマンド行で以下のコマンド種別を提供する：
   - **キーフレーム宣言**: 直前の暗黙キーフレームに名前を付与
   - **キーフレーム指定**: 指定キーフレーム + オフセット秒数を基準時刻として設定
   - **Barrier 指定**: dola `BarrierKind`（All/Any/Explicit）に対応する進行停止点
4. The pasta DSL 拡張パーサー shall キーフレーム名として空でない任意の文字列を許容する。
5. The pasta DSL 拡張パーザー shall キーフレーム指定のオフセット秒数として 0.0 以上の浮動小数点数を受け入れる。
6. If 同一シーン内で重複するキーフレーム名が宣言された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
7. If キーフレーム指定で未宣言のキーフレーム名が参照された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
8. The pasta DSL 拡張パーサー shall キーフレーム名のスコープをシーン単位に限定する。
9. The pasta DSL 拡張パーサー shall キーフレーム指定行以降のアクション行・エイリアス参照を、指定されたキーフレーム + オフセットを `start_time` として dola CueSheet に変換する。
10. The pasta DSL 拡張パーサー shall 同一キーフレーム + オフセットを持つ複数の要素を並列演出として認識し、それぞれ別の `Cue` エントリとして生成する。

> **設計注記**: 
> - 具体的なコマンド文法（キーフレーム宣言・指定・Barrier の記法）は design.md で決定する
> - 文法設計原則：
>   - **シンプルなフレーズ**: 冗長な記号を避け、意図が直感的に理解できる形式
>   - **英語・日本語両対応**: どちらの言語でも自然に記述可能
>   - **明瞭なコマンド名**: 省略表記を避け、プログラムソースコード級の明確な単語を使用（例：`barrier`、`keyframe`、`goto` など）
>   - **拡張性**: 将来的なコマンド追加（例：`timeout`、`wait` など）に対応可能な構造
> - 並列演出検出のロジック、RoutingCommand（Add/Switch/Remove）の自動生成規則は設計フェーズで詳細化する

---

### 要件 4: エイリアス定義行

**Objective:** スクリプト作者として、`@alias_name` に対して CueCommand の詳細（コマンド種別 + 引数）を定義したい。そうすることで、アクション行で `@alias_name` を参照するだけで CueCommand を挿入できる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall キューシートモードでエイリアス定義行（具体的な文法は設計フェーズで決定）を認識し、`@alias_name = CueCommand（引数）` 形式の定義を許可する。
2. The pasta DSL 拡張パーサー shall 以下の CueCommand バリアントに対応するエイリアス定義を提供する：
   - `Choice { id, text }` — 選択肢データ（例：`@はい = 選択肢（yes、「はい！」）`）
   - `Emote { key }` — 表情変更（例：`@笑顔 = 表情（笑顔）`）
   - `Custom { command, params }` — カスタムコマンド（将来拡張）
3. The pasta DSL 拡張パーサー shall エイリアス定義のスコープをシーン単位とし、同一シーン内で有効とする。
4. The pasta DSL 拡張パーサー shall グローバルスコープのエイリアス定義を将来拡張として考慮可能な文法とする（現バージョンでは未実装でも可）。
5. When アクション行で `@alias_name` が使用され、かつエイリアス定義が存在する場合、the pasta DSL 拡張パーサー shall そのエイリアスを対応する CueCommand に展開する。
6. When アクション行で `@alias_name` が使用され、エイリアス定義が存在せず、pasta DSL のランダムワード置換辞書にも存在しない場合、the pasta DSL 拡張パーサー shall フォールバックルール（例：`@笑顔 → Emote { key: "笑顔" }`）を適用する。

> **設計注記**: エイリアス定義行の文法（例：`@alias = 関数（引数）` 形式、コマンド名とパラメータのエンコード規則）、複数引数のカンマ区切りルール、日本語文字列の扱いは design.md で詳細化する。

---

### 要件 5: Say/Emote のアクション行対応

**Objective:** スクリプト作者として、既存の pasta DSL アクション行（`actor：content` および `@command` 記法）を使って dola `CueCommand::Text` と `CueCommand::Emote` を記述したい。そうすることで、通常の会話記述が自然にキューシートに変換される。

#### Acceptance Criteria

1. When キューシートモードでアクション行 `actor：content` が記述された場合、the pasta DSL 拡張パーサー shall `content` を `CueCommand::Text(content)` にマッピングする。
2. When アクション行で `@command` 記法（例：`さくら：こんにちわ@happy`）が使用された場合、the pasta DSL 拡張パーサー shall `@command` 部分を要件 4 のエイリアス解決ルールに従って処理し、エイリアス未定義の場合は `CueCommand::Emote { key: "happy" }` にフォールバックする。
3. The pasta DSL 拡張パーサー shall アクション行の `actor` 部分を ActorKey として解釈し、後続のルーティング自動生成（要件 6）の入力とする。
4. The pasta DSL 拡張パーサー shall 1つのアクション行に複数の `@command` が含まれる場合、設計フェーズで決定する挙動（最初のみ有効、最後のみ有効、エラー）を適用する。

---

### 要件 6: Routing の自動生成

**Objective:** スクリプト作者として、ルーティング制御（`RoutingCommand`）を明示的に記述せず、アクション行とアクター配置（`％`行）の組み合わせから自動生成してほしい。そうすることで、記述量を減らしつつ並列演出を実現できる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall キューシートモードでアクション行 `actor：content` を検出した際、その `actor` と現在のアクター配置（`％`行による定義）を照合し、適切な `RoutingCommand` を自動生成する。
2. When 同一 ActorKey が初めて出現した場合、the pasta DSL 拡張パーサー shall `RoutingCommand::RouteAdd { target, to }` を生成する。
3. When 同一 ActorKey が既にルーティング済みで、異なる CueTarget（shell/balloon）に切り替える場合、the pasta DSL 拡張パーサー shall `RoutingCommand::RouteSwitch { target, to }` を生成する。
4. When 並列演出（キーフレーム指定による同一時刻の複数アクター）が検出された場合、the pasta DSL 拡張パーサー shall 各アクターに対して `RouteAdd` を使用し、既存ルーティングを維持しつつ追加先を登録する。
5. The pasta DSL 拡張パーサー shall アクター配置（`％`行）の `％actor＝slot_id` 記法を解析し、ActorKey → スロット番号のマッピング情報をシーンスコープ内で保持する。
6. The pasta DSL 拡張パーサー shall `％`行が存在しない場合のデフォルトスロット割り当てルール（例：出現順に 0, 1, 2...）を設計フェーズで決定する。

> **設計注記**: ルーティング状態の追跡、Add/Switch の判定ロジック、並列演出時の RouteAdd 優先適用ルールは design.md で詳細化する。

---

### 要件 7: 後方互換性と既存文法との共存

**Objective:** プロジェクト関係者として、既存の pasta スクリプトが変更なく動作し続けることを保証したい。そうすることで、拡張導入による既存コンテンツの破壊リスクをゼロにできる。

#### Acceptance Criteria

1. The pasta DSL 拡張パーサー shall `＆type：cuesheet` を持たないシーンを完全に現行 pasta DSL 仕様で処理し、キューシート専用構文（キーフレーム宣言・指定行、Barrier 指定行、エイリアス定義行）を解釈しない。
2. The pasta DSL 拡張パーサー shall 既存の属性行構文（`&key：value`）、アクター配置（`％`行）、アクション行（`actor：content`）の挙動を変更しない。
3. The pasta DSL 拡張パーサー shall `@command` 記法の既存の挙動（ランダムワード置換辞書による置換、未定義時はそのまま残す）をキューシートモード外で維持する。
4. The pasta DSL 拡張パーサー shall キューシートモード内でも、既存の pasta DSL 構文（シーン定義、ローカルシーン、変数参照など）が適切に機能することを保証する。

---

### 要件 8: エラーハンドリング

**Objective:** スクリプト作者として、文法エラーの箇所が特定できるエラーメッセージを受け取りたい。そうすることで、キューシート記述の誤りを迅速に修正できる。

#### Acceptance Criteria

1. If キーフレーム指定行で未宣言のキーフレーム名が参照された場合、the pasta DSL 拡張パーサー shall 行番号・キーフレーム名を含むパースエラーを報告する。
2. If キーフレーム宣言行で重複したキーフレーム名が使用された場合、the pasta DSL 拡張パーサー shall 重複エラーを報告する。
3. If オフセット秒数に負数または不正なリテラルが指定された場合、the pasta DSL 拡張パーサー shall リテラル解析エラーを報告する。
4. If エイリアス定義行で不正な引数（例：空文字列 id、不正な JSON）が指定された場合、the pasta DSL 拡張パーサー shall 引数検証エラーを報告する。
5. If アクター配置（`％`行）で不正なスロット番号（例：負数、非整数）が指定された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
6. The pasta DSL 拡張パーサー shall 全てのパースエラーに対して、行番号・カラム番号・エラー種別・修正ヒントを含むメッセージを生成する。

---

### 要件 9: 設計成果物要件

**Objective:** プロジェクト関係者として、本仕様を元に pasta_dsl の実装者が文法拡張を実施できる状態の設計書と動作サンプルファイルを得たい。そうすることで、コード実装フェーズに確実に移行できる。

#### Acceptance Criteria

1. The pasta DSL 拡張仕様 shall 全機能（暗黙キーフレーム、キューコマンド行、エイリアス定義、Say/Emote、Routing 自動生成）を網羅したサンプルシーンを含む `cue.pasta` ファイルを成果物として提供する。
2. The pasta DSL 拡張仕様 shall 並行演出（複数アクターの同一キーフレーム + オフセット）を示すシーンを `cue.pasta` に含める。
3. The pasta DSL 拡張仕様 shall `design.md` に要件 1～8 の全機能を実現するために必要な pasta_dsl 変更対象ファイル（`.pest` 文法ファイル・AST・パーサー・IR）と変更内容の指針を記載する。
4. The pasta DSL 拡張仕様 shall `design.md` にキューコマンド行（`!` 行）の具体的な文法（EBNF または PEG 記法）を明示する。
5. The pasta DSL 拡張仕様 shall `design.md` に実装フェーズ計画（段階的 MVP）を記載し、最小実装（MVP）から段階的に拡張できる構成とする。
6. The pasta DSL 拡張仕様 shall `cue.pasta` を現行の pasta_dsl ではコンパイルできないことを明記した免責コメントを当ファイル冒頭に記載する。
7. The pasta DSL 拡張仕様 shall エイリアス定義の CueCommand 記法（コマンド名、引数エンコード規則）を `design.md` に表形式で明示し、実装者が正確に参照できるようにする。

---

## Notes

- **設計フェーズへの引き継ぎ事項**: 
    - キューコマンド行（`!` 行）の具体的文法（キーフレーム宣言・指定・Barrier の各コマンド記法）
    - エイリアス定義行の具体的文法とコマンド記法
    - RoutingCommand 自動生成ロジックの詳細
    - 並列演出検出のアルゴリズム
    - アクション行での複数 `@command` 処理方針
    - 文法設計原則：シンプル・明瞭・英語日本語両対応・拡張性

- **将来拡張候補**:
    - グローバルスコープのエイリアス定義
    - CueCommand::Move/Jump の記法
    - CueCommand::Custom の詳細パラメータ記法
    - キューコマンド行への新規コマンド追加（timeout、wait など）
    - Storyboard 統合（キーフレーム相互参照）


