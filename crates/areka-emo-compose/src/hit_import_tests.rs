//! 子の領域の持ち込み（要件 4.1〜4.10・7.1・8.3・タスク 5.2）。
//!
//! 持ち込み済みの列を [`EmoWorld::hit_regions`] で引き、`hit_region_in`（画家則＝後ろが手前）で
//! 当てて確かめる。並びは［子から持ち込む分（element定義の番号の昇順）］→［親に直接書いた領域］。

use areka_parsers::shell::{Collision, CollisionName, parse};

use super::HitRegions;
use crate::hit::{RegionPriority, hit_region_in};
use crate::world::{EmoWorld, SurfaceIndex};

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

fn world_of(text: &str) -> EmoWorld {
    EmoWorld::build(&parse(text))
}

fn hit(world: &EmoWorld, id: u32, x: i64, y: i64) -> Option<String> {
    let list = world.hit_regions(id).expect("面の表に在る番号");
    hit_region_in(list, x, y, RegionPriority::Painter).map(str::to_string)
}

/// 面の表で `id` に [`HitRegions`] が付いているか。
fn has_regions(world: &EmoWorld, id: u32) -> bool {
    let entity = world.world().resource::<SurfaceIndex>().0[&id];
    world.world().get::<HitRegions>(entity).is_some()
}

fn coll(index: u32, left: i64, top: i64, right: i64, bottom: i64, name: &str) -> Collision {
    Collision {
        index,
        left,
        top,
        right,
        bottom,
        name: CollisionName::new(name.to_string()),
    }
}

/// 子の領域が element定義の X,Y だけずれた位置で当たり、元の位置では当たらない（要件 4.1・4.2）。
#[test]
fn child_region_hits_at_shifted_position() {
    let w = world_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\nelement1,overlay,1,10,20\n}\n\
         surface1\n{\ncollision0,0,0,5,5,Eye\n}\n",
    );
    assert_eq!(
        w.hit_regions(0),
        Some(&[coll(0, 10, 20, 15, 25, "Eye")][..])
    );
    assert_eq!(hit(&w, 0, 12, 22).as_deref(), Some("Eye"));
    assert_eq!(hit(&w, 0, 2, 2), None);
    assert!(has_regions(&w, 0));
    // 子そのものの列は転記のまま。
    assert_eq!(w.hit_regions(1), Some(&[coll(0, 0, 0, 5, 5, "Eye")][..]));
    assert!(!has_regions(&w, 1));
}

/// 重なった 2 つの子では、element定義の番号の大きい子が手前（サーフェスの番号ではない・要件 4.3）。
#[test]
fn higher_numbered_element_child_is_front() {
    let children = "surface1\n{\ncollision0,0,0,10,10,One\n}\n\
                    surface2\n{\ncollision0,0,0,10,10,Two\n}\n";
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,1,0,0\nelement2,overlay,2,0,0\n}}\n{children}"
    ));
    assert_eq!(hit(&w, 0, 5, 5).as_deref(), Some("Two"));
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,2,0,0\nelement2,overlay,1,0,0\n}}\n{children}"
    ));
    assert_eq!(hit(&w, 0, 5, 5).as_deref(), Some("One"));
}

/// 1 つの子の中の手前奥は、その子を単独で表示したときと同じ（要件 4.4）。
#[test]
fn order_inside_child_matches_standalone() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,100,0\n}\n\
         surface1\n{\ncollision0,0,0,10,10,Back\ncollision1,5,5,15,15,Front\n\
         collision2,8,8,9,9,Tiny\n}\n",
    );
    for (x, y) in [(2, 2), (7, 7), (8, 8), (12, 12), (9, 6)] {
        let alone = hit(&w, 1, x, y);
        assert_eq!(hit(&w, 0, x + 100, y), alone, "点 ({x},{y})");
    }
    assert_eq!(hit(&w, 0, 107, 7).as_deref(), Some("Front"));
}

/// 親に直接書いた領域が、子から持ち込んだ領域より手前（要件 4.5）。
#[test]
fn parent_direct_region_is_front() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\ncollision0,0,0,10,10,Face\n}\n\
         surface1\n{\ncollision0,5,5,20,20,Hand\n}\n",
    );
    assert_eq!(hit(&w, 0, 7, 7).as_deref(), Some("Face"));
    assert_eq!(hit(&w, 0, 15, 15).as_deref(), Some("Hand"));
    // 並びは［持ち込み］→［直接］。
    assert_eq!(
        w.hit_regions(0),
        Some(&[coll(0, 5, 5, 20, 20, "Hand"), coll(0, 0, 0, 10, 10, "Face")][..])
    );
}

