# ギャップ分析: areka-P0-choice-timeout-directive

> 調査日 2026-10-03・ブランチ `claude/areka-p0-choice-timeout-1b7ecf`（`main` `1ce4c74e` の上に要件のコミット `3b636b6c`）。
> コードは「何の定義か」（関数名・型名・テスト名）で指す。行番号は目安。

## 1. 分析の要約

- **台本を読む側の追加そのものは小さい。** `compile`（`crates/areka-sakura/src/compile.rs`）の `Instruction::GenericCommand` の腕で `name == "set"` かつ `raw_args[0] == "choicetimeout"` を見て「最後の値」を覚え、走査の後に置く選択待ちの区切り（`emit_barrier(.., BarrierKind::WaitForChoice { timeout })`）へ入れるだけで、要件 3.1（選択肢より後ろ）・3.2（最後が勝つ）・3.3（`\e`／`\-` の後ろは数えない）が構造的に満たされる。台本全体の先読みは要らない（`\e`／`\-` で走査を `break` する既存の作りと、区切りを走査の後に置く既存の作りがそのまま効く）。
- **要件 5 は、compile だけでは満たせない（確認済み）。** dola の `TimedSchedule::tick` は、区切りに着いた時点で `WaitForChoice { timeout: Some(d) }` を「`barrier_offset + d` を過ぎていれば飛ばす」。`d <= 0`（`0`・`-1`）なら必ず飛ばし、正の値でも時計が一度に `d` 以上進めば飛ばす。飛ばすと `CuePlayer` は `WaitingForChoice` に入らず、kanade へ `ChoiceWaiting` が届かない。値を `Option<f64>` 1 本で表したまま dola の外だけで直す方法は、正の値について存在しない（§4 の案 B'）。
- **候補は 3 系統。** 案 A＝dola の `TimedSchedule::tick` で `WaitForChoice` の値を「自分で解く期限」として扱わない（1 腕の変更・既存テストの書き換え 0）。案 B＝区切りには今日どおり `None` を書き、値は区切りの外（`CompiledTalk` → `TalkDriver`）で運ぶ（dola 無改変・drive.rs が増える・「値の正本は区切り」の約束 DD-8 と食い違う）。案 C＝`BarrierKind` の形を変える（触る範囲が最大）。
- **触る文書は要件 10 の 2 本より多い。** 台帳の行の `owner` を本 spec へ替えると、台帳の整合テスト（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` の腕 c・f）が `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` 表の書き換えを要求する。状態を `implemented` にすると、証拠（ソースの `// ukadoc: <URL>` 行）と報告 2 本（`report/sakura-script.md`・`report/summary.md`）の作り直しも要る。`doc/choice-cascade-compat.md` の行 5b・5d も着地で古くなる。
- **研究の要る点**: 警告の記録を捕まえるテストの置き場（`areka-sakura` には記録を捕まえる道具の開発時依存が無い＝足すと `Cargo.toml`・`Cargo.lock` に触れる）と、要件 9.4「台本から kanade の期限まで通し」のテストの置き場（kanade の `choice_deadline` は `pub(crate)` で、`schedule/` の下は 0 ファイルの約束）。

## 2. 現状の調査

### 2.1 値の通り道（台本 → kanade の期限）

| 段 | 定義 | 単位・型 | 今日の値 |
|---|---|---|---|
| ① 台本 | `\![set,choicetimeout,N]` | ミリ秒（正典） | 読まれない |
| ② 字句・構文 | `areka_parsers::sakura` の `scan_bracket_args`（lexer.rs）→ `decode_passthrough_bang`（decode.rs） | 文字列 | `Instruction::GenericCommand { name: "set", raw_args: ["choicetimeout", "N"] }` |
| ③ compile | `compile` の `Instruction::GenericCommand` の腕 → `CueCommand::command_carrier("set", raw_args)` を 0 秒の cue として転記。走査の後、選択肢が 1 つでもあれば `emit_barrier(scope, offset, BarrierKind::WaitForChoice { timeout: None })` を最終 offset に 1 個 | 秒（`Option<f64>`） | 常に `None` |
| ④ 再生層 | dola `to_talk_schedule`（sheet.rs）が `Entry::Barrier` に写す → `TimedSchedule::tick` が区切りで止まる → `CuePlayer::tick`（runtime.rs）が `WaitingForChoice` へ | 秒 | — |
| ⑤ 通知 | `TalkDriver::notify_choice_waiting_if_newly_waiting`（drive.rs）が `player.current_barrier()` の `WaitForChoice { timeout }` をそのまま `areka_talk::ChoiceWaiting.timeout_directive_secs` へ。起点は `player.occupancy_horizon()` | 秒 | `None` |
| ⑥ 中継 | areka-ghost `DispatcherState::on_choice_waiting`（dispatcher.rs）が起点を `base_now + round(secs × 1000)` で ms に換算し、指令は無改変で `KanadeMsg::ChoiceWaiting.timeout_directive_secs`（areka-kanade の msg.rs）へ | 秒 | `None` |
| ⑦ 期限 | `steady::on_choice_waiting` → `choice::choice_deadline(display_end, timeout_directive_secs, choice_timeout_default_ms)`：`None`＝既定・`NaN` または `<= 0.0`＝無期限・`> 0.0`＝`display_end + round(v × 1000)` ms（飽和加算） | 秒 → ms | 既定 30,000 ms |

補足（要件の文言との差・情報のみ）: 要件の序文は運び手の通知を「areka-kanade の `msg.rs` にある `ChoiceWaiting`」と書いているが、sakura → ghost の段の型の正本は `areka-talk`（`crates/areka-talk/src/lib.rs` の `pub struct ChoiceWaiting`）で、kanade の `msg.rs` にあるのは次の段の `KanadeMsg::ChoiceWaiting` である。どちらも秒の `timeout_directive_secs` を無改変で運ぶので、要件 8.2（下流の単位は秒のまま）の判断は変わらない。

**`timeout` の値を読む者の全数**（ワークスペース全体を grep）:

