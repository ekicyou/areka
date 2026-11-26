# 移行の記録（dola-interrupt-ticks）

この記録は、時刻を `f64` の秒から 100ns の整数の型（`TimePoint`・`TimeSpan`・`TimeDelta`）へ替えた移行の、外した物・変えた物・確かめた物を残す。節の分け方は設計の「記録」に従う。

- 前半（段 0〜5・タスク 1.1〜6.7）の分を、7.1 が書いた。後半（段 7〜8）の分は、12.3 が各節の「後半」に足す。
- 数字の書き方: `ms(n)` は n ミリ秒の長さ、`at_ms(n)` は時計の 0 から n ミリ秒の時刻、`delta_ms(n)` は符号つきの n ミリ秒の差。「1 刻み」は 100ns。

## 1. 外したテストと理由

要件 9 の 4（表せない入力だけを確かめていた物）・9 の 5（ミリ秒の格子の継ぎを確かめていた物）・9 の 7（`f64` の丸めそのものを確かめていた物）に当たる物と、確かめていた事そのものが無くなった物を挙げる。置き換え先が在る物は「→」で書く。

### 1.1 前半

#### 古い時計（dola）

| テスト | 理由 | 置き換え先 |
|---|---|---|
| `runtime::clock::tests::now_is_monotonically_increasing`・`now_returns_positive_finite_value`・`now_has_millisecond_precision` | 古い QPC の時計（実数の秒）をファイルごと消した | `time/clock_tests.rs`（戻らない事・OS の値と同じ物差し） |
| `error.rs::display_loop_offset_variants` の `LoopOffsetNegativeMin`・`LoopOffsetNegativeMax` の 2 行 | variant を消した。負のずらしは読み込みの失敗になる | `validation/schema_test.rs::time_field_tests`・`runtime/loop_offset_test/loop_offset_serde.rs::negative_or_non_finite_offsets_fail_to_load` |

#### cue の層（dola）

| テスト | 理由 | 置き換え先 |
|---|---|---|
| `src/cue/schedule/tests.rs` の `tick_with_nan_time_delivers_all_pending_payloads`・`tick_with_nan_time_stops_at_barrier_then_normal_tick_recovers`・`insert_nan_offset_fires_debug_assert_in_debug_builds`（節「非有限時刻の境界」ごと） | NaN の時刻・隔たりは型で作れない（9 の 4） | — |
| `src/cue/schedule/tests.rs::rounds_down_value_really_rounds_down_when_subtracted`（定数 `ROUNDS_DOWN` ごと） | 実数の引き算で手前へ落ちる事が前提。整数の足し算に丸めが無い（9 の 7） | `payload_is_delivered_at_exact_absolute_fire_time_and_not_one_tick_before` |
| `tests/cue/sheet_test.rs::cue_sheet_new_with_nan_start_time_does_not_panic`（節「非有限 start_time の境界」ごと） | NaN の開始は型で作れない（9 の 4） | — |
| `tests/cue/sheet_test.rs::to_talk_schedule_clamps_non_finite_and_negative_duration_into_envelope_and_horizon` | 有限でない・負の長さは型で作れない（9 の 4） | `tests/cue/sheet_serde_test.rs` の `json_negative_lengths_fail_to_load` |
| `tests/cue/sheet_test.rs::end_time_equals_occupancy_horizon_for_non_finite_durations` | 同上 | `end_time_and_occupancy_horizon_are_exact`・`to_talk_schedule_completes_exactly_at_sheet_absolute_end_time` |
| `tests/cue/sheet_test.rs::off_grid_values_keep_plain_sum` | ミリ秒の格子の継ぎ（`sum_on_ms_grid`）が無くなり、確かめる中身が無い（9 の 5） | — |
| `tests/cue/sheet_sort_test.rs::sort_with_many_nan_follows_total_order`（補助の `assert_total_order`・`black_box`・`Ordering` ごと） | NaN の開始時刻は型で作れない（9 の 4） | `sort_many_cues_is_ascending_and_stable`（入力の並べ替えは単体テストの `insert_each_matches_per_entry_insert_including_equal_time_fifo`・`insert_same_offset_payloads_delivered_fifo`） |
| `tests/cue/sheet_sort_test.rs::negative_zero_keeps_script_order` | −0.0 が無い（9 の 7） | `equal_start_time_keeps_script_order_through_schedule` |
| `tests/cue/schedule_test.rs::player_emits_cue_at_sheet_absolute_fire_time` | 定数 `ROUNDS_DOWN = 2.5999999999999996` と起点 100.0 の実数の丸めの継ぎが前提（9 の 5・9 の 7） | `player_emits_cue_exactly_at_sheet_absolute_fire_time`（ミリ秒の格子の外の隔たり 25,999,999 刻みで、1 刻み手前では配られず、ちょうどで配られる） |

#### storyboard の層（dola）

| テスト | 理由 | 置き換え先 |
|---|---|---|
| `validation/schema_test.rs::invalid_numbers_are_rejected` の 4 件（`trans.duration=NaN`・`duration=-3.0`・`delay=-1.0`・`loop_offset=Scalar(NaN)`。ケース 13 → 9 件） | 検査の誤りから読み込みの失敗へ替わった（9 の 4） | `schema_test.rs::time_field_tests::negative_or_non_finite_times_fail_to_load` |
| `validation/schema_test.rs::negative_scalar_loop_offset_reports_one_error`（`LoopOffsetNegativeMax`） | 同上 | `runtime/loop_offset_test/loop_offset_serde.rs::negative_or_non_finite_offsets_fail_to_load` |
| `runtime/loop_offset_test/loop_offset_validation.rs` の `v14_negative_min_error`・`v15_negative_max_error`・`v15_negative_scalar_error`・`nan_scalar_loop_offset_is_rejected`・`nan_range_max_is_rejected_even_with_positive_min`・`nan_range_min_is_rejected` | 同上 | 同上 |
| `compile/boundary_test.rs::invalid_delay_and_duration_are_rejected_before_compile`・`nan_delay_that_bypassed_overlap_check_is_rejected`・`between_nan_delay_is_rejected` | 負・NaN・無限大の delay／duration は型で作れない（9 の 4） | `validation/schema_test.rs::time_field_tests::negative_or_non_finite_times_fail_to_load`・`general/time_range_test.rs::negative_and_non_finite_durations_fail_to_load_in_toml` |
| `compile/boundary_test.rs::nan_trigger_start_offset_is_rejected`（`compile_storyboard` の開始の時刻と引き金のずらしに NaN を渡す物） | 同上 | `trigger/serde_test.rs::non_finite_trigger_start_offset_fails_to_load` |
| `compile/boundary_test.rs::segment_sort_with_many_nan_follows_total_order` の NaN の回 | NaN の開始時刻は型で作れない | 残りを `segment_sort_is_ascending_by_start_time` へ改名 |
| `trigger/compile_test.rs::trigger_fire_time_with_start_time_offset` | コンパイルの開始の時刻を足す事が無くなった | 頭からの発火時刻は `trigger_after_keyframe_fire_time`・`trigger_at_start_fire_time` |
| 確かめの行: `compile/sequential.rs::sequential_first_entry_uses_compile_start_time` の `result.start_time == 3.0`・`compile/compilation_flow.rs::complex_multi_variable_mixed_placement` の `result.start_time` | 欄 `CompiledStoryboard.start_time` が無くなった | テストは `sequential_first_entry_starts_from_head` へ改名して残した |
| `src/runtime/instance_manager/tests.rs::nan_finish_deadline_never_fires` | NaN の期限は型で作れない（9 の 4） | — |
| `src/runtime/instance_manager/tests.rs::infinite_finish_deadline_fires_only_at_infinity` | 無限大の期限は型で作れない | `max_finish_deadline_fires_only_at_max` |
| `src/runtime/loop_controller/tests.rs::process_loops_huge_end_time_concludes` | 1e17 秒の足し算が値を変えない（丸め消え）事が前提（9 の 7） | `process_loops_lap_beyond_time_range_concludes` |
| `tests/trigger/event_order_test.rs::loop_end_trigger_survives_pause_rounding` | 守っていた 1 ulp の継ぎが消えた（9 の 7） | `loop_end_trigger_fires_after_pause_shift` |
| `runtime/loop_offset_test/loop_offset_compile.rs::e2e_f64_precision_many_loops` | 実数の精度の積もりが前提（9 の 7） | `e2e_many_loops_with_wait_stay_exact` |
| `runtime/loop_integration_test.rs::end_time_unified_for_single_and_infinite` | 「1 回と無限で終わりの予定が同じ」の前提が、終わりの予定の時刻の意味の変化で成り立たない | `planned_end_time_for_each_loop_shape` |

