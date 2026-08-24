//! `FrameBudget`（budget.rs）の単体檻。
//!
//! 本ファイルが固定するのは **確保計数シームの意味論**（Requirement 1.3・design.md
//! §FrameBudget・D6）である。すなわち
//!
//! - 確保が起きたと申告されたときにだけカウンタが増える（申告なしでは増えない）
//! - 発生点とフィールドが 1:1 で対応する（取り違えると別のフィールドが動く）
//! - 適用単位の増分は [`FrameBudget::take_delta`] の取り出しでリセットされる
//! - 累積は取り出しでリセットされず、run 全体を通して増え続ける
//!
//! の 4 点である。増分は perf サマリ行の `alloc_*` フィールドへそのまま載り（Requirement 1.3）、
//! 累積はテストが「定常状態で新規確保が起きていない」を主張するための読み取り口になる。
//!
//! # 席の再利用（task 5.2 で本器へ入った）
//!
//! [`FrameBudget`] は計数だけでなく再利用席（合成先の常設席・リサンプル作業領域の席・マスクの
//! 輪番）を持つ。ゆえに計数は「呼び手が確保したと申告した回数」から**「席の再利用が成立せず
//! 結局確保した回数」**へ意味が移った（design.md §FrameBudget・D2/D3/D6）。上の 4 点の意味論は
//! そのまま生きている（増分と累積の関係は席の導入で動かない）。
//!
//! 本ファイルが席について固定するのは次の 3 点である:
//!
//! - **定常状態は完全ゼロ**（Requirement 3.1）: 初回確保が済んだ後の反復では全フィールドが 0
//! - **寸法変化は席ごとに一度だけ**（Requirement 3.2）: 変化した適用で 1 増え、以後また 0
//! - **黙って確保しない**（design.md §Error Handling）: 席が期待どおり再利用できない境界
//!   （回収不成立・マスク輪番の単独所有が不成立）でも確保は必ず計数に現れる
//!
//! # 回る実体の本数（キャッシュ容量 3・要件 7.1）
//!
//! キャッシュ容量が 1 → 3 になったため、毎コマ経路を回る実体の本数が増えた。本ファイルの
//! [`Flow2`] は **4 種類のキーを巡回**させてミス経路を演じる（巡回長 4 > 容量 3 ゆえ毎適用が
//! ミスし、毎適用で追い出しが 1 件成立する＝本番の「毎コマ引き当て外れ」と同じ形）。ゆえに
//!
//! - 非恒等 k: 表示バッファは**キャッシュの 3 本**が回り、合成先席は 1 本で固定
//! - 恒等 k: 合成先席と表示バッファが交代するので **4 本**が両方の役を回る
//! - マスク: キャッシュの 3 本＋輪番の空き 1 枚＝**4 本**
//!
//! 立ち上がり（暖機）に要する適用数は本数ぶんであり、[`WARMUP`] はそれに合わせてある。
//!
//! # 「同じ寸法の反復」だけでは再利用を主張できない（本 spec が 3 度踏んだ罠）
//!
//! 毎回まっさらに確保する実装は、同じ寸法の反復では毎回**同じ容量**に着地するため、容量比較の
//! 檻を素通りする。task 4.1／4.2／5.1 で変異を捕まえたのはいずれも
//! ⑴**より小さい寸法の後続要求で容量が縮まないこと** ⑵**ポインタ同一性**の 2 種だけだった。
//! ゆえに本ファイルの再利用主張は必ずこの 2 種のどちらかで書く。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! 本檻は時刻にも実行速度にも一切触れない（回数・ポインタ・寸法のみ）。純 x64 の常設テストで
//! あり、環境変数ゲートも `#[ignore]` も持たない（Requirement 6.3）。

use super::*;

use areka_emo_compose::{BindSet, PatternState, resample};

use crate::cache::ComposeCache;

/// 発生点の全数（design.md §FrameBudget の席一覧と 1:1）。
///
/// 4 つ未満へ縮んだら perf サマリ行のフィールドが欠け、判定スクリプトの契約が壊れる。
#[test]
fn every_allocation_site_is_enumerated() {
    assert_eq!(
        AllocSite::ALL.len(),
        4,
        "確保発生点は 4 つ（合成先・表示バッファ・リサンプル作業領域・当たり判定マスク）"
    );
}

/// Requirement 1.3 観測完了: 新品の器は増分・累積とも全て 0。
///
/// 「初期値が 0 でない」実装は、定常状態ゼロの主張（Requirement 3.1）を最初から成立不能にする。
#[test]
fn a_fresh_budget_counts_nothing() {
    let budget = FrameBudget::new();

    assert_eq!(
        *budget.cumulative(),
        BudgetCounters::default(),
        "新品の累積は全て 0"
    );
    for site in AllocSite::ALL {
        assert_eq!(
            budget.cumulative().count(site),
            0,
            "{site:?} の累積が 0 でない"
        );
    }
}

/// Requirement 1.3 観測完了: **確保が起きたときだけ**カウンタが増える。
///
/// 申告のない適用（＝是正後の定常状態が目指す形）では、何度増分を取り出しても全フィールドが
/// 0 のままである。これは「呼ばれるたびに数える」誤実装（例: 取得シームの入口で無条件に
/// 増やす）を殺す——それでは再利用の成立・不成立が区別できず、計数が判定材料にならない。
#[test]
fn nothing_is_counted_without_an_allocation() {
    let mut budget = FrameBudget::new();

    for _ in 0..8 {
        assert_eq!(
            budget.take_delta(),
            BudgetDelta::default(),
            "確保の申告が無い適用の増分は 0（定常状態の形）"
        );
    }
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters::default(),
        "確保の申告が無ければ累積も動かない"
    );
}

