# Requirements Document

| 項目               | 内容                                         |
| ------------------ | -------------------------------------------- |
| **Document Title** | wintf キューシステム（cue-system）要件定義書 |
| **Version**        | 2.0                                          |
| **Date**           | 2026-02-27                                   |
| **Priority**       | P0 (MVP必須)                                 |
| **Status**         | 📋 Generated - v2.0（DD9 絶対時刻方式適用） |

---

## Introduction

本仕様書は、演出指令（キュー）の構造化定義・配送・消費メカニズムを ECS 上に確立する汎用基盤仕様の要件を定義する。本仕様は実質的に「コンテンツ再生を指示するためのミニ言語」の設計であり、伺かプラットフォームの「さくらスクリプト」が果たしていた役割を、ECS アーキテクチャと Rust の型安全性で再構成するものである。

### 設計メタファー: 舞台演出のキューシート

> **演劇シーンを与えたら、演者が演じてくれる**

cue-system は**舞台演出のキューシート**をメタファーとし、**dola の思想**（宣言的構造 → コンパイル → 時刻ベース実行）を共有する。

| 概念                        | 説明                                                     | さくらスクリプトでの対応                       | dola での対応               |
| --------------------------- | -------------------------------------------------------- | ---------------------------------------------- | --------------------------- |
| **キューシート (CueSheet)** | 構造化された演出台本（相対時刻）。どの演者がいつ何をするかを記述する | さくらスクリプト文字列そのもの                 | Document/Storyboard         |
| **演者 (Actor)**            | キューシートの指示を受けて演技するエンティティ           | スコープ対象（`\0` = さくら、`\1` = うにゅう） | Variable の対象             |
| **キュー (Cue)**            | 個々の演出指示（CueSheet 内での相対時刻保持）                                           | 各タグ（`\s[0]`, `\w[500]`）や表示文字         | Transition/Duration         |
| **キューキュー (CueQueue)** | 各演者の実行可能な絶対時刻コマンド列                 | ベースウェアの内部バッファ（非公開）           | Runtime state               |
| **配送 (Dispatch)**         | 台本をコンパイル（絶対時刻化）して各演者に配る行為                                   | スコープ切替によるコマンドの暗黙的な振り分け   | compile + playback start    |

### さくらスクリプトからの設計継承と脱構築

さくらスクリプトの設計をギャップ分析レベルで評価し、継承すべき概念と再設計すべき概念を分離する。詳細な調査レポートは [research.md](./research.md) を参照。

#### 継承する概念（オマージュ）

| 概念                   | さくらスクリプトでの表現                    | cue-system での再構成                           |
| ---------------------- | ------------------------------------------- | ----------------------------------------------- |
| **単一台本モデル**     | 1つのスクリプトが全キャラクターの演技を記述 | CueSheet が複数演者への指示を包含               |
| **直感的なタイミング** | `\w[N]`, `\x`, `\_q` の3種                  | start_time 差分（間合い）、WaitForInput（対話待ち）、同一 start_time（即時一括） |
| **掛け合い**           | `\0`, `\1` でキャラクター間を行き来         | CueSheet 内でアクターを明示指定、配送時にキュー分離 |
| **演出のメタファー**   | スクリプト = 舞台の脚本                     | CueSheet = 舞台のキューシート                   |
| **逐次投入の自然さ**   | テキストとタグが混在し読みやすい            | pasta DSL 層がこの書きやすさを維持              |

#### 脱構築する概念（再設計）

| さくらスクリプトの課題   | 影響                                       | cue-system での解決方針                      |
| ------------------------ | ------------------------------------------ | -------------------------------------------- |
| テキストストリーム形式   | パースエラーが実行時まで検出不可           | 型付き enum コマンド（コンパイル時検証）     |
| 非統一タグ構文           | `\s[N]`, `\w5`, `\![cmd,p]` が混在         | 統一された CueCommand enum 体系              |
| 逐次専用（並列不可）     | テキスト表示中にSE再生を同時に行えない     | エンティティごとの独立キュー（本質的に並列） |
| グローバルスコープ状態   | 長いスクリプトで現在のスコープが不明瞭     | 各コマンドが対象演者を明示的に保持           |
| コンテンツと制御の密結合 | テキストだけ抽出、タイミングだけ変更が困難 | 構造化コマンドでフィルタリング・変換可能     |
| 固定タグ形式             | ベースウェア間の互換性問題                 | 消費者ごとの型安全な拡張メカニズム           |

