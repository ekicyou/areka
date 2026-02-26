# Gap Analysis: wintf-P0-cue-system

| 項目             | 内容                                            |
| ---------------- | ----------------------------------------------- |
| **対象仕様**     | wintf-P0-cue-system（演出キュー配送基盤）       |
| **分析日**       | 2026-02-26                                      |
| **Requirements** | v1.0（8要件 / 47受入基準 + 3NFR / 9受入基準）   |
| **分析種別**     | グリーンフィールド新規 + 既存パターン統合       |
| **分析範囲**     | crates/wintf/src/ecs/, crates/dola/src/runtime/ |

---

## 1. 現状調査サマリ

### 1.1 既存アセット

| アセット                                  | パス                                    | 関連性                                                                    |
| ----------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------- |
| `TypewriterToken` (Stage 1 IR)            | `ecs/widget/text/typewriter_ir.rs`      | cue-system の **先行実装**。Text, Wait, FireEvent の3バリアント enum      |
| `TypewriterTalk`                          | `ecs/widget/text/typewriter.rs`         | CueQueue の特殊化版（丸ごと差し替えモデル、FIFO ではない）                |
| `TypewriterState`                         | `ecs/widget/text/typewriter.rs`         | 消費ステート管理の先行実装（Playing/Paused/Completed）                    |
| `TypewriterTimeline` (Stage 2 IR)         | `ecs/widget/text/typewriter_ir.rs`      | glyphレベルタイムライン。消費プロトコルの参考実装                         |
| `TypewriterEvent` / `TypewriterEventKind` | `ecs/widget/text/typewriter_ir.rs`      | イベント通知パターン（SparseSet + Changed\<T\>）                          |
| `Typewriter` コンポーネント               | `ecs/widget/text/typewriter.rs`         | on_add フックで Visual + 空 TypewriterTalk を自動挿入するパターン         |
| `update_typewriters` システム             | `ecs/widget/text/typewriter_draw.rs`    | フレーム単位のタイムライン進行 + FireEvent 処理。消費プロトコルの参考実装 |
| `DolaRuntime` ファサード                  | `crates/dola/src/runtime/facade.rs`     | タイミングオーケストレーション。subscribe/load_document/start/update API  |
| `EvaluatedValue` / `UpdateResult`         | `crates/dola/src/runtime/types.rs`      | dola→ECS 差分配信の出力型                                                 |
| `Messages<T>` (Drag系)                    | `ecs/world/mod.rs`                      | bevy_ecs メッセージキュー。init_resource + FrameFinalize 更新パターン     |
| `CommandSender` (mpsc)                    | `ecs/widget/bitmap_source/task_pool.rs` | 非同期→ECS コマンド送信。CueSheet の非同期投入路の候補                    |
| `WintfTaskPool`                           | `ecs/widget/bitmap_source/task_pool.rs` | BoxedCommand のドレイン→World適用パターン                                 |
| `FrameTime` リソース                      | `ecs/graphics/`                         | フレーム時刻（f64秒）。ウェイト計測のタイムソース                         |
| スケジュール実行順序                      | `ecs/world/mod.rs`                      | Input → Update → Layout → Draw → Composition → FrameFinalize              |
| `DragConfig` / `OnDrag` 等                | `ecs/drag/mod.rs`                       | SparseSet コンポーネント + on_add パターンの参考                          |
| `Brush` / `BrushInherit`                  | `ecs/widget/brushes.rs`                 | コンポーネント自動挿入 + 解決パターンの参考                               |

### 1.2 確立済みパターン

- **on_add フックチェーン**: `Typewriter → Visual + TypewriterTalk` の自動挿入パターン
- **SparseSet ストレージ**: 動的変更が頻繁なコンポーネントの標準（TypewriterTalk, TypewriterEvent, DragConfig 等）
- **2段階 IR パターン**: Stage 1 IR（外部インターフェース）→ Stage 2 IR（内部処理用）の分離
- **フレーム単位の状態更新**: `update_typewriters` が FrameTime 基準でタイムライン消費 + イベント発火
- **Messages\<T\> ライフサイクル**: `init_resource::<Messages<T>>()` + FrameFinalize での `.update()` 呼び出し
- **mpsc ドレインパターン**: CommandSender → InputスケジュールでのWorld適用
- **Changed\<T\> リアクティブクエリ**: コンポーネント変更検出による遅延処理
- **`#[cfg(feature = "...")]` 条件コンパイル**: dola 統合のフィーチャーフラグパターン（設計済み、実装未着手）
- **DeferredWorld::commands()**: on_add フック内でのエンティティ操作

