//! α トリミング（bbox 算出・オフセット記録・全透明→空）。
//!
//! 要件 **R4**。
//!
//! premultiplied BGRA バッファを stride 込みで走査し、α>0 のタイト矩形（bbox）を
//! 算出する（`AlphaMask::from_pbgra32` の走査パターンを踏襲）。トリムオフセット・
//! トリム寸・原寸を記録し、「配置座標＋トリムオフセットで転写すれば見た目等価」の
//! 規約を成立させる。全透明の element は空エントリ（転写スキップ）とする。
//!
//! α 閾値はトリム専用に**厳密 α>0**（ヒットテストの 128 ではない・4.1 の文言に忠実）。
//! α==1 の画素も不透明として bbox に含める。

use crate::normalize::NormalizedImage;
use crate::table::{Point, Size};

/// トリム結果（原寸＋配置エントリ・全透明は空）。
///
/// `original` は常に原画像の寸法（全透明でも記録・4.2/4.5）。`placement=None` は
/// 全 α==0 の空エントリ（以降の焼付をスキップできる・4.4）。
#[derive(Clone, Debug)]
pub struct TrimResult {
    /// 原寸（4.2/4.5）。全透明でも記録する。
    pub original: Size,
    /// トリム後の配置エントリ。`None`＝全透明（空エントリ・焼付スキップ・4.4）。
    pub placement: Option<Trimmed>,
}

/// トリム後の配置エントリ（tightly packed premultiplied BGRA）。
///
/// `trim_offset`＋配置座標で原画像全体を焼き付けた場合と見た目が等価になる（4.5）。
/// `pbgra` は隙間なく詰めた最小矩形（`stride == size.w*4`・premultiplied 素通し・4.3）。
#[derive(Clone, Debug)]
pub struct Trimmed {
    /// 原画像内の bbox 左上（4.2）。
    pub trim_offset: Point,
    /// トリム後寸法（α>0 を過不足なく含む最小・4.2）。
    pub size: Size,
    /// トリム後 premultiplied BGRA（隙間なし・`len == stride * size.h`・4.3）。
    pub pbgra: Vec<u8>,
    /// 行あたりバイト数（tightly packed ゆえ `size.w * 4`）。
    pub stride: u32,
}

/// α トリマー（bbox 算出・空判定・素通し焼付）。
pub struct Trimmer;

