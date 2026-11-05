# audit-seriko 6.1 audit のかけ直し（1 回目）

- HEAD: `b4f39286`（origin/main を取り込んだ後）・枝 `claude/kiro-start-audit-seriko-8ceffb`
- 道具: `ponytail:ponytail-audit`。範囲は `crates/areka-seriko/` だけ。要件 1.2・1.3・5.1 に従い、指摘の数に上限を置かずに全部を挙げた。「見ていない」ファイルは無い
- 読んだファイル: 87 本、計 28,184 行（`src/` 78・`tests/` の Rust 6・fixtures 2・`Cargo.toml`）。下の一覧を全部 1 行ずつ読んだ。8 人の読み手で分け、本番のファイルは根拠を確かめるために重ねて読んだ
- 動かしたもの: `cargo clippy -p areka-seriko --all-targets --no-deps`（clippy 0.1.99）・`cargo test -p areka-seriko --lib settle`（13 件緑）。全体テスト・`--ignored`・areka-impl-watch.exe は回していない。コードは 1 バイトも変えていない
- 扱わないもの: 元の指摘 18（`emit_display` の包み・種類 (1)・残す）と 20（`trigger.rs` の `ponytail:` 注・種類 (2)・残す）は裁定どおりなので挙げない。1〜17・19 は直し済みとし、直しが足りない所と直しが生んだ所だけを挙げた

## このクレートがすること・想定した負荷

SERIKO（シェルの面のアニメーション）を、ゴーストごとの役者のスレッドで 16ms ごとに進め、表示の指令を出す。台本の合図（`\s`・`\b`・`\![bind]`）を受けて面と着せ替えを替える。`--offline-dump` のために面を進め切る道も持つ。想定した負荷は、利用者 1 人・ゴースト数体・役者 1 体あたり毎秒約 60 刻み。

## 読んだファイル（`crates/areka-seriko/` から・行数）

`Cargo.toml` 33
`src/`: actor.rs 831・actor_bind.rs 109・actor_bind_loop_tests.rs 906・actor_bind_noevent_tests.rs 293・actor_calls_tests.rs 133・actor_clock_tests.rs 252・actor_dispatch_tests.rs 871・actor_parts_tests.rs 176・actor_replace_tests.rs 437・actor_stage_tests.rs 263・actor_strict_tests.rs 222・actor_talk_tests.rs 274・actor_test_support.rs 188・actor_tick_tests.rs 380・actor_trigger_fixture_tests.rs 283・bind.rs 605・bind_forms_tests.rs 282・bind_named_tests.rs 111・bind_tests.rs 815・calls.rs 195・calls_tests.rs 328・lib.rs 76・lib_test_support.rs 100・looper.rs 976・looper_always_tests.rs 455・looper_balloon_tests.rs 301・looper_calls.rs 208・looper_calls_tests.rs 418・looper_film_count_tests.rs 90・looper_parts_emo2_tests.rs 352・looper_parts_tests.rs 309・looper_replace_tests.rs 100・looper_talk_tests.rs 892・looper_test_support.rs 57・looper_tests.rs 970・looper_trigger.rs 203・looper_trigger_tests.rs 693・output.rs 300・parts.rs 964・parts_always_tests.rs 256・parts_calls.rs 183・parts_calls_tests.rs 230・parts_film_tests.rs 363・parts_tests.rs 902・parts_trigger.rs 106・parts_trigger_tests.rs 852・resolve.rs 298・sample_test_support.rs 41・sample_test_support_tests.rs 18・settle.rs 119・settle_tests.rs 223・shell_fallback.rs 61・shell_fallback_tests.rs 141・skin_facts.rs 114・skin_facts_tests.rs 69・state.rs 774・state_bind_pattern_tests.rs 807・state_rebase_tests.rs 84・state_stage_tests.rs 139・state_surface_tests.rs 464・state_test_support.rs 13・table.rs 980・table_always_tests.rs 173・table_calls.rs 233・table_calls_check_tests.rs 122・table_calls_dot_sakura_tests.rs 131・table_calls_tests.rs 157・table_film_tests.rs 222・table_interval_words_tests.rs 123・table_parts_tests.rs 152・table_trigger_tests.rs 164・talk.rs 123・talk_tests.rs 197・timeline.rs 591・timeline_calls_tests.rs 87・timeline_repeat_tests.rs 338・trigger.rs 251・trigger_tests.rs 486
`tests/`: balloon_face_e2e.rs 228・bind_e2e.rs 527・common/mod.rs 104・cue_sequence.rs 87・loop_integration.rs 645・regression.rs 194・fixtures/trigger-intervals/README.md 65・fixtures/trigger-intervals/surfaces.txt 96

1,000 行を越えるファイルは無い（近いのは table.rs 980・looper.rs 976・looper_tests.rs 970・parts.rs 964）。

## 指摘

件数: 必ず直す 1・直したほうがよい 11・できれば 14（計 26）。

### 必ず直す（1 件）

