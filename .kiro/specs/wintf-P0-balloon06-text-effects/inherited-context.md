# 親仕様からの引継ぎコンテキスト

> **親仕様**: `wintf-P0-balloon-system`
> **対象要件**: R11（テキストエフェクト）、R12（タイプライター統合）

---

## 参照すべき設計情報

### design.md

- **GlyphDrawData コンポーネント定義**: opacity、color、transform フィールドをアニメーション可能なグリフ描画データ
- **GlyphTimeline / GlyphEffect コンポーネント定義**: グリフ単位のタイムライン管理
- **dola マッピング構造**: dola の Variable → GlyphDrawData フィールドへの自動バインディング
- **タイプライター効果**: opacity 0→1 のシーケンシャル遷移として表現。dola Storyboard で制御
- **テキストエフェクション一覧**: フェードイン、スライドイン、バウンス、シェイク、虹色
- **dola 依存パス**: `crates/dola` を直接パス参照で依存追加（workspace 外 crate）

### research.md — dola 関連

- **依存方式**: `dola = { path = "../../crates/dola" }` で直接参照。dola は workspace メンバーではないが同一リポジトリ内
- **D5**: dola の Storyboard → GlyphDrawData へのバインディングメカニズム。Variable::Float → opacity/transform パラメータへのマッピング
- **D4**: タイプライター効果は dola の SequenceTransition として実装。各グリフに遅延付き opacity 遷移を設定

### research.md — パフォーマンス

- **P1**: dola の毎フレーム update コストは Variable 数に比例。200グリフ × 3パラメータ = 600 Variable でも <1ms を想定。ボトルネックは描画側

---

## 子仕様スコープ

- dola アニメーションシステムとの統合
- グリフ単位のエフェクト適用（opacity, color, transform）
- タイプライター効果（シーケンシャル表示）の dola Storyboard 実装
- テキストエフェクト定義と適用メカニズム
- balloon03-content のグリフパイプラインが前提
