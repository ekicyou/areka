//! 片割れが消えた後、**残っている窓の表示状態が一切変わらない**ことを固定する
//! ヘッドレステスト（実機・実ディスプレイ不要）。
//!
//! # 兄弟ファイルとの分担
//!
//! 要件 1.5 は 2 つのことを言っている——「表示順の調整を行わない」と「残っている窓の
//! 表示状態を変更しない」である。前者（指令が 1 本も出ないこと・理由つきで見送ること）は
//! [`zorder_pair_maintain_tests`](super::tests) が既に固定している。本ファイルが受け持つのは
//! **後者**、すなわち残った窓そのものを実測して「動いていない・変わっていない」ことを
//! 確かめる側である（design.md「Testing Strategy > Integration Tests」4 の
//! 「残存窓の `WindowPos`／スタイル不変」）。
//!
//! 指令の本数だけを数えると、本 system 以外の経路（切離しのパスなど）が窓へ書き込んだ場合を
//! 取り逃がす。よって本ファイルは**窓の側から**測る——矩形・スタイル・拡張スタイル・可視性、
//! そして測れる場合は重なりの隣まで、処理の前後で 1 つも変わらないことを突き合わせる。
//!
//! # 片割れが消える経路は 2 つあり、両方を見る
//!
//! - **要求を持ったまま相手が消えた**（維持系の見送り経路）——子窓で測るので重なりの隣まで
//!   決定論的に確かめられる。
//! - **owner を張ったペアの相手が消えた**（切離しのパス）——owner 関係はトップレベル窓の
//!   間でしか意味を持たないため、こちらはトップレベル窓で作る。トップレベル窓の**重なり**は
//!   同一テストバイナリ内で決定論にならない（他テスト・他プロセスの窓が割り込む）ので、
//!   この経路では重なり以外の facet を突き合わせる。owner が外れること自体は要件 5.9 の
//!   要求であり、これは「表示状態」ではない。
//!
//! # 「書き込まれていない」をどう測るか
//!
//! 適用は [`SetWindowPosCommand`] のキュー経由であり、flush 時に `guarded_set_window_pos` が
//! debug 水準の実施記録を必ず 1 行出す。よってその行を数えることが「指令が何本出たか」の
//! 直接の観測になる。「0 本」を主張する捕捉窓には、**確かに 1 本出る対照の指令**
//! （重なりも位置も動かさない no-op）を必ず併置し、`0 + 対照 1` = ちょうど 1 本で固定する
//! ——捕捉が死んでいれば対照も 0 本になって赤くなり、余計な指令が出れば 2 本で赤くなる。
//!
//! シングルスレッド実行器を明示する理由は他の維持系テストと同じである（既定の多スレッド
//! 実行器では [`capture_under_filter`] が 1 行も拾えず、記録の検査が空虚に緑になる）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, GetWindowRect,
    IsWindow, SET_WINDOW_POS_FLAGS, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use super::apply_zorder_pair_maintenance;
use crate::api::{get_window_above, get_window_below, is_window_visible};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::types::{Point, SizeI};
use crate::ecs::window::{
    KeepDirectlyAbove, OwnerLink, ReassertZOrder, SetWindowPosCommand, WindowHandle, WindowPos,
    ZOrderPairStrategy, establish_owner_links,
};

/// 実機サインオフの `RUST_LOG` に、指令の実施記録を出す `command` モジュールを足したもの
/// （兄弟の維持系テストと同一）。
const CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";

const FIX_TAG: &str = "[zorder-pair] fix";
const SKIP_TAG: &str = "[zorder-pair] skip";
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
/// `guarded_set_window_pos` の実施記録（＝実際に発行された指令 1 本）。
const SET_WINDOW_POS_TAG: &str = "[guarded_set_window_pos] Calling SetWindowPos";
/// 切離しが走った記録の判定語。
const DETACHED_MARK: &str = "owner を切り離しました";

// -------------------------------------------------------------------------
// 実窓・World・巡回のヘルパ
// -------------------------------------------------------------------------

