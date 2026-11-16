//! `shell_box_assets.rs` の兄弟テスト（tasks.md 8.1・要件 3.10・4.1・5.1）。
//!
//! 面の表と箱の表はテストの中の surfaces.txt の文面から組む（検体は読まない）。別名の写しと
//! 在るサーフェス番号は本番と同じく `EmoWorld::alias_snapshot`・`EmoWorld::surface_ids` から採る。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use areka_emo_compose::{EmoWorld, fold_boxes};
use areka_emo_text::state::SurfaceKeyOutcome;
use areka_parsers::shell::{parse, parse_boxes};
use areka_seriko::SurfaceResolver;

use super::{box_font_search_dirs, resolve_for_text};

/// 箱のある `surface1000`。シェルに 9999 は無い。別名 `egao` は 1000 を、`nai` は 9999 を先頭に持つ。
const SHELL: &str = "\
balloon.fuda
{
size,100,50
}
surface1000
{
element1,balloon,fuda,0,0
}
kero.surface.alias
{
egao,[1000]
nai,[9999,1000]
}
";

/// 本番と同じ材料（別名の写しで組んだ seriko の解決器・面の表に在る番号）を文面から組む。
fn resolver_and_ids() -> (SurfaceResolver, BTreeSet<u32>) {
    let world = EmoWorld::build(&parse(SHELL));
    let (layout, report) = fold_boxes(&parse_boxes(SHELL), &BTreeMap::new(), &world);
    assert_eq!(report.issues, vec![], "文面は誤りを持たない");
    assert_eq!(layout.placements(1000).len(), 1, "surface1000 は箱を持つ");
    let ids: BTreeSet<u32> = world.surface_ids().collect();
    assert_eq!(ids, BTreeSet::from([1000]), "シェルに在るのは 1000 だけ");
    (SurfaceResolver::new(world.alias_snapshot()), ids)
}

#[test]
fn font_search_dirs_are_shell_then_ghost() {
    let dirs = box_font_search_dirs(
        Path::new("ghost/a/shell/master"),
        Path::new("ghost/a/ghost/master"),
    );
    assert_eq!(
        dirs,
        vec![
            PathBuf::from("ghost/a/shell/master"),
            PathBuf::from("ghost/a/ghost/master"),
        ]
    );
}

#[test]
fn surface_in_shell_is_show() {
    let (resolver, ids) = resolver_and_ids();
    assert_eq!(
        resolve_for_text(&resolver, &ids, "1000"),
        SurfaceKeyOutcome::Show(1000)
    );
}

#[test]
fn surface_missing_from_shell_is_unresolved() {
    let (resolver, ids) = resolver_and_ids();
    assert_eq!(
        resolve_for_text(&resolver, &ids, "9999"),
        SurfaceKeyOutcome::Unresolved
    );
}

#[test]
fn alias_follows_its_first_id() {
    let (resolver, ids) = resolver_and_ids();
    assert_eq!(
        resolve_for_text(&resolver, &ids, "egao"),
        SurfaceKeyOutcome::Show(1000)
    );
    // 先頭が 9999（シェルに無い）なら、後ろに 1000 があっても解決できない（seriko と同じ先頭固定）。
    assert_eq!(
        resolve_for_text(&resolver, &ids, "nai"),
        SurfaceKeyOutcome::Unresolved
    );
}

#[test]
fn minus_one_is_hide() {
    let (resolver, ids) = resolver_and_ids();
    assert_eq!(
        resolve_for_text(&resolver, &ids, "-1"),
        SurfaceKeyOutcome::Hide
    );
}

#[test]
fn unreadable_key_is_unresolved() {
    let (resolver, ids) = resolver_and_ids();
    for key in ["shiranai", "", "-2", "4294967296"] {
        assert_eq!(
            resolve_for_text(&resolver, &ids, key),
            SurfaceKeyOutcome::Unresolved,
            "{key:?}"
        );
    }
}
