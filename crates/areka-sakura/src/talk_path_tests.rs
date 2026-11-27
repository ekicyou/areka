//! `resolve_in` の決まりを、在るパスの集合を述語にして fs を読まずに確かめる。

use super::*;
use std::collections::HashSet;

const MASTER: &str = r"C:\ghost\x\ghost\master";
const ROOT: &str = r"C:\ghost\x";

fn place() -> GhostPlace {
    GhostPlace {
        master: PathBuf::from(MASTER),
        root: PathBuf::from(ROOT),
    }
}

fn solve(
    place: Option<&GhostPlace>,
    written: &str,
    order: SearchOrder,
    present: &[PathBuf],
) -> Result<PathBuf, Unresolved> {
    let present: HashSet<&Path> = present.iter().map(PathBuf::as_path).collect();
    resolve_in(place, written, order, |p| present.contains(p))
}

fn in_master(written: &str) -> PathBuf {
    Path::new(MASTER).join(written)
}

fn in_root(written: &str) -> PathBuf {
    Path::new(ROOT).join(written)
}

#[test]
fn absolute_path_is_taken_as_written() {
    let abs = r"D:\docs\readme.txt";
    let present = [PathBuf::from(abs)];
    for order in [SearchOrder::MasterOnly, SearchOrder::MasterThenRoot] {
        assert_eq!(
            solve(Some(&place()), abs, order, &present),
            Ok(PathBuf::from(abs))
        );
        assert_eq!(
            solve(Some(&place()), abs, order, &[]),
            Err(Unresolved::NotFound {
                last_tried: PathBuf::from(abs)
            })
        );
    }
}

#[test]
fn master_only_finds_in_master() {
    let present = [in_master("a.txt")];
    assert_eq!(
        solve(Some(&place()), "a.txt", SearchOrder::MasterOnly, &present),
        Ok(in_master("a.txt"))
    );
}

#[test]
fn master_only_does_not_look_at_root() {
    let present = [in_root("a.txt")];
    assert_eq!(
        solve(Some(&place()), "a.txt", SearchOrder::MasterOnly, &present),
        Err(Unresolved::NotFound {
            last_tried: in_master("a.txt")
        })
    );
}

#[test]
fn master_then_root_prefers_master() {
    let present = [in_master("a.txt"), in_root("a.txt")];
    assert_eq!(
        solve(
            Some(&place()),
            "a.txt",
            SearchOrder::MasterThenRoot,
            &present
        ),
        Ok(in_master("a.txt"))
    );
}

#[test]
fn master_then_root_falls_back_to_root() {
    let present = [in_root("a.txt")];
    assert_eq!(
        solve(
            Some(&place()),
            "a.txt",
            SearchOrder::MasterThenRoot,
            &present
        ),
        Ok(in_root("a.txt"))
    );
}

#[test]
fn master_then_root_reports_root_as_last_tried() {
    assert_eq!(
        solve(Some(&place()), "a.txt", SearchOrder::MasterThenRoot, &[]),
        Err(Unresolved::NotFound {
            last_tried: in_root("a.txt")
        })
    );
}

#[test]
fn parent_steps_are_joined_without_normalizing() {
    let written = r"..\..\shell\master\surface0.png";
    let joined = in_master(written);
    assert!(joined.to_string_lossy().contains(r"master\..\..\shell"));
    assert_eq!(
        solve(
            Some(&place()),
            written,
            SearchOrder::MasterOnly,
            std::slice::from_ref(&joined)
        ),
        Ok(joined)
    );
    let normalized = PathBuf::from(r"C:\ghost\x\shell\master\surface0.png");
    assert_eq!(
        solve(
            Some(&place()),
            written,
            SearchOrder::MasterOnly,
            &[normalized]
        ),
        Err(Unresolved::NotFound {
            last_tried: in_master(written)
        })
    );
}

#[test]
fn relative_path_without_place_is_no_place() {
    let present = [in_master("a.txt"), in_root("a.txt")];
    for order in [SearchOrder::MasterOnly, SearchOrder::MasterThenRoot] {
        assert_eq!(
            solve(None, "a.txt", order, &present),
            Err(Unresolved::NoPlace)
        );
    }
}

#[test]
fn absolute_path_without_place_resolves() {
    let abs = r"D:\docs\readme.txt";
    assert_eq!(
        solve(None, abs, SearchOrder::MasterOnly, &[PathBuf::from(abs)]),
        Ok(PathBuf::from(abs))
    );
}

#[test]
fn empty_path_is_empty_before_anything_else() {
    let present = [PathBuf::from(MASTER), PathBuf::from(ROOT)];
    for order in [SearchOrder::MasterOnly, SearchOrder::MasterThenRoot] {
        assert_eq!(
            solve(Some(&place()), "", order, &present),
            Err(Unresolved::Empty)
        );
        assert_eq!(solve(None, "", order, &present), Err(Unresolved::Empty));
    }
}

#[test]
fn reason_words() {
    assert_eq!(Unresolved::Empty.reason(), "empty_path");
    assert_eq!(Unresolved::NoPlace.reason(), "no_place");
    assert_eq!(
        Unresolved::NotFound {
            last_tried: PathBuf::new()
        }
        .reason(),
        "not_found"
    );
}

#[test]
fn resolve_emits_no_log_records() {
    let missing = r"C:\areka-talk-path-tests-missing\nothing.txt";
    let (results, events) = log_capture_kit::capture(|| {
        [
            resolve(Some(&place()), "", SearchOrder::MasterOnly),
            resolve(None, "a.txt", SearchOrder::MasterOnly),
            resolve(Some(&place()), "a.txt", SearchOrder::MasterThenRoot),
            resolve(None, missing, SearchOrder::MasterOnly),
        ]
    });
    assert_eq!(results[0], Err(Unresolved::Empty));
    assert_eq!(results[1], Err(Unresolved::NoPlace));
    assert!(events.is_empty(), "記録は呼び手が残す: {events:?}");
}
