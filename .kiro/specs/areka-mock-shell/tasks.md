# Implementation Plan

## Task Overview
- **Feature**: areka-mock-shell
- **Total**: 6 major tasks, 15 sub-tasks
- **Estimated Effort**: 1-3 hours per sub-task

## Implementation Tasks

- [x] 1. クレート基盤セットアップ
- [x] 1.1 (P) Cargo.toml とクレート構造を作成
  - `crates/areka/Cargo.toml` を新規作成し、バイナリクレートとして設定
  - `name = "areka"`, `version = "0.0.1"`, `publish = true` を設定
  - ワークスペース依存（wintf, human-panic, tracing等）を追加
  - `description`, `license`, `repository` メタデータを設定
  - _Requirements: 5.1, 5.2, 5.4_

- [x] 1.2 (P) シェルアセットを移動
  - `shell/` ディレクトリを `crates/areka/shell/` に `git mv` で移動
  - アセットパス（`base.png`）がワークスペースルートからの相対パスで参照可能であることを確認
  - _Requirements: 5.5_

- [x] 1.3 ダミーファイルの削除とステアリング更新
  - `crates/wintf/examples/areka.rs` を `git rm` で削除
  - `.kiro/steering/structure.md` の areka クレート Status を「未作成」→「モック実装」に更新
  - _Requirements: 5.6_

- [x] 2. エントリポイントと初期化
- [x] 2.1 main関数の骨格を実装
  - `src/main.rs` にエントリポイントを作成
  - human-panic セットアップを実装
  - tracing-subscriber を `EnvFilter` 付きで初期化（デフォルト `info` レベル）
  - リリースビルド時のコンソール非表示設定（`windows_subsystem = "windows"`）を追加
  - _Requirements: 4.3_

- [x] 2.2 WinThreadMgr 初期化と操作ガイド出力
  - `WinThreadMgr::new()` でフレームワークを初期化
  - `world.borrow()` で EcsWorld への参照を取得
  - 操作ガイド（ドラッグ移動・ダブルクリック終了）をコンソールに出力
  - `mgr.run()` でブロッキングメッセージループを実行
  - _Requirements: 4.2, 4.3_

- [x] 3. シェルウィンドウ実装
- [x] 3.1 シェルウィンドウEntity構築関数を実装
  - `Window`, `WindowStyle(WS_POPUP | WS_VISIBLE, WS_EX_NOREDIRECTIONBITMAP)` でタイトルバーなし透過ウィンドウを設定
  - `WindowPos` でデスクトップ中央付近の初期位置を設定
  - `BoxStyle` で 320×420px サイズを指定
  - `ShellWindowMarker` コンポーネントでクエリ識別用マーカーを追加
  - _Requirements: 1.1_

- [x] 3.2 キャラクター画像表示を実装
  - `BitmapSource::new("crates/areka/shell/base.png")` でキャラクター画像を読み込み
  - `BoxStyle` で画像の配置を設定
  - `ChildOf(shell_window_entity)` で親子関係を設定
  - _Requirements: 1.2, 5.3_

- [x] 3.3 ドラッグ設定とイベントハンドラを登録
  - `DragConfig { move_window: true }` でネイティブドラッグを有効化
  - `OnDrag(on_shell_drag)` ハンドラをシェルウィンドウに追加
  - `OnPointerPressed(on_shell_pressed)` ハンドラをシェルウィンドウに追加
  - _Requirements: 3.1_

- [x] 4. バルーンウィンドウ実装
- [x] 4.1 バルーンウィンドウEntity構築関数を実装
  - `Window`, `WindowStyle(WS_POPUP | WS_VISIBLE, WS_EX_NOREDIRECTIONBITMAP)` で透過ウィンドウを設定
  - `WindowPos` でシェルの右側（x + 335px, y + 0px）に配置
  - `BoxStyle` で幅 200px, 高さ 350px のサイズを設定
  - `BalloonWindowMarker` コンポーネントでクエリ識別用マーカーを追加
  - `ChildOf(shell_entity)` でシェルウィンドウとの親子関係を設定
  - _Requirements: 2.1, 2.2, 2.5, 5.3_

- [x] 4.2 (P) バルーン背景矩形を実装
  - `Rectangle::new()` で矩形ウィジェットを作成
  - `Brushes::with_foreground(D2D1_COLOR_F { r: 1.0, g: 1.0, b: 0.95, a: 0.85 })` で薄いクリーム色の半透明背景を設定
  - `BoxStyle { flex_grow: Some(1.0), .. }` でバルーンウィンドウ全体を覆う配置
  - `ChildOf(balloon_window_entity)` で親子関係を設定
  - _Requirements: 2.3, 5.3_

