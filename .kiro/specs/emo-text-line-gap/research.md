# 調べの記録: emo-text-line-gap

## 1. ギャップ分析（要件の段・2026-10-31・ブランチの先頭 `cb8efc37`＝main `9604677a` の上）

> 本節は `/kiro-validate-gap` の出力。決めは出さず、案と見込みと調べ残しを並べる。数の見込みは**まだ測っていない**（要件 1.1 で測る）。測った数は §2 以降へ書き足す。

### 1.1 まとめ

- **直す所は 1 か所に集まっている**: 横書きの帯は `choice.rs` の `line_bands`（211 行）の横書きの腕（232〜238 行）だけが決め、塗り（`decorate_canvas`→`viewbox_draw_render.rs::highlight_rect`）・当たり（`derive_hit_rows`）・字の矩形（`glyph_cells`）・ダーティ帯（`expand_overhang_for_band`）は全部この列を読む。直しは塗りと当たりへ一緒に効く（要件 3.1 は形の上で既に満たされている）。
- **食い違いの源**: `draw_metrics.rs::measure_line_box_ratio`（385 行）は `DWRITE_FONT_METRICS` の `(ascent + descent) ÷ upem` だけを返し、`lineGap` を読まない。横書きの行は `create_line_layout`（`draw.rs` 491 行）が行の間隔を設定せず（横書きは何もしない）既定の組み方で組むので、DirectWrite は行の高さを `ascent + descent + lineGap` にする。
- **一番大きい分かれ目は「DirectWrite が書体の行間を字の上と下のどちらに置くか」**。外の資料（§1.5）は「上に置く」を示すが、Microsoft の資料は書いていない＝要件 1.1 で測るまで決まらない。下の §1.4 の見込みの表のとおり、**案 2 は 3 つの仮説のどれでも中央に置ける**が、**案 1 は「上下に半分ずつ」のときだけ**中央になり、「上に置く」なら 3〜5 画素ずれたまま残る。
- 案 2 は既存の形（縦書きの `vertical_em_box_left` を 1 度だけ測って比で持つ・測れなければ `warn!` で縮退）をそのまま横書きへ写せる。書体の行間 0 の書体と `FixedMetrics` では足す量が 0 になり、今の檻は数も画素も動かない見込み（要件 5.1）。
- 規模 S（4〜7 タスク）・危険 低〜中（未知は DirectWrite の行間の置き場 1 点で、それは最初に測る）。

### 1.2 今の形（file:line は `cb8efc37`）

#### 帯を決める所

| 所 | 中身 |
|---|---|
| `choice.rs:137` `highlight_band_extent(font_height, line_box_height, line_pitch)` | `clamp(line_box_height, em, max(em, pitch))`。厚み。 |
| `choice.rs:175` `highlight_band_offset(line_box_height, band_extent)` | `round(max(0, line_box_height − extent) / 2)`。行矩形の上辺からの寄せ（0 以上）。**「行の箱の上端＝行矩形の上端」を暗に前提にしている**（行の箱の中央へ寄せる式）。 |
| `choice.rs:211` `line_bands` | 行ごとに em（行矩形の丈＝その行の最大の em）を取り、`metrics.line_box_height(em)` を読む。縦書きの腕は `vertical_em_box_left` から、横書きの腕は上の 2 関数から。 |
| `layout.rs:140` `GlyphMetrics::line_box_height` | 説明は「DirectWrite は行を ascent＋descent で組む」。書体の行間を持つ書体ではこの説明が事実と違う（DirectWrite の行の高さは行間を含む）。 |
| `layout.rs:163` `GlyphMetrics::vertical_em_box_left`（既定の実装）・`layout.rs:170` `column_centred_em_box_left` | 既定の実装は `line_box_height` を読む（列の箱の真ん中）。 |
| `draw_metrics.rs:348` `DWriteMetrics::line_box_height` | `em × line_box_ratio`（生成時に 1 度だけ測った比）。 |
| `draw_metrics.rs:359` `DWriteMetrics::vertical_em_box_left` | 縦書きに束縛＝測った比 × em・向き違い＝0・**横書きに束縛（`None`）＝既定の式 `column_centred_em_box_left(line_box_height)`**。 |
| `draw_metrics.rs:149` `new_shared` の縮退 | 比を測れないとき `warn!`＋`line_pitch(h) / h`（要件 4.2 が保つ扱い）。 |
| `draw_metrics.rs:385` `measure_line_box_ratio` | `GetMetrics` の ascent・descent・upem。`lineGap` は読まない。 |
| `draw_metrics.rs:433` `measure_vertical_em_box_left_ratio` | 本番と同じ書式・同じ生成口・「あ」1 字の TextLayout の `GetMetrics().left + GetLineMetrics()[0].baseline`。**案 2 の前例**。 |

