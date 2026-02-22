# ギャップ分析レポート

| 項目 | 内容 |
|------|------|
| **対象仕様** | wintf-P0-balloon-system |
| **分析日** | 2026-02-22 |
| **対象バージョン** | requirements.md v3.1-draft |
| **前回分析** | v3.0（本レポートで全面更新） |

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
| グラフィックスコア | `ecs/graphics/core.rs` | `GraphicsCore`: D3D11, DXGI, D2D1Factory, DWrite, 共有DeviceContext一括管理 |
| グラフィックスコンポーネント | `ecs/graphics/components.rs` | `WindowGraphics`, `VisualGraphics`, `SurfaceGraphics`, `SurfaceGraphicsDirty` |
| Visual | `ecs/graphics/visual.rs` | `Visual` { is_visible, opacity, transform_origin }。on_addで`Arrangement`等連鎖挿入 |
| コマンドリスト | `ecs/graphics/command_list.rs` | `GraphicsCommandList` — `ID2D1CommandList` ラッパー |
| 合成 | `ecs/graphics/compositor.rs` | `WindowD3D11Compositor` — per-window合成リソース |
| レンダー | `ecs/graphics/compositor_systems/render.rs` | `composite_render_system` (z-order合成) + `ulw_present_system` (ULW転送) |
| ULWユーティリティ | `com/ulw.rs` | `transfer_to_hbitmap` + `present_layered_window` |
| タイプライター | `ecs/widget/text/typewriter.rs` | `Typewriter`, `TypewriterTalk`, `TypewriterLayoutCache` (📖参考実装) |
| タイプライターIR | `ecs/widget/text/typewriter_ir.rs` | Stage 1 `TypewriterToken`, Stage 2 `TimelineItem`, `TypewriterTimeline` |
| タイプライターレイアウト | `ecs/widget/text/typewriter_layout.rs` | `init_typewriter_layout`, `convert_to_timeline` (クラスタベース分解) |
| タイプライター描画 | `ecs/widget/text/typewriter_draw.rs` | `update_typewriters`, `draw_typewriters` (visible_cluster_count制御) |
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
| **`ChildOf(parent)` 階層** | 親子関係によるウィジェットツリー構築 | 全子仕様: 描画責務ごとにエンティティ分離 |
| **コマンドキュー** | `SetWindowPosCommand`: thread_localキューでバッチ処理 | Req 2: 位置追従のウィンドウ操作 |
| **描画パイプライン** | Widget→`GraphicsCommandList`→`composite_render_system`(z-order合成)→`ulw_present_system`(ULW転送) | 全描画責務の基本フロー |
| **2段階IR** | Stage 1 `TypewriterToken`→Stage 2 `TimelineItem`。外部/内部の責務分離 | balloon02-content: 同パターンを参考にグリフベースIR設計 |
| **ブラシ継承** | `Brush::Inherit`→`resolve_inherited_brushes`で親から解決 | balloon01-core: フレーム描画のスタイル継承 |
| **SparseSetハンドラ** | `EventHandler<T>` は SparseSet ストレージ（少数エンティティに最適化） | balloon03-link, balloon04-choice: イベントハンドラ |
| **ダーティビット** | `SurfaceGraphicsDirty`, `Changed<T>` でシステムトリガー | 全描画責務: 再描画の最適化 |

### 1.3 制約事項

- `on_add` フック内では `Commands` が使えない（`DeferredWorld` のみ）
- `SetWindowPosCommand` は tick 終了後にフラッシュされる（即時反映不可）
- ULW 合成モードでは `UpdateLayeredWindow` のたびに全面再描画が必要
- PointerState の `WheelDelta` は蓄積されるが、スクロールウィジェットが未実装
- **wintf ↔ dola 依存なし**: `Cargo.toml` レベルで接続されていない。統合層の設計が必要
- Typewriter の描画方式: `visible_cluster_count` でオン/オフのみ。文字単位の不透明度制御は未実装
- `HitTestPoint`（座標→文字位置）API未ラップ。`HitTestTextPosition`（文字位置→座標）のみ実装済み

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

---

## 2. 要件−資産マッピング

