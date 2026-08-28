//! 利用者の操作による重なりの変化への追随（要件 1.3／7.5）。
//!
//! 窓の位置が**外部由来**で変わったとき——利用者が窓をクリックして手前へ持ち上げた、
//! 別のアプリが前に出た——グループ機構は「是正が要るかもしれない」という印を立て、
//! 次の画面更新を促す。**変化の内訳（`WINDOWPOS` のフラグや挿入先）は一切読まない**。
//! 実際に是正が要るかどうかを決めるのは維持系の観測であり、要らなければ同値ガードが
//! 指令 0 本で吸収する（design「トリガ 2 点」）。
//!
//! # 何を固定するか
//!
//! 1. **判断**——引くか引かないかを純関数 [`wants_group_follow`](super::wants_group_follow)
//!    として切り出し、真理値表そのものを固定する（項を 1 つ落とす変異がここで赤くなる）。
//! 2. **状態**——印は立つ側にしか動かない。トリガが印を降ろす経路を作らない。
//! 3. **結線**——その判断が**本番の受理経路に結びついている**ことを本文の字面で固定する
//!    （呼出はちょうど 1 つ・`is_echo` を読んだ後・`WINDOWPOS` を読む前・巡を回す前）。
//! 4. **最終形**——実窓 4 枚で、1 枚を手前へ持ち上げた後、続く巡で宣言どおりの相対順へ
//!    戻ること。トリガを引かない対照を同じテストの中に置く。
//!
//! 判断だけを試験しても「呼ばれていること」は誰も見ていない（task 4.2 の教訓＝単体の檻は
//! 結線の檻の代わりにならない）。位置まで押さえるのは、印を立てる場所がずれても**振る舞い
//! だけを読む檻は 1 本も赤くならない**からである（task 3.1 の教訓）。
//!
//! # なぜ起床の旗そのものを読まないのか
//!
//! 旗（`tick_wake`）はプロセスに 1 組しかなく、`wintf` の検査は並列に走る。しかも
//! `ZORDER` を立てるのは本トリガだけではない（既存のペア機構と維持系が同じビットを
//! 立て、それらの検査は共有の錠を取らない）。ゆえに共有の旗の上では「立っていない」も
//! 「立っている」も証拠にならない——どちらも走行のたびに結論が変わりうる
//! （`tick_wake_tests.rs:15-17`・task 4.3 が敷いた作法）。よって旗は字面で押さえる。

use bevy_ecs::prelude::{Entity, World};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GW_HWNDNEXT, GetTopWindow, GetWindow, HWND_TOP, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, WINDOW_EX_STYLE, WINDOW_STYLE, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use super::{note_external_zorder_change, wants_group_follow};
use crate::ecs::window::{
    FrontScan, GroupProbe, SetWindowPosCommand, ZOrderGroupSpec, ZOrderGroups,
    drain_window_pos_commands, flush_window_pos_commands, measure_windows_in_front,
};

// ===========================================================================
// ⑴ 判断——真理値表
// ===========================================================================

/// トリガを引くのは「外部由来」かつ「グループが 1 本でもある」ときだけである。
///
/// 4 通りすべてを置くのは、項を 1 つ落とす変異（こだまを見ない／宣言の有無を見ない）が
/// 片側の入力しか置かない檻では素通りするからである（task 1.2 の教訓）。
#[test]
fn the_follow_trigger_fires_only_for_external_changes_while_a_group_exists() {
    assert!(
        wants_group_follow(false, true),
        "外部由来の変化でグループが在るのに追随しない（利用者の操作で崩れたまま戻らない・要件 1.3）"
    );
    assert!(
        !wants_group_follow(true, true),
        "自分が出した指令のこだまで追随している（是正→こだま→再検証の輪が回り続ける）"
    );
    assert!(
        !wants_group_follow(false, false),
        "グループが 1 本も無いのに追随している（既定状態＝非強制の巡で維持系が起きる・要件 6.4）"
    );
    assert!(
        !wants_group_follow(true, false),
        "こだまでも宣言でもないのに追随している"
    );
}

// ===========================================================================
// ⑵ 状態——印は立つ側にしか動かない
// ===========================================================================

/// 実体を n 個作る（値としての `Entity` が欲しいだけ）。
fn entities(n: usize) -> Vec<Entity> {
    let mut world = World::new();
    (0..n).map(|_| world.spawn_empty().id()).collect()
}

