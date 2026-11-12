# audit-dola かけ直し 21・4 回目（ponytail-audit・範囲 `crates/dola/`）

**この範囲がしていること:** dola は 2 つのエンジンを持つ。1 つは宣言したアニメ（変数・遷移・キーフレーム・ストーリーボード）を検証してコンパイルし、`DolaRuntime` が時刻を受けて値の差分を購読者へ配るもの。もう 1 つは cue（`CueSheet` → `TimedSchedule` → `CuePlayer`）で、さくらスクリプトの台本を絶対時刻で全部の受け手へ配るもの。

**想定した負荷:** 1〜3 回目と同じ。1 つのプロセスの UI スレッドが 60fps 前後で `tick` を呼ぶ。インスタンスは十数個、cue は台本 1 枚で数百。文書はゴーストの作者が書く（＝作者の書き損じや極端な値が入りうる入口）。PC のスリープの後などに、1 回の tick で時刻が大きく飛ぶことがある。

**対象:** HEAD `4c193c9d`。3 回目の 3 件の直し（`4c193c9d`・tick の中の出来事を時刻の順に）と tasks.md の「## Implementation Notes」を読んだうえで、新しい指摘と、直しが持ち込んだ欠陥だけを挙げる。

**数えなかったもの:**
- 要件 11.3 の残す 3 点: #4 の副作用（周回の長さより後ろのトリガーは、どの周回でも発火しない）・`WaitForInput` と `resolve_click`（`\x` の受け皿）・機能 `yaml` と `serde_yaml`。
- 開発者の決めとして残る 3 点: (a) ベジェの制御点を「有限」より狭めるか・(b) 未来の時刻の `start()` が、ぶつかる古いインスタンスを今すぐ終わらせるか・(c) `MIN_LOOP_DURATION` を有限のループにも当てるか。
- 1 回の tick で多くの周回を飛ばす重さ（トリガーを持つループ）。`process_loops` の `shortcut:` の注（R3-03）に書いてある。手元の release の build で測り直すと、0.1 秒の周回で 8 時間飛ぶと約 0.72 秒・28.8 万件で、注の「約 0.67 秒」とほぼ同じ。

**3 回目の直しの確かめ:** 3 件とも、指摘した形どおりに直っている（下の「指摘にしなかったもの」）。出来事の繰り返し（`update_internal` の `while let`）は必ず止まる。ただし、R3-01 の 3 つ目の例（Never の X の自然の終わりとトリガー）を縛る檻が無い（R4-01）。また、直しで有限のループ（トリガーなし）の周回の終わりが 1 つずつ出来事になり、時刻が大きく飛んだ tick で前より遅くなった。`process_loops` の注の「トリガーなしなら 1 周は数を足すだけ」は有限のループでは事実と合わない（R4-02）。テストの説明に監査の符号が戻った（R4-03）。

---

## Must fix（0 件）

なし。

---

## Should fix（1 件）

