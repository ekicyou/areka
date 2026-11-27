//! emo2 の面で、計画の絵の項の順・位置・外形が CPU の命令の列と同じことを固める。

use super::test_support::{bake_emo2, parse_emo2_shell, shell_master_dir};
use super::{BindSet, MemoryDecoder, PatternState};
use crate::plan_test_support::{
    bound_world, draw_plan_of, frame, op_images, opaque_wxh, plan_images, plan_of,
};
use crate::{Mix, PlanItem};

/// emo2 の全部の element の絵を 4×4 の不透明で焼いた表と面の表（どの element も項になる）。
fn emo2_all_drawn() -> (crate::EmoWorld, areka_emo_atlas::AtlasTable) {
    let shell = parse_emo2_shell();
    let base = shell_master_dir();
    let mut dec = MemoryDecoder::new();
    for e in shell.surfaces.iter().flat_map(|s| &s.elements) {
        let (w, h, stride, bgra, alpha) = opaque_wxh(4, 4, 10, 20, 30);
        dec.insert(base.join(e.path.as_str()), w, h, stride, bgra, alpha);
    }
    let atlas = bake_emo2(&shell, &base, &dec);
    (bound_world(&shell, &[], &atlas), atlas)
}

/// surface0（静的）・surface1000 の着せ替え・まばたきのコマで、計画が CPU の命令の列と一致する。
#[test]
fn emo2_plan_matches_cpu_ops() {
    let (world, atlas) = emo2_all_drawn();
    let dress = BindSet::from_ids([1100, 1200, 1302, 1400]);
    let mut blink = PatternState::default();
    blink.set(1400, frame(1412, 0, 0));
    let cases = [
        (0, BindSet::default(), PatternState::default()),
        (1000, dress.clone(), PatternState::default()),
        (1000, dress, blink),
    ];
    for (id, binds, pattern) in &cases {
        let (ops, extent) = plan_of(&world, &atlas, *id, binds, pattern);
        assert!(!ops.is_empty(), "検体: 面 {id} は命令を持つ");
        let plan = draw_plan_of(&world, &atlas, *id, binds, pattern).expect("計画を組める");
        assert_eq!(plan_images(&plan), op_images(&ops), "面 {id}");
        assert_eq!(plan.extent, extent.expect("外形"), "面 {id}");
        assert!(
            plan.items.iter().all(|i| matches!(
                i,
                PlanItem::Image {
                    mix: Mix::SourceOver,
                    range: None,
                    ..
                }
            )),
            "面 {id}: overlay だけ"
        );
    }
    let (with_blink, _) = plan_of(&world, &atlas, 1000, &cases[2].1, &cases[2].2);
    let (without, _) = plan_of(&world, &atlas, 1000, &cases[1].1, &cases[1].2);
    assert_ne!(
        op_images(&with_blink),
        op_images(&without),
        "検体: コマで命令が変わる"
    );
}
