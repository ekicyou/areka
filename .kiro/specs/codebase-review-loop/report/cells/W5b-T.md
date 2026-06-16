# W5b-T: wintf 図形・画像・ブラシ × テスト網羅性

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W5b-T（領域 W5b「wintf 図形・画像・ブラシ」 × 観点 T「テスト網羅性」）
- 性質: 既存挙動の特性化テスト追加（挙動変更なし。R2.1, R5.1）。**W5b 領域の最初のセル**（先行 W5b 断片なし）。対象3モジュール群のモジュール×テスト対応表をゼロから作成した。
- requirements: 2.1, 2.5, 2.7, 2.8, 4.1, 5.1
- design: プロジェクト・プロファイル S2/S9、レビュー観点列 T、CellExecutor 観点別規則（T）、セル断片様式、提案記録様式
- 参考: `bitmap_source/` の分離テストパターン（実 WIC/D2D を要する部分と純粋ロジックを分離してテスト可能にする設計）を shapes/・brushes.rs にも応用。

## 対象ファイル一覧（W5b = `widget/{shapes,bitmap_source}/` + `brushes.rs`）

**shapes/**（2ファイル）
- `mod.rs`（re-export のみ、2 LOC）
- `rectangle.rs`（Rectangle / Color エイリアス / colors 非推奨モジュール / draw_rectangles システム、284 LOC）

**bitmap_source/**（8ファイル）
- `mod.rs`（re-export のみ、22 LOC）
- `alpha_mask.rs`（AlphaMask: from_pbgra32 / is_hit / width / height、純粋ビットパック、本体89 LOC + in-source tests）
- `bitmap_source.rs`（BitmapSource コンポーネント + on_add/on_remove フック、148 LOC）
- `resource.rs`（BitmapSourceResource / BitmapSourceGraphics、100 LOC）
- `systems.rs`（resolve_path / load_bitmap_source / draw_bitmap_sources / drain_task_pool_commands / generate_alpha_mask_system、424 LOC）
- `task_pool.rs`（WintfTaskPool: spawn / drain、109 LOC）
- `wic_core.rs`（WicCore: WIC ファクトリ保持、37 LOC）
- `tests.rs`（分離 in-source テスト、本セルで拡張）

**brushes.rs**（Brush enum / Brushes / BrushInherit / 既定色定数、本体163 LOC + in-source tests）

合計 約1,690 LOC（design.md W5b 概算 1,680 と整合。境界 = 指定3モジュール群のみ。他 widget サブモジュール（text/）には一切触れていない）。

## モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象（主要 関数/型） | デバイス依存 | 既存テスト所在・件数 | 追加 | 所見 |
|------------|------|------|-----------|------|------|
| `shapes/mod.rs` | re-export のみ | なし | — | 0件 | テスト対象なし |
| `shapes/rectangle.rs` | `Rectangle::new/default`、`Color` エイリアス、`colors::*`（非推奨6定数）、`TRANSPARENT_COLOR`、`draw_rectangles` システムの色解決フォールバック | `draw_rectangles` は**全面 D2D**。`Rectangle` 構築・色定数・色解決式は**デバイス非依存** | **なし（in-source 0件）** | **4件** | **最大の空白**: 図形ウィジェットの in-source テストがゼロだった。`Rectangle::new/default` 等価・`TRANSPARENT_COLOR` 透明性・`Color` 型エイリアス互換・非推奨 `colors` と `Brush` 定数の RGBA 一致を固定 |
| `bitmap_source/mod.rs` | re-export のみ | なし | — | 0件 | テスト対象なし |
| `bitmap_source/alpha_mask.rs` | `AlphaMask::{from_pbgra32, is_hit, width, height}`（ビットパック1bit/px・MSBファースト・閾値128） | **なし（純粋計算）** | in-source 8件（透明/不透明/閾値127/128/範囲外/混在/wide/サイズアクセサ） | **5件** | 空白: width=0/height=0 退化、**パディング付き stride**（行頭 `y*stride` 計算）、**バイト境界跨ぎ**（x=7→bit0/x=8→bit7）、最終有効ピクセル判定が未固定だった |
| `bitmap_source/bitmap_source.rs` | `BitmapSource::new`、`on_bitmap_source_add`（Visual/Graphics/HitTest 自動挿入 + 非同期ロード起動）/`on_bitmap_source_remove` | フックは World 操作だが、WicCore/TaskPool 取得後の**実 WIC ロード**へ分岐 | `tests.rs` 2件（new/from_string）+ `tests/widget/bitmap_source_integration_test.rs`（実 WIC、5件） | 0件 | `BitmapSource::new` は既存2件で充足。on_add フックの自動挿入は実デバイス統合経路（WicCore 不在時 warn → return）で純粋抽出不能。所見1 |
| `bitmap_source/resource.rs` | `BitmapSourceResource::{new,source,alpha_mask,set_alpha_mask}`、`BitmapSourceGraphics::{new,bitmap,set_bitmap,invalidate,is_valid}` + Send/Sync | `BitmapSourceResource::new` は**実 IWICBitmapSource 必須**。`BitmapSourceGraphics` は Option 保持で純粋 | `tests.rs` 4件（Resource Send/Sync・Graphics new/invalidate/Send-Sync） | 0件 | `BitmapSourceGraphics` は既存4件で充足。`BitmapSourceResource` の `alpha_mask()` getter（αマスク変換ロジック到達）は実 WIC 構築が前提で未到達 → **既存 P51**（テスト用コンストラクタ）が該当。所見2 |
| `bitmap_source/systems.rs` | **`resolve_path`（パス解決）**、`load_bitmap_source`（WIC デコード）、`draw_bitmap_sources`（D2D 描画）、`drain_task_pool_commands`、`generate_alpha_mask_system`（寸法計算→from_pbgra32） | `resolve_path` は**デバイス非依存**。load/draw/generate は**全面 WIC/D2D** | `tests/widget/bitmap_source_integration_test.rs`（load_bitmap_source 実 WIC、6件） | **3件** | **空白**: `resolve_path`（絶対パスそのまま返却 / 相対パスを exe ディレクトリ基準で join）が**完全に未テストだった**。`generate_alpha_mask_system` の寸法 u32 乗算オーバーフロー（systems.rs:402-403）はデバイス依存・挙動変更要 → **P55**。所見3,4 |
| `bitmap_source/task_pool.rs` | `WintfTaskPool::{new,spawn,drain_and_apply,drain_commands,send_command,is_empty}` | なし（mpsc + TaskPool、bevy World） | `tests.rs` 4件（creation/drain_empty/command_send_receive/is_empty 経由） | 0件 | 既存4件で送受信・ドレインを充足。`is_empty` は `#[cfg(test)]` の簡易実装（常に true、コメントで明記）で観測価値低。重複追加せず（所見5） |
| `bitmap_source/wic_core.rs` | `WicCore::{new,factory}` + Send/Sync | **実 WIC ファクトリ（CoCreateInstance）必須** | `tests.rs` 4件（creation/factory_access/clone/send_sync、全て `with_com_initialized`） | 0件 | 実 COM/WIC 必須だが既存4件が COM 初期化込みで網羅済み。純粋ロジックなし |
| `brushes.rs` | `Brush`（enum/Default/as_color/is_inherit/6定数）、`Brushes`（Default/with_*/Clone/PartialEq）、`BrushInherit`、`DEFAULT_FOREGROUND/BACKGROUND` | **なし（純粋データ）** | in-source 8件（as_color 2・is_inherit・constants・Brushes default/with_foreground/with_background/with_colors） | **6件** | 空白: `Brush::default()`=Inherit、`Brush` PartialEq の色成分/バリアント識別、**既定色定数**（draw フォールバックが依存）、`Brushes` Clone+PartialEq（Changed 検出前提）、`BrushInherit::default`、**色解決フォールバック式**（draw_rectangles と等価）が未固定だった |

