//! 「常時最前面ではない普通の窓」という既定の重なりと、常時最前面の政策を後から
//! 足しても要件 1 の隣接が壊れないことを固定するテスト（要件 8.1／8.2）。
//!
//! # 兄弟ファイルとの分担
//!
//! 判断そのものが常時最前面を返さないこと（返り値の型にその値が存在しないこと）は
//! `zorder_pair_tests.rs` の
//! `decide_pair_fix_covers_every_arm_and_uses_a_band_window_as_base_only_for_a_character_in_the_band` が既に固定して
//! いる。本ファイルが受け持つのは**窓の側**である——実際の窓を作り、確立系と維持系を
//! 通したあとに OS から拡張スタイルを読み戻して、常時最前面の印が付いていないことを
//! 測る。判断の型が正しくても、適用の経路（`SetWindowPos` の挿入位置）が窓を常時最前面の
//! 帯へ押し込む余地は型では消せないため、窓から測る側が要る。
//!
//! # 常時最前面の「帯」とは
//!
//! Windows はトップレベル窓を 2 つの帯に分けて重ねる——常時最前面（`WS_EX_TOPMOST`）の
//! 一群が、そうでない一群より必ず手前に来る。`\v` 相当の政策を後から足すというのは、
//! ゴースト一式をこの帯へ移すということである（要件 8.2）。よって独立性の主張は
//! 「一式が同じ帯に居ても、ペアの隣接は同じように成立する」という形で確かめられる。
//!
//! **組と鎖は帯を書かない**。本番でゴーストの窓を帯へ入れる・帯から出すのは帯の働きの
//! ファイル（`window_band.rs`。ほかの帯の書き手はその頭の doc に挙げてある）であり、ここで
//! 帯を作るのはテストが自分で生成した窓に対してだけである。
//! 組の是正が帯の窓を挿入位置に指すのは、キャラクターも帯に居るときだけである
//! （`stay-on-top-directives` 要件 2.4・末尾の 1 件）。
//!
//! # トップレベル窓と子窓の使い分け
//!
//! 重なりの隣接そのものは同一テストバイナリ内のトップレベル窓では決定論にならない
//! （他テスト・他プロセスの窓が割り込む。加えて `HWND_TOP` がデスクトップのどこまで
//! 持ち上がるかは、そのとき前面に居るのが誰かに左右される——実測で、同じ手順が通常帯の
//! 最上まで届く回と自プロセスの窓の手前で止まる回の両方を観測した）。そのため隣接を測る側は
//! 兄弟の維持系テストと同じく私有の親窓＋子窓で行う（子窓の兄弟列は親が閉じており、
//! `HWND_TOP` は必ずその列の先頭になる）。
//! 一方、常時最前面の印そのものはトップレベル窓でしか意味を持たないので、印を測る側は
//! トップレベル窓で作る（印の読み戻しは割り込みの影響を受けず決定論的である）。
//!
//! # 明示実行へ隔離した 3 本
//!
//! 次の 3 本は `#[ignore]` と本体 1 行目の門（`require_explicit_real_window_run`）で
//! 既定の走行から外してある（根治の引受先は `zorder-chain-residue` の A-1）。
//!
//! - `pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares`（帯の印だけを見る）
//! - `the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band`
//! - `a_pair_inside_the_band_is_fixed_directly_above_the_character_within_the_band`

use bevy_ecs::prelude::*;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, HWND_BOTTOM, HWND_NOTOPMOST, SW_SHOWNA, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::api::{clear_window_owner, get_window_above, get_window_below, set_window_owner};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::{
    InsertSpec, PairFixDecision, PairObservation, PairTrigger, decide_pair_fix,
    measure_below_is_always_on_top, measure_window_above, measure_window_below,
    measure_window_is_always_on_top,
};
use crate::ecs::window::{
    KeepDirectlyAbove, OwnerLink, ReassertZOrder, SetWindowPosCommand, ZOrderPairStrategy,
    flush_window_pos_commands,
};

use super::pair_fix_command;
use crate::ecs::window::zorder_pair_test_support::{
    CAPTURE_DIRECTIVES, FIX_TAG, SKIP_TAG, VERIFY_FAILED_TAG, assert_line_count,
    create_banded_test_parent, create_test_child, create_window, destroy_test_window, insert_after,
    lines_with, maintain_schedule, pair_schedule, require_explicit_real_window_run, spawn_window,
};

// -------------------------------------------------------------------------
// 帯の印を測る実窓のヘルパ（共通の下ごしらえは `zorder_pair_test_support`）
// -------------------------------------------------------------------------

/// 本番のゴースト窓と同じ拡張スタイルのトップレベル窓（`WS_VISIBLE` は付けない
/// ——画面に何も出さずに印だけを測るため）。
fn create_ghost_shaped_toplevel(title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0),
        WINDOW_STYLE(WS_POPUP.0),
        None,
    )
}

