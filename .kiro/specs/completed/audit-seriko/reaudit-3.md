# audit-seriko 6.1 audit のかけ直し（3 回目）

- HEAD: `7055bfe6`・枝 `claude/kiro-start-audit-seriko-8ceffb`
- 道具: `ponytail:ponytail-audit`。範囲は `crates/areka-seriko/` だけ。要件 1.2・1.3・5.1 に従い、指摘の数に上限を置かずに全部を挙げた。「見ていない」ファイルは無い
- 範囲: 86 ファイル・28,388 行（本番 22 ファイル 8,396 行・テストとテストの道具 61 ファイル 19,798 行・テスト用の素材 2 ファイル 161 行・`Cargo.toml` 33 行）
- 読み方: 本番の 22 ファイルは取りまとめ役が 1 行目から最後の行まで読んだ（`calls.rs`・`timeline.rs`・`trigger.rs`・`talk.rs`・`looper.rs`・`looper_trigger.rs`・`looper_calls.rs`・`parts.rs`・`parts_trigger.rs`・`parts_calls.rs`・`actor.rs`・`actor_bind.rs`・`settle.rs`・`table.rs`（中のテストを含む）・`table_calls.rs`・`state.rs`・`bind.rs`・`output.rs`・`resolve.rs`・`skin_facts.rs`・`shell_fallback.rs`・`lib.rs`）。テストの 61 ファイルと素材 2 ファイルは 4 人の読み手で分けて全部の行を読み、読み手の挙げた件は取りまとめ役が該当の行を開いて確かめた
- 動かしたもの（コードは 1 バイトも変えていない）
  - `cargo clippy -p areka-seriko --all-targets --no-deps -- -D warnings`: 警告 0
  - `cargo doc -p areka-seriko --no-deps`: 警告 0
  - `cargo test -p areka-seriko`: 564 件緑（537＋5＋9＋9＋4＋0）
  - 全体テスト・`--ignored`・areka-impl-watch.exe は回していない
- 扱わないもの: research.md §3 の 1〜20（18・20 は「残す」の裁定どおり）、reaudit-1.md の R1〜R26、reaudit-2.md の S1〜S24（どれも直し済み）。tasks.md の Implementation Notes に「前からある狭い穴」として記録された 2 件（下の T2・T3・T4）は、まだ直っていないので指摘として挙げた

## このクレートがすること・想定した負荷

SERIKO（シェルの面のアニメーション）を、ゴーストごとの役者のスレッドで 16ms ごとに進め、表示の指令を出す。台本の合図（`\s`・`\b`・`\![bind]`）を受けて面と着せ替えを替える。想定した負荷は、利用者 1 人・ゴースト数体・役者 1 体あたり毎秒約 60 刻み。

## 指摘

### 必ず直す（4 件）

**T1. 再生中で見送った `runonce` が、後で古い時刻から鳴り直し、働きのコマを 2 度当てる**（`src/trigger.rs:121`・`:150-164`）
- 何が悪いか: `runonce` は、見え始めの時刻にすでに同じ animation が再生中だと、`commit` が印を付けずに見送る。次の刻みからも `next_start` は同じ見え始めの時刻を返し続ける。その再生が終わって表から外れた刻みに、見え始めの時刻（もう過去）で鳴る。新しい再生は「当て終えた印」を持たないので、見え始めから今までの働きのコマを全部当て直す。
- 具体例: バルーンの窓が閉じている間に、抽選の animation の `start,R` が `runonce` の R を時刻 t2 に始める。窓が t3 に開く（見え始め＝t3）。t3 に R は再生中なので見送る。R の再生が t4 に終わると、次の刻みで R が t3 から始まった扱いになり、R の `start`／`stop`／`move` のコマが過去の時刻でもう一度当たる。部品でも同じ。部品が見えない間に進んでいた抽選の時計の呼び出しを見えた刻みで当てると、R が見え始めより前に始まる。そのため見送りになり、同じことが起きる。
- いちばん小さい完全な直し: `commit` で、`runonce` は再生中でも印を付ける（`if anim.trigger == LoopTrigger::Runonce { self.fired.insert(anim.id); }` を `if playing { return false; }` の前へ移す）。見送った回は鳴らさない。これは `periodic` が見送った周を後から鳴らさないのと同じ決まりで、完了した `areka-P0-seriko-trigger-intervals` の要件 2.1（切り替わった刻みに 1 回）・5.3（再生中はやり直さない）に合う。`trigger_tests.rs:112-120` の `runonce_polled_while_playing_keeps_its_one_shot` は今の見送りを固めているので、期待を「見送ったら鳴らない」に直す。上の具体例を一番上と部品で 1 本ずつテストにする。
- 種類 3.1: (1) には当たらない（要件の文ではなく、テストが固めている振る舞い）。(2) にも当たらない。(3) は当たらない。ukadoc の `runonce` は「表示された時に 1 回」で、遅れて鳴るほうが正典から外れている。推奨は「直す」。

