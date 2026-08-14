//! 定常アロケーション 0 の檻（design.md §Testing Strategy「Integration Tests（presenter 経由）」項目 1・
//! Requirement 3.1／3.2／6.1／6.2／6.3）。
//!
//! 本ファイルは **本物の `apply_show` を駆動して**、毎コマ経路の再利用が実際に成立していることを
//! 固定する。`budget_tests.rs` は `FrameBudget` の席を手で演じる単体檻であり、`show.rs` がその席を
//! どう使っているかは 1 つも言わない。ここがその継ぎ目を覆う。
//!
//! # 固定する 6 点
//!
//! 1. **確保計数の増分 0**（Requirement 3.1）: 暖機後の反復で 4 発生点の累積が 1 も動かない
//! 2. **暖機の形**（Requirement 3.2）: 立ち上がりの確保が「席ごとに一度」の裁定済みの形どおりに
//!    現れて 0 へ落ちる。**1 適用へ丸めない**——交代する 2 本のバッファは 1 適用ずれて伸びるため、
//!    暖機を 1 適用と決め打つと片方の成長が黙って隠れる
//! 3. **合成先と表示バッファの先頭位置**: 実体そのものが回り続けている……**ただしこの主張は
//!    傍証であって単独の検出器ではない**。計数を動かさずに実体だけ作り直す実装は、解放直後の
//!    再確保が同じ番地を返すと素通りする。実測と、確実に殺す側の檻の所在は下の
//!    §番地の主張はどこまで効くか（実測）に置く
//! 4. **マスク輪番の 2 本集合不変**: 実体はちょうど 2 種類で、そこに新顔が混じらない
//! 5. **下流供給は共有であって複製でない**（Requirement 3.1 の「マスクの下流供給（複製）」・
//!    追補 §A7）: `AlphaMaskResource` が握る実体がキャッシュスロットの `Arc` と同一番地であり、
//!    参照数がちょうど 2 である
//! 6. **引き当てヒット経路も踏む**（Requirement 3.1 の裁定文「毎コマ経路（ヒットを含む）」）:
//!    上の 1〜4 を主張する 2 つの檻は**設計上すべての適用がミスする**。ミスしか踏まない檻だけでは
//!    `show.rs` 手順 (3)（アップロード＋マスク供給＋可視化＝合成の有無に依らず走る区間）の
//!    ヒット側が 1 度も動かない。[`the_hit_test_mask_is_supplied_by_sharing_not_by_copying`] が
//!    同じキーの再適用を 1 回入れてその穴を塞ぐ
//!
//! 5 は本ファイル以前にリポジトリのどこにも檻が無かった。`show.rs` の `set_shared` を `set`
//! （実体の複製）へ戻しても既存 184 件は全て緑のままであり、機序は「`set` は値を**新しい `Arc`**
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
//! # 番地の主張はどこまで効くか（実測・本ファイルの弱点の登記）
//!
//! 上の但し書きは一般論ではなく、本ファイルに**実際に開いている穴**である。番地だけが頼りになる
//! 変異——実体を作り直しつつ到達済みバイト長（高水位）を持ち越すため計数が 1 件も動かない形——を
//! 2 種類当てて実測した（既定の走らせ方は `cargo test -p areka-emo-present --lib`・既定の並列度）。
//! **走ごとの赤の回数（x/y）を書くのは本節の表だけである**——他の節は自分のどの assert が鳴るかを
//! 質的に書き、回数が要るときはここを見る。
//!
//! - **変異 A**: `presenter/budget.rs` の `FrameBudget::display_buffer` が回収した `composed` を
//!   捨てて `ComposedSurface::default()` を返す（`display_reached` は持ち越す）
//! - **変異 B**: `presenter/budget.rs` の `seat::SurfaceSeat::lend` の冒頭で `surface` だけを
//!   `ComposedSurface::default()` へ差し替える（`reached` は持ち越す）
//!
//! どちらも計数には 1 件も現れない。出所は [`FrameBudget`] の高水位が**バッファの実体に束ねられて
//! いない**ことである（`display_reached` は器のフィールドで、表示バッファ自身はキャッシュが
//! 所有する）。実体を差し替えても高水位は前の実体の値を覚えたままなので、伸長が「到達済み以下」と
//! 判定されて計数へ現れない。ゆえに番地だけが頼りになる。
//!
//! | 変異 | 検出器（檻・assert の文言） | 走らせ方 | 赤の走数 |
//! |---|---|---|---|
//! | A | k=2/1 の檻 `a_steady_scaled_run_reuses_every_buffer_and_allocates_nothing` の「表示バッファが別の実体になった」 | `--lib` 全走 | 0/4 |
//! | A | 恒等 k の檻 `a_steady_identity_run_alternates_between_exactly_two_buffers_and_allocates_nothing` の交代関係「前回の表示バッファが合成先席へ回っていない」「前回の合成先席が表示バッファへ回っていない」 | `--lib` 全走 | 4/4 |
//! | A | `presenter/budget_tests.rs` の席単体の檻（下記 3 本） | `--lib` 全走 | 4/4 |
//! | B | k=2/1 の檻の「合成先席が別の実体になった」 | 本ファイル絞り込み | 0/4 |
//! | B | 同上 | `--lib` 全走（実装時の計測） | 1/5 |
//! | B | 同上 | `--lib` 全走（レビューの再計測） | 2/4 |
//! | B | 恒等 k の檻 | `--lib` 全走 | 3/4 |
//! | B | `presenter/budget_tests.rs` の席単体の檻（下記 3 本） | `--lib` 全走 | 4/4 |
//!
//! 席単体の檻の 3 本は `the_display_buffer_hands_back_the_recycled_allocation_unchanged`・
//! `the_compose_destination_seat_is_allocated_once_and_then_lent_again`・
//! `the_steady_state_keeps_handing_back_the_same_allocations` である（3 本のうちどの assert が
//! 鳴ったかまでは記録していない）。走らせ方の欄は上記の既定（`--lib` 全走）で、「本ファイル
//! 絞り込み」と明記した行だけが例外である。B の 1/5 と 2/4 は**同じ変異・同じ走らせ方の別々の
//! 計測**（実装時とレビューの再計測）であり、どちらも消さずに並べてある。
//!
//! **読み方: 決定論的と呼べるのは表で 4/4 の行だけである。** それ以外の行は傍証であり、単独では
//! 再利用の証拠にならない。本ファイル自身の番地主張は 0/4・1/5・2/4 の行であって、この意味で
//! 傍証である。
//!
//! # 再計測の手順（信用ではなく測り直しで確かめる）
//!
//! 変異 A は `presenter/budget.rs` の `FrameBudget::display_buffer` へ、変異 B は同ファイルの
//! `seat::SurfaceSeat::lend` の冒頭へ、上の定義どおりに当てる。走らせ方は
//! `cargo test -p areka-emo-present --lib budget_steady_state_tests`（本ファイル絞り込み）と
//! `cargo test -p areka-emo-present --lib`（全走）の**両方**を踏むこと——表のとおり両者で結果が
//! 食い違う。番地の主張は走ごとに揺れるため、いずれも最低 4 走して赤の回数で読む
//! （1 走の緑は何も言わない）。
//!
//! [`FrameBudget`]: super::budget::FrameBudget
//!
//! # 中身が空の実装で偽の合格を出さない
//!
//! すべての反復で表示バッファに**不透明画素が実在すること**を併せて確認し、加えて GPU 供給面の
//! readback が非ゼロであることを 1 度確かめる。0×0 や真っ黒を回し続ける実装は「確保が起きない」
//! を空虚に満たしてしまうため、その口を閉じる。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）
//!
//! 本ファイルは時刻にも経過にも一切触れない（回数・番地・参照数・画素のみ）。純 x64 の常設テストで
//! あり、環境変数ゲートも `#[ignore]` も持たない（Requirement 6.3）。

