//! ゴーストの中の `balloon\` の走査（areka-P0-ghost-inner-balloon 要件 1.1〜1.3・10.2・10.3）。
//!
//! 一時フォルダは `temp-path-kit` の `under_target` の下だけに組む（要件 10.2）。

use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;
use tracing::Level;

use crate::catalog::scan_ghost_balloons;
use crate::test_log_capture::{CapturedEvent, capture};

const TARGET: &str = "areka_ghost::catalog";

/// `path` に `body` を書く（親フォルダも作る）。
fn put(path: &Path, body: &str) {
    std::fs::create_dir_all(path.parent().expect("親")).unwrap();
    std::fs::write(path, body).unwrap();
}

/// 一時フォルダの下にゴーストのフォルダを作って返す。
fn ghost(tmp: &TempPath) -> PathBuf {
    let g = tmp.child("ghost_a");
    put(&g.join(r"ghost\master\descript.txt"), "name,G\r\n");
    g
}

fn warns(events: &[CapturedEvent]) -> Vec<&str> {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::WARN)
        .map(|e| e.event.as_deref().unwrap_or("<event 無し>"))
        .collect()
}

/// `balloon\` の直下の `type` 無し・`type,balloon` は数え、`root` はゴーストのフォルダ・`dir` は
/// `<ゴースト>\balloon\<名>`。ゴーストの直下・`balloon\` そのもの・さらに下は数えない。
/// `type,shell` は除いて `catalog_type_not_balloon` を 1 件だけ残す（要件 1.1〜1.3）。
#[test]
fn ghost_balloon_dir_counts_direct_children_by_root_rules() {
    let tmp = TempPath::under_target("ghost-inner-balloon-scan");
    let g = ghost(&tmp);
    put(&g.join(r"balloon\plain\descript.txt"), "name,Plain\r\n");
    put(
        &g.join(r"balloon\typed\descript.txt"),
        "name,Typed\r\ntype,balloon\r\n",
    );
    put(
        &g.join(r"balloon\shell_like\descript.txt"),
        "type,shell\r\n",
    );
    // 数えない場所: ゴーストの直下・`balloon\` そのもの・`balloon\<名>\<下>\`。
    put(&g.join(r"direct\descript.txt"), "type,balloon\r\n");
    put(&g.join(r"balloon\descript.txt"), "type,balloon\r\n");
    put(
        &g.join(r"balloon\outer\inner\descript.txt"),
        "type,balloon\r\n",
    );

    let mut listed = Vec::new();
    let events = capture(|| listed = scan_ghost_balloons(&g));

    let folders: Vec<&str> = listed.iter().map(|b| b.identity.folder.as_str()).collect();
    assert_eq!(folders, ["plain", "typed"]);
    for b in &listed {
        assert_eq!(b.root, g);
        assert_eq!(b.dir, g.join("balloon").join(&b.identity.folder));
    }
    assert_eq!(listed[0].identity.name.as_deref(), Some("Plain"));
    assert_eq!(warns(&events), ["catalog_type_not_balloon"]);
}

/// `balloon\` が無ければ 0 件で記録も 0 件（要件 1.1・10.3）。
#[test]
fn ghost_without_balloon_dir_lists_zero_silently() {
    let tmp = TempPath::under_target("ghost-inner-balloon-none");
    let g = ghost(&tmp);

    let mut listed = vec![];
    let events = capture(|| listed = scan_ghost_balloons(&g));

    assert!(listed.is_empty());
    assert!(events.iter().all(|e| e.target != TARGET));
}
