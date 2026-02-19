# Requirements Document

## Project Description (Input)
Phase 4 方針変更: DComp完全除去 → 切り替え式バックエンド実装。

透過ウィンドウのクリックスルーが必要な場合は ULW（UpdateLayeredWindow）パイプライン、通常のウィンドウUIには DComp パイプラインを使用する切り替え式アーキテクチャを実装する。将来的には DComp を WinRT Compositor（Windows.UI.Composition）ベースへ移行することも視野に入れる。

### 背景・動機
- Phase 1〜3 で DComp → ULW 移行が完了し、現在は ULW パイプラインのみがアクティブ
- DComp のシステム関数・コンポーネント・COM ラッパーはコードとして完全に残存（スケジュール登録のみ解除）
- GraphicsCore は現在も DComp デバイスを初期化している
- デスクトップマスコット（透過・クリックスルー必須）は ULW、通常ウィンドウ UI は DComp（将来は WinRT Compositor）で描画する二刀流が最適
- Window エンティティ単位で合成モードを切り替え、同一アプリ内で ULW ウィンドウと DComp ウィンドウを共存させる

### 設計方針
- Window エンティティに合成モード（CompositionMode enum: ULW / DComp）を持たせ、ウィンドウ単位で描画パイプラインを切り替える
- 描画コマンド生成（GraphicsCommandList）は両パイプラインで共有
- ECS システムは CompositionMode に基づきクエリフィルタリングで分岐
- 将来の WinRT Compositor 対応を見据え、CompositionMode に拡張余地を持たせる

## Requirements
<!-- Will be generated in /kiro:spec-requirements phase -->