use super::*;

use std::sync::Arc;
use std::time::Duration;

use wintf::ecs::widget::bitmap_source::AlphaMask;

use super::test_support::{build_target_assets, spawn_window_with_dpi};

// ── fixture ────────────────────────────────────────────────────────────────────────────

/// native 外形（k=2/1 で 480×360）。寸法は全反復で固定＝定常状態の定義そのものである。
const NATIVE_W: u32 = 240;
const NATIVE_H: u32 = 180;

/// 暖機の適用回数。交代する 2 本のバッファ（恒等 k の合成先席・マスク輪番）が
/// **1 適用ずつずれて**立ち上がるため、2 適用では足りない回がある。4 で全席が定常へ入る。
const WARMUP: usize = 4;

/// 定常状態の観測反復数。
const STEADY: usize = 8;

/// 交互適用の 2 パターン（引き当てを毎回外すための異なるキー）。
///
/// surface 1000 は `animations` を 1 件も持たないため、bind 集合は `build_plan` の走査対象に
/// 一度も現れない——**合成結果にも外形にも影響しない**。ゆえに「毎回ミスするが毎回同じ絵・同じ
/// 寸法」という、定常状態の観測にちょうど要る入力になる。
fn binds_a() -> BindSet {
    BindSet::from_ids(1..=1)
}

