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

## 3.6

- 外したテスト・分かれ道・ログは無い。足した関数 `areka_sakura::reveal::chunk_reveal_offsets` と隣のテスト 12 本。割り切れる長さ（字数 × 50 ms）では今の値と同じ（`production_length_matches_current_values`）。割り切れない長さでは、どの字も今の実数の式の値から 100ns 未満だけ動く（`chained_chunks_keep_error_below_one_tick_and_never_go_back` ほか）。今の実数の式（emo-text の `extend_chunk`・seriko の `push_chunk`）の置き換えは 4.2・4.5。

## 3.7

- 外した確かめ: `compiled_start_times_are_finite_non_negative_non_decreasing` → `compiled_start_times_are_non_decreasing`（有限・非負は型が保証する。非減少と同じ時刻の確かめは残した）。`compile_choice_timeout_tests.rs` の補助 `parse` の `is_finite`。
- 改名（実数のビットの比べ → ちょうどの型の値）: `start_times_sit_on_the_millisecond_grid` → `start_times_accumulate_exactly`・`wait_duration_is_on_the_ms_grid` → `wait_duration_is_exact`・`integer_ms_round_trips_through_secs` → `integer_ms_is_exact`。
- 変える前後の期待: `ChoiceTimeoutDirective::Secs(..)` → `Delta(delta_ms(..))`、桁あふれの行は `Delta(TimeDelta::MAX/MIN)`。`trailing_explicit_wait_remains_in_sheet_and_extends_extent`（実数の和 0.9500000000000001 → `ms(950)`）。起点を押していない台本は `absolute_end_time()` → `relative_end()`。
- 足したテスト: `compile_arm_tests.rs::sysvar_after_text_starts_at_the_preceding_offset`。

## 3.8

- 外したテスト: `drive_delivery_tests.rs::non_finite_tick_is_ignored_and_playback_survives`（有限でない刻みを捨てて `error!` を出す枝の檻。`Tick(TimePoint)` で作れず、枝は 3.5 で取り除いた）。
- 変える前後の期待（名前は変えていない）: 秒の `Tick(x.y)` → `Tick(at_ms(..))`。`ChoiceWaiting` の欄の名前は `display_end`・`timeout_directive`、`display_end` の期待は「起点 + `menu_horizon()`」。`one_step_clock_jump_enters_choice_wait_for_each_directive` の指令 `Some(0.5)`・`Some(0.0)`・`Some(-0.001)`・`None` → `delta_ms(500)`・`delta_ms(0)`・`delta_ms(-1)`・`None`。`same_fixture_and_tick_sequence_produces_identical_observation_each_run` の鍵 `to_bits` → `TimeSpan`。`compile_broadcast_stream_preserves_order_and_choices_also_land_in_bag` は起点を押した台本から再生機を組む。同じ時刻・前の時刻の刻みで 2 度届かない期待（`duplicate_and_backward_tick_do_not_double_fire`）はそのまま。

## 3.9

- 取り除いた分かれ道: `choice_deadline` の `v.is_nan()` の腕と `(v * 1000.0).round() as u64` の飽和の変換。消した型: `MonotonicMs`（再公開ごと）。
- 外したテスト（`schedule/choice.rs` の中）: `choice_deadline_nan_is_indefinite`（NaN は型で表せない）・`choice_deadline_rounds_fractional_seconds_half_away_from_zero`（秒から型への丸めは指令の縁＝areka-sakura の `from_millis_saturating` へ移った）・`choice_deadline_zero_is_indefinite` の中の −0.0 の件。
- 書き替えたテスト: `choice_deadline_tiny_positive_is_immediate_not_indefinite` → `choice_deadline_smallest_positive_is_not_indefinite`（1e-9 秒・期待 1,000 ms → 100ns・期待 `at_ms(1000)` + 1 刻み）・`choice_deadline_positive_adds_seconds` → `choice_deadline_positive_adds_directive`・`choice_deadline_saturates_on_extreme_values`（`INFINITY`／`1e300` → `TimeDelta::MAX`、期待 `u64::MAX` → `TimePoint::MAX`）・`choice_deadline_negative_is_indefinite`（`NEG_INFINITY` → `TimeDelta::MIN`、−0.001 → 1 刻みの負）。
- ログの欄: `deadline_ms`→`deadline`・`overshoot_ms`→`overshoot`・`display_end_ms`→`display_end`・`timeout_directive_secs`→`timeout_directive`・`now_ms`→`now`（Display で出す）。
- `OnSecondChange` の Reference0: 「ミリ秒 ÷ 3,600,000」→「100ns の数 ÷ 36,000,000,000」（`at_ms` で書いた値なら時の数は同じ）。

