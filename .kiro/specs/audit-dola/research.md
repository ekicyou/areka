# ギャップ分析: audit-dola

> 2026-11-06・枝 `claude/kiro-start-audit-dola-58361c`（HEAD `9d0dd4e8`＝`main` の `73c5b25d`＋spec の初期化 1 コミット。`git fetch` 後の `origin/main` も `73c5b25d`＝取り込み済み・`crates/dola/` の差分 0）。
> 物差しは ponytail のプラグインの `ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.3/skills/ponytail-audit/SKILL.md`。5.3.2 の同じファイルとバイト単位で同じ＝`diff` で 0）。
> 作法のうち 2 つを外した（要件 1.3・1.4）: 「最大 20 件」は外して全件を載せる。「読まなかった所（Not checked）」は残さない（範囲の 109 ファイルを全部読んだ）。
> 読み方: 6 本の読み手（読むだけのサブエージェント）に範囲を分けて全行を読ませ（下の §1 の表の「読み手」）、出た指摘をここでまとめ、重なりを 1 件に寄せ、番号を振り直した。載せた指摘は、こちらでファイルと行を開いて中身を確かめた。読み手の行番号が実物とずれていたもの（`tests/runtime/` の一部）は、関数名で引き直した行に直した。確かめられなかったものは落とした。

## 1. 範囲の測り直し（要件 1.2）

| | 起票のとき（main `e3c2cbe4`） | 棚卸㉟（main `18e4cb30`） | 今（HEAD `9d0dd4e8`） |
|---|---|---|---|
| ファイルの数 | 109 | 109 | **109**（Rust 107＋`Cargo.toml`＋`README.md`。fixture のファイルは無い） |
| コードの行 | 8,778 | — | **6,639**（33 ファイル） |
| テストの行 | 15,932 | 18,237 | **18,237**（74 ファイル） |
| `Cargo.toml`＋`README.md` | — | — | 68 行 |
| 合計 | — | 24,944 | **24,944** |

- 数え方: `_tests.rs`・`tests/`・`_test_support.rs` に加え、`src/runtime/*/tests.rs`（`#[cfg(test)] mod tests` の中身・5 ファイル 2,215 行）もテストに数えた。起票のときの「コード 8,778」はこの 5 ファイルをコードに数えていた（名前の決まりでは `tests.rs` が落ちる）。
- 1,000 行の近く: `tests/cue/schedule_test.rs` 960・`src/cue/command_tests.rs` 830・`tests/cue/sheet_test.rs` 783。本番のコードの最大は `src/runtime/facade.rs` 522。1,000 行の番人の例外表（`crates/log-capture-kit/tests/`）に dola のファイルは無い。
- 読み手の割り当て（全行・合わせて 24,944 行）:

| 読み手 | 受け持ち | 行 |
|---|---|---|
| A | `src/cue/`（7）・`src/runtime/clock.rs`・`src/lib.rs`・`Cargo.toml`・`README.md`・`tests/general.rs`・`tests/general/`（10） | 4,132 |
| B | `tests/cue.rs`・`tests/cue/`（11） | 3,729 |
| C | `src/runtime/` の clock.rs のほか全部（16） | 4,609 |
| D | `src/compile/`・`src/validate/`・`src/error.rs`・`builder.rs`・`storyboard.rs`・`transition.rs`・`value.rs`・`document.rs`・`easing.rs`・`variable.rs`・`playback.rs`・`tests/validation.rs`・`tests/validation/`（8） | 4,126 |
| E | `tests/compile.rs`・`tests/compile/`・`tests/trigger.rs`・`tests/trigger/` | 4,548 |
| F | `tests/runtime.rs`・`tests/runtime/`（16） | 3,800 |

## 2. この範囲がしていること・想定した負荷

- **何をするか**: dola は 2 つのエンジンを持つ。
  - **cue の層**（`dola::cue`）: 絶対時刻の台本 `CueSheet` を、時刻で配る `TimedSchedule<TalkCue>` へ直し（`to_talk_schedule`）、受け身の再生機 `CuePlayer` が外から渡された時刻で進めて、登録された全部の受け手（`CueSink`）へ合図を配る。選択肢の区切りで止まり、外からの解決で再開する。占有の終わり（`occupancy_horizon`）に届いたら完了。**本番で毎トーク動いている**（`areka-sakura` の `drive.rs`・`compile.rs`、areka 本体・emo-text・seriko・ghost の受け手）。
  - **storyboard の層**（`DolaRuntime`・`compile_storyboard`・`validate`）: 宣言的アニメの文書（変数・遷移・キーフレーム・ストーリーボード）を読み、検証・コンパイルし、時刻で値を評価して購読者へ差分を配る。**本番に結線されていない**（wintf の `DolaAnimator`／`tick_dola_animators` は `crates/wintf/src/ecs/mod.rs:20` で再公開されるだけで、生成も登録も 0 件＝grep。`dola-interrupt-ticks` の brief 26 行目も同じ）。ただし dola は `publish = true` の公開クレートで、`dola-interrupt-ticks` が storyboard の層を公開の口として扱う。
  - 時計 `dola::runtime::clock::now()`（QPC÷QPF）は本番の 13 か所から呼ばれる。
- **想定した負荷**: 利用者は 1 人。cue の tick は毎フレーム（約 60fps）。台本は大きいことがある（数 KB で線を引かない）。時刻は正確に扱う（設計の大原則・丸めない）。storyboard の層は本番の負荷が 0 なので、Must fix は「公開の API が誤った結果を返す」の意味で、今の画面には出ていない（cue の層の #8 だけは今の本番に出る）。

## 3. 指摘の一覧（全 88 件）

組の名は audit の 3 つ: **Must fix（必ず直す）**・**Should fix（直したほうがよい）**・**Nice to have（できれば）**。番号は 3 つの組を通して 1 から（以後の段はこの番号で指す・要件 1.5）。

- 数: **Must fix 8 件・Should fix 45 件・Nice to have 35 件**。
- 「公開の形」の欄: 直しが dola の公開の形（範囲の外から呼べる型・関数・欄・機能の名前と形）を変えるか。**「変わる」は C18 のあいだ取れない**（要件 6.4・6.5＝議題）。「足す」は C18 でも取れる。「—」は変わらない。
- 「範囲外」の欄: 直しが `crates/dola/` の外のファイルに及ぶとき、そのファイル。**C18 で触れない所**（本体 `crates/areka/` 全部・`areka-emo-text` の `sink*`／`actor*`・wintf の `ecs/mod.rs`／`ecs/world/mod.rs`）には【C18 不可】を付けた。
- 「残す候補」の欄: 直さずに残してよい 3 種類（⑴ 開発者の裁定・完了した spec の決め／⑵ わざと置いた先送りの印／⑶ SSP・ukadoc の正典に関わる振る舞いの変更）に当たるかもしれないもの。決めるのは要件の討議（要件 2.2）。「—」は当たらない。
- 申し送りの件との対応（要件 1.6）: **件 1＝#8**（#9 は同じ行の別の欠陥）・**件 2＝§5**（#15 は同じ型の問題）・**件 3＝§6**（#60 は同じ場所）。

### 3.1 Must fix（8 件）

| # | 場所 | 何が壊れるか（具体的な場合） | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 1 | `src/runtime/facade.rs:430-440`（`process_triggers`）。テストは `tests/trigger/runtime_test.rs` 全部と `tests/runtime/conflict_resolution_test/error_handling.rs:201`〜（どれも `start(..,0.0)`・一時停止なし） | **トリガーが親の開始時刻と一時停止の分だけずれて発火する**。`start_internal` は台本を 0.0 起点でコンパイルする（`facade.rs:96`）ので `fire_time` は相対の値なのに、435 行目で壁時計の `inst.start_time` を引く。`start("parent",10.0)`・kf 1.0 で発火の台本は `wall_fire = 10+(1−10) = 1.0` になり、最初の tick（10.0）で即発火し、子は過去の 1.0 開始として同じ tick で終わる（子のアニメが丸ごと飛ぶ）。0.5 で pause・5.0 で resume すると 5.0 で発火する（正しくは 5.5・`pause_accumulated` を足していない。評価の側 `timeline_manager/mod.rs:238-253` は足す）。本番の時刻（QPC 秒）は 0 にならないので、使い始めればトリガーは必ずこうなる | 436-437 行目を `inst.loop_start_time + inst.pause_accumulated + trigger.fire_time / inst.time_scale` の 1 式に。430-432 行目の古い注も直す。開始 10.0 と、pause→resume の 2 本のテスト | — | — | — |
| 2 | `src/runtime/loop_controller/mod.rs:115-129`（`advance_loop`）。テストの穴 `tests/runtime/loop_offset_test/loop_offset_compile.rs:176-244`（`_changes` を捨てて値を見ない） | **loop_offset の待ちが周回の起点に足されない**。`end_time` には待ちを足すが `loop_start_time` は `+= loop_duration` だけ。無限ループ・1 秒・`Range{2,2}` では、t=1 の進めで待ちが入らず 2 周目がすぐ始まり（完了 `dola-storyboard-random-loop-offset` 要件 2.1「次の周回開始前に待機」に反する）、2 回進んだ後は起点が 2.0・終わりが 7.0 で、t=4〜5 の実効時刻は 2〜3＝3 周目は動かない。#3 と重なって、2 周目の後は値が 2.0 直前（約 0.983）で止まり、最終値 1.0 も届かない（要件 2.4「遅延中は最終値を維持」に反する） | 進める前に `instance.loop_start_time = instance.end_time - instance.pause_accumulated;`（待ち 0 なら今と同じ値）。#3 と組で直し、待ちを固定した（min==max の）e2e で各時刻の値を `expect` する | — | — | — |
| 3 | `src/runtime/timeline_manager/mod.rs:268-274`（`evaluate_segments` の早期 `None`）・`:104-114`（`None` の項目を消す）・`:166-196`（`evaluate_all_for_group`） | **生きているインスタンスでも、変数ごとの最後のセグメントを越えた時点で項目を消す**。x（0→1・1 秒）と y（0→1・2 秒）の台本を 60fps で刻むと、0.992 の次の 1.008 で x の項目が消え、x は 0.992 で凍る。t=2.0 の Conclude の `collect_final_values` にも x はもう無い＝x は 1.0 に届かない。`loop_count=2` なら 2 周目の x は動かない。割り込み（Cancel／Trim）が「相手の終わりの後・次の tick の前」に来ると、`evaluate_all_for_group` が `None` を飛ばして凍結値が空になり、y は 47.5 のまま（自然終了なら 50） | 268-274 行目の早期 `return None` を消す（後ろの 309 行目が最後の to_value を返して値を保つ）。生きた項目を消すのは `remove_entries` だけにする。今の振る舞いを固定するテスト（`timeline_manager/tests.rs` の `expired_entry_auto_removal`・`evaluate_multi_segment_progression`）は直しに合わせて改める（要件 4.5）。長さの違う 2 変数の facade テストを 1 本 | — | — | —（116 行目の「cleanup は呼び出し側」は設計のメモで裁定ではない） |
| 4 | `src/compile/mod.rs:253-259`（`base_duration`）・`:284-287`（`total_base_duration`）。誤りを固定するテスト `tests/compile/integration_test/compilation_flow.rs:119-121`（`== 1.0`） | **台本の長さに先頭の遅れが入らず、動きの途中で終わる**。長さを「最後の end − 最初の start」で出すので、delay 0.5・duration 1.0 は segments [0.5,1.5] なのに長さ 1.0。ランタイムは `end_time = start + total/time_scale`（`facade.rs:99・204`）で t=1.0 に Conclude し、x は途中（0.5）から最終値へ跳ぶ。ループなら後ろの半分は再生されない。`compilation_flow` の "entrance" も実は 1.5 秒なのに 1.0 | `base_duration = segments.last().end_time - start_time`（compile の起点から。完了 `dola-compiled-transition` 要件 5.5 と `compile/types.rs:48` の doc「最終セグメント end_time - start_time」に合う）。`compilation_flow.rs:121` を 1.5 へ。遅れのある 1 本のランタイムのテストを足す | —（値だけ。`total_base_duration` を読むのは dola の runtime だけ＝grep） | — | 付随: 最後のセグメントより後に置いたトリガー（例 `at: kf1 + 0.5`）は、親が先に終わって発火しない。トリガーを長さに足さないのは完了 `dola-nested-storyboard` 要件 3.3 の決め＝⑴（要件 1.1「at/between に従い起動」と食い違う） |
| 5 | `src/compile/resolve.rs:42-83`（`build_dependency_graph`・78-79 行目の注「Sequential entries: no explicit dependency」）・`:176-192`（at の無い純粋 KF） | **前方参照があると、「前のエントリに続く」配置が配列の順でなく処理の順で決まる**。依存のグラフに at と between の辺しか無く、Kahn 法の最小ヒープが辺の無いエントリを先に取る。⑴ [0: x・at "k"・to 1, 1: x・連結・to 2, 2: keyframe "k" at start+5] は、期待 [5,6] 0→1・[6,7] 1→2 に対し、実際は [0,1] 0→2・[5,6] 2→1（時刻も値も逆・エラーは出ない）。⑵ [0: x・at "k", 1: at の無い keyframe "m"（またはトリガー）, 2: "k" at start+5] は正しい文書なのに「Pure keyframe without 'at': no previous entry keyframe time available」で落ちる | at も between も無いエントリに辺を足す（純粋 KF とトリガーは idx−1 へ、連結の遷移は同じ変数の配列上の直前のエントリへ）。上の 2 例をテストに | — | — | ⑴ の確かめが要る: 完了 spec の間で「前エントリ連結」の定めが食い違う（`dola-animation-system`「1 つ前のエントリ」・`dola-compiled-transition` 3.5「同一変数の直前セグメント」）＝どちらに合わせるかは討議で決める |
| 6 | `src/validate/rules.rs:332-360`（`validate_trigger_cycles`）・`:372-406`（`dfs_detect_cycle`。386-388・394-398 行目が `path`／`in_stack` を戻さずに `return`） | **トリガーの循環の検出が、循環に入っていないストーリーボードを循環と報告し、数と中身が走るたびに変わる**。{a→b, b→a, c→a} で、根の順（`HashMap`）が a か b を先に取ると、1 件目 [a,b,a] の後も `path=[a,b]`・`in_stack` が残り、根 c から偽の [a,b,c,a] を 2 件目に出す。c が先なら 1 件。`tests/trigger/validation_test.rs:377-385` は「HashMap 順序依存だが」と集合で比べて逃げている | 根ごとの呼び出しの後で `path.clear(); in_stack.clear();`、根は `doc.storyboard.keys()`（`BTreeMap` の順）で回す（再帰の深さは #49 で反復へ）。{a↔b, c→a} で `TriggerCycle` がちょうど 1 件・中身が完全一致のテスト | — | — | — |
| 7 | `src/runtime/conflict_resolver.rs:153-180`（`detect_overlaps`） | **競合の判定が、インスタンスごとの相対時刻どうしを比べる**（壁時計でも time_scale でもない）。A（Never）を t=0 に開始・x は delay 5 で [5,6]、B を t=5.0 に開始・x は [0,1]。相対の [0,1] と [5,6] は重ならないと判じ、B は Never を無視して始まる（壁時計では両方 [5,6]）。逆向き（壁時計で隣り合うのに競合）は `tests/runtime/conflict_resolution_test/conflict_detection.rs:163`（`time_shifted_start_on_same_variable_still_conflicts`）が固定し「P11 参照」とあるが、その記録はツリーに無い | 消す直しを推す: セグメントの二重ループ（153-180）を消し、「同じ変数に生きた項目があれば競合」にする（今も「大きい group_id が勝つ」ので、後の台本は待ちの間も from_value で上書きしている＝既存のテストの大半はもともとこの結果）。壁時計へ直して比べる案もある（決めが要る） | —（`StartResult.affected_group_ids` の中身がこの場合だけ変わる） | — | —（特性化のテストはあるが裁定の記録は無い） |
| 8 | **件 1**: `src/cue/sheet.rs:85-92`（`absolute_end_time`）・`:165-169`（`to_talk_schedule` の horizon）。広がり: `src/cue/schedule.rs:224`（到達 `start_time + entry_offset > current_time`）・`:339`（`is_completed`）・`:364-366`（`occupancy_horizon`）・`sheet.rs:71-73`（`absolute_fire_time`）。テストの側（T1）: `tests/cue/runtime_test_broadcast_tests.rs:210-250`・`runtime_test_occupancy_horizon_tests.rs:14-53`・`sheet_test.rs:706-740` | **整数 ms で組んだ台本の終わりが ms の格子から外れ、`TalkDone` と後始末が 1 刻み遅れる**。150 ms＋800 ms は `0.15+0.8 = 0.9500000000000001`（`950/1000.0 = 0.95` と違う・python で確かめた）。Tick の式 `(now-base) as f64 / 1000.0` が 0.95 を渡しても完了しない。**新しく分かったこと**: 相対の horizon を格子に載せても、`drive.rs:318` が最初の tick の時刻 `t`（格子の値）をアンカーに刻み、`schedule.rs:339` は `start_time + horizon` を足すので、その和がまた外れる（t=0〜600 s・7 種の長さで測ると 276,924 組のうち 32,049 組＝約 11.6% が格子の値より大きくなる）。各合図の到達（224 行目）も同じ和で、合図が 1 刻み遅れうる。テストは期待値を本番と同じ f64 の和で作る（`tick(d + wait_dur)`）ので緑のまま | 「始まりと長さ（およびアンカーと相対）がどちらも格子に載るなら ms の整数で足してから秒に直す・載らなければ今と同じ f64 の和」を 1 つの関数（例 `sum_on_ms_grid(a, b)`・格子の判定は `(x*1000.0).round()/1000.0 == x`）にして、`absolute_end_time`・`to_talk_schedule` の horizon・`schedule.rs` の到達／`is_completed`／`occupancy_horizon`・`absolute_fire_time` で使う。テストは 7.1〜7.4（150＋800＝0.95 の数の書き込み・アンカー付きの完了・格子外は今と同じ）を足し、T1 の 3 本のうち 1 本を `tick(0.95)` に替える | —（値だけ。要件 7.5 のとおり `absolute_end_time` の形と `append_epilogue` は変えない） | 範囲の外の写し `crates/areka/src/emo2_boot/talk_lifecycle.rs:288`（`cue.at + cue.duration` で同じ終端を自前で足す）【C18 不可】＝dola を直すと 1 ulp 食い違いうる（§7 の議題 8） | — |

