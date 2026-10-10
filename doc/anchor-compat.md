# アンカー 互換対応表（Reference 割付・送出の順・正典が黙る分岐の裁定）

> 出所 spec: `areka-P0-anchor-tag-canon`（要件 7.1・7.2・7.5）
> 正本: 当該 spec の `design.md`「主要な設計判断」「Components and Interfaces」「Error Handling」と、`tasks.md` の「Implementation Notes」。**正典引用は同 spec の `requirements.md` の冒頭（Introduction と「正典の出所」・2026-10-10 に ukadoc MCP で引いた記述）から転記**しており、本書で新たな引用を作っていない。
> 位置づけ: `COMPAT_ARCHITECTURE.md` §2「沈黙ルール」（ukadoc が沈黙/曖昧な箇所は areka 裁量で決定し、判断を対応表に明記する）の運用実体のうち、**アンカー（`\_a`・バルーンの本文の中の押せるリンク）の領域の詳細台帳**。同書 §8 の横断表は要約・本書が詳細。選択肢（`\q`）の側の詳細は `choice-cascade-compat.md`。
> 生成物ではなく手編集の記録文書である。

---

## 0. 読み方（provenance と印）

各行には**出所を表す 1 値**を必ず付ける。語彙は `choice-cascade-compat.md` §0 と同じ 3 値である。

| provenance | 意味 |
|---|---|
| `ukadoc` | 正典（ukadoc）に直接の記述があり、areka はその記述どおりに動作する |
| `ssp_secondary` | 正典本文では二次的／実装依存と位置づけられる記述に由来する |
| `areka_discretion` | 正典が沈黙している、または正典記述間に字面の揺れがあるため areka が裁定した |

- §1 は `ukadoc` の行だけ、§2 は `areka_discretion` の行だけである。§2 の行が正典の記述を借りているときは、「正典引用・根拠」の欄に引用の記号（Q-1〜Q-7）を書く。
- 本書に `ssp_secondary` の行は無い（0 行）。
- **確認待ち**の印が付いた行は、開発者の確認がまだ済んでいない裁定である（本書では 2.2 の D-4 の 1 行だけ）。
- **列の意味**: `挙動`＝areka が実際に行うこと／`provenance`＝上記 3 値／`正典引用・根拠`＝引用の記号と、その挙動を決めている関数・固定しているテストの名前／`反証・注記`＝反対に読める記述・固定するテストが無い所・追跡先。
- コードは行番号でなく、関数・型・テストの名前で指す。テストのファイルは、初出の所にだけパスを書く。
- 用語: **場所**＝スコープと文字の行き先（普通のバルーンか、シェルの中のバルーンの箱か）の組。**定常**＝起動が済み、終了や切り替えの処理に入っていない、普段の運行。**合図**＝台本を再生の単位に直したもの（`dola` の `CueCommand`）。

---

## 1. Reference の割付と送出の順（正典に根拠がある決め・要件 7.1）

### 1.1 正典の引用（`requirements.md` の冒頭から転記）

| 記号 | ukadoc の項 | 引用 | ukadoc のページ |
|---|---|---|---|
| Q-1 | さくらスクリプト `\_a[ID]` | 「`\_a`がくるまでの範囲をアンカーとする。…ジャンパをクリックするとSHIORIイベントOnAnchorSelectが開始される(SSPのみ先立ってOnAnchorSelectExが開始される)。IDはReference0に格納される。」 | `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_a_5bID_5d:1` |
| Q-2 | さくらスクリプト `\_a[ID]` | 「選択肢タイムアウトしないが、通常トーク同様バルーンタイムアウトする」 | Q-1 と同じ項（転記元は `requirements.md` の Introduction） |
| Q-3 | さくらスクリプト `\_a[ID,r2,r3...]` | 「ジャンパとなったテキストがReference0に、IDはReference1に、続く引数はReference2以降に格納される。OnAnchorSelectExに続けてOnAnchorSelectも発生する」 | `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_a_5bID_2cr2_2cr3..._5d:1` |
| Q-4 | さくらスクリプト `\_a[OnID,r0,r1...]` | 「IDが"On"で始まっている場合は、クリックするとSHIORIイベントOnID(書いた通りのイベント)が開始される。IDに続く引数が順番にReference0以降に格納される。」 | `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_a_5bOnID_2cr0_2cr1..._5d:1` |
| Q-5 | SHIORI イベント `OnAnchorSelectEx` | 「`\_a`ジャンパがクリックされた瞬間に発生。このイベントにSHIORIが何も返さなかった場合にのみ、続けてOnAnchorSelectが発生する。」Reference0＝ジャンパのテキスト・Reference1＝ID・Reference*＝拡張情報（`\_a` タグ内の 2 番目以降の引数） | `https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnAnchorSelectEx:1` |
| Q-6 | SHIORI イベント `OnAnchorSelect` | 「`\_a`ジャンパがクリックされた瞬間に発生。」Reference0＝選択されたジャンパの ID | `https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnAnchorSelect:1` |
| Q-7 | バルーンの descript.txt の `anchor.style`（選択中アンカーの形状） | 既定値は `underline`（文の引用ではなく、値の転記） | `https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#anchor.style_2c_5f62_72b6:1` |

