//! 既定バルーンの 23 ファイルの組み立て（spec: default-balloon-selfmade 要件 2.2・4.1・8.2・8.3）。
//!
//! 端末へ出さず、ファイルも書かない（書くのは入口だけ）。ずれの見張りのテストもこのファイルと
//! 子を `#[path]` の兄弟として読むので、子は `super::` で参照する。

use super::draw::{self, Picture};
use super::texts;
use super::webp;
use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

/// 絵の寸法の表（design.md「ファイルの表」）。描いた絵がこれと 1 つでも違えば書かない。
const SIZES: &[(&str, u32, u32)] = &[
    ("arrow0", 11, 10),
    ("arrow1", 11, 10),
    ("balloonc0", 400, 63),
    ("balloonc1", 400, 63),
    ("balloonc2", 400, 63),
    ("balloonc3", 400, 63),
    ("balloonk0", 326, 96),
    ("balloonk1", 326, 96),
    ("balloons0", 326, 169),
    ("balloons1", 326, 169),
    ("balloons2", 326, 348),
    ("balloons3", 326, 348),
    ("online0", 77, 36),
    ("sstp", 7, 9),
];

/// 書庫の中身 23 ファイルの（名前, バイト列）を名前順で返す。寸法が表と違えば誤り（ASCII の文）。
pub fn build() -> Result<Vec<(String, Vec<u8>)>, String> {
    let pngs = draw::png_pictures();
    let online = draw::online_frames();
    check_sizes(&pngs, &online)?;
    let mut files = Vec::new();
    for (name, pic) in &pngs {
        files.push((format!("{name}.png"), encode_png(name, pic)?));
    }
    files.push((
        "online0.webp".to_string(),
        webp::animated_webp(&online, draw::ONLINE_FRAME_MS),
    ));
    for (name, bytes) in texts::text_files() {
        files.push((name.to_string(), bytes));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// PNG の 13 枚とオンラインのコマを寸法の表と比べる。表に無い絵・表にあって描いていない絵も誤り。
fn check_sizes(pngs: &[(&str, Picture)], online: &[Picture]) -> Result<(), String> {
    let mut drawn: Vec<(&str, &Picture)> = pngs.iter().map(|(n, p)| (*n, p)).collect();
    drawn.extend(online.iter().map(|p| ("online0", p)));
    for (name, pic) in &drawn {
        let Some((_, w, h)) = SIZES.iter().find(|(n, _, _)| n == name) else {
            return Err(format!("size table has no entry for {name}"));
        };
        if (pic.width, pic.height) != (*w, *h) || pic.rgba.len() != (*w * *h * 4) as usize {
            return Err(format!(
                "size mismatch: {name} is {}x{}, table says {w}x{h}",
                pic.width, pic.height
            ));
        }
    }
    for (name, _, _) in SIZES {
        if !drawn.iter().any(|(n, _)| n == name) {
            return Err(format!("picture missing: {name}"));
        }
    }
    Ok(())
}

/// `Rgba8` を `PngEncoder::new` の既定の設定で書く（同じ画素からは同じバイト列）。
fn encode_png(name: &str, pic: &Picture) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(&pic.rgba, pic.width, pic.height, ExtendedColorType::Rgba8)
        .map_err(|_| format!("png encode failed: {name}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pic(width: u32, height: u32) -> Picture {
        Picture {
            width,
            height,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    #[test]
    fn drawn_pictures_match_the_size_table() {
        assert_eq!(
            check_sizes(&draw::png_pictures(), &draw::online_frames()),
            Ok(())
        );
    }

    #[test]
    fn a_wrong_size_is_an_error() {
        let mut pngs = draw::png_pictures();
        pngs[0].1 = pic(12, 10);
        let err = check_sizes(&pngs, &draw::online_frames()).unwrap_err();
        assert!(err.contains("arrow0"), "{err}");
        let mut online = draw::online_frames();
        online[2] = pic(77, 35);
        let err = check_sizes(&draw::png_pictures(), &online).unwrap_err();
        assert!(err.contains("online0"), "{err}");
    }

    #[test]
    fn a_missing_or_unknown_picture_is_an_error() {
        let mut pngs = draw::png_pictures();
        pngs.pop();
        assert!(
            check_sizes(&pngs, &draw::online_frames())
                .unwrap_err()
                .contains("sstp")
        );
        let mut pngs = draw::png_pictures();
        pngs.push(("extra", pic(1, 1)));
        assert!(
            check_sizes(&pngs, &draw::online_frames())
                .unwrap_err()
                .contains("extra")
        );
    }

    #[test]
    fn build_returns_23_files_in_name_order_deterministically() {
        let files = build().unwrap();
        assert_eq!(files.len(), 23);
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert!(names.contains(&"online0.webp") && names.contains(&"readme.txt"));
        for (name, bytes) in &files {
            if name.ends_with(".png") {
                assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "{name}");
            }
        }
        assert_eq!(files, build().unwrap(), "not deterministic");
    }
}
