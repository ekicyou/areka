# audit-dola かけ直し 21・1 回目（ponytail-audit・範囲 `crates/dola/`）

**この範囲がしていること:** dola は 2 つのエンジンを持つ。1 つは宣言したアニメ（変数・遷移・キーフレーム・ストーリーボード）を検証してコンパイルし、`DolaRuntime` が時刻を受けて値の差分を購読者へ配るもの。もう 1 つは cue（`CueSheet` → `TimedSchedule` → `CuePlayer`）で、さくらスクリプトの台本を絶対時刻で全部の受け手へ配るもの。

**想定した負荷:** 1 つのプロセスの UI スレッドが 60fps 前後で `tick` を呼ぶ。インスタンスは十数個、cue は台本 1 枚で数百。文書はゴーストの作者が書く（＝作者の書き損じや極端な値が入りうる入口）。

**数えなかったもの（要件 11.3）:** #4 の副作用（最後の区切りより後のトリガーは親が先に終わるので発火しない）・`BarrierKind::WaitForInput` と `CuePlayer::resolve_click`（`\x` の受け皿）・機能 `yaml` と `serde_yaml`（dola-yaml-backend の持ち物）。

---

## Must fix（4 件）

### R1-01 即時のセグメントが、後ろのセグメントを全部隠す
- **重さ:** must-fix
- **場所:** `crates/dola/src/runtime/timeline_manager/mod.rs:285-303`（`evaluate_segments`、とくに 292-296 行）
- **何が悪いか:** セグメントを前から見ていき、長さ 0 の即時のセグメントに来たとき、時刻がその始まり以降なら**その場で値を返して抜ける**。後ろのセグメントは一度も見ない。例: 「x を 5 にする（duration なし）」→「x を 5→10 に 1 秒」という 2 エントリ。セグメントは [0,0] と [0,1] で、t=0.5 の値は 7.5 のはずが 5 のまま。終わりに `conclude` が最後の値 10 を配るので、動かずに 5 から 10 へ跳ぶ。途中に即時のセグメントがある台本（0→1・即時に 5・5→0）も、即時の時刻から先は終わりまで 5 で止まる。重なりの検査（`compile/mod.rs` の `prev.end_time > next.start_time`）は [0,0] と [0,1] を重なりとみなさないので、コンパイルは通る。テストは即時のセグメント 1 本だけの `evaluate_zero_duration_segment_returns_to_value` しか無い。同じ形の `find_active_segment`（329 行）は `active` に入れて先へ進むので正しい。
- **直し:** 即時の枝で返さず、先へ進める。`if duration <= 0.0 { if effective_time >= seg.start_time { return ... } }` を消し、`else if` だけを `if duration > 0.0 && effective_time < seg.end_time { return 途中の値 }` にする。後ろが無ければ、今ある「最後の値を保つ」（306 行）が即時の値を返す。檻は 1 本: `[即時 0→5 @0, 5→10 @0..1]` の t=0.5 が 7.5、`[0→1 @0..1, 即時 5 @1, 5→0 @1..2]` の t=1.5 が 2.5。
- **触る所:** 本番の振る舞い（値の配り方）。

### R1-02 loop_offset のイージングの出力を [0,1] に収めず、待ちが min を割り、負にもなる。イージングによっては tick が戻らない
- **重さ:** must-fix
- **場所:** `crates/dola/src/runtime/loop_controller/mod.rs:47-59`（`generate_delay`）・`:126-132`（`process_loops` の while）
- **何が悪いか:** 待ちは `min + eased * (max - min)` で、`eased` はイージングの出力そのもの。Back 系（最小で約 −0.1）・Elastic 系・Sine 系（R1-03 のため −1〜1 を行き来する）・ベジェの制御点（文書の任意の有限の値）は [0,1] の外を出す。すると待ちは min より短くなり、負にもなる。検証（V14〜V16）は「待ちは min 以上 max 以下で、どちらも 0 以上」を約束しているので、それに反する。最悪の例: `loop_count: -1`・長さ 1 秒・`loop_offset: {min: 0, max: 1, easing: {type: cubic_bezier, x0: -5, x1: -5, x2: -5, x3: -5}}`。毎回 `eased = -5` で待ち −5 秒、`end_time` は 1 周ごとに 4 秒戻る。`while current_time >= instance.end_time` が永遠に真のままになり、UI スレッドが止まる。文書の値は全部有限なので、検証は通る。
- **直し:** `let eased = apply_easing(t, Some(easing)).clamp(0.0, 1.0);`（1 行。NaN は来ない＝t は [0,1) で、制御点は有限）。檻 1 本: 上の制御点 −5 のベジェと Back-in で、`generate_delay` が乱数の種を変えても `[min, max]` に入る。
- **触る所:** 本番の振る舞い（待ちの長さ）。

