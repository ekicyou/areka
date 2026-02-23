# ギャップ分析レポート

| 項目 | 内容 |
|------|------|
| **対象仕様** | wintf-P0-balloon-system |
| **分析日** | 2026-02-23 |
| **対象バージョン** | requirements.md v3.2 |
| **前回分析** | v3.2初回。本レポートで「1グリフ＝1エンティティ」アーキテクチャ分析を追加 |

---

## 1. 現状調査サマリ

### 1.1 関連資産の概要

| 分類 | ファイル/モジュール | 説明 |
|------|---------------------|------|
| ウィンドウ管理 | `ecs/window/components.rs` | `Window`, `WindowStyle`, `CompositionMode` (ULW/DComp) + `on_window_add` フック |
| ウィンドウハンドル | `ecs/window/window_handle.rs` | `WindowHandle` (HWND+HINSTANCE), DPI取得, client↔window座標変換 |
| ウィンドウ位置 | `ecs/window/window_pos.rs` | `WindowPos` (位置/サイズ/SWPフラグ), `ZOrder` enum, builder pattern |
| ウィンドウ生成 | `ecs/window/window_system.rs` | `create_windows` 排他システム: `Without<WindowHandle>` で未作成Window検出→`CreateWindowExW` |
| コマンドキュー | `ecs/window/command.rs` | `SetWindowPosCommand` — thread_localキューでバッチ処理 |
| DPI | `ecs/window/dpi.rs` | `DPI` — DPIスケーリングリソース |
| モニター | `ecs/window/monitor.rs` | `Monitor { bounds, work_area, dpi }` — マルチモニター対応 |
| グラフィックスコア | `ecs/graphics/core.rs` | `GraphicsCore`: D3D11, DXGI, D2D1Factory, DWrite, **共有DeviceContext1つ** |
| グラフィックスコンポーネント | `ecs/graphics/components.rs` | `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty` |
| Visual | `ecs/graphics/visual.rs` | `Visual` { is_visible, opacity, transform_origin }。on_addで`Arrangement`+`BrushInherit`+[DComp: `VisualGraphics`+`SurfaceGraphics`+`SurfaceGraphicsDirty`]連鎖挿入 |
| コマンドリスト | `ecs/graphics/command_list.rs` | `GraphicsCommandList` — `Option<ID2D1CommandList>` ラッパー |
| 合成 | `ecs/graphics/compositor.rs` | `WindowD3D11Compositor` — per-window合成リソース |
| レンダー | `ecs/graphics/compositor_systems/render.rs` | `composite_render_system` (再帰z-order合成+opacity累積) + `ulw_present_system` (ULW転送) |
| ULWユーティリティ | `com/ulw.rs` | `transfer_to_hbitmap` + `present_layered_window` |
| タイプライター | `ecs/widget/text/typewriter.rs` | `Typewriter`, `TypewriterTalk`, `TypewriterLayoutCache` (📖参考実装) |
| タイプライターIR | `ecs/widget/text/typewriter_ir.rs` | Stage 1 `TypewriterToken`, Stage 2 `TimelineItem`, `TypewriterTimeline` |
| タイプライターレイアウト | `ecs/widget/text/typewriter_layout.rs` | `init_typewriter_layout`, `convert_to_timeline` (クラスタベース分解) |
| タイプライター描画 | `ecs/widget/text/typewriter_draw.rs` | `update_typewriters`, `draw_typewriters` (visible_cluster_count + SetDrawingEffect方式) |
| Label | `ecs/widget/text/label.rs` | `Label` (静的テキスト), `TextDirection` (縦横4方向), `TextLayoutResource` |
| Rectangle | `ecs/widget/shapes/rectangle.rs` | `Rectangle` — 最小ウィジェット。`on_add`で`Visual`自動挿入 |
| Brush | `ecs/widget/brushes.rs` | `Brush` {Inherit/Solid}, `Brushes`, `BrushInherit` マーカー, 継承解決パターン |
| BitmapSource | `ecs/widget/bitmap_source/` | PNG画像表示ウィジェット |
| ポインタ | `ecs/pointer/` | `PointerState`, `WheelDelta`, `Phase<T>` (Tunnel/Bubble), 全5種ハンドラ |
| ヒットテスト | `ecs/layout/hit_test/` | `HitTestMode` {None/Bounds/AlphaMask/NamedRegions}, `hit_test_in_window()` |
| ヒットリージョン | `ecs/layout/hit_region/` | `HitRegionMap` (rect/polygon/colormap), `Shape` |
| ドラッグ | `ecs/drag/` | `DragConfig`, `DragEvent`, `OnDrag` |
| レイアウト | `ecs/layout/` | `TaffyStyle`, `BoxStyle`, `Arrangement`→`GlobalArrangement`伝播 |
| DirectWrite | `com/dwrite.rs` | `DWriteFactoryExt`, `DWriteTextLayoutExt` (GetClusterMetrics/HitTestTextPosition) |
| D2Dラッパー | `com/d2d/` | コマンドシンクとコマンドタイプ定義 |
| dola (ランタイム) | `crates/dola/src/runtime/` | `DolaRuntime` — load_document→start→update→subscribe完全ランタイム |
| dola (ストーリーボード) | `crates/dola/src/storyboard.rs` | `Storyboard`, `InterruptionPolicy` (Cancel/Conclude/Trim/Compress/Never) |
| dola (イージング) | `crates/dola/src/easing.rs` | 30+種ネームドイージング + パラメトリック(ベジェ) |
| dola (変数) | `crates/dola/src/variable.rs` | `AnimationVariableDef` — `Integer { typewriter }` フィールドあり |
| モック実装 | `areka/src/main.rs` | `BalloonWindowMarker`, ハードコード配置、ChildOf階層、Typewriter再生 |

### 1.2 確立済みパターン

| パターン | 説明 | 活用先 |
|---------|------|--------|
| **on_add hookチェーン** | `Window` 追加→`Visual`+`WindowPos`+`DPI`自動挿入。`Visual` 追加→`Arrangement`+`BrushInherit`自動挿入 | balloon01-core: `BalloonWindow` on_add で同パターン踏襲 |
| **排他システム** | `create_windows`: `World` 直接取得→即時反映 | バルーン固有の初期化に同パターン適用 |
| **`ChildOf(parent)` 階層** | 親子関係によるウィジェットツリー構築。`Children.iter()` が Z-order の権威的ソース | **全子仕様**: 描画責務ごとにエンティティ分離。**グリフエンティティの親子構造にも適用** |
| **コマンドキュー** | `SetWindowPosCommand`: thread_localキューでバッチ処理 | Req 2: 位置追従のウィンドウ操作 |
| **描画パイプライン** | Widget→`GraphicsCommandList`→`composite_render_system`(z-order合成)→`ulw_present_system`(ULW転送) | 全描画責務の基本フロー。**グリフエンティティもこのパイプラインに自然に乗る** |
| **2段階IR** | Stage 1 `TypewriterToken`→Stage 2 `TimelineItem`。外部/内部の責務分離 | balloon02-content: 同パターンを参考にグリフベースIR設計 |
| **ブラシ継承** | `Brush::Inherit`→`resolve_inherited_brushes`で親から解決 | グリフエンティティのテキスト色を親から継承 |
| **SparseSetハンドラ** | `EventHandler<T>` は SparseSet ストレージ（少数エンティティに最適化） | balloon03-link, balloon04-choice: イベントハンドラ |
| **ダーティビット** | `SurfaceGraphicsDirty`, `Changed<T>` でシステムトリガー | **グリフエンティティの再描画最適化にも適用** |
| **Entity-per-Visual描画** | 各エンティティが `Visual` を持てば自動的にレンダリングパイプラインに参加 | **1グリフ=1エンティティの前提パターン** |

