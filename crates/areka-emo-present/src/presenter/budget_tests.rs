//! `FrameBudget`（budget.rs）の単体檻。
//! 固定するのは確保計数シームの意味論: 申告されたときだけ増える・発生点とフィールドが 1:1・
//! 増分は `take_delta` でリセットされ累積はされない。増分は perf サマリ行の `alloc_*` へ載り、
//! 累積はテストが「定常状態で新規確保が起きていない」を主張する読み取り口になる。
//!
//! 発生点は合成先とマスクの 2 つ。席について次の 3 点を固定する:
//!
//! - 定常状態は完全ゼロ
//! - 寸法変化は席ごとに一度だけ計数され、以後また 0
//! - 回収不成立・マスク輪番の単独所有不成立でも確保は必ず計数に現れる（黙って確保しない）
//!
//! 回る実体は 4 本。キャッシュ容量 3 に対し [`Flow2`] が 4 種類のキーを巡回させ、毎適用ミス＋追い出し
//! 1 件（本番の「毎コマ引き当て外れ」と同じ形）にする。表示バッファと合成先席の交代
//! （[`FrameBudget::swap_native_scratch`]）は拡大率に依らず無条件に走り、キャッシュ 3 本＋席 1 本、
//! マスクはキャッシュ 3 本＋輪番の空き 1 枚が回る。暖機に要する適用数がこの本数で、[`WARMUP`] が対応する。
//!
//! 同じ寸法の反復だけでは再利用を主張できない（毎回まっさらに確保する実装も毎回同じ容量に着地して
//! 素通りする）。再利用の主張は「より小さい寸法の後続要求で容量が縮まない」か「ポインタ同一性」の
//! どちらかで書く。実時間は合否に使わない（純 x64 の常設テスト）。

use super::*;

use areka_emo_compose::{BindSet, PatternState};

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

    budget.note_alloc(AllocSite::ComposeDst);
    budget.note_alloc(AllocSite::Mask);
    budget.note_alloc(AllocSite::Mask);
    let first = budget.take_delta();
    assert_eq!(
        first,
        BudgetDelta {
            alloc_compose_dst: 1,
            alloc_mask: 2,
        },
        "同一発生点の複数回申告は足し合わされること"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_mask: 2,
        },
        "累積は増分と同じ値から始まる"
    );

    assert_eq!(
        budget.take_delta(),
        BudgetDelta::default(),
        "取り出し済みの増分が次の適用へ持ち越されている（リセット漏れ）"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_mask: 2,
        },
        "累積は増分の取り出しでリセットされないこと"
    );

    budget.note_alloc(AllocSite::ComposeDst);
    let third = budget.take_delta();
    assert_eq!(third.alloc_compose_dst, 1, "適用 3 の増分");
    assert_eq!(third.alloc_mask, 0, "適用 3 で申告していない発生点は 0");
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 2,
            alloc_mask: 2,
        },
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
        &[AllocSite::ComposeDst],
        &[],
        &[AllocSite::Mask, AllocSite::Mask],
        &[AllocSite::ComposeDst, AllocSite::ComposeDst],
    ];

    let mut summed = [0_u64; 2];
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
    // 台本の実数（発生点ごとの申告回数）。総和どうしの空虚な一致を防ぐ。
    assert_eq!(budget.cumulative().count(AllocSite::ComposeDst), 3);
    assert_eq!(budget.cumulative().count(AllocSite::Mask), 2);
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
    /// 交代**後**の合成先席（＝次の適用が合成する器）。
    native: *const u8,
    /// この適用でキャッシュへ挿した表示バッファ。
    display: *const u8,
    mask: *const AlphaMask,
}

