//! 入れ子をたどる走査が、深い鎖で落ちず、菱形の広がりで固まらないこと（指摘 3・要件 4.4・4.5）。
//!
//! 合成・外形・シェルの外形・当たり判定の持ち込み・見える部品／到達の確かめへ同じ検体を通す。どれも
//! 予算の値に依らない: 深い鎖は予算の記録を見ず、菱形（2^30 の道）は必ず切れる。

use std::path::Path;

use areka_emo_atlas::{AtlasTable, SetId};
use areka_parsers::shell::parse;

use crate::bind::BindSet;
use crate::composed::ComposedSurface;
use crate::log_capture::capture_logs;
use crate::pattern::PatternState;
use crate::plan::compute_extent;
use crate::plan_test_support::bake_atlas;
use crate::world::EmoWorld;
use crate::{Composer, shell_bounds};

/// 再帰なら 1 段ごとにスタックを食い、256 KiB では深い鎖であふれる。
const SMALL_STACK: usize = 256 * 1024;

/// `f` をスタック 256 KiB のスレッドで回す（あふれればプロセスごと落ちる）。
fn on_small_stack(f: impl FnOnce() + Send) {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(SMALL_STACK)
            .spawn_scoped(s, f)
            .expect("スレッドを起こせる")
            .join()
            .expect("走査は落ちない");
    });
}

/// 面 0 … `last - 1` がそれぞれ次の面を `places` か所に置き、面 `last` が絵 `s.png` を持つシェル。
fn world_of(last: u32, places: u32) -> (EmoWorld, AtlasTable) {
    let mut text = String::from("charset,UTF-8\n");
    for i in 0..last {
        text.push_str(&format!("surface{i}\n{{\n"));
        for p in 0..places {
            text.push_str(&format!("element{p},overlay,{},{p},0\n", i + 1));
        }
        text.push_str("}\n");
    }
    text.push_str(&format!(
        "surface{last}\n{{\nelement0,overlay,s.png,0,0\n}}\n"
    ));
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let mut world = EmoWorld::build(&parse(&text));
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas)
}

/// 面 0 を合成する（命令列は合成器の作業場から写して返す）。
fn compose(composer: &mut Composer, world: &EmoWorld, atlas: &AtlasTable) -> Vec<crate::BlitOp> {
    let mut out = ComposedSurface::new(0, 0);
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    composer
        .compose_into(&mut out, world, atlas, 0, &binds, &pattern)
        .expect("合成できる");
    composer.ops.clone()
}

/// 深さ: 面 0 → 1 → … → 10000 の element定義の鎖を 5 つの走査が小さなスタックで回りきる。
#[test]
fn deep_chain_walks_finish_on_a_small_stack() {
    const LAST: u32 = 10_000;
    on_small_stack(|| {
        // 当たり判定の持ち込みは組み立て（build）の中で全部の面を根に回る。
        let (world, atlas) = world_of(LAST, 1);
        let ops = compose(&mut Composer::new(), &world, &atlas);
        assert_eq!(ops.len(), 1, "鎖の先の絵 1 枚");
        let extent = compute_extent(&world, &atlas, 0);
        assert_eq!((extent.w, extent.h), (2, 2));
        shell_bounds(&world, &atlas);
        let mut parts = Vec::new();
        world.nest_table().visible_parts(
            0,
            &BindSet::default(),
            &PatternState::default(),
            &mut parts,
        );
        assert_eq!(parts.len(), LAST as usize, "面 1 … 10000 が部品");
    });
}