### 1.3 制約事項

- `on_add` フック内では `Commands` が使えない（`DeferredWorld` のみ）
- `SetWindowPosCommand` は tick 終了後にフラッシュされる（即時反映不可）
- ULW 合成モードでは `UpdateLayeredWindow` のたびに全面再描画が必要
- PointerState の `WheelDelta` は蓄積されるが、スクロールウィジェットが未実装
- **wintf ↔ dola 依存なし**: `Cargo.toml` レベルで接続されていない。統合層の設計が必要
- Typewriter の描画方式: `visible_cluster_count` でオン/オフのみ。文字単位の不透明度制御は未実装
- `HitTestPoint`（座標→文字位置）API未ラップ。`HitTestTextPosition`（文字位置→座標）のみ実装済み
- **`d2d_device_context` はグローバル共有の1つ**: すべての Draw システムが同一 DC で CommandList 作成。**並列 Draw は不可能**
- **DComp モードはエンティティ単位コスト高**: 各エンティティに `IDCompositionVisual3` + `IDCompositionSurface`（GPUテクスチャ）を要する

### 1.4 モック実装の状態（`areka/src/main.rs`）

モックが実装済みの機能:
- シェル(320×420px PNG) + バルーン(200×350px) の2ウィンドウ生成
- `WS_POPUP | WS_VISIBLE` + `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`
- `ChildOf(parent)` による親子階層: balloon → background(Rectangle) → typewriter(Typewriter)
- `OnDrag` で手動追従、`OnPointerPressed` でダブルクリック終了
- `TypewriterToken::Text` / `Wait` によるテキストストリーム再生

モックの制約（製品化ギャップ）:
- 定数オフセット (`BALLOON_OFFSET_X=335`)、固定サイズ (200×350)
- 手動追従ハンドラ（ECSシステムとしての自動追従なし）
- スクロール/リンク/選択肢/エフェクト なし

### 1.5 レンダリングパイプライン深掘り調査

#### ULW モード（バルーンの主要対象）

**描画データフロー**:
```
Entity spawn → Visual on_add (Arrangement+BrushInherit) → PreLayout → Layout →
Widget Draw (CreateCommandList→BeginDraw→draw→EndDraw→Close→insert GraphicsCommandList) →
composite_render_system (再帰z-order合成、opacity累積、dirty subtree check) →
ulw_present_system (単一ビットマップへ転送→UpdateLayeredWindow)
```

**エンティティ単位コスト（ULWモード）:**
- `Visual` コンポーネント: ~16 bytes
- `Arrangement` + `GlobalArrangement`: ~120 bytes
- `BrushInherit` マーカー: ~1 byte
- `GraphicsCommandList` (COM): 数百 bytes
- **DComp関連コンポーネントは不要**（Surface, VisualGraphics なし）
- `composite_render_system` での走査は 1 visit/entity

**最適化機構:**
- `Changed<GraphicsCommandList>`: CommandList が変更されたエンティティのみ再描画
- `Changed<GlobalArrangement>`: 位置・サイズ変更検出
- ダーティサブツリー判定: 再帰走査で変更なしサブツリーをスキップ

#### DComp モード

**エンティティ単位コスト（DCompモード）:**
- ULW モードのすべて **＋**:
- `VisualGraphics` (Table): `IDCompositionVisual3` — GPU上の Visual ノード
- `SurfaceGraphics` (Table): `IDCompositionSurface` — **GPUテクスチャ**（遅延作成: CommandList 存在時のみ）
- `SurfaceGraphicsDirty` (Table): 再描画要求フレーム番号
- `IDCompositionDevice::Commit()` のコストは Visual 数に比例

**遅延 Surface 作成**: `deferred_surface_creation_system` は `GraphicsCommandList` が挿入されたエンティティにのみ Surface を作成。CommandList 削除時は Surface をクリア。

### 1.6 DirectWrite API カバレッジ

#### ラップ済み API

| API | メソッド | 用途 |
|-----|---------|------|
| `DWriteCreateFactory` | `dwrite_create_factory()` | ファクトリ作成 |
| `CreateTextFormat` | `DWriteFactoryExt::create_text_format()` | フォント設定 |
| `CreateTextLayout` | `DWriteFactoryExt::create_text_layout()` | テキストレイアウト作成 |
| `GetClusterMetrics` | `DWriteTextLayoutExt::get_cluster_metrics()` | クラスタメトリクス取得 |
| `GetClusterCount` | `DWriteTextLayoutExt::get_cluster_count()` | クラスタ数取得 |
| **`HitTestTextPosition`** | `DWriteTextLayoutExt::hit_test_text_position()` | **文字位置→座標変換** (`HitTestResult { point_x, point_y, metrics { left, top, width, height } }`) |

#### 未ラップだがグリフ単位制御に重要な API

| API | 重要度 | 用途 |
|-----|--------|------|
| `HitTestPoint` (座標→文字位置) | 高 | リンクヒットテスト (balloon03-link) |
| `IDWriteTextRenderer` (カスタムレンダラ) | 中〜高 | グリフラン単位の描画分配（エンティティ方式の描画手段候補） |
| `GetLineMetrics` | 中 | 行単位メトリクス（スクロール、ルビ配置） |
| `GetOverhangMetrics` | 低 | テキスト領域のオーバーハング計算 |

#### `HitTestTextPosition` で取得できるグリフ位置情報

各クラスタ/グリフの矩形 `(left, top, width, height)` が取得可能。**グリフエンティティの `Arrangement` 設定に直接使用できる**。

### 1.7 typewriter 実装の知見

**2段階IR**:
- Stage 1: `TypewriterToken` { Text / Wait / FireEvent } — 外部インターフェース
- Stage 2: `TimelineItem` { Glyph { cluster_index, show_at } / Wait / FireEvent } — 内部タイムライン

**描画方式**: 1つの `IDWriteTextLayout` を全体で共有し、`SetDrawingEffect` で非表示部分に透明ブラシを適用。`DrawTextLayout` で一括描画。

