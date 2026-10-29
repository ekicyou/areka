//! 既定バルーンの絵柄の値と、面と印の絵の描き方（spec: default-balloon-selfmade）。

/// α 付き（乗算していない）RGBA の画素の並び。
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// 色は RGB を 0〜255、α を 0〜1 で持つ。
type Color = [f32; 4];
type P = (f32, f32);

// 絵柄の値（design.md「絵柄の値」・議題 2 の案 A「白と灰」）。
/// 地: 白・不透明。
const FILL: Color = [255.0, 255.0, 255.0, 1.0];
/// 縁: 灰。
const BORDER: Color = [150.0, 154.0, 164.0, 1.0];
/// 縁の幅（距離が −1.5〜0 の帯）。
const BORDER_WIDTH: f32 = 1.5;
/// 角の丸み（入力の箱も同じ）。
const RADIUS: f32 = 8.0;
/// 箱の外形を面の端から内へ寄せる幅。
const INSET: f32 = 0.5;
/// しっぽの底辺を箱の縁から内へ寄せる幅。
const TAIL_BASE_INSET: f32 = 6.0;
/// しっぽの底辺の上端と下端。
const TAIL_TOP: f32 = 20.0;
const TAIL_BOTTOM: f32 = 38.0;
/// しっぽの先の高さ（先は面の端）。
const TAIL_TIP_Y: f32 = 23.0;
/// しっぽの幅: 本体側と相方側。
const TAIL_WIDTH_SAKURA: f32 = 9.0;
const TAIL_WIDTH_KERO: f32 = 10.0;
/// 矢印は縁の灰、SSTP とオンラインは青。
const ARROW_COLOR: Color = BORDER;
const ACCENT: Color = [22.0, 143.0, 231.0, 1.0];
/// SSTP の印の角の丸み。
const SSTP_RADIUS: f32 = 2.0;
/// オンラインの印: 点の中心の x・y、半径、灯っていない点の α。
const ONLINE_DOTS_X: [f32; 3] = [26.0, 38.5, 51.0];
const ONLINE_DOT_Y: f32 = 18.0;
const ONLINE_DOT_RADIUS: f32 = 4.0;
const ONLINE_DIM_ALPHA: f32 = 0.25;
/// オンラインの印の 1 コマのミリ秒。
pub const ONLINE_FRAME_MS: u32 = 300;

const CLEAR: Color = [0.0; 4];

fn sqrt_len(x: f32, y: f32) -> f32 {
    // hypot は環境の数学ライブラリに依るので使わない（sqrt は正しく丸められる）
    (x * x + y * y).sqrt()
}

/// 角丸の箱 (x0,y0)〜(x1,y1) までの符号つき距離（内が負）。
fn sd_round_box(p: P, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (bx, by) = ((x1 - x0) / 2.0, (y1 - y0) / 2.0);
    let qx = (p.0 - cx).abs() - bx + r;
    let qy = (p.1 - cy).abs() - by + r;
    sqrt_len(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

/// 三角 a・b・c までの符号つき距離（内が負）。
fn sd_tri(p: P, a: P, b: P, c: P) -> f32 {
    let sub = |u: P, v: P| (u.0 - v.0, u.1 - v.1);
    let dot = |u: P, v: P| u.0 * v.0 + u.1 * v.1;
    let (e0, e1, e2) = (sub(b, a), sub(c, b), sub(a, c));
    let (v0, v1, v2) = (sub(p, a), sub(p, b), sub(p, c));
    let pq = |v: P, e: P| {
        let t = (dot(v, e) / dot(e, e)).clamp(0.0, 1.0);
        (v.0 - e.0 * t, v.1 - e.1 * t)
    };
    let (q0, q1, q2) = (pq(v0, e0), pq(v1, e1), pq(v2, e2));
    let s = (e0.0 * e2.1 - e0.1 * e2.0).signum();
    let d0 = (dot(q0, q0), s * (v0.0 * e0.1 - v0.1 * e0.0));
    let d1 = (dot(q1, q1), s * (v1.0 * e1.1 - v1.1 * e1.0));
    let d2 = (dot(q2, q2), s * (v2.0 * e2.1 - v2.1 * e2.0));
    let dx = d0.0.min(d1.0).min(d2.0);
    let dy = d0.1.min(d1.1).min(d2.1);
    -dx.sqrt() * dy.signum()
}

/// 座標から色を決める関数を、画素ごとに 4×4 の標本の α で重みを付けた平均で塗る。
fn raster(width: u32, height: u32, f: impl Fn(P) -> Color) -> Picture {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let mut acc = [0.0f32; 4];
            for j in 0..4 {
                for i in 0..4 {
                    let p = (
                        x as f32 + (i as f32 + 0.5) / 4.0,
                        y as f32 + (j as f32 + 0.5) / 4.0,
                    );
                    let c = f(p);
                    acc[0] += c[0] * c[3];
                    acc[1] += c[1] * c[3];
                    acc[2] += c[2] * c[3];
                    acc[3] += c[3];
                }
            }
            if acc[3] <= 0.0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let u = |v: f32| (v / acc[3]).round().clamp(0.0, 255.0) as u8;
                let a = (acc[3] / 16.0 * 255.0).round().clamp(0.0, 255.0) as u8;
                rgba.extend_from_slice(&[u(acc[0]), u(acc[1]), u(acc[2]), a]);
            }
        }
    }
    Picture {
        width,
        height,
        rgba,
    }
}