### 3.2 Should fix（45 件）

#### cue の層（本番）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 9 | `src/cue/sheet.rs:85-92` と `:133-139・165-169`（件 1 と同じ行） | **終わりの時刻を出す 2 つの口が長さの丸めを別の規則でやり、食い違う**。`absolute_end_time` は `clamp_duration` を通さない。`Cue{start 1.0, duration -5.0}` 1 つなら end_time は 0.0・horizon は 1.0。NaN では end_time がその合図を飛ばし horizon は start を数える。+inf なら end_time だけ inf＝`append_epilogue`（`areka-sakura/src/compile.rs:561`）が締めを置けず、トークが終わらない。`schedule.rs:356-366` の doc は「両者は一致する」と書く。今は `compile.rs:43` の事後条件（有限で非負）で起きない | 91 行目を `cue.start_time + clamp_duration(cue.duration)` に（#8 の関数と同じ所で 1 本に）。[NaN, inf, −3] で `absolute_end_time() == to_talk_schedule(&s).occupancy_horizon()` のテスト | — | — | — |
| 10 | `src/cue/sheet.rs:41-46`・`schedule.rs:142-146`・`:165-169`・`src/compile/mod.rs:229-233`・`:290-295`（どれも `partial_cmp(..).unwrap_or(Equal)`）。注 `sheet.rs:39-40`・`schedule.rs:109-114・152-154` | **時刻の並べ替えの比べ方が全順序でなく、今の Rust では panic しうる**。NaN が何とでも Equal になり推移律が崩れる。std の文書は Rust 1.81 からの `sort_by` に「全順序でなければ panic してよい」と書く（作業場の rustc は 1.99.1）。注は「panic しない」と書く（誤り）。panic は致命の時だけ、の決まりに反する | 5 か所とも `total_cmp`（降順は `b.offset().total_cmp(&a.offset())`）。各 1 行・注も直す。NaN を 20 件以上含む並べ替えのテスト | — | — | — |
| 11 | `src/cue/runtime.rs:201-206`（待機中の tick は schedule を進めない）・`schedule.rs:199-215・236-254`（時間切れ）・`command.rs:93`（`WaitForInput.timeout`）・`:98-99`（`BarrierKind::Timeout`）・`runtime.rs:86-87・268-269` | **`CuePlayer` では `WaitForInput{timeout:Some}` の時間切れが効かない**。`[WaitForInput{timeout:Some(1.0)}@0, Text@0.1]` で `tick(0)` の後 `tick(5.0)` を呼んでも、待機中は早く return して `schedule.tick` を呼ばない＝`TimedSchedule` だけなら 1.0 で解ける区切りが永久に解けない。`WaitForInput` も `BarrierKind::Timeout` も、dola の外で作る所は 0（grep・sakura が置くのは `WaitForChoice` だけ＝`compile.rs:441-462`） | `WaitForInput` の `timeout` 欄と `BarrierKind::Timeout` を消す（`barrier_timeout_offset` の欄・時間切れの枝・`BarrierReached::Timeout` も消える）。`WaitForInput` 自体と `resolve_click` は残し、`talk-fast-forward` の `\x` の受け皿だという実在の理由を doc に書く | **変わる**（`BarrierKind` の形） | `areka-sakura/src/drive.rs:436-447`（`_` で受ける＝書き換え不要） | ⑵ `WaitForInput`・`resolve_click`＝`talk-fast-forward` の brief（82 行目付近「dola の `WaitForInput` と `CuePlayer::resolve_click` は在る」）が `\x` の行き先に挙げる（今は理由の doc が無い）。⑶ ukadoc「`\x` バルーンをクリック待ちにする」（`ukadoc:list/sakura-script/wait/x`）・`\x[noclear]` にも時間切れの引数は無い＝`timeout` を消しても正典に反しない |
| 12 | `src/cue/command.rs:70-80`（`EntityKey`）・`:106-118`（`RoutingCommand`）・`:262-263・278-282`（`CuePayload::Routing`）・`schedule.rs:22-23・54-55・259-262・296-306`（`next_routing`）・`:346`・`sheet.rs:186` | **配送の制御（Routing）が丸ごと死んでいる**。`CuePlayer` は `next_routing` を呼ばない＝台本に `RouteRemove` を入れると `routing_buffer` に積まれ、次に進む tick で記録も無く捨てられる（記録の無い失敗の道）。読み手の `CueQueue` は退役済み。`RoutingCommand`・`CuePayload::Routing`・`Entry::Routing`・`next_routing`・`EntityKey` は dola の外で 0 件（crates/・tools/ を grep） | 型・枝・欄・テスト（`tests/cue/schedule_test.rs`・`sheet_test.rs:624-673`・`runtime_test_preview_tests.rs`・`command_tests.rs`）を全部消す | **変わる** | — | ⑴ 完了 `areka-P0-sakura-engine` の design.md 137-139 行目「Barrier/Routing は M-dialogue の写像先として温存」。M-dialogue は着地済みで、Routing は使われず、今は持ち主の spec が無い（.kiro を grep して 0） |
| 13 | `src/cue/runtime.rs:103-104・204・221-224・286-288`（`filtered_ready`・`ready()`）・`:414-416`（`remaining()`） | **`CuePlayer::ready()` は本番で誰も読まず、待機中や区切りの時間待ちの間は前の合図を返し続ける**。`[Text "before"@0, Timeout{1.0}@0.1, Text@0.2]` で `tick(0.1)` の後 `tick(0.5)` を呼ぶと、schedule は早く戻り集めを据え置くので `ready()` は "before" をまた返す（doc「直前 tick で配送可能になった cue」に反する）。本番は毎 tick 合図を 1 回余分に複製する。dola の外の `.ready()` は `areka/src/menu/trigger_show_tests.rs`（別の型）だけ、`.remaining()` は 0 | `filtered_ready`・`ready()`・`remaining()` を消し、配送の門の中で `self.schedule.ready()` から直に配る（欄が別なので借用は通る）。テスト（`runtime_test_*.rs` の 23 か所）は記録する受け手（`runtime_test_test_support.rs` の `recording_sink`）で見る形へ | **変わる** | — | — |
| 14 | `src/cue/schedule.rs:150-170`（`extend`） | **`TimedSchedule::extend` は同じ時刻の項目を `insert` と逆の順（LIFO）で配る**。`extend([P(1.0,"a"), P(1.0,"b")])` の後 `tick(1.0)` で `["b","a"]`（`insert`／`insert_each` なら `["a","b"]`）。`sheet.rs:151` の doc 自身が「extend は逆順化するため使わない」と書く。呼び手はテストだけ（`schedule_test.rs:526-543・603-610・732-745`）。引く「提案記録 P22」は無い | `extend` と 3 本のテストを消す（または `insert_each` へ） | **変わる** | — | — |
| 15 | `src/cue/runtime.rs:326-340`（`skip_barrier`）・注 `:262-264` | **`skip_barrier` は本番から呼べず、それを逃げ道と書く注が誤り**。選択肢 0 件の `WaitForChoice` で止まったとき、注は「skip_barrier で脱出可能」と書くが、`CuePlayer` は sakura のアクターに閉じ、外から届ける口は「新設しない」と決まっている（`areka-ghost/src/dispatcher.rs:235`）＝本当の逃げ道は Close（player を捨てる＝`drive.rs:388・493` の `drop(player)`）。呼び手は `tests/cue/runtime_test_barrier_tests.rs` だけ | `skip_barrier` を消し、注を「Close（上流が player を捨てる）で抜ける」に。件 2 と同じ判断にまとめる | **変わる** | — | — |
| 16 | `src/cue/command.rs:64-67`（`CueTarget::Window`） | **`CueTarget::Window` を作る所も比べる所も無い**（`cue_target_of` は Shell と Balloon しか返さない・名前の表 `command_target_of` は退役＝`areka/src/emo2_boot/consumer_ledger.rs:4`）。使うのは `command_tests.rs:259・276・792-817` だけ | 値と、`cue_target_window_is_additive_unit_variant`（792-818）と対応表の行を消す | **変わる**（serde の形の値が 1 つ減る） | — | ⑴ 完了 `areka-P0-sakura-dialogue-tags` の design.md 9・158-159 行目が足した（相手の関数は後に退役） |
| 17 | `src/cue/command.rs:151-152`（`CueCommand::EntityRef`）・`sink.rs:129` | **`EntityRef` は本番で誰も作らないのに、4 つのクレートが守りの枝を持つ**（`areka-seriko/src/actor.rs:745-752` の注「今の台本の変換は EntityRef を作らない」）。テスト以外の作り手 0 | 値と、受ける枝とテストを消す | **変わる** | `areka-seriko/src/actor.rs:745`・`actor_dispatch_tests.rs`・`areka-emo-text/src/actor.rs:412`【C18 不可】・`state.rs:674`・`state_cue_apply_tests.rs:581`・`areka-ghost/src/sink.rs:120・252` | ⑴ 完了 `areka-P0-balloon-face-cue` の design.md 104 行目が「EntityRef の M-boot 未対応の warn」を先例に引く |
| 18 | `src/cue/sheet.rs:99-113`（`filter_by_actor`・`actors`）・`:120-123`（`len`） | **本番に呼び手が無い 3 つの口に、テストが 64 行付く**（`actors` は O(n·a)）。`filter_by_actor` は 0 件。dola の外の `.actors()` は emo-text の状態表の別の口（`areka/src/emo2_boot/frame_attach_tests.rs:839` がその違いを書く）。`sheet.len()` は外で 0 | 3 つと `sheet_test.rs:110-173` を消す | **変わる** | — | — |
| 19 | `src/cue/runtime.rs:130-147`（`from_schedule`）・`tests/cue/runtime_test_barrier_tests.rs:264-299` | **`from_sheet` と `from_schedule` の等価を見るテストは落ちようがない**（`from_sheet` の中身が `Self::from_schedule(to_talk_schedule(sheet))` そのもの）。`from_schedule` の外の呼び手は 0（`areka-kanade/src/actor_tests.rs:725` は名前の部分一致で無関係） | テストを消し、`from_schedule` を非公開にする（または `from_sheet` へ畳む） | **変わる** | — | — |