## 3.10

- 外したテスト・分かれ道・ログは無い。変える前後の期待（名前は変えていない）: `MonotonicMs(n)` → `at_ms(n)`。`msg_tests::kanade_config_new_supplies_documented_defaults`（`close_talk_deadline_ms == 30_000` → `close_talk_deadline == ms(30_000)`、`choice_timeout_default` も同じ）。`schedule_tests::choice_waiting_deadline_follows_dd8_directive_mapping`（指令の秒 → `delta_ms`、期待 `Option<TimePoint>`）。`choice_timeout_never_fires_for_indefinite_deadline`（`u64::MAX` → `TimePoint::MAX`）。`schedule_log_firing_tests::info_choice_waiting_established_logs_candidate_count_and_deadline`（欄 `deadline_ms` → `deadline`、期待 `"Some(32000)"` → 文字の形 `Some(32.0000000)`）。

## 3.11

- 外したテスト・分かれ道・ログは無い。変える前後の期待（名前は変えていない）: 刻みの時刻 `MonotonicMs(n)`・整数 → `at_ms(n)`。`CHOICE_DEADLINE_MS ± x` → `CHOICE_DEADLINE ± ms(..)`。`choice_test_timeout_directive_tests` の指令 `Some(1.234)`・`Some(0.0)`・`Some(-0.001)` → `delta_ms(1_234)`・`delta_ms(0)`・`delta_ms(-1)`。`close_test_handshake_tests` の `close_talk_deadline_ms = u64::MAX`・`5_000` → `close_talk_deadline = TimeSpan::MAX`・`ms(5_000)`。`status_query_test` の欄 `timeout_directive_secs` → `timeout_directive`。

## 3.12

- 段 2 の締め: `cargo test -p wintf`（lib 1607 passed・22 ignored ほか全部緑）・`-p areka-talk`（14）・`-p areka-sakura`（161）・`-p areka-kanade`（651 + 77）が 1 本ずつ緑。`cargo build -p wintf --examples` 可。整形の検査は緑。clippy の警告はこの spec が触ったファイルに新しく足した物は無い（`wintf/examples/typewriter_demo.rs` の `needless_update` 2 件は付け替えの前から在る）。

## 4.1

- 外したテスト・分かれ道・ログは無い。ログの欄: `send_tick` の debug `now_ms` → `now`、settle の warn `wait_ms` → `wait`、配送の失敗の error に `fire_time` を足した（Display）。
- 変える前後の期待: `settle_tests::wait_over_the_cap_stops_at_the_cap`（字面 `"wait_ms=600001"` → `wait=` を `FromStr` で読んで `ms(600_001)` と比べる）・`settle_time_is_the_longest_runonce`（整数 → `ms(want)`）。table.rs の中のテストと table_always・table_film・table_trigger・table_calls・table_calls_check の期待は同じ値を `ms(..)`／`at_ms(..)` へ写した。
- 型: `LoopTrigger::Always/Periodic` の周期は `NonZeroU64` → `TimeSpan`（0 は表を組む所で落とす）。seriko の時計は `TimePoint` を返す（ミリ秒への丸めの元が無くなる）。settle は軸 0 で `TalkCue::scheduled(ZERO, ZERO, ..)` を組む。

## 4.2

