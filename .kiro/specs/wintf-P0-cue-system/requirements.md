# Requirements Document

| 項目 | 内容 |
|------|------|
| **Document Title** | wintf キューシステム（cue-system）要件定義書 |
| **Version** | 1.0 |
| **Date** | 2026-02-26 |
| **Priority** | P0 (MVP必須) |
| **Status** | 📋 Generated - v1.0 |

---

## Introduction

本仕様書は、演出指令（キュー）の構造化定義・配送・消費メカニズムを ECS 上に確立する汎用基盤仕様の要件を定義する。本仕様は実質的に「コンテンツ再生を指示するためのミニ言語」の設計であり、伺かプラットフォームの「さくらスクリプト」が果たしていた役割を、ECS アーキテクチャと Rust の型安全性で再構成するものである。

### 設計メタファー: 舞台演出のキューシート

> **演劇シーンを与えたら、演者が演じてくれる**

cue-system は**舞台演出のキューシート**をメタファーとする。

| 概念 | 説明 | さくらスクリプトでの対応 |
|------|------|------------------------|
| **キューシート (CueSheet)** | 構造化された演出台本。どの演者がいつ何をするかを記述する | さくらスクリプト文字列そのもの |
| **演者 (Performer)** | キューシートの指示を受けて演技するエンティティ | スコープ対象（`\0` = さくら、`\1` = うにゅう） |
| **キュー (Cue)** | 個々の演出指示 | 各タグ（`\s[0]`, `\w[500]`）や表示文字 |
| **キューキュー (CueQueue)** | 各演者の手持ちの、次の演技指示を待つ行列 | ベースウェアの内部バッファ（非公開） |
| **配送 (Dispatch)** | 台本を各演者に配る行為 | スコープ切替によるコマンドの暗黙的な振り分け |

### さくらスクリプトからの設計継承と脱構築

さくらスクリプトの設計をギャップ分析レベルで評価し、継承すべき概念と再設計すべき概念を分離する。詳細な調査レポートは [research.md](./research.md) を参照。

#### 継承する概念（オマージュ）

| 概念 | さくらスクリプトでの表現 | cue-system での再構成 |
|------|------------------------|---------------------|
| **単一台本モデル** | 1つのスクリプトが全キャラクターの演技を記述 | CueSheet が複数演者への指示を包含 |
| **直感的なタイミング** | `\w[N]`, `\x`, `\_q` の3種 | Wait, WaitForInput, Instant の型安全な表現 |
| **掛け合い** | `\0`, `\1` でキャラクター間を行き来 | CueSheet 内で演者を明示指定、配送時にキュー分離 |
| **演出のメタファー** | スクリプト = 舞台の脚本 | CueSheet = 舞台のキューシート |
| **逐次投入の自然さ** | テキストとタグが混在し読みやすい | pasta DSL 層がこの書きやすさを維持 |

#### 脱構築する概念（再設計）

| さくらスクリプトの課題 | 影響 | cue-system での解決方針 |
|----------------------|------|----------------------|
| テキストストリーム形式 | パースエラーが実行時まで検出不可 | 型付き enum コマンド（コンパイル時検証） |
| 非統一タグ構文 | `\s[N]`, `\w5`, `\![cmd,p]` が混在 | 統一された CueCommand enum 体系 |
| 逐次専用（並列不可） | テキスト表示中にSE再生を同時に行えない | エンティティごとの独立キュー（本質的に並列） |
| グローバルスコープ状態 | 長いスクリプトで現在のスコープが不明瞭 | 各コマンドが対象演者を明示的に保持 |
| コンテンツと制御の密結合 | テキストだけ抽出、タイミングだけ変更が困難 | 構造化コマンドでフィルタリング・変換可能 |
| 固定タグ形式 | ベースウェア間の互換性問題 | 消費者ごとの型安全な拡張メカニズム |

### スコープ

**含まれるもの:**
- CueSheet（構造化演出台本）のデータモデル定義
- CueCommand（演出指令）の基盤コマンド型定義
- CueQueue（エンティティキュー）コンポーネント設計
- CueSheet → CueQueue 配送メカニズム
- キュー消費プロトコル（消費者システム向けインターフェース）
- タイミング制御セマンティクス（Wait, WaitForInput, Instant, Skip）
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

