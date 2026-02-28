# Gap Analysis: wintf-P0-dola-boundary

| 項目               | 内容                                                                |
| ------------------ | ------------------------------------------------------------------- |
| **Document Title** | dola ランタイム責務境界 — 実装ギャップ分析                           |
| **Version**        | 3.0                                                                 |
| **Date**           | 2026-02-28 (更新: 2026-02-28)                                       |
| **Requirements**   | v3.0（Req 0 + Req 1-7 + Req 8 + Req 9 + 2 NFR、CueCommand 全バリアント dola 移管反映） |
| **Status**         | 📊 Analyzed (方針確定、設計フェーズ待ち)                            |

---

## 1. 分析サマリー

- **スコープ**: cue モジュール内に誤配置された DolaRuntime の除去、dola への離散コマンドスケジューリング移管（Req 0）、CueCommand 全 11 バリアントの dola 全面移管（Req 8）、cue モジュール再設計方針（Req 9）。コード変更は中〜大規模で、設計判断が多い
- **最大の課題**: `TimedSchedule<T>` の API 設計（D1）。この決定が wintf 側の CueQueue 再設計と balloon06 の DolaBridgeResource 設計を拘束する
- **既存資産の活用度**: cue パイプラインは DolaRuntime に依存しないため、除去（Req 2）は低リスク。dola の既存型システムは `TimedSchedule<T>` の基盤として自然に活用可能
- **隣接仕様との関係**: `wintf-P0-balloon06-text-effects` の `inherited-context.md` が `ecs/dola_bridge/` モジュールと `DolaBridgeResource` を想定。Option D 採用により dola に `TimedSchedule<T>` が揃った状態で balloon06 の実装フェーズに入れる
- **承認方針**: **Option D（dola-first）** — 2026-02-28 にユーザー承認済み。dola が bevy_ecs 非依存の範疇で可能な限りアニメーションエンジンの責務を担う

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

**結論**: DolaRuntime は定義されているが、実稼働パスには一切組み込まれていない。テスト内でのみ手動で World に挿入されている。

### 2.3 既存 ECS Resource パターン

wintf 内の `#[derive(Resource)]` 型:

| Resource | モジュール | 初期化場所 | Send/Sync | 用途 |
|----------|-----------|------------|-----------|------|
| `App` | `ecs/app.rs` | `EcsWorld::new()` | 自動 | アプリ状態 |
| `FrameTime(pub f64)` | `ecs/graphics/core.rs` | `EcsWorld::new()` | 自動（Copy） | フレーム時刻 |
| `FrameCount(pub u32)` | `ecs/graphics/core.rs` | `EcsWorld::new()` | 自動（Copy） | フレーム番号 |
| `TaffyLayoutResource` | `ecs/layout/taffy.rs` | `EcsWorld::new()` | 自動 | レイアウトエンジン |
| `GraphicsCore` | `ecs/graphics/core.rs` | `EcsWorld::new()` | `unsafe impl` | D2D/D3D デバイス |
| `DCompGraphicsResource` | `ecs/graphics/dcomp_resource.rs` | DComp モード時のみ | `unsafe impl` | DComp デバイス |
| `WintfTaskPool` | `ecs/widget/.../task_pool.rs` | `EcsWorld::new()` | 自動 | 非同期タスク |
| **`DolaRuntime`** | **`ecs/cue/runtime.rs`** | **未登録** | **`unsafe impl`** | **dola ラッパー** |

**パターン抽出**:
- COM ポインタ / `Rc` を内部に持つ Resource は `unsafe impl Send + Sync` を手動実装
- `Option<Inner>` パターンで遅延初期化に対応（GraphicsCore, DCompGraphicsResource）
- Resource はその機能ドメインのモジュールに配置（graphics リソースは `ecs/graphics/` 内）

### 2.4 隣接仕様: balloon06-text-effects の DolaBridgeResource

`wintf-P0-balloon06-text-effects/inherited-context.md` が以下の設計を想定:

