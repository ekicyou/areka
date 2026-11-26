# 記録に書く事の控え（7.1・12.3 が `migration-ledger.md` へ集める）

進行役が、各タスクの報告に挙がった物をここへ写す。7.1・12.3 が記録を書いた後に、このファイルを消す。

## 2.1

- 外したテスト: `runtime::clock::tests` の `now_is_monotonically_increasing`・`now_returns_positive_finite_value`・`now_has_millisecond_precision`。古い QPC の時計（実数の秒）をファイルごと消したため。同じ事は `time/clock_tests.rs`（戻らない事・OS の値と同じ物差し）が確かめる。
- 外したテストの行: `error.rs` の `display_loop_offset_variants` の `LoopOffsetNegativeMin`・`LoopOffsetNegativeMax` の 2 行。variant を消したため。負のずらしは読み込みの失敗になる。
- 変えた期待: `display_loop_offset_variants` の `LoopOffsetRangeInverted` の字面「min 3 > max 1」→「min 3.0000000 > max 1.0000000」（長さの型の文字の形）。
- 足したテスト: `display_time_variants`（`TimeOutOfDocumentRange`・`LoopDurationOutOfRange` の字面）。
- 取り除いた本番の分かれ道: 古い時計の QPC の周波数の割り算と、Result を捨てていた所。消した欄: `CompiledStoryboard.start_time`。

## 2.2

- 取り除いた分かれ道: `sum_on_ms_grid`（ミリ秒の格子の継ぎ）とその再公開・`clamp_duration`（有限でない・負の長さを 0 にする）・`CueSheet::new` の −0.0／NaN の全順序の比べ（`start_time` の安定な並べ替えに替えた）。取り除いたログは無い。
- 変える前後の期待: `absolute_start_time()` は押されていない時に 0.0 → `None`。`absolute_fire_time()`／`absolute_end_time()` は実数 → `Option<TimePoint>`。`to_talk_schedule` は起点が無いと `Err(OriginNotStamped)`。直列化は実数の秒 → 100ns の整数、起点と時間切れは無い時に欄ごと書かない。
- 持ち主のタスクが外す・直す見込みのテスト（2.4・2.6・2.7）: `tests/cue/sheet_test.rs` の `cue_sheet_new_with_nan_start_time_does_not_panic`・`to_talk_schedule_clamps_non_finite_and_negative_duration_into_envelope_and_horizon`・`end_time_equals_occupancy_horizon_for_non_finite_durations`、`tests/cue/sheet_sort_test.rs` の `sort_with_many_nan_follows_total_order`・`negative_zero_keeps_script_order`、`src/cue/schedule/tests.rs` の NaN の 3 本。ミリ秒の格子の物（`end_time_of_ms_grid_cues_is_exact`・`occupancy_horizon_matches_ms_grid_end`・`completes_exactly_at_ms_grid_end`）はちょうどの値へ写せば残せる。

## 2.3

- 取り除いた分かれ道（`cue/schedule.rs`）: `insert`・`insert_each` の「隔たりは 0 以上」の `debug_assert`（型が保証する）・−0.0 を +0.0 にそろえる `total_cmp` の並べ替え（安定な `sort_by_key(Reverse(offset))` に替えた。同じ瞬間の FIFO は変わらない）・NaN を「届いた」に倒す `>` の否定の比べと `#[allow(clippy::neg_cmp_op_on_partial_ord)]`・`sum_on_ms_grid` の 3 か所（到達・完了・`occupancy_horizon`）。取り除いたログは無い。
- 外す見込みのテスト（2.4・2.5 が外す。NaN の時刻・隔たりは型で作れない）: `schedule/tests.rs` の `tick_with_nan_time_delivers_all_pending_payloads`・`tick_with_nan_time_stops_at_barrier_then_normal_tick_recovers`・`insert_nan_offset_fires_debug_assert_in_debug_builds`。
- 変える前後の期待: `CuePlayer::from_sheet` に起点の押されていない台本 → 前は起点 0 として動く、後は `Err(CueError::OriginNotStamped)`。`carrier_cue_builds_canonical_carrier_at_zero` → 前は `at == 0.0`・`duration == 0.0`、後は `fire_time == TimePoint::ZERO`・`at`・`duration` が `TimeSpan::ZERO`。到達の判定 → 前は `sum_on_ms_grid(起点, 隔たり) > 今` の否定、後は `起点 + 隔たり <= 今` の整数の比べ。

