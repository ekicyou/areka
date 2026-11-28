//! `FrameBudget`（budget.rs）の単体檻。
//! 固定するのは確保計数シームの意味論: 申告されたときだけ増える・発生点とフィールドが 1:1・
//! 増分は `take_delta` でリセットされ累積はされない。増分は perf サマリ行の `alloc_*` へ載り、
//! 累積はテストが「定常状態で新規確保が起きていない」を主張する読み取り口になる。
//!
//! 発生点は計画の 1 つ。席について次の 3 点を固定する:
//!
//! - 定常状態は完全ゼロ
//! - 大きさの変化は実体ごとに一度だけ計数され、以後また 0
//! - 回収不成立でも確保は必ず計数に現れる（黙って確保しない）
//!
//! 回る実体は 4 本。キャッシュ容量 3 に対し [`Flow2`] が 4 種類のキーを巡回させ、毎適用ミス＋追い出し
//! 1 件（本番の「毎コマ引き当て外れ」と同じ形）にする。計画の席と合成メモの計画の交代
//! （[`FrameBudget::swap_plan_scratch`]）は無条件に走り、計画はキャッシュ 3 本＋席 1 本が回る。
//! 暖機に要する適用数がこの本数で、[`WARMUP`] が対応する。
//!
//! 同じ大きさの反復だけでは再利用を主張できない（毎回まっさらに確保する実装も毎回同じ容量に着地して
//! 素通りする）。再利用の主張は「より小さい後続要求で容量が縮まない」か「ポインタ同一性」の
//! どちらかで書く。実時間は合否に使わない（純 x64 の常設テスト）。

use super::*;

use areka_emo_compose::{BindSet, PatternState, PlanItem};

use crate::cache::ComposeCache;

/// 増分は取り出しでリセットされ、累積はリセットされない。
///
/// # 殺す誤実装
///
/// - `take_delta` がリセットしない → 適用 2 の増分に適用 1 の値が残り赤
/// - `take_delta` が累積まで巻き戻す → 適用 2 の累積 assert で赤
/// - 増分と累積が同一の器を共有している → どちらかの assert で必ず赤
#[test]
fn take_delta_resets_the_increment_while_the_cumulative_keeps_growing() {
    let mut budget = FrameBudget::new();

    budget.note_alloc(AllocSite::Plan);
    budget.note_alloc(AllocSite::Plan);
    budget.note_rebuilds(1);
    assert_eq!(
        budget.take_delta(),
        BudgetDelta {
            alloc_plan: 2,
            node_rebuilds: 1,
        },
        "同一発生点の複数回申告は足し合わされること"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_plan: 2,
            node_rebuilds: 1,
        },
        "累積は増分と同じ値から始まる"
    );

    assert_eq!(
        budget.take_delta(),
        BudgetDelta::default(),
        "取り出し済みの増分が次の適用へ持ち越されている（リセット漏れ）"
    );
    assert_eq!(
        budget.cumulative().alloc_plan,
        2,
        "累積は増分の取り出しでリセットされないこと"
    );

    budget.note_alloc(AllocSite::Plan);
    assert_eq!(budget.take_delta().alloc_plan, 1, "適用 3 の増分");
    assert_eq!(
        budget.cumulative().alloc_plan,
        3,
        "累積は run 全体で積み上がること"
    );
}

/// 増分と累積の整合: 適用ごとに取り出した増分の総和が、累積と一致する。
///
/// 片方だけを数える実装を殺す。判定スクリプトは増分を、テストは累積を根拠にするため、両者が
/// 食い違うと静かな誤診断を生む。
#[test]
fn the_sum_of_per_apply_deltas_equals_the_cumulative() {
    let mut budget = FrameBudget::new();
    let program: [&[AllocSite]; 4] = [
        &[AllocSite::Plan],
        &[],
        &[AllocSite::Plan, AllocSite::Plan],
        &[AllocSite::Plan],
    ];

    let mut summed = [0_u64; 1];
    for apply in program {
        for site in apply {
            budget.note_alloc(*site);
        }
        let delta = budget.take_delta();
        for site in AllocSite::ALL {
            summed[site as usize] += u64::from(delta.count(site));
        }
    }

    for site in AllocSite::ALL {
        assert_eq!(
            budget.cumulative().count(site),
            summed[site as usize],
            "{site:?}: 増分の総和と累積が食い違っている"
        );
    }
    // 台本の実数（申告回数）。総和どうしの空虚な一致を防ぐ。
    assert_eq!(budget.cumulative().count(AllocSite::Plan), 4);
}

