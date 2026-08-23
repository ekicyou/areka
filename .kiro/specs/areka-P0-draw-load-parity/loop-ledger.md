## 状態
- goal: draw-load-parity
- iteration: 3
- phase: FINAL
- pending_run: C:\Users\maz-o\AppData\Local\areka-diag\perf-loop\draw-load-parity\iter-3
- streak_no_gain: 3
- best_idle_cpu_pct: 15.80
- baseline_idle_cpu_pct: 15.80
- started_at: 2026-08-22T23:41:05Z
- run: 87696907
- capabilities: elevated:false;xperf:true;pdb:true;function_stage:UNAVAILABLE;reason:not_elevated;judge:0.4.0;python:3.13.15;pwsh:7.6.4;checkin_min:30;selftest:ok
- previous_phase: REMEASURE
- toolfix_used: 5
- not_quiet_retries: 4

## 周 1 — 2026-08-23T04:39:30Z
- hypothesis: tick gate default ON: try_tick_world が 13 スケジュールを 120 回/秒 全部回す（tick の 98% は表示に変化なし）。起床旗の無い tick を門で省けば UI スレッド（段② 48.91%）の定常 CPU が下がる。周 1 は仕組みの A/B（順位表からの選択は周 2 以降・tasks 9.2）
- candidate: stage=thread rank=1 item=ui(main tid=22252) share=48.91% (stage4 ticks_per_sec=119.50 skipped_pct=0.00 top=framefinalize 31.41%/draw 22.62%)
- files_changed: crates/wintf/src/ecs/world/mod.rs, crates/wintf/src/ecs/world/world_tick_gate_tests.rs, crates/wintf/src/ecs/world/tick_gate.rs, crates/wintf/src/runtime/tick_bridge.rs, crates/areka/src/tick_gate_config.rs, tools/perf/README.md
- runs: -
- before_idle_cpu_pct: -
- after_idle_cpu_pct: -
- delta_pct: -
- noise_pct: -
- secondary: -
- tests: green (5,757 passed; cargo test --workspace code=0)
- followup: FAIL clickthrough=PASS drag=FAIL(drag_not_followed: char/balloon Δ=+0 vs 期待 +80・kind=write 0 行) dpi=PASS balloon_follow=INCONCLUSIVE(depends_on_drag) — 対照: 同じ手順で A 側（門 OFF・bin-A）は全 PASS → 門 ON がドラッグ追随を壊す本物の欠陥
- verdict: FOLLOWUP_FAIL
- commit: -
- skipped_candidates: none (周 1 は仕組みの A/B で候補選びをしない・tasks 9.2)
- duration_min: 95
- reason: iteration1 mechanism A/B: A=gate default OFF (HEAD), B=gate default ON; stage3=UNAVAILABLE(not_elevated); agent-model-warning:perf-measure=missing-first-line(model opus passed); agent-model-warning:perf-implement=missing-first-line(model opus passed); followup_fail: gate ON breaks drag (A control PASS) → 門の起床旗にドラッグ経路の穴（rearm_tick_while_dragging/pointer 生産者）が残る。周 2 以降の候補＝穴を塞いでから門 ON を再 A/B; followup は desktop lock 中に 2 度 INCONCLUSIVE（環境）→ 対話デスクトップ復帰を待って実走; CORRECTION(周 3 で判明・2026-08-23): この周の followup は target\release＝prepare-ab が残した A 実行体（門 OFF・sha b4ed1b79…）で走っていた（道具の穴・TOOLFIX 4＝走行前に作業ツリーから作り直す）。drag FAIL は門 ON の欠陥ではなく門 OFF での不再現の失敗（ロック解除直後の 1 回）。「門 ON がドラッグ追随を壊す」は撤回＝B の追随は未検証

