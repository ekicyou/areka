# 要件定義: wintf-dcomp-migration-4-dcomp-removal

## 導入

Phase 4 — DComp コード削除・クリーンアップ。Phase 1〜3 で DComp→ULW（UpdateLayeredWindow）の完全移行が完了した後、残存する DComp 関連コード一式を削除し、コードベースをクリーンな状態にする最終フェーズである。

### 背景

親仕様 `wintf-dcomp-to-layered-migration` の Req 2.5 により、Phase 4 完了時に DComp 関連コードの完全除去が要求されている。現時点で `crates/wintf/src/` 配下に約 69 箇所の `IDComposition` 参照が残存しており、以下のファイル群が削除・クリーンアップの対象となる：

- `com/dcomp.rs`（約315行）— DComp API 拡張 trait 群
- `ecs/graphics/visual_manager.rs` — DComp Visual 階層管理
- `ecs/graphics/components.rs` — DComp COM 型を保持するコンポーネント群
- `ecs/graphics/systems.rs` — DComp パイプラインのシステム関数群
- `ecs/graphics/core.rs` — GraphicsCore の DComp デバイスフィールド
- `examples/dcomp_demo.rs` — DComp 専用デモ

### 前提条件

- Phase 1（D2D1 合成スタック構築）が完了していること
- Phase 2（DComp→D2D1 パイプライン切り替え）が完了していること
- Phase 3（UpdateLayeredWindow 統合）が完了していること
- 上記 3 フェーズ完了により、DComp コードが実行時に一切使用されない状態であること

---

## 要件一覧

### Requirement 1: com/dcomp.rs モジュール完全削除

**Objective:** 開発者として、使われなくなった DComp COM ラッパーモジュールを完全に除去し、COM レイヤーの見通しを改善したい。

_Parent: Req 1.1, 1.2_

#### Acceptance Criteria

1. The wintf crate shall `com/dcomp.rs` ファイル（約315行、DComp 拡張 trait 群: `DCompDevice`, `DCompDesktop`, `DCompTarget`, `DCompVisual`, `DCompSurface`, `DCompAnimation`, `DCompTransform` 等）を完全に削除する
2. The wintf crate shall `com/mod.rs` から `pub mod dcomp;` 宣言を除去する
3. The wintf crate shall `com/dcomp.rs` 内の trait を `use` していた全ファイルから当該 `use` 文を除去する
4. When `com/dcomp.rs` が削除された時, the wintf crate shall コンパイルエラーがゼロであること

### Requirement 2: DComp ECS コンポーネント削除

**Objective:** 開発者として、DComp COM 型を直接保持する ECS コンポーネント定義を削除し、Phase 1 で導入した新コンポーネントのみの構成にしたい。

_Parent: Req 1.1, 6.3_

#### Acceptance Criteria

1. The wintf crate shall `ecs/graphics/components.rs` から以下の DComp 専用コンポーネントを削除する：
   - `VisualGraphics`（`IDCompositionVisual3` の `inner` / `parent_visual` フィールド保持、`on_remove` フックで `RemoveVisual` 実行）
   - `SurfaceGraphics`（`IDCompositionSurface` 保持）
2. The wintf crate shall `VisualGraphics` / `SurfaceGraphics` を参照する全ての `use` 文、クエリ引数、システム関数パラメータを除去する
3. The wintf crate shall `Visual` コンポーネントの `on_visual_add` フック内で `VisualGraphics::default()` / `SurfaceGraphics::default()` を自動挿入していたロジックを除去する（Phase 1 の新コンポーネント挿入ロジックが代替済みであること）
4. While `SurfaceGraphicsDirty` / `SurfaceCreationStats` が DComp 固有の用途のみで使用されている場合, the wintf crate shall これらのコンポーネントも合わせて削除する（Phase 1 で再利用されている場合は保持）

### Requirement 3: DComp ECS システム関数削除

**Objective:** 開発者として、Phase 2 で world.rs スケジュールから登録解除済みの DComp システム関数の実装コードを削除し、デッドコードを除去したい。

_Parent: Req 1.1, 3.3_

#### Acceptance Criteria

