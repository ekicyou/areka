use super::*;
use super::test_support::*;

/// premultiplied BGRA 画素列（行優先）から `ComposedSurface` を組む（テスト補助）。
fn surface_of(w: u32, h: u32, px: &[[u8; 4]]) -> ComposedSurface {
    assert_eq!(px.len(), (w * h) as usize, "画素数と外形が一致すること");
    let mut s = ComposedSurface::new(w, h);
    let stride = s.stride() as usize;
    let bytes = s.bytes_mut();
    for (i, p) in px.iter().enumerate() {
        let (x, y) = (i % w as usize, i / w as usize);
        let o = y * stride + x * 4;
        bytes[o..o + 4].copy_from_slice(p);
    }
    s
}

/// 不透明グレー画素（B=G=R=v・A=255）。premultiplied 不変条件 B,G,R ≤ A を満たす。
fn gray(v: u8) -> [u8; 4] {
    [v, v, v, 255]
}

/// グレー値表を BGRA バイト列へ展開する（golden 比較用）。
fn gray_bytes(vals: &[u8]) -> Vec<u8> {
    vals.iter().flat_map(|&v| gray(v)).collect()
}

/// f64 参照 bilinear（**テスト専用オラクル**・本番経路には浮動小数を一切持ち込まない）。
///
/// 本実装と同一の写像規約——画素中心を合わせた den/num の有理逆写像
/// `src = (d + 0.5)·den/num − 0.5`・隣接 2 点の独立エッジクランプ——を浮動小数で解く。
/// golden 値を「観測結果の追認」でなく**独立に導出した真値との一致**として検証するために用いる。
/// 整数実装との差は重み量子化（1/65536）＋最終丸め（±0.5）に収まる。
fn oracle(src: &ComposedSurface, scale: ScaleRatio, dx: u32, dy: u32, c: usize) -> f64 {
    let ratio = scale.den as f64 / scale.num as f64;
    let sx = (dx as f64 + 0.5) * ratio - 0.5;
    let sy = (dy as f64 + 0.5) * ratio - 0.5;
    let (x0, y0) = (sx.floor(), sy.floor());
    let (fx, fy) = (sx - x0, sy - y0);
    let at = |x: f64, y: f64| -> f64 {
        let cx = x.max(0.0).min((src.width() - 1) as f64) as usize;
        let cy = y.max(0.0).min((src.height() - 1) as f64) as usize;
        src.bytes()[cy * src.stride() as usize + cx * 4 + c] as f64
    };
    let top = at(x0, y0) * (1.0 - fx) + at(x0 + 1.0, y0) * fx;
    let bot = at(x0, y0 + 1.0) * (1.0 - fx) + at(x0 + 1.0, y0 + 1.0) * fx;
    top * (1.0 - fy) + bot * fy
}

/// 出力全画素がオラクル真値と量子化誤差内で一致することを確認する（golden 値の妥当性検証）。
fn assert_matches_oracle(src: &ComposedSurface, scale: ScaleRatio, out: &ComposedSurface) {
    for dy in 0..out.height() {
        for dx in 0..out.width() {
            for c in 0..4usize {
                let got =
                    out.bytes()[dy as usize * out.stride() as usize + dx as usize * 4 + c];
                let want = oracle(src, scale, dx, dy, c);
                assert!(
                    (got as f64 - want).abs() <= 0.6,
                    "({dx},{dy}) ch{c}: 整数実装 {got} と f64 オラクル {want} が乖離"
                );
            }
        }
    }
}

/// 要件 7.2: 恒等 k（1/1）は入力バイトの厳密コピー（既存 golden が一切変化しない構造保証）。
#[test]
fn resample_identity_is_byte_copy() {
    let src = surface_of(
        3,
        2,
        &[
            [1, 2, 3, 250],
            [10, 20, 30, 255],
            [0, 0, 0, 0],
            [77, 66, 55, 200],
            [255, 255, 255, 255],
            [9, 8, 7, 128],
        ],
    );
    let mut out = ComposedSurface::new(0, 0);
    resample(&src, ScaleRatio::ONE, &mut out);
    assert_eq!((out.width(), out.height()), (3, 2));
    assert_eq!(out.stride(), src.stride());
    assert_eq!(out.bytes(), src.bytes(), "恒等はバイト恒等コピー");

    // 96/96 のように既約化で 1/1 になる比も同じ恒等経路を通る。
    let mut out2 = ComposedSurface::new(0, 0);
    resample(
        &src,
        ScaleRatio::new(AUTHOR_DPI, AUTHOR_DPI).unwrap(),
        &mut out2,
    );
    assert_eq!(out2.bytes(), src.bytes());
}

