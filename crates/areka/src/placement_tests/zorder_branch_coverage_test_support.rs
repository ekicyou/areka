//! 分岐の対応表（`zorder_group_branch_coverage_tests.rs`・
//! `zorder_inversion_branch_coverage_tests.rs`）が共に通す検査。
//!
//! 表は「分岐の名前の並び」「テストファイルの表（見出し・中身・そのファイルを組み込む本番
//! ファイルの中身）」「分岐 → 錨のテスト関数の表」の 3 つ。検査は次のとおり。
//!
//! ⑴ 分岐名が重複せず、表に載る分岐名がどれも知っている名前で、どの分岐も 1 行以上を持つ。
//! ⑵ 名指ししたテスト関数がそのファイルに実在し（註釈行の除去後）、直前に `#[test]` が付く。
//! ⑶ 表に載るテストファイルが本番ファイルの `#[path = …]` 宣言でクレートへ組み込まれている。

use std::collections::BTreeSet;

/// 註釈の行を落とす（説明文に書いてあるだけの綴りを「在る」と数えない）。改行コードは先に揃える。
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
fn source_of(files: &[(&str, &'static str, &'static str)], label: &str) -> &'static str {
    files
        .iter()
        .find(|(name, _, _)| *name == label)
        .map(|(_, src, _)| *src)
        .unwrap_or_else(|| panic!("対応表が知らないテストファイルの見出しを指している: {label}"))
}

/// 対応表の 3 つの検査を順に通す。
pub(super) fn check_table(
    branches: &[&str],
    files: &[(&str, &'static str, &'static str)],
    coverage: &[(&str, &str, &str)],
) {
    let known: BTreeSet<&str> = branches.iter().copied().collect();
    assert_eq!(known.len(), branches.len(), "分岐名が重複している");
    for (branch, file, name) in coverage {
        assert!(
            known.contains(branch),
            "未知の分岐名: {branch}（{file}::{name}）"
        );
    }
    let covered: BTreeSet<&str> = coverage.iter().map(|(branch, _, _)| *branch).collect();
    for branch in branches {
        assert!(
            covered.contains(branch),
            "分岐「{branch}」を覆うテストが表に無い"
        );
    }
    for (branch, file, name) in coverage {
        let code = code_only(source_of(files, file));
        assert!(
            code.contains(&format!("\nfn {name}(")),
            "分岐「{branch}」の錨 {file}::{name} が実在しない（改名か削除）"
        );
        assert!(
            code.contains(&format!("\n#[test]\nfn {name}(")),
            "分岐「{branch}」の錨 {file}::{name} に #[test] が付いていない（実行されない）"
        );
    }
    for (label, _, owner) in files {
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

/// 註釈の除去が両方向で効いていることの較正（効かないと説明文だけで表が緑になり、効き過ぎると
/// 何を消しても緑になる）。
#[test]
fn the_comment_stripper_is_calibrated_in_both_directions() {
    let code =
        code_only("/// #[test]\n/// fn t_only_in_a_comment() {}\n#[test]\nfn t_real_one() {}\n");
    assert!(
        !code.contains("t_only_in_a_comment"),
        "註釈にしか無い名前が除去後も残っている"
    );
    assert!(
        code.contains("\n#[test]\nfn t_real_one("),
        "コード行の #[test] つき関数が除去で消えている"
    );
}

/// 検査そのものが赤を出せることの較正（実在しない錨・未知の分岐名・覆われない分岐を 1 つずつ通す）。
#[test]
fn check_table_fails_on_a_missing_anchor_an_unknown_branch_and_an_uncovered_branch() {
    let files = [(
        "a/x_tests.rs",
        "#[test]\nfn t_one() {}\n",
        "#[path = \"x_tests.rs\"]\nmod x;",
    )];
    let ok = [("b", "a/x_tests.rs", "t_one")];
    check_table(&["b"], &files, &ok);
    let run = |branches: &'static [&'static str],
               coverage: &'static [(&'static str, &'static str, &'static str)]| {
        std::panic::catch_unwind(|| check_table(branches, &files, coverage)).is_err()
    };
    assert!(
        run(&["b"], &[("b", "a/x_tests.rs", "t_gone")]),
        "実在しない錨を通した"
    );
    assert!(
        run(&["b"], &[("c", "a/x_tests.rs", "t_one")]),
        "未知の分岐名を通した"
    );
    assert!(
        run(&["b", "d"], &[("b", "a/x_tests.rs", "t_one")]),
        "覆われない分岐を通した"
    );
}
