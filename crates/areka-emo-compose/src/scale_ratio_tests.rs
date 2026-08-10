use super::*;
use super::test_support::*;
use std::collections::HashSet;

/// 要件 1.1: 構築は gcd で既約正準化する（120/96 → 5/4）。
#[test]
fn new_reduces_to_canonical_form() {
    let k = ScaleRatio::new(120, AUTHOR_DPI).unwrap();
    assert_eq!((k.num, k.den), (5, 4));

    for (dpi, expect) in [
        (96u32, (1u32, 1u32)),
        (120, (5, 4)),
        (144, (3, 2)),
        (168, (7, 4)),
        (192, (2, 1)),
    ] {
        let k = ScaleRatio::new(dpi, AUTHOR_DPI).unwrap();
        assert_eq!((k.num, k.den), expect, "dpi={dpi}");
    }
}

/// 要件 1.1: 0 は分子・分母のいずれでも構築失敗（パニックしない）。
#[test]
fn new_rejects_zero() {
    assert!(ScaleRatio::new(0, 96).is_none());
    assert!(ScaleRatio::new(96, 0).is_none());
    assert!(ScaleRatio::new(0, 0).is_none());
    assert!(ScaleRatio::new(1, 1).is_some());
}

/// 要件 1.3: `ONE` は正準の恒等であり、既定値でもある。
#[test]
fn one_is_canonical_identity() {
    assert_eq!((ScaleRatio::ONE.num, ScaleRatio::ONE.den), (1, 1));
    assert!(ScaleRatio::ONE.is_identity());
    assert_eq!(ScaleRatio::ONE.as_f32(), 1.0);
    assert_eq!(ScaleRatio::default(), ScaleRatio::ONE);
    assert_eq!(ScaleRatio::new(96, 96).unwrap(), ScaleRatio::ONE);
}

/// 要件 1.1: `Eq`/`Hash` は正準形で厳密（キャッシュキーの一意性）。
#[test]
fn eq_and_hash_are_strict_on_canonical_form() {
    let a = ScaleRatio::new(120, 96).unwrap();
    let b = ScaleRatio::new(5, 4).unwrap();
    let c = ScaleRatio::new(4, 5).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);

    let mut set = HashSet::new();
    set.insert(a);
    set.insert(b);
    assert_eq!(set.len(), 1, "同値は同一ハッシュキーへ畳まれる");
    set.insert(c);
    assert_eq!(set.len(), 2, "逆比は別キー");
}

/// 要件 1.3/7.2: 恒等判定は 1/1 のときに限り真。
#[test]
fn is_identity_holds_only_for_one() {
    assert!(ScaleRatio::new(96, 96).unwrap().is_identity());
    assert!(ScaleRatio::new(7, 7).unwrap().is_identity());
    assert!(!ScaleRatio::new(120, 96).unwrap().is_identity());
    assert!(!ScaleRatio::new(96, 120).unwrap().is_identity());
    assert!(!ScaleRatio::new(192, 96).unwrap().is_identity());
}

/// 要件 1.6: 乗算合成（アプリ管理拡大率 × DPI 由来 k）は約分済みの積を返す。
#[test]
fn mul_composes_and_reduces() {
    let k = ScaleRatio::new(120, 96).unwrap(); // 5/4
    // アプリ管理拡大率 1.0 固定シーム: ONE との積は恒等元。
    assert_eq!(ScaleRatio::ONE.mul(k), k);
    assert_eq!(k.mul(ScaleRatio::ONE), k);
    assert_eq!(ScaleRatio::ONE.mul(ScaleRatio::ONE), ScaleRatio::ONE);

    // アプリ 2.0 × DPI 5/4 = 5/2（最終拡大率 2.5）。
    let app = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(app.mul(k), ScaleRatio::new(5, 2).unwrap());
    assert_eq!(k.mul(app), ScaleRatio::new(5, 2).unwrap(), "乗算は可換");

    // 逆数同士は恒等へ約分される。
    let a = ScaleRatio::new(1_000_000, 3).unwrap();
    let b = ScaleRatio::new(3, 1_000_000).unwrap();
    assert_eq!(a.mul(b), ScaleRatio::ONE);
}

/// 要件 1.6: 積は u64 中間で計算され、u32 域の桁溢れでラップしない。
#[test]
fn mul_uses_wide_intermediate_without_wrapping() {
    // u32 積が溢れる大きさでも約分で恒等へ戻る（中間が u32 なら破綻する）。
    let a = ScaleRatio::new(4_000_000_000, 1).unwrap();
    let b = ScaleRatio::new(1, 4_000_000_000).unwrap();
    assert_eq!(a.mul(b), ScaleRatio::ONE);
    // 縮退が起きない大きさでは近似も警告も起こらない（陰性確認）。
    assert_eq!(
        ScaleRatio::new(65_535, 1)
            .unwrap()
            .mul(ScaleRatio::new(65_535, 1).unwrap()),
        ScaleRatio::new(4_294_836_225, 1).unwrap()
    );
}