ページの綴りは網羅台帳の目録（`doc/ukadoc-coverage/catalog.toml` の `url`）のものである。

### 1.2 割付と順

| # | 裁定項目 | 挙動 | provenance | 正典引用・根拠 | 反証・注記 |
|---|---|---|---|---|---|
| A-1 | `OnAnchorSelectEx` の Reference | Reference0＝アンカーの範囲に表示された文字／Reference1＝ID／Reference2 以降＝`\_a[ID,…]` の 2 番目以降の引数を記述順 | `ukadoc` | Q-3・Q-5。組み立ては `crates/areka-kanade/src/schedule/events.rs` の `on_anchor_select_ex`。テストは `crates/areka-kanade/src/schedule/events_anchor_tests.rs` の `on_anchor_select_ex_is_get_with_text_id_then_arguments_in_written_order` | 生成正本 `doc/shiori/fragments/events/14.choice.toml`（`[entry."OnAnchorSelectEx"]`）は当該エントリの `provenance` を `ssp_secondary` と記録している（Q-1 も「SSPのみ」と書く）。**割付そのものは Q-3・Q-5 の記述に一致**するため本書は `ukadoc` として扱い、粒度の差だけをここに注記する。Reference0 の文字の細かい決めは 2.1 の 10 |
| A-2 | `OnAnchorSelect` の Reference | Reference0＝ID の 1 個だけ（範囲の文字・引数は載せない） | `ukadoc` | Q-1・Q-6。組み立ては `events.rs` の `on_anchor_select`。テストは `events_anchor_tests.rs` の `on_anchor_select_is_get_with_id_only` | — |
| A-3 | ID が `On` で始まるアンカー | ID と同じ名前のイベントを、事前の登録なしに書いたとおりの名前で送る。Reference0 以降＝ID に続く引数を記述順 | `ukadoc` | Q-4。`crates/areka-kanade/src/schedule/anchor.rs` の `plan_anchor` が `On` で始まるかどうかを見て、`on_anchor` が選択肢の `On` 始まりと同じ組み立て（`events.rs` の `on_choice_named`）を呼ぶ。テストは `crates/areka-kanade/src/schedule/anchor_step_tests.rs` の `on_prefixed_id_sends_only_the_event_of_that_name` | 範囲の文字と ID を Reference に載せないこと・`OnAnchorSelectEx`／`OnAnchorSelect` を先に送らないことは正典が黙る所（2.1 の 6）。`On` の判定は大文字小文字を区別する前方一致（2.1 の 4） |
| A-4 | 2 つのイベントの順 | ID が `On` で始まらないアンカーは、`OnAnchorSelectEx` を先に、`OnAnchorSelect` を後に送る | `ukadoc` | Q-1（「先立って」）・Q-3（「続けて」）。テストは `anchor_step_tests.rs` の `canonical_id_sends_select_ex_then_select_and_then_nothing` | 順そのものに揺れは無い。揺れがあるのは**後の `OnAnchorSelect` を出す条件**（2.1 の 5） |
| A-5 | アンカーは答えの待ちを作らない | アンカーがあっても、選択肢の答えを待つ区切り（`WaitForChoice`）を出さず、選択肢の時間切れも無い。バルーンの時間切れは普通の台詞と同じに働き、アンカーがあることでは遅れない。バルーンの本文が消えれば、アンカーも一緒に消える | `ukadoc` | Q-2。区切りを出すかどうかは選択肢だけを数える（`crates/areka-sakura/src/compile_anchor_tests.rs` の `anchors_alone_do_not_raise_the_choice_fence`）。バルーンの時間切れを止める観測が読む `TextLayerRuntime::choice_active` は選択肢の範囲だけを数える（`crates/areka-emo-text/src/actor_route_tests.rs` の `anchor_only_scope_is_hit_active_but_not_choice_active`）。本文の消去は `crates/areka-emo-text/src/actor_clear_atomicity_tests.rs` の `clear_drops_anchor_hit_rows_and_hit_active_atomically` | 台詞の終わり（`\e`）だけではバルーンは消えないので、アンカーはバルーンが閉じるまで押せる |

---

## 2. areka の裁定（正典が黙る分岐・要件 7.2）

### 2.1 設計までに決めたこと