**R1. `-1` のコマのすぐ後に待ち 0 の働きのコマがあると、再生が止まらず「面 4294967295」が欄に入る**（`src/timeline.rs:105-117` `frame_at`・`src/looper_calls.rs:182-187` `put_visible`・`src/parts.rs:502-514` `write`・`src/looper.rs:154-156` `pattern_frame`）
- 何か: 経過の時間から今のコマを決め、描くコマを欄に置く道。一番上も部品も同じ道を通る。
- 問題:
  - `frame_at` が `Stopped` を返すのは、「今のコマ」が負の番号の描くコマのときだけ。
  - 待ち 0 のコマは前のコマと同じ締め切りを分け合い、最後のものが今のコマになる。働きのコマ（`start`・`stop`・`insert`・読めない相手）の待ちは `table_calls.rs:74-77` で 0 になる。`move` も待ちを 0 と書ける。
  - だから `-1` の後に待ち 0 の働きのコマが続くと、今のコマは働きのコマになり、答えは `Active` か `FinishedResidual` になる。
  - `put_visible`／`write` は「i 以下で最後の描くコマ」を選ぶので、`-1` のコマを選ぶ。`pattern_frame` の `f.surface_id as u32` で `-1` が 4294967295 になり、そのまま欄に入る。
- 具体例: `animation0.interval,random,2`・`pattern0,overlay,100,0,0,0`・`pattern1,overlay,-1,50,0,0`・`pattern2,insert,0,0,0,0`（`pattern2,start,[3]` でも同じ）。
  - 期待: 経過 50 で止まり、ベースへ戻る（要件 4.3）。
  - 実際: 欄に面 4294967295 が載る。停止の記録は出ず、`FinishedResidual` の腕に入って「末尾残留」の記録が出る。
  - 当たらないもの: 働きのコマそのもの（`push_due` が `-1` で止まる）。`always`（`always_visible` が負の番号を見る）。
- 一番小さい直し:
  - `frame_at` で、今のコマの代わりに `visible_frame(frames, i)` が指すコマを見て、それが負の描くコマなら `Stopped` を返す。直すのは 1 か所で、一番上と部品の両方が直る。
  - 赤のテストを 1 本立てる（上の 3 コマ・経過 50 → `Stopped`）。
  - `frame_at` の doc の「現在コマの `surface_id < 0` なら」を「i 以下で最後の描くコマが負なら」に揃える。
- 確かめ方: 読みで確かめただけで、赤のテストはまだ立てていない（読むだけの段のため）。直すタスクは赤を立ててから始める。
- 3 種類: **(3) に触れる**。
  - ukadoc（https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/ の pattern の項）では、サーフェス ID の `-1` でアニメーションが終わる。SSP も `-1` で再生を終える。
  - 今の振る舞いはこれから外れていて、直すと正典に寄る。
  - 推す裁定: **直す**。

### 直したほうがよい（11 件）

**R2. `calls.rs` の `-1` の余裕は届かない枝で、上のコメントは事実と違う**（`src/calls.rs:175-176`・持ち越し b）
- 何か: `always` の働きのコマを、前の印（`through`）より後から窓に入れる道。`let first = (lo / period).saturating_sub(1);` と、その上のコメント「累積の待ちは周の長さ以下なので、1 つ前の周の終わりのコマも窓に入りうる」。
- 問題:
  - 周の長さは、最後のコマの累積の待ち以上になる。手書きの `always` は待ちの合計（`table.rs:343`）で、動く絵の子はさらに長い。
  - だから 1 つ前の周 L-1 のコマの時刻は、大きくても `start + L*period ≤ start + lo`、つまり印以下になる。`due` は `at > through` を求めるので、必ず外れる。
  - 印が無いときは `lo = 0` で、どちらの式でも 0 になる。
  - `started_at_ms` を後から動かす道は無く、どの `start` も印を `None` に戻す。だから `through >= start` がいつも成り立つ。
  - 結局 `-1` は答えに出ず、周 1 つ分の無駄な走査が増えるだけ。コメントは逆のことを言っている。
- 具体例: `.saturating_sub(1)` を外しても、`always_lap_end_frame_is_hit_exactly_once_across_ticks`（`calls_tests.rs:316-328`）は `[Some(100), None, Some(200), None, Some(300)]` のままで緑になる（5.2 の記録と同じ）。
- 直し:
  - `let first = lo / period;` にする。
  - コメントを「累積の待ちは周の長さ以下なので、印より後のコマは印の周から始まる」に替える。
  - `calls_tests.rs:313-314` の説明も「周の境目の回帰のテスト」に直す。テストそのものは、`due` が `>=` になったときに赤になるので残す。
- 3 種類: 当たらない（振る舞いは変わらない）。

**R3. `actor_tick_tests.rs` に移したテストが、着せ替えの組を見ていない**（`src/actor_tick_tests.rs:148-150`・`182`・`214-219`・`242`・`302`・`352-356`・`367-371`・持ち越し c）
- 何か: タスク 3 で移した、刻みの道の 4 本。
- 問題:
  - `tick_boundary_cross_emits_show_carrying_pattern` は `{1100,1207}` を載せて組むのに、確かめは `Show { scope, surface_id, pattern, .. }` で、`binds` を `..` で読み飛ばす。
  - `emote_surface_change_resets_loop_playback` も、2 か所とも `..` で読み飛ばす。
  - `tick_with_no_shown_slot_is_complete_no_op` と `send_tick_after_actor_stopped_logs_debug_no_panic` は、組がどうでも答えが変わらない所（何も出ない・記録だけ）で組を使っている。
  - 名前や doc は「着せ替えを見る」とは言っていない。それでも、役者の層で「刻みが出す `Show` は今の着せ替えを載せる」を見るテストがこのファイルに無い。looper の層でこれを見ているのは `looper_parts_emo2_tests.rs:82` だけ。
- 具体例: 182 行の `[1100, 1207]` を `[1100]` に替えても、刻みの腕が `binds` を空で出すように壊れても、4 本とも緑のまま。
- 直し: `tick_boundary_cross_emits_show_carrying_pattern` の腕で `binds` を受け、`assert_eq!(binds, &BindSet::from_ids([1100, 1207]), "刻みの Show は今の着せ替えを載せる");` を 1 行足す。ほかの 3 本は今のままでよい。
- 3 種類: 当たらない。