- 外したテストは無い。取り除いた分かれ道: `TalkFeed::push_chunk` の実数の式（`r = max(prev + duration/count, at)`）と `count == 0` の早い return（1 字ごとの間隔の関数に替えた）・`TalkEpoch::wall_ms` の「最も近い 1 ms へ丸める」・`TalkEpoch` の実数の起点。
- ログの欄（Display）: trigger の `log_fired` `started_at_ms` → `started_at`、呼び出しの始め・同じ時刻の 2 度目の開始・引き金を構えた debug `at_ms` → `at`、区切りの文字の warn と非単調な刻みの debug `now_ms` → `now`、`last_seen` は Display。
- 振る舞いの変化: `calls.rs` の `just_after`（印の次の時刻 +1 ms → +100ns）・区切りの文字の壁時刻がミリ秒の丸めの無いちょうどの値・割り切れない長さの塊の字の時刻は実数の式から 100ns 未満だけ違う（本番の字数 × 50 ms は今と同じ）。
- 変える前後の期待（timeline.rs の中のテスト）: `starting_at_sets_strictly_future_boundary`・`poll_*` の 4 本・`frame_at_*` を `at_ms`／`ms` へ写した。`u64::MAX` の経過 → `TimeSpan::MAX`。talk_tests の `wall_ms` の丸めの期待は 4.3 が書き直す。
- 移した関数: `start_top_always`（looper.rs → looper_calls.rs。looper.rs は 857 行）。

## 4.3

- 外したテスト・分かれ道・ログは無い。分けたファイル: `looper_tests.rs`（975 行）→ `looper_tests.rs`・`looper_order_tests.rs`・`looper_progress_tests.rs`・`looper_blink_tests.rs`。`looper_talk_tests.rs`（861 行）→ `looper_talk_tests.rs`・`looper_talk_part_tests.rs`。テストは 173 本のまま。
- seriko のミリ秒への丸めが無くなった所: `talk_tests.rs::wall_time_rounds_to_the_nearest_millisecond` → `wall_time_keeps_sub_millisecond_precision`（起点 1000 ms − 0.25 ms を `at_ms(1000)` で観測。前は `wall(0)`・`wall(0.5 ms)`・`wall(1 ms)` が 1000・1000・1001 ms、後は 999.75・1000.25・1000.75 ms ちょうど）。
- 変える前後の期待: `timeline_repeat_tests.rs::lap_of_splits_elapsed_by_period`（`lap_of(u64::MAX, 1)` の `(u64::MAX, 0)` → `lap_of(TimeSpan::MAX, ms(1))` の商と余りを刻みで）。`looper_trigger_tests.rs::glyph_missing_from_feed_falls_back_to_tick_time_with_one_warn`（欄 `now_ms=1234` → `now=` の文字の形）。`calls_tests.rs` の `period_ms: NonZeroU64` → `period: TimeSpan`。ほかはミリ秒の整数を `at_ms`／`ms` へ写した。

## 4.4

- 外したテスト・分かれ道・ログは無い。分けたファイル: `parts_tests.rs`（975 行）→ `parts_tests.rs`・`parts_refresh_tests.rs`、`parts_trigger_tests.rs`（906 行）→ `parts_trigger_tests.rs`・`parts_trigger_talk_tests.rs`。テストは 156 本（lib 全体 545）のまま。
- 変える前後の期待: `parts_calls_tests::part_refresh_judges_periodic_before_applying_calls`（欄の字面 `started_at_ms=1000` → `started_at=` の文字の形）。ほかは秒・ミリ秒の整数を `ms`／`at_ms` へ写した（振る舞いの変わった期待は無い）。`balloon_hide` の `Duration::from_millis(50).as_secs_f64()` → `at_ms(50)`。
- seriko の締め: `cargo test -p areka-seriko` は lib 545・結合テスト全部緑。

## 4.5

- 外したテスト・分かれ道は無い。取り除いたログの欄: `state.rs` の Text と Choice の debug! の `interval`（実数の間隔そのものが無くなった）。足したログの欄: 受け口の配送の失敗の error に `fire_time`。ほかの時刻の欄は Display。
- 字の時刻の式: 実数の `interval` と `max(prev + interval, chunk_start)` → `chunk_reveal_offsets`（割り切れる長さでは値は同じ）。`sink.rs` の中のテストは秒 → `ms(..)` へ写した。

## 4.6

