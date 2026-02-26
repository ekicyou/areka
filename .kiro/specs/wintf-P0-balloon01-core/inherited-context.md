# 親仕様からの引継ぎコンテキスト

> **親仕様**: `wintf-P0-balloon-system`
> **対象要件**: R1（ウィンドウ生成）、R2（フレーム描画）、R3（配置制御）、R4（表示制御）
> **アーキテクチャ要件**: AR-1（複合ウィジェット構造）、AR-2（描画責務分離）、AR-3（描画責務間独立性）

---

## 参照すべき設計情報

### design.md

- **Balloon コンポーネント定義**: anchor（キャラクターEntity参照）、placement、on_add フックによる子エンティティ spawn
- **BalloonSkinDef コンポーネント定義**: スキン定義インターフェース契約（背景・角丸・枠線・しっぽ）
- **BalloonPlacement**: 配置方向・オフセット、デスクトップ領域外自動反転
- **エンティティ階層モデル**: Balloon → BalloonFrame → BalloonContentArea（Mermaid 図参照）
- **コンポーネント構成パターン**: 各エンティティに付与するコンポーネントの組み合わせ
- **モジュール配置**: `ecs/widget/balloon/mod.rs`（Balloon）、`frame.rs`（BalloonFrame, BalloonSkinDef）、`placement.rs`（placement_system）
- **システムフロー「バルーン生成シーケンス」**: on_add → Commands → 子エンティティ spawn
- **制約事項**: ULW / DComp 両モード対応
- **エラーハンドリング戦略**: エンティティ解決エラー → バルーン非表示化、スキン定義エラー → デフォルトスキンにフォールバック

### research.md

- **Errata E1**: on_add フック内で `DeferredWorld::commands()` は使用可能。`on_window_add` が実証済みパターン。thread_local コマンドキューは不要
- **Errata E2**: BalloonContentArea は BalloonFrame の子（`ChildOf(balloon_frame)`）。Balloon の直接の子ではない
- **D1**: BalloonAnchor は `anchor: Entity` フィールド方式。Relation API は安定性未確認のため採用せず

---

## 子仕様スコープ

- バルーンウィンドウの生成・配置・表示制御
- フレーム描画基盤（スキンインターフェース）
- 複合ウィジェットとしてのエンティティ階層構築
- 他の全バルーン子仕様が依存する基盤レイヤー
