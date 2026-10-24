//! 配置の準備が descript の既定の面と動かせない指定を 1 度読み、準備の結果に載せることの檻
//! （descript-window-keys 要件 2.1・2.2・2.5・4.1）。

use std::collections::BTreeSet;
use std::fs;

use super::resolver::SizePx;
use super::shared_test_support::{TempDir, WA, synth_declared_dpi_ghost, with_com_initialized};
use super::*;

/// emo2 の面 0（surface0.png）の原寸。
const SURFACE0: SizePx = SizePx { w: 434, h: 687 };

/// 相方側の既定の面を 0・ゴーストの動かせない指定を `true` にした合成ゴーストを準備すると、
/// 準備の結果の既定の面は測りに使った値（相方側の窓が面 0 の大きさ）で、動かせない指定も載る。
#[test]
fn prepare_carries_default_surfaces_used_by_measure_and_balloon_dontmove() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "96", "96", None);
        fs::write(
            ghost_root.join("ghost/master/descript.txt"),
            "charset,UTF-8\nname,えも\nsakura.name,むらさき\nkero.name,エモ\n\
             kero.seriko.defaultsurface,0\nballoon.dontmove,true\n",
        )
        .expect("ghost descript");

        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
            .expect("合成ゴーストの準備は成功する");

        assert_eq!(p.default_surfaces.surface(0), 0);
        assert_eq!(p.default_surfaces.surface(1), 0, "相方側の descript の値");
        assert_eq!(
            p.placements[1].char_size, SURFACE0,
            "測りは準備の結果に載せた既定の面（面 0）で合成した"
        );
        assert_eq!(p.balloon_dontmove, BTreeSet::from([0, 1]));
    });
}

/// 何も書かれていなければ組み込みの既定の面と空の動かせない指定が載る（要件 2.4・4.1）。
#[test]
fn prepare_carries_builtin_values_when_descript_is_silent() {
    with_com_initialized(|| {
        let root = TempDir::new();
        let (ghost_root, balloon_dir) = synth_declared_dpi_ghost(&root, "96", "96", None);
        let p = prepare_ghost_windows_with_work_area(&ghost_root, &balloon_dir, WA, Some(96))
            .expect("合成ゴーストの準備は成功する");
        assert_eq!(p.default_surfaces, DefaultSurfaces::builtin());
        assert!(p.balloon_dontmove.is_empty());
    });
}