| 項目 | 内容 |
|------|------|
| **型名** | `DolaBridgeResource` |
| **モジュール** | `ecs/dola_bridge/mod.rs` |
| **方式** | 共有 ECS Resource |
| **API** | `load_document`, `start`, `bind`, `unbind`, `pause`, `resume` |
| **所有モデル** | Document 単位でロード。複数バルーンが同一定義を共有可能 |
| **条件コンパイル** | `#[cfg(feature = "dola")]` |

**重要**: この設計は**未実装**（balloon06 は `phase: "init"`）。本仕様の決定が balloon06 の設計を拘束する。

### 2.5 dola 依存の現状

```toml
# crates/wintf/Cargo.toml
dola = { path = "../dola" }  # 無条件必須依存
```

- `FrameTime` の初期化に `dola::runtime::clock::now()` を使用（`world/mod.rs` L50, L453）
- `CueCommand::Custom` のパラメーター型に `dola::DynamicValue` を使用
- dola feature flag は現在使われていない（cue-system 設計時に必須化された）

---

## 3. 要件別ギャップ分析

### Req 1: DolaRuntime 所有モデル定義

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| 所有モデルの選択肢評価 | Resource パターン 7 件の前例 | **Decision Needed**: 3 選択肢のトレードオフ評価が未実施 |
| 複数インスタンスの独立性保証 | `dola::runtime::DolaRuntime` はグローバル状態なし | Gap なし（dola 側で対応済み） |
| 生存期間ルール | `EcsWorld::new()` 初期化 / `Option<>` 遅延初期化パターン | Gap なし（既存パターンで対応可能） |
| `Rc<DynamicValue>` の Thread Safety | `unsafe impl Send/Sync` の前例（GraphicsCore, DCompGraphicsResource） | Gap なし（既存パターンで対応可能） |

**所有モデル選択肢の評価**:

| 選択肢 | 説明 | 適合度 | 課題 |
|--------|------|--------|------|
| **(a) コンポーネント内部フィールド** | 各 Balloon/Spot エンティティが独自の DolaRuntime を持つ | ✅ 複数インスタンス自然対応<br>✅ エンティティライフサイクルと一致 | ❌ DolaRuntime が `Send/Sync` でないため ECS Component にできない（`unsafe` ラッパー必須）<br>❌ Document 共有の効率低下 |
| **(b) EcsWorld 外部** | アプリケーション層が DolaRuntime を管理し、必要時に貸し出す | ✅ ECS 制約から解放<br>✅ 自由な設計 | ❌ wintf のレイヤー依存方向に反する<br>❌ 消費者からのアクセスパスが煩雑 |
| **(c) 専用モジュールの ECS Resource** | `ecs/dola/` に Resource として配置（balloon06 の DolaBridgeResource 構想に近い） | ✅ 既存パターン踏襲<br>✅ ECS システムから自然にアクセス<br>✅ balloon06 設計との整合性高い | ❌ シングルトン制約（1 World = 1 Resource）<br>⚠️ 複数ランタイム対応には内部に `HashMap` 等が必要 |

**Research Needed**: (c) を採用した場合、Document 単位のスライス管理（balloon06 の `load_document` / `start` 等）と複数ランタイム（1 ランタイム per Document or per Actor）のトレードオフを設計フェーズで深掘りする必要がある。

---

### Req 2: cue モジュールからの除去

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| `runtime.rs` 削除 | 55 行の独立ファイル | Gap なし（参照なし。削除のみ） |
| `update_dola_runtime` 削除 | `systems.rs` L14-17 | Gap なし（`update_cue_sheet_trackers` は独立） |
| `pub use` 削除 | `mod.rs` L311, L313 | Gap なし |
| cue パイプライン動作維持 | 75 テスト全パス確認済み | Gap なし |
| 統合テスト対応 | `cue_dola_integration_test.rs` 147 行、5 テスト | **Migration Needed**: テストの移動先 or 廃止の判断 |

**リスク**: **低**。cue パイプラインは DolaRuntime に一切依存しないことが確認済み。

