# audit-seriko 6.1 audit のかけ直し（2 回目）

- HEAD: `6394c98f`・枝 `claude/kiro-start-audit-seriko-8ceffb`
- 道具: `ponytail:ponytail-audit`。範囲は `crates/areka-seriko/` だけ。要件 1.2・1.3・5.1 に従い、指摘の数に上限を置かずに全部を挙げた。「見ていない」ファイルは無い
- 読んだファイル: 87 本、計 27,972 行（`src/` 78・`tests/` の Rust 6・fixtures 2・`Cargo.toml`）。8 人の読み手で分けて、どのファイルも 1 行目から最後の行まで読んだ。本番のコードの根拠（`timeline.rs`・`looper.rs`・`parts.rs`・`calls.rs`）は取りまとめ役が読み直して確かめた
- 動かしたもの: `cargo clippy -p areka-seriko --all-targets --no-deps -- -D warnings`（警告 0・終了コード 0）・`cargo doc -p areka-seriko --no-deps`（警告 20 件）・`cargo test -p areka-seriko --lib parts`（75 件緑・読み手の 1 人が回した）。全体テスト・`--ignored`・areka-impl-watch.exe は回していない。コードは 1 バイトも変えていない
- 扱わないもの: research.md §3 の指摘 1〜20（18 `emit_display` の包み・20 `trigger.rs` の `ponytail:` の注は「残す」の裁定どおり）と、reaudit-1.md の R1〜R26（18b1be40..6394c98f で直し済み）。直しが足りない所と、直しが生んだ所だけを挙げた

## このクレートがすること・想定した負荷

SERIKO（シェルの面のアニメーション）を、ゴーストごとの役者のスレッドで 16ms ごとに進め、表示の指令を出す。台本の合図（`\s`・`\b`・`\![bind]`）を受けて面と着せ替えを替える。`--offline-dump` のために面を進め切る道も持つ。想定した負荷は、利用者 1 人・ゴースト数体・役者 1 体あたり毎秒約 60 刻み。

## 読んだファイル（`crates/areka-seriko/` から・行数）

`Cargo.toml` 33
`src/`: actor.rs 835・actor_bind.rs 109・actor_bind_loop_tests.rs 889・actor_bind_noevent_tests.rs 275・actor_calls_tests.rs 128・actor_clock_tests.rs 240・actor_dispatch_tests.rs 867・actor_parts_tests.rs 190・actor_replace_tests.rs 429・actor_stage_tests.rs 263・actor_strict_tests.rs 222・actor_talk_tests.rs 274・actor_test_support.rs 206・actor_tick_tests.rs 365・actor_trigger_fixture_tests.rs 283・bind.rs 603・bind_forms_tests.rs 282・bind_named_tests.rs 111・bind_tests.rs 786・calls.rs 196・calls_tests.rs 328・lib.rs 76・lib_test_support.rs 107・looper.rs 987・looper_always_tests.rs 449・looper_balloon_tests.rs 302・looper_calls.rs 208・looper_calls_tests.rs 475・looper_film_count_tests.rs 90・looper_parts_emo2_tests.rs 350・looper_parts_tests.rs 309・looper_replace_tests.rs 100・looper_talk_tests.rs 881・looper_test_support.rs 67・looper_tests.rs 970・looper_trigger.rs 203・looper_trigger_tests.rs 693・output.rs 217・parts.rs 968・parts_always_tests.rs 308・parts_calls.rs 183・parts_calls_tests.rs 230・parts_film_tests.rs 363・parts_tests.rs 900・parts_trigger.rs 106・parts_trigger_tests.rs 852・resolve.rs 298・sample_test_support.rs 41・sample_test_support_tests.rs 18・settle.rs 119・settle_tests.rs 218・shell_fallback.rs 61・shell_fallback_tests.rs 141・skin_facts.rs 114・skin_facts_tests.rs 69・state.rs 774・state_bind_pattern_tests.rs 803・state_rebase_tests.rs 84・state_stage_tests.rs 139・state_surface_tests.rs 464・state_test_support.rs 9・table.rs 980・table_always_tests.rs 173・table_calls.rs 233・table_calls_check_tests.rs 122・table_calls_dot_sakura_tests.rs 131・table_calls_tests.rs 157・table_film_tests.rs 222・table_interval_words_tests.rs 123・table_parts_tests.rs 152・table_trigger_tests.rs 164・talk.rs 123・talk_tests.rs 197・timeline.rs 574・timeline_calls_tests.rs 106・timeline_repeat_tests.rs 320・trigger.rs 251・trigger_tests.rs 486
`tests/`: balloon_face_e2e.rs 157・bind_e2e.rs 410・common/mod.rs 225・cue_sequence.rs 72・loop_integration.rs 614・regression.rs 159・fixtures/trigger-intervals/README.md 65・fixtures/trigger-intervals/surfaces.txt 96

1,000 行を越えるファイルは無い（一番長いのは looper.rs 987）。

## 指摘

