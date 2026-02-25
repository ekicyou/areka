# Research & Design Decisions

## Summary
- **Feature**: `type-consolidation`
- **Discovery Scope**: Extension（Light Discovery）
- **Key Findings**:
  1. windows 0.62.2 に `D2D_POINT_2F` は存在しない — `Vector2 { X: f32, Y: f32 }` が代替
  2. Win32 `SIZE` のフィールドは `cx`/`cy` であり `width`/`height` ではない — ただし `#[repr(C)]` 同一レイアウトにより transmute 安全
  3. transform モジュールの型には `#[deprecated]` 属性が未付与（ドキュメントのみ非推奨）

## Research Log

### 1. D2D/Win32 構造体メモリレイアウト検証

- **Context**: Req2-4, Req6 の `#[repr(C)]` メモリレイアウト互換戦略の根拠を実コードで検証
- **Sources Consulted**: windows 0.62.2 / windows-numerics 0.3.1 のソースコード（Cargo registry）
- **Findings**:
  | 外部型 | フィールド順 | `#[repr(C)]` | derive |
  |--------|-------------|-------------|--------|
  | `D2D_RECT_F` | `left: f32, top: f32, right: f32, bottom: f32` | ✅ | `Clone, Copy, Debug, Default, PartialEq` |
  | `D2D_SIZE_F` | `width: f32, height: f32` | ✅ | `Clone, Copy, Debug, Default, PartialEq` |
  | `POINT` | `x: i32, y: i32` | ✅ | `Clone, Copy, Debug, Default, PartialEq` |
  | `SIZE` | `cx: i32, cy: i32` | ✅ | `Clone, Copy, Debug, Default, PartialEq` |
  | `Vector2` | `X: f32, Y: f32` | ✅ | `Clone, Copy, Debug, Default, PartialEq` |

- **Implications**:
  - `D2D_POINT_2F` は windows 0.62.2 に**存在しない**。D2D API は `Vector2`（PascalCase フィールド `X`, `Y`）を直接使用。`PointF` のメモリレイアウト互換ターゲットは `Vector2` となる
  - `SIZE { cx, cy }` と `SizeI { width, height }` はフィールド名は異なるが、`#[repr(C)]` + 同一フィールド順・同一型により transmute 安全。ただし `From` 実装では明示的フィールドマッピング（`cx ↔ width`）が可読性の点で推奨
  - `D2D_SIZE_F { width, height }` と独自 `Size { width, height }` は名前・順序とも完全一致

### 2. ecs/mod.rs 再エクスポート構造

- **Context**: 新型導入時の公開パス設計（Req1, Req7）
- **Sources Consulted**: `crates/wintf/src/ecs/mod.rs`（全73行）
- **Findings**:
  - `pub use layout::*` → `Size`, `Offset`, `LayoutScale`, `D2DRect`, `D2DRectExt`, `Arrangement`, `GlobalArrangement` 等がフラットに公開
  - `pub use pointer::{ ..., PhysicalPoint, ... }` → pointer 版 `PhysicalPoint`（i32）のみが明示的にエクスポート
  - `pub use transform::*` → `Transform`, `GlobalTransform`, `Translate`, `Scale`, `Rotate` 等
  - `pub use window::{ ..., WindowPos, ... }`
- **Implications**: 新共通型を `layout/` 配下に定義すれば `pub use layout::*` で自動公開される。専用サブモジュール（`ecs/types/`）を新設する場合は `ecs/mod.rs` に `pub use types::*` を追加する必要がある

### 3. WindowPos コンポーネントの Win32 型使用パターン

- **Context**: Req6 の Win32 型置換の影響範囲調査
- **Sources Consulted**: `crates/wintf/src/ecs/window/window_pos.rs`（全436行）
- **Findings**:
  - `pub position: Option<POINT>`, `pub size: Option<SIZE>` として直接保持
  - `set_window_pos()` で `pos.x, pos.y` / `size.cx, size.cy` を直接フィールドアクセス
  - `to_window_coords()` で `POINT { x: 0, y: 0 }`, `SIZE { cx: 0, cy: 0 }` リテラル構築
  - `to_window_coords_for_creation()` で `position.x == CW_USEDEFAULT`, `size.cx == CW_USEDEFAULT` 比較