### 子仕様 1: balloon01-core（DR-1: フレーム描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 1: ウィンドウ生成** | 1 | `Window` + `on_window_add` フック | バルーン専用コンポーネント（`BalloonWindow`）未定義。シェル↔バルーン ECS関係なし | Missing |
| | 2 | `Window` は複数生成可能 | キャラクター単位の多重バルーン管理ロジックなし | Missing |
| | 3 | `CompositionMode::ULW` で透過ウィンドウ生成済み | ✅ 既存パターンで対応可能（モックで実証済み） | — |
| | 4 | `on_remove` フック基盤あり | バルーン固有のクリーンアップロジック未定義 | Missing |
| | 5 | bevy_ecs でエンティティ管理済み | バルーン専用マーカー/関係コンポーネント未定義 | Missing |
| **Req 2: 配置制御** | 1 | モック: `BALLOON_OFFSET_X=335` ハードコード | 自動配置アルゴリズム未実装 | Missing |
| | 2 | なし | 配置方向（上/下/左/右）指定機能なし | Missing |
| | 3 | モック: `OnDrag` ハンドラで手動 `SetWindowPosCommand` | ECSシステムとしての自動追従未実装 | Missing |
| | 4 | `Monitor.work_area` でデスクトップ領域取得可能 | 自動反転ロジック未実装 | Missing |
| | 5 | なし | オフセット距離設定コンポーネント未定義 | Missing |
| **Req 3: 表示制御** | 1 | `WindowPos.show_window` / `hide_window` フラグ | ✅ 既存機構で対応可能 | — |
| | 2 | `WindowPos.zorder` (TopMost 等) | ✅ 前面表示は既存 ZOrder で可能 | — |
| | 3 | `WindowPos.hide_window` | ✅ ウィンドウ非表示+エンティティ保持は既存で可能 | — |
| | 4 | `WindowPos.size` + `BoxStyle` | ✅ サイズ設定可能 | — |

### 子仕様 2: balloon02-content（DR-2+DR-3: ビューポート+テキスト基本描画）

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 4: コンテンツ領域** | 1 | `BoxStyle` + `LayoutRoot` 階層 | コンテンツ領域コンポーネント未定義 | Missing |
| | 2 | `BoxStyle` に margin/padding あり | ✅ 既存BoxStyleで設定可能 | — |
| | 3 | `BoxStyle` の content-based sizing | コンテンツ→ウィンドウへのサイズ自動調整ロジック未実装 | Missing |
| | 4 | `BoxStyle` に max_width/max_height | ✅ taffy の制約で設定可能 | — |
| **Req 5: テキスト描画実装** | 1 | `TextDirection` (縦横4方向), `DWriteFactoryExt` | 📖 typewriter P0のDirectWrite実装を参考に新規実装が必要 | Missing |
| | 2 | `TypewriterTimeline` (Stage 2 IRでクラスタベース分解) | **グリフ単位分割→配列生成が未実装**。typewriterは `visible_cluster_count` による全文再描画方式 | Missing |
| | 3 | **dola↔wintf接続なし** (Cargo.toml未接続) | **グリフ配列→アニメーションマッピング構造が完全未実装** | Missing |
| | 4 | `TypewriterTalk` (`visible_cluster_count` 増加による逐次表示) | 📖 参考にはなるが、グリフベース方式での文字単位表示制御は新規実装 | Constraint |
| | 5 | なし | **濁点・半濁点ウェイト調整が未実装** | Missing |
| | 6 | `TypewriterToken::Wait` (固定待機) | **さくらスクリプト的ウェイト挿入マーカー未実装** | Missing |
| | 7 | `Typewriter { font_family, font_size }` | 📖 参考。新規実装でのスタイル設定はフォント・サイズ・色の設計が必要 | Constraint |
| **Req 6: スクロール** | 1 | なし | スクロールコンテナウィジェット未実装 | Missing |
| | 2 | なし | テキスト描画進行追従のスクロール制御未実装 | Missing |
| | 3 | `WheelDelta` 取得可能 | ホイール→スクロール変換ロジック未実装 | Missing |
| | 4 | なし | ページ送り機構未実装 | Missing |