**重要な示唆**: typewriter は「1テキスト=1エンティティ=1CommandList」方式。グリフ単位のエンティティ分解は行っておらず、これがバルーンシステムの**新規設計部分**となる。

---

## 2. 「1グリフ＝1エンティティ」アーキテクチャ分析

### 2.1 コンセプト

> **着想**: 「テキストアニメーションですが、１グリフ＝１エンティティに分割して個別制御するくらいのことを考えたいです。分割さえできてしまえばコマンドリストの再生とアニメーションの割り当てに帰結しますので。」

テキストを個々のグリフ（表示上の最小文字単位）に分割し、各グリフを独立した ECS エンティティとして管理するアーキテクチャ。このアプローチの核心は、**テキストアニメーションを「テキスト固有の問題」から「エンティティの汎用的なプロパティ制御」へ帰結させる**点にある。

```
BalloonContent (Entity)
  ├─ GlyphContainer: テキスト全体のレイアウト管理、共有TextLayout保持
  │
  ├─ GlyphEntity[0] — ChildOf(container)
  │    ├─ Visual { opacity, is_visible, transform_origin }
  │    ├─ Arrangement { offset: (x0, y0), size: (w0, h0) }  ← HitTestTextPositionから算出
  │    ├─ GraphicsCommandList  ← 当該グリフ1文字の描画
  │    └─ (GlyphInfo { cluster_index, text_position, ... })
  │
  ├─ GlyphEntity[1] — ChildOf(container)
  │    └─ (同構造)
  ├─ ...
  └─ GlyphEntity[N] — ChildOf(container)
```

### 2.2 既存パイプラインとの統合

「1グリフ＝1エンティティ」方式の最大の利点は、**既存の描画パイプラインに変更を加えることなく動作する**点にある:

| 既存機構 | グリフエンティティでの役割 |
|---------|-------------------------|
| `Visual.opacity` | **文字単位フェードイン/アウト** — dola が opacity を 0→1 制御するだけ（G17 解決） |
| `Visual.is_visible` | **文字単位の表示/非表示** — タイプライター効果の基盤 |
| `Arrangement.offset` | **文字位置** — `HitTestTextPosition` で取得した座標を設定 |
| `GraphicsCommandList` | **文字描画** — 各グリフが自身の CommandList に1文字を描画 |
| `composite_render_system` | **自動合成** — z-order に従いすべてのグリフを1枚に合成（変更不要） |
| `BrushInherit` | **テキスト色の継承** — 親コンテナから色を継承 |
| `ChildOf(parent)` | **親子階層** — コンテナ→グリフの関係を既存の仕組みで表現 |
| `Changed<T>` 検出 | **再描画最適化** — 変更のないグリフは再描画をスキップ |

### 2.3 レンダリングモード別実現可能性

#### ULW モード ✅ 推奨

- 各グリフエンティティのコスト: ECS コンポーネント群 + CommandList (COM) のみ
- **GPU 単位のリソース不要**（全グリフを1枚のビットマップに合成）
- `composite_render_system` が再帰走査で全グリフを子として合成
- ダーティ判定により変更なしフレームのオーバーヘッドは最小限

#### DComp モード ⚠️ 非推奨（グリフ単位では）

- 各グリフに `IDCompositionVisual3` + `IDCompositionSurface` (GPU テクスチャ) が必要
- 100文字 = 100個の小テクスチャ → **GPU メモリ断片化リスク**
- `IDCompositionDevice::Commit()` のコストが Visual 数に比例
- DComp モードのバルーンでは、グリフ単位エンティティの**代わりに論理エンティティ方式**（§2.5参照）を検討すべき

### 2.4 定量的コスト見積もり（ULW モード）

| リソース | 1グリフ | 100グリフ | 200グリフ |
|----------|---------|----------|----------|
| Entity (bevy_ecs Table) | ~100 B | ~10 KB | ~20 KB |
| Visual + Arrangement + GlobalArrangement | ~140 B | ~14 KB | ~28 KB |
| GraphicsCommandList (COM) | ~数百 B | ~数十 KB | ~数十 KB |
| `CreateCommandList` 呼出/dirty frame | 1回 | 100回 | 200回 |
| `composite_render_system` 走査 | 1 visit | 100 visits | 200 visits |

**ボトルネック候補**: `CreateCommandList` × N 回/dirty frame。ただし:
- フェードイン中のグリフのみが dirty（全グリフ同時再描画は稀）
- `Changed<GraphicsCommandList>` によりクリーンなグリフはスキップ
- 通常のバルーンテキストは 20〜200 文字程度（十分に許容範囲）

**ECS スケーリング**: bevy_ecs 0.18 は Table ストレージで連続メモリ配置。10,000+ エンティティはゲーム向けの通常ユースケースであり、200 グリフエンティティは問題にならない。

### 2.5 代替: 論理エンティティ方式

グリフエンティティが**描画（CommandList）を持たず**、位置情報とアニメーション状態のみを保持する方式:

```
GlyphEntity[i]:
  ├─ GlyphInfo { cluster_index, text_position }
  ├─ GlyphAnimState { opacity: f32, offset: Vec2 }  ← dolaがこれを制御
  └─ (Visual/CommandList なし)

GlyphContainer:
  ├─ 共有 IDWriteTextLayout
  └─ draw時: 全GlyphEntityのAnimStateを読み取り、
     SetDrawingEffect + DrawTextLayout で一括描画
```

**トレードオフ:**
- ✅ CommandList × N のコストが不要（1つの CommandList で全文描画）
- ✅ DComp モードでも使用可能（Surface は 1 つのみ）
- ❌ 既存の `Visual.opacity` → `composite_render_system` パスを使えない（専用システム必要）
- ❌ `SetDrawingEffect` でアルファを制御する場合、グリフ単位の移動エフェクトが困難
- ❌ 描画ロジックが typewriter の拡張に近くなり、「コマンドリストの再生に帰結する」という設計意図から外れる

### 2.6 要件への影響分析

| 要件 | 従来の想定 | 1グリフ=1エンティティでの変化 |
|------|-----------|---------------------------|
| **Req 6 AC2** (グリフ分割) | 「配列を生成」 | 「グリフエンティティ群を生成（spawn）」に具体化 |
| **Req 7 AC1** (タイプライター効果) | 独自制御方式を設計 | `Visual.is_visible` の順次切替に帰結 |
| **Req 7 AC4** (dolaマッピング) | マッピング構造を新規設計 | dola変数 → `Visual.opacity` / `Arrangement.offset` バインディング |
| **Req 11 AC1-2** (フェードイン/アウト) | 文字単位不透明度制御を新規実装 | **`Visual.opacity` で解決済み**（既存インフラ活用） |
| **Req 11 AC3** (タイミング) | エフェクトタイミング管理を新規実装 | dola の Storyboard タイムラインに帰結 |
| **Req 11 AC4** (複数エフェクト同時) | 複合エフェクトエンジン新規 | 各グリフに複数 dola 変数を割り当て |
| **Req 12 AC1-5** (dola統合) | 文字単位変数バインディング全設計 | **エンティティプロパティバインディング**に標準化 |

