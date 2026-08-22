//! 合成キャッシュの**容量と置換方式**を実 `apply_show` の上で固定する檻
//! （要件 7.1・2026-08-15 の開発者裁定で容量 1 → 3・置換方式 LRU）。
//!
//! # なぜ `cache_tests.rs` だけでは足りないのか（継ぎ目そのものを覆う）
//!
//! `cache_tests.rs` は [`ComposeCache`] を直接叩き、容量 3・LRU・回収の意味論を単体で固定する。
//! しかしそれは**`show.rs` がどの口を呼ぶか**を 1 つも言わない。置換が LRU として働くには、
//! 引き当てがヒットした回に [`ComposeCache::touch`]（最近使用順を動かす口）が呼ばれていなければ
//! ならない——`show.rs` がそこを [`ComposeCache::get`]（順を動かさない読み取り）へ替えると、
//! 置換は挿入順（FIFO）へ**静かに退化する**。
//!
//! この退化は
//!
//! - 表示バイトを 1 バイトも変えない（引き当ての判定規則は同じ）
//! - 当たり判定マスクを変えない
//! - 確保計数を変えない（追い出しは相変わらず 1 適用 1 件）
//!
//! ゆえに、既存のどの檻も赤にならない。本 spec は同じ形——**継ぎ目の両端は覆われているのに
//! 継ぎ目自体が覆われていない**——で一度失敗している（task 5.3 の `set_shared`／`set`）。
//! 本ファイルはその継ぎ目を実 `apply_show` の上で塞ぐ。
//!
//! # なぜ置換方式が load-bearing なのか
//!
//! 容量 3 という裁定は、実走行の適用列を **LRU で再生**した命中率（キャラ面 0.0% → 56.2%）を
//! 根拠に下りている（`remeasure-2026-08-15.md` §4）。FIFO へ退化すると、まばたきのように
//! 「直前に使った面へ戻る」列で命中しなくなり、裁定の数字が実装と対応しなくなる。
//!
//! # 観測の仕方（保持そのものを見る）
//!
//! 「どのキーが残っているか」は [`ComposeCache::get`] で直接読む。`get` は最近使用順を動かさない
//! ので、**檻が自分の観測で置換順を書き換えてしまうことがない**（順を動かす読み取りだと、
//! 保持を確かめた順番によって次の追い出し先が変わってしまう）。
//!
//! # 実時間を合否条件に使わない（Requirement 6.2）／純 x64 常設（Requirement 6.3）
//!
//! 判定量は保持の有無と件数だけである。環境変数ゲートも `#[ignore]` も持たない。
//!
//! [`ComposeCache`]: crate::cache::ComposeCache
//! [`ComposeCache::touch`]: crate::cache::ComposeCache::touch
//! [`ComposeCache::get`]: crate::cache::ComposeCache::get

use super::*;

use std::time::Duration;

use super::test_support::{build_target_assets, spawn_window_with_dpi};

/// 合成キャッシュの容量（`cache.rs` の `CAPACITY`・要件 7.1 の裁定値）。
const CACHE_CAPACITY: usize = 3;

/// native 外形（本ファイルは絵の中身を見ないので小さくてよい）。
const NATIVE_W: u32 = 8;
const NATIVE_H: u32 = 6;

/// surface id（`animations` を 1 件も持たないため bind 集合は合成結果に影響しない＝
/// 「キーだけが違う」入力を好きなだけ作れる）。
const SURFACE: u32 = 1000;

/// n 番目の合成入力キー（bind 集合だけで弁別する）。
fn key(n: u32) -> BindSet {
    BindSet::from_ids(n..=n)
}

/// GPU World ＋ DPI 付き窓 ＋ 装着済み target を 1 組作る（恒等 k）。
fn rig() -> (World, EmoPresenter) {
    let mut world = super::test_support::make_world_with_gpu();
    let mut presenter = EmoPresenter::new();
    let window = spawn_window_with_dpi(&mut world, 96);
    let (emo_world, atlas, _golden) = build_target_assets(NATIVE_W, NATIVE_H, 0xC1);
    presenter
        .attach_target(&mut world, TargetId(0), window, emo_world, atlas, 96)
        .expect("attach_target 失敗");
    (world, presenter)
}

/// `ShowSurface` を 1 回適用する（成立しなければ即座に落とす＝以後の観測を空虚にしない）。
fn show(presenter: &mut EmoPresenter, world: &mut World, n: u32) {
    let (tx, rx) = reply_channel::<PresentOutcome>();
    presenter.apply(
        world,
        PresentCommand::ShowSurface {
            target: TargetId(0),
            surface_id: SURFACE,
            binds: key(n),
            pattern: PatternState::default(),
            reply: Some(tx),
        },
    );
    assert!(
        matches!(rx.recv_timeout(Duration::from_secs(10)), Ok(Ok(()))),
        "前提: ShowSurface が成立すること（成立しない適用の観測は意味を持たない）"
    );
}

/// キー `n` のエントリが保持されているか（最近使用順は動かさない）。
fn holds(presenter: &EmoPresenter, n: u32) -> bool {
    presenter
        .targets
        .get(&TargetId(0))
        .expect("装着済み target")
        .cache
        .get(SURFACE, &key(n), &PatternState::default(), ScaleRatio::ONE)
        .is_some()
}

