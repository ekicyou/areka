//! 予算が数えなかった仕事の量（指摘 76〜79・82）: 面に入るたびになめる行・重ねる画素・範囲の見出しの写し・
//! 根をまたぐ持ち込み。どれも上限で止まり、記録は 1 度だけ出る。

use std::collections::BTreeMap;
use std::path::Path;

use areka_parsers::shell::{parse, parse_boxes};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;

use crate::bind::BindSet;
use crate::composed::ComposedSurface;
use crate::hit_import::{HitRegions, attach_hit_regions_limited};
use crate::log_capture::capture_logs;
use crate::pattern::PatternState;
use crate::plan_test_support::{bake_atlas, bound_world, errors, ops_limited};
use crate::world::{EmoWorld, Records};
use crate::{ComposeError, Composer, fold_boxes};

/// 指摘 76: 面 0 が置く面 1 は絵 1 枚と、命令にならない（絵の解決できない）element 1,000 行を持つ。入る 1 と
/// 積み荷 1 は予算 100 の内でも、なめる行（1,001）が越えるので面 1 は開かず、`error!` は合成器 1 つで 1 行。
/// 入る 1＋なめる 1,001＋積み荷 1＝1,003 なら切れず、1 つ少ない 1,002 では切れる（内訳の 3 つとも数える）。
#[test]
fn plan_counts_the_scanned_rows_of_an_entered_surface() {
    let mut text = String::from(
        "charset,UTF-8\nsurface0\n{\nelement0,overlay,1,0,0\n}\n\
         surface1\n{\nelement0,overlay,s.png,0,0\n",
    );
    for e in 1..=1000 {
        text.push_str(&format!("element{e},overlay,none.png,0,0\n"));
    }
    text.push_str("}\n");
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let world = bound_world(&parse(&text), &[], &atlas);
    let run = |limit: usize| {
        let (ops, logs) = ops_limited(&world, &atlas, 0, limit);
        (ops.len(), errors(&logs, "walk=\"plan\""))
    };
    assert_eq!(run(100), (0, 1), "なめる行で切れ、記録は 1 行");
    assert_eq!(run(1_002), (0, 1), "1 単位足りなければ切れる");
    assert_eq!(run(1_003), (1, 0), "予算の内なら絵を描き、記録は無い");
}

/// 指摘 77: 外形 2×2（面積 4）の面 0 に 2×2 の絵を 100 枚重ねる（400 画素）。重ねてよいのは面積×64＝256 画素
/// なので、命令は頭の 64 枚で止まり、`error!` は合成器 1 つで 1 行（いちからと矩形の重ね直しの両方）。
#[test]
fn compose_stops_at_the_pixel_limit_once_per_composer() {
    let mut text = String::from("charset,UTF-8\nsurface0\n{\n");
    for e in 0..100 {
        text.push_str(&format!("element{e},overlay,s.png,0,0\n"));
    }
    text.push_str("}\n");
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let world = bound_world(&parse(&text), &[], &atlas);
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let mut composer = Composer::new();
    let (mut out, mut base) = (ComposedSurface::new(0, 0), ComposedSurface::new(0, 0));
    let logs = capture_logs(|| {
        composer
            .compose_into(&mut out, &world, &atlas, 0, &binds, &pattern)
            .expect("合成できる");
        assert_eq!(composer.ops.len(), 64, "いちから: 頭の 64 枚");
        composer
            .compose_into_reusing(&mut base, None, &world, &atlas, 0, &binds, &pattern)
            .expect("合成できる");
        composer
            .compose_into_reusing(&mut out, Some(&base), &world, &atlas, 0, &binds, &pattern)
            .expect("合成できる");
        assert_eq!(composer.base_ops.len(), 64, "重ね直し: 頭の 64 枚");
    });
    assert_eq!(errors(&logs, "重ねる画素"), 1, "{logs}");
}

/// 指摘 78: 範囲の見出しで写す行の合計は畳み込み 1 回で 2^20 まで。面 0 の 1 行の後、`surface1-1024` に
/// 1,024 行を書くと合計 2^20＋1 で越えるので、その記述子だけを捨てて `error!` は 1 行（面 0 は残る）。
#[test]
fn fold_drops_the_descriptor_over_the_copied_line_limit() {
    let mut text =
        String::from("charset,UTF-8\nsurface0\n{\ncollision0,0,0,1,1,A\n}\nsurface1-1024\n{\n");
    for c in 0..1024 {
        text.push_str(&format!("collision{c},0,0,1,1,R\n"));
    }
    text.push_str("}\n");
    let mut world = None;
    let logs = capture_logs(|| world = Some(EmoWorld::build(&parse(&text))));
    let world = world.expect("組める");
    assert!(world.surface(0).is_some(), "上限の内の面 0 は残る");
    assert!(world.surface(1).is_none() && world.surface(1024).is_none());
    assert_eq!(errors(&logs, "写す行"), 1, "{logs}");
    assert!(logs.contains("lines=1024"), "{logs}");
}