### 2.7 アーキテクチャ評価サマリ

| 評価軸 | フルエンティティ方式（ULW） | 論理エンティティ方式 |
|--------|---------------------------|-------------------|
| 既存パイプライン活用 | ◎ 変更不要 | △ 専用描画システム必要 |
| dola 統合の自然さ | ◎ エンティティプロパティ直接制御 | ○ AnimState経由の間接制御 |
| per-glyph opacity | ◎ Visual.opacity で解決 | △ SetDrawingEffect で近似 |
| per-glyph 移動 | ◎ Arrangement.offset | × 困難（全文レイアウトが崩れる） |
| DComp 対応 | × 非推奨 | ◎ 問題なし |
| 描画コスト (N glyphs) | △ N × CreateCommandList | ◎ 1 × CreateCommandList |
| 設計の一貫性 | ◎ 「分割→再生に帰結」 | △ 専用ロジックが追加 |

**推奨**: ULW モードのバルーンでは**フルエンティティ方式**を採用。DComp モード対応が将来必要になった場合は論理エンティティ方式をフォールバックとして検討。

---

## 3. 要件−資産マッピング

### 子仕様 1: balloon01-core（DR-1: フレーム描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 1: ウィンドウ生成** | 1 | `Window` + `on_window_add` フック | バルーン専用コンポーネント（`BalloonWindow`）未定義。シェル↔バルーン ECS関係なし | Missing |
| | 2 | `Window` は複数生成可能 | キャラクター単位の多重バルーン管理ロジックなし | Missing |
| | 3 | `CompositionMode::ULW` で透過ウィンドウ生成済み | ✅ 既存パターンで対応可能（モック実証済み） | — |
| | 4 | `on_remove` フック基盤あり | バルーン固有のクリーンアップロジック未定義 | Missing |
| | 5 | bevy_ecs でエンティティ管理済み | バルーン専用マーカー/関係コンポーネント未定義 | Missing |
| **Req 2: フレーム描画** | 1 | なし | スキン定義インターフェース未定義 | Missing |
| | 2 | なし | スキンに基づく背景描画未実装 | Missing |
| | 3 | なし | 角丸・枠線描画ウィジェット未実装 | Missing |
| | 4 | なし | しっぽ描画未実装 | Missing |
| | 5 | なし | スキン更新時の再描画機構未実装 | Missing |
| **Req 3: 配置制御** | 1 | モック: `BALLOON_OFFSET_X=335` ハードコード | 自動配置アルゴリズム未実装 | Missing |
| | 2 | なし | 配置方向（上/下/左/右）指定機能なし | Missing |
| | 3 | モック: `OnDrag` ハンドラで手動 `SetWindowPosCommand` | ECSシステムとしての自動追従未実装 | Missing |
| | 4 | `Monitor.work_area` でデスクトップ領域取得可能 | 自動反転ロジック未実装 | Missing |
| | 5 | なし | オフセット距離設定コンポーネント未定義 | Missing |
| **Req 4: 表示制御** | 1 | `WindowPos.show_window` / `hide_window` フラグ | ✅ 既存機構で対応可能 | — |
| | 2 | `WindowPos.zorder` (TopMost 等) | ✅ 前面表示は既存 ZOrder で可能 | — |
| | 3 | `WindowPos.hide_window` | ✅ ウィンドウ非表示+エンティティ保持は既存で可能 | — |
| | 4 | `WindowPos.size` + `BoxStyle` | ✅ サイズ設定可能 | — |

### 子仕様 2: balloon02-content（DR-2+DR-3: ビューポート+テキスト基本描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 5: コンテンツ領域** | 1 | `BoxStyle` + `LayoutRoot` 階層 | コンテンツ領域コンポーネント未定義 | Missing |
| | 2 | `BoxStyle` に margin/padding あり | ✅ 既存BoxStyleで設定可能 | — |
| | 3 | `BoxStyle` の content-based sizing | コンテンツ→ウィンドウへのサイズ自動調整ロジック未実装 | Missing |
| | 4 | `BoxStyle` に max_width/max_height | ✅ taffy の制約で設定可能 | — |
| **Req 6: グリフ分割** | 1 | `TextDirection` (縦横4方向), `DWriteFactoryExt` | 📖 typewriter P0のDirectWrite実装を参考に新規実装が必要 | Missing |
| | 2 | `HitTestTextPosition` ラップ済み（文字位置→矩形座標取得可能） | **グリフエンティティ spawn パイプライン未実装**。ただし `HitTestTextPosition` で各グリフの `(left, top, width, height)` は取得可能 → `Arrangement` への変換は直接的 | Missing |
| | 3 | `Typewriter { font_family, font_size }` | 📖 参考。新規実装でのスタイル設定はフォント・サイズ・色の設計が必要 | Constraint |
| **Req 7: 表示制御** | 1 | `Visual.is_visible` (エンティティ単位の表示切替) | **グリフエンティティ方式なら `is_visible` の順次切替でタイプライター効果が実現可能**。制御ロジックは新規 | Constraint |
| | 2 | なし | 濁点・半濁点ウェイト調整が未実装 | Missing |
| | 3 | `TypewriterToken::Wait` (固定待機) | さくらスクリプト的ウェイト挿入マーカー未実装 | Missing |
| | 4 | `Visual.opacity` + dola (未接続) | **グリフエンティティ方式なら dola→`Visual.opacity` バインディングに帰結**。dola↔wintf統合層のみ新規 | Missing |
| **Req 8: スクロール** | 1 | なし | スクロールコンテナウィジェット未実装 | Missing |
| | 2 | なし | テキスト描画進行追従のスクロール制御未実装 | Missing |
| | 3 | `WheelDelta` 取得可能 | ホイール→スクロール変換ロジック未実装 | Missing |
| | 4 | なし | ページ送り機構未実装 | Missing |

### 子仕様 3: balloon03-link（DR-4: リンク描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 9: クリッカブルテキスト** | 1 | `HitRegionMap` (rect/polygon) | テキスト位置→ヒットリージョン自動生成が未実装 | Missing |
| | 2 | `Phase<T>` イベントシステム (完備) | リンクイベント型（`LinkClicked { action }` 等）未定義 | Missing |
| | 3 | なし | リンク外観カスタマイズ機構未実装 | Missing |
| | 4 | `OnPointerEntered/Exited` (5種ハンドラ完備) | **グリフエンティティが個別にポインタイベントを受けられる可能性あり**。ホバー状態管理は新規 | Missing |
| | 5 | `DWriteTextLayoutExt::hit_test_text_position` (位置→座標) | **`HitTestPoint` (座標→位置) が未ラップ**。リンクヒットテストに必須 | Missing |
| | 6 | `TypewriterToken` (Text/Wait/FireEvent のみ) | リンク用トークン variant 未定義 | Missing |

