//! timeline: 抽選境界・1/N 判定・経過時刻→現在コマの決定論純関数コア（要件 2.1/2.4/4.1/4.3/4.4/4.5/7.1/9.4）。
//!
//! 全関数が純粋（引数のみに依存・I/O なし・スレッド／GPU／実時間非依存）。時刻と乱数は注入シーム
//! （`LoopRng`・`u64` 経過時刻）で受け取り、実 entropy 源・実クロックを評価経路から排除する（7.1）。
//! これにより注入 tick＋注入乱数で全分岐を決定論的にテストできる（[deterministic-test-coverage-mandate]）。
//!
//! # 構成
//!
//! - [`seeded_rng`]／[`should_fire`] — SplitMix64 の純粋 PRNG と 1/N 抽選（2.1/2.4）。
//! - [`LotteryBoundary`] — 1000ms 絶対グリッドの跨ぎ検出（毎秒抽選の写像・catch-up 1 回）。
//!   ghost [`crate`] 外の `BoundarySchedule` と同じ「1 回だけ発火・次境界へスナップ」政策。
//! - [`frame_at`]／[`FrameStatus`] — 再生開始からの経過 `elapsed_ms` に対する現在コマ判定
//!   （累積 wait デッドライン列・`-1` 停止・末尾残留のデファクト 2 点を焼き込む・4.1–4.4/9.4）。

use crate::table::LoopFrame;

/// 乱数注入シーム（クロック注入と同型・7.1）。`[0, bound)` の一様整数を返す。
pub type LoopRng = Box<dyn FnMut(u32) -> u32 + Send>;

/// 決定論 PRNG（SplitMix64）。テストでも本番でも使える純粋構築子（同一シード→同一系列）。
///
/// SplitMix64 の状態前進 → 乗算シフト縮約 `(next_u64 as u128 * bound as u128) >> 64` で `[0, bound)`
/// へ写す（除算不使用ゆえ `bound == 0` でも panic せず 0 を返す＝defensive）。呼び手は通常 `bound >= 1`
/// （表構築ガードで `k >= 1`）を保証するが、境界防御として 0 も安全に扱う。
pub fn seeded_rng(seed: u64) -> LoopRng {
    let mut state = seed;
    Box::new(move |bound: u32| -> u32 {
        // SplitMix64: 状態を黄金比定数で前進し、雪崩ミックスして次の 64bit 値を得る。
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        // 乗算シフト縮約（Lemire）: 除算・剰余なしで [0, bound) の一様整数へ。bound==0 は積 0 → 0。
        ((u128::from(z) * u128::from(bound)) >> 64) as u32
    })
}

/// 1/N 抽選（2.1/2.4）。`rng(k) == 0` で発火（確率 1/k）。呼び手は `k >= 1` を保証（表構築ガード）。
pub fn should_fire(k: u32, rng: &mut LoopRng) -> bool {
    rng(k) == 0
}

/// 1000ms 絶対グリッド境界（毎秒抽選の写像・catch-up 1 回）。
pub struct LotteryBoundary {
    next_boundary_ms: u64,
}

/// 1 秒（1000ms）グリッド。ghost `BoundarySchedule` と同じ絶対グリッド整列（毎秒抽選の写像）。
const LOTTERY_GRID_MS: u64 = 1000;

/// `LOTTERY_GRID_MS` の倍数のうち `now_ms` より厳密に大きい最小値（例: 0→1000・1000→2000・1500→2000）。
fn next_boundary_strictly_after(now_ms: u64) -> u64 {
    (now_ms / LOTTERY_GRID_MS + 1) * LOTTERY_GRID_MS
}

impl LotteryBoundary {
    /// `now_ms` より厳密に未来の次境界（1000 の倍数）から開始する。
    ///
    /// `now_ms` がちょうど境界上でも次の境界へ進める（起動直後の即時発火を避け、常に「次はまだ来て
    /// いない未来の境界」を保つ不変条件・ghost `BoundarySchedule::starting_at` と同一政策）。
    pub fn starting_at(now_ms: u64) -> Self {
        LotteryBoundary {
            next_boundary_ms: next_boundary_strictly_after(now_ms),
        }
    }

