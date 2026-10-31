//! パレットの絵の検体を作る（spec: emo-atlas-palette-alpha 要件 3.5・5.4・5.7・5.8）。
//!
//! `cargo run -p areka-emo-atlas --example gen_palette_samples` で
//! `src/testdata/palette/` に 6 本を書く。使うのは std だけ。
//! 圧縮しない（zlib の無圧縮の 1 ブロック）ので、何度走らせても同じバイト列になる。
//! 中身の説明は同じフォルダの `README.md`。wintf の `tests/com/wic_test.rs` も同じ検体を読む。

use std::path::Path;

const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];
const WHITE: [u8; 3] = [255, 255, 255];

// ---- PNG ----

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in bytes {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

const PNG_GRAY: u8 = 0;
const PNG_PALETTE: u8 = 3;

/// 2×2 の PNG。`values` は (0,0)・(1,0)・(0,1)・(1,1) の順の番号（グレーなら値）。
/// `bit_depth` は 8 か 4。
fn png(
    bit_depth: u8,
    color_type: u8,
    palette: &[[u8; 3]],
    trns: Option<&[u8]>,
    values: [u8; 4],
) -> Vec<u8> {
    let mut raw = Vec::new();
    for row in values.chunks(2) {
        raw.push(0); // フィルタ 0（無し）
        match bit_depth {
            8 => raw.extend_from_slice(row),
            // 2 画素で 1 バイト（上位の 4 ビットが左）。行は 1 バイトで割り切れる。
            4 => raw.push((row[0] << 4) | row[1]),
            _ => unreachable!("8 か 4 ビットだけ"),
        }
    }
    // zlib: 見出し 78 01・無圧縮の最後のブロック・Adler-32。
    let mut idat = vec![0x78, 0x01, 0x01];
    let len = raw.len() as u16;
    idat.extend_from_slice(&len.to_le_bytes());
    idat.extend_from_slice(&(!len).to_le_bytes());
    idat.extend_from_slice(&raw);
    idat.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&2u32.to_be_bytes());
    ihdr.extend_from_slice(&2u32.to_be_bytes());
    ihdr.extend_from_slice(&[bit_depth, color_type, 0, 0, 0]);

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    png_chunk(&mut out, b"IHDR", &ihdr);
    if !palette.is_empty() {
        png_chunk(&mut out, b"PLTE", &palette.concat());
    }
    if let Some(trns) = trns {
        png_chunk(&mut out, b"tRNS", trns);
    }
    png_chunk(&mut out, b"IDAT", &idat);
    png_chunk(&mut out, b"IEND", &[]);
    out
}

// ---- GIF ----

/// 2×2 の GIF89a。大域の色の表 4 色（赤・緑・青・白）・透明の番号 1・番号 0,0,1,2。
/// LZW は最小の符号の長さ 2。符号ごとに消去の符号（4）を挟むので表が育たず、どれも 3 ビット。
fn trans_gif() -> Vec<u8> {
    let mut out = b"GIF89a".to_vec();
    out.extend_from_slice(&[0x02, 0x00, 0x02, 0x00]); // 幅 2・高さ 2
    out.extend_from_slice(&[0x91, 0x00, 0x00]); // 大域の色の表あり（4 色）・背景 0・縦横比 0
    out.extend_from_slice(&[RED, GREEN, BLUE, WHITE].concat());
    // 図形の制御の拡張: 透明の旗・待ち時間 0・透明の番号 1。
    out.extend_from_slice(&[0x21, 0xF9, 0x04, 0x01, 0x00, 0x00, 0x01, 0x00]);
    out.extend_from_slice(&[0x2C, 0, 0, 0, 0, 0x02, 0x00, 0x02, 0x00, 0x00]);
    out.push(2); // LZW の最小の符号の長さ

    let (clear, end) = (4u32, 5u32);
    let mut codes = Vec::new();
    for index in [0, 0, 1, 2] {
        codes.extend_from_slice(&[clear, index]);
    }
    codes.push(end);
    // 下位のビットから詰める。
    let (mut bytes, mut acc, mut nbits) = (Vec::new(), 0u32, 0);
    for code in codes {
        acc |= code << nbits;
        nbits += 3;
        while nbits >= 8 {
            bytes.push(acc as u8);
            acc >>= 8;
            nbits -= 8;
        }
    }
    if nbits > 0 {
        bytes.push(acc as u8);
    }
    out.push(bytes.len() as u8);
    out.extend_from_slice(&bytes);
    out.push(0x00); // 中身の終わり
    out.push(0x3B);
    out
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/testdata/palette");
    std::fs::create_dir_all(&dir).expect("検体のフォルダを作れない");
    let write = |name: &str, bytes: &[u8]| {
        std::fs::write(dir.join(name), bytes).expect("検体を書けない");
        println!("{name}: {} バイト", bytes.len());
    };

    let rgb = [RED, GREEN, BLUE];
    // 番号 0＝赤 透明度 255・1＝緑 透明度 0・2＝青 透明度 128。
    let mixed: &[u8] = &[0xFF, 0x00, 0x80];
    write(
        "idx8_mixed.png",
        &png(8, PNG_PALETTE, &rgb, Some(mixed), [0, 0, 1, 2]),
    );
    write(
        "idx4_mixed.png",
        &png(4, PNG_PALETTE, &rgb, Some(mixed), [0, 0, 1, 2]),
    );
    write(
        "idx8_plain.png",
        &png(8, PNG_PALETTE, &rgb, None, [0, 0, 1, 1]),
    );
    write(
        "idx8_opaque_trns.png",
        &png(
            8,
            PNG_PALETTE,
            &rgb,
            Some(&[0xFF, 0xFF, 0xFF]),
            [0, 0, 1, 1],
        ),
    );
    // グレーの tRNS は 16 ビットの値: 0 を透明にする。
    write(
        "gray8_trns.png",
        &png(8, PNG_GRAY, &[], Some(&[0x00, 0x00]), [255, 255, 0, 128]),
    );
    write("trans.gif", &trans_gif());
}
