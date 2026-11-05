# ギャップ分析: audit-seriko（ponytail-audit の結果）

> 2026-11-05・main `967044ac` の上のワークツリー `claude/kiro-start-audit-seriko-8ceffb` で取った。
> この spec ではギャップ分析が audit そのもの（brief の Approach・要件 1）。物差しは ponytail 5.3.2 の
> `/ponytail:ponytail-audit`。作法のうち「最大 20 件」と「読まなかった所（Not checked）」は外した
> （要件 1.2・1.3）。コードは 1 行も変えていない。

## 1. 範囲の測り直し（要件 1.5）

範囲は `crates/areka-seriko/` の全部（84 ファイル）。

| 種類 | ファイル数 | 行数 | 中身 |
| --- | --- | --- | --- |
| 本番のコード | 22 | 8,356 | `src/` の `*_tests.rs`・`*_test.rs`・`*_test_support.rs` 以外（`lib.rs` を含む）。うち 4 ファイルがファイルの中にテストを持つ（`timeline.rs` 229〜608 行・`table.rs` 568〜929 行・`resolve.rs` 122〜315 行・`output.rs` 145〜303 行＝計 1,095 行）。それを除いた本番だけの行は約 7,261 行 |
| テスト | 59 | 20,008 | `src/` の兄弟テスト 54 ファイル（18,176 行・`*_test_support.rs` と `sample_test_support*.rs` を含む）＋ `tests/` の 5 ファイル（1,832 行） |
| テスト用の素材 | 2 | 161 | `tests/fixtures/trigger-intervals/README.md`（65）・`surfaces.txt`（96） |
| クレートの設定 | 1 | 33 | `Cargo.toml` |

- 起票のとき（main `e3c2cbe4`）は「60 ファイル・コード 7,172 行・テスト 15,894 行」。数え方が違う（起票は `lib.rs` やテスト用の素材の扱いが違い、その後の `seriko-pattern-calls` などでファイルが増えた）ので、上の表を今の正本にする。
- 読み方: 本番の 22 ファイルは全行を自分で読んだ。テストの 59 ファイルと素材 2 ファイルは、重ならない 3 組に分けて読み手 3 人に全行を読ませ、その指摘は 1 件ずつ該当の行を開いて確かめ、確かめられなかったものは捨てた（下の「指摘にしなかったもの」）。読まなかったファイルは 0。

## 2. このクレートがすること・想定した負荷

**このクレートがすること:** シェルとバルーンの面の SERIKO アニメーション（伺かの「表情の小さな動き」）を受け持つ。台本の合図（`\s`・`\b`・`\![bind]`）を受けて面と着せ替えの状態を決め、刻み（16ms ごと）ごとに抽選・`always`・`runonce`／`periodic`／`talk`・呼び出し（`start` など）・部品（element の子）の時計を進めて、表示の層へ指令を出す。独立したスレッド 1 本の上の役者で、入れ物（inbox）は 1 つ。

**想定した負荷:** 利用者 1 人のデスクトップで、ゴースト 1 体・スコープ数個・面ごとの animation が数十本まで。刻みは毎秒 60 回前後。ネットワークも複数のプロセスからの書き込みも無い。この負荷では、毎刻みの小さな確保（`stage_slots()` の `Vec`・`ActorKey` の写し）は問題にならないので指摘にしていない。

## 3. 指摘の一覧（audit の番号のまま・要件 1.4）

組の名は audit の 3 組に当てて「必ず直す・直したほうがよい・できれば」。どの指摘にも、残してよい 3 種類（要件 3）に当たりうるかと、直しが範囲の外に及ぶかを書いた。C17 の兄弟の範囲は `audit-parsers-sakura`（`crates/areka-parsers/`・`crates/areka-sakura/`）・`audit-emo-present`・`audit-wintf-graphics-world`（wintf `src/ecs`）・`audit-ghost-catalog`（`areka-ghost`・`areka-nar`・`areka-user-data`）・`audit-wintf-runtime`（wintf の `src/ecs` の外）・`audit-emo-compose`。

### 必ず直す（0 件）

バグ・危ない所・記録の無いデータの失い方・この負荷で壊れる所は **0 件**。本番の 22 ファイルを全部読んで、間違った答えを返す道や、`panic` で役者が止まる通常の入力の道は見つからなかった（壊れた入力はどれも `error!`／`warn!` を残して読み飛ばしている）。

### 直したほうがよい（12 件）

