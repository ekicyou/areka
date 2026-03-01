# Research & Design Decisions

---
**Feature**: `wintf-P0-dola-boundary`
**Discovery Scope**: Complex Integration（dola 新規モジュール + wintf 移管 + ECS Component 新設）
**Key Findings**:
- dola は flat module 構成（`src/` 直下）と directory module（`compile/`, `runtime/`, `validate/`）の混合。新規 `cue/` ディレクトリモジュールが最適
- `DolaRuntime::update()` は内部で 5 フェーズ処理を行い `UpdateResult` を返却。`tick/last_result` 分離は内部フィールド追加で実現可能
- wintf の `CueQueue` は 434 行のモノリシック実装。`TimedSchedule<T>` 抽出により 200 行以上を dola に移管可能
- `cue_dola_integration_test.rs` は 8 テスト中 5 件が DolaRuntime Resource テスト、3 件が FrameTime テスト
---

## Research Log

### dola モジュール構成パターン

- **コンテキスト**: 新規 `cue/` モジュールの配置先と構成パターンを決定する
- **調査対象**: `crates/dola/src/lib.rs`, `crates/dola/src/compile/`, `crates/dola/src/runtime/`
- **所見**:
  - `compile/` は `mod.rs` + `resolve.rs` + `types.rs` の 3 ファイル構成
  - `validate/` は `mod.rs` + `rules.rs` の 2 ファイル構成
  - `runtime/` は `mod.rs` + `facade.rs` + `types.rs` + `clock.rs` + 5 internal modules
  - トップレベル `lib.rs` は private mod + pub use で flat re-export パターン
- **結論**: `cue/` ディレクトリモジュールを作成。`mod.rs`（re-export）+ `schedule.rs`（TimedSchedule, Entry, BarrierKind）+ `command.rs`（CueCommand, RoutingCommand, CuePayload, ドメイン型）+ `sheet.rs`（CueSheet, compile_sheet, CompiledCue）

### CueQueue → TimedSchedule 抽出可能性

- **コンテキスト**: wintf `CueQueue`（434 行）のどの部分が dola `TimedSchedule<T>` に抽出可能か
- **調査対象**: `crates/wintf/src/ecs/cue/queue.rs`
- **所見**:
  - `push_sorted`（降順 binary search + insert）: 汎用化可能 → `TimedSchedule::insert()`
  - `pop_ready`（末尾 pop + 時刻比較 + barrier 遷移）: コア部分を `advance()` に抽出可能
  - `BarrierState`（kind + start_time + timeout + first_valid）: `Entry::Barrier` + `BarrierKind` に分解
  - `pending_choices`: CueCommand 固有ロジック。`TimedSchedule<T>` のスコープ外 → wintf `CueQueue` に残留
  - `CueQueueState` (Playing/Paused/WaitingForClick/WaitingForChoice/Error/Completed): ECS 固有状態。wintf に残留
  - `cue_sheet_entity`, `playback_rate`, `capacity`: ECS 固有。wintf に残留
- **結論**: 時刻ソート・消費・バリア管理の**コアロジック**（約 150 行相当）を `TimedSchedule<T>` に抽出。ECS 固有の状態管理・Choice 蓄積・Entity 参照は wintf `CueQueue` に残留

### dispatch フロー分析

- **コンテキスト**: `compile_sheet` 関数の設計に向けた dispatch フローの理解
- **調査対象**: `crates/wintf/src/ecs/cue/dispatch.rs`
- **所見**:
  - `dispatch_cue_sheet_internal()` が核心: `CueSheet.cues()` を走査し、ルーティングコマンドは `EntityRegistry` に適用、非ルーティングコマンドは `absolute_time = start_time + cue.start_time` で `CueQueue.push_sorted()` に挿入
  - `EntityRegistry` による Actor → Entity 解決は ECS 固有
  - 相対→絶対変換のロジック自体は単純な加算: `cue.start_time + sheet_start_time`
- **結論**: dola の `compile_sheet(sheet: &CueSheet) -> Vec<CompiledCue>` として相対時刻を 0 ベース相対オフセットに正規化。CuePayload から Entry<CueCommand> への変換も含む。絶対時刻への変換は TimedSchedule::new(start_time) が担当。Entity 解決は wintf dispatch が担当

### DolaRuntime tick/last_result 分離

