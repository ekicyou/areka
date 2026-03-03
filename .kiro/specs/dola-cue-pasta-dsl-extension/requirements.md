# 要件定義書: dola-cue-pasta-dsl-extension

> **バージョン**: v3（2026-03-03 — Q1〜Q7 全議論を反映）

## プロジェクト概要（入力）

dola クレートの拡張の準備。完了した「wintf-P0-dola-boundary」仕様において設計された `CueSheet` データモデルについて、現行 pasta DSL の文法を拡張し、テキストとして記述できるようにする。

**本仕様のスコープ**: テキスト表現の設計および成果物ファイルの生成のみ。コード実装は行わない。  
**成果物**: `cue.pasta`（動作サンプル・全機能網羅） + `design.md`（pasta_core への実装仕様指示）

## イントロダクション

本ドキュメントは `dola` クレートの `CueSheet` データモデルを pasta DSL のテキストとして記述可能にするための **pasta DSL 文法拡張** に関する要件を定義する。

### 設計方針

- **行指向文法の維持**: pasta DSL の基本原則「1行＝1役割」を踏襲
- **暗黙キーフレーム**: 各アクション行の終了時点で自動的に基準時刻が進む。ただし時刻算出はパーサーの責務ではなく、外部注入する Duration Resolver トレイトが担う
- **責務分離の原則**: DSL は構造と順序を宣言する。タイミング制御・ライフサイクル管理はアプリ層の責務
- **既存構造の活用**: アクター指定（`actor：content`）、属性（`&key：value`）、アクター配置（`%`行）は既存文法を可能な限り再利用
- **英語・日本語両対応**: キーワードをどちらの言語でも自然に記述可能

### スコープ外

- **CueSheet → Storyboard 起動**: 連続値アニメーションとの連携記法
- **Storyboard キーフレーム同期**: CueSheet 側の同期点としての使用
- **時刻・キーフレーム相互変換**: `start_time: f64` と `KeyframeRef` の連携
- **コード実装**: すべての変更は設計・仕様の策定のみ

---

## 要件

### 要件 1: キューシートモード識別

**目的**: スクリプト作者として、シーン単位でキューシートモードを有効化できる仕組みがほしい。通常の pasta 会話シーンとキューシート演出シーンを同一ファイル内に混在させることができる。

#### 受入基準

1. When シーン定義の直後に属性行 `＆type：cuesheet`（または半角 `&type:cuesheet`）が記述された場合、the pasta DSL 拡張パーサー shall そのシーンをキューシートモードとして解釈する。
2. The pasta DSL 拡張パーサー shall `＆type：cuesheet` が存在しないシーンを現行 pasta 文法として扱い、キューシート専用構文（`!` コマンド行、エイリアス定義行）を有効化しない。
3. When `＆type：cuesheet` 属性がグローバルシーン（`＊`）に付与された場合、the pasta DSL 拡張パーサー shall そのグローバルシーン全体をキューシートモードとして解釈する。
4. The pasta DSL 拡張パーサー shall `＆type：cuesheet` を既存の属性行構文規則（シーン定義の直後にのみ配置可能）に従って解析する。

---

### 要件 2: 暗黙キーフレームと時刻算出パイプライン

**目的**: スクリプト作者として、各アクション行の終了時点が自動的にキーフレームとして機能してほしい。明示的な時刻指定なしにシーケンシャルな会話フローを記述できる。

#### 受入基準

1. When キューシートモードでアクション行（`actor：content` 形式）が記述された場合、the pasta DSL 拡張パーサー shall その行の終了時点に暗黙的な基準時刻の進行点を生成する。
2. When 次のアクション行が明示キーフレーム指定行（`!` コマンド）なしで記述された場合、the pasta DSL 拡張パーサー shall 直前の暗黙キーフレームを基準時刻の起点として扱う。
3. The pasta DSL 拡張パーサー shall シーン開始時の初期基準時刻を `0.0` とする。
4. The pasta DSL 拡張パーサー shall 暗黙キーフレームのスコープをシーン単位に限定し、別シーンへ跨がらせない。
5. The pasta DSL 拡張パーサー shall 各キューコマンドの所要時間（duration）を自ら算出せず、行の出現順序と構造情報のみを出力する。
6. The CueSheet 構築層 shall 所要時間を外部注入インターフェース（Duration Resolver トレイト）から取得し、`Cue.start_time` を算出する。dola パーサー層では所要時間を確定しない。