### 子仕様 3: balloon03-link（DR-4: リンク描画）**[P0]**

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 7: クリッカブルテキスト** | 1 | `HitRegionMap` (rect/polygon) | テキスト位置→ヒットリージョン自動生成が未実装 | Missing |
| | 2 | `Phase<T>` イベントシステム (完備) | リンクイベント型（`LinkClicked { action }` 等）未定義 | Missing |
| | 3 | なし | リンク外観カスタマイズ機構未実装 | Missing |
| | 4 | `OnPointerEntered/Exited` (5種ハンドラ完備) | ホバー状態管理コンポーネント未定義（ハンドラフック自体は再利用可能） | Missing |
| | 5 | `DWriteTextLayoutExt::hit_test_text_position` (位置→座標) | **`HitTestPoint` (座標→位置) が未ラップ**。リンクヒットテストに必須 | Missing |
| | 6 | `TypewriterToken` (Text/Wait/FireEvent のみ) | **リンク用トークン variant 未定義** | Missing |

### 子仕様 4: balloon04-choice（DR-5: 選択肢UI描画）**[P0]**

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 8: 選択肢バルーン** | 1 | `Window` + balloon01-core の `BalloonAnchor` | ChoiceBalloon 専用コンポーネント未定義 | Missing |
| | 2 | `BoxStyle` (flexbox column対応) | ✅ flexbox 縦並び可能 | — |
| | 3 | `Phase<T>` イベントシステム | 選択肢イベント型（`ChoiceSelected { index, id }` 等）未定義 | Missing |
| | 4 | `OnPointerEntered/Exited` | ホバー状態ウィジェット未実装（ボタン相当ウィジェットがない） | Missing |
| | 5 | WM_KEYDOWN (ESC のみ) | **キーボードナビゲーション基盤未実装**（上下キー・Enter） | Missing |

### 子仕様 5: balloon05-text-effects（DR-6: テキストエフェクト描画）**[P0]**

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 9: 文字単位エフェクト** | 1 | Typewriter描画: `visible_cluster_count` でオン/オフのみ | **文字単位フェードイン（不透明度0→1）未実装** | Missing |
| | 2 | なし | 文字単位フェードアウト未実装 | Missing |
| | 3 | なし | エフェクトタイミング・継続時間の文字ごと管理未実装 | Missing |
| | 4 | なし | 複数エフェクト同時適用エンジン未実装 | Missing |
| | 5 | なし | エフェクト適用中の描画領域管理（クリッピング拡張）未実装 | Missing |
| **Req 10: dolaアニメーション統合** | 1 | `dola`: `DolaRuntime` + `compile_storyboard` 完全ランタイム | **wintf↔dola依存なし（Cargo.tomlレベルで未接続）** | Missing |
| | 2 | `dola::easing`: 30+種ネームドイージング + パラメトリック | イージング→テキストエフェクトへの適用機構未実装 | Missing |
| | 3 | `dola::playback`: `PlaybackState` (Idle/Playing/Paused/Completed/Cancelled) | dola↔bevy_ecs スケジュール統合未実装 | Missing |
| | 4 | `dola`: `InterruptionPolicy` (Cancel/Conclude/Trim/Compress/Never) | アニメーション中断制御のECS統合未実装 | Missing |
| | 5 | `dola::variable`: `AnimationVariableDef::Integer { typewriter }` フィールドあり | **文字単位変数バインディング（グリフ配列→dola変数）未設計** | Missing |

### 子仕様 6: balloon06-ruby（DR-7: ルビ描画）**[P1]**

| 要件 | AC# | 既存資産 | ギャップ | 分類 |
|------|------|---------|---------|------|
| **Req 11: ルビ表示** | 1 | `IDWriteTextLayout` 基本ラップのみ | ルビ用 DirectWrite API 未ラップ（`IDWriteTextLayout1` 以降） | Missing |
| | 2 | なし | 横書きルビ配置ロジック未実装 | Missing |
| | 3 | なし | 縦書きルビ配置ロジック未実装 | Missing |
| | 4 | なし | ルビフォントサイズ自動調整未実装 | Missing |
| | 5 | IR: `TypewriterToken` (Text/Wait/FireEvent のみ) | **ルビ用トークン variant 未定義** | Missing |

---

## 3. ギャップサマリ

