//! OS の「開く」関数を綴る所を、本体の `src` ほかの木を歩いて固める構造の試験。

use std::path::Path;

// -------------------------------------------------------------------------
// 見張り
// -------------------------------------------------------------------------

/// 本体（`crates/areka/src`）・`crates/areka-runtime/src`・`crates/areka-input/src`・`crates/areka-session/src` の 4 つの木の下で OS の「開く」関数
/// （Shell と Execute を繋いだ名）を綴るのは OS の境界のファイルだけ（分ける前に本体の `src` が
/// 覆っていた範囲を歩く）。
///
/// 針は連結で組み、このファイルには繋いだ綴りを書かない（見張り自身を数えないように）。
#[test]
fn only_the_os_port_spells_shell_execute() {
    let needle = ["Shell", "Execute"].concat();
    let roots = [
        ("areka", Path::new(env!("CARGO_MANIFEST_DIR")).join("src")),
        (
            "areka-runtime",
            areka_session::crate_source_test_support::crate_path("areka-runtime", "src"),
        ),
        (
            "areka-input",
            areka_session::crate_source_test_support::crate_path("areka-input", "src"),
        ),
        (
            "areka-session",
            areka_session::crate_source_test_support::crate_path("areka-session", "src"),
        ),
    ];
    let mut hits = Vec::new();
    for (krate, src) in roots {
        let mut stack = vec![src.clone()];
        let mut read = 0;
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("src を読める") {
                let path = entry.expect("項目を読める").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    read += 1;
                    if std::fs::read_to_string(&path)
                        .expect("ソースを読める")
                        .contains(&needle)
                    {
                        let rel = path.strip_prefix(&src).expect("src の下");
                        hits.push(format!(
                            "{krate}/{}",
                            rel.to_string_lossy().replace('\\', "/")
                        ));
                    }
                }
            }
        }
        assert!(
            read > 0,
            "{krate} の src を 1 つも読んでいない（走査が空振りしている）"
        );
    }
    assert_eq!(
        hits,
        vec!["areka-session/readme/os_port.rs".to_owned()],
        "綴るのは areka-session の readme/os_port.rs だけ"
    );
}