1. **非単調の刻みを無視する番人のテストが、番人を消しても落ちない**（`src/looper_tests.rs:591-609`・番人は `src/looper.rs:433-441`）
   - **何か:** 刻みの時刻が前より戻ったら、何もせずに空を返す決まり（要件 1.2 の防御）。
   - **問題:** テストは「1000 の刻み → 500 の刻み」で、表には抽選だけがある。番人を消しても、500 は抽選の境目（2000）の前で、再生中の animation も無いので、どのみち空が返り乱数も引かれない。だから番人が壊れても緑のまま。`last_seen` も確かめていない。
   - **直し方:** 再生中の animation を作ってから戻る刻みを送る（例: 1000 で発火 → 1150 → 1050）。指令が出ないこと、`last_seen` が 1150 のままであることを確かめる。テストの差し替えだけ。
   - **直さないと:** 番人を誤って消しても誰も気づかず、時計が前へ戻ったときにコマが巻き戻って見える。
   - 3 種類: 当たらない。範囲の外: 及ばない。

2. **テストのファイル 1 つが 1,000 行の上限を越え、例外表に頼っている**（`src/actor_bind_loop_tests.rs`＝1,375 行・例外表は `crates/log-capture-kit/tests/file_length_guard_test.rs:75-78`・件数は同じファイルの 101 行）
   - **何か:** 1 ファイル 1,000 行の見張り（要件 6.1）。このファイルは見張りを作る前から越えていたので例外表に載っている。
   - **問題:** 1012〜1375 行の `mod tick_loop_tests` が別の話（刻みの道）で、しかも 1039〜1129 行の道具は `src/looper_tests.rs` の道具の写し（`pat`＝13 行・`surface_with`＝26 行・`shell_table_of`＝36＋50 行・`table_single`＝55 行・`live_cfg`＝118 行の `cfg`・`always_fire`＝102 行・`shown_shell_states`≒110 行）。`table_single`・`cfg`・`always_fire` はもう `pub(crate)` で、`actor_replace_tests.rs` は使っている。
   - **直し方:** `tick_loop_tests` を兄弟のファイル（例 `actor_tick_tests.rs`）へ移し、写しの道具は `looper_tests` のものを使う。両方とも 1,000 行の下に入るので、例外表の行を消して件数を 9 → 8 にする（例外表は「今も越えている」ことを確かめるので、行を残すと赤になる）。
   - **直さないと:** 例外表が 1 件ずつ減っていく流れが止まり、このファイルに足すたびに大きくなる。
   - 3 種類: 当たらない。範囲の外: **及ぶ**＝`crates/log-capture-kit/tests/file_length_guard_test.rs`（持ち主は `audit-test-kits`。C17 の外で今は走っていない）。要件 7.4 のとおり、その行と件数だけを触る。

3. **本番の `BindResolver::new` と `BindOptionDecls` はテストしか使わず、本番の作り方と違う答えの表を作る**（`src/bind.rs:112-129`・`199-227`・`lib.rs:59`）
   - **何か:** 着せ替えの名前の表を組む口。本番は `BindResolver::from_groups`（`bind.rs:231-269`）だけを使う。
   - **問題:** `new` はツリー全体で本番の呼び手が 0（seriko のテスト 17 か所と `crates/areka/src/emo2_boot/shell_carry_tests.rs:19・38-43` だけ）。しかも `new` で組んだ表は行の順（`written`）と 3 人目以降（`chars`）を持たないので、カテゴリ単位の形 `\![bind,カテゴリ,,1]` は本番なら当たるのに、`new` の表ではいつも `NotFound` になり、スコープ 2 以降は解けない（`bind.rs:281` の doc が自分で認めている）。テストで `new` を使うと本番と違う振る舞いを試すことになる。
   - **直し方（選ぶのは設計）:** ⒜ `new` と `BindOptionDecls` を消し、テストは `from_groups` で組む。ただし `BindGroupName` は `#[non_exhaustive]` で parsers の外から組めない（`crates/areka-parsers/src/package/model.rs:99-101`）ので、テストは descript の文字列を読む口で組む（その口が文字列だけで呼べるかは Research Needed）。⒝ `new` を残すなら、テスト専用であることを `#[doc(hidden)]` などで示すだけでは直らないので、本番と同じ答えになるよう `written` と `chars` も受けるようにする。⒜ が消すほう。
   - **直さないと:** テストが本番と違う表で緑になり、カテゴリ単位の形や 3 人目以降の誤りを見逃す。
   - 3 種類: 当たらない（`BindResolver::empty` の「署名は不変」＝`bind.rs:300`・bindopt 設計 D3 は触らない）。範囲の外: **及ぶ**＝`crates/areka/src/emo2_boot/shell_carry_tests.rs`（持ち主は本体の `emo2_boot` を受け持つ見直し＝C17 の外・今は走っていない）。

