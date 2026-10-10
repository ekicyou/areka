# ギャップ分析: areka-P0-seriko-trigger-intervals

> 2026-10-10・`kiro-validate-gap`。入力は確定済みの `requirements.md`・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）。対象の木はワークツリー `claude/seriko-trigger-intervals-490874`（main `226109e8` の上）。本書は判断の材料と選択肢を出すもので、決定は設計の段で行う。

## 0. 要約

- **読み手**: `Interval`（`crates/areka-parsers/src/shell/model.rs`）は `#[non_exhaustive]` で、`normalize_interval`（同 `decode.rs`）は `talk,数値`・`periodic,数値` を語だけ `Other("talk")`／`Other("periodic")` に写し**数値を捨てている**（要件 1 の欠落はここ 1 か所）。腕を足しても、`Interval` を見る本番の関数は seriko の表のほかに `is_always_interval`（compose `nesting.rs`）・`is_bind_interval`（compose `plan.rs`）の 2 つだけで、どちらも `matches!` なので答えが変わらない＝「読み手では `Interval` に腕を足すだけ」の約束は成り立つ。
- **再生の芯は全部ある**: 「引き金が来たら開始の時刻を覚え、刻みごとに経過を `frame_at` で引く」という形（`LoopRuntime::on_tick` の (3)・`PartClocks::look`）が `random` のためにすでに在り、`-1` の停止・末尾の保持・再生中は始め直さない・経過は「今 − 開始」で丸めない、の決まりは**そのまま使える**。足りないのは「いつ始めるか」を決める 3 つの引き金の判定と、その状態（面に入った時刻・文字の到着の時刻・1 回だけの印）。
- **文字の到着はもう届いている**: `CuePlayer` は全 cue を全 sink へ放送し、seriko の `handle_message` は `Text` を `Some(CueTarget::Balloon)` の腕で `debug!` して捨てている。`TalkCue` は `at`（台本の秒）・`duration`（文字数 × 50 ms）・`Text(String)` を持つので、書記素クラスタで割れば「i 文字目が現れる時刻」は seriko の中で求まる（`areka_sakura::cluster` は seriko から使える）。**ただし時計が 2 つある**（§1.7）。これが設計の最大の分かれ目。
- **部品**: `PartClocks` の `gate`（`Lottery`／`Always`／`Off`）に 3 つの腕を足す形は自然だが、`runonce` には「終わった印」、`periodic` には「見え始めた時刻」が要り、今の `PartAnim`（`Playing`／`Residual`）では足りない。さらに `rebuild` の 1 回目が見えない部品も評価する性質（`seriko-rebuild-hidden-lottery`）が、3 語では「見えない部品で再生が始まる」に化けるので、要件 5.6 のためにここで塞ぐ必要がある。
- **推し**: 引き金の判定と状態を新しい兄弟モジュール（例 `trigger.rs`）に純関数＋小さな状態として置き、`looper.rs`（一番上）と `parts.rs`（部品）は呼ぶだけにする混成案（§3 の C）。規模 M・リスク中（時計の写し方と部品の門の 2 点が判断の芯）。

## 1. 今あるもの（現状調査）

### 1.1 読み手（`crates/areka-parsers/src/shell/`）

- `Interval`（`model.rs`）: `Bind`・`Random{k}`・`BindRandom{k}`・`Other(Box<str>)`。`#[non_exhaustive]`・`Clone, Debug, PartialEq`。`Other` は**語だけ**（第 2 欄）を持ち、第 3 欄（数値）は持たない。
- `normalize_interval(fields)`（`decode.rs`）: `bind`／`random`／`bind+random` だけ型へ写し、それ以外は `Other(field[1])`。数値は `field_u32`（欠落・非数値は 0）で読む。**数値の有無・0・非数値の記録は出さない**（読み手は失敗しない・パニックしない・`tracing` の呼び出しが 1 つも無い層）。
- 直書きの範囲: 本番で `Interval::` を綴るのは `decode.rs`・`model.rs` のほかに `areka-emo-atlas/src/manifest.rs`（`Interval::Bind` を 1 か所組む）・compose の `is_always_interval`／`is_bind_interval`・seriko の `table.rs` の match だけ。テストは `table.rs` 内蔵と兄弟（`table_*_tests.rs`）・`decode_tests_animation_tests.rs` ほか。
- 既存のテスト: `table_interval_words_tests.rs`（`sometimes`／`rarely` の読み替え）・`table.rs` 内蔵の「`Other("runonce")` は非採録で `vocab="runonce"` が `debug!` に載る」檻 ← **3 語を採ると、この檻の `runonce` の例は書き替えが要る**（別の非駆動の語へ替える）。