---

### Req 3: 配置先決定

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| 新規モジュール作成 | `ecs/` に 10+ のドメインモジュールが存在 | Gap なし（パターン確立済み） |
| balloon06 との整合 | `inherited-context.md` で `ecs/dola_bridge/` を想定 | **Alignment Needed**: 名前とスコープの統一 |
| レイヤー依存方向 | ECS レイヤー内で完結 | Gap なし |
| `update_dola_runtime` の再配置 or 廃止 | 現在の実装は結果破棄 | **Decision Needed**: 新配置先で UpdateResult を活用する設計が必要（Req 4 と連動） |

**候補モジュール名**:

| 候補 | 根拠 | balloon06 整合 |
|------|------|----------------|
| `ecs/dola/` | dola ランタイムの ECS 統合基盤 | ⚠️ balloon06 は `dola_bridge/` を想定 |
| `ecs/dola_bridge/` | balloon06 の inherited-context と一致 | ✅ 完全一致 |
| `ecs/animation/` | より汎用的な名前 | ❌ balloon06 と不一致 |

---

### Req 4: UpdateResult 活用方針

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| `changes` 消費パターン | balloon06 の `dola_sync_system` 設計が PropertyBinding → コンポーネント更新を想定 | **Research Needed**: 具体的な消費者が未実装のため、本仕様では方針のみ定義 |
| `triggered` 消費パターン | 既存の消費者なし | **Scope Decision**: 本仕様で定義するか、将来仕様に委譲するか |

**balloon06 から読み取れる消費パターン**:
```
runtime.update(time) → changes → PropertyTarget 解決 → コンポーネント更新
```
これは ECS システム（`dola_sync_system`）内で行う想定。

---

### Req 5: 時刻基準統一

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| 時刻供給元の決定 | `FrameTime.0` = `dola::runtime::clock::now()`（world/mod.rs で毎フレーム更新） | Gap なし（既に統一されている） |
| スケジュール順序 | 13 フェーズのスケジュールラベルが定義済み | Gap なし（Update フェーズ先頭に配置可能） |
| フレーム内一貫性 | `FrameTime` の不変性テスト（`cue_dola_integration_test.rs`）が存在 | Gap なし |

**結論**: 時刻基準は既に事実上統一されている。文書化のみ必要。

---

### Req 6: dola 統合ガイドライン

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| 「思想共有」の定義 | cue-system design.md の "dola の思想" 記述 | **Documentation Needed** |
| 「ランタイム利用」の定義 | balloon06 の inherited-context に統合フロー記載 | **Documentation Needed** |
| 統合手順書 | balloon06 の research.md に断片的記載 | **Consolidation Needed** |

**リスク**: **低**。文書化タスクのみ。設計判断は Req 1/3/4 の結果に依存。

---

### Req 7: cue-system 設計ドキュメント是正

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| design.md の DolaRuntime 記述修正 | design.md 内に DolaRuntime 参照 20 箇所 | **Edit Needed**: mermaid 図、Component Summary、Tech Stack、Req Traceability |
| validation-report の Section 11 | 既に Post-Implementation Discovery として記録済み | Gap なし（参照追加のみ） |

**影響箇所の詳細**:

| design.md の箇所 | 変更内容 |
|------------------|----------|
| L83: Architecture Boundary Map の `DOLA` ノード | 除去またはスコープ外注記 |
| L108: 設計逸脱注記 | dola-boundary 仕様への参照に変更 |
| L119: Tech Stack の "dola（必須依存）" 行 | "dola-boundary 仕様で管理" に変更 |
| L208: Req Traceability の Req 6 行 | dola-boundary 仕様への参照に変更 |
| L234: Component Summary の DolaRuntime 行 | 除去 + dola-boundary 参照 |
| L744-790: DolaRuntime 設計詳細セクション | 除去 or dola-boundary 参照に差し替え |
| L807, L839, L873: ファイル構成・re-export・mermaid | 更新 |

---

