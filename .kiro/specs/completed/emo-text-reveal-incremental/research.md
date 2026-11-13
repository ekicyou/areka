# ギャップ分析: emo-text-reveal-incremental

> 2026-11-13・`main` `4863cb66` の上の作業枝（`3d057cb9`）で測った。要件（requirements.md）の確定版に対する、今のコードとの差。決めは書かず、選択肢と確かめたことだけを書く。

## 1. 分析の要約

- **件 1 は「前の区切りを残して後ろだけ区切り直す」で結果を変えずに直せる（コードで確かめた）。** `budouy` 0.2.2 の `parse_boundaries_from_chars` は字 `i` の前の境目を `chars[i-3..i+3)` の 6 字だけで決める。後ろに字が足されて変わりうるのは、前の字の列の終わりの 2 か所（`i = 旧の長さ−2`・`旧の長さ−1`）と新しい位置だけ。区切り直す範囲の頭から 3 字前の字を左の文脈として渡せば、公開の口（`Parser::parse` か `Parser::parse_boundaries`）だけで全文を区切ったのと同じ境目が出る。フォークも `[patch]` も要らない。
- **件 2 は 2 つの手間に分かれる。** (a) 行き先の替わりの写し（`copy_script` → `take_script`）は、指定の列（`applied`）を丸ごと複製し（`clone`）、丸ごと当て直す（`rebase`）。(b) 2 層の差し込み（`set_look_layers`・`set_box_traits`）は、値が前と同じでも丸ごと当て直す。(b) は値を比べて飛ばすだけで直る。(a) は「当て直しを差分にする」だけでは足りず、列の複製（と記録済みの集合 `warned` の複製）も総数に比例するので、列の持ち方を変える必要がある。
- **件 3 の解決は 3 か所から呼ばれるが、警告を書く場所は 2 か所で足りる。** 普通のバルーンは装着と再追従が合流する `finish_registration`／`remember_inputs`（`actor_attach.rs`）、箱は `set_box_layout`（`actor_box.rs`・箱の名前ごとに 1 度の解決）。`register_box` は覚えた解決（`box_resolved`）を読むだけで、解決を呼ばない（brief の「`register_box`」は誤り・要件は `set_box_layout` に直してある）。警告の `value` の欄は生の値を要るので、`Copy` の `WritingDirectionDecision` には載らず、モデルを持つ口で書くことになる。
- **行数**: 触る見込みのファイルはどれも 1,000 行の内に収まる見込み。`actor_present.rs`（896 行）は余白が小さいので、区切りを伸ばす中身は `segment.rs`（566 行）か新しいファイルへ置くのが無理がない。`look.rs`（998 行）は件 2 のどの案でも触らずに済む。
- **研究の要る点**: 列を場所の間で分け合う形（`Rc`／`Arc`・場所ではなくスコープに持たせる）と `Decoration` の `PartialEq`・`rehearsal_copy` の複製との兼ね合い。檻を GPU の要らない純粋なテストで置ける位置（`BreakMemo::plan` は今は外から呼べない）。

## 2. 今の形（確かめたこと）

### 2.1 件 1: 届いた字の区切り

| 項目 | 今の形 | 場所 |
|---|---|---|
| 区切りの覚え | `BreakMemo { last: Option<((消した回数, 項目の数), SegmentPlan)> }`。鍵が外れたら `segment_plan(items)` で全部を区切り直す | `actor_present.rs` 766〜787 行 |
| 区切りの本体 | `segment_plan` は `LineBreak`・`CursorMove` で区切ったひと続き（run）ごとに `parser.parse(run_text)` をかけ、塊のバイト長を `assign_items_to_chunks` でグリフ（書記素クラスタ）へ写す | `segment.rs` 102〜196 行 |
| 呼び手 | `arrange_lines` の `Basis::Arrived` の腕（文節の折返しで先渡しが無い・食い違う）だけ。`CharByChar` は区切らない。`Basis::Full` は先渡しの側の区切り（`lookahead.rs` の `Section::plan` を 1 度だけ） | `actor_present.rs` 736〜745 行 |
| 覚えの持ち主 | `ActorRender::break_memo`（場所ごと）。`ActorRender` は箱の表の替え（`set_box_layout`）・再追従の作り直し（`refresh_actor_binding` の下の段）・拡大率と装置の作り直しで捨てられる | `actor.rs` 206 行・`actor_attach.rs`・`actor_box.rs` 52〜62 行 |
| 項目の増え方 | `apply_cue` の `Text`・`Choice`・`NewLine`・`Cursor`・印の字は**後ろへ足すだけ**。消えるのは `clear_content`（`Clear`・`ClearAll`）だけで、そのとき `clears` が必ず 1 進む。グリフは cue ごとに `clusters(text)` で切るので、後から届いた字が前のグリフの中身を書き換えることは無い | `state.rs` 538〜660 行・`state_decoration.rs` 246 行 |
| 今の試験の数え | `BreakMemo::runs`（`#[cfg(test)]`・区切り直した回数）。`actor_frame_memo_tests.rs` の 3 本が「項目が同じなら区切り直さない（1 のまま）」「項目が増えたら 2」「消して同じ数を書き直したら区切り直す」を固定している（GPU の World を使う） | `actor_frame_memo_tests.rs` 267〜336 行 |
| 見た数の数えの先例 | `layout_scan.rs` の `FOLLOWING_SCAN_VISITS`（`#[cfg(test)] thread_local!` の `Cell<usize>`） | `layout_scan.rs` 15〜20 行 |
| 検査の読み口 | `arrange_for_test` は呼ぶたびに `BreakMemo::default()` を渡す（覚えを使わない） | `actor.rs` 686〜700 行 |

**`budouy` 0.2.2 の確かめ（要件の論点 (a)）**: 登録簿の写し（`C:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\budouy-0.2.2\src\parser.rs`）を読んだ。