## 2.4

- 外したテスト・分かれ道・ログは無い。変える前後の期待（実数の秒 → ちょうどの 100ns の値）: `domain_types_serde_roundtrip`・`cue_payload_and_cue_serde_roundtrip`（時間切れ 3.0 → `delta_ms(3000)`）、`wait_carries_no_action_and_time_lives_in_envelope_duration`・`cue_duration_roundtrip_preserves_value`・`duration_is_uniform_envelope_field_across_all_payloads`（秒 → `ms(..)`）、`cue_duration_defaults_to_zero_for_legacy_serialized_data`（JSON の秒 → 100ns の整数）。

## 2.5

- 外したテスト（NaN の時刻・隔たりは型で作れない）: `tick_with_nan_time_delivers_all_pending_payloads`・`tick_with_nan_time_stops_at_barrier_then_normal_tick_recovers`・`insert_nan_offset_fires_debug_assert_in_debug_builds`（節「非有限時刻の境界」ごと）。
- 外したテスト: `rounds_down_value_really_rounds_down_when_subtracted` と定数 `ROUNDS_DOWN`（実数の引き算で手前へ落ちる事が前提。整数の足し算に丸めが無い）。守っていた「ちょうどの刻みで届く」は `payload_is_delivered_at_exact_absolute_fire_time_and_not_one_tick_before` が引き継ぐ。
- 変える前後の期待: `payload_is_delivered_at_exact_absolute_fire_time` → `..._and_not_one_tick_before` へ改名し、1 刻み手前で届かない事を足した。`completion_is_reached_at_exact_occupancy_horizon` に 1 刻み手前で未完了を足した。`choice_barrier_with_negative_timeout_stops_on_arrival` の時間切れ −0.001 秒 → `delta_ms(-1)`。ほかの入力は秒 → `at_ms`・`ms`・`delta_ms` のちょうどの値。

## 2.6

- 外したテスト（`tests/cue/sheet_test.rs`）: `cue_sheet_new_with_nan_start_time_does_not_panic`（NaN の開始は型で作れない。節「非有限 start_time の境界」ごと）・`to_talk_schedule_clamps_non_finite_and_negative_duration_into_envelope_and_horizon`（有限でない・負の長さは型で作れない。読み込みの失敗になり `json_negative_lengths_fail_to_load` が引き継ぐ）・`end_time_equals_occupancy_horizon_for_non_finite_durations`（同上。等しい事は `end_time_and_occupancy_horizon_are_exact`・`to_talk_schedule_completes_exactly_at_sheet_absolute_end_time` が確かめる）・`off_grid_values_keep_plain_sum`（ミリ秒の格子の継ぎが無くなり、確かめる中身が無い）。
- 改名・置き換え: `cue_sheet_absolute_start_time_defaults_to_zero` → `unstamped_sheet_has_no_absolute_times`（0.0 → `None`）。`cue_sheet_deserializes_without_absolute_start_time_as_zero` → `json_missing_duration_reads_as_zero`・`json_unstamped_sheet_omits_origin_and_roundtrips`。`cue_sheet_serde_roundtrip`・`cue_sheet_serde_roundtrip_preserves_absolute_start_time` → `json_roundtrip_preserves_every_field`。`to_talk_schedule_carries_finite_duration_untransformed` の `to_bits` の比べ → 100ns の整数の `==`。ミリ秒の格子の 3 本（`end_time_of_ms_grid_cues_is_exact`・`occupancy_horizon_matches_ms_grid_end`・`completes_exactly_at_ms_grid_end`）→ `end_time_and_occupancy_horizon_are_exact`・`completes_exactly_at_end`。
- 足したテスト（新しい `tests/cue/sheet_serde_test.rs`）: JSON と TOML の往復・100ns の整数で書く事・起点の押されていない台本・0 と負と無い時間切れ・整数でない値・負の長さ・i64 を越える整数・`null` を「無い」と読む・実数の秒の古い文書を拒む。

