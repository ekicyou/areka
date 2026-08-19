//! 沈降観測（[`observe_marked_pair_sinks`](super::observe_marked_pair_sinks) と
//! [`mark_pair_sink_observation`](super::mark_pair_sink_observation)）に対する
//! ヘッドレステスト（実機・実ディスプレイ不要）。
//!
//! 本ファイルが固定するのは 4 つである。
//!
//! - **非活性化の巡では測らない**——`WM_ACTIVATE(WA_INACTIVE)` の受理で付くのは目印だけで、
//!   実測記録はその巡には 1 本も出ない（遅延 1 巡・要件 4.4／7.5）。
//! - **次の巡でちょうど 1 本出る**——目印は一回限りで、次の巡以降は無音に戻る。
//! - **記録の中身**——ペアの隣接が保たれているかと、前面窓より背面に居るかが載る。
//! - **窓の挙動を変えない**——観測は読み取りだけで、指令を 1 本も出さず重なりも動かない。
//!
//! # 「前面窓より背面か」をどう決定論的に測るか（採った道とその理由）
//!
//! `GetForegroundWindow` が何を返すかはテスト環境では決まらない（他プロセスの窓が前面に
//! 居るのが普通である）。そこで**判定そのものを純関数**
//! [`is_behind_foreground`](super::is_behind_foreground) へ切り出し、前面窓の有無・自分自身が
//! 前面・手前の列に居る・列の外に居る・走査が不完全、の 5 通りを実ハンドル無しで網羅する。
//! 実窓を通す側のテストは「記録に欄が出ること」と「テスト窓は決して前面になり得ないので
//! `behind_foreground=true` にはならないこと」だけを主張する——値そのものの正しさは
//! 純関数側で固定されている。
//!
//! # なぜ親窓の下の子窓で測るのか
//!
//! トップレベル窓の重なり隣接は同一テストバイナリ内で決定論にならない（兄弟列がデスクトップ
//! 全体で共有される）。`api.rs`・維持系のテストと同じく、テスト専用の親窓を 1 枚立てて
//! その子窓で測る——兄弟列がその数枚に閉じるため、隣接も「隣が無いこと」も確定する。
//!
//! シングルスレッド実行器を明示する理由も同じである（既定の多スレッド実行器では
//! [`capture_under_filter`] が 1 行も拾えず、記録の検査が空虚に緑になる）。

use std::cell::RefCell;
use std::rc::Rc;

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{Schedule, SingleThreadedExecutor};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ACTIVATE, WS_CHILD,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use super::{SinkObservationPending, is_behind_foreground, mark_pair_sink_observation};
use crate::api::get_window_below;
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::{FrontScan, measure_windows_in_front};
use crate::ecs::window::{
    KeepDirectlyAbove, SetWindowPosCommand, WindowHandle, ZOrderPairStrategy,
    apply_zorder_pair_maintenance,
};
use crate::ecs::world::EcsWorld;
use crate::executor::util::WindowMessage;

/// 実機サインオフの `RUST_LOG`（design.md「Real-Machine Signoff」の wintf 側）に、
/// 指令の実施記録を出す `command` モジュールを足したもの（維持系のテストと同じ）。
const CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";

const SINK_TAG: &str = "[zorder-pair] sink-observed";
const SKIP_TAG: &str = "[zorder-pair] skip";
/// `guarded_set_window_pos` の実施記録（＝実際に発行された指令 1 本）。
const SET_WINDOW_POS_TAG: &str = "[guarded_set_window_pos] Calling SetWindowPos";
/// ペア宣言の無い窓に付いていた目印を落とした記録。
const ORPHAN_MARK: &str = "沈降観測の目印がペア宣言の無い窓";

/// `WM_ACTIVATE` の非活性化（`WA_INACTIVE` = 0）。
const WA_INACTIVE: usize = 0;
/// `WM_ACTIVATE` の活性化（`WA_ACTIVE` = 1）。
const WA_ACTIVE: usize = 1;

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

/// テスト専用の親窓（兄弟列を閉じるための入れ物）。
///
/// **可視で作る**——実測層は可視の窓だけを隣と数えるため（`zorder_pair` の実測層 doc）、
/// 隠れた親の下の子窓では隣接も手前側の列も測れない（`IsWindowVisible` は当該窓と祖先
/// すべての `WS_VISIBLE` を見る）。0x0 の道具窓（`WS_EX_TOOLWINDOW`）を活性化を伴わない
/// `SW_SHOWNA` で見せるため、画面にもタスクバーにも現れず、前面窓も奪わない。
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