/// 常時最前面の帯に居るトップレベル窓（`\v` 相当の政策を足した後の姿を模す）。
fn create_always_on_top_toplevel(title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(WS_EX_LAYERED.0 | WS_EX_TOOLWINDOW.0 | WS_EX_TOPMOST.0),
        WINDOW_STYLE(WS_POPUP.0),
        None,
    )
}

fn ex_style_of(hwnd: HWND) -> isize {
    // SAFETY: Win32 境界。自プロセス所有の実 HWND に対する読み取りのみ。
    unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) }
}

/// 常時最前面の印が付いているか（OS から読み戻す）。
fn is_always_on_top(hwnd: HWND) -> bool {
    (ex_style_of(hwnd) as u32) & WS_EX_TOPMOST.0 != 0
}

/// 活性化を伴わずに窓を見せる（実測層は可視の窓だけを隣と数えるため、隣接を測る窓は
/// 可視でなければならない）。0x0 の道具窓なので画面にもタスクバーにも現れない。
fn show_without_activating(hwnd: HWND) {
    // SAFETY: Win32 境界。自プロセス所有の窓を活性化せずに表示する。
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNA);
    }
}

/// 窓を重なりの最背面へ落とす（是正が実際に動かす余地を作るための下準備）。
fn sink_to_bottom(hwnd: HWND) {
    // SAFETY: Win32 境界。自プロセス所有の実 HWND に対する重なりのみの変更。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .expect("SetWindowPos should sink a self-owned test window");
    }
}

// -------------------------------------------------------------------------
// 要件 8.2: 常時最前面の政策を足しても、ペアの隣接は同じように成立する
// -------------------------------------------------------------------------

/// 帯の内外で変わってはならない観測（隣接の成立と、出た記録の本数）。
///
/// 帯の所属そのものは当然に食い違うので、この構造体には入れず別に確かめる。
#[derive(Debug, PartialEq, Eq)]
struct ConvergenceOutcome {
    /// バルーンの最も近い可視の背後がキャラ窓
    balloon_directly_above_character: bool,
    /// キャラ窓の最も近い可視の手前がバルーン
    character_directly_below_balloon: bool,
    fix_records: usize,
    verify_failed_records: usize,
    skip_records: usize,
}

/// 「バルーン／割り込み／キャラ」の順（手前 → 背後）から是正を 1 巡走らせ、
/// 隣接が成立するかと記録の本数を返す。返り値の 2 つ目は処理後の帯の所属。
fn run_pair_convergence(tag: PCWSTR, always_on_top: bool) -> (ConvergenceOutcome, bool) {
    let parent = create_banded_test_parent(tag, always_on_top);
    assert_eq!(
        is_always_on_top(parent),
        always_on_top,
        "前提: 意図した帯に一式を置けているはず（常時最前面={always_on_top}）"
    );

    let character_hwnd = create_test_child(parent, w!("character"));
    let intruder_hwnd = create_test_child(parent, w!("intruder"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    insert_after(intruder_hwnd, balloon_hwnd);
    insert_after(character_hwnd, intruder_hwnd);
    assert_eq!(
        get_window_below(balloon_hwnd).expect("GetWindow は成功するはず"),
        Some(intruder_hwnd),
        "前提: バルーンの 1 つ背後は割り込み窓（＝キャラの隣に居ない）"
    );

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
    world.entity_mut(balloon).insert(ReassertZOrder::default());

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 1 巡目で判断し指令を積み、flush で OS の重なりが動き、2 巡目で実測照合する。
        schedule.run(&mut world);
        flush_window_pos_commands();
        schedule.run(&mut world);
    });

    let outcome = ConvergenceOutcome {
        balloon_directly_above_character: get_window_below(balloon_hwnd)
            .expect("GetWindow は成功するはず")
            == Some(character_hwnd),
        character_directly_below_balloon: get_window_above(character_hwnd)
            .expect("GetWindow は成功するはず")
            == Some(balloon_hwnd),
        fix_records: lines_with(&out, FIX_TAG).len(),
        verify_failed_records: lines_with(&out, VERIFY_FAILED_TAG).len(),
        skip_records: lines_with(&out, SKIP_TAG).len(),
    };
    let band_after = is_always_on_top(parent);
    destroy_test_window(parent);
    (outcome, band_after)
}

