# 実装計画

- [x] 1. 本番の関数を足し、提示をその呼び出しへ替える
- [x] 1.1 行の列から選択肢の範囲と帯を出す関数と、その結果からクリックを受ける範囲を出す手続きを足し、提示をそれへ替える
  - 設計の Service Interface のとおり、範囲と帯を持つ結果の型・それを返す関数・引数を取らない当たりの手続きを、配置の関数の近くに置く。中では今の 3 つの手順を、今と同じ順と同じ引数で呼ぶ
  - 提示の中で範囲の割り当てと帯の直接の呼び出しを関数 1 回に替え、強調の描画と箱の字の矩形へ結果の範囲と帯を渡す。描き替えたときだけ当たりの手続きを呼ぶ
  - ホバーの絞り・済んだ選択肢の絞り・当たりの写しの組み立て・窓の上の写しは今の位置と中身のまま残す
  - 字幅は関数の呼び出しの間だけ借り、続く描画の可変の借用と重ならない
  - できあがり: `cargo build -p areka-emo-text -j 2` が警告なしで通り、本番の提示のファイルの中で 3 つの手順を呼ぶ文が新しい関数と手続きの中だけになっている（取り込みの `use` の行を除く）。`choice.rs`・`lib.rs`・`Cargo.toml` に差分が無い
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 2.4, 5.1, 5.2, 5.3, 5.4_

- [x] 2. 検査を本番の関数の呼び出しへ替え、帯の共有の検査を足す
- [x] 2.1 文節の折り返しの形を段階ごとに判定する検査の選択肢の判定を、本番の関数の呼び出しへ替える
  - 行の列は今の検査用の読み口のまま得て、決まった字幅で本番の関数を呼び、範囲と当たりをその結果から取る。結果の名前は判定の側にある同じ名前とぶつからないものにする
  - 3 つの手順の取り込みを外し、判定の文（覆う字の比べ・行内の区間の比べ・最後の段階）は 1 字も変えない
  - できあがり: その検査のファイルから 3 つの手順の名前が消え、`cargo test -p areka-emo-text -j 2 choice_highlight_and_click_ranges_come_from_the_judged_rows` が緑
  - 2.2 とはファイルが重ならないが、crate の検査は全ファイルをまとめてビルドするので、片方の書きかけがもう片方の確かめを壊す。並走させず順に行う
  - _Requirements: 2.5, 3.1, 3.2, 3.3, 3.4, 5.5_
- [x] 2.2 選択肢とアンカーの当たりを出す支えを、本番の関数の呼び出しへ替える
  - 選択肢の子の検査から本番の関数へ届くよう、actor の側に検査のビルドだけの再輸出を 2 行足す（ほかの宣言と公開の範囲は変えない）
  - 支えの中の配置の部分を、行の列と描画範囲を返す小さな支えへそのまま切り出し、当たりの支えはそれから本番の関数と当たりの手続きを呼ぶ形にする。返す値の形は今と同じ
  - 既存の選択肢・アンカーの檻の判定は変えない
  - できあがり: その検査のファイルで 3 つの手順を呼ぶ文が無くなり、`cargo test -p areka-emo-text -j 2 anchor_tests` が判定を書き換えずに緑。`cargo build -p areka-emo-text -j 2`（検査なしのビルド）に再輸出が入らず警告も出ない
  - _Requirements: 2.5, 3.1, 3.2, 3.3, 3.4, 3.5, 5.5, 5.6_
- [x] 2.3 強調へ渡す帯とクリックを受ける範囲のブロック軸が同じ帯から出ることを判定する検査を 1 本足す
  - 2.2 と同じファイルを触り、2.2 の支えと再輸出を使うので 2.2 の後に行う
  - 横書き・縦書き右から左・縦書き左から右の 3 つで回す。横書きは行をまたぐ選択肢、縦書きは前置き 19 字＋選択肢 4 字の列をまたぐ台本を使う
  - 判定の前に、どの向きでも範囲が 2 行（列）以上に分かれていることを確かめる
  - 当たりの行の数が範囲の数と同じで 0 でないこと、各行のブロック軸の近い辺と遠い辺が、結果の帯の寄せと丈から出る値に等しいことを判定する
  - _Depends: 2.2_
  - できあがり: 新しい検査 `highlight_bands_and_hit_rows_share_one_band_in_every_writing_mode` が緑で、ファイルが 1,000 行以下
  - _Requirements: 3.6, 4.2_