| 既存資産 | 関係性 |
|---------|--------|
| `TypewriterToken` (Stage 1 IR) | cue-system の基盤コマンド型の先行実装。Text, Wait, FireEvent の3バリアント |
| `TypewriterTalk` | CueQueue の特殊化（丸ごと差し替え方式）。cue-system はこれを append 可能な汎用キューに拡張 |
| `DolaRuntime` | タイミングオーケストレーションの外部エンジン。`subscribe`/`update` によるタイムライン連携 |
| `Messages<T>` (Drag系) | 参考にした ECS パターン。cue-system はコンポーネントベースで実装 |
| `CommandSender` (mpsc) | 非同期→ECS の通信経路。CueSheet の非同期受け渡しに活用可能 |
| pasta DSL | CueSheet の主要な生成者。里々がさくらスクリプトを生成する関係に相当 |

### 消費者（本仕様の利用者となる仕様群）

| 消費者 | コマンド例 | 位置づけ |
|--------|-----------|---------|
| balloon03-content | テキスト表示、Wait、スタイル変更、感情値切替、Clear | バルーンのコンテンツ再生 |
| animation-system | サーフェス切替、トランジション、ポーズ変更 | キャラクター演技 |
| 将来の演出要素 | SE再生、画面効果、ウィンドウ演出 | 拡張演出 |

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

| 方式 | 仕組み | 評価 |
|------|--------|------|
| A: コンポーネント差し替え | `XxxTalk::new(commands)` で丸ごと insert | Typewriter踏襲。「追加」が不自然 |
| B: 子エンティティ追加 | `ContentCommand` entity を `ChildOf` で spawn | ECS的に自然。順序保証の設計が必要 |
| C: Messages\<T\> | bevy_ecs メッセージキュー経由 | 既存パターン(Drag系)。送り先特定が課題 |
| D: VecDeque コンポーネント | ルートに `CueQueue(VecDeque)` を付与、直接 push | 明白。Changed 検出が Mut\<T\> 必須 |

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

### Requirement 1: CueSheet — 構造化演出台本モデル

**Objective:** 開発者として、複数の演者への演出指示を含む構造化された台本（CueSheet）を定義したい。それにより pasta DSL や外部システムが型安全に演出シーンを記述でき、cue-system がそれを各演者に配送できる基盤を確立できる。

**さくらスクリプト継承**: 単一台本モデル（1つのスクリプトが全キャラクターの演技を記述する概念）を型安全な構造に再構成

#### Acceptance Criteria

1. **The** Cue System **shall** 演出台本を表現する CueSheet データ構造を提供する
2. **The** Cue System **shall** CueSheet 内の各指示（Cue）が対象演者の識別子（PerformerKey）を明示的に保持する設計とする
3. **The** Cue System **shall** CueSheet 内の指示を挿入順序で保持する（演出の時系列を保証）
4. **The** Cue System **shall** 1つの CueSheet 内に複数の演者への指示を混在して記述できる
5. **The** Cue System **shall** CueSheet から特定演者の指示のみをフィルタリング抽出する API を提供する
6. **The** Cue System **shall** CueSheet を Clone, Debug derive 可能にする（ログ出力・複製対応）

---

### Requirement 2: CueCommand — 型安全な基盤コマンド体系

**Objective:** 開発者として、演出指示を型安全な enum として定義したい。それにより文字列パースに依存せず、コンパイル時に不正なコマンドを検出でき、IDE 補完の恩恵を受けられる。

**さくらスクリプト脱構築**: テキストストリーム形式・非統一タグ構文（`\s[N]`, `\w5`, `\![cmd,p]`）を、統一された型付き enum に置換

#### Acceptance Criteria

1. **The** Cue System **shall** 基盤コマンド型を enum として定義する
2. **The** Cue System **shall** 基盤コマンドにテキスト表示バリアント（表示対象文字列を保持）を含める
3. **The** Cue System **shall** 基盤コマンドに時間ウェイトバリアント（待機時間を保持）を含める
4. **The** Cue System **shall** 基盤コマンドにユーザー入力待ちバリアント（タイムアウト設定を任意で保持）を含める
5. **The** Cue System **shall** 基盤コマンドに即時モード切替バリアント（以降の表示系コマンドのウェイトを無効化）を含める
6. **The** Cue System **shall** 基盤コマンドにコンテンツクリアバリアントを含める
7. **The** Cue System **shall** 基盤コマンドにスタイル変更バリアント（感情値キーを保持）を含める
8. **The** Cue System **shall** 基盤コマンドに消費者固有コマンドを格納するための拡張バリアントを含める
9. **The** Cue System **shall** 各コマンドバリアントのパラメータに適切な Rust 型を付与する（文字列パラメータへの依存を最小化）
10. **The** Cue System **shall** 基盤コマンド型に Clone, Debug を derive する

