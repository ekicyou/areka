//! 計画を組む口が `overlay` だけの面で CPU の命令の列と同じ順・位置・外形・上限を出す檻。

use std::path::Path;

use areka_parsers::shell::{BLEND_ELEMENT_WORDS, parse};

use super::{Mix, PlanItem};
use crate::bind::BindSet;
use crate::error::ComposeError;
use crate::log_capture::capture_logs;
use crate::pattern::PatternState;
use crate::plan_test_support::{
    bake_atlas, bound_world, draw_plan_of, elem, elem_with_method, errors, frame, id_to_path,
    named_ops, op_images, plan_images, plan_of, shell_of, surface,
};
use crate::world::EmoWorld;
use crate::{Composer, DrawPlan};

const RELS: &[&str] = &["a.png", "b.png", "c.png", "d.png", "e.png", "f.png"];

/// 層 (i) の絵 2 枚と element定義の子の面、層 (ii) の着せ替え 10・`always` 20・今のコマ 30 を持つ面 0。
const LAYERED: &str = "charset,UTF-8\n\
    surface0\n{\n\
    element0,overlay,a.png,0,0\n\
    element1,overlay,b.png,5,6\n\
    element2,overlay,1,3,4\n\
    animation30.interval,random,2\n\
    animation30.pattern0,overlay,5,100,0,0\n\
    animation20.interval,always\n\
    animation20.pattern0,overlay,4,0,2,2\n\
    animation10.interval,bind\n\
    animation10.pattern0,overlay,2,0,1,1\n\
    }\n\
    surface1\n{\nelement0,overlay,c.png,0,0\n}\n\
    surface2\n{\nelement0,overlay,d.png,0,0\n}\n\
    surface4\n{\nelement0,overlay,e.png,0,0\n}\n\
    surface5\n{\nelement0,overlay,f.png,0,0\n}\n";

fn layered() -> (EmoWorld, areka_emo_atlas::AtlasTable) {
    let atlas = bake_atlas(Path::new("shell/master"), RELS);
    (bound_world(&parse(LAYERED), &[], &atlas), atlas)
}

fn assert_all_source_over(plan: &DrawPlan) {
    for item in &plan.items {
        assert!(
            matches!(
                item,
                PlanItem::Image {
                    mix: Mix::SourceOver,
                    range: None,
                    ..
                }
            ),
            "{item:?}"
        );
    }
}

/// 層 (i) → 層 (ii) を ID の順（着せ替え 10 → `always` 20 → 今のコマ 30）に並べた絵の項が、CPU の命令の列と
/// 順・位置まで同じで、外形も同じ。
#[test]
fn layered_surface_plan_matches_cpu_ops() {
    let (world, atlas) = layered();
    let binds = BindSet::from_ids([10]);
    let mut pattern = PatternState::default();
    pattern.set(30, frame(5, 7, 8));
    let (ops, extent) = plan_of(&world, &atlas, 0, &binds, &pattern);
    let plan = draw_plan_of(&world, &atlas, 0, &binds, &pattern).expect("計画を組める");

    let map = id_to_path(&atlas, RELS);
    assert_eq!(
        named_ops(&ops, &map),
        [
            ("a.png", 0, 0),
            ("b.png", 5, 6),
            ("c.png", 3, 4),
            ("d.png", 1, 1),
            ("e.png", 2, 2),
            ("f.png", 7, 8),
        ],
        "検体: CPU の命令の列"
    );
    assert_eq!(plan_images(&plan), op_images(&ops));
    assert_all_source_over(&plan);
    assert_eq!(plan.extent, extent.expect("外形"));
    assert_eq!(plan.extent, world.extent(0));
    assert_eq!((plan.surface_id, plan.base_surface), (0, 0));
}

/// 同じ合成器で状態を替えて組み直すと、前の項を残さず今の状態の項だけになる。
#[test]
fn plan_into_rebuilds_from_empty() {
    let (world, atlas) = layered();
    let mut composer = Composer::new();
    let mut out = composer
        .plan(
            &world,
            &atlas,
            0,
            &BindSet::from_ids([10]),
            &PatternState::default(),
        )
        .expect("計画を組める");
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    composer
        .plan_into(&mut out, &world, &atlas, 0, &binds, &pattern)
        .expect("計画を組める");
    let (ops, _) = plan_of(&world, &atlas, 0, &binds, &pattern);
    assert_eq!(plan_images(&out), op_images(&ops));
    assert_eq!(out.items.len(), 4, "着せ替え 10 の項が残らない");
}

/// 外形 2×2 に 2×2 の絵を 100 枚重ねる面は、上限（面積×64）で頭の 64 枚だけが項になり、命令の列と同じ。
/// 上限越えの `error!` は合成器 1 つにつき 1 行。
#[test]
fn plan_keeps_the_pixel_limit_of_the_cpu_ops() {
    let mut text = String::from("charset,UTF-8\nsurface0\n{\n");
    for e in 0..100 {
        text.push_str(&format!("element{e},overlay,a.png,0,0\n"));
    }
    text.push_str("}\n");
    let atlas = bake_atlas(Path::new("shell/master"), &["a.png"]);
    let world = bound_world(&parse(&text), &[], &atlas);
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let (ops, _) = plan_of(&world, &atlas, 0, &binds, &pattern);
    assert_eq!(ops.len(), 64, "検体: CPU の命令は頭の 64 枚");
    let mut composer = Composer::new();
    let mut plan = None;
    let logs = capture_logs(|| {
        for _ in 0..2 {
            plan = Some(composer.plan(&world, &atlas, 0, &binds, &pattern));
        }
    });
    let plan = plan.expect("組んだ").expect("計画を組める");
    assert_eq!(plan_images(&plan), op_images(&ops));
    assert_eq!(errors(&logs, "重ねる画素"), 1, "{logs}");
}

/// 描けない語（`blend-*`）の element は CPU の命令の列には残るが重ねないので、計画の項にしない。
#[test]
fn undrawable_element_is_not_a_plan_item() {
    let word = BLEND_ELEMENT_WORDS[0];
    let shell = shell_of(vec![surface(
        0,
        vec![
            elem(0, "a.png", 0, 0),
            elem_with_method(1, "b.png", 1, 0, word),
        ],
    )]);
    let atlas = bake_atlas(Path::new("shell/master"), &["a.png", "b.png"]);
    let world = bound_world(&shell, &[], &atlas);
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    let (ops, _) = plan_of(&world, &atlas, 0, &binds, &pattern);
    assert_eq!(ops.len(), 2, "検体: {word} の命令も積まれる");
    let plan = draw_plan_of(&world, &atlas, 0, &binds, &pattern).expect("計画を組める");
    assert_eq!(plan_images(&plan), op_images(&ops[..1]));
}

/// 面が無いときの失敗は CPU の合成と同じ。
#[test]
fn missing_surface_fails_like_compose() {
    let (world, atlas) = layered();
    let (binds, pattern) = (BindSet::default(), PatternState::default());
    assert_eq!(
        draw_plan_of(&world, &atlas, 99, &binds, &pattern),
        Err(ComposeError::SurfaceNotFound(99))
    );
}