- 本体のコードで `WaitForChoice` の `timeout` の**値**を読むのは 2 か所だけ: dola `TimedSchedule::tick`（自分で解く期限として）と areka-sakura `TalkDriver::notify_choice_waiting_if_newly_waiting`（素通しで運ぶ）。
- `{ .. }` で種類だけを見るのは dola `CuePlayer::tick`・areka-emo-text の `emo2_fixture_e2e_test`・各テスト。
- その先は `timeout_directive_secs` として areka-ghost dispatcher（素通し）→ kanade `choice_deadline`（写像）。
- `BarrierKind` を作る本体のコードは compile の 1 か所だけ（`WaitForInput`・`Timeout` を作る本体のコードは 0。テストだけ）。

### 2.2 字句・構文の出力の形（要件 2.3・6・3.3 の前提）

`scan_bracket_args` の規則（カンマ区切り・空白の切り落としなし・`"…"` は 1 引数・`]` の直前が空でも既にカンマで 1 個以上積んでいれば空文字を 1 個積む）と `decode_passthrough_bang`（第 1 引数＝`name`・残り＝`raw_args`）から:

| 台本 | `GenericCommand` |
|---|---|
| `\![set,choicetimeout,500]` | `name="set"`・`raw_args=["choicetimeout","500"]` |
| `\![set,choicetimeout]` | `raw_args=["choicetimeout"]`（時間の欄なし） |
| `\![set,choicetimeout,]` | `raw_args=["choicetimeout",""]`（空の欄 1 個。既存テスト `bracket_arg_splitting_is_unchanged` の `\![a,,c]` → `["a","","c"]` と同じ規則） |
| `\![set,choicetimeout,"500"]` | `raw_args=["choicetimeout","500"]`（引用符は外れる） |
| `\![set,choicetimeout, 500]` | `raw_args=["choicetimeout"," 500"]`（**空白は残る**） |
| `\![set,choicetimeout,500,x]` | `raw_args=["choicetimeout","500","x"]`（余分な欄） |
| `…\e\![set,choicetimeout,500]` | `\e` は `Instruction::End`。compile は `End`／`Quit` で `break` するので、後ろの `GenericCommand` は compile に届かない（パーサの出力には残る） |

`crates/areka-parsers/` の変更は要らない（要件の「0 ファイル」と合う）。

### 2.3 dola の扱い（要件 5 の検証）

`TimedSchedule::tick`（`crates/dola/src/cue/schedule.rs`）の該当箇所:

- **着いたときの判定**: `Entry::Barrier(barrier_offset, kind)` を pop すると、`timeout_dur` を `WaitForInput { timeout } => *timeout`・**`WaitForChoice { timeout } => *timeout`**・`Timeout { duration } => Some(*duration)` で取り、`Some(dur)` なら `timeout_abs = barrier_offset + dur` とし、**`offset >= timeout_abs` なら `continue`（区切りを飛ばして次の entry へ）**。そうでなければ `barrier_timeout_offset = Some(timeout_abs)` を覚えて止まる。
  - pop の条件が `entry_offset <= offset` なので、`dur <= 0.0` のときは `offset >= barrier_offset >= barrier_offset + dur` が常に真＝**必ず飛ばす**（`0`・`-1` を秒にした `0.0`・`-0.001` はここで必ず消える）。
  - 正の `dur` でも、区切りへ着く tick の時刻が `barrier_offset + dur` 以上なら飛ばす。既存の `drive_choice_tests` の補助 `drive_menu_to_barrier` は区切り 0.35 秒に対して Tick(0.0) → Tick(0.5) と進めるので、指定が 150 ms 未満ならこの補助だけで飛ばされる。本番の Tick は ms 刻みで送られる（`DispatcherState::on_tick` の `(now - base) / 1000.0`）ので、`1` のような小さな値はほぼ確実に飛ぶ。
- **止まっている間の判定**: `current_barrier.is_some()` のとき `barrier_timeout_offset` を過ぎれば自分で解く。ただし `CuePlayer::tick` は `WaitingForChoice` の間 `schedule.tick` を呼ばずに早く戻るので、選択待ちではこの経路は**到達しない**（完了 `areka-P0-choice-select-events` の設計 §「タイムアウトの死んだ seam」と `doc/choice-cascade-compat.md` 行 5d が記録済み）。**到達するのは「着いたときの判定」の方で、こちらはどの文書にも記録が無い**——今日は compile が `None` しか書かないので表に出ていないだけである。
- **同じ腕を固定している既存テスト**（`crates/dola/tests/cue/schedule_test.rs`）: `input_barrier_with_timeout_skipped_when_jumped_past`（`WaitForInput { timeout: Some(0.5) }` を 1.0 に置き、tick(2.0) で飛ぶ）・`barrier_timeout_auto_releases`（`WaitForInput { timeout: Some(2.0) }` が止まった後に自分で解く）・`timeout_barrier_skipped_when_already_past`／`timeout_barrier_auto_releases`（`Timeout`）。**`WaitForChoice` に `Some` を入れて tick するテストは 0 本**（`WaitForChoice { timeout: Some(..) }` を書くのは `crates/dola/src/cue/command_tests.rs` の `barrier_kind_three_variants`（作るだけ）と `cue_payload_and_cue_serde_roundtrip`（serde の往復）の 2 本だけ）。
- 区切りが飛ばされると: `CuePlayer` は `Playing` のまま末尾へ進み、占有 horizon で `Completed` → `TalkDone`。`ChoiceWaiting` は送られず、kanade に選択待ちの帳簿ができないので、表示された選択肢を押しても kanade の受領検証（`choice_rejected_no_wait`）で棄てられる。利用者から見ると「選択肢が出るが選べない」。

### 2.4 単位の行き来（要件 1.4・8）