/// 親と同じ名前の子の領域は、親の領域と重なっていない場所でも持ち込まない（要件 4.8）。
#[test]
fn child_region_with_parent_name_never_hits() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\ncollision0,0,0,10,10,Head\n}\n\
         surface1\n{\ncollision0,50,50,60,60,Head\ncollision1,70,70,80,80,Eye\n}\n",
    );
    assert_eq!(hit(&w, 0, 55, 55), None);
    assert_eq!(hit(&w, 0, 75, 75).as_deref(), Some("Eye"));
    assert_eq!(
        w.hit_regions(0),
        Some(
            &[
                coll(1, 70, 70, 80, 80, "Eye"),
                coll(0, 0, 0, 10, 10, "Head")
            ][..]
        )
    );
}

/// 孫→子→親の段ごとに名前で落とす（要件 4.9）。孫の A は子に A が在るので子で落ち（親に A が無くても
/// 親へ届かない）、孫の B は子へは持ち込まれるが親に B が在るので親で落ちる。位置は各段の X,Y の和。
#[test]
fn names_are_dropped_level_by_level() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,100,0\ncollision0,0,0,5,5,B\n}\n\
         surface1\n{\nelement1,overlay,2,10,20\ncollision0,0,0,5,5,A\n}\n\
         surface2\n{\ncollision0,0,0,5,5,A\ncollision1,30,30,35,35,B\ncollision2,40,40,45,45,C\n}\n",
    );
    // 子 1 の列: 孫の B・C（10,20 ずらし）→ 子の A。
    assert_eq!(
        w.hit_regions(1),
        Some(
            &[
                coll(1, 40, 50, 45, 55, "B"),
                coll(2, 50, 60, 55, 65, "C"),
                coll(0, 0, 0, 5, 5, "A"),
            ][..]
        )
    );
    // 親 0 の列: 子の列から B を落とし（100,0 ずらし）→ 親の B。孫の A は子の A に隠れて来ない。
    assert_eq!(
        w.hit_regions(0),
        Some(
            &[
                coll(2, 150, 60, 155, 65, "C"),
                coll(0, 100, 0, 105, 5, "A"),
                coll(0, 0, 0, 5, 5, "B"),
            ][..]
        )
    );
    assert_eq!(hit(&w, 0, 112, 22), None, "孫の A は子で落ちる");
    assert_eq!(hit(&w, 0, 142, 52), None, "孫の B は親で落ちる");
    assert_eq!(hit(&w, 0, 152, 62).as_deref(), Some("C"));
}

/// 親に無い同じ名前を 2 つの子が持てば、両方とも持ち込む（要件 4.10）。
#[test]
fn same_name_from_two_children_both_hit() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\nelement2,overlay,2,100,0\n}\n\
         surface1\n{\ncollision0,0,0,10,10,Eye\n}\n\
         surface2\n{\ncollision0,0,0,10,10,Eye\n}\n",
    );
    assert_eq!(hit(&w, 0, 5, 5).as_deref(), Some("Eye"));
    assert_eq!(hit(&w, 0, 105, 5).as_deref(), Some("Eye"));
    assert_eq!(w.hit_regions(0).map(<[_]>::len), Some(2));
}

/// 無い番号・範囲を超える数・自分自身を指す辺からは持ち込まない（要件 4.6）。持ち込みが無いので何も付かない。
#[test]
fn missing_numbers_and_self_cycle_import_nothing() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,9999,0,0\nelement2,overlay,4294967296,0,0\n\
         collision0,0,0,10,10,Head\n}\n\
         surface1\n{\nelement1,overlay,1,50,50\ncollision0,0,0,10,10,Self\n}\n",
    );
    assert!(!has_regions(&w, 0));
    assert!(!has_regions(&w, 1));
    assert_eq!(w.hit_regions(0), Some(&[coll(0, 0, 0, 10, 10, "Head")][..]));
    assert_eq!(hit(&w, 1, 55, 55), None);
}