fn create_window(
    title: PCWSTR,
    ex_style: WINDOW_EX_STYLE,
    style: WINDOW_STYLE,
    parent: Option<HWND>,
) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで 0x0 のテスト窓を生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            ex_style,
            w!("Static"),
            title,
            style,
            0,
            0,
            0,
            0,
            parent,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a test window")
    }
}

/// テスト専用の親窓（兄弟列を閉じるための入れ物・可視で作る理由は維持系テストと同じ）。
fn create_test_parent(title: PCWSTR) -> HWND {
    let parent = create_window(
        title,
        WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
        WINDOW_STYLE(WS_POPUP.0),
        None,
    );
    // SAFETY: Win32 境界。自プロセス所有の窓を活性化せずに表示する。
    unsafe {
        let _ = ShowWindow(parent, SW_SHOWNA);
    }
    parent
}

/// 親窓の子として可視の実窓を生成する（初期の重なりは必ず [`insert_after`] で明示的に作る）。
fn create_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        Some(parent),
    )
}

/// owner 関係を張れる非表示トップレベル窓（子窓への `GWLP_HWNDPARENT` 書込は
/// owner ではなく親の付け替えになるため `WS_POPUP` で作る・切離しテストと同一のレシピ）。
fn create_test_toplevel(title: PCWSTR) -> HWND {
    create_window(title, WINDOW_EX_STYLE(0), WINDOW_STYLE(WS_POPUP.0), None)
}

/// `hwnd` を `after` のすぐ背後（1 つ背面側）へ移す。
fn insert_after(hwnd: HWND, after: HWND) {
    // SAFETY: Win32 境界。自プロセス所有の実 HWND に対する重なりのみの変更。
    unsafe {
        SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
        .expect("SetWindowPos should reorder self-owned test windows");
    }
}

fn destroy_test_window(hwnd: HWND) {
    // SAFETY: Win32 境界。生成した所有ウィンドウを破棄する（子窓は親の破棄で連鎖する）。
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// 窓がまだ実在するか。
fn is_alive(hwnd: HWND) -> bool {
    // SAFETY: Win32 境界。読み取りのみ。
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

fn declare_pair(world: &mut World, balloon: Entity, character: Entity) {
    world
        .entity_mut(balloon)
        .insert(KeepDirectlyAbove { peer: character });
}

/// 一回限りの再断行要求を外から挿入する（要件 2.6 のシームと同じ形）。
fn request_reassert(world: &mut World, balloon: Entity) {
    world.entity_mut(balloon).insert(ReassertZOrder::default());
}

/// 維持系だけを載せた 1 巡分の schedule。
fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_pair_maintenance);
    schedule
}

/// 本番と同じ順（確立 → 維持）で並べた schedule。切離しは維持系の中で走る。
fn pair_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems((establish_owner_links, apply_zorder_pair_maintenance).chain());
    schedule
}

/// 重なりも位置も動かさない対照の指令を 1 本だけ発行する。
///
/// 「是正の指令が出ていない」ことを主張する捕捉窓に必ず併置する——捕捉が死んでいれば
/// この 1 本も現れず、テストは赤になる。
fn issue_control_command(hwnd: HWND) {
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SET_WINDOW_POS_FLAGS(SWP_NOMOVE.0 | SWP_NOSIZE.0 | SWP_NOACTIVATE.0 | SWP_NOZORDER.0),
        None,
    ));
    SetWindowPosCommand::flush();
}

fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// 指定タグの行がちょうど `expected` 本であることを確かめる。
fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

// -------------------------------------------------------------------------
// 残る窓の表示状態のスナップショット
// -------------------------------------------------------------------------

/// 「表示状態」として突き合わせる facet の一式。
///
/// 重なり（`above`／`below`）は測れる場合にだけ入れる——トップレベル窓の隣は同一テスト
/// バイナリ内で決定論にならないため、そちらの経路では `None` を入れて比較から外す
/// （外していることが読めるよう、フィールドごと消さずに `Option` で残す）。
#[derive(Debug, PartialEq)]
struct DisplayState {
    rect: (i32, i32, i32, i32),
    style: isize,
    ex_style: isize,
    visible: bool,
    /// 1 つ手前の窓（子窓の経路でのみ測る）
    above: Option<Option<HWND>>,
    /// 1 つ背後の窓（子窓の経路でのみ測る）
    below: Option<Option<HWND>>,
}