### 1.3 コーディング規約

- コンポーネント定義: `#[derive(Component)]` + `#[component(storage = "SparseSet", on_add = ..., on_remove = ...)]`
- フック関数シグネチャ: `fn on_xxx_add(mut world: DeferredWorld, hook: HookContext)`
- 自動挿入前に `world.get::<T>(entity).is_none()` で存在チェック
- ウィジットモジュール配置: `ecs/widget/{widget_name}/` サブディレクトリ
- ログレベル: `tracing::debug!`（状態変化）, `tracing::trace!`（高頻度処理）, `tracing::warn!`（回復可能エラー）
- テスト配置: `crates/wintf/tests/{module}/` 統合テスト + ファイル内 `#[cfg(test)]` ユニットテスト

---

## 2. 要件別アセットマップ

### Req 1: CueSheet — 構造化演出台本モデル

| AC  | 技術要素                 | 既存アセット                        | ギャップ                                                  |
| --- | ------------------------ | ----------------------------------- | --------------------------------------------------------- |
| AC1 | CueSheet データ構造      | —                                   | **Missing**: `CueSheet` 構造体（Vec\<Cue\> + メタデータ） |
| AC2 | PerformerKey 識別子      | —                                   | **Missing**: `PerformerKey` 型定義（文字列 or enum）      |
| AC3 | 挿入順序保持             | `Vec<TypewriterToken>` で実績あり   | ギャップなし（Vec で順序保持は自然）                      |
| AC4 | 複数演者の混在記述       | —                                   | **Missing**: `Cue` 構造体（PerformerKey + CueCommand）    |
| AC5 | 演者別フィルタリング API | —                                   | **Missing**: `filter_by_performer()` 等の API             |
| AC6 | Clone, Debug derive      | TypewriterToken は Debug + Clone 済 | ギャップなし（derive マクロ付与のみ）                     |

**評価**: CueSheet は完全新規のデータ構造。TypewriterToken の Vec パターンを拡張した構成で、複雑度は低い。PerformerKey の型設計（String vs enum vs Entity）が設計フェーズの論点。

---

### Req 2: CueCommand — 型安全な基盤コマンド体系

| AC   | 技術要素                   | 既存アセット                     | ギャップ                                                        |
| ---- | -------------------------- | -------------------------------- | --------------------------------------------------------------- |
| AC1  | 基盤コマンド enum          | `TypewriterToken`（3バリアント） | **Extend**: 3→8+ バリアントへの大幅拡張                         |
| AC2  | テキスト表示バリアント     | `TypewriterToken::Text(String)`  | ギャップなし（直接対応、型も同一）                              |
| AC3  | 時間ウェイトバリアント     | `TypewriterToken::Wait(f64)`     | ギャップなし（直接対応、型も同一）                              |
| AC4  | ユーザー入力待ちバリアント | —                                | **Missing**: `WaitForInput { timeout: Option<f64> }` バリアント |
| AC5  | 即時モード切替バリアント   | —                                | **Missing**: `Instant` バリアント                               |
| AC6  | コンテンツクリアバリアント | —                                | **Missing**: `Clear` バリアント                                 |
| AC7  | スタイル変更バリアント     | —                                | **Missing**: `StyleChange { key: String }` バリアント           |
| AC8  | 拡張バリアント             | `TypewriterToken::FireEvent`     | **Redesign**: FireEvent を汎用拡張機構に再設計                  |
| AC9  | 型安全パラメータ           | TypewriterToken で実績あり       | ギャップなし（Rust enum の自然な型付け）                        |
| AC10 | Clone, Debug derive        | TypewriterToken は Debug + Clone | ギャップなし                                                    |

**評価**: TypewriterToken が3バリアントの先行実装として存在。cue-system はこれを8+バリアントに拡張する新規 enum として定義。TypewriterToken との後方互換は `From` トレイト変換で対応可能。

---

### Req 3: CueQueue — エンティティキューコンポーネント

