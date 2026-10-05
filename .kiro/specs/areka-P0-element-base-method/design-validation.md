# 設計の検証レポート: areka-P0-element-base-method

- 対象: `.kiro/specs/areka-P0-element-base-method/design.md`（要件は確定済み）
- 実施: 2026-10-05・対話なし（設計の文面を信じず、引用されたコードを実際に読んで照合した）
- 判定: **GO**（下の 3 件は設計の討議とタスク生成で拾えば足りる。作り直しは要らない）

## 1. 検証のまとめ

設計の中心（読み手が値にする語を `overlay` と `base` の 2 語へ広げる・`Element` に欄を足さない・描けない行は別の転記が並べて `load_shell_target` が 1 か所で警告する）は、実際のコードと合っている。下流（畳み込み・土台の決定・外形）は届いた `Element` を描画メソッドに関係なく一律に扱っているので、「`base` は `overlay` と同じ見え方」が作りで成り立つ。設計が引用した行番号と事実は、確かめた範囲ですべて正しかった。直すべき点は、実機の確認の基準 1 か所の誤りと、テストの効き方 2 点である。

## 2. コードと照合した事実

| 設計の主張 | 照合の結果 |
|---|---|
| `decode_elements` は第 2 欄が `overlay` と完全一致する行だけを値にし、ほかを記録なしで捨てる。`surface*`・`surface.append*` の両方が通る | 正しい（`decode.rs` の `decode_elements`・呼び出しは `decode_surface_body` と `decode_append_block`） |
| 字句解析は欄の前後の空白を落とす。大文字小文字は変えない | 正しい（`lexer.rs` の `split_csv` が各欄を `trim()`） |
| `normalize_element` は描画メソッドを見ずに `Overlay` で置き、X,Y を位置にする | 正しい（`fold.rs` の `normalize_element`） |
| 土台の決定は「層 0 の element が在るか」だけを見る | 正しい（`base_image.rs` の `apply_base_images`） |
| 外形は届いた element の原寸と位置だけで決まる | 正しい（`plan.rs` の `flatten_extent`） |
| `parse_boxes` は第 2 欄が `balloon` の行だけを原文のまま並べ、記録を出さない | 正しい（`boxes.rs` の `surface_lines`） |
| 製品の中で surfaces.txt を読む入口は `load_shell_target` だけ。呼び出しは起動（`emo2_boot/assets.rs`）と窓の配置の測定（`placement/measure.rs`） | 正しい。ほかの `shell::parse` の呼び出しはテスト・試験用の入口・バルーンの合成用の文面だけだった |
| 付け替える既存テストは 3 本 | 正しい。`decode_tests_lenient_input_tests.rs` の 2 本（`non_overlay_element_method_is_absorbed_but_overlay_sibling_survives`・`mixed_valid_and_subset_out_lines_in_one_surface`）と `validation_tests.rs` の 1 本（`subset_out_features_absorbed_via_public_parse_without_breaking_neighbors`）。`unknown.block.head` の中の `base` の行は読まれないので触らなくてよい |
| 登録簿は `add`・`bind` を `Overlay` に写し、`is_implemented()` は pattern定義の門と共用 | 正しい（`method.rs`・`plan.rs`・`nesting.rs`・`blit.rs`） |
| 台帳の 2 行が在る | 正しい（`base:1` は `vocabulary-only`・element定義の行は `degraded`・記録なし） |

追加で確かめたこと。

- 同梱の検体（`vendors/sample_ghost/*.nar`）の surfaces.txt で、element定義の描画メソッドを数えた。`emo2` は `overlay` 62 行だけ。`claudia` は `base` 5 行・`overlay` 3 行。`R_POST_and_KOMAINU`・`konnoyayame` は element定義が 0 行。したがって、検体を使う既存のテストで本 spec の警告が新しく出たり、`used`／`shadowed` の数が動いたりするものは無い（`claudia` をシェルとして読む既存のテストは無い）。
- `claudia` の絵の実寸: `surface0.png`・`surface10.png`・`surface19.png`・`surface29.png` は 333×500、`surface1000.png`・`surface1001.png` は 100×56、`anthony_eyes11.png` は 71×42。要件 4.3 の 333×500 は絵の実寸と合う。
- 未確認: 「`Element` の構造体リテラル 46 か所」の数は数えていない（判定に影響しない）。