4. **一番上と部品の引き金の配線が同じことを 2 か所で書いている**（`src/looper_trigger.rs:53-73` と `src/parts_trigger.rs:58-78`・`looper_trigger.rs:124-147` と `parts_trigger.rs:106-132`・`src/looper.rs:219-224` と `src/parts.rs:183-188`）
   - **何か:** 引き金の状態を構える／窓の開け閉めを写す `match`、鳴らしたときの記録の `match`、「3 語の引き金か」の判定。
   - **問題:** 3 組とも中身が同じで、片方だけ直すと一番上と部品で振る舞いがずれる（spec `areka-P0-seriko-trigger-intervals` 要件 5.4 は「一番上でも部品でも同じ型・同じ関数」を求めている）。
   - **直し方:** `trigger.rs` の `Armed` に「在れば窓を写し、無ければ構える」口を 1 つ、鳴らした記録の関数を 1 つ置き、判定は 1 つ（例 `LoopTrigger` の関数）にまとめる。行は減る。
   - **直さないと:** 次に引き金の決まりを変えたとき、片方だけ変わる。
   - 3 種類: 当たらない。範囲の外: 及ばない。

5. **働きのコマの貸し口が 2 つで同じ決まりを写し、その決まりにテストが無い**（`src/looper_calls.rs:105-110`・`132-150` と `src/parts_calls.rs:44-51`・`74-93`）
   - **何か:** 呼び出し（`start`・`stop` など）を当てた印を進める決まり「前より前の時刻では印を戻さない」と、止めた相手が `always` なら印を付ける決まり。
   - **問題:** 2 つの貸し口（`TopPlays`・`PartPlays`）が同じ式を持つ。しかも「印を戻さない」（`max`）を通るテストが無い（`looper_calls_tests.rs:379-398` は前へ進む順だけ）。片方の `max` を消しても緑のまま、同じ働きのコマを 2 度当てる。
   - **直し方:** 2 つの式を `calls.rs` の小さな関数 1 つずつにまとめ、合図の時刻が前の刻みより前の場合のテストを 1 本足す。
   - **直さないと:** 合図が刻みより僅かに前の時刻で届いたとき、`move` や `start` が 2 度当たりうる変更を誰も止めない。
   - 3 種類: 当たらない。範囲の外: 及ばない。

6. **テストの道具の写しが多い**（下の各所）
   - **何か:** テストが表や時計や記録の捕まえを組む小さな道具。
   - **問題:** 同じ中身が何か所にもある。直すときは全部を同じに直す必要がある。
     - `table_of`（読む → 組む → 表）: `looper_test_support.rs:12`・`parts_tests.rs:21`・`looper_always_tests.rs:18`・`looper_parts_tests.rs:16`・`parts_always_tests.rs:17`・`table_parts_tests.rs:23`・`actor_calls_tests.rs:63`・`actor_clock_tests.rs:36`（ほかに中に書いたもの: `looper_balloon_tests.rs:220-222`・`parts_calls_tests.rs:26`・`looper_parts_emo2_tests.rs:98`・`actor_parts_tests.rs:64-65`・`actor_talk_tests.rs:45-46`・`actor_trigger_fixture_tests.rs:34`）
     - `scope()`: `looper_test_support.rs:16`・`parts_tests.rs:25`・`looper_always_tests.rs:22`・`looper_balloon_tests.rs:19`・`parts_always_tests.rs:21`・`parts_calls_tests.rs:29`・`parts_film_tests.rs:69`・`actor_stage_tests.rs:18`
     - `playing(at)`: `looper_balloon_tests.rs:47`・`looper_parts_tests.rs:239`・`looper_talk_tests.rs:615`・`parts_always_tests.rs:62`・`parts_film_tests.rs:125`・`parts_trigger_tests.rs:146`
     - `capture_logs` の 6 本（`actor_test_support.rs:178`・`state_test_support.rs:21`・`looper_test_support.rs:62`・`parts_tests.rs:666`・`looper_tests.rs:873`・`table.rs:587`）
     - `table_and_logs` の 6 本（`table_always_tests.rs:17`・`table_calls_check_tests.rs:42`・`table_calls_tests.rs:13`・`table_trigger_tests.rs:14`・`table_interval_words_tests.rs:25`・`actor_trigger_fixture_tests.rs:32`）と `warns` の 3 本（`table_always_tests.rs:38`・`table_film_tests.rs:75`・`table_trigger_tests.rs:30`）・`KINDS` の 2 本（`table_calls_check_tests.rs:33-40`＝`table_calls_dot_sakura_tests.rs:16-23`）
     - 役者の時計つきの組み（Rig）4 本: `actor_test_support.rs:43-96`（`ClockRig`）・`actor_clock_tests.rs:48-98`・`actor_parts_tests.rs:53-117`・`actor_stage_tests.rs:42-103`。`send` の中身はほぼ同じ（`actor_talk_tests.rs:50` は `type Rig = ClockRig` で使い回している＝前例）
     - `balloon_cue`: `actor_dispatch_tests.rs:166`・`actor_replace_tests.rs:49`・`actor_strict_tests.rs:26`（`actor_test_support.rs:98` の `cue()` で足りる）
     - 1 コマの `PatternState`: `state_rebase_tests.rs:9`・`state_stage_tests.rs:19`・`state_bind_pattern_tests.rs:348`・`565`／1 コマの `LoopFrame`: `timeline.rs:235`・`timeline_repeat_tests.rs:13`・`timeline_calls_tests.rs:12`・`calls_tests.rs:50`
     - `tests/` の 2 組: `tests/cue_sequence.rs:17-60` と `tests/regression.rs:18-60` は doc 1 行を除いて同じ（`emote_cue`・emo2 の `surfaces.txt`・別名の表）、`tests/balloon_face_e2e.rs:41-69` と `tests/bind_e2e.rs:53-81` も doc の言い回しを除いて同じ
   - **直し方:** 今ある受け口（`looper_test_support.rs`・`actor_test_support.rs`・`state_test_support.rs`）へ寄せ、層をまたぐもの（`table_of`・`scope`・`capture_logs`）はクレートに 1 つのテスト用の受け口へ。`tests/` は `tests/common/mod.rs` を 1 つ作る。行は減る（`looper_tests.rs` 970 行・`parts_tests.rs` 921 行・`looper_talk_tests.rs` 898 行の上限の余裕も増える）。
   - **直さないと:** 例えば記録の捕まえ方を変えるとき、6 か所を同じに直す。
   - 3 種類: 当たらない。範囲の外: 及ばない。

