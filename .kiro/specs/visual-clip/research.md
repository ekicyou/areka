# Research & Design Decisions: visual-clip

## Summary
- **Feature**: visual-clip
- **Discovery Scope**: Extension（既存 Visual / レンダリングシステムの拡張）
- **Key Findings**:
  - DComp `IDCompositionVisual3::SetClip` は `IDCompositionClip` を受け取り、`None` でクリップ解除可能
  - ULW パスの `render_subtree` は Transform → Draw → Children の順序で、Push/Pop をその流れに挿入可能
  - D2D コマンドインフラ（PushAxisAlignedClip, PushLayer, PopAxisAlignedClip, PopLayer）は既存
  - `ID2D1RoundedRectangleGeometry` ラッパーのみ新規追加が必要

## Research Log

### DComp SetClip API — windows-rs での呼び出し方法
- **Context**: Req 4 AC5 で `Visual.clip = None` 時に null を渡してクリップ解除する必要がある。windows-rs での null パラメーター渡しの具体的方法を確認。
- **Sources Consulted**: windows-rs の `Param<T>` トレイト仕様、既存 `dcomp.rs` の `set_content`, `add_visual` パターン
- **Findings**:
  - `IDCompositionVisual3::SetClip` は `P0: Param<IDCompositionClip>` 型パラメーターを受け取る
  - `Param<T>` トレイトにより `None` を渡すと null ポインタとして解釈される
  - 既存パターン: `set_content<P0: Param<IUnknown>>` が同じ方式（`None` で content 解除）
  - `set_clip` のシグネチャ: `fn set_clip<P0>(&self, clip: P0) -> Result<()> where P0: Param<IDCompositionClip>`
- **Implications**: Option B の clip_sync_system で `Some(clip_shape)` → `set_clip(rectangle_clip)`, `None` → `set_clip(None::<IDCompositionClip>)` のパターンで実装可能

### DComp IDCompositionRectangleClip 作成
- **Context**: Req 6 AC1 で `create_rectangle_clip` ラッパーが必要
- **Sources Consulted**: Windows SDK ドキュメント、windows-rs クレートの型定義
- **Findings**:
  - `IDCompositionDevice3::CreateRectangleClip()` → `Result<IDCompositionRectangleClip>`
  - `IDCompositionRectangleClip` は `IDCompositionClip` を実装
  - 設定メソッド: `SetLeft`, `SetTop`, `SetRight`, `SetBottom` (矩形座標)
  - 角丸: `SetTopLeftRadiusX`, `SetTopLeftRadiusY`, `SetTopRightRadiusX`, `SetTopRightRadiusY`, `SetBottomLeftRadiusX`, `SetBottomLeftRadiusY`, `SetBottomRightRadiusX`, `SetBottomRightRadiusY`
  - 全メソッドが `f32` 値を直接受け取るバリアント (`Set*2`) を持つ
- **Implications**: `DCompositionDeviceExt` に `create_rectangle_clip` を追加。ClipShape の全3バリアントを `IDCompositionRectangleClip` 1つで表現可能

### ULW モード — ID2D1Factory からの Geometry 作成
- **Context**: Req 9 AC2/AC3 で `ID2D1RoundedRectangleGeometry` および `ID2D1PathGeometry` を作成する必要がある。`ID2D1Factory` へのアクセス方法を確認。
- **Sources Consulted**: `render.rs` の `CompositeContext`、`command_types.rs` の `PushLayer` 構造体、D2D API ドキュメント
- **Findings**:
  - `CompositeContext` は `dc: &ID2D1DeviceContext` を保持
  - `ID2D1DeviceContext::GetFactory()` → `ID2D1Factory` を取得可能（`ID2D1Resource` トレイト経由）
  - `ID2D1Factory::CreateRoundedRectangleGeometry(&D2D1_ROUNDED_RECT)` → `Result<ID2D1RoundedRectangleGeometry>`
  - `D2D1FactoryExt::create_path_geometry()` は既に `com/d2d/mod.rs:22` に存在
  - `ID2D1RoundedRectangleGeometry` は `ID2D1Geometry` を実装 → `PushLayer` の `geometric_mask` に直接渡せる
- **Implications**: `render_subtree` 内で `dc.GetFactory()` → `factory.CreateRoundedRectangleGeometry()` の流れで Geometry を取得。`D2D1FactoryExt` に `create_rounded_rectangle_geometry` ラッパーを追加するのが望ましい

