# 設計の検証: emo-text-reveal-incremental

- 対象: `.kiro/specs/emo-text-reveal-incremental/design.md`（要件 `requirements.md`・調査 `research.md`）
- 検証日: 2026-11-13（ブランチ `claude/kiro-start-emo-text-reveal-incremental-61e501`・`ffbf08ce9`）
- 判定: **GO**（下の 3 件は設計の文とテストの作りの直しで、形を変える必要は無い）

## 設計の検証のまとめ

3 件とも「今ある型の持ち方を変える」だけで、新しい層も公開の型も作らず、純粋層と actor 層の向き・`budouy` を使うのは `segment.rs` だけという決まり・`crates/areka/` に触らない約束を守っている。設計の事実の主張をコードで確かめたところ、`budouy` 0.2.2 の特徴量の窓（字 `i` の前の境目は `chars[i−3..i+3)` だけで決まり、後ろへ字を足して変わりうるのは `i ≥ L−2` だけ）・`Note::Unowned` がキーだけで決まること・書字方向の解決の本番の呼び手が `actor_decoration.rs` の 1 か所だけであることは、どれも正しかった。ただし件 1 の伸ばしの規則に、新しく始まったひと続き（run）の最初のグリフを塊に入れない書き漏れが 1 つあり、件 2 の「前と後の比べ」の正が新しい作りの全部の道そのものなので、直した 2 つの意味（所有外のキーの作り直し・記録済みの印の移し）の食い違いを見られない。

## コードで確かめたこと

### 件 1（`BreakMemo` の伸ばし）

- `budouy-0.2.2/src/parser.rs` の `parse_boundaries_from_chars`: 13 の特徴が読むのは `substring(chars, i−3, i)` から `substring(chars, i, i+3)` までで、端は `saturating_sub` と `min(len)` で詰める。左の文脈を 3 字渡せば `i ≥ p` の位置の答えは全文のときと同じになり、右端は全文と同じ所で終わる。`p = max(1, L−2)`・`s = p−3` は正しい。
- `parse` は `parse_boundaries_from_chars` の答えを `split_by_boundaries` で切っただけなので、今の `segment_plan`（`parser.parse(run_text)` の塊のバイト長）と、設計の `parse_boundaries`（字の番号）は同じ境目を使う。境目は 1 から始まり狭義に増えるので、空の塊は出ない。
- グリフから塊への写し: 今の `assign_items_to_chunks` は「グリフの頭のバイトが属する塊」で決めるので、字の番号で言えば「グリフ `j−1` の頭の字と、グリフ `j` の頭の字の間（左を含まず右を含む）に境目が 1 つでもあれば新しい塊」と同じになる（設計の規則 4）。グリフ `j` の扱いが変わりうるのは頭の字が `p` 以上のグリフだけ、も正しい。
- 前提「消した回数が同じ間、項目は後ろへ足されるだけ」: 項目を足すのは `state.rs` の 547・557・620・646・657 行だけで、消すのは `clear_content`（`state_decoration.rs` 247 行）だけ。`\c`（565 行）と台本の先頭（585 行）は必ず `clears` を進める。箱の場所の `clears` は `set_box_index`（`state_route.rs` 74 行）で消えるが、同じ `set_box_layout` が箱の `ActorRender` を捨てる（`actor_box.rs` 59〜62 行）。`forget_pristine_places` は `clears` を残す。どれも設計の書いたとおり。
- `actor.rs` 698 行の `present::BreakMemo::default()` は `#[cfg(test)]` の `arrange_for_test` だけで、本番の道ではない。

### 件 2（`Decoration` の写しと 2 層）