> **設計注記**: パーサー → IR（順序 + 構造） → Duration Resolver 注入 → CueSheet（時刻確定）というパイプラインを想定。Duration Resolver トレイトは CueSheet 初期化時またはビルダーに注入する。

---

### 要件 3: キューコマンド行（`!` 行）

**目的**: スクリプト作者として、キーフレーム制御・Barrier 指定・バルーンクリアなどの演出制御コマンドを統一的な記法で記述したい。時系列制御とインタラクティブな演出フローを宣言的に表現できる。

#### 受入基準

1. The pasta DSL 拡張パーサー shall キューシートモードで `!` または `！` で始まる行をキューコマンド行として認識する。
2. The pasta DSL 拡張パーサー shall キューコマンド行をシーンスコープ内の行種別として扱い、アクション行の本文に含めない。
3. The pasta DSL 拡張パーサー shall キューコマンド行で以下のコマンド種別を提供する:
   - **キーフレーム宣言**: 直前の暗黙キーフレームに名前を付与する
   - **キーフレーム指定**: 指定キーフレーム名 + オフセット秒数を、以降の行の基準時刻として設定する
   - **Barrier 指定**: dola `BarrierKind`（`WaitForInput` / `WaitForChoice` / `Timeout`）に対応する進行停止点
   - **Clear**: dola `CueCommand::Clear` に対応するバルーンクリア指令（明示記述時のみ生成）
4. The pasta DSL 拡張パーサー shall キーフレーム名として空でない任意の文字列を許容する。
5. The pasta DSL 拡張パーサー shall キーフレーム指定のオフセット秒数として 0.0 以上の浮動小数点数を受け入れる。
6. If 同一シーン内で重複するキーフレーム名が宣言された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
7. If キーフレーム指定で未宣言のキーフレーム名が参照された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
8. The pasta DSL 拡張パーサー shall キーフレーム名のスコープをシーン単位に限定する。
9. The pasta DSL 拡張パーサー shall キーフレーム指定行以降のアクション行を、指定されたキーフレーム + オフセットを基準時刻として dola CueSheet に変換する。
10. The pasta DSL 拡張パーサー shall 同一基準時刻を持つ複数の要素を並列演出として認識し、それぞれ別の `Cue` エントリとして生成する。
11. The pasta DSL 拡張パーサー shall `CueCommand::Clear` をスクリプト作者が `!clear` と明示記述した場合のみ生成する。シーン遷移時の自動 Clear 挿入は行わない（アプリ層の責務）。

> **設計注記**: 具体的なコマンド文法（各コマンドの記法・英語/日本語キーワード対応表）は design.md で決定する。

---

### 要件 4: エイリアス定義行

**目的**: スクリプト作者として、`@alias_name` に対して CueCommand の詳細（コマンド種別 + 引数）を定義したい。アクション行で `@alias_name` を参照するだけで CueCommand を挿入できる。

#### 受入基準

1. The pasta DSL 拡張パーサー shall キューシートモードでエイリアス定義行を認識し、`@alias_name = CueCommand(引数)` 形式（`=` / `＝` セパレータ）の定義を許可する。
2. The pasta DSL 拡張パーサー shall 以下の CueCommand バリアントに対応するエイリアス定義を提供する:
   - `Choice { id, text }` — 選択肢データ（例: `@はい = Choice(yes, "はい！")`）
   - `Emote { key }` — 表情変更（例: `@笑顔 = Emote(smile)`）
   - `Custom { command, params }` — カスタムコマンド（将来拡張）
3. The pasta DSL 拡張パーサー shall エイリアス定義のスコープをシーン単位とし、同一シーン内で有効とする。
4. The pasta DSL 拡張パーサー shall グローバルスコープのエイリアス定義を将来拡張として考慮可能な文法とする（現バージョンでは未実装でも可）。
5. When アクション行で `@alias_name` が使用され、かつエイリアス定義が存在する場合、the pasta DSL 拡張パーサー shall そのエイリアスを対応する CueCommand に展開する。
6. When アクション行で `@alias_name` が使用され、エイリアス定義が存在せず、pasta DSL のランダムワード置換辞書にも存在しない場合、the pasta DSL 拡張パーサー shall フォールバックとして `CueCommand::Emote { key: "alias_name" }` を生成する。