件数: 必ず直す 2・直したほうがよい 10・できれば 12（計 24）。

### 必ず直す（2 件）

**S1. R12 の直しが生んだもの: 再生が終わる刻みに抽選が当たると、古い再生の最後の働きのコマ（`start`・`stop`・`move` など）が当たらずに消える**（`src/looper.rs:484-492`（一番上）・`src/parts.rs:654-658`（部品））
- 何か: 境目をまたいだ刻みの抽選で「再生中か」を決める所。R12 で「再生の記録が在り、かつ `frame_at` が `Pending` か `Active`」に替えた。
- 問題:
  - 一番上も部品も、抽選（`looper.rs` の (2)・`parts.rs:654-675`）が働きのコマを当てる `run_top_calls`／`run_part_calls` より前に回る。
  - この刻みで末尾か `-1` に着いた再生は「再生中でない」になり、当たると `Playback{started_at_ms: now, acts_through: None}` で上書きされる。
  - そのあと `run_calls` が見るのは新しい再生だけ。古い再生のうち予定の時刻が（前の印, now] に入る働きのコマは、待ち行列に 1 度も入らない。
  - 直しの前は記録が残っていれば「再生中」なので、先に働きのコマが当たり、その後の進行で再生が終わっていた。
  - ついでに、古い再生の「停止」「末尾残留」の記録（tools/perf が読む行）と、`-2` の 1 回だけの warn も、その刻みでは出ない。
- 具体例: `animation0.interval,random,2`・`pattern0,overlay,100,0,0,0`・`pattern1,overlay,101,1000,0,0`・`pattern2,start,1`（待ち 0・開始から 1000 で当たる）と、`animation1.interval,never`・`pattern0,overlay,110,0,0,0`。乱数 `[0,0]`、刻み 0・1000・2000。
  - 1000 で animation0 が当たる。2000 では経過 1000 で `FinishedResidual(2)`、境目もまたぐので、もう一度当たる。
  - 期待: 2000 で animation1 が始まる（`started(1) == Some(2000)`）。実際: 始まらない。18b1be40 の前は始まっていた。
  - 16ms の刻みでは、再生を終える刻みが境目の刻みと重なり、抽選が当たったときに起きる。
- 一番小さい直し:
  - `calls.rs` に `pub(crate) fn has_due_acts(anim, start, through, now) -> bool` を足す（`push_due` を使い捨ての集合へ入れ、空でないかを返す）。
  - 一番上と部品の両方の「再生中か」を「`Pending`/`Active`、**または** `has_due_acts(..)` が真」にする。
  - 上の例の赤のテストを `looper_calls_tests.rs` と `parts_calls_tests.rs` に 1 本ずつ立てる。
  - この狭い場合だけは、R12 の前と同じく抽選が次の境目まで待つ。古い再生の働きのコマを当ててから入れ替える形もあるが、手が大きい。
- 3 種類: **(1) に触れる**。
  - 完了 spec `seriko-pattern-calls` 要件 3.1（働きのコマは予定の時刻に当たる）に反している。直しはこれを戻す方向。
  - R12 の裁定（終わっていた再生は抽選に戻す）を、「当て残しの働きのコマが無いときだけ」へ狭める。
  - (3) には当たらない（ukadoc に「同じ刻みで終わりと再抽選が重なったとき」の定めは無い）。
  - 推す裁定: **直す**（`has_due_acts` の形）。

**S2. R1 の直しが足りない: `-1` の後ろの描くコマへ進めてしまう道が 2 つ残る**（`src/timeline.rs:106-119` `frame_at`・固めているテスト `src/timeline.rs:540-557`）
- 何か: 経過の時間から今のコマを決め、`-1` で止める所。R1 で「i 以下で最後の描くコマが負なら `Stopped`」にした。
- 問題:
  - 待ち 0 のコマは前のコマと同じ締め切りを分け合い、最後のものが今のコマになる。`-1` のすぐ後に待ち 0 の**描く**コマがあると、`visible_frame` はその後ろのコマを指す。`-1` は今のコマにも描くコマにもならない。
  - もう 1 つは、刻みが `-1` の短い窓を飛び越えたとき。`-1` の待ちが 50、次のコマの待ちが 5 なら、`-1` が今のコマなのは経過 [50,55) だけ。16ms の刻みはこの窓を外すことがあり、外すと後ろのコマへ進む。刻みが止まったとき・R12 の見え直した部品も同じ。
  - `frame_at_any_negative_surface_id_is_stopped`（`timeline.rs:553-557`）は、経過 80 で `FinishedResidual(2)` を確かめていて、「`-2` の先へ進める」振る舞いを固めている。552 行のコメント「frame_at は現在コマのみ見るので」も R1 の直しで偽になった。
