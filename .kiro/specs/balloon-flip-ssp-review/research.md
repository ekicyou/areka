# Research & Gap Analysis: balloon-flip-ssp-review

> 2026-11-20 `/kiro-validate-gap`（`/kiro-start` の中の下請け）。main `9bf6e32e` の上の、このワークツリーのコードを読んだだけ。SSP と areka の測り・画面の入力はしていない。ukadoc は MCP の検索で `windowposition.limit` の項目を 1 件引いた。

## 1. 要約

- 並べ替え・押し込み・重なりの判定は、どれも `crates/areka-placement/src/balloon_limit.rs` の純関数 `place_balloon_in_work_area`（172 行目）1 本に集まっている。呼び手は起動の関門 `apply_balloon_limit_one`（同 291 行目）と書き込みの関門 `apply_balloon_limit_gate`（`follow/window_move.rs` 984 行目）の 2 つだけ。バルーンのドラッグを離したときの補正（`follow/balloon_drag.rs` 497 行目）は `limit_correction` だけを呼び、並べ替えに入らない。どの案でも、直す所はこの 1 本の関数と、その戻りの型 `BalloonPlace` で足りる。
- 押し込みの式 `limit_correction`／`clamp_rect_to_work_area` は、キャラクター窓の寄せ（`follow/work_area_clamp.rs` 379 行目・`follow/move_shift.rs`）も使う。どの案でも、この 2 つの式は変えない（変えると要件 8.2 のキャラクター窓の決まりが変わる）。
- 案 ⑴ でも、要件 7.3〜7.5 の決定論のテスト（10-30 の形・11-16 のドラッグの後の形・どの案でも変わらない形）は今のテストに無いので、足す必要がある。⑴ は「§8 の書き直しだけ」ではなく「§8 の書き直し＋テストの追加」。
- 案 ⑵ は `place_balloon_in_work_area` の並べ替えの腕を外すだけで、押し込みの結果は今の `Pushed` と同じになる。今の押し込みのテストの多くは、10-30 に並べ替えを避けるためキャラクター窓をはみ出させる「前提合わせ」をしてあるので、⑵ では緑のまま残る見込み。赤になるのは並べ替えそのものを固定したテスト（主に `balloon_limit_flip_tests.rs`）と、判定し直しの 3 本。
- 案 ⑶ は新しい分岐（横にだけはみ出すときは動かさない）と、「はみ出したまま置いた」記録が要る。今の押し込みのテストのうち、キャラクター窓が収まっていて横にだけはみ出す形を押し込みで固定しているもの（`balloon_limit_gate_tests.rs` の「左辺のみ」「右辺のみ」・`crates/areka/src/main_restore_seam_tests.rs`）が赤になる。⑶ は縦と横の両方にはみ出す形の扱いと、記録の出し方（毎回の書き込みで出るか）を決める必要がある。

## 2. 今のコード（Current State）

### 2.1 決まりを持つ所と呼び手

| 所 | 行 | 役目 |
|---|---|---|
| `balloon_limit.rs` `clamp_rect_to_work_area` | 106 | 4 辺を作業領域へ収める式（キャラクター窓の寄せと共有） |
| `balloon_limit.rs` `limit_correction` | 122 | 収める必要があるときだけ `Some(収めた位置)`（キャラクター窓の寄せ・ドラッグを離したときの補正と共有） |
| `balloon_limit.rs` `BalloonPlace` | 133 | `Fits`・`Flipped`・`Pushed`・`PushedOverlap { to, scope }`。`how()` は `fits`／`flipped`／`pushed`、`overlap_scope()` は `PushedOverlap` だけ `Some` |
| `balloon_limit.rs` `place_balloon_in_work_area` | 172 | ① `limit_correction` が `None` なら `Fits` ② キャラクター窓が収まり横の位置が動くなら、縦の中心線で映した位置を試す ③ 映した先が横に入り、ほかのキャラクター窓に重ならなければ `Flipped`、重なれば `PushedOverlap` ④ それ以外は `Pushed` |
| `balloon_limit.rs` `counted_overlap` | 217 | 重なりの判定（自分の窓に収まる部分は数えない） |
| `balloon_limit.rs` `apply_balloon_limit`／`apply_balloon_limit_one` | 270／291 | 起動の関門。全スコープのキャラクター窓の矩形の列を組んで判定に渡す。`limit=0` は素通し、作業領域が決まらなければ `[balloon-limit] Unresolved` の `warn!` で素通し、動かしたときだけ `[balloon-limit] Clamp` の `info!`（`how`・`overlap_scope`） |
| `follow/window_move.rs` `apply_balloon_limit_gate` | 984 | 書き込みの関門。`enqueue_window_set_pos`（706 行目）の中で毎回のバルーンの書き込みに当てる。決まらないものごとに `Unresolved` の `warn!` で素通し。判定の列は `shown_char_rects`（1078 行目） |
| `follow/other_chars.rs` `shown_char_rects`／`is_shown` | 22／42 | 表示しているキャラクター窓の矩形の列（箱の文字の面 `emo-text-box-surface` は数えない） |
| `follow/other_chars.rs` `register_balloon_rejudge`／`rejudge_all_balloons` | 54／70 | 判定し直し。`FrameFinalize` に `run_if(resource_added::<ChainFinalized>)` で載せる系と、`reset_pos.rs` 134 行目・`chain_realign.rs` 427 行目からの直の呼び出し。中身は全スコープの `follow_balloon(.., Placement(MoveCue))` |
| `crates/areka/src/ghost_session.rs` | 100 | `register_balloon_rejudge(world)` の 1 行（このウェーブでは触らない） |
| `follow/balloon_drag.rs` `apply_release_limit_correction` | 424・497 | 利用者がバルーンを離したときの補正。`limit_correction` だけ（並べ替えない・要件 8.3 で変えない） |

