# Product Overview

**wintf** (Windows Tategaki Framework) は、Windows上で日本語の縦書き描画をサポートするため、bevy_ecsを基盤としたUI実装を提供するRustライブラリです。「伺か」のような日本語縦書きを必要とするデスクトップアプリケーションの基盤として設計されています。

## Core Capabilities

- **ウィンドウ管理**: Win32 APIを使用したウィンドウの生成、管理、メッセージ処理
- **2D描画**: DirectComposition、Direct2D、DirectWriteを活用した高品質な2D描画
- **縦書きテキスト**: DirectWriteによる日本語縦書き・横書き両対応のテキストレンダリング
- **透過ウィンドウ**: レイヤードウィンドウまたはDirectCompositionによる透過処理とヒットテスト
- **画像表示**: WIC (Windows Imaging Component)を使用した透過画像の読み込みと描画

## Target Use Cases

- 「伺か」のようなデスクトップマスコットアプリケーション
- 日本語縦書きテキストを必要とするWindows向けGUIアプリケーション
- 透過ウィンドウと高度なインタラクションを持つデスクトップツール
- DirectComposition/Direct2Dを使用した高パフォーマンスな2D描画アプリケーション

## Value Proposition

Rust言語による型安全性とメモリ安全性を保ちながら、Windows固有の低レベルAPI（DirectComposition、Direct2D、DirectWrite）を使用した日本語縦書き描画を実現します。既存のUIフレームワークでは困難な、透過ウィンドウと縦書きテキストを組み合わせた高度な表現が可能です。

## areka アプリケーション層

**areka** は wintf を基盤としたデスクトップマスコット・プラットフォームです。「伺か」にインスピレーションを得た、ゴースト（キャラクター）常駐型のデスクトップアプリケーションを実現します。

### 構成

- **areka バイナリクレート** *(予定)*: wintf / dola を統合し、ゴースト実行環境を提供
- **dola ライブラリクレート**: 宣言的アニメーション定義フォーマット（JSON/TOML/YAML）
- **pasta** *(外部リポジトリ)*: 里々インスパイアの会話記述DSLスクリプトエンジン

### アルファリリースターゲット: ぱすたさん

| 属性 | 内容 |
|------|------|
| 名前 | ぱすたさん |
| 種別 | ゴースト（pasta DSL 解釈・実行） |
| シェル | 1体キャラクター表示（透過ウィンドウ、60表情パターン） |
| バルーン | 縦書きタイプライター付き吹き出し |

詳細: `doc/PASTA_PROFILE.md`, `doc/ROADMAP.md`

---
_Focus on patterns and purpose, not exhaustive feature lists_
