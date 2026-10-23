//! [`compare_dirs`](super::compare_dirs) の試験（要件 13.2・13.5）。
//!
//! 検体の PNG は `target\test-roots\` の下に `image` で書く。

use super::*;
use image::{Rgba, RgbaImage};
use temp_path_kit::TempPath;

/// 比べる 2 つのフォルダ（SSP 側・areka 側）を `target\` の下に作る。
fn dirs() -> (TempPath, TempPath) {
    (
        TempPath::under_target("surface-compare-ssp"),
        TempPath::under_target("surface-compare-areka"),
    )
}

/// 1 色で塗った w×h の PNG を書く。
fn write_png(dir: &TempPath, name: &str, w: u32, h: u32, px: [u8; 4]) -> RgbaImage {
    let img = RgbaImage::from_pixel(w, h, Rgba(px));
    img.save(dir.child(name)).expect("検体の PNG を書けるはず");
    img
}

fn only(verdicts: Vec<FileVerdict>) -> FileVerdict {
    assert_eq!(verdicts.len(), 1, "組は 1 つのはず: {verdicts:?}");
    verdicts.into_iter().next().unwrap()
}

#[test]
fn same_pixels_are_same() {
    let (ssp, areka) = dirs();
    write_png(&ssp, "surface0.png", 3, 2, [10, 20, 30, 128]);
    // 名前は大文字小文字を区別せずに組になる。
    write_png(&areka, "Surface0.PNG", 3, 2, [10, 20, 30, 128]);
    let got = only(compare_dirs(ssp.path(), areka.path()).unwrap());
    assert_eq!(got.name, "surface0.png");
    assert_eq!(got.verdict, Verdict::Same);
}

#[test]
fn different_size_is_size_differs() {
    let (ssp, areka) = dirs();
    write_png(&ssp, "surface1.png", 3, 2, [0, 0, 0, 255]);
    write_png(&areka, "surface1.png", 4, 5, [0, 0, 0, 255]);
    let got = only(compare_dirs(ssp.path(), areka.path()).unwrap());
    assert_eq!(
        got.verdict,
        Verdict::SizeDiffers {
            ssp: (3, 2),
            areka: (4, 5)
        }
    );
}

#[test]
fn different_pixels_report_count_and_max_diffs() {
    let (ssp, areka) = dirs();
    write_png(&ssp, "surface2.png", 4, 4, [100, 100, 100, 200]);
    let mut img = RgbaImage::from_pixel(4, 4, Rgba([100, 100, 100, 200]));
    // 色の差の最大は 2 つ目の画素の B（-40）、α の差の最大は 3 つ目の画素（+30）。
    img.put_pixel(0, 0, Rgba([105, 100, 100, 200]));
    img.put_pixel(1, 0, Rgba([100, 90, 60, 190]));
    img.put_pixel(2, 3, Rgba([100, 100, 100, 230]));
    img.save(areka.child("surface2.png")).unwrap();
    let got = only(compare_dirs(ssp.path(), areka.path()).unwrap());
    assert_eq!(
        got.verdict,
        Verdict::PixelsDiffer {
            count: 3,
            max_color: 40,
            max_alpha: 30
        }
    );
}

#[test]
fn file_on_one_side_only_is_reported_in_name_order() {
    let (ssp, areka) = dirs();
    write_png(&ssp, "surface20.png", 1, 1, [0, 0, 0, 0]);
    write_png(&areka, "surface10.png", 1, 1, [0, 0, 0, 0]);
    // PNG でないファイルは比べない。
    std::fs::write(ssp.child("readme.txt"), "x").unwrap();
    let got = compare_dirs(ssp.path(), areka.path()).unwrap();
    assert_eq!(
        got,
        vec![
            FileVerdict {
                name: "surface10.png".into(),
                verdict: Verdict::OnlyIn(Side::Areka)
            },
            FileVerdict {
                name: "surface20.png".into(),
                verdict: Verdict::OnlyIn(Side::Ssp)
            },
        ]
    );
}

#[test]
fn unreadable_file_is_reported_with_its_side() {
    let (ssp, areka) = dirs();
    write_png(&ssp, "surface3.png", 1, 1, [0, 0, 0, 255]);
    std::fs::write(areka.child("surface3.png"), b"not a png").unwrap();
    let got = only(compare_dirs(ssp.path(), areka.path()).unwrap());
    match got.verdict {
        Verdict::Unreadable { side, message } => {
            assert_eq!(side, Side::Areka);
            assert!(!message.is_empty());
        }
        other => panic!("読めないはず: {other:?}"),
    }
}

#[test]
fn missing_dir_is_err() {
    let (ssp, _areka) = dirs();
    assert!(compare_dirs(ssp.path(), &ssp.child("no-such-dir")).is_err());
}