7. **確かめが弱く、壊れても落ちないテストが 4 本**
   - `src/actor_bind_noevent_tests.rs:249-253`（`prefix_alike_name_is_not_bind`）: `logs.iter().all(|l| l.contains("level=DEBUG"))` は記録が 0 行でも真。`actor.rs:663-667` の `debug!` を消しても緑。→ 記録が空でないこと（その 1 行が在ること）も確かめる。
   - `src/table.rs:742-743`: コメントは「採録アニメは 2 本のみ」だが、確かめは `!table.is_empty()` だけ。実際は 3 本（surface30 の `CallOnly` を含む・736-740 行）で、コメントも誤り。余分な animation が入っても緑。→ 本数を正しく数えて等しいことを確かめ、コメントを直す。
   - `src/table_calls_check_tests.rs:112-135`（`ticks_add_no_warn`）: 刻みを回して警告が増えないことだけを確かめ、刻みが何かをしたこと（指令や再生）を確かめない。刻みの道が何もしなくても緑。→ 刻みの間に指令が出たことを 1 つ確かめる。
   - `src/table_calls_dot_sakura_tests.rs:51-64`（`word_warns` の 2 つ目の枝）と較正 `124-133`: 較正は 1 つ目の枝（6 種）しか通らない。2 つ目の枝は `method=start` の綴りに頼るので、記録の欄の形が変わると黙って 0 件になり、「警告 0 件」のテスト（94 行）が緑のまま。→ 2 つ目の枝の較正を 1 本足す（作れなければ枝を消す・設計で決める）。
   - 3 種類: 当たらない。範囲の外: 及ばない。