**R4. `actor.rs`・`resolve.rs` に作業の頃の説明が残り、一部は事実と違う**（持ち越し a）
- 何か: 元の指摘 12 で直した残り。どれもコメントだけで、コードは変わらない。
- 事実と違うもの:
  - **`src/resolve.rs:95-96`・`:118`・`:267`（テスト）**: 「M-boot は数値のみ・名前解決は将来 additive」。
    - 事実: 名前の形の `\b` は文字の層（emo-text）が箱の名前で読むと決まっていて、seriko が名前を引くことは無い。
    - 同じファイルの `:84-85`（`NameForm` の doc・指摘 12 で直した所）と `actor.rs:692-694` は正しく書いており、ファイルの中で食い違っている。
    - 直し: 「名前の形は文字の層が読む。seriko は数だけを読む」に揃える。`:92` の「（M-boot: alias／名前解決なし）」からも「M-boot:」を外す。
  - **`src/actor.rs:648`**: 「上記以外の非キャリア（M-boot compile は非生成）」。
    - 事実: この腕に届かない理由は構造にある。`dola` の `cue_target_of` が `None` を返すのは `Wait` と `Custom` だけで、どちらも手前の腕で捌いている。
    - 直し: 「dola に variant が増えたときの防御」に直す（腕は残す）。
  - **`src/actor.rs:155-160`（`close` の doc）**: 「kanade による停止駆動（単一 Close funnel）の受け口」「終了同期テスト（本タスク・後続 4.1）」。
    - 事実: 本番の止め方は、送り手を全部 drop する道だけ（`crates/areka/src/ghost_session.rs:663`・`emo2_boot/mod.rs:627`）。ツリー全体を grep すると、seriko の外で `close()`／`SerikoMsg::Close` を使う所は 0 件で、呼ぶのは seriko のテストだけ。
    - 直し: 「明示の停止の口（今はテストが使う）。本番は送り手を全部 drop して止める」に直す。
    - `:29` のモジュールの doc の「停止は 2 経路」は正しいので残す。
- 古いだけのもの（偽ではない）:
  - `actor.rs:14-15`「後続の時間駆動ループ（seriko-loop）が同じ発行点を再利用できる」（ループはもう在り、刻みの腕が使っている）。
  - `:268-269`「cue 適用駆動（本タスク）でも後続の時間駆動ループでも」。
  - `:741`「M-boot では非到来」。
  - 直し: 「後続」「本タスク」「M-boot」の語を外し、今の事実の形にする。
  - 次のものは経緯として正しいので、直すなら番号と符牒だけ外す（できれば）: `:22`・`:232` の「task 4.1」、`:24-25`・`:238` の「task 8.1 で撤去」、`:285-286`「additive 追加（D4）」、`:351`「発見 C」、`:626`「申し送り是正」。
- 記録の文言（直さない）: `actor.rs:745` の `"seriko: EntityRef は M-boot で未対応; 防御的に読み飛ばす（R6.2）"`。直すと記録の文言が変わるので触らない。2 つのファイルで M-boot などを含む記録の文字列は、この 1 本だけ。
- 3 種類: 当たらない。

**R5. `state.rs`・`bind.rs`・`parts.rs`・`bind_tests.rs` にも同じ種類の古い説明が残り、2 か所は逆のことを言う**（元の指摘 12 の残り）
- 事実と違うもの:
  - **`src/bind.rs:346-350`・`src/bind_tests.rs:555`・`:577`**:
    - 書いてあること: `bind.rs` は「本タスクは類別のみ … 実際の適用はアクター（task 6.1）の責務」。`bind_tests.rs:555` は「Toggle は M1 縮退・warn+skip は actor 責務」、`:577` は「CategoryWide はカテゴリ単位では厳密に action されない M1 縮退ゆえ寛容」。
    - 事実: 今は `apply_bind_directive`（`bind.rs:538-539`）が、Toggle もカテゴリ単位の形も自分で当てる（`bind_forms_tests.rs` が見ている）。
    - 害: 読む人は、`\![bind,腕,伸び]` が警告されて読み飛ばされると信じてしまう。
    - 直し: 「今の着脱の反対を当てる」「定義の順に 1 つずつ当てる」に直す。`bind.rs:383`・`:397` の「M1 類別」も同じに直す。
  - **`src/state.rs:13`**「実際の発行は後続タスク（アクター層 `emit_display`）の責務」と **`:85`**「運ぶ口（`SerikoMsg::Stage`）と世代の扱いは task 5.1」: どちらももう在る（`actor.rs:83`）。
- 古いだけのもの:
  - `state.rs:74`・`:155`・`:175` の「後続タスク」、`:168` の「task 1.2/2.4 で解決される」、`:80`・`:255`・`:651` の「M-boot」。
  - `parts.rs:877` の「（task 5.1）」。
  - `bind.rs:244` の「既存テスト（task 6.4）」、`:250` の「並走 spec の無干渉前提として固定」。
  - `bind_tests.rs:43`・`:113`・`:527`・`:687` の節の見出し「Task 3.1／5.1／3.2／3.3」（どの spec の番号か分からない）。
- 直し: 「後続タスク」「task N」「M-boot」「M1」「本タスク」を外すか、今の持ち主の名に替える（コメントだけ）。
- 3 種類: 当たらない。