1. The wintf crate shall `ecs/graphics/systems.rs` から以下の DComp パイプラインシステム関数の実装コードを削除する：
   - `visual_hierarchy_sync_system` — DComp Visual 親子階層同期
   - `init_window_graphics` — DComp WindowGraphics 初期化
   - `deferred_surface_creation_system` — DComp Surface 遅延作成
   - `cleanup_surface_on_commandlist_removed` — Surface クリーンアップ
   - `render_surface` — DComp Surface への BeginDraw/EndDraw 描画
   - `visual_property_sync_system` — Visual プロパティ同期（SetOffset, SetOpacity 等）
   - `commit_composition` — `IDCompositionDevice3::Commit()` 呼び出し
   - `create_window_graphics_for_hwnd` — DComp Target 作成ヘルパー
   - `create_surface_for_visual` — DComp Surface 作成ヘルパー
   - `draw_recursive` — 旧描画方式（既にデッドコード）
   - `init_window_visual` — Deprecated 空実装
   - `sync_surface_from_arrangement` — Deprecated デッドコード
   - ※ `visual_resource_management_system` / `window_visual_integration_system` は `visual_manager.rs` 内に定義されており Req 4 のスコープで削除される
2. The wintf crate shall 上記システム関数を `pub` エクスポートしていた場合、`ecs/graphics/mod.rs` の re-export を除去する
3. When 全 DComp システム関数が削除された時, the `ecs/world.rs` shall 当該関数への参照（コメント含む）がゼロであること

### Requirement 4: visual_manager.rs モジュール完全削除

**Objective:** 開発者として、DComp Visual 階層管理専用のリソースマネージャーモジュールを完全に除去したい。

_Parent: Req 1.1, 6.4_

#### Acceptance Criteria

1. The wintf crate shall `ecs/graphics/visual_manager.rs` ファイルを完全に削除する
2. The wintf crate shall `ecs/graphics/mod.rs` から `pub mod visual_manager;` 宣言を除去する
3. The wintf crate shall `visual_manager` モジュール内の型・関数を `use` していた全ファイルから当該 `use` 文を除去する
4. When `visual_manager.rs` が削除された時, the wintf crate shall コンパイルエラーがゼロであること

### Requirement 5: dcomp_demo.rs サンプル削除

**Objective:** 開発者として、DComp API を直接使用するデモプログラムを削除し、example 一覧をクリーンに保ちたい。

_Parent: Req 8.4_

#### Acceptance Criteria

1. The wintf crate shall `examples/dcomp_demo.rs`（DComp カードフリップデモ、`IDCompositionDevice3` / `IDCompositionVisual3` / `IDCompositionSurface` / `IDCompositionRotateTransform3D` 等を使用）を削除する
2. Where `Cargo.toml`（wintf crate）に `dcomp_demo` の `[[example]]` エントリが存在する場合, the wintf crate shall 当該エントリを除去する
3. When `dcomp_demo.rs` が削除された時, the `cargo build --examples` shall 全 example がビルドに成功すること

### Requirement 6: GraphicsCore の DComp フィールド除去

**Objective:** 開発者として、GraphicsCore 構造体から DComp デバイス関連フィールドと初期化コードを除去し、D2D1 デバイス中心のシンプルな構成にしたい。

_Parent: Req 5.1, 5.3_

#### Acceptance Criteria

1. The wintf crate shall `ecs/graphics/core.rs` の `GraphicsCore` 構造体から以下のフィールドを除去する：
   - `desktop: IDCompositionDesktopDevice`
   - `dcomp: IDCompositionDevice3`
2. The wintf crate shall `GraphicsCore` の初期化フローから `DCompositionCreateDevice3` 呼び出しおよび関連する DComp デバイス作成コードを除去する
3. The wintf crate shall `ecs/graphics/core.rs` から `IDComposition` で始まる全ての `use` 文を除去する
4. If `GraphicsCore` の `invalidate()` / 再初期化フローに DComp 再初期化ステップが残っている場合, the wintf crate shall 当該ステップを除去する

### Requirement 7: use 文・参照の網羅的クリーンアップ