/// Requirement 1.3 観測完了: 発生点とフィールドが 1:1 で対応する（取り違えを殺す）。
///
/// 各発生点を 1 回だけ申告し、⑴対応する**名前つきフィールド**が 1 になる ⑵他の 3 フィールドが
/// 0 のままである、を全発生点について確認する。フィールドの割り当てを 1 組でも入れ替えると
/// （例: `Xmap` が `alloc_mask` を増やす）、この檻は必ず赤になる。
#[test]
fn each_site_increments_only_its_own_field() {
    for site in AllocSite::ALL {
        let mut budget = FrameBudget::new();
        budget.note_alloc(site);
        let delta = budget.take_delta();

        // 名前つきフィールドで直接受ける（`count` の実装が壊れていても素通ししない）。
        let named = match site {
            AllocSite::ComposeDst => delta.alloc_compose_dst,
            AllocSite::ResampleDst => delta.alloc_resample_dst,
            AllocSite::Xmap => delta.alloc_xmap,
            AllocSite::Mask => delta.alloc_mask,
        };
        assert_eq!(
            named, 1,
            "{site:?} の申告が対応フィールドへ載っていない: {delta:?}"
        );

        for other in AllocSite::ALL {
            if other != site {
                assert_eq!(
                    delta.count(other),
                    0,
                    "{site:?} の申告が無関係な発生点 {other:?} を動かした: {delta:?}"
                );
            }
        }
    }
}

