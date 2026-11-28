//! pattern定義の `base` が段の静的な層を置き換える計画の値と、計画の土台の面の欄の檻。

use std::path::Path;

use areka_emo_atlas::AtlasTable;
use areka_parsers::shell::parse;

use crate::bind::BindSet;
use crate::method::ComposeMethod;
use crate::pattern::{PatternFrame, PatternState};
use crate::plan_test_support::{
    bake_atlas, bound_world, frame, id_to_path, named_ops, op_images, ops_and_extent, plan_images,
    plan_of,
};
use crate::world::EmoWorld;

pub(crate) const RELS: &[&str] = &["a.png", "b.png", "c.png", "d.png", "e.png"];

/// 面 0: 層 (i) の絵 2 枚・今のコマで動く 10・着せ替え 20（pattern0 が `base`→面 2・pattern1 が overlay）。
/// 面 7: `always` の `base`→面 1。面 8: 面 9 を (3,4) に置く。面 9: `always` の `base`→面 1。
/// 面 11: `always` の `base`→`-1`。面 12: 着せ替え 21 の pattern0 が `base`→`-1`。
pub(crate) const SHELL: &str = "charset,UTF-8\n\
    surface0\n{\n\
    element0,overlay,a.png,0,0\n\
    element1,overlay,b.png,5,6\n\
    animation10.interval,random,2\n\
    animation10.pattern0,base,1,100,7,3\n\
    animation20.interval,bind\n\
    animation20.pattern0,base,2,0,9,9\n\
    animation20.pattern1,overlay,3,0,4,4\n\
    }\n\
    surface1\n{\nelement0,overlay,c.png,0,0\n}\n\
    surface2\n{\nelement0,overlay,d.png,0,0\n}\n\
    surface3\n{\nelement0,overlay,e.png,0,0\n}\n\
    surface7\n{\n\
    element0,overlay,a.png,0,0\n\
    animation40.interval,always\n\
    animation40.pattern0,base,1,0,5,5\n\
    }\n\
    surface8\n{\nelement0,overlay,9,3,4\n}\n\
    surface9\n{\n\
    element0,overlay,b.png,0,0\n\
    animation50.interval,always\n\
    animation50.pattern0,base,1,0,6,6\n\
    }\n\
    surface11\n{\n\
    element0,overlay,a.png,0,0\n\
    animation60.interval,always\n\
    animation60.pattern0,base,-1,0,0,0\n\
    }\n\
    surface12\n{\n\
    element0,overlay,a.png,0,0\n\
    animation21.interval,bind\n\
    animation21.pattern0,base,-1,0,0,0\n\
    }\n";

pub(crate) fn world() -> (EmoWorld, AtlasTable) {
    let atlas = bake_atlas(Path::new("shell/master"), RELS);
    (bound_world(&parse(SHELL), &[], &atlas), atlas)
}

pub(crate) fn base_frame(surface_id: u32, x: i64, y: i64) -> PatternFrame {
    PatternFrame {
        method: ComposeMethod::Base,
        ..frame(surface_id, x, y)
    }
}

