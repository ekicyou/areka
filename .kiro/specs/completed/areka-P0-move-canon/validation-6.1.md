# タスク 6.1 の確かめ（areka-P0-move-canon）

確かめた時点のコミット: `8574c20a`（タスク 1.1〜5 まで）。

## 1. テストの結果

1 回だけ回した。赤は 0 件なので、回し直しはしていない。

| コマンド | テストの束 | 結果 |
|---|---|---|
| `cargo test -p areka -p areka-kanade --no-fail-fast -j 4` | areka のユニットテスト（`src\main.rs`） | 3127 通過・失敗 0・無視 2 |
| 〃 | `tests\embedded_icon_test.rs` | 2 通過・失敗 0 |
| 〃 | `tests\emo2_real_run.rs` | 1 通過・失敗 0 |
| 〃 | `tests\mcp_get_log_real_run.rs` | 1 通過・失敗 0 |
| 〃 | `tests\smoke_boot_loop_exit.rs` | 5 通過・失敗 0 |
| 〃 | areka-kanade のユニットテスト（`src\lib.rs`） | 589 通過・失敗 0 |
| 〃 | `tests\kanade.rs` | 85 通過・失敗 0 |
| `cargo test -p ukadoc-survey -j 4` | 全部の束 | 601＋6＋113＋5 通過・失敗 0 |
| `cargo fmt --all -- --check` | — | 差分なし（終了コード 0） |
| `cargo test -p areka -j 4 a_script_reset_sends_nothing_to_kanade`（後から足したテストだけ） | areka のユニットテスト | 1 通過・失敗 0 |

無視の 2 件は本 spec の前からあるもの（本 spec の変更では増えていない）。

## 2. 要件 9.3 の分岐ごとの檻

要件 9.3 が挙げる分岐は「二つの書き方の解釈・参照点の計算・基準の解決・吸い付きの当て直し・使えない組み合わせと読めない値の置き換え・窓の固定の指令を受けないこと・並べ直しの判断」の 7 つ。どれも実際の画面を使わない決定論のテストで、上の 1 回の実行で緑だった。パスは `crates/areka/src/` から。

### 2.1 二つの書き方の解釈（要件 1）

| 確かめること | テスト |
|---|---|
| 名前付きの形と旧い位置の形が同じ指令になる（1.1・1.4） | `emo2_boot/move_cue_args_tests.rs::named_and_positional_read_to_same_directive` |
| キー名の大小・並び順を問わない（1.2） | `move_cue_args_tests.rs::named_keys_are_case_and_order_insensitive` |
| 省略・空の欄の既定（1.3・1.4） | `move_cue_args_tests.rs::omitted_and_empty_fields_take_the_same_defaults` |
| `fix` と負の数（1.5・1.6） | `move_cue_args_tests.rs::fix_and_negative_numbers_in_both_forms` |
| 点の無い 1 語は両軸へ（1.7） | `move_cue_args_tests.rs::bare_word_applies_to_both_axes` |
| `--option` の繰り返しをすべて読む（1.8・1.9） | `move_cue_args_tests.rs::option_repeats_are_all_read` |
| 正典に無いキー・値・形を無視して記録（1.10） | `move_cue_args_tests.rs::ignored_parts_are_recorded_and_the_rest_is_read` |
| 重複キーは後勝ちで記録（1.11） | `move_cue_args_tests.rs::duplicate_key_last_wins_and_is_recorded` |
| 旧い位置の形の 6 欄の読み方（1.12） | `move_cue_args_tests.rs::positional_reads_six_fields_and_ignores_the_rest` |
| `--` で始まる引数の有無で形を決める（1.13） | `move_cue_args_tests.rs::form_is_named_iff_some_arg_starts_with_double_dash` |
| 名前付きの形が線へ送られる | `emo2_boot/move_cue_move_sink_tests.rs::named_form_is_sent` |
| emo2 の台本が記録なしで読める | `move_cue_args_tests.rs::emo2_scripts_read_without_notes` |

### 2.2 参照点の計算（要件 2.1〜2.6・6）

