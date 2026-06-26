# Technology Stack

updated_at: 2026-06-26

## Architecture

Rust 2024を前提にしたマルチクレート構成です。wintfはbevy_ecsベースのWindows UIフレームワーク、dolaはプラットフォーム非依存の演出定義ライブラリ、arekaは両者を統合する実アプリ層です。ECSによる状態管理とWindowsネイティブAPIの直接利用を組み合わせ、論理ツリーとビジュアルツリーを分離します。

## Core Technologies

- **Language**: Rust 2024 Edition
- **Graphics**: DirectComposition, Direct2D, Direct3D11
- **Text**: DirectWrite（縦書き・横書き対応）
- **Imaging**: WIC（Windows Imaging Component）
- **Window System**: Win32 API

## Key Libraries

- **bevy_ecs** (0.18.0): ECSアーキテクチャの実装基盤
- **bevy_app** (0.18.0): ECSスケジュールとアプリライフサイクル統合
- **thiserror** (2): エラー型定義（全クレート共通）
- **windows** (0.62.2): Windows APIバインディング
- **windows-core** (0.62.2): Windows Core API
- **taffy** (0.9.2): レイアウトエンジン
- **async-executor** (1.13.3): 非同期タスク実行
- **bevy_tasks** (0.18.0): タスク実行基盤
- **tracing / tracing-subscriber**: 構造化ロギング
- **windows-numerics** (0.3.1): Windows数値型サポート
- **ambassador** (0.5.0): トレイト委譲（delegation）マクロ。COM/状態ラッパーのボイラープレート削減に使用
- **nonmax** (0.5.5): ニッチ最適化された非最大整数型
- **pasta_core** (0.1.6): 里々インスパイアの会話DSLエンジン。`[patch.crates-io]` で `vendors/pasta/` のサブモジュールへ差し替え（後述の Key Technical Decisions 参照）

### dola クレート依存
- **serde** (1): シリアライズ/デシリアライズ基盤
- **serde_json** (1, feature: `json`): JSON対応（デフォルト有効）
- **toml** (0.8, feature: `toml`): TOML対応
- **serde_yaml** (0.9, feature: `yaml`): YAML対応
- **interpolation** (0.3.0): イージング・補間計算の基盤（`easing.rs`, `runtime/interpolator/`）
- **rand** (0.10.0): アニメーション系の乱数生成
- **pasta_core**: DSL連携のための直接依存（wintf経由ではなくdolaが直接取り込む）

## Development Standards

### Type Safety
Rust言語の型システムを最大限に活用。`unsafe`ブロックはWindows API呼び出し時のみに限定し、安全性を文書化。

### Code Quality
- モジュール単位で責務を明確に分離（`com/`, `ecs/`, `api.rs`など）
- Windows COMオブジェクトのライフタイム管理を厳密に実施
- エラーハンドリングは `thiserror` を使用して構造化enumを定義する（全クレート共通規約）
- Windows API境界では `windows::core::Result` を使用し、内部エラーへの変換は `#[from]` で行う

### Testing

- `cargo test` を基準に、クレートごとの統合テストとin-sourceテストを併用する
- wintfは`tests/`配下をドメイン別に分割し、トップレベルのエントリポイントから束ねる
- examplesは手動検証とグラフィックス挙動確認の補助であり、テストの代替ではない

## Development Environment

### Required Tools
- Rust 2024 Edition
- Windows 10/11（DirectComposition対応）
- Visual Studio Build Tools（Windows SDKが必要）

### Common Commands
```bash
# Dev (サンプル実行): cargo run -p wintf --example taffy_flex_demo_old
# Build: cargo build
# Build (Release最適化): cargo build --release
# Test: cargo test
```

## Key Technical Decisions

- **ECS採用**: 複雑なGUI要素の管理とヒットテストロジックをコンポーネントベースで実装
- **DirectComposition**: ハードウェアアクセラレーションによる高速な合成処理と透過ウィンドウの実現
- **透過の合成方式は ULW/DComp 切替式（実装済み）**: 伺か型マスコットは「別プロセスのウィンドウ上に乗り、透明ピクセル上のクリックをその別プロセスへ透過させる」のが中核要件。これを満たせるのは実質 `UpdateLayeredWindow`（`ULW_ALPHA`/`AC_SRC_ALPHA` でアルファ0ピクセルがOSレベルで自動クリック透過）。そこでウィンドウ単位に `CompositionMode` enum で **ULW（デフォルト）⇄ DirectComposition** を選択する切替基盤を実装済み（生成時固定、生存中の動的切替は非対応）。COMラッパーは `com/ulw.rs`。`WM_NCHITTEST`→`HTTRANSPARENT` はプロセス境界を越えず別プロセス透過には使えない点に注意。`SetWindowRgn` 方式は DComp 描画をクリップするため却下済み（`_rejected/wintf-P0-click-through-rgn`）
- **DirectWrite**: 高品質な日本語テキストレンダリングと縦書き対応
- **Workspace構成**: フレームワーク、演出定義、実アプリを分離したモノレポ構成
- **Release最適化**: サイズ最適化（`opt-level='z'`, `lto=true`）でバイナリサイズを削減
- **レガシーAPI非推奨化**: `win_message_handler`, `win_thread_mgr`, `winproc` は `#[deprecated]` 指定済み。新規コードでは `ecs/window_proc/` 配下のモジュールを使用する
- **構造化ログ**: `tracing` を全体規約とし、subscriber初期化はアプリ層で行う
- **pasta のベンダリング**: 外部依存だった `pasta_core` を git サブモジュール（`vendors/pasta/`）として同梱し、`[patch.crates-io]` でローカルパスへ差し替える。wintf/dola/areka とDSLエンジンを同一ワークスペースで協調開発するための運用。クローン時は `git submodule update --init` が必要
- **ukadoc互換ベースウェア戦略（2026-06-26）**: areka を ukadoc準拠の互換ベースウェア（SSP代替）として確立する。SERIKO/MAYUNA完全マップ＋さくらスクリプト優先度順。SERIKO/さくらスクリプトランナーは「タイミング特化の下位層 dola」の上に建てる上位層。SERIKOを平坦サブセットに内包する**階層サーフェスエンジン**（エレメント→別サーフェス定義参照・wintf visual-tree＋dola nested-storyboard）。SHIORIは内部唯一ABI=`IShiori`(COM, HSTRING/UTF-16)、ネイティブ=in-proc COM、過去互換=32bit Rustホスト（flat-C/HGLOBAL/charset/SAORI同居/自前IPC）。詳細の正本は `doc/COMPAT_ARCHITECTURE.md`

---
Document standards and patterns, not every dependency.
