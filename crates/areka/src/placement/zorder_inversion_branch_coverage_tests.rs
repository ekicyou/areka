//! 要件 8.2（`areka-P0-zorder-scope-inversion`）が名指しする 10 分岐（並びも語も要件本文の逐語）と、
//! 結び直しの 5 通り・外し損ねの巡の輪の確かめを覆う「実際に実行されるテスト」の対応表。
//!
//! 作りは完了 spec の対応表（`zorder_group_branch_coverage_tests.rs`）と同じ: 各行の錨が
//! ⑴ そのファイルに実在し ⑵ 直前に `#[test]` が付き ⑶ そのファイルが本番ファイルの `#[path]` で
//! 組み込まれ ⑷ どの分岐も 1 行以上を持ち、未知の分岐名が無い——を機械で確かめる。
//!
//! 本機能の判断は areka（解釈・合成・公開）と wintf（組の外し・結び直し・輪の確かめ）に
//! またがるので、wintf 側の錨もファイルを読んで（`include_str!`）同じ検査に掛ける。

use std::collections::BTreeSet;

/// 要件 8.2 の 10 分岐（並びも語も要件本文の逐語）と、design の Testing Strategy「替え玉で 1 巡」の
/// 7（結び直しの 5 通り）・4 と 8（外し損ねの巡の輪の確かめ）。
const BRANCHES: [&str; 16] = [
    "逆転",
    "非隣接",
    "逆転と補いの組み合わせ（`s0,b0,s1`）",
    "逆転の無い明示の記法",
    "数字だけの記法",
    "解除で組の重なりへ戻ること",
    "descript の逆転",
    "バルーンの窓の後からの出現と破棄",
    "キャラクターの窓の破棄",
    "逆転や非隣接を含む並びが枝分かれの無い一直線の並びの鎖になること",
    "結び直し⑴ 相手の実体が無い",
    "結び直し⑵ どちらかのハンドルが無い",
    "結び直し⑶ 同じ巡に確立系が張っている",
    "結び直し⑷ 張れた",
    "結び直し⑸ 張れなかった",
    "外し損ねの巡の輪の確かめ",
];

/// 対応表が読むテストファイル（見出し・中身・そのファイルを組み込む本番ファイルの中身）。
const TEST_FILES: [(&str, &str, &str); 11] = [
    (
        "placement/zorder_group_ledger_written_order_tests.rs",
        include_str!("zorder_group_ledger_written_order_tests.rs"),
        include_str!("zorder_group_ledger.rs"),
    ),
    (
        "placement/zorder_chain_compose_tests.rs",
        include_str!("zorder_chain_compose_tests.rs"),
        include_str!("zorder_chain_compose.rs"),
    ),
    (
        "placement/zorder_chain_compose_written_order_tests.rs",
        include_str!("zorder_chain_compose_written_order_tests.rs"),
        include_str!("zorder_chain_compose.rs"),
    ),
    (
        "placement/zorder_chain_inversion_order_tests.rs",
        include_str!("zorder_chain_inversion_order_tests.rs"),
        include_str!("zorder_chain_compose.rs"),
    ),
    (
        "emo2_boot/frame/zorder_drain_written_order_tests.rs",
        include_str!("../emo2_boot/frame/zorder_drain_written_order_tests.rs"),
        include_str!("../emo2_boot/frame/zorder_drain.rs"),
    ),
    (
        "emo2_boot/frame/zorder_descript_tests.rs",
        include_str!("../emo2_boot/frame/zorder_descript_tests.rs"),
        include_str!("../emo2_boot/frame/zorder_descript.rs"),
    ),
    (
        "wintf/zorder_chain_release_tests.rs",
        include_str!("../../../wintf/src/ecs/window/zorder_chain_release_tests.rs"),
        include_str!("../../../wintf/src/ecs/window/zorder_chain.rs"),
    ),
    (
        "wintf/zorder_chain_apply_release_tests.rs",
        include_str!("../../../wintf/src/ecs/window/zorder_chain_apply_release_tests.rs"),
        include_str!("../../../wintf/src/ecs/window/zorder_chain_apply.rs"),
    ),
    (
        "wintf/zorder_chain_order_inversion_tests.rs",
        include_str!("../../../wintf/src/ecs/window/zorder_chain_order_inversion_tests.rs"),
        include_str!("../../../wintf/src/ecs/window/zorder_chain_apply.rs"),
    ),
    (
        "wintf/zorder_pair_establish_release_tests.rs",
        include_str!("../../../wintf/src/ecs/window/zorder_pair_establish_release_tests.rs"),
        include_str!("../../../wintf/src/ecs/window/zorder_pair_establish.rs"),
    ),
    (
        "wintf/zorder_pair_maintain_release_tests.rs",
        include_str!("../../../wintf/src/ecs/window/zorder_pair_maintain_release_tests.rs"),
        include_str!("../../../wintf/src/ecs/window/zorder_pair_maintain.rs"),
    ),
];

