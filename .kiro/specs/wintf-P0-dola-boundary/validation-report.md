# 実装検証レポート

| 項目 | 内容 |
|------|------|
| **仕様** | wintf-P0-dola-boundary |
| **検証日** | 2026-03-02 |
| **検証対象** | requirements.md v4.2 / design.md v2.0 / tasks.md |
| **判定** | ✅ **GO** — 全 23 タスク完了・全要件充足 |

---

## 1. テスト実行結果

| テストスイート | 合格 | 失敗 | 無視 | 結果 |
|-------------|------|------|------|------|
| `cargo test -p wintf --test ecs` | 79 | 0 | 0 | ✅ |
| `cargo test -p dola` (全テストバイナリ) | 398 | 0 | 1 | ✅ |
| **合計** | **477** | **0** | **1** | ✅ |

> 無視 1 件: `DolaRuntime` doctest（`compile_storyboard` を使う既存サンプルコード、本仕様スコープ外）

---

## 2. 要件トレーサビリティ

### Requirement 1: dola 演出スケジューリング基盤

| AC | 要件概要 | 実装ファイル | 判定 |
|----|---------|------------|------|
| 1.1 | dola は bevy_ecs 依存を持たない | `crates/dola/Cargo.toml` — bevy_ecs なし（grep: 0件） | ✅ |
| 1.2 | `TimedSchedule<T>` — Entry 3種分離・2フェーズ API | `crates/dola/src/cue/schedule.rs` | ✅ |
| 1.3 | バリア管理 API (`current_barrier`, `notify_barrier_resolved`, `next_routing`) | `crates/dola/src/cue/schedule.rs` L210, L231 | ✅ |
| 1.4 | `CueSheet` + `compile_sheet` | `crates/dola/src/cue/sheet.rs` | ✅ |
| 1.5 | `CueCommand` 6バリアント（serde対応含む） | `crates/dola/src/cue/command.rs` L118 | ✅ |
| 1.5a | `RoutingCommand` 3バリアント | `crates/dola/src/cue/command.rs` L102-L106 | ✅ |
| 1.6 | ドメイン型 (`ActorKey`, `CueTarget`, `EntityKey`, `Cue`) | `crates/dola/src/cue/command.rs` | ✅ |
| 1.7 | `DolaRuntime` の `tick()` / `last_result()` 分離 | `crates/dola/src/runtime/facade.rs` L315, L323 | ✅ |
| 1.8 | 連続値タイムラインと離散コマンドの責務分離 | 2エンジンをモジュール分離（`runtime/` vs `cue/`） | ✅ |
| 1.9 | pasta DSL インターフェース設計考慮 | Non-Goal（本仕様スコープ外、design.md 記載） | ✅ |

### Requirement 2: DolaAnimator コンポーネント設計

| AC | 要件概要 | 実装ファイル | 判定 |
|----|---------|------------|------|
| 2.1 | `DolaAnimator` ECS Component + `unsafe impl Send + Sync` | `crates/wintf/src/ecs/dola/mod.rs` L43, L51-L52 | ✅ |
| 2.2 | `tick_dola_animators` システム (`Query<&mut DolaAnimator>` + `Res<FrameTime>`) | `crates/wintf/src/ecs/dola/mod.rs` | ✅ |
| 2.3 | 消費者が `last_result()` で読み取る構成 | `dola_animator_test.rs` — consumer_reads_last_result テスト ✅ | ✅ |
| 2.4 | 配置先 `ecs/dola/` モジュール確定 | `crates/wintf/src/ecs/dola/mod.rs` | ✅ |
| 2.5 | balloon06 `DolaBridgeResource` との整合性文書化 | `mod.rs` L15: DolaBridgeResource 上書き設計の旨記載 | ✅ |

### Requirement 3: cue モジュール整理

| AC | 要件概要 | 実装ファイル | 判定 |
|----|---------|------------|------|
| 3.1 | DolaRuntime 関連コード除去 (`runtime.rs`, `update_dola_runtime`, `pub use DolaRuntime`) | grep: cue/ 内に0件 | ✅ |
| 3.2 | `CueQueue` が `dola::TimedSchedule<dola::CueCommand>` を内包する設計 | `crates/wintf/src/ecs/cue/queue.rs` | ✅ |
| 3.3 | バリア管理を dola に委譲、ECS 側は薄いラッパー | `queue.rs` — `TimedSchedule` メソッド委譲 | ✅ |
| 3.4 | `Entity::to_bits()` / `Entity::from_bits()` 変換 | `queue.rs` L139, L158 | ✅ |
| 3.5 | `pub use dola::cue::{...}` re-export で後方互換 | `crates/wintf/src/ecs/cue/command.rs` L6 | ✅ |