| AC  | 技術要素                 | 既存アセット                                 | ギャップ                                                  |
| --- | ------------------------ | -------------------------------------------- | --------------------------------------------------------- |
| AC1 | ECS コンポーネント       | `TypewriterTalk`（SparseSet コンポーネント） | **Missing**: `CueQueue` コンポーネント（VecDeque ベース） |
| AC2 | FIFO セマンティクス      | `VecDeque` が pointer/types.rs で利用実績    | ギャップなし（VecDeque で O(1) push_back/pop_front）      |
| AC3 | append API               | `TypewriterTalk::new()` は丸ごと差し替えのみ | **Missing**: `push_back()` / `extend()` API               |
| AC4 | pop_front API            | —                                            | **Missing**: `pop_front()` API                            |
| AC5 | peek API                 | —                                            | **Missing**: `front()` / `peek()` API                     |
| AC6 | is_empty / len API       | —                                            | **Missing**: キュー状態問い合わせ API                     |
| AC7 | clear API                | —                                            | **Missing**: `clear()` API                                |
| AC8 | エンティティごとの独立性 | TypewriterTalk がエンティティ独立性を実証    | ギャップなし（ECS コンポーネントの本質的な特性）          |

**評価**: `VecDeque<CueCommand>` をラップした新規コンポーネント。TypewriterTalk の「丸ごと差し替え」モデルから「append 可能キュー」への根本的な設計転換。VecDeque は pointer モジュールで利用実績あり。実装は薄いラッパーで複雑度は低い。

---

### Req 4: CueSheet 配送メカニズム

| AC  | 技術要素                           | 既存アセット                      | ギャップ                                                       |
| --- | ---------------------------------- | --------------------------------- | -------------------------------------------------------------- |
| AC1 | PerformerKey→CueQueue 分配         | —                                 | **Missing**: 配送関数 / 配送システム                           |
| AC2 | 演者レジストリ / 解決関数          | —                                 | **Missing**: PerformerKey → Entity 解決メカニズム              |
| AC3 | 出現順の保持                       | Vec の順序保持パターン            | ギャップなし（フィルタリング後も順序保持）                     |
| AC4 | CueQueue への末尾追加              | —                                 | **Missing**: 配送→CueQueue.extend() の統合ロジック             |
| AC5 | 未解決 PerformerKey のハンドリング | `tracing::warn!` パターン確立済み | ギャップなし（ログパターン流用）                               |
| AC6 | 逐次投入（既存キューへの追加）     | TypewriterTalk は丸ごと差し替え   | **Redesign**: 追加投入モデルは CueQueue の append で自然に実現 |

**評価**: 完全新規。TypewriterTalk の `new()` による丸ごと差し替えモデルとは根本的に異なる「逐次投入」モデル。PerformerKey の解決メカニズム（レジストリ方式 vs クエリ方式 vs マーカーコンポーネント方式）が設計フェーズの主要論点。

---

### Req 5: キュー消費プロトコル

| AC  | 技術要素             | 既存アセット                                    | ギャップ                                                             |
| --- | -------------------- | ----------------------------------------------- | -------------------------------------------------------------------- |
| AC1 | フレーム単位の消費   | `update_typewriters` で実績あり                 | **Missing**: CueQueue 向けの汎用消費プロトコル定義                   |
| AC2 | Wait のブロッキング  | `TypewriterTalk::update()` で Wait 処理実装済み | **Adapt**: TypewriterTalk の Wait 処理パターンを CueQueue 向けに汎化 |
| AC3 | 入力待ちブロッキング | —                                               | **Missing**: WaitForInput の消費ブロッキングセマンティクス           |
| AC4 | 即時モードの処理     | —                                               | **Missing**: Instant モードでの Wait 無視ロジック                    |
| AC5 | バッチ消費パターン   | TypewriterTalk は1フレームで複数 Glyph を消費   | **Adapt**: 非ブロッキングコマンドの連続消費パターンを汎化            |
| AC6 | 消費ステート管理     | `TypewriterState`（Playing/Paused/Completed）   | **Extend**: TypewriterState を汎用化 + WaitForInput 状態追加         |
| AC7 | 消費完了状態         | `TypewriterState::Completed`                    | **Adapt**: 既存パターンの流用                                        |

**評価**: TypewriterTalk の `update()` メソッドが消費プロトコルの概念実証（POC）として機能。ただし TypewriterTalk は Stage 2 IR（グリフ単位）に変換後に消費するのに対し、CueQueue は Stage 1 IR レベルで直接消費するため、レイヤーが異なる。WaitForInput と Instant モードは完全新規。消費プロトコルを「仕様として文書化」するか「コード実装として提供」するかが設計フェーズの論点。

