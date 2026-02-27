# Implementation Plan: wintf-P0-cue-system

| 項目               | 内容                                         |
| ------------------ | -------------------------------------------- |
| **Document Title** | wintf キューシステム（cue-system）実装タスク |
| **Version**        | 1.0                                          |
| **Date**           | 2026-02-27                                   |
| **Requirements**   | v2.3（9 Req + 3 NFR）                       |
| **Design**         | v2.1（14 DD）                                |
| **Status**         | 📋 Generated                                |

---

## Implementation Tasks

### Phase 1: Data Model Foundation

- [ ] 1. データモデル基盤の確立
- [ ] 1.1 (P) 演者識別型とコマンド列構造の実装
  - ActorKey ニュータイプ（String ラッパー）を定義し、型安全な演者識別を実現
  - `From<&str>` / `From<String>` トレイトで変換を提供
  - Cue 構造体（actor + start_time + command）を定義
  - CueSheet 構造体（Vec\<Cue\> ラッパー）を定義し、`new()` で start_time 昇順ソートを保証
  - `filter_by_actor()` / `actors()` / `is_empty()` / `len()` API を実装
  - Clone, Debug derive を適用
  - _Requirements: 1_

- [ ] 1.2 (P) 型安全コマンド体系の実装
  - CueCommand enum を 11 バリアントで定義（Text, Clear, Emote, Choice, WaitForChoice, WaitForClick, EntityRef, Custom, RouteAdd, RouteSwitch, RouteRemove）
  - Custom バリアントに `dola::DynamicValue` をパラメーター型として採用
  - `is_barrier()` / `is_routing_command()` 分類メソッドを実装
  - Clone, Debug derive を適用
  - _Requirements: 2, 7_

- [ ] 1.3 (P) ルーティングスロット識別子の実装
  - CueTarget enum（Shell, Balloon）を定義
  - PartialEq, Eq, Hash, Clone, Debug derive を適用
  - EntityKey enum（Actor, Spot, Balloon）を定義して名前空間統合
  - _Requirements: 4_

- [ ] 1.4 (P) 絶対時刻コマンドエントリーの実装
  - TimedCue 構造体（start_time: f64 + command: CueCommand）を定義
  - メモリーサイズ検証（`static_assert!(size_of::<TimedCue>() <= 64)`）を実装
  - Clone, Debug derive を適用
  - _Requirements: 3, NFR-1_

### Phase 2: Component Layer

- [ ] 2. コンポーネント層の構築
- [ ] 2.1 (P) キュー状態管理型の実装
  - CueQueueState enum（Playing, Paused, WaitingForClick, WaitingForChoice, Error, Completed）を定義
  - PartialEq, Clone, Debug derive を適用
  - _Requirements: 5_

- [ ] 2.2 (P) バリア関連型の実装
  - BarrierResponse enum（Skipped, Click, Choice, Timeout）を定義
  - BarrierKind enum（Choice, Click）を定義
  - BarrierState 内部型（first_valid + kind）を実装
  - PendingChoice 構造体（id + text）を実装
  - Clone, Debug derive を各型に適用
  - _Requirements: 2, 5_

- [ ] 2.3 演出指示キューコンポーネントの実装
  - CueQueue コンポーネント（`#[component(storage = "SparseSet")]`）を定義
  - 内部データ（降順 Vec\<TimedCue\>, state, playback_rate, capacity, pending_choices, cue_sheet_entity, barrier_state）を実装
  - `new()` / `with_capacity()` コンストラクターを実装
  - _Requirements: 3_

- [ ] 2.4 キュー追加 API の実装
  - `push_sorted()` — binary search + shift による降順ソート維持挿入を実装
  - `extend_sorted()` — 一括追加 + 再ソートを実装
  - capacity 超過時に `CueSystemError::CapacityExceeded` を返却
  - _Requirements: 3, 8_