/// Requirement 1.3 観測完了: 増分は取り出しでリセットされ、累積はリセットされない。
///
/// 適用 3 回分を演じる:
/// - 適用 1: 合成先 1 回・マスク 2 回の確保（同一発生点の複数回申告が足し合わされる）
/// - 適用 2: 確保なし（定常状態）→ 増分は全 0・累積は据え置き
/// - 適用 3: 表示バッファ 1 回 → 増分に現れ、累積は 1・2 を保ったまま伸びる
///
/// # 殺す誤実装
///
/// - `take_delta` がリセットしない → 適用 2 の増分に適用 1 の値が残り赤
/// - `take_delta` が累積まで巻き戻す → 適用 2 の累積 assert で赤
/// - 増分と累積が同一の器を共有している → どちらかの assert で必ず赤
#[test]
fn take_delta_resets_the_increment_while_the_cumulative_keeps_growing() {
    let mut budget = FrameBudget::new();

    // 適用 1: 初回確保（寸法確定前の一度きりの形）。
    budget.note_alloc(AllocSite::ComposeDst);
    budget.note_alloc(AllocSite::Mask);
    budget.note_alloc(AllocSite::Mask);
    let first = budget.take_delta();
    assert_eq!(
        first,
        BudgetDelta {
            alloc_compose_dst: 1,
            alloc_resample_dst: 0,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "同一発生点の複数回申告は足し合わされること"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_resample_dst: 0,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "累積は増分と同じ値から始まる"
    );

    // 適用 2: 確保なし（定常状態）。増分 0・累積据え置き。
    assert_eq!(
        budget.take_delta(),
        BudgetDelta::default(),
        "取り出し済みの増分が次の適用へ持ち越されている（リセット漏れ）"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_resample_dst: 0,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "累積は増分の取り出しでリセットされないこと"
    );

    // 適用 3: 表示バッファの再確保（寸法変化の形）。
    budget.note_alloc(AllocSite::ResampleDst);
    let third = budget.take_delta();
    assert_eq!(third.alloc_resample_dst, 1, "適用 3 の増分");
    assert_eq!(
        third.alloc_compose_dst, 0,
        "適用 3 で申告していない発生点は 0"
    );
    assert_eq!(
        *budget.cumulative(),
        BudgetCounters {
            alloc_compose_dst: 1,
            alloc_resample_dst: 1,
            alloc_xmap: 0,
            alloc_mask: 2,
        },
        "累積は run 全体で積み上がること"
    );
}

/// 増分と累積の整合: 適用ごとに取り出した増分の総和が、累積と一致する。
///
/// これは「累積だけ・増分だけ」を数える片肺実装を殺す（片方が動かなければ総和が食い違う）。
/// 判定スクリプトは行ごとの増分を、テストは累積を根拠にするため、両者が同じ事実を指していない
/// 状態は静かな誤診断を生む。
#[test]
fn the_sum_of_per_apply_deltas_equals_the_cumulative() {
    let mut budget = FrameBudget::new();
    let program: [&[AllocSite]; 4] = [
        &[AllocSite::ComposeDst, AllocSite::Xmap],
        &[],
        &[AllocSite::Mask, AllocSite::Mask, AllocSite::ResampleDst],
        &[AllocSite::Xmap],
    ];

    let mut summed = [0_u64; 4];
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
    assert_eq!(budget.cumulative().count(AllocSite::ComposeDst), 1);
    assert_eq!(budget.cumulative().count(AllocSite::ResampleDst), 1);
    assert_eq!(budget.cumulative().count(AllocSite::Xmap), 2);
    assert_eq!(budget.cumulative().count(AllocSite::Mask), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// 席の再利用（task 5.2・Requirement 3.1/3.2・design.md D2/D3・Flow 2）
// ═══════════════════════════════════════════════════════════════════════════

/// 恒等 k（`1/1`）。
fn identity() -> ScaleRatio {
    ScaleRatio::new(1, 1).expect("非ゼロの比は必ず構築できる")
}

/// 非ゼロ既約 k の構築補助（`ScaleRatio::new` は 0 でのみ失敗する）。
fn k(num: u32, den: u32) -> ScaleRatio {
    ScaleRatio::new(num, den).expect("非ゼロの比は必ず構築できる")
}

/// 席の中身を `w×h` へ伸ばす（`compose_into` が合成先席へ書くときと同じ伸長を踏む）。
///
/// `ComposedSurface` の寸法合わせは emo-compose の `pub(crate)` ゆえ本クレートからは直接
/// 呼べない。恒等 k の公開 `resample` は入口で `out.resize_for_full_overwrite(外形)` を通る
/// （task 5.4 で `resize_and_clear` から差し替わった。長さが変われば従来どおり `clear`＋
/// `resize` が走るため**伸長・容量再利用の挙動は同一**で、恒等 k は続く `copy_from_slice` が
/// 全バイトを書くので中身も全 0 のまま変わらない）ため、**本物の合成が席へ及ぼすのと同じ
/// 伸長・同じ容量再利用**を公開経路だけで再現できる。
fn fill_extent(dst: &mut ComposedSurface, w: u32, h: u32) {
    let src = ComposedSurface::new(w, h);
    resample(&src, identity(), dst);
}

/// 1 適用で観測した値（計数の増分・表示バッファの実体・マスクの実体）。
///
/// ポインタは**同一性の主張にのみ**使う（読み出しはしない）。再利用が成立していれば同じ
/// 実体が返り続け、毎回まっさらに確保する実装では実体が入れ替わる。
struct Applied {
    delta: BudgetDelta,
    native: *const u8,
    display: *const u8,
    mask: *const AlphaMask,
}

/// design.md Flow 2（是正後の apply_show ミス経路）を席の側から演じる装置。
///
/// `show.rs` の再配線は task 5.3 の領分ゆえ、本檻は同じ順序を手で踏む——合成（常設席）→
/// 容量回収（合成成功後に限る）→ 表示バッファ席 → k 適用（恒等なら swap・非恒等なら
/// 常設作業席でリサンプル）→ マスク輪番 → 表示バッファと `Arc` マスクを原子対で挿入 →
/// 下流供給（`AlphaMaskResource::set_shared` 相当）。
///
/// `resource` を持つのが要点である。下流が現行マスクを保持し続けるからこそ現行マスクの
/// 参照数は 2 になり、**前々回のマスクだけが単独所有（＝in-place 再生成できる）**という
/// 輪番の前提（design.md D3）が本物になる。ここを省くと輪番の主張が空虚になる。
///
/// # 毎回ミスさせるにはキーを 4 種類回す（キャッシュ容量 3・要件 7.1）
///
/// 本装置は**ミス経路**を演じる。ところがキャッシュ容量が 1 → 3 になった今、同じキーを繰り返し
/// 挿入する形では表が 1 件で埋まらず、追い出しが一度も起きない（本番なら同じキーは 2 回目から
/// ヒットするので、ミス経路の反復として成立しない）。ゆえに **surface id を 4 種類**巡回させる:
/// 巡回長 4 > 容量 3 なので、LRU では入ってくるキーが必ず 1 手前で追い出されており、
/// **毎適用がミスかつ毎適用で追い出しが 1 件成立する**——これが本番の「毎コマ引き当て外れ」に
/// 対応する定常状態である。
struct Flow2 {
    budget: FrameBudget,
    cache: ComposeCache,
    /// `AlphaMaskResource` 相当（下流が現行マスクの `Arc` を保持し続ける）。
    resource: Option<Arc<AlphaMask>>,
    /// 適用回数（巡回するキーの選択に使う）。
    applies: usize,
}

/// 巡回するキーの本数。**キャッシュ容量より大きい**こと自体が要件である（毎適用ミスの条件）。
const ROTATING_KEYS: usize = 4;

/// 暖機に要する適用数＝回る実体の本数。
///
/// 表示バッファ（キャッシュの 3 本）・マスク（3 本＋空き 1 枚）・恒等 k の交代（4 本）は
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
    fn apply(&mut self, native_w: u32, native_h: u32, scale: ScaleRatio) -> Applied {
        // 巡回長 4 > 容量 3 ゆえ、この id は 1 手前で追い出されている＝必ずミスする。
        let surface_id = 1000 + (self.applies % ROTATING_KEYS) as u32;
        self.applies += 1;
        assert!(
            !self.cache.touch(
                surface_id,
                &BindSet::default(),
                &PatternState::default(),
                scale
            ),
            "前提: 本装置はミス経路を演じる（巡回長 {ROTATING_KEYS} > 容量ゆえ必ずミスする）"
        );
        // (1) 合成: 常設席へ native 外形の結果を書く（`compose_into` 相当）。
        let native_ptr = self.budget.native_scratch(|dst| {
            fill_extent(dst, native_w, native_h);
            dst.bytes().as_ptr()
        });

        // (2) 容量回収は**合成成功後にのみ**（Flow 2 の規律）。
        let recycled = self.cache.take_recycled();

        // (3) 表示バッファ席と、輪番へ返すマスク。
        let (mut display, retired) = self.budget.display_buffer(recycled);

        // (4) k 適用: 恒等は常設席と交代（コピーなし）・非恒等は常設作業席でリサンプル。
        if scale.is_identity() {
            self.budget.swap_native_scratch(&mut display);
        } else {
            self.budget.resample_native_into(scale, &mut display);
        }
        let display_ptr = display.bytes().as_ptr();

        // (5) マスク輪番（in-place 再生成・単独所有が不成立なら確保＋計数）。
        let mask = self.budget.regenerate_mask(
            retired,
            display.bytes(),
            display.width(),
            display.height(),
            display.stride(),
        );
        let mask_ptr = Arc::as_ptr(&mask);

        // (6) 表示バッファと `Arc` マスクを原子対で挿入（native 原寸も同じエントリへ束ねる）。
        self.cache.insert(
            surface_id,
            BindSet::default(),
            PatternState::default(),
            scale,
            display,
            Arc::clone(&mask),
            (native_w, native_h),
        );

        // (7) 下流供給（参照カウント増のみ）。旧マスクはここで手放され、輪番の空きが単独所有になる。
        self.resource = Some(mask);

        Applied {
            delta: self.budget.take_delta(),
            native: native_ptr,
            display: display_ptr,
            mask: mask_ptr,
        }
    }
}

/// Requirement 3.1: 非恒等 k の定常状態で **4 フィールドとも増分 0**（完全ゼロ）。
///
/// ウォームアップ（合成先席・キャッシュの 3 本・マスク 4 枚の立ち上げ）が済んだ後は、同一寸法の
/// 反復で 1 件の確保も起きない。累積も据え置きであることを同時に見る（増分だけ 0 で累積が伸びる
/// 片肺実装を通さない）。
#[test]
fn a_steady_run_allocates_nothing_at_all() {
    let mut rig = Flow2::new();
    // ウォームアップ（回る実体の本数ぶん）。
    for _ in 0..WARMUP {
        rig.apply(40, 20, k(2, 1));
    }
    let warm = *rig.budget.cumulative();

    for i in 0..8 {
        let seen = rig.apply(40, 20, k(2, 1));
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

/// Requirement 3.2 観測完了（**立ち上がりの形を丸めない**・非恒等 k）: 確保は「実体ごとに一度」の
/// 形で現れて 0 へ落ちる。
///
/// 実測の形は `[1,1,1,1]` → `[0,1,0,1]` → `[0,1,0,1]` → `[0,0,0,1]` → 以後 0 である。読み方:
///
/// - `alloc_compose_dst` は初回のみ（非恒等 k の合成先席は 1 本で固定・交代しない）
/// - `alloc_resample_dst` が 3 回＝**キャッシュが保持する表示バッファの本数**。キャッシュに空きが
///   あるあいだは回収が成立しないので、3 本が 1 適用ずつ順に起こされる
/// - `alloc_xmap` は初回のみ（作業席は 1 本）
/// - `alloc_mask` が 4 回＝キャッシュの 3 本＋輪番の空き 1 枚
///
/// 立ち上がりを 1 適用へ丸めるとこれらの成長が観測の外へ落ち、黙って確保する実装を通してしまう。
#[test]
fn the_warm_up_allocates_once_per_rotating_buffer_then_settles() {
    let mut rig = Flow2::new();
    let shape: Vec<[u32; 4]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(40, 20, k(2, 1)).delta;
            [
                d.alloc_compose_dst,
                d.alloc_resample_dst,
                d.alloc_xmap,
                d.alloc_mask,
            ]
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            [1, 1, 1, 1],
            [0, 1, 0, 1],
            [0, 1, 0, 1],
            [0, 0, 0, 1],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ],
        "立ち上がりが「実体ごとに一度」の形から外れている（Requirement 3.2）"
    );
    // 実体の本数として読み直す（形の丸暗記ではなく由来を主張する）。
    let total = *rig.budget.cumulative();
    assert_eq!(
        total.alloc_resample_dst, 3,
        "表示バッファの起こし直しはキャッシュが保持する本数ぶんのはず"
    );
    assert_eq!(
        total.alloc_mask, 4,
        "マスクはキャッシュの 3 本＋輪番の空き 1 枚ぶん"
    );
    assert_eq!(
        total.alloc_compose_dst, 1,
        "非恒等 k の合成先席は 1 本で固定"
    );
    assert_eq!(total.alloc_xmap, 1, "作業席は 1 本");
}

/// Requirement 3.1: 恒等 k（常設席と表示バッファの swap 経路）でも定常状態は完全ゼロ。
///
/// swap 経路はバッファが役割を交代し続けるため、**立ち上がりに本数ぶんの確保が要る**
/// （各バッファが一度ずつ外形へ伸びる）。それが済めば以後は 1 件も確保しない。
#[test]
fn a_steady_run_with_identity_scale_allocates_nothing_at_all() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20, identity());
    }
    let warm = *rig.budget.cumulative();

    for i in 0..8 {
        let seen = rig.apply(40, 20, identity());
        assert_eq!(
            seen.delta,
            BudgetDelta::default(),
            "恒等 k の定常状態 {i} 回目で確保が起きた"
        );
    }
    assert_eq!(*rig.budget.cumulative(), warm, "恒等 k で累積が伸びた");
    // 交代する 4 本ぶん（キャッシュの 3 本＋合成先席 1 本）。恒等 k では全ての伸長が合成先席の
    // 側で起きるため `alloc_compose_dst` に集まる。リサンプル経路を通らないので表示バッファ席・
    // 作業領域は動かない。
    assert_eq!(
        warm.alloc_compose_dst, 4,
        "恒等 k の立ち上がりは交代する 4 本ぶんのはず: {warm:?}"
    );
    assert_eq!(
        warm.alloc_resample_dst, 0,
        "恒等 k はリサンプル先を持たない"
    );
    assert_eq!(warm.alloc_xmap, 0, "恒等 k は作業領域に触れない");
}