追加テスト合計 **18件**（rectangle 4・alpha_mask 5・bitmap_source/tests.rs 3・brushes 6）。**プロダクションコードの変更なし**（R5.1 充足）。新規テストファイルなし（既存 in-source `mod tests` 3ファイル + 分離 `tests.rs` への追記）。

## 追加したテスト一覧（ファイル・テスト名・狙い）

**`crates/wintf/src/ecs/widget/brushes.rs`（in-source `mod tests`, 6件）**
- `test_brush_default_is_inherit` — `Brush::default()` が `Inherit`（as_color=None）
- `test_brush_partial_eq_distinguishes_color_and_variant` — Solid の色成分（α差含む）とバリアント差を PartialEq が識別
- `test_default_color_constants` — `DEFAULT_FOREGROUND`=黒(0,0,0,1)・`DEFAULT_BACKGROUND`=透明(0,0,0,0) の値固定（draw 系継承フォールバックの依存定数）
- `test_brushes_clone_and_eq` — `Brushes` の Clone 一致 + フィールド変更で不等（Changed 検出前提）
- `test_brush_inherit_marker_default` — `BrushInherit` マーカーの Default 構築
- `test_foreground_color_resolution_fallback` — `draw_rectangles` の `as_color().unwrap_or_else(DEFAULT_FOREGROUND)` 等価式: Solid はその色、Inherit は黒へフォールバック

