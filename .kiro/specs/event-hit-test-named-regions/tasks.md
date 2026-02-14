# Implementation Plan

## Overview
本実装計画は、名前付きヒット領域システム（event-hit-test-named-regions）の実装タスクを定義します。矩形・多角形・カラーマップ画像による部位別ヒット判定を、既存のヒットテストシステムに統合します。

## Task Breakdown

- [x] 1. 基盤整備と型定義
- [x] 1.1 (P) serde optional feature 追加
  - Cargo.toml に serde 依存を optional として追加（`serde = { version = "1", optional = true, features = ["derive"] }`）
  - features セクションに `serde = ["dep:serde"]` を追加
  - ビルドとコンパイル確認
  - _Requirements: 7.4_

- [x] 1.2 (P) HitTestMode enum に NamedRegions バリアント追加
  - `ecs/layout/hit_test.rs` の HitTestMode に NamedRegions バリアントを追加
  - 既存バリアント（None, Bounds, AlphaMask）の動作に影響しないことを確認
  - Debug, Clone, Copy, PartialEq, Eq トレイト derive
  - _Requirements: 1.1, 1.4_

- [x] 1.3 (P) HitRegionError 型定義
  - `ecs/layout/hit_region.rs` に HitRegionError enum を定義
  - InsufficientVertices（頂点数不足）、InvalidRectSize（矩形サイズ不正）、ImageLoadFailed（画像読込失敗）のバリアントを実装
  - thiserror::Error derive と各バリアントのエラーメッセージ設定
  - _Requirements: 7.3_

- [x] 2. データ構造とビルダー実装
- [x] 2.1 (P) ShapeRegion と Shape enum 実装
  - Shape enum（Rect, Polygon）を定義、DIP 単位の座標を保持
  - ShapeRegion struct（name, shape）を定義
  - `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` を適用
  - Debug, Clone トレイト derive
  - _Requirements: 2.1, 2.2, 2.3, 4.1, 4.2, 4.3_

- [x] 2.2 (P) RegionKind enum 実装
  - RegionKind enum（Shapes(Vec<ShapeRegion>), ColorMap(ColorMapData)）を定義
  - 排他的バリアント設計により矩形/多角形方式とカラーマップ方式を分離
  - `#[cfg_attr(feature = "serde", ...)]` を適用
  - Debug, Clone トレイト derive
  - _Requirements: 8.2, 8.5, 8.6_

- [x] 2.3 (P) ColorMapData と ColorMapDef 実装
  - ColorMapData struct（index_map, region_names, width, height）を定義（キャッシュデータ、serde対象外）
  - ColorMapDef struct（image_path, mapping）を定義（serde対象）
  - ColorMapping struct（color: [u8; 3], name: String）を定義
  - Debug, Clone トレイト derive
  - _Requirements: 3.1, 3.2, 3.3_

- [x] 2.4 HitRegionMapBuilder 実装
  - HitRegionMapBuilder struct と new(), rect(), polygon(), build() メソッドを実装
  - build() でバリデーションエラー時に HitRegionError を返す（パニックしない）
  - 矩形（width > 0, height > 0）、多角形（vertices.len() >= 3）のバリデーションロジック実装
  - _Requirements: 8.3, 7.3_

- [x] 3. カラーマップ読み込みとキャッシュ
- [x] 3.1 カラーマップ画像読み込み実装
  - ColorMapData::from_image() メソッドを実装
  - WIC（com::wic）を使用した PNG デコード（create_decoder_from_filename → frame(0) → format_converter PBGRA32）
  - 全ピクセル走査で RGB→リージョンID 変換、index_map（Vec<u8>）構築
  - マッピング外色は ID 0（無名）として扱う
  - _Requirements: 3.7, 3.8_

- [x] 3.2 HitRegionMap::from_color_map 実装
  - HitRegionMap コンポーネントの from_color_map() ファクトリメソッドを実装
  - HashMap<(u8, u8, u8), String> からカラーマッピングを受け取る
  - 内部で ColorMapData::from_image() を呼び出し、RegionKind::ColorMap を構築
  - windows::core::Result<Self> を返す
  - _Requirements: 8.4_