### 1.2 seriko の表（`crates/areka-seriko/src/table.rs`・779 行）

- `LoopTrigger`: `Random{k}`・`BindRandom{k}`・`Always{period_ms, laps}`。`#[non_exhaustive]` ではない（`Copy`・`Eq`）。腕を足すと `looper.rs`（抽選の輪・`start_top_always`・`top_has_always`・`put_top_always`）・`parts.rs`（`gate`・`is_finite`）・テストの match にコンパイル時の漏れが出る（網羅 match なので漏れは見つかる）。
- `AnimationTable::from_world_and_films`: 面を昇順に回し、`interval` の match → `k==0` の `warn!` → コマの整列 → 空の `warn!` → `always` の周期、の順。**3 語の `debug!`／`warn!`（要件 7.1・7.2）を置く場所はこの match と縮退ガードの並びにそのまま足せる**。
- 門: `is_continuous()`（`has_animated_parts || has_always`）。`LoopRuntime::refresh` はこれが偽なら即 `None`、`on_tick` の (3) は「再生中が無い・一番上に `always` が無い・動く部品が見えない」slot を飛ばす。**3 語を持つ表はこの門を通れないので、`has_triggers`（あるいは語ごとの印）を表に足し、`refresh` と `on_tick` の門に加える必要がある**（要件 8.1 の「無いシェルで仕事を増やさない」はこの門で担保する）。
- バルーンの表はスコープ別（`balloon_tables: BTreeMap<ActorKey, AnimationTable>`）。`talk` の cue はスコープ付きで届くので、そのスコープのバルーン表を引くだけで要件 5.5 が成り立つ。

### 1.3 刻みと再生（`looper.rs`・682 行／`timeline.rs`・564 行）

- `on_tick(now_ms, states)`: (1) 単調性 → (2) 1000 ms 境界を跨いだ刻みだけ抽選（`LoopTrigger::Always` は乱数の前で `continue`）→ (3) 進行（`playback: HashMap<(scope, slot), HashMap<anim_id, Playback{started_at_ms}>>`・`frame_at(frames, now − started)`）→ (4) `commit_pattern`。
  - **3 語の再生は (3) にそのまま乗る**: `playback` に `Playback{started_at_ms}` を入れれば、`Pending`→`Active`→`FinishedResidual`／`Stopped` と `-1` の消去・末尾の保持・`info!` が `random` と同じに出る（要件 5.1・5.2）。再生中は `playback` にエントリが在るので「始め直さない」（要件 3.3・4.7・5.3）も `contains_key` 1 つ。
  - 乱数は (2) の `should_fire` でだけ消費する。3 語は (2) に入れなければ乱数を消費しない（要件 5.7・8.2）。
- `refresh(scope, slot, at_ms, states)`: `\s`・`\b`・着せ替え・窓の知らせの **4 か所**から呼ばれる。`always` の時計を出来事の時刻で作る場所＝`runonce` を始める候補の場所だが、**着せ替え（要件 2.4）と窓の知らせでは `runonce` を鳴らしてはならない**ので、「面に入った」ことを区別して渡す必要がある（§6 議題 3）。
- `on_surface_changed(scope, slot)`: 面の切り替えで当該 slot の `playback` を全部捨てる。呼ぶ側（`actor.rs`）は `apply` が `Changed` のときだけ呼び、その直後に `refresh` を呼ぶ＝「切り替え → 捨てる → 新しい面で始める」の順はもう在る。
- `event_ms()`: 時計（`SerikoClock`）が在れば今の時刻、無ければ `None`（`refresh` は直前の刻みの時刻で代用）。本番は `spawn_seriko_clocked` で `GetTickCount64` を渡す（`crates/areka/src/emo2_boot/mod.rs` の `tick_count_ms`）。
- `timeline.rs` の純関数: `frame_at`／`current_frame_index`／`lap_of`／`always_at`。`periodic` の「何周目か」は `lap_of(now − entered, period)` と同じ割り算で求まる（丸めない）。

### 1.4 面の状態（`state.rs`・728 行）

