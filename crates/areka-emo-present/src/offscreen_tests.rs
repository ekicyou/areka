use super::*;

use areka_emo_compose::plan::Extent;
use windows::Win32::Graphics::Direct2D::D2D1_DEVICE_CONTEXT_OPTIONS_NONE;

use crate::gpu_atlas::tests::{two_picture_atlas, warp};
use crate::presenter::test_support::{bake_world, elem, shell_of, surface};

const SURFACE: u32 = 1000;

/// 乗算済みの `w×h`。α は `alpha(x, y)`、色は座標から作って α を掛ける。
fn picture(w: u32, h: u32, salt: u8, alpha: impl Fn(u32, u32) -> u8) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let a = alpha(x, y);
            let pm = |c: u32| ((c % 256) * a as u32 / 255) as u8;
            out.extend_from_slice(&[
                pm(x * 29 + salt as u32),
                pm(y * 37 + salt as u32),
                pm((x + y) * 17 + salt as u32),
                a,
            ]);
        }
    }
    out
}

/// `overlay` だけの面: 透明・半透明・不透明の列のある土台に、半透明の段のある絵が重なる。
fn overlay_only() -> (EmoWorld, AtlasTable) {
    bake_world(
        shell_of(vec![surface(
            SURFACE,
            vec![elem("a.png", 0, 0), elem("b.png", 5, 3)],
        )]),
        vec![
            (
                "a.png",
                16,
                12,
                picture(16, 12, 11, |x, y| match (x, y) {
                    (_, 0) | (15, _) => 0,
                    (0..=3, _) => 100,
                    _ => 255,
                }),
            ),
            (
                "b.png",
                9,
                7,
                picture(9, 7, 90, |x, y| [255, 200, 128, 60][((x + y) % 4) as usize]),
            ),
        ],
    )
}

fn assert_within_one(got: &ComposedSurface, want: &ComposedSurface) {
    assert_eq!(
        (got.width(), got.height()),
        (want.width(), want.height()),
        "原寸"
    );
    for (i, (g, w)) in got.bytes().iter().zip(want.bytes()).enumerate() {
        let (px, ch) = (i / 4, i % 4);
        let (x, y) = (px as u32 % want.width(), px as u32 / want.width());
        assert!(
            g.abs_diff(*w) <= 1,
            "({x},{y}) 成分 {ch}: hardware {g}・WARP {w}"
        );
    }
}

fn render(
    renderer: &mut OffscreenRenderer,
    world: &EmoWorld,
    atlas: &AtlasTable,
) -> ComposedSurface {
    renderer
        .render(
            world,
            atlas,
            SURFACE,
            &BindSet::default(),
            &PatternState::default(),
        )
        .expect("画面の外へ描く")
}

#[test]
fn hardware_overlay_only_matches_warp_within_one() {
    let (world, atlas) = overlay_only();
    let want = render(
        &mut OffscreenRenderer::warp().expect("WARP の描き手"),
        &world,
        &atlas,
    );
    assert!(
        want.bytes().chunks(4).any(|p| p[3] != 0 && p[3] != 255),
        "見本に半透明の画素がある"
    );
    assert!(
        want.bytes().chunks(4).any(|p| p[3] == 0),
        "見本に透明の画素がある"
    );
    let got = render(
        &mut OffscreenRenderer::hardware().expect("hardware の描き手"),
        &world,
        &atlas,
    );
    assert_within_one(&got, &want);
}

#[test]
fn the_same_atlas_is_uploaded_once_across_renders() {
    let (world, atlas) = overlay_only();
    let mut renderer = OffscreenRenderer::warp().expect("WARP の描き手");
    for _ in 0..2 {
        renderer
            .render(
                &world,
                &atlas,
                SURFACE,
                &BindSet::default(),
                &PatternState::default(),
            )
            .expect("画面の外へ描く");
    }
    let gpu = renderer.gpu.as_ref().expect("控え");
    assert_eq!(
        (gpu.uploads(), gpu.recordings()),
        (1, 2),
        "頁 1 枚・葉 2 つ"
    );
}

#[test]
fn a_plan_that_cannot_be_drawn_is_a_render_error_of_its_surface() {
    let mut gpu = GpuAtlas::new(&warp(), &two_picture_atlas()).expect("GpuAtlas");
    // SAFETY: Windows API 境界。生きている装置から DC を作り、上限を読むだけ。
    let max = unsafe {
        warp()
            .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
            .expect("DC")
            .GetMaximumBitmapSize()
    };
    let plan = DrawPlan {
        surface_id: 7,
        base_surface: 7,
        extent: Extent { w: max + 1, h: 1 },
        items: Vec::new(),
    };
    let err = OffscreenRenderer::render_plan(&mut gpu, &plan).expect_err("描けない大きさ");
    assert!(
        matches!(
            err,
            PresentError::Compose(ComposeError::Render { surface: 7, .. })
        ),
        "{err:?}"
    );
}