### 3.1 主要ギャップ一覧

| # | ギャップ | 影響範囲 | 深刻度 | v3.0からの変化 |
|---|---------|---------|--------|---------------|
| G1 | バルーン専用ECSコンポーネント群が未定義 | balloon01-core 全体 | 高 | — |
| G2 | シェル↔バルーンのECSリレーション機構なし | Req 1, 2 | 高 | — |
| G3 | 自動配置・追従・反転アルゴリズム未実装 | Req 2 全AC | 高 | — |
| G4 | **グリフ単位分割→配列生成パイプライン未実装** | Req 5 AC2 | **高** | 🆕 v3.1で追加 |
| G5 | **グリフ配列→dola/アニメーションマッピング構造未実装** | Req 5 AC3 | **高** | 🆕 v3.1で追加 |
| G6 | **濁点・半濁点ウェイト調整未実装** | Req 5 AC5 | 中 | 🆕 v3.1で追加 |
| G7 | **さくらスクリプト的ウェイト挿入未実装** | Req 5 AC6 | 中 | 🆕 v3.1で追加 |
| G8 | ビューポート/クリッピング未実装 | Req 4, 6 | 中 | — |
| G9 | スクロールコンテナウィジェット未実装 | Req 6 全AC | 中 | — |
| G10 | コンテンツ→ウィンドウ サイズ自動調整未実装 | Req 4 AC3 | 中 | — |
| G11 | `HitTestPoint`（座標→文字位置）API未ラップ | Req 7 AC5 | 中 | — |
| G12 | リンクイベント型・ホバー状態管理未定義 | Req 7 全AC | 中 | — |
| G13 | TypewriterToken にリンク variant なし | Req 7 AC6 | 中 | — |
| G14 | ChoiceBalloon専用コンポーネント未定義 | Req 8 AC1 | 中 | — |
| G15 | ボタン相当ウィジェット未実装 | Req 8 AC4 | 中 | — |
| G16 | キーボードナビゲーション基盤不足（ESCのみ） | Req 8 AC5 | 中 | — |
| G17 | 文字単位不透明度制御未実装（visible_cluster_countはオン/オフのみ） | Req 9 全AC | 高 | — |
| G18 | **wintf↔dola Cargo.toml依存なし（完全未接続）** | Req 10 全AC | **最高** | 深刻度↑ |
| G19 | DolaRuntime↔bevy_ecsスケジュール統合未実装 | Req 10 AC3 | 高 | — |
| G20 | 文字単位エフェクト定義フォーマット未設計 | Req 9, 10 | 高 | — |
| G21 | エフェクトタイミング管理（テキスト表示タイミング↔dola開始タイミング同期）未実装 | Req 10 AC5 | 高 | — |
| G22 | フレーム描画ウィジェット（枠線+角丸+しっぽ）未実装 | DR-1 | 中 | — |
| G23 | DirectWriteルビAPI未ラップ (P1) | Req 11 全AC | 中 | P1のため低優先 |
| G24 | TypewriterToken にルビ variant なし (P1) | Req 11 AC5 | 中 | P1のため低優先 |

### 3.2 v3.0→v3.1 ギャップ変化サマリ

| 変更点 | 影響 |
|--------|------|
| **typewriter P0 → 📖参考実装** | Req 5 の全ACが「既存活用」から「新規実装（参考あり）」に変化。G4-G7が新規発生 |
| **グリフベースアーキテクチャ追加** | G4（グリフ分割）、G5（マッピング構造）が最重要ギャップとして新規追加。balloon02-contentの工数↑ |
| **dola依存の重要度↑** | G18（wintf↔dola未接続）の深刻度が「最高」に。統合層設計がクリティカルパスに |
| **P0/P1分離** | ルビ（G23, G24）がP1に移動し、P0のクリティカルパスから除外 |
| **子仕様名の連番化** | balloon01-core～balloon06-ruby への名称変更。ギャップマッピングの対応関係は維持 |

### 3.3 既存資産の活用可能ポイント

