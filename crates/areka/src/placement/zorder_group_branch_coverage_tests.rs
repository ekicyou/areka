//! 要件 10.2 が名指しする 10 分岐と、それを覆う「実際に実行されるテスト」の対応表。
//!
//! # なぜ表を置くか
//!
//! 要件 10.2 は分岐の名前を 10 個並べているが、どの分岐がどのテストで覆われているかは
//! テスト名の綴りを人が読み解くしかない。分岐の 1 つを覆っていた唯一のテストが消えても、
//! 残りが全部緑なら誰も気づかない——「網羅している」という主張が、どこにも書かれて
//! いないために検査できない状態になる。
//!
//! # 表が名簿倒れにならないための形
//!
//! ただ名前を並べただけの表は、現実からずれても赤にならない（`tick_wake.rs` の散文名簿が
//! 実際にずれ、見つけたのは人の目でテストではなかった）。そこで本ファイルは表の各行に
//! ついて次を機械で検査する。
//!
//! ⑴ 名指ししたテスト関数が、そのファイルに**実在し**（`fn 名前(` が註釈行の除去後に在る）、
//! ⑵ その直前に `#[test]` が付いている（＝**実行される**。名前だけ残った私設ヘルパーは通さない）、
//! ⑶ そのテストファイルが本番ファイルの `#[path = …]` 宣言で**クレートへ組み込まれている**
//!    （＝ディスクに在るだけでコンパイルされないファイルを「覆っている」と数えない）、
//! ⑷ 10 分岐のどれもが少なくとも 1 行を持ち、表に**未知の分岐名が現れない**。
//!
//! 改名・削除・宣言外しのいずれもこの場で赤になる。
//!
//! # 範囲
//!
//! 10 分岐はいずれも解釈・台帳・起動時適用の判断であり、錨はすべて areka 側に在る。
//! wintf 側（重なりの是正の要否・見送りの理由語）は別クレートのファイルを読めないため
//! ここには載せない——`tick_gate_config_producers_tests.rs` が示した「クレートの境界で
//! 表を分ける」流儀に揃えてある。wintf 側の分岐は `zorder_group_decision_tests.rs` が
//! 自前で覆う。

use std::collections::BTreeSet;

/// 要件 10.2 が名指しする 10 分岐。並びも語も要件本文の逐語である。
///
/// 9 番目を tasks.md は「shell 設定の適用」と言い換えているが、ここは要件の字面
/// （「descript 適用」）を採る——対応表が名指すべきは要件の分岐そのものだからである。
const BRANCHES: [&str; 10] = [
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
];

/// 対応表が読むテストファイル（見出し・中身・そのファイルを組み込む本番ファイルの中身）。
///
/// 3 つ目の欄は「このテストファイルが本当にコンパイルされるか」を見るためのもので、
/// `#[path = "…"]` 宣言を持っている本番ファイルの中身を入れる。
const TEST_FILES: [(&str, &str, &str); 5] = [
    (
        "placement/zorder_group_ledger_tests.rs",
        include_str!("zorder_group_ledger_tests.rs"),
        include_str!("zorder_group_ledger.rs"),
    ),
    (
        "placement/zorder_group_ledger_state_tests.rs",
        include_str!("zorder_group_ledger_state_tests.rs"),
        include_str!("zorder_group_ledger.rs"),
    ),
    (
        "emo2_boot/zorder_cue_tests.rs",
        include_str!("../emo2_boot/zorder_cue_tests.rs"),
        include_str!("../emo2_boot/zorder_cue.rs"),
    ),
    (
        "emo2_boot/frame/zorder_drain_tests.rs",
        include_str!("../emo2_boot/frame/zorder_drain_tests.rs"),
        include_str!("../emo2_boot/frame/zorder_drain.rs"),
    ),
    (
        "emo2_boot/frame/zorder_descript_tests.rs",
        include_str!("../emo2_boot/frame/zorder_descript_tests.rs"),
        include_str!("../emo2_boot/frame/zorder_descript.rs"),
    ),
];