- `parse(&str)` も `parse_boundaries(&str)`（公開・字の番号で返す）も、`chars` を集めて `parse_boundaries_from_chars` を呼ぶ。
- `for i in 1..len` の各 `i` で、得点は `base_score`＋13 の特徴の重みの和。特徴が読む字は `UW1`＝`[i-3]`・`UW2`＝`[i-2]`・`UW3`＝`[i-1]`・`UW4`＝`[i]`・`UW5`＝`[i+1]`・`UW6`＝`[i+2]`・`BW1`＝`[i-2..i)`・`BW2`＝`[i-1..i+1)`・`BW3`＝`[i..i+2)`・`TW1`＝`[i-3..i)`・`TW2`＝`[i-2..i+1)`・`TW3`＝`[i-1..i+2)`・`TW4`＝`[i..i+3)`。端は `saturating_sub` と `min(len)` で**切り詰める**（切り詰めた短い綴りは別の鍵になる）。
- よって境目 `i` は `chars[i-3..i+3)`（`i−3`〜`i+2` の 6 字）と、左端・右端への近さだけで決まる。旧の長さ `L` の後ろへ字を足すと、右端の切り詰めが外れるのは `i+3 > L` の位置＝`i ≥ L−2` だけ。`i ≤ L−3` の境目は動かない。
- 左の文脈: 区切り直す頭を `p = max(0, L−2)` とすると、`s = max(0, p−3)` から後ろの字の列を渡せば、`i ≥ p` の位置の 13 の鍵は全文のときと同じになる（`s = 0` なら左端も同じ）。右端は全文と同じ所で終わる。したがって `parse_boundaries(&run[s..])` の `i−s ≥ p−s` の境目は、全文を区切った境目と一致する。
- 境目から塊への写し（`assign_items_to_chunks`）は、グリフ `j` が新しい塊の頭になるかを「`j−1` の頭の字と `j` の頭の字の間に境目があるか」で決める。`j` の頭の字が `p` より前なら、その間の境目はどれも動かない。動きうるのは頭の字が `p` 以降のグリフ（各グリフは 1 字以上なので、旧の末尾から高々 2 個）と、その直前の塊の長さだけ。
- run（改行・`\_l` で区切ったひと続き）は独立に区切るので、閉じた run の区切りは後から変わらない。新しく届いた項目に改行か `\_l` があれば、開いていた run はそこで閉じ、その時点の右端で末尾 2 か所を決め直す。

### 2.2 件 2: 見た目の当て直し

| 項目 | 今の形 | 場所 |
|---|---|---|
| 持つもの | `Decoration { layers, current, base, applied: Vec<Vec<String>>, unowned, warned }`（場所ごと・`Clone`・`PartialEq`） | `state_decoration.rs` 96〜136 行 |
| `\f` 1 件 | `apply_font_args` が `current` へ積み増し、成功した指定だけ `applied` へ足す（当て直さない） | 同 343〜430 行 |
| 一括の戻し | `reset_look_to`（`\f[default]`・`\f[disable]`・`reset_for_new_talk`・`reset_decoration`）が `applied` を空にし `base` を替える | 同 309〜330 行 |
| 写し | `carry_script_decor` → `copy_script`: 元の場所の `decor` を**丸ごと `clone`**（`applied`・`unowned`・`warned`・`layers`）し、`take_script` が 4 欄を再び `clone` して `rebase`（`base_look()` から `applied` を全部当て直す） | 同 536〜577 行・163〜170 行 |
| 写しの呼び手 | `set_dest`（`\s`・`\b[名前]`・シェルの戻り先）・`fold_shared_into_balloon`（`reset_routes`＝台本の先頭とシェルの切替・`set_box_index`） | `state_route.rs` 65〜218 行 |
| 2 層の差し込み | `set_look_layers`（`register_actor` から・装着と再追従の下の段で毎回）と `set_box_traits`（`set_box_layout` から・今ある箱の場所ごと）が、値を比べずに `rebase` | `state_decoration.rs` 491〜502 行・590〜594 行・`actor_attach.rs` 178〜179 行・`actor_box.rs` 124 行 |
| `LookLayers` の比べ | `#[derive(Clone, Debug, PartialEq)]`（比べるのに `look.rs` を触らなくてよい） | `look.rs` 222〜223 行 |
| 空回し | `rehearse_again`（装着・箱の表の受け取り）が `TextLayerState` を写して台本を空回しする。写しの中でも同じ写しと当て直しが走る | `actor.rs` 667〜671 行・`state.rs` 471〜476 行 |

**論点 (b) の確かめ（差分だけ当てて結果が同じか）**:

- スコープに従う場所に `\f` が入るのは、その場所が持ち主（`route.shared`）のときだけ（文字と `\f` の宛先は今の行き先で、従う場所へ替わるときは必ず写して持ち主にする）。箱に閉じる箱（`FontFollow::Box`）は写しの元にも先にもならない（`carry_script_decor` が早く戻る・`shared` を替えない）。
- だから、一括の戻しが間に無い限り、持ち主から戻ってきた列は「その場所が前に持っていた列」をそのまま頭に含む（頭が一致する）。その場所の `current` は「自分の `layers` の上で自分の列を全部当てた結果」に等しい（`layers` が替わればその場で当て直すので）。頭の一致を確かめられれば、足りない後ろの分だけを今の `current` へ当てた結果は、全部当て直した結果と同じになる（`apply_font_tag` は `current`・`layers`・トークンだけで決まる。相対値も順に当てる点で同じ）。
- ただし「頭が一致するか」を列の比べで確かめると、それ自体が総数に比例する。一括の戻しのたびに新しい番号を振る（戻しの世代）などで、（世代, 当てた数）の比べにする必要がある。`base` は戻しのときだけ替わるので世代で決まる。
- 一括の戻しの後に戻ってきた場合（世代が違う）は、その場所はその世代の指定をまだ 1 件も当てていないので、全部を 1 度ずつ当てる。これは要件 2.1 の「まだ当てていない指定だけ」に当たる。
- 戻しは場所ごとにも全部の場所にも起きる（`\f[default]` は今の行き先だけ・`ClearAll` と `reset_decoration(None)` は全部の場所）。持ち主でない場所が自分だけ戻されることもある（全部の場所の戻し）。世代は場所をまたいで一意の数（場所ごとの数え直しでは衝突する）にする必要がある。
- **差分の当て直しだけでは足りない**: `copy_script` の `state.decor.clone()` と `take_script` の `from.applied.clone()` が列の長さに比例する。`warned`（記録済みの（種別, キー, 値の列））も、不正な指定が多い台本では指定の数に近づき、写しのたびに複製される。要件 2.1 の「1 回の写しの手間（指定の列を写すことと当て直すこと）」を満たすには、列を写さない形（分け合う・スコープに 1 つだけ持つ）が要る。`warned` を数えに入れるかは要件の言葉からは読み切れない（下の議題 5）。

