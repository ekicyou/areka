# Gap Analysis: wintf-P0-balloon01-core

| 項目             | 内容                                          |
| ---------------- | --------------------------------------------- |
| **対象仕様**     | wintf-P0-balloon01-core（バルーンコア子仕様） |
| **分析日**       | 2026-02-26                                    |
| **Requirements** | v2.0（7要件 / 27受入基準）                    |
| **分析範囲**     | crates/wintf/src/, crates/areka/src/main.rs   |

---

## 1. 現状調査サマリ

### 1.1 既存アセット

| アセット                             | パス                             | 関連性                                                                                                   |
| ------------------------------------ | -------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `Visual` + `on_visual_add` フック    | `ecs/graphics/visual.rs`         | 子エンティティに自動挿入される。Balloon が Visual を持てば Arrangement, BrushInherit, DComp も連鎖挿入   |
| `Arrangement` + `on_arrangement_add` | `ecs/layout/arrangement.rs`      | GlobalArrangement + ArrangementTreeChanged の自動挿入。レイアウト統合の基盤                              |
| `GraphicsCommandList`                | `ecs/graphics/command_list.rs`   | フレーム描画パイプラインの出力先                                                                         |
| `ClipShape`                          | `ecs/graphics/clip.rs`           | `RoundedRectangle`, `RoundedRectangleIndividual` — コンテンツ領域クリッピングに使用可能                  |
| D2D1 ジオメトリAPI                   | `com/d2d/mod.rs`                 | `create_path_geometry()`, `create_rounded_rectangle_geometry()`, `FillGeometry` — しっぽ・角丸描画の基盤 |
| `Rectangle` ウィジェット             | `ecs/widget/shapes/rectangle.rs` | `on_add`/`on_remove` フック + `GraphicsCommandList` 描画パターン。フレーム描画の参考実装                 |
| `BitmapSource` ウィジェット          | `ecs/widget/bitmap_source.rs`    | 画像背景描画用。スキン画像の描画子ウィジットとして活用可能                                               |
| areka モック実装                     | `crates/areka/src/main.rs`       | 現行のアドホックなバルーン模擬。BalloonMarker + 手動配置。将来の置き換え対象                             |

### 1.2 確立済みパターン

- **on_add フックチェーン**: `Visual → Arrangement` の連鎖挿入パターン。Balloon はこのチェーンの起点として同じパターンを踏襲する
- **DeferredWorld::commands()**: on_add フック内で子エンティティ spawn に使用（`on_window_add` で実証済み）
- **ChildOf 階層**: bevy_ecs 0.18 の `ChildOf(parent)` による親子関係構築
- **Changed\<T\> リアクティブクエリ**: `Changed<BalloonSkinDef>` パターンで描画子ウィジットの再構築に使用
- **GraphicsCommandList パイプライン**: Draw ステージで CommandList 生成 → RenderSurface で合成 → Composition でコミット
- **CompositionMode 分岐**: ULW / DComp 両対応。`find_owner_window_composition_mode()` で ChildOf チェーンを辿りモード判定

### 1.3 コーディング規約

- コンポーネント定義: `#[derive(Component)]` + `#[component(on_add = ...)]`
- フック関数シグネチャ: `fn on_xxx_add(mut world: DeferredWorld, context: HookContext)`
- 自動挿入前に `world.get::<T>(entity).is_none()` で既存チェック
- `tracing::debug!` / `tracing::error!` でログ出力

---

## 2. 要件別アセットマップ

### Req 1: 複合ウィジェットエンティティ階層の構築

| AC  | 技術要素                                  | 既存アセット                       | ギャップ                                                         |
| --- | ----------------------------------------- | ---------------------------------- | ---------------------------------------------------------------- |
| AC1 | Balloon → BalloonFrame 子spawn            | `on_window_add` パターン           | **Missing**: `Balloon` コンポーネント + on_add フック            |
| AC2 | BalloonFrame → BalloonContentArea 子spawn | `DeferredWorld::commands()`        | **Missing**: `BalloonFrame`, `BalloonContentArea` コンポーネント |
| AC3 | 3層エンティティ階層                       | `ChildOf` 階層パターン             | **Missing**: 階層構築ロジック（パターンは確立済み）              |
| AC4 | DeferredWorld::commands() 遅延実行        | `on_window_add` で実証済み         | ギャップなし（パターン流用）                                     |
| AC5 | Visual + Arrangement 自動挿入             | `on_visual_add` → Arrangement 連鎖 | ギャップなし（Visual 挿入すれば連鎖で Arrangement 自動付与）     |