**T2. 刻みの中で、抽選と `always` の再生が「今」に始まった後に、それより前の時刻の呼び出しを当てる（一番上と部品）**（`src/looper.rs:470-539`・`:580-582`・`src/parts.rs:620-676`）
- 何が悪いか: 刻みは、抽選（2）と一番上の `always` の誕生を now の時刻で先に済ませる。その後に、前の刻みから now までの働きのコマ（引き金の開始の時刻ごとと now まで）を当てる。部品も同じで、1 段目の `step` が抽選・`always` を先に決めてから `fire_part_triggers` と `run_part_calls` を回す。時刻の順が逆になる。
- 具体例 1: バルーンの窓が刻み now に開き、一番上の `always` A がそこで生まれる。その前から走っていた抽選の再生 L の `stop,A` のコマ（時刻 t < now）を、その後に当てる。A は t にはまだ無かったので何もしないのが正しい。実際は A を止め、「止めた always」の印を付けてしまう。印は面を出し直すまで残るので、A はそれまで動かない。
- 具体例 2: 抽選の animation X を、別の再生の `start,X` が t < now に始めるはずだった刻み。抽選は X を「再生中でない」と見て乱数を 1 つ引き、X を now で始める。その後に `start,X` を当てると t で入れ替わる。正しい順なら X は再生中なので、抽選は乱数を引かない。乱数の引く数が変わる。
- いちばん小さい完全な直し:
  - 一番上: `on_tick` を 3 つの輪に分ける。1 つ目の輪で、面ごとに引き金と `run_top_calls(now)` を回す。2 つ目の輪で、今の抽選を今と同じ順（スコープ → シェル → バルーン → 番号）で回す。3 つ目の輪で、面ごとに `start_top_always` の後にもう一度 `run_top_calls(now)` を回す（now に始まった再生の頭のコマを当てる）。その後に進行と部品を回す。乱数を引く順は変わらない。
  - 部品: `step` の中で、`fire_part_triggers` → `run_part_calls(now)` → 抽選・`always_start` → `run_part_calls(now)` → `place` の順にする。部品 1 つの中の並べ替えなので、乱数を引く順は変わらない。
  - 上の 2 つの具体例をテストにする。
- 種類 3.1: (1) には当たらない（固定の消費順 D-7 は保つ）。(2) にも当たらない（コードに `shortcut:` の注は無く、tasks.md にだけ書いてある）。(3) は当たらない。同じ刻みの中の順は ukadoc に書かれておらず、SSP は刻みの格子を持たない。直すと、時刻どおりの順という今の設計の決まり（時刻は正確に扱う）に揃う。推奨は「直す」。