/// 要件 2.5: 出力外形は丸め単一権威 `scaled_extent` と厳密一致する（欠け・切り捨てなし）。
#[test]
fn resample_extent_matches_scaled_extent() {
    let src = surface_of(4, 3, &[gray(64); 12]);
    for (num, den) in [
        (2u32, 1u32),
        (5, 4),
        (3, 2),
        (7, 4),
        (1, 2),
        (1, 3),
        (1, 100),
    ] {
        let k = ScaleRatio::new(num, den).unwrap();
        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);
        assert_eq!(
            (out.width(), out.height()),
            k.scaled_extent(src.width(), src.height()),
            "k={num}/{den}"
        );
        assert_eq!(out.stride(), out.width() * 4);
        assert_eq!(
            out.bytes().len(),
            (out.stride() * out.height()) as usize,
            "k={num}/{den}"
        );
    }
}

/// 要件 2.1: 整数 2 倍拡大の golden（bilinear の重みが 1/4 刻みで厳密＝丸め非依存）。
///
/// 2×2 グレー（0,80 / 160,240）→ 4×4。端は clamp ゆえ原画素そのもの、内側は
/// 0.75/0.25 の混合になる（手計算値とオラクルの双方で固定する）。
#[test]
fn resample_two_times_matches_golden() {
    let src = surface_of(2, 2, &[gray(0), gray(80), gray(160), gray(240)]);
    let k = ScaleRatio::new(2, 1).unwrap();
    let mut out = ComposedSurface::new(0, 0);
    resample(&src, k, &mut out);

    assert_eq!((out.width(), out.height()), (4, 4));
    assert_matches_oracle(&src, k, &out);
    #[rustfmt::skip]
    let expect = gray_bytes(&[
          0,  20,  60,  80,
         40,  60, 100, 120,
        120, 140, 180, 200,
        160, 180, 220, 240,
    ]);
    assert_eq!(out.bytes(), expect.as_slice());
}

/// 要件 2.1/2.5: 非整数 k（5/4）の決定論 golden＋反復不変（同一入力→同一バイト）。
///
/// 4 チャンネルを別値にした 4×4 → 5×5。チャンネル取り違え（BGRA 順の崩れ）も検出する。
#[test]
fn resample_five_quarters_is_deterministic_golden() {
    const V: [u8; 16] = [
        10, 200, 30, 240, 250, 5, 128, 64, 33, 99, 210, 7, 180, 21, 66, 143,
    ];
    let px: Vec<[u8; 4]> = V.iter().map(|&v| [v, v / 2, v / 4, 255]).collect();
    let src = surface_of(4, 4, &px);
    let k = ScaleRatio::new(5, 4).unwrap();

    let mut out = ComposedSurface::new(0, 0);
    resample(&src, k, &mut out);
    assert_eq!((out.width(), out.height()), (5, 5));
    assert_matches_oracle(&src, k, &out);

    // golden はオラクル（上の `assert_matches_oracle`）で真値との一致を検証済みの値。
    // 四隅は入力四隅そのもの（エッジクランプ）・(0,2) は重み厳密 1/2 の中点 115=(200+30)/2。
    #[rustfmt::skip]
    let expect: [u8; 100] = [
         10,   5,   2, 255,  143,  71,  36, 255,  115,  58,  29, 255,   93,  46,  23, 255,  240, 120,  60, 255,
        178,  89,  44, 255,   98,  49,  24, 255,   81,  40,  20, 255,  104,  52,  26, 255,  117,  58,  29, 255,
        142,  71,  35, 255,   79,  39,  19, 255,  111,  55,  27, 255,  129,  64,  32, 255,   36,  18,   9, 255,
         77,  38,  19, 255,   76,  38,  19, 255,  121,  60,  30, 255,  131,  65,  32, 255,   48,  23,  11, 255,
        180,  90,  45, 255,   69,  34,  17, 255,   44,  22,  11, 255,   89,  44,  22, 255,  143,  71,  35, 255,
    ];
    assert_eq!(out.bytes(), &expect[..], "非整数 k の golden");
    // 四隅＝入力四隅の恒等（クランプの独立確認）。
    assert_eq!(&out.bytes()[..4], &[10, 5, 2, 255]);
    assert_eq!(&out.bytes()[96..], &[143, 71, 35, 255]);

    // 反復不変（決定論）: 同一入力の再実行が同一バイトを返す。
    let mut again = ComposedSurface::new(0, 0);
    resample(&src, k, &mut again);
    assert_eq!(again.bytes(), out.bytes(), "同一 (src, k) はバイト決定論");
}