### 2.3 件 3: 書字方向の宣言の警告

| 項目 | 今の形 | 場所 |
|---|---|---|
| 記録を出す所 | `classify_vertical` と `classify_writing_mode` の `warn!`（`value` の欄に生の値）、`resolve` の併記の腕の `debug!`（`vertical`・`writing_mode` の生の値） | `writing.rs` 195〜234 行・289〜313 行 |
| 解決の呼び口 | `WritingMode::resolve` → `WritingDirectionDecision::resolve`。本番で呼ぶのは `ResolvedBalloonText::resolve_with_background` の 1 か所（`actor_decoration.rs` 214 行） | `writing.rs` 86〜88 行 |
| 普通のバルーン | 装着 `register_actor_binding` → `resolve_stamped` → `finish_registration` → `register_actor`＋`remember_inputs`。再追従 `refresh_actor_binding` は 3 段: 上の段（入力も結線も同じ）は解かない／中の段（入力は違うが解いた値が同じ）は解いて `remember_inputs` だけ／下の段は解いて `finish_registration` | `actor_attach.rs` 219〜290 行・339〜431 行 |
| 箱 | `set_box_layout` が箱の名前ごとに 1 度だけ `resolve_with_background(&def.model, …)` を呼び、`box_resolved` に覚える。`register_box` は覚えを読むだけ | `actor_box.rs` 45〜125 行・419〜482 行 |
| 「値が新しく決まったときだけ」の先例 | ① `warn_coarse_wrap_threshold`（`register_actor` の中・前の値は `layout_input` の領域・値は解決の結果）／② `warn_cursor_fallback`（`remember_inputs` の中・前の値は `resolve_inputs` の**前のモデル**から読み直す・`register_actor` が記憶を消す前に取り出す） | `actor_attach.rs` 47〜96 行・60〜85 行（`remember_inputs`）・`actor_decoration.rs` 165〜195 行 |
| `region.rs` の `debug!` の先例（論点 (c)） | `region.rs` は `warn!` を登録口へ移したが、未指定の成分の縮退の `debug!`（`resolve_or` 439 行・`origin` 481〜495 行）は「別の症状の記録」として解決の中に残した。件 3 の併記の `debug!` は宣言の誤りと同じ系統の記録なので、要件 3.1・3.5 どおり警告と一緒に移すのは先例と食い違わない。ただし `region.rs` の `debug!` は今後も再追従の中の段・下の段で繰り返し出る（この spec の範囲の外） | `region.rs` 247〜271 行・434〜495 行 |
| `crates/areka/` の呼び手 | `frame/attach.rs` 493 行（`register_actor_view`）・`frame/scale_text.rs` 135 行（`refresh_actor_scale`）・`frame_balloon_runtime_test_support.rs` 48 行（`register_actor` を直に）。警告の文言を grep している所は `crates/` 全体で `writing.rs` と `writing_decision_tests.rs` だけ | — |
| 試験 | `writing_decision_tests.rs` は `#[test]` 11 本（`count_levels` で件数・`assert_capture_alive` で捕捉の生存）、`writing.rs` の tests は 4 本。どちらも `PURE_SOURCES` に載る。登録口を通す試験の先例は `actor_region_warn_tests.rs`（World は予約スロットの entity のためだけ・GPU は要らない・`SOURCES_OUTSIDE_THE_PURE_SCAN` に載る） | `lib.rs` 233〜236 行・`actor_region_warn_tests.rs` 140〜170 行 |

**論点 (c) の確かめ**:

- 書字方向は `set_box_layout`（箱）と `resolve_stamped`（普通のバルーン）で解かれる。`register_box` では解かれない。
- 普通のバルーンで、中の段は「モデルは変わったが解いた値は同じ」で、`register_actor` に届かない。たとえば `vertical` が `0` から `2` に変わっても書字方向は横書きのまま（`ResolvedBalloonText` に分類は載っていない）なので中の段になる。分類は変わっている（`Horizontal` → `Invalid`）ので要件 3.2 では警告が要る。**警告を `register_actor` に置くと、この場合に記録が出ない**（要件 3.7 に触れる）。`remember_inputs` は装着・中の段・下の段のどれでも通るので、そこに置けば漏れない。
- 直の登録口 `register_actor`（公開・呼び手は `crates/areka/` の試験の支えだけ）はモデルを受け取らない。解決が記録を出さなくなると、この道で不正な宣言を渡しても記録は出ない。要件 3.7 の「登録口」にこの道を含めるかで形が変わる（下の議題 7）。

### 2.4 行数と一覧（論点 (d)）

| ファイル | 今の行数 | この spec の見込み |
|---|---|---|
| `actor_present.rs` | 896 | 件 1。区切りを伸ばす中身をここに書くと 1,000 行に近づく。`BreakMemo` を `segment.rs` 側へ寄せれば数行の差 |
| `segment.rs` | 566（うち 207 行目から下はファイルの中の tests） | 件 1 の中身の置き場の候補。新しいテストは兄弟ファイルへ（structure.md の決まり） |
| `state_decoration.rs` | 649 | 件 2。列の持ち方を変えると +50〜100 行の見込み |
| `state_route.rs` | 234 | 件 2 で写しの呼び方が変われば触る |
| `state.rs` | 789 | 件 2 で戻しの世代の数えを `TextLayerState` に持たせるなら触る |
| `writing.rs` | 400 | 件 3。記録を出す関数を足しても余る |
| `actor_attach.rs` | 439 | 件 3（普通のバルーンの登録口） |
| `actor_box.rs` | 536 | 件 3（箱の登録口） |
| `actor_decoration.rs` | 331 | 件 3 で記録の関数をここに置く案もある（`warn_cursor_fallback` と同じ置き場） |
| `actor.rs` | 917 | 件 3 で `ResolvedBalloonText` に欄を足す案なら +数行 |
| `writing_decision_tests.rs` | 361 | 件 3 で登録口の側へ移す（移した先が World を使うなら `SOURCES_OUTSIDE_THE_PURE_SCAN` 側のファイルになる） |
| `actor_frame_memo_tests.rs` | 336 | 件 1 の `runs` の 3 本（期待値を保つ） |
| `look.rs` | 998 | どの案でも触らずに済む見込み（`LookLayers` の `PartialEq` は既にある・`apply_font_tag` は呼ぶだけ） |

