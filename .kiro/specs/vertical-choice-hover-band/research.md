# ギャップ分析: vertical-choice-hover-band

> 2026-10-23・`/kiro-validate-gap`・main `15d8b0f3` の上のブランチ `claude/kiro-start-vertical-choice-hover-band-9ac66b`（`f20d0cd5`）で調べた。SSP と画面は動かしていない（SSP の MCP での測りは開発者の許しが要る＝議題）。撮った物は完了 spec `areka-P0-pasta-in-windows-verification` の作業ツリーに残っていた物を読んだだけ。

## 1. まとめ

- **帯の置き場の源は 1 か所**（`choice.rs` の `line_bands`）で、塗り（`decorate_canvas` → `viewbox_draw_render.rs` の `highlight_rect`）と当たり（`derive_hit_rows`）と字の矩形（`glyph_cells`）へ同じ値が配られている。単一の源の形はそのまま使える。
- **原因の見込みは「値の当て方」**: 3 か所（`derive_hit_rows`・`highlight_rect`・`glyph_cells`）と消し残しの手当て（`expand_overhang_for_band`）が、右から左の縦書き（`VerticalRl`）でも左から右（`VerticalLr`）と同じく**列の左辺を近い辺**として帯を右へ伸ばしている。ところが DirectWrite の右から左の列送りは、列を行の箱の**右辺**に揃え、和文フォントでは列（ascent＋descent）が em より太いぶん**左へ**はみ出す。areka の中でも可視窓の計算（`layout.rs:419`）は右から左の近い辺を `rect.right` として扱っており、帯の側だけがこの約束から外れている。
- **撮った物の測り**（下の 3 節）: 帯はインクの**右**（前の列の側）へ約 18 物理画素ずれ、「雨だよ」のインク 36 画素のうち右の 22 画素だけを覆い、隣の「晴れてる」のインクに 12 画素かかっている。brief の「帯が列の左へずれ」は逆で、白抜きの字の帯の外の部分が白い背景に溶けて「字の左が切れた」ように見えている。
- **左から右の縦書きは今の式で列の中央に乗る見込み**（列が左辺から右へ伸びるので横書きと同じ形）。今ある縦書きの描画の検査が左から右だけなのは、壊れていない側だけを見ていたことになる。
- 規模 **S**・危険 **低〜中**（DirectWrite の縦書きでのインクの置き方は決定論のテストで確かめる必要がある）。

## 2. 今の形（file:line は `f20d0cd5`）

| 所 | 今の形 | 縦書きでの扱い |
|---|---|---|
| `crates/areka-emo-text/src/choice.rs:138` `highlight_band_extent` | 帯の厚み＝`clamp(行の箱, em, max(em, 行送り))` | 軸に依らない式（そのままで良い見込み） |
| `choice.rs:176` `highlight_band_offset` | 寄せ＝`round(max(0, 行の箱 − 厚み) / 2)` | 軸に依らない式（「近い辺から内へ」の量） |
| `choice.rs:206` `line_bands` | 行ごとに em（横＝`bottom−top`・縦＝`right−left`）から厚みと寄せを 1 度だけ決める | 縦の em の読み替えは正しい（`choice_tests.rs:706` が固定） |
| `choice.rs:289` `derive_hit_rows`（330〜335 行） | 縦は `left = rect.left + offset`・`right = rect.left + offset + extent` | **右から左でも左辺起点**（問題の 1 つ目） |
| `choice.rs:415` `glyph_cells`（432〜441 行） | 近い辺＝縦は `r.left`、遠い辺＝`max(r.right, near + offset + extent)` | **右から左でも左辺起点**（字の矩形が帯を含む約束が鏡にならない） |
| `viewbox_draw_render.rs:568` `highlight_rect`（587〜592 行） | 縦は `left = dx + block_offset + offset`（右へ伸ばす） | **右から左でも左辺起点**（塗り） |
| `viewbox_draw_render.rs:606` `expand_overhang_for_band` | 縦は `right` だけを `offset + extent − em` まで広げる | 右から左で帯が左へ出るなら `left` 側を広げないと消し残る（要件 3.3） |
| `viewbox_draw_render.rs:210` 描く原点 | 縦は `(dx + block_offset, dy)` | 行の TextLayout の幅＝em（`draw_line_store.rs:161`）。列の箱はその中で DirectWrite が置く |
| `draw.rs` `DirectionRecipe` | 右から左＝`READING_TOP_TO_BOTTOM`＋`FLOW_RIGHT_TO_LEFT`・段落 NEAR | 右から左の列は行の箱の右辺に揃う |
| `layout.rs:419` `visible_window` | 右から左の近い辺＝`-rect.right`・遠い辺＝`-rect.left` | **右から左の近い辺は右**という約束は areka の中に既に在る |
| `viewbox_diff.rs:330` 描き替えの範囲 | 縦は `x0 − overhang.left 〜 x0 + w + overhang.right` | `left` を広げれば受け取れる形は既に在る |
| `crates/areka/src/input_events/balloon.rs:286` `hit_choice_row` | 半開区間 `x >= left && x < right` | 隣の列の帯が接しても 1 点は 1 つにしか当たらない（要件 2.3） |
| アンカー（`\_a`） | 同じ `line_bands`→`derive_hit_rows`→`decorate_canvas` を通る（`choice_anchor_tests.rs:124`） | 源を直せば揃って直る（要件 1.7） |

