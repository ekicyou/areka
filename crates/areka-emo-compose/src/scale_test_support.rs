use crate::composed::ComposedSurface;

/// 作者基準 DPI（ukadoc 正典既定）。DPI 対照表の分母。
pub(super) const AUTHOR_DPI: u32 = 96;

/// 決定的なテスト画像（線形合同法・premultiplied 不変条件 B,G,R ≤ A を満たす）。
///
/// 実機相当の外形（emo2 native 382×547）を扱うため、画素表を書き下す形ではなく生成器を使う。
/// 同一 `(w, h)` は常に同一バイト列を返す（実時間・乱数源に依存しない・要件 6.2/6.3）。
///
/// α が画素ごとに変わるため、**α を特別扱いする実装**（4 チャンネル同式でない実装）を
/// 弁別できる。全不透明の画像が要る場合は [`opaque_gradient_surface`] を使う。
pub(super) fn deterministic_surface(w: u32, h: u32) -> ComposedSurface {
    let mut s = ComposedSurface::new(w, h);
    let stride = s.stride() as usize;
    let bytes = s.bytes_mut();
    let mut state: u32 = 0x9E37_79B9;
    for y in 0..h as usize {
        for x in 0..w as usize {
            // 線形合同法（Numerical Recipes 係数）。決定的・種は固定。
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let a = (state >> 24) as u8;
            // premultiplied: 各色成分を α で頭打ちにする（B,G,R ≤ A）。
            let ch = |v: u8| ((v as u16 * a as u16) / 255) as u8;
            let px = [
                ch((state >> 16) as u8),
                ch((state >> 8) as u8),
                ch(state as u8),
                a,
            ];
            let o = y * stride + x * 4;
            bytes[o..o + 4].copy_from_slice(&px);
        }
    }
    s
}

/// 全不透明（α=255）の座標由来グラデーション（`salt` で内容を振る）。
///
/// [`deterministic_surface`] は α が乱れるため「不透明画素が実在する」という非空性の主張に
/// 使えない（偶然どの画素も α<255 になり得る）。本生成器は α を 255 に固定し、B・G・R は
/// 座標で変化させるため、**不透明かつ一様でない**——バイト等価の主張が空虚にならない
/// 入力として使える。premultiplied 不変条件（B,G,R ≤ A = 255）は自明に満たす。
pub(super) fn opaque_gradient_surface(w: u32, h: u32, salt: u8) -> ComposedSurface {
    let mut s = ComposedSurface::new(w, h);
    let stride = s.stride() as usize;
    let bytes = s.bytes_mut();
    for y in 0..h as usize {
        for x in 0..w as usize {
            let b = (x as u8).wrapping_mul(3).wrapping_add(salt);
            let g = (y as u8).wrapping_mul(5).wrapping_add(salt);
            let r = ((x + y) as u8).wrapping_mul(7).wrapping_add(salt);
            let o = y * stride + x * 4;
            bytes[o..o + 4].copy_from_slice(&[b, g, r, 0xFF]);
        }
    }
    s
}