/// グループを 1 本だけ持つ受け口を組む（印は降りた状態から始める）。
fn groups_with(id: u32, members: &[Entity]) -> ZOrderGroups {
    let mut groups = ZOrderGroups::default();
    groups.groups.push(ZOrderGroupSpec {
        id,
        members: members.to_vec(),
    });
    groups
}

/// 外部由来の変化は印を立て、こだまと既定状態は立てない。
#[test]
fn an_external_change_raises_the_mark_and_an_echo_does_not() {
    let members = entities(2);

    // 外部由来——立つ。
    let mut declared = groups_with(51, &members);
    assert!(
        !declared.pending,
        "檻の前提が崩れている（受け口は印の降りた状態から始まる）"
    );
    note_external_zorder_change(&mut declared, false);
    assert!(
        declared.pending,
        "外部由来の変化で印が立たない（利用者が持ち上げた重なりが戻らない・要件 1.3）"
    );

    // こだま——立たない。
    let mut echoed = groups_with(52, &members);
    note_external_zorder_change(&mut echoed, true);
    assert!(
        !echoed.pending,
        "こだまで印が立っている（自分の是正が次の是正を呼ぶ）"
    );

    // 既定状態（宣言が 1 本も無い）——立たない。
    let mut bare = ZOrderGroups::default();
    note_external_zorder_change(&mut bare, false);
    assert!(
        !bare.pending,
        "宣言が 1 本も無いのに印が立っている（既定状態の挙動が導入前と変わる・要件 6.4）"
    );
}

/// トリガは印を降ろさない——引かなかった巡も、立っている印はそのまま残る。
///
/// 安全側の不変条件は「検証待ちがある ⇒ 印が立っている」であり、印を降ろす経路は
/// 維持系の⑤（維持対象の全グループが成立した巡）ただ 1 つである。トリガが
/// `pending = false` を書くと、是正の要求が**誰にも記録されずに**消える。
#[test]
fn the_trigger_never_lowers_a_raised_mark() {
    let members = entities(2);

    // こだま——引かないが、既に立っている印は落とさない。
    let mut echoed = groups_with(61, &members);
    echoed.pending = true;
    note_external_zorder_change(&mut echoed, true);
    assert!(
        echoed.pending,
        "こだまの巡に印が落ちている（未処理の是正要求が記録も無く消える）"
    );

    // 宣言が無い巡も同じ（印を降ろすのは維持系の⑤だけ）。
    let mut bare = ZOrderGroups::default();
    bare.pending = true;
    note_external_zorder_change(&mut bare, false);
    assert!(
        bare.pending,
        "宣言の無い巡にトリガが印を落としている（印を降ろす口はここではない）"
    );
}

// ===========================================================================
// ⑶ 結線——判断が本番の受理経路に結びついている
// ===========================================================================

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

/// 本文に 1 度だけ現れるはずの字面の位置（見つからなければその場で落とす）。
fn index_of(source: &str, needle: &str) -> usize {
    source
        .find(needle)
        .unwrap_or_else(|| panic!("本番の字面 `{needle}` が見つからない（檻の前提が崩れている）"))
}