> **設計注記**:
> - エイリアス定義行の具体的な文法（コマンド名の英語/日本語対応表、引数エンコード規則、複数引数の区切り文字）は design.md で決定する。
> - AC 6 の Emote フォールバック根拠: `@command` の最頻出用途が表情変更（Emote）であるため、未定義時のフォールバックとして最も自然な選択となる。

---

### 要件 5: アクション行の CueCommand マッピング

**目的**: スクリプト作者として、既存の pasta DSL アクション行（`actor：content` および `@command` 記法）を使って dola `CueCommand::Text` と `CueCommand::Emote` を記述したい。通常の会話記述が自然にキューシートに変換される。

#### 受入基準

1. When キューシートモードでアクション行 `actor：content` が記述された場合、the pasta DSL 拡張パーサー shall `content` 部分を `CueCommand::Text(content)` にマッピングする。
2. When アクション行で `@command` 記法（例: `さくら：こんにちわ@happy`）が使用された場合、the pasta DSL 拡張パーサー shall `@command` 部分を要件 4 のエイリアス解決ルールに従って処理し、エイリアス未定義の場合は `CueCommand::Emote { key: "happy" }` にフォールバックする。
3. The pasta DSL 拡張パーサー shall アクション行の `actor` 部分を ActorKey として解釈し、後続のルーティング自動生成（要件 6）の入力とする。
4. When キューシートモードで継続行（`:content` 形式）が記述された場合、the pasta DSL 拡張パーサー shall そのコンテンツを `\n` を区切りとして直前のアクション行の `CueCommand::Text` に結合し、同一 Cue として扱う。タイムライン上は前行の暗黙キーフレームに続くルールを適用する。
5. If キューシートモードで継続行に `@command` が含まれる場合、the pasta DSL 拡張パーサー shall パースエラーを報告する（継続行内 `@command` は不許可）。
6. When アクション行に複数の `@command` が含まれる場合（例: `さくら：＠笑顔　ふふーんいいでしょ。＠驚き　あ！`）、the pasta DSL 拡張パーサー shall それらを出現順に処理し、テキスト断片と CueCommand を交互に生成する。生成される Cue は同一 ActorKey で順次並ぶ。

> **設計注記**: 複数 `@command` を含むアクション行は、テキスト → Emote → テキスト → Emote の順に複数 Cue を生成する。各 Cue の `start_time` 計算は Duration Resolver の責務。

---

### 要件 6: Routing の自動生成と明示指定

**目的**: スクリプト作者として、ルーティング制御（`RoutingCommand`）をアクター配置（`%`行）から自動生成させつつ、`!route_add` / `!route_switch` コマンドで任意の EntityKey を明示指定できるようにしたい。伺か標準の「スロット = シェル・バルーン両方の宛先」という慣習と、より汎用的な「特定 Entity への個別ルーティング」の両方を記述できる。

#### 受入基準

