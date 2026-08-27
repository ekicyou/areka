//! グループの連鎖を**実窓 4 枚**へ実際に適用し、最終的な重なりが宣言どおりになることを
//! 固定するテスト（要件 1.1／1.2）。
//!
//! 判断の側——どの窓を動かすか・先頭を動かさないか——は兄弟の
//! `zorder_group_decision_tests.rs` と
//! [`zorder_group_maintain_tests`](super::zorder_group_maintain_tests) が偽ハンドルで
//! 固定している。本ファイルが受け持つのはその先、**出した指令が Windows 上で本当に宣言
//! どおりの重なりに着くか**である。偽ハンドルの檻は指令の中身までしか見ておらず、
//! 「`SetWindowPos` の挿入位置の意味論をこちらが取り違えている」形は 1 本も赤にしない。
//!
//! # 何を固定するか
//!
//! - **最終形**——連鎖を流し終えたあと、4 枚の相対順が宣言順（手前から `0,1,2,3`）に
//!   なっていること。始点は宣言の**逆順**（`3,2,1,0`）に揃えるので、4 枚すべてが動く。
//! - **両経路**——一括投入（`DeferWindowPos`）と 1 本ずつの `SetWindowPos`（縮退経路）の
//!   どちらでも同じ最終形に着くこと。加えて**両者が互いに一致する**こと。片方ずつ期待値と
//!   比べるだけだと、両経路が同時に同じ方向へずれたときに食い違いを見逃す。
//! - **先頭を動かさないこと**——連鎖が積む指令は `chain` の長さちょうどであり、先頭の窓を
//!   対象にした指令は 1 本も出ない。これは**最終形からは見えない**（先頭を自分自身の直後へ
//!   差し込む指令は重なりを変えないため、最終形だけを見る檻は素通りする）ので、積まれた
//!   指令そのものに対して別途主張する。
//!
//! # 一括投入の側が測っているもの
//!
//! グループの連鎖は自己参照（`chain[i]` を直前の窓の直後へ）なので一括で積んでよい、と
//! [`zorder_group_maintain`](super) の module doc が述べている。その根拠は
//! 「`DeferWindowPos` の一括投入は積んだ順を保存する」であり、
//! `command_batch_tests.rs` の
//! `a_batched_zorder_list_lands_in_the_same_order_as_one_write_per_command` が
//! **素の Z 指令列**について実窓で示している。本ファイルはそれを**本機能の指令構築
//! （`decide_group_fix` → `enqueue_group_chain` → `pair_fix_command`）を通した形**で
//! もう一度示す——Win32 が順を保つことではなく、こちらの組み立てが順を保つことが主張で
//! ある。
//!
//! # 実窓を使うのに決定論である理由（要件 10.3）
//!
//! 測るのは**この 4 枚どうしの相対順だけ**である。
//!
//! - 走査（[`relative_z_order`]）は最前面から `GW_HWNDNEXT` で降りながら、自分が作った
//!   窓だけを拾う。他プロセスの窓が何枚挟まっていても添字の列は変わらない。**隣接では
//!   なく順序**で見るので、既定の IME 窓のような不可視の隣も結果を動かさない。
//! - 窓は自プロセスが作った 0x0・不可視の道具窓であり、他のテストも他のアプリもこれらを
//!   動かさない。指令はすべて `SWP_NOACTIVATE` なので活性化も奪わない。
//! - 指令キューは `thread_local!`（`command.rs`）なので、並列に走る他テストの積んだ指令が
//!   この巡に混ざることはない。
//! - 始点は生成順に頼らず [`arrange_z`] で明示的に組み、その成立をテスト本体が自己検査する
//!   ——始点が揃っていなければ以降の比較は空虚だからである。
//!
//! したがって本ファイルが赤くなったら、それはデスクトップの雑音ではなく**指令構築の
//! 回帰**である。唯一の例外は始点の自己検査が落ちる場合で、そのときは「始点が揃って
//! いない」と名指しで落ちる。
//!
//! 走査と添字化（[`relative_z_order`]／[`z_shape`]）は `command_batch_tests.rs` の同名の
//! 助手と同じ形である。あちらは非公開の内側に閉じており、共有するには測り方を公開面へ
//! 押し出すことになるので写した。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GetTopWindow, GetWindow, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::enqueue_group_chain;
use crate::ecs::window::zorder_group::{GroupFixDecision, GroupObservation, decide_group_fix};
use crate::ecs::window::{
    SetWindowPosCommand, drain_window_pos_commands, flush_window_pos_commands,
    with_forced_batch_begin_failure,
};