### ULW モード — PushAxisAlignedClip / PushLayer のパフォーマンス
- **Context**: Req 9 で毎フレーム Geometry 作成のコストを評価
- **Sources Consulted**: D2D パフォーマンスガイドライン、DrawCommand パターン
- **Findings**:
  - `PushAxisAlignedClip` はハードウェアアクセラレーションされ、極めて軽量
  - `PushLayer` + Geometry は Layer 作成コストがあるが、小規模ツリーでは無視できる
  - `ID2D1RoundedRectangleGeometry` は不変オブジェクト（サイズ/radius 変更時に再作成必要）
  - Phase 1 では毎フレーム再作成で十分。将来、キャッシュが必要ならば `VisualGraphics` にキャッシュ可能
- **Implications**: Phase 1 では `render_subtree` 内で毎回 Geometry を作成して即時破棄。パフォーマンス問題は wintf のユースケース（少数の Visual エンティティ）では発生しない見込み

### Changed<Arrangement> と Changed<Visual> の二重実行
- **Context**: gap-analysis の Research Needed #2。同一フレームで両方が変更された場合の挙動
- **Sources Consulted**: bevy_ecs の `Changed<T>` セマンティクス、`visual_property_sync_system` の実装
- **Findings**:
  - bevy_ecs の `Or<(Changed<A>, Changed<B>)>` は「AまたはBが変更された」エンティティを返す
  - 同一フレームで両方変更されても、システムは1回だけ実行される
  - `visual_property_sync_system` は毎実行時に最新の `Arrangement` + `Visual` を読み取るため、二重実行の問題は発生しない
  - `clip_sync_system` も同様のパターンで実装すれば問題なし
- **Implications**: 二重実行回避は不要。`Or<(Changed<Arrangement>, Changed<GlobalArrangement>, Changed<Visual>)>` で十分

### render_subtree へのクリップ挿入位置
- **Context**: ULW モードのクリップ適用タイミング
- **Sources Consulted**: `render.rs:110-200` の render_subtree 実装
- **Findings**:
  - 現在のフロー: visibility check → opacity計算 → SetTransform → draw_with_opacity → 子再帰
  - クリップは SetTransform **後** に挿入する（ローカル座標系でクリップを定義するため）
  - Push は draw_with_opacity の **前**、Pop は子再帰の **後** に配置
  - `PushAxisAlignedClip` の clipRect は current transform 空間で解釈される → (0,0)-(w,h) を指定すれば transform によって自動変換される
  - Push/Pop はペアで必ず実行する必要がある（Pop 漏れは D2D state corruption）
- **Implications**: 変更後のフロー: visibility → opacity → SetTransform → **Push clip** → draw → children → **Pop clip**。ペア実行保証には RAII ガード（`ClipGuard` struct）を使用し、エラー時・early return 時も確実に Pop を実行（既存の `DcTargetGuard` パターンに準拠）

## Architecture Pattern Evaluation

| Option                                | Description                                          | Strengths                          | Risks / Limitations                                                    | Notes                              |
| ------------------------------------- | ---------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------- | ---------------------------------- |
| A: visual_property_sync_system 内統合 | クリップ同期を既存システムに追加                     | ファイル変更最小、新規ファイルなし | visual_property_sync_system 肥大化、DCompGraphicsResource への依存追加 | 小規模変更には適切だが拡張性に難   |
| B: clip.rs + clip_sync_system（推奨） | 型定義とDComp同期を新規モジュール/システムとして分離 | 責務分離明確、テスト容易、拡張性高 | 新規ファイル2つ、システム数増加                                        | SurfaceMask 追加時に自然に拡張可能 |
| C: ハイブリッド                       | 型定義は分離、同期は既存システムに統合               | 型は整理される                     | visual_property_sync_system の引数変更が必要                           | 中途半端な分離                     |

## Design Decisions

### Decision: Option B — 新規モジュール + 新規システム
- **Context**: クリップ機能の型定義とDComp同期ロジックの配置場所
- **Alternatives Considered**:
  1. Option A — visual_property_sync_system 内に全統合
  2. Option B — clip.rs + clip_sync_system で完全分離
  3. Option C — 型のみ分離、同期は統合