---

### Req 6: タイミング制御と dola 統合

| AC  | 技術要素                     | 既存アセット                                      | ギャップ                                                               |
| --- | ---------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------- |
| AC1 | システム時間ベースの経過計測 | `FrameTime.elapsed_secs()` で実績あり             | ギャップなし（FrameTime リソース利用）                                 |
| AC2 | pause API                    | `TypewriterTalk::pause()` で実績あり              | **Adapt**: CueQueue 向け pause API                                     |
| AC3 | resume API                   | `TypewriterTalk::resume()` で実績あり             | **Adapt**: CueQueue 向け resume API                                    |
| AC4 | skip API                     | `TypewriterTalk::skip()` で実績あり               | **Adapt**: CueQueue 全コマンド即時消費                                 |
| AC5 | 消費速度変更                 | `Typewriter.default_char_wait` がグリフ間ウェイト | **Missing**: 速度倍率フィールド                                        |
| AC6 | dola タイムライン連携        | `DolaRuntime::update(time)` が確立済み API        | **Missing**: CueQueue 消費と dola タイムラインの連携システム           |
| AC7 | dola 変数公開                | `DolaRuntime::subscribe()` が確立済み API         | **Missing**: CueQueue 消費進行を dola 変数として公開するバインディング |

**評価**: TypewriterTalk が pause/resume/skip の概念実証として存在。dola 統合は `#[cfg(feature = "dola")]` で条件コンパイルする方針が確立済み（ただし wintf Cargo.toml に dola 依存はまだ未追加）。DolaBridgeResource（balloon-system 設計書で定義済み、コード未実装）との関係整理が設計フェーズの主要論点。

---

### Req 7: コマンド型安全拡張メカニズム

| AC  | 技術要素                      | 既存アセット                         | ギャップ                                                               |
| --- | ----------------------------- | ------------------------------------ | ---------------------------------------------------------------------- |
| AC1 | 拡張バリアントによる格納      | `TypewriterToken::FireEvent`         | **Redesign**: FireEvent は特定用途。汎用拡張バリアントへの再設計が必要 |
| AC2 | ドメイン固有コマンドの取出し  | —                                    | **Missing**: 拡張コマンドのパターンマッチ + skip/passthrough パターン  |
| AC3 | Debug トレイト要求            | TypewriterToken は Debug derive 済み | ギャップなし（derive マクロ + トレイト境界）                           |
| AC4 | enum ベースの static dispatch | —                                    | **Design Decision**: enum ネスト vs trait object vs generic の選択     |
| AC5 | ドキュメント / 使用例         | —                                    | **Missing**: バルーン向け・アニメーション向けの拡張例ドキュメント      |

**評価**: 拡張メカニズムの設計は cue-system の核心的な設計判断。TypewriterToken::FireEvent は Entity + EventKind という特定のペイロードを持つが、汎用拡張は任意のドメインコマンドを格納する必要がある。enum ネスト方式（`Extension(BalloonCommand)` / `Extension(AnimationCommand)`）が有力候補だが、消費者が増えた場合の開閉原則への影響が設計フェーズの論点。

---

### Req 8: エラーハンドリングと堅牢性

| AC  | 技術要素                   | 既存アセット                         | ギャップ                                                  |
| --- | -------------------------- | ------------------------------------ | --------------------------------------------------------- |
| AC1 | キャパシティ上限チェック   | —                                    | **Missing**: オプショナルなキャパシティ設定 + warn ログ   |
| AC2 | 未知コマンドのスキップ     | —                                    | **Missing**: 消費者側の unknown コマンドハンドリング      |
| AC3 | despawn 耐性               | TypewriterTalk の on_remove フック   | **Adapt**: CueQueue の on_remove フック（クリーンアップ） |
| AC4 | 空 CueSheet のハンドリング | —                                    | ギャップなし（空 Vec での no-op は自然）                  |
| AC5 | 部分的失敗の許容           | `tracing::warn!` + continue パターン | ギャップなし（既存パターン流用）                          |

**評価**: エラーハンドリングの大部分は既存パターンの流用。キャパシティ上限はオプショナル設計（デフォルト無制限、意図的に設定できるオプション）。

---

### NFR-1: パフォーマンス