- [x] 4.3 (P) 縦書きテキスト表示を実装
  - `Typewriter { font_family: "メイリオ", font_size: 16.0, direction: TextDirection::VerticalRightToLeft, default_char_wait: 0.08 }` を設定
  - `Brushes::with_colors(foreground_black, transparent_bg)` で文字色を設定
  - テキストを文字単位で `TypewriterToken::Char(c)` に分解し、改行・空行を適切に処理
  - `TypewriterTalk::new(tokens, start_time)` でテキストトークンを設定
  - `BoxStyle` でマージン付き配置を設定
  - `ChildOf(balloon_background_entity)` で背景矩形の子として配置
  - _Requirements: 2.4, 5.3_

- [x] 5. インタラクション実装
- [x] 5.1 非同期UI構築タスクを実装
  - `world.borrow().spawn(|tx| async { ... })` で非同期タスクを起動
  - `tx.send(Box::new(|world| { ... }))` でコマンドを送信
  - コマンド内でシェルウィンドウとバルーンウィンドウを生成し、EntityIDを受け渡す
  - _Requirements: 4.1_

- [x] 5.2 バルーン追従ハンドラを実装
  - `OnDrag` ハンドラ内でシェルの現在 `WindowPos.position` を取得
  - バルーンの `WindowPos.position` をシェル位置 + オフセット（x: +335px, y: +0px）で計算
  - `SetWindowPosCommand` を発行してバルーンのウィンドウ位置を即時反映
  - _Requirements: 3.2_

- [x] 5.3 ダブルクリック終了ハンドラを実装
  - `OnPointerPressed` ハンドラ内で `Phase::Bubble(state)` の `state.double_click == DoubleClick::Left` を検査
  - 一致した場合、`ShellWindowMarker` と `BalloonWindowMarker` を持つ全エンティティを検索
  - 全エンティティを `world.despawn()` で破棄し、アプリケーションを終了させる
  - _Requirements: 3.3_

- [x] 6. 統合とテスト
- [x] 6.1* 手動動作テストを実施
  - `cargo run -p areka` でシェルとバルーンが表示されることを確認（Acceptance Criteria: 1.1, 2.1）
  - シェルウィンドウをドラッグ移動し、バルーンが追従することを確認（Acceptance Criteria: 3.1, 3.2）
  - シェルウィンドウをダブルクリックし、アプリケーションが終了することを確認（Acceptance Criteria: 3.3）
  - `RUST_LOG=debug cargo run -p areka` でデバッグログが出力されることを確認（Acceptance Criteria: 4.3）
  - シェル画像が正しく透過表示されることを確認（Acceptance Criteria: 1.2）
  - バルーンに縦書きテキストが表示されることを確認（Acceptance Criteria: 2.4）
  - _Requirements: 1.1, 1.2, 2.1, 2.4, 3.1, 3.2, 3.3, 4.3_

- [x] 6.2* ビルドテストを実施
  - `cargo build -p areka` が成功することを確認（Acceptance Criteria: 5.1）
  - `cargo test` でワークスペース全体のテストが既存テストを壊さないことを確認
  - `cargo check -p areka` でコンパイルエラーがないことを確認
  - _Requirements: 5.1, 5.2, 5.3, 5.4_

## Requirements Coverage

| Requirement | Tasks | Coverage Check |
|-------------|-------|----------------|
| 1.1 (シェルウィンドウ透過表示) | 3.1, 6.1 | ✓ |
| 1.2 (キャラクター画像表示) | 3.2, 6.1 | ✓ |
| 1.3 (クリックスルー) | — | スコープ外 |
| 2.1 (バルーン表示) | 4.1, 6.1 | ✓ |
| 2.2 (ポップアップスタイル) | 4.1 | ✓ |
| 2.3 (背景矩形) | 4.2 | ✓ |
| 2.4 (縦書きテキスト) | 4.3, 6.1 | ✓ |
| 2.5 (シェル右側配置) | 4.1 | ✓ |
| 3.1 (ドラッグ移動) | 3.3, 6.1 | ✓ |
| 3.2 (バルーン追従) | 5.2, 6.1 | ✓ |
| 3.3 (ダブルクリック終了) | 5.3, 6.1 | ✓ |
| 4.1 (非同期UI構築) | 5.1 | ✓ |
| 4.2 (操作ガイド) | 2.2 | ✓ |
| 4.3 (ログ制御) | 2.1, 6.1 | ✓ |
| 5.1 (Cargo.toml) | 1.1, 6.2 | ✓ |
| 5.2 (publish=true) | 1.1, 6.2 | ✓ |
| 5.3 (wintf公開APIのみ) | 3.2, 4.1, 4.2, 4.3, 6.2 | ✓ |
| 5.4 (ワークスペース依存) | 1.1, 6.2 | ✓ |
| 5.5 (アセット移動) | 1.2 | ✓ |
| 5.6 (ダミー削除・structure更新) | 1.3 | ✓ |

**All 18 acceptance criteria covered** (1.3 intentionally out of scope)
