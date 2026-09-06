# 再導出台帳（タスク 1.1・要件 7.1／9.5）

作成日: 2026-09-05 / ブランチ `claude/areka-p0-emo-text-line-height-b69d6c`（HEAD `ac3d0f73`・`main` は `0b64d648`）

この台帳は、行送りの式を旧式 `ceil(font.height × 1.25)` から新式 `font.height + 行間 2` へ改めるにあたって、
期待値を計算で導き直す必要のあるファイルを 1 か所にまとめたものです。
以降のタスク 2.x／3.x は、design.md の表ではなく **この 1 ファイルだけ** を参照先とします。

行送りの対応表（新旧）:

| `font.height` | 旧 `ceil(h × 1.25)` | 新 `h + 2` |
|---|---|---|
| 10 | 13 | **12** |
| 12 | 15 | **14** |
| 20 | 25 | **22** |
| 28 | 35 | **30** |
| 40 | 50 | **42** |

---

## 1. 着手時の前提確認（要件 9.5）

### 1.1 `draw.rs` を触る進行中の spec が本ブランチだけであること

```
git log main..HEAD --oneline -- crates/areka-emo-text/src/draw.rs
```

出力は **空**（該当コミット 0 件・終了コード 0）。本ブランチの 12 コミットが触ったファイルにも
`crates/areka-emo-text/` 配下は 1 つも含まれていません（`git diff --name-only main..HEAD` で確認・
変更は spec 文書・`doc/ukadoc-coverage/`・`crates/areka-ghost`・`crates/ukadoc-survey` のみ）。

したがって **`draw.rs` を触るのは本仕様が唯一** です。`text-decoration-canon` は W13・未着手。

### 1.2 対象 6 ファイルの現行行数（1,000 行未満であること）

`wc -l` の実測値です。design.md「Modified Files」が括弧書きで挙げる現行行数とすべて一致しました。

| ファイル | 現行行数 | 1,000 行未満 | design の記載 | 見込み行数（design） |
|---|---|---|---|---|
| `crates/areka-emo-text/src/draw.rs` | 980 | ○ | 980 一致 | ≈ 985 |
| `crates/areka-emo-text/src/layout.rs` | 890 | ○ | 890 一致 | ≈ 910 |
| `crates/areka-emo-text/src/state.rs` | 499 | ○ | 499 一致 | ≈ 530 |
| `crates/areka-emo-text/src/region.rs` | 863 | ○ | 863 一致 | ≈ 890 |
| `crates/areka-emo-text/src/actor.rs` | 879 | ○ | 879 一致 | 不変 |
| `crates/areka-emo-text/src/choice.rs` | 550 | ○ | 550 一致 | 不変 |

6 ファイルとも 1,000 行未満です。見込み行数の最大は `layout.rs` の ≈ 910 で、
行数の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs`）の例外表に触れる必要はありません。

### 1.3 このタスクで Rust ソースは 1 行も変えていない

`cargo build -p areka-emo-text` が終了コード 0（警告は既存の `pasta_core` パッチ未使用の 1 件のみ）。

---

## 2. 洗い出しの手順と、design の一覧との突き合わせ（要件 7.1）

### 2.1 実行した検索

design.md「Testing Strategy／再導出台帳」が入口として指定する crate 全域の検索を実行しました。

```
rg -l "1\.25|1\.33|37\.24|line_pitch|line_box|band|pitch" \
   crates/areka-emo-text/src crates/areka-emo-text/tests crates/areka-emo-text/examples
```

→ **57 ファイル**。

この検索語は「pitch」等の語を含まない数値だけの依存を取りこぼすため、補助の検索も行いました。

```
rg -l "13\.0|15\.0|22\.5|25\.0|35\.0|50\.0|12\.5|7\.5|37\.24|18\.75" \
   crates/areka-emo-text/src crates/areka-emo-text/tests crates/areka-emo-text/examples