### スコープ

**含まれるもの:**
- CueSheet（構造化演出台本）のデータモデル定義
- CueCommand（演出指令）の基盤コマンド型定義
- CueQueue（エンティティキュー）コンポーネント設計
- CueSheet → CueQueue 配送メカニズム
- キュー消費プロトコル（消費者システム向けインターフェース）
- タイミング制御セマンティクス（start_time 絶対時刻指定、WaitForInput、Skip）
- dola との統合インターフェース（タイミングオーケストレーション）
- 消費者固有コマンド型の拡張メカニズム

**含まれないもの:**
- バルーンのテキスト描画実装（balloon03-content の責務）
- キャラクターアニメーション実装（animation-system の責務）
- dola ランタイム自体の実装（dola クレートの責務）
- pasta DSL のパーサー・コンパイラ（外部リポジトリの責務）
- 具体的な描画・音声再生の実装（各消費者仕様の責務）
- フレーム外観定義（balloon01-core の BalloonSkinDef の責務）

### 既存資産との関係

| 既存資産                       | 関係性                                                                                     |
| ------------------------------ | ------------------------------------------------------------------------------------------ |
| `TypewriterToken` (Stage 1 IR) | cue-system の基盤コマンド型の先行実装。Text, Wait, FireEvent の3バリアント                 |
| `TypewriterTalk`               | CueQueue の特殊化（丸ごと差し替え方式）。cue-system はこれを append 可能な汎用キューに拡張 |
| `DolaRuntime`                  | タイミングオーケストレーションの外部エンジン。`subscribe`/`update` によるタイムライン連携  |
| `Messages<T>` (Drag系)         | 参考にした ECS パターン。cue-system はコンポーネントベースで実装                           |
| `CommandSender` (mpsc)         | 非同期→ECS の通信経路。CueSheet の非同期受け渡しに活用可能                                 |
| pasta DSL                      | CueSheet の主要な生成者。里々がさくらスクリプトを生成する関係に相当                        |

### 消費者（本仕様の利用者となる仕様群）

| 消費者            | コマンド例                                          | 位置づけ                 |
| ----------------- | --------------------------------------------------- | ------------------------ |
| balloon03-content | テキスト表示、Wait、スタイル変更、感情値切替、Clear | バルーンのコンテンツ再生 |
| animation-system  | サーフェス切替、トランジション、ポーズ変更          | キャラクター演技         |
| 将来の演出要素    | SE再生、画面効果、ウィンドウ演出                    | 拡張演出                 |

### 依存関係

- **参照**: `wintf-P0-typewriter` ✅（TypewriterToken IR パターンの先行実装）
- **参照**: `wintf-P0-event-system` ✅（Messages\<T\> パターン）
- **ブロッカー先**: `wintf-P0-balloon01-core`（本仕様の完了がバルーン v3.0 の前提）
- **後続消費者**: `wintf-P0-animation-system`, `wintf-P0-balloon03-content`
- **外部**: pasta DSL（CueSheet の主要な生成者）

### 設計制約

- TypewriterToken の Stage 1 IR パターンとの後方互換性を考慮する
- bevy_ecs 0.18.0 のコンポーネントシステム制約に従う
- `on_add` フック、`Changed<T>`、`ChildOf` パターンを活用する
- `#[cfg(feature = "dola")]` でのフィーチャーフラグによる条件コンパイルを維持
- 伺かのさくらスクリプトとの互換性は**非目標**（設計のオマージュであり、パースの互換ではない）

### 先行議論コンテキスト（balloon01-core v2.0 レビューより引継ぎ）

<details>
<summary>検討済みの配送方式（4方式比較）</summary>

| 方式                       | 仕組み                                          | 評価                                   |
| -------------------------- | ----------------------------------------------- | -------------------------------------- |
| A: コンポーネント差し替え  | `XxxTalk::new(commands)` で丸ごと insert        | Typewriter踏襲。「追加」が不自然       |
| B: 子エンティティ追加      | `ContentCommand` entity を `ChildOf` で spawn   | ECS的に自然。順序保証の設計が必要      |
| C: Messages\<T\>           | bevy_ecs メッセージキュー経由                   | 既存パターン(Drag系)。送り先特定が課題 |
| D: VecDeque コンポーネント | ルートに `CueQueue(VecDeque)` を付与、直接 push | 明白。Changed 検出が Mut\<T\> 必須     |

