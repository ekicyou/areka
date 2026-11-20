use std::cell::Cell;
use std::path::Path;

use areka_emo_atlas::{DecodeError, DecodedImage, ElementDecoder, MemoryDecoder};
use areka_parsers::shell::{CollisionRegion, CollisionShape};
use log_capture_kit::capture;
use temp_path_kit::TempPath;

use tracing::Level;

use crate::shell_target::test_support::{
    decoder_with, describe, from_shell_target, warn_lines, write_descript,
};
use crate::shell_target::{load_shell_target, load_shell_target_quiet};

const UNREADABLE: &str = "region の画像が読めない";

struct Counting {
    inner: MemoryDecoder,
    unmultiplied: Cell<usize>,
}

impl ElementDecoder for Counting {
    fn decode(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        self.inner.decode(path)
    }
    fn decode_unmultiplied(&self, path: &Path) -> Result<DecodedImage, DecodeError> {
        self.unmultiplied.set(self.unmultiplied.get() + 1);
        self.inner.decode_unmultiplied(path)
    }
}

fn counting(inner: MemoryDecoder) -> Counting {
    Counting {
        inner,
        unmultiplied: Cell::new(0),
    }
}

fn write_shell(dir: &Path, body: &str) {
    std::fs::write(
        dir.join("surfaces.txt"),
        format!("charset,UTF-8\nsurface0\n{{\n{body}}}\n"),
    )
    .expect("surfaces.txt 作成");
    write_descript(dir);
    std::fs::File::create(dir.join("surface0.png")).expect("面の画像作成");
}

fn region_of(world: &areka_emo_compose::EmoWorld, index: usize) -> CollisionRegion {
    match &world.hit_regions(0).expect("面 0")[index].shape {
        CollisionShape::Region(region) => region.clone(),
        other => panic!("region でない: {other:?}"),
    }
}

#[test]
fn region_mask_is_on_the_face_table_in_both_loads() {
    let dir = TempPath::new("shell-regions-ok");
    write_shell(dir.path(), "collisionex0,Head,region,mask.png,255,0,0\n");
    let mut inner = decoder_with(dir.path(), &["surface0.png"]);
    inner.insert_unmultiplied(
        dir.path().join("mask.png"),
        DecodedImage {
            width: 2,
            height: 1,
            stride: 8,
            bgra: vec![0, 0, 255, 255, 0, 255, 0, 255],
            has_alpha: true,
        },
    );
    let dec = counting(inner);

    for quiet in [false, true] {
        let mut target = if quiet {
            load_shell_target_quiet(dir.path(), &dec)
        } else {
            load_shell_target(dir.path(), &dec)
        }
        .expect("読める");
        for world in [target.take_world(), target.build_world()] {
            let masks = world.region_masks().expect("面の表にマスクの集まりが在る");
            let mask = masks.get(&region_of(&world, 0)).expect("マスクが引ける");
            assert!(mask.is_set(0, 0) && !mask.is_set(1, 0));
        }
    }
    assert_eq!(dec.unmultiplied.get(), 2, "読み込みごとに 1 度だけ読む");
}

#[test]
fn shell_without_region_reads_no_region_image() {
    let dir = TempPath::new("shell-regions-none");
    write_shell(dir.path(), "collision0,0,0,1,1,Head\n");
    let dec = counting(decoder_with(dir.path(), &["surface0.png"]));

    load_shell_target(dir.path(), &dec).expect("読める");
    load_shell_target_quiet(dir.path(), &dec).expect("読める");
    assert_eq!(dec.unmultiplied.get(), 0);
}

#[test]
fn unreadable_file_pointed_by_three_regions_warns_once() {
    let dir = TempPath::new("shell-regions-bad");
    write_shell(
        dir.path(),
        concat!(
            "collisionex0,A,region,bad.png,255,0,0\n",
            "collisionex1,B,region,BAD.png,0,255,0\n",
            "collisionex2,C,region,bad.png,0,0,255,true\n",
        ),
    );
    let mut inner = decoder_with(dir.path(), &["surface0.png"]);
    inner.insert_corrupt(dir.path().join("bad.png"), "壊れている");
    let dec = counting(inner);

    let (mut target, events) = capture(|| load_shell_target(dir.path(), &dec).expect("読める"));
    let lines: Vec<String> = from_shell_target(&events, Level::WARN)
        .into_iter()
        .map(describe)
        .filter(|l| l.contains(UNREADABLE))
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("file=\"bad.png\"") && lines[0].contains("壊れている"),
        "{lines:?}"
    );
    let world = target.take_world();
    let masks = world.region_masks().expect("面の表にマスクの集まりが在る");
    assert!(masks.get(&region_of(&world, 0)).is_none());

    let (_, quiet_events) = capture(|| load_shell_target_quiet(dir.path(), &dec).expect("読める"));
    assert!(
        warn_lines(&quiet_events)
            .iter()
            .all(|l| !l.contains(UNREADABLE)),
        "{quiet_events:?}"
    );
}
