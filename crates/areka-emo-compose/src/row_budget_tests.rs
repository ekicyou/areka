//! 予算を面の行の全部で数える（指摘 129〜132・134・136・137）: どの走査も面に入るときに「入る 1 ＋面の行の全部」を
//! 引き、足りなければ入らない。旧い単位（入る 1 だけ・命令列は element と animation の数だけ）では入ってしまう
//! 小さな版で、仕事が上限で止まることを確かめる。

use std::path::Path;

use areka_parsers::shell::parse;

use crate::bind::BindSet;
use crate::budget::Budget;
use crate::hit_import::regions_of;
use crate::log_capture::capture_logs;
use crate::method::ComposeMethod;
use crate::pattern::PatternState;
use crate::plan::{Extent, PlanScratch, compute_extent_limited, derive_ops};
use crate::plan_bounds::shell_bounds_limited;
use crate::plan_test_support::bake_atlas;
use crate::plan_test_support::bound_world;
use crate::world::EmoWorld;

/// `needle` を含む `error!` の行の数。
fn errors(logs: &str, needle: &str) -> usize {
    logs.lines()
        .filter(|l| l.contains("level=ERROR") && l.contains(needle))
        .count()
}

/// 面 0 が面 1 を `(x, 0)` に置き、面 1 は絵 1 枚と `extra`（行の並び）を持つシェルを、絵を束縛して組む。
fn child_with_rows(x: i64, extra: &str) -> (EmoWorld, areka_emo_atlas::AtlasTable) {
    let text = format!(
        "charset,UTF-8\nsurface0\n{{\nelement0,overlay,1,{x},0\n}}\n\
         surface1\n{{\nelement0,overlay,s.png,0,0\n{extra}}}\n"
    );
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let world = bound_world(&parse(&text), &[], &atlas);
    (world, atlas)
}

/// `n` 行の `prefix{i}{suffix}`。
fn rows(n: usize, prefix: &str, suffix: &str) -> String {
    (0..n).map(|i| format!("{prefix}{i}{suffix}\n")).collect()
}

/// 指摘 129: 面 1 は絵 1 枚と、描かない pattern 1,000 行の animation 1 つを持つ。旧い単位（入る 1＋element 1＋
/// animation 1＋積み荷 1＝4）なら予算 100 で開くが、pattern の行も数えると 1＋1＋（1＋1,000）＋積み荷 1＝1,004 なので
/// 開かず、`error!` は合成器 1 つで 1 行。1,004 なら切れない。
#[test]
fn plan_counts_the_pattern_rows_of_an_entered_surface() {
    let patterns = rows(1000, "animation0.pattern", ",overlay,-1,0,0,0");
    let (world, atlas) = child_with_rows(0, &format!("animation0.interval,never\n{patterns}"));
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let run = |limit: usize| {
        let mut scratch = PlanScratch::with_limit(limit);
        let mut ops = Vec::new();
        let logs = capture_logs(|| {
            for _ in 0..2 {
                ops.clear();
                derive_ops(&mut ops, &mut scratch, &world, &atlas, 0, &binds, &pattern);
            }
        });
        (ops.len(), errors(&logs, "walk=\"plan\""))
    };
    assert_eq!(run(100), (0, 1), "pattern の行で切れ、記録は 1 行");
    assert_eq!(run(1_004), (1, 0), "予算の内なら絵を描き、記録は無い");
}