- 具体例: `animation0.interval,random,2`・`pattern0,overlay,100,0,0,0`・`pattern1,overlay,-1,50,0,0`・`pattern2,overlay,101,0,0,0`。
  - 期待: 経過 50 で止まり、ベースへ戻る（要件 4.3）。
  - 実際: 経過 50 で `current_frame_index` も `visible_frame` も 2（101）なので `FinishedResidual(2)`。一番上は欄に 101 を残して「末尾残留」の記録を出し、部品は `Look::Finished` から `Residual` になる。
  - 当たらないもの: `always`（`always_at`／`always_visible` は途中の `-1` の先へ進むのが要件 4.5 どおりで、`frame_at` を通らない）。働きのコマ（`push_due` は最初の負の描くコマで止まる）。
- 一番小さい直し:
  - 「i 以下で最初の負の描くコマ」を返す 1 行を足す: `frames[..=i].iter().position(|f| f.act == FrameAct::Draw && f.surface_id < 0)`。
  - `frame_at` の `Stopped` の判定をこれにする。warn のために負の番号を取り出す 2 か所（`looper.rs:286-287`・`parts.rs:146-148`）もこれに替える。
  - `timeline.rs:553-557` の期待を `Stopped` に直し、552 行のコメントを消す。上の 3 コマ（経過 50 → `Stopped`）の赤のテストを `timeline_calls_tests.rs` に 1 本立てる。
  - `frame_at` の doc（`:99-104`）と `FrameStatus::Stopped` の doc（`:91`）を「i 以下に負の描くコマが在れば」に揃える。
- 3 種類: **(3) に触れる**。
  - ukadoc（https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/ の pattern の項）では、サーフェス ID の `-1` でアニメーションが終わる。SSP も `-1` で再生を終え、後ろのコマへは進まない。
  - 今の振る舞いはこれから外れていて、直すと正典に寄る。
  - 推す裁定: **直す**。

### 直したほうがよい（10 件）

**S3. `always` の働きのコマの追いつきに上限が無く、`always` が別の `always` を `start` すると 2 乗で増える**（`src/calls.rs:96-101`・`:174-191`）
- 何か: 前の印から今までに来た働きのコマを待ち行列へ入れる所。`always` は周ごとに繰り返して入れる。
- 問題:
  - 印から長く空くと、`push_due` は `lo/period` から `now/period` までの全部の周を入れる。
  - `start` の相手が `always` だと、周ごとに相手をやり直し、そのたびに相手の今までの全部の周を入れ直す（`started` は時刻ごとに空になるので止まらない）。古い分は捨てられるが、入れる仕事は約 L²/2。
  - 空く場面: 見えない部品の終わりなしの `always` の時計は残り、印も動かない（R12 で書いたとおり）。眠りからの復帰などで刻みが止まったとき。
  - 抽選は `LotteryBoundary` でわざと 1 回だけ追いつくが、働きのコマには同じ止めが無い。
- 具体例: 周 100ms の `always` 2 つ、1 つ目に `pattern1,start,1`。1 時間空くと L = 36,000 周で、1 刻みに約 6.5 億回の挿入（固まるか、記憶が足りなくなる）。1 つの `always` に `move` があるだけなら、8 時間で 28.8 万回。
- 一番小さい直し（時刻を丸めない形）: 待ち行列に `always` の周を 1 つずつ入れる。周の最後の働きのコマを取り出したときに次の周を入れる。挿入の数は実際に当てる数と同じになり、やり直した相手の先の周は入らない。
  - 手の小さい代わり: 追いつく周の数に上限を置き、`shortcut:` の注を付ける。ただしこれは時刻の扱いを変えるので開発者の裁定が要る。
- 確かめ方: 読みで確かめた。時間は測っていない。
- 3 種類: 周を 1 つずつ入れる形なら当たらない（当てる時刻も順も変わらない）。上限を置く形なら **(1) に触れる**（「時刻は正確に扱う」の原則）。推す裁定: **周を 1 つずつ入れる形で直す**。

**S4. R12 の直しで `on_surface_changed_removes_playback` が落ちなくなった**（`src/looper_tests.rs:571-589`）
- 問題: 表は `[(2106,0),(2110,150)]`。2000 の時点の経過は 1000ms なので `FinishedResidual` になり、R12 の後の見方では「再生中でない」。`on_surface_changed` が再生を捨てても捨てなくても、2000 の抽選は乱数を引き、`calls == 2` で緑になる。
- 具体例: `looper.rs:744` の `self.playback.remove(&(scope.clone(), slot));` を消しても緑のまま。直しの前は、記録が残れば「再生中」で calls が 1 になり赤になっていた。同じ直しで `playing_anim_is_not_relotteried` は 150 → 1500 に替えたが、このテストは替えていない。
- 直し: 571 行の表を `&[(2106, 0), (2110, 1500)]` にする（2000 で `Active` のまま。捨てなければ calls が 1 で赤）。
- 3 種類: 当たらない。