**R6. `pattern_frame` の doc が「部品は今のコマを渡す（5.3 で描くコマへ揃える）」のまま**（`src/looper.rs:151-153`）
- 問題: 部品の `write`（`parts.rs:503`）は、もう `visible_frame` で選んだ描くコマを渡している。doc を読んだ人は、部品だけ働きのコマが欄へ届くと思ってしまう。
- 直し: 「一番上も部品も描くコマ（`visible_frame` の答え）を渡す」に直す。同じファイルの `:495` の「・task 3.4」も外す。
- 3 種類: 当たらない。

**R7. 指摘 12 で短くした `looper_parts_emo2_tests.rs` の理由の節が、今は事実と違う**（`src/looper_parts_emo2_tests.rs:17-21`）
- 問題:
  - 06f23fa4 で手書きの一覧を消したとき、理由 1「seriko クレートの手組みの表はどれも、pattern定義が指すサーフェスに自分の animation を持たせていない」を現在形で残した。
  - 元は「HEAD eb1a7e77 で測った」という前提つきの文だった。前提が外れて今の事実の主張になり、それが偽になっている。
- 具体例:
  - `looper_parts_tests.rs:197` の `part_draws_follow_all_top_draws_in_fixed_order` では、surface100 の `pattern0,overlay,50` が指す 50 が `random,5` を持ち、部品として抽選される（乱数の列に 5 が出る）。
  - 同じファイルの `CHILD`（`:50`）も同じ。
- 直し: 17〜21 行の節を消す。端（要件 7.5）は、同じファイルの `emo2_part_lottery_after_stalled_ticks_is_one_extra_draw_and_at_most_80ms` がじかに檻に入れている。
- 3 種類: 当たらない。

**R8. `settled_surface_carries_no_shift` が空と空を比べている**（`src/looper_calls_tests.rs:403-418`）
- 問題: 表の最後のコマが `-1` なので、ずれのある面もずれの無い面も、書き出しの答えは空の `PatternState` になる（417 行がまさにそれを確かめている）。
- 具体例: 回帰で `move` の x,y（3,5 など）が animation 0 の欄のコマの位置に漏れても、最後の `-1` が欄ごと消すので緑のまま。
- 直し:
  - settle に渡す本文から、最後の `animation0.pattern6,overlay,-1,100,0,0` を外す。最後に描くコマが欄に残る形にする。
  - そのうえで、`moved.get(0).map(|f| (f.surface_id, f.x, f.y))` がずれの無い値（例 `Some((101, 0, 0))`）であることを確かめる。値は直すときに測って決める。
- 3 種類: 当たらない。

**R9. `actor_dispatch_tests.rs` の説明が「非 Shell は warn!」のまま**（`src/actor_dispatch_tests.rs:141-147`）
- 問題: 今の非 Shell の受け取りは `debug!`（`actor.rs:610`）。同じファイルの `:738-782` は WARN が 0 件であることを確かめている。説明だけが逆のことを言っている。
- 直し: 「Unresolved の error!／EntityRef の warn!／非 Shell の debug!」に直す。
- 3 種類: 当たらない。

**R10. `output.rs` のモジュールの doc が、本番の発行先を古いまま書いている**（`src/output.rs:5-6`・`:96-98`）
- 問題: doc は「emo-present の表示指令 API が未完成でも」「本番の発行先は `ghost-setup` の領分」と言う。実際の本番の発行先は `crates/areka/src/emo2_boot/adapter.rs` の `PresentBridge` で、emo-present も在る。
- 直し: 本番の発行先は `PresentBridge` だと書く。`:18`・`:40`・`:205` の「M-boot」も外す。
- 3 種類: 当たらない。

**R11. 統合テストの組みの写しが 2 組残っている**（元の指摘 6 の残り）
- 写しその 1: `tests/balloon_face_e2e.rs:47-110` の `run_scenario` と `tests/bind_e2e.rs:118-187` の `run_core`。
  - 同じ手順: 役者と talk を起こす → 刻みを入れる → `TalkDone` を待つ（同じ `let…else` と assert）→ talk と役者を join する → 記録を写す。
  - 違うのは入力（別名の表・静的な着せ替え・名前の表・文字の受け手・`TalkId`）だけ。前者は `run_core(BindSet::from_ids([]), BindResolver::empty(), …, Box::new(NullTextSink))` そのもの。
- 写しその 2: 「起こす → 送る → close → join → 記録を写す」が 5 回ある。
  - 場所: `tests/cue_sequence.rs:271-283`・`:310`、`tests/regression.rs:46-60`・`:107-118`・`:162-172`、`tests/loop_integration.rs:77-101`。
  - emo2 の既定の着せ替え `[1100, 1207, 1302, 1500, 1800]` も 2 回書いている（`cue_sequence.rs:264`・`regression.rs:22`）。
  - `loop_integration::emote`（`:63-70`）は `common::emote_cue` と同じなのに、このファイルは `mod common` を使っていない。
- 具体例: 同期の決まり（待ちの上限や `TalkDone` の約束）を変えるとき、2〜5 か所を同じに直す必要がある。どこかがずれても誰も気づかない。
- 直し:
  - `run_core` を `tests/common/mod.rs` へ移し、`balloon_face_e2e::run_scenario` を 1 行の呼び出しにする（約 55 行減る）。
  - `common::run_cues(resolver, binds, cues) -> Vec<DisplayCommand>` と `emo2_binds()` を置き、5 か所を寄せる。
  - `cue_sequence.rs` は、同じ素材・同じ組みで「静観→2106」を `regression.rs:170` と重ねて見ている。`regression.rs` へ畳むのも手。
- 3 種類: 当たらない。

