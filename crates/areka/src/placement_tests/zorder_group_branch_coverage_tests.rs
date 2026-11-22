//! 11 分岐と、それを覆う「実際に実行されるテスト」の対応表。
//!
//! # なぜ表を置くか
//!
//! 分岐の名前を 11 個並べても、どの分岐がどのテストで覆われているかは
//! テスト名の綴りを人が読み解くしかない。分岐の 1 つを覆っていた唯一のテストが消えても、
//! 残りが全部緑なら誰も気づかない——「網羅している」という主張が、どこにも書かれて
//! いないために検査できない状態になる。
//!
//! # 表が名簿倒れにならないための形
//!
//! ただ名前を並べただけの表は、現実からずれても赤にならない（`tick_wake.rs` の散文名簿が
//! 実際にずれ、見つけたのは人の目でテストではなかった）。そこで表の各行を機械で検査する。
//! 検査の中身は兄弟の `zorder_branch_coverage_test_support.rs`（`check_table`）に 1 つだけ書いて
//! あり、本ファイルの表と `zorder_inversion_branch_coverage_tests.rs` の表が共に通す。改名・削除・宣言外しのいずれも
//! この場で赤になる。
//!
//! # 範囲
//!
//! 11 分岐はいずれも解釈・台帳・起動時適用と、在庫から鎖を導く合成の判断であり、
//! 錨はすべて areka 側に在る。11 番目（窓の出現・破棄に伴う繋ぎ直しの導出）のうち、
//! **何を繋ぐか**を在庫から導く段（`compose_chain` と drain 相の差分公開）は areka 側に在るので
//! ここで覆う。導かれた鎖の差から**どの繋ぎを切って張るか**を組む段（`plan_chain_ops`）と、
//! 重なりの是正の要否・見送りの理由語は wintf 側に在るので、ここには載せない。
//! wintf 側の分岐は `zorder_chain_tests.rs`／`zorder_chain_apply_tests.rs` が
//! 自前で覆う。

use super::zorder_branch_coverage_test_support::check_table;
use areka_session::crate_source;

/// 11 分岐。「窓の出現・破棄に伴う繋ぎ直しの導出」は並立複合語として 1 分岐に数える
/// （「窓の出現」だけ・「破棄に伴う繋ぎ直しの導出」だけを分岐名にすると後者が文の断片になる）。
const BRANCHES: [&str; 11] = [
    "数値モード",
    "明示モード",
    "モード混在の拒否",
    "タグ内重複要素の拒否",
    "グループをまたぐ再指定の拒否",
    "要素 2 個未満の無視",
    "スコープ内隣接との矛盾の調停",
    "解除",
    "descript 適用",
    "解釈失敗",
    "窓の出現・破棄に伴う繋ぎ直しの導出",
];

/// 対応表が読むテストファイル（見出し・中身・そのファイルを組み込む本番ファイルの中身）。
///
/// 3 つ目の欄は「このテストファイルが本当にコンパイルされるか」を見るためのもので、
/// `#[path = "…"]` 宣言を持っている本番ファイルの中身を入れる。
const TEST_FILES: [(&str, &str, &str); 8] = [
    (
        "areka-placement/zorder_group_ledger_tests.rs",
        crate_source!("areka-placement", "src/zorder_group_ledger_tests.rs"),
        crate_source!("areka-placement", "src/zorder_group_ledger.rs"),
    ),
    (
        "areka-placement/zorder_group_ledger_state_tests.rs",
        crate_source!("areka-placement", "src/zorder_group_ledger_state_tests.rs"),
        crate_source!("areka-placement", "src/zorder_group_ledger.rs"),
    ),
    (
        "areka-placement/zorder_group_ledger_written_order_tests.rs",
        crate_source!(
            "areka-placement",
            "src/zorder_group_ledger_written_order_tests.rs"
        ),
        crate_source!("areka-placement", "src/zorder_group_ledger.rs"),
    ),
    (
        "areka-runtime/zorder_cue_tests.rs",
        crate_source!("areka-runtime", "src/zorder_cue_tests.rs"),
        crate_source!("areka-runtime", "src/zorder_cue.rs"),
    ),
    (
        "areka-runtime/frame/zorder_drain_tests.rs",
        crate_source!("areka-runtime", "src/frame/zorder_drain_tests.rs"),
        crate_source!("areka-runtime", "src/frame/zorder_drain.rs"),
    ),
    (
        "areka-runtime/frame/zorder_descript_tests.rs",
        crate_source!("areka-runtime", "src/frame/zorder_descript_tests.rs"),
        crate_source!("areka-runtime", "src/frame/zorder_descript.rs"),
    ),
    (
        "areka-placement/zorder_chain_compose_tests.rs",
        crate_source!("areka-placement", "src/zorder_chain_compose_tests.rs"),
        crate_source!("areka-placement", "src/zorder_chain_compose.rs"),
    ),
    (
        "areka-runtime/frame/zorder_drain_projection_tests.rs",
        crate_source!(
            "areka-runtime",
            "src/frame/zorder_drain_projection_tests.rs"
        ),
        crate_source!("areka-runtime", "src/frame/zorder_drain.rs"),
    ),
];