/// 要件 2.5: 縮小（k<1）も外形・内容が決定論的（4×4 → 2×2・オラクル一致）。
#[test]
fn resample_downscale_matches_oracle_and_golden() {
    #[rustfmt::skip]
    let v = [
          0,  40,  80, 120,
         10,  50,  90, 130,
        200, 210, 220, 230,
        240, 250, 255,   5,
    ];
    let px: Vec<[u8; 4]> = v.iter().map(|&x| gray(x)).collect();
    let src = surface_of(4, 4, &px);
    let k = ScaleRatio::new(1, 2).unwrap();

    let mut out = ComposedSurface::new(0, 0);
    resample(&src, k, &mut out);
    assert_eq!((out.width(), out.height()), (2, 2));
    assert_matches_oracle(&src, k, &out);
    // 1/2 縮小は 2×2 ブロックの厳密平均（重みが全て 1/2）:
    // (0+40+10+50)/4=25・(80+120+90+130)/4=105・(200+210+240+250)/4=225・
    // (220+230+255+5)/4=177.5 → round half up → 178（丸め規約が効く画素）。
    assert_eq!(out.bytes(), gray_bytes(&[25, 105, 225, 178]).as_slice());
}

/// 設計 D5: premultiplied ドメインの不変条件（B,G,R ≤ A）が補間後も保たれる。
///
/// bilinear は重み和が厳密に 1（65536）の凸結合であり、丸めも単調ゆえ
/// premultiplied を崩さない（α を別扱いしない＝非乗算化しない証拠）。
#[test]
fn resample_preserves_premultiplied_invariant() {
    // 半透明・全透明・不透明の混在（各画素 B,G,R ≤ A）。
    let px = [
        [0, 0, 0, 0],
        [128, 64, 32, 128],
        [255, 255, 255, 255],
        [10, 5, 1, 10],
        [200, 100, 50, 200],
        [0, 0, 0, 0],
        [7, 7, 7, 7],
        [64, 32, 16, 64],
        [255, 0, 0, 255],
    ];
    let src = surface_of(3, 3, &px);
    for (num, den) in [(2u32, 1u32), (5, 4), (7, 3), (1, 2)] {
        let k = ScaleRatio::new(num, den).unwrap();
        let mut out = ComposedSurface::new(0, 0);
        resample(&src, k, &mut out);
        for (i, p) in out.bytes().chunks_exact(4).enumerate() {
            assert!(
                p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3],
                "k={num}/{den} 画素{i} が premultiplied を破っている: {p:?}"
            );
        }
    }
}