```

→ 39 ファイル。うち 57 件に含まれないのは `src/state_cursor_coord_parse_tests.rs` 1 件だけで、
中身は `parse_cursor_coord("50%")` の `value: 50.0`（座標の解析層・行送りとは無関係）。**追加不要**。

### 2.2 design が挙げる一覧の実数え

design.md と `research.md` §3.3 が挙げるファイルを重複なく数えると **37 ファイル** でした。
両文書が使う「32 ファイル」という呼び方は数え違いです（内訳: 純粋層 21・COM 層 7・`tests/` 7・example 2）。
37 ファイルはすべて実在し（`ls` で確認）、すべて 2.1 の検索にヒットします（取りこぼし 0 件）。

**訂正**: 以降この台帳では「32 ファイル」ではなく「design が挙げる 37 ファイル」と呼びます。

### 2.3 差分

57（検索ヒット） − 37（design の一覧） = **20 ファイル** が一覧の外にありました。内訳:

| 区分 | 件数 | ファイル |
|---|---|---|
| design の「Modified Files」に別枠で載っている実装ファイル | 9 | `src/state.rs`・`src/layout.rs`・`src/region.rs`・`src/draw.rs`・`src/choice.rs`・`src/actor.rs`・`src/canvas.rs`・`src/viewbox_draw.rs`・`examples/emo-text-typewriter-demo.rs` |
| **台帳へ新たに追記した（本節 3 の A 群に反映）** | 5 | `src/layout_cursor_order_tests.rs`・`src/choice_decorate_tests.rs`・`src/layout_test_support.rs`・`src/viewbox_test_support.rs`・`src/canvas.rs`（実装ファイルだが in-file テストの再導出が要る＝design の「不変」の是正） |
| 検索語には当たるが行送りの数値に依存しない（作業なし・§5 に記録） | 7 | `src/cursor_tag.rs`・`src/viewbox.rs`・`src/surface.rs`・`src/actor_clear_atomicity_tests.rs`・`src/viewbox_choice_marker_tests.rs`・`examples/emo-text-layer.rs`・`examples/emo-text-layer/verdict.rs` |

（`src/canvas.rs` は 1 行目と 2 行目の両方に現れます。実装ファイルとしては design の Modified Files にあり、
in-file テストの再導出という点では新規の追記です。）

**結論（突き合わせの明示）**: design の一覧との差分は 0 件ではありません。
**4 ファイルを新規に追記**し（`layout_cursor_order_tests.rs`・`choice_decorate_tests.rs`・
`layout_test_support.rs`・`viewbox_test_support.rs`）、
**1 ファイルの分類を是正**しました（`canvas.rs`＝design は「不変」だが in-file テストが赤になる）。
追記後の再導出対象は **41 ファイル**（37 + 4）です。

---

## 3. 再導出台帳（本体）

分類の意味:

- **A** 純粋層（`FixedMetrics`）の期待値。DirectWrite を使わない固定寸のテスト。
- **B** COM 層・実フォント／既定フォントを使うテスト。
- **C** 「N 行が収まり N+1 行目であふれる」という容量の前提を持つテスト。
- **D** 検証対象が裁定で存在しなくなり退役するもの（代替を立てて本数は減らさない）。

「改名追随」欄は、`TextLayerConfig` の旧調整値の識別子 `line_pitch_factor` を名前で参照しているかどうか
（＝クレートのコンパイル可否に関わるか）です。詳細は §4。

### 3.1 A 群（純粋層・`FixedMetrics`）

| ファイル | 分類 | 旧値 → 新値 | 改名追随 | 出所 |
|---|---|---|---|---|
| `src/layout_wrap_tests.rs` | A | 行 top 13 → **12**・bottom 23 → **22**・font 12 の `pitch × Σratio 1.5` 22.5 → **21**・`\n[0.5]` 7.5 → **7**・bottom 27 → **26**・満杯 3 行の境界 36 → **34**（4 行目 46 > 34 → `block_offset −12`） | 不要 | design |
| `src/layout_segmented_tests.rs` | A | 13 → **12**・23 → **22**・15×1.5 → **21** | 不要 | design |
| `src/layout_visible_window_tests.rs` | A | 境界 36 → **34**（3 行の下端 10/22/34 がちょうど）・4 行目 46 > 34 → `−12`・6 行時 70−36 = **34** → `−36`・縦 rl 列左端 390/378/366/354（4 列目 354 < 360 → `+12`）・lr 列右端 10/22/34/46 → `−12`。**タスク 3.1 の実走で 3 件を追記**: ⑴ `all_lines_overflowing_saturates_to_newest_line`（font 50 → pitch 63 → **52**・tops 0/52/104・オフセット −126 → **−104**）は行に無かった。⑵ `fractional_ratio_feed_scrolls_by_fractional_line_distance` も行に無く、しかも**新ピッチが偶数だと端数が消える**（`14 × 0.5 = 7.0`）。テスト名と doc が「端数そのもの（整数量子化しない）」を主題にしているため、font 12 → **13**（pitch 15）へ導き直して `−7.5` を保った＝**入力を動かした唯一の箇所**（要件 9.1 の「期待値の更新のみ」の例外として登記）。⑶ 既に緑だった `horizontal_within_region_does_not_scroll` と `trailing_pending_newline_does_not_trigger_overflow` も「3 行ちょうど収まる」を前提にしており、境界 36 のままだと 2px の余裕が生まれて意味を失う。両方 36 → **34** へ | 不要 | design ＋ **タスク 3.1 で追記** |
| `src/layout_cursor_overflow_tests.rs` | A | 境界 36 → **34**・素の 4 行 top 0/12/24/36・`\_l[,@-2lh]` = 36−24 = **12**・5 行目 `{10,12,20,22}`（最新行 22 ≤ 34 で非発火）／対照 `{1, −12}`・6 行 top 0..60 の 7 行目 `{2, −24}`・13/26/39 → **12/24/36** | 不要 | design |
| `src/layout_cursor_tests.rs` | A | 13 → **12** 系 | 不要 | design |
| `src/layout_cursor_center_origin_tests.rs` | A | 同上 | 不要 | design |
| `src/layout_cursor_vertical_tests.rs` | A | 同上（列送りへ軸読み替え）。**タスク 3.1 で追記**: `\_l[-13,0]`／`\_l[13,0]` の**実数値そのものが 1 列送り**を表しており、doc も「2 列目」「自動列送りと同値」と述べている。±13 → **±12** へ動かさないと、既に緑のテストが偽の主張を残す | 不要 | design ＋ **タスク 3.1 で追記** |
| `src/layout_cursor_vertical_canon_tests.rs` | A | 同上。**タスク 3.1 で追記**: この 2 本は `\_l[±13,0]` を**同じ走行の自動列送りと突き合わせて**いるため、±12 へ動かさないと赤になる。上の兄弟ファイルの実数値も同時に動かさないと、片方が 388、片方が 387 を「2 列目」と述べる食い違いが残る | 不要 | design ＋ **タスク 3.1 で追記** |
| `src/layout_cursor_wiring_tests.rs` | A | 同上 | 不要 | design |
| **`src/layout_cursor_order_tests.rs`（262 行）** | **A** | `written_order_decides_relative_cursor_against_newline` と `written_order_applies_newlines_before_and_after_the_cursor` の `lines[..].rect.top` が **13.0 → 12.0**（2 本）・`113.0 → 112.0`（2 本＝100 + 12）。doc の「行送り 0 + 13 = 13」「100 + 13 = 113」「100 + 2×13 = 126」を 12／112／124 へ | 不要 | **本台帳で追加** |
| `src/cursor_tag_tests.rs` | A | `LINE_PITCH` 由来の値（13 → **12**） | 不要 | design |
| `src/cursor_tag_resolve_tests.rs` | A | 同上 | 不要 | design |
| `src/cursor_tag_test_support.rs`（106 行） | A | `LINE_PITCH: f32 = 13.0` → **12.0**・doc「1lh＝1em＋行間。`ceil(10 × 1.25) = 13`」→「`10 + 2 = 12`」。`lh` 係数 12 と `em` 10 は引き続き相異なる（要件 7.5） | 不要 | design |
| **`src/layout_test_support.rs`（95 行）** | **A（doc のみ）** | 共通前提の doc「font 10 → pitch 13（ceil(12.5)）」→「font 10 → pitch **12**（10 + 2）」。値の宣言はないので赤にはならないが、旧式を「共通前提」と述べる記述を残さない（要件 7.5） | 不要 | **本台帳で追加** |
| **`src/viewbox_test_support.rs`（108 行）** | **A（doc のみ）** | 同じ 1 行の doc「font 10 → pitch 13（ceil(12.5)）」→ 12（10 + 2） | 不要 | **本台帳で追加** |
| `src/state_cue_apply_tests.rs` | A | `line_gap == 2.0` の確認へ差し替え。`assert_eq!(config.line_pitch_factor, 1.25)` は新しい欄名・新しい値へ | **要** | design |
| `src/state_reveal_tests.rs` | A | `1.25` は再生時刻の値であり行送りではない。**作業なし**（誤検知の記録として台帳に残す） | 不要 | design |
| `src/choice_tests.rs` | A | 帯の clamp 系は退役させない。注入ピッチ 35 → **30**・中間値の例 32 → **29**（`clamp(29, 28, 30) = 29`・`clamp(37.242, 28, 30) = 30`） | 不要 | design |
| **`src/choice_decorate_tests.rs`（428 行）** | **A** | `const TEST_BAND: f32 = 13.0` → **12.0**（font 10 の帯上限＝新ピッチ）・doc「em ボックス丈 10.0 ではなく 13.0」→ 12.0・合成入力 `glyph_resident((0.0, 13.0))` **7 箇所**（`:89 :112 :147 :205 :336 :365 :403`）→ `(0.0, 12.0)`・および倍数の 1 箇所 `glyph_resident((0.0, 26.0))`（旧ピッチ 13 の 2 行目）→ `(0.0, 24.0)`。倍数を置き忘れると 0／12／26 という新旧どちらの格子でもない並びが残る。**注意**: 装飾は受け取った帯をそのまま焼き込むだけなので値を変えなくても緑のまま。旧値を「正典」と述べる doc の是正が要件 7.5 の本体で、注入値はそれに揃える | 不要 | **本台帳で追加** |
| `src/viewbox_axis_tests.rs` | A | `1.25` は DPI 拡大率 k。**作業なし** | 不要 | design |
| `src/viewbox_dirty_tests.rs` | A | 13 → **12**・露出帯 `{0,88,400,12}` →（ガード）→ `{0,87,400,13}`・列帯 `{0,0,12,200}` → `{0,0,13,200}`・`by = −12` | 不要 | design |
| `src/viewbox_plan_commit_tests.rs` | A | `window(1, −13.0)` → **−12**・13 → **12**・前提 doc | 不要 | design |
| `src/actor_tests.rs` | A | `1.25` は k。**作業なし** | 不要 | design |
| `src/actor_scale_refresh_tests.rs` | A | 同上。**作業なし** | 不要 | design |
| `src/actor_choice_contract_tests.rs` | A | `pitch = FONT_H + 2.0 = 14`・`indent_y = 2lh = 28`。式を inline で書かず `TextLayerConfig::default().line_pitch(FONT_H)` を呼ぶ | 不要 | design |
| **`src/canvas.rs`（722 行・in-file テスト）** | **A** | `from_layout_generates_one_glyph_resident_per_line`: `r1.transform.offset()` **(0.0, 13.0) → (0.0, 12.0)**・doc「変換 = (0, 13)（pitch 分の平行移動）」→ (0, 12)。`fractional_line_feed_survives_in_translation`: `vec![(0.0,0.0),(0.0,15.0),(0.0,22.5)]` → ⚠**この置換値は採らないこと**。`(0,0),(0,14),(0,21)` はすべて整数で、テスト名（`fractional_line_feed`）が主題にしている端数が消える＝緑のまま意味を失う。タスク 3.1 が `layout_visible_window_tests.rs` の同型のテストで採った手当と揃えて、font 12 → **13**（pitch 15）へ導き直し `(0,0),(0,15.0),(0,22.5)` を保つか、端数を落としてよい理由を明記すること。**タスク 3.1 の実走で判明**: このファイルの赤は 2 本でなく **3 本**で、3 本目は `from_layout_translation_carries_line_origin`（本台帳に記載が無かった）。**据え置き**: `from_layout_maps_empty_line_to_empty_resident` の `top: 15.0` と `(0.0, 15.0)` は手で組んだ合成入力（レイアウトから導いていない）ため不変。`apply((1.0,1.0)) == (3.0, 7.5)` は変換の算術で行送り無関係。`:176-182` の帯 doc も不変 | 不要 | **本台帳で是正**（design「Modified Files」は `canvas.rs` を「不変」としているが、上記 2 本の in-file テストはレイアウト経由の期待値であり赤になる） |
| `tests/pipeline_test.rs` | A | 横書き: 行下端 10/22/34/46・境界 36 は据え置き（3 行が収まる前提のみ）・4 行目 46 > 36 → `−12`。縦書き: 列 i の左端 = 346 − 12i。**25 列では 25 列目の左端 58 ≥ 36 であふれない**ため **27 列**へ導き直す（27 列目の左端 346−312 = 34 < 36）・オフセット `+12`・reveal 途中の `lines.len()` 期待も列数に合わせて再計算 | 不要 | design |

### 3.2 B 群（COM 層・実フォント／既定フォント）

| ファイル | 分類 | 旧値 → 新値 | 改名追随 | 出所 |
|---|---|---|---|---|
| `src/draw_format_metrics_tests.rs`（737 行） | B | `line_pitch(12)` 15 → **14**・`line_pitch(10)` 13 → **12**。係数 2.0 の非既定検査は非既定 `line_gap` の分岐へ（D 群も参照）。`line_box_height(28) = 37.24` の検査は**不変** | **要** | design |
| `src/viewbox_draw_frame_render_tests.rs` | B | 6 行 y = 0,12,…,**60**・`block_offset −12` | 不要 | design |
| `src/viewbox_draw_live_diff_tests.rs` | B/C | font 20: P = 20+2 = **22**・F = 20（`2P+F = 64 ≤ 80 ≤ 3P+F = 86`）→ 面寸 (160,80)／(80,160) は**据え置き**。font 12: P = **14**・F = 12（`40 ≤ 50 ≤ 54`）→ (80,50)／(50,80) 据え置き＝doc のみ。負の対照 `live_diff_detects_injected_divergence` は新式でも赤のまま（要件 7.4） | 不要 | design。**ただし §3.5 の裁定案件 R-2 を参照**——タスク 2.1 の実走で `yugothic_real_fixture_matches_oracle_byte_for_byte` が赤になり、面寸の据え置きとは別の未決事項（製品側の欠陥の疑い）が残っている |
| `src/viewbox_draw_choice_hover_tests.rs` | B | 注入 `band_extent` 13 → **12**（font 10 のピッチ上限）。「帯 > em ボックス 10」の関係と `expand_overhang_for_band` の検査は保たれる | 不要 | design |
| `src/viewbox_draw_png_dump_tests.rs` | B | ピッチを実行時に `metrics.line_pitch` から読む＝**自動追随・作業なし** | 不要 | design |
| `tests/draw_readback_test.rs` | B | `PITCH` 15 → **14** | 不要 | design |
| `tests/viewbox_blit_spike.rs` | B | `N` 15 → **14**・`BLOCK_POS` [10,25,40,55] → **[10,24,38,52]**・doc 3 か所 | 不要 | design |
| `tests/scale_invariance_test.rs` | B | font 40: pitch 50 → **42**・`block_offset −50 → −42`・3 行目の下端 46+84+40 = **170 > 168** で縦スクロールは引き続き発火（行数不変の検査は新式でも成立）。縦書き font 10: rl 4 列目の左端 400−10−36 = 354 < 360 → `+12`・lr 4 列目の右端 46 > 40 → `−12`。`1.25` の主用途は DPI 拡大率 k で、そちらは不変 | 不要 | design |
| `tests/emo2_fixture_e2e_test.rs` | B（文言のみ） | 本体側（font 28）のピッチは 35 → **30** になるが、書き換える対象は `:533` の `eprintln!` 文言「Yu Gothic UI 行ボックス 37.24 → 帯はピッチ 35 で頭打ち」→ 30 の **1 か所だけ**。`\b35\b` の全ヒットがこの 1 行（実測）。走査 y 窓（`:503-507`）は `row0_cl`／`row1_cl` の block 起点から実行時に導出しており帯寸に依存しない＝自動追随。文字送りは不変 | 不要 | **本台帳で是正**（design の「hover 帯の y 範囲」は書き換え対象ではない） |
| `tests/choice_fixture_test.rs` | B（自動追随） | 帯・行送りの数値定数を**持たない**。ハイライト矩形は実行時の `ChoiceHitRow`（`:348` で `choice_hit_rows` から取得）から導出され、走査 y 窓も `:394` の記述どおり「hover 行の block 起点〜次行の block 起点＝帯寸に依存しない独立の窓」＝**自動追随・作業なし**。検索に当たった理由は `:393` の doc「Yu Gothic UI は行ボックス 1.3301em」1 行のみで、`35` はファイル内に 1 件も無い（`rg "\b35\b"` がヒット 0 件・終了コード 1） | 不要 | **本台帳で是正**（design は「hover 帯 30」と書くが、書き換える定数は存在しない）。**ただし §3.5 の裁定案件 R-1 を参照**——タスク 2.1 の実走で `real_font_menu_hover_render_dumps_png` が赤になり、期待値の再導出ではなく裁定で解く案件へ移った |

em（文字の大きさ）は変わらないため、グリフ描画・文字送り・折返し位置は不変で、
ピッチ由来の差だけが出ます（Yu Gothic UI を使うテストの x 座標は不変）。

### 3.3 C 群（容量の前提・要件 7.3）

| ファイル | 分類 | 旧値 → 新値 | 改名追随 | 出所 |
|---|---|---|---|---|
| `tests/viewbox_scroll_test.rs` | C | `PITCH` 15 → **14**・`FILL_LINES` は **8 のまま**（7×14+12 = 110 ≤ 120・8×14+12 = 124 > 120 で `const _: () = assert!` が両方成り立つ）・容量の式を述べる doc を書き直す | 不要 | design |
| `src/viewbox_draw_live_diff_tests.rs` | C | 面寸は据え置き。容量式の doc を P = 22／14 で書き直す（B 群にも記載） | 不要 | design |
| `examples/emo-text-layer/scenario.rs`（116 行） | C | 横書き容量 3 → **4** 行（4 行目の下端 164 ≤ 168・5 行目 194 > 168）・縦書き容量 9 → **10** 列（`floor((320−28)/30)+1`）・pitch 35 → **30**・`LINE3` の「3 行ちょうどの最終行」前提を「4 行のうちの 3 行目」へ・`OVERFLOW_LINES 9` は据え置き（3+9 = 12 行 > 4・13 列 > 10）・`EXPOSURE_BAND_DRAW_BOUND 3` は実走で再確認 | 不要 | design |
| `src/draw_oracle_tests.rs` | C | ＭＳ ゴシック 10・pitch 12 で行下端 **10/22/34/46**。`validrect.bottom 40` に対し 3 行目 34 ≤ 40・4 行目 46 > 40 なので「4 行目があふれる」前提は**そのまま成立**。コメントの数値だけ書き直す。スクロール後は `−12` | 不要 | design |
| `src/viewbox_draw_oracle_regression_tests.rs` | C | font 28・pitch 30 で「行 1 セル 0..28・行間 **28..30**・行 2 セル **30..58**」。2px の行間領域が残るので行境界の欠け診断は意味を保つ。本体側と同寸（320×122）で 4 行目 90..118 まで収まり 5 行目であふれる。要件 7.4 の「両側とも同じ寸法」の確認はこのファイルで行う | 不要 | design |
| `examples/emo-text-layer/drive.rs` | C（自動追随） | `TextLayerConfig::…line_pitch(font_height)` を実行時に読む。**作業なし**（scenario.rs 側の容量前提が変わるため、実走の観測値は再確認する） | 不要 | design |

### 3.4 D 群（退役・要件 7.2 の個別記録）

| 退役するテスト | 場所 | 根拠 | 代替（本数は減らさない） |
|---|---|---|---|
| `fixed_metrics_line_pitch_ceils_fractional_values` | `src/layout_wrap_tests.rs` | 検証対象の `ceil` の端数処理そのものが裁定で存在しなくなる | `fixed_metrics_line_pitch_adds_default_gap` へ名前と本文を差し替え（`12 → 14`・`10 → 12`・`h + 2` 以外の式で赤になること） |
| `dwrite_metrics_line_pitch_follows_config_canon` の「係数 2.0」分岐 | `src/draw_format_metrics_tests.rs` | 係数の乗算が式から消える | 非既定 `line_gap` の分岐へ差し替え |

**退役させないもの**（明示）: `line_box_height` 系・帯の clamp 系・`expand_overhang_for_band` 系。

### 3.5 R 群（裁定で解く案件・期待値の再導出では解けない）

タスク 2.1 の実走で赤になった 2 本です。**どちらも 3.x（期待値の再導出）の対象ではありません。**
数値を付け替えても意味が失われるだけなので、下の指示のとおりに扱ってください。

| # | テスト | 赤の中身 | 扱い | 担当 |
|---|---|---|---|---|
| R-1 | `tests/choice_fixture_test.rs::real_font_menu_hover_render_dumps_png` | hover 行のインクの縦範囲 y5..22 がハイライト矩形 y5..21 の内側に収まらない（帯 30 に対しインクの下端が **1 画素**下）。ベースラインが design metrics の ascent（2210/2048 × 28 ≈ 30.2）に置かれる帰結で、design §「帯の防御式を保つ」の残存リスクの的中 | **裁定 2026-09-06 ＝ 1 画素のはみ出しを許容**（要件 3.6・design §4.1／§「帯の防御式を保つ」）。検査を「帯の下端からのはみ出しは **1 画素以内**」へ導き直す。**帯は広げない**（隣接する行の帯と重なり、どの選択肢を指しているかの一意性が壊れる）。これは**裁定による意味の変更**であって許容幅を緩めたものではないので、導き直す側は裁定日（2026-09-06）と理由をコメントに引くこと。2 画素以上なら改めて裁定へ | タスク 5.2 |
| R-2 | `src/viewbox_draw_live_diff_tests.rs::yugothic_real_fixture_matches_oracle_byte_for_byte` | 行の下端からはみ出したインクの切り落とし（D2・t=2.10・visible=27・相違行 y=[0]）。参照描画との画素単位の等価比較で、実描画の経路がはみ出したインクを切り落とし、参照側は切り落とさない | **未決**。R-1 の裁定では片づかない。再描画の対象範囲が行の下へはみ出したインクを覆えていない**製品側の欠陥の疑い**として、それ自体の是非を調べる。**帯を広げて隠さない・テストを緩めない**。最初に見るのは `viewbox_draw.rs` の `expand_overhang_for_band`（はみ出しの分だけ再描画の範囲を広げるために置かれている関数） | タスク 3.4（要件 7.4 の「参照描画との画素等価比較が両側とも同じ寸法で動くことを確認し、差分を注入する負の対照が新しい式でも赤になることを示す」がこの検査そのもの。3.4 の境界の中で根本原因を直せない場合は、その時点で改めて引受先を立てて登記する） |

---

## 4. 別欄: 旧調整値の識別子を名前で参照している箇所（改名追随）

期待値の再導出とは別の欄です。**クレートがコンパイルできるかどうか**に関わるため分けています。

現行の調整値の識別子は `TextLayerConfig::line_pitch_factor`（`src/state.rs` の
`pub struct TextLayerConfig` のフィールド・既定 `1.25`）です。

`rg -n "line_pitch_factor" crates/` の全ヒットは **18 件・6 ファイル**。すべて `areka-emo-text` の中にあり、
他クレートは 1 件もありません。

| ファイル | 参照の種類 | 追随の内容 |
|---|---|---|
| `src/state.rs` | 定義（doc・フィールド宣言・`Default`） | 欄を `line_gap`（既定 2.0）へ置き換え、`line_pitch(&self, font_height)` と `normalized()` を足す |
| `src/layout.rs` | doc の参照＋`FixedMetrics::line_pitch` の式（`TextLayerConfig::default().line_pitch_factor` を掛けて `ceil`） | `TextLayerConfig::default().line_pitch(font_height)` を返す形へ |
| `src/draw.rs` | doc 3 か所・`DWriteMetrics` の保持フィールド宣言・縮退経路の `fallback = config.line_pitch_factor`・組み立て・`line_pitch` の式 | 保持するのを係数ではなく `TextLayerConfig` にし、`self.config.line_pitch(font_height)` を返す。縮退値の名前も追随 |
| `src/draw_format_metrics_tests.rs` | **構造体リテラル** `TextLayerConfig { line_pitch_factor: 2.0 }`＋doc | 非既定 `line_gap` の分岐へ（D 群と同じ作業） |
| `src/state_cue_apply_tests.rs` | 値比較 `assert_eq!(config.line_pitch_factor, 1.25)`＋コメント | 新しい欄名・新しい既定値（`line_gap == 2.0`）へ |
| `examples/emo-text-typewriter-demo.rs` | コメント「config は line_pitch_factor のみゆえ既定でよい」 | 文言を新しい欄名へ |

### 4.1 構造体リテラルの所在（コンパイルが壊れる箇所）

`rg -n "TextLayerConfig\s*\{" crates/` の結果、`TextLayerConfig` を波括弧で組み立てているのは
**`src/draw_format_metrics_tests.rs` の 1 か所だけ**です（他のヒットは `pub struct` 宣言・`impl Default`・
`fn config(&self) -> &TextLayerConfig` の戻り型）。

`crates/areka` 側の 12 ファイル（`emo2_boot/`・`input_events/` 配下）は `TextLayerConfig` を使いますが、
すべて `TextLayerConfig::default()` 経由で、フィールドを名前で書いていません。
したがって **`line_gap` の追加でコンパイルが壊れるのは上記 1 か所のみ**、`crates/areka` は無傷です。

---

## 5. 別欄: 警告 1 件の追加による再導出（`TextRegion::resolve`）

`region.rs` の `TextRegion::resolve` の末尾に「折返し基準が描画範囲の外に解決された」場合の
警告 1 件（`warn!`）が加わります。行送りとは無関係ですが、ログ件数を数えているテストを
赤にし得るため、1 行として台帳に置きます。

**実測の結果、現状のどの検査も赤になりません。**

| 場所 | ログ件数の固定 | 新しい警告の影響 |
|---|---|---|
| `src/region.rs` の in-file テスト `two_layer_merged_fixture_yields_nondegenerate_region` | `assert_eq!(warns, 0, "非退化矩形は warn を記録しない")` | この fixture は本体側（`descript.txt` の `wordwrappoint.x = -34` を `balloons0s.txt` が `-49` で上書き）で、折返し基準 351 ≤ 描画範囲の右端 356。**警告は出ないので 0 のまま緑**。テスト内の `assert_eq!(region.wrap_threshold(), 351.0)` も不変 |
| `src/region.rs` の in-file テスト `base_layer_only_validrect_is_degenerate_and_warns` | `assert!(warns >= 1, ...)` | 下限のみの検査なので警告が増えても緑 |
| `src/region.rs` の in-file テスト（`ScaleContract::new` の `assert_eq!(warns, 1, ...)`） | 件数固定あり | `TextRegion::resolve` を通らない（拡大率の契約）ので**無影響** |
| `tests/shipped_fixture_region_test.rs`（397 行） | **なし**（`count_levels`／`warn` の記述が 0 件） | 無影響 |
| `src/cursor_tag_test_support.rs`（106 行） | **なし**（`TextRegion::resolve` を呼ぶがログを捕まえていない） | 無影響 |

**注**: design.md はこの 3 か所を「ログ件数を固定しているもの」としてまとめて挙げていますが、
実際に件数を固定しているのは `region.rs` の in-file テストだけで、しかも該当の fixture では
新しい警告が発火しません。相方側（`balloonk0s.txt`＝折返し基準 254 > 描画範囲の右端 240）を
使う新規の検査（`src/region_inline_limit_tests.rs`・要件 6.7）を足すときに、
警告 1 件をそこで固定します。

---

## 6. 検索に当たるが行送りの数値に依存しないファイル（作業なし）

再検索したときに「台帳から漏れている」と誤って判断しないための記録です。

| ファイル | 当たった理由 | 判定 |
|---|---|---|
| `src/cursor_tag.rs` | `CursorBasis::line_pitch` のフィールドと `unit_coefficient` の `Lh` 分岐 | 数値定数を持たない。`\_l` の `lh` 係数は `line_pitch` を受け取るだけで自動追随。作業なし |
| `src/viewbox.rs` | doc に「行 pitch」の語・`exposure_band`（露出帯＝スクロールで新しく見える帯）の関数名 | 行送りの数値定数を持たない。露出帯の値を固定しているのは `viewbox_dirty_tests.rs`（A 群）側。作業なし |
| `src/surface.rs` | `row_pitch`（Direct3D の `RowPitch`＝1 行のバイト数） | 別物。誤検知 |
| `src/actor_clear_atomicity_tests.rs`（396 行） | `ink_in_band`・`bands` の語 | 帯は実行時の `choice_hit_rows` の矩形から作っており、行送りの数値定数を持たない。自動追随。作業なし |
| `src/viewbox_choice_marker_tests.rs`（234 行） | `band_extent: 10.0` の注入 | 手で入れた合成値で、コメントも「ここでは em ボックス丈で足りる」。帯の上限は旧 13 → 新 12 でどちらも 10 を上回るため前提は保たれる。作業なし |
| `examples/emo-text-layer.rs` | `band_ink`・`inline_extent_first_band` の `use` 行 | 再輸出のみ。作業なし |
| `examples/emo-text-layer/verdict.rs` | `pitch` を引数で受ける観測関数 | 呼び出し側（`drive.rs`）が実行時に読む。自動追随。作業なし |
| `src/state_cursor_coord_parse_tests.rs`（254 行） | 補助検索の `50.0`（`"50%"` の解析） | 座標の解析層。行送り無関係。作業なし |

---

## 7. design との相違点のまとめ（要件 7.1 の「着手時に確かめる」の結果）

| # | design の記述 | 実測 | 本台帳での扱い |
|---|---|---|---|
| 1 | 「計 32 ファイル」 | 重複なく数えると **37 ファイル** | 呼び方を「37 ファイル」へ訂正。取りこぼしはなく、数え方の誤りのみ |
| 2 | `src/canvas.rs`（722）は**不変** | in-file テスト **3 本**がレイアウト経由の行送りを固定しており赤になる（`(0.0, 13.0)`・`[(0,0),(0,15.0),(0,22.5)]`・`from_layout_translation_carries_line_origin`。3 本目はタスク 3.1 の実走で判明） | A 群へ追加（§3.1） |
| 3 | 一覧に `layout_cursor_order_tests.rs` がない | `lines[..].rect.top` に `13.0`・`113.0` を直接固定（4 か所） | A 群へ追加 |
| 4 | 一覧に `choice_decorate_tests.rs` がない | `TEST_BAND = 13.0` と「13.0 が正典」と述べる doc | A 群へ追加（要件 7.5） |
| 5 | 一覧に `layout_test_support.rs`／`viewbox_test_support.rs` がない | 「共通前提: font 10 → pitch 13（ceil(12.5)）」の doc | A 群へ追加（doc のみ・要件 7.5） |
| 6 | 警告 1 件で `region.rs` in-file テスト・`shipped_fixture_region_test.rs`・`cursor_tag_test_support.rs` が赤になり得る | 件数を固定しているのは `region.rs` のみで、その fixture（本体側・soft 351 ≤ hard 356）では警告が出ない。他 2 ファイルはログを捕まえていない | §5 に実測として記録。既存の赤は発生しない見込み |
| 7 | B 群の `tests/choice_fixture_test.rs`（hover 帯 30・文字送り不変） | 帯・行送りの数値定数を持たない。ハイライト矩形は実行時の `ChoiceHitRow` から導出され、走査窓も `:394` の記述どおり帯寸に依存しない。doc `:393` の「1.3301em」だけが検索に当たる（`35` はファイル内に 0 件） | §3.2 に残したまま**自動追随・作業なし**へ是正（`viewbox_draw_png_dump_tests.rs`・`drive.rs` と同じ扱い＝実質は §6 の「作業なし」と同じ） |
| 8 | 同上（#7 の「自動追随・作業なし」） | タスク 2.1 の実走で `real_font_menu_hover_render_dumps_png` が赤。定数は持たないが、実行時に導出した帯（30）へ実フォントのインク（下端 y22）が **1 画素**収まらない | §3.5 の **R-1** へ移す（期待値の再導出ではなく**裁定 2026-09-06 ＝ 1 画素のはみ出しを許容**で解く。担当はタスク 5.2） |
| 9 | B/C 群の `src/viewbox_draw_live_diff_tests.rs`（面寸は据え置き＝doc のみ） | タスク 2.1 の実走で `yugothic_real_fixture_matches_oracle_byte_for_byte` が赤。面寸とは別に、行の下端からはみ出したインクの切り落としで参照描画と食い違う | §3.5 の **R-2** へ移す（**未決**・製品側の欠陥の疑いとして調べる。帯を広げず・テストを緩めない）。引受先はタスク 3.4 |

以上 9 件は、いずれも「設計を黙って写さず、実測との食い違いを記録する」方針での追記・訂正です（#8／#9 はタスク 2.1 の実走で判明した分で、2026-09-06 に追記しました）。
値そのものの導出（新式 `h + 2`）は design.md §4.1 の正典表どおりで、変更はありません。