#### cue の層（テスト）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 20 | `src/cue/schedule.rs:291-293`（`notify_barrier_resolved` が集めを空にする）。テスト `runtime_test_barrier_tests.rs:78-80`・`schedule_test.rs:239-245`・`runtime_test_preview_tests.rs:80-91`（どれも解決の後の tick が後の時刻） | **293 行目を消しても `cargo test -p dola` は緑**。本番 `drive.rs:540-543` は解決の直後に同じ時刻で tick する＝この行が無いと冪等の守りに止められ、区切りの後ろの締めが 1 フレーム遅れる。捕まえるのは範囲の外の `areka-sakura/src/drive_choice_tests.rs:602-606` だけ | payload@1.0 → WaitForChoice@1.0 → "after"@1.0 で `tick(1.0)` → notify → `tick(1.0)` の後 `ready()==["after"]` を見るテストを 1 本 | — | — | — |
| 21 | `src/cue/runtime.rs:296-302`（`resolve_click` の守り）・`runtime_test_barrier_tests.rs:207-226` | **非待機での解決が no-op かを Playing でしか試さない**。守りを「WaitingForInput か WaitingForChoice なら解く」に広げても緑＝選択待ちをクリックで解く誤りを捕まえられない | `WaitingForChoice` のときの `resolve_click` が no-op（状態も `pending_choices` も変わらない）のテストを 1 本（#11 で `WaitForInput` を残す前提） | — | — | — |
| 22 | `tests/cue/tick_last_result_test.rs` 全体（57-107・113-131・137-143・18-51） | **落ちようのないテスト 3 本・道具の写し・置き場の誤り**。`update` は tick して複製するだけ（`facade.rs:327-331`）なので等価を見ても落ちない。113-131 は同じ欄を 2 回読んで比べる。137-143 は `tests/runtime/facade_test/diff_delivery.rs` の同じもの。`simple_float_doc`（18-51）は `tests/runtime/facade_test/mod.rs:30-63` とバイト単位で同じ（`diff` で確かめた）。DolaRuntime のテストなのに cue の束（`tests/cue.rs`）にある | ファイルを消し、意味のある `tick_updates_last_result`（173-203）だけ `tests/runtime/facade_test/` へ移してそこの `simple_float_doc` を使う | — | — | — |
| 23 | `tests/cue/sink_test.rs:13-46・99-171・264-313` | **テストが自分で書いた演者を試している**（`honored_durations` はテストのコードが自分で push する）。本番で触るのは `cue_target_of` だけで、それは 178-254 が全部の種類で縛る。127-134・147-160 の「含まない」は直前の `assert_eq` から当然に出る。275-290 は名前を変えて回すが本番は `Custom { .. }` で名前を見ない。294-312 は `src/cue/command_tests.rs:707-787` の写し（302-303 の注がそう書く） | 99-171 と 264-313 を消し（約 −120 行）、`default_emit_with`（325-337）は小さな受け手で書き直す | — | — | — |
| 24 | `runtime_test_occupancy_horizon_tests.rs:55-73` と `runtime_test_broadcast_tests.rs:309-326`（同じ台本・tick・assert）・`occupancy_horizon_tests.rs:14-53` と `broadcast_tests.rs:210-250`・`schedule_test.rs:90-105` と `128-139`・`:298-313` と `162-180`・`:30-41`（`511-524` に含まれる）・clear の 3 本（`107-118`・`182-195`・`468-482`）・`occupancy_horizon_tests.rs:102-109`（`83-100` に含まれる） | **同じことを 2 回確かめる重複テスト**。決まりを変えると直す所が 2 か所に増え、どちらも相手より多くは捕まえない | 各組の後ろの方を消す（約 −110 行） | — | — | — |

#### クレートの設定・時計・値の型

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 25 | `Cargo.toml:20-22・24-26・28-30・35-39`（`json`／`toml`／`yaml` の機能）・`tests/general/integration_test/serialization_round_trip.rs:40-80`・`e2e_full_flow.rs:73-87`・`README.md:13・17` | **`toml`／`yaml` の機能は全体テストで一度も動かず、`json` も本番のコードでは使われない**。`tools/test-all.ps1:66` は機能を足さずに `cargo test --workspace` を回し、dola の機能を有効にするクレートは 0＝`#[cfg(feature="toml")]` のテストは毎回飛ぶ（壊れても赤にならない）。`toml` 0.8 は dola だけが使い、ワークスペースの `toml` 1 と二重に `Cargo.lock` に載る（`Cargo.lock:2226-2239`）。serde_json は `#[cfg(test)]` とテストでしか使わないのに通常の依存として wintf・areka の木に入る | 機能 `json`／`toml` と optional の依存を消し、serde_json を `[dev-dependencies]` へ。toml の往復のテストと `feature_json_enabled_by_default` を消し、README を直す。yaml は `dola-yaml-backend` の持ち物（brief・`deny.toml:42-46`）なので触らない | **変わる**（機能の名前） | `Cargo.lock`（toml 0.8 の項が消える）＝**C18 の約束 ⑹「`Cargo.lock` の席は `placement-crate-split` だけ」に当たる** | ⑵ yaml は持ち主の spec が決まっている。toml・json は — |
| 26 | `src/playback.rs`（25 行）・`lib.rs:13・29`・`tests/general/core_types_test/storyboard_playback.rs:147-180`・`README.md:19` | **`PlaybackState`／`ScheduleRequest` は再公開されているが誰も使わない**（ランタイムは自分の `InstanceState` を使う）。crates/・tools/ の全体で、この 3 ファイルにしか出ない | `playback.rs`・lib.rs の 2 行・テストの節を消し、README を直す（約 −60 行） | **変わる** | — | ⑴ 完了 `dola-animation-system` の要件 6.1〜6.5 が定めた型（design は「型のみ、ロジックなし」） |
| 27 | `src/runtime/clock.rs:68-120`（`now_call_overhead_is_negligible`） | **時間を壁時計で測るテストが決定論でない**。1 万回の平均が 1μs 未満を `Instant` で測る。全体テストの `-j 4` や重いテストと並ぶと、途中でスレッドが 10 ms 横取りされるだけで赤になる（コードは正しいまま）。`eprintln!` も 6 行出す | このテストを消す（単調・正の有限・1 ms の差の 3 本は残す） | — | — | — |
| 28 | `src/value.rs:9`（`derive(PartialEq)`）・`:21-25`（`Eq`）・`:30-54`（`Hash` は `to_bits`）。テスト `tests/general/core_types_test/dynamic_value.rs:111-138`（約束違反を固定） | **`DynamicValue` の等しさとハッシュが食い違う**。`0.0 == -0.0` なのにハッシュが違い（HashMap の約束違反）、NaN は自分と等しくない。Object 変数の値に NaN（YAML の `.nan`）があると、intern の表（`src/runtime/interpolator/mod.rs:19-37`）が評価のたびに新しい鍵を足し（60fps で毎秒 60 件）、`Rc::ptr_eq` の差分が毎フレーム「変わった」と通知する | Float のハッシュで 0.0 と −0.0 を同じに、NaN を 1 つの値に寄せる（2 行）。テストは「等しければ同じハッシュ」を見る形へ反転 | —（`PartialEq` は変えない） | — | —（`value.rs:23-24` は「実用上ない前提」と書くだけで限界の注釈ではない） |