/// 広がり: 30 段の菱形（2^30 の道）を予算で切り、`error!` は範囲ごとに 1 行・2 回の合成は同じ命令列。
#[test]
fn diamond_walks_are_cut_once_per_scope_and_compose_the_same() {
    const LAST: u32 = 30;
    let mut world = None;
    let build = capture_logs(|| world = Some(world_of(LAST, 2)));
    let (world, atlas) = world.expect("組める");
    assert_once_per_scope(&build, "hit_import");
    // 外形は束縛のとき（`bind_atlas`）に根ごとに 1 度だけたどる（指摘 8）。
    assert_once_per_scope(&build, "extent");

    let mut composer = Composer::new();
    let mut first = Vec::new();
    let logs = capture_logs(|| first = compose(&mut composer, &world, &atlas));
    assert_eq!(errors(&logs, "plan", 0), 1, "{}", errors_only(&logs));
    assert_eq!(
        errors(&logs, "extent", 0),
        0,
        "合成は外形をたどらない: {logs}"
    );
    let mut second = Vec::new();
    let logs = capture_logs(|| second = compose(&mut composer, &world, &atlas));
    assert_eq!(
        errors(&logs, "plan", 0),
        0,
        "同じ合成器の 2 回目は記録を増やさない（指摘 24）: {}",
        errors_only(&logs)
    );
    assert!(!first.is_empty());
    assert_eq!(first, second, "2 回の合成の命令列が同じ");

    let logs = capture_logs(|| {
        compute_extent(&world, &atlas, 0);
    });
    assert_eq!(errors(&logs, "extent", 0), 1, "{}", errors_only(&logs));

    let logs = capture_logs(|| {
        shell_bounds(&world, &atlas);
    });
    assert_once_per_scope(&logs, "bounds");
    assert_eq!(
        errors(&logs, "extent", 0),
        0,
        "シェルの外形は表を引く: {logs}"
    );

    // 到達の確かめは各面を 1 度しか見ないので菱形でも切れず、循環も無い。
    let logs = capture_logs(|| assert!(world.nest_report().issues.is_empty()));
    assert!(!logs.contains("level=ERROR"), "{logs}");
}

/// `walk` の `error!` が根ごとに高々 1 行、根の面 0 では必ず 1 行。
fn assert_once_per_scope(logs: &str, walk: &str) {
    assert_eq!(errors(logs, walk, 0), 1, "{walk}: {}", errors_only(logs));
    for root in 1..=30 {
        assert!(
            errors(logs, walk, root) <= 1,
            "{walk} {root}: {}",
            errors_only(logs)
        );
    }
}

fn errors(logs: &str, walk: &str, root: u32) -> usize {
    let (walk, root) = (format!("walk=\"{walk}\""), format!("root={root} "));
    logs.lines()
        .filter(|l| l.contains("level=ERROR") && l.contains(&walk) && l.contains(&root))
        .count()
}

