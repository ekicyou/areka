//! 控える範囲と重ね書きの控えの檻（要件 2.1・2.3・2.4・10.5）。
//!
//! このモジュールはファイルシステムに触れないので、表だけの試験の根は実在しない架空のパスで
//! 足りる（本物の置き場は読みも書きもしない＝要件 10.5）。`FsPersistIo` と `save_scope` を通す
//! 試験（要件 2.1・2.5・2.6）は木が変わらないことを見るので、`TempPath` の根を使う。
//! プロセスに 1 つの状態を並んだ試験と分け合うため、根は試験ごとに別の名前にし、
//! `withhold_everything` は呼ばない。

use super::*;
use crate::persist::{
    FsPersistIo, PersistIo, PersistKey, PersistOutcome, PersistScope, ScopeRoots, save_scope,
};
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

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
fn fs_commit_inside_creates_nothing_and_read_returns_staged() {
    let tmp = TempPath::under_target("sylphya-withhold-commit");
    let file = tmp.child("ghost").join("areka").join("sylphya.toml");
    let _guard = WithholdGuard::under(tmp.path());
    let io = FsPersistIo;
    assert!(io.withholds(&file));
    io.commit(&file, "a = 1").expect("控えへの入れは失敗しない");
    assert!(
        tree(tmp.path()).is_empty(),
        "親のフォルダも一時ファイルも宛先も作らない"
    );
    assert_eq!(
        io.read(&file).unwrap().as_deref(),
        Some("a = 1"),
        "控えを先に見る"
    );
}

#[test]
fn fs_read_falls_back_to_file_when_nothing_staged() {
    let tmp = TempPath::under_target("sylphya-withhold-read");
    let file = tmp.child("sylphya.toml");
    std::fs::write(&file, "b = 2").unwrap();
    let _guard = WithholdGuard::under(tmp.path());
    assert_eq!(FsPersistIo.read(&file).unwrap().as_deref(), Some("b = 2"));
}

#[test]
fn save_scope_inside_returns_withheld_and_logs_it() {
    let tmp = TempPath::under_target("sylphya-withhold-save");
    let roots = ScopeRoots {
        app: Some(tmp.child("areka")),
        ..ScopeRoots::default()
    };
    let _guard = WithholdGuard::under(tmp.path());
    let entries = vec![(PersistKey::LastGhost, "emo2".to_owned())];
    let (outcome, events) =
        log_capture_kit::capture(|| save_scope(PersistScope::App, &roots, &FsPersistIo, entries));
    assert_eq!(outcome, PersistOutcome::Withheld);
    assert!(tree(tmp.path()).is_empty(), "木は変わらない");
    assert!(
        events.iter().any(|e| e.level == tracing::Level::DEBUG
            && e.field_str("event") == Some("persist_withheld")),
        "persist_withheld の debug が出る: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|e| e.message().contains("persist commit failed")),
        "保存の失敗の記録は出ない: {events:?}"
    );
    // 同じ走りの中では書けたときと同じ値を読み返せる（要件 2.3）。
    let loaded = crate::persist::load_scope(PersistScope::App, &roots, &FsPersistIo);
    assert_eq!(loaded, vec![(PersistKey::LastGhost, "emo2".to_owned())]);
}

#[test]
fn ensure_root_dir_skips_inside_and_creates_outside() {
    let inside = TempPath::under_target("sylphya-withhold-dir-in");
    let outside = TempPath::under_target("sylphya-withhold-dir-out");
    let _guard = WithholdGuard::under(inside.path());
    FsPersistIo::ensure_root_dir(&inside.child("ghost")).unwrap();
    assert!(!inside.child("ghost").exists(), "範囲の中では作らない");
    FsPersistIo::ensure_root_dir(&outside.child("ghost")).unwrap();
    assert!(outside.child("ghost").is_dir(), "範囲の外では作る");
}