**R12. 見えない部品の抽選の時計: まとめて追いつくことは無いが、実は終わっていた再生が抽選を 1 回止める**（`src/parts.rs:654`・持ち越し d）
- 知らせの中身: audit-emo-compose から「見えない間の部品の抽選の時計が古いまま残り、見え直したときに経過した分を一度に進める」と知らせがあった。
- 調べた結果（知らせの形はバグではない）:
  - 見えない部品は `step` が呼ばれないので、抽選も進行もせず、乱数も引かない。
  - `drop_hidden`・`drop_unseen_transient` が捨てるのは、回数つきの `always` と 3 語の時計だけ。抽選の時計（`Playing{started_at}`／`Residual`）は残る。
  - 見え直した刻みの進行は、`frame_at(now − started_at)` で今のコマを 1 回で求める。
  - `LotteryBoundary::poll` は、何度境目を越えても 1 刻みに 1 回だけ真を返す。
  - だから、何度も鳴ることも、まとめて抽選することも、初めからやり直すことも起きない。
  - 見えない間も時間が進んでいた扱いは、完了 spec `areka-P0-surface-element-nesting` の要件 5.7（2026-10-04 開発者の裁定）と「時刻は正確に扱う」の原則どおり。
- 残る差（これが指摘）: 見え直した刻みでは、抽選の前に `playing`（時計がまだ `Playing` か）を見て、進行（`place`）はその後にする。だから実際の時間ではとうに終わっていた再生でも、その刻みの抽選を受けない。
- 具体例（100 は bind+random,2 でコマ 40 → 100ms 後に 41。40 は random,2 でコマ 42 → 1000ms 後に -1）:
  - 1000: 100 と 40 が当たる。
  - 1100: 100 のコマが 41 になり、40 が見えなくなる。時計は `Playing{1000}` のまま残る。
  - 2000: 実際の時間では、ここで 40 の再生が終わる。
  - 3000: 100 が当たり、40 が見え直す。40 は「再生中」と見なされて抽選されない。進行で `-1` を過ぎたと分かって時計が消え、「part 停止」の記録もこの時点で出る（本当は 2000）。
  - 4000: 40 の次の抽選。1 秒遅れている。
- 起きる頻度:
  - HEAD では、面の切り替えから戻った最初の刻みが境目をまたいだときだけ起きる（約 1.6%）。
  - audit-emo-compose（枝の `a7bad45c`）が入った後は、着せ替えのコマで見え直す部品は必ず境目の刻みに戻るので、毎回起きる。
- 直し:
  - `parts.rs:654` の `playing` を「時計が `Playing` で、かつ `look(anim, clock, now_ms)` が `Nothing` か `Frame`」にする。終わっていた古い時計は抽選に戻り、当たれば `Playing{now}` に置き換わる。
  - 乱数を引く回数が変わるので、回数を数えているテストの期待値を直す。
  - 一番上（`looper.rs:478`）も「抽選が進行より前」の同じ順だが、毎刻み進むので遅れは 1 刻みだけ。直さなくてよい。
- 3 種類:
  - **(1) に触れる**: `areka-P0-surface-element-nesting` 要件 5.7 の裁定「見えない間も進んでいた扱い」。直しはこの裁定の延長（終わっていたものを終わっていたと扱う）で、裁定を変えない。
  - **(3) には当たらない**: 子の時計の扱いは ukadoc に定めが無く、areka の決めとして登記済み（同 spec 要件 5.18）。ukadoc の rarely などの「そのサーフェスである間」の抽選の決まりからも外れない。
  - 推す裁定: **直す**。ただし audit-emo-compose が着いた後に、その枝のテストと合わせて直すほうが手戻りが少ない。
- 前提の訂正:
  - 知らせにあったテスト名 `part_hidden_by_the_parent_frame_change_is_not_left_in_the_field_and_keeps_its_random_clock` は、取り込んだ origin/main にもこの枝にも無い。`a7bad45c` は main に着いていない（`git merge-base --is-ancestor a7bad45c origin/main` が偽）。
  - HEAD の名は `part_hidden_by_a_later_frame_in_the_same_tick_is_not_left_in_the_field`（`parts_tests.rs:385-392`）のまま。6.1 の名の一覧の読み替えはまだ要らない。
  - あちらが着いた後の取り込みでは、`parts_tests.rs` がぶつかりうる（この枝も 836db5e2 で触っている）。
- 関連（範囲の外・持ち主 audit-emo-compose）:
  - `parts.rs:10-12` のモジュールの doc は「見えない部品には触らない（抽選・進行・欄への書き込みのどれもしない・要件 5.11）」と言う。
  - これは、HEAD の `parts_tests.rs:385-392` が固めている「親の着せ替えのコマが pattern0 を置き換えている間も、その先が毎刻み評価され抽選される」と合わない。
  - 原因は emo-compose の `visible_parts` で、あちらの枝で直っている。ここでは直さない。

### できれば（14 件）

**R13. clippy 0.1.99 の警告 4 件**（持ち越し f）
- `cargo clippy -p areka-seriko --all-targets --no-deps` の結果は、着手の時点と同じ 4 件（増えていない）。lib に 2 件、lib test に 4 件（うち 2 件は重なり）。
  - `src/actor.rs:91`: large_enum_variant（`SerikoReplace::Shell` が 424 バイト以上）。
  - `src/looper.rs:417`: collapsible_if（着手の時点の 433 行から行がずれただけ）。
  - `src/looper_talk_tests.rs:597`・`src/looper_trigger_tests.rs:504`: collapsible_if。