</details>

<details>
<summary>確定済み設計判断 D1-D5</summary>

- **D1: ファサードパターン原則** — 外部入力はすべてルートエンティティのコンポーネントで受け取る
- **D2: TextDirection は静的属性** — 変更時はコンテンツ全削除
- **D3: 感情値ベース BalloonStyleMap** — KV形式 + デフォルト値
- **D4: dola は内部サブシステムとしても利用可能** — タイミング計算、グリフ表示制御
- **D5: コンテンツコマンド配送は cue-system に外部化** — 横断的関心事として分離

</details>

---

## Requirements

### Requirement 1: CueSheet — 構造化演出台本モデル（絶対時刻キーフレーム方式）

**Objective:** 開発者として、複数の演者への演出指示を含む構造化された台本（CueSheet）を定義したい。それにより pasta DSL や外部システムが型安全に演出シーンを記述でき、cue-system がそれを各演者に配送できる基盤を確立できる。

**さくらスクリプト継承**: 単一台本モデル（1つのスクリプトが全キャラクターの演技を記述する概念）を型安全な構造に再構成

**設計方針 (DD9)**: 絶対時刻キーフレーム方式を採用。各 Cue は CueSheet 開始からの絶対秒数 `start_time` を保持し、並行実行は同一 `start_time` の指定で表現する。投入順≠実行順であり、タイミング計算は CueSheet 生成者（pasta DSL 等）のコンパイル時に行う。

#### Acceptance Criteria

1. **The** Cue System **shall** 演出台本を表現する CueSheet データ構造を提供する（pure Vec<Cue>、メタデータフィールドなし）
2. **The** Cue System **shall** CueSheet 内の各指示（Cue）が対象演者の識別子（ActorKey）を明示的に保持する設計とする
3. **The** Cue System **shall** 各 Cue に CueSheet ローカル時刻（start_time: f64、秒単位、CueSheet 開始時点からの相対秒数）を保持させる
4. **The** Cue System **shall** CueSheet 内の Cue を start_time の昇順で保持する（同一時刻のコマンドは挿入順で安定ソート）
5. **The** Cue System **shall** 1つの CueSheet 内に複数の演者への指示を混在して記述できる
6. **The** Cue System **shall** 同一 start_time に複数の Cue を配置することで並行実行を表現できる
7. **The** Cue System **shall** CueSheet から特定演者の指示のみをフィルタリング抽出する API を提供する
8. **The** Cue System **shall** CueSheet を Clone, Debug derive 可能にする（ログ出力・複製対応）

---

### Requirement 2: CueCommand — 型安全な基盤コマンド体系（絶対時刻方式対応）

**Objective:** 開発者として、演出指示を型安全な enum として定義したい。それにより文字列パースに依存せず、コンパイル時に不正なコマンドを検出でき、IDE 補完の恩恵を受けられる。

**さくらスクリプト脱構築**: テキストストリーム形式・非統一タグ構文（`\s[N]`, `\w5`, `\![cmd,p]`）を、統一された型付き enum に置換

**DD9 による変更点**:
- ~~時間ウェイトバリアント（Wait）~~ → 不要。タイミング間隔は Cue の start_time 差分で表現される
- ~~即時モード切替バリアント（Instant）~~ → 不要。同一 start_time の指定で代替される
- ~~WaitForInput~~ → `WaitForClick` / `WaitForChoice` / `Choice` の3バリアントに再設計（選択肢データとバリアを分離）

**バリアント一覧（確定・8バリアント）**:

```rust
enum CueCommand {
    Text(String),                                      // テキスト表示（意味解釈は消費者の責務）
    Clear,                                             // コンテンツクリア
    Emote { key: String },                             // 演技発現（キーの意味解釈は消費者の責務）
    Choice { id: String, text: String },               // 選択肢データ（先積み、WaitForChoice の前に連続投入）
    WaitForChoice { timeout: Option<f64> },            // 選択肢バリア（直前の Choice 群を提示してブロック）
    WaitForClick  { timeout: Option<f64> },            // クリック待ちバリア
    EntityRef(bevy_ecs::entity::Entity),               // ECS エンティティ渡し（消費者が解釈）
    Custom { command: String, params: DynamicValue },  // 消費者固有コマンド（dola::DynamicValue）
}
```