- `lib.rs` の一覧: `PURE_SOURCES`（`assert_eq!(PURE_SOURCES.len(), 88, …)` の母数の行も直す）と `SOURCES_OUTSIDE_THE_PURE_SCAN`。和が実ファイルの集合と突き合わされる（`every_source_file_is_either_scanned_or_explicitly_excluded`）ので、新しいファイル（テストのファイルを含む）はどちらかへ必ず載せる。今のファイル数は 152。
- 純粋なファイル（`segment.rs`・`state_*.rs`・`writing.rs` とその兄弟テスト）は `PURE_SOURCES`、World や COM を使うテストは `SOURCES_OUTSIDE_THE_PURE_SCAN`。

## 3. 要件と今の形の対応

| 要件 | 今の形 | 差 |
|---|---|---|
| 1.1 区切り直す字の数を定数に | 全部を区切り直す | **Missing**: 伸ばす形 |
| 1.2 そのフレームで全部を区切った結果と同じ | 同じ（全部を区切るので） | **Constraint**: 伸ばす形でも §2.1 の窓の規則で保てる |
| 1.3 消えた・覚えが捨てられたら作り直す | 鍵の（消した回数, 項目の数）で作り直す | 鍵を「（消した回数）が同じで項目が増えた」に読み替える。覚えは再追従の作り直し・拡大率の替わりでも捨てられる（要件は箱の表の替えだけを挙げている＝下の議題 2） |
| 1.4 1 字ずつ・全文の道は今のまま | 腕が分かれている | 差なし（`Basis::Arrived` の腕だけを触る） |
| 1.5 見た数の数えと 2 乗の檻 | `runs`（回数）だけ・字の数は数えていない | **Missing**: 字の数の数え・数千字の檻。GPU の要らない位置で回す口が要る（下の議題 3） |
| 1.6 `\n`・`\_l`・`\c` を含む各時点の一致 | 無い | **Missing**: 前と後を比べる決定論のテスト（`segment_plan(全部)` を正とする） |
| 2.1 写しを総数に比例させない | 列を複製して全部当て直す | **Missing**: 列を写さない持ち方＋世代と数での差分 |
| 2.2 同じ値の 2 層は当て直さない | 毎回当て直す | **Missing**: `LookLayers` を比べて飛ばす（1 行級） |
| 2.3 違う値なら 1 度ずつ当て直す | 全部当て直す（今と同じ） | 差なし |
| 2.4 当て直しは 2 層の替わりでだけ | 写しでも当て直す | 2.1 と同じ直しで満たす |
| 2.5 今の見た目＝土台から全部当て直した結果 | 写しのたびに作り直すので成り立つ | **Constraint**: 差分にしても成り立つことを試験の側の「全部当て直す」形と比べて確かめる |
| 2.6 戻し・`\c`・記録を今のまま | — | **Constraint**: `warned` を写す形を変えるなら記録の件数が変わらないこと |
| 2.7 写しと当て直しの数えと檻 | 無い | **Missing** |
| 2.8 混ぜた台本で全部当て直す形と一致 | 無い | **Missing**: 試験の側に今の `rebase` 相当の正を残す |
| 3.1 解決は記録を出さない・結果は今のまま | 解決が記録を出す | **Missing**: `classify_*` と `resolve` から記録を外す |
| 3.2・3.3 新しく決まったときだけ 1 件 | 解くたびに出る | **Missing**: 前の分類と比べて書く口（§2.3） |
| 3.4 箱の名前ごとに 1 件 | 同じ（`set_box_layout` が名前ごとに 1 度解く） | 書く場所を解決の外（`set_box_layout` の中）へ移すだけ |
| 3.5 併記の `debug!` も同じ規則 | 解くたびに出る | 3.2 と同じ口へ |
| 3.6 試験の移し替え | 解決を直に呼んで数える 15 本 | **Missing**: 登録口を通す試験・解決が記録しない試験・再追従で繰り返さない試験 |
| 3.7 記録の無い不正な宣言の道を作らない | 解決が必ず出す | **Constraint**: 中の段と直の `register_actor` の扱い（§2.3） |
| 4.3 `crates/areka/` に触らない・公開の形は足すだけ | — | **Constraint**: `ResolvedBalloonText` は全部の欄が公開。欄を足すと外の構造体の字面の組み立てが壊れうるが、組み立てているのは `areka-emo-text` の中だけ（`crates/areka/` は `resolve` 経由のみ）だった |
| 4.5 1,000 行 | §2.4 | **Constraint**: `actor_present.rs` の余白 |
| 4.6 `budouy` をそのまま | 公開の `parse`・`parse_boundaries` で足りる | 差なし |

## 4. 実装の選択肢

### 件 1

- **案 1-A（伸ばす・既にある型を拡げる）**: `BreakMemo` の鍵を（消した回数, 覚えた項目の数）にし、同じ消した回数で項目が増えたら、覚えた `SegmentPlan` の末尾 2 字ぶんの塊を外し、旧の末尾から 5 字前（左の文脈 3 字＋決め直す 2 字）から後ろだけを `budouy` にかけて足し直す。旧の末尾の近くの字は `items` から後ろ向きに高々数個読めば足りる（各グリフは 1 字以上）ので、新しく持つ状態は要らない。中身は `segment.rs` に `SegmentPlan` を伸ばす関数として置き、`BreakMemo` はそれを呼ぶだけにする。
  - ✅ 新しい状態がほぼ要らない・`SegmentPlan` の形と呼び手はそのまま・`actor_present.rs` の行数が増えない
  - ❌ 塊の割り当て（`assign_items_to_chunks`）の「直前の塊の長さを詰め直す」所が細かい。run の閉じ（改行・`\_l` が新しく届いた）の腕が要る
