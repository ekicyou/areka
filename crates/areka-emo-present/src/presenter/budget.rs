//! `FrameBudget`: 毎フレーム経路における新規確保・容量成長の**唯一の計数シーム**。
//!
//! 「定常状態で表示バッファの新規確保が起きていない」（Requirement 3.1・判定式⑶）を、推測でなく
//! 数値で言い切るための計数器である。確保の発生点は 4 つ（design.md §FrameBudget の席一覧と 1:1）:
//!
//! | 発生点 | 増えるフィールド | 何の確保か |
//! |---|---|---|
//! | [`AllocSite::ComposeDst`] | `alloc_compose_dst` | native 合成先 |
//! | [`AllocSite::ResampleDst`] | `alloc_resample_dst` | 表示バッファ（リサンプル先） |
//! | [`AllocSite::Xmap`] | `alloc_xmap` | リサンプル作業領域（x 軸写像表） |
//! | [`AllocSite::Mask`] | `alloc_mask` | 当たり判定マスク |
//!
//! # なぜ 1 箇所で数えるのか（design.md D6）
//!
//! プロセス全体の確保を捕まえる `#[global_allocator]` 差し替えは、emo-compose の既存予算檻が
//! **棄却済み**の方針である（`areka-emo-compose/src/golden_tests_determinism_budget_tests.rs`
//! の「アプローチ (B)」——プロセス全体を汚染し既存テストを不安定化しうる）。代わりに、
//! 確保が起き得る発生点そのものを名前で数える。数えられるのは「この経路で意図している確保」
//! だけであり、それが観測したい対象と一致する。
//!
//! # 計数の意味は段階で変わらない（増分と累積の関係）
//!
//! 現時点の器は**計数だけ**を持ち、再利用席（合成先の常設席・リサンプル作業領域の席・マスクの
//! 輪番）は後段が同じ器へ足す。ゆえに今は各発生点が「確保したとき」に [`FrameBudget::note_alloc`]
//! を呼ぶ。席が入った後は、同じシームが「再利用が成立しなかったとき」——つまり結局確保した
//! とき——に呼ばれる側へ回る。どちらの段でも計数の意味は「新規確保・容量成長が 1 回起きた」で
//! 変わらないため、増分・累積の意味論と本モジュールの API は席の導入で動かない。
//!
//! # 増分と累積を両方出す理由（Requirement 1.3）
//!
//! - **増分**（[`BudgetDelta`]）: 適用 1 回分。perf サマリ行の `alloc_*` フィールドへそのまま載り、
//!   判定スクリプトが行単位で「この適用で確保が起きたか」を機械集計する
//! - **累積**（[`BudgetCounters`]）: 取り出しでリセットされない run 全体の合計。テストが
//!   「ウォームアップ後の N 反復で 1 件も増えていない」を主張する読み取り口になる
//!
//! 両者は同じ申告から同時に更新されるため、適用ごとの増分の総和は必ず累積に一致する。
//!
//! # 毎フレーム経路の上に載る（計数自体が負荷にならない）
//!
//! 計数は整数の飽和加算のみで、確保もログもロックも行わない。ログ設定も参照しない——
//! 表示経路がログ設定で分岐しないという構造（Requirement 1.5）は計数側でも保つ。

/// 確保の発生点（design.md §FrameBudget の席一覧・perf サマリ行の `alloc_*` と 1:1）。
///
/// 列挙を増減させると perf サマリ行のフィールド集合が変わる＝判定スクリプトとの契約変更である。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AllocSite {
    /// native 合成先の新規確保／容量成長。
    ComposeDst,
    /// 表示バッファ（リサンプル先）の新規確保／容量成長（容量の回収が不成立の場合を含む）。
    ResampleDst,
    /// リサンプル作業領域（x 軸写像表）の新規確保／容量成長。
    Xmap,
    /// 当たり判定マスクの新規確保（輪番スロットの単独所有が不成立の場合を含む）。
    Mask,
}

impl AllocSite {
    /// 全発生点（走査の正本。檻はこれを回して「4 つある」ことごと固定する）。
    ///
    /// 読み手は現時点ではテストのみ（`budget_tests.rs`）。製品コードからの走査は task 6.1
    /// （定常アロケーション 0 の檻）が全発生点を回す形で入る。
    #[allow(dead_code)]
    pub(super) const ALL: [AllocSite; 4] = [
        AllocSite::ComposeDst,
        AllocSite::ResampleDst,
        AllocSite::Xmap,
        AllocSite::Mask,
    ];
}

/// 1 適用分の確保計数スナップショット（perf サマリ行の `alloc_*` フィールドの供給源）。
///
/// [`FrameBudget::take_delta`] が取り出しと同時に器側をリセットするため、この値は常に
/// 「直前の取り出しから今回までの 1 適用分」を表す。定常状態では全フィールドが 0 になる
/// ことが是正後の不変量である（Requirement 3.1）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BudgetDelta {
    /// native 合成先の新規確保／容量成長。
    pub(super) alloc_compose_dst: u32,
    /// 表示バッファの新規確保／容量成長。
    pub(super) alloc_resample_dst: u32,
    /// リサンプル作業領域の新規確保／容量成長。
    pub(super) alloc_xmap: u32,
    /// 当たり判定マスクの新規確保。
    pub(super) alloc_mask: u32,
}

