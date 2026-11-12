# audit-dola かけ直し 21・3 回目（ponytail-audit・範囲 `crates/dola/`）

**この範囲がしていること:** dola は 2 つのエンジンを持つ。1 つは宣言したアニメ（変数・遷移・キーフレーム・ストーリーボード）を検証してコンパイルし、`DolaRuntime` が時刻を受けて値の差分を購読者へ配るもの。もう 1 つは cue（`CueSheet` → `TimedSchedule` → `CuePlayer`）で、さくらスクリプトの台本を絶対時刻で全部の受け手へ配るもの。

**想定した負荷:** 1・2 回目と同じ。1 つのプロセスの UI スレッドが 60fps 前後で `tick` を呼ぶ。インスタンスは十数個、cue は台本 1 枚で数百。文書はゴーストの作者が書く（＝作者の書き損じや極端な値が入りうる入口）。PC のスリープの後などに、1 回の tick で時刻が大きく飛ぶことがある（`MIN_LOOP_DURATION` の注がこの場面を想定している）。

**対象:** HEAD `d2e6befe`。2 回目の 5 件の直し（`f4f5d247`・`d2e6befe`）と tasks.md の「## Implementation Notes」（「21（」で始まる 3 行を含む）を読んだうえで、新しい指摘と、直しが持ち込んだ欠陥だけを挙げる。

**数えなかったもの:**
- 要件 11.3 の残す 3 点: #4 の副作用（最後の区切りより後のトリガーは発火しない・R2-03 の後は周回ごとに成り立ち、コマの刻みに依らない）・`WaitForInput` と `resolve_click`（`\x` の受け皿）・機能 `yaml` と `serde_yaml`。
- 開発者の決めとして残る 3 点: (a) ベジェの制御点を「有限」より狭めるか・(b) 未来の時刻の `start()` が、ぶつかる古いインスタンスを今すぐ終わらせるか・(c) `MIN_LOOP_DURATION` を有限のループにも当てるか。

**2 回目の直しの確かめ:** 5 件とも、指摘した形どおりに直っている（下の「指摘にしなかったもの」）。ただし R2-03 の直し（周回を 1 周ずつ進めて、飛ばした周回のトリガーも起こす）は、「1 コマずつ回したときと同じ結果」に届いていない。トリガーが起きる順が時刻の順でないので、2 つの親の子がぶつかると結果がコマの刻みで変わる（R3-01）。`finish` の期限も同じ種類の取りこぼしを持つ（R3-02・前からある）。1 周ずつ子を起こすので、時刻が大きく飛んだ tick が重くなり、`process_loops` の注の「軽い反復」が事実と合わなくなった（R3-03）。

---

## Must fix（0 件）

なし。

---

## Should fix（2 件）