#### storyboard の層（文書の検証とコンパイル）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 29 | `src/validate/rules.rs:27-90・285-326`・`src/runtime/facade.rs:196-216`・NOTE `compile/mod.rs:43-51・225-237・281-283`・`resolve.rs:276-283`・`storyboard.rs:79-81`。特性化のテスト `tests/compile/boundary_test.rs:49-232`・`tests/validation/transition_test/transition_v13_nan.rs`（`v12_nan_tests`）・`schema_test.rs:112-124`（`loop_count_gap_tests`）・`tests/runtime/loop_offset_test/loop_offset_validation.rs:148-194`（`nan_boundary_tests`）・`tests/runtime/facade_test/load_start.rs:261-388`（`time_scale_boundary`） | **数値の有限性と符号を検証しない。テストはその誤りを「今の振る舞い」として固定する**。`time_scale: 0` → `end_time=+inf`（インスタンスが終わらず毎フレーム評価され続ける）。entry が空で `time_scale=0` → NaN。`time_scale: -1` → 開始と同時に Conclude。`duration = nan` は重なりの検査も between の逆転の検査も素通りし NaN のセグメントが出る。`duration = -3` は逆向きのセグメント（total は 0 に隠れる）。`loop_count = 0` は `storyboard.rs:79` の doc で「エラー」なのに validate を通り start で初めて落ちる。`loop_offset: .nan` は 1 周目の後 end_time=NaN で二度と周回しない | rules.rs に規則を 1 つ: delay・duration・offset・trigger_start_offset・time_scale・loop_offset の min/max・initial/min/max・from/to/relative_to が有限・duration≥0・time_scale>0・loop_count は ≥1 か −1。固定しているテストを「拒まれる」側へ直す。負の delay を許すかは決めが要る（今は「前へずらす」をテストで固定するだけ） | 足す（`DolaError` に種類を足すか、既存の `InvalidEntry` を使う。範囲の外で `DolaError` を match する所は 0） | — | —（NOTE が引く P8/P14/P20/P26 は裁定でなく、記録はツリーに無い＝#53） |
| 30 | `src/compile/resolve.rs:17-30`（`entry_keyframe_name`）・`:46-48`・`validate/rules.rs:112-126`（V3）・`tests/compile/boundary_test.rs:253`（誤りを固定） | **利用者の keyframe 名「`__implicit_N`」が暗黙の名前と黙ってぶつかる**。entry0 が名前を省いて `__implicit_0`、entry1 が明示で `keyframe="__implicit_0"`。V2 は明示どうしの重複しか見ないので通り、`kf_to_entry`・`keyframe_times` は後勝ち＝`at "__implicit_0"` の時刻が黙って取り違えられる（1.0 のつもりが 5.0） | V3 で `starts_with("__implicit_")` も予約にする（1 行）。固定するテストを「拒まれる」へ | —（その名前を拒む振る舞いだけ） | — | —（NOTE は提案 P21 を引くだけ） |
| 31 | `src/compile/resolve.rs:154-209`（`resolve_pure_keyframe_time` は `at` だけを見る）・`:67-77`（グラフには between の辺を足す）・`validate/mod.rs:112-116` | **トリガーと純粋 KF の `between` が黙って無視され、前のエントリの時刻で発火する**。[0: "a" at start+1, 1: "b" at start+3, 2: "c" at start+10, 3: trigger・between {a,b}] の entry 3 は 10 秒で発火する。テストは 0 本（`tests/trigger` に between は 0 件）。同じく `trigger_start_offset` をトリガー以外に書くと黙って無視される | between のトリガーの意味を決めて 1 分岐を足す（発火を between.from にする案）か、validate で変数を持たないエントリの between を拒む。`trigger_start_offset` だけのエントリは validate で拒む。どちらにもテストを 1 本 | —（拒む案は validate と `DolaDocumentBuilder::build` の振る舞いが変わる。範囲の外の使い手 `crates/wintf/src/ecs/dola/mod.rs`・`crates/wintf/tests/ecs/dola_animator_test.rs` は between を使わない） | — | ⑴ 完了 `dola-nested-storyboard` の要件 1.1・4.4 と design.md 201 行目「at/between/keyframe はトリガーでも使用可能」（意味の定めは無い）＝拒む案は裁定が要る |
| 32 | `src/runtime/document_store.rs:30-34`（`store` は `validate` だけ）・`src/builder.rs:49-57`・`validate/mod.rs:68-75`（V7 は片向き）・`compile/resolve.rs:261-269`（「shouldn't happen」の枝）・`:211-220`（空の at）・`:307-317`・`compile/mod.rs:68-77` | **validate を通るのに compile で落ちる入力がある。読み込みは成功し、前の正しい文書が捨てられ、start がすべて失敗する**。`{variable:"x"}`（transition 無し）は V7 も V9 も見ないので通り、compile で「Entry has variable but no transition」。`"at": []`・keyframe の循環（a at b・b at a）も validate は見ない。正しい v1 の後にこういう v2 を `load_document` すると Ok が返り v1 が消え、`start` は `CompileError` を返し続ける。`DolaDocumentBuilder::build` も同じ（「正しい」と言われた文書が start で落ちる） | `DocumentStore::store` と `build` で、validate の代わりに全ストーリーボードを `compile_storyboard(&doc, name, 0.0)` にかける（compile は中で validate する）。V7 を「variable と transition は両方あるか両方無いか」にする。前のエントリが失敗したときの連鎖のエラー（`resolve.rs:168・183・322`）にテストを 1 本 | —（壊れた文書を読み込みの時点で拒む振る舞いだけ。範囲の外の呼び手は wintf のテストだけで、どれも正しい文書） | — | — |
| 33 | `src/validate/rules.rs:262-274`（from と to だけを見る）・`compile/resolve.rs:383-400・415-425`・`compile/types.rs:50-55`（`min_value`／`max_value`） | **`relative_to` が値域の検証を抜け、`min_value`／`max_value` はどこからも読まれない**。opacity が [0,1]・initial 0.5 で `relative_to = 0.8` なら to=1.3。validate は通り、ランタイムは 1.3 を出す。2 つの欄を読むのは compile とテストだけ（runtime にクランプは無い・grep） | compile で `resolve_to_value` の後に値域と比べて `ValueOutOfRange` を積む（relative_to を拾える）。クランプを runtime で使うかは決めが要る | — | — | ⑴ 欄そのものは完了 `dola-compiled-transition` の要件 5.6（「ランタイムのクランプ処理用」）の決め＝消さずに使う側を作る |
| 34 | `src/error.rs:105-295`・`Cargo.toml:16`（`thiserror`） | **`thiserror` を依存に置いたまま使わず、`Display` を 190 行の手書きで持つ**（crates/dola を grep して `thiserror`・`#[error` は 0。`RuntimeError` も `runtime/types.rs:119` で手書き）。文を 1 つ変えるたびに match の腕を手で直す | `#[derive(thiserror::Error)]` と `#[error("...")]` を付けて手書きの impl を消す（約 −190 行・`error.rs` のテストの文字列はそのまま）。依存を消す案は `Cargo.lock` の dola の依存の行が変わる（C18 の約束 ⑹）ので取らない | — | — | — |

#### storyboard の層（ランタイム）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 35 | `src/runtime/facade.rs:133-150`・`conflict_resolver.rs:106-129`（`terminate_instance` は `trigger_store` に触らない）・SAFETY の注 `facade.rs:417-422`。残置を固定するテスト `tests/runtime/conflict_resolution_test/error_handling.rs:92-196` | **競合で終わったインスタンスの `trigger_store` が残り続ける**。トリガー付きの親を割り込みで止めるたびに `Vec<CompiledTrigger>` が 1 件残り、DolaRuntime の寿命の間増える（cancel と自然終了の道は `facade.rs:261・514` で消すが、競合の道だけ消さない）。注「conclude/cancel で同時に削除」も事実と違う | `start_internal` の競合の解決の後に `for g in &affected { self.trigger_store.remove(g); }` の 1 行。件数を見るテストを crate の中に 1 本・固定するテストを改める | — | — | —（テストの doc が言う「D1a-V 申し送り」の記録はツリーに無い） |
| 36 | `src/runtime/instance_manager/mod.rs:60`（`HashMap`）・`facade.rs:346-357・408`・`conflict_resolver.rs:31・41-54`（`HashSet`）・`subscription_manager/mod.rs:122`。逃げ `tests/runtime/conflict_resolution_test/conflict_detection.rs:239`（`sort_unstable`） | **同じ tick での自然終了・トリガー・競合の処理順が HashMap 任せで、結果が走るたびに変わる**。親 P1・P2 がどちらも t=1 で、同じ変数 x を触る子 C1・C2 を起こすと、どちらの子が後の group_id を取って生き残るかがプロセスごとに変わる。`StartResult.affected_group_ids` の並びも変わる | `instances` を `BTreeMap<u64,_>`、`detect_overlaps` の集合を `BTreeSet` にする（型を替えるだけ・補助関数も合わせる）。2 親の同時発火のテストを 1 本 | —（`affected_group_ids` の並びが昇順に決まるだけ・範囲の外で並びに頼る所は 0） | — | — |
| 37 | `src/runtime/timeline_manager/mod.rs:129`（`collect_final_values`）・`:222`（`collect_current_segment_final_values`） | **Object 型の最終値だけ intern を通らず、同じ値なのに「変化あり」が出る**。2 つは `Interpolator::interpolate`（プール無し）で毎回新しい `Rc` を作り、差分は `Rc::ptr_eq` で比べる。Object の台本（"a"→"b" の即時切替）を 2 回走らせると、2 回目の Conclude で値は "b" のままなのに change が出る（wintf の ANIM の旗も無駄に立つ） | 2 つとも `&mut self` にし、`interpolate_with_pool(…, Some(&mut self.intern_pool))` を使う | — | — | — |
| 38 | `src/runtime/facade.rs:466-483`（`TriggerResult::Error`）→ `crates/wintf/src/ecs/dola/mod.rs:182-192` | **トリガーの失敗（子の起動の失敗）がどこにも記録されず、テストも無い**。子が Never と競合したとき・差し替えた文書に子の台本が無いとき、`TriggerResult::Error` が結果に載るが、唯一の消費者 `tick_dola_animators` は件数を見て旗を立てるだけで中身を捨てる。`fired` は先に立つので再試行も無い。dola に記録の依存（tracing）は無い。`Err` の枝を `if let Ok` に書き換えて黙って捨てても、どのテストも落ちない（`TriggerResult::Error` を作るテストは 0） | Never の競合で Error が返るテストを 1 本。記録は ⑴ dola に tracing を足す（依存を足す＝C18 の約束 ⑹・要件 5.7 で議題）か、⑵ 消費者の `tick_dola_animators` で `error!` を出す（wintf の `ecs/dola/mod.rs`・C18 の禁止の表には入らない。持ち主は完了した `audit-wintf-graphics-world`） | — | ⑵ の案なら `crates/wintf/src/ecs/dola/mod.rs` | — |
| 39 | `crates/wintf/src/ecs/dola/mod.rs:182-192`（旗の判断）・DolaRuntime に「走っているか」を問う口が無い | **delay の間や loop_offset の待ちの間に、次の画面更新の予約が切れうる**。delay 1.0 の台本では、最初の tick で from_value の change が出た後、変化の無いフレームが続き ANIM の旗が立たない。ほかに起こす者がいなければ心拍の周期まで tick が止まり、動き出しが遅れる。読んでの推定で、心拍がどれだけ埋めるかは測っていない | `pub fn has_instances(&self) -> bool` を足し、旗の判断に使う | **足す** | `crates/wintf/src/ecs/dola/mod.rs:187` | — |

