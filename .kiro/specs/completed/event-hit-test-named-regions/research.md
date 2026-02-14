# Research & Design Decisions

## Summary
- **Feature**: `event-hit-test-named-regions`
- **Discovery Scope**: Extension（既存ヒットテストシステムの拡張）
- **Key Findings**:
  - `hit_test_entity()` の match 分岐追加パターンが `AlphaMask` 拡張時に確立済み
  - 座標変換は `(point - bounds.left) / bounds_width` の正規化→ターゲット座標スケーリングで統一
  - `serde` は wintf に未依存、optional feature flag で追加可能

## Research Log

### 既存 hit_test_entity の拡張パターン
- **Context**: Named Regions を hit_test.rs にどう統合するかの調査
- **Sources Consulted**: `ecs/layout/hit_test.rs` L142-L198, `alpha_mask.rs` L15-L74
- **Findings**:
  - `hit_test_entity` は `bool` を返す（Entity hit or miss）
  - AlphaMask 分岐: bounds判定→コンポーネント取得→座標変換→ピクセル判定
  - フォールバック: コンポーネント不在時は `true`（Bounds結果を維持）
  - `HitTest` コンポーネント未付与時のデフォルトは `Bounds`
- **Implications**: 
  - Named Regions は同じmatch分岐パターンで追加
  - `hit_test_entity` の返値が `bool` のため、リージョン名は別関数 (`hit_test_entity_ex`) で提供
  - 既存 `hit_test` / `hit_test_in_window` API は `bool` / `Option<Entity>` のまま維持

### 座標変換（screen → local）
- **Context**: ヒット領域定義がローカル座標（DIP単位）のため、変換方式の決定が必要
- **Sources Consulted**: `arrangement.rs` L80-L85, L200-L236, `hit_test.rs` L185-L196
- **Findings**:
  - `GlobalArrangement.bounds` は物理ピクセル座標（LayoutRoot基準）
  - AlphaMask 変換パターン: `rel = (point - bounds.left) / bounds_width` → 0.0〜1.0 正規化 → `mask_coord = rel * mask_size`
  - 軸平行変換のみ（rotation/skew なし）、`Matrix3x2::inverse()` は windows-numerics に不在
  - ローカル座標（DIP単位）への変換: `local = rel * entity_logical_size`
- **Implications**: 
  - AlphaMask と同一パターンで座標変換を実装
  - 矩形/多角形: 正規化 → DIP座標にスケーリング
  - カラーマップ: 正規化 → 画像ピクセル座標にスケーリング（AlphaMask と完全同一）
  - エンティティの論理サイズが必要 → `TaffyComputedLayout` か `Arrangement` から取得

### WIC カラーマップ画像読込
- **Context**: カラーマップ画像のピクセルアクセス方式の調査
- **Sources Consulted**: `com/wic.rs` L115-L123, `alpha_mask.rs` from_pbgra32
- **Findings**:
  - `copy_pixels(rect, stride, buffer)` で全ピクセルデータ取得可能
  - PBGRA32 フォーマット: [B, G, R, A] × width × height
  - AlphaMask は α値を2値化; カラーマップは RGB値をキーとしてルックアップ
  - `WICImagingFactoryExt` → `create_decoder_from_filename` → `frame(0)` → `create_format_converter` → PBGRA32
- **Implications**: 
  - カラーマップ画像キャッシュ: 全ピクセルRGBバッファ `Vec<u8>` + `HashMap<(u8,u8,u8), String>` のルックアップで O(1) 判定
  - AlphaMask のビットパックではなく、ピクセルごと3バイト（RGB）かインデックスで保持
  - 最適化: 読込時に「ピクセル座標 → リージョンID」のインデックスマップに変換可能

### serde optional 依存
- **Context**: wintf への serde 追加方針
- **Sources Consulted**: `crates/wintf/Cargo.toml`, `crates/dola/Cargo.toml`
- **Findings**:
  - wintf: serde 未使用、feature flags なし
  - dola: `serde = { version = "1", features = ["derive"] }` + `serde_json`, `toml`, `serde_yaml` (feature gated)
  - workspace Cargo.toml で serde は共有依存として定義されていない
- **Implications**: 
  - `serde = { version = "1", optional = true, features = ["derive"] }` を wintf/Cargo.toml に追加
  - `[features]` セクション: `serde = ["dep:serde"]`
  - `#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]` マクロで条件付き derive

### D2DRectExt::contains の境界仕様
- **Context**: 矩形リージョン判定における既存 contains の再利用可否
- **Sources Consulted**: `rect.rs` L140-L142
- **Findings**:
  - `x >= left && x <= right && y >= top && y <= bottom` — inclusive 境界
  - 物理ピクセル座標用（f32）
- **Implications**: 矩形リージョンの DIP 座標判定にもそのまま使用可能（f32 の点-in-矩形は汎用）

### event-mouse-basic との統合
- **Context**: マウスイベントシステムでヒットテスト結果がどう消費されるか
- **Sources Consulted**: `.kiro/specs/completed/event-mouse-basic/design.md`
- **Findings**:
  - Phase 1 は `hit_test_placeholder`（常にウィンドウエンティティを返す）
  - `MouseState` コンポーネント: `screen_point`, `local_point`, `left_down`, `right_down`, `middle_down` 等
  - 将来的に `hit_test_in_window` → `hit_test_in_window_ex` に置き換えてリージョン名を `MouseState` に含めることが可能
