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
//! 2. **暖機の形**（Requirement 3.2）: 立ち上がりの確保が「実体ごとに一度」の裁定済みの形どおりに
//!    現れて 0 へ落ちる。**1 適用へ丸めない**——回る実体は 1 適用ずつずれて伸びるため、
//!    暖機を 1 適用と決め打つと残りの成長が黙って隠れる
//! 3. **合成先と表示バッファの先頭位置**: 実体そのものが回り続けている……**ただしこの主張は
//!    傍証であって単独の検出器ではない**。計数を動かさずに実体だけ作り直す実装は、解放直後の
//!    再確保が同じ番地を返すと素通りする。実測と、確実に殺す側の檻の所在は下の
//!    §番地の主張はどこまで効くか（実測）に置く
//! 4. **マスク輪番の集合不変**: 実体はちょうど [`WARMUP`] 種類（キャッシュの 3 本＋輪番の
//!    空き 1 枚）で、そこに新顔が混じらない
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
//! | A | 本ファイル k=2/1 の檻の「暖機の確保の形が……」 | 計数 | 4/4 |
//! | A | 本ファイル恒等 k の檻の「暖機の確保の形が……」 | 計数 | 4/4 |
//! | A | `presenter/budget_tests.rs` の `the_warm_up_allocates_once_per_rotating_buffer_then_settles` ほか計 10 本 | 計数・外形 | 4/4 |
//! | A | `presenter_budget_equivalence_tests.rs` の器の本数 | 番地 | 4/4 |
//! | B | `presenter/budget_tests.rs` の `a_smaller_extent_and_the_regrowth_allocate_nothing` | 計数 | 4/4 |
//! | B | `presenter/budget_tests.rs` の `the_compose_destination_seat_is_allocated_once_and_then_lent_again` | 番地 | 4/4 |
//! | B | 本ファイル恒等 k の檻の交代関係「前回の合成先席が表示バッファへ回っていない」 | 同時に生きている 2 実体の関係 | 4/4 |
//! | B | 本ファイル k=2/1 の檻の「合成先席が別の実体になった」 | 番地 | **1/4** |
//!
//! **変異 A は要件 7.1（容量 3）の実装で計数側へ移った。** 出所は `FrameBudget` が到達済み寸法
//! （高水位）を器のフィールドで覚えるのをやめ、**実体そのものの `Vec` 容量**を呼び出しの前後で
//! 読み比べる形になったことである（`presenter/budget.rs` 冒頭 §何をもって「確保した」と数えるか）。
//! 回収バッファを捨てて空を返せば容量 0 から伸びるので、必ず 1 件計数される——**以前は高水位が
//! 前の実体の値を覚えていたため計数に 1 件も現れず、走ごとに揺れる番地の一致（0/4）だけが
//! 頼りだった**。
//!
//! **変異 B は計数では捕まらない**（上の表のとおり本ファイルの k=2/1 の檻は 1/4）。理由は、容量の
//! 読み取りが**貸し出しの前**に行われるため、貸し出しの最中に実体を差し替えると「前の実体の容量」と
//! 「新しい実体が伸びた後の容量」を比べることになり、両者が一致すれば増えたと判定されないからで
//! ある。決定論的に殺すのは ⑴縮小 → 再拡大の檻（新しい実体は容量 0 から始まるので再拡大で必ず
//! 計数される）⑵席単体の番地の檻 ⑶恒等 k の交代関係、の 3 本である。
//!
//! 本ファイルに残る番地の主張（合成先席の同一性・表示バッファの集合・マスクの集合）は独立な
//! 裏取りとして置いてある。番地は解放直後の再確保で偶然一致し得るため、**単独では証拠に
//! ならない**という読み方は変わらない。
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

/// 巡回するキーの本数。**キャッシュ容量（3・要件 7.1）より大きい**こと自体が要件である。
///
/// 容量 1 の頃は 2 パターンの交互適用で毎回ミスした。容量 3 では 2 パターンは**両方とも表に
/// 収まってしまい、以後すべてヒットする**——ミス経路を主張する本ファイルの檻が空虚になる。
/// 巡回長を容量より 1 大きくすると、入ってくるキーが必ず 1 手前で追い出されているため
/// **毎適用がミスかつ毎適用で追い出しが 1 件成立する**（本番の「毎コマ引き当て外れ」と同じ形）。
const ROTATING_KEYS: usize = 4;

/// 合成キャッシュの容量（`cache.rs` の `CAPACITY`・要件 7.1 の裁定値）。
///
/// `cache.rs` 側は私有定数ゆえここから参照できない。値が食い違えば
/// [`a_steady_scaled_run_reuses_every_buffer_and_allocates_nothing`] の表示バッファ本数の主張が
/// 赤になる（＝黙ってずれない）。
const CACHE_CAPACITY: usize = 3;

