//! 維持系 [`apply_zorder_pair_maintenance`](super::apply_zorder_pair_maintenance) に対する
//! ヘッドレステスト（実機・実ディスプレイ不要）。
//!
//! 本ファイルが固定するのは 4 つである。
//!
//! - **要求の一生**——挿入 → 適用（指令発行）→ 検証待ち → 実測照合 → 要求の消滅、という
//!   状態遷移がひと繋がりで辿れること（design.md「Testing Strategy > Unit Tests」4・要件 7.6）。
//! - **既に隣接しているなら一切の指令が出ないこと**——理由つきの見送りだけが残り、
//!   `SetWindowPos` は 1 度も呼ばれない（収束の同値ガード・要件 6.3）。
//! - **検証不一致は error で 1 度だけ記録し、再試行の輪を作らないこと**（要件 6.2）。
//! - **片割れが消えた巡は error ではなく理由つきの見送りになること**（要件 1.5 の正常系）。
//!
//! # なぜ親窓の下の子窓 2〜3 枚で測るのか（採った道とその理由）
//!
//! 維持系は `GetWindow` の実測に基づいて判断し、`SetWindowPos` で実際に重なりを動かす。
//! ここを偽の実装へ差し替えると「重なりが本当に動いたか」は永久に検査されない。一方、
//! **トップレベル窓の重なり隣接は同一テストバイナリ内で決定論にならない**——兄弟列が
//! デスクトップ全体で共有され、他テストや他プロセスの窓が 2 枚の間へ割り込みうる
//! （`api.rs` のラッパテストで実測済みの制約）。そこで本ファイルは `api.rs` と同じ道を採り、
//! **テスト専用の親窓を 1 枚立て、その子窓**でキャラ窓・バルーン窓（必要なら割り込み窓）を
//! 作る。兄弟列がその数枚に閉じるため、隣接も「隣が無いこと」も決定論的に確定する。
//!
//! # 「指令が出ない」をどう測るか
//!
//! 適用は `SetWindowPosCommand` のキュー経由であり、flush 時に
//! `guarded_set_window_pos` が debug 水準の実施記録を必ず 1 行出す。よって
//! **その行を数えることが「指令が何本出たか」の直接の観測**になる。
//! 「0 本」を主張するテストには、同じ捕捉窓の中で**確かに 1 本出る対照の指令**
//! （重なりも位置も動かさない no-op）を併置し、`0 + 対照 1` = ちょうど 1 本で固定する
//! ——捕捉が死んでいれば対照も 0 本になって赤くなり、余計な指令が出れば 2 本で赤くなる。
//!
//! シングルスレッド実行器を明示する理由は確立系のテストと同じである（既定の多スレッド
//! 実行器では [`capture_under_filter`] が 1 行も拾えず、記録の検査が空虚に緑になる）。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ExecutorKind, Schedule};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, HWND_TOP, HWND_TOPMOST, SET_WINDOW_POS_FLAGS, SWP_NOACTIVATE,
    SW_SHOWNA, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos, ShowWindow, WINDOW_EX_STYLE,
    WINDOW_STYLE, WS_CHILD, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use super::{IssuedPairFix, apply_zorder_pair_maintenance, pair_fix_command, plan_pair_fix};
use crate::api::{get_window_above, get_window_below};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::zorder_pair::{
    ExpectedOrder, InsertSpec, PairFixDecision, PairObservation, PairTrigger, SkipReason,
    measure_window_below,
};
use crate::ecs::window::{
    KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle, ZOrderPairStrategy,
};

/// 実機サインオフの `RUST_LOG`（design.md「Real-Machine Signoff」の wintf 側）に、
/// 指令の実施記録を出す `command` モジュールを足したもの。
///
/// 足しているのは本テストが「指令が何本出たか」を数えるためであり、サインオフの
/// 判定語を増やしているわけではない。
const CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";