/// 要件 1.6: 約分後も u32 域へ収まらない病的比は、大きい側を `u32::MAX` へ張り付ける
/// 線形縮小で近似縮退する（比の保存ではないが誤差は 1 量子化ステップ以内）。
#[test]
fn mul_degrades_proportionally_when_product_exceeds_u32() {
    // 65_537² = 4_295_098_369（u32::MAX = 4_294_967_295 を 131_074 だけ超える）。
    // 大きい側 num を u32::MAX へ張り付け、den は 1*u32::MAX/4_295_098_369 = 0 → 最小 1。
    let big = ScaleRatio::new(65_537, 1).unwrap();
    let sq = big.mul(big);
    assert_eq!(
        (sq.num, sq.den),
        (4_294_967_295, 1),
        "誤差 0.0031%（真値 4_295_098_369）"
    );
    assert_eq!(sq, big.mul(big), "縮退も決定論的");
    assert_eq!(sq.scale_len(1), 4_294_967_295);

    // 分母側が超過する対称ケース（den を u32::MAX へ張り付け・num は最小 1）。
    let tiny = ScaleRatio::new(1, 65_537).unwrap();
    let sq_inv = tiny.mul(tiny);
    assert_eq!((sq_inv.num, sq_inv.den), (1, 4_294_967_295));
    assert_eq!(sq_inv, tiny.mul(tiny), "縮退も決定論的");

    // 縮退後も分子・分母は常に非ゼロ（as_f32 が 0 / NaN / inf にならない）。
    assert!(sq.as_f32() > 0.0 && sq.as_f32().is_finite());
    assert!(sq_inv.as_f32() > 0.0 && sq_inv.as_f32().is_finite());
}

/// 要件 1.4（ログ規律・フォールバック発生＝`warn!`）: 近似縮退は縮退前後の値つきで
/// `warn!` を発する（ログ無し失敗経路の禁止）。縮退しない通常経路は無音（非空虚性）。
#[test]
fn mul_degradation_emits_warn_log() {
    use crate::log_capture::capture_logs;

    let big = ScaleRatio::new(65_537, 1).unwrap();
    let out = capture_logs(|| {
        let _ = big.mul(big);
    });
    assert!(out.contains("level=WARN"), "縮退は warn 発火: {out}");
    assert!(out.contains("target=areka_emo_compose"), "target: {out}");
    assert!(out.contains("orig_num=4295098369"), "縮退前分子: {out}");
    assert!(out.contains("orig_den=1"), "縮退前分母: {out}");
    assert!(out.contains("num=4294967295"), "縮退後分子: {out}");
    assert!(out.contains("収まらず近似縮退"), "縮退の説明: {out}");

    // 通常経路（u32 域に収まる積）はログを一切出さない。
    let quiet = capture_logs(|| {
        let k = ScaleRatio::new(120, AUTHOR_DPI).unwrap();
        let _ = k.mul(ScaleRatio::new(2, 1).unwrap());
        let _ = ScaleRatio::ONE.mul(ScaleRatio::ONE);
    });
    assert!(quiet.is_empty(), "縮退しない積は無音: {quiet}");
}

/// 要件 1.2/1.6: `as_f32` は代表 DPI で厳密値を返す（照会契約の出口ビュー）。
#[test]
fn as_f32_yields_exact_dpi_values() {
    for (dpi, expect) in [
        (96u32, 1.0f32),
        (120, 1.25),
        (144, 1.5),
        (168, 1.75),
        (192, 2.0),
    ] {
        let k = ScaleRatio::new(dpi, AUTHOR_DPI).unwrap();
        assert_eq!(k.as_f32(), expect, "dpi={dpi}");
    }
}

/// 要件 1.1/1.3/2.2/2.5: DPI 対照表 × 代表原寸が決定論的に一致する。
///
/// 96（k=1/1・等倍）／120（5/4）／144（3/2）／168（7/4）／192（2/1）の 5 水準で、
/// 代表原寸の k 倍寸が期待表と厳密一致すること・96 と 192 が同一寸にならないこと
/// （k=1.0 固定の途中状態を残さない・要件 2.2）を固定する。
#[test]
fn dpi_table_scaled_extent_is_deterministic() {
    const NATIVE: [u32; 10] = [1, 2, 3, 48, 100, 127, 200, 255, 300, 401];
    // (窓 DPI, 各 NATIVE に対する期待 k 倍寸)
    const TABLE: [(u32, [u32; 10]); 5] = [
        (96, [1, 2, 3, 48, 100, 127, 200, 255, 300, 401]),
        (120, [1, 3, 4, 60, 125, 159, 250, 319, 375, 501]),
        (144, [2, 3, 5, 72, 150, 191, 300, 383, 450, 602]),
        (168, [2, 4, 5, 84, 175, 222, 350, 446, 525, 702]),
        (192, [2, 4, 6, 96, 200, 254, 400, 510, 600, 802]),
    ];

    for (dpi, expect) in TABLE {
        let k = ScaleRatio::new(dpi, AUTHOR_DPI).unwrap();
        for (i, &len) in NATIVE.iter().enumerate() {
            assert_eq!(k.scale_len(len), expect[i], "dpi={dpi} len={len}");
            // 同一入力の反復が同一出力（決定論）。
            assert_eq!(k.scale_len(len), k.scale_len(len));
            // 外形は各軸への scale_len 適用と厳密一致。
            assert_eq!(
                k.scaled_extent(len, len),
                (expect[i], expect[i]),
                "dpi={dpi} len={len}"
            );
        }
    }

    // 要件 2.2: 96 水準と 192 水準は同一物理寸にならない（k=1.0 固定の途中状態の排除）。
    let k96 = ScaleRatio::new(96, AUTHOR_DPI).unwrap();
    let k192 = ScaleRatio::new(192, AUTHOR_DPI).unwrap();
    assert_ne!(k96.scaled_extent(100, 200), k192.scaled_extent(100, 200));
    assert_eq!(k192.scaled_extent(100, 200), (200, 400));
}