## 2.7

- 外したテスト（`tests/cue/sheet_sort_test.rs`）: `sort_with_many_nan_follows_total_order`（NaN の開始時刻は型で作れない。補助の `assert_total_order`・`black_box`・`Ordering` も消した）→ 置き換え `sort_many_cues_is_ascending_and_stable`（同じ瞬間 3 件ずつ・降順 24 組が昇順かつ安定に並ぶ）。並べる前の入力を時刻の表に並べさせる部分は、単体テストの `insert_each_matches_per_entry_insert_including_equal_time_fifo`・`insert_same_offset_payloads_delivered_fifo` が確かめる。`negative_zero_keeps_script_order`（−0.0 が無い）→ 置き換え `equal_start_time_keeps_script_order_through_schedule`。
- 外したテスト（`tests/cue/schedule_test.rs`）: `player_emits_cue_at_sheet_absolute_fire_time`（定数 `ROUNDS_DOWN = 2.5999999999999996` と起点 100.0 の実数の丸めの継ぎが前提）→ 置き換え `player_emits_cue_exactly_at_sheet_absolute_fire_time`（ミリ秒の格子の外の隔たり 25,999,999 で、1 刻み手前では配られず、ちょうどで配られる）。

## 2.8

- 外したテスト・分かれ道・ログは無い。足したテスト: `from_sheet_rejects_sheet_without_stamped_origin`。
- 変える前後の期待: `broadcast_delivers_identical_stream_to_every_sink_at_same_absolute_time`（起点 100.0 秒 → `at_ms(100_000)`、期待の合図に発火時刻 100,000／100,500／101,500 ms を足した）。`is_completed_is_gated_by_occupancy_horizon_for_trailing_wait`（`absolute_end_time() - 0.01` → 終わりの 1 刻み手前、`end == at_ms(950)` を足した）。`occupancy_horizon_returns_anchor_plus_relative_horizon`（102.5 秒 → `at_ms(102_500)`、`sheet.absolute_end_time()` と `Some(..)` で比べる）。`occupancy_horizon_matches_completion_threshold`（1 刻み手前 `at_ms(11_999)` で未完了を足した）。ほかは型だけ替えた。起点の無かった台本は起点 0 を押して組む。

## 2.9

- 取り除いた本番の分かれ道（`validate/rules.rs`）: `validate_transition_numbers` の delay・duration の「有限で 0 以上」（`InvalidNumber`）・`validate_numbers` の loop_offset・loop_offset.min／max・offset・trigger_start_offset の「有限」（`InvalidNumber`）・`validate_loop_offset` の `LoopOffsetNegativeMin`／`Max`。取り除いたログは無い。
- 足した分かれ道: 文書の時間の欄の全部（名前つきとインラインの transition の delay・duration、loop_offset の Scalar と Range の min・max、`KeyframeRef::WithOffset.offset`、`trigger_start_offset`）を文書の値の上限で見て `TimeOutOfDocumentRange` にする。
- 変える前後の期待: `builder.rs` の `storyboard_builder_loop_offset_set`（`Scalar(1.5)` → `Scalar(TimeSpan::from_millis(1500))`）。
- untagged の `LoopOffset` は、負の値や上限を越える値の読み込みの失敗の文が「data did not match any variant of untagged enum」になる（テストは失敗した事を見る）。

## 2.10

