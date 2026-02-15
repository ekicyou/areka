# Requirements Document

## Project Description (Input)
SetWindowRgn ベースのクリックスルー（クロスプロセス対応）実装。

wintf フレームワークにおいて、HitTest::None エンティティのクリックスルーをクロスプロセスで実現する。
従来の WS_EX_TRANSPARENT + WM_NCHITTEST (HTTRANSPARENT) アプローチは同一スレッド内のウィンドウ間でしか機能しないことが判明したため、SetWindowRgn を使用してウィンドウリージョンからクリックスルー領域を除外する方式に切り替える。

### 背景
- WS_EX_TRANSPARENT: "siblings beneath the window (that were created by the same thread)" のみ対象
- HTTRANSPARENT: DWM Step 2 で同一スレッド内の兄弟ウィンドウのみ転送
- SetWindowRgn: DWM Step 1 でリージョン外をスキップ → クロスプロセスで貫通可能

### 技術要件
- DirectComposition (WS_EX_NOREDIRECTIONBITMAP) との互換性が必要
- HitTest::None エンティティの bounds をリージョンから除外
- レイアウト変更時にリージョンを動的に更新
- ドラッグ中のリージョン一時拡張（ドラッグ操作の継続性保証）

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