#### storyboard の層（テスト）

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 40 | `tests/runtime/conflict_resolution_test/termination_strategy.rs:10-32`（`cancel_freezes_at_interrupt_time`）・`:73-93`（`conclude_jumps_to_current_segment_final`）・`:146-162`（`trim_cuts_at_interrupt_time`）・`:169-185`（`compress_jumps_to_all_final_values`） | **割り込みの 4 つの戦略のテストが値を見ず、戦略を取り替えても緑**。Trim を `collect_final_values` に替えても、Conclude と Compress を入れ替えても 4 本とも緑（新しい SB が同じ変数を上書きするので確定値は配信に出ない・テストは `!changes.is_empty()` だけ） | 新しい SB が触らない変数（`make_multi_sb_doc` の y）と 2 セグメントの SB で、各戦略の値を `expect` する | — | — | — |
| 41 | `termination_strategy.rs:55`・`error_handling.rs:72`・`tests/runtime/facade_test/termination.rs:87-90`（`_ => {}`）・`:96`・`update_flow.rs:93・96・104`・`loop_integration_test.rs:194・276・345`・`tests/trigger/runtime_test.rs:376-381` | **`if let Some`／`_ => {}` で値の検査が素通りする（12 か所）**。`facade.rs:501` の `force_update_last_values` を消すと `finish` の最終値は届かない（opacity は None）が、`termination.rs:96` は検査せず緑。子 C の評価を壊して t=1.0 で opacity が配られなくても `trigger_chain_a_b_c` は緑 | `.expect("…")` にして値を `assert_eq!`（1 行ずつ）。`termination.rs:87` は clippy の `single_match`（件 3）と同じ所 | — | — | — |
| 42 | `tests/runtime/facade_test/load_start.rs:31・39・52`・`termination.rs:31・97`・`update_flow.rs:28・48・57・94・97・105`・`loop_integration_test.rs:117・122・132・163・174・186・196・232・239・278・306・325・337・347`（25 か所の `< 0.05`） | **時刻の許し幅が 50 ms で、時刻の誤りを隠す**。resume が pause の長さを 40 ms 少なく数えても緑。線形補間は f64 の誤差の内でぴったり合う（「時刻は正確に」） | 許し幅を 1e-9 にする | — | — | — |
| 43 | `tests/runtime/facade_test/diff_delivery.rs:43`（`invalid_document_preserves_existing`）・`:13`（`same_variable_carries_over_value`）・`conflict_resolution_test/error_handling.rs:55`（`never_new_instance_cleaned_up_on_conflict`） | **名前が言う事を確かめていないテスト**。1 本目は有効な空の文書を渡して Ok だけを見る（store が失敗時に既存の文書を消すよう壊しても緑）。2 本目は新しい SB を始めるので、`load_document` が配信の記録を消すよう壊しても緑。3 本目は `conflict_resolver.rs:45` の `remove` を消しても緑（公開の API から片付けが見えない） | 1 本目は schema_version "99.0" で Err を見てから `start("fade_in")` の成功を見る。2 本目は 2 枚目を読んだ後 start せずに update して差分 0。3 本目は crate の中で `instances` の件数を見る | — | — | — |
| 44 | `tests/runtime/loop_offset_test/loop_offset_compile.rs:176-196`（`e2e_with_fixed_delay_extends_playback`）・`:213-244`・`:318-330`（`e2e_extreme_small_delay`）・本番 `facade.rs:107-108`（`Scalar(v)` は 0〜v の乱数）・`:345`（`rand::rng()`＝OS の種） | **loop_offset の e2e テストが assert 0 で、乱数も使う**。#2 が現にあるのに緑。「fixed delay」は名前と違い `Scalar(5.0)` で 0〜5 秒の乱数 | min==max の Range にして各時刻の値を `expect`（#2 の檻を兼ねる） | — | — | — |
| 45 | `tests/runtime/core_types_test.rs:488-508`（`start_result_fields`・`start_result_infinity`）・`src/runtime/types.rs:214`・doc `types.rs:132-133` | **落ちようのないテストと、誤った doc を写したテスト**。構造体に入れた値を読み戻すだけ。`start_result_infinity` は doc「無限ループ時は `f64::INFINITY`」を写すが、本当は 1 周分（`facade.rs:98`・`loop_integration_test.rs` が「INFINITY ではない」を固定） | テストを消し、doc を「無限ループでも 1 周分」に直す | — | — | — |
| 46 | `tests/runtime/` の `relative_to: None` を全欄で手書きした `TransitionDef` 32 か所（`conflict_resolution_test/mod.rs` 11 ほか）・`facade_test/mod.rs:30-63` と `tests/cue/tick_last_result_test.rs:18-51`（同じ） | **文書を組み立てる写しが 32 か所**。`TransitionDef` に欄が 1 つ増えると 32 か所を直す。`TransitionDef` は `Default` を derive し（`src/transition.rs:26`）、`DolaDocumentBuilder` もある | `tests/runtime` の共通の所に `linear(var, from, to, dur) -> StoryboardEntry` を 1 本、文書は `DolaDocumentBuilder` で組む（約 −400 行） | — | — | — |
| 47 | `src/runtime/facade.rs:453-461`（発火済みの印）・`tests/trigger/runtime_test.rs:65`（`trigger_fires_only_once_per_loop`） | **「発火済み」の印を消しても、トリガーのテストが全部緑**。テストのトリガーは全部親の終わり（kf1=1.0）で発火し、親は同じ tick で Conclude される。本番（60fps）で親が発火後も走ると毎フレーム子が作り直される回帰を見逃す | 2 秒の親で kf 1.0 に発火する文書で `update(1.0)`→1 件・`update(1.016)`→0 件のテストに差し替える | — | — | — |
| 48 | `src/compile/mod.rs:289-295`（fire_time の並べ替え）・`tests/trigger/compile_test.rs:87-170` | **並べ替えを消しても、並べ替えのテストが緑**（テストの文書はトポロジカル順が時刻順と同じ）。同時刻の `source_entry_index` の順も未検証 | 遅く発火するトリガーを配列の前に置く文書（期待 [entry2, entry1]）と同時刻の 1 本 | — | — | — |
| 49 | `tests/compile/boundary_test.rs:320`（`long_dependency_chain_compiles_without_stack_exhaustion`・1,000 段）・`tests/trigger/validation_test.rs:300-345`（200 段）。本番 `src/validate/rules.rs:362-406`（再帰の DFS） | **「スタックを使い切らない」と言う 2 本は、再帰に戻しても落ちない**（テストのスレッドは 2 MB）。逆に `dfs_detect_cycle` は本当に再帰で、数万段のトリガーの鎖の細工文書は abort しうる（注は「P23 として提案記録済み」と書くが P23 は無い） | `dfs_detect_cycle` を明示のスタックで反復に（#6 と同じ所）。2 本は `std::thread::Builder::new().stack_size(64*1024)` の中で深い鎖を回す形に | — | — | — |
| 50 | `tests/trigger/runtime_test.rs:386-415`（`parent_cancel_does_not_affect_child`＝cancel を呼ばない）・`:419-443`（`full_lifecycle_with_triggers`＝自然終了を見ない）・`:111-129`（両方と言うが triggered しか見ない）・`:258-265`（`!is_empty()`） | **名前と違うことを見るテスト**。親の cancel が子まで消すよう壊しても、cancel を呼ぶトリガーのテストが無いので気づかない。1 周に 2 回発火しても 262 行目は緑 | 386 は親を `rt.cancel(gid)` してから子の changes を見る形に。419 と 111 は消す（`trigger_starts_child_storyboard` と重複）。262 は `assert_eq!(len, 1)` | — | — | — |
| 51 | `tests/trigger/validation_test.rs:42-69`（439-465 に含まれる）・`:71-107`（347-386 に含まれる）・`tests/compile/metadata_test.rs:333-414` と `tests/runtime/loop_offset_test/loop_offset_compile.rs:54-93`（loop_offset の伝播と往復）・`tests/compile/serde_test.rs:11-46・74-87`（93-136 に含まれる）・`tests/trigger/serde_test.rs:57-85`（`compile/serde_test.rs:138-195` に含まれる・start_offset が出ない確認は 1 本残す）・`tests/compile/transition_test.rs:13-54・174-218`（`compilation_flow.rs` と同じ）・`metadata_test.rs:121-156・260-294・296-331`・`time_resolution_test/sequential.rs:139-174`（176-213 とほぼ同じ）・`tests/trigger/compile_test.rs:33-46` と `58-70`・`tests/validation/transition_test/transition_v7_v10.rs:96-138` と `transition_v13_nan.rs:81-123`（名前と注のほか同じ＝`diff` で確かめた）・`with_transition`（`transition_v11_v12.rs:151-169` と `transition_v13_nan.rs:134-152`） | **同じ内容を 2 回確かめる重複テスト**。名前付き遷移の解決を直すと同じ理由で 2 本が赤になる。loop_offset の伝播を変えると 2 つのテスト binary の 6 本を直す | 「含まれる」側を消し、`with_transition` と `sb(entries)` を `transition_test/mod.rs` に 1 つずつ（約 −500 行） | — | — | — |

#### 両方の層にまたがるもの

| # | 場所 | 何が壊れるか | いちばん小さい直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 52 | `src/runtime/facade.rs:319-331`（`#[deprecated] update`）・`#![allow(deprecated)]` 6 つ（`tests/runtime/{conflict_resolution_test,facade_test,loop_offset_test}/mod.rs:1`・`loop_integration_test.rs:1`・`tests/trigger/runtime_test.rs:1`・`tests/cue/tick_last_result_test.rs:1`）・doc `clock.rs:13`・`facade.rs:29`・`:323`（clippy `doc_lazy_continuation`） | **非推奨の `DolaRuntime::update` を呼ぶのはテストだけ**（約 125 か所）。本番は `tick`＋`last_result`（`wintf/src/ecs/dola/mod.rs:129・138`）。API を 2 本保ち、テストは本番と違う口を回す。`clock.rs:13` の doc は今は無い形 `runtime.update(subscriber_id, time)` を案内する | `update` を消し、テストを `rt.tick(t); let result = rt.last_result();` に置き換える（機械的）。6 つの allow と doc も直す | **変わる**（pub fn を消す・範囲の外の呼び手 0） | — | —（deprecated の注はあるが限界も使い手も無い） |
| 53 | `proposals.md`／P 番号（P8・P9・P11・P14・P15・P16・P20・P21・P22・P23・P25・P26）を引く 35 か所（`compile/mod.rs:43-51・225-228・236-237・281-283`・`resolve.rs:19-24・276-283`・`rules.rs:29-31・281-284・364-371`・`conflict_resolver.rs:176`・`facade.rs:203`・`instance_manager/mod.rs:157・188・224`・`instance_manager/tests.rs:349・366`・`interpolator/mod.rs:166`・`loop_controller/mod.rs:172`・`types.rs:33`・`sheet.rs:39-40`・`schedule.rs:109-114・152-154・183-188`・`tests/validation/schema_test.rs:114-118` ほか）・古いテストの道（`facade.rs:202` の `tests/runtime/facade_test.rs::time_scale_boundary`＝今は `facade_test/load_start.rs:261`・`rules.rs:30` の `tests/validation/transition_test.rs::v12_nan_tests`・`rules.rs:283` の `tests/runtime/loop_offset_test.rs`）・今は無い原本の行番号（`tests/cue/runtime_test_test_support.rs:1-8`・`runtime_test_barrier_tests.rs:1-2`・`broadcast_tests.rs:1-5`・`choice_delivery_tests.rs:1`・`occupancy_horizon_tests.rs:1-2`） | **注が、存在しない提案の文書と古いテストの道を指す**（`git ls-files \| grep proposals` は 0）。読み手は「誰かが先送りを追っている」と取り違える（実際には持ち主の無い先送り）。`runtime_test.rs` は 37 行なのに「原本 `:656-658`」を探させる | #29・#30・#10 などを直すときに NOTE ごと消す。残すものは道を今のファイルへ直す | — | — | — |

### 3.3 Nice to have（35 件）

