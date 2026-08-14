//! # physical_extent_arbitration_test — 供給面寸 f32 経路の裁定を固定する決定論檻
//!
//! **裁定日**: 2026-08-14（開発者裁定）。
//! **出典 spec**: `areka-P0-scale-exact-rational`（完了後は `.kiro/specs/completed/` 配下。
//! パスではなく spec 名で辿ること）。
//! **計測日**: 2026-08-14（到達 23 比 × 寸 1..=1200 の総当たり）。既存の 2026-07-30 実測
//! （`region.rs` の登記表）も裁定の根拠として併存する。
//!
//! ## この檻が固定するもの
//!
//! 文字供給面の確保寸 [`ScaleContract::physical_extent`]（`ceil(image px 寸 × k_f32)`）は、
//! k が [`ScaleRatio::as_f32`] 由来の f32 であるため、真の積が整数になるはずの場合に `ceil`
//! が **+1 側へ振れる**ことがある。2026-08-14 の裁定はこれを**是正せず許容する**と決めた。
//! 裁定の土台は「誤差は常に +1 側のみ・−1 は起きない・差は高々 1」という性質であり、
//! 本ファイルはその性質を主張ではなく検証された不変条件にする（requirements 3.1〜3.7）。
//!
//! ## 檻の到達範囲（重要・design.md C3 Implementation Notes と同旨）
//!
//! 本檻は [`ScaleRatio::as_f32`] 以降の算術（`as_f32` → [`ScaleContract::new`] →
//! [`ScaleContract::physical_extent`]）を貫通する。一方、本番で k が経由する
//! `TextSlotView.scale`（`crates/areka-emo-present/src/presenter/read.rs:109`）→
//! `TextSlotBinding::from_view`／`TextSlotBinding::new` の搬送層は **f32 素通し**を前提とする
//! （現物は素通しで、正規化は [`ScaleContract::new`] へ委譲されている）。
//! 搬送層が scale を変換するよう変わった場合、その変換は本檻の外であり、注入点の再設計が要る。
//!
//! ## 道具の較正（[[subagent-tooling-can-be-wrong-calibrate-it]]）
//!
//! 検証の道具そのものが壊れていても緑は出る。裁定時の初回集計は実際に道具の誤りを踏んだ。
//! そのため本ファイルは、比集合導出ヘルパと 2 つの寸導出ヘルパ（本番経路・整数オラクル）が
//! **既知値を逐語再現する**ことを最初に確かめる較正テストを備える。
//! 較正が緑でない限り、後続の判定は意味を持たない。

use areka_emo_compose::ScaleRatio;
use areka_emo_text::region::{ImagePx, ScaleContract};

// ── 検証対象の DPI 格子（design.md D3・requirements 3.3） ──────────────────────

/// 作者基準 DPI の語彙（`descript_balloon.dpi` として現実的に置かれる値）。
const AUTHOR_DPI: [u32; 4] = [72, 96, 120, 144];

/// モニタ DPI の対応域（Windows の 100%〜300% 系列）。
const MONITOR_DPI: [u32; 8] = [96, 120, 144, 168, 192, 216, 240, 288];

/// DPI 格子から導かれる到達比の個数（裁定実測の前提集合そのものの固定）。
///
/// 作者 DPI の語彙またはモニタ DPI の対応域が広がればこの値は変わる。その場合は
/// 裁定の実測表（`region.rs` の登記）も同時に更新すること（design.md Revalidation Triggers）。
const REACHABLE_RATIO_COUNT: usize = 23;

/// 突合する寸の上限（`1..=MAX_EXTENT` の全ての寸を検証する・requirements 3.3）。
const MAX_EXTENT: u32 = 1200;

/// 誤差が実在する 2 比（既約 `(num, den)`・design.md D4）。
///
/// 12/5 は 6/5 の 2 倍尺で f32 仮数が同一のため、この 2 比は実質
/// **「1.2 の f32 表現」という一点**に帰着する（裁定根拠⑶）。
const ERROR_RATIOS: [(u32, u32); 2] = [(6, 5), (12, 5)];