### R1-03 Sine 系と Elastic 系の曲線が誤っていて、SineOut と ElasticIn は終わりに from の値へ戻る（外のクレート `interpolation` 0.3.0 の PI_2）
- **重さ:** must-fix
- **場所:** `crates/dola/src/runtime/interpolator/mod.rs:147-158`（`apply_named_easing` の SineIn〜ElasticInOut の 6 行）。もとは外のクレート `interpolation-0.3.0/src/ease.rs:96`（`PI_2 = 6.283…`＝2π）と `:237-246`・`:308-329`
- **何が悪いか:** 外のクレートは π/2 を使うべき式に 2π を入れている。dola への効き目は、注（10.2）が言う「曲線が少しずれる」より重い:
  - `SineOut(t) = sin(2πt)`: t=1 で約 −2.4e-16。`value_at(seg, 1.0)` は from の値になるので、**sine_out の遷移は最後に from の値へ戻って終わる**（`conclude` の最終値も from）。途中では 0.25 で to を通り、0.75 で from の向こう側（例 0→100 なら −100）まで行く。
  - `SineIn(t) = sin(2π(t−1)) + 1`: 0→2→1→0→1 と 1 周の波を描く（0.25 で to の 2 倍）。
  - `ElasticIn(1) = sin(13·2π)·1 ≈ 0`: **elastic_in も最後に from の値へ戻る**。`ElasticInOut` は真ん中で 0 付近から 0.5 付近へ跳ぶ。
  - loop_offset のイージングに使うと、R1-02 の負の待ちを作る。

  dola の本番の文書（wintf）は今これらの名前を使っていないが、公開のイージングの名前（`sine_in` など）として文書に書け、書けば必ずこうなる。
- **直し:** 外のクレートはそのまま使う決まりなので、dola の `apply_named_easing` の 6 行を自前の式にする（Penner の式。`SineIn = 1 − cos(t·π/2)`・`SineOut = sin(t·π/2)`・`ElasticIn = sin(13·π/2·t)·2^(10(t−1))` など。`SineInOut` はクレートの式が正しいのでそのまま）。合わせて 6 本の既知の値の檻（t=0・0.5・1）。注 10.2 は「別の spec で起票」としているが、終わりの値が from に戻るのは誤った最終値なので、この spec の中か次の版の前に直すべきと判断する。
- **触る所:** 本番の振る舞い（補間の値）。

### R1-04 トリガーで起こした子の、そのトリガーは同じ tick では見ない。子がその tick で終わると、子のトリガーは二度と発火しない
- **重さ:** must-fix
- **場所:** `crates/dola/src/runtime/facade.rs:349-371`（`update_internal` の Step 1.5 と Step 2）・`:396-485`（`process_triggers`）
- **何が悪いか:** `process_triggers` は、tick の始まりに生きていたインスタンスのトリガーだけを集めて子を起こす。その中で起きた子のトリガーは見ない。そのすぐ後の Step 2 は、起きたばかりの子も含めて自然の終わりを判定し、`conclude_internal` が子の `trigger_store` を消す。例 1: 親 A がトリガーだけで B を起こし、B もトリガーだけ（長さ 0）で C を起こす台本。`start("A")` → `tick` で B は起きるが、B は長さ 0 なのでその tick で終わり、**C は決して起きない**（同じ B を `start("B")` で直に起こせば C は起きる）。例 2: 既存のテスト `trigger_chain_a_b_c` の台本（B は 0.5 秒・終わりの 0.5 で C を起こす）を、t=0 を飛ばして初めの tick を t=0.6 にする（コマ落ち）。B は 0.0 開始・0.5 終わりで、その tick で終わり、C は起きない。t=0 で tick したときだけ C が起きる＝結果がコマの時刻で変わる。要件 11.3 の残す項目（最後の区切りより**後ろ**のトリガー）とは違い、ここは区切りの上か内側のトリガーで、直に start したときは発火する。今の本番（wintf）はトリガーを使う文書を持たないが、使えば必ず当たる。
- **直し:** `process_triggers` の Step 1〜3 を「集めた分が空になるまで」繰り返す（`loop { 集める; if pending.is_empty() { break } 旗を立てる; 子を起こす }`）。トリガーの循環は検証（V15t）が拒むので必ず止まる。起きた子のトリガーも Step 2 の前に発火する。檻 2 本: 長さ 0 の B を挟んだ A→B→C が 1 回の tick で C まで起きる・`trigger_chain_a_b_c` を初めの tick 0.6 で回しても C が起きて、C の値が t=0.6 で 0.6 になる。
- **触る所:** 本番の振る舞い（トリガーの発火）。