- 変換は compile の 1 か所で `N as f64 / 1000.0`。下流は秒のまま（要件 8.2 と合う）。
- 戻りは kanade の `choice_deadline` の `(v * 1000.0).round() as u64`。整数 N について `round(N / 1000.0 * 1000.0) == N` が成り立つかを Node.js（IEEE 754 倍精度・Rust の `f64` と同じ）で確かめた: **N = 1〜5,000,000 の全数で一致**・2,147,483,647 と 9,007,199,254,740 でも一致・2^53−1 付近で初めて 1 ずれる。理屈でも、除算と乗算の誤差は合わせて相対 2^-52 程度で、N < 2^51（約 7 万年分のミリ秒）なら 0.5 未満に収まる。
- 起点 `display_end` は dispatcher が別に `round(horizon_secs × 1000)` で ms にしてから足すので、期限は「表示の終わり（ms）＋ N」でちょうどになる（端数は起点側の丸めにだけ入る）。
- **大きすぎる値（要件 1.5）**: Rust の `"99999999999999999999".parse::<i64>()` は `Err`（`IntErrorKind::PosOverflow`）になるので、そのまま「読めない値」として扱うと要件 1.5（非常に長い時間切れ）ではなく要件 6.1（既定へ倒す）になってしまう。設計で「数字だけの綴りのあふれ」を区別する必要がある（`IntErrorKind` で見分ける・`u64`／`i128` で読む・飽和させる、など）。秒にした後は `choice_deadline` の飽和キャストで `u64::MAX` に留まるので落ちない。

### 2.5 台帳・文書（要件 10）

- **台帳の行**: `doc/ukadoc-coverage/ledger/sakura-script.toml` の `[entry."ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1"]`。今は `status = "vocabulary-only"`・`owner = "areka-P0-sakura-time-directives"`・`priority = "A1"`・`values = ["交わり"]`。注記は「壊れ方: 黙って壊れる」「ログ: 出る（配送先の 3 つの受け口が担当外の名前として debug! を残す。compile の catch-all は通らない）」「語彙の登記: COMPAT §8 の allowlist の行」。
- **実装済みにすると働く検査**（`crates/ukadoc-survey`）:
  - `check::content` の `check_evidence`: `implemented` の項目は、ソースに正典 URL（`// ukadoc: https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1`）が 1 件以上無いと `ImplementedWithoutEvidence` で赤。置き場は定義の箇所（テストの中は証拠に数えない）。前例は `crates/areka-sakura/src/sysvar.rs` の `%username`・`crates/areka/src/emo2_boot/zorder_cue.rs` の `set,zorder`。
  - `check::freshness`: ドメイン別の報告 `doc/ukadoc-coverage/report/sakura-script.md` は台帳から作り直した本文と全文一致が要る → `cargo run -p ukadoc-survey -- report`。
  - `tests/consistency/documents_checks.rs` の判定 ⑹: 全体の報告 `doc/ukadoc-coverage/report/summary.md` も全文一致が要る → `cargo run -p ukadoc-survey -- report-summary`。
  - **`tests/consistency/spec_checks.rs` の判定 ⑸**: 腕 f「台帳の非空の宛先はすべて `roadmap-draft.md` の `[[spec]]` か `briefing.md` の `[[owner_completed]]` の名前」・腕 c「`[[spec]].owner_count` ＝ 台帳でその名前を宛先に持つ項目の数」・腕 a「`[briefs].count` ＝ `[[spec]]` の行数」。`owner` を本 spec に替えると、`doc/ukadoc-coverage/roadmap-draft.md` に本 spec の `[[spec]]` 行を足し（`bundle = "会話"`＝`linkage.md` の束「会話」に属する・`owner_count = 1`・`stage`・`wave` は前例どおり写す）、`areka-P0-sakura-time-directives` の `owner_count` を 11 → 10 に、`[briefs].count` を 39 → 40 にし、段階ごとの表の「会話」の行の依存する spec の件数も直し、前例（「2026-09-27 の 2 行目の追加」ほか）の型で追加の理由を 1 段落書く必要がある。**要件 10 はこのファイルを挙げていない。**
- **`doc/COMPAT_ARCHITECTURE.md` §8**: 「compile 側時間指令 allowlist」の行（`set,choicetimeout` を逐語で名指し・「M1 は非実導出」）と、選択確定カスケードの行（`doc/choice-cascade-compat.md` へ委譲）がある。要件 10.3・10.4 の 2 点を書き足す場所は特定できた。
- **着地で古くなる他の記述**（要件 10 の外）:
  - `doc/choice-cascade-compat.md` 行 5b「M1 で実際に流れるのは既定値のみ（台本タグの解釈は追跡 spec `areka-P0-sakura-time-directives` の領分）」→ 本 spec の着地で嘘になる。
  - 同 行 5d「`TimedSchedule` のバリア自動解除機構は選択バリアには使用しない……将来 dola 側 seam を生かす場合は本行を改訂」→ 案 A を取るなら「着いたときに飛ばす判定も選択の区切りには効かない」ことを書き足す。案 B なら「区切りの値は常に `None`」の前提を書く。
  - `doc/ukadoc-coverage/briefing-sakura-script.md` の ⑶ 担当の突合表（`areka-P0-sakura-time-directives` が `\![set,choicetimeout,時間]` を「所有」）と ⑵ の消費側の表（`set,choicetimeout` が「消費されない」）。機械の検査（判定 ⑴ は `linkage.md`・`briefing.md`・`roadmap-draft.md` の 3 本だけを見る）には掛からないが、記述は古くなる。
  - `crates/areka-sakura/src/compile.rs` の区切りの注記（「台本からの時間指定は追跡 spec の領分ゆえ本層は値を供給しない」）。
- **完了後の扱い**: `/kiro-complete` で `completed/` に移っても、腕 b は `.kiro/specs/` 直下と `completed/` の和を見るので赤にならない（前例 `areka-P0-ghost-change-name-resolution` は完了後も `[[spec]]` に残っている）。

### 2.6 テストの置き場と道具