- `applied` を変えるのは `apply_font_args`（足す）・`reset_look_to`（空にする）・`take_script`（写す）だけで、`\f` が入る場所は今の行き先（`state.rs` 667 行の `place_entry(&dest)`）だけ。スコープに従う場所が行き先になるのは `set_dest` → `carry_script_decor` を通るときだけで、そのとき `route.shared` も同じ場所になる。`reset_routes` は先に `fold_shared_into_balloon` で持ち主を普通のバルーンへ戻してから行き先を直に書く。よって「1 つの系列へ書き込むのは今の持ち主だけ」は今のコードで成り立つ。
- 一括の戻しは場所ごとに走る（`\f[default]`・`\f[disable]` は今の行き先だけ、台本の先頭はすべての場所、`reset_decoration(None)` もすべての場所）。場所ごとに別の番号を振るので、戻した場所とそうでない場所が同じ印を持つことはない。差分の道が選ばれるのは、同じ印で先の数が元の数以下のときだけで、そのとき先の `applied` は元の頭になっている。
- 所有外のキー: `look.rs` の `apply_font_tag` は、`default`・`disable`・真偽 5 項目・上下付き・`height`・`color`・`name` のどれにも当たらないときだけ `is_unowned(key)` を見る。前の分かれもキーだけで決まるので、`Note::Unowned` はキーだけで決まる。`unowned` へ入れるのは `apply_font_args` の `Ok(Some(Note::Unowned))` の腕だけ（同じ呼び出しで `applied` へも足す）で、空にするのは `reset_look_to` だけ。当て直せば同じ `unowned` が出る、は正しい。
- 記録済みの印（`warned`）を動かす: `warned` を読むのは `warn_once`・`warn_script_once` で、どちらも今の行き先の場所で走る。持ち主でなくなった場所がまた行き先になるときは、`carry_script_decor` が必ず持ち主からの写しを先に入れる（普通のバルーンはいつもスコープに従う）。台本の先頭はすべての場所の `warned` を空にしてから `forget_pristine_places` を呼ぶので、作りたてと同じかの判定も変わらない。`state_decoration_carry_tests.rs` 400 行の `has_script()` は台本の先頭の後なので変わらない。
- `set_box_traits` と `set_look_layers` の呼び手は `actor_box.rs` 124 行と `actor_attach.rs` 179 行だけ（`crates/areka/` には無い）。`set_box_traits` の直後に必ず `set_box_index`（125 行）が箱の場所を捨てる。

### 件 3（書字方向の記録）

- `WritingMode::resolve` の本番の呼び手は `actor_decoration.rs` 214 行（`resolve_with_background`）だけ。`resolve_with_background` を本番で呼ぶのは `resolve_stamped`（装着 225 行・再追従 380 行）と `set_box_layout`（`actor_box.rs` 104 行）だけで、箱の登録（`register_box`）は覚えた解決の複製を使い、解き直さない。
- 普通のバルーンの登録で `register_actor` を呼ぶのは `finish_registration` だけ、`remember_inputs` を呼ぶのは `finish_registration` と再追従の中の段だけで、記憶（`resolve_inputs`）を消すのは直の `register_actor` だけ。設計の道の表（装着＝前なし・上の段＝呼ばない・中の段＝記憶の値・下の段＝`finish_registration` の前に取り出した値・箱＝前なし・直の口＝呼ばない）は、`warn_cursor_fallback` の今の置き場と同じで、要件 3.2〜3.5・3.7 を満たす。
- 書字方向の警告の文言を数える試験は `writing.rs` の tests と `writing_decision_tests.rs` の 2 つだけ（`crates/` を文言で探して確かめた）。

### 共通の約束

- `lib.rs` の `PURE_SOURCES` の母数は今 88（525 行）で、設計の 88 → 90 と合う。`actor_region_warn_tests.rs` は `SOURCES_OUTSIDE_THE_PURE_SCAN` の側（469 行）にあり、新しい `actor_writing_warn_tests.rs` を同じ側に置く案と合う。
- 行数の見込み: `segment.rs` 566 → 約 700・`state_decoration.rs` 649 → 約 740・`writing.rs` 400 → 約 450・`actor_attach.rs` 439 → 約 470・`actor.rs` 917 → 約 920。どれも 1,000 行の内。`look.rs`（998 行）には触らない。
- 公開の形: `BreakMemo` は `pub(super)` から `pub(crate)` へ移るだけ、`Lineage`・`ScriptDelta` は私の型、`DirectionDecl`・`note_direction_declarations` は `pub(crate)`。`Decoration` に私の欄が 1 つ増えるだけで、公開の形は変わらない。
- 数えの形は `layout_scan.rs` の `FOLLOWING_SCAN_VISITS`（`#[cfg(test)]` の `thread_local!` の `Cell<usize>`）と同じ。

## 大事な指摘（3 件）