**T3. 合図の経路（`refresh`）は `periodic`・`talk` を判定しないまま、合図の時刻までの呼び出しを当てる**（`src/looper.rs:849-884`）
- 何が悪いか: `refresh` は `runonce` だけを `fire_top_triggers` に渡し、その後で `run_top_calls(at)` を合図の時刻 at まで回す。前の刻みと at の間に `periodic` の境目 T（T < at）があると、T の判定は次の刻みに回る。そのときには T より後の呼び出しをもう当て終えている。
- 具体例: `periodic,1` の P の境目が 2000。2010 に `\s` と同じ面の着せ替えの合図が来る。合図の前から走っている再生 Q の `stop,P` のコマは 2005 にある。正しい順なら、P は 2000 に始まり、2005 に止まる。実際は、2010 の `refresh` が `stop,P` を当てる（P は再生していないので何もしない）。次の刻みが P を 2000 で始め、P は止まらずに最後まで流れる。
- いちばん小さい完全な直し: `refresh` でも `fire_top_triggers` に 3 語を全部渡す。文字の窓は `on_tick` と同じ形で組み、`refresh` の終わりに、その面の数えを `advance_talk` で at の時点まで進める（同じ区切りを次の刻みで 2 度鳴らさないため）。部品の `refresh` も同じにする。`judge_part` の `runonce_only` を外し、在る状態にも `fire_part_triggers` を回す。上の具体例をテストにする。
- 種類 3.1: T2 と同じ。(1)・(2)・(3) のどれにも当たらない。推奨は「直す」。直しが大きいと見るなら、`shortcut:` の注をコードに書いて開発者の裁定で (2) として残す道もある。ただし今の負荷（合図は台本ごとに何度も来る）で起きうるので、推奨は直すこと。

**T4. 新しく見えた部品の `talk` が、見え始めより前の区切りの時刻で、呼び出しの後に判定される**（`src/trigger.rs:128-140`・`src/parts.rs:576-613`）
- 何が悪いか: 部品が刻み now で新しく見えると、1 段目がその部品の呼び出しを now まで当てる。その後で、2 段目の `judge_part` が now で構えて `talk` を判定する。`next_start` の `talk` の腕は見え始めの時刻を見ないので、開始の時刻は区切りの文字が現れた時刻 T（前の刻みと now の間・T < now）になる。部品は T にはまだ見えておらず、呼び出しはもう now まで当たっている。
- 具体例: 1 段目で当てた `start` のコマが、その talk の animation を T' に始める（T < T' ≤ now）。2 段目は T で「再生中か」を測り、T' の再生を見て見送る。T には再生していなかったので、正しくは T で鳴る。
- いちばん小さい完全な直し: `next_start` の `talk` の腕で、開始の時刻を見え始めの時刻より前にしない（`.then(|| (window.wall_ms)(glyph).max(since))`）。新しく見えた部品では開始の時刻が now になるので、「同じ時刻 T の中は今ある再生のコマが先」の決まりで、判定は呼び出しの後で正しくなる。一番上の面は、構えるときにその時刻までの文字を数え済みにしているので、答えは変わらない（1 ms の丸めのずれだけ）。上の具体例をテストにする。
- 種類 3.1: (1)・(2)・(3) のどれにも当たらない（ずれは刻み 1 回より小さく、ukadoc に書かれていない所）。推奨は「直す」。

### 直したほうがよい（6 件）

**T5. 着せ替えから外れた再生を捨てたことを、どのテストも確かめていない**（`src/looper_tests.rs:720-728`）
- 何が悪いか: 最後の確かめ（2000 の刻みで乱数が 2 回目に引かれる）は「再生も捨てた」証として書かれている。しかし 2000 では経過が 1000 で、`frame_at` は末尾（`FinishedResidual`）になる。R12 の直しの後は、再生が残っていても抽選は「再生中でない」と見て引く。
- 具体例: `looper.rs:679-681` の `pb.remove(&anim_id)` を消しても全部緑のまま。本番では、着せ替えが外れている間、刻みごとに「bind から外れた ID の再生を停止」の `info!` が出続ける。
- 直し: 1100 の刻みの後に `assert!(!rt.playback.contains_key(&(scope.clone(), Slot::Shell)))` を足し、720-721 のコメントを直す。

**T6. 部品の `periodic` を止めた後に境目で始め直すテストが、始めた時刻を確かめていない**（`src/parts_calls_tests.rs:295-309`・`:289`）
- 何が悪いか: 確かめるのは 2100 の欄が 120 であることだけ。
- 具体例: 1970 の `stop,0` が当たらなくても、2000 でなく刻みの 2100 で始め直しても、欄は 120 で緑のまま。`:289` の「0 は 3000 に始まる」も同じで、3100 に始まっても 120 になる。
- 直し: `assert_eq!(rt.parts.clock(&scope(), 100, 0), crate::test_support::part_playing(2000))` を足す。`:289` には `part_playing(3000)` を足す。