- 行き先（検査の誤り → 読み込みの失敗）: `schema_test.rs` の `invalid_numbers_are_rejected` の 4 件（`trans.duration=NaN`・`duration=-3.0`・`delay=-1.0`・`loop_offset=Scalar(NaN)`）を外し（ケース 13 → 9 件）、`schema_test.rs::time_field_tests::negative_or_non_finite_times_fail_to_load` へ。`negative_scalar_loop_offset_reports_one_error`（`LoopOffsetNegativeMax`）と、`loop_offset_validation.rs` の `v14_negative_min_error`・`v15_negative_max_error`・`v15_negative_scalar_error`・`nan_scalar_loop_offset_is_rejected`・`nan_range_max_is_rejected_even_with_positive_min`・`nan_range_min_is_rejected` を外し、`loop_offset_serde.rs::negative_or_non_finite_offsets_fail_to_load` へ。
- 変える前後の期待: `all_violations_are_collected`（`["time_scale","loop_count","duration"]` → `["time_scale","loop_count"]`。負の長さは作れない）。`v16_range_inverted_error`（`*min == 5.0` → `TimeSpan::from_millis(5000)`）。`tests/trigger/serde_test.rs` の `trigger_start_offset` は符号つきになり、−0.5 秒の往復を確かめる。ほかは実数の秒 → ちょうどの値。
- 足したテスト: `schema_test.rs::time_field_tests`（往復・上限の端・キーフレームのずらしの両側・検査の報告の順）、`loop_offset_validation.rs::document_range_tests`、`loop_offset_serde.rs` の 4 本、`trigger/serde_test.rs` の 2 本、`trigger/validation_test.rs::trigger_start_offset_beyond_document_range_is_rejected`、新しい `tests/general/time_range_test.rs`（往復・上限の端・`compile_storyboard` と `DolaDocumentBuilder::build` が拒む・toml）。

## 2.11

- 外したテスト: `compile/boundary_test.rs` の `invalid_delay_and_duration_are_rejected_before_compile`（負・NaN・inf の delay／duration は型で作れない。行き先は `validation/schema_test.rs::time_field_tests::negative_or_non_finite_times_fail_to_load`・`general/time_range_test.rs::negative_and_non_finite_durations_fail_to_load_in_toml`）・`nan_delay_that_bypassed_overlap_check_is_rejected`・`between_nan_delay_is_rejected`（同上）・`nan_trigger_start_offset_is_rejected`（行き先 `trigger/serde_test.rs::non_finite_trigger_start_offset_fails_to_load`）。`trigger/compile_test.rs::trigger_fire_time_with_start_time_offset`（開始の時刻を足す事が無くなった。頭からの発火時刻は `trigger_after_keyframe_fire_time`・`trigger_at_start_fire_time` が確かめる）。
- 0 でない開始の時刻の 5 か所: `segment_sort_with_many_nan_follows_total_order` の NaN の回を外し、残りを `segment_sort_is_ascending_by_start_time` へ改名。`at_placement.rs::pure_keyframe_without_at_as_first_entry_falls_back_to_start`（開始 5 秒で 5.0〜6.0 → 0〜1.0 秒）。`sequential.rs::sequential_with_delay`（開始 10 秒で 10.5〜11.5 → 0.5〜1.5 秒）。`sequential_first_entry_uses_compile_start_time` → `sequential_first_entry_starts_from_head`（4.0〜6.0 → 1.0〜3.0 秒、`result.start_time == 3.0` の確かめは欄と一緒に外した）。`trigger_fire_time_with_start_time_offset` は上のとおり。表の外で外した確かめ: `compilation_flow.rs::complex_multi_variable_mixed_placement` の `result.start_time`。
- 変える前後の期待: `overflowing_sums_of_finite_numbers_are_rejected`（小さい倍率の回が `InvalidNumber{loop_duration}` → `LoopDurationOutOfRange`、長さ 1e300 → 2^51−1。時刻のあふれの回は `time_sums_beyond_the_type_range_are_rejected` へ移した）。`toml_non_finite_numbers_are_rejected_by_validate`（時間の欄を外し 8 件 → 5 件、行き先 `toml_non_finite_times_fail_to_load`）。ほかは秒 → ちょうどの値、`result.time_scale` は `.get()` で比べる。
- 取り除いた本番の分かれ道: 時刻の欄（fire_time・keyframe_time・end_time）を inf で見る `push_if_overflowed`（relative_to の値の分は残した）・`total_base_duration / time_scale` が inf なら `InvalidNumber{loop_duration}`・−0.0／NaN をそろえる `total_cmp` の並べ替え（区間と引き金の 2 か所）。取り除いたログは無い。
- 足したテスト: `boundary_test.rs::time_sums_beyond_the_type_range_are_rejected`・`toml_non_finite_times_fail_to_load`・新しい `tests/compile/start_shift_test.rs::values_and_triggers_shift_exactly_by_the_start_difference`。

