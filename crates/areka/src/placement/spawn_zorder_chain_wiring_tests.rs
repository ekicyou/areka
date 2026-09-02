//! 鎖の適用系を本番の結線（[`wire_zorder_pair`](super::wire_zorder_pair)）へ載せたこと
//! を、**本番の経路そのもの**で固定するテスト（task 3.2・要件 6.4／9.5／14.5）。
//!
//! # 本ファイルが受け持つ主題——仕上げの処理列の末尾に鎖の適用系が居るか
//!
//! 本番の確定段（`FrameFinalize`）には 3 本が**この順で**載る。
//!
//! 1. 所有関係の確立（[`establish_owner_links`](wintf::ecs::window::establish_owner_links)）
//! 2. スコープ内ペアの維持（[`apply_zorder_pair_maintenance`](wintf::ecs::window::apply_zorder_pair_maintenance)）
//! 3. **鎖の適用**（[`apply_zorder_chain`](wintf::ecs::window::apply_zorder_chain)）
//!
//! 鎖の適用が末尾なのは、同じ巡のうちに⑴確立系が張った owner と⑵ペア機構が直したスコープ
//! 内の隣接の**両方が済んだ姿**を前提に、スコープをまたぐ繋ぎだけを書きたいからである
//! （design.md「Revalidation Triggers」がこの順を明記している）。順序を落とすと鎖の適用が
//! ペア機構より前へ回りうるが、その差は**実窓を持たない檻には原理的に映らない**——両者は
//! 書込先（スコープ内ペアの owner ／スコープをまたぐ owner）が交わらないので、判断だけを
//! 測っているテストは全部緑のままである。
//!
//! よってここは判断ではなく**結線**を測る。
//!
//! - 本番の `wire_zorder_pair` が組んだ確定段を実際に回すと、鎖の適用系が**その巡のうちに**
//!   受け口を読むこと（[`the_wired_route_runs_the_chain_apply_in_the_same_pass`]・対照つき）
//! - グループ指定が一つも無い走行では、既存のペア機構の記録が従来どおりの語彙で出て、
//!   重なり系の新しい記録は 1 行も出ないこと（要件 6.4）
//! - 3 本の並び順そのもの（[`the_wiring_chains_the_three_systems_with_a_sync_point`]）
//!
//! # 決定論の担保——測るのは「積まれた指令」と「受け口の印」だけ
//!
//! 本ファイルは重なりの**実測値**を 1 つも検査しない。作る窓はすべて不可視なので、実測層
//! （可視の窓しか通さない）から見ればペアは隣接しておらず、ペア機構は「是正が要る」と判断
//! する側で確定する。他プロセスの窓が何枚あっても、この判断は変わらない。指令キューは
//! スレッドローカルなので、並行に走る他テストの指令が混ざることもない。
//!
//! 指令は**流さない**（`flush` を呼ばない）。積まれた列を引き取って読むだけであり、実際の
//! `SetWindowPos` は 1 度も走らない。

use bevy_ecs::prelude::*;
use bevy_ecs::schedule::Schedules;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP,
};
use windows::core::{PCWSTR, w};
use wintf::ecs::window::{ZOrderChainPlan, drain_window_pos_commands};
use wintf::ecs::{FrameFinalize, KeepDirectlyAbove, WindowHandle};

use super::wire_zorder_pair;
use crate::placement::test_support::capture_logs;

// -------------------------------------------------------------------------
// 実窓・World のヘルパ（`spawn_zorder_pair_wiring_tests.rs` と同一レシピ）
// -------------------------------------------------------------------------

/// 自プロセス所有の非表示トップレベル窓を作る。
fn create_test_window(title: PCWSTR) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。定義済 "Static" クラスで非表示ウィンドウを生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("Static"),
            title,
            WINDOW_STYLE(WS_POPUP.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a hidden test window")
    }
}

/// 作った窓をすべて破棄する（作った枚数と壊す枚数を必ず揃える）。
fn destroy(windows: &[HWND]) {
    for hwnd in windows {
        // SAFETY: Win32 境界。自プロセスが生成した窓を破棄する。
        unsafe {
            let _ = DestroyWindow(*hwnd);
        }
    }
}