- **Implications**: `Point`/`SizeI` に変更する場合、Win32 API 呼び出し境界で `.into()` 変換が必要。フィールドアクセスパターンが `x`/`y`, `width`/`height` に変わるため、`cx`/`cy` 参照箇所は全て書き換えが必要

### 4. Arrangement / GlobalArrangement 詳細

- **Context**: Req3, Req4 の既存コンポーネントへの影響評価
- **Sources Consulted**: `crates/wintf/src/ecs/layout/arrangement.rs`（全218行）
- **Findings**:
  - `Arrangement` → `offset: Offset, scale: LayoutScale, size: Size` — 全て既存 metrics 型
  - `GlobalArrangement` → `transform: Matrix3x2, bounds: D2DRect` — `D2DRect` は `D2D_RECT_F` エイリアス
  - `D2D_RECT_F` リテラル直接構築が3箇所: `default()`, `local_bounds()`, `Mul<Arrangement>`
- **Implications**: 共通型 `Rect` 導入後、`D2DRect` が `Rect` のエイリアスになれば、`GlobalArrangement.bounds` の型は自動的に `Rect` になる。リテラル構築箇所は `Rect { left, top, right, bottom }` に書き換え

### 5. D2DRectExt トレイト全メソッド

- **Context**: Req4 AC4 のトレイト移植範囲の確定
- **Sources Consulted**: `crates/wintf/src/ecs/layout/rect.rs`（全165行）
- **Findings**:
  | メソッド | シグネチャ | 備考 |
  |----------|-----------|------|
  | `from_offset_size` | `(offset: Offset, size: Size) -> Self` | 構築 |
  | `width` | `(&self) -> f32` | `right - left` |
  | `height` | `(&self) -> f32` | `bottom - top` |
  | `offset` | `(&self) -> Vector2` | 左上座標 |
  | `size` | `(&self) -> Vector2` | サイズ |
  | `set_offset` | `(&mut self, offset: Vector2)` | 左上設定 |
  | `set_size` | `(&mut self, size: Vector2)` | サイズ設定 |
  | `set_left/top/right/bottom` | `(&mut self, val: f32)` | 個別設定 |
  | `contains` | `(&self, x: f32, y: f32) -> bool` | 点包含判定 |
  | `union` | `(&self, other: &Self) -> Self` | 外接矩形 |
  | `validate` | `(&self)` | debug_assertions のみ |
  
  自由関数: `transform_rect_axis_aligned(rect: &D2DRect, matrix: &Matrix3x2) -> D2DRect`

- **Implications**: `offset()`/`size()` が `Vector2` を返す点が要注意。独自 `Rect` 移行時に `PointF`/`Size` に変更するか、`Vector2` 互換を維持するかの設計判断が必要

### 6. PhysicalPoint 二重定義の詳細

- **Context**: Req2 の PhysicalPoint 統合方針
- **Sources Consulted**: `ecs/pointer/types.rs`, `ecs/layout/hit_test/mod.rs`, `ecs/window_proc/mouse_*.rs`
- **Findings**:
  | モジュール | フィールド型 | 用途 | derive |
  |-----------|-------------|------|--------|
  | `pointer::PhysicalPoint` | `i32` | Win32マウスメッセージ座標 | `Debug, Clone, Copy, Default, PartialEq, Eq` |
  | `hit_test::PhysicalPoint` | `f32` | ヒットテスト座標 | `Debug, Clone, Copy, PartialEq` |
  
  - `ecs/mod.rs` は pointer 版のみ公開
  - `mouse_move.rs`, `mouse_click.rs`, `mouse_dblclick_wheel.rs` で `PhysicalPoint as HitTestPoint` エイリアス使用
  - `hit_test_in_window` 内で `position.x as f32` の i32→f32 変換が発生