- **コンテキスト**: `update()` の返り値を内部フィールドに格納する改修の実現可能性
- **調査対象**: `crates/dola/src/runtime/facade.rs` L311
- **所見**:
  - 現行: `pub fn update(&mut self, current_time: f64) -> UpdateResult`
  - `UpdateResult` は `changes: Vec<(i64, EvaluatedValue)>` + `triggered: Vec<TriggerResult>` の 2 フィールド
  - `EvaluatedValue` は `Object(Rc<DynamicValue>)` バリアントを含む → `Clone` は可能だが Rc のクローンコストあり
  - `UpdateResult` を `Option<UpdateResult>` として構造体フィールドに保持し、`tick()` で上書き、`last_result()` で `Option::as_ref()` で返却するのが最適
- **結論**: `DolaRuntime` に `last_update_result: Option<UpdateResult>` フィールドを追加。`update()` は deprecated → `tick()` + `last_result()` に分離。後方互換のため `update()` は `tick()` 呼び出し + `last_result().cloned()` で維持可能

### balloon06 DolaBridgeResource との整合

- **コンテキスト**: balloon06-text-effects の inherited-context.md が想定する `DolaBridgeResource` との整合
- **調査対象**: `wintf-P0-balloon06-text-effects/inherited-context.md`（gap-analysis.md §2.4 引用）
- **所見**:
  - `DolaBridgeResource` は `ecs/dola_bridge/mod.rs` に配置を想定
  - API: `load_document`, `start`, `bind`, `unbind`, `pause`, `resume`
  - **問題**: Resource（共有シングルトン）前提 → DolaAnimator は Component（エンティティごと）
  - balloon06 は `phase: "init"` — 実装未着手
- **結論**: balloon06 の DolaBridgeResource 設計は DolaAnimator Component 設計に合わせて調整が必要。モジュール名は `ecs/dola/` を採用し、balloon06 の `dola_bridge/` 想定を上書きする。balloon06 側の context を更新する

---

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: cue/ flat module** | `cue/` 内に全型を `mod.rs` 1 ファイルで定義 | 最小ファイル数 | 500+ 行の巨大ファイル | ❌ 却下 |
| **B: cue/ directory module** | `cue/mod.rs` + `schedule.rs` + `command.rs` + `sheet.rs` | 責務分離明確、既存パターン踏襲 | ファイル数増 | ✅ 採用 |
| **C: schedule を runtime 内に配置** | `runtime/schedule.rs` として DolaRuntime と同居 | 層の統合 | 連続値と離散を混在、責務曖昧 | ❌ 却下（2 エンジン分離原則に反する） |

---

## Design Decisions

### Decision D4: wintf 側の型接続 — re-export only

- **コンテキスト**: dola に移管した `CueCommand`, `ActorKey`, `CueTarget`, `EntityKey`, `Cue`, `CueSheet`, `BarrierKind` を wintf がどう参照するか
- **代替案**:
  1. `type CueCommand = dola::CueCommand;` — 型エイリアスによる re-export
  2. `struct WintfCueCommand(dola::CueCommand)` — newtype ラッパー
- **採用**: **Option 1（re-export only）**
- **根拠**:
  - `CueCommand` は dola と wintf で同一のセマンティクス。wintf 固有の拡張不要
  - `EntityRef(u64)` の `u64 ↔ Entity` 変換は push/pop 境界メソッドで処理するため、型レベルでの差異不要
  - newtype は `match` でのパターンマッチにアンラップが必要となり、消費者コードの可読性が低下する
  - 将来 wintf 固有の拡張が必要になった場合、re-export から newtype への移行は後方互換を保って可能
- **トレードオフ**: wintf 固有メソッド追加不可 vs コード簡潔性。簡潔性を優先
- **フォローアップ**: wintf `cue/command.rs` を re-export ファイルに簡素化

### Decision D5: 移行戦略 — Phase 1 → 2a → 2b+3 段階移行

- **コンテキスト**: dola 新規実装と wintf 除去の順序
- **代替案**:
  1. Phase 1→2→3 の厳密な順序
  2. Phase 2（除去のみ）を先行し、Phase 1 + 3 を後続
  3. Phase 1 → 2a（DolaRuntime 除去）→ 2b+3（CueCommand 移管 + CueQueue 再設計を同時）