**Choice + WaitForChoice プロトコル**:
- `Choice` は `WaitForChoice` の**前に**任意個数キューに積む
- `WaitForChoice` が pop された時点で先行する `Choice` 群を選択肢として提示しブロック開始
- `WaitForChoice` 消費時に先行 `Choice` が 0 件 → プロトコル違反として `CueSheetResult::Error` を即時発行

#### Acceptance Criteria

1. **The** Cue System **shall** 基盤コマンド型を enum として定義する
2. **The** Cue System **shall** 基盤コマンドにテキスト表示バリアント `Text(String)` を含める（意味解釈は消費者の責務）
3. **The** Cue System **shall** 基盤コマンドにコンテンツクリアバリアント `Clear` を含める
4. **The** Cue System **shall** 基盤コマンドに演技発現バリアント `Emote { key: String }` を含める（演技キーの意味解釈は消費者が担う）
5. **The** Cue System **shall** 基盤コマンドに選択肢先積みバリアント `Choice { id: String, text: String }` を含める
6. **The** Cue System **shall** 基盤コマンドに選択肢バリア `WaitForChoice { timeout: Option<f64> }` を含める
7. **The** Cue System **shall** 基盤コマンドにクリック待ちバリア `WaitForClick { timeout: Option<f64> }` を含める
8. **The** Cue System **shall** 基盤コマンドに ECS エンティティ渡しバリアント `EntityRef(Entity)` を含める（消費者が解釈）
9. **The** Cue System **shall** 基盤コマンドに消費者固有コマンドバリアント `Custom { command: String, params: DynamicValue }` を含める（`DynamicValue` は `dola::DynamicValue` を使用）
10. **The** Cue System **shall** 各コマンドバリアントのパラメータに適切な Rust 型を付与する（文字列パラメータへの依存を最小化）
11. **The** Cue System **shall** 基盤コマンド型に Clone, Debug を derive する

---

### Requirement 3: CueQueue — エンティティキューコンポーネント

**Objective:** 開発者として、各演者エンティティに時刻付き演出指示のキューを持たせたい。それにより外部から任意のタイミングでコマンドを追加（append）でき、消費者システムが時刻到達順にコマンドを消費できるキューを確立する。

**さくらスクリプト脱構築**: グローバルスコープ状態 → エンティティごとの独立キュー。TypewriterTalk の丸ごと差し替え → append 可能なキュー。

**DD9 による変更点**: CueQueue の各エントリは `(start_time, CueCommand)` ペア（TimedCue）として保持される。start_time 昇順で並び、消費者は現在時刻が start_time に到達したコマンドを消費する。

#### Acceptance Criteria

1. **The** Cue System **shall** CueQueue を ECS コンポーネントとして提供する
2. **The** Cue System **shall** CueQueue の各エントリを時刻付きコマンド（start_time + CueCommand）として保持する
3. **The** Cue System **shall** CueQueue 内のエントリを start_time の昇順で維持する
4. **The** Cue System **shall** CueQueue にコマンドを start_time 順序を維持して追加する API を提供する
5. **The** Cue System **shall** CueQueue から時刻到達済みの先頭コマンドを取得・除去する API を提供する
6. **The** Cue System **shall** CueQueue の先頭要素を除去せずに参照（peek）する API を提供する
7. **The** Cue System **shall** CueQueue の空判定（is_empty）および件数取得（len）API を提供する
8. **The** Cue System **shall** CueQueue のコンテンツ全消去（clear）API を提供する
9. **The** Cue System **shall** 同一 World 内に複数の CueQueue を独立して存在させられる（エンティティごとに1つ）

---

### Requirement 4: CueSheet 配送 — コンパイルと演者への分配

**Objective:** 開発者として、作成した CueSheet を各演者エンティティに配送したい。それにより CueSheet の相対時刻が世界時刻に変換（コンパイル）され、演者ごとの独立した CueQueue に絶対時刻コマンドとして振り分けられ、個別の消費タイミング制御が可能になる。

**さくらスクリプト脱構築**: スコープ切替（`\0`, `\1`）による暗黙の振り分け → 明示的な ActorKey + 自動分配

**dola 思想の継承**: CueSheet（宣言的、相対時刻）→ コンパイル（絶対時刻化）→ CueQueue（実行可能形式）という変換パイプライン

#### Acceptance Criteria

