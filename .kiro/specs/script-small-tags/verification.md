# script-small-tags 検証の記録

## 8.1 目印の字が層の外へ漏れないことと、分かち書きの折り返し（要件 5.5・5.7）

### 目印の字（U+FFFC）の漏れ口 — 0 件

目印の字 `MARKER_GLYPH`（U+FFFC）を作るのは `crates/areka-emo-text/src/state.rs` の印の腕（`TextItem::glyph(MARKER_GLYPH)`）の 1 か所だけ。台本・記録・SHIORI へ渡る文字列は元の綴り（`\![*]`・`%*`）のままで、U+FFFC は文字の層の項目の列（`TextItem::Glyph`）とそこから組んだ行（`PositionedGlyph`）の中にしか居ない。`TextItem` と `PositionedGlyph` を `areka-emo-text` の外で読む所は無い（`crates` 全体の grep で 0 件）。層の中で字の文字列を読む所を全部見た。

| 読む所 | 外へ出るか | 根拠 |
|---|---|---|
| アンカーの題（`OnAnchorSelectEx` の Reference0） | 出ない | 6.2 で範囲だけ伸ばし題に足さない形にした（`state.rs` の印の腕の `extend_open_anchor_range(1)`）。`state_marker_tests.rs` の `!spans[0].label.contains(MARKER_GLYPH)` が固定 |
| 選択肢の題（`OnChoiceSelect(Ex)`） | 出ない | 題は `\q` の合図の文字列から取る（`state.rs` の `Choice` の腕の `label: text.clone()`）。目印の字は合図の前に別の項目として積むので題に入らない |
| 測り（`layout_styled.rs`） | 出ない | `stand_in` で全角の空白に替えて測る。`layout_styled_tests.rs` が固定 |
| 描き（`draw.rs:665`・`viewbox_draw_render.rs:83/98/206`） | 出ない | 行の文字列は `inline_marker::line_text`（目印の字だけ全角の空白に読み替える）で組む（6.4）。`inline_marker_tests.rs` が固定 |
| 装飾の範囲（`viewbox_draw_decoration.rs:71`・`viewbox_draw_render.rs:777`） | 出ない | UTF-16 の単位の数を数えるだけ（U+FFFC も全角の空白も 1 単位） |
| 行の指紋（`viewbox_diff.rs:257/268`） | 出ない | 描き直しの要否を比べる内部の値。記録にも MCP にも出さない |
| 分かち書き（`segment.rs`） | 出ない | budouy へ渡す run の文字列に入るが、返るのは塊の長さだけで外へは出ない（下の節） |
| 行の写し（`canvas.rs:327`） | 出ない | 行を行の座標へ移すだけ |
| 記録（`tracing` の各行） | 出ない | `areka-emo-text` の記録の行で字・項目・行・題を値として出す行は無い（grep で 0 件） |
| `dump_balloon` の答え | 出ない | 画素の読み戻し（`crates/areka/src/mcp/dump_balloon.rs`）。字の文字列は答えに入らない |

設計（「実装の段で確かめること」）が塞いだアンカーの題のほかに、漏れ口は見つからなかった。

### 分かち書き（budouy）が目印の字の前後に置く改行の候補

budouy 0.2.2 の既定の日本語モデルで確かめた（`segment.rs` の `budouy_never_breaks_right_after_the_marker_glyph` が固定）。

| 入力（￼＝U+FFFC） | 塊 |
|---|---|
| `￼はい` | `￼は`｜`い` |
| `￼はい￼いいえ` | `￼は`｜`い`｜`￼いいえ` |
| `￼￼はい` | `￼￼は`｜`い` |
| `どうする？￼はい` | `どうする？`｜`￼は`｜`い` |
| `今日は￼いい天気ですね` | `今日は`｜`￼いい`｜`天気ですね` |
| `￼Yes` | `￼Yes` |
| `￼選択肢その一` | `￼選択肢`｜`その`｜`一` |
| `天気￼晴れ` | `天気￼晴れ` |
| `あ￼` | `あ￼` |

- budouy は目印の字の**直後**に塊の境を置かない。印は後ろの字と同じ塊に入るので、分かち書きの折り返しでは印と選択肢の頭の字は別の行に分かれない。
- 境は印の**前**には置かれうる（`どうする？`｜`￼は`）。このとき印は選択肢の字と一緒に次の行へ送られる。
- 別の行に分かれうるのは、分かち書きに依らない折り返しのときだけ: 塊が 1 行より長くて字単位の折り返しに縮退したとき（`layout.rs` のゲート③の長い塊の縮退）と、`validrect` の無条件の折り返し。
- SSP の折り返しは測っていない。

互換の対応表 §8 にはまだこの spec の行が無い（8.3 で足す）。上の結果（印は後ろの字と同じ塊・別の行に分かれるのは長い塊の縮退と `validrect` の無条件の折り返しのときだけ・SSP は測っていない）は 8.3 で §8 の行に書く。