- `ScopeStates::apply(scope, Show(id))`: 既に `Shown(id)` なら `Unchanged`、それ以外（未知・`Hidden`・別の番号）は `Changed`。**これが要件 2.1・2.3・2.5・2.6 の判定そのもの**（同じ面の再指定は `Unchanged`＝鳴らない・非表示からの復帰は `Changed`＝鳴る）。`apply_balloon` も同型。着せ替えは `apply_bind`→`commit_bind` で `apply` を通らない（要件 2.4）。
- `stage_slots()`: 進行の対象の (scope, slot, sid, open)。シェルは表示中だけ、バルーンは面の番号が分かる全スコープで `open` は窓の知らせ。`periodic`／`talk` の「非表示なら鳴らさない」はこの列挙と `open` で判定できる。
- 面の切り替えの時刻そのものは今は覚えていない（`ScopeState::Shown(u32)` に時刻は無い）。`periodic` の起点（要件 3.1）と `talk` の数え直し（要件 4.4）には「この slot にこの面で入った時刻」が要る。置き場の候補: `LoopRuntime` の新しい表（`playback` と同じ鍵）か、`ScopeState::Shown` に時刻を足す（`state.rs` の `PartialEq` を使う比較が `Shown(id) == Shown(id)` で時刻を含むと壊れるので、同居させるなら別の欄）。

### 1.5 部品（`parts.rs`・625 行）

- 時計の鍵は (scope, slot) × (`PartKey`, anim id)、値は `PartAnim::{Playing{started_at_ms}, Residual{frame_index}}`。
- `gate(anim, binds)`: `Lottery(k)`／`Always{..}`／`Off`。`advance`（刻み）と `refresh`（切り替え直後・抽選しない）の 2 本の閉包が同じ `gate` と `look` を使う。
- `rebuild`: `pattern.clear_parts()` → `visible_parts` → 未評価の部品を評価 → 増えなくなるまで → **評価したが最終的に見えない部品のコマを外す**（`evaluated.len() > visible.len()` の枝）。ここが `seriko-rebuild-hidden-lottery` の性質（見えていない部品も 1 回目は評価する＝乱数が回る）。
  - 3 語を `gate` に足すと、この 1 回目の評価で**見えない部品の `runonce` が始まり、`periodic` の起点が立ち、`talk` が数え始める**。コマは外されるが時計は残るので、後で見えたときに「途中のコマから」になる（要件 5.6・5.9 に反する）。
- 見えなくなった部品の時計: 回数つきの `always` だけ `drop_unseen_finite` で捨てる。終わりなしの `always`・抽選の `Playing` は残す（見えなかった間も進んでいた扱い）。**3 語は「見えなくなったら止め、再び見えたら新しい起点」（要件 5.9）なので、見えなくなった時点で捨てる側**＝`drop_unseen_finite` の判定 `is_finite` を「捨てる種類か」に広げるのが最小の変更。
- `runonce` の「1 回だけ」は、再生が終わると（`Residual` に移るか `Stopped` で消えるか）次の刻みで時計が無い＝また始まる、になるので**終わった印（`Done` 相当）が要る**。`Residual` は `-1` 無しの末尾だけに来るので代用できない。

### 1.6 文字の cue が seriko へ届く形

- `TalkCue { at: f64（台本の秒）, actor, command, duration: f64 }`（`crates/dola/src/cue/command.rs`）。`Text(String)` の `duration` は `text_playback_duration`（`crates/areka-sakura/src/duration.rs`）＝`cluster_count × 50 ms` を秒にした値。`compile.rs` は `Instruction::Text` 1 つにつき cue 1 つ（文字列の塊ごと・`\w` などは別の `Wait` cue）。
- 配送: `CuePlayer::tick(t)`（`crates/dola/src/cue/runtime.rs`）が「時刻の来た cue」を**登録順に全 sink へ放送**する。seriko の `SerikoSink` は全 cue を `SerikoMsg::Cue` として inbox へ入れ、`handle_message` の `cue_target_of` で `Text` は `Some(CueTarget::Balloon)` → `debug!` して `Continue`。**ここが文字を読む入口**（`actor.rs` の `Some(other) =>` の腕）。
- `CueSink::preview(upcoming)`（登録時に残りの cue 列を 1 度渡す）は `SerikoSink` が上書きしていない（既定は何もしない）。先渡しの列を使えば `talk` の時刻を先に全部計算できるが、本 spec は「届いた cue を読む」（要件の Boundary）で足りる。
- バリア（`\x`・選択肢）: `TimedSchedule::notify_barrier_resolved` はバリアの印を外すだけで、後続 cue の `at` はずらさない。つまり**バリアの後の cue の `at` は台本の上の時刻のままで、実際に届く時刻はそれより遅い**。emo-text は `TalkClock` の `epoch = max(now − at)` でこれを吸収している（§1.7）。
- 文字でない知らせ（要件 4.10）: `NewLine`・`Clear`・`ClearAll`・`Choice`・`Cursor`・`Custom`・`Wait` は別の variant なので、`Text` の腕だけを読めば数えない形になる。`Choice{text}` は emo-text ではグリフとして現れるが、要件 4.10 は数えないと決めている。