    /// `now_ms` が次境界へ到達済みなら **1 回だけ** `true`（複数跨ぎでも 1 回＝catch-up）＋次境界へスナップ。
    ///
    /// 到達済みなら次デッドラインを `now_ms` より厳密に未来のグリッド倍数へスナップするため、複数境界を
    /// 跨いでいてもバーストで打ち返さない（catch-up＝抽選 1 回・ghost `BoundarySchedule::poll` と同一政策）。
    pub fn poll(&mut self, now_ms: u64) -> bool {
        if now_ms < self.next_boundary_ms {
            return false;
        }
        self.next_boundary_ms = next_boundary_strictly_after(now_ms);
        true
    }
}

/// 経過時刻に対する現在コマ判定（4.1–4.4・9.4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameStatus {
    /// 先頭コマのデッドライン未到達（まだ何も出さない）。
    Pending,
    /// `frames[i]` が現在コマ（4.2 現在コマ 1 枚）。
    Active(usize),
    /// 現在コマの surface_id が負（`-1` 等）→ 停止・ベース復帰（4.3）。
    Stopped,
    /// `-1` なし末尾到達 → `frames[i]`（最終コマ）残留・終了状態（4.4/9.4）。
    FinishedResidual(usize),
}

/// 経過時刻 `elapsed_ms` に対する現在コマを判定する（累積 wait デッドライン列・4.1–4.4/9.4）。
///
/// コマ k のデッドライン `t_k = Σ_{j<=k} wait_ms[j]`（`wait_k` は前コマからコマ k へ切り替わる遅延）。
/// 現在 index は `t_k <= elapsed_ms` を満たす最大の k。誰も満たさない（`elapsed < t_0`）なら
/// [`FrameStatus::Pending`]。現在コマの `surface_id < 0` なら [`FrameStatus::Stopped`]（method/x/y 無視・
/// 4.3。`-1` と他負値の区別＋warn! は LoopRuntime 責務）。末尾かつ非負なら [`FrameStatus::FinishedResidual`]
/// （末尾残留・恒久・4.4/9.4）、それ以外は [`FrameStatus::Active`]。デッドラインは単調非減少ゆえ、超過時点で
/// 走査を打ち切れる（`wait == 0` 連鎖は同一デッドラインを共有し末尾 index が現在コマになる）。
pub fn frame_at(frames: &[LoopFrame], elapsed_ms: u64) -> FrameStatus {
    let mut acc: u64 = 0;
    let mut current: Option<usize> = None;
    for (i, f) in frames.iter().enumerate() {
        acc = acc.saturating_add(u64::from(f.wait_ms));
        if acc <= elapsed_ms {
            current = Some(i);
        } else {
            // デッドラインは以降も非減少ゆえ、超過した時点で以降のコマも未到達。
            break;
        }
    }

    match current {
        None => FrameStatus::Pending,
        Some(i) => {
            if frames[i].surface_id < 0 {
                FrameStatus::Stopped
            } else if i == frames.len() - 1 {
                FrameStatus::FinishedResidual(i)
            } else {
                FrameStatus::Active(i)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_compose::ComposeMethod;

    /// テスト用 1 コマ（method/x/y は frame_at では無視されるため既定 Overlay/0）。
    fn frame(surface_id: i64, wait_ms: u32) -> LoopFrame {
        LoopFrame {
            surface_id,
            method: ComposeMethod::Overlay,
            wait_ms,
            x: 0,
            y: 0,
        }
    }

    // ── seeded_rng / should_fire ─────────────────────────────────────────────

    /// 固定シードは再現系列を返す（同一シード→同一列・純粋 PRNG）。
    #[test]
    fn seeded_rng_is_reproducible_for_fixed_seed() {
        let mut a = seeded_rng(0xDEAD_BEEF);
        let mut b = seeded_rng(0xDEAD_BEEF);
        let seq_a: Vec<u32> = (0..8).map(|_| a(1000)).collect();
        let seq_b: Vec<u32> = (0..8).map(|_| b(1000)).collect();
        assert_eq!(seq_a, seq_b, "同一シードは同一系列（決定論）");
    }

    /// 異なるシードは（一般に）異なる系列を返す。
    #[test]
    fn seeded_rng_differs_across_seeds() {
        let mut a = seeded_rng(1);
        let mut b = seeded_rng(2);
        let seq_a: Vec<u32> = (0..8).map(|_| a(1000)).collect();
        let seq_b: Vec<u32> = (0..8).map(|_| b(1000)).collect();
        assert_ne!(seq_a, seq_b, "別シードは別系列");
    }

    /// 出力は常に `[0, bound)` に収まる（乗算シフト縮約）。
    #[test]
    fn seeded_rng_stays_in_range() {
        let mut r = seeded_rng(42);
        for _ in 0..10_000 {
            let v = r(7);
            assert!(v < 7, "出力は [0,7) に収まる: {v}");
        }
    }

    /// `bound == 0` は panic せず 0 を返す（乗算シフトは除算不使用ゆえ自然に 0）。
    #[test]
    fn seeded_rng_bound_zero_is_defensive_no_panic() {
        let mut r = seeded_rng(123);
        for _ in 0..100 {
            assert_eq!(r(0), 0, "bound==0 は defensive に 0");
        }
    }

    /// `k == 1` は常に発火（`rng(1)` は常に 0 ＝ `[0,1)` は 0 のみ）。
    #[test]
    fn should_fire_k1_always_fires() {
        let mut r = seeded_rng(7);
        for _ in 0..1000 {
            assert!(should_fire(1, &mut r), "k==1 は毎回発火");
        }
    }

    /// `should_fire` は `rng(k) == 0` のときだけ発火する（fire 条件の檻）。
    #[test]
    fn should_fire_iff_rng_returns_zero() {
        // rng を注入して 0/非0 を交互に返させ、発火が rng==0 と厳密一致することを示す。
        let mut toggle = 0u32;
        let mut rng: LoopRng = Box::new(move |_bound| {
            let v = toggle;
            toggle = if toggle == 0 { 1 } else { 0 };
            v
        });
        assert!(should_fire(4, &mut rng), "rng==0 → 発火");
        assert!(!should_fire(4, &mut rng), "rng==1 → 非発火");
        assert!(should_fire(4, &mut rng), "rng==0 → 発火");
        assert!(!should_fire(4, &mut rng), "rng==1 → 非発火");
    }

    /// 大量試行での 1/N 頻度が概ね期待どおり（決定論シードで固定・分布の健全性確認）。
    #[test]
    fn should_fire_frequency_is_approximately_one_over_k() {
        let mut r = seeded_rng(0x1234_5678);
        let trials = 100_000;
        let k = 4;
        let fires = (0..trials).filter(|_| should_fire(k, &mut r)).count();
        // 1/4 ≈ 25000。決定論シードなので固定だが、広めの許容で分布健全性のみ確認。
        assert!(
            (20_000..30_000).contains(&fires),
            "1/4 抽選の発火数は概ね 25000 付近: {fires}"
        );
    }

    // ── LotteryBoundary ──────────────────────────────────────────────────────

    /// `starting_at` は now より厳密に未来の次境界（1000 の倍数）を起点にする。
    #[test]
    fn starting_at_sets_strictly_future_boundary() {
        // 非境界起点。
        let mut b = LotteryBoundary::starting_at(2347);
        assert!(!b.poll(2999), "3000 未満は非跨ぎ");
        assert!(b.poll(3000), "3000 到達で跨ぎ");

        // ちょうど境界上でも次の境界へ進む（即時発火を避ける）。
        let mut b = LotteryBoundary::starting_at(1000);
        assert!(!b.poll(1000), "起点ちょうど境界上は即時発火しない（次境界は 2000）");
        assert!(!b.poll(1999), "2000 未満は非跨ぎ");
        assert!(b.poll(2000), "2000 到達で跨ぎ");
    }

    /// 非跨ぎは `false`・境界ちょうどで `true`・跨いだら次境界へスナップ。
    #[test]
    fn poll_fires_once_on_boundary_and_snaps() {
        let mut b = LotteryBoundary::starting_at(0);
        // 起点 0 の次境界は 1000。
        assert!(!b.poll(0), "0 は非跨ぎ");
        assert!(!b.poll(999), "999 は非跨ぎ");
        assert!(b.poll(1000), "1000 ちょうどで発火");
        // スナップ後は次境界 2000 まで非跨ぎ。
        assert!(!b.poll(1000), "同一時刻の再 poll は再発火しない（スナップ済み）");
        assert!(!b.poll(1999), "1999 は非跨ぎ");
        assert!(b.poll(2000), "2000 で発火");
    }

    /// 複数境界を一気に跨いでも発火は **1 回**（catch-up）＋ now より厳密未来へスナップ。
    #[test]
    fn poll_collapses_multiple_missed_boundaries_into_one() {
        let mut b = LotteryBoundary::starting_at(0);
        // 0 → 5500 へ一気に進む＝1000,2000,3000,4000,5000 の 5 境界を跨ぐが 1 回だけ true。
        assert!(b.poll(5500), "複数跨ぎでも 1 回だけ発火（catch-up）");
        // 次境界は 5500 より厳密未来の 6000 へスナップ済み＝中間境界は再送されない。
        assert!(!b.poll(5500), "同一時刻の再 poll は発火しない");
        assert!(!b.poll(5999), "5999 は非跨ぎ");
        assert!(b.poll(6000), "6000 で次の発火");
    }

    /// catch-up 後の以降の poll は再び定刻扱いに戻る（1 秒刻みで 1 回ずつ）。
    #[test]
    fn poll_returns_to_regular_cadence_after_catchup() {
        let mut b = LotteryBoundary::starting_at(0);
        assert!(b.poll(3200), "3200 で catch-up 発火（次境界 4000）");
        assert!(b.poll(4000), "4000 で定刻発火");
        assert!(b.poll(5000), "5000 で定刻発火");
        assert!(!b.poll(5500), "5500 は非跨ぎ");
    }

    // ── frame_at: 基本経路 ───────────────────────────────────────────────────

    /// 先頭 wait > 0 のとき、デッドライン未到達は `Pending`（まだ何も出さない）。
    #[test]
    fn frame_at_pending_before_first_deadline() {
        let frames = vec![frame(100, 40), frame(101, 40)];
        // t = [40, 80]。0..39 は Pending。
        assert_eq!(frame_at(&frames, 0), FrameStatus::Pending);
        assert_eq!(frame_at(&frames, 39), FrameStatus::Pending);
        // 40 ちょうどで先頭コマへ。
        assert_eq!(frame_at(&frames, 40), FrameStatus::Active(0));
    }

    /// 先頭 wait == 0 のときは elapsed 0 で即先頭コマ（Pending を経ない）。
    #[test]
    fn frame_at_leading_zero_wait_shows_frame0_immediately() {
        let frames = vec![frame(100, 0), frame(101, 40), frame(102, 40)];
        // t = [0, 40, 80]。
        assert_eq!(frame_at(&frames, 0), FrameStatus::Active(0));
        assert_eq!(frame_at(&frames, 39), FrameStatus::Active(0));
        assert_eq!(frame_at(&frames, 40), FrameStatus::Active(1));
        assert_eq!(frame_at(&frames, 79), FrameStatus::Active(1));
        // 末尾（非負）＝FinishedResidual。
        assert_eq!(frame_at(&frames, 80), FrameStatus::FinishedResidual(2));
    }

    /// 累積 wait デッドラインの境界値（t_k ちょうど／t_k−1／t_k+1）を全て檻に入れる。
    #[test]
    fn frame_at_cumulative_deadline_boundary_values() {
        // waits 10/20/30 → t = [10, 30, 60]。全て非負・末尾は FinishedResidual。
        let frames = vec![frame(1, 10), frame(2, 20), frame(3, 30)];

        // t_0 = 10 前後。
        assert_eq!(frame_at(&frames, 9), FrameStatus::Pending, "t_0−1 は Pending");
        assert_eq!(frame_at(&frames, 10), FrameStatus::Active(0), "t_0 ちょうどで Active(0)");
        assert_eq!(frame_at(&frames, 11), FrameStatus::Active(0), "t_0+1 は Active(0)");

        // t_1 = 30 前後。
        assert_eq!(frame_at(&frames, 29), FrameStatus::Active(0), "t_1−1 は Active(0)");
        assert_eq!(frame_at(&frames, 30), FrameStatus::Active(1), "t_1 ちょうどで Active(1)");
        assert_eq!(frame_at(&frames, 31), FrameStatus::Active(1), "t_1+1 は Active(1)");

        // t_2 = 60 前後（末尾＝FinishedResidual）。
        assert_eq!(frame_at(&frames, 59), FrameStatus::Active(1), "t_2−1 は Active(1)");
        assert_eq!(
            frame_at(&frames, 60),
            FrameStatus::FinishedResidual(2),
            "t_2 ちょうどで末尾残留"
        );
        assert_eq!(
            frame_at(&frames, 61),
            FrameStatus::FinishedResidual(2),
            "t_2+1 も末尾残留"
        );
    }

    // ── frame_at: 終端意味論（Stopped / FinishedResidual 恒久） ──────────────────

    /// `-1` 末尾コマ到達 → `Stopped`（method/x/y 無視・4.3）＝ kero 実測列 0/40/80。
    #[test]
    fn frame_at_kero_real_sequence_stops_on_negative_tail() {
        // emo2 kero 実測: frames 2106(wait0)/2110(wait40)/-1(wait80)。t = [0, 40, 120]。
        let frames = vec![frame(2106, 0), frame(2110, 40), frame(-1, 80)];
        assert_eq!(frame_at(&frames, 0), FrameStatus::Active(0), "0 で 2106");
        assert_eq!(frame_at(&frames, 39), FrameStatus::Active(0));
        assert_eq!(frame_at(&frames, 40), FrameStatus::Active(1), "40 で 2110");
        assert_eq!(frame_at(&frames, 119), FrameStatus::Active(1));
        // t_2 = 120: -1 コマ → Stopped。
        assert_eq!(frame_at(&frames, 120), FrameStatus::Stopped, "120 で -1 → 停止");
        // 恒久: さらに時間が進んでも Stopped のまま。
        assert_eq!(frame_at(&frames, 100_000), FrameStatus::Stopped, "-1 停止は恒久");
    }

    /// `-1` なし末尾 → `FinishedResidual(last)` 恒久（4.4/9.4）＝ sakura 実測列 0/150/22。
    #[test]
    fn frame_at_sakura_real_sequence_finishes_residual_permanently() {
        // emo2 sakura 実測: frames 1412(wait0)/1411(wait150)/1410(wait22)。t = [0, 150, 172]。全て非負。
        let frames = vec![frame(1412, 0), frame(1411, 150), frame(1410, 22)];
        assert_eq!(frame_at(&frames, 0), FrameStatus::Active(0), "0 で 1412");
        assert_eq!(frame_at(&frames, 149), FrameStatus::Active(0));
        assert_eq!(frame_at(&frames, 150), FrameStatus::Active(1), "150 で 1411");
        assert_eq!(frame_at(&frames, 171), FrameStatus::Active(1));
        // t_2 = 172: 末尾（非負）→ FinishedResidual(2)。
        assert_eq!(
            frame_at(&frames, 172),
            FrameStatus::FinishedResidual(2),
            "172 で最終コマ 1410 残留"
        );
        // 恒久: 任意に大きな経過でも FinishedResidual(2) のまま（残留・9.4）。
        assert_eq!(
            frame_at(&frames, 172),
            FrameStatus::FinishedResidual(2),
            "残留は恒久"
        );
        assert_eq!(
            frame_at(&frames, u64::MAX),
            FrameStatus::FinishedResidual(2),
            "u64::MAX 経過でも末尾残留は不変（恒久）"
        );
    }

    /// 単一コマ・非負 → 到達後は即 `FinishedResidual(0)` 恒久。
    #[test]
    fn frame_at_single_nonneg_frame_is_finished_residual() {
        // wait0 単一コマ。t = [0]。
        let frames = vec![frame(500, 0)];
        assert_eq!(frame_at(&frames, 0), FrameStatus::FinishedResidual(0));
        assert_eq!(frame_at(&frames, u64::MAX), FrameStatus::FinishedResidual(0), "恒久");

        // 先頭 wait > 0 の単一コマ: 到達前は Pending、到達後は残留。
        let frames = vec![frame(500, 30)];
        assert_eq!(frame_at(&frames, 29), FrameStatus::Pending);
        assert_eq!(frame_at(&frames, 30), FrameStatus::FinishedResidual(0));
        assert_eq!(frame_at(&frames, 1_000_000), FrameStatus::FinishedResidual(0));
    }

    /// 単一コマ・`-1` → 到達後は即 `Stopped` 恒久。
    #[test]
    fn frame_at_single_negative_frame_is_stopped() {
        let frames = vec![frame(-1, 0)];
        assert_eq!(frame_at(&frames, 0), FrameStatus::Stopped);
        assert_eq!(frame_at(&frames, u64::MAX), FrameStatus::Stopped, "恒久");
    }

    /// `-1` 以外の負値も `Stopped`（負値一般で停止・-1 と他負値の区別＋warn! は LoopRuntime 責務）。
    #[test]
    fn frame_at_any_negative_surface_id_is_stopped() {
        let frames = vec![frame(100, 0), frame(-2, 40), frame(101, 40)];
        // t = [0, 40, 80]。中間コマが -2。
        assert_eq!(frame_at(&frames, 0), FrameStatus::Active(0));
        assert_eq!(frame_at(&frames, 40), FrameStatus::Stopped, "-2 も負値ゆえ Stopped");
        // -2 コマで停止（frame_at は現在コマのみ見るので、その先の 80 でも 101 が現在コマ）。
        assert_eq!(frame_at(&frames, 80), FrameStatus::FinishedResidual(2), "80 で末尾 101 残留");
    }

    /// wait 0 連鎖: 全 wait 0 のとき elapsed 0 で末尾（最大 index）が現在コマ。
    #[test]
    fn frame_at_all_zero_waits_selects_last_index() {
        let frames = vec![frame(1, 0), frame(2, 0), frame(3, 0)];
        // t = [0, 0, 0]。elapsed 0 で t_k <= 0 を満たす最大 k = 2（末尾）。
        assert_eq!(frame_at(&frames, 0), FrameStatus::FinishedResidual(2), "全 0 wait は末尾が現在コマ");
        // -1 が末尾に含まれる wait 0 連鎖 → Stopped。
        let frames = vec![frame(1, 0), frame(2, 0), frame(-1, 0)];
        assert_eq!(frame_at(&frames, 0), FrameStatus::Stopped, "末尾 -1 → 停止");
    }

    /// 疎 index 由来（table.rs が 0/1/3 を密 Vec へ正規化）でも位置ベースで一貫評価。
    #[test]
    fn frame_at_sparse_derived_dense_vec() {
        // 疎 pattern index 0/1/3 は table.rs で密 Vec[3] に正規化済み。frame_at は位置で評価。
        // waits 0/40/80（kero 相当）。t = [0, 40, 120]。末尾 -1。
        let frames = vec![frame(2106, 0), frame(2110, 40), frame(-1, 80)];
        assert_eq!(frame_at(&frames, 0), FrameStatus::Active(0));
        assert_eq!(frame_at(&frames, 40), FrameStatus::Active(1));
        assert_eq!(frame_at(&frames, 120), FrameStatus::Stopped);
    }

    /// `FrameStatus` は `Send`（純関数の返値がスレッド越え搬送に耐える）。
    #[test]
    fn frame_status_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<FrameStatus>();
    }
}
