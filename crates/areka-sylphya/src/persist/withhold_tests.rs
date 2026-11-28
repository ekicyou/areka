//! 控える範囲の旗と、範囲の中の型つきの包みのテスト。
//!
//! 旗だけの試験の根は実在しない架空のパスで足りる。型つきの包みを通す試験は木が変わらないことを
//! 見るので、`TempPath` の根を使う。プロセスに 1 つの状態を並んだ試験と分け合うため、根は試験ごとに
//! 別の名前にし、`withhold_everything` は呼ばない。

use super::*;
use crate::persist::{
    PersistKey, PersistOutcome, PersistScope, ScopeRoots, ensure_root_dir, load_scope, save_scope,
};
use crate::store::{FsBackend, StoreBackend};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use temp_path_kit::TempPath;

fn root(label: &str) -> PathBuf {
    PathBuf::from(format!(
        r"C:\withhold-test\areka-{label}-{}",
        std::process::id()
    ))
}

fn backend() -> Arc<dyn StoreBackend> {
    FsBackend::with_manual_clock()
}

#[test]
fn dropping_the_guard_forgets_registration() {
    let r = root("drop");
    let file = r.join("ghost").join("sylphya.toml");
    {
        let _guard = WithholdGuard::under(&r);
        assert!(withholds(&r), "根そのものは範囲の中");
        assert!(withholds(&file), "根の下は範囲の中");
    }
    assert!(!withholds(&file), "落とした後は範囲の外");
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
}

/// 根の下に在るものの並び（木の前後比べ用）。
fn tree(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            out.push(e.path());
            out.extend(tree(&e.path()));
        }
    }
    out.sort();
    out
}

#[test]
fn save_scope_inside_returns_withheld_and_reads_back() {
    let tmp = TempPath::under_target("sylphya-withhold-save");
    let roots = ScopeRoots {
        app: Some(tmp.child("areka")),
        ..ScopeRoots::default()
    };
    let _guard = WithholdGuard::under(tmp.path());
    let backend = backend();
    let entries = vec![(PersistKey::LastGhost, "emo2".to_owned())];
    let mut outcome = None;
    let events = crate::test_log_capture::capture(|| {
        outcome = Some(save_scope(PersistScope::App, &roots, &backend, entries))
    });
    assert_eq!(outcome.unwrap(), PersistOutcome::Withheld);
    assert!(tree(tmp.path()).is_empty(), "木は変わらない");
    assert!(
        events.iter().any(|e| e.level == tracing::Level::DEBUG
            && e.field_str("event") == Some("persist_withheld")),
        "persist_withheld の debug が出る: {events:?}"
    );
    let loaded = load_scope(PersistScope::App, &roots, &backend);
    assert_eq!(
        loaded,
        vec![(PersistKey::LastGhost, "emo2".to_owned())],
        "同じ走りの中では書けたときと同じ値を読み返せる"
    );
}

/// 読み取りだけの走りは使えない既存のファイルで止まらない（何も書かないので守る物が無い）。
#[test]
fn save_scope_inside_does_not_stop_on_an_unusable_existing_file() {
    let tmp = TempPath::under_target("sylphya-withhold-unusable");
    let root = tmp.child("areka");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("sylphya.toml");
    let newer = "format-version = 2\n[boot]\ncount = \"9\"\n";
    std::fs::write(&file, newer).unwrap();
    let roots = ScopeRoots {
        app: Some(root),
        ..ScopeRoots::default()
    };
    let _guard = WithholdGuard::under(tmp.path());
    let outcome = save_scope(
        PersistScope::App,
        &roots,
        &backend(),
        vec![(PersistKey::BootCount, "1".to_owned())],
    );
    assert_eq!(outcome, PersistOutcome::Withheld);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        newer,
        "ファイルはそのまま"
    );
}

#[test]
fn ensure_root_dir_skips_inside_and_creates_outside() {
    let inside = TempPath::under_target("sylphya-withhold-dir-in");
    let outside = TempPath::under_target("sylphya-withhold-dir-out");
    let _guard = WithholdGuard::under(inside.path());
    let backend = backend();
    ensure_root_dir(&inside.child("ghost"), &backend).unwrap();
    assert!(!inside.child("ghost").exists(), "範囲の中では作らない");
    ensure_root_dir(&outside.child("ghost"), &backend).unwrap();
    assert!(outside.child("ghost").is_dir(), "範囲の外では作る");
}