#### 使い手の crate

| テスト | 理由 | 置き換え先 |
|---|---|---|
| `areka-sakura` の `drive_delivery_tests.rs::non_finite_tick_is_ignored_and_playback_survives` | 有限でない刻みを捨てる枝の檻。`Tick(TimePoint)` で作れず、枝も取り除いた（9 の 4） | — |
| `areka-sakura` の `compile_sheet_tests.rs::compiled_start_times_are_finite_non_negative_non_decreasing` の有限・非負の主張 | 型が保証する | `compiled_start_times_are_non_decreasing`（非減少と同じ時刻の確かめは残した） |
| `areka-sakura` の `compile_choice_timeout_tests.rs` の補助 `parse` の `is_finite` | 同上 | — |
| `areka-kanade` の `schedule/choice.rs::choice_deadline_nan_is_indefinite` | NaN は型で表せない（9 の 4） | — |
| `areka-kanade` の `schedule/choice.rs::choice_deadline_rounds_fractional_seconds_half_away_from_zero` | 秒から型への丸めは、指令の縁（areka-sakura の `TimeDelta::from_millis_saturating`）へ移った（9 の 7） | — |
| `areka-kanade` の `schedule/choice.rs::choice_deadline_zero_is_indefinite` の中の −0.0 の件 | −0.0 が無い | 0 の件は残した |
| `areka-runtime` の `talk_lifecycle_tests.rs::non_finite_cue_times_are_skipped_with_a_record` | NaN・無限大は型で書けない（9 の 4） | — |
| `areka-runtime` の `hover_inject.rs::cycle_ordinal_defensive_clamps_and_guards` | 負の時刻・負の周期・NaN（9 の 4） | 周期 0 の確かめは `cycle_ordinal_zero_period_is_none` |
| `areka-runtime` の `balloon_visibility_tests.rs` の節「端の値」の `a_negative_display_end_is_rounded_up_to_zero_before_anchoring`・`a_non_number_display_end_is_rounded_up_to_zero_before_anchoring`・`a_non_number_current_time_never_expires_the_measurement` | どれも型で書けない入力（9 の 4） | 時刻の分からないフレームで満了させない事は `balloon_visibility_timeout_suppression_cases.rs` の場合 ⑾ と `no_measurement_starts_in_frames_without_time` |
| `areka-runtime` の `balloon_visibility_phase_wake_tests.rs::non_finite_deadline_arms_nothing` | 無限大・NaN の期限と今は型で書けない（9 の 4） | 期限か今が無い場合は `no_measurement_arms_nothing` |
| `areka-emo-text` の `actor_lookahead_tests.rs` の `Rig::play` のミリ秒の格子への丸め（テストの組み立ての中の処理） | 実数の誤差を吸うためだけの丸め。判定の時刻はちょうどの `[start_time, start_time + duration]` になった | — |

### 1.2 後半

（12.3 が書く）

## 2. 取り除いた本番の分かれ道とログ

要件 7 の 9 に当たる物。取り除いたログは、そこでだけ出していた物を名指す。ログの欄の名前と字面の変化は 3 の「ログの欄」に書く。

### 2.1 前半

| crate・所 | 取り除いた分かれ道 | 一緒に取り除いたログ |
|---|---|---|
| dola `runtime/clock.rs` | 古い時計（QPC の周波数の割り算と、`Result` を捨てていた所）。ファイルごと消し、`Cargo.toml` から `Win32_System_Performance` を外した | 無し |
| dola `compile` | 欄 `CompiledStoryboard.start_time` と、`compile_storyboard` の開始の時刻の引数 | 無し |
| dola `cue/sheet.rs`・`cue/schedule.rs` | `sum_on_ms_grid`（ミリ秒の格子の継ぎ）とその再公開・3 か所の呼び出し（到達・完了・`occupancy_horizon`）／`clamp_duration`（有限でない・負の長さを 0 にする cue の長さの clamp）／`CueSheet::new` の −0.0・NaN の全順序の比べ（安定な並べ替えへ）／`insert`・`insert_each` の「隔たりは 0 以上」の `debug_assert`（型が保証する）／−0.0 を +0.0 にそろえる `total_cmp` の並べ替え（安定な `sort_by_key(Reverse(offset))` へ。同じ瞬間の FIFO は変わらない）／NaN を「届いた」に倒す `>` の否定の比べと `#[allow(clippy::neg_cmp_op_on_partial_ord)]` | 無し |
| dola `validate/rules.rs` | `validate_transition_numbers` の delay・duration の「有限で 0 以上」（`InvalidNumber`）／`validate_numbers` の loop_offset・loop_offset.min／max・offset・trigger_start_offset の「有限」（`InvalidNumber`）／`validate_loop_offset` の `LoopOffsetNegativeMin`／`Max`（storyboard の文書の検査の決まり） | 無し |
| dola `compile` | 時刻の欄（fire_time・keyframe_time・end_time）を無限大で見る `push_if_overflowed`（relative_to の値の分は残した）／`total_base_duration / time_scale` が無限大なら `InvalidNumber{loop_duration}`／−0.0・NaN をそろえる `total_cmp` の並べ替え（区間と引き金の 2 か所） | 無し |
| dola `runtime` | `next_event_time` の NaN を捨てる `filter`／`check_finish_deadlines` の NaN の期限の扱い／`trigger_wall_time` の「周回の終わりで抑える」`.min(end_time)` の継ぎ／`process_loops` の「丸めで時刻が進まない」を見る `before + (loop_duration + max_delay) <= before`（checked の和で「延ばせない」を見る形へ）／`t.next_down()`（100ns 手前へ）／周回の後の待ちの `f64::INFINITY` の番兵（列挙へ）／積んでいた欄 `end_time`（式で求める） | 無し |
| wintf タイプライター | `skip` の `f64::INFINITY`（行程の終わりまで進める形へ） | 無し |
| areka-sakura `drive.rs`・`compile.rs` | `on_tick` の有限でない刻みの守り（有限でない刻み）／`compile.rs` の `Instruction::Wait` の 1 ms 未満の切り捨てと、`ms_to_secs`・`text_playback_ms` | `error!` "non-finite Tick ignored; schedule not advanced" |
| areka-kanade `schedule/choice.rs` | `choice_deadline` の `v.is_nan()` の腕と `(v * 1000.0).round() as u64` の飽和の変換／型 `MonotonicMs`（再公開ごと） | 無し |
| areka-seriko | `TalkFeed::push_chunk` の実数の式（`r = max(prev + duration/count, at)`）と `count == 0` の早い return（1 字ごとの間隔の関数へ）／`TalkEpoch::wall_ms` の「最も近い 1 ms へ丸める」／`TalkEpoch` の実数の起点 | 無し |
| areka-emo-text `state.rs` | 実数の `interval` と `max(prev + interval, chunk_start)`（`chunk_reveal_offsets` へ） | Text と Choice の `debug!` の欄 `interval`（実数の間隔そのものが無くなった） |
| areka-ghost `ticker.rs` | 関数 `real_clock`（`GetTickCount64` の読み取りとその `// SAFETY:`）／`duration_to_millis` のミリ秒への切り捨て | 無し（catch-up と disconnected の info の文言は同じ） |
| areka-ghost dispatcher | 選択待ちの表示の終わりをミリ秒へ四捨五入する所（`(secs*1000).round() as u64`）。`base_now` が無い時の `warn!` は前半では残した（後半の 9.1 で消す） | 無し |
| areka-runtime `talk_lifecycle.rs` | 有限でない占有の終わりを捨てる分かれ道（有限でない終わり。horizon の `NEG_INFINITY` の番兵は `Option<TimeSpan>` へ） | その分かれ道の `warn!` |
| areka-runtime `hover_inject.rs` | 負の frame_time を 0 に寄せる所／有限でない・負の周期を止める守り（NaN の周期） | 無し |
| areka-runtime `lib.rs` | `GetTickCount64` を読む `tick_count_ms`（`unsafe` の塊ごと）と `MonotonicMs` で包む所 | 無し |
| areka-runtime バルーンの寿命 | `balloon_visibility_wait.rs` の「非数の現在時刻」の扱い・組み立ての `ended_at` の `.max(0.0)`／`balloon_visibility_decision.rs` の `DisplayEndAt` の `end.max(0.0)`（負の表示の終わりを 0 に寄せる所）／`balloon_visibility_phase.rs` の `visibility_wake` の `!remaining.is_finite()` の守り | 無し |