| # | 裁定項目 | 挙動 | provenance | 正典引用・根拠 | 反証・注記 |
|---|---|---|---|---|---|
| 1 | 話している最中のアンカーのクリック | 話している最中でも、表示済みのアンカーの上の左クリックは**アンカーの選択として使い、台詞を中断しない**（同じ押下を中断やシェルの操作に重ねない）。続く 2 打目（左ダブルクリック）も中断にしない。答えが台本なら、再生中の台詞を新しい台詞に置き換える（選択肢の答えを待っていたなら、その待ちも置き換えで消える）。答えが無ければ（204）、台詞も選択肢の待ちもそのまま続く | `areka_discretion` | 正典は話している最中の押下の扱いに沈黙。開発者確認 2026-10-10（要件討議 議題 2・要件 3.4）。テストは `crates/areka/src/input_events/user_break_tests.rs` の `second_click_after_selecting_an_anchor_is_not_a_break`、`crates/areka/src/input_events/shell_box_handler_tests.rs` の `box_anchor_click_while_talking_selects_without_breaking`、`anchor_step_tests.rs` の `script_reply_while_talking_replaces_the_talk_and_clears_the_choice_ledger`・`no_content_while_talking_leaves_the_talk_and_the_choice_ledger` | 置き換えは、選択肢の答えの台本と同じ既存の規律（再生の枠は 1 つ）。押下の判定の順は 選択肢 → アンカー → 中断 で、後の `areka-P0-talk-fast-forward` の早送りはこの後ろに並ぶ |
| 2a | 閉じの無い開き（`\_a[x]あ`） | 範囲を**台本の表示の終わり（`\e`・`\-`・末尾）まで**とする。compile が走査の終わりに閉じを補い、警告 `anchor_unclosed`（開きの位置 `index`・`id`）を 1 件残す。`\e`・`\-` より後ろの閉じでは閉じない | `areka_discretion` | 正典は開きと閉じが対にならない台本に沈黙（要件 1.8）。判定は `crates/areka-sakura/src/anchor_pair.rs` の `pair_anchors`。テストは `crates/areka-sakura/src/anchor_pair_tests.rs` の `open_without_close_is_unclosed_at_the_open`・`nothing_is_counted_after_end_or_quit`、`compile_anchor_tests.rs` の `unclosed_anchor_is_closed_at_the_end_of_the_scan` | 警告を出すのは compile だけ。文字の層は、表示の前に台本の全文を先に通す下見でも本番でも警告を出さない（`crates/areka-emo-text/src/state_anchor_tests.rs` の `rehearsal_emits_no_records`）。警告は `RUST_LOG=areka_sakura=warn` で拾える |
| 2b | 開きの重なり（`\_a[x]あ\_a[y]い\_a`） | **直前のアンカーをそこで閉じて**、新しいアンカーを始める（入れ子にしない）。警告 `anchor_reopened`（新しい開きの位置 `index`・その `id`）を 1 件残す | `areka_discretion` | 正典沈黙（要件 1.9）。テストは `anchor_pair_tests.rs` の `open_while_open_is_reopened_at_the_new_open`、`compile_anchor_tests.rs` の `reopened_anchor_is_closed_before_the_new_open` | 重なりの開きがそのまま閉じられないときは 2.2 の D-1 |
| 2c | 開いていないのに閉じ（`あ\_aい`） | 閉じを**無視して**表示を続ける。警告 `anchor_stray_close`（閉じの位置 `index` だけ。`id` は無い）を 1 件残す | `areka_discretion` | 正典沈黙（要件 1.10）。テストは `anchor_pair_tests.rs` の `close_while_not_open_is_stray_at_the_close`、`compile_anchor_tests.rs` の `stray_close_emits_nothing`・`mixed_breakages_warn_once_each_in_script_order` | 崩れが混ざっても、警告は崩れ 1 件につき 1 件で、台本の順に並ぶ |
| 3 | ID が空（`\_a[]`） | ID が空の開きとして**成立させる**。警告も「読めなかった」印も付けない。押せば ID が空のまま送る（ID の位置は空の文字列で作る） | `areka_discretion` | 正典は ID の形を限っていない（要件 1.11）。テストは `crates/areka-parsers/src/sakura/decode_anchor_tests.rs` の `empty_id_is_still_an_open`・`no_form_is_noted_raw_or_generic`、`compile_anchor_tests.rs` の `four_forms_become_anchor_cues_in_script_order`、`crates/areka-kanade/src/schedule/anchor_tests.rs` の `plan_is_named_only_for_on_prefixed_ids`（空の ID は `OnAnchorSelectEx` を送る側） | ID の位置を常に作るのは `events.rs` の `select_ex_get`・`select_get`（選択肢と共通の組み立て）。空の ID を送る所まで通すテストは無い（0 本）。記録に載せる ID も空のまま（最終段でも ID を覚える＝`AnchorStage::Final`） |
| 4 | ID の綴りに意味を持たせない | `On` で始まるかどうか（A-3）のほかは、ID の綴りを読まず、そのまま送る。選択肢の `script:` の約束はアンカーには**無い**（`\_a[script:…]` は ID が `script:…` のアンカーで、`OnAnchorSelectEx` を送る）。`On` の判定は大文字小文字を区別する前方一致で、`On` の 2 文字だけの ID も `On` 始まりの側に入る | `areka_discretion` | ukadoc の `\_a` には、`\q` の `script:` のような、ID の綴りに意味を持たせる形が無い（`requirements.md` の冒頭の注記・要件 4.1）。判定は `anchor.rs` の `plan_anchor`。テストは `anchor_tests.rs` の `plan_is_named_only_for_on_prefixed_ids`（`script:OnMenu`・`on_menu`・`ONMENU`・空は `OnAnchorSelectEx` の側） | 選択肢の `script:` の決めは `choice-cascade-compat.md` の 7a |
| 5 | `OnAnchorSelect` を続ける条件 | `OnAnchorSelectEx` にゴーストが**何も返さなかった（204）ときだけ**、続けて `OnAnchorSelect` を送る。台本が返れば送らない。送信の失敗（`error!` の記録 `anchor_shiori_failed_as_204`）と、GET では起きない応答（`warn!` の記録 `anchor_unexpected_reply`）も、204 と同じに進める | `areka_discretion` | 記述の間に字面の揺れがある。Q-5 の「何も返さなかった場合にのみ」を採った（選択肢の `OnChoiceSelectEx` → `OnChoiceSelect` と同じ形・`choice-cascade-compat.md` の 2b）。応答の扱いは `anchor.rs` の `on_anchor_reply`。テストは `anchor_step_tests.rs` の `canonical_id_sends_select_ex_then_select_and_then_nothing`・`one_notification_makes_one_event_chain_and_at_most_one_talk`・`send_failure_advances_like_no_content_with_one_error_and_no_fault` | **反証（字面の揺れ）**: Q-3 の「OnAnchorSelectExに続けてOnAnchorSelectも発生する」は、常に続くとも読める。送信の失敗を 204 と同じにし、アプリの終了へ倒さないのは正典が黙る所（要件 4.10）。失敗の記録は 1 回につき 1 件で、`id`・段（`select_ex`／`final`）・出所・誤りの内容が載る |
| 6 | `On` で始まる ID のとき、ほかのイベントを先に送るか | ID と同じ名前のイベント **1 本だけ**を送る。`OnAnchorSelectEx`／`OnAnchorSelect` は送らない（0 本）。範囲の文字と ID は Reference に載せない | `areka_discretion` | Q-4 は、その名前のイベントが始まることと、引数が Reference0 以降に入ることを書くだけで、ほかのイベントを先に送るかどうかに沈黙。選択肢の同じ裁定（`choice-cascade-compat.md` の 1）に揃えた（要件 4.4）。テストは `anchor_step_tests.rs` の `on_prefixed_id_sends_only_the_event_of_that_name` | — |
| 7 | 選択肢とアンカーが重なったときの判定の順 | 同じ点に選択肢の行とアンカーの範囲の両方が当たるときは、**選択肢を先に**判定する（台本での定義の順に関わらない）。アンカーが選択肢を包む形（`\_a[x]…\q[題,ID]…\_a`）では、選択肢の上は選択肢、それ以外はアンカー。ホバーの強調も押下も同じ順で、普通のバルーンと箱で同じ | `areka_discretion` | 正典沈黙（要件 3.5）。判定は `crates/areka/src/input_events/balloon.rs` の `hit_choice_row`（選択肢を先に見て、当たらなければアンカーを見る）。テストは `crates/areka/src/input_events/balloon_pure_core_tests.rs` の `overlapping_choice_wins_over_anchor_in_either_order`・`anchor_is_hit_only_where_no_choice_covers_the_point`、`crates/areka/src/input_events/shell_box_tests.rs` の `anchor_in_box_is_hovered_and_choice_comes_first` | 同じ種類の範囲どうしは重ならない。包まれた選択肢の文字は、アンカーの Reference0 にも入る（10） |
| 8 | 既定の見た目 | マウスが乗っていないアンカーの文字に**下線**を引く（見た目は 1 種類だけ）。マウスが乗っている間は、選択肢の行に乗ったときと同じ強調にする。普通のバルーンと箱で同じ。作者の `\f[anchor*]`・`\f[anchor.font.color]`・descript の `anchor.*` 族は、今までどおり受け取って保持するだけで、見た目を変えない | `areka_discretion` | Q-7 の既定値 `underline` を**借りた**。Q-7 は「選択中アンカーの形状」の既定値であり、マウスが乗っていないときの見た目に使うのは areka の裁定である（areka は `anchor.style` そのものをまだ読まない）。開発者確認 2026-10-10（要件討議 議題 1・要件 5.1: 下線がバルーンの中で初めての、常に見える目印になることを了解の上で採用）。下線を決めるのは `crates/areka-emo-text/src/state_decoration.rs` の `push_current_style`。テストは `state_anchor_tests.rs` の `underline_number_covers_only_the_characters_inside_the_range`・`box_text_gets_the_same_underline_numbers`、`crates/areka/src/input_events/balloon_pointer_handler_tests.rs` の `moved_off_an_anchor_injects_the_highlight_off` | 下線の位置は、既に在る文字の下線と同じ規則（縦書きの位置の決め直しは `areka-P0-anchor-style-canon`）。作者の指定で見た目を変えるのも同 spec（§3） |
| 9 | 範囲の中で作者が下線を切ったとき | 範囲の中に `\f[underline,false]` を書いても、**アンカーの下線が勝つ**。作者のほかの指定（太字など）は範囲の中でも効く。作者の装飾の状態そのものは変えないので、閉じた後の文字にも、`\f[default]`／`\f[disable]` の戻し先にも、アンカーの下線は残らない | `areka_discretion` | 正典沈黙（要件 5.1・5.4）。テストは `state_anchor_tests.rs` の `anchor_underline_wins_over_author_underline_off`・`author_decoration_state_is_untouched_by_the_anchor`・`bulk_reset_restores_the_same_look_with_or_without_the_anchor` | 「押せる所には必ず下線がある」を保つための決め。`areka-P0-anchor-style-canon` が下線の決め方を差し替えるときに見直す |
| 10 | Reference0 の文字（`OnAnchorSelectEx`） | 開きから閉じまでに、開いた場所へ追記された文字を、書記素クラスタの単位でそのまま連ねたもの。改行・`\f`・`\_l` は入らない。アンカーが選択肢を包んでいれば、選択肢の文字も入る。押した時点までに文字の層へ届いた分が載り、その中には、1 字ずつ出ている途中でまだ見えていない字も入る（押せるのは見えている字の上だけ） | `areka_discretion` | Q-3 は「ジャンパとなったテキスト」と書くだけで、改行・装飾・表示の途中の扱いに沈黙（要件 1.7・4.1）。範囲の文字を伸ばすのは `crates/areka-emo-text/src/state_anchor.rs` の `extend_open_anchor` の 1 か所。テストは `state_anchor_tests.rs` の `open_text_close_records_the_range_and_its_text`・`range_counts_grapheme_clusters`・`newline_and_decoration_between_join_only_characters`・`anchor_wrapping_a_choice_includes_the_choice_text`、`crates/areka-emo-text/src/choice_anchor_tests.rs` の `hit_area_stops_at_what_has_been_revealed` | 範囲の文字が伸びるのは文字の合図が文字の層に届いたときなので、まだ届いていない後ろの文字は、押した時点の Reference0 に入らない。話している最中に押したときの Reference0 を直に固定するテストは無い（0 本。`hit_area_stops_at_what_has_been_revealed` は、届いた合図の全文ぶんが範囲に入ることまでを見る）。別のスコープや別の箱へ書いた文字の扱いは 2.2 の D-2 |
| 11 | 引数が無いときの位置 | 引数が 1 つも無ければ、対応する Reference の位置を**作らない**（空の文字列で埋めない）。`OnAnchorSelectEx` は Reference1 で終わり、`On` で始まる ID のイベントは Reference が 0 個になる。引数の途中の空のトークン（`\_a[ID,,x]`）は潰さず、空の文字列のまま位置を保つ | `areka_discretion` | 正典（Q-3）は、続く引数が Reference2 以降に入ると書くだけで、引数が無いときの位置に沈黙（要件 4.6）。選択肢と同じ規則（`choice-cascade-compat.md` の R-5）。テストは `events_anchor_tests.rs` の `on_anchor_select_ex_without_arguments_stops_at_reference1`、`anchor_step_tests.rs` の `on_prefixed_id_sends_only_the_event_of_that_name`・`canonical_id_sends_select_ex_then_select_and_then_nothing`、`decode_anchor_tests.rs` の `empty_tokens_are_kept_in_place` | — |
| 12 | 応答を待っている間の 2 回目の押下 | kanade は、1 件の知らせについて「送る → 応答を受ける → 次の段を送る」を一続きで済ませてから、次の知らせを読む。途中に別の押下は割り込まないので、応答待ちの間の 2 回目を断る分岐は無い（0 か所）。1 回の知らせにつき、送るイベントの列は 1 本、台詞の起動は高々 1 回 | `areka_discretion` | 正典沈黙（内部の処理の順・要件 4.11）。一続きの処理は `crates/areka-kanade/src/actor.rs` の `drive_translating`。どの段の応答を待っているかの記憶（`State.anchor`）は、応答で必ず取り出される。テストは `anchor_step_tests.rs` の `one_notification_makes_one_event_chain_and_at_most_one_talk` | 利用者が続けて 2 回押せば、知らせは 2 件として順に処理される（2.2 の D-4） |
| 13 | 消えたバルーンのアンカーとの照合 | アンカーには、選択肢のような選択待ちの記録（候補・期限・台詞の番号との照合）を**持たない**。届いた知らせは全件そのまま送る。知らせが作られてから kanade が受け取るまでの隙間に台詞が置き換わっても、利用者が押したアンカーのイベントを送る。バルーンの本文が消えた後は範囲が無いので、知らせそのものが作られない | `areka_discretion` | 正典沈黙。開発者裁定 2026-10-10（要件討議 議題 3・要件 4.12: 照合の鍵を新設しない）。受理は `anchor.rs` の `on_anchor`（覚えるのは段だけ）、振り分けは `crates/areka/src/input_events/choice_drain.rs` の `forward_all`（テスト `forward_all_routes_each_kind_to_its_own_message_in_arrival_order`）。消去は `actor_clear_atomicity_tests.rs` の `clear_drops_anchor_hit_rows_and_hit_active_atomically` | アンカーは選択待ちの印（`Status` の `choosing`）を立てない（`anchor_step_tests.rs` の `anchor_never_raises_the_choosing_mark`） |