- **案 1-B（開いている run の状態を持つ）**: 覚えに「開いている run の頭のグリフ番号・字の列（`Vec<char>` か `String`）・グリフごとの字の数」を持ち、伸ばしはその状態の上で行う。
  - ✅ 後ろ向きに `items` を読み直さない・処理の筋が素直
  - ❌ run の長さの状態を持つ（字の数に比例する記憶が 1 つ増える）・`Clone` の手間
- **案 1-C（run ごとの覚えだけ）**: 閉じた run の区切りを覚え、開いている run だけを毎回全部区切る。
  - ✅ いちばん小さい
  - ❌ 1 つの run が長い（改行の無い長い台詞）と 2 乗が残り、要件 1.5 の檻（1 つのひと続きで数千字）が赤のまま。**要件を満たさない**
- 置き場: 案 1-A・1-B とも、中身を `segment.rs`（純粋層）に置けば、`budouy` を使う所が `segment.rs` だけという今の決まり（モジュールの説明「budouy 消費の唯一の場所」）を保てる。新しいファイル（例: `segment_grow.rs` と兄弟テスト）に分ける案もある（このウェーブで emo-text に新しいファイルを足せるのはこの spec だけ）。

### 件 2

- **案 2-A（差分＋列を分け合う・今の場所ごとの形を保つ）**: `applied` を場所の間で分け合える持ち方（`Rc<…>`／`Arc<…>` の追記だけの列と、場所ごとの「当てた数」）にし、一括の戻しのたびに場所をまたいで一意の世代を振る。写しは（世代, 当てた数）を比べ、同じ世代なら後ろの分だけ、違えば全部を 1 度ずつ当てる。2 層の差し込みは `LookLayers` を比べて同じなら飛ばす。
  - ✅ 今の「場所ごとの `Decoration`」と呼び手（`state_route.rs`）の形をほぼ保つ
  - ❌ 分け合う列と `PartialEq`（`forget_pristine_places` の「作りたてと同じか」・テストの比べ）の意味を揃える工夫が要る。`unowned`・`warned` の写しは別に考える
- **案 2-B（スコープに 1 つだけ持つ）**: 台本の指定の 4 欄（`base`・`applied`・`unowned`・`warned`）を場所から外し、スコープ（`ScopeRoute` の隣）に 1 つだけ持つ。場所は `layers`・`current` と「どの世代の何件目まで当てたか」だけを持つ。箱に閉じる箱だけは自分の 4 欄を持つ。
  - ✅ 写しそのものが消える（`copy_script` は「当てた数の追いつき」になる）。`warned` の複製の問題も消える
  - ❌ `state_decoration.rs`・`state_route.rs`・`state.rs` と、4 欄を直に見ているテスト（`state_decoration_carry_tests.rs` ほか）の書き換えが大きい。持ち主（`shared`）の考え方が変わる
- **案 2-C（2 層の差し込みだけ直す）**: `set_look_layers`・`set_box_traits` で値が同じなら飛ばすだけ。
  - ✅ 1〜2 行
  - ❌ 行き先の替わりの S × N が残り、要件 2.1・2.7 を満たさない。**要件を満たさない**（2-A・2-B のどちらでも、まずこれを入れる）

### 件 3

- **案 3-A（`remember_inputs` に置く・`warn_cursor_fallback` の先例）**: 解決から記録を外し、`writing.rs` に「モデルと前のモデル（無ければ `None`）から、分類が新しく決まったときだけ記録する」純粋な関数を置く。普通のバルーンは `remember_inputs`（装着・中の段・下の段のどれも通る）から、前の値は `resolve_inputs` の前のモデルで呼ぶ。箱は `set_box_layout` の解決の隣で `None` を前として呼ぶ。
  - ✅ 生の値をモデルから取れる・中の段の漏れが無い・`Decision` の `Copy` を保つ・公開の形が変わらない
  - ❌ 直の `register_actor` の道では記録が出ない（議題 7）。前のモデルの分類を毎回分類し直す（軽い）
- **案 3-B（`register_actor` に置く・`warn_coarse_wrap_threshold` の先例）**: `ResolvedBalloonText` に分類（と生の不正な値）を載せ、`register_actor` で `layout_input` の前の値と比べて書く。
  - ✅ 直の登録口も含め、登録口が 1 つに見える
  - ❌ 分類が `ResolvedBalloonText` の比べに入るので、今は中の段で止まる再追従が下の段（描画資源の作り直し）に変わる場合が出る（結果は同じでも作り直しが増える）。生の値を載せると `ResolvedBalloonText` が重くなる。公開の欄が増える
- **案 3-C（解決に「黙る」版を足す）**: 解決の中身はそのままに、記録を出す版と出さない版を分け、再追従だけ黙る版を使う。
  - ✅ 小さい
  - ❌ 要件 3.1（解決の中では記録を出さない）に合わない。中の段・下の段で分類が替わったときの記録の規則（3.2）を別に持つことになる。**要件に合わない**

## 5. 規模と危うさ

- **規模: M（6〜9）** — 件 1 は手順の細かい所（塊の割り当ての詰め直し・run の閉じ）と前と後の一致の檻で 2〜3、件 2 は列の持ち方の変更と世代と檻で 3〜4（案 2-B なら +1〜2）、件 3 は記録の口の移しと 15 本の試験の移し替えで 2。brief の見込み（S〜M・6〜11）の内。
- **危うさ: 中** — 件 1 は `budouy` の窓の規則をコードで確かめたので、結果の一致は決定論のテストで固められる（低）。件 2 は「頭の一致」を世代で言い換える所に見落とし（場所ごとの戻し・`fold_shared_into_balloon`・空回しの写し）があると、見た目が黙って変わる（中）。件 3 は記録の件数が変わる所なので、中の段と直の登録口の扱いを決めれば低。

## 6. 設計の段への申し送り

### 議題（要件の討議へ）