1. **The** Cue System **shall** CueSheet 配送時に sheet_start_time（世界絶対時刻）を受け取り、各 Cue のローカル時刻を世界絶対時刻に変換する（`world_time = sheet_start_time + cue.start_time`）
2. **The** Cue System **shall** 絶対時刻化された各 Cue を、ActorKey に対応する演者エンティティの CueQueue に分配する配送メカニズムを提供する
3. **The** Cue System **shall** ActorKey から対象エンティティを解決する仕組みを提供する
4. **The** Cue System **shall** 配送時に CueQueue 内の絶対時刻昇順を維持する（既存コマンドとのマージ挿入）
5. **When** CueSheet がシステムに投入された時, **the** Cue System **shall** 対象となる全演者の CueQueue にコマンドを配送する
6. **If** CueSheet 内の ActorKey に対応するエンティティが見つからない場合, **the** Cue System **shall** `tracing::warn!` でログ出力し、該当コマンドをスキップする（他の演者への配送は継続）
7. **The** Cue System **shall** 既に CueQueue にコマンドが存在するエンティティに対しても、追加の CueSheet 配送による絶対時刻順マージ追加を行える（任意タイミングでの逐次投入）

---

### Requirement 5: CueQueue 消費プロトコル — 時刻ベース実行制御

**Objective:** 開発者として、CueQueue のコマンドを時刻到達順に消費したい。それにより絶対時刻方式（DD9）に基づいた正確なタイミング制御が可能になり、並行実行やタイムライン操作を自然に実現できる。

**さくらスクリプト脱構築**: FIFO 先頭消費（一本道） → 時刻到達ベース消費（並行可能）

**dola 思想の継承**: playback(current_time) パターン — 外部から時刻を受け取り、到達済みコマンドを返す

#### Acceptance Criteria

1. **The** Cue System **shall** 消費者システムがフレームごとに CueQueue 内の時刻到達済みコマンド（current_time ≥ start_time）を消費できるプロトコルを定義する
2. **The** Cue System **shall** CueQueue は経過時刻を管理せず、消費時に外部から current_time（世界絶対時刻）を受け取る設計とする
3. **The** Cue System **shall** 同一 start_time のコマンドをフレーム内で一括消費（並行消費）できるパターンを定義する
4. **When** CueQueue 内にユーザー入力待ちコマンドが時刻到達した場合, **the** Cue System **shall** 外部入力を受信するまで当該演者の CueQueue タイムライン進行をブロックするセマンティクスを定義する（演者ごとブロック。他の演者のタイムラインは独立して進行）
5. **The** Cue System **shall** キュー消費の現在状態（再生中・入力待ち・完了）を追跡する消費ステート管理を提供する
6. **When** CueQueue の全コマンドが消費された時, **the** Cue System **shall** 消費完了状態を示す

---

### Requirement 6: タイミング制御と dola 統合（思想の共有）

**Objective:** 開発者として、dola アニメーションシステムと思想を共有したタイミング制御を行いたい。それにより、テキスト表示とアニメーションが同一の時間軸で制御され、コンテンツの同期が保証される。cue-system は dola の「宣言的構造 → コンパイル → 時刻ベース実行」というパイプラインを対話的台本の領域で実現する。

**dola との思想共有**:
- CueSheet ≈ dola::Document/Storyboard（宣言的、相対時刻）
- dispatch(sheet_start_time) ≈ dola::compile（絶対時刻化）
- CueQueue ≈ dola::Runtime（実行可能形式）
- pop_ready(current_time) ≈ dola::playback（時刻ベース消費）

#### Acceptance Criteria

1. **The** Cue System **shall** playback_rate（再生速度倍率）を適用可能な設計とする
2. **When** dola feature が有効な時, **the** Cue System **shall** dola の `DolaBridgeResource` 経由でタイムライン制御を統合する
3. **The** Cue System **shall** CueQueue の時刻進行を dola の時刻精度（f64 秒）と互換にする
4. **The** Cue System **shall** dola feature が無効な時でも独立して動作できる設計とする（`#[cfg(feature = "dola")]` による条件コンパイル）

---

### Requirement 7: コマンド型安全拡張メカニズム

**Objective:** 開発者として、消費者ごとに固有のコマンド型を型安全に定義したい。それにより balloon 向けのテキスト系コマンドと animation 向けのサーフェス系コマンドを、共通の配送・消費基盤上で安全に扱える。