/// 設計「Risks」: 端画素の外挿はエッジクランプで固定（決定論・外挿しない）。
#[test]
fn resample_clamps_at_edges() {
    // 1×1 の拡大は全画素が原画素と同一（クランプのみ＝外挿値が混ざらない）。
    let one = surface_of(1, 1, &[[12, 34, 56, 200]]);
    let mut out = ComposedSurface::new(0, 0);
    resample(&one, ScaleRatio::new(3, 1).unwrap(), &mut out);
    assert_eq!((out.width(), out.height()), (3, 3));
    for p in out.bytes().chunks_exact(4) {
        assert_eq!(p, [12, 34, 56, 200], "1×1 の拡大は全画素がクランプ複製");
    }

    // 2×1 の 4 倍拡大 → 8×4。縦は 1 行しかないため全行が同一（縦クランプ）。
    let two = surface_of(2, 1, &[gray(0), gray(255)]);
    let mut wide = ComposedSurface::new(0, 0);
    let k4 = ScaleRatio::new(4, 1).unwrap();
    resample(&two, k4, &mut wide);
    assert_eq!((wide.width(), wide.height()), (8, 4));
    assert_matches_oracle(&two, k4, &wide);
    let stride = wide.stride() as usize;
    for dy in 1..4usize {
        assert_eq!(
            &wide.bytes()[dy * stride..(dy + 1) * stride],
            &wide.bytes()[..stride],
            "単一行入力の拡大は全行がクランプ複製（行 {dy}）"
        );
    }

    // 横方向: 両端は原画素そのもの（範囲外を混ぜない）・内側は 1/8 刻みの厳密混合。
    // src 座標 = (d+0.5)/4 − 0.5 → d=0,1 は負（→0 クランプ）・d=6,7 は 1 以上（→1 クランプ）。
    let row: Vec<u8> = wide.bytes()[..stride]
        .chunks_exact(4)
        .map(|p| p[0])
        .collect();
    assert_eq!(row.first(), Some(&0), "左端は原画素 0（負側外挿なし）");
    assert_eq!(row.last(), Some(&255), "右端は原画素 255（右側外挿なし）");
    assert!(
        row.windows(2).all(|w| w[0] <= w[1]),
        "単調（外挿の跳ねなし）: {row:?}"
    );
    // 0.125/0.375/0.625/0.875 × 255 = 31.875/95.625/159.375/223.125（round half up）。
    assert_eq!(row, vec![0, 0, 32, 96, 159, 223, 255, 255]);
}

/// 事前条件違反（外形ゼロ）でもパニックせず、空の出力と `warn!` を返す（log-first）。
#[test]
fn resample_zero_extent_is_empty_and_warns() {
    use crate::log_capture::capture_logs;

    let empty = ComposedSurface::new(0, 0);
    let mut out = ComposedSurface::new(4, 4);
    let logged = capture_logs(|| {
        resample(&empty, ScaleRatio::new(2, 1).unwrap(), &mut out);
    });
    assert_eq!((out.width(), out.height()), (0, 0));
    assert!(out.bytes().is_empty());
    assert!(
        logged.contains("level=WARN"),
        "外形ゼロは warn 発火: {logged}"
    );
    assert!(
        logged.contains("target=areka_emo_compose"),
        "target: {logged}"
    );

    // 片軸のみゼロ（高さ 0）も同様に非パニック・空出力。
    let flat = ComposedSurface::new(3, 0);
    let mut out2 = ComposedSurface::new(1, 1);
    resample(&flat, ScaleRatio::new(5, 4).unwrap(), &mut out2);
    assert_eq!((out2.width(), out2.height()), (4, 0));
    assert!(out2.bytes().is_empty());

    // 正常経路は無音（非空虚性）。
    let src = surface_of(2, 2, &[gray(1), gray(2), gray(3), gray(4)]);
    let quiet = capture_logs(|| {
        let mut o = ComposedSurface::new(0, 0);
        resample(&src, ScaleRatio::new(5, 4).unwrap(), &mut o);
        resample(&src, ScaleRatio::ONE, &mut o);
    });
    assert!(quiet.is_empty(), "正常な k× 転写は無音: {quiet}");
}

/// 出力バッファ再利用: 直前の内容・外形が残らず、毎回 `scaled_extent` どおりに上書きされる。
#[test]
fn resample_reuses_output_buffer_without_residue() {
    let src = surface_of(2, 2, &[gray(0), gray(80), gray(160), gray(240)]);
    let k2 = ScaleRatio::new(2, 1).unwrap();
    let mut out = ComposedSurface::new(0, 0);

    resample(&src, k2, &mut out);
    let big = out.bytes().to_vec();

    // 大→小→大: 縮小後に伸長しても残像が混ざらない。
    resample(&src, ScaleRatio::new(1, 2).unwrap(), &mut out);
    assert_eq!((out.width(), out.height()), (1, 1));
    resample(&src, k2, &mut out);
    assert_eq!(out.bytes(), big.as_slice(), "再利用バッファでも同一結果");

    // 恒等も同じバッファへ正しく畳み込まれる。
    resample(&src, ScaleRatio::ONE, &mut out);
    assert_eq!(out.bytes(), src.bytes());
}