/// 要件 2.5: 端数ちょうど 0.5 は 0 から遠い側（切り上げ）へ丸める。
#[test]
fn scale_len_rounds_half_away_from_zero() {
    let half = ScaleRatio::new(1, 2).unwrap();
    // 0.5 / 1.5 / 2.5 / 3.5 がすべて切り上がる。
    assert_eq!(half.scale_len(1), 1);
    assert_eq!(half.scale_len(3), 2);
    assert_eq!(half.scale_len(5), 3);
    assert_eq!(half.scale_len(7), 4);
    // 端数なしは素通し。
    assert_eq!(half.scale_len(2), 1);
    assert_eq!(half.scale_len(4), 2);

    let k54 = ScaleRatio::new(5, 4).unwrap();
    assert_eq!(k54.scale_len(2), 3); // 2.5
    assert_eq!(k54.scale_len(6), 8); // 7.5
    assert_eq!(k54.scale_len(10), 13); // 12.5

    let k32 = ScaleRatio::new(3, 2).unwrap();
    assert_eq!(k32.scale_len(1), 2); // 1.5
    assert_eq!(k32.scale_len(3), 5); // 4.5
    assert_eq!(k32.scale_len(5), 8); // 7.5

    let quarter = ScaleRatio::new(1, 4).unwrap();
    assert_eq!(quarter.scale_len(2), 1); // 0.5
    assert_eq!(quarter.scale_len(6), 2); // 1.5
    assert_eq!(quarter.scale_len(10), 3); // 2.5

    // 0.5 未満は切り捨て側（最小 1px 保証と区別される丸めそのものの挙動）。
    assert_eq!(ScaleRatio::new(1, 3).unwrap().scale_len(3), 1); // 1.0
    assert_eq!(ScaleRatio::new(2, 5).unwrap().scale_len(3), 1); // 1.2 → 1
    assert_eq!(ScaleRatio::new(3, 5).unwrap().scale_len(3), 2); // 1.8 → 2
}

/// 要件 2.5: 非ゼロ原寸は最小 1px（縮小で表示が消えない）。
#[test]
fn scale_len_clamps_nonzero_to_min_one_pixel() {
    let tiny = ScaleRatio::new(1, 100).unwrap();
    assert_eq!(tiny.scale_len(1), 1, "0.01 → 1（最小 1px）");
    assert_eq!(tiny.scale_len(49), 1, "0.49 → 1（最小 1px）");
    assert_eq!(tiny.scale_len(50), 1, "0.5 → 1（丸めが自然に 1）");
    assert_eq!(tiny.scale_len(200), 2);
    assert_eq!(ScaleRatio::new(1, 1000).unwrap().scale_len(1), 1);
    // 外形も両軸で最小 1px。
    assert_eq!(tiny.scaled_extent(1, 1), (1, 1));
}

/// 要件 2.5: 0 は 0 のまま（存在しない寸法を作らない）。
#[test]
fn scale_len_zero_stays_zero() {
    let k = ScaleRatio::new(192, AUTHOR_DPI).unwrap();
    assert_eq!(k.scale_len(0), 0);
    assert_eq!(k.scaled_extent(0, 0), (0, 0));
    assert_eq!(k.scaled_extent(0, 10), (0, 20), "軸ごとに独立して丸める");
    assert_eq!(ScaleRatio::ONE.scale_len(0), 0);
}

/// 要件 1.3/7.2: 恒等 k は入力を素通しする（既存等倍出力と等価）。
#[test]
fn identity_scale_is_passthrough() {
    for len in [0u32, 1, 2, 3, 127, 4096, u32::MAX] {
        assert_eq!(ScaleRatio::ONE.scale_len(len), len);
    }
    assert_eq!(ScaleRatio::ONE.scaled_extent(300, 401), (300, 401));
    assert_eq!(
        ScaleRatio::new(96, AUTHOR_DPI)
            .unwrap()
            .scaled_extent(300, 401),
        (300, 401)
    );
}

/// 要件 2.5: 大寸でも中間演算が溢れず、u32 超過は飽和（ラップしない）。
#[test]
fn scale_len_handles_large_extents_without_overflow() {
    let k54 = ScaleRatio::new(5, 4).unwrap();
    assert_eq!(k54.scale_len(2_000_000_000), 2_500_000_000);

    let k74 = ScaleRatio::new(7, 4).unwrap();
    assert_eq!(k74.scale_len(1_000_000_000), 1_750_000_000);

    // u32 を超える結果は飽和（ラップアラウンドなら 8_589_934_590 - 2^32 = 4_294_967_294）。
    let k2 = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(k2.scale_len(u32::MAX), u32::MAX);
    assert_eq!(k2.scaled_extent(u32::MAX, u32::MAX), (u32::MAX, u32::MAX));

    // 巨大な分子・分母の組でもパニックしない（中間幅の証明）。
    let extreme = ScaleRatio::new(u32::MAX, u32::MAX - 1).unwrap();
    assert_eq!(extreme.scale_len(u32::MAX), u32::MAX);
    let shrink = ScaleRatio::new(1, u32::MAX).unwrap();
    assert_eq!(shrink.scale_len(u32::MAX), 1);
}

