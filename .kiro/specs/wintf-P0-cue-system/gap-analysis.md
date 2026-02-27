# Gap Analysis: wintf-P0-cue-system

| 項目             | 内容                                            |
| ---------------- | ----------------------------------------------- |
| **対象仕様**     | wintf-P0-cue-system（演出キュー配送基盤）       |
| **分析日**       | 2026-02-27                                      |
| **Requirements** | v2.0（8要件 / 45受入基準 + 3NFR / 10受入基準）  |
| **分析種別**     | グリーンフィールド新規 + 既存パターン統合       |
| **分析範囲**     | crates/wintf/src/ecs/, crates/dola/src/runtime/ |
| **前提設計判断** | DD9: 絶対時刻キーフレーム方式（start_time付きCue） |

---

## 1. 現状調査サマリ

### 1.1 既存アセット (特に重要な発見事項)

| アセット                                  | パス                                    | 関連性                                                                    |
| ----------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------- |
| `TypewriterToken` (Stage 1 IR)            | `ecs/widget/text/typewriter_ir.rs`      | cue-system の **先行実装**。Text, Wait, FireEvent の3バリアント enum      |
| `TypewriterTalk`                          | `ecs/widget/text/typewriter.rs`         | CueQueue の特殊化版（丸ごと差し替えモデル、**既に start_time 機構保持**） |
| `TypewriterTimeline` (Stage 2 IR)         | `ecs/widget/text/typewriter_ir.rs`      | **既に絶対時刻ベースのタイムライン実装（show_at, start_at, fire_at 保持）** — DD9検証の実証 |
| `TypewriterState`                         | `ecs/widget/text/typewriter.rs`         | 消費ステート管理の先行実装（Playing/Paused/Completed）                    |
| `TypewriterEvent` / `TypewriterEventKind` | `ecs/widget/text/typewriter_ir.rs`      | イベント通知パターン（SparseSet + Changed\<T\>）                          |
| `Typewriter` コンポーネント               | `ecs/widget/text/typewriter.rs`         | on_add フックで Visual + 空 TypewriterTalk を自動挿入するパターン         |
| `update_typewriters` システム             | `ecs/widget/text/typewriter_draw.rs`    | フレーム単位のタイムライン進行 + FireEvent 処理。消費プロトコルの参考実装 |
| `DolaRuntime` ファサード                  | `crates/dola/src/runtime/facade.rs`     | タイミングオーケストレーション。subscribe/load_document/start/update API。**f64秒の絶対時刻ベース** |
| `EvaluatedValue` / `UpdateResult`         | `crates/dola/src/runtime/facade.rs`     | dola→ECS 差分配信の出力型                                                 |
| `Messages<T>` (Drag系)                    | `ecs/world/mod.rs`                      | bevy_ecs メッセージキュー。init_resource + FrameFinalize 更新パターン     |
| `CommandSender` (mpsc)                    | `ecs/widget/bitmap_source/task_pool.rs` | 非同期→ECS コマンド送信。CueSheet の非同期投入路の候補                    |
| `WintfTaskPool`                           | `ecs/widget/bitmap_source/task_pool.rs` | BoxedCommand のドレイン→World適用パターン                                 |
| `FrameTime` リソース                      | `ecs/graphics/core.rs`                  | フレーム時刻（**f64秒、起動時からの絶対時刻**）。ウェイト計測のタイムソース。**`GetSystemTimePreciseAsFileTime` 使用** |
| スケジュール実行順序                      | `ecs/world/mod.rs`                      | Input → **Update** → PreLayout → Layout → PostLayout → UISetup → GraphicsSetup → Draw → ... → FrameFinalize |
| `DragConfig` / `OnDrag` 等                | `ecs/drag/mod.rs`                       | SparseSet コンポーネント + on_add パターンの参考                          |
| `Brush` / `BrushInherit`                  | `ecs/widget/brushes.rs`                 | コンポーネント自動挿入 + 解決パターンの参考                               |

### 1.2 確立済みパターン

**レイヤー構成**: COM → ECS → Message Handling の3層
**ECS スケジュール順**: `Input` → `Update`(キュー消費) → `PreLayout` → `Layout` → `PostLayout` → `UISetup` → `GraphicsSetup` → `Draw` → `PreRenderSurface` → `RenderSurface` → `Composition` → `CommitComposition` → `FrameFinalize`

#### コンポーネント設計パターン

- **on_add フックチェーン**: `Typewriter → Visual + TypewriterTalk` の自動挿入パターン（全10箇所の on_add hook 実装確認済み）
- **SparseSet ストレージ**: 動的変更が頻繁なコンポーネントの標準（TypewriterTalk, TypewriterEvent, DragConfig 等、計27コンポーネント）
- **2段階 IR パターン**: Stage 1 IR（外部インターフェース）→ Stage 2 IR（内部処理用）の分離
- **Changed\<T\> リアクティブクエリ**: コンポーネント変更検出による遅延処理（計8箇所で利用実績）
- **on_remove クリーンアップ**: TypewriterTalk の on_remove フックでリソース解放

#### タイミング & 時間管理パターン（dola 思想との共有）

- **FrameTime 絶対時刻**: `elapsed_secs() -> f64` — 起動時からの絶対秒数。`GetSystemTimePreciseAsFileTime` ベース（100ns 精度）
- **DolaRuntime 絶対時刻**: `update(current_time: f64)` — **FrameTime と同じ時間軸の f64 秒**
- **dola 思想**: `Document/Storyboard（宣言的、相対時刻） → compile（絶対時刻化） → Runtime（実行可能） → playback(current_time)（消費）`
- **cue-system との対応**: `CueSheet（相対時刻） → dispatch(sheet_start_time)（コンパイル） → CueQueue（絶対時刻） → pop_ready(current_time)（消費）`
- **TypewriterTimeline 絶対時刻**: Stage 2 IR の TimelineItem が `show_at`, `start_at`, `fire_at` フィールドを保持 — **DD9 方式の実証例**
- **TypewriterTalk の start_time**: `start_time: f64` + `paused_elapsed: f64` で pause/resume を管理
- **update() 消費ループ**: `while elapsed >= show_at/start_at/fire_at` で時刻到達判定 → `next_item_index` カーソルで進行

#### 通信・配送パターン

- **Messages\<T\> ライフサイクル**: `world.init_resource::<Messages<T>>()` + FrameFinalize での `Messages::update()` 呼び出し
- **mpsc ドレインパターン**: `CommandSender` → `Receiver<BoxedCommand>` → Input スケジュールで `drain_commands()` → `cmd(world)` 適用
- **`#[cfg(feature = "...")]` 条件コンパイル**: dola 統合のフィーチャーフラグパターン（設計済み、実装未着手）
- **DeferredWorld::commands()**: on_add フック内でのエンティティ操作（`world.get::<T>(entity).is_none()` で存在チェック → `world.commands().entity(entity).insert(Y)` で追加）

#### データ構造パターン

- **VecDeque 利用**: `pointer/types.rs` の `PointerBuffer { samples: VecDeque<PositionSample> }` — **ただし ECS Component ではない**（thread_local リングバッファ）
- **Vec の順序保持**: `TypewriterToken: Vec<Token>` で挿入順保持（FIFO 的なアクセスで実績）

### 1.3 コーディング規約