impl DisplayState {
    /// 重なりまで含めて測る（兄弟列が閉じている子窓の経路）。
    fn measure_with_zorder(hwnd: HWND) -> Self {
        let mut state = Self::measure_without_zorder(hwnd);
        state.above = Some(get_window_above(hwnd).expect("GetWindow は成功するはず"));
        state.below = Some(get_window_below(hwnd).expect("GetWindow は成功するはず"));
        state
    }

    /// 重なり以外を測る（トップレベル窓の経路）。
    fn measure_without_zorder(hwnd: HWND) -> Self {
        let mut rect = RECT::default();
        // SAFETY: Win32 境界。自プロセス所有の実 HWND に対する読み取りのみ。
        unsafe {
            GetWindowRect(hwnd, &mut rect).expect("GetWindowRect は成功するはず");
        }
        // SAFETY: Win32 境界。スタイルの読み取りのみ。
        let (style, ex_style) = unsafe {
            (
                GetWindowLongPtrW(hwnd, GWL_STYLE),
                GetWindowLongPtrW(hwnd, GWL_EXSTYLE),
            )
        };
        Self {
            rect: (rect.left, rect.top, rect.right, rect.bottom),
            style,
            ex_style,
            visible: is_window_visible(hwnd),
            above: None,
            below: None,
        }
    }
}

/// 残る窓へ載せる `WindowPos`。既定値と取り違えない値を入れて、書き換えが起きれば
/// 突き合わせで必ず落ちるようにする。
fn marked_window_pos() -> WindowPos {
    WindowPos {
        position: Some(Point { x: 137, y: 241 }),
        size: Some(SizeI {
            width: 320,
            height: 240,
        }),
        ..WindowPos::new()
    }
}

// -------------------------------------------------------------------------
// 経路 1: 要求を持ったまま相手が消えた（維持系の見送り）
// -------------------------------------------------------------------------