- **採用**: **Option 3（段階移行）**
- **根拠**:
  - Phase 2a（DolaRuntime 除去）は消費者ゼロのため即時実行可能（Phase 1 に非依存）
  - Phase 2b（CueCommand 移管）は Phase 1 の `CueCommand` 定義に依存 → Phase 1 後
  - Phase 3（CueQueue 再設計）は Phase 1 の `TimedSchedule<T>` に依存 → Phase 1 後
  - Phase 2b と Phase 3 は密結合のため同時実行が効率的
- **トレードオフ**: 並行可能な作業を分割することで各 PR のサイズを小さく保つ vs 全体工期
- **フォローアップ**: tasks.md で Phase 2a を Phase 1 と並行可能なタスクとして記載

### Decision D7: dola feature flag — 必須依存（flag なし）

- **コンテキスト**: `CueSheet` 系モジュールを `#[cfg(feature = "cue")]` で分離するか
- **代替案**:
  1. `#[cfg(feature = "cue")]` で opt-in
  2. 必須依存（常に含む）
- **採用**: **Option 2（必須依存）**
- **根拠**:
  - `CueSheet` / `TimedSchedule` は外部依存を追加しない（serde は既存 feature で管理）
  - dola クレートの位置づけは「アニメーション実現のための汎用道具集」— 2 エンジン（dola + キューシート）が core
  - `CueCommand` が `DynamicValue` を使用（dola コア型）→ 循環的依存の分離が煩雑
  - pasta DSL が `CueSheet` を出力する想定 → opt-out される可能性が低い
- **トレードオフ**: バイナリサイズ微増 vs API 表面の単純化。サイズ増は無視できるレベル（型定義のみ、ランタイム依存なし）
- **フォローアップ**: `CueSheet` の serde 対応は既存 `json`/`toml`/`yaml` feature に相乗り

### Decision D8: cue_dola_integration_test.rs — 分割移行

- **コンテキスト**: 既存 8 テストの処遇
- **代替案**:
  1. 全廃止
  2. DolaAnimator テストに全書き直し
  3. 分割: FrameTime テストは維持、DolaRuntime テストは DolaAnimator テストに書き直し
- **採用**: **Option 3（分割移行）**
- **根拠**:
  - FrameTime 3 テスト（`frame_time_consistent_within_frame`, `frame_time_default_initializes_to_zero`, `frame_time_injectable_for_testing`）は DolaRuntime に無関係。既存テストファイル（`tests/ecs/graphics/` 等）に移動
  - DolaRuntime 5 テスト（Resource init, Default, facade_mut update, system runs, multiple frames）：Resource → Component 変更により無効化。DolaAnimator の同等テストを `tests/ecs/dola/` に新設
  - `cue_dola_integration_test.rs` ファイル自体は削除
- **トレードオフ**: テスト移行コスト vs テストカバレッジ維持。カバレッジ維持を優先
- **フォローアップ**: `tests/ecs.rs` の `#[path]` mod 宣言を更新

---

## Risks & Mitigations

- **CueCommand 移管時のコンパイルエラー波及** — re-export（D4 決定）により `use wintf::ecs::cue::CueCommand` のインポートパスが維持され、波及なし
- **CueQueue 再設計の複雑さ** — Phase 2b+3 で同時実施し、`TimedSchedule<T>` を内包する薄いラッパーとすることで再設計範囲を限定
- **balloon06 設計との乖離** — balloon06 は `phase: "init"` で未実装。本仕様の DolaAnimator 設計を正とし、balloon06 の inherited-context を更新する
- **`unsafe impl Send + Sync` の安全性根拠** — DolaAnimator の `tick()` 呼び出しを `tick_dola_animators` システムの `Query<&mut>` 排他アクセスに限定することで、Rc の thread-safety を保証。文書化必須

---

## References

- `crates/dola/src/runtime/facade.rs` L311 — 現行 `update()` シグネチャ
- `crates/wintf/src/ecs/cue/queue.rs` — CueQueue 実装（TimedSchedule 抽出元）
- `crates/wintf/src/ecs/cue/dispatch.rs` — dispatch フロー（compile_sheet 設計参照）
- `crates/wintf/tests/ecs/cue_dola_integration_test.rs` — 移行対象テスト
- bevy_ecs 0.18.0 Component trait — `Send + Sync + 'static` 要件
