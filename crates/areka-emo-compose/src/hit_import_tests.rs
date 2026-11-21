//! 子の領域の持ち込み。
//!
//! 持ち込み済みの列を [`EmoWorld::hit_regions`] で引き、並びそのもの（後ろが手前）を確かめる。並びは［子から持ち込む分（element定義の番号の昇順）］→［親に直接書いた領域］。

use areka_parsers::shell::{CollisionBounds, CollisionShape, parse};

use super::HitRegions;
use crate::plan_test_support::{coll, component_of};
use crate::world::EmoWorld;

const FIXTURE: &str = include_str!("../tests/fixtures/surface-nesting/surfaces.txt");

fn world_of(text: &str) -> EmoWorld {
    EmoWorld::build(&parse(text))
}

fn names(world: &EmoWorld, id: u32) -> Vec<&str> {
    let list = world.hit_regions(id).expect("面の表に在る番号");
    list.iter().map(|c| c.name.as_str()).collect()
}

/// 面の表で `id` に [`HitRegions`] が付いているか。
fn has_regions(world: &EmoWorld, id: u32) -> bool {
    component_of::<HitRegions>(world.world(), id).is_some()
}

/// 子の領域が element定義の X,Y だけずれた位置で当たり、元の位置では当たらない。
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
    assert!(has_regions(&w, 0));
    // 子そのものの列は転記のまま。
    assert_eq!(w.hit_regions(1), Some(&[coll(0, 0, 0, 5, 5, "Eye")][..]));
    assert!(!has_regions(&w, 1));
}

/// 重なった 2 つの子では、element定義の番号の大きい子が手前（サーフェスの番号ではない）。
#[test]
fn higher_numbered_element_child_is_front() {
    let children = "surface1\n{\ncollision0,0,0,10,10,One\n}\n\
                    surface2\n{\ncollision0,0,0,10,10,Two\n}\n";
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,1,0,0\nelement2,overlay,2,0,0\n}}\n{children}"
    ));
    assert_eq!(names(&w, 0), ["One", "Two"]);
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,2,0,0\nelement2,overlay,1,0,0\n}}\n{children}"
    ));
    assert_eq!(names(&w, 0), ["Two", "One"]);
}

/// 1 つの子の中の手前奥は、その子を単独で表示したときと同じ。
#[test]
fn order_inside_child_matches_standalone() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,100,0\n}\n\
         surface1\n{\ncollision0,0,0,10,10,Back\ncollision1,5,5,15,15,Front\n\
         collision2,8,8,9,9,Tiny\n}\n",
    );
    assert_eq!(names(&w, 0), names(&w, 1));
    assert_eq!(names(&w, 0), ["Back", "Front", "Tiny"]);
    assert_eq!(
        w.hit_regions(0),
        Some(
            &[
                coll(0, 100, 0, 110, 10, "Back"),
                coll(1, 105, 5, 115, 15, "Front"),
                coll(2, 108, 8, 109, 9, "Tiny"),
            ][..]
        )
    );
}

/// 親に直接書いた領域が、子から持ち込んだ領域より手前。
#[test]
fn parent_direct_region_is_front() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\ncollision0,0,0,10,10,Face\n}\n\
         surface1\n{\ncollision0,5,5,20,20,Hand\n}\n",
    );
    // 並びは［持ち込み］→［直接］。
    assert_eq!(
        w.hit_regions(0),
        Some(&[coll(0, 5, 5, 20, 20, "Hand"), coll(0, 0, 0, 10, 10, "Face")][..])
    );
}

/// 親と同じ名前の子の領域は、親の領域と重なっていない場所でも持ち込まない。
#[test]
fn child_region_with_parent_name_never_hits() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\ncollision0,0,0,10,10,Head\n}\n\
         surface1\n{\ncollision0,50,50,60,60,Head\ncollision1,70,70,80,80,Eye\n}\n",
    );
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

/// 孫→子→親の段ごとに名前で落とす。孫の A は子に A が在るので子で落ち（親に A が無くても
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
}