#### 字の置き場（横書き）

- 行の TextLayout は行矩形の左上を原点に描く（`canvas.rs:340` 住人の平行移動＝行矩形の原点・`draw.rs:713`／`viewbox_draw_render.rs:423` の `draw_text_layout`）。行矩形の丈は em（`layout.rs:501`）。TextLayout の箱の高さも em（`create_line_layout` の `max_height = block_extent`）で、段落の寄せは NEAR。ゆえに**字の基線は行矩形の上辺から DirectWrite の `GetLineMetrics()[0].baseline` の所**に在る。
- 字の箱（要件の用語）は `[baseline − ascent, baseline + descent]`（ascent・descent は `DWRITE_FONT_METRICS` の値 × em ÷ upem）。書体の行間 0 では `baseline = ascent` なので字の箱＝`[0, line_box_height]`＝今の式の前提と同じ。

#### 帯を読む下流（変えずに済む見込みの所）

| 所 | 読む物 | 案 2 で変わるか |
|---|---|---|
| `choice.rs:304` `derive_hit_rows` | `LineBand` の 2 値 | 変わらない（値だけが変わる） |
| `choice.rs:432` `glyph_cells` | 行矩形と帯の和（正の寄せも負の寄せも扱える） | 変わらない |
| `viewbox_draw_render.rs:699` `highlight_rect` | `band_offset`・`band_extent` | 変わらない |
| `viewbox_draw_render.rs:738` `expand_overhang_for_band` | 横書きは**遠端（下）だけ**を `offset + extent − font.height` で広げる。近端（上）は広げない＝横書きの寄せが負になると上の塗りが消し残る | 書体の行間が正なら寄せは正のまま＝変わらない。**行間が負の書体**（`DWRITE_FONT_METRICS::lineGap` は INT16 で負になり得る）でだけ問題になり得る（§1.6 Research Needed） |
| `crates/areka/src/input_events/balloon.rs` | 半開区間の当たり（`y >= top && y < bottom`） | 変わらない（要件 3.3 は帯が接するだけなら満たされる） |

#### `line_box_height` を読む所の全部（`rg line_box_height`）

- 本番: `choice.rs:225`（帯）・`layout.rs:164`（縦書きの既定の実装）・`draw_metrics.rs:364`（横書きに束縛した物の縦書きの問い＝既定の式）。
- 本番での `draw_metrics.rs:364` の到達: `actor_decoration.rs:66` は描画器と同じ `mode` で `DWriteMetrics` を作り、`actor_present.rs:591` の `line_bands` も同じ `mode` を渡す＝**本番では横書きに束縛した物へ縦書きを問う道は無い見込み**（設計の段で確かめる）。
- テスト（期待値を式から取る物・書体の行間 0 の書体か `FixedMetrics`）: `choice_tests.rs:731`（980 行）・`actor_choice_contract_tests.rs:858`（991 行・Yu Gothic UI 28 で `highlight_band_extent`／`highlight_band_offset` を直に呼ぶ）・`viewbox_draw_choice_hover_tests.rs:673`（記録だけ）・`viewbox_draw_vertical_band_readback_tests.rs:84`・`viewbox_draw_vertical_band_scroll_tests.rs:67`・`choice_decorate_tests.rs:742/771/796`・`draw_metrics_styled_tests.rs:389`（横書きに束縛した物の `vertical_em_box_left` ＝ 既定の式、を固定）・`draw_format_metrics_tests.rs:502`（比が ascent＋descent 由来であることを固定・922 行）・`tests/staysee_balloon_fixture/region.rs:303`（ＭＳ ゴシックで行の箱＝em）・`tests/kero_menu_capacity_test.rs:273`（委譲）。
- **形の制約**: `highlight_band_extent`／`highlight_band_offset` の**引数の形を変えると**、上限に近く本ウェーブの触る一覧に無い `actor_choice_contract_tests.rs`（991 行）と `choice_tests.rs`（980 行）が直る羽目になる。2 関数の形は保ち、書体の行間の分は `line_bands` の中で足すのが触る所を最小にする。

