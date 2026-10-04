//! 参照の表と「今の絵に出ている部品」（要件 5.11・5.13・8.3）。
//!
//! 規則（`flatten_surface` と同じ辺）: 一番上から ① element定義の子 ② 有効な着せ替えの pattern0 の先
//! （同じ animation の番号にコマが在ればコマが置き換える）③ そのサーフェスのコマ（一番上は今までの欄・
//! 部品は部品の欄）のうち描画メソッドが動くもので、着せ替えの種類なら有効なものの先、をたどる。
//! 先祖へ戻る辺は進まない。

use std::collections::{BTreeMap, BTreeSet};

use areka_parsers::shell::parse;

use super::{NestTable, SurfaceParts};
use crate::bind::BindSet;
use crate::method::ComposeMethod;
use crate::pattern::{PatternFrame, PatternState};
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

/// 検体の面の表（ファイル名の慣習だけで建つ 2・11 を含む）。
fn fixture_world() -> EmoWorld {
    let images = BTreeMap::from([
        (2, "surface2.png".to_string()),
        (11, "surface11.png".to_string()),
    ]);
    EmoWorld::build_with_images(&parse(FIXTURE), &images)
}

fn table_of(text: &str) -> NestTable {
    EmoWorld::build(&parse(text)).nest_table()
}

fn frame(surface_id: u32) -> PatternFrame {
    PatternFrame {
        surface_id,
        method: ComposeMethod::Overlay,
        x: 0,
        y: 0,
    }
}

fn visible(table: &NestTable, top: u32, binds: &[u32], pattern: &PatternState) -> Vec<u32> {
    let mut out = Vec::new();
    table.visible_parts(
        top,
        &BindSet::from_ids(binds.iter().copied()),
        pattern,
        &mut out,
    );
    out
}

fn listed(world: &EmoWorld, table: &NestTable) -> BTreeSet<u32> {
    world
        .surface_ids()
        .filter(|&id| table.parts(id).is_some())
        .collect()
}

/// 入れ子も着せ替えの種類の animation も無いシェルでは表が空・部品も無い。
#[test]
fn shell_without_nesting_or_binds_gives_empty_table() {
    let table = table_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\nanimation0.interval,random,2\nanimation0.pattern0,overlay,1,50,0,0\n}\n\
         surface1\n{\nelement0,overlay,eye.png,0,0\n}\n",
    );
    assert!(table.is_empty());
    assert_eq!(table, NestTable::default());
    // 一番上のコマ（random）の先は部品に数える（③）が、表は空のまま。
    let mut pattern = PatternState::default();
    pattern.set(0, frame(1));
    assert_eq!(visible(&table, 0, &[], &pattern), vec![1]);
    assert_eq!(
        visible(&table, 0, &[], &PatternState::default()),
        Vec::<u32>::new()
    );
}

/// 検体の表: 載るのは子か着せ替えを持つ面だけ。無い番号・範囲を超える数は子にならない。
#[test]
fn fixture_table_lists_children_and_bind_targets() {
    let world = fixture_world();
    let table = world.nest_table();

    assert_eq!(
        listed(&world, &table),
        BTreeSet::from([0, 1, 2, 30, 31, 60, 61, 62, 71]),
        "50（無い番号だけ）・10・40（random だけ）は載らない"
    );
    let children = |id: u32| table.parts(id).unwrap().children.clone();
    assert_eq!(children(0), vec![10, 30]);
    // surface.append1 の element2 が 30 を足す（element定義の番号の昇順）。
    assert_eq!(children(1), vec![10, 30]);
    // ファイル名の慣習だけで建つ 11 も「在る」。
    assert_eq!(children(2), vec![10, 11]);
    assert_eq!(children(30), vec![31]);
    assert_eq!(children(31), vec![32]);
    assert_eq!(children(60), vec![60]);
    assert_eq!(children(61), vec![62]);
    assert_eq!(children(62), vec![61]);
    assert_eq!(children(71), vec![70]);
    assert_eq!(
        table.parts(0),
        Some(&SurfaceParts {
            children: vec![10, 30],
            bind_targets: vec![(100, 40)],
            bind_ids: vec![100],
        })
    );
}

