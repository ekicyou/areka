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

cue-system は**舞台演出のキューシート**をメタファーとする。

| 概念                        | 説明                                                     | さくらスクリプトでの対応                       |
| --------------------------- | -------------------------------------------------------- | ---------------------------------------------- |
| **キューシート (CueSheet)** | 構造化された演出台本。どの演者がいつ何をするかを記述する | さくらスクリプト文字列そのもの                 |
| **演者 (Actor)**            | キューシートの指示を受けて演技するエンティティ           | スコープ対象（`\0` = さくら、`\1` = うにゅう） |
| **キュー (Cue)**            | 個々の演出指示                                           | 各タグ（`\s[0]`, `\w[500]`）や表示文字         |
| **キューキュー (CueQueue)** | 各演者の手持ちの、次の演技指示を待つ行列                 | ベースウェアの内部バッファ（非公開）           |
| **配送 (Dispatch)**         | 台本を各演者に配る行為                                   | スコープ切替によるコマンドの暗黙的な振り分け   |

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
3. **The** Cue System **shall** 各 Cue に CueSheet 開始時点からの絶対時刻（start_time: f64、秒単位）を保持させる
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
- ユーザー入力待ち（WaitForInput）は残存。タイムラインを対話的にブロックする意味的に異なるコマンド

#### Acceptance Criteria

1. **The** Cue System **shall** 基盤コマンド型を enum として定義する
2. **The** Cue System **shall** 基盤コマンドにテキスト表示バリアント（表示対象文字列を保持）を含める
3. **The** Cue System **shall** 基盤コマンドにユーザー入力待ちバリアント（タイムアウトを任意で保持）を含める
4. **The** Cue System **shall** 基盤コマンドにコンテンツクリアバリアントを含める
5. **The** Cue System **shall** 基盤コマンドに演技発現バリアント（演技キーを保持。Emote { key: String }）を含める
6. **The** Cue System **shall** 基盤コマンドに消費者固有コマンドを格納するための拡張バリアントを含める
7. **The** Cue System **shall** 各コマンドバリアントのパラメータに適切な Rust 型を付与する（文字列パラメータへの依存を最小化）
8. **The** Cue System **shall** 基盤コマンド型に Clone, Debug を derive する

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

### Requirement 4: CueSheet 配送メカニズム（絶対時刻保持配送）

**Objective:** 開発者として、CueSheet を投入するだけで各演者の CueQueue にコマンドが自動配送されるようにしたい。それにより外部システム（pasta DSL 等）は CueSheet の構築に専念でき、配送の詳細を意識する必要がない。

**さくらスクリプト継承・脱構築**: スコープ切替（`\0`, `\1`）による暗黙の振り分け → 明示的な ActorKey + 自動分配

**DD9 による変更点**: 配送時に各 Cue の start_time を保持したまま各演者の CueQueue に分配する。CueQueue への挿入は start_time 順序を維持し、既存エントリとマージする形で追加される。

#### Acceptance Criteria

1. **The** Cue System **shall** CueSheet 内の各 Cue を、ActorKey に対応する演者エンティティの CueQueue に分配する配送メカニズムを提供する
2. **The** Cue System **shall** ActorKey から対象エンティティを解決する仕組みを提供する
3. **The** Cue System **shall** 配送時に各 Cue の start_time を保持したまま CueQueue に挿入する
4. **The** Cue System **shall** 配送時に CueQueue 内の start_time 昇順を維持する（既存コマンドとのマージ挿入）
5. **When** CueSheet がシステムに投入された時, **the** Cue System **shall** 対象となる全演者の CueQueue にコマンドを配送する
6. **If** CueSheet 内の ActorKey に対応するエンティティが見つからない場合, **the** Cue System **shall** `tracing::warn!` でログ出力し、該当コマンドをスキップする（他の演者への配送は継続）
7. **The** Cue System **shall** 既に CueQueue にコマンドが存在するエンティティに対しても、追加の CueSheet 配送による start_time 順マージ追加を行える（任意タイミングでの逐次投入）

---

### Requirement 5: キュー消費プロトコル（時刻到達消費モデル）

**Objective:** 開発者として、消費者システム（Typewriter、AnimationSystem 等）が CueQueue からコマンドを消費するための統一的なプロトコルを利用したい。それにより各消費者が一貫した方法でキューを処理でき、タイミング制御の共通セマンティクスを保証できる。