- [ ] 2.5 時刻ベース消費プロトコルの実装
  - `pop_ready(current_time)` — 時刻到達済みコマンドを末尾から pop（O(1)）して返却
  - バリア中は空 Vec を返却
  - Choice コマンドは pending_choices に蓄積（返却しない）
  - WaitForChoice 到達時の先行 Choice 検証（0 件 → Error）
  - WaitForClick 到達時のブロック遷移
  - `peek()` — 先頭要素を参照のみ
  - _Requirements: 3, 5, 8_

- [ ] 2.6 バリア制御 API の実装
  - `resolve_click()` — WaitingForClick → Playing 遷移
  - `resolve_choice(choice_id)` — WaitingForChoice 解除 + 該当 id 返却
  - `check_timeout(current_time)` — タイムアウト判定 + BarrierResponse::Timeout 設定
  - `skip_barrier()` — 強制スキップ（barrier_state クリア）
  - `pending_barrier_kind()` — 現在のバリア種別を返却
  - _Requirements: 5, 9_

- [ ] 2.7 (P) キュー制御・照会 API の実装
  - `pause()` / `resume()` — state 切替
  - `clear()` — queue + pending_choices + barrier_state をクリア
  - `set_cue_sheet(entity)` / `cue_sheet_entity()` — 供給元 Tracker の参照管理
  - `state()` / `is_empty()` / `len()` / `pending_choices()` — 状態照会
  - _Requirements: 3, 5_

### Phase 3: System & Resource Layer

- [ ] 3. システム・リソース層の構築
- [ ] 3.1 (P) エラー型の定義
  - CueSystemError enum（thiserror 2 採用）を定義
  - EmptyChoiceBarrier / EntityNotFound / CapacityExceeded バリアントを実装
  - Display + Error トレイトを thiserror マクロで自動導出
  - _Requirements: 8_

- [ ] 3.2 (P) 実行結果型の定義
  - CueSheetResult enum（Completed, Cancelled, Timeout, Choice, Error）を定義
  - Clone, Debug derive を適用
  - _Requirements: 9_

- [ ] 3.3 (P) 統合レジストリリソースの実装
  - EntityRegistry リソース（HashMap\<EntityKey, Entity\>）を定義
  - `register()` / `resolve()` — 汎用登録・解決 API
  - `register_actor()` / `resolve_actor()` — アクター向けショートカット
  - `routes_for_actor()` — 指定アクターの全スロットを返却
  - Default, Debug derive を適用
  - _Requirements: 4_

- [ ] 3.4 配送待ちコンポーネントの実装
  - PendingCueSheet コンポーネント（`#[component(storage = "SparseSet")]`）を定義
  - sheet: CueSheet + start_time: f64 フィールドを保持
  - _Requirements: 4_

- [ ] 3.5 内部配送ヘルパーの実装
  - `dispatch_cue_sheet_internal()` 関数を実装
  - 各 Cue を走査し、ルーティングコマンドは EntityRegistry を更新（CueQueue には入らない）
  - 非ルーティングコマンドは `routes_for_actor()` で全スロットにブロードキャスト
  - `cue.start_time + sheet_start_time` で絶対時刻化
  - 各スロットの CueQueue に `push_sorted(TimedCue)` で配送
  - 配送先リスト（Vec\<(ActorKey, CueTarget, Entity)\>）を CueSheetHandle で返却
  - ActorKey 未解決時は `tracing::warn!` でログ出力 + skip
  - _Requirements: 4, 8_

- [ ] 3.6 配送システムの実装
  - `dispatch_pending_cue_sheets()` システム（Update スケジュール、消費者より前）を実装
  - PendingCueSheet を検出し、`dispatch_cue_sheet_internal()` を呼び出し
  - PendingCueSheet を除去し、同一エンティティに CueSheetTracker を付与
  - _Requirements: 4_

- [ ] 3.7 実行状態追跡コンポーネントの実装
  - CueSheetTracker コンポーネント（`#[component(storage = "SparseSet")]`）を定義
  - 配送先リスト（targets）+ 実行結果（result）+ キャンセルフラグ（cancelled）+ バリア状態（barrier_state）を保持
  - `result()` — 実行結果を poll（None = 実行中）
  - `cancel()` — 外部キャンセル要求
  - `receive_barrier(response)` — 消費者からのバリア応答報告
  - _Requirements: 9_

