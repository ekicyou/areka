//! # physical_extent_arbitration_test — 供給面寸 f32 経路の裁定を固定する決定論檻
//!
//! 裁定日: 2026-08-14（開発者裁定）。計測日: 2026-08-14（到達 23 比 × 寸 1..=1200 の総当たり）。
//! 既存の 2026-07-30 実測（`region.rs` の登記表）も裁定の根拠として併存する。
//!
//! 文字供給面の確保寸 [`ScaleContract::physical_extent`]（`ceil(image px 寸 × k_f32)`）は、k が
//! [`ScaleRatio::as_f32`] 由来の f32 であるため、真の積が整数になるはずの場合に `ceil` が +1 側へ振れることがある。
//! 裁定はこれを是正せず許容すると決めた。土台は「誤差は常に +1 側のみ・−1 は起きない・差は高々 1」という性質で、
//! 本ファイルはそれを検証された不変条件にする。
//!
//! 檻の到達範囲: [`ScaleRatio::as_f32`] 以降の算術（`as_f32` → [`ScaleContract::new`] → [`ScaleContract::physical_extent`]）
//! を貫通する。本番で k が経由する `TextSlotView.scale`（`crates/areka-emo-present/src/presenter/read.rs` の
//! `text_slot_view` 内 `scale: applied.as_f32()`）→ `TextSlotBinding::from_view`／`TextSlotBinding::new` の搬送層は
//! f32 素通しを前提とする（正規化は [`ScaleContract::new`] へ委譲されている）。搬送層が scale を変換するよう変わった場合、
//! その変換は本檻の外であり、注入点の再設計が要る。
//!
//! 道具の較正: 検証の道具そのものが壊れていても緑は出る（裁定時の初回集計は実際に道具の誤りを踏んだ）ので、
//! 比集合導出ヘルパが既知値を逐語再現することを最初に確かめる較正テストを備える。較正が緑でない限り後続の判定は意味を持たない。
//! 2 つの寸導出ヘルパ（本番経路・整数オラクル）の既知値は 6/5・12/5 の代表例のテストが固定する。

use areka_emo_compose::ScaleRatio;
use areka_emo_text::region::{ImagePx, ScaleContract};

// ── 検証対象の DPI 格子 ──────────────────────

/// 作者基準 DPI の語彙（`descript_balloon.dpi` として現実的に置かれる値）。
const AUTHOR_DPI: [u32; 4] = [72, 96, 120, 144];

/// モニタ DPI の対応域（Windows の 100%〜300% 系列）。
const MONITOR_DPI: [u32; 8] = [96, 120, 144, 168, 192, 216, 240, 288];

/// DPI 格子から導かれる到達比の個数（裁定実測の前提集合の固定）。格子が広がればこの値と、
/// 裁定の実測表（`region.rs` の登記）も同時に更新すること。
const REACHABLE_RATIO_COUNT: usize = 23;

/// 突合する寸の上限（`1..=MAX_EXTENT` の全ての寸を検証する）。
const MAX_EXTENT: u32 = 1200;

/// 誤差が実在する 2 比（既約 `(num, den)`）。12/5 は 6/5 の 2 倍尺で f32 仮数が同一のため、
/// 実質「1.2 の f32 表現」という一点に帰着する。
const ERROR_RATIOS: [(u32, u32); 2] = [(6, 5), (12, 5)];

/// 誤差が実在する比 1 つあたりの誤り件数（寸 `1..=`[`MAX_EXTENT`] の総当たり・2026-08-14 実測）。
const ERROR_COUNT_PER_RATIO: usize = 81;

// ── ヘルパ（テスト内私有・製品コードには足さない） ────────────────────────────