/// 距離から地・縁・外を決める。
fn shade(sd: f32) -> Color {
    if sd < -BORDER_WIDTH {
        FILL
    } else if sd < 0.0 {
        BORDER
    } else {
        CLEAR
    }
}

/// しっぽ付きの面。しっぽ側の辺はしっぽの幅だけ内へ寄せる。
fn face(width: u32, height: u32, tail_right: bool, tail_width: f32) -> Picture {
    let (w, h) = (width as f32, height as f32);
    let (x0, x1) = if tail_right {
        (INSET, w - tail_width - INSET)
    } else {
        (tail_width + INSET, w - INSET)
    };
    let tri = if tail_right {
        let bx = x1 - TAIL_BASE_INSET;
        ((bx, TAIL_TOP), (bx, TAIL_BOTTOM), (w - INSET, TAIL_TIP_Y))
    } else {
        let bx = x0 + TAIL_BASE_INSET;
        ((bx, TAIL_BOTTOM), (bx, TAIL_TOP), (INSET, TAIL_TIP_Y))
    };
    raster(width, height, |p| {
        let b = sd_round_box(p, x0, INSET, x1, h - INSET, RADIUS);
        shade(b.min(sd_tri(p, tri.0, tri.1, tri.2)))
    })
}

/// 入力の箱（400×63・しっぽなし）。
fn input_box() -> Picture {
    raster(400, 63, |p| {
        shade(sd_round_box(
            p,
            INSET,
            INSET,
            400.0 - INSET,
            63.0 - INSET,
            RADIUS,
        ))
    })
}

/// 矢印（11×10）。下は上を上下に返した三角。
fn arrow(up: bool) -> Picture {
    raster(11, 10, |p| {
        let d = if up {
            sd_tri(p, (1.0, 8.5), (10.0, 8.5), (5.5, 1.5))
        } else {
            sd_tri(p, (1.0, 1.5), (5.5, 8.5), (10.0, 1.5))
        };
        if d < 0.0 { ARROW_COLOR } else { CLEAR }
    })
}

/// SSTP の印（7×9・角丸の箱を青で塗る）。
fn sstp() -> Picture {
    raster(7, 9, |p| {
        let d = sd_round_box(p, INSET, INSET, 7.0 - INSET, 9.0 - INSET, SSTP_RADIUS);
        if d < 0.0 { ACCENT } else { CLEAR }
    })
}

/// PNG で持つ 13 枚（拡張子なしの名前, 絵）を名前順で返す。
pub fn png_pictures() -> Vec<(&'static str, Picture)> {
    let s = TAIL_WIDTH_SAKURA;
    let k = TAIL_WIDTH_KERO;
    vec![
        ("arrow0", arrow(true)),
        ("arrow1", arrow(false)),
        ("balloonc0", input_box()),
        ("balloonc1", input_box()),
        ("balloonc2", input_box()),
        ("balloonc3", input_box()),
        ("balloonk0", face(326, 96, true, k)),
        ("balloonk1", face(326, 96, false, k)),
        ("balloons0", face(326, 169, true, s)),
        ("balloons1", face(326, 169, false, s)),
        ("balloons2", face(326, 348, true, s)),
        ("balloons3", face(326, 348, false, s)),
        ("sstp", sstp()),
    ]
}