/// 親窓の子として**可視の**実窓を生成する（本番のゴースト窓と同じ可視の窓を模す）。
///
/// 生成直後の兄弟間の重なりは前提にしない——初期配置は必ず [`insert_after`] で明示的に作る
/// （子窓は先に作ったものほど手前であり、生成順から前後関係を読むと逆になる）。
fn create_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        Some(parent),
    )
}

/// 親窓の子として**不可視の**実窓を生成する（OS が owner の直上に置く補助窓に当たる役）。
fn create_hidden_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0),
        Some(parent),
    )
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

/// Win32 へは渡さない偽ハンドル（値としてのみ扱う）。
fn fake_hwnd(v: usize) -> HWND {
    HWND(v as *mut _)
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

/// 維持系だけを載せた 1 巡分の schedule（沈降観測は維持系の中で回る）。
fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    schedule.add_systems(apply_zorder_pair_maintenance);
    schedule
}

/// 重なりも位置も動かさない対照の指令を 1 本だけ発行する。
///
/// 「観測は指令を出さない」ことを主張する捕捉窓に必ず併置する——捕捉が死んでいれば
/// この 1 本も現れず、テストは赤になる。
fn issue_control_command(hwnd: HWND) {
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        None,
    ));
    SetWindowPosCommand::flush();
}

fn lines_with<'a>(out: &'a str, tag: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(tag)).collect()
}

/// 指定タグの行がちょうど `expected` 本であることを確かめる。
///
/// 「0 本でないこと」だけを見ると余計に出る側の欠陥（同じ観測が 2 度走る・目印が消えない等）を
/// 見逃すため、常に本数で固定する。
fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

/// バルーンがキャラのすぐ手前に居るペアを実窓で 1 組つくる。
struct Pair {
    parent: HWND,
    character: HWND,
    balloon: HWND,
}

impl Pair {
    fn create(tag: PCWSTR) -> Self {
        let parent = create_test_parent(tag);
        let character = create_test_child(parent, w!("character"));
        let balloon = create_test_child(parent, w!("balloon"));
        // 「バルーン／キャラ」の順（手前 → 背後）を明示的に作る。
        insert_after(character, balloon);
        let pair = Self {
            parent,
            character,
            balloon,
        };
        assert_eq!(
            get_window_below(pair.balloon).expect("GetWindow は成功するはず"),
            Some(pair.character),
            "前提: バルーンの 1 つ背後はキャラ窓（＝隣接している）"
        );
        pair
    }

    fn destroy(self) {
        destroy_test_window(self.parent);
    }
}

/// `WM_ACTIVATE` を 1 通配送する（本番と同じ配送表を通す）。
fn dispatch_activate(world: &Rc<RefCell<EcsWorld>>, entity: Entity, activation_state: usize) {
    let message = WindowMessage {
        hwnd: HWND(std::ptr::null_mut()),
        msg: WM_ACTIVATE,
        wparam: WPARAM(activation_state),
        lparam: LPARAM(0),
    };
    crate::ecs::dispatch_window_message(world, entity, &message);
}

// -------------------------------------------------------------------------
// 遅延 1 巡——非活性化の巡には測らず、次の巡に 1 本だけ出す
// -------------------------------------------------------------------------