/// 誤差が実在する比 1 つあたりの誤り件数（寸 `1..=`[`MAX_EXTENT`] の総当たり・2026-08-14 実測）。
const ERROR_COUNT_PER_RATIO: usize = 81;

/// 誤りが 1 件も出ない比の個数（到達比から [`ERROR_RATIOS`] を除いた残り＝21）。
///
/// 一覧を直書きせず [`REACHABLE_RATIO_COUNT`] からの差で導く。比集合が広がればこの値も動き、
/// ゼロ件側の assert が赤になる（design.md Revalidation Triggers）。
const ZERO_ERROR_RATIO_COUNT: usize = REACHABLE_RATIO_COUNT - ERROR_RATIOS.len();

/// 上下界の総当たりで踏むべき評価回数（到達比 23 × 寸 1200 ＝ 27,600）。
///
/// ループが途中で痩せても緑は出るため、踏んだ回数そのものを期待値として固定する。
const EVALUATION_COUNT: usize = REACHABLE_RATIO_COUNT * MAX_EXTENT as usize;

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

/// 検証対象の到達比（既約 `(num, den)`）を DPI 格子から導出する（design.md D3）。
///
/// 作者 DPI [`AUTHOR_DPI`] × モニタ DPI [`MONITOR_DPI`] の全組で
/// `k = モニタ DPI ÷ 作者基準 DPI` を作り、gcd で約分して重複を除く。
/// 比の一覧を直書きせず格子から導くのは、「現実的な組合せから導かれる比を網羅」という
/// requirements 3.3 の直訳であり、格子が変われば集合も自動で追随させるためである。
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

/// 真値オラクル: `ceil(v × num / den)` を**整数演算のみ**で計算する（design.md D2）。
///
/// オラクル自体が浮動小数へ依存すると検証が循環するため、u64 中間の切り上げ除算だけで組む。
/// 本檻の入力域（`v ≤ 1200`・`num ≤ 288`）では u64 中間は桁溢れせず、
/// パニックもラップアラウンドも起こさない（requirements 3.7）。
fn true_ceil(v: u32, num: u32, den: u32) -> u32 {
    ((v as u64 * num as u64).div_ceil(den as u64)) as u32
}

/// 本番と同一経路で供給面寸を導出する（design.md D1）。
///
/// `ScaleRatio::new(num, den)` → [`ScaleRatio::as_f32`] → [`ScaleContract::new`] →
/// [`ScaleContract::physical_extent`] を実際に通す。`num as f32 / den as f32` を
/// 本ファイル内で再実装しないのは、檻に入れるべきものが「式の写し」ではなく
/// **本番配管の性質**だからである。
///
/// `ScaleRatio::new` は `Option` を返すが、格子由来の入力は常に正であり `None` は起こらない。
/// 万一到達したなら前提が崩れているので、黙って読み飛ばさず `expect` で即座に失敗させる。
fn supply_extent_via_f32_path(v: u32, num: u32, den: u32) -> u32 {
    let ratio = ScaleRatio::new(num, den)
        .expect("到達比は格子由来で num/den ともに正のため ScaleRatio::new は必ず Some を返す");
    let contract = ScaleContract::new(ratio.as_f32(), None);
    contract.physical_extent(ImagePx(v as f32))
}

/// 1 つの比について、寸 `1..=`[`MAX_EXTENT`] を総当たりした誤りの集計。
struct ErrorTally {
    /// 実際に踏んだ評価回数（ループが痩せていないことの証跡）。
    evaluated: usize,
    /// 本番経路の結果が整数オラクルの真値と食い違った件数。
    errors: usize,
    /// 最初に食い違った `(寸, 実測値, 真値)`。1 件も無ければ `None`。
    first: Option<(u32, u32, u32)>,
}