/// トリガの呼出はちょうど 1 つで、こだま判定の後・`WINDOWPOS` を読む前・巡を回す前に在る。
///
/// 押さえるのは 5 点である。
///
/// 1. 呼出はちょうど 1 つ——別経路から二重に叩く形を塞ぐ。
/// 2. 判断と 2 つの作用（印・旗）が 1 つの `if` に閉じている。無条件に立てれば既定状態の
///    静穏が死に、片方だけ立てれば「印はあるのに画面更新が省略される」形になる。
/// 3. 位置——`is_echo` を読んだ**後**であり、`WM_WINDOWPOSCHANGED` の本体の中に在る。
/// 4. 位置——巡を回す（`try_tick_on_vsync`）**前**である。後ろへ移すと、印が立ってから
///    実際に維持系が回るまで画面更新 1 回ぶん遅れる。**この遅れは最終形からは見えない**
///    （次の巡で直るため）ので、振る舞いだけを読む檻は 1 本も赤くならない。
/// 5. 変化の内訳を読まない——`WINDOWPOS` を読む行より前に置くことで、フラグや挿入先を
///    見て判断する余地を構造から断つ（design「`wp.flags`／`hwndInsertAfter` は解析しない」）。
#[test]
fn the_follow_trigger_is_wired_once_before_the_tick_and_reads_no_windowpos() {
    let code = code_only(include_str!("window_pos.rs"));

    assert!(
        code.contains("fn wants_group_follow("),
        "説明文を落とす処理が本文まで落としている"
    );

    // 判断もトリガもモジュール私設である。`pub(crate)` へ広げると、この檻が数える
    // 「呼出はちょうど 1 つ」は `window_pos.rs` しか走査していないので、他モジュールに
    // 生えた第 2 の本番経路を 1 本も捕まえられない。可視性そのものを錠に使う。
    assert!(
        code.contains("\nfn wants_group_follow("),
        "判断がモジュール外から呼べる（判断を迂回しない第 2 の生産者を作れてしまう）"
    );
    assert!(
        code.contains("\nfn note_external_zorder_change("),
        "トリガがモジュール外から呼べる（要件 1.3 の生産者がちょうど 1 つでなくなる）"
    );
    assert_eq!(
        code.matches("wants_group_follow(").count(),
        2,
        "判断の定義と呼出が 1 対 1 でない（判断を迂回する第 2 の経路がある疑い）"
    );
    assert_eq!(
        code.matches("note_external_zorder_change(").count(),
        2,
        "トリガの定義と呼出が 1 対 1 でない（要件 1.3 の生産者はちょうど 1 つ）"
    );

    // 判断と 2 つの作用が 1 つの `if` に閉じている。
    assert!(
        squeeze(&code).contains(
            "if wants_group_follow(is_echo, !groups.groups.is_empty()) { groups.pending = true; crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::ZORDER); }"
        ),
        "印と旗が判断に守られていない（無条件なら既定状態の静穏が死に、片方だけなら要求が足踏みする）"
    );
    assert_eq!(
        code.matches("tick_wake::mark(").count(),
        1,
        "旗を立てる呼出がちょうど 1 つでない"
    );
    assert_eq!(
        code.matches("groups.pending = true;").count(),
        1,
        "印を立てる書込がちょうど 1 つでない"
    );

    // 印を降ろす経路を作らない（安全側の不変条件＝検証待ちがある ⇒ 印が立っている）。
    assert!(
        !code.contains("pending = false"),
        "トリガ側に印を降ろす書込がある（是正の要求が記録も無く消える）"
    );

    // 位置——`is_echo` の後、`WINDOWPOS` を読む前、巡を回す前、そして受理経路の中。
    let handler_at = index_of(&code, "pub(super) fn WM_WINDOWPOSCHANGED(");
    let echo_at = index_of(
        &code,
        "let is_echo = crate::ecs::window::is_self_initiated();",
    );
    let call_at = index_of(&code, "note_external_zorder_change(&mut groups, is_echo);");
    let windowpos_at = index_of(&code, "let windowpos = lparam.0 as *const WINDOWPOS;");
    let tick_at = index_of(&code, "let _ = world.try_tick_on_vsync();");
    let dpi_at = index_of(&code, "pub(super) fn WM_DPICHANGED(");
    assert!(
        handler_at < echo_at && echo_at < call_at,
        "トリガがこだま判定より前に在る（受理={handler_at}・こだま={echo_at}・トリガ={call_at}）"
    );
    assert!(
        call_at < windowpos_at,
        "トリガが `WINDOWPOS` を読んだ後に在る（変化の内訳を見て判断する余地が残る・トリガ={call_at}・読み取り={windowpos_at}）"
    );
    assert!(
        call_at < tick_at,
        "トリガが巡を回した後に在る（印が立ってから維持系が回るまで画面更新 1 回ぶん遅れる・トリガ={call_at}・巡={tick_at}）"
    );
    assert!(
        call_at < dpi_at,
        "トリガが `WM_WINDOWPOSCHANGED` の本体の外に在る（トリガ={call_at}・次の関数={dpi_at}）"
    );

    // 変化の内訳を読む字面がそもそも 1 つも無い（design の明示的な禁止）。
    assert!(
        !code.contains("wp.flags"),
        "`WINDOWPOS` のフラグを読んでいる（design が解析を禁じている）"
    );
    assert!(
        !code.contains("hwndInsertAfter") && !code.contains("hwnd_insert_after"),
        "`WINDOWPOS` の挿入先を読んでいる（design が解析を禁じている）"
    );
}

// ===========================================================================
// ⑷ 最終形——実窓 4 枚で、手前へ持ち上げた 1 枚が続く巡で戻る
// ===========================================================================

/// 1 グループの窓数。
const GROUP_SIZE: usize = 4;

/// 宣言どおりの相対順（手前から順の添字）。
const DECLARED: [usize; GROUP_SIZE] = [0, 1, 2, 3];

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