fn errors_only(logs: &str) -> String {
    logs.lines()
        .filter(|l| l.contains("level=ERROR"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `insert` の連鎖: 面 0 の有効な純 `bind` 0 … 4999 が順に次を `insert` し、最後が絵を描く。
#[test]
fn insert_chain_composes_on_a_small_stack() {
    const LAST: u32 = 4_999;
    let mut text = String::from("charset,UTF-8\nsurface0\n{\nelement0,overlay,s.png,0,0\n");
    for i in 0..LAST {
        text.push_str(&format!(
            "animation{i}.interval,bind\nanimation{i}.pattern0,insert,{}\n",
            i + 1
        ));
    }
    text.push_str(&format!(
        "animation{LAST}.interval,bind\nanimation{LAST}.pattern0,overlay,1,0,0,0\n}}\n\
         surface1\n{{\nelement0,overlay,s.png,0,0\n}}\n"
    ));
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let mut world = EmoWorld::build(&parse(&text));
    world.bind_atlas(&atlas, SetId(0));
    on_small_stack(|| {
        let mut out = ComposedSurface::new(0, 0);
        let binds = BindSet::from_ids(0..=LAST);
        let mut composer = Composer::new();
        composer
            .compose_into(
                &mut out,
                &world,
                &atlas,
                0,
                &binds,
                &PatternState::default(),
            )
            .expect("合成できる");
        assert_eq!(composer.ops.len(), 2, "面 0 の絵と、連鎖の先の面 1 の絵");
    });
}

/// 覚え（きれいに終わった面の答え）: 全部の面を根に回す 3 つの走査で、根の間で覚えを共有した答えと記録が、
/// 覚えを使わない答えと記録に一致する。予算を 0 から全部入れる数の先まで 1 つずつ変え、残りが覚えた分に
/// 足りず覚えを使えない場合と、循環（面 4 ⇄ 5）の下で覚えない場合を含める。
#[test]
fn memo_gives_the_same_answers_and_records_as_walking_again() {
    use std::collections::HashSet;

    use crate::budget::Budget;
    use crate::hit_import::regions_of;
    use crate::plan::{Extent, compute_extent_limited};
    use crate::plan_bounds::{BoundsScratch, walk};

    let text = "charset,UTF-8\n\
        surface0\n{\nelement0,overlay,1,0,0\nelement1,overlay,1,3,-2\nelement2,overlay,4,0,0\n\
        collision0,0,0,5,5,A\n}\n\
        surface1\n{\nelement0,overlay,2,1,1\nelement1,overlay,2,-4,0\ncollision0,0,0,2,2,B\n}\n\
        surface2\n{\nelement0,overlay,3,0,7\nelement1,overlay,3,2,2\ncollision0,1,1,3,3,C\n\
        animation0.interval,bind\nanimation0.pattern0,overlay,3,5,5\n}\n\
        surface3\n{\nelement0,overlay,s.png,0,0\ncollision0,0,0,1,1,D\ncollision1,0,0,1,1,C\n}\n\
        surface4\n{\nelement0,overlay,5,1,0\nelement1,overlay,2,0,3\n}\n\
        surface5\n{\nelement0,overlay,4,0,1\nelement1,overlay,3,-6,0\n}\n";
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let mut world = EmoWorld::build(&parse(text));
    world.bind_atlas(&atlas, SetId(0));
    let ids: Vec<u32> = world.surface_ids().collect();
    let mut hits = [0; 3];
    for limit in 0..=40 {
        let (mut extent_memo, mut bounds_memo, mut hit_memo) = Default::default();
        let mut scratch = BoundsScratch::default();
        for &id in &ids {
            let mut again = (Extent { w: 0, h: 0 }, None, Vec::new());
            let again_logs = capture_logs(|| {
                again.0 =
                    compute_extent_limited(&mut HashSet::new(), &world, &atlas, id, limit, None);
                let fresh = &mut BoundsScratch::default();
                again.1 = walk(fresh, None, &world, &atlas, id, limit);
                let master = world.surface(id).expect("面");
                again.2 = regions_of(&world, master, &mut Budget::new(limit), None);
            });
            let mut memo = (Extent { w: 0, h: 0 }, None, Vec::new());
            let memo_logs = capture_logs(|| {
                memo.0 = compute_extent_limited(
                    &mut HashSet::new(),
                    &world,
                    &atlas,
                    id,
                    limit,
                    Some(&mut extent_memo),
                );
                memo.1 = walk(
                    &mut scratch,
                    Some(&mut bounds_memo),
                    &world,
                    &atlas,
                    id,
                    limit,
                );
                let master = world.surface(id).expect("面");
                memo.2 = regions_of(&world, master, &mut Budget::new(limit), Some(&mut hit_memo));
            });
            assert_eq!(memo, again, "予算 {limit}・根 {id}");
            assert_eq!(memo_logs, again_logs, "予算 {limit}・根 {id}");
        }
        // 循環の下（面 4・5）は 3 つの覚えのどれにも入らない。
        for cyc in [4, 5] {
            assert!(!extent_memo.contains_key(&cyc), "外形・面 {cyc}");
            assert!(!bounds_memo.contains_key(&cyc), "シェルの外形・面 {cyc}");
            assert!(!hit_memo.contains_key(&cyc), "持ち込み・面 {cyc}");
        }
        hits[0] += usize::from(extent_memo.contains_key(&1));
        hits[1] += usize::from(bounds_memo.contains_key(&1));
        hits[2] += usize::from(hit_memo.contains_key(&1));
    }
    assert!(
        hits.iter().all(|&n| n > 0),
        "予算が足りれば 3 つとも覚える: {hits:?}"
    );
}

/// 面 0 … `last - 1` がそれぞれ次の面を 2 か所に置き（菱形）、面 `last` が絵 `s.png` を `leaves` 枚持つシェル。
fn diamond_with_leaves(last: u32, leaves: u32) -> (EmoWorld, AtlasTable) {
    let mut text = String::from("charset,UTF-8\n");
    for i in 0..last {
        text.push_str(&format!(
            "surface{i}\n{{\nelement0,overlay,{0},0,0\nelement1,overlay,{0},1,0\n}}\n",
            i + 1
        ));
    }
    text.push_str(&format!("surface{last}\n{{\n"));
    for p in 0..leaves {
        text.push_str(&format!("element{p},overlay,s.png,{p},0\n"));
    }
    text.push_str("}\n");
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let mut world = EmoWorld::build(&parse(&text));
    world.bind_atlas(&atlas, SetId(0));
    (world, atlas)
}

/// 段の積み荷（指摘 21）: 小さな予算の命令列は、葉の絵の多い菱形でも命令の数が予算を越えない
/// （根の面 0 は子へ入る積み荷だけ）。`error!` は 1 行。
#[test]
fn plan_counts_the_layer_payload_against_the_budget() {
    let (world, atlas) = diamond_with_leaves(6, 50);
    let mut ops = Vec::new();
    let logs = capture_logs(|| {
        let mut scratch = crate::plan::PlanScratch::with_limit(64);
        let (binds, pattern) = (BindSet::default(), PatternState::default());
        crate::plan::derive_ops(&mut ops, &mut scratch, &world, &atlas, 0, &binds, &pattern);
    });
    assert!(!ops.is_empty());
    assert!(ops.len() <= 64, "命令 {} 個は予算 64 以下", ops.len());
    assert_eq!(errors(&logs, "plan", 0), 1, "{}", errors_only(&logs));
}

/// 段の積み荷（指摘 21）: 葉に絵 1,000 枚の 15 段の菱形（入る面は予算の内側）でも、本番の予算の合成の
/// 命令の数は予算以下（直しが無いと約 3,200 万個）。
#[test]
fn diamond_with_many_leaf_images_composes_within_the_budget() {
    let (world, atlas) = diamond_with_leaves(15, 1_000);
    let ops = compose(&mut Composer::new(), &world, &atlas);
    assert!(!ops.is_empty());
    assert!(
        ops.len() <= crate::budget::VISIT_BUDGET,
        "命令 {} 個は予算以下",
        ops.len()
    );
}

/// 持ち込みの数（指摘 22）: 面 0 が子 1（当たり判定 300 個）を 300 行置くと、直しが無ければ 9 万個を
/// 持ち込む。取り込む領域も予算から引くので、持ち込みの列は予算で止まり `error!` は 1 行。
#[test]
fn hit_import_counts_the_imported_regions_against_the_budget() {
    const N: u32 = 300;
    let mut text = String::from("charset,UTF-8\nsurface0\n{\n");
    for p in 0..N {
        text.push_str(&format!("element{p},overlay,1,{p},0\n"));
    }
    text.push_str("}\nsurface1\n{\n");
    for c in 0..N {
        text.push_str(&format!("collision{c},0,0,5,5,R{c}\n"));
    }
    text.push_str("}\n");
    let mut world = None;
    let logs = capture_logs(|| world = Some(EmoWorld::build(&parse(&text))));
    let world = world.expect("組める");
    let imported = world.hit_regions(0).map_or(0, <[_]>::len);
    assert!(imported > 0, "持ち込みはある");
    assert!(
        imported <= crate::budget::VISIT_BUDGET,
        "持ち込み {imported} 個は予算以下"
    );
    assert_eq!(errors(&logs, "hit_import", 0), 1, "{}", errors_only(&logs));
}

/// 面積の上限（指摘 23）: 一辺は上限の内でも、面積が 2^25 画素を越える外形は `ExtentTooLarge`。
/// ちょうど 2^25 画素（8192×4096）は通る。
#[test]
fn extent_over_the_area_limit_is_too_large() {
    let plan = |w: i64, h: i64| {
        // 2×2 の絵を (w-2, h-2) に置く子で、外形を w×h にする。
        let text = format!(
            "charset,UTF-8\nsurface0\n{{\nelement0,overlay,1,{},{}\n}}\n\
             surface1\n{{\nelement0,overlay,s.png,0,0\n}}\n",
            w - 2,
            h - 2
        );
        let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
        let mut world = EmoWorld::build(&parse(&text));
        world.bind_atlas(&atlas, SetId(0));
        let (binds, pattern) = (BindSet::default(), PatternState::default());
        let mut scratch = crate::plan::PlanScratch::default();
        let mut ops = Vec::new();
        crate::plan::build_plan(&mut ops, &mut scratch, &world, &atlas, 0, &binds, &pattern)
    };
    assert_eq!(
        plan(8192, 4096),
        Ok(crate::plan::Extent { w: 8192, h: 4096 })
    );
    assert_eq!(
        plan(8192, 4097),
        Err(crate::ComposeError::ExtentTooLarge {
            surface: 0,
            w: 8192,
            h: 4097
        })
    );
}
