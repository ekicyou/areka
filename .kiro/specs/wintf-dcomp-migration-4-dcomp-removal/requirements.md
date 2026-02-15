# 要件定義: wintf-dcomp-migration-4-dcomp-removal

## 概要

Phase 4 — DComp コード削除・クリーンアップ。Phase 1-3 で DComp→ULW の完全移行が完了した後、残存する DComp 関連コード（com/dcomp.rs、DComp コンポーネント、DComp システム関数、visual_manager.rs、dcomp_demo.rs）を削除し、コードベースをクリーンな状態にする。

---

## 要件一覧

### Requirement 1: com/dcomp.rs ファイル削除

**Objective:** 開発者として、使われなくなった DComp COM ラッパーファイルを削除したい。

_Parent: Req 1.1_

#### Acceptance Criteria

1. The `com/dcomp.rs` ファイル（約315行）shall 完全に削除される
2. The `com/mod.rs`（または lib.rs の com モジュール宣言）shall `dcomp` モジュールの `mod` 宣言を除去する
3. The 削除後 shall コンパイルエラーがゼロであること

### Requirement 2: DComp ECS コンポーネント削除

**Objective:** 開発者として、DComp 専用の ECS コンポーネント定義を削除したい。

_Parent: Req 1.1_

#### Acceptance Criteria

1. The `ecs/graphics/components.rs` shall 以下のコンポーネントを削除する：
   - `VisualGraphics`（IDCompositionVisual3 保持）
   - `SurfaceGraphics`（IDCompositionSurface 保持）
   - `SurfaceGraphicsDirty`
   - `SurfaceCreationStats`
2. The 削除後 shall これらのコンポーネント型を参照するコードがゼロであること

### Requirement 3: DComp ECS システム関数削除

**Objective:** 開発者として、DComp 専用の ECS システム関数のコードを削除したい。

_Parent: Req 1.1_

#### Acceptance Criteria

1. The `ecs/graphics/systems.rs` shall research.md で RED 分類された以下のシステム関数のコードを削除する：
   - `visual_resource_management_system`
   - `visual_hierarchy_sync_system`
   - `init_window_graphics`
   - `window_visual_integration_system`
   - `deferred_surface_creation_system`
   - `cleanup_surface_on_commandlist_removed`
   - `render_surface`
   - `visual_property_sync_system`
   - `commit_composition`
2. The 削除後 shall Phase 2 で world.rs から登録解除されたシステム関数の実装コードが存在しないこと
3. The 必要に応じて shall systems.rs からエクスポートされていた関数シグネチャの use 文を他のファイルからも除去する

### Requirement 4: visual_manager.rs ファイル削除

**Objective:** 開発者として、DComp 固有のリソースマネージャーファイルを削除したい。

_Parent: Req 1.1_

#### Acceptance Criteria

1. The `ecs/graphics/visual_manager.rs` ファイル（約170行）shall 完全に削除される
2. The `ecs/graphics/mod.rs` shall `visual_manager` モジュールの `mod` 宣言を除去する
3. The 他のファイルから shall `visual_manager` への参照がゼロであること

### Requirement 5: dcomp_demo.rs 削除

**Objective:** 開発者として、DComp API を直接使用するデモプログラムを削除したい。

_Parent: Req 8.4_

#### Acceptance Criteria

1. The `examples/dcomp_demo.rs` shall 削除される
2. The `Cargo.toml`（wintf crate）shall `[[example]]` セクションから `dcomp_demo` エントリを除去する（存在する場合）
3. The `cargo build --examples` shall 全 example がビルドに成功すること

### Requirement 6: use 文・参照の最終クリーンアップ

**Objective:** 開発者として、DComp 関連の import 文や型参照の残存をゼロにしたい。

_Parent: Req 5.1_

#### Acceptance Criteria

1. The `ecs/graphics/core.rs` shall DComp 関連の `use` 文を全て除去する
2. The `ecs/graphics/` 配下の全ファイル shall `IDComposition` で始まる型への参照がゼロであること
3. The `crates/wintf/src/` 配下の全ファイル shall `dcomp` モジュールへの参照がゼロであること

### Requirement 7: テストファイルの修正

**Objective:** 開発者として、DComp 関連テストの修正で全テストがパスする状態にしたい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The `crates/wintf/tests/` 配下のテストファイル shall DComp コンポーネント・システムを参照するテストを修正または削除する
2. The `cargo test` shall 全テストパスすること
3. The テスト修正 shall コンポーネント型の変更（VisualGraphics → なし等）に追従する

### Requirement 8: Phase 4 最終検証基準

**Objective:** 開発者として、DComp コード完全削除後の品質基準を明確にしたい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The `grep -r "IDComposition" crates/wintf/src/` shall ゼロ件を返すこと
2. The `grep -r "dcomp" crates/wintf/src/` shall DComp 関連コード参照がゼロであること（コメントやドキュメント内の言及は許容）
3. The `cargo test` shall 全テストパスすること
4. The `cargo build --examples` shall 全 example ビルドに成功すること
5. The `cargo clippy` shall 新規 warning がゼロであること
6. The 削除行数 shall com/dcomp.rs（約315行）+ visual_manager.rs（約170行）+ REDシステム関数 + DCompコンポーネント定義を合計し、コードベースの純減を確認すること

---

## 要件トレーサビリティ（親仕様 → 子仕様）

| 親要件 | 子仕様要件 |
|--------|-----------|
| Req 1.1 (影響範囲特定) | Req 1, 2, 3, 4 |
| Req 5.1 (DComp初期化除去) | Req 6 |
| Req 8.4 (dcomp_demo.rs削除) | Req 5 |
| Req 10.1 (検証基準) | Req 7, 8 |