/// 暖機の適用回数＝回る実体の本数。
///
/// 表示バッファ（キャッシュの 3 本）・マスク（3 本＋輪番の空き 1 枚）・恒等 k の交代（4 本）は
/// いずれも **1 適用ずつずれて**立ち上がるため、全ての実体が定常へ入るまでにこれだけかかる。
const WARMUP: usize = ROTATING_KEYS;

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
fn observe(presenter: &EmoPresenter, world: &World, binds: &BindSet, scale: ScaleRatio) -> Seen {
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

/// 相異なり（順序は問わない・番地の集合を数えるための補助）。
fn distinct<T: Ord + Copy>(items: impl Iterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = items.collect();
    v.sort_unstable();
    v.dedup();
    v
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

/// Requirement 3.1／3.2／6.1 観測完了（**リサンプル経路・k=2/1**）: 4 パターンの巡回適用
/// （毎回ミス）で暖機した後、定常状態の反復が 1 件も確保せず、合成先席の実体が動かず、表示
/// バッファがキャッシュの 3 本を回り、マスク輪番がちょうど 4 本で回り、下流供給が共有のままである。
///
/// # 暖機の形を丸めない（Requirement 3.2）
///
/// 立ち上がりの確保は `[1,1,1,1]` → `[0,1,0,1]` → `[0,1,0,1]` → `[0,0,0,1]` → 以後 0 という形で
/// 現れる（要件 7.1・容量 3）。読み方は
///
/// - `alloc_resample_dst` の 3 件＝**キャッシュが保持する表示バッファの本数**。表に空きがある
///   あいだは回収が成立しないので、3 本が 1 適用ずつ順に起こされる
/// - `alloc_mask` の 4 件＝キャッシュの 3 本＋輪番の空き 1 枚
/// - `alloc_compose_dst`・`alloc_xmap` は初回のみ（どちらも 1 本で固定）
///
/// 暖機を 1 適用へ丸めるとこれらが観測の外へ落ち、残りのバッファの成長が黙って隠れる。
///
/// # 殺す誤実装（**変異を実際に当てて、鳴った主張を書き取った**・task 7.3 で全件再測）
///
/// 推測で書かない——本 spec は「同じ寸法の反復では再利用を主張できない」を 4 度踏んでおり、
/// 5 度目は「どの主張が鳴るか」を取り違えた doc として現れた。以下は全て実行して観測した結果で
/// ある（走らせ方は `cargo test -p areka-emo-present --lib`・全走）。
///
/// 目を引くのは、**全件が定常の番地主張ではなく暖機の形（計数の指紋）で落ちる**ことである。
/// 立ち上がりの `[1,1,1,1]` → `[0,1,0,1]` → `[0,1,0,1]` → `[0,0,0,1]` という形は再利用の成立を
/// きわめて細く縛るため、定常の反復へ入る前にそこで死ぬ。裏を返せば、**定常の番地主張が単独で
/// 仕留めた変異は 1 件も無い**——これが上の「番地は傍証」という位置づけの実測上の根拠である。
///
/// - `seat::SurfaceSeat::lend` の冒頭で席を丸ごと作り直す（`*self = Self::default()`＝容量の
///   読み取りより**前**に差し替えるので計数へ現れる形）
///   → **「暖機の確保の形が Requirement 3.2 の……から外れている」で RED**（クレート全 202 件中
///     13 件が赤・本檻と恒等 k の檻を含む）
/// - 席の実体だけ差し替える（`self.surface = ComposedSurface::default()`＝上の §番地の主張は
///   どこまで効くか の**変異 B**）
///   → **本檻の計数の主張は鳴らない**。本檻で鳴り得るのは「合成先席が別の実体になった」（番地）
///     だが**走ごとに揺れる（1/4）**。決定論的に殺すのは `presenter/budget_tests.rs` の
///     `a_smaller_extent_and_the_regrowth_allocate_nothing`（計数）・
///     `the_compose_destination_seat_is_allocated_once_and_then_lent_again`（番地）と、
///     恒等 k の檻の交代関係である（いずれも 4/4・§番地の主張はどこまで効くか の表）
/// - 容量回収（`ComposeCache::take_recycled`）が満杯でなくても剥がす
///   → 暖機の形が崩れて **「暖機の確保の形が……」で RED**（全 17 件赤）。`display` の番地主張は
///     そこへ到達する前に落ちる
/// - `FrameBudget::display_buffer` が回収した `composed` を捨てて空バッファを返す（上の
///   §番地の主張はどこまで効くか の**変異 A**）
///   → 同じ「暖機の確保の形が……」で RED（4/4・クレート全 202 件中 13 件が赤）。**要件 7.1 の
///     実装で計数側へ移った**——以前は本檻が緑のままで、恒等 k の檻と席単体の檻だけが頼りだった
/// - マスク輪番を止めて毎コマ `AlphaMask::from_pbgra32` する
///   → 同じ「暖機の確保の形が……」で RED（全 12 件赤）。実体集合の主張までは到達しない
/// - 下流供給を `set`（実体の複製）へ戻す
///   → [`assert_mask_is_shared_not_copied`] の**番地の側**（「当たり判定へ供給したマスクが
///     キャッシュスロットの実体と別物」）で RED。参照数の側より先にこちらが鳴る
///   （全 3 件赤＝本檻・恒等 k の檻・A7 専用の檻）
#[test]
fn a_steady_scaled_run_reuses_every_buffer_and_allocates_nothing() {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    attach(&mut presenter, &mut world, 192, 0xD1);
    let scale = ScaleRatio::new(2, 1).expect("2/1 は構築できる");

    // 暖機: 立ち上がりの確保が「実体ごとに一度」の形で現れて 0 へ落ちる。
    let mut prev = [0_u64; 4];
    let mut warm_shape: Vec<[u64; 4]> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, i, scale);
        warm_shape.push(delta(prev, seen.cumulative));
        prev = seen.cumulative;
    }
    assert_eq!(
        warm_shape,
        vec![[1, 1, 1, 1], [0, 1, 0, 1], [0, 1, 0, 1], [0, 0, 0, 1]],
        "暖機の確保の形が Requirement 3.2 の「実体ごとに一度」から外れている\
         （表示バッファ 3 本＋マスク 4 本が 1 適用ずつずれて立ち上がる形）"
    );

    // 供給面の readback が非ゼロ（GPU まで実際に絵が載っていることの 1 度きりの確認）。
    let bytes = presenter.read_back(TargetId(0)).expect("read_back 失敗");
    assert!(
        !bytes.is_empty() && bytes.iter().any(|&b| b != 0),
        "供給面の readback が空（表示が実際には成立していない＝以後の観測が空虚）"
    );

    // 定常状態の基準を「一巡ぶん」（＝回る実体の本数）で取る——マスクは 4 本、表示バッファは
    // キャッシュの 3 本が順に出るため、基準 1 本では残りを新顔と誤認する。
    let mut base: Vec<Seen> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP + i, scale);
        let at = format!("定常の基準適用 {i}");
        assert_not_empty(&seen, &at);
        assert_mask_is_shared_not_copied(&seen, &at);
        base.push(seen);
    }
    let cumulative = base[0].cumulative;
    for (i, seen) in base.iter().enumerate() {
        assert_eq!(
            seen.cumulative, cumulative,
            "基準の適用 {i} で確保が起きた（暖機が終わっていない＝以後の比較が空虚）"
        );
    }
    let known_masks: Vec<*const AlphaMask> = distinct(base.iter().map(|s| s.mask));
    assert_eq!(
        known_masks.len(),
        WARMUP,
        "マスク輪番が {WARMUP} 本で回っていない（1 本へ潰れている／新しい実体を作っている）"
    );
    let known_displays: Vec<*const u8> = distinct(base.iter().map(|s| s.display));
    assert_eq!(
        known_displays.len(),
        CACHE_CAPACITY,
        "表示バッファがキャッシュの {CACHE_CAPACITY} 本で回っていない"
    );

    for i in 0..STEADY {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP * 2 + i, scale);
        let at = format!("定常 {i} 回目");

        assert_eq!(
            seen.cumulative, cumulative,
            "{at}: 累積計数が動いた（定常状態で確保が起きている）"
        );
        // 次の 3 本は計数の主張と**対で**読む（上の §番地の主張はどこまで効くか）。実体を作り
        // 直す変異は計数側で決定論的に死ぬようになったが、番地は解放直後の再確保で偶然一致し
        // 得るため単独では証拠にならない——独立な裏取りとして残している。
        assert_eq!(
            seen.native, base[0].native,
            "{at}: 合成先席が別の実体になった（常設席で回っていない）"
        );
        assert!(
            known_displays.contains(&seen.display),
            "{at}: 表示バッファが既知の {CACHE_CAPACITY} 本のどれでもない（回収した容量で回っていない）"
        );
        assert!(
            known_masks.contains(&seen.mask),
            "{at}: マスクが輪番の {WARMUP} 本のどれでもない（新しい実体を作っている）"
        );
        assert_mask_is_shared_not_copied(&seen, &at);
        assert_not_empty(&seen, &at);
    }
}