---

### Requirement 3: CueQueue — エンティティキューコンポーネント

**Objective:** 開発者として、各演者エンティティに演出指示のキューを持たせたい。それにより外部から任意のタイミングでコマンドを追加（append）でき、消費者システムが先頭から順次消費できる FIFO キューを確立する。

**さくらスクリプト脱構築**: グローバルスコープ状態 → エンティティごとの独立キュー。TypewriterTalk の丸ごと差し替え → append 可能なキュー。

#### Acceptance Criteria

1. **The** Cue System **shall** CueQueue を ECS コンポーネントとして提供する
2. **The** Cue System **shall** CueQueue を FIFO（先入先出）セマンティクスで動作させる
3. **The** Cue System **shall** CueQueue にコマンドを末尾追加（append）する API を提供する
4. **The** Cue System **shall** CueQueue からコマンドを先頭取得・除去（pop_front）する API を提供する
5. **The** Cue System **shall** CueQueue の先頭要素を除去せずに参照（peek）する API を提供する
6. **The** Cue System **shall** CueQueue の空判定（is_empty）および件数取得（len）API を提供する
7. **The** Cue System **shall** CueQueue のコンテンツ全消去（clear）API を提供する
8. **The** Cue System **shall** 同一 World 内に複数の CueQueue を独立して存在させられる（エンティティごとに1つ）

---

### Requirement 4: CueSheet 配送メカニズム

**Objective:** 開発者として、CueSheet を投入するだけで各演者の CueQueue にコマンドが自動配送されるようにしたい。それにより外部システム（pasta DSL 等）は CueSheet の構築に専念でき、配送の詳細を意識する必要がない。

**さくらスクリプト継承・脱構築**: スコープ切替（`\0`, `\1`）による暗黙の振り分け → 明示的な PerformerKey + 自動分配

#### Acceptance Criteria

1. **The** Cue System **shall** CueSheet 内の各 Cue を、PerformerKey に対応する演者エンティティの CueQueue に分配する配送メカニズムを提供する
2. **The** Cue System **shall** PerformerKey から対象エンティティを解決する仕組み（演者レジストリまたは解決関数）を提供する
3. **The** Cue System **shall** 配送時に各演者向けのコマンド順序を CueSheet 内の出現順で保持する
4. **When** CueSheet がシステムに投入された時, **the** Cue System **shall** 対象となる全演者の CueQueue にコマンドを末尾追加する
5. **If** CueSheet 内の PerformerKey に対応するエンティティが見つからない場合, **the** Cue System **shall** `tracing::warn!` でログ出力し、該当コマンドをスキップする（他の演者への配送は継続）
6. **The** Cue System **shall** 既に CueQueue にコマンドが存在するエンティティに対しても、追加の CueSheet 配送による末尾追加を行える（任意タイミングでの逐次投入）

---

### Requirement 5: キュー消費プロトコル

**Objective:** 開発者として、消費者システム（Typewriter、AnimationSystem 等）が CueQueue からコマンドを消費するための統一的なプロトコルを利用したい。それにより各消費者が一貫した方法でキューを処理でき、タイミング制御の共通セマンティクスを保証できる。

**さくらスクリプト脱構築**: ベースウェア内部の逐次ブロッキング解釈 → ECS システムによるフレーム単位の非同期消費

#### Acceptance Criteria