#### 検査の形の前例

- 記録だけの検査（無視の印つき）: `viewbox_draw_choice_hover_tests.rs` の `vertical_em_box_from_column_baseline_record`（完了 `vertical-choice-hover-band` タスク 5.1）。
- 実物の書体の読み戻し（名前を固定・代わりの書体では赤）: `viewbox_draw_vertical_band_readback_tests.rs` の `R_FONTS`（候補列 → 引けるはずの名前）。`FontCatalog::new`・`pick`・`family_for` と `ResolvedFont::resolve` は `pub`（`draw_catalog.rs:120/141/242/263`）なので、統合テスト（`tests/`）からも同じ引き方ができる。
- 横書きの実物の読み戻し: `tests/line_pitch_readback_test.rs`（709 行・Yu Gothic UI 28 だけ・`hover_band_centers_in_the_line_box_and_contains_choice_ink` が帯とインクの上下を読む）。`choice`・`layout`・`draw` は `pub mod`（`lib.rs:49/52/54`）なので、偽の `GlyphMetrics` を使う決定論のテストも `tests/` に置ける＝**`lib.rs` の一覧の席を使わずに済む**。

### 1.3 要件と持ち物の対応（Missing／Unknown／Constraint）

| 要件 | 今ある物 | 穴 |
|---|---|---|
| 1.1 游ゴシック 20・28 の行の箱・基線・インク・今の帯の読み戻し | 縦書きの記録の検査（5.1）の形・横書きの読み戻しの形（`line_pitch_readback_test.rs`） | **Missing**: 横書きで `GetLineMetrics` の baseline・height を読み、字の箱と帯とインクを並べる記録の検査。**Unknown**: DirectWrite が行間を上下のどちらに置くか（§1.5） |
| 1.2 行間 0 の書体の基準 | Yu Gothic UI 28 は `line_pitch_readback_test.rs` の表（帯 y4..33・インク y7..31）。20 とＭＳ ゴシックは記録が無い | **Missing**: 同じ記録の検査に 2 書体を足す |
| 1.3／1.4 裁定・ずれていなければ止まる | — | 手順だけ（コードの穴ではない） |
| 1.5 後から回し直せる記録だけの検査 | `#[ignore]` の前例 | **Constraint**: 置き場（§1.7 の決め 6） |
| 2.1 字の箱の中心に帯の中心 | 縦書きは測った基線から置く形で済み | **Missing**: 横書きの字の箱の上端（＝基線 − ascent）を `GlyphMetrics` に出す口（案 2）か、行の箱の比の意味を変える（案 1） |
| 2.2 em 以上・行送り以下・インクを覆う | `highlight_band_extent` の clamp | 厚みは今の式で満たす見込み。**Unknown**: 游ゴシックの仮名と漢字のインクが厚み 22（20）・30（28）に収まるか（字の箱 25.74・36.04 より狭い帯） |
| 2.3 隣の行と重ならない | 厚みの上限＝行送り・寄せは行ごとに同じ | 満たす見込み（どの行も同じ寄せ・厚み＝行送りで接するだけ） |
| 2.4 大きい字の行 | `line_bands` は行の最大の em で引く | 案 2 の比×em は、行の最大の字が同じ書体なら比例で正しい（DirectWrite の行の基線は run の基線の最大＝一番大きい run が決める）。**Constraint**: 行の中で `\f[name,…]` で別の書体に替えた字は今も束縛の書体の比で見る（今からある限り・`balloon-font-file`／`text-typesetting` の持ち分） |
| 2.5 アンカーも同じ形 | アンカーも `line_bands` を通る（`choice_anchor_tests.rs`） | 穴なし |
| 2.6 厚みの決まりを書体の名前で分けない | 式は書体の名前を読まない | 穴なし（裁定の中身しだい） |
| 2.7 満たせなければ止まる | — | 手順だけ |
| 3.1〜3.3 当たり＝塗り | 1 つの関数（`line_bands`）から両方 | 穴なし（`choice-ranges-one-function` で済み） |
| 4.1 行の箱の寸法を読む所の食い違い | — | **Missing**: 横書きの帯（案で解く）と、`DWriteMetrics::vertical_em_box_left` の `None` の枝（横書きに束縛した物の縦書きの問い）の扱い。`layout.rs:127` の説明も事実と違う |
| 4.2 縮退の扱いを保つ | `new_shared` の `warn!`＋ピッチ相当 | 案 2 で足す測りの縮退（測れなければ足す量 0＝今の帯・`warn!`）を同じ型で足す |
| 4.3 §8 の行 | `doc/COMPAT_ARCHITECTURE.md:124` §8 の表 | 末尾に 1 行 |
| 5.1 行間 0 の書体の画素 | 既定フォントのバイト一致の檻（`hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage` ほか）・`line_pitch_readback_test.rs` | 案 2 は足す量が 0 で動かない見込み。案 1 も行間 0 では比が同じで動かない見込み |
| 5.2 縦書き | 縦書きは測った基線（`Some(ratio)`）で決まる | 案 1 は `None` の枝（本番で到達しない見込み）だけが動く。案 2 は動かない |
| 5.3 字の描画 | 書式・TextLayout に触らない | どちらの案も触らない（書式に行の間隔を設定する案は範囲外） |
| 5.4 檻を消さない・ゆるめない | — | **Constraint**: 期待値を書き換えてよいのは行間を持つ書体の横書きの帯と当たりの檻だけ。`draw_metrics_styled_tests.rs:389`（縦書きの既定の式の固定）を書き換える案はこの枠に入るか要確認（§1.7 の決め 4） |
| 6.1 行間を持つ寸法の決定論のテスト | `FixedMetrics`（比 1.33・行間 0）だけ | **Missing**: 行間を持つ偽の `GlyphMetrics`（例: 字の箱の上端を返す） |
| 6.2 実物の游ゴシックの読み戻し | 縦書きの R の形 | **Missing**: 横書き版。書体の名前の固定 |
| 6.3 直しを外すと赤 | 縦書きの 19.3 節の手順 | 手順だけ |
| 6.4 排他テストの机で短く 1 回 | `doc/impl-watch.md` の手順 | 手順だけ |

