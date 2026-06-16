# W4b-T: wintf ヒットテスト・計測 × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W4b-T（領域 W4b「wintf ヒットテスト・計測」 × 観点 T「テスト網羅性」）
- 性質: 既存挙動の特性化テスト追加＋過不足整理（挙動変更なし。R2.1, R5.1）
- requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9、レビュー観点列 T、CellExecutor 観点別規則（T）、セル断片様式

## 対象ファイル一覧（W4b 分担）

W4a-T.md のファイル分担定義に従い、W4b 担当分のみを対象とした:

- `crates/wintf/src/ecs/layout/hit_test/`（`mod.rs` + in-source `tests.rs` / `tests_ex.rs`）
- `crates/wintf/src/ecs/layout/hit_region/`（`mod.rs` + in-source `tests.rs`）
- `crates/wintf/src/ecs/layout/metrics.rs`
- `crates/wintf/src/ecs/layout/rect.rs`
- `crates/wintf/src/ecs/layout/systems/monitor_systems.rs`
- `crates/wintf/src/ecs/layout/systems/window_pos_systems.rs`

W4a 分担ファイル（taffy/arrangement/box_style/dimension 系・mod.rs の LayoutRoot）には一切触れていない。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要関数/型） | 既存テスト所在・件数 | 追加 | 過不足整理の所見 |
|------------|------|-----------|------|------|
| `hit_test/mod.rs` (560 LOC) | `HitTestMode`/`HitTest`、`hit_test_entity`、`hit_test_entity_ex`、`hit_test`/`hit_test_ex`、`hit_test_in_window`/`_ex` | in-source `tests.rs` 17件 + `tests_ex.rs` 22件（合計39件）。Bounds/None/AlphaMask フォールバック/NamedRegions/合成α境界（128/255）/ツリー走査前面優先/ウィンドウ座標変換まで網羅 | **3件** | 空白: ① `hit_test_in_window_ex` の `position: None` 早期 return 経路（不在のみ既存）② NamedRegions の退化 bounds（幅0/高さ0）フォールバック分岐（mod.rs:394）が未検証だった。**AlphaMask 座標変換の幾何ロジック本体（screen→mask 正規化, mod.rs:231-249/350-377）は WIC 依存のため未テスト → 所見1/P51** |
| `hit_region/mod.rs` (505 LOC) | `point_in_polygon`（ray casting）、`HitRegionMap::hit_test_region`（Shapes/ColorMap）、`HitRegionMapBuilder`（検証）、`ColorMapData`（hit_test/width/height）、`HitRegionError` Display | in-source `tests.rs` 30件。凸/凹多角形・三角形・矩形境界・先勝ち・混在・ColorMap 各象限・非正方形・エラー Display を網羅 | **4件** | 空白: ① `ColorMapData::width()/height()` アクセサ直接検証なし ② `ColorMapData::hit_test` の region_id が region_names 範囲外（`get(id-1)`=None）の防御経路 ③ `point_in_polygon` の閉じ辺（最終頂点→始点, `j=n-1` 初期化）をまたぐ判定 ④ Shapes 退化 entity_size（幅0）でのゼロ除算/パニック非発生 |
| `metrics.rs` (93 LOC) | `LayoutScale`（validate/default）、`Opacity`（deprecated; validate/clamped/default）、`TextLayoutMetrics`（default/PartialEq） | `Opacity::clamped` のみ visual/component_test:171 で間接検証。`TextLayoutMetrics` 構築は widget/vertical_text_layout_test。**各 Default 値・validate() の警告経路は未検証だった** | **9件**（新規 `tests/layout/metrics_test.rs`） | `LayoutScale::default`（恒等 1.0,1.0）・`validate` の非ゼロ noop / ゼロ警告経路（x/y/両方）、`Opacity::default`（1.0）・`clamped` 範囲内外・`validate` が値を変更しないこと、`TextLayoutMetrics::default`（0,0）・PartialEq を固定 |
| `rect.rs` (208 LOC) | `D2DRectExt`（from_offset_size/width/height/offset/size/set_offset/set_size/set_left/top/right/bottom/contains/union/validate）、`transform_rect_axis_aligned` | `tests/layout/arrangement_bounds_test.rs` に 12件（from_offset_size/width/height/offset/size/set_offset/set_size/contains/union/validate panic 2件/transform 4件） | **5件**（arrangement_bounds_test へ追記） | 空白: 単独エッジセッター `set_left/set_top/set_right/set_bottom` 4件（set_offset/set_size のみ既存）と `validate()` の正常系（panic しない・退化矩形 left==right も「不正でない」）を固定 |
| `systems/monitor_systems.rs` (278 LOC) | `get_virtual_desktop_bounds`、`initialize_layout_root`、`update_monitor_layout_system`、`detect_display_change_system` | `tests/window/monitor_hierarchy_test.rs` 9件（initialize の singleton/列挙/階層、taffy 変換、detect_display_change のフラグ/構成維持、後方互換） | **2件**（monitor_hierarchy_test へ追記） | 空白: `update_monitor_layout_system`（`Changed<Monitor>` で BoxStyle.size/inset 再計算）が直接未検証だった。実 HMONITOR 不要のデバイス非依存ロジックのため合成 Monitor で特性化。`get_virtual_desktop_bounds`/`initialize_layout_root`/`detect_display_change_system` の実モニタ列挙部分はデバイス依存（所見2） |
| `systems/window_pos_systems.rs` (170 LOC) | `window_pos_sync_system`、`sync_window_arrangement_from_window_pos` | 4ファイルで厚く網羅: `feedback_loop_convergence_test`（DPI96/192・Changed フィルタ・CW_USEDEFAULT・None position・等値スキップ・収束）、`graphics_sync_test`（同期/無効 bounds スキップ/echo back）、`drag/window_dragging_filter_test`（ドラッグ中サイズのみ/位置スキップ/Without フィルタ）、`boxstyle_coordinate_separation_test` | **0件** | 全分岐（通常更新・ドラッグ中サイズのみ・無効 bounds・position None・CW_USEDEFAULT・等値）が既存で網羅済み。重複追加せず（過不足整理: 不足なし） |