/// [`binds_a`] と 1 ビット異なる集合（キーが違えば必ずミス＝再合成する）。
fn binds_b() -> BindSet {
    BindSet::from_ids(2..=2)
}

/// 交互適用の n 回目に使う bind 集合。
fn binds_at(i: usize) -> BindSet {
    if i % 2 == 0 { binds_a() } else { binds_b() }
}

/// GPU World ＋ DPI 付き窓 ＋ 装着済み target を 1 組作る（`author_dpi` は 96 固定）。
///
/// `window_dpi` が 96 なら恒等 k（swap 交代経路）、192 なら k=2/1（リサンプル経路）である。
fn attach(presenter: &mut EmoPresenter, world: &mut World, window_dpi: u16, salt: u8) {
    let window = spawn_window_with_dpi(world, window_dpi);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, salt);
    presenter
        .attach_target(world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
}

/// `ShowSurface` を 1 回適用する（成立しなければ即座に落とす＝以後の観測を空虚にしない）。
fn show(presenter: &mut EmoPresenter, world: &mut World, binds: BindSet) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: 1000,
            binds,
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提: ShowSurface が成立すること（成立しない適用の観測は意味を持たない）"
    );
}

// ── 観測 ───────────────────────────────────────────────────────────────────────────────

/// 1 適用の直後に読み取った観測一式。
///
/// 番地は**同一性の主張にのみ**使う（読み出しはしない）。
struct Seen {
    /// 4 発生点の累積計数（`[compose_dst, resample_dst, xmap, mask]`）。
    cumulative: [u64; 4],
    /// 合成先の常設席が抱えるバイト列の先頭位置。
    native: *const u8,
    /// 表示バッファ（キャッシュスロットの表示用サーフェス）の先頭位置。
    display: *const u8,
    /// キャッシュスロットが握るマスクの実体。
    mask: *const AlphaMask,
    /// **下流（`AlphaMaskResource`）が握るマスクの実体**（追補 §A7 の継ぎ目）。
    supplied: *const AlphaMask,
    /// スロットのマスクの参照数（共有供給なら 2＝スロット＋下流）。
    mask_refs: usize,
    /// 表示バッファ中の不透明画素数（空実装の偽合格を防ぐ非空性ガード）。
    opaque_pixels: usize,
}

