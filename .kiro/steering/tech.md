# Technology Stack

## Architecture

ECSアーキテクチャ（Entity Component System）をベースとした、Windows固有のグラフィックスAPIとの統合。論理ツリーとビジュアルツリーの二層構造により、アプリケーションロジックと描画処理を分離。

## Core Technologies

- **Language**: Rust 2024 Edition
- **Graphics**: DirectComposition, Direct2D, Direct3D11
- **Text**: DirectWrite (縦書き・横書き対応)
- **Imaging**: WIC (Windows Imaging Component)
- **Window System**: Win32 API

## Key Libraries

- **bevy_ecs** (0.18.0): ECSアーキテクチャの実装基盤
- **thiserror** (2): エラー型定義（全クレート共通）
- **windows** (0.62.2): Windows API バインディング
- **windows-core** (0.62.2): Windows Core API
- **taffy** (0.9.2): レイアウトエンジン
- **euclid** (0.22.11): 2D/3D幾何計算
- **async-executor** (1.13.3): 非同期タスク実行
- **windows-numerics** (0.3.1): Windows数値型サポート

### dola クレート依存
- **serde** (1): シリアライズ/デシリアライズ基盤
- **serde_json** (1, feature: `json`): JSON対応（デフォルト有効）
- **toml** (0.8, feature: `toml`): TOML対応
- **serde_yaml** (0.9, feature: `yaml`): YAML対応

## Development Standards

### Type Safety
Rust言語の型システムを最大限に活用。`unsafe`ブロックはWindows API呼び出し時のみに限定し、安全性を文書化。

### Code Quality
- モジュール単位で責務を明確に分離（`com/`, `ecs/`, `api.rs`など）
- Windows COMオブジェクトのライフタイム管理を厳密に実施
- エラーハンドリングは `thiserror` を使用して構造化 enum を定義する（全クレート共通規約）
- Windows API 境界では `windows::core::Result` を使用し、内部エラーへの変換は `#[from]` で行う

### Testing
サンプルアプリケーション（`examples/areka.rs`, `examples/dcomp_demo.rs`）で動作確認を実施

## Development Environment

### Required Tools
- Rust 2024 Edition
- Windows 10/11 (DirectComposition対応)
- Visual Studio Build Tools (Windows SDKが必要)

### Common Commands
```bash
# Dev (サンプル実行): cargo run --example areka
# Build: cargo build
# Build (Release最適化): cargo build --release
# Test: cargo test
```

## Key Technical Decisions

- **ECS採用**: 複雑なGUI要素の管理とヒットテストロジックをコンポーネントベースで実装
- **DirectComposition**: ハードウェアアクセラレーションによる高速な合成処理と透過ウィンドウの実現
- **DirectWrite**: 高品質な日本語テキストレンダリングと縦書き対応
- **Workspace構成**: 将来的な機能拡張に備えたモノレポ構成（`crates/wintf`）
- **Release最適化**: サイズ最適化（`opt-level='z'`, `lto=true`）でバイナリサイズを削減

---
_Document standards and patterns, not every dependency_
