# Gap Analysis: wintf-P0-dola-boundary

| 項目               | 内容                                                            |
| ------------------ | --------------------------------------------------------------- |
| **Document Title** | dola ランタイム責務境界 — 実装ギャップ分析                      |
| **Version**        | 5.0                                                             |
| **Date**           | 2026-03-01                                                      |
| **Requirements**   | v4.2（命名確定・内部整合リファイン完了）                         |
| **Status**         | 📊 Analyzed（v5.0: D1-D3/D6 全確定・設計フェーズ待ち）         |

---

## 1. 分析サマリー

- **スコープ**: ①dola への離散コマンドスケジューリング基盤新規実装、②DolaRuntime API 分離（`update` → `tick/last_result`）、③wintf cue モジュールから誤配置 DolaRuntime 除去、④wintf `CueCommand` / ドメイン型の dola への移管
- **最大の課題**: `CueCommand` の移管と wintf 側 `Entity` 参照処理 — `Entity` は bevy_ecs 型のため u64 変換が境界に必要。wintf cue テスト（cue 系）の移行方針が設計フェーズで決定が必要
- **既存資産の活用度**: dola はすでに bevy_ecs 非依存。`CueQueue.push_sorted` / `pop_ready` のロジックが `TimedSchedule<T>` の参考実装として転用可能。誤配置コードは参照者がなく低リスクで除去可能
- **採用方針**: **Option D（dola-first）** — dola が bevy_ecs 非依存の範疇で可能な限りアニメーションエンジンの責務を担う
- **確定済み設計決定**: D1（`TimedSchedule<T>` API）、D2（`CueCommand` 名称）、D3（`CueSheet` / `compile_sheet` 名称）、D6（`update_dola_runtime` 廃止）— 設計フェーズへの持越しは D4/D5/D7/D8 のみ

---

## 2. 現状調査（Current State Investigation）

### 2.1 DolaRuntime 関連コードの所在

| ファイル | 行数 | 内容 | 消費者 |
|----------|------|------|--------|
| `ecs/cue/runtime.rs` | 55 行 | `DolaRuntime` ラッパー（`#[derive(Resource)]`, `unsafe impl Send/Sync`） | テストのみ |
| `ecs/cue/systems.rs` L14-17 | 4 行 | `update_dola_runtime` — `FrameTime.0` → `DolaRuntime.update()`, 結果破棄 | テストのみ |
| `ecs/cue/mod.rs` L311 | 1 行 | `pub use runtime::DolaRuntime;` | テストのみ |
| `ecs/cue/mod.rs` L313 | 1 行 | `pub use systems::update_dola_runtime;` | テストのみ |
| `tests/ecs/cue_dola_integration_test.rs` | 147 行 | 5 テスト（Resource 初期化、Default、update 呼び出し、FrameTime 連携、マルチフレーム） | — |

### 2.2 DolaRuntime の EcsWorld 登録状況

| 項目 | 状態 |
|------|------|
| `EcsWorld::new()` での `insert_resource` | **未登録** |
| スケジュールへのシステム登録 | **未登録** |
| `ecs/mod.rs` での再エクスポート | **なし**（`pub mod cue;` 経由のみ） |
| `lib.rs` での再エクスポート | **なし** |

**結論**: DolaRuntime は定義されているが、実稼働パスには一切組み込まれていない。

### 2.3 既存 ECS Resource パターン

| Resource | モジュール | 初期化 | Send/Sync | 用途 |
|----------|-----------|--------|-----------|------|
| `App` | `ecs/app.rs` | `EcsWorld::new()` | 自動 | アプリ状態 |
| `FrameTime(pub f64)` | `ecs/graphics/core.rs` | `EcsWorld::new()` | 自動（Copy） | フレーム時刻 |
| `FrameCount(pub u32)` | `ecs/graphics/core.rs` | `EcsWorld::new()` | 自動（Copy） | フレーム番号 |
| `TaffyLayoutResource` | `ecs/layout/taffy.rs` | `EcsWorld::new()` | 自動 | レイアウト |
| `GraphicsCore` | `ecs/graphics/core.rs` | `EcsWorld::new()` | `unsafe impl` | D2D/D3D |
| `DCompGraphicsResource` | `ecs/graphics/dcomp_resource.rs` | DComp 時のみ | `unsafe impl` | DComp |
| `WintfTaskPool` | `ecs/widget/.../task_pool.rs` | `EcsWorld::new()` | 自動 | 非同期タスク |
| **`DolaRuntime`** | **`ecs/cue/runtime.rs`** | **未登録** | **`unsafe impl`** | **dola ラッパー** |

**パターン**: COM ポインタ / `Rc` を内部に持つ型は `unsafe impl Send + Sync` を手動実装。