**S5. `CallOutcome::then` の時刻で比べる枝にテストが無い**（`src/calls.rs:36-42`）
- 問題: `(Some(a), Some(b)) if a > b => self` の枝に入るのは、一番上と部品（または部品 2 つ）が同じ刻みで両方 `move` したときだけ。そのテストが無い（parts_calls_tests は部品だけ、looper_calls_tests は一番上だけ）。
- 具体例: `then` を `CallOutcome { shift: next.shift.or(self.shift) }` に書き替えても、全部のテストが緑のまま。後の時刻の `move` が先の時刻の `move` に負ける。
- 直し: `calls_tests.rs` に確かめ 3 つの単体テストを 1 本足す（先の時刻の `next` は負ける・同じ時刻は `next` が勝つ・`next` が `None` なら `self`）。
- 3 種類: 当たらない。

**S6. `SerikoMsg::Close` の doc が「kanade 由来の停止指令」のまま**（`src/actor.rs:84`・R4 の直し残り）
- 問題: R4 は `close()` の doc（`:159-163`）だけを「今はテストが使う・本番は送り手を全部 drop して止める」に直した。同じことを言う variant の doc が残った。ツリー全体を grep して、seriko の外で `SerikoMsg::Close`／`SerikoSink::close` を使う所は 0 件。
- 直し: 「明示の停止の口（`SerikoSink::close`・今はテストが使う）。本番は送り手を全部 drop して止める」に替える。
- 3 種類: 当たらない。

**S7. 「`name == "bind"` のみが bind パイプラインへ入る」は偽**（`src/actor.rs:622-624`・`src/actor_bind_loop_tests.rs:11`）
- 問題: 実際は `:664` で `bind` と `bind-noevent` の 2 つとも着せ替えの腕へ入る（`actor_bind_noevent_tests.rs` が固めている）。読んだ人は `\![bind-noevent,…]` が読み飛ばされると信じてしまう。
- 直し: 「`name` が `bind` か `bind-noevent` のキャリアだけが着せ替えの腕へ入る」に替える（`:561-562` の `dispatch_message` の doc は正しいので、それに揃える）。
- 3 種類: 当たらない。

**S8. R12 の直しで偽になった doc: 「エントリを持つ id が再生中＝抽選対象外」**（`src/looper.rs:110`・`:121`）
- 問題: R12 の後は、この刻みで `-1` か末尾に着いた再生は、記録が在っても抽選される。`:121`「エントリの有無が Idle/Playing を分ける」も半分しか正しくない。
- 直し: 「`frame_at` が `Pending`/`Active` のエントリが再生中（抽選しない）。この刻みで終わったエントリは抽選に戻る」に直す。S1 を直すなら「当て残しの働きのコマが在るものも再生中」を足す。
- 3 種類: 当たらない。

**S9. 採る animation の種類の doc が「`Random`/`BindRandom` のみ」のまま（R24 の直し残り）**（`src/lib.rs:14`・`src/table.rs:186-189`・`:211`）
- 問題: R24 は `LoopAnimation::trigger` の doc（`table.rs:129`）を 7 種に直したが、同じことを書いた 3 か所が残った。実際は `always`・`runonce`・`periodic`・`talk` も採る（`table.rs:224-243`。`table_trigger_tests.rs` の `three_words_are_adopted_and_debug_logged_once` が見ている）。
- 直し: 3 か所とも「[`LoopTrigger`] の 7 種を採る」に揃える（モジュールの doc `table.rs:7-28` と同じ）。
- 3 種類: 当たらない。

**S10. `state.rs` の 2 つの doc が、知らせの面と窓の閉じたバルーンについて逆のことを言う**（`src/state.rs:665-670` `shown_slots`・`:603-609` `commit_pattern`・`:108-112`／`:119` `PatternApplyOutcome`）
- 問題:
  - `shown_slots` の doc は「抽選・進行の対象の slot の列挙」と言うが、使うのは抽選の輪だけ（`looper.rs:465`）。進行は `stage_slots`（`looper.rs:541`・`:826`）。
  - `commit_pattern` の doc は「Balloon は `balloon` map を見て、`Hidden`／未知なら Unchanged」「表示中 slot にだけ届く契約」と言う。実際（`:634-640`）は `balloon_face` を見るので、`\b` が無くても知らせの面が在れば `Changed(ShowBalloon)` を返し、窓が閉じたバルーンにも毎刻み届く。
- 具体例: `state_stage_tests.rs:45` は `\b` 無し・知らせの面 4 への commit が `Changed(ShowBalloon{4})` になることを、`:78` は「窓が閉じていても指令は出す」を確かめている。どちらも doc と逆。
- 直し: `shown_slots` は「抽選の対象の列挙（進行の対象は `stage_slots`）」に、`commit_pattern` は「Shell は `scopes`、Balloon は `balloon_face`（`\b` の面か知らせの面）。面が無ければ Unchanged」に、`PatternApplyOutcome` は「面の在る slot（窓が閉じていても届く）」に直す。コメントだけ。
- 3 種類: 当たらない。