/// 親に無い同じ名前を 2 つの子が持てば、両方とも持ち込む。
#[test]
fn same_name_from_two_children_both_hit() {
    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\nelement2,overlay,2,100,0\n}\n\
         surface1\n{\ncollision0,0,0,10,10,Eye\n}\n\
         surface2\n{\ncollision0,0,0,10,10,Eye\n}\n",
    );
    assert_eq!(
        w.hit_regions(0),
        Some(
            &[
                coll(0, 0, 0, 10, 10, "Eye"),
                coll(0, 100, 0, 110, 10, "Eye")
            ][..]
        )
    );
}

/// 無い番号・範囲を超える数・自分自身を指す辺からは持ち込まない。持ち込みが無いので何も付かない。
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
    assert_eq!(w.hit_regions(1), Some(&[coll(0, 0, 0, 10, 10, "Self")][..]));
}

/// 相互の循環では、根から見て先祖へ戻る辺だけを持ち込まない（合成が描く範囲と同じ）。
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

/// pattern定義が指すサーフェス（着せ替えの pattern0 を含む）からは持ち込まない。
#[test]
fn pattern_targets_import_nothing() {
    let w = world_of(
        "surface0\n{\nelement0,overlay,body.png,0,0\ncollision0,0,0,10,10,Head\n\
         animation0.interval,random,2\nanimation0.pattern0,overlay,1,50,0,0\n\
         animation1.interval,bind\nanimation1.pattern0,overlay,1,0,0,0\n}\n\
         surface1\n{\ncollision0,0,0,100,100,Mouth\n}\n",
    );
    assert!(!has_regions(&w, 0));
    assert_eq!(w.hit_regions(0), Some(&[coll(0, 0, 0, 10, 10, "Head")][..]));
}

/// 入れ子の無いシェル（emo2）では、どのサーフェスにも [`HitRegions`] が付かず、当たり判定の列は転記の
/// ままの `SurfaceMaster.collisions`。無い番号は `None`。
#[test]
fn shell_without_nesting_gets_no_hit_regions() {
    let w = EmoWorld::build(&crate::sample_test_support::emo2_shell());
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
    // 親 B: 子を (90,50) → Head (90,50)-(110,60)・Eye (95,50)-(105,60)（Head が先・Eye が後）。
    assert_eq!(
        w.hit_regions(1),
        Some(
            &[
                coll(0, 90, 50, 110, 60, "Head"),
                coll(1, 95, 50, 105, 60, "Eye")
            ][..]
        )
    );
    assert_eq!(
        w.hit_regions(2),
        Some(&[coll(0, 0, 0, 20, 10, "Head"), coll(1, 5, 0, 15, 10, "Eye")][..])
    );
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
    // 持ち込みが在るのは子 10 を置く 0・1・2 だけ（10 自身・領域の無い入れ子・pattern定義の先には付かない）。
    let with: Vec<u32> = w.surface_ids().filter(|&id| has_regions(&w, id)).collect();
    assert_eq!(with, vec![0, 1, 2]);
}

/// 広がりの予算（22・130）: 予算は入る面と面の行と持ち込む領域を数える（collision 1 行の子 1 つで
/// 入る 1 ＋行 1 ＋渡す 1＝3）。予算 6 の持ち込みは
/// element定義の子 3 つのうち 2 つ目まで入って 3 つ目を持ち込まず、`error!` を 1 行だけ出す。黙らせる
/// 組み方の予算は同じく断って記録を出さない。
#[test]
fn spent_budget_does_not_import_children_not_yet_entered() {
    use crate::budget::Budget;
    use crate::log_capture::capture_logs;
    use crate::world::Records;

    let w = world_of(
        "surface0\n{\nelement1,overlay,1,0,0\nelement2,overlay,2,0,0\nelement3,overlay,3,0,0\n}\n\
         surface1\n{\ncollision0,0,0,5,5,One\n}\n\
         surface2\n{\ncollision0,0,0,5,5,Two\n}\n\
         surface3\n{\ncollision0,0,0,5,5,Three\n}\n",
    );
    let master = w.surface(0).expect("面 0");
    let mut list = Vec::new();
    let logs = capture_logs(|| list = super::regions_of(&w, master, &mut Budget::new(6), None));
    assert_eq!(
        list,
        [coll(0, 0, 0, 5, 5, "One"), coll(0, 0, 0, 5, 5, "Two")]
    );
    assert_eq!(logs.matches("level=ERROR").count(), 1, "{logs}");
    assert!(
        logs.lines().any(|l| l.contains("level=ERROR")
            && l.contains("walk=\"hit_import\"")
            && l.contains("root=0")),
        "{logs}"
    );
    let logs = capture_logs(|| {
        list = super::regions_of(
            &w,
            master,
            &mut Budget::with_records(6, Records::Quiet),
            None,
        )
    });
    assert_eq!(list.len(), 2);
    assert!(logs.is_empty(), "{logs}");
    // 予算が足りれば 3 つとも持ち込む（本番の組み方）。
    assert_eq!(w.hit_regions(0).map(<[_]>::len), Some(3));
}