- [ ] 3.8 Tracker 更新ロジックの実装
  - `update()` メソッドに 4 フェーズアルゴリズムを実装
  - **Phase 1**: `detect_barrier_if_needed()` — 全 CueQueueState を走査し、WaitingForClick/Choice を検出したら BarrierState 生成
  - **Phase 2**: `check_barrier_timeout()` — タイムアウト判定 + first_valid 設定
  - **Phase 3**: `resolve_barrier_if_ready()` — first_valid が Some なら残スロットに skip_barrier() 強制適用 + Click/Choice/Timeout 分岐処理
  - **Phase 4**: `check_completion()` — 全スロット Completed → CueSheetResult::Completed
  - プライベートヘルパーメソッドに分割してテスタビリティ向上
  - _Requirements: 9_

- [ ] 3.9 (P) Tracker 更新システムの実装
  - `update_cue_sheet_trackers()` システム（Update スケジュール、消費者システムの後）を実装
  - FrameTime から current_time を取得
  - 全 CueSheetTracker の `update(world, current_time)` を呼び出し
  - _Requirements: 9_

- [ ] 3.10 (P) dola ランタイムリソースの実装
  - DolaRuntime リソース（dola::DolaFacade ラッパー）を定義
  - `new()` / `facade()` / `facade_mut()` API を実装
  - Default derive を適用
  - _Requirements: 6_

- [ ] 3.11 (P) dola ランタイム更新システムの実装
  - `update_dola_runtime()` システム（Update スケジュール）を実装
  - FrameTime の elapsed_secs() を dola に渡して更新
  - _Requirements: 6_

### Phase 4: Integration & Testing

- [ ] 4. 統合・テスト基盤の構築
- [ ] 4.1 (P) 消費者向けコード例の実装
  - バルーン向け消費パターン（Text, Clear, Choice, WaitForChoice 処理）をドキュメントに記述
  - アニメーション向け消費パターン（Emote, Custom コマンド処理）をドキュメントに記述
  - バリア応答のハンドラー / 非ハンドラー判定パターンを記述
  - _Requirements: 2, 5, 7_

- [ ] 4.2 (P) モジュール構造の整備
  - `crates/wintf/src/ecs/cue/` ディレクトリを作成
  - `mod.rs` — 公開 API エクスポート
  - `data_model.rs` — CueSheet, CueCommand, ActorKey, CueTarget, TimedCue
  - `component.rs` — CueQueue, CueQueueState, BarrierState, PendingChoice
  - `tracker.rs` — CueSheetTracker
  - `registry.rs` — EntityRegistry
  - `runtime.rs` — DolaRuntime
  - `error.rs` — CueSystemError, CueSheetResult
  - `dispatch.rs` — dispatch_cue_sheet_internal, dispatch_pending_cue_sheets
  - `systems.rs` — update_cue_sheet_trackers, update_dola_runtime
  - _Requirements: NFR-3_

- [ ] 4.3 (P) ログ出力の実装
  - CueSheet 配送時に `tracing::debug!` でログ出力
  - CueQueue 消費時に `tracing::trace!` でログ出力（高頻度のため trace レベル）
  - ActorKey 未解決時に `tracing::warn!` でログ出力
  - 全コマンド型・エラー型の Debug derive を確認
  - _Requirements: NFR-2_

- [ ] 4.4 (P) ユニットテスト: データモデルの検証
  - CueSheet の start_time 昇順ソート保証を検証
  - CueSheet の `filter_by_actor()` 動作を検証
  - CueCommand の `is_barrier()` / `is_routing_command()` 分類を検証
  - TimedCue のメモリーサイズ（≤ 64B）を assert
  - _Requirements: 1, 2, NFR-1_

- [ ] 4.5 (P) ユニットテスト: CueQueue 基本操作の検証
  - `push_sorted()` の降順ソート維持を検証
  - `pop_ready()` の時刻到達判定を検証
  - `peek()` の非破壊参照を検証
  - capacity 超過時の CapacityExceeded エラーを検証
  - 空キューの `is_empty()` / `len()` を検証
  - _Requirements: 3, 8_