- 外したテスト・分かれ道・ログは無い。変える前後の期待: `choice_anchor_tests::hit_area_stops_at_what_has_been_revealed`（覗く時刻 `shown − 0.5` 秒が shown=0 で負になり長さの型で書けないので、合図の時刻を 1000 ms へずらし `ms(1000*shown + 500)` で覗く。期待の表は同じ）。`state_reveal_tests::visible_glyphs_progress_with_duration_derived_interval`（±0.01 秒の余白の近似 → 境目のちょうどの値 `ms(1049)`→1・`ms(1050)`→2・`ms(1099)`→2・`ms(1100)`→3 と `reveal_times_of == [1000,1050,1100] ms`）。「全部が現れている時刻」（100.0・42.0・`f64::MAX` など）→ `TimeSpan::MAX`。ほかは秒 → `ms(..)`。

## 4.7

- 外したテスト・分かれ道・ログは無い。外した丸め: `actor_lookahead_tests.rs` の `Rig::play` の、実数の誤差を吸うためだけのミリ秒の格子への丸め（判定の時刻はちょうどの `[start_time, start_time + duration]`）。
- 変える前後の期待: `actor_box_present_tests::unpresented_or_unrevealed_boxes_are_not_shown`（時刻 −1.0 で 1 字も見えない確かめ → 字を 1000 ms に置く場の fixture で `ms(999)` は空・`ms(1000)` ちょうどで載る）。ほかは秒 → `ms(..)`、手で組む合図は `TalkCue::scheduled(..)`、`{t:.2}` の表示は文字の形 `{t}`。
- `cargo test -p areka-emo-text --lib` は 1198 passed。

## 4.8

- 外したテスト・分かれ道・ログは無い。変える前後の期待: 秒 → `ms(..)` を 1 対 1 で写した（`pipeline_test::typewriter_reveal_drives_layout_and_window_through_the_pipeline`・`clear_mid_reveal_resets_visibility_and_window_through_the_pipeline` の期待表、`box_attach_test` の `LATE`、`viewbox_scroll_test` の `FULLY_REVEALED`、`decoration_readback` の `SETTLED` ほか）。
- 例: `last_click_time` の番兵 −10.0 → `Option<TimePoint>`、`WATCHDOG_SECS` → `WATCHDOG: TimeSpan`、`CYCLE_SECS` → `CYCLE`、ログと panic の文 `{t:.1}s` → 文字の形 `{t}`。例は公開の `TimeSpan::from_millis` を使う（テストの組み立てを使わない）。
- emo-text の締め: `cargo test -p areka-emo-text` は lib 1198 と結合テスト全部緑、例が組める。

## 4.9

- 取り除いた分かれ道: 関数 `real_clock`（`GetTickCount64` の読み取りとその `// SAFETY:`）・`duration_to_millis` のミリ秒への切り捨て。外したテスト・ログは無い（catch-up と disconnected の info の文言は同じ）。
- 時計の出どころが替わる事による本番の変化（決定論のテストは偽の時計なので結果は同じ）: 眠り（前は眠りの間も数え、復帰で大きく跳んで `ticker catch-up` の info を出し、過ぎた選択待ちの期限は復帰してすぐ `ChoiceTimeoutDue` を送った → 後は眠りを数えず、跳ばず、期限は眠りの分だけ後ろへずれる）。`OnSecondChange`（Reference0 へ渡る値が眠りを含まない時刻になり、眠った機械では時の数が前より小さい）。分解能（前は OS の刻み約 15.6 ms ごと → 後は 100ns の単位で、残りの時間も 100ns ちょうどの `Duration`）。境の格子の 0 の位置（前は起動からの時刻 → 後は眠りを含まない割り込みの時刻の 0。刻み役どうしは同じ格子）。
- 変える前後の期待: `ticker_tests.rs` を `at_ms`・`ms` へ写した（`next_deadline_ms` → `next_deadline`）。`wake_remaining_is_zero_at_and_after_the_wake_and_max_without_one` に「1 刻み先なら 100ns」を足した。`poll_skips_multiple_missed_boundaries_and_fires_only_once`・`spawn_loop_ticker_catch_up_delivers_the_current_clock` は期待は同じで、説明の「サスペンド復帰」を「処理の大幅な遅れ」に直した。