/// ハンドル付きの窓 entity を作る。
fn spawn_window(world: &mut World, hwnd: HWND) -> Entity {
    world
        .spawn(WindowHandle {
            hwnd,
            instance: HINSTANCE::default(),
        })
        .id()
}

// -------------------------------------------------------------------------
// 必須の檻（task 3.2）——鎖の適用系が本番の結線の上を走っている
// -------------------------------------------------------------------------

/// 本番の結線を回すと、鎖の適用系が**その巡のうちに**受け口を読む（要件 14.5）。
///
/// # 何を観測しているのか
///
/// 受け口（[`ZOrderChainPlan`]）の `dirty` は「内容が変わった」の印であり、**適用系だけが**
/// 降ろす。よって印が降りていることは、確定段に鎖の適用系が実在して走ったことと同義で
/// ある。望む鎖は空（`chain: None`）にしてあるので、実行環境への書込は 1 度も起きない
/// ——測っているのは配線の実在であって、鎖の中身の正しさではない（そちらは wintf 側の
/// 兄弟テストが受け持つ）。
///
/// # 何が壊れると赤くなるか
///
/// 確定段の 3 本目を落とす、退役した維持系へ戻す、別のスケジュールへ載せ替える——いずれも
/// 印が立ったまま残る。挙動としての差は「重なりが組み替わらない」だけであり、判断だけを
/// 測っている wintf 側の決定論テストは 1 本も赤くならない。
///
/// # 対照
///
/// 結線しない World では印がそのまま残る。これが無いと「印は誰も立てないので常に降りて
/// いる」形の壊れ方に気づけない（片側だけの主張は空虚である）。
#[test]
fn the_wired_route_runs_the_chain_apply_in_the_same_pass() {
    let mut wired = World::new();
    wired.init_resource::<Schedules>();
    wired.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });
    wire_zorder_pair(&mut wired);

    wired.run_schedule(FrameFinalize);

    assert!(
        !wired.resource::<ZOrderChainPlan>().dirty,
        "本番の結線を回しても受け口の印が降りていない（確定段に鎖の適用系が載っていない）"
    );

    // 対照——印を降ろしているのは確かに結線された適用系である。
    let mut unwired = World::new();
    unwired.init_resource::<Schedules>();
    unwired.insert_resource(ZOrderChainPlan {
        chain: None,
        dirty: true,
    });
    unwired.add_schedule(Schedule::new(FrameFinalize));

    unwired.run_schedule(FrameFinalize);

    assert!(
        unwired.resource::<ZOrderChainPlan>().dirty,
        "結線していない World でも印が降りている（上の檻が配線を 1 ビットも測っていない）"
    );
}

// -------------------------------------------------------------------------
// 既定状態（グループ指定が一つも無い走行）——要件 6.4／9.5
// -------------------------------------------------------------------------