**`crates/wintf/src/ecs/widget/shapes/rectangle.rs`（in-source `mod tests`, 4件・新規 mod）**
- `test_rectangle_new_equals_default` — `Rectangle::new()`/`Rectangle`/`default()` のユニット同一性
- `test_transparent_color_constant_is_fully_transparent` — 内部 `TRANSPARENT_COLOR`（描画前クリア用）が α=0
- `test_color_type_alias_is_d2d_color` — `Color` エイリアスが `D2D1_COLOR_F` として構築・代入互換
- `test_deprecated_colors_match_brush_constants` — 非推奨 `colors::*` 6定数が `Brush::*` 対応色と同一 RGBA（旧/新 API 色定義の一致。`#[allow(deprecated)]`）

**`crates/wintf/src/ecs/widget/bitmap_source/tests.rs`（分離 in-source, 3件）**
- `test_resolve_path_absolute_is_returned_unchanged` — 絶対パスは current_exe を参照せずそのまま返却
- `test_resolve_path_relative_is_joined_under_exe_dir` — 相対パスを `current_exe().parent()` 配下へ join（exe ディレクトリ起点の絶対パス化）
- `test_resolve_path_relative_preserves_subdirectories` — ネストした相対パス（a/b/c.png）のコンポーネント保持

**`crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs`（in-source `mod tests`, 5件）**
- `test_zero_width_produces_empty_mask` — width=0 退化（空マスク・全座標 false）
- `test_zero_height_produces_empty_mask` — height=0 退化（幅ありでも行なし・ヒットなし）
- `test_padded_stride_reads_correct_rows` — stride > width*4 のパディング行で `y*stride` 行頭計算が正しいこと
- `test_bit_packing_across_byte_boundary` — x=7（バイト0 bit0）/x=8（バイト1 bit7）の MSBファースト跨ぎ
- `test_is_hit_at_last_valid_pixel` — 最終有効ピクセル(width-1,height-1)のヒットと直近範囲外の非ヒット

## 除外したテスト

なし。対象3モジュール群の既存テスト（brushes in-source 8件・alpha_mask in-source 8件・bitmap_source/tests.rs 13件・統合 bitmap_source_integration_test 11件）には重複・死テスト（到達不能・常に真・対象消失）は検出されなかった。`task_pool.rs::is_empty` は `#[cfg(test)]` の簡易実装（常に true）だが、既存 `test_wintf_task_pool_creation` が「新規作成直後は空」という前提のもとで使用しており、トートロジーではない（プールの初期状態仕様の固定として機能）。過不足整理の結論: **不足のみ存在（18件で充足）、過剰なし**。

## テスト不能箇所・深掘り所見（R2.8）

1. **`on_bitmap_source_add` / `draw_bitmap_sources` / `draw_rectangles` は全面 GPU/WIC 依存** — (a) `draw_rectangles`/`draw_bitmap_sources` は `ID2D1DeviceContext::CreateCommandList`/`SetTarget`/`BeginDraw`/`EndDraw`/`CreateSolidColorBrush`/`FillRectangle`/`CreateBitmapFromWicBitmap`/`DrawBitmap` を直接呼び、実 `GraphicsCore`（D3D11+D2D デバイス）を要する。各システムは「色/リソース解決→描画記録→CommandList 挿入」の手続きで、抽出可能な純粋計算ブロックは**色解決フォールバックのみ**（これは brushes 側で特性化済み）。(b) `on_bitmap_source_add` は World 操作（Visual/Graphics/HitTest 自動挿入）部分は純粋だが、後半が `WicCore`/`WintfTaskPool` 取得 → 実 WIC 非同期ロードへ分岐し、自動挿入の検証は Label の前例（`tests/visual/widget_visual_auto_insert_test.rs`）と同型の実デバイス統合域。**ユニット到達不能は環境制約**でありコード改善余地ではないため提案化しない（既存の実デバイス統合テスト群＝tests/graphics・tests/widget が最終的な回帰検知器）。