**さくらスクリプト脱構築**: `\!` 拡張メカニズム（文字列パラメータ、ベースウェア固有）→ `Custom { command: String, params: DynamicValue }` バリアントによる型安全な拡張

**DD12 による確定**: `Custom` バリアントは `dola::DynamicValue`（JSON 互換辞書型）をパラメータとして採用。`Box<dyn Trait>` は `CueCommand: Clone` と相性が悪いため不採用。消費者はコマンド名（String）でパターンマッチして処理する。

#### Acceptance Criteria

1. **The** Cue System **shall** `Custom { command: String, params: DynamicValue }` バリアントを通じて消費者固有コマンドを CueQueue に格納できる仕組みを提供する
2. **The** Cue System **shall** 消費者が `Custom` バリアントのコマンド名で分岐し、自ドメイン以外のコマンドを安全にスキップまたは通過させるパターンを提供する
3. **The** Cue System **shall** `Custom` バリアントが `Clone + Debug` を満たすことを保証する（`DynamicValue: Clone + Debug` による）
4. **The** Cue System **shall** `Custom` バリアントの `params` に `DynamicValue::Null` を使用することで引数なしコマンドを表現できる設計とする
5. **The** Cue System **shall** 消費者固有コマンドの使用例（バルーン向け・アニメーション向け）をドキュメントとして提供する

---

### Requirement 8: エラーハンドリングと堅牢性

**Objective:** 開発者として、不正なコマンドやエッジケースに対してシステムが安定して動作し、予測可能な挙動を示すようにしたい。それにより開発・デバッグ時の問題特定が容易になる。

#### Acceptance Criteria

1. **If** CueQueue に対してキャパシティ上限が設定されており上限を超えてコマンドが追加された場合, **the** Cue System **shall** `tracing::warn!` でログ出力し、超過分の処理方針を呼び出し元に通知する
2. **If** 消費者システムが CueQueue 内のコマンド型を認識しない場合, **the** Cue System **shall** 当該コマンドを安全にスキップし `tracing::debug!` でログ出力する
3. **The** Cue System **shall** CueQueue を保持するエンティティが despawn されても panic しない
4. **The** Cue System **shall** 空の CueSheet が配送された場合にエラーを発生させず無操作で完了する
5. **If** ActorKey の解決に失敗した場合, **the** Cue System **shall** 他の演者への正常な配送を継続する（部分的失敗の許容）
6. **If** CueQueue 内のコマンドの start_time が現在の経過時刻より過去である場合, **the** Cue System **shall** 当該コマンドを遅延到達として即時消費する（タイムラインの追いつき処理）

---

### Requirement 9: CueSheet ライフサイクルと実行結果 — フィーチャーモデル

**Objective:** 開発者として、CueSheet を「開始して決定論的に実行し、結果を返す」フィーチャーとして扱いたい。それにより上位のオーケストレーション層が CueSheet の実行結果を await し、選択肢の選択・完了・キャンセル・タイムアウトに応じた次の処理へ分岐できる。

**設計メタファー**: OS の Modal Dialog — 開始すると決定論的に動作し、終了時に DialogResult を返す。1 CueSheet = 1 フィーチャー実行単位。

**T7/T8 統合**:
- T7（中断・キャンセル）→ `CueSheetResult::Cancelled` として本 Requirement に統合
- T8（動的生成）→ スコープ外。CueQueue の追記型設計により逐次投入は自然に実現（上位層/pasta の責務）

#### Acceptance Criteria

1. **The** Cue System **shall** CueSheet の実行結果を表す `CueSheetResult` 型を提供する（バリアント: `Completed` / `Cancelled` / `Timeout` / `Choice { id: String }` / `Error(CueSystemError)`。`CueSystemError` は `thiserror` で定義される構造化エラー型）
2. **When** CueSheet 内の全演者の CueQueue が消費完了した時, **the** Cue System **shall** `CueSheetResult::Completed` を通知する
3. **When** CueSheet が外部から中断・キャンセルされた時, **the** Cue System **shall** `CueSheetResult::Cancelled` を通知する
4. **When** WaitForInput に timeout が設定されており期限を超過した時, **the** Cue System **shall** `CueSheetResult::Timeout` を通知する
5. **When** 選択肢コマンドがユーザーによって選択された時, **the** Cue System **shall** `CueSheetResult::Choice { id }` を通知する
6. **The** Cue System **shall** `CueSheetResult` を上位層（オーケストレーション）が Rust 的な await パターンで受け取れる形式で提供する（ECS 的な具体的実装は DD11 として設計フェーズで決定）
7. **When** `WaitForChoice` コマンドが消費された時点で対象演者キューに先行する `Choice` コマンドが 0 件だった場合, **the** Cue System **shall** `CueSheetResult::Error(CueSystemError::EmptyChoiceBarrier { actor })` を発行し CueSheet を即時終了する