1. **件 1 の伸ばし方（案 1-A／1-B）**: 旧の末尾の近くを `items` から読み直すか、開いている run の状態を覚えに持つか。どちらも結果は同じ。置き場を `segment.rs` にするか新しいファイルにするか。
2. **件 1 の作り直しの時（要件 1.3 の列挙）**: 覚えは箱の表の替えのほか、再追従の作り直し（下の段）・拡大率と装置の作り直しでも `ActorRender` ごと捨てられ、次のフレームで全部を 1 度区切り直す。結果は同じで回数も出来事 1 回につき 1 度。要件 1.3 の「区切りの覚えが捨てられる」にこれらも含むと読んでよいか。
3. **件 1 の檻の位置（要件 1.5）**: 今の `BreakMemo::runs` の檻は GPU の World を使う `actor_frame_memo_tests.rs` にある。数千字を 1 字ずつ届ける檻は、`BreakMemo`（か `segment.rs` の伸ばす関数）を純粋に呼ぶテストで置くのが軽い。字の数の数えは `segment.rs` の `budouy` に渡す所で数える（`FOLLOWING_SCAN_VISITS` と同じ形）。あわせて `runs` の意味を「区切りを計算した回数（伸ばしも 1 回）」に保ち、今の 3 本の期待値（1・2・作り直し）を変えないか。
4. **件 2 の持ち方（案 2-A／2-B）**: 列を場所の間で分け合うか、スコープに 1 つだけ持つか。2-B は写しが消えて素直だが書き換えが大きい。
5. **件 2 の数えに `warned`・`unowned` の写しを含めるか（要件 2.1・2.7）**: 要件は「指定の列を写すこと」を挙げる。`warned` は不正な指定の数だけ伸び、今は写しのたびに複製される。案 2-A を取るなら、`warned` も分け合うか、数えの外に置くかを決める（2-B なら問題にならない）。
6. **件 3 の記録の口（案 3-A／3-B）**: 普通のバルーンは `remember_inputs`（`warn_cursor_fallback` の先例・中の段も通る）か `register_actor`（`warn_coarse_wrap_threshold` の先例）か。`register_actor` に置くと、`vertical` が `0` から `2` に変わる再追従（書字方向は同じ）で記録が出ない（要件 3.7 に触れる）か、分類を `ResolvedBalloonText` に載せて下の段に変える必要がある。
7. **直の `register_actor` の道（要件 3.7）**: 公開の `register_actor` はモデルを受け取らず、呼び手は `crates/areka/` の試験の支えだけ。解決が記録しなくなると、この道に不正な宣言を渡しても記録は出ない。この道を要件 3.7 の「登録口」から外す（説明に書く）か、生の値を `ResolvedBalloonText` に載せてこの道でも書くか。
8. **前の値との比べ方（要件 3.2・3.3）**: 要件は「分類」で比べる。分類が `Invalid` のまま生の値だけが替わる（`2` → `3`）再追従では記録が出ない。`warn_cursor_fallback` の先例は生の値で比べる。分類で比べる（要件の字面）か、生の値で比べる（新しい不正な値も 1 件出る）か。後者は要件 3.3 の言い換えが要る。

### 要件の討議の結論（2026-11-13・開発者が一括承認）

- 議題 2・5 はカテゴリ A として要件に足した（1.3 の列挙を広げた・2.1 の写しに `warned`・`unowned` を含めた）。
- 議題 8 → 要件 3.2・3.3: 装着（切り替えを含む）は前の値に依らず必ず 1 件。再追従は宣言の生の値が前と違うときだけ 1 件。分類では比べない。
- 議題 7 → 要件 3.7: 直の `register_actor` は登録口に含めない（解決済みの値を受け取る試験の口・記録は解決させた側が持つと説明に書く）。
- 議題 1・3・4・6 は設計の段で決める。議題 6 は、上の 3.3（再追従で生の値を比べる・中の段も通る）を満たす置き場を選ぶ。

### 研究の要る点（Research Needed）

- 案 2-A の分け合う列: `TextLayerState` は `rehearsal_copy` で丸ごと複製され、`Decoration` は `PartialEq` で比べられる（`forget_pristine_places`・テスト）。`Rc` で足りるか（`TextLayerState` が別のスレッドへ渡らないか）、比べの意味（中身で比べるか）を設計で確かめる。
- 件 2 の世代: 「場所をまたいで一意」の数えを `TextLayerState` に持つと `state.rs`（789 行）に触る。空回しの写しの中で世代が進んでも本番と食い違わないか。
- 件 1 の伸ばしで `assign_items_to_chunks` を再利用できるか（区切り直す窓の頭がグリフの途中に落ちるとき・結合文字が別の cue で届いたとき）。`parse_boundaries`（字の番号）を使う方が写しが単純かもしれない。
- 範囲の外の覚え書き（決めではない）: 字が届くたびの配置（`layout_styled`）と、全文の道の `begins_with`（`lookahead.rs` 91〜95 行・届いた項目と全文の頭の比べ）は、今も 1 フレームあたり届いた量に比例する。この spec が消すのは区切りと当て直しの 2 乗だけで、要件の Objective の「1 フレームが重くならない」は区切りの項についての言い方になる。

## 7. 設計の段の調べと決め（2026-11-13・`/kiro-spec-design -y`）

### 7.1 調べの種類と使った決まり