/// 一式が常時最前面の帯に居ても、居なくても、ペアの隣接はまったく同じように成立する
/// （要件 8.2——隣接を保つ仕組みと、一式を最前面へ上げる政策とは互いに独立している）。
///
/// 同じ筋書きを帯の外と中で 1 度ずつ走らせ、観測が**一致すること**を主張する。
/// 片側だけを見て「成立した」と言うと、帯の有無に関係なく成立する主張と区別できない
/// ——2 本並べて突き合わせることで、独立性そのものが観測の対象になる。
///
/// 組と鎖は帯を書かない（帯を当てるのは `window_band.rs`。ここでは親窓をテストが自分で帯へ置く）。
#[test]
fn pair_adjacency_converges_identically_inside_and_outside_the_always_on_top_band() {
    let (outside, outside_band) = run_pair_convergence(w!("wintf-zorder-band-outside"), false);
    let (inside, inside_band) = run_pair_convergence(w!("wintf-zorder-band-inside"), true);

    // まず帯の中で不変条件が成立していること（要件 1 の隣接）。
    assert!(
        inside.balloon_directly_above_character,
        "常時最前面の帯の中でもバルーンはキャラのすぐ手前に来るはず: {inside:?}"
    );
    assert!(
        inside.character_directly_below_balloon,
        "逆向きの実測でも隣接しているはず: {inside:?}"
    );
    assert_eq!(inside.fix_records, 1, "是正の記録は 1 本: {inside:?}");
    assert_eq!(inside.verify_failed_records, 0, "不一致は無い: {inside:?}");
    assert_eq!(inside.skip_records, 0, "見送っていない: {inside:?}");

    // そして帯の外と一致すること（＝常時最前面の有無は隣接の成立に何も影響しない）。
    assert_eq!(
        inside, outside,
        "常時最前面の有無で隣接の成立も記録も変わってはならない（要件 8.2）"
    );

    // 対照: 帯の読み取りが常に同じ答えを返しているのではない（2 通りが実際に出ている）。
    assert!(!outside_band, "対照: 帯へ入れていない側は最後まで帯の外");
    assert!(
        inside_band,
        "常時最前面の指定は是正のあとも残っている（是正が政策を剥がさない）"
    );
}

// -------------------------------------------------------------------------
// 要件 8.1／8.2: 是正の指令は帯の所属を動かさない
// -------------------------------------------------------------------------