- コンポーネント定義: `#[derive(Component)]` + `#[component(storage = "SparseSet", on_add = ..., on_remove = ...)]`
- フック関数シグネチャ: `fn on_xxx_add(mut world: DeferredWorld, hook: HookContext)`
- 自動挿入前に `world.get::<T>(entity).is_none()` で存在チェック
- ウィジットモジュール配置: `ecs/widget/{widget_name}/` サブディレクトリ
- ログレベル: `tracing::debug!`（状態変化）, `tracing::trace!`（高頻度処理）, `tracing::warn!`（回復可能エラー）
- テスト配置: `crates/wintf/tests/{module}/` 統合テスト + ファイル内 `#[cfg(test)]` ユニットテスト

### 1.4 重大な発見事項（DD9 検証）

#### ✅ DD9 絶対時刻方式の実証例が既存実装に存在

**TypewriterTimeline (Stage 2 IR)** は既に絶対時刻キーフレームモデルを実装している:

```rust
pub enum TimelineItem {
    Glyph { cluster_index: u32, show_at: f64 },
    Wait { duration: f64, start_at: f64 },
    FireEvent { target: Entity, event: TypewriterEventKind, fire_at: f64 },
}
```

- 各 item が絶対時刻フィールド（`show_at`, `start_at`, `fire_at`）を保持
- `total_duration: f64` で末尾時刻を事前計算
- `TypewriterTalk::update()` の while ループで `elapsed >= show_at` チェック → `next_item_index` カーソル進行

**DD9 の意義**: Stage 1 IR（TypewriterToken）は Wait バリアントで相対時刻を表現していたが、Stage 2 変換後は絶対時刻に変換済み。cue-system は Stage 1 レベルから絶対時刻で設計することで、**Stage 2 変換を不要にする**。

#### ✅ VecDeque は ECS Component として未使用

全コードベース調査の結果:
- VecDeque の唯一の使用箇所: `pointer/types.rs:210` の `PointerBuffer { samples: VecDeque<PositionSample> }`
- **これは thread_local リングバッファであり、ECS Component ではない**
- したがって CueQueue が `VecDeque<TimedCue>` を採用しても前例なし問題は存在しない

#### ⚠️ Changed\<T\> の gotcha

`window_proc/window_pos.rs:95-119` のコメントから:
- `Mut<T>` での get_mut は **内容が変わらなくても Changed フラグを立てる**（bevy_ecs 0.18 の仕様）
- CueQueue が `VecDeque<TimedCue>` を保持する場合、`&mut CueQueue` でのアクセスは空キューでも Changed を発火
- 対策: 「追加時だけ Mut を取得」または「Changed\<CueQueue\> フィルタを使わない」設計が必要

---

## 2. 要件別アセットマップ

### Req 1: CueSheet — 構造化演出台本モデル（絶対時刻キーフレーム方式）

| AC  | 技術要素                 | 既存アセット                        | ギャップ                                                  |
| --- | ------------------------ | ----------------------------------- | --------------------------------------------------------- |
| AC1 | CueSheet データ構造      | —                                   | **Missing**: `CueSheet` 構造体（Vec\<Cue\>、メタデータなし） |
| AC2 | ActorKey 識別子          | —                                   | **Missing**: `ActorKey` 型定義（文字列 or enum）          |
| AC3 | start_time 保持          | TypewriterTimeline が `show_at` 保持 | **Missing**: `Cue` 構造体に `start_time: f64` フィールド（CueSheet ローカル相対時刻）   |
| AC4 | start_time 昇順保持      | —                                   | **Missing**: Vec\<Cue\> の安定ソートロジック               |
| AC5 | 複数演者の混在記述       | —                                   | **Missing**: `Cue` 構造体（ActorKey + CueCommand + start_time） |
| AC6 | 同一 start_time 並行実行 | —                                   | ギャップなし（同時刻 Cue の順序保持で実現）                |
| AC7 | 演者別フィルタリング API | —                                   | **Missing**: `filter_by_actor()` 等の API                 |
| AC8 | Clone, Debug derive      | TypewriterToken は Debug + Clone 済 | ギャップなし（derive マクロ付与のみ）                     |

**評価**: CueSheet は完全新規のデータ構造。Cue.start_time は **CueSheet ローカル時刻（相対秒数）**。TypewriterTimeline が絶対時刻モデルの実証なので、設計パターンは確立済み。ActorKey の型設計（String vs NewType vs Entity）が設計フェーズの論点。

**dola 思想との対応**: CueSheet ≈ dola::Document/Storyboard（相対時刻構造）

---

### Req 2: CueCommand — 型安全な基盤コマンド体系（絶対時刻方式対応）

| AC   | 技術要素                   | 既存アセット                     | ギャップ                                                        |
| ---- | -------------------------- | -------------------------------- | --------------------------------------------------------------- |
| AC1  | 基盤コマンド enum          | `TypewriterToken`（3バリアント） | **Extend**: 3→5 バリアントへの拡張（Wait/Instant **削除済み**） |
| AC2  | テキスト表示バリアント     | `TypewriterToken::Text(String)`  | ギャップなし（直接対応、型も同一）                              |
| AC3  | ユーザー入力待ちバリアント | —                                | **Missing**: `WaitForInput { timeout: Option<f64> }` バリアント |
| AC4  | コンテンツクリアバリアント | —                                | **Missing**: `Clear` バリアント                                 |
| AC5  | 演技発現バリアント         | —                                | **Missing**: `Emote { key: String }` バリアント（演技キー保持） |
| AC6  | 拡張バリアント             | `TypewriterToken::FireEvent`     | **Redesign**: FireEvent を汎用拡張機構に再設計                  |
| AC7  | 型安全パラメータ           | TypewriterToken で実績あり       | ギャップなし（Rust enum の自然な型付け）                        |
| AC8  | Clone, Debug derive        | TypewriterToken は Debug + Clone | ギャップなし                                                    |

**評価**: DD9 により Wait バリアント **削除**（タイミングは start_time 差分で表現）、Instant バリアント **削除**（同一 start_time で並行実行）。5バリアント（Text, WaitForInput, Clear, Emote, Extension）+ 将来拡張余地で「5+」表記。TypewriterToken との後方互換は `From` トレイト変換で対応可能。

---

### Req 3: CueQueue — エンティティキューコンポーネント（絶対時刻順管理）

| AC  | 技術要素                 | 既存アセット                                 | ギャップ                                                  |
| --- | ------------------------ | -------------------------------------------- | --------------------------------------------------------- |
| AC1 | ECS コンポーネント       | `TypewriterTalk`（SparseSet コンポーネント） | **Missing**: `CueQueue` コンポーネント                    |
| AC2 | 時刻付きエントリ（TimedCue）| TypewriterTimeline の TimelineItem が実証    | **Missing**: `TimedCue { start_time: f64, command: CueCommand }` |
| AC3 | start_time 昇順維持      | —                                            | **Missing**: BinaryHeap or ソート済み Vec                 |
| AC4 | start_time 順序保持追加 API | —                                         | **Missing**: `push_sorted()` / `extend_sorted()` API      |
| AC5 | 時刻到達コマンド取得 API | TypewriterTalk::update() が `elapsed >= show_at` 実装 | **Missing**: `pop_ready(current_time)` API |
| AC6 | peek API                 | —                                            | **Missing**: `peek_next()` API                            |
| AC7 | is_empty / len API       | —                                            | **Missing**: キュー状態問い合わせ API                     |
| AC8 | clear API                | —                                            | **Missing**: `clear()` API                                |
| AC9 | エンティティごとの独立性 | TypewriterTalk がエンティティ独立性を実証    | ギャップなし（ECS コンポーネントの本質的な特性）          |