### 1.4 直し方の案

共通の記号（大きさ em・行矩形の上辺を 0）: `A`＝ascent、`D`＝descent、`G`＝DirectWrite の行の高さ −（A＋D）、字の箱の上端 `t`（＝基線 − A）、行の箱の丈 `B = A + D`（今の `line_box_height`）、厚み `E = clamp(B, em, pitch)`。

#### 案 1: 行の箱の比に書体の行間を数える（`measure_line_box_ratio` を DirectWrite の行の高さへ）

- 直す所: `draw_metrics.rs` の `measure_line_box_ratio` だけ（＋説明・§8）。帯の式は今のまま `offset = round((B' − E') / 2)`、`B' = A + D + G`。
- 変種 1a: `DWRITE_FONT_METRICS::lineGap` を足す。**Unknown**: この欄が返すのが hhea の 1024 か、DirectWrite が実際に使う差 645（＝3281 − 2636、§1.5）か。1024 なら行の高さ 32.04 と合わない（35.74 になる）。
- 変種 1b: 「あ」1 字の横書きの TextLayout の `GetLineMetrics()[0].height` を比にする（縦書きの測りと同じ型）。表の欄の意味に頼らない。
- 帰結: 帯は「DirectWrite の行の箱の中央」に置かれる。これが字の箱の中央になるのは**行間が字の上下に半分ずつ置かれるときだけ**（下の表）。
- 4.1 の第 2 項（縦書きの既定の逃げ道）: `None` の枝と既定の実装が新しい `line_box_height` を読むので値が動く（本番で到達しない見込み・`draw_metrics_styled_tests.rs:389` は両辺が同じ関数を読むので緑のまま）。
- 厚み: `B'` は 20・28 では行送りで頭打ちになり今と同じ。`A + D < pitch < A + D + G` になる組（行送りの行間を大きく取った設定など）では行間の分だけ厚くなる。
- ✅ 直す所が 1 関数・新しい口が要らない。❌ 「行間の置き場」の仮説が外れると中央にならない。❌ `line_box_height` の意味（字の箱 → 行の箱）が変わり、`layout.rs:127` の説明とそれを読む下流の前提（`highlight_band_offset` の「行の箱の上端＝行矩形の上端」）を書き直す必要がある。