### NFR-1: 後方互換性

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| 920+ テスト維持 | DolaRuntime は EcsWorld に未登録、cue テスト 75 件は DolaRuntime 不使用 | Gap なし |
| 統合テスト対応 | `cue_dola_integration_test.rs` 5 テストが `wintf::ecs::cue::runtime::DolaRuntime` を参照 | **Migration Needed** |
| サンプルアプリ | DolaRuntime を使用する example なし | Gap なし |
| 公開 API | `ecs/mod.rs` に DolaRuntime 再エクスポートなし | Gap なし |

---

### NFR-2: 設計文書一貫性

| 技術ニーズ | 既存資産 | ギャップ |
|-----------|----------|----------|
| ARCHITECTURE.md 更新 | §4 "dola の責務" に DolaRuntime 未記載 | **Update Needed**: 配置先決定後に反映 |
| structure.md 更新 | dola クレート構造記載あり。ECS モジュールに cue/ の記載なし | **Update Needed**: 新モジュール追加後に反映 |

---

## 4. 実装アプローチ評価

### Option A: Extend Existing — cue モジュール内で再設計

**説明**: `runtime.rs` と `update_dola_runtime` を cue モジュール内で修正。UpdateResult を活用するよう変更。配置は変えない。

**Trade-offs**:
- ✅ 変更箇所最小（ファイル移動なし）
- ❌ **cue モジュールの責務違反が残る** — cue は「演出指令の配送基盤」であり DolaRuntime 管理は本来の役割ではない
- ❌ balloon06 の `dola_bridge/` 設計との不整合
- ❌ Req 2（除去）の要件を満たせない

**結論**: **不採用**。要件に反する。

---

### Option B: Create New — 新規 `ecs/dola/` モジュール

**説明**: `ecs/dola/` モジュールを新規作成し、DolaRuntime ラッパー・更新システム・統合ガイドラインを集約。

**結論**: **不採用**。Req 0（dola 移管方針）を反映しておらず、cue 層から移動するだけで責務境界の根本問題が解決しない。

---

### Option C: Hybrid — 新規 `ecs/dola_bridge/` モジュール（balloon06 統合）

**説明**: balloon06 の `inherited-context.md` が想定する `ecs/dola_bridge/` をそのまま採用。

**結論**: **部分採用**。DolaRuntime の ECS 配置先には引き続き有力だが、Req 0（dola 移管）との組み合わせが必要。Option D に包含。

---

### ~~Option Z: 現状維持 — 連続値タイムライン vs 離散コマンドキューの分離を維持~~

**棄却確定（2026-02-28）**: ユーザー承認により正式に棄却。dola は離散コマンドスケジューリングを包含する汎用アニメーションエンジンとして拡張する。

> 「dola に離散コマンドスケジューリングを持たせる」承認します。dola は、bevy_ecs に依存しない範疇で、可能な限りアニメーションエンジンとしての責務を移譲させたい。

---

### 🟢 Option D: dola-first — dola に離散コアを移管、wintf は ECS 統合層（**採用方針**）

**説明**: Req 0 の承認方針を実装する。ECS 非依存の演出スケジューリング機能を dola に移管し、wintf は ECS 結合レイヤーとして位置付ける。

**フェーズ分割**:

| フェーズ | 仕様 | 内容 |
|----------|------|------|
| **Phase 1: dola 新規型** | **dola-boundary（本仕様）** | `TimedSchedule<T>`, `CueCommand`（全 11 バリアント、EntityRef(u64)）, `CueScript`, `compile_script`, ドメイン型（`ActorKey`, `CueTarget`, `EntityKey`）を dola に追加 |
| **Phase 2: wintf cue 除去** | **dola-boundary（本仕様）** | `ecs/cue/runtime.rs` 削除、`update_dola_runtime` 削除、cue テスト移動 |
| **Phase 3: wintf cue 再設計** | **dola-boundary または別仕様** | `CueQueue` を `dola::TimedSchedule<dola::CueCommand>` を内包した形に再設計、wintf は ECS ラッピング + u64 ↔ Entity 変換のみ |
| **Phase 4: balloon06 統合** | **balloon06-text-effects** | `DolaBridgeResource` 実装、PropertyBinding, dola_sync_system |

