# 親仕様からの引継ぎコンテキスト

> **親仕様**: `wintf-P0-balloon-system`
> **対象要件**: R5（コンテンツ領域管理）、R6（テキストレイアウトとグリフ分割）、R7（テキスト表示制御）、R8（コンテンツスクロール）

---

## 参照すべき設計情報

### design.md

- **BalloonContentArea コンポーネント定義**: コンテンツ領域、マージン・パディング、最大サイズ制約
- **GlyphContainer / GlyphInfo / GlyphDrawData コンポーネント定義**: グリフエンティティの構成
- **ScrollState コンポーネント定義**: スクロール位置管理
- **コンポーネント構成パターン**: GlyphContainer エンティティ、GlyphEntity エンティティの構成
- **システムフロー「テキスト描画パイプライン」**: テキスト入力 → TextLayout → CustomTextRenderer → GlyphEntity spawn
- **モジュール配置**: `ecs/widget/text/glyph.rs`、`glyph_draw.rs`、`glyph_timeline.rs`、`com/dwrite_ext.rs`
- **パフォーマンスとスケーラビリティ**: グリフ spawn ≤5ms（200文字）、ダーティグリフのみ再描画、レイアウト再計算回避
- **将来拡張の考慮テーブル**: コンテンツ領域の拡張ポイント確保（ポートレート・インライン画像の後付け対応）

### research.md — 設計決定

- **D3 rev.1**: CustomTextRenderer（IDWriteTextRenderer1 実装）採用。DrawGlyphRun コールバックでグリフデータをキャプチャ。既存 RecCommandSink パターンで COM 実装複雑さ解消済み
- **D3 rev.2（★本子仕様で最終決定）**: グリフ分解粒度の選択
  - 案A: グリフラン単位（最シンプル、文字単位アニメ不可）
  - 案B: 文字単位分解（clusterMap、合字困難）
  - 案C: ハイブリッド
  - 案D: グリフ単位分解（glyphCount ループ、完全な1グリフ=1エンティティ）
  - 暫定方針: 案D を想定した記述を採用、代替案への変更余地あり
- **D6**: PushAxisAlignedClip によるクリッピング。SetTransform の変換行列影響あり、ローカル座標指定。バルーンでは軸平行変換のみのため問題なし
- **D8**: テキスト変更時は全グリフ despawn → 新グリフ spawn（全再構築方式）

### research.md — パフォーマンス戦略

- **P2**: ULW 完全サポート・文字数制限なし。既存タイプライター実装（文字更新ごとに全レイアウト再計算）との比較ベンチマーク実施。フォールバック: コンテンツ領域専用サーフェス + CommandList 焼き付け最適化
- **Errata E3**: DComp モード推奨（CommandList 焼き付け後は SetOpacity/SetTransform のみで GPU 合成）。ULW モードは毎フレーム N×DrawGlyphRun CPU 実行コストあり
- **R1**: ULW パフォーマンスリスク — 初回レイアウト + 以降はプロパティ切替のみの設計で既存実装より高速なはず

### research.md — スコープ外決定

- **P3**: インライン要素（テキスト行内画像・絵文字）は P0 範囲外。将来はグリフエンティティ延長方式または WebView が有力

---

## 子仕様スコープ

- コンテンツ領域のビューポート描画（DR-2）とテキスト基本描画（DR-3）
- DirectWrite 縦横書きテキストレイアウト構築
- グリフ分割パイプライン（テキスト → グリフエンティティ群）
- テキスト表示制御（タイプライター効果、ウェイト調整、dola マッピング構造）
- コンテンツスクロール（自動追従・マウスホイール・ページ送り）
- 既存 typewriter P0 は参考実装として活用、新規実装が必要