### 2.2 実装で決めたこと（正典・設計のどちらも黙っていた所）

| # | 裁定項目 | 挙動 | provenance | 正典引用・根拠 | 反証・注記 |
|---|---|---|---|---|---|
| D-1 | 重なりの開きが閉じられないまま終わる（`\_a[x]あ\_a[y]い`） | 2 つ目の開きの位置に、警告が **2 件**付く（`anchor_reopened` → `anchor_unclosed` の順）。閉じは、2 つ目の開きの前と表示の終わりの 2 か所に補う。同じ位置に同じ種類の警告は重ならない | `areka_discretion` | 要件 1.8 と 1.9 がそれぞれ警告 1 件を求めるので、両方を出す（設計の当初の「同じ位置に 2 件は付かない」を改めた）。`anchor_pair.rs` の `pair_anchors`。テストは `compile_anchor_tests.rs` の `reopened_and_never_closed_gets_both_supplied_closes_and_two_warnings` | `check_script` の診断も同じ開きに 2 件（D-7） |
| D-2 | アンカーを開いたまま、別のスコープや別の箱へ書いた文字 | 開いた場所と別の場所へ書いた文字は、範囲に入らず、Reference0 に入らず、下線も付かない（`\_a[x]あ\1い\_a` の範囲の文字は「あ」）。開いた場所へ戻って書いた文字は、範囲に続く（`\_a[x]あ\1い\0う\_a` は「あう」）。閉じは、どのスコープで書かれても、開いている範囲を閉じる。開いているアンカーは台本の全体で高々 1 つ | `areka_discretion` | 要件 1.7 の「開きから閉じまでに表示される文字の並び」を、場所ごとに読んだ。押せる所と下線の所を一致させる。`state_anchor.rs` の `TextLayerState::open_anchor`・`close_anchor`。テストは `state_anchor_tests.rs` の `close_addressed_to_another_scope_closes_the_open_range`・`close_after_a_destination_switch_closes_the_open_range`・`text_back_in_the_open_place_continues_the_range`・`text_in_another_place_while_open_is_not_underlined` | 閉じが開いた場所と別の場所宛てに届いても、後ろの文字が黙って範囲に入ることは無い |
| D-3 | アンカーを受ける時期 | アンカーの選択を受けるのは**定常の間だけ**。終了の挨拶・ゴーストの切り替えの送り出しの台詞・切り替えの別れの台詞の中のアンカーは、バルーンに出ていて押せても、何も送らない（0 本）。起動の途中・終了の処理中も同じ。警告 `anchor_rejected_phase`（`id`・`scope`・そのときの相 `phase`）を 1 件残して棄却する | `areka_discretion` | 正典沈黙（要件 4 は時期を限っていない）。判定は `anchor.rs` の `on_anchor`。テストは `anchor_step_tests.rs` の `anchor_outside_steady_is_rejected_and_nothing_is_queued`（9 つの相）、`anchor_tests.rs` の `anchor_outside_steady_is_rejected_with_a_warning` | 棄却するのは kanade の側。押下を受ける側（`crates/areka/src/input_events/`）は相を見ないので、発行の記録 `anchor_selected` は出る |
| D-4 | アンカーの上の左ダブルクリック | **2 回の選択として送る**。選択の確定はダブルクリックかどうかを見ず（選択肢と同じ）、アンカーには選択待ちの記録が無い（2.1 の 13）ので、2 件目を落とす所が無い。最初の答えが 204 なら、ゴーストは `OnAnchorSelectEx`（→ `OnAnchorSelect`）を 2 回受ける。答えが台本なら 1 打目で台詞が置き換わり、置き換えで範囲が消えた後の 2 打目は何も送らない | `areka_discretion`・**確認待ち** | 正典はダブルクリックの扱いに沈黙。普通のバルーンは `crates/areka/src/input_events/balloon_pressed.rs` の `on_balloon_pointer_pressed`（選択の確定は `double_click` を見ない）、箱は `crates/areka/src/input_events/shell_box_handler.rs` の `press_with_point`（選択の判定が先）。要件 4.11（知らせ 1 件につき列 1 本）とは矛盾しない。2 打目が中断にならないことは `user_break_tests.rs` の `second_click_after_selecting_an_anchor_is_not_a_break` | **開発者への確認待ち**: 2 回目を抑えるかどうかは未確認で、確認が済んだらこの行を改める。2 打目も選択になることを押下の入口から通して固定するテストは無い（0 本）。答えが台本のときに 2 打目が届くかどうかは、置き換えが 2 打目より先に画面へ届くかで決まる |
| D-5 | 中身の無い対だけの台本（`\_a[x]\_a`） | 合図を 2 つ（開きと閉じ）持つので、**空の台本ではない**。`\q`・`\f` だけの台本と同じく、先頭で全員のバルーンの本文を消す（`ClearAll`）。開いていない閉じ `\_a` だけの台本は、合図が 1 つも無く、空の台本のまま（本文を消さない） | `areka_discretion` | 正典沈黙。`crates/areka-sakura/src/compile.rs` は、合図を 1 つでも持つ台本の先頭に `ClearAll` を置く。閉じだけの台本が空であることは `compile_anchor_tests.rs` の `stray_close_emits_nothing` | 本 spec の前は `\_a` を読み捨てていたので、どちらも空の台本だった。中身の無い対だけの台本を直に固定するテストは無い（0 本） |
| D-6 | 終了や切り替えの保留中の受理と、応答の出所の名前 | 終了や切り替えの要求が保留になっている間（話し終わるのを待っている間）も、定常であればアンカーを受ける（選択肢と同じ。保留を見る分岐は 0 か所）。`On` で始まる ID のアンカーの応答に付く出所の名前（記録用）は、選択肢の `On` 始まりと同じ `"OnChoiceEvent"` | `areka_discretion` | 正典沈黙（内部の記録の名前）。`anchor.rs` の `on_anchor` は相だけを見る。出所の名前は `actor.rs` の `EventId::Choice` の分岐。`anchor_step_tests.rs` の `send_failure_advances_like_no_content_with_one_error_and_no_fault` は、記録が応答の出所の名前をそのまま載せることを見る（名前はテストが自分で入れている） | 保留中の受理を固定するテストは無い（0 本・抑止の分岐を置かないという決め）。`On` で始まる ID のアンカーに `OnChoiceEvent` の名前を付ける所を固定するテストも無い（0 本）。記録でアンカー由来かどうかは、名前（`anchor_accepted`・`anchor_shiori_failed_as_204`）で見分ける |
| D-7 | `check_script`（台本を再生せずに確かめる道具）の診断 | 崩れた 3 形（2a〜2c）を、再生と同じ判定（`pair_anchors`）で、種類 `unpaired_tag` の診断として返す。D-1 の形は同じ開きに 2 件。`\e`・`\-` より後ろの開き／閉じには何も出さない（再生でも読まれない）。4 つの形のどれも「知らないタグ」「誰も拾わない命令」と診断しない（0 件） | `areka_discretion` | 正典の外（areka の道具・要件 6.1〜6.3）。`crates/areka/src/mcp/check_script_judge.rs` の `diagnose`。テストは `crates/areka/src/mcp/check_script_judge_tests.rs` の `the_four_anchor_forms_are_not_reported`・`row15_anchor_without_a_close`・`row16_anchor_opened_while_another_is_open`・`row17_close_without_an_open_anchor`・`reopened_and_never_closed_gives_two_diagnostics_on_the_second_open`・`anchors_after_end_or_quit_are_not_reported` | `check_script` のほかの診断は `\e`・`\-` で打ち切らない（書いてある誤りは知らせる）が、開きと閉じの対応の崩れだけは再生に合わせる。診断の表は `doc/ssp-mcp/areka-tools.md` |
| D-8 | アンカーが開いている間に本文が消されたとき（`\_a[x]あ\cい\_a`） | 本文の消去（`\c`）で、それまでの範囲と「開いている」印が一緒に消える。消去の後ろの文字（「い」）は範囲に入らず、下線も付かず、押せない。後から届く閉じは何もしない。開きと閉じの対は整っているので、再生の警告も `check_script` の診断も出ない（0 件） | `areka_discretion` | 正典沈黙（アンカーの途中で本文を消す形）。要件 2.7（本文が消えればアンカーも消える）を、開いている途中にも当てはめた読み。`crates/areka-emo-text/src/state_decoration.rs` の `clear_content` が範囲の列と印を一緒に消す。テストは `crates/areka-emo-text/src/state_anchor_tests.rs` の `clear_drops_the_ranges_and_the_open_mark` | SSP は消去の後ろもアンカーとして続ける可能性がある（未確認）。実機の確認の後に開発者へ見せる項目。消去の後ろの文字が範囲に入らないことを直に名指すテストの名前は上の 1 本 |