1. The pasta DSL 拡張パーサー shall キューシートモードでアクション行 `actor：content` を検出した際、その `actor` のスロット割り当て状態を照合し、未登録であれば `RoutingCommand::RouteAdd` を自動生成する。
2. When 同一 ActorKey が未割り当て（スロット未登録）の状態で初出現した場合、the pasta DSL 拡張パーサー shall Shell・Balloon の両 CueTarget に対して `RoutingCommand::RouteAdd` を自動生成する（`%actor=slot` の伺かスロット慣習に対応）。
3. The pasta DSL 拡張パーサー shall `!route_add[target, entity_key]`（または `！ルート追加[target, entity_key]`）コマンドを認識し、指定した CueTarget・EntityKey で `RoutingCommand::RouteAdd` を明示生成する。これにより Shell と Balloon を個別の EntityKey（異なるスロット・Spot 等）に割り当てることができる。
4. The pasta DSL 拡張パーサー shall `!route_switch[target, entity_key]`（または `！ルート切替[target, entity_key]`）コマンドを認識し、指定した CueTarget の配送先 Entity を切り替える `RoutingCommand::RouteSwitch` を明示生成する。
5. When 並列演出（キーフレーム指定による同一基準時刻の複数アクター）が検出された場合、the pasta DSL 拡張パーサー shall 各アクターに対して未割り当てのものに `RouteAdd` を使用し、既存ルーティングを維持しつつ追加先を登録する。
6. The pasta DSL 拡張パーサー shall アクター配置（`%`行）の `%actor=slot_id` 記法を解析し、ActorKey → スロット番号のマッピング情報を保持する。これは Shell・Balloon 両 Target を同一スロットに一括割り当てするショートハンドである。
7. The pasta DSL 拡張パーサー shall スロット割り当てをセッションをまたいで永続させる（最終シーンの割り当てを継続する）。`%` 行が存在する場合はその指定を優先する。`%` 行がなく未割り当てのアクターが出現した場合は、現在未使用の最小スロット番号（0 番起算）を割り当てる。
8. The pasta DSL 拡張パーサー shall `RoutingCommand::RouteRemove` をスクリプト作者が明示的に `!route_remove[target]` と記述した場合のみ生成する。シーン終了時の自動 Remove は行わない（アプリ層の責務）。
9. The pasta DSL 拡張パーサー shall `!route_add` / `!route_switch` の `entity_key` 引数として以下の形式を受け入れる：`actor:<name>:<target>`（例: `actor:さくら:shell`）、`spot:<name>`、`balloon:<name>`。
10. The CueSheet 構築層 shall 現在のスロット割り当て状態を問い合わせる API（例: `get_slot_assignment(actor) -> Option<SlotId>`）を提供する。これにより RouteAdd 自動生成要否の判定が可能となる。

> **設計注記**: `%actor=slot` は Shell・Balloon を同一スロットに一括割り当てするショートハンド。`!route_add[shell, actor:さくら:shell]` + `!route_add[balloon, spot:stage_balloon]` のように個別指定すれば任意の EntityKey を割り当てられる。RouteSwitch は自動生成せず、明示 `!route_switch` コマンドによってのみ発行する。

---

### 要件 7: 後方互換性と既存文法との共存

**目的**: プロジェクト関係者として、既存の pasta スクリプトが変更なく動作し続けることを保証したい。拡張導入による既存コンテンツの破壊リスクをゼロにできる。

#### 受入基準

1. The pasta DSL 拡張パーサー shall `＆type：cuesheet` を持たないシーンを完全に現行 pasta DSL 仕様で処理し、キューシート専用構文（`!` キューコマンド行、エイリアス定義行）を解釈しない。
2. The pasta DSL 拡張パーサー shall 既存の属性行構文（`&key：value`）、アクター配置（`%`行）、アクション行（`actor：content`）の挙動を変更しない。
3. The pasta DSL 拡張パーサー shall `@command` 記法の既存の挙動（ランダムワード置換辞書による置換、未定義時はそのまま残す）をキューシートモード外で維持する。
4. The pasta DSL 拡張パーサー shall キューシートモード内でも、既存の pasta DSL 構文（シーン定義、ローカルシーン、変数参照、継続行など）が適切に機能することを保証する。

---

### 要件 8: エラーハンドリング

**目的**: スクリプト作者として、文法エラーの箇所が特定できるエラーメッセージを受け取りたい。キューシート記述の誤りを迅速に修正できる。

#### 受入基準

1. If キーフレーム指定行で未宣言のキーフレーム名が参照された場合、the pasta DSL 拡張パーサー shall 行番号・キーフレーム名を含むパースエラーを報告する。
2. If キーフレーム宣言行で重複したキーフレーム名が使用された場合、the pasta DSL 拡張パーサー shall 重複エラーを報告する。
3. If オフセット秒数に負数または不正なリテラルが指定された場合、the pasta DSL 拡張パーサー shall リテラル解析エラーを報告する。
4. If エイリアス定義行で不正な構文（例: 空のコマンド名、括弧の不一致）が記述された場合、the pasta DSL 拡張パーサー shall 構文検証エラーを報告する。
5. If アクター配置（`%`行）で不正なスロット番号（例: 負数、非整数）が指定された場合、the pasta DSL 拡張パーサー shall パースエラーを報告する。
6. If 継続行に `@command` が含まれる場合、the pasta DSL 拡張パーサー shall 継続行内 `@command` 不許可エラーを報告する。
7. The pasta DSL 拡張パーサー shall 全てのパースエラーに対して、行番号・カラム番号・エラー種別・修正ヒントを含むメッセージを生成する。

---

### 要件 9: 設計成果物要件