// ------------------------------------------------------------------
// task 6.1: 純関数の全網羅（既存の檻が届いていなかった領域を塞ぐ）
// ------------------------------------------------------------------

/// 設計 D5（premultiplied ドメイン）: **α 可変**入力の厳密 golden（縮小 1/2）。
///
/// 既存の厳密値 golden は全て α=255 で、α 可変ケースは不変条件（B,G,R ≤ A）しか
/// 見ていなかった。α=255 では非乗算化が恒等写像ゆえ、**「非乗算化 → 補間 → 再乗算」
/// する実装でも既存テストは全緑になる**。本テストはその変異を殺す。
///
/// # 殺す変異（変異注入の実測に基づく。「排他的」＝α 可変 golden 2 本のみ失敗・既存は全緑）
///
/// - **排他的**: `resample` の内側ループを「straight α ドメインで補間（B,G,R を α で割り、
///   補間後に再び α で乗ずる）」へ差し替える変異。
///   例: 出力(0,0) は原画素 4 点 `[255,255,255,255] / [0,0,0,0] / [0,0,0,0] / [0,0,0,0]`
///   の premultiplied 平均 `255/4 = 63.75 → 64` になるが、非乗算化経路では
///   straight 色平均 `63.75→64` × α 平均 `64` / 255 ＝ **16** へ落ちる。
/// - **排他的**: 4 チャンネルのうち α だけ別式にする変異（例 α を 4 近傍の max にする）。
///   既存 golden は α=255 一色ゆえ α 列の差を見分けられない。
/// - **既存と共倒れ**: 最終丸め（round half up）を切り捨てへ変える変異——48/25/14/77/98/65
///   の各値が動くが、既存 `resample_clamps_at_edges`／
///   `resample_downscale_matches_oracle_and_golden`／
///   `resample_five_quarters_is_deterministic_golden` も同時に死ぬ（実測 5 失敗）。
/// - **既存と共倒れ**: BGRA チャンネル順の取り違え（実測 4 失敗＝α 可変 golden 2 本＋
///   既存 `resample_clamps_at_edges`／`resample_five_quarters_is_deterministic_golden`）。
///
/// # 殺せない変異（担当は別テスト）
///
/// - 重み量子化の分解能を落とす変異（`WEIGHT_SHIFT` 16bit→8bit 等）。k=1/2 の重みは
///   厳密に 1/4 ずつで粗い量子化でも割り切れるため本テストは緑のまま。その檻は重みが
///   割り切れない k を使う既存 `resample_five_quarters_is_deterministic_golden` が持つ。
#[test]
fn resample_alpha_varying_downscale_is_premultiplied_golden() {
    // 各画素は premultiplied 不変条件 B,G,R ≤ A を満たす（α は 0〜255 に散らす）。
    #[rustfmt::skip]
    let px: [[u8; 4]; 16] = [
        [255, 255, 255, 255], [  0,   0,   0,   0], [128,  64,  32, 200], [ 10,  10,  10,  10],
        [  0,   0,   0,   0], [  0,   0,   0,   0], [ 50,  25,  12,  60], [  4,   2,   1,   8],
        [200, 150, 100, 255], [  8,   4,   2,  16], [  0,   0,   0,   0], [255,   0,   0, 255],
        [100,  80,  60, 120], [  0,   0,   0,   0], [  1,   1,   1,   1], [  3,   2,   1,   4],
    ];
    let src = surface_of(4, 4, &px);
    let k = ScaleRatio::new(1, 2).unwrap();

    let mut out = ComposedSurface::new(0, 0);
    resample(&src, k, &mut out);
    assert_eq!((out.width(), out.height()), (2, 2));
    // 独立導出（f64 オラクル＝premultiplied 各チャンネル独立 bilinear）との一致。
    assert_matches_oracle(&src, k, &out);

    // k=1/2 の重みは厳密に 1/4 ずつ（2×2 ブロックの round half up 平均）。
    // 各値は「和 S に対し floor((S+2)/4)」で手計算した真値:
    //   (0,0) B,G,R,A: S=255 → 64
    //   (1,0) B:192→48 G:101→25 R:55→14 A:278→70
    //   (0,1) B:308→77 G:234→59 R:162→41 A:391→98
    //   (1,1) B:259→65 G:3→1   R:2→1   A:260→65
    #[rustfmt::skip]
    let expect: [u8; 16] = [
         64,  64,  64,  64,   48,  25,  14,  70,
         77,  59,  41,  98,   65,   1,   1,  65,
    ];
    assert_eq!(
        out.bytes(),
        &expect[..],
        "α 可変の premultiplied 厳密 golden"
    );

    // 非乗算化変異の急所を単独でも固定する（straight 経路なら 16 前後へ落ちる画素）。
    assert_eq!(
        &out.bytes()[..4],
        &[64, 64, 64, 64],
        "premultiplied 平均 255/4=63.75→64（非乗算化経路なら 16 になる）"
    );
}