**評価**: DD9 により FIFO の VecDeque から「時刻到達順消費」の優先度キュー的構造に変更。実装選択肢は (a) `BinaryHeap<TimedCue>`（最小ヒープ、O(log n) push）、(b) `Vec<TimedCue>` ソート済み（O(n) 挿入、O(1) 先頭 pop）、(c) `VecDeque<TimedCue>` ソート済み。TypewriterTalk との対比: TypewriterTalk は丸ごと差し替えモデル、CueQueue は逐次 append + 時刻到達消費モデル。

---

### Req 4: CueSheet 配送メカニズム（絶対時刻保持・マージ挿入配送）

| AC  | 技術要素                       | 既存アセット                      | ギャップ                                                       |
| --- | ---------------------------------- | --------------------------------- | -------------------------------------------------------------- |
| AC1 | sheet_start_time 受け取り + コンパイル         | —                                 | **Missing**: `dispatch(sheet, sheet_start_time)` 関数            |
| AC2 | ActorKey→CueQueue 分配         | —                                 | **Missing**: 配送関数 / 配送システム                           |
| AC3 | 演者レジストリ / 解決関数          | —                                 | **Missing**: ActorKey → Entity 解決メカニズム              |
| AC4 | 絶対時刻順マージ挿入          | —                                 | **Missing**: CueQueue への既存エントリとのマージロジック        |
| AC5 | 全演者への配送                     | —                                 | **Missing**: `for cue in cuesheet { dispatch(...) }` ループ    |
| AC6 | 未解決 ActorKey のハンドリング | `tracing::warn!` パターン確立済み | ギャップなし（ログパターン流用）                               |
| AC7 | 逐次投入（既存キューへの追加）     | TypewriterTalk は丸ごと差し替え   | **Redesign**: 追加投入モデルは CueQueue の append で自然に実現 |

**評価**: DD9 により配送ロジックが変更 — **コンパイル（sheet_start_time + cue.start_time = 世界絶対時刻）+ 絶対時刻順マージ挿入**が必要。ActorKey の解決メカニズム（レジストリ方式 vs クエリ方式 vs マーカーコンポーネント方式）が設計フェーズの主要論点（DD2）。

**dola 思想との対応**: dispatch(sheet_start_time) ≈ dola::compile（相対時刻を絶対時刻化）

---

### Req 5: キュー消費プロトコル（時刻到達消費モデル）

| AC  | 技術要素             | 既存アセット                                    | ギャップ                                                             |
| --- | -------------------- | ----------------------------------------------- | -------------------------------------------------------------------- |
| AC1 | 時刻到達消費         | TypewriterTalk::update() で `elapsed >= show_at` 判定 | **Adapt**: `while queue.peek_next().start_time <= current_time` パターンを汎化 |
| AC2 | current_time 受け取り | —                                           | **Missing**: `pop_ready(current_time: f64)` プロトコル（CueQueue は経過時刻を持たない）         |
| AC3 | 同一 start_time 一括消費 | —                                           | **Missing**: 同時刻コマンド完全消費ループ                             |
| AC4 | 入力待ちブロッキング | —                                               | **Missing**: WaitForInput の演者ごとブロッキングセマンティクス（Q5 確定）  |
| AC5 | 消費ステート管理     | `TypewriterState`（Playing/Paused/Completed）   | **Extend**: TypewriterState を汎用化 + WaitingForInput 状態追加      |
| AC6 | 消費完了状態         | `TypewriterState::Completed`                    | **Adapt**: 既存パターンの流用                                        |

**評価**: DD9 により消費プロトコルが **FIFO 先頭消費 → 時刻到達消費**に変更。**CueQueue は経過時刻を管理せず、外部から current_time を受け取る**（Q6 確定）。TypewriterTalk::update() が時刻ベース判定の実証実装として存在。WaitForInput のブロッキングスコープは演者ごとブロック確定（Q5）。

**dola 思想との対応**: pop_ready(current_time) ≈ dola::playback（外部から時刻を受け取り、到達済み要素を返す）

---

### Req 6: タイミング制御と dola 統合（統一時間軸モデル）

| AC  | 技術要素                     | 既存アセット                                      | ギャップ                                                               |
| --- | ---------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------- |
| AC1 | システム時間ベースの経過計測 | `FrameTime.elapsed_secs()` で実績あり             | ギャップなし（FrameTime リソース利用）                                 |
| AC2 | pause API                    | `TypewriterTalk::pause(current_time)` で実績あり  | **Adapt**: CueQueue 向け pause API                                     |
| AC3 | resume API                   | `TypewriterTalk::resume(current_time)` で実績あり | **Adapt**: CueQueue 向け resume API                                    |
| AC4 | skip API                     | `TypewriterTalk::skip()` で実績あり               | **Adapt**: 経過時刻を末尾コマンド start_time まで進める                |
| AC5 | playback_rate（速度倍率）    | —                                                 | **Missing**: 速度倍率フィールド + 適用ロジック                         |
| AC6 | dola タイムライン連携        | `DolaRuntime::update(time: f64)` が確立済み API   | ギャップなし（FrameTime と dola が同じ f64 秒時間軸）                  |
| AC7 | dola 変数公開                | `DolaRuntime::subscribe(var_name)` が確立済み API | **Missing**: CueQueue 消費進行を dola 変数として公開するバインディング |

**評価**: TypewriterTalk が pause/resume/skip の概念実証として存在。DD9 により playback_rate の意味が明確化 — start_time に対する倍率。dola 統合は `#[cfg(feature = "dola")]` で条件コンパイルする方針が確立済み（ただし wintf Cargo.toml に dola 依存はまだ未追加、C1 制約）。FrameTime と DolaRuntime が**同じ f64 秒時間軸**を使用しており、統一が容易。

---

### Req 7: コマンド型安全拡張メカニズム

| AC  | 技術要素                      | 既存アセット                         | ギャップ                                                               |
| --- | ----------------------------- | ------------------------------------ | ---------------------------------------------------------------------- |
| AC1 | 拡張バリアントによる格納      | `TypewriterToken::FireEvent`         | **Redesign**: FireEvent は特定用途。汎用拡張バリアントへの再設計が必要 |
| AC2 | ドメイン固有コマンドの取出し  | —                                    | **Missing**: 拡張コマンドのパターンマッチ + skip/passthrough パターン  |
| AC3 | Debug トレイト要求            | TypewriterToken は Debug derive 済み | ギャップなし（derive マクロ + トレイト境界）                           |
| AC4 | enum ベースの static dispatch | —                                    | **Design Decision**: enum ネスト vs trait object vs generic の選択（DD3） |
| AC5 | ドキュメント / 使用例         | —                                    | **Missing**: バルーン向け・アニメーション向けの拡張例ドキュメント      |

**評価**: 拡張メカニズムの設計は cue-system の核心的な設計判断（DD3）。TypewriterToken::FireEvent は Entity + EventKind という特定のペイロードを持つが、汎用拡張は任意のドメインコマンドを格納する必要がある。enum ネスト方式（`Extension(BalloonCommand)` / `Extension(AnimationCommand)`）が有力候補だが、消費者が増えた場合の開閉原則への影響が設計フェーズの論点。

