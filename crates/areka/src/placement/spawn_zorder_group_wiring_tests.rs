//! グループ維持系を本番の結線（[`wire_zorder_pair`](super::wire_zorder_pair)）へ載せたこと
//! を、**本番の経路そのもの**で固定するテスト（task 6.1・要件 6.3／6.4／9.5）。
//!
//! # 本ファイルが受け持つ主題——ペア機構との調停が本番で本当に効いているか
//!
//! 維持系の調停は「この巡に既存のペア機構が是正を出したか」を
//! `Query<(), Added<IssuedPairFix>>` で見る。ところがペア機構はその目印を**遅延コマンド**で
//! 付けるので、同じ巡のうちに見えるかどうかは**両者の間に同期点が入るか**だけに懸かって
//! いる。`chain()` はその同期点を置くが、`chain_ignore_deferred()` へ替えたり順序指定を
//! 落としたりすると同期点は消え、調停は**静かに**無効になる——両機構が同じ巡に窓を動かす
//! ようになるのに、真偽値を直に渡して判断だけを測っている wintf 側の決定論テストは
//! **全部緑のまま**である（真偽値がそもそも偽で届くようになるため）。
//!
//! よってここは判断ではなく**結線**を測る。本番の `wire_zorder_pair` が組んだ確定段を実際に
//! 回し、
//!
//! - ペア機構が**実際に是正を出した巡**にグループの指令が 1 本も出ないこと、
//! - ペア機構が是正を出さない巡には**連鎖がそのまま現れる**こと、
//!
//! の**対**を同じ World の連続する 2 巡で示す。片側だけでは空虚である——グループが常に
//! 黙っていても、常に喋っていても、片方の主張は緑になる。
//!
//! # なぜ実窓なのか
//!
//! ペア機構の確立系は `SetWindowLongPtrW(GWLP_HWNDPARENT)` を実際に呼び、維持系の判断は
//! `GetWindow` の実測を見る。偽のハンドルではどちらも成立せず、「ペア機構が是正を出した巡」
//! そのものを作れない。自プロセスが作った窓の間の owner 関係と、**自分が積んだ指令の列**は
//! 決定論的である（`zorder_group_order_tests.rs` と同じ判断）。
//!
//! # 決定論の担保——測るのは「積まれた指令」だけ
//!
//! 本ファイルは重なりの**実測値**を 1 つも検査しない。作る窓はすべて不可視なので、
//! 実測層（可視の窓しか通さない）から見ればペアは隣接しておらず、グループの相対順も
//! 成立していない——どちらの機構も「是正が要る」と判断する側で確定する。他プロセスの窓が
//! 何枚あっても、この 2 つの判断は変わらない。指令キューはスレッドローカルなので、並行に
//! 走る他テストの指令が混ざることもない。
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
use wintf::ecs::window::{ZOrderGroupSpec, ZOrderGroups, drain_window_pos_commands};
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

/// 積まれた指令を引き取り、`(動かした窓, 挿入先)` を与えた窓集合の添字で読む。
///
/// 添字にするのは、`HWND` の生値では「どの窓か」が読めないからである。集合の外を指す
/// 指令は `None` として残る——濾し落とすと「構成外の窓を動かした」が消える。
fn drained_as_indices(set: &[HWND]) -> Vec<(Option<usize>, Option<usize>)> {
    drain_window_pos_commands()
        .iter()
        .map(|cmd| {
            (
                set.iter().position(|w| *w == cmd.hwnd),
                cmd.hwnd_insert_after
                    .and_then(|after| set.iter().position(|w| *w == after)),
            )
        })
        .collect()
}

// -------------------------------------------------------------------------
// 必須の檻（task 4.1 → 6.1）——調停が本番の結線の上で成立している
// -------------------------------------------------------------------------

