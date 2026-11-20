use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use areka_emo_atlas::{DecodeError, DecodedImage, ElementDecoder, MemoryDecoder};
use areka_parsers::shell::CollisionRegion;

use super::{RegionMasks, region_mask};

/// 画素を BGRA の順で並べた 1 行の絵。
fn row(pixels: &[[u8; 4]]) -> DecodedImage {
    DecodedImage {
        width: pixels.len() as u32,
        height: 1,
        stride: pixels.len() as u32 * 4,
        bgra: pixels.iter().flatten().copied().collect(),
        has_alpha: true,
    }
}

fn bits(mask: &super::RegionMask) -> Vec<bool> {
    (0..mask.width()).map(|x| mask.is_set(x, 0)).collect()
}

fn region(file: &str, rgb: [u8; 3], invert: bool) -> CollisionRegion {
    CollisionRegion {
        file: file.to_string(),
        red: rgb[0],
        green: rgb[1],
        blue: rgb[2],
        invert,
        x: 0,
        y: 0,
    }
}

/// 読んだパスを控える読み手。
struct Counting {
    inner: MemoryDecoder,
    reads: RefCell<Vec<PathBuf>>,
}

impl ElementDecoder for Counting {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        self.inner.decode(path)
    }

    fn decode_unmultiplied(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        self.reads.borrow_mut().push(path.to_path_buf());
        self.inner.decode_unmultiplied(path)
    }
}

const RED: [u8; 4] = [0, 0, 255, 255];
const TRANSPARENT_RED: [u8; 4] = [0, 0, 255, 0];
const HALF_RED: [u8; 4] = [0, 0, 255, 128];
const NEAR_RED: [u8; 4] = [0, 0, 254, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

#[test]
fn color_rule_matches_exactly_and_treats_transparent_as_black() {
    let image = row(&[RED, TRANSPARENT_RED, HALF_RED, NEAR_RED, BLACK]);
    assert_eq!(
        bits(&region_mask(&image, [255, 0, 0], false)),
        [true, false, true, false, false]
    );
    assert_eq!(
        bits(&region_mask(&image, [255, 0, 0], true)),
        [false, true, false, true, true]
    );
    assert_eq!(
        bits(&region_mask(&image, [0, 0, 0], false)),
        [false, true, false, false, true]
    );
}

#[test]
fn image_all_in_the_color_sets_every_pixel() {
    let image = row(&[RED, RED, RED]);
    let mask = region_mask(&image, [255, 0, 0], false);
    assert_eq!(bits(&mask), [true, true, true]);
    assert!(!mask.is_set(3, 0));
    assert!(!mask.is_set(0, 1));
}

#[test]
fn stride_padding_is_skipped() {
    let image = DecodedImage {
        width: 1,
        height: 2,
        stride: 8,
        bgra: [RED, BLACK, BLACK, RED].concat(),
        has_alpha: true,
    };
    let mask = region_mask(&image, [255, 0, 0], false);
    assert!(mask.is_set(0, 0));
    assert!(!mask.is_set(0, 1));
}

#[test]
fn load_reads_each_file_once_and_keeps_masks_per_color_and_invert() {
    let base = Path::new("shell");
    let mut inner = MemoryDecoder::new();
    inner.insert_unmultiplied(base.join("mask/a.png"), row(&[RED, BLACK]));
    let decoder = Counting {
        inner,
        reads: RefCell::new(Vec::new()),
    };
    let regions = [
        region("mask/a.png", [255, 0, 0], false),
        region("MASK\\A.PNG", [255, 0, 0], true),
        region("Mask/A.png", [0, 0, 0], false),
        region("mask/a.png", [255, 0, 0], false),
    ];
    let masks = RegionMasks::load(regions.iter(), base, &decoder);

    assert_eq!(decoder.reads.borrow().len(), 1);
    assert!(masks.unreadable().is_empty());
    let a = masks.get(&regions[0]).expect("mask");
    assert_eq!(bits(a), [true, false]);
    assert_eq!(
        bits(masks.get(&regions[1]).expect("inverted")),
        [false, true]
    );
    assert_eq!(bits(masks.get(&regions[2]).expect("black")), [false, true]);
    assert!(Arc::ptr_eq(a, masks.get(&regions[3]).expect("same")));
}

#[test]
fn unreadable_files_get_one_record_each_and_no_mask() {
    let base = Path::new("shell");
    let mut decoder = MemoryDecoder::new();
    decoder.insert_corrupt(base.join("broken.png"), "bad header");
    decoder.insert(base.join("premul.png"), 1, 1, 4, RED.to_vec(), true);
    let regions = [
        region("missing.png", [255, 0, 0], false),
        region("MISSING.png", [255, 0, 0], true),
        region("broken.png", [255, 0, 0], false),
        region("premul.png", [255, 0, 0], false),
    ];
    let masks = RegionMasks::load(regions.iter(), base, &decoder);

    let names: Vec<&str> = masks.unreadable().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["missing.png", "broken.png", "premul.png"]);
    assert!(masks.unreadable()[1].1.contains("bad header"));
    for r in &regions {
        assert!(masks.get(r).is_none());
    }
}