/// 指摘 78: animation の中の pattern の行も、animation ごとの interval の 1 行も写す行に数える。面 0 の 1 行の後、
/// `surface1-1024` に animation 1 つ・pattern 1,023 行を書くと 1,024×(1＋1,023)＝2^20 で合計 2^20＋1 になり
/// 越えるので、その記述子を捨てて `error!` は 1 行（interval の 1 行を数え落とすと収まってしまう）。
#[test]
fn fold_counts_the_pattern_rows_as_copied_lines() {
    let mut text = String::from(
        "charset,UTF-8\nsurface0\n{\ncollision0,0,0,1,1,A\n}\nsurface1-1024\n{\nanimation0.interval,never\n",
    );
    for p in 0..1023 {
        text.push_str(&format!("animation0.pattern{p},overlay,0,0,0,0\n"));
    }
    text.push_str("}\n");
    let mut world = None;
    let logs = capture_logs(|| world = Some(EmoWorld::build(&parse(&text))));
    let world = world.expect("組める");
    assert!(world.surface(0).is_some(), "上限の内の面 0 は残る");
    assert!(world.surface(1).is_none() && world.surface(1024).is_none());
    assert_eq!(errors(&logs, "写す行"), 1, "{logs}");
}

/// 指摘 78: 写す行の合計がちょうど 2^20（`surface1-1024` に 1,024 行）なら上限の内で、記述子は残り
/// `error!` は出ない。
#[test]
fn fold_keeps_the_descriptor_at_exactly_the_copied_line_limit() {
    let mut text = String::from("charset,UTF-8\nsurface1-1024\n{\n");
    for c in 0..1024 {
        text.push_str(&format!("collision{c},0,0,1,1,R\n"));
    }
    text.push_str("}\n");
    let mut world = None;
    let logs = capture_logs(|| world = Some(EmoWorld::build(&parse(&text))));
    let world = world.expect("組める");
    assert!(world.surface(1).is_some() && world.surface(1024).is_some());
    assert_eq!(errors(&logs, "写す行"), 0, "{logs}");
}

/// 指摘 78: 箱の行も同じ上限で、越えた記述子を捨てて `error!` は 1 行。
#[test]
fn box_fold_drops_the_descriptor_over_the_copied_line_limit() {
    let mut text = String::from(
        "charset,UTF-8\nballoon.a\n{\nsize,10,10\n}\n\
         surface0\n{\nelement0,balloon,a,0,0\n}\nsurface1-1024\n{\n",
    );
    for e in 0..1024 {
        text.push_str(&format!("element{e},balloon,a,0,0\n"));
    }
    text.push_str("}\n");
    let world = EmoWorld::build(&parse("charset,UTF-8\n"));
    let mut layout = None;
    let logs = capture_logs(|| {
        layout = Some(fold_boxes(&parse_boxes(&text), &BTreeMap::new(), &world).0);
    });
    let layout = layout.expect("畳める");
    assert_eq!(layout.placements(0).len(), 1, "上限の内の面 0 は残る");
    assert!(layout.placements(1).is_empty() && layout.placements(1024).is_empty());
    assert_eq!(errors(&logs, "写す行"), 1, "{logs}");
}

/// 指摘 79: 面 0 … 9 がそれぞれ面 10（当たり判定 3 つ）を置く。根をまたぐ持ち込みの予算 10 は面 0〜2 の
/// 9 本で残り 1 になり、面 3 から先には付けない。`error!` は 1 行で、黙らせる組み方では出ない。
#[test]
fn hit_import_counts_the_regions_across_roots_against_one_budget() {
    let mut text = String::from("charset,UTF-8\n");
    for i in 0..10 {
        text.push_str(&format!("surface{i}\n{{\nelement0,overlay,10,0,0\n}}\n"));
    }
    text.push_str(
        "surface10\n{\ncollision0,0,0,1,1,A\ncollision1,0,0,1,1,B\ncollision2,0,0,1,1,C\n}\n",
    );
    for records in [Records::Emit, Records::Quiet] {
        let mut world = EmoWorld::build(&parse(&text));
        let w = world.world_mut();
        let attached: Vec<Entity> = w
            .query_filtered::<Entity, With<HitRegions>>()
            .iter(w)
            .collect();
        assert_eq!(attached.len(), 10, "本番の予算なら全部の面に付く");
        for e in attached {
            w.entity_mut(e).remove::<HitRegions>();
        }
        let logs = capture_logs(|| attach_hit_regions_limited(&mut world, records, 10));
        for i in 0..10 {
            let n = world.hit_regions(i).map_or(0, <[_]>::len);
            assert_eq!(n, if i < 3 { 3 } else { 0 }, "面 {i}");
        }
        let expected = usize::from(records.emits());
        assert_eq!(errors(&logs, "持ち込む領域"), expected, "{logs}");
    }
}

/// 指摘 82: 面 0 が 2×2 の絵を持つ面 1 を (-2, 0) に置くと外形は幅 0・高さ 2。幅か高さが 0 なら
/// `EmptyComposition`。
#[test]
fn zero_width_extent_is_empty_composition() {
    let text = "charset,UTF-8\nsurface0\n{\nelement0,overlay,1,-2,0\n}\n\
                surface1\n{\nelement0,overlay,s.png,0,0\n}\n";
    let atlas = bake_atlas(Path::new("shell/master"), &["s.png"]);
    let world = bound_world(&parse(text), &[], &atlas);
    assert_eq!((world.extent(0).w, world.extent(0).h), (0, 2), "検体の外形");
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let result = Composer::new().compose(&world, &atlas, 0, &binds, &pattern);
    assert_eq!(result.err(), Some(ComposeError::EmptyComposition(0)));
}