### 1.7 時計の事情（設計の芯）

| 誰 | 時計 | 単位・刻み |
| --- | --- | --- |
| seriko の刻み・出来事の時刻（`SerikoClock`） | `GetTickCount64` | ms・刻みは `LoopTickerConfig` 既定 16 ms。`GetTickCount64` の分解能は OS 既定で 10〜16 ms |
| 台本の再生（`drive.rs` の `player.tick(t)`） | `dola::runtime::clock::now`（QPC 秒） | 刻みは ghost ticker `base_interval` 既定 50 ms＝cue は 50 ms 単位の束で届く |
| 文字の層（emo-text）の表示 | `TalkClock::talk_time(frame_now)`・`epoch = max(QPC_now − cue.at)` を cue 到着ごとに更新 | UI フレーム（60 Hz） |

- emo-text の i 文字目の時刻（`crates/areka-emo-text/src/state.rs` の `RevealSchedule::extend_chunk`）: `r_i = max(r_{i-1} + interval, chunk_start)`、`interval = duration / glyph_count`、先頭は `r_0 = cue.at`。全部**台本の秒**で、実際に見える壁時刻は `epoch + r_i`。
- seriko は `cue.at` を seriko の ms 時計へ写す手段を今持たない。選択肢は §3.2。どれを採っても「文字の層と**同じ時計の値**で同じフレームに揃う」ことは構造上保証できない（時計の種類も分解能も違う）。要件 4.5 の「同じフレームで始める・1 フレーム後に始めない」は、**seriko の側で「文字の時刻 t_k を越えた最初の刻みで、経過を t_k から数えて始める」（状態の形で解く・刻みの遅れは経過に数える）**ことで満たすのが現実的で、UI への到達の遅れは `random` の再生と同じ経路（`PresentBridge`）になる。
- `GetTickCount64` の 10〜16 ms の粗さ: 50 ms 間隔の文字に対して誤差 1 文字未満。`periodic` は秒単位なので影響なし。研究項目（§5）に残す。

### 1.8 台帳・検体・テストの置き場