/// 直前の適用が残した状態を読み取る。
///
/// `binds`／`scale` はいま適用したキーそのもの——ここでの `cache.get` は必ずヒットする
/// （ミスしたら「直前の適用がスロットを埋めていない」＝檻の前提が壊れている）。
fn observe(
    presenter: &EmoPresenter,
    world: &World,
    binds: &BindSet,
    scale: ScaleRatio,
) -> Seen {
    let target = presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target");

    let c = target.budget.cumulative();
    let cumulative = [
        c.alloc_compose_dst,
        c.alloc_resample_dst,
        c.alloc_xmap,
        c.alloc_mask,
    ];
    let native = target.budget.native_scratch_ptr();

    let entry = target
        .cache
        .get(1000, binds, &PatternState::default(), scale)
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
fn apply_and_observe(
    presenter: &mut EmoPresenter,
    world: &mut World,
    i: usize,
    scale: ScaleRatio,
) -> Seen {
    let binds = binds_at(i);
    show(presenter, world, binds.clone());
    observe(presenter, world, &binds, scale)
}

/// 累積計数の差（この適用で起きた確保）。
fn delta(before: [u64; 4], after: [u64; 4]) -> [u64; 4] {
    [
        after[0] - before[0],
        after[1] - before[1],
        after[2] - before[2],
        after[3] - before[3],
    ]
}

/// 下流供給が**共有**（参照カウント増）であって複製でないことを主張する（追補 §A7）。
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

// ── 檻 ────────────────────────────────────────────────────────────────────────────────

/// Requirement 3.1／3.2／6.1 観測完了（**リサンプル経路・k=2/1**）: 2 パターンの交互適用
/// （毎回ミス）で暖機した後、定常状態の反復が 1 件も確保せず、合成先席・表示バッファの実体が
/// 動かず、マスク輪番がちょうど 2 本で回り、下流供給が共有のままである。
///
/// # 暖機の形を丸めない（Requirement 3.2）
///
/// 立ち上がりの確保は `[1,1,1,1]` → `[0,0,0,1]` → 以後 0 という形で現れる。2 適用目のマスク 1 件は
/// **輪番の 2 本目**であり、「交代する 2 本は 1 適用ずれて伸びる」という裁定済みの形そのものである。
/// 暖機を 1 適用へ丸めるとこの 1 件が観測の外へ落ち、片方のバッファの成長が黙って隠れる。
///
/// # 殺す誤実装（**変異を実際に当てて、鳴った主張を書き取った**）
///
/// 推測で書かない——本 spec は「同じ寸法の反復では再利用を主張できない」を 4 度踏んでおり、
/// 5 度目は「どの主張が鳴るか」を取り違えた doc として現れた。以下は全て実行して観測した結果で
/// ある（走らせ方は `cargo test -p areka-emo-present --lib budget_steady_state_tests`。番地に
/// 関わる 2 件だけは全走 `--lib` でも測った＝上の §番地の主張はどこまで効くか）。
///
/// 目を引くのは、**6 件のうち 3 件が定常の番地主張ではなく暖機の形（計数の指紋）で落ちる**
/// ことである。立ち上がりの `[1,1,1,1]` → `[0,0,0,1]` → 0 という形は再利用の成立をきわめて細く
/// 縛るため、定常の反復へ入る前にそこで死ぬ。裏を返せば、**定常の番地主張が単独で仕留めた変異は
/// 1 件も無い**——これが上の「番地は傍証」という位置づけの実測上の根拠である。
///
/// - `seat::SurfaceSeat::lend` の冒頭で席も高水位も作り直す（`*self = Self::default()`）
///   → 暖機の形が `[[1,1,1,1],[1,0,0,1],[1,0,0,0],[1,0,0,0]]` になり
///     **「暖機の確保の形が Requirement 3.2 の……から外れている」で RED**
/// - 席の実体だけ差し替えて高水位を持ち越す（`self.surface = ComposedSurface::default()`＝上の
///   §番地の主張はどこまで効くか の**変異 B**）
///   → 計数は 1 件も動かない。鳴り得るのは「合成先席が別の実体になった」だが**走ごとに揺れる**。
///     確実に殺すのは `presenter/budget_tests.rs` の席単体の檻である
///     （赤の走数は §番地の主張はどこまで効くか の表の B の行）
/// - 容量回収（`ComposeCache::take_recycled`）が常に `None` を返す
///   → 暖機の形が `[[1,1,1,1],[0,1,0,1],[0,1,0,1],[0,1,0,1]]` になり
///     **「暖機の確保の形が……」で RED**。`display` の番地主張はそこへ到達する前に落ちる
/// - `FrameBudget::display_buffer` が回収した `composed` を捨てて空バッファを返す
///   （`display_reached` は持ち越す＝計数が動かない形。上の §番地の主張はどこまで効くか の
///   **変異 A**）
///   → **本檻は緑のまま**。殺すのは恒等 k の檻の交代関係と `presenter/budget_tests.rs` の
///     席単体の檻である（赤の走数は §番地の主張はどこまで効くか の表の A の行）
/// - マスク輪番を止めて毎コマ `AlphaMask::from_pbgra32` する
///   → 暖機の形が `[[1,1,1,1],[0,0,0,1],[0,0,0,1],[0,0,0,1]]` になり
///     **「暖機の確保の形が……」で RED**。実体集合の主張までは到達しない
/// - 下流供給を `set`（実体の複製）へ戻す
///   → [`assert_mask_is_shared_not_copied`] の**番地の側**（「当たり判定へ供給したマスクが
///     キャッシュスロットの実体と別物」）で RED。参照数の側より先にこちらが鳴る
#[test]
fn a_steady_scaled_run_reuses_every_buffer_and_allocates_nothing() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, 192, 0xD1);
    let scale = ScaleRatio::new(2, 1).expect("2/1 は構築できる");

    // 暖機: 立ち上がりの確保が「席ごとに一度」の形で現れて 0 へ落ちる。
    let mut prev = [0_u64; 4];
    let mut warm_shape: Vec<[u64; 4]> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, i, scale);
        warm_shape.push(delta(prev, seen.cumulative));
        prev = seen.cumulative;
    }
    assert_eq!(
        warm_shape,
        vec![[1, 1, 1, 1], [0, 0, 0, 1], [0, 0, 0, 0], [0, 0, 0, 0]],
        "暖機の確保の形が Requirement 3.2 の「席ごとに一度」から外れている\
         （2 適用目のマスク 1 件＝輪番の 2 本目を含む）"
    );

    // 供給面の readback が非ゼロ（GPU まで実際に絵が載っていることの 1 度きりの確認）。
    let bytes = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert!(
        !bytes.is_empty() && bytes.iter().any(|&b| b != 0),
        "供給面の readback が空（表示が実際には成立していない＝以後の観測が空虚）"
    );

    // 定常状態の基準を 2 適用で取る——輪番は 2 本が交互に出るため、基準 1 本では「もう 1 本」を
    // 新顔と誤認する。ここで 2 本が**実際に異なる**ことも同時に固定する（1 本へ潰れた実装＝
    // 毎回同じバッファを再生成する形は、下流が握っている最中の実体を書き換えることになる）。
    let first = apply_and_observe(&mut presenter, &mut world, WARMUP, scale);
    let second = apply_and_observe(&mut presenter, &mut world, WARMUP + 1, scale);
    for (seen, at) in [(&first, "定常の基準適用 1"), (&second, "定常の基準適用 2")] {
        assert_not_empty(seen, at);
        assert_mask_is_shared_not_copied(seen, at);
    }
    assert_eq!(
        second.cumulative, first.cumulative,
        "基準の 2 適用の間で確保が起きた（暖機が終わっていない＝以後の比較が空虚）"
    );
    assert_ne!(
        first.mask, second.mask,
        "マスク輪番の 2 本が交代していない（1 本を使い回している）"
    );
    let known_masks = [first.mask, second.mask];

    for i in 0..STEADY {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP + 2 + i, scale);
        let at = format!("定常 {i} 回目");

        assert_eq!(
            seen.cumulative, first.cumulative,
            "{at}: 累積計数が動いた（定常状態で確保が起きている）"
        );
        // 次の 2 本は**傍証**である（上の §番地の主張はどこまで効くか）。高水位
        // （`FrameBudget::display_reached`・`seat::SurfaceSeat::reached`）はバッファの実体に
        // 束ねられていないため、実体を作り直しつつ高水位を持ち越す実装は計数を 1 件も動かさず、
        // 解放直後の再確保が同じ番地を返せばこの 2 本も素通りする（実測済み）。この 2 本を
        // 素通りする形を決定論的に殺す檻は変異ごとに違う——表示バッファ側（変異 A）は恒等 k の
        // 檻の交代関係と `presenter/budget_tests.rs` の席単体の檻、合成先席側（変異 B）は席単体
        // の檻だけである（赤の走数は §番地の主張はどこまで効くか の表）。ここに残しているのは、
        // それらが同時に緩んだときの追加の網としてである。
        assert_eq!(
            seen.native, first.native,
            "{at}: 合成先席が別の実体になった（常設席で回っていない）"
        );
        assert_eq!(
            seen.display, first.display,
            "{at}: 表示バッファが別の実体になった（回収した容量で回っていない）"
        );
        assert!(
            known_masks.contains(&seen.mask),
            "{at}: マスクが輪番の 2 本のどちらでもない（新しい実体を作っている）"
        );
        assert_mask_is_shared_not_copied(&seen, &at);
        assert_not_empty(&seen, &at);
    }
}

