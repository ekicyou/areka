# 9.1 設計と実装の再突合（2026-08-23・読み取り専用の検査結果）

> 9.1「設計の file:line と実装の一致を再突合」の成果物。9.4（登記）で design.md／requirements.md の改訂に使う。
> 検査対象は HEAD（`0c59a572`〜`bb204110`）。`git diff origin/main --stat -- '*Cargo.toml'` は空（要件 8.6 成立）。origin/main からの先行コミットは 45。

## (A) design.md の file:line 引用 → 現況

検査 68 件。**OK 21 件**（`Cargo.toml:48-56`／`:63-93`・`monitor_systems_transition_tests.rs:367-371`・`transition_diag_tests.rs:778-782`・`visual_sync.rs:25-70`・`pointer/systems.rs:17-33`・`frame.rs:158-233`・`actor.rs:640`・`:744-805`・`controller.rs:416-457`・`controller.rs:212`・`monitor.rs:34`・`ticker.rs:57-65`・`ticker.rs:262`・`transition_diag.rs:54`・`:622-627`・`window_pos.rs:290`（`ecs/window_proc/`）・`frame_harness_tests.rs:397`・`kiro-validate-impl/SKILL.md:72-84`・`adapter.rs` PresentBridge・UI スレッド登録点 `runtime/mod.rs:134`）。以下は SHIFTED／STALE／N/A。

| design.md | 引用 | 状態 | 現況 |
|---|---|---|---|
| L60, L745 | `tick_bridge.rs:65-68`（wintf-vsync） | SHIFTED | `:67-69`。登録は `vsync_loop` 内 `:119-123` |
| L60 | `tick_bridge.rs:114-134`（vsync_loop/DwmFlush） | SHIFTED | `fn vsync_loop` `:115`・`DwmFlush` `:129` |
| L60 | `tick_bridge.rs:218-236`（run_async_tick） | SHIFTED | `:270` |
| L60, L821 | `tick_bridge.rs:187-210`（tick_one_frame） | SHIFTED | `:208-228`。注入口 `tick_one_frame_with` `:230-268` |
| L61 | `world/mod.rs:488-566`（try_tick_world） | SHIFTED | `:606-712` |
| L61 | `:490／:493／:517-524／:525-534／:536-541／:545／:548-560／:563` | SHIFTED | `608／616／620-648／635-658／659-664／668／680-704／707-709` |
| L61, L228 | `world/mod.rs:657-702`・`:707`（順序テスト） | SHIFTED | `:817`・`:867` |
| L62 | `world/mod.rs:104-160`＋`:117,135,141,146,151,156` | SHIFTED | 区間 `:120-180`・`SingleThreadedExecutor` は `137,155,161,166,171,176` |
| L826 | `world/mod.rs:108-112,138,159`（C17） | SHIFTED | 同上 |
| L62 | `bevy_ecs-0.19.1/…/multi_threaded.rs:274` | N/A | vendor 外・未検証 |
| L63, L817 | `frame/scale_text.rs:255-275`／`:255` | SHIFTED | `fn run_text_phase` `:274` |
| L63 | `balloon_visibility_phase.rs:64-95` | SHIFTED | `:66` |
| L64 | `runtime/mod.rs:307-328`（click_wake 中継） | SHIFTED | `:311-330` |
| L64, L745 | `clickthrough/monitor.rs:87-88` | SHIFTED | 名前 `:88`・spawn `:90`・登録 `:144` |
| L64, L711 | `ticker.rs:203-206,223-226,305-308`（catch-up） | SHIFTED | `202-205, 222-225, 304-307` |
| L745 | `ticker.rs:179,289`（登録点） | **STALE** | ticker.rs に登録は無い。`areka_actor::install_thread_start_hook`＋`areka/src/thread_roles.rs:55`（Implementation Notes 2.3） |
| L745 | `areka-actor/src/spawn.rs:48-49`（登録点） | **STALE** | `install_thread_start_hook` `:49`・フック呼出 `:102-103`。wintf 呼出は無い |
| L65 | `command.rs:49` `AtomicI32` | **STALE** | `thread_local! … Cell<i32>` `:70`（task 4・意図どおり） |
| L65, L228 | `command.rs:76-79／:86-88／:96-114／:129-155／:657-679` | SHIFTED | `104／117-119／128-146／164／692` |
| L66 | `transition_diag.rs:633-635`（emit_line） | SHIFTED | `:631` |
| L66 | `invoke-perf-run.ps1:102-105` | SHIFTED | `RUST_LOG_VALUE` `:30`/`:106`・`-ConfirmQuiet` `:132`・exit 2 `:66` |
| L66 | `judge-perf.py:106/364/380/396/451-452/466/588/3470-3490` | SHIFTED | `150／412／432／457／512-513（+528,535 新設）／661／783／3691,3739` |
| L66 | `main.rs:126-128`（subscriber）・`:793`（AREKA_APP_SMOKE_EXIT_MS） | SHIFTED | `136-139`；`725`（使用）/`828`（定数） |
| L67 | kiro-impl「派遣 3 箇所（model 指定無し＝継承）」 | **STALE** | 規則 `:47-48`・派遣 `:100/:118/:149`・引き渡し `:188` |
| L67 | 「`.claude/agents/` は未作成」 | **STALE** | `perf-{measure,analyze,implement,review}.md` の 4 本が存在 |
| L817 | `emo2_boot/adapter.rs:87-94`（PresentBridge::send） | SHIFTED | `:109` |

## (B) File Structure Plan との差分