/// 1 つの比について誤り件数を集計する（design.md C3 テスト②③の共通土台）。
///
/// 件数だけでは失敗メッセージに「どの寸で割れたか」を書けないため、最初の食い違いを
/// 併せて持ち帰る（design.md Error Handling——ログ突合なしにその 1 件を再現できる形）。
///
/// 判定は等値比較（`実測値 != 真値`）のみで行い、`u32` の減算を一切用いない。差の大きさは
/// 上下界のテストが受け持つため、ここにラップアラウンドの余地を作らない（requirements 3.7）。
fn tally_extent_errors(num: u32, den: u32) -> ErrorTally {
    let mut tally = ErrorTally {
        evaluated: 0,
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
        tally.evaluated += 1;
    }
    tally
}

// ── 較正（本タスクの観測可能な完了・design.md C3 Validation） ──────────────────

/// 比集合導出・整数オラクル・本番経路ヘルパの 3 点を、既知値の逐語再現で較正する。
///
/// 判定に使う道具が壊れていても緑は出るため、後続の判定より先に道具の側を固定する
/// （[[subagent-tooling-can-be-wrong-calibrate-it]]）。ここで確かめるのは次の 4 点である。
///
/// 1. 到達比がちょうど [`REACHABLE_RATIO_COUNT`] 個（＝23）であること。
/// 2. 誤差が実在する 2 比（6/5・12/5）が集合に含まれること——この 2 比を取りこぼすと、
///    「誤りは 2 比のみ」という裁定根拠が空振りの緑になる。
/// 3. 返る比がすべて既約かつ重複なしであること（オラクルと本番経路が同じ比を見る前提）。
/// 4. 既知値の逐語再現: 比 6/5・寸 25 で**本番経路が 31**・**整数オラクルが 30** を返すこと
///    （2026-07-30 の実測表に載る代表例そのもの）。差 +1 がここで現れなければ、
///    どちらかのヘルパが本番と別の経路を辿っている。
#[test]
fn helpers_are_calibrated_against_known_values() {
    let ratios = reachable_ratios();

    assert_eq!(
        ratios.len(),
        REACHABLE_RATIO_COUNT,
        "到達比の個数が期待と異なる（作者 DPI {AUTHOR_DPI:?} × モニタ DPI {MONITOR_DPI:?} \
         の約分・重複排除後）: 実測 {} 個 / 期待 {REACHABLE_RATIO_COUNT} 個・実測集合 {ratios:?}",
        ratios.len()
    );

    for known in [(6u32, 5u32), (12u32, 5u32)] {
        assert!(
            ratios.contains(&known),
            "誤差が実在する比 {}/{} が到達比集合に含まれていない（実測集合 {ratios:?}）",
            known.0,
            known.1
        );
    }

    for (i, &(num, den)) in ratios.iter().enumerate() {
        assert_eq!(
            gcd(num, den),
            1,
            "到達比 {num}/{den} が既約でない（gcd={}）",
            gcd(num, den)
        );
        assert!(
            !ratios[..i].contains(&(num, den)),
            "到達比 {num}/{den} が重複している（実測集合 {ratios:?}）"
        );
    }

    // 既知値の逐語再現（比 6/5・寸 25）。本番経路は +1 側へ振れ、整数オラクルは真値を返す。
    let (num, den, v) = (6u32, 5u32, 25u32);
    // 2026-07-30 実測表の代表例（`region.rs` の裁定登記に載る値）。
    let expected_via_f32 = 31u32;
    let expected_truth = 30u32;
    let via_f32 = supply_extent_via_f32_path(v, num, den);
    let truth = true_ceil(v, num, den);
    assert_eq!(
        via_f32, expected_via_f32,
        "本番経路（ScaleRatio::as_f32 → ScaleContract::physical_extent）の既知値が再現しない: \
         num={num} den={den} v={v} 実測={via_f32} 期待={expected_via_f32}"
    );
    assert_eq!(
        truth, expected_truth,
        "整数オラクル ceil(v·num/den) の既知値が再現しない: \
         num={num} den={den} v={v} 実測={truth} 期待={expected_truth}"
    );
    assert_eq!(
        via_f32 - truth,
        1,
        "既知の +1 誤差が現れない（どちらかのヘルパが本番と別経路を辿っている疑い）: \
         num={num} den={den} v={v} 本番経路={via_f32} 真値={truth}"
    );
}

