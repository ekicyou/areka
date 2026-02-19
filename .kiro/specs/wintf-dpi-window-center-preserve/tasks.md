# Implementation Plan

- [ ] 1. handlers.rs をファイル別に分割してウィンドウプロシージャモジュールを整理する
- [ ] 1.1 ライフサイクルとディスプレイ変更ハンドラを lifecycle.rs に切り出す
  - `window_proc/` 配下に `lifecycle.rs` を新設する
  - `WM_NCCREATE`, `WM_NCDESTROY`, `WM_ERASEBKGND`, `WM_PAINT`, `WM_CLOSE`, `WM_DISPLAYCHANGE` の各関数を移動する（~117行）
  - `mod.rs` に `mod lifecycle;` を追加し、`pub(super)` で各関数を再エクスポートする
  - ビルドが通ることを確認する（`cargo build`）
  - _Requirements: 4.4_

- [ ] 1.2 マウス移動・ヒットテスト・離脱ハンドラを mouse_move.rs に切り出す
  - `window_proc/` 配下に `mouse_move.rs` を新設する
  - `WM_NCHITTEST`, `WM_MOUSEMOVE`, `WM_MOUSELEAVE` および `collect_entities_to_leave` ヘルパーを移動する（~473行）
  - `mod.rs` に `mod mouse_move;` を追加し、`pub(super)` で各関数を再エクスポートする
  - ビルドが通ることを確認する
  - _Requirements: 4.4_

- [ ] 1.3 ボタン・ホイールハンドラを mouse_button.rs に切り出す
  - `window_proc/` 配下に `mouse_button.rs` を新設する
  - `handle_button_message`, `handle_double_click_message` の共通ヘルパーと、ボタン6種（LBUTTONDOWN/UP, RBUTTONDOWN/UP, MBUTTONDOWN/UP, XBUTTONDOWN/UP）、ダブルクリック4種、`WM_MOUSEWHEEL`, `WM_MOUSEHWHEEL` を移動する（~630行）
  - `mod.rs` に `mod mouse_button;` を追加し再エクスポートする
  - ビルドが通ることを確認する
  - _Requirements: 4.4_

- [ ] 1.4 キーボード・ドラッグ制御ハンドラを keyboard.rs に切り出す
  - `window_proc/` 配下に `keyboard.rs` を新設する
  - `WM_KEYDOWN`, `WM_CANCELMODE`, `WM_ACTIVATE`, および `find_ancestor_with_drag_config` ヘルパーを移動する（~175行）
  - `mod.rs` に `mod keyboard;` を追加し再エクスポートする
  - ビルドが通ることを確認する
  - _Requirements: 4.4_

- [ ] 1.5 残った位置・DPI ハンドラをもとに window_pos.rs を完成させ handlers.rs を削除する
  - `handlers.rs` の残存内容（`WM_WINDOWPOSCHANGED`, `WM_DPICHANGED`、共通の use 宣言）を `window_pos.rs` に移動する（~360行）
  - `mod.rs` に `mod window_pos;` を追加し、これまで `handlers::` で参照していた全エントリを各サブモジュールから再エクスポートするよう更新する
  - `handlers.rs` を削除する
  - `cargo build` および `cargo test` が全件通ることを確認する
  - _Requirements: 4.4_

- [ ] 2. DPI 変更時の中心保持補正ロジックを window_pos.rs に実装する

- [ ] 2.1 (P) BoxStyle 論理サイズから新 DPI スケールの物理サイズを算出するロジックを実装する
  - `window_pos.rs` 内に純粋関数として実装する
  - `BoxStyle.size` が `Some(BoxSize { width: Some(Px(w)), height: Some(Px(h)) })` の場合は `(w * dpi.scale_x()).ceil() as i32` および `(h * dpi.scale_y()).ceil() as i32` を返す
  - `BoxStyle.size` が `None` または `Dimension::Px` 以外の場合は `None` を返す（補正スキップ用フォールバック）
  - `window_pos_sync_system` と同一の ceiling 変換ロジックを使用し計算結果の一致を保証する
  - _Requirements: 1.1, 1.2, 4.2, 4.3_

- [ ] 2.2 (P) 旧物理サイズと新物理サイズから中心保持補正量を算出するロジックを実装する
  - `window_pos.rs` 内に純粋関数として実装する
  - 補正量を `((old_cx - new_cx) / 2, (old_cy - new_cy) / 2)` として算出する
  - サイズが同一の場合は `(0, 0)` を返す
  - 数学的証明: `corrected_pos + new_size/2 = client_pos + old_size/2`（中心不変性）を实装に反映する
  - _Requirements: 1.1, 1.2_

- [ ] 2.3 DPI 変更時の補正エントリポイントを実装して WM_WINDOWPOSCHANGED ハンドラに統合する
  - 2.1・2.2 で実装した関数を組み合わせたエントリポイントを `window_pos.rs` に追加する
  - `dpi_context` が `None` の場合は `client_pos` をそのまま返す（DPI 変更なし・手動リサイズ時は補正しない）
  - `dpi_context` が `Some` かつ `BoxStyle.size` 取得成功時のみ補正を適用し、失敗時は `client_pos` にフォールバックする
  - `WM_WINDOWPOSCHANGED` ハンドラの通常パスで `window_pos.position` 設定直前に呼び出し、補正済みの値を使用するよう変更する
  - サイズ変化なしの場合（correction = (0,0)）は `trace!` ログを出力する
  - 補正適用時は `debug!` ログで補正前後の中心座標と補正量を出力する（`[WM_WINDOWPOSCHANGED] DPI center correction applied` プレフィックス）
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3, 3.1, 3.2, 4.1, 4.2, 4.3, 6.1, 6.2, 6.3, 7.1, 7.2_

- [ ] 3. 補正ロジックのユニットテストを実装する

- [ ] 3.1 (P) 中心保持補正量算出のユニットテストを実装する
  - `window_pos.rs` の `#[cfg(test)]` ブロックに追加する
  - サイズ縮小ケース（200%→125%、800×600 → 500×375）の補正量が `(150, 112)` となることを検証する
  - サイズ拡大ケース（125%→200%、500×375 → 800×600）の補正量が `(-150, -112)` となることを検証する
  - サイズ同一ケースで補正量が `(0, 0)` となることを検証する
  - 補正前後でウィンドウ中心座標が一致することを数値で検証する
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1, 3.2_

- [ ] 3.2 (P) BoxStyle 物理サイズ変換のユニットテストを実装する
  - `window_pos.rs` の `#[cfg(test)]` ブロックに追加する
  - `BoxSize { width: Some(Px(400.0)), height: Some(Px(300.0))}` × DPI 125%（scale=1.25）→ `SIZE { cx: 500, cy: 375 }` を検証する
  - `BoxStyle.size` が `None` の場合に `None` を返すことを検証する
  - ceiling 処理の境界値（小数点以下の切り上げ）を検証する
  - _Requirements: 1.1, 4.2_

- [ ] 3.3 SetWindowPosGuard カウンタ方式の動作を確認する（先行実装済み）
  - `window.rs` の `SELF_INITIATED_DEPTH` が `AtomicI32` で実装されていることを確認する
  - `SetWindowPosGuard::new()` でカウンタが +1、`Drop` で -1 されることを確認する
  - `is_self_initiated()` がカウンタ > 0 で `true` を返すことを確認する
  - `cargo test` で既存テストが全件通ることを確認する
  - _Requirements: 5.1, 5.2, 5.3_
