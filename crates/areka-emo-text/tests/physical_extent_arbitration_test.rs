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