#### 案 2: 横書きの帯も DirectWrite が測った基線から置く（`GlyphMetrics` に横書きの字の箱の上端を足す）

- 直す所:
  - `layout.rs`: `GlyphMetrics` に既定の実装つきのメソッド（仮名 `horizontal_char_box_top(font_height) -> f32`・既定 0＝今の前提）を足す。`FixedMetrics` と既存の偽の実装は触らない。
  - `draw_metrics.rs`: `new_shared` で、横書きに束縛した物だけ 1 度、`create_line_layout`（横書き）で「あ」の TextLayout を作り `GetLineMetrics()[0].baseline − A × em ÷ upem` を比で持つ（A は `measure_line_box_ratio` が既に読む face metrics から。今は和だけを返すので、ascent も返す形に広げるか別に読む）。測れなければ `warn!`＋0（＝今の帯）。
  - `choice.rs`: `line_bands` の横書きの腕で `offset = round(t + (B − E) / 2)`（`t = metrics.horizontal_char_box_top(em)`）。`highlight_band_offset` の形は保ち、`t` を足す所を `line_bands` に置けば、上限に近い 2 ファイルを触らない（丸めの順を `round(t) + round(...)` にするか `round(t + ...)` にするかは設計で決める。行間 0 ではどちらも同じ）。
- 帰結: 帯は字の箱の中央（要件 2.1 の文面どおり）。行間の置き場がどうであっても、測った基線から出すので正しい。
- 4.1: 横書きの帯は `line_box_height` を「字の箱の丈」として読み続ける（意味を「字の箱」と書き直す）。DirectWrite の行の箱とは読まない。縦書きの既定の逃げ道は値が動かない（§1.7 の決め 4 で、`None` の枝を読まない形へ改めるかを別に決める）。
- 厚み: 今の `E` のまま（行間を厚みに数えない）。
- ✅ 縦書きと同じ「単一の源・1 度だけ測る」型。✅ 行間 0 の書体・`FixedMetrics` で足す量が 0＝今の檻が動かない見込み。✅ 仮説に依らない。❌ `GlyphMetrics` に口が 1 つ増える。❌ 測りが 1 つ増える（生成時に TextLayout 1 つ）。

#### 案 3（参考・範囲外）: 横書きの書式に行の間隔を設定して行間を消す

- `SetLineSpacing` で字の組み方を変える。字の置き場が動く＝要件 5.3（字の描画を変えない）と Boundary の Out に当たる。要件の討議で要ると決まらない限り取らない。

