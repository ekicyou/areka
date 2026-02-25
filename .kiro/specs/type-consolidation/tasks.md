# Implementation Plan

## Task Progress

- **Total Major Tasks**: 7
- **Total Sub-Tasks**: 10
- **Requirements Coverage**: All 7 requirements (1-7) mapped

## Task List

- [ ] 1. 共通型モジュールの作成と基本型定義
- [ ] 1.1 ecs/types.rs の作成と6つの幾何型定義
  - Point, PointF, Size, SizeI, Offset, Rect の構造体を `#[repr(C)]` で定義する
  - Size と Offset には `Component` derive を追加する（ECS コンポーネントフィールドとして使用されるため）
  - Point, PointF, SizeI, Rect は pure primitive 型として Component derive なし
  - 全型に `Debug, Clone, Copy, Default, PartialEq` の基本 derive を適用する（整数型には `Eq` も追加）
  - _Requirements: 1.1, 1.3, 2.1, 2.2, 3.1, 3.2, 3.3, 4.2_

- [ ] 1.2 Win32/D2D1 型との From/Into 変換実装
  - Point ↔ POINT（フィールド名完全一致）、PointF ↔ Vector2（X/Y ↔ x/y マッピング）、Size ↔ D2D_SIZE_F（完全一致）、SizeI ↔ SIZE（cx/cy ↔ width/height マッピング）、Rect ↔ D2D_RECT_F（完全一致）の変換を実装する
  - メモリレイアウト互換により実質ゼロコスト変換となることを確認する
  - 各変換のラウンドトリップテストを作成する（値が保存されることの検証）
  - `std::mem::size_of` と `std::mem::align_of` が外部型と一致することを検証するテストを追加する
  - _Requirements: 2.3, 3.1, 3.3, 4.2, 6.3, 6.5_

- [ ] 1.3 ecs/mod.rs での re-export 設定と単体テスト作成
  - `pub use types::{Point, PointF, Size, SizeI, Offset, Rect};` を追加し、既存の `pub use layout::*` パターンと共存させる
  - Default 値が期待通り（全フィールド 0/0.0）であることを検証するテストを追加する
  - PartialEq が同一フィールド値で等値判定することを確認するテストを追加する
  - `cargo build` と `cargo test` で基本動作を確認する
  - _Requirements: 1.2, 7.1_

- [ ] 2.1 (P) pointer/types.rs の PhysicalPoint → Point 置換
  - `PhysicalPoint` 構造体定義を削除し、`use crate::ecs::types::Point;` に置換する
  - `PointerState` 等のフィールド型を `PhysicalPoint` → `Point` に変更する
  - 後方互換のため `pub type PhysicalPoint = Point;` エイリアスを追加する
  - `cargo build` と関連テストでコンパイルを確認する
  - _Requirements: 2.4, 2.6, 7.2, 7.5_

- [ ] 2.2 (P) hit_test/mod.rs の PhysicalPoint → PointF 置換
  - `PhysicalPoint` 構造体定義を削除し、`use crate::ecs::types::PointF;` に置換する
  - `hit_test()`, `hit_test_in_window()` 等の引数型を `PointF` に変更する
  - `mouse_move.rs` 等の `PhysicalPoint as HitTestPoint` エイリアスを `PointF as HitTestPoint`（または直接 `PointF` 使用）に変更する
  - `cargo build` と関連テストでコンパイルを確認する
  - _Requirements: 2.5, 2.6_

- [ ] 3.1 (P) layout/metrics.rs の Size/Offset re-export 置換
  - `Size` と `Offset` の構造体定義を削除する
  - `pub use crate::ecs::types::{Size, Offset};` に置き換える
  - `LayoutScale` と `TextLayoutMetrics` は metrics.rs 内に維持する（レイアウト専用型）
  - `Arrangement`, `GlobalArrangement` 等のコンポーネントが引き続き Size/Offset を使用できることを確認する
  - `cargo build` と `tests/layout.rs` で動作を確認する
  - _Requirements: 3.4, 3.5, 3.6, 7.2_

- [ ] 4.1 (P) layout/rect.rs の D2DRect alias と D2DRectExt 移行
  - `pub type D2DRect = Rect;` に変更する（`D2D_RECT_F` → `Rect` への型ターゲット変更）
  - `D2DRectExt` トレイトの実装対象を `D2D_RECT_F` から `Rect` に変更する
  - `offset()` / `size()` の戻り値型を `Vector2` → `PointF` / `Size` に変更する
  - `set_offset()` / `set_size()` の引数型を `Vector2` → `PointF` / `Size` に変更する（または `impl Into<PointF>` で互換維持）
  - `transform_rect_axis_aligned` の引数・戻り値を `&Rect` / `Rect` に変更する
  - `tests/layout/arrangement_bounds_test.rs` の 4 箇所（L127, L141, L153, L166）の Vector2 参照を PointF/Size に修正する
  - `cargo build` と関連テストで動作を確認する
  - _Requirements: 4.3, 4.4, 4.6_