// ═══════════════════════════════════════════════════════════════════════════
// 席の再利用
// ═══════════════════════════════════════════════════════════════════════════

/// 1 適用で観測した値（計数の増分・回る実体の番地）。
///
/// ポインタは**同一性の主張にのみ**使う（読み出しはしない）。再利用が成立していれば同じ
/// 実体の集合が返り続け、毎回まっさらに確保する実装では実体が入れ替わる。
struct Applied {
    delta: BudgetDelta,
    /// 交代**後**の計画の席（＝次の適用が計画を組む器）。
    seat: *const PlanItem,
    /// この適用でキャッシュへ挿した計画。
    inserted: *const PlanItem,
}

/// apply_show のミス経路を席の側から演じる装置。
///
/// `show.rs` の順序を手で踏む: 計画（常設席）→ 容量回収（計画の成功後に限る）→ 交代 → 挿入。
///
/// 同じキーの繰り返しではミス経路にならないので、surface id を 4 種類巡回させる
/// （巡回長 4 > 容量 3 で毎適用ミス＋追い出し 1 件）。
struct Flow2 {
    budget: FrameBudget,
    cache: ComposeCache,
    /// 適用回数（巡回するキーの選択に使う）。
    applies: usize,
}

/// 巡回するキーの本数。**キャッシュ容量より大きい**こと自体が必須である（毎適用ミスの条件）。
const ROTATING_KEYS: usize = 4;

/// 暖機に要する適用数＝回る実体の本数（キャッシュの 3 本＋席 1 本が 1 適用ずつずれて伸びる）。
const WARMUP: usize = ROTATING_KEYS;

impl Flow2 {
    fn new() -> Self {
        Self {
            budget: FrameBudget::new(),
            cache: ComposeCache::new(),
            applies: 0,
        }
    }

    /// ミス経路を 1 適用ぶん演じ（計画の項を `items` 個にする）、この適用の増分と実体を返す。
    fn apply(&mut self, items: usize) -> Applied {
        // 巡回長 4 > 容量 3 ゆえ、この id は 1 手前で追い出されている＝必ずミスする。
        let surface_id = 1000 + (self.applies % ROTATING_KEYS) as u32;
        self.applies += 1;
        assert!(
            !self
                .cache
                .touch(surface_id, &BindSet::default(), &PatternState::default()),
            "前提: 本装置はミス経路を演じる（巡回長 {ROTATING_KEYS} > 容量ゆえ必ずミスする）"
        );
        self.budget.plan_scratch(|plan| {
            plan.items.clear();
            plan.items
                .extend(std::iter::repeat_n(PlanItem::GroupEnd, items));
        });

        // 容量回収は計画の成功後にのみ。
        let recycled = self.cache.take_recycled();
        let mut plan = FrameBudget::recycled_plan(recycled);
        self.budget.swap_plan_scratch(&mut plan);
        let inserted = plan.items.as_ptr();
        let seat = self.budget.planned().items.as_ptr();

        self.cache.insert(
            surface_id,
            BindSet::default(),
            PatternState::default(),
            plan,
        );

        Applied {
            delta: self.budget.take_delta(),
            seat,
            inserted,
        }
    }
}

/// 定常状態は増分 0（完全ゼロ）。
///
/// 累積も据え置きであることを同時に見る（増分だけ 0 で累積が伸びる片肺実装を通さない）。
#[test]
fn a_steady_run_allocates_nothing_at_all() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(20);
    }
    let warm = *rig.budget.cumulative();

    for i in 0..8 {
        assert_eq!(
            rig.apply(20).delta,
            BudgetDelta::default(),
            "定常状態の {i} 回目で確保が起きた（完全ゼロが不成立）"
        );
    }
    assert_eq!(
        *rig.budget.cumulative(),
        warm,
        "定常状態で累積が伸びた（増分だけ 0 に見せる実装を通さない）"
    );
}