- 直し:
  - collapsible_if の 3 件は `if a && let …` の 1 つにまとめる（機械的）。
  - `SerikoReplace` は、シェルの差し替えのときだけ 1 回送る依頼で、大きくても困らない。`#[expect(clippy::large_enum_variant, reason = "差し替えのときだけ 1 回送る")]` を付ける。`Box` にする案もあるが、呼び手と照合の形が変わるので推さない。
- 3 種類: 当たらない。toolchain が固定されていないので、直さないと次の clippy でも出続ける。

**R14. 公開の口のうち 4 つは、seriko の外に使い手が無い**（`src/lib.rs:70`・`:76`・`:16`）
- 対象: `pub use timeline::{FrameStatus, frame_at, should_fire}` と `pub use shell_fallback::SurfaceKeep`。
- 確かめ:
  - ツリー全体を grep して、seriko の外の使い手は 0 件。`should_fire` は `crates/areka/src/emo2_boot/spine_seriko_loop_tests.rs:370` のコメントの中に語として出るだけ。
  - `SurfaceKeep` を返す `decide` は再輸出されていない。
  - `LoopRng`・`seeded_rng` は本体が使うので残す。
  - この 4 つは、元の指摘 15 の 7 つに入っていなかった。
- 直し:
  - 4 つを `pub use` から外す（`SurfaceKeep` は `pub(crate)` へ）。
  - `lib.rs:16` の doc の「1000ms 絶対グリッド跨ぎ検出」も外す。指摘 15 で `LotteryBoundary` を `pub(crate)` にしたため。
- 3 種類: 当たらない。

**R15. 自分と比べるだけの確かめ（元の指摘 13 の漏れ）**
- `src/timeline_repeat_tests.rs:196-205` `assert_jump_equals_ticks`（呼び手は `:221-223`・`:236-239`）:
  - 輪の中で `ticked` を毎回上書きするので、最後の値は `always_at(f, p, n, start + 1000)` そのもの。それを同じ呼び出しと比べている。`always_at` は状態を持たないので落ちない。
  - 直し: 道具と 2 つの `for start in …` を消す（約 15 行）。「1 秒飛んだら 1 秒進む」は、固定値の確かめ `:211-220`・`:231-235` が見ている。
- `src/timeline.rs:586-590` `frame_status_is_send`:
  - `usize` を持つ `Copy` の列挙は必ず `Send` なので、落ちない。
  - 直し: 消す。
- `src/settle_tests.rs:105` `tick_interval_is_the_production_16ms`:
  - 定数と数の字面を比べるだけ。本体の刻み（`crates/areka-ghost` の `ticker.rs:262` の `Duration::from_millis(16)`）が 10ms に変わっても緑のまま。
  - 直し: 消す。定数を共有してつなぐ案もあるが、範囲の外に及ぶので推さない。
- `src/output.rs:203-249` `balloon_display_commands_construct_and_match`:
  - 指摘 13 の直しで、中の `x == x.clone()` だけを消した。残りも、derive した `PartialEq` が違う値を違うと言うことを見ているだけで、seriko の本体をどう壊しても赤にならない。
  - 直し: 消す。
- 3 種類: 当たらない。

**R16. 重なったテスト（元の指摘 14 の漏れ）**
- `src/bind_tests.rs:209-236` `policy_both_options_declared_prefers_multiple`: 4 つの確かめは全部、同じ表で `policy_matrix_sakura_covers_all_declaration_forms`／`policy_matrix_kero_covers_all_declaration_forms` が見ている。→ 消す。
- `src/timeline.rs:575-583` `frame_at_sparse_derived_dense_vec`: `frame_at_kero_real_sequence_stops_on_negative_tail`（`:462`）と同じコマで、確かめはその一部だけ。名にある「疎」も試していない（それは `table.rs:634-690` が見ている）。→ 消す。
- `src/output.rs:157-194` と `:254-299`（送った順に記録するモックのテスト 2 本）: `MockSurfaceOutput::send` は種類を見ずに積むだけなので、送る種類が違うだけの同じテストになっている。`mock_starts_empty`（`:198`）も、上のテストが件数を見ている。→ 種類を混ぜた 1 本にまとめる。
- `tests/bind_e2e.rs:416-429`・`:513-526`: すぐ上で `Vec` 全体の等しさを確かめた後に、部品だけを見直している。→ 消す。
- `tests/balloon_face_e2e.rs:176-181`: シナリオ 1 が `:118` で同じ字面に固めたものと比べ直している。→ 消す。
- どれも単独では落ちない確かめ。3 種類: 当たらない。

**R17. 空の列でも通る `.all()`**（`src/actor_calls_tests.rs:80-86`）
- `later.iter().all(|c| matches!(c, Show{..}))` は、`rig.tick(200)` が何も返さなくても真になる。説明は「2 枚目の絵（120）だけ」と言っている。
- 直し: `assert_eq!(frames(&later, 0), vec![("0", Some(120))]);` にする。
- 3 種類: 当たらない。

**R18. 「状態だけ直す」を確かめていない**（`src/actor_bind_loop_tests.rs:612-641` `bind_apply_on_hidden_scope_state_only_no_emit`）
- 見出しの表（`:304-305`）は「非表示 scope は状態のみ更新」と言うが、確かめは「何も出さない」だけ。
- 隠れたスコープで当てるのを飛ばすように壊れても、緑のまま（状態の層は `state_bind_pattern_tests` が見ている）。
- 直し: `assert_eq!(states.current_binds(&scope), &BindSet::from_ids([1100, 1207, 1302]));` を足す。
- 3 種類: 当たらない。