### Requirement 4: UpdateResult 活用方針

| AC | 要件概要 | 実装ファイル | 判定 |
|----|---------|------------|------|
| 4.1 | `changes` 消費パターン定義 | 本仕様スコープ外として文書化（balloon06-text-effects に委譲）、`mod.rs` コメント記載 | ✅ |
| 4.2 | `triggered` 消費パターン定義 | 同上 | ✅ |
| 4.3 | スコープ外の場合、将来仕様を明記 | design.md Non-Goals 記載 | ✅ |

### Requirement 5: 設計ドキュメント整合性

| AC | 要件概要 | 実装ファイル | 判定 |
|----|---------|------------|------|
| 5.1 | dola 統合ガイドライン整備 | `mod.rs` コメント + design.md Architecture Boundary Map | ✅ |
| 5.2 | wintf-P0-cue-system design.md の誤記是正 | design.md v2.0: DolaRuntime を「インフラ」として記載している箇所を是正済み | ✅ |
| 5.3 | `ARCHITECTURE.md` / `structure.md` への DolaAnimator 記載 | `doc/ARCHITECTURE.md` §4.3 + `steering/structure.md` §6-7 | ✅ |
| 5.4 | 将来の独自タイムライン選択時の判断基準文書化 | design.md 記載 | ✅ |

### NFR-1: 後方互換性

| AC | 要件概要 | 確認方法 | 判定 |
|----|---------|---------|------|
| NFR 1.1 | wintf 全テスト（ecs: 79件）パス | 実行確認 ✅ | ✅ |
| NFR 1.2 | dola 既存テスト（398件）パス | 実行確認 ✅ | ✅ |
| NFR 1.3 | cue モジュール公開 API から DolaRuntime 除去 | grep 0件確認 ✅ | ✅ |

---

## 3. 設計整合性チェック

| チェック項目 | 結果 |
|------------|------|
| dola `cue/` モジュール新設（`TimedSchedule`, `CueSheet`, `CueCommand`, ドメイン型） | ✅ |
| wintf `ecs/dola/` モジュール新設（`DolaAnimator`, `tick_dola_animators`） | ✅ |
| wintf `ecs/cue/` から DolaRuntime 誤配置コード除去 | ✅ |
| dola が bevy_ecs に非依存（ECS レイヤー依存方向遵守） | ✅ |
| `TimedSchedule` の 0ベース相対オフセット設計（`new(0.0)` とバグ修正適用済み） | ✅ |
| EntityRef 変換境界（push: `to_bits`, pop: `from_bits`）がwintf 側に限定 | ✅ |
| `unsafe impl Send + Sync` の安全性根拠が `Query<&mut DolaAnimator>` 排他アクセスにより確立 | ✅ |

---

## 4. 判明した課題・リスク

| 区分 | 内容 | 重大度 | 対応 |
|------|------|--------|------|
| ℹ️ 設計注記 | `reset_schedule()` は `_start_time` パラメータを現在無視し `new(0.0)` で固定（統合テストで検証済み）。将来マルチ開始時刻サポート時に再設計が必要 | Low | TODO コメント記載済み |
| ℹ️ 設計注記 | `UpdateResult` 消費は balloon06-text-effects 仕様に委譲。本仕様スコープ外として合意済み | Low | 次仕様で対応 |
| ℹ️ 非推奨 | `runtime_mut()` は `pub(crate)` に限定。外部テストからは `with_runtime()` パターンで対応 | Low | 設計意図通り |

---

## 5. タスク完了確認

全 23 サブタスクが `[x]` 完了状態：

- **Task 1-3**: dola `cue/` モジュール新設（全型・関数） ✅
- **Task 4**: `DolaRuntime::tick/last_result` 分離 ✅
- **Task 5**: wintf `ecs/cue/` 除去・再エクスポート ✅
- **Task 6-7**: `CueQueue` リファクタリング ✅
- **Task 8**: テスト移行（7ファイル書き直し + 統合テスト新規） ✅
- **Task 9**: `DolaAnimator` Component + テスト ✅
- **Task 10**: 全回帰テスト ✅

---

## 6. 判定

```
┌─────────────────────────────────────────────┐
│  ✅  GO — 本仕様の実装は完了です              │
│                                             │
│  wintf: 79 passed / dola: 398 passed        │
│  全 23 タスク完了 / 全 P0 要件充足           │
│  軽微な残存課題（Low severity）は次仕様へ   │
└─────────────────────────────────────────────┘
```

本仕様 `wintf-P0-dola-boundary` は **正常に実装完了** しています。  
次の仕様 `wintf-P0-cue-system` の unblock 条件を満たしています。
