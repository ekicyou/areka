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
    capture_logs(|| second = compose(&mut composer, &world, &atlas));
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
    let mut hits = 0;
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
        // 循環の下（面 4・5）は覚えない。
        assert!(!hit_memo.contains_key(&4) && !hit_memo.contains_key(&5));
        hits += usize::from(hit_memo.contains_key(&1));
    }
    assert!(hits > 0, "予算が足りれば覚える");
}
