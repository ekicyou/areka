//! 是正前のリサンプル意味論に対するバイト等価の檻（task 6.2・要件 3.3／6.2／6.3／6.4・
//! design.md 追補 §A2／§A3 の「必須の固定」）。
//!
//! # 何のための檻か
//!
//! これから入る 2 つの是正——**5.4**（リサンプル出力の冗長なゼロ埋め除去・追補 §A2）と
//! **5.5**（リサンプル計算そのものの是正・追補 §A3）——は、いずれも**表示バイトを 1 バイトも
//! 変えてはならない**（要件 3.3／6.4）。本ファイルはその禁止を機械的に強制する安全網である。
//!
//! # 「是正前の経路をテスト向けに残す」形として何を選んだか
//!
//! 選んだのは**凍結した参照実装**（[`prior_path_resample`]）である。golden バイトの焼き込みでも、
//! 本番同士（`resample` 対 `resample_with`）の突き合わせでもない。理由は 1 つで、
//! **5.5 は `resample_with` の内側ループを書き換える**からである:
//!
//! - 本番同士の比較は無力になる。`resample` は `resample_with` へ委譲する薄い形ゆえ、
//!   内側ループを書き換えれば**両辺が同時に同じだけ変わって緑のまま**通る。
//! - 焼き込み golden は変化を捕まえるが「なぜ違うのか」を何も言わず、入力 fixture を
//!   足すたびに値を貼り直す必要がある。
//! - 参照実装は本番と**別の場所に住み、5.5 の書き換え対象ではない**。5.5 が内側ループを
//!   どう作り替えても、参照実装は是正前の意味論を保ったまま残り、出力が 1 バイトでも
//!   ずれた瞬間に赤になる。
//!
//! 参照実装は本番コードの写しではない。本番が写像を**逐次前進**（`AxisWalk` の分子・剰余の
//! 積み上げ）で解くのに対し、参照は同じ規約 `src = (d + 1/2)·den/num − 1/2` を出力座標ごとの
//! **閉じた式**として直に解く。ゆえに前進の繰り上げに欠陥が入れば両者は食い違う——同じ
//! 構造を 2 度書いただけの「オラクル」にはならない。
//!
//! 参照が凍結しているのは次の 5 点である（是正前の意味論そのもの）。
//!
//! 1. 出力外形＝`round_half_away_from_zero(len × num / den)`・非ゼロ入力は最小 1
//! 2. 画素中心を合わせた有理逆写像と、隣接 2 点の**独立**エッジクランプ
//! 3. 混合重みの 16bit 固定小数点量子化（`(rem << 16) / (2·num)`・切り捨て）
//! 4. B・G・R・A の 4 チャンネルへ**同一式**（premultiplied ドメイン・α を特別扱いしない）
//! 5. 最終丸め `(v + 2^31) >> 32`（round half up）
//!
//! 参照は恒等（k=1/1）でも**一般式のまま**解く。本番は恒等をバイト恒等コピーへ短絡するので、
//! 両者の一致は「短絡が一般式と同じ絵を出す」ことの独立検証にもなっている。
//!
//! # 実時間を合否に使わない（要件 6.2）／純 x64 常設（要件 6.3）
//!
//! 本ファイルの判定量は外形とバイトだけである。環境変数ゲートも `#[ignore]` も持たない。
//!
//! # 外形ゼロを入れていない理由（4.1 からの引き継ぎ）
//!
//! 既存の `resample_zero_extent_is_empty_and_warns` は `warn!` の呼出位置の捕捉に依存しており、
//! subscriber 無しで先にその位置を踏むと捕捉判定が固定されて既存檻が偽陰性になる。ゆえに
//! 本ファイルは外形ゼロを一切踏まない（4.1 が同じ理由で避けた形を踏襲する）。

use super::test_support::*;
use super::*;

// ── 凍結参照実装（是正前の意味論・テスト専用オラクル）────────────────────────────────────