/// 面 1000 に "Y" が 20 行、面 100+i が "Z" 1 行と面 1000 を置き、面 i が
/// "Y" 1 行と面 100+i を置く（i は 20 組）。根をまたいで共有する覚えは面 100+i ごとに 21 本の列を持ち、根の
/// 答えも覚えるので、上限が無ければ合計 480 本になる。覚えの予算 100 では合計が 100 以下で止まり、後の面は
/// 覚えず、どの根の答えも覚えを使わないときと同じ。
#[test]
fn memo_stops_at_its_budget_and_answers_stay_the_same() {
    use super::{HitMemo, regions_of};
    use crate::budget::{Budget, VISIT_BUDGET};

    const N: u32 = 20;
    let mut text = String::from("surface1000\n{\n");
    for c in 0..20 {
        text.push_str(&format!("collision{c},0,0,5,5,Y\n"));
    }
    text.push_str("}\n");
    for i in 0..N {
        text.push_str(&format!(
            "surface{}\n{{\nelement0,overlay,1000,0,0\ncollision0,0,0,5,5,Z\n}}\n\
             surface{i}\n{{\nelement0,overlay,{},0,0\ncollision0,0,0,5,5,Y\n}}\n",
            100 + i,
            100 + i
        ));
    }
    let w = world_of(&text);
    let mut memo = HitMemo::new(100);
    for id in w.surface_ids() {
        let master = w.surface(id).expect("面");
        let with_memo = regions_of(&w, master, &mut Budget::new(VISIT_BUDGET), Some(&mut memo));
        let without = regions_of(&w, master, &mut Budget::new(VISIT_BUDGET), None);
        assert_eq!(with_memo, without, "根 {id}");
    }
    let stored: usize = memo.map.values().map(|(_, list)| list.len()).sum();
    assert!(stored <= 100, "覚えの合計 {stored} 本は上限以下");
    assert!(memo.map.contains_key(&100), "初めの面は覚える");
    assert!(
        !memo.map.contains_key(&(100 + N - 1)),
        "上限の後の面は覚えない"
    );
}

/// 形ごとの持ち込みのずらし: 全形が飽和の足し算で、腕は変わらない。
#[test]
fn shifted_moves_every_shape_saturating() {
    use super::shifted;
    use areka_parsers::shell::{CollisionBounds, CollisionRegion, CollisionShape};
    let b = CollisionBounds {
        left: 1,
        top: 2,
        right: i64::MAX,
        bottom: 4,
    };
    let moved = CollisionBounds {
        left: 11,
        top: 22,
        right: i64::MAX,
        bottom: 24,
    };
    assert_eq!(
        shifted(CollisionShape::Rect(b), 10, 20),
        CollisionShape::Rect(moved)
    );
    assert_eq!(
        shifted(CollisionShape::Ellipse(b), 10, 20),
        CollisionShape::Ellipse(moved)
    );
    assert_eq!(
        shifted(
            CollisionShape::Circle {
                x: 1,
                y: i64::MIN,
                radius: 7
            },
            10,
            -20
        ),
        CollisionShape::Circle {
            x: 11,
            y: i64::MIN,
            radius: 7
        }
    );
    assert_eq!(
        shifted(CollisionShape::Polygon(vec![(0, 0), (5, 6)]), 1, 2),
        CollisionShape::Polygon(vec![(1, 2), (6, 8)])
    );
    let r = CollisionRegion {
        file: "a.png".into(),
        red: 1,
        green: 2,
        blue: 3,
        invert: true,
        x: 0,
        y: 0,
    };
    assert_eq!(
        shifted(CollisionShape::Region(r.clone()), 3, 4),
        CollisionShape::Region(CollisionRegion { x: 3, y: 4, ..r })
    );
}