| # | 場所 | 中身 | 直し | 公開の形 | 範囲外 | 残す候補 |
|---|---|---|---|---|---|---|
| 54 | `src/runtime/document_store.rs:42`（`get_storyboard`）・`timeline_manager/mod.rs:151`（`has_entries`）・`instance_manager/mod.rs:23`（`StoryboardInstance`・`group_id` と `base_duration` は本番で読まれない） | 理由の doc の無い `#[allow(dead_code)]` が 3 つ（使うのはテストだけ） | `#[cfg(test)]` に（`subscription_manager` の `get_variable_name` と同じ形）。読まれない 2 欄は消す | — | — | —（理由の doc が無いので ⑵ に当たらない） |
| 55 | `src/runtime/loop_controller/mod.rs:44-88` と `interpolator/mod.rs:111-176` | イージングの 31 行の対応表が 2 か所。片方を直し忘れても両方とも組み立つ | loop_controller 側を消し、`interpolator::apply_easing` を共用（約 −45 行） | — | — | — |
| 56 | `src/runtime/mod.rs:22`（`pub use interpolator::Interpolator`） | 外の呼び手 0（dola のテストにも 0） | 再公開を消して `pub(crate)` に | **変わる** | — | — |
| 57 | `src/runtime/document_store.rs:13-48` | `Option<DolaDocument>` に validate を添えただけの包み（中身 3 行） | facade に `doc.validate()?; self.document = Some(doc)`（#32 の後なら compile）を書き、ファイルを消す（テスト込み約 −150 行） | — | — | — |
| 58 | `src/runtime/facade.rs:188-189` → `compile/mod.rs:52` | load で検査済みの文書全体（トリガーの循環の DFS 込み）を、start のたび・子の start のたびに検査し直す | compile 側に検査を省く入口を 1 つ | — | — | — |
| 59 | `src/runtime/facade.rs:222-227`・`instance_manager/mod.rs:162-175` | `pause` と `set_pause_start` が 2 段。片方だけだと `pause_start` の無い Paused ができ、防御の分岐（`timeline_manager/mod.rs:245`・resume）が要る | `pause(gid, t)` に合わせ、防御の分岐とそのテストを消す | — | — | — |
| 60 | `src/runtime/instance_manager/mod.rs:71-121`（`create_instance` は引数 15 個・返り値は facade が捨てる） | 件 3 の `too_many_arguments` と同じ所 | facade で `StoryboardInstance` を組み `insert` だけに | —（`pub(crate)`） | — | — |
| 61 | `src/runtime/conflict_resolver.rs:73-85` | Never の検査と値の match が 2 重で、`unreachable!` と `expect` の panic の道がある | `let Some(terminal_state) = InstanceState::from_policy(policy) else { continue };` を値の match の前に置き、Never の腕と `expect` を消す | — | — | — |
| 62 | `src/runtime/subscription_manager/mod.rs:19-20・76・90`（`id_to_name` は解除しても消さない）・`interpolator/mod.rs:19-37`（`ObjectInternPool` に消す口が無い） | 増えるだけの表が 2 つ（購読の解除と再登録の繰り返し・Object の値の違う文書の差し替え） | `unsubscribe`／`unsubscribe_all` で `id_to_name` も消す。`load_document` でプールを空にする | — | — | — |
| 63 | `src/runtime/subscription_manager/mod.rs:94-99・122-128`・`timeline_manager/mod.rs:71`・`facade.rs:366-367` | 60fps の毎フレーム、購読の名前を `String` で複製し `Vec` と `HashMap` を作る | `subscribed_ids` を直に回して `&str` で評価 | — | — | — |
| 64 | `src/runtime/facade.rs:265-270`（`finish(group_id, offset)`） | `offset` は実際には絶対の期限（doc もそう書く） | 引数名を `deadline` に | —（引数名だけ） | — | — |
| 65 | `src/runtime/facade.rs:149`（`compiled.triggers.clone()`） | この後で `compiled.triggers` を使わない | `std::mem::take` | — | — | — |
| 66 | `src/runtime/types.rs:43-51`（`EvaluatedValue` の `Display`） | 使うのはテストだけ | 消す | **変わる**（trait の実装が減る・外の呼び手 0） | — | — |
| 67 | `src/runtime/instance_manager/tests.rs:139-147`（名前に反し `set_finish_deadline` を呼ばず 388-395 と同じ）・`interpolator/tests.rs:495-542`（「対応の正しさ」と名乗るが Float かどうかしか見ず 402-448 と同じ・QuadraticIn と Out を取り違えても通る） | src の中のテストの重複 | 片方を消し、既知の値を 2〜3 個確かめる | — | — | — |
| 68 | `src/compile/resolve.rs:133-147` | `KeyframeCycle` が循環に入っていないエントリまで並べる（a↔b に c at a が加わると「a -> b -> c」） | 残った節から閉路を 1 本たどって出す。または Display を「解けなかった keyframe」へ言い換える（1 行） | — | — | — |
| 69 | `src/compile/mod.rs:89`（`var_segments` が `HashMap`）・`:238-249`（242 行目 `entry_index: 0, // approximate`） | 重なりのエラーが常に entry 0 を指し、2 変数の重なりのエラーの順が走るたびに変わる | 元の `entry_idx` を横に持って報告に使い、`var_segments` を `BTreeMap` に | — | — | — |
| 70 | `src/error.rs:13・45-52`・`validate/rules.rs:112-126`（`_sb_name` を捨てる）・`:262-274`・`:295-323` | `ValueOutOfRange`（遷移の分）と `ReservedKeyframeName` に場所が無い（10 本のうちどれか分からない）。`loop_offset = -1` は `LoopOffsetNegativeMax` と `LoopOffsetRangeInverted` の 2 件になる | 2 つの種類に storyboard（と entry_index）を足す。Scalar の場合は V16 を飛ばす | **変わる**（`DolaError` の欄・外で match する所は 0） | — | — |
| 71 | `src/compile/resolve.rs:193-206`（"start" は必ず在る）・`:250-259`（V5 が先に落とす）・`:410`・`compile/mod.rs:149-152・160-163`（V4 が先に落とす） | どの入力でも通らない枝が約 30 行（読み手は起こりうる失敗と取り違える）。文字列 "no previous entry in array"・"Named transition" を見るテストは 0 | 先頭は `Some(start_time)`、前提は `debug_assert` で表し、残りを消す | — | — | — |
| 72 | `src/compile/mod.rs:203`（`from_value.clone()`）・`resolve.rs:241-271`（`TransitionDef` を毎回複製） | start ごと・子のトリガーごとに compile をかけ直すので、エントリごとに複製する | move と借用に | — | — | — |
| 73 | `src/compile/types.rs:17・42・59・77・93`（`Serialize`／`Deserialize`）・縛るテスト `tests/compile/serde_test.rs`・`compilation_flow.rs:285-319`・`tests/trigger/serde_test.rs:57-85`・`loop_offset_compile.rs:81-93` | コンパイル結果を保存する所は無い（start ごとに作り直す・dola の外に `Compiled*` は 0）のに、derive の往復を約 6 本が守る | derive とテストを消す（約 −250 行） | **変わる** | — | — |
| 74 | `tests/compile/` の `AnimationVariableDef::Float{initial:0.0,min:None,max:None}` の字面 59 か所（trigger に 15）・補助の写し（`boundary_test.rs:13-22`・`metadata_test.rs:351-360`・`error_test.rs:264-268`・`boundary_test.rs:25-38` と `keyframe_and_trigger.rs:169-181`）・空の文書の字面 3 か所・統合テストの中の効かない `#[cfg(test)] mod` 14 か所・`extract_float` の写し（`tests/trigger/common/mod.rs:99-104` と `tests/runtime/loop_integration_test.rs:91-96`） | 欄を 1 つ足すと約 70 か所を手で直す | `tests/compile/common` に `float_var`・`transition_entry` を置き、`..Default::default()` を使い、cfg(test) を消す。`extract_float` は #41 の後に消す（約 −600 行） | — | — | — |
| 75 | `tests/trigger/compile_test.rs:27・29・41-45・65-69・80-84`・`tests/trigger/runtime_test.rs:101-105`・`tests/compile/integration_test/keyframe_and_trigger.rs:219-223` | ちょうどの値が出る時刻（10.0+1.0 など）を 1e-9 の幅で比べる（同じ compile の他のテストは `assert_eq`） | `assert_eq` に | — | — | — |
| 76 | `tests/runtime/core_types_test.rs:10-216` | 状態遷移のテスト 25 本が 49 組のうち 25 組しか見ない（`(Created, Cancelled) => Ok` を誤って足しても緑） | 期待の表 1 本で 49 組（約 −150 行）。`from_policy_results_are_terminal` は残す（`instance_state.rs:64` の doc が名前で引く） | — | — | — |
| 77 | `src/cue/runtime.rs:60-61・237-240`（`PendingChoice.text`） | 本番で読まれず（`drive.rs:432-435` は `id` だけ）、選択肢ごとに複製される | 欄を消す | **変わる** | `areka-sakura/src/drive_delivery_tests.rs:644` | — |
| 78 | `src/cue/schedule.rs:285`（`_choice_id`）・`runtime.rs:316-317` | 使わない引数のために、解決のたびに `String` を作って捨てる | 引数を消す（dola の `tests/cue/schedule_test.rs` の 10 か所も） | **変わる** | — | — |
| 79 | `src/runtime/clock.rs:26` | 起動後に変わらない `QueryPerformanceFrequency` を毎フレームと合図ごとに呼ぶ | `static FREQ: OnceLock<f64>` に 1 度だけ | — | — | — |
| 80 | `src/cue/schedule.rs:222-230` | `last()` を見てから `pop().unwrap()`（unwrap と SAFETY の注が要る） | `while let Some(entry) = self.entries.pop_if(\|e\| !(start + e.offset() > current_time))`（NaN の扱いを変えないよう `!(>)` で） | — | — | — |
| 81 | `src/cue/command_tests.rs:12-19・21-35・37-77`（自分で書いた vec の長さ）・`:438-447`（derive した Hash）・`:571-585`（代入した値の読み返し） | 落ちようのないテスト（822-830 が撤去した「自分を確かめるだけの檻」と同じ型） | 消す | — | — | — |
| 82 | `src/cue/command_tests.rs:633-699`（`duration_is_uniform_envelope_field_across_all_payloads`） | 「全 variant」と言うのに 12 個で `Cursor` が抜けている | `CueCommand::Cursor{..}.into()` を 1 行足す | — | — | — |
| 83 | `tests/general/integration_test/domain_integration.rs:36・70・147・181・244・294・339`・`e2e_full_flow.rs:69` | `build()` は中で `validate()?` を呼ぶ（`builder.rs:49-57`）ので、`.build().unwrap()` の後の `assert!(doc.validate().is_ok())` は落ちない | 8 行を消す | — | — | — |
| 84 | `src/cue/mod.rs:11` | 「CueCommand｜データ系 7 バリアント」（実際は 13・`command.rs:124` も 13） | 数を直す（#16・#17 で減ればその数） | — | — | — |
| 85 | `tests/cue/runtime_test_broadcast_tests.rs:90-98`・`sheet_test.rs:283-285・539・579`・`schedule_test.rs:315-323` | 書いた式をなぞるだけで落ちようがない assert（足し直し・`assert_ne` の重ね・clone==自分・自分で書いた vec の数・notify の前後で変わりえない値） | 消す。315-323 は残すなら「tick(1.0) の後に notify しても `ready()==["x"]` のまま」を見る形に | — | — | — |
| 86 | `tests/cue/schedule_test.rs:842-867`（自前の `Times` の受け手）・`sheet_test.rs` の `Cue {` の手書き 45 か所・Wait の Cue の手書き（`broadcast_tests.rs:31-36・216-221`・`occupancy_horizon_tests.rs:20-25`） | テストの道具の写し（`runtime_test_test_support.rs:13-45・56-71` は runtime_test の下からしか見えない） | test_support を `tests/cue.rs` 直下の mod へ上げ、`wait()` を足して sheet と schedule でも使う（約 −250 行） | — | — | — |
| 87 | `tests/cue/schedule_test.rs`（960 行） | 件 1・#20 のテスト（約 40 行）を足すと 1,000 行の番人が赤 | #14・#24・#85・#86 の削除で約 800 行に（分けなくてよい）。それより前に足すなら兄弟のファイルへ | — | — | — |
| 88 | `tests/cue/runtime_test.rs:20・23・26・29・32・35` | 統合テストの中の `#[cfg(test)]` は効かない（常に cfg(test) で組まれる） | 6 行を消す | — | — | — |

## 4. 指摘にしなかったもの（確かめた結果）

- **DolaRuntime を消す案**: 本番に結線されていないが、アーキテクチャの決め（2 つのアニメエンジン）と `dola-interrupt-ticks` が storyboard の層を公開の口として扱う＝⑴ と見て出さなかった。
- **cue の層の NaN と非有限の特性化**（`schedule_test.rs:545-610`・`sheet_test.rs:175-204`）: 完了した spec の決め（`cue-playback-duration` の design.md 405 行目・`sakura-engine` の research.md 234 行目）で、上流が塞ぐ（`drive.rs:299` の `is_finite` の守り・compile の有限で非負の保証）＝⑴。ただし並べ替えの panic の危うさは別（#10）。
- `rounds_down_value_really_rounds_down_when_subtracted`（`schedule_test.rs:825-828`）は落ちようがないが、続く「ちょうどの時刻」の 4 本の前提の較正なので残す（clippy の `assertions_on_constants` は件 3 で `const { assert!(..) }` に直す）。
- `preview`・`announce_choice_wait`・選択肢の配送列とバッグの檻、`CueProbe`／`emit_with`／`choice_wait_began` は本番の決まりを実際に縛り、範囲の外で使われている。
- `CueSheet` の Deserialize は並べ替えを通らないが、本番の道に serde の読み込みは 0 で、`to_talk_schedule` が並べ直す。
- `TalkCue.at` が相対の秒である件: 読み手（areka の `talk_clock.rs`・seriko の `looper.rs`）はそれを前提に起点を推す＝食い違いは無い。
- 依存: `interpolation`（interpolator・loop_controller）・`rand`（`facade.rs:345`）・`windows`（clock）は使われている。`thiserror` は #34。
- `DolaError` の 21 種類はどれも作られる所がある。`rules.rs:259` の `unreachable!()` は外側の match で到達しない。
- Integer 変数の to=1.5 は補間の後に round する作り。`DolaDocumentBuilder` の同じ名前の上書き（後勝ち）はテストで決めとして固定。`StoryboardBuilder` は compile が必ず validate するので抜け穴でない。
- 大きな時刻の飛びで周回を繰り返す件（P9）: 周回は最短 0.1 秒で、8 時間の休止の後でも約 29 万回の軽い繰り返しで済む。`loop_start_time` に f64 を足し続ける誤差はマスコットの寿命の内で無視できる。
- unwrap（`instance_manager/mod.rs:120`・`timeline_manager/mod.rs:271`）と添字（`facade.rs:429・459`）は不変条件で守られる（#1 の直しでも変わらない）。
- `clock.rs` の `Result` を捨てる件は SAFETY に理由がある（XP 以降は失敗しない・0 で割っても panic しない）。`counter as f64` の精度は 2^53 まで足りる。
- テストの一時フォルダ・`std::env::temp_dir`: 範囲の全ファイルで 0（ファイルに触るテストが無い）。実時間の待ちは `clock.rs` の 2 本（#27 と `now_has_millisecond_precision` の 1 ms の sleep）だけ。
- 割り当ての外で読み手が気付いたもの（持ち場の spec の物）: `crates/areka/src/emo2_boot/talk_lifecycle.rs:288` が horizon の式を dola の外で書き直している（#8 の範囲外の欄）。

### わざと置いた先送りの印（⑵ の候補・今も限界の内側）

- `src/cue/runtime.rs:165-166` の `ponytail:`（受け手を登録するたびに残りの列を写す・「受け手は十数個・合図は多くて数百」）: 本番の登録はトーク 1 つにつき 2〜3（`drive.rs:324-333`）で、写すのは 1 トークにつき受け手の数×1 回（毎フレームではない）。Text の合図は文字の続きごと（`compile.rs:189・320`）。限界の内側＝指摘にしない。
- `shortcut:` の注は 0 件。理由の doc つきの `#[allow(dead_code)]` も 0 件（理由の無い 3 つは #54）。
- `NOTE(D1a-V)`・`NOTE(D2-V)`・`NOTE(D3-V)` の注は限界を名指して P 番号へ先送りするが、先送り先（`proposals.md`）がツリーに無い＝⑵ に当たらない（#29・#30・#49・#53）。
- `#[deprecated] update`（`facade.rs:327`）は「後方互換」と書くが外の使い手が 0＝#52。

## 5. 件 2: `CuePlayer::stop` の呼び手（ツリー全体の grep）

- 定義: `crates/dola/src/cue/runtime.rs:361-366`。
- 呼ぶ所（`.stop()` を `player`／cue で絞った grep・テストと文字列を含む）:
  - `crates/areka/src/emo2_boot/frame/window_state_tests.rs:496`（`script_stopped_before_the_directive_leaves_the_state`・「指令の所へ来る前に中断した台本では状態が変わらない（要件 1.4）」の檻）【C18 不可】
  - `crates/dola/tests/cue/runtime_test_broadcast_tests.rs:293`・`runtime_test_occupancy_horizon_tests.rs:168`
- **本番のコードの呼び手は 0**。`areka-sakura/src/drive.rs` は中断で `stop` を呼ばず、player を捨てる（`drive.rs:388`・`:493` の `drop(player)`）。brief の「本体の `window_state_tests.rs:496` も呼ぶ」は今も正しい。
- 2 つの案（要件 8.2・決めは討議）:
  - ⑴ 語彙の口として残し、doc に「本番の中断は player を捨てる（`drive.rs` の `drop(player)`）。`stop` は捨てずに同じ player で以降の配送を止めたい呼び手のための口で、今の呼び手は areka の `window_state_tests.rs` の中断の檻」と、実在の理由を書く（範囲の中だけで閉じる・C18 で取れる）。
  - ⑵ 消す: `window_state_tests.rs:496` を `drop(player)`（または `player` を作り直す形）に直す要がある＝本体 `crates/areka/` に触る【C18 不可】。受け手の spec は `placement-crate-split` の着地の後の、本体の見直し（`audit-areka-app` など・台帳で確かめる）。
