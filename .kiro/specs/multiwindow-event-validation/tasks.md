# Implementation Plan

## タスク概要

全7つの主要タスク、20サブタスクで構成。全13要件（1.1-1.4, 2.1-2.3, 3.1-3.2※, 4.1-4.3, 5.1-5.3）をカバー。

※ 3.2 は既存実装で充足済み（`DragState::Dragging` の hwnd フィールド使用）、タスク不要。

## 実装順序

1. 基盤（Task 1）完了後、Task 2-5 が並列実行可能
2. コア実装（Task 1-5）完了後、Task 6-7 が並列実行可能

---

## Tasks

- [ ] 1. find_owner_window ユーティリティ実装
- [ ] 1.1 ChildOf 逆走査ロジックの実装
  - エンティティから ChildOf チェーンを辿り、Window コンポーネントを持つ最初の祖先を返す関数を実装
  - エンティティ自身が Window の場合は自身を返す
  - ChildOf を持たないエンティティに到達した場合は None を返す
  - `pub(crate) fn find_owner_window(world: &World, entity: Entity) -> Option<Entity>` シグネチャで `ecs/window.rs` に実装
  - _Requirements: 2.1, 2.2, 2.3, 4.1, 4.2, 4.3_

- [ ] 1.2 find_owner_window のユニットテスト
  - 2つの Window エンティティとそれぞれの子ウィジェットを手動で構築する Pure ECS テスト
  - 各ウィジェットの find_owner_window が正しい Window を返すことを検証
  - LayoutRoot 直下のエンティティが None を返すことを検証
  - _Requirements: 5.1_

---

- [ ] 2. (P) build_bubble_path の Window 境界停止修正
- [ ] 2.1 (P) Window コンポーネント検出ロジック追加
  - 既存の `build_bubble_path` 関数（`ecs/pointer/dispatch.rs`）のループ内に Window 停止条件を追加
  - ChildOf 逆走査中、親エンティティが Window コンポーネントを持つ場合はそこで停止
  - Window エンティティ自身はパスに含める
  - 関数シグネチャ変更なし（内部ロジックのみ修正）
  - _Requirements: 4.1, 4.2, 4.3_

- [ ] 2.2* (P) build_bubble_path テストケース追加
  - LayoutRoot → Window → Container → Widget の階層を Pure ECS で構築
  - Widget からの bubble path が Window で終了することを検証
  - LayoutRoot が含まれないことを確認
  - _Requirements: 4.1_

---

- [ ] 3. (P) WM_MOUSELEAVE のウィンドウスコーピング修正
- [ ] 3.1 (P) PointerState クリアのスコープフィルタ追加
  - `handlers.rs` の WM_MOUSELEAVE ハンドラ（L829-849 付近）を修正
  - PointerState を持つ全エンティティをクエリした後、各エンティティに対して `find_owner_window` を呼び出し
  - 返値が当該ウィンドウと一致するエンティティのみから PointerState を削除、PointerLeave を付与
  - 他ウィンドウのエンティティはスキップ
  - _Requirements: 2.1, 2.2, 2.3_

- [ ] 3.2 (P) thread_local バッファのスコープ付きクリア
  - POINTER_BUFFERS 等の thread_local バッファクリアにも同じウィンドウフィルタを適用
  - 当該ウィンドウに属するエンティティのバッファエントリのみを削除
  - _Requirements: 2.3_

---

- [ ] 4. (P) WM_MOUSEMOVE の leave 処理スコーピング修正
- [ ] 4.1 (P) collect_entities_to_leave ヘルパー関数実装
  - `handlers.rs` に private 関数 `collect_entities_to_leave(world: &World, window_entity: Entity, exclude: Entity) -> Vec<Entity>` を実装
  - PointerState を持つ全エンティティをクエリ
  - 各エンティティに対して `find_owner_window` を呼び、window_entity と一致し、かつ exclude と異なるエンティティのみを収集
  - _Requirements: 2.3_

- [ ] 4.2 (P) WM_MOUSEMOVE ハンドラの2箇所修正
  - ヒット成功分岐（L671-686 付近）とヒット失敗分岐（L732-740 付近）の `entities_to_leave` 収集箇所を `collect_entities_to_leave` 呼び出しに置き換え
  - 既存の target_entity との比較ロジックは維持
  - コード重複を排除
  - _Requirements: 2.3_

---

- [ ] 5. (P) ドラッグ終了の HWND ガード追加
- [ ] 5.1 (P) handle_button_message の HWND 検証ロジック追加
  - `handlers.rs` の handle_button_message 内、WM_LBUTTONUP 処理部分（L1030-1060 付近）を修正
  - `read_drag_state` でスナップショットを取得
  - パターンマッチで状態別に HWND 一致を検証：
    - `Dragging { hwnd: drag_hwnd, .. }`: drag_hwnd と hwnd が一致する場合のみ end_dragging 実行
    - `Preparing { entity, .. }` / `JustStarted { entity, .. }`: `find_owner_window(world, entity) == Some(window_entity)` の場合のみ end_dragging 実行
    - その他: スキップ（異なるウィンドウまたはドラッグ状態でない）
  - DragState 構造体への変更なし
  - _Requirements: 3.1_

