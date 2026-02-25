# Requirements Document

## Project Description (Input)
wintf論理VisualコンポーネントにDComp / D2D デュアルモード対応のクリップ機能を統合する。Arrangementのサイズに基づく矩形クリッピングを提供し、角丸矩形（全角統一・各角個別設定）に対応する。DComp モードでは IDCompositionRectangleClip、ULW モードでは D2D PushAxisAlignedClip / PushLayer を用いてクリップを実現する。フラット構造のClipShape enum（案A）を採用：Rectangle（角張った矩形）、RoundedRectangle（全角統一の角丸）、RoundedRectangleIndividual（各角個別設定）の3バリアント。Phase 1では全3バリアントを両モードで実装し、SurfaceMaskは将来の拡張として enum に余地を確保。

### 設計方針
- **サイズベース**: クリップ矩形は (0,0) から (Arrangement.size.width, Arrangement.size.height) まで
- **型安全性**: 排他的な選択肢を enum バリアントで表現（Option による冗長性回避）
- **デュアルモード**: DComp と ULW の両描画パイプラインで一貫したクリップ挙動を提供
- **DirectComposition / D2D APIとの対応**: DComp は IDCompositionRectangleClip、ULW は D2D PushAxisAlignedClip / PushLayer と対応
- **段階的拡張**: 初期実装後、SurfaceMask や楕円形の角などを追加可能

### Phase 1 実装スコープ
- ClipShape enum 定義（Rectangle, RoundedRectangle, RoundedRectangleIndividual）
- Visual への clip: Option<ClipShape> フィールド追加
- DComp モード: clip_sync_system 実装（Changed<Visual> による IDCompositionRectangleClip 同期）
- DComp モード: COM API 拡張（DCompositionDeviceExt::create_rectangle_clip, DCompositionVisualExt::set_clip）
- ULW モード: render_subtree でのクリップ描画（PushAxisAlignedClip / PushLayer + RoundedRectangleGeometry）
- システムスケジュール登録（Composition フェーズ、visual_property_sync_system の後）

### 実装しない（後回し）
- ClipShape::SurfaceMask（任意形状・グラデーションマスク）
- アニメーション対応（dola統合）
- 楕円形の角（rx, ry が異なる角丸）

## Introduction

Visualコンポーネントにクリッピング機能を追加し、描画領域の矩形制約と角丸表現を可能にする。DComp モードでは DirectComposition の IDCompositionRectangleClip、ULW モードでは D2D の PushAxisAlignedClip / PushLayer を用いてクリッピングを実現する。論理 Visual の `clip` プロパティとして抽象化し、既存の `opacity` / `transform_origin` と同等のパターンで統合する。開発者は CompositionMode を意識せず、`Visual.clip` を設定するだけでクリップが適用される。

## Requirements

### Requirement 1: ClipShape 型定義

**Objective:** 開発者として、クリップ形状を型安全に指定したい。排他的な選択肢が enum バリアントとして表現されることで、不正な状態を型レベルで防止できるようにする。

#### Acceptance Criteria

1. wintf は `ClipShape` enum を提供 shall する。以下の3バリアントを含む:
   - `Rectangle` — 角張った矩形クリップ（追加パラメーターなし）
   - `RoundedRectangle { radius: f32 }` — 全角統一の角丸矩形クリップ
   - `RoundedRectangleIndividual { top_left: f32, top_right: f32, bottom_left: f32, bottom_right: f32 }` — 各角個別設定の角丸矩形クリップ
2. `ClipShape` は `Debug`, `Clone`, `PartialEq` を derive shall する。
3. When `RoundedRectangle` の `radius` に負の値が指定された場合, wintf shall 0.0 にクランプする。
4. When `RoundedRectangleIndividual` の各フィールドに負の値が指定された場合, wintf shall 該当フィールドを 0.0 にクランプする。

### Requirement 2: Visual コンポーネントへの clip フィールド追加

**Objective:** 開発者として、既存の Visual コンポーネントにクリップを設定したい。`opacity` や `is_visible` と同様の使い勝手で、オプショナルなプロパティとして利用できるようにする。