/// 重みの固定小数点分解能（是正前の意味論として**凍結**した値）。
///
/// 本番の `WEIGHT_SHIFT` を参照せず数値を直書きするのは、本番側の定数を書き換える是正が
/// 参照実装まで一緒に動いてしまう（＝檻が自分から目を閉じる）ことを防ぐためである。
const PRIOR_WEIGHT_ONE: u64 = 1 << 16;

/// 2 軸重み積の丸め半量（`1 << 31`）。同じ理由で直書きする。
const PRIOR_PRODUCT_HALF: u64 = 1 << 31;

/// 出力 1 座標ぶんの入力サンプル指定（参照実装の内部表現）。
struct PriorSample {
    /// 手前側の入力座標（クランプ済み）。
    i0: usize,
    /// 奥側の入力座標（クランプ済み）。
    i1: usize,
    /// 奥側の重み（`0..PRIOR_WEIGHT_ONE`）。
    w: u64,
}

/// 是正前の外形規約: `round(len × num / den)`（round half away from zero・非ゼロ入力は最小 1）。
fn prior_extent(len: u32, num: u32, den: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    let scaled = (2 * len as u128 * num as u128 + den as u128) / (2 * den as u128);
    u32::try_from(scaled.max(1)).unwrap_or(u32::MAX)
}

/// 是正前の写像規約を出力座標ごとに**閉じた式**で解く（本番の逐次前進とは別経路）。
///
/// 画素中心写像 `src = (d + 1/2)·den/num − 1/2` の分子は `(2d+1)·den − num`、分母は `2·num`
/// である。整数部を床方向（Euclid）で取り、剰余を 16bit へ切り捨て量子化する。
fn prior_axis(d: u32, num: u32, den: u32, len: u32) -> PriorSample {
    let denom = 2 * num as i128;
    let n = (2 * d as i128 + 1) * den as i128 - num as i128;
    let index = n.div_euclid(denom);
    let rem = n.rem_euclid(denom) as u128;
    let last = (len - 1) as i128;
    PriorSample {
        i0: index.clamp(0, last) as usize,
        i1: (index + 1).clamp(0, last) as usize,
        w: ((rem << 16) / denom as u128) as u64,
    }
}

/// 是正前のリサンプル（外形・密なバイト列を新しい `Vec` で返す）。
///
/// 出力先を毎回まっさらに起こすため、本関数は**使い回しバッファの残留とは無縁**である。
/// これが 5.4（ゼロ埋め除去）に対する対照側になる。
fn prior_path_resample(src: &ComposedSurface, scale: ScaleRatio) -> (u32, u32, Vec<u8>) {
    // `ScaleRatio` の分子・分母は私有だが、本モジュールは `scale` の子孫ゆえ読める。
    let (num, den) = (scale.num, scale.den);
    let out_w = prior_extent(src.width(), num, den);
    let out_h = prior_extent(src.height(), num, den);
    let mut out = vec![0_u8; out_w as usize * out_h as usize * 4];
    if src.width() == 0 || src.height() == 0 {
        return (out_w, out_h, out);
    }

    let src_stride = src.stride() as usize;
    let src_bytes = src.bytes();
    for dy in 0..out_h {
        let ys = prior_axis(dy, num, den, src.height());
        let inv_wy = PRIOR_WEIGHT_ONE - ys.w;
        for dx in 0..out_w {
            let xs = prior_axis(dx, num, den, src.width());
            let inv_wx = PRIOR_WEIGHT_ONE - xs.w;
            for c in 0..4_usize {
                let at = |iy: usize, ix: usize| src_bytes[iy * src_stride + ix * 4 + c] as u64;
                let top = at(ys.i0, xs.i0) * inv_wx + at(ys.i0, xs.i1) * xs.w;
                let bottom = at(ys.i1, xs.i0) * inv_wx + at(ys.i1, xs.i1) * xs.w;
                let v = top * inv_wy + bottom * ys.w;
                let o = (dy as usize * out_w as usize + dx as usize) * 4 + c;
                out[o] = ((v + PRIOR_PRODUCT_HALF) >> 32) as u8;
            }
        }
    }
    (out_w, out_h, out)
}