**S11. `drop_finite` のテストの doc が「面が隠れたとき用」のまま**（`src/parts_film_tests.rs:319`）
- 問題: 面が隠れたときに呼ぶのは `drop_hidden`（`looper.rs:830`・引き金の状態も消す）。`drop_finite` の本番の呼び手は `actor.rs:511` の 1 か所だけで、バルーンの窓の新しい出番に使う（引き金の状態は消さず隠すだけ）。`parts.rs:881` と `parts_trigger_tests.rs:524` は正しく書いている。
- 具体例: この doc を信じて `\s[-1]` の道に `drop_finite` を使うと、`runonce` がもう鳴らなくなる（`parts_trigger_tests.rs:527` が見ている違い）。
- 直し: 「（面が隠れたとき用）」を「（窓の新しい出番用）」に替える。
- 3 種類: 当たらない。

**S12. `surfaces_whose_words_were_refused_stay_still` の doc が偽で、9104 の回は落ちない**（`src/actor_trigger_fixture_tests.rs:175-177`）
- 問題: doc は「採らなかった animation はどれもコマを持つか `runonce` なので、採っていれば指令が出る」と言う。けれども 9104 の animation0 はコマが 0 個の `runonce`。採っても `current_frame_index` が `None` で `Pending` のままなので、何も出ない。
- 具体例: `table.rs` の「空のコマ列は採らない」の手当てを外しても、9104 の回はこのテストで緑のまま（9104 を採らないことを見ているのは `fixture_table_logs_five_adoptions_and_exactly_three_warnings` の `ids(9104).is_empty()` と warn の数だけ）。
- 直し: doc を「9103 は採っていればどこかで指令が出る。9104（コマの無い runonce）は採っても何も出ないので、採らないことは表のテストが見ている」に直す。輪から 9104 を外してもよい。
- 3 種類: 当たらない。

### できれば（12 件）

**S13. `parts.rs` の 2 つの doc が中身と違う**（`src/parts.rs:519-522` `advance`・`:942-943` `is_empty`）
- `advance`: 「窓が閉じていると今ある終わりなしの時計だけが進む」と言うが、抽選の再生中の時計も `place`→`progress` で進む。`close` は 3 語の時計も捨て、引き金の状態を隠す。→「抽選の再生と終わりなしの `always` は進み、回数つきの時計と 3 語の時計は捨て、引き金の状態は隠す」に直す。
- `is_empty`: 「`advance` が 1 度も走っていないか」と言うが、`refresh`（`:735-739`）も入れ物を作り、`clear(slot)` は別の種類の入れ物を残す。→「どのスコープ・面の種類にも入れ物が無いか（`advance`・`refresh` が作る）」に直す。
- 3 種類: 当たらない。

**S14. 公開の口のうち 4 つは seriko の外に使い手が無い（R14 の漏れ）**（`src/lib.rs:63-64`・`:68`・doc `:23-24`）
- 対象: `BindDirective`・`parse_bind_directive`・`BalloonResolve`・`resolve_balloon_key`。
- 確かめ: ツリー全体を grep して、seriko の外の使い手は 0 件。出てくるのはコメントの中の語だけ（`tests/bind_e2e.rs:24`・`tests/balloon_face_e2e.rs:24`・`areka-emo-compose/src/boxes.rs:358`・`areka-emo-text/src/sink.rs:137`）。ほかの公開の型の形にも出てこない。クレートの中では `actor.rs:692`・`actor_bind.rs:24` が使うので、外しても dead_code にはならない。
- 直し: 4 つを `pub use` から外し、`lib.rs:23-24` の doc の項も外す。
- 3 種類: 当たらない。

**S15. テストの道具の写し（R19 の一覧に無かったもの・直しが生んだもの）**
- 部品の欄を読む `part_frames` が 3 本（`actor_test_support.rs:187`・`parts_tests.rs:33`・`looper_parts_tests.rs:41`、1 字も違わない）と、`actor_clock_tests.rs:141-145` の閉包 `part`。→ `lib_test_support.rs` に 1 本置き、ほかは `use` する。
- `actor_dispatch_tests.rs:152-179` の `entityref_cue`・`text_cue`・`wait_cue` は、同じファイルが取り込む `cue(at, scope, command, duration)`（`actor_test_support.rs:140`）の写し（元の指摘 6 で `balloon_cue` を寄せたのと同じ）。→ `cue(..)` の呼び出しにし、3 本を消す（約 25 行減る）。
- 絵の欄を読む道具が 4 つ（`looper_always_tests.rs:21` `film_cell`・`looper_balloon_tests.rs:46` `film_cell`・`parts_film_tests.rs:110` `picture`・`actor_stage_tests.rs:66` `cell`）。→ `lib_test_support.rs` に `film_picture(&PatternState, FilmId)` を 1 つ置く。
- `ScopeStates::new(DefaultBinds::uniform(BindSet::default()))` を手で書いた所が 26 か所と、同じ中身の道具 2 つ（`looper_calls_tests.rs:54` `no_binds`・`shell_fallback_tests.rs:16-18` `states`）。→ `fresh_states` の隣に `empty_states()` を 1 つ置く。
- parts の道具: 6.1 の直しで足した `parts_always_tests.rs:45-64` の `refresh` と `parts_trigger_tests.rs:114-128` の `Rig::refresh` は中身が同じで、`parts_tests.rs:490` の `peek_at` もほぼ同じ。`parts_always_tests.rs:18-42` の `tick` は `parts_tests::tick` と同じ。`parts_calls_tests.rs:39-45` の `part_cell` は `part_cell_of`（`:120`）と同じ。→ `parts_tests` に `refresh_at(..)` を 1 つ置いて寄せ、`part_cell` は `part_cell_of` を呼ぶ形にする（40〜60 行減る）。
- `test_support::part_playing` を使わず `PartAnim::Playing { started_at_ms, acts_through: None }` を手で書いた所が、`parts_tests.rs`（133・208・327・375・470・538・618）と `looper_parts_tests.rs:70-73`・`:131-134`。`looper_parts_tests.rs:70` の `let playing = ...` は、15 行で取り込んだ `part_playing as playing` を隠している。→ `part_playing(..)` を使う。
- 3 種類: 当たらない。

