//! 定常アロケーション 0 の檻（design.md §Testing Strategy「Integration Tests（presenter 経由）」項目 1・
//! 付録 A′ #9・Requirement 3.1／3.2／6.1／6.2／6.3）。
//!
//! 本ファイルは **本物の `apply_show` を駆動して**、毎コマ経路の再利用が実際に成立していることを
//! 固定する。`budget_tests.rs` は `FrameBudget` の席を手で演じる単体檻であり、`show.rs` がその席を
//! どう使っているかは 1 つも言わない。ここがその継ぎ目を覆う。
//!
//! # 交代は拡大率で分岐しない（裁定 D・要件 1.3／7.3）
//!
//! 合成先席と表示バッファの交代（`FrameBudget::swap_native_scratch`）は `show.rs` のミス経路で
//! **無条件に**走る——表示へ載せるのは原寸そのものであり、CPU で画素を拡大する段が経路から
//! 消えたためである。ゆえに**窓 DPI が 96（k 恒等）でも 192（k=2/1）でも、回る実体の本数も
//! 立ち上がりの確保の形も同一**でなければならない。本ファイルが 2 水準を別々の檻で踏むのは
//! その一致そのものを固定するためである。
//!
//! # 固定する 6 点
//!
//! 1. **確保計数の増分 0**（Requirement 3.1）: 暖機後の反復で 2 発生点の累積が 1 も動かない
//! 2. **暖機の形**（Requirement 3.2）: 立ち上がりの確保が「実体ごとに一度」の裁定済みの形どおりに
//!    現れて 0 へ落ちる。**1 適用へ丸めない**——回る実体は 1 適用ずつずれて伸びるため、
//!    暖機を 1 適用と決め打つと残りの成長が黙って隠れる
//! 3. **交代の実体そのもの**: 前回の合成先席が今回の表示バッファへ回る。同時に生きている 2 つの
//!    実体の役割入れ替えを主張するので、番地の偶然一致が入り込む余地が無い
//! 4. **回る実体の集合が閉じている**: 表示バッファ・合成先席はちょうど [`WARMUP`] 種類
//!    （キャッシュの 3 本＋合成先席 1 本）、マスクも [`WARMUP`] 種類で、そこに新顔が混じらない
//! 5. **下流供給は共有であって複製でない**（Requirement 3.1 の「マスクの下流供給（複製）」）:
//!    `AlphaMaskResource` が握る実体がキャッシュスロットの `Arc` と同一番地であり、
//!    参照数がちょうど 2 である
//! 6. **引き当てヒット経路も踏む**（Requirement 3.1 の裁定文「毎コマ経路（ヒットを含む）」）:
//!    上の 1〜4 を主張する 2 つの檻は**設計上すべての適用がミスする**。ミスしか踏まない檻だけでは
//!    `show.rs` 手順 (3)（表示記録の反映＋マスク供給＋可視化＝合成の有無に依らず走る区間）の
//!    ヒット側が 1 度も動かない。2 つの檻の本体の末尾が
//!    同じキーの再適用を 1 回入れてその穴を塞ぐ
//!
//! 5 は本ファイル以前にリポジトリのどこにも檻が無かった。`show.rs` の `set_shared` を `set`
//! （実体の複製）へ戻しても既存のテストは全て緑のままであり、機序は「`set` は値を**新しい `Arc`**
//! で包むためキャッシュ側の `Arc` は単独所有のまま `Arc::get_mut` が成功し続け、計数もマスクの
//! 実体集合も 1 つも動かない」——複製は `Vec` のクローンとして、計数される席の**外**で起きる。
//! 継ぎ目の両端（wintf の `set_shared` 単体檻・`budget_tests.rs` の手組み Flow 2）はどちらも
//! 覆われていたのに、`show.rs` がどちらを呼ぶかを言う檻だけが無かった。
//!
//! # 「同じ寸法の反復」だけでは再利用を主張できない（本 spec が 4 度踏んだ罠）
//!
//! 毎回まっさらに確保する実装は、同じ寸法の反復では毎回**同じ容量**に着地するため容量比較を
//! 素通りする。ゆえに本ファイルの再利用主張は必ず ⑴実体の番地 ⑵累積計数の据え置き の 2 本立てで
//! 書き、番地の一致は計数の据え置きと**必ず対で**主張する（番地は解放直後の再確保で偶然一致し
//! 得るため単独では証拠にならない）。
//!
//! # 番地の主張はどこまで効くか（実測・登記）
//!
//! 上の但し書きは一般論ではなく、本ファイルに**実際に開いていた（一部は今も開いている）穴**で
//! ある。番地だけが頼りになりうる変異——実体を作り直す形——を 2 種類当てて実測した
//! （走らせ方は `cargo test -p areka-emo-present --lib`・全走・既定の並列度。各変異 4 走）。
//! **走ごとの赤の回数（x/y）を書くのは本節の表だけである**——他の節は自分のどの assert が鳴るかを
//! 質的に書き、回数が要るときはここを見る。
//!
//! - **変異 A**: `presenter/budget.rs` の `FrameBudget::display_buffer` が回収した `composed` を
//!   捨てて `ComposedSurface::default()` を返す
//! - **変異 B**: `presenter/budget.rs` の `seat::SurfaceSeat::lend` で、**容量を読んだ直後・
//!   閉包へ貸す直前**に `self.surface` を `ComposedSurface::default()` へ差し替える
//!   （読み取りより後に置くのが要点で、前に置くと計数へ現れて A と同じ形になる）
//!
//! | 変異 | 検出器（檻・assert の文言） | 何を見ているか | 赤の走数 |
//! |---|---|---|---|
//! | A | 本ファイル 2 水準の檻の「暖機の確保の形が……」 | 計数 | 4/4 |
//! | A | `presenter/budget_tests.rs` の `the_warm_up_allocates_once_per_rotating_buffer_then_settles` ほか | 計数・外形 | 4/4 |
//! | A | `presenter_budget_equivalence_tests.rs` の器の本数 | 番地 | 4/4 |
//! | B | `presenter/budget_tests.rs` の `a_smaller_extent_and_the_regrowth_allocate_nothing` | 計数 | 4/4 |
//! | B | `presenter/budget_tests.rs` の `the_compose_destination_seat_is_allocated_once_and_then_lent_again` | 番地 | 4/4 |
//! | B | 本ファイルの交代関係「前回の合成先席が表示バッファへ回っていない」 | 同時に生きている 2 実体の関係 | 4/4 |
//!
//! **変異 A は要件 7.1（容量 3）の実装で計数側へ移った。** 出所は `FrameBudget` が到達済み寸法
//! （高水位）を器のフィールドで覚えるのをやめ、**実体そのものの `Vec` 容量**を呼び出しの前後で
//! 読み比べる形になったことである（`presenter/budget.rs` 冒頭 §何をもって「確保した」と数えるか）。
//! 回収バッファを捨てて空を返せば容量 0 から伸びるので、必ず 1 件計数される。
//!
//! **変異 B は計数では捕まらない。** 理由は、容量の読み取りが**貸し出しの前**に行われるため、
//! 貸し出しの最中に実体を差し替えると「前の実体の容量」と「新しい実体が伸びた後の容量」を
//! 比べることになり、両者が一致すれば増えたと判定されないからである。決定論的に殺すのは
//! ⑴縮小 → 再拡大の檻（新しい実体は容量 0 から始まるので再拡大で必ず計数される）⑵席単体の
//! 番地の檻 ⑶本ファイルの交代関係、の 3 本である。
//!
//! # 再計測の手順（信用ではなく測り直しで確かめる）
//!
//! 変異 A は `presenter/budget.rs` の `FrameBudget::display_buffer` へ、変異 B は同ファイルの
//! `seat::SurfaceSeat::lend` へ、上の定義どおりに当てる。走らせ方は
//! `cargo test -p areka-emo-present --lib`（全走）で、**赤になった檻の一覧を毎走まるごと記録する**
//! （上の表は 1 本ずつ絞り込むのではなく全走の失敗一覧から作った）。計数で死ぬ主張は 1 走で
//! 決まるが、番地の主張は走ごとに揺れるため最低 4 走して赤の回数で読む。
//!
//! [`FrameBudget`]: super::budget::FrameBudget
//!
//! # 中身が空の実装で偽の合格を出さない
//!
//! すべての反復で表示バッファに**不透明画素が実在すること**を併せて確認し、加えて読み戻しが
//! 非ゼロであることを 1 度確かめる。0×0 や真っ黒を回し続ける実装は「確保が起きない」を空虚に
//! 満たしてしまうため、その口を閉じる。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! 本ファイルは時刻にも経過にも一切触れない（回数・番地・参照数・画素のみ）。純 x64 の常設テストで
//! あり、環境変数ゲートも `#[ignore]` も持たない（Requirement 6.3）。