// ── 非空性ガード（空の入出力で等価主張を空虚に満たさせない）──────────────────────────────

/// 期待バイト列が**中身のある絵**であることを主張する。
///
/// 全ゼロ（＝完全透明）や単一値の一様面は「バイト等価」を無条件に満たしてしまう。ゆえに
/// ⑴非ゼロのバイトが在ること ⑵不透明画素（α=255）が在ること ⑶2 種類以上の値が在ること
/// を確かめる。1×1 入力の等倍のように**構造的に一様**な場合は ⑶ を免除する。
fn assert_not_empty(bytes: &[u8], out_w: u32, out_h: u32, at: &str) {
    assert!(!bytes.is_empty(), "{at}: 出力が空（等価主張が空虚）");
    assert!(
        bytes.iter().any(|&b| b != 0),
        "{at}: 出力が全ゼロ（完全透明を回して等価主張を空虚に満たしている）"
    );
    assert!(
        bytes.chunks_exact(4).any(|px| px[3] == 0xFF),
        "{at}: 不透明画素が 1 つも無い（α が全域で落ちている＝入力 fixture が壊れている）"
    );
    if out_w as usize * out_h as usize >= 4 {
        assert!(
            bytes.iter().any(|&b| b != bytes[0]),
            "{at}: 出力が一様（どの実装でも一致してしまう入力＝弁別力が無い）"
        );
    }
}

// ── 檻 1: 是正前の意味論とのバイト等価（5.5 の安全網）──────────────────────────────────

/// 判定に使う `(src 外形)` の一覧。端（1 画素）・縦横比の偏り・実機相当までを含む。
const SIZES: [(u32, u32); 12] = [
    (1, 1),
    (1, 7),
    (7, 1),
    (2, 2),
    (3, 5),
    (5, 3),
    (8, 6),
    (13, 9),
    (16, 9),
    (23, 37),
    (37, 23),
    (64, 40),
];

/// 判定に使う `k = num/den` の一覧。
///
/// 整数倍（重みが 0 か厳密 1/2 のみ＝丸めの引き分けが多発する）・端数（重みが割り切れない）・
/// 縮小・既約化前の形（`120/96`＝5/4）を混ぜる。
const RATIOS: [(u32, u32); 17] = [
    (1, 1),
    (2, 1),
    (3, 1),
    (4, 1),
    (5, 4),
    (4, 5),
    (6, 5),
    (12, 5),
    (1, 2),
    (2, 3),
    (3, 2),
    (3, 7),
    (7, 3),
    (7, 6),
    (9, 8),
    (120, 96),
    (192, 96),
];