const FIX_TAG: &str = "[zorder-pair] fix";
const SKIP_TAG: &str = "[zorder-pair] skip";
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";
/// `guarded_set_window_pos` の実施記録（＝実際に発行された指令 1 本）。
const SET_WINDOW_POS_TAG: &str = "[guarded_set_window_pos] Calling SetWindowPos";
/// ペア宣言の無い窓に付いた要求を落とした記録。
const ORPHAN_REQUEST_MARK: &str = "ペア宣言の無い窓に再断行の要求";
/// 出した指令の持ち越しが無い検証段階の要求を落とした記録。
const UNBACKED_VERIFY_MARK: &str = "検証待ちの要求に、出した指令の持ち越しがありません";

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
/// 隠れた親の下の子窓では隣接そのものが測れない（`IsWindowVisible` は当該窓と祖先すべての
/// `WS_VISIBLE` を見るので、親が隠れていれば子はどう作っても不可視になる）。0x0 の道具窓
/// （`WS_EX_TOOLWINDOW`）を活性化を伴わない `SW_SHOWNA` で見せるため、画面にもタスクバーにも
/// 現れず、他のテストが握っている前面窓も奪わない。
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
/// 生成直後の兄弟間の重なりは前提にしない——初期配置は必ず [`insert_after`] で明示的に作る。
/// 実測すると子窓は**先に作ったものほど手前**（新しい子窓は兄弟列の背面側へ入る）であり、
/// 生成順から前後関係を読むと逆になる。`api.rs` の z オーダーのラッパテストも同じ理由で
/// 生成順に依存しない形にしてある。
fn create_test_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        Some(parent),
    )
}

/// `hwnd` を `after` のすぐ背後（1 つ背面側）へ移す（`api.rs` のテストと同じ手）。
///
/// `SetWindowPos` の `hWndInsertAfter` は「位置決め対象の 1 つ手前に来る窓」であり、
/// 対象はその**背面側**へ入る。
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