**T7. 「呼び出しがちょうど開始の時刻に始めた再生は入れ替えない」の決まりにテストが無い**（`src/looper_trigger.rs:125`・`src/parts_trigger.rs:112`・`src/looper_calls_tests.rs:531-556`）
- 何が悪いか: `p.started_at_ms != started_at_ms &&` を消しても `call_starting_the_trigger_animation_at_its_start_time_fires_its_head_once` は緑のまま。消すと引き金は自分の再生を「再生中」と見て `commit` せず、鳴らした記録が抜ける。`runonce` なら印が付かない（T1 と重なる）。
- 直し: そのテストを `capture_logs` の中で回し、「trigger periodic を鳴らした」の行が 2 本（1030 と 2030）あることを確かめる。部品の道（`parts_trigger.rs:112`）にも同じ形の確かめを 1 本足す。

**T8. テストの時計が途中で逆に進む**（`src/actor_clock_tests.rs:135-139` `bind_cue_starts_part_always_at_the_clock_time_of_processing`）
- 何が悪いか: `rig.hear(1030, Emote)` の後に `rig.tick(0)` を送る。時計が 1030 → 0 → 1030 と動き、`ClockRig` の約束（`actor_test_support.rs:44-45`「観測を追い越さない」）を破る。今は面 0 で合図に始まるものが無いので緑。
- 直し: 135 行の時刻を 0 にするか、139 行の `rig.tick(0)` と順を入れ替える。

**T9. コメントが、確かめていない道を別のテストが確かめていると書く**（`src/actor_bind_loop_tests.rs:283`）
- 何が悪いか: 「未知 scope は `bind_scope_unmapped_warns_no_emit`」とある。そのテストが送るのは数でないスコープ `"x"` で、通るのは `NoNamespace`（`warn!`・状態は変えない）の道。数のスコープで状態がまだ無いとき（`state.rs` の `None => StateOnly`）の道ではない。
- 直し: 括弧の中を `state_bind_pattern_tests.rs` の「未知 scope も StateOnly」のテストの名に替えるか、括弧を消す。

**T10. 確かめていないことを言う、テストの文言と doc**
- `tests/loop_integration.rs:277-281`: 「-1 到達までは再生中で再抽選しない」とあるが、1000 から 1120 の間に 2 つ目の境目が無い。再生中に抽選し直す誤りがあっても `calls == 1` で緑。直し: 「発火判定は最初の境界で 1 回」に縮める（その性質は `playing_anim_not_relotteried_across_boundary_full_path` が確かめている）。
- `src/table_film_tests.rs:209-210` `from_world_without_atlas_has_no_film_rows`: doc は「`from_world` が `film_sheets()`／`film_skips()` を通す・記録も無い」と言う。しかし記録を捨てており、世界にアトラスが無いので、子の定義を無視する `from_world` でも緑。直し: 記録を確かめるか、doc を「アトラスの無い世界では子の行が無い」に縮める。
- `src/actor_dispatch_tests.rs:812-815` `non_shell_and_wait_are_no_op_then_shell_cue_still_emits`: 「焼き込み絶対時刻どおり・タイミングも乱さない（二重待ちなし）」と言うが、確かめるのは記録の列だけ。seriko は届いた順に出して `cue.at` を見ないので、時刻はどこでも測っていない。直し: doc を「Shell でない合図と Wait を読み飛ばし、続く Emote は出る」に縮める。

### できれば（5 件）