- **種類**: 今ある仕組みの拡張（light discovery）。外の新しい依存は無い。`budouy` 0.2.2 の公開の口（`Parser::parse`・`Parser::parse_boundaries`）を登録簿の写し（`budouy-0.2.2/src/parser.rs` 29〜95 行）で読み直した。外の資料（ウェブ）は引いていない（新しいライブラリが無いため）。
- **使った決まり**: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`。画面・使い勝手の技能は当たらない（文字の層の中の作りの直しだけ）。小さな完全な直しを選ぶ方針（ponytail）で、各件とも「今ある型の持ち方を変える」案から取った。

### 7.2 確かめたこと（設計で頼る事実）

| 事実 | 確かめた所 | 設計への効き |
|---|---|---|
| `parse_boundaries` は字の番号（`char`）で境目を返す公開の口。`parse` と同じ `parse_boundaries_from_chars` を通る | `parser.rs` 29〜45 行 | 件 1 の伸ばしはこの口だけで足りる（フォーク不要） |
| 境目 `i` の 13 の特徴は `chars[i−3..i+3)` と端の切り詰めだけを読む | `parser.rs` 54〜92 行 | 決め直す頭 `p = max(1, L−2)`・左の文脈 `s = p−3` |
| グリフ → 塊の写し（`assign_items_to_chunks`）は「グリフの頭のバイトが属する塊が前と違えば新しい塊」＝「（前のグリフの頭, このグリフの頭］に境目があれば新しい塊」 | `segment.rs` 167〜203 行 | 件 1 は字の番号で同じ規則を言い直す。動きうるのは頭の字が `p` 以上のグリフだけ |
| 項目は後ろへ足されるだけ・消すのは `clear_content` だけで必ず `clears` が進む。普通のバルーンの `clears` は消えない（`forget_pristine_places` も残す）。箱の `clears` を消す `set_box_index` と同じ `set_box_layout` が箱の `ActorRender` を捨てる | `state.rs` 488〜496・557〜657 行・`state_route.rs` 70〜79 行・`actor_box.rs` 52〜62 行 | 件 1 の「消した回数が同じなら後ろへ足されるだけ」は今の `BreakMemo` の鍵と同じ前提で成り立つ |
| `BreakMemo` は `ActorRender` の欄で `Clone` を持たない。`runs` は `actor_frame_memo_tests.rs` の 4 本が見る | `actor.rs` 206〜207 行・`actor_frame_memo_tests.rs` 267〜336 行 | 件 1 は型を `segment.rs` へ移しても名前と `runs` の意味を保てる |
| `Note::Unowned` はキーだけで決まる（`is_unowned(key)`）。`unowned` へ入れるのは `applied` へ足す同じ腕だけ・空にするのも同じ戻しだけ | `look.rs` 756〜793 行・`state_decoration.rs` 343〜390 行 | 件 2 は `unowned` を写さず、当てるときに作れる |
| `warned` を読むのは今の行き先（`warn_once`・`warn_script_once`）だけ。スコープに従う今の行き先はいつも持ち主（`carry_script_decor` が `shared` を行き先にする） | `state_decoration.rs` 432〜470・536〜549 行 | 件 2 は `warned` を複製せず持ち主から持ち主へ動かせる |
| `set_box_traits` の呼び手は `set_box_layout` の 1 か所で、直後の `set_box_index` が箱の場所を全部捨てる | `actor_box.rs` 124〜125 行 | 「箱の種類が生きている箱の場所の上で替わる」場面は無い＝1 つの系列への書き手は 1 つ |
| `ActorTextState` の比べ（`PartialEq`）は `forget_pristine_places` が作りたてとの比べに使う。台本の先頭の後の戻しで印を振るので、印を比べに入れると作りたてと同じにならない | `state.rs` 488〜496 行 | 件 2 の印は比べに入れない |
| 生の値で前と比べる記録の先例 `warn_cursor_fallback` は `remember_inputs`（装着・中の段・下の段の全部が通る）から呼ばれ、前の値は記憶の入力から `register_actor` の前に取り出す | `actor_attach.rs` 233〜282 行 | 件 3 は同じ置き場と同じ順を取る |
| 箱の定義の 3 つの警告は `register_box`（表示のときに名前ごと 1 度）が書くが、書字方向の警告は今 `set_box_layout` の解決（表の受け取りで名前ごと 1 度）で出る | `actor_box.rs` 99〜118・436〜443 行 | 件 3 の箱は要件 3.4 どおり `set_box_layout` に置く（`register_box` へ寄せると、出る時と出ない箱が変わる） |

### 7.3 決め（議題 1・3・4・6 と研究の要る点）

#### 件 1 の伸ばし方 → 案 1-B（開いている run の状態を持つ）・置き場は `segment.rs`

- **選んだ形**: `BreakMemo` を `segment.rs` へ移し、開いている run の字（`Vec<char>`）・グリフの頭の字の位置・境目・前に決めた長さを持たせる。項目が増えたら新しい項目を run に足し、run を決めるとき（改行・`\_l` で閉じるときと最後）に `p = max(1, L−2)` から後ろの境目だけを決め直し、頭の字が `p` 以上のグリフだけ塊を付け直す。作り直しは「覚えを空にして同じ道で伸ばす」。
- **案 1-A を取らない訳**: 旧の末尾の近くを `items` から後ろ向きに読み直す形は、新しい状態は要らないが、グリフ `J−1`（頭の字が `p` より前で、境目 `p` を中に含みうるグリフ）の中の境目が要る。結合文字の長いグリフでは読み直す字の数がそのグリフの長さに比例し、「新しい字＋定数」（要件 1.1）を字の数で言えなくなる。1-B は境目を持つので、決め直すのはいつも `p` 以上だけで済む。
- **置き場**: `segment.rs`（`budouy` を使う唯一の所という決まりを保つ・`SegmentPlan` の欄を直に足し引きできる）。新しいソースファイル（`segment_grow.rs`）は作らない（テストの兄弟ファイル `segment_grow_tests.rs` だけ）。`actor_present.rs` は定義を消して `pub(super) use` で今の名前を保つ（896 → 約 875 行）。

#### 件 1 の檻の位置 → 純粋なテストで `BreakMemo::plan` を直に呼ぶ・`runs` の 3 本は期待値を保つ

- 数え `SEGMENTED_CHARS` は `budouy` に渡す字の数で、伸ばしの `parse_boundaries` と `segment_plan` の `parse` の**両方**で足す。片方だけだと、`BreakMemo::plan` を「届くたびに `segment_plan`」へ戻しても数えが増えず檻が赤にならない。
- 檻は `segment_grow_tests.rs`（`PURE_SOURCES`）に置き、GPU の World を使わない。`BreakMemo` が `arrange_lines` から外されたら、`actor_frame_memo_tests.rs` の `runs` の期待値（1・2）が赤になる。
- `runs` は「鍵が変わった呼び出しの数（伸ばしも 1 回）」で、今の 3 本の期待値と読み戻しの 1 本の期待値（2）を変えない。

#### 件 2 の持ち方 → 案 2-A の変形（場所ごとの列を保ち、系列の印と当てた数で差分を写す・列は分け合わない）

- **選んだ形**: `Decoration` に系列の印 `Lineage`（一括の戻しのたびに `static AtomicU64` から一意の番号）を足す。写しは先の（印, 当てた数）を見て、印が同じで数が元以下なら元の `applied` の後ろだけを複製して当て、そうでなければ元の土台から全部を当てる。`unowned` は当てるときの `Note::Unowned` から作る。`warned` は持ち主から持ち主へ動かす（`mem::take`）。2 層の差し込みは値を比べて同じなら何もしない。
- **`Rc`／`Arc` で分け合う形を取らない訳**: `rehearsal_copy` が `TextLayerState` を丸ごと複製するので、分け合う列に空回しの中で足すと本番の列が書き換わる（深い複製の手書きが要る）。`Rc<RefCell<…>>` は `TextLayerState` の `Send` を壊しうる。`Rc<Vec>` を `make_mut` で伸ばすと、ほかの場所が持っている間は複製になり 2 乗が残る。場所ごとに持つ列の総量は「当てた指定の数 × 行き先になった場所の数」で、要件 2.7 の上の数そのものなので、分け合う得が無い。
- **案 2-B（スコープに 1 つだけ持つ）を取らない訳**: 写しそのものは消えるが、公開の読み口 `ActorTextState::unowned_vocab`（場所ごと）の意味が変わり（要件 4.3 の「足すだけ」に触れる）、`apply_font_args`・`push_current_style`・`reset_look*` がスコープの状態を引数に取るよう `state.rs` の `apply_cue` の腕を全部書き換えることになる。持ち主（`shared`）の考え方も変わる。結果は 2-A の変形と同じで、手間の上の数も同じ。
- **印を比べに入れない**: `Lineage` の `PartialEq` は手書きでいつも真（§7.2 の `forget_pristine_places`）。写しの判定は中の番号を直に比べる。
- **前提「1 つの系列への書き手は 1 つ」**: 系列の指定を足すのは今の行き先だけで、スコープに従う今の行き先は持ち主で、箱に閉じる箱は写しの元にも先にもならない（§7.2）。写しの差分の道で `debug_assert`（先の最後の指定＝元の同じ位置の指定）を置く。

#### 件 2 の数えと前と後の比べ → 数え 2 つと切り替え 1 つ（どれも `#[cfg(test)]`）

