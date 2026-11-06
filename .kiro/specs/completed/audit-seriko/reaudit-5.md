# audit-seriko 6.1 audit のかけ直し（5 回目）

- HEAD: `bd312b93`・枝 `claude/kiro-start-audit-seriko-8ceffb`
- 道具: `ponytail:ponytail-audit`。範囲は `crates/areka-seriko/` だけ。要件 1.2・1.3・5.1 に従い、指摘の数に上限を置かずに全部を挙げた。「見ていない」ファイルは無い
- 範囲: 88 ファイル・28,737 行
  - 本番 24 ファイル 8,528 行
  - テストとテストの道具 61 ファイル 20,015 行
  - テスト用の素材 2 ファイル 161 行
  - `Cargo.toml` 33 行
- 読み方
  - 本番のファイルは、全部の行を誰かが読んだ。
    - 取りまとめ役: `looper.rs`・`looper_refresh.rs`・`looper_calls.rs`・`parts.rs`・`parts_trigger.rs`・`parts_calls.rs`・`parts_refresh.rs`
    - 読み手 E: `actor.rs`・`actor_bind.rs`・`bind.rs`・`state.rs`・`table.rs`（中のテストを含む）・`table_calls.rs`・`output.rs`・`resolve.rs`・`settle.rs`・`lib.rs`
    - 読み手 B: `trigger.rs`・`calls.rs`・`talk.rs`・`timeline.rs`・`shell_fallback.rs`・`skin_facts.rs`
    - 読み手 C: `looper_trigger.rs`
  - テストの 61 ファイルと素材 2 ファイルと `Cargo.toml` は、4 人の読み手（A: actor の 13 本、B: bind・state・timeline などの 19 本、C: looper の 11 本、D: parts・table・`tests/` の 21 本）で分けて、全部の行を読んだ。
  - 読み手の挙げた件は、取りまとめ役が該当の行を開いて確かめた（V1・V2・V3・V4・V6・V7 は本文の行も読み直した）。
  - 「壊しても緑のまま」は読みで確かめたもので、摂動は回していない。
- 動かしたもの（コードは 1 バイトも変えていない）
  - `cargo clippy -p areka-seriko --all-targets --no-deps -- -D warnings`: 警告 0
  - `cargo doc -p areka-seriko --no-deps`: 警告 0
  - `cargo test -p areka-seriko`: 573 件緑（546＋5＋9＋9＋4＋0）
  - 全体テスト・`--ignored`・areka-impl-watch.exe は回していない
- 扱わないもの: research.md §3 の 1〜20（18・20 は「残す」の裁定どおり）、reaudit-1.md の R1〜R26、reaudit-2.md の S1〜S24、reaudit-3.md の T1〜T15、reaudit-4.md の U1〜U20。どれも直し済み。決め済みの「`refresh` で止まった頭の記録が 1 刻み後」（D4）、適用の前の状態を指す「旧パーツ」の型の語、`looper_parts_emo2` の「本 spec の前と同じ」も扱わない

## このクレートがすること・想定した負荷

SERIKO（シェルの面のアニメーション）を、ゴーストごとの役者のスレッドで 16ms ごとに進め、表示の指令を出す。台本の合図（`\s`・`\b`・`\![bind]`）を受けて、面と着せ替えを替える。想定した負荷は、利用者 1 人・ゴースト数体・役者 1 体あたり毎秒約 60 刻み。

## round 4 の直しを確かめた結果

- 本番のバグは見つからなかった。
- U1（`looper_refresh.rs:157-166`）: 鳴り直した頭が `Pending`・`Stopped` なら欄を外す。`FrameStatus` は 4 つの形だけなので、`_` の腕は `Pending` と `Stopped` に当たる。どちらも刻みの道（`looper.rs:279-281`・`:322`）と同じ扱い。足したテスト `refresh_clears_the_leftover_frame_when_the_refired_head_is_pending`（`looper_trigger_tests.rs:775`）は、直しを戻すと赤になる。ただし `Stopped` の側を確かめるテストが無い（V1）。
- U2・U3・U4・U6・U9・U10・U11・U19: 足した確かめは、直しを戻すと赤になる。時計の順も戻らない。
- U7（`parts_trigger.rs:21-32`）: `or_insert_with` の形にした。呼び手は 2 つとも、構えてある部品を先に外している（`parts.rs` の 2 段目・`parts_refresh.rs:100`）。doc も合っている。
- U8・U14・U17: 本番の doc は直っている。テストの側に同じ種類の残りがある（V4・V15）。
- U13: `ElementKind::Film` を手で組む所は、もう無い。絵の列（`FilmSheet`）の写しは残っている（V13）。
- U12・U20: 写しは消えた。U12 で消したテストの見出しが古くなった（V9）。
- 1,000 行の上限: どのファイルも越えていない（最大は `looper_tests.rs` の 981 行）。