---

### Req 8: エラーハンドリングと堅牢性

| AC  | 技術要素                   | 既存アセット                         | ギャップ                                                  |
| --- | -------------------------- | ------------------------------------ | --------------------------------------------------------- |
| AC1 | キャパシティ上限チェック   | —                                    | **Missing**: オプショナルなキャパシティ設定 + warn ログ   |
| AC2 | 未知コマンドのスキップ     | —                                    | **Missing**: 消費者側の unknown コマンドハンドリング      |
| AC3 | despawn 耐性               | TypewriterTalk の on_remove フック   | **Adapt**: CueQueue の on_remove フック（クリーンアップ） |
| AC4 | 空 CueSheet のハンドリング | —                                    | ギャップなし（空 Vec での no-op は自然）                  |
| AC5 | 部分的失敗の許容           | `tracing::warn!` + continue パターン | ギャップなし（既存パターン流用）                          |
| AC6 | 遅延到達の追いつき処理     | —                                    | **Missing**: start_time < current_time のコマンド即時消費 |

**評価**: エラーハンドリングの大部分は既存パターンの流用。AC6（遅延到達）は DD9 特有の新要求 — start_time が過去のコマンドを即時消費する catch-up 処理。

---

### Req 9: CueSheet ライフサイクルと実行結果 — フィーチャーモデル

| AC  | 技術要素                      | 既存アセット                                      | ギャップ                                                                     |
| --- | ----------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------------- |
| AC1 | `CueSheetResult` 型           | —                                                 | **Missing**: `CueSheetResult` enum（Completed/Cancelled/Timeout/Choice）      |
| AC2 | Completed 通知                | `TypewriterState::Completed` パターン             | **Adapt**: 全演者 CueQueue 消費完了の検知ロジック                             |
| AC3 | Cancelled 通知                | —                                                 | **Missing**: 外部キャンセル API + Cancelled 発行メカニズム                    |
| AC4 | Timeout 通知                  | WaitForInput の `timeout: Option<f64>` フィールド | **Adapt**: タイムアウト超過検知 + Timeout 発行ロジック                        |
| AC5 | Choice 通知                   | —                                                 | **Missing**: 選択肢選択イベントの検知 + Choice 発行メカニズム                |
| AC6 | 上位層への await 形式提供     | —                                                 | **Missing**: Rust 的 await パターン（DD11 で実装方式決定）                    |

**評価**: CueSheet を「フィーチャー実行単位（Modal Dialog モデル）」として定義する新 Requirement。T7（キャンセル）を Cancelled バリアントとして統合。T8（動的生成）はスコープ外確定。ECS 的な await 実現方法（Observer vs AsyncTask vs Poll）が DD11 として設計フェーズの主要論点。

**dola 思想との対応**: CueSheet 実行完了通知 ≈ dola の playback 完了通知パターン

---

### NFR-1: パフォーマンス

| AC  | 技術要素               | 既存アセット                           | ギャップ                                                               |
| --- | ---------------------- | -------------------------------------- | ---------------------------------------------------------------------- |
| AC1 | 効率的な追加・消費     | **BinaryHeap は O(log n)**             | ギャップなし（O(log n) 許容に緩和確定。実用キュー長で問題なし）        |
| AC2 | 空キュー時の走査最小化 | bevy_ecs クエリフィルタ                | **Design Decision**: With\<CueQueue\> + Changed でフィルタするか        |
| AC3 | メモリサイズ最適化     | TypewriterToken は32バイト未満（推定） | **Verify**: CueCommand のサイズをコンパイル時に assert する            |
| AC4 | TimedCue 64バイト制約  | —                                      | **Verify**: `size_of::<TimedCue>() <= 64` を assert（NFR-1 AC4 で追加） |

**評価**: DD9 により O(1) 保証は非現実的（時刻順キューの本質的制約）。AC1 を「効率的な追加・消費（O(log n) 許容）」に緩和確定（Q1 結論）。BinaryHeap 採用により実用上の影響は軽微（log₂(100) ≈ 7 回の比較）。

---

### NFR-2: デバッグ容易性

| AC  | 技術要素         | 既存アセット               | ギャップ                                      |
| --- | ---------------- | -------------------------- | --------------------------------------------- |
| AC1 | Debug derive     | 全既存コンポーネントで実績 | ギャップなし                                  |
| AC2 | 配送ログ         | `tracing::debug!` パターン | **Missing**: dispatch_cue_sheet のログ出力    |
| AC3 | 消費ログ         | `tracing::trace!` パターン | **Missing**: キュー消費のトレースログ         |
| AC4 | 経過時刻ログ     | —                          | **Missing**: 経過時刻と次回消費予定のログ出力 |

**評価**: 全 AC が既存ログパターンの適用で対応可能。AC4 は DD9 特有の追加（start_time ベースの次回消費予定時刻）。

---

### NFR-3: ECS 親和性

| AC  | 技術要素               | 既存アセット                  | ギャップ                             |
| --- | ---------------------- | ----------------------------- | ------------------------------------ |
| AC1 | bevy_ecs 0.18 準拠     | 全既存コンポーネントで実績    | ギャップなし                         |
| AC2 | レイヤー依存方向の遵守 | COM → ECS → Message Handling  | ギャップなし（ECS レイヤー内で完結） |
| AC3 | SparseSet ストレージ   | TypewriterTalk, DragConfig 等 | ギャップなし                         |

**評価**: 全 AC が既存パターンの準拠で自動的に満たされる。

---

## 3. ギャップサマリ

### Missing（新規作成が必要）

| #   | アイテム                                                     | 関連要件 | 複雑度                                |
| --- | ------------------------------------------------------------ | -------- | ------------------------------------- |
| M1  | `CueSheet` 構造体（Vec\<Cue\>、メタデータなし）              | Req 1    | 低（データ構造のみ）                  |
| M2  | `Cue` 構造体（ActorKey + CueCommand + **start_time**）        | Req 1    | 低                                    |
| M3  | `ActorKey` 型定義                                            | Req 1, 4 | 低〜中（型設計の判断が必要、DD1）     |
| M4  | `CueCommand` enum（**5バリアント** — Wait/Instant削除済み）  | Req 2    | 低（TypewriterToken の簡略化）        |
| M5  | `TimedCue` 構造体（start_time + CueCommand）                 | Req 3    | 低                                    |
| M6  | `CueQueue` コンポーネント（時刻順キュー + API）              | Req 3    | 低〜中（データ構造選択が影響）        |
| M7  | 配送関数 / 配送システム（`dispatch_cue_sheet`）              | Req 4    | 中（ActorKey 解決 + マージ挿入）      |
| M8  | ActorKey → Entity 解決メカニズム（レジストリ or クエリ）     | Req 4    | 中（設計判断が必要、DD2）             |
| M9  | WaitForInput の演者ごとブロッキングセマンティクス            | Req 5    | 中（外部入力との連携設計、Q5 確定）   |
| M10 | 同一 start_time 一括消費ロジック                             | Req 5    | 低                                    |
| M11 | 時刻到達消費プロトコル（`pop_ready`）                        | Req 5    | 低〜中（TypewriterTalk パターン汎化） |
| M12 | 消費ステート enum（CueQueueState + WaitingForInput）         | Req 5    | 低（TypewriterState の拡張）          |
| M13 | playback_rate フィールド + 適用ロジック                      | Req 6    | 低                                    |
| M14 | dola 連携インターフェース（`#[cfg(feature = "dola")]`）      | Req 6    | 中〜高（DolaBridgeResource 設計依存） |
| M15 | 拡張コマンドの型設計                                         | Req 7    | 中〜高（核心的設計判断、DD3）         |
| M16 | 拡張コマンドの消費パターン文書                               | Req 7    | 低（ドキュメントのみ）                |
| M17 | キャパシティ上限チェック（オプショナル）                     | Req 8    | 低                                    |
| M18 | 遅延到達コマンドの追いつき処理                               | Req 8    | 低                                    |
| M19 | モジュール構造（`ecs/cue/` or `ecs/widget/cue/`）            | 全体     | 低（スキャフォールド）                |
| M20 | `CueSheetResult` enum（Completed/Cancelled/Timeout/Choice）  | Req 9    | 低                                    |
| M21 | 全演者 CueQueue 完了検知ロジック                             | Req 9    | 中（全演者の状態追跡が必要）          |
| M22 | CueSheetResult 通知メカニズム（DD11 決定後実装）             | Req 9    | 中〜高（await 実現方法による）        |