**S16. 重なったテスト（R16 の漏れ）**（`src/bind_tests.rs:72-96`・`policy_empty_or_absent_is_default`）
- `policy_mustselect_only_declared_category_per_namespace` の 4 つの確かめは、`policy_matrix_sakura_covers_all_declaration_forms` と `policy_is_namespace_isolated_between_sakura_and_kero` が全部見ている。表が違うだけで、壊したときにこのテストだけが赤になる形は無い。→ 消す（`mustselect_resolver` は `category_ids_collects_category_members_ascending` が使うので残す）。
- `policy_empty_or_absent_is_default` の 1 つ目の確かめは、`empty_resolver_is_default_policy_and_never_reaches_apply` の輪の中にある。→ 2 つ目（tiny_resolver）だけ残す。
- 3 種類: 当たらない。

**S17. テストの名前が中身と違う**（`src/table_always_tests.rs:105-107`・`:150-151`）
- 名前 `is_continuous_is_true_only_when_always_or_animated_parts_exist` は「always か動く部品のときだけ真」と言うが、今は `runonce`／`periodic`／`talk` でも真（`table.rs:517`・`table_trigger_tests.rs:227` が見ている）。150-151 行の「本 spec の前と同じ」「今の門」は作業の頃の語。
- 直し: 名前を `random_only_is_not_continuous_always_is` などにし、「本 spec の前」を外す。
- 3 種類: 当たらない。

**S18. R12 と R1 の直しで足したテスト 2 本が、`always` のテストのファイルに置かれている**（`src/parts_always_tests.rs:253-308`）
- 問題: 2 本は `always` を含まない（`random` の再生の抽選と、`-1`/`-2` の後の働きのコマ）。このファイルのモジュールの doc は spec を animated-image-playback と名乗るので、278 行の「要件 4.1」はその spec の要件 4.1（`always` を最初のコマから始める）と読まれ、中身と合わない。
- 直し: 2 本を `parts_tests.rs` へ移すか、要件の番号に spec の名を付ける。
- 3 種類: 当たらない。

**S19. どの文書のものか分からない符牒（直しが新しく入れたものを含む）**
- 直しが入れたもの: 「re-audit R1／R8／R12」が `actor_parts_tests.rs:175`・`looper_calls_tests.rs:411`・`:428`・`:448`・`parts_always_tests.rs:254`・`:278`・`timeline_calls_tests.rs:90`。spec が `completed/` へ移ると、どの文書の R 番号か分からない。→ 外すか「audit-seriko reaudit-1 R12」の形にする。
- 前からあるもの:
  - `actor_tick_tests.rs:99`・`:155`「5.3 の直接 on_tick 檻」と `actor.rs:15`・`:269`「（5.3）」（R25 で同じファイルの「Task 6.1」は消した）。→「looper の層で on_tick をじかに呼ぶ檻」などに替え、`actor.rs` の「（5.3）」は外す。
  - `looper.rs:16`・`:259`・`looper_tests.rs:323`・`:347` の「討議 #2」、`looper_tests.rs:482` の「檻 10」。→ 番号を外すか spec の名を付ける。
  - `bind.rs:385`「design（…行 ~326）」・`:563`「design 行 322」: どの design か書いておらず、指す先（`.kiro/specs/completed/areka-P0-mayuna-compose/design.md`）の行もずれている。→ spec の名を入れ、行の番号を外す。
- 3 種類: 当たらない。

**S20. テストの小さな誤記**
- `actor_dispatch_tests.rs:233`・`:309`: doc は「`capture_text` で捕捉」と言うが、中身は `capture_logs_flow`。→ 名を直す。
- `state_stage_tests.rs:96`「知らせの無い流れ（`emo2`・今のテスト）」: 本番の emo2 は毎フレーム知らせを送る（`crates/areka/src/emo2_boot/frame/status_report.rs:129-154`）。→「知らせを送らない流れ（知らせを使わないテストなど）」に直す。
- `looper_test_support.rs:1-2`: 使い手を looper_trigger_tests・looper_talk_tests の 2 本としか書いていないが、`looper_parts_tests.rs:11` と `looper_always_tests.rs`（`switch_at` を 13 か所）も使う。→「looper のテストが共有する足場」に替える。
- `tests/bind_e2e.rs:254`: `Show{1000,{1207}}}` の `}` が 1 つ多い。→ `Show{1000,{1207}}`。
- 3 種類: 当たらない。