2. **`BitmapSourceResource` の αマスク変換ロジックが実 WIC 構築に密結合（→ 既存 P51）** — `BitmapSourceResource::new(source: IWICBitmapSource)` が唯一の公開コンストラクタで実 WIC ソースを要求するため、`alpha_mask()` getter（および hit_test 統合層の screen→mask 座標変換）はデバイス非依存ロジックでありながら到達に COM/WIC 初期化が必要。これは **W4b-T が既に P51 として記録済み**（`BitmapSourceResource` にテスト専用コンストラクタを追加し、AlphaMask ヒットテスト変換を単体到達可能にする提案）。W5b-T では αマスクの**変換先**である `AlphaMask::from_pbgra32`/`is_hit` 単体を13件（既存8＋追加5）で固定済みのため、P51 実施時は生成側（WIC）と消費側（AlphaMask）が両端から保護される。本セルからの重複提案はしない。

3. **`resolve_path` の純粋性とテスト可能性** — `resolve_path` は WIC/D2D に一切依存せず、絶対パス分岐は完全に純粋、相対パス分岐は `std::env::current_exe()` の戻り値（テスト実行時は常に存在）に対する決定的 join である。bitmap_source の「純粋ロジック分離」設計の好例であり、本セルで3件追加して特性化した（絶対そのまま・相対 join・サブディレクトリ保持）。深掘りの結果、ロジックにバグ・前提誤りは検出されなかった（3件すべて初回実行で導出どおり一致）。

4. **`generate_alpha_mask_system` の寸法 u32 乗算オーバーフロー（→ P55）** — systems.rs:402-403 の `stride = width * 4` / `buffer_size = (stride * height) as usize` は WIC `get_size()` 由来の `(u32, u32)` 同士の乗算で、巨大画像（例 33000×33000）では u32 範囲を超え、デバッグでは panic（外部画像由来 DoS 経路）・リリースではラップして過小バッファ確保となる。これは **W4b-V の P53（`hit_region::ColorMapData::from_image` の同型オーバーフロー）と同一クラスの別箇所**（αマスク生成経路）。本経路は実 WIC/COM + 実画像デコードを要しユニット到達不能、かつ対策（checked_mul / usize 昇格 + スキップ/エラー）が panic→スキップという観測挙動変更を伴うため、R2.4/R5.2 に従い本ループでは実装せず **P55** に記録した。リポジトリ内実利用画像は 8x8〜16x16 のため現状実害は未発現。

5. **`task_pool.rs::is_empty` の `#[cfg(test)]` 簡易実装** — 「`try_iter()` は非破壊的でないためチャネル状態を確認できない」というコメント付きで常に true を返す。新規作成直後の空状態前提でのみ使われ（`test_wintf_task_pool_creation`）、汎用の空判定ではない。これを「正確な空判定」へ変える改善は (a) チャネル状態を覗く API の追加（公開面変更）または (b) カウンタ導入（プロダクション構造変更）を要し、テスト網羅性の範囲を超える。T セルでは現行挙動（新規作成直後 true）を既存テストが固定している事実を確認するに留め、提案化しない（観測価値の低い内部テストヘルパのため）。

## proposals へ回した候補

- **P55**: `generate_alpha_mask_system` の画像寸法に対する整数オーバーフロー検証の欠如（外部画像由来 u32 乗算。P53 と同一クラスの別箇所、統合実施推奨）

既存提案との関連（新規採番せず参照のみ）:
- **P51**（W4b-T）: `BitmapSourceResource` のテスト用コンストラクタ追加 — 所見2 のとおり W5b 域でも αマスク変換到達の障壁として該当。P51 実施で本セルの AlphaMask 特性化（消費側）と接続される。