/// Requirement 3.1 の再利用が**名目でなく実体**であること（ポインタの集合）。
///
/// 定常状態では合成先席は**同じ 1 本**が貸し出され続け、表示バッファは**キャッシュが保持する
/// 3 本の中だけ**を回る（容量 3・要件 7.1）。本数が増えれば確保し直しており、1 本へ潰れれば
/// 追い出しと回収が噛み合っていない。
///
/// # 計数の檻と対で読む
///
/// 実体を差し替える変異は、確保の観測が実体の容量差になった今では計数側でも捕まる（新しい実体は
/// 容量 0 から伸びるため）。本檻はその独立な裏取りであり、[`fill_extent`] が転写元を先に確保する
/// ため解放された席の番地は転写元に取られる（同じ番地を引き当てて偽の緑になる余地が小さい）。
#[test]
fn the_steady_state_keeps_handing_back_the_same_allocations() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20, k(2, 1));
    }

    let first = rig.apply(40, 20, k(2, 1));
    let mut displays = vec![first.display];
    for i in 0..7 {
        let seen = rig.apply(40, 20, k(2, 1));
        assert_eq!(
            seen.native, first.native,
            "{i} 回目の合成先席が別の実体になった（常設席になっていない）"
        );
        displays.push(seen.display);
    }
    let mut distinct = displays.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        3,
        "表示バッファが回る実体はキャッシュが保持する 3 本のはず（確保し直している／潰れている）"
    );
    assert!(
        displays.len() > distinct.len(),
        "同じ器が一度も再登場しない（回収した容量で回っていない）"
    );
}

