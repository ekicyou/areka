//! `io/sources.rs` の在中テスト。
//!
//! 本物の作業ツリーを読むだけのテストで、ファイルは 1 つも作らない
//! （設計 File Structure Plan）。走査そのものの正しさは実測でしか示せない——
//! 見本のディレクトリを作れない以上、repo の実体が唯一の相手である。
//!
//! **否定の主張には必ず肯定の主張を対で置く**。「調査クレートのファイルが 1 つも
//! 無い」は、走査が何も返さなくても成立してしまう。だから同じテストの中で
//! 「実在する既知のファイルが入っている」ことも確かめる（設計
//! Existing Architecture Analysis が借りる 3 系統目の作法）。

use super::*;
use crate::io::paths::workspace_root;

/// 走査の結果を 1 度だけ取る。
fn walked() -> Vec<(String, String)> {
    walk(&workspace_root()).expect("ワークスペースの走査は通るはず")
}

/// 走査に必ず現れる既知の実在ファイル（areka の語彙台帳。設計 D-5 が名指ししている）。
const KNOWN_FILES: &[&str] = &[
    "crates/areka-sylphya/src/vocab/dotted.rs",
    "crates/areka-sylphya/src/vocab/shiori_resource.rs",
    "crates/log-capture-kit/tests/workspace_scan/mod.rs",
];

// ---- 自分自身を除く（設計 D-3）----

/// 調査クレート由来のファイルが 1 つも混じらないこと。
///
/// 混じると、見本データに書いた ukadoc の URL が本物の証拠として読まれる。
/// 肯定側（既知のファイルが入っている・非空）を同じテストに置いて、
/// 走査が空を返して素通りする形を塞ぐ。
#[test]
fn walk_excludes_the_survey_crate_itself() {
    let files = walked();

    // 肯定側——走査が実際に動いていること。
    assert!(!files.is_empty(), "1 本も走査できていない");
    for known in KNOWN_FILES {
        assert!(
            files.iter().any(|(path, _)| path == known),
            "実在するはずのファイルが走査に無い: {known}"
        );
    }

    // 否定側——調査クレートは 1 本も入らない。
    let leaked: Vec<&String> = files
        .iter()
        .map(|(path, _)| path)
        .filter(|path| path.starts_with("crates/ukadoc-survey/"))
        .collect();
    assert!(
        leaked.is_empty(),
        "調査クレート自身のファイルが混じっている: {leaked:?}"
    );

    // この crate に実在するファイルを名指しで確かめる（除外が「たまたま」でないこと）。
    assert!(
        !files
            .iter()
            .any(|(path, _)| path == "crates/ukadoc-survey/src/io/sources.rs"),
        "このファイル自身が走査に入っている"
    );
}

// ---- 返すパスの形 ----

/// 区切りは `/` に揃える（環境で報告が変わらないため・設計「入出力層」）。
#[test]
fn walk_returns_forward_slash_paths() {
    let files = walked();
    assert!(!files.is_empty(), "1 本も走査できていない");
    for (path, _) in &files {
        assert!(!path.contains('\\'), "逆斜線の区切りが残っている: {path}");
    }
    // 肯定側——階層が実際に `/` で綴られている（平らな名前だけではない）。
    assert!(
        files.iter().any(|(path, _)| path.contains('/')),
        "階層のあるパスが 1 本も無い"
    );
}

/// ワークスペース根からの相対（絶対パスでもドライブ文字でもない）。
#[test]
fn walk_returns_paths_relative_to_the_workspace_root() {
    let files = walked();
    // 走査が空だと以下の全称の主張が無条件に真になり、この檻は何も守らない。
    assert!(!files.is_empty(), "走査が 1 本も返していない");
    for (path, _) in &files {
        assert!(
            path.starts_with("crates/"),
            "根からの相対になっていない: {path}"
        );
        assert!(!path.contains(':'), "絶対パスが混じっている: {path}");
        assert!(
            path.ends_with(".rs"),
            "Rust ファイル以外が混じっている: {path}"
        );
    }
}

/// 名前順・重複なし（順序を固定するのは、落ちたときの出力を再現可能にするため）。
#[test]
fn walk_returns_a_sorted_list_without_duplicates() {
    let files = walked();
    assert!(files.len() > 1, "並びを確かめるには 2 本以上要る");
    for pair in files.windows(2) {
        assert!(
            pair[0].0 < pair[1].0,
            "名前順・重複なしになっていない: {} → {}",
            pair[0].0,
            pair[1].0
        );
    }
}

// ---- 除外するディレクトリ ----

/// 成果物・外部取り込み・版管理は走査に入らない。
#[test]
fn walk_excludes_build_output_and_vendored_and_git_dirs() {
    let files = walked();
    for (path, _) in &files {
        for excluded in ["/target/", "/vendors/", "/.git/"] {
            assert!(
                !path.contains(excluded),
                "除外するディレクトリの下を拾っている: {path}"
            );
        }
    }
    // 肯定側——除外しないもの（本番・テスト・実行例）は拾えている。
    assert!(
        files.iter().any(|(path, _)| path.contains("/src/")),
        "本番のソースが 1 本も無い"
    );
    assert!(
        files.iter().any(|(path, _)| path.contains("/tests/")),
        "テストのソースが 1 本も無い"
    );
}

// ---- 返す本文 ----

/// 本文は読み込みの整形（復帰文字を落とす）を通っている。
#[test]
fn walk_returns_bodies_without_carriage_returns() {
    let files = walked();
    for (path, body) in &files {
        assert!(!body.contains('\r'), "復帰文字が残っている: {path}");
    }
    // 肯定側——既知のファイルの中身が実際に読めている。
    let known = "crates/areka-sylphya/src/vocab/dotted.rs";
    let body = files
        .iter()
        .find(|(path, _)| path == known)
        .map(|(_, body)| body)
        .unwrap_or_else(|| panic!("既知のファイルが無い: {known}"));
    assert!(!body.is_empty(), "本文が空: {known}");
    assert!(
        body.contains("ukadoc"),
        "正典 URL の手掛かりが読めていない: {known}"
    );
}

// ---- 失敗が黙って通らないこと ----

/// 実在しない根を渡したら、探したパスを載せた失敗が返る（黙って空を返さない）。
#[test]
fn walk_reports_a_root_it_cannot_read() {
    let root = workspace_root().join("no-such-root-for-ukadoc-survey-tests");
    assert!(!root.exists(), "前提が崩れている: {}", root.display());
    let err = walk(&root).expect_err("無い場所は走査できないはず");
    match &err {
        SurveyError::Io { path, reason } => {
            assert!(
                path.contains("no-such-root-for-ukadoc-survey-tests"),
                "探したパスが載っていない: {path}"
            );
            assert!(!reason.is_empty(), "理由が空");
        }
        other => panic!("読み書きの失敗として返るはず: {other:?}"),
    }
}
