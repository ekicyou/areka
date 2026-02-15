# Research & Design Decisions: wintf-dcomp-migration-0-visual-opacity-dataflow

## Summary
- **Feature**: `wintf-dcomp-migration-0-visual-opacity-dataflow`
- **Discovery Scope**: Extension（既存 ECS システムの拡張）
- **Key Findings**:
  1. Widget on_add フックは既に `DeferredWorld::get::<Visual>()` による既存チェックを実装済み ⇒ 上書き問題は**存在しない**（R1, R2 解消）
  2. `Changed<Visual>` はコードベースで未使用（0件）。`transform_origin` は spawn 時のみ設定の静的フィールド ⇒ 粒度問題は**実質ゼロ**（R3 解消）
  3. `visual_property_sync_system` の Opacity 参照は 3 箇所（クエリ定義 2 + 同期ロジック 1）、hit_test は 2 箇所。全て機械的置換で移行可能

## Research Log

### R1: DeferredWorld::get\<T\>() API 可用性

- **Context**: Widget on_add フック内で `Visual` コンポーネントの既存チェックに必要
- **Sources Consulted**: コードベース内の全 `DeferredWorld` 使用パターン、bevy_ecs 0.18.0 ソース
- **Findings**:
  - `DeferredWorld::get::<T>(entity)` は **完全に利用可能**
  - コードベース内に 10+ 箇所の使用実績あり:
    - `ecs/graphics/components.rs` L268-275: `on_visual_add` 内で 5 つの `get` 呼び出し
    - `ecs/widget/shapes/rectangle.rs` L103: `world.get::<Visual>(hook.entity).is_some()`
    - `ecs/widget/text/label.rs` L54: 同パターン
    - `ecs/widget/text/typewriter.rs` L66-67: `is_none()` チェック
    - `ecs/widget/bitmap_source/bitmap_source.rs` L53-55: 3 つの `get` 呼び出し
    - `ecs/window.rs` L1141: `world.get::<Visual>(entity).is_none()`
  - `get_mut` も利用可能（`rectangle.rs` L115 で実績あり）
- **Implications**: A-2 方式（`DeferredWorld::get` による既存チェック）は確実に動作する。さらに、**既に全ウィジェットで実装済み**のため追加対応不要。

### R2: on_add フック内でのコンポーネント上書き挙動

- **Context**: `.spawn((Visual { opacity: 0.5, .. }, Rectangle::new()))` で Rectangle の on_add が `Visual::default()` を再挿入するか
- **Sources Consulted**: 全 Widget on_add フック実装、bevy_ecs 0.18 の spawn 処理順序
- **Findings**:
  - **bevy_ecs 0.18 の spawn 順序**: バンドル内の全コンポーネントが一括挿入 → 各 on_add フックが順次発火
  - **全 Widget on_add フックに既存チェックあり**:

    | Widget | ファイル | チェック方法 |
    |--------|---------|-------------|
    | Rectangle | `widget/shapes/rectangle.rs` L103 | `world.get::<Visual>(hook.entity).is_some()` → return |
    | Label | `widget/text/label.rs` L54 | 同上 |
    | Typewriter | `widget/text/typewriter.rs` L66 | `world.get::<Visual>(entity).is_none()` → insert |
    | BitmapSource | `widget/bitmap_source/bitmap_source.rs` L53 | `world.get::<Visual>(entity).is_some()` で分岐 |
    | Window | `window.rs` L1141 | `world.get::<Visual>(entity).is_none()` → insert |

  - **テストによる裏付け**: `widget_visual_auto_insert_test.rs` の `test_label_with_existing_visual` テストがカスタム Visual 保持を検証
  - **Caveat**: `visual_manager.rs` の `insert_visual()` ヘルパーは既存チェックなしで直接 `entity_mut.insert(visual)` するが、Widget on_add からは使用されていない

- **Implications**: **Widget on_add 競合問題はリスクではなく、既に解決済み**。gap-analysis の「高」リスクは「解消」に格下げ。

### R3: Changed\<Visual\> の粒度問題

- **Context**: `Visual` には `opacity`, `is_visible`, `transform_origin` の 3 フィールドがあり、`Changed<Visual>` は全フィールド変更で発火する
- **Sources Consulted**: コードベース内の `Changed<>` 使用パターン、`transform_origin` 変更箇所
- **Findings**:
  - `Changed<Visual>` の現在の使用: **0 件**（コードベースに存在しない）
  - `transform_origin` の動的変更: **実質ゼロ**（テストでのカスタム設定のみ。アニメーションやランタイム変更なし）
  - 既存の粒度対策パターン: `BoxStyle` の座標分離（`boxstyle_coordinate_separation_test.rs`）
  - `visual_property_sync_system` の現在のフィルタ: `Changed<Arrangement> | Changed<GlobalArrangement> | Changed<Opacity>`
- **Implications**: `Changed<Visual>` を追加しても、`transform_origin` による不要発火は実質的に発生しない。値差分チェックは不要。将来 `transform_origin` がアニメーション対象になった場合はフィールド分離を検討。