fn hwnd_hex(hwnd: HWND) -> String {
    format!("0x{:X}", hwnd.0 as usize)
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

/// 一回限りの再断行要求を外から挿入する（要件 2.6 のシームと同じ形＝未適用の既定値）。
fn request_reassert(world: &mut World, balloon: Entity) {
    world.entity_mut(balloon).insert(ReassertZOrder::default());
}

/// 維持系だけを載せた 1 巡分の schedule。
fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(apply_zorder_pair_maintenance);
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
/// 「0 本でないこと」だけを見ると余計に出る側の欠陥（同じ是正が 2 度走る・再試行の輪が
/// できる等）を見逃すため、常に本数で固定する。
fn assert_line_count(out: &str, tag: &str, expected: usize, what: &str) {
    let found = lines_with(out, tag);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{tag}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

/// キャラ窓・割り込み窓・バルーン窓の 3 枚を「バルーン／割り込み／キャラ」の順
/// （手前 → 背後）で用意する。バルーンはキャラのすぐ手前に**居ない**。
struct Trio {
    parent: HWND,
    character: HWND,
    intruder: HWND,
    balloon: HWND,
}

impl Trio {
    fn create(tag: PCWSTR) -> Self {
        let parent = create_test_parent(tag);
        let character = create_test_child(parent, w!("character"));
        let intruder = create_test_child(parent, w!("intruder"));
        let balloon = create_test_child(parent, w!("balloon"));
        // 「バルーン／割り込み／キャラ」の順（手前 → 背後）を明示的に作る。
        insert_after(intruder, balloon);
        insert_after(character, intruder);
        let trio = Self {
            parent,
            character,
            intruder,
            balloon,
        };
        assert_eq!(
            get_window_below(trio.balloon).expect("GetWindow は成功するはず"),
            Some(trio.intruder),
            "前提: バルーンの 1 つ背後は割り込み窓（＝キャラの隣に居ない）"
        );
        trio
    }

    fn destroy(self) {
        destroy_test_window(self.parent);
    }
}

// -------------------------------------------------------------------------
// 要求の一生（挿入 → 適用 → 検証待ち → 実測照合 → 消滅）
// -------------------------------------------------------------------------

/// 外から挿入された再断行の要求が、適用され、次巡で実測照合され、消える。
///
/// 挿入の形は要件 2.6 のシームそのもの（`ReassertZOrder::default()`）であり、
/// 本テストが要件 7.6 の受入形——非表示 → 表示の再断行を決定論的テストで受け入れる——に当たる。
/// 検査は自前の帳簿だけでなく **OS 側の重なり**（`GetWindow` 実測）まで見る。
#[test]
fn reassert_request_is_applied_verified_and_then_disappears() {
    let trio = Trio::create(w!("wintf-zorder-maintain-lifecycle"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, trio.character);
    let balloon = spawn_window(&mut world, trio.balloon);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 1 巡目: 判断して指令を積み、検証待ちへ入る（実際の書込は flush 後）。
        schedule.run(&mut world);
        assert_eq!(
            world
                .entity(balloon)
                .get::<ReassertZOrder>()
                .map(|r| r.pending_verify),
            Some(Some(ExpectedOrder {
                above: trio.balloon,
                below: trio.character,
            })),
            "適用した巡では要求が検証待ちの段階へ進み、期待隣接を持つはず"
        );
        assert!(
            world.entity(balloon).contains::<IssuedPairFix>(),
            "次巡の記録に載せる「出した指令」が持ち越されているはず"
        );

        // tick 後の flush（本番と同じ経路）。ここで初めて OS の重なりが動く。
        SetWindowPosCommand::flush();

        // 2 巡目: 実測照合と記録、そして要求の消滅。
        schedule.run(&mut world);
        // 3 巡目: 要求が消えているので何も起きない（毎フレーム巡回では動かない）。
        schedule.run(&mut world);
    });

    assert_line_count(&out, SET_WINDOW_POS_TAG, 1, "発行された指令は 1 本だけ");
    assert_line_count(&out, FIX_TAG, 1, "是正の記録は検証の巡に 1 本だけ");
    assert_line_count(&out, VERIFY_FAILED_TAG, 0, "期待どおりなので不一致は無い");
    assert_line_count(&out, SKIP_TAG, 0, "見送っていない");

    let fix = lines_with(&out, FIX_TAG)[0];
    assert!(
        fix.contains(&format!("insert_after={}", hwnd_hex(trio.intruder))),
        "出した指令の挿入位置はキャラの 1 つ手前だった窓のはず: {fix}"
    );
    assert!(
        fix.contains(&format!(
            "measured_next_after_fix={}",
            hwnd_hex(trio.character)
        )),
        "適用後の実測はキャラ窓のはず: {fix}"
    );

    assert_eq!(
        get_window_below(trio.balloon).expect("GetWindow は成功するはず"),
        Some(trio.character),
        "OS 側でもバルーンの 1 つ背後がキャラ窓になっているはず"
    );
    assert_eq!(
        get_window_above(trio.character).expect("GetWindow は成功するはず"),
        Some(trio.balloon),
        "逆向きの実測でも隣接しているはず"
    );
    assert!(
        !world.entity(balloon).contains::<ReassertZOrder>(),
        "検証を終えた要求は消えているはず"
    );
    assert!(
        !world.entity(balloon).contains::<IssuedPairFix>(),
        "指令の持ち越しも要求とともに消えるはず"
    );

    trio.destroy();
}

// -------------------------------------------------------------------------
// 既に隣接している場合は一切の指令を出さない
// -------------------------------------------------------------------------

/// 既にバルーンがキャラのすぐ手前に居るなら、理由つきで見送り、指令は 1 本も出さない。
#[test]
fn already_adjacent_pair_issues_no_command_at_all() {
    let parent = create_test_parent(w!("wintf-zorder-maintain-adjacent"));
    let character_hwnd = create_test_child(parent, w!("character"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    // バルーンがキャラのすぐ手前に居る初期状態を明示的に作る。
    insert_after(character_hwnd, balloon_hwnd);
    assert_eq!(
        get_window_below(balloon_hwnd).expect("GetWindow は成功するはず"),
        Some(character_hwnd),
        "前提: バルーンの 1 つ背後はキャラ窓（＝既に隣接）"
    );

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 対照: 捕捉が生きていれば必ず 1 本現れる指令（重なりも位置も動かさない）。
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "是正の指令は出ず、対照の 1 本だけが出るはず",
    );
    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    assert_line_count(&out, FIX_TAG, 0, "是正していないので是正の記録は出ない");
    assert_line_count(&out, VERIFY_FAILED_TAG, 0, "検証段階へ進んでいない");

    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains("reason=AlreadyAdjacent"),
        "見送りの理由は「既に隣接している」であるはず: {skip}"
    );

    assert!(
        !world.entity(balloon).contains::<ReassertZOrder>(),
        "一回限りの要求は見送りとともに消えるはず（毎巡の再評価を残さない）"
    );
    assert!(
        !world.entity(balloon).contains::<IssuedPairFix>(),
        "指令を出していないので持ち越しも作らない"
    );
    assert_eq!(
        get_window_below(balloon_hwnd).expect("GetWindow は成功するはず"),
        Some(character_hwnd),
        "重なりは動いていないはず"
    );

    destroy_test_window(parent);
}

/// 要求が無ければ、宣言があり隣接もしていない状態でも維持系は何もしない（トリガ駆動のみ）。
///
/// 対照は 2 本立てる——同じ捕捉窓に**要求を持つ別のペア**（既に隣接しているので見送りが
/// 1 本出る）を置いて記録側の捕捉が生きていることを示し、指令の側は no-op の 1 本で示す。
/// これで「要求の無いペアからは記録も指令も出ない」の主張が、捕捉の死では成立しなくなる。
#[test]
fn without_a_request_the_maintenance_does_nothing() {
    let trio = Trio::create(w!("wintf-zorder-maintain-no-trigger"));
    let control_parent = create_test_parent(w!("wintf-zorder-maintain-no-trigger-control"));
    let control_character = create_test_child(control_parent, w!("character"));
    let control_balloon = create_test_child(control_parent, w!("balloon"));
    insert_after(control_character, control_balloon);

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, trio.character);
    let balloon = spawn_window(&mut world, trio.balloon);
    declare_pair(&mut world, balloon, character);

    // 対照のペア（要求を持つので確かに記録が出る側）
    let other_character = spawn_window(&mut world, control_character);
    let other_balloon = spawn_window(&mut world, control_balloon);
    declare_pair(&mut world, other_balloon, other_character);
    request_reassert(&mut world, other_balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(trio.balloon);
    });

    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "要求が無ければ指令は出ず、対照の 1 本だけが出るはず",
    );
    assert_line_count(
        &out,
        SKIP_TAG,
        1,
        "記録が出るのは要求を持つ対照のペアだけ（要求の無いペアは判断そのものを行わない）",
    );
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains(&format!("entity={other_balloon:?}")),
        "見送りの記録は対照のペアを指すはず: {skip}"
    );
    assert_line_count(&out, FIX_TAG, 0, "是正しない");
    assert_eq!(
        get_window_below(trio.balloon).expect("GetWindow は成功するはず"),
        Some(trio.intruder),
        "重なりは動いていないはず"
    );

    destroy_test_window(control_parent);
    trio.destroy();
}