## 2.12

- 外したテストは無い。取り除いた本番の分かれ道（取り除いたログは無い）: `next_event_time` の NaN を捨てる `filter` とその説明・`check_finish_deadlines` の NaN の期限の説明・`trigger_wall_time` の「周回の終わりで抑える」`.min(end_time)` の継ぎ（G は減らず G(fire) ≤ G(total) なので要らない）・`process_loops` の「丸めで時刻が進まない」を見る `before + (loop_duration + max_delay) <= before`（checked の和で「延ばせない」を見る形に替えた）・`t.next_down()`（100ns 手前に替えた）・周回の後の待ちの `f64::INFINITY` の番兵（列挙に替えた）・積んでいた欄 `end_time`（式で求める）。
- 変える前後の期待（2.13 がテストを直す時に名前で挙げる）: 終わりの予定の時刻の意味 — `loop_integration_test::finite_loop_3_cycles`（1.0 → `Some(3 秒)`）・`infinite_loop_continues_until_cancel`（1.0 → `None`）・`end_time_unified_for_single_and_infinite`（2 つとも 1.0 → 1 回は `Some(1 秒)`・無限は `None`）・`loop_offset_compile::e2e_no_offset_natural_end`（1.0 → `Some(2 秒)`）・`e2e_pause_during_delay_preserves_timing`（`resume` が 9.0 → `None`）・`facade_test/update_flow::pause_freezes_value_and_resume_continues`（1.2 → `Some(1.2 秒)`）・`trigger/runtime_test` の子の `end_time`（2.5 → `Some(2.5 秒)`）。`instance_manager/tests::resume_with_time_before_pause_start_shrinks_end_time` は戻り値（`planned_end`）で見る。下限の境 — `core_types_test::too_short_duration_with_infinite_loop_display`（0.05 → 500,000、文は "0.0500000"・"min: 0.1000000"）・`loop_integration_test::short_duration_infinite_loop_error`（整数の比べ）。倍率が 1 でない時 — `timeline_manager/tests::effective_time_with_time_scale`（F の切り捨ての値）。`event_order_test::loop_end_trigger_survives_pause_rounding`（守っていた 1 ulp の継ぎが消えた）。

## 2.13