## Architecture Pattern Evaluation

| Option | 概要 | 強み | リスク・限界 | 備考 |
|--------|------|------|-------------|------|
| **A: 直接拡張** | `Visual` にメソッド追加 + sync system クエリ修正 | 最小変更、既存パターン活用 | Widget on_add 競合（→**解消済み**） | 推奨 |
| B: ブリッジ | `opacity_bridge_system` で `Opacity` → `Visual.opacity` 同期 | 互換性完全維持 | 間接層の複雑度、毎フレーム同期コスト | 却下 |
| **C: ハイブリッド** | A ベース + hit_test 移行を Phase 0 に含む | データ不整合リスク解消 | Phase 0 スコープ微増 | **採用** |

**採用理由**: R2 の調査で Widget on_add 競合が解消済みと判明したため、Option A と C の差は hit_test 移行のみ。要件定義フェーズの Q2 判断で hit_test を Phase 0 に含めることが決定済みのため、Option C を採用。

## Design Decisions

### Decision: Visual メソッド API 設計

- **Context**: `Visual` に opacity 操作メソッドを追加する必要がある（Req 1.2）
- **Alternatives Considered**:
  1. `set_opacity(f32)` — setter のみ（内部クランプ）
  2. `set_opacity(f32)` + `clamped_opacity()` — setter + getter（`Opacity::clamped()` 相当）
  3. フィールド直接アクセスのみ（メソッド追加なし）
- **Selected Approach**: Option 2 — `set_opacity()` + `clamped_opacity()` + `set_visible()`
- **Rationale**:
  - `set_opacity` はクランプロジックをカプセル化し、不正値を防止
  - `clamped_opacity` は既存 `Opacity::clamped()` の移行先として必要
  - `set_visible` は `is_visible` の setter（将来のバリデーション拡張に備える）
  - フィールド direct access は `pub` のまま維持（既存コードとの互換性）
- **Trade-offs**: メソッド追加によるメンテナンスコスト（低）vs API の安全性向上（高）
- **Follow-up**: `Opacity::validate()` のログ出力を `set_opacity()` に移植するか設計フェーズで決定

### Decision: visual_property_sync_system の変更検出方式

- **Context**: Opacity 分離コンポーネント → Visual 統合フィールドへの変更検出移行
- **Alternatives Considered**:
  1. `Changed<Visual>` をフィルタに追加（単純置換）
  2. `Changed<Visual>` + 値差分チェック（前回値キャッシュ）
  3. 新規 `OpacityDirty` マーカーコンポーネント
- **Selected Approach**: Option 1 — `Changed<Visual>` 単純置換
- **Rationale**:
  - R3 の調査で `transform_origin` の動的変更は実質ゼロ → 過剰発火なし
  - 値差分チェックはキャッシュ用コンポーネント追加が必要で複雑度が増す
  - マーカーコンポーネントは手動管理が煩雑
- **Trade-offs**: 将来 `transform_origin` がアニメーション対象になると過剰発火のリスク（低）
- **Follow-up**: Phase 1 以降で `transform_origin` が動的変更される場合、フィールド分離を再検討

### Decision: hit_test の Opacity 読み取り移行

- **Context**: `hit_test_entity` / `hit_test_entity_ex` が `world.get::<Opacity>()` で透明度を取得
- **Alternatives Considered**:
  1. `world.get::<Visual>(entity).map(|v| v.clamped_opacity())` — 新メソッド使用
  2. `world.get::<Visual>(entity).map(|v| v.opacity.clamp(0.0, 1.0))` — インライン
  3. 据え置き（Phase 2 延期）
- **Selected Approach**: Option 1 — `clamped_opacity()` メソッド使用
- **Rationale**: クランプロジックの重複を避け、`Visual` のメソッドに統一
- **Trade-offs**: `Visual` コンポーネントが hit_test からの読み取り対象になる（モジュール境界跨ぎ）が、既存の `world.get::<Opacity>()` も同様のパターン

## Risks & Mitigations

- ~~Widget on_add 上書きリスク（高）~~ → **解消**: 全フックに既存チェック実装済み（R2）
- ~~Changed\<Visual\> 過剰発火（低）~~ → **許容**: transform_origin の動的変更は実質ゼロ（R3）
- ~~hit_test データ不整合（中）~~ → **解消**: Phase 0 で hit_test も移行（Q2 判断）
- Example 移行時の visual regression（低） → `Opacity(0.5)` → `Visual { opacity: 0.5, .. }` は等価な値変更。手動実行で確認
- `visual_manager.rs` の `insert_visual()` ヘルパー（低） → Widget on_add からは使用されていない。将来の呼び出し元で注意

## References
- bevy_ecs 0.18.0 — プロジェクト内 `Cargo.lock` で確認（L329-330）
- `Opacity::clamped()` 実装 — `crates/wintf/src/ecs/layout/metrics.rs` L121-123
- Widget on_add パターン — `crates/wintf/tests/widget_visual_auto_insert_test.rs`
- BoxStyle 座標分離テスト — `crates/wintf/tests/boxstyle_coordinate_separation_test.rs`