- `COPIED_SPECS`（写しで当てた数）・`REBASED_SPECS`（当て直しで当てた数）。
- 要件 2.8 の「全部当て直す形の結果」は、試験だけの切り替え `REPLAY_ALL`（写しをいつも全部の道・差し込みをいつも当て直し）で同じ台本をもう 1 度流して作る。この spec の前の動き（`take_script` の丸ごとの写し＋`rebase`）と同じ見た目になる。前の作りをテストの側へ写し書きするより小さく、写し書きが本番とずれる危うさも無い。

#### 件 3 の記録の口 → 案 3-A（`remember_inputs`・生の値でキーごとに比べる）・箱は `set_box_layout`

- 普通のバルーンは `remember_inputs`（装着・中の段・下の段の全部が通る）に `previous_direction: Option<DirectionDecl>` を足す。装着は `None`（要件 3.2・前に依らず 1 回目）、再追従は記憶の入力から取り出した宣言（下の段は `finish_registration` の前に取り出す）。
- 比べはキーごとの生の値（要件 3.3・討議の結論）。片方のキーだけが変わった再追従で、変わらなかったキーの警告は繰り返さない。矛盾併記の `debug!` は、どちらかの生の値が変わったか前が無いとき。
- 案 3-B（`register_actor`）を取らない訳: 中の段（`vertical` が `0` から `2` に変わっても書字方向は横書き）が `register_actor` に届かず記録が漏れる（要件 3.7）。漏らさないために分類を `ResolvedBalloonText` に載せると、中の段が下の段（描画資源の作り直し）に変わり、公開の欄が増える。
- 記録の文言と欄（`value`・`vertical`・`writing_mode`）は 1 文字も変えない。`balloon` の欄を足すのは要件 3.2 の「今と同じ欄」に触れるので足さない。

### 7.4 まとめの 3 つの見方（design-synthesis）

- **まとめられるもの**: 3 件は「全部やり直す」を「変わった所だけ」にする同じ考えだが、持つもの（字の境目・指定の列・宣言の生の値）も層も違う。共通の型や口は作らない（作っても実装が 1 つずつしかない）。件 3 は既にある `warn_cursor_fallback` の型（前の値を記憶の入力から取り出して比べる）に揃えた。
- **作るか借りるか**: BudouX の差分の区切りは `budouy` に口が無い（公開は `parse`・`parse_boundaries` だけ）。フォークも `[patch]` もしない決まり（要件 4.6）なので、公開の `parse_boundaries` に窓の分だけの字を渡す形で借りる。数えの形は `layout_scan.rs` の `FOLLOWING_SCAN_VISITS` を借りる。
- **削ったもの**: 新しい本番のソースファイル（`segment_grow.rs`）・分け合う列（`Rc`）・スコープへの持ち替え（2-B）・`ResolvedBalloonText` の新しい欄・書字方向の解決の「黙る版」（3-C）・前の作りのテスト側への写し書き。

### 7.5 危うさと手当て

- **`budouy` の版上げで窓が変わる** — `segment_grow_tests.rs` の前と後の比べ（`segment_plan` が正）が見張る。design.md の Revalidation Triggers に挙げた。
- **件 2 の「1 つの系列への書き手は 1 つ」が将来の変更で崩れる** — `debug_assert` と `REPLAY_ALL` の比べ。Revalidation Triggers に挙げた。
- **件 2 の `warned` を動かすことで、持ち主でない場所の `warned` が空になる** — 読む所が無い（§7.2）。比べ（`PartialEq`）には入るが、比べるのは台本の先頭の後（全部の場所の `warned` が空）の `forget_pristine_places` と、同じ状態の前後を比べるテストだけ。
- **シェルの切替で生まれ直した箱の場所は、次に行き先になると持ち主の指定を全部当てる** — 生まれ直しは別の場所として要件 2.7 の上の数に入れる（design.md に書いた）。
