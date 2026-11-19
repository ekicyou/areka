//! 読み（`parse`）→ 合成（`EmoWorld::build`）→ 面の当たり判定の並び → 名前の解決までを通す重なりの檻
//! （spec: collisionex-regions 要件 2.1〜2.5・3.1・3.2・3.6・5.3・6.1・8.1・8.2）。
//!
//! 今の判定が当たるのは矩形だけ（楕円・円・多角形・`region` は読めるが当たらない）。
//! 座標と名前は本テストのための作り物。

use areka_parsers::shell::parse;

use crate::hit::{RegionPriority, hit_region_in, hit_region_scaled_in};
use crate::scale::ScaleRatio;
use crate::world::EmoWorld;

fn world(shell: &str) -> EmoWorld {
    EmoWorld::build(&parse(shell))
}

/// 面 `id` の並びを引いて (x, y) の名前を解決する（本番と同じ引き方）。
fn name(w: &EmoWorld, id: u32, x: i64, y: i64) -> Option<String> {
    let list = w.hit_regions(id).expect("面の当たり判定");
    hit_region_in(list, x, y, RegionPriority::Painter).map(str::to_string)
}

#[test]
fn collisionex_rect_is_closed_and_reversed_never_hits() {
    let w = world(
        "surface0\n{\ncollisionex0,R,rect,10,20,30,40\ncollisionex1,Rev,rect,60,60,50,50\n}\n",
    );
    // 中・四辺の上・四隅は当たり、1 つ外は外れる。
    for (x, y) in [(20, 30), (10, 20), (30, 40), (10, 40), (30, 20)] {
        assert_eq!(name(&w, 0, x, y).as_deref(), Some("R"), "({x},{y})");
    }
    for (x, y) in [(9, 30), (31, 30), (20, 19), (20, 41), (0, 0)] {
        assert_eq!(name(&w, 0, x, y), None, "({x},{y})");
    }
    // 反転した矩形は並べ直さず、どの点にも当たらない。
    for (x, y) in [(55, 55), (50, 50), (60, 60), (60, 50)] {
        assert_eq!(name(&w, 0, x, y), None, "反転 ({x},{y})");
    }
}

#[test]
fn shapes_not_yet_working_never_hit_and_never_cover_a_rect_below() {
    // 矩形の上に後から書いた 4 つの形は、同じ点でも下の矩形の名前を返す。
    let w = world(
        "surface0\n{\n\
         collisionex0,Under,rect,0,0,100,100\n\
         collisionex1,E,ellipse,0,0,100,100\n\
         collisionex2,C,circle,50,50,40\n\
         collisionex3,P,polygon,0,0,100,0,50,100\n\
         collisionex4,G,region,m.png,255,0,0\n\
         }\n",
    );
    // 4 つの形が並びに入っていること（読み落としで檻が空振りしないように）。
    assert_eq!(w.hit_regions(0).expect("面の当たり判定").len(), 5);
    for (x, y) in [(50, 50), (10, 5), (99, 99)] {
        assert_eq!(name(&w, 0, x, y).as_deref(), Some("Under"), "({x},{y})");
    }
    // 下に矩形が無ければ、形の上でも当たり無し。
    let w = world(
        "surface0\n{\n\
         collisionex1,E,ellipse,0,0,100,100\n\
         collisionex2,C,circle,50,50,40\n\
         collisionex3,P,polygon,0,0,100,0,50,100\n\
         collisionex4,G,region,m.png,255,0,0\n\
         }\n",
    );
    for (x, y) in [(50, 50), (10, 5), (99, 99)] {
        assert_eq!(name(&w, 0, x, y), None, "({x},{y})");
    }
}

#[test]
fn later_written_rect_is_in_front_across_collision_and_collisionex() {
    // collision* が先・collisionex が後 → 後の collisionex が手前。
    let w = world("surface0\n{\ncollision0,0,0,50,50,Old\ncollisionex1,New,rect,0,0,50,50\n}\n");
    assert_eq!(name(&w, 0, 25, 25).as_deref(), Some("New"));
    // 逆の書き順 → 後の collision が手前。
    let w = world("surface0\n{\ncollisionex0,Old,rect,0,0,50,50\ncollision1,0,0,50,50,New\n}\n");
    assert_eq!(name(&w, 0, 25, 25).as_deref(), Some("New"));
    // 重なっていない部分は、それぞれの名前が当たる。
    let w = world("surface0\n{\ncollision0,0,0,50,50,A\ncollisionex1,B,rect,25,25,80,80\n}\n");
    assert_eq!(name(&w, 0, 10, 10).as_deref(), Some("A"));
    assert_eq!(name(&w, 0, 70, 70).as_deref(), Some("B"));
    assert_eq!(name(&w, 0, 30, 30).as_deref(), Some("B"));
}

#[test]
fn append_collision_is_in_front_and_collision_sort_changes_nothing() {
    let body = "surface0\n{\ncollisionex0,Brace,rect,0,0,50,50\n}\nsurface.append0\n{\ncollision1,0,0,50,50,Append\n}\n";
    assert_eq!(name(&world(body), 0, 25, 25).as_deref(), Some("Append"));
    // collision-sort はどの値でも並びを変えない（後が手前のまま）。
    for sort in ["ascend", "descend", "none", ""] {
        let w = world(&format!("collision-sort,{sort}\n{body}"));
        assert_eq!(
            name(&w, 0, 25, 25).as_deref(),
            Some("Append"),
            "sort={sort:?}"
        );
        let names: Vec<_> = w
            .hit_regions(0)
            .unwrap()
            .iter()
            .map(|c| c.name.as_str().to_string())
            .collect();
        assert_eq!(names, ["Brace", "Append"], "sort={sort:?}");
    }
}

#[test]
fn scaled_back_mapping_is_the_same_for_collisionex_rects_and_lookups_are_deterministic() {
    let src = "surface0\n{\ncollision0,10,10,40,40,Old\ncollisionex1,Ex,rect,30,30,80,80\n}\n";
    let w = world(src);
    let list = w.hit_regions(0).unwrap();
    let k = ScaleRatio::new(2, 1).expect("k");
    // 縮約後の点で collisionex の矩形の名前が引ける（2 倍の窓の点 ÷2）。
    let hit = hit_region_scaled_in(list, 100, 100, k, RegionPriority::Painter);
    assert_eq!(hit.region, Some("Ex"));
    assert_eq!(hit.surface_point, (50, 50));
    // 窓の点を縮約したサーフェス点で、直の照合と必ず一致する（collision* の矩形と同じ戻し）。
    for px in (0..200).step_by(7) {
        for py in (0..200).step_by(11) {
            let s = hit_region_scaled_in(list, px, py, k, RegionPriority::Painter);
            let direct = hit_region_in(
                list,
                s.surface_point.0,
                s.surface_point.1,
                RegionPriority::Painter,
            );
            assert_eq!(s.region, direct, "({px},{py})");
        }
    }
    // 同じ入力を 2 回引いても、組み直しても同じ結果。
    let again = world(src);
    for (x, y) in [(5, 5), (20, 20), (35, 35), (80, 80), (81, 81)] {
        assert_eq!(name(&w, 0, x, y), name(&w, 0, x, y));
        assert_eq!(name(&w, 0, x, y), name(&again, 0, x, y), "({x},{y})");
    }
}