**R19. テストの道具の写し（元の指摘 6 の一覧に無かったもの）**
- 着せ替えの合図の組みが 6 か所:
  - `src/actor_bind_loop_tests.rs:21-29` の `bind_carrier_cue`（同じファイルの `:32-40` の `named_carrier_cue(scope, "bind", tokens)` と同じ中身）
  - `actor_replace_tests.rs:37-47`・`actor_bind_noevent_tests.rs:45-55`・`actor_parts_tests.rs:65-70`・`actor_clock_tests.rs:148-152`
  - 引数が空の `Custom` の合図も 2 か所（`actor_tick_tests.rs:17-33`・`actor_bind_noevent_tests.rs:279-287`）。
  - 直し: `actor_test_support` に `carrier_cue(scope, name, tokens)` を 1 つ置く。
- `src/actor_dispatch_tests.rs:103-106`: `tiny_resolver()` の写し。しかも数の鍵 "2100" を送るので、別名を引かない。→ `tiny_resolver()` を使う。
- `src/actor_test_support.rs:31-35` の `fresh_states` と `src/state_test_support.rs:11-13` の `empty_states`: 中身が同じ。→ `lib_test_support.rs` へ 1 つにまとめる。
- looper のテストの写し:
  - `src/looper_balloon_tests.rs:227` の `top_started`（＝`test_support::started(rt, Slot::Balloon, 0)`）
  - `src/looper_always_tests.rs:36` の `top_cell`（＝`test_support::cell(states, Slot::Shell, 0)`）
  - 同じファイルの `:20` の `switch`（`test_support::switch` の `at_ms` が `None` なだけ）
  - `src/looper_talk_tests.rs:273-283` と `:800-810` の `note` の closure（1 字も違わない）
  - 直し: 既にある口へ寄せる。
- parts のテストの写し:
  - `src/parts_tests.rs:613-641`・`:686-715` に、12 引数の `PartClocks::advance` の呼び出しの写しが 4 つある。`tick`（`:49-75`）が `scope()` を固定しているため。
  - `src/parts_always_tests.rs:140-184`・`parts_tests.rs:885-895` に、`refresh` の写しがある。
  - 直し: `tick` に `scope` を受けさせる（`parts_film_tests.rs:72-107` の `tick_at` の形）。約 45 行減る。
- 3 種類: 当たらない。

**R20. 使われない柔らかさ**（`src/actor_clock_tests.rs:36-41` `manual_clock`）
- doc は「テストが値を書き替えられる時計」と言うが、ただ 1 つの呼び手（`:246`）は手綱を捨てている（`_now`）。
- 直し: `Arc::new(|| 1030)` にして、関数と `AtomicU64`／`Ordering` の import を消す。
- 3 種類: 当たらない。

**R21. `loop_integration.rs` の素通しの包み**（`tests/loop_integration.rs:577-585`）
- `mod warn_capture { pub fn buffer() }` は `install_global_capture_all()` を呼ぶだけ。
- 直し: `:594` でじかに呼び、モジュールとその doc（`:566-585`）を消す。
- 3 種類: 当たらない。元の指摘 18 の `emit_display` とは別物で、発行点の決めごとには関わらない。

**R22. 要らなくなった並べ替え**（`src/state_bind_pattern_tests.rs:793-797`）
- 指摘 11 で `shown_slots()` の順は決まった（`state_stage_tests.rs:206` が見ている）。それなのに、ここは `format!("{:?}")` で並べ替えたままで、続く確かめも `len` と `contains` だけ。
- 直し: 並べ替えと `mut` を消す。
- 3 種類: 当たらない。

**R23. `calls.rs` の前提の doc が実際と違う**（`src/calls.rs:51`）
- doc は「どの再生も `now_ms` 以前に始まっている」を前提にしている。けれども `looper.rs:632` に書いてあるとおり、合図の時刻で始めた再生は、刻みより僅かに後に始まりうる。
- 害は無い。印が `now_ms` になり、次の刻みで始まりのコマが当たる。
- 直し: doc に「`now_ms` より後に始まった再生は次の刻みを待つ」と書く。
- 3 種類: 当たらない。

**R24. `table.rs` の doc とテスト名が中身と違う**（`src/table.rs:129`・`:633`）
- `LoopAnimation::trigger` の doc は「`Random`/`BindRandom`/`Always`」の 3 種のまま。今は 7 種ある（`:63`）。
- テスト名 `only_random_and_bindrandom_are_recorded_others_debug_logged` に対し、本文は「純 Bind は CallOnly として載る」を確かめている。
- 直し: doc を直し、テスト名を中身に合わせる。
- 3 種類: 当たらない。

**R25. 小さな誤ったコメント（テスト）と、古い語**
- 誤ったコメント:
  - `src/table_interval_words_tests.rs:99`「（今日は 0 件＝まばたきが再生されない…）」: 確かめは「ちょうど 1 件」。「今日は 0 件」は spec の前の話。
  - `src/state_surface_tests.rs:243`・`:250`「ShowBalloon は scope と surface_id のみ」: 確かめる値は `pattern` も持つ。→「binds を持たない」に直す。
  - `src/shell_fallback_tests.rs:80`: 汎用の `fn ids` に「新しいシェルの面の表（0・5）」の doc が付いている。→「番号の列を集合にする」に直す。
  - `src/looper_always_tests.rs:276`「（進行は task 3.6・ここでは入れ物を直に満たす）」と `src/looper_balloon_tests.rs:5`「（運ぶ口は task 5.1）」: どちらも今は在る（`looper.rs:530`・`crates/areka/src/emo2_boot/frame/status_report.rs:186`）。→ 括弧を外す。