/// 指摘 130: 面 1 は絵 1 枚と当たり判定 1,000 行を持つ（入る 1＋行 1,001＝1,002）。旧い単位（入る 1 だけ）なら
/// 予算 100 で入るが、面ごとの外形とシェルの外形はどちらも入らず（面 1 の絵を数えない）、`error!` を 1 行出す。
#[test]
fn extent_and_bounds_count_the_rows_of_an_entered_surface() {
    let (world, atlas) = child_with_rows(-5, &rows(1000, "collision", ",0,0,1,1,R"));
    let run_extent = |limit: usize| {
        let mut extent = Extent { w: 0, h: 0 };
        let logs = capture_logs(|| {
            extent =
                compute_extent_limited(&mut Default::default(), &world, &atlas, 0, limit, None);
        });
        (extent, errors(&logs, "walk=\"extent\""))
    };
    assert_eq!(run_extent(100), (Extent { w: 0, h: 0 }, 1));
    assert_ne!(
        run_extent(1_002).0,
        Extent { w: 0, h: 0 },
        "予算の内なら数える"
    );
    // シェルの外形: 面 0 から (-5, 0) に置いた面 1 だけが左端を負にする（面 1 を根にすると 0）。
    let run_bounds = |limit: usize| {
        let mut left = 0;
        let logs = capture_logs(|| left = shell_bounds_limited(&world, &atlas, limit).left);
        (left, errors(&logs, "walk=\"bounds\""))
    };
    assert_eq!(run_bounds(100), (0, 1));
    assert_eq!(run_bounds(1_002), (-5, 0));
}

/// 指摘 130: 当たり判定の持ち込みも、入るときに面の行の全部を引く。旧い単位なら予算 100 で面 1 に入って領域を
/// 99 本持ち込むが、入る 1＋行 1,001 が越えるので 1 本も持ち込まない。入る 1,002＋渡す 1,000＝2,002 なら全部。
#[test]
fn hit_import_counts_the_rows_of_an_entered_surface() {
    let (world, _) = child_with_rows(0, &rows(1000, "collision", ",0,0,1,1,R"));
    let master = world.surface(0).expect("面 0");
    let run = |limit: usize| {
        let mut list = Vec::new();
        let logs =
            capture_logs(|| list = regions_of(&world, master, &mut Budget::new(limit), None));
        (list.len(), errors(&logs, "walk=\"hit_import\""))
    };
    assert_eq!(run(100), (0, 1));
    assert_eq!(run(2_002), (1000, 0));
}

/// 指摘 137: 渡す列を予算で切るときは奥（持ち込み）から落とし、直接の領域（いちばん手前）を残す。面 1 は
/// 直接の領域 `Direct` と、`G1`・`G2` を持つ面 2 を置く。予算 9（面 1 に入る 3・面 2 に入る 3・面 2 から渡す 2）で
/// 面 1 から渡せるのは 3 本のうち 1 本だけ＝`Direct`。
#[test]
fn hit_import_drops_the_far_regions_first() {
    let text = "charset,UTF-8\nsurface0\n{\nelement0,overlay,1,0,0\n}\n\
                surface1\n{\nelement0,overlay,2,0,0\ncollision0,0,0,1,1,Direct\n}\n\
                surface2\n{\ncollision0,0,0,1,1,G1\ncollision1,0,0,1,1,G2\n}\n";
    let world = EmoWorld::build(&parse(text));
    let master = world.surface(0).expect("面 0");
    let mut list = Vec::new();
    let logs = capture_logs(|| list = regions_of(&world, master, &mut Budget::new(9), None));
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Direct"], "{logs}");
    assert_eq!(errors(&logs, "walk=\"hit_import\""), 1, "{logs}");
}

/// 指摘 134: 描かない描画メソッドの命令は重ねないので、重ねる画素の上限に数えない。外形 2×2（上限 256 画素）に
/// 描かない 2×2 の命令 100 枚（数えれば 400 画素）と描く命令 1 枚を並べても、1 枚も捨てない。
#[test]
fn undrawn_ops_count_no_pixels() {
    let (world, atlas) = child_with_rows(0, "");
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut drawn = Vec::new();
    derive_ops(
        &mut drawn,
        &mut PlanScratch::default(),
        &world,
        &atlas,
        1,
        &binds,
        &pattern,
    );
    assert_eq!(drawn.len(), 1, "検体: 絵 1 枚の命令");
    let undrawn = crate::BlitOp {
        method: ComposeMethod::Replace,
        ..drawn[0].clone()
    };
    let mut ops = vec![undrawn; 100];
    ops.push(drawn[0].clone());
    let extent = world.extent(1);
    assert_eq!((extent.w, extent.h), (2, 2), "検体の外形");
    assert_eq!(crate::blit::cap_pixels(&mut ops, extent, &atlas), 0);
    assert_eq!(ops.len(), 101);
}

