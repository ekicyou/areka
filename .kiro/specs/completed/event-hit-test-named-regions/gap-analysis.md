# ギャップ分析レポート

| 項目 | 内容 |
|------|------|
| **Document Title** | event-hit-test-named-regions ギャップ分析 |
| **Date** | 2026-02-14 |
| **Requirements** | event-hit-test-named-regions/requirements.md v1.0 |
| **Codebase** | crates/wintf (commit: main HEAD) |

---

## 1. 現在の状態（Current State）

### 1.1 関連アセットマップ

| カテゴリ | ファイル/モジュール | 概要 |
|---------|---------------------|------|
| ヒットテスト基盤 | `ecs/layout/hit_test.rs` (692行) | `HitTestMode`{None,Bounds,AlphaMask}, `HitTest`コンポーネント, `hit_test_entity`, `hit_test`, `hit_test_in_window` |
| αマスク | `ecs/widget/bitmap_source/alpha_mask.rs` (207行) | `AlphaMask` struct（ビットパック）, `is_hit(x,y)`, `from_pbgra32()` |
| NCHitTestキャッシュ | `ecs/nchittest_cache.rs` (227行) | `cached_nchittest()` — 1エントリ/HWNDキャッシュ |
| 配置計算 | `ecs/layout/arrangement.rs` | `GlobalArrangement{transform: Matrix3x2, bounds: D2DRect}` |
| 矩形判定 | `ecs/layout/rect.rs` | `D2DRectExt::contains(x,y)` |
| ツリー走査 | `ecs/common/tree_iter.rs` | `DepthFirstReversePostOrder` — 深さ優先逆後順走査 |
| WIC画像読込 | `com/wic.rs` (148行) | `WICBitmapSourceExt::copy_pixels()` によるピクセルデータ取得 |
| BitmapSource | `ecs/widget/bitmap_source/` | 画像ウィジェット、リソース管理、αマスク生成パイプライン |
| イベントハンドラ | `ecs/window_proc/handlers.rs` | WM_MOUSEMOVE / WM_LBUTTONDOWN / WM_NCHITTEST で `hit_test_in_window` 呼出 |
| レイアウトmod | `ecs/layout/mod.rs` | `pub use hit_test::*` で全公開 |

### 1.2 既存の設計パターン

- **HitTestMode拡張パターン**: `event-hit-test-alpha-mask` で確立済み。enumバリアント追加 → `hit_test_entity` 内に分岐追加 → 対応コンポーネント/リソースで判定
- **座標変換**: `GlobalArrangement.bounds` の物理ピクセル座標に対して相対座標(0.0〜1.0)を計算し、マスク座標へスケーリング
- **コンポーネント配置**: `HitTest` は `ecs::layout`、`AlphaMask` は `ecs::widget::bitmap_source` に配置
- **フォールバック**: コンポーネント不在時は上位モード(Bounds)にフォールバック

### 1.3 主要な慣習

- モジュール分離: COM → ECS → Message Handling の依存方向
- `#[derive(Component)]` + `pub struct` + コンストラクタメソッド
- tracing ロギング（構造化フィールド）
- テストは `tests/` ディレクトリに配置

---

## 2. 要件→アセット対応表（ギャップ特定）

| 要件 | 既存アセット | ギャップ | 分類 |
|------|-------------|---------|------|
| **Req 1**: HitTestMode拡張 | `HitTestMode` enum (3バリアント) | `NamedRegions` バリアント追加が必要 | 軽微な拡張 |
| **Req 2**: 矩形ヒット領域 | `D2DRectExt::contains()` | ローカル座標系での名前付き矩形リストの保持・判定ロジック **Missing** | 新規実装 |
| **Req 3**: カラーマップ画像 | `WICBitmapSourceExt::copy_pixels()`, `AlphaMask::from_pbgra32()` | カラーマップ画像読込・色→領域名ルックアップ **Missing**。ピクセルアクセス基盤はあり | 新規実装（WIC基盤は再利用可能） |
| **Req 4**: 多角形領域 | なし | Ray Casting法アルゴリズム **Missing**。外部クレート不要（純粋計算） | 新規実装 |
| **Req 5**: HitTestResult | `hit_test`系は `Option<Entity>` を返すのみ | `HitTestResult` struct、拡張API (`hit_test_ex`) **Missing** | 新規実装 |
| **Req 5.3**: ローカル座標変換 | `GlobalArrangement.transform`（乗算のみ）| **screen→local逆変換 Missing**。`Matrix3x2::inverse()` が windows-numerics に不在 | **Research Needed** |
| **Req 6**: 優先順位 | なし | 定義順序による優先順位 + `priority` オーバーライド **Missing** | 新規実装（設計が必要） |
| **Req 7**: JSON読込 | wintfに `serde` 未依存 | serde/serde_json 依存追加、JSONスキーマ定義 **Missing** | 新規実装 + 依存追加 |
| **Req 8**: HitRegionMap | なし | コンポーネント全体、ビルダーAPI **Missing** | 新規実装 |