/// 要件 2.5: 丸め規約 round half away from zero を、**丁度 .5 とその両隣**の対で固定する。
///
/// 片側だけの主張では「常に切り上げ」実装も緑になってしまう。`.5` 未満（切り捨て）・
/// `.5` 丁度（切り上げ）・`.5` 超（切り上げ）の 3 点を隣接入力で対にする。
///
/// # 殺す変異（変異注入の実測に基づく。**排他的キルは持たない**）
///
/// 丸め変異の一次防衛線は既存 `scale_len_rounds_half_away_from_zero`／
/// `dpi_table_scaled_extent_is_deterministic` が既に張っており、下の 3 変異はいずれも
/// それらと共倒れになる。本テストの役割は「.5 未満／.5 丁度／.5 超」を隣接入力の対として
/// 契約に明文化することであって、新しい変異を単独で捕まえることではない。
///
/// - 常に切り上げ（`div_ceil`）: `…499/1000`（0.499）が 1 つ上へずれる。実測 5 失敗＝
///   本テスト・`as_f32_is_query_view_not_dimension_authority`・
///   `scale_len_u128_intermediate_beats_u64_overflow`＋既存 2 本
///   （`scale_len_rounds_half_away_from_zero`／`dpi_table_scaled_extent_is_deterministic`）。
/// - 常に切り捨て（`len·num/den`）: `…500/1000`（0.5 丁度）が 1 つ下へずれる。実測 4 失敗＝
///   本テスト＋既存 3 本（上記 2 本と `resample_zero_extent_is_empty_and_warns`）。
/// - round half to **even**: `n` が偶数の `n+0.5` が `n` へ落ちる（下の両ループが検出）。
///   実測 3 失敗＝本テスト＋既存 2 本（`scale_len_rounds_half_away_from_zero` も
///   同じ変異で死ぬ）。
///
/// なお本テストの入力はすべて結果 ≥ 1 ゆえ、最小 1px クランプが丸めを覆い隠さない
/// （min1px と丸めの檻を分離する）。
#[test]
fn scale_len_half_tie_pairs_pin_round_half_away_from_zero() {
    // 1/1000: 隣接入力で 0.499 / 0.500 / 0.501 の 3 点を対にする（ε＝1/1000）。
    let milli = ScaleRatio::new(1, 1000).unwrap();
    for m in 1u32..=6 {
        let base = 1000 * m;
        assert_eq!(milli.scale_len(base + 499), m, "{m}.499 は切り捨て");
        assert_eq!(milli.scale_len(base + 500), m + 1, "{m}.5 丁度は切り上げ");
        assert_eq!(milli.scale_len(base + 501), m + 1, "{m}.501 は切り上げ");
    }

    // 1/2: 端数なし（n）と丁度 .5（n+0.5）を全 n で対にする。
    // n が偶数の n+0.5 は round half to even なら n へ落ちるため、その変異も死ぬ。
    let half = ScaleRatio::new(1, 2).unwrap();
    for n in 1u32..=12 {
        assert_eq!(half.scale_len(2 * n), n, "端数なし n={n}");
        assert_eq!(half.scale_len(2 * n + 1), n + 1, "n+0.5 は上へ n={n}");
    }

    // DPI 対照表の k でも同様（3/2 と 7/4 の丁度 .5 と直下）。
    let k32 = ScaleRatio::new(3, 2).unwrap();
    assert_eq!(k32.scale_len(5), 8, "7.5 丁度 → 8");
    assert_eq!(k32.scale_len(4), 6, "6.0 は素通し");
    let k74 = ScaleRatio::new(7, 4).unwrap();
    assert_eq!(k74.scale_len(2), 4, "3.5 丁度 → 4");
    assert_eq!(
        k74.scale_len(6),
        11,
        "10.5 丁度 → 11（half-to-even なら 10）"
    );
    assert_eq!(k74.scale_len(5), 9, "8.75 → 9");
}

/// 要件 1.6: 乗算合成は可換かつ結合的で、真値（未約分の積）と厳密一致する。
///
/// # 殺す変異（変異注入の実測に基づく。**排他的キルは持たない**）
///
/// - `mul` の gcd 約分を**完全に**（積直後と縮退後の 2 箇所とも）落とす変異は、
///   `assert_eq!(a.mul(b), ScaleRatio::new(an·bn, ad·bd).unwrap())` の同値主張が殺す。
///   `new` は正準化する一方、約分を失った `mul` は未約分のまま返すため、`5/4 × 2/1` が
///   `{num:10, den:4}` vs `{num:5, den:2}` で食い違う（`Eq` はフィールド比較ゆえ、
///   両辺が正準形であるときに限り「同じ有理数」を意味する）。実測 3 失敗＝本テスト＋既存
///   `mul_composes_and_reduces`／`mul_uses_wide_intermediate_without_wrapping`。
/// - **片側だけの gcd 削除は等価変異**（実測: 積直後のみ削除・縮退後のみ削除、どちらも全緑）。
///   縮退が起きない限り、どちらか一方が残っていれば結果は既約になるためである。片側削除を
///   殺すには「縮退経路で shrink が共通因子を作り直す」witness が要るが、本ファイルの
///   どのテストもそこへ到達していない。
/// - 積の分子・分母を取り違える変異も同じ同値主張が殺す（既存 `mul_composes_and_reduces`
///   と共倒れ）。
///
/// # 殺せない主張（契約の明文化であって檻ではない）
///
/// 可換律・結合律のアサート自体は変異検出力を持たない。約分の有無に関わらず `(a·b)·c` と
/// `a·(b·c)` はどちらも分子 `an·bn·cn`・分母 `ad·bd·cd` へ落ちるため、gcd を落としても
/// 両辺が同時に動いて等式は保たれる。さらに 3 重ループは**同一 `TABLE` を独立に走査する**
/// （`a=b=c` を許す）ため、既約後 `TABLE` の 7³=343 通りの上界は分子 2197（13³）・分母
/// 4913（17³）——`mul` の飽和縮退（u32 超過）には**到達しない**。同節が触れる
/// `ScaleRatio::new(an·bn, ad·bd)` は生値（96・120 を含む）を使うため別系統で上界
/// 9216/14400 だが、これも到達しない。
/// ゆえに「約分を落とすと中間値が u32 域を超えて縮退し `(a·b)·c != a·(b·c)` が破れる」という
/// 機構は本テストでは発火し得ない。
#[test]
fn mul_is_commutative_and_associative() {
    const TABLE: [(u32, u32); 7] =
        [(5, 4), (3, 2), (7, 4), (2, 1), (1, 3), (96, 120), (13, 17)];
    for &(an, ad) in TABLE.iter() {
        let a = ScaleRatio::new(an, ad).unwrap();
        for &(bn, bd) in TABLE.iter() {
            let b = ScaleRatio::new(bn, bd).unwrap();
            assert_eq!(a.mul(b), b.mul(a), "可換: {an}/{ad} × {bn}/{bd}");
            assert_eq!(
                a.mul(b),
                ScaleRatio::new(an * bn, ad * bd).unwrap(),
                "積は未約分の真値と同値: {an}/{ad} × {bn}/{bd}"
            );
            for &(cn, cd) in TABLE.iter() {
                let c = ScaleRatio::new(cn, cd).unwrap();
                assert_eq!(
                    a.mul(b).mul(c),
                    a.mul(b.mul(c)),
                    "結合律: {an}/{ad} × {bn}/{bd} × {cn}/{cd}"
                );
            }
        }
    }
}