| AC  | 技術要素               | 既存アセット                           | ギャップ                                                               |
| --- | ---------------------- | -------------------------------------- | ---------------------------------------------------------------------- |
| AC1 | O(1) 追加・消費        | `VecDeque` で保証                      | ギャップなし                                                           |
| AC2 | 空キュー時の走査最小化 | bevy_ecs クエリフィルタ                | **Design Decision**: With\<CueQueue\> + Added/Changed でフィルタするか |
| AC3 | メモリサイズ最適化     | TypewriterToken は32バイト未満（推定） | **Verify**: CueCommand のサイズをコンパイル時に assert する            |

**評価**: VecDeque の O(1) 特性により AC1 は自動的に満たされる。空キュー走査回避は bevy_ecs のクエリフィルタリングで対応可能。

---

### NFR-2: デバッグ容易性

| AC  | 技術要素     | 既存アセット               | ギャップ                                   |
| --- | ------------ | -------------------------- | ------------------------------------------ |
| AC1 | Debug derive | 全既存コンポーネントで実績 | ギャップなし                               |
| AC2 | 配送ログ     | `tracing::debug!` パターン | **Missing**: dispatch_cue_sheet のログ出力 |
| AC3 | 消費ログ     | `tracing::trace!` パターン | **Missing**: キュー消費のトレースログ      |

**評価**: 全 AC が既存ログパターンの適用で対応可能。

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
| M1  | `CueSheet` 構造体（Vec\<Cue\> + メタデータ）                 | Req 1    | 低（データ構造のみ）                  |
| M2  | `Cue` 構造体（PerformerKey + CueCommand）                    | Req 1    | 低                                    |
| M3  | `PerformerKey` 型定義                                        | Req 1, 4 | 低〜中（型設計の判断が必要）          |
| M4  | `CueCommand` enum（8+バリアント）                            | Req 2    | 低（TypewriterToken の拡張）          |
| M5  | `CueQueue` コンポーネント（VecDeque ラッパー + API）         | Req 3    | 低（薄いラッパー）                    |
| M6  | 配送関数 / 配送システム（`dispatch_cue_sheet`）              | Req 4    | 中（PerformerKey 解決が必要）         |
| M7  | PerformerKey → Entity 解決メカニズム（レジストリ or クエリ） | Req 4    | 中（設計判断が必要）                  |
| M8  | WaitForInput のブロッキングセマンティクス                    | Req 5    | 中（外部入力との連携設計）            |
| M9  | Instant モードの消費ロジック                                 | Req 5    | 低（フラグベース）                    |
| M10 | 消費ステート enum（CueQueueState）                           | Req 5    | 低（TypewriterState の拡張）          |
| M11 | 消費速度倍率フィールド                                       | Req 6    | 低                                    |
| M12 | dola 連携インターフェース（`#[cfg(feature = "dola")]`）      | Req 6    | 中〜高（DolaBridgeResource 設計依存） |
| M13 | 拡張コマンドの型設計                                         | Req 7    | 中〜高（核心的設計判断）              |
| M14 | 拡張コマンドの消費パターン文書                               | Req 7    | 低（ドキュメントのみ）                |
| M15 | キャパシティ上限チェック（オプショナル）                     | Req 8    | 低                                    |
| M16 | モジュール構造（`ecs/cue/` or `ecs/widget/cue/`）            | 全体     | 低（スキャフォールド）                |

### Adapt（既存パターンの汎化・適用）

| #   | アイテム                           | 元パターン                      | 関連要件 |
| --- | ---------------------------------- | ------------------------------- | -------- |
| A1  | CueQueue の pause/resume/skip API  | TypewriterTalk の同名メソッド   | Req 6    |
| A2  | フレーム単位消費プロトコル         | update_typewriters の走査ループ | Req 5    |
| A3  | on_remove フックでのクリーンアップ | on_typewriter_talk_remove       | Req 8    |
| A4  | CueCommand に Clone, Debug derive  | TypewriterToken の derive       | Req 2    |
| A5  | SparseSet コンポーネント宣言       | TypewriterTalk の #[component]  | Req 3    |

### Redesign（根本的な再設計）

