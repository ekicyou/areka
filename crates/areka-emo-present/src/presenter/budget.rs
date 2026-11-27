//! `FrameBudget`: 毎フレーム経路における新規確保・容量成長の**唯一の計数シーム**。
//!
//! 「定常状態で計画の新規確保が起きていない」を数値で言い切るための計数器。確保の発生点は
//! 1 つ（席の一覧と 1:1）:
//!
//! | 発生点 | 増えるフィールド | 何の確保か |
//! |---|---|---|
//! | [`AllocSite::Plan`] | `alloc_plan` | 描く計画の項の列（＝交代で合成メモの計画にもなる） |
//!
//! 確保とは別に、節の写しが作り直した項の数（`node_rebuilds`）も同じ器で数える。perf サマリ行の
//! `alloc_compose_dst` は CPU の合成先を持たないので常に 0（`judge-perf.py` の必須の欄なので欄は残す）。
//!
//! # なぜ 1 箇所で数えるのか
//!
//! プロセス全体の確保を捕まえる `#[global_allocator]` 差し替えは、プロセス全体を汚染し既存テストを
//! 不安定化しうるので採らない。代わりに、確保が起き得る発生点そのものを名前で数える。
//!
//! # 席
//!
//! 器は計数に加えて**再利用席**（計画の常設席 `DrawPlan`）を持つ。貸し出し口は [`FrameBudget::plan_scratch`]、
//! 合成メモの計画との交代は [`FrameBudget::swap_plan_scratch`]。[`FrameBudget::note_alloc`] は
//! 「席の再利用が成立せず結局確保したとき」に席メソッドの内部から呼ばれる。
//!
//! 合成メモの計画は本器が所有しない——所有はキャッシュ側にあり、追い出しエントリの
//! 容量を [`ComposeCache::take_recycled`] で回収して回す。本器は [`FrameBudget::recycled_plan`] で
//! その仲介だけを担う。
//!
//! # 回る実体の本数（キャッシュ容量 3 での形）
//!
//! キャッシュが最大 3 エントリを保持するため、計画はキャッシュの 3 本 ＋ 計画の席の 1 本 ＝ 最大 4 本が回る。
//! 追い出し 1 件が次の適用の入力になる。**本器が実体ごとの状態を覚えないのはこのためである**——
//! 高水位を器のフィールドとして持つと、高水位が役割に紐づき実体に紐づかず、4 本が入れ替わる途中で
//! 他の本の高水位を当てて成長を見逃す。立ち上がりに要する適用数は「本数ぶん」であり、「一度だけ」は
//! **確保対象ごとに一度**の意味で読む。
//!
//! # 何をもって「確保した」と数えるか（実体の容量差）
//!
//! 席そのものの `DrawPlan::items` の容量を、呼び出しの**前後で読み比べる**。
//! `Vec` の容量は再確保でしか増えず、`clear`／`truncate` で縮みもしないため、
//! 「増えた＝この呼び出しで確保が起きた」が厳密に決まる。差し替えられた実体の容量は 0 から始まるので、
//! 同じ大きさへ伸びた時点で必ず 1 件計数される（見逃し＝定常アロケーション 0 の偽の成立は起こらない）。
//!
//! 回収エントリが無い、といった境界では**確保する**が、**黙っては確保しない**——必ず該当カウンタを増やす。
//!
//! # 増分と累積を両方出す理由
//!
//! - **増分**（[`BudgetDelta`]）: 適用 1 回分。perf サマリ行の `alloc_*` フィールドへそのまま載り、
//!   判定スクリプトが行単位で「この適用で確保が起きたか」を機械集計する
//! - **累積**（[`BudgetCounters`]）: 取り出しでリセットされない run 全体の合計。テストが
//!   「ウォームアップ後の N 反復で 1 件も増えていない」を主張する読み取り口になる
//!
//! 両者は同じ申告から同時に更新されるため、適用ごとの増分の総和は必ず累積に一致する。計数は整数の
//! 飽和加算のみで、確保もログもロックも行わず、ログ設定も参照しない。
//!
//! [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled

use areka_emo_compose::DrawPlan;
use areka_emo_compose::plan::Extent;

use crate::cache::CacheEntry;

/// 項を持たない計画（席と回収の不成立の始まり）。
fn empty_plan() -> DrawPlan {
    DrawPlan {
        surface_id: 0,
        base_surface: 0,
        extent: Extent { w: 0, h: 0 },
        items: Vec::new(),
    }
}

/// 確保の発生点（perf サマリ行の `alloc_*` と 1:1）。
///
/// 列挙を増減させると perf サマリ行のフィールド集合が変わる＝判定スクリプトとの契約変更である。
///
/// **本モジュール私有**: 計数シームが 1 箇所であるには発生点の名前を外へ出せないことが条件で、
/// 外に出ていれば席を経由しない計数が書けてしまう。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AllocSite {
    /// 計画の項の列の新規確保／容量成長（交代で合成メモの計画へ出る側の確保もここで数える）。
    Plan,
}