/// 要件 1.6: u32 域を超える積は「大きい側を `u32::MAX` へ張り付ける」飽和縮退になる。
///
/// # 殺す変異（変異注入の実測に基づく。**排他的キルは持たない**）
///
/// 下の 3 変異はいずれも既存 `mul_degrades_proportionally_when_product_exceeds_u32`
/// （と、ログを見る `mul_degradation_emits_warn_log`）と共倒れになる。本テストの役割は
/// 縮退の契約——「大きい側は `u32::MAX` ちょうど・小さい側は最小 1・決定論」——を
/// 複数の base 族で明文化することである。
///
/// - 縮退を「素朴な半減（`div_ceil(2)`）」へ差し替える実装（設計レビューで REJECT
///   された案）。`den == 1` の族では真値の **50%** へ落ちるため、下の厳密値と
///   相対誤差上限の双方が破れる。実測 3 失敗＝本テスト＋既存 2 本。
/// - 大きい側の張り付け先を `u32::MAX` 以外（例 `u32::MAX/2`）にする変異
///   （`max(num, den) == u32::MAX` の主張が死ぬ）。実測 3 失敗＝本テスト＋既存 2 本。
/// - 小さい側の `max(1)` を落とす変異。縮小後 `den == 0` となり、`gcd(u32::MAX, 0)` で
///   割った正準化の結果が `(1, 0)` へ落ちるため、まず `(sq.num, sq.den) == (u32::MAX, 1)`
///   の厳密値主張が発火する（`as_f32` も inf になるが、そこへ到達する前に死ぬ）。
///   実測 2 失敗＝本テスト＋既存 `mul_degrades_proportionally_when_product_exceeds_u32`。
#[test]
fn mul_saturating_degradation_pins_largest_to_u32_max() {
    for base in [65_536u32, 65_537, 100_000, 1_000_000, u32::MAX] {
        let up = ScaleRatio::new(base, 1).unwrap();
        let sq = up.mul(up);
        assert_eq!(
            (sq.num, sq.den),
            (u32::MAX, 1),
            "base={base}: 大きい側は u32::MAX へ張り付き、小さい側は最小 1"
        );
        assert_eq!(sq.num.max(sq.den), u32::MAX, "base={base}");
        assert!(sq.as_f32().is_finite() && sq.as_f32() > 0.0, "base={base}");

        // 逆数側（分母が溢れる対称ケース）。
        let down = ScaleRatio::new(1, base).unwrap();
        let sq_inv = down.mul(down);
        assert_eq!((sq_inv.num, sq_inv.den), (1, u32::MAX), "base={base}");
        assert_eq!(sq_inv.num.max(sq_inv.den), u32::MAX, "base={base}");

        // 決定論（同一入力は同一縮退）。
        assert_eq!(sq, up.mul(up), "base={base}");
        assert_eq!(sq_inv, down.mul(down), "base={base}");
    }

    // 真値が u32 域の直上にある族では、縮退後の比が真値へ十分近いこと。
    // 素朴な半減なら相対誤差 0.5 になり、この上限を破る。
    for base in [65_536u64, 65_537] {
        let truth = (base * base) as f64;
        let k = ScaleRatio::new(base as u32, 1)
            .unwrap()
            .mul(ScaleRatio::new(base as u32, 1).unwrap());
        let got = k.num as f64 / k.den as f64;
        let rel = (got - truth).abs() / truth;
        assert!(
            rel < 1.0e-3,
            "base={base}: 縮退後 {got} が真値 {truth} から乖離（相対誤差 {rel}）"
        );
    }
}