/// 分岐 → それを覆うテスト（分岐の見出し・テストファイルの見出し・テスト関数の名前）。
///
/// 1 分岐に複数行あってよい。解釈の段（台帳の純関数）・台帳の状態遷移・取り出しの段の
/// 記録・起動時適用と、同じ分岐を別の高さで覆う行が並ぶ。
const COVERAGE: [(&str, &str, &str); 38] = [
    // ── 数値モード（要件 1.1・1.2）
    (
        "数値モード",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp1_numeric_mode_expands_each_scope_into_balloon_then_char",
    ),
    (
        "数値モード",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl5_numeric_and_explicit_modes_denote_the_same_scope",
    ),
    (
        "数値モード",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb01_a_numeric_setting_seats_exactly_one_descript_base",
    ),
    // ── 明示モード（要件 2.1・2.2）
    (
        "明示モード",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp2_explicit_long_form_keeps_one_window_per_element",
    ),
    (
        "明示モード",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp3_explicit_short_form_equals_long_form_and_numeric_expansion",
    ),
    (
        "明示モード",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp4_balloon_and_char_of_same_scope_are_distinct_windows",
    ),
    // ── モード混在の拒否（要件 2.3）
    (
        "モード混在の拒否",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp6_reject_mode_mixed",
    ),
    (
        "モード混在の拒否",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp12_mode_mixed_takes_precedence_over_duplicate",
    ),
    // ── タグ内重複要素の拒否（要件 3.4・3.5）
    (
        "タグ内重複要素の拒否",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp7_reject_duplicate_window_element",
    ),
    (
        "タグ内重複要素の拒否",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp4_balloon_and_char_of_same_scope_are_distinct_windows",
    ),
    // ── グループをまたぐ再指定の拒否（要件 3.2・3.3・5.5）
    (
        "グループをまたぐ再指定の拒否",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl4_redesignating_a_scope_rejects_the_whole_tag",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl6_rejection_names_every_colliding_scope_once_in_order",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl7_a_partially_overlapping_tag_is_rejected_in_full",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "emo2_boot/frame/zorder_drain_tests.rs",
        "t_zdr06_cross_group_redesignation_is_rejected_whole_and_recorded",
    ),
    (
        "グループをまたぐ再指定の拒否",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb10_the_seated_base_takes_part_in_the_redesignation_refusal",
    ),
    // ── 要素 2 個未満の無視（要件 1.6）
    (
        "要素 2 個未満の無視",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp8_reject_too_few_elements_counted_before_expansion",
    ),
    (
        "要素 2 個未満の無視",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp5_exactly_two_elements_is_accepted",
    ),
    (
        "要素 2 個未満の無視",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb06_short_and_empty_settings_are_rejected_with_a_reason",
    ),
    // ── スコープ内隣接との矛盾の調停（要件 2.4）
    (
        "スコープ内隣接との矛盾の調停",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp15_inverted_scope_pair_is_folded_into_balloon_then_char",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp16_non_adjacent_scope_pair_is_folded_at_first_appearance",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp17_already_adjacent_block_is_recorded_as_not_reordered",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp21_every_paired_scope_is_adjacent_with_balloon_first_after_normalization",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "emo2_boot/frame/zorder_drain_tests.rs",
        "t_zdr04_normalization_is_surfaced_only_when_the_author_order_was_adjusted",
    ),
    (
        "スコープ内隣接との矛盾の調停",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb04_an_inverted_setting_is_normalized_and_the_adjustment_is_recorded",
    ),
    // ── 解除（要件 4.1〜4.3）
    (
        "解除",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl12_reset_drops_tag_groups_and_returns_to_the_descript_base",
    ),
    (
        "解除",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl13_reset_without_a_base_returns_to_the_default_state",
    ),
    (
        "解除",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl14_reset_frees_tag_scopes_but_not_base_scopes",
    ),
    (
        "解除",
        "emo2_boot/zorder_cue_tests.rs",
        "t_zcs2_reset_zorder_becomes_a_reset_directive",
    ),
    (
        "解除",
        "emo2_boot/frame/zorder_drain_tests.rs",
        "t_zdr08_reset_falls_back_to_the_descript_base_and_records_the_result",
    ),
    // ── descript 適用（要件 5.1〜5.3）
    (
        "descript 適用",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb03_the_setting_is_read_by_the_very_function_the_tag_uses",
    ),
    (
        "descript 適用",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb08_the_seated_base_projects_on_the_first_pass_without_any_tag",
    ),
    (
        "descript 適用",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb11_the_boot_entry_point_seats_the_base_in_the_wired_ledger",
    ),
    (
        "descript 適用",
        "placement/zorder_group_ledger_state_tests.rs",
        "t_zgl8_at_most_one_descript_base_is_kept",
    ),
    // ── 解釈失敗（要件 8.1・5.4）
    (
        "解釈失敗",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp9_reject_unparsable_token_carries_the_received_token",
    ),
    (
        "解釈失敗",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp10_vocabulary_is_case_sensitive",
    ),
    (
        "解釈失敗",
        "placement/zorder_group_ledger_tests.rs",
        "t_zgp11_unparsable_token_takes_precedence_over_other_rejects",
    ),
    (
        "解釈失敗",
        "emo2_boot/frame/zorder_drain_tests.rs",
        "t_zdr05_unparsable_tag_leaves_the_ledger_untouched_and_is_recorded",
    ),
    (
        "解釈失敗",
        "emo2_boot/frame/zorder_descript_tests.rs",
        "t_zdb05_an_unreadable_setting_seats_nothing_and_records_value_and_reason",
    ),
];