**T11. どの文書のものか分からない番号と作業の頃の語（S19 の漏れ・直しが新しく入れたもの）**
- `src/calls.rs:61` 「（入れる数は当てる数と同じ・S3）」: 354ddf82 が入れた。本番のコードに再監査の番号が入っている。
- `src/state_stage_tests.rs:120` 「指摘 11」: spec の名が無い。
- `src/timeline.rs:5` 「[deterministic-test-coverage-mandate]」: 開発者の覚え書きのファイルの名で、コードの読み手には意味が無い。
- `src/looper_balloon_tests.rs:153` 「3.4 の道」・`src/actor_dispatch_tests.rs:32` 「emo-text 6.1」・`:157` 「3.2 の複合シナリオ」・`:761-762` 「5.2 が Wait cue を発行し始めた今」: どの spec の番号か書いていない。
- 「今と同じ」「今までどおり」「今までの読み飛ばし」: `src/actor_strict_tests.rs:55`・`:79`・`src/actor_clock_tests.rs:198`・`src/actor_trigger_fixture_tests.rs:51`・`src/actor_talk_tests.rs:221`・`src/actor_test_support.rs:30-32`。
- 直し: 番号を消すか、spec の名を添える（`calls.rs` は「・S3」を消す・`state_stage_tests.rs` は「audit-seriko 指摘 11」）。`timeline.rs:5` は括弧ごと消す。「今」の比べは消す。

**T12. 中身と違う doc とテストの文言**
- `src/table.rs:64-67`（`LoopTrigger` の doc）と `:24-25`（モジュールの doc）が `runonce`・`periodic`・`talk` を「後ろの 3 つ」「後ろ 3 種」と呼ぶ。`CallOnly` を足した後は後ろの 3 つが `Periodic`・`Talk`・`CallOnly` で、`Always`・`CallOnly` も抽選しない。直し: 「後ろの 3 つ」を外し、3 語の名だけで書く。
- `src/bind_tests.rs:334` の文言「明示 "0"/"1" のみ・一般 parse でない」: `scope_namespace` は今は数を一般に読み、5 行前で `"12"` が `Char(12)` になることを確かめている。直し: 「非数値 scope は写像なし（D7・要件 14.2）」にする。
- `src/actor_dispatch_tests.rs:143-146` の節の見出しが「`capture_text` 直下で」と言うが、節のテストは `capture_logs_flow` を使う。直し: 名を替える。
- `src/looper_tests.rs:86` の `counting_rng` の doc が「残量」を確かめられると言うが、`RngProbe.values` を読む口もテストも無い。直し: 「・残量」を消す。

**T13. 重なったテスト**
- `src/actor_tick_tests.rs:12-88` `bind_noncanonical_addressee_severity_split` の前半（宛名 `bind` で `WARN` が 1 行）は、`actor_bind_noevent_tests.rs:257` `noncanonical_bind_noevent_warns_like_bind` が `bind`・`bind-noevent` の両方で確かめている。直し: 前半を消し、後半（他人宛は `debug!` だけ）を `actor_bind_noevent_tests.rs` へ移し、刻みのファイルの見出しを刻みのテストだけの説明にする。
- `src/actor_dispatch_tests.rs`: `emit_forwards_cue_to_inbox`（:7-28）と `cue_sink_emit_forwards_cue_to_inbox`（:40-66）は同じ `CueSink::emit` → `deliver` を通る。`emit_after_receiver_gone_logs_no_panic`（:68-91）と `emit_after_actor_stopped_logs_no_panic`（:327-363）も同じ送り損ねを同じ確かめで見る。`deliver` を壊すと 2 本とも同時に赤になり、片方だけが赤になることは無い。直し: 1 本ずつに畳む（Text の合図は 1 本目の中の 2 つ目の `emit` にする）。ついでに、`:102` の局所の `use areka_emo_compose::BindSet;` は `:152` のモジュールの `use` と重なる。

**T14. テストの道具の写し（S15 の漏れ）**
- `count(lines, needle)` が 3 か所に同じ形である: `src/looper_test_support.rs:58-61`・`src/table_calls_check_tests.rs:33`・`src/actor_trigger_fixture_tests.rs:42-44`。
- `src/looper_talk_tests.rs:85-103` の `cue`・`text`・`clear_all` は `src/actor_test_support.rs:140-158` と同じ。
- `src/looper_talk_tests.rs:617-624` の `part_cells` は `test_support::part_frames(…, 100)` と同じ。`src/parts_calls_tests.rs:114-119` の `part_cell_of` も `part_frames` の上に組める。
- `src/actor_tick_tests.rs:108-115` の `fire_once` は `counting_rng(&[0]).0` と同じ。`:128-130` は `fresh_states()` を手で書いている。
- `src/actor_test_support.rs:14-21` `emote_cue`・`:123-131` `noncanonical_custom_cue` が、同じファイルの `cue(at, scope, command, duration)`（:140）を使わずに `TalkCue` を組む。
- `src/table_calls_check_tests.rs:44`・`:92`・`src/table_calls_dot_sakura_tests.rs:48` が、`test_support::warns` を使わずに `contains("WARN")` で手で絞る。
- `animated-playback/shell/surfaces.txt` の `include_str!` の定数を、`src/table_film_tests.rs:18` と `src/table_always_tests.rs:13` の両方が持つ。
- 直し: `count` と `cue`／`text`／`clear_all` と素材の定数は `lib_test_support.rs` へ 1 つずつ寄せ、ほかの写しは在る道具に替える。