**さくらスクリプト脱構築**: ベースウェア内部の逐次ブロッキング解釈 → ECS システムによるフレーム単位の時刻到達消費

**DD9 による変更点**:
- FIFO 先頭消費 → 現在時刻 ≥ start_time のコマンドを時系列順に消費
- ~~即時モード消費~~ → 不要（同一 start_time 指定で代替済み）
- バッチ消費 → 同一 start_time のコマンドをフレーム内で並行消費
- WaitForInput は現在時刻のタイムライン進行を対話的にブロックする

#### Acceptance Criteria

1. **The** Cue System **shall** 消費者システムがフレームごとに CueQueue 内の時刻到達済みコマンド（現在時刻 ≥ start_time）を消費できるプロトコルを定義する
2. **The** Cue System **shall** 同一 start_time のコマンドをフレーム内で一括消費（並行消費）できるパターンを定義する
3. **When** CueQueue 内にユーザー入力待ちコマンドが時刻到達した場合, **the** Cue System **shall** 外部入力を受信するまで当該演者の CueQueue タイムライン進行をブロックするセマンティクスを定義する（演者ごとブロック。他の演者のタイムラインは独立して進行）
4. **The** Cue System **shall** CueQueue の経過時刻（CueSheet 開始からの相対秒数）を管理する仕組みを提供する
5. **The** Cue System **shall** キュー消費の現在状態（再生中・入力待ち・完了）を追跡する消費ステート管理を提供する
6. **When** CueQueue の全コマンドが消費された時, **the** Cue System **shall** 消費完了状態を示す

---

### Requirement 6: タイミング制御と dola 統合（統一時間軸モデル）

**Objective:** 開発者として、CueSheet の経過時刻管理と dola オーケストレーションを統一的な時間軸で利用したい。それにより CueSheet のタイムラインと dola アニメーションが同期した演出が可能となる。

**さくらスクリプト継承**: `\w[N]`（間合い）→ start_time 差分で表現、`\x`（クリック待ち）→ WaitForInput、`\_q`/`\t`（即時表示）→ 同一 start_time

**DD9 による変更点**:
- 消費速度変更 = CueQueue 経過時刻の進行速度倍率として明確化
- dola 統合 = CueSheet の start_time と dola の時間軸を統一可能

#### Acceptance Criteria

1. **The** Cue System **shall** CueQueue の経過時刻をシステム時間（FrameTime）ベースで進行させる
2. **The** Cue System **shall** CueQueue のタイムライン進行を一時停止（pause）する API を提供する
3. **The** Cue System **shall** CueQueue のタイムライン進行を再開（resume）する API を提供する
4. **The** Cue System **shall** CueQueue の残コマンドを即時完了（skip）する API を提供する（経過時刻を末尾コマンドの start_time まで即座に進める）
5. **The** Cue System **shall** CueQueue の経過時刻進行速度を変更する倍率（playback_rate）を提供する（倍速/低速対応。速度 = start_time 進行に対する倍率）
6. **Where** dola フィーチャーが有効な場合, **the** Cue System **shall** CueQueue の経過時刻と DolaRuntime の時間軸を統一し、連携させるインターフェースを提供する
7. **Where** dola フィーチャーが有効な場合, **the** Cue System **shall** dola の subscribe メカニズムを通じて CueQueue の消費進行状況を dola 変数として公開できる仕組みを提供する

---

### Requirement 7: コマンド型安全拡張メカニズム

**Objective:** 開発者として、消費者ごとに固有のコマンド型を型安全に定義したい。それにより balloon 向けのテキスト系コマンドと animation 向けのサーフェス系コマンドを、共通の配送・消費基盤上で安全に扱える。

**さくらスクリプト脱構築**: `\!` 拡張メカニズム（文字列パラメータ、ベースウェア固有）→ Rust enum ベースの型安全な拡張

#### Acceptance Criteria

1. **The** Cue System **shall** 基盤コマンド型の拡張バリアントを通じて消費者固有コマンドを CueQueue に格納できる仕組みを提供する
2. **The** Cue System **shall** 消費者が自ドメインの拡張コマンドを取り出し、それ以外のコマンドを安全にスキップまたは通過させるパターンを提供する
3. **The** Cue System **shall** 拡張コマンドの型が Debug トレイトを実装することを要求する（構造化ログ対応）
4. **The** Cue System **shall** 型拡張において `Any` ベースのダウンキャストよりも enum ベースの static dispatch を推奨する設計とする
5. **The** Cue System **shall** 消費者固有コマンド型の定義例（バルーン向け・アニメーション向け）をドキュメントとして提供する