#### 測る前の見込み（仮説ごと・論理画素・行矩形の上辺 0）

游ゴシック: A＋D ＝ 25.74（20）・36.04（28）、G ＝ 6.30（20）・8.82（28）、行送り 22・30、今の厚み E ＝ 22・30。字の箱の中心との差（帯の中心 − 字の箱の中心）。

| 仮説（DirectWrite が G を置く所） | 大きさ | 字の箱 | 今の帯（差） | 案 1（差） | 案 2（差） |
|---|---|---|---|---|---|
| 上（基線 ＝ A ＋ G） | 20 | 6.30..32.04 | 2..24（**−6.2**） | 5..27（**−3.2**） | 8..30（−0.2） |
| 上 | 28 | 8.82..44.86 | 3..33（**−8.8**） | 7..37（**−4.8**） | 12..42（＋0.2） |
| 半分ずつ | 20 | 3.15..28.89 | 2..24（**−3.0**） | 5..27（0.0） | 5..27（0.0） |
| 半分ずつ | 28 | 4.41..40.45 | 3..33（**−4.4**） | 7..37（−0.4） | 7..37（−0.4） |
| 下（基線 ＝ A） | 20 | 0..25.74 | 2..24（＋0.1） | 5..27（**＋3.1**） | 2..24（＋0.1） |
| 下 | 28 | 0..36.04 | 3..33（0.0） | 7..37（**＋4.0**） | 3..33（0.0） |

- 「下」なら今の帯は既にずれていない＝要件 1.4（直しに入らず決めを仰ぐ）の場面。
- どの仮説でも、どの案でも、厚みは行送りに等しく寄せは行ごとに同じ＝隣の帯と接するだけで重ならない（要件 2.3）。
- インクを覆うか（要件 2.2）は測らないと分からない（帯 22・30 は字の箱 25.74・36.04 より狭い。Yu Gothic UI 20 の帯 22 は下の余り 0 で覆っている＝`choice.rs:161` の表）。

### 1.5 外の資料（DirectWrite の行間の置き場）