## 4.10

- 取り除いた本番の分かれ道: 選択待ちの表示の終わりをミリ秒へ四捨五入する所（`(secs*1000).round() as u64`）。今は丸めずに 100ns で足し戻す。設計の「前半に入る振る舞いの変化」の一覧に無い変化なので、記録の 3 に足す。`base_now` が無い時の `warn!` は前半では残す（9.1 で消す）。
- ログの欄: `choice_waiting_stale` の `warn!` の `display_end_elapsed_secs`（秒の実数）→ `display_end`（Display）。外したテストは無い。
- 分けたファイル: `dispatcher_choice_tests.rs`（908 行）→ `dispatcher_choice_tests.rs`・`dispatcher_choice_waiting_tests.rs`・`dispatcher_selection_tests.rs`（テスト 15 本のまま）。
- 改名と変える前後の期待: `choice_waiting_converts_elapsed_secs_to_ms_from_the_tick_base_and_forwards_to_kanade` → `choice_waiting_adds_the_tick_base_back_to_display_end_and_forwards_to_kanade`（前は 4,000 + round(356.7) = 4,357 と 4,000 + round(1,250.4) = 5,250 で丸めを両側から挟んだ。後は 4,000 ms + 356.7 ms を丸めずに足す・`at_ms(1_250)` → `at_ms(5_250)`、指令 `Some(12.0)` → `Some(delta_ms(12_000))`）。`tick_relay_converts_absolute_now_to_elapsed_seconds_from_first_tick` → `tick_relay_converts_absolute_now_to_offset_from_first_tick`。`resolve_choice_relays_id_to_matching_talk_without_touching_slot_or_base`（`(0.0, 0.5)` → `(at_ms(0), at_ms(500))`）。`script_choice_timeout_1234_reaches_kanade_with_display_end_and_directive`（`Some(1.234)` → `Some(delta_ms(1_234))`）。

## 4.11

- 外したテスト・分かれ道・ログは無い。足したログの欄: `sink.rs` の `LogSink::emit` に `fire_time`、`at` は Display（文字の形）。
- 変える前後の期待: `sink.rs` の `emit_logs_structured_event_with_at_actor_and_command_kind`・`emit_logs_structured_event_for_newline_cue`（欄 `at` の実数 `Some(1.5)`・`Some(3.25)` → `FromStr` で読んで `TimeSpan::from_millis(1500)`・`(3250)`）。`spine_e2e_test_s5_close_deadline`（`close_talk_deadline_ms` → `close_talk_deadline`）。`spine_e2e_test_s1_boot_success`（期待 `(f64, CueCommand)` → `(TimeSpan, CueCommand)`）。ほかは `MonotonicMs(n)` → `at_ms(n)`・`Tick(0.0)` → `Tick(at_ms(0))`。
- areka-ghost の締め: `cargo test -p areka-ghost` は lib 235・ghost 39・ほか 8 が緑。

## 4.12

- 段 3 の締め: `cargo test -p areka-seriko`（lib 545・結合テスト全部）・`-p areka-emo-text`（lib 1198・2 ignored・結合テスト全部）・`-p areka-ghost`（lib 235・ghost 39・ほか 8）が 1 本ずつ緑。`cargo build -p areka-emo-text --examples` 可。整形の検査は緑。

## 5.1