// -------------------------------------------------------------------------
// 検証不一致（error・再試行の輪を作らない）
// -------------------------------------------------------------------------

/// 指令を出したのに重なりが動かなかった場合、次巡で error を 1 度だけ記録し、
/// 再度の指令は出さない（再試行の輪を作らない）。
///
/// 「効かなかった」状況は**指令を積んだまま flush しない**ことで決定論的に作る。
/// 捕捉の最後に flush して指令の本数を数えるため、この 1 本が最初の是正だけであること
/// ——すなわち検証不一致が新たな指令を生んでいないこと——も同時に固定される。
#[test]
fn verification_mismatch_is_recorded_once_without_retry() {
    let trio = Trio::create(w!("wintf-zorder-maintain-mismatch"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, trio.character);
    let balloon = spawn_window(&mut world, trio.balloon);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 1 巡目: 指令を積む（flush しない＝重なりは動かない）。
        schedule.run(&mut world);
        // 2 巡目: 実測は期待と食い違う。
        schedule.run(&mut world);
        // 3 巡目以降: 要求は消えており、再試行は起こらない。
        schedule.run(&mut world);
        schedule.run(&mut world);
        // 溜まっている指令をすべて出し切って本数を数える。
        SetWindowPosCommand::flush();
    });

    assert_line_count(
        &out,
        VERIFY_FAILED_TAG,
        1,
        "検証不一致の記録は 1 本だけ（再試行しない）",
    );
    assert_line_count(&out, FIX_TAG, 0, "一致していないので是正の記録は出ない");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "発行された指令は最初の 1 本だけ（不一致が新たな指令を生まない）",
    );

    let failed = lines_with(&out, VERIFY_FAILED_TAG)[0];
    assert!(
        failed.contains(&format!("expected_above={}", hwnd_hex(trio.balloon))),
        "期待した手前側はバルーン窓のはず: {failed}"
    );
    assert!(
        failed.contains(&format!("expected_below={}", hwnd_hex(trio.character))),
        "期待した背後側はキャラ窓のはず: {failed}"
    );
    assert!(
        failed.contains(&format!("measured={}", hwnd_hex(trio.intruder))),
        "実測は割り込み窓のはず: {failed}"
    );
    assert!(
        !world.entity(balloon).contains::<ReassertZOrder>(),
        "不一致でも要求はそこで消える（再試行の輪を作らない）"
    );

    trio.destroy();
}