/// 1 グループの窓数（連鎖が 3 段になる＝軸が 2 度進む最小より 1 段深い）。
const GROUP_SIZE: usize = 4;

/// 宣言どおりの最終形（手前から順の添字）。
const DECLARED: [usize; GROUP_SIZE] = [0, 1, 2, 3];

/// 始点（宣言の逆順）——4 枚すべてが動かなければ [`DECLARED`] へ着かない配置。
const REVERSED: [usize; GROUP_SIZE] = [3, 2, 1, 0];

// ---------------------------------------------------------------------------
// 実窓（0x0・不可視・トップレベル）
// ---------------------------------------------------------------------------

/// 0x0 の不可視の道具窓を作る（既定クラス `Static`・自前の窓手続きは要らない）。
fn create_test_window(title: PCWSTR) -> HWND {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    // SAFETY: Win32 境界。自プロセス所有の 0x0 窓を生成する。
    unsafe {
        let hinstance = GetModuleHandleW(None).expect("GetModuleHandleW");
        CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0),
            w!("Static"),
            title,
            WINDOW_STYLE(WS_POPUP.0),
            10,
            10,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .expect("CreateWindowExW should create a test window")
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

// ---------------------------------------------------------------------------
// Z の読み取りと助走
// ---------------------------------------------------------------------------

/// 与えた窓集合だけを Z の上から下へ並べて返す。
///
/// 最前面から `GW_HWNDNEXT` で降りながら、集合に属する窓だけを拾う。**生の 1 歩では
/// 測らない**——不可視の隣が間に挟まるので、隣接ではなく順序で見る。
fn relative_z_order(windows: &[HWND]) -> Vec<HWND> {
    let mut result = Vec::new();
    // SAFETY: Win32 境界。デスクトップ配下の最前面窓を得る読み取り専用 API。
    let mut cursor = unsafe { GetTopWindow(None) }.ok();
    let mut steps = 0usize;
    while let Some(hwnd) = cursor {
        if hwnd.is_invalid() {
            break;
        }
        if windows.contains(&hwnd) && !result.contains(&hwnd) {
            result.push(hwnd);
            if result.len() == windows.len() {
                break;
            }
        }
        steps += 1;
        // 走査が終わらない事態（別プロセスの窓が増え続ける等）で固まらない保険。
        if steps > 100_000 {
            break;
        }
        // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
        cursor = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();
    }
    result
}

/// 組内の並びを**添字の列**（上から下）で表す。
fn z_shape(set: &[HWND]) -> Vec<usize> {
    relative_z_order(set)
        .iter()
        .filter_map(|hwnd| set.iter().position(|w| w == hwnd))
        .collect()
}

/// Z のみを動かす素の指令（助走専用——**測る対象ではない**）。
fn zorder_command(hwnd: HWND, after: HWND) -> SetWindowPosCommand {
    SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        Some(after),
    )
}

/// 組を `order`（手前から順の添字）の並びへ揃える。
///
/// 生成順に依る初期 Z を当てにしないための助走であり、本機能の指令構築は通さない。
/// 1 本ずつの経路で流すのは、助走そのものが一括投入の正しさに依存しないためである。
fn arrange_z(set: &[HWND], order: &[usize]) {
    for pair in order.windows(2) {
        SetWindowPosCommand::enqueue(zorder_command(set[pair[1]], set[pair[0]]));
    }
    with_forced_batch_begin_failure(flush_window_pos_commands);
}

// ---------------------------------------------------------------------------
// 本機能の指令構築（判断 → 連鎖の発行）
// ---------------------------------------------------------------------------

/// 宣言順に並べた 4 枚から、本番の判断が返す連鎖（先頭と残り）を得る。
///
/// 観測は手で組む——本ファイルの始点は宣言の**逆順**なので `order_ok` が偽なのは実際の
/// 重なりと一致しており、観測から判断への繋ぎは兄弟の決定論テストが持つ。ここで
/// [`decide_group_fix`] を通すのは、先頭と残りの切り分けまで含めて**本番の経路**で
/// 指令を組ませるためである。
fn chain_for(set: &[HWND]) -> (HWND, Vec<HWND>) {
    let observation = GroupObservation {
        id: 7,
        hwnds: set.to_vec(),
        measured_front: Vec::new(),
        missing: 0,
        order_ok: false,
        scan_complete: Some(true),
    };
    match decide_group_fix(&observation) {
        GroupFixDecision::Chain { head, chain } => (head, chain),
        other => panic!("是正が要る観測なのに連鎖が返らない: {other:?}"),
    }
}