use super::*;

use std::sync::Arc;

use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::test_support::{build_target_assets, show_with_ok, spawn_window_with_dpi};

// ── fixture ────────────────────────────────────────────────────────────────────────────

/// native 外形。寸法は全反復で固定＝定常状態の定義そのものである。
const NATIVE_W: u32 = 240;
const NATIVE_H: u32 = 180;

/// 巡回するキーの本数。**キャッシュ容量（3・要件 7.1）より大きい**こと自体が要件である。
///
/// 容量 1 の頃は 2 パターンの交互適用で毎回ミスした。容量 3 では 2 パターンは**両方とも表に
/// 収まってしまい、以後すべてヒットする**——ミス経路を主張する本ファイルの檻が空虚になる。
/// 巡回長を容量より 1 大きくすると、入ってくるキーが必ず 1 手前で追い出されているため
/// **毎適用がミスかつ毎適用で追い出しが 1 件成立する**（本番の「毎コマ引き当て外れ」と同じ形）。
const ROTATING_KEYS: usize = 4;

use crate::cache::CAPACITY as CACHE_CAPACITY;

/// 暖機の適用回数＝回る実体の本数。
///
/// 表示バッファと合成先席（キャッシュの 3 本＋席 1 本＝4 本が交代する）・マスク（3 本＋輪番の
/// 空き 1 枚＝4 本）はいずれも **1 適用ずつずれて**立ち上がるため、全ての実体が定常へ入るまでに
/// これだけかかる。
const WARMUP: usize = CACHE_CAPACITY + 1;