// -------------------------------------------------------------------------
// 片割れの不在（要件 1.5 の正常系——error にしない）
// -------------------------------------------------------------------------

/// 指令を出した後・検証の前に相手が消えた場合、error ではなく理由つきの見送りになる。
#[test]
fn peer_lost_before_verification_skips_instead_of_failing() {
    let trio = Trio::create(w!("wintf-zorder-maintain-peer-lost"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, trio.character);
    let balloon = spawn_window(&mut world, trio.balloon);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 検証の前にキャラ窓の実体が消える（窓そのものは残っている）。
        world.entity_mut(character).despawn();
        schedule.run(&mut world);
        schedule.run(&mut world);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains("reason=PeerMissing"),
        "見送りの理由は「相手が居ない」であるはず: {skip}"
    );
    assert_line_count(
        &out,
        VERIFY_FAILED_TAG,
        0,
        "相手の消滅は正常系であり error にしない（要件 1.5）",
    );
    assert_line_count(&out, FIX_TAG, 0, "検証を行っていないので是正の記録も出ない");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "残る窓へは書き込まない（最初の是正の 1 本だけ）",
    );
    assert!(!world.entity(balloon).contains::<ReassertZOrder>());
    assert!(!world.entity(balloon).contains::<IssuedPairFix>());

    trio.destroy();
}