/// デスクトップを最前面から辿り、与えた窓集合だけを上から下へ並べて返す。
///
/// **生の 1 歩では測らない**——不可視の隣が間に挟まるので、隣接ではなく順序で見る
/// （既定の IME 窓の前例）。`zorder_group_order_tests.rs` の同名の助手と同じ形である。
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

/// 組を `order`（手前から順の添字）の並びへ揃える（助走——本機能の経路は通さない）。
fn arrange_z(set: &[HWND], order: &[usize]) {
    for pair in order.windows(2) {
        SetWindowPosCommand::enqueue(zorder_command(set[pair[1]], set[pair[0]]));
    }
    flush_window_pos_commands();
}

/// 利用者の操作を模して 1 枚を最前面へ持ち上げる（`HWND_TOP`——常時最前面の帯へは入れない）。
fn raise_to_top(hwnd: HWND) {
    SetWindowPosCommand::enqueue(SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        Some(HWND_TOP),
    ));
    flush_window_pos_commands();
}

/// 実窓を相手にする実測の口。
///
/// # なぜ前面走査を差し替えるのか
///
/// 本番の走査（[`measure_windows_in_front`]）は**可視の窓だけ**を列に入れる。ここで使う
/// 道具窓は `WS_VISIBLE` を持たない 0x0 の窓なので、本番の走査からはどれも見えず、
/// 相対順の判定材料が空になる——差し替えないと、どう並べても「崩れている」としか読めない。
/// 可視の窓にすれば本番の走査を通せるが、利用者のデスクトップに実物の窓を出し、活性化で
/// 並びが動く余地を作ることになる。よってここは**同じデスクトップの同じ Z 順**を、可視の
/// 濾過だけ外して読む。
///
/// # 本番との差は 3 つある
///
/// 1. **可視の濾過が無い**——上記のとおり、`WS_VISIBLE` を持たない道具窓を読むため。
/// 2. **`windows` に載せるのはグループのメンバーだけ**——本番は出会った窓を種別で選ばずに
///    載せるが、ここは同じデスクトップに居合わせた無関係の窓（他アプリ・既定 IME 窓）を
///    落とす。相対順の判定はメンバーどうしの前後関係しか見ないので結論は変わらない。
/// 3. **`reached_top` は「起点に出会った」で立てる**——本番は最前面まで走査し切れたかを
///    表すが、ここは起点より手前を数え終えた時点で走査を打ち切るため、その打ち切りを
///    「起点までは正しく数え切った」の意味で立てている。起点より奥は判定に使わない。
///
/// いずれもこのテストの合否を変えないが、**本番の走査そのものを試したことにはならない**。
/// ⑴ が実在の差であること——本番の走査ではこの道具窓が 1 枚も見えないこと——だけは
/// テスト本体が本番の走査（[`measure_windows_in_front`]）を 1 度呼んで自己検査する。
/// ⑵⑶ に対応する自己検査は無い（判定に使わない部分なので置いていない）。
///
/// 実体からハンドルを引く側（[`GroupProbe::resolve`]）は差し替えない。
struct RealWindowProbe {
    handles: Vec<(Entity, HWND)>,
}

impl RealWindowProbe {
    fn new(members: &[Entity], hwnds: &[HWND]) -> Self {
        Self {
            handles: members.iter().copied().zip(hwnds.iter().copied()).collect(),
        }
    }
}

impl GroupProbe for RealWindowProbe {
    fn resolve(&self, entity: Entity) -> Option<HWND> {
        self.handles
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, hwnd)| *hwnd)
    }

    fn scan_in_front(&self, hwnd: HWND) -> FrontScan {
        let members: Vec<HWND> = self.handles.iter().map(|(_, h)| *h).collect();
        let mut ahead = Vec::new();
        let mut reached_top = false;
        // SAFETY: Win32 境界。デスクトップ配下の最前面窓を得る読み取り専用 API。
        let mut cursor = unsafe { GetTopWindow(None) }.ok();
        let mut steps = 0usize;
        while let Some(seen) = cursor {
            if seen.is_invalid() {
                break;
            }
            if seen == hwnd {
                reached_top = true;
                break;
            }
            if members.contains(&seen) {
                ahead.push(seen);
            }
            steps += 1;
            if steps > 100_000 {
                break;
            }
            // SAFETY: Win32 境界。窓ハンドルに対する読み取り専用の走査。
            cursor = unsafe { GetWindow(seen, GW_HWNDNEXT) }.ok();
        }
        // 走査は上から下へ進んだので、近い順（起点へ近い方から）へ直す。
        ahead.reverse();
        FrontScan {
            windows: ahead,
            reached_top,
        }
    }
}