/// 要件 3.3／6.4（**5.5 の安全網**）: `resample`／`resample_with` の出力が、凍結した是正前の
/// 参照実装と全ケースで 1 バイトも違わない。
///
/// α 可変（[`deterministic_surface`]）と全不透明（[`opaque_gradient_surface`]）の 2 系統の入力を
/// [`SIZES`]×[`RATIOS`] の全組で通す。α 可変だけだと「α を特別扱いする実装」は捕まえられるが
/// 非空性ガードの不透明画素が偶然消え得るため、両系統を必ず踏む。
///
/// # ここが赤になるべき変更（5.5 が踏み得る形）
///
/// 内側ループの範囲検査除去・行スライスの前取り・チャンネル展開・横補間の行間共有——追補 §A3 が
/// 挙げるどの手も、**丸め規約・写像・重み分解能を 1 つでも動かせば**ここで赤になる。逆に
/// バイトを変えない書き換えなら緑のまま通る（＝性能改善そのものは妨げない）。
#[test]
fn resample_matches_the_frozen_prior_path_byte_for_byte() {
    let mut shared = ResampleScratch::default();
    let mut checked = 0_usize;

    for (w, h) in SIZES {
        for (kind, src) in [
            ("alpha_varying", deterministic_surface(w, h)),
            ("opaque", opaque_gradient_surface(w, h, 0x5A)),
        ] {
            for (num, den) in RATIOS {
                let k = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");
                let at = format!("{kind} {w}x{h} k={num}/{den}");
                let (want_w, want_h, want) = prior_path_resample(&src, k);

                // 是正前の参照が「中身のある絵」を出していること（等価主張の非空虚性）。
                if kind == "opaque" {
                    assert_not_empty(&want, want_w, want_h, &at);
                }

                let mut got = ComposedSurface::new(0, 0);
                resample(&src, k, &mut got);
                assert_eq!(
                    (got.width(), got.height()),
                    (want_w, want_h),
                    "{at}: resample の外形が是正前と違う"
                );
                assert_eq!(
                    got.bytes(),
                    want.as_slice(),
                    "{at}: resample の出力が是正前と 1 バイト以上違う"
                );

                let mut got_with = ComposedSurface::new(0, 0);
                resample_with(&src, k, &mut got_with, &mut shared);
                assert_eq!(
                    (got_with.width(), got_with.height()),
                    (want_w, want_h),
                    "{at}: resample_with の外形が是正前と違う"
                );
                assert_eq!(
                    got_with.bytes(),
                    want.as_slice(),
                    "{at}: resample_with の出力が是正前と 1 バイト以上違う"
                );
                checked += 1;
            }
        }
    }

    assert_eq!(
        checked,
        SIZES.len() * 2 * RATIOS.len(),
        "全組を踏んでいない（ループが黙って縮んでいる）"
    );
}

/// 要件 3.3／6.4: **実機相当の外形**（emo2 native 382×547）でも是正前とバイト等価。
///
/// 小さい入力だけでは、行の刻み・行間の写像の繰り返し（k=2/1 では同じ入力行ペアが連続する
/// 出力行に現れる）といった、追補 §A3 の「横方向の中間結果の共有」が触る構造を実寸で踏めない。
/// 実機の 2 水準（192DPI＝k=2/1・120DPI＝k=5/4）と等倍を実寸で通す。
#[test]
fn resample_matches_the_frozen_prior_path_at_real_device_extents() {
    let src = opaque_gradient_surface(382, 547, 0x3C);
    let mut scratch = ResampleScratch::default();

    for (num, den) in [(1_u32, 1_u32), (2, 1), (5, 4)] {
        let k = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");
        let at = format!("emo2 native 382x547 k={num}/{den}");
        let (want_w, want_h, want) = prior_path_resample(&src, k);
        assert_not_empty(&want, want_w, want_h, &at);

        let mut got = ComposedSurface::new(0, 0);
        resample(&src, k, &mut got);
        assert_eq!(
            (got.width(), got.height()),
            (want_w, want_h),
            "{at}: 外形が是正前と違う"
        );
        assert_eq!(
            got.bytes(),
            want.as_slice(),
            "{at}: 出力が是正前と 1 バイト以上違う"
        );

        let mut got_with = ComposedSurface::new(0, 0);
        resample_with(&src, k, &mut got_with, &mut scratch);
        assert_eq!(
            got_with.bytes(),
            want.as_slice(),
            "{at}: resample_with の出力が是正前と 1 バイト以上違う"
        );
    }
}

// ── 檻 2: 使い回し出力バッファの残留（5.4 の安全網）──────────────────────────────────────

/// 使い回す `out` を用意する（指定外形で確保し、全バイトを `fill` で埋める）。
fn prefilled(w: u32, h: u32, fill: u8) -> ComposedSurface {
    let mut s = ComposedSurface::new(w, h);
    s.bytes_mut().fill(fill);
    s
}