### 子仕様 4: balloon04-choice（DR-5: 選択肢UI描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 10: 選択肢バルーン** | 1 | `Window` + balloon01-core の `BalloonAnchor` | ChoiceBalloon 専用コンポーネント未定義 | Missing |
| | 2 | `BoxStyle` (flexbox column対応) | ✅ flexbox 縦並び可能 | — |
| | 3 | `Phase<T>` イベントシステム | 選択肢イベント型（`ChoiceSelected { index, id }` 等）未定義 | Missing |
| | 4 | `OnPointerEntered/Exited` | ホバー状態ウィジェット未実装（ボタン相当ウィジェットがない） | Missing |
| | 5 | WM_KEYDOWN (ESC のみ) | キーボードナビゲーション基盤未実装（上下キー・Enter） | Missing |

### 子仕様 5: balloon05-text-effects（DR-6: テキストエフェクト描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 11: 文字単位エフェクト** | 1 | **`Visual.opacity` + `composite_render_system`** | **グリフエンティティなら既存の opacity 合成で対応可能**。ただし dola→opacity バインディングが未実装 | Constraint |
| | 2 | `Visual.opacity` (0→1の逆) | グリフエンティティなら同上。バインディング未実装 | Constraint |
| | 3 | なし | エフェクトタイミング・継続時間の文字ごと管理未実装。**dola Storyboard のタイムラインに帰結する設計** | Missing |
| | 4 | なし | 複数エフェクト同時適用エンジン未実装。**各グリフに複数 dola 変数を割り当てる設計** | Missing |
| | 5 | **`Arrangement.offset` + `GlobalArrangement` 伝播** | グリフエンティティは個別に `Arrangement` を持つため、**描画領域管理は既存のレイアウトシステムに帰結** | Constraint |
| **Req 12: dola統合** | 1 | `dola`: `DolaRuntime` + `compile_storyboard` 完全ランタイム | **wintf↔dola依存なし（Cargo.tomlレベルで未接続）** | Missing |
| | 2 | `dola::easing`: 30+種ネームドイージング + パラメトリック | イージング→エンティティプロパティへの適用機構未実装 | Missing |
| | 3 | `dola::playback`: `PlaybackState` (Idle/Playing/Paused/Completed/Cancelled) | dola↔bevy_ecs スケジュール統合未実装 | Missing |
| | 4 | `dola`: `InterruptionPolicy` (Cancel/Conclude/Trim/Compress/Never) | アニメーション中断制御のECS統合未実装 | Missing |
| | 5 | `dola::variable`: `AnimationVariableDef::Integer { typewriter }` フィールドあり | **グリフエンティティ方式ではエンティティプロパティバインディングに帰結**。バインディング機構は未実装 | Missing |

### 子仕様 6: balloon06-ruby（DR-7: ルビ描画）**[P1]**

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 13: ルビ表示** | 1 | `IDWriteTextLayout` 基本ラップのみ | ルビ用 DirectWrite API 未ラップ（`IDWriteTextLayout1` 以降） | Missing |
| | 2 | なし | 横書きルビ配置ロジック未実装 | Missing |
| | 3 | なし | 縦書きルビ配置ロジック未実装 | Missing |
| | 4 | なし | ルビフォントサイズ自動調整未実装 | Missing |
| | 5 | IR: `TypewriterToken` (Text/Wait/FireEvent のみ) | ルビ用トークン variant 未定義 | Missing |

### 子仕様 7: balloon-reference-skin（検証用スキン）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 14: 検証用バルーンスキン** | 1 | なし | 単色背景スキン定義未実装 | Missing |
| | 2 | なし | 角丸矩形形状定義未実装 | Missing |
| | 3 | なし | しっぽ座標・形状定義未実装 | Missing |

---

## 4. ギャップサマリ

### 4.1 主要ギャップ一覧

| # | ギャップ | 影響範囲 | 深刻度 | v3.2-r1からの変化 |
|---|---------|---------|--------|-------------------|
| G1 | バルーン専用ECSコンポーネント群が未定義 | balloon01-core 全体 | 高 | — |
| G2 | シェル↔バルーンのECSリレーション機構なし | Req 1, 3 | 高 | — |
| G3 | 自動配置・追従・反転アルゴリズム未実装 | Req 3 全AC | 高 | — |
| G4 | **グリフエンティティ spawn パイプライン未実装** | Req 6 AC2 | **高** | 🔄 「配列生成」→「エンティティ spawn」に具体化。`HitTestTextPosition` で座標取得は可能 |
| G5 | **dola→エンティティプロパティ・バインディング未実装** | Req 7 AC4, Req 12 | **高** | 🔄 「マッピング構造」→「dola→ECSプロパティバインディング」に具体化。`Visual.opacity` / `Arrangement.offset` への直接バインディング |
| G6 | 濁点・半濁点ウェイト調整未実装 | Req 7 AC2 | 中 | — |
| G7 | さくらスクリプト的ウェイト挿入未実装 | Req 7 AC3 | 中 | — |
| G8 | ビューポート/クリッピング未実装 | Req 5, 8 | 中 | — |
| G9 | スクロールコンテナウィジェット未実装 | Req 8 全AC | 中 | — |
| G10 | コンテンツ→ウィンドウ サイズ自動調整未実装 | Req 5 AC3 | 中 | — |
| G11 | `HitTestPoint`（座標→文字位置）API未ラップ | Req 9 AC5 | 中 | — |
| G12 | リンクイベント型・ホバー状態管理未定義 | Req 9 全AC | 中 | — |
| G13 | テキスト入力形式にリンク variant なし | Req 9 AC6 | 中 | — |
| G14 | ChoiceBalloon専用コンポーネント未定義 | Req 10 AC1 | 中 | — |
| G15 | ボタン相当ウィジェット未実装 | Req 10 AC4 | 中 | — |
| G16 | キーボードナビゲーション基盤不足（ESCのみ） | Req 10 AC5 | 中 | — |
| G17 | ~~文字単位不透明度制御未実装~~ → **グリフエンティティ方式で解決可能** | Req 11 全AC | ~~高~~ → **低** | 🔄 `Visual.opacity` + `composite_render_system` で既存パイプラインに帰結。**dola バインディング（G5）のみが残課題** |
| G18 | **wintf↔dola Cargo.toml依存なし（完全未接続）** | Req 12 全AC | **最高** | — |
| G19 | DolaRuntime↔bevy_ecsスケジュール統合未実装 | Req 12 AC3 | 高 | — |
| G20 | ~~文字単位エフェクト定義フォーマット未設計~~ → **エンティティプロパティに標準化** | Req 11, 12 | ~~高~~ → **中** | 🔄 グリフエンティティの `Visual.opacity` / `Arrangement.offset` 等の ECS プロパティに帰結。エフェクト固有フォーマットは不要 |
| G21 | テキスト表示タイミング↔dola開始タイミング同期未実装 | Req 12 AC5 | 高 | 🔄 グリフの `is_visible` 切替タイミングと dola Storyboard 開始の協調。設計は明確化（グリフ表示イベント→dola開始トリガー） |
| ~~G22~~ | ~~フレーム描画ウィジェット未実装~~ | — | — | ✅ Req 2で解決済み |
| G23 | DirectWriteルビAPI未ラップ (P1) | Req 13 全AC | 中 | P1のため低優先 |
| G24 | テキスト入力形式にルビ variant なし (P1) | Req 13 AC5 | 中 | P1のため低優先 |
| **G25** | **グリフエンティティの描画方式未決定** | Req 6 AC2, Req 11 | **中** | 🆕 各グリフが自身の CommandList に1文字を描画する方式の詳細設計が必要（per-char TextLayout / カスタム TextRenderer 等） |
| **G26** | **グリフエンティティのライフサイクル管理未設計** | Req 6, 7 | **中** | 🆕 テキスト変更時のグリフエンティティ群の生成・破棄・再利用戦略が未設計 |