## 周 2 — 2026-08-23T06:31:33Z
- hypothesis: tick gate default ON（再 A/B・ドラッグ穴を塞いで）: UI スレッド 56% の中身は 13 本を毎コマ全部回す tick（skip 0%・119.58 回/秒）。周 1 の drag FAIL の原因＝起床旗の生産者が DraggingState 成分に依存し、権威状態のスレッド局所 DragState（Preparing/JustStarted/JustEnded）を代表していない。旗を状態機械側へ寄せてから門 ON を再 A/B する
- candidate: stage=thread rank=1 item=ui(main tid=34772) share=56.0% catalog=C16
- files_changed: crates/wintf/src/ecs/drag/systems.rs, crates/wintf/src/ecs/drag/systems_tests.rs(new), crates/wintf/src/ecs/drag/state/mod.rs, crates/wintf/src/ecs/drag/state/tests.rs, crates/wintf/src/ecs/world/mod.rs, crates/wintf/src/ecs/world/tick_gate.rs, crates/wintf/src/ecs/world/tick_gate_tests.rs, crates/wintf/src/ecs/world/world_tick_gate_tests.rs, crates/wintf/src/runtime/tick_bridge.rs, crates/areka/src/tick_gate_config.rs, tools/perf/README.md
- runs: A1=iter-2\A1(6.29) B1=iter-2\B1(5.96) A2=iter-2\A2(13.28) B2=iter-2\B2(6.28)
- before_idle_cpu_pct: 9.79
- after_idle_cpu_pct: 6.12
- delta_pct: -3.67
- noise_pct: 6.99
- secondary: p95_ms=162.160/160.425, catchup=16.500/19, allocs=0/0
- tests: green (5,767 passed; cargo test --workspace code=0)
- followup: PASS clickthrough=PASS drag=PASS dpi=PASS balloon_follow=PASS（門 ON＋穴塞ぎ・drag Δ=+80 一致・write 3 行）
- verdict: WORSE
- commit: -
- skipped_candidates: thread#2 unregistered_rest 40.3% no_signal(帰属不明・段③待ち); thread#3 ticker_loop 1.7% no_signal; thread#4 cursor_monitor 1.2% no_signal; thread#5-10 no_signal; phase#1 framefinalize 34.8% no_signal(門の結論まで分離不能・C17/C18); phase#2 draw 22.6% no_signal; phase#3-6 no_signal; function SetWindowPos系 no_signal(段③ UNAVAILABLE); function compose/blit out_of_scope
- duration_min: 170
- reason: iteration2: B=drag 起床旗を DragState 起点へ＋門の既定 ON, A=HEAD(門 OFF); stage3=UNAVAILABLE(not_elevated); handoff:areka-P0-emo2-conformance-e2e:crates/wintf/src/ecs/drag/systems.rs; handoff:areka-P0-present-write-coherence:crates/wintf/src/ecs/drag/systems.rs; not_quiet: measure-ab 1 回目は A1/B1 の後で静寂確認 NOT_QUIET(exit 2)→ -Resume で 1 回やり直し; not_quiet(2): -Resume 後も B2 の走行後確認が NOT_QUIET(machine 10.8% vs 閾値 10.0・この機械の遊休 8〜9% に対し閾値が際どい)→ 4 走行とも走行前は QUIET・areka exit 0 なので -Resume で compare へ進めた（走行後の NOT_QUIET は B1=14.6%/B2=10.8%）; compare: 副指標が悪化した（catchup）（catchup A=15/18 B=19/19＝A 自身の散らばり内だが count 規則で悪化）; 主指標 delta -3.67 は noise 6.99(|A1-A2|・A2=13.28 が外れ値) に埋もれ差なし帯; 門 ON の省略率は measure-ab では点灯しないため未観測; SIZE small RISK low; CORRECTION(周 3 で判明): この周の followup PASS も A 実行体（sha e3d256c7…）で走っており、B（門 ON＋穴塞ぎ）の追随は未検証。compare は bin-A 対 bin-B で有効。周 3 の rank-run 1 回目は target\release に残っていた B 実行体（sha 3cadb820…）を測っていた＝点灯つき 7 分で定常 3.30%（p50 3.11・skipped 87.6%・heartbeat 20.8%）に対し周 2 の rank（A・点灯）は 17.04%＝門 ON の点灯走行は目標 3.0% 近傍（参考観測・合否外）

