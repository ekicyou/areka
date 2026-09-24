use super::*;
use Class::*;

/// w×h の premultiplied BGRA（x が `xs` の範囲なら `color`・α=255、他は透明）。
fn image(w: u32, h: u32, xs: std::ops::Range<u32>, color: Bgr) -> Vec<u8> {
    let mut v = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in xs.clone() {
            let i = ((y * w + x) * 4) as usize;
            v[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
        }
    }
    v
}

#[test]
fn signature_and_picture_rule_on_tiny_images() {
    let (red, blue) = ([0, 0, 255], [255, 0, 0]);
    fn img(bytes: &[u8]) -> Img<'_> {
        Img {
            w: 64,
            h: 64,
            stride: 256,
            bytes,
        }
    }
    let (p, q) = (image(64, 64, 0..48, red), image(64, 64, 16..64, blue));
    // 升 16 列（1 升 4 px）: 0–3 列は P だけ・4–11 列は両方・12–15 列は Q だけ（各列 16 升）。
    let sig = derive(&img(&p), &img(&q)).unwrap();
    assert_eq!(
        (sig.only_p.len(), sig.only_q.len(), sig.both.len()),
        (64, 64, 128)
    );

    let rect = RECT {
        left: 0,
        top: 0,
        right: 64,
        bottom: 64,
    };
    assert_eq!(classify_picture(&sig, &rect, &p, 256).0, P);
    assert_eq!(classify_picture(&sig, &rect, &q, 256).0, Q);
    assert_eq!(
        classify_picture(&sig, &rect, &vec![0u8; 64 * 64 * 4], 256).0,
        Neither
    );
    let mut over = q.clone(); // P を Q の上に描いた絵
    over[..].chunks_mut(4).zip(p.chunks(4)).for_each(|(o, s)| {
        if s[3] == 255 {
            o.copy_from_slice(s)
        }
    });
    assert_eq!(classify_picture(&sig, &rect, &over, 256).0, Both);

    // 同じ絵どうしは見分けられない。
    assert!(derive(&img(&p), &img(&p)).is_err());

    // 当たり判定の規則（「P だけ」が無い対では Both が出ない）。
    assert_eq!(hit_rule(true, 1.0, 1.0, 1.0), Both);
    assert_eq!(hit_rule(false, f32::NAN, 1.0, 1.0), Q);
    assert_eq!(hit_rule(false, f32::NAN, 0.0, 1.0), P);
}

/// 「P だけ」が無い対の絵の「両方」は b と ab_p で決まる（ab_q ではない）。
#[test]
fn picture_both_without_only_p_uses_ab_p() {
    let nan = f32::NAN;
    assert_eq!(picture_rule(false, nan, 0.5, 0.5, 0.0), Both);
    assert_eq!(
        picture_rule(false, nan, 0.5, 0.49, 0.5),
        Unmeasurable(Why::Ambiguous)
    );
    assert_eq!(
        picture_rule(false, nan, 0.49, 0.5, 0.0),
        Unmeasurable(Why::Ambiguous)
    );
}