- [ ] 4.6 (P) ユニットテスト: バリアプロトコルの検証
  - Choice 先積み + WaitForChoice ブロック遷移を検証
  - WaitForChoice 空打ちで EmptyChoiceBarrier エラーを検証
  - WaitForClick ブロック + resolve_click() 解除を検証
  - `check_timeout()` のタイムアウト判定を検証
  - `skip_barrier()` の強制スキップを検証
  - _Requirements: 2, 5, 9_

- [ ] 4.7 (P) ユニットテスト: EntityRegistry の検証
  - `register_actor()` / `resolve_actor()` の登録・解決を検証
  - `routes_for_actor()` の全スロット取得を検証
  - EntityKey 名前空間の分離を検証
  - 未登録キーの解決が None を返すことを検証
  - _Requirements: 4_

- [ ] 4.8 統合テスト: CueSheet 配送 E2E の検証
  - CueSheet 生成 → PendingCueSheet spawn → dispatch → CueQueue 配送を E2E で検証
  - 複数演者への配送 + 独立消費を検証
  - ルーティングコマンド（RouteAdd/Switch/Remove）が EntityRegistry のみ更新することを検証
  - 非ルーティングコマンドが全スロットにブロードキャストされることを検証
  - ActorKey 未解決時の warn + 他演者正常配送を検証
  - _Requirements: 1, 4, 8_

- [ ] 4.9 統合テスト: CueSheetTracker ライフサイクルの検証
  - 全演者 Completed → CueSheetResult::Completed を検証
  - `cancel()` → CueSheetResult::Cancelled を検証
  - WaitForChoice + 選択応答 → CueSheetResult::Choice を検証
  - タイムアウト → CueSheetResult::Timeout を検証
  - プロトコル違反 → CueSheetResult::Error を検証
  - _Requirements: 9_

- [ ] 4.10 (P) 統合テスト: dola 統合の検証
  - DolaRuntime リソース初期化を検証
  - `update_dola_runtime()` システムの FrameTime 連携を検証
  - FrameTime と dola::clock::now() の時刻基準統一を検証
  - _Requirements: 6_

- [ ] 4.11* (P) パフォーマンステスト: キュー操作ベンチマーク
  - `push_sorted()` の 100件/1000件 挿入時間を測定
  - `pop_ready()` の 100件/1000件 消費時間を測定
  - 空 CueQueue の `pop_ready()` コストを測定
  - _Requirements: NFR-1_

---

## Requirements Coverage

| Requirement | Title | Covered by Tasks |
|-------------|-------|------------------|
| 1 | CueSheet 構造化台本 | 1.1, 4.4, 4.8 |
| 2 | CueCommand 型安全コマンド | 1.2, 2.2, 4.1, 4.4, 4.6 |
| 3 | CueQueue キューコンポーネント | 1.4, 2.1, 2.3, 2.4, 2.5, 2.7, 4.5 |
| 4 | CueSheet 配送 | 1.3, 3.3, 3.4, 3.5, 3.6, 4.7, 4.8 |
| 5 | 消費プロトコル | 2.1, 2.2, 2.5, 2.6, 2.7, 4.1, 4.6 |
| 6 | dola 統合 | 3.10, 3.11, 4.10 |
| 7 | コマンド拡張 | 1.2, 4.1 |
| 8 | エラーハンドリング | 2.4, 2.5, 3.1, 3.5, 4.5, 4.6, 4.8 |
| 9 | CueSheet ライフサイクル | 2.6, 3.2, 3.7, 3.8, 3.9, 4.6, 4.9 |
| NFR-1 | パフォーマンス | 1.4, 4.4, 4.11 |
| NFR-2 | デバッグ容易性 | 4.3 |
| NFR-3 | ECS 親和性 | 4.2 |

---

## Version History

| Version | Date       | Changes                                    |
| ------- | ---------- | ------------------------------------------ |
| 1.0     | 2026-02-27 | 初版生成。requirements v2.3 + design v2.1 から 47 サブタスク生成。並列実行マーカー (P) 適用 |