### 2.4 balloon06-text-effects の DolaBridgeResource

`wintf-P0-balloon06-text-effects/inherited-context.md` の想定:

| 項目 | 内容 |
|------|------|
| **型名** | `DolaBridgeResource` |
| **モジュール** | `ecs/dola_bridge/mod.rs` |
| **方式** | 共有 ECS Resource |
| **API** | `load_document`, `start`, `bind`, `unbind`, `pause`, `resume` |

**重要**: この設計は**未実装**（balloon06 は `phase: "init"`）。本仕様の DolaAnimator Component 設計により、Resource 前提から Component 前提への調整が必要。

### 2.5 dola 依存の現状

```toml
# crates/wintf/Cargo.toml
dola = { path = "../dola" }  # 無条件必須依存
```

- `FrameTime` の初期化に `dola::runtime::clock::now()` を使用（`world/mod.rs` L50, L453）
- `CueCommand::Custom` のパラメーター型に `dola::DynamicValue` を使用
- dola feature flag は現在未使用

---

## 3. 要件別ギャップ分析

### Req 1: dola 演出スケジューリング基盤

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| 層構造（上位/下位） | 層概念は座標指定済み、コードなし | Gap なし（要件に層構造定義を提示済み） |
| `TimedSchedule<T>` 型内部構造 | 未実装 | ✅ **決定済み**: `Entry<T> = Payload(f64, T) \| Barrier(f64, BarrierKind)` 型レベル分離 |
| `TimedSchedule<T>` API | wintf `CueQueue::pop_ready()` が原型 | ✅ **決定済み**: `tick(&mut self, f64)`（冪等）+ `ready(&self) -> &[T]`（次の `tick()` まで何度でも参照可能）の 2 フェーズ分離。`DolaRuntime` の `tick/last_result` と対称 |
| バリア管理 API | wintf `CueQueue.barrier_state` | ✅ **決定済み**: `current_barrier(&self) -> Option<&BarrierKind>` + `resolve_barrier(&mut self)` |
| `CueSheet` + `compile_sheet` | wintf `CueSheet`（削除）+ `dispatch` の一部 | **New Implementation** — dola に新規型・新規関数 |
| `CueCommand` 9 バリアント（バリア 2 件を `BarrierKind` に分離） | wintf `command.rs`（11 バリアント） | **Migration + Reduction** — `WaitForChoice`/`WaitForClick` を `BarrierKind` に移動して移管 |
| 演出ドメイン型 | wintf `cue/mod.rs` | **Migration** — `ActorKey`, `CueTarget`, `EntityKey`, `Cue` を dola に移管 |
| `tick/last_result` API 分離 | 現行 `update(&mut self) -> UpdateResult` | **Refactor** — dola 側の `DolaRuntime` を `tick()` + `last_result()` に分離 |
| 連続値タイムラインとの責務分離 | 現行モジュール構成 | **Architecture** — モジュール構成で分離を表現 |
| pasta DSL 互換 | 未実装 | **Design Only** — インターフェース設計の考慮 |
| bevy_ecs 非依存 | dola は現在 bevy_ecs に依存しない | Gap なし |

---

### Req 2: DolaAnimator コンポーネント設計

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| `DolaAnimator` Component | `ecs/cue/runtime.rs` の Resource ラッパーが原型 | **New Implementation** — Resource → Component、`tick()` + `last_result()` API に対応 |
| `unsafe impl Send + Sync` | 前例あり（GraphicsCore, DCompGraphicsResource） | Gap なし（パターン確立済み） |
| `tick_dola_animators` システム | `update_dola_runtime` が原型 | **New Implementation** — `Query<&mut DolaAnimator>` で全エンティティ一括 tick |
| Update スケジュール先頭配置 | 13 フェーズのスケジュールラベル定義済み | Gap なし |
| 消費者パターン | `.after()` パターンが ECS スケジュールに存在 | Gap なし |
| 配置先モジュール | `ecs/` に 10+ のドメインモジュール | **Decision Needed** — `ecs/dola/` or `ecs/dola_bridge/` |
| balloon06 整合 | `DolaBridgeResource`（Resource 前提） | **Alignment Needed** — Resource → Component 調整 |

**配置先候補**:

| 候補 | 根拠 | balloon06 整合 |
|------|------|----------------|
| `ecs/dola/` | dola ランタイムの ECS 統合基盤 | ⚠️ balloon06 は `dola_bridge/` を想定 |
| `ecs/dola_bridge/` | balloon06 の inherited-context と名称一致 | ⚠️ Resource → Component 再設計が必要 |

---