/// 註釈の行を落とす——説明文に書いてあるだけの綴りを「在る」と数えないため
/// （`tick_gate_config_producers_tests.rs` と同じ流儀）。改行コードは先に揃える。
fn code_only(src: &str) -> String {
    let normalized = src.replace("\r\n", "\n");
    let body = normalized
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    // 先頭行も「行頭」として探せるよう、番兵の改行を 1 つ前置きする。
    format!("\n{body}\n")
}

/// 見出しからテストファイルの中身を引く。見出しが表に無ければその場で落とす。
fn source_of(label: &str) -> &'static str {
    TEST_FILES
        .iter()
        .find(|(name, _, _)| *name == label)
        .map(|(_, src, _)| *src)
        .unwrap_or_else(|| panic!("対応表が知らないテストファイルの見出しを指している: {label}"))
}

/// 表に載る分岐の見出しは、要件 10.2 の 10 個のいずれかでなければならない。
///
/// 綴り違いの分岐名を足しても表の行数が増えるだけで気づけないので、こちら側から挟む。
#[test]
fn t_zbc1_the_table_names_no_branch_outside_the_ten() {
    let known: BTreeSet<&str> = BRANCHES.into_iter().collect();
    for (branch, file, name) in COVERAGE {
        assert!(
            known.contains(branch),
            "要件 10.2 に無い分岐名が表に載っている: {branch}（{file}::{name}）"
        );
    }
}

/// 10 分岐のどれもが、少なくとも 1 本のテストで覆われていなければならない。
#[test]
fn t_zbc2_every_one_of_the_ten_branches_has_at_least_one_test() {
    let covered: BTreeSet<&str> = COVERAGE.iter().map(|(branch, _, _)| *branch).collect();
    for branch in BRANCHES {
        assert!(
            covered.contains(branch),
            "要件 10.2 の分岐「{branch}」を覆うテストが 1 本も表に無い"
        );
    }
    assert_eq!(
        covered.len(),
        BRANCHES.len(),
        "覆われた分岐の数が 10 でない（覆われている: {covered:?}）"
    );
}

/// 名指ししたテストは実在し、かつ `#[test]` が付いている＝実行される。
///
/// 改名・削除はここで落ちる。`#[test]` を外して私設ヘルパーへ格下げする形も落ちる
/// （名前だけ残っていても「実行されるテスト」ではないため）。
#[test]
fn t_zbc3_every_named_test_exists_and_is_executed() {
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

/// 表に載るテストファイルは、本番ファイルの `#[path = …]` 宣言でクレートへ組み込まれている。
///
/// `include_str!` はディスクのファイルを読むだけなので、モジュール宣言を外された
/// テストファイルも「実在する」ままになる。そこだけは別の証拠で押さえる。
#[test]
fn t_zbc4_every_listed_test_file_is_compiled_into_the_crate() {
    for (label, _, owner) in TEST_FILES {
        let file_name = label
            .rsplit('/')
            .next()
            .expect("見出しはファイル名で終わる");
        let owner_code = code_only(owner);
        assert!(
            owner_code.contains(&format!("#[path = \"{file_name}\"]")),
            "{label} を組み込む #[path] 宣言が本番ファイルに無い（テストが 1 本も走らない）"
        );
    }
}

/// 註釈の除去が両方向で効いていることの較正。
///
/// 除去が効いていないと、説明文に名前が書いてあるだけのテストを「実在する」と数える。
/// 除去が効き過ぎるとコード行まで消えて、何を消しても緑になる。両側から挟む。
#[test]
fn t_zbc5_the_comment_stripper_is_calibrated_in_both_directions() {
    let sample = "/// #[test]\n/// fn t_only_in_a_comment() {}\n#[test]\nfn t_real_one() {}\n";
    let code = code_only(sample);

    assert!(
        !code.contains("t_only_in_a_comment"),
        "註釈にしか無い名前が除去後も残っている（説明文だけで表が緑になる）"
    );
    assert!(
        code.contains("\n#[test]\nfn t_real_one("),
        "コード行の #[test] つき関数が除去で消えている（何を消しても緑になる）"
    );
}