/// 一式が同じ帯を共有していれば、是正の 3 通りの指令はどれも帯の所属を動かさない。
///
/// 是正は `SetWindowPos` の挿入位置指定だけで行う。この API は挿入位置に指定した窓の帯へ
/// 対象を引き込む性質を持つため、「判断が常時最前面を返さない」だけでは窓が帯を跨がない
/// 保証にならない——実際に指令を発行して印を読み戻す必要がある。
///
/// 末尾の対照は 2 つ置いてある。①帯から外す指令を出せば印は実際に落ちる（読み取りが
/// 恒真ではない）。②帯の外の窓を帯の中の窓のすぐ背後へ入れると印が付く（＝挿入位置が
/// 帯を跨ぐと所属が動くという OS の性質そのもの）。
///
/// **本テストが帯を跨がないのは、挿入位置に相方そのものを渡しているからである**——
/// 本番の腕①の挿入位置は実測（キャラ窓の最も近い可視の手前）で決まるので、そこに他アプリの
/// 常時最前面窓が居ると、対照②の性質によってバルーン窓が帯へ引き込まれる。この形は
/// `zorder_pair.rs` の `place_above` が [`InsertSpec::TopOfNormalBand`] を選ぶことで
/// 是正済みであり、その主張は下の
/// [`a_fix_whose_insert_position_is_always_on_top_keeps_the_balloon_out_of_the_band`] が
/// 持つ。対照②はその危険が実在する（＝是正が要る）ことを実測で確定させるために置いてある。
#[test]
#[ignore = "帯から外す指令が効かない回がある（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）"]
fn pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares() {
    require_explicit_real_window_run();

    let character = create_always_on_top_toplevel(w!("wintf-zorder-band-cmd-character"));
    let balloon = create_always_on_top_toplevel(w!("wintf-zorder-band-cmd-balloon"));
    assert!(
        is_always_on_top(character) && is_always_on_top(balloon),
        "前提: ペアの 2 窓とも常時最前面の帯に居るはず"
    );

    // 腕①（`PlaceAboveOverBelow` の `After`・要件 1.2）の形の指令。挿入位置にキャラそのものを渡すので
    // バルーンはキャラのすぐ背後へ入る（本番はキャラの最も近い可視の手前の窓を渡す）。見るのは帯の印だけ。
    SetWindowPosCommand::enqueue(pair_fix_command(balloon, InsertSpec::After(character)));
    flush_window_pos_commands();
    assert!(is_always_on_top(balloon), "腕①でバルーンが帯から出た");
    assert!(is_always_on_top(character), "腕①でキャラが帯から出た");

    // 腕②「最上位へ」（キャラより手前に可視の窓が無い縁）
    SetWindowPosCommand::enqueue(pair_fix_command(balloon, InsertSpec::TopEdge));
    flush_window_pos_commands();
    assert!(is_always_on_top(balloon), "腕②でバルーンが帯から出た");

    // 腕③「キャラをバルーンのすぐ背後へ」（要件 1.3）
    SetWindowPosCommand::enqueue(pair_fix_command(character, InsertSpec::After(balloon)));
    flush_window_pos_commands();
    assert!(is_always_on_top(character), "腕③でキャラが帯から出た");
    assert!(is_always_on_top(balloon), "腕③でバルーンが帯から出た");

    // 対照①: 帯から外す指令なら印は実際に落ちる。
    let control = create_always_on_top_toplevel(w!("wintf-zorder-band-cmd-control"));
    assert!(is_always_on_top(control), "前提: 対照窓は帯の中で始まる");
    // SAFETY: Win32 境界。自プロセス所有の実 HWND を帯の外へ移す。
    unsafe {
        SetWindowPos(
            control,
            Some(HWND_NOTOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .expect("SetWindowPos should move a self-owned window out of the band");
    }
    assert!(
        !is_always_on_top(control),
        "対照①: 印の読み取りが常に真を返しているわけではない"
    );

    // 対照②: 挿入位置が帯を跨ぐと所属は動く（上の 3 通りが跨がない理由の裏返し）。
    let outsider = create_ghost_shaped_toplevel(w!("wintf-zorder-band-cmd-outsider"));
    assert!(!is_always_on_top(outsider), "前提: 帯の外で始まる");
    SetWindowPosCommand::enqueue(pair_fix_command(outsider, InsertSpec::After(balloon)));
    flush_window_pos_commands();
    assert!(
        is_always_on_top(outsider),
        "対照②: 帯の中の窓のすぐ背後へ入れると常時最前面になる（OS の性質）"
    );

    for hwnd in [character, balloon, control, outsider] {
        destroy_test_window(hwnd);
    }
}

// -------------------------------------------------------------------------
// 要件 8.1: 既定の一式は帯の外のまま
// -------------------------------------------------------------------------

/// 本番と同じ形のゴースト窓へ確立系と維持系を通しても、常時最前面の印は付かない。
///
/// areka 側の組立が `WindowStyle` に常時最前面を含めないことは
/// `spawn_assembly_tests::t_i2_no_window_has_ws_ex_topmost` が固定している（`ex_style` を
/// 完全一致で押さえているので、最小化・タスクバー露出の印が紛れ込む余地も無い）。
/// 本テストが足すのは**その後**である——ペアの機構（owner の確立と是正の適用）を実際に
/// 通したあとに、OS から拡張スタイルを読み戻して 1 ビットも変わっていないことを測る。
///
/// 空回りで緑にならないよう、機構が実際に働いたこと（owner が張られたこと）を併せて
/// 主張し、印の読み取り自体が生きていること（帯の中の窓では真になること）も対照で示す。
#[test]
fn the_pair_machinery_never_puts_ghost_shaped_windows_into_the_always_on_top_band() {
    let character_hwnd = create_ghost_shaped_toplevel(w!("wintf-zorder-default-character"));
    let balloon_hwnd = create_ghost_shaped_toplevel(w!("wintf-zorder-default-balloon"));
    assert!(
        !is_always_on_top(character_hwnd) && !is_always_on_top(balloon_hwnd),
        "前提: 本番と同じ形のゴースト窓は帯の外で始まる（要件 8.1）"
    );
    let ex_before = (ex_style_of(character_hwnd), ex_style_of(balloon_hwnd));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });

    let mut schedule = pair_schedule();
    // 記録は本テストの主張の対象ではないが、確立と是正の error 記録を素通しにしないよう
    // 捕捉の中で回す。
    let _out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        flush_window_pos_commands();
        schedule.run(&mut world);
        flush_window_pos_commands();
        schedule.run(&mut world);
    });

    // 空回り検知: 確立系が実際に owner を張った（機構を通っていないのに緑にならない）。
    assert!(
        world.entity(balloon).contains::<OwnerLink>(),
        "前提: 確立系が実際に owner を張っているはず（通っていなければ主張が空になる）"
    );

    assert!(
        !is_always_on_top(character_hwnd),
        "キャラ窓に常時最前面の印が付いた（要件 8.1）"
    );
    assert!(
        !is_always_on_top(balloon_hwnd),
        "バルーン窓に常時最前面の印が付いた（要件 8.1）"
    );
    assert_eq!(
        (ex_style_of(character_hwnd), ex_style_of(balloon_hwnd)),
        ex_before,
        "拡張スタイルはペアの機構を通しても 1 ビットも変わらないはず"
    );

    // 対照: 印の読み取りは帯の中の窓では実際に真になる。
    let probe = create_always_on_top_toplevel(w!("wintf-zorder-default-probe"));
    assert!(
        is_always_on_top(probe),
        "対照: 読み取りが常に偽を返しているわけではない"
    );

    for hwnd in [character_hwnd, balloon_hwnd, probe] {
        destroy_test_window(hwnd);
    }
}

// -------------------------------------------------------------------------
// 要件 8.1／1.2: 挿入位置が常時最前面の窓だったときの是正
// -------------------------------------------------------------------------