追加テスト合計 **23件**（hit_test ex 3・hit_region 4・metrics 9・rect 5・monitor 2）。プロダクションコードの変更なし（R5.1 充足）。新規テストファイル1件（`tests/layout/metrics_test.rs`）+ 既存4ファイルへの追記 + 束ね役 `tests/layout.rs` への mod 1行追記。

### 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/layout/hit_test/tests_ex.rs`（in-source, 3件）**
- `test_hit_test_in_window_ex_position_none` — `window_pos.position?` の None 早期 return（None → None）
- `test_hit_test_entity_ex_named_regions_degenerate_bounds_width` — 幅0 bounds で contains 通過後 `bounds_width<=0` 分岐 → Hit(None)
- `test_hit_test_entity_ex_named_regions_degenerate_bounds_height` — 高さ0 bounds で `bounds_height<=0` 分岐 → Hit(None)

**`crates/wintf/src/ecs/layout/hit_region/tests.rs`（in-source, 4件）**
- `test_color_map_data_size_accessors` — `ColorMapData::width()/height()`
- `test_color_map_data_hit_test_id_out_of_range_names` — region_id > region_names.len() → None（`get(id-1)` 防御）
- `test_point_in_polygon_closing_edge` — 閉じ辺（最終頂点→始点）をまたぐ内外判定
- `test_shapes_hit_test_degenerate_entity_size` — 幅0 entity_size でゼロ除算/パニックなし → None

**`crates/wintf/tests/layout/metrics_test.rs`（新規, 9件）**
- `test_layout_scale_default_is_identity`、`test_layout_scale_validate_nonzero_is_noop`、`test_layout_scale_validate_zero_does_not_panic`
- `test_opacity_default_is_fully_opaque`、`test_opacity_clamped_within_range`、`test_opacity_clamped_out_of_range`、`test_opacity_validate_does_not_panic_or_mutate`
- `test_text_layout_metrics_default_is_zero`、`test_text_layout_metrics_partial_eq`

**`crates/wintf/tests/layout/arrangement_bounds_test.rs`（追記, 5件）**
- `test_d2drect_set_left`/`_set_top`/`_set_right`/`_set_bottom` — 各単独エッジセッター（他エッジ不変の副作用なし）
- `test_d2drect_validate_valid_does_not_panic` — validate 正常系（退化矩形 left==right を含む）

**`crates/wintf/tests/window/monitor_hierarchy_test.rs`（追記, 2件 + ヘルパ `make_test_monitor`）**
- `test_update_monitor_layout_recomputes_box_style` — Changed<Monitor> で size/inset が physical_size/top_left から再計算
- `test_update_monitor_layout_skips_unchanged` — 未変更 Monitor は再計算されない（Changed フィルタ）

## 除外したテスト

なし。本領域は設計の想定どおり「比較的テストが厚い」状態で、重複テスト・死テスト（到達不能・常に真・対象消失）は検出されなかった。`window_pos_systems` は4ファイルで網羅されているが各テストは異なる分岐・スケジュール連結シナリオを固定しており冗長ではない（除外対象なし）。過不足整理の結論: **不足のみ存在（23件で充足）、過剰なし**。

## テスト不能箇所・深掘り所見（R2.8）

