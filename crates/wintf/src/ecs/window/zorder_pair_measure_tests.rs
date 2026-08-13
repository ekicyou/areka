//! 隣接の実測層（`measure_window_above`／`measure_window_below`）に対する実窓テストと、
//! そこを通した維持系の 1 本。
//!
//! 本ファイルが固定するのは 1 点である——**隣接は「最も近い可視の隣」で測る**。
//!
//! 実機で確定した事実がその根拠である。スレッド既定の IME 窓（クラス `IME`・不可視・
//! 矩形 0x0）はキャラ窓を owner に持つため、Windows がキャラ窓の直上に置く。バルーン窓も
//! 同じキャラ窓を owner に持つ被 owner 窓なので、キャラ窓の直上には被 owner 窓が 2 枚並び、
//! その内部順序は我々の制御外である。よって `GetWindow` を 1 歩だけ辿る測り方では、
//! 配置が正しくても隣接判定が落ちる。可視の窓だけを見ればバルーンはキャラのすぐ手前に居る。
//!
//! # 「常に読み飛ばす」実装で緑にならないための対照
//!
//! 読み飛ばしを可視・不可視の別なく行う実装でも、不可視窓を挟んだ側のテストだけなら緑に
//! なる。よって**可視の窓を挟んだ場合は隣接が成立しない**（挟まった可視窓がそのまま返る）
//! ことを同じ形の対照として併せて固定する。
//!
//! # なぜ親窓の下の子窓で測るのか
//!
//! トップレベル窓の兄弟列はデスクトップ全体で共有され、他テストや他プロセスの窓が割り込む
//! ため隣接が決定論にならない（`api.rs`・維持系・沈降観測の各テストと同じ制約）。テスト専用の
//! 親窓を 1 枚立て、その子窓で測ることで兄弟列がその数枚に閉じる。
//!
//! **本ファイルの親窓は「可視」で作る**——`IsWindowVisible` は当該窓と祖先すべての
//! `WS_VISIBLE` を見るため、親が隠れていれば子はどう作っても不可視になり、可視／不可視の
//! 対照そのものが作れない。親は 0x0 の道具窓（`WS_EX_TOOLWINDOW`）であり、活性化を伴わない
//! `SW_SHOWNA` で見せるので、画面にもタスクバーにも現れない。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{ExecutorKind, Schedule};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    ShowWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_CHILD, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

use super::{SIBLING_SCAN_LIMIT, measure_window_above, measure_window_below};
use crate::api::{get_window_above, get_window_below, is_window_visible};
use crate::ecs::test_support::capture_under_filter;
use crate::ecs::window::{
    KeepDirectlyAbove, ReassertZOrder, SetWindowPosCommand, WindowHandle, ZOrderPairStrategy,
    apply_zorder_pair_maintenance,
};

/// 実機サインオフの `RUST_LOG`（design.md「Real-Machine Signoff」の wintf 側）。
const CAPTURE_DIRECTIVES: &str = "info,wintf::ecs::window::zorder_pair=debug";

/// 維持系まで通すテスト用（指令の実施記録を出す `command` モジュールを足したもの）。
const MAINTAIN_CAPTURE_DIRECTIVES: &str =
    "info,wintf::ecs::window::zorder_pair=debug,wintf::ecs::window::command=debug";

/// 走査が上限で打ち切られた記録。
const LIMIT_MARK: &str = "走査が上限に達しました";
/// 走査そのものが失敗した記録（捕捉が生きていることの対照に使う）。
const FAILURE_MARK: &str = "走査に失敗しました";
/// 是正の記録（指令と実測を同一行に載せる）。
const FIX_TAG: &str = "[zorder-pair] fix";
/// 検証不一致の記録（error 水準）。
const VERIFY_FAILED_TAG: &str = "[zorder-pair] verify-failed";

// -------------------------------------------------------------------------
// 実窓のヘルパ
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

/// 可視のテスト専用親窓（兄弟列を閉じる入れ物・子を可視にできる唯一の形）。
///
/// 活性化を伴わない `SW_SHOWNA` で見せる——他のテストが握っている前面窓を奪わない。
fn create_visible_test_parent(title: PCWSTR) -> HWND {
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
    assert!(
        is_window_visible(parent),
        "前提: 親窓が可視でなければ子窓は決して可視にならない"
    );
    parent
}