/// 要件 1.2/2.5: `as_f32` は**照会契約の出口ビュー**であり寸法権威ではない。
///
/// doc 契約「寸法・画素演算にこの値を使ってはならない」を、f32 経路と
/// [`ScaleRatio::scale_len`] が実際に食い違う具体例で固定する。誰かが `scale_len` を
/// 「`as_f32` を使う実装」へ書き換えたら、下の `assert_ne!` と厳密値の双方が死ぬ。
///
/// # アサーションの性格（契約の檻と、性質の記録の別）
///
/// `assert_eq!(via_f32, 2_500_000_000)`（f32 経路の値）と
/// `assert_ne!(as_f32(1/3) as f64, 1.0/3.0)` は、**本番コードの契約ではなく IEEE754
/// binary32 の性質**を固定する主張である（24bit 仮数では `2_000_000_001` そのものが
/// 丸められる／`1/3` は 2 冪分母でないため f32 で厳密表現できない）。本番実装を
/// どう変えてもこの 2 行は動かない。ここに置く意図は「なぜ f32 を寸法権威にできないか」の
/// 根拠を実行可能な形で残すことで、実装契約の檻は同じテスト内の `scale_len` 側の厳密値
/// （`2_500_000_001`・最小 1px）が担う。
///
/// # 殺す変異（変異注入の実測に基づく）
///
/// - `scale_len` を `(len as f32 * self.as_f32()) as u32` へ差し替える（仮数欠落で
///   大寸が 1px ずれ、極小 k で 0 へ潰れる）。**既存と共倒れ**——実測 6 失敗＝本テスト・
///   `scale_len_half_tie_pairs_pin_round_half_away_from_zero`・
///   `scale_len_u128_intermediate_beats_u64_overflow`＋既存 3 本
///   （`scale_len_rounds_half_away_from_zero`／`dpi_table_scaled_extent_is_deterministic`／
///   `resample_zero_extent_is_empty_and_warns`）。
///   ただし丸めを保った穏当版 `(len as f32 * self.as_f32()).round() as u32` では既存が
///   全緑になり、本テストと `scale_len_u128_…` の**新 2 本だけが落ちる**——本テストの
///   固有価値はこの「f32 化が丸め規約を保っていても寸法権威にならない」域にある。
/// - `as_f32` を `num as f32 / den as f32` 以外（例: 先に整数除算）へ変える。
///   **既存と共倒れ**——実測 3 失敗＝本テスト＋既存 `as_f32_yields_exact_dpi_values`／
///   `mul_degrades_proportionally_when_product_exceeds_u32`。
/// - `scale_len` の最小 1px クランプを落とす。**既存と共倒れ**——実測 2 失敗＝本テスト＋
///   既存 `scale_len_clamps_nonzero_to_min_one_pixel`。
#[test]
fn as_f32_is_query_view_not_dimension_authority() {
    // 2 冪分母は f32 で厳密（照会値としての厳密性）。
    for (num, den, expect) in [
        (1u32, 2u32, 0.5f32),
        (1, 4, 0.25),
        (3, 8, 0.375),
        (7, 4, 1.75),
        (2, 1, 2.0),
        (9, 16, 0.5625),
    ] {
        assert_eq!(
            ScaleRatio::new(num, den).unwrap().as_f32(),
            expect,
            "{num}/{den}"
        );
    }
    // 非 2 冪は f32 で厳密表現できない（＝丸めの権威にできない）。
    assert_ne!(
        ScaleRatio::new(1, 3).unwrap().as_f32() as f64,
        1.0f64 / 3.0f64,
        "1/3 は f32 では厳密でない"
    );

    // 大寸: f32 の 24bit 仮数では原寸そのものが丸められ、結果が 1px ずれる。
    let k54 = ScaleRatio::new(5, 4).unwrap();
    assert_eq!(k54.as_f32(), 1.25);
    let len = 2_000_000_001u32;
    assert_eq!(k54.scale_len(len), 2_500_000_001, "整数権威の厳密値");
    let via_f32 = (len as f32 * k54.as_f32()) as u32;
    assert_eq!(via_f32, 2_500_000_000, "f32 経路は仮数欠落で 1px 少ない");
    assert_ne!(via_f32, k54.scale_len(len), "f32 は寸法権威になり得ない");

    // 極小: f32 の切り捨てキャストは表示を消すが、scale_len は最小 1px を守る。
    for (num, den) in [(1u32, 3u32), (2, 3), (1, 1000)] {
        let k = ScaleRatio::new(num, den).unwrap();
        assert_eq!(k.scale_len(1), 1, "{num}/{den}: 最小 1px");
        assert_eq!(
            (1.0f32 * k.as_f32()) as u32,
            0,
            "{num}/{den}: f32 キャストは 0 へ潰す"
        );
    }
}

/// 要件 2.5: `scale_len` の中間は **u128**——u64 では溢れる入力でも厳密値を返す。
///
/// 既存の大寸テスト `scale_len_handles_large_extents_without_overflow` も
/// `extreme = u32::MAX/(u32::MAX−1)` × `len = u32::MAX` で**既に u64 溢れ域を踏んでいる**
/// ため、中間幅そのものの檻は既存にもある。既存に欠けていたのは、結果が `u32::MAX` へ
/// 飽和しない witness——すなわち「**u64 なら溢れるのに真値は u32 域に収まる**」入力——で、
/// これが無いと「溢れる域だけ `u32::MAX` へ逃げる」実装を見分けられない。本テストは
/// その witness を構成し、飽和値ではない厳密値を主張する。
///
/// - `k = (u32::MAX − 1)/u32::MAX`、`len = u32::MAX` のとき
///   `2·len·num ≈ 3.69e19 > u64::MAX ≈ 1.84e19`（u64 なら debug でパニック・
///   release ならラップ）。真値は `4_294_967_294`（＝飽和値 `u32::MAX` ではない）。
///
/// # 殺す変異（変異注入の実測に基づく）
///
/// - **既存と共倒れ**: `scale_len` の `u128` を `u64` へ落とす（オーバーフローで落ちる）。
///   実測 2 失敗＝本テスト＋既存 `scale_len_handles_large_extents_without_overflow`。
/// - **排他的キル**: 「**u64 が溢れる域でのみ** `u32::MAX` へ逃げる」変異（溢れ判定を入れて
///   早期 `return u32::MAX`）。実測 1 失敗＝本テストのみ（既存は全て緑）。既存の大寸テストは
///   結果が `u32::MAX` へ飽和する族ばかりで飽和値と厳密値を区別できないため、この変異は
///   既存の檻を素通りする。本テストの固有価値はこの 1 変異に限定される。
#[test]
fn scale_len_u128_intermediate_beats_u64_overflow() {
    let k = ScaleRatio::new(u32::MAX - 1, u32::MAX).unwrap();
    assert!(!k.is_identity(), "既約のまま恒等短絡へ落ちない");

    // (2·len·num + den) / (2·den) を u128 で厳密に解いた値。
    assert_eq!(k.scale_len(u32::MAX), 4_294_967_294);
    assert_eq!(k.scale_len(4_000_000_000), 3_999_999_999);
    assert_eq!(
        k.scaled_extent(u32::MAX, 4_000_000_000),
        (4_294_967_294, 3_999_999_999),
        "外形も各軸へ同一権威を適用"
    );

    // 飽和値ではないこと（「無条件 u32::MAX」変異の直接の檻）。
    assert_ne!(k.scale_len(u32::MAX), u32::MAX);
    assert_ne!(k.scale_len(4_000_000_000), u32::MAX);

    // 決定論（同一入力は同一出力）。
    assert_eq!(k.scale_len(u32::MAX), k.scale_len(u32::MAX));
}