/// 1 巡ぶん維持系を回し、積まれた指令を実際に窓へ書き込んで、その本数を返す。
fn run_pass_and_flush(groups: &mut ZOrderGroups, probe: &RealWindowProbe) -> usize {
    crate::ecs::window::run_group_maintenance_pass(groups, false, probe);
    let issued = drain_window_pos_commands();
    let count = issued.len();
    for cmd in issued {
        SetWindowPosCommand::enqueue(cmd);
    }
    flush_window_pos_commands();
    count
}

/// 手前へ持ち上げられた窓は、追随トリガの後の巡で宣言どおりの相対順へ戻る（要件 1.3）。
///
/// 対照を 2 つ同じテストの中に置く。
///
/// - **トリガを引かない巡**——印が立たないので維持系は観測すらせず、崩れたまま残る。これが
///   無いと「持ち上げても勝手に戻る（トリガは飾り）」形が緑で通る。
/// - **静穏の巡**——宣言どおりに並んでいる間は指令が 1 本も出ない（同値ガード）。
#[test]
fn a_window_raised_by_the_user_returns_to_the_declared_order_on_the_following_passes() {
    let _residue = drain_window_pos_commands();

    let set: Vec<HWND> = (0..GROUP_SIZE)
        .map(|_| create_test_window(w!("window-pos/zorder-group-follow")))
        .collect();
    let members = entities(GROUP_SIZE);
    let probe = RealWindowProbe::new(&members, &set);
    let mut groups = groups_with(71, &members);

    // 助走——宣言どおりに並べる。
    arrange_z(&set, &DECLARED);
    let start = z_shape(&set);

    // 自己検査——本番の走査は不可視のこれらを 1 枚も拾わない（差し替えの根拠）。
    let production_scan = measure_windows_in_front(set[GROUP_SIZE - 1]);
    let production_saw_members = production_scan
        .windows
        .iter()
        .any(|seen| set.contains(seen));

    // ⑴ 静穏——印が降りている巡は観測すらしない。
    let quiet = run_pass_and_flush(&mut groups, &probe);
    let quiet_shape = z_shape(&set);

    // ⑵ 利用者の操作——最も奥の 1 枚を最前面へ持ち上げる。
    raise_to_top(set[GROUP_SIZE - 1]);
    let raised = z_shape(&set);

    // ⑶ 対照——トリガを引かなければ、崩れたままである。
    let untriggered = run_pass_and_flush(&mut groups, &probe);
    let untriggered_shape = z_shape(&set);

    // ⑷ 本番の追随トリガ（外部由来＝こだまではない）。
    note_external_zorder_change(&mut groups, false);
    let marked = groups.pending;

    // ⑸ 続く巡——連鎖が出て、書き込まれる。
    let fixing = run_pass_and_flush(&mut groups, &probe);
    let fixed_shape = z_shape(&set);

    // ⑹ さらに続く巡——照合が成立し、印が降り、指令はもう出ない。
    let settling = run_pass_and_flush(&mut groups, &probe);
    let settled_pending = groups.pending;
    let settled_shape = z_shape(&set);

    destroy(&set);
    let _residue = drain_window_pos_commands();

    // 助走の自己検査——始点が揃っていなければ以下の比較は空虚。
    assert_eq!(
        start,
        DECLARED.to_vec(),
        "始点が宣言どおりに揃っていない: {start:?}"
    );
    assert!(
        !production_saw_members,
        "本番の走査が不可視の道具窓を拾った（走査を差し替える根拠が失われている——本番の走査で組み直すこと）"
    );

    // ⑴ 静穏。
    assert_eq!(quiet, 0, "印の降りた巡に指令が出ている（既定の静穏が死ぬ）");
    assert_eq!(
        quiet_shape,
        DECLARED.to_vec(),
        "誰も動かしていない巡に並びが変わった: {quiet_shape:?}"
    );

    // ⑵ 持ち上げが効いている（効いていなければ以降は空虚）。
    assert_eq!(
        raised[0],
        GROUP_SIZE - 1,
        "持ち上げた窓が最前面に居ない（檻の前提が崩れている）: {raised:?}"
    );
    assert_ne!(
        raised,
        DECLARED.to_vec(),
        "持ち上げても並びが宣言どおりのまま（檻の前提が崩れている）"
    );

    // ⑶ 対照——トリガが無ければ何も起きない。
    assert_eq!(
        untriggered, 0,
        "トリガを引いていない巡に是正が出ている（門が開きっぱなし）"
    );
    assert_eq!(
        untriggered_shape, raised,
        "トリガを引いていないのに並びが戻った（トリガが是正の必要条件になっていない）"
    );

    // ⑷⑸⑹ トリガ → 是正 → 成立。
    assert!(
        marked,
        "外部由来の変化で印が立たない（利用者の操作に追随しない・要件 1.3）"
    );
    assert_eq!(
        fixing,
        GROUP_SIZE - 1,
        "是正の指令が「先頭を除く残り」ちょうどでない"
    );
    assert_eq!(
        fixed_shape,
        DECLARED.to_vec(),
        "続く巡で宣言どおりの相対順へ戻らない（要件 1.3）: {fixed_shape:?}"
    );
    assert_eq!(
        settling, 0,
        "成立した巡に指令が出ている（同値ガードが効いていない）"
    );
    assert!(
        !settled_pending,
        "成立した巡に印が降りていない（起床の旗が立ち続ける）"
    );
    assert_eq!(
        settled_shape,
        DECLARED.to_vec(),
        "成立した後に並びが崩れた: {settled_shape:?}"
    );
}