---

### Requirement 8: エラーハンドリングと堅牢性

**Objective:** 開発者として、不正なコマンドやエッジケースに対してシステムが安定して動作し、予測可能な挙動を示すようにしたい。それにより開発・デバッグ時の問題特定が容易になる。

#### Acceptance Criteria

1. **If** CueQueue に対してキャパシティ上限が設定されており上限を超えてコマンドが追加された場合, **the** Cue System **shall** `tracing::warn!` でログ出力し、超過分の処理方針を呼び出し元に通知する
2. **If** 消費者システムが CueQueue 内のコマンド型を認識しない場合, **the** Cue System **shall** 当該コマンドを安全にスキップし `tracing::debug!` でログ出力する
3. **The** Cue System **shall** CueQueue を保持するエンティティが despawn されても panic しない
4. **The** Cue System **shall** 空の CueSheet が配送された場合にエラーを発生させず無操作で完了する
5. **If** PerformerKey の解決に失敗した場合, **the** Cue System **shall** 他の演者への正常な配送を継続する（部分的失敗の許容）
6. **If** CueQueue 内のコマンドの start_time が現在の経過時刻より過去である場合, **the** Cue System **shall** 当該コマンドを遅延到達として即時消費する（タイムラインの追いつき処理）

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

## Open Discussion Topics（未決事項）

以下の議題はレビューセッションでの確認を要する。要件の承認判断に影響する可能性がある。

### T6: CueSheet タイムライン管理の主体（Req 5 AC4, Req 6 に影響）

| 候補                               | 説明                                                      | トレードオフ                          |
| ---------------------------------- | --------------------------------------------------------- | ------------------------------------- |
| **A: 各 CueQueue が独立管理**      | CueQueue コンポーネントに elapsed_time フィールドを保持   | シンプル。CueSheet 全体の同期が困難   |
| **B: ルートに CueTimeline 配置**   | CueSheet 投入先エンティティに共有タイムラインコンポーネント | 全演者の同期が容易。ルート設計に依存 |
| **C: タイムラインエンティティ分離** | CueSheet ごとに専用エンティティで経過時刻を管理          | 柔軟だがエンティティ増加              |

**検討ポイント**: DD9 の絶対時刻方式を活かすには、CueSheet 全体の基準時刻が必要。T5 の WaitForInput ブロッキングスコープとも関連する。

### T7: CueSheet の中断・キャンセルセマンティクス（Req 4, Req 8 に影響）

- 再生中の CueSheet を中断して新しい CueSheet に切り替える場合の挙動は？
- さくらスクリプトでは新スクリプト受信時に旧スクリプトを破棄（暗黙の clear + replace）
- CueQueue の追記型設計と「台本の差し替え」の関係整理が必要
- 候補A: `CueQueue::clear()` → 新 CueSheet 配送（明示的な差し替え）
- 候補B: CueSheet にメタデータ（replace_mode: bool）を付与
- 候補C: 配送 API で clear + dispatch のアトミック操作を提供

### T8: 動的生成シナリオへの対応（Req 1, Req 4 に影響）

- DD9 の絶対時刻方式は事前にタイミングが確定するシナリオに最適
- LLM リアルタイム応答ストリーミングなど、事前に全コマンドの start_time を決定できない場合がある
- 候補A: pasta DSL 層が都度 start_time を計算し、ミニ CueSheet を逐次投入
- 候補B: CueSheet に「追記モード」を設け、前回末尾の start_time からの相対時刻で追記
- 候補C: 本仕様のスコープ外とし、動的生成は別仕様で扱う

---

## Version History

| Version | Date       | Changes                                                                         |
| ------- | ---------- | ------------------------------------------------------------------------------- |
| 1.0     | 2026-02-26 | 初版生成（8要件 + 3NFR）                                                        |
| 1.0.1   | 2026-02-26 | レビュー自明修正（F1: C3但し書き, F2: Req4 AC2実装詳細削除）                    |
| 2.0     | 2026-02-27 | DD9 絶対時刻キーフレーム方式適用。Req 1,2,3,4,5,6 を全面書き換え。議題T5-T8追加 |

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