### Adapt（既存パターンの汎化・適用）

| #   | アイテム                           | 元パターン                      | 関連要件 |
| --- | ---------------------------------- | ------------------------------- | -------- |
| A1  | CueQueue の pause/resume/skip API  | TypewriterTalk の同名メソッド   | Req 6    |
| A2  | 時刻到達消費プロトコル             | update_typewriters の走査ループ + `elapsed >= show_at` 判定 | Req 5    |
| A3  | on_remove フックでのクリーンアップ | on_typewriter_talk_remove       | Req 8    |
| A4  | CueCommand に Clone, Debug derive  | TypewriterToken の derive       | Req 2    |
| A5  | SparseSet コンポーネント宣言       | TypewriterTalk の #[component]  | Req 3    |

### Redesign（根本的な再設計）

| #   | アイテム                                        | 既存                                | 理由                                                               |
| --- | ----------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------ |
| R1  | 差し替え → append モデルへの転換                | TypewriterTalk::new()（丸ごと差替） | CueQueue は逐次投入が本質であり、差し替えモデルは互換性がない      |
| R2  | FIFO → 時刻到達消費への転換                     | VecDeque の先頭消費                 | DD9 絶対時刻方式により、start_time 到達判定ベースの消費に変更      |
| R3  | start_time 順マージ挿入                         | append のみの追加                   | DD9 により既存キューとの時刻順マージが必要                         |
| R4  | FireEvent → 汎用拡張バリアント                  | TypewriterToken::FireEvent          | 特定用途の2フィールド variant → 任意ドメインコマンドの格納機構へ   |
| R5  | Stage 1 IR 消費 → Stage 2 IR 変換なしの直接消費 | TypewriterTimeline（Stage 2 変換）  | CueQueue は Stage 1 レベルで消費。Stage 2 変換は各消費者の内部処理 |

### Design Decision（設計フェーズで決定すべき事項）

| #   | 決定事項                                                                | 選択肢                                                                                                                       | 影響範囲        |
| --- | ----------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | --------------- |
| DD1 | **PerformerKey の型**: String vs NewType vs Entity 直接                 | (a) `String`（柔軟）, (b) `NewType(String)`（型安全）, (c) `Entity`（直接参照）                                              | Req 1, 4        |
| DD2 | **演者解決メカニズム**: レジストリ vs クエリ vs コンポーネントマーカー  | (a) `HashMap<PerformerKey, Entity>` リソース, (b) `Query<(Entity, &PerformerMarker)>`, (c) 関数引数で渡す                    | Req 4           |
| DD3 | **拡張コマンドの型構造**: enum ネスト vs generic vs trait object        | (a) `Extension(Box<dyn CueExtension>)`, (b) `Extension<T: CueExtension>(T)`, (c) `BalloonCmd(BalloonCommand)` 固定バリアント | Req 7           |
| DD4 | **消費プロトコルの提供形態**: ドキュメント仕様 vs ヘルパーコード        | (a) 消費パターンの文書化のみ, (b) `CueConsumer` trait 提供, (c) ヘルパー関数群                                               | Req 5           |
| DD5 | **CueQueue のモジュール配置**: `ecs/cue/` vs `ecs/widget/cue/`          | (a) `ecs/cue/`（ウィジット横断的だから widget の外）, (b) `ecs/widget/cue/`（widget と同列）                                 | 全体            |
| DD6 | **TypewriterToken との関係**: 置換 vs 共存 vs From 変換                 | (a) CueCommand が TypewriterToken を完全置換, (b) 共存 + From 変換, (c) TypewriterToken を CueCommand に統合                 | Req 2, 後方互換 |
| DD7 | **CueSheet 投入の API**: コンポーネント差し替え vs 関数呼び出し vs 両方 | (a) `commands.entity(e).insert(PendingCueSheet(sheet))`, (b) `dispatch_cue_sheet(world, sheet)`, (c) 両方                    | Req 4           |
| DD8 | **dola 統合の粒度**: 最小限 vs DolaBridgeResource 完全統合              | (a) インターフェース定義のみ, (b) DolaBridgeResource と CueQueue の連携システム実装                                          | Req 6           |
| **DD9** | **タイミングモデル**: ~~相対時刻~~ vs **絶対時刻キーフレーム方式** ✅     | ~~(a) Wait コマンドによる相対時刻（FIFO 逐次消費）~~, **(b) Cue に start_time フィールドを付与し絶対時刻管理（並行実行可能） — 採用済み** | Req 1,2,3,4,5,6 全体 |
| DD10 | **コマンド複雑性の哲学**: 決定論的データ vs 手続き的プログラム | (a) 純粋なデータ列（Wait バリアントなし、start_time 差分でタイミング表現）— 採用済み, (b) Wait 等の手続き的コマンドを許容 | Req 2, 将来拡張 |
| DD11 | **CueSheetResult の ECS 的 await 実現**: Observer vs AsyncTask vs Poll | (a) bevy Observer/Event（ECS イベント駆動）, (b) bevy AsyncTask（実際の Future/await）, (c) Component Poll（消費者がポーリング） | Req 9, オーケストレーション層 |

### Constraint（既存アーキテクチャ制約）

| #   | 制約                                                                                                                                                                 | 影響              |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| C1  | wintf Cargo.toml に dola 依存が未追加（`dola = { path = "../dola", optional = true }` が必要）                                                                       | Req 6             |
| C2  | DolaBridgeResource は balloon-system 設計書で定義済みだがコード未実装                                                                                                | Req 6             |
| C3  | TypewriterToken / TypewriterTalk は typewriter 専用で、cue-system とは独立して存在し続ける（DD6 の決定に依存。DD6-a 採用時は CueCommand が完全置換し本制約は無効化） | Req 2（後方互換） |
| C4  | on_add フック内で World にアクセスできる範囲が限定的（DeferredWorld の制約）                                                                                         | Req 4（配送設計） |
| C5  | bevy_ecs 0.18 の Component derive は `Clone` を求めない（手動実装は可能だが derive が自然）                                                                          | Req 3             |
| C6  | スケジュール実行順は Input → **Update** → PreLayout → ... が固定。CueQueue 消費は Update スケジュールが適切                                                          | Req 5             |
| C7  | Changed\<T\> は `Mut<T>` 取得時に無条件で発火（内容変更の有無に関わらず） — 空キューでも発火する点に注意                                                             | NFR-1 AC2         |

