use super::ScaleRatio;
use super::fixture::{EXPECT_K_ENV, RATIO_PARTS_MAX_DEN};

// ---------------------------------------------------------------------------
// 実適用 k の表示と期待ゲート（env `AREKA_COLLISION_PROBE_EXPECT_K`）
// ---------------------------------------------------------------------------

/// [`ScaleRatio`] を `"<num>/<den>"` 表記へ整形する（常設 greppable ログ・assert メッセージ共通）。
///
/// 既約分子・分母を復元できなかった場合（[`RATIO_PARTS_MAX_DEN`] 超過の病的比）は `Debug` 表記へ縮退する
/// ——ログ表現の縮退であって判定には一切関与しない（無言では落とさない）。
pub(super) fn format_ratio(k: ScaleRatio) -> String {
    match ratio_parts(k) {
        Some((num, den)) => format!("{num}/{den}"),
        None => format!("{k:?}"),
    }
}

/// [`ScaleRatio`] の既約 `(num, den)` を**公開面のみ**で復元する（ログ表記専用）。
///
/// `ScaleRatio` は `num`／`den` アクセサを公開していない（`scale.rs` の申し送り: アクセサ新設は W6.5
/// `scale-exact-rational` の領分であり本 spec では追加しない）。そこで乗算方向の権威
/// [`ScaleRatio::scale_len`] と `ScaleRatio::new` の**正準化された等価判定**だけで復元する:
/// 分母候補 `d` を 1 から増やし `new(scale_len(d), d) == k` が最初に成立した `d` が既約分母である
/// （`gcd(num, den) == 1` ゆえ `0 < d < den` では `d·num/den` が整数にならず、丸めた比の正準形が `k` に
/// 一致することはない）。厳密であり、÷k の再実装でも丸め規約の持ち込みでもない。
fn ratio_parts(k: ScaleRatio) -> Option<(u32, u32)> {
    (1..=RATIO_PARTS_MAX_DEN).find_map(|den| {
        let num = k.scale_len(den);
        (ScaleRatio::new(num, den) == Some(k)).then_some((num, den))
    })
}

/// 期待 k ゲート（要件 4.1）: env [`EXPECT_K_ENV`] が指定されているときだけ、実適用 k との厳密一致を
/// hard assert する（design Error Handling「probe 期待 k 不一致 → hard assert で loud fail」）。
///
/// 未指定なら**何も assert しない**（実測ログのみ）——開発機の k がいくつであっても probe をそのまま
/// 実行できるようにするためであり、水準を偽って通す余地は生まない（指定した水準は必ず検査される）。
/// 値が解釈不能なとき（非数値・0 分母など）は環境設定ミスゆえ loud に panic する。
pub(super) fn assert_expected_ratio(applied: ScaleRatio) {
    let Some(expect) = expected_ratio() else {
        tracing::info!(
            env = EXPECT_K_ENV,
            k = %format_ratio(applied),
            "collision-probe: 期待 k ゲート未指定 — 実測 k のログのみ（実機サインオフでは水準ごとに設定すること）"
        );
        return;
    };
    assert_eq!(
        applied,
        expect,
        "collision-probe: 実適用 k={} が期待 k={}（env {EXPECT_K_ENV}）と不一致 — 表示スケール設定と期待値が食い違っている（この水準の証跡は無効）",
        format_ratio(applied),
        format_ratio(expect)
    );
    tracing::info!(
        env = EXPECT_K_ENV,
        k = %format_ratio(applied),
        "collision-probe: 期待 k ゲート通過（実適用 k が期待値と厳密一致）"
    );
}

/// env [`EXPECT_K_ENV`] を [`ScaleRatio`] へ解釈する（`"5/4"`＝分数・`"2"`＝整数 2/1・未設定/空は `None`）。
///
/// 解釈不能な値は probe の実行条件そのものが誤っている（＝採取する証跡が無意味になる）ため、縮退せず
/// panic で loud に落とす。
fn expected_ratio() -> Option<ScaleRatio> {
    let raw = std::env::var(EXPECT_K_ENV).ok()?;
    let spec = raw.trim();
    if spec.is_empty() {
        return None;
    }
    let (num_s, den_s) = match spec.split_once('/') {
        Some((n, d)) => (n.trim(), d.trim()),
        None => (spec, "1"),
    };
    let parse = |s: &str| -> u32 {
        s.parse::<u32>().unwrap_or_else(|e| {
            panic!(
                "collision-probe: env {EXPECT_K_ENV}={spec:?} を解釈できない（`5/4` または `2` 形式の正整数）: {e}"
            )
        })
    };
    let (num, den) = (parse(num_s), parse(den_s));
    Some(ScaleRatio::new(num, den).unwrap_or_else(|| {
        panic!("collision-probe: env {EXPECT_K_ENV}={spec:?} は 0 を含む（num>0・den>0 が必要）")
    }))
}