/// オンラインの印の 3 コマ（77×36）。コマ k は点 k を不透明、ほかの点を α 0.25 で描く。
pub fn online_frames() -> Vec<Picture> {
    (0..ONLINE_DOTS_X.len())
        .map(|k| {
            raster(77, 36, |p| {
                for (i, cx) in ONLINE_DOTS_X.iter().enumerate() {
                    if sqrt_len(p.0 - cx, p.1 - ONLINE_DOT_Y) - ONLINE_DOT_RADIUS < 0.0 {
                        let a = if i == k { 1.0 } else { ONLINE_DIM_ALPHA };
                        return [ACCENT[0], ACCENT[1], ACCENT[2], a];
                    }
                }
                CLEAR
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(p: &Picture, x: u32, y: u32) -> u8 {
        p.rgba[((y * p.width + x) * 4 + 3) as usize]
    }

    fn find<'a>(all: &'a [(&'static str, Picture)], name: &str) -> &'a Picture {
        &all.iter()
            .find(|(n, _)| *n == name)
            .expect("missing picture")
            .1
    }

    #[test]
    fn png_names_and_sizes_match_the_file_table() {
        let all = png_pictures();
        let got: Vec<(&str, u32, u32)> = all.iter().map(|(n, p)| (*n, p.width, p.height)).collect();
        assert_eq!(
            got,
            vec![
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
                ("sstp", 7, 9),
            ]
        );
        for (n, p) in &all {
            assert_eq!(p.rgba.len(), (p.width * p.height * 4) as usize, "{n}");
        }
    }

    #[test]
    fn faces_have_opaque_clear_and_soft_pixels() {
        for (n, p) in &png_pictures() {
            let a: Vec<u8> = p.rgba.chunks(4).map(|c| c[3]).collect();
            assert!(a.contains(&255), "{n} opaque");
            assert!(a.contains(&0), "{n} clear");
            assert!(a.iter().any(|v| *v != 0 && *v != 255), "{n} soft");
        }
    }

    #[test]
    fn colors_follow_the_value_table() {
        let all = png_pictures();
        let s0 = find(&all, "balloons0");
        let i = ((80 * s0.width + 150) * 4) as usize;
        assert_eq!(&s0.rgba[i..i + 4], &[255, 255, 255, 255], "fill white");
        // 上の辺の縁の帯（y 0.5〜2.0）の中の画素は灰で不透明
        let i = ((1 * s0.width + 150) * 4) as usize;
        assert_eq!(&s0.rgba[i..i + 4], &[150, 154, 164, 255], "border gray");
        let ar = find(&all, "arrow0");
        let i = ((7 * ar.width + 5) * 4) as usize;
        assert_eq!(&ar.rgba[i..i + 4], &[150, 154, 164, 255], "arrow gray");
        let ss = find(&all, "sstp");
        let i = ((4 * ss.width + 3) * 4) as usize;
        assert_eq!(&ss.rgba[i..i + 4], &[22, 143, 231, 255], "sstp blue");
    }

    #[test]
    fn tails_point_right_on_even_faces_and_left_on_odd() {
        let all = png_pictures();
        // しっぽ側の端は、しっぽの先（y 23）で描かれ、しっぽの外（y 10）で透明。
        // 反対の端は箱の縁なので y 10 でも描かれる。
        for (n, right, w) in [
            ("balloons0", true, 9),
            ("balloons1", false, 9),
            ("balloons2", true, 9),
            ("balloons3", false, 9),
            ("balloonk0", true, 10),
            ("balloonk1", false, 10),
        ] {
            let p = find(&all, n);
            let (tail_x, other_x) = if right {
                (p.width - 2, 1)
            } else {
                (1, p.width - 2)
            };
            assert!(alpha(p, tail_x, 23) > 0, "{n} tail tip");
            assert_eq!(alpha(p, tail_x, 10), 0, "{n} outside tail");
            assert!(alpha(p, other_x, 10) > 0, "{n} box edge");
            // しっぽの幅の内側（端から w 画素の帯）は y 10 で透明
            let strip_x = if right { p.width - w } else { w - 1 };
            assert_eq!(alpha(p, strip_x, 10), 0, "{n} tail width");
        }
    }

    #[test]
    fn input_boxes_are_the_same_picture() {
        let all = png_pictures();
        let c0 = &find(&all, "balloonc0").rgba;
        for n in ["balloonc1", "balloonc2", "balloonc3"] {
            assert_eq!(&find(&all, n).rgba, c0, "{n}");
        }
    }

    #[test]
    fn arrows_are_flipped_vertically() {
        let all = png_pictures();
        let (up, down) = (find(&all, "arrow0"), find(&all, "arrow1"));
        // 上の矢印は先が上（y 2 の中央が灯り y 8 の端も灯る）、下は逆
        assert!(alpha(up, 5, 2) > 0 && alpha(up, 1, 2) == 0);
        assert!(alpha(down, 5, 7) > 0 && alpha(down, 1, 7) == 0);
    }

    #[test]
    fn online_frame_k_lights_dot_k_only() {
        let frames = online_frames();
        assert_eq!(frames.len(), 3);
        let centers = [(26, 18), (38, 18), (51, 18)];
        for (k, f) in frames.iter().enumerate() {
            assert_eq!((f.width, f.height), (77, 36));
            for (i, (x, y)) in centers.iter().enumerate() {
                let want = if i == k { 255 } else { 64 };
                assert_eq!(alpha(f, *x, *y), want, "frame {k} dot {i}");
                let j = ((y * f.width + x) * 4) as usize;
                assert_eq!(&f.rgba[j..j + 3], &[22, 143, 231], "frame {k} dot {i}");
            }
            assert_eq!(alpha(f, 2, 2), 0, "frame {k} outside");
        }
    }

    #[test]
    fn drawing_is_repeatable() {
        let a: Vec<Vec<u8>> = png_pictures().into_iter().map(|(_, p)| p.rgba).collect();
        let b: Vec<Vec<u8>> = png_pictures().into_iter().map(|(_, p)| p.rgba).collect();
        assert!(a == b);
    }
}