## 指摘

### 必ず直す（0 件）

無い。

### 直したほうがよい（7 件）

**V1. U1 で足した `refresh` の「止まった頭の欄を外す」腕を確かめるテストが無い**【テスト】（`src/looper_refresh.rs:163-165`・テストは `src/looper_trigger_tests.rs:775` の 1 本だけ）
- 何が悪いか: 新しいテストは `Pending` しか通らない。
- 具体例: `_ => remove` を `FrameStatus::Pending => remove, FrameStatus::Stopped => {}` に替えても緑のまま。この道には実際に届く。`start` は再生を入れ替えるが欄に触らない（`looper_calls.rs:113-131`）。そのため、前の再生のコマが欄に残ったまま、呼ばれた頭（`called`）が `refresh` の時刻にはもう `Stopped` になっていることがある。
  - animation1 は `never` で、111（待ち 0）→ `-1`（待ち 100）。
  - animation0 は `runonce` で、`start,1`（0）→ 100（待ち 50）→ `start,1`（50）。
  - 10 に面を出すと 1 が 10 に始まり、30 の刻みで 111 が載る。
  - `refresh(Some(200))` で 1 は 60 に始め直され、経過 140 で `Stopped` になる。直しがあれば欄 1 は `None`。上の摂動では 111 が残る。
- いちばん小さい完全な直し: この例をテストに 1 本足す。あわせて `_` を `FrameStatus::Pending | FrameStatus::Stopped` と書く（形が増えたときに、黙って欄を外さないように）。
- 種類 3.1: (1)〜(3) のどれにも当たらない。推奨は「直す」。

**V2. looper のテストの道具 `switch` は「アクターの面の切り替えと同じ順で踏む」と言うが、`on_surface_shown` を踏まない**【テスト】（`src/looper_test_support.rs:16`・`:27-40`）
- 何が悪いか: アクターは `states.apply` の直後に、結果が `Changed` でも `Unchanged` でも `on_surface_shown` を呼ぶ（`actor.rs:777`・`:519-536`）。これでずれを (0,0) へ戻し、止めた `always` の印を消す。`switch_at` はこれを飛ばす。同じ「アクターと同じ順」を前提にしている所は、ほかにもある。
  - `looper_trigger_tests.rs:4`・`looper_always_tests.rs:4`・`looper_talk_tests.rs:5-6` のモジュールの doc
  - `looper_parts_tests.rs:225`
  - `looper_film_count_tests.rs:42`
- 具体例: `stop,0` で止めた面 0 の `always` を、`switch` で 1 → 0 と行き来させる。アクターなら印が消えて、0 は始め直す。道具の上では止まったまま。`move` のずれも、道具の上では戻す命令が出ない。今のテストには、この食い違いを踏むものは無い（読みで確かめた）。
- いちばん小さい完全な直し: `switch_at` の `states.apply` の直後（`Changed` を見る前）に `rt.on_surface_shown(&scope(), Slot::Shell);` を 1 行足す。既存のテストは切り替えの前にずれも印も持たないので、緑のままの見込み（読みだけで、回していない）。もう 1 つの道は、doc に「`on_surface_shown` は踏まない」と書くこと。
- 種類 3.1: (1)〜(3) のどれにも当たらない。推奨は「直す」（1 行足すほう）。

**V3. `alternativestop` が「再生中で絞らずに並びの全部から選ぶ」ことを確かめていない**【テスト】（`src/calls_tests.rs:173-189` `alternative_stop_choosing_a_not_playing_target_does_nothing`）
- 何が悪いか: doc はそう言うが、テストは選びに渡す数（並びの長さ）を見ていない。`fixed_pick` は渡された数を無視して、決まった値を返す。
- 具体例: `calls.rs:103-104` の Stop の腕で、選ぶ前に `targets` を再生中のものだけに絞る（再生中は 1 だけなので、並びは `[1]` になる）。
  - `fixed_pick(1)` の回: `get(1)` が `None` になって何も止まらず、期待の `[0,1]` のまま緑。
  - `fixed_pick(0)` の回: 1 が止まり、期待の `[0]` のまま緑。