- Microsoft の `DWRITE_FONT_METRICS` の説明は「推奨の行送り（基線から基線）は ascent・descent・lineGap の和」とだけ書き、行間を字の上下のどちらに置くかは書いていない（[DWRITE_FONT_METRICS](https://learn.microsoft.com/en-us/windows/win32/api/dwrite/ns-dwrite-dwrite_font_metrics)）。
- `DWRITE_LINE_SPACING`（Windows 10 の `IDWriteTextFormat3`）の説明は、既定の組み方では `baseline` の値と行間を前後へ分ける割合 `leadingBefore` を読まないこと、`fontLineGapUsage` で書体の行間を行の寸法に入れるかを選べることを書く（[DWRITE_LINE_SPACING](https://learn.microsoft.com/en-us/windows/win32/api/dwrite_3/ns-dwrite_3-dwrite_line_spacing)）。既定の組み方での置き場は書いていない。
- Wine の DirectWrite の作り直し（`dlls/dwrite/layout.c` の `layout_get_font_height`）は、既定の組み方の基線を `ascent + lineGap`、行の高さを `ascent + descent + lineGap` にしている＝**行間を字の上に置く**（[wine-mirror/wine layout.c](https://raw.githubusercontent.com/wine-mirror/wine/master/dlls/dwrite/layout.c)）。Wine は互換を目指した別の実装で、Windows の DirectWrite の正本ではない。**見込みは「上」だが、要件 1.1 の測りで決める**。
- 数の裏づけ（完了 `vertical-choice-hover-band` research.md 16.4・17.2・17.6）: 游ゴシックの書体の表は hhea 1802／−455／1024・OS/2 win 2017／619（upem 2048）。比から出す行の箱 25.74 ＝ (2017 ＋ 619) ÷ 2048 × 20 ＝ **DirectWrite の ascent・descent は OS/2 の win の値**。DirectWrite の列の箱 32.04 ＝ (1802 ＋ 455 ＋ 1024) ÷ 2048 × 20 ＝ **行の高さは hhea の和**。差 G ＝ 645 設計単位（20 で 6.30・28 で 8.82）。`DWRITE_FONT_METRICS::lineGap` がこの 645 を返すか hhea の 1024 を返すかは資料に無い（案 1a の前提）。

### 1.6 調べ残し（Research Needed）

1. **DirectWrite が既定の組み方で行間を字の上下のどちらにどれだけ置くか**（`GetLineMetrics()[0].baseline` を游ゴシック 20・28 の横書きで読めば決まる）。要件 1.1 の測りの中心。
2. `DWRITE_FONT_METRICS::lineGap` の値（645 か 1024 か）。案 1a を取るときだけ要る。
3. 游ゴシックの仮名と漢字のインクの上下が、字の箱の中央に置いた厚み 22・30 の帯に収まるか（要件 2.2・2.7 の場面の有無）。
4. 拡大率 2 での物理画素の余り（縦書きの前例は拡大率 1・2 の両方を測った）。
5. 最後の行の帯が行矩形の下へ 10 画素前後出る（案 2・「上」の仮説・20 で帯 8..30）ことが、可視の窓・面の端で切られないか。字のインク自体も今から同じだけ下へ出ているので新しい問題ではない見込みだが、ダーティ帯（`expand_overhang_for_band`）が遠端を広げることを読み戻しで確かめる。
6. 行間が負の書体（`lineGap` は INT16）で案 2 の寄せが負になる組があるか。あれば `expand_overhang_for_band` の横書きの近端（`viewbox_draw_render.rs:747`・本ウェーブの触る一覧に無い）が要る。手元の検体に該当は無い見込み。
7. `DWriteMetrics::vertical_em_box_left` の `None` の枝が本番で本当に到達しないか（`actor_decoration.rs:66` と `actor_present.rs:591` が同じ `mode` を渡すこと以外の道が無いか）。

### 1.7 設計・要件の討議へ渡す決め

1. **測りの時期**: 要件の段の終わりに測るか、最初のタスクで測るか（要件 1.1 は「直し方を決める前」とだけ定める）。測った数しだいで §1.4 の表のどの行かが決まり、案 1 が成り立つかどうかもそこで決まる。
2. **案 1／案 2 の裁定**（要件 1.3）。測る前の見込みでは、案 2 は 3 つの仮説のどれでも中央、案 1 は「半分ずつ」のときだけ中央。案 1 を取るなら 1a（表の `lineGap`）か 1b（TextLayout の行の高さ）かも決める。
3. **厚みの決まり**（要件 2.6）: 書体の行間を厚みに数えるか。游ゴシック 20・28 ではどちらでも行送りで頭打ちで同じ。違いが出るのは字の箱 ＜ 行送り ＜ 行の箱の組だけ。
4. **縦書きの em の箱の既定の逃げ道**（要件 4.1 の第 2 項）: `DWriteMetrics` の `None` の枝（横書きに束縛した物への縦書きの問い）を (a) 今の式のまま「字の箱の丈を読む・本番で到達しない」と説明で示す、(b) 0（列のレシピが em の箱を真ん中に置く＝`Some(_)` の向き違いと同じ）へ改めて行の箱を読まない形にする。(b) は `draw_metrics_styled_tests.rs:389` の期待値を書き換えるので、要件 5.4 の「書き換えてよい檻」の枠に入るかを決める。`FixedMetrics` の既定の実装は「書体から測った」寸法ではないので 4.1 の外と読むかも合わせて決める。
5. **`line_box_height` の意味と名前**: 案 2 なら「字の箱の丈」と説明を書き直すだけにするか、名前まで改めるか。名前を改めると `rg line_box_height` の 20 か所余り（上限に近い `choice_tests.rs` 980・`actor_choice_contract_tests.rs` 991・`draw_format_metrics_tests.rs` 922 を含み、本ウェーブの触る一覧に無いファイルも多い）に手が届く。
6. **検査の置き場**（要件 1.5・6.1・6.2）: 記録だけの検査・決定論のテスト・実物の読み戻しを、(a) `tests/line_pitch_readback_test.rs`（709 行・3 本を足すと 1,000 行に近づく・`clippy-199-lints` の段 2 の既存の指摘と同じファイル）、(b) `tests/` の新しいファイル（`lib.rs` の一覧の席を使わない・`choice`・`layout`・`draw` は `pub`）、(c) `src/` の兄弟のテストファイル（席を使う）のどれに置くか。
7. **丸めの順**（案 2）: `round(t + (B − E) / 2)` か `round(t) + round((B − E) / 2)` か。行間 0 ではどちらも今と同じ。要件 2.1 の「1 画素以内」はどちらでも満たす見込み。
8. **読み戻しの拡大率**: 要件 1.1・6.2 は物理画素と書くが拡大率を定めない。縦書きの前例（拡大率 1・2）に揃えるか。
9. **要件 1.4 の場面の扱い**: 測って「下」（今の帯がずれていない）だったときの次の手（spec を閉じるか・4.1 の説明の直しと §8 の行だけを残すか）を前もって決めておくか。

### 1.8 規模と危険

- **規模 S**（4〜7 タスク: 測りの記録 1・裁定・直し 1〜2〔口＋測り＋帯の式〕・檻と読み戻し 1〜2・§8 と記録 1）。既存の型（縦書きの 5.1〜5.3）をそのまま写せる。
- **危険 低〜中**: 未知は DirectWrite の行間の置き場とインクの収まりの 2 点で、どちらも最初の測りで決まる。案 2 は行間 0 の書体で足す量が 0 なので、今のバイト一致の檻を動かさない見込み。案 1 は仮説が外れると要件 2.1 を満たせず、要件 2.7 の場面になる。

### 1.9 本ウェーブとの約束の照らし合わせ

- 触る見込みのファイル: 案 2 ＝ `layout.rs`（551 行・口 1 つ）・`draw_metrics.rs`（462 行・測り 1 つ）・`choice.rs`（790 行・横書きの腕）・`tests/` の検査（決め 6）・§8。案 1 ＝ `draw_metrics.rs`（＋`layout.rs` の説明）・`tests/`・§8。いずれも C15 の約束の一覧の中。
- 触らない見込み: `viewbox_draw_render.rs`（寄せが正のままなら）・`actor_choice_contract_tests.rs`・`choice_tests.rs`・`draw_format_metrics_tests.rs`（2 関数の形を保てば）。
- 依存は足さない（`windows` の DirectWrite の型は既に使っている）。

### 1.10 要件の討議の確定（2026-10-31・開発者が全議題を推奨案で承認）

1. §1.7 の決め 1〜3: **案 2 に裁定**（測る前に決める・3 つの仮説のどれでも中央に置けるため）。厚みは書体の行間を数えない（今の式のまま）。測りは裁定の確かめとして実装の最初のタスクで行う（要件 1.1・1.3）。
2. §1.7 の決め 4: 縦書きの既定の逃げ道（`DWriteMetrics::vertical_em_box_left` の `None` の枝）は、本番で通らないことを設計で確かめ、確かめられたら値を変えず説明で示すだけ。通る道があれば行の箱を読まない形へ改める（要件 4.4）。
3. §1.7 の決め 9: 測って「下」（今の帯がずれていない）だったら、帯の式に触らず、`line_box_height` の説明の直し・記録だけの検査・§8 の行を残して閉じる。止まらない（要件 1.4）。
4. 設計の段へ回すもの: §1.7 の決め 5〜8（`line_box_height` の名前・検査の置き場・丸めの順・読み戻しの拡大率）と §1.6 の 5〜7（最後の行の帯のはみ出し・行間が負の書体・`None` の枝の到達）、案 2 の口の形と縮退（`warn!`＋0）。