- 外したテスト: `instance_manager/tests::nan_finish_deadline_never_fires`（NaN の期限は型で作れない）。置き換え: `infinite_finish_deadline_fires_only_at_infinity` → `max_finish_deadline_fires_only_at_max`、`loop_controller/tests::process_loops_huge_end_time_concludes`（1e17 秒の丸め消えが前提）→ `process_loops_lap_beyond_time_range_concludes`、`event_order_test::loop_end_trigger_survives_pause_rounding`（1 ulp の継ぎが前提）→ `loop_end_trigger_fires_after_pause_shift`、`loop_offset_compile::e2e_f64_precision_many_loops` → `e2e_many_loops_with_wait_stay_exact`、`loop_integration_test::end_time_unified_for_single_and_infinite`（前提が成り立たない）→ `planned_end_time_for_each_loop_shape`（開始 500ms・1 周 1 秒で 1 回 `Some(1500ms)`・待ち無し 3 周 `Some(3500ms)`・決まった待ち 250ms の 3 周 `Some(4000ms)`・引く待ち `None`・終わらない `None`）。取り除いた分かれ道・ログは無い。
- 足したテスト: `infinite_loop_lower_bound_uses_lap_length_through_time_scale`（倍率 0.5 で台本 500,000 は始まり、499,999 は `TooShortDurationWithInfiniteLoop`）・`e2e_pause_with_infinite_loop_has_no_planned_end`・`drawn_wait_finite_loop_resume_returns_end_only_in_last_lap`（引く待ちの有限ループで、最後の周回の前の `resume` は `None`、最後の周回の中では終わりの時刻）。
- 変える前後の期待: 終わりの予定の時刻 — `loop_count_1_compatible_with_existing_behavior`（1.0 → `Some(1000ms)`）・`finite_loop_3_cycles`（1.0 → `Some(3000ms)`）・`infinite_loop_continues_until_cancel`（1.0 → `None`）・`e2e_no_offset_natural_end`（1.0 → `Some(2000ms)`）・`e2e_pause_during_delay_preserves_timing`（無限で `resume` 9.0 → 3 周・決まった待ち 5 秒で `end_time` `Some(13s)`、3.0 秒で止め 5.0 秒で戻すと `Some(15s)`）・`pause_freezes_value_and_resume_continues`（1.2 → `Some(1200ms)`）・`facade_test/load_start` の各 `end_time`・`trigger_with_start_offset`（2.5 → `Some(2500ms)`）・`skipped_loops_fire_their_triggers`（`[0.7,1.7,2.7]` → `Some(..)`）・`instance_manager/tests::pause_and_resume`（`Some(4000ms)`）・`resume_with_time_before_pause_start_shrinks_end_time`（`pause_accumulated == -2000ms`、戻り `Some(0)`）。下限 — `short_duration_infinite_loop_error`（整数の比べ）・`too_short_duration_with_infinite_loop_display`（文 "0.0500000"・"min: 0.1000000"）。倍率が 1 でない時 — `effective_time_with_time_scale`（値は変わらず 50.0）。`event_order_test` の 60fps のコマは `(k·10,000,000 + 30)/60` 刻み。
- 分けたファイル: `tests/trigger/runtime_test.rs`（854 行）→ `runtime_test.rs`・`runtime_loop_test.rs`・`runtime_chain_test.rs`。

## 2.14

- 変えた期待: `tests/cue/sheet_test.rs::stamping_absolute_start_time_leaves_relative_times_untouched` の 2 か所 `ms(1000)` → `ms(500)`（2.6 で型へ写す時の書き誤り。台本の相対の終わりは 500ms）。
- 締めの検査: `cargo test -p dola`・`cargo test -p dola --features toml`・`cargo check -p dola --features toml --target x86_64-unknown-linux-gnu`・clippy（機能 toml）は緑。dola の公開の口で f64 を返す物は `TimeScale::get`（倍率）と `TimeScale::ratio_of`（比）だけで、秒の実数を返す公開の口は 0 件。

## 3.1

- 外したテスト・分かれ道・ログは無い。テストの時刻を型へ写しただけ（`ecs/dola/mod.rs` のテスト・`tests/ecs/dola_animator_test.rs`・`tests/graphics/surface_embedding_test.rs`。期待は同じ）。

## 3.2