- 取り除いた分かれ道とログ: `talk_lifecycle.rs` の「有限でない占有の終わりを warn! して捨てる」分かれ道（horizon の `NEG_INFINITY` の番兵は `Option<TimeSpan>` に替えた）・`hover_inject.rs` の負の frame_time を 0 に寄せる所と、有限でない・負の周期を止める守り・`balloon_visibility_wait.rs` の「非数の現在時刻」の説明・組み立ての `ended_at` の `.max(0.0)`・`lib.rs` の `GetTickCount64` を読む `tick_count_ms`（`unsafe` の塊ごと）と `MonotonicMs` で包む所。
- 外したテスト: `talk_lifecycle_tests.rs::non_finite_cue_times_are_skipped_with_a_record`（NaN・無限大は型で書けない）・`hover_inject.rs::cycle_ordinal_defensive_clamps_and_guards`（負の時刻・負の周期・NaN。周期 0 の確かめは `cycle_ordinal_zero_period_is_none` に残した）。
- 足した確かめ: `parse_balloon_timeout_reads_every_shape_of_the_value` に `"1000000000000000"` → `Default`・unreadable、`parse_invalid_values_are_disabled` に `cycle:18446744073709551615` → `Disabled`。
- 改名とログの欄: `configured_timeout_secs` → `configured_timeout`・`resolve_timeout_secs` → `resolve_timeout`・`DEFAULT_BALLOON_TIMEOUT_SECS` → `DEFAULT_BALLOON_TIMEOUT`・`TalkTimeout::Millis(u64)` → `After(TimeSpan)`・`HoverInjectConfig::Cycle { period_ms }` → `{ period }`。欄 `timeout_secs`→`timeout`・`default_secs`→`default`・`period_ms`→`period`（Display）。
- 変える前後の期待: `horizon_matches_dola_occupancy_definition`（0.95 → `ms(950)`）・talk_clock の 4 本（許容幅つきの比べ → ちょうどの値）・`talk_ended_carries_the_injected_clock_once_when_dropped`（1.234 → `ms(1_234)`）・`talk_clock_now_talk_time_reads_the_injected_clock`（2.5 → `ms(2_500)`）・台本の通しの 2 本（`absolute_end_time()` → `relative_end()`）・seriko_clock_tests（ミリ秒 → `TimePoint::from_ticks`）。

## 5.2

- 取り除いた分かれ道: `balloon_visibility_decision.rs` の `DisplayEndAt` の `end.max(0.0)`（負・非数を 0 へ寄せる所）とその説明。
- 外したテスト（`balloon_visibility_tests.rs` の「端の値」の節）: `a_negative_display_end_is_rounded_up_to_zero_before_anchoring`・`a_non_number_display_end_is_rounded_up_to_zero_before_anchoring`・`a_non_number_current_time_never_expires_the_measurement`（どれも型で書けない入力。時刻の分からないフレームで満了させない事は `balloon_visibility_timeout_suppression_cases.rs` の場合 ⑾ と `no_measurement_starts_in_frames_without_time` が押さえる）。
- 変える前後の期待: `parse_accepts_positive_milliseconds`（`"18446744073709551615"` を受け入れ側から外し、`parse_rejects_zero_negative_non_numeric_and_overflow` に「長さの型の範囲を越える」として足した。9 → 10 件）。採った値の記録の欄 `timeout_secs`／`default_secs` → `timeout`／`default`（"30.0" → "30.0000000"、`FromStr` で読んで比べる）。改名 `configured_timeout_secs_reads_env_once_and_is_stable` → `configured_timeout_reads_env_once_and_is_stable`。ほかは秒 → `ms`／`delta_ms`。

## 5.3

- 取り除いた分かれ道: `visibility_wake` の `!remaining.is_finite()` の守りとその説明。外したテスト: `balloon_visibility_phase_wake_tests.rs::non_finite_deadline_arms_nothing`（無限大・NaN の期限と今は型で書けない。期限か今が無い場合は `no_measurement_arms_nothing` が押さえる）。
- 変える前後の期待: 起こしのテストは秒 → `ms(..)`、`Duration::from_secs_f64(x)` → `Duration::from_millis(..)`、上限の確かめは `ms(u32::MAX)` と `MAX_WAIT == 1 時間`。`DisplayEndAt(2/5/3 秒)` → `Some(ms(5000))`。記録のテストは欄の字面（"4.5"・"10.0"）→ `FromStr` で読んで `ms(..)` と比べる。ログの欄 `stopped_at`・`display_end`・`deadline`・`talk_time` は Display。`MAX_WAIT_SECS: f64` → `MAX_WAIT: Duration`。

## 5.4

