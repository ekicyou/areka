# Implementation Plan

- [x] 1. SetWindowPosGuard をネスト対応の save/restore 方式に修正する
- [x] 1.1 `IS_SELF_INITIATED` フラグの保存・復元ロジックを実装する
  - `SetWindowPosGuard` 構造体に `previous: bool` フィールドを追加する
  - 構築時に現在の `IS_SELF_INITIATED` 値を `previous` に保存し、`true` をセットする
  - `Drop` 実装で `IS_SELF_INITIATED.set(self.previous)` として復元する（現行の無条件 `false` セットを廃止）
  - ドラッグ中 DPI 変更の Case A/C（ネストシナリオ）で正しく動作することを確認する
  - _Requirements: 3.2, 4.1_

- [x] 2. `update_arrangements_system` に DPI スケール設定ロジックを追加する
- [x] 2.1 Window エンティティの `Arrangement.scale` を DPI スケールで設定する
  - `Window` マーカーコンポーネントを持つエンティティに対してのみ `Arrangement.scale` を `{x: DPI.scale_x(), y: DPI.scale_y()}` に設定する
  - `DPI` コンポーネントが存在しない Window エンティティは `(1.0, 1.0)` にフォールバックする
  - Window 以外のエンティティは引き続き `LayoutScale::default()` = `(1.0, 1.0)` を維持する
  - `Changed<DPI>` を持つエンティティのみ処理するよう絞り込み、不要な再計算を避ける
  - _Requirements: 1.1, 1.3, 2.1, 2.2, 2.3_

- [x] 3. `WM_DPICHANGED` ハンドラをレイアウトシステム主導方式に書き換える
- [x] 3.1 DPI コンポーネントをハンドラ内で直接更新する
  - `WM_DPICHANGED` 受信時に World を borrow して対象ウィンドウエンティティの `DPI` コンポーネントを `new_dpi` で直接更新し、`Changed<DPI>` を発火させる
  - World borrow のスコープを最小限（DPI 更新のみ）に留め、後続の TLS 操作・SetWindowPos と混在させない
  - _Requirements: 4.1_

- [x] 3.2 `DpiChangeContext` を設定し `SWP_NOSIZE` を維持したまま SetWindowPos を呼ぶ
  - World borrow 解放後に `DpiChangeContext::set(new_dpi)` を呼ぶ（echo bypass 防止 + BoxStyle skip 信号）
  - `guarded_set_window_pos` を `suggested_rect` の **位置のみ**（`SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE`）で呼び出す
  - `suggested_rect` の幅・高さは使わない（サイズ決定は ECS パイプラインに委ねる）
  - _Requirements: 4.1, 4.2, 3.2_

- [x] 4. `WM_WINDOWPOSCHANGED` ハンドラの BoxStyle 更新ロジックを修正する
- [x] 4.1 bypass 判定と BoxStyle skip 判定を分離する
  - `use_bypass = is_echo && dpi_context.is_none()` として、DPI 変更時（`dpi_context.is_some()`）は bypass しない
  - `skip_box_style = is_echo || dpi_context.is_some()` として、DPI 変更時は BoxStyle.size を更新しない
  - `skip_box_style` が `false`（外部リサイズ時のみ）の場合に `physical_width / dpi.scale_x()`、`physical_height / dpi.scale_y()` で論理 px に変換して BoxStyle.size を設定する
  - `Changed<WindowPos>` は DPI 変更時も発火させ、`sync_window_arrangement_from_window_pos` が `Arrangement.offset` を更新できるようにする
  - _Requirements: 1.1, 1.2, 3.1, 3.2, 4.2_

- [x] 4.2 DPI 変更時の DPI コンポーネント更新をハンドラから除去する
  - 旧来の「`DpiChangeContext` から DPI を取り出して `DPI` コンポーネントを更新する」ロジックを削除する（タスク 3.1 で WM_DPICHANGED 側に移管済みのため）
  - `DpiChangeContext` の読み取りは echo bypass / BoxStyle skip の判定にのみ使用する
  - _Requirements: 3.2, 4.2_

- [x] 5. (P) `dump_all_windows_dpi` 関数のログ移行と情報追加を行う
- [x] 5.1 (P) `println!` を `info!` マクロに移行する
  - `dump_all_windows_dpi` 内の全 `println!` を `tracing::info!` に置換する
  - `dump_children_dpi` などのヘルパー関数も同様に移行する
  - steering/logging.md の構造化フィールド規約（`key=value` 形式）に準拠する
  - _Requirements: 7.1, 7.2, 7.4_

- [x] 5.2 (P) `BoxStyle.size` の論理 px サイズをログ出力に追加する
  - 各エンティティダンプに `BoxStyle.size.width`、`BoxStyle.size.height` の論理 px 値を `info!` で出力する
  - `BoxStyle` コンポーネントへのクエリを `dump_all_windows_dpi` に追加する
  - _Requirements: 7.3, 7.4_

- [x] 6. (P) `taffy_flex_demo` の `run_demo` タイムラインを短縮する
  - 総所要時間を 60 秒から約 4 秒に短縮する
  - タイムライン: `0s→create → 1s待機 → dump① → 1s待機 → change_layout_parameters → 1s待機 → dump② → 1s待機 → close`
  - `change_layout_parameters` は Taffy レイアウト変更の統合テストとして維持する
  - DPI ダンプを change 前後の 2 回実行し、変動フレームを確実にキャプチャする
  - _Requirements: 6.1, 6.2_

- [x] 7. DPI スケーリング修正の統合検証を行う
- [x] 7.1 `cargo test` で既存テストの回帰がないことを確認する
  - `cargo test` を実行してすべてのテストが PASS することを確認する
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3_

- [x] 7.2 デモ実行で DPI スケーリングの正しさをログ検証する
  - `RUST_LOG=info cargo run --example taffy_flex_demo` を実行して手作業なしに自動終了することを確認する
  - dump① ログで `GA.bounds.width ≈ BoxStyle.width × 1.25`（Window 1, 125% DPI）を確認する
  - dump① ログで `GA.bounds.width ≈ BoxStyle.width × 2.00`（Window 2, 200% DPI）を確認する
  - 両 Window の `BoxStyle.size` が同一の論理 px 値であることを確認する
  - _Requirements: 1.1, 1.3, 2.1, 2.3, 5.1, 5.2, 6.1, 6.2, 7.1, 7.2, 7.3, 7.4_

- [x] 7.3 DPI 変更ラウンドトリップをドラッグ操作で検証する
  - プログラマティック移動（WindowPos 直接変更）によるラウンドトリップを検証し、`BoxStyle.size` が維持されること（800×700 論理px 不変）を確認済み ✅
  - 200%→125% ドラッグ時のウィンドウ戻り現象は本仕様スコープ外と判定。根本原因（DPI縮小時の中心座標ずれ）は新仕様 `wintf-dpi-window-center-preserve` に分離
  - _Requirements: 3.1, 3.2, 4.1, 4.2_
