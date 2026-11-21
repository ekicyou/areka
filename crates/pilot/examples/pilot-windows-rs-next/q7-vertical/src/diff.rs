//! 旧・新の絵（premultiplied BGRA）の比べ。合格の閾値は置かない。
//! 平行移動 (dx, dy) は「旧の (x, y) と新の (x+dx, y+dy) を比べる」。
//! 新が旧を右へ 2 px ずらした絵なら最良のずれは (2, 0)。

pub struct Shot {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DiffReport {
    pub differing_pixels: u64,
    pub max_delta: u8,
    pub best_shift: (i32, i32),
    pub differing_at_best: u64,
    /// 大きさが違い、小さい方に合わせて数えた。
    pub size_mismatch: bool,
}

const RANGE: i32 = 8;

fn px(s: &Shot, x: i64, y: i64) -> Option<[u8; 4]> {
    if x < 0 || y < 0 || x >= s.width as i64 || y >= s.height as i64 {
        return None;
    }
    let i = ((y as usize) * s.width as usize + x as usize) * 4;
    s.bgra.get(i..i + 4).map(|p| [p[0], p[1], p[2], p[3]])
}

/// 範囲外は透明。
fn px_or_clear(s: &Shot, x: i64, y: i64) -> [u8; 4] {
    px(s, x, y).unwrap_or([0; 4])
}

/// 旧の矩形と、ずれで旧の座標へ戻した新の矩形の和を数える（重なりの外へ出入りした墨も違い）。
fn count_at(old: &Shot, new: &Shot, dx: i32, dy: i32) -> u64 {
    let (dx, dy) = (dx as i64, dy as i64);
    let (x0, x1) = (0.min(-dx), (old.width as i64).max(new.width as i64 - dx));
    let (y0, y1) = (0.min(-dy), (old.height as i64).max(new.height as i64 - dy));
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            n += (px_or_clear(old, x, y) != px_or_clear(new, x + dx, y + dy)) as u64;
        }
    }
    n
}

pub fn compare(old: &Shot, new: &Shot) -> DiffReport {
    let w = old.width.min(new.width) as i64;
    let h = old.height.min(new.height) as i64;
    let (mut differing_pixels, mut max_delta) = (0u64, 0u8);
    for y in 0..h {
        for x in 0..w {
            let (a, b) = (px(old, x, y).unwrap(), px(new, x, y).unwrap());
            if a != b {
                differing_pixels += 1;
                for k in 0..4 {
                    max_delta = max_delta.max(a[k].abs_diff(b[k]));
                }
            }
        }
    }
    // 同数なら原点に近いずれを選ぶ。
    let mut best = (u64::MAX, i32::MAX, (0, 0));
    for dy in -RANGE..=RANGE {
        for dx in -RANGE..=RANGE {
            let key = (count_at(old, new, dx, dy), dx.abs() + dy.abs());
            if key < (best.0, best.1) {
                best = (key.0, key.1, (dx, dy));
            }
        }
    }
    DiffReport {
        differing_pixels,
        max_delta,
        best_shift: best.2,
        differing_at_best: best.0,
        size_mismatch: (old.width, old.height) != (new.width, new.height),
    }
}

/// 違う画素を赤・同じ墨（旧のアルファが 0 でない）を灰・それ以外を白で、旧の大きさの PNG に書く。
pub fn write_diff_png(
    old: &Shot,
    new: &Shot,
    shift: (i32, i32),
    path: &std::path::Path,
) -> std::io::Result<()> {
    let rgba = diff_rgba(old, new, shift);
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(std::fs::File::create(path)?),
        old.width,
        old.height,
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(std::io::Error::other)?;
    w.write_image_data(&rgba).map_err(std::io::Error::other)
}

fn diff_rgba(old: &Shot, new: &Shot, shift: (i32, i32)) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(old.width as usize * old.height as usize * 4);
    for y in 0..old.height as i64 {
        for x in 0..old.width as i64 {
            let a = px(old, x, y).unwrap();
            let b = px_or_clear(new, x + shift.0 as i64, y + shift.1 as i64);
            rgba.extend_from_slice(&match b == a {
                true if a[3] != 0 => [128, 128, 128, 255],
                true => [255, 255, 255, 255],
                false => [255, 0, 0, 255],
            });
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(w: u32, h: u32, ink: &[(u32, u32)]) -> Shot {
        let mut bgra = vec![0; (w * h * 4) as usize];
        for &(x, y) in ink {
            let i = ((y * w + x) * 4) as usize;
            bgra[i..i + 4].copy_from_slice(&[10, 20, 30, 255]);
        }
        Shot {
            width: w,
            height: h,
            bgra,
        }
    }

    #[test]
    fn identical_is_zero() {
        let a = shot(32, 32, &[(5, 5), (6, 5), (7, 9)]);
        let r = compare(&a, &shot(32, 32, &[(5, 5), (6, 5), (7, 9)]));
        assert_eq!(
            (r.differing_pixels, r.max_delta, r.best_shift),
            (0, 0, (0, 0))
        );
        assert_eq!(r.differing_at_best, 0);
        assert!(!r.size_mismatch);
    }

    #[test]
    fn one_pixel_differs() {
        let a = shot(32, 32, &[(5, 5)]);
        let mut b = shot(32, 32, &[(5, 5)]);
        b.bgra[(5 * 32 + 5) * 4 + 3] = 200;
        let r = compare(&a, &b);
        assert_eq!((r.differing_pixels, r.max_delta), (1, 55));
    }

    #[test]
    fn shifted_by_two_finds_offset() {
        let a = shot(32, 32, &[(10, 10), (11, 10), (10, 12), (14, 15)]);
        let b = shot(32, 32, &[(12, 10), (13, 10), (12, 12), (16, 15)]);
        let r = compare(&a, &b);
        assert!(r.differing_pixels > 0);
        assert_eq!((r.best_shift, r.differing_at_best), ((2, 0), 0));
    }

    /// ずらしで重なりの外へ出た墨も違いとして数える（重なりだけ数えると隠れる）。
    #[test]
    fn edge_ink_is_not_hidden_by_shift() {
        let a = shot(32, 32, &[]);
        let b = shot(32, 32, &[(0, 0), (1, 0)]);
        let r = compare(&a, &b);
        assert_eq!((r.best_shift, r.differing_at_best), ((0, 0), 2));
    }

    /// 透明どうし（範囲外は透明と見る）は白。
    #[test]
    fn png_out_of_bounds_empty_is_white() {
        let e = shot(4, 4, &[]);
        assert!(
            diff_rgba(&e, &e, (8, 0))
                .chunks(4)
                .all(|p| p == [255, 255, 255, 255])
        );
    }

    #[test]
    fn size_mismatch_uses_overlap_and_png_writes() {
        let a = shot(32, 32, &[(5, 5)]);
        let b = shot(16, 40, &[(5, 5)]);
        let r = compare(&a, &b);
        assert!(r.size_mismatch);
        assert_eq!(r.differing_pixels, 0);
        let p = std::env::current_exe().unwrap().with_extension("diff.png");
        write_diff_png(&a, &b, (0, 0), &p).unwrap();
        assert!(std::fs::metadata(&p).unwrap().len() > 0);
        let _ = std::fs::remove_file(p);
    }
}