- いちばん小さい完全な直し: 1 回目の選びを `Box::new(|b| { assert_eq!(b, 2, "絞らない"); 1 })` にする。`alternative_start_picks_one_with_the_pick_rng`（`:164-167`）と同じ形。
- 種類 3.1: (1)〜(3) のどれにも当たらない。推奨は「直す」。

**V4. 部品の `refresh` を「時計を書き換えない口」と呼び、「抽選の時計も乱数も触らない」「何も記録しない」と書くテストの説明が偽（U8 の直しの漏れ）**【説明】
- 場所:
  - `src/parts_tests.rs:445`・`:453`・`:479`・`:542`・`:570`・`:749`・`:807`
  - `src/actor_parts_tests.rs:157`
  - `src/looper_parts_tests.rs:79`
- 何が悪いか: round 4 で、本番の doc（`parts.rs:31-36`・`parts_refresh.rs:27-29`・`looper_refresh.rs:24-25`）は「`always` の時計と 3 語の時計を書き換える（働きのコマが始めた・止めた時計と選びの乱数を除く）」に直した。テストの説明は、古い呼び名と例外の無い言い方のまま。`:749` は「何も記録しない」と言う。しかし `refresh` は `always` の時計が生まれた記録と、部品の引き金を鳴らした記録を出す。
- 具体例: 同じ `refresh_at` を通るテストが、逆のことを確かめている。
  - `parts_trigger_tests.rs:622-623`: `refresh` が時計 `playing(1234)` を作る。
  - `parts_always_tests.rs:140-142`: `always` の時計を作る。
  - `parts_calls_tests.rs:443-450`: `refresh` が「part trigger periodic を鳴らした」の行を出すことを前提にしている。
- いちばん小さい完全な直し:
  - 呼び名を「出来事の直後の作り直し（`refresh`）」に替える。
  - `:453`・`:479`・`looper_parts_tests.rs:79` は「抽選の animation の時計は書き換えない（働きのコマが始めた・止めた時計を除く）」にする。
  - `:749`・`:807` は「抽選の発火・末尾での保持・停止の記録は出さない（出すのは `advance` だけ）」にする。
  - `actor_parts_tests.rs:157` も同じ語に揃える。
- 種類 3.1: (1)〜(3) のどれにも当たらない。推奨は「直す」。

**V5. U16 で足した `actor_dispatch_tests.rs` の「番号の出どころ」の行が、cue-playback-duration の番号を seriko-engine のものとしてしまう**【説明】（`src/actor_dispatch_tests.rs:3-4`）
- 何が悪いか: `//!` は「無印と R は areka-P0-seriko-engine。duration を守るのは 2.3・R5.4 で、cue-playback-duration」と書く。しかし本文には、cue-playback-duration の番号を指す行がほかにもある。
  - `:640`「2.2/2.3/5.4/11.6」
  - `:688`「R2.2/2.3」
  - `:708`・`:737`「5.4」
  - `:762`・`:794`「2.2/2.3」
- 具体例: `:688` の「R2.2」を `//!` どおりに読むと、seriko-engine の要件 2.2（alias／name を解決表で解く）になる。中身は cue-playback-duration の 2.2（担当外の cue でローカルな遅延を生まない）。`:737` の「5.4」も、seriko-engine では「Send で発行する」になってしまう。
- いちばん小さい完全な直し: `//!` を「2.2・2.3・5.4・11.6（R つきを含む）と D4 は areka-P0-cue-playback-duration」にする。
- 種類 3.1: (1)〜(3) のどれにも当たらない。推奨は「直す」。