---

## 4. 実装アプローチ検討

### Option A: TypewriterToken の拡張 + TypewriterTalk の改修

**適用可能性**: 低（DD9 により非推奨）

TypewriterToken に WaitForInput, Clear, StyleChange, Extension バリアントを追加し、TypewriterTalk を時刻順キュー ベースに改修する方法。

**トレードオフ**:
- ✅ ファイル数が増えない
- ✅ 既存の TypewriterTalk テストが流用可能
- ❌ DD9 絶対時刻方式により Wait バリアント削除が必要 → TypewriterToken の既存コードとの不整合
- ❌ Typewriter 専用の概念（グリフ、TextLayout）と汎用キューの概念が混在
- ❌ TypewriterLayoutCache（Stage 2 IR 変換）との整合性が崩れる
- ❌ balloon/animation 消費者がTypewriter モジュールに依存する不自然な構造
- ❌ 単一責任の原則に違反

**DD9 影響**: Wait バリアントを削除すると TypewriterToken の既存消費者が全て破壊される。TypewriterToken は Stage 2 変換前提の IR なので DD9 との整合が困難。

---

### Option B: 新規モジュール `ecs/cue/` の作成 ⭐⭐

**適用可能性**: 高（**DD9 採用により最有力候補**）

`ecs/cue/` に CueSheet, CueCommand, CueQueue, 配送システムを独立モジュールとして定義。TypewriterToken / TypewriterTalk はそのまま存続させ、将来的に CueQueue → TypewriterTalk への変換層を設ける。

**ディレクトリ構造**:
```
ecs/
├── cue/
│   ├── mod.rs           ← CueSheet, Cue, PerformerKey, TimedCue, re-exports
│   ├── command.rs       ← CueCommand enum + 拡張バリアント型定義（6+バリアント）
│   ├── queue.rs         ← CueQueue コンポーネント（時刻順キュー、BinaryHeap or Vec）
│   ├── dispatch.rs      ← dispatch_cue_sheet + 演者解決 + マージ挿入
│   └── consumer.rs      ← 消費プロトコルヘルパー / CueQueueState
├── widget/
│   └── text/
│       └── typewriter*.rs  ← 変更なし（存続）
```

**統合ポイント**:
- `ecs/mod.rs` に `pub mod cue;` を追加
- CueQueue 消費は各消費者の Update システムで実行（`while queue.peek_next().start_time <= frame_time.elapsed_secs()` パターン）
- TypewriterTalk との関係は `From<CueCommand> for TypewriterToken` 変換で段階移行（DD6-b 採用時）
- dola 統合は `cue/dola_bridge.rs` を `#[cfg(feature = "dola")]` で追加

**トレードオフ**:
- ✅ 明確な責務分離（cue は cross-cutting concern として widget の外に配置）
- ✅ TypewriterToken / TypewriterTalk への影響ゼロ
- ✅ 後続消費者（balloon, animation）が自然に依存できる構造
- ✅ テスト容易（cue/ 単体でテスト可能、COM 依存なし）
- ✅ dola 統合を feature flag で隔離可能
- ✅ **DD9 絶対時刻方式との親和性が高い**（TypewriterTimeline が実証済みのパターン）
- ✅ **時刻順キュー構造の設計自由度**（BinaryHeap/Vec の選択が可能）
- ❌ ファイル数が増加（5〜6ファイル）
- ❌ TypewriterTalk への変換層が追加のオーバーヘッド（DD6-b 採用時）

**DD9 対応**:
- `Cue { performer, command, start_time }` 構造で CueSheet を定義
- `TimedCue { start_time, command }` で CueQueue エントリを保持
- 配送時にマージ挿入（`insert_sorted()` or `BinaryHeap::push()`）
- 消費時に `while peek_next().start_time <= current_time` で時刻到達判定

---

### Option C: CueCommand を wintf 外部のクレートとして分離

**適用可能性**: 低〜中（DD9 により時期尚早）

CueCommand / CueSheet の型定義を `crates/cue/` として独立クレート化し、wintf と areka の両方から参照する方法。

**トレードオフ**:
- ✅ pasta DSL クレートから直接参照可能
- ✅ wintf への依存なしで CueSheet 構築が可能
- ❌ CueQueue（ECS コンポーネント）は bevy_ecs 依存のため wintf 内に必要
- ❌ クレート分割の早期最適化（消費者が未実装の段階では時期尚早）
- ❌ Cargo workspace の複雑度増加
- ❌ DD9 による設計変更の影響範囲が複数クレートに跨る

**DD9 影響**: start_time を持つ Cue 構造が変更された場合、外部クレートの更新も必要になり依存関係の管理が複雑化。

---

### Option D: 時刻順キューの実装選択肢（DD9 固有の検討）

DD9 絶対時刻方式により、CueQueue の内部データ構造の選択が重要になる。

#### D-1: BinaryHeap\<Reverse\<TimedCue\>\> （最小ヒープ）

```rust
struct CueQueue {
    queue: BinaryHeap<Reverse<TimedCue>>,
    // ...
}
```

**特性**:
- push: O(log n)
- peek: O(1)
- pop: O(log n)
- start_time でソート（Ord trait 実装で最小ヒープ化）

**トレードオフ**:
- ✅ 優先度キューの標準実装
- ✅ 大量エントリでもパフォーマンス安定
- ❌ NFR-1 AC1 の「O(1) 償却」要件を満たせない
- ❌ TimedCue に Ord trait 実装が必要（CueCommand に PartialOrd 要求）

#### D-2: Vec\<TimedCue\> ソート済み

```rust
struct CueQueue {
    queue: Vec<TimedCue>,
    // 常に start_time 昇順を維持
}
```

**特性**:
- push: O(n) （二分探索 + insert）
- peek: O(1) （先頭参照）
- pop: O(1) （先頭除去後、shift 不要なら）or O(n) （Vec::remove(0) の場合）

**トレードオフ**:
- ✅ peek/pop が O(1)（逆順走査なら）
- ✅ キャッシュフレンドリー（連続メモリ）
- ❌ push が O(n)
- ❌ 実用上のキュー長が短い（数十〜数百）場合のみ有利

#### D-3: VecDeque\<TimedCue\> ソート済み（妥協案）

```rust
struct CueQueue {
    queue: VecDeque<TimedCue>,
    // 常に start_time 昇順を維持
}
```

**特性**:
- push: O(n) （線形探索 + insert）
- peek: O(1) （front 参照）
- pop: O(1) （pop_front）

**トレードオフ**:
- ✅ pop_front が O(1)
- ✅ VecDeque の既存利用実績（pointer モジュール）
- ✅ 両端アクセスが効率的
- ❌ push が O(n)
- ❌ ランダムアクセスが Vec より遅い

#### 推奨: D-1 (BinaryHeap) + NFR-1 AC1 の柔軟化

**根拠**:
- TypewriterTimeline は時刻順 Vec で実績あり（Stage 2 変換時に全体ソート済み）
- CueQueue は逐次追加が本質なので、BinaryHeap の O(log n) push が自然
- 実用上のキュー長は短い（数十〜数百）ため、O(log n) のオーバーヘッドは無視できる
- NFR-1 AC1 を「O(1) 償却」→「効率的な追加・消費（O(log n) 許容）」に緩和を設計フェーズで提案

