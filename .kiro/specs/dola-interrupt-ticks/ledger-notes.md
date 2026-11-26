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