/// Requirement 3.2: 寸法拡大は **確保対象ごとに一度だけ**計数され、その後また 0 に戻る。
///
/// # 代金は実体の本数ぶんの適用に分かれる（in-place 再生成＝確保なし、ではない）
///
/// `AlphaMask::regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` であり、詰め長が
/// そのバッファの容量を超えれば `resize` は**確実に確保する**（40×20 → 60×30・k=2/1 では
/// 表示 80×40 → 120×60 で詰め長 400 → 900 バイト）。回る実体は 1 適用ずつずれて再生成される
/// ため、全部が伸びるまでに本数ぶんの適用がかかり、各適用でちょうど 1 件ずつ計数される。
///
/// 以前この檻は拡大時の `alloc_mask` を 0 と書いていた。それは「in-place なら確保なし」という
/// 誤った決め打ちを正解として焼き付けたもので、**実測 400→900 バイトの成長がどこにも数えられて
/// いなかった**（レビューが検出）。design.md §Error Handling の「黙って確保しない」に反する
/// 唯一の席だった。
///
/// 立ち上がりと同じ形が出るのは偶然ではない——寸法が変われば「その寸法では初めて」なので、
/// どの実体も一度ずつ伸び直す（表示バッファ 3 本・マスク 4 本・合成先席 1 本・作業席 1 本）。
#[test]
fn a_larger_extent_costs_exactly_one_allocation_per_buffer_then_returns_to_zero() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20, k(2, 1));
    }

    let shape: Vec<[u32; 4]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(60, 30, k(2, 1)).delta;
            [
                d.alloc_compose_dst,
                d.alloc_resample_dst,
                d.alloc_xmap,
                d.alloc_mask,
            ]
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            [1, 1, 1, 1],
            [0, 1, 0, 1],
            [0, 1, 0, 1],
            [0, 0, 0, 1],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ],
        "寸法拡大の代金が「実体ごとに一度」の形から外れている（Requirement 3.2）"
    );
}

/// Requirement 3.2 の要の檻: **より小さい寸法の後続要求で容量が縮まない**。
///
/// 本 spec が 3 度踏んだ罠（task 4.1／4.2／5.1）への対策そのものである。同一寸法の反復だけを
/// 見る檻は「毎回まっさらに確保する」実装を素通りさせる（毎回同じ容量に着地するため）。
/// 縮小 → 再拡大（元の最大寸へ戻る）の往復で 1 件も計数されないことが、席が容量を保持し続けて
/// いることの唯一の証拠になる。
#[test]
fn a_smaller_extent_and_the_regrowth_allocate_nothing() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(60, 30, k(2, 1));
    }

    // 縮小: 容量は足りている＝再確保は要らない。
    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(20, 10, k(2, 1)).delta,
            BudgetDelta::default(),
            "縮小 {i} 回目で確保が起きた（席の容量が縮んでいる＝往復確保を作る実装）"
        );
    }
    // 再拡大: 既に到達済みの寸法ゆえ容量が残っている＝やはり再確保は要らない。
    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(60, 30, k(2, 1)).delta,
            BudgetDelta::default(),
            "元の最大寸への再拡大 {i} 回目で確保が起きた（容量が保持されていない）"
        );
    }
}