/// 最大公約数（Euclid の互除法・整数のみ）。比の既約化に用いる。
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// 検証対象の到達比（既約 `(num, den)`）を DPI 格子から導出する。作者 DPI [`AUTHOR_DPI`] × モニタ DPI [`MONITOR_DPI`] の
/// 全組で `k = モニタ DPI ÷ 作者基準 DPI` を作り、gcd で約分して重複を除く。直書きせず格子から導くのは、
/// 格子が変われば集合が自動で追随するため。
fn reachable_ratios() -> Vec<(u32, u32)> {
    let mut ratios: Vec<(u32, u32)> = Vec::new();
    for author in AUTHOR_DPI {
        for monitor in MONITOR_DPI {
            let g = gcd(monitor, author);
            let reduced = (monitor / g, author / g);
            if !ratios.contains(&reduced) {
                ratios.push(reduced);
            }
        }
    }
    ratios
}

/// 真値オラクル: `ceil(v × num / den)` を整数演算のみで計算する（浮動小数へ依存すると検証が循環する）。
/// 本檻の入力域（`v ≤ 1200`・`num ≤ 288`）では u64 中間は桁溢れしない。
fn true_ceil(v: u32, num: u32, den: u32) -> u32 {
    ((v as u64 * num as u64).div_ceil(den as u64)) as u32
}

/// 本番と同一経路で供給面寸を導出する: `ScaleRatio::new(num, den)` → [`ScaleRatio::as_f32`] →
/// [`ScaleContract::new`] → [`ScaleContract::physical_extent`]。式を本ファイル内で再実装しないのは、
/// 檻に入れるべきものが式の写しでなく本番配管の性質だから。
/// 格子由来の入力は常に正で `ScaleRatio::new` の `None` は起こらないので、到達したなら前提崩れとして `expect` で即失敗させる。
fn supply_extent_via_f32_path(v: u32, num: u32, den: u32) -> u32 {
    let ratio = ScaleRatio::new(num, den)
        .expect("到達比は格子由来で num/den ともに正のため ScaleRatio::new は必ず Some を返す");
    let contract = ScaleContract::new(ratio.as_f32(), None);
    contract.physical_extent(ImagePx(v as f32))
}

/// 1 つの比について、寸 `1..=`[`MAX_EXTENT`] を総当たりした誤りの集計。
struct ErrorTally {
    /// 本番経路の結果が整数オラクルの真値と食い違った件数。
    errors: usize,
    /// 最初に食い違った `(寸, 実測値, 真値)`。1 件も無ければ `None`。
    first: Option<(u32, u32, u32)>,
}

/// 1 つの比について誤り件数を集計する。最初の食い違いも持ち帰り、失敗メッセージにどの寸で割れたかを書けるようにする。
/// 判定は等値比較のみで `u32` の減算を用いない（差の大きさは上下界のテストが受け持つ）。
fn tally_extent_errors(num: u32, den: u32) -> ErrorTally {
    let mut tally = ErrorTally {
        errors: 0,
        first: None,
    };
    for v in 1..=MAX_EXTENT {
        let actual = supply_extent_via_f32_path(v, num, den);
        let truth = true_ceil(v, num, den);
        if actual != truth {
            tally.errors += 1;
            if tally.first.is_none() {
                tally.first = Some((v, actual, truth));
            }
        }
    }
    tally
}

// ── 較正（観測できる完了の確かめ） ──────────────────

/// 比集合導出を、既知値の逐語再現で較正する。判定に使う道具が壊れていても緑は出るので、後続の判定より先に道具の側を固定する。
///
/// 1. 到達比がちょうど [`REACHABLE_RATIO_COUNT`] 個（＝23）であること。
/// 2. 誤差が実在する 2 比（[`ERROR_RATIOS`]＝6/5・12/5）が集合に含まれること。取りこぼすと「誤りは 2 比のみ」という裁定根拠が空振りの緑になる。
///
/// 本番経路のヘルパと整数オラクルの既知値（比 6/5・寸 25 で 31 と 30、比 12/5 で 61 と 60）は
/// [`error_counts_fixed_for_six_fifths_and_twelve_fifths`] が受け持つ。
#[test]
fn ratio_set_helper_is_calibrated_against_known_values() {
    let ratios = reachable_ratios();

    assert_eq!(
        ratios.len(),
        REACHABLE_RATIO_COUNT,
        "到達比の個数が期待と異なる（作者 DPI {AUTHOR_DPI:?} × モニタ DPI {MONITOR_DPI:?} \
         の約分・重複排除後）: 実測 {} 個 / 期待 {REACHABLE_RATIO_COUNT} 個・実測集合 {ratios:?}",
        ratios.len()
    );

    for known in ERROR_RATIOS {
        assert!(
            ratios.contains(&known),
            "誤差が実在する比 {}/{} が到達比集合に含まれていない（実測集合 {ratios:?}）",
            known.0,
            known.1
        );
    }
}

