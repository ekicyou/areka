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

## 8.4 ぱすたの手順書の段 2 の実機（要件 8.3）

`doc/pasta-real-run.md` の 3 節（準備）・4 節（起動）・5 節の段 2 を 1 回回した。結果は 3 件とも確かめられた。

### 同定と走行

- コミット `89309243`（debug 版）・書庫 `pasta-in-windows.nar` の sha256 `d55ba2407727467b835d1603059303cd8c76ea236ab86cfcb4a4b4baf9d4b7d2`。
- 回の名前 `ssts1`・2026-10-31 22:20〜22:22（日本時間。記録の時刻は UTC で 13:20〜13:22）・areka の pid 15096・終了コード 0（`\0\-` を送って areka が自分で終わった）。
- 4 節の起動の塊のまま。`RUST_LOG` だけに `areka_seriko=debug` を足した（`info,areka::boot_resolve=debug,areka::install=debug,areka_nar=debug,areka::input_events=debug,areka_seriko=debug`）。安全弁 `AREKA_APP_SMOKE_EXIT_MS=900000`。
- areka の MCP の待受は 9802（`skipped=[9801, 9821]`。SSP が起きていた）。台本を送る前に、areka にしか無い `check_script` が `OK:0 diagnostics` と答えることで、相手が areka であることを確かめた。SSP には触っていない。
- 今の書庫では、縦書きのバルーンがゴーストの中から選ばれた（`balloon_resolved route=Descript dir=...\ghost\pasta-in-windows\balloon\pasta_tategaki`）。
- 選択肢はクリックしていない（この回は SendInput の許可が無い）。30 秒の時間切れで閉じた。
- 数: panic 0・ERROR 0・WARN 100（内訳は下）・`char_code_dropped` 0。

### 3 件の結果

| 件 | 確かめた事 | 結果 | 根拠 |
|---|---|---|---|
| 17 全角の空白の埋め込み | 起動の台本の `\_u[0x3000]` が捨てられずに通るか | 確かめた | OnFirstBoot の台本（`13:20:12.887222Z` の `areka::log::script` の行）に `\_u[0x3000]` が入り、`char_code_dropped` の行は 0 件。見た目では確かめていない（ぱすたの台本はこの空白の直後に `\c` でバルーンを消すので画面に残らない）。選択肢テストの台本には `\_u` が無い |
| 3 選択肢の前の印 | `raise_event 選択肢テスト` の 4 つの `\![*]\q[...]` の前に印が出るか | 確かめた | 縦書きの 4 つの選択肢（晴れてる・雨だよ・曇ってる・ないしょ）のそれぞれの頭に既定の丸が出た（`2-choices.png`・`2-choices-dump_balloon.png`）。縦書きのバルーンは印の絵を持たないので、印の絵の info が scope 0 と 1 で 1 行ずつ出た（下） |
| 18 `bind-noevent` とカテゴリ単位の形 | `\![bind-noevent,…]` が `bind` と同じに効き、記録に `name="bind-noevent"` が出るか | 確かめた（補いの台本で） | ぱすた自身は段 2 で `bind-noevent` を出さなかった（出たのは `name="bind"` の行だけ。ぱすたが `bind-noevent` を出すのは `appearance.lua` の外見の戻しのときだけ）。そこで下の補いの台本を `check_script`（`OK:0 diagnostics`）の後に `sakurascript` で送り、記録の 4 行で確かめた |

件 18 の補いの台本:

```text
\0\![bind-noevent,閉じる,赤,1]\_w[3000]\![bind-noevent,閉じる,,0]\_w[3000]\![bind-noevent,閉じる,,1]\_w[3000]\![bind-noevent,閉じる,赤,0]\e
```

記録の 4 行（`ssts1.stdout.log` から逐語）:

```text
2026-10-31T13:21:54.497593Z  INFO actor{actor=seriko}: areka_seriko::actor::bind_arm: seriko: bind 適用 name="bind-noevent" scope=0 category=閉じる part=赤 id=1500 on=true
2026-10-31T13:21:57.494171Z  INFO actor{actor=seriko}: areka_seriko::actor::bind_arm: seriko: bind 適用 name="bind-noevent" scope=0 category=閉じる part=赤 id=1500 on=false
2026-10-31T13:22:00.495973Z  INFO actor{actor=seriko}: areka_seriko::actor::bind_arm: seriko: bind 適用 name="bind-noevent" scope=0 category=閉じる part=赤 id=1500 on=true
2026-10-31T13:22:03.494109Z  INFO actor{actor=seriko}: areka_seriko::actor::bind_arm: seriko: bind 適用 name="bind-noevent" scope=0 category=閉じる part=赤 id=1500 on=false
```

- 2 行目はカテゴリ単位の `,,0`、3 行目はカテゴリ単位の `,,1` の結果（カテゴリ「閉じる」はパーツが「赤」1 つだけ）。
- 撮った物は狙いの時刻とずれた。`3-bindnoevent-1-after-cat-on.png`（13:22:01.7 に書いた）はカテゴリ単位の `,,1` が効いている間で、× が赤い。`3-bindnoevent-2`〜`4-after-red-off.png`（13:22:04.9・08.2・15.9）はどれも最後の `赤,0` の後で、× は赤くない。最初の `赤,1` と `,,0` が効いている間の姿は撮れていないので、記録の 4 行で確かめた。

印の絵の info の記録（逐語）:

```text
2026-10-31T13:20:11.843465Z  INFO areka_emo_present::balloon_definitions: balloon_definitions: 印の絵が無い面がある（既定の丸を描く） balloon_dir=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\pasta-verify\root\ghost\pasta-in-windows\balloon\pasta_tategaki scope=0
2026-10-31T13:20:11.849411Z  INFO areka_emo_present::balloon_definitions: balloon_definitions: 印の絵が無い面がある（既定の丸を描く） balloon_dir=C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\pasta-verify\root\ghost\pasta-in-windows\balloon\pasta_tategaki scope=1
```

### WARN 100 行の内訳

| 対象 | 数 | 中身 | この spec のコードか |
|---|---|---|---|
| `areka_seriko::actor::bind_arm` | 96 | `seriko: mustselect カテゴリの脱衣指示を無視（正典・解除不可・bindopt 3.2） name="bind"`（目 36・口 30・うで 18・眉 12） | **対象のモジュールはこの spec のもの**（4.2 で腕を `actor_bind.rs` へ出した。`name` の欄もこの spec で足した）。ただし行そのものは前からある振る舞いで、ぱすたが mustselect のカテゴリに `\![bind,口,…,0]` などを出したときの正典どおりの無視。どれも `name="bind"` で、`bind-noevent` の行は 0 件 |
| `areka_emo_atlas` | 4 | `bake: element が全透明（α=0）でトリム後 0 寸です` が `balloonk0.png` と `pasta/null.png` で 2 行ずつ | この spec の外（ゴーストとバルーンの絵の中身） |

### 既知の件

- ぱすたの起動の台本（OnFirstBoot）は最後に面 1002（ボールと × だけ）を出し、ボールのクリックを待つ。なので、この回の撮った物には身体が映っていない（`mouse-click-wheel-events` の持ち分。手順書 5 節の「代わりの操作」の小節）。この回では代わりの操作は使っていない。

### 置き場

すべて `C:\home\maz\git\areka\.claude\worktrees\kiro-start-bridge-store-user-root-d1760a\target\pasta-verify\` の下。

- 記録: `logs\ssts1.stdout.log`・`logs\ssts1.stderr.log`・`logs\ssts1.pid.txt`・`logs\ssts1.exitcode.txt`
- 撮った物: `shots\ssts1\1-boot.png`・`2-choices.png`・`2-choices-dump_balloon.png`・`3-bindnoevent-1-after-cat-on.png`・`3-bindnoevent-2-after-red-off.png`・`3-bindnoevent-3-after-red-off.png`・`3-bindnoevent-4-after-red-off.png`