- [x] 4. 判定ロジック実装
- [x] 4.1 (P) point_in_polygon 関数実装
  - Ray Casting 法による多角形内外判定アルゴリズムを実装
  - 凸多角形・凹多角形両対応、辺上の点の処理を考慮
  - O(n) 計算量（n = 頂点数）、計算最適化
  - _Requirements: 4.5_

- [x] 4.2 HitRegionMap と hit_test_region メソッド実装
  - HitRegionMap struct（kind: RegionKind）を定義
  - Component derive と `#[cfg_attr(feature = "serde", ...)]` を適用
  - hit_test_region(rel_x, rel_y, entity_size) メソッドを実装（正規化座標パターン踏襲）
  - Shapes 方式: local_x = rel_x * entity_size.width で DIP ローカル座標変換、定義順先勝ちルール
  - ColorMap 方式: pixel_x = (rel_x * width) as u32 で画像ピクセル座標変換
  - _Requirements: 8.1, 8.7, 8.8, 6.1, 6.2, 6.3_

- [x] 5. ヒットテスト拡張API
- [x] 5.1 (P) HitTestResult 型定義
  - HitTestResult struct（entity: Entity, region: Option<String>）を定義
  - Debug, Clone トレイト derive
  - 既存モード（Bounds, AlphaMask）でのフォールバック動作（region: None）を設計
  - _Requirements: 5.1, 5.2_

- [x] 5.2 hit_test_entity_ex 実装
  - hit_test_entity_ex(world, entity, point) -> RegionHit 内部関数を実装
  - RegionHit enum（Miss, Hit(Option<String>)）を定義
  - HitTestMode::NamedRegions 分岐を追加、HitRegionMap クエリとフォールバック（1.3）を実装
  - GlobalArrangement.bounds からの正規化座標計算（rel_x, rel_y）、Arrangement.size 取得
  - HitRegionMap::hit_test_region() 呼び出しとリージョン名解決
  - _Requirements: 1.2, 1.3, 5.3, 5.4, 5.5_

- [x] 5.3 hit_test_ex と hit_test_in_window_ex 実装
  - hit_test_ex(world, root, screen_point) -> Option<HitTestResult> を実装
  - hit_test_in_window_ex(world, window, client_point) -> Option<HitTestResult> を実装
  - 既存 DepthFirstReversePostOrder ツリー走査を再利用、hit_test_entity_ex から最前面エンティティと領域名を取得
  - 既存 hit_test / hit_test_in_window API の後方互換性維持（内部で hit_test_entity_ex を呼び出すリファクタリング可能）
  - _Requirements: 5.1, 5.6_

- [x] 6. テスト実装
- [x] 6.1 (P) ユニットテスト（型、ビルダー、ジオメトリ判定）
  - point_in_polygon: 凸多角形、凹多角形、辺上の点、外部の点
  - ShapeRegion::Rect: 境界値（inclusive）、内部、外部
  - HitRegionMapBuilder::build: 正常構築、バリデーションエラー（頂点不足、負サイズ）
  - ColorMapData::hit_test: マッピング内色、マッピング外色、範囲外座標
  - HitRegionMap::hit_test_region: Shapes 方式の先勝ちルール検証
  - _Requirements: 2.4, 2.5, 4.4, 4.6, 6.1, 6.3, 7.3_

- [x] 6.2 (P) 統合テスト（ヒットテスト、座標変換、フォールバック）
  - hit_test_entity_ex + NamedRegions + HitRegionMap: 矩形領域ヒット/ミス
  - hit_test_entity_ex + NamedRegions + HitRegionMap 不在: Bounds フォールバック（region: None）
  - hit_test_ex / hit_test_in_window_ex: ツリー走査で最前面エンティティとリージョン名を返す
  - 座標変換: screen→正規化座標→ローカル/ピクセル座標の変換精度（bounds 基準、AlphaMask パターン踏襲）
  - 既存 hit_test / hit_test_in_window の後方互換性検証（NamedRegions モード以外の動作不変）
  - _Requirements: 1.3, 3.4, 3.5, 3.6, 5.4, 5.5, 5.6_

- [ ] 6.3* (P) レンダリングベースラインテスト（オプショナル）
  - acceptance criteria（2.4, 2.5, 3.4, 3.5, 4.4, 4.5, 4.6）に対するレンダリングベースライン検証
  - カラーマップ画像スケーリング、多角形内外判定の視覚的確認
  - MVP 後に延期可能
  - _Requirements: 2.4, 2.5, 3.4, 3.5, 4.4, 4.5, 4.6_

