//! 動く絵の見分け: ファイルの見出し（PNG のチャンク・WebP の RIFF チャンク）だけを読み、
//! コマが 2 枚以上の APNG・WebP かどうかとコマの枚数・寸法を答える
//! （spec: areka-P0-animated-image-decode 要件 1.1〜1.5・6.3）。画素は解かない。
//!
//! 読むのはチャンクの見出し 8 バイトと、`IHDR`・`acTL`・`VP8X` の中身の先頭だけ。ほかの中身は
//! seek で飛ばす。チャンクの長さが残りのファイルより長ければそこで止め、それまでの結果で答える
//! （長さの欄の値でメモリを取らないので、壊れた長さでも読み過ぎない）。

use std::io::{Read, Seek, SeekFrom};

use super::AnimationInfo;

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// `VP8X` の旗のうち「動く」の 1 ビット。
const WEBP_ANIMATION_FLAG: u8 = 0x02;

/// 動く APNG・WebP なら見出しを返す。それ以外（静止画・GIF・読めない）は None。
///
/// 署名で形式を分ける（拡張子は見ない）。読み手の位置はどこからでもよい（先頭へ戻して読む）。
pub(crate) fn sniff<R: Read + Seek>(reader: &mut R) -> Option<AnimationInfo> {
    let total = reader.seek(SeekFrom::End(0)).ok()?;
    let head = read_at::<12, _>(reader, 0)?;
    if head[..8] == PNG_SIGNATURE {
        sniff_png(reader, total)
    } else if &head[..4] == b"RIFF" && &head[8..] == b"WEBP" {
        sniff_webp(reader, total)
    } else {
        None
    }
}

/// PNG: `IDAT` に着くまでチャンクの見出しをたどり、`IHDR` の寸法と `acTL` の枚数を読む。
fn sniff_png<R: Read + Seek>(reader: &mut R, total: u64) -> Option<AnimationInfo> {
    let mut pos = 8u64;
    let mut size = None;
    let mut frames = 0u32;
    // 見出し（長さ 4・種類 4）＋中身＋CRC 4。
    while let Some(header) = read_at::<8, _>(reader, pos) {
        let len = u64::from(be32(&header[..4]));
        let kind = &header[4..];
        let end = pos + 12 + len;
        if kind == b"IDAT" || end > total {
            break;
        }
        if kind == b"IHDR" && len >= 8 {
            let Some(d) = read_at::<8, _>(reader, pos + 8) else {
                break;
            };
            size = Some((be32(&d[..4]), be32(&d[4..])));
        } else if kind == b"acTL" && len >= 4 {
            let Some(d) = read_at::<4, _>(reader, pos + 8) else {
                break;
            };
            frames = be32(&d);
        }
        pos = end;
    }
    let (width, height) = size?;
    (frames >= 2).then_some(AnimationInfo {
        width,
        height,
        frame_count: frames,
    })
}

/// WebP: 最初のチャンクが動きの旗の立った `VP8X` なら寸法を読み、`ANMF` の数を数える。
fn sniff_webp<R: Read + Seek>(reader: &mut R, total: u64) -> Option<AnimationInfo> {
    let header = read_at::<8, _>(reader, 12)?;
    let len = u64::from(le32(&header[4..]));
    if &header[..4] != b"VP8X" || len < 10 || 20 + len > total {
        return None;
    }
    let d = read_at::<10, _>(reader, 20)?;
    if d[0] & WEBP_ANIMATION_FLAG == 0 {
        return None;
    }
    let width = le24(&d[4..7]) + 1;
    let height = le24(&d[7..10]) + 1;

    // 見出し（種類 4・長さ 4）＋中身＋奇数長なら詰め 1 バイト。
    let mut pos = 20 + len + (len & 1);
    let mut frames = 0u32;
    while let Some(header) = read_at::<8, _>(reader, pos) {
        let len = u64::from(le32(&header[4..]));
        if pos + 8 + len > total {
            break;
        }
        if &header[..4] == b"ANMF" {
            frames = frames.saturating_add(1);
        }
        pos += 8 + len + (len & 1);
    }
    (frames >= 2).then_some(AnimationInfo {
        width,
        height,
        frame_count: frames,
    })
}

/// `pos` から `N` バイトを読む。読めなければ None。
fn read_at<const N: usize, R: Read + Seek>(reader: &mut R, pos: u64) -> Option<[u8; N]> {
    let mut buf = [0u8; N];
    reader.seek(SeekFrom::Start(pos)).ok()?;
    reader.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn le24(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], 0])
}

#[cfg(test)]
#[path = "sniff_tests.rs"]
mod tests;