**Trade-offs**:
- ✅ dola の「Declarative Orchestration」の理念に直結（連続値補間 + 離散コマンドスケジューリングの両立）
- ✅ pasta DSL による高レベル演出表現が wintf 非依存で実現可能になる
- ✅ wintf 以外のプラットフォーム（CLI ツール、テストハーネス）からも `TimedSchedule<T>` を利用できる
- ⚠️ dola クレートの追加工数が発生（新規型 3〜4 + API 設計）
- ⚠️ `TimedSchedule` の API 設計が後の wintf 再設計を拘束するため、慣れ今日のリスクあり

---

## 5. 実装複雑度とリスク

### 工数見積

| 要件 | 工数 | 根拠 |
|------|------|------|
| Req 0: dola 新規型 | **L** (5-7日) | `TimedSchedule<T>`, バリア状態機械, `CueCommand` enum, `CueScript`, `compile_script` の新規実装。dola のモジュール構成変更を伴う |
| Req 2: cue 除去 | **S** (1日) | ファイル削除 + re-export 修正 + テスト移動 |
| Req 8: CueCommand 全移管 | **M** (2-3日) | wintf の `command.rs` 全 11 バリアント + ドメイン型を dola に移管、EntityRef(u64) 変換、re-export で後方互換性維持 |
| Req 9: cue モジュール再設計 | **M** (3-4日) | `CueQueue` を `dola::TimedSchedule<T>` 内包形に再設計、環境整備 |
| Req 1: 所有モデル | **M** (2-3日) | 設計判断 + 文書化 + balloon06 整合確認 |
| Req 3: 配置先 | **S** (0.5日) | Option D 採用により `ecs/dola_bridge/` 一択（Follows D1） |
| Req 4: UpdateResult | **S** (0.5日) | 方針決定 + 文書化（実装は balloon06 に委譲） |
| Req 5: 時刻統一 | **S** (0.5日) | 文書化のみ |
| Req 6: ガイドライン | **M** (2日) | 2層モデル（思想共有 vs ランタイム利用）の文書化 |
| Req 7: design.md 是正 | **S** (1日) | 20 箇所の編集 |
| NFR-1/2: 後方互換+文書 | **S** (1日) | テスト実行 + 文書更新 |

**全体工数**: **L〜XL** (2週前後)。当初準 M 評価から Req 0/8/9（dola 新規実装）の追加により大幅増大。

### リスク評価

| リスク | レベル | 根拠 |
|--------|--------|------|
| dola API 設計の慣れいく日 | **高** | `TimedSchedule<T>` のインターフェース設計が後の wintf 再設計を拘束する |
| CueCommand 全移管の破壊的変更 | **中** | wintf 内の全テスト（cue 系 75 件）、re-export / 型エイリアスで緩和可能。EntityRef(u64) 変換レイヤー追加が必要 |
| balloon06 との設計不整合 | **中** | 引き続き存在するが、Option D 採用により `DolaBridgeResource` 設計に引き継ぎやすくなる |
| cue 除去によるリグレッション | **低** | DolaRuntime は cue パイプラインに未接続。920+ テスト中 DolaRuntime 参照は 5 テストのみ |

---

## 6. 推奨事項（設計フェーズへの引き継ぎ）

### 採用アプローチ: Option D（dola-first）

**根拠**:
1. **承認方針の実行**: 「dola は bevy_ecs に依存しない範疇で可能な限り責務を移譲」を体現する唯一の選択肢
2. **dola の Orchestration 理念と的合**: 「Declarative Orchestration for Live Animation」は連続値補間だけでなく離散コマンドの時刻スケジューリングも包含する
3. **pasta DSL との将来統合**: pasta を利用した高レベル演出表現が wintf 非依存で実現可能になる
4. **balloon06 との孔空性**: `DolaBridgeResource` 実装する時に dola の `TimedSchedule<T>` が既に利用可能な状態になる