- 同じ型の問題: `skip_barrier`（#15）は範囲の外の呼び手が 0 で、消すのは範囲の中で閉じる（公開の形が変わる＝C18 では取れない）。

## 6. 件 3: clippy の測り直し（要件 9）

- `cargo clippy -p dola --all-targets --no-deps -- -D warnings`（rustc 1.99.1・2026-11-06）: **終了コード 101**。lib で 21 件・lib test で 23 件（21 件は lib と同じ・2 件は `loop_controller/tests.rs`）。brief の 21／23 と同じ。
- **新しく分かったこと**: `-D warnings` では lib が先に止まるので、統合テストの binary は検査されない。`-D warnings` を外して回すと（`cargo clippy -p dola --all-targets --no-deps`・終了コード 0）、統合テストにさらに **3 件**が出る＝**直す場所は全部で 26 か所**。
- brief の記述の誤り: 「`doc_lazy_continuation`〔`runtime/conflict_resolver.rs` 42 行目〕」は違う＝`conflict_resolver.rs:42` は `collapsible_if`、`doc_lazy_continuation` は `runtime/facade.rs:323`（#52 の `update` の doc）。

| 種類 | 件数 | 場所 |
|---|---|---|
| `collapsible_if` | 18 | `validate/rules.rs:36・47・62・73・100・118・214・226・236・263・352`（11）・`validate/mod.rs:47・58・153`（3）・`runtime/timeline_manager/mod.rs:126・182`（2）・`runtime/subscription_manager/mod.rs:166`・`runtime/conflict_resolver.rs:42` |
| `too_many_arguments` | 2 | `runtime/instance_manager/mod.rs:71`（15/7・#60）・`compile/resolve.rs:284`（8/7） |
| `manual_range_contains` | 2 | `runtime/loop_controller/tests.rs:349・377` |
| `doc_lazy_continuation` | 1 | `runtime/facade.rs:323` |
| `assertions_on_constants` | 1 | `tests/cue/schedule_test.rs:827`（較正＝§4・`const { assert!(..) }` に） |
| `module_inception` | 1 | `tests/runtime/conflict_resolution_test/conflict_detection.rs:6`（`mod conflict_detection` の中にもう一度 `mod conflict_detection`） |
| `single_match` | 1 | `tests/runtime/facade_test/termination.rs:87`（#41 と同じ所） |

- 公開の形との関係（要件 9.3）: `too_many_arguments` の 2 か所はどちらも 非公開のモジュールの関数（`instance_manager` は `runtime` の私的な mod・`resolve` は `compile` の私的な mod）＝引数を減らしても公開の形は変わらない（#60 の形なら警告ごと消える）。許す（`#[allow]` に理由）か、引数の束の構造体にするかは討議で決める。
- `clippy-199-lints` の brief（12 行目・88 行目）が dola の段 1 として同じ 21 件を数えている＝本 spec が直した記録を残す（要件 9.4）。統合テストの 3 件はその brief に載っていない。

## 7. 要件と今のコードの対応（Requirement-to-Asset Map）

| 要件 | 今あるもの | ギャップ |
|---|---|---|
| 1（洗い出し） | 本書 §1・§3 | 済み（109 ファイル・88 件・件 1〜3 の対応つき） |
| 2（討議で確定） | §3 の「残す候補」の欄・§9 | 討議で 3 種類の裁定を仰ぐ（**Constraint**）。⑶ に当たる指摘は見つからなかった（#11 は ukadoc で確かめた） |
| 3（全部直す・呼び手まで） | 「範囲外」の欄 | 範囲の外へ及ぶ直し: #8（任意・本体）・#11・#17・#25（`Cargo.lock`）・#38（任意）・#39・#77 と件 2 ⑵＝**Constraint** |
| 4（テストで縛る） | dola のテストは時刻を外から渡す決定論の形（`runtime_test_test_support.rs` の `recording_sink`） | Must fix とバグ・危ない所の各件に落ちるテストが要る（#1〜#10・#20・#21・#28〜#33・#35〜#38）＝**Missing**。ログを見るテストの道具（`log-capture-kit`）は dola に依存が無い＝記録を足すなら要る（**Constraint**・#38） |
| 5（決まり） | 1,000 行の番人 | `schedule_test.rs` 960 行（#87）＝足す前に減らすか兄弟へ。dola に記録の依存（tracing）が無い＝「error! と Err の両方」は dola の中では作れない（#29・#38＝要件 5.7 の議題） |
| 6（範囲の外・C18） | — | 公開の形が変わる指摘 18 件と件 2 の ⑵（#11〜#19・#25・#26・#52・#56・#66・#70・#73・#77・#78、件 2 ⑵）は C18 で取れない（要件 6.5 の議題）。`Cargo.lock` に触る #25 は約束 ⑹ に当たる |
| 7（件 1） | `sheet.rs` の 2 つの和 | #8＋#9。アンカーの和（`schedule.rs`）まで直さないと 7.3 が成り立たない（**Research Needed**） |
| 8（件 2） | `CuePlayer::stop` | §5 の 2 案 |
| 9（件 3） | — | §6 の 26 か所 |
| 10（かけ直し 0 件） | — | 最後のタスクで同じ作法でかけ直す |

## 8. 実装の進め方の案

- **案 A（今のファイルを直す・推す）**: 指摘ごとに今のファイルの中で直す。新しい本番のファイルは作らない（#8 の関数は `sheet.rs` か `schedule.rs` の中に置く）。テストの道具は層ごとに 1 か所（`tests/cue.rs` 直下の support・`tests/runtime` の共通・`tests/compile/common`）へ寄せる。
  - 良い: 今の層と決まりのまま・差分が小さい・消す直しが多い。悪い: `schedule_test.rs` は先に減らす順にする。
- **案 B（層で切る）**: cue の層（本番に動いている）を先に直して全体テスト、次に storyboard の層。
  - 良い: 本番に響く #8〜#10・#20 が早く入る。悪い: #10 のように両方の層にまたがる直しを 2 回に分ける。
- **案 C（組ごと）**: Must fix を先に 1 段で直して全体テスト、次に Should fix、最後に Nice to have。
  - 良い: 危ない所が早く直る。悪い: 同じファイル（`facade.rs`・`timeline_manager/mod.rs`・`schedule.rs`）に 3 回触る。
- 規模の見立て: **L**（1〜2 週間）。理由: 88 件・2 つの層・公開の形を変える 19 件の行き先の決めが要る。危うさ: **中**（#3 と #2 の組・#5 の依存の辺・#8 のアンカーの和は振り方を誤ると時刻の檻が大量に動く。ほかは既にある形の中）。
- 減る行の見立て: 道具の写し・重複・落ちないテストを消して、テストの側で約 −2,000〜−2,500 行。本番は約 −400 行（公開の形を変える削除を含めると約 −700 行）。依存は C18 では 1 つも増やさない（#34 は derive の形なら `Cargo.lock` が変わらない）。

## 9. 討議に渡す議題の種（要件 2）

1. **直さずに残す候補（⑴ 開発者の裁定・完了した spec の決め）**: #4 付随（トリガーを長さに足さない＝`dola-nested-storyboard` 3.3）・#5（「前エントリ連結」の定めが 2 つの完了 spec で食い違う＝どちらに合わせるか）・#12（Routing の温存＝`areka-P0-sakura-engine` design 137-139）・#16（`CueTarget::Window`＝`areka-P0-sakura-dialogue-tags`）・#17（`EntityRef`＝`areka-P0-balloon-face-cue` design 104）・#26（`PlaybackState`＝`dola-animation-system` 6.1〜6.5）・#31（between を拒む案＝`dola-nested-storyboard` 1.1・4.4）・#33（min/max の欄＝`dola-compiled-transition` 5.6）。
2. **直さずに残す候補（⑵ 先送りの印）**: #11 の `WaitForInput`・`resolve_click`（`talk-fast-forward` の `\x` の受け皿＝残すなら理由の doc を足す）・#25 の yaml（`dola-yaml-backend`）・件 2 の ⑴。
3. **⑶（SSP・ukadoc）**: 当たる指摘は見つからなかった。#11 は ukadoc で確かめた: 「`\x` バルーンをクリック待ちにする」（`ukadoc:list/sakura-script/wait/x`）・`\x[noclear]` にも時間切れの引数は無い＝`WaitForInput.timeout` と `BarrierKind::Timeout` を消しても正典に反しない。
4. **C18 の約束 ⑶（公開の形を変えない）に当たる 18 件と件 2 の ⑵ の行き先**（要件 6.5）: #11・#12・#13・#14・#15・#16・#17・#18・#19・#25・#26・#52・#56・#66・#70・#73・#77・#78、件 2 の ⑵。案: ⓐ 本 spec の中で「相手（`placement-crate-split` ほか C18 の 7 本）の着地の後」の段に回す・ⓑ 同じ範囲の 2 本目の spec（下の 12）へ送る・ⓒ ⑴ として残す（#12・#16・#17・#26）。範囲の外の呼び手が【C18 不可】の所にあるのは #17（`areka-emo-text/src/actor.rs`）と件 2 ⑵（`crates/areka/`）。ほかは dola の外の呼び手が 0 か、C18 で触れる所。
5. **C18 の約束 ⑹（`Cargo.lock` の席）**: #25（機能と toml 0.8 の依存を消す）は `Cargo.lock` が変わる＝C18 では取れない。#34 は derive の形なら `Cargo.lock` は変わらない（依存を消す形は変わる）。
6. **記録の依存（要件 5.7）**: dola に tracing が無いので、#29（検証の失敗）・#38（トリガーの失敗）の `error!` は dola の中で出せない。案: ⓐ 依存を足す（C18 の約束 ⑹＝議題・`tech.md` への登記の案）・ⓑ dola は `Err`／`TriggerResult::Error` を返すだけにして、記録は呼び手の境目（wintf の `tick_dola_animators`）で出す（`cue/sink.rs:25` と同じ「記録は呼び手の境目」の作り）。
7. **件 1 の直し方**（要件 7）: ⓐ 推す＝「両方が格子に載るなら整数 ms で足す」関数を 1 つ置き、`absolute_end_time`・`to_talk_schedule` の horizon・`schedule.rs` の到達／`is_completed`／`occupancy_horizon`・`absolute_fire_time` で使う（アンカーの和まで＝§3 #8 の測りで 11.6% が外れる）。ⓑ 相対の horizon だけを直す（要件 7.1・7.2 は満たすが、アンカーの和で 7.3 が成り立たない場合が残る）。どちらも公開の形は変えない。範囲の外の写し `talk_lifecycle.rs:288`【C18 不可】は dola と 1 ulp 食い違いうる＝相手の着地の後に dola の関数を呼ぶ形へ申し送るか、今のままにするかを決める。
8. **件 2 の行き先**（要件 8.2）: §5 の ⑴（理由の doc を書いて残す・C18 で取れる）か ⑵（本体の着地の後に消す・受け手は台帳で確かめる）。
9. **件 3 の許し**（要件 9.2）: `too_many_arguments` 2 か所を直すか（#60 の形で `create_instance` は消える・`resolve.rs:284` は引数の束）、理由を書いて許すか。ほかの 24 か所は振る舞いを変えずに直せる。
10. **#7 の直し方**: 「同じ変数に生きた項目があれば競合」（消す直し）か、壁時計へ直して比べるか。前者は `time_shifted_start_on_same_variable_still_conflicts` の固定と同じ向き。
11. **#31 の意味**: トリガーと純粋 KF の between を「from の時刻で発火」とするか、拒むか（⑴ の裁定が要る）。#29 の負の delay を許すか。
12. **タスクの数と切り分け（要件 2.6）**: 直しをファイルのまとまりで束ねた見立て＝Must fix 8 件で 6〜7 タスク（#2 と #3 は 1 つ・#4〜#6 は compile と validate で 2 つ）・Should fix 45 件で 17〜20・Nice to have 35 件で 8〜10・件 3 で 1・かけ直しと全体テストで 2＝**約 34〜40 タスク**で 20 を越える。切り分けの案:
    - 案 ①（公開の形で切る・推す）: 1 本目（本 spec・C18）＝公開の形を変えない指摘 70 件（Must fix 8・Should fix のうち 33 件・Nice to have のうち 29 件）＋件 1・件 3・件 2 の ⑴＋かけ直し（約 26〜30）。2 本目（同じ範囲・C18 の後）＝公開の形を変える 18 件（Should fix 12・Nice to have 6）＋件 2 の ⑵（約 7〜9）。1 本目はこのままでは 20 を越えるので、Nice to have のテストだけの寄せ（#74〜#76・#81〜#88＝11 件）と Should fix のテストだけの重複の寄せ（#22〜#24・#46・#51）を 2 本目へ回して約 20〜22 にする（2 本目は約 14〜17）。C18 の約束とそのまま合い、2 本目は `dola-yaml-backend`・`dola-interrupt-ticks` の前に取れる。
    - 案 ②（組で切る）: 1 本目＝Must fix＋Should fix（公開の形を変えないもの）＋件 1〜3＋かけ直し（約 20〜23）・2 本目＝Nice to have＋公開の形を変えるもの（約 14〜17）。要件 2.6 の「組ごと」に近い。
    - どちらでも要件 10（かけ直し 0 件）は、1 本目のかけ直しを「1 本目の受け持つ指摘が 0 件（2 本目へ送った指摘と裁定で残した指摘は数えない）」で数える決めが要る。

