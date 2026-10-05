//! 動く絵の再生の検体が今の読み手で読めること（spec: areka-P0-animated-image-playback
//! task 1.2・要件 9.4）。
//!
//! 検体 `areka-emo-compose/tests/fixtures/animated-playback/`（中身は同じフォルダの README）の
//! シェルを本物の読み手 [`WicDecoderArm`] と [`load_shell_target`] で、バルーンの面を
//! [`resolve_balloon_faces`] → [`build_balloon_target_from_faces`] で読み、失敗も `warn!` も
//! 出ないことと、写した絵が README のとおりのコマ・待ち時間・繰り返しで焼けることを確かめる。
//! 再生の意味を確かめるのは後続のタスクで、ここは検体そのものの番をするだけである。

use super::*;

use std::num::NonZeroU32;

use areka_emo_atlas::{LoopCount, WicDecoderArm};

use super::test_support::{capture_events, with_com_initialized};
use crate::balloon::{build_balloon_target_from_faces, resolve_balloon_faces};

fn fixture_dir(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../areka-emo-compose/tests/fixtures/animated-playback")
        .join(sub)
}

/// 焼いた動く絵の（コマの数・待ち時間・繰り返し）。
fn film(atlas: &AtlasTable, name: &str) -> (usize, Vec<u32>, LoopCount) {
    let id = atlas
        .resolve(SetId(0), name)
        .unwrap_or_else(|| panic!("{name} は焼かれているはず"));
    let anim = atlas
        .animation(id)
        .unwrap_or_else(|| panic!("{name} は動く絵のはず"));
    (anim.frames.len(), anim.delays_ms.clone(), anim.loop_count)
}

fn finite(n: u32) -> LoopCount {
    LoopCount::Finite(NonZeroU32::new(n).unwrap())
}

#[test]
fn the_shell_loads_with_every_film_and_no_warnings() {
    with_com_initialized(|| {
        let dir = fixture_dir("shell");
        let arm = WicDecoderArm::new().expect("WIC の工場が作れる");
        let (target, events) =
            capture_events(|| load_shell_target(&dir, &arm).expect("検体のシェルは読める"));

        let warns: Vec<_> = events
            .iter()
            .filter(|e| e.level <= tracing::Level::WARN)
            .map(|e| e.message().to_string())
            .collect();
        assert!(warns.is_empty(), "warn!/error! が出た: {warns:?}");
        assert!(
            target.bake_errors().is_empty(),
            "{:?}",
            target.bake_errors()
        );
        assert!(
            target.nest_report.issues.is_empty(),
            "{:?}",
            target.nest_report
        );
        assert!(target.dangling.is_empty(), "{:?}", target.dangling);

        let atlas = target.atlas();
        let infinite2 = (2, vec![100, 100], LoopCount::Infinite);
        assert_eq!(film(atlas, "rgb.apng"), infinite2);
        assert_eq!(film(atlas, "rgb.webp"), infinite2);
        assert_eq!(film(atlas, "alpha.webp"), (3, vec![100, 0, 70], finite(3)));
        assert_eq!(
            film(atlas, "surface1.png"),
            (4, vec![333, 0, 70, 1], finite(2)),
            "surface1.png（中身は basic.apng）が面 1 の土台"
        );

        // 面 2 は element0 が在るので、動く surface2.png は土台に使わない（焼かない）。
        assert_eq!(
            target
                .base_images
                .shadowed
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(atlas.resolve(SetId(0), "surface2.png").is_none());

        let world = target.build_world();
        let ids: BTreeSet<u32> = world.surface_ids().collect();
        assert_eq!(ids, BTreeSet::from([0, 1, 2, 3, 10, 11, 20, 30, 31]));
    });
}

#[test]
fn the_balloon_faces_resolve_and_bake_as_films() {
    with_com_initialized(|| {
        let dir = fixture_dir("balloon");
        let faces = resolve_balloon_faces(&dir, 0).expect("面 0 は解決できる");
        assert_eq!(
            faces.iter().map(|f| f.surface_id).collect::<Vec<_>>(),
            vec![0, 1]
        );

        let arm = WicDecoderArm::new().expect("WIC の工場が作れる");
        let Ok((_world, atlas)) = build_balloon_target_from_faces(&dir, &arm, &faces) else {
            panic!("バルーンの面は焼ける");
        };
        assert_eq!(
            film(&atlas, "balloons0.png"),
            (2, vec![100, 100], LoopCount::Infinite)
        );
        assert_eq!(
            film(&atlas, "balloons1.png"),
            (3, vec![100, 0, 70], finite(3))
        );
    });
}