| #   | アイテム                                        | 既存                                | 理由                                                               |
| --- | ----------------------------------------------- | ----------------------------------- | ------------------------------------------------------------------ |
| R1  | 差し替え → append モデルへの転換                | TypewriterTalk::new()（丸ごと差替） | CueQueue は逐次投入が本質であり、差し替えモデルは互換性がない      |
| R2  | FireEvent → 汎用拡張バリアント                  | TypewriterToken::FireEvent          | 特定用途の2フィールド variant → 任意ドメインコマンドの格納機構へ   |
| R3  | Stage 1 IR 消費 → Stage 2 IR 変換なしの直接消費 | TypewriterTimeline（Stage 2 変換）  | CueQueue は Stage 1 レベルで消費。Stage 2 変換は各消費者の内部処理 |

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
| DD9 | **タイミングモデル**: 相対時刻 vs 絶対時刻キーフレーム方式              | (a) Wait コマンドによる相対時刻（FIFO 逐次消費）, (b) Cue に start_time フィールドを付与し絶対時刻管理（並行実行可能）       | Req 1,2,5,6 全体 |

### Constraint（既存アーキテクチャ制約）

| #   | 制約                                                                                                                                                                 | 影響              |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- |
| C1  | wintf Cargo.toml に dola 依存が未追加（`dola = { path = "../dola", optional = true }` が必要）                                                                       | Req 6             |
| C2  | DolaBridgeResource は balloon-system 設計書で定義済みだがコード未実装                                                                                                | Req 6             |
| C3  | TypewriterToken / TypewriterTalk は typewriter 専用で、cue-system とは独立して存在し続ける（DD6 の決定に依存。DD6-a 採用時は CueCommand が完全置換し本制約は無効化） | Req 2（後方互換） |
| C4  | on_add フック内で World にアクセスできる範囲が限定的（DeferredWorld の制約）                                                                                         | Req 4（配送設計） |
| C5  | bevy_ecs 0.18 の Component derive は `Clone` を求めない（手動実装は可能だが derive が自然）                                                                          | Req 3             |
| C6  | スケジュール実行順は Input → Update → ... が固定。CueQueue 消費は Update スケジュールが適切                                                                          | Req 5             |

---

## 4. 実装アプローチ検討

### Option A: TypewriterToken の拡張 + TypewriterTalk の改修

**適用可能性**: 低

TypewriterToken に WaitForInput, Instant, Clear, StyleChange, Extension バリアントを追加し、TypewriterTalk を VecDeque ベースに改修する方法。

**トレードオフ**:
- ✅ ファイル数が増えない
- ✅ 既存の TypewriterTalk テストが流用可能
- ❌ Typewriter 専用の概念（グリフ、TextLayout）と汎用キューの概念が混在
- ❌ TypewriterLayoutCache（Stage 2 IR 変換）との整合性が崩れる
- ❌ balloon/animation 消費者がTypewriter モジュールに依存する不自然な構造
- ❌ 単一責任の原則に違反

---

### Option B: 新規モジュール `ecs/cue/` の作成 ⭐

**適用可能性**: 高（推奨候補）

`ecs/cue/` に CueSheet, CueCommand, CueQueue, 配送システムを独立モジュールとして定義。TypewriterToken / TypewriterTalk はそのまま存続させ、将来的に CueQueue → TypewriterTalk への変換層を設ける。

**ディレクトリ構造**:
```
ecs/
├── cue/
│   ├── mod.rs           ← CueSheet, Cue, PerformerKey, re-exports
│   ├── command.rs       ← CueCommand enum + 拡張バリアント型定義
│   ├── queue.rs         ← CueQueue コンポーネント（VecDeque ラッパー）
│   ├── dispatch.rs      ← dispatch_cue_sheet + 演者解決
│   └── consumer.rs      ← 消費プロトコルヘルパー / CueQueueState
├── widget/
│   └── text/
│       └── typewriter*.rs  ← 変更なし（存続）
```

**統合ポイント**:
- `ecs/mod.rs` に `pub mod cue;` を追加
- CueQueue 消費は各消費者の Update システムで実行
- TypewriterTalk との関係は `From<CueCommand> for TypewriterToken` 変換で段階移行
- dola 統合は `cue/dola_bridge.rs` を `#[cfg(feature = "dola")]` で追加

**トレードオフ**:
- ✅ 明確な責務分離（cue は cross-cutting concern として widget の外に配置）
- ✅ TypewriterToken / TypewriterTalk への影響ゼロ
- ✅ 後続消費者（balloon, animation）が自然に依存できる構造
- ✅ テスト容易（cue/ 単体でテスト可能、COM 依存なし）
- ✅ dola 統合を feature flag で隔離可能
- ❌ ファイル数が増加（4〜5ファイル）
- ❌ TypewriterTalk への変換層が追加のオーバーヘッド