/// 設計 D5（premultiplied ドメイン）: **α 可変**入力の厳密 golden（拡大 2/1）。
///
/// 縮小 golden と同じ変異を、重みが 1/4・3/4 になる拡大経路でも殺す。4 チャンネルを
/// 別値にしてあるため BGRA 順の取り違えも検出する。
///
/// # 殺す変異（変異注入の実測に基づく。「排他的」＝α 可変 golden 2 本のみ失敗・既存は全緑）
///
/// - **排他的**: 非乗算化 → 再乗算する実装。例: 出力(1,0) は `0.75·p00 + 0.25·p01`
///   （p01 は α=0 の全透明）＝ `(191,150,75,191)` だが、straight 経路では
///   色 191.25 × α 191.25 / 255 ＝ **143** へ落ちる。
/// - **排他的**: α だけ別式にする変異（縮小版 golden と同じ檻）。
/// - **既存と共倒れ**: エッジクランプの改変（四隅が原画素と一致しなくなる）。実測 4 失敗＝
///   本テスト＋既存 `resample_clamps_at_edges`／`resample_two_times_matches_golden`／
///   `resample_five_quarters_is_deterministic_golden`（縮小版 golden はこの変異では死なない）。
/// - **既存と共倒れ**: round half up の改変（124.5→125・57.5→58・73.5→74・72.5→73 が動く）。
///   実測 5 失敗＝α 可変 golden 2 本＋既存 3 本（縮小版 golden の doc に列挙）。
/// - **既存と共倒れ**: BGRA チャンネル順の取り違え。4 チャンネルを別値にしてあるため本テストは
///   確実に落ちるが、既存 `resample_clamps_at_edges`／
///   `resample_five_quarters_is_deterministic_golden` も同時に落ちる。
///
/// # 殺せない変異（担当は別テスト）
///
/// - 重み量子化の分解能を落とす変異。k=2/1 の重みは厳密に 1/4・3/4（`AxisWalk` の 16bit
///   量子化が割り切れる）ゆえ粗い量子化でも値が動かない。縮小版 golden と同じ限界である。
#[test]
fn resample_alpha_varying_upscale_is_premultiplied_golden() {
    let src = surface_of(
        2,
        2,
        &[
            [255, 200, 100, 255], // 不透明
            [0, 0, 0, 0],         // 全透明
            [64, 32, 16, 64],     // 半透明
            [128, 96, 0, 200],    // 半透明・R=0
        ],
    );
    let k = ScaleRatio::new(2, 1).unwrap();

    let mut out = ComposedSurface::new(0, 0);
    resample(&src, k, &mut out);
    assert_eq!((out.width(), out.height()), (4, 4));
    assert_matches_oracle(&src, k, &out);

    // 重みは厳密に 1/4・3/4（AxisWalk の 16bit 量子化が割り切れる）。
    // 手計算の真値（round half up）。
    #[rustfmt::skip]
    let expect: [u8; 64] = [
        255, 200, 100, 255,   191, 150,  75, 191,    64,  50,  25,  64,     0,   0,   0,   0,
        207, 158,  79, 207,   163, 125,  59, 168,    76,  58,  20,  89,    32,  24,   0,  50,
        112,  74,  37, 112,   108,  74,  28, 121,   100,  73,   9, 140,    96,  72,   0, 150,
         64,  32,  16,  64,    80,  48,  12,  98,   112,  80,   4, 166,   128,  96,   0, 200,
    ];
    assert_eq!(
        out.bytes(),
        &expect[..],
        "α 可変拡大の premultiplied 厳密 golden"
    );

    // 四隅は原画素そのもの（エッジクランプ・外挿なし）。
    assert_eq!(&out.bytes()[..4], &[255, 200, 100, 255]);
    assert_eq!(&out.bytes()[60..], &[128, 96, 0, 200]);

    // 非乗算化変異の急所（全透明画素と混ざる位置）。
    assert_eq!(
        &out.bytes()[4..8],
        &[191, 150, 75, 191],
        "0.75·不透明 + 0.25·全透明（非乗算化経路なら B=143 になる）"
    );

    // premultiplied 不変条件は当然保たれる（厳密 golden の副次確認）。
    for (i, p) in out.bytes().chunks_exact(4).enumerate() {
        assert!(
            p[0] <= p[3] && p[1] <= p[3] && p[2] <= p[3],
            "画素{i} が premultiplied を破っている: {p:?}"
        );
    }
}