- compile の兄弟テスト（`#[path = "…"] mod …;` を compile.rs の末尾に足す形・前例 `compile_arm_tests.rs`・`compile_font_tests.rs`・`compile_sheet_tests.rs`、補助 `compile_test_support.rs` の 1 引数 `compile`・`command_of`）→ 要件 9.1 の表のテストは compile.rs の宣言 1 行＋新しい兄弟ファイルで書ける。
- 要件 9.3（時計を一度に進めても選択待ちに入る）は、再生層まで通す必要がある。前例は `crates/areka-sakura/src/drive_choice_tests.rs`（`spawn_talk` ＋注入 Tick ＋ done ポートで `ChoiceWaiting` を受ける・`choice_waiting_notifies_exactly_once_with_ids_horizon_and_timeout`）。新しい兄弟ファイルにするなら宣言は drive.rs に置くのが自然（compile.rs から drive の補助 `drive_test_support` を使う宣言も書けるが不自然）。dola 単体の判定は `crates/dola/tests/cue/schedule_test.rs` か `runtime_test_barrier_tests.rs` に足せる（案 A のとき）。
- 既存の「指定なしなら `None`」を固定しているテスト（要件 9.6・書き換えずに通す対象）: `compile_arm_tests.rs` の 6 か所・`compile_sheet_tests.rs` の 1 か所・`drive_choice_tests.rs` の `choice_waiting_notifies_exactly_once_with_ids_horizon_and_timeout`（`timeout_directive_secs: None`）。どの案でも台本に指定が無ければ値は `None` のままなので、これらは無改変で緑のはず。
- **記録を捕まえる道具（要件 9.2）**: `areka-sakura` の `Cargo.toml` には開発時依存が 1 つも無い（`log-capture-kit` も `tracing-subscriber` も無い）。足すと `Cargo.lock` の `areka-sakura` の依存の並びが変わる。同じウェーブ C1 の `mcp-server-core` が `Cargo.lock` を触る（roadmap C1 の注記）。代わりに、`log-capture-kit` を既に持つ `areka-ghost`（`crate::test_log_capture::capture`・前例 `sink.rs` の `diagnostic_default_wiring_logs_each_cue_exactly_once_through_broadcast` は `areka_sakura::compile` をテストスレッドで呼んで捕まえている）に置く手もある。
- **要件 9.4（台本から kanade の期限まで通し）**: kanade の `choice_deadline` は `pub(crate)` で、その単体テストは `schedule/choice.rs` の中にある（`choice_deadline_positive_adds_seconds`・`choice_deadline_rounds_fractional_seconds_half_away_from_zero` ほか）。`crates/areka-kanade/src/schedule/` は 0 ファイルの約束なので、ここには足せない。候補: (i) `crates/areka-kanade/tests/kanade/` の外側の檻（`choice_test_timeout_tests.rs` が注入 Tick で `OnChoiceTimeout` の発火時刻を固定している）に `timeout_directive_secs: Some(N / 1000.0)` を入れた檻を足す——ただし同じウェーブ C1 の `status-execution-states` が kanade を触る（`choice_test_choosing_status_tests.rs` など同じ檻群の近く）。(ii) areka-ghost の dispatcher の actor 檻（実 talk を通して `KanadeMsg::ChoiceWaiting` の `display_end` と指令を受ける）で「台本 → kanade の入口」までを固定し、入口から期限までは既存の `choice_deadline` の単体テストに任せる（2 段の合成で「通し」とみなせるかは設計で決める）。

## 3. 要件と資産の対応

| 要件 | 使える資産 | 欠けているもの | 種別 |
|---|---|---|---|
| 1.1・1.3 正の N で時間切れ／選べる | kanade の期限・発火・受領検証（完了 `choice-select-events`） | compile が値を区切りへ入れる処理 | Missing |
| 1.2 `OnChoiceTimeout` の経路 | `fire_choice_timeout_if_due` ほか | なし（読むだけ） | — |
| 1.4 端数の誤差なし | `choice_deadline` の四捨五入・dispatcher の換算 | ms→秒の 1 行（整数 N なら往復で一致・§2.4） | Missing（小） |
| 1.5 大きすぎる N | `choice_deadline` の飽和 | 整数のあふれを「読めない」から区別する読み方 | Missing／判断 |
| 2.1・2.2 `0`・`-1`・負で無期限 | `choice_deadline` の `<= 0.0` → `None` | dola が区切りを必ず飛ばす（§2.3） | **Constraint** |
| 2.3・2.4 省略・空欄・指定なしで既定 | `None` ＝既定（DD-8） | 空欄を省略と同じに読む処理 | Missing（小） |
| 3.1〜3.4 位置・回数・終わりのタグ・選択肢なし | compile の `break` と走査後の区切り | 腕の中で「最後の値」を覚える 1 変数 | Missing（小） |
| 4.1・4.2 その台本だけ | compile は純関数・talk ごとに 1 回 | なし（構造で満たす・テストで固定） | — |
| 5.1〜5.3 値に依らず選択待ち | `CuePlayer` の待機・drive の通知 | dola の「着いたら飛ばす」判定を選択の区切りに効かせない手立て | **Constraint**（§5） |
| 5.4 他の区切りは今日のまま | `WaitForInput`・`Timeout` の既存テスト | 変更を `WaitForChoice` の腕に閉じる | Constraint |
| 6.1〜6.3 読めない値の警告 | `tracing`（compile は catch-all で既に `debug!` を出す） | 警告の 1 行・talk_id を載せるかの判断 | Missing／判断 |
| 7.1〜7.3 転記と時刻の並びを変えない | 汎用キャリアの転記・`emo2_boot/consumer_ledger.rs` に `("set","choicetimeout")` の登録は無い | 転記を残したまま値も読む（腕の中で両方） | — |
| 8.1・8.2 変換 1 か所 | 下流は秒で統一済み | compile の 1 行 | — |
| 9.1 区切りの値のテスト | compile の兄弟テストの型 | 新しい兄弟ファイル | Missing |
| 9.2 警告の記録のテスト | `log-capture-kit`（sakura には無い） | 道具の置き場 | **Unknown** |
| 9.3 時計を飛ばしても待つテスト | `drive_choice_tests` の補助 | 新しいテスト（＋案 A なら dola 側） | Missing |
| 9.4 台本から期限まで通し | kanade の外側の檻・ghost の actor 檻 | 置き場（`schedule/` は 0 ファイル） | **Unknown** |
| 10.1〜10.4 台帳と §8 | 台帳の行・§8 の 2 行・`ukadoc-survey` の生成器 | `roadmap-draft.md` ほか（§2.5） | Constraint（範囲の拡大） |

## 4. 要件 5 をどこで満たすか（案の比較）

### 案 A: dola で、選択の区切りの値を「自分で解く期限」として扱わない