### ギャップサマリ

- **既存アセットで直接カバー**: Req 1（enum拡張）のみ
- **基盤の再利用が可能**: Req 3（WICピクセルアクセス）、Req 5.3（transform行列の一部）
- **完全新規**: Req 2, 4, 5, 6, 7, 8

---

## 3. 技術的ギャップ詳細

### 3.1 screen → local 逆変換（Critical）

**現状**: `GlobalArrangement.transform` は `Matrix3x2`（親→子の累積変換行列）。乗算・オフセット・スケール取得は可能だが、**逆行列計算メソッドがない**。

**AlphaMask での回避策**: `bounds` の left/top/width/height から0.0〜1.0の相対座標を計算する線形スケーリング。回転・スキューがない（軸平行変換のみ）前提。

**本仕様での影響**:
- 矩形・多角形のヒット領域はローカル座標（DIP単位）で定義する
- スクリーン座標→ローカル座標変換が必要
- 現在の軸平行変換のみ（steering: `structure.md` で `transform/` は非推奨、`Arrangement` ベース推奨）であれば、bounds相対座標からの線形スケーリングで対応可能

**オプション**:
- **A**: AlphaMaskと同じ線形スケーリング（軸平行変換限定、回転なし前提）— **最もシンプル、現行アーキテクチャと整合**
- **B**: `Matrix3x2` の逆行列を手動実装（2x3行列の逆変換: `ad - bc` で行列式計算）— 汎用的だが、現在不要な複雑さ
- **C**: `euclid` クレート（依存済み？）の変換機能を活用 — **Research Needed**

**推奨**: 設計フェーズで方針決定。P1機能であり、軸平行変換前提でOption Aが低リスク。

### 3.2 serde 依存の追加（Moderate）

**現状**: `wintf` クレートに `serde` / `serde_json` 依存なし。`dola` クレートのみで使用。

**必要な変更**:
- `Cargo.toml` に `serde`, `serde_json` を追加（feature flag推奨: `region-json` 等）
- ワークスペースレベルで `serde`, `serde_json` を共有依存として定義するか検討

**リスク**: wintfはグラフィックス/ECS基盤ライブラリ。JSON読込は上位層（arekaクレート）の責務とする設計もあり得る。

**オプション**:
- **A**: wintfに直接serde依存追加（feature flag付き）
- **B**: `HitRegionMap` をデータのみのstruct（Serialize/Deserialize不要）とし、JSON読込は外部（areka側）で行い、ビルダーAPIで構築
- **C**: 別クレート `wintf-region-io` 等に分離

### 3.3 カラーマップ画像読込（Moderate）

**現状**: WIC画像読込パイプラインは `BitmapSource` ウィジェット用に存在。`copy_pixels()` で全ピクセルデータ取得可能。

**新規に必要な処理**:
1. カラーマップ画像をWICで読込（既存パイプラインの再利用可能）
2. ピクセルデータを `HashMap<Rgb, String>`（色→領域名）でルックアップ
3. 読込結果のキャッシュ（`AlphaMask` パターンに準拠可能）

**ギャップ**:
- 色の比較: RGB完全一致 vs 近似マッチング（閾値付き）— **Research Needed（要設計判断）**
- 画像サイズ≠エンティティサイズ時のスケーリング
- カラーマップ用の `copy_pixels` はPBGRA32フォーマット前提で可能（αチャンネル無視でRGB抽出）

### 3.4 PhysicalPoint の重複（Low）

**現状**: `hit_test::PhysicalPoint` と `pointer::PhysicalPoint` が2つ存在。`ecs/mod.rs` で `pointer` 版を再エクスポート。

**影響**: 拡張API (`hit_test_ex`) の戻り値型に `PhysicalPoint` を使う場合、どちらを使用するか統一が必要。

---

## 4. 実装アプローチオプション

### Option A: 既存コンポーネントの拡張

**方針**: `hit_test.rs` 内に集約。`HitRegionMap` コンポーネントと判定ロジックを同ファイルに追加。

**変更ファイル**:
- `ecs/layout/hit_test.rs` — `HitTestMode::NamedRegions` 追加、`HitRegionMap` 定義、`hit_test_entity` 分岐追加、`hit_test_ex` API追加
- `ecs/layout/mod.rs` — pub use 追加

**トレードオフ**:
- ✅ ファイル数最小、既存パターンの自然な延長
- ✅ AlphaMask拡張と同じパターンで開発者に親しみやすい
- ❌ `hit_test.rs` が692行→推定1200行以上に肥大化
- ❌ 矩形判定/多角形判定/カラーマップ読込が1ファイルに混在
- ❌ serde依存がhit_test全体に波及

