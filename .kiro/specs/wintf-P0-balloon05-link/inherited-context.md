# 親仕様からの引継ぎコンテキスト

> **親仕様**: `wintf-P0-balloon-system`
> **対象要件**: R9（クリッカブルテキスト）

---

## 参照すべき設計情報

### design.md

- **LinkRegion コンポーネント定義**: テキスト範囲へのリンク付与。`link_id: String`, `action: String`, `text_range: Range<u32>`, `style: LinkStyle`, `is_hovered: bool`
- **LinkClicked イベント型**: リンククリックイベント（`Phase<LinkClicked>::Bubble` で親チェーンに伝播）。`link_id: String`, `action: String`
- **LinkStyle**: `color`, `hover_color`, `underline` フィールド
- **ヒットテスト方式**: エンティティレベル判定。`hit_test_in_window` → グリフエンティティ特定（`HitTestMode::Bounds`）→ `GlyphInfo.text_position` → `LinkRegion.text_range` マッチ。DirectWrite `HitTestPoint` API は不要
- **ホバー**: `OnPointerMoved` → グリフエンティティ判定 → `LinkRegion.is_hovered` 更新 → Brush 変更
- **モジュール配置**: `ecs/widget/text/link.rs`

### research.md

- **D3**: グリフ分解方式の選択結果がリンク範囲特定に直接影響。グリフエンティティ → `LinkRegion` コンポーネント付与で範囲をマーク
- **G11 改訂**: 1グリフ＝1エンティティ方式により、DirectWrite `HitTestPoint` API のラップは不要。既存の `GlobalArrangement.bounds` によるエンティティレベルヒットテストで対応
- **R4**: 縦書き時のヒットテスト精度は検証が必要

---

## 子仕様スコープ

- テキスト内リンク（LinkRegion）の定義と描画（下線表示）
- グリフエンティティベースのヒットテストによるリンク検出
- ホバー時のビジュアルフィードバック（色変更・カーソル変更）
- クリック時の LinkClicked イベント発火
- リンク情報のテキスト入力形式（BalloonToken::LinkStart / LinkEnd）での受け渡し
- balloon03-content のグリフパイプライン完成が前提