/// 面 `top` の CPU の命令の列を (絵, X, Y) で返し、計画の絵の項が同じ順・位置で、土台の面の欄が
/// [`EmoWorld::current_base_surface`] と同じことを確かめて、その土台の面も返す。
fn planned(
    world: &EmoWorld,
    atlas: &AtlasTable,
    top: u32,
    binds: &BindSet,
    pattern: &PatternState,
) -> (Vec<(&'static str, i64, i64)>, u32) {
    let (ops, _) = ops_and_extent(world, atlas, top, binds, pattern);
    let plan = plan_of(world, atlas, top, binds, pattern).expect("計画を組める");
    assert_eq!(plan_images(&plan), op_images(&ops), "計画と命令の列が同じ");
    assert_eq!(
        plan.base_surface,
        world.current_base_surface(top, binds, pattern),
        "計画の土台の面は 1 か所の答えと同じ"
    );
    (named_ops(&ops, &id_to_path(atlas, RELS)), plan.base_surface)
}

/// 今のコマが `base` なら、層 (i) が指す面に置き換わり、コマの X・Y（7,3）は無視される。
#[test]
fn base_frame_replaces_the_static_layer_and_ignores_xy() {
    let (world, atlas) = world();
    let mut pattern = PatternState::default();
    pattern.set(10, base_frame(1, 7, 3));
    let got = planned(&world, &atlas, 0, &BindSet::default(), &pattern);
    assert_eq!(got, (vec![("c.png", 0, 0)], 1));
}

/// コマが終われば（欄が空になれば）元の層 (i) と元の面に戻る。
#[test]
fn ending_the_base_frame_restores_the_original() {
    let (world, atlas) = world();
    let mut pattern = PatternState::default();
    pattern.set(10, base_frame(1, 0, 0));
    planned(&world, &atlas, 0, &BindSet::default(), &pattern);
    pattern.remove(10);
    let got = planned(&world, &atlas, 0, &BindSet::default(), &pattern);
    assert_eq!(got, (vec![("a.png", 0, 0), ("b.png", 5, 6)], 0));
}

/// ON の着せ替えの pattern0 の `base` は層 (i) を置き換え（XY の 9,9 は無視）、pattern1 はその上に重なる。
/// OFF なら元に戻る。
#[test]
fn dress_up_pattern0_base_replaces_while_on() {
    let (world, atlas) = world();
    let none = PatternState::default();
    let on = planned(&world, &atlas, 0, &BindSet::from_ids([20]), &none);
    assert_eq!(on, (vec![("d.png", 0, 0), ("e.png", 4, 4)], 2));
    let off = planned(&world, &atlas, 0, &BindSet::default(), &none);
    assert_eq!(off, (vec![("a.png", 0, 0), ("b.png", 5, 6)], 0));
}

/// 置き換えが複数あるとき、描く順の後（着せ替え 20 が今のコマ 10 の後）が勝ち、負けた方は描かない。
#[test]
fn the_later_base_in_drawing_order_wins() {
    let (world, atlas) = world();
    let mut pattern = PatternState::default();
    pattern.set(10, base_frame(1, 0, 0));
    let got = planned(&world, &atlas, 0, &BindSet::from_ids([20]), &pattern);
    assert_eq!(got, (vec![("d.png", 0, 0), ("e.png", 4, 4)], 2));
}

/// `always` の休みのコマの `base` も置き換える（XY の 5,5 は無視）。
#[test]
fn always_base_replaces_the_static_layer() {
    let (world, atlas) = world();
    let got = planned(
        &world,
        &atlas,
        7,
        &BindSet::default(),
        &PatternState::default(),
    );
    assert_eq!(got, (vec![("c.png", 0, 0)], 1));
}

/// 入れ子の面の `base` はその面の層 (i) を面の位置 (3,4) で置き換え、根の土台の面は変わらない。
#[test]
fn nested_base_replaces_the_child_layer_only() {
    let (world, atlas) = world();
    let got = planned(
        &world,
        &atlas,
        8,
        &BindSet::default(),
        &PatternState::default(),
    );
    assert_eq!(got, (vec![("c.png", 3, 4)], 8));
}

/// 指す面が無い・`-1` の `base` は置き換えず、そのコマは何も描かない。
#[test]
fn missing_or_minus_one_target_does_not_replace() {
    let (world, atlas) = world();
    let mut pattern = PatternState::default();
    pattern.set(10, base_frame(99, 0, 0));
    let missing = planned(&world, &atlas, 0, &BindSet::default(), &pattern);
    assert_eq!(missing, (vec![("a.png", 0, 0), ("b.png", 5, 6)], 0));
    let none = PatternState::default();
    let always = planned(&world, &atlas, 11, &BindSet::default(), &none);
    assert_eq!(always, (vec![("a.png", 0, 0)], 11));
    let dress = planned(&world, &atlas, 12, &BindSet::from_ids([21]), &none);
    assert_eq!(dress, (vec![("a.png", 0, 0)], 12));
}

/// 無い面を指す `base` が描く順の後に在っても、在る面を指す前の `base` が置き換える。
#[test]
fn a_missing_later_base_does_not_beat_an_existing_one() {
    let (world, atlas) = world();
    let mut pattern = PatternState::default();
    pattern.set(10, base_frame(1, 0, 0));
    pattern.set(30, base_frame(99, 0, 0));
    let got = planned(&world, &atlas, 0, &BindSet::default(), &pattern);
    assert_eq!(got, (vec![("c.png", 0, 0)], 1));
}
