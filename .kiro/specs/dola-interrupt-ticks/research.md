# Gap Analysis: dola-interrupt-ticks

- 測った木: ワークツリー `kiro-next-a8a376`（ブランチ `claude/dola-interrupt-ticks-d6fdc4`・HEAD `0227087b8`＝main `24ddd93a0` に spec の初期化を足した所）。2026-11-26。
- 調べ方: リポジトリの全体に及ぶので、5 本の調べ（dola の cue の層／dola の storyboard の層・runtime・時計／下位の使い手 4 crate／上位の使い手 5 crate／ノベルの 5 つと文書）を先に行い、この文書はその結果をまとめ直した。調べの控えは一時の置き場に在って残らないので、設計が要る数・行番号・一覧は、この文書に写してある。
- cargo は回していない（読むだけ）。数は grep の行数とファイル数。「約」と書いた数は、時刻でない小数も含む上限の数。
- 行番号は測った時点の物。

## 0. 自分で確かめ直した主張

判断の根拠になる主張と、調べの間で食い違った所は、コードを開いて確かめ直した。

| 確かめた事 | 結果 |
|---|---|
| crate の依存の辺（各 `Cargo.toml`） | 下の 2.1 のとおり。9 つの外で `dola` を引く crate は 0。`areka-input` は `dola` に依存しない（`areka-sakura`・`areka-runtime` の再公開の経由だけ） |
| `dola::runtime::clock::now` の呼び出し | 本番 3（wintf `ecs/world/mod.rs:114`・`:563`、`areka-runtime/src/lib.rs:465`）・テスト 6（`areka-session` 2・本体 4）・説明の文だけ 4。brief の「13」は説明の文を含む数 |
| `dola::cue::sum_on_ms_grid` | `pub`（`crates/dola/src/cue/schedule.rs:16`・`mod.rs` で公開）。dola の外の呼び手は `areka-runtime/src/talk_lifecycle.rs`（本番）と `talk_lifecycle_tests.rs` |
| `present_frame(` の数 | `areka-emo-text` の中 157（src 119・tests 30・examples 8）。外は `areka-input` 3・`areka-runtime` 2・先進坑 1（下の行）。本体の 1 件は説明の文 |
| 先進坑の置き場 | `crates/pilot/examples/pilot-windows-rs-next/` は自分の `[workspace]` を持つ別のワークスペース。`q7-vertical/src/old_side.rs:35`・`:93-94`・`:99` が `areka-emo-text` を path で引いて `TalkCue { at: 0.0, .. }` と `present_frame(.., 100.0)` を書く。`tools/test-all.ps1` に `pilot` の語は 0＝全体テストは、ここを組まない |
| `TalkCue.at` の中身 | 相対（`crates/dola/src/cue/sheet.rs:148` が `cue.start_time` を写す）。説明の文は「発火時刻」（`command.rs:289`） |
| 選択待ちの時間切れ | `BarrierKind::WaitForChoice { timeout: Option<f64> }`（`command.rs:85`）。`areka-sakura/src/compile.rs:524` が `i64` の ms を `ms as f64 / 1000.0` にし、正規化しない（負のまま）。`areka_talk::ChoiceWaiting` の 2 欄（`areka-talk/src/lib.rs:101`・`:109`）と `KanadeMsg::ChoiceWaiting.timeout_directive_secs: Option<f64>`（`areka-kanade/src/msg.rs:242`）を通り、kanade の `choice_deadline`（`schedule/choice.rs:99-114`）が「無い＝既定・0 と負と NaN＝無期限・正＝期限」と読む |
| `areka-talk` の依存 | 0（`Cargo.toml` の説明が「std-only, zero dependencies」）。引くのは `areka-sakura`・`areka-ghost`・`areka-kanade` |
| dispatcher の刻み | `areka-ghost/src/dispatcher.rs:366-373`（最初の刻みを `base_now` にして `(now − base) / 1000.0` を送る）と、逆の写し `:281-285`（`base + round(秒 × 1000)`） |
| 刻みの時計 | `areka-ghost/src/ticker.rs:76-82` と `areka-runtime/src/lib.rs:650-654` が `GetTickCount64`（ミリ秒）。dola の時計とは別の関数 |
| `TalkClock` の見積もり | `areka-runtime/src/talk_clock.rs:34-53`（`epoch = max(epoch, clock() − at)`）・`:56-68`（負は 0） |
| 止めて戻す穴 | `crates/dola/src/runtime/instance_manager/mod.rs:82-98` の `transition` は `pause_start` に触らない。`instance_state.rs:44` が `Paused → Playing` を許す。`transition` の本番の呼び手は `facade.rs:167`（開始）・`:226`・`:495`・`conflict_resolver.rs:118`（終わりの状態）だけ |
| 終わらないループの終わり | `facade.rs:103-104` は「無限ループでも 1 周分の end_time」。`f64::INFINITY` が時刻に出るのは `timeline_manager/mod.rs:257-262`（周回の後の待ちで最後の値を保つ番兵）だけ |
| 丸めを固定するテスト | `loop_controller/tests.rs:226-235`（`1e17`）・`tests/compile/boundary_test.rs:142-180`（`1e308`・`1e300`）・wintf `promotion_tests.rs:132`・`:150`・`:260`（`1e-9`）・`tests/cue/sheet_test.rs`（`to_bits` 14 行）・`schedule/tests.rs:500-514`（`ROUNDS_DOWN`）・`talk_clock.rs`（`1e-9` 8 行） |
| ログの欄を実数で読み戻すテスト | `areka-ghost/src/sink.rs:72`（`at = cue.at`）と同じファイルのテスト `:139-153`・`:191`・`:209`（文字を `f64` に解いて `Some(1.5)`・`Some(3.25)` と比べる） |
| 見どころの決まり | `doc/release-notes/README.md:88-96`。字面は「使う人やゴーストを作る人が何かを直さなければならない変更」 |
| 版の行 | 根の `Cargo.toml:8`（`version = "0.1.0"`）・`:32`（`dola = { version = "0.1.0", path = "crates/dola" }`）。dola は `version.workspace = true`。全体テストの包みの段は `cargo package --no-verify --offline`（`tools/crates-io.ps1:322`） |
| 5 つが両方の brief に在る事 | `dola-interrupt-ticks/brief.md:87-95` と `talk-anchor-interrupt-ticks/brief.md:100-108` が同じ文 |
| git のフック | 共通の `.git/hooks` に見本のほかは無い＝コミットの時に組む仕掛けは無い |

調べの間の食い違いと、採った値:

- cue の層の行番号: 1 本目の調べの `sheet.rs`・`runtime.rs` の行番号は、ファイルをつないだ通し番号だった（`sheet.rs` は +305、`runtime.rs` は +279）。この文書は実のファイルの行番号で書く（`sheet.rs:28` 起点の欄・`:53`・`:59`・`:67`・`:80`・`:102` `clamp_duration`・`:112` `relative_end`・`:138` `to_talk_schedule`、`runtime.rs:170` `tick`・`:181` 時刻の表を進める所・`:195-203` 配る所・`:309` `occupancy_horizon`）。
- 正準のキャリアの合図の使い手の数: brief は 22、1 本目の調べは 26。数え直すと、`dola::cue::test_support` を直に引くのは 16 ファイル、`carrier_cue` の名（包みを含む）を書くのは 35 ファイル（`areka-runtime` 16・本体 8・`areka-seriko` 6・`areka-ghost` 2・`areka-session` 2・`areka-sakura` 1〔関数の名に字が出るだけ〕）。どの数でも、組み立ての本文 1 か所（`crates/dola/src/cue/test_support.rs:8`・`:11` の `at: 0.0`・`duration: 0.0`）が吸う事は変わらない。
- 項目 4 の持ち主: 5 本目の調べは「dispatcher を絶対の刻みへ替えるのは `talk-anchor-interrupt-ticks`」を推し、要件 14 は「この spec が dispatcher を替え、刻みの時計は替えない」と書く。5 章の決め 1 に出す。
- 止めた時刻より前で戻す入力: 5 本目の調べは「符号つきで今のまま」を推し、要件 10 の 4（仮定 5）は「0 として扱う」と書く。5 章の決め 8 に出す。

## 1. 要約

- **在る物**: dola は時刻を外から受ける受け身の作りで、自分の中から「今」を読まない（要件 2 の 8 は今も成り立つ）。cue の層と storyboard の層はコードで互いを参照しない。トークのスレッド（`areka-sakura` の `drive.rs`）は、もう絶対の刻みで動く。正準のキャリアの合図は組み立て 1 か所に寄っている。`QueryUnbiasedInterruptTimePrecise` は今の `windows` 0.62.2 に在る（機能 `Win32_System_WindowsProgramming`）。
- **無い物**: 時刻と長さの型・作者が書く単位から変える口・倍率を掛けて整数へ戻す 1 か所・起点を置き直す口・受け口への知らせ・合図の絶対の発火時刻。どれも新しく作る。
- **型が 2 つでは足りない**: 符号つきの差が要る所が、選択待ちの時間切れの指令・storyboard の文書の負のオフセット・実効の時刻・`TalkClock` と seriko の起点の見積もり・バルーンの待ちの残りに在る。
- **要件のままでは成り立たない所が在る**（4 章・18 件）。大きいのは、負を運ぶ選択待ちの時間切れ・9 つの外の `areka-talk` と `areka-kanade`・刻みの時計の物差し・storyboard の文書の型・丸めそのものを固定するテスト。（13 章で改めた）
- **進め方**: 継ぎを置かずに、下の crate から上へ 1 回で切り替える。前半（型の付け替え・合図と刻みの形は最後の形まで）と後半（振る舞い）に分けて確かめる形を推す（8 章）。並行の単位で刻んだタスクは 50〜53 の見込み。（13 章で改めた）

## 2. 今のコード

### 2.1 crate の依存の辺とコンパイルの順

ワークスペースの members は `crates/*` だけ。`dola` を引く crate と辺（括弧は dev の依存）:

| crate | 引く物（この spec に関わる物だけ） |
|---|---|
| `dola` | ワークスペースの crate は 0 |
| `wintf` | `dola`（`workspace = true`） |
| `areka-talk` | 0（`dola` も引かない） |
| `areka-sakura` | `dola`・`areka-talk` |
| `areka-kanade` | `areka-talk`（`dola` は引かない） |
| `areka-seriko` | `dola`・`areka-sakura`（dev: `dola` の機能 `test-support`） |
| `areka-emo-text` | `dola`・`areka-sakura`・`wintf` |
| `areka-ghost` | `areka-talk`・`areka-kanade`・`areka-sakura`（dev: `dola` の `test-support`。本番は `dola` を引かない） |
| `areka-runtime` | `wintf`・`dola`・`areka-emo-text`・`areka-ghost`・`areka-kanade`・`areka-sakura`・`areka-seriko`（機能 `test-support` が `dola/test-support` を付ける） |
| `areka-input` | `wintf`・`areka-runtime`・`areka-emo-text`・`areka-ghost`・`areka-kanade`・`areka-sakura`（dev: `areka-seriko`・`areka-runtime` の `test-support`） |
| `areka-session` | `wintf`・`dola`・`areka-ghost`・`areka-kanade`・`areka-seriko`・`areka-runtime`・`areka-input`（dev: `areka-emo-text`・`areka-sakura`） |
| 本体 `areka` | 上の全部 |

コンパイルの順（下が先）:

```
dola
 ├─ wintf
 └─ areka-talk ─ areka-sakura ─┬─ areka-seriko
                               ├─ areka-emo-text（wintf も要る）
                               └─ areka-ghost（areka-kanade も要る）
                                   └─ areka-runtime（seriko・emo-text・ghost の全部が要る）
                                       └─ areka-input
                                           └─ areka-session
                                               └─ areka（本体）
```

- `wintf` と `areka-sakura` は互いに独立。`areka-seriko`・`areka-emo-text`・`areka-ghost` は互いに独立（3 つとも `areka-sakura` の後。`areka-emo-text` だけ `wintf` も待つ）。`areka-runtime` より上は 1 本の列。
- `areka-sakura` の `contract.rs:59-62` が dola の cue の型（`TalkCue`・`Cue`・`CueSheet`・`BarrierKind`・`CueSink` ほか）を再公開している。`areka-seriko`・`areka-emo-text`・`areka-ghost`・`areka-input` は、ここから型を受ける。**`areka-sakura` より上は、`areka-sakura` が替わった時に一緒に替わる**（新旧を並べて crate ごとに移す形は、ここで効かなくなる）。
- `cargo test` は例（`examples/`）も組む。`dola` の字を書かないのに壊れる例が在る: 本体の `examples/emo-present/{reconcile,systems,state}.rs`（wintf の `FrameTime.0` を `f64` で読む）・`areka-emo-text` の `examples/emo-text-layer/`・`emo-text-typewriter-demo.rs`・wintf の `examples/typewriter_demo.rs`。

### 2.2 dola の cue の層

置き場と行数: `crates/dola/src/cue/{mod.rs 34, command.rs 305, schedule.rs 279, sheet.rs 158, runtime.rs 322, sink.rs 150, test_support.rs 17}`。隣の単体テスト `command_tests.rs 586`・`schedule/tests.rs 563`・`test_support_tests.rs 22`。結合テストは入口 `tests/cue.rs` と `tests/cue/` の 13 ファイル。benches・examples は dola に無い。

**時刻が公開の口を通る所は 20**（瞬間 9・長さ 10・符号つきの差 1）:

| 所 | 項目 | 種類 | 直列化 |
|---|---|---|---|
| `command.rs:85` | `BarrierKind::WaitForChoice { timeout: Option<f64> }` | 符号つきの差（負が来る） | する |
| `command.rs:251` | `Cue.start_time` | 長さ（起点からの隔たり） | する |
| `command.rs:272` | `Cue.duration`（`#[serde(default)]`） | 長さ | する |
| `command.rs:290` | `TalkCue.at` | 長さ（起点からの隔たり。名前と説明は「発火時刻」） | しない |
| `command.rs:300` | `TalkCue.duration` | 長さ | しない |
| `sheet.rs:28` | `CueSheet.absolute_start_time`（非公開の欄・`#[serde(default)]`・0.0 が「押されていない」の印） | 瞬間 | する |
| `sheet.rs:53`・`:59`・`:67`・`:80` | `with_absolute_start_time`・`absolute_start_time()`・`absolute_fire_time(&Cue)`・`absolute_end_time()` | 瞬間 | — |
| `schedule.rs:16` | `pub fn sum_on_ms_grid(a, b)` | 瞬間＋長さ／長さ＋長さの両方に使う | — |
| `schedule.rs:36`・`:38`・`:43` | `Entry::Payload(f64, T)`・`Entry::Barrier(f64, _)`・`Entry::offset()` | 長さ | — |
| `schedule.rs:103` | `TimedSchedule::with_horizon(start_time, horizon)` | 瞬間・長さ | — |
| `schedule.rs:169`・`:273` | `TimedSchedule::tick(current_time)`・`occupancy_horizon()` | 瞬間 | — |
| `runtime.rs:170`・`:309` | `CuePlayer::tick(current_time)`・`occupancy_horizon()` | 瞬間 | — |

`sink.rs` は `f64` が 0（時刻は `TalkCue` に包まれて `emit`・`emit_with`・`preview` を通る）。`CueSink` の口は `emit`・`emit_with`・`preview`・`choice_wait_began` の 4 つ。起点を受ける口は無い。

**そのまま整数へ移らない所**:

1. `sum_on_ms_grid`（`schedule.rs:16-25`）: ミリ秒の格子で足す継ぎ。注に「時刻が整数になったら消す」。整数では素の足し算になり、関数ごと消える。
2. NaN を「届いた」に倒す比べ（`schedule.rs:168-169`・`:189` の `!(… > current_time)`）。整数では NaN が無く、`<=` で足りる。
3. 手前の守りと同じ時刻の守り（`schedule.rs:170-176`）。整数でそのまま移る。符号なしだと「今 − 起点」は、手前の守りの後でしか引けない。
4. 0 以上の `debug_assert`（`schedule.rs:121-125`・`:139-143`）。長さの型が保証するので消える。
5. 並べ替え（`sheet.rs:42` の `(x + 0.0).total_cmp(..)`）。整数では `Ord` の安定な並べ替えで足りる（−0.0 と NaN の扱いが無くなる）。
6. 長さの clamp（`sheet.rs:102`・有限でなく 0 以上でない物は 0。+∞ も 0＝開発者の決め）。型の中では起きなくなるが、作者が書く単位から変える縁に、同じ決めを 1 か所で持つ事になる。
7. `fold(0.0_f64, f64::max)`（`sheet.rs:115`）。NaN を読み飛ばす性質に頼っている。整数では `.max()`。
8. 既定の 0.0: `CueSheet.absolute_start_time`（押されていない印）・`Cue.duration`（欄の無い古い形は 0）・`TimedSchedule::new` の占有 0。時刻の型に既定の値（0）が要る。
9. 選択待ちの `timeout` は負になる（0 章）。dola のテスト `schedule/tests.rs:258-262` が `Some(-0.001)` を通す。

**直列化の形（今）**: `Cue`＝`{"actor":"sakura","start_time":0.0,"payload":{"Command":{"Text":"hi"}},"duration":0.5}`。`CueSheet`＝`{"absolute_start_time":42.0,"cues":[...]}`。`BarrierKind`＝`"WaitForInput"`／`{"WaitForChoice":{"timeout":3.0}}`（または `null`）。直列化の形に載る時刻は 4 つ（`Cue.start_time`・`Cue.duration`・`WaitForChoice.timeout`・`CueSheet.absolute_start_time`）。`TalkCue` は直列化しない。

- 確かめているのは dev の依存 `serde_json` のテストだけ（`tests/cue/sheet_test.rs:221-279` の 3 件・`command_tests.rs:40-43`）。TOML の口（`DolaDocument::from_toml_str`／`to_toml_string`）は storyboard の文書だけで、`CueSheet` を含まない。
- **dola の外に、直列化した `CueSheet` を読む所は 0**（直列化の道具を使うファイルのうち `CueSheet`・`BarrierKind`・`CueCommand` を参照する物は、dola のほかに無い）。台本は今、プロセスの中でしか渡っていない＝仮定 4（互換の口を持たない）で壊れる物は無い。
- 整数にすると `0.0` 形の古い資産は読めない。TOML の整数は符号つき 64 ビットなので、「終わり無し」を最大値で表す形は TOML に書けない。

**テスト**（`#[test]` 112 本・手で組む `Cue {` 26・`TalkCue {` 4）:

| ファイル | 行 | テスト | 小数の行 | 備考 |
|---|---|---|---|---|
| `src/cue/command_tests.rs` | 586 | 20 | 26 | 直列化の形を文字で固定 |
| `src/cue/schedule/tests.rs` | 563 | 32 | 169 | `Entry::Payload(f64, ..)` を直に組む |
| `src/cue/test_support_tests.rs` | 22 | 2 | 2 | `carrier_cue` |
| `tests/cue/test_support.rs` | 75 | 0 | 2 | 共有の組み立て `barrier`・`text`・`choice`・`wait`（引数は `f64` の秒） |
| `tests/cue/sheet_test.rs` | 615 | 29 | 188 | 組み立て 24 回＋手組み 13 |
| `tests/cue/sheet_sort_test.rs` | 95 | 2 | 17 | NaN 24 件と −0.0 の全順序（全体が整数で作れない入力） |
| `tests/cue/schedule_test.rs` | 23 | 1 | 3 | 丸めの見本（全体） |
| `tests/cue/sink_test.rs` | 143 | 3 | 3 | 手組み 1 |
| `tests/cue/runtime_test_barrier_tests.rs` | 219 | 4 | 29 | |
| `tests/cue/runtime_test_broadcast_tests.rs` | 256 | 7 | 55 | 期待の `TalkCue` を手組み 3 |
| `tests/cue/runtime_test_choice_delivery_tests.rs` | 214 | 3 | 38 | 自前の組み立て 2 |
| `tests/cue/runtime_test_occupancy_horizon_tests.rs` | 80 | 3 | 34 | |
| `tests/cue/runtime_test_preview_tests.rs` | 240 | 6 | 26 | |