/// 定常状態の観測反復数。
const STEADY: usize = 8;

/// 巡回適用の n 回目に使う bind 集合（引き当てを毎回外すための異なるキー）。
///
/// surface 1000 は `animations` を 1 件も持たないため、bind 集合は `build_plan` の走査対象に
/// 一度も現れない——**合成結果にも外形にも影響しない**。ゆえに「毎回ミスするが毎回同じ絵・同じ
/// 寸法」という、定常状態の観測にちょうど要る入力になる。
fn binds_at(i: usize) -> BindSet {
    let id = 1 + (i % ROTATING_KEYS) as u32;
    BindSet::from_ids(id..=id)
}

/// GPU World ＋ DPI 付き窓 ＋ 装着済み target を 1 組作る（`author_dpi` は 96 固定）。
///
/// `window_dpi` は導かれる k（96 なら恒等・192 なら 2/1）を決めるだけで、**毎コマ経路の
/// バッファの回り方は変えない**（交代は無条件・裁定 D）。
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
    /// 2 発生点の累積計数（`[compose_dst, mask]`）。
    cumulative: [u64; 2],
    /// 合成先の常設席が抱えるバイト列の先頭位置（**交代後**＝次の適用が合成する器）。
    native: *const u8,
    /// 表示バッファ（キャッシュスロットの表示用サーフェス）の先頭位置。
    display: *const u8,
    /// キャッシュスロットが握るマスクの実体。
    mask: *const AlphaMask,
    /// **下流（`AlphaMaskResource`）が握るマスクの実体**（下流供給の継ぎ目）。
    supplied: *const AlphaMask,
    /// スロットのマスクの参照数（共有供給なら 2＝スロット＋下流）。
    mask_refs: usize,
    /// 表示バッファ中の不透明画素数（空実装の偽合格を防ぐ非空性ガード）。
    opaque_pixels: usize,
}