#[test]
fn face_tables_share_one_mask_collection() {
    let masks = Arc::new(RegionMasks::default());
    let shell = areka_parsers::shell::Shell {
        surfaces: Vec::new(),
        appends: Vec::new(),
        aliases: Vec::new(),
        animation_sort: None,
        collision_sort: None,
        definitions: Vec::new(),
    };
    let mut a = crate::EmoWorld::build(&shell);
    let mut b = crate::EmoWorld::build(&shell);
    assert!(a.region_masks().is_none());
    a.set_region_masks(masks.clone());
    b.set_region_masks(masks.clone());
    assert!(Arc::ptr_eq(a.region_masks().unwrap(), &masks));
    assert!(Arc::ptr_eq(b.region_masks().unwrap(), &masks));
}

/// RGBA の組を BGRA の画素へ。
fn px(r: u8, g: u8, b: u8, a: u8) -> [u8; 4] {
    [b, g, r, a]
}

#[test]
fn alpha_pixels_follow_the_measured_ssp_table() {
    // A..F と地: α=0 は蓄えた RGB に依らず黒、α≥1 は蓄えた RGB のまま。
    let image = row(&[
        px(255, 0, 0, 0),
        px(255, 0, 0, 128),
        px(255, 0, 0, 255),
        px(255, 0, 0, 1),
        px(255, 0, 0, 254),
        px(255, 255, 255, 0),
        px(0, 0, 0, 0),
    ]);
    let t = true;
    let f = false;
    assert_eq!(
        bits(&region_mask(&image, [255, 0, 0], false)),
        [f, t, t, t, t, f, f]
    );
    assert_eq!(
        bits(&region_mask(&image, [255, 0, 0], true)),
        [t, f, f, f, f, t, t]
    );
    assert_eq!(
        bits(&region_mask(&image, [0, 0, 0], false)),
        [t, f, f, f, f, t, t]
    );
}

#[test]
fn color_must_match_in_all_three_components() {
    let image = row(&[
        px(255, 0, 0, 255),
        px(254, 0, 0, 255),
        px(255, 1, 0, 255),
        px(255, 0, 1, 255),
        px(0, 255, 0, 255),
        px(255, 255, 255, 255),
    ]);
    assert_eq!(
        bits(&region_mask(&image, [255, 0, 0], false)),
        [true, false, false, false, false, false]
    );
    assert_eq!(
        bits(&region_mask(&image, [254, 0, 0], false)),
        [false, true, false, false, false, false]
    );
}

#[test]
fn inverted_small_image_sets_everything_but_the_color_and_nothing_outside() {
    let image = DecodedImage {
        width: 4,
        height: 4,
        stride: 16,
        bgra: (0..16)
            .flat_map(|i| {
                if (1..=2).contains(&(i % 4)) && (1..=2).contains(&(i / 4)) {
                    px(255, 0, 0, 255)
                } else {
                    px(255, 255, 255, 255)
                }
            })
            .collect(),
        has_alpha: true,
    };
    let mask = region_mask(&image, [255, 0, 0], true);
    assert!(!mask.is_set(1, 1));
    assert!(!mask.is_set(2, 2));
    assert!(mask.is_set(0, 0));
    assert!(mask.is_set(3, 3));
    assert!(mask.is_set(0, 1));
    assert!(!mask.is_set(4, 4));
    assert!(!mask.is_set(4, 0));
    assert!(!mask.is_set(0, 4));
}

#[test]
fn spellings_of_one_file_are_read_once() {
    let base = Path::new("shell");
    let mut inner = MemoryDecoder::new();
    inner.insert_unmultiplied(base.join("a/b.png"), row(&[RED]));
    let decoder = Counting {
        inner,
        reads: RefCell::new(Vec::new()),
    };
    let regions = [
        region("a/b.png", [255, 0, 0], false),
        region(r"A\B.PNG", [255, 0, 0], false),
        region(r"a\b.Png", [255, 0, 0], false),
    ];
    let masks = RegionMasks::load(regions.iter(), base, &decoder);

    assert_eq!(*decoder.reads.borrow(), [base.join("a/b.png")]);
    for r in &regions {
        assert_eq!(bits(masks.get(r).expect("mask")), [true]);
    }
}

#[test]
fn unreadable_file_is_read_once_and_recorded_once() {
    let base = Path::new("shell");
    let decoder = Counting {
        inner: MemoryDecoder::new(),
        reads: RefCell::new(Vec::new()),
    };
    let regions = [
        region("gone.png", [255, 0, 0], false),
        region("GONE.png", [255, 0, 0], true),
        region("gone.png", [0, 0, 0], false),
    ];
    let masks = RegionMasks::load(regions.iter(), base, &decoder);

    assert_eq!(decoder.reads.borrow().len(), 1);
    assert_eq!(masks.unreadable().len(), 1);
    for r in &regions {
        assert!(masks.get(r).is_none());
    }
}
