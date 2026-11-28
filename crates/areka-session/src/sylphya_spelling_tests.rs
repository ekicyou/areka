//! 記憶のファイルの名前と `rusqlite` の綴りが核の口の中だけにあることを、ソースを読んで固定する試験。
//!
//! 本番のソース＝`crates/` の下の `.rs` から、`*_tests.rs`・`*_test.rs`・`test_support` を含む名前・`tests/` の下を除き、
//! `#[cfg(test)]` の付いた項目と、`//` で始まる行（`///`・`//!` を含む）を読まない。
//! 消した口の名前は、試験のファイルもコメントも含めて生の本文で探す。

use std::path::{Path, PathBuf};

const NAMES: [&str; 2] = ["sylphya.toml", "sylphya.sqlite"];

/// 名前を綴ってよい本番のファイル（`crates/` からの道・前方一致）。
const NAME_OWNERS: [&str; 3] = [
    "areka-sylphya/src/store/",
    "areka-sylphya/src/persist/format.rs",
    "areka-session/src/alert.rs",
];

const SQLITE_OWNERS: [&str; 1] = ["areka-sylphya/src/store/"];

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("項目を読めない").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn rel(path: &Path) -> String {
    path.strip_prefix(crates_dir())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_owned()
}

fn is_test_file(rel: &str) -> bool {
    rel.ends_with("_tests.rs")
        || rel.ends_with("_test.rs")
        || rel.contains("test_support")
        || rel.contains("/tests/")
}

/// コメントの行と `#[cfg(test)]` の付いた項目を落とした本文。
/// 項目は `{` を開いたなら深さが 0 へ戻る行まで、開かないなら `;` か `,` で終わる行まで
/// （欄・列挙子・腕も項目に数える）。囲む塊の `}` で深さが負になったら、その行から読む。
/// 読み飛ばしの状態は `(深さ, '{' を開いたか)`。
// shortcut: 文字列やコメントの中の `{` `}` も数える、試験の項目の中にそれが片寄って現れたら字句を読む形へ上げる。
fn code_lines(src: &str) -> String {
    const ATTR: &str = "#[cfg(test)]";
    let mut out = String::new();
    let mut skip: Option<(i32, bool)> = None;
    for line in src.lines() {
        let t = line.trim_start();
        let rest = match skip {
            Some(_) => t,
            None if t.starts_with(ATTR) => &t[ATTR.len()..],
            None => {
                if !t.starts_with("//") {
                    out.push_str(line);
                    out.push('\n');
                }
                continue;
            }
        };
        let (mut depth, mut opened) = skip.unwrap_or((0, false));
        for c in rest.chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if depth < 0 {
            out.push_str(line);
            out.push('\n');
            skip = None;
            continue;
        }
        let ended = if opened {
            depth == 0
        } else {
            rest.trim_end().ends_with([';', ','])
        };
        skip = (!ended).then_some((depth, opened));
    }
    out
}

/// 本番のソースの `(道, 本文)`。
fn production_sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    walk(&crates_dir(), &mut files);
    let mut out: Vec<(String, String)> = files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| (rel(p), p))
        .filter(|(r, _)| !r.ends_with("sylphya_spelling_tests.rs"))
        .filter(|(r, _)| !is_test_file(r))
        .map(|(r, p)| {
            let src = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{r}: {e}"));
            (r, code_lines(&src))
        })
        .collect();
    out.sort();
    out
}

fn offenders(needles: &[&str], owners: &[&str]) -> Vec<String> {
    let all = production_sources();
    assert!(all.len() > 100, "ソースを読めていない: {} 本", all.len());
    all.into_iter()
        .filter(|(r, _)| !owners.iter().any(|o| r.starts_with(o)))
        .filter(|(_, src)| needles.iter().any(|n| src.contains(n)))
        .map(|(r, _)| r)
        .collect()
}

#[test]
fn code_lines_drops_only_the_items_marked_for_tests() {
    let src = "a\n    #[cfg(test)] mod x;\nb\n#[cfg(test)]\nmod y {\n    fn f() {}\n}\nc\n  #[cfg(test)]\n  #[path = \"z.rs\"]\n  mod z;\n// d\nd\n";
    assert_eq!(code_lines(src), "a\nb\nc\nd\n");
    let src =
        "struct S {\n    #[cfg(test)]\n    a: u8,\n    b: u8,\n}\n#[cfg(not(test))]\nfn g() {}\n";
    assert_eq!(
        code_lines(src),
        "struct S {\n    b: u8,\n}\n#[cfg(not(test))]\nfn g() {}\n"
    );
    let src = "S {\n    #[cfg(test)]\n    a: 1,\n    b: 2,\n};\nenum E {\n    #[cfg(test)]\n    A,\n    B,\n}\nmatch x {\n    #[cfg(test)]\n    A => f(),\n    #[cfg(test)]\n    C => {\n        g();\n    }\n    _ => h(),\n}\nstruct T {\n    #[cfg(test)]\n    a: u8\n}\nfn k() {}\n";
    assert_eq!(
        code_lines(src),
        "S {\n    b: 2,\n};\nenum E {\n    B,\n}\nmatch x {\n    _ => h(),\n}\nstruct T {\n}\nfn k() {}\n"
    );
}

#[test]
fn memory_file_names_are_spelled_only_by_the_store_the_toml_reading_and_notices() {
    let bad = offenders(&NAMES, &NAME_OWNERS);
    assert!(
        bad.is_empty(),
        "記憶のファイルの名前を口の外で綴っている: {bad:?}"
    );
}

#[test]
fn rusqlite_is_spelled_only_by_the_store() {
    let bad = offenders(&["rusqlite"], &SQLITE_OWNERS);
    assert!(bad.is_empty(), "rusqlite を核の外で綴っている: {bad:?}");
}

/// 識別子の一部（`runtime_sinks` など）には当たらないよう、前後が識別子の文字でない現れだけを数える。
fn spells_word(src: &str, word: &str) -> bool {
    src.match_indices(word).any(|(i, _)| {
        let ident = |c: char| c.is_alphanumeric() || c == '_';
        !src[..i].chars().next_back().is_some_and(ident)
            && !src[i + word.len()..].chars().next().is_some_and(ident)
    })
}

#[test]
fn removed_injection_port_and_decision_struct_are_nowhere() {
    let removed = ["RuntimeCommandSink", "runtime_sink", "SylphyaCore"];
    let mut files = Vec::new();
    walk(&crates_dir(), &mut files);
    files.retain(|p| p.extension().is_some_and(|e| e == "rs"));
    assert!(
        files.len() > 100,
        "ソースを読めていない: {} 本",
        files.len()
    );
    let bad: Vec<String> = files
        .iter()
        .map(|p| (rel(p), p))
        .filter(|(r, _)| !r.ends_with("sylphya_spelling_tests.rs"))
        .filter(|(r, p)| {
            let src = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{r}: {e}"));
            removed.iter().any(|w| spells_word(&src, w))
        })
        .map(|(r, _)| r)
        .collect();
    assert!(bad.is_empty(), "消した口の名前が残っている: {bad:?}");
}