/// 検証待ちの巡に相手の**ハンドルだけ**が外れた場合も、理由つきの見送りになる。
///
/// 相手の実体は生きているので「相手が居ない」では言い当てられない——切離しの側が
/// `PeerLoss::HandleRemoved` として別に扱っているのと同じ状態である。実測に使う HWND が
/// 無い以上、照合は成立しないが検証不一致でもない。
///
/// 捕捉窓には 1 巡目の是正の指令（確かに 1 本出る）が同居するので、「検証不一致が 0 本」の
/// 主張が捕捉の死によって恒真になることはない。
#[test]
fn peer_losing_only_its_handle_before_verification_skips_with_handle_missing() {
    let trio = Trio::create(w!("wintf-zorder-maintain-peer-handle-lost"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, trio.character);
    let balloon = spawn_window(&mut world, trio.balloon);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        // 1 巡目: 是正を適用し、検証待ちへ進む。
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 実体は残したまま OS のハンドルだけが外れる（切離し側の `HandleRemoved` と同じ状態）。
        world.entity_mut(character).remove::<WindowHandle>();
        // 2 巡目: 検証段階だが、実測に使う HWND がもう無い。
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    let skip = lines_with(&out, SKIP_TAG)[0];
    assert!(
        skip.contains("reason=HandleMissing"),
        "理由は「ハンドルが無い」であるはず（相手の実体は生きている）: {skip}"
    );
    assert_line_count(
        &out,
        VERIFY_FAILED_TAG,
        0,
        "照合を行っていないので検証不一致にはしない",
    );
    assert_line_count(&out, FIX_TAG, 0, "照合を行っていないので是正の記録も出ない");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "出るのは 1 巡目の是正だけ（この 1 本が捕捉の生存を示す対照でもある）",
    );
    assert!(!world.entity(balloon).contains::<ReassertZOrder>());
    assert!(!world.entity(balloon).contains::<IssuedPairFix>());

    trio.destroy();
}

/// 要求を受けた巡で既に相手が居なければ、指令を出さずに理由つきで見送る。
#[test]
fn request_for_a_missing_peer_issues_no_command() {
    let parent = create_test_parent(w!("wintf-zorder-maintain-peer-gone"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = world.spawn_empty().id();
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);
    world.entity_mut(character).despawn();

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    assert!(lines_with(&out, SKIP_TAG)[0].contains("reason=PeerMissing"));
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "是正の指令は出ず、対照の 1 本だけが出るはず",
    );
    assert!(!world.entity(balloon).contains::<ReassertZOrder>());

    destroy_test_window(parent);
}