/// design.md §Error Handling: **回収が成立しなかったら黙って確保せず必ず計数へ現す**。
///
/// キャッシュが無効化されると表が空になり、再び満杯になるまで回収エントリを得られない。ここで
/// 無言のまま確保すると「定常アロケーション 0」が観測上は成立しているのに実際は確保している、
/// という隠れた縮退になる。無効化が捨てるのは表示バッファとマスクの原子対**まるごと**（3 対）
/// ゆえ、代金は複数の適用に分かれて現れる。実測の形は
/// `[0,1,0,0]` → `[0,1,0,1]` → `[0,1,0,1]` → `[0,0,0,1]` → 以後 0 である。読み方:
///
/// - `alloc_resample_dst` が 3 回＝捨てられた表示バッファ 3 本を起こし直すぶん
/// - `alloc_mask` が 3 回＝捨てられたマスク 3 枚ぶん（無効化の直前に輪番が握っていた空き 1 枚は
///   生き残るので、1 適用目のマスクだけが確保なしで済む）
/// - 合成先席と作業席は無効化の影響を受けない（キャッシュの外に在る）
///
/// **代金が複数の適用に分かれること自体を固定する**のは、一部を黙って確保する実装を通さない
/// ためである。
#[test]
fn a_failed_recycle_is_counted_never_swallowed() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20, k(2, 1));
    }

    // 回収元を断つ（アトラス再構築・ghost 再読込に相当）。
    rig.cache.invalidate_all();
    let shape: Vec<[u32; 4]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(40, 20, k(2, 1)).delta;
            [
                d.alloc_compose_dst,
                d.alloc_resample_dst,
                d.alloc_xmap,
                d.alloc_mask,
            ]
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            [0, 1, 0, 0],
            [0, 1, 0, 1],
            [0, 1, 0, 1],
            [0, 0, 0, 1],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ],
        "回収不成立の代金が計数へ現れていない（黙って確保している）"
    );
}