**V6. 検体の受け口の自己テストが、ほかのテストと全部重なっている**【テスト】（`src/sample_test_support_tests.rs` 全体・`src/sample_test_support.rs:39-41`）
- 何が悪いか: `konnoyayame_shell_root().join("surfaces.txt")` が在ることは、`table_interval_words_tests.rs:100-103`（`konnoyayame_surface0_animation0_is_recorded`）が同じパスを `fs::read(..).expect` で読むことで、もう確かめている。それなのに doc は、綴り違い・展開先の取り違え・登記漏れは「どれもこの 1 本で赤になる」と言う。また doc は「検体の受け口が」と全部を指すように書くが、ほかの 2 つの口（`emo2_root`・`dot_sakura_shell_root`）は見ていない。
- 具体例: `konnoyayame_shell_root` の `"shell/master"` を `"shell/mastr"` に壊すと、2 本とも赤になる。自己テストだけが赤になる壊し方は無い（U12 と同じ種類）。
- いちばん小さい完全な直し: `sample_test_support_tests.rs` と、`sample_test_support.rs:39-41` の `#[cfg(test)] #[path] mod tests;` を消す。research.md 指摘 10 で里々の自己テストを消したのと同じ扱い。
- 種類 3.1: (1) に当たらない（完了した areka-P0-shell-implicit-surface の要件 7 の 11・12 は「共有の道具で受ける」「新しいファイルに置く」で、この自己テストを求めていない）。(2)・(3) にも当たらない。推奨は「直す」。

**V7. `AnimationTable::from_world` の doc が「それ以外の `Other` は `debug!`」と言うが、`talk`・`periodic` の崩れた綴りは `warn!`**【説明】（`src/table.rs:188-190`・`:211-213`）
- 何が悪いか: 次の場合は、どれも `warn!` を出す。しかし `:190` が `warn!` の場合として挙げるのは「`k == 0`・コマ列空」だけ。
  - 先頭の語が `talk`・`periodic` の `Other`（`table.rs:276-283`）
  - `periodic,0`・`talk,0`（`:231`・`:240`）
  - 待ちの合計が 0 の `always`（`:345`）
- モジュールの doc（`:26` の節）は言い直しているが、`from_world` の doc とループの中のコメント（`:211-213`）には言い直しが無い。
- 具体例: `animation0.interval,talk,abc` → `Other("talk,abc")` → 実際は `warn!`「数値が無効ゆえ非採録」。doc は `debug!` と言う。
- いちばん小さい完全な直し:
  - `:189` に「先頭の語が `talk`・`periodic` の `Other` は `warn!`」を足す。
  - `:190` に「数値が 0 の `periodic`・`talk`・待ちの合計が 0 の `always`」を足す。
  - `:212` のコメントも同じに揃える。
- 種類 3.1: (1)〜(3) のどれにも当たらない。推奨は「直す」。

### できれば（10 件）

**V8. U5 の直しの残り（`actor_clock_tests.rs`）**【テスト】
- `:165-166` の `run_actor` の doc は「時計が刻みを追い越さないよう」と言う。しかし時計は 1030 で止まっていて、止まった時計は刻みを追い越せない。消した道で起きていたのは、刻み 1100 が時計を追い越した後に、合図が 1030 で処理される（時刻が戻る）ことだった。
  - 直し: 「時計は 1030 で止まっているので、刻み 1100 の後に合図を流すと時刻が戻る。だから流さない」にする。
- `:18`・`:23`: `TOP_ALWAYS` の面 1（`surface1\n{\n}\n`）と、doc の「1 は何も持たない。」が死んだ素材になった。`\s[1]` を外した後、このファイルで面 1 を引くものは無い。
  - 直し: 両方を消す。

**V9. `actor_bind_loop_tests.rs:17` の見出しが、このファイルに無い檻（名前の門）を持つと言う（U12 の削除で古くなった）**【説明】
- 中身: 見出しは「名前自己選別（`name` が `bind` か `bind-noevent`）→ …」の一本経路を、このファイルで檻に入れると言う。U12 でケース 22 を消した後、担当外の名前を流すテストはこのファイルに 0 本。残っているのは `actor_bind_noevent_tests.rs:237` の `prefix_alike_name_is_not_bind` だけ。
- 直し: ④ と同じく「（担当外の名前は `actor_bind_noevent_tests.rs`）」を添える。

**V10. `actor_bind_loop_tests.rs:64`・`:90` の「6.3」が、名指しの spec と合わない**【説明】
- 中身: `//!` は R・D を areka-P0-mayuna-compose としている。印の無い「6.3/3.5/7.1」の 3.5 と 7.1 は mayuna の番号と合う。しかし mayuna の 6.3 は「emo-present 本体を改変しない」で、「適用後も処理を続ける」とは関係が無い。
- 直し: `:64`・`:90` から「6.3」を消す（または正しい出どころを名指す）。

