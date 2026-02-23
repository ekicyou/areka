# 親仕様からの引継ぎコンテキスト

> **親仕様**: `wintf-P0-balloon-system`
> **対象要件**: R11（文字単位エフェクト）、R12（dola アニメーション統合）

---

## 参照すべき設計情報

### design.md

- **DolaBridgeResource リソース定義**: `DolaRuntime` を ECS `Resource` としてラップ。`load_document`, `start`, `bind`, `unbind`, `pause`, `resume` のサービスインターフェース
- **PropertyBinding / PropertyTarget**: dola Variable → エンティティプロパティ（`AnimatableProperty`）への対応付け
- **AnimatableProperty enum**: `Opacity`, `IsVisible`, `OffsetX`, `OffsetY` — グリフエンティティの `Visual.opacity`, `Visual.is_visible`, `Arrangement.offset` にバインド
- **dola_sync_system**: 毎フレーム `runtime.update(time)` → `changes` イテレート → `PropertyTarget` 解決 → コンポーネント更新
- **GlyphTimeline / GlyphTimelineEntry**: グリフレベルタイムライン（`show_at`, `weight`, `link_id`）。BalloonToken → GlyphTimeline への IR 変換は GlyphContainer が担当
- **タイプライター効果**: opacity 0→1 のシーケンシャル遷移として表現。dola Storyboard で制御
- **モジュール配置**: `ecs/dola_bridge/mod.rs`（DolaBridgeResource）、`ecs/dola_bridge/sync.rs`（dola_sync_system, PropertyBinding）
- **条件コンパイル**: `#[cfg(feature = "dola")]` で dola 依存をオプショナル化

### research.md — dola 関連

- **依存方式**: `dola = { path = "../dola", optional = true }` で wintf の Cargo.toml に追加（G18）。areka が `wintf = { features = ["dola"] }` で有効化
- **D7**: dola_bridge ECS リソース設計 — 共有 ECS Resource 方式を採用。DolaRuntime は document 単位でロード、複数バルーンが同一定義を共有可能
- **D4**: タイプライター効果は dola の SequenceTransition として実装。各グリフに遅延付き opacity 遷移を設定
- **dola↔ECS 統合フロー**: `subscribe(variable_name)` → `variable_id: i64` → `update(time)` → `changes: Vec<(i64, EvaluatedValue)>` で差分配信

### research.md — パフォーマンス

- **P1**: dola の毎フレーム update コストは Variable 数に比例。200グリフ × 3パラメータ = 600 Variable でも <1ms を想定。ボトルネックは描画側

---

## 子仕様スコープ

- DolaBridgeResource の実装（DolaRuntime の ECS Resource 化）
- PropertyBinding によるグリフエンティティプロパティへの自動バインディング
- グリフ単位のエフェクト適用（opacity, is_visible, offset）
- タイプライター効果（シーケンシャル表示）の dola Storyboard 実装
- dola_sync_system による毎フレーム同期
- balloon03-content のグリフパイプラインが前提