**評価**: パターンは完全に確立済み。Balloon / BalloonFrame / BalloonContentArea の3コンポーネント定義と on_add フック実装が必要。

---

### Req 2: バルーンウィジェットの構成

| AC  | 技術要素                     | 既存アセット          | ギャップ                                                    |
| --- | ---------------------------- | --------------------- | ----------------------------------------------------------- |
| AC1 | spawn で描画可能             | on_add フックチェーン | **Missing**: `Balloon` コンポーネント定義（Req 1 と共通）   |
| AC2 | 複数バルーン独立生成         | ECS パターン          | ギャップなし（ECS の自然なパターン）                        |
| AC3 | ECS エンティティ管理         | bevy_ecs 基盤         | ギャップなし                                                |
| AC4 | despawn 時子エンティティ解放 | 既存 despawn 処理     | **Research Needed**: ChildOf cascade despawn が自動か要確認 |

**評価**: Balloon はキャラクターへの参照を持たない純粋な描画ウィジェット。spawn/despawn は ECS の標準パターンで対応可能。

---

### Req 3: バルーンスキン定義インターフェース

| AC  | 技術要素                   | 既存アセット                | ギャップ                                             |
| --- | -------------------------- | --------------------------- | ---------------------------------------------------- |
| AC1 | 背景定義（単色/画像）      | `Brush` コンポーネント      | **Missing**: `BalloonSkinDef` コンポーネント         |
| AC2 | 枠線パラメータ             | —                           | **Missing**: 枠線パラメータ構造体                    |
| AC3 | しっぽ定義（角度・サイズ） | —                           | **Missing**: しっぽ定義構造体（角度パラメータ）      |
| AC4 | パディング定義             | `BoxStyle.padding` パターン | **Missing**: BalloonSkinDef 内のパディングフィールド |
| AC5 | 不正入力フォールバック     | —                           | **Missing**: バリデーション + デフォルトスキン定義   |

**評価**: `BalloonSkinDef` は完全新規。しっぽの方向は角度（f32）で表現。データ構造のみでロジック少量。

---

### Req 4: フレーム描画の委譲設計

| AC  | 技術要素                           | 既存アセット                                     | ギャップ                                                                          |
| --- | ---------------------------------- | ------------------------------------------------ | --------------------------------------------------------------------------------- |
| AC1 | 子ウィジットへの描画委譲構造       | `ChildOf` 階層パターン, `Rectangle` 描画パターン | **Missing**: `BalloonFrame` → 描画子ウィジット spawn・管理ロジック                |
| AC2 | Image 背景の BitmapSource 活用     | `BitmapSource` ウィジット                        | ギャップなし（既存ウィジットを ChildOf で配置）                                   |
| AC3 | 背景・枠線・角丸・しっぽの描画委譲 | D2D1 ジオメトリ API, `Rectangle`, `BitmapSource` | **Missing**: フレーム描画ウィジット（本仕様内 or 孫仕様で対応、設計フェーズ判断） |
| AC4 | SkinDef 変更時の子ウィジット再構築 | `Changed<T>` パターン                            | **Missing**: `Changed<BalloonSkinDef>` による再構築システム                       |
| AC5 | ULW/DComp 両対応                   | 既存 `GraphicsCommandList` パイプライン          | **Constraint**: 子ウィジットが既存パイプラインに乗るため本質的にモード非依存      |

**評価**: 描画の実装責務を BalloonFrame の子ウィジットに委譲。Image 背景は既存 `BitmapSource` を活用可能。balloon01-core は委譲構造とスキン定義管理に集中。

---

### Req 5: バルーン表示制御

| AC  | 技術要素                 | 既存アセット        | ギャップ                                                   |
| --- | ------------------------ | ------------------- | ---------------------------------------------------------- |
| AC1 | 表示/非表示制御          | `Visual.is_visible` | ギャップなし（既存フィールド利用）                         |
| AC2 | 非表示時エンティティ保持 | ECS 設計パターン    | ギャップなし（Visual.is_visible=false + エンティティ保持） |