---

## Non-Functional Requirements

### NFR-1: パフォーマンス

1. **The** Cue System **shall** CueQueue のコマンド追加・時刻到達消費操作を効率的に実行する（O(log n) 許容。実用上のキュー長は数十〜数百を想定）
2. **While** CueQueue が空の状態, **the** Cue System **shall** 消費者システムの不要な走査を最小化する
3. **The** Cue System **shall** コマンド型のメモリサイズをキャッシュフレンドリーな範囲に抑える
4. **The** Cue System **shall** TimedCue（start_time + CueCommand）の合計メモリサイズを 64 バイト以下に維持する

### NFR-2: デバッグ容易性

1. **The** Cue System **shall** 全てのコマンド型に Debug derive を付与する
2. **The** Cue System **shall** CueSheet の配送ログを `tracing::debug!` で出力する
3. **The** Cue System **shall** CueQueue の消費ログを `tracing::trace!` で出力する（高頻度のため trace レベル）
4. **The** Cue System **shall** CueQueue の経過時刻と次回消費予定時刻をログ出力できる

### NFR-3: ECS 親和性

1. **The** Cue System **shall** bevy_ecs 0.18.0 のコンポーネントシステムに準拠する
2. **The** Cue System **shall** 既存の wintf レイヤー構造（COM → ECS → Message Handling）の依存方向に違反しない
3. **The** Cue System **shall** CueQueue を SparseSet ストレージで管理する（動的変更が頻繁なため）

---

## Version History

| Version | Date       | Changes                                                                         |
| ------- | ---------- | ------------------------------------------------------------------------------- |
| 1.0     | 2026-02-26 | 初版生成（8要件 + 3NFR）                                                        |
| 1.0.1   | 2026-02-26 | レビュー自明修正（F1: C3但し書き, F2: Req4 AC2実装詳細削除）                    |
| 2.0     | 2026-02-27 | DD9 絶対時刻キーフレーム方式適用。Req 1,2,3,4,5,6 を全面書き換え。議題T5-T8追加 |
| 2.1     | 2026-02-27 | Q5-Q8 議論完了。performer→actor 用語統一。CueSheet 相対時刻・CueQueue 外部 current_time 受取確定。dola 思想統一。Req 9 追加（フィーチャーモデル・CueSheetResult）。T2/T5/T6/T7/T8 全議題削除 |
| 2.2     | 2026-02-27 | CueCommand 8バリアント確定。WaitForInput → WaitForClick/WaitForChoice/Choice に再設計（データとバリアの分離）。EntityRef/Custom(DynamicValue) 追加。CueSheetResult::Error 追加。Req 7 を DynamicValue 方針に更新 |

---

## Dependencies

### 参照仕様

| 仕様                      | 参照内容                                                           |
| ------------------------- | ------------------------------------------------------------------ |
| `wintf-P0-typewriter` ✅   | TypewriterToken（Stage 1 IR）パターン、TypewriterTalk の消費モデル |
| `wintf-P0-event-system` ✅ | Messages\<T\> パターン、CommandSender (mpsc) パターン              |

### 後続仕様（cue-system を利用する仕様）

| 仕様                         | 利用内容                                             |
| ---------------------------- | ---------------------------------------------------- |
| `wintf-P0-balloon01-core`    | ブロッカー先。バルーンのコンポーネント構成原則に反映 |
| `wintf-P0-balloon03-content` | テキスト系 CueCommand の消費者実装                   |
| `wintf-P0-animation-system`  | サーフェス系拡張コマンドの消費者実装                 |
| `areka-P0-reference-ghost`   | pasta DSL → CueSheet の生成者                        |

### 外部依存

| 外部                        | 依存内容                                                                    |
| --------------------------- | --------------------------------------------------------------------------- |
| pasta DSL（外部リポジトリ） | CueSheet の主要な生成者。pasta → CueSheet 変換はアプリケーション層の責務    |
| dola クレート               | タイミングオーケストレーション。`#[cfg(feature = "dola")]` で条件コンパイル |
