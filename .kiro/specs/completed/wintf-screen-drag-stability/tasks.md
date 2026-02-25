# Implementation Tasks

## 1. CaptureGuard RAII コンポーネント

- [x] 1.1 (P) CaptureGuard 構造体を実装する
  - `CaptureGuard` 構造体を定義し、`hwnd` と `released` フィールドを持つ
  - `acquire(hwnd)` コンストラクタで `SetCapture` を呼び出し、debug ログを出力する
  - `mark_released()` メソッドで `released` フラグを true にセットする
  - Drop trait 実装で、`released` が false の場合のみ `ReleaseCapture` を呼び出し、debug ログを出力する
  - _Requirements: 1.1, 1.2, 1.5, 5.4, 5.5_

## 2. DragState への CaptureGuard 統合

- [x] 2.1 DragState 各バリアントに capture_guard フィールドを追加する
  - `Preparing`, `JustStarted`, `Dragging` バリアントに `capture_guard: CaptureGuard` フィールドを追加
  - パターンマッチ箇所を網羅的に修正（コンパイラエラーで検出）
  - _Requirements: 1.1, 1.2, 1.5_

- [x] 2.2 start_preparing 関数で CaptureGuard を生成する
  - `CaptureGuard::acquire(hwnd)` を呼び出して guard を生成
  - `DragState::Preparing` に格納する
  - キャプチャ取得失敗時のフォールバック処理（ログ出力のみ、処理継続）
  - _Requirements: 1.1, 1.3_

- [x] 2.3 状態遷移時に capture_guard を引き継ぐ
  - `check_threshold` (Preparing → JustStarted) で capture_guard を移動
  - `start_dragging` (JustStarted → Dragging) で capture_guard を移動
  - `update_dragging` (Dragging → Dragging) で capture_guard を保持
  - _Requirements: 1.1, 1.2_

- [x] 2.4 end_dragging / cancel_dragging でドロップによる解放を実現する
  - `DragState::Idle` への遷移により、旧状態の Drop が発火
  - `CaptureGuard::drop()` が自動的に `ReleaseCapture` を呼ぶ
  - _Requirements: 1.2, 5.4_

## 3. WM_CAPTURECHANGED ハンドラ

- [x] 3.1 (P) keyboard.rs に handle_capture_changed 関数を追加する
  - WM_CAPTURECHANGED メッセージを受信したとき、DragState が Preparing / JustStarted / Dragging の場合に処理
  - `capture_guard.mark_released()` を呼んでから `cancel_dragging()` を実行
  - 既に Idle の場合は早期 return（冪等性）
  - DragEndEvent を `cancelled: true` で発行
  - _Requirements: 1.4_

- [x] 3.2 (P) ecs_wndproc ディスパッチテーブルに WM_CAPTURECHANGED を追加する
  - `mod.rs` のディスパッチテーブルに `WM_CAPTURECHANGED` エントリを追加
  - `handle_capture_changed` 関数にルーティング
  - _Requirements: 1.4_

## 4. WindowDragging フィルタ追加

- [x] 4.1 (P) window_pos_sync_system に Without<WindowDragging> を追加する
  - Query の型パラメータに `Without<WindowDragging>` を追加
  - ドラッグ中は `Changed<GlobalArrangement>` → WindowPos の書き戻しをスキップ
  - _Requirements: 2.4, 3.4, 4.3_

- [x] 4.2 (P) sync_window_arrangement_from_window_pos に Without<WindowDragging> を追加する
  - Query の型パラメータに `Without<WindowDragging>` を追加
  - ドラッグ中は `Changed<WindowPos>` → `Arrangement.offset` の同期をスキップ
  - _Requirements: 2.4, 3.4_

- [x] 4.3 (P) apply_window_pos_changes に Without<WindowDragging> を追加する
  - Query の型パラメータに `Without<WindowDragging>` を追加
  - ドラッグ中は `Changed<WindowPos>` → `SetWindowPos` の発行をスキップ
  - _Requirements: 2.4, 4.2_

## 5. テストと検証

- [x] 5.1 (P) CaptureGuard の単体テストを実装する
  - RAII 動作: acquire → スコープ終了で Drop → ReleaseCapture 呼び出し確認
  - mark_released: フラグセット後の Drop で ReleaseCapture が呼ばれないこと確認
  - _Requirements: 1.1, 1.2, 1.5_

- [x] 5.2 (P) パニック安全性テストを実装する
  - `std::panic::catch_unwind` + thread spawn で検証
  - `start_preparing` → `panic!()` → thread 終了時に `ReleaseCapture` が呼ばれること
  - _Requirements: 1.5, 5.5_

- [x] 5.3 (P) WindowDragging フィルタの統合テストを実装する
  - `WindowDragging` insert 状態で `window_pos_sync_system` がスキップされること
  - `WindowDragging` remove 後に `dispatch_drag_events` Ended パスが `Arrangement.offset` を正しく同期すること
  - _Requirements: 2.4, 3.4, 5.1, 5.3_

- [x]* 5.4 (P) E2E 手動検証シナリオを実行する
  - DPI 境界横断ドラッグ（200% → 125%）でドラッグが途切れないこと確認
  - 同一 DPI 境界横断ドラッグ（100% → 100%）で安定すること確認
  - ESC キャンセルでキャプチャが解放されること確認
  - Alt+Tab でキャプチャ喪失時にドラッグが安全に終了すること確認
  - _Requirements: 1.2, 1.4, 2.1, 2.5_

## 6. 統合と最終検証

- [x] 6.1 全要件をカバーする統合テストを実行する
  - すべての Acceptance Criteria が満たされていることを確認
  - 既存のドラッグテストが正常にパスすること確認
  - リグレッションがないことを検証
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 2.1, 2.4, 2.5, 3.4, 4.2, 4.3, 5.1, 5.3, 5.4, 5.5_