### 2.2 取り違えやすい同じ綴り

- キャラクター窓の鎖の寄せ（`follow/work_area_clamp.rs`）も `how="flipped"` を出すが、記録の語は `[work-area-clamp] Clamp`（39 行目）で、バルーンの `[balloon-limit] Clamp` とは別物。`crates/areka/src/runtime_tests/frame_chain_finalize_clamp_tests.rs`・`frame_chain_finalize_remembered_tests.rs`・`crates/areka-runtime/src/frame_chain_realign_adjacency_tests.rs`・`follow_work_area_clamp_chain_tests.rs` の `how="flipped"` はキャラクター窓の話で、この spec の範囲の外（要件 8.2）。どの案でも触らない。

### 2.3 ほかの安全網

- バルーンの随伴には、可視性の遷移の守り（`follow/drag_follow.rs` 448 行目 `guard_balloon_position`→`evaluate_visibility_guard`）が先にかかり、バルーンが画面から完全に見えなくなる移り方だけを止める。書き込みの関門はその後で最後の言葉を持つ（`follow_balloon_limit_tests.rs` 309 行目の檻）。案 ⑶ で「はみ出しの大きい形」がどう見えるかは、この守りが効く引き金かどうかでも変わる（3.3 の Research Needed）。

### 2.4 ukadoc

- `windowposition.limit,0/1`（https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.limit）: 「バルーンを強制的に画面内に維持するかどうか。1で維持」・既定値 1・「SSPに限り、descript.txtにも書ける」・対応 SSP 2.5.10。
- 2 点を設計と測りへ渡す: ⑴ ukadoc の語は「画面」で、areka は作業領域で収める（11-16 の測りでは作業領域の右端と画面の右端がどちらも 2880 なので、食い違いの判定には効かない）。⑵ 「descript.txt にも書ける」は、ゴースト・シェル側の descript にも書けるという読みになる。場面 ⑷（`windowposition.limit,1` の明示）は、入っているバルーンに無くても、書き足せば作れる。ただし書き足しは設定の変更に当たるので、要件 1.6 の別の許しが要る。

### 2.5 テストの置き場と大きさ