### 4.2 「1グリフ＝1エンティティ」によるギャップ変化サマリ

| 変化 | 影響 |
|------|------|
| **G17: 深刻度 高→低** | 文字単位不透明度が `Visual.opacity` で直接制御可能になった。専用実装不要 |
| **G20: 深刻度 高→中** | エフェクト定義が ECS プロパティに標準化。専用フォーマット設計が不要 |
| **G4: 具体化** | 「グリフ配列生成」→「グリフエンティティ spawn + Arrangement 設定」に具体化。`HitTestTextPosition` で座標取得の道筋が明確 |
| **G5: 具体化** | 「マッピング構造」→「dola→ECSプロパティ・バインディング」に具体化。汎用的な dola 統合の一部として設計可能 |
| **G21: 具体化** | グリフ表示イベント（`is_visible` 切替）→ dola Storyboard 開始 の因果関係が明確化 |
| **G25, G26: 新規** | エンティティ方式固有の新たな設計課題 |

### 4.3 既存資産の活用可能ポイント

| 資産 | 活用先 | 活用方法 |
|------|--------|---------|
| `Window` + `on_window_add` hookチェーン | Req 1: バルーンウィンドウ生成 | `BalloonWindow` on_add で同パターン踏襲 |
| `CompositionMode::ULW` | Req 1 AC3: 透過ウィンドウ | そのまま利用可能（モック実証済み） |
| `WindowPos` + `SetWindowPosCommand` | Req 3, 4: 位置・表示制御 | コマンドキューパターン再利用 |
| `Monitor.work_area` | Req 3 AC4: デスクトップ境界判定 | 値取得済み。反転判定ロジックのみ新規 |
| `BoxStyle` (margin/padding/flex) | Req 5: コンテンツ領域レイアウト | そのまま利用可能 |
| `TextDirection` (縦横4方向) | Req 6 AC1: 縦書き・横書き | DirectWrite統合パターンを参考 |
| `TypewriterTimeline` (2段階IR) | Req 6+7: テキスト描画 | 📖 IRパターンを参考に設計 |
| `DWriteTextLayoutExt` | Req 6: DirectWrite操作 | 📖 `HitTestTextPosition` でグリフ座標取得 |
| **`Visual.opacity` + `composite_render_system`** | **Req 11: 文字単位エフェクト** | **グリフエンティティの opacity を直接制御（G17解決の根拠）** |
| **`Visual.is_visible`** | **Req 7 AC1: タイプライター効果** | **グリフエンティティの表示切替でタイプライター効果を実現** |
| **`Arrangement.offset`** | **Req 11: 移動エフェクト** | **グリフエンティティの位置を dola で制御** |
| `WheelDelta` | Req 8 AC3: ホイールスクロール | 値取得済み。消費ロジックのみ新規 |
| `Phase<T>` (Tunnel/Bubble) イベントシステム | Req 9, 10: イベント発火 | イベント型の追加のみ |
| `OnPointerEntered/Exited` (全5種ハンドラ) | Req 9 AC4, Req 10 AC4: ホバー | ポインタイベントフック再利用 |
| `HitRegionMap` (rect/polygon/colormap) | Req 9: リンクヒットテスト | テキスト座標→領域生成が新規 |
| `DolaRuntime` (load→start→update→subscribe) | Req 12: dola統合 | ランタイムは完成。wintf統合層が新規 |
| `AnimationVariableDef::Integer { typewriter }` | Req 12: dola×テキスト連携 | 設計時点でTypewriter統合が想定されていた |
| `EasingFunction` (30+種) | Req 12 AC2: イージング | そのまま利用可能 |
| `InterruptionPolicy` (5種) | Req 12 AC4: 中断制御 | そのまま利用可能 |
| モック実装 (`areka/src/main.rs`) | 全子仕様: 構築パターン | エンティティ構築パターンを参考 |

---

## 5. 実装アプローチ選択肢

### Option A: グリフ＝フルエンティティ（推奨）

**方針**: 各グリフを `Visual` + `GraphicsCommandList` を持つ完全なエンティティとして spawn。ULW モードを前提とし、既存描画パイプラインをそのまま活用。

- **テキスト表示パイプライン**:
  1. **分割フェーズ**: `IDWriteTextLayout` 作成 → `HitTestTextPosition` で各グリフの矩形取得 → グリフエンティティ群を `ChildOf(container)` で spawn。各エンティティに `Visual` + `Arrangement(offset, size)` を設定
  2. **描画フェーズ**: 各グリフエンティティが自身の `GraphicsCommandList` に1文字を描画。`Changed<T>` 検出によりダーティなグリフのみ再描画
  3. **アニメーションフェーズ**: dola が各グリフエンティティの `Visual.opacity` / `Arrangement.offset` 等を制御。`composite_render_system` が合成

- **新規モジュール構成**:
  - `ecs/widget/balloon/` — バルーンコア（`BalloonWindow`, `ChoiceBalloon`; DR-1）
  - `ecs/widget/balloon/placement.rs` — 配置アルゴリズム（`BalloonAnchor`, `BalloonPlacement`）
  - `ecs/widget/balloon/content.rs` — コンテンツ領域管理（DR-2: ビューポート+スクロール）
  - `ecs/widget/text/glyph.rs` — **グリフエンティティ定義+spawn パイプライン**
  - `ecs/widget/text/glyph_draw.rs` — **グリフ単位描画システム**
  - `ecs/widget/text/glyph_timeline.rs` — **タイプライター効果のタイムライン管理**
  - `ecs/widget/text/link.rs` — リンクウィジェット（DR-4）
  - `ecs/widget/choice.rs` — 選択肢項目ウィジェット（DR-5）
  - `ecs/dola_bridge/` — **dola↔bevy_ecs統合層**（DolaRuntimeのECSリソース化、プロパティバインディング）
  - `com/dwrite_ext.rs` — `HitTestPoint` + 追加DirectWrite拡張