/// apply_show のミス経路を席の側から演じる装置。
///
/// `show.rs` の順序を手で踏む: 合成（常設席）→ 容量回収（合成成功後に限る）→ 表示バッファ席 →
/// 交代 → マスク輪番 → 表示バッファと `Arc` マスクを原子対で挿入 → 下流供給。
///
/// `resource` が下流の現行マスク保持を表す。これが無いと現行マスクの参照数が 2 にならず、
/// 「前々回のマスクだけが単独所有」という輪番の前提が空虚になる。
///
/// 同じキーの繰り返しではミス経路にならないので、surface id を 4 種類巡回させる
/// （巡回長 4 > 容量 3 で毎適用ミス＋追い出し 1 件）。
struct Flow2 {
    budget: FrameBudget,
    cache: ComposeCache,
    /// `AlphaMaskResource` 相当（下流が現行マスクの `Arc` を保持し続ける）。
    resource: Option<Arc<AlphaMask>>,
    /// 適用回数（巡回するキーの選択に使う）。
    applies: usize,
}

/// 巡回するキーの本数。**キャッシュ容量より大きい**こと自体が必須である（毎適用ミスの条件）。
const ROTATING_KEYS: usize = 4;

/// 暖機に要する適用数＝回る実体の本数。
///
/// 交代する表示バッファ・合成先席（キャッシュの 3 本＋席 1 本）とマスク（3 本＋空き 1 枚）は
/// いずれも 1 適用ずつずれて外形へ伸びるので、全ての実体が定常へ入るまでにこれだけかかる。
const WARMUP: usize = ROTATING_KEYS;

impl Flow2 {
    fn new() -> Self {
        Self {
            budget: FrameBudget::new(),
            cache: ComposeCache::new(),
            resource: None,
            applies: 0,
        }
    }

    /// ミス経路を 1 適用ぶん演じ、この適用の増分と実体を返す。
    fn apply(&mut self, native_w: u32, native_h: u32) -> Applied {
        // 巡回長 4 > 容量 3 ゆえ、この id は 1 手前で追い出されている＝必ずミスする。
        let surface_id = 1000 + (self.applies % ROTATING_KEYS) as u32;
        self.applies += 1;
        assert!(
            !self
                .cache
                .touch(surface_id, &BindSet::default(), &PatternState::default()),
            "前提: 本装置はミス経路を演じる（巡回長 {ROTATING_KEYS} > 容量ゆえ必ずミスする）"
        );
        self.budget.native_scratch(|dst| {
            dst.resize_and_clear(native_w, native_h);
        });

        // 容量回収は合成成功後にのみ。
        let recycled = self.cache.take_recycled();

        let (mut display, retired) = FrameBudget::display_buffer(recycled);

        self.budget.swap_native_scratch(&mut display);
        let display_ptr = display.bytes().as_ptr();
        let native_ptr = self.budget.native_scratch_ptr();

        // マスクは原寸バイト由来。
        let mask = self.budget.regenerate_mask(
            retired,
            display.bytes(),
            display.width(),
            display.height(),
            display.stride(),
        );
        let mask_ptr = Arc::as_ptr(&mask);

        // 表示記録は GPU 不要の空リスト。
        self.cache.insert(
            surface_id,
            BindSet::default(),
            PatternState::default(),
            display,
            Arc::clone(&mask),
            wintf::ecs::GraphicsCommandList::empty(),
        );

        // 旧マスクはここで手放され、輪番の空きが単独所有になる。
        self.resource = Some(mask);

        Applied {
            delta: self.budget.take_delta(),
            native: native_ptr,
            display: display_ptr,
            mask: mask_ptr,
        }
    }
}

/// 定常状態は 2 フィールドとも増分 0（完全ゼロ）。
///
/// 累積も据え置きであることを同時に見る（増分だけ 0 で累積が伸びる片肺実装を通さない）。
/// 立ち上がりの内訳は `the_warm_up_allocates_once_per_rotating_buffer_then_settles` が持つ。
#[test]
fn a_steady_run_allocates_nothing_at_all() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20);
    }
    let warm = *rig.budget.cumulative();

    for i in 0..8 {
        let seen = rig.apply(40, 20);
        assert_eq!(
            seen.delta,
            BudgetDelta::default(),
            "定常状態の {i} 回目で確保が起きた（Requirement 3.1 の完全ゼロが不成立）"
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
/// 実測は `[1,1]` が 4 適用ぶん、以後 0。`alloc_compose_dst` の 4 回は交代で回る本数（キャッシュの
/// 3 本＋合成先席 1 本）で、キャッシュに空きがあるあいだは回収が成立しないので 1 適用ずつ順に
/// 起こされる。`alloc_mask` の 4 回はキャッシュの 3 本＋輪番の空き 1 枚。立ち上がりを 1 適用へ
/// 丸めるとこの成長が観測の外へ落ち、黙って確保する実装を通してしまう。
#[test]
fn the_warm_up_allocates_once_per_rotating_buffer_then_settles() {
    let mut rig = Flow2::new();
    let shape: Vec<[u32; 2]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(40, 20).delta;
            [d.alloc_compose_dst, d.alloc_mask]
        })
        .collect();
    assert_eq!(
        shape,
        vec![[1, 1], [1, 1], [1, 1], [1, 1], [0, 0], [0, 0], [0, 0]],
        "立ち上がりが「実体ごとに一度」の形から外れている（Requirement 3.2）"
    );
}