| 確かめること | テスト |
|---|---|
| 窓どうしの参照点の組をすべて（2.1・2.2） | `emo2_boot/move_cue_geometry_tests.rs::window_frame_all_ref_point_pairs` |
| モニタの矩形の参照点の組をすべて（2.1・2.2） | `move_cue_geometry_tests.rs::area_frame_all_ref_point_pairs` |
| `base` は既定の基準位置（2.3） | `move_cue_geometry_tests.rs::base_word_reads_canon_default_basepos`・`canon_default_basepos_is_half_width_and_bottom` |
| X・Y にだけ拡大率を掛ける（2.4・2.5） | `move_cue_geometry_tests.rs::k2_scales_script_offsets_only` |
| `fix` の軸は今の位置を保つ（1.5） | `move_cue_geometry_tests.rs::fix_axis_keeps_current_edge` |
| 旧い emo2 の着地が前と同じ（2.6） | `move_cue_geometry_tests.rs::legacy_emo2_lands_where_it_did_before`・`legacy_base_base_scales_both_axes_with_sign` |
| emo2「エモの位置調整」が右下を左下へそろえる（6.1・6.2） | `move_cue_geometry_tests.rs::emo2_adjust_aligns_bottom_right_to_bottom_left` |
| SSP の実測表と同じ位置 | `move_cue_geometry_tests.rs::ssp_measurement_table_positions_match` |
| 奇数の大きさ・負の座標・極端な値 | `move_cue_geometry_tests.rs::odd_sizes_and_negative_coords_round_down`・`extreme_values_saturate_without_panic` |
| バルーンが付いて動き、位置の記憶に書かない（2.7・2.9） | `emo2_boot/move_cue_place_tests.rs::move_keeps_balloon_offset_and_writes_nothing_else`・`emo2_boot/move_cue_apply_move_tests.rs::apply_keeps_balloon_offset` |
| 動かした記録に基準と参照点が載る（9.1） | `move_cue_place_tests.rs::move_applied_records_base_and_ref_points` |

### 2.3 基準の解決（要件 3）

| 確かめること | テスト |
|---|---|
| `screen`（主・副・どのモニタにもいない）・`primaryscreen`・`global`・`me`・スコープ・無いスコープ→`screen`（3.1〜3.7） | `emo2_boot/move_cue_place_tests.rs::every_base_resolves_to_its_frame` |
| 語の読み分けと、他のゴーストの名前→スコープ 0＋記録（3.8） | `emo2_boot/move_cue_args_tests.rs::base_vocabulary_and_other_ghost_name`（スコープ 0 の矩形の解決は上の `every_base_resolves_to_its_frame` の scope0 の行） |
| 置き換えを 1 件ずつ記録（3.7・4.7） | `move_cue_place_tests.rs::substitutions_are_logged_one_by_one` |
| 材料が無いときは記録して飛ばす | `move_cue_place_tests.rs::missing_material_skips_with_warn` |

### 2.4 吸い付きの当て直し（要件 2.10〜2.12）

| 確かめること | テスト |
|---|---|
| 下端に吸い付いた窓は台本の Y を使わない・`free` は台本どおり（2.10・2.11） | `emo2_boot/move_cue_place_tests.rs::bottom_anchor_ignores_script_y_and_free_keeps_it` |
| 移動の後の表情の切り替えで吸い戻されない（2.12） | `move_cue_place_tests.rs::resize_after_move_keeps_bottom_and_center` |
| 同じフレームの大きさの変化の後に移動を解く | `emo2_boot/frame_move_order_tests.rs::a_move_in_the_same_frame_as_a_resize_aligns_with_the_new_size`・`a_move_before_the_resnap_misses_the_right_edge`・`the_frame_calls_the_move_drain_after_the_resnap_and_before_the_chain_finalize` |
| 吸い付きの材料が無いときは記録して台本の位置へ | `move_cue_place_tests.rs::missing_anchored_writes_script_position_with_warn` |

### 2.5 使えない組み合わせと読めない値の置き換え（要件 4）

| 確かめること | テスト |
|---|---|
| `screen`／`primaryscreen` の参照点の `base` → `center`（軸ごと・4.1・4.3） | `emo2_boot/move_cue_geometry_tests.rs::area_base_substitution_is_per_axis` |
| `global` は `left.top` 以外の参照点を使わない（4.2） | `move_cue_geometry_tests.rs::origin_ignores_non_left_top_offset` |
| 正典に無い参照点の語 → `left`／`top`＋記録（4.4） | `emo2_boot/move_cue_args_tests.rs::unknown_ref_words_fall_back_per_axis` |
| 読めない X・Y → `fix`＋記録（4.5） | `move_cue_args_tests.rs::unparsable_axis_falls_back_to_fix` |
| 読めない `time` → 0＋記録・正の `time` も記録（4.6・5.1） | `move_cue_args_tests.rs::duration_unparsable_is_zero_and_positive_is_recorded` |
| 置き換えはトークを止めず 1 件ずつ記録（4.7） | `emo2_boot/move_cue_place_tests.rs::substitutions_are_logged_one_by_one`・`emo2_boot/move_cue_move_severity_log_tests.rs::parse_notes_warn_once_each_and_directive_is_sent` |