// ── 上下界（全到達比 × 全寸の総当たり） ────────────────

/// 供給面寸は全到達比・全寸で真値以上かつ差 1 以内であることを検証する。裁定が「誤差は常に +1 側のみで文字は切れない」と
/// 言い切れる土台は次の 2 つの境界で、到達比 [`REACHABLE_RATIO_COUNT`] 個 × 寸 `1..=`[`MAX_EXTENT`] の全組で突合する。
///
/// - 下界: 実測値が真値を下回らないこと。下回れば確保寸が足りず文字が欠ける（許容されるのは「余る」側だけ）。
/// - 上界: 実測値と真値の差が 0 または 1 に限られること。2 以上ずれると「余りは高々 1 画素＝不可視」という前提が崩れる。
///
/// 2 つの境界は個別の assert とし、失敗時に分子・分母・寸・実測値・真値を並べる（「下回った」のか「離れすぎた」のかを読めるように）。
///
/// 差は `u32` 同士の減算なので、`実測値 < 真値` のまま引くと巨大値へ回り込んで上界の assert が別の顔で誤報する。
/// そこで下界を先に確かめ、通過した後にだけ減算する。
///
/// 整数オラクルの中間値は `v as u64 * num as u64` で最大 345,600、除数 `den` は格子由来で常に正なので桁溢れもゼロ除算もない。
/// 実 DPI モニタ・GPU・実窓は使わず、拡大率は [`ScaleContract::new`] へ注入する純粋な算術として走る。
#[test]
fn supply_extent_bounds_hold_for_all_reachable_ratios() {
    // 到達比の個数（総当たりの前提）は `ratio_set_helper_is_calibrated_against_known_values` が固定する。
    let ratios = reachable_ratios();
    for &(num, den) in &ratios {
        for v in 1..=MAX_EXTENT {
            let actual = supply_extent_via_f32_path(v, num, den);
            let truth = true_ceil(v, num, den);

            // 下界を先に確かめる。ここを通さずに引き算へ進むと u32 が回り込む。
            assert!(
                actual >= truth,
                "供給面寸が真値を下回った（確保寸が足りず文字が欠ける）: \
                 num={num} den={den} v={v} 実測値={actual} 真値={truth}"
            );

            let diff = actual - truth;
            assert!(
                diff <= 1,
                "供給面寸と真値の差が 1 を超えた（余りは高々 1 画素という裁定の前提が崩れる）: \
                 num={num} den={den} v={v} 実測値={actual} 真値={truth} 差={diff}"
            );
        }
    }
}

// ── 誤り件数と代表例の固定 ────────────────────────