- 足した分かれ道（記録のために挙げる）: areka-sakura で `CuePlayer::from_sheet` が `Err` の時に `error!` を出して、その台詞を中断として終える（起点を押した直後なので届かない）。dola の検査で、文書の時間の欄の全部（名前つきとインラインの transition の delay・duration、loop_offset の Scalar と Range の min・max、`KeyframeRef::WithOffset.offset`、`trigger_start_offset`）を文書の値の上限で見て `TimeOutOfDocumentRange` にする。wintf のタイプライターで、作者の秒が長さにできない時に `warn!` を出して長さ 0 にする。

### 2.2 後半

（12.3 が書く。設計の「記録」の 2 が挙げる `base_now`・時計が無い時の合図と窓の知らせ（seriko）・`drive.rs` の同じ時刻の守り・`settle_completion_after_resolve`・`TalkClock::observe_cue`・`ClockedTextSink`・`TalkEpoch`・`resolve_talk_time`・`TalkEnd::TimeUnknown` と、その `warn!`・`SerikoClock`・`spawn_seriko_clocked` ほか）

## 3. 変える前の期待と、変えた後の期待

要件 9 の 6 に当たる。前半の全体テストが「変わらない」と示す範囲（ミリ秒の格子に載る時刻・表せる値）の外に在る変化を、テストの名前で名指しする。

### 3.1 前半: 設計の一覧（16 件）の数え直し

設計の「前半に入る、振る舞いの変化」の 16 件を、前半のタスクの報告と照らして数え直した。16 件の全部が、作業の中に在った。

1. **時計の出どころ**（4.1・4.9・5.1）: 刻み役（`areka-ghost` の ticker）と seriko の時計を、`GetTickCount64` から `dola::now`（`QueryUnbiasedInterruptTimePrecise`）へ替えた。ミリ秒の値を 10,000 倍して型に載せる仮の形は置いていない。決定論のテストは偽の時計なので結果は同じ。本番で変わる事:
   - 眠り: 前は眠りの間も数え、復帰で大きく跳んで `ticker catch-up` の info を出し、過ぎた選択待ちの期限は復帰してすぐ `ChoiceTimeoutDue` を送った。後は眠りを数えず、跳ばず、期限は眠りの分だけ後ろへずれる。
   - `OnSecondChange`: Reference0 の式が「ミリ秒 ÷ 3,600,000」から「100ns の数 ÷ 36,000,000,000」になり、渡る値が眠りを含まない時刻になる（眠った機械では時の数が前より小さい）。
   - 分解能: 前は OS の刻み（約 15.6 ms）ごと。後は 100ns の単位で、残りの時間も 100ns ちょうどの `Duration`。
   - 境の格子の 0 の位置: 前は起動からの時刻、後は眠りを含まない割り込みの時刻の 0（刻み役どうしは同じ格子）。
   - seriko のミリ秒への丸めが無くなる。
   - テスト: `areka-ghost` の `ticker_tests.rs` は `at_ms`・`ms` へ写した（`next_deadline_ms` → `next_deadline`）。`wake_remaining_is_zero_at_and_after_the_wake_and_max_without_one` に「1 刻み先なら 100ns」を足した。`poll_skips_multiple_missed_boundaries_and_fires_only_once`・`spawn_loop_ticker_catch_up_delivers_the_current_clock` は期待が同じで、説明の「サスペンド復帰」を「処理の大幅な遅れ」に直した。`areka-seriko` の `talk_tests.rs::wall_time_rounds_to_the_nearest_millisecond` → `wall_time_keeps_sub_millisecond_precision`（起点 1000 ms − 0.25 ms を `at_ms(1000)` で観測。前は `wall(0)`・`wall(0.5 ms)`・`wall(1 ms)` が 1000・1000・1001 ms、後は 999.75・1000.25・1000.75 ms ちょうど）。`areka-runtime` の seriko_clock_tests（ミリ秒 → `TimePoint::from_ticks`）。