---

## Should fix（8 件）

### R1-05 有限でも極端な数で、補間の値が NaN／inf になり、そのまま配られる（注 10.1 の分）。和があふれる所が 3 つある
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/interpolator/mod.rs:100`・`:107`（`interpolation::lerp` = `from + (to − from)·t`）・`timeline_manager/mod.rs:37-45`（`fit` は NaN を素通しする）・`compile/resolve.rs:387`（`from + relative_to`）・`:332-333`・`:349-350`（`base_time + delay`・`+ duration`）・`:257`（`t + offset`）・`compile/mod.rs:230`（`v < min || v > max` は inf を値域の無い変数で通す）
- **何が悪いか:** 検証は文書の数が 1 つずつ有限かだけを見る。和や差が inf になる場合は見ていない:
  1. from=−1.7e308・to=1.7e308 だと `to − from` が inf になる。t=0（始まる前と始まりの瞬間）は `inf·0 = NaN`、途中は inf。`fit` は NaN と比べても偽なので、**値域のある変数でも NaN をそのまま配る**。NaN は自分と等しくないので、差分が毎フレーム「変化あり」になる（`types.rs:29-35` の注のとおり）。
  2. `relative_to` の結果（例 from 1e308＋1e308）が inf。値域の無い変数ならコンパイルの値域の検査を通る。
  3. `delay: 1e308`・`duration: 1e308` で終わりの時刻が inf。台本の長さも `end_time` も inf になり、インスタンスは永遠に終わらない（`has_instances()` がずっと真で、wintf は画面の更新を予約し続ける）。トリガーの発火の時刻も inf で、発火しない。
  4. lerp の形のせいで、終わりの値も正確でない: from=1e16・to=1 だと `1e16 + (1 − 1e16)·1 = 0` で、to の 1 にならない。
- **直し:** (a) 補間を dola の 1 行 `from * (1.0 - t) + to * t` にする（両端が正確で、有限どうしではあふれない。`interpolation::lerp` を使う所は 2 か所だけ）。(b) コンパイルで、作った `to_value`・`segment_end`・キーフレームの時刻が有限かを確かめ、外れたら `InvalidNumber` を積む（`compile/mod.rs` の値域の検査の隣に数行）。(c) ベジェの制御点が巨大なときの `eased_t` のあふれは (a) でも残る。制御点を「有限」から「たとえば |x| ≤ 1e3」へ絞る規則を足すかは開発者の決め（CSS の cubic-bezier は y がふつう −1〜2）。
- **触る所:** 本番の振る舞い（極端な文書を拒む・端の値が正確になる）。

### R1-06 `at`／`between` のエントリの from が処理の順で決まり、無関係のキーフレームの並べ方で値が跳ぶ（注 7.1 の分）
- **重さ:** should-fix
- **場所:** `crates/dola/src/compile/mod.rs:184-194`（`sequential` でないときは `var_last_entry`＝処理の順で最後に解いた分から値を取る）
- **何が悪いか:** 処理の順は Kahn の並べで、時刻の順でも配列の順でもない。例: `[0] x at "k" to 10`・`[1] x at "j" to 3`・`[2] kf "k" = start+5`・`[3] kf "j" = start+1`（どれも duration 1・from なし）。処理は 2→0→3→1 で、`[0]` は from＝初期値 0、`[1]` は from＝`[0]` の to の 10。時刻の並びは `[1]`（1〜2 秒）→`[0]`（5〜6 秒）なので、t=1 で 0 から 10 へ跳んで 3 へ動き、t=5 で 3 から 0 へ跳んで 10 へ動く。`[2]` と `[3]` を入れ替えるだけで跳びは消える（処理が 3→1→2→0 になり、from が 0→3・3→10 とつながる）。完了 dola-compiled-transition 要件 4.2 の「直前」に決めが無い（注のとおり）。セグメントは時刻の順に並べて評価するので、「時刻の上で直前のセグメントの to」と読むのが正しいと判断する。配列の順も、上の例では `[0]` が from＝初期値になり、跳びが残る。
- **直し:** `at`／`between` で from の無いセグメントは、コンパイルの終わり（Step 5 の並べ替えの後）で、変数ごとに時刻の上で直前のセグメントの to（無ければ初期値）から from を埋める。`relative_to` の to と値域の検査もそこで出す。檻 1 本: 上の例で `[2]`・`[3]` の順に関わらず、from が 0→3・3→10。
- **触る所:** 本番の振る舞い（from を省いた at/between の値）。

### R1-07 長さ 0（またはごく短い）の有限のループで、1 回の tick が loop_count 回まで回る
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/loop_controller/mod.rs:126-132`（while）・`facade.rs:217-227`（長さ 0 と短すぎる周回を拒むのは `loop_count == -1` だけ）
- **何が悪いか:** 有限の `loop_count` では、長さ 0 も、0.1 秒より短い周回も許される（完了 dola-runtime-3-facade 要件 3 は −1 だけを拒むと決めている）。長さ 0・待ちなしだと `advance_loop` が `end_time` を進めないので、while は `loop_count` 回まわってから終わる。`loop_count: 2147483647` とトリガーだけ・キーフレームだけのストーリーボードで、最初の tick が約 21 億回まわる（周回ごとにトリガーの印の列も回す）。UI スレッドが秒単位で止まる。長さ 1e-9 秒でも、1 秒の tick の間で最大 10 億回まわる。文書は検証を通る。
- **直し:** 周回で時刻が進まないなら、残りの周回もその瞬間に終わる。while の中で `let before = instance.end_time; advance_loop(..); if instance.end_time <= before { return LoopAction::Conclude; }`（長さ 0 の結果は今と同じで、回数だけが 1 回になる）。ごく短い周回の上限も要るなら、`MIN_LOOP_DURATION` を有限のループにも当てるかは開発者の決め（今の完了 spec と違う決めになる）。檻 1 本: 長さ 0・`loop_count` 1e9 の tick がすぐ返り、終わる。
- **触る所:** 本番の振る舞い（長さ 0 の有限のループの周回の数え方。外から見える値は同じ）。