### Req 3: cue モジュール整理

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| DolaRuntime 関連コード除去 | 55 + 4 + 2 行の独立コード | Gap なし（参照なし、削除のみ） |
| cue パイプライン動作維持 | 75 テスト全パス確認済み | Gap なし |
| `CueQueue` リファクタリング | `CueQueue`（434 行）+ `CueSheetTracker` | **Redesign Needed** — `dola::TimedSchedule<dola::CueCommand>` 内包形に再設計 |
| `u64 ↔ Entity` 変換 | `Entity::to_bits()` / `from_bits()` が利用可能 | **New Implementation** — push/pop 境界に変換レイヤー追加 |
| re-export 後方互換 | `type` エイリアスパターンは Rust 標準 | Gap なし |
| 移行戦略 | 未決定 | **Decision Needed** — 即時置換 vs 段階的移行 |
| 統合テスト | `cue_dola_integration_test.rs` 5 テスト | **Migration Needed** — 移動 or 書き直し or 廃止 |

**リスク**: DolaRuntime 除去は**低リスク**。cue パイプラインは DolaRuntime に一切依存しない。

---

### Req 4: UpdateResult 活用方針

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| `changes` 消費パターン | balloon06 の `dola_sync_system` が PropertyBinding → コンポーネント更新を想定 | **Research Needed** — 具体的消費者が未実装 |
| `triggered` 消費パターン | 既存の消費者なし | **Scope Decision** |

---

### Req 5: 設計ドキュメント整合性

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| dola 統合ガイドライン | cue-system design.md, balloon06 context に断片的記載 | **Documentation Needed** |
| cue-system design.md 是正 | design.md 内に DolaRuntime 参照 20 箇所 | **Edit Needed** |
| ARCHITECTURE.md / structure.md | dola クレート構造記載あり | **Update Needed** — 配置先決定後に反映 |

**design.md 影響箇所**:

| 箇所 | 変更内容 |
|------|----------|
| L83: Architecture Boundary Map | `DOLA` ノード除去 or スコープ外注記 |
| L119: Tech Stack | "dola-boundary 仕様で管理" に変更 |
| L208: Req Traceability | dola-boundary 参照に変更 |
| L234: Component Summary | DolaRuntime 行除去 |
| L744-790: DolaRuntime 設計詳細 | 除去 or dola-boundary 参照 |

---

### NFR-1: 後方互換性

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| wintf 920+ テスト | DolaRuntime は EcsWorld に未登録 | Gap なし |
| cue 75 テスト | DolaRuntime 不使用 | Gap なし |
| 統合テスト | `cue_dola_integration_test.rs` 5 テストが `DolaRuntime` 参照 | **Migration Needed** |
| dola 既存テスト | 新規型追加は既存に影響しない | Gap なし |
| サンプルアプリ | DolaRuntime を使用する example なし | Gap なし |
| 公開 API | `ecs/mod.rs` に DolaRuntime 再エクスポートなし | Gap なし |

---

## 4. 実装アプローチ

### 採用: Option D（dola-first）

ECS 非依存の演出スケジューリング機能を dola に移管し、wintf は ECS 結合レイヤーとして位置付ける。

**フェーズ分割**:

| フェーズ | 仕様 | 内容 |
|----------|------|------|
| **Phase 1: dola 新規型** | 本仕様 | `TimedSchedule<T>`, `CueCommand`（9 バリアント）, `CueSheet`, `compile_sheet`, ドメイン型 4 種, `tick/last_result` API 分離 |
| **Phase 2: wintf 除去** | 本仕様 | `ecs/cue/runtime.rs` 削除、`update_dola_runtime` 削除、テスト移動 |
| **Phase 3: wintf 再設計** | 本仕様 or 別仕様 | `CueQueue` を `dola::TimedSchedule<dola::CueCommand>` 内包形に再設計 |
| **Phase 4: balloon06 統合** | balloon06-text-effects | DolaAnimator 活用、PropertyBinding, dola_sync_system |

**根拠**:
- dola の「Declarative Orchestration」理念に直結
- pasta DSL による高レベル演出表現が wintf 非依存で実現可能
- wintf 以外のプラットフォームからも `TimedSchedule<T>` を利用可能

---

## 5. 実装複雑度とリスク

### 工数見積

| 要件 | 工数 | 根拠 |
|------|------|------|
| Req 1: dola 新規型 + API | **L** (5-7日) | `TimedSchedule<T>`, `BarrierKind`（3 種）, `CueCommand`（9 バリアント）, `CueSheet`, `compile_sheet`, ドメイン型 4 種, `tick/last_result` 分離 |
| Req 2: DolaAnimator | **M** (2-3日) | Component 実装 + tick システム + 配置先決定 |
| Req 3: cue 整理 | **M** (3-4日) | 除去（S）+ CueQueue 再設計（M）+ 移行戦略 |
| Req 4: UpdateResult | **S** (0.5日) | 方針決定 + 文書化（実装は balloon06 に委譲） |
| Req 5: ドキュメント | **M** (2日) | ガイドライン整備 + design.md 是正 20 箇所 + ARCHITECTURE.md |
| NFR-1: 後方互換 | **S** (1日) | テスト実行 + 統合テスト移行 |