/// 非活性化を受理した巡には実測記録が 1 本も出ず、次の巡にちょうど 1 本出る。
///
/// これが本タスクの中核である。`WM_ACTIVATE(WA_INACTIVE)` は活性化トランザクションの
/// **途中**に届き、新しい前面窓の浮上が終わっていないことがある——その瞬間に測ると実装に
/// 欠陥が無くても偽の失敗を記録してしまうため、目印だけを置いて実測は次の巡へ回す。
///
/// 「非活性化の巡には出ない」の主張が捕捉の死で恒真にならないよう、**同じ捕捉窓に別ペアの
/// 観測を 1 本併置**する（そちらは目印が先に付いており、この巡で必ず記録が出る）。
/// 記録が 1 本だけであることと、その 1 本が**対照ペアの entity を指している**ことの
/// 両方を検査する。
#[test]
fn deactivation_marks_only_and_the_measurement_lands_on_the_next_pass() {
    let observed = Pair::create(w!("wintf-zorder-sink-delayed"));
    let control = Pair::create(w!("wintf-zorder-sink-delayed-control"));

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (character, balloon, control_balloon) = {
        let mut borrow = world.borrow_mut();
        let w = borrow.world_mut();
        w.insert_resource(ZOrderPairStrategy::default());
        let character = spawn_window(w, observed.character);
        let balloon = spawn_window(w, observed.balloon);
        declare_pair(w, balloon, character);

        let control_character = spawn_window(w, control.character);
        let control_balloon = spawn_window(w, control.balloon);
        declare_pair(w, control_balloon, control_character);
        // 対照は先に目印を持たせておく（この巡で必ず記録が出る側）。
        w.entity_mut(control_balloon).insert(SinkObservationPending);
        (character, balloon, control_balloon)
    };

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 対照ペアの観測（＝捕捉が生きている証拠。ちょうど 1 本出る）。
        schedule.run(world.borrow_mut().world_mut());
        // 観測対象の窓が非活性化される。ここでは目印が付くだけで、実測はしない。
        dispatch_activate(&world, character, WA_INACTIVE);
    });

    assert_line_count(
        &out,
        SINK_TAG,
        1,
        "非活性化の巡には測らない（出るのは対照の 1 本だけ）",
    );
    assert!(
        lines_with(&out, SINK_TAG)[0].contains(&format!("entity={control_balloon:?}")),
        "この巡の記録は対照ペアを指すはず: {}",
        lines_with(&out, SINK_TAG)[0]
    );
    assert!(
        world
            .borrow()
            .world()
            .entity(balloon)
            .contains::<SinkObservationPending>(),
        "非活性化の受理で、宣言を持つ側（バルーン窓）へ目印が付くはず"
    );

    // 次の巡。ここで初めて実測と記録が起きる。
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(world.borrow_mut().world_mut());
    });

    assert_line_count(&out, SINK_TAG, 1, "遅らせた実測は次の巡にちょうど 1 本");
    let line = lines_with(&out, SINK_TAG)[0];
    assert!(
        line.contains(&format!("entity={balloon:?}")),
        "記録は非活性化されたペアの宣言を持つ側を指すはず: {line}"
    );
    assert!(
        line.contains("adjacency_ok=true"),
        "沈んでもペアの隣接は保たれているはず: {line}"
    );
    assert!(
        !world
            .borrow()
            .world()
            .entity(balloon)
            .contains::<SinkObservationPending>(),
        "目印は一回限りで、測った巡に消えるはず"
    );

    control.destroy();
    observed.destroy();
}

/// 活性化（`WA_ACTIVE`）では目印が付かない——沈降の観測点は「前面から外れる瞬間」だけである。
#[test]
fn activation_does_not_mark_a_sink_observation() {
    let pair = Pair::create(w!("wintf-zorder-sink-activate"));

    let world = Rc::new(RefCell::new(EcsWorld::new()));
    let (character, balloon) = {
        let mut borrow = world.borrow_mut();
        let w = borrow.world_mut();
        let character = spawn_window(w, pair.character);
        let balloon = spawn_window(w, pair.balloon);
        declare_pair(w, balloon, character);
        (character, balloon)
    };

    dispatch_activate(&world, character, WA_ACTIVE);
    assert!(
        !world
            .borrow()
            .world()
            .entity(balloon)
            .contains::<SinkObservationPending>(),
        "活性化では目印を付けない"
    );

    // 対照: 同じ窓の非活性化なら確かに目印が付く（付かない側の主張が恒真にならない）。
    dispatch_activate(&world, character, WA_INACTIVE);
    assert!(
        world
            .borrow()
            .world()
            .entity(balloon)
            .contains::<SinkObservationPending>(),
        "非活性化なら目印が付くはず"
    );

    pair.destroy();
}

/// 目印は一回限り——観測の済んだ次の巡は無音に戻る（毎巡の観測にしない）。
#[test]
fn the_sink_observation_happens_only_once_per_mark() {
    let pair = Pair::create(w!("wintf-zorder-sink-once"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, pair.character);
    let balloon = spawn_window(&mut world, pair.balloon);
    declare_pair(&mut world, balloon, character);
    world.entity_mut(balloon).insert(SinkObservationPending);

    let mut schedule = maintain_schedule();
    let first = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
    });
    assert_line_count(&first, SINK_TAG, 1, "目印の次の巡に 1 本");

    let second = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        // 対照: 捕捉が生きていれば必ず現れる 1 本。
        issue_control_command(pair.balloon);
    });
    assert_line_count(&second, SINK_TAG, 0, "目印が消えた後は無音");
    assert_line_count(
        &second,
        SET_WINDOW_POS_TAG,
        1,
        "対照の 1 本だけ（捕捉は生きている）",
    );

    pair.destroy();
}