### 今の検査が固定している縦書きの値

| 検査 | 固定している物 | 本 spec での扱い |
|---|---|---|
| `choice_tests.rs:833` `hit_row_vertical_block_band_uses_band_extent_from_left_edge` | 右から左・左から右の**両方**で帯＝`left 起点で右へ` | 右から左の期待値は崩れ 23 そのものを固めている＝書き換えが要る |
| `choice_tests.rs:906` `derive_hit_rows_shifts_vertical_band_by_offset_on_block_axis` | 同上（寄せ 2 を右へ） | 同上 |
| `choice_tests.rs:706` `line_bands_read_the_block_axis_in_vertical_modes` | 縦の em＝列幅（厚みだけ） | 厚みは変えない見込み＝そのまま通る |
| `choice_glyph_cells_tests.rs:225〜238` | 右から左と左から右で字の矩形が**同じ値** | 右から左は鏡の値に書き換えが要る |
| `choice_glyph_cells_tests.rs:248` `every_choice_hit_row_lies_inside_the_glyph_cells` | 3 方向で当たり行が字の矩形に入る | 直した後も通るべき檻（3 か所を揃えて直す証拠になる） |
| `viewbox_draw_choice_hover_tests.rs:397` `vertical_band_offset_shifts_paint_on_block_axis` | 左から右だけ・寄せの差だけ（絶対位置もインクも見ない） | そのまま。右から左とインクの被りは新しく足す |
| `choice_decorate_tests.rs:290` | 縦の行内の原点の差し引き | 帯の置き場に依らない＝そのまま |

横書きの検査（`choice_tests.rs:795`〜・`:883`〜・`:931`〜、`actor_choice_contract_tests.rs:830` の Yu Gothic UI の読み戻し、既定フォントのバイト一致）は、横書きの分岐に触らなければ動かない（要件 5）。

## 3. 撮った物の測り（崩れ 23 の量）