// ------------------------------------------------------------------
// areka-P0-collision-dpi-hittest task 1: `unscale_coord`（除算方向の丸め権威）
// 設計 DD-1／Testing Strategy「Unit Tests（scale.rs — unscale_coord の丸め権威檻）」の 6 項目。
// ------------------------------------------------------------------

/// 檻 1（要件 1.5／2.5）: k=1 は**全域恒等** `s(v)=v`——負値・0・i64 極値近傍を含む。
///
/// 要件 1.5 の no-op 保存はこの厳密恒等に依拠する。i64 極値を入れてあるため、
/// 中間演算を i128 でなく i64 で書いた実装は `2·v+1` の桁溢れで死ぬ。
#[test]
fn unscale_coord_is_exact_identity_for_k_one() {
    for v in [
        0i64,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        100,
        -100,
        123_456_789,
        -123_456_789,
        i32::MAX as i64,
        i32::MIN as i64,
        i64::MAX - 1,
        i64::MAX,
        i64::MIN + 1,
        i64::MIN,
    ] {
        assert_eq!(ScaleRatio::ONE.unscale_coord(v), v, "k=1 の恒等: v={v}");
    }

    // 既約化で 1/1 になる比も同じ恒等経路（96/96 は k=1）。
    let k = ScaleRatio::new(AUTHOR_DPI, AUTHOR_DPI).unwrap();
    for v in [-7i64, 0, 7, i64::MAX, i64::MIN] {
        assert_eq!(k.unscale_coord(v), v, "96/96 の恒等: v={v}");
    }

    // 決定論（同一入力は同一出力）。
    assert_eq!(
        ScaleRatio::ONE.unscale_coord(-42),
        ScaleRatio::ONE.unscale_coord(-42)
    );
}

/// 檻 2（要件 3.5 の期待値・DD-1 代表値）: k=2 表——100→50・101→50 と負値の床方向規約。
///
/// DD-1 が棄却した候補の witness を兼ねる:
/// - `scale_len` の round half away from zero を鏡写しにする候補 C なら 101→51（半画素ずれ）。
/// - `as` キャスト（0 方向切り捨て）なら -1→0・-3→-1（負側で床でなくなる）。
#[test]
fn unscale_coord_k2_table_pins_pixel_center_inverse() {
    let k2 = ScaleRatio::new(2, 1).unwrap();

    assert_eq!(k2.unscale_coord(100), 50, "DD-1 代表値");
    assert_eq!(k2.unscale_coord(101), 50, "DD-1 代表値（奇数座標）");
    assert_ne!(k2.unscale_coord(101), 51, "候補 C（長さの丸めの鏡写し）ではない");
    assert_eq!(k2.unscale_coord(102), 51);
    assert_eq!(k2.unscale_coord(103), 51);
    assert_eq!(k2.unscale_coord(0), 0);
    assert_eq!(k2.unscale_coord(1), 0);
    assert_eq!(k2.unscale_coord(2), 1);

    // 負値は Euclid 除算＝床方向（`as` の 0 方向切り捨てなら -1・-2 が 0 へ落ちる）。
    assert_eq!(k2.unscale_coord(-1), -1);
    assert_eq!(k2.unscale_coord(-2), -1);
    assert_eq!(k2.unscale_coord(-3), -2);
    assert_eq!(k2.unscale_coord(-4), -2);

    // 192dpi 由来の 2/1 も同一（構築経路によらない）。
    let k192 = ScaleRatio::new(192, AUTHOR_DPI).unwrap();
    assert_eq!(k192.unscale_coord(101), 50);
}

/// 檻 3（要件 3.5 の期待値・DD-1 代表値）: k=5/4 表——1→1・6→5（割り切れない縮約）。
///
/// 素の floor（DD-1 候補 A）なら 1→0（`floor(1·4/5)=0`）へ落ちる。本表はその棄却の witness。
#[test]
fn unscale_coord_k54_table_pins_non_divisible_reduction() {
    let k54 = ScaleRatio::new(5, 4).unwrap();

    assert_eq!(k54.unscale_coord(1), 1, "DD-1 代表値（候補 A なら 0）");
    assert_ne!(k54.unscale_coord(1), 0, "素の floor ではない");
    assert_eq!(k54.unscale_coord(6), 5, "DD-1 代表値");

    // 物理 0..=10 の全域表（s(v) = ⌊((2v+1)·4)/10⌋）。
    const TABLE: [i64; 11] = [0, 1, 2, 2, 3, 4, 5, 6, 6, 7, 8];
    for (v, &want) in TABLE.iter().enumerate() {
        assert_eq!(k54.unscale_coord(v as i64), want, "k=5/4 v={v}");
    }

    // 負側も床方向で定義される。
    assert_eq!(k54.unscale_coord(-1), -1);
    assert_eq!(k54.unscale_coord(-2), -2);

    // 120dpi 由来の 5/4 も同一。
    let k120 = ScaleRatio::new(120, AUTHOR_DPI).unwrap();
    assert_eq!(k120.unscale_coord(6), 5);
}