/// 相互の循環では、根から見て先祖へ戻る辺だけを持ち込まない（合成が描く範囲と同じ・要件 4.6）。
#[test]
fn mutual_cycle_cuts_only_the_back_edge() {
    let w = world_of(
        "surface1\n{\nelement1,overlay,2,10,0\ncollision0,0,0,5,5,One\n}\n\
         surface2\n{\nelement1,overlay,1,20,0\ncollision0,0,0,5,5,Two\n}\n",
    );
    assert_eq!(
        w.hit_regions(1),
        Some(&[coll(0, 10, 0, 15, 5, "Two"), coll(0, 0, 0, 5, 5, "One")][..])
    );
    assert_eq!(
        w.hit_regions(2),
        Some(&[coll(0, 20, 0, 25, 5, "One"), coll(0, 0, 0, 5, 5, "Two")][..])
    );
}

/// pattern定義が指すサーフェス（着せ替えの pattern0 を含む）からは持ち込まない（要件 4.7）。
#[test]
fn pattern_targets_import_nothing() {
    let w = world_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\ncollision0,0,0,10,10,Head\n\
         animation0.interval,random,2\nanimation0.pattern0,overlay,1,50,0,0\n\
         animation1.interval,bind\nanimation1.pattern0,overlay,1,0,0,0\n}\n\
         surface1\n{\ncollision0,0,0,100,100,Mouth\n}\n",
    );
    assert!(!has_regions(&w, 0));
    assert_eq!(hit(&w, 0, 50, 50), None);
}

/// 入れ子の無いシェル（emo2）では、どのサーフェスにも [`HitRegions`] が付かず、当たり判定の列は転記の
/// ままの `SurfaceMaster.collisions`（要件 7.1）。無い番号は `None`。
#[test]
fn shell_without_nesting_gets_no_hit_regions() {
    let path = crate::sample_test_support::emo2_root().join("shell/master/surfaces.txt");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
    let w = world_of(&text);
    let ids: Vec<u32> = w.surface_ids().collect();
    assert!(
        ids.iter()
            .any(|&id| !w.surface(id).unwrap().collisions.is_empty()),
        "emo2 は領域を持つ（空の列での素通りを防ぐ）"
    );
    for id in ids {
        assert!(!has_regions(&w, id), "surface{id}");
        assert_eq!(
            w.hit_regions(id),
            Some(w.surface(id).unwrap().collisions.as_slice()),
            "surface{id}"
        );
    }
    assert_eq!(w.hit_regions(u32::MAX), None);
}

/// 検体: 子 10 に書いた名前が、子を置いた位置で親から引ける（完了の姿）。
///
/// 親 B（1）は子 10 を (90,50) に置き、領域を持たないので Head・Eye の両方が来る（Eye が後ろ＝手前）。
/// 親 C（2）は element0 で子 10 を (0,0) に置く。親 A（0）は Head を持つので子の Head は来ず、Eye は
/// 来るが親の Head（0,0〜100,60）の下に隠れる（親が手前）。
#[test]
fn fixture_parents_hit_child_names_at_placed_position() {
    let w = world_of(FIXTURE);
    assert_eq!(hit(&w, 1, 100, 55).as_deref(), Some("Eye"));
    assert_eq!(hit(&w, 1, 92, 55).as_deref(), Some("Head"));
    assert_eq!(hit(&w, 1, 5, 5), None);
    assert_eq!(hit(&w, 2, 10, 5).as_deref(), Some("Eye"));
    assert_eq!(hit(&w, 2, 2, 5).as_deref(), Some("Head"));
    assert_eq!(
        w.hit_regions(0),
        Some(
            &[
                coll(1, 25, 30, 35, 40, "Eye"),
                coll(0, 0, 0, 100, 60, "Head"),
                coll(1, 0, 60, 100, 200, "Body"),
            ][..]
        )
    );
    assert_eq!(hit(&w, 0, 30, 35).as_deref(), Some("Head"));
    // 持ち込みが在るのは子 10 を置く 0・1・2 だけ（10 自身・領域の無い入れ子・pattern定義の先には付かない）。
    let with: Vec<u32> = w.surface_ids().filter(|&id| has_regions(&w, id)).collect();
    assert_eq!(with, vec![0, 1, 2]);
}
