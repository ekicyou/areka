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
