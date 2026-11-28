use super::*;

use areka_emo_atlas::{DecodedImage, MemoryDecoder};

use super::test_support::{CapturedEvent, gradient, paint_screen};

/// 5 つの形（`region`・矩形・楕円・円・多角形）を 16 画素幅ずつ並べた、80×16 の面の `collisionex`。
/// `region` の画像は [`FIVE_SHAPES_MASK`]。
pub(super) const FIVE_SHAPES: &str = concat!(
    "collisionex0,R,region,mask.png,255,0,0\n",
    "collisionex1,B,rect,16,0,32,16\n",
    "collisionex2,E,ellipse,32,0,48,16\n",
    "collisionex3,C,circle,56,8,8\n",
    "collisionex4,P,polygon,64,0,80,0,80,16\n",
);

/// 左 8 列が赤（指定色）・残りが青の 16×16 の乗算しない画像。
pub(super) const FIVE_SHAPES_MASK: &str = "mask.png";

/// 乗算しない BGRA の絵（`pixel` が画素ごとの BGRA を返す）。
pub(super) fn straight_image(w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> DecodedImage {
    DecodedImage {
        width: w,
        height: h,
        stride: w * 4,
        bgra: (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .flat_map(|(x, y)| pixel(x, y))
            .collect(),
        has_alpha: true,
    }
}

/// 面 0 に `body`（`collisionex` などの行）を書いたシェルを一時フォルダへ作って読み、面の表と
/// 焼いた絵を返す。面の絵は乗算済みの `face`（幅・高さ・BGRA）、`region` の画像は
/// 乗算しない絵として復号器に載せる。
pub(super) fn load_hit_shell(
    body: &str,
    (w, h, face): (u32, u32, Vec<u8>),
    regions: Vec<(&str, DecodedImage)>,
) -> (EmoWorld, AtlasTable) {
    let dir = temp_path_kit::TempPath::new("presenter-hit-shell");
    std::fs::write(
        dir.path().join("surfaces.txt"),
        format!("charset,UTF-8\nsurface0\n{{\n{body}}}\n"),
    )
    .expect("surfaces.txt 作成");
    std::fs::write(
        dir.path().join("descript.txt"),
        "charset,UTF-8\nseriko.use_self_alpha,1\n",
    )
    .expect("descript.txt 作成");
    std::fs::File::create(dir.path().join("surface0.png")).expect("面の画像作成");
    let mut dec = MemoryDecoder::new();
    dec.insert(dir.path().join("surface0.png"), w, h, w * 4, face, true);
    for (name, image) in regions {
        dec.insert_unmultiplied(dir.path().join(name), image);
    }
    let mut target =
        crate::shell_target::load_shell_target(dir.path(), &dec).expect("シェルは読める");
    (target.take_world(), target.atlas().clone())
}

/// 5 つの形のシェル（80×16・全不透明）の面の表と焼いた絵。
pub(super) fn five_shapes_assets() -> (EmoWorld, AtlasTable) {
    let mask = straight_image(16, 16, |x, _| {
        if x < 8 {
            [0, 0, 255, 255]
        } else {
            [255, 0, 0, 255]
        }
    });
    load_hit_shell(
        FIVE_SHAPES,
        (80, 16, gradient(80, 16, 0x15)),
        vec![(FIVE_SHAPES_MASK, mask)],
    )
}

/// 画面へ描いてから、窓 client 物理 px の点で引いた当たり判定の名前。
pub(super) fn name_at<'a>(
    world: &mut World,
    presenter: &'a EmoPresenter,
    target: TargetId,
    x: i64,
    y: i64,
) -> Option<&'a str> {
    paint_screen(world);
    presenter.hit_region_client(world, target, x, y).region
}

/// ID の描き先の世代（描き先が無ければ `None`）。
pub(super) fn hit_generation(presenter: &EmoPresenter, target: TargetId) -> Option<u64> {
    presenter.targets[&target]
        .hit_picture
        .as_ref()
        .map(|p| p.target.generation())
}

/// マスクの GPU の絵を作った回数の累計。
pub(super) fn mask_pictures_created(presenter: &EmoPresenter, target: TargetId) -> u64 {
    presenter.targets[&target].hit_masks.pictures_created()
}

/// 水準が `WARN` の記録の件数。
pub(super) fn warn_count(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .count()
}
