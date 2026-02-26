# Gap Analysis: visual-clip

## 0. 分析スコープ

`visual-clip` 仕様の要件（Req 1〜8）と既存コードベースとの差分を分析し、実装アプローチの選択肢を提示する。

---

## 1. 要件ごとのアセットマッピング

### Req 1: ClipShape 型定義

| 項目                             | 状態               | 既存アセット                                                         |
| -------------------------------- | ------------------ | -------------------------------------------------------------------- |
| `ClipShape` enum                 | **Missing**        | 該当なし。新規型を定義する必要あり                                   |
| `Debug, Clone, PartialEq` derive | **Pattern Exists** | `Visual` が同パターンを使用（`visual.rs:26`）                        |
| 負値クランプ                     | **Pattern Exists** | `Visual::set_opacity()` に clamp パターンあり（`visual.rs:138-148`） |

### Req 2: Visual コンポーネントへの clip フィールド追加

| 項目                             | 状態        | 既存アセット                                                                             |
| -------------------------------- | ----------- | ---------------------------------------------------------------------------------------- |
| `Visual.clip: Option<ClipShape>` | **Missing** | `Visual` struct に `is_visible`, `opacity`, `transform_origin` のみ（`visual.rs:27-31`） |
| `Default` 実装                   | **Extend**  | 既存 `Default` impl（`visual.rs:122-130`）に `clip: None` を追加                         |
| `Changed<Visual>` 検知           | **Exists**  | bevy_ecs の `Changed<T>` は既に `visual_property_sync_system` で使用中                   |

### Req 3: クリップ矩形の座標計算

| 項目                        | 状態               | 既存アセット                                                                             |
| --------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| Arrangement.size からの算出 | **Pattern Exists** | `Arrangement` が `size: Size` フィールドを持つ（`arrangement.rs:11-14`）                 |
| `local_bounds()` メソッド   | **Exists**         | `Arrangement::local_bounds()` が `(0,0)-(width,height)` を返す（`arrangement.rs:18-24`） |
| ゼロサイズ回避              | **Missing**        | 新規ガード条件が必要                                                                     |

### Req 4: DComp モード — DirectComposition との同期（clip_sync_system）

| 項目                              | 状態               | 既存アセット                                                                                 |
| --------------------------------- | ------------------ | -------------------------------------------------------------------------------------------- |
| `clip_sync_system`                | **Missing**        | 新規システム。パターンは `visual_property_sync_system`（`visual_sync.rs:185-269`）に準拠可能 |
| `IDCompositionRectangleClip` 作成 | **Missing**        | COM API に `create_rectangle_clip` ラッパーなし                                              |
| `SetClip` 呼び出し                | **Missing**        | `DCompositionVisualExt` に `set_clip` メソッドなし                                           |
| null でクリップ解除               | **Missing**        | 新規。`set_content` の null パターンを参考にできる                                           |
| エラーハンドリング                | **Pattern Exists** | `visual_property_sync_system` の `error!` ログパターン                                       |

### Req 5: DPI スケーリング対応

| 項目                                     | 状態               | 既存アセット                                                                                               |
| ---------------------------------------- | ------------------ | ---------------------------------------------------------------------------------------------------------- |
| DComp: `GlobalArrangement` スケール適用  | **Pattern Exists** | `visual_property_sync_system` で `scale_x/scale_y` を offset に適用（`visual_sync.rs:225-232`）            |
| `Changed<GlobalArrangement>` 検知        | **Exists**         | `visual_property_sync_system` の `Or<(Changed<Arrangement>, Changed<GlobalArrangement>, Changed<Visual>)>` |
| DComp: radius のスケーリング             | **Missing**        | 新規。offset スケーリングと同様のパターンで実装                                                            |
| ULW: SetTransform による自動スケーリング | **Exists**         | `render_subtree` の `adjusted_transform` が DPI スケールを含む（`render.rs:163-171`）                      |

### Req 6: COM API 拡張

| 項目                                           | 状態        | 既存アセット                                                  |
| ---------------------------------------------- | ----------- | ------------------------------------------------------------- |
| `DCompositionDeviceExt::create_rectangle_clip` | **Missing** | `dcomp.rs` の `DCompositionDeviceExt` トレイトに追加が必要    |
| `DCompositionVisualExt::set_clip`              | **Missing** | `dcomp.rs` の `DCompositionVisualExt` トレイトに追加が必要    |
| unsafe ラッパーパターン                        | **Exists**  | 既存の `set_opacity`, `set_offset_x` 等（`dcomp.rs:160-180`） |