- **Implications**: `pointer::PhysicalPoint` → `Point { x: i32, y: i32 }`, `hit_test::PhysicalPoint` → `PointF { x: f32, y: f32 }` に統合することで、名前衝突を根本解消

### 7. transform/components.rs の非推奨状態

- **Context**: Req5 の `#[deprecated]` マーキング対象の確認
- **Sources Consulted**: `crates/wintf/src/ecs/transform/components.rs`（全194行）
- **Findings**:
  - `#[deprecated]` 属性は**未付与**。全型（`Transform`, `GlobalTransform`, `Translate`, `Scale`, `Rotate`, `Skew`, `TransformOrigin`, `TransformTreeChanged`）が現役コード上は非推奨マークなし
  - steering の `structure.md` とギャップ分析ドキュメントでは「非推奨」と記載されているが、コード上の属性はない
  - `Translate { x: f32, y: f32 }` と `Offset { x: f32, y: f32 }` が同一メモリレイアウトだが、意味的に区別（layout offset vs CSS transform translate）
- **Implications**: Req5 AC2 の実装として `#[deprecated(since = "...", note = "...")]` 属性の付与が必要

### 8. SIZE フィールド名のコードベース使用状況

- **Context**: `SizeI { width, height }` 導入時の `SIZE { cx, cy }` 書き換え箇所の特定
- **Sources Consulted**: プロジェクト全体の `SIZE` 使用箇所
- **Findings**:
  | ファイル | パターン |
  |---------|---------|
  | `window_pos.rs:58` | `pub size: Option<SIZE>` フィールド定義 |
  | `window_pos.rs:306` | `size.cx, size.cy` 直接アクセス |
  | `window_pos_systems.rs:56` | `SIZE { cx: width.ceil() as i32, cy: height.ceil() as i32 }` |
  | `render.rs:14` | `use windows::Win32::Foundation::SIZE` インポート |
  | `dpi_helpers.rs:119` | テスト内使用 |
- **Implications**: `WindowPos.size` を `SizeI` に変更する場合、上記全箇所で `cx`/`cy` → `width`/`height` への書き換え＋Win32 API 境界での `.into()` 変換追加が必要

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: ecs/types.rs 単一ファイル | 全共通型を1ファイルに集約 | シンプル、変更最小 | ファイル肥大化リスク | 型数が6-8個であれば許容範囲 |
| B: ecs/types/ ディレクトリモジュール | point.rs, size.rs, rect.rs, convert.rs に分割 | 責務分離、拡張容易 | ファイル数増加 | 将来の型追加に備える |
| C: layout/types.rs に配置 | 既存 layout/ 内に新設 | `pub use layout::*` で自動公開 | layout 依存の印象 | 最小変更だが概念的に不正確 |

**Selected**: **Option A（ecs/types.rs 単一ファイル）** — 理由:
- 導入する型数（`Point`, `PointF`, `Size`, `SizeI`, `Offset`, `Rect`）に対してディレクトリは過剰
- 全型が `#[repr(C)]` + フィールドのみのシンプルな構造体で、ロジックが少ない
- `From`/`Into` 実装も各型で数行程度
- 将来的に型数が増えた場合、ディレクトリモジュールへの移行は後方互換を保って実施可能

## Design Decisions

### Decision: D2DRectExt の offset()/size() 戻り値型

- **Context**: 現在 `Vector2` を返しているメソッドを共通型に変更するか
- **Alternatives Considered**:
  1. `PointF`/`Size` に変更 — 型体系の一貫性向上
  2. `Vector2` を維持 — 既存呼び出し元の変更不要
- **Selected Approach**: `PointF`/`Size` に変更
- **Rationale**: 共通型体系の中核メソッドが外部型を返すのは不整合。`PointF`/`Size` → `Vector2` の `From` 変換を提供すれば、呼び出し元は `.into()` で対応可能
- **Trade-offs**: 呼び出し元の軽微な修正が必要だが、型安全性が向上
- **Follow-up**: `Vector2` を直接必要とする呼び出し元の特定と移行パス確認

### Decision: WindowPos の Win32 型置換方針