#### Acceptance Criteria

1. `Visual` コンポーネントは `clip: Option<ClipShape>` フィールドを持つ shall する。
2. `Visual::default()` は `clip: None`（クリップなし） shall とする。
3. When `Visual.clip` が `None` から `Some(ClipShape::*)` に変更された場合, bevy_ecs の `Changed<Visual>` shall 検知する。
4. When `Visual.clip` が `Some(ClipShape::*)` から `None` に変更された場合, bevy_ecs の `Changed<Visual>` shall 検知する。

### Requirement 3: クリップ矩形の座標計算

**Objective:** 開発者として、クリップ領域を手動で座標指定する必要がないようにしたい。Arrangement のサイズに基づいてクリップ矩形が自動算出されるようにする。

#### Acceptance Criteria

1. wintf shall クリップ矩形を Arrangement のサイズから自動算出する。left=0, top=0, right=size.width, bottom=size.height とする。
2. When `Arrangement` のサイズが変更された場合, wintf shall クリップ矩形を再計算して描画パイプライン（DComp / ULW）に反映する。
3. While Arrangement のサイズが (0, 0) の場合, wintf shall クリップを適用しない（ゼロサイズの矩形クリップは回避する）。

### Requirement 4: DComp モード — DirectComposition との同期（clip_sync_system）

**Objective:** DComp モードにおいて、Visual の clip プロパティを設定するだけで、DirectComposition のクリップが自動的に同期されるようにしたい。ULW モードのクリップ描画は Req 9 で扱う。

#### Acceptance Criteria

1. When `Visual.clip` が変更された場合（`Changed<Visual>`）, wintf shall `IDCompositionRectangleClip` を作成し、`IDCompositionVisual3::SetClip` で適用する。
2. When `Visual.clip` が `Some(ClipShape::Rectangle)` に設定された場合, wintf shall 角の半径を設定せず（すべて 0.0）矩形クリップを適用する。
3. When `Visual.clip` が `Some(ClipShape::RoundedRectangle { radius })` に設定された場合, wintf shall 全8つの角丸パラメーター（TopLeft/TopRight/BottomLeft/BottomRight の RadiusX/RadiusY）に同一の `radius` 値を設定する。
4. When `Visual.clip` が `Some(ClipShape::RoundedRectangleIndividual { .. })` に設定された場合, wintf shall 各角の RadiusX と RadiusY を対応するフィールド値に設定する。
5. When `Visual.clip` が `None` に設定された場合, wintf shall `IDCompositionVisual3::SetClip` に null を渡してクリップを解除する。
6. If `IDCompositionRectangleClip` の作成または `SetClip` が失敗した場合, wintf shall `error!` ログを出力して処理を継続する（クラッシュしない）。

### Requirement 5: DPI スケーリング対応

**Objective:** 開発者として、高DPI環境でもクリップが正しく適用されるようにしたい。

#### Acceptance Criteria

1. While CompositionMode が DComp の場合, wintf shall クリップ矩形のサイズに `GlobalArrangement` の累積スケール値を適用して物理ピクセル座標に変換する。
2. While CompositionMode が DComp の場合, wintf shall 角丸の半径に `GlobalArrangement` の累積スケール値を適用して物理ピクセル値に変換する。
3. When DPI が変更された場合（`Changed<GlobalArrangement>`）, wintf shall クリップ設定を再同期する。
4. While CompositionMode が ULW の場合, wintf shall クリップ矩形と角丸半径に別途 DPI スケーリングを適用しない（D2D の SetTransform が DPI スケールを含むため、論理ピクセル座標をそのまま使用する）。

### Requirement 6: COM API 拡張

**Objective:** wintf の COM ラッパー層に、クリッピングに必要な DirectComposition API のラッパーを追加する。

#### Acceptance Criteria