- **変更**: `TimedSchedule::tick` の `timeout_dur` の腕を `BarrierKind::WaitForChoice { .. } => None` にする（`WaitForInput`・`Timeout` の腕はそのまま）。これで「着いたら飛ばす」も「止まっている間に自分で解く」（`barrier_timeout_offset`）も選択の区切りには効かなくなる。区切りの値は `current_barrier()` から今日どおり読めるので、drive.rs の通知は無改変で `Some(v)` を運ぶ。
- **触るファイル**: `crates/dola/src/cue/schedule.rs`（腕 1 行＋注記）、必要なら `crates/dola/src/cue/command.rs` の `WaitForChoice` の注記（「値は上位層へ運ぶ指令で、再生層は解かない」）と `runtime.rs` の `occupancy_horizon` 周りの注記、dola のテスト（`crates/dola/tests/cue/schedule_test.rs` か `runtime_test_barrier_tests.rs` に新しい檻）、compile.rs と兄弟テスト、文書（§2.5）。
- **既存テストの書き換え**: **0 本**（§2.3 のとおり、`WaitForChoice` に `Some` を入れて tick する既存テストは無い。`WaitForInput`・`Timeout` の檻は腕を触らないので緑のまま）。
- **良い点**: 値の正本が区切り 1 か所のまま（完了 `choice-select-events` の DD-8・`areka_talk::ChoiceWaiting` の注記「バリアのタイムアウト指令」と合う）。`doc/choice-cascade-compat.md` 行 5d「タイムアウトの権威は kanade に一本化」を構造で強制できる。早送り（`talk-fast-forward`）が時計を飛ばしても壊れない（要件の Adjacent expectations と合う）。
- **気になる点**: brief の「触るソースは compile.rs と兄弟テストだけ」を dola へ広げる（開発者の了承が要る）。`WaitForInput` と非対称になる（理由を注記で残す）。`dola` は `publish = true` で crates.io に名前の確保だけ（0.0.1）があり、本格の公開は C2 の `crates-io-publish` から——公開前なので後方互換の心配は小さいが、意味の変更であることは注記に残す。ウェーブ C1 の他の 6 本は dola を触らない（roadmap C1 の各「触る場所」）ので衝突は 0。

### 案 B: 区切りには今日どおり `None` を書き、値は区切りの外で運ぶ

- **変更**: `CompiledTalk` に欄（例: 選択の時間の指令 `Option<f64>`）を足し、drive.rs の `on_start` → `TalkPhase::Armed`／`Driving` に持ち回り、`notify_choice_waiting_if_newly_waiting` が区切りの値ではなくその欄を運ぶ。
- **触るファイル**: compile.rs（欄の追加）、drive.rs（相の 2 つの形と通知）、sakura のテスト。dola は無改変。`CompiledTalk` を分解して受ける者は他に無い（grep で確認・`areka/src/emo2_boot/talk_lifecycle_tests.rs` は戻り値を受けるだけ）。
- **既存テストの書き換え**: 0 本の見込み（区切りの値も通知の値も、指定なしなら `None` のまま）。ただし drive の相の形を変えるので、`drive_*_tests` が相を直接作っていないかは設計で再確認。
- **気になる点**: **要件 9.1 の文言（「選択待ちの区切りに入る時間の値が期待どおり」）と合わない**——区切りの値は常に `None` で、期待どおりになるのは別の欄になる。正本が 2 か所（区切りは `None` と言い、通知は `Some(v)` と言う）になり、`areka_talk::ChoiceWaiting` の注記「バリアのタイムアウト指令」・DD-8 の「入口は dola の `WaitForChoice{timeout}`」が嘘になる。dola の「着いたら飛ばす」判定は残るので、将来だれかが区切りに値を入れれば同じ穴が開く。

### 案 B': 値の綴りだけを工夫する（`Option<f64>` のまま・dola も drive も無改変）

- 「時間切れなし」を `Some(f64::NAN)` にすると、dola では `offset >= barrier + NaN` が偽で止まり続け、kanade は `NaN` を無期限へ畳む（`choice_deadline` が明示している）。`Some(f64::INFINITY)` でも dola は止まり続け、kanade は `u64::MAX` の期限（実質無期限だが「無期限」ではなく「非常に長い期限」）になる。
- **正の N はこの方法では救えない**（時計が一度に N 以上進めば飛ぶ）ので、要件 5.2 を満たせない。さらに `CueSheet` は serde で書き出せる形で、`serde_json` は `NaN`・`INFINITY` を `null` にするので往復で `None`（既定）に化ける。**成立しない**と判断する（記録のため挙げる）。

### 案 C: `BarrierKind` の形を変える（別の欄を足す・名前を変える）

- 例: `WaitForChoice { timeout, directive }` のように「自分で解く期限」と「上位へ運ぶ指令」を分ける。
- **触るファイル**: dola の `command.rs`・`schedule.rs`、`WaitForChoice { timeout: None }` を字面で書いている 13 か所（compile.rs 1・`compile_arm_tests.rs` 6・`compile_sheet_tests.rs` 1・dola のテスト 5）＋ `command_tests.rs` の 2 か所、drive.rs、serde の形（`command_tests.rs` の JSON の期待値）。
- **既存テストの書き換え**: 10 本超（要件 9.6「書き換えずに通す」に反する）。触る範囲が最大で、得るものは案 A と同じ。

### まとめ

| | dola | drive.rs | 正本 | 既存テストの書き換え | 要件 9.1 の文言 |
|---|---|---|---|---|---|
| A | 1 腕 | 無改変 | 区切り 1 か所 | 0 | 合う |
| B | 無改変 | 増える | 2 か所 | 0（見込み） | 合わない |
| B' | 無改変 | 無改変 | 区切り | 0 | 正の値で要件 5.2 を満たせない |
| C | 形を変える | 増える | 区切り | 10 本超 | 合う |

## 5. 規模とリスク

- **規模: S**（4〜6 タスク）。compile の読み取りと焼き込み・dola の 1 腕（案 A）・テスト 3 群（compile／drive／dola）・文書と台帳の連鎖。brief の見立て（XS〜S・3〜5）より文書の連鎖のぶん少し増える。
- **リスク: 低〜中**。コードの変更は既存の型に乗る小さなもの。中にしているのは、(1) dola の意味の変更を brief の範囲外へ広げる判断が要ること、(2) 台帳の連鎖（`roadmap-draft.md`・報告 2 本・`summary.md`）が同じウェーブの他 spec と共有ファイルになりうること（`summary.md` は台帳 4 本から作り直すので、他の spec が別の台帳を動かして先に着地すると、作り直しが必要になる。手で数を直さず生成器で作り直すこと）。