/// 可視の子窓（`WS_VISIBLE` 付き・親が可視なので `IsWindowVisible` も真になる）。
fn create_visible_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0),
        Some(parent),
    )
}

/// 不可視の子窓（`WS_VISIBLE` 無し＝実機の既定 IME 窓に当たる役）。
fn create_hidden_child(parent: HWND, title: PCWSTR) -> HWND {
    create_window(
        title,
        WINDOW_EX_STYLE(0),
        WINDOW_STYLE(WS_CHILD.0),
        Some(parent),
    )
}

/// `hwnd` を `after` のすぐ背後（1 つ背面側）へ移す。
///
/// 初期配置は生成順に依存させない——子窓は先に作ったものほど手前であり、生成順から
/// 前後関係を読むと逆になる（`api.rs` のラッパテストと同じ作法）。
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

/// 「バルーン／割り込み 1 枚／キャラ」の順（手前 → 背後）で 3 枚を用意する。
struct Trio {
    parent: HWND,
    balloon: HWND,
    intruder: HWND,
    character: HWND,
}

impl Trio {
    /// `visible_intruder` が真なら割り込み窓を可視で作る（対照側）。
    fn create(tag: PCWSTR, visible_intruder: bool) -> Self {
        let parent = create_visible_test_parent(tag);
        let balloon = create_visible_child(parent, w!("balloon"));
        let intruder = if visible_intruder {
            create_visible_child(parent, w!("intruder"))
        } else {
            create_hidden_child(parent, w!("intruder"))
        };
        let character = create_visible_child(parent, w!("character"));
        // 「バルーン／割り込み／キャラ」の順（手前 → 背後）を明示的に作る。
        insert_after(intruder, balloon);
        insert_after(character, intruder);

        let trio = Self {
            parent,
            balloon,
            intruder,
            character,
        };
        // 前提の確認——素の `GetWindow` では割り込み窓が確かに間に居る。
        assert_eq!(
            get_window_below(trio.balloon).expect("GetWindow は成功するはず"),
            Some(trio.intruder),
            "前提: 素の 1 歩ではバルーンの背後は割り込み窓"
        );
        assert!(
            is_window_visible(trio.balloon) && is_window_visible(trio.character),
            "前提: ペアの 2 窓はどちらも可視"
        );
        assert_eq!(
            is_window_visible(trio.intruder),
            visible_intruder,
            "前提: 割り込み窓の可視性は指定どおり"
        );
        trio
    }

    fn destroy(self) {
        destroy_test_window(self.parent);
    }
}

// -------------------------------------------------------------------------
// 最も近い可視の隣
// -------------------------------------------------------------------------

/// 不可視の窓が間に居ても、実測は**その先の可視の窓**を返す（隣接は成立する）。
#[test]
fn measurement_skips_invisible_windows_and_reports_the_nearest_visible_neighbour() {
    let trio = Trio::create(w!("wintf-zorder-measure-hidden"), false);

    assert_eq!(
        measure_window_below(trio.balloon),
        Some(trio.character),
        "不可視の窓は遮蔽しない——バルーンの最も近い可視の背後はキャラ窓"
    );
    assert_eq!(
        measure_window_above(trio.character),
        Some(trio.balloon),
        "逆向きの実測も同じ——キャラ窓の最も近い可視の手前はバルーン窓"
    );

    trio.destroy();
}

/// 対照: **可視**の窓が間に居れば隣接は成立せず、その可視窓がそのまま返る。
///
/// これが無いと「常に読み飛ばす」実装（可視性を見ない実装）でも上のテストは緑になる。
#[test]
fn measurement_stops_at_a_visible_intruder() {
    let trio = Trio::create(w!("wintf-zorder-measure-visible"), true);

    assert_eq!(
        measure_window_below(trio.balloon),
        Some(trio.intruder),
        "可視の窓は読み飛ばさない——返るのは割り込み窓であってキャラ窓ではない"
    );
    assert_ne!(
        measure_window_below(trio.balloon),
        Some(trio.character),
        "可視の窓を挟んだ状態を隣接と判定してはならない"
    );
    assert_eq!(
        measure_window_above(trio.character),
        Some(trio.intruder),
        "逆向きの実測も同じ"
    );

    trio.destroy();
}

// -------------------------------------------------------------------------
// 走査の上限（不可視窓が延々と続く場合）
// -------------------------------------------------------------------------