- [ ] 5.1 (P) window/window_pos.rs の POINT/SIZE → Point/SizeI 置換
  - `position: Option<POINT>` → `position: Option<Point>` に変更する
  - `size: Option<SIZE>` → `size: Option<SizeI>` に変更する
  - Win32 API 呼び出し箇所（`set_window_pos()` 等）で `.into()` による変換を追加する
  - `POINT { x: 0, y: 0 }` リテラルを `Point { x: 0, y: 0 }` に置換する
  - `SIZE { cx: ..., cy: ... }` リテラルを `SizeI { width: ..., height: ... }` に置換する
  - `size.cx == CW_USEDEFAULT` を `size.width == CW_USEDEFAULT` に変更する
  - `cargo build` と `tests/window.rs` で動作を確認する
  - _Requirements: 6.1, 6.2_

- [ ] 6.1 (P) transform/components.rs に #[deprecated] 属性追加
  - `Transform`, `GlobalTransform`, `Translate`, `Scale`, `Rotate`, `Skew`, `TransformOrigin` に `#[deprecated]` 属性を追加する
  - 非推奨メッセージで代替手段（`Arrangement` ベースのレイアウトシステム）を案内する
  - 型の定義自体は変更しない（既存の互換性を維持）
  - `cargo build` でコンパイルを確認する
  - _Requirements: 5.1, 5.2_

- [ ] 7. 統合テストと検証
- [ ] 7.1 全テストスイートの実行と修正
  - `cargo test` を実行し、全テストがパスすることを確認する
  - 失敗したテストについて、型パス変更漏れや型名変更による影響を修正する
  - 特に `tests/ecs.rs`, `tests/layout.rs`, `tests/visual.rs`, `tests/widget.rs`, `tests/window.rs` を重点的に検証する
  - _Requirements: 7.3_

- [ ] 7.2 全サンプルのビルドと動作確認
  - `cargo build --examples` で全サンプルがコンパイル通過することを確認する
  - 主要なサンプル（`examples/areka.rs`, `examples/dcomp_demo.rs` 等）を実行し、動作に異常がないことを確認する
  - コンパイルエラーがあれば、型パスや型名の変更漏れを修正する
  - _Requirements: 7.4_

- [ ] 7.3 後方互換性の検証
  - 既存の型パス（`use crate::ecs::Size` 等）が引き続き動作することを確認する
  - `pub use` re-export が正しく機能していることを確認する
  - 型エイリアス（`D2DRect`, `PhysicalPoint`）が正しく機能することを確認する
  - `cargo build` と `cargo test` の最終全パスを確認する
  - _Requirements: 7.1, 7.2, 7.5_

## Requirements Coverage Matrix

| Requirement | Acceptance Criteria | Task(s)      |
| ----------- | ------------------- | ------------ |
| 1           | 1.1, 1.2, 1.3       | 1.1, 1.3     |
| 2           | 2.1-2.6             | 1.1, 1.2, 2.1, 2.2 |
| 3           | 3.1-3.6             | 1.1, 1.2, 3.1 |
| 4           | 4.1-4.6             | 1.1, 1.2, 4.1 |
| 5           | 5.1, 5.2            | 6.1          |
| 6           | 6.1-6.5             | 1.2, 5.1     |
| 7           | 7.1-7.5             | 1.3, 2.1, 3.1, 7.1, 7.2, 7.3 |

## Implementation Notes

### Execution Order
1. Task 1 を順次実行（1.1 → 1.2 → 1.3）し、共通型モジュールの基盤を確立する
2. Task 1 完了後、Task 2-6 は並行実行可能（`(P)` マークあり）
3. Task 7 は全タスク完了後に実行し、統合検証を行う

### Parallel Execution
- Task 2.1 と 2.2: 異なるモジュール（pointer, hit_test）への変更で依存なし
- Task 3.1: layout/metrics.rs のみへの変更
- Task 4.1: layout/rect.rs のみへの変更
- Task 5.1: window/window_pos.rs のみへの変更
- Task 6.1: transform/components.rs のみへの変更

### Critical Path
Task 1 → (Task 2-6 並行) → Task 7

### Rollback Strategy
- 各 Phase（Major Task）の完了条件: `cargo build` + `cargo test` が全パスすること
- Phase 内で解消不能なコンパイルエラーが発生した場合、`git revert` で Phase 単位でロールバックする