### 2.3 dola の storyboard の層・runtime・時計

`f64` の字の数: `src`（cue を除く）258・cue の `src` 44・`tests`（cue を除く）122。

**時刻が公開の口を通る所は 29**（瞬間 10・長さ 12・符号つきの差 7）:

- 文書の型（serde・`lib.rs` で公開）: `LoopOffset::Scalar(f64)`（`storyboard.rs:20`・長さ）・`LoopOffsetRange.min`／`max`（`:30`・`:32`・長さ）・`Storyboard.time_scale`（`:76`・実数）・`StoryboardEntry.trigger_start_offset: Option<f64>`（`:121`・**符号つき**＝検査は有限だけ）・`KeyframeRef::WithOffset.offset`（`:143`・**符号つき**＝`tests/compile/time_resolution_test/at_placement.rs:232-274` が −0.5 を固定）・`TransitionDef.delay`（`transition.rs:42`・長さ・既定 0.0）・`TransitionDef.duration: Option<f64>`（`:45`・長さ）。
- コンパイルの結果（`lib.rs:21-24` で公開・serde 無し）: `CompiledStoryboard.start_time`（`compile/types.rs:20`・runtime はいつも 0.0 で作る＝`facade.rs:101`）・`time_scale`（`:24`・実数）・`total_base_duration`（`:32`・長さ）・`CompiledVariableTimeline.base_duration`（`:45`・符号つきになりうる）・`CompiledSegment.start_time`／`end_time`（`:56`・`:59`・説明は「絶対時刻」だが runtime の中では起点からの符号つきの差）・`CompiledTrigger.fire_time`（`:84`）・`start_offset: Option<f64>`（`:88`・符号つき）・`compile_storyboard(.., start_time: f64)`（`compile/mod.rs:69`）。
- runtime の口: `DolaRuntime::start(name, start_time)`（`facade.rs:86`）・`pause`（`:200`）・`resume`（`:205`・新しい終わりの時刻を返す）・`finish(group_id, deadline)`（`:237`）・`tick(current_time)`（`:269`）・`StartResult.end_time`（`runtime/types.rs:83`）・`RuntimeError::TooShortDurationWithInfiniteLoop { duration }`（`:58`・表示は `{duration:.3}s`）・`MIN_LOOP_DURATION: f64 = 0.1`（`:11`・`pub(crate)`）・`clock::now() -> f64`（`runtime/clock.rs:13`）・`StoryboardBuilder::time_scale`（`builder.rs:83`・実数）。
- `DolaError` の欄: `LoopOffsetNegativeMin`／`Max { value }`（`error.rs:109`・`:112`）・`LoopOffsetRangeInverted { min, max }`（`:117-118`）・`InvalidNumber { value }`（`:159`・時刻と値の両方に使う）。
- crate の中の欄（`instance_manager/mod.rs` の `StoryboardInstance`）: `time_scale`・`pause_accumulated`（符号つき）・`pause_start: Option<f64>`・`finish_deadline: Option<f64>`・`end_time`・`loop_start_time`・`loop_duration`・`loop_offset_min`／`max`。

**倍率と進み具合の計算（要件 3 の 6 が 1 か所にまとめたい所）**:

| # | 所 | 式 | 種類 |
|---|---|---|---|
| ★1 | `runtime/timeline_manager/mod.rs:251-253` | `(now − loop_start_time − pause_accumulated) × time_scale` | 符号つきの差 × 実数 → 実効の時刻 |
| ★2 | `runtime/facade.rs:509` | `fire_time ÷ time_scale` | ★1 の逆 |
| ★3 | `runtime/facade.rs:524` | `total_base_duration ÷ time_scale` | 1 周の実の長さ |
| ★4 | `compile/mod.rs:373` | ★3 と同じ式の写し（あふれの判定） | |
| ★5 | `runtime/timeline_manager/mod.rs:290` | `(effective_time − seg.start_time) ÷ duration` | 進み具合（実数） |
| ★6 | `runtime/loop_controller/mod.rs:54` | `min + eased × (max − min)` | 長さ × 実数 → 長さ |

★1 と ★2 は逆の式で、丸めの向きが合わないと「周回の終わりちょうどのトリガー」がずれる（今は `facade.rs:506-511` の `.min(inst.end_time)` で抑えている）。

**そのほかの、そのまま整数へ移らない所**: `facade.rs:307` の `t.next_down()`（`f64` の 1 つ前の値。整数では 1 を引く）・`facade.rs:340`・`:361`（NaN を捨てる `f64::min` と `is_nan`）・`facade.rs:435`（瞬間＋符号つきの差）・`facade.rs:526`（`duration == 0.0`）・`facade.rs:531`（`duration < 0.1`。0.1 は 2 進で表せない。100ns なら 1,000,000 でちょうど）・`loop_controller/mod.rs:132-136`（**`f64` の吸収**＝巨大な終わりの時刻に足しても変わらない事を見て、終わらないループを止める。整数では吸収が無く、代わりにあふれる）・`timeline_manager/mod.rs:259-260`（負の実効の時刻で `f64::INFINITY` の番兵）・`compile/mod.rs:29`・`:374`（和や割り算のあふれを `inf` で見る。整数では `checked_add`）・`compile/mod.rs:260-263`・`:382-385`（`total_cmp`）・`compile/resolve.rs:248`（瞬間＋符号つきの差）・`validate/rules.rs:54`・`:104`・`:106`（有限かつ 0 以上の検査）・`runtime/types.rs:56`（秒の表示。テストが "0.050" と "0.1" を固定）。

**止めて戻す時間の数え方**: `pause`（`instance_manager/mod.rs:101-106`）が `pause_start` を入れ、`resume`（`:108-128`）が `current_time − pause_start` を `pause_accumulated` と `end_time` に足す。穴は 0 章のとおり今も在る。逆の向きの穴も在る（`transition(gid, Paused)` を直に呼ぶと、`pause_start` の無い止まった状態ができる。`instance_manager/tests.rs:37-42` がそれを正しい物として使っている）。穴を踏むテストは今 0 本（`tests.rs:46-52` の `transition_paused_to_playing` は `transition` だけで止めて戻すので踏まない）。止めている間も、終わりの期限（`finish`）は状態を問わず出来事に入る（`facade.rs:342-344`）。

**時計**: `runtime/clock.rs`（74 行）。`QueryPerformanceFrequency` を 1 度読み、`QueryPerformanceCounter ÷ frequency` を返す。テスト 3 本（単調・正で有限・1 ms 眠った差が正）。モジュールは `#[cfg(target_os = "windows")]`（`runtime/mod.rs:15`）、依存は `[target.'cfg(windows)'.dependencies]`（述語が違うが、今の target では同じ）。

**`windows` クレート**: `Cargo.lock` は 0.62.2。`QueryUnbiasedInterruptTimePrecise() -> u64` は `windows-0.62.2/src/Windows/Win32/System/WindowsProgramming/mod.rs:776`（`api-ms-win-core-realtime-l1-1-1.dll`・失敗の道が型に無い）。機能 `Win32_System_WindowsProgramming` は根にも dola にも無い（`*.toml` で 0 件）。dola は `windows = { workspace = true, features = ["Win32_System_Performance"] }` で、根の機能の一覧を受け継ぐ。

**結線**: storyboard の層は本番に未結線。dola の外で `DolaRuntime`・`TransitionDef`・`StoryboardBuilder`・`DolaDocument` を使うのは wintf の `ecs/dola/mod.rs`（`DolaAnimator`・`tick_dola_animators`）と `tests/ecs/dola_animator_test.rs` だけで、`tick_dola_animators` を本番の schedule に足す所は無い。

**テスト**（「呼」＝`.tick(`・`.start(`・`.pause(`・`.resume(`・`.finish(` の数、「固定」＝時刻の欄を `assert_eq!` で厳密に比べる行）:

| ファイル（行） | 小数の行 | 呼 | 固定 |
|---|---|---|---|
| `src/runtime/facade_tests.rs`（233） | 28 | 5 | 0 |
| `src/runtime/instance_manager/tests.rs`（293） | 29 | 9 | 3 |
| `src/runtime/interpolator/tests.rs`（633） | 140 | 0 | 0（`CompiledSegment` の字面 10 か所の置物の時刻だけ） |
| `src/runtime/loop_controller/tests.rs`（442） | 90 | 0 | 17 |
| `src/runtime/timeline_manager/tests.rs`（620） | 155 | 0 | 0 |
| `tests/compile/**`（10 ファイル） | 計 476 | 0 | 65 |
| `tests/general/**`（9 ファイル） | 計 143 | 2 | 0 |
| `tests/runtime/conflict_resolution_test/*`（4） | 計 147 | 54 | 0 |
| `tests/runtime/facade_test/*`（5） | 計 174 | 77 | 2 |
| `tests/runtime/loop_integration_test.rs`（476） | 132 | 41 | 0 |
| `tests/runtime/loop_offset_test/*`（4） | 計 162 | 18 | 1 |
| `tests/trigger/compile_test.rs`（212） | 43 | 0 | 7 |
| `tests/trigger/event_order_test.rs`（278） | 54 | 16 | 0 |
| `tests/trigger/runtime_test.rs`（854） | 160 | 46 | 0 |
| `tests/trigger/validation_test.rs`（470） | 52 | 0 | 0 |
| `tests/validation/**`（7） | 計 143 | 0 | 1 |

`StoryboardInstance` を字面で組むテストは 3 か所（`instance_manager/tests.rs:7`・`loop_controller/tests.rs:9-37`・`timeline_manager/tests.rs:89-90`）。値を比べる `1e-9`（`loop_offset_compile.rs`・`loop_integration_test.rs` 19 件・`load_start.rs` 6 件）は補間した値の比べで、時刻の型に左右されない。文書の往復のテスト（`tests/general/integration_test/serialization_round_trip.rs`・`tests/runtime/loop_offset_test/loop_offset_serde.rs`・`tests/trigger/serde_test.rs`・`tests/general/core_types_test/document_variable.rs`）は秒の数を固定する。

### 2.4 下位の使い手

**wintf**

- 時計を読む所: `ecs/world/mod.rs:114`（世界を作る時）・`:563`（進める回の頭）。`FrameTime(pub f64)`（`ecs/graphics/core.rs:323-324`・`#[derive(Resource, Default, Debug, Clone, Copy)]`＝時刻の型に `Default` が要る）。
- `FrameTime` を読む wintf の中の本番: `ecs/dola/mod.rs:119-121`・`:181-184`（`DolaAnimator::tick(f64)`）・`ecs/widget/text/typewriter_draw.rs:75`（タイプライター）・`ecs/graphics/visual/systems.rs:248`・`:282`・`:392`（自動の上げ下げの判断へ `now: Option<f64>`）。
- **タイプライター（dola は呼ばないが `FrameTime` を読む）の公開の口**: `TypewriterTalk::new(tokens, start_time: f64)`・`pause(f64)`・`resume(f64)`・`update(f64, ..)`・`start_time() -> f64`（`typewriter/mod.rs:135`・`:150`・`:158`・`:224`・`:210`）・`Typewriter.default_char_wait: f64`（`:54`・既定 0.05）・`TypewriterToken::Wait(f64)`（`typewriter_ir.rs:20`）・`TimelineItem` の `show_at`・`duration`・`start_at`・`fire_at`（`:79-95`）。使うのは wintf の中だけ（本番の `new` は `on_typewriter_add` の 1 か所・時刻 0.0）。そのまま移らない計算: `current_time − start_time`（`mod.rs:152`・`:160`・`:232`。開始より前の刻みで負）・`advance(f64::INFINITY, ..)`（`:179`・skip）・`typewriter_layout.rs:211`・`:216`・`:227` の秒の積み上げ。
- **自動の上げ下げの判断**（`pub(crate)`）: `visual/promotion.rs` の `RuleState` の `since`・`last_now`・`updates`（`f64`）と定数 `*_SECS` 4 本（1.0・3.0・3.0・3.0）。`now.max(p)`（`:80-82`）・`t − s < hold`（`:84`・`:129`・`:141`・`:155`）。
- テスト: `ecs/dola/mod.rs` の中のテスト（時刻の行 約 25）・`tests/ecs/dola_animator_test.rs`（約 8）・`typewriter/tests.rs`（**944 行**。`new` 19・`update` 15・`pause` 5・`resume` 2・`skip` 4・`Wait(` 4・`show_at` ほか 21・`FrameTime(` 3）・`promotion_tests.rs`（`step`／`absorb` 35 呼び出し・時刻の行 約 52）・`promotion_round_log_tests.rs`（約 40）・`systems_promotion_tests.rs:90`・`systems_tests.rs:40`・`tests/graphics/surface_embedding_test.rs:581`・`:691`・`world_tick_gate_tests.rs:269`・`:287`（`FrameTime` の前後を等しいと比べる）。`ecs/world/tick_gate_tests.rs:352` は `dola/mod.rs` の字面の検査（`tick_wake::mark(ANIM)` の行を残せば壊れない）。
- `FrameTime` の、wintf の外の読み手: `areka-runtime` 本番 2（`balloon_visibility_phase.rs:91`・`frame/scale_text.rs:229`）・テスト。本体は 12 ファイル・`areka-session` 2・`areka-emo-text` 3（例）。`FrameTime(` の字は wintf 16・本体 12・`areka-runtime` 2・`areka-session` 1。
- 範囲の外の時計: `Instant::now` が wintf に 94 行（`world/mod.rs:559-563` では `FrameTime` と `TickStart(Instant)` を同じ所で置く＝範囲の内と外が隣り合う）。

**areka-sakura**（28 ファイル・最大 758 行）

- 公開の口: `SakuraMsg::Tick(f64)`（`contract.rs:25`）・`duration::text_playback_duration(&str) -> f64`（`duration.rs:33`）・`ChoiceTimeoutDirective::Secs(f64)`（`compile.rs:495` 近辺・`lib.rs:43` で再公開。読み手は本体の `mcp/check_script_judge.rs:12`・`:52`）・`CompiledTalk.sheet: CueSheet`。
- **作者が書く単位から秒へ変える縁は、ここに集まっている**: `duration.rs:45-46` の `ms_to_secs(ms: u64)`（唯一の変換で、`compile.rs:161`・`:171`・`:418` と `duration.rs:34` が呼ぶ）。例外が 2 つ: `compile.rs:170-172`（`Instruction::Wait(Duration)` を整数の ms にして 1 ms 未満を切り捨てる。注が `shortcut:` と明記。元は `areka-parsers` が `\w`・`\_w` を `from_millis` で作る）と `compile.rs:524`（選択待ちの時間切れ。`i64`・符号つき・飽和）。
- 合図を組む所は `compile.rs:475` の `emit` の 1 か所。起点を押す所は `drive.rs:317`（最初の刻みの時刻）の 1 か所。
- そのまま移らない所: `drive.rs:298-301`（有限でない刻みを `error!` で弾く。整数では要らない）・`compile_sheet_tests.rs:192-196`（`is_finite`）・`:555`（1 ulp ずれる例）。`drive.rs:346` の「前より大きい刻みだけ」は `Ord` で移る。
- テスト（8 ファイルが `dola` を書く）: `SakuraMsg::Tick(` は 101（`drive_lifecycle_tests.rs`・`drive_delivery_tests.rs`・`drive_choice_tests.rs`・`drive_strict_tests.rs`・`drive_choice_wait_began_tests.rs`・`drive_choice_timeout_tests.rs`）。時刻の行は `drive_delivery_tests.rs` 約 86・`drive_lifecycle_tests.rs` 約 78・`compile_sheet_tests.rs` 約 61・`compile_arm_tests.rs` 約 53・`drive_choice_tests.rs` 約 42・`compile_choice_timeout_tests.rs` 約 23。期待値を `text_playback_duration("hello")` や `ms_to_secs(..)` で組む所が 30 か所近く（ミリ秒の格子で丸めを避ける作法＝整数では作法ごと要らなくなる）。`drive_delivery_tests.rs:65` の `same_sheet_started_at_different_times_delivers_cue_at_different_absolute_fire_times` が、起点 10・20 の刻みで正しく配る事を固定している（トークのスレッドは、もう絶対の刻みで動く）。

**areka-seriko**（84 ファイル）

- 公開の口に dola の秒は無い（`TalkEpoch`・`TalkFeed` は `pub(crate)`）。`SerikoMsg::Cue(TalkCue)` が dola の型を載せる。seriko の時計は `GetTickCount64` のミリ秒（範囲の外）。
- **範囲の内と外の継ぎ目が `talk.rs` の 3 式**: `:32`（起点 ms＝今 ms − 合図の秒 × 1000）・`:42`（台本の秒＝(今 − 起点) ÷ 1000、負は 0）・`:50`（台本の秒 → 壁の ms。最も近い 1 ms へ丸め）。起点は最大を取る（`:33-36`）。
- 1 字ごとの間隔: `talk.rs:76-90` の `push_chunk`（`interval = duration_s / count as f64`・`r = max(prev + interval, at_s)`）。説明（`talk.rs:6-12`）が、`areka-emo-text` の `RevealSchedule::extend_chunk` と「同じ式・同じ計算の順」を保つと約束している。
- 触る本番: `talk.rs`（全体）・`looper.rs:382-396`（`observe_cue` の 4 行）・`looper_trigger.rs:163-223`・`settle.rs:97`（本番のコードで `TalkCue { at: 0.0, .., duration: 0.0 }` を手で組む）・`actor.rs:227`（ログの欄）。
- テスト: 共有の組み立て `lib_test_support.rs`（`cue`・`text`＝字数 × 0.05・`clear_all`）・`actor_test_support.rs`（`emote_cue`）・`tests/common/mod.rs`（`emote_cue`・`run_core(.., ticks: &[f64], ..)`）。時刻の行は計 約 230（`talk_tests.rs` 65・`looper_talk_tests.rs` 36＋組み立ての呼び出し 43・`actor_talk_tests.rs` 23・`tests/regression.rs` 15 ほか）。`carrier_cue` だけを使うファイルは触らない。

**areka-emo-text**（198 ファイル）

- 公開の口: `present_frame(runtime, world, talk_time: f64)`（`actor_present.rs:72`）・`TextLayerState::visible_glyphs(&self, &ActorKey, t: f64)`（`state.rs:625`・呼び出し 60）・`RevealSchedule::{times() -> &[f64], visible(t: f64)}`（`state.rs:262-281`）。時計は持たない（いつも外から受ける）。
- 1 字ごとの間隔: `state.rs:494`・`:541`（`interval = cue.duration / glyph_count as f64`）と `extend_chunk`（`:290-298`・`r = max(prev + interval, chunk_start)`）。
- 触る本番: `state.rs`・`actor_present.rs`（`talk_time` の引数 3・読む所 3）・`actor_box.rs:172-233`・`actor.rs:641`・`:693`・`sink.rs:115`（ログ）。
- **テストの組み立てが 1 か所に寄っていない**: `TalkCue {` を組む手助けが約 20 本に分かれている（`actor_test_support.rs`・`state_test_support.rs`・`lookahead_tests.rs:63`・`choice_anchor_tests.rs:23`・`viewbox_draw_live_diff_tests.rs:99`・`:110`・`:120`・`viewbox_draw_test_support.rs:234`・`actor_lookahead_tests.rs:260`・`tests/support/mod.rs:183`・`tests/support/fixture.rs:126`、それに `tests/` の 10 ファイルの自前の `cue`＝`attach_wiring_test.rs`・`draw_readback_test.rs`・`pipeline_test.rs`・`scale_invariance_test.rs`・`viewbox_scroll_test.rs`・`box_attach_test.rs`・`decoration_readback/mod.rs`・`choice_fixture_test.rs`・`line_pitch_readback_test.rs`・`shell_balloon_fixture_test.rs`、例の `scenario.rs:131`・`emo-text-typewriter-demo.rs:95`）。`carrier_cue` は使っていない（dev の依存に dola の `test-support` が無い）。
- 時刻の行は計 約 700（`state_reveal_tests.rs` 59・`state_cue_apply_tests.rs` 55・`actor_choice_contract_tests.rs` 47・`actor_clear_atomicity_tests.rs` 44・`tests/pipeline_test.rs` 30・`actor_runtime_frame_tests.rs` 30 ほか）。
- 例のそのまま移らない所: `examples/emo-text-layer/drive.rs:34`・`:37`（`last_click_time = -10.0` の負の番兵）・`:738-760`（二分と掃引）・`scenario.rs:46`（秒の配列）。