/// 実測した挿入位置が他アプリの常時最前面窓だったとき、是正はバルーンを帯へ引き込まない。
///
/// # なぜこの筋書きが実デスクトップで起きるのか
///
/// キャラ窓が**可視の通常窓のうち最前面**に居るとき、キャラ窓の「最も近い可視の手前」は
/// 常時最前面の帯の最下段——すなわち他アプリの常時最前面窓——になる。実測したこの機の
/// デスクトップは可視 30〜35 枚のうち常時最前面が 4〜6 枚あり、帯の境目は実在する。
/// `SetWindowPos` の `hWndInsertAfter` に帯の中の窓を渡すと、対象窓には `WS_EX_TOPMOST` が
/// **付いてしまう**（本ファイル既存の対照②が固定している OS の性質）。
///
/// この形が診断で見つからないのは、引き込まれた後もバルーンはキャラのすぐ手前に居るためで
/// ある——次巡の実測照合は一致し、`verify-failed` も `error!` も出ない。露見するのは
/// 他アプリを活性化したときの `sink-observed` だけ（要件 4.1／4.2 の毀損）である。
///
/// 主張は帯の所属そのもの（要件 8.1）で、対照として同じ窓・同じ経路で**帯へ引き込む指令**も
/// 出す（読み取りが常に偽を返しているのではないこと、および危険が実在することの両方を示す）。
#[test]
fn a_fix_whose_insert_position_is_always_on_top_keeps_the_balloon_out_of_the_band() {
    let character = create_ghost_shaped_toplevel(w!("wintf-zorder-aot-insert-character"));
    let balloon = create_ghost_shaped_toplevel(w!("wintf-zorder-aot-insert-balloon"));
    // 他アプリの常時最前面窓の代役。自プロセスの窓で作るのは、他プロセスの窓を挿入位置に
    // 渡すと `SetWindowPos` がアクセス拒否になることがあるためである（実測）。
    let foreign_top = create_always_on_top_toplevel(w!("wintf-zorder-aot-insert-foreign"));
    assert!(
        !is_always_on_top(character) && !is_always_on_top(balloon),
        "前提: ペアの 2 窓は帯の外で始まる（要件 8.1）"
    );
    assert!(is_always_on_top(foreign_top), "前提: 代役は帯の中に居る");

    // キャラが通常帯の最上に居る形の観測——キャラの最も近い可視の手前は帯の中の窓である。
    let observation = PairObservation {
        trigger: PairTrigger::Reassert,
        strategy: ZOrderPairStrategy::default(),
        above_alive: true,
        below_alive: true,
        above_hwnd: Some(balloon),
        below_hwnd: Some(character),
        // バルーンの最も近い可視の背後はキャラではない（＝まだ隣接していない）
        measured_below_of_above: None,
        measured_above_of_below: Some(foreign_top),
        // 実測層が測った帯の所属（本テストでは代役の窓から実際に読み取る）
        measured_above_of_below_is_always_on_top: measure_window_is_always_on_top(foreign_top),
        // ペアは帯の外で始まる（今の振る舞い）
        measured_below_is_always_on_top: false,
    };
    let PairFixDecision::PlaceAboveOverBelow { insert_after } = decide_pair_fix(&observation)
    else {
        panic!("再断行の要求はバルーンを手前へ寄せる腕になるはず");
    };

    SetWindowPosCommand::enqueue(pair_fix_command(balloon, insert_after));
    flush_window_pos_commands();
    assert!(
        !is_always_on_top(balloon),
        "挿入位置が常時最前面の窓でも、バルーンに常時最前面の印を付けてはならない（要件 8.1）"
    );
    assert!(
        !is_always_on_top(character),
        "キャラ窓も帯の外のままであるはず"
    );

    // 対照: 同じ経路で帯の中の窓のすぐ背後へ入れる指令を出せば、印は実際に付く
    // （上の主張が「読み取りが常に偽」による恒真になっていない）。
    let outsider = create_ghost_shaped_toplevel(w!("wintf-zorder-aot-insert-outsider"));
    assert!(!is_always_on_top(outsider), "前提: 対照窓は帯の外で始まる");
    SetWindowPosCommand::enqueue(pair_fix_command(outsider, InsertSpec::After(foreign_top)));
    flush_window_pos_commands();
    assert!(
        is_always_on_top(outsider),
        "対照: 帯の中の窓を挿入位置にすると常時最前面になる（この危険が実在する）"
    );

    for hwnd in [character, balloon, foreign_top, outsider] {
        destroy_test_window(hwnd);
    }
}