### Req 7: システムスケジュール統合

| 項目                               | 状態               | 既存アセット                                                             |
| ---------------------------------- | ------------------ | ------------------------------------------------------------------------ |
| Composition スケジュール           | **Exists**         | `schedule_labels.rs:93` に定義済み                                       |
| `visual_property_sync_system` の後 | **Extend**         | `world/mod.rs:272-280` の `add_systems(Composition, ...)` チェーンに追加 |
| モード別分岐                       | **Pattern Exists** | `visual.rs:87` の `is_dcomp_mode` チェックパターン                       |

### Req 8: 将来の拡張性

| 項目                       | 状態                | 既存アセット                                                                   |
| -------------------------- | ------------------- | ------------------------------------------------------------------------------ |
| `#[non_exhaustive]` 非付与 | **Design Decision** | 既存 enum パターンとの一貫性を確認（クレート内部型に non_exhaustive は不使用） |
| パターンマッチ網羅性       | **Exists**          | Rust コンパイラが自動保証                                                      |

### Req 9: ULW モード — D2D 描画パイプラインでのクリップ適用

| 項目                                 | 状態        | 既存アセット                                                                                              |
| ------------------------------------ | ----------- | --------------------------------------------------------------------------------------------------------- |
| `render_subtree` へのクリップ追加    | **Missing** | 新規。`render_subtree`（`render.rs:110-210`）に clip 分岐を追加                                           |
| `PushAxisAlignedClip` (Rectangle)    | **Exists**  | D2D command type が `com/d2d/command_types.rs:487` に実装済み                                             |
| `PushLayer` (角丸共通)               | **Exists**  | D2D command type が `com/d2d/command_types.rs:503` に実装済み                                             |
| `ID2D1RoundedRectangleGeometry` 作成 | **Missing** | `ID2D1Factory::CreateRoundedRectangleGeometry` ラッパーが必要（RoundedRectangle 用）                      |
| `ID2D1PathGeometry` (各角個別)       | **Exists**  | `D2D1FactoryExt::create_path_geometry` が `com/d2d/mod.rs:22` に実装済み（RoundedRectangleIndividual 用） |
| サブツリークリッピング               | **Missing** | Push → 自Entity描画 → 子再帰 → Pop の構造に `render_subtree` を変更                                       |
| `PopAxisAlignedClip` / `PopLayer`    | **Exists**  | `com/d2d/command_types.rs` に `PopAxisAlignedClip` / `PopLayer` が実装済み                                |

### Req 10: クリッピング検証デモ

| 項目                            | 状態               | 既存アセット                                                                               |
| ------------------------------- | ------------------ | ------------------------------------------------------------------------------------------ |
| デュアルモードデモプログラム    | **Missing**        | 新規 example ファイル。`multi_backend_demo.rs` をテンプレートに使用可能                     |
| ULW/DComp 2ウィンドウ同時表示   | **Pattern Exists** | `multi_backend_demo.rs` が同パターンを実装済み（UlwDemoWindow / DCompDemoWindow マーカー） |
| 同一レイアウト構造              | **Pattern Exists** | `ulw_twin_demo.rs:create_simple_window` が同レイアウト複製パターンを実装済み               |
| ウィンドウサイズ追従レイアウト  | **Pattern Exists** | flex grow を使った可変サイズレイアウトが既存デモに多数存在                                  |
| クリップ効果の視覚化            | **Missing**        | 新規。はみ出す子要素 + 親に clip 設定のレイアウトを構築                                     |
| 3バリアントの全表示             | **Missing**        | 新規。Rectangle / RoundedRectangle / RoundedRectangleIndividual を異なる要素に適用         |

---

## 2. 実装アプローチの選択肢

### Option A: 既存 visual_property_sync_system を拡張

**概要**: クリップ同期を `visual_property_sync_system` 内に追加する。

**変更対象ファイル**:
- `com/dcomp.rs` — COM API ラッパー追加
- `ecs/graphics/visual.rs` — `Visual` にフィールド追加、`ClipShape` 定義を同ファイルに配置
- `ecs/graphics/systems/visual_sync.rs` — 既存関数内にクリップ同期ロジック追加