// ------------------------------------------------------------------
// task 4.1: 作業領域受け取り形（`resample_with` ＋ `ResampleScratch`）
// ------------------------------------------------------------------

/// 決定的なテスト画像（線形合同法・premultiplied 不変条件 B,G,R ≤ A を満たす）。
///
/// 実機相当の外形（emo2 native 382×547）を扱うため、画素表を書き下す [`surface_of`] では
/// なく生成器を使う。同一 `(w, h)` は常に同一バイト列を返す（実時間・乱数源に依存しない・
/// 要件 6.2/6.3）。
fn deterministic_surface(w: u32, h: u32) -> ComposedSurface {
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

/// 要件 3.1／6.4: `resample_with` は `resample` と同一入力でバイト等価（外形・全バイト）。
///
/// 恒等（k=1/1）・拡大（k=2/1＝実機 emo2 の 192DPI 条件）・分数比（k=5/4）・縮小の各経路を
/// 通す。作業領域は**新品**と**使い回し**の 2 通りで検証し、前ケースの残留が結果へ漏れない
/// ことも同時に固定する（合否条件は外形とバイトのみ・実時間を一切見ない・要件 6.2）。
#[test]
fn resample_with_is_byte_equivalent_to_resample() {
    // (src_w, src_h, num, den)
    let cases: [(u32, u32, u32, u32); 7] = [
        (382, 547, 1, 1), // 恒等・実機 emo2 の native 外形
        (382, 547, 2, 1), // 実機 emo2 の 192DPI（764×1094）
        (37, 23, 5, 4),   // 分数比（重みが割り切れない）
        (16, 9, 2, 3),    // 縮小
        (5, 4, 1, 2),     // 縮小（整数倍）
        (1, 1, 7, 3),     // 1 画素からの拡大（エッジクランプのみ）
        (3, 11, 3, 7),    // 縦長の縮小
    ];
    // 使い回し用の作業領域（ケースをまたいで持ち回す）。
    let mut shared = ResampleScratch::default();

    for (w, h, num, den) in cases {
        let src = deterministic_surface(w, h);
        let k = ScaleRatio::new(num, den).unwrap();

        let mut want = ComposedSurface::new(0, 0);
        resample(&src, k, &mut want);

        // 新品の作業領域。
        let mut fresh_scratch = ResampleScratch::default();
        let mut got_fresh = ComposedSurface::new(0, 0);
        resample_with(&src, k, &mut got_fresh, &mut fresh_scratch);
        assert_eq!(
            (got_fresh.width(), got_fresh.height()),
            (want.width(), want.height()),
            "{w}x{h} k={num}/{den}: 新品作業領域の外形が resample と一致"
        );
        assert_eq!(
            got_fresh.bytes(),
            want.bytes(),
            "{w}x{h} k={num}/{den}: 新品作業領域でバイト等価"
        );

        // 使い回しの作業領域（直前ケースの写像表が残っている）。
        let mut got_shared = ComposedSurface::new(0, 0);
        resample_with(&src, k, &mut got_shared, &mut shared);
        assert_eq!(
            (got_shared.width(), got_shared.height()),
            (want.width(), want.height()),
            "{w}x{h} k={num}/{den}: 使い回し作業領域の外形が resample と一致"
        );
        assert_eq!(
            got_shared.bytes(),
            want.bytes(),
            "{w}x{h} k={num}/{den}: 使い回し作業領域でもバイト等価（残留が漏れない）"
        );
    }
}

/// 要件 3.1（作業領域の再利用）: `ResampleScratch` の容量は出力幅へ到達後は成長しない。
///
/// 反復呼び出しで容量が一定であること・より小さい出力幅の呼び出しで縮まない（＝元の幅へ
/// 戻したときに再確保しない）こと・写像表の長さが毎回ちょうど出力幅であること（残留エントリ
/// なし）を固定する。判定量はいずれも容量・長さ・バイトのみで、実時間を用いない（要件 6.2）。
#[test]
fn resample_with_scratch_capacity_does_not_grow_on_reuse() {
    let src = deterministic_surface(64, 40);
    let k = ScaleRatio::new(2, 1).unwrap();
    let mut out = ComposedSurface::new(0, 0);
    let mut scratch = ResampleScratch::default();

    // 初回で作業領域が出力幅へ到達する。
    resample_with(&src, k, &mut out, &mut scratch);
    let cap = scratch.x_map.capacity();
    let out_w = out.width() as usize;
    assert_eq!(out_w, 128, "k=2/1 の出力幅");
    assert_eq!(scratch.x_map.len(), out_w, "写像表は出力幅ちょうど");
    assert!(cap >= out_w, "初回で出力幅ぶんの容量へ到達している: {cap}");

    // 同一寸法の反復適用で容量は 1 バイトも増えない（毎回新規確保していれば崩れる）。
    for i in 0..8 {
        resample_with(&src, k, &mut out, &mut scratch);
        assert_eq!(
            scratch.x_map.capacity(),
            cap,
            "反復 {i}: 到達後の容量が成長した"
        );
        assert_eq!(scratch.x_map.len(), out_w, "反復 {i}: 写像表の長さは出力幅");
    }

    // より小さい出力幅でも容量は縮まない（縮小 → 再伸長の往復確保を作らない）。
    let small = deterministic_surface(9, 7);
    resample_with(&small, k, &mut out, &mut scratch);
    let small_w = out.width() as usize;
    assert_eq!(small_w, 18, "小さい方の出力幅");
    assert_eq!(scratch.x_map.capacity(), cap, "小さい幅で容量が縮んだ");
    assert_eq!(scratch.x_map.len(), small_w, "小さい幅でも残留エントリなし");

    // 小さい幅の結果は新品の作業領域と 1 バイトも違わない（残留内容が漏れない）。
    let mut want_small = ComposedSurface::new(0, 0);
    resample(&small, k, &mut want_small);
    assert_eq!(
        out.bytes(),
        want_small.bytes(),
        "使い回し作業領域の残留が小さい幅の結果へ漏れた"
    );

    // 元の幅へ戻しても再確保が起きない。
    resample_with(&src, k, &mut out, &mut scratch);
    assert_eq!(
        scratch.x_map.capacity(),
        cap,
        "元の幅へ戻した際に再確保が起きた"
    );
    assert_eq!(scratch.x_map.len(), out_w);
}

/// 恒等 k（1/1）は作業領域を一切使わない（写像表は空のまま・バイト恒等コピー・要件 7.2）。
#[test]
fn resample_with_identity_leaves_scratch_untouched() {
    let src = deterministic_surface(11, 6);
    let mut out = ComposedSurface::new(0, 0);
    let mut scratch = ResampleScratch::default();

    resample_with(&src, ScaleRatio::ONE, &mut out, &mut scratch);
    assert_eq!(out.bytes(), src.bytes(), "恒等はバイト恒等コピー");
    assert_eq!(scratch.x_map.len(), 0, "恒等経路は写像表を組まない");
    assert_eq!(scratch.x_map.capacity(), 0, "恒等経路は確保しない");
}