## 6. 設計へ持ち越す判断事項

1. **要件 5 をどこで満たすか**（§4）。案 A（dola の 1 腕・既存テストの書き換え 0・正本 1 か所）か案 B（dola 無改変・drive.rs 改変・正本 2 か所・要件 9.1 の文言と食い違う）。案 A なら brief の「触るソースは compile.rs と兄弟テストだけ」を `crates/dola/src/cue/schedule.rs`（＋注記・dola のテスト）へ広げる了承が要る。
2. **案 A を取る場合の範囲**: 「着いたら飛ばす」だけを止めるか、「止まっている間に自分で解く」（`barrier_timeout_offset`）も合わせて止めるか。後者は `CuePlayer` からは到達しないが、`TimedSchedule` を直接使う者には効く。1 腕の変更なら両方が同時に止まる。
3. **台帳の宛先を本 spec へ替えることに伴う文書の拡大**: 要件 10.1（引受先を本 spec に替える）を満たすと、整合テストの判定 ⑸ が `doc/ukadoc-coverage/roadmap-draft.md`（`[[spec]]` 行の追加・`areka-P0-sakura-time-directives` の `owner_count` 11 → 10・`[briefs].count` 39 → 40・段階ごとの表・追加の理由の段落）を要求する。報告 2 本（`report/sakura-script.md`・`report/summary.md`）の作り直しと、ソースへの正典 URL の 1 行（証拠）も要る。これらを要件 10 に書き足すか、設計の「触る文書」にだけ書くか。
4. **着地で古くなる記述をどこまで直すか**: `doc/choice-cascade-compat.md` 行 5b・5d、`doc/ukadoc-coverage/briefing-sakura-script.md` の ⑵ ⑶ の行、compile.rs の区切りの注記。要件 10 には無い。
5. **時間の欄の読み方の細目**（正典は黙っている。§8 の裁量の 1 行に入れるかも含めて）:
   - 前後の空白（`\![set,choicetimeout, 500]`）を切り落とすか、読めない値にするか。
   - 先頭の `+`（`+500`。Rust の `i64` の読み取りは受け付ける）・`-0`・全角数字を許すか。
   - 余分な欄（`\![set,choicetimeout,500,x]`）を無視して 2 欄目だけを読むか、読めない値にするか。
   - 大きすぎる正の整数（要件 1.5）・小さすぎる負の整数を、読めない値（`IntErrorKind::PosOverflow`／`NegOverflow`）から区別して、それぞれ「非常に長い」「時間切れなし」に倒す方法。
6. **「時間切れなし」の区切りの値をそろえるか**: `0` → `Some(0.0)`、`-1` → `Some(-0.001)`、`-5` → `Some(-0.005)` のように変換をそのまま通すか、すべて `Some(0.0)`（または `Some(-1.0)`）にそろえるか。kanade の写像はどれでも同じ結果。要件 9.1 のテストの期待値と、kanade の記録（`choice_waiting_established` の `timeout_directive_secs`）に出る値が変わる。
7. **読めない値の警告の出し方**:
   - compile は talk_id を知らない（純関数）。警告に talk_id を載せたいなら、compile は診断を値として返し、drive.rs が talk_id 付きで記録する形になる（drive.rs が増える）。compile の中で `warn!` を出すだけなら drive.rs は無改変（compile は catch-all で既に `debug!` を出しているので前例はある）。
   - 選択肢の無い台本に読めない値があるとき（要件 3.4「誤りとしても扱わない」と要件 6.1「警告を残す」の交わり）に警告を出すか。
   - 読めない値の後に読める値があるとき（要件 6.2）にも、読めない方の警告を出すか。
8. **警告の記録を捕まえるテストの置き場**（要件 9.2）: `areka-sakura` に `log-capture-kit` の開発時依存を足す（`Cargo.toml`・`Cargo.lock` に触れる。C1 の `mcp-server-core` も `Cargo.lock` を触る）か、既に道具を持つ `areka-ghost` の兄弟テストに置くか。
9. **要件 9.4 の「通し」のテストの置き場**: kanade の `schedule/` は 0 ファイルの約束で `choice_deadline` は `pub(crate)`。kanade の外側の檻（`crates/areka-kanade/tests/kanade/`・C1 の `status-execution-states` と近い）に置くか、areka-ghost の actor 檻で「台本 → kanade の入口」を固定して既存の `choice_deadline` の単体テストと合わせて通しとみなすか。
10. **要件 9.3 のテストの宣言の置き場**: drive の補助（`drive_test_support`）を使う新しい兄弟テストは drive.rs に `#[path]` の宣言を 1 行足すのが自然。brief の「compile.rs とその兄弟テスト」を drive.rs の宣言 1 行へ広げてよいか（案 B なら drive.rs はどのみち変わる）。

## 7. Research Needed（設計で確かめること）

- 案 A の場合、dola の `TimedSchedule` を `CuePlayer` 以外から直接使う者が本当に居ないこと（今日の grep では `crates/dola/src/cue/runtime.rs` の `CuePlayer` と `sheet.rs` の `to_talk_schedule` とテストだけ）と、dola の README・設計文書に `WaitForChoice` の自己解除を約束した記述が無いこと（完了 `wintf-P0-cue-system` の要件は `WaitForInput` の時間切れだけを約束している）。
- `roadmap-draft.md` の `[[spec]]` 行の `stage` と `wave` に何を写すか（前例は「束の段階」と「正本ロードマップのウェーブ」＝「会話」の束の段階・`C1-②`）。
- 同じウェーブ C1 の他の spec が台帳（4 本のどれか）を動かすか。動かすなら `report/summary.md` を後から着地する側が作り直す（手で数を直さない）。

## 8. 要件ディスカッションでの扱い（2026-10-03）