## 3. 重要な指摘（3 件）

### 指摘 1: 実機の確認の基準に事実の誤りがある（`shadowed` に載る番号）

設計の Monitoring は「`shadowed` の数が surface6・11・19・26・29 の分を含むことをログで見る」と書いている。しかし `claudia` のシェルに `surface6.png`・`surface11.png`・`surface26.png` は無い（在るのは `surface0`・`1`・`2`・`4`・`5`・`7`・`10`・`19`・`25`・`29`・`1000`・`1001`）。`shadowed` は「画像が在るのに `element0` が在るので使わなかった番号」なので、本 spec の後に増えるのは **19 と 29 の 2 件だけ**である。6・11・26 はそもそも隠す画像が無い。

- 影響: 実機の確認を文面どおりに行うと、正しく動いていても「足りない」と読める。基準に合わせようとして実装や検体をいじる誘因になる。
- 直し方: タスクと実機の手順では「`shadowed` が 19・29 の 2 件増える（`used` が 2 件減る）。6・11・26 は `shadowed` にも `used` にも載らない」と書く。6・11・26 の成否は大きさ（333×500）で見る。

### 指摘 2: 壊れても赤くならない確認が混ざっている・不具合の実物が檻に入っていない

- 結合テスト 4 の「その後 `build_world` を複数回呼んでも本 spec の記録が増えない」は、失敗しようがない。`build_world` は `Shell` から面の表を組むだけで、描けない行は `Shell` に入っていないから、警告を出す材料を持たない。要件 2.3 を実際に守っているのは「警告を出す場所が `load_shell_target` の 1 か所」という作りである。このままなら「作りで成り立つ・テストでは固定しない」と書くほうが正直で、確かめるなら「`load_shell_target` を 2 回呼ぶと警告がちょうど 2 倍になる（呼び出し 1 回につき 1 行 1 件）」の形にする。
- 単体テスト 5（3 つの転記が行を漏れなく重なりなく分ける）は、左辺の「`element` の行の数」をテストの文面に手で書いた数にしないと意味が無い（同じ字句解析で数えると、両辺が同じ道具に頼る）。設計は数え方を書いていない。
- 単体テスト 1 の較正「element の件数が 0 でない」は弱い。文面に元から `overlay` の行があれば常に真になる。「`base` の行の数だけ `Element` が増える」（`base` の行を除いた文面との件数の差）で較正するのがよい。等しさの判定そのものは、`base` が捨てられれば件数が合わずに赤くなるので効いている。
- 報告された不具合の実物（無改変の `claudia` の surface6・11・26）は実機の確認にしか出てこない。`claudia` は `sample-ghost-kit` に登記済みで、`shell_target` の既存の檻は同じ受け口（`shell_target_test_support.rs`）で `emo2` などの検体を DLL なしで読んでいる。同じ形で「`claudia` を `load_shell_target` で読み、6・11・26 の外形が 333×500・本 spec の警告が 0 件」を 1 本足せば、実機でしか分からなかった欠陥が決定論のテストに入る。要件 4.1 は合成の検体で満たせるが、実物の 1 本は安く、後戻りを最も直接に止める。

### 指摘 3: 同じ文面を 2 度読む作りのずれは今は起きないが、守りが 1 方向にしか効いていない

描けない行の転記（`parse_undrawn_elements`）が画像の読み手の分類からずれうるかを、実際のコードで突き合わせた。