/// 立ち上がりの形を丸めない: 確保は「実体ごとに一度」の形で現れて 0 へ落ちる。
///
/// 4 回は交代で回る本数（キャッシュの 3 本＋席 1 本）で、キャッシュに空きがあるあいだは回収が成立しない
/// ので 1 適用ずつ順に起こされる。立ち上がりを 1 適用へ丸めるとこの成長が観測の外へ落ち、黙って確保する
/// 実装を通してしまう。
#[test]
fn the_warm_up_allocates_once_per_rotating_plan_then_settles() {
    let mut rig = Flow2::new();
    let shape: Vec<u32> = (0..WARMUP + 3)
        .map(|_| rig.apply(20).delta.alloc_plan)
        .collect();
    assert_eq!(
        shape,
        vec![1, 1, 1, 1, 0, 0, 0],
        "立ち上がりが「実体ごとに一度」の形から外れている"
    );
}

/// 再利用が名目でなく実体であること（ポインタの集合と交代の関係）。
///
/// 定常状態では計画の席と合成メモの計画が同じ 4 本を役割を替えながら巡る。「前回の席が今回キャッシュへ
/// 挿す計画になる」は、同時に生きている 2 実体の役割入れ替えを主張するので番地の偶然一致が入り込まず、
/// `swap_plan_scratch` を複写へ戻す変異をここで殺す。
#[test]
fn the_steady_state_keeps_handing_back_the_same_allocations() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(20);
    }

    let seen: Vec<Applied> = (0..8).map(|_| rig.apply(20)).collect();
    for (i, now) in seen.iter().enumerate() {
        assert_ne!(
            now.seat, now.inserted,
            "{i} 回目: 席と挿した計画が同一実体（交代が成立していない）"
        );
        if i > 0 {
            assert_eq!(
                now.inserted,
                seen[i - 1].seat,
                "{i} 回目: 前回の席がキャッシュへ回っていない（swap になっていない）"
            );
        }
    }

    let mut distinct: Vec<*const PlanItem> = seen
        .iter()
        .map(|s| s.seat)
        .chain(seen.iter().map(|s| s.inserted))
        .collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        WARMUP,
        "交代で回る実体はキャッシュの 3 本＋席 1 本のはず（確保し直している／潰れている）"
    );
}

/// 計画の拡大は実体ごとに一度だけ計数され、その後また 0 に戻る。
#[test]
fn a_larger_plan_costs_exactly_one_allocation_per_plan_then_returns_to_zero() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(20);
    }

    let shape: Vec<u32> = (0..WARMUP + 3)
        .map(|_| rig.apply(30).delta.alloc_plan)
        .collect();
    assert_eq!(
        shape,
        vec![1, 1, 1, 1, 0, 0, 0],
        "拡大の代金が「実体ごとに一度」の形から外れている"
    );
}

/// 要の檻: より小さい計画の後続要求で容量が縮まない。
///
/// 縮小 → 再拡大（元の最大へ戻る）の往復で 1 件も計数されないことが、席が容量を保持し続けていることの
/// 唯一の証拠になる（同じ大きさの反復だけでは「毎回まっさらに確保する」実装を素通りさせる）。
#[test]
fn a_smaller_plan_and_the_regrowth_allocate_nothing() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(30);
    }

    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(10).delta,
            BudgetDelta::default(),
            "縮小 {i} 回目で確保が起きた（席の容量が縮んでいる）"
        );
    }
    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(30).delta,
            BudgetDelta::default(),
            "元の最大への再拡大 {i} 回目で確保が起きた（容量が保持されていない）"
        );
    }
}