- **§6 の 1（要件 5 の満たし方）**: 境界は要件側で決着した——dola の選択待ちの区切りの扱いに限って触ってよい（開発者の方針「根本が境界の外でも並走が無ければ境界を広げて直す」。C1 の他の 6 本は dola に触らない）。案 A・B のどちらを取るかは設計で決める（本分析の推しは案 A）。
- **§6 の 3・4（台帳の連鎖と古くなる記述）**: 要件 10.5・10.6 として要件へ書き足した。
- **§6 の 2・5〜10**: 設計（`/kiro-spec-design`）で決める。§6 の 8 は、同じウェーブ C1 の `mcp-server-core` が `Cargo.lock` を触るので、`Cargo.toml`・`Cargo.lock` に触れない置き場（例: 道具を既に持つ `areka-ghost`）を先に検討する。
- **複数回の指定（brief の議題）**: 「最後に書かれたものが勝つ」で要件 3.2 に確定した（台本は頭から順に実行されるので、表示の終わりの時点で効いているのは最後の指定、という読み）。

## 9. 設計フェーズの調査と決定（2026-10-03・`/kiro-spec-design`）

### 9.1 要約

- **Feature**: `areka-P0-choice-timeout-directive`
- **Discovery Scope**: Extension（既存の値の通り道に乗る拡張・light discovery）。外部の依存・新しいライブラリは無し。
- **Key Findings**:
  - `TimedSchedule` を `CuePlayer`（`crates/dola/src/cue/runtime.rs`）と `to_talk_schedule`（`sheet.rs`）以外から直接使う本番コードは無い（`crates/dola/src/runtime/facade.rs`・`areka-sakura/src/lib.rs`・`dola/src/cue/mod.rs` の言及は doc 注記だけ）。完了 `wintf-P0-cue-system` の要件は `WaitForInput` の時間切れだけを約束している（要件 4「WaitForInput に timeout が設定されており期限を超過した時」）。→ 案 A の前提が成り立つ。
  - `ukadoc-survey` の証拠の走査（`evidence/candidates.rs`）は `_tests.rs`・`/tests/`・`#[cfg(test)]` の塊を候補から外す。証拠の `// ukadoc:` 行は `compile.rs` 本体の定義の箇所に置く。
  - `ukadoc-survey` の副手続きの名前は `report`・`report-summary`・`check`（`cli/cli_tests.rs` が固定）。
  - areka-ghost の `sink.rs` のテストが `areka_sakura::compile` をテストスレッドで呼んで `test_log_capture::capture` で捕まえている前例があり、`areka-sakura` に開発時依存を足さずに警告の檻を張れる。
  - 同じウェーブ C1 の他の 6 本は `crates/areka-ghost/src/dispatcher.rs`・`crates/areka-sakura/src/`・`crates/dola/` に触らない（roadmap C1 の各「触る場所」）。`status-execution-states` は `crates/areka-kanade/src/status.rs` と届け口を触るが、`tests/kanade/choice_test.rs` の接続宣言 1 行の追加が重なりうる（重なっても隣接行の追加で解ける）。

### 9.2 Design Decisions

#### Decision: 要件 5 は dola の 1 腕で満たす（案 A）
- **Context**: `TimedSchedule::tick` の `timeout_dur` が `WaitForChoice { timeout }` の値を「着いたら飛ばす／止まっている間に自分で解く」期限として扱うため、台本の値をそのまま区切りへ入れると `0`・`-1` は必ず飛ばされ、正の値も時計の進み方で飛ばされる（§2.3）。
- **Alternatives Considered**: 案 A（dola の 1 腕）・案 B（区切りには `None`・値は `CompiledTalk` の別欄で drive.rs まで）・案 B'（NaN／∞ の綴り・不成立）・案 C（`BarrierKind` の形を変える）。
- **Selected Approach**: `timeout_dur` の腕を `BarrierKind::WaitForChoice { .. } => None` にする。「着いたら飛ばす」と「止まっている間に自分で解く」の両方が同時に止まる（§6-2 は「両方」）。
- **Rationale**: 1 腕の変更・既存テストの書き換え 0・値の正本が区切り 1 か所のまま（DD-8・`areka_talk::ChoiceWaiting` の注記と合う）・`doc/choice-cascade-compat.md` 行 5d「権威は kanade に一本化」を構造で強制できる・早送りで時計が飛んでも壊れない。境界の拡大は要件ディスカッションで了承済み（§8）。
- **Trade-offs**: `WaitForInput` と非対称になる（注記で理由を残す）。`dola` は公開前（0.0.1 の名前確保のみ）なので後方互換の負担は小さいが、意味の変更として `command.rs` の doc と行 5d に残す。
- **Follow-up**: `schedule_test.rs` に選択の区切りの檻（`Some(0.0)`・`Some(-0.001)`・`Some(0.5)` を一度に越える・止まっている間に解かない）。

#### Decision: 読み取りは `GenericCommand` の腕の中・純関数 `parse_choice_timeout` に閉じる
- **Context**: 台本全体の先読み（brief の想定）は要らない——`End`／`Quit` の `break` と「区切りは走査後に置く」既存の作りで 3.1・3.3 が成り立つ。
- **Selected Approach**: 腕の先頭の転記は無改変。その後に `name == "set"` かつ `raw_args[0] == "choicetimeout"` のときだけ `parse_choice_timeout(raw_args)` を呼び、局所変数 `last_choice_timeout: Option<f64>` を上書きする。走査後の `emit_barrier` にその値を入れる。
- **Rationale**: 変換 1 か所（8.1）・台本の文字列から直に檻を張れる（9.1）・後続の spec が同じ腕を触っても「転記 → 読み取り」の 2 段が崩れにくい。
- **Trade-offs**: `compile` は talk_id を知らないので警告に talk_id は載らない（下記）。

#### Decision: 時間の欄の読み方の細目（§6-5・§6-6）
- 欄なし・空文字 → 既定（`None`）。前後の空白は `trim` してから読む。`+500` は受ける。余分な欄（3 欄目以降）は見ない。`i64` で読み、`PosOverflow` → `i64::MAX` ms・`NegOverflow` → `i64::MIN` ms へ飽和（1.5・2.2。`u64`／`i128` で読む案は 40 桁超でまた同じ問題になるので飽和を選ぶ）。それ以外の `Err`（`InvalidDigit`・小数・単位付き・全角数字）→ 読めない値。
- 値の正規化はしない（`-1` → `Some(-0.001)`・`-5` → `Some(-0.005)` のまま）。kanade の写像はどれでも同じで、記録に台本の値がそのまま残る方が追いやすい。`Some(v)` の `v` は常に有限（serde で `null` に化ける値を区切りへ入れない）。
- 裁量の記録（10.3）: 複数回は最後が勝つ・`-1` 以外の負の値も時間切れなし・空欄は省略と同じ・読めない値は既定として扱い警告・終わりのタグより後ろは数えない・前後の空白は切り落とす・余分な欄は無視・桁あふれは飽和。