**Objective:** 開発者として、コードベース全体から DComp 関連の import 文や型参照の残存をゼロにしたい。

_Parent: Req 5.1, 5.3_

#### Acceptance Criteria

1. The wintf crate shall `ecs/graphics/` 配下の全ファイルから `IDComposition` で始まる型への参照をゼロにする
2. The wintf crate shall `crates/wintf/src/` 配下の全ファイルから `crate::com::dcomp` / `com::dcomp` への参照をゼロにする
3. The wintf crate shall `com/animation.rs` 内の `IDCompositionAnimation` パラメータ参照（約2箇所）を確認し、Phase 1 の新アニメーション方式で代替済みであれば除去する（DComp Animation API が引き続き必要な場合は保持理由を文書化）
4. The wintf crate shall `ecs/world.rs` 内の DComp パイプライン関連コメント（約4箇所）を削除または新パイプラインの記述に更新する
5. The wintf crate shall `windows` クレートの `Cargo.toml` features リストから `Win32_Graphics_DirectComposition` feature を除去する（他の用途で必要な場合は保持）

### Requirement 8: テストコードの修正

**Objective:** 開発者として、DComp 関連テストの修正・削除により全テストがパスする状態にしたい。

_Parent: Req 10.1_

#### Acceptance Criteria

1. The wintf crate shall `ecs/graphics_tests.rs` 内の DComp Visual / Commit 関連テスト（約3箇所の `IDComposition` 参照）を削除または新パイプライン相当のテストに書き換える
2. The wintf crate shall `crates/wintf/tests/` 配下のテストファイルで `VisualGraphics` / `SurfaceGraphics` / `IDComposition` 型を参照するテストを修正または削除する
3. When 全テスト修正が完了した時, the `cargo test` shall 全テストパスすること

### Requirement 9: Phase 4 最終検証基準

**Objective:** 開発者として、DComp コード完全削除後の包括的な品質基準を明確にし、移行完了を宣言できるようにしたい。

_Parent: Req 2.5, 10.1_

#### Acceptance Criteria

1. The `grep -r "IDComposition" crates/wintf/src/` shall ゼロ件を返すこと（ソースコード内に `IDComposition` 参照が皆無）
2. The `grep -r "dcomp" crates/wintf/src/` shall DComp 関連コード参照がゼロであること（ドキュメントコメント内の歴史的言及や移行メモは許容）
3. The `cargo build` shall コンパイルエラーゼロで成功すること
4. The `cargo test` shall 全テストパスすること
5. The `cargo build --examples` shall 全 example ビルドに成功すること
6. The `cargo clippy` shall 新規 warning がゼロであること（DComp 削除に起因する未使用 import / デッドコード警告がないこと）
7. The 削除 shall `com/dcomp.rs`（約315行）+ `visual_manager.rs` + DComp システム関数 + DComp コンポーネント定義 + `dcomp_demo.rs` の合計行数をコードベースから純減させ、削除規模を git diff --stat で確認すること

---

## 要件トレーサビリティ（親仕様 → 子仕様）

| 親要件 | 子仕様要件 | 概要 |
|--------|-----------|------|
| Req 1.1, 1.2 (影響範囲特定・廃止ファイル) | Req 1, 2, 3, 4 | DComp ファイル・コンポーネント・システム・マネージャーの削除 |
| Req 2.5 (Phase 4完了時DComp除去) | Req 9 | 最終検証基準 |
| Req 3.3 (DCompステージ置換) | Req 3 | DComp システム関数削除 |
| Req 5.1, 5.3 (DComp初期化・フィールド除去) | Req 6, 7 | GraphicsCore DComp 除去、use文クリーンアップ |
| Req 6.3 (VisualGraphics/SurfaceGraphics一新) | Req 2 | DComp コンポーネント削除 |
| Req 6.4 (visual_manager置換) | Req 4 | visual_manager.rs 削除 |
| Req 8.4 (dcomp_demo.rs削除) | Req 5 | dcomp_demo.rs 削除 |
| Req 10.1 (検証基準) | Req 8, 9 | テスト修正・最終検証 |