- テストの説明に残る作業の頃の語:
  - `src/actor_dispatch_tests.rs:90`・`:155`・`:408-409`・`:484`・`:548`・`:665-666`・`:728`・`:735-736`・`:790`: 「本タスク」「M-boot 未対応」「RED では…落ちる」。
  - `actor_bind_loop_tests.rs:9`・`actor_tick_tests.rs:111`: 「Task 6.1/6.3」。
  - `tests/balloon_face_e2e.rs:5`・`tests/bind_e2e.rs:6`: 「本体コード（src/）は一切変更せず…新規 API 追加なし」。
  - `balloon_face_e2e.rs:49-50`: 「M-boot にバルーン着せ替えは無く」。事実は正しいので、語だけ外す。
  - `tests/cue_sequence.rs:231`「task 3.2 の re-export」と `:263`「build.rs / emo-compose 実測相当」。その値を計算する `build.rs` は無い。`tests/regression.rs:20` にも同じ文がある。
  - `tests/regression.rs:8`・`:96-97`「task 4.1 の同期 handler テスト」: 今の在りかは `actor_dispatch_tests.rs`。
  - `tests/loop_integration.rs:1`「（task 10.1）」、`:34`・`:591`「5.3 の 10.1 繰り延べ」、`:572`・`:574-576`（移す前の経緯）。
  - `tests/loop_integration.rs:14`「既存 cue_sequence.rs/bind_e2e.rs と同一技法」: **偽**。`bind_e2e` は、talk のスレッドが終わって送り手が切れるのを待つ別の技法を使う。
  - `Cargo.toml:13-16`: 「task 4.1」「task 6.1」「task 8.1」。
- 直し: 番号と経緯を外し、`loop_integration.rs:14` は事実に合わせて直す。
- 3 種類: 当たらない。

**R26. `tests/common/mod.rs` の `emo2_surfaces_txt` は外から呼ばれない**
- 使うのは、同じモジュールの `emo2_resolver` だけ。→ 私有にするか、`emo2_resolver` の中へ入れる。
- ほかの項目は、どれもどこかの統合テストが使っていて、死んだコードは無い。`#![allow(dead_code)]` は「この 1 本のテストでは使わない」を黙らせるためのもので、正しい。
- 3 種類: 当たらない。

## 持ち越しの項目（tasks.md `## Implementation Notes`）への答え

| 項目 | 答え |
| --- | --- |
| a. `actor.rs`・`resolve.rs` の古い説明 | 指摘 **R4**（偽のもの 3 組・古いだけのもの・直さない記録の文言 1 本）。同じ種類が他のファイルにも残っている（**R5**・R6・R9・R10・R25） |
| b. `calls.rs` の `-1` とコメント | 指摘 **R2**（`-1` は答えに出ない・コメントは偽） |
| c. `actor_tick_tests.rs` が着せ替えを見ない | 指摘 **R3** |
| d. 見えない部品の抽選の時計 | 知らせの形（まとめて追いつく）は**問題ではない**。何度も鳴らず、まとめて抽選もしない（裁定どおり）。残る「1 回分の抽選の遅れ」を指摘 **R12** に挙げた |
| e. 引き金の記録の target の変更 | **問題ではない**。`target/`・`.git/` を除くツリー全体（全種類のファイル・`RUST_LOG` の文字列・`*.md`・`*.ps1`・`*.py`・`*.toml`）を探して、`looper::top_trigger`・`parts::part_trigger` で絞る所は 0 件。テストの `capture_logs` は文言だけを見る。fixtures の README の `RUST_LOG` は `areka_seriko=debug` で、新しい target も入る。`tools/perf` が読む `areka_seriko::looper` の行（抽選発火・停止）は `looper.rs:515`・`:303` から出たままで、影響しない。完了 spec や `*.log` に残る古い target は過去の記録で、何も絞っていない |
| f. clippy | 指摘 **R13**（4 件。着手の時点から増えていない） |
| g. `log-capture-kit` の `event_tests.rs:15` | 範囲の外（持ち主 `audit-test-kits`）。在りかは `crates/log-capture-kit/src/event_tests.rs:15`（`tests/` ではなく `src/`）。`Capture::on_event` はツリーのどこにも無いので、括弧書きは古い。同じファイルの `:18` にある `choice_drain.rs` についての同じ名も古い。ここでは直さず、完了の棚卸で持ち主へ渡す |

## まとめ

| 組 | 件数 | 番号 |
| --- | --- | --- |
| 必ず直す | 1 | R1 |
| 直したほうがよい | 11 | R2〜R12 |
| できれば | 14 | R13〜R26 |

- 要件 3.1 の 3 種類に当たるもの:
  - R1: (3)。推す裁定は「直す」。
  - R12: (1) に触れる。推す裁定は「直す」（ただし audit-emo-compose が着いた後）。
  - ほかの指摘はどれも当たらない。
- Verdict: まず R1（`-1` の後に待ち 0 の働きのコマがあると、面 4294967295 が欄に入る）を、赤のテストを立ててから直す。その次に古い説明（R4・R5）と弱いテスト（R3・R8）。
- Lean: 約 -200 行（重なり・自分比べのテストの削除、統合テストの組みの寄せ、写しの道具の寄せ）。依存は減らない。
- 見ていないもの: 無い（87 本を全部読んだ）。ただし R1・R2・R12 の「壊しても緑のまま」「赤になる」は読みで確かめたもので、摂動は回していない。
