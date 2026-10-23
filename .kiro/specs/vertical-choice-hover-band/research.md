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