2. **1 字ごとの間隔の式**（3.6・4.2・4.5）: 実数の間隔を足し込む式（emo-text の `extend_chunk`・seriko の `push_chunk`）を、字ごとに 1 回だけ丸める整数の式 `areka_sakura::reveal::chunk_reveal_offsets` へ替えた。割り切れる長さ（字数 × 50 ms）では値は同じ（`production_length_matches_current_values`）。割り切れない長さでは、どの字も今の実数の式の値から 100ns 未満だけ動く（`chained_chunks_keep_error_below_one_tick_and_never_go_back` ほか、3.6 の 12 本）。emo-text の `state_reveal_tests::visible_glyphs_progress_with_duration_derived_interval` は ±0.01 秒の余白の近似 → 境目のちょうどの値（`ms(1049)`→1・`ms(1050)`→2・`ms(1099)`→2・`ms(1100)`→3、`reveal_times_of == [1000,1050,1100] ms`）。
3. **ミリ秒の格子に載らない時刻の足し算**（2.2・2.3）: 到達の判定が「`sum_on_ms_grid(起点, 隔たり) > 今` の否定」から「`起点 + 隔たり <= 今` の整数の比べ」になった。テスト: `tests/cue/schedule_test.rs::player_emits_cue_exactly_at_sheet_absolute_fire_time`（格子の外の隔たり）。ミリ秒の格子の 3 本 `end_time_of_ms_grid_cues_is_exact`・`occupancy_horizon_matches_ms_grid_end`・`completes_exactly_at_ms_grid_end` → `end_time_and_occupancy_horizon_are_exact`・`completes_exactly_at_end`。`src/cue/schedule/tests.rs::payload_is_delivered_at_exact_absolute_fire_time` → `..._and_not_one_tick_before`（1 刻み手前で届かない事を足した）。`completion_is_reached_at_exact_occupancy_horizon` に 1 刻み手前で未完了を足した。
4. **台本の起点の `Option` と、`from_sheet` の `Err`**（2.2・2.3・2.8・3.5）: `absolute_start_time()` は押されていない時に 0.0 → `None`。`absolute_fire_time()`／`absolute_end_time()` は実数 → `Option<TimePoint>`。`to_talk_schedule`・`CuePlayer::from_sheet` は起点が無いと、前は起点 0 として動き、後は `Err(CueError::OriginNotStamped)`。テスト: `tests/cue/sheet_test.rs` の `cue_sheet_absolute_start_time_defaults_to_zero` → `unstamped_sheet_has_no_absolute_times`（0.0 → `None`）・`from_sheet_rejects_sheet_without_stamped_origin`（足した）・`carrier_cue_builds_canonical_carrier_at_zero`（`at == 0.0`・`duration == 0.0` → `fire_time == TimePoint::ZERO`・`at`・`duration` が `TimeSpan::ZERO`）。起点を押していない台本の終わりは `absolute_end_time()` → `relative_end()`（areka-sakura の `compile_*` のテスト・areka-runtime の台本の通しの 2 本）。areka-sakura では `from_sheet` の `Err` を `error!` と中断で受ける（2 の「足した分かれ道」）。
5. **表せない値を直していた所が、失敗になる**（2.2・2.6・3.5・3.9）: cue の長さの clamp が消え、有限でない・負の長さは型で作れず、直列化した台本の読み込みでは失敗になる（`tests/cue/sheet_serde_test.rs` の `json_negative_lengths_fail_to_load` ほか）。選択待ちの時間切れの指令は `ChoiceTimeoutDirective::Secs(f64)` → `Delta(TimeDelta)` で、NaN と無限大は作れない。areka-sakura の `compile_choice_timeout_tests.rs` の期待は `ChoiceTimeoutDirective::Secs(..)` → `Delta(delta_ms(..))`、桁あふれの指令は `Delta(TimeDelta::MAX/MIN)`。areka-kanade の `choice_deadline_saturates_on_extreme_values`（`INFINITY`／`1e300` → `TimeDelta::MAX`、期待 `u64::MAX` → `TimePoint::MAX`）・`choice_deadline_negative_is_indefinite`（`NEG_INFINITY` → `TimeDelta::MIN`、−0.001 → 1 刻みの負）・`choice_deadline_tiny_positive_is_immediate_not_indefinite` → `choice_deadline_smallest_positive_is_not_indefinite`（1e-9 秒・期待 1,000 ms → 100ns・期待 `at_ms(1000)` + 1 刻み）。
6. **storyboard の文書の、失敗の出る所**（2.9・2.10・2.11）: 有限でない長さ・負の長さ・負の周回のずらしは、検査（`InvalidNumber`・`LoopOffsetNegativeMin`／`Max`）の誤りから、読み込みの失敗へ替わった。テスト: 1.1 の storyboard の層の表の行き先と、`all_violations_are_collected`（`["time_scale","loop_count","duration"]` → `["time_scale","loop_count"]`）・`toml_non_finite_numbers_are_rejected_by_validate`（時間の欄を外して 8 件 → 5 件、行き先 `toml_non_finite_times_fail_to_load`）。untagged の `LoopOffset` は、負や上限を越える値の読み込みの失敗の文が「data did not match any variant of untagged enum」になる（テストは失敗した事を見る）。
7. **storyboard の文書の値の上限**（1.4・2.9・2.10）: 絶対値が 2^51（約 7 年）以上の時間の値は、読む・検査・書くのどれでも拒む（`TimeOutOfDocumentRange`）。足したテスト: `schema_test.rs::time_field_tests`（上限の端）・`loop_offset_validation.rs::document_range_tests`・`trigger/validation_test.rs::trigger_start_offset_beyond_document_range_is_rejected`・`general/time_range_test.rs`（往復・上限の端・`compile_storyboard` と `DolaDocumentBuilder::build` が拒む・toml）。
8. **storyboard の文書の秒の丸め**（1.1・1.4・2.10）: 文書の秒は読む縁で最も近い 100ns へ丸める（`secs × 1e7` の積を round、中間は 0 から遠い側）。区間の境と引き金の時刻は 100ns の格子に載る。テスト: `v16_range_inverted_error`（`*min == 5.0` → `TimeSpan::from_millis(5000)`）・`builder.rs::storyboard_builder_loop_offset_set`（`Scalar(1.5)` → `Scalar(TimeSpan::from_millis(1500))`）ほか、秒 → ちょうどの値。
9. **倍率が 1 でない再生の、出来事の時刻**（2.12・2.13）: 区間の境・引き金・周回の終わりは、切り捨ての写し F とその逆 G で決まる。引き金の予定の時刻を周回の終わりで抑える継ぎ（`.min(end_time)`）は消えた。テスト: `timeline_manager/tests::effective_time_with_time_scale`（F の切り捨ての値。値は変わらず 50.0）・`infinite_loop_lower_bound_uses_lap_length_through_time_scale`（足した）。
10. **終わらないループの下限の境**（2.12・2.13）: 1 周の長さと下限の比べが整数でちょうどになった。テスト: `loop_integration_test::short_duration_infinite_loop_error`（整数の比べ）・`core_types_test::too_short_duration_with_infinite_loop_display`（0.05 → 500,000 刻み、文は "0.0500000"・"min: 0.1000000"）・`infinite_loop_lower_bound_uses_lap_length_through_time_scale`（倍率 0.5 で台本 500,000 刻みは始まり、499,999 刻みは `TooShortDurationWithInfiniteLoop`）。
11. **終わりの予定の時刻の意味**（2.12・2.13）: 開始と `resume` の戻り値が、今の周回の終わり（`f64`）から、決まっていれば再生の全体の終わり（`Some(TimePoint)`）、決まっていなければ `None` になった。テスト（前 → 後）: `loop_count_1_compatible_with_existing_behavior`（1.0 → `Some(1000ms)`）・`finite_loop_3_cycles`（1.0 → `Some(3000ms)`）・`infinite_loop_continues_until_cancel`（1.0 → `None`）・`loop_offset_compile::e2e_no_offset_natural_end`（1.0 → `Some(2000ms)`）・`e2e_pause_during_delay_preserves_timing`（無限で `resume` 9.0 → 3 周・決まった待ち 5 秒で `end_time` `Some(13s)`、3.0 秒で止め 5.0 秒で戻すと `Some(15s)`）・`facade_test/update_flow::pause_freezes_value_and_resume_continues`（1.2 → `Some(1200ms)`）・`facade_test/load_start` の各 `end_time`・`trigger/runtime_test` の `trigger_with_start_offset`（2.5 → `Some(2500ms)`）・`skipped_loops_fire_their_triggers`（`[0.7,1.7,2.7]` → `Some(..)`）・`instance_manager/tests::pause_and_resume`（`Some(4000ms)`）・`resume_with_time_before_pause_start_shrinks_end_time`（`pause_accumulated == -2000ms`、戻り `Some(0)`）・`planned_end_time_for_each_loop_shape`（足した: 開始 500ms・1 周 1 秒で 1 回 `Some(1500ms)`・待ち無し 3 周 `Some(3500ms)`・決まった待ち 250ms の 3 周 `Some(4000ms)`・引く待ち `None`・終わらない `None`）・`e2e_pause_with_infinite_loop_has_no_planned_end`（足した）・`drawn_wait_finite_loop_resume_returns_end_only_in_last_lap`（足した）。
12. **出来事の直前の時刻**（2.12・2.13）: 「出来事の時刻より前に終わる周回まで進める」は、`next_down`（f64 の 1 つ手前）から 100ns 手前になった。テスト: `event_order_test::loop_end_trigger_survives_pause_rounding` → `loop_end_trigger_fires_after_pause_shift`。`event_order_test` の 60fps のコマは `(k·10,000,000 + 30)/60` 刻み。
13. **wintf の作者の秒**（3.2）: タイプライターの待ちと 1 字の待ちの既定が有限でない・負・範囲の外の時、長さ 0 として `warn!` を出す。前は負が行程の時刻を戻し、NaN は後ろの全部を壊し、無限大は後ろが来なかった。足したテスト: `unrepresentable_waits_become_zero_length`・`unrepresentable_default_char_wait_becomes_zero_length`。`timeline_puts_glyph_after_wait_when_cluster_spans_two_chars`（`abs() < 1e-9` → `assert_eq!(show_at, ms(1400))`）。
14. **`Instruction::Wait` の 1 ms 未満**（3.5・3.7）: 切り捨てる継ぎが消え、100ns の格子でちょうど残る（今の parser は整数のミリ秒しか作らないので本番の値は変わらない）。テスト: `start_times_sit_on_the_millisecond_grid` → `start_times_accumulate_exactly`・`wait_duration_is_on_the_ms_grid` → `wait_duration_is_exact`・`integer_ms_round_trips_through_secs` → `integer_ms_is_exact`・`trailing_explicit_wait_remains_in_sheet_and_extends_extent`（実数の和 0.9500000000000001 → `ms(950)`）。`duration.rs` の `char_count_times_nominal_ms_in_seconds` → `char_count_times_nominal_ms`（実数の秒 → `ms(250)`）。
15. **ログの欄**（3.9・4.1・4.2・4.5・4.10・4.11・5.1〜5.6）: 時刻と長さの欄が、秒の小数 7 桁の字面（Display）と、単位の語の無い名前になった。名前の変化: kanade の `deadline_ms`→`deadline`・`overshoot_ms`→`overshoot`・`display_end_ms`→`display_end`・`timeout_directive_secs`→`timeout_directive`・`now_ms`→`now`／seriko の `send_tick` の `now_ms`→`now`・settle の `wait_ms`→`wait`・trigger の `log_fired` の `started_at_ms`→`started_at`・呼び出しの始め・同じ時刻の 2 度目の開始・引き金を構えた debug の `at_ms`→`at`・区切りの文字の warn と非単調な刻みの debug の `now_ms`→`now`・`last_seen` は Display／ghost dispatcher の `choice_waiting_stale` の `display_end_elapsed_secs`→`display_end`／runtime の `timeout_secs`→`timeout`・`default_secs`→`default`・`period_ms`→`period`、`stopped_at`・`display_end`・`deadline`・`talk_time` は Display。足した欄: seriko・emo-text・ghost の `LogSink::emit` と受け口の配送の失敗に `fire_time`。消した欄: emo-text の `interval`。テスト（前 → 後）: `schedule_log_firing_tests::info_choice_waiting_established_logs_candidate_count_and_deadline`（`"Some(32000)"` → `Some(32.0000000)`）・`settle_tests::wait_over_the_cap_stops_at_the_cap`（`"wait_ms=600001"` → `wait=` を `FromStr` で読んで `ms(600_001)`）・`looper_trigger_tests.rs::glyph_missing_from_feed_falls_back_to_tick_time_with_one_warn`（`now_ms=1234` → `now=` の文字の形）・`parts_calls_tests::part_refresh_judges_periodic_before_applying_calls`（`started_at_ms=1000` → `started_at=` の文字の形）・areka-ghost `sink.rs` の `emit_logs_structured_event_with_at_actor_and_command_kind`・`emit_logs_structured_event_for_newline_cue`（`Some(1.5)`・`Some(3.25)` → `FromStr` で読んで `TimeSpan::from_millis(1500)`・`(3250)`）・areka-runtime の採った値の記録（"30.0" → "30.0000000"）と記録のテスト（"4.5"・"10.0" → `FromStr` で読んで `ms(..)`）・`script_occupancy_end_becomes_the_measurement_anchor_in_the_decision_core`（Debug の実数 → Display の文字の形）。
16. **コンパイルの開始の時刻**（2.1・2.11）: `compile_storyboard` の開始の時刻の引数と `CompiledStoryboard.start_time` が消えた。0 でない開始の時刻でコンパイルしていた 5 か所: `segment_sort_with_many_nan_follows_total_order` の NaN の回を外して `segment_sort_is_ascending_by_start_time` へ・`at_placement.rs::pure_keyframe_without_at_as_first_entry_falls_back_to_start`（開始 5 秒で 5.0〜6.0 → 0〜1.0 秒）・`sequential.rs::sequential_with_delay`（開始 10 秒で 10.5〜11.5 → 0.5〜1.5 秒）・`sequential_first_entry_uses_compile_start_time` → `sequential_first_entry_starts_from_head`（4.0〜6.0 → 1.0〜3.0 秒）・`trigger_fire_time_with_start_time_offset`（外した）。引き金の子は `start(name, TimePoint)` の起点で始め、区間と引き金を「起点＋隔たり」で求める（`tests/compile/start_shift_test.rs::values_and_triggers_shift_exactly_by_the_start_difference` を足した）。