/// 誤りが出る 2 比（6/5・12/5）の件数と代表例を、是正しない判断の期待値として固定する。
///
/// 計測日 2026-08-14（到達 23 比 × 寸 1..=1200 の総当たり）。ここに並ぶ 81/81 件と代表例は直すべき欠陥の記録ではなく、
/// 裁定は誤差を是正しないと決めている。件数が動けば赤になるが、それは実装の劣化ではなく裁定の再審トリガ——本番の f32 経路
/// （`ScaleRatio::as_f32` / `ScaleContract::physical_extent` の式）か到達比集合のどちらかが変わった合図であり、許容の判断を下し直す必要がある。
///
/// - 比 6/5: 誤り**ちょうど 81 件**・代表例 **寸 25 → 実測 31（真値 30）**
/// - 比 12/5: 誤り**ちょうど 81 件**・代表例 **寸 25 → 実測 61（真値 60）**
///
/// 代表例は件数とは別に個別の assert とする。件数だけでは誤り方（+1 側へ 1 だけ振れる）が入れ替わっても緑のままになり得る。
/// 2 比の件数が一致することは、12/5 が 6/5 の 2 倍尺で f32 仮数を共有する（欠陥の正体が「1.2 の f32 表現」一点である）ことの傍証でもある。
/// 2 比が到達比集合に実在することは [`ratio_set_helper_is_calibrated_against_known_values`] が担保する。
///
/// 件数を期待値として固定できるのは、突合の両辺が決定論だから。f32 の乗除算と `ceil` は IEEE 754 単精度でビット単位に一意で
/// x64・arm64 とも同値、整数オラクルは u64 の切り上げ除算のみ。実行環境・回数で件数は揺れない。
/// 実 DPI モニタ・GPU・実窓は使わない。
#[test]
fn error_counts_fixed_for_six_fifths_and_twelve_fifths() {
    for (num, den) in ERROR_RATIOS {
        let tally = tally_extent_errors(num, den);

        assert_eq!(
            tally.errors, ERROR_COUNT_PER_RATIO,
            "誤り件数が固定値から動いた（＝裁定の再審トリガ・2026-08-14 実測との相違）: \
             num={num} den={den} 寸域 1..={MAX_EXTENT} 実測 {} 件 / 期待 {ERROR_COUNT_PER_RATIO} 件・\
             最初の食い違い（寸, 実測値, 真値）={:?}",
            tally.errors, tally.first
        );
    }

    // 代表例（2026-08-14 実測）。件数とは別に、誤り方そのものを個別に固定する。
    for (num, den, v, expected_actual, expected_truth) in
        [(6u32, 5u32, 25u32, 31u32, 30u32), (12, 5, 25, 61, 60)]
    {
        let actual = supply_extent_via_f32_path(v, num, den);
        let truth = true_ceil(v, num, den);
        assert_eq!(
            actual, expected_actual,
            "代表例の本番経路の値が固定値から動いた（＝裁定の再審トリガ）: \
             num={num} den={den} v={v} 実測値={actual} 真値={truth} 期待実測値={expected_actual}"
        );
        assert_eq!(
            truth, expected_truth,
            "代表例の真値が固定値から動いた（整数オラクルの前提が崩れている疑い）: \
             num={num} den={den} v={v} 実測値={actual} 真値={truth} 期待真値={expected_truth}"
        );
    }
}

// ── ゼロ件集合の固定 ──────────────────────────────────

/// 誤りが出る 2 比を除く残り 21 比では、誤りが 1 件も出ないことを固定する。
///
/// 計測日 2026-08-14。裁定根拠⑶「救える範囲が極小」は誤りの側（各 81 件）だけでは成立せず、誤りゼロの側を同時に押さえて
/// 初めて「救える範囲は 2 比に限られる」と言い切れる。是正しない判断を期待値として固定するので、ゼロでない比が現れれば赤になるが、
/// それは実装の劣化ではなく裁定の再審トリガ（誤差の広がりを前提から見直す合図）である。
///
/// 対象の 21 比は直書きせず、[`reachable_ratios`] から [`ERROR_RATIOS`] を除いて導く。個数と [`ERROR_RATIOS`] の所属は
/// [`ratio_set_helper_is_calibrated_against_known_values`] が固定するので、残りはちょうど 21 比になり比集合の変化を取りこぼさない。
/// 突合の両辺は決定論で、集計は等値比較のみ。実 DPI モニタ・GPU・実窓は使わない。
#[test]
fn remaining_ratios_have_zero_errors() {
    let remaining: Vec<(u32, u32)> = reachable_ratios()
        .into_iter()
        .filter(|ratio| !ERROR_RATIOS.contains(ratio))
        .collect();

    for &(num, den) in &remaining {
        let tally = tally_extent_errors(num, den);
        assert_eq!(
            tally.errors, 0,
            "誤りゼロのはずの比で食い違いが出た（＝裁定の再審トリガ・誤差が 2 比の外へ広がった）: \
             num={num} den={den} 寸域 1..={MAX_EXTENT} 実測 {} 件 / 期待 0 件・\
             最初の食い違い（寸, 実測値, 真値）={:?}",
            tally.errors, tally.first
        );
    }
}
