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
//! # 席は 2 つ・発生点も 2 つ（裁定 D・T-N7）
//!
//! [`FrameBudget`] は計数だけでなく再利用席（合成先の常設席・マスクの輪番）を持つ。ゆえに計数は
//! 「呼び手が確保したと申告した回数」から**「席の再利用が成立せず結局確保した回数」**へ意味が
//! 移った（design.md §FrameBudget・D2/D3/D6）。上の 4 点の意味論はそのまま生きている。
//!
//! CPU で画素を拡大する段が経路から消えた（拡大は wintf の描画経路の変換行列が掛ける）ため、
//! リサンプル出力の席と写像表の席は**器ごと無くなった**。残る発生点は合成先とマスクの 2 つである。
//!
//! 本ファイルが席について固定するのは次の 3 点である:
//!
//! - **定常状態は完全ゼロ**（Requirement 3.1）: 初回確保が済んだ後の反復では全フィールドが 0
//! - **寸法変化は席ごとに一度だけ**（Requirement 3.2）: 変化した適用で 1 増え、以後また 0
//! - **黙って確保しない**（design.md §Error Handling）: 席が期待どおり再利用できない境界
//!   （回収不成立・マスク輪番の単独所有が不成立）でも確保は必ず計数に現れる
//!
//! # 回る実体の本数（キャッシュ容量 3・要件 7.1）と**拡大率で分岐しない交代**
//!
//! [`Flow2`] は **4 種類のキーを巡回**させてミス経路を演じる（巡回長 4 > 容量 3 ゆえ毎適用が
//! ミスし、毎適用で追い出しが 1 件成立する＝本番の「毎コマ引き当て外れ」と同じ形）。
//!
//! 合成先席と表示バッファの交代（[`FrameBudget::swap_native_scratch`]）は `show.rs` のミス経路で
//! **無条件に**走る（要件 1.3／7.3）。ゆえに拡大率での場合分けは 1 つも無く、
//!
//! - **表示バッファと合成先席**: キャッシュの 3 本＋席 1 本＝**4 本**が役割を交代しながら回る
//! - **マスク**: キャッシュの 3 本＋輪番の空き 1 枚＝**4 本**
//!
//! 立ち上がり（暖機）に要する適用数は本数ぶんであり、[`WARMUP`] はそれに合わせてある。
//!
//! # 「同じ寸法の反復」だけでは再利用を主張できない（本 spec が 3 度踏んだ罠）
//!
//! 毎回まっさらに確保する実装は、同じ寸法の反復では毎回**同じ容量**に着地するため、容量比較の
//! 檻を素通りする。変異を捕まえたのはいずれも
//! ⑴**より小さい寸法の後続要求で容量が縮まないこと** ⑵**ポインタ同一性**の 2 種だけだった。
//! ゆえに本ファイルの再利用主張は必ずこの 2 種のどちらかで書く。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! 本檻は時刻にも実行速度にも一切触れない（回数・ポインタ・寸法のみ）。純 x64 の常設テストで
//! あり、環境変数ゲートも `#[ignore]` も持たない（Requirement 6.3）。

use super::*;

use areka_emo_compose::{BindSet, PatternState};

use crate::cache::ComposeCache;