### 3.2 前半: 一覧に無かった変化（型を替える作業で見つかった物）

1. **選択待ちの表示の終わりのミリ秒の丸め**（4.10）: areka-ghost の dispatcher が、表示の終わり（台詞の最初の刻みからの秒）をミリ秒へ四捨五入して刻みの基準へ足していた。後は丸めずに 100ns で足し戻す。テスト: `choice_waiting_converts_elapsed_secs_to_ms_from_the_tick_base_and_forwards_to_kanade` → `choice_waiting_adds_the_tick_base_back_to_display_end_and_forwards_to_kanade`（前は 4,000 + round(356.7) = 4,357 と 4,000 + round(1,250.4) = 5,250 で丸めを両側から挟んだ。後は 4,000 ms + 356.7 ms を丸めずに足す・`at_ms(1_250)` → `at_ms(5_250)`、指令 `Some(12.0)` → `Some(delta_ms(12_000))`）。`tick_relay_converts_absolute_now_to_elapsed_seconds_from_first_tick` → `tick_relay_converts_absolute_now_to_offset_from_first_tick`。
2. **項目の無いトークの、開始より前の刻み**（3.2）: wintf のタイプライターで、前は `Completed`（進行度 1.0）、後は `Playing`（進行度 0）。足したテスト: `update_before_start_reveals_nothing`・`glyph_appears_exactly_at_show_at`・`pause_before_start_keeps_remaining_lead_on_resume`・`skip_before_start_runs_whole_timeline`・`timeline_end_is_where_last_item_finishes`。
3. **seriko の「印の次の時刻」**（4.2）: `calls.rs` の `just_after` が +1 ms から +100ns になった。区切りの文字の壁時刻はミリ秒の丸めの無いちょうどの値になった（1 の seriko の丸めと同じ根）。
4. **バルーンの寿命の設定の、型の範囲を越える値**（5.1・5.2）: 前は受け入れていた大きなミリ秒の値が、長さの型の範囲を越えるとして拒まれる。テスト: `parse_accepts_positive_milliseconds` から `"18446744073709551615"` を外し、`parse_rejects_zero_negative_non_numeric_and_overflow` に足した（9 → 10 件）。`parse_balloon_timeout_reads_every_shape_of_the_value` に `"1000000000000000"` → `Default`・読めない、を足した。`hover_inject` の `parse_invalid_values_are_disabled` に `cycle:18446744073709551615` → `Disabled` を足した。
5. **時間の和のあふれの誤りの種類**（2.11）: `overflowing_sums_of_finite_numbers_are_rejected` の小さい倍率の回が `InvalidNumber{loop_duration}` → `LoopDurationOutOfRange`、長さ 1e300 → 2^51−1。時刻のあふれの回は `time_sums_beyond_the_type_range_are_rejected` へ移した。
6. **誤りの文の字面**（2.1）: `display_loop_offset_variants` の `LoopOffsetRangeInverted` が「min 3 > max 1」から「min 3.0000000 > max 1.0000000」（長さの型の文字の形）になった。足したテスト `display_time_variants`。
7. **cue の台本の直列化の形**（2.2・2.6）: 実数の秒から 100ns の整数になり、起点と時間切れは無い時に欄ごと書かない。実数の秒で書いた古い文書は読めない（`tests/cue/sheet_serde_test.rs` の「実数の秒の古い文書を拒む」）。`null` は「無い」と読む。テスト: `cue_sheet_deserializes_without_absolute_start_time_as_zero` → `json_missing_duration_reads_as_zero`・`json_unstamped_sheet_omits_origin_and_roundtrips`、`cue_sheet_serde_roundtrip`・`cue_sheet_serde_roundtrip_preserves_absolute_start_time` → `json_roundtrip_preserves_every_field`、`cue_duration_defaults_to_zero_for_legacy_serialized_data`（JSON の秒 → 100ns の整数）。
8. **例の表示の字面**（4.8・6.5）: 例 `emo-present` の「2.5 秒周期」が文字の形「2.5000000 秒周期」に、emo-text の例のログと panic の文 `{t:.1}s` が文字の形 `{t}` になった。

### 3.3 前半: 単位を写しただけのテスト（変化 0）

次のタスクのテストは、秒・ミリ秒の値を、ちょうどの 100ns の値（`ms`・`at_ms`・`delta_ms`・`TimeSpan::from_millis`）へ写しただけで、期待は同じ（要件 9 の 1・9 の 2）。比べを緩めた物・確かめる項目を減らした物は無い。