## verification (S2)

- BEFORE: 親のベースライン（1468 passed / 0 failed・クリーンワークツリー）を信頼して流用（design のフェーズ0 ベースライン規定 + 親指示「BEFORE S2 は省略可」に従う）。なお触れたバイナリの BEFORE 内訳は git diff（追加 18 件・削除 0 件）と AFTER 実測の差分から逆算して検証した。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1486 passed / 0 failed**（全テストバイナリで failed=0、awk による全 `test result` 行の合算で実測）。
  - グローバル合計は 1468 → 1486（**+18**）。追加分はすべて wintf lib の in-source（`--lib`）テスト。
  - 触れたファイルの in-source 件数内訳（git diff の `#[test]` 実数と完全一致）:
    - `shapes/rectangle.rs`: **0 → 4（+4）**
    - `brushes.rs`: **8 → 14（+6）**
    - `bitmap_source/tests.rs`: **13 → 16（+3）**
    - `bitmap_source/alpha_mask.rs`: **8 → 13（+5）**
    - 合計 **+18**（git diff: 追加 `#[test]` 18・削除 0 で裏取り）
  - wintf lib バイナリ全体: AFTER 281 passed（W5b 追加 18 を含む）。
  - 反復検証: `cargo test -p wintf --lib widget::` で widget モジュール **72 passed / 0 failed**（既存54 + 追加18）。内訳 `widget::brushes` 14・`widget::shapes` 4・`widget::bitmap_source` 29。
  - 全18件が初回実行で合格（特性化テスト = GREEN by construction。後述 RED 代替を参照）。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W5b 境界外 `tests/ecs`）: `cargo test --workspace` の全量実行で `tests/ecs` は 79 passed / 0 failed と合格（隔離再実行不要）。確認のため隔離再実行 `cargo test -p wintf --test ecs cue_performance_test` も実施し 5 passed / 0 failed で安定合格。本セルの追加テストとは無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib` は既存警告 156 件を出力。**いずれも本セルの追加テスト由来ではない**。触れたファイルに紐づく警告4件はすべて**プロダクションコード既存**:
  - `alpha_mask.rs:35` / `alpha_mask.rs:73` — `manual_div_ceil`（`(width + 7) / 8` → `.div_ceil(8)`、`from_pbgra32`/`is_hit` 本体）
  - `brushes.rs:85` — `derivable_impls`（`impl Default for Brush` は手動だが derive 不可: Inherit を既定にするため正当。S 観点での要検討）
  - `rectangle.rs:128` — `type_complexity`（`draw_rectangles` の Query 型）
  - これらは S 観点（簡素化）の候補であり、追加テストコードによる新規警告の導入はゼロ。S3 規定によりブロッカーとせず記録に留める。

## RED フェーズ代替の検証

追加18件はすべて既存挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に各ソース仕様から導出した:
- **brushes**: `Brush::default`=Inherit（`impl Default`）、PartialEq の derive 比較（D2D1_COLOR_F の f32 成分比較）、`DEFAULT_FOREGROUND`=BLACK・`DEFAULT_BACKGROUND`=TRANSPARENT の定数定義、`Brushes` の derive Clone/PartialEq、color フォールバックは `draw_rectangles` のソース式 `as_color().unwrap_or_else(|| DEFAULT_FOREGROUND.as_color().unwrap())` を転記。
- **rectangle**: ユニット構造体の同一性、`TRANSPARENT_COLOR` 定数の α=0、`Color = D2D1_COLOR_F` 型エイリアス、`colors::*` と `Brush::*` の定数定義の RGBA 一致。
- **resolve_path**: `path.is_absolute()` 分岐（絶対はそのまま）/ else 分岐（`current_exe().parent().join(path)`）をソースから導出。
- **alpha_mask**: `row_bytes = ceil(width/8)`・`pixel_offset = y*stride + x*4`・α=`pixels[offset+3]`・閾値128・MSBファースト（`bit_index = 7 - x%8`）・範囲外 `x>=width || y>=height` を `from_pbgra32`/`is_hit` のソースから導出。

初回実行で18件全件が導出どおり一致し、バグ・前提誤りは検出されなかった（深掘りを要する初回失敗なし）。