impl AllocSite {
    /// 全発生点（走査の正本。檻はこれを回して発生点ごとの計数を確かめる）。
    ///
    /// 読み手は本モジュールのテスト（`budget_tests.rs`）のみ。presenter 経由の檻は [`BudgetCounters`]
    /// の名前つきフィールドを直接読む。増減は perf サマリ行のフィールド集合の照合が捕まえる。
    #[cfg(test)]
    const ALL: [AllocSite; 1] = [AllocSite::Plan];
}

/// 1 適用分の確保計数スナップショット（perf サマリ行の `alloc_*` フィールドの供給源）。
///
/// [`FrameBudget::take_delta`] が取り出しと同時に器側をリセットするため、この値は常に直前の取り出しから
/// 今回までの 1 適用分を表す。定常状態では全フィールドが 0 になる。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct BudgetDelta {
    /// 計画の項の列の新規確保／容量成長。
    pub(super) alloc_plan: u32,
    /// 節の写しが作り直した項の数。
    pub(super) node_rebuilds: u32,
}

impl BudgetDelta {
    /// 発生点で引く（全発生点の走査用。名前つきフィールドと同じ値を返す）。
    ///
    /// 現時点の読み手はテストのみ（[`AllocSite::ALL`] を回す檻）。製品コードの
    /// perf サマリ行は名前つきフィールドを直接使う。
    #[cfg(test)]
    fn count(&self, site: AllocSite) -> u32 {
        match site {
            AllocSite::Plan => self.alloc_plan,
        }
    }

    /// 1 件加算する（飽和。1 適用で u32 を溢れさせる経路は無いが、値を巻き戻さない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::Plan => &mut self.alloc_plan,
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
    /// 計画の項の列の累積。
    pub(super) alloc_plan: u64,
    /// 節の写しが作り直した項の数の累積。
    pub(super) node_rebuilds: u64,
}

impl BudgetCounters {
    /// 発生点で引く（全発生点の走査用）。
    ///
    /// 読み手は本モジュールのテストのみ（presenter 経由の檻は名前つきフィールドを直接読む）。
    #[cfg(test)]
    fn count(&self, site: AllocSite) -> u64 {
        match site {
            AllocSite::Plan => self.alloc_plan,
        }
    }

    /// 1 件加算する（飽和。溢れても値を巻き戻さない＝計数を捏造しない）。
    fn bump(&mut self, site: AllocSite) {
        let slot = match site {
            AllocSite::Plan => &mut self.alloc_plan,
        };
        *slot = slot.saturating_add(1);
    }
}

/// 席と「確保したかどうかの観測」を**分離不能に**束ねる私有入れ物。
///
/// # なぜ入れ物へ閉じ込めるのか（変異検査が暴いた欠陥）
///
/// 観測を席と別の場所に置くと、**席だけを差し替える改変が計数へ現れない**経路ができる。ゆえに
/// フィールドを私有にし、**席へ触れる唯一の道**を観測込みのメソッドにする。観測は実体の
/// `Vec` 容量を呼び出しの前後で読み比べる（本モジュール冒頭の「何をもって「確保した」と数えるか」節）。
mod seat {
    use areka_emo_compose::DrawPlan;

    /// 計画の常設席。**高水位を持たない**——観測は席の項の列の容量そのもので行う。
    #[derive(Debug)]
    pub(super) struct PlanSeat {
        plan: DrawPlan,
    }

    impl Default for PlanSeat {
        fn default() -> Self {
            Self {
                plan: super::empty_plan(),
            }
        }
    }

    impl PlanSeat {
        /// 席を閉包へ貸す。閉包の中で**項の列の容量が増えたら** `true`（＝確保が起きた）。
        pub(super) fn lend<R>(&mut self, borrow: impl FnOnce(&mut DrawPlan) -> R) -> (R, bool) {
            let before = self.plan.items.capacity();
            let produced = borrow(&mut self.plan);
            (produced, self.plan.items.capacity() > before)
        }

        pub(super) fn get(&self) -> &DrawPlan {
            &self.plan
        }

        /// 合成メモの計画との交代（複写も確保も起きない）。
        pub(super) fn swap(&mut self, plan: &mut DrawPlan) {
            std::mem::swap(&mut self.plan, plan);
        }
    }
}

/// 毎フレーム経路の確保計数の所有者。適用をまたいで存続し、累積を保持する。
///
/// 再利用席（計画の常設席）は本型のフィールドである。
///
/// - Preconditions: 適用対象（表示 target）ごとに 1 つ持ち、使い回す。
/// - Postconditions: [`take_delta`](FrameBudget::take_delta) は増分を返して器側を 0 に戻す。
///   [`cumulative`](FrameBudget::cumulative) は読み取りのみでリセット手段を持たない。
/// - Invariants: 適用ごとの増分の総和は累積に一致する。定常状態では
///   [`take_delta`](FrameBudget::take_delta) の全フィールドが 0 で、寸法変化時は席ごとに増えて
///   また 0 へ戻る。「一度だけ」は**確保対象ごとに一度**の意で、複数の計画が回る席（最大 4 本）
///   では代金が回る本数ぶんの適用に分かれて 1 件ずつ現れる。
#[derive(Debug, Default)]
pub(super) struct FrameBudget {
    /// 取り出し待ちの適用単位の増分。
    pending: BudgetDelta,
    /// run 全体の累積（リセットしない）。
    total: BudgetCounters,
    /// 計画の常設席（`plan_into` の出力先）。
    plan: seat::PlanSeat,
}