/// 子の列の形だけを順に取り出す。
fn shapes(w: &EmoWorld, id: u32) -> Vec<CollisionShape> {
    let list = w.hit_regions(id).expect("面の表に在る番号");
    list.iter().map(|c| c.shape.clone()).collect()
}

const FIVE_SHAPES: &str = "surface1\n{\ncollisionex0,R,rect,1,2,3,4\ncollisionex1,E,ellipse,1,2,3,4\n\
     collisionex2,C,circle,5,6,7\ncollisionex3,P,polygon,0,0,5,6,9,1\n\
     collisionex4,G,region,m.png,255,0,0\n}\n";

/// 取り込み経由: 5 つの形が element定義の X,Y だけずれて親に並ぶ。円の半径はそのまま・
/// 多角形は全頂点が順を保ってずれ・`region` は置く位置がずれる。どの形も並びから除かれない。
#[test]
fn import_shifts_all_five_shapes_by_element_offset() {
    use areka_parsers::shell::CollisionRegion;
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,1,10,20\n}}\n{FIVE_SHAPES}"
    ));
    let b = |l, t, r, bo| CollisionBounds {
        left: l,
        top: t,
        right: r,
        bottom: bo,
    };
    let region = match &shapes(&w, 1)[4] {
        CollisionShape::Region(r) => r.clone(),
        other => panic!("region のはず: {other:?}"),
    };
    assert_eq!(
        shapes(&w, 0),
        [
            CollisionShape::Rect(b(11, 22, 13, 24)),
            CollisionShape::Ellipse(b(11, 22, 13, 24)),
            CollisionShape::Circle {
                x: 15,
                y: 26,
                radius: 7
            },
            CollisionShape::Polygon(vec![(10, 20), (15, 26), (19, 21)]),
            CollisionShape::Region(CollisionRegion {
                x: region.x + 10,
                y: region.y + 20,
                ..region
            }),
        ]
    );
    // 子そのものの列はずれない。
    assert_eq!(shapes(&w, 1).len(), 5);
}

/// `i64` の端の近くでは飽和し、パニックしない。
#[test]
fn import_saturates_near_i64_limits() {
    let max = i64::MAX;
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,1,{max},{}\n}}\n\
         surface1\n{{\ncollisionex0,R,rect,1,2,{max},4\ncollisionex1,C,circle,5,6,7\n\
         collisionex2,P,polygon,0,0,5,6,9,1\n}}\n",
        i64::MIN
    ));
    assert_eq!(
        shapes(&w, 0),
        [
            CollisionShape::Rect(CollisionBounds {
                left: max,
                top: i64::MIN + 2,
                right: max,
                bottom: i64::MIN + 4
            }),
            CollisionShape::Circle {
                x: max,
                y: i64::MIN + 6,
                radius: 7
            },
            CollisionShape::Polygon(vec![
                (max, i64::MIN),
                (max, i64::MIN + 6),
                (max, i64::MIN + 1)
            ]),
        ]
    );
}

/// 親に直接書いた同じ名前は `region` でも子の同じ名前を落とし、落とされなかった形は並びに残る。
#[test]
fn parent_direct_region_drops_child_same_name() {
    let w = world_of(&format!(
        "surface0\n{{\nelement1,overlay,1,10,20\ncollisionex0,G,region,p.png,0,255,0\n}}\n{FIVE_SHAPES}"
    ));
    let names: Vec<&str> = w
        .hit_regions(0)
        .unwrap()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    // 子の G（region）は落ち、残り 4 つの後ろに親の G が手前で続く。
    assert_eq!(names, ["R", "E", "C", "P", "G"]);
    assert!(matches!(
        w.hit_regions(0).unwrap()[4].shape,
        CollisionShape::Region(ref r) if r.file == "p.png"
    ));
}