- [x] 3. 並びを崩す変異で赤を確かめ、全体を緑に戻す
- [x] 3.1 変異 V1〜V7 を 1 つずつ入れて検査の赤を確かめ、記録する
  - 設計の変異の表の V1〜V7 を本番の関数と当たりの手続きにだけ 1 つずつ入れ、`cargo test -p areka-emo-text -j 2` の選択肢の検査（2.1・2.2・2.3 の検査と選択肢の檻）を回して戻す
  - V4・V5 は GPU を使わない検査だけで赤にならなければならない。どれかが赤にならなければ、判定を緩めずに止めて報告する
  - できあがり: このファイルの実装の記録に、変異ごとの中身と赤になった検査の名前の表があり、7 つとも赤が記録されている
  - _Depends: 2.1, 2.2, 2.3_
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
- [x] 3.2 変異を全部戻した状態で、crate の検査と決まりの番人を緑にする
  - `git diff` で変異の跡が無いことと、`choice.rs`・`lib.rs`・`Cargo.toml` に差分が無いことを確かめる
  - できあがり: `cargo test -p areka-emo-text -j 2` が全部緑（1,000 行の番人と純粋層の一覧の番人を含む）で、触ったファイルがどれも 1,000 行以下
  - _Requirements: 2.5, 4.5, 5.2, 5.3, 5.4, 5.7, 5.8_

## Implementation Notes

### 3.1 変異の確かめ（2026-10-31）

変異は `crates/areka-emo-text/src/actor_present.rs` の `ChoiceRanges`・`choice_ranges`・`hit_rows` にだけ 1 つずつ入れ、そのたびに `cargo test -p areka-emo-text -j 2 --no-fail-fast` を回して、変異の前の写しへ戻した。7 つとも赤になった。全部を戻した後の同じ回し方（`--no-fail-fast` なし）は全部緑。

赤になった検査は、何度も出るものを次の組にまとめて書く。

- **アンカーの檻（GPU を使わない・決まった字幅）** — `choice::anchor_tests` の 7 本: `anchor_across_a_wrap_has_a_hit_segment_on_each_line`・`highlight_bands_and_hit_rows_share_one_band_in_every_writing_mode`（帯の共有の検査）・`hit_area_stops_at_what_has_been_revealed`・`mixed_choices_and_anchors_keep_their_rects_and_kinds`・`same_kind_ranges_are_disjoint_and_an_anchor_may_wrap_a_choice`・`vertical_anchor_band_has_the_same_shape_as_a_choice`・`vertical_choice_across_columns_has_a_band_on_each_column`
- **折り返しの検査（GPU を使わない・決まった字幅）** — `actor::lookahead_shapes_tests::choice_highlight_and_click_ranges_come_from_the_judged_rows`
- **提示の檻（GPU を使う・`present_frame` を通る）** — 17 本: `actor::choice_contract_tests` の 7 本（`a_decorated_choice_gets_a_taller_hit_band_through_the_production_frame`・`choice_rows_render_at_indented_positions_readback_pixel_cage`・`hover_move_and_release_leave_no_residue_with_band_offset_readback`・`hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage`・`invert_hover_paints_default_font_color_fill_and_inverted_text_readback_pixel_cage`・`present_populates_choice_hit_rows_and_nochange_preserves_snapshot`・`present_with_hover_adds_highlight_pixels_over_non_hover_frame`）、`actor::choice_settle_tests` の 2 本（`settled_balloon_row_is_unpressable_and_unlit_on_the_gpu`・`settled_box_row_is_unpressable_and_unlit_on_the_gpu`）、`actor::clear_atomicity_tests` の 6 本（`clear_all_resets_hover_and_hit_rows_for_every_actor`・`clear_drops_anchor_hit_rows_and_hit_active_atomically`・`clear_fullclear_pixels_vanish_with_empty_hit_rows_same_frame_atomic`・`clear_resets_hover_and_invalidates_hit_rows_atomically_no_stale_highlight`・`hit_rows_carry_the_kind_of_their_span`・`new_talk_clearall_then_new_choice_set_retains_only_new_set_atomic`）、`actor::region_collapse_present_tests::collapsing_after_drawing_releases_the_surface_and_recovery_reattaches`、`actor::scroll_tests::manual_position_shifts_choice_hit_rows_by_the_lines_scrolled_back`
- **結合の檻（GPU を使う）** — 3 本: `tests/choice_fixture_test.rs` の `real_font_menu_hover_render_dumps_png`、`tests/emo2_fixture_e2e_test.rs` の `real_emo2_menu_cue_sequence_renders_and_hovers_headless`、`tests/line_pitch_readback_test.rs` の `hover_band_centers_in_the_line_box_and_contains_choice_ink`