**S21. 統合テストの同期の手順が、2 つのモジュールの doc に写しで残り、片方は無い名を使う**（`tests/balloon_face_e2e.rs:8-17`・`tests/bind_e2e.rs:8-17`）
- 問題: R11 で同期のコードは `common::run_core` へ移り、その doc（`tests/common/mod.rs:112-115`）が手順を書いている。2 つのモジュールの doc には 5 段の写しが残り、すでに食い違う。balloon は「surface_sink へ move 済みの `SerikoSink`」と書くが、`surface_sink` という名は無い（`spawn_talk` の引数は `sinks`）。
- 具体例: `run_core` の同期の決まり（5 秒の待ちや `TalkDone` の約束）を変えても、2 つの doc は古いまま残る。R11 が止めようとしたずれそのもの。
- 直し: 5 段の列を「同期は `common::run_core` のとおり（sleep/polling なし・要件 5.3）」の 1 行にする（約 18 行減る）。
- 3 種類: 当たらない。

**S22. rustdoc の警告 20 件（`cargo doc -p areka-seriko --no-deps`）**
- 公開の doc から私有の項目への link（12 件）: 生成した文書では死んだ link になる。
  - `actor.rs:77`・`:128`・`:239`・`:285` → `handle_message`
  - `bind.rs:360`・`:367` → `apply_bind_directive`（`bind` は私有のモジュールで、再輸出されていない）
  - `looper.rs:60` → `LoopRuntime`
  - `settle.rs:50` → `SETTLE_MAX_TICKS`
  - `state.rs:114`・`:614` → `ScopeStates::commit_bind`
  - `state.rs:320` → `crate::shell_fallback::decide`
  - `state.rs:673` → `crate::looper::LoopRuntime`
  - 直し: 角括弧を外して `` `handle_message` `` のように文字だけにする（`state.rs:114`・`:614`・`:673` は下の `[...]: ...` の定義の行も消す）。
- 解決できない link（8 件）:
  - `actor.rs:291` `` [`BindChoicePolicy::Default`] ``: `actor.rs` から型が見えない。→ `` [`BindChoicePolicy::Default`](crate::BindChoicePolicy::Default) ``。
  - `state.rs:73` `` [`LoopRuntime`] ``: 見えないうえに私有。→ 角括弧を外す。
  - `state.rs:191`・`:259` `` [`SurfaceTarget::Show(id)`] ``: 括弧つきの名は引けない。→ `` [`SurfaceTarget::Show`]`(id)` ``。
  - `state.rs:197`・`:198` `` [`current_binds`] ``: 行の途中の `[current_binds]: ScopeStates::current_binds` は定義として読まれない。→ `` [`current_binds`](Self::current_binds) `` にし、198 行の後ろの定義を消す。
  - `state.rs:248`・`:258` `` [`apply`] ``: → `` [`apply`](Self::apply) ``。
- 3 種類: 当たらない。

**S23. `cue_sequence.rs` と `regression.rs` の重なりが残る（R11 の 3 つ目の項はしていない）**（`tests/cue_sequence.rs` 全体・`tests/regression.rs:126-159`）
- 問題: 同じ `run_cues` と同じ emo2 の素材で、cue_sequence はスコープ 0 の「静観→2106」・Hide・スコープ 1 の数の鍵 "10" を見る。regression の 3 件目は「通常→2100」とスコープ 1 の「静観→2106」、1 件目は Hide を見る。tasks.md 6.1 の組 D に、ここをどうしたかの記録が無い。
- 具体例: 「静観」が 2206 に解けるように壊れると、両方が赤になる。テストの実行ファイルごとに、emo2 の読み込みと検体の展開が 1 回ずつ余分にかかる。
- 直し: `cue_sequence_emits_expected` を `regression.rs` の 4 件目へ移し、`cue_sequence.rs` を消す（見出しと import の約 20 行と、テストの実行ファイル 1 つが減る）。価値は小さく、残しても筋は通る。
- 3 種類: 当たらない。

**S24. 指摘 S2 の関連: `timeline.rs:552` のコメントが R1 の直しで偽**
- 「frame_at は現在コマのみ見るので」は、今の `frame_at`（i 以下で最後の描くコマを見る）と合わない。S2 を直せば期待ごと書き替わる。S2 を直さないなら「i 以下で最後の描くコマ（101）が非負なので」に替える。
- 3 種類: 当たらない。

## 直した所を確かめて、問題が無かったもの