### R3-01 1 回の tick の中で、トリガーが時刻の順でなく親の group_id の順（と周回の順）に起きるので、ぶつかる子の勝ち負けがコマの刻みで変わる（R2-03 の直しが届いていない所）
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/facade.rs:404-496`（`fire_due_triggers`・`pending` を親の group_id の順に積んで、その順に `start_internal` を呼ぶ）・`:334-364`（`update_internal` の Step 1.5 と Step 2 の繰り返し）・`src/runtime/loop_controller/mod.rs:143-146`（トリガーを持つインスタンスは 1 周で戻る）
- **何が悪いか:** 子を起こす `start_internal` は、ぶつかる古いインスタンスを方針（Cancel など）で終わらせる。なので、子を起こす順が結果を決める。60fps なら子は発火の時刻の順に起きる。ところが 1 回の tick に発火がいくつも入ると、`fire_due_triggers` は親の group_id の順に起こす。周回を飛ばしたときは、R2-03 の直しで「1 周ずつ」進むが、これは周回の数の順で、時刻の順ではない。
  - 例 1（周回なし）: 親 p1（group 1）が 0.9 秒に子 A（x を 10→11・5 秒）を、親 p2（group 2）が 0.1 秒に子 B（x を 20→21・5 秒）を起こす。A・B とも Cancel。60fps で 0〜1 秒を回すと、0.1 秒に B、0.9 秒に A が起き、A が B を終わらせる。1.0 秒の x は **10.02**（A の値）。tick を 0.0 の次に 1.0 にすると、A が先、B が後に起き、B が A を終わらせる。x は **20.18**（B の値）。手元でこの台本を組んで回し、2 つの値を確かめた。
  - 例 2（周回あり）: 親 P1 が 1 秒 × 3 周で 0.9 秒に A を、親 P2 が 0.25 秒 × 12 周で 0 秒に B を起こす。60fps なら最後に起きるのは 2.9 秒の A。tick を 0.0 → 3.5 と飛ばすと、P1 の 3 周目の A（2.9 秒）は繰り返しの 3 回目に、P2 の 12 周目の B（2.75 秒）は 12 回目に起きる。後に起きた B が A を終わらせ、時刻の上では先の B が勝つ。
  - 同じ形で、親の周回の終わりや自然の終わりも時刻の順に並ばない。例: Never の X（x を 0〜1.2 秒）と、2.5 秒に x の子を起こす親。60fps なら X は 1.2 秒に終わり、子は起きる。tick を 0.0 → 3.0 にすると、トリガーの集め（Step 1.5）が X の終わり（Step 2）より先なので、子は X とぶつかって `TriggerResult::Error`（Conflict）になる。
  - 同じ時刻のトリガーは親の group_id の順（`simultaneous_triggers_resolve_in_group_order` の決め）で、ここは変えなくてよい。今の本番（wintf）はトリガーを使う文書を持たないが、使えば当たる。
- **直し:** 1 回の tick の中の出来事を時刻の順に処理する。`update_internal` の繰り返しの頭で、「次の出来事の時刻」＝ Playing のインスタンスの、まだ起きていないトリガーの壁時計の時刻（周回の長さの内のもの）と `end_time` の最小を求める。それが `current_time` より後なら抜ける。そうでなければ、その時刻を `current_time` の代わりに `process_triggers` と Step 2 へ渡す。同じ時刻のトリガーは今どおり group_id の順。`process_loops` の「トリガーを持つなら 1 周で戻る」はそのままでよい（時刻で 1 つずつ進むので）。檻 2 本: 例 1 を 60fps と 0.0 → 1.0 の 2 通りで回し、どちらも x が 10.02。例 2 を 60fps と 0.0 → 3.5 で回し、勝つ子が同じ。
- **触る所:** 本番の振る舞い（トリガーで起こす子の順・ぶつかったときの勝ち負け）。

### R3-02 `finish` の期限を同じ tick で越えると、期限より前に起きるはずだったトリガーが起きない（R2-03 と同じ種類・前からある）
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/facade.rs:328-332`（`update_internal` の Step 1 が、トリガーを集める前に期限の来たインスタンスを Conclude する）
- **何が悪いか:** Step 1 は tick の頭で、期限を過ぎたインスタンスを全部終わらせる。トリガーを集めるのはその後なので、期限より前の時刻のトリガーでも、同じ tick で期限を越えると集められない。例: 親が 1 秒 × 3 周で、0.5 秒に子を起こす。`finish(親, 2.0)` を呼ぶ。60fps で回すと、子は 0.5 秒と 1.5 秒の **2 回**起き、2.0 秒に親が終わる。tick を 0.0 の次に 3.5 にすると、Step 1 が親をすぐ終わらせ、子は **0 回**。手元でこの台本を回し、2 と 0 を確かめた。1 コマの中でも起きる: 発火 0.5 秒・期限 0.505 秒で、tick が 0.49 → 0.51 なら、0.5 秒の子は起きない。要件 11.3 の残す項目（周回の長さより後ろのトリガー）とは別で、ここは期限より前のトリガー。
- **直し:** 期限も R3-01 の「出来事」の 1 つとして時刻の順に並べる（期限の時刻を次の出来事の時刻の候補に入れ、その時刻に来たら Conclude する）。R3-01 を直さないなら、最小の形はこれ: Step 1 を繰り返しの後ろへ移し、期限のあるインスタンスは `fire_due_triggers` と Step 2 で `current_time` の代わりに `min(current_time, 期限)` を使う。檻 1 本: 上の台本を tick 0.0 → 3.5 で回し、`triggered` に子が 2 つ。
- **触る所:** 本番の振る舞い（期限で終えるインスタンスのトリガー）。

---

## Nice to have（1 件）

### R3-03 トリガーを持つループは 1 周ごとに子を起こすので、時刻が大きく飛んだ tick が重い。`process_loops` の注の「軽い反復」が事実と合わない（R2-03 の直しが持ち込んだ）
- **重さ:** nice-to-have
- **場所:** `crates/dola/src/runtime/loop_controller/mod.rs:121-127`（「周回は最短 0.1s なので、8 時間の休止の後でも約 29 万回の軽い反復で済む」）・`src/runtime/types.rs:83-85`（`MIN_LOOP_DURATION` の「極端な周回数処理を防止する」）
- **何が悪いか:** R2-03 の前は、飛ばした周回は数を足すだけで、トリガーは起きなかった。今は 1 周ごとに `start_internal`（子のストーリーボードのコンパイル・競合の解決・表への挿入）と、その子の Conclude を回し、`TriggerResult` を 1 件ずつ積む。手元で release の build で測った（無限のループの親が周回の半ばで子を起こす台本・tick を 0.0 の次に大きく飛ばす）: 1 秒の周回で 1 時間 → 約 10 ms・3,600 件。1 秒の周回で 8 時間 → 約 69 ms・28,800 件。0.1 秒の周回で 8 時間 → **約 0.67 秒**・**288,000 件**（`last_result().triggered` に次の tick まで残る）。debug の build ではさらに遅い。1 回きりの止まりで、データは壊れない。今の本番（wintf）はトリガーを使わないので当たらない。上限で打ち切る・剰余で飛ばす形は、注にあるとおり外から見える振る舞い（`loops_completed`・乱数の消費・起きた子の列）を変えるので、前の決めのとおり入れない。
- **直し:** 注を事実に合わせ、限界を書く（`shortcut:` の形）。例: 「`shortcut:` トリガーを持つループは 1 周ごとに子を起こすので、0.1 秒の周回で 8 時間飛ぶと 1 回の tick に約 0.7 秒・28.8 万件の結果になる。トリガーを使う文書が本番に入ったら、飛ばす周回の子の扱いを決め直す」。`MIN_LOOP_DURATION` の注は「周回の数の上限を作る（トリガーを持つループの重さは `process_loops` の注）」に直す。コードは変えない。
- **触る所:** 注と説明文だけ。