- 今はずれない。語の判定は `is_image_element_method` の 1 関数を両方が使う。見出しの判定（`surface.append` で始まる → `surface` で始まる）は `decode.rs`・`boxes.rs` と同じ。キーの判定（`element` で始まる）も同じ。`decode.rs` には「閉じずに終わったブレスも値にする」枝が在るが、字句解析が閉じないブレスを丸ごと不正な断片にするので、その枝には届かない。したがって「閉じたブレスだけを見る」転記と結果は一致する。
- 残る弱点 ⑴: `balloon` の語が 3 か所の書き写しになる（`boxes.rs` の判定・新しい `undrawn.rs` の除外・読み手が値にしないこと）。画像の語は 1 関数にまとまるのに、箱の語はまとまらない。将来だれかが箱の側の語を足すと、箱の転記は拾い、描けない行の転記も警告する（二重）。テスト 5 は文面に書いた語しか見ないので、この種のずれを捕まえない。直し方は小さい: `decode.rs` の `is_image_element_method` の隣に箱の語の判定を 1 つ置き、`undrawn.rs` はその 2 つだけで対象を決める（`boxes.rs` を触らない方針は保てる。箱の側の付け替えは後続でよい）。
- 残る弱点 ⑵: 見出しの判定は 3 か所の書き写しのままである（Revalidation Triggers に書いてあるが、テストは無い）。テスト 5 の文面に、`descript`・`balloon.*`・`kero.surface.alias`・未知の見出しのブレス、ブレスの外の行、**閉じずに終わる `surface*`ブレス**を混ぜておくと、見出しとブレスの扱いのずれも数で捕まる。設計のテスト 4 は前の 2 つだけを挙げており、閉じないブレスが抜けている。
- 補足（指摘ではない）: `load_shell_target` は起動のたびに 2 回呼ばれる（窓の配置の測定と起動）。描けない行 1 行につき、起動 1 回で警告が 2 件出る。要件 2.3 の文面（読み込み 1 回につき 1 件）には合っており、入れ子・箱の報告と同じ今の型だが、利用者から見ると重複に見える。設計は Risks に書いているので、実機の確認で「2 件出るのが正しい」と手順に明記しておくとよい。

## 4. 設計の良い点

1. **「同じ見え方」をテストでなく作りで保証している。** `base` の行と `overlay` の行が同じ `Element` の値になるので、下流は見分けようがない。`Element` に欄を足す案（brief の文面）を採らなかった理由（読む者がいない・生の `elements` を読む所すべてにふるいが要る・後続が形を決める）は、コードを読んだ結果と合う。差分は読み手の判定 1 か所と新しい転記 1 つに収まる。
2. **黙って捨てる経路が無くなる。** 今まで記録の無かった「`overlay` 以外の行を捨てる」経路が、「描く」か「警告する」のどちらかに必ず入る。`add`・`bind` を pattern定義向けの登録簿に通さず描かない側に置いた判断も、ukadoc の「elementでの使用は未定義」に沿っており、黙って描かれる事故を避けている。

## 5. 最終判定

**GO。**

- 理由: 設計の事実の主張はコードと合っており、要件 1〜4 のすべてに持ち主と確かめ方がある。境界は狭く、既存の検体とテストへの波及は付け替えの 3 本だけである。指摘 3 件はどれも作りの変更を求めず、基準の文面とテストの書き方で直る。
- 次の手順: 設計の討議で指摘 1〜3 の扱いを決め、`/kiro-spec-tasks areka-P0-element-base-method` へ進む。タスクには少なくとも次を入れる。
  1. 実機の基準を「`shadowed` は 19・29 の 2 件増」に直す（指摘 1）。
  2. `build_world` の確認を「2 回読むと 2 倍」に替えるか外す・テスト 5 の左辺を手で書いた数にする・テスト 1 の較正を件数の差にする・`claudia` の実物の檻を 1 本足すかを決める（指摘 2）。
  3. 箱の語の判定を `decode.rs` の 1 関数に置くか・テスト 5 の文面に閉じないブレスとほかの見出しを混ぜる（指摘 3）。