- R1（`frame_at`／`visible_frame`）: `Stopped` の後に `frames[i]` を添字に取る呼び手は無い。`put_top_play`（`looper.rs:258`）・`look`（`parts.rs:141-149`）は `visible_frame` を通して負の番号を取る。合図の先頭の再生（`looper.rs:887`）は `Active|FinishedResidual` だけを置く。最後のコマが描くコマの後の働きのコマのときは `FinishedResidual(last)` を返し、`put_visible`／`write` が前の描くコマを置く。R1 のテスト（`looper_calls_tests.rs:430`・`:450`、`parts_always_tests.rs` の 2 本目）は直しを戻すと赤になる。
- R12（抽選の「再生中か」）: 一番上（`looper.rs:484-492`）・部品（`parts.rs:656-658`）・引き金の `playing_at`（`looper_trigger.rs:103-106`）は同じ見方に揃った。`Residual` の時計は前と同じく抽選の対象、最初の待ちの前（`Look::Nothing`）は再生中のまま。新しいテスト（`actor_parts_tests.rs` の `top_and_child_ending_on_a_boundary_both_restart_on_it`・`parts_always_tests.rs` の `play_that_ended_while_hidden_is_lotteried_on_the_reshow_tick`）は直しを外すと赤になる。残る問題は S1・S4・S8。
- R2（`calls.rs` の窓 `first = lo / period`）: 正しい。前の周のコマはどれも印以下。`laps` の `n-1` は n ≥ 1 なので安全で、`first > last` は空の範囲になる。R23 の doc（`:51`）も事実どおり。
- 落ち着かせの上限（`settle.rs`）: 600,000ms ちょうどで warn 0、+1ms で warn 1 と前のコマ。どちらもテストが在る。16ms は `areka-ghost/src/ticker.rs:262` と合う。
- テストの道具（`lib_test_support.rs`・`actor_test_support.rs`・`looper_test_support.rs`・`state_test_support.rs`・`tests/common/mod.rs`）: どの項目にも使い手がある。`fresh_states` は 1 本に寄った。`run_core` は 5 秒の `recv_timeout`・`TalkDone` の `let…else`・`Ended` の確かめ・talk→seriko の join の順を全部保っている。`run_actor` は前の写しと同じく close→join。`#![allow(dead_code)]` は、どの統合テストも common の一部しか使わないので今も正しい。`emo2_surfaces_txt` は私有になった（R26）。
- R3・R8・R17・R18 で直したテスト: どれも、壊すと赤になる形になった（R3 は `actor_replace_tests.rs:225-227` が動的な組を見ているので穴は無い）。
- R4・R5・R9・R10・R25 の古い語: 担当の場所では消えている。残りは S6・S9・S19 に挙げた。`actor.rs:749` の記録の文言に残る「M-boot」は、裁定どおり触らない。`tasks.md N` の参照のうち、同じ文に spec の名が付いているものはたどれるので挙げない。
- R13: clippy は `-D warnings` で 0 件。`SerikoReplace` の `#[expect(clippy::large_enum_variant)]` も正しく付いている。
- R14: 外した 4 つ（`FrameStatus`・`frame_at`・`should_fire`・`SurfaceKeep`）に外の使い手は無い。`pub fn decide` が `pub(crate)` の型を返すが、モジュールが私有なので届かない。
- Cargo.toml: 依存（dola・areka-sakura・areka-emo-compose・areka-parsers・areka-actor・tracing）と開発用の依存（log-capture-kit・sample-ghost-kit）は、どれも使われている。fixtures の記録の数（debug 5・warn 2+1）は README と surfaces.txt で合っている。

## まとめ

| 組 | 件数 | 番号 |
| --- | --- | --- |
| 必ず直す | 2 | S1・S2 |
| 直したほうがよい | 10 | S3〜S12 |
| できれば | 12 | S13〜S24 |

- 要件 3.1 の 3 種類に当たるもの:
  - S1: **(1) に触れる**（完了 spec `seriko-pattern-calls` 要件 3.1 を戻す方向・R12 の裁定を「当て残しが無いときだけ」へ狭める）。推す裁定は「直す」。
  - S2: **(3) に触れる**（ukadoc の `-1` でアニメーションが終わる、へ寄せる）。推す裁定は「直す」。
  - S3: 周を 1 つずつ入れる形なら当たらない。上限を置く形なら **(1) に触れる**（「時刻は正確に扱う」）。推す裁定は「周を 1 つずつ入れる形で直す」。
  - ほかの指摘はどれも当たらない。
- Verdict: まず S1（R12 の直しが働きのコマを落とす）と S2（`-1` の先へ進む道）を、赤のテストを立ててから直す。その次に S3（`always` の追いつきの 2 乗）と S4（落ちなくなったテスト）。
- Lean: 約 -150 行（写しの道具の寄せ・重なったテストの削除・doc の写しの削除・`cue_sequence.rs` の畳み込み）。依存は減らない。
- 見ていないもの: 無い（87 本を全部読んだ）。ただし S1〜S5・S12 の「壊しても緑のまま」「赤になる」は読みで確かめたもので、摂動は回していない。S3 の時間は測っていない。