### R4-01 有限のループの周回の終わりを出来事に入れる枝（`inst.loop_count != -1`）を縛る檻が無い（R3-01 の 3 つ目の例）
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/facade.rs:379-381`（`next_event_time` の `if inst.loop_count != -1 || !inst.trigger_states.is_empty() { take(inst.end_time); }`）・`crates/dola/tests/trigger/event_order_test.rs`（檻は R3-01 の例 1・例 2 と R3-02 と一時停止の丸めの 4 本）
- **何が悪いか:** この枝は、トリガーを持たない有限のループ（ふつうの 1 回きりのストーリーボードを含む）の自然の終わりを、同じ tick の中のトリガーより先に処理するためにある。R3-01 の 3 つ目の例がそれで、今の HEAD では直っている。ところが、`inst.loop_count != -1 ||` を消してもテストは全部通る。
  - 例: Never の X（x を 0→1・1.2 秒・1 回）と、0 秒に起こした親 P（a を 3 秒・2.5 秒に子 C を起こす・C は x を 5→6）。60fps で 0〜3 秒を回すと、X は 1.2 秒に終わり、C は `Started`。tick を 0.0 → 3.0 にしても HEAD は `Started`。枝を消した写しでは、X の終わりが出来事にならず、2.5 秒のトリガーが生きた X とぶつかって `Error`（Conflict）になる。作業場の外（scratchpad）に crate を写して枝を消し、この台本と `cargo test --test trigger --test runtime --lib` を回して確かめた（テストは 219・119・42 本とも緑のまま）。
  - 同じ写しで、期限を出来事に入れる行（`take(deadline)`）を消すと `finish_delayed_conclude` が落ち、`check_finish_deadlines(t)` を `current_time` にすると `triggers_before_finish_deadline_fire_in_the_crossing_tick` が落ちる。最後の `process_loop_ends(current_time)` を消すとループのテスト 4 本が落ちる。檻が無いのはこの枝だけ。
- **直し:** `event_order_test.rs` に上の台本の檻を 1 本足す。60fps と 0.0 → 3.0 の 2 通りで回し、どちらも `triggered` が `Started` 1 件。コードは変えない。
- **触る所:** テストだけ。

---

## Nice to have（2 件）

### R4-02 有限のループ（トリガーなし）は 1 周ごとに出来事になるので、時刻が大きく飛んだ tick が 3 回目の直しの前より数十倍遅い。`process_loops` の注の「トリガーなしなら 1 周は数を足すだけ」が有限のループでは合わない（3 回目の直しが持ち込んだ）
- **重さ:** nice-to-have
- **場所:** `crates/dola/src/runtime/facade.rs:335-344`（出来事 1 つごとに `next_event_time`・`check_finish_deadlines`・`process_triggers`・`process_loop_ends` が全部のインスタンスを見る）・`:379-381`（有限のループは周回ごとに `end_time` を出来事にする）・`src/runtime/loop_controller/mod.rs:127`（「トリガーなしなら 1 周は数を足すだけで、0.1s の周回で 8 時間飛んでも約 29 万回の軽い反復」）
- **何が悪いか:** 3 回目の直しの前は、トリガーの無い有限のループの飛ばした周回は `process_loops` の中の軽い反復だった。今は周回の終わりが 1 つずつ出来事になり、1 周ごとに表の全部のインスタンスを 4 回なめ、Vec を 3 つ作る。なので 1 周の重さがインスタンスの数に比例する。手元で release の build で、直しの前（`HEAD~1` を `git archive` で写したもの）と HEAD を同じ台本で測った（有限のループを 0 秒に起こし、tick を 0.0 の次に全部の周回の後へ飛ばす）:
  - 0.01 秒 × 100 万周・ほかのインスタンス 0: 1.2 ms → **75 ms**。
  - 0.01 秒 × 100 万周・ほかに無限のループ 10 個: 1.5 ms → **161 ms**。
  - 0.1 秒 × 10 万周・ほかに 10 個: 0.2 ms → **17.5 ms**。
  - 1 回きりの止まりで、結果は同じ（データは壊れない）。有限のループに `MIN_LOOP_DURATION` が当たらないのは決め (c) のとおりで、周回の数の上限は `loop_count`（最大約 21 億）。ふつうの文書（数回の点滅など）では当たらない。今の本番（wintf）は `start` を呼ばないので当たらない。注の「トリガーなしなら 1 周は数を足すだけ」は無限のループ（出来事に入れない）でだけ正しい。
- **直し:** 注を事実に合わせ、限界を書く（R3-03 と同じ形・コードは変えない）。例: 「トリガーなしの無限のループは出来事に入れず、1 周は数を足すだけ。`shortcut:` トリガーなしの有限のループは周回の終わりごとに出来事になり、1 周ごとに全部のインスタンスを見る（0.01 秒 × 100 万周を一度に飛ばすと、ほかに 10 個あって約 0.16 秒）。大きな `loop_count` の文書が本番に入ったら、`next_event_time` で最後の周回の終わりの下限（`end_time + 残りの周回 × (loop_duration + 待ちの最小)`）だけを出来事にする」。
- **触る所:** 注だけ。

### R4-03 新しいテストの説明に監査の符号（R3-01・R3-02）が戻った（R2-05 と同じ種類）
- **重さ:** nice-to-have
- **場所:** `crates/dola/tests/trigger/event_order_test.rs:1`（「…時刻の順に処理する（R3-01・R3-02）。」）
- **何が悪いか:** R2-05 で、監査の報告を読まないと意味が取れない符号（`R1-02` など）をテストから消した。3 回目の直しで足したこのファイルの説明に、同じ種類の符号が入った。`crates/dola` の中で `R[0-9]-[0-9]{2}` に当たるのはこの 1 か所だけ（grep で確かめた）。
- **直し:** 「（R3-01・R3-02）」を消す。前の文と 2 行目が、何を守るテストかを平易に言っている。
- **触る所:** テストの説明だけ。

---

## 判定と数

- **判定:** must-fix は 0 件。先に R4-01（檻 1 本）を足す。R4-02 と R4-03 は注と説明だけ。3 件とも本番の振る舞いを変えない。
- **Lean:** −0 行（減らす指摘は無い・R4-03 は 1 行の中の字だけ）。依存は −0。
- **数:** must-fix 0・should-fix 1・nice-to-have 2（合計 3）。上限は設けていない。

## 指摘にしなかったもの（決めとして読んだもの・3 回目の直しの確かめ）

- 出来事の繰り返しが止まること: `next_event_time` が出す時刻は、どれも同じ周の中で片づく。期限は Step 1 が Conclude する（Paused でも `check_finish_deadlines` が拾い、Paused → Concluded は通る遷移）。まだ起きていないトリガーは `fire_due_triggers` が同じ `trigger_wall_time` で拾って印を立てる。有限のループとトリガーを持つループの `end_time` は、`process_loops` が必ず 1 周進めるか Conclude する。NaN の期限は `f64::min` と最後の `filter` が捨てる（`check_finish_deadlines` も拾わないので、同じ時刻が出続けることは無い）。
- R3-01: 例 1・例 2 は檻 `triggers_in_one_tick_fire_in_time_order`・`skipped_loop_triggers_fire_in_time_order_across_parents` が 60fps と一度に飛ばす 2 通りで縛る。3 つ目の例（Never）は HEAD で直っている（上の R4-01 で回して確かめた）。負の `trigger_start_offset`（検証は有限だけを求める）で子が過去に起きると、次の出来事の時刻は前へ戻るが、60fps でも子の過去のトリガーは同じコマで起きるので、順は同じ。
- R3-02: 期限は出来事に入り、期限より前のトリガーは起きる。同じ時刻の期限とトリガーは Step 1 が先（60fps でちょうどその時刻のコマが来たときと同じ）。檻 `triggers_before_finish_deadline_fire_in_the_crossing_tick`。
- R3-03: 注は事実に合う（測り直して約 0.72 秒）。`MIN_LOOP_DURATION` の注も直っている。
- `trigger_wall_time` の `.min(inst.end_time)`: 周回の頭からの式と `end_time` の式は一時停止と待ちの足し方が違うので、丸めで 1 ulp ずれる分だけを抑える。周回の長さの内のトリガーでは、これより大きくずれることは無い（`advance_loop` と `resume` が両方の式に同じ量を足す）。檻 `loop_end_trigger_survives_pause_rounding`。
- トリガーなしの無限のループは出来事の時刻ごとに `process_loop_ends(t)` で進むが、それは Step 1.5 の後。なので、同じ時刻に起きた子とぶつかって Cancel・Trim・Conclude で終わるときの確定値（`evaluate_all_for_group` などの値）は、前の周回の起点で測ることがある。この値は `last_values` へ入るだけで、同じ変数は起きた子の項目が必ず評価を取る（競合は子の変数で判定する）ので、配る値は変わらない。3 回目の直しの前からある形で、指摘にしなかった。
- cue の側（`runtime.rs`・`schedule.rs`）と compile・validate は 3 回目から変わっていない。

## 調べなかったこと

- 作業場のファイルは報告のほかに変えていない。R4-01 の台本と枝を消した写し・R4-02 の時間は、作業場の外（scratchpad）に `crates/dola` と `HEAD~1` の `crates/dola`（`git archive` で写した）を置き、小さな実行ファイルと `cargo test` を回した。時間は手元の 1 台の release の build の 1 回の値。写しの `cargo test`（全部）は緑。clippy は写しの Cargo.toml に機能 `yaml` を書かなかったので `unexpected_cfgs` で止まり、作業場の clippy は回していない。
- src は 3 回目の直しで変わったファイル（`facade.rs`・`loop_controller/mod.rs`・`types.rs`）と、出来事の繰り返しが呼ぶ `instance_manager/mod.rs`・`instance_state.rs`・`conflict_resolver.rs`・`timeline_manager/mod.rs` の評価の部分を全部読んだ。ほかの src は 3 回目から変わっておらず、読み直していない。tests/ は直しで足した `event_order_test.rs` と、変わった `trigger.rs`・`runtime_test.rs` の行を読み、残りは枝を消した写しでテストを回して檻の有無を見るだけにした。