/// 直前の適用が残した状態を読み取る。
///
/// `binds` はいま適用したキーそのもの——ここでの `cache.get` は必ずヒットする
/// （ミスしたら「直前の適用がスロットを埋めていない」＝檻の前提が壊れている）。
fn observe(presenter: &EmoPresenter, world: &World, binds: &BindSet) -> Seen {
    let target = presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target");

    let c = target.budget.cumulative();
    let cumulative = [c.alloc_compose_dst, c.alloc_mask];
    let native = target.budget.native_scratch_ptr();

    let entry = target
        .cache
        .get(1000, binds, &PatternState::default())
        .expect("直前の適用がスロットを埋めている");
    let display = entry.composed.bytes().as_ptr();
    let mask = Arc::as_ptr(&entry.mask);
    let mask_refs = Arc::strong_count(&entry.mask);
    let opaque_pixels = entry
        .composed
        .bytes()
        .chunks_exact(4)
        .filter(|px| px[3] == 0xFF)
        .count();

    let surface_entity = target
        .mount
        .as_ref()
        .expect("表示成立後は mount が生成済み")
        .surface_entity();
    let supplied = world
        .get::<AlphaMaskResource>(surface_entity)
        .expect("表示成立後は surface entity に AlphaMaskResource が載る")
        .mask()
        .expect("表示成立後はマスクが供給済み") as *const AlphaMask;

    Seen {
        cumulative,
        native,
        display,
        mask,
        supplied,
        mask_refs,
        opaque_pixels,
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
    observe(presenter, world, &binds)
}

/// 相異なり（順序は問わない・番地の集合を数えるための補助）。
fn distinct<T: Ord + Copy>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = items.collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// 累積計数の差（この適用で起きた確保）。
fn delta(before: [u64; 2], after: [u64; 2]) -> [u64; 2] {
    [after[0] - before[0], after[1] - before[1]]
}

/// 下流供給が**共有**（参照カウント増）であって複製でないことを主張する。
///
/// `show.rs` が `set_shared` ではなく `set`（実体の複製）を呼ぶと、`AlphaMaskResource` は
/// スロットとは別の `Arc` を握る——番地が食い違い、スロット側の参照数も 2 から 1 へ落ちる。
/// 番地と参照数の 2 本立てにしてあるのは、番地の一致が偶然（解放直後の再確保）で成立し得る
/// 一般論への備えである（ここでは両者が同時に生きているため偶然の余地は無いが、主張の
/// 独立性を保つ）。
fn assert_mask_is_shared_not_copied(seen: &Seen, at: &str) {
    assert_eq!(
        seen.supplied, seen.mask,
        "{at}: 当たり判定へ供給したマスクがキャッシュスロットの実体と別物（下流供給が\
         複製になっている＝Requirement 3.1 の「マスクの下流供給（複製）」が生きている）"
    );
    assert_eq!(
        seen.mask_refs, 2,
        "{at}: スロットのマスクの参照数が 2 でない（スロット＋下流の共有が成立していない）"
    );
}

/// 表示バッファに不透明画素が実在することを主張する（中身が空の実装の偽合格を防ぐ）。
fn assert_not_empty(seen: &Seen, at: &str) {
    assert!(
        seen.opaque_pixels > 0,
        "{at}: 表示バッファに不透明画素が 1 つも無い（空を回して「確保が起きない」を\
         空虚に満たしている）"
    );
}

/// 定常アロケーション 0 と交代の回転を 1 水準ぶん検査する（下の 2 本の檻の本体）。
///
/// **窓 DPI（＝導かれる k）で分岐しない**のがこの関数の存在理由である。2 つの檻が同じ本体を
/// 別の水準で呼び、同じ形を要求することで「交代が拡大率で分岐しない」（要件 1.3／7.3）が
/// 檻の構造そのものになる。
///
/// # 暖機の形（Requirement 3.2）
///
/// `[1,1]` が 4 適用ぶん、以後 0。合成先の代金が 4 適用に分かれるのは交代で回る 4 本
/// （キャッシュの 3 本＋合成先席 1 本）ぶんであり、マスクも同じく 4 本（3 本＋輪番の空き 1 枚）
/// ぶんである。暖機を 1 適用へ丸めるとこれらが観測の外へ落ち、残りのバッファの成長が黙って隠れる。
///
/// # 殺す誤実装（変異を実際に当てて、鳴った主張を書き取った）
///
/// 推測で書かない——本 spec は「同じ寸法の反復では再利用を主張できない」を 4 度踏んでいる。
/// 多くは暖機の形（計数の指紋）で落ちるが、**計数が鳴らない変異が 4 件ある**——変異 B・
/// `swap_native_scratch` の複写化（どちらも交代関係で落ちる）と、`set` への差し戻し・ヒット経路だけの
/// `set` への差し戻し（どちらも番地の側で落ちる）。ゆえに番地・交代関係の主張は計数の傍証ではなく、計数では捕まらない形を仕留める
/// 独立の檻である。
///
/// - `seat::SurfaceSeat::lend` の冒頭で席を丸ごと作り直す（容量の読み取りより**前**に差し替える
///   ので計数へ現れる形）→ 「暖機の確保の形が……から外れている」で RED
/// - 席の実体だけ差し替える（§番地の主張はどこまで効くか の**変異 B**）→ 計数の主張は鳴らない。
///   決定論的に殺すのは下の交代関係と `budget_tests.rs` の 2 本である
/// - 容量回収（`ComposeCache::take_recycled`）が満杯でなくても剥がす → 暖機の形で RED
/// - `FrameBudget::display_buffer` が回収した `composed` を捨てて空バッファを返す（**変異 A**）
///   → 暖機の形で RED
/// - マスク輪番を止めて毎コマ `AlphaMask::from_pbgra32` する → 暖機の形で RED
/// - `swap_native_scratch` を複写へ戻す → 交代関係「前回の合成先席が表示バッファへ回っていない」
///   で RED
/// - 下流供給を `set`（実体の複製）へ戻す → [`assert_mask_is_shared_not_copied`] の番地の側で RED
/// - 下流供給を**ヒット経路のときだけ** `set` へ戻す → 末尾のヒット 1 適用の
///   [`assert_mask_is_shared_not_copied`] の番地の側で RED（ミスの適用は緑のまま。2026-11-06 に
///   当てて、2 水準とも「…のヒット経路」で赤になることを確かめた）
fn assert_steady_run_rotates_and_allocates_nothing(window_dpi: u16, salt: u8) {
    let level = format!("窓 DPI {window_dpi}");
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, window_dpi, salt);

    // 暖機: 立ち上がりの確保が「実体ごとに一度」の形で現れて 0 へ落ちる。
    let mut prev = [0_u64; 2];
    let mut warm_shape: Vec<[u64; 2]> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, i);
        warm_shape.push(delta(prev, seen.cumulative));
        prev = seen.cumulative;
    }
    assert_eq!(
        warm_shape,
        vec![[1, 1]; WARMUP],
        "{level}: 暖機の確保の形が Requirement 3.2 の「実体ごとに一度」から外れている\
         （交代する {WARMUP} 本＋マスク {WARMUP} 本が 1 適用ずつずれて立ち上がる形）"
    );

    // 実適用 k が窓 DPI から導かれていること（k がキー要素でなくなった今、これを言わないと
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
        assert_mask_is_shared_not_copied(&seen, &at);
        assert_ne!(
            seen.native, seen.display,
            "{at}: 合成先席と表示バッファが同一実体（交代が成立していない）"
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
    // 交代の実体そのもの: 前回の合成先席が今回の表示バッファへ回る（複写化するとここが崩れる）。
    // 同時に生きている 2 実体の役割入れ替えゆえ、番地の偶然一致が入り込む余地が無い。
    for i in 1..base.len() {
        assert_eq!(
            base[i].display,
            base[i - 1].native,
            "{level} 基準 {i}: 前回の合成先席が表示バッファへ回っていない（swap になっていない）"
        );
    }
    // 回る実体の集合。合成先席と表示バッファは同じ 4 本を役割を替えながら巡る。
    let known_buffers: Vec<*const u8> = distinct(
        base.iter()
            .map(|s| s.native)
            .chain(base.iter().map(|s| s.display)),
    );
    assert_eq!(
        known_buffers.len(),
        WARMUP,
        "{level}: 交代で回る実体が {WARMUP} 本でない（確保し直している／潰れている）"
    );
    let known_masks: Vec<*const AlphaMask> = distinct(base.iter().map(|s| s.mask));
    assert_eq!(
        known_masks.len(),
        WARMUP,
        "{level}: マスク輪番が {WARMUP} 本で回っていない（1 本へ潰れている／新しい実体を作っている）"
    );

    let mut prev_native = base[base.len() - 1].native;
    let mut prev_mask = base[base.len() - 1].mask;
    for i in 0..STEADY {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP * 2 + i);
        let at = format!("{level} の定常 {i} 回目");

        assert_eq!(
            seen.cumulative, cumulative,
            "{at}: 累積計数が動いた（定常状態で確保が起きている）"
        );
        // 次の番地と交代関係の主張は、計数では捕まらない変異 B（席の実体だけの差し替え）を
        // 仕留める（上の §番地の主張はどこまで効くか）。計数の裏取りではない。
        assert!(
            known_buffers.contains(&seen.native),
            "{at}: 合成先席が既知の {WARMUP} 本のどれでもない（確保し直している）"
        );
        assert!(
            known_buffers.contains(&seen.display),
            "{at}: 表示バッファが既知の {WARMUP} 本のどれでもない（回収した容量で回っていない）"
        );
        assert_eq!(
            seen.display, prev_native,
            "{at}: 前回の合成先席が表示バッファへ回っていない（複写化・交代の消失）"
        );
        assert_ne!(
            seen.native, seen.display,
            "{at}: 合成先席と表示バッファが同一実体（交代が成立していない）"
        );
        assert!(
            known_masks.contains(&seen.mask),
            "{at}: マスクが輪番の {WARMUP} 本のどれでもない（新しい実体を作っている）"
        );
        assert_mask_is_shared_not_copied(&seen, &at);
        assert_not_empty(&seen, &at);

        prev_native = seen.native;
        prev_mask = seen.mask;
    }

    // ヒット経路: 直前の適用と**同じ**キーをもう一度渡す。下流供給の 1 文は合成の有無に依らず
    // 走るので、ミスしか踏まないとこの経路を 1 度も動かさない。ヒットの成立はマスクの実体が
    // 動かないことで示す（ミスなら輪番が 1 つ進んで同時に生きている別の実体へ移る）。確保増分 0
    // は単独ではヒットの証拠にならない（定常状態ではミスも 0 件）。次の番号のキーを渡して
    // ミスにすると「マスクの実体が動いた」で赤になることを較正で確かめた。下流供給をヒット経路の
    // ときだけ `set` へ戻すと、下の `assert_mask_is_shared_not_copied` が 2 水準とも赤になることも
    // 当てて確かめた（2026-11-06）。
    let hit = apply_and_observe(&mut presenter, &mut world, WARMUP * 2 + STEADY - 1);
    let at = format!("{level} のヒット経路");
    assert_eq!(
        hit.mask, prev_mask,
        "{at}: マスクの実体が動いた＝引き当てがミスしている（ヒット経路を観測できていない＝以後の主張が空虚）"
    );
    assert_eq!(
        hit.cumulative, cumulative,
        "{at}: 確保が起きた（ヒットなら合成もマスク再生成も走らない）"
    );
    assert_mask_is_shared_not_copied(&hit, &at);
    assert_not_empty(&hit, &at);
}

