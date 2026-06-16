# W4b-V: wintf ヒットテスト・計測 × 脆弱性レビュー

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- 観点: V（脆弱性レビュー）。基準: design.md「Security Considerations」（unsafe 境界・整数変換の切り捨て/オーバーフロー・Win32/COM ハンドルのリーク/二重解放・外部入力の検証欠如・panic 経路 DoS を点検し、**挙動を変えない範囲（内部チェック・debug_assert・安全な型置換）の対策のみ投入**。API シグネチャ/エラー応答を変える対策は proposals.md へ）。CellExecutor V 規則（R2.3/R2.4）、観点順序 T→S→V（R2.7、W4b-T/W4b-S 完了済みの回帰検知器上で実行）。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations、CellExecutor 観点別規則（V）、提案記録様式、セル断片様式。
- 領域（W4b、W4a-T.md 確定の分担）: `crates/wintf/src/ecs/layout/` のうち `hit_test/`（mod.rs + tests.rs/tests_ex.rs）・`hit_region/`（mod.rs + tests.rs）・`metrics.rs`・`rect.rs`・`systems/monitor_systems.rs`・`systems/window_pos_systems.rs`。W4a 分担（taffy/arrangement/box_style/dimension・mod.rs の LayoutRoot・taffy/arrangement systems）には一切触れていない。
- 起点: W4b-S 適用後のクリーンなワークツリー（HEAD = `fc529bb`、ベースライン 1446 passed / 0 failed）。

## 点検手法

境界内 6 ファイル（+ in-source tests 3 ファイル）を grep（複数パターン）＋全文精読で走査した。

- panic 経路: `unwrap()` / `expect(` / `panic!` / `unreachable!` / `unimplemented!` / `todo!` / 配列・スライス添字 `[i]`/`[idx]`/`[offset]`
- 整数境界: `as i32` / `as u32` / `as f32` / `as usize`（切り捨て・符号反転・飽和）・整数乗算（`*`）のオーバーフロー・ゼロ除算（`/`）・`checked_`/`saturating_`/`wrapping_`
- モニタ境界: monitor 系・window_pos systems のモニタ追加/削除/解像度変更/DPI 変更時の退化（ゼロ枚・負座標・退化矩形）

到達可能性判定のため、`AlphaMask::is_hit`（widget/bitmap_source/alpha_mask.rs:67、**境界外**）の範囲チェックと `Monitor::physical_size`/`top_left`（window/monitor.rs:142/149、**境界外**、読み取りのみ）の戻り値型も確認し、座標変換の飽和キャストが下流で吸収される安全鎖を実証した。

## 発見した脆弱性候補と判定

### 1. panic 経路 — 大半が現状安全。1 件に挙動非破壊の不変条件 debug_assert を適用

境界内の `unwrap()`/`expect()`/`panic!` の出現はプロダクション経路に**ゼロ**（テスト内 `.unwrap()` を除く）。`unreachable!` 1 箇所・配列/スライス添字 数箇所を個別判定した。