/// Requirement 3.1／3.2／6.1 観測完了（**交代経路・恒等 k**）: 拡大率が等倍（100% 表示の一般条件）
/// でも定常状態の確保は 0 で、合成先席と表示バッファは**ちょうど 4 本の実体を回し続ける**
/// （キャッシュの 3 本＋合成先席 1 本・要件 7.1）。
///
/// 恒等 k は合成先席と表示バッファを `swap` で入れ替える（コピーもリサンプルも起きない）。ゆえに
/// 番地は「不変」ではなく「4 本集合の巡回」になる——集合が 4 本を超えれば確保し直しており、
/// 減れば交代が起きていない（＝どこかで複写している）。
///
/// # 暖機の形
///
/// `[1,0,0,1]` が 4 適用ぶん、以後 0。合成先の代金が 4 適用に分かれるのが巡回する 4 本ぶんであり、
/// マスクも同じく 4 本ぶんである。リサンプル経路を通らないため `alloc_resample_dst`／`alloc_xmap`
/// は全適用で 0 のままである。
///
/// # 本檻が担う固有の役割（番地の偶然一致に強い唯一の関係）
///
/// 下の交代関係——「**前回の合成先席が今回の表示バッファへ回る**」——は、同時に生きている
/// 2 つの実体の役割入れ替えを主張する。解放も再確保も介在しないため、番地の偶然一致が入り込む
/// 余地が無い。`swap` を複写へ戻す変異はここで死ぬ。
///
/// なお逆向き（前回の表示バッファが今回の合成先席へ回る）は**もはや成り立たない**——容量 3 では
/// 追い出されるのは最も古い引き当てであって直前の挿入ではないため、合成先席へ回ってくるのは
/// 3 適用前に表示したバッファである。巡回の周期はその代わりに「連続する 4 適用の合成先席が
/// 互いに異なる」で固定する。
#[test]
fn a_steady_identity_run_rotates_through_exactly_four_buffers_and_allocates_nothing() {
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
        vec![[1, 0, 0, 1], [1, 0, 0, 1], [1, 0, 0, 1], [1, 0, 0, 1]],
        "恒等 k の暖機は巡回する 4 本・マスク 4 本ぶんが 1 適用ずつずれて現れる形のはず"
    );

    // 基準を一巡ぶん（4 適用）で取る。
    let mut base: Vec<Seen> = Vec::new();
    for i in 0..WARMUP {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP + i, scale);
        let at = format!("恒等 k 基準適用 {i}");
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
            "恒等 k の基準の適用 {i} で確保が起きた（暖機が終わっていない＝以後の比較が空虚）"
        );
    }
    // 交代の実体そのもの: 前回の合成先席が今回の表示バッファへ回る（複写化するとここが崩れる）。
    for i in 1..base.len() {
        assert_eq!(
            base[i].display,
            base[i - 1].native,
            "基準 {i}: 前回の合成先席が表示バッファへ回っていない（swap になっていない）"
        );
    }
    // 巡回の周期: 連続する 4 適用の合成先席は互いに異なる（1 本へ潰れた・本数が減った形を殺す）。
    let known_buffers: Vec<*const u8> = distinct(
        base.iter()
            .map(|s| s.native)
            .chain(base.iter().map(|s| s.display)),
    );
    assert_eq!(
        known_buffers.len(),
        WARMUP,
        "恒等 k で巡回する実体が {WARMUP} 本でない（確保し直している／潰れている）"
    );
    let known_masks: Vec<*const AlphaMask> = distinct(base.iter().map(|s| s.mask));
    assert_eq!(
        known_masks.len(),
        WARMUP,
        "恒等 k でマスク輪番が {WARMUP} 本で回っていない"
    );

    let mut prev = base[base.len() - 1].native;
    for i in 0..STEADY {
        let seen = apply_and_observe(&mut presenter, &mut world, WARMUP * 2 + i, scale);
        let at = format!("恒等 k 定常 {i} 回目");

        assert_eq!(
            seen.cumulative, cumulative,
            "{at}: 累積計数が動いた（恒等 k の定常状態で確保が起きている）"
        );
        assert!(
            known_buffers.contains(&seen.native),
            "{at}: 合成先席が既知の {WARMUP} 本のどれでもない（確保し直している）"
        );
        assert!(
            known_buffers.contains(&seen.display),
            "{at}: 表示バッファが既知の {WARMUP} 本のどれでもない（確保し直している）"
        );
        assert_eq!(
            seen.display, prev,
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

        prev = seen.native;
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