/// 通常帯の最上へ置く是正でも、バルーンはキャラのすぐ手前に入る（要件 1.2）。
///
/// # なぜ隣接だけ子窓で測るのか
///
/// 常時最前面の印はトップレベル窓でしか意味を持たないので、印の主張は上の
/// [`a_fix_whose_insert_position_is_always_on_top_keeps_the_balloon_out_of_the_band`] が
/// トップレベル窓で行う。一方、隣接そのものは同一テストバイナリ内のトップレベル窓では
/// 決定論にならない——`HWND_TOP` がデスクトップのどこまで持ち上がるかは、そのとき前面に
/// 居るのが誰かに左右される（実測: 同じ手順でも通常帯の最上まで届く回と、自プロセスの窓の
/// 手前までで止まる回があった）。よって隣接は本ファイル冒頭の方針どおり私有の親窓＋子窓で
/// 測る。子窓の兄弟列は親が閉じているため、`HWND_TOP` は必ず兄弟列の先頭になる。
///
/// 筋書きは本番の欠陥と同じ形にしてある——キャラが自分の帯の**最上**に居り、その手前には
/// 指してはならない窓（常時最前面の窓）しか居ない。是正はその窓を指さず帯の最上へ置き、
/// 結果としてキャラのすぐ手前に入る。
#[test]
fn the_top_of_normal_band_fix_still_lands_the_balloon_directly_above_the_character() {
    let parent = create_banded_test_parent(w!("wintf-zorder-aot-adjacency"), false);
    let character_hwnd = create_test_child(parent, w!("character"));
    let intruder_hwnd = create_test_child(parent, w!("intruder"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    // 手前 → 背後: キャラ／割り込み／バルーン（キャラが自分の帯の最上に居る形）
    insert_after(intruder_hwnd, character_hwnd);
    insert_after(balloon_hwnd, intruder_hwnd);
    assert_eq!(
        measure_window_above(character_hwnd),
        None,
        "前提: キャラは兄弟列の最上に居る（本番でキャラが通常帯の最上に居るのと同じ形）"
    );
    assert_ne!(
        measure_window_below(balloon_hwnd),
        Some(character_hwnd),
        "前提: バルーンはまだキャラのすぐ手前に居ない"
    );

    // キャラの手前に居るのが常時最前面の窓だった、という観測。
    let foreign_top = create_always_on_top_toplevel(w!("wintf-zorder-aot-adjacency-foreign"));
    let observation = PairObservation {
        trigger: PairTrigger::Reassert,
        strategy: ZOrderPairStrategy::default(),
        above_alive: true,
        below_alive: true,
        above_hwnd: Some(balloon_hwnd),
        below_hwnd: Some(character_hwnd),
        measured_below_of_above: None,
        measured_above_of_below: Some(foreign_top),
        measured_above_of_below_is_always_on_top: measure_window_is_always_on_top(foreign_top),
        // ペアは帯の外で始まる（今の振る舞い）
        measured_below_is_always_on_top: false,
    };
    assert_eq!(
        decide_pair_fix(&observation),
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::TopOfNormalBand,
        },
        "前提: この観測は通常帯の最上へ置く腕になる"
    );

    SetWindowPosCommand::enqueue(pair_fix_command(balloon_hwnd, InsertSpec::TopOfNormalBand));
    flush_window_pos_commands();
    assert_eq!(
        measure_window_below(balloon_hwnd),
        Some(character_hwnd),
        "帯の最上へ置いた後、バルーンの最も近い可視の背後はキャラであるはず（要件 1.2）"
    );
    assert_eq!(
        measure_window_above(character_hwnd),
        Some(balloon_hwnd),
        "逆向きの実測でも隣接しているはず"
    );

    // 対照: 同じ配置から割り込み窓を挿入位置にすると隣接は成立しない
    // （上の主張が「何を指しても隣接する」による恒真になっていない）。
    insert_after(balloon_hwnd, intruder_hwnd);
    SetWindowPosCommand::enqueue(pair_fix_command(
        balloon_hwnd,
        InsertSpec::After(intruder_hwnd),
    ));
    flush_window_pos_commands();
    assert_ne!(
        measure_window_below(balloon_hwnd),
        Some(character_hwnd),
        "対照: 割り込み窓の背後へ入れればキャラの隣にはならない"
    );

    for hwnd in [foreign_top, parent] {
        destroy_test_window(hwnd);
    }
}

/// 帯の所属の実測は 2 通りの答えを実際に返し、読めなかったときは帯の中へ倒す。
///
/// この 1 ビットが挿入位置を分ける（[`InsertSpec::After`] か
/// [`InsertSpec::TopOfNormalBand`] か）ので、読み取りが常に同じ答えを返していないことを
/// 両側で示す必要がある。読めなかった場合に**帯の中へ倒す**のは、`false` へ倒すと
/// 「帯の中の窓を挿入位置に指す」経路が復活し、要件 8.1 を毀損する側へ倒れるからである。
/// 倒したこと自体は黙って落とさず記録に残す。
#[test]
fn measuring_band_membership_answers_both_ways_and_falls_back_into_the_band_on_failure() {
    let inside = create_always_on_top_toplevel(w!("wintf-zorder-band-read-inside"));
    let outside = create_ghost_shaped_toplevel(w!("wintf-zorder-band-read-outside"));
    assert!(
        measure_window_is_always_on_top(inside),
        "帯の中の窓は帯の中と読めるはず"
    );
    assert!(
        !measure_window_is_always_on_top(outside),
        "帯の外の窓は帯の外と読めるはず（読み取りが恒真になっていない）"
    );

    // 読めない窓（無効なハンドル）は帯の中として扱い、倒したことを記録に残す。
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        assert!(
            measure_window_is_always_on_top(HWND::default()),
            "読めなかったときは帯の中へ倒す（要件 8.1 を毀損する側へ倒さない）"
        );
    });
    assert!(
        out.contains("常時最前面かを読めませんでした"),
        "読めなかった事実が記録に残っていない: {out}"
    );

    for hwnd in [inside, outside] {
        destroy_test_window(hwnd);
    }
}

