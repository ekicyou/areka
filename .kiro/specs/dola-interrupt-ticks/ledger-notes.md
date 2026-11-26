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