- 外したテスト・確かめ・分かれ道・ログは無い。変える前後の期待: 秒 → `ms(..)`、`X - 0.001` → `earlier(X, ms(1))`、`TalkTimeout::Millis(n)` → `After(ms(n))`（`talk_timeout_replaces_the_wait_until_the_next_talk` の期限 `2.0 + 1234.0/1000.0` → `ms(2000) + ms(1234)` ほか）。`script_occupancy_end_becomes_the_measurement_anchor_in_the_decision_core` は起点 0 を押した台本で、ログの欄 `display_end`・`deadline` を Debug の実数 → Display の文字の形で比べる。

## 5.5

- 外したテスト・分かれ道・ログ・変わった期待は無い（手で組む合図を `TalkCue::scheduled(TimePoint::ZERO, ..)` へ、`frame/window_state_tests.rs` の `player_for` は起点 0 を押した台本で組む）。

## 5.6

- 外したテスト・分かれ道・変わった期待は無い（単位を写しただけ。`strict_judges_tests.rs` の 1 刻み 1e9 秒 → `TimeSpan::from_millis(u32::MAX)`。台本は数時間に収まるので 1 刻みで流れ切る）。ログの欄 `talk_time` は Display。`frame.rs` の `resolve_talk_time` の再公開は `#[cfg(test)]`。
- areka-runtime は組める: `cargo test -p areka-runtime` は 1080 passed・3 ignored。

## 5.7

- 段 4 の締め: `cargo test -p areka-runtime` は 1080 passed・0 failed・3 ignored（5.6 のレビューで今のコードの上で回した）。

## 6.1

- 外したテスト・分かれ道・ログ・変わった期待は無い（手で組む合図を `TalkCue::scheduled` へ、`present_frame` の秒 → `ms(..)`、テストの時計は `TimePoint::ZERO`）。`cargo test -p areka-input` は 294 passed。

## 6.2

- 外したテスト・分かれ道・ログ・変わった期待は無い（本番は直す所が無かった。テストの時計は `TimePoint`、`dola::runtime::clock::now` → `dola::now`、手で組む合図は `TalkCue::scheduled`）。`cargo test -p areka-session` は 534 passed・3 ignored。

## 6.3

- 外したテスト・分かれ道・ログは無い。変える前後の期待は単位の写しだけ（`strict_judges_tests.rs` の `t < 1e7` 秒・刻み `1e6` 秒 → `at_ms(10_000_000_000)`・`ms(1_000_000_000)`、`balloon_visibility_phase_notice_tests.rs` の `END + WAIT - 0.001` 秒 → `END + 1999 ms`）。手で組む合図は `TalkCue::scheduled`、`run_text_phase(.., Some(ms(..)))`。

## 6.4

- 外したテスト・分かれ道・ログ・変わった期待は無い（`dola::runtime::clock::now` → `dola::now`、`play_into` は起点 0 を押した台本で組む、`dump_balloon_gpu_tests.rs` の経過の比べは時刻の型）。

## 6.5

- 外したテスト・分かれ道・ログ・変わった期待は無い（`thread_roles_tests.rs` の `NEVER_FIRES` → `TimeSpan`、`offline_dump/render_tests.rs` の `settle_time` の期待 → `TimeSpan`、例 `emo-present` の `CYCLE_INTERVAL_SECS: f64` → `CYCLE_INTERVAL: TimeSpan`・`next_switch_at: TimePoint`）。例の表示「2.5 秒周期」が文字の形「2.5000000 秒周期」になった。

## 6.6

- 外したテスト・分かれ道・ログ・変わった期待は無い（先進坑の `q7-vertical/src/old_side.rs` の `SETTLED` → `TimeSpan::from_millis(100_000)`、合図は `TalkCue::scheduled`）。先進坑のワークスペースで `cargo check` が通る。

## 6.7

- 段 5 の締め: `cargo test -p areka` は全部緑（lib 1263 ほか）、`cargo build -p areka --examples` 可、`cargo build --workspace` 可（`pilot` の manifest の警告 2 件はこの spec の前から在る）。

## 7.2

- 全体テストの 1 回目（`tools/test-all.ps1 -Format`、コミット f022cab6c、排他テストの机を取って 1 回）: 23 段すべて緑（x64 workspace tests 661 秒・dola toml feature tests・dola non-windows check を含む）。整形の差分は出なかった。