- **`hit_test/mod.rs:322` `HitTestMode::None => unreachable!()`** — 直前の早期 return（mod.rs:306-308、`if mode == HitTestMode::None { return RegionHit::Miss; }`）により `match mode` 到達時点で `None` は構造的に除外される。リリースでも到達不能であり DoS panic 経路ではない。**現状安全（対策不要）。** コメント追記も検討したが、早期 return が直上にあり自己文書化されているため churn 回避で見送り。
- **`hit_region/mod.rs:225` `self.index_map[index]`** — 添字 `index = (pixel_y * width + pixel_x) as usize`。直前の範囲チェック（:221-223、`pixel_x < width && pixel_y < height`）が `index < width*height` を保証し、構造体不変条件 `index_map.len() == width*height`（`from_image` の :188 `vec![0u8; width*height]` で確立）と併せて添字は常に範囲内。**範囲チェックは width*height 上限を保証するが、index_map 長がそれと一致する不変条件はチェックの外側にある**ため、手構築の `ColorMapData`（フィールドは private だが in-source テストは直接構築する）で寸法と index_map 長が不整合だと OOB panic に至り得る。→ **挙動非破壊の debug_assert を適用**（下記）。リリース挙動は不変、デバッグでのみ不整合構築を検出。
- **`hit_region/mod.rs:230` `region_names.get((region_id - 1) as usize)`** — `region_id == 0` を :226 でチェック済みのため `region_id - 1` のアンダーフローなし、`.get()` は範囲外で `None` を返す全域関数。**現状安全**（W4b-T の `test_color_map_data_hit_test_id_out_of_range_names` で特性化済み）。
- **`hit_region/mod.rs:478-479` `vertices[i]` / `vertices[j]`** — `i ∈ 0..n`、`j` は `n-1` 初期化後に `j = i` 更新で常に `0..n`。`n < 3` は :470-472 で早期 return。**現状安全**（W4b-T の closing_edge テストで特性化済み）。
- **`hit_region/mod.rs:193-195/199 `buffer[offset/+1/+2]` / `index_map[i]`** — WIC デコード経路（`from_image`）。`offset = i*4`、`i ∈ 0..pixel_count`、`buffer.len() = stride*height = width*4*height ≥ 4*pixel_count` のため `offset+2 < buffer.len()`。構造的に安全だが実 WIC/COM 初期化が必要でユニット到達不能（P51 同根）。**安全鎖は確立**。なお確保段階の乗算オーバーフローは別所見（→ P53）。

### 2. 整数境界 — 座標変換の飽和キャストは安全。確保段階の乗算オーバーフロー 1 件を提案化

- **座標変換 `f32 as u32`（hit_test:245-246/369-370、hit_region:371-372）** — Rust の浮動小数→整数キャストは**飽和的**（負値→0、範囲超過→u32::MAX、NaN→0、UB なし）。変換後のピクセル座標は下流の範囲チェック（`AlphaMask::is_hit` の `x >= width → false`、`ColorMapData::hit_test` の `pixel >= width → None`）で吸収される。極値・負値・非有限の正規化座標がパニックせず縮退する安全挙動は**未特性化だった** → **特性化テスト 5 件を追加**（下記）。**現状安全。**
- **座標変換 `f32 as i32`（window_pos_systems:63-64/83-84）** — 飽和キャスト。size 経路は `width <= 0.0 || height <= 0.0` ガード（:52）で非正値を除外（NaN は `NaN <= 0.0` が false で通過するが `NaN.ceil() as i32 == 0` に縮退、panic なし）。供給元は GlobalArrangement（taffy 計算結果＝内部 ECS 値、外部ファイル流入経路なし）。NaN/inf 縮退は W4a-V が同型で「内部値・外部入力なし」と判定済みのため**二重記録せず現状安全**と判定。
- **座標変換 `i32 as f32`（monitor_systems:62-67/119、hit_test:477-478/543-544、window_pos:154-155）** — i32→f32 は実用範囲（モニタ座標・ウィンドウ座標）で無損失。供給元は `GetSystemMetrics`/`EnumDisplayMonitors`（Win32、システム制御）・WindowPos（内部）。**現状安全。**
- **ゼロ除算（hit_region:493 `(y - yj) / (yi - yj)`）** — ガード条件 `(yi > y) != (yj > y)` が `yi != yj` を含意し除数は非ゼロ。→ **不変条件コメントを追記**（挙動不変）。他の除算は座標変換の `bounds_width`/`bounds_height` だが、いずれも `<= 0.0` ガード後に実行（hit_test:236/363/394）。**現状安全。**
- **確保段階の u32 乗算オーバーフロー（hit_region `from_image`:179/180/187）→ P53（提案化）** — 外部 PNG の幅・高さ `(u32, u32)` で `width*4`・`stride*height`・`width*height` を計算。巨大寸法（例 65536×65536）で u32 を超え、**デバッグでは桁あふれ panic（外部ファイル由来 DoS 経路）・リリースでは黙ってラップ**して過小確保となる。対策（checked_mul / usize 昇格＋新エラー応答）は `from_image` の戻り値（`windows::core::Result` への新エラー経路追加、デバッグ panic→Err 化）という外部観測可能な挙動変更を伴うため、R2.4/R5.2 に従い**本ループでは実装せず P53 に記録**。実利用画像は小サイズで現状実害は未発現。

### 3. モニタ構成変更時の境界条件 — 現状安全（対策不要）

`systems/monitor_systems.rs` を精読。`initialize_layout_root`・`update_monitor_layout_system`・`detect_display_change_system` のいずれも:

- **ゼロ枚モニタ**: `enumerate_monitors()` が空 Vec を返すと `for monitor in monitors` ループが実行されず、LayoutRoot のみ生成して正常終了（panic なし）。
- **追加/削除/解像度変更**: `detect_display_change_system` は handle→entity の HashMap で差分処理し、`commands.spawn`/`despawn`・`taffy_res.create_node`/`remove_node` の失敗はすべて `if let Err(e) = ... { error!(...) }` でログのみ（panic なし）。`layout_root.single()` は `let Ok(...) else` で不在時 warn＋早期 return。
- **負座標・退化矩形**: モニタ矩形（RECT, i32）から `physical_size`/`top_left` で算出する f32 はそのまま `Dimension::Px`/`LengthPercentageAuto::Px` に格納されるのみで、本ファイル内に添字・除算・unwrap なし。退化（width 0 等）でも taffy 側に渡るだけで panic 経路なし。

`get_virtual_desktop_bounds`（`GetSystemMetrics`）・`enumerate_monitors`（実 Win32 列挙）はデバイス依存でユニット決定不能（W4b-T 所見2 と同じ環境制約）。`update_monitor_layout_system` の純粋ロジックは W4b-T で合成 Monitor により特性化済み。**現状安全（対策不要）。**

### 4. metrics.rs / rect.rs / window_pos_systems.rs — 現状安全（対策不要）

- **metrics.rs**: `LayoutScale`/`Opacity`/`TextLayoutMetrics` はすべて f32 フィールドの POD。`validate()` は `== 0.0` / 範囲チェックで warn ログを出すのみ（panic なし・値不変）。`clamped()` は `f32::clamp`（NaN 素通りだが panic なし）。整数キャスト・添字・unsafe・除算なし。**現状安全。**
- **rect.rs**: `D2DRectExt` 全メソッドは f32 加減算・min/max のみ。`validate()` は `#[cfg(debug_assertions)]` 付き `debug_assert!`（既存・退化矩形 left==right は不正でない）。`transform_rect_axis_aligned` は Matrix3x2 の f32 演算のみ。整数キャスト・添字・除算・unsafe なし。**現状安全。**
- **window_pos_systems.rs**: 整数キャストは所見2で判定済み（飽和・ガード後）。`window_pos.position?` 相当は `let Some(...) else { continue }`、CW_USEDEFAULT スキップあり。添字・unwrap・除算なし。**現状安全。**