`<pasta-in-windows の作業ツリー>\target\pasta-verify-alt\shots\alt2\` の 3 枚（1150×800・同じ置き場）を `System.Drawing` で 1 列ずつ数えた。字の色と帯の色はどちらも (140,128,116)（ぱすたの `cursor.brush.color` ＝ `font.color`）、乗せた字は白。

| 撮った物 | 選択肢の列のインクの x（y 60〜230） |
|---|---|
| `2-choice-zorder.png`（乗せていない） | ないしょ 191–221・曇ってる 233–266・**雨だよ 276–311**・晴れてる 322–355 |
| `2b-choice-hover.png`（「雨だよ」に乗せた） | 帯（塗りの色が縦に続く所）**x 290–333**・y 62–181 |

- 帯の幅 44 物理画素。ぱすたは `font.height,20`・游ゴシック Medium（行の箱 ≒ 1.33em）なので厚み＝`clamp(26.6, 20, 22)` ＝ 22・寄せ 2。44 ÷ 22 から拡大率は約 2 と読める。
- 帯の左辺 290 はインクの左辺 276 より 14 画素右、帯の右辺 333 はインクの右辺 311 より 22 画素右で、隣の列「晴れてる」のインク（322〜）に 12 画素かかる。帯の中心はインクの中心より約 +18 物理画素（約 +9 画像画素）右。インク 36 画素のうち覆うのは右の 22 画素（約 6 割）＝「半分しか塗らない」の正体。
- 予想との照らし合わせ（仮説）: 右から左の列が行の箱の右辺に揃い左へ 6.6 はみ出すなら、列は `rect.left − 6.6 〜 rect.left + 20`（中心 +6.7）、今の帯は `rect.left + 2 〜 + 24`（中心 +13）で、ずれは +6.3 画像画素。測った +9 と向きは合い、量は字の形（「雨だよ」のインクの片寄り）と丸めの分だけ違う。**量の一致は決定論のテストで確かめる**（5 節の R1・R2）。
- 左から右の列なら列は `rect.left 〜 + 26.6`（中心 +13.3）で、今の帯（中心 +13）に合う見込み。

## 4. 要件ごとの持ち物と穴

| 要件 | 使える物 | 穴 |
|---|---|---|
| 1.1 右から左の帯が列に合う | `line_bands` の厚みと寄せの値 | **Missing**: 右から左の近い辺（右）を起点にする当て方（3 か所＋消し残し） |
| 1.2 左から右 | 今の式 | **Unknown**: 今のままで合うかを実フォントの読み戻しで確かめる |
| 1.3 行内の範囲は字の範囲 | `annotate_lines` の `inline_range` | なし（縦の行内軸は今のまま） |
| 1.4 隣の列にかからない | 厚み＝行送りで頭打ち・半開区間 | 右から左を鏡にすれば列ごとの帯は接するだけ。**Constraint**: 寄せの丸め（`round`）を鏡にしたとき隙間・重なりが出ないことを確かめる |
| 1.5 折り返し | 行ごとの `LineChoiceSegment` | なし |
| 1.6 大きい字の列 | 行ごとの em（`choice_tests.rs:685`） | **Unknown**: 縦書きで大きさの違う字が同じ列に在るとき、DirectWrite が小さい字を列のどこに置くか |
| 1.7 アンカー | 同じ源 | なし（源を直せば揃う） |
| 2.1 当たり＝塗り | 単一の源・`the_hit_band_and_the_painted_band_come_from_the_same_line_list` | 右から左の両方の値を鏡にする所が 2 ファイルに分かれる（選ぶ案しだい） |
| 2.2 帯の中をクリック | `hit_choice_row`・`to_window_physical` | なし |
| 2.3 境の点 | 半開区間 | なし |
| 3.1 スクロール | `block_offset`（描画）・`committed`（当たり） | なし（帯の起点の足し算は同じ所を通る） |
| 3.2 倍率 | `SetTransform(scale k)`・`to_window_physical` の ×k | なし |
| 3.3 消し残し | `expand_overhang_for_band`・`viewbox_diff.rs:330` の `overhang.left` | **Missing**: 右から左で帯が左へ出る分を `left` に入れる |
| 4.1 SSP の測り | `ssp-measurement.md` の段 2（乗せた帯は撮っていない）・SSP に `pasta_tategaki` の写しが在る（`C:\wintools\ssp\balloon\pasta_tategaki`） | **人の待ち**: 開発者の許し・画面の操作 |
| 4.2〜4.4 | — | 4.1 の結果しだい（SSP が em の幅で塗るか列の箱で塗るかは不明） |
| 5.1〜5.3 横書き | 横書きの分岐 | 触らなければ守れる |
| 6.1〜6.3 読み戻しの検査 | `viewbox_draw_choice_hover_tests.rs`（465 行）・`Rig`・`block_axis_ink_span`・`DWriteMetrics::new`（`actor_choice_contract_tests.rs:845` の型）・`live_diff_model_font(name, height)` | **Missing**: 右から左・左から右の実フォント（Yu Gothic UI）の読み戻しで、インクの x 範囲が帯の x 範囲に入ることを見る検査 |
| 6.4 外すと赤 | — | 直しを戻して赤になるのを 1 度確かめて記録する手順 |
| 6.5・6.6 実機 | `doc/pasta-real-run.md`・`real-run-mcp-kit` の道具 | **人の待ち**: 開発者の許し・在席の机 |

## 5. 調べ残し（設計の段で確かめる）

- **R1 列の箱の位置と幅**: 右から左・左から右で、幅 em の TextLayout の中に DirectWrite が列（`GetLineMetrics` の `height`・`baseline`）をどこに置くか。`viewbox_draw_underline_tests.rs:91` が横書きで `GetLineMetrics` を読む型を持つ。縦書きの列の厚みが `ascent + descent` か `+ lineGap` を含むかもここで分かる。
- **R2 縦書きのインクの置き方**: 縦の字のインクが列の箱の中央に乗るか（横書きは「行の箱の中央へ寄せる」で合わせた）。乗らないなら寄せの式そのもの（値の決め方）も縦書きで変える必要がある。R1 と同じ読み戻しの検査で分かる。
- **R3 SSP の帯の形**（要件 4.1）: em の幅で塗るか、列の箱で塗るか、列送りの幅で塗るか。areka の今の厚み（行送りで頭打ち）と食い違えば要件 4.3 の決めを仰ぐ。
- **R4 大きい字の混じる列**（要件 1.6）: 縦書きで `\f[height,…]` の字が混じる列で、小さい字が列のどこに乗るか。
- **R5 寄せの丸めの向き**: `highlight_band_offset` は `round` で、鏡にしたときに帯の端が画素の境に乗るか（拡大率 1.25・1.5 でも消し残りが無いか）。
- **R6 `--offline-dump`**（完了 `offline-surface-dump`）はシェルの面の書き出しで、バルーンの選択肢に乗せた状態は出せない見込み＝実機の確かめは `doc/pasta-real-run.md` の道で行う。

## 6. 直し方の案

### 案 A: 使う所で右から左を鏡にする（今の部品を広げる）

- `derive_hit_rows`・`glyph_cells`（`choice.rs`）と `highlight_rect`・`expand_overhang_for_band`（`viewbox_draw_render.rs`）の縦の腕を `VerticalRl` と `VerticalLr` に分け、右から左は `right = rect.right − offset`・`left = right − extent`、消し残しは `left` を広げる。
- ✅ `LineBand` の意味（近い辺から内への量）を変えない。式が「近い辺」の約束（`layout.rs:419`）と揃う。
- ❌ 同じ鏡の式が 4 か所に散る（1 か所を忘れると塗りと当たりがずれる）。`viewbox_draw_render.rs` に触る＝次のウェーブの `balloon-scroll-arrows` と同じファイル（このウェーブでは重ならない）。

### 案 B: 単一の源（`line_bands`）で鏡を吸収する

- `LineBand.offset` を「行矩形の左辺（横は上辺）から帯の起点までの符号つきの量」と定め、右から左だけ `offset = em − 寄せ − extent`（ぱすたの値で `20 − 2 − 22 = −4`）を出す。`derive_hit_rows`・`highlight_rect` は今の足し算のまま正しい位置になる。`glyph_cells` は既に `near.min(near + offset)` で負の寄せを受ける形。`expand_overhang_for_band` だけ負の寄せを `left` へ入れる直しが要る。
- ✅ 鏡の判断が 1 か所（「描画とヒットの唯一の源」の約束をそのまま守る）。差分が最も小さい。
- ❌ `LineBand.offset` と `highlight_band_offset` の文書（「内側へ寄せる量・負にならない」）と意味がずれる＝文書の書き換えが要る。消し残しの手当ては結局 `viewbox_draw_render.rs` に触る。

### 案 C: 列の箱を DirectWrite から測って帯に使う（混ぜる形）

- 行の TextLayout の `GetLineMetrics`／はみ出しの実測から列の箱を取り、帯をそれに合わせる。
- ✅ 縦書きの字の置き方が仮説と違っても実物に合う。
- ❌ 純粋な層（`choice.rs`・決定論の檻）に COM の測りの値を流す配管が要り、行のキャッシュと帯の単一の源の順序が変わる。規模が M へ上がる。R1・R2 が仮説どおりなら要らない。

新しいファイルを作る案は、ウェーブの約束（emo-text にソースファイルを足さない）で除いた。

## 7. 規模と危険

- **規模 S**（3〜5 タスク）: 式の直しは数行×2 ファイル、検査の書き換え 3 本、読み戻しの検査を 2 方向分足す、SSP の測り、実機の確かめ。
- **危険 低〜中**: 今の型の延長で済むが、R1・R2（DirectWrite の縦書きの列の置き方）が仮説と違うと寄せの式（値の決め方）まで直すことになる。決定論の読み戻しで先に確かめられる。

## 8. 設計の段への申し送り

- 先に R1・R2 を実フォント（Yu Gothic UI・比 1.33）の読み戻しで確かめ、右から左の列の箱が右辺に揃って左へはみ出すこと、左から右は今の式で合うことを数で押さえる。
- 案 B を第一の候補、案 A を次の候補として比べる（どちらも塗り・当たり・字の矩形・消し残しの 4 つを同じ式から出す）。
- 書き換える既存の検査（`choice_tests.rs` の 2 本・`choice_glyph_cells_tests.rs` の 1 本）は「崩れを固めていた期待値」として扱いを決める。`choice_tests.rs` は 967 行で、書き換えで 1,000 行を超えないこと。
- 新しい読み戻しの検査は `viewbox_draw_choice_hover_tests.rs`（465 行）に置き、当たりと塗りの数値の一致（要件 6.2）は `choice_glyph_cells_tests.rs`（323 行）か `choice_anchor_tests.rs`（273 行）に置ける。
- SSP の測り（要件 4.1）は設計の前の人の待ち。

## 9. SSP の測り（要件 4.1・2026-10-23・開発者の許し「許すので、もう一度 ssp の画面操作を頼んで測って」）

- **SSP**: `SSP/2.9.23 (20261021-5; Windows NT 10.0.26300)`。ゴースト「窓際のぱすたさん」、バルーン「窓際のぱすたさん縦書き」（`C:\wintools\ssp\balloon\pasta_tategaki`・`vertical,1`・`font.height,20`・游ゴシック Medium・`cursor.style,square`・`cursor.blendmethod,none`）。バルーンは既にこれで、切り替え・reload はしていない。画面の拡大率は 200%（撮った物の 1 画素＝物理の 1 画素＝バルーンの 0.5 画素）。
- **出した台本**（MCP の `sakurascript`）: `\0\![set,choicetimeout,0]\![set,balloontimeout,0]今日の天気は？\n\n\q[晴れてる,OnVcTest1]\n\q[雨だよ,OnVcTest2]\n\q[曇ってる,OnVcTest3]\n\q[ないしょ,OnVcTest4]`。測った後に `\e` で閉じた（`set` はそのスクリプトの中だけ効く＝残り物なし）。
- **撮った物**（`evidence/`・computer-use の拡大の撮影・JPEG）:
  - `ssp-choice-nohover.jpg`・`ssp-choice-hover-amedayo.jpg`・`ssp-choice-hover-hareteru.jpg`＝同じ範囲（311×207）。乗せていない／「雨だよ」に乗せた／「晴れてる」に乗せた。
  - `ssp-choice-after-leave.jpg`＝別の範囲（207×207）。「雨だよ」に乗せた後、マウスをバルーンの外へ出した。
- **数えた値**（`System.Drawing` で 1 列ずつ。x は撮った物の左端から、物理画素）:

| 物 | x の範囲 | 幅 |
|---|---|---|
| 「ないしょ」のインク | 38–65 | 28 |
| 「曇ってる」のインク | 81–111 | 31 |
| **「雨だよ」のインク** | **124–156** | 33 |
| 「晴れてる」のインク | 168–200 | 33 |
| **「雨だよ」に乗せた帯** | **118–157** | **40（＝バルーンの 20＝em）** |

  - 帯は「雨だよ」のインクを全幅で覆う（左に 6・右に 1 余る。帯の中心 137.5、インクの中心 140）。隣の列のインクとは、左（「曇ってる」）に 6、右（「晴れてる」）に 10 の隙間があり、かからない。
  - 列の間隔は約 43〜44 物理画素（バルーンの約 22）。帯（20）は列の間隔より細く、隣の帯とは接しない。
  - 縦（行内の軸）では、帯は y 25–140、インクは y 29–140。帯は選択肢の字の範囲だけを塗っている（列の全長ではない）。
- **areka の崩れとの比べ**（3 節）: areka の帯は 44 物理画素（バルーンの 22）で、インクより右へ約 18 ずれている。SSP の帯は 40（em）で、インクにほぼ中央で乗っている。
- **要件 1 との食い違い（要件 4.3）**: なし。SSP の形は「列の字のインクを全幅で覆う・隣の列にかからない・行内は字の範囲」で、要件 1.1・1.3・1.4 と合う。
- **設計への申し送り**: SSP の帯の厚みは em（20）で、areka の今の頭打ち（行送りの 22）とは違う。要件 4.2（SSP と同じ形）に合わせるなら、縦書きの厚みは em になる見込み。横書きは変えない（要件 5）。下の設計判断 4 で決める。
- **範囲の外で見つけたずれ**: SSP は、マウスをバルーンの外へ出しても最後に乗せた選択肢の帯を残す（`ssp-choice-after-leave.jpg`）。areka は窓の外へ出ると帯を消す（`crates/areka/src/input_events/balloon_exit.rs` の `clear_balloon_hover_on_leave`）。縦書きに限らない話なので本 spec の外とし、扱いは要件の討議に回す。
  - **決め（要件の討議 議題 3・2026-10-23・開発者「推奨で」）**: areka は今のまま（選択肢から離れたら帯を消す・完了 `areka-P0-choice-interact` の要件 1.3）とし、起票しない。ukadoc は「選択中／非選択中」の色を分けるだけで、マウスが離れたときの移り方を書いていない。開発者の見立ては「SSP の方がおかしい」。帯が残ると、その所をクリックすれば選ばれるように見え、当たり＝塗り（本 spec の要件 2）と食い違う。SSP がバルーンの中の余白へ動いたときに帯を残すかは測っていない。キーボードで選択肢を選ぶ仕組みを作るときは、「選択中」の持ち方として見直す。

### 設計判断（要件の討議 2026-10-23 で設計の段へ回した物）

1. 原因の置き場: 「当て方」（右から左の近い辺は右）と見るか、R1・R2 の読み戻しで「決め方」（寄せの式）まで縦書き用に変えるか。
2. 直す形: 案 B（`line_bands` で符号つきの寄せ）か案 A（4 か所で鏡）か案 C（DirectWrite で列の箱を測る）か。
3. 左から右の縦書き: 今の式のまま検査だけ足すか、読み戻しの結果で式も変えるか（要件 1.2 はどちらでも満たす）。
4. 縦書きの帯の厚み: 今の行送りでの頭打ち（ぱすたで 22）を保つか、列の箱（26.6）か、SSP で測った em（20・9 節）か。要件 4.2 から em が第一の候補。食い違いが出たら要件 4.3 で開発者へ。
5. 消し残し（要件 3.3）: 右から左で帯が左へ出る分を `expand_overhang_for_band` の `left` へ入れる形。
6. 読み戻しの検査（要件 6.1〜6.3）の置き場（`viewbox_draw_choice_hover_tests.rs`・`choice_glyph_cells_tests.rs`・`choice_anchor_tests.rs`）と書体（Yu Gothic UI か游ゴシック Medium か）。
7. 寄せの丸め（`round`）を鏡にしたときの端（R5）と、大きい字の混じる列（R4）。

## 10. 設計の段の調べ（2026-10-23・`/kiro-spec-design -y`・ブランチの先頭 `97d8ab80`）

- **調べの種類**: 軽い調べ（今ある仕組みの縦書きの枝を直す拡張）。外の資料は引いていない（新しい依存も新しい API も無い）。SSP と画面は動かしていない。
- **使った手引き**: `kiro-spec-design` の `rules/`（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と、ひな形 `.kiro/settings/templates/specs/design.md`・`research.md`。steering は `tech.md`（テスト）・`structure.md`（兄弟のテスト・1,000 行・純粋層の字面検査）・`focus.md`／`roadmap.md`（C11 の約束）を読んだ。
- **確かめた事実**（2 節の表は `97d8ab80` でも行番号まで合っている。足した物だけ書く）:

| 所 | 分かったこと | 設計への効き目 |
|---|---|---|
| `choice.rs:289` `derive_hit_rows`・`viewbox_draw_render.rs:568` `highlight_rect` | どちらも「行矩形の左辺（横書きは上辺）＋ `offset` から `extent`」の足し算だけで、`offset` の符号を見ない | `offset` を符号つきにすれば、この 2 か所は**書き換えなしで**右から左の帯を正しく置く |
| `choice.rs:415` `glyph_cells` | 近い辺を `near.min(near + offset)` で取る | 負の `offset` を既に受ける＝書き換えなし |
| `viewbox_draw_render.rs:606` `expand_overhang_for_band` | 縦書きは `right` だけを広げる | 帯が行矩形の左へ出る分（`−offset`）を `left` へ入れる直しが要る（ここだけがコードの直し） |
| `canvas.rs:189〜207` `ChoiceLineContent` の 2 欄の説明 | 「近端から内側へ寄せる量」「`round(max(0, …)/2)`」と書いている | 縦書きでは意味が変わる＝説明だけ書き換える（コードは変えない）。C11 のほかの 7 本は emo-text に触らない（`roadmap.md` の C11 の行）ので重ならない |
| `lib.rs` の純粋層の字面検査（`PURE_SOURCES`） | `choice_glyph_cells_tests.rs` などの `choice_*_tests.rs` は `windows` を書けない一覧に載る | 実フォントの読み戻し（COM）は `viewbox_draw_choice_hover_tests.rs` に置く。純粋の検査は `choice_decorate_tests.rs`・`choice_anchor_tests.rs`・`choice_glyph_cells_tests.rs` |
| `actor_choice_contract_tests.rs`（991 行） | 実フォントで帯を読み戻す型（`DWriteMetrics::new`・実フォントが在るかの前置き）を持つが、もう足せない | 型だけ借りて `viewbox_draw_choice_hover_tests.rs`（465 行）へ足す |
| `viewbox_draw_test_support.rs` の `live_diff_model_font(name, height)` | 書字の開始の角へ寄せる（右から左は右上）モデルを作る | 読み戻しの模型にそのまま使える |
| `choice_glyph_cells_tests.rs:248` の前置き | 「帯の遠い辺が em を越える」を 3 方向で前提にしている | 縦書きの帯が em の丈になると左から右で前提が崩れる＝前置きの書き換えが要る（6.7 の扱い） |
| `choice_tests.rs:706` | 右から左で em 40 の帯の丈を 42 と固めている | em の丈（40）へ書き換える（6.7 の扱い） |
| 既定フォント（ＭＳ ゴシック・行の箱＝em） | 縦書きの今の帯は丈 em・寄せ 0 | 新しい式でも丈 em・寄せ 0（行の箱の余りが 0）＝既定フォントの縦書きも 1 画素も動かない |

- **撮った物から読んだ模型の当て（3 節の数を読み直した・仮説）**: 「雨だよ」のインクは行矩形の左辺から見て −5〜+13（バルーンの画素・拡大率 2 から戻した）。DirectWrite が右から左の列（幅＝行の箱 26.6）を行矩形の右辺に揃えるなら、列の箱は −6.6〜+20。em の箱が列の箱のどこに乗るかを割合 c（列の箱の余り s ＝ 行の箱 − em のうち、em の箱の**左**に置かれる割合）で表すと:
  - c ＝ 0（em の箱が列の箱の左に寄る）: em の箱 −6.6〜+13.4 ⊇ インク −5〜+13。**よく合う**。
  - c ＝ 0.5（真ん中）: −3.3〜+16.7。インクの左 1.7 がはみ出す。
  - c ＝ 1（右に寄る＝行矩形そのもの）: 0〜20。インクの左 5 がはみ出す。
  - どれが正しいかは撮った物の 1 書体・1 大きさでは決めきれない（游ゴシック Medium の行の箱の比は 25〜29 のどれでも 3 節の帯 22・寄せ 2 と合う）。**設計の段の結論: c は実装の最初のタスクの読み戻しで決める**（下の判断 1）。
- **大きさの違う列が隣り合うときの当て（R4 の続き）**: 列送りは閉じる列の em ＋行間 2 で進む。c ＜ 1 のとき、右から左で em 40 の列の帯は左へ round((1−c)·13.2) 出て、隣の em 20 の列の帯の右辺（行矩形の右辺から round((1−c)·6.6) 左）と重なり得る（c ＝ 0 で 4 画素）。このとき DirectWrite の字の箱そのものが隣の列の字の箱に食い込んでいる見込み（列の並べ方＝完了 `balloon-vertical-canon` の持ち分）で、どの帯でも要件 1.1・1.6（自分のインクを覆う）と 1.4（隣のインクにかからない）を同時には満たせない。横書きも同じ形（大きい行の帯が次の行の帯へ食い込む）を今のまま持っている。**扱い**: 設計は「帯は自分の列の em の箱に置く」を通し、純粋の検査で重なりの量を測る。重なりが出たら完了とせず、量を添えて開発者の決めを仰ぐ（要件 4.3 と同じ扱い）。
- **横倒しの欧文の字（ASCII）**: 縦書きで横に倒す欧文の字は、インクが em の箱ではなく行の箱（ascent＋descent）に沿って乗る見込みで、em の丈の帯からはみ出し得る。SSP の測りは仮名と漢字だけ。ぱすたの選択肢は仮名と漢字。**扱い**: 読み戻しで 1 度だけ "Ag" の列のインクを測って記録する（合否にはしない）。はみ出したら開発者へ報告する（要件 4.2 の SSP の形＝em と要件 1.1 の食い違いになり得るため・4.3 と同じ扱い）。

### 設計判断（要件の討議から回された 7 つの決め）

1. **原因の置き場**: 「当て方」と「決め方」の両方。当て方＝右から左の列は行矩形の右辺に揃う（DirectWrite の段落の寄せ NEAR）のに、帯は左辺から右へ伸ばしていた。決め方＝厚みを横書きと同じ「行の箱を行送りで頭打ち」にしていた（SSP は em）。直しは `line_bands` の縦書きの枝で、厚みを em、寄せを「行矩形の左辺から em の箱の左辺までの符号つきの量」にする。em の箱の置き場の割合 c は最初のタスクの読み戻しで決める（候補 0・0.5・1。撮った物では 0 が合う）。どの c でも合わないとき（書体ごとに c が違う・インクが em の箱からはみ出す）は、行の箱の比だけでは決まらない＝`GlyphMetrics` に縦書きの em の箱の置き場を測って渡す口が要る（`layout.rs` の trait に既定の実装つきのメソッドを 1 つ）。C11 の約束の外の `layout.rs` に届くので、そこで止めて設計を直し、開発者に知らせる。
2. **直す形**: **案 B**（単一の源で符号つきの寄せ）。案 A（4 か所で鏡）は同じ鏡の式が 2 ファイルに散る。案 C（DirectWrite で列の箱を描くときに測る）は当たりの側へ測った値を流す配管が要り、規模が M に上がる。B は `derive_hit_rows`・`highlight_rect`・`glyph_cells` を書き換えずに済み、直すコードは `line_bands` の縦書きの枝と `expand_overhang_for_band` の `left` だけ。
3. **左から右の縦書き**: 右から左と同じ決まり（厚み em・寄せ round(c·s)）にする。今の式（丈 22・寄せ 2）のままにすると要件 4.2（SSP の形）と 2 つの向きで帯の形が違う。c ＝ 0 なら左から右の帯は行矩形そのもの。
4. **縦書きの帯の厚み**: em（列の em＝行矩形の幅＝その列で一番大きい字の em）。SSP の測り（9 節・帯 20＝em）に合わせる（要件 4.2）。列送りは em ＋行間なので、同じ大きさの列の帯は行間の分だけ離れる（要件 1.4・2.3）。
5. **消し残し**: `expand_overhang_for_band` の縦書きの枝で `left = max(実測の left, −offset)` を足す（`right` は今の式のまま）。横書きの枝は変えない。描き替えの範囲は `viewbox_diff.rs:330` の `x0 − overhang.left` が既に受ける。
6. **検査の置き場と書体**: 実フォントの読み戻しは `viewbox_draw_choice_hover_tests.rs`（COM が書ける）。書体は Yu Gothic UI（比 1.33・6.3）と、ぱすたのバルーン `pasta_tategaki` の `font.name`（游ゴシック Medium）を areka の本番と同じ解決で引いた物の 2 つ。大きさ 20 と 28、拡大率 1 と 2（ぱすたの机は 200%）。純粋の検査は `choice_decorate_tests.rs`（当たり＝塗りの数・隣の列・大きい字）と `choice_anchor_tests.rs`（アンカー）。`choice_tests.rs` には足さず、書き換えだけ。
7. **寄せの丸めと大きい字**: 丸めは横書きと同じ `round`（0.5 は 0 から遠い側）。右から左は `−round((1−c)·s)` で、左から右の `round(c·s)` の鏡になる。行矩形の辺は整数（em と行送りが整数）なので、帯の辺も整数の画素に乗る。分数の拡大率は今の ×k の道（`to_window_physical`・`SetTransform`・描き替えの範囲の余白）をそのまま通る。大きい字の混じる列は、列の em（＝一番大きい字）で厚みを決める。小さい字は同じ基線に乗るので大きい字の em の箱の中に入る見込み。隣の列との重なりは上の「大きさの違う列」の扱い。

### 案の比べ

| 案 | 中身 | 良い所 | 悪い所 | 決め |
|---|---|---|---|---|
| A | 使う 4 か所（当たり・字の矩形・塗り・消し残し）で右から左を鏡にする | `offset` の意味（近い辺から内へ）が変わらない | 同じ鏡の式が 2 ファイルに散り、1 か所を忘れると塗りと当たりがずれる | 採らない |
| B | `line_bands` が縦書きの帯を「行矩形の左辺からの符号つきの寄せ」で出す | 鏡の判断が 1 か所（描画とヒットの唯一の源のまま）・コードの直しは 2 か所 | `LineBand.offset` と `ChoiceLineContent.band_offset` の説明の書き換えが要る | **採る** |
| C | 描くときに DirectWrite で列の箱を測って帯に使う | 字の置き方が仮説と違っても実物に合う | 純粋層へ測った値を流す配管・行のキャッシュとの順序が変わる・規模 M | 採らない（判断 1 の行き止まりのときに口だけ作る案は残す） |

### まとめの 3 つの見方（synthesis）

- **一般化**: 「帯＝列の em の箱」は横書きの「帯＝行の箱の中央・行送りで頭打ち」と同じ形の関数（行ごとに厚みと寄せの 2 値を 1 度だけ決めて配る）で、`LineBand` の 2 値で両方を表せる。新しい型は作らない。
- **作るか借りるか**: 帯の置き場を DirectWrite に聞く口（`HitTestTextRange` など）は、当たりの側（純粋層）へ届かない。今の `line_box_height` だけで決まる式を作る。
- **削る**: 新しい関数・型・設定・ファイルは作らない。鏡の式は 1 か所。消し残しの直しは 1 行。

## 11. 設計の討議の決め（2026-10-23）

- 検証（`design-validation.md`）の判定は GO。指摘 2（c の決め方）と軽い気付き（倍率 1.5）は自明な直しとして design.md に入れた: c は読み戻しで測った値（0〜1）とし、向きで違えば定数を 2 つ持つ。`layout.rs` へ届く行き止まりは書体ごとに値が違うときだけ。消し残しの読み戻し S は倍率 1 と 1.5 で回す。
- 議題 1（大きい字の列が隣の列に食い込むとき・開発者「承認」＝推奨案）: 帯は自分の字を覆う方を先にする（要件 1.6 が 1.4 より先）。T1 で、`\f[height,40]` の列と隣の em 20 の列の字のインクそのものが重なるかを測る。重なるなら列の並べ方の崩れとして、本 spec の境界を広げて `layout.rs` の縦書きの列送りを直す（C11 のほかの 7 本は emo-text に触らない＝並走と重ならない）。重ならず帯だけが重なるなら、横書きと同じ形として受け入れ、重なりの量を P3 で固定する。要件 1.4 に「同じ大きさの列どうしの決まり」と書き添えた。