/// グループ指定が一つも無い走行では、既存のペア機構の記録が従来どおりの語彙で出て、
/// 重なり系の新しい記録は 1 行も出ない（要件 6.4・完了状態）。
///
/// 受け口そのものを置かない——これが本番の既定状態である（取り出しの相はグループが 1 本も
/// 無い間、鎖を公開しない）。よって適用系は仕事を持たず、積まれる指令はペア機構の 1 本だけ
/// である。
///
/// 判定に `origin=` の件数を使わないのは、書込の観測タグが凍結済みの `pair_fix_command`
/// 由来で `origin=zorder-pair` になるからである（task 4.1 の申し送り）。語彙の判定は診断タグ
/// `[zorder-pair]`／`[zorder-group]`／`[zorder-chain]` で行う。
#[test]
fn a_run_without_group_declarations_keeps_the_pair_records_in_their_old_vocabulary() {
    let char_hwnd = create_test_window(w!("areka zorder chain default char"));
    let balloon_hwnd = create_test_window(w!("areka zorder chain default balloon"));

    let mut world = World::new();
    world.init_resource::<Schedules>();
    let char_window = spawn_window(&mut world, char_hwnd);
    let balloon_window = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon_window)
        .insert(KeepDirectlyAbove { peer: char_window });

    wire_zorder_pair(&mut world);
    let _residue = drain_window_pos_commands();

    let (_, events) = capture_logs(|| world.run_schedule(FrameFinalize));
    let commands = drain_window_pos_commands();

    destroy(&[char_hwnd, balloon_hwnd]);
    let _residue = drain_window_pos_commands();

    let messages: Vec<String> = events.iter().map(|e| e.message().to_string()).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("[zorder-pair] owner-established")),
        "既存のペア機構の確立の記録が従来どおりの語彙で出ていない: {messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("[zorder-group]")),
        "グループ指定が一つも無い走行でグループ系の記録が出ている（既定＝非強制が崩れている）: {messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("[zorder-chain]")),
        "グループ指定が一つも無い走行で鎖系の記録が出ている（既定＝非強制が崩れている）: {messages:?}"
    );
    assert_eq!(
        commands.len(),
        1,
        "グループ指定の無い走行で積まれる指令は既存のペア機構の 1 本だけ: {commands:?}"
    );
    assert_eq!(
        commands[0].hwnd, balloon_hwnd,
        "積まれた 1 本が既存のペア機構の是正（バルーン窓）ではない"
    );
}

// -------------------------------------------------------------------------
// 結線の字面——並び順と同期点を消す書き換えを名指しで塞ぐ
// -------------------------------------------------------------------------

/// 結線は 3 本を**この並びの・同期点つきの連なり**で確定段へ載せる
/// （所有関係の確立 → スコープ内ペアの維持 → 鎖の適用。`*_ignore_deferred` を使わない）。
///
/// 上の挙動の檻は「鎖の適用系が居ること」までしか測れない——**3 本目であること**は、
/// 書込先の交わらない 2 機構の前後を入れ替えても挙動に現れないので、字面でしか押さえら
/// れない。同期点の有無も同様に「同じ巡のうちに前段の結果が見えるか」という形でしか現れず、
/// その観測は実窓と実 Win32 を要する。
///
/// 走査は**説明文を落とした本文**へ当てる。素の全文には本 doc の語も入るので、当てる先を
/// 間違えると検査が恒真になる（対照はこのテストの末尾 3 本）。
#[test]
fn the_wiring_chains_the_three_systems_with_a_sync_point() {
    let raw = include_str!("spawn.rs");
    let code = code_only(raw);
    let squeezed = squeeze(&code);

    assert!(
        squeezed.contains(
            "FrameFinalize, ( establish_owner_links, apply_zorder_pair_maintenance, apply_zorder_chain, ) .chain(),"
        ) || squeezed.contains(
            "FrameFinalize, (establish_owner_links, apply_zorder_pair_maintenance, apply_zorder_chain).chain(),"
        ),
        "確定段へ載る 3 本の連なり（末尾が鎖の適用系）が本文に見当たらない（結線の回帰）: {squeezed}"
    );
    assert!(
        !code.contains("apply_zorder_group_maintenance"),
        "退役する維持系がまだ確定段の結線に居る（順序指定の差し替え漏れ）"
    );
    assert!(
        !code.contains("ignore_deferred"),
        "同期点を挿さない連ね方（`*_ignore_deferred`）が使われている＝前段が遅延コマンドで付けた結果が同じ巡に見えない"
    );

    // 対照——落とし過ぎ／落とし漏れが無いこと。
    assert!(
        code.contains("pub fn wire_zorder_pair(world: &mut World) {"),
        "説明文を落とす処理が本文まで落としている"
    );
    assert!(
        !code.contains("スコープ間には一切張らない"),
        "説明文が落ちていない（走査が恒真になっている）"
    );
    assert!(
        raw.contains("スコープ間には一切張らない"),
        "対照の前提が崩れている（素の全文に説明文が無い）"
    );
}

/// 説明文（`//` で始まる行）を落とし、コードだけの本文を返す。
fn code_only(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 空白の連なりを 1 つに詰める（改行や字下げの入り方で檻が壊れないようにする）。
fn squeeze(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}
