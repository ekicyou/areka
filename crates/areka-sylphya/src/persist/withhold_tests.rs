//! 控える範囲と重ね書きの控えの檻（要件 2.1・2.3・2.4・10.5）。
//!
//! このモジュールはファイルシステムに触れないので、根は実在しない架空のパスで足りる
//! （本物の置き場は読みも書きもしない＝要件 10.5）。プロセスに 1 つの状態を並んだ試験と
//! 分け合うため、根は試験ごとに別の名前にし、`withhold_everything` は呼ばない。

use super::*;
use std::path::{Path, PathBuf};

fn root(label: &str) -> PathBuf {
    PathBuf::from(format!(
        r"C:\withhold-test\areka-{label}-{}",
        std::process::id()
    ))
}

#[test]
fn staged_content_reads_back_inside_the_root() {
    let r = root("readback");
    let file = r.join("sylphya.toml");
    let _guard = WithholdGuard::under(&r);
    assert!(withholds(&r), "根そのものは範囲の中");
    assert!(withholds(&file), "根の下は範囲の中");
    stage(&file, "a = 1");
    assert_eq!(staged(&file).as_deref(), Some("a = 1"));
    stage(&file, "a = 2");
    assert_eq!(
        staged(&file).as_deref(),
        Some("a = 2"),
        "最後に控えた中身を返す"
    );
}

#[test]
fn dropping_the_guard_forgets_registration_and_staged_content() {
    let r = root("drop");
    let file = r.join("ghost").join("sylphya.toml");
    {
        let _guard = WithholdGuard::under(&r);
        stage(&file, "x");
        assert_eq!(staged(&file).as_deref(), Some("x"));
    }
    assert!(!withholds(&file), "落とした後は範囲の外");
    assert_eq!(staged(&file), None, "落とすとその根の下の控えも消える");
}

#[test]
fn unregistered_sibling_root_is_outside() {
    let r = root("sibling-a");
    let other = root("sibling-b");
    let _guard = WithholdGuard::under(&r);
    assert!(!withholds(&other));
    assert!(!withholds(&other.join("sylphya.toml")));
}

#[test]
fn prefix_match_is_by_path_component() {
    let base = root("component");
    let one = PathBuf::from(format!("{}-1", base.display()));
    let ten = PathBuf::from(format!("{}-10", base.display()));
    let _guard = WithholdGuard::under(&one);
    assert!(withholds(&one.join("sylphya.toml")));
    assert!(!withholds(&ten), "`…-1` を登録しても `…-10` は範囲の外");
    assert!(!withholds(&ten.join("sylphya.toml")));
}

#[test]
fn key_ignores_case_and_separator() {
    let r = root("case");
    let _guard = WithholdGuard::under(&r);
    let upper = PathBuf::from(r.to_string_lossy().to_uppercase()).join("SYLPHYA.TOML");
    let slashed = Path::new(&r.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"))
        .join("sylphya.toml");
    assert!(withholds(&upper));
    assert!(withholds(&slashed));
    stage(&upper, "c");
    assert_eq!(
        staged(&slashed).as_deref(),
        Some("c"),
        "大小と区切りを問わず同じ鍵"
    );
}