8. **危ない分岐なのにテストの無い本番の道**（どれも「壊れたら落ちるテスト」が 0 本）
   - `src/table.rs:227-243`: `periodic,0`・`talk,0`（型の上で 0 のとき）を警告して採らない枝。読み手（`crates/areka-parsers/src/shell/decode.rs:434-438`）は 0 を `Other` に倒すので文字列からは来ないが、`table.rs` の中のテストは型を手で組むので試せる。
   - `src/table.rs:398-405`: 動く絵の子の待ちの合計が 0 のとき警告して採らない枝。
   - `src/bind.rs:367-373`（`name_of`）: 本番の呼び手は `crates/areka/src/emo2_boot/shell_carry.rs:173・184`（読み直しで着せ替えを名前で出し直す）。テストは見つかる 1 件（`bind_tests.rs:501`）だけで、未宣言の番号（`None`）と、同じ番号に名前が 2 つあるとき「表の順で最初」（doc 363 行）は試していない。
   - `src/bind.rs:520-544`（`apply_named_bind` をスコープ 2 以降で）: `resolve` の `Char(2)` は試している（`bind_tests.rs:488`）が、当てる道（排他・mustselect）は 3 人目で試していない。
   - `src/calls.rs:167-184`: `always` の働きのコマを、印（`through`）がある状態で続けて当てる道（前の周の終わりのコマを窓に入れる `first = lo/period - 1` の決まり）。`calls_tests.rs:265-292` は 1 回ずつしか回さない。
   - `src/actor.rs:204-208`（`send_replace` が受け手の消えた後に `false` を返す）: どのテストも `true` しか見ない。本番の呼び手は `false` で `error!` を残す決まり（doc 203 行）。
   - **直し方:** 各 1 本、小さいテストを足す（本番は変えない）。足す先のファイルの 1,000 行に注意（`table.rs` 929 行・`calls_tests.rs` 301 行）。
   - 3 種類: 当たらない。範囲の外: 及ばない。

9. **`glyph_wall_ms` の代わりの道が `debug_assert!` のせいで試せない**（`src/looper_trigger.rs:172-190`）
   - **何か:** 文字の列に区切りの文字が無いという決まりの破れのとき、`warn!` を残して刻みの時刻で代える道。
   - **問題:** すぐ前の `debug_assert!`（181 行）が、テストと開発のビルドでは先に `panic` する。だから `warn!` の道はテストで通せず、開発のビルドで実機を回すと役者のスレッドが落ちる（落ちる道と記録を残して続ける道が、ビルドで違う）。
   - **直し方（選ぶのは設計）:** ⒜ `debug_assert!` を消し、`warn!` の道をテスト 1 本で固める（記録を残して続ける決まりに揃う）。⒝ 決まりの破れは致命と決めて、`warn!` の道を消す。
   - **直さないと:** 開発のビルドの実機の確かめで、口パクの区切りがずれた瞬間に SERIKO が止まる。
   - 3 種類: (1) に触れうる＝記憶の決めごと「ログ無し失敗経路の禁止（error!+Err・panic は致命限定）」。⒜ はこれに沿う。範囲の外: 及ばない。