// ── 檻 ────────────────────────────────────────────────────────────────────────────────

/// Requirement 3.1／3.2／6.1 観測完了（**非等倍・k=2/1**）: 拡大率が 2 倍でも、毎コマ経路は
/// 等倍とまったく同じ 4 本の交代で回り、定常状態の確保は 0 である。
///
/// 裁定 D 以前はここが「リサンプル経路」で、合成先席は 1 本に固定・表示バッファはキャッシュの
/// 3 本だけを回っていた。CPU リサンプルの段が消えた今、**非等倍に固有の器は 1 つも無い**
/// ——それを固定するのが本檻の役割である（下の等倍の檻と同じ本体を呼ぶ）。
#[test]
fn a_steady_scaled_run_reuses_every_buffer_and_allocates_nothing() {
    assert_steady_run_rotates_and_allocates_nothing(192, 0xD1);
}

/// Requirement 3.1／3.2／6.1 観測完了（**等倍**・100% 表示の一般条件）: 拡大率が等倍でも
/// 定常状態の確保は 0 で、合成先席と表示バッファは**ちょうど 4 本の実体を回し続ける**
/// （キャッシュの 3 本＋合成先席 1 本・要件 7.1）。
///
/// 上の非等倍の檻と**同一の本体**を窓 DPI だけ変えて呼ぶ。2 本が同じ形を要求することが
/// 「交代は拡大率で分岐しない」（要件 1.3／7.3）の観測形である。
#[test]
fn a_steady_identity_run_rotates_through_exactly_four_buffers_and_allocates_nothing() {
    assert_steady_run_rotates_and_allocates_nothing(96, 0xD2);
}