1. `DCompositionDeviceExt` トレイトは `create_rectangle_clip(&self) -> Result<IDCompositionRectangleClip>` メソッドを提供 shall する。
2. `DCompositionVisualExt` トレイトは `set_clip` メソッドを提供 shall する。`IDCompositionClip` の設定とクリップ解除（null 渡し）の両方をサポートする。
3. While COM API 呼び出しが `unsafe` ブロックを必要とする場合, wintf shall 安全なラッパー関数として上位層に提供する。

### Requirement 7: システムスケジュール統合

**Objective:** クリップ処理が各描画パイプラインの適切なタイミングで実行されるようにする。

#### Acceptance Criteria

1. wintf shall DComp モードの clip_sync_system を Composition スケジュールフェーズで実行する。
2. wintf shall clip_sync_system を `visual_property_sync_system` の後に実行する（Offset/Opacity の同期後にクリップを適用する）。
3. While CompositionMode が DComp の場合, wintf shall clip_sync_system（Req 4）でクリップ同期を行い、ULW の場合は composite_render_system 内の render_subtree（Req 9）でクリップ描画を行う。

### Requirement 8: 将来の拡張性

**Objective:** Phase 1 以降の機能追加（SurfaceMask、アニメーション等）が後方互換性を保って行えるようにする。

#### Acceptance Criteria

1. `ClipShape` enum は `#[non_exhaustive]` を付与 shall しない（クレート内部型のため、パターンマッチの網羅性チェックを維持する）。
2. `ClipShape` enum のバリアント追加は、既存コードの修正を要求する（match の網羅性で検出できる） shall とする。
3. `Visual.clip` フィールドは `Option<ClipShape>` 型のまま維持 shall する。新しいクリップ形状は `ClipShape` のバリアント追加で対応する。

### Requirement 9: ULW モード — D2D 描画パイプラインでのクリップ適用

**Objective:** ULW モード（デフォルト）でもクリップが正しく動作するようにしたい。DComp モードと同等のクリッピング挙動を D2D レンダリングパイプラインで実現する。

#### Acceptance Criteria

1. When `Visual.clip` が `Some(ClipShape::Rectangle)` かつ CompositionMode が ULW の場合, wintf shall `render_subtree` 内で `PushAxisAlignedClip` / `PopAxisAlignedClip` を用いて矩形クリッピングを適用する。
2. When `Visual.clip` が `Some(ClipShape::RoundedRectangle { .. })` かつ CompositionMode が ULW の場合, wintf shall `PushLayer` + `ID2D1RoundedRectangleGeometry` を用いて角丸クリッピングを適用する。
3. When `Visual.clip` が `Some(ClipShape::RoundedRectangleIndividual { .. })` かつ CompositionMode が ULW の場合, wintf shall `PushLayer` + `ID2D1PathGeometry`（各角個別の円弧を持つカスタム角丸矩形パス）を用いて角丸クリッピングを適用する。
4. wintf shall ULW モードのクリップを当該エンティティおよびその子エンティティの描画全体に適用する（DComp モードの `IDCompositionVisual::SetClip` と同等のサブツリークリッピング）。
5. クリップ矩形は Arrangement のローカル座標 (0, 0)-(width, height) とする（Req 3 と同一基準）。
6. When `Visual.clip` が `None` の場合, wintf shall ULW モードでクリッピングを適用しない。
7. If D2D クリップ操作（PushAxisAlignedClip / PushLayer）が失敗した場合, wintf shall `error!` ログを出力して描画処理を継続する。
8. ULW モードで使用する D2D API（PushAxisAlignedClip、PushLayer、PathGeometry）は既存の `com/d2d` モジュールに基盤が実装済みであり、`ID2D1RoundedRectangleGeometry` 作成ラッパーのみ追加が必要。

## Out of Scope

以下は本仕様のスコープ外とし、将来の仕様で扱う:

- **ClipShape::SurfaceMask**: サーフェスベースの任意形状マスク
- **アニメーション対応**: dola 統合によるクリップパラメーターのアニメーション
- **楕円形の角**: RadiusX と RadiusY を異なる値にする楕円形の角丸
- **クリップのネスト/合成**: 複数クリップの論理演算（交差・合成）