### 指摘 1: 新しく始まったひと続きの最初のグリフが塊に入らない（件 1 の規則 3・4）

- **何が問題か**: 規則 3 は「頭の字が `p` 以上の最初のグリフを `J` とする」で、`p = max(1, L−2) ≥ 1`、グリフ 0 の頭の字は 0 なので、`J` はいつも 1 以上になる。前に決めたときの字の数が 0（`L = 0`＝改行か `\_l` の後に新しく始まったひと続き、まだ区切っていない覚え、作り直しの後）なら、グリフ 0 はまだどの塊にも入っていないのに、規則 3 の「残った最後の塊（グリフ `J−1` を含む・この run のもの）」は存在せず、規則 4 は `J` から後ろしか見ない。書いてあるとおりに作ると、どのひと続きでも最初のグリフが計画から落ちる（規則 4 の「`j = 0` なら新しい塊」の枝にも届かない）。「空の覚えから伸ばすのは run ごとに `p = 1`・`s = 0` で全部を区切ることで `segment_plan` と同じ」という説明も、この点で字面どおりには成り立たない。
- **影響**: 区切りの結果が今と変わる（要件 1.2・4.1 に反する）。テスト 2（前と後の一致）がすぐ赤にするので本番へは出ないが、タスクを書く人と実装する人が設計の文を正として進むと、直しの手戻りになる。
- **直し方**: 規則 3 を「`L = 0` なら `J = 0`（このひと続きの塊はまだ無いので、捨てるものも詰めるものも無い）、`L ≥ 1` なら頭の字が `p` 以上の最初のグリフを `J` とする」と書き分ける。あわせて要件 1.5 の上の数を「新しく届いた字（`char`）の数＋5」で書き、テスト 1 の上の数（`6 × 4,000`）は 1 グリフ 1 字の台本で数えると書く（「か゚」のような 2 字のグリフを混ぜると、1 グリフあたり 7 字になる）。
- **要件**: 1.1・1.2・1.3・1.5・4.1
- **設計の場所**: 「System Flows」の「件 1: 届いた字の区切りを伸ばす」の settle の規則 1〜5 と「作り直し（要件 1.3）」、「Testing Strategy」の `segment_grow_tests.rs` の 1

### 指摘 2: 件 2 の「全部当て直す形」との比べが、直した意味を見られない

- **何が問題か**: `REPLAY_ALL` が真のときの動きは「写しをいつも全部の道にし、2 層の差し込みを値に依らず当て直す」で、全部の道は新しい作りのもの（`unowned` を空にして当てながら作る・`warned` を元から動かす・土台を元の土台にする）である。この spec の前の作り（`take_script` が 4 欄を複製してから `rebase`）とは別物なので、テスト 2（要件 2.8）は差分の道と全部の道が互いに合うことは確かめられても、全部の道そのものが今の動きと同じかは確かめられない。とくに所有外のキーを当てながら作ること（指摘の (c)）と、記録済みの印を複製でなく動かすことは、比べの両側が同じ作りなので食い違いが出ても見えない。今のコードを読む限り食い違いは無い（上の「コードで確かめたこと」）が、それは読み手の推論で、檻ではない。
- **影響**: 要件 4.1（見た目の結果を変えない）の見張りが、将来の変更（`apply_font_tag` の分け方や `warned` の読み手が増える）で黙って外れうる。設計の Revalidation Triggers に挙げた前提の破れを、テストが拾えない。
- **直し方**: テストの側に、作りに依らない正を 1 つ足す。各時点ですべての場所について「今の見た目＝土台の層の見た目から `applied` を順に `apply_font_tag` で当てた結果」「`unowned`＝`applied` のうち `Note::Unowned` を返す指定を順に入れた結果」を、その場で一から計算して比べる（`look.rs` を呼ぶだけで数行）。記録済みの印は、テスト 4（記録が今のまま）で「持ち主でなくなった場所がもう一度行き先になってから同じ不正な指定を当てても、1 台詞 1 度」を足して固定する。`REPLAY_ALL` を残すなら、真のときは 4 欄の複製と `rebase` という前の作りそのものにする方が、比べの正として素直である。
- **要件**: 2.5・2.6・2.8・4.1
- **設計の場所**: 「試験の作りだけの数えと切り替え」の `REPLAY_ALL`、「Testing Strategy」の `state_decoration_incremental_tests.rs` の 2 と 4