impl BudgetDelta {
    /// 発生点で引く（全発生点の走査用。名前つきフィールドと同じ値を返す）。
    ///
    /// 現時点の読み手はテストのみ（[`AllocSite::ALL`] を回す檻）。製品コードの
    /// perf サマリ行は名前つきフィールドを直接使う。
    #[allow(dead_code)]
    pub(super) fn count(&self, site: AllocSite) -> u32 {
        match site {
            AllocSite::ComposeDst => self.alloc_compose_dst,
            AllocSite::ResampleDst => self.alloc_resample_dst,
            AllocSite::Xmap => self.alloc_xmap,
            AllocSite::Mask => self.alloc_mask,
        }
    }

    /// 1 件加算する（飽和。1 適用で u32 を溢れさせる経路は無いが、値を巻き戻さない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::ComposeDst => &mut self.alloc_compose_dst,
            AllocSite::ResampleDst => &mut self.alloc_resample_dst,
            AllocSite::Xmap => &mut self.alloc_xmap,
            AllocSite::Mask => &mut self.alloc_mask,
        };
        *slot = slot.saturating_add(1);
    }
}

/// run 全体の累積計数（取り出しでリセットされない）。
///
/// 幅が [`BudgetDelta`]（u32）より広い u64 なのは、長時間走行（20 分超）の合計を素直に載せる
/// ためである。増分は 1 適用分ゆえ u32 で溢れない。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BudgetCounters {
    /// native 合成先の累積。
    pub(super) alloc_compose_dst: u64,
    /// 表示バッファの累積。
    pub(super) alloc_resample_dst: u64,
    /// リサンプル作業領域の累積。
    pub(super) alloc_xmap: u64,
    /// 当たり判定マスクの累積。
    pub(super) alloc_mask: u64,
}

impl BudgetCounters {
    /// 発生点で引く（全発生点の走査用）。
    ///
    /// 現時点の読み手はテストのみ。累積を「全発生点で 0 のまま」と主張する檻は task 6.1 が置く。
    #[allow(dead_code)]
    pub(super) fn count(&self, site: AllocSite) -> u64 {
        match site {
            AllocSite::ComposeDst => self.alloc_compose_dst,
            AllocSite::ResampleDst => self.alloc_resample_dst,
            AllocSite::Xmap => self.alloc_xmap,
            AllocSite::Mask => self.alloc_mask,
        }
    }

    /// 1 件加算する（飽和。溢れても値を巻き戻さない＝計数を捏造しない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::ComposeDst => &mut self.alloc_compose_dst,
            AllocSite::ResampleDst => &mut self.alloc_resample_dst,
            AllocSite::Xmap => &mut self.alloc_xmap,
            AllocSite::Mask => &mut self.alloc_mask,
        };
        *slot = slot.saturating_add(1);
    }
}

/// 毎フレーム経路の確保計数の所有者。適用をまたいで存続し、累積を保持する。
///
/// - Preconditions: 適用対象（表示 target）ごとに 1 つ持ち、適用のたびに使い回す。
/// - Postconditions: [`take_delta`](FrameBudget::take_delta) は増分を返して器側を 0 に戻す。
///   [`cumulative`](FrameBudget::cumulative) は読み取りのみでリセット手段を持たない。
/// - Invariants: 適用ごとの増分の総和は累積に一致する。是正後の定常状態では
///   [`take_delta`](FrameBudget::take_delta) の全フィールドが 0（Requirement 3.1）であり、
///   寸法変化時は一度だけ増えてまた 0 へ戻る（Requirement 3.2）。
///
/// 後段は本型へ再利用席（合成先の常設席・リサンプル作業領域の席・マスクの輪番）を**フィールドと
/// して足す**。計数の API 面はそのとき変わらない。
#[derive(Debug, Default)]
pub(super) struct FrameBudget {
    /// 取り出し待ちの適用単位の増分。
    pending: BudgetDelta,
    /// run 全体の累積（リセットしない）。
    total: BudgetCounters,
}

impl FrameBudget {
    /// 空の計数器を作る（全カウンタ 0）。
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 新規確保・容量成長が **1 回起きた**ことを記録する（計数シームの唯一の入口・D6）。
    ///
    /// 増分と累積を同時に進める。確保が起きなかった経路（席の再利用が成立した場合）では
    /// 呼ばない——「呼ばれた回数」ではなく「確保した回数」を数えることが、再利用の成立・
    /// 不成立を判定材料にする条件である。
    ///
    /// 確保もログもロックも行わないため、毎フレーム経路の上で無条件に呼んでよい。
    pub(super) fn note_alloc(&mut self, site: AllocSite) {
        self.pending.bump(site);
        self.total.bump(site);
    }

    /// この適用分の増分を取り出してリセットする（perf サマリ行へ載せる値）。
    ///
    /// 累積には触れない。取り出しは**表示成立点 1 箇所**（`show.rs` の perf サマリ行 emit の
    /// 直前）で行う。早期復帰した適用の増分は取り出されず次の適用へ持ち越されるが、確保自体は
    /// 実際に起きているため、成立した次の行に現れるのが正しい（累積は常に厳密）。
    pub(super) fn take_delta(&mut self) -> BudgetDelta {
        std::mem::take(&mut self.pending)
    }

    /// run 全体の累積カウンタ（読み取りのみ）。
    ///
    /// テストが「ウォームアップ後の N 反復で 1 件も増えていない」を主張する口である。
    ///
    /// 現時点の読み手はテストのみ（`budget_tests.rs`）。presenter 経由で累積を主張する檻は
    /// task 6.1（定常アロケーション 0）が置く。
    #[allow(dead_code)]
    pub(super) fn cumulative(&self) -> &BudgetCounters {
        &self.total
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