- 台帳 `doc/ukadoc-coverage/ledger/assets.toml`: `talk_2c_6570_5024:1`・`runonce:1`・`periodic_2c_6570_5024:1` は `vocabulary-only`・担当なし・優先度 A6。`always:1` は `degraded`・担当が本 spec（要件 10.3 で触らない）。`sometimes`・`rarely` の行の注記に「`runonce`・`periodic,数値` などは非駆動」とあり、本 spec の着地でこの文も古くなる（直すかは議題 §6-9）。整合の見張りは `crates/ukadoc-survey/tests/consistency.rs` 系（`perturb.rs` が `assets.toml` を読む）。
- 検体の前例: `crates/areka-emo-compose/tests/fixtures/animated-playback/`（`shell/`＋`balloon/`・単色の PNG を標準ライブラリで作った・README に面の役の表）。E2E の前例: `crates/areka/src/emo2_boot/film_playback_e2e_tests.rs`（`load_shell_target` で本物の読み手 → `EmoWorld::build` → `AnimationTable::from_world` → 時計つきの本物の seriko アクター → `\s[0]` → 刻み → 出た `PatternState` を `Composer` で合成）。実機の前例: `sample-ghost-kit` の `SampleRoot::add_shell_copy(from, to_folder, name)` で emo2 の写しに検体のシェルを足し、`target\` の下で起こす（`animated-image-playback` の research に手順）。実機の縛りは `tests/emo2_real_run.rs` の `AREKA_EMO2_REAL_RUN`。
- 同梱の検体 4 本（emo2・claudia・konnoyayame・R_POST_and_KOMAINU）に `talk`・`runonce`・`periodic` は 0 件（brief の再測定）。
- テストの作法: 兄弟ファイル・1,000 行以下（`tests/file_length_guard_test.rs` が見張る）・ログは `log-capture-kit`。`looper_tests.rs` は `rt.on_tick(now, &mut states)` の直呼びで刻みを注入する型、`actor_clock_tests.rs`／`film_playback_e2e_tests.rs` は `SerikoClock` を `Arc<AtomicU64>` で注入する型。

## 2. 要件 → 資産の対応（Missing／Unknown／Constraint）

| 要件 | 使える資産 | 欠けているもの | 印 |
| --- | --- | --- | --- |
| 1 読み込み | `Interval`（non_exhaustive）・`normalize_interval`・`field_u32` | `Talk{n}`／`Periodic{secs}`／`Runonce` の腕と数値の保持。無効な数値のとき「元の綴り」をどう運ぶか（読み手は記録を出さない層） | Missing・Constraint |
| 2 runonce | `apply` の `Changed`／`Unchanged`・`on_surface_changed`→`refresh` の順 | 「面に入った」と「着せ替え・窓の知らせ」を `refresh` で区別する口。終わった後に再び始めない印は一番上では不要（`playback` を捨てるのは切り替えだけ・終わっても再抽選の輪に入らない）が、**部品では要る** | Missing |
| 3 periodic | `lap_of`・`frame_at`・`playback`・`stage_slots` の `open` | 「面に入った時刻」の表・刻みごとの「周を跨いだか」の判定・一番上に `periodic` が在る slot を (3) の門に通す印 | Missing |
| 4 talk | `Text` の cue（`at`・`duration`・文字列）・`areka_sakura::cluster::clusters`・`stage_slots`・`playback` | `Text` を読む腕・スコープごとの文字の時刻の列（塊で持てる）・時刻の写し（§3.2）・面の切り替えで 0 に戻す・隠れている間は捨てる | Missing・Unknown（時計） |
| 5 共通 | `frame_at`・`-1`・`info!`・`gate`／`look`・`rebuild` | `LoopTrigger` の 3 腕・`gate` の 3 腕・部品の「見え始めた時刻」「終わった印」・見えない部品で始めない形（`rebuild` の 1 回目） | Missing・Constraint（hidden-lottery） |
| 6 時刻 | `SerikoClock`・`event_ms`・「経過＝今 − 開始」 | 文字の時刻を seriko の時計へ写す決まり | Unknown |
| 7 記録 | `from_world_and_films` の `debug!`／`warn!` の並び・`info!` の発火記録 | 3 語の採録の `debug!`・無効の `warn!`・`talk` の開始を文字ごとに出さない水準の決め | Missing（小） |
| 8 変わらない | `is_continuous()` の門・`disabled()` | `has_triggers` の印と、無い表で `Text` を今までどおり `debug!` だけで捨てる早期分岐 | Missing（小） |
| 9 テスト | `looper_tests` の刻み注入・`actor_clock_tests` の時計注入・`parts_tests`・`log-capture-kit` | 偽の文字の到着＝`SerikoMsg::Cue(Text)` を注入するテスト（`actor_dispatch_tests.rs` に `Text` の組み立て関数が既に在る） | — |
| 10 台帳 | `assets.toml` の 3 行・見張り | 3 行の書き替えと note | — |
| 11 検体・実機 | `animated-playback` の検体の型・`add_shell_copy`・`film_playback_e2e_tests` の型 | 口パクの検体（新規）・置き場 | Missing |

## 3. 実装の案

### 3.1 全体の形

**A. 既存の 3 ファイルへ直書き（伸ばす）**
- `LoopTrigger` に `Runonce`／`Periodic{period_ms}`／`Talk{n}` を足し、`looper.rs` の `on_tick`・`refresh` と `parts.rs` の `gate`・`advance`・`refresh` に判定を書く。文字の列は `LoopRuntime` の欄。
- ○ ファイルが増えない・既存の流れに沿う。
- ✕ `looper.rs`（682）・`parts.rs`（625）が 1,000 行へ近づく。判定が 2 本の閉包（`advance`／`refresh`）と一番上の 2 か所（`on_tick`／`refresh`）に **4 重に**散る。

**B. 新しい兄弟モジュールへ分ける（新設）**
- `trigger.rs`（仮）: 引き金の時計の純関数と状態。例: `enum TriggerClock { Runonce{done: bool}, Periodic{entered_at_ms, last_lap}, Talk{..} }` と `fn poll(trigger, clock, now_ms, playing: bool) -> Start(Option<u64 /*開始の時刻*/>)` のような「今の刻みで始めるか・始めるなら何時に始まったことにするか」を返す関数。文字の時刻の列（`TalkSchedule`：塊ごとの開始・間隔・数・数えた数）もここ。
- `looper.rs`／`parts.rs` は「鍵 → `TriggerClock`」の表を持ち、刻みと切り替えで `poll` を呼び、`Some(t)` なら `Playback{started_at_ms: t}`／`PartAnim::Playing{started_at_ms: t}` を入れるだけ。
- ○ 判断の分岐が 1 か所・純関数で檻に入る（偽の時刻だけ）・一番上と部品で同じ決まりが機械的に揃う（要件 5.4）。
- ✕ 表（`AnimationTable`）の印・`actor.rs` の `Text` の腕・`state.rs` の「面に入った時刻」は結局触る。

**C. 混成（推し）**: B の `trigger.rs` を芯にし、`looper.rs`／`parts.rs`／`actor.rs`／`table.rs` は配線だけ。`timeline.rs` には純関数（例 `laps_between(entered, prev_now, now, period)`）を 1〜2 本足すに留める。テストは `trigger_tests.rs`（純関数）＋`looper_trigger_tests.rs`・`parts_trigger_tests.rs`・`actor_talk_tests.rs`（配線）の兄弟ファイル。

### 3.2 文字の時刻を seriko の時計へ写す 3 案（要件 4.1・4.5・6.1・6.3）

| 案 | 決まり | ○ | ✕ |
| --- | --- | --- | --- |
| **T1 到着の時刻を頭にする** | `Text` が inbox に届いたとき `event_ms()` を読み、`i` 文字目＝`到着 + i × (duration / n)`。`at` は使わない | 最も単純。バリアの後も自然 | 台本の刻みが 50 ms なので到着は最大 50 ms（＝1 文字）遅れる。emo-text は `at` 基準なので、文字と口が最大 1 文字ずれる。塊の継ぎ目で `max(prev_end, chunk_start)` の決まりを自前に持たないと、`\w` 無しの連続した塊が重なる |
| **T2 自前の起点の見積もり（`TalkClock` と同じ式を seriko の時計で）** | cue が届くたび `epoch_ms = max(epoch_ms, now_ms − at × 1000)` を持ち、`i` 文字目＝`epoch_ms + r_i × 1000`（`r_i` は emo-text と同じ式） | emo-text と**同じ式**なので、2 つの時計の差（QPC vs GetTickCount64・到着の経路の差）を除いて揃う。新しい時計は持ち込まない（seriko の時計を読むだけ）・dola／emo-text に触らない | 式を 2 か所に持つ（emo-text の `RevealSchedule` と seriko）。「時計を 1 つ持ち込んだ」と見えないか要確認。新しいトークで起点が前へ飛ぶ性質（単調 max）も写す |
| **T3 台本の時計（`TalkClock`）を seriko へ注入** | `spawn_seriko_clocked` に `Fn() -> Option<f64>`（今の talk 相対秒）を足し、`r_i` と比べる | 文字の層と**同じ値**を見る＝ずれの根が消える | seriko に時計が 2 本になる（刻みは ms・文字は talk 秒）＝要件 6.3「新しい時計を持ち込まない」の読み方しだいで抵触。`crates/areka/src/emo2_boot/mod.rs` の結線を触る（約束の外ではないが本 spec の列に無い）。偽の時計が 2 本になりテストが重い |

いずれも「文字の時刻 `t_k`（k×n 文字目）を刻みが越えたら、`started_at = t_k` で始める」は共通（刻みの遅れは経過に数える・要件 3.4／4.6 の「1 回にまとめる」は「越えた区切りが 2 つ以上でも開始は最新の 1 つ」で満たす）。

### 3.3 `runonce` の鳴らし方 2 案

- **R1 `refresh` に理由を渡す**: `refresh(scope, slot, at_ms, states, reason: Entered | Rebind | Stage)`。`Entered` のときだけ `runonce` を始め `periodic` の起点を置く。呼ぶ 4 か所のうち `apply` の後の 2 か所が `Entered`。
- **R2 別の口 `on_surface_entered`**: `actor.rs` の `apply`→`Changed` の直後（`on_surface_changed` の後・`refresh` の前）に呼ぶ。`refresh` の署名は不変。
- どちらも `at_ms` が `None`（時計も刻みも無い）なら始められない。本番は時計が常に在る。テストは注入で足りる。

### 3.4 部品の門と「見えない部品で始めない」3 案（要件 5.6・5.9・`seriko-rebuild-hidden-lottery`）

- **P1 評価を 2 段にする**: `rebuild` の中の評価では「既にある時計を読む」だけにし、`visible` が確定した後にもう 1 周、見える部品だけに引き金の `poll`（時計の誕生・開始）を回してから欄に書く。抽選の乱数の消費は今までどおり 1 回目（hidden-lottery は別 spec のまま）。
- **P2 `rebuild` の 1 回目から外側の今のコマで見える部品だけを評価する**: hidden-lottery の案 ⑴ を本 spec で一緒に入れる。乱数の消費の並びが変わる（要件 8.2「乱数の消費の並びを変えない」に抵触しうる＝既存の決定論テストの期待値が動く）。
- **P3 見えなくなった部品の引き金の時計を `drop_unseen` で捨てる（P1 と併用）**: `is_finite` の判定を「捨てる種類」（回数つき `always`＋3 語）へ広げる。これだけでも「見えなくなったら止め、再び見えたら新しい起点」（要件 5.9）が成り立つ。

### 3.5 部品の時計の状態（`PartAnim`）

- 今: `Playing{started_at_ms}`・`Residual{frame_index}`。
- 要るもの: `runonce` の「終わった」（`Done`）、`periodic` の「見え始めた時刻と最後に鳴らした周」、`talk` の数え（スコープ単位で一番上と共有できる＝文字の列は部品ごとに持たず、部品は「今の区切りの番号」だけ覚える）。
- 案: `PartAnim` に腕を足す（`Done`・`Armed{entered_at_ms, last_lap}`）か、引き金の状態を別の表（鍵は同じ）に分ける。後者なら `look`／`drop_finite_where` は触らずに済む。

### 3.6 読み手の無効な数値と「元の綴り」（要件 1.5・7.2）

- 読み手は記録を出さない層・失敗しない層。`talk,0`・`talk,abc`・`talk`（欠落）をどう運ぶか:
  - **N1** `Interval::Talk{n: 0}` へ倒す（`Random{k:0}` と同じ流儀）。表の `k==0` と同じ `warn!` を出す。✕ 「元の綴り」（`abc`）は失われる（`warn!` には `talk,0` としか書けない）。
  - **N2** 数値が読めないときは `Interval::Other("talk,abc")`（第 2 欄以降を `,` で繋いだ原文）へ写し、表の `Other` の腕で先頭の語が `talk`／`periodic` なら `warn!`、それ以外は今までどおり `debug!`。✕ `Other` に初めて `,` 入りの値が現れる（今は語だけ）。○ 綴りが残る・既存の `Other` の意味（原文の忠実な転記）に沿う。
  - **N3** `Interval::Talk{n: u32, raw: Box<str>}` のように綴りも持つ。✕ 型が重い・`Copy` でない値が増える。
- `periodic` の数値の単位は秒。ms への写しは `secs × 1000`（丸めなし）。上限（`u32` 秒 × 1000 が `u64` に収まる）は問題なし。

## 4. 規模とリスク

- **規模: M**（brief の 12〜15 タスクと整合）。内訳の目安: 読み手 1〜2・表と印 1〜2・`trigger.rs` 2〜3・一番上の配線（runonce／periodic／talk）3・部品 2〜3・記録と台帳 1・検体と E2E・実機 2〜3。
- **リスク: 中**。
  - 時計の写し方（§3.2）を間違えると「文字と口がずれる」が実機でしか見えない（檻は決まりを固定できても、2 つの時計の差は固定できない）。実機の確かめ（要件 11）で `RUST_LOG` を開けて `talk` の開始時刻と emo-text の `Text cue 適用` の `at`・`interval` を突き合わせる。
  - 部品（§3.4）: `rebuild` の流れを変えると `parts_tests.rs`（836 行）・`looper_parts_*_tests.rs` の期待値が動きうる。P1＋P3 なら `random` の評価の順と乱数は不変。
  - `LoopTrigger` に腕を足すと `Copy`／`Eq` の網羅 match が各所で赤になる（見つけやすい反面、同じウェーブに seriko を触る spec は居ないので衝突は無い）。
  - 文字の列の大きさ: 1 文字 1 要素で持つと長い台詞で伸びる（「ゴーストの台本の大きさを甘く見ない」）。塊（cue）単位で持ち、数え終えた塊は捨てる形にすれば O(塊)。

## 5. 設計へ持ち越す研究項目（Research Needed）

1. `GetTickCount64` の分解能（10〜16 ms）が 50 ms 間隔の文字の区切りに与える揺れの実測（実機の記録で `talk` の開始時刻の差を見る）。
2. T2 を採る場合、新しいトークで `epoch` が前へ飛ぶ場面（`TalkClock` の単調 max と同じ）と、`\x` の後の cue の `at` が実時刻より小さい場面での挙動（emo-text と同じ式なら同じにずれる）。
3. `apply` の `Changed` が「未知のスコープへの最初の `Show`」でも立つこと（起動直後の `\s[0]`）を `runonce` の最初の表示に使える前提の確認（`state_surface_tests.rs` に既存の檻が在るか）。
4. バルーンの面の `periodic`／`talk` で、窓が閉じている間（`open=false`）をどう扱うか（`always` は「時計を作らない・回数つきは捨てる」）。要件 3.2・4.9 の「非表示」に窓の閉じを含めるかは正典が沈黙＝裁量として台帳の note に書く候補。
5. `Choice{text}` を数えない（要件 4.10）ことが、選択肢だけの台詞で口が動かない見た目になる点の実機確認（SSP の挙動は測らない方針なので、ukadoc の文言「バルーン内にテキストが表示されていく時」の読みで決める）。
6. 口パクの検体の置き場: `crates/areka-emo-compose/tests/fixtures/`（前例と同じ・compose の下だが compose のソースには触れない）か、seriko の下に新しく切るか。E2E は `crates/areka/src/emo2_boot/` の兄弟（前例 `film_playback_e2e_tests.rs`）。

## 6. 設計判断の議題（要件討議へ）

1. **文字の時刻の写し方**: T1（到着の時刻）／T2（自前の起点の見積もり＝emo-text と同じ式を seriko の時計で）／T3（台本の時計を注入）。推しは T2（emo-text と同じ式・新しい時計を持ち込まない・dola／emo-text に触らない）。T3 は要件 6.3 の「新しい時計」の読み方に関わる。
2. **「同じフレーム」の達成の形**: seriko は「区切りの時刻 `t_k` を越えた最初の刻みで `started_at = t_k` として始める」までを約束し、画面への到達は `random` と同じ経路（1 刻み 16 ms＋提示）とする。これで要件 4.5 を満たすと読んでよいか（文字の層と同じ UI フレームでの同時描画までは構造上保証しない）。
3. **`runonce` の口**: R1（`refresh` に理由を渡す）／R2（別の口 `on_surface_entered`）。
4. **部品の門**: P1（評価を 2 段にし、見えると確定した部品だけ引き金を起こす）＋P3（見えなくなった部品の 3 語の時計を捨てる）を本 spec で入れ、`seriko-rebuild-hidden-lottery` は「乱数の並び」の話だけ残して別 spec のままにする、でよいか。P2（1 回目から見える部品だけ）は要件 8.2 の乱数の並びを動かすので避ける。
5. **無効な数値の運び方**: N1（`n: 0` へ倒す・綴りは残らない）／N2（`Other("talk,abc")` へ原文を残す）／N3（型に綴りを足す）。推しは N2（読み手は記録を出さない層のまま・表で `warn!`・綴りが残る）。
6. **`periodic` の「周を跨いだ」の数え方**: `entered_at` からの `lap_of` で「前の刻みの周 < 今の周」なら 1 回だけ、開始の時刻は「今の周の頭」（最新の境目）。前の周を覚える（`last_lap`）か、再生中かつ `playback` の `started_at` から逆算するか。
7. **`talk` の文字の列の持ち方**: 塊（cue）単位（開始・間隔・数・継ぎ目は `max(prev_end, chunk_start)`）で持ち、数え終えた塊を捨てる。1 文字 1 要素は持たない。面の切り替えで列も数も捨てる（要件 4.4）。隠れている間は塊を受け取らない（要件 4.9）。
8. **一番上の `talk` と部品の `talk` の数え**: 文字の数はスコープごとに 1 つ（要件 4.3）で、一番上と部品は同じ数を見る。部品が後から見えた場合、区切りの数え直しは「部品が見え始めた時点の数から」にするか、スコープの数をそのまま使うか（要件 5.9 の「見え始めた瞬間を起点」の `talk` への当てはめ）。
9. **台帳の `sometimes`／`rarely` の note の古くなる 1 文**（「`runonce`・`periodic,数値` などは非駆動」）: 本 spec で直すか（要件 10.1 は 3 行だけを挙げる）。
10. **検体の置き場と形**: `animated-playback` と同じ `crates/areka-emo-compose/tests/fixtures/<名>/`（シェルだけで足りるか・バルーンの面の `talk` も検体に置くか）。E2E の相手は `film_playback_e2e_tests.rs` の型（本物の読み手＋本物の seriko＋偽の時計）。