impl Trimmer {
    /// 正規化済み premultiplied BGRA 画像から α>0 タイト矩形を算出する。
    ///
    /// stride 込み走査（行頭 `y*stride`・画素 `+x*4`・α は `+3`）は
    /// `AlphaMask::from_pbgra32` の前例に倣うが、閾値は**厳密 α>0**（4.1）。
    /// α>0 画素が 1 つも無ければ `placement=None`（全透明・4.4）。それ以外は
    /// bbox を新規 tightly-packed バッファへ抽出し（画素は無変換・premultiplied
    /// のまま素通し・4.3/D8）、原画像内オフセットと寸法を記録する（4.2）。
    pub fn trim(&self, img: &NormalizedImage) -> TrimResult {
        let original = Size {
            w: img.width,
            h: img.height,
        };

        // α>0 タイト bbox を走査（AlphaMask::from_pbgra32 の stride パターン踏襲・閾値のみ差）。
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        let mut any = false;

        for y in 0..img.height {
            let row_start = (y * img.stride) as usize;
            for x in 0..img.width {
                // PBGRA32: 4 byte/画素・α は 4 byte 目（オフセット +3）。
                let alpha_offset = row_start + (x as usize * 4) + 3;
                let alpha = img.pbgra.get(alpha_offset).copied().unwrap_or(0);
                // 厳密 α>0（ヒットテストの 128 ではない・4.1）。
                if alpha > 0 {
                    any = true;
                    if x < min_x {
                        min_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }

        // 全透明（α>0 画素ゼロ）＝空エントリ・焼付スキップ（4.4）。
        if !any {
            return TrimResult {
                original,
                placement: None,
            };
        }

        let size = Size {
            w: max_x - min_x + 1,
            h: max_y - min_y + 1,
        };
        let out_stride = size.w * 4;

        // bbox 領域を tightly-packed な新規バッファへ行単位でコピー（premultiplied 素通し・4.3/D8）。
        let mut pbgra = vec![0u8; (out_stride * size.h) as usize];
        for row in 0..size.h {
            let src_y = min_y + row;
            let src_start = (src_y * img.stride + min_x * 4) as usize;
            let dst_start = (row * out_stride) as usize;
            let len = (size.w * 4) as usize;
            pbgra[dst_start..dst_start + len]
                .copy_from_slice(&img.pbgra[src_start..src_start + len]);
        }

        TrimResult {
            original,
            placement: Some(Trimmed {
                trim_offset: Point {
                    x: min_x as i32,
                    y: min_y as i32,
                },
                size,
                pbgra,
                stride: out_stride,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 手組み premultiplied BGRA から NormalizedImage を作る（stride == width*4）。
    fn img(width: u32, height: u32, pbgra: Vec<u8>) -> NormalizedImage {
        let stride = width * 4;
        assert_eq!(pbgra.len(), (stride * height) as usize, "test buffer size");
        NormalizedImage {
            width,
            height,
            stride,
            pbgra,
        }
    }

    /// 指定座標に α（BGRA の A）を書き込む（B/G/R は 0・premultiplied 適合）。
    fn set_alpha(buf: &mut [u8], stride: u32, x: u32, y: u32, a: u8) {
        let off = (y * stride + x * 4) as usize;
        buf[off + 3] = a;
    }

    /// 4.1: 片側だけ不透明 → タイト bbox（4×4 の内側 x∈[2,3] y∈[1,2] のみ α>0）。
    #[test]
    fn tight_bbox_for_one_side_opaque() {
        let (w, h) = (4u32, 4u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        for y in 1..=2 {
            for x in 2..=3 {
                set_alpha(&mut buf, w * 4, x, y, 255);
            }
        }
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("has opaque pixels");
        assert_eq!(t.trim_offset, Point { x: 2, y: 1 });
        assert_eq!(t.size, Size { w: 2, h: 2 });
        assert_eq!(res.original, Size { w: 4, h: 4 });
    }

    /// 4.1: 単一コーナー（右下 1px のみ）でも過不足なくタイト。
    #[test]
    fn tight_bbox_single_corner() {
        let (w, h) = (3u32, 2u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        set_alpha(&mut buf, w * 4, w - 1, h - 1, 200);
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("corner opaque");
        assert_eq!(t.trim_offset, Point { x: 2, y: 1 });
        assert_eq!(t.size, Size { w: 1, h: 1 });
    }

    /// 4.1: 右半分のみ不透明 → 左半分は切り落とされる。
    #[test]
    fn tight_bbox_right_half() {
        let (w, h) = (4u32, 3u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        for y in 0..h {
            for x in 2..w {
                set_alpha(&mut buf, w * 4, x, y, 255);
            }
        }
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("right half opaque");
        assert_eq!(t.trim_offset, Point { x: 2, y: 0 });
        assert_eq!(t.size, Size { w: 2, h: 3 });
    }

    /// 4.4: 全透明 → placement None・original は記録される。
    #[test]
    fn all_transparent_is_empty_entry() {
        let (w, h) = (5u32, 4u32);
        let buf = vec![0u8; (w * 4 * h) as usize]; // 全 α==0
        let res = Trimmer.trim(&img(w, h, buf));
        assert!(res.placement.is_none(), "all-transparent → None");
        assert_eq!(res.original, Size { w: 5, h: 4 }, "original still recorded");
    }

    /// 4.2: trim_offset / size / original の三者を具体値で記録。
    #[test]
    fn records_offset_size_original() {
        let (w, h) = (6u32, 5u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        // x∈[1,4] y∈[2,3]
        for y in 2..=3 {
            for x in 1..=4 {
                set_alpha(&mut buf, w * 4, x, y, 255);
            }
        }
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("opaque region");
        assert_eq!(t.trim_offset, Point { x: 1, y: 2 });
        assert_eq!(t.size, Size { w: 4, h: 2 });
        assert_eq!(res.original, Size { w: 6, h: 5 });
        // 三者整合（invariant）: trim_offset + size <= original。
        assert!(t.trim_offset.x as u32 + t.size.w <= res.original.w);
        assert!(t.trim_offset.y as u32 + t.size.h <= res.original.h);
    }

    /// 4.5（最強）: 座標不変。トリム結果を trim_offset へ blit し直すと原画像に一致。
    ///
    /// bbox 外は全て α==0（premultiplied では BGRA すべて 0）ゆえ、tight 矩形を
    /// trim_offset へ焼き戻すと原バッファとバイト単位で一致する＝「配置座標＋
    /// trim_offset で原画像全焼付と見た目等価」を証明する。
    #[test]
    fn coordinate_invariance_reconstructs_original() {
        let (w, h) = (5u32, 5u32);
        let orig_stride = w * 4;
        let mut orig = vec![0u8; (orig_stride * h) as usize];
        // 内側 x∈[1,3] y∈[1,2] に「色付き」premultiplied 画素（B,G,R <= A）。
        for y in 1..=2u32 {
            for x in 1..=3u32 {
                let off = (y * orig_stride + x * 4) as usize;
                orig[off] = 10; // B
                orig[off + 1] = 20; // G
                orig[off + 2] = 30; // R
                orig[off + 3] = 200; // A（>0）
            }
        }

        let res = Trimmer.trim(&img(w, h, orig.clone()));
        let t = res.placement.expect("has opaque region");

        // 新規ゼロバッファ（original 寸・dest stride = original.w*4）へ trim_offset で blit。
        let dst_stride = res.original.w * 4;
        let mut recon = vec![0u8; (dst_stride * res.original.h) as usize];
        for row in 0..t.size.h {
            let src_start = (row * t.stride) as usize;
            let src_len = (t.size.w * 4) as usize;
            let dst_y = t.trim_offset.y as u32 + row;
            let dst_x = t.trim_offset.x as u32;
            let dst_start = (dst_y * dst_stride + dst_x * 4) as usize;
            recon[dst_start..dst_start + src_len]
                .copy_from_slice(&t.pbgra[src_start..src_start + src_len]);
        }

        // バイト単位で原画像に一致（bbox 外は両者とも全 0）。
        assert_eq!(
            recon, orig,
            "reconstructed buffer equals original byte-for-byte"
        );
    }

    /// 4.1: 閾値は α>0（128 ではない）。α==1 の画素は bbox に含まれる。
    ///
    /// 128 ヒットテスト閾値なら除外される α==1 画素が、トリムでは bbox を広げる
    /// ことを示す（コントラスト: α==1 は「不透明として採用」）。
    #[test]
    fn threshold_is_strictly_greater_than_zero_not_128() {
        let (w, h) = (4u32, 1u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        // 左端 x=0 に α==1（>0 だが 128 未満）。右端 x=3 に α==255。
        set_alpha(&mut buf, w * 4, 0, 0, 1);
        set_alpha(&mut buf, w * 4, 3, 0, 255);
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("some opaque");
        // α==1 が含まれるなら bbox は x=0 から始まり幅 4（全幅）。
        assert_eq!(t.trim_offset, Point { x: 0, y: 0 });
        assert_eq!(t.size, Size { w: 4, h: 1 });
    }

    /// α==1 単独でも空にならない（>0 判定の直接確認）。
    #[test]
    fn alpha_one_alone_is_not_empty() {
        let (w, h) = (2u32, 2u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        set_alpha(&mut buf, w * 4, 1, 1, 1); // α==1
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("alpha==1 counts as opaque");
        assert_eq!(t.trim_offset, Point { x: 1, y: 1 });
        assert_eq!(t.size, Size { w: 1, h: 1 });
    }

    /// 全画素 α>0 → trim_offset(0,0)・size==original・pbgra は入力と一致（stride==width*4）。
    #[test]
    fn full_opaque_is_identity() {
        let (w, h) = (3u32, 2u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        for i in 0..(w * h) as usize {
            buf[i * 4] = (i as u8).wrapping_mul(3); // B（適当な色）
            buf[i * 4 + 3] = 255; // A
        }
        let expected = buf.clone();
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("full opaque");
        assert_eq!(t.trim_offset, Point { x: 0, y: 0 });
        assert_eq!(t.size, res.original);
        assert_eq!(t.size, Size { w: 3, h: 2 });
        // 入力 stride == width*4 ゆえ tightly-packed 出力は入力と一致。
        assert_eq!(t.pbgra, expected);
    }

    /// 7: Trimmed は tightly packed（stride==size.w*4・len==stride*size.h）。
    #[test]
    fn trimmed_is_tightly_packed() {
        let (w, h) = (4u32, 4u32);
        let mut buf = vec![0u8; (w * 4 * h) as usize];
        for y in 1..=2 {
            for x in 1..=3 {
                set_alpha(&mut buf, w * 4, x, y, 255);
            }
        }
        let res = Trimmer.trim(&img(w, h, buf));
        let t = res.placement.expect("opaque");
        assert_eq!(t.stride, t.size.w * 4, "tight stride");
        assert_eq!(
            t.pbgra.len(),
            (t.stride * t.size.h) as usize,
            "len == stride * height"
        );
    }

    /// 8: パディング付き入力 stride（stride > width*4）でも正しくトリム＆tightly-packed 出力。
    #[test]
    fn padded_input_stride_trims_correctly() {
        let (w, h) = (3u32, 3u32);
        let stride = w * 4 + 8; // 行末に 8 byte パディング
        let mut buf = vec![0u8; (stride * h) as usize];
        // x∈[1,2] y∈[0,1] に α>0（行頭は stride ベース）。
        for y in 0..=1u32 {
            for x in 1..=2u32 {
                let off = (y * stride + x * 4 + 3) as usize;
                buf[off] = 255;
            }
        }
        // 色の混入も確認: (1,0) に B/G/R を入れる（行頭 y=0 ゆえ x=1 のバイト位置 = 4）。
        let px = 4usize;
        buf[px] = 5;
        buf[px + 1] = 6;
        buf[px + 2] = 7;

        let normalized = NormalizedImage {
            width: w,
            height: h,
            stride,
            pbgra: buf,
        };
        let res = Trimmer.trim(&normalized);
        let t = res.placement.expect("opaque region");
        assert_eq!(t.trim_offset, Point { x: 1, y: 0 });
        assert_eq!(t.size, Size { w: 2, h: 2 });
        // 出力は tightly packed（パディングを含まない）。
        assert_eq!(t.stride, 2 * 4);
        assert_eq!(t.pbgra.len(), (t.stride * t.size.h) as usize);
        // (1,0) の色が出力先頭画素へ正しく転写されている。
        assert_eq!(&t.pbgra[0..4], &[5, 6, 7, 255]);
    }
}