## 周 3 — 2026-08-23T08:36:33Z
- hypothesis: C17 単スレッド実行器: 名簿外 51.8%（5.859 cpu_s/60s）は tick 駆動の ComputeTaskPool ワーカー（門 ON の残置実行体では tick 省略 87.6% に比例して 0.578 cpu_s へ落ちる＝tick 由来）。既定の多スレッド実行器のままの 7 本（Input/Update/PreLayout/Layout/PostLayout/Draw/FrameFinalize）を SingleThreadedExecutor へ寄せ、120 回/秒×7 回のワーカー起床・待機（1 tick 当たりワーカー側 815µs 対 UI 自身 695µs）を消す
- candidate: stage=thread rank=1 item=unregistered_rest share=51.8% catalog=C17
- files_changed: crates/wintf/src/ecs/world/mod.rs, crates/wintf/src/ecs/world/world_executor_tests.rs(new), crates/wintf/src/ecs/layout/systems/monitor_systems_transition_tests.rs, crates/wintf/src/ecs/window/transition_diag_tests.rs, crates/wintf/src/ecs/dola/mod.rs, crates/wintf/tests/ecs/world_lifecycle_test.rs
- runs: A1=iter-3\A1(27.5・走行後 NOT_QUIET 21.4%) B1=iter-3\B1(7.31・走行後 NOT_QUIET 20.5%) A2=iter-3\A2(14.7) B2=iter-3\B2(10.51)
- before_idle_cpu_pct: 21.10
- after_idle_cpu_pct: 8.91
- delta_pct: -12.19
- noise_pct: 12.80
- secondary: p95_ms=165.796/165.985, catchup=123/122, allocs=0/3
- tests: green (5,759 passed; cargo test --workspace code=0)
- followup: PASS clickthrough=PASS drag=PASS dpi=PASS balloon_follow=PASS（B 実行体 sha 82b11fd4… で実走・TOOLFIX 4 後）
- verdict: WORSE
- commit: -
- skipped_candidates: thread#2 ui 44.2% already_tried(周 2 C16 WORSE); thread#3 ticker_loop 1.93% no_signal; thread#4 cursor_monitor 1.10% no_signal; thread#5-10 no_signal; phase#1 framefinalize 34.0% lower_rank(C18 次点); phase#2 draw 22.28% lower_rank(C18); phase#3-10 no_signal; function SetWindowPos系 no_signal(段③ UNAVAILABLE); function compose/blit out_of_scope
- duration_min: 150
- reason: iteration3: B=13 本すべて single_threaded(label) 構築（挿入順不変・Cargo.toml 非接触・門の既定 OFF のまま）, A=HEAD; stage3=UNAVAILABLE(not_elevated); handoff:areka-P0-test-cage-determinism:crates/wintf/src/ecs/window/transition_diag_tests.rs; handoff:areka-P0-emo2-conformance-e2e:crates/wintf/src/ecs/world/mod.rs; handoff:areka-P0-present-write-coherence:crates/wintf/src/ecs/world/mod.rs; rank-run 1 回目は残置 B 実行体を測った（rank-STALE-Bbinary へ退避・TOOLFIX 4）; impl: ComputeTaskPool::get_or_init を EcsWorld::new() に明示（多スレッド実行器の副作用で立っていたプールが無くなり par_iter が初回 tick で panic する穴）→ par_iter のワーカー起床は今周では残る; not_quiet: measure-ab 1 回目は A1 の後の静寂確認で NOT_QUIET(exit 2)→ -Resume で 1 回やり直し; not_quiet(2): B1 の後も machine 20.5%（利用者の作業で機械が忙しい時間帯）→ 走行前の静寂待ちつきで -Resume を続ける（開発者指示「止めない」）; compare: 副指標が悪化した（allocs）（allocs A=0/0 B=1/5・catchup A=222/24 B=48/196＝B2 は静かな条件で catch-up 8 倍・単スレッド化で tick 壁時計が伸び進行境界を跨ぐ筋と整合）; 主指標 delta -12.19 は noise 12.8（A1=27.5 が忙しい時間帯）に埋没; A1/B1 は走行後 NOT_QUIET＝疑わしい走行（TOOLFIX 5 で以後は再利用しない）だが B2（静か）の allocs 5 だけで count 規則の WORSE は動かない