| ファイル | 行数 | 中身 |
|---|---|---|
| `balloon_limit.rs` | 350 | 本体 |
| `balloon_limit_flip_tests.rs` | 565 | 並べ替え・重なり・2 つの関門・離したときの補正・件 2（右に出すずらし） |
| `balloon_limit_gate_tests.rs` | 472 | 起動の関門の押し込み（多くはキャラクター窓をはみ出させる前提合わせ `char_overflowing`） |
| `balloon_limit_tests.rs` | 353 | `limit_correction` の式（関数を変えないので不変） |
| `follow_balloon_limit_tests.rs` | — | 書き込みの関門の押し込み（前提合わせ `char_overflowing_world`） |
| `follow_other_chars_tests.rs` | — | `shown_char_rects` 7 本・判定し直し 4 本 |
| `follow/window_move.rs` | 1,234 | 行数の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs` 72 行目）の例外表に載る。番人は「例外なのに 1,000 行以下」も赤にするが、この spec の増減は数行なので効かない |

## 3. 要件ごとの対応表（Requirement-to-Asset Map）

| 要件 | 今ある物 | 足りない物・制約 |
|---|---|---|
| 1 測り | SSP の MCP（`get_property` の `currentghost.scope(n).rect`・`currentghost.balloon.scope(n).rect`）、11-16 の手順（`completed/balloon-flip-side-rules/research.md` 9 節）、排他テストの机 | **Missing**: 4 つの場面の測りそのもの（開発者の机・許しごと）。**Constraint**: 起動の直後は起動し直し、場面 ⑷ は descript の書き足しが要りうる＝どちらも別の許し。**Unknown**: 左の画面（倍率が違う）の座標と撮った画像の対応（11-16 に読み切れなかった） |
| 1.5 areka の値 | 決定論のテストの形で、同じ矩形を入れれば今の決まりの位置が出る | **Missing**: 測った形ごとの「areka での位置」の表（実機を起こさず、テストか計算で出せる） |
| 2 裁定の記録 | requirements.md の「決めごと」の節（空） | 文書だけ |
| 3.1 案 ⑴ | 今のコードのまま | 無し |
| 3.2 案 ⑵ | `Pushed` の腕・`limit_correction` | **Missing**: 並べ替えの腕を外す直し（数行の削除）。`chars` の引数と `PushedOverlap` を残すか（議題） |
| 3.3〜3.4 案 ⑶ | `Fits` は「動かさない・記録しない」 | **Missing**: 「キャラクター窓が収まり、横にだけはみ出す」の判定（今の 191 行目の条件の一部と同じ）と、動かさない腕。**Unknown**: 縦と横の両方にはみ出す形をどう置くか（議題） |
| 3.5 `flipped` を出さない | `how()` の 1 か所 | ⑵・⑶ で `Flipped` が作られなくなれば自然に出ない（型から外すかは議題） |
| 3.6 2 つの関門で同じ | 両関門が同じ純関数を呼ぶ | 無し（関数 1 本を直せば両方に当たる） |
| 3.7 スコープ数を決め打ちしない | 純関数はスコープを見ない・`flip_is_the_same_for_every_scope` | 無し |
| 3.8 `limit=0` | 両関門の入り口 | 無し |
| 3.9 表示の位置だけ | 両関門は `balloon_pos`／書き込み位置だけを返す | 無し |
| 3.10 ⑶ と明示の `limit,1` | `BalloonLimit(bool)` は明示か既定かを持たない | **Missing（⑶ で分ける裁定のときだけ）**: 明示と既定を見分ける値。descript の読み（`config.rs`）から関門まで運ぶ道を足す必要があり、規模が 1 段大きくなる。**Research Needed** |
| 4.1〜4.2 記録 | `[balloon-limit] Clamp` の `how`・`overlap_scope` | 無し（⑴・⑵） |
| 4.3 ⑶ のはみ出したまま | 無し（`Fits` は記録しない） | **Missing**: 欄か `how` の値。**Constraint**: 書き込みの関門は毎回のバルーンの書き込みで走るので、窓のドラッグ中は毎回出る（今の押し込みの `info!` と同じ頻度）。`Fits` を記録しないのは「毎回の偽の補正の記録」を避けるためだった（`window_move.rs` 978〜983 行目）。⑶ の記録の段（`info`／`debug`）と頻度は議題 |
| 4.4 `fits` は記録しない | 両関門 | 無し |
| 4.5〜4.6 失敗の道 | 両関門の `Unresolved` の `warn!` と素通し | 無し。純関数の直しは失敗を増やさない（`Err` を返す新しい道は出ない見込み） |
| 5 §8 の行 | `doc/COMPAT_ARCHITECTURE.md` 438 行目（`balloon-flip-side-rules` の ⑶ で「見直しはこの spec」と書き足し済み） | **Missing**: 書き直し。行は 1 行に長い文を持つ表の行なので、書き直しで短くなる |
| 6 ukadoc への文案 | 無し | **Missing**: 文案（research.md の 1 節が仮置き） |
| 7.1・7.3〜7.5 テスト | 並べ替えのテストは 1108 幅・872 幅の形を使うが、10-30 の形（窓 `0,336` 1108×1368・バルーン 872×900）と 11-16 のドラッグの後の形（窓 `2524,1304,2860,1704`・バルーン 288×203）は無い | **Missing（どの案でも）**: 2 つの形 × 2 つの関門＋変わらない形のテスト |
| 7.2 赤を先に | — | ⑵・⑶ のとき、直す前に新しいテストを赤で確かめる |
| 7.6 外した段のテスト | 4 節の表 | 案ごとに扱いが要る |
| 7.7 見た目の変わる場面の数え | `completed/balloon-flip-side-rules/research.md` 2.6 節の検体の表（`balloon.alignment` とずらし） | **Unknown**: 「今は並べ替えで置かれている場面」は窓の位置しだいで、検体を読むだけでは数が決まらない。数え方（どの検体のどの側のバルーンが窓を端へ寄せたときに並べ替わりうるか）を決める必要がある。**Research Needed** |
| 8.1〜8.6 変えないもの | 鎖の寄せ・キャラクター窓の戻し・縦の置き方・キーワードの式・離したときの補正・件 2 は、どれも純関数の外か、`limit_correction` だけを使う | 件 2 のテスト（`right_from_start_equals_left_mirrored_for_descript_part`）は並べ替えを使って式を確かめているので、⑵・⑶ では確かめ方を書き換える必要がある（決まり自体は変えない） |

## 4. 案ごとの直す所と変わるテスト

### 案 ⑴ 今のまま

- **直す所**: `doc/COMPAT_ARCHITECTURE.md` 438 行目の書き直し・research.md（測りと文案）・テストの追加（要件 7.3〜7.5。置き場は `balloon_limit_flip_tests.rs`、565 行なので数十行足せる）。記録を足すか（Open Questions 8）で `balloon_limit.rs` に触るかが決まる。
- **変わるテスト**: 無し（足すだけ）。
- **ウェーブの約束**: `ghost_session.rs`・`frame/` に触らない。
- **規模**: 2〜3 タスク（テスト 1・§8 と文案 1・必要なら記録 1）。**Effort S**・**Risk Low**（足すだけで振る舞いが変わらない）。

### 案 ⑵ 押し込みだけ

- **直す所（最小）**: `place_balloon_in_work_area` の 190〜211 行目（映しの腕）を外し、`limit_correction` の結果を `Pushed` で返す。`counted_overlap`・`chars` の引数・`PushedOverlap` は通らなくなる。
  - 外し方の 2 つ: (a) 関数の中身だけ外し、引数と型は残す（使わない引数・作られない変種が残る＝読み手を迷わせる。Rust は使われない変種に警告を出しうる） (b) `chars`・`PushedOverlap`・`counted_overlap`・`Flipped` を外し、呼び手 2 か所（`apply_balloon_limit` の列の組み立て・`window_move.rs` 1078 行目の `shown_char_rects`）も外す。(b) なら `follow/other_chars.rs` の `shown_char_rects`／`is_shown`／`TEXT_BOX_SURFACE_NAME` が使われなくなり、一緒に外す（`balloon-text-visual-tree` とは、箱の文字の面の名前を読む所が減る向きの重なり）。
  - ⑵ では純関数の答えが `Fits` か `Pushed` だけになり、`place_balloon_in_work_area` は `limit_correction` と同じになる。関数ごと外して両関門が `limit_correction` を直に呼ぶ形（離したときの補正と同じ形）も取れる。
- **判定し直しの系**: 押し込みは自分のキャラクター窓と作業領域だけで決まるので、判定し直しても位置は変わらない（同じ位置の書き込みが 1 巡に数本増えるだけ）。残せばこのウェーブの約束を守れる。外すなら `ghost_session.rs` 100 行目の 1 行が要り、次のウェーブへ回る（要件 2.4）。直の呼び手（`reset_pos.rs` 134・`chain_realign.rs` 427）は `areka-placement` の中なので、このウェーブで外せる。ただし系の登録を残したまま中身だけ外すことはできない（登録の関数は `ghost_session.rs` から呼ばれている）ので、「直の呼び手だけ外し、系は残す」か「全部残す」かになる。
- **変わるテスト（赤になる見込み）**:
  - `balloon_limit_flip_tests.rs`: 並べ替えを主張する純関数・関門のテスト（`left_side_balloon_of_left_edge_window_flips_to_the_right_keeping_y`・`right_side_balloon_of_right_edge_window_flips_to_the_left`・`flipped_position_overflowing_only_vertically_is_clamped_vertically`・重なりの 7 本・`boot_gate_flips_keeps_offset_and_logs_how`・`boot_gate_pushes_on_overlap_and_logs_the_scope`・`write_gate_flips_and_returns_to_the_original_side`・`write_gate_pushes_on_overlap_with_a_shown_partner`・`write_gate_flips_when_the_partner_is_hidden_or_undetermined`・`balloon_move_locks_do_not_stop_the_flip`・`flip_is_the_same_for_every_scope`）。押し込みに書き換えるか除く。`fallback_cases_return_the_current_push_in`・`fitting_balloon_is_left_alone`・`*_does_not_flip_when_limit_is_disabled`・`release_correction_never_flips` は緑のまま。
  - 同ファイルの件 2 の 2 本（`right_from_start_equals_left_mirrored_for_descript_part`・`mirror_of_windowposition_part_keeps_current_formula`）: 映しの結果を使って式を確かめているので、映しが無くなれば書き換えるか除く（件 2 の決まり自体は `resolver_resolve_balloon_tests.rs` 226 行目以降の檻が持つ）。
  - 判定し直しの 3 本: `follow_other_chars_tests.rs` の `rejudge_after_chain_finalize_pushes_the_balloon_onto_the_moved_partner_once`・`reset_pos_tests.rs` 509 行目・`chain_realign_script_moved_tests.rs` 113 行目。どれも「先に並べ替わった後、相方が来て押し込みへ倒れる」を前提にしているので、前提が作れなくなる。系を残すなら「判定し直しても位置が変わらない」形に書き換える。
  - 前提合わせのテスト（`balloon_limit_gate_tests.rs`・`follow_balloon_limit_tests.rs`・`main_restore_seam_tests.rs`）は、押し込みの位置が今と同じなので緑のまま。前提合わせの説明（「並べ替えを避けるため」）は古くなる。
- **規模**: 4〜8 タスク。**Effort S〜M**・**Risk Low〜Medium**（削る直しで、押し込みは今の式のまま。危うさは、10-30 の形のバルーンがキャラクター窓に重なって戻ることと、檻の書き換えの量）。

### 案 ⑶ はみ出しを許す

- **直す所**: `place_balloon_in_work_area` で「キャラクター窓が収まり、横にだけはみ出す」なら動かさない腕を足し、それ以外は今の押し込み（要件 3.4）。並べ替えと重なりの腕は ⑵ と同じく外す（外し方の (a)／(b) も同じ）。
  - 縦と横の両方にはみ出す形（キャラクター窓は収まっている）は、要件 3.3（横にだけ）にも 3.4（縦にはみ出す形は押し込み）にも当たりうる。押し込み（4 辺）にするか、縦だけ収めて横ははみ出したままにするか（1 軸だけ収める式は `clamp_axis` がある）を決める必要がある。
  - 記録（要件 4.3）: 動かさない腕を `BalloonPlace` の新しい変種にして、`[balloon-limit] Clamp` に `how` の新しい値（例: 綴りは設計で決める）を足すか、別の欄を足すか。記録の語は増やさない（要件 4.2）。ただし今の 2 つの関門は「動かしたときだけ」記録し、位置を書き換える作りなので、「動かさないが記録する」腕を両関門に足す必要がある（`let … else { return }` の形を直す）。
  - 明示の `limit,1` を分ける裁定（要件 3.10）のときは、`BalloonLimit(bool)` と `ScopePlacement.balloon_limit` に「明示か」を足し、descript の読み（`config.rs`）から運ぶ直しが加わる。
- **判定し直しの系**: ⑵ と同じ（残しても位置は変わらない）。
- **変わるテスト（赤になる見込み）**: ⑵ の全部に加えて、キャラクター窓が収まっていて横にだけはみ出す形を押し込みで固定しているもの。
  - `balloon_limit_gate_tests.rs` `boot_gate_limit_arms_cover_every_edge_at_every_dpi_level`（420 行目でキャラクター窓を作業領域に収め、「左辺のみ」「右辺のみ」の 2 行が押し込みを期待する。四隅の 4 行は縦と横の両方の形＝上の議題しだい）。
  - `crates/areka/src/main_restore_seam_tests.rs` 216 行目付近（キャラクター窓 `(30,300)-(430,900)` は収まり、バルーンが左へ 20px だけはみ出す形を `(0,310)` への押し込みで固定）。`areka` の crate だが `ghost_session.rs` ではないので、このウェーブで直せる。
  - `balloon_limit_flip_tests.rs` `fallback_cases_return_the_current_push_in` の `both-sides`・`centered`（キャラクター窓が収まり横にはみ出す）は、⑶ では動かさない側に変わる。
  - そのほか、バルーンの書き込みは全部書き込みの関門を通るので、横のはみ出しを暗に作っているフレームのテスト（`crates/areka/src/runtime_tests/`・`crates/areka-runtime/src/`）が赤になりうる。静的には数え切れないので、要件 7.2 のとおり直した後の全体のテストで拾う（**Research Needed**）。
- **規模**: 6〜10 タスク（明示の `limit,1` を分けるならさらに 2〜3）。**Effort M**・**Risk Medium**（新しい分岐と記録の形、縦横の両方の形の裁定、可視性の守りとの関係、暗に横のはみ出しを作るテストの数が読めない）。

### 3 案に共通

- `limit_correction`・`clamp_rect_to_work_area` は変えない（キャラクター窓の寄せと共有）。
- 要件 7.3〜7.5 のテストはどの案でも足す。7.3（10-30 の形）は作業領域 `0,0,2880,1704` で ⑴ `(1108,336)`・⑵ `(0,336)`・⑶ `(-872,336)`、7.4（11-16 のドラッグの後の形）は ⑴ `(2441,1188)`（映し: 2524＋2860−(2655＋288)）・⑵ `(2592,1188)`（2880−288）・⑶ `(2655,1188)` になることを計算で確かめた（要件の値と合う）。
- どの案も `areka-runtime` の `frame/` に触らない。`ghost_session.rs` は判定し直しの系の登録を外すときだけ要る。

## 5. 推す方向（決めではない）

- 案の選びは測り（要件 1）の結果しだいなので、ここでは決めない。コードの側から言えるのは次のとおり。
  - ⑴ と ⑵ は、純関数 1 本の中の削除か無変更で、どちらも危うさが小さい。⑵ は外し方 (b) で関数ごと `limit_correction` に戻すと、離したときの補正と同じ形になりコードが減る。
  - ⑶ は新しい分岐と記録を持ち込み、縦横の両方の形・記録の頻度・明示の `limit,1` の 3 つの裁定が増える。
  - ⑵・⑶ とも、判定し直しの系は残しても位置が変わらない見込みなので、このウェーブでは残し、外すかは次のウェーブ（`session-module-gather` などと順を決める）に回すのが約束に沿う。直の呼び手 2 か所だけをこのウェーブで外すかは議題。

## 6. Research Needed（設計の段へ）

1. 要件 1 の 4 つの場面の SSP の振る舞い（測り）。特に場面 ⑴（はみ出しの大きい形）で SSP が戻すかどうかが ⑶ の決め手。
2. 場面 ⑷ を、入っているバルーンへの書き足しで作るか（ukadoc の「SSPに限り、descript.txtにも書ける」によりゴースト側の descript でも作れる見込み・書き足しは別の許し）。
3. ⑶ で、可視性の遷移の守り（`guard_balloon_position`）がかかる引き金とかからない引き金で、はみ出しの大きいバルーンの見え方がどう違うか（決定論のテストで確かめられる）。
4. 要件 7.7 の数え方: 窓の位置しだいで並べ替わる場面を、検体のどの値（`balloon.alignment`・`windowposition.x`・キーワード）から数えるか。キーワード（`center` など）のバルーンは映しても横の位置が変わらないので並べ替わらない（今の 200 行目）。
5. ⑶ で暗に横のはみ出しを作っているフレームのテストの数（直した後の全体のテストで拾う）。
6. 左の画面（倍率の違う画面）の座標と撮った画像の対応（11-16 に読み切れなかった・場面の測りで本体を左の画面の左端へ寄せる形を数えるなら要る）。

## 7. 要件の話し合いへの議題（ギャップ分析から出たもの）

1. **案 ⑴ でもテストは足す**: 要件 7.3〜7.5 は今のテストに無い。⑴ の規模は「§8 だけ」でなく「§8＋テスト（＋記録を足すなら記録）」。
2. **⑵・⑶ の外し方**: 並べ替えの腕だけ外して引数と型を残す (a) か、`chars`・`PushedOverlap`・`Flipped`・`counted_overlap`・`shown_char_rects` まで外す (b) か。(b) なら `overlap_scope` の欄も出なくなり（綴りは消える）、要件 8.6 の「出なくなるだけ」との関係を確かめる。
3. **判定し直しの直の呼び手**: 系の登録（`ghost_session.rs`）は残す前提で、`reset_pos.rs`・`chain_realign.rs` の直の呼び出しをこのウェーブで外すか、系と一緒に次のウェーブへ回すか。
4. **⑶ の縦と横の両方にはみ出す形**: 4 辺の押し込みか、縦だけ収めて横ははみ出したままか。
5. **⑶ の記録の形と段**: `how` の新しい値か別の欄か。書き込みの関門は毎回のバルーンの書き込みで走るので、`info` で出すと窓のドラッグ中は毎回出る。
6. **⑶ と、利用者がバルーンを離したときの補正の食い違い**: 離したときの補正は今の押し込みのまま（要件 8.3）なので、⑶ では「窓を動かすと横にはみ出したままになるが、バルーンを端で離すと押し込まれる」になる。範囲の外のまま承知するかを確かめる。
7. **件 2 のテストの確かめ方**: ⑵・⑶ では映しが無くなるので、`right_from_start_equals_left_mirrored_for_descript_part` などを、映しを使わない形（右に出した位置の式だけ）に書き換えるか除くか。件 2 の決まりは変えない。
8. **要件 7.7 の数え方**: 窓の位置しだいの場面を、何をもって「今は並べ替えで置かれている」と数えるか（Research Needed 4）。
9. **場面 ⑷ の作り方**: 入っている物に無ければ「測れなかった」で済ませるか、ゴースト側の descript に 1 行書き足して作るか（別の許し・要件 1.2・1.6）。

## 8. SSP の測り（2026-11-20・1 回目・場面 ⑴ と ⑶）

> 開発者の許し（2026-11-20「測ってよい」・範囲は `\![move]` と面の切り替え）。測る前に同じウェーブの 2 セッション（`audit-emo2-boot-frame`・`input-crate-split`）へ知らせた。SSP の MCP の `sakurascript` と `get_property` だけを使った。画面の入力・reload・インストール・設定の変更・SSP の起こし直しはしていない。

- 検体: えも2DEBUG（シェル `C:\home\maz\git\ghost_dev\debug\emo2-debug\shell\master\`）・バルーン `kakukaku for emo-gs`（`C:\wintools\ssp\balloon\emo2-kakukaku`）。SSP の版のプロパティ `system.version` は無かった（11-16 と同じ SSP 2.9.34 の見込み・版は開発者に確かめる）。
- 初めの位置: 本体の窓 `-2231,1248,-1849,1795`・バルーン `-2334,1091,-1934,1315`／相方の窓 `-2567,1395,-2231,1795`・バルーン `-2436,1279,-2148,1482`（左の画面）。初めの面は本体 1000・相方 2206（`\s[静観]`）。

### 8.1 場面 ⑶（面の切り替えでバルーンの大きさが変わるとき）はこの検体では作れない

- バルーン `kakukaku for emo-gs` の面は、本体 `balloons0.png` 400×224・相方 `balloonk0.png` 288×203 の 1 種類ずつしか無い（ほかは `balloonc1〜4.png` の通信箱）。`\b[n]` でバルーンの大きさは変えられない。
- キャラクターの面を切り替えても窓の大きさは変わらなかった（本体 `\s[20]`・相方 `\s[2110]` の後も、窓の矩形は初めと同じ）。シェルの descript に面ごとの `balloon.offset` も無い。面の切り替えでバルーンの位置や大きさが変わる形は、この検体では作れない＝**測れなかった**（要件 1.2）。
- 手違いの記録: 本体の `\s[20]` は、この検体に無い面で、本体が消えた（`surface.num` が −1）。すぐ `\s[1000]` へ戻し、相方は `\s[静観]`（2206）へ戻した。窓の矩形は終始動いていない。

### 8.2 場面 ⑴（はみ出しの大きい形）は `\![move]` だけでは作れない

- バルーンの位置は窓の端からの決まったずらしで決まる（本体は窓の左端より 103px 左・相方は窓の右端より 63px 右へ出る）。キャラクター窓を作業領域に収めたまま `\![move]` で端へ寄せても、はみ出しは本体 103px／400px・相方 63px／288px より大きくならない（11-16 の 9.2 節の形がその最大）。
- バルーンの大半を外へ出すには、バルーンのずらしを `\![set,balloonoffset,@-300,@0]`（ukadoc: descript.txt の値からの相対の指定・https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-set-balloonoffset-x-y）で一時に動かし、測った後に `@0,@0` で戻す手がある。これはバルーンの置き方を変える台本の指令なので、今回の許しの外とし、まだ打っていない。

## 8.3 SSP の測り（2026-11-20・2 回目・場面 ⑴ をバルーンのずらしで作る）

> 開発者の許し（2026-11-20「動かして測る」・`\![set,balloonoffset]` を一時に動かし、測った後に戻す）。測る前と後に同じウェーブの 2 セッションへ知らせた。画面の入力・reload・インストール・descript の書き足し・SSP の起こし直しはしていない。

- まず相方を主の画面の右端へ `\1\![move,--X=5091,--Y=-95,--base=me]` で寄せたが、縦の量が効かず（`--base=screen` の `--Y=1304` も効かない）、窓は `2524,1395,2860,1795` で作業領域の下端 1704 を越えた＝「キャラクター窓がはみ出している形」になるので使わなかった（11-16 の 9.2 節と同じ）。相方はすぐ元の `-2567,1395,-2231,1795` へ戻した。
- 本体を左の画面の左端へ寄せた（`\0\![move,--X=-1182,--Y=0,--base=me]`）。窓 `-3413,1248,-3031,1795`。SSP の座標で左の画面は `-3413,260,0,2393` なので、窓は縦も横も収まっている（作業領域は OS の値で画面と同じ＝タスクバーは主の画面だけ）。

| バルーンのずらし | 本体のバルーン（SSP の `get_property`） | 作業領域の外へ出た幅 | SSP の扱い |
|---|---|---|---|
| 元のまま | `-3516,1091,-3116,1315` | 103px／400px | はみ出したまま（11-16 と同じ） |
| `@-250,@0` | `-3766,1091,-3366,1315` | 353px／400px（88%） | はみ出したまま |
| `@-310,@0` | `-3826,1091,-3426,1315` | 400px／400px（全部が外・画面の端から 13px 離れる） | はみ出したまま＝バルーンは画面から全く見えない |
| `@-310,@0` のまま窓を `\![move]` で 1px 右へ動かして戻し、本体が話し直す | `-3826,1091,-3426,1315` | 400px／400px | はみ出したまま（窓の移動でも戻さない） |

- `@-250` のとき画面を撮った（`target\balloon-flip-ssp-review\ssp-left-edge-offset-250.png`・左の画面の左上 900×1100 を物理の座標で切り出し）。本体の左上に幅 400 のバルーンは写っていない＝押し込まれていない。外に残る 47px 分の帯は、白い背景と見分けられず写りでは確かめられなかった（値は `get_property` が正本）。
- 戻し: `\0\![set,balloonoffset,@0,@0]` と `\![move,--X=1182,--Y=0,--base=me]` の後、本体の窓 `-2231,1248,-1849,1795`・バルーン `-2334,1091,-1934,1315`、相方の窓 `-2567,1395,-2231,1795` を読み、初めと同じに戻ったことを確かめた。
- **分かったこと**: SSP 2.9.34 は、キャラクター窓が作業領域に収まっていれば、バルーンの大半が外へ出ても、全部が外へ出て見えなくなっても、バルーンを並べ替えも押し込みもしない。`windowposition.limit` を書いていない（既定 1 の）バルーンで、ukadoc の「1で維持」と食い違う（要件 6.1 の文案を作る条件に当たる）。
- **areka で同じ形を置いたときの位置**（`place_balloon_in_work_area` の式で計算・テストでは回していない）: 窓 `-3413..-3031`・作業領域の左端 −3413・バルーンの幅 400・相方の窓はバルーンと縦に重ならない。
  - 元のずらし（x = −3516）: ⑴ `(-3110,1091)`・⑵ `(-3413,1091)`・⑶ `(-3516,1091)`。
  - ずらし `@-250`（x = −3766）: ⑴ 映して `(-3078,1091)`（−3413−3031−(−3766+400)）・⑵ 押し込みで `(-3413,1091)`・⑶ そのまま `(-3766,1091)`。
  - ずらし `@-310`（x = −3826）: ⑴ `(-3018,1091)`・⑵ `(-3413,1091)`・⑶ `(-3826,1091)`（全部が外）。⑶ を採ると、可視性の遷移の守り（`guard_balloon_position`）がこの形にかかるかを設計の段で確かめる（6 節の 3）。
- **まだ測っていない場面**: ⑵ 起動の直後（SSP の起こし直しが要る）・⑷ `windowposition.limit,1` の明示（ゴースト側の descript への 1 行の書き足しと reload が要る）。⑶ 面の切り替えは 8.1 節のとおりこの検体では作れない。

## 8.4 SSP の測り（2026-11-20・3 回目・場面 ⑷ `windowposition.limit,1` の明示）

> 開発者の許し（2026-11-20「④を測る」）。測る前に同じウェーブの 2 セッションへ知らせた。画面の入力・インストール・SSP の起こし直しはしていない。

- 書き足した所: 使っているバルーンの置き場は `currentghost.balloon.path` で `C:\wintools\ssp\balloon\emo2-kakukaku\`（同じ名前 `kakukaku for emo-gs` の `emo-kakukaku` もあるが使われていない）。ukadoc は `windowposition.limit` を「Balloon設定 > descript.txt」に置き「SSPに限り、descript.txtにも書ける」と書くので、そのバルーンの `descript.txt` の末尾に `windowposition.limit,1` を 1 行足した（要件 1.2 の「ゴースト側の descript」から読み替えた。足す前の写しは `target\balloon-flip-ssp-review\emo2-kakukaku-descript.txt.bak`）。`reload`（balloon）の後、誤りの記録は無かった。
- 測った形: 2 回目（8.3 節）と同じく本体を左の画面の左端へ寄せ（窓 `-3413,1248,-3031,1795`）、ずらしを動かした。

| バルーンのずらし | 本体のバルーン | 外へ出た幅 | SSP の扱い |
|---|---|---|---|
| `@-250,@0` | `-3766,1091,-3366,1315` | 353px／400px | はみ出したまま（明示なしと同じ値） |
| `@-310,@0` | `-3826,1091,-3426,1315` | 400px／400px（全部が外） | はみ出したまま（明示なしと同じ値） |

- 戻し: ずらしを `@0,@0`、窓を元へ戻し、`descript.txt` を写しから書き戻して、ハッシュが写しと同じことを確かめ、もう一度 `reload`（balloon）した（誤りの記録なし）。本体の窓 `-2231,1248,-1849,1795`・バルーン `-2334,1091,-1934,1315`・相方の窓 `-2567,1395,-2231,1795` を読み、初めと同じに戻ったことを確かめた。
- **分かったこと**: `windowposition.limit,1` を明示に書いても、SSP 2.9.34 は既定のときと 1 ピクセルも違わず、バルーンを画面の中へ戻さなかった。SSP は明示と既定を分けていない（決めごとの 7 により、⑶ を採っても分けない）。
- **確かめられなかったこと**: SSP がこの行を読んだかどうかは、振る舞いに差が出ないので外からは見分けられない（`balloons0s.txt` には書いていない）。

## 8.5 測りのまとめ（2026-11-20）

| 場面 | 結果 |
|---|---|
| ⑴ はみ出しの大きい形 | 測った。88% でも全部でも SSP は戻さない（8.3 節） |
| ⑵ 起動の直後 | まだ測っていない（SSP の起こし直しが要る） |
| ⑶ 面の切り替えでバルーンの大きさが変わる | この検体では作れない＝測れなかった（8.1 節） |
| ⑷ `windowposition.limit,1` の明示 | 測った。明示でも既定と同じく戻さない（8.4 節） |

- SSP 2.9.34 は、キャラクター窓が作業領域に収まっていれば、バルーンが横にどれだけはみ出しても並べ替えも押し込みもしない。ukadoc の `windowposition.limit` の既定 1「バルーンを強制的に画面内に維持」と食い違う（要件 6.1 の条件に当たる）。

## 9. ukadoc の製作元への報告の文案（要件 6・送るのは開発者）

> 画像は `target\balloon-flip-ssp-review\ssp-left-edge-offset-250.png`（送るときに添える・リポジトリには入れない）。

---

**件名**: `windowposition.limit` の既定値 1 の説明と、SSP 2.9.34 の振る舞いの違い

ukadoc の `windowposition.limit,0/1`（https://ukatech.shillest.net/ukadoc-spec/descript_balloon.html#windowposition.limit）は、次のように書いています。

> バルーンを強制的に画面内に維持するかどうか。1で維持、0で維持しない(画面から出そうでも無視)
> 既定値: 1

SSP 2.9.34 で測ったところ、既定のとき（バルーンに `windowposition.limit` を書いていないとき）も、バルーンの descript.txt に `windowposition.limit,1` を明示に書いたときも、バルーンは画面の中に保たれませんでした。

- **検体**: ゴースト えも2DEBUG（シェルは master）。バルーン `kakukaku for emo-gs`（本体のバルーン 400×224）。
- **画面**: 2 枚。測ったのは、左の画面（SSP の座標で `-3413,260,0,2393`）の左端です。
- **手順**:
  1. `\0\![move,--X=-1182,--Y=0,--time=0,--base=me]` で、本体の窓を左の画面の左端へ寄せました。窓は `-3413,1248,-3031,1795` で、画面の中に収まっています。
  2. `\0\![set,balloonoffset,@-250,@0]` か `@-310,@0` でバルーンを外側へずらし、本体に話させました。
  3. `currentghost.balloon.scope(0).rect` を読みました。
- **結果**:

| ずらし | バルーンの矩形 | 画面の外へ出た幅 |
|---|---|---|
| 元のまま | `-3516,1091,-3116,1315` | 103px |
| `@-250` | `-3766,1091,-3366,1315` | 353px |
| `@-310` | `-3826,1091,-3426,1315` | 400px（全部・画面から見えない） |

- 窓を `\![move]` で動かした後に話させても、同じ値でした。`windowposition.limit,1` を明示に書いても同じ値でした。
- マウスのドラッグで窓を主の画面の右端へ寄せた形（2026-11-16）でも、バルーンは右へ 63px はみ出したまま出ました。そのとき SSP は、窓のほうを作業領域の下端へ押し上げています。

「1で維持」が、SSP の今の振る舞い（キャラクター窓が画面に収まっていれば、バルーンは画面から出ても戻さない）と合っていないように見えます。説明の改め（どの場面で維持するのか）か、SSP の側の確かめをお願いできれば幸いです。

---