---

### Option C: CueCommand を wintf 外部のクレートとして分離

**適用可能性**: 低〜中

CueCommand / CueSheet の型定義を `crates/cue/` として独立クレート化し、wintf と areka の両方から参照する方法。

**トレードオフ**:
- ✅ pasta DSL クレートから直接参照可能
- ✅ wintf への依存なしで CueSheet 構築が可能
- ❌ CueQueue（ECS コンポーネント）は bevy_ecs 依存のため wintf 内に必要
- ❌ クレート分割の早期最適化（消費者が未実装の段階では時期尚早）
- ❌ Cargo workspace の複雑度増加

---

## 5. 工数・リスク評価

### 工数: **M（3〜7日）**

**根拠**:
- データ構造定義（CueSheet, CueCommand, CueQueue）: **1〜2日** — TypewriterToken パターンの拡張で既存テンプレートが豊富
- 配送メカニズム + 演者解決: **1〜2日** — PerformerKey 解決の設計判断を含む
- 消費プロトコル + 状態管理: **1〜2日** — TypewriterTalk の update() パターンを汎化
- 拡張コマンド型設計: **0.5〜1日** — 設計判断が中心、実装は少量
- dola 連携インターフェース: **0.5〜1日** — feature flag + 薄いインターフェース定義
- テスト: **1日** — 配送・消費・境界条件の統合テスト

### リスク: **中**

**リスク要因**:

| リスク                          | 影響度 | 発生確率 | 緩和策                                                              |
| ------------------------------- | ------ | -------- | ------------------------------------------------------------------- |
| 拡張コマンド型設計の収束遅延    | 高     | 中       | 初期は固定バリアント（BalloonCmd/AnimationCmd）で始め、後で汎化     |
| PerformerKey 解決が複雑化       | 中     | 低       | 最小限の HashMap レジストリから開始                                 |
| TypewriterTalk との統合の複雑さ | 中     | 低       | 段階的移行（CueQueue → TypewriterTalk 変換層を中間に挿入）          |
| dola 統合の設計範囲膨張         | 高     | 中       | cue-system では薄いインターフェースのみ定義、実質的な統合は後続仕様 |
| 消費者不在での実装検証困難      | 中     | 高       | テスト用のモック消費者を用意、TypewriterTalk 変換でE2E検証          |

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option B（新規モジュール `ecs/cue/` の作成）** を推奨。cue-system はウィジット横断的な基盤であり、`ecs/widget/` ではなく `ecs/cue/` として独立モジュールに配置すべき。

### 設計フェーズの重点事項

1. **DD3（拡張コマンドの型構造）** — cue-system の拡張性を決定づける核心的判断。enum ネスト vs trait object vs generic の3方式を具体コード例で比較し、消費者（balloon, animation）の使い勝手とコンパイル時安全性のバランスを評価する
2. **DD1/DD2（PerformerKey + 解決メカニズム）** — CueSheet 配送の具体的な実現方式。ECS World へのアクセス方法が影響する
3. **DD6（TypewriterToken との関係）** — 後方互換と段階移行のパス設計
4. **DD7（CueSheet 投入 API）** — ファサードパターン原則（D1）との整合
5. **DD8（dola 統合の粒度）** — cue-system 内での最小限の定義と後続仕様での実装の境界

### 持ち越し調査事項

- `VecDeque<CueCommand>` のメモリレイアウト検証（CueCommand の `size_of` が 64バイト以下か）
- bevy_ecs 0.18 での `Changed<CueQueue>` 検出タイミング（Mut\<T\> 経由の場合に内容変更なしでも反応するか）
- pasta DSL の CueSheet 出力想定フォーマット（外部リポジトリとのインターフェース契約）

### TypewriterToken との移行戦略

cue-system の完成後、TypewriterToken / TypewriterTalk は以下のいずれかの移行パスを取る:

1. **共存パス**: CueQueue → TypewriterTalk 変換層（`From<Vec<CueCommand>> for TypewriterTalk`）を設け、段階的に消費者を CueQueue 直接消費に移行
2. **統合パス**: balloon03-content 実装時に TypewriterTalk を廃止し、CueQueue を直接消費する新 Typewriter システムに移行
3. 移行パスの最終判断は balloon03-content の設計フェーズで行う