**追加（計画に無い）**: `tools/perf/perf-loop.common.ps1`・`perf-loop.measure.ps1`（7.3 分割）／`perf_ledger_goal.py`・`perf_ledger_selftest.py`（5.4/5.5）／`perf_rank_dump.py`（6.2）／`perf_compare_selftest.py`（6.3）／`judge_perf_catchup.py`（6.1）／`fixtures/{C1_catchup_by_target_pass,C2_catchup_by_target_fail,T2_tick_lines_absent}`／`fixtures/generate.py`（改変）／`.claude/skills/perf-loop-iteration/templates/{agent-prompts.md,entry.json}`／wintf `ecs/world/world_tick_gate_tests.rs`／areka `thread_roles.rs(+_tests)`・`tick_gate_config.rs`・`tick_gate_config_tests.rs`・`tick_gate_config_producers_tests.rs`・`emo2_boot/balloon_visibility_phase_wake_tests.rs`／areka-actor `src/lib.rs`・`src/spawn.rs`・`src/spawn_hook_tests.rs`（crate ごと計画外＝2.3 のフック）／改変のみ計画外: `wintf/src/ecs/app.rs`・`runtime/mod.rs`・`ecs/graphics/systems/init.rs`・`emo2_boot/{mod.rs,move_cue.rs,talk_lifecycle.rs,hover_inject.rs,frame_text_scale_tests.rs}`。

**計画にあって未作成**: `tools/perf/wpa/cpu-sampled.wpaProfile`（C8 代替 backend・TOML の `[sampling] backend` は保持）。`loop-ledger.md`・`results/` はループ生成物。

**改名・移動**: `ecs/window_proc/dispatch.rs` → 実体 `ecs/window_proc/mod.rs`／生産者の字面検査 = `tick_gate_tests.rs`（wintf 10 本）＋`areka/src/tick_gate_config_producers_tests.rs`（areka 8 本）／`ecs/drag/` → `drag/mod.rs`＋`drag/systems.rs`／`ecs/dola/mod.rs` の `mark(ANIM)` は計画どおりだが本番未登録で不活性／C17〜C19 の候補ファイルは非接触（周回の候補）。

## (C) 設計節ごとの逸脱（権威＝tasks.md Implementation Notes）

| 設計節 | 逸脱 | 出所 |
|---|---|---|
| C1 TOML 例 | `[sampling] backend` が `GOAL_SCHEMA` の必須なのに例に無い（無いと周 0 の goal-check が exit 3） | 5.5・8.2 |
| C2 遷移表 | `IMPLEMENT.implement_blocked → RECORD` が無い | 7.5 |
| C2／Flow 1 | FINAL で全 PASS でなければ RANK へ戻る（設計は FINAL 終端）／周 2 以降も rank-run が要る | 7.5 |
| C5 終了語彙 | `function_stage` の理由に `probe_failed`・`dry_run` を追加 | 7.2・7.3 |
| C5／Error Handling | Error Handling は静寂超過→MEASURE_FAILED、C5 と実装は exit 2 `EXIT_NOT_QUIET` | 7.3 |
| C5 | 絶対値合否（measure-baseline／final）は `shiori_helper_present` でなければ judge しない（exit 4）・measure-ab は `iteration_build != release` を exit 3 | 7.3 |
| C8 -SelfTest ⒜ | perf-rank.py を通さず自前の関門＋`FIXTURE_EXPECT_*` 6 定数・`-Stop` 非昇格は exit 1 | 5.3 |
| C10 | NA 副指標は採用を止めない／差なし帯＋副指標悪化→WORSE／judge exit 1 は判定不能にしない／`compare.json` は append へ直接渡せない（6 鍵抽出） | 6.3 |
| C11 | 状態ブロック＝8 鍵＋`run`・`capabilities`・`previous_phase`・`toolfix_used` | 5.4・7.5 |
| C13 | `kind=monitor` は変化時のみ→OS 列挙／`win_kind` は `char`/`balloon`（`transition_diag.rs:305` の例 `"shell"` は陳腐化） | 6.4 |
| C14 | 登録点は ticker.rs／spawn.rs でなく `areka_actor::install_thread_start_hook`＋`thread_roles.rs`／名簿に登録解除なし | 2.3・2.1 |
| C16 | 「epoch 確立かつ未完」→`reveal_pending`（未リビールのグリフ有無）／`sinks` 順序が前提／旗は `tx.send` の後 | 3.5 |
| C20 | `frame_transition_atomicity_tests` は 3 本（設計の 4 は doc 内の字面を数えた誤り） | 4 |
| C15 | `[tick]` 行に span 前置 `actor{actor=emo-text}:`／`ui_cpu_us` は 15,625µs 量子 | 2.2 |

## (D) requirements.md 改訂欄の候補

1. 8.6＝`Cargo.toml` 非接触は成立（`git diff origin/main -- '*Cargo.toml'` 空）。肯定的に登記（ToolHelp 回避→自前名簿）。
2. 1.4 の停止理由は文面どおり（FINAL で全 PASS でないのは停止理由ではない）。設計 Flow 1／C2 側を改める。
3. pwc brief の既存行 121-122 は dlp を「W8」と書いたまま（正は W6.9）。
4. 1.12／1.13＝`[agent-model] Opus 5 (1M context)` を Fable から観測（7.4 追記）。
5. brief 由来の areka 側数値は Bevy 0.19／Taffy 0.13 更新（`bf2d7950`）前＝周 0 のベースラインが権威。`results/summary.md` にその注記が無い。
6. cage への申し送り: `lock_self_initiated_for_test` 退役（21 箇所／5 ファイル）・兄弟 4 ファイルの doc「プロセス共有」陳腐化・`tick_one_frame_with` は私有。
7. 3.5 の未強制の前提は `command.rs` module doc に登記済み（`window_pos.rs:290` は現在も正）。