| 資産 | 活用先 | 活用方法 |
|------|--------|---------|
| `Window` + `on_window_add` hookチェーン | Req 1: バルーンウィンドウ生成 | `BalloonWindow` on_add で同パターン踏襲 |
| `CompositionMode::ULW` | Req 1 AC3: 透過ウィンドウ | そのまま利用可能（モック実証済み） |
| `WindowPos` + `SetWindowPosCommand` | Req 2, 3: 位置・表示制御 | コマンドキューパターン再利用 |
| `Monitor.work_area` | Req 2 AC4: デスクトップ境界判定 | 値取得済み。反転判定ロジックのみ新規 |
| `BoxStyle` (margin/padding/flex) | Req 4: コンテンツ領域レイアウト | そのまま利用可能 |
| `TextDirection` (縦横4方向) | Req 5 AC1: 縦書き・横書き | DirectWrite統合パターンを参考 |
| `TypewriterTimeline` (2段階IR) | Req 5: テキスト描画 | 📖 IRパターンを参考に新グリフベースIR設計 |
| `DWriteFactoryExt` / `DWriteTextLayoutExt` | Req 5: DirectWrite操作 | 📖 API利用方法を参考 |
| `WheelDelta` | Req 6 AC3: ホイールスクロール | 値取得済み。消費ロジックのみ新規 |
| `Phase<T>` (Tunnel/Bubble) イベントシステム | Req 7, 8: イベント発火 | イベント型の追加のみ |
| `OnPointerEntered/Exited` (全5種ハンドラ) | Req 7 AC4, Req 8 AC4: ホバー | ポインタイベントフック再利用 |
| `HitRegionMap` (rect/polygon/colormap) | Req 7: リンクヒットテスト | テキスト座標→領域生成が新規 |
| `composite_render_system` (opacity対応) | Req 9: 文字単位エフェクト | opacity合成パターンを参考。文字単位への適用は新規 |
| `DolaRuntime` (load→start→update→subscribe) | Req 10: dola統合 | ランタイムは完成。wintf統合層が新規 |
| `AnimationVariableDef::Integer { typewriter }` | Req 10: dola×テキスト連携 | 設計時点でTypewriter統合が想定されていた |
| `EasingFunction` (30+種) | Req 10 AC2: イージング | そのまま利用可能 |
| `InterruptionPolicy` (5種) | Req 10 AC4: 中断制御 | そのまま利用可能 |
| モック実装 (`areka/src/main.rs`) | 全子仕様: 構築パターン | エンティティ構築パターンを参考 |

---

## 4. 実装アプローチ選択肢

### Option A: 既存コンポーネント拡張

**方針**: 既存 Typewriter / Window コンポーネントを直接拡張。テキスト描画は Typewriter にグリフ分割機能を追加。

- **対象ファイル**: `typewriter.rs`, `typewriter_ir.rs`, `typewriter_draw.rs`, `window/components.rs`
- **互換性**: `Typewriter` への破壊的変更。既存のTypewriter利用箇所（モック含む）に影響
- **メンテナンス性**: Typewriter の責務が膨張（基本描画+グリフ分割+アニメーションマッピング+ウェイト調整）

**トレードオフ**:
- ✅ 新規ファイル最少
- ✅ 既存のDirectWrite統合コードを直接利用
- ❌ Typewriter の単一責任原則に違反（参考実装が本体に合体）
- ❌ 既存Typewriter利用箇所への影響が不明確
- ❌ グリフベースアーキテクチャの要件がTypewriterの設計意図と合わない

### Option B: 新規コンポーネント作成（推奨）

**方針**: バルーン専用コンポーネント群を新規モジュールとして作成。テキスト描画はTypewriterとは独立した新規実装（参考のみ）。dola統合層を新設。

- **新規モジュール構成**:
  - `ecs/widget/balloon/` — バルーンコア（`BalloonWindow`, `ChoiceBalloon`; DR-1）
  - `ecs/widget/balloon/placement.rs` — 配置アルゴリズム（`BalloonAnchor`, `BalloonPlacement`）
  - `ecs/widget/balloon/content.rs` — コンテンツ領域管理（DR-2: ビューポート+スクロール）
  - `ecs/widget/text/balloon_text.rs` — **グリフベーステキスト描画**（DR-3: 新規実装）
  - `ecs/widget/text/glyph_pipeline.rs` — **グリフ分割+配列生成パイプライン**
  - `ecs/widget/text/link.rs` — リンクウィジェット（DR-4）
  - `ecs/widget/choice.rs` — 選択肢項目ウィジェット（DR-5）
  - `ecs/widget/text/text_effects.rs` — 文字単位エフェクト（DR-6）
  - `ecs/dola_bridge/` — **dola↔bevy_ecs統合層**（DolaRuntimeのECSリソース化、スケジュール統合）
  - `com/dwrite_ext.rs` — `HitTestPoint` + ルビ用DirectWrite拡張