/// 両窓が生きていても、まだ OS のハンドルが付いていなければ指令を出さずに見送る。
#[test]
fn request_without_handles_issues_no_command() {
    let parent = create_test_parent(w!("wintf-zorder-maintain-handleless"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = world.spawn_empty().id();
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(&out, SKIP_TAG, 1, "見送りは理由つきで 1 本だけ");
    assert!(lines_with(&out, SKIP_TAG)[0].contains("reason=HandleMissing"));
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "是正の指令は出ず、対照の 1 本だけが出るはず",
    );
    assert!(!world.entity(balloon).contains::<ReassertZOrder>());

    destroy_test_window(parent);
}

// -------------------------------------------------------------------------
// 要求の形が壊れている場合（記録を残して落とす）
// -------------------------------------------------------------------------

/// ペア宣言を持たない窓に付いた要求は、記録を残して落とす（黙って読み飛ばさない）。
///
/// 同じ捕捉窓に、確かに記録が出る正しいペア（隣接済み → 見送り 1 本）を併置する。
#[test]
fn request_on_a_window_without_declaration_is_dropped_with_a_record() {
    let parent = create_test_parent(w!("wintf-zorder-maintain-orphan"));
    let character_hwnd = create_test_child(parent, w!("character"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    insert_after(character_hwnd, balloon_hwnd);

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    // 対照（確かに記録が出る側・既に隣接しているので見送りになる）
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    // 宣言を持たない窓へ付いた要求（挿入元の誤り）
    let stray = spawn_window(&mut world, fake_hwnd(0x00C1));
    request_reassert(&mut world, stray);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(
        &out,
        ORPHAN_REQUEST_MARK,
        1,
        "宣言の無い要求は 1 度だけ記録されて消えるはず",
    );
    assert_line_count(&out, SKIP_TAG, 1, "対照のペアは理由つきで 1 度見送られる");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "どちらも指令を出さず、対照の 1 本だけが出るはず",
    );
    assert!(
        lines_with(&out, ORPHAN_REQUEST_MARK)[0].contains(&format!("entity={stray:?}")),
        "記録は宣言の無い窓を指すはず"
    );
    assert!(
        !world.entity(stray).contains::<ReassertZOrder>(),
        "落とした要求は残さない（毎巡読み飛ばす状態を作らない）"
    );

    destroy_test_window(parent);
}

/// 出した指令の持ち越しが無い検証待ちの要求は、検証せずに記録を残して落とす。
///
/// [`ReassertZOrder`] は他クレートから挿入される公開の契約点であり、検証段階の値を
/// 外から作ることも言語上は可能である。その場合に「出してもいない指令」を記録したり、
/// 偽の検証不一致を error で残したりしないことを固定する。
#[test]
fn pending_verification_without_an_issued_command_is_dropped_with_a_record() {
    let parent = create_test_parent(w!("wintf-zorder-maintain-unbacked"));
    let character_hwnd = create_test_child(parent, w!("character"));
    let balloon_hwnd = create_test_child(parent, w!("balloon"));
    insert_after(character_hwnd, balloon_hwnd);

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    // 対照（確かに記録が出る側）
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);

    // 外から検証段階の値だけを作られたペア（持ち越しが無い）。
    // この枝は実測へ進む前に落ちるため、ハンドルは Win32 へ渡らない偽の値でよい。
    let other_character = spawn_window(&mut world, fake_hwnd(0x00D1));
    let other_balloon = spawn_window(&mut world, fake_hwnd(0x00D2));
    declare_pair(&mut world, other_balloon, other_character);
    world.entity_mut(other_balloon).insert(ReassertZOrder {
        pending_verify: Some(ExpectedOrder {
            above: fake_hwnd(0x00D2),
            below: fake_hwnd(0x00D1),
        }),
    });

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        schedule.run(&mut world);
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        issue_control_command(balloon_hwnd);
    });

    assert_line_count(
        &out,
        UNBACKED_VERIFY_MARK,
        1,
        "持ち越しの無い検証待ちは 1 度だけ記録されて消えるはず",
    );
    assert_line_count(&out, FIX_TAG, 0, "出してもいない指令を是正として記録しない");
    assert_line_count(&out, VERIFY_FAILED_TAG, 0, "偽の検証不一致も記録しない");
    assert_line_count(&out, SKIP_TAG, 1, "対照のペアは理由つきで 1 度見送られる");
    assert_line_count(
        &out,
        SET_WINDOW_POS_TAG,
        1,
        "どちらも指令を出さず、対照の 1 本だけが出るはず",
    );
    assert!(
        !world.entity(other_balloon).contains::<ReassertZOrder>(),
        "落とした要求は残さない"
    );

    destroy_test_window(parent);
}

// -------------------------------------------------------------------------
// 検証で使う実測（記録の付随フィールド）
// -------------------------------------------------------------------------

/// 背後側の走査でも、失敗は「背後に誰も居ない」と同じ `None` へ潰れるが黙って消えはしない。
///
/// 無効ハンドルでの走査は必ず失敗する。値としては不在（記録では番兵）になり、失敗そのものは
/// 記録に残る（[areka-log-first-no-silent-failure]）。対照は実窓での成功例
/// （本ファイルの各シナリオが `Some` を得ていること）。
#[test]
fn measure_window_below_maps_failure_to_absence_and_records_it() {
    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        assert_eq!(measure_window_below(HWND::default()), None);
    });
    assert!(
        !out.trim().is_empty(),
        "走査の失敗は記録に残るはず（黙って None へ潰さない）"
    );
}

