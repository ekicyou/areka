# 要件定義書: wintf-dcomp-migration-0-visual-opacity-dataflow

## 導入

本子仕様は親仕様 `wintf-dcomp-to-layered-migration` の Phase 0「Visual 透明度データフロー確立」を担当する。DComp → Layered Window 移行において、Phase 1 以降の D2D1 合成描画パイプライン（`composite_render_system`）が `Visual.opacity` / `Visual.is_visible` を正しく読み取れるよう、Widget 層からこれらフィールドへのデータフローを確立する。

### コンテキスト

現行パイプラインでは Widget が `Opacity` コンポーネント（`ecs/layout/metrics.rs`）を設定し、`visual_property_sync_system`（`ecs/graphics/systems.rs`）が `Option<&Opacity>` を読み取って `IDCompositionVisual3::SetOpacity()` を呼び出している。変更検出は `Changed<Opacity>` で行われる。

一方、`Visual` コンポーネント（`ecs/graphics/components.rs` L249-L254）には `opacity: f32` / `is_visible: bool` フィールドが既に存在するが、Widget コードはこれらフィールドを一切書き込んでおらず、常にデフォルト値（`opacity: 1.0`, `is_visible: true`）のままである。`visual_property_sync_system` のクエリにも `Visual` コンポーネントは含まれていない。

Phase 1 では `composite_render_system` が `Visual.opacity` を読んで階層的な opacity 累積描画を行う設計だが、現状では Widget → `Visual.opacity` への書き込みパスが存在しないため、全エンティティの opacity が 1.0 として処理されてしまう。

本仕様は、Widget 層から `Visual.opacity` / `Visual.is_visible` への書き込みを確立し、段階的に `Opacity` コンポーネントを廃止する方針を策定する。これにより、DComp パイプライン（`visual_property_sync_system`）と D2D1 パイプライン（`composite_render_system`）の両方が同一データソース（`Visual` コンポーネント）から透明度情報を取得できるようになる。

### 本子仕様のスコープ

- `Visual.opacity` / `Visual.is_visible` への Widget 層書き込みパス実装
- `visual_property_sync_system` の `Visual` フィールド読み取りへの移行
- `Opacity` コンポーネント（metrics.rs）の deprecation マーキング
- Phase 1 合成描画システムが依存する Visual データフローの事前整備

### Non-Goals

- `Opacity` コンポーネントの即座削除（deprecation 期間を設ける）
- Widget API の破壊的変更（既存コードの互換性維持）
- Layout 層（`Arrangement`, `GlobalArrangement`）への opacity フィールド追加（Visual 層で十分）
- `composite_render_system` の実装（Phase 1 で実施）

---

## Requirements

### Requirement 1: Visual.opacity データフロー確立

**Objective:** 開発者として、Widget 層から `Visual.opacity` へ透明度を書き込むデータフローが欲しい。これにより Phase 1 の D2D1 合成描画システムが正しく opacity を読み取れる。

_Parent: wintf-dcomp-to-layered-migration Req 2.1（フェーズ0 定義）, Req 6.2（Visual コンポーネント設計）_

#### Acceptance Criteria

1. When Widget が透明度を設定する, the wintf shall その値を `Visual.opacity` フィールドに直接書き込む

2. The wintf shall `Visual.opacity` フィールドの値を 0.0（完全透明）から 1.0（完全不透明）の範囲に自動クランプする

3. When `Opacity` コンポーネント（metrics.rs）が存在しないエンティティに対して `Visual.opacity` が設定される, the wintf shall `Opacity` コンポーネントへの依存なく正常に動作する

4. The wintf shall `Visual.opacity` のデフォルト値として 1.0（完全不透明）を維持する（既存コードの互換性保証）

5. When `Visual.opacity` が変更される, the wintf shall bevy_ecs の `Changed<Visual>` クエリで変更検出可能にする

### Requirement 2: Visual.is_visible データフロー確立

**Objective:** 開発者として、Widget 層から `Visual.is_visible` へ可視性フラグを書き込むデータフローが欲しい。これにより Phase 1 の D2D1 合成描画システムが非表示エンティティをスキップできる。

_Parent: wintf-dcomp-to-layered-migration Req 2.1（フェーズ0 定義）, Req 6.2（Visual コンポーネント設計）_

#### Acceptance Criteria

1. When Widget が可視性を変更する, the wintf shall その値を `Visual.is_visible` フィールドに直接書き込む

2. The wintf shall `Visual.is_visible` フィールドを `bool` 型として明確な true/false 値で保持する