- **Selected Approach**: Option B。`clip.rs` に `ClipShape` enum を定義し、`clip_sync.rs` に DComp 専用の `clip_sync_system` を配置。ULW は `render_subtree` 内で処理。
- **Rationale**:
  - `clip_sync_system` は `DCompGraphicsResource` を必要とし、既存 `visual_property_sync_system` にこの依存を追加すると責務が曖昧になる
  - `ClipShape` は将来 SurfaceMask バリアントが追加される前提。独立モジュールが自然
  - wintf の既存パターン（モジュール分離 + `pub use`）との一貫性
  - `visual_sync.rs` (277行) にさらにロジックを追加するのは保守性の観点で非推奨
- **Trade-offs**: 新規ファイル2つ増加するが、変更箇所が明確に分離される
- **Follow-up**: Phase 1 完了後、パフォーマンスプロファイリングで Geometry 再作成コストを検証

### Decision: Phase 1 では毎回 IDCompositionRectangleClip / Geometry を再作成
- **Context**: Changed<Visual> 時に RectangleClip を毎回作成するか、キャッシュするか
- **Alternatives Considered**:
  1. 毎回作成 — シンプル、Changed イベント時のみ実行
  2. VisualGraphics にキャッシュ — 再利用可能だが設計複雑化
- **Selected Approach**: 毎回作成
- **Rationale**: Changed<Visual> / Changed<Arrangement> 時のみ実行されるため負荷は低い。ULW の Geometry は毎フレーム作成だが、wintf のユースケース（数十〜数百 Visual）では問題にならない
- **Trade-offs**: 多数の Visual かつ高頻度変更環境ではキャッシュが有利だが、Phase 1 のスコープ外
- **Follow-up**: パフォーマンス問題発生時に VisualGraphics キャッシュを検討

### Decision: ULW クリップの Push/Pop ペア保証方式
- **Context**: render_subtree 内で Push 後に Pop を確実に実行する方法
- **Alternatives Considered**:
  1. bool フラグ + 条件付き Pop — シンプルだが Pop 漏れリスク
  2. RAII ガード — `Drop` で Pop を自動呼び出し
  3. クロージャ方式 — Push → closure(draw + children) → Pop
- **Selected Approach**: bool フラグ + 条件付き Pop
- **Rationale**: render_subtree は既にエラーを `error!` でログして続行するパターン。RAII ガードは DC の参照ライフタイムが複雑化する。bool フラグはクロージャより可読性が高く、既存コードスタイルと一致。Push 失敗時は `clipped = false` として描画は継続（クリップなしで描画）
- **Trade-offs**: Pop 漏れは手動管理のリスクだが、Push と Pop が同一関数内の近い行にあるため実質的リスクは低い
- **Follow-up**: なし

## Risks & Mitigations
- `render_subtree` の構造変更がすべての ULW 描画に影響 — 既存の描画テスト + clip_demo による回帰確認で緩和
- DComp `SetClip(None)` の挙動未検証 — clip_demo で実際に None 設定→クリップ解除を視覚確認
- `PushLayer` + Geometry のアンチエイリアス品質 — D2D1_ANTIALIAS_MODE_PER_PRIMITIVE を使用、clip_demo で目視確認
- 大量 Visual エンティティでの Geometry 毎フレーム作成コスト — Phase 1 では許容、プロファイル結果に基づきキャッシュ導入

## References
- [IDCompositionRectangleClip](https://learn.microsoft.com/en-us/windows/win32/api/dcomp/nn-dcomp-idcompositionrectangleclip) — DComp 矩形クリップ API
- [IDCompositionVisual3::SetClip](https://learn.microsoft.com/en-us/windows/win32/api/dcomp/nf-dcomp-idcompositionvisual-setclip) — Visual へのクリップ設定
- [ID2D1DeviceContext::PushAxisAlignedClip](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-pushaxisalignedclip) — D2D 矩形クリップ
- [ID2D1DeviceContext::PushLayer](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1rendertarget-pushlayer) — D2D レイヤーベースクリップ
- [ID2D1Factory::CreateRoundedRectangleGeometry](https://learn.microsoft.com/en-us/windows/win32/api/d2d1/nf-d2d1-id2d1factory-createroundedrectanglegeometry) — 角丸矩形ジオメトリ