/// 再利用が名目でなく実体であること（ポインタの集合と交代の関係）。
///
/// 定常状態では合成先席と表示バッファが同じ 4 本を役割を替えながら巡る。本数が増えれば確保し直し、
/// 減れば交代が成立していない。「前回の合成先席が今回の表示バッファになる」は、同時に生きている
/// 2 実体の役割入れ替えを主張するので番地の偶然一致が入り込まず、`swap_native_scratch` を複写へ
/// 戻す変異をここで殺す。
#[test]
fn the_steady_state_keeps_handing_back_the_same_allocations() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20);
    }

    let first = rig.apply(40, 20);
    let mut seen = vec![first];
    for _ in 0..7 {
        seen.push(rig.apply(40, 20));
    }

    for (i, now) in seen.iter().enumerate() {
        assert_ne!(
            now.native, now.display,
            "{i} 回目: 合成先席と表示バッファが同一実体（交代が成立していない）"
        );
        if i > 0 {
            assert_eq!(
                now.display,
                seen[i - 1].native,
                "{i} 回目: 前回の合成先席が表示バッファへ回っていない（swap になっていない）"
            );
        }
    }

    let mut distinct: Vec<*const u8> = seen
        .iter()
        .map(|s| s.native)
        .chain(seen.iter().map(|s| s.display))
        .collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        WARMUP,
        "交代で回る実体はキャッシュの 3 本＋合成先席 1 本のはず（確保し直している／潰れている）"
    );
}

/// 寸法拡大は確保対象ごとに一度だけ計数され、その後また 0 に戻る。
///
/// in-place 再生成でも確保は起きる。`AlphaMask::regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)`
/// で、詰め長が容量を超えれば `resize` は確実に確保する（40×20 → 60×30 で詰め長 100 → 240 バイト・
/// 行はバイト境界まで詰める）。回る実体は 1 適用ずつずれて再生成されるので、全部が伸びるまでに本数
/// ぶんの適用がかかり、各適用でちょうど 1 件ずつ計数される。立ち上がりと同じ形が出るのは、寸法が
/// 変われば「その寸法では初めて」になるため。
#[test]
fn a_larger_extent_costs_exactly_one_allocation_per_buffer_then_returns_to_zero() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20);
    }

    let shape: Vec<[u32; 2]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(60, 30).delta;
            [d.alloc_compose_dst, d.alloc_mask]
        })
        .collect();
    assert_eq!(
        shape,
        vec![[1, 1], [1, 1], [1, 1], [1, 1], [0, 0], [0, 0], [0, 0]],
        "寸法拡大の代金が「実体ごとに一度」の形から外れている（Requirement 3.2）"
    );
}

/// 要の檻: より小さい寸法の後続要求で容量が縮まない。
///
/// 同一寸法の反復だけを見る檻は「毎回まっさらに確保する」実装を素通りさせる（毎回同じ容量に
/// 着地する）。縮小 → 再拡大（元の最大寸へ戻る）の往復で 1 件も計数されないことが、席が容量を
/// 保持し続けていることの唯一の証拠になる。
#[test]
fn a_smaller_extent_and_the_regrowth_allocate_nothing() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(60, 30);
    }

    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(20, 10).delta,
            BudgetDelta::default(),
            "縮小 {i} 回目で確保が起きた（席の容量が縮んでいる＝往復確保を作る実装）"
        );
    }
    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(60, 30).delta,
            BudgetDelta::default(),
            "元の最大寸への再拡大 {i} 回目で確保が起きた（容量が保持されていない）"
        );
    }
}