**トレードオフ**:
- ✅ ファイル変更が最小限（新規ファイルなし）
- ✅ 既存のクエリ・Changed 検知をそのまま活用
- ✅ DCompGraphicsResource の追加パラメーターが不要（IDCompositionDevice3 経由で RectangleClip 作成可能）
- ❌ `visual_property_sync_system` が肥大化する
- ❌ `ClipShape` を `visual.rs` に混在させると責務が曖昧に
- ❌ `IDCompositionDevice3` への参照が `visual_property_sync_system` に必要（現在は不要）

### Option B: 新規モジュール clip.rs + clip_sync_system（推奨）

**概要**: クリップ専用のモジュールとシステムを新規作成する。DComp 同期は専用システム、ULW は render_subtree 内で処理。

**変更対象ファイル**:
- `com/dcomp.rs` — COM API ラッパー追加（`create_rectangle_clip`, `set_clip`）
- `com/d2d/mod.rs` — D2D1FactoryExt に `create_rounded_rectangle_geometry` ラッパー追加（ULW 角丸用）
- `ecs/graphics/clip.rs` — **新規**: `ClipShape` enum 定義
- `ecs/graphics/visual.rs` — `Visual` に `clip: Option<ClipShape>` フィールド追加
- `ecs/graphics/mod.rs` — `clip` モジュール追加、`pub use`
- `ecs/graphics/systems/clip_sync.rs` — **新規**: `clip_sync_system` 定義（DComp モード用）
- `ecs/graphics/systems/mod.rs` — `clip_sync` モジュール追加
- `ecs/graphics/compositor_systems/render.rs` — `render_subtree` にクリップ描画追加（ULW モード用）
- `ecs/world/mod.rs` — Composition スケジュールに clip_sync_system 登録
- `examples/clip_demo.rs` — **新規**: クリッピング検証デモ（ULW/DComp 2ウィンドウ、可変サイズレイアウト）

**トレードオフ**:
- ✅ 責務が明確に分離（clip = 独立モジュール）
- ✅ 既存コードへの影響が最小限
- ✅ テストしやすい
- ✅ `visual_property_sync_system` が肥大化しない
- ✅ SurfaceMask 追加時に拡張しやすい
- ❌ 新規ファイルが2つ増える
- ❌ Composition スケジュールのシステム数が増える

### Option C: visual_property_sync_system 内に統合 + ClipShape を clip.rs に分離

**概要**: 型定義は分離するが、同期ロジックは既存システムに統合するハイブリッド。

**変更対象ファイル**:
- `com/dcomp.rs` — COM API ラッパー追加
- `ecs/graphics/clip.rs` — **新規**: `ClipShape` enum 定義
- `ecs/graphics/visual.rs` — `Visual` にフィールド追加
- `ecs/graphics/mod.rs` — `clip` モジュール追加
- `ecs/graphics/systems/visual_sync.rs` — クリップ同期ロジック追加
- `ecs/world/mod.rs` — 変更なし（既存 Composition 登録を流用）

**トレードオフ**:
- ✅ 型定義は分離されて整理されている
- ✅ システム数が増えない
- ❌ `visual_property_sync_system` のクエリに `DCompGraphicsResource` が追加で必要
- ❌ 既存関数の引数変更が必要

---

## 3. クリップ同期システムの設計上の注意点

### IDCompositionDevice3 へのアクセス

**課題**: `IDCompositionRectangleClip` の作成には `IDCompositionDevice3::CreateRectangleClip()` が必要。現在の `visual_property_sync_system` は `IDCompositionDevice3` を直接参照していない。

**対応策**:
- Option B の場合: `clip_sync_system` に `Res<DCompGraphicsResource>` を注入
- Option A/C の場合: `visual_property_sync_system` に `Res<DCompGraphicsResource>` を追加

### RectangleClip の再作成/キャッシュ戦略

**課題**: `Visual.clip` が変更されるたびに `IDCompositionRectangleClip` を新規作成するか、キャッシュするか。

**選択肢**:
1. **毎回作成（推奨）**: シンプル。Changed<Visual> 時のみ実行されるので頻度は低い
2. **VisualGraphics にキャッシュ**: `VisualGraphics` に `clip: Option<IDCompositionRectangleClip>` を追加。SurfaceMask 拡張時に有利だが設計が複雑化

**推奨**: Phase 1 では毎回作成。パフォーマンス問題が出たらキャッシュを検討。

### Arrangement サイズ変更時の再同期