---

## 判定と数

- **判定:** must-fix は 0 件。先に R3-01 を直す。R3-02 は同じ直し（期限を出来事に入れる）で一緒に閉じられる。R3-03 は注だけ。R3-01 と R3-02 は本番の振る舞い（トリガーの発火）を変えるが、今の本番（wintf）はトリガーを使わないので、今の本番の振る舞いは変わらない。
- **Lean:** −0 行（減らす指摘は無い）。依存は −0。
- **数:** must-fix 0・should-fix 2・nice-to-have 1（合計 3）。上限は設けていない。

## 指摘にしなかったもの（決めとして読んだもの・2 回目の直しの確かめ）

- R2-01: `lerp` の頭の `if from == to { return from; }` で、保つ遷移は途中のどの t でもちょうど from。Integer も同じ道。檻 `hold_segment_stays_exactly_at_value`（100・0.1・0.3・255 を 1001 点）がある。
- R2-02: `evaluate_segments` の `prev` は、終わったセグメント（即時のセグメントを含む）しか指さない（進行中のセグメントは手前で返る）。隙間の値は直前の to になり、Conclude（`find_active_segment`）・Cancel と Trim（`evaluate_all_for_group`）の値と食い違わなくなった。最初のセグメントの前と、2 周目の先頭の遅れの中は今どおり from（R1-10 の決め）。周回の後の待ち（`value_time` が inf を返す）は最後の値。
- R2-03: 繰り返しは止まる。Step 2 の対象は `current_time >= end_time` の Playing だけで、`process_loops` は必ず 1 周進めるか Conclude する。進まない周回は `before + (loop_duration + max_delay) <= before` で Conclude。トリガーの循環は V15t が拒む。起こした子が無限のループでも、同じ変数の次の子が競合で終わらせる（Never なら次の子が Error）ので、子の数は増え続けない。`within_loop`（`fire_time / time_scale <= loop_duration`）は、発火の時刻がちょうど周回の終わりのトリガーを含み、それは Step 2 が周回を進める前の同じ tick で起きる。周回の頭（fire_time 0）のトリガーも、繰り返しの 2 回目で同じ tick に起きる（2 回目の報告で言った 1 コマの遅れも消えた）。残るのは時刻の順（R3-01）・期限（R3-02）・重さ（R3-03）。
- R2-04: `cargo doc -p dola --no-deps` を作業場の外の target で回し、警告 0 を確かめた。
- R2-05: `crates/dola` に「update()」と「R1-」「R2-」の符号はもう無い（grep で確かめた）。
- cue の側（`runtime.rs`・`schedule.rs`）は 2 回目から変わっていない。`TimedSchedule::tick` の冪等の守りと `CuePlayer::tick` の「残りが減ったときだけ配る」を読み直し、同じ時刻の再 tick で二重に配らないこと、区切りの解決の後に区切りの後ろを集められることを確かめた。

## 調べなかったこと

- `cargo test`・clippy は回していない（報告のファイルだけを書く決め）。R3-01 の例 1 と R3-02、R3-03 の時間は、作業場の外（scratchpad）に `dola` を path で引く小さな実行ファイルを作って回した（作業場のファイルは変えていない）。R3-01 の例 2 と Never の例は、コードを読んで出した具体例で、回してはいない。R3-03 の時間は手元の 1 台の release の build の 1 回の値。
- src は 2 回目の直しで変わったファイル（`facade.rs`・`loop_controller/mod.rs`・`timeline_manager/mod.rs`・`interpolator/mod.rs`）と、それが呼ぶ `instance_manager`・`conflict_resolver`・`subscription_manager`・`types.rs`・`clock.rs`・`compile/mod.rs`・`compile/resolve.rs`・`validate/mod.rs`・cue の `runtime.rs`・`schedule.rs` を全部読んだ。`validate/rules.rs`・`error.rs`・`cue/sheet.rs`・`cue/command.rs`・`cue/sink.rs`・`builder.rs` ほかの小さな型のファイルは 2 回目から変わっておらず、流し読みにとどめた。tests/ は 2 回目の直しで足したテスト（`skipped_loops_fire_their_triggers`・`trigger_after_loop_end_does_not_fire_in_skipped_loops`・`gap_between_segments_holds_previous_end_value`・`hold_segment_stays_exactly_at_value`）と `simultaneous_triggers_resolve_in_group_order` を読み、残りの約 14,000 行は一字ずつは読んでいない。