- **統合ポイント**:
  - `BalloonWindow` の `on_add` フックで `Window` + `WindowStyle` を自動挿入
  - `BalloonAnchor { target: Entity, direction, offset }` でシェル↔バルーン関係をECSで表現
  - バルーン配置システムを `PreLayout` スケジュールに登録
  - dola_bridge: `DolaRuntime` を `Resource` として管理、`Update` スケジュールで `update(dt)` 呼び出し
  - グリフ分割結果 → dola変数 → 描画パラメータ のデータフロー

- **責任境界**:
  - `BalloonWindow` = ライフサイクル管理（生成・破棄）
  - `BalloonAnchor` = アンカー対象 + 配置パラメータ
  - `BalloonPlacement` = 計算結果（実配置方向・座標キャッシュ）
  - `glyph_pipeline` = テキスト→グリフ配列変換（3フェーズの第1段階）
  - `dola_bridge` = dola↔ECSの唯一の接続点
  - 配置システム = `BalloonAnchor` + `Monitor` → `WindowPos` 書き換え

**トレードオフ**:
- ✅ 明確な責任分離（描画責務=モジュール境界=子仕様境界）
- ✅ テスト容易性（各コンポーネント単体テスト可能）
- ✅ 既存 Typewriter に侵入的変更なし
- ✅ グリフベースアーキテクチャを最初から設計可能
- ✅ dola統合層が独立しており、他のwintf機能からもdolaを利用可能
- ❌ ファイル数増加
- ❌ DirectWriteラッパーの一部コードが参照実装からの書き直しになる

### Option C: ハイブリッドアプローチ

**方針**: balloon01-core/balloon04-choice は新規コンポーネント（Option B）、balloon02-content のテキスト描画は既存 Typewriter IR の拡張 + グリフ分割レイヤーを上に追加。

- **フェーズ分割**:
  1. balloon01-core: 新規 `ecs/widget/balloon/` モジュール
  2. balloon02-content: **TypewriterToken/TimelineItem** のIR形式を拡張 + **グリフ分割レイヤー**を別ファイルで追加
  3. balloon03-link: TypewriterToken に `Link` variant 追加 + 新規ヒットテスト
  4. balloon04-choice: 完全新規ウィジェット
  5. balloon05-text-effects: 新規 `text_effects.rs` + dola統合

**トレードオフ**:
- ✅ Typewriter の2段階IRパターンを直接継承（実績あるパターン）
- ✅ DirectWriteラッパーの書き直しが最小限
- ❌ Typewriter IRの拡張がTypewriter仕様の責任範囲を曖昧にする
- ❌ 「参考実装」と宣言したのに実質的に依存してしまう矛盾
- ❌ グリフベースアーキテクチャの3フェーズ設計が既存IR構造に制約される

---

## 5. 子仕様別 工数・リスク評価

