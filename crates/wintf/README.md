# wintf — Windows Tategaki Framework

> Windows上で日本語縦書き描画をサポートする、bevy_ecs ベースのUIフレームワーク

---

## 概要

**wintf** (Windows Tategaki Framework) は、ECSアーキテクチャを基盤に DirectComposition / Direct2D / DirectWrite を統合した Rust 製 Windows UI フレームワークです。「伺か」のようなデスクトップマスコットアプリケーションに必要な、透過ウィンドウ・日本語縦書き描画・高精度ヒットテストを提供します。

---

## 主要機能

| 機能 | 説明 |
|------|------|
| **ECS統合** | bevy_ecs + bevy_app による宣言的UI管理 |
| **DirectComposition** | ハードウェアアクセラレーション合成、透過ウィンドウ |
| **Direct2D描画** | 高品質2Dレンダリング、Surface最適化 |
| **縦書きテキスト** | DirectWrite による日本語縦書き・横書き両対応 |
| **Flexboxレイアウト** | Taffy エンジンによるレイアウト計算 |
| **Image ウィジェット** | WIC画像読込、透過PNG、非同期タスクプール |
| **タイプライター** | 文字単位表示制御（pause/resume/skip） |
| **ポインターイベント** | Tunnel/Bubble 2フェーズ伝播、アルファマスクヒットテスト |
| **ドラッグシステム** | エンティティドラッグ＋ウィンドウ移動 |
| **DPI対応** | Per-Monitor DPI、マルチモニタ対応 |

---

## アーキテクチャ

wintf は3層構造で責務を分離しています。

```
┌─────────────────────────────────┐
│  Message Handling               │  WndProcメッセージ処理・スレッド管理
│  (winproc.rs, win_*.rs, api.rs) │
├─────────────────────────────────┤
│  ECS Component Layer            │  コンポーネント定義・システム実行
│  (ecs/)                         │
├─────────────────────────────────┤
│  COM Wrapper Layer              │  Windows COM APIのRustラッパー
│  (com/)                         │
└─────────────────────────────────┘
```

---

## サンプル実行

```bash
# Flexboxレイアウトデモ（画像ウィジェット + ドラッグ操作）
cargo run --example taffy_flex_demo

# タイプライターデモ（横書き・縦書きテキスト表示）
cargo run --example typewriter_demo

# DirectComposition描画デモ
cargo run --example dcomp_demo

# マルチウィンドウテスト
cargo run --example multi_window_test
```

---

## モジュール一覧

### ECS (`src/ecs/`)

| モジュール | 責務 |
|-----------|------|
| `app.rs` | ECS Appスケジュール管理 |
| `world.rs` | ECS World 管理・tick 実行 |
| `window.rs` | Win32ウィンドウのライフサイクル管理とECS統合 |
| `window_proc/` | ウィンドウプロシージャのECS統合 |
| `window_system.rs` | ウィンドウ作成・破棄システム |
| `graphics/` | Direct2D/DirectCompositionリソース管理 |
| `monitor.rs` | マルチモニタ・ディスプレイエンティティ管理 |
| `layout/` | Taffy Flexbox統合、Arrangement配置計算 |
| `common/` | 階層伝播システム（ジェネリックツリー走査） |
| `widget/` | UIウィジェット（Label, shapes, BitmapSource, brushes） |
| `pointer/` | ポインターイベント配信（Tunnel/Bubble） |
| `drag/` | ドラッグシステム（エンティティ＋ウィンドウ移動） |
| `transform/` | 変換システム（**非推奨**: Arrangement推奨） |
| `nchittest_cache.rs` | WM_NCHITTESTキャッシュ最適化 |

### COM Wrapper (`src/com/`)

DirectComposition, Direct2D, Direct3D11, DirectWrite, WIC, Windows Animation の Rust ラッパー。

### Message Handling (`src/`)

`winproc.rs`, `win_message_handler.rs`, `win_thread_mgr.rs`, `api.rs` — Win32メッセージループとスレッド管理。

---

## 詳細設計参照

wintf の詳細設計は 12 章の仕様書にまとめられています：

1. [ECSコンポーネント](../../doc/spec/01-ecs-components.md)
2. [ウィジェットツリー](../../doc/spec/02-widget-tree.md)
3. [システム分離](../../doc/spec/03-system-separation.md)
4. [レイアウトシステム](../../doc/spec/04-layout-system.md)
5. [レイアウト詳細](../../doc/spec/05-layout-details.md)
6. [Visual/DirectComposition](../../doc/spec/06-visual-directcomp.md)
7. [更新フロー](../../doc/spec/07-update-flow.md)
8. [イベントシステム](../../doc/spec/08-event-system.md)
9. [ヒットテスト](../../doc/spec/09-hit-test.md)
10. [UI要素](../../doc/spec/10-ui-elements.md)
11. [使用例](../../doc/spec/11-usage-examples.md)
12. [Visual最適化](../../doc/spec/12-visual-optimization.md)

仕様概要は [doc/spec/README.md](../../doc/spec/README.md) を参照。
