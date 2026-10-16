//! 起動時の並びの決め方と記憶の値の形の決定論テスト（要件 1.3〜1.14・6.8・9.2・9.4・12.1・12.7）。
//!
//! 入力はすべて値で注入する（プロセスの環境変数は触らない）。実在する根は
//! `TempPath::under_target`（ワークツリーの `target\` の下）に掘る。

use super::*;
use areka_ghost::{BasewareRoot, BasewareRoots};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use temp_path_kit::TempPath;

/// `target\` の下に実在するフォルダを 1 つ掘る。
fn real_dir(tmp: &TempPath, name: &str) -> PathBuf {
    let dir = tmp.path().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn env(value: &str) -> Option<OsString> {
    Some(OsString::from(value))
}

fn dirs(r: &ResolvedRoots) -> Vec<PathBuf> {
    r.roots
        .roots()
        .iter()
        .map(|x| x.dir().to_path_buf())
        .collect()
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
fn default_is_exe_dir_alone() {
    let tmp = TempPath::under_target("roots-default");
    let exe = real_dir(&tmp, "exe");
    let r = resolve_roots_from(None, None, Some(exe.clone())).unwrap();
    assert_eq!(dirs(&r), vec![exe.clone()]);
    assert_eq!(r.origin, RootsOrigin::Default);
    assert_eq!(
        r.status,
        vec![RootStatus {
            dir: exe,
            found: true
        }]
    );
    assert!(r.skipped.is_empty());
    assert!(!r.memory_fell_back);
}

#[test]
fn default_without_exe_location_fails() {
    assert_eq!(
        resolve_roots_from(None, None, None),
        Err(RootError::ExeLocationUnavailable {
            tried: vec![],
            origin: RootsOrigin::Default,
            skipped: vec![],
            memory_fell_back: false,
        })
    );
}

#[test]
fn memory_keeps_its_order() {
    let tmp = TempPath::under_target("roots-memory-order");
    let exe = real_dir(&tmp, "exe");
    let a = real_dir(&tmp, "a");
    let b = real_dir(&tmp, "b");
    let value = format!("{}|{EXE_TOKEN}|{}", s(&b), s(&a));
    let r = resolve_roots_from(None, Some(value), Some(exe.clone())).unwrap();
    assert_eq!(dirs(&r), vec![b, exe, a]);
    assert_eq!(r.origin, RootsOrigin::Memory);
    assert!(r.skipped.is_empty());
    assert!(!r.memory_fell_back);
}

#[test]
fn env_single_ignores_memory() {
    let tmp = TempPath::under_target("roots-env-single");
    let exe = real_dir(&tmp, "exe");
    let a = real_dir(&tmp, "a");
    let b = real_dir(&tmp, "b");
    let r = resolve_roots_from(env(&s(&a)), Some(s(&b)), Some(exe)).unwrap();
    assert_eq!(dirs(&r), vec![a]);
    assert_eq!(r.origin, RootsOrigin::EnvVar);
}

#[test]
fn env_multiple_keeps_order_and_ignores_memory() {
    let tmp = TempPath::under_target("roots-env-multi");
    let exe = real_dir(&tmp, "exe");
    let a = real_dir(&tmp, "a");
    let b = real_dir(&tmp, "b");
    let r = resolve_roots_from(
        env(&format!("{};{}", s(&b), s(&a))),
        Some(EXE_TOKEN.to_string()),
        Some(exe),
    )
    .unwrap();
    assert_eq!(dirs(&r), vec![b, a]);
    assert_eq!(r.origin, RootsOrigin::EnvVar);
    assert!(r.skipped.is_empty());
}

#[test]
fn env_empty_items_are_skipped_and_noted() {
    let tmp = TempPath::under_target("roots-env-empty");
    let a = real_dir(&tmp, "a");
    let b = real_dir(&tmp, "b");
    let r = resolve_roots_from(env(&format!("{};;{}", s(&a), s(&b))), None, None).unwrap();
    assert_eq!(dirs(&r), vec![a.clone(), b]);
    assert_eq!(r.skipped, vec![RootSkip::Empty { index: 1 }]);

    let r = resolve_roots_from(env(&format!("{};", s(&a))), None, None).unwrap();
    assert_eq!(dirs(&r), vec![a]);
    assert_eq!(r.skipped, vec![RootSkip::Empty { index: 1 }]);
}

#[test]
fn env_all_empty_fails() {
    for (value, count) in [("", 1), (";", 2), (";;", 3)] {
        assert_eq!(
            resolve_roots_from(env(value), Some("x".into()), None),
            Err(RootError::EnvAllEmpty {
                skipped: (0..count).map(|index| RootSkip::Empty { index }).collect(),
                memory_fell_back: false,
            }),
            "{value:?}"
        );
    }
}

#[test]
fn duplicates_keep_the_upper_one() {
    let tmp = TempPath::under_target("roots-dup");
    let a = real_dir(&tmp, "a");
    let lower = PathBuf::from(s(&a).to_lowercase());
    let trailing = PathBuf::from(format!("{}\\", s(&a)));
    let r = resolve_roots_from(
        env(&format!("{};{};{}", s(&a), s(&lower), s(&trailing))),
        None,
        None,
    )
    .unwrap();
    assert_eq!(dirs(&r), vec![a.clone()]);
    assert_eq!(
        r.skipped,
        vec![
            RootSkip::Duplicate {
                dir: lower,
                kept: a.clone()
            },
            RootSkip::Duplicate {
                dir: trailing,
                kept: a
            },
        ]
    );
}

#[test]
fn relative_env_item_is_absolutized_against_cwd() {
    let tmp = TempPath::under_target("roots-relative");
    let a = real_dir(&tmp, "a");
    let rel = "areka-roots-relative-item-that-does-not-exist";
    let r = resolve_roots_from(env(&format!("{};{rel}", s(&a))), None, None).unwrap();
    let expected = std::path::absolute(rel).unwrap();
    assert!(expected.is_absolute());
    assert_eq!(dirs(&r), vec![a.clone(), expected.clone()]);
    // 見つからない根も並びに残る（要件 9.2）。
    assert_eq!(
        r.status,
        vec![
            RootStatus {
                dir: a,
                found: true
            },
            RootStatus {
                dir: expected,
                found: false
            },
        ]
    );
}

#[test]
fn memory_empty_string_falls_back_to_default() {
    let tmp = TempPath::under_target("roots-memory-empty");
    let exe = real_dir(&tmp, "exe");
    let r = resolve_roots_from(None, Some(String::new()), Some(exe.clone())).unwrap();
    assert_eq!(dirs(&r), vec![exe]);
    assert_eq!(r.origin, RootsOrigin::Default);
    assert!(r.memory_fell_back);
    assert!(r.skipped.is_empty());
}

#[test]
fn memory_unreadable_items_are_skipped_and_noted() {
    let tmp = TempPath::under_target("roots-memory-unreadable");
    let exe = real_dir(&tmp, "exe");
    let a = real_dir(&tmp, "a");
    let value = format!("|relative\\dir|C:\\bad*name|{}", s(&a));
    let r = resolve_roots_from(None, Some(value), Some(exe)).unwrap();
    assert_eq!(dirs(&r), vec![a]);
    assert_eq!(r.origin, RootsOrigin::Memory);
    assert!(!r.memory_fell_back);
    assert_eq!(
        r.skipped,
        vec![
            RootSkip::Unreadable {
                item: String::new(),
                reason: "empty"
            },
            RootSkip::Unreadable {
                item: "relative\\dir".into(),
                reason: "relative"
            },
            RootSkip::Unreadable {
                item: "C:\\bad*name".into(),
                reason: "invalid"
            },
        ]
    );
}

#[test]
fn memory_with_no_readable_item_falls_back_with_marker() {
    let tmp = TempPath::under_target("roots-memory-zero");
    let exe = real_dir(&tmp, "exe");
    let r = resolve_roots_from(None, Some("rel|".into()), Some(exe.clone())).unwrap();
    assert_eq!(dirs(&r), vec![exe]);
    assert_eq!(r.origin, RootsOrigin::Default);
    assert!(r.memory_fell_back);
    assert_eq!(r.skipped.len(), 2);
}

#[test]
fn memory_exe_token_without_exe_location_falls_back_and_fails() {
    assert_eq!(
        resolve_roots_from(None, Some(format!("|{EXE_TOKEN}")), None),
        // 失敗しても控えと「既定へ倒した」は残る（記録の warn のため）。
        Err(RootError::ExeLocationUnavailable {
            tried: vec![],
            origin: RootsOrigin::Default,
            skipped: vec![
                RootSkip::Unreadable {
                    item: String::new(),
                    reason: "empty"
                },
                RootSkip::Unreadable {
                    item: EXE_TOKEN.into(),
                    reason: "exe_unavailable"
                },
            ],
            memory_fell_back: true,
        })
    );
    let (items, skipped) = decode_memory(EXE_TOKEN, None);
    assert!(items.is_empty());
    assert_eq!(
        skipped,
        vec![RootSkip::Unreadable {
            item: EXE_TOKEN.into(),
            reason: "exe_unavailable"
        }]
    );
}

#[test]
fn exe_token_follows_the_moved_exe() {
    let tmp = TempPath::under_target("roots-exe-moved");
    let before = real_dir(&tmp, "before");
    let after = real_dir(&tmp, "after");
    let b = real_dir(&tmp, "b");
    let value = format!("{EXE_TOKEN}|{}", s(&b));
    let r = resolve_roots_from(None, Some(value.clone()), Some(before.clone())).unwrap();
    assert_eq!(dirs(&r), vec![before, b.clone()]);
    let r = resolve_roots_from(None, Some(value), Some(after.clone())).unwrap();
    assert_eq!(dirs(&r), vec![after, b]);
}

#[test]
fn no_root_found_fails_with_all_tried() {
    let tmp = TempPath::under_target("roots-none-found");
    let x = tmp.path().join("missing-x");
    let y = tmp.path().join("missing-y");
    assert_eq!(
        resolve_roots_from(env(&format!("{};;{};{}", s(&x), s(&y), s(&x))), None, None),
        // 失敗しても空の項目と重複の控えは残る（記録の warn のため）。
        Err(RootError::NoneFound {
            tried: vec![x.clone(), y],
            origin: RootsOrigin::EnvVar,
            skipped: vec![
                RootSkip::Empty { index: 1 },
                RootSkip::Duplicate {
                    dir: x.clone(),
                    kept: x
                },
            ],
            memory_fell_back: false,
        })
    );
    let exe = tmp.path().join("missing-exe");
    assert_eq!(
        resolve_roots_from(None, None, Some(exe.clone())),
        Err(RootError::NoneFound {
            tried: vec![exe],
            origin: RootsOrigin::Default,
            skipped: vec![],
            memory_fell_back: false,
        })
    );
}

#[test]
fn encode_and_decode_round_trip() {
    let tmp = TempPath::under_target("roots-round-trip");
    let exe = real_dir(&tmp, "exe");
    let b = real_dir(&tmp, "b");
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(b.clone()),
        // 実行ファイルのフォルダは末尾の区切りがあっても同じ根として `<exe>` で書く（要件 6.8）。
        BasewareRoot::new(PathBuf::from(format!("{}\\", s(&exe)))),
    ]);
    let value = encode_memory(&roots, Some(&exe));
    assert_eq!(value, format!("{}|{EXE_TOKEN}", s(&b)));
    let (items, skipped) = decode_memory(&value, Some(&exe));
    assert_eq!(items, vec![b.clone(), exe]);
    assert!(skipped.is_empty());
    // 実行ファイルの場所が取れないときは `<exe>` に読み替えず絶対パスで書く。
    assert_eq!(
        encode_memory(&BasewareRoots::single(b.clone()), None),
        s(&b)
    );
}