---

## 5. 工数・リスク評価

### 工数: **M（4〜8日）**

**根拠**:
- データ構造定義（CueSheet, Cue, TimedCue, CueCommand, CueQueue）: **1.5〜2.5日**
  - DD9 により Wait/Instant バリアント削除で簡易化（-0.5日）
  - start_time フィールド + TimedCue 構造追加（+0.5日）
  - 時刻順キュー実装選択の検証（BinaryHeap vs Vec）（+0.5日）
- 配送メカニズム + 演者解決 + **マージ挿入**: **1.5〜2.5日**
  - PerformerKey 解決の設計判断を含む（DD1, DD2）
  - **DD9 により単純 append → start_time 順マージ挿入に複雑化**（+0.5日）
- 消費プロトコル + 状態管理: **1.5〜2日**
  - TypewriterTalk の update() パターンを汎化（時刻到達判定への変更）
  - WaitForInput のブロッキングセマンティクス（T5 議題の解決が前提）
- 拡張コマンド型設計: **0.5〜1日** — 設計判断が中心（DD3）、実装は少量
- dola 連携インターフェース: **0.5〜1日** — feature flag + 薄いインターフェース定義
- テスト: **1〜1.5日**
  - 配送・マージ挿入・時刻到達消費・境界条件の統合テスト
  - start_time が過去のコマンドの追いつき処理テスト（DD9 特有）

**DD9 による工数増**: 約 +1日（マージ挿入ロジック + 時刻到達消費の複雑化）

### リスク: **中〜高**

**リスク要因**:

| リスク                          | 影響度 | 発生確率 | 緩和策                                                              |
| ------------------------------- | ------ | -------- | ------------------------------------------------------------------- |
| 拡張コマンド型設計の収束遅延    | 高     | 中       | 初期は固定バリアント（BalloonCmd/AnimationCmd）で始め、後で汎化     |
| PerformerKey 解決が複雑化       | 中     | 低       | 最小限の HashMap レジストリから開始                                 |
| **時刻順マージ挿入の複雑性**    | 中     | 中       | **BinaryHeap 採用で自動ソート / 単体テストで検証**                  |
| **時刻到達消費の境界条件**      | 中     | 中       | **TypewriterTalk の while elapsed >= show_at パターンを厳密に移植** |
| TypewriterTalk との統合の複雑さ | 中     | 低       | 段階的移行（CueQueue → TypewriterTalk 変換層を中間に挿入）          |
| dola 統合の設計範囲膨張         | 高     | 中       | cue-system では薄いインターフェースのみ定義、実質的な統合は後続仕様 |
| 消費者不在での実装検証困難      | 中     | 高       | テスト用のモック消費者を用意、TypewriterTalk 変換でE2E検証          |
| **時刻順キュー選択の性能影響**  | 中     | 低       | **実用上のキュー長は短い（数十〜数百）、BinaryHeap でも問題なし**   |

**DD9 により新規追加されたリスク**:
- 時刻順マージ挿入の複雑性（中）
- 時刻到達消費の境界条件（中）
- 時刻順キュー選択の性能影響（低）

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option B（新規モジュール `ecs/cue/` の作成）+ Option D-1（BinaryHeap 時刻順キュー）** を推奨。

**根拠**:
- cue-system はウィジット横断的な基盤であり、`ecs/widget/` ではなく `ecs/cue/` として独立モジュールに配置すべき
- **DD9 絶対時刻方式により TypewriterToken との統合は困難** → 独立モジュールが必須
- **TypewriterTimeline が絶対時刻モデルの実証済み** → 設計パターンの確立と検証済み
- BinaryHeap は時刻順キューの標準実装であり、実装・テストが容易
- NFR-1 AC1 を「効率的な追加・消費（O(log n) 許容）」に緩和することで、実用上の問題なし

### 設計フェーズの重点事項

#### 優先度 High（MVP の核心判断）

1. **DD3（拡張コマンドの型構造）** ⭐⭐⭐
   - cue-system の拡張性を決定づける核心的判断
   - enum ネスト vs trait object vs generic の3方式を具体コード例で比較
   - 消費者（balloon, animation）の使い勝手とコンパイル時安全性のバランスを評価
   - 影響範囲: Req 7 全体、後続消費者の実装パターン

2. **DD1/DD2（ActorKey + 解決メカニズム）** ⭐⭐
   - CueSheet 配送の具体的な実現方式
   - 推奨: DD1-b (NewType), DD2-a (HashMap レジストリ) で開始
   - ECS World へのアクセス方法が影響する
   - 影響範囲: Req 1, 4

3. **時刻順キュー実装 + NFR-1 AC1 緩和** ⭐⭐
   - BinaryHeap\<Reverse\<TimedCue\>\> の採用確定
   - NFR-1 AC1 を「O(1) 償却」→「効率的な追加・消費（O(log n) 許容）」に要件緩和を提案
   - TimedCue に Ord trait 実装（start_time 優先、command は Eq のみ要求）
   - 影響範囲: Req 3, NFR-1

#### 優先度 Middle（設計整合性・将来拡張）

4. **✅ Q5 確定: WaitForInput のブロッキングスコープ = 演者ごとブロック**
   - 各 Cue が ActorKey を明示的に保持するため、WaitForInput は該当演者の CueQueue のみブロック
   - 他の演者のタイムラインは独立して進行
   - ECS の「エンティティ独立の原則」に合致
   - 影響範囲: Req 5 AC3

5. **T6 議題解決: CueSheet タイムライン管理の主体** ⭐
   - 各 CueQueue が独立管理 vs ルートに CueTimeline 配置 vs タイムラインエンティティ分離
   - 推奨: 各 CueQueue が経過時刻を独立管理（シンプル、DD9 と親和性高い）
   - ただし CueSheet 全体の同期が不要な場合のみ有効
   - 影響範囲: Req 5 AC4, Req 6, T5 との相互依存

6. **DD6（TypewriterToken との関係）** ⭐
   - 推奨: DD6-b（共存 + From 変換）で段階移行
   - CueCommand の Wait バリアント削除により、From\<TypewriterToken\> for CueCommand は実装不可
   - 逆方向の From\<CueCommand\> for TypewriterToken は可能（WaitForInput/Clear/StyleChange を Wait に変換）
   - 影響範囲: Req 2, 後方互換

7. **DD7（CueSheet 投入 API）** ⭐
   - 推奨: DD7-c（両方）— `PendingCueSheet` コンポーネント + `dispatch_cue_sheet` 関数
   - ファサードパターン原則（D1）との整合のため、コンポーネント経由を主要 API とする
   - 関数呼び出しはテスト用・直接呼び出し用に提供
   - 影響範囲: Req 4

#### 優先度 Low（後続仕様で詳細化）

8. **DD8（dola 統合の粒度）**
   - 推奨: DD8-a（インターフェース定義のみ）
   - cue-system では `#[cfg(feature = "dola")]` で薄いインターフェースのみ定義
   - 実質的な統合（DolaBridgeResource との連携）は balloon03-content / animation-system で実装
   - 影響範囲: Req 6 AC6-7

9. **DD5（モジュール配置）**
   - 確定: `ecs/cue/`
   - 横断的基盤なので widget の外

