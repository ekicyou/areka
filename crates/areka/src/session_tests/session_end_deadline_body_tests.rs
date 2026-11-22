//! 期限つきの降ろしと見張りの張りを呼ぶ所を、本体の `src` ほかの木を歩いて固める構造の試験。

use std::collections::BTreeSet;
use std::path::Path;

// ============================ 期限つきの降ろしと見張りの張りを呼ぶ所

/// 走査の字面: 期限つきの降ろし・見張りの張り・上限を受ける入口の呼び出し。
const DEADLINE_CALL_TOKENS: [&str; 3] = [".shutdown_within(", ".arm(", "end_session_within("];

/// 本番ソースでそれらを呼んでよい行（`crates/` からの相対パス＝`<crate>/<src の中のパス>`・前後の空白を落とした行）。
/// OS のセッションの終了の受け手が定数を渡す 1 行・受け手の降ろし・`GhostSession` の委譲の 3 行だけ。
const ALLOWED_DEADLINE_CALLS: [(&str, &str); 3] = [
    (
        "areka/session/ghost_session.rs",
        "let (result, fired) = runtime.shutdown_within(reason, budget);",
    ),
    (
        "areka/session/session_end.rs",
        "end_session_within(world, SESSION_END_SHIORI_LIMIT)",
    ),
    (
        "areka/session/session_end.rs",
        "session.shutdown_within(CloseReason::System, WaitBudget { started, limit });",
    ),
];

/// 本体（`crates/areka/src/`）・`crates/areka-runtime/src/`・`crates/areka-input/src/` の本番 `.rs`（テストとテストの土台＝
/// `_tests.rs`・`_support.rs` を除く）を `(<crate>/<src の中のパス>, 本文)` で集める。分ける前に本体の
/// `src` が覆っていた範囲を歩く。
fn production_sources() -> Vec<(String, String)> {
    fn walk(krate: &str, root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|e| {
            panic!("木を歩けない（走査が空振りする）: {} — {e}", dir.display())
        });
        for entry in entries {
            let path = entry.expect("ディレクトリ項目が読めない").path();
            if path.is_dir() {
                walk(krate, root, &path, out);
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.ends_with(".rs")
                || name.ends_with("_tests.rs")
                || name.ends_with("_support.rs")
            {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .expect("走査の根の下に無い")
                .to_string_lossy()
                .replace('\\', "/");
            let src = std::fs::read_to_string(&path).expect("本番ファイルが読めない");
            out.push((format!("{krate}/{rel}"), src));
        }
    }
    let mut out = Vec::new();
    for (krate, root) in [
        ("areka", Path::new(env!("CARGO_MANIFEST_DIR")).join("src")),
        (
            "areka-runtime",
            crate::crate_source_test_support::crate_path("areka-runtime", "src"),
        ),
        (
            "areka-input",
            crate::crate_source_test_support::crate_path("areka-input", "src"),
        ),
    ] {
        let before = out.len();
        walk(krate, &root, &root, &mut out);
        assert!(
            out.len() > before,
            "{krate} の src に本番ファイルが 1 つも無い（走査が空振りしている）"
        );
    }
    out
}

/// 走査の当たり: 字面を含む行（註釈の行と関数の定義の行を除く）を `(相対パス, 行)` で集める。
fn deadline_call_sites(files: &[(String, String)]) -> BTreeSet<(String, String)> {
    files
        .iter()
        .flat_map(|(rel, src)| {
            src.lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("//") && !line.contains("fn "))
                .filter(|line| DEADLINE_CALL_TOKENS.iter().any(|t| line.contains(t)))
                .map(move |line| (rel.clone(), line.to_owned()))
        })
        .collect()
}

/// 当たりと許可の差（表に無い当たり・当たりの無い表の行）。
fn deadline_call_mismatch(
    found: &BTreeSet<(String, String)>,
) -> (Vec<(String, String)>, Vec<(String, String)>) {
    let allowed: BTreeSet<(String, String)> = ALLOWED_DEADLINE_CALLS
        .iter()
        .map(|(rel, line)| ((*rel).to_owned(), (*line).to_owned()))
        .collect();
    (
        found.difference(&allowed).cloned().collect(),
        allowed.difference(found).cloned().collect(),
    )
}

/// 本番ソースで期限つきの降ろしと見張りの張りを呼ぶのは、OS のセッションの終了の受け手と
/// `GhostSession` の委譲だけ（表に無い当たりも、当たりの無い表の行も赤）。
#[test]
fn only_session_end_calls_the_bounded_shutdown() {
    let found = deadline_call_sites(&production_sources());
    assert_eq!(
        deadline_call_mismatch(&found),
        (vec![], vec![]),
        "期限つきの降ろし・見張りの張りの呼び出しが許可表と合わない（表に無い当たり・当たりの無い表の行）: {found:?}"
    );
}

/// 走査の較正: 本番ソースの写しから許可の行を 1 行消すと「当たりの無い表の行」、見張りの張りを
/// 1 行足すと「表に無い当たり」で赤になる（検査が空振りしていない）。
#[test]
fn deadline_call_scan_turns_red_on_one_line_removed_or_added() {
    let files = production_sources();

    let removed: Vec<(String, String)> = files
        .iter()
        .map(|(rel, src)| {
            let src = if rel == "areka/session/session_end.rs" {
                src.replace("end_session_within(world, SESSION_END_SHIORI_LIMIT)", "")
            } else {
                src.clone()
            };
            (rel.clone(), src)
        })
        .collect();
    let (extra, missing) = deadline_call_mismatch(&deadline_call_sites(&removed));
    assert_eq!(
        (extra, missing),
        (
            vec![],
            vec![(
                "areka/session/session_end.rs".to_owned(),
                "end_session_within(world, SESSION_END_SHIORI_LIMIT)".to_owned()
            )]
        ),
        "許可の行を 1 行消しても赤にならない"
    );

    let mut added = files.clone();
    added.push((
        "areka/main.rs".to_owned(),
        "let guard = probe.arm(budget);".to_owned(),
    ));
    let (extra, missing) = deadline_call_mismatch(&deadline_call_sites(&added));
    assert_eq!(
        (extra, missing),
        (
            vec![(
                "areka/main.rs".to_owned(),
                "let guard = probe.arm(budget);".to_owned()
            )],
            vec![]
        ),
        "見張りの張りを 1 行足しても赤にならない"
    );
}