/// 組ごと別のアプリの後ろへ回っても、グループ内の相対順は保たれ、是正は 1 本も出ない（要件 7.5）。
///
/// 「別のアプリが前に出た」は、グループに属さない窓を 4 枚より手前へ置くことで模す。
/// このとき変化の内訳を読まないトリガは当然引かれる（余分に 1 巡ぶん検証が走る）が、
/// 相対順は部分列として保たれているので、同値ガードが指令 0 本で吸収する——design の
/// 「位置変化でも余分に検証が走るが同値ガードが 0 本で吸収」がここで実測される。
#[test]
fn the_internal_order_survives_the_whole_group_going_behind_another_window() {
    let _residue = drain_window_pos_commands();

    let set: Vec<HWND> = (0..GROUP_SIZE)
        .map(|_| create_test_window(w!("window-pos/zorder-group-behind")))
        .collect();
    let intruder = create_test_window(w!("window-pos/zorder-group-intruder"));
    let members = entities(GROUP_SIZE);
    let probe = RealWindowProbe::new(&members, &set);
    let mut groups = groups_with(81, &members);

    arrange_z(&set, &DECLARED);
    let start = z_shape(&set);

    // 別のアプリが前に出た——4 枚は揃って後ろへ回る。
    raise_to_top(intruder);
    let mut with_intruder = set.clone();
    with_intruder.push(intruder);
    let layered = z_shape(&with_intruder);
    let behind_shape = z_shape(&set);

    // 内訳を読まないトリガは引かれる。
    note_external_zorder_change(&mut groups, false);
    let marked = groups.pending;

    // 続く巡——相対順は保たれているので指令は 1 本も出ず、印は降りる。
    let issued = run_pass_and_flush(&mut groups, &probe);
    let after_pending = groups.pending;
    let after_shape = z_shape(&set);
    let after_layered = z_shape(&with_intruder);

    destroy(&with_intruder);
    let _residue = drain_window_pos_commands();

    assert_eq!(
        start,
        DECLARED.to_vec(),
        "始点が宣言どおりに揃っていない: {start:?}"
    );
    assert_eq!(
        layered[0], GROUP_SIZE,
        "割り込んだ窓が最前面に居ない（檻の前提が崩れている）: {layered:?}"
    );
    assert_eq!(
        behind_shape,
        DECLARED.to_vec(),
        "後ろへ回った時点で内側の相対順が既に崩れている: {behind_shape:?}"
    );
    assert!(marked, "外部由来の変化で印が立たない（要件 1.3）");
    assert_eq!(
        issued, 0,
        "相対順が保たれているのに是正が出ている（同値ガードが吸収していない・要件 7.5）"
    );
    assert!(
        !after_pending,
        "成立した巡に印が降りていない（起床の旗が立ち続ける）"
    );
    assert_eq!(
        after_shape,
        DECLARED.to_vec(),
        "後ろへ回った組の内側の相対順が保たれていない（要件 7.5）: {after_shape:?}"
    );
    assert_eq!(
        after_layered[0], GROUP_SIZE,
        "グループに属さない窓が動かされた（要件 2.5）: {after_layered:?}"
    );
}