fn lines_with<'a>(out: &'a str, mark: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(mark)).collect()
}

fn assert_line_count(out: &str, mark: &str, expected: usize, what: &str) {
    let found = lines_with(out, mark);
    assert_eq!(
        found.len(),
        expected,
        "{what}: `{mark}` の行は {expected} 本のはずが {} 本だった\n---捕捉全文---\n{out}",
        found.len()
    );
}

/// 不可視の窓が上限を超えて続く場合、値は**不在（番兵）へ倒れ**、打切りは記録に残る。
///
/// 上限に達したことを「隣が居ない」と静かに同一視すると、測れなかっただけの巡が
/// 偽の判定として記録に残る。よって値は不在にしつつ、打切りそのものを必ず記録する。
///
/// 捕捉窓には**確実に拾える対照**（無効ハンドルでの走査失敗）を併置してある——
/// 捕捉が死んでいればこちらも 0 本になり、テストは赤になる。
#[test]
fn measurement_falls_back_to_absence_when_the_scan_hits_its_limit() {
    let parent = create_visible_test_parent(w!("wintf-zorder-measure-limit"));
    let balloon = create_visible_child(parent, w!("balloon"));
    let character = create_visible_child(parent, w!("character"));
    insert_after(character, balloon);
    // 上限ちょうどの枚数の不可視窓を、バルーンの背後へ順に積む（可視のキャラ窓は最後尾へ）。
    let mut tail = balloon;
    let mut first_hidden = None;
    for _ in 0..SIBLING_SCAN_LIMIT {
        let hidden = create_hidden_child(parent, w!("hidden"));
        insert_after(hidden, tail);
        first_hidden.get_or_insert(hidden);
        tail = hidden;
    }
    insert_after(character, tail);
    let first_hidden = first_hidden.expect("上限は 1 以上のはず");

    // 前提: 列は「バルーン／不可視 512 枚／キャラ」の順で閉じている。
    assert_eq!(
        get_window_below(balloon).expect("GetWindow は成功するはず"),
        Some(first_hidden),
        "前提: バルーンの背後は不可視窓の列"
    );
    assert_eq!(
        get_window_below(character).expect("GetWindow は成功するはず"),
        None,
        "前提: キャラ窓は列の最後尾"
    );
    // 対照: 同じ列でも可視の窓が上限の内側に居れば、走査はちゃんと見つける。
    assert_eq!(
        measure_window_above(first_hidden),
        Some(balloon),
        "対照: 上限に達しない距離の可視窓は返る（常に不在へ倒す実装なら赤になる）"
    );

    let out = capture_under_filter(CAPTURE_DIRECTIVES, || {
        assert_eq!(
            measure_window_below(balloon),
            None,
            "上限に達したら不在（番兵）へ倒す——偽の隣接も偽の失敗も作らない"
        );
        // 対照: 同じ捕捉窓で確実に 1 本出る記録（捕捉が生きている証拠）。
        assert_eq!(measure_window_below(HWND::default()), None);
    });

    assert_line_count(&out, LIMIT_MARK, 1, "打切りはちょうど 1 本記録される");
    assert_line_count(&out, FAILURE_MARK, 1, "対照の走査失敗もちょうど 1 本");
    assert!(
        lines_with(&out, LIMIT_MARK)[0].contains(&format!("limit={SIBLING_SCAN_LIMIT}")),
        "打切りの記録には上限値が載るはず\n---捕捉全文---\n{out}"
    );

    destroy_test_window(parent);
}

// -------------------------------------------------------------------------
// 維持系まで通した形（不可視の窓が間に残っても検証は通る）
// -------------------------------------------------------------------------

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
///
/// シングルスレッド実行器を明示するのは、既定の多スレッド実行器では
/// [`capture_under_filter`] が 1 行も拾えず記録の検査が空虚に緑になるためである。
fn maintain_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.set_executor_kind(ExecutorKind::SingleThreaded);
    schedule.add_systems(apply_zorder_pair_maintenance);
    schedule
}

fn hwnd_hex(hwnd: HWND) -> String {
    format!("0x{:X}", hwnd.0 as usize)
}