/// 回収が成立しなかったら黙って確保せず必ず計数へ現す。
///
/// キャッシュが無効化されると表が空になり、再び満杯になるまで回収エントリを得られない。ここで
/// 無言のまま確保すると、定常アロケーション 0 が観測上は成立しているのに実際は確保している
/// 隠れた縮退になる。
///
/// 代金は `ComposeDst` に 1 適用遅れて現れる。回収不成立の適用では `display_buffer` が返した空の
/// バッファがそのまま合成先席へ送り込まれ、合成済みの中身（容量つき）が表示バッファへ出る。空に
/// なった合成先席の伸長は次の適用の合成で起きる。
///
/// 実測は `[0,0]` → `[1,1]` → `[1,1]` → `[1,1]` → 以後 0。無効化が捨てるのは表示バッファとマスクの
/// 原子対まるごと（3 対）で代金は 3 適用ぶんあり、初回のマスクだけは無効化の直前に輪番が握って
/// いた空き 1 枚が生き残るので確保なしで済む。代金が複数の適用に分かれることを固定するのは、
/// 一部を黙って確保する実装を通さないため。
#[test]
fn a_failed_recycle_is_counted_never_swallowed() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20);
    }

    // 回収元を断つ（アトラス再構築・ghost 再読込に相当）。
    rig.cache.invalidate_all();
    let shape: Vec<[u32; 2]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(40, 20).delta;
            [d.alloc_compose_dst, d.alloc_mask]
        })
        .collect();
    assert_eq!(
        shape,
        vec![[0, 0], [1, 1], [1, 1], [1, 1], [0, 0], [0, 0], [0, 0]],
        "回収不成立の代金が合成先席とマスクへ現れていない（黙って確保している）"
    );
}

/// マスク輪番はキャッシュの 3 本＋空き 1 枚で回り、定常状態では in-place 再生成だけになる。
///
/// 立ち上がりは 4 適用ぶん（4 枚の確保）。実体がちょうど 4 種類で回ることも固定する（計数だけでは
/// 「毎回確保しているのに数え忘れている」実装と区別できない）。
#[test]
fn the_mask_rotation_settles_into_in_place_regeneration() {
    let mut rig = Flow2::new();

    let mut seen = Vec::new();
    for i in 0..WARMUP {
        let now = rig.apply(40, 20);
        assert_eq!(
            now.delta.alloc_mask, 1,
            "立ち上がりの {i} 枚目のマスクが確保されていない"
        );
        seen.push(now.mask);
    }
    let known = seen.clone();

    for i in 0..8 {
        let now = rig.apply(40, 20);
        assert_eq!(
            now.delta.alloc_mask, 0,
            "輪番が立ち上がった後の {i} 回目でマスクを確保した"
        );
        assert!(
            known.contains(&now.mask),
            "{i} 回目のマスクが既知の {WARMUP} 枚のどれでもない（輪番が新しい実体を作っている）"
        );
        seen.push(now.mask);
    }

    seen.sort_unstable();
    seen.dedup();
    assert_eq!(
        seen.len(),
        WARMUP,
        "マスクの実体はキャッシュの 3 本＋空き 1 枚のはず（輪番が無限成長している）"
    );
}