/// Requirement 3.1／3.2／6.1 観測完了（**交代経路・恒等 k**）: 拡大率が等倍（100% 表示の一般条件）
/// でも定常状態の確保は 0 で、合成先席と表示バッファは**ちょうど 2 本の実体を交代し続ける**。
///
/// 恒等 k は合成先席と表示バッファを `swap` で入れ替える（コピーもリサンプルも起きない）。ゆえに
/// 番地は「不変」ではなく「2 本集合の交互」になる——集合が 2 本を超えれば確保し直しており、
/// 1 本へ潰れれば交代が起きていない（＝どこかで複写している）。
///
/// # 暖機の形
///
/// `[1,0,0,1]` → `[1,0,0,1]` → 以後 0。合成先の 1 件が 2 適用に分かれるのが交代する 2 本ぶんで
/// あり、マスクの 1 件が 2 適用に分かれるのが輪番 2 本ぶんである。リサンプル経路を通らないため
/// `alloc_resample_dst`／`alloc_xmap` は全適用で 0 のままである。
///
/// # 殺す誤実装（**変異を実際に当てて、鳴った主張を書き取った**）
///
/// - `seat::SurfaceSeat::swap` を複写（`*buffer = self.surface.clone()`）へ戻す
///   → 席が永久に 1 本になるので合成先の代金が 2 適用に分かれず、暖機の形が
///     `[[1,0,0,1],[0,0,0,1],[0,0,0,0],[0,0,0,0]]` になって**「恒等 k の暖機は交代する 2 本・
///     輪番 2 本ぶんが 1 適用ずつずれて現れる形のはず」で RED**。定常の 2 本集合の
///     主張までは到達しない
/// - 交代のたびに席へ新しいバッファを起こす（`swap` の直後に `surface`／`reached` を作り直す）
///   → 暖機の形が `[[1,0,0,1],[1,0,0,1],[1,0,0,0],[1,0,0,0]]` になって**同じ「恒等 k の暖機は
///     ……」で RED**
///
/// # 本檻が担う固有の役割（番地の偶然一致に強い唯一の関係）
///
/// 下の交代関係——「前回の表示バッファが合成先席へ回り、前回の合成先席が表示バッファへ回る」
/// ——は、**同時に生きている 2 つの実体**の役割入れ替えを主張する。解放も再確保も介在しないため、
/// k≠1 の檻の番地不変主張を素通りする変異（`display_buffer` が回収バッファを捨てて空から
/// 作り直す形＝ファイル冒頭の §番地の主張はどこまで効くか の**変異 A**）をここが決定論的に
/// 捕まえる（赤の走数は同節の表の A の行）。
#[test]
fn a_steady_identity_run_alternates_between_exactly_two_buffers_and_allocates_nothing() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    // 窓 DPI ＝ author_dpi ＝ 96 ゆえ恒等 k（swap 交代経路）。
    attach(&mut presenter, &mut world, 96, 0xD2);
    let scale = ScaleRatio::new(1, 1).expect("1/1 は構築できる");

    let mut prev = [0_u64; 4];
    let mut warm_shape: Vec<[u64; 4]> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, i, scale);
        warm_shape.push(delta(prev, seen.cumulative));
        prev = seen.cumulative;
    }
    assert_eq!(
        warm_shape,
        vec![[1, 0, 0, 1], [1, 0, 0, 1], [0, 0, 0, 0], [0, 0, 0, 0]],
        "恒等 k の暖機は交代する 2 本・輪番 2 本ぶんが 1 適用ずつずれて現れる形のはず"
    );

    // 基準を 2 適用で取る（交代する 2 本・輪番の 2 本はいずれも 2 適用で一巡する）。
    let first = apply_and_observe(&mut presenter, &mut world, WARMUP, scale);
    let second = apply_and_observe(&mut presenter, &mut world, WARMUP + 1, scale);
    for (seen, at) in [(&first, "恒等 k 基準適用 1"), (&second, "恒等 k 基準適用 2")] {
        assert_not_empty(seen, at);
        assert_mask_is_shared_not_copied(seen, at);
        assert_ne!(
            seen.native, seen.display,
            "{at}: 合成先席と表示バッファが同一実体（交代が成立していない）"
        );
    }
    assert_eq!(
        second.cumulative, first.cumulative,
        "基準の 2 適用の間で確保が起きた（暖機が終わっていない＝以後の比較が空虚）"
    );
    // 交代の実体そのもの: 1 適用で役割がちょうど入れ替わる。
    assert_eq!(
        second.native, first.display,
        "前回の表示バッファが合成先席へ回っていない（swap になっていない）"
    );
    assert_eq!(
        second.display, first.native,
        "前回の合成先席が表示バッファへ回っていない（swap になっていない）"
    );
    assert_ne!(
        first.mask, second.mask,
        "恒等 k でマスク輪番の 2 本が交代していない"
    );
    let known_buffers = [first.native, first.display];
    let known_masks = [first.mask, second.mask];

    let mut prev = second;
    for i in 0..STEADY {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP + 2 + i, scale);
        let at = format!("恒等 k 定常 {i} 回目");

        assert_eq!(
            seen.cumulative, first.cumulative,
            "{at}: 累積計数が動いた（恒等 k の定常状態で確保が起きている）"
        );
        // 2 本集合の交互を許容する形（不変ではなく「集合が 2 本のまま」＋「毎回入れ替わる」）。
        assert!(
            known_buffers.contains(&seen.native),
            "{at}: 合成先席が既知の 2 本のどちらでもない（確保し直している）"
        );
        assert!(
            known_buffers.contains(&seen.display),
            "{at}: 表示バッファが既知の 2 本のどちらでもない（確保し直している）"
        );
        assert_eq!(
            seen.native, prev.display,
            "{at}: 前回の表示バッファが合成先席へ回っていない（複写化・交代の消失）"
        );
        assert_eq!(
            seen.display, prev.native,
            "{at}: 前回の合成先席が表示バッファへ回っていない（複写化・交代の消失）"
        );
        assert!(
            known_masks.contains(&seen.mask),
            "{at}: マスクが輪番の 2 本のどちらでもない（新しい実体を作っている）"
        );
        assert_mask_is_shared_not_copied(&seen, &at);
        assert_not_empty(&seen, &at);

        prev = seen;
    }
}