**T15. `empty_states()` に寄せ損ねた、空の着せ替えの手書き（S15 の漏れ）**
- `src/looper_tests.rs:212`・`:432`・`:468`・`:498`・`:548` と `src/looper_replace_tests.rs:17`・`:54` に `ScopeStates::new(DefaultBinds::uniform(BindSet::from_ids([])))` が残る。`src/looper_parts_emo2_tests.rs:249` は `build_static_bindset(&[])`。`src/looper_parts_tests.rs:17-19` の `no_binds()` は :76 で 1 回使うだけ。
- 直し: `empty_states()` に替える（`rebase_shell` に渡す値はそのままでよい）。

## 確かめて、問題が無かったもの

- round 2 の直しで組み直した所: `calls.rs` の待ち行列（`next_due`・`push_next`・`has_due_acts`）は、周を割り算で出し、`after` と印より後の最初のコマだけを入れる。同じ時刻の中の 2 度目の開始の止め（`already` と `started`）は、`run_calls` を開始の時刻ごとに何度呼んでも効く。`timeline.rs` の `stop_frame`／`frame_at` と `calls.rs` の `-1` の扱いは同じ決まり（描くコマの負の番号で止まる・`always` は止まらない）。`fire_top_triggers`・`fire_part_triggers` は、開始の時刻の順に T まで当ててから T で測る形が一番上と部品で揃っている。`judge_part` が何も始めなかったときに捨てる結果は、1 段目が now まで当て終えた後なので空。
- S1〜S5・S24 の直しのテストは、直しを戻すと赤になる（読み手が読みで確かめた。摂動は回していない）。
- `trigger_tests.rs:41-50` の `poll`（消した `Armed::poll` の代わり）は、本番の `next_start` → `commit` の組と同じ。
- `tests/fixtures/trigger-intervals/README.md` は `surfaces.txt` と emo2 の実物（絵・面・bindgroup の番号・時刻・記録の数）に合う。
- 決定論: 実の時計・眠り・スレッドの順に頼るテストは無い。
- 公開の口: `lib.rs` の再輸出は、どれも外に使い手が在るか、外に使い手の在る公開の型の署名に出る。`build_static_bindset` の外の使い手はこのクレートの `tests/` だけだが、research.md の指摘 15 の決め（クレートの統合テストも使い手に数える）に従い挙げない。
- 1,000 行の上限: どのファイルも越えていない（最大は `parts.rs` の 983 行）。

## まとめ

| 組 | 件数 | 番号 |
| --- | --- | --- |
| 必ず直す | 4 | T1〜T4 |
| 直したほうがよい | 6 | T5〜T10 |
| できれば | 5 | T11〜T15 |

- 新しい指摘は 15 件で、0 件ではない。T2〜T4 は tasks.md に「前からある狭い穴」と記録されていた 2 件（合図の経路・抽選と `always` の順／新しく見えた部品の `talk`）を、直す形で挙げたもの。T1 は round 2 の直し（印を付けずに見送る `commit`）と呼び出しが重なって表に出た、前からある穴。
- 残す裁定の候補: どれも 3.1 の (1)〜(3) に当たらない。T3 だけは、`shortcut:` の注を書いて開発者が裁定すれば (2) として残せる。ただし推奨は直すこと。
- 見ていないもの: 無い（86 ファイルを全部読んだ）。T1〜T7 の「壊しても緑」「起きる」は読みで確かめたもので、摂動は回していない。