## 10. 調べが要るもの（Research Needed）

- **#8 のアンカーの和**: `drive.rs:318` が刻むアンカー `t` と、その後の Tick の値の作り（`(now-base) as f64 / 1000.0`）が、いつも ms の格子の値かを設計で確かめる（格子の値なら ⓐ の直しで 7.3 が成り立つ）。到達（`schedule.rs:224`）を格子で足すと、合図が今より 1 刻み早く届く場合がある＝範囲の外の時刻の檻（`areka-sakura` の `drive_*_tests`・本体の `talk_lifecycle_tests` ほか）の期待が動くかを数える（`absolute_end_time()` から期待を作るテストは動かない）。
- **#5**: 2 つの完了 spec の「前エントリ連結」の定めの食い違い（`dola-animation-system` と `dola-compiled-transition` 3.5）。
- **#2・#3 の組**: 待ちの間の値の保持（要件 2.4）と、項目を消さない形での Conclude の最終値の取り方が、既存の `loop_integration_test.rs` の期待とどう合うか。
- **#39**: wintf の心拍が delay の間をどれだけ埋めるか（測っていない・推定）。
- **#10 の panic**: std の文書の約束から言っていて、実際に再現はしていない（NaN を 20 件以上含む並べ替えのテストで再現の形を作る）。
- 指摘 #1〜#7・#20・#21・#40・#47 の「壊しても緑」「誤った結果」は、コードを読んで辿った結果で、赤のテストを回しての確認はしていない（直しのテストで、直しを外すと赤になる形を作る）。

## 11. 設計の段の調べ（2026-11-06・`/kiro-spec-design audit-dola -y`）

- **発見の型**: 今ある仕組みの直し（Extension）＝軽い発見（つなぎ目を中心に）。外の調べ（Web）は要らない＝依存を足さず、標準ライブラリ（`f64::total_cmp`・`Vec::pop_if`〔Rust 1.86〕・`std::sync::OnceLock`・`f64::round`）と、すでに依存にある `thiserror`（ワークスペースの依存・`Cargo.lock` は変わらない）だけを使う。
- **使った決まり**: kiro-spec-design の rules（design-principles・design-discovery-light・design-synthesis・design-review-gate）・ponytail の「いちばん小さな完全な変更」（消せるものは消す・呼び手とテストまで直し切る）・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C18 の行と約束 ⑴〜⑺）。
- **読み方**: 範囲の外のテストの当たりは、読むだけのサブエージェント 1 本に任せた（結論だけを受け取った）。ほかはこちらでファイルを開いて確かめた。

### 11.1 §10 の調べの答え

1. **#8 のアンカーは格子に載るか**: 載る。本番のアンカーは必ず **0.0**。`crates/areka-ghost/src/dispatcher.rs:392-399` は、トークの最初の `now` を起点にして `(now - base) / 1000.0` を送る＝最初の Tick は 0.0 で、`drive.rs:318` がそれを刻む。後の Tick は `n / 1000.0`（格子の値）。0.0＋x は x そのもの＝本番ではアンカーとの和は今も外れない。本番で外れるのは相対の和（合図の始まり＋長さ）だけ。§3 #8 の「約 11.6%」は任意のアンカーで測った数で、本番の形ではない。要件 7.7 はテストと後の `dola-interrupt-ticks`（アンカーが 0 でなくなる）のための決めとして、同じ関数を通す形で守る。
2. **範囲の外で結果が変わるテストの数**: dola の cue を回す範囲の外のテスト（`areka-emo-text`・`areka-sakura`・`areka-seriko`・`areka-ghost`・本体 `areka` の `emo2_boot`・wintf）を全部当たった。**時刻で進めるテストで結果が変わるものは 0 本**。変わるのは `crates/areka-sakura/src/compile_sheet_tests.rs` の **2 本だけ**で、どちらも期待の値を f64 の和で作っているため。
   - `trailing_explicit_wait_remains_in_sheet_and_extends_extent`（333 行目付近）: `absolute_end_time() == d + w`（d=0.15・w=0.8＝f64 では 0.9500000000000001）。
   - `append_epilogue_appends_carrier_at_tail_horizon_without_extending`（459 行目）: `append_epilogue` は `absolute_end_time()`（直した後は 0.95）に置くが、テストの補助 `relative_horizon`（381-386 行目）が f64 の和を返す。
   - 直し: 期待を格子の値（`ms_to_secs(150 + 800)` の形）か `absolute_end_time()` から作る。このファイルは要件 6.3 の触れない所に入らず、C18 のほかの 7 本の触るファイルにも入らない＝**前の段で直す**。
   - 当たらない理由（要点）: emo-text の先読みのテスト（アンカー 100.0・150.0・刻みは `anchor + t`）の合図の時刻は、どれも 50 ms の倍数（文字 50 ms・待ち 150／300／450 ms）で最大約 3.0 秒。アンカー 100.0・150.0 と 50 ms の倍数（20 秒まで）で、f64 の和と格子の和が違う組は 0（python で全部数えた）。drive の各テストのアンカーは 0.0（一部 7.0・10.0・20.0）で、刻みは字面か `absolute_end_time()` から作る。自前の和（0.25+0.8 など）は 1 つずつ確かめて格子の和と同じ値。本体の `emo2_boot` のテストは期待を `absolute_fire_time()`／`absolute_end_time()` から作る＝一緒に動く。wintf の `dola_animator_test.rs` は storyboard の層だけで cue を使わない。
3. **ずれの向き**: 格子の和と f64 の和は、どちら向きにも 1 ulp ずれる（始まり 0〜5 秒の 1 ms 刻み × 長さ 9 種で、f64 の和が小さい組 5,916・大きい組 4,959）。f64 の和が格子より小さい組では、「f64 の和の時刻ちょうどで刻んで届く・終わる」を見るテストが「まだ」に変わる＝dola の中のテストで期待を f64 の和で作っているもの（§3 #8 の T1 の 3 本ほか）は、期待を格子の値か `absolute_end_time()`／`occupancy_horizon()` から作る形へ改める（要件 4.5）。
4. **#2・#3 と `loop_integration_test.rs` の期待**: このファイルの台本は 1 変数・delay 0・loop_offset 無し（`loop_doc`・`short_duration_loop_doc`）＝最後のセグメントの終わりと周回の終わりが同じ時刻で、周回の処理（`update_internal` の Step 2）が評価（Step 3）より先に走る＝#2・#3・#4 の直しで期待は変わらない。変わるのは loop_offset のテスト（#44 で書き直す）と `timeline_manager/tests.rs` の 2 本（`expired_entry_auto_removal`・`evaluate_multi_segment_progression`）。
5. **#39 の心拍**: 測らない。wintf の起床の旗は大域の静的な値（`crates/wintf/src/ecs/world/tick_wake.rs:260`）で、並んで走るテストからは立つ・立たないを確かめられない。走っているインスタンスがある間は毎フレーム起こすのが正しい形なので、心拍がどれだけ埋めるかは直しの要否に関わらない。檻は dola の側（delay の間、変化が 0 件でも `has_instances()` が真）に置き、旗の行は 1 行の結線にする。
6. **#10 の panic**: 再現の形は作らない（std の約束は「panic してよい」で、必ず panic するわけではない）。檻は「NaN を 20 件以上含む並べ替えが `total_cmp` の順（正の NaN が末尾）になる」を見るテスト。今の `partial_cmp(..).unwrap_or(Equal)` では NaN の置き場が入力の順で変わるので、このテストは直しの前に赤になる。
7. **#5**: 要件 11.7 で決着（変数を持つ遷移は同じ変数の配列上の直前のエントリへ・変数を持たないエントリは配列の 1 つ前へ）。

### 11.2 合成（design-synthesis）

- **一般化**: 件 1 の 7 か所の和（#8・#9・要件 7.7）は「2 つの秒を足す」という 1 つの問題＝関数 `sum_on_ms_grid` 1 本。#9 の「2 つの口が別の規則で終わりを出す」は、相対の終わりを出す私的な関数 1 本を 2 つの口が呼ぶ形で消える。#29・#30・#31・#32・#58 は「読み込みで通った文書は start で落ちない」という 1 つの約束にまとまる＝読み込みのときに全ストーリーボードを compile にかけ、start は検査を省く crate の中の入口を使う。
- **作るか借りるか**: 依存を足さない（C18 の約束 ⑹・要件 5.6）。上の std の関数と、すでにある `thiserror` を使う。格子の判定も std の `round` だけで足りる。
- **単純化**: 新しい本番のファイルは作らない。消すファイル＝`src/runtime/document_store.rs`（#57）・`tests/cue/tick_last_result_test.rs`（#22）・後の段の `src/playback.rs`（#26）と `tests/compile/serde_test.rs`（#73）。足すファイルは `src/runtime/facade_tests.rs`（DolaRuntime の中の欄を見るテストの置き場・#35・#43・#57 の移し先）の 1 つだけ。

### 11.3 設計で決めたこと（要件 11 のほか）

- `sum_on_ms_grid` は前の段では `pub(crate)`（`src/cue/schedule.rs`）。後の段（要件 7.9）で `pub` にして `dola::cue` から再公開する（後で使うための口を先に開けない）。
- **#28**: 等しさとハッシュを同じ鍵（±0 を 1 つ・NaN を 1 つ）で作る。§3 の「`PartialEq` は変えない」案では、NaN の鍵は表で見つからない（HashMap は等しさで引く）ので intern の表が増え続ける欠陥が残る＝`PartialEq` を手書きにして Float の腕だけ鍵で比べる。範囲の外で `DynamicValue` を使う 13 ファイル（多くは本体 `emo2_boot` の合図のテスト）が NaN どうしの不等に頼っていないことを、実装のときにツリー全体で確かめる（要件 3.2）。
- **#29**: 失敗の種類 `DolaError::InvalidNumber { location, field, value }` を 1 つ足す（足すだけ＝C18 で取れる）。エントリの形の誤り（#31 の `trigger_start_offset`・#32 の V7）は既存の `InvalidEntry` を使う。
- **#39**: `has_instances()` は一時停止中のインスタンスも数える（一時停止のあいだに finish の期限が来る場合も刻みが要る＝正しい側に倒す）。
- **テストの読み替えは前の段で**: #52（`update` → `tick`＋`last_result`・6 つの `#![allow(deprecated)]` も外す）・#13（`CuePlayer::ready()` を読むテスト → 記録する受け手）・#19（同値のテストを消す）は、テストだけの直しなので前の段で済ませる。後の段は公開の口を消すだけにして、相手の着地を待った後の差分を小さくする。
- **#51 の `Compiled*` の serde の分**（`tests/compile/serde_test.rs`・`tests/trigger/serde_test.rs:57-85`）は、#73 が後の段でまとめて消すので、前の段では触らない（同じ所を 2 回直さない）。
- **件 2** は前の段では何もしない（要件 8.1）。後の段で `stop`・`skip_barrier` を消す。
- **要件 7.4 と #9 の読み合わせ**: 7.4 の「今と同じ値」は「格子へ丸めない」の意味と読む。非有限・負の長さの合図は #9 で clamp を通るので、`absolute_end_time` だけは今と違う値（`occupancy_horizon` と同じ値）になる。本番の台本は `areka-sakura/src/compile.rs:43` の事後条件でこの形を作らない。
- **件 1 の檻のアンカー**: 「ちょうど届いたら完了」（7.3・7.7）の檻はアンカー 1.0・刻み `1950.0 / 1000.0` で組む。直す前の和 `1.0 + 0.9500000000000001` は 1.95 を越える＝直しを外すと赤になる（アンカー 1〜19,999 ms のうち 1,573 通りがこの形・python で数えた）。
- **件 3 の `doc_lazy_continuation`**（`facade.rs:323`＝#52 の `update` の説明）: 前の段で説明の字下げを直して clippy を緑にし、後の段で `update` ごと消す（要件 9.1 は前の段の終わりでも満たす）。

### 11.4 設計の見直しの門（design-review-gate）

- 1 回目: 要件の番号の機械の当たりで 1.2・1.3・1.4・1.6・9.2・10.2 が範囲の書き方（「1.1〜1.7」）のため字面で見つからなかった＝対応表を 1 つずつの番号に直した。判断の見直しで、要件 7.4 と #9 の読み合わせ（上の 11.3）と、件 1 の 7.3 の檻が直す前に赤になるアンカーの選び方を足した。
- 2 回目: 番号 71 件すべてが設計に載る・境目の 4 節が空でない・ファイルの計画に道がある・部品はどれもファイルの道を持つ＝通った。要件の穴や食い違いで止める所は無い。