**V11. どの spec の番号か分からないもの（U16 の漏れ）**【説明】
- テストの中のモジュール:
  - `src/actor_bind_noevent_tests.rs:218`「（D7）」: このファイルの spec（script-small-tags）の design に D7 は無い。指しているのは areka-P0-mayuna-compose の D7。
  - `src/state_rebase_tests.rs:1`「要件 2.6・3.5」: areka-P0-shell-balloon-switch。
  - `src/state_surface_tests.rs`: モジュールの doc が無いまま、要件 3.1〜4.6・6.1・DD8・R4.6 を使う。
  - `src/state_bind_pattern_tests.rs`: モジュールの doc が無いまま、R3.4〜R3.8・D5・D9・要件 5.1/6.1/6.2/2.1・bindopt を使う。
  - `src/bind_tests.rs:1`: R3.2〜R4.3・D7〜D9・決定3 を使い、spec の名は `:344` の 1 節にしかない。
  - `src/lib_test_support.rs:13` `KINDS`「（要件の番号）」: seriko-pattern-calls。
  - `src/looper_replace_tests.rs:1-2`「（要件 2.8・3.5）」: areka-P0-shell-balloon-switch。
  - `src/looper_tests.rs`: モジュールの doc が無いまま、「要件 3.1」など（areka-P0-seriko-loop）・「D-7」・「bindopt 7.3」を使う。
- 統合テスト（出どころは、そのファイルを足した commit で確かめた）:
  - `tests/balloon_face_e2e.rs:1`: areka-P0-balloon-face-cue。
  - `tests/bind_e2e.rs:1`: areka-P0-mayuna-compose。「bindopt」は一度も書き下されていない。
  - `tests/regression.rs:1`: areka-P0-seriko-engine。
  - `tests/loop_integration.rs:1`: areka-P0-seriko-loop。
  - `tests/common/mod.rs:177` の「DD-6」と `:31` の「段 ③」も、何を指すか書いていない。
- 直し: 各ファイルに、spec の名を書いた `//!` の行を 1 つ足す（「bindopt＝areka-P0-bindoption-exclusivity」を含む）。`actor_bind_noevent_tests.rs:218` は「（areka-P0-mayuna-compose D7）」にする。`KINDS` の doc には「seriko-pattern-calls の」を足す。`common/mod.rs:31` は「段 ③」を「`SampleRoot` の `Drop`」と言い換える。

**V12. 確かめていない「状態不変」を言う**【説明】（`src/actor_dispatch_tests.rs:155-161` ケース 3・`src/actor_bind_loop_tests.rs:841` ケース 21）
- 中身: どちらも、確かめているのは発行が 0 件であることだけで、状態は見ていない。今の入力（EntityRef・解決できない名前）では状態を変える道が無いので、今は害が無い。
- 直し: 文言から「状態不変」を外す。または、`states.current_binds` と `current_pattern` の前後が等しいことを 1 行足す。

**V13. U13 の残り: 絵の列（`FilmSheet`）を組む手書きが 4 通りある**【テスト】
- 場所:
  - `src/settle_tests.rs:74-82` と `src/looper_film_count_tests.rs:30-38`: 中身ごと同じ（`FilmId(10)`・`a.png`・コマ 10/11/12・各 100ms・`laps: None`・`(2,2)`・`rest: 0`）。
  - `src/parts_film_tests.rs:35-45` の `sheet` と `src/table_film_tests.rs:23-33` の `sheet`: 引数の型（`FilmId` か `u32` か）とパスの組み方が違うだけ。
  - `src/table.rs:937-941` にもある。
- 具体例: `FilmSheet` に欄が 1 つ増えると、5 か所を手で直すことになる。
- 直し: `lib_test_support.rs` の `film_world` の隣に `film_sheet(id: FilmId, path: &str, delays_ms: &[u32], laps: Option<u32>) -> FilmSheet` を 1 つ置き、写しを替える。

**V14. 部品の時計を読む道具が 3 通りある（U20 の片側）**【テスト】
- どれも、部品 100 の時計を読む。
  - `src/looper_tests.rs:128` の `part_clock`（シェル）
  - `src/looper_balloon_tests.rs:260-263` の閉包
  - `src/looper_talk_tests.rs:588-592` の `part_clock(st, slot, id)`
- `src/looper_parts_tests.rs` は `rt.parts.clock(&scope, 100, 0)` を 13 か所、手で書いている。
- 直し: `film_clock` と対になる `part_clock_at(rt, slot, part, id)` を `looper_test_support.rs` に置き、使い回す。