3. The wintf shall `Visual.is_visible` のデフォルト値として `true` を維持する（既存コードの互換性保証）

4. When `Visual.is_visible` が変更される, the wintf shall bevy_ecs の `Changed<Visual>` クエリで変更検出可能にする

5. The wintf shall `Visual.is_visible = false` のエンティティに対しても、既存の描画システム（`draw_rectangles`, `draw_labels` 等）で GraphicsCommandList 生成を継続する（描画スキップ判定は合成システム側の責務）

### Requirement 3: visual_property_sync_system の移行

**Objective:** 開発者として、既存の DComp パイプライン（`visual_property_sync_system`）が `Opacity` コンポーネントではなく `Visual.opacity` フィールドを読むように移行したい。これにより DComp/D2D1 両パイプラインが同一データソースを共有できる。

_Parent: wintf-dcomp-to-layered-migration Req 2.1（フェーズ0 定義）, Req 6.2（Visual コンポーネント設計）_

#### Acceptance Criteria

1. The `visual_property_sync_system` shall `Visual.opacity` フィールドを読み取り、`IDCompositionVisual3::SetOpacity()` に渡す

2. The `visual_property_sync_system` shall `Changed<Visual>` クエリで opacity 変更を検出し、DComp 同期を実行する

3. The `visual_property_sync_system` shall クエリから `Option<&Opacity>` および `Changed<crate::ecs::layout::Opacity>` を削除し、`Visual` のみをデータソース兼変更検出対象とする（完全切断方式：`Opacity` コンポーネントは一切参照しない）

4. When `Visual.is_visible = false` のエンティティを処理する, the `visual_property_sync_system` shall `SetOpacity(0.0)` を呼び出すことで非表示を実現する（DComp は is_visible 概念を持たないため）

### Requirement 4: Opacity コンポーネント廃止方針

**Objective:** 開発者として、`Opacity` コンポーネント（metrics.rs）を段階的に廃止する明確な方針とタイムラインが欲しい。これにより重複したデータソースを整理し、コードベースを簡素化できる。

_Parent: wintf-dcomp-to-layered-migration Req 2.1（フェーズ0 定義）_

#### Acceptance Criteria

1. The wintf shall `Opacity` コンポーネントに `#[deprecated]` 属性を付与し、deprecation メッセージに「`Visual.opacity` フィールドを使用してください」を含める

2. The wintf shall Phase 1 完了まで `Opacity` コンポーネントの存在を許容する（互換性維持期間）

3. The wintf shall Phase 2 開始時に `Opacity` コンポーネントを使用する全コード（Layout 層の `hit_test.rs`、Widget 実装含む）から参照を削除する

4. The wintf shall Phase 3 開始前に `Opacity` コンポーネント定義を `ecs/layout/metrics.rs` から完全削除する

5. When `Opacity` コンポーネント削除が完了する, the wintf shall `cargo build` にコンパイルエラーが発生しないことを保証する（全参照削除済みの検証）

### Requirement 5: Phase 0 検証基準

**Objective:** 開発者として、Phase 0 の完了を客観的に判定できる検証基準が欲しい。

_Parent: wintf-dcomp-to-layered-migration Req 10.1, 10.2_

#### Acceptance Criteria

1. The wintf shall `Visual.opacity` および `Visual.is_visible` への書き込みが正常動作することを unit test で検証する

2. The `visual_property_sync_system` shall `Visual.opacity` フィールドを読み取り `SetOpacity()` に正しく渡すことを integration test で検証する

3. When `Opacity` コンポーネントに `#[deprecated]` が付与される, the wintf shall コンパイル時に deprecation warning が表示されることを確認する

4. The wintf shall 既存の全 example（`dcomp_demo.rs`, `taffy_flex_demo.rs` 等）が正常動作し、visual regression が発生しないことを手動検証する

5. The wintf shall `cargo test` で全テストがパスし、既存テストへの回帰がないこと

---

## 要件カバレッジサマリー

| 子仕様要件 | 親要件 | 概要 |
|-----------|--------|------|
| Req 1 | 2.1, 6.2 | Visual.opacity データフロー確立 |
| Req 2 | 2.1, 6.2 | Visual.is_visible データフロー確立 |
| Req 3 | 2.1, 6.2 | visual_property_sync_system の移行 |
| Req 4 | 2.1 | Opacity コンポーネント廃止方針 |
| Req 5 | 10.1, 10.2 | Phase 0 検証基準 |