/// 要件 3.3／6.4（**5.4 の安全網**・追補 §A2 が名指しする「残像混入の唯一の防波堤」）:
/// 直前の内容が残った `out` へ書いても、まっさらな `out` へ書いた結果と 1 バイトも違わない。
///
/// 追補 §A2 は `resize_and_clear`（毎回ゼロ埋め）を `resize_for_full_overwrite`（長さが同じなら
/// **何もしない**）へ差し替える。その前提は「`resample` が出力の全バイトを書き潰す」ことであり、
/// **前提が崩れれば前フレームの絵がそのまま画面へ出る**。ここはその前提を機械的に固定する。
///
/// # 踏む残留の形（4 通り）
///
/// 1. `0xAA` で塗った同外形の `out`（追補 §A2 が例示する形そのもの）
/// 2. **別の絵**を書いた後の同外形の `out`（実機の毎コマ経路＝直前のコマが残っている形）
/// 3. **バイト長が同じで外形だけ違う** `out`（`8×6` の後に `6×8`）——長さ一致を「そのまま使える」
///    と読む実装は、外形フィールドの更新まで飛ばして黙って壊れる
/// 4. 目標より大きい／小さい `out`（伸長・縮小の両方向）
///
/// 3 を入れてあるのは、長さだけを見る実装が**まっさらな経路では正しく動く**ためである
/// （新品は長さが違うので必ず作り直される）。使い回しの側でしか現れない欠陥はここでしか死なない。
///
/// # `0xAA` の塗り潰しが効く理由（presenter 層の檻では代替できない）
///
/// 「出力の一部の領域を毎回同じように書き落とす」変異は、**器を 0 から始めるかぎり検出できない**
/// ——書き落とされた領域は誰にも書かれないまま 0 であり続け、使い回しの側もまっさらな側も同じ
/// 0 になるからである。実測でも、5.4 を誤って実装した形（ゼロ埋めを外し、かつ最終行を書かない）を
/// 当てたとき `areka-emo-present` の等価檻 3 本は緑のままだった（本檻は赤・「0xAA の残留が
/// 出力へ混ざった」で鳴り、差分の末尾行が `170` で埋まっていることまで確認した）。
/// 器を 0 以外で塗れるのは `ComposedSurface::bytes_mut` が届く本クレート内だけである。
#[test]
fn resample_into_a_reused_output_is_byte_equal_to_a_fresh_output() {
    // (src_w, src_h, num, den)
    let cases: [(u32, u32, u32, u32); 6] = [
        (8, 6, 1, 1),  // 恒等（バイト恒等コピー経路）
        (8, 6, 2, 1),  // 整数倍の拡大
        (8, 6, 5, 4),  // 端数の拡大（実機 125%）
        (16, 9, 2, 3), // 縮小
        (37, 23, 7, 6),
        (1, 1, 7, 3), // 1 画素からの拡大（エッジクランプのみ）
    ];
    // 「別の絵」を書き込む側の入力（外形は同じ・バイトは別）。
    let mut shared = ResampleScratch::default();

    for (w, h, num, den) in cases {
        let src = opaque_gradient_surface(w, h, 0x11);
        let other = opaque_gradient_surface(w, h, 0x99);
        assert_ne!(
            src.bytes(),
            other.bytes(),
            "fixture 前提: 2 枚が同じ絵では残留の混入を弁別できない"
        );
        let k = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");
        let at = format!("{w}x{h} k={num}/{den}");

        // 対照（まっさらな出力先）。
        let mut fresh = ComposedSurface::new(0, 0);
        resample(&src, k, &mut fresh);
        let (out_w, out_h) = (fresh.width(), fresh.height());
        let want = fresh.bytes().to_vec();
        assert_not_empty(&want, out_w, out_h, &at);

        // ⑴ 0xAA 塗り潰しの同外形。
        let mut reused = prefilled(out_w, out_h, 0xAA);
        resample(&src, k, &mut reused);
        assert_eq!(
            (reused.width(), reused.height()),
            (out_w, out_h),
            "{at}: 0xAA 塗りの使い回しで外形が変わった"
        );
        assert_eq!(
            reused.bytes(),
            want.as_slice(),
            "{at}: 0xAA の残留が出力へ混ざった（追補 §A2 の前提「全バイトを書き潰す」が崩れている）"
        );

        // ⑵ 直前のコマ（別の絵）が残っている同外形。
        let mut carried = ComposedSurface::new(0, 0);
        resample(&other, k, &mut carried);
        let stale = carried.bytes().to_vec();
        assert_ne!(
            stale, want,
            "{at}: fixture 前提が崩れた（2 枚のリサンプル結果が同一では残留を弁別できない）"
        );
        resample(&src, k, &mut carried);
        assert_eq!(
            carried.bytes(),
            want.as_slice(),
            "{at}: 直前のコマの絵が残った（毎コマ経路で残像が出る形）"
        );

        // ⑶ 伸長・縮小の両方向（外形もバイト長も違う出力先からの使い回し）。
        for (pw, ph, kind) in [
            (out_w + 5, out_h + 3, "より大きい"),
            (
                out_w.div_ceil(2).max(1),
                out_h.div_ceil(2).max(1),
                "より小さい",
            ),
        ] {
            let mut resized = prefilled(pw, ph, 0xAA);
            resample(&src, k, &mut resized);
            assert_eq!(
                (resized.width(), resized.height()),
                (out_w, out_h),
                "{at}: {kind}出力先からの使い回しで外形が事後条件どおりでない"
            );
            assert_eq!(
                resized.bytes(),
                want.as_slice(),
                "{at}: {kind}出力先からの使い回しで残留が混ざった"
            );
        }

        // ⑷ 作業領域受け取り形でも同じ（本番の毎コマ経路が通るのはこちら）。
        let mut reused_with = prefilled(out_w, out_h, 0x5C);
        resample_with(&src, k, &mut reused_with, &mut shared);
        assert_eq!(
            reused_with.bytes(),
            want.as_slice(),
            "{at}: resample_with の使い回し出力先へ残留が混ざった"
        );
    }
}