10. **DD4（消費プロトコルの提供形態）**
    - 推奨: DD4-a（ドキュメント仕様のみ）+ DD4-c（ヘルパー関数群）
    - `CueConsumer` trait は over-engineering の可能性
    - ドキュメントで消費パターンを明示 + `pop_ready(current_time)` 等のヘルパー API で十分

### 持ち越し調査事項

1. **TimedCue のメモリレイアウト検証**
   - `size_of::<TimedCue>() <= 64` を assert（NFR-1 AC4）
   - CueCommand の各バリアントサイズを確認
   - Extension バリアントが Box\<dyn Trait\> の場合は fat pointer 16バイト

2. **BinaryHeap の Ord 実装**
   - TimedCue に `#[derive(PartialEq, Eq, PartialOrd, Ord)]` を付与
   - start_time 優先でソート、command は Eq のみ実装
   - Reverse\<TimedCue\> で最小ヒープ化

3. **bevy_ecs 0.18 での Changed\<CueQueue\> 検出タイミング**
   - Mut\<T\> 経由の場合に内容変更なしでも反応するか（C7 制約の確認）
   - 空キュー走査回避の設計に影響
   - 対策案: Changed フィルタを使わず、With\<CueQueue\> のみで全 CueQueue を走査

4. **pasta DSL の CueSheet 出力想定フォーマット**
   - 外部リポジトリとのインターフェース契約
   - DD9 絶対時刻方式により、pasta のコンパイル時に start_time を計算する必要がある
   - pasta 側の実装方針確認が必要

### TypewriterToken との移行戦略

cue-system の完成後、TypewriterToken / TypewriterTalk は以下の移行パスを取る:

#### 推奨パス: 段階的共存 （DD6-b 採用時）

1. **Phase 1: 共存期** — cue-system 実装完了直後
   - TypewriterToken / TypewriterTalk は変更なし
   - CueCommand → TypewriterToken 変換層を実装（`From<CueCommand> for TypewriterToken` — ただし WaitForInput/Clear/StyleChange は Wait(0.0) に変換）
   - Typewriter は従来通り TypewriterToken を消費

2. **Phase 2: 試験導入** — balloon03-content 実装時
   - balloon が CueQueue を直接消費する新システムを構築
   - Typewriter は引き続き TypewriterToken を使用（並行稼働）
   - 実装・テストで CueQueue の有効性を検証

3. **Phase 3: 統合** — balloon03-content 安定後
   - Typewriter の内部実装を CueQueue ベースに移行（外部 API は維持）
   - TypewriterToken は外部互換性のためのファサードとして残存
   - TypewriterTalk の Stage 2 変換は内部実装として保持

**別パス: 完全置換** （DD6-a 採用時、非推奨）

- CueCommand が TypewriterToken を完全置換
- Typewriter を廃止し、balloon03-content で新 Typewriter を実装
- 既存の examples/ が全て破壊される
- リスクが高く、段階移行の利点がない

### Open Discussion Topics（未決事項）への対応

#### T2: スタイル変更コマンドの用語選定

**提案**: `style_key` → `context_key` に変更

**根拠**:
- 「スタイル」は CSS 的な視覚属性を連想させる（色、フォント等）
- cue-system の意図は「演者の演出コンテキスト切替」（感情値、モード、シーン等を包含）
- balloon では「感情値」、animation では「ポーズセット」「シーン」等、消費者により解釈が異なる
- `context_key: String` が最も汎用的で、各消費者が自由に解釈できる

**代替案**: `mode_key` （直感的だが範囲が狭い）、`preset_key` （事前定義を強調）

#### T7: CueSheet の中断・キャンセルセマンティクス

**提案**: `CueQueue::clear()` + 新 CueSheet 配送の2段階操作を標準パターンとする

**根拠**:
- さくらスクリプトの暗黙的 clear + replace は、ECS の明示性原則に反する
- 消費者が明示的に `queue.clear()` を呼び出すことで、意図が明確になる
- 配送 API は追記専用とし、clear はアプリケーション層の責務

**代替案**: CueSheet にメタデータ `replace_mode: bool` を追加（複雑化、非推奨）

#### T8: 動的生成シナリオへの対応

**提案**: 本仕様のスコープ外 — pasta DSL 層の責務

**根拠**:
- DD9 絶対時刻方式は事前に start_time が確定するシナリオに最適化
- LLM リアルタイムストリーミング等は pasta が都度 start_time を計算し、ミニ CueSheet を逐次投入すればよい
- cue-system 側の追加設計は不要

**将来拡張**: pasta が「前回末尾の start_time」を記憶し、相対時刻→絶対時刻変換を行う補助 API を提供する選択肢はある

---

## 7. Version History & Session Context

### Version History

| Version | Date       | Changes                                                                         |
| ------- | ---------- | ------------------------------------------------------------------------------- |
| 1.0     | 2026-02-26 | 初版生成（FIFO モデル前提、DD9 未決定）                                         |
| 2.0     | 2026-02-27 | DD9 絶対時刻キーフレーム方式適用。全セクション再評価。13領域網羅的コードベース調査反映 |

### 2.0 での主要変更点

#### 新規発見事項の反映

1. **TypewriterTimeline が DD9 の実証例**
   - Stage 2 IR の TimelineItem が show_at, start_at, fire_at で絶対時刻保持
   - TypewriterTalk::update() の while ループが elapsed >= show_at パターン
   - cue-system は Stage 1 レベルから絶対時刻で設計することで Stage 2 変換を不要化

2. **VecDeque は ECS Component として未使用**
   - 唯一の使用箇所は pointer/types.rs の thread_local リングバッファ
   - CueQueue が VecDeque を採用しても前例問題は存在しない

3. **FrameTime と DolaRuntime が同じ f64 秒時間軸**
   - FrameTime: elapsed_secs() — 起動時からの絶対秒数
   - DolaRuntime: update(current_time: f64) — 同じ時間軸
   - DD9 により dola 統合が自然に実現可能

4. **Changed<T> の gotcha**
   - Mut<T> 取得は内容変更なしでも Changed フラグを立てる
   - 空 CueQueue でも &mut CueQueue アクセスで Changed 発火
   - 対策: Changed フィルタを使わない / 追加時だけ Mut 取得

#### DD9 による設計変更

1. **データ構造**
   - Cue { performer, command, start_time } — start_time フィールド追加
   - TimedCue { start_time, command } — CueQueue エントリ型
   - Wait/Instant バリアント **削除** → 6+バリアント CueCommand

2. **CueQueue 内部構造**
   - FIFO (VecDeque) → 時刻順キュー (BinaryHeap or Vec)
   - O(1) append → O(log n) push（NFR-1 AC1 緩和提案）

3. **配送メカニズム**
   - 単純 append → start_time 順マージ挿入

4. **消費プロトコル**
   - FIFO 先頭消費 → 時刻到達消費 (while peek().start_time <= current_time)
   - Instant モード処理 **削除**（同時刻指定で代替）

5. **工数・リスク**
   - 工数: M (3〜7日) → M (4〜8日)（+1日、マージ挿入の複雑化）
   - リスク: 中 → 中〜高（時刻順マージ挿入・時刻到達消費の境界条件）

---

*分析完了。v2.0 は DD9 絶対時刻キーフレーム方式を全面適用し、13領域の網羅的コードベース調査結果を反映した決定版。TypewriterTimeline が DD9 の実証例として存在することを確認。工数 M (4〜8日)、リスク 中〜高。8つの残設計判断（DD1〜DD8）を設計フェーズで解決。*