/// 檻 4（DD-1「端の注意」）: `scaled_extent` が切り上げた最終物理列は native 寸を 1 超える。
///
/// k=7/6・native 27 → 物理 32px（`scale_len(27)=32`）。最終列 31 の縮約は 27 ——
/// native の有効添字 0..=26 の**外側**である。collision 矩形は native 寸内ゆえ自然に None となり、
/// 定義された結果を返す（panic なし・要件 2.5 と整合）。
#[test]
fn unscale_coord_k76_final_column_maps_outside_native_extent() {
    let k76 = ScaleRatio::new(7, 6).unwrap();
    const NATIVE: u32 = 27;

    // 前提: 乗算方向権威が 31.5 を切り上げて 32 にしている。
    assert_eq!(k76.scale_len(NATIVE), 32);

    assert_eq!(
        k76.unscale_coord(31),
        NATIVE as i64,
        "最終物理列は native 寸(27)ちょうど＝有効添字 0..=26 の外側"
    );
    assert!(
        k76.unscale_coord(31) >= NATIVE as i64,
        "端の注意が成立している"
    );
    assert_eq!(k76.unscale_coord(30), 26, "その 1 つ手前は最終 native 画素");
    assert_eq!(k76.unscale_coord(0), 0);

    // 縮約結果が native 域に収まる範囲（0..=30）では添字が有効。
    for v in 0i64..=30 {
        let s = k76.unscale_coord(v);
        assert!((0..NATIVE as i64).contains(&s), "v={v} → s={s}");
    }
}

/// 檻 5（要件 2.3 の根拠）: v について**単調非減少**。
///
/// 閉区間矩形の逆像が物理空間でも連続区間になり、境界画素の内外一貫が k によらず
/// 保存されることの根拠（DD-1 性質 b）。
#[test]
fn unscale_coord_is_monotonic_non_decreasing() {
    for (num, den) in [
        (1u32, 1u32),
        (2, 1),
        (5, 4),
        (3, 2),
        (7, 4),
        (7, 6),
        (1, 2),
        (1, 100),
        (100, 1),
        (192, AUTHOR_DPI),
    ] {
        let k = ScaleRatio::new(num, den).unwrap();
        let mut prev = k.unscale_coord(-200);
        for v in -199i64..=400 {
            let cur = k.unscale_coord(v);
            assert!(
                cur >= prev,
                "k={num}/{den}: s({v})={cur} が s({})={prev} を下回った",
                v - 1
            );
            prev = cur;
        }
        // 非自明性（定値写像で単調性を満たす退化実装を排除）。
        assert!(
            k.unscale_coord(400) > k.unscale_coord(-200),
            "k={num}/{den}: 全域定値ではない"
        );
    }
}

/// 檻 6（要件 2.5・DD-1 縮小規約）: k<1 × i64 極値は**飽和縮小**（panic なし・飽和域で定値）。
///
/// `as` キャスト（ラップ＝単調性が破れる）でも `try_into().unwrap()`（panic）でもなく、
/// 飽和が唯一の整合解。Win32 実座標は i32 域に束縛されるため実経路で飽和は発生しない。
#[test]
fn unscale_coord_saturates_at_i64_extremes_for_k_below_one() {
    let half = ScaleRatio::new(1, 2).unwrap();

    // 非飽和域は厳密値 s(v) = 2v+1。
    assert_eq!(half.unscale_coord(0), 1);
    assert_eq!(half.unscale_coord(1), 3);
    assert_eq!(half.unscale_coord(100), 201);
    assert_eq!(half.unscale_coord(-1), -1);
    assert_eq!(half.unscale_coord(-100), -199);
    // Win32 実座標域（i32）では飽和しない＝防御規約であることの witness。
    assert_eq!(half.unscale_coord(i32::MAX as i64), 4_294_967_295);
    assert_eq!(half.unscale_coord(i32::MIN as i64), -4_294_967_295);

    // 正側の飽和（ラップなら負値になる）。
    assert_eq!(half.unscale_coord(i64::MAX), i64::MAX);
    assert_eq!(half.unscale_coord(i64::MAX - 1), i64::MAX);
    assert_eq!(
        half.unscale_coord(i64::MAX),
        half.unscale_coord(i64::MAX - 1),
        "飽和域は定値"
    );
    // 負側の飽和。
    assert_eq!(half.unscale_coord(i64::MIN), i64::MIN);
    assert_eq!(half.unscale_coord(i64::MIN + 1), i64::MIN);
    assert_eq!(
        half.unscale_coord(i64::MIN),
        half.unscale_coord(i64::MIN + 1),
        "飽和域は定値"
    );

    // 極端な k<1（1/u32::MAX）でも panic せず飽和する（i128 中間の桁溢れなし）。
    let shrink = ScaleRatio::new(1, u32::MAX).unwrap();
    assert_eq!(shrink.unscale_coord(i64::MAX), i64::MAX);
    assert_eq!(shrink.unscale_coord(i64::MIN), i64::MIN);
    assert_eq!(shrink.unscale_coord(0), (u32::MAX as i64) / 2);

    // k>1 側は極値でも飽和しない（縮小方向ゆえ i64 域に収まる）。
    let k2 = ScaleRatio::new(2, 1).unwrap();
    assert_eq!(k2.unscale_coord(i64::MAX), i64::MAX / 2);
    assert_eq!(k2.unscale_coord(i64::MIN), i64::MIN / 2);
}