| 子仕様 | 工数 | リスク | 根拠 | v3.0比較 |
|--------|------|--------|------|---------|
| **balloon01-core** | M (3–7日) | Low | 既存 `Window` + `WindowPos` パターン踏襲。配置アルゴリズムは新規だが技術的に明確。フレーム描画ウィジェット(DR-1)は `GraphicsCommandList` パターンに乗るのみ | 変化なし |
| **balloon02-content** | **L (1–2週)** | **Medium** | **v3.0のM→Lに変更**。typewriterの直接統合ではなく新規実装が必要。グリフ単位分割パイプラインの設計・実装が加わる。ビューポート/スクロールも新規。ただしtypewriter📖とDirectWriteラッパーが参考資料として活用可能 | 工数↑リスク↑ |
| **balloon03-link** | M (3–7日) | Medium | `HitTestPoint` APIラップが新規。ただしイベントシステムは完備。テキストヒットテストの縦書き精度が要検証 | — |
| **balloon04-choice** | S (1–3日) | Low | balloon01-core パターン再利用。flexbox縦並び対応済み。キーボード操作のスコープ次第でMに変動 | 変化なし |
| **balloon05-text-effects** | **L (1–2週)** | **High** | **v3.0のM→Lに変更**。**最大リスク**: wintf↔dola接続が未確立（Cargo.tomlレベルで未接続）。DolaRuntimeのECSリソース化、グリフ配列→dola変数バインディング、タイミング同期の全てが新規設計。dola側の `Integer { typewriter }` フィールドが設計意図の手がかりだが、具体的な統合パターンは未検証 | 工数↑リスク↑ |
| **balloon06-ruby** (P1) | L (1–2週) | High | DirectWriteルビAPI + 縦書きルビ配置。P1のためクリティカルパス外 | P1移動 |

### 全体工数: XL (5–8週)

> **v3.0比較**: 3–5週 → **5–8週**に増加。主因はグリフベースアーキテクチャ新規実装（balloon02-content）とdola統合層の設計負荷（balloon05-text-effects）。

---

## 6. 設計フェーズへの申し送り事項

### 6.1 推奨アプローチ

**Option B（新規コンポーネント作成）を推奨**。理由：

1. **描画責務=モジュール境界**の原則と一致し、子仕様の独立開発に最適
2. **グリフベースアーキテクチャを白紙から設計**できる（既存Typewriter IRの制約を受けない）
3. **dola統合層（`dola_bridge`）の独立化**により、バルーン以外のwintf機能でもdolaを利用できる拡張性
4. 既存 `Typewriter` への侵入的変更を回避し、📖参考実装の役割を明確に維持
5. `on_add` hookパターン、`Phase<T>` イベントシステム等の確立済みパターンはそのまま踏襲可能

### 6.2 クリティカルパス

```
balloon01-core (M, Low)
  ├── balloon02-content (L, Medium) ← グリフベースアーキテクチャが設計の鍵
  │     ├── balloon03-link (M, Medium)
  │     ├── balloon05-text-effects (L, High) ← dola統合が最大リスク
  │     └── balloon06-ruby (L, High) [P1]
  └── balloon04-choice (S, Low) ← 並行開発可能
```

**ボトルネック**: balloon02-content → balloon05-text-effects のパスが最長かつ最高リスク。特に **dola↔wintf統合層の設計** は早期に着手すべき。

### 6.3 設計フェーズでの決定事項

| # | 決定事項 | 関連要件 | 優先度 |
|---|---------|---------|--------|
| D1 | `BalloonAnchor` の ECS 表現（Relation vs コンポーネント内 Entity 参照） | Req 1, 2 | 高 |
| D2 | 配置システムのスケジュール位置（PreLayout? Update?） | Req 2 | 中 |
| D3 | **グリフ分割パイプラインのIR設計**（入力形式、グリフ+表示位置情報の構造体定義） | Req 5 AC2 | **最高** |
| D4 | **グリフ配列→dola/アニメーション管理へのマッピングインターフェース** | Req 5 AC3 | **最高** |
| D5 | 濁点・半濁点の結合文字判定ロジック（Unicode解析 vs DirectWriteクラスタ情報活用） | Req 5 AC5 | 高 |
| D6 | スクロールコンテナの描画方式（D2D1 `PushAxisAlignedClip` / `PushLayer` vs オフスクリーン） | Req 6 | 中 |
| D7 | **dola_bridge のECSリソース設計**（`DolaRuntime` のライフサイクル管理、subscribe方式） | Req 10 | **最高** |
| D8 | 文字単位エフェクトのデータモデル（グリフごとの状態: opacity, position, color, etc.） | Req 9 | 高 |
| D9 | キーボードナビゲーションの実装方式（フォーカスシステム要否） | Req 8 AC5 | 中 |
| D10 | ルビの実装方式（DirectWrite ネイティブ vs 手動配置）(P1) | Req 11 | 低 |

### 6.4 リサーチ項目