// -------------------------------------------------------------------------
// 読み取り専用——指令を出さず、重なりも動かさない
// -------------------------------------------------------------------------

/// 沈降観測は窓の挙動を一切変えない（指令を 1 本も出さず、重なりも動かない）。
#[test]
fn the_sink_observation_issues_no_command_and_moves_nothing() {
    let parent = create_test_parent(w!("wintf-zorder-sink-readonly"));
    let character_hwnd = create_test_child(parent, w!("character"));
    let intruder_hwnd = create_test_child(parent, w!("intruder"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    // 「バルーン／割り込み／キャラ」の順（手前 → 背後）＝**隣接が壊れている**状態。
    // 観測がうっかり是正してしまえば、この重なりが動いて赤になる。
    insert_after(intruder_hwnd, balloon_hwnd);
    insert_after(character_hwnd, intruder_hwnd);

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    world.entity_mut(balloon).insert(SinkObservationPending);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 対照: 捕捉が生きていれば必ず現れる 1 本。
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(&out, SINK_TAG, 1, "観測の記録は 1 本");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "観測は指令を出さない（出るのは対照の 1 本だけ）",
    );
    let line = lines_with(&out, SINK_TAG)[0];
    assert!(
        line.contains("adjacency_ok=false"),
        "隣接が壊れていることが記録に出るはず: {line}"
    );
    assert_eq!(
        get_window_below(balloon_hwnd).expect("GetWindow は成功するはず"),
        Some(intruder_hwnd),
        "観測は重なりを動かさない"
    );

    destroy_test_window(parent);
}

/// 記録には前面窓との相対の欄が必ず出る。
///
/// テストの窓は非表示の子窓であり**決して前面窓になり得ない**（前面窓は必ずこの兄弟列の
/// 外に居る）。ゆえに `behind_foreground=true` は構造的に起こらない——値そのものの
/// 網羅は純関数 [`is_behind_foreground`] のテストが担う。
#[test]
fn the_sink_record_carries_the_foreground_relation() {
    let pair = Pair::create(w!("wintf-zorder-sink-foreground"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, pair.character);
    let balloon = spawn_window(&mut world, pair.balloon);
    declare_pair(&mut world, balloon, character);
    world.entity_mut(balloon).insert(SinkObservationPending);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
    });

    let line = lines_with(&out, SINK_TAG)[0];
    assert!(line.contains("foreground="), "前面窓の欄が無い: {line}");
    assert!(
        line.contains("behind_foreground="),
        "前面窓との相対の欄が無い: {line}"
    );
    assert!(
        !line.contains("behind_foreground=true"),
        "兄弟列の外に居る前面窓に対して「背面に居る」とは判定されないはず: {line}"
    );

    pair.destroy();
}

// -------------------------------------------------------------------------
// 目印を付ける先の決定（どちらの窓が非活性化されても宣言を持つ側に付く）
// -------------------------------------------------------------------------

/// 目印はキャラ窓側・バルーン窓側のどちらが非活性化されても、宣言を持つ側へ付く。
///
/// 記録の `entity` を宣言を持つ側（バルーン窓）に揃えるためである——他の記録もすべて
/// 宣言を持つ側を指しており、`declared` レコードとの entity 結合が 1 通りに定まる。
#[test]
fn the_mark_always_lands_on_the_declaration_holder() {
    let mut world = World::new();
    let character = spawn_window(&mut world, fake_hwnd(0x0E01));
    let balloon = spawn_window(&mut world, fake_hwnd(0x0E02));
    declare_pair(&mut world, balloon, character);

    // キャラ窓側（宣言を持たない＝peer として参照されている側）が非活性化された場合。
    assert_eq!(
        mark_pair_sink_observation(&mut world, character),
        Some(balloon),
        "peer 側の非活性化でも、目印は宣言を持つ側へ付くはず"
    );
    assert!(world.entity(balloon).contains::<SinkObservationPending>());
    assert!(
        !world.entity(character).contains::<SinkObservationPending>(),
        "宣言を持たない側には付けない"
    );

    world.entity_mut(balloon).remove::<SinkObservationPending>();

    // バルーン窓側（宣言を持つ側）が非活性化された場合。
    assert_eq!(
        mark_pair_sink_observation(&mut world, balloon),
        Some(balloon)
    );
    assert!(world.entity(balloon).contains::<SinkObservationPending>());

    // ペアの当事者でない窓が非活性化されても、何も付かない。
    let stray = spawn_window(&mut world, fake_hwnd(0x0E03));
    assert_eq!(
        mark_pair_sink_observation(&mut world, stray),
        None,
        "ペアの当事者でなければ観測すべきものが無い"
    );
    assert!(!world.entity(stray).contains::<SinkObservationPending>());
}