- dola: 2.4（`domain_types_serde_roundtrip`・`cue_payload_and_cue_serde_roundtrip`（時間切れ 3.0 → `delta_ms(3000)`）・`wait_carries_no_action_and_time_lives_in_envelope_duration`・`cue_duration_roundtrip_preserves_value`・`duration_is_uniform_envelope_field_across_all_payloads`）・2.5（`choice_barrier_with_negative_timeout_stops_on_arrival` の −0.001 秒 → `delta_ms(-1)` ほか）・2.6（`to_talk_schedule_carries_finite_duration_untransformed` の `to_bits` の比べ → 100ns の整数の `==`。要件 9 の 7 の型の、実数のビットの等しさの比べを、ちょうどの値の等しさへ置き換えた物）・2.14（`tests/cue/sheet_test.rs::stamping_absolute_start_time_leaves_relative_times_untouched` の 2 か所 `ms(1000)` → `ms(500)`。2.6 で型へ写す時の書き誤りの直しで、台本の相対の終わりは前の期待どおり 500ms）・2.8（`broadcast_delivers_identical_stream_to_every_sink_at_same_absolute_time` は起点 100.0 秒 → `at_ms(100_000)`、期待の合図に発火時刻 100,000／100,500／101,500 ms を足した・`is_completed_is_gated_by_occupancy_horizon_for_trailing_wait` の `absolute_end_time() - 0.01` → 終わりの 1 刻み手前、`end == at_ms(950)` を足した・`occupancy_horizon_returns_anchor_plus_relative_horizon` は 102.5 秒 → `at_ms(102_500)`、`sheet.absolute_end_time()` と `Some(..)` で比べる・`occupancy_horizon_matches_completion_threshold` に 1 刻み手前 `at_ms(11_999)` で未完了を足した。起点の無かった台本は起点 0 を押して組む）・2.10（`tests/trigger/serde_test.rs` の `trigger_start_offset` は符号つきになり、−0.5 秒の往復）・2.11（`result.time_scale` は `.get()` で比べる）。2.9〜2.13 のほかの期待は秒 → ちょうどの値。
- wintf: 3.1（`ecs/dola/mod.rs` のテスト・`tests/ecs/dola_animator_test.rs`・`tests/graphics/surface_embedding_test.rs`）・3.3（自動の上げ下げの「ちょうど前・少し越え」1e-9 秒 → 1 刻み: `active_quiet_demotes_only_after_quiet_and_hold`・`active_not_quiet_stays`・`exactly_one_second_window_promotes_just_over_does_not`、`just_under_8hz_never_promotes` の 1/7 秒 + 1e-6 秒 → 1,428,571 + 10 刻み）。
- areka-sakura: 3.5（`empty_text_has_zero_duration`（0.0 → `TimeSpan::ZERO`）・`counts_clusters_not_bytes_or_utf16_units`（`ms(150)`）・`zwj_sequence_counts_as_one_cluster`（`ms(50)`））・3.8（`Tick(x.y)` → `Tick(at_ms(..))`・`ChoiceWaiting` の欄 `display_end` の期待は「起点 + `menu_horizon()`」・`one_step_clock_jump_enters_choice_wait_for_each_directive`（指令 `Some(0.5)`・`Some(0.0)`・`Some(-0.001)`・`None` → `delta_ms(500)`・`delta_ms(0)`・`delta_ms(-1)`・`None`）・`same_fixture_and_tick_sequence_produces_identical_observation_each_run`・`compile_broadcast_stream_preserves_order_and_choices_also_land_in_bag`・`duplicate_and_backward_tick_do_not_double_fire`）。
- areka-kanade: 3.9（`choice_deadline_positive_adds_seconds` → `choice_deadline_positive_adds_directive`）・3.10（`MonotonicMs(n)` → `at_ms(n)`・`msg_tests::kanade_config_new_supplies_documented_defaults`（`close_talk_deadline_ms == 30_000` → `close_talk_deadline == ms(30_000)`、`choice_timeout_default` も同じ）・`schedule_tests::choice_waiting_deadline_follows_dd8_directive_mapping`（指令の秒 → `delta_ms`、期待 `Option<TimePoint>`）・`choice_timeout_never_fires_for_indefinite_deadline`（`u64::MAX` → `TimePoint::MAX`））・3.11（刻みの時刻 `MonotonicMs(n)`・整数 → `at_ms(n)`・`CHOICE_DEADLINE_MS ± x` → `CHOICE_DEADLINE ± ms(..)`・`choice_test_timeout_directive_tests` の指令 `Some(1.234)`・`Some(0.0)`・`Some(-0.001)` → `delta_ms(1_234)`・`delta_ms(0)`・`delta_ms(-1)`・`close_test_handshake_tests` の `close_talk_deadline_ms = u64::MAX`・`5_000` → `close_talk_deadline = TimeSpan::MAX`・`ms(5_000)`・`status_query_test` の欄 `timeout_directive_secs` → `timeout_directive`）。
- areka-seriko: 4.1（`settle_time_is_the_longest_runonce`（整数 → `ms(want)`）・table.rs の中のテストと table_always・table_film・table_trigger・table_calls・table_calls_check の期待（同じ値を `ms(..)`／`at_ms(..)` へ）。settle は軸 0 で `TalkCue::scheduled(ZERO, ZERO, ..)` を組む）・4.2（timeline の `starting_at_sets_strictly_future_boundary`・`poll_*` の 4 本・`frame_at_*`、`u64::MAX` の経過 → `TimeSpan::MAX`）・4.3（`timeline_repeat_tests.rs::lap_of_splits_elapsed_by_period`（`lap_of(u64::MAX, 1)` の `(u64::MAX, 0)` → `lap_of(TimeSpan::MAX, ms(1))` の商と余りを刻みで）・`calls_tests.rs` の `period: TimeSpan`・ほかはミリ秒の整数を `at_ms`／`ms` へ）・4.4（`balloon_hide` の `Duration::from_millis(50).as_secs_f64()` → `at_ms(50)` ほか、秒・ミリ秒の整数を `ms`／`at_ms` へ）。
- areka-emo-text: 4.5（`sink.rs` の中のテストは秒 → `ms(..)`）・4.6（`choice_anchor_tests::hit_area_stops_at_what_has_been_revealed` は覗く時刻 `shown − 0.5` 秒が shown=0 で負になり長さの型で書けないので、合図の時刻を 1000 ms へずらし `ms(1000*shown + 500)` で覗いて同じ枝を確かめた。期待の表は同じ。「全部が現れている時刻」（100.0・42.0・`f64::MAX` など）→ `TimeSpan::MAX`）・4.7（`actor_box_present_tests::unpresented_or_unrevealed_boxes_are_not_shown` は時刻 −1.0 の代わりに字を 1000 ms に置き、`ms(999)` は空・`ms(1000)` で載る。手で組む合図は `TalkCue::scheduled(..)`、`{t:.2}` の表示は文字の形 `{t}`）・4.8（秒 → `ms(..)` を 1 対 1 で写した: `pipeline_test::typewriter_reveal_drives_layout_and_window_through_the_pipeline`・`clear_mid_reveal_resets_visibility_and_window_through_the_pipeline` の期待表、`box_attach_test` の `LATE`、`viewbox_scroll_test` の `FULLY_REVEALED`、`decoration_readback` の `SETTLED` ほか）。
- areka-ghost: 4.10（`resolve_choice_relays_id_to_matching_talk_without_touching_slot_or_base` の `(0.0, 0.5)` → `(at_ms(0), at_ms(500))`・`script_choice_timeout_1234_reaches_kanade_with_display_end_and_directive` の `Some(1.234)` → `Some(delta_ms(1_234))`）・4.11（`spine_e2e_test_s5_close_deadline`（`close_talk_deadline_ms` → `close_talk_deadline`）・`spine_e2e_test_s1_boot_success`（期待 `(f64, CueCommand)` → `(TimeSpan, CueCommand)`）・`MonotonicMs(n)` → `at_ms(n)`・`Tick(0.0)` → `Tick(at_ms(0))`）。
- areka-runtime: 5.1（`horizon_matches_dola_occupancy_definition`（0.95 → `ms(950)`）・talk_clock の 4 本は許容幅つきの比べ → ちょうどの値・`talk_ended_carries_the_injected_clock_once_when_dropped`（1.234 → `ms(1_234)`）・`talk_clock_now_talk_time_reads_the_injected_clock`（2.5 → `ms(2_500)`））・5.2（`configured_timeout_secs_reads_env_once_and_is_stable` → `configured_timeout_reads_env_once_and_is_stable`）・5.3（起こしのテストは秒 → `ms(..)`、`Duration::from_secs_f64(x)` → `Duration::from_millis(..)`、上限の確かめは `ms(u32::MAX)` と `MAX_WAIT == 1 時間`・`DisplayEndAt(2/5/3 秒)` → `Some(ms(5000))`）・5.4（秒 → `ms(..)`、`X - 0.001` → `earlier(X, ms(1))`、`TalkTimeout::Millis(n)` → `After(ms(n))`。`talk_timeout_replaces_the_wait_until_the_next_talk` の期限 `2.0 + 1234.0/1000.0` → `ms(2000) + ms(1234)` ほか）・5.5（手で組む合図を `TalkCue::scheduled(TimePoint::ZERO, ..)` へ、`frame/window_state_tests.rs` の `player_for` は起点 0 を押した台本で組む）・5.6（`strict_judges_tests.rs` の 1 刻み 1e9 秒 → `TimeSpan::from_millis(u32::MAX)`。台本は数時間に収まるので 1 刻みで流れ切る。`frame.rs` の `resolve_talk_time` の再公開は `#[cfg(test)]`）。
- 本体 `areka`: 6.3（`runtime_tests/strict_judges_tests.rs` の `t < 1e7` 秒・刻み `1e6` 秒 → `at_ms(10_000_000_000)`・`ms(1_000_000_000)`、`balloon_visibility_phase_notice_tests.rs` の `END + WAIT - 0.001` 秒 → `END + 1999 ms`。手で組む合図は `TalkCue::scheduled`、`run_text_phase(.., Some(ms(..)))`）。6.4・6.5 は単位を写しただけ（6.4 の `dola::runtime::clock::now` → `dola::now`、`play_into` は起点 0 を押した台本で組む、`dump_balloon_gpu_tests.rs` の経過の比べは時刻の型、6.5 の `thread_roles_tests.rs` の `NEVER_FIRES` と `offline_dump/render_tests.rs` の `settle_time` の期待は `TimeSpan`）。
- areka-input（6.1。手で組む合図を `TalkCue::scheduled` へ、`present_frame` の秒 → `ms(..)`、テストの時計は `TimePoint::ZERO`）・areka-session（6.2。テストの時計は `TimePoint`、`dola::runtime::clock::now` → `dola::now`）・先進坑（6.6。`q7-vertical/src/old_side.rs` の `SETTLED` → `TimeSpan::from_millis(100_000)`）: 変わった期待は無い。
- 名前を替えた物（振る舞いは同じ）: wintf の自動の上げ下げの定数 `PROMOTE_WINDOW_SECS`・`DEMOTE_QUIET_SECS`・`HOLD_AFTER_PROMOTE_SECS`・`HOLD_AFTER_DEMOTE_SECS` → `PROMOTE_WINDOW`・`DEMOTE_QUIET`・`HOLD_AFTER_PROMOTE`・`HOLD_AFTER_DEMOTE`（長さの型。3.3）／areka-talk の `ChoiceWaiting.display_end_elapsed_secs: f64` → `display_end: TimePoint`・`timeout_directive_secs: Option<f64>` → `timeout_directive: Option<TimeDelta>`（3.4。areka-talk は本番の依存に dola を引く）／areka-seriko の `LoopTrigger::Always/Periodic` の周期 `NonZeroU64` → `TimeSpan`（0 は表を組む所で落とすので、受け入れる値は同じ。4.1）・`calls_tests.rs` の `period_ms: NonZeroU64` → `period: TimeSpan`（4.3）／areka-runtime の `configured_timeout_secs` → `configured_timeout`・`resolve_timeout_secs` → `resolve_timeout`・`DEFAULT_BALLOON_TIMEOUT_SECS` → `DEFAULT_BALLOON_TIMEOUT`・`TalkTimeout::Millis(u64)` → `After(TimeSpan)`・`HoverInjectConfig::Cycle { period_ms }` → `{ period }`（5.1）・`MAX_WAIT_SECS: f64` → `MAX_WAIT: Duration`（5.3）／例の `WATCHDOG_SECS` → `WATCHDOG: TimeSpan`・`CYCLE_SECS` → `CYCLE`・`last_click_time` の番兵 −10.0 → `Option<TimePoint>`（4.8）・`emo-present` の `CYCLE_INTERVAL_SECS: f64` → `CYCLE_INTERVAL: TimeSpan`・`next_switch_at: TimePoint`（6.5）。
- 足したテスト（新しい確かめ。前の期待は無い）: areka-sakura の `duration.rs::unrepresentable_millis_stick_to_the_max`（3.5）・`compile_arm_tests.rs::sysvar_after_text_starts_at_the_preceding_offset`（3.7）。