/// Requirement 3.1 観測完了（**追補 §A7・下流供給の複製ゼロ**）: 当たり判定へ渡すマスクは
/// `Arc` の共有であって実体の複製ではない。
///
/// # なぜ専用の檻が要るのか（この穴が生き残った理由）
///
/// 継ぎ目の**両端**は覆われていた——wintf の `hit_test_shared_mask_tests.rs` は `set_shared` が
/// 確保を共有することを直接呼出で証明し、`budget_tests.rs` の手組み Flow 2 は輪番が 2 本で回る
/// ことを証明する。しかし `show.rs` が `set` と `set_shared` の**どちらを呼ぶか**を言う檻が 1 本も
/// 無かった。`set` へ戻しても既存 184 件は全て緑のままである: `set` は値を新しい `Arc` で包むので
/// キャッシュ側の `Arc` は単独所有のままであり、`Arc::get_mut` は成功し続け、確保計数もマスクの
/// 実体集合も 1 つも動かない。複製は `Vec` のクローンとして、計数される席の**外**で起きる。
///
/// 本檻は実 `apply_show` を通した後に、下流が握る実体とスロットの `Arc` を突き合わせる。
/// 拡大率が非等倍・等倍のどちらでも同じことを主張する（供給の 1 文は k で分岐しないため、
/// 両方を踏むことで「片方だけ直っている」形も残さない）。
///
/// # 引き当てヒット経路の陽性対照（Requirement 3.1 の裁定文「毎コマ経路（ヒットを含む）」）
///
/// 上 2 つの檻は**設計上すべての適用がミスする**（キーを毎回変える）。ところが `show.rs` の
/// 手順 (3)——供給面アップロード＋マスク供給＋可視化——は合成の有無に依らず走るため、`set_shared`
/// を含む下流供給の 1 文は**ヒット経路にも乗っている**。ミスしか踏まない檻の集合は、この経路を
/// 1 度も動かさないまま「毎コマ経路は全て覆った」と言えてしまう。
///
/// 本 spec は同じ穴の形で一度失敗している（task 2.3 の偽合格は、判定素材のどれ 1 つもプール経路を
/// 踏んでいなかったために 3 回のレビューを生き延びた）。ゆえにここで**同じ bind 集合をもう一度**
/// 適用してヒットを起こし、その 1 適用でも供給が共有のままであることを見る。
///
/// ヒットが実際に成立したことは**マスクの実体が動かないこと**で示す——ミスなら輪番が 1 つ進んで
/// もう 1 本の実体（同時に生きている別の `Arc`）へ移るため、番地の偶然一致が入り込む余地が無い。
/// 確保増分 0 の側は単独ではヒットの証拠にならない（定常状態ではミスも 0 件だからである）。
/// この判別が効いていることは較正で確かめた: 最後の適用のキーを 1 ビット変えて**ミスにすると
/// 「マスクの実体が動いた」で赤**になり、そのとき確保増分の主張は最後まで鳴らない。
///
/// # このヒット 1 適用が塞いだ穴（実測）
///
/// `show.rs` 手順 (3) の供給を**ヒット経路のときだけ** `set`（複製）へ戻す変異を当てると、
/// クレート全 187 件のうち**赤になるのは本檻のヒット 1 適用だけ**である（`--lib` 全走で
/// 186 passed / 1 failed・鳴るのは [`assert_mask_is_shared_not_copied`] の番地の側）。
/// 上 2 つの檻は設計上ミスしか踏まないため緑のままだった。
#[test]
fn the_hit_test_mask_is_supplied_by_sharing_not_by_copying() {
    for (window_dpi, num, den, salt) in [(192_u16, 2_u32, 1_u32, 0xD3_u8), (96, 1, 1, 0xD4)] {
        let mut world = super::test_support::make_world_with_gpu();
        let mut presenter = EmoPresenter::new();
        attach(&mut presenter, &mut world, window_dpi, salt);
        let scale = ScaleRatio::new(num, den).expect("非ゼロの比は構築できる");

        // 初回・輪番立ち上がり・定常のそれぞれで同じことが成り立つ（毎回ミスする）。
        let mut missed = None;
        for i in 0..WARMUP + STEADY {
            let seen = apply_and_observe(&mut presenter, &mut world, i, scale);
            let at = format!("k={num}/{den} の {i} 回目");
            assert_not_empty(&seen, &at);
            assert_mask_is_shared_not_copied(&seen, &at);
            missed = Some(seen);
        }
        let missed = missed.expect("反復は 1 回以上ある");

        // ヒット経路: 直前の適用と**同じ**キーをもう一度渡す。
        let last = WARMUP + STEADY - 1;
        let hit = apply_and_observe(&mut presenter, &mut world, last, scale);
        let at = format!("k={num}/{den} のヒット経路");
        assert_eq!(
            hit.mask, missed.mask,
            "{at}: マスクの実体が動いた＝引き当てがミスしている（ヒット経路を観測できていない\
             ＝以後の主張が空虚）"
        );
        assert_eq!(
            hit.cumulative, missed.cumulative,
            "{at}: 確保が起きた（ヒットなら合成もリサンプルもマスク再生成も走らない）"
        );
        assert_mask_is_shared_not_copied(&hit, &at);
        assert_not_empty(&hit, &at);
    }
}