**V15. 前の版と比べる語の残り（U17 の漏れ）**【説明】
- 本番:
  - `src/state.rs:223`「（動的 bind 不在なら static_binds＝非退行・R3.8）」→「（動的 bind 不在なら static_binds・R3.8）」
  - `src/state.rs:435`「**非退行（要件 3.8）**: 本メソッドは…」→「**表示の状態には触れない（要件 3.8）**: …」
  - `src/actor.rs:247`「知らせてから今の送り出しをする」→「知らせてから `deliver` で送る」
- テスト:
  - `tests/bind_e2e.rs:73-75`・`:329-331`「期待値は（bindopt 是正の前後で）不変である（bindopt 設計 §テスト影響監査）」→「1 カテゴリ 1 パーツなので、排他置換と加算の結果は同じ集合になる」だけを残す。
  - `tests/bind_e2e.rs:301`「pre-fix の積算バグなら」→「積算する実装なら最終が `{1207,1301,1303,1304}` になるので、全体の等しさの照合がこの違いを捕まえる」
- 「今の面」「今の着脱」のように、今の状態を言う「今」は残してよい。

**V16. `lib.rs:19-20` の「bind 解決層 … 純関数群（parsers 非依存）」は偽**【説明】
- 中身: `BindResolver` を中身つきで組む口は `from_groups(&BindGroupDefaults)` だけ（`bind.rs:181`）。`DefaultBinds::from_groups` も同じで、型は `areka_parsers::package`（`bind.rs:24`）。parsers を使わずに組める口は `empty()` しか無い。
- 直し: 「（parsers 非依存）」を消す。または「（descript の読み `BindGroupDefaults` から組む）」に替える。

**V17. `actor.rs:18`・`:299-306` の「破損入力（Unresolved/Invalid=error!／EntityRef=warn!）のみが真の異常」の「のみ」は偽**【説明】
- 中身: 着せ替えの腕にも、次の記録がある。
  - `error!`: Malformed（`actor_bind.rs:27`）・NotFound（`:49`）
  - `warn!`: NoNamespace（`:42`）・MustSelectKept（`:59`）・自分宛の非正準の Custom（`actor.rs:639`）・未知の Shell 系 command（`:756`）
- 具体例: `\![bind,腕,存在しない,1]` → `error!` を出す。doc の一覧には無い。
- 直し: 「のみが」を「などが」にし、「着せ替えの腕の記録は `actor_bind.rs`」を 1 句足す。

V8〜V17 は、どれも 3.1 の (1)〜(3) に当たらない。推奨は「直す」。

## 参考（指摘にしない）