/// 相手が消えた巡、残るバルーン窓には**何も書き込まれない**（要件 1.5 の後半）。
///
/// 見送りの記録が出ることは兄弟ファイルが固定している。ここで見るのは窓の側であり、
/// 矩形・スタイル・拡張スタイル・可視性・重なりの隣、そして ECS の [`WindowPos`] まで
/// 処理の前後で 1 つも変わらないことを突き合わせる。
///
/// 「指令が 0 本」の主張には、同じ捕捉窓の中に確かに 1 本出る対照の指令
/// （重なりも位置も動かさない no-op）を併置してある。
#[test]
fn peer_loss_leaves_the_surviving_windows_display_state_untouched() {
    let parent = create_test_parent(w!("wintf-zorder-survivor-skip"));
    let character_hwnd = create_test_child(parent, w!("character"));
    let intruder_hwnd = create_test_child(parent, w!("intruder"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    // 「バルーン／割り込み／キャラ」の順（手前 → 背後）を明示的に作る＝隣接していない。
    insert_after(intruder_hwnd, balloon_hwnd);
    insert_after(character_hwnd, intruder_hwnd);

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    world.entity_mut(balloon).insert(marked_window_pos());
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    // 相手の実体が先に消える（窓そのものは残っている＝要件 1.5 の正常系）。
    world.entity_mut(character).despawn();

    let before = DisplayState::measure_with_zorder(balloon_hwnd);
    assert_eq!(
        before.below,
        Some(Some(intruder_hwnd)),
        "前提: バルーンの 1 つ背後は割り込み窓（＝キャラの隣に居ない）"
    );

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 対照: 捕捉が生きていれば必ず現れる 1 本（窓は動かさない）。
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    assert!(
        lines_with(&out, SKIP_TAG)[0].contains("reason=PeerMissing"),
        "見送りの理由は「相手が居ない」であるはず: {out}"
    );
    assert_line_count(&out, FIX_TAG, 0, "是正していないので是正の記録は出ない");
    assert_line_count(&out, VERIFY_FAILED_TAG, 0, "正常系なので error にしない");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "残る窓へは書き込まず、対照の 1 本だけが出るはず",
    );

    assert_eq!(
        DisplayState::measure_with_zorder(balloon_hwnd),
        before,
        "残る窓の表示状態（矩形・スタイル・可視性・重なりの隣）が変わっている（要件 1.5）"
    );
    assert_eq!(
        world.get::<WindowPos>(balloon).copied(),
        Some(marked_window_pos()),
        "残る窓の WindowPos が書き換えられている（要件 1.5・design Integration Tests 4）"
    );
    assert!(
        !world.entity(balloon).contains::<ReassertZOrder>(),
        "一回限りの要求は理由つきの見送りとともに消える"
    );

    destroy_test_window(parent);
}

// -------------------------------------------------------------------------
// 経路 2: owner を張ったペアの相手が消えた（切離しのパス）
// -------------------------------------------------------------------------

/// 切離しが走る巡も、残るバルーン窓の表示状態は変わらない（要件 1.5・5.9）。
///
/// 外れるのは owner 関係だけである——owner は「表示状態」ではなく、外すこと自体が
/// 要件 5.9 の要求（破棄の連鎖を起こさせない）である。
///
/// 重なりの隣は突き合わせない——owner 関係を張るにはトップレベル窓である必要があり、
/// トップレベル窓の隣は同一テストバイナリ内で決定論にならない（他テストや他プロセスの窓が
/// 割り込む）。実際の重なりは経路 1 と実機サインオフ（要件 7.2）が受け持つ。
#[test]
fn detaching_a_lost_peer_leaves_the_surviving_windows_display_state_untouched() {
    let character_hwnd = create_test_toplevel(w!("wintf-zorder-survivor-detach-char"));
    let balloon_hwnd = create_test_toplevel(w!("wintf-zorder-survivor-detach-balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    world.entity_mut(balloon).insert(marked_window_pos());
    declare_pair(&mut world, balloon, character);

    let mut schedule = pair_schedule();
    schedule.run(&mut world);
    assert!(
        world.entity(balloon).contains::<OwnerLink>(),
        "前提: 確立系が owner を張っているはず"
    );
    // 重なりの是正は本ファイルの担当ではない（残すと実窓を動かす指令が飛ぶ）。
    // 確立系が入れた要求を外し、**その巡で積まれた指令も捕捉窓の外で流し切る**
    // ——キューはスレッドローカルに残るため、流し忘れると次の flush で対照に紛れ込む
    // （実際に紛れ込ませて 2 本になる赤を確認済み）。
    world.entity_mut(balloon).remove::<ReassertZOrder>();
    SetWindowPosCommand::flush();

    let before = DisplayState::measure_without_zorder(balloon_hwnd);

    // 相手の実体が消える——この巡で切離しが走る。
    world.entity_mut(character).despawn();

    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 対照: 捕捉が生きていれば必ず現れる 1 本（窓は動かさない）。
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(
        &out,
        DETACHED_MARK,
        1,
        "切離しは確かに走る（この主張が無いと表示状態の不変は空虚になる）",
    );
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "切離しは窓へ書き込まず、対照の 1 本だけが出るはず",
    );

    assert_eq!(
        DisplayState::measure_without_zorder(balloon_hwnd),
        before,
        "残る窓の表示状態（矩形・スタイル・可視性）が変わっている（要件 1.5）"
    );
    assert_eq!(
        world.get::<WindowPos>(balloon).copied(),
        Some(marked_window_pos()),
        "残る窓の WindowPos が書き換えられている（要件 1.5）"
    );
    assert!(
        is_alive(balloon_hwnd),
        "残る窓は破棄に巻き込まれない（要件 5.7）"
    );
    assert!(
        !world.entity(balloon).contains::<OwnerLink>(),
        "切離しの後に張った事実の記録は残さない（要件 5.9）"
    );

    destroy_test_window(balloon_hwnd);
    destroy_test_window(character_hwnd);
}