/// 本番の結線を回すと、ペア機構が是正を出した巡はグループの指令が 0 本になり、
/// ペア機構が出さない次の巡に連鎖がそのまま現れる（要件 6.3・調停）。
///
/// # 何が壊れると赤くなるか
///
/// 確立系・ペア維持系・グループ維持系の間の**同期点**が消えたとき——結線を
/// `chain_ignore_deferred()` へ替える、順序指定ごと落とす、といった変更である。同期点が
/// 無いと、ペア機構が遅延コマンドで付けた目印は同じ巡のグループ維持系から見えず、
/// `pair_fix_this_pass` が偽で届く。判断は正しいまま（偽なら発行する）なので wintf 側の
/// 決定論テストは 1 本も赤くならず、**巡 1 でグループの連鎖が混ざる**ここだけが落ちる。
///
/// # 2 巡を 1 本のテストに閉じている理由
///
/// 「出した巡」と「出さない巡」の違いを**ペア機構の状態だけ**にするためである。World も窓も
/// グループの宣言も同じまま、巡 1（ペアが是正を出す）から巡 2（ペアは検証だけで指令を
/// 出さない）へ移る。別々の World で組むと、グループが黙った理由が調停なのか World の
/// 組み方の違いなのかを分けられない。
#[test]
fn the_wired_route_holds_group_commands_on_a_pair_fix_pass_and_releases_them_on_the_next() {
    let char_hwnd = create_test_window(w!("areka zorder group wiring char"));
    let balloon_hwnd = create_test_window(w!("areka zorder group wiring balloon"));
    let group_hwnds: Vec<HWND> = (0..3)
        .map(|_| create_test_window(w!("areka zorder group wiring member")))
        .collect();
    // 添字の読み替え表——0=キャラ 1=バルーン 2..5=グループ構成窓。
    let all: Vec<HWND> = [char_hwnd, balloon_hwnd]
        .into_iter()
        .chain(group_hwnds.iter().copied())
        .collect();

    let mut world = World::new();
    world.init_resource::<Schedules>();
    let char_window = spawn_window(&mut world, char_hwnd);
    let balloon_window = spawn_window(&mut world, balloon_hwnd);
    world
        .entity_mut(balloon_window)
        .insert(KeepDirectlyAbove { peer: char_window });
    let members: Vec<Entity> = group_hwnds
        .iter()
        .map(|hwnd| spawn_window(&mut world, *hwnd))
        .collect();
    // 検証待ちと連続失敗数は維持系の内部状態なので外から組めない（既定のまま置く）。
    let mut groups = ZOrderGroups::default();
    groups.groups.push(ZOrderGroupSpec {
        id: 42,
        members: members.clone(),
    });
    groups.pending = true;
    world.insert_resource(groups);

    wire_zorder_pair(&mut world);
    let _residue = drain_window_pos_commands();

    // 巡 1——確立系が owner を張って再断行の要求を挿し、ペア維持系がそれを消費して是正を
    // 出す。グループ維持系はその巡に出た目印を見て見送る。
    world.run_schedule(FrameFinalize);
    let first = drained_as_indices(&all);
    let pending_after_first = world.resource::<ZOrderGroups>().pending;

    // 巡 2——ペア維持系は検証待ちの照合だけを行い、指令を出さない。調停の理由が消えるので
    // グループの連鎖が現れる。
    world.run_schedule(FrameFinalize);
    let second = drained_as_indices(&all);

    destroy(&all);
    let _residue = drain_window_pos_commands();

    // 巡 1 の非空虚性——ペア機構が**実際に**是正を出していなければ、以下の「0 本」は
    // 調停を 1 ビットも測っていない。
    assert_eq!(
        first.len(),
        1,
        "巡 1 に積まれた指令が 1 本ちょうどではない（ペア機構の是正が出ていないか、グループの連鎖が混ざっている）: {first:?}"
    );
    assert_eq!(
        first[0].0,
        Some(1),
        "巡 1 に積まれた 1 本が既存のペア機構の是正（バルーン窓）ではない: {first:?}"
    );
    assert!(
        pending_after_first,
        "調停で見送った巡に印が降りている（次の巡でやり直せない＝要件 8.3 の黙った断念）"
    );

    // 巡 2——先頭は動かさず、残り 2 枚を直前の窓の直後へ差し込む連鎖ちょうど。
    assert_eq!(
        second,
        vec![(Some(3), Some(2)), (Some(4), Some(3))],
        "巡 2 の連鎖が「先頭を除く残りを直前の窓の直後へ」になっていない（結線か調停の回帰）"
    );
}

// -------------------------------------------------------------------------
// 既定状態（グループ指定が一つも無い走行）——要件 6.4／9.5
// -------------------------------------------------------------------------

/// グループ指定が一つも無い走行では、既存のペア機構の記録が従来どおりの語彙で出て、
/// グループ系の記録は 1 行も出ない（要件 6.4・完了状態）。
///
/// 判定に `origin=` の件数を使わないのは、グループ発行の指令が凍結済みの `pair_fix_command`
/// を通るため書込の観測タグが `origin=zorder-pair` になるからである（task 4.1 の申し送り）。
/// 語彙の判定は診断タグ `[zorder-pair]`／`[zorder-group]` で行う。
#[test]
fn a_run_without_group_declarations_keeps_the_pair_records_in_their_old_vocabulary() {
    let char_hwnd = create_test_window(w!("areka zorder group default char"));
    let balloon_hwnd = create_test_window(w!("areka zorder group default balloon"));

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
// 結線の字面——同期点を消す書き換えを名指しで塞ぐ
// -------------------------------------------------------------------------

/// 結線は 3 本を**同期点つきの連なり**で確定段へ載せる（`*_ignore_deferred` を使わない）。
///
/// 上の挙動の檻が主であり、ここはその 1 点だけを字面で二重化する。同期点の有無は
/// 「グループの連鎖が 1 巡早く混ざる」という形でしか挙動に現れず、その観測は実窓と実 Win32
/// を要する——道具の側が壊れた巡に静かに緑へ倒れないよう、字面でも押さえておく。
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
            "FrameFinalize, ( establish_owner_links, apply_zorder_pair_maintenance, apply_zorder_group_maintenance, ) .chain(),"
        ) || squeezed.contains(
            "FrameFinalize, (establish_owner_links, apply_zorder_pair_maintenance, apply_zorder_group_maintenance).chain(),"
        ),
        "確定段へ載る 3 本の連なりが本文に見当たらない（結線の回帰）: {squeezed}"
    );
    assert!(
        !code.contains("ignore_deferred"),
        "同期点を挿さない連ね方（`*_ignore_deferred`）が使われている＝遅延コマンドで付く目印が同じ巡に見えず、調停が静かに無効になる"
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