const LEDGER: &str = "placement/zorder_group_ledger_written_order_tests.rs";
const COMPOSE: &str = "placement/zorder_chain_compose_tests.rs";
const COMPOSE_WO: &str = "placement/zorder_chain_compose_written_order_tests.rs";
const REAL_A: &str = "placement/zorder_chain_inversion_order_tests.rs";
const DRAIN_WO: &str = "emo2_boot/frame/zorder_drain_written_order_tests.rs";
const DESCRIPT: &str = "emo2_boot/frame/zorder_descript_tests.rs";
const RELEASE: &str = "wintf/zorder_chain_release_tests.rs";
const APPLY: &str = "wintf/zorder_chain_apply_release_tests.rs";
const REAL_W: &str = "wintf/zorder_chain_order_inversion_tests.rs";
const ESTABLISH: &str = "wintf/zorder_pair_establish_release_tests.rs";
const MAINTAIN: &str = "wintf/zorder_pair_maintain_release_tests.rs";

/// 分岐 → それを覆うテスト（分岐・テストファイルの見出し・テスト関数の名前）。
const COVERAGE: [(&str, &str, &str); 52] = [
    // ── 逆転（要件 1）
    ("逆転", LEDGER, "t_zwo03_adjacent_inversion_is_kept"),
    (
        "逆転",
        COMPOSE_WO,
        "t_zcw01_inverted_scope_zero_joins_only_two_edges",
    ),
    ("逆転", RELEASE, "an_inverted_pair_is_released"),
    (
        "逆転",
        APPLY,
        "pair_to_inversion_releases_between_detach_and_attach",
    ),
    (
        "逆転",
        REAL_W,
        "an_inverted_plan_lands_on_real_windows_and_reset_restores_the_pairs",
    ),
    // ── 非隣接（要件 2）
    (
        "非隣接",
        LEDGER,
        "t_zwo02_split_order_is_kept_verbatim_and_classified",
    ),
    (
        "非隣接",
        COMPOSE_WO,
        "t_zcw02_split_scopes_join_every_consecutive_pair",
    ),
    (
        "非隣接",
        DRAIN_WO,
        "t_zdw02_split_line_names_both_split_shapes",
    ),
    ("非隣接", RELEASE, "a_split_pair_is_released"),
    (
        "非隣接",
        REAL_W,
        "a_split_plan_lands_on_real_windows_and_reset_restores_the_pairs",
    ),
    // ── 逆転と補いの組み合わせ（要件 1.2・3）
    (
        "逆転と補いの組み合わせ（`s0,b0,s1`）",
        LEDGER,
        "t_zwo01_pasta_inversion_keeps_written_order_and_fills_b1_before_s1",
    ),
    (
        "逆転と補いの組み合わせ（`s0,b0,s1`）",
        LEDGER,
        "t_zwo06_partner_is_filled_right_behind_a_balloon_and_right_before_a_char",
    ),
    (
        "逆転と補いの組み合わせ（`s0,b0,s1`）",
        DRAIN_WO,
        "t_zdw01_inverted_with_implied_partner_line_is_verbatim",
    ),
    (
        "逆転と補いの組み合わせ（`s0,b0,s1`）",
        REAL_A,
        "the_pasta_directive_lands_s0_b0_b1_s1_on_real_windows",
    ),
    // ── 逆転の無い明示の記法（要件 1.6・4.4）
    (
        "逆転の無い明示の記法",
        LEDGER,
        "t_zwo04_balloon_then_char_stays_adjacent",
    ),
    (
        "逆転の無い明示の記法",
        DRAIN_WO,
        "t_zdw03_existing_spellings_false_and_partner_are_kept",
    ),
    ("逆転の無い明示の記法", RELEASE, "a_contiguous_pair_is_kept"),
    (
        "逆転の無い明示の記法",
        MAINTAIN,
        "a_pair_the_chain_keeps_contiguous_is_still_fixed",
    ),
    // ── 数字だけの記法（要件 4.2）
    (
        "数字だけの記法",
        LEDGER,
        "t_zwo10_numeric_mode_expands_as_before_with_no_record",
    ),
    (
        "数字だけの記法",
        COMPOSE,
        "t_zcc01_numeric_mode_puts_left_scope_in_front_as_a_block",
    ),
    // ── 解除で組の重なりへ戻ること（要件 4.3・4.5）
    (
        "解除で組の重なりへ戻ること",
        DRAIN_WO,
        "t_zdw06_reset_returns_the_plan_to_pairs",
    ),
    (
        "解除で組の重なりへ戻ること",
        DRAIN_WO,
        "t_zdw07_reset_keeps_the_descript_inversion_and_pairs_the_rest",
    ),
    (
        "解除で組の重なりへ戻ること",
        DRAIN_WO,
        "t_zdw08_descript_reload_returns_the_plan_to_pairs",
    ),
    (
        "解除で組の重なりへ戻ること",
        RELEASE,
        "when_the_chain_goes_away_every_marked_pair_is_restored",
    ),
    (
        "解除で組の重なりへ戻ること",
        APPLY,
        "reset_to_none_nudges_each_restored_pair",
    ),
    (
        "解除で組の重なりへ戻ること",
        APPLY,
        "reset_to_descript_base_nudges_the_whole_chain_once",
    ),
    (
        "解除で組の重なりへ戻ること",
        ESTABLISH,
        "after_the_chain_is_cleared_a_new_balloon_is_linked_as_a_pair",
    ),
    // ── descript の逆転（要件 1.3）
    (
        "descript の逆転",
        DRAIN_WO,
        "t_zdw05_descript_inversion_goes_through_the_tag_interpretation",
    ),
    (
        "descript の逆転",
        DESCRIPT,
        "t_zdb04_an_inverted_setting_is_seated_in_written_order_and_recorded",
    ),
    (
        "descript の逆転",
        ESTABLISH,
        "a_pair_the_chain_releases_is_not_linked_when_its_handles_arrive",
    ),
    // ── バルーンの窓の後からの出現と破棄（要件 6.1・6.2）
    (
        "バルーンの窓の後からの出現と破棄",
        REAL_A,
        "a_balloon_that_appears_after_the_directive_joins_in_written_order",
    ),
    (
        "バルーンの窓の後からの出現と破棄",
        REAL_A,
        "a_balloon_destroyed_and_recreated_returns_to_its_written_place",
    ),
    // ── キャラクターの窓の破棄（要件 6.4）
    (
        "キャラクターの窓の破棄",
        REAL_A,
        "a_destroyed_character_leaves_the_rest_in_written_order",
    ),
    (
        "キャラクターの窓の破棄",
        APPLY,
        "handle_wait_carries_over_even_when_the_pair_release_called_the_os",
    ),
    (
        "キャラクターの窓の破棄",
        APPLY,
        "restore_with_missing_peer_only_drops_the_mark",
    ),
    // ── 逆転や非隣接を含む並びが枝分かれの無い一直線の並びの鎖になること
    (
        "逆転や非隣接を含む並びが枝分かれの無い一直線の並びの鎖になること",
        COMPOSE_WO,
        "t_zcw04_every_written_order_yields_one_straight_line",
    ),
    (
        "逆転や非隣接を含む並びが枝分かれの無い一直線の並びの鎖になること",
        COMPOSE_WO,
        "t_zcw05_the_checker_catches_a_pair_edge_written_by_the_chain",
    ),
    (
        "逆転や非隣接を含む並びが枝分かれの無い一直線の並びの鎖になること",
        APPLY,
        "pair_to_inversion_releases_between_detach_and_attach",
    ),
    // ── 結び直しの 5 通り
    (
        "結び直し⑴ 相手の実体が無い",
        APPLY,
        "restore_with_missing_peer_only_drops_the_mark",
    ),
    (
        "結び直し⑵ どちらかのハンドルが無い",
        APPLY,
        "restore_with_missing_handle_only_drops_the_mark",
    ),
    (
        "結び直し⑶ 同じ巡に確立系が張っている",
        APPLY,
        "restore_after_same_round_establishment_only_drops_the_mark",
    ),
    (
        "結び直し⑷ 張れた",
        APPLY,
        "restore_links_the_pair_and_records_it",
    ),
    (
        "結び直し⑸ 張れなかった",
        APPLY,
        "restore_failure_is_recorded_and_the_rest_continue",
    ),
    (
        "結び直し⑸ 張れなかった",
        APPLY,
        "a_failed_restore_still_lets_the_chain_attach_and_nudge",
    ),
    // ── 外し損ねの巡の輪の確かめ（正常な巡は読まない・3 つの源のどれでも確かめる・輪と読めないの見送り）
    (
        "外し損ねの巡の輪の確かめ",
        APPLY,
        "a_clean_reset_reads_no_owner_before_restoring",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        APPLY,
        "a_clean_round_attaches_without_owner_reads",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        APPLY,
        "a_stuck_detach_refuses_the_restore_that_would_loop",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        APPLY,
        "a_diverged_round_checks_each_attach_and_refuses_the_unreadable_one",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        APPLY,
        "a_failed_departing_detach_alone_checks_the_attach",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        APPLY,
        "a_diverged_pair_release_alone_checks_the_attach",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        RELEASE,
        "meeting_the_owned_window_on_the_walk_is_a_loop",
    ),
    (
        "外し損ねの巡の輪の確かめ",
        RELEASE,
        "a_failed_read_on_the_walk_is_unreadable",
    ),
];