### R1-08 まだ始まっていないインスタンスが、生きている古いインスタンスの値を今すぐ上書きする
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/timeline_manager/mod.rs:118-141`（`evaluate` は group_id の最も大きい項目を取る）・`:286-288`（始まる前は最初のセグメントの from を返す）・`facade.rs:438`（`child_start = wall_fire_time + start_offset`）・`:465`（子の競合の判定から親を外す）
- **何が悪いか:** 例: 親が x を 0→1 に 2 秒で動かし、0.5 秒で子を `trigger_start_offset: 1.0` で起こす。子は x を 5→6 に動かす。親は競合から外れるので生きたまま。`evaluate` は group_id の大きい子を取り、子の実効時刻は負なので from の 5 を返す。**0.5 秒で x が 5 に跳び**、子が始まる 1.5 秒までの親の動きは見えない。`start(name, 未来の時刻)` も同じで、競合の終わりの処理は古いインスタンスを今すぐ終わらせ、未来の時刻での値を今配る。`evaluate` の注は「最新 group_id 優先（Tier 2 暫定動作）」と書いている。
- **直し:** `evaluate` で、始まっていない（`loops_completed == 0` かつ実効時刻 < 最初のセグメントの始まり）項目は、始まった項目が 1 つでもあれば候補にしない。全部が始まっていないときだけ今の from を返す。競合（未来の start）の側をどうするかは開発者の決め。檻 1 本: 上の親子の例で t=1.0 の x が親の値 0.5。
- **触る所:** 本番の振る舞い（同じ変数に複数のインスタンスがあるときの値）。

### R1-09 `InvalidLoopCount` の検査と種類が届かない死んだコードのまま残る（注 6.2 の「後の段で消す」が済んでいない）
- **重さ:** should-fix
- **場所:** `crates/dola/src/runtime/facade.rs:209-212`・`src/runtime/types.rs:75-77`（`RuntimeError::InvalidLoopCount`）・注 `src/runtime/loop_controller/mod.rs:30-34`（保証の出どころを facade の検査と書く）・テスト `tests/runtime/facade_test/load_start.rs:175` の注
- **何が悪いか:** `loop_count` が 0 や −2 以下なら `validate_numbers`（`validate/rules.rs` の `lc >= 1 || lc == -1`）が `InvalidNumber` で拒み、`load_document` を通らない。facade の検査はどの入力でも通らず、公開の種類 `InvalidLoopCount` は作られない（外のクレートの呼び手 0）。loop_controller の注は、もう無い保証の道を指している。
- **直し:** facade の 4 行と `InvalidLoopCount` の種類を消し、loop_controller の注を「validate の数値の規則が −1 か 1 以上に限る」に直す。種類を表すテスト（`core_types_test.rs` など）にあれば一緒に消す。
- **触る所:** 本番の公開の型（`RuntimeError` の種類が 1 つ減る・振る舞いは同じ）。

### R1-10 「周回ごとに先頭の遅れを繰り返す」を縛る檻が無い（注 5.4 の分）
- **重さ:** should-fix
- **場所:** `crates/dola/src/compile/mod.rs:304-307`（長さはコンパイルの起点から数える）・`src/runtime/loop_controller/mod.rs:69-88`（`advance_loop` は `loop_duration` ごと起点を進める）。テストの穴: `tests/runtime/facade_test/load_start.rs:98-125`（1 周だけ）・`tests/runtime/loop_integration_test.rs`（先頭の遅れの無い台本だけ）
- **何が悪いか:** 先頭の遅れ（最初の遷移の delay）は長さに入り、ループでも毎周くり返す（完了 dola-runtime-5-loop の決め）。この振る舞いを守るテストが無い。誰かが「2 周目からは遅れを抜く」と直しても、長さを先頭の遅れの後から数える（#4 の前の形に戻す）と直しても、テストは緑のまま。
- **直し:** 檻 1 本: delay 0.5・duration 1.0・`loop_count: 2` で、t=1.75（2 周目の遅れの中）が 2 周目の from、t=2.5 が 2 周目の途中の値、t=3.0 で終わる。
- **触る所:** テストだけ。

### R1-11 loop_offset の e2e テスト 4 本が値も終わりも確かめない。「差分が空」を「終わった」の証拠にしている
- **重さ:** should-fix
- **場所:** `crates/dola/tests/runtime/loop_offset_test/loop_offset_compile.rs:133-153`（`e2e_no_offset_natural_end`）・`:187-201` 付近（`e2e_loop_count_1_ignores_offset`）・`:285-302` 付近（`e2e_without_offset_backward_compatible`）・`:336-370`（`e2e_large_delay`）・`:372-391`（`e2e_f64_precision_many_loops`＝assert 0 本）
- **何が悪いか:** `let _changes = ...` で結果を捨て、終わりを `changes.is_empty()` だけで見ている。値を最後の値で保っている間（周回の待ちの中・自然に終わる直前）も差分は空なので、インスタンスが終わらなくても緑になる。`e2e_large_delay` は「50 秒ではまだ生きている」と注に書くが確かめない。`e2e_f64_precision_many_loops` は 115 秒まで進めて何も見ない（ループが止まらなくなれば終わらないだけで、精度の誤りは見えない）。`e2e_loop_count_1_ignores_offset` は待ちが無視されたことを見ない。research #44 で直したのは 176-244・318-330 の 3 本だけで、残りがこの 5 本。
- **直し:** 終わりは `rt.has_instances()` で見る（11-13 に足した口）。`e2e_large_delay` は t=50 で `has_instances()` が真・t=106 で偽。`e2e_f64_precision_many_loops` は t=115 の値を周回の式（1.1 秒周期）から出して比べる。`e2e_no_offset_natural_end` は t=2.5 で最終値 1.0 が配られることを見る。`e2e_loop_count_1_ignores_offset` は t=1.5 で `has_instances()` が偽。
- **触る所:** テストだけ。

### R1-12 `typewriter` は文書からランタイムのヒントまで運ばれるが、誰も読まない
- **重さ:** should-fix
- **場所:** `crates/dola/src/variable.rs:28-29`・`src/compile/types.rs:75-78`（`VariableTypeHint::Integer { typewriter }`）・`src/compile/resolve.rs:403-405`
- **何が悪いか:** 注は「設定時: initial=0, 終了値=文字列長」と言うが、検証もコンパイルもランタイムもそれを確かめず、使わない（dola の中の読み手 0・外のクレートの読み手 0。外の `typewriter` の語は areka の別の文字送りの仕組み）。`start` のたびに文字列ごと複製される。完了 dola-compiled-transition の要件「ヒントとして含める（ランタイムがインデックスから部分文字列を取得するため）」は、ランタイム側が作られないまま残った半分の機能。
- **直し:** 使う spec が無いなら、欄 2 つと複製を消す（serde は知らない欄を捨てるので、`typewriter` を書いた文書も読める）。使う予定があるなら、その spec の名前を注に書いて残す。どちらにするかは開発者の決め。
- **触る所:** 本番の公開の型（`AnimationVariableDef::Integer`・`VariableTypeHint::Integer` の欄）。

---

## Nice to have（5 件）

### R1-13 Object の値を毎フレーム 2 回深く複製してから intern を引く
- **重さ:** nice-to-have
- **場所:** `crates/dola/src/runtime/interpolator/mod.rs:31-36`（`intern` は当たっても `value.clone()` を鍵に作る）・`:89`（`transition_value_to_dynamic` が `DynamicValue` を複製）
- **何が悪いか:** 購読した Object の変数 1 つにつき、毎 tick で `DynamicValue` を 2 回複製し、中身全部をハッシュする。Map や Array の値が大きいと、値が変わらないフレームでも同じ手間がかかる。
- **直し:** `intern` を `if let Some(rc) = self.pool.get(&value) { return rc.clone(); }` から始め、外れたときだけ鍵を作る。さらに減らすなら `transition_value_to_dynamic` を参照で引く（`HashMap::get` は `&DynamicValue` で引ける）。
- **触る所:** 本番（速さだけ・振る舞いは同じ）。

### R1-14 外に呼び手が無い公開の口が残る
- **重さ:** nice-to-have
- **場所:** `crates/dola/src/runtime/mod.rs:20`（`pub use InstanceState`＝公開の口がこの型を返さない）・`src/validate/mod.rs:16`（実装 1 つの `trait Validate`）・`src/runtime/facade.rs:181`（`calculate_end_time`）・`:304`（`unsubscribe_all`）・`src/cue/schedule.rs:94`・`:122`・`:270`（`TimedSchedule::new`・`insert`・`clear`。本番は `to_talk_schedule` の `with_horizon`＋`insert_each` だけ）
- **何が悪いか:** どれも dola の外のクレートの呼び手 0 で、dola のテストのためだけに公開している。`insert` は NaN を debug_assert でしか止めない（release で並びが崩れる）口で、公開のまま置く理由が無い。
- **直し:** 一つずつ決める。`InstanceState` は再公開を消し、状態遷移の表のテストをクレートの中へ移す。`Validate` は `DolaDocument::validate` の固有のメソッドにする。`TimedSchedule` の 3 つは、テスト（`tests/cue/schedule_test.rs` の 74 か所）を `CueSheet` 経由に書き直せるなら `pub(crate)` にする。書き直しの手間の方が大きいなら残してよい。
- **触る所:** 本番の公開の API（外の呼び手 0 なので振る舞いは同じ）。

### R1-15 古い `multiple_triggers_sorted_by_fire_time` は並べ替えを消しても落ちず、新しい 2 本と重なる
- **重さ:** nice-to-have
- **場所:** `crates/dola/tests/trigger/compile_test.rs:76-162`
- **何が悪いか:** トリガーが配列の順でも処理の順でも既に時刻の順なので、`compile/mod.rs` の並べ替えを消しても緑（注 7.1 の言うとおり）。並べ替えは、同じファイルの `late_trigger_earlier_in_array_sorted_by_fire_time`・`same_fire_time_triggers_sorted_by_source_entry_index` が守っている。
- **直し:** 消す。名前の言う「時刻の順」は新しい 2 本が見ている。
- **触る所:** テストだけ。

### R1-16 `sheet_test.rs` の horizon のテストが、ms の格子の値でなく素の和で tick する（注 1.2 の分）
- **重さ:** nice-to-have
- **場所:** `crates/dola/tests/cue/sheet_test.rs:522`・`:525`（`schedule.tick(d + wait_dur ...)`）
- **何が悪いか:** `0.15 + 0.8 = 0.9500000000000001` で tick するので、horizon を格子の 0.95 で出しても素の和で出しても緑。格子の振る舞い自体は `:568-604` の 7.1〜7.4 のテストが守っているので、ここは古い形が残っているだけ。
- **直し:** `schedule.tick(0.95)`（と `0.94`）に書き換える。
- **触る所:** テストだけ。

### R1-17 古びた注と説明文
- **重さ:** nice-to-have
- **場所:** `crates/dola/src/runtime/types.rs:92`（「update() の返却値」＝`update` は 20.2 で消えた。今は `tick` の結果で、`last_result` が返す）・`src/cue/command.rs:62`（`CueTarget::Balloon` が「WaitForChoice を主に消費」と言うが、WaitForChoice は区切り（`BarrierKind`）で、`cue_target_of` の分類に出てこない）・`D1a-V`・`D1b-T`・`D2-T`・`D3-V` などの段の符号（`src/builder.rs:114`・`:140`・`src/compile/resolve.rs:129`・`src/cue/schedule.rs:116`・`:164`・`src/runtime/loop_controller/mod.rs:30`・`:120`・`src/runtime/instance_manager/tests.rs:128` ほか約 30 か所）
- **何が悪いか:** 前の 2 つは事実と違う。段の符号は、もう無い作業の段を指す符号で、読み手には意味が取れない（research #53 の P 番号と同じ種類。P 番号は直したが、D 番号は残った）。
- **直し:** 前の 2 つは事実に合わせて 1 行ずつ直す。段の符号は消し、要る注だけを平易な文で残す。
- **触る所:** 注と説明文だけ。

---

## 判定と数

- **判定:** まず R1-01（即時のセグメントで値が止まる）と R1-02（イージングによっては tick が戻らない）を直す。どちらも 1〜3 行で、今の本番の文書の書き方で当たりうる。次に R1-03 と R1-04。
- **Lean:** −約 20 行（R1-09・R1-12・R1-15）。依存は −0（R1-05 の (a) と R1-03 で `interpolation` の使い所が減る。ただしベジェと残りのイージングがまだ使うので、依存は消えない）。
- **数:** must-fix 4・should-fix 8・nice-to-have 5（合計 17）。上限は設けていない。

## 指摘にしなかったもの（決めとして読んだもの）

- 注 6.3: トリガーのエントリに変数だけ・遷移だけがあると V16t と V7 の 2 件が積まれる。どちらも事実なので誤りではない。
- 注 9.1・13.1: Cancel と Trim は意図して同じ道。Trim 専用の檻が無いのは、値で見分けられないため。
- 注 10.2 の `InstanceManager::transition(gid, Playing)` が Paused から戻すと `pause_start` が残る件: 呼び手が無い（facade は Created からしか呼ばない）ので、今は起きる道が無い。私的な関数の罠として覚えておく程度。
- 注 19: 本体は長さを 0 へ切り上げない。負の長さは本番の台本に届かない。
- `CueSheet` を serde で読むと `cues()` が並べ替えられない件: 本番で読む所が無く、`to_talk_schedule` は並べ直すので、振る舞いの差は 0。
- `InterruptionPolicy` の検出が時刻の重なりを見ない件: 要件 11.8 の決め。

## 調べなかったこと

- `cargo test` も clippy も回していない（読むだけの作業）。R1-01・R1-02・R1-04 は、コードを読んで出した具体例で、テストを書いて赤を立ててはいない。
- tests/ の約 16,000 行は、全部の行を一字ずつは読んでいない。弱い assert の型（`_changes`・`_ => {}`・`is_empty` を終わりの証拠に使う形・古い注）を grep し、当たったファイルと、トリガー・ループ・horizon のテストを読んだ。
- 外のクレート `interpolation` は、ease.rs と lerp.rs だけを読んだ。