### 2.6 窓の固定の指令を受けないこと（要件 7）

| 確かめること | テスト |
|---|---|
| `set,position`／`reset,position` はどの受け手にも届かない（7.1） | `emo2_boot/consumer_ledger_agreement_tests.rs::set_and_reset_position_reach_no_sink` |
| 線へも送られない | `emo2_boot/move_cue_move_sink_tests.rs::other_execute_and_position_tags_are_not_sent` |

### 2.7 並べ直しの判断（要件 8）

| 確かめること | テスト |
|---|---|
| `OnResetWindowPos` の答え 7 通りの判断（8.4・8.5） | `menu/reset_pos_frame_tests.rs::decide_reset_classifies_all_seven_answers` |
| 送れないときはすぐ並べ直す（8.5） | `reset_pos_frame_tests.rs::raise_without_sender_resets_at_once`・`raise_with_closed_sender_resets_at_once` |
| 選ぶと `OnResetWindowPos` を送り答えを待つ（8.3） | `reset_pos_frame_tests.rs::raise_sends_event_and_holds_one_pending_receiver`・`poll_waits_while_no_answer`・`choosing_places_pending_and_greys_the_item_until_answered` |
| 台本が返れば並べ直さない（8.4） | `reset_pos_frame_tests.rs::poll_script_leaves_positions_and_clears_pending` |
| 返事なし・失敗なら並べ直す（8.5） | `reset_pos_frame_tests.rs::poll_no_reply_resets_and_clears_pending`・`poll_failures_reset_with_warn_and_clear_pending`・`poll_dropped_receiver_warns_without_reset_and_clears_pending` |
| 項目の名前（8.1・8.2）と窓が無いときの灰色 | `reset_pos_frame_tests.rs::item_shows_with_default_and_ghost_names`・`item_is_disabled_without_ghost_windows` |
| `\![execute,resetwindowpos]` が並べ直しの手続きへ届く（8.6・8.7） | `emo2_boot/move_cue_move_sink_tests.rs::execute_resetwindowpos_sends_reset`・`move_cue_move_severity_log_tests.rs::execute_resetwindowpos_sends_without_warn`・`emo2_boot/frame_move_order_tests.rs::a_reset_reaches_the_reset_procedure_in_the_frame_order` |
| 台本の `\![execute,resetwindowpos]` は kanade へ何も送らない（`OnResetWindowPos` を送るのはメニューだけ・8.7） | `emo2_boot/frame_move_order_tests.rs::a_script_reset_sends_nothing_to_kanade` |
| 既定の位置へ戻り、位置の記憶に書く（8.6・8.8・9.2） | `placement/reset_pos_tests.rs::shifted_windows_return_to_default_and_are_persisted` |
| 材料・大きさが無いとき（8.6） | `reset_pos_tests.rs::missing_material_only_warns`・`scope_without_size_is_left_out_and_rest_is_solved`・`boot_prepare_carries_the_default_placement_basis` |

7 つの分岐のどれにも緑の檻があった。ただし要件 8.7（台本の並べ直しは `OnResetWindowPos` を送らない）は、これまで作りの上で保証されているだけで、直接確かめる檻が無かった。そこで `a_script_reset_sends_nothing_to_kanade` を足した。

- 中身: メニューの経路が読むのと同じ送り口（`GhostSlot` の会話の kanade）を World に置き、move drain に `PositionCue::ResetWindowPos` を流す。並べ直しの手続きには届き（`reset_window_pos_skipped` が 1 行）、kanade の受け手は空で、答えの受け手（`PendingResetRaise`）も置かれないことを確かめる。
- 赤の確かめ: drain の並べ直しの枝を、一時的にメニューの経路（`raise_reset_window_pos`）に置き換えて回すと、「並べ直しが手続きへ届いていない」で赤になった。元に戻すと緑。

## 3. 1,000 行の決まり

`git diff --name-only 30ded80e..HEAD -- '*.rs'` で出た、本 spec で変えた .rs ファイルの行数（`wc -l`）。「前」は `30ded80e` の時点。

