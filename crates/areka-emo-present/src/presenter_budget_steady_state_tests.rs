//! 定常アロケーション 0 の檻（presenter 経由）。
//!
//! 本ファイルは **本物の `apply_show` を駆動して**、毎コマ経路の再利用が実際に成立していることを
//! 固定する。`budget_tests.rs` は `FrameBudget` の席を手で演じる単体檻であり、`show.rs` がその席を
//! どう使っているかは 1 つも言わない。ここがその継ぎ目を覆う。
//!
//! # 交代は拡大率で分岐しない
//!
//! 計画の席と合成メモの計画の交代（`FrameBudget::swap_plan_scratch`）は `show.rs` のミス経路で
//! **無条件に**走る——計画は原寸で、拡大は描画の係数だけが持つ。ゆえに**窓 DPI が 96（k 恒等）でも
//! 192（k=2/1）でも、回る実体の本数も立ち上がりの確保の形も同一**でなければならない。本ファイルが
//! 2 水準を別々の檻で踏むのはその一致そのものを固定するためである。
//!
//! # 固定する 5 点
//!
//! 1. **確保計数の増分 0**: 暖機後の反復で累積が 1 も動かない
//! 2. **暖機の形**: 立ち上がりの確保が「実体ごとに一度」の形どおりに現れて 0 へ落ちる。
//!    **1 適用へ丸めない**——回る実体は 1 適用ずつずれて伸びるため、暖機を 1 適用と決め打つと
//!    残りの成長が黙って隠れる
//! 3. **交代の実体そのもの**: 前回の計画の席が今回キャッシュへ挿す計画へ回る。同時に生きている
//!    2 つの実体の役割入れ替えを主張するので、番地の偶然一致が入り込む余地が無い
//! 4. **回る実体の集合が閉じている**: 計画はちょうど [`WARMUP`] 種類（キャッシュの 3 本＋席 1 本）で、
//!    そこに新顔が混じらない
//! 5. **引き当てヒット経路も踏む**: 上の 1〜4 を主張する檻は設計上すべての適用がミスするので、
//!    本体の末尾が同じキーの再適用を 1 回入れてヒット側を動かす
//!
//! # 「同じ大きさの反復」だけでは再利用を主張できない
//!
//! 毎回まっさらに確保する実装は、同じ大きさの反復では毎回**同じ容量**に着地するため容量比較を
//! 素通りする。ゆえに本ファイルの再利用主張は必ず ⑴実体の番地 ⑵累積計数の据え置き の 2 本立てで
//! 書き、番地の一致は計数の据え置きと**必ず対で**主張する（番地は解放直後の再確保で偶然一致し
//! 得るため単独では証拠にならない）。席の実体だけを貸し出しの最中に差し替える変異は容量の読み比べに
//! 現れないので、交代関係の番地の主張がそれを仕留める。
//!
//! [`FrameBudget`]: super::budget::FrameBudget
//!
//! # 中身が空の実装で偽の合格を出さない
//!
//! すべての反復で計画に項が在ることを併せて確認し、加えて読み戻しが非ゼロであることを 1 度確かめる。
//!
//! # 実時間を合否条件に使わない
//!
//! 本ファイルは時刻にも経過にも一切触れない（回数・番地・画素のみ）。環境変数ゲートも `#[ignore]` も
//! 持たない。

use super::*;

use areka_emo_compose::PlanItem;

use super::test_support::{build_target_assets, show_with_ok, spawn_window_with_dpi};

// ── fixture ────────────────────────────────────────────────────────────────────────────

/// native 外形。寸法は全反復で固定＝定常状態の定義そのものである。
const NATIVE_W: u32 = 240;
const NATIVE_H: u32 = 180;