// -------------------------------------------------------------------------
// 適用の組立（純関数）
// -------------------------------------------------------------------------

/// 是正の指令は重なりだけを動かす——位置・寸法・活性化を伴わない（要件 1.6）。
#[test]
fn pair_fix_command_moves_only_the_zorder() {
    let target = fake_hwnd(0x0A01);
    let after = fake_hwnd(0x0A02);

    let cmd = pair_fix_command(target, InsertSpec::After(after));
    assert_eq!(cmd.hwnd, target);
    assert_eq!(
        cmd.flags,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        "位置・寸法・活性化を伴わない 3 つのフラグちょうどであるはず（重なりの抑止は付かない）"
    );
    assert_eq!(cmd.hwnd_insert_after, Some(after));
    assert_eq!((cmd.x, cmd.y, cmd.width, cmd.height), (0, 0, 0, 0));

    // 縁（背後側より手前に可視の窓が居ない）は最上位への挿入になる。
    let top = pair_fix_command(target, InsertSpec::TopEdge);
    assert_eq!(top.flags, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    assert_eq!(top.hwnd_insert_after, Some(HWND_TOP));
    // 常時最前面（`HWND_TOPMOST`）は決して指さない（要件 4.3／8.1）。
    assert_ne!(top.hwnd_insert_after, Some(HWND_TOPMOST));
    assert_eq!(
        top.flags & SWP_NOZORDER,
        SET_WINDOW_POS_FLAGS(0),
        "重なりを動かす指令なので SWP_NOZORDER は立たない"
    );
}

/// 適用の計画は「動かす窓・挿入位置・期待隣接」を腕から一意に決める（要件 3.4）。
#[test]
fn plan_pair_fix_picks_the_moved_window_per_arm() {
    let above = fake_hwnd(0x0B01);
    let below = fake_hwnd(0x0B02);
    let front_of_below = fake_hwnd(0x0B03);
    let obs = PairObservation {
        trigger: PairTrigger::Reassert,
        strategy: ZOrderPairStrategy::default(),
        above_alive: true,
        below_alive: true,
        above_hwnd: Some(above),
        below_hwnd: Some(below),
        measured_below_of_above: None,
        measured_above_of_below: Some(front_of_below),
        measured_above_of_below_is_always_on_top: false,
    };

    let plan = plan_pair_fix(
        &obs,
        PairFixDecision::PlaceAboveOverBelow {
            insert_after: InsertSpec::After(front_of_below),
        },
    )
    .expect("手前へ寄せる腕は計画になるはず");
    assert_eq!(plan.target, above, "動かすのは手前側の窓");
    assert_eq!(plan.insert_after, InsertSpec::After(front_of_below));
    assert_eq!(plan.expected, ExpectedOrder { above, below });

    let plan = plan_pair_fix(
        &obs,
        PairFixDecision::PlaceBelowUnderAbove {
            insert_after: above,
        },
    )
    .expect("背後へ寄せる腕は計画になるはず");
    assert_eq!(plan.target, below, "動かすのは背後側の窓");
    assert_eq!(plan.insert_after, InsertSpec::After(above));
    assert_eq!(
        plan.expected,
        ExpectedOrder { above, below },
        "どちらの腕でも収束する隣接は同じ"
    );

    assert!(
        plan_pair_fix(&obs, PairFixDecision::Skip(SkipReason::AlreadyAdjacent)).is_none(),
        "見送りからは計画が作られない（指令を出さない）"
    );

    let handleless = PairObservation {
        above_hwnd: None,
        ..obs.clone()
    };
    assert!(
        plan_pair_fix(
            &handleless,
            PairFixDecision::PlaceAboveOverBelow {
                insert_after: InsertSpec::TopEdge
            }
        )
        .is_none(),
        "ハンドルが揃わない観測からは計画が作られない"
    );
}