| ファイル | 今 | 前 |
|---|---:|---:|
| crates/areka-kanade/src/schedule/events.rs | 850 | 845 |
| crates/areka-kanade/src/schedule/events_change_tests.rs | 257 | 255 |
| crates/areka-kanade/src/schedule/events_tests.rs | 861 | 858 |
| crates/areka-kanade/src/schedule/resources.rs | 364 | 359 |
| crates/areka/src/emo2_boot/balloon_visibility_lifecycle_e2e_tests.rs | 204 | 204 |
| crates/areka/src/emo2_boot/consumer_ledger.rs | 959 | 943 |
| crates/areka/src/emo2_boot/consumer_ledger_agreement_tests.rs | 355 | 334 |
| crates/areka/src/emo2_boot/frame.rs | 580 | 568 |
| crates/areka/src/emo2_boot/frame/drain_resnap.rs | 546 | 533 |
| crates/areka/src/emo2_boot/frame/switch.rs | 636 | 635 |
| crates/areka/src/emo2_boot/frame/switch_finish_tests.rs | 818 | 790 |
| crates/areka/src/emo2_boot/frame/wiring.rs | 368 | 368 |
| crates/areka/src/emo2_boot/frame_attach_tests.rs | 901 | 898 |
| crates/areka/src/emo2_boot/frame_drain_text_tests.rs | 349 | 302 |
| crates/areka/src/emo2_boot/frame_move_order_tests.rs | 252 | （新規） |
| crates/areka/src/emo2_boot/frame_test_support.rs | 902 | 902 |
| crates/areka/src/emo2_boot/frame_visibility_integration_tests.rs | 597 | 597 |
| crates/areka/src/emo2_boot/mod.rs | 916 | 916 |
| crates/areka/src/emo2_boot/move_cue.rs | 308 | 683 |
| crates/areka/src/emo2_boot/move_cue_apply_move_tests.rs | 407 | 395 |
| crates/areka/src/emo2_boot/move_cue_args.rs | 257 | （新規） |
| crates/areka/src/emo2_boot/move_cue_args_tests.rs | 436 | （新規） |
| crates/areka/src/emo2_boot/move_cue_geometry.rs | 194 | （新規） |
| crates/areka/src/emo2_boot/move_cue_geometry_tests.rs | 654 | （新規） |
| crates/areka/src/emo2_boot/move_cue_move_severity_log_tests.rs | 243 | 185 |
| crates/areka/src/emo2_boot/move_cue_move_sink_tests.rs | 187 | 131 |
| crates/areka/src/emo2_boot/move_cue_place.rs | 206 | （新規） |
| crates/areka/src/emo2_boot/move_cue_place_tests.rs | 507 | （新規） |
| crates/areka/src/emo2_boot/move_cue_tests.rs | （削除） | 394 |
| crates/areka/src/emo2_boot/spine.rs | 937 | 937 |
| crates/areka/src/emo2_boot/zorder_wiring_tests.rs | 538 | 531 |
| crates/areka/src/ghost_session.rs | 884 | 878 |
| crates/areka/src/ghost_session_restart_tests.rs | 433 | 431 |
| crates/areka/src/ghost_session_switch_tests.rs | 524 | 523 |
| crates/areka/src/input_events/balloon_test_support.rs | 196 | 196 |
| crates/areka/src/menu/captions.rs | 319 | 314 |
| crates/areka/src/menu/captions_tests.rs | 431 | 421 |
| crates/areka/src/menu/mod.rs | 368 | 359 |
| crates/areka/src/menu/mod_registry_tests.rs | 193 | 192 |
| crates/areka/src/menu/mod_wiring_tests.rs | 456 | 453 |
| crates/areka/src/menu/plan.rs | 135 | 135 |
| crates/areka/src/menu/plan_tests.rs | 372 | 369 |
| crates/areka/src/menu/reset_pos_frame.rs | 139 | （新規） |
| crates/areka/src/menu/reset_pos_frame_tests.rs | 313 | （新規） |
| crates/areka/src/placement/follow/window_move.rs | **1221** | **1221** |
| crates/areka/src/placement/mod.rs | 912 | 902 |
| crates/areka/src/placement/reset_pos.rs | 122 | （新規） |
| crates/areka/src/placement/reset_pos_tests.rs | 389 | （新規） |
| crates/areka/src/placement/spawn.rs | 776 | 775 |
| crates/ukadoc-survey/tests/consistency/documents_non_vacuity.rs | 770 | 766 |

1,000 行を超えるのは `placement/follow/window_move.rs`（1,221 行）だけで、本 spec の前から同じ 1,221 行である（タスク 2.1 で説明の 1 行を書き替えただけで、行数は変わっていない）。本 spec で 1,000 行を超えたファイル、超える側へ増えたファイルは無い。一番大きいのは `emo2_boot/consumer_ledger.rs` の 959 行。