### 2.3 要件 7.2 が挙げる 7 項目と本表の行

| 要件 7.2 の項目 | 本表の行 |
|---|---|
| 話している最中のアンカーのクリックの扱い（要件 3.4） | 2.1 の 1 |
| 閉じの無い開き／開きの重なり／開き無しの閉じの扱い（要件 1.8〜1.10） | 2.1 の 2a・2b・2c（と 2.2 の D-1） |
| ID が空のときの扱い（要件 1.11） | 2.1 の 3 |
| ID の綴りに意味を持たせないこと（要件 4.1） | 2.1 の 4 |
| `OnAnchorSelect` を 204 のときだけ続ける読み | 2.1 の 5 |
| 選択肢とアンカーが重なったときの判定の順（要件 3.5） | 2.1 の 7 |
| マウスが乗っていないときの既定の見た目を descript `anchor.style` の既定値から借りたこと（要件 5.1） | 2.1 の 8 |

---

## 3. 後の spec への波及と申し送り

### 3.1 後の 4 spec

本 spec が作った「範囲を押せる仕組み」の上に、次の 4 本が乗る。本表の裁定を変えるときは、これらの前提が変わる。

| 後の spec | 本 spec の上に乗る所・差し替える所 |
|---|---|
| `areka-P0-anchor-style-canon`（見た目の作者指定） | 2.1 の 8・9 を差し替える。下線を決めている 1 か所（`state_decoration.rs` の `push_current_style`＝アンカーが開いている間、見た目の写しに下線を立てる）を、`anchor.style` の解決に置き換える。`\f[anchor*]` 16 項目・`\f[anchor.font.color]`・descript の `anchor.*` 族の意味付け、訪問済みの記録、縦書きの下線の位置の決め直しを持つ。それまで、これらの指定は受け取って保持するだけ（`ActorTextState::unowned_vocab`） |
| `areka-P0-range-choice-tag`（`\__q` の範囲の選択肢） | 開きから閉じまでの文字を 1 つの範囲にする記録（`state_anchor.rs`）と、選択肢とアンカーが種類違いで並ぶ共通の範囲の列（`ChoiceSpan` の `kind`＝`SpanKind`）の上に乗る。開きと閉じの対応の判定（`pair_anchors`）、崩れた形と別の場所へ書いた文字の扱い（2.1 の 2a〜2c・2.2 の D-1・D-2）を揃えるかどうかは、その spec で決める |
| `areka-P0-balloon-link-hover`（`OnAnchorHover`・行き先の説明） | 当たりの行（`ChoiceHitRow`）が種類・ID・範囲の文字・引数を持つので、アンカーの上にマウスがあることと、イベントに載せる材料をそこから読める。ホバーの判定の順（2.1 の 7）と Reference0 の文字の決め（2.1 の 10）を共有する |
| `areka-P0-link-context-copy`（右クリックでの行き先のコピー） | 今は、右ボタンと中ボタンの押下をアンカーの選択にしない（普通のバルーンは `on_balloon_pointer_pressed` が左ボタンだけを受ける。箱は `shell_box_handler_tests.rs` の `right_press_on_box_anchor_is_not_a_selection`）。右クリックの入口を足すときは、同じ当たりの行（種類・ID・引数・範囲の文字）を読む |