/// キャラ窓**自身**の帯の実測も 2 通りの答えを実際に返し、読めなかったときは帯の外へ倒す。
///
/// 挿入位置の窓の実測（直上のテスト）と倒す向きが逆である——こちらの真は「帯の窓を
/// 挿入位置に指してよい」を開くので、読めないときに真へ倒すと帯の外のキャラのバルーンを
/// 帯へ引き込む（要件 3.3）。倒したことは 1 度だけ記録に残す。
#[test]
fn measuring_the_character_band_answers_both_ways_and_falls_back_out_of_the_band_on_failure() {
    let inside = create_always_on_top_toplevel(w!("wintf-zorder-below-band-inside"));
    let outside = create_ghost_shaped_toplevel(w!("wintf-zorder-below-band-outside"));
    assert!(
        measure_below_is_always_on_top(inside),
        "帯の中のキャラ窓は帯の中と読めるはず"
    );
    assert!(
        !measure_below_is_always_on_top(outside),
        "帯の外のキャラ窓は帯の外と読めるはず（読み取りが恒真になっていない）"
    );

    // 読めない窓（無効なハンドル）は帯の外として扱い、倒したことを 1 度だけ記録に残す。
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        assert!(
            !measure_below_is_always_on_top(HWND::default()),
            "読めなかったときは帯の外へ倒す（帯の外のキャラのバルーンを帯へ引き込まない）"
        );
    });
    assert_line_count(
        &out,
        "キャラ窓が常時最前面かを読めませんでした",
        1,
        "読めなかった事実の記録",
    );

    for hwnd in [inside, outside] {
        destroy_test_window(hwnd);
    }
}

/// 実窓のペアで、帯に入らないことと隣接が保たれることを**同じ 1 本で**測る。
///
/// # なぜこれがトップレベル窓でも決定論になるのか
///
/// 本ファイル冒頭のとおり、`HWND_TOP` がデスクトップのどこまで持ち上がるかは決定論では
/// ない。それでも本テストが安定するのは、**主張が持ち上がり幅に依存しないから**である
/// ——バルーン窓はキャラ窓を owner に持つ被 owner 窓であり（案 A の確立・`OwnerLink`）、
/// Windows は被 owner 窓を owner のすぐ手前に保ち、**owner ごと一組で動かす**。
/// よって持ち上がりが通常帯の最上まで届いても途中で止まっても、バルーンはキャラの
/// すぐ手前に居続ける。
///
/// 実測（本テストと同じ手順を 3 巡）: 被 owner 窓は idx 29 → 4、owner も 30 → 5 と
/// **一緒に動き**、`WS_EX_TOPMOST` は付かず、隣接は前後とも成立した。加えて owner と
/// 被 owner の**間に割り込み窓を差し込むこと自体ができない**——owner のすぐ背後を指定して
/// 挿入しても、OS は割り込み窓を owner 一組の**背後**（idx 31）へ落とす。
///
/// 末尾の対照が、この隣接が `HWND_TOP` の副産物ではなく owner 関係の帰結であることを示す
/// ——owner を張らない 2 窓に同じ指令を出すと、手前へ出るのはバルーンだけでキャラは
/// 置き去りになり、隣接は成立しない。
///
/// # 本テストが discriminate するもの・しないもの（較正の記録）
///
/// owner を張る 1 行を外すと隣接の主張が落ちる（＝owner 関係を実際に測っている）。一方で
/// **指令の写像先を「重なりを動かさない」へ変えても本テストは緑のまま**である——被 owner の
/// ペアでは隣接が指令ではなく owner 関係で保たれているので、これは実装欠陥ではなく事実の
/// とおりである。よって本テストが受け持つのは「新しい挿入位置が 2 つの不変条件（帯に
/// 入らない・隣接が壊れない）を破らないこと」であり、**指令そのものの正しさ**（`HWND_TOP`
/// であって `HWND_NOTOPMOST` ではないこと）は兄弟の
/// [`the_top_of_normal_band_fix_still_lands_the_balloon_directly_above_the_character`] が
/// 子窓で受け持つ（あちらは写像先を変えると赤くなる）。
#[test]
#[ignore = "実窓の重なり順は他プロセスの可視窓に割り込まれて間欠的に赤くなる（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）"]
fn the_top_of_normal_band_fix_keeps_a_real_owned_pair_adjacent_and_out_of_the_band() {
    require_explicit_real_window_run();

    let character = create_ghost_shaped_toplevel(w!("wintf-zorder-owned-lift-character"));
    let balloon = create_ghost_shaped_toplevel(w!("wintf-zorder-owned-lift-balloon"));
    for hwnd in [character, balloon] {
        show_without_activating(hwnd);
    }
    // 案 A の確立系（`establish_owner_links`）が張るのと同じ owner 関係。
    set_window_owner(balloon, character).expect("owner を張れるはず");
    sink_to_bottom(character);
    sink_to_bottom(balloon);
    assert!(
        !is_always_on_top(character) && !is_always_on_top(balloon),
        "前提: ペアの 2 窓は帯の外で始まる（要件 8.1）"
    );

    SetWindowPosCommand::enqueue(pair_fix_command(balloon, InsertSpec::TopOfNormalBand));
    flush_window_pos_commands();

    assert!(
        !is_always_on_top(balloon),
        "通常帯の最上へ置く指令でバルーンに常時最前面の印が付いた（要件 8.1）"
    );
    assert!(
        !is_always_on_top(character),
        "同じ指令でキャラが帯へ入った（要件 8.1）"
    );
    assert_eq!(
        measure_window_below(balloon),
        Some(character),
        "持ち上がり幅によらず、バルーンはキャラのすぐ手前に居るはず（要件 1.2）"
    );

    // 対照: owner を張っていない 2 窓では同じ指令で隣接しない
    // （上の隣接が指令の副産物ではなく owner 関係の帰結であることの証拠）。
    let lone_character = create_ghost_shaped_toplevel(w!("wintf-zorder-lone-lift-character"));
    let lone_balloon = create_ghost_shaped_toplevel(w!("wintf-zorder-lone-lift-balloon"));
    for hwnd in [lone_character, lone_balloon] {
        show_without_activating(hwnd);
    }
    sink_to_bottom(lone_character);
    sink_to_bottom(lone_balloon);
    SetWindowPosCommand::enqueue(pair_fix_command(lone_balloon, InsertSpec::TopOfNormalBand));
    flush_window_pos_commands();
    assert_ne!(
        measure_window_below(lone_balloon),
        Some(lone_character),
        "対照: owner が無ければキャラは置き去りになり隣接しない"
    );

    let _ = clear_window_owner(balloon);
    for hwnd in [character, balloon, lone_character, lone_balloon] {
        destroy_test_window(hwnd);
    }
}