/// 巡回するキーの本数。**キャッシュ容量（3）より大きい**こと自体が要件である。
///
/// 容量 1 の頃は 2 パターンの交互適用で毎回ミスした。容量 3 では 2 パターンは**両方とも表に
/// 収まってしまい、以後すべてヒットする**——ミス経路を主張する本ファイルの檻が空虚になる。
/// 巡回長を容量より 1 大きくすると、入ってくるキーが必ず 1 手前で追い出されているため
/// **毎適用がミスかつ毎適用で追い出しが 1 件成立する**（本番の「毎コマ引き当て外れ」と同じ形）。
const ROTATING_KEYS: usize = 4;

use crate::cache::CAPACITY as CACHE_CAPACITY;

/// 暖機の適用回数＝回る実体の本数。
///
/// 計画（キャッシュの 3 本＋席 1 本＝4 本が交代する）は **1 適用ずつずれて**立ち上がるため、
/// 全ての実体が定常へ入るまでにこれだけかかる。
const WARMUP: usize = CACHE_CAPACITY + 1;

/// 定常状態の観測反復数。
const STEADY: usize = 8;

/// 巡回適用の n 回目に使う bind 集合（引き当てを毎回外すための異なるキー）。
///
/// surface 1000 は `animations` を 1 件も持たないため、bind 集合は `build_plan` の走査対象に
/// 一度も現れない——**計画にも外形にも影響しない**。ゆえに「毎回ミスするが毎回同じ絵・同じ
/// 寸法」という、定常状態の観測にちょうど要る入力になる。
fn binds_at(i: usize) -> BindSet {
    let id = 1 + (i % ROTATING_KEYS) as u32;
    BindSet::from_ids(id..=id)
}

/// GPU World ＋ DPI 付き窓 ＋ 装着済み target を 1 組作る（`author_dpi` は 96 固定）。
///
/// `window_dpi` は導かれる k（96 なら恒等・192 なら 2/1）を決めるだけで、**毎コマ経路の
/// 計画の回り方は変えない**（交代は無条件）。
fn attach(presenter: &mut EmoPresenter, world: &mut World, window_dpi: u16, salt: u8) {
    let window = spawn_window_with_dpi(world, window_dpi);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, salt);
    presenter.attach_target(TargetId(0), window, emo_world, atlas, 96);
}

// ── 観測 ───────────────────────────────────────────────────────────────────────────────

/// 1 適用の直後に読み取った観測一式。
///
/// 番地は**同一性の主張にのみ**使う（読み出しはしない）。
struct Seen {
    /// 計画の席の確保の累積計数。
    cumulative: u64,
    /// 計画の席が抱える項の列の先頭位置（**交代後**＝次の適用が計画を組む器）。
    seat: *const PlanItem,
    /// キャッシュスロットの計画の項の列の先頭位置。
    inserted: *const PlanItem,
    /// スロットの計画の項の数（空実装の偽合格を防ぐ非空性ガード）。
    items: usize,
}

/// 直前の適用が残した状態を読み取る。
///
/// `binds` はいま適用したキーそのもの——ここでの `cache.get` は必ずヒットする
/// （ミスしたら「直前の適用がスロットを埋めていない」＝檻の前提が壊れている）。
fn observe(presenter: &EmoPresenter, binds: &BindSet) -> Seen {
    let target = presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target");

    let entry = target
        .cache
        .get(1000, binds, &PatternState::default())
        .expect("直前の適用がスロットを埋めている");
    Seen {
        cumulative: target.budget.cumulative().alloc_plan,
        seat: target.budget.planned().items.as_ptr(),
        inserted: entry.plan.items.as_ptr(),
        items: entry.plan.items.len(),
    }
}

/// 1 適用してその直後を観測する。
fn apply_and_observe(presenter: &mut EmoPresenter, world: &mut World, i: usize) -> Seen {
    let binds = binds_at(i);
    // 成立しなければ即座に落とす（以後の観測を空虚にしない）。
    show_with_ok(
        presenter,
        world,
        TargetId(0),
        1000,
        binds.clone(),
        PatternState::default(),
    );
    observe(presenter, &binds)
}