/// 検体の部品: 仕込んだ子・3 段の入れ子・pattern定義の先が出る。
#[test]
fn fixture_visible_parts_follow_children_binds_and_frames() {
    let table = fixture_world().nest_table();
    let empty = PatternState::default();

    // ① 子と 3 段の入れ子。着せ替え 100 が無効なら 40 は出ない。
    assert_eq!(visible(&table, 0, &[], &empty), vec![10, 30, 31, 32]);
    // ② 有効な着せ替えの pattern0 の先。
    assert_eq!(visible(&table, 0, &[100], &empty), vec![10, 30, 31, 32, 40]);
    assert_eq!(visible(&table, 1, &[], &empty), vec![10, 30, 31, 32]);
    assert_eq!(visible(&table, 2, &[], &empty), vec![10, 11]);
    assert_eq!(visible(&table, 71, &[], &empty), vec![70]);
    // 部品の無い面。
    assert_eq!(visible(&table, 32, &[100], &empty), Vec::<u32>::new());

    // ③ 部品のコマの先（10 のまばたき→12・40 の口→41）。部品の欄から引く。
    let mut pattern = PatternState::default();
    pattern.set_part(10, 0, frame(12));
    pattern.set_part(40, 0, frame(41));
    assert_eq!(
        visible(&table, 0, &[100], &pattern),
        vec![10, 12, 30, 31, 32, 40, 41]
    );
    // 40 が出ていなければ、40 の部品のコマは数えない。
    assert_eq!(visible(&table, 0, &[], &pattern), vec![10, 12, 30, 31, 32]);
    // 一番上の欄の同じ番号のコマは部品のコマとして読まない（欄が混ざらない）。
    let mut top_only = PatternState::default();
    top_only.set(0, frame(12));
    assert_eq!(visible(&table, 1, &[], &top_only), vec![10, 12, 30, 31, 32]);
    let mut wrong_field = PatternState::default();
    wrong_field.set_part(1, 0, frame(12));
    assert_eq!(visible(&table, 1, &[], &wrong_field), vec![10, 30, 31, 32]);
}

/// ② 同じ animation の番号にコマが在れば、pattern0 の先でなくコマの先を数える。
#[test]
fn frame_replaces_bind_pattern0() {
    let table = fixture_world().nest_table();
    let mut pattern = PatternState::default();
    pattern.set(100, frame(41));
    assert_eq!(
        visible(&table, 0, &[100], &pattern),
        vec![10, 30, 31, 32, 41]
    );

    // 置き換えたコマが描かれないメソッドなら、どちらの先も数えない。
    let mut not_drawn = PatternState::default();
    not_drawn.set(
        100,
        PatternFrame {
            method: ComposeMethod::Reduce,
            ..frame(41)
        },
    );
    assert_eq!(visible(&table, 0, &[100], &not_drawn), vec![10, 30, 31, 32]);
}