/// 発生点の全数（design.md §FrameBudget の席一覧と 1:1・T-N7）。
///
/// 2 つから増減したら perf サマリ行のフィールド集合が変わり、判定スクリプトの契約が壊れる。
#[test]
fn every_allocation_site_is_enumerated() {
    assert_eq!(
        AllocSite::ALL.len(),
        2,
        "確保発生点は 2 つ（合成先・当たり判定マスク）"
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
/// 各発生点を 1 回だけ申告し、⑴対応する**名前つきフィールド**が 1 になる ⑵もう一方のフィールドが
/// 0 のままである、を全発生点について確認する。フィールドの割り当てを入れ替えると
/// （例: `Mask` が `alloc_compose_dst` を増やす）、この檻は必ず赤になる。
#[test]
fn each_site_increments_only_its_own_field() {
    for site in AllocSite::ALL {
        let mut budget = FrameBudget::new();
        budget.note_alloc(site);
        let delta = budget.take_delta();

        // 名前つきフィールドで直接受ける（`count` の実装が壊れていても素通ししない）。
        let named = match site {
            AllocSite::ComposeDst => delta.alloc_compose_dst,
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
/// - 適用 3: 合成先 1 回 → 増分に現れ、累積は 1・2 を保ったまま伸びる
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
            alloc_mask: 2,
        },
        "累積は増分の取り出しでリセットされないこと"
    );

    // 適用 3: 合成先席の再確保（寸法変化の形）。
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
/// これは「累積だけ・増分だけ」を数える片肺実装を殺す（片方が動かなければ総和が食い違う）。
/// 判定スクリプトは行ごとの増分を、テストは累積を根拠にするため、両者が同じ事実を指していない
/// 状態は静かな誤診断を生む。
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
// 席の再利用（Requirement 3.1/3.2・design.md D2/D3・Flow 2）
// ═══════════════════════════════════════════════════════════════════════════

/// 席の外形を `w×h` へ合わせる。
///
/// 本物の合成（`compose_into` → `blit`）が合成先席に対して通すのと**同じメソッド**を呼ぶ。
/// ゆえに確保の起こり方と容量の引き継ぎ方——本檻が観測する唯一のもの——は本番と一致する。
fn fill_extent(dst: &mut ComposedSurface, w: u32, h: u32) {
    dst.resize_and_clear(w, h);
}

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

/// design.md Flow 2（apply_show のミス経路）を席の側から演じる装置。
///
/// `show.rs` の順序を手で踏む——合成（常設席）→ 容量回収（合成成功後に限る）→ 表示バッファ席 →
/// **交代**（拡大率に依らず常に swap）→ マスク輪番 → 表示バッファと `Arc` マスクを原子対で
/// 挿入 → 下流供給（`AlphaMaskResource::set_shared` 相当）。
///
/// `resource` を持つのが要点である。下流が現行マスクを保持し続けるからこそ現行マスクの
/// 参照数は 2 になり、**前々回のマスクだけが単独所有（＝in-place 再生成できる）**という
/// 輪番の前提（design.md D3）が本物になる。ここを省くと輪番の主張が空虚になる。
///
/// # 毎回ミスさせるにはキーを 4 種類回す（キャッシュ容量 3・要件 7.1）
///
/// 本装置は**ミス経路**を演じる。同じキーを繰り返し挿入する形では表が 1 件で埋まらず、追い出しが
/// 一度も起きない（本番なら同じキーは 2 回目からヒットするので、ミス経路の反復として成立しない）。
/// ゆえに **surface id を 4 種類**巡回させる: 巡回長 4 > 容量 3 なので、LRU では入ってくるキーが
/// 必ず 1 手前で追い出されており、**毎適用がミスかつ毎適用で追い出しが 1 件成立する**——これが
/// 本番の「毎コマ引き当て外れ」に対応する定常状態である。
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
        // (1) 合成: 常設席へ native 外形の結果を書く（`compose_into` 相当）。
        self.budget.native_scratch(|dst| {
            fill_extent(dst, native_w, native_h);
        });

        // (2) 容量回収は**合成成功後にのみ**（Flow 2 の規律）。
        let recycled = self.cache.take_recycled();

        // (3) 表示バッファ席と、輪番へ返すマスク。
        let (mut display, retired) = self.budget.display_buffer(recycled);

        // (4) 交代（複写も確保も起きない）。**拡大率に依らず常にこの経路**（要件 1.3／7.3）。
        self.budget.swap_native_scratch(&mut display);
        let display_ptr = display.bytes().as_ptr();
        let native_ptr = self.budget.native_scratch_ptr();

        // (5) マスク輪番（in-place 再生成・単独所有が不成立なら確保＋計数）。原寸バイト由来。
        let mask = self.budget.regenerate_mask(
            retired,
            display.bytes(),
            display.width(),
            display.height(),
            display.stride(),
        );
        let mask_ptr = Arc::as_ptr(&mask);

        // (6) 原寸の表示バッファと `Arc` マスクを原子対で挿入（表示記録は GPU 不要の空リスト）。
        self.cache.insert(
            surface_id,
            BindSet::default(),
            PatternState::default(),
            display,
            Arc::clone(&mask),
            wintf::ecs::GraphicsCommandList::empty(),
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

/// Requirement 3.1: 定常状態は **2 フィールドとも増分 0**（完全ゼロ）。
///
/// ウォームアップ（交代で回る 4 本・マスク 4 枚の立ち上げ）が済んだ後は、同一寸法の反復で
/// 1 件の確保も起きない。累積も据え置きであることを同時に見る（増分だけ 0 で累積が伸びる
/// 片肺実装を通さない）。
///
/// 立ち上がりの内訳（交代する 4 本＋マスク 4 本）も併せて主張する——交代経路では全ての伸長が
/// 合成先席の側で起きるため、バッファの代金は `alloc_compose_dst` に集まる。
#[test]
fn a_steady_run_allocates_nothing_at_all() {
    let mut rig = Flow2::new();
    // ウォームアップ（回る実体の本数ぶん）。
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
    assert_eq!(
        warm.alloc_compose_dst, 4,
        "立ち上がりは交代する 4 本（キャッシュの 3 本＋合成先席 1 本）ぶんのはず: {warm:?}"
    );
    assert_eq!(
        warm.alloc_mask, 4,
        "マスクはキャッシュの 3 本＋輪番の空き 1 枚ぶんのはず: {warm:?}"
    );
}

/// Requirement 3.2 観測完了（**立ち上がりの形を丸めない**）: 確保は「実体ごとに一度」の
/// 形で現れて 0 へ落ちる。
///
/// 実測の形は `[1,1]` が 4 適用ぶん、以後 0 である。読み方:
///
/// - `alloc_compose_dst` が 4 回＝**交代で回る本数**（キャッシュの 3 本＋合成先席 1 本）。
///   キャッシュに空きがあるあいだは回収が成立しないので、4 本が 1 適用ずつ順に起こされる
/// - `alloc_mask` が 4 回＝キャッシュの 3 本＋輪番の空き 1 枚
///
/// 立ち上がりを 1 適用へ丸めるとこれらの成長が観測の外へ落ち、黙って確保する実装を通してしまう。
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
    // 実体の本数として読み直す（形の丸暗記ではなく由来を主張する）。
    let total = *rig.budget.cumulative();
    assert_eq!(
        total.alloc_compose_dst, 4,
        "交代で回る本数はキャッシュの 3 本＋合成先席 1 本のはず"
    );
    assert_eq!(
        total.alloc_mask, 4,
        "マスクはキャッシュの 3 本＋輪番の空き 1 枚ぶん"
    );
}

/// Requirement 3.1 の再利用が**名目でなく実体**であること（ポインタの集合と交代の関係）。
///
/// 定常状態では合成先席と表示バッファは**同じ 4 本**を役割を替えながら巡る（キャッシュの 3 本＋
/// 席 1 本・要件 7.1）。本数が増えれば確保し直しており、減れば交代が成立していない。
///
/// # 交代関係は番地の偶然一致に強い唯一の主張
///
/// 「**前回の合成先席が今回の表示バッファになる**」は、同時に生きている 2 つの実体の役割
/// 入れ替えを主張する。解放も再確保も介在しないため、番地の偶然一致が入り込む余地が無い。
/// `swap_native_scratch` を複写へ戻す変異はここで死ぬ。
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

/// Requirement 3.2: 寸法拡大は **確保対象ごとに一度だけ**計数され、その後また 0 に戻る。
///
/// # 代金は実体の本数ぶんの適用に分かれる（in-place 再生成＝確保なし、ではない）
///
/// `AlphaMask::regenerate_from_pbgra32` は `clear`＋`resize(詰め長, 0)` であり、詰め長が
/// そのバッファの容量を超えれば `resize` は**確実に確保する**（40×20 → 60×30 で詰め長
/// 100 → 225 バイト）。回る実体は 1 適用ずつずれて再生成されるため、全部が伸びるまでに本数ぶんの
/// 適用がかかり、各適用でちょうど 1 件ずつ計数される。
///
/// 以前この檻は拡大時の `alloc_mask` を 0 と書いていた。それは「in-place なら確保なし」という
/// 誤った決め打ちを正解として焼き付けたもので、**実測の成長がどこにも数えられていなかった**
/// （レビューが検出）。design.md §Error Handling の「黙って確保しない」に反する唯一の席だった。
///
/// 立ち上がりと同じ形が出るのは偶然ではない——寸法が変われば「その寸法では初めて」なので、
/// どの実体も一度ずつ伸び直す（交代する 4 本・マスク 4 本）。
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

/// Requirement 3.2 の要の檻: **より小さい寸法の後続要求で容量が縮まない**。
///
/// 本 spec が 3 度踏んだ罠への対策そのものである。同一寸法の反復だけを見る檻は「毎回まっさらに
/// 確保する」実装を素通りさせる（毎回同じ容量に着地するため）。縮小 → 再拡大（元の最大寸へ戻る）
/// の往復で 1 件も計数されないことが、席が容量を保持し続けていることの唯一の証拠になる。
#[test]
fn a_smaller_extent_and_the_regrowth_allocate_nothing() {
    let mut rig = Flow2::new();
    for _ in 0..WARMUP {
        rig.apply(60, 30);
    }

    // 縮小: 容量は足りている＝再確保は要らない。
    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(20, 10).delta,
            BudgetDelta::default(),
            "縮小 {i} 回目で確保が起きた（席の容量が縮んでいる＝往復確保を作る実装）"
        );
    }
    // 再拡大: 既に到達済みの寸法ゆえ容量が残っている＝やはり再確保は要らない。
    for i in 0..WARMUP {
        assert_eq!(
            rig.apply(60, 30).delta,
            BudgetDelta::default(),
            "元の最大寸への再拡大 {i} 回目で確保が起きた（容量が保持されていない）"
        );
    }
}

/// design.md §Error Handling: **回収が成立しなかったら黙って確保せず必ず計数へ現す**。
///
/// キャッシュが無効化されると表が空になり、再び満杯になるまで回収エントリを得られない。ここで
/// 無言のまま確保すると「定常アロケーション 0」が観測上は成立しているのに実際は確保している、
/// という隠れた縮退になる。
///
/// # 代金は `ComposeDst` に・1 適用遅れて現れる（交代経路の形）
///
/// 回収不成立の適用そのものでは確保が 1 件も起きない——`display_buffer` が返した空のバッファは
/// そのまま合成先席へ送り込まれ、代わりに合成済みの中身（容量つき）が表示バッファへ出るからで
/// ある。空になったのは**合成先席**であり、その伸長は次の適用の合成で起きて `alloc_compose_dst`
/// に載る。代金は取りこぼされていない（黙って確保していない）が、**1 適用遅れる**。
///
/// 実測の形は `[0,0]` → `[1,1]` → `[1,1]` → `[1,1]` → 以後 0 である。無効化が捨てるのは表示
/// バッファとマスクの原子対**まるごと**（3 対）ゆえ代金は 3 適用ぶんあり、初回のマスクだけは
/// 無効化の直前に輪番が握っていた空き 1 枚が生き残るので確保なしで済む。
///
/// **代金が複数の適用に分かれること自体を固定する**のは、一部を黙って確保する実装を通さない
/// ためである。
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

/// design.md D3／§Error Handling: **単独所有が不成立なら確保し、必ず計数へ現す**。
///
/// 下流以外の第三者が現行マスクの `Arc` を握り続けると、それが輪番の空きスロットへ回ってきた
/// 適用で `Arc::get_mut` が成立しない。ここで無言のまま新規確保すると、計数上は定常ゼロのまま
/// 実際には毎回確保する縮退が作れてしまう。
///
/// 余分な複製を握った適用の **4 つ後**（そのマスクが空きスロットへ回る適用）でちょうど 1 件
/// 計数され、複製を手放せば輪番が自力で立ち直ることまで見る。「4 つ後」は回る実体の本数
/// （キャッシュの 3 本＋空き 1 枚）そのものである。
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

    // 人質を手放せば輪番は自力で立ち直る。
    drop(hostage);
    for i in 0..WARMUP + 4 {
        assert_eq!(
            rig.apply(40, 20).delta.alloc_mask,
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
/// ポインタ同一性の檻を素通りし得る。「回収した外形と長さがそのまま返る」ことだけが、素通しを
/// 実体で言い切れる観測になる。新品は必ず 0×0 から始まるため、この檻は番地の巡り合わせに
/// 左右されない。
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
/// 席を直接叩く最小の檻（Flow2 を通さない＝交代が挟まらない）。閉包から実体のポインタを
/// 持ち出して同一性で見る。
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