/// 指摘 131: 波括弧が置き換える collision の前の行は畳み込みの最後に 1 度だけ除く。波括弧・追記・波括弧・追記の
/// 順に同じ番号を足しても、並びは「最後の波括弧より前の同じ番号を除き、後は書いた順」のまま。
#[test]
fn brace_replaces_earlier_collisions_once_at_the_end() {
    let text = "charset,UTF-8\n\
        surface0\n{\ncollision0,0,0,1,1,A\ncollision1,0,0,1,1,B\n}\n\
        surface.append0\n{\ncollision0,0,0,1,1,C\n}\n\
        surface0\n{\ncollision0,0,0,1,1,D\ncollision0,0,0,1,1,E\n}\n\
        surface.append0\n{\ncollision0,0,0,1,1,F\ncollision1,0,0,1,1,G\n}\n\
        surface0\n{\ncollision1,0,0,1,1,H\n}\n";
    let world = EmoWorld::build(&parse(text));
    let names: Vec<&str> = world
        .surface(0)
        .expect("面 0")
        .collisions
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, ["D", "E", "F", "H"]);
}

/// 指摘 136: 重なりの `warn!` は（面, 記述子）ごとに 1 行で、重なった数を `count` に書く。2 つ目の波括弧が element
/// 10 行と animation 5 つを重ねても、element と animation で 1 行ずつ。
#[test]
fn overlap_warns_once_per_descriptor_with_the_count() {
    let body = format!(
        "{}{}",
        rows(10, "element", ",overlay,s.png,0,0"),
        (0..5)
            .map(|a| format!(
                "animation{a}.interval,never\nanimation{a}.pattern0,overlay,-1,0,0,0\n"
            ))
            .collect::<String>()
    );
    let text = format!("charset,UTF-8\nsurface0\n{{\n{body}}}\nsurface0\n{{\n{body}}}\n");
    let logs = capture_logs(|| drop(EmoWorld::build(&parse(&text))));
    let warns = |needle: &str| -> Vec<&str> {
        logs.lines()
            .filter(|l| l.contains("level=WARN") && l.contains(needle))
            .collect()
    };
    let element = warns("element番号の二重定義");
    assert_eq!(element.len(), 1, "{logs}");
    assert!(
        element[0].contains("count=10") && element[0].contains("element=0"),
        "{logs}"
    );
    let animation = warns("animation番号の二重定義");
    assert_eq!(animation.len(), 1, "{logs}");
    assert!(
        animation[0].contains("count=5") && animation[0].contains("animation_id=0"),
        "{logs}"
    );
}

/// 指摘 132: 同じ子を指す element定義が多くても、（子, 親）の組ごとに 1 度だけたどる。面 0 が面 1 を 1,000 行で
/// 置き、面 1 … 200 が鎖になる（循環は無い）。報告は空で、`reaches` を呼ぶのは組の数の 200 回（行ごとに
/// たどり直すと 1,199 回）。鎖そのものの深さの 2 乗は design の「未解決」。
#[test]
fn nest_report_walks_each_child_parent_pair_once() {
    let mut text = String::from("charset,UTF-8\nsurface0\n{\n");
    text.push_str(&rows(1_000, "element", ",overlay,1,0,0"));
    text.push_str("}\n");
    for i in 1..200 {
        text.push_str(&format!(
            "surface{i}\n{{\nelement0,overlay,{},0,0\n}}\n",
            i + 1
        ));
    }
    text.push_str("surface200\n{\ncollision0,0,0,1,1,A\n}\n");
    let world = EmoWorld::build(&parse(&text));
    crate::nesting::REACHES_CALLS.with(|n| n.set(0));
    assert!(world.nest_report().issues.is_empty());
    assert_eq!(crate::nesting::REACHES_CALLS.with(|n| n.get()), 200);
}