#### Decision: 警告は `compile` の中で `warn!`・診断を値で返さない（§6-7）
- **Selected Approach**: `tracing::warn!(event = "choice_timeout_unreadable", raw = %欄, "[compile] …")`。読めない指定 1 回につき 1 行。選択肢の有無で出し分けない（走査中は分からない・読めない綴りは作者が直すべきもの。要件 3.4「誤りとしても扱わない」は「失敗にしない・振る舞いを変えない」の意味で、記録の 1 行はこれに反しない）。読めない指定の後に読める指定があっても読めない方の警告は出る（6.2 は値の決め方の話で、記録を消す理由にはならない）。
- **Rationale**: talk_id を載せるために `compile` の署名を変え drive.rs を広げる価値が無い。drive.rs の既存 info（`choice barrier reached; notifying ChoiceWaiting`・talk_id 付き）と並べれば talk は特定できる。

#### Decision: テストの置き場（§6-8・§6-9・§6-10）
- 9.2（警告の捕捉）と 9.4 の前半（台本 → `KanadeMsg::ChoiceWaiting`）は areka-ghost の新しい兄弟テスト `dispatcher_choice_timeout_tests.rs`（接続宣言は `dispatcher.rs` に 1 行）。`log-capture-kit` と `spawn_dispatcher` を既に持ち、`Cargo.toml`・`Cargo.lock` に触れない。
- 9.4 の後半（入口 → 期限）は kanade の外側の檻 `tests/kanade/choice_test_timeout_directive_tests.rs`（接続宣言は `choice_test.rs` に 1 行）。`schedule/` は 0 ファイルのまま。2 つの檻が同じ `N = 1234`・`Some(1.234)` を境の値として共有し、合わせて「通し」とみなす。
- 9.3 は areka-sakura の新しい兄弟テスト `drive_choice_timeout_tests.rs`（接続宣言は `drive.rs` に 1 行・本体の改変 0）と dola の `schedule_test.rs` への追記。
- 9.1 は `compile_choice_timeout_tests.rs`。`barrier_of` は `compile_arm_tests.rs` から `compile_test_support.rs` へ移す（共有ヘルパの集約規約）。

### 9.3 Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|---|---|---|---|---|
| 案 A（採用） | dola の 1 腕で選択の区切りの値を解かない | 正本 1 か所・既存テスト書き換え 0・早送りに強い | `WaitForInput` と非対称・dola の意味の変更 | 境界の拡大は要件で了承済み |
| 案 B | 値を区切りの外（`CompiledTalk` の欄）で運ぶ | dola 無改変 | 正本 2 か所・DD-8 と注記が嘘になる・9.1 の文言と合わない・穴が残る | 却下 |
| 案 B' | NaN／∞ の綴り | 無改変 | 正の値を救えない・serde で `null` に化ける | 不成立 |
| 案 C | `BarrierKind` の形を変える | 意味が型で分かれる | 既存テスト 10 本超の書き換え（9.6 に反する） | 却下 |

### 9.4 Risks & Mitigations

- `status-execution-states`（C1）が `tests/kanade/choice_test.rs` に接続宣言を足すと隣接行で重なる — 先に着地した側の末尾へ 1 行足し直す（本体の衝突は無い）。
- 同じウェーブの他 spec が台帳 4 本のどれかを動かすと `report/summary.md` が古くなる — 後から着地する側が生成器で作り直す（手で数を直さない）。
- `roadmap-draft.md` の件数（`owner_count`・`[briefs].count`・段階ごとの表）を引き算で書くと整合テストが赤になる — 前例どおり数え直した値を書く。
- `i64::MAX as f64 / 1000.0` の秒を kanade が `(v*1000).round() as u64` で戻すと 9.2e18（`u64::MAX` 未満）になり、`saturating_add` で落ちない（`choice_deadline_saturates_on_extreme_values` が既に極端値を固定している）。

### 9.5 References
- ukadoc `\![set,choicetimeout,時間]`: `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1`
- 完了 `areka-P0-choice-select-events` design（DD-8・F3）・`doc/choice-cascade-compat.md` 行 5a〜5d・10
- 完了 `wintf-P0-cue-system` requirements（要件 4: `WaitForInput` の時間切れだけを約束）
- `.kiro/steering/structure.md`「Unit Tests」（兄弟テストの接続規約・共有ヘルパの集約）・`logging.md`（`log-capture-kit` を通す）

## 10. 設計ディスカッションでの扱い（2026-10-03）

設計レビュー（`design-validation.md`・判定 GO）の 3 点はいずれも勝者が明白だったので、自明な修正として design.md へ反映し、開発者へ伺う議題は 0 件とした。

- **指摘 1（要件 9.4 の満たし方）**: ghost のテストと kanade の外側のテストの 2 本の合成で満たすと確定。境の値 `1234` を共有し、両方の doc 注記に対になるテストの名前を書く。1 本で貫く案は `schedule/` 0 ファイル・`choice_deadline` が `pub(crate)` のため取れない。
- **指摘 2（要件 3.4 と警告）**: 3.4「誤りとしても扱わない」は「失敗にしない・台本の振る舞いを変えない」の意味で、読めない値の警告は選択肢の有無に依らず出す（6.1 は無条件）。警告文にタグの綴り（バックスラッシュ＋感嘆符）は書かない（Rust の文字列で不正なエスケープになる）。
- **指摘 3（文面と実物のずれ）**: `spawn_dispatcher` は `dispatcher.rs` の `pub fn`・kanade の `establish_choice_wait` は使わず注入列を自前で書く・`briefing-sakura-script.md` の「語彙の登記」の表の行も見る・報告 2 本は後から着地する側が作り直す、の 4 点を design.md へ転記した。