## Requirements Coverage Check

| Requirement | Summary | Task Coverage |
|-------------|---------|---------------|
| 1.1 | HitTestMode に NamedRegions 追加 | 1.2 |
| 1.2 | Bounds 判定後に領域名解決 | 5.2 |
| 1.3 | HitRegionMap 不在時フォールバック | 5.2, 6.2 |
| 1.4 | 既存モード維持 | 1.2 |
| 2.1 | 矩形領域定義サポート | 2.1 |
| 2.2 | 矩形領域データ保持 | 2.1 |
| 2.3 | 矩形座標ローカル系DIP | 2.1 |
| 2.4 | 矩形領域ヒット判定 | 6.1, 6.3* |
| 2.5 | 複数矩形領域 | 6.1 |
| 3.1 | カラーマップ画像サポート | 2.3 |
| 3.2 | カラーマップデータ保持 | 2.3 |
| 3.3 | RGB マッピング | 2.3 |
| 3.4 | マッピング内色の領域名返却 | 6.2, 6.3* |
| 3.5 | マッピング外色の無名ヒット | 6.2, 6.3* |
| 3.6 | カラーマップ画像スケーリング | 6.2, 6.3* |
| 3.7 | WIC 画像読み込み | 3.1 |
| 3.8 | カラーマップキャッシュ | 3.1 |
| 4.1 | 多角形領域サポート | 2.1 |
| 4.2 | 多角形データ保持 | 2.1 |
| 4.3 | 多角形座標ローカル系DIP | 2.1 |
| 4.4 | 多角形頂点数検証 | 6.1, 6.3* |
| 4.5 | Ray Casting 法内外判定 | 4.1, 6.3* |
| 4.6 | CSS polygon 記法対応 | 6.1, 6.3* |
| 4.7 | 矩形と多角形混在 | (2.1, 2.2 で構造的に実現) |
| 5.1 | HitTestResult 提供 | 5.1 |
| 5.2 | HitTestResult フィールド | 5.1 |
| 5.3 | 拡張API（hit_test_ex）提供 | 5.2, 5.3 |
| 5.4 | 既存モードで region: None | 6.2 |
| 5.5 | NamedRegions モードで region 返却 | 6.2 |
| 5.6 | 既存API後方互換 | 5.3, 6.2 |
| 6.1 | 重複領域の先勝ち順序 | 4.2, 6.1 |
| 6.2 | カラーマップ重複なし | 4.2 |
| 6.3 | 混在時の定義順序優先 | 4.2, 6.1 |
| 7.1 | serde Serialize/Deserialize | 1.1, 2.1, 2.2, 2.3 |
| 7.2 | シリアライズ可能構造 | 2.1, 2.2, 2.3 |
| 7.3 | バリデーションエラー処理 | 1.3, 2.4, 6.1 |
| 7.4 | serde optional 依存 | 1.1 |
| 7.5 | JSON/TOML パース上位層責務 | (設計決定、実装不要) |
| 8.1 | HitRegionMap コンポーネント | 4.2 |
| 8.2 | RegionKind 排他的設計 | 2.2 |
| 8.3 | ビルダーAPI | 2.4 |
| 8.4 | color_map メソッド | 3.2 |
| 8.5 | カラーマップ方式の排他性 | 2.2 |
| 8.6 | 矩形/多角形方式の排他性 | 2.2 |
| 8.7 | hit_test_region メソッド | 4.2 |
| 8.8 | 空の場合の None 返却 | 4.2 |

## Implementation Notes

- **並列実行可能タスク**: (P) マーカー付きタスクは独立して並列実行可能
- **依存関係**: 
  - 3.1, 3.2 は 2.3 に依存（ColorMapData 定義が必要）
  - 4.2 は 2.1, 2.2, 4.1 に依存（ShapeRegion, RegionKind, point_in_polygon が必要）
  - 5.2, 5.3 は 4.2, 5.1 に依存（HitRegionMap, HitTestResult が必要）
  - 6.1, 6.2 はすべての実装タスク完了後に実行
- **オプショナルテスト**: 6.3* はベースライン検証であり、MVP 後に延期可能
- **AlphaMask パターン踏襲**: 座標変換（正規化座標 0.0〜1.0）、hit_test.rs L215-220 実装を参照