impl FrameBudget {
    /// 空の計数器を作る（全カウンタ 0）。
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// 新規確保・容量成長が **1 回起きた**ことを記録する（計数シームの唯一の入口）。
    ///
    /// 増分と累積を同時に進める。確保が起きなかった経路（席の再利用が成立した場合）では呼ばない——
    /// 「確保した回数」を数えることが、再利用の成立・不成立の判定材料になる条件である。確保もログも
    /// ロックも行わないため、毎フレーム経路の上で無条件に呼んでよい。
    ///
    /// # 可視性は**私有**である
    ///
    /// 正しい呼び手は本モジュールの席メソッドだけ。外から直接叩けると「シーム 1 箇所」が形骸化し、
    /// 席を通さない確保が黙って計数へ紛れ込む。テスト（`budget_tests.rs`）は子モジュールゆえ私有の
    /// まま到達できる。
    fn note_alloc(&mut self, site: AllocSite) {
        self.pending.bump(site);
        self.total.bump(site);
    }

    /// 計画の常設席を貸し出す（`plan_into` の出力先）。項の列の容量が増えたときにだけ
    /// [`AllocSite::Plan`] を計数する。
    ///
    /// # なぜ閉包で貸すのか
    ///
    /// 計画の大きさは `plan_into` の内側で決まり、呼び手も本器も**事前には知らない**ので、
    /// 伸長の観測は席を使い終えた直後にしか置けない。`&mut` を裸で返すと観測点が呼び手の記憶頼みになり、
    /// 忘れれば**黙って確保する**経路ができる。閉包で囲めば貸し出しと計数が構造的に対になる。
    pub(super) fn plan_scratch<R>(&mut self, borrow: impl FnOnce(&mut DrawPlan) -> R) -> R {
        let (produced, grew) = self.plan.lend(borrow);
        if grew {
            self.note_alloc(AllocSite::Plan);
        }
        produced
    }

    /// 計画の常設席がいま抱えている計画（読むだけ）。
    pub(super) fn planned(&self) -> &DrawPlan {
        self.plan.get()
    }

    /// 合成メモの計画との交代——計画の席と回収した計画を入れ替える（複写も確保も起きない）。
    pub(super) fn swap_plan_scratch(&mut self, plan: &mut DrawPlan) {
        self.plan.swap(plan);
    }

    /// 節の写しが作り直した項の数を足す（確保の席ではないが、同じ器で増分と累積を数える）。
    pub(super) fn note_rebuilds(&mut self, n: usize) {
        let n = u32::try_from(n).unwrap_or(u32::MAX);
        self.pending.node_rebuilds = self.pending.node_rebuilds.saturating_add(n);
        self.total.node_rebuilds = self.total.node_rebuilds.saturating_add(u64::from(n));
    }

    /// 回収した追い出しエントリから計画の容量を取り出す。
    ///
    /// `recycled` は [`ComposeCache::take_recycled`] が返す追い出しエントリで、**計画と記録の成功後にのみ**取る
    /// こと（呼び手の責務）。呼び手は返した計画を [`FrameBudget::swap_plan_scratch`] で計画の席と交代させる。
    /// 回収が成立しなかった場合（キャッシュにまだ空きがある暖機中・[`invalidate_all`] 直後）は空の計画から
    /// 始める。これは交代で計画の席へ入り、次の適用の計画で伸びる＝**そこで必ず確保が起きる**が、計数は
    /// 伸長が起きる席メソッド（[`plan_scratch`]）側が容量差として拾う。ここで先回りして数えると二重になる。
    ///
    /// [`ComposeCache::take_recycled`]: crate::cache::ComposeCache::take_recycled
    /// [`invalidate_all`]: crate::cache::ComposeCache::invalidate_all
    /// [`plan_scratch`]: FrameBudget::plan_scratch
    pub(super) fn recycled_plan(recycled: Option<CacheEntry>) -> DrawPlan {
        match recycled {
            // 回収するのは確保だけ（新しいエントリの中身は今回の計画が決める）。
            Some(CacheEntry { plan, .. }) => plan,
            None => empty_plan(),
        }
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
    /// 読み手はテストのみ（`budget_tests.rs` が席の単体で、
    /// `presenter_budget_steady_state_tests.rs` が実 `apply_show` を通した定常状態で読む）。
    #[cfg(test)]
    pub(super) fn cumulative(&self) -> &BudgetCounters {
        &self.total
    }
}

#[cfg(test)]
#[path = "budget_tests.rs"]
mod tests;