**課題**: `Arrangement` のサイズが変更されると、クリップ矩形も再計算が必要。`Changed<Visual>` だけでは不十分。

**対応**: `Changed<Arrangement>` または `Changed<GlobalArrangement>` もトリガーとする。Option B では独自のクエリフィルタを定義可能。

### ULW モード: render_subtree の構造変更

**課題**: ULW モードでは `render_subtree`（`render.rs:110-210`）にクリップ描画を追加する必要がある。DComp モードの `Visual::SetClip` はサブツリー全体をクリップするため、ULW でも同等の挙動が求められる。

**現在の render_subtree 構造**:
1. Get entity data → visibility check → opacity calculation
2. `SetTransform(adjusted_transform)` 
3. Draw entity content with opacity
4. Recurse to children

**クリップ追加後の構造**:
1. Get entity data → visibility check → opacity calculation
2. `SetTransform(adjusted_transform)`
3. **If clip: Push clip (PushAxisAlignedClip or PushLayer)**
4. Draw entity content with opacity
5. Recurse to children
6. **If clip: Pop clip (PopAxisAlignedClip or PopLayer)**

**注意点**:
- Push/Pop はペアで呼び出す必要があり、エラー時にも Pop を確実に実行する（RAII パターン or finally 相当）
- `PushAxisAlignedClip` のクリップ矩形は current transform 空間。`SetTransform` 後に呼ぶため、ローカル座標 (0,0)-(w,h) を指定すれば transform により物理座標に自動変換される
- `PushLayer` + `ID2D1RoundedRectangleGeometry` の場合、Geometry 作成に `ID2D1Factory` が必要。`dc.GetFactory()` で取得可能
- 角丸の場合の Geometry 作成コストについて: 毎フレーム作成 vs キャッシュ（Phase 1 では毎フレーム作成で十分な可能性が高い）

---

## 4. 実装複雑度とリスク

### 工数見積

**M（3〜5日）**

理由:
- DComp 側: 既存パターン（`visual_property_sync_system`）の踏襲で済む
- ULW 側: `render_subtree` の構造変更（Push → Draw → 子再帰 → Pop）が必要
- ULW 角丸: `ID2D1RoundedRectangleGeometry` 作成パスの新規実装
- COM API ラッパーは定型的
- テストは既存のインテグレーションテストパターンを流用可能

### リスク評価

**Low〜Medium**

理由:
- DirectComposition の `IDCompositionRectangleClip` は成熟した API（Low）
- D2D の `PushAxisAlignedClip` は既にコマンド型が存在（Low）
- ULW `PushLayer` + `ID2D1RoundedRectangleGeometry` はレンダリングパスの構造変更を伴う（Medium）
- `render_subtree` の変更はすべての ULW 描画に影響するため、回帰テストが重要
- 影響範囲: `Visual` フィールド追加 + 新規システム（DComp） + 既存レンダリング関数修正（ULW）

---

## 5. 推奨事項

### 推奨アプローチ: **Option B（新規モジュール + 新規システム）**

**具体的な理由**:
1. `clip_sync_system` は `DCompGraphicsResource` を必要とするため、既存 `visual_property_sync_system` に統合すると引数変更が発生する
2. `ClipShape` は将来 SurfaceMask が追加される前提のため、独立モジュールとして管理する方が自然
3. wintf の既存パターン（モジュール分離 + `pub use`）と一貫性がある
4. `visual_sync.rs` の277行にさらに同期ロジックを追加するのは保守性の観点で非推奨
5. ULW モードの `render_subtree` 変更は Option B でも避けられないが、DComp 同期が独立システムであれば変更箇所が明確に分離される

### 設計フェーズへの持ち越し事項

- **Research Needed #1**: `IDCompositionVisual3::SetClip` に null ポインタを渡してクリップ解除する具体的な API 呼び出し方法（`windows-rs` での `None` パラメーターの扱い）
- **Research Needed #2**: `Changed<Arrangement>` と `Changed<Visual>` が同一フレームで両方発生した場合の二重実行回避が必要か
- **Research Needed #3**: ULW モードで `ID2D1RoundedRectangleGeometry` を作成するための `ID2D1Factory` へのアクセス方法（`ID2D1DeviceContext::GetFactory()` またはリソースとして保持）
- **Research Needed #4**: ULW モードの `PushLayer` + Geometry によるクリッピングのパフォーマンス特性（毎フレーム Geometry 再作成のコスト）