/// 相異なり（順序は問わない・番地の集合を数えるための補助）。
fn distinct<T: Ord + Copy>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = items.collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// スロットの計画に項が在ることを主張する（中身が空の実装の偽合格を防ぐ）。
fn assert_not_empty(seen: &Seen, at: &str) {
    assert!(
        seen.items > 0,
        "{at}: スロットの計画に項が 1 つも無い（空を回して「確保が起きない」を空虚に満たしている）"
    );
}

/// 定常アロケーション 0 と交代の回転を 1 水準ぶん検査する（下の 2 本の檻の本体）。
///
/// **窓 DPI（＝導かれる k）で分岐しない**のがこの関数の存在理由である。2 つの檻が同じ本体を
/// 別の水準で呼び、同じ形を要求することで「交代が拡大率で分岐しない」が檻の構造そのものになる。
///
/// # 暖機の形
///
/// `1` が 4 適用ぶん、以後 0。代金が 4 適用に分かれるのは交代で回る 4 本（キャッシュの 3 本＋席 1 本）
/// ぶんである。暖機を 1 適用へ丸めるとこれらが観測の外へ落ち、残りの計画の成長が黙って隠れる。
///
/// # 殺す誤実装
///
/// - 容量回収（`ComposeCache::take_recycled`）が満杯でなくても剥がす → 暖機の形で RED
/// - `FrameBudget::recycled_plan` が回収した計画を捨てて空の計画を返す → 暖機の形で RED
/// - 席の実体だけを貸し出しの最中に差し替える → 計数は鳴らない。交代関係で RED
/// - `swap_plan_scratch` を複写へ戻す → 交代関係「前回の席がキャッシュへ回っていない」で RED
fn assert_steady_run_rotates_and_allocates_nothing(window_dpi: u16, salt: u8) {
    let level = format!("窓 DPI {window_dpi}");
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, window_dpi, salt);

    // 暖機: 立ち上がりの確保が「実体ごとに一度」の形で現れて 0 へ落ちる。
    let mut prev = 0_u64;
    let mut warm_shape: Vec<u64> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, i);
        warm_shape.push(seen.cumulative - prev);
        prev = seen.cumulative;
    }
    assert_eq!(
        warm_shape,
        vec![1; WARMUP],
        "{level}: 暖機の確保の形が「実体ごとに一度」から外れている\
         （交代する {WARMUP} 本が 1 適用ずつずれて立ち上がる形）"
    );

    // 実適用 k が窓 DPI から導かれていること（k がキー要素でない今、これを言わないと
    // 2 水準のレーンが同じ k で走る恒真に化ける）。
    assert_eq!(
        presenter.applied_ratio(TargetId(0)),
        crate::ScaleRatio::new(u32::from(window_dpi), 96),
        "{level}: 実適用 k が窓 DPI／author_dpi から導かれていない"
    );

    // 読み戻しが非ゼロ（表示に実際に絵が載っていることの 1 度きりの確認）。
    let bytes = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert!(
        !bytes.is_empty() && bytes.iter().any(|&b| b != 0),
        "{level}: 読み戻しが空（表示が実際には成立していない＝以後の観測が空虚）"
    );

    // 定常状態の基準を「一巡ぶん」（＝回る実体の本数）で取る——基準 1 本では残りを新顔と誤認する。
    let mut base: Vec<Seen> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP + i);
        let at = format!("{level} の基準適用 {i}");
        assert_not_empty(&seen, &at);
        assert_ne!(
            seen.seat, seen.inserted,
            "{at}: 席とスロットの計画が同一実体（交代が成立していない）"
        );
        base.push(seen);
    }
    let cumulative = base[0].cumulative;
    for (i, seen) in base.iter().enumerate() {
        assert_eq!(
            seen.cumulative, cumulative,
            "{level}: 基準の適用 {i} で確保が起きた（暖機が終わっていない＝以後の比較が空虚）"
        );
    }
    // 交代の実体そのもの: 前回の席が今回スロットへ挿す計画へ回る（複写化するとここが崩れる）。
    for i in 1..base.len() {
        assert_eq!(
            base[i].inserted,
            base[i - 1].seat,
            "{level} 基準 {i}: 前回の席がスロットへ回っていない（swap になっていない）"
        );
    }
    // 回る実体の集合。席とスロットの計画は同じ 4 本を役割を替えながら巡る。
    let known: Vec<*const PlanItem> = distinct(
        base.iter()
            .map(|s| s.seat)
            .chain(base.iter().map(|s| s.inserted)),
    );
    assert_eq!(
        known.len(),
        WARMUP,
        "{level}: 交代で回る実体が {WARMUP} 本でない（確保し直している／潰れている）"
    );

    let mut prev_seat = base[base.len() - 1].seat;
    let mut prev_inserted = base[base.len() - 1].inserted;
    for i in 0..STEADY {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP * 2 + i);
        let at = format!("{level} の定常 {i} 回目");

        assert_eq!(
            seen.cumulative, cumulative,
            "{at}: 累積計数が動いた（定常状態で確保が起きている）"
        );
        assert!(
            known.contains(&seen.seat),
            "{at}: 席が既知の {WARMUP} 本のどれでもない（確保し直している）"
        );
        assert!(
            known.contains(&seen.inserted),
            "{at}: スロットの計画が既知の {WARMUP} 本のどれでもない（回収した容量で回っていない）"
        );
        assert_eq!(
            seen.inserted, prev_seat,
            "{at}: 前回の席がスロットへ回っていない（複写化・交代の消失）"
        );
        assert_ne!(
            seen.seat, seen.inserted,
            "{at}: 席とスロットの計画が同一実体（交代が成立していない）"
        );
        assert_not_empty(&seen, &at);

        prev_seat = seen.seat;
        prev_inserted = seen.inserted;
    }

    // ヒット経路: 直前の適用と**同じ**キーをもう一度渡す。ヒットの成立はスロットの計画の実体が
    // 動かないことで示す（ミスなら交代で同時に生きている別の実体へ移る）。確保増分 0 は単独では
    // ヒットの証拠にならない（定常状態ではミスも 0 件）。
    let hit = apply_and_observe(&mut presenter, &mut world, WARMUP * 2 + STEADY - 1);
    let at = format!("{level} のヒット経路");
    assert_eq!(
        hit.inserted, prev_inserted,
        "{at}: スロットの計画の実体が動いた＝引き当てがミスしている（ヒット経路を観測できていない）"
    );
    assert_eq!(
        hit.cumulative, cumulative,
        "{at}: 確保が起きた（ヒットなら計画は組まない）"
    );
    assert_not_empty(&hit, &at);
}

// ── 檻 ────────────────────────────────────────────────────────────────────────────────

/// **非等倍・k=2/1**: 拡大率が 2 倍でも、毎コマ経路は等倍とまったく同じ 4 本の交代で回り、
/// 定常状態の確保は 0 である（下の等倍の檻と同じ本体を呼ぶ）。
#[test]
fn a_steady_scaled_run_reuses_every_plan_and_allocates_nothing() {
    assert_steady_run_rotates_and_allocates_nothing(192, 0xD1);
}

/// **等倍**: 拡大率が等倍でも定常状態の確保は 0 で、計画は**ちょうど 4 本の実体を回し続ける**
/// （キャッシュの 3 本＋席 1 本）。上の非等倍の檻と**同一の本体**を窓 DPI だけ変えて呼ぶ。
#[test]
fn a_steady_identity_run_rotates_through_exactly_four_plans_and_allocates_nothing() {
    assert_steady_run_rotates_and_allocates_nothing(96, 0xD2);
}