/// 要件 3.3／6.4（追補 §A2 の ⑶）: **バイト長が同じで外形だけ違う**出力先の使い回しでも、
/// 外形とバイトが事後条件どおりになる。
///
/// `8×6` と `6×8` はどちらも 192 バイトである。「長さが一致していれば何もしない」を
/// **外形フィールドの更新まで飛ばす**と読んだ実装は、まっさらな出力先では長さが違うので
/// 必ず作り直され**正しく動く**——使い回しの側でしか壊れない。上の檻の ⑶ が伸縮を見ているのに対し、
/// ここは長さが動かない唯一の抜け道を名指しで塞ぐ。
#[test]
fn resample_into_a_reused_output_of_equal_length_but_different_extent_is_correct() {
    let k = ScaleRatio::ONE;
    let tall = opaque_gradient_surface(6, 8, 0x21);
    let wide = opaque_gradient_surface(8, 6, 0x21);

    let mut want_tall = ComposedSurface::new(0, 0);
    resample(&tall, k, &mut want_tall);
    let mut want_wide = ComposedSurface::new(0, 0);
    resample(&wide, k, &mut want_wide);
    assert_eq!(
        want_tall.bytes().len(),
        want_wide.bytes().len(),
        "前提: 6×8 と 8×6 のバイト長は等しい（この檻が狙う唯一の抜け道）"
    );
    assert_ne!(
        want_tall.bytes(),
        want_wide.bytes(),
        "前提: 2 つの出力が同一バイトでは弁別できない"
    );
    assert_not_empty(want_wide.bytes(), 8, 6, "8x6 k=1/1");

    // 同じ `out` を 6×8 → 8×6 → 6×8 と往復させる。
    let mut out = ComposedSurface::new(0, 0);
    resample(&tall, k, &mut out);
    resample(&wide, k, &mut out);
    assert_eq!(
        (out.width(), out.height()),
        (8, 6),
        "長さが同じ使い回しで外形が更新されていない"
    );
    assert_eq!(
        out.bytes(),
        want_wide.bytes(),
        "長さが同じ使い回しでバイトが事後条件どおりでない"
    );

    resample(&tall, k, &mut out);
    assert_eq!(
        (out.width(), out.height()),
        (6, 8),
        "戻す向きでも外形が更新されていない"
    );
    assert_eq!(out.bytes(), want_tall.bytes(), "戻す向きでバイトが違う");
}