**トレードオフ**:
- ✅ 既存描画パイプラインに変更不要（`composite_render_system` がそのまま動作）
- ✅ 文字単位エフェクトが`Visual.opacity`で解決（G17が消滅）
- ✅ 「分割→再生に帰結」という設計意図に最も忠実
- ✅ dola統合が「エンティティプロパティバインディング」に標準化（バルーン以外でも再利用可能）
- ✅ グリフ単位の移動エフェクト（スライドイン等）も `Arrangement.offset` で自然に実現
- ❌ ULW モード前提（DComp モードでは非推奨）
- ❌ N × `CreateCommandList` のコスト（ダーティグリフのみに限定で軽減）
- ❌ グリフエンティティのライフサイクル管理（テキスト変更時の spawn/despawn）が必要

### Option B: グリフ＝論理エンティティ（ハイブリッド）

**方針**: グリフエンティティは位置情報+アニメーション状態のみを保持し、描画は親コンテナが `SetDrawingEffect` で一括描画。

- **データフロー**:
  - グリフエンティティ: `GlyphInfo` + `GlyphAnimState { opacity, offset }` のみ（`Visual`/`CommandList` なし）
  - 描画時: 親エンティティが全グリフの `AnimState` を読み取り、`SetDrawingEffect` で各文字のアルファ値を設定 → `DrawTextLayout` で一括描画

**トレードオフ**:
- ✅ CommandList × 1 で全文描画（描画コスト低）
- ✅ DComp モードでも問題なし
- ❌ 既存の `Visual.opacity` パスが使えない（専用描画システム必要）
- ❌ グリフ単位の移動エフェクトが困難（テキストレイアウト全体が崩れる）
- ❌ 「コマンドリストの再生に帰結する」設計意図から外れる（専用ロジック追加）
- ❌ `SetDrawingEffect` は色（アルファ含む）のみ制御可能で、位置・スケール等は制御不可

### Option C: グリフ＝データ配列（エンティティ分割なし）

**方針**: typewriter 方式を拡張。グリフ情報は配列として管理し、1つのエンティティが全テキストを描画。

- **データフロー**:
  - `GlyphArray { items: Vec<GlyphState { opacity, position, ... }> }` コンポーネント
  - 描画時: 配列をイテレートし、カスタム描画ロジックで各文字を描画

**トレードオフ**:
- ✅ 最小限のエンティティ数
- ✅ 描画最適化が容易（バッチ処理）
- ❌ ECS パターンを活用できない（配列内部のインデックス管理が必要）
- ❌ dola統合が「配列インデックス→変数」の独自設計になる
- ❌ 既存のレイアウト・描画パイプラインを活用できない
- ❌ テキストという特殊ドメインに閉じたソリューション

---

## 6. 子仕様別 工数・リスク評価

| 子仕様 | 工数 | リスク | 根拠 | v3.2-r1比較 |
|--------|------|--------|------|-------------|
| **balloon01-core** | M (3–7日) | Low | 既存 `Window` + `WindowPos` パターン踏襲。配置アルゴリズムは新規だが技術的に明確 | — |
| **balloon02-content** | **L (1–2週)** | **Medium** | グリフエンティティ spawn パイプラインが中核。`HitTestTextPosition` で座標取得の道筋は明確だが、グリフ描画方式（G25）とライフサイクル管理（G26）の設計が必要。ビューポート/スクロールも新規 | リスク微減（道筋明確化） |
| **balloon03-link** | M (3–7日) | Medium | `HitTestPoint` APIラップが新規。グリフエンティティのポインタイベント活用可能性あり。縦書き精度要検証 | — |
| **balloon04-choice** | S (1–3日) | Low | balloon01-core パターン再利用。flexbox縦並び対応済み | — |
| **balloon05-text-effects** | **M→L (5日–1.5週)** | **Medium** | **v3.2-r1の L(High) から改善**。グリフエンティティ方式により G17（文字単位opacity）が解決済み、G20（エフェクト定義）が簡素化。残課題は dola↔wintf 接続（G18）と dola→ECSプロパティ・バインディング（G5）。dola 統合層の設計は依然として重要だが、バインディング対象がECSプロパティに標準化されたことで設計の見通しが改善 | **工数↓リスク↓** |
| **balloon06-ruby** (P1) | L (1–2週) | High | DirectWriteルビAPI + 縦書きルビ配置。P1のためクリティカルパス外 | — |
| **balloon-reference-skin** | XS (0.5–1日) | Low | 単色背景・D2D1角丸矩形・しっぽ座標JSON定義のみ | — |

### 全体工数: L–XL (4–7週)

> **v3.2-r1比較**: 5–8週 → **4–7週**。主因: balloon05-text-effects のリスク低減（G17解決、G20簡素化）。ただし balloon02-content のグリフ spawn パイプライン（G4 具体化、G25, G26 新規）は依然として中核的な設計課題。

---

## 7. 設計フェーズへの申し送り事項

### 7.1 推奨アプローチ

**Option A（グリフ＝フルエンティティ）を推奨**。理由：

1. **「分割→再生に帰結」の設計意図に最も忠実**: グリフがエンティティになれば、テキストアニメーションは「エンティティの描画コマンド再生+プロパティ制御」に帰結する
2. **既存パイプラインの完全活用**: `Visual.opacity`, `Arrangement.offset`, `composite_render_system`, `Changed<T>` 検出がすべてそのまま適用
3. **G17（文字単位opacity）の消滅**: 最も深刻なギャップの1つが既存インフラで解決
4. **dola統合の標準化**: 「dola→エンティティプロパティ・バインディング」はバルーン以外のwintf機能（UI要素のアニメーション等）にも再利用可能
5. **移動エフェクトの自然な実現**: `Arrangement.offset` 制御でスライドイン等が可能（Option B/C では困難）

### 7.2 クリティカルパス

```
balloon01-core (M, Low)
  ├── balloon-reference-skin (XS, Low) ← 検証用スキン（クリティカルパス外）
  ├── balloon02-content (L, Medium) ← グリフエンティティ spawn が設計の鍵
  │     ├── balloon03-link (M, Medium)
  │     ├── balloon05-text-effects (M-L, Medium) ← dola統合（改善済み）
  │     └── balloon06-ruby (L, High) [P1]
  └── balloon04-choice (S, Low) ← 並行開発可能
```

**ボトルネック**: balloon01-core → balloon02-content → balloon05-text-effects。特に **dola↔wintf 接続（G18）** の早期着手が重要。ただし balloon05-text-effects のリスクは v3.2-r1 から Low→Medium に改善。