**全体工数**: **L〜XL** (2週前後)

### リスク評価

| リスク | レベル | 根拠 |
|--------|--------|------|
| ~~`TimedSchedule<T>` API 設計~~ | ✅ **解溈** | API 確定（`Entry<T>` 型分離 + `tick/ready` 2 フェーズ）により拘束リスク解溈 |
| CueCommand 全移管の影響 | **中** | wintf cue テスト 75 件に影響。re-export で緩和可能 |
| balloon06 との設計不整合 | **中** | Resource → Component 調整が必要だが balloon06 は未実装 |
| cue 除去リグレッション | **低** | DolaRuntime は cue パイプラインに未接続 |

---

## 6. 推奨事項（設計フェーズへの引き継ぎ）

### 設計フェーズで決定すべき事項

| # | 決定事項 | 関連要件 | 優先度 |
|---|----------|----------|--------|
| ~~D1~~ | ~~`TimedSchedule<T>` の API 設計~~ → ✅ 確定: `Entry<T> = Payload(f64, T) \| Barrier(f64, BarrierKind)` 型レベル分離、`tick()` + `ready(&self) -> &[T]` 2 フェーズ API（`tick/last_result` と対称） | Req 1 AC2-3 | ✅ **決定済み** |
| ~~D2~~ | ~~dola での演出コマンド enum 名称候補~~ → ✅ 確定: `CueCommand`（9 バリアント、バリア 2 件は `BarrierKind` に分離） | Req 1 AC5 | ✅ **決定済み** |
| ~~D3~~ | ~~`CueScript` 候補名称~~ → ✅ 確定: `CueSheet` / `compile_sheet`（wintf 側の同名型は削除） | Req 1 AC4 | ✅ **決定済み** |
| **D4** | wintf 側の型接続: `type CueCommand = dola::CueCommand` re-export のみ vs newtypeラッパー | Req 3 AC5 | 高 |
| **D5** | 移行戦略: Phase 1→2→3 の順序と Phase 2 先行の可否 | Req 3 AC6 | 高 |
| ~~D6~~ | ~~`update_dola_runtime` の処遇~~ → ✅ 確定: **廃止**（Req 3 AC1 で明文化） | Req 3 AC1 | ✅ **決定済み** |
| **D7** | dola feature flag: `CueSheet` 系を `#[cfg(feature = "cue")]` として分離するか、必須依存とするか | 横断 | 中 |
| **D8** | `cue_dola_integration_test.rs` 処遇: `DolaAnimator` テストに書き直す vs 廃止 | Req 3 | 低 |

### Research Items

1. ~~**`TimedSchedule<T>` のバリア設計**~~: 決定済み — `Entry<T> = Payload(f64, T) | Barrier(f64, BarrierKind)` により旧 `WaitForChoice`/`WaitForClick` は `CueCommand` から除外され `BarrierKind`（WaitForInput/WaitForChoice/Timeout 3 種）として `Entry::Barrier` で管理される
2. **balloon06 との DolaBridgeResource API 整合**: `TimedSchedule<T>` 実装後の `DolaBridgeResource` の `start`, `bind`, `unbind` API との相互作用
3. **pasta DSL インターフェース設計**: pasta の出力形式が `dola::CueSheet` の構造と直接対応するか、変換層が必要か

---

## 7. 要件→資産マトリクス

| 要件 | 既存資産 | ギャップステータス |
|------|----------|-------------------|
| **Req 1: dola 基盤** | dola 既存プロジェクト構造、wintf `CueQueue`/`command.rs` が原型 | **New Implementation + Migration** |
| **Req 2: DolaAnimator** | `ecs/cue/runtime.rs` (Resource ラッパー)、`update_dola_runtime` | **New Implementation** — Resource → Component |
| **Req 3: cue 整理** | runtime.rs (55行), systems.rs (4行), mod.rs (2行), CueQueue (434行) | 除去: ✅ Ready / 再設計: **Redesign Needed** |
| Req 4: UpdateResult | balloon06 の dola_sync_system 設計 | **Research Needed** |
| Req 5: ドキュメント | design.md 影響 20 箇所特定済み、ARCHITECTURE.md | **Documentation Needed** |
| NFR-1: 後方互換 | 920+ テスト、DolaRuntime 未登録 | ✅ Ready（除去分）/ **Pending**（dola 変更分） |