### 2.5 上位の使い手と、9 つの外

| crate | `.rs` | 時刻・合図に触るファイル | 手で組む `TalkCue {` | 本番で時刻が流れる所 |
|---|---|---|---|---|
| `areka-runtime` | 218 | 約 52 | 27 | `talk_clock.rs`・`talk_lifecycle.rs`・`balloon_visibility*`・`hover_inject.rs`・`frame/scale_text.rs`・`lib.rs:465` |
| `areka-ghost` | 66 | 約 18 | 5 | `dispatcher.rs` の 2 か所と `sink.rs:72`（ログ） |
| `areka-session` | 119 | 5 | 3 | 0（`runtime_boot.rs:82`・`:84` が `TalkClock` を渡すだけ） |
| `areka-input` | 48 | 2 | 3 | 0 |
| 本体 `areka` | 307 | 約 30（例 4 を除く） | 6 | 0（時刻はテストと例だけ） |

**areka-runtime の公開の口（秒の `f64`）**: `TalkClock::new(Arc<dyn Fn() -> f64 + Send + Sync>)`（`talk_clock.rs:23`）・`observe_cue(at: f64)`（`:34`）・`talk_time(frame_now: f64) -> Option<f64>`（`:56`）・`now_talk_time() -> Option<f64>`（`:72`）・`TalkLifecycleSignal::DisplayEndAt(f64)`／`TalkEnded { at: Option<f64> }`（`talk_lifecycle.rs:72`・`:80`）・`balloon_visibility::configured_timeout_secs() -> f64`（`balloon_visibility_wait.rs:67`）・`frame::run_text_phase(.., talk_time_override: Option<f64>)`（`frame/scale_text.rs:226`）・`hover_inject` の `frame_time: f64`／`period_secs: f64`（`hover_inject.rs:96`・`:120`・`:160`・`:191`）。

- 本番で `dola` の時計を読むのは `lib.rs:465` の 1 行。合図の時刻を読む本番は `talk_clock.rs:104`・`:110` と `talk_lifecycle.rs:279`（`sum_on_ms_grid(cue.at, cue.duration)`）・`:286-287`（ログ）だけ。**ほかの 7 つの受け口（`change_cue`・`move_cue`・`reload_cue`・`switch_cue`・`user_break_cue`・`window_state_cue`・`zorder_cue`）と `shell_carry.rs` は、合図の時刻を 1 度も読まない**＝本番は取り込みの行のほか変わらない。
- 作者の単位（ミリ秒）から秒へ変える本番の縁: `balloon_visibility_wait.rs:50`（環境変数の待ち）・`:79`（`\![set,balloontimeout,ms]`・説明に「丸めない」）・`hover_inject.rs:128`（周期）。定数 `DEFAULT_BALLOON_TIMEOUT_SECS: f64 = 30.0`（`balloon_visibility.rs:101`）・`MAX_WAIT_SECS: f64 = 3600.0`（`balloon_visibility_phase.rs:151`）。
- **そのまま移らない所**: `talk_clock.rs:36`（`clock() − at`。テストの注入では `clock() < at` が現に起きる）・`:40`（最大）・`:67`（負は 0）・`talk_lifecycle.rs:169`（「まだ見ていない」を `f64::NEG_INFINITY` で表す）・`:279-296`（`end.is_finite()` で捨てて `warn!`）・`balloon_visibility_decision.rs:144-145`・`balloon_visibility_phase.rs:173-190`（`remaining = deadline − now` が負になる・`is_finite`・`Duration::from_secs_f64(remaining.min(3600.0))`）・`hover_inject.rs:96-110`（`!(period_secs > 0.0)`・`(t / period_secs).floor() as u64`）・`frame/scale_text.rs:214-218`（`areka-emo-text` の `times()` の末尾と比べる＝型は下の crate が決める）。
- 表せなくなる入力を確かめるテスト: `balloon_visibility_phase_wake_tests.rs:65`・`:70`・`:75`（INFINITY・NaN）・`balloon_visibility_tests.rs:801`・`:833`（NaN）・`talk_lifecycle_tests.rs:274-275`（NaN・INFINITY）・`hover_inject.rs:352`（NaN の周期）・`talk_clock.rs:316`（`talk_time(-5.0)`）・`balloon_visibility_lifecycle_e2e_tests.rs:85`（`fold(NEG_INFINITY, f64::max)`）。大きい時刻: `strict_judges_tests.rs:173`（`areka-runtime`）と本体の `runtime_tests/strict_judges_tests.rs:195` の `t += 1e9`（10 億秒＝1e16。64 ビットに収まる）。
- テストの家族（並行の割り方の材料）:

| 家族 | ファイル | 中身 |
|---|---|---|
| `talk_clock`・`talk_lifecycle` | 5 | `TalkClock` の本体・時計の注入（`controllable_clock`・`set_now`）。**上の全部の土台** |
| `balloon_visibility*` | 18（`areka-runtime` 17＋本体 1） | 秒の小数の行がいちばん多い（約 400）。3 つに割れる: 判断の芯（`_decision`・`_wait`・`balloon_visibility.rs`・`_tests`・`_box_tests`・`_timeout_*`）／相（`_phase*`）／通し（`_choice_wait`・`_lifecycle_e2e`・`_talk_end`） |
| 変更の受け口（`change_cue*`・`move_cue*`・`reload_cue*`・`switch_cue*`・`zorder_cue*`・`zorder_wiring`・`window_state_cue*`・`frame/window_state_tests`・`user_break_cue*`） | 約 19 | 大半が `carrier_cue`。手組みは `at: 0.0, duration: 0.0` だけ。`window_state_tests` だけ `CuePlayer` と `absolute_end_time()` を使う |
| `frame_*` | 9 | `zero_clock()`・`FrameTime`・`run_text_phase(.., Some(f64))`・`CuePlayer` |
| ghost の `dispatcher*`（4）・`prop_*`／`sink.rs`（4）・`tests/ghost/`（3） | 11 | `SakuraMsg::Tick` の検査・`ChoiceWaiting` の秒（32 行） |
| 本体の `runtime_tests/`（13）・`session_tests/`（6）・`mcp/`（3）・`install_cue*`（2）・`update_cue*`（2）・`examples/emo-present/`（5） | 約 31 | `FrameTime`・`TalkClock`・`CuePlayer`・`run_text_phase`・`DisplayEndAt` |
| `areka-session`（5）・`areka-input`（2） | 7 | `TalkClock::new`・`FrameTime(dola::runtime::clock::now())`・手組み数件 |

- 手で組む `TalkCue {` のうち、0 でない時刻・長さを持つ物（個別に直す）: `talk_clock.rs:347`・`:390`・`:454`・`balloon_visibility_choice_wait_tests.rs:332`・`:338`・`balloon_visibility_lifecycle_e2e_tests.rs:110`・`balloon_visibility_phase_tests.rs:98`・`balloon_visibility_phase_reappear_tests.rs:148-163`・`talk_lifecycle_tests.rs:30-50`・`:553`・`talk_lifecycle_signals_tests.rs:22`・`frame_visibility_integration_tests.rs:115`・ghost の `tests/ghost/spine_e2e_test_tests.rs:122`・`:128`。残りは `at: 0.0, duration: 0.0` だけ。
- `CuePlayer` を直に回すテスト: `balloon_visibility_lifecycle_e2e_tests.rs:79-104`・`frame_attach_tests.rs:456-477`・`frame/window_state_tests.rs:219-288`・`talk_lifecycle_tests.rs:380-554`・本体の `session_tests/mod.rs:166-168`（`play_into`・9 か所から呼ばれる）・`frame_balloon_timeout_notice_e2e_tests.rs:23`。

**9 つの外で触れる物**

- `areka-talk`（依存 0）: `ChoiceWaiting { display_end_elapsed_secs: f64, timeout_directive_secs: Option<f64> }`（`lib.rs:101`・`:109`）。組むのは `areka-sakura/src/drive.rs:447-461`（`player.occupancy_horizon()` をそのまま入れる）、受けるのは `areka-ghost/src/dispatcher.rs:248-300`。
- `areka-kanade`（`dola` を引かない）: `KanadeMsg::ChoiceWaiting { display_end: MonotonicMs, timeout_directive_secs: Option<f64> }`（`msg.rs:233-243`）・`schedule/choice.rs:99-114`・`schedule/steady.rs:171`・`schedule/types.rs:63`・`actor.rs`。この 2 つの名前を書くのは kanade の 18 ファイル（本番 6・テスト 12）・約 32 行。
- 先進坑（0 章）: 別のワークスペースで、全体テストは組まない。`areka-emo-text` の公開の口が替わると、黙って壊れる。

### 2.6 ノベルの 5 つに当たる今のコード

- **本番の起点は、いつも 0 秒**。dispatcher が最初の刻みを基準にして相対の秒を送り、`drive.rs:317` がそれを起点に押す。だから `absolute_start_time`・`absolute_fire_time`・`absolute_end_time`・`occupancy_horizon` は、本番では「台詞の頭からの秒」と同じ値である。
- **起点を動かす口は dola のどこにも無い**。`TimedSchedule` の起点（`schedule.rs:63`）は作る時に 1 回入るだけ（`with_horizon`・`:103`）。`schedule.rs` の `pub fn` は `sum_on_ms_grid`・`offset`・`with_horizon`・`insert_each`・`tick`・`ready`・`current_barrier`・`notify_barrier_resolved`・`remaining`・`is_completed`・`occupancy_horizon` の 11 個で、起点に書くのは `with_horizon` だけ。区切りを解く口（`:219-227`）は、溜めた取り出しを空にするが起点は動かさない。
- **受け口は起点を 1 度も受け取らない**（2.2）。だから文字の層（`TalkClock`）と seriko（`TalkEpoch`）は、届いた合図から起点を見積もり、最大を取る＝起点は前（過去の側）へ動かせない。
- **字の現れる時刻は相対で持つ**（`areka-emo-text` の `RevealSchedule`）。時計の起点が動けば、もう配った字は付いて来る作りである。
- **再生機は、時刻の表を進め終えてから、まとめて配る**（`runtime.rs:181` で進め、`:195-203` で `ready()` を全部の受け口へ）。取り出しごとに何かを挟む場所は、今は無い。時刻の表は中身の型を知らず（`TimedSchedule<T>`）、到達の比べで作った「起点＋相対」の和は捨てている（`schedule.rs:189`）。
- **絶対の発火時刻を出す関数は在る**（`CueSheet::absolute_fire_time`・`sheet.rs:67`）が、受け口へは届かない。本番の呼び手は 0（テストだけ）。
- **意味が変わるテスト**（要件 10〜14 の「変える前の期待」の候補）:
  - 起点の見積もり: `talk_clock.rs` の `new_talk_rebases_epoch_forward`（:237）・`intra_talk_jitter_does_not_lower_epoch`（:286）・`clocked_sink_forwards_cue_unchanged_and_observes_at`（:339）・`clocked_sink_forwards_emit_with_after_observing_time`（:449）、seriko の `talk_tests.rs` の `epoch_is_the_largest_now_minus_cue_time`（:18）。起点を置き直さない間は、今の期待のまま残る（要件 12 の 10）。
  - 占有の終わり: `schedule/tests.rs` の `occupancy_horizon_is_invariant_across_ticks`（:461）は残る。説明の文「刻みでは変わらない」（`runtime.rs:307`・`schedule.rs:270-271`）は「起点を置き直すと動く」に直す。
  - 刻み: `dispatcher_slot_tests.rs` の `tick_relay_converts_absolute_now_to_elapsed_seconds_from_first_tick`（:299）、`dispatcher_choice_tests.rs` の `resolve_choice_relays_id_to_matching_talk_without_touching_slot_or_base`（:127・:147 が「最初は 0.0・500 ms 後は 0.5」を固定）・`choice_waiting_converts_elapsed_secs_to_ms_from_the_tick_base_and_forwards_to_kanade`（:427）。`tick_relay_releases_only_the_cues_due_at_each_elapsed_time`（`dispatcher_slot_tests.rs:286`）は期待の書き方だけ替わる。
  - 止めて戻す: `instance_manager/tests.rs` の `resume_with_time_before_pause_start_shrinks_end_time`（:229-245・`pause_accumulated == -2.0` を固定）・`transition_playing_to_paused`（:37-42）・`transition_paused_to_playing`（:46-52）。

### 2.7 1,000 行に近いファイル