- 外したテストは無い。分けたファイル: `wintf/src/ecs/widget/text/typewriter/tests.rs`（944 行）→ 共通の組み立て＋`tests/{talk,events,timeline,cache,draw}.rs`。足したテスト: `glyph_appears_exactly_at_show_at`・`pause_before_start_keeps_remaining_lead_on_resume`・`skip_before_start_runs_whole_timeline`・`update_before_start_reveals_nothing`・`unrepresentable_waits_become_zero_length`・`unrepresentable_default_char_wait_becomes_zero_length`・`timeline_end_is_where_last_item_finishes`。
- 変える前後の期待: `timeline_puts_glyph_after_wait_when_cluster_spans_two_chars`（`abs() < 1e-9` → `assert_eq!(show_at, ms(1400))`）。作者の秒が負・NaN・無限大・範囲の外 → 前は負が時刻を戻し、NaN は後ろの全部を壊し、無限大は後ろが来なかった。後は長さ 0 として `warn!`。項目の無いトークの開始より前の刻み → 前は `Completed`（進行度 1.0）、後は `Playing`（進行度 0）。
- 取り除いた分かれ道: `skip` の `f64::INFINITY`（行程の終わりまで進める形に替えた）。取り除いたログは無い。

## 3.3

- 外したテスト・分かれ道・ログは無い。定数の名前を替えた: `PROMOTE_WINDOW_SECS`・`DEMOTE_QUIET_SECS`・`HOLD_AFTER_PROMOTE_SECS`・`HOLD_AFTER_DEMOTE_SECS` → `PROMOTE_WINDOW`・`DEMOTE_QUIET`・`HOLD_AFTER_PROMOTE`・`HOLD_AFTER_DEMOTE`（長さの型）。
- 変える前後の期待: 「ちょうど前・少し越え」を 1e-9 秒で表していた `active_quiet_demotes_only_after_quiet_and_hold`・`active_not_quiet_stays`・`exactly_one_second_window_promotes_just_over_does_not` → ちょうど 1 刻み。`just_under_8hz_never_promotes` の間隔 1/7 秒 + 1e-6 秒 → 1,428,571 + 10 刻み。ほかは秒 → ちょうどの値。

## 3.4

- 外したテスト・分かれ道・ログは無い。欄の名前を替えた: `ChoiceWaiting.display_end_elapsed_secs: f64` → `display_end: TimePoint`、`timeout_directive_secs: Option<f64>` → `timeout_directive: Option<TimeDelta>`。areka-talk は本番の依存に dola を引く（説明の「依存 0」を改めた）。

## 3.5

- 取り除いた分かれ道とログ: `drive.rs` の `on_tick` の有限でない刻みの守りと、その `error!`（"non-finite Tick ignored; schedule not advanced"）。外した継ぎ: `compile.rs` の Wait の 1 ms 未満の切り捨てと、`ms_to_secs`・`text_playback_ms`。
- 足した分かれ道: `CuePlayer::from_sheet` が `Err` の時、`error!` を出してその台詞を中断として終える（起点を押した直後なので届かない）。
- 振る舞いの変化: 1 ms 未満の `Instruction::Wait` が 100ns の格子でちょうど残る（今の parser は整数のミリ秒しか作らないので本番の値は変わらない）。`ChoiceTimeoutDirective::Secs(f64)` → `Delta(TimeDelta)`。
- 変える前後の期待（`duration.rs` の中のテスト）: `char_count_times_nominal_ms_in_seconds` → `char_count_times_nominal_ms`（実数の秒 → `ms(250)`）。`empty_text_has_zero_duration`（0.0 → `TimeSpan::ZERO`）・`counts_clusters_not_bytes_or_utf16_units`（`ms(150)`）・`zwj_sequence_counts_as_one_cluster`（`ms(50)`）。足したテスト `unrepresentable_millis_stick_to_the_max`。
- 3.8 で外す見込み: `drive_delivery_tests.rs::non_finite_tick_is_ignored_and_playback_survives`。`compile_sheet_tests.rs::compiled_start_times_are_finite_non_negative_non_decreasing` の有限の主張と `compile_choice_timeout_tests.rs` の `is_finite` は型で表せない。