### 3.4 後半

（12.3 が書く）

## 4. 締めの検査の結果

### 4.1 残っていない物の検索（前半の分・7.1）

式が在ると分かっている字に当たる事を、移行の前のコミット `e37aee1fc`（タスクを作った直後・実装の前）の木で先に確かめてから、今の木（`73d8eb51a` の上・7.1 の直しを含む）で 0 件を確かめた。範囲は、書いた所のほかは `crates/` の下の追跡されたファイルの全部（`.rs`・`Cargo.toml`・`.md`。外のクレートの写しの `vendors/` は入れない）。

| 検索 | 式（`git grep -n`） | 範囲 | 較正（`e37aee1fc`） | 今 |
|---|---|---|---|---|
| ミリ秒の格子の継ぎ | `-F 'sum_on_ms_grid'` | `crates` | 20 行 | 0 |
| 古い dola の時計 | `-F 'runtime::clock'` | `crates` | 12 行 | 0 |
| 古い刻み役の時計 | `-F 'GetTickCount64'` | `crates` | 14 行（ticker.rs 8・runtime lib.rs 3・kanade msg.rs 1・sakura contract.rs 1・session boot_config.rs 1） | 0 |
| ミリ秒の時刻の型 | `-F 'MonotonicMs'` | `crates` | 418 行 | 0 |
| dola の中の QPC | `-F 'QueryPerformance'` | `crates/dola` | 7 行（`src/runtime/clock.rs`） | 0 |
| dola の QPC の機能 | `-F 'Win32_System_Performance'` | `crates/dola` | 1 行（`Cargo.toml`） | 0 |
| コンパイルの開始の時刻の引数 | `-E 'compile_storyboard\([^,()]*,[^,()]*,'`（呼び出しの 3 つ目の引数）と、定義 `fn compile_storyboard` の引数の目視 | `crates` | 62 行（`compile_storyboard(doc, sb_name, 0.0)` ほか）・定義に `start_time: f64` | 0・定義は `(doc, storyboard_name)` の 2 引数 |
| dola の秒の実数を返す口の名前 | `-F 'as_secs_f64'` | `crates/dola` | 一時ファイル（`fn as_secs_f64(self) -> f64` の行）に 1 件当たる（`e37aee1fc` の dola には元から無い） | 0 |

- 7.1 の時に、検索の 3 行が説明の文に残っていたので直した: `crates/areka-session/src/boot_config.rs` の `ghost_boot_options` の説明「実クロック `GetTickCount64`」→「実クロック `dola::now`」、`crates/areka-kanade/tests/kanade/close_test.rs`・`full_run_test.rs` の `//!` の「[`MonotonicMs`] Tick」→「`TimePoint` の Tick」。コードの直しは無い。
- `crates/` の外: `QueryPerformanceCounter` は先進坑の例 `crates/pilot/examples/pilot-balloon-asset-swap/`（`observe.rs`・`swap.rs`）が自分の計測に使う。dola の外で、dola の時計ではないので、検索の範囲（dola の中）の外である。`doc/` と `.kiro/steering/` の古い記述（`doc/roadmap-overview-2026-10-16.html`・`roadmap-history.md`・`roadmap.md` の過去の記録）は、文書の直し（12.2）の持ち物である。
- 設計の「残っていない物の検索」のうち、後半が取り除く物（`SerikoClock`・`ClockedTextSink`・`TalkEpoch`・`observe_cue`・`resolve_talk_time`・`now_talk_time`・`base_now`・`TimeUnknown`・`settle_completion_after_resolve`）は、前半では残る。12.3 が検索する。