番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`（上限は `workspace_scan/mod.rs:49` の `LINE_LIMIT = 1000`・例外の表に dola は無い）。触るファイルのうち 840 行以上の物（今の行数）:

| ファイル | 行 | 触る量 |
|---|---|---|
| `areka-ghost/tests/ghost/inproc_e2e_test.rs` | 964 | 小（`carrier_cue` 45・`at` の順の検査 1） |
| `wintf/src/ecs/widget/text/typewriter/tests.rs` | 944 | 全面（タイプライターを範囲に入れる時） |
| `areka-runtime/src/frame_test_support.rs` | 939 | `zero_clock()` の 1 か所 |
| `areka-ghost/src/dispatcher_choice_tests.rs` | 908 | `ChoiceWaiting` の 32 行 |
| `areka-emo-text/src/lookahead_tests.rs` | 901 | 組み立てと 21 行 |
| `areka-seriko/src/looper.rs` | 887 | 4 行 |
| `areka-emo-text/src/actor.rs` | 873 | 2 か所 |
| `areka-emo-text/src/actor_present.rs` | 866 | 11 行 |
| `areka/src/runtime_tests/spine_conformance_lap_tests.rs` | 862 | 2 か所 |
| `areka-seriko/src/looper_talk_tests.rs` | 861 | 全面（36 行＋組み立ての呼び出し 43） |
| `dola/tests/trigger/runtime_test.rs` | 854 | 時刻の行 160・呼 46 |
| `areka-runtime/src/balloon_visibility_phase.rs` | 848 | `visibility_wake` ほか |
| `areka-runtime/src/balloon_visibility_tests.rs` | 847 | NaN の 2 件・定数 |

時刻を作る書き方が長いと、整形の折り返しで行が延びる。全面を触る 4 本（`typewriter/tests.rs`・`looper_talk_tests.rs`・`trigger/runtime_test.rs`・`dispatcher_choice_tests.rs`）は、延びたら主題ごとに分ける前提で段取りを組む。cue の層と `areka-sakura` に 850 行以上は無い。

### 2.8 文書

| 所 | 今の字面 | 直す向き |
|---|---|---|
| `.kiro/steering/tech.md:55` | dola の `windows`（`Win32_System_Performance`）・`clock::now()`＝`QueryPerformanceCounter` | 機能と時計の関数（眠りの間を数えない） |
| `.kiro/steering/structure.md:253` | cue の欄の名前と型（`duration: f64`・`absolute_start_time`）・バリアントの数 | 型の名へ（バリアントの数も実の 13 に） |
| `crates/dola/README.md:21` | 「Cue sheets: absolute-time scripts broadcast to subscribers」だけ（秒とも時計とも書いていない） | 要件 16 の 1・2 を足す |
| `crates/dola/src/runtime/clock.rs:3-9`・`cue/sheet.rs:12`・`command.rs:84`・`:289`・`:295`・`cue/schedule.rs:10-25`・`:32`・`runtime/facade.rs:234` | 「f64 秒」「壁時計の絶対秒」「`clock::now() + offset_secs`」 | 型の名へ |
| `crates/wintf/src/ecs/graphics/core.rs:316-322` | 「f64 秒、OS起動時=0秒」「`QueryPerformanceCounter / QueryPerformanceFrequency`」 | 型と時計 |
| `crates/areka-runtime/src/talk_clock.rs:7-12`・`lib.rs:391` | 「talk 起点相対秒」「`FrameTime` と同源の QPC 秒」 | 型と時計 |
| `crates/areka-sakura/src/contract.rs:24-25`・`duration.rs:41-43` | 「talk 起点の elapsed を算出して送る」・ミリ秒の格子の説明 | 要件 14 の後の意味 |
| `crates/areka-kanade/src/msg.rs:229-243` | 「dispatcher が `base_now` で経過秒 → 単調 ms へ換算済み」 | 要件 14 の後の意味 |
| `doc/release-notes/README.md:88-96` | 「使う人やゴーストを作る人」 | crates.io の利用者が字面に入っていない（5 章の決め 10） |
| `doc/crates-io-publish.md` | 公開の版・順・鍵だけ | 壊れる変更の告知の欄は無い |
| `.kiro/steering/roadmap.md:132`・`:385`・`:386` | 「20 を越えたら…切る案が brief に在る」「型の付け替えだけ」「L（14〜20）」・`talk-anchor-interrupt-ticks` の範囲 | 棚卸の持ち物。この spec の要件が決まった後の姿と合わない |
| `doc/COMPAT_ARCHITECTURE.md:42-66`・`doc/` のほか | 秒・時計の字面は 0（ノベルの作り直しのフォルダの外） | 直す所は無い |

brief の古くなった所（今のコードと合わない）: 「`BarrierKind` の `timeout`／`duration`」（`duration` は無い）・「`TalkCue { at, duration }`」（`at` は相対）・「終わらないループの終わりは `f64::INFINITY`」（今は 1 周分）・「`clock::now` を呼ぶ所は 13」（呼び出しは 9）・「直列化の往復」と YAML（今は TOML だけで、`CueSheet` は JSON のテストだけ）・Scope の「型を替えるだけで、振る舞いは変えない」と Out of Boundary（要件 10〜14 と食い違う）・80 行目の「20 を超えるなら storyboard の層を後ろへ切る」と 109 行目の「分割の案」（要件 15 と食い違う）・`emo2_boot` の名（今は `areka-runtime`）・申し送りの行番号（`crates/areka-emo-text/src/talk_clock.rs:38-57` は無いファイル＝今は `crates/areka-runtime/src/talk_clock.rs:34-53`、`dispatcher.rs:392-399` は今 `:366-373`）。

## 3. 要件と今の資産の対応

札: **無い**＝新しく作る／**縛り**＝今の作りが形を縛る／**不明**＝設計で調べる。

| 要件 | 今の資産 | 隔たり |
|---|---|---|
| 1 時刻と長さの型 | ワークスペースに時刻と長さの型は無い（wintf の `api.rs` の CPU 時間の素の `u64` の欄と、`areka-session` のテストの `Ticks` だけ） | **無い**。**縛り**: 符号つきの差が要る所が在る（4 章の 3）・`FrameTime` と serde の既定のために既定の値（0）が要る・「終わり無し」を使うのは終わりの時刻ではなく、周回の後の待ちの番兵と終わりの期限 |
| 2 「今」を読む時計 | `runtime/clock.rs`（QPC の秒）。呼び出しは本番 3・テスト 6。dola は自分の中から読まない（2 の 8 は今も成り立つ） | **無い**: 機能 `Win32_System_WindowsProgramming`。**縛り**: `cfg` の述語が 2 通り。**不明**: Windows の外で組める事を確かめる仕組みが無い（CI は 2 本とも Windows） |
| 3 作者が書く単位からの変え方 | 縁は `areka-sakura/src/duration.rs:45`（1 か所）＋例外 2（`compile.rs:170-172`・`:524`）、`areka-runtime` の 3 か所、dispatcher の 2 式、dola の ★1〜★6 | **無い**: 変える口と丸めの 1 か所・倍率の 1 か所。**縛り**: ★1 と ★2 は逆の対 |
| 4 cue の層の口 | 公開の時刻 20（2.2） | **縛り**: `timeout` は負を運ぶ・`TalkCue.at` は相対・`sum_on_ms_grid` は公開で外に呼び手が在る |
| 5 storyboard の層の口 | 公開の時刻 29（2.3）。本番に未結線 | **縛り**: 文書の型に負のオフセット・終わらないループの終わりは今も有限。**不明**: 文書の型をどちらで持つか（5 章の決め 4） |
| 6 直列化の形 | JSON のテストだけ。外の読み手 0。TOML は storyboard の文書だけ | **無い**: 整数の形と失敗の道。**縛り**: TOML の整数は符号つき 64 ビット |
| 7 使い手の追従 | 2.4・2.5 の数 | **縛り**: `areka-sakura` の再公開で、上は一緒に替わる・`areka-emo-text` の組み立てが約 20 本・9 つの外に 3 つ（`areka-talk`・`areka-kanade`・先進坑） |
| 8 「今」を読む本番の所 | wintf `world/mod.rs:114`・`:563`、`areka-runtime/src/lib.rs:465` | 差し替えは 3 行。**縛り**: `FrameTime` を読む wintf のタイプライターと自動の上げ下げの判断 |
| 9 振る舞いが変わらない事の示し方 | 既存のテスト（dola 112＋storyboard の側・使い手の各家族） | **縛り**: 丸めそのものを固定するテストが在る（4 章の 9）・1 字ごとの間隔は 100ns より細かい所で値が変わる（4 章の 10） |
| 10 止めて戻す | `instance_manager/mod.rs:82-128`・穴を踏むテスト 0 | **無い**: 穴を赤にするテスト。逆の向きの穴も在る |
| 11 起点の置き直し | 口は 0 | **無い**。**縛り**: 時刻の表は進め終えてから配る・同じ時刻の守り |
| 12 置き直しが配った合図にも効く | 受け口に知らせの口は 0。読む側は最大の見積もり | **無い**。**縛り**: seriko の起点はミリ秒の時計の上・文字の層の時計は UI のスレッドから読まれる |
| 13 絶対の発火時刻 | `CueSheet::absolute_fire_time` は在るが受け口へ届かない | **無い**: `TalkCue` の欄・取り出す時の和を残す所。**縛り**: 手で組む `TalkCue {` が dola の外に 64 か所（`areka-runtime` 27・`areka-emo-text` 16・本体 6・`areka-ghost` 5・`areka-input` 3・`areka-seriko` 3・`areka-session` 3・先進坑 1）・dola に 7 |
| 14 刻みが絶対の時刻 | `dispatcher.rs:366-373`・`:281-285`。トークのスレッドは絶対の刻みで動く | **縛り**: 刻みの時計は `GetTickCount64`（4 章の 6）・`ChoiceWaiting` は 9 つの外の型 |
| 15 1 本で終える縛り | 番人（1,000 行）・全体テストの段（`dola toml feature tests` を含む） | **無い**: 公開の口に `f64` の秒が残っていない事を示す検査。**縛り**: 840 行以上の 13 ファイル |
| 16 文書 | 2.8 | **縛り**: 見どころの決まりの字面・両方の brief の同じ文 |

## 4. 要件の側の課題

調べで、要件の文のままでは成り立たない所と、要件が拾っていない事実。どれもコードで確かめた（0 章）。

1. **選択待ちの時間切れは負を運ぶ（要件 4 の 1・6 の 1・6 の 5・3 の 4）**。`\![set,choicetimeout,-1]` は `Secs(-0.001)` になり、正規化されずに kanade まで届く（kanade が「0 と負は無期限」と読む）。桁あふれは `i64::MAX`／`i64::MIN` の ms へ飽和する。要件は「区切りの時間切れ」を長さの型と言い、負は「作れない」「読み込みの失敗」と言う＝今の振る舞いと両立しない。
2. **追従する相手が 9 つの外に在る（要件 7 の 1・14 の 6・7 の 5・15 の 5・Adjacent の「kanade は変えない」）**。`areka_talk::ChoiceWaiting` の 2 欄は秒の `f64` で、`areka-talk` は依存 0 の crate。刻みを絶対にすると、`display_end_elapsed_secs` は「経過」でも「秒」でもなくなる。`areka-kanade` の `timeout_directive_secs: Option<f64>` も同じ値を受ける。`areka-sakura` が秒の実数へ戻して渡せば要件 7 の 5 と 15 の 5 に触れ、型で渡せば 9 つの外を触る。
3. **型が「時刻」と「長さ」の 2 つでは足りない（要件 1 の 1・1 の 5・5 の 2）**。符号つきの差が要る所: 上の 1、storyboard の文書の `KeyframeRef::WithOffset.offset`（−0.5 を固定するテストが在る）と `trigger_start_offset`、コンパイル後の区間の時刻（起点より前に置ける）、実効の時刻（★1）、`TalkClock` と seriko の起点の見積もり（今 − 合図の時刻）、バルーンの待ちの残り（期限 − 今）。要件 1 の 5 は「負を区別できる形で返す」とだけ言い、それを運ぶ型を置いていない。
4. **`TalkCue.at` は相対（要件 4 の 1・13 の 2・13 の 7）**。名前と説明は「発火時刻」だが、中身は起点からの隔たりである。要件 4 の 1 は「`TalkCue` の時刻と長さ」としか言わない。`at` に時刻の型を当てると誤る。`at` は長さ、新しく足す絶対の発火時刻が時刻、と欄ごとに書く必要が在る。
5. **終わらないループの終わりの時刻は、今も有限（要件 1 の 7・5 の 5）**。今は 1 周分の終わりの時刻を持ち、周回ごとに延ばす（テスト「not INFINITY」が固定）。要件 5 の 5 の文のままだと、振る舞いを変える事になる。`f64::INFINITY` が時刻に出るのは、周回の後の待ちで最後の値を保つ番兵だけ。終わらないループを止める決まりは `f64` の吸収に頼っていて（`loop_controller/mod.rs:132-136`）、整数ではあふれの決まり（要件 1 の 6）で決め直す所になる。
6. **刻みの時計の物差しが、dola の時計と違う（要件 14 の 7・1 の 2・5 の 4・6 の 3・8 の 3）**。刻みは `GetTickCount64` のミリ秒で、要件 14 の 7 はそれを時刻の型へ写すと言う。その値は、要件 1 の 2 の「`QueryUnbiasedInterruptTimePrecise` と同じ物差し」ではない。この spec の後、`FrameTime`（dola の時計）と、cue の道を流れる時刻（起点・絶対の発火時刻・占有の終わり・直列化した起点）は、同じ型なのに比べられない。要件 5 の 4 と 6 の 3 は、型と決定論のテストの上でだけ成り立つ。仮定 1 は気付いているが、要件 1 の 2・5 の 4・6 の 3 の文には断りが無い。（13 章で改めた）
7. **storyboard の文書の型をどう持つかで、範囲が大きく変わる（要件 5 の 2・5 の 3・6 の 6・15 の 4・仮定 2）**。文書の型（`TransitionDef.delay`・`duration`・`LoopOffset`・オフセット）を新しい型にすると、検査の決まり（有限かつ 0 以上＝`validate/rules.rs`）の大半と `DolaError` の欄（`InvalidNumber { value: f64 }` ほか）が「起きない」事になり、`tests/validation/**` と文書を字面で組むテストの全部に及ぶ。文書の型を秒のまま残し、コンパイルの縁で 1 回だけ変える形なら、`storyboard.rs`・`transition.rs`・`builder.rs`・`validate/**`・`error.rs`・`tests/validation/**`・文書の往復のテストは触らない。ただし公開の文書の型に秒の `f64` が残る。
8. **使い手が自分で持つ公開の口が、名指しされていない（要件 7 の 1・7 の 5・8 の 1・16 の 3・仮定 3）**。`areka-emo-text` の `present_frame(.., talk_time: f64)`（中 157・外 6）・`visible_glyphs`（60）・`RevealSchedule::times()`、wintf のタイプライターの口と自動の上げ下げの判断、`areka-sakura` の `text_playback_duration`・`ChoiceTimeoutDirective::Secs`、`areka-runtime` の `TalkClock` の 4 口・`TalkLifecycleSignal`・`configured_timeout_secs`・`run_text_phase`・`hover_inject`。要件 7 の 5 のとおりなら全部が替わるが、要件 8 の 1 と 16 の 3 は wintf について `FrameTime` しか言わない（タイプライターの口も crates.io の利用者に見える）。
9. **丸めそのものを固定するテストが在る（要件 9 の 2・9 の 4・9 の 5）**。要件 9 が拾うのは「表せない入力（NaN・無限大・負）だけを確かめるテスト」と「ミリ秒の格子の継ぎを確かめるテスト」。次の物は、どちらにも当たらず、「同じ意味の値へ写すだけ」（9 の 2）でも通らない。
   - 100ns より細かい境: wintf `promotion_tests.rs:132`・`:150`・`:260`（`1e-9` 秒の手前と向こう）・`:269-270`（`1.0 / 7.0`）。
   - ビットの一致: `tests/cue/sheet_test.rs`（`to_bits` 14 行・`1.0 / 3.0`・`12.345_678_9`・`f64::MIN_POSITIVE`）・`off_grid_values_keep_plain_sum`（:595）。
   - 丸めの見本: `schedule/tests.rs:490-524`・`tests/cue/schedule_test.rs`（`ROUNDS_DOWN = 2.5999999999999996`）・`tests/trigger/event_order_test.rs:146-175`（13/60 秒・14/60 秒で 1 ulp を再現。同じファイルの 1/60 秒刻み 9 か所）・`areka-sakura` の `compile_sheet_tests.rs:555`。
   - 吸収とあふれの番兵: `loop_controller/tests.rs:226-235`（`1e17`）・`tests/compile/boundary_test.rs:40-248`（`1e308`・`1e300`・NaN・inf で `InvalidNumber`）。
   - 許す幅: `talk_clock.rs` の `1e-9`（8 行）・wintf `typewriter/tests.rs:520`（整数では厳密な一致へ強められる）。
   - 表示: `tests/runtime/core_types_test.rs:96-115`（"0.050"・"0.1"）。
   あわせて、本番の「表せない入力のための枝」（`drive.rs:298-301` の `error!`・`talk_lifecycle.rs:279-296` の `warn!`・`balloon_visibility_phase.rs:173-190` の `is_finite`・`hover_inject.rs:96-110`）は、枝ごと記録も消える。要件 7 の 3 の「振る舞いを変えない」が、これを含むかを書いていない。
10. **1 字ごとの間隔は、整数にすると 100ns より細かい所で値が変わる（要件 7 の 3・9 の 1）**。今は `長さ ÷ 字数` の実数を足し込む（`areka-emo-text` の `state.rs:290-298` と seriko の `talk.rs:76-90`。2 つは「同じ式・同じ計算の順」の約束）。割り切れない時（例 100 ms を 3 字）、字の現れる時刻は今の値から 100ns 未満ずれる。「文字がいつ現れるかを変えない」は、この幅を許すと書かないと成り立たない。
11. **起点が、時刻の型の下の端より前になる入力（要件 11 の 3・1 の 5・1 の 6）**。時刻を符号なしで持つと、0 から数える偽の時計のテスト（今の決定論のテストの大半）で、起点を前へ置き直すと下の端を越える。要件 11 の 3 は「前へも置ける」と言うが、端に当たった時の結果を言っていない。
12. **逆の向きの穴（要件 10 の 3）**。要件は「再生中なのに、止めた時点の記録が残っている」だけを言う。「止まっているのに、止めた時点の記録が無い」状態（`transition(gid, Paused)` の直呼び）も作れ、それを正しい物として使うテストが在る。
13. **ログに出す時刻の形が決まっていない（要件 7 の 5）**。合図の時刻をログの欄に出す所が `areka-ghost/src/sink.rs:72`・`areka-emo-text` の `state.rs:498`・`:554`・`:584`・`sink.rs:115`・seriko の `actor.rs:227`・`areka-sakura` の `drive.rs:299`・`:348`・`:456`・`areka-runtime` の `talk_lifecycle.rs:286-287`・dispatcher の `:252` に在り、ghost のテストは欄の文字を `f64` に解いて比べる。要件は「ログへ出す所は実数へ直してよい」とだけ言い、秒で出し続けるのか、100ns の整数で出すのかを決めていない。
14. **消える公開の関数が、壊れる変更の一覧に無い（要件 4 の 3・16 の 2）**。`dola::cue::sum_on_ms_grid` は公開で、dola の外（`areka-runtime` の本番）からも呼ばれている。要件 16 の 2 が挙げるのは「公開の口の型・直列化の形・これまでの時計」。
15. **版の行（要件 16 の 2・15 の 3）**。dola は `version.workspace = true` で、版を動かすのは `/release` が触る根の `Cargo.toml` の 3 行（うち `:32` が dola）。この spec が着地してから次の公開までの間、木の中の dola 0.1.0 は、crates.io に在る dola 0.1.0 と公開の形が違う。全体テストの包みの段は `--no-verify --offline` なので緑のまま。索引を読む形（`tools/crates-io.ps1 -Verify`）で wintf を組むと、公開済みの dola と合わずに落ちる。要件は、この spec が版の行を動かさない事と、次の公開が壊れる変更の版上げになる事を書いていない。（13 章で改めた）
16. **見どころの「必ず載せるもの」の字面（要件 16 の 4）**。決まりは「使う人やゴーストを作る人が何かを直さなければならない変更」で、crates.io の利用者は字面に入っていない。要件 16 の 4 は、この決まりに当たる前提で書かれている。（13 章で改めた）
17. **5 つが 2 本の brief に同じ文で在る（仮定 1・Out of scope・要件 12・14）**。項目 1（置き直した起点に付いて来る）と項目 4（刻みが絶対の時刻）は、`talk-anchor-interrupt-ticks` の In とも同じ所を指す。要件は仮定 1 で線を引いたが、`talk-anchor-interrupt-ticks` の brief・この spec の brief の 80 行目と 109 行目・Scope と Out of Boundary・`roadmap.md:132` は古いままで、要件 16 の直す文書に入っていない。（13 章で改めた）
18. **Windows の外で組める事の示し方が無い（要件 1 の 8・2 の 7）**。確かめる仕組みは今 0（CI は Windows だけ・`--target` を名指す段は無い）。dola の `[package.metadata.docs.rs]` は `targets` を指定していない。

小さい物（設計で拾えば足りる）: 先進坑の 3 か所（0 章）は全体テストが組まないので、黙って壊れる。例は `dola` の字を書かずに壊れる（2.1）。`Cue.duration` の「欄の無い古い形は 0 として読める」の約束は、欄が無い時だけ守られる。

## 5. 開発者が要件の段で決めること

答えで範囲か、外から見える振る舞いが変わる物だけ。推す案は、プロジェクトの決まり（時刻は正確に扱う・1 フレーム遅らせない・ベースウェアに判断やモードを足さない・外の crate をそのまま使い足さない・規模を理由に切らない・テストは判断の分かれ道を固め、古くなった物は理由を残して外す）に沿って選んだ。

| # | 決めること | 案 | 推す案 |
|---|---|---|---|
| 1 | 刻みが運ぶ絶対の時刻の物差し（課題 6） | ⑴ 要件のまま（刻みの時計は替えない）。要件 1 の 2・5 の 4・6 の 3 に「この spec の後も、cue の道の絶対の時刻は刻みの時計の物差しで、`FrameTime` と比べる所は作らない。そろえるのは `talk-anchor-interrupt-ticks`」と断りを足す ⑵ 刻みが、ミリ秒の値と dola の時計の読みの両方を運ぶ ⑶ 刻みの既定の時計を dola の時計へ替える（kanade と seriko の時計の進み方が変わる＝`talk-anchor-interrupt-ticks` の芯がこの spec に入る） | **⑴**。⑵ は後で消す組を足す。⑶ は眠りの後の時間切れなど kanade の見える動きが変わり、brief の Out に当たる。この spec の中に、2 つの物差しを比べる本番の使い手は居ない（仮定 6） |
| 2 | 選択待ちの時間切れの指令の運び方（課題 1） | ⑴ 符号つきの差の型（100ns）で、負のまま運ぶ（要件 6 の 5 の「負は失敗」から、この欄を外す。`i64` の端は縁で飽和） ⑵ 作者が書いたミリ秒の整数のまま運ぶ（dola の公開の口に物差しの違う欄が 1 つ残る） ⑶ `areka-sakura` の縁で「無い・無効・長さ」に分けて運ぶ（「0 と負は無効」の判断が kanade から移る） ⑷ この欄だけ秒の実数のまま残す | **⑴**。型は課題 3 で、どのみち要る。⑶ は kanade の権限を動かす。⑷ は要件 4 の 2 と 15 の 4 に例外を作る |
| 3 | 9 つの外の追従（課題 2） | ⑴ `areka-talk` の `ChoiceWaiting` と `areka-kanade` の指令の欄を、追従の相手に足す（kanade の「無い・0 と負・正」の読み方と期限の権限は変えず、ミリ秒への換算だけ整数になる）。先進坑の 3 か所も直す ⑵ `areka-talk` だけ替え、kanade へは dispatcher の今の継ぎ目で今の形（秒の実数）へ写す（要件 7 の 5 に名指しの例外） ⑶ どちらも替えず、`areka-sakura` が秒の実数へ戻して渡す | **⑴**。⑵ と ⑶ は、実数の秒へ戻す所が spec の終わりに残る（要件 15 の 5）。kanade の NaN と無限大の指令のテストは、表せない入力として記録して外す |
| 4 | storyboard の文書の型（課題 7・仮定 2） | ⑴ 文書の型も新しい型で持ち、秒で読み書きする（仮定 2 のまま） ⑵ 文書の型は作者が書く秒のまま残し、コンパイルの縁で 1 回だけ変える（要件 5 の 3・15 の 4 の「除く物」に、文書の定義の型を足す） | **⑵**。検査の決まりと `DolaError`・`tests/validation/**`・文書の往復のテストが今のまま残り、外すテストが減る。「作者が書く単位はそのまま」に沿う。コンパイルの結果と runtime の口は、新しい型になる |
| 5 | 使い手が自分で持つ公開の口の範囲（課題 8・仮定 3） | ⑴ 全部を新しい型で持つ（仮定 3 のまま。wintf のタイプライターと自動の上げ下げの判断を含む。作者が書く値＝`TypewriterToken::Wait`・`default_char_wait` は秒のまま、読む縁で変える） ⑵ 台本の時刻が通る道（`areka-sakura`・`areka-seriko`・`areka-emo-text`・`areka-runtime`）は新しい型、wintf のタイプライターと自動の上げ下げの判断は `FrameTime` を読む縁で秒へ戻す | **⑴**。⑵ は、丸めの種が wintf の中に残る。⑴ を取るなら、要件 8 と 16 の 3 にタイプライターの口を書き足す |
| 6 | 丸めそのものを固定するテストと、表せない入力のための本番の枝（課題 9・10） | ⑴ 要件 9 に足す:「丸めそのもの（ulp・ビットの一致・吸収・100ns より細かい境）を固定するテストは、同じ分かれ道を整数の境（1 つ手前・ちょうど・1 つ先）で確かめるテストへ置き換える。分かれ道そのものが無くなる物は、理由を記録して外す。表せなくなった入力のための本番の枝とその記録は消す。1 字ごとの間隔は、今の値から 100ns 未満の差を許し、差を積もらせない」 ⑵ 要件は今のまま、テストごとに設計で決める | **⑴**。⑵ は、要件 9 の 2（期待を弱めない）と当たる |
| 7 | ログに出す時刻の形（課題 13） | ⑴ 今までどおり秒の小数で出す（欄の名前と形を変えない） ⑵ 100ns の整数で出す（欄の名前も替える） | **⑴**。ログを読む人・道具・テストの形が変わらない |
| 8 | 止めた時刻より前で戻す入力（仮定 5・課題 12） | ⑴ 止めていた長さを 0 として扱う（要件 10 の 4 のまま）。逆の向きの穴も要件 10 の 3 に足す ⑵ 符号つきで今のまま受ける（終わりの時刻が縮む） | **⑴**。前へ戻る時刻は呼び手の誤りで、終わりの時刻を縮める理由が無い。固定しているテストは、変える前後の期待として記録する |
| 9 | 起点が型の下の端より前になる時（課題 11） | ⑴ 要件 11 の 3 に「端に当たっても、項目を落とさず同じ結果を出す」を足す（持ち方は設計） ⑵ 端で止める（飛ばした長さの一部が消える） | **⑴**。⑵ は「時刻は正確に」に反する |
| 10 | 壊れる変更の告知（課題 14・15・16） | ⑴ 見どころの決まりに「crates.io へ出すクレートの公開の形が壊れる時」を足し、dola の README に書き換え方を置く。要件 16 の 2 に、消える公開の関数と wintf の口を足し、「この spec は版の行を動かさない。次の公開は壊れる変更の版上げ」と書く ⑵ dola の README だけ | **⑴** |
| 11 | brief と隣の brief の書き直し（課題 17） | ⑴ 要件の承認と一緒に、この spec の brief と `talk-anchor-interrupt-ticks` の brief の申し送りを「dola の口と、置き直しに付いて来る事と、刻みの値（この spec）」と「刻みの時計・見積もりをやめて押された起点を読む事（`talk-anchor-interrupt-ticks`）」に書き分ける。roadmap の行は棚卸へ申し送る ⑵ 棚卸に全部を任せる | **⑴** |
| 12 | Windows の外で組める事の示し方（課題 18） | ⑴ 全体テストに、Windows でない target の `cargo check -p dola` を 1 段足す（rustup の target を 1 つ足す。外の crate は足さない） ⑵ `cfg` の字面の検査だけ ⑶ 今と同じ（示さない） | **⑴**。要件 1 の 8 と 2 の 7 を、検査で示せる |

要件の討議での扱い: 決め 2・6・7・9 と、決め 8 のうち逆の向きの穴は、「型の付け替えで振る舞いを変えない」「時刻は正確に扱う」から答えが 1 つに決まるので、自明な修正として要件へ入れた（要件 1 の 7・1 の 9・3 の 8・4 の 5・4 の 6・5 の 5・5 の 8・6 の 5・7 の 8〜10・9 の 7・9 の 8・10 の 5・11 の 12・16 の 2）。課題 3・4・5・14 も同じ修正で拾った。決め 11 は決め 1 と一緒に扱う。残る決め 1・3・4・5・8・10・12 と、要件の仮定 4・6 を、議題として開発者に確かめる。

討議で決まった事: 決め 1 は、時計を dola の時計の 1 本にそろえ、`GetTickCount64` は使わない事に決まった。`talk-anchor-interrupt-ticks` の範囲は、この spec に入れた（要件 17・12 の 9・12 の 10・14 の 6・14 の 7）。決め 3 は ⑴（`areka-talk`・`areka-kanade`・先進坑も追従する）。決め 5 は ⑴（タイプライターと自動の上げ下げの判断を含め、使う所まで新しい型）。決め 10 は、古い形の説明も書き換え方の案内も書かず、見どころの決まりは直さず、版の行も動かさない（要件 16 の 2）。決め 11 は決め 1 と一緒に済んだ。決め 12 は ⑴（全体テストに 1 段足す・要件 15 の 8）。決め 4 と 8 は、まだ討議の中。広がった範囲は 13 章。

その後に決まった事: 決め 4 は、文書の定義の型も新しい型で持つ（時間の次元を持つ値は、絶対も相対も全部 100ns の整数。実数は次元の無い値だけ＝要件 5 の 9〜11）。決め 8 は、符号つきで受ける（止めた時刻より前で戻しても、止めた時点の値から続ける＝要件 10 の 4）。課題 5 の扱いは改め、終わらないループは公開の口で「終わりが無い」を返す（要件 5 の 5。周回の終わりは中の値）。課題 13 の扱いも改め、ログは値を落とさずに出す（要件 7 の 8）。

推す案の根拠の扱い: この文書が推す案の根拠に、作業の量（タスクの増減・外すテストの数）を挙げた所が在る（5 章の決め 4・5、8.5、13.11）。作業の量は、案を選ぶ根拠にしない。数は、段取りを組むための見込みとしてだけ読む。

## 6. 設計で決めること

（14 章で決めた）

どう作るかの決め。開発者への問いではない。

1. **型と時計の置き場**: dola の中の 1 か所か、下の小さい crate か。`areka-talk` は依存 0 の約束を持ち、kanade は dola を引いていない。`ChoiceWaiting` の欄を型で持つか、素の整数で持つかも、ここで決まる。
2. **型の細部**: 符号つきの差の型の形・あふれ（失敗にするか張り付けるか）・既定の値（`FrameTime` の `Default`・serde の既定）・「終わり無し」の形（周回の後の待ちの番兵と、終わりの期限の「無し」。TOML は最大値を書けない）・表示（ログの秒）。
3. **丸めの 1 か所**: 作者が書く単位からの変え方（ちょうど中間の扱い）・★1〜★6 をまとめる所（★1 と ★2 を逆の対にする）・`next_down` の代わり・`MIN_LOOP_DURATION`（1,000,000）・終わらないループを止める決まり（吸収の代わりに、あふれで終える）。
4. **起点の置き直しの口**: 口の形（「今が相対のいくつ」で置くか、動かす長さで置くか）・1 回の刻みの中で取り出しごとに置き直す手段（再生機は今、進め終えてから配る）・取り出しを「絶対の発火時刻と中身の組」にする形（時刻の表は中身の型を知らないまま）・受け口の知らせの口（既定の実装が何もしない形＝要件 12 の 3）・合図と知らせを起きた順に配る仕組み・最初の起点を同じ口で知らせるか・同じ時刻の守りとの兼ね合い（区切りを解く口が、溜めた取り出しを空にしている前例）・完了の後の置き直し・先渡し（`preview`）の合図の発火時刻・配る順の発火時刻の単調を約束にするか（dola はログの依存を持たない）。
5. **`TalkCue` の形**: 新しい欄の名前・`at` の名前を残すか・手で組む所（dola の外 64・中 7）の書き方（字面のままか、組み立ての関数か）・`PartialEq` に発火時刻が入る事と、合図の列を比べるテストの期待。
6. **1 字ごとの間隔**: 整数での式（何番目かから求めて積もらせない形）を、`areka-emo-text` と seriko で同じにする。
7. **読む側が置き直しに付いて来る形**: `TalkClock` は最大の見積もりを残したまま、知らせの長さと向きを足す（要件 12 の 9 の「打ち消さない」をどう保つか）。seriko の起点はミリ秒の時計の上なので、100ns の長さをミリ秒へ写す丸めが要る。UI のスレッドが読む時計と、トークのスレッドの知らせの順。
8. **口パク**: 起点が飛んで複数の字がまとめて現れた時の `talk` の引き金を、今の決まりで読んで確かめる（要件 12 の 7）。
9. **OS の関数の呼び方**: 機能 `Win32_System_WindowsProgramming` を足す形（dola の manifest に直に書くか、根の一覧を受け継ぐままか）・`Win32_System_Performance` を外すか・`cfg` の述語をそろえる・docs.rs の `targets`。
10. **`areka-sakura` の縁**: `ms_to_secs` を置き換える 1 か所・`Duration` から直に変える形（1 ms 未満を切り捨てる `shortcut:` を消すか）・`ChoiceTimeoutDirective` の形。
11. **起点の「押されていない」の印**: 今は 0.0。0 の時刻は、偽の時計のテストでは普通の値である。
12. **記録の置き場**: 外したテストと理由（要件 9 の 4・5）・変える前後の期待（要件 9 の 6）・見どころの材料（要件 16 の 4）。
13. **公開の口に `f64` の秒が残っていない事の示し方**（要件 15 の 4）: 検索の式か、番人のテストか。除く物の一覧。
14. **1,000 行に近いファイルの手当て**（2.7）: 時刻を短く書ける組み立ての置き場・延びたら分けるファイル。
15. **段取りの確かめ**（8 章の推す形）と、設計の前の確かめ（main に合わせ直す・同じ時に走らせない相手が走っていない事・先に着地した spec が dola に足した口が無い事）。

## 7. 作り方の案

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| A 今のファイルの中で書き換える | 新しいファイルは時刻の型と時計だけ。起点の置き直し・知らせ・発火時刻も `schedule.rs`（279）・`runtime.rs`（322）・`sink.rs`（150）・`talk_clock.rs`（481）の中へ足す | ファイルが増えない。今の層の分け方のまま | 置き直しの決まりとテストが、型の付け替えと同じファイルに混ざり、前半と後半を別々に確かめにくい。`schedule/tests.rs`（563）が延びる |
| B 新しい部品として建てる | cue の層を新しい型で別の置き場に建て、使い手を移してから古い方を消す。または、型と時計を下の小さい crate に切る | 古い口が生きている間、ワークスペースが緑 | `areka-sakura` の再公開で、上は一緒に替わる＝緑が保てるのは dola と `areka-sakura` の間だけ。cue の本体 約 1,100 行とテスト 約 2,400 行が一時 2 つになる。下の crate は crates.io へ出す物が 1 つ増える |
| C 型と時計と置き直しは新しいファイル、今の層はその場で書き換える | `dola` に時刻の型・変える口・倍率の 1 か所・時計の新しいモジュール。cue の層と storyboard の層は、その場で型を替える。起点の置き直しと知らせとそのテストは、cue の層の新しいファイルへ | 前半（型）と後半（振る舞い）がファイルで分かれる。1,000 行の決まりに沿う。層の分け方は今のまま | 新しいファイルが 4〜6 本増える |

推すのは **C**（設計で確かめる）。

## 8. 1 本の spec で最後まで通す段取り

### 8.1 縛り

- cargo は crate ごとに組む。dola の公開の型が替わると、上の crate は、自分が替わるまで組めない。**dola を切り替えてから本体が替わり終わるまで、ワークスペースの全体（`cargo build --workspace`・`tools/test-all.ps1`・全体の clippy）は赤**である。その間に使える確かめは、crate ごとの `cargo test -p <crate>`（下が緑なら組める）と `cargo fmt`。コミットの時に組む仕掛けは無い（0 章）ので、赤の間もブランチへコミットできる。
- 1 つの crate の中では、`src` の中のテスト（`*_tests.rs`）は全部が 1 つの実行ファイルになる。**同じ crate の別のテストのファイルを別々の作業者が直す時、テストを走らせられるのは全員が終わった後**。それまでは `cargo check -p <crate> --all-targets` の誤りの一覧から、自分のファイルの分が 0 になった事を確かめる。`tests/` の入口（dola なら `cue`・`compile`・`general`・`runtime`・`trigger`・`validation`）は別々の実行ファイルなので、`--test <名前>` で別々に走らせられる。
- 並行にできるのは編集で、組む確かめは 1 本ずつ（同じ `target` の錠と、重いビルドを同時に走らせない決まり＝`tech.md`）。
- `areka-sakura` の再公開（2.1）で、`areka-sakura` より上は、形の替わる所を crate ごとにずらせない。

### 8.2 先に替える共有のファイル

ほかのファイルの家族が使うので、家族を並べる前に 1 人が替える。

| crate | ファイル | 理由 |
|---|---|---|
| `dola` | 時刻の型と時計の新しいモジュール・`src/lib.rs`・`Cargo.toml` | 全部の前提 |
| `dola` | `src/cue/test_support.rs`（`carrier_cue`） | 外の 16 ファイルが直に、35 ファイルが名前で使う。ここを替えれば、その使い手は触らない |
| `dola` | `tests/cue/test_support.rs`（`barrier`・`text`・`choice`・`wait`） | 引数の形で、`sheet_test.rs` ほかの呼び出し約 70 か所の書き方が決まる |
| `dola` | `src/compile/types.rs` | コンパイルの側と runtime の側の両方が、この欄を読む |
| `areka-talk` | `src/lib.rs`（`ChoiceWaiting`） | `areka-sakura`・`areka-ghost`（・`areka-kanade`）が使う |
| `areka-sakura` | `contract.rs`（再公開と `SakuraMsg::Tick`）・`duration.rs`（変える 1 か所） | 上の crate の全部が受ける |
| `wintf` | `ecs/graphics/core.rs`（`FrameTime`） | wintf の中の 3 系統と、上の 4 crate が読む |
| `areka-seriko` | `lib_test_support.rs`・`actor_test_support.rs`・`tests/common/mod.rs` | 共有の組み立て |
| `areka-emo-text` | `actor_test_support.rs`・`state_test_support.rs`・`viewbox_draw_test_support.rs`・`tests/support/mod.rs`・`tests/support/fixture.rs` | 共有の組み立て（`tests/` の 10 ファイルの自前の `cue` は、それぞれの家族が持つ） |
| `areka-runtime` | `talk_clock.rs`・`talk_lifecycle.rs`・`lib.rs:465`・`consumer_ledger_test_support.rs`（`broken_custom_cue`）・`frame_test_support.rs`（`zero_clock`）・`balloon_visibility_phase_test_support.rs`・`balloon_visibility_test_support.rs`・`frame_balloon_runtime_test_support.rs`・`balloon_visibility_wait.rs`／`hover_inject.rs` の変える所 | 機能 `test-support` や `pub` で、ほかの家族と上の crate が使う |
| `areka-input` | `balloon_test_support.rs` | 本体と `areka-session` が使う |
| `areka-session` | `shell_balloon_switch_session_lap_test_support.rs`・`runtime_boot_tests.rs` | 本体が使う |

### 8.3 別々の作業者が、同じファイルに触らずに直せる組

- **dola の中**（土台の後）:
  - cue の本体は 1 人で 1 段（`command.rs` → `schedule.rs` → `sheet.rs` → `runtime.rs` → `test_support.rs`・`mod.rs`。互いに呼び合うので分けない。計 約 1,100 行）。storyboard の側の本体とは別のファイルなので、同じ時に進められる。
  - storyboard の側は、`compile/types.rs` の後に 2 組: コンパイルの組（`compile/mod.rs`・`compile/resolve.rs`・`tests/compile/**`・`tests/trigger/compile_test.rs`）と、runtime の芯の組（`runtime/{facade.rs, facade_tests.rs, types.rs, instance_manager/*, loop_controller/*, timeline_manager/*, conflict_resolver.rs}`・`tests/runtime/**`・`tests/trigger/{runtime_test,event_order_test}.rs`。`StoryboardInstance` の欄を全部が使うので割らない）。
  - 本体の後のテストは、組ごとに別のファイル: `command_tests.rs`／`schedule/tests.rs`／`tests/cue/test_support.rs` を先に替えてから `sheet_test.rs`・`sheet_sort_test.rs`＋`schedule_test.rs`・`runtime_test_*` の 5 本＋`sink_test.rs`／`interpolator/tests.rs`（置物の時刻だけ）。
  - 触らずに済む見込み: `easing.rs`・`runtime/interpolator/mod.rs`・`runtime/subscription_manager/*`・`value.rs`・`variable.rs`。決め 4 が ⑵ なら `storyboard.rs`・`transition.rs`・`builder.rs`・`validate/**`・`error.rs`・`tests/validation/**`・文書の往復のテストも。
- **下位**: `wintf`（`FrameTime` と世界と `DolaAnimator`／タイプライター／自動の上げ下げの判断の 3 組）と、`areka-talk`＋`areka-sakura`（本体／コンパイルのテスト／再生のテストの 3 組）は、crate が別で互いに独立。
- **その上**: `areka-seriko`（本体と組み立て／テスト）・`areka-emo-text`（本体と組み立て／`src` のテスト 2 組／`tests/` と例）・`areka-ghost`（dispatcher の家族／`prop_*` と `sink.rs`／`tests/ghost/`）・（決め 3 が ⑴ なら）`areka-kanade` は、crate が別。
- **`areka-runtime`**（土台の後）: `balloon_visibility*` の 3 組／変更の受け口 約 19 ファイル／`frame_*` 9 ファイルの 5 組。
- **上の 3 つ**: `areka-input`（2）→ `areka-session`（5）→ 本体（`runtime_tests/`・`session_tests/`・`mcp/`・`install_cue*`＋`update_cue*`・`examples/` の組）。本体の中は、組ごとに別のファイル。

### 8.4 途中を保つやり方の比べ

| 案 | 中身 | 赤の間 | 確かめ方 | 悪い所 |
|---|---|---|---|---|
| 1 継ぎ無し・1 回で全部 | 下の crate から上へ、型の付け替えと振る舞いの変更を同じ時に入れる | dola の切り替えから本体まで | crate ごとの `cargo test -p`。最後に全体テスト | 型の付け替えで振る舞いが変わっていない事（要件 9 の 1）と、振る舞いを変えた所（要件 9 の 6）を別々に示せない。dispatcher のテストは、型と意味が同時に変わる |
| **2 継ぎ無し・前半と後半** | 前半: 下から上へ、型の付け替えだけ（要件 1〜9）。ただし**合図と刻みの形は、前半で最後の形まで替える**（`TalkCue` に絶対の発火時刻の欄を足す＝起点が動かない間は「起点＋相対」で、振る舞いは変わらない。`SakuraMsg::Tick` と `ChoiceWaiting` は新しい型にするが、dispatcher が送る値の意味は今のまま）。前半の終わりに全体テストを 1 回。後半: 振る舞い（要件 10〜14）。起点を置き直す口・受け口の知らせ・読む側の追従は足すだけの変更で、ワークスペースは緑のまま進む。dispatcher の値の意味を替えるのは、ここ | 前半の、dola の切り替えから本体まで。後半は赤にならない | 前半は crate ごとの `cargo test -p`、終わりに全体テスト 1 回（付け替えの前と同じ結果）。後半は要件ごとのテストと、完了の全体テスト | 全体テストが 1 回増える。`areka-ghost` の dispatcher の家族を 2 度触る（1 度目は型、2 度目は値の意味） |
| 3 仮の継ぎを置く | 秒の実数から新しい型へ黙って変わる変換（`From<f64>` と、それを受ける引数）を dola に仮に置き、使い手を crate ごとに移し、最後に消す | 小さくなるが、0 にはならない | 緑の間は全体テストが使える | 継ぎが効くのは関数の引数だけ。欄を字面で組む所（`TalkCue {` 64＋7・`FrameTime(` 31・`Cue {`）と欄を読む所・`SakuraMsg::Tick` のバリアントは、継ぎがあっても組めない。まだ替えていない呼び出しが、丸めつきで黙って通る（替え残しが誤りに出ない）。要件 15 の 5 のとおり最後に消す仕事と、消した事の検査が増える |
| 4 新旧の口を並べる | 7 章の案 B | dola と `areka-sakura` の間だけ緑 | — | `areka-sakura` より上は 1 回の切り替えになるので、案 2 と赤の間はほぼ変わらない。本体とテストが一時 2 つになる |

**推すのは案 2**（設計で確かめる）。継ぎが 0 なので、要件 15 の 5 は「置かなかった」で満たせる。形を前半で最後まで替えるので、手で組む `TalkCue {` の 71 か所と `SakuraMsg::Tick(` の 111 か所（`areka-sakura` 101・`areka-ghost` 7・ほか 3）を 2 度触らない。要件 9 の 1（前半の終わり）と 9 の 6（後半）を、別々の全体テストで示せる。

赤の間（前半）の進め方: 段ごとに「下の段の crate が `cargo test -p` で緑」を次の段の入口にする。上の段の作業者は、下が緑になる前から編集を始めてよいが、確かめは下が緑になってから。全体テストは、前半の終わりと完了の時の 2 回だけにする（重いテストを何度も回さない）。

### 8.5 段とタスクの数（並行の単位で刻んだ見込み）

（13 章で改めた）

| 段 | 中身 | 並べられる数 | タスク |
|---|---|---|---|
| 0 | dola の土台（型・変える口・倍率の 1 か所・時計・`Cargo.toml`）。足すだけで、ワークスペースは緑 | 2 | 3 |
| 1 | dola の付け替え（cue の本体 1・`compile/types.rs`＋コンパイルの組 2・runtime の芯 2・cue のテスト 3・storyboard の側のテスト 2・直列化の形と往復 1）。確かめは `cargo test -p dola` と `--features toml` | 2〜5 | 10〜11 |
| 2 | `wintf`（3）と `areka-talk`＋`areka-sakura`（3） | 2〜6 | 6 |
| 3 | `areka-seriko`（2）・`areka-emo-text`（4）・`areka-ghost`（2）・`areka-kanade`（決め 3 が ⑴ なら 1） | 3〜8 | 8〜9 |
| 4 | `areka-runtime`（土台 1・`balloon_visibility*` 3・変更の受け口 1・`frame_*` 1） | 1 → 5 | 6 |
| 5 | `areka-input`・`areka-session`（1〜2）・本体（2〜3・先進坑を含む） | 1 → 3 | 4〜5 |
| 6 | 前半の締め（残った `f64` の秒の検索・外したテストの記録・全体テスト 1 回） | 1 | 1 |
| 7 | 後半（止めて戻す 1・起点の置き直し 2・絶対の発火時刻を取り出す時に決める 1・知らせと読む側〔`TalkClock`・seriko・バルーンの寿命〕3・刻みと `ChoiceWaiting` 2）。ワークスペースは緑のまま | 2〜3 | 9 |
| 8 | 文書（dola の README・壊れる変更の説明・`tech.md`・`structure.md`・コードの説明・brief）と締めの検査（公開の口に `f64` の秒が無い事・継ぎが無い事） | 2 | 3 |
| 計 | | | **50〜53**（決め 4 を ⑴ にすると +3〜4、決め 5 を ⑵ にすると −2） |

brief の見込み（18〜26）より多い。使い手の crate が 9 つ（と外の 2 つ）に在り、テストの家族を別々の作業者に割るので、1 つずつは小さい。数を理由に範囲を切る事は、ここでは勧めない（要件 15）。

## 9. 外す・置き換える見込みのテスト（設計の一覧の種）

| 種類 | 所 | 扱いの見込み |
|---|---|---|
| NaN・無限大・負の時刻（表せない入力） | dola: `tests/cue/sheet_test.rs:89-109`・`:306-336`・`:603-615`、`tests/cue/sheet_sort_test.rs`（全体）、`schedule/tests.rs:333-381`（3 本）、`instance_manager/tests.rs:248-265`、`interpolator/tests.rs:405-490`（3 か所）、`runtime/clock.rs:54-63`（`is_finite`）。使い手: `areka-sakura` の `drive_delivery_tests.rs:429-430`・`compile_sheet_tests.rs:192-196`、`areka-runtime` の 2.5 の一覧、seriko の `looper_talk_tests.rs:381`（「全部」の意味の無限大）、`areka-emo-text` の `state_place_tests.rs:179-180`（`f64::MAX`） | 作者が書く単位から変える口のテストへ置き換えるか、理由を記録して外す（要件 9 の 4）。「全部」「いつでも」の意味の物は、型の端の値で書き直す |
| ミリ秒の格子の継ぎ | `tests/cue/sheet_test.rs:500-518`・`:553-601`、`runtime_test_broadcast_tests.rs:150-180`、`areka-runtime` の `talk_lifecycle_tests.rs:435` ほか、`areka-emo-text` の `actor_lookahead_tests.rs:139-150` | 継ぎと一緒に外し、ちょうどの刻みで届く事を整数で確かめるテストを残す（要件 9 の 5） |
| 丸めそのもの | 4 章の課題 9 の一覧 | 5 章の決め 6 |
| 負の時間切れ | `schedule/tests.rs:258-262`・`areka-sakura` の `compile_choice_timeout_tests.rs:46-54` | 5 章の決め 2（⑴ なら、符号つきの差で残る。`i64` の端は飽和のテストになる） |
| 振る舞いを変える前の期待 | 2.6 の「意味が変わるテスト」 | 変える前後の期待として記録する（要件 9 の 6） |

## 10. 規模とリスク

- **規模: XL**（2 週間を越える仕事）。dola の公開の口 49 か所（cue 20・storyboard 29）と、使い手の 9 crate＋外の 2 crate のテストの大半に及ぶ。新しい型・新しい口（起点の置き直し・知らせ・絶対の発火時刻）を含む。（13 章で改めた）
- **リスク: 高**。外の技術に分からない所は少ない（OS の関数 1 つ・`windows` の機能 1 つ）。危うさは幅と決まりに在る: ⑴ 前半の間、全体テストが使えない ⑵ 丸めの決まり（★1 と ★2 の対・1 字ごとの間隔・作者の単位の縁）を誤ると、ちょうどの刻みの届き方が変わる ⑶ 起点を置き直す時の順の保証（落とさない・2 度届けない・起きた順）は、今のどこにも前例が無い ⑷ 1,000 行に近い 13 ファイル ⑸ 同じ時に走らせない相手が多い（`element-animation-easing`・`seriko-talk-clock-fidelity`・`talk-fast-forward`・`sakura-embed-directive`・`tick-gate-adoption`）。

## 11. 設計で調べること

（調べた結果は 14 章）

- `GetTickCount64` と `QueryUnbiasedInterruptTimePrecise` の進み方の違い（眠りの間を数えるか・0 の時点）。一般の知識では、`GetTickCount64` は眠りの間も数える。コードでも文書でも確かめていない。5 章の決め 1 の ⑴ の断りの文に要る。
- Windows でない target の `cargo check -p dola` が、今の `Cargo.lock` と道具で通るか（5 章の決め 12）。
- docs.rs の既定の target と、公開の文書から時計の関数が抜ける事（一般の知識・未確認）。
- dola の公開の manifest に乗る `windows` の機能（根の一覧を受け継いでいる見込み。`cargo package` の出力で確かめていない）。
- `QueryUnbiasedInterruptTimePrecise` の、arm64 と対応する一番古い Windows（`tech.md` の対応は Windows 10 1903 から）での有無。
- 新しい型を `tracing` の欄に出す形（今の `f64` の欄と同じ字面を保てるか）。
- seriko の口パクの今の決まり（1 回の刻みの間に複数の字が現れた時＝要件 12 の 7）。
- 設計の前に: main に合わせ直し、同じ時に走らせない相手が走っていない事と、先に着地した spec が dola に足した口が無い事を数える。

## 12. 設計フェーズへの勧め

（14 章で改めた）

- 作り方は 7 章の案 C、段取りは 8.4 の案 2 を出発点にする。
- 先に 5 章の 12 件を要件の討議で決める。決め 1〜5 は、型の数・追従する crate の数・触るファイルの数を動かす。（13 章で改めた）
- 設計の最初に、6 章の 1〜3（置き場・型の細部・丸めの 1 か所）を固める。ほかの全部がこれに乗る。
- 起点の置き直し（6 章の 4）は、今の再生機の「進め終えてから配る」形を変える所なので、順の保証を先に文で決めてから口の形を選ぶ。

## 13. 時計の統一で広がった範囲

要件の討議で時計を 1 本にそろえると決まり（5 章）、`talk-anchor-interrupt-ticks` の範囲（要件 17・12 の 9・12 の 10・14 の 6・14 の 7）が、この spec に入った。この章は、その範囲を今のコードで測り直した物で、1・4・5・8・10・12 章のうち古くなった所は、ここが改める。測った木は同じワークツリー（HEAD `4a8ff9d08`）。数は grep の行数とファイル数。0 と書いた物は、同じ式が在ると分かっている所に当たる事を先に確かめた。

### 13.1 `GetTickCount64` を読む所と、時計を差し込む所

- **読む所は本番 2・テスト 0**（`windows::Win32::System::SystemInformation::GetTickCount64()` の呼び出し）:
  - `areka-ghost/src/ticker.rs:80`（`real_clock`）。刻み役の既定の時計で、`TickerConfig::default`（:65-73・50 ms と 1000 ms の 2 系統）と `LoopTickerConfig::default`（:279-286・16 ms）の両方が使う。
  - `areka-runtime/src/lib.rs:653`（`tick_count_ms`）。:537 で `SerikoClock` にし、seriko の起動（:538 `spawn_seriko_clocked`）と SERIKO のループの刻み（:631 → `loop_ticker_config` :657-663）の両方へ渡す。
- 説明の文だけの物は 12 行（`ticker.rs:60`・`:66`・`:76`・`:78`・`:266`・`:274`・`:280`、`areka-runtime/src/lib.rs:535`・`:652`、`areka-kanade/src/msg.rs:29`、`areka-sakura/src/contract.rs:24`、`areka-session/src/boot_config.rs:539`）。`:78` と `:652` は呼び出しに付く `SAFETY:` で、呼び出しと一緒に消える。
- ほかの時計の関数（`GetTickCount`・`timeGetTime`・`QueryInterruptTime*`・`QueryUnbiased*`）を読む所は `crates` に 0。機能 `Win32_System_SystemInformation`（根の `Cargo.toml:69`）は `areka-session/src/log_history.rs:23` の `GetLocalTime`（暦の時刻・範囲の外）も使うので、残る。
- **時計を差し込む所**:
  - ghost の刻み: `TickerConfig.clock: Box<dyn Fn() -> MonotonicMs + Send>`（`ticker.rs:62`）・`LoopTickerConfig.clock`（:276）。本番は既定のまま（`areka-session/src/boot_config.rs:558`・`ghost_session.rs:379` の `TickerMode::Real(Default::default())`。受けるのは `areka-ghost/src/runtime.rs:96`・`:723`）。
  - seriko: `SerikoClock = Arc<dyn Fn() -> u64 + Send + Sync>`（`areka-seriko/src/actor.rs:328`・ミリ秒）。持つのは `SerikoSink.clock`（:140）と `LoopRuntime`（`looper.rs:139`・`with_clock` :357・`event_ms` :364）。読むのは `actor.rs:189`・`:530`・`:727`・`:770`・`looper.rs:379`。
  - kanade: 時計を持たない。時刻は刻み（`KanadeMsg::Tick { now }`・`ChoiceTimeoutDue { now }`＝`msg.rs:203`・`:206`）と選択待ちの知らせ（`ChoiceWaiting.display_end`・:239）で外から届く。
  - 選択待ちの起こし: `TickerMsg::WakeKanadeAt(MonotonicMs)`（`ticker.rs:29`）・`KanadeConfig.choice_wake: Option<Sender<MonotonicMs>>`（`msg.rs:576`）・中継 `areka-ghost/src/runtime.rs:620`・`:731`。
- brief の `emo2_boot/mod.rs`・`emo2_boot/talk_clock.rs` の道は古い。今の結線は `areka-runtime/src/lib.rs`（:465 文字の層の時計・:537 seriko の時計）と `areka-runtime/src/talk_clock.rs`。
- **テストの偽の時計**（どれも型が替わる）: `areka-ghost/src/ticker_tests.rs`（`TickerConfig {` 4・`LoopTickerConfig {` 5）・`areka-ghost/src/runtime_choice_wake_tests.rs:68`・`areka/src/thread_roles_tests.rs:81`・`:90`・`areka-runtime/src/seriko_clock_tests.rs:15`・`film_playback_e2e_tests.rs:409`（`AtomicU64` の時計）・`areka-seriko/src/actor_clock_tests.rs:229`・`actor_stage_tests.rs:205`・`areka-session/src/runtime_boot_tests.rs:253-260`・`ghost_switch_test_support.rs:211`・`:595`（`talk_clock_ms`）。実の時計で回すテスト（`TickerConfig::default()`）は `areka-ghost/src/runtime_tests.rs:624`・`:704`・`tests/real_pasta_online.rs:163`・`tests/ghost/snapshot_capture_test.rs:185`・`tests/ghost/real_pasta_test.rs:123`。

`MonotonicMs` を書くファイル（crate ごと）:

| crate | 本番のファイル | テストのファイル | 行 |
|---|---|---|---|
| `areka-kanade` | 10（`actor.rs`・`lib.rs`・`msg.rs`・`schedule/{change,choice,close,events,mod,steady,types}.rs`） | 45 | 286 |
| `areka-ghost` | 3（`ticker.rs` 15・`dispatcher.rs` 5・`runtime.rs` 2） | 13 | 117 |
| `areka-runtime` | 1（`lib.rs` 3） | 0 | 3 |
| `areka-session` | 0 | 2 | 4 |
| 本体 `areka` | 0 | 3（`runtime_tests/spine.rs`・`spine_conformance_support.rs`・`spine_conformance_support_tests.rs`） | 8 |
| 計 | 14 | 63 | 418 |

### 13.2 kanade の `MonotonicMs`

- 定義: `msg.rs:32` の `pub struct MonotonicMs(pub u64)`（`Ord` まで導出）。説明 :29-30 は「GetTickCount64 相当」。`lib.rs:56` で再公開。
- 運ぶ口: `KanadeMsg::Tick`・`ChoiceTimeoutDue`・`ChoiceWaiting { display_end: MonotonicMs, timeout_directive_secs: Option<f64> }`（`msg.rs:203`・`:206`・`:233-243`）・`KanadeConfig.choice_wake`（:576）。中の口: `Input::Tick`・`ChoiceWaiting`・`ChoiceTimeoutDue`（`schedule/types.rs:21`・`:62`・`:69`）・相の `deadline: Option<MonotonicMs>`（:147・:154・:161）・選択の帳簿の `deadline`（:192）・`State.last_now`（:219）・`Action::ArmChoiceWake`（:403）。関数は `steady.rs:170`・`:389`・`:533`・`:662`・`:684`・`events.rs:468`・`change.rs:306`・`:438`・`close.rs:118`・`:161-195`・`actor.rs:481`。
- 足し引き:
  - 選択待ちの期限 `choice.rs:100-114`（`display_end + round(秒 × 1000)`・`saturating_add`・NaN と 0 以下は無期限・指令が無ければ既定）。期限の比べは `steady.rs:540`（`now < deadline`）。
  - 終わりの握手と送り出しの台詞の再生完了待ちの上限: `close.rs:169-170`（`now + close_talk_deadline_ms`・`saturating_add`）・`:185-195`（超過の量 `saturating_sub`）。`change.rs:254`・`:306-330` が同じ関数を使う。
  - 長さの設定 `KanadeConfig.close_talk_deadline_ms`・`choice_timeout_default_ms`（`msg.rs:551`・`:563`・既定はどちらも 30_000）。作者が書く値ではなく、ベースウェアの定数。
- **`OnSecondChange`**: `events.rs:468-470` が Reference0 を `now.0 / MS_PER_HOUR`（:93 `3_600_000`）の 10 進の文字列にして SHIORI へ送る＝時（hour）の整数。1000 ms の刻みごとに `steady.rs:709` から出る。ukadoc は Reference0 を「OSの連続起動時間（hour）」とだけ書く（`ukadoc:list/shiori-event/time/OnSecondChange`）。時計を替えると、割る数は 100ns で 36,000,000,000 になり、値は眠りの間を数えない（要件 17 の 7）。SSP が何を数えているかは測っていない。
- ログの欄はミリ秒の整数: `now_ms`・`deadline_ms`・`display_end_ms`・`overshoot_ms`（`steady.rs:205-206`・`:553-554`・`:572-573`・`:669`・`mod.rs:294`・`close.rs:133`・`:141`・`change.rs:320`・`:325`）。
- テスト: `MonotonicMs(` の字面を書くテストのファイル 39・行 148（`steady_choice_timeout_tests.rs` 16・`schedule_tests.rs` 16・`schedule_log_firing_tests.rs` 14・`user_break_tests.rs` 9・`change_tests.rs` 8・`tests/kanade/steady_test.rs` 8 ほか）と、本番のファイルの中のテスト（`close.rs` 19 行・`choice.rs` 約 30 行）。`tests/kanade/choice_test_test_support.rs:73` の `MonotonicMs(1_000)` は、brief の申し送りのとおり定数の名へ替える。
- 840 行以上: `schedule/events.rs` 941・`schedule/steady.rs` 926・`actor.rs` 873。どれも時刻の行は 2〜6 で、替える量は小さい。

### 13.3 areka-ghost の刻み・中継・受け口

- `ticker.rs`（344 行）: 周期は `base_interval` 50 ms・`kanade_interval` 1000 ms（:69-70）・ループ 16 ms（:283）。刻みは `Tick { now: MonotonicMs }`（:51-52）で、時計の読みそのもの（ミリ秒）を運ぶ。境は「時計の 0 からの周期の倍数」（`BoundarySchedule`・:95-160・`next_grid_multiple_strictly_after` :170-172）で、別の刻み役どうしも同じ格子に乗る。待ちは `recv_timeout`（残りは `Duration::from_millis`・:44・:126）。遅れて境を 2 つ以上またいだら 1 回にまとめ、`info!` を出す。kanade へは `KanadeMsg::Tick { now }`（:228）と起こし `ChoiceTimeoutDue { now }`（:237-239）。
- `dispatcher.rs`（478 行）: `DispatcherMsg::Tick { now }`（:37）・`base_now: Option<MonotonicMs>`（:101）。刻みの中継は :366-381（最初の刻みを `base_now` にし、`(now − base) / 1000.0` を `SakuraMsg::Tick(f64)` で送る）。逆の写しは :281-285（`base + round(秒 × 1000)`）。`base_now` が無い時の `warn!`（:268-276）。刻みを絶対の時刻にすると、`base_now` は使い道が無くなり、この 3 か所ごと消える。
- `sink.rs`（264 行）: **起点を押さない**（brief の「`sink.rs` は `0.0` を押す」は古い）。:72 はログの欄 `at = cue.at` だけ。テストの `sample_cue(at: f64)`（:168-175）が `duration: 0.0` で組む。
- 起点を押す本番は `areka-sakura/src/drive.rs:317`（`with_absolute_start_time(t)`・`t` は中継された相対の秒）の 1 か所で、`CuePlayer::from_sheet` の本番の呼び手もここ（:318）だけ。テストでは 10 ファイル・11 か所（`areka/src/session_tests/mod.rs:166` ほか）が起点を押さずに再生機を作る。
- テスト: `MonotonicMs(`・`Tick { now`・`DispatcherMsg::Tick` を書くテストのファイル 13・行 123（`ticker_tests.rs` 36・`dispatcher_choice_tests.rs` 20・`dispatcher_slot_tests.rs` 15 ほか）。840 行以上は `dispatcher_choice_tests.rs` 908・`tests/ghost/inproc_e2e_test.rs` 964（2.7 に在る）。

### 13.4 areka-seriko の時計がどこまで届くか

- 時計と刻み: `SerikoClock`（`actor.rs:328`・ミリ秒の `u64`）・`SerikoMsg::Tick { now_ms: u64 }`（:75）・`SerikoSink::send_tick(now_ms)`（:175・公開）・`LoopRuntime::on_tick(now_ms)`（`looper.rs:437`）。
- **seriko のアニメの時刻は、全部この時計の上にある**。本番のファイルで `now_ms`・`*_ms` を書くのは 17（`looper.rs` 36 行・`timeline.rs` 35・`looper_trigger.rs` 34・`parts.rs` 30・`parts_trigger.rs` 23・`table.rs` 21・`actor.rs` 20・`trigger.rs` 20・`parts_calls.rs` 17・`calls.rs` 16・`parts_refresh.rs` 15・`looper_calls.rs` 14・`looper_refresh.rs` 12・`talk.rs` 10・`settle.rs` 8・`table_calls.rs` 4・`actor_bind.rs` 2）。`now_ms: u64` か `*_ms: u64` を引数に取る関数は 21。中身:
  - コマの進み（`timeline.rs:106` `frame_at(.., elapsed_ms)`・`:122` `lap_of`・`:130`）
  - 毎秒の抽選の境（`LotteryBoundary`・`timeline.rs:48-82`・1000 ms の格子。ghost の刻み役と同じ「時計の 0 からの倍数」）
  - `periodic`（`table.rs:222-226`）と `always` の 1 周（:336-346）
  - 口パクの引き金（`trigger.rs:60-95` の `at_ms`・`wall_ms`）
  - 着せ替えと部品の始まりの時刻（`looper.rs:110` `started_at_ms`・`parts.rs:68`）
  - 窓の知らせに載せる時刻（`actor.rs:189`・`:530`・`:727`・`:770`）
- **作者が書く値（読む縁で 1 回だけ長さへ変える所）**: pattern定義のコマの待ち `LoopFrame.wait_ms: u32`（`table.rs:115`・1 ms 単位）・`periodic,秒` の秒（`table.rs:223` が `secs × 1000`）・`always` の 1 周（`table.rs:336` が `wait_ms` の和）。3 つとも表を組む所（`from_world_and_films`・`table.rs:196` から）に集まっている。毎秒の抽選の 1000 ms と刻みの 16 ms はベースウェアの定数。
- 台本の時刻との継ぎ目（`talk.rs`・123 行）: `TalkEpoch.epoch_ms: Option<f64>`（:23）・`observe(at_s, now_ms)`（:32-38・起点＝今 − 合図の秒 × 1000 の最大）・`talk_time`（:41-44・負は 0）・`wall_ms`（:49-52・最も近い 1 ms へ丸める）。`TalkFeed` は台本の秒の `f64` で持つ（:55-63・`push_chunk` :76-90）。読む所は `looper.rs:382`（`observe`）・`looper_trigger.rs:155-180`・`:217-223`、起点を捨てる所は `looper.rs:849`。同じ物差しになれば、`wall_ms` の 1 ms の丸めは要らなくなる。
- テスト: `now_ms`・`send_tick(`・`on_tick(`・`Tick { now_ms`・`SerikoClock` を書くテストのファイル 26・行 373（`looper_trigger_tests.rs` 78・`looper_tests.rs` 70・`looper_calls_tests.rs` 52・`parts_calls_tests.rs` 42・`looper_always_tests.rs` 27 ほか）。組み立て `actor_test_support.rs:94`・`:109`・`:114`（`send(ms, ..)`・`tick(ms)`・`hear(ms, cue)`）。`looper_talk_tests.rs` は `.tick(`・`hear(` を 128 回呼ぶ（2.4 の「36 行＋組み立ての呼び出し 43」より多い）。
- 840 行以上: `looper_tests.rs` 975・`parts_tests.rs` 975・`parts_trigger_tests.rs` 906・`looper.rs` 887・`looper_talk_tests.rs` 861。上の 2 本は上限まで 25 行しか無い。ミリ秒の整数から時刻と長さを 1 語で作る組み立てを先に置かないと、整形の折り返しで越える。

### 13.5 起点を見積もる読み手と、押された起点を読むのに要る物

- 見積もる所は 2 つ: `areka-runtime/src/talk_clock.rs:34-53`（`observe_cue`・最大）と `:56-68`（`talk_time`・負は 0）、seriko の `talk.rs:32-52`。見積もりを読む本番は `areka-runtime/src/talk_lifecycle.rs:248`（`now_talk_time`＝バルーンの寿命の時計の元）・`frame/scale_text.rs:201`（文字の層）・seriko の `looper_trigger.rs:163`・`:219`。
- 押された起点を読むのに dola から要る物: ⑴ 合図の絶対の発火時刻（要件 13）。読み手は「起点＝絶対の発火時刻 − `at`」を得る（要件 13 の 6 で、刻みが遅れても揺れない）。⑵ 置き直しの知らせ（要件 12 の 1）。⑶ 合図の無い間（最初の合図の前・合図と合図の間）に起点を持ち続ける事。最初の起点を知らせで配るか、最初の合図から読むかは設計（6 章の 4）。
- 今の見積もりを固定するテスト: `talk_clock.rs` の `new_talk_rebases_epoch_forward`（:237）・`intra_talk_jitter_does_not_lower_epoch`（:286）・`talk_time_clamps_negative_to_zero`（:321）・`clocked_sink_forwards_cue_unchanged_and_observes_at`（:339）・`clocked_sink_forwards_emit_with_after_observing_time`（:449）、seriko の `talk_tests.rs` の `epoch_is_the_largest_now_minus_cue_time`（:18）・`wall_time_rounds_to_the_nearest_millisecond`（:43）。時計を差し込む `controllable_clock`・`set_now`（`talk_clock.rs:129`・`:140`）を使うテストは 10 ファイル・45 行。2.6 は「起点を置き直さない間は今の期待のまま残る」と書いたが、要件 12 の 9 で見積もりそのものが無くなるので、「最大を取る」「揺れで起点が戻らない」の 2 本と seriko の 2 本は、押された起点を読むテストへ置き換え、変える前後の期待として記録する（要件 9 の 6）。

### 13.6 眠りで振る舞いが変わる所

今の時計の上で眠りの間も進む物（一般の知識では `GetTickCount64` は眠りの間も数える。確かめていない）は、替えた後は眠りの間に進まない。

| 所 | 今（復帰した瞬間） | 替えた後 |
|---|---|---|
| 選択待ちの時間切れ（`choice.rs:100-114`・既定 30 秒・起こし `WakeKanadeAt`） | 眠った分が経ったとして切れうる | 眠る前の残りから続く |
| 終わりの握手・送り出しの台詞の再生完了待ちの上限（`close.rs:169-170`・`change.rs:254`・30 秒） | 同上 | 同上 |
| `OnSecondChange` の Reference0（`events.rs:470`） | 眠った分が入る | 入らない |
| 刻み（50・1000・16 ms・`ticker.rs`） | 境を飛ばしてまとめた 1 回と `info!` が出る | 出ない |
| seriko のコマの進み・`periodic`・`always`・毎秒の抽選の境 | 眠った分だけ進む | 眠る前から続く |
| 台本の再生（字・口パク・待ち。刻みの相対の秒） | 眠った分だけ進む | 進まない |
| 文字の層の時計・バルーンの寿命・`FrameTime` | QPC（眠りの間を数えるかは確かめていない・11 章） | 数えない |

範囲の外で、眠りの間も数え続ける物が残る: `std::time::Instant` で測る締め切り（例 `areka-kanade/src/shiori/probe.rs:238-239` の SHIORI の応答の予算・wintf の `Instant::now` 94 行＝2.4）。`instant-to-interrupt-ticks` が着地するまで、2 つの数え方が並ぶ。

### 13.7 `talk-anchor-interrupt-ticks` を名指す、ほかの spec の brief

直さない（棚卸の持ち物）。行番号は測った時点の物。

- `instant-to-interrupt-ticks/brief.md:46`・`:61`・`:87`
- `seriko-talk-clock-fidelity/brief.md:84`・`:86`・`:89`・`:92`・`:94`・`:96`
- `talk-script-metadata/brief.md:42`・`:71`・`:77`・`:91`
- `parallel-visual-script/brief.md:67`・`:73`
- `talk-fast-forward/brief.md:95`
- `talk-thread-count-growth/brief.md:59`
- `window-timed-directives/brief.md:67`
- `sound-canon-tags/brief.md:61`
- `shell-hover-animation/brief.md:65`
- brief の外: `.kiro/steering/roadmap.md:134`・`:164`・`:386`・`:1048`・`:1174`、暫定の設計とその確かめ（`element-animation-easing`・`parallel-visual-script`・`talk-fast-forward`・`sakura-time-directives`・`sound-canon-tags`・`shell-multi-compose`）、`doc/ideas/novel-redesign-2026-11-20/` の 6 ファイル。

### 13.8 依存の辺

- `dola` はワークスペースの crate を 1 つも引かない（`serde`・`thiserror`・`interpolation`・`rand`・機能 `toml`・Windows だけ `windows`）。だから、だれが `dola` を引いても輪はできない。
- 型が要る crate:

| crate | `dola` を引くか | 要る物 | crates.io へ出すか |
|---|---|---|---|
| `areka-kanade` | 引かない（`areka-actor`・`areka-talk`・`shiori-host32-host`・`tracing`・`thiserror`） | 時刻の型・長さの型・符号つきの差（時計は要らない） | 出さない（`publish = false`） |
| `areka-talk` | 依存 0（manifest の説明が「std-only, zero dependencies」） | `ChoiceWaiting` の 2 欄の型 | 出さない |
| `areka-ghost` | 本番は引かない（dev だけ） | 型と時計（刻み役の既定の時計） | 出さない |
| `areka-seriko` | 引く | 型と時計の型 | 出さない |

- 型と時計を `dola` に置くと: `areka-kanade`・`areka-talk`・`areka-ghost`（本番）に `dola` の辺が 1 本ずつ増える。kanade と `areka-talk` は storyboard の層ごと組む事になり、`areka-talk` の「依存 0」の説明は改める。crates.io へ出す物（`tools/crates-io.ps1:68` の `dola`・`wintf`・`wintf-winmsg-executor`）は増えない。
- `dola` の下の小さい crate に置くと: `dola`・kanade・`areka-talk`・`areka-ghost`・seriko がそれを引く。`dola` と `wintf` が引くので、crates.io へ出す物が 1 つ増える（外の crate を足す事にはならない＝要件 15 の 6）。
- `areka-talk` の欄だけ素の整数で持つ形は、要件 4 の 6・17 の 3 の「型のまま」と当たる。

### 13.9 要件の側の課題（4 章の続き）

19. **kanade は時計を読まない（要件 17 の 1）**。kanade の時刻は刻みと選択待ちの知らせで届く（13.1）。「ghost の刻みと seriko の時計は dola の時計から読み、kanade は刻みが運ぶ dola の時計の値で動く」と書くのが正しい。
20. **ミリ秒の整数で外へ出す所（要件 17 の 3・7 の 5・7 の 8）**。kanade・ghost・seriko のログの欄はミリ秒の整数（13.2）で、要件 7 の 8 の括弧「秒の小数」に当たらない。`OnSecondChange` の時（hour）と、OS の待ち（`recv_timeout` へ渡す `Duration`）へ渡す長さも、出す縁で換算が要る。要件 17 の 3 の除く物に「表示・ログ・SHIORI へ出す縁と、OS の待ちへ渡す縁」を足し、7 の 8 を「欄の名前と形（秒の小数・ミリ秒の整数）を変えない」に直す。
21. **起点に押す値が 2 通りに読める（要件 17 の 8・14 の 3・12 の 9）**。17 の 8 は「配る時の dola の時計の値」、14 の 3 は「最初の刻みの絶対の時刻」。刻みの値なら、テストが刻みを注入して決定論で回る（要件 7 の 7）。17 の 8 を「最初の刻みが運ぶ dola の時計の値」にそろえる。あわせて、本番の配り手は `drive.rs:317` の 1 か所だけで、テストは起点を押さない台本を今どおり作る（要件 6 の 4・13.3）ので、「どの配り手でも」は「本番の配り手」に限る。12 の 9 の「再生機が押した起点」も、押すのはトークのスレッドである。
22. **起点より前の「今」（要件 12 の 10）**。フレームの頭で読んだ時刻は、別のスレッドの刻みで押された起点より前になりうる。今は 0 に止めている（`talk_clock.rs:67`・seriko `talk.rs:42`）。要件 12 の 10 に「起点より前は 0」を足す。
23. **刻みを絶対にして要らなくなる本番の分かれ道（要件 7 の 9・14 の 6）**。dispatcher の `base_now`（:101）・逆の写しの丸め（:281-285）・`base_now` が無い時の `warn!`（:268-276）が消える。要件 7 の 9 は「表せなくなった入力のための枝」だけを言うので、「絶対の刻みで要らなくなった枝」も取り除いて記録する物に足す。
24. **眠りの決まりの及ぶ所と確かめ方（要件 17 の 6・2 の 5）**。17 の 6 の「締め切りと台本の再生」は、この spec が替える時計の上の物だけで、`Instant` で測る締め切り（範囲の外）は眠りの間も数え続ける（13.6）。その断りと、確かめ方（偽の時計を止めている間に締め切りも台本も進まない決定論のテスト＋時計の関数の決まりの静的な証跡。眠らせる実機の確かめは要件にしない）を書く。

どれも答えが 1 つに決まるので、自明な修正として要件へ入れられる。開発者の答えで範囲が変わる物は無い（型の置き場は設計の決め。13.8 の「下の小さい crate」を設計が選ぶ時だけ、crates.io へ出す物が増えるので開発者に確かめる）。

### 13.10 設計で決める事（足す分）

（14 章で決めた）

1. 型の置き場（6 章の 1 と同じ決め）: `dola` に置いて kanade・`areka-talk`・`areka-ghost` に辺を足すか、下の小さい crate か（13.8）。
2. 偽の時計の形: `TickerConfig.clock`・`LoopTickerConfig.clock`・`SerikoClock` を「時刻を返す関数」にする。テストの数字は、ミリ秒の整数から時刻と長さを作る 1 語の組み立てで書く（1,000 行に近いファイルの手当て）。
3. 長さを OS の待ちの `std::time::Duration` へ写す 1 か所と、その丸めの向き（切り上げなら、早く起きて空回りしない）。
4. 格子の 0: 周期の倍数を時計の 0 から数える作り（刻み役・seriko の毎秒の抽選）を、そのまま新しい物差しへ移す（50・1000・16 ms は 100ns でちょうど）。
5. `OnSecondChange` の Reference0 の割る数の置き場と、時の値を固定するテスト。
6. kanade の設定の長さ（`close_talk_deadline_ms`・`choice_timeout_default_ms`）を長さの型にする形と、名前。
7. seriko の作者の値の縁（`table.rs` の 3 か所）と、`TalkEpoch`・`TalkFeed` を時刻と長さの型へ替える形（要件 7 の 10 の同じ式・`wall_ms` の丸めを無くす）。
8. 時計の出どころを替える時期（13.11）。

### 13.11 段取りの改め

8.4 の案 2（継ぎ無し・前半と後半）のまま、次のとおり足す。

- **前半（型の付け替え）**: kanade・ghost の刻み・seriko の時刻を、新しい型へ替える。本番の 2 か所の時計（`ticker.rs:80`・`areka-runtime/src/lib.rs:653`）は、型と一緒に dola の時計へ替える。`GetTickCount64` のミリ秒を 10,000 倍して型に載せる継ぎは置かない（要件 15 の 5）。決定論のテストは偽の時計で回るので、時計の出どころが替わっても結果は変わらない。替えた事で変わる振る舞い（眠りの扱い・`OnSecondChange` の値・分解能）は、要件 9 の 6 の「変えた後の期待」に名指しで記録する。dispatcher が送る値は、前半では今の意味（台詞の最初の刻みからの隔たり）のまま、型だけ替える。
- **後半（振る舞い）**: 刻みを絶対の時刻にする（dispatcher の `base_now` を消す・トークのスレッドが最初の刻みを起点に押す・`ChoiceWaiting` の表示の終わりを換算せずに渡す）、文字の層と seriko とバルーンの寿命の時計が見積もりをやめて押された起点を読む、眠りの決まりと `OnSecondChange` の確かめ。
- **並べられる物**: kanade は `areka-talk` と dola の後で、`areka-sakura` と並べられる。`areka-ghost` は kanade と `areka-sakura` の後で、seriko・`areka-emo-text` と並べられる。seriko の本番とテストは、表を組む所（`table.rs` ほか）・ループと部品の芯（`looper*`・`parts*`・`timeline.rs`・`trigger.rs`）・テストの 2 組に割れる。

| 段 | 中身 | 並べられる数 | タスク |
|---|---|---|---|
| 0 | dola の土台（型・変える口・倍率の 1 か所・時計・`Cargo.toml`）と、Windows でない target の検査の段（要件 15 の 8） | 2 | 4 |
| 1 | dola の付け替え（8.5 のまま） | 2〜5 | 10〜11 |
| 2 | `wintf`（3）・`areka-talk`＋`areka-sakura`（3）・`areka-kanade`（本番と組み立て 1・`schedule` のテスト 1・`tests/kanade` 1） | 3〜7 | 9 |
| 3 | `areka-seriko`（本番の時計と表の縁 1・芯 1・テスト 2）・`areka-emo-text`（4）・`areka-ghost`（刻み役と時計 1・dispatcher の家族 1・`prop_*`／`sink.rs`／`tests/ghost/` 1） | 3〜9 | 11 |
| 4 | `areka-runtime`（土台と結線 1・`balloon_visibility*` 3・変更の受け口 1・`frame_*` 1・seriko の時計のテスト 0〜1） | 1 → 5 | 6〜7 |
| 5 | `areka-input`・`areka-session`（偽の刻みの時計を含む）・本体（`thread_roles_tests.rs`・`runtime_tests/spine*` と先進坑を含む） | 1 → 3 | 5 |
| 6 | 前半の締め（残った `f64` の秒と `GetTickCount64` の検索・外したテストの記録・全体テスト 1 回） | 1 | 1 |
| 7 | 後半（止めて戻す 1・起点の置き直し 2・絶対の発火時刻 1・知らせと押された起点を読む側〔`TalkClock`・seriko・バルーンの寿命〕4・刻みを絶対にする〔dispatcher・`drive.rs`・`ChoiceWaiting`〕2・眠りと `OnSecondChange` 1） | 2〜3 | 11 |
| 8 | 文書（8.5 に、ghost・kanade・seriko の時計の説明を足す）と締めの検査 | 2 | 3 |
| 計 | | | **60〜62**（決め 4 を ⑴ にすると +3〜4） |

8.5 の 50〜53 から、kanade（+2）・ghost の刻み役（+1）・seriko の時計（+2）・結線とテストの時計（+1）・後半の読む側と眠り（+2）・検査の段（+1）・決め 5 が ⑴ に決まった分（−2 の見込みが消えた）で増えた。規模は XL のまま、リスクは高のまま（10 章）で、新しく加わる危うさは ⑹ 1,000 行に近い seriko の 5 本（うち 2 本は残り 25 行）と kanade の 3 本 ⑺ 眠りの振る舞いの変化を、決定論のテストでは偽の時計の上でしか示せない事。

## 14. 設計の段の調べと決め

設計の段で調べた事と、決めた事。1〜13 章のうち、この章と食い違う所は、この章が改める。決めの本文は `design.md` に在り、ここには、比べた案と根拠を残す。

案は、設計として必然かどうかで選んだ。タスクの数・触るファイルの数・外すテストの数は、段取りを組むための見込みとしてだけ書く。

### 14.1 要約

- **Feature**: `dola-interrupt-ticks`
- **Discovery Scope**: Complex Integration（今の仕組みの土台の作り直しと、ワークスペースの全体の追従）。調べは full。
- **Key Findings**:
  - 時刻の型と時計は、dola を引かない crate でも要る（`areka-kanade`・`areka-talk`・`areka-ghost` の本番と、後の `instant-to-interrupt-ticks` が触る `shiori-host32-host`・`areka-nar`・`areka-emo-present`・`areka-actor` ほか）。置き場は dola の下が正しい。
  - 起点は 0 より前になる。時刻の表だけでなく、起点を持つ受け手の全部がそれを表せなければならない。
  - 今の口パクの決まり（1 回の刻みでは、いちばん新しい区切りの 1 回だけ）は、刻みの細かさで結果が変わる。大前提（要件 18 の 1）と両立しない。
  - 時刻の表の「進め終えてから配る」形と溜めは、取り出しごとの置き直しと両立しない。溜めを無くすと、同じ時刻の刻み直しの決まり（要件 11 の 9・10）は、守りを書かずに成り立つ。
  - 倍率の写しは、逆を「前向きの写しが届く最小の値」と定義すれば、丸めの向きを合わせる工夫が要らない。

### 14.2 調べの記録

#### main との隔たり

- **Context**: 設計の前に、同じ時に走らせない相手が dola に口を足していないかを確かめる（要件の申し送りの 10）。
- **Findings**: `origin/main` を取り寄せて比べた。ブランチの頭より先の main のコミットは 0。`crates/dola` に触るコミットも 0。先に着地した spec が dola に足した口は無い。
- **Implications**: 2 章の数は、そのまま設計の前提に使える。

#### Windows でない target での検査

- **Context**: 要件 15 の 8 の段が、今の `Cargo.lock` と道具で通るか（11 章）。
- **Findings**: `cargo check -p dola --target aarch64-unknown-linux-gnu` は、今の dola で通る（依存は serde・thiserror・interpolation・rand で、リンクは要らない）。この機械に入っている Windows でない target は `aarch64-unknown-linux-gnu`・`wasm32-unknown-unknown`・`wasm32-wasip1`。全体テストは i686 の target を `rustup target add` の段で入れている（`tools/test-all.ps1` の `add i686 target`）。
- **Implications**: 段は `rustup target add x86_64-unknown-linux-gnu` と `cargo check -p dola --features toml --target x86_64-unknown-linux-gnu` で作れる。CI は `rustup` を持つので、同じ段が通る。

#### OS の関数

- **Findings**: `windows` 0.62.2 の `Win32::System::WindowsProgramming` に `pub unsafe fn QueryUnbiasedInterruptTimePrecise() -> u64` が在る（ソースで確かめた）。失敗を返さない。ワークスペースは今、`windows` の機能で OS の関数を呼んでいる（束縛を自分で書く形ではない）。
- **Implications**: 時刻の土台の crate に、機能 `Win32_System_WindowsProgramming` を足す。dola は時計を持たなくなるので、`windows` の依存（`Win32_System_Performance`）を外す。眠りの間を数えない事は OS の文書の定義で、機械を眠らせる確かめはしない（要件 17 の 10）。

#### 「今」を読む crate

- **Context**: 型と時計の置き場（6 章の 1・13.8）。
- **Findings**: 本番のコードに `Instant::now` を書く crate は、`wintf`（14 ファイル）・本体（10）・`areka-runtime`（6）・`areka-session`（6）・`real-run-kit`（4）・`areka-input`（3）・`shiori-host32-host`（3）・`areka-actor`・`areka-emo-present`・`areka-emo-text`・`areka-ghost`・`areka-nar`・`areka-placement`・`areka-sakura`・`areka-sylphya`・`areka-user-data`（各 1）。このうち dola を引くのは `wintf`・本体・`areka-runtime`・`areka-session`・`areka-emo-text`・`areka-sakura` だけ。`instant-to-interrupt-ticks` の brief は、`shiori-host32-host`・`areka-nar`・`areka-emo-present` を In に挙げ、32 ビットの補助プロセスを議題に挙げている。
- **Implications**: 時計が dola の中に在ると、SHIORI の受け皿やインストールの部品が、時計を読むためだけに dola（serde・rand・interpolation を連れる）を引く事になる。

#### 時刻の表と再生機の、dola の外の使い手

- **Findings**: dola の外で `TimedSchedule` と `to_talk_schedule` の字が出るのは、説明の文だけ（`areka-sakura/src/lib.rs`・`compile.rs`・`drive_delivery_tests.rs`・`areka-runtime/src/talk_lifecycle_tests.rs`）。口を直に呼ぶ所は 0。`CueSheet::absolute_end_time`・`absolute_fire_time` は、`areka-sakura/src/compile.rs:548`（本番）とテスト約 30 か所が呼ぶ（テストの大半は、起点を押していない台本の「相対の終わり」として使っている）。
- **Implications**: 時刻の表の口の形を替えても、dola の外は壊れない。起点を `Option` にすると、起点を押さずに `absolute_end_time` を呼ぶテストは、相対の終わり（`relative_end`）か、起点を押す形へ直る。

#### 受け口の実装

- **Findings**: `CueSink` の実装は本番 19・テスト 20 余り。ほかの受け口を包む物は 2 つ（`areka-sakura` の `StrictTap`・`areka-runtime` の `ClockedTextSink`）。合図の時刻を読む本番の受け手は、文字の層（`at` と `duration`）・`ClockedTextSink`（`at`）・`BalloonLifecycleSink`（`at` と `duration`）・seriko（`at` と `duration`）と、ログだけの `LogSink`。7 つの変更の受け口は読まない。
- **Implications**: 知らせの口（既定が何もしない）を足す時、包む 2 つは中へ通さなければならない。

#### 文字の層の先渡し

- **Findings**: 文字の層は、先渡しの合図を、配られる合図と同じ関数（`apply_cue`）へ、空回しの写しの上で当てる（`areka-emo-text/src/lookahead.rs`・`state.rs` の `rehearsal_copy`）。時刻は使わない。
- **Implications**: 先渡しの合図と、配られる合図の型を分けると、同じ関数を通せない。型は 1 つにし、先渡しの発火時刻は「その時の起点での予定の値」とする。

#### seriko の口パクの今の決まり

- **Context**: 要件 12 の 7 と、6 章の 8。
- **Findings**（`areka-seriko/src/trigger.rs:128-141`・`looper_trigger.rs:110-145`・`trigger_tests.rs:363-461`）:
  - 1 回の刻みで判定するのは、越えた区切りのうち、いちばん新しい 1 つだけ。手前の区切りは捨て、次の刻みへ持ち越さない（`talk_boundaries_crossed_in_one_window_fire_once_at_the_latest`）。
  - 再生の始まりは、刻みの時刻でなく、区切りの字が現れた時刻。過去なら、過ぎた分だけコマを進める。
  - その時刻に再生中なら、見送る。見送った区切りは戻らない（`talk_boundary_reached_while_playing_is_skipped_for_good`）。
  - 数えは前へしか進まない。
- **Implications**: 区切り B1（時刻 T1）と B2（T2）を、細かい刻みで 1 つずつ越えると「T1 で始まり、T2 で再生中なら見送る」。粗い刻みでまとめて越えると「B2 だけを判定して T2 で始まる」。再生の長さが区切りの間より長い時、2 つの結果は違う。同じ台本を受けた 2 つの受け手の口の姿が、刻みの取り方で分かれる。

#### seriko の合図の時刻と、起点

- **Findings**: seriko は、届いた全部の種類の合図で起点の見積もりを動かす（`looper.rs:382`）。本番で合図を手で組む 2 か所（`areka-runtime/src/shell_carry.rs` の着せ替えの運び・`areka-seriko/src/settle.rs` の書き出し）は、`at` 0・長さ 0 の合図を seriko へ直に入れる。台本の途中で着せ替えの運びが走ると、起点の見積もりが動きうる（コードの読み。固定するテストは無い）。台詞を中断した直後の消去は、新しい台詞の 0 秒と、前の台詞の秒を比べている（`seriko-talk-clock-fidelity` の brief の件 3）。
- **Implications**: seriko が字の現れる時刻を絶対の時刻で持てば、起点を持つ必要が無い。手で組んだ合図は字を運ばないので、何も動かさない。消去は絶対の時刻どうしの比べになり、台詞をまたいでも正しい。

#### ノベルの作り直しの文書

- **Findings**: `doc/ideas/novel-redesign-2026-11-20/` の `00`・`01`・`02`・`04`・`07`・`91`・`92`・`93` を読んだ。文書が dola に求める言い方は 2 つ: 「今 `now` が、相対の時刻 `rel` である」ように起点を置く（`01` 5.2）と、「その合図の長さだけ起点を前へ動かす」（`00` 4.1）。置き直しは、区切りで止まったままでも要る（`01` 5.7）。置く時刻は「今」でない事が在る（音の終わりの時刻＝`04` 5.4）。並走の流れは、流れごとの時刻の表と起点を持ち、再生機を通さない（`02` 6.3）。合図を配り・知らせを出し・区切りを解く所は、主の流れと並走で同じ 1 本にする（`91` の 2）。文書どうしで食い違う所（値を時間で変える変化の始まり）は、`00` と `93` が後の決めである。
- **Implications**: 置き直しの口は、時刻の表に置く（再生機だけに置かない）。口の形は「時刻と台本の時刻の組」と「動かす長さ」の 2 つ。1 つずつ当てた結果は、`design.md` の「使い手の居ない口の確かめ」の表。

### 14.3 案の比べ

| 案 | 中身 | 良い所 | 悪い所 |
|---|---|---|---|
| 置き場 A | 時刻の型と時計を dola の中（`dola::time`）に置く | crate が増えない | dola を使わない crate が、時計と型のためだけに dola を引く。`areka-talk` は依存を持たない約束の crate で、`areka-kanade` は cue の語彙を知らない作りである |
| 置き場 B（採る） | dola の下の小さい crate に置き、dola が再公開する | 依存の向きが正しい。std だけで組める | crates.io へ出す crate が 1 つ増える（開発者に確かめる） |
| 型 A | 時刻を符号なし（`u64`）で持つ | OS の関数の戻りの型と同じ | 0 より前の起点を表せない。起点を持つ所の全部に、別の型が要る |
| 型 B（採る） | 時刻を符号つき（`i64`）で持つ | 起点を 1 つの型で表せる。TOML の整数に全部の値が書ける | 表せる上の端が半分になる（約 2 万 9 千年） |
| 時刻の表 A | `tick`＋`ready` を残し、置き直しの時に溜めを空にする | 今の口の形が残る | 取り出しごとに置き直す場所が無い。同じ時刻の刻み直しの守りと、溜めを空にする処理が、置き直しのたびに絡む |
| 時刻の表 B（採る） | 取り出し口を 1 件ずつの `pop_due` だけにする | 置き直しを挟める。守りが要らない | 既存の単体テストは、取り出しを集める手助けを通す形へ直る（確かめる中身は変わらない） |
| 再生機 A | `tick_with(now, 合図ごとに呼ぶ関数)` で、関数が置き直しを返す | 1 回の呼び出しで済む | 呼び戻しの型が口に出る。並走の流れを進める所が、同じ形を使えない |
| 再生機 B（採る） | `step`（1 件）と `tick`（全部） | 時刻の表の `pop_due` と同じ形。置き直しは、呼び手がいつもの口を呼ぶだけ | — |
| 起点の読み方 A | 受け手の全部が、知らせの口だけから起点を受ける（最初の起点も知らせる） | 口が 1 本 | 合図が運ぶ発火時刻と、同じ事を 2 度運ぶ。要件 12 の 9 は、発火時刻から読むと言う |
| 起点の読み方 B（採る） | 文字の層の時計は `fire_time − at` と知らせ。seriko は絶対の時刻で持つ | 見積もりが無い。seriko は起点を持たない | 文字の層の時計は、台詞をまたいで起点を持ち続ける（今と同じ） |

### 14.4 設計の決め

#### 決め: 時刻の土台の置き場

- **Context**: 6 章の 1・13.8・13.10 の 1。
- **Alternatives Considered**: 14.3 の置き場 A と B。
- **Selected Approach**: B。仮の名は `dola-time`。dola は `dola::time` として型と関数を再公開する（「今」を読む関数の定義は 1 つ）。直列化のアダプタは dola が持ち、土台の crate は serde を引かない。
- **Rationale**: 14.2 の「今を読む crate」。この spec の後、時刻の土台の仕様は動かさない。後で置き場を移すと、土台を使う全部の crate の依存を付け替える事になる。
- **Trade-offs**: crates.io へ出す crate が増える。次の公開の時に、新しい名前の初回の公開の手順が要る。
- **Follow-up**: 設計の討議で、開発者に確かめる（`design.md` の未決 1）。名前も、その時に決める。

#### 決め: 型の中身と範囲の端

- **Selected Approach**: `TimePoint`・`TimeDelta` は `i64`、`TimeSpan` は 0 以上の `i64`。足し引きの演算子は端で張り付き、`checked_*` は `None` を返す。「終わりが無い」は `Option`。
- **Rationale**: 0 より前の起点（14.3 の型 A と B）。選択待ちの時間切れは、作者のミリ秒が `i64` の端まで来て、今も端へ張り付けている。パニックは致命に限る決まりである。
- **Follow-up**: 設計の討議で、符号つきにする事を確かめる（未決 2）。

#### 決め: 丸め

- **Selected Approach**: 最も近い 100ns。ちょうど中間は 0 から遠い側。作者の単位・比の掛け算・倍率の掛け算の全部で同じ。
- **Rationale**: 負の台本の時刻（起点より前）でも、0 を挟んで対称になる。

#### 決め: 倍率の写し

- **Context**: 2.3 の ★1〜★6。★1 と ★2 は逆の対で、今は「周回の終わりの時刻で抑える」継ぎで食い違いを隠している。
- **Alternatives Considered**:
  1. 逆を `s ÷ k` の丸めで求め、丸めの向きを前向きと合わせる。
  2. 逆を「前向きの写しが届く最小の値」と定義し、前向きの写しだけを使って求める。
- **Selected Approach**: 2。
- **Rationale**: 1 は、実数の掛け算と割り算の丸めが 2 段に重なるので、向きを合わせても、端の 1 つで食い違う入力が残りうる。2 は、定義から食い違わない（前向きの写しが減らない事だけを使う）。
- **Trade-offs**: 逆を求めるのに、前向きの写しを数回から数十回呼ぶ。引き金の予定の時刻は、開始の時に求めて持つ。

#### 決め: 直列化

- **Selected Approach**: `CueSheet` は 100ns の整数。storyboard の文書は秒。文書の時間の値は、往復がちょうどになる範囲（2^51 の 100ns＝約 7 年）までを読む。
- **Rationale**: 100ns の整数 n を秒の実数で書いて読み戻すと、誤差は n × 2^-52 ほどで、n が 2^51 より小さければ、最も近い整数は n に戻る。範囲を切ると、要件 6 の 7（読む → 書く → 読むで等しい）が、どの値でも成り立つ。

#### 決め: 台本の起点の印

- **Context**: 6 章の 11。今は 0.0 が「押されていない」の印である。
- **Alternatives Considered**:
  1. 既定の 0 を印として残す。
  2. `Option<TimePoint>` にし、押されていない台本から再生機を作れなくする。
- **Selected Approach**: 2。
- **Rationale**: 0 は普通の時刻である。1 だと、起点を押し忘れた台本は「起点がずっと昔」と読まれ、全部の合図が最初の刻みで一度に配られる（要件 18 の 3 の追い付き）。ログ無しの失敗の道になる。

#### 決め: 合図の形と先渡し

- **Selected Approach**: `TalkCue` に `fire_time` を足す。`at` の名前は残し、型で区別する（要件 4 の 5）。先渡しの発火時刻は、その時の起点での予定の値。手で組む所は `TalkCue::scheduled` を通す。
- **Rationale**: 14.2 の「文字の層の先渡し」。

#### 決め: 口パクの決まり

- **Context**: 要件 12 の 7・18 の 1・18 の 6。14.2 の「seriko の口パクの今の決まり」。
- **Alternatives Considered**:
  1. 今の決まりを保つ（いちばん新しい区切りの 1 回だけ）。
  2. 越えた区切りを古い順に全部、それぞれの字が現れた時刻で判定する。
- **Selected Approach**: 2。
- **Rationale**: 2 は、細かい刻みで 1 つずつ越えた時の結果を、粗い刻みでも作る。口の姿が、台本と起点と今だけで決まる。同じ時刻に現れた字（`\_q` の区間）では、1 と同じ結果になる（最初の区切りで始まり、後ろは再生中で見送られる）。「越えた分を積まない」の意図（まとめて越えても、再生を重ねない）は、2 でも保たれる。
- **Trade-offs**: 時刻の違う区切りをまとめて越えた時の結果が、今と変わる。固定しているテストは、変える前後の期待として記録する。
- **Follow-up**: 要件 12 の 7 の字面は「今の決まりで」である。設計の討議で確かめる（未決 3）。

#### 決め: 段取り

- **Context**: 8.4・13.11。
- **Selected Approach**: 8.4 の案 2（継ぎ無し・前半と後半）。前半で、合図と刻みの形を最後の形まで替える。
- **Rationale**: 要件 9 の 1（付け替えで結果が変わらない）と 9 の 6（変えた所）を、別々の確かめで示すため。継ぎを置かないのは、黙って通る変換が、替え残しをコンパイルの誤りから隠すため。
- **Follow-up**: 段ごとの家族の分け方と確かめは、`design.md` の「Migration Strategy」と「File Structure Plan」。

### 14.5 まとめ直し

- **同じ問題の言い換え**: 「起点の置き直しが、もう配った合図に効く」（要件 12）・「受け手が起点を見積もらない」（要件 12 の 9）・「遅れて届いた合図の追い付き」（要件 18）は、同じ 1 つの事である: 受け手は、再生機が押した時刻（発火時刻と、起点の動き）だけを使う。3 つを別々の仕組みにせず、`fire_time` と `origin_moved` の 2 つで作る。
- **在る物を使う**: 1 字ごとの間隔は、`seriko-talk-clock-fidelity` の brief が挙げる 2 つの向き（1 本の関数を両方が呼ぶ・写しを比べるテスト）のうち、1 本の関数にする。置き場は、両方が今も引いている `areka-sakura`（1 字の長さを決める `duration.rs` の隣）。時計は OS の関数をそのまま使い、自前の起点も倍率も持たない。
- **無くす物**: `sum_on_ms_grid`・時刻の表の溜めと同じ時刻の守り・長さの clamp・NaN を「届いた」に倒す比べ・seriko の `TalkEpoch` とミリ秒への丸め・dispatcher の `base_now` と逆の写し・kanade の `MonotonicMs`・dola の `runtime/clock.rs`・「周回の終わりの時刻で抑える」継ぎ・無限大の番兵。
- **足さない物**: 次に届く時刻の照会・区切りの台本の時刻の照会・早送りが済んだ事の知らせ・速さの状態。どれも、この spec の要件に無く、後の spec が、口の形を変えずに足せる。

### 14.6 リスクと手当て

- 前半の間、全体テストが使えない — 段ごとに、下の crate が `cargo test -p` で緑である事を、次の段の入口にする。全体テストは、前半の締めと完了の 2 回。
- 丸めの決まりを誤ると、ちょうどの刻みの届き方が変わる — 丸めを 1 つの crate の 2 つのファイル（`convert.rs`・`scale.rs`）に閉じ、整数の境（1 つ手前・ちょうど・1 つ先）のテストを置く。
- 知らせの口を、包む受け口が中へ通し忘れる — 既定が「何もしない」なので、コンパイルでは止まらない。包む受け口ごとに、通る事のテストを置く。
- 口パクの決まりを変えると、固定しているテストが赤になる — 変える前後の期待を記録に名指しで書く。同じ時刻の字では結果が変わらない事を、テストで示す。
- 1,000 行に近いファイル — テストの数字を 1 語の組み立てで書き、越える見込みのファイルは、先に主題ごとに分ける。
- 眠りの振る舞いは、偽の時計の上でしか示せない — 偽の時計を止める決定論のテストと、時計の関数の説明で示す（要件 17 の 10）。
- 時刻の土台の crate の名前が、crates.io で取れない — 名前は仮で、設計の討議で決める。

### 14.7 読んだ手引き

- `.claude/skills/kiro-spec-design/rules/` の `design-principles.md`・`design-discovery-full.md`・`design-synthesis.md`・`design-review-gate.md`。境界を先に書く事・依存の向きを 1 つに決める事・まとめ直しの 3 つの見方（同じ問題の言い換え・在る物を使う・無くす）を使った。
- `.kiro/steering/` の `tech.md`（dola の依存・全体テストの段・出す crate の一覧）・`structure.md`（テストのファイルの接続の決まり・1,000 行）・`workflow.md`（完了の関門）。

### 14.8 参照

- `windows` 0.62.2 の `src/Windows/Win32/System/WindowsProgramming/mod.rs`（`QueryUnbiasedInterruptTimePrecise`）。
- `doc/ideas/novel-redesign-2026-11-20/`（`00-architecture.md` 4.1〜4.3・`01-playback-control.md` 5.2〜5.9・`02-parallel.md` 6.3〜6.12・`04-sound.md` 5.4・`07-timed-values.md` 5.3〜5.9・`91-validation-seams.md`・`92-validation-code.md`）。
- `.kiro/specs/instant-to-interrupt-ticks/brief.md`・`.kiro/specs/seriko-talk-clock-fidelity/brief.md`。