// -------------------------------------------------------------------------
// 測れない巡（理由つきの見送り・要件 6.3）
// -------------------------------------------------------------------------

/// 相手が先に消えていれば、実測せず理由つきで見送る（要件 1.5 の正常系）。
#[test]
fn a_mark_whose_peer_is_gone_skips_with_a_reason() {
    let pair = Pair::create(w!("wintf-zorder-sink-peer-gone"));
    let control = Pair::create(w!("wintf-zorder-sink-peer-gone-control"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, pair.character);
    let balloon = spawn_window(&mut world, pair.balloon);
    declare_pair(&mut world, balloon, character);
    world.entity_mut(balloon).insert(SinkObservationPending);
    world.entity_mut(character).despawn();

    // 対照（確かに記録が出る側）
    let control_character = spawn_window(&mut world, control.character);
    let control_balloon = spawn_window(&mut world, control.balloon);
    declare_pair(&mut world, control_balloon, control_character);
    world
        .entity_mut(control_balloon)
        .insert(SinkObservationPending);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains("reason=PeerMissing"),
        "見送りの理由は「相手が居ない」であるはず: {skip}"
    );
    assert!(
        skip.contains(&format!("entity={balloon:?}")),
        "見送りの記録は相手を失ったペアを指すはず: {skip}"
    );
    assert_line_count(&out, SINK_TAG, 1, "測れたのは対照のペアだけ");
    assert!(
        lines_with(&out, SINK_TAG)[0].contains(&format!("entity={control_balloon:?}")),
        "実測記録は対照のペアを指すはず"
    );
    assert!(
        !world.entity(balloon).contains::<SinkObservationPending>(),
        "測れなくても目印はこの巡で消える（毎巡の再評価を残さない）"
    );

    control.destroy();
    pair.destroy();
}