// ── 上下界（全到達比 × 全寸の総当たり・design.md C3 テスト①） ────────────────

/// 供給面寸は全到達比・全寸で真値以上かつ差 1 以内であることを検証する。
///
/// 裁定（2026-08-14）が「誤差は常に +1 側のみで文字は切れない」と言い切れる土台は、
/// 次の 2 つの境界がどの比・どの寸でも破れないことである。本テストは到達比
/// [`REACHABLE_RATIO_COUNT`] 個 × 寸 `1..=`[`MAX_EXTENT`] の全 [`EVALUATION_COUNT`] 組で
/// 両方を突合する（requirements 3.3）。
///
/// - **下界**（requirements 3.1）: 実測値が真値を下回らないこと。下回れば確保寸が足りず
///   文字が欠ける——裁定が許容しているのは「余る」側だけであり、欠ける側は許容ではない。
/// - **上界**（requirements 3.2）: 実測値と真値の差が 0 または 1 に限られること。2 以上ずれると
///   「余りは高々 1 画素＝不可視」という裁定の前提そのものが崩れる。
///
/// 2 つの境界は個別の assert として置く（requirements 3.5）。単一の真偽値へ畳むと、
/// 赤になったときに「下回った」のか「離れすぎた」のかが失敗メッセージから読めなくなる。
/// いずれの assert も失敗時に分子・分母・寸・実測値・真値を並べるため、ログ突合なしに
/// その 1 件だけを再現できる（design.md Error Handling）。
///
/// ## ラップアラウンドを踏まない順序（requirements 3.7）
///
/// 差は `u32` 同士の減算で求めるため、`実測値 < 真値` のまま引くと巨大値へ回り込み、
/// 上界の assert が「差 2 以上」という別の顔で誤報する。そこで**下界を先に確かめ**、
/// 通過した後にだけ減算する。この順序により減算は常に非負であり、回り込みは起こらない。
///
/// ## 桁溢れとパニックが起きないこと（requirements 3.7）
///
/// 整数オラクルの中間値は `v as u64 * num as u64` であり、本檻の入力域は
/// `v ≤ 1200`・`num ≤ 288` ゆえ最大でも 345,600 で `u64` の桁を全く脅かさない。
/// 除数 `den` は格子由来で常に正のためゼロ除算もない。総当たりが最後まで走り切り
/// 評価回数が [`EVALUATION_COUNT`] に一致することを末尾で確かめる。
///
/// ## 実行条件（requirements 3.6）
///
/// 実 DPI モニタ・GPU・実窓を一切使わない。拡大率は [`ScaleContract::new`] へ注入するだけで、
/// OS の DPI 状態にも描画資源にも触れない純粋な算術として走る。
#[test]
fn supply_extent_bounds_hold_for_all_reachable_ratios() {
    let ratios = reachable_ratios();
    assert_eq!(
        ratios.len(),
        REACHABLE_RATIO_COUNT,
        "到達比の個数が期待と異なるため総当たりの前提が崩れている: \
         実測 {} 個 / 期待 {REACHABLE_RATIO_COUNT} 個・実測集合 {ratios:?}",
        ratios.len()
    );

    let mut evaluated = 0usize;
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

            // ここへ到達した時点で actual >= truth が確定しているため、減算は必ず非負。
            let diff = actual - truth;
            assert!(
                diff <= 1,
                "供給面寸と真値の差が 1 を超えた（余りは高々 1 画素という裁定の前提が崩れる）: \
                 num={num} den={den} v={v} 実測値={actual} 真値={truth} 差={diff}"
            );

            evaluated += 1;
        }
    }

    assert_eq!(
        evaluated, EVALUATION_COUNT,
        "総当たりの評価回数が期待と異なる（ループが痩せている疑い）: \
         実測 {evaluated} 回 / 期待 {EVALUATION_COUNT} 回"
    );
}