| # | 項目 | 理由 | 優先度 |
|---|------|------|--------|
| R1 | **DirectWrite `GetClusterMetrics` → グリフ単位分割の精度検証** | グリフベースアーキテクチャの実現可能性に直結。特に縦書き時のクラスタ→グリフ分解精度 | **最高** |
| R2 | **dola `DolaRuntime` のECSリソース化パターン** | `update(dt)` の呼び出し頻度、`subscribe` のイベントモデルがbevy_ecsのスケジュールにどう統合されるか | **最高** |
| R3 | **dola変数とグリフ配列の対応付け方式** | `AnimationVariableDef::Integer { typewriter }` の設計意図を踏まえた、グリフごとのopacity/positionバインディング | **最高** |
| R4 | `IDWriteTextLayout::HitTestPoint` の精度（縦書き時） | リンクヒットテストの信頼性 | 高 |
| R5 | taffy 0.9 のスクロールコンテナ/overflow サポート状況 | taffyレベルでscrollが使えるか、D2Dクリッピングで独自実装が必要か | 中 |
| R6 | ULW合成モードでのクリッピング描画パフォーマンス | スクロール時の60fps維持がULWで可能か | 中 |
| R7 | `bevy_ecs` 0.18 の Relation API 安定性 | BalloonAnchor にRelation が使えるか | 低 |
| R8 | `IDWriteTextLayout1::SetPairKerning` / ルビ用 DirectWrite API の可用性 (P1) | windows-rs クレートでの API 提供状況 | 低 |

---

## 7. 非機能要件ギャップ

| NFR | 現状 | ギャップ |
|-----|------|---------|
| **NFR-1: パフォーマンス** | ULW全面再描画。Typewriterは変更時のみ再描画。**グリフ単位分割は全レイアウト再計算を回避する設計** | スクロール時の60fps維持がULWで可能か要検証 (R6)。グリフ分割のオーバーヘッド（初回コスト vs 逐次再描画コスト）の評価が必要 |
| **NFR-2: 互換性** | DPI対応済み (`Monitor.dpi`)。Win10 1803+ ターゲット | DirectWriteルビAPI (P1) のWin10 1803互換性を確認要 (R8) |
| **NFR-3: ECS統合** | 既存パターン確立済み | ✅ Option B で要件充足可能。**dola統合のECSアーキテクチャが最優先決定事項** (D7, R2, R3) |

---

## 8. 全体準備レベルサマリ

| 子仕様 | 準備レベル | 主要ギャップ | 既存資産活用度 | v3.0比較 |
|--------|-----------|-------------|--------------|---------|
| **balloon01-core** | 🟡 **60%** | 自動配置・追従の新規実装。ユーティリティは堅固 | 高 | — |
| **balloon02-content** | 🟠 **35%** | **グリフ分割パイプライン**全て新規。スクロール/ビューポート新規。typewriterは📖参考のみ | 中 | **55%→35%** |
| **balloon03-link** | 🟠 **30%** | テキストヒットテスト(`HitTestPoint`)、リンクイベント、ホバーFB全て新規 | 低〜中 | — |
| **balloon04-choice** | 🟡 **40%** | ボタンウィジェット新規。イベントシステム再利用可。キーボードNav不足 | 中 | — |
| **balloon05-text-effects** | 🔴 **15%** | **wintf↔dola未接続**。文字単位エフェクト機構全て新規 | 低 | — |
| **balloon06-ruby** (P1) | 🔴 **10%** | DirectWriteルビAPI未ラップ。縦書きルビ配置全て新規 | 低 | P1移動 |

### クリティカルリスクTOP 3

1. **🔴 dola統合 (G18, G19, G20, G21)**: Cargo.tomlレベルで未接続。ECSリソース化、スケジュール統合、文字単位変数バインディングの全設計が必要。**最優先リサーチ対象**
2. **🟠 グリフベースアーキテクチャ (G4, G5)**: 要件の中核。DirectWriteのクラスタ情報→グリフ配列変換の精度検証が必要。設計決定D3, D4が他の子仕様に波及
3. **🟡 テキストヒットテスト (G11)**: `HitTestPoint` APIのラップ自体は明確だが、縦書き時の精度が未検証。balloon03-linkのブロッカー