/// 回収が成立しなかったら黙って確保せず必ず計数へ現す。
///
/// キャッシュが無効化されると表が空になり、再び満杯になるまで回収エントリを得られない。代金は 1 適用
/// 遅れて現れる: 回収不成立の適用では `recycled_plan` が返した空の計画がそのまま席へ送り込まれ、
/// 中身（容量つき）がキャッシュへ出て、空になった席の伸長は次の適用で起きる。無効化が捨てるのは
/// 3 本で、代金が複数の適用に分かれることを固定するのは一部を黙って確保する実装を通さないため。
#[test]
fn a_failed_recycle_is_counted_never_swallowed() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(20);
    }

    // 回収元を断つ（アトラス再構築・ghost 再読込に相当）。
    rig.cache.invalidate_all();
    let shape: Vec<u32> = (0..WARMUP + 3)
        .map(|_| rig.apply(20).delta.alloc_plan)
        .collect();
    assert_eq!(
        shape,
        vec![0, 1, 1, 1, 0, 0, 0],
        "回収不成立の代金が計画の席へ現れていない（黙って確保している）"
    );
}

/// 回収エントリの計画を素通しで返し、回収不成立は空から始める。
///
/// 回収エントリを捨てて新品を返す変異は、捨てた直後の確保が同じ番地を引き当て得るので、番地に加えて
/// 新品の容量 0 を見る。
#[test]
fn the_recycled_plan_hands_back_the_recycled_allocation_unchanged() {
    let mut budget = FrameBudget::new();

    let mut recycled = empty_plan();
    recycled.items.push(PlanItem::GroupEnd);
    let items = recycled.items.as_ptr();
    let plan = FrameBudget::recycled_plan(Some(CacheEntry {
        plan: recycled,
        snapshot: Default::default(),
    }));
    assert_eq!(
        plan.items.as_ptr(),
        items,
        "エントリの計画が席の交代へ返されていない（別の実体にすり替わっている）"
    );
    assert_eq!(plan.items.len(), 1, "回収した計画の中身が返っていない");
    assert_eq!(
        budget.take_delta(),
        BudgetDelta::default(),
        "回収が成立した貸し出しで確保を数えている"
    );

    let fresh = FrameBudget::recycled_plan(None);
    assert_eq!(fresh.items.capacity(), 0, "回収不成立の計画は空から始まる");
}

/// 計画の常設席は項の列が伸びた回だけ数え、同じ寸の貸し出しでは数えない。席を交代させて受け取った
/// 新しい実体が伸びれば、また 1 件数える。
#[test]
fn the_plan_seat_counts_only_the_growth_of_its_item_list() {
    let mut budget = FrameBudget::new();
    let fill = |plan: &mut DrawPlan, n: usize| {
        plan.items.clear();
        plan.items
            .extend(std::iter::repeat_n(PlanItem::GroupEnd, n));
    };

    budget.plan_scratch(|plan| fill(plan, 5));
    assert_eq!(
        budget.take_delta().alloc_plan,
        1,
        "初回の伸長を数えていない"
    );
    for i in 0..3 {
        budget.plan_scratch(|plan| fill(plan, 5));
        assert_eq!(
            budget.take_delta(),
            BudgetDelta::default(),
            "同じ寸の {i} 回目の貸し出しで確保を数えた"
        );
    }
    budget.plan_scratch(|plan| fill(plan, 2));
    assert_eq!(
        budget.take_delta().alloc_plan,
        0,
        "縮めた貸し出しで確保を数えた"
    );

    let mut fresh = empty_plan();
    budget.swap_plan_scratch(&mut fresh);
    assert_eq!(fresh.items.len(), 2, "交代で席の計画が外へ出ていない");
    budget.plan_scratch(|plan| fill(plan, 5));
    assert_eq!(
        budget.take_delta().alloc_plan,
        1,
        "交代で受け取った空の実体の伸長を数えていない"
    );
    assert_eq!(budget.cumulative().alloc_plan, 2, "累積は 2 回の伸長");
}

/// 節の写しが作り直した項の数は増分と累積の両方に積まれ、増分は取り出しで 0 に戻る。
#[test]
fn node_rebuilds_are_counted_per_apply_and_in_total() {
    let mut budget = FrameBudget::new();
    budget.note_rebuilds(3);
    budget.note_rebuilds(2);
    assert_eq!(budget.take_delta().node_rebuilds, 5);
    assert_eq!(
        budget.take_delta().node_rebuilds,
        0,
        "取り出しで 0 に戻らない"
    );
    budget.note_rebuilds(1);
    assert_eq!(budget.cumulative().node_rebuilds, 6);
}