- **Implications**: 
  - Named Regions は `hit_test_in_window_ex` → `HitTestResult` を返す拡張APIで提供
  - リージョン名の `MouseState` への統合は `event-mouse-basic` Phase 2 以降で検討

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: hit_test.rs 内に集約 | 全てを hit_test.rs に追加 | ファイル数最小、既存パターンの延長 | 692行→1200行以上に肥大化 | 不採用 |
| B: hit_region/ サブモジュール | 新規サブディレクトリ作成 | 明確な責務分離 | ファイル数増加（5-6ファイル） | 過剰分割の懸念 |
| **C: hit_region.rs 単一ファイル** | データ型→ hit_region.rs、判定拡張→ hit_test.rs | 責務分離と簡潔さのバランス | カラーマップWIC依存が hit_region.rs に入る | **採用** |

## Design Decisions

### Decision: HitRegionMap の enum バリアント設計（排他的方式）
- **Context**: カラーマップと矩形/多角形が排他的という要件
- **Alternatives Considered**:
  1. `HitRegionMap` に `Vec<HitRegion>` を持たせ、region type で分岐
  2. `HitRegionMap` を enum (`ShapeRegions(Vec<ShapeRegion>)` | `ColorMap(ColorMapData)`) にする
- **Selected Approach**: enum バリアント方式（Option 2）
- **Rationale**: 排他的設計を型システムで強制。`ShapeRegions` は矩形と多角形の混在を許容し、`ColorMap` はカラーマップ専用
- **Trade-offs**: enum match のボイラープレートが増えるが、不正な状態を型レベルで防止
- **Follow-up**: ビルダーAPIで排他制約をコンパイル時に保証するか、ランタイム検証にするか（設計で決定）

### Decision: 座標変換は AlphaMask 踏襲の線形スケーリング
- **Context**: screen → local 座標変換方式
- **Alternatives Considered**:
  1. Matrix3x2 逆行列を手動実装
  2. euclid クレートの変換機能
  3. AlphaMask 踏襲の bounds ベース線形スケーリング
- **Selected Approach**: 線形スケーリング（Option 3）
- **Rationale**: 現行アーキテクチャは軸平行変換のみ。回転/スキューは transform/ で非推奨。既存パターンとの整合性を優先
- **Trade-offs**: 将来回転対応が必要な場合は方式変更が必要
- **Follow-up**: なし（Structure steering で軸平行変換が明記）

### Decision: カラーマップキャッシュはインデックスマップ方式
- **Context**: カラーマップ画像のピクセルアクセス性能
- **Alternatives Considered**:
  1. 毎回 RGB バッファから色を読み取り、HashMap でルックアップ
  2. 読込時にピクセル→リージョンIDのインデックスマップを構築
- **Selected Approach**: インデックスマップ方式（Option 2）
- **Rationale**: 読込は1回のみ、判定は毎フレーム。O(1) のインデックス参照で高速。メモリはピクセル数 × 1byte（リージョンID）
- **Trade-offs**: 初回読込時のコスト増加（全ピクセル走査）
- **Follow-up**: リージョン数が 256 以下なら u8 インデックス、それ以上なら u16

### Decision: serde は optional feature flag
- **Context**: wintf への serde 依存追加
- **Alternatives Considered**:
  1. 無条件追加
  2. feature flag で optional
  3. areka 層のみで使用
- **Selected Approach**: feature flag（Option 2）
- **Rationale**: wintf はグラフィックス基盤。serde が不要なユースケースではバイナリサイズ・コンパイル時間を節約
- **Trade-offs**: `#[cfg_attr]` のボイラープレート
- **Follow-up**: workspace Cargo.toml に serde を共有依存として追加するか検討

## Risks & Mitigations
- **カラーマップ色比較精度**: 画像編集ソフトのアンチエイリアスで色がずれる → RGB完全一致を基本、マッピング外色は無名ヒット（region: None）として処理
- **HitTestResult 導入の後方互換**: 既存 API は `Option<Entity>` → 既存 API は変更せず `_ex` サフィックスの拡張 API を追加
- **カラーマップメモリ使用量**: 大画像の場合インデックスマップが肥大 → インデックスマップは u8 で 1 byte/pixel、1000x1000 = 1MB 程度で許容範囲

## References
- [windows-rs Matrix3x2](https://microsoft.github.io/windows-docs-rs/doc/windows/Foundation/Numerics/struct.Matrix3x2.html) — inverse() メソッドなし
- [Ray Casting Algorithm](https://en.wikipedia.org/wiki/Point_in_polygon#Ray_casting_algorithm) — 多角形内外判定の標準アルゴリズム
- [CSS clip-path: polygon()](https://developer.mozilla.org/en-US/docs/Web/CSS/basic-shape/polygon) — 多角形定義の参考記法
- [WIC Pixel Formats](https://learn.microsoft.com/en-us/windows/win32/wic/-wic-codec-native-pixel-formats) — PBGRA32 フォーマット仕様