1. **AlphaMask ヒットテストの座標変換幾何ロジックは WIC 依存（→ P51）** — `hit_test_entity` / `hit_test_entity_ex` の AlphaMask モードは、矩形通過後に screen→mask 正規化（`rel = (point - bounds.left)/bounds_width`）と mask 座標化（`(rel * mask.width()) as u32`）を行い `AlphaMask::is_hit` を呼ぶ（mod.rs:231-249, 350-377）。この座標変換ロジック自体は純粋だが、到達には `BitmapSourceResource` に `set_alpha_mask` 済みの実体が必要で、`BitmapSourceResource::new(source)` が実 `IWICBitmapSource` を要求する（COM/WIC 初期化が必要）。既存 in-source テストは「BitmapSourceResource なし/αマスク未生成」のフォールバック経路（return true）のみを固定し、変換本体は未到達。`AlphaMask::from_pbgra32`/`is_hit` 単体は bitmap_source/alpha_mask.rs の in-source 10件で網羅済みのため、ヒットテスト統合層での変換は二重テストに近いが、`rel→mask` の丸め（`as u32` 切り捨て）と bounds 原点減算の結合は未固定。layout ドメインテストに COM/WIC 依存を持ち込むのは過剰（karpathy 簡素性）と判断し本セルでは見送り、テスト用コンストラクタの提案を P51 に記録した。

2. **monitor_systems の実モニタ列挙部分はデバイス依存** — `get_virtual_desktop_bounds`（`GetSystemMetrics`）・`initialize_layout_root` / `detect_display_change_system` の `enumerate_monitors()` 呼び出しは実モニタ構成・実 Win32 API に依存し、ユニットで決定的に検証できない。既存テストは「実環境に≥1モニタが存在し構成が安定」前提で singleton 性・階層・フラグ遷移を固定済み。本セルでは純粋ロジックの `update_monitor_layout_system`（合成 Monitor で検証可能）の空白のみ埋めた。`detect_display_change_system` の追加/更新/削除の差分処理は実モニタのホットプラグが必要で深掘り不能（既存の「構成維持」テストで no-op 経路のみ固定）。提案化はしない（テスト不能は環境制約であり、コード側の改善余地ではないため）。

3. **`hit_test_entity`（非 ex 版）の AlphaMask 退化 bounds フォールバック** — mod.rs:236-238 の `bounds_width<=0` → return true も所見1と同じく WIC 依存で未到達。ex 版の NamedRegions 退化 bounds は HitRegionMap のみで到達できるため本セルで固定したが、AlphaMask 退化分岐は P51 の構成が整って初めて到達可能。

4. **`Opacity` は deprecated だが現役利用あり** — `metrics.rs` の `Opacity` は `#[deprecated(note="Use Visual.opacity instead")]` だが、`Visual::clamped_opacity` の等価性検証（visual/component_test）で参照され、R2.9 の「利用ゼロ」条件を満たさない。本 T セルでは挙動を特性化するに留め、削除可否判断は S/V セルおよび W3 系の Visual 移行状況に委ねる（本セルからの提案化はしない）。

## proposals へ回した候補

- **P51**: `BitmapSourceResource` のテスト用コンストラクタ追加（AlphaMask ヒットテスト変換ロジックの単体到達を可能にする）

## verification (S2)

- BEFORE: 親のベースライン（1423 passed / 0 failed）を信頼して流用（クリーンワークツリー）。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1446 passed / 0 failed**（+23 = 追加テスト数と一致、削除なし）。
  - 内訳確認: 新規テスト関数 23件（hit_test ex 3・hit_region 4・metrics 9・rect 5・monitor 2）。
  - 反復検証: `--test layout` 単体 170 passed / 0 failed、`--lib layout::hit` 76 passed、`--test window monitor_hierarchy` 11 passed、すべて 0 failed。
  - 全23件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。

## flaky

- `cue_performance_test::bench_pop_ready_empty_queue`（既知・負荷依存・W4b 境界外 `tests/ecs`）: `cargo test --workspace` を `cargo clippy --workspace` と同時並走させた回でのみ 1 件失敗（CPU 競合）。**単独実行ではすべて合格**（`cargo test -p wintf --test ecs cue_performance_test` 隔離再実行で 5 passed / 0 failed、`bench_pop_ready_empty_queue` 含む安定合格を確認）。設計のフレーキー判定規則（隔離で非再現・かつ当該セル境界外）に従いフレーキーとして通過。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy --workspace` は既存の警告約177件を出力（`too many arguments`、`if 文の collapse`、`doc list item indentation`、`duplicated attribute`、`Default 実装の推奨` 等）。**いずれも本セルの追加テスト由来ではなく既存プロダクションコードの警告**。本セルではテスト追加のみでプロダクションコード未変更のため新規警告の導入なし。S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当）。

## RED フェーズ代替の検証

追加23件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した: LayoutScale/Opacity の Default 値と validate の「警告のみ・値不変」契約、D2DRectExt 各セッターの「対象フィールドのみ更新」契約、ray casting の閉じ辺評価、退化 bounds のゼロ除算回避フォールバック、`update_monitor_layout_system` の physical_size/top_left→BoxStyle 写像。初回実行で23件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