// ── 誤り件数と代表例の固定（design.md C3 テスト②・D4） ────────────────────────

/// 誤りが出る 2 比（6/5・12/5）の件数と代表例を、**是正しない判断の期待値**として固定する。
///
/// **計測日 2026-08-14**（到達 23 比 × 寸 1..=1200 の総当たり）。ここに並ぶ 81/81 件と
/// 代表例は「直すべき欠陥の記録」ではない。2026-08-14 の裁定は誤差を**是正しない**と決めており、
/// 本テストはその**判断そのものを期待値として固定する**（requirements 3.4・design.md D4）。
/// したがって件数が動けば赤になるが、それは実装の劣化ではなく **裁定の再審トリガ**である
/// ——本番の f32 経路（`ScaleRatio::as_f32` / `ScaleContract::physical_extent` の式）か
/// 到達比集合のどちらかが変わった合図であり、許容の判断を下し直す必要がある。
///
/// ## 固定する値（2026-08-14 実測）
///
/// - 比 6/5: 誤り **ちょうど 81 件**・代表例 **寸 25 → 実測 31（真値 30）**
/// - 比 12/5: 誤り **ちょうど 81 件**・代表例 **寸 25 → 実測 61（真値 60）**
///
/// 代表例は件数とは別に個別の assert として置く。件数だけでは「81 件出ている」ことしか
/// 言えず、誤り方（+1 側へ 1 だけ振れる）が入れ替わっても緑のままになり得るためである。
/// 2 比の件数が一致することは、12/5 が 6/5 の 2 倍尺で f32 仮数を共有する——すなわち
/// 欠陥の正体が「1.2 の f32 表現」一点である——という裁定根拠⑶の傍証でもある。
///
/// なお 2 比が到達比集合に実在することは較正テスト
/// [`helpers_are_calibrated_against_known_values`] が担保する（ここでは重ねない）。
///
/// ## 同一入力に対し常に同一結果であること（requirements 3.7）
///
/// 件数を期待値として固定できる前提は、突合の両辺が決定論であることである。
/// 本番経路側の f32 の乗除算と `ceil` は IEEE 754 単精度で結果がビット単位に一意と規定され、
/// x64・arm64 のいずれでも同一の値を返す。整数オラクル側は u64 の切り上げ除算のみで
/// 浮動小数を含まない。よって同一入力は常に同一結果を返し、実行環境・実行回数で件数は揺れない。
/// 集計は等値比較のみで行い `u32` の減算を用いないため、ラップアラウンドも起こらない。
///
/// ## 実行条件（requirements 3.6）
///
/// 実 DPI モニタ・GPU・実窓を一切使わない。拡大率は [`ScaleContract::new`] へ注入するだけの
/// 純粋な算術として走る。
#[test]
fn error_counts_fixed_for_six_fifths_and_twelve_fifths() {
    for (num, den) in ERROR_RATIOS {
        let tally = tally_extent_errors(num, den);

        assert_eq!(
            tally.evaluated, MAX_EXTENT as usize,
            "誤り集計の評価回数が期待と異なる（ループが痩せている疑い）: \
             num={num} den={den} 実測 {} 回 / 期待 {} 回",
            tally.evaluated, MAX_EXTENT
        );

        assert_eq!(
            tally.errors, ERROR_COUNT_PER_RATIO,
            "誤り件数が固定値から動いた（＝裁定の再審トリガ・2026-08-14 実測との相違）: \
             num={num} den={den} 寸域 1..={MAX_EXTENT} 実測 {} 件 / 期待 {ERROR_COUNT_PER_RATIO} 件・\
             最初の食い違い（寸, 実測値, 真値）={:?}",
            tally.errors, tally.first
        );
    }

    // 代表例（2026-08-14 実測・design.md D4）。件数とは別に、誤り方そのものを個別に固定する。
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

// ── ゼロ件集合の固定（design.md C3 テスト③） ──────────────────────────────────

/// 誤りが出る 2 比を除く残り 21 比では、誤りが **1 件も出ない**ことを固定する。
///
/// **計測日 2026-08-14**。裁定根拠⑶「救える範囲が極小」は、誤りの側（各 81 件）だけでは
/// 成立しない。**誤りゼロの側**を同時に押さえて初めて「救える範囲は 2 比に限られる」と
/// 言い切れる。本テストもまた**是正しない判断を期待値として固定する**ものであり
/// （requirements 3.4）、ゼロでない比が現れれば赤になる——それは実装の劣化ではなく
/// **裁定の再審トリガ**（誤差の広がりを前提から見直す合図）である。
///
/// 対象の 21 比は一覧を直書きせず、[`reachable_ratios`] から [`ERROR_RATIOS`] を除いて導く。
/// 到達比集合が変われば残りの個数が [`ZERO_ERROR_RATIO_COUNT`] から外れてここが赤になり、
/// 比集合の変化を取りこぼさない（design.md Revalidation Triggers）。
///
/// ゼロ件の主張は、ループが空でも同じく緑になる。そこで踏んだ評価回数
/// （21 比 × 寸 [`MAX_EXTENT`]）も併せて固定し、空振りの緑を塞ぐ。
///
/// ## 同一入力に対し常に同一結果であること（requirements 3.7）
///
/// 突合の両辺（IEEE 754 単精度の乗除算・`ceil` と、u64 整数の切り上げ除算）はいずれも
/// 決定論であり、x64・arm64 で同一の結果を返す。集計は等値比較のみで `u32` の減算を
/// 用いないため、ラップアラウンドも起こらない。
///
/// ## 実行条件（requirements 3.6）
///
/// 実 DPI モニタ・GPU・実窓を一切使わない純粋な算術として走る。
#[test]
fn remaining_ratios_have_zero_errors() {
    let remaining: Vec<(u32, u32)> = reachable_ratios()
        .into_iter()
        .filter(|ratio| !ERROR_RATIOS.contains(ratio))
        .collect();

    assert_eq!(
        remaining.len(),
        ZERO_ERROR_RATIO_COUNT,
        "誤りゼロを期待する比の個数が変わった（到達比集合が変化した合図＝裁定の再審トリガ）: \
         実測 {} 個 / 期待 {ZERO_ERROR_RATIO_COUNT} 個・除外した比 {ERROR_RATIOS:?}・実測集合 {remaining:?}",
        remaining.len()
    );

    let mut evaluated = 0usize;
    for &(num, den) in &remaining {
        let tally = tally_extent_errors(num, den);
        assert_eq!(
            tally.errors, 0,
            "誤りゼロのはずの比で食い違いが出た（＝裁定の再審トリガ・誤差が 2 比の外へ広がった）: \
             num={num} den={den} 寸域 1..={MAX_EXTENT} 実測 {} 件 / 期待 0 件・\
             最初の食い違い（寸, 実測値, 真値）={:?}",
            tally.errors, tally.first
        );
        evaluated += tally.evaluated;
    }

    let expected_evaluated = ZERO_ERROR_RATIO_COUNT * MAX_EXTENT as usize;
    assert_eq!(
        evaluated, expected_evaluated,
        "ゼロ件集合の評価回数が期待と異なる（ループが痩せて空振りの緑になっている疑い）: \
         実測 {evaluated} 回 / 期待 {expected_evaluated} 回"
    );
}