---

*分析完了。cue-system は TypewriterToken/TypewriterTalk の先行実装から多くのパターンを継承できるが、「差し替え→append」「Stage 2 変換→Stage 1 直接消費」「特定用途→汎用拡張」の3つの根本的な再設計が必要。工数 M（3〜7日）、リスク中。9つの設計判断（DD1〜DD9）を設計フェーズで解決する。*

---

## 7. セッション継続情報（2026-02-26 レビューセッション）

### 実施済み作業

| # | 作業内容 | コミット |
|---|---------|----------|
| 1 | 要件 v1.0 生成 | `28222f0` |
| 2 | ギャップ分析 v1.0 生成 | `b4d0a67` |
| 3 | レビュー自明修正（F1: C3 但し書き追加、F2: Req 4 AC2 実装詳細削除） | `1a8c85b` |

### レビュー結果

#### 自明な修正（完了）
- **F1**: gap-analysis C3 に DD6 依存の但し書き追加
- **F2**: requirements Req 4 AC2 から実装詳細「（演者レジストリまたは解決関数）」を削除

#### 議題 T1/T3/T4 の統合結論（DD9 として追加）

**決定事項**: **絶対時刻キーフレーム方式**を採用

##### 設計方針

```rust
struct Cue {
    performer: PerformerKey,
    command: CueCommand,
    start_time: f64,  // CueSheet 開始からの絶対秒数
}
```

- CueSheet = 時系列イベントリスト（投入順 ≠ 実行順）
- 並行実行: 複数 Cue に同じ start_time を設定
- タイミング計算: pasta DSL のコンパイル時に相対時刻→絶対時刻変換
- 消費プロトコル: 現在時刻 ≥ start_time のコマンドを時系列順に消費

##### 解決される問題

| 旧議題 | 問題 | 解決 |
|--------|------|------|
| **T1** | Instant モードの解除方法が不明 | 複数コマンドに同じ start_time を設定するだけ。Instant バリアント不要 |
| **T3** | 消費速度変更の適用対象が曖昧 | start_time に倍率適用と明確 |
| **T4** | dola subscribe は dola→ECS、ECS→dola の公開方法不明 | CueSheet の start_time = dola の時間軸で統一可能 |

##### 影響を受ける要件（要書き換え）

- **Req 1**: AC 追加「各 Cue が絶対時刻 start_time を保持」「投入順≠実行順」
- **Req 2**: AC5 削除（Instant バリアント）、AC4 変更（WaitForInput のタイムアウト解釈）
- **Req 4**: AC 追加「配送時に start_time でソート挿入」
- **Req 5**: AC1 変更「先頭消費→時刻到達消費」、AC4 削除（即時モード）、AC5 変更「バッチ→同時刻並行」
- **Req 6**: AC5 明確化（速度 = start_time 倍率）、AC7 変更（dola 連携の統一時間軸）

#### 残議題

**T2: 感情値キーの用語問題**
- 現状: Req 2 AC7「スタイル変更バリアント（感情値キーを保持）」
- 指摘: 基盤コマンドがバルーン固有の語彙「感情値」を使用
- 論点: 横断的基盤にふさわしい汎用名（「スタイルキー」「モードキー」等）にすべきか？
- 状況: 未着手（次セッションで議論）

### 次セッションのタスク

1. **T2 議題の解決** — 感情値キーの用語について開発者と確認
2. **DD9 に基づく requirements.md 書き換え** — Req 1, 2, 4, 5, 6 を絶対時刻方式に転換
3. **gap-analysis.md 更新** — DD9 影響による Missing/Adapt/Redesign 項目の再評価
4. **設計フェーズ移行判断** — 全議題解決後、requirements 承認 → design 生成

### 技術メモ

- VecDeque 利用実績: `ecs/pointer/types.rs:210` で `VecDeque<PositionSample>` 確認済み
- DolaBridgeResource: balloon-system 設計書で定義済み、コード未実装（C2 制約）
- DolaRuntime API: `subscribe(var_name)` → 変数 ID、`update(time)` → UpdateResult（差分値）、絶対時刻ベースの時間軸
- さくらスクリプト `\_q`: 「以降即時表示」だが絶対時刻方式では不要（同時刻指定で代替）