### 4.2 実数の秒と型を行き来するだけの口

- 式: `git ls-files '*.rs'`（`vendors/` を除く）の全部の関数の頭を、正規表現 `\bfn\s+(\w+)\s*(<[^>{]*>)?\s*\(([^)]*)\)\s*->\s*([^{;]+?)\s*(?:where\b[^{]*)?[{;]`（複数行にまたがる頭も読む）で取り、(a) 引数に `f64` が在って戻り値に `TimePoint`・`TimeSpan`・`TimeDelta` が在る物、(b) 戻り値が `f64`（`Option`・`Result` の中を含む）で、引数（型引数を含む）に `TimePoint`・`TimeSpan`・`TimeDelta`・`Secs` が在るか、`self` を取って名前に `secs` を含む物、を挙げる。
- 較正: 一時ファイルに書いた `fn secs_to_point(secs: f64) -> TimePoint`・複数行の `fn secs_to_span(\n secs: f64,\n) -> Result<TimeSpan, E>`・`fn span_to_secs(s: TimeSpan) -> f64`・`fn as_secs_f64(self) -> f64`・`fn to_secs<T: Secs>(value: T) -> Result<f64, String>` の 5 件に、5 件とも当たった。
- 今の木で当たった 8 件と、どれも「行き来するだけの口」でない理由:

| 所 | 口 | 理由 |
|---|---|---|
| `crates/dola/src/time/convert.rs:58`・`:84` | `TimeSpan::try_from_secs_f64`・`TimeDelta::try_from_secs_f64` | 作者が書く単位から変える口（要件 3）。表せない値は `Err(TimeRangeError)` で、黙って変えない |
| `crates/dola/src/time/scale.rs:66` | `mul_unit(self, ratio: f64) -> TimeSpan` | 比を掛ける口（倍率・進み具合） |
| `crates/dola/src/time/scale.rs:80` | `ratio_of(self, whole) -> Option<f64>` | 比を返す口（進み具合） |
| `crates/dola/src/time/serde_secs.rs:49` | `to_secs<T: Secs>(value) -> Result<f64, String>`（`pub(crate)` のモジュールの中の私的な関数。対の `from_secs` も同じ） | storyboard の文書の直列化の形（要件 6）を書く・読む縁。文書の値の上限を見て、越えれば誤り |
| `crates/dola/tests/runtime/loop_offset_test/loop_offset_compile.rs:120` | `opacity_at(.., t: TimePoint) -> Option<f64>` | テストの手助け。返すのは時間で変わる値そのもの（不透明度） |
| `crates/dola/tests/trigger/event_order_test.rs:74` | `final_x(.., times: &[TimePoint]) -> f64` | 同上（位置） |
| `crates/wintf/src/ecs/widget/text/typewriter_layout.rs:249` | `author_span(secs: f64, field) -> TimeSpan` | タイプライターの作者の秒を読む縁で 1 回だけ変える所（要件 7 の 12）。変えられない値は `warn!` を出して長さ 0 |

- ミリ秒の値を 10,000 倍して型に載せる仮の形: 式 `-E 'from_ticks\([^)]*\*\s*10_?000\b'` は一時ファイルの `TimePoint::from_ticks(ms * 10_000)` に当たり、今の木で当たる 7 行は、どれも dola の結合テストの組み立て（`tests/cue/runtime_test.rs`・`sheet_serde_test.rs`・`sheet_test.rs`・`tests/runtime/support.rs`・`tests/trigger/common/mod.rs`。dola は自分の機能 `test-support` を使えないので `at_ms` と同じ物を手で持つ）で、本番は 0 件。
- 結果: 実数の秒と時刻の型を行き来するだけの口は 0 件（要件 15 の 5）。

### 4.3 継ぎを置かなかった事

前半の全部の段で、継ぎ（実数の秒から型へ黙って変わる仮の変換）を置かなかった（要件 15 の 5・設計の「継ぎを置かない」）。

- dola の口を替えた段 1 の始まりから、使い手を全部替えた段 5 の終わり（6.7）まで、ワークスペースは組めなかった。替え残しは、どれもコンパイルの誤りに出て、crate ごとに直した（段 2〜5 の締めで、crate ごとに `cargo test -p` が緑、6.7 で `cargo build --workspace` が通った）。
- 時計は、ミリ秒の `GetTickCount64` の値を 10,000 倍して型に載せる形を経ずに、`dola::now` へ直に替えた（4.1・4.9・5.1）。
- 4.2 の検索で、行き来するだけの口は 0 件。

### 4.4 公開の口の `f64`（2.14 の時点・要件 15 の 4 の前半の分）

dola の公開の口で `f64` を返す物は `TimeScale::get`（倍率）と `TimeScale::ratio_of`（比）だけで、秒の実数を返す公開の口は 0 件（2.14）。終わりの検査（全部の `pub` の欄・引数・戻り値の `f64` の一覧）は 12.3 が行う。

### 4.5 段ごとの締めの結果

| 段 | 結果 |
|---|---|
| 1（2.14） | `cargo test -p dola`・`cargo test -p dola --features toml`・`cargo check -p dola --features toml --target x86_64-unknown-linux-gnu`・clippy（機能 toml）が緑 |
| 2（3.12） | `cargo test -p wintf`（lib 1607 passed・22 ignored ほか）・`-p areka-talk`（14）・`-p areka-sakura`（161）・`-p areka-kanade`（651 + 77）が緑。`cargo build -p wintf --examples` 可 |
| 3（4.12） | `cargo test -p areka-seriko`（lib 545）・`-p areka-emo-text`（lib 1198・2 ignored）・`-p areka-ghost`（lib 235・ghost 39・ほか 8）が緑。`cargo build -p areka-emo-text --examples` 可 |
| 4（5.7） | `cargo test -p areka-runtime`（1080 passed・3 ignored） |
| 5（6.1〜6.7） | `cargo test -p areka-input`（294）・`-p areka-session`（534・3 ignored）・`-p areka`（lib 1263 ほか）が緑。`cargo build -p areka --examples`・`cargo build --workspace` 可。先進坑のワークスペースで `cargo check` が通る |
| 6（7.2） | 全体テストの 1 回目は 7.2 が回して書く |

- 1,000 行に近いファイルを、育てる前に話題で分けた所（テストの本数は分ける前と同じ）: dola の `tests/trigger/runtime_test.rs`（854 行）→ `runtime_test.rs`・`runtime_loop_test.rs`・`runtime_chain_test.rs`（2.13）／wintf の `wintf/src/ecs/widget/text/typewriter/tests.rs`（944 行）→ 共通の組み立て＋`tests/{talk,events,timeline,cache,draw}.rs`（3.2）／areka-seriko の `looper_tests.rs`（975 行）→ `looper_tests.rs`・`looper_order_tests.rs`・`looper_progress_tests.rs`・`looper_blink_tests.rs`、`looper_talk_tests.rs`（861 行）→ `looper_talk_tests.rs`・`looper_talk_part_tests.rs`（4.3。173 本のまま）、`parts_tests.rs`（975 行）→ `parts_tests.rs`・`parts_refresh_tests.rs`、`parts_trigger_tests.rs`（906 行）→ `parts_trigger_tests.rs`・`parts_trigger_talk_tests.rs`（4.4。156 本のまま）、関数 `start_top_always` を looper.rs（857 行）から looper_calls.rs へ移した（4.2）／areka-ghost の `dispatcher_choice_tests.rs`（908 行）→ `dispatcher_choice_tests.rs`・`dispatcher_choice_waiting_tests.rs`・`dispatcher_selection_tests.rs`（4.10。15 本のまま）。

各段で整形の検査は緑。clippy の警告で、この spec が触ったファイルに新しく足した物は無い（`wintf/examples/typewriter_demo.rs` の `needless_update` 2 件と、`pilot` の manifest の警告 2 件は、この spec の前から在る）。

### 4.6 後半

（12.3 が書く。届いた時に当てる所が残っていない事・受け手が起点を持たない事・公開の口の `f64` の一覧・後半の残っていない物の検索）