## 適用した挙動非破壊対策（2 ファイル・3 箇所）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `hit_region/mod.rs` | `ColorMapData::hit_test`（:225 直前） | `debug_assert!(index < self.index_map.len(), ...)` ＋不変条件コメント | debug_assert（内部不変条件） | リリースで compile-out（挙動不変）。デバッグでは全 well-formed `ColorMapData`（`index_map.len() == width*height`）で常に真＝発火せず、手構築の寸法/長さ不整合のみ検出。R2.3「挙動を変えない内部チェック」に該当。 |
| `hit_region/mod.rs` | `point_in_polygon`（:489 交点計算直前） | 除数 `(yi - yj)` 非ゼロの構造的根拠を明記する不変条件コメント | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。ガード条件がゼロ除算を構造的に排除する根拠を W4a-V の SAFETY 注記方針と整合的に明文化。 |
| `hit_region/tests.rs` | 末尾（in-source `#[cfg(test)]`） | 整数境界・極値座標の特性化テスト 5 件＋ヘルパ 1 | 特性化/回帰テスト（S9 命名準拠） | 飽和キャスト安全鎖（極値→u32::MAX→範囲外→None、負→0、NaN→0、+inf→範囲外）を固定。W4b-T 未カバーの危険境界（極値座標）を特性化。 |

### 追加した特性化テスト一覧（`hit_region/tests.rs`、in-source 5 件＋ヘルパ）

- `make_fill_color_map_2x2`（ヘルパ・非テスト）— 2x2 全画素 id=1 のカラーマップ
- `test_color_map_extreme_positive_rel_saturates_to_none` — rel=1e10 / f32::MAX → u32::MAX 飽和 → 範囲外 → None（パニックなし）
- `test_color_map_negative_rel_saturates_to_zero_pixel` — rel=-5.0 → 0 飽和 → (0,0) 範囲内 → "fill"
- `test_color_map_nan_rel_does_not_panic` — NaN → 0 → (0,0) → "fill"、+inf → u32::MAX 飽和 → 範囲外 → None（パニックなし）
- `test_shapes_extreme_and_nonfinite_rel_do_not_panic` — Shapes 分岐の極大/負/NaN 座標 → 比較 false 化 → None（添字等の危険操作なし）
- `test_shapes_zero_extent_rect_via_inclusive_boundary` — 極小（正）幅矩形（build 通過）の境界包含点ヒットの安全性

## proposals.md へ回した候補（P53〜）

- **P53**: `ColorMapData::from_image` の画像寸法に対する整数オーバーフロー検証の欠如（外部 PNG 由来 u32 乗算）。kind: 挙動変更を伴う脆弱性対策。`from_image` の戻り値（新エラー応答・デバッグ panic→Err 化）という外部観測可能な挙動変更を伴うため記録のみ。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- P51（BitmapSourceResource テスト用コンストラクタ）: AlphaMask 座標変換本体の単体到達に必要な構成は本セルでも未充足（COM/WIC 依存）。P53 の境界特性化も P51 整備が前提。再記録せず参照に留めた。
- W4a-V の NaN/inf 縮退判定（layout 境界に外部入力の流入経路なし）: window_pos の `f32 as i32` NaN 縮退に同様に適用し、二重記録せず現状安全と判定。

## verification (S2)