/// 是正の後にペアの間へ残るのが**不可視**の窓なら、検証は隣接と判定して是正の記録を出す。
///
/// 実機では、キャラ窓を owner に持つスレッド既定の IME 窓（不可視・0x0）がキャラ窓の直上に
/// 居座るため、`GetWindow` を 1 歩だけ辿る測り方では毎回 `verify-failed` になっていた。
/// 可視の窓だけを見れば配置は正しい——本テストはその形をそのまま固定する。
///
/// 同じ捕捉窓に、**確かに落ちる対照のペア**（間に入るのが可視の窓・指令を flush しないので
/// 実測が期待と食い違う）を併置してある。2 種の記録がそれぞれ 1 本ずつ出ることと、
/// **どちらの行がどちらの entity を指しているか**まで検査する。
#[test]
fn an_invisible_window_between_the_pair_still_verifies_as_adjacent() {
    // 対象のペア: 「不可視／キャラ／バルーン」の順（手前 → 背後）。是正でバルーンが最上位へ
    // 上がり、不可視窓はバルーンとキャラの間に残る。
    let parent = create_visible_test_parent(w!("wintf-zorder-measure-maintain"));
    let hidden = create_hidden_child(parent, w!("hidden"));
    let character_hwnd = create_visible_child(parent, w!("character"));
    let balloon_hwnd = create_visible_child(parent, w!("balloon"));
    insert_after(character_hwnd, hidden);
    insert_after(balloon_hwnd, character_hwnd);
    assert_eq!(
        get_window_above(character_hwnd).expect("GetWindow は成功するはず"),
        Some(hidden),
        "前提: キャラ窓の 1 つ手前は不可視の窓"
    );

    // 対照のペア: 間に入るのは**可視**の窓。別の親窓に閉じるので対象のペアと干渉しない。
    let control = Trio::create(w!("wintf-zorder-measure-maintain-control"), true);

    let mut world = World::new();
    world.insert_resource(ZOrderPairStrategy::default());
    let character = spawn_window(&mut world, character_hwnd);
    let balloon = spawn_window(&mut world, balloon_hwnd);
    declare_pair(&mut world, balloon, character);
    request_reassert(&mut world, balloon);
    let control_character = spawn_window(&mut world, control.character);
    let control_balloon = spawn_window(&mut world, control.balloon);
    declare_pair(&mut world, control_balloon, control_character);

    let mut schedule = maintain_schedule();
    let out = capture_under_filter(MAINTAIN_CAPTURE_DIRECTIVES, || {
        // 1 巡目: 対象のペアが指令を積む。flush して実際に重なりを動かす。
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
        // 2 巡目: 対象のペアの検証（不可視窓を読み飛ばして隣接と判定される）。
        // 同じ巡で対照のペアの要求を挿し、次巡で必ず落ちる形を作る。
        request_reassert(&mut world, control_balloon);
        schedule.run(&mut world);
        // 3 巡目: 対照のペアの検証。指令を flush していないので実測は期待と食い違う。
        schedule.run(&mut world);
        SetWindowPosCommand::flush();
    });

    assert_line_count(&out, FIX_TAG, 1, "是正の記録は対象のペアの 1 本だけ");
    assert_line_count(
        &out,
        VERIFY_FAILED_TAG,
        1,
        "検証不一致は対照のペアの 1 本だけ",
    );

    let fix = lines_with(&out, FIX_TAG)[0];
    assert!(
        fix.contains(&format!("entity={balloon:?}")),
        "是正の記録は対象のペアのバルーン窓を指すはず: {fix}"
    );
    assert!(
        fix.contains(&format!(
            "measured_next_after_fix={}",
            hwnd_hex(character_hwnd)
        )),
        "実測は不可視窓を読み飛ばしてキャラ窓になるはず: {fix}"
    );
    let failed = lines_with(&out, VERIFY_FAILED_TAG)[0];
    assert!(
        failed.contains(&format!("entity={control_balloon:?}")),
        "検証不一致の記録は対照のペアのバルーン窓を指すはず: {failed}"
    );

    // OS 側の実配置: 不可視の窓は確かに間に残っている（読み飛ばしたのであって、
    // 我々が不可視窓を動かしたわけではない）。
    assert_eq!(
        get_window_below(balloon_hwnd).expect("GetWindow は成功するはず"),
        Some(hidden),
        "素の 1 歩ではバルーンの背後は不可視の窓のまま"
    );
    assert_eq!(
        measure_window_below(balloon_hwnd),
        Some(character_hwnd),
        "可視の窓だけを見ればバルーンのすぐ背後はキャラ窓"
    );

    control.destroy();
    destroy_test_window(parent);
}
