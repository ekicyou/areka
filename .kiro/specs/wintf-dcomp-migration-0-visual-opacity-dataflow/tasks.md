# Implementation Plan: wintf-dcomp-migration-0-visual-opacity-dataflow

## Tasks

- [ ] 1. Visual コンポーネント API 拡張
- [ ] 1.1 opacity/is_visible の setter/getter メソッド実装
  - `set_opacity()`: 0.0〜1.0 クランプ + 範囲外値の warn ログ出力（`Opacity::validate()` 移植）
  - `clamped_opacity()`: 防御的クランプ読み取り（常に 0.0〜1.0 を返す）
  - `set_visible()`: `is_visible` フィールド設定
  - 既存 `pub opacity: f32` フィールドはそのまま維持（後方互換性）
  - _Requirements: 1.1, 1.2, 2.1_

- [ ] 1.2 Visual API の unit tests
  - `set_opacity()` 正常範囲値・境界値（0.0, 1.0）・範囲外値（-0.1, 1.5）のクランプ検証
  - `clamped_opacity()` が `Opacity::clamped()` と同等の動作をすることを検証
  - `set_visible()` の true/false 設定検証
  - `Visual::default()` が `opacity == 1.0`, `is_visible == true` を保証することを検証
  - _Requirements: 5.1_

- [ ] 2. visual_property_sync_system の Visual 移行
- [ ] 2.1 Opacity 依存削除と Visual クエリへの切り替え
  - クエリから `Option<&crate::ecs::layout::Opacity>` を削除し `&Visual` に置換
  - フィルタから `Changed<crate::ecs::layout::Opacity>` を削除し `Changed<Visual>` に置換
  - Opacity 同期ロジック（L1086-1097）を `visual.clamped_opacity()` 呼び出しに置換
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 2.2 is_visible 対応の非表示ロジック追加
  - `Visual.is_visible = false` のエンティティに対して `SetOpacity(0.0)` を呼び出す分岐を追加
  - 可視エンティティには `SetOpacity(visual.clamped_opacity())` を呼び出す
  - _Requirements: 3.4_

- [ ] 2.3 (P) sync system integration tests
  - `visual_property_sync_system` が `Visual.opacity` を読み取り `SetOpacity()` に正しく渡すことを検証
  - `Visual.is_visible = false` で `SetOpacity(0.0)` が呼ばれることを検証
  - `Changed<Visual>` フィルタが opacity 変更で発火することを検証
  - Widget spawn 時に `Visual { opacity: 0.5, .. }` を指定した場合の sync system 動作を検証
  - spawn 後に `set_opacity()` を呼び出した場合の `Changed<Visual>` 発火と DComp 反映を検証
  - _Requirements: 5.2_

- [ ] 3. hit_test 層の Visual 移行
- [ ] 3.1 (P) hit_test 関数の Opacity → Visual 読み取り変更
  - `hit_test_entity` (L204-207) の Opacity 読み取りを `Visual.clamped_opacity()` に置換
  - `hit_test_entity_ex` (L339-342) の Opacity 読み取りを `Visual.clamped_opacity()` に置換
  - import パス `use crate::ecs::graphics::Visual` を追加
  - フォールバック値 1.0 を維持（`Visual` 未挿入エンティティ対応）
  - _Requirements: 3.5_

- [ ] 3.2 (P) hit_test テストの Visual 移行
  - テスト 6 関数の `Opacity(値)` spawn を `Visual { opacity: 値, ..Default::default() }` に置換
  - `hit_test_entity` が `Visual.opacity` からα判定値を正しく取得することを検証
  - _Requirements: 3.6_

- [ ] 4. (P) Opacity コンポーネント deprecation
  - `Opacity` 構造体に `#[deprecated(since = "0.1.0", note = "Use Visual.opacity instead")]` 属性を付与
  - 既存の impl ブロック（`validate()`, `clamped()`, `Default`）はそのまま維持
  - CI ビルドで deprecation 警告が表示されることを確認
  - _Requirements: 4.1, 4.2_

- [ ] 5. Phase 0 統合検証
- [ ] 5.1 Example コード visual regression 確認
  - `dcomp_demo.rs`, `taffy_flex_demo.rs` 等の既存 Example を手動実行
  - visual regression（描画の意図しない変化）が発生しないことを確認
  - _Requirements: 5.4_

- [ ] 5.2 全テスト実行と回帰確認
  - `cargo test` を実行し全テストがパスすることを確認
  - 既存テストへの回帰（意図しない破壊）がないことを確認
  - deprecation 警告が `Opacity` 使用箇所で表示されることを確認
  - _Requirements: 5.3, 5.5_

## Requirements Coverage

| Requirement | Tasks |
|-------------|-------|
| 1.1 Widget → Visual.opacity 書き込み | 1.1 |
| 1.2 opacity クランプ | 1.1 |
| 1.3 Changed\<Visual\> 変更検出 | （bevy_ecs 自動、1.1 で基盤確立） |
| 2.1 Widget → Visual.is_visible 書き込み | 1.1 |
| 2.2 Changed\<Visual\> is_visible 検出 | （bevy_ecs 自動、1.1 で基盤確立） |
| 2.3 描画システム継続 | （既存動作維持、非破壊的変更） |
| 3.1 sync system Visual.opacity 読み取り | 2.1 |
| 3.2 sync system Changed\<Visual\> | 2.1 |
| 3.3 sync system Opacity 完全切断 | 2.1 |
| 3.4 is_visible → SetOpacity(0.0) | 2.2 |
| 3.5 hit_test Visual.opacity 読み取り | 3.1 |
| 3.6 hit_test テスト移行 | 3.2 |
| 4.1 Opacity deprecated 付与 | 4 |
| 4.2 CI 警告可視化 | 4 |
| 5.1 Visual 書き込み unit test | 1.2 |
| 5.2 sync system integration test | 2.3 |
| 5.3 deprecation warning 確認 | 5.2 |
| 5.4 Example visual regression 確認 | 5.1 |
| 5.5 全テストパス確認 | 5.2 |