- BEFORE: 親検証済みベースライン（W4b-S 直後 = 1446 passed / 0 failed、HEAD `fc529bb`、クリーンワークツリー）を信頼し省略。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、19.23s）。
  - `cargo test --workspace` → **1451 passed / 0 failed**（per-binary 集計: passed 合計 1451・failed 合計 0・ignored 32）。ベースライン 1446 から **+5 = 追加した特性化テスト 5 件と一致**（削除ゼロ）。
  - 反復検証: `--lib ecs::layout::hit_region` 39 passed（34 既存＋5 新規）、`--lib ecs::layout` 81 passed、`--test layout` 170 passed、すべて 0 failed。
- 全 5 件が初回実行で合格（特性化テスト＝GREEN by construction。下記 RED 代替を参照）。

## clippy（S3・記録のみ・非ブロッカー）

`cargo clippy -p wintf` の W4b 境界内 span を抽出。**いずれも本セル編集に起因しない既存 lint**であり、本セルの編集（hit_region への debug_assert・コメント・テスト追加）は**新規 clippy 警告を一切導入していない**。

- `hit_region/mod.rs:190`: `from_image` の `for i in 0..pixel_count` ループ（needless_range_loop 系。**本セル未編集の WIC 経路**）。
- `window_pos_systems.rs:22/134`: bevy システム引数 `Query<...>` の `very complex type`（W4a-V が taffy_systems で記録したのと同系統。広域様式 refactor で churn のため見送り）。

S3 規定によりブロッカーとせず記録に留める（簡素化は S 観点の担当・P52 等に関連）。

## RED フェーズ代替の検証

追加 5 件はすべて既存の安全な飽和挙動の characterization のため RED は N/A（GREEN by construction）。期待値は実装と独立に Rust の浮動小数→整数飽和キャスト仕様（負→0・範囲超過→u32::MAX・NaN→0）と下流の範囲チェック（`ColorMapData::hit_test` の `pixel >= width → None`）から導出した。初回実行で 5 件全件が導出どおり一致し、飽和キャスト＋範囲チェックの安全鎖が現行実装を正確に固定していることを相互確認した。debug_assert も全 well-formed 構築で発火せず（既存 33+新規追加分の hit_region テストが緑のまま）、リリース挙動不変を S2 全量（1451=1446+5）で実証した。

## flaky

- `cue_performance_test::bench_pop_ready_empty_queue`（既知・負荷依存・**W4b 境界外** `tests/ecs`）: `cargo test --workspace` を別の cargo プロセスと競合させた回でのみ 1 件 FAILED（CPU 競合）。**単独実行では合格**（`cargo test -p wintf --test ecs cue_performance_test` 隔離再実行で 5 passed / 0 failed、`bench_pop_ready_empty_queue` 含む安定合格を確認）。設計のフレーキー判定規則（隔離で非再現・かつ当該セル境界外）に従いフレーキーとして通過。本セルの追加テストとは無関係。最終判定の単独 S2 全量（1451/0）には本フレーキーは含まれていない。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダなし）。本セルの変更は debug_assert 1・不変条件コメント 2 箇所・特性化テスト 5 件のみで、新たな unsafe・スタブ・TODO を導入していない。
- TODO/FIXME 残存なし（本セル追加分に TODO/FIXME なし）。
- 点検は境界内 6 ファイル（+ in-source tests 3）を grep＋精読で網羅。panic 経路（unreachable 1・添字 数箇所）を個別に到達可能性判定し、座標変換の飽和キャスト・モニタ構成変更境界・ゼロ除算をすべて判定。挙動非破壊対策が妥当な 1 件（index_map 不変条件 debug_assert）と特性化価値のある極値座標 5 件を適用、挙動変更を要する 1 件（from_image 乗算オーバーフロー）を P53 へ記録。
- テストは意味を持つ: 既存（1446 全量・34 hit_region）が回帰検知器として機能し、+5 の特性化テストが飽和キャスト安全鎖を固定。件数整合（1451=1446+5）でコメント/debug_assert の非破壊を実証。
- 境界遵守: 変更は `hit_region/mod.rs`・`hit_region/tests.rs`（いずれも W4b 境界内）＋ `proposals.md`（提案台帳）のみ。tasks.md 未更新・コミット未作成・W4a 分担ファイル/他領域/`vendors/`/機能spec文書への変更なし。
- 結論: 本境界は脆弱性耐性が高く、warranted な挙動非破壊対策は index_map 不変条件 debug_assert＋ゼロ除算根拠コメントの 2 箇所と極値座標特性化 5 件に限られた。確保段階の整数オーバーフロー（P53）は挙動変更を伴うため記録に留め、その他の panic 経路・整数境界・モニタ境界はすべて現状安全と判定して churn を回避した。