/// design.md D3: マスク輪番は**キャッシュの 3 本＋空き 1 枚**で回り、定常状態では in-place
/// 再生成だけになる。
///
/// 立ち上がりは 4 適用ぶん（4 枚の確保）。以後は空きスロットのマスクが単独所有なので確保は
/// 起きない。実体が **ちょうど 4 種類**で回ることを同時に固定する（計数だけでは「毎回確保して
/// いるのに数え忘れている」実装と区別できない）。
#[test]
fn the_mask_rotation_settles_into_in_place_regeneration() {
    let mut rig = Flow2::new();

    let mut seen = Vec::new();
    for i in 0..WARMUP {
        let now = rig.apply(40, 20, k(2, 1));
        assert_eq!(
            now.delta.alloc_mask, 1,
            "立ち上がりの {i} 枚目のマスクが確保されていない"
        );
        seen.push(now.mask);
    }
    let known = seen.clone();

    for i in 0..8 {
        let now = rig.apply(40, 20, k(2, 1));
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

/// design.md D3／§Error Handling: **単独所有が不成立なら確保し、必ず計数へ現す**。
///
/// 下流以外の第三者が現行マスクの `Arc` を握り続けると、それが輪番の空きスロットへ回ってきた
/// 適用で `Arc::get_mut` が成立しない。ここで無言のまま新規確保すると、計数上は定常ゼロのまま
/// 実際には毎回確保する縮退が作れてしまう。
///
/// 余分な複製を握った適用の **4 つ後**（そのマスクが空きスロットへ回る適用）でちょうど 1 件
/// 計数され、複製を手放せば輪番が自力で立ち直ることまで見る。「4 つ後」は回る実体の本数
/// （キャッシュの 3 本＋空き 1 枚）そのもので、容量 1 の頃の「2 つ後」がここへ移った。
#[test]
fn a_mask_slot_that_is_not_uniquely_owned_allocates_and_is_counted() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP + 4 {
        rig.apply(40, 20, k(2, 1));
    }

    // この適用のマスクを第三者として握る（＝以後このマスクは単独所有にならない）。
    rig.apply(40, 20, k(2, 1));
    let hostage = Arc::clone(rig.resource.as_ref().expect("下流へ供給済み"));

    // 人質のマスクが空きスロットへ回るまでの適用は、他の実体が単独所有ゆえ in-place で通る。
    let held: Vec<u32> = (0..WARMUP + 4)
        .map(|_| rig.apply(40, 20, k(2, 1)).delta.alloc_mask)
        .collect();
    assert_eq!(
        held,
        vec![0, 0, 0, 1, 0, 0, 0, 0],
        "単独所有の不成立がちょうど 1 件・輪番の一巡後に現れるはず（黙って確保していないか）"
    );

    // 人質を手放せば輪番は自力で立ち直る。
    drop(hostage);
    for i in 0..WARMUP + 4 {
        assert_eq!(
            rig.apply(40, 20, k(2, 1)).delta.alloc_mask,
            0,
            "複製を手放した後の {i} 回目で輪番が立ち直っていない"
        );
    }
}

/// 表示バッファ席（A2/A6）は回収エントリを**素通しで**返す（design.md D2⑵）。
///
/// # ポインタ同一性では足りない（本檻の存在理由）
///
/// 回収エントリを捨てて新品を返す変異は、捨てた直後の確保が**同じ番地を引き当てる**ため
/// ポインタ同一性の檻を素通りし得る。計数の檻も素通りする——到達済み寸法は器側の記憶であり、
/// 実体が入れ替わったことを知らないからである（レビューの変異検査で実測確認）。
/// 「回収した外形と長さがそのまま返る」ことだけが、素通しを実体で言い切れる観測になる。
/// 新品は必ず 0×0 から始まるため、この檻は番地の巡り合わせに左右されない。
#[test]
fn the_display_buffer_hands_back_the_recycled_allocation_unchanged() {
    let mut budget = FrameBudget::new();

    // 回収エントリ相当（外形は「新品の 0×0」と紛れない値にする）。
    let mut composed = ComposedSurface::default();
    fill_extent(&mut composed, 7, 3);
    let bytes = composed.bytes().len();
    let mask = Arc::new(AlphaMask::from_pbgra32(
        composed.bytes(),
        composed.width(),
        composed.height(),
        composed.stride(),
    ));
    let mask_ptr = Arc::as_ptr(&mask);

    let (display, retired) = budget.display_buffer(Some(CacheEntry {
        composed,
        mask,
        native: (7, 3),
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

    // 回収不成立は新品（0×0）から始まり、マスクも渡されない。
    let (fresh, none) = budget.display_buffer(None);
    assert_eq!(fresh.width(), 0, "回収不成立の表示バッファは空から始まる");
    assert_eq!(
        fresh.bytes().len(),
        0,
        "回収不成立の表示バッファは空から始まる"
    );
    assert!(none.is_none(), "回収不成立で輪番へ返すマスクは無い");
}

/// 合成先の常設席（A1）は初回だけ確保し、以後は同じ実体を貸し続ける。
///
/// 席を直接叩く最小の檻（Flow2 を通さない）。閉包から実体のポインタを持ち出して同一性で見る。
#[test]
fn the_compose_destination_seat_is_allocated_once_and_then_lent_again() {
    let mut budget = FrameBudget::new();

    let first = budget.native_scratch(|dst| {
        fill_extent(dst, 40, 20);
        dst.bytes().as_ptr()
    });
    assert_eq!(
        budget.take_delta().alloc_compose_dst,
        1,
        "常設席の初回伸長が計数されていない"
    );

    for i in 0..6 {
        let again = budget.native_scratch(|dst| {
            fill_extent(dst, 40, 20);
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

/// **作業領域は器が持つ席そのものである**（使い捨てでも毎回新品でもない）。
///
/// # この檻が殺す変異クラス（レビューの変異検査が実測した 2 種）
///
/// 計数を「出力幅の高水位」で代用していた以前の形は、作業領域を一度も観測していなかった。
/// 次の 2 種はどちらも 180 本の檻を 1 本も赤にしなかった:
///
/// | 変異 | 何が起きるか | 本檻のどの節が赤になるか |
/// |---|---|---|
/// | ⑶a `self.scratch = ResampleScratch::default();` を `resample_with` の直前に置く | 席の容量が毎回 0 から出力幅へ伸びる＝毎適用で確保 | 節⑵（反復で 0 のはずが 1 になる） |
/// | ⑶b 使い捨てのローカル `ResampleScratch` を作って `resample_with` へ渡す（席は不使用） | 席の容量が永久に 0 のまま動かない＝一度も計数されない | 節⑴（初回の 1 件が 0 になる） |
///
/// 是正後は [`ResampleScratch::capacity`] を呼び出しの前後で読み比べる形になっており、
/// **観測対象が席そのもの**である。⑶a は毎回の再確保として、⑶b は「席が一度も使われて
/// いない」として、どちらも計数へ現れる。
///
/// 節⑶（縮小 → 再拡大）は容量の持ち越しを見る従来からの節で、席の容量を毎回捨てる実装を殺す。
///
/// [`ResampleScratch::capacity`]: areka_emo_compose::scale::ResampleScratch::capacity
#[test]
fn the_resample_scratch_is_the_budgets_own_seat_not_a_throwaway() {
    let mut rig = Flow2::new();

    // ⑴ 初回のリサンプルは席の写像表を 0 から出力幅ぶんへ伸ばす＝必ず 1 件計数される。
    //    席を使わない実装（使い捨てローカル）はここで 0 になる。
    assert_eq!(
        rig.apply(40, 20, k(2, 1)).delta.alloc_xmap,
        1,
        "初回のリサンプルで席の写像表が伸びていない（席そのものを使っていない）"
    );

    // ⑵ 同じ出力幅の反復では 1 件も増えない。
    //    席を毎回まっさらに起こす実装はここで毎回 1 になる。
    for i in 0..6 {
        assert_eq!(
            rig.apply(40, 20, k(2, 1)).delta.alloc_xmap,
            0,
            "反復 {i} 回目で作業領域を確保し直した（席が持ち越されていない）"
        );
    }

    // ⑶ 縮小 → 元の幅への再拡大でも増えない（席が容量を保持している）。
    for w in [10, 40] {
        assert_eq!(
            rig.apply(w, 20, k(2, 1)).delta.alloc_xmap,
            0,
            "出力幅 {w} の要求で作業領域が再確保された（容量が縮んでいる）"
        );
    }

    assert_eq!(
        rig.budget.cumulative().alloc_xmap,
        1,
        "作業領域の確保は初回の 1 件だけのはず"
    );
}

/// 恒等 k（swap 経路）の寸法拡大は交代する 4 本ぶん＝4 適用に分かれて 1 件ずつ計数される。
///
/// 恒等 k では合成先席と表示バッファが役割を交代し続けるため、確保対象は**キャッシュの 3 本＋
/// 合成先席 1 本**の 4 本ある。寸法が変われば 4 本とも伸びる必要があり、代金は 4 適用に分かれる
/// （Requirement 3.2 の「一度だけ」＝**確保対象ごとに一度**）。5 適用目からはまた完全ゼロへ戻る。
///
/// リサンプル経路を通らないので `alloc_resample_dst`・`alloc_xmap` は全適用で 0 のままである。
#[test]
fn an_identity_scale_dimension_change_costs_one_allocation_per_swapped_buffer() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20, identity());
    }

    let seen: Vec<BudgetDelta> = (0..WARMUP + 2)
        .map(|_| rig.apply(60, 30, identity()).delta)
        .collect();
    let composed: Vec<u32> = seen.iter().map(|d| d.alloc_compose_dst).collect();
    assert_eq!(
        composed,
        vec![1, 1, 1, 1, 0, 0],
        "恒等 k の寸法拡大は交代する 4 本ぶんで打ち止めのはず: {seen:?}"
    );
    for (i, delta) in seen.iter().enumerate() {
        assert_eq!(
            delta.alloc_resample_dst, 0,
            "{i}: 恒等 k はリサンプル先を持たない"
        );
        assert_eq!(delta.alloc_xmap, 0, "{i}: 恒等 k は作業領域に触れない");
    }
    // マスクも輪番 4 本ぶん（詰め長 100 → 240 バイト）で、その後は 0 に戻る。
    let masks: Vec<u32> = seen.iter().map(|d| d.alloc_mask).collect();
    assert_eq!(
        masks,
        vec![1, 1, 1, 1, 0, 0],
        "マスク輪番 4 本ぶんの成長: {seen:?}"
    );
}

/// 恒等 k で回収が不成立になったときの代金は **`ComposeDst` に・1 適用遅れて**現れる。
///
/// # design.md:330 の宣言との食い違いを実挙動の側で固定する
///
/// design.md:330 は `alloc_resample_dst` を「A2: 表示バッファの新規確保/成長（**リサイクル
/// 不成立を含む**）」と宣言している。ところが恒等 k の swap 経路では、回収不成立の適用そのもの
/// では確保が 1 件も起きない——`display_buffer` が返した空のバッファはそのまま合成先席へ
/// 送り込まれ、代わりに合成済みの中身（容量つき）が表示バッファへ出るからである。空になった
/// のは**合成先席**であり、その伸長は次の適用の合成で起きて `alloc_compose_dst` に載る。
///
/// 代金は取りこぼされていない（黙って確保していない）が、載る**フィールドと適用**が design.md
/// の宣言と一致しない。これは実装の欠陥ではなく design.md の記述が swap 経路を織り込んで
/// いないことによる。本檻は**実挙動を正**として固定する（フィールドの割り当てを変えるのは
/// 判定スクリプトとの契約変更であり、実装判断で行える範囲を超える）。
///
/// 実測の形は `[0,0,0,0]` → `[1,0,0,1]` → `[1,0,0,1]` → `[1,0,0,1]` → 以後 0 である。
/// 3 対が捨てられたので代金は 3 適用ぶんあり、いずれも 1 適用遅れて合成先席へ載る。
#[test]
fn an_identity_scale_failed_recycle_lands_on_the_compose_seat_one_apply_later() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(40, 20, identity());
    }

    rig.cache.invalidate_all();

    let shape: Vec<[u32; 4]> = (0..WARMUP + 3)
        .map(|_| {
            let d = rig.apply(40, 20, identity()).delta;
            [
                d.alloc_compose_dst,
                d.alloc_resample_dst,
                d.alloc_xmap,
                d.alloc_mask,
            ]
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            [0, 0, 0, 0],
            [1, 0, 0, 1],
            [1, 0, 0, 1],
            [1, 0, 0, 1],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
            [0, 0, 0, 0],
        ],
        "恒等 k の回収不成立の代金が合成先席とマスクへ現れていない（黙って確保している）"
    );
}

/// リサンプル作業領域の席（A3）は出力幅の最大へ一度だけ到達し、以後は成長しない。
///
/// 計数は席の容量差（[`ResampleScratch::capacity`] の呼び出し前後）で決まるため、**より小さい
/// 出力幅の呼び出しで増えない**ことと、元の最大幅へ戻しても増えないことが、席が容量を保持して
/// いることの観測になる。
///
/// [`ResampleScratch::capacity`]: areka_emo_compose::scale::ResampleScratch::capacity
#[test]
fn the_resample_scratch_seat_reaches_its_width_once_and_never_regrows() {
    let mut rig = Flow2::new();

    rig.apply(40, 20, k(2, 1));
    assert_eq!(
        rig.budget.cumulative().alloc_xmap,
        1,
        "作業領域の初回確保が計数されていない"
    );

    // 同一幅の反復では増えない。
    for _ in 0..3 {
        assert_eq!(rig.apply(40, 20, k(2, 1)).delta.alloc_xmap, 0);
    }
    // 出力幅の拡大でちょうど 1 回。
    assert_eq!(
        rig.apply(80, 20, k(2, 1)).delta.alloc_xmap,
        1,
        "出力幅の拡大が計数されていない"
    );
    // 縮小と再拡大では 1 件も増えない（容量を保持している証拠）。
    for w in [10, 40, 80, 20, 80] {
        assert_eq!(
            rig.apply(w, 20, k(2, 1)).delta.alloc_xmap,
            0,
            "出力幅 {w} の要求で作業領域が再確保された（容量が縮んでいる）"
        );
    }
    assert_eq!(
        rig.budget.cumulative().alloc_xmap,
        2,
        "作業領域の確保は初回と幅拡大の 2 件だけのはず"
    );
}