/// まだ OS のハンドルが付いていなければ、実測せず理由つきで見送る。
#[test]
fn a_mark_without_handles_skips_with_a_reason() {
    let control = Pair::create(w!("wintf-zorder-sink-handleless-control"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = world.spawn_empty().id();
    let balloon = spawn_window(&mut world, fake_hwnd(0x0F01));
    declare_pair(&mut world, balloon, character);
    world.entity_mut(balloon).insert(SinkObservationPending);

    // 対照（確かに記録が出る側）
    let control_character = spawn_window(&mut world, control.character);
    let control_balloon = spawn_window(&mut world, control.balloon);
    declare_pair(&mut world, control_balloon, control_character);
    world
        .entity_mut(control_balloon)
        .insert(SinkObservationPending);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(skip.contains("reason=HandleMissing"), "{skip}");
    assert!(skip.contains(&format!("entity={balloon:?}")), "{skip}");
    assert_line_count(&out, SINK_TAG, 1, "測れたのは対照のペアだけ");
    assert!(lines_with(&out, SINK_TAG)[0].contains(&format!("entity={control_balloon:?}")));

    control.destroy();
}

/// ペア宣言を失った窓に残っていた目印は、記録を残して落とす（黙って読み飛ばさない）。
#[test]
fn a_mark_on_a_window_without_declaration_is_dropped_with_a_record() {
    let control = Pair::create(w!("wintf-zorder-sink-orphan-control"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let stray = spawn_window(&mut world, fake_hwnd(0x0F11));
    world.entity_mut(stray).insert(SinkObservationPending);

    // 対照（確かに記録が出る側）
    let control_character = spawn_window(&mut world, control.character);
    let control_balloon = spawn_window(&mut world, control.balloon);
    declare_pair(&mut world, control_balloon, control_character);
    world
        .entity_mut(control_balloon)
        .insert(SinkObservationPending);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
    });

    assert_line_count(&out, ORPHAN_MARK, 1, "宣言の無い目印は 1 度だけ記録される");
    assert!(
        lines_with(&out, ORPHAN_MARK)[0].contains(&format!("entity={stray:?}")),
        "記録は宣言の無い窓を指すはず"
    );
    assert_line_count(&out, SINK_TAG, 1, "測れたのは対照のペアだけ");
    assert!(
        !world.entity(stray).contains::<SinkObservationPending>(),
        "落とした目印は残さない"
    );

    control.destroy();
}

// -------------------------------------------------------------------------
// 「前面窓より背面か」の判定（純関数・実ハンドル不要）
// -------------------------------------------------------------------------

fn scan(windows: &[HWND], reached_top: bool) -> FrontScan {
    FrontScan {
        windows: windows.to_vec(),
        reached_top,
    }
}

/// 前面窓が手前の列に居れば「背面に居る」、列を最前面まで辿って居なければ「背面ではない」。
#[test]
fn is_behind_foreground_reads_the_front_chain() {
    let target = fake_hwnd(0x0101);
    let front = fake_hwnd(0x0102);
    let other = fake_hwnd(0x0103);

    assert_eq!(
        is_behind_foreground(target, Some(front), &scan(&[other, front], true)),
        Some(true),
        "前面窓が自分より手前の列に居れば背面に居る"
    );
    assert_eq!(
        is_behind_foreground(target, Some(front), &scan(&[other], true)),
        Some(false),
        "最前面まで辿って前面窓が居なければ、自分は前面窓より背面ではない"
    );
    assert_eq!(
        is_behind_foreground(target, Some(target), &scan(&[], true)),
        Some(false),
        "自分自身が前面窓なら背面ではない"
    );
}

/// 判定そのものが取れなかった場合は番兵（偽と混同しない）。
#[test]
fn is_behind_foreground_reports_absence_rather_than_a_false_negative() {
    let target = fake_hwnd(0x0201);
    let front = fake_hwnd(0x0202);

    assert_eq!(
        is_behind_foreground(target, None, &scan(&[], true)),
        None,
        "前面窓が取れなければ判定できない"
    );
    assert_eq!(
        is_behind_foreground(target, Some(front), &scan(&[], false)),
        None,
        "列を最前面まで辿れていなければ「背面ではない」と断じない"
    );
    // 対照: 走査が不完全でも、前面窓が既に見つかっていれば判定できる。
    assert_eq!(
        is_behind_foreground(target, Some(front), &scan(&[front], false)),
        Some(true),
        "見つかった時点で判定は確定する"
    );
}

// -------------------------------------------------------------------------
// 手前側の走査（実窓）
// -------------------------------------------------------------------------

/// 手前側の走査は、対象より手前の**可視の**窓を手前へ向かう順に集め、最前面まで辿った
/// ことを示す。不可視の窓は列に入らない（前面窓は必ず可視ゆえ判定は変わらない）。
#[test]
fn measure_windows_in_front_collects_the_visible_chain_up_to_the_top() {
    let parent = create_test_parent(w!("wintf-zorder-sink-scan"));
    let back = create_test_child(parent, w!("back"));
    let middle = create_test_child(parent, w!("middle"));
    let hidden = create_hidden_test_child(parent, w!("hidden"));
    let front = create_test_child(parent, w!("front"));
    // 「front／hidden／middle／back」の順（手前 → 背後）を明示的に作る。
    insert_after(hidden, front);
    insert_after(middle, hidden);
    insert_after(back, middle);

    let scanned = measure_windows_in_front(back);
    assert_eq!(
        scanned.windows,
        vec![middle, front],
        "手前へ向かう順に集まり、不可視の窓は入らないはず"
    );
    assert!(
        !scanned.windows.contains(&hidden),
        "不可視の窓は手前の列に数えない"
    );
    assert!(scanned.reached_top, "最前面まで辿り着いたはず");

    // 対照: 最前面の窓から辿れば、手前には誰も居ない。
    let from_front = measure_windows_in_front(front);
    assert!(from_front.windows.is_empty());
    assert!(from_front.reached_top);

    destroy_test_window(parent);
}

/// 走査に失敗した場合は「最前面まで辿った」と偽らず、失敗そのものを記録に残す。
#[test]
fn measure_windows_in_front_reports_failure_as_an_incomplete_chain() {
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        let scanned = measure_windows_in_front(HWND::default());
        assert!(scanned.windows.is_empty());
        assert!(
            !scanned.reached_top,
            "失敗を「最前面まで辿った」と偽らない（偽の判定を作らない）"
        );
    });
    assert!(
        !out.trim().is_empty(),
        "走査の失敗は記録に残るはず（黙って潰さない）"
    );
}