**評価**: 全 AC が既存コンポーネントの機能で対応可能。新規実装なし。

---

### Req 6: エラーハンドリングと堅牢性

| AC  | 技術要素                        | 既存アセット     | ギャップ                                               |
| --- | ------------------------------- | ---------------- | ------------------------------------------------------ |
| AC1 | SkinDef 不在 → デフォルト       | —                | **Missing**: デフォルト BalloonSkinDef + フォールバック |
| AC2 | 不正パラメータ → フォールバック | `tracing::warn!` | **Missing**: バリデーションロジック                     |

**評価**: スキン定義のフォールバック処理のみ。anchor 関連のエラー処理は不要（anchor 自体が削除されたため）。

---

### Req 7: モジュール配置と拡張性

| AC  | 技術要素                      | 既存アセット                        | ギャップ                                   |
| --- | ----------------------------- | ----------------------------------- | ------------------------------------------ |
| AC1 | `ecs/widget/balloon/mod.rs`   | `ecs/widget/mod.rs`（balloon なし） | **Missing**: ディレクトリ + モジュール作成 |
| AC2 | `ecs/widget/balloon/frame.rs` | —                                   | **Missing**: ファイル作成                  |
| AC3 | ChildOf 拡張ポイント          | `ChildOf` パターン確立済み          | ギャップなし（特別な拡張機構不要）         |
| AC4 | レイヤー依存方向の遵守        | COM → ECS → Message Handling        | ギャップなし（新モジュールは ECS レイヤー内） |

**評価**: ファイル構造のスキャフォールド。既存の `ecs/widget/` 配下に `balloon/` サブモジュールを追加。placement.rs は不要（配置制御はスコープ外）。

---

## 3. ギャップサマリ

### Missing（新規作成が必要）

| #   | アイテム                                                              | 関連要件         | 複雑度                           |
| --- | --------------------------------------------------------------------- | ---------------- | -------------------------------- |
| M1  | `Balloon` コンポーネント + `on_balloon_add` フック                    | Req 1, 2         | 低（on_window_add テンプレート） |
| M2  | `BalloonFrame` コンポーネント                                         | Req 1, 4         | 低                               |
| M3  | `BalloonContentArea` コンポーネント                                   | Req 1            | 低                               |
| M4  | `BalloonSkinDef` コンポーネント（背景・枠線・しっぽ角度・パディング） | Req 3            | 低（データ定義のみ）             |
| M5  | フレーム描画委譲ロジック（SkinDef → 子ウィジット spawn・管理）        | Req 4            | 低〜中（既存ウィジット活用）     |
| M6  | フレーム描画ウィジット（背景・枠線・角丸・しっぽ）                    | Req 4 AC3        | 中（本仕様内 or 孫仕様）         |
| M7  | デフォルト BalloonSkinDef + バリデーション                            | Req 3 AC5, Req 6 | 低                               |
| M8  | モジュール構造（balloon/mod.rs, frame.rs）                            | Req 7            | 低（スキャフォールド）           |

### Research Needed（設計フェーズで調査）

| #   | アイテム                                                         | 影響範囲  |
| --- | ---------------------------------------------------------------- | --------- |
| RN1 | `ChildOf` despawn 時の cascade 動作確認（bevy_ecs 0.18 の仕様） | Req 2 AC4 |

### Constraint（既存アーキテクチャ制約）

| #   | 制約                                                                     | 影響         |
| --- | ------------------------------------------------------------------------ | ------------ |
| C1  | on_add フック内で直接の子 spawn は不可（DeferredWorld::commands() 必須） | Req 1 AC4    |
| C2  | ULW / DComp 両モード対応必須                                             | Req 4 AC5    |
| C3  | BalloonContentArea は BalloonFrame の子（inherited-context.md 制約）     | Req 1 AC2    |
| C4  | Balloon は透過 Visual で自身は CommandList を作らず全描画を子に委譲      | Req 1, Req 4 |
| C5  | Balloon は純粋な描画ウィジェット。配置・追従・ウィンドウ管理の責務なし   | 全体         |

---

## 4. 実装アプローチ検討