- [ ] 5.2* (P) ドラッグ HWND ガードのテストケース
  - DragState を Dragging(hwnd_A) に設定した状態で、hwnd_B からのボタンアップが end_dragging をスキップすることを検証
  - Preparing/JustStarted 状態でも同様の検証
  - _Requirements: 5.3_

---

- [ ] 6. (P) taffy_flex_demo のマルチウィンドウ化
- [ ] 6.1 (P) create_flexbox_window のパラメータ化
  - 既存の `create_flexbox_window(world: &mut World)` を `create_flexbox_window(world: &mut World, title: &str, position: POINT) -> Entity` にリファクタリング
  - ウィンドウタイトルと初期位置を引数で受け取る
  - 既存の全ウィジェット構成（RedBox、BlueBox、GreenBox、FlexContainer、SeikatuImage）を完全再現
  - イベントハンドラ関数は既存をそのまま共有（sender/entity 引数で動的に動作）
  - _Requirements: 1.1, 1.3_

- [ ] 6.2 (P) run_demo での複数ウィンドウ生成
  - `run_demo` 内で `create_flexbox_window` を2回呼び出し
  - 1つ目: タイトル "wintf - Taffy Flexbox Demo (Window 1)"、位置 (0, 0)
  - 2つ目: タイトル "wintf - Taffy Flexbox Demo (Window 2)"、位置 (850, 0)
  - 各ウィンドウに独立したウィジェットツリーを構築
  - _Requirements: 1.1, 1.2_

- [ ] 6.3 (P) マルチウィンドウイベント動作の検証ログ追加
  - 各イベントハンドラ内の tracing ログに window entity ID または window title を含める
  - ウィンドウ別のイベント動作が確認可能になるようログ出力を強化
  - 既存のログレベル（info/debug）を維持
  - _Requirements: 1.4_

- [ ] 6.4* (P) 手動検証の実施
  - `cargo run --example taffy_flex_demo` でデモを起動し、2つのウィンドウが表示されることを確認
  - 各ウィンドウで独立にクリック、ドラッグ、ホバーが動作することを視覚的に検証
  - tracing ログ（`RUST_LOG=debug` 設定）でウィンドウ独立性を確認
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

---

- [ ] 7. (P) マルチウィンドウ統合テストの実装
- [ ] 7.1 (P) test_find_owner_window ユニットテスト
  - Task 1.2 で作成済みの場合はスキップ
  - Pure ECS テストで find_owner_window の基本動作を検証
  - _Requirements: 5.1_

- [ ] 7.2 (P) test_mouseleave_scoped_pointer_clear 統合テスト
  - 2つの Window 配下にそれぞれ PointerState を持つエンティティを配置
  - Window A のスコープ付きクリア処理を模擬実行
  - Window A のエンティティの PointerState が削除され、Window B のエンティティは維持されることを検証
  - _Requirements: 5.2_

- [ ] 7.3 (P) test_build_bubble_path_stops_at_window 統合テスト
  - Task 2.2 で作成済みの場合はスキップ
  - build_bubble_path が Window で停止することを検証
  - _Requirements: 5.1_

- [ ] 7.4 (P) test_drag_state_hwnd_guard 統合テスト
  - Task 5.2 で作成済みの場合はスキップ
  - DragState HWND ガードの動作を検証
  - _Requirements: 5.3_

- [ ] 7.5 (P) tests/multiwindow_event_test.rs ファイル作成
  - 上記テストを1つのファイルにまとめる
  - `cargo test` で全テストが実行可能なことを確認
  - _Requirements: 5.1, 5.2, 5.3_

---

## 要件カバレッジ

| 要件 | カバーするタスク |
|------|------------------|
| 1.1 | 6.1, 6.2 |
| 1.2 | 6.2 |
| 1.3 | 6.1 |
| 1.4 | 6.3 |
| 2.1 | 1.1, 3.1 |
| 2.2 | 1.1, 3.1 |
| 2.3 | 1.1, 3.1, 3.2, 4.1, 4.2 |
| 3.1 | 5.1 |
| 3.2 | （既存実装で充足済み） |
| 4.1 | 1.1, 2.1 |
| 4.2 | 1.1, 2.1 |
| 4.3 | 1.1, 2.1 |
| 5.1 | 1.2, 7.1, 7.3, 7.5 |
| 5.2 | 7.2, 7.5 |
| 5.3 | 5.2, 7.4, 7.5 |

全13要件を完全カバー（3.2 は既存実装で対応済みのためタスク不要）。