| 変異 | 入れた中身（具体的に） | 赤になった検査の名前（GPU を使うかどうか添える） |
|---|---|---|
| V1 | `choice_ranges` の `annotate_lines(lines, spans)` を `annotate_lines(lines.get(1..).unwrap_or(&[]), spans)` に替えた（範囲の割り当てへ先頭の行を落とした列を渡す） | アンカーの檻 7 本（GPU なし）・折り返しの検査（GPU なし）・提示の檻 17 本（GPU）・結合の檻 3 本（GPU）。lib の中で 25 本 |
| V2 | `choice_ranges` の `line_bands(lines, mode, metrics)` を `line_bands(&[], mode, metrics)` に替えた（帯へ空の列を渡す） | V1 と同じ全部に加えて `actor::box_present_tests::box_choice_rows_equal_balloon_rows_shifted_by_the_box_position`（GPU）。lib の中で 26 本 |
| V3 | `hit_rows` が `derive_hit_rows` へ渡す `self.lines` を `self.lines.get(1..).unwrap_or(&[])` に替えた（当たりへ先頭の行を落とした列を渡す） | V1 と同じ（アンカーの檻 7 本・折り返しの検査は GPU なし、提示の檻 17 本・結合の檻 3 本は GPU） |
| V4 | `ChoiceRanges` に変異の間だけの欄 `hit_bands: Vec<LineBand>` を足し、返す `bands` は `line_bands(..)` の各帯の `offset` を `+ 3.0` ずらした列、`hit_bands` はずらす前の `line_bands(..)`、`hit_rows` は `&self.hit_bands` を使う（強調の側だけ崩す） | `choice::anchor_tests::highlight_bands_and_hit_rows_share_one_band_in_every_writing_mode`（**GPU なし**）、`actor::choice_contract_tests` の `hover_move_and_release_leave_no_residue_with_band_offset_readback`・`hover_toggle_paints_fill_and_stays_dirty_limited_readback_pixel_cage`・`invert_hover_paints_default_font_color_fill_and_inverted_text_readback_pixel_cage`（GPU）、結合の檻 3 本（GPU） |
| V5 | `hit_rows` が `derive_hit_rows` へ渡す `&self.bands` を、各帯の `offset` を `+ 3.0` ずらした写し（`self.bands.iter().map(..).collect::<Vec<_>>()`）に替え、返す `bands` は元のまま（クリックの側だけ崩す） | アンカーの檻 7 本（**GPU なし**・帯の共有の検査を含む）、`actor::choice_contract_tests` の `choice_rows_render_at_indented_positions_readback_pixel_cage`・`hover_move_and_release_leave_no_residue_with_band_offset_readback`・`invert_hover_paints_default_font_color_fill_and_inverted_text_readback_pixel_cage`（GPU）、結合の檻 3 本（GPU） |
| V6 | `choice_ranges` の `segments` を `{ let _ = annotate_lines(lines, spans); Vec::new() }` に替えた（範囲の割り当てを抜いて空の列） | V2 と同じ（アンカーの檻 7 本・折り返しの検査は GPU なし、提示の檻 17 本・`box_choice_rows_equal_balloon_rows_shifted_by_the_box_position`・結合の檻 3 本は GPU） |
| V7 | `hit_rows` を `let _ = derive_hit_rows(..);` の後に `Vec::new()` を返す形に替えた（当たりを抜いて空の列） | V2 と同じ（アンカーの檻 7 本・折り返しの検査は GPU なし、提示の檻 17 本・`box_choice_rows_equal_balloon_rows_shifted_by_the_box_position`・結合の檻 3 本は GPU） |

- V4・V5 は、GPU を使わない検査（V4 は帯の共有の検査、V5 はアンカーの檻 7 本）だけでも赤になる（要件 4.2）。
- 判定はどれも緩めていない。戻した後の `git diff -- crates/` は空。