### Option A: 既存コンポーネントの拡張

**適用可能性**: 低

既存の `Rectangle` 等の描画ウィジェットにバルーン固有の構造を追加する方法。

- バルーンは複合ウィジェット（3層構造）であり、単一ウィジェットの拡張では表現不可能
- スキン定義・描画委譲は独自の管理が必要

**トレードオフ**:
- ✅ ファイル数が増えない
- ❌ 複合ウィジェットの要件を満たせない
- ❌ 不適切: バルーンは独立した複合描画要素

---

### Option B: 新規コンポーネント群の作成 ⭐

**適用可能性**: 高（推奨候補）

`ecs/widget/balloon/` に新規モジュールを作成し、Balloon / BalloonFrame / BalloonContentArea / BalloonSkinDef を独立コンポーネントとして定義。

- 明確な責務分離: バルーン描画ロジックは balloon/ モジュールに閉じる
- 既存 Visual / Arrangement を on_add チェーンで活用（拡張ではなく構成）
- 後続子仕様は BalloonContentArea の ChildOf に新エンティティを追加するだけ
- **配置・ウィンドウ管理は外部の責務**: Balloon は純粋な描画ウィジェット

**統合ポイント**:
- `Balloon` の on_add で `Visual` を同一エンティティに挿入
- `ecs/widget/mod.rs` に `pub mod balloon;` を追加

**トレードオフ**:
- ✅ 明確な責務分離と拡張性
- ✅ 既存コンポーネントへの影響ゼロ
- ✅ 後続子仕様の独立開発が容易
- ✅ テスト容易（balloon/ 単体でテスト可能）
- ✅ 配置・ウィンドウ管理の責務を持たないためシンプル
- ❌ ファイル数が増加（2ファイル: mod.rs, frame.rs）

---

## 5. 工数・リスク評価

### 工数: **S（1〜3日）**

**根拠**: v1.0 から placement_system と anchor 関連を削除。新規コンポーネント定義は4つ（Balloon, BalloonFrame, BalloonContentArea, BalloonSkinDef）で、いずれも既存 on_add パターンのテンプレート流用。描画は子ウィジット委譲モデルにより既存ウィジット（BitmapSource 等）を活用可能。主要な新規実装はスキン定義と描画委譲ロジックに集中。

### リスク: **低**

**根拠**:
- 全コンポーネントが on_add フックチェーンという確立済みパターンに準拠
- 配置・追従ロジックがスコープ外となり、複雑な外部依存がゼロ
- 描画は子ウィジット委譲モデルにより既存 GraphicsCommandList パイプラインを利用
- D2D1 ジオメトリ API（PathGeometry, RoundedRectangleGeometry）は既に COM ラッパーで利用可能
- 外部依存・未知技術なし

---

## 6. 設計フェーズへの推奨事項

### 推奨アプローチ

**Option B（新規コンポーネント群の作成）** を推奨。純粋な描画ウィジェットとしてのバルーン設計に最適。

### 設計フェーズで決定すべき事項

1. **しっぽ描画のジオメトリ設計** — PathGeometry によるカスタム形状生成の具体的なアルゴリズムと座標計算方式。角度パラメータからの頂点計算
2. **フレーム描画ウィジットの分割** — 背景・枠線・角丸・しっぽの描画ウィジットを本仕様内で新規作成するか孫仕様（balloon02-reference-skin）に分離するか

### 持ち越し調査事項

- **RN1**: bevy_ecs 0.18 での `ChildOf` エンティティ despawn 時の cascade 動作（子自動 despawn か手動必要か）
- バルーンの初期サイズ決定戦略（コンテンツ依存 vs 固定 vs SkinDef 指定）
- areka モック実装から Balloon コンポーネントへの移行パス

### 親仕様への見直し推奨

- **R3（配置制御）**: balloon01-core のスコープから外れた。外部の協調制御システムの責務として再定義が必要
- **R1（ウィンドウ生成）**: バルーンはウィンドウではなく描画ウィジェット。用語と責務の見直しが必要

---

*分析完了。v1.0 から配置制御（Req 5）と anchor 依存を削除し、純粋な描画ウィジェットとして責務を明確化。工数は S（1〜3日）に縮小、リスクは低を維持。*