### Option B: 新規コンポーネント群を作成

**方針**: `ecs/layout/hit_region/` サブモジュールを新規作成。ヒット領域の定義・判定ロジックを分離。

**新規ファイル**:
- `ecs/layout/hit_region/mod.rs` — `HitRegionMap`, `HitRegion`, `RegionShape` enum
- `ecs/layout/hit_region/shapes.rs` — `RectRegion`, `PolygonRegion`, Ray Casting法
- `ecs/layout/hit_region/color_map.rs` — カラーマップ画像読込・判定
- `ecs/layout/hit_region/builder.rs` — ビルダーAPI
- `ecs/layout/hit_region/serde.rs` — JSON読込（feature flag付き）

**変更ファイル**:
- `ecs/layout/hit_test.rs` — `HitTestMode::NamedRegions` 追加、`hit_test_entity` に分岐1つ追加、`hit_test_ex` API
- `ecs/layout/mod.rs` — `pub mod hit_region;` 追加

**トレードオフ**:
- ✅ 明確な責務分離（hit_test = 判定エンジン、hit_region = 領域データ）
- ✅ 個別テスト容易（Ray Casting法の単体テスト等）
- ✅ serde依存をfeature flag + hit_regionモジュール内に隔離可能
- ✅ `AlphaMask` が `widget/bitmap_source/` にあるように、データは使用箇所の近くに配置するパターンと整合
- ❌ ファイル数増加（5-6ファイル）
- ❌ モジュール間インターフェース設計が必要

### Option C: ハイブリッド（推奨候補）

**方針**: コアデータ型は `ecs/layout/hit_region.rs`（単一ファイル）に配置。カラーマップ画像処理は `widget/bitmap_source/` に近い場所に配置。

**ファイル構成**:
- `ecs/layout/hit_region.rs` — `HitRegionMap`, `HitRegion`, `RegionShape`, ビルダーAPI, 矩形/多角形判定
- `ecs/layout/hit_test.rs` — `NamedRegions` 分岐追加、`HitTestResult`, `hit_test_ex`
- JSON読込は feature flag で分離（wintfへのserde依存を optional に）

**トレードオフ**:
- ✅ ファイル数を抑えつつ責務を分離
- ✅ hit_test.rs の肥大化を回避（分岐追加は数十行）
- ✅ 段階的実装が可能（矩形→多角形→カラーマップ→JSON読込）
- ❌ カラーマップ関連のWIC呼出しが `hit_region.rs` から `com/wic.rs` への依存を生む

---

## 5. 実装複雑度・リスク評価

### 工数見積: **M（3-7日）**

**根拠**:
- 新規データモデル（HitRegionMap, RegionShape）の設計・実装
- Ray Casting法は標準的アルゴリズム（数十行）
- WICパイプラインの再利用でカラーマップ実装を効率化
- 既存パターン（AlphaMask拡張）に従えるため、設計上の不確実性は低い
- JSON読込 + serde統合が追加工数の主因

### リスク: **Medium**

| リスク項目 | 影響 | 対策 |
|-----------|------|------|
| screen→local逆変換 | 座標変換の精度 | 軸平行変換前提で線形スケーリング（AlphaMask踏襲） |
| serde依存追加 | wintfクレートの責務境界 | feature flag化、optional依存 |
| カラーマップの色比較精度 | 画像編集ソフトのアンチエイリアスによる色ずれ | RGB完全一致を基本、近似マッチングはオプション |
| HitTestResult導入による既存API影響 | 後方互換性 | 既存APIは変更せず、拡張APIを別名で追加 |
| PhysicalPoint の型重複 | 型の混乱 | 設計フェーズで統一方針を決定 |

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ: Option C（ハイブリッド）

`hit_region.rs` で領域データ型を集約し、`hit_test.rs` への変更を最小限に抑える。

### 設計フェーズで決定すべき事項

1. **座標変換方式**: 線形スケーリング（AlphaMask踏襲）vs Matrix3x2逆行列
2. **serde依存戦略**: wintfにoptional依存 vs areka層でJSON→ビルダー変換
3. **カラーマップの色比較**: RGB完全一致 vs 近似マッチング（閾値）
4. **HitRegionMap のモジュール配置**: `ecs::layout` vs `ecs::widget` 新サブモジュール
5. **カラーマップ画像データの保持形式**: 全ピクセルRGBバッファ vs 色→座標リストの事前変換（ルックアップテーブル）

### Research Needed items

- `euclid` クレートの依存状況と Matrix3x2 互換性
- `windows-numerics::Matrix3x2` に inverse() が追加される可能性
- カラーマップ画像でアンチエイリアスを考慮した近似色マッチングのベストプラクティス
- 「伺か」の `surfaces.txt` collision 定義フォーマットとの互換性検討