**目的**: プロジェクト関係者として、本仕様を元に pasta_dsl の実装者が文法拡張を実施できる状態の設計書と動作サンプルファイルを得たい。コード実装フェーズに確実に移行できる。

#### 受入基準

1. The pasta DSL 拡張仕様 shall 全機能（暗黙キーフレーム、`!` キューコマンド行、エイリアス定義、アクション行マッピング、Routing 自動生成）を網羅したサンプルシーンを含む `cue.pasta` ファイルを成果物として提供する。
2. The pasta DSL 拡張仕様 shall 並列演出（複数アクターの同一基準時刻 + オフセット）を示すシーンを `cue.pasta` に含める。
3. The pasta DSL 拡張仕様 shall `cue.pasta` 冒頭に、本ファイルが現行 pasta_core ではコンパイルできないことを明記した免責コメントを記載する。
4. The pasta DSL 拡張仕様 shall `design.md` に要件 1〜8 の全機能を実現するために必要な pasta_dsl 変更対象ファイル（`.pest` 文法ファイル・AST・パーサー・IR）と変更内容の指針を記載する。
5. The pasta DSL 拡張仕様 shall `design.md` にキューコマンド行（`!` 行）の具体的な文法（EBNF または PEG 記法）を明示する。
6. The pasta DSL 拡張仕様 shall `design.md` にエイリアス定義行の PEG 文法と CueCommand 記法（コマンド名の英語/日本語対応表、引数エンコード規則、フォールバック規則）を表形式で明示する。
7. The pasta DSL 拡張仕様 shall `design.md` に Duration Resolver トレイトの設計（インターフェース定義、CueSheet ビルダーへの注入方法）を記述する。
8. The pasta DSL 拡張仕様 shall `design.md` に実装フェーズ計画（段階的 MVP）を記載し、最小実装（MVP）から段階的に拡張できる構成とする。
9. The pasta DSL 拡張仕様 shall `design.md` に `get_slot_assignment()` API の仕様と RouteAdd/Remove 判定ロジックの設計を記載する。

---

## 注記

### Q1〜Q7 ディスカッション確定事項

| # | 議題 | 決定 |
|---|------|------|
| Q1 | 暗黙キーフレームの所要時間算出 | Duration Resolver トレイト（外部注入）。パーサーは順序・構造のみ出力 |
| Q2 | `@command` 未定義時の挙動 | `CueCommand::Emote { key }` にフォールバック（最頻出用途だから） |
| Q3 | 継続行の CueCommand::Text 挙動 | `\n` 結合で同一 Cue に追記。継続行内 `@command` は不許可 |
| Q4 | `%` 行不在時のスロット割り当て | セッション永続。未割り当てアクターのみ空き最小番号から割り当て |
| Q5 | `CueCommand::Clear` 生成ポリシー | `!clear` 明示のみ。自動生成はアプリ層の責務 |
| Q6 | `RouteRemove`・`RouteSwitch` 発行条件 | 明示 `!route_remove` / `!route_switch` のみ。自動生成はアプリ層の責務 |
| Q7 | 1行内複数 `@command` の処理 | 全て適用。出現順に Text/Emote を交互に生成 |

### 設計フェーズへの引き継ぎ事項（design.md で詳細化）

- `!` コマンド行の具体的な PEG/EBNF 文法（キーフレーム宣言・指定・Barrier・Clear・route_remove の各記法）
- エイリアス定義行の PEG 文法（`@alias = Command(args)` の `=` セパレータ形式）
- CueCommand 記法対応表（英語/日本語キーワード）
- Duration Resolver トレイトの型定義
- RouteAdd 自動生成・RouteSwitch 明示コマンドの判定ロジック詳細
- `get_slot_assignment()` API 仕様
- `!route_add` / `!route_switch` の EntityKey 引数 PEG 文法
- 並列演出検出アルゴリズム
- 実装アプローチ選択（A: pasta_dsl 完結 / B: ブリッジクレート / C: ハイブリッド）
- 実装 MVP フェーズ計画

### 将来拡張候補

- グローバルスコープのエイリアス定義
- `CueCommand::Custom` の詳細パラメータ記法
- キューコマンド行への新規コマンド追加（`timeout`, `wait` など）
- Storyboard 統合（CueSheet → Storyboard 起動、キーフレーム相互参照）