/// ③ 有効でない着せ替えの種類のコマは数えない。着せ替えでないコマは無条件に数える。
#[test]
fn inactive_bind_frames_are_not_counted() {
    let text = "\
surface0
{
element0,overlay,body.png,0,0
element1,overlay,5,0,0
animation9.interval,bind
animation9.pattern0,overlay,6,0,0,0
}
surface5
{
element0,overlay,part.png,0,0
animation7.interval,bind+random,2
animation7.pattern1,overlay,6,50,0,0
animation8.interval,random,2
animation8.pattern1,overlay,7,50,0,0
}
surface6
{
element0,overlay,eye.png,0,0
}
surface7
{
element0,overlay,eye.png,0,0
}
";
    let table = table_of(text);
    // pattern0 を持たない bind+random だけの 5 も載る。
    assert_eq!(
        table.parts(5),
        Some(&SurfaceParts {
            children: vec![],
            bind_targets: vec![],
            bind_ids: vec![7],
        })
    );

    // 一番上の欄: 無効な着せ替え 9 のコマは数えない。
    let mut top = PatternState::default();
    top.set(9, frame(7));
    assert_eq!(visible(&table, 0, &[], &top), vec![5]);
    assert_eq!(visible(&table, 0, &[9], &top), vec![5, 7]);

    // 部品の欄: 無効な着せ替え 7 のコマは数えない・random 8 は数える。
    let mut part = PatternState::default();
    part.set_part(5, 7, frame(6));
    part.set_part(5, 8, frame(7));
    assert_eq!(visible(&table, 0, &[], &part), vec![5, 7]);
    assert_eq!(visible(&table, 0, &[7], &part), vec![5, 6, 7]);

    // 描画メソッドが動かないコマは数えない。
    let mut not_drawn = PatternState::default();
    not_drawn.set_part(
        5,
        8,
        PatternFrame {
            method: ComposeMethod::Replace,
            ..frame(7)
        },
    );
    assert_eq!(visible(&table, 0, &[], &not_drawn), vec![5]);
}

/// 着せ替えの pattern0 の先は index 0・0 以上・描画メソッドが動くものだけ（欄 2 が animation の番号の語も除く）。
#[test]
fn bind_targets_keep_only_drawable_pattern0() {
    let text = "\
surface0
{
element0,overlay,body.png,0,0
animation1.interval,bind
animation1.pattern0,overlay,-1,0,0,0
animation2.interval,bind
animation2.pattern0,reduce,3,0,0,0
animation3.interval,bind
animation3.pattern0,insert,4,0,0,0
animation4.interval,bind
animation4.pattern1,overlay,3,0,0,0
animation5.interval,bind
animation5.pattern0,add,3,0,0,0
animation6.interval,random,2
animation6.pattern0,overlay,4,0,0,0
}
surface3
{
element0,overlay,eye.png,0,0
}
surface4
{
element0,overlay,eye.png,0,0
}
";
    let table = table_of(text);
    assert_eq!(
        table.parts(0),
        Some(&SurfaceParts {
            children: vec![],
            bind_targets: vec![(5, 3)],
            bind_ids: vec![1, 2, 3, 4, 5],
        })
    );
    assert_eq!(
        visible(&table, 0, &[1, 2, 3, 4, 5, 6], &PatternState::default()),
        vec![3]
    );
}

/// 先祖へ戻る辺は進まない（自分自身・相互・コマが一番上へ戻る）。有限で終わる。
#[test]
fn back_edges_to_ancestors_are_not_followed() {
    let table = fixture_world().nest_table();
    let empty = PatternState::default();
    assert_eq!(visible(&table, 60, &[], &empty), Vec::<u32>::new());
    assert_eq!(visible(&table, 61, &[], &empty), vec![62]);
    assert_eq!(visible(&table, 62, &[], &empty), vec![61]);

    // 部品のコマが一番上・途中の先祖を指しても、そこへは戻らない。
    let mut pattern = PatternState::default();
    pattern.set_part(32, 0, frame(30));
    pattern.set_part(31, 0, frame(0));
    pattern.set_part(10, 0, frame(12));
    assert_eq!(visible(&table, 0, &[], &pattern), vec![10, 12, 30, 31, 32]);
}

/// 渡された列は先に空にしてから入れる（呼び手の列を使い回す）。
#[test]
fn out_is_cleared_and_reused() {
    let table = fixture_world().nest_table();
    let mut out = vec![999, 1, 2];
    table.visible_parts(2, &BindSet::default(), &PatternState::default(), &mut out);
    assert_eq!(out, vec![10, 11]);
    table.visible_parts(32, &BindSet::default(), &PatternState::default(), &mut out);
    assert!(out.is_empty());
}