### 指摘 3: 生まれたての場所の系列の印が、どの場所でも 0 で重なる

- **何が問題か**: 設計は「生まれたての場所は 0」とし、差分の道を「先の数が 0 より大きく、同じ印で、先の数が元の数以下」で選ぶ。台本の先頭の戻しはその時に在る場所にしか番号を振らないので、台詞の途中で初めて生まれた場所（初めて出たスコープの普通のバルーン、初めて行き先になった箱）は、どれも印 0 のまま `\f` を受けられる。今のコードでは、印 0 で `\f` を持つ場所が 2 つ在っても、写しの元と先になるのはスコープに従う場所だけで、箱に閉じる箱は写しに加わらず、箱の種類が替わるときは必ず箱の場所が捨てられる（`set_box_traits` の直後の `set_box_index`）ので、食い違いは起きない。ただしこれは「箱の種類は箱の場所と一緒にしか替わらない」という、設計の Revalidation Triggers に書かれていない前提に乗っている。`set_box_traits` は公開の口で、単独で呼べば、印 0 で自分の指定を持つ箱に閉じる箱がスコープに従う箱に替わり、印 0 の持ち主から差分の道で写しを受けて、別の系列の指定を頭とみなす。
- **影響**: 今の本番では起きないが、起きたときは見た目が黙って変わる（記録も出ない）。直すのは 1 行で済むのに、見張りは `debug_assert`（最後の指定が同じか）だけで、たまたま同じ指定なら通ってしまう。
- **直し方**: 場所が生まれるとき（`Decoration::default` と `with_layers`）にも、一括の戻しと同じく新しい番号を振る。印 0 の特別扱いが消え、「同じ印なら同じ系列」が前提なしで成り立つ。比べ（`PartialEq`）は印を見ないので、`forget_pristine_places` の判定は変わらない。番号を振る手間は場所が生まれる回数だけで、写しの手間の上の数も変わらない。振らないままにするなら、Revalidation Triggers に「箱の種類（`font.follow`）が、箱の場所を捨てずに替わる道が増える」を足す。
- **要件**: 2.1・2.5・4.1
- **設計の場所**: 「件 2: 行き先が替わるときの写し」の「系列の印 `Lineage`」、「State Management」の `NEXT_LINEAGE`、「Revalidation Triggers」

## 設計の良いところ

- **結果を変えないことを、読んだ根拠と檻の両方で示している**。件 1 は `budouy` 0.2.2 の特徴量を読んで窓の定数を導き、版を上げたら前と後の比べのテストを回すことを Revalidation Triggers に書いた。件 2 は「1 つの系列への書き手は 1 つ」「所有外のキーはキーだけで決まる」「`warned` は今の行き先だけが読む」を個別に挙げ、どれもコードと合っていた。性能の檻を時間でなく見た数（`SEGMENTED_CHARS`・`COPIED_SPECS`・`REBASED_SPECS`）で固定しているので、機械の速さに依らず決定論で、2 乗へ戻すと赤になる。
- **件 3 の置き場が先例とそろっている**。記録を `remember_inputs`（装着・再追従の中の段・下の段が必ず通る所）に置き、前の値を生の値で比べ、下の段では `register_actor` が記憶を消す前に取り出す——どれも `warn_cursor_fallback` と同じ順と形で、「`vertical` が 0 から 2 に変わっても書字方向は同じ」という中の段の道でも記録が漏れない。箱は名前ごとに 1 度解く `set_box_layout` の隣に置き、直の口 `register_actor` は記録しないことを説明に書く。

## 最終の判定

- **判定: GO**
- **理由**: 3 件とも既にある層・持ち主・呼び方を保ったまま中の持ち方だけを変える形で、要件 1〜4 の全部に持ち場とテストが当たっている。上の 3 件は、どれも設計の文とテストの作りを直せば済み、形の作り直しは要らない。
- **次の段**: 設計の討議（`/kiro-design-discussion emo-text-reveal-incremental`）で上の 3 件を扱い、design.md に反映してから `/kiro-spec-tasks emo-text-reveal-incremental` へ進む。
