//! 形ごとの当たりの実 GPU のテスト。

use areka_emo_atlas::WicDecoderArm;

use super::test_support::{
    attach_and_show, capture, five_shapes_assets, hit_generation, make_world_with_gpu,
    mask_pictures_created, name_at, warn_count,
};
use super::{EmoPresenter, TargetId};
use crate::shell_target::test_support::claudia_shell_dir;

const T: TargetId = TargetId(0);

#[test]
fn claudia_face_0_resolves_ellipse_polygons_and_rect_names() {
    let mut world = make_world_with_gpu();
    let dec = WicDecoderArm::new().expect("WIC ファクトリが作れる");
    let mut shell = crate::shell_target::load_shell_target(&claudia_shell_dir(), &dec)
        .expect("claudia のシェルは読める");
    let assets = (shell.take_world(), shell.atlas().clone());
    let mut presenter = EmoPresenter::new();
    attach_and_show(&mut presenter, &mut world, T, assets, 0, 96);

    for (x, y, want) in [
        (152, 483, "Foot"),
        (250, 228, "Fan"),
        (150, 435, "Skirt"),
        (165, 70, "Head"),
    ] {
        assert_eq!(name_at(&presenter, T, x, y), Some(want), "({x},{y})");
    }
}

#[test]
fn five_shapes_resolve_at_every_display_scale() {
    for (dpi, k) in [(96, 1), (192, 2)] {
        let mut world = make_world_with_gpu();
        let mut presenter = EmoPresenter::new();
        attach_and_show(&mut presenter, &mut world, T, five_shapes_assets(), 0, dpi);

        let (names, events) = capture(|| {
            [(4, 8), (24, 8), (40, 8), (56, 8), (76, 4), (70, 14)]
                .map(|(x, y)| name_at(&presenter, T, x * k, y * k))
        });
        assert_eq!(
            names,
            [Some("R"), Some("B"), Some("E"), Some("C"), Some("P"), None],
            "dpi {dpi}"
        );
        assert_eq!(warn_count(&events), 0, "{events:?}");
        assert_eq!(hit_generation(&presenter, T), Some(1));
        assert_eq!(mask_pictures_created(&presenter, T), 1);
    }
}