- `src/trigger_tests.rs:101-110`: U19 で 1266 の回も `|_| false` になり、9000 の回（「再生が終わった後」）とまったく同じ場合になった。どちらか 1 行は要らない。
- `src/actor_tick_tests.rs:11`: spec の名が `//`（`//!` でない）にある。「R の番号は seriko-loop」とあるが、印の無い番号も seriko-loop のもの。
- `src/actor_clock_tests.rs:231-236`: 時計つきの回は否定（刻み 1100 で出ない）だけを確かめる。`sink.send_tick(1130)` を足せば、正の確かめ（`(0, Some(102))`）にできる。
- `src/parts_calls_tests.rs:446-454`・`src/parts_tests.rs:960-966`: 手書きの `.filter(contains)` は、同じファイルの `count(..)` で 1 行に書ける。
- `src/parts_film_tests.rs:38`: `FilmSheet.path` が `"10.png"`・`"20.png"` で、`film_world` に渡すパスは `a.png`・`b.png`（記録にしか使わないので害は無い）。`:6`「（`table_film_tests.rs` と同じ組み方）」は「`film_world` で組む」のほうが正確。
- `src/table_calls_dot_sakura_tests.rs:126`: `capture_lines` を直に呼んでいる（`capture_logs` で書ける）。
- `src/table_interval_words_tests.rs:63-77`: doc は「`debug!` に残す」と言うが、`level=DEBUG` は見ていない（同じ道は `table_always_tests.rs:66` が level つきで確かめる）。
- `src/looper_refresh.rs:23`・`:25`: 折り返していない長い doc の行。
- `src/looper_parts_tests.rs:306`: 「1100・1400・1500 のどの刻みでも」の 1100 は、刻みではなく `refresh`。
- `src/looper_always_tests.rs:5-6`: モジュールの doc は「`at_ms` に `None`」と言うが、`event_time_is_not_fed_into_tick_guard` は `Some(500)` を使う。
- `src/looper_trigger_tests.rs:547-548`: 「表示の指令も生まれない」。`switch` の `Show` は出ていて、生まれないのは刻みの指令だけ。
- `src/looper_test_support.rs:65`・`:68`: `areka_emo_compose::` の完全な道で書いている。`src/looper_tests.rs:106-115`: `ActorKey::from("0")` を手で書いている（`scope()` がある）。
- `src/state_bind_pattern_tests.rs:779`「未知は除外」: 状態を持たないスコープは、もともと列に出ない。
- `src/shell_fallback_tests.rs:1` は要件 3.2〜3.4、本番の `shell_fallback.rs:1` は 3.2〜3.5 と書く。
- `src/state_test_support.rs:3`: 「テストの道具は…1 か所の定義」と書いた同じファイルで、`binds_1100_1207` を自前で定義している。
- `src/state_stage_tests.rs:11-18` の `note`: `StageNote::Balloon` を手で組む 6 か所目。ただし置き場（state の中）から `looper_test_support` へは届かないので、残してよい。
- `src/actor.rs:290`: 「根拠は … `resolve` が常に `None`」。カテゴリ単位の形の根拠は、`written` が空なので `category_parts_in_order` が空を返すこと（結論は正しい）。
- `src/actor.rs:519`・`src/looper.rs:804`「面を出す合図を解いた後に」: `\s[-1]`・`\b[-1]` でも呼ばれる。
- `src/bind.rs:139`「（`BindResolver::from_groups` だけが持つ）」は、`new` を消した後の比べの名残り。`bind.rs:112-118`・`:242` は sakura と kero しか書かず、`chars` を落としている。`apply_bind_directive`・`AppliedBind` は再輸出されていないので、`pub(crate)` で足りる。`build_static_bindset` が `use` の行より前にある。
- `src/state.rs:128`・`:146`「書き込むのは `apply_bind`」: `apply_bind_exclusive` も書き、`rebase_shell` が消す。`:144`・`:148` の段落の途中の `[current_binds]: …` は、定義として読まれない（私有の欄なので `cargo doc` は警告しない）。
- `src/output.rs:14-20`: 列挙の doc が 8 つの形のうち 4 つしか書いていない。`src/table_calls.rs:40`: `Inert` の doc に、`check_calls` が純 bind の呼び出しと `move` を変えたものが入っていない。
- `src/actor.rs:11-13`「Shell 系 `Emote{key}` のみが一本経路」: `\b` と着せ替えも同じ形の経路を持つ。`src/table.rs:145`「ghost 再読込は再構築＝spawn し直し」: シェルの差し替えは表を替えるだけ。
- `src/parts_calls.rs:4` の「2 回」（round 4 の参考と同じ）。`src/looper_calls_tests.rs:154` の「M2」は requirements.md 4.5 の「SSP の測り M2」と合う。
- `Cargo.toml:15`「元々 dola 型で」: 「dola の型で」で足りる。

## まとめ

| 組 | 件数 | 番号 |
| --- | --- | --- |
| 必ず直す | 0 | — |
| 直したほうがよい | 7 | V1〜V7 |
| できれば | 10 | V8〜V17 |

| 札 | 件数 | 番号 |
| --- | --- | --- |
| 【本番の振る舞い】 | 0 | — |
| 【テスト】 | 7 | V1・V2・V3・V6・V8・V13・V14 |
| 【説明】 | 10 | V4・V5・V7・V9・V10・V11・V12・V15・V16・V17 |

- 新しい指摘は 17 件で、0 件ではない。件数は 26 → 24 → 15 → 20 → 17。本番の振る舞いを変える指摘は 0 件（round 4 は U1 の 1 件）。
- round 4 の直しが生んだもの・半分だけ直したもの: V1（U1 の `Stopped` の腕にテストが無い）、V4（U8 のテスト側の漏れ）、V5（U16 で足した行の取り違え）、V8（U5 の残り）、V9（U12 で古くなった見出し）、V11（U16 の漏れ）、V13（U13 の残り）、V14（U20 の片側）、V15（U17 の漏れ）。
- 残す裁定の候補: どれも 3.1 の (1)〜(3) に当たらない。推奨はすべて直すこと。
- 見ていないもの: 無い（88 ファイルを全部読んだ）。「壊しても緑」は読みで確かめたもので、摂動は回していない。V2 の「直しても緑のまま」も読みだけの見込み。