/// 要件 7.1 観測完了（**本番経路の容量**）: 実 `apply_show` を通しても、異なる 3 キーが同時に
/// 保持され、4 本目でちょうど 1 本が落ちる。
///
/// `cache_tests.rs` の同型の檻が `ComposeCache` 単体で言うことを、**presenter が実際に使う
/// 経路**でも言う。容量を 1 へ戻す・2 へ縮める・4 以上へ広げる、のいずれもここで赤になる。
#[test]
fn the_real_apply_path_retains_exactly_the_approved_number_of_entries() {
    let (mut world, mut presenter) = rig();

    for n in 1..=CACHE_CAPACITY as u32 {
        show(&mut presenter, &mut world, n);
    }
    for n in 1..=CACHE_CAPACITY as u32 {
        assert!(
            holds(&presenter, n),
            "容量 {CACHE_CAPACITY}: 異なる {CACHE_CAPACITY} キーは同時に保持される（キー {n} が落ちている）"
        );
    }

    // 上限を超える 1 本。落ちるのはちょうど 1 本（最も古い引き当て＝キー 1）である。
    show(&mut presenter, &mut world, CACHE_CAPACITY as u32 + 1);
    let retained: Vec<u32> = (1..=CACHE_CAPACITY as u32 + 1)
        .filter(|n| holds(&presenter, *n))
        .collect();
    assert_eq!(
        retained,
        vec![2, 3, 4],
        "上限超過の挿入で落ちるのは最も古い引き当ての 1 本だけ（保持されているのは {retained:?}）"
    );
}

/// 要件 7.1 観測完了（**継ぎ目そのもの**）: `apply_show` の引き当てがヒットすると、その
/// エントリは**最近使用へ引き上げられる**——`show.rs` は順を動かす口を呼んでいる。
///
/// # LRU と FIFO が食い違う形を作る
///
/// キー 1・2・3 を表示してから**キー 1 をもう一度表示する**（＝ヒット）。この時点で
///
/// - 挿入順の最古は キー 1
/// - 最近使用の最古は キー 2
///
/// と 2 つの順序が食い違う。ここでキー 4 を表示すると
///
/// - **LRU（正）**: キー 2 が落ち、キー 1 は残る
/// - **FIFO（誤）**: キー 1 が落ち、キー 2 が残る
///
/// と結果が反転するので、1 本の檻で `show.rs` の呼ぶ口が決まる。
///
/// # この檻だけが捕まえる退化
///
/// `show.rs` の手順 (1) を [`ComposeCache::touch`] から
/// `cache.get(..).is_some()` へ替える変異は、**表示バイト・当たり判定マスク・確保計数のどれも
/// 変えない**。クレートの他の檻はすべて緑のままである（実測）。
///
/// [`ComposeCache::touch`]: crate::cache::ComposeCache::touch
#[test]
fn a_cache_hit_through_apply_show_promotes_the_entry_to_most_recently_used() {
    let (mut world, mut presenter) = rig();

    for n in 1..=CACHE_CAPACITY as u32 {
        show(&mut presenter, &mut world, n);
    }

    // 挿入順の最古（キー 1）を**本番経路で**引き当て直す＝ヒット。
    show(&mut presenter, &mut world, 1);
    assert!(
        holds(&presenter, 1),
        "前提: 再表示したキー 1 は保持されている"
    );

    // 上限を超える 1 本。LRU ならキー 2 が落ち、FIFO ならキー 1 が落ちる。
    show(&mut presenter, &mut world, CACHE_CAPACITY as u32 + 1);

    assert!(
        holds(&presenter, 1),
        "直前に引き当てたキー 1 が落ちた＝`show.rs` の引き当てが最近使用順を動かしていない\
         （置換が挿入順＝FIFO へ退化している。容量 3 の裁定は LRU 再生の命中率が根拠である）"
    );
    assert!(
        !holds(&presenter, 2),
        "最も古い引き当てのキー 2 が残っている＝追い出し先が LRU になっていない"
    );
    assert!(holds(&presenter, 3), "キー 3 は落ちない");
    assert!(holds(&presenter, 4), "いま表示したキーは保持される");
}

/// 要件 7.1／3.1 観測完了: 保持されているキーの再表示は**ヒットする**（合成し直さない）。
///
/// 容量 3 の裁定はこの一点のためにある——まばたきのように数コマを行き来する列で、直前だけでなく
/// **数コマ前の面**にも命中することが命中率 0.0% → 56.2% の中身である。
///
/// ヒットが成立したことは**マスクの実体が動かないこと**で示す（ミスなら輪番が 1 つ進んで別の
/// `Arc` へ移るため、番地の偶然一致が入り込む余地が無い）。
#[test]
fn showing_a_face_from_a_few_frames_ago_hits_instead_of_recomposing() {
    let (mut world, mut presenter) = rig();

    let mask_of = |presenter: &EmoPresenter, n: u32| {
        std::sync::Arc::as_ptr(
            &presenter
                .targets
                .get(&TargetId(0))
                .expect("装着済み target")
                .cache
                .get(SURFACE, &key(n), &PatternState::default(), ScaleRatio::ONE)
                .expect("保持されているキー")
                .mask,
        )
    };

    // 3 コマぶん表示する（キー 1 は 2 コマ前になる）。
    for n in 1..=CACHE_CAPACITY as u32 {
        show(&mut presenter, &mut world, n);
    }
    let before = mask_of(&presenter, 1);

    // 2 コマ前の面へ戻る＝ヒット。容量 1 ではここが必ずミスだった。
    show(&mut presenter, &mut world, 1);
    assert_eq!(
        mask_of(&presenter, 1),
        before,
        "2 コマ前の面の再表示でマスクの実体が動いた＝引き当てがミスしている\
         （容量 3 の裁定が期待した命中が成立していない）"
    );

    // 陰性対照: 一度も表示していないキーは当然ミスする（上の一致が恒真でないことの担保）。
    assert!(
        !holds(&presenter, 99),
        "未表示のキーが保持されている＝引き当てが常に成立している（檻が恒真）"
    );
    show(&mut presenter, &mut world, 99);
    assert!(holds(&presenter, 99), "表示したキーは保持される");
}