- **Context**: `WindowPos` が `Option<POINT>`/`Option<SIZE>` を直接保持
- **Alternatives Considered**:
  1. `Option<Point>`/`Option<SizeI>` に変更 — ECS 層の Win32 依存排除
  2. Win32 型を維持し、境界でのみ変換 — 変更量最小
- **Selected Approach**: `Option<Point>`/`Option<SizeI>` に変更（Req6 AC2 準拠）
- **Rationale**: WindowPos はコンポーネント（ECS層の公開API）であり、Win32 型がコンポーネントのフィールドに露出すべきでない。メモリレイアウト互換により変換コストはゼロ
- **Trade-offs**: `cx`/`cy` → `width`/`height` の書き換えが必要。`CW_USEDEFAULT` 比較パターンは `Point`/`SizeI` のフィールド名に合わせて修正
- **Follow-up**: `window_pos_systems.rs` での SIZE リテラル構築パターンの確認

### Decision: PointF のメモリレイアウト互換ターゲット

- **Context**: windows 0.62.2 に `D2D_POINT_2F` が存在しない
- **Alternatives Considered**:
  1. `Vector2 { X, Y }` を互換ターゲットとする
  2. 互換ターゲットを設定しない（独立定義のみ）
- **Selected Approach**: `Vector2` を互換ターゲットとする
- **Rationale**: D2D API が `Vector2` をポイント型として使用している事実に基づく。`#[repr(C)]` + `f32 × 2` でメモリレイアウト互換。`From<PointF> for Vector2` / `From<Vector2> for PointF` を提供
- **Trade-offs**: `Vector2` は `windows-numerics` クレート由来であり、wintf の直接依存に含める必要がある（既に依存済み）
- **Follow-up**: requirements.md の Req2 AC2 は「D2D1 `D2D_POINT_2F` と互換」と記載 — 設計上は `Vector2` が実際のターゲットである旨をドキュメント化

### Decision: LayoutScale のスコープ

- **Context**: gap-analysis.md の Research Needed #4
- **Alternatives Considered**:
  1. 共通型モジュールに移動 — 汎用スケール型として
  2. layout/ に維持 — レイアウト専用として
- **Selected Approach**: layout/ モジュールに維持
- **Rationale**: `LayoutScale` は `Default` が `(1.0, 1.0)` であり、レイアウトスケール専用のセマンティクスを持つ。唯一の使用箇所は `Arrangement.scale`。DPI コンポーネントは `u16` 型で別概念。汎用スケール型のニーズは現時点で存在しない
- **Trade-offs**: 将来的に汎用スケールが必要になった場合、改めて共通化を検討
- **Follow-up**: なし

## Risks & Mitigations

- **高リスク**: PhysicalPoint の二重定義統合 — `HitTestPoint` エイリアス使用箇所（mouse_move.rs 等）の全書き換えが必要 → 移行時に `pub type HitTestPoint = PointF;` を一時的に提供し段階移行
- **高リスク**: `D2D_POINT_2F` 非存在の発見 — requirements.md に「D2D_POINT_2F 互換」と記載あり → design.md で `Vector2` が実際のターゲットであることを明記（requirements の意図は満たされる）
- **中リスク**: D2DRectExt の offset()/size() 戻り値型変更 — `Vector2` を返す呼び出し元への影響 → `From` 変換提供＋段階移行
- **中リスク**: WindowPos の `cx`/`cy` → `width`/`height` フィールド名変更 — 比較ロジック（`CW_USEDEFAULT`）への影響 → `SizeI` のフィールドで同様の比較が可能
- **低リスク**: transform モジュールに `#[deprecated]` 未付与 — Req5 で対応予定、影響範囲は限定的

## References

- [windows 0.62.2 D2D_RECT_F 定義](https://docs.rs/windows/0.62.2) — `#[repr(C)]` 確認済み
- [windows-numerics 0.3.1 Vector2 定義](https://docs.rs/windows-numerics/0.3.1) — D2D_POINT_2F の代替
- [Rust Reference: repr(C)](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation) — フィールド名はレイアウトに影響しない