/// 連鎖を積む（本番と同じ [`enqueue_group_chain`]）。
fn issue_chain(set: &[HWND]) {
    let (head, chain) = chain_for(set);
    enqueue_group_chain(head, &chain);
}

// ---------------------------------------------------------------------------
// 実窓 4 枚での最終形（要件 1.1／1.2）
// ---------------------------------------------------------------------------

/// 連鎖を実窓へ適用すると、一括投入でも 1 本ずつでも宣言どおりの重なりに着く。
#[test]
fn a_group_chain_lands_in_the_declared_order_on_both_write_paths() {
    let _residue = drain_window_pos_commands();

    let batch_set: Vec<HWND> = (0..GROUP_SIZE)
        .map(|_| create_test_window(w!("zorder-group-order/batch")))
        .collect();
    let sequential_set: Vec<HWND> = (0..GROUP_SIZE)
        .map(|_| create_test_window(w!("zorder-group-order/sequential")))
        .collect();

    // 助走——どちらの組も宣言の逆順から始める（4 枚すべてが動かないと着かない配置）。
    arrange_z(&batch_set, &REVERSED);
    arrange_z(&sequential_set, &REVERSED);
    let batch_start = z_shape(&batch_set);
    let sequential_start = z_shape(&sequential_set);

    // ⑴ 積まれた指令そのもの——先頭を動かさないことは最終形からは見えない。
    // ここで積んだぶんは適用せずに引き取り、⑵⑶で同じ連鎖を積み直して流す。
    issue_chain(&batch_set);
    let issued = drain_window_pos_commands();
    let issued_targets: Vec<Option<usize>> = issued
        .iter()
        .map(|cmd| batch_set.iter().position(|w| *w == cmd.hwnd))
        .collect();
    let issued_anchors: Vec<Option<usize>> = issued
        .iter()
        .map(|cmd| {
            cmd.hwnd_insert_after
                .and_then(|after| batch_set.iter().position(|w| *w == after))
        })
        .collect();

    // ⑵ 一括投入（`DeferWindowPos`）。
    issue_chain(&batch_set);
    flush_window_pos_commands();
    let batched = z_shape(&batch_set);

    // ⑶ 1 本ずつの `SetWindowPos`（縮退経路）。
    issue_chain(&sequential_set);
    with_forced_batch_begin_failure(flush_window_pos_commands);
    let sequential = z_shape(&sequential_set);

    destroy(&batch_set);
    destroy(&sequential_set);
    let _residue = drain_window_pos_commands();

    // 助走の自己検査——始点が揃っていなければ以下の比較は空虚。
    assert_eq!(
        batch_start,
        REVERSED.to_vec(),
        "一括投入側の始点が宣言の逆順に揃っていない: {batch_start:?}"
    );
    assert_eq!(
        sequential_start,
        REVERSED.to_vec(),
        "1 本ずつ側の始点が宣言の逆順に揃っていない: {sequential_start:?}"
    );

    // 最終形——両経路とも宣言どおり。
    assert_eq!(
        batched,
        DECLARED.to_vec(),
        "一括投入した連鎖の最終的な重なりが宣言順と違う（要件 1.1／1.2）"
    );
    assert_eq!(
        sequential,
        DECLARED.to_vec(),
        "1 本ずつ適用した連鎖の最終的な重なりが宣言順と違う（要件 1.1／1.2）"
    );
    // 期待値との比較 2 本だけだと、両経路が同じ方向へずれたときに食い違いが見えない。
    assert_eq!(
        batched, sequential,
        "一括投入と 1 本ずつで連鎖の最終的な重なりが食い違う"
    );

    // 先頭は動かさない——指令は残り 3 枚ぶんちょうどで、対象も挿入先も構成窓だけ。
    assert_eq!(
        issued_targets,
        vec![Some(1), Some(2), Some(3)],
        "連鎖が動かした窓が「先頭を除く残り」と違う（先頭を動かす形は最終形に現れない）"
    );
    assert_eq!(
        issued_anchors,
        vec![Some(0), Some(1), Some(2)],
        "連鎖の挿入先が直前の窓になっていない"
    );
}