/// 分岐 → それを覆うテスト（分岐の見出し・テストファイルの見出し・テスト関数の名前）。
///
/// 1 分岐に複数行あってよい。解釈の段（台帳の純関数）・台帳の状態遷移・取り出しの段の
/// 記録・起動時適用と、同じ分岐を別の高さで覆う行が並ぶ。
const COVERAGE: [(&str, &str, &str); 50] = [
    // ── 数値モード
    (
        "数値モード",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp1_numeric_mode_expands_each_scope_into_balloon_then_char",
    ),
    (
        "数値モード",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl5_numeric_and_explicit_modes_denote_the_same_scope",
    ),
    (
        "数値モード",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb01_a_numeric_setting_seats_exactly_one_descript_base",
    ),
    // 解釈の先——在庫から鎖を導く段（`compose_chain`）でも同じ分岐を覆う。
    (
        "数値モード",
        "areka-placement/zorder_chain_compose_tests.rs",
        "t_zcc01_numeric_mode_puts_left_scope_in_front_as_a_block",
    ),
    // ── 明示モード
    (
        "明示モード",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp2_explicit_long_form_keeps_one_window_per_element",
    ),
    (
        "明示モード",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp3_explicit_short_form_equals_long_form_and_numeric_expansion",
    ),
    (
        "明示モード",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp4_balloon_and_char_of_same_scope_are_distinct_windows",
    ),
    (
        "明示モード",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp10_vocabulary_ignores_case",
    ),
    (
        "明示モード",
        "areka-placement/zorder_chain_compose_tests.rs",
        "t_zcc02_explicit_mode_matches_the_numeric_shape",
    ),
    // ── モード混在の拒否
    (
        "モード混在の拒否",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp6_reject_mode_mixed",
    ),
    (
        "モード混在の拒否",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp12_mode_mixed_takes_precedence_over_duplicate",
    ),
    // ── タグ内重複要素の拒否
    (
        "タグ内重複要素の拒否",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp7_reject_duplicate_window_element",
    ),
    (
        "タグ内重複要素の拒否",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp4_balloon_and_char_of_same_scope_are_distinct_windows",
    ),
    // ── グループをまたぐ再指定の拒否
    (
        "グループをまたぐ再指定の拒否",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl4_redesignating_a_scope_rejects_the_whole_tag",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl6_rejection_names_every_colliding_scope_once_in_order",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl7_a_partially_overlapping_tag_is_rejected_in_full",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "areka-runtime/frame/zorder_drain_tests.rs",
        "t_zdr06_cross_group_redesignation_is_rejected_whole_and_recorded",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb10_the_seated_base_takes_part_in_the_redesignation_refusal",
    ),
    // ── 要素 2 個未満の無視
    (
        "要素 2 個未満の無視",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp8_reject_too_few_elements_counted_before_expansion",
    ),
    (
        "要素 2 個未満の無視",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp5_exactly_two_elements_is_accepted",
    ),
    (
        "要素 2 個未満の無視",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb06_short_and_empty_settings_are_rejected_with_a_reason",
    ),
    // ── スコープ内隣接との矛盾の調停
    // 調停の答えは「書かれた順を採る」。
    // 分岐そのもの（逆転・非隣接・隣り合いの見分け）は残るので、その解釈の檻を指す。
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_group_ledger_written_order_tests.rs",
        "t_zwo03_adjacent_inversion_is_kept",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_group_ledger_written_order_tests.rs",
        "t_zwo02_split_order_is_kept_verbatim_and_classified",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_group_ledger_written_order_tests.rs",
        "t_zwo04_balloon_then_char_stays_adjacent",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_group_ledger_written_order_tests.rs",
        "t_zwo01_pasta_inversion_keeps_written_order_and_fills_b1_before_s1",
    ),
    // 片方の窓だけが指名された形も同じ調停の一部である（相棒窓の畳み込み）。
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp23_balloon_only_scope_gains_its_char_window_right_below",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp24_char_only_scope_gains_its_balloon_window_right_above",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-runtime/frame/zorder_drain_tests.rs",
        "t_zdr04_the_written_order_shape_is_surfaced_only_for_the_explicit_mode",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-runtime/frame/zorder_drain_tests.rs",
        "t_zdr11_an_implied_partner_window_is_named_in_the_accepted_record",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb04_an_inverted_setting_is_seated_in_written_order_and_recorded",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "areka-placement/zorder_chain_compose_tests.rs",
        "t_zcc03_folded_partial_specification_yields_full_scope_blocks",
    ),
    // ── 解除
    (
        "解除",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl12_reset_drops_tag_groups_and_returns_to_the_descript_base",
    ),
    (
        "解除",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl13_reset_without_a_base_returns_to_the_default_state",
    ),
    (
        "解除",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl14_reset_frees_tag_scopes_but_not_base_scopes",
    ),
    (
        "解除",
        "areka-runtime/zorder_cue_tests.rs",
        "t_zcs2_reset_zorder_becomes_a_reset_directive",
    ),
    (
        "解除",
        "areka-runtime/frame/zorder_drain_tests.rs",
        "t_zdr08_reset_falls_back_to_the_descript_base_and_records_the_result",
    ),
    // ── descript 適用
    (
        "descript 適用",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb03_the_setting_is_read_by_the_very_function_the_tag_uses",
    ),
    (
        "descript 適用",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb08_the_seated_base_projects_on_the_first_pass_without_any_tag",
    ),
    (
        "descript 適用",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb11_the_boot_entry_point_seats_the_base_in_the_wired_ledger",
    ),
    (
        "descript 適用",
        "areka-placement/zorder_group_ledger_state_tests.rs",
        "t_zgl8_at_most_one_descript_base_is_kept",
    ),
    // ── 解釈失敗
    (
        "解釈失敗",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp9_reject_unparsable_token_carries_the_received_token",
    ),
    (
        "解釈失敗",
        "areka-placement/zorder_group_ledger_tests.rs",
        "t_zgp11_unparsable_token_takes_precedence_over_other_rejects",
    ),
    (
        "解釈失敗",
        "areka-runtime/frame/zorder_drain_tests.rs",
        "t_zdr05_unparsable_tag_leaves_the_ledger_untouched_and_is_recorded",
    ),
    (
        "解釈失敗",
        "areka-runtime/frame/zorder_descript_tests.rs",
        "t_zdb05_an_unreadable_setting_seats_nothing_and_records_value_and_reason",
    ),
    // ── 窓の出現・破棄に伴う繋ぎ直しの導出
    // 在庫が動いた巡に鎖の中身が動くこと（導出）と、動いていない巡には動かないこと（対照）は
    // drain 相が受け持ち、在庫の欠けを鎖へどう射影するかは合成の段が受け持つ。
    (
        "窓の出現・破棄に伴う繋ぎ直しの導出",
        "areka-runtime/frame/zorder_drain_projection_tests.rs",
        "t_zdp06_publishing_happens_only_when_the_content_actually_differs",
    ),
    (
        "窓の出現・破棄に伴う繋ぎ直しの導出",
        "areka-runtime/frame/zorder_drain_projection_tests.rs",
        "t_zdp04_destroyed_scope_leaves_the_chain_without_dragging_others_with_it",
    ),
    (
        "窓の出現・破棄に伴う繋ぎ直しの導出",
        "areka-runtime/frame/zorder_drain_projection_tests.rs",
        "t_zdp08_a_stale_registry_entry_is_skipped_while_its_living_siblings_are_kept",
    ),
    (
        "窓の出現・破棄に伴う繋ぎ直しの導出",
        "areka-runtime/frame/zorder_drain_projection_tests.rs",
        "t_zdp14_an_unnamed_scope_appearing_and_leaving_is_detected_too",
    ),
    (
        "窓の出現・破棄に伴う繋ぎ直しの導出",
        "areka-placement/zorder_chain_compose_tests.rs",
        "t_zcc04_absent_scope_is_reported_and_never_dropped_from_the_group",
    ),
    (
        "窓の出現・破棄に伴う繋ぎ直しの導出",
        "areka-placement/zorder_chain_compose_tests.rs",
        "t_zcc05_partially_present_scope_keeps_the_surviving_window_in_the_chain",
    ),
];

/// 11 分岐のどれもを、実在して実行されるテストが覆い、表のテストファイルがクレートへ
/// 組み込まれている（検査は `check_table`）。
#[test]
fn t_zbc_the_table_covers_every_branch_with_executed_tests() {
    check_table(&BRANCHES, &TEST_FILES, &COVERAGE);
}