10. **呼ばれない検体の受け口とその自己テスト**（`src/sample_test_support.rs:21-24・30-33`・`src/sample_test_support_tests.rs:20-28`）
    - **何か:** 里々の検体 `R_POST_and_KOMAINU` のシェルのフォルダを返す口。
    - **問題:** seriko の中で使うのは自分の自己テストだけ（ツリー全体を grep した。本体の areka は自前の写しを持つ）。そのテストのために、ライブラリのテストを回すたびに検体を取り寄せて展開する。
    - **直し方:** 口・`static`・自己テストを消す（約 15 行）。
    - **直さないと:** 使わない検体の展開に毎回の時間と `target\` の場所を使う。
    - 3 種類: 当たらない。範囲の外: 及ばない（`R_POST_and_KOMAINU` の登記は `sample-ghost-kit` 側にあり、ほかが使うので残る）。

11. **`ScopeStates` が「`ActorKey` は並べられない」という誤った理由で `HashMap` を使い、後から並べ替えている**（`src/state.rs:4-5・132-133・137`・並べ替えは `src/state.rs:338` と `src/looper.rs:476-481`）
    - **何か:** スコープごとの面の状態の入れ物。
    - **問題:** `ActorKey` は `Ord` を持つ（`crates/dola/src/cue/command.rs:21`）。`state.rs` 自身が 338 行で並べ替え、382 行で `BTreeSet` に入れている。doc が嘘で、そのために `shown_slots()` の答えが順不同になり、抽選の固定の順（D-7）を毎刻み並べ替えで作っている。
    - **直し方:** `scopes`・`balloon` を `BTreeMap` にし、`shown_slots()` を `stage_slots()` と同じ「スコープの昇順 → シェル → バルーン」で組めば、2 か所の並べ替えとその説明が消える。抽選の順は今と同じ（決定論のテストの期待値は変わらない）。
    - **直さないと:** 読む人が doc を信じて誤った前提で直す。
    - 3 種類: 当たらない。範囲の外: 及ばない（`ScopeStates` の公開の口の形は変わらない）。

12. **古くなって事実と食い違う説明**（読む人を誤らせるものだけ）
    - `src/actor.rs:29-31`「本タスクは列挙とチャネル形のみ用意し、ループ自体は結線しない」・`145-146`「本タスクでは単一送出端で足りる」: もう結線済みで、複製して配っている。
    - `src/resolve.rs:84-85`: 名前形の `\b` は「M-boot 未対応」「呼び手が warn!＋skip」とあるが、今は文字の層が読む正当な形で、呼び手は `debug!`（`actor.rs:691-700`）。
    - `src/state_surface_tests.rs:172`「切替 API を持たない」: `rebase_shell`（`state.rs:310`）がその口。
    - `src/looper_parts_emo2_tests.rs:17-72`: 「HEAD `eb1a7e77` の」ほかのテストの手書きの一覧で、もう食い違う（`looper_tests.rs` の「0→1000 が 21 本」は今 24 本のテストに `on_tick(0,` が 23 か所）。これからも古びる。
    - **直し方:** 誤った文を今の事実に直すか消す。一覧（`looper_parts_emo2_tests.rs`）は理由の 2 行だけ残して手書きの数を消す。
    - 3 種類: 当たらない。範囲の外: 及ばない。

### できれば（8 件）

13. **何とも比べていない・自分と比べるだけのテスト**（消す）
    - `src/resolve.rs:206-211`・`306-314`（純粋な関数を同じ入力で 2 回呼んで比べる）・`src/output.rs:214・219`（`assert_eq!(x, x.clone())`）・`src/timeline_repeat_tests.rs:293-305`（状態の無い `always_at` の順を入れ替えるだけ）・`src/settle_tests.rs:193-198`（毎回新しく組むので同じに決まる）・`src/timeline.rs:514-519`（同じ `frame_at(&frames, 172)` を 2 回。「任意に大きな経過」は次の `u64::MAX` が見ている）。
    - 隠れた大域の状態が入ったときだけ落ちる。今のコードには入りえないので、消して行を減らす。3 種類: 当たらない。

14. **同じことを確かめる重なったテスト**（消す）
    - `src/bind_tests.rs:375`（`empty_resolver_resolves_all_none`）は `:275` が全部の名前空間とカテゴリで同じことを見ている。`:787`（`accumulate_off_absent_is_noop`）は `:744`（`accumulate_idempotent_off_off`）の 1 つ目の比べと同じ。`:775`（`accumulate_on_off_round_trip`）は `:696`（`accumulate_basic_on_then_off`）と同じ。
    - `src/state_surface_tests.rs:157-170`・`215-230` は `:6-21` の最初の確かめ（静的な着せ替えを載せた `Show`）と同じ。
    - 3 種類: 当たらない。

15. **使われない公開の口**（`src/lib.rs:59-76`）
    - `accumulate`・`SETTLE_TICK_MS`・`lap_of`・`always_at`・`AlwaysView`・`LotteryBoundary`・`ScopeState` は、ツリー全体（ほかのクレート・seriko の `tests/`）で使う所が 0 で、ほかの公開の口の形にも出ない。`pub(crate)` に下げて `lib.rs` の doc の列から外す。
    - 3 種類: 当たらない。範囲の外: 及ばない（grep で 0 件を確かめた。消す前の確かめは要件 7.1 どおり記録する）。

16. **いつも `Some` を渡す引数**（`src/trigger.rs:65-67`）
    - `Armed::arm(at_ms: Option<u64>, …)` の呼び手 6 か所（`looper_trigger.rs:71`・`parts_trigger.rs:76`・`trigger_tests.rs`）は全部 `Some(..)`。`u64` にすれば `.filter` が `open.then_some(at_ms)` になる。3 種類: 当たらない。

17. **届かない `else`**（`src/parts.rs:560-565`）
    - `contains_key` → `insert` の直後の `get_mut` の `else { return CallOutcome::default() }` は通らない。写しを避ける今の形（90 行の理由）を保ったまま、`if let Some(..) = get_mut { .. } else { insert して取る }` の 1 つにまとめる。3 種類: 当たらない。

18. **素通しの包み `emit_display`**（`src/actor.rs:266-273`・呼び手は `actor.rs` の 8 か所と `actor_bind.rs:107`）
    - 中身は `out.send(command)` だけ。
    - 3 種類: **(1) に当たりうる**＝完了 spec `areka-P0-seriko-engine`（要件の前書き 11 行「状態→合成指令の単一発行点」）と `areka-P0-seriko-loop`（要件 86 行「既存の単一発行点および冪等ガードを継承する」）が「単一発行点」を決めている。包みを消すと「発行は 1 つの関数を通る」の形が変わるので、要件の討議の議題にする（残す案を推す）。

19. **書き出しの面の進め方が待ちの長さに比例して回る**（`src/settle.rs:94-98`）
    - **何か:** `--offline-dump` が面 1 つを `runonce` の待ちの合計まで 16ms ごとに進める輪。
    - **問題:** 待ちは作者が書いた `u32`（最大約 49 日）。例えば `runonce` の 1 コマに `wait` を 99,999,999 と書くと、約 625 万回 `handle_message` を回して書き出しが長く止まる。
    - **直し方（選ぶのは設計）:** ⒜ 刻みの数に上限を置いて、越えたら `warn!` して打ち切る。⒝ 何もしない（作者の誤りで、道具が遅くなるだけ）。
    - 3 種類: **(1) に触れうる**＝完了 spec `areka-P0-offline-surface-dump` の「画面と同じ道で進める」（`settle.rs:1-8`・design「SettleSurface」）。⒜ でも上限までは同じ道なので触れないが、討議で確かめる。範囲の外: 及ばない（呼び手 `crates/areka/src/offline_dump/` は形が変わらない）。

20. **`ponytail:` の注釈が付いた限界**（`src/trigger.rs:167-171`）
    - **何か:** 消去の知らせで文字の数えを揃えるとき、数え始めが 0 で止まると次の `talk` の区切りが 1 回遅れることがある、という書かれた限界。
    - audit の作法では、限界を書いた注釈は「今の負荷が限界を越えていなければ決めごと」。起きるのは「起点の見積もりが前へ動いて現れた数が減って見える間に消去が届いた」ときで、今の実機でどれだけ起きるかは測っていない（Research Needed）。
    - 3 種類: **(2) に当たる**（わざと置いた先送りの印）。直すと口パクの時刻が変わるので **(3) にも触れる**。ukadoc（https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/#talk ）の `talk,数値` は「数値分の文字がくるごとにアニメーションする。」。限界は「1 回遅れることがある」だけで正典の言い方からは外れないが、直すなら数え始めを符号つきにする設計が要る。残す案を推す。

## 4. 指摘にしなかったもの（確かめて捨てたもの）

- `tests/balloon_face_e2e.rs:119`・`tests/bind_e2e.rs:195` の `recv_timeout(5 秒)`: 終わり方は FIFO と `join` で決まり、5 秒は止まったときに落とすための上限。結果は時刻に依らない（5 秒止まるほど詰まったときだけ赤）。具体的に壊れる場面が無いので捨てた。
- `tests/loop_integration.rs:577-640` のプロセス全体の記録の捕まえ: 「`-2` の警告を出すのはこのテストだけ」と doc に書いた上での形で、今は正しい。
- 記録の目印 `"animation_id=1"` などの区切りの無い部分一致（`parts_trigger_tests.rs:406` ほか）・`parts_tests.rs:56-60` の部品の番号 0〜999 だけの走査: 今の素材に 10 以上・1000 以上の番号が無いので、今は誤って当たらない。
- `looper_parts_tests.rs:315-318` は「落ちない」と読み手が言ったが、`runonce` が何度も鳴るバグなら時計が残って落ちるので、確かめは生きている。
- `actor_dispatch_tests.rs:10` と `:43`、`:68` と `:376` は似ているが、確かめる性質（担当外の cue も橋渡しする・本物の役者を止めた後）が違う。
- `actor.rs:647-652`・`751-757` の届かない防御の腕: `match` の網羅に要る形で、記録を残すので捨てた。
- `MockSurfaceOutput` が本番のコードに在ること: ほかのクレートのテスト 17 か所が使い、`#[cfg(test)]` ではクレートの外へ出せないので残す。
- `calls.rs` の `always` の追いつき（長く止まった後に周の数だけ働きのコマを当てる）: 記憶の決めごと「時刻は正確に扱う（遅れたら過ぎた時間の分だけ進める）」どおり。
- clippy の指摘の機械的な直し（`clippy-199-lints` の brief 63 行の `actor.rs` の `large_enum_variant`・`looper.rs` の `collapsible_if`）: audit が同じ所を指していない（指摘 1 は `looper.rs:433-441` のテストを直すだけで、番人の形は変えない）ので触らない（要件 7.5）。

## 5. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | 足りないもの・縛り |
| --- | --- | --- |
| 1 audit をかける | この報告（全 84 ファイル） | なし |
| 2 全部直す | 指摘 20 件（必ず 0・直したほうがよい 12・できれば 8） | 3（`BindGroupName` が組めない＝Research Needed）・9・19 は直し方を設計で選ぶ |
| 3 残してよい 3 種類 | 当たりうるのは 9（1）・18（1）・19（1）・20（2）（3） | 討議で裁定が要る（Constraint） |
| 4 直したことを示すテスト | 決定論の檻の作り（注入の時計・乱数・`log-capture-kit`） | 本番のバグは 0 件なので「直したバグ 1 件ごとのテスト」は 0 本。テストの足しは 1・5・7・8・9 |
| 5 直した後の audit で 0 件 | 同じ作法でかけ直す | 検証の段のタスク |
| 6 守る決まり | 1,000 行の番人・`error!`＋`Err` | **縛り**: `looper.rs` 993 行（残り 7 行）・`parts.rs` 973・`looper_tests.rs` 970・`table.rs` 929・`parts_tests.rs` 921。足す直しは同じファイルで削る分と組にする。依存は足さない（どの指摘も足さずに直せる）。`doc/`・`docs/` を名指す行は範囲に 0 行で、どの直しも足さない |
| 7 消す前の確かめと呼び元 | grep で確かめた（15・10・3） | 範囲の外へ及ぶのは 2（`log-capture-kit`）と 3（`crates/areka` のテスト 1 ファイル）だけ。どちらも C17 の兄弟の範囲ではない |

## 6. 直し方の進め方の案

- **案 A（今のファイルに手を入れる）:** 指摘ごとに今のファイルを直し、写しは今ある受け口へ寄せる。新しいファイルは 2 の分割先と、6 の `tests/common/mod.rs`（と、要るならクレートに 1 つのテスト用の受け口）だけ。
  - ✅ 行が減る・形が今の作法のまま。❌ 6 は触るテストのファイルが多い（約 40 ファイル）。
- **案 B（テストの道具を先に 1 つの受け口にまとめてから直す）:** 6 を先にやり、その上で 1・5・7・8 のテストを足す。
  - ✅ 足すテストが最初からまとめた道具を使える。❌ 最初の段が大きい。
- **案 C（混ぜる）:** 本番の直し（3・4・5・9・11・15〜19）を先、テストだけの直し（1・2・6・7・8・10・12〜14）を後に分ける。20 タスクを越えそうなら、後ろの組を 2 本目の spec にできる。

## 7. 規模とリスク

- **タスクの見積もり:** 約 16〜19 タスク（1・2・3・4・5・6〔src と tests/ の 2 つ〕・7・8・9・10・11・12・13＋14・15〜17・18・19・20・最後の audit のかけ直しと全体テスト）。上限 20 の近くなので、要件の討議で「テストだけの直し（6・7・10・12・13・14）を同じ範囲の 2 本目へ切る」を議題にできる（要件 2.4）。切らずに 1 本で収まる見込みのほうが高い。
- **手間:** M（3〜7 日）＝既にある作法の中の直しで、新しい仕組みは無いが、6 で触るファイルが多い。
- **リスク:** 低＝本番の振る舞いを変える直しは 11（順は同じ）・9・19 だけで、どれも決定論のテストで縛れる。上限 1,000 行の近いファイルの足し引きだけ注意。

## 8. 設計の段へ持ち越す調べ（Research Needed）

1. 指摘 3 の ⒜: `areka-parsers` に、descript の文字列だけから `BindGroupDefaults` を作る口があるか（無ければ、テストの側は今の `bind_tests.rs:443-459` の `groups_of` のような WorkDir 経由になる）。
2. 指摘 20: 「数え始めが 0 で止まる」が今の実機（emo2 と検体）でどれだけ起きるか。
3. **兄弟との申し合わせ（2026-11-05・`audit-parsers-sakura` から）**:
   - 向こうの指摘 15 で、bindgroup の欄が「スコープ番号を鍵にした 1 つの表」に変わる。`bind.rs` の `from_groups`（229-269 行）の 3 枝は、向こうが呼び出し元の直しとして畳む。こちらは `from_groups` の中身に触らない。
   - 向こうの指摘 36 で `resolve.rs` の別名の引きを、38 で `Surface.id` の字面を触る。どちらも向こうの実装の最後の段で行う。行がぶつかったら、後から着地するほうが main を取り込んで揃える。
   - 新しい表の型を parsers の外から組めるかは、向こうに頼んだ。組めれば、指摘 3 の ⒜ のテストはその型で組む（調べ 1 が解ける）。組めなければ descript の文字列を読む口で組む。
4. 指摘 11: `ScopeStates` を `BTreeMap` にしたとき、抽選の順のテスト（`looper_tests.rs`・`looper_always_tests.rs`・`looper_trigger_tests.rs` の乱数の列を焼き込んだもの）がそのまま緑か。