// -------------------------------------------------------------------------
// stay-on-top-directives 要件 2.4: 帯の中の組の是正
// -------------------------------------------------------------------------

/// キャラクターとバルーンが帯に居て間に他の帯の窓が居るとき、是正はバルーンを帯の中で
/// キャラクターのすぐ手前に置き、次の巡の検証が通る。
///
/// キャラクターも帯に居るので、手前の帯の窓（割り込み）のすぐ背後を挿入位置にしてよい。
/// キャラクターが帯の外のときの避け方（通常帯の最上＝`HWND_TOP`）のままだと、バルーンは
/// 帯の最上へ出て割り込みの手前に留まり、隣り合わない——この 1 件はその違いを見分ける。
#[test]
#[ignore = "実窓の重なり順は他プロセスの可視窓に割り込まれて間欠的に赤くなる（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）"]
fn a_pair_inside_the_band_is_fixed_directly_above_the_character_within_the_band() {
    require_explicit_real_window_run();

    let character_hwnd = create_always_on_top_toplevel(w!("wintf-zorder-inband-character"));
    let intruder_hwnd = create_always_on_top_toplevel(w!("wintf-zorder-inband-intruder"));
    let balloon_hwnd = create_always_on_top_toplevel(w!("wintf-zorder-inband-balloon"));
    for hwnd in [character_hwnd, intruder_hwnd, balloon_hwnd] {
        show_without_activating(hwnd);
    }
    // 手前 → 背後: バルーン／割り込み／キャラ（どれも帯の中）
    insert_after(intruder_hwnd, balloon_hwnd);
    insert_after(character_hwnd, intruder_hwnd);
    assert_eq!(
        measure_window_below(balloon_hwnd),
        Some(intruder_hwnd),
        "前提: バルーンの 1 つ背後は帯の割り込み窓"
    );
    assert_eq!(
        measure_window_above(character_hwnd),
        Some(intruder_hwnd),
        "前提: キャラの 1 つ手前は帯の割り込み窓"
    );

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
    world.entity_mut(balloon).insert(ReassertZOrder::default());

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        flush_window_pos_commands();
        schedule.run(&mut world);
    });

    assert_eq!(
        measure_window_below(balloon_hwnd),
        Some(character_hwnd),
        "バルーンは帯の中でキャラのすぐ手前に居るはず（要件 2.4）"
    );
    assert_eq!(
        measure_window_above(character_hwnd),
        Some(balloon_hwnd),
        "逆向きの実測でも隣接しているはず"
    );
    for hwnd in [character_hwnd, intruder_hwnd, balloon_hwnd] {
        assert!(is_always_on_top(hwnd), "是正で帯から落ちた窓がある");
    }
    assert_eq!(
        lines_with(&out, FIX_TAG).len(),
        1,
        "是正の記録は 1 本: {out}"
    );
    assert_eq!(
        lines_with(&out, VERIFY_FAILED_TAG).len(),
        0,
        "検証は通るはず: {out}"
    );

    for hwnd in [character_hwnd, intruder_hwnd, balloon_hwnd] {
        destroy_test_window(hwnd);
    }
}