4 本が前提にしてよいもの:

- 押せる範囲の記録 `ChoiceSpan`・当たりの行 `ChoiceHitRow`・選択の知らせ `ChoiceSelection` は、種類 `SpanKind`（`Choice`／`Anchor`）の欄を持つ。型の名前は選択肢だけだった頃のままである。
- 通し番号（`ordinal`）は選択肢とアンカーで共通。同じ種類の範囲は重ならず、アンカーは選択肢を包んでよい（`choice_anchor_tests.rs` の `same_kind_ranges_are_disjoint_and_an_anchor_may_wrap_a_choice`）。
- `TextLayerRuntime::choice_active` は「選択肢の範囲があるか」（バルーンの時間切れを止める観測が読む）、`TextLayerRuntime::hit_active` は「押せる範囲があるか（種類を問わない）」（ホバーと押下の前段が読む）。どちらを読むかを変える spec は、A-5 を確かめる。

### 3.2 申し送り

- **`dola` の公開（`areka-P0-release-cycle` の記録へ）**: 合図の語彙 `CueCommand`（`crates/dola/src/cue/command.rs`）に 2 種類——`AnchorBegin { id, references }` と `AnchorEnd`——を足した。`dola` は crates.io に公開しているので、次の公開は公開 API の追加になる。`CueCommand` は `#[non_exhaustive]` ではないので、全種類を catch-all なしで並べている利用側は、分岐を 2 つ足す必要がある。`references` が空なら直列化でキーを出さない（`Choice` と同じ規約）。
- **`check_script` の診断の種類**: `unpaired_tag` が 1 つ増えた（D-7）。診断の表（`doc/ssp-mcp/areka-tools.md`）には 3 行（閉じ無し・重なり・開いていない閉じ）が載っている。
- **生成正本**: `doc/shiori/fragments/` 配下は生成物であり手編集しない。正典側の記述が更新されたときは生成元を更新し、本表の `provenance` と反証の欄を見直す（`COMPAT_ARCHITECTURE.md` §2「ukadoc 更新時は正典に従い是正」）。
