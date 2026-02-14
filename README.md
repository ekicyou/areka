# areka — デスクトップマスコット・プラットフォーム

> Rust製デスクトップマスコット・プラットフォーム

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue)](LICENSE)
[![Windows](https://img.shields.io/badge/Platform-Windows_10%2F11-0078D6?logo=windows)](https://www.microsoft.com/windows)

---

## プロジェクト概要

**areka** は、デスクトップ上にキャラクター（ゴースト）を常駐させ、ユーザーとの自然な対話を実現するデスクトップマスコット・プラットフォームです。

ECSアーキテクチャ、DirectComposition による高品質レンダリング、日本語縦書き描画、そして宣言的アニメーション定義を基盤に据えています。

---

## スクリーンショット

> 📷 *準備中 — アルファリリースにて掲載予定*

<!--
![areka demo](shell/icon.png)
-->

---

## 二層構造

areka は **UIフレームワーク** と **アプリケーション** の二層で構成されています。

| 層 | クレート | 役割 |
|----|---------|------|
| **フレームワーク** | `wintf` | Windows Tategaki Framework — ECS + DirectComposition + Direct2D による汎用Windows UIフレームワーク |
| **フレームワーク** | `dola` | Declarative Orchestration for Live Animation — 宣言的アニメーション定義フォーマット |
| **アプリケーション** | `areka` *(予定)* | デスクトップマスコット・プラットフォーム本体（バイナリクレート） |
| **外部** | [`pasta`](https://github.com/ekicyou/pasta) | 里々インスパイアの会話記述DSLスクリプトエンジン |

---

## クレート構成

```
areka/                          # ワークスペースルート
├── crates/
│   ├── wintf/                  # Windows Tategaki Framework（UIフレームワーク）
│   ├── dola/                   # 宣言的アニメーション定義
│   └── areka/                  # (予定) マスコットアプリ本体
└── pasta (外部リポジトリ)       # 会話記述DSL
```

詳細は [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) を参照。

---

## 技術スタック

| カテゴリ | 技術 | バージョン |
|---------|------|-----------|
| 言語 | Rust | 2024 Edition |
| ECS | bevy_ecs | 0.18.0 |
| Windows API | windows-rs | 0.62.2 |
| グラフィックス | DirectComposition + Direct2D + Direct3D11 | — |
| テキスト | DirectWrite（縦書き・横書き両対応） | — |
| レイアウト | Taffy (Flexbox) | 0.9.2 |
| アニメーション定義 | dola (JSON / TOML / YAML) | — |
| イメージング | WIC (Windows Imaging Component) | — |

---

## 🎯 アルファリリース目標: ぱすたさん

**ぱすたさん** — areka プロジェクト初のデスクトップマスコット。

| 属性 | 内容 |
|------|------|
| 名前 | ぱすたさん |
| 種別 | ゴースト（pasta DSL 解釈・実行） |
| シェル | 1体キャラクター表示（透過ウィンドウ、60表情パターン） |
| バルーン | 縦書きタイプライター付き吹き出し |
| スクリプト | pasta DSL（里々インスパイアのカスタムDSL） |

詳細は [doc/PASTA_PROFILE.md](doc/PASTA_PROFILE.md) を参照。

---

## 現在の到達点

57件の仕様を完了し、基盤レイヤーの約70%を構築済み。

### ✅ 実装済み基盤機能

**ECS基盤 (7件)**
- [x] bevy_ecs + bevy_app 統合
- [x] コンポーネントグループ整理、エンティティ追跡
- [x] ジェネリックツリーシステム、Changed検出パターン

**グラフィックス / DirectComposition (12件)**
- [x] D3D11 → DXGI → DirectComposition → D2D パイプライン
- [x] ビジュアルツリー実装・ECS自動同期
- [x] Surface生成最適化、VSync優先レンダリング
- [x] デバイスロスト対応、グラフィックリソース再初期化

**レイアウト (6件)**
- [x] Taffy Flexbox統合
- [x] 軸平行バウンディングボックス管理
- [x] BoxStyle統合、レイアウト→グラフィックス同期

**ウィンドウ管理 / DPI (8件)**
- [x] Win32ウィンドウ管理（マルチウィンドウ対応）
- [x] Per-Monitor DPI伝播、マルチモニタ対応
- [x] ECS ↔ Win32 双方向同期

**ポインター / イベント (9件)**
- [x] ヒットテスト（アルファマスク対応、キャッシュ最適化）
- [x] イベント配信（Tunnel/Bubble 2フェーズ）
- [x] ドラッグシステム（エンティティ＋ウィンドウ移動）
- [x] ダブルクリック検出

**ウィジェット (4件)**
- [x] Image ウィジェット（WIC読込、透過PNG、非同期タスクプール）
- [x] Brush コンポーネント分離

**テキスト / 縦書き (2件)**
- [x] 横書き DirectWrite テキストレンダリング
- [x] 日本語縦書きレイアウト（Label コンポーネント）

**タイプライター (1件)**
- [x] 文字単位表示制御、pause/resume/skip

**アニメーション定義 (1件)**
- [x] dola クレート（宣言的アニメーションデータモデル）

**スクリプトエンジン (1件)**
- [x] pasta DSL 設計完了（外部リポジトリ）

---

## 開発ロードマップ概要

| フェーズ | 内容 | 状態 |
|---------|------|------|
| **Phase A** | 基盤完成（イベントシステム残件、アニメーション統合） | 🔵 進行中 |
| **Phase B** | 表示層（バルーンシステム、ウィンドウ配置） | ⚪ 未着手 |
| **Phase C** | コンテンツ（リファレンスシェル/バルーン/ゴースト） | ⚪ 未着手 |
| **Phase D** | アプリ統合（areka クレート、システムトレイ、永続化） | ⚪ 未着手 |
| **Phase E** | アルファ出荷（統合テスト、リリースビルド） | ⚪ 未着手 |

詳細は [doc/ROADMAP.md](doc/ROADMAP.md) を参照。

---

## ビルド手順

### 前提条件

- Rust 2024 Edition (rustup で最新版を推奨)
- Windows 10/11
- Visual Studio Build Tools (Windows SDK)

### コマンド

```bash
# ビルド
cargo build

# サンプル実行
cargo run --example taffy_flex_demo

# テスト
cargo test

# リリースビルド（サイズ最適化）
cargo build --release
```

---

## ドキュメントガイド

| ドキュメント | 内容 |
|-------------|------|
| [doc/CONSTITUTION.md](doc/CONSTITUTION.md) | 設計理念・責務境界・プロジェクト憲法 |
| [doc/ROADMAP.md](doc/ROADMAP.md) | アルファリリースまでのロードマップ |
| [doc/ARCHITECTURE.md](doc/ARCHITECTURE.md) | クレート構成・モジュール構成の俯瞰 |
| [doc/PASTA_PROFILE.md](doc/PASTA_PROFILE.md) | ぱすたさんプロファイル |
| [doc/spec/](doc/spec/) | wintf 詳細設計仕様（12章） |
| [crates/wintf/README.md](crates/wintf/README.md) | wintf クレート概要 |
| [crates/dola/README.md](crates/dola/README.md) | dola クレート概要 |
| [doc/DEVLOG_ORIGINAL_README.md](doc/DEVLOG_ORIGINAL_README.md) | 旧README（開発ログとして保存） |

---

## ライセンス

MIT OR Apache-2.0（[Cargo.toml](Cargo.toml) 参照）

---

<sub>areka — 「あなたのデスクトップに、もうひとりの住人を」</sub>