1. **The** Cue System **shall** 消費者システムがフレームごとに CueQueue の先頭からコマンドを消費できるプロトコルを定義する
2. **When** CueQueue の先頭が時間ウェイトコマンドの場合, **the** Cue System **shall** 指定時間が経過するまで後続コマンドの消費をブロックするセマンティクスを定義する
3. **When** CueQueue の先頭がユーザー入力待ちコマンドの場合, **the** Cue System **shall** 外部入力を受信するまで後続コマンドの消費をブロックするセマンティクスを定義する
4. **When** 即時モードが有効な場合, **the** Cue System **shall** テキスト表示コマンドを待ち時間なしで即時消費する
5. **The** Cue System **shall** 非ブロッキングコマンド（テキスト表示、スタイル変更等）をフレーム内で連続消費（バッチ消費）できるパターンを定義する
6. **The** Cue System **shall** キュー消費の現在状態（消費中・ウェイト中・入力待ち・完了）を追跡する消費ステート管理を提供する
7. **When** CueQueue の全コマンドが消費された時, **the** Cue System **shall** 消費完了状態を示す

---

### Requirement 6: タイミング制御と dola 統合

**Objective:** 開発者として、シンプルな Wait コマンドから高度な dola オーケストレーションまで段階的にタイミング制御を利用したい。それにより単純なコマンドベースのタイミングと、宣言的アニメーション定義による精密なタイミングを組み合わせた豊かな演出が可能となる。

**さくらスクリプト継承**: `\w[N]`（間合い）、`\x`（クリック待ち）、`\_q`/`\t`（即時表示）の直感的なタイミング制御

#### Acceptance Criteria

1. **The** Cue System **shall** 時間ウェイトコマンドの経過計測をシステム時間ベースで行う
2. **The** Cue System **shall** CueQueue の消費を一時停止（pause）する API を提供する
3. **The** Cue System **shall** CueQueue の消費を再開（resume）する API を提供する
4. **The** Cue System **shall** CueQueue の残コマンドを即時完了（skip）する API を提供する
5. **The** Cue System **shall** CueQueue の消費速度を変更する手段を提供する（倍速/低速対応）
6. **Where** dola フィーチャーが有効な場合, **the** Cue System **shall** DolaRuntime のタイムライン進行と CueQueue の消費タイミングを連携させるインターフェースを提供する
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

---

## Non-Functional Requirements

### NFR-1: パフォーマンス

1. **The** Cue System **shall** CueQueue のコマンド追加・先頭消費操作を O(1) 償却で実行する
2. **While** CueQueue が空の状態, **the** Cue System **shall** 消費者システムの不要な走査を最小化する
3. **The** Cue System **shall** コマンド型のメモリサイズをキャッシュフレンドリーな範囲に抑える

### NFR-2: デバッグ容易性

1. **The** Cue System **shall** 全てのコマンド型に Debug derive を付与する
2. **The** Cue System **shall** CueSheet の配送ログを `tracing::debug!` で出力する
3. **The** Cue System **shall** CueQueue の消費ログを `tracing::trace!` で出力する（高頻度のため trace レベル）

### NFR-3: ECS 親和性

1. **The** Cue System **shall** bevy_ecs 0.18.0 のコンポーネントシステムに準拠する
2. **The** Cue System **shall** 既存の wintf レイヤー構造（COM → ECS → Message Handling）の依存方向に違反しない
3. **The** Cue System **shall** CueQueue を SparseSet ストレージで管理する（動的変更が頻繁なため）

---

## Dependencies

### 参照仕様

| 仕様 | 参照内容 |
|------|---------|
| `wintf-P0-typewriter` ✅ | TypewriterToken（Stage 1 IR）パターン、TypewriterTalk の消費モデル |
| `wintf-P0-event-system` ✅ | Messages\<T\> パターン、CommandSender (mpsc) パターン |

### 後続仕様（cue-system を利用する仕様）

| 仕様 | 利用内容 |
|------|---------|
| `wintf-P0-balloon01-core` | ブロッカー先。バルーンのコンポーネント構成原則に反映 |
| `wintf-P0-balloon03-content` | テキスト系 CueCommand の消費者実装 |
| `wintf-P0-animation-system` | サーフェス系拡張コマンドの消費者実装 |
| `areka-P0-reference-ghost` | pasta DSL → CueSheet の生成者 |

### 外部依存

| 外部 | 依存内容 |
|------|---------|
| pasta DSL（外部リポジトリ） | CueSheet の主要な生成者。pasta → CueSheet 変換はアプリケーション層の責務 |
| dola クレート | タイミングオーケストレーション。`#[cfg(feature = "dola")]` で条件コンパイル |