/// 註釈の行を落とす（説明文に書いてあるだけの綴りを「在る」と数えない）。改行コードは先に揃える。
fn code_only(src: &str) -> String {
    let normalized = src.replace("\r\n", "\n");
    let body = normalized
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("\n{body}\n")
}

fn source_of(label: &str) -> &'static str {
    TEST_FILES
        .iter()
        .find(|(name, _, _)| *name == label)
        .map(|(_, src, _)| *src)
        .unwrap_or_else(|| panic!("対応表が知らないテストファイルの見出しを指している: {label}"))
}

#[test]
fn t_zic1_the_table_names_only_known_branches_and_covers_all_of_them() {
    let known: BTreeSet<&str> = BRANCHES.into_iter().collect();
    assert_eq!(known.len(), BRANCHES.len(), "分岐名が重複している");
    for (branch, file, name) in COVERAGE {
        assert!(
            known.contains(branch),
            "未知の分岐名: {branch}（{file}::{name}）"
        );
    }
    let covered: BTreeSet<&str> = COVERAGE.iter().map(|(branch, _, _)| *branch).collect();
    for branch in BRANCHES {
        assert!(
            covered.contains(branch),
            "分岐「{branch}」を覆うテストが表に無い"
        );
    }
}

#[test]
fn t_zic2_every_named_test_exists_and_is_executed() {
    for (branch, file, name) in COVERAGE {
        let code = code_only(source_of(file));
        assert!(
            code.contains(&format!("\nfn {name}(")),
            "分岐「{branch}」の錨 {file}::{name} が実在しない（改名か削除）"
        );
        assert!(
            code.contains(&format!("\n#[test]\nfn {name}(")),
            "分岐「{branch}」の錨 {file}::{name} に #[test] が付いていない（実行されない）"
        );
    }
}

#[test]
fn t_zic3_every_listed_test_file_is_compiled_into_its_crate() {
    for (label, _, owner) in TEST_FILES {
        let file_name = label
            .rsplit('/')
            .next()
            .expect("見出しはファイル名で終わる");
        assert!(
            code_only(owner).contains(&format!("#[path = \"{file_name}\"]")),
            "{label} を組み込む #[path] 宣言が本番ファイルに無い（テストが 1 本も走らない）"
        );
    }
}