/// 単独所有が不成立なら確保し、必ず計数へ現す。
///
/// 下流以外が現行マスクの `Arc` を握り続けると、それが輪番の空きスロットへ回ってきた適用で
/// `Arc::get_mut` が成立しない。無言で新規確保すると、計数上は定常ゼロのまま毎回確保する縮退が
/// 作れてしまう。
///
/// 複製を握った適用の 4 つ後（回る実体の本数＝キャッシュの 3 本＋空き 1 枚）でちょうど 1 件計数され、
/// 複製を手放せば輪番が自力で立ち直ることまで見る。
#[test]
fn a_mask_slot_that_is_not_uniquely_owned_allocates_and_is_counted() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP + 4 {
        rig.apply(40, 20);
    }

    // この適用のマスクを第三者として握る（＝以後このマスクは単独所有にならない）。
    rig.apply(40, 20);
    let hostage = Arc::clone(rig.resource.as_ref().expect("下流へ供給済み"));

    // 人質のマスクが空きスロットへ回るまでの適用は、他の実体が単独所有ゆえ in-place で通る。
    let held: Vec<u32> = (0..WARMUP + 4)
        .map(|_| rig.apply(40, 20).delta.alloc_mask)
        .collect();
    assert_eq!(
        held,
        vec![0, 0, 0, 1, 0, 0, 0, 0],
        "単独所有の不成立がちょうど 1 件・輪番の一巡後に現れるはず（黙って確保していないか）"
    );

    drop(hostage);
    for i in 0..WARMUP + 4 {
        assert_eq!(
            rig.apply(40, 20).delta.alloc_mask,
            0,
            "複製を手放した後の {i} 回目で輪番が立ち直っていない"
        );
    }
}

/// 表示バッファ席は回収エントリを素通しで返す。
///
/// 回収エントリを捨てて新品を返す変異は、捨てた直後の確保が同じ番地を引き当て得るのでポインタ
/// 同一性の檻を素通りする。回収した外形と長さがそのまま返ることなら番地の巡り合わせに左右されず、
/// 新品は必ず 0×0 から始まる。
#[test]
fn the_display_buffer_hands_back_the_recycled_allocation_unchanged() {
    let mut budget = FrameBudget::new();

    // 回収エントリ相当（外形は「新品の 0×0」と紛れない値にする）。
    let mut composed = ComposedSurface::default();
    composed.resize_and_clear(7, 3);
    let bytes = composed.bytes().len();
    let mask = Arc::new(AlphaMask::from_pbgra32(
        composed.bytes(),
        composed.width(),
        composed.height(),
        composed.stride(),
    ));
    let mask_ptr = Arc::as_ptr(&mask);

    let (display, retired) = FrameBudget::display_buffer(Some(CacheEntry {
        composed,
        mask,
        display: wintf::ecs::GraphicsCommandList::empty(),
    }));

    assert_eq!(
        display.width(),
        7,
        "回収した外形が返っていない（新品を返した）"
    );
    assert_eq!(
        display.height(),
        3,
        "回収した外形が返っていない（新品を返した）"
    );
    assert_eq!(display.bytes().len(), bytes, "回収した長さが返っていない");
    assert_eq!(
        retired.as_ref().map(Arc::as_ptr),
        Some(mask_ptr),
        "エントリのマスクが輪番へ返されていない（別の実体にすり替わっている）"
    );
    assert_eq!(
        budget.take_delta(),
        BudgetDelta::default(),
        "回収が成立した貸し出しで確保を数えている"
    );

    let (fresh, none) = FrameBudget::display_buffer(None);
    assert_eq!(fresh.width(), 0, "回収不成立の表示バッファは空から始まる");
    assert_eq!(
        fresh.bytes().len(),
        0,
        "回収不成立の表示バッファは空から始まる"
    );
    assert!(none.is_none(), "回収不成立で輪番へ返すマスクは無い");
}

/// 合成先の常設席は初回だけ確保し、以後は同じ実体を貸し続ける。
///
/// 席を直接叩く最小の檻（Flow2 を通さないので交代が挟まらない）。
#[test]
fn the_compose_destination_seat_is_allocated_once_and_then_lent_again() {
    let mut budget = FrameBudget::new();

    let first = budget.native_scratch(|dst| {
        dst.resize_and_clear(40, 20);
        dst.bytes().as_ptr()
    });
    assert_eq!(
        budget.take_delta().alloc_compose_dst,
        1,
        "常設席の初回伸長が計数されていない"
    );

    for i in 0..6 {
        let again = budget.native_scratch(|dst| {
            dst.resize_and_clear(40, 20);
            dst.bytes().as_ptr()
        });
        assert_eq!(
            budget.take_delta(),
            BudgetDelta::default(),
            "常設席の {i} 回目の貸し出しで確保が起きた"
        );
        assert_eq!(again, first, "常設席が {i} 回目に別の実体へ差し替わった");
    }
}