### 設計フェーズで決定すべき事項

| # | 決定事項 | 優先度 |
|---|----------|--------|
| D1 | `TimedSchedule<T>` の API 設計: バリア型をジェネリック型パラメータにするか、別トレイト `Barrierlike` で抽象化するか | 最高 |
| D2 | dola での演出コマンド enum の名前: `ScriptCommand`, `OrchestratorCommand`, `CueCommand` | 高 |
| D3 | `CueScript` 候補名称: `Script`, `OrchestratorScript` | 中 |
| D4 | wintf 側の型接続: `type CueCommand = dola::CueCommand` re-export のみ vs ラッパー型 | 中 |
| D5 | Req 9 の移行戦略: (a) 即時置換 vs (b) 段階的移行 | 高 |
| D6 | `update_dola_runtime` の処遇: 廃止 vs `ecs/dola_bridge/` へ移動 vs balloon06 に委譲 | 中 |
| D7 | dola feature flag: 必須依存を維持 vs `#[cfg(feature = "dola")]` 再導入 | 中 |
| D8 | `cue_dola_integration_test.rs` の処遇: 移動 vs 書き直し vs 廃止 | 低 |

### Research Items（設計フェーズで調査）

1. **`TimedSchedule<T>` のバリア設計**: `WaitForChoice` / `WaitForClick` がバリアかつ `T` のインスタンスでもある場合、型システム上どう表現するか。`TimedSchedule<Void>` と `TimedSchedule<CueCommand>` の統合方法
2. **balloon06 との DolaBridgeResource API 整合**: `TimedSchedule<T>` の実装後、`DolaBridgeResource` の `start`, `bind`, `unbind` API との相互作用
3. **`Rc<DynamicValue>` の Thread Safety 監査**: `TimedSchedule<T>` に `DynamicValue` を保持する場合の `Send` / `Sync` 安全性
4. **pasta DSL インターフェース設計**: pasta の出力形式が `CueScript` の構造と直接対応つくか、変換層が必要か

---

## 7. 要件→資産マトリクス

| 要件 | 既存資産 | ギャップステータス |
|------|----------|-------------------|
| **Req 0: dola 新規型** | dola の既存 Rust プロジェクト構造 | **New Implementation** — dola に TimedSchedule・CoreCommand・CueScript 追加 |
| Req 1: 所有モデル | Resource パターン 7 件 | **Decision Needed** |
| Req 2: cue 除去 | runtime.rs (55 行), systems.rs (4 行), mod.rs (2 行) | ✅ Ready（削除のみ） |
| Req 3: 配置先 | Option D 採用により `ecs/dola_bridge/` に自然収束 | **Follows D1** |
| Req 4: UpdateResult | balloon06 の dola_sync_system 設計 | **Research Needed** |
| Req 5: 時刻統一 | FrameTime = clock::now() 、既に統一 | ✅ Ready（文書化のみ） |
| Req 6: ガイドライン | cue-system design.md, balloon06 context | **Documentation Needed** |
| Req 7: design.md 是正 | design.md 内 20 箇所特定済み | ✅ Ready（編集作業のみ） |
| **Req 8: CueCommand 全移管** | wintf `command.rs` (81 行)、10/11 バリアントが ECS 非依存、EntityRef のみ Entity 使用→u64 変換で解決 | **Full Migration** — 全 11 バリアント + ドメイン型を dola に移管、wintf は re-export + u64 ↔ Entity 変換のみ |
| **Req 9: cue 再設計** | wintf `CueQueue`（434 行）+ `CueSheetTracker` | **Redesign Needed** — 移行戦略を設計フェーズで決定 |
| NFR-1: 後方互換 | 920+ テスト、DolaRuntime 未登録 | ✅ Ready（cue 除去分）/ **Pending**（dola 変更分） |
| NFR-2: 文書一貫性 | ARCHITECTURE.md, structure.md | **Update Needed** |