/// fixture 自身の健全性: 2 系統の生成器が**互いに違う中身**を作り、非空性ガードが要求する
/// 性質（非ゼロ・不透明画素・非一様）を実際に満たす。
///
/// 檻の前提が壊れていたら等価主張は全て空虚になる。本 spec は fixture 側の破綻で 3 度
/// 偽の緑を踏んでいるため、生成器そのものを 1 本の檻で押さえる。
#[test]
fn the_fixture_generators_produce_distinguishable_non_empty_content() {
    let a = opaque_gradient_surface(16, 9, 0x11);
    let b = opaque_gradient_surface(16, 9, 0x99);
    assert_ne!(a.bytes(), b.bytes(), "salt 違いが同じ絵になっている");
    assert_not_empty(a.bytes(), 16, 9, "opaque_gradient_surface");
    assert!(
        a.bytes().chunks_exact(4).all(|px| px[3] == 0xFF),
        "opaque_gradient_surface は全画素 α=255 のはず"
    );

    let v = deterministic_surface(16, 9);
    assert_ne!(v.bytes(), a.bytes(), "2 系統の生成器が同じ絵を出している");
    assert!(
        v.bytes().iter().any(|&x| x != 0),
        "deterministic_surface が全ゼロ"
    );
    assert!(
        v.bytes().chunks_exact(4).any(|px| px[3] != 0xFF),
        "deterministic_surface の α が可変でない（α 特別扱いの変異を弁別できない）"
    );
    assert!(
        v.bytes()
            .chunks_exact(4)
            .all(|px| px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]),
        "deterministic_surface が premultiplied 不変条件（B,G,R ≤ A）を破っている"
    );
}

/// 参照実装そのものの較正: **是正前の意味論を写し取れていること**を、本番と独立に確かめられる
/// 3 つの性質で押さえる。
///
/// 道具が壊れていれば緑は何も意味しない（steering「検証の道具そのものが壊れる・較正せよ」）。
/// 本番との一致だけを見ていると、**両方が同じだけ間違っている**場合に気付けない——ここは
/// 本番を一切呼ばずに参照だけを検査する。
#[test]
fn the_frozen_reference_itself_satisfies_the_prior_contract() {
    // ⑴ 恒等は一般式のままでも入力のバイト恒等コピーになる（本番の短絡の正しさの根拠）。
    let src = deterministic_surface(9, 5);
    let (w, h, bytes) = prior_path_resample(&src, ScaleRatio::ONE);
    assert_eq!((w, h), (9, 5), "恒等の外形は素通し");
    assert_eq!(bytes, src.bytes(), "恒等の一般式がバイト恒等コピーでない");

    // ⑵ 外形は round half away from zero（切り捨てなら 6×5/4=7.5 → 7 になる）。
    let (w, h, _) = prior_path_resample(
        &opaque_gradient_surface(6, 5, 1),
        ScaleRatio::new(5, 4).unwrap(),
    );
    assert_eq!((w, h), (8, 6), "6×5/4=7.5→8・5×5/4=6.25→6 の丸め規約");

    // ⑶ premultiplied 不変条件（B,G,R ≤ A）が補間後も保たれる（凸結合＋単調丸め）。
    let (_, _, up) =
        prior_path_resample(&deterministic_surface(7, 6), ScaleRatio::new(3, 2).unwrap());
    assert!(
        up.chunks_exact(4)
            .all(|px| px[0] <= px[3] && px[1] <= px[3] && px[2] <= px[3]),
        "参照実装が premultiplied 不変条件を破った"
    );

    // ⑷ 端のクランプ: 1 画素入力を拡大すると全画素が原画素そのものになる。
    let one = opaque_gradient_surface(1, 1, 0x40);
    let (ow, oh, edge) = prior_path_resample(&one, ScaleRatio::new(4, 1).unwrap());
    assert_eq!((ow, oh), (4, 4), "1 画素 ×4 の外形");
    assert!(
        edge.chunks_exact(4).all(|px| px == one.bytes()),
        "エッジクランプが原画素を返していない"
    );
}