### 7.3 設計フェーズでの決定事項

| # | 決定事項 | 関連要件 | 優先度 |
|---|---------|---------|--------|
| D1 | `BalloonAnchor` の ECS 表現（Relation vs コンポーネント内 Entity 参照） | Req 1, 3 | 高 |
| D2 | 配置システムのスケジュール位置（PreLayout? Update?） | Req 3 | 中 |
| D3 | **グリフエンティティの描画方式**（per-char IDWriteTextLayout / カスタム IDWriteTextRenderer / DrawGlyphRun） | Req 6 AC2 | **最高** |
| D4 | **dola→ECSプロパティ・バインディングのインターフェース設計** | Req 7 AC4, Req 12 | **最高** |
| D5 | 濁点・半濁点の結合文字判定ロジック（Unicode解析 vs DirectWriteクラスタ情報活用） | Req 7 AC2 | 高 |
| D6 | スクロールコンテナの描画方式（D2D1 `PushAxisAlignedClip` / `PushLayer` vs オフスクリーン） | Req 8 | 中 |
| D7 | **dola_bridge のECSリソース設計**（`DolaRuntime` のライフサイクル管理、subscribe方式） | Req 12 | **最高** |
| D8 | **グリフエンティティのライフサイクル管理**（テキスト変更時の spawn/despawn/再利用戦略） | Req 6, 7 | **高** |
| D9 | キーボードナビゲーションの実装方式（フォーカスシステム要否） | Req 10 AC5 | 中 |
| D10 | ルビの実装方式（DirectWrite ネイティブ vs 手動配置）(P1) | Req 13 | 低 |

### 7.4 リサーチ項目

| # | 項目 | 理由 | 優先度 |
|---|------|------|--------|
| R1 | **グリフエンティティの per-char 描画方式の性能検証**（100文字時の `CreateCommandList` × N のスループット） | グリフ=フルエンティティ方式のボトルネック候補 | **最高** |
| R2 | **dola `DolaRuntime` のECSリソース化パターン** | `update(dt)` の呼び出し頻度、`subscribe` のイベントモデルがbevy_ecsスケジュールにどう統合されるか | **最高** |
| R3 | **dola変数→ECSプロパティの対応付け方式** | `AnimationVariableDef::Integer { typewriter }` の設計意図を踏まえた、`Visual.opacity` / `Arrangement.offset` バインディング | **最高** |
| R4 | **`HitTestTextPosition` の縦書き精度検証** | 縦書き時に各グリフの矩形が正しく取得できるか。グリフエンティティの `Arrangement` 設定の信頼性に直結 | **高** |
| R5 | `IDWriteTextLayout::HitTestPoint` の精度（縦書き時） | リンクヒットテストの信頼性 | 高 |
| R6 | taffy 0.9 のスクロールコンテナ/overflow サポート状況 | taffyレベルでscrollが使えるか、D2Dクリッピングで独自実装が必要か | 中 |
| R7 | ULW合成モードでのクリッピング描画パフォーマンス | スクロール時の60fps維持がULWで可能か | 中 |
| R8 | `bevy_ecs` 0.18 の Relation API 安定性 | BalloonAnchor にRelation が使えるか | 低 |
| R9 | `IDWriteTextLayout1::SetPairKerning` / ルビ用 DirectWrite API の可用性 (P1) | windows-rs クレートでの API 提供状況 | 低 |

---

## 8. 非機能要件ギャップ

| NFR | 現状 | ギャップ |
|-----|------|---------|
| **NFR-1: パフォーマンス** | ULW全面再描画。Typewriterは変更時のみ再描画 | **グリフエンティティ方式**: N × `CreateCommandList` コストが未検証（R1）。ダーティグリフのみ再描画で軽減見込み。bevy_ecs の Table ストレージで 200 エンティティは問題なし。スクロール 60fps は ULW で要検証 (R7) |
| **NFR-2: 互換性** | DPI対応済み (`Monitor.dpi`)。Win10 1803+ ターゲット | DirectWriteルビAPI (P1) のWin10 1803互換性を確認要 (R9) |
| **NFR-3: ECS統合** | 既存パターン確立済み | ✅ **グリフ=フルエンティティ方式は ECS 統合に最も適合**。`Visual`, `Arrangement`, `ChildOf`, `Changed<T>` 等の既存 ECS 機構をすべて活用。**dola統合のECSアーキテクチャが最優先決定事項** (D4, D7, R2, R3) |

---

## 9. 全体準備レベルサマリ

| 子仕様 | 準備レベル | 主要ギャップ | 既存資産活用度 | v3.2-r1比較 |
|--------|-----------|-------------|--------------|-------------|
| **balloon01-core** | 🟡 **60%** | 自動配置・追従の新規実装。ユーティリティは堅固 | 高 | — |
| **balloon02-content** | 🟠 **40%** | **グリフ spawn パイプライン**が中核。`HitTestTextPosition` で座標取得の道筋あり。スクロール/ビューポート新規 | 中 | **35%→40%**（道筋明確化） |
| **balloon03-link** | 🟠 **30%** | テキストヒットテスト(`HitTestPoint`)、リンクイベント、ホバーFB全て新規 | 低〜中 | — |
| **balloon04-choice** | 🟡 **40%** | ボタンウィジェット新規。イベントシステム再利用可。キーボードNav不足 | 中 | — |
| **balloon05-text-effects** | 🟠 **30%** | **wintf↔dola未接続**。ただし G17 解決 + G20 簡素化によりエフェクト実装の見通し改善 | 中 | **15%→30%** |
| **balloon06-ruby** (P1) | 🔴 **10%** | DirectWriteルビAPI未ラップ。縦書きルビ配置全て新規 | 低 | — |
| **balloon-reference-skin** | 🟢 **(XS)** | 全て新規だが規模XS（単色背景・角丸矩形・しっぽ定義のみ） | — | — |

### クリティカルリスクTOP 3

1. **🔴 dola統合 (G18, G19, G5)**: Cargo.tomlレベルで未接続。ECSリソース化、スケジュール統合、**エンティティプロパティ・バインディング**の設計が必要。ただしグリフエンティティ方式によりバインディング対象が標準化（`Visual.opacity`, `Arrangement.offset`）されたため、**設計の見通しは改善**。最優先リサーチ対象
2. **🟠 グリフ spawn パイプライン (G4, G25, G26)**: テキスト→グリフエンティティ群への変換が中核。`HitTestTextPosition` で座標取得可能だが、per-char 描画方式（D3）とライフサイクル管理（D8）の設計が必要。性能検証（R1）がゲート
3. **🟡 縦書き精度 (R4, R5)**: `HitTestTextPosition` / `HitTestPoint` の縦書き時の精度が未検証。グリフエンティティの `Arrangement` 配置精度とリンクヒットテスト精度に直結
