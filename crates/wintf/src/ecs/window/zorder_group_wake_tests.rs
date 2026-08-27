//! ⑥ 起床の印——是正が適用されるまで「次の画面更新も回してほしい」と言い続ける（要件 7.4）。
//!
//! 表示に変化が無い巡は省略され得る。省略の向こうで是正が足踏みしないよう、維持系は
//! **是正が終わっていない間**だけ毎巡ひとつ印を立てる。逆に終わったのに立て続ければ、
//! 省略の仕組みそのものが実質無効になる。**どちらへ倒れても実害があるので、本ファイルは
//! 両側を挟む。**
//!
//! # なぜ旗そのものを読まないのか
//!
//! 旗（`tick_wake`）はプロセスに 1 組しかなく、`wintf` の検査は並列に走る。しかも
//! `ZORDER` を立てるのは本系統だけではない——既存のペア機構
//! （[`zorder_pair_maintain`](crate::ecs::window::zorder_pair_maintain)）が同じビットを
//! 立て、その検査は共有の錠（`TICK_WAKE_TEST_LOCK`）を取らない。ゆえに共有の旗の上では
//!
//! - 「立っていない」は主張できない（他人が立てた旗で赤くなる）
//! - 「立っている」も証拠にならない（他人が立てた旗で緑になる＝変異体を素通りさせる）
//!
//! ——どちらも走行のたびに結論が変わりうる形であり、要件 10.3（単体走行と全体走行が
//! 一致する）を満たさない。よって本ファイルは 2 段で挟む。
//!
//! 1. **判断**——「まだ促すか」を純関数 [`wants_wake`](super::wants_wake) として切り出し、
//!    真理値表そのものを固定する（項を 1 つ落とす変異がここで赤くなる）。
//! 2. **結線**——その判断が**本番の巡の末尾で旗に結びついている**ことを本文の字面で
//!    固定する（呼出はちょうど 1 つ・旗は `ZORDER`・①〜⑤を回した後・脱出路が無い側に
//!    置く）。task 3.1 の `t_zcs14` と同じ作法である。
//!
//! 2 段が要るのは、判断だけを試験しても「呼ばれていること」は誰も見ていないからである
//! （task 4.2 の教訓＝単体の檻は結線の檻の代わりにならない）。

use super::zorder_group_maintain_tests::{
    FakeProbe, clear_queue, entities, fake_hwnd, groups_with, issued_targets,
};
use super::{run_group_maintenance_pass, wants_wake};
use crate::ecs::window::{ZOrderGroups, drain_window_pos_commands};

use bevy_ecs::prelude::Entity;
use windows::Win32::Foundation::HWND;

/// 巡の終わりの状態から「次の画面更新を促すか」を読む（本番の⑥と同じ判断を通す）。
fn asks_for_next_tick(groups: &ZOrderGroups) -> bool {
    wants_wake(groups.pending, groups.has_verify())
}

/// 手前・奥の 2 枚組を 1 本組む（実体とハンドル）。
fn two_window_group(base: usize) -> (Vec<Entity>, [HWND; 2]) {
    let members = entities(2);
    let hwnds = [fake_hwnd(base + 1), fake_hwnd(base + 2)];
    (members, hwnds)
}

// ===========================================================================
// ⑥ の判断——真理値表
// ===========================================================================

/// 促すのは「是正が終わっていない」間だけである（4 通りすべてを固定する）。
///
/// 項を 1 つ落とす変異（`pending` だけを見る／検証待ちだけを見る）は、片側の入力しか
/// 置かない檻では素通りする。4 行そろえてあるのはそのためである。
#[test]
fn the_wake_is_asked_exactly_while_a_correction_is_outstanding() {
    assert!(
        !wants_wake(false, false),
        "印も検証待ちも無い巡は促さない（促し続けると表示に変化の無い巡を省く仕組みが死ぬ）"
    );
    assert!(
        wants_wake(true, false),
        "是正が要るかもしれない印が残る間は促す（要件 7.4）"
    );
    assert!(
        wants_wake(false, true),
        "出した連鎖の照合が済んでいない間は促す（照合は次の巡でしか採れない）"
    );
    assert!(wants_wake(true, true), "印も検証待ちも残る巡は当然促す");
}

// ===========================================================================
// 表示に変化が無い状況——是正が適用されるまで促し続け、適用後に促さなくなる
// ===========================================================================

/// 同じ重なりが続く巡を並べても、是正が適用されるまで毎巡促し、適用された巡で止まる。
///
/// 「表示に変化が無い」は、実測の口が**毎巡まったく同じ値**を返すことで模す（巡ごとに
/// 崩れ方が変わるなら、それは表示が変化しているということである）。是正が実際に効いた
/// ことは、実測の口を「直った側」に差し替えて表す——指令の書込は巡の後であり、効いたか
/// どうかは次の巡の実測でしか分からないからである。
#[test]
fn a_quiet_display_is_asked_every_pass_until_the_fix_lands_and_not_after() {
    let (members, hwnds) = two_window_group(0xB00);
    // 崩れたまま動かない表示（末尾の手前に構成窓が 1 枚も居ない）
    let broken = FakeProbe::new()
        .with_handles(&members, &hwnds)
        .with_front(hwnds[1], &[], true);
    // 是正が効いた後の表示（末尾の手前に先頭の窓が居る）
    let fixed =
        FakeProbe::new()
            .with_handles(&members, &hwnds)
            .with_front(hwnds[1], &[hwnds[0]], true);

    let mut groups = groups_with(11, &members);
    clear_queue();

    // 1 巡目——連鎖を出し、照合待ちを預ける。促す。
    run_group_maintenance_pass(&mut groups, false, &broken);
    assert_eq!(
        issued_targets(&drain_window_pos_commands()),
        vec![hwnds[1]],
        "崩れている巡に是正の指令が出ていない（檻の前提が崩れている）"
    );
    assert!(
        asks_for_next_tick(&groups),
        "是正を出した巡は促す（書込は巡の後・照合は次の巡）"
    );

    // 2 巡目——表示は 1 画素も変わっていない。照合は不一致になり、また出す。促す。
    run_group_maintenance_pass(&mut groups, false, &broken);
    assert_eq!(
        issued_targets(&drain_window_pos_commands()),
        vec![hwnds[1]],
        "変化の無い巡で是正が止まっている（足踏みしたまま促すだけになる）"
    );
    assert!(
        asks_for_next_tick(&groups),
        "是正が適用されていないのに促さなくなった（省略された画面更新の向こうで足踏みする）"
    );

    // 3 巡目——是正が効いた。照合が成立し、印が降り、預かりも無くなる。促さない。
    run_group_maintenance_pass(&mut groups, false, &fixed);
    assert!(
        drain_window_pos_commands().is_empty(),
        "成立した巡に指令が出ている（同値ガードが効いていない）"
    );
    assert!(
        !groups.pending,
        "成立した巡に印が降りていない（檻の前提が崩れている）"
    );
    assert!(
        !groups.has_verify(),
        "成立した巡に照合待ちが残っている（檻の前提が崩れている）"
    );
    assert!(
        !asks_for_next_tick(&groups),
        "是正が適用された後も促し続けている（表示に変化の無い巡を省く仕組みが死ぬ）"
    );

    // 4 巡目——静穏。何度回しても促さない。
    run_group_maintenance_pass(&mut groups, false, &fixed);
    assert!(
        drain_window_pos_commands().is_empty(),
        "静穏の巡に指令が出ている"
    );
    assert!(!asks_for_next_tick(&groups), "静穏の巡で促している");
}

// ===========================================================================
// 見送った巡も促す——止まったまま静かになる経路を作らない
// ===========================================================================

/// 同じ巡にペア機構が是正を出していて見送った巡（③）も、次の画面更新を促す。
///
/// 見送りは断念ではなく、印は残る。ここで促さないと、表示に変化の無いまま次の巡が
/// 省略され、要求だけが残って誰も動かないという形になりうる。対照として、見送りの理由が
/// 無い巡（＝実際に是正を出す巡）も同じく促すことを併置してある——「常に真」を見分ける
/// ためではなく、見送りが**促す理由を消していない**ことを示すためである。
#[test]
fn a_pass_deferred_to_the_pair_mechanism_still_asks_for_the_next_tick() {
    let (members, hwnds) = two_window_group(0xC00);
    let broken = FakeProbe::new()
        .with_handles(&members, &hwnds)
        .with_front(hwnds[1], &[], true);

    let mut groups = groups_with(21, &members);
    clear_queue();

    // ペア機構が同じ巡に是正を出した——こちらは 1 本も出さずに見送る。
    run_group_maintenance_pass(&mut groups, true, &broken);
    assert!(
        drain_window_pos_commands().is_empty(),
        "見送った巡に指令が出ている（檻の前提が崩れている）"
    );
    assert!(
        groups.pending,
        "見送った巡に印が落ちている（次の巡でやり直せない）"
    );
    assert!(
        asks_for_next_tick(&groups),
        "見送った巡で促していない（省略された画面更新の向こうで要求が足踏みする）"
    );

    // 対照: ペア機構が出していない巡は実際に是正を出し、やはり促す。
    run_group_maintenance_pass(&mut groups, false, &broken);
    assert_eq!(
        issued_targets(&drain_window_pos_commands()),
        vec![hwnds[1]],
        "見送りの理由が無い巡でも是正が出ていない"
    );
    assert!(asks_for_next_tick(&groups), "是正を出した巡で促していない");
}

// ===========================================================================
// 既定状態＝非強制——グループが 1 本も無ければ何も促さない（要件 6.1／6.4）
// ===========================================================================

/// グループが 1 本も宣言されていなければ、促す理由は一度も立たない。
///
/// 印が最初から無い巡と、印だけが立った巡（追随トリガの取りこぼしなど）の両方を歩く。
/// 対照として、**宣言があって崩れている**巡では促すことを同じテストの中に置く——
/// 「そもそも何も動いていないから促さない」形と区別するためである。
#[test]
fn nothing_is_asked_while_no_group_is_declared() {
    let quiet = FakeProbe::new();
    clear_queue();

    // 受け口はあるが宣言も印も無い。
    let mut groups = ZOrderGroups::default();
    run_group_maintenance_pass(&mut groups, false, &quiet);
    assert!(
        drain_window_pos_commands().is_empty(),
        "宣言の無い巡に指令が出ている"
    );
    assert!(
        !asks_for_next_tick(&groups),
        "宣言が 1 本も無い巡で促している（既定状態の挙動が導入前と変わる・要件 6.4）"
    );

    // 印だけが立っても、宣言が無ければその巡で降りて、次からは促さない。
    groups.pending = true;
    run_group_maintenance_pass(&mut groups, false, &quiet);
    assert!(
        !groups.pending,
        "宣言が無いのに印が残っている（檻の前提が崩れている）"
    );
    assert!(
        !asks_for_next_tick(&groups),
        "宣言が 1 本も無いのに促し続けている"
    );

    // 対照: 宣言があって崩れていれば促す。
    let (members, hwnds) = two_window_group(0xD00);
    let broken = FakeProbe::new()
        .with_handles(&members, &hwnds)
        .with_front(hwnds[1], &[], true);
    let mut declared = groups_with(31, &members);
    run_group_maintenance_pass(&mut declared, false, &broken);
    let _ = drain_window_pos_commands();
    assert!(
        asks_for_next_tick(&declared),
        "宣言があって崩れている巡でも促していない（促す経路そのものが死んでいる疑い）"
    );
}

// ===========================================================================
// 結線——判断が本番の巡の末尾で旗に結びついている
// ===========================================================================

/// 旗を立てる呼出はちょうど 1 つで、⑥の判断に守られ、①〜⑤の後・脱出路の無い側に在る。
///
/// 判断（真理値表）だけを固定しても、それが**呼ばれている**ことは誰も見ていない
/// （task 4.2 の教訓）。旗はプロセス共有で並列走行するため動的には読めない（本ファイル
/// 冒頭）ので、ここは本文の字面で押さえる。押さえるのは 5 点である。
///
/// 1. 呼出はちょうど 1 つ——後ろに残したまま先頭でも叩く形を塞ぐ。
/// 2. 立てるのは重なりの旗（`ZORDER`）。
/// 3. ⑥の判断が旗の直前に在る——項を落とせば真理値表が、外せばここが赤くなる。
/// 4. 位置——①〜⑤を回した**後**であり、かつ早期の脱出が 1 つも無い側に在る。②の門と
///    ③の調停は `return` で抜けるので、旗をその内側へ移すと見送った巡が促さなくなる。
/// 5. 迂回路が無い——①〜⑤だけを回す側（旗を立てない双子）を呼べるのは⑥の内側だけで
///    あり、本番の system は旗の立つ入口を通る。①〜⑤を切り出した以上、**本番の呼び先を
///    1 語替えるだけで要件 7.4 が本番だけで死ぬ**経路がここで生まれている。判断も位置も
///    正しいままなので、他の 4 点はその書き換えを 1 本も捕まえられない。
#[test]
fn the_wake_is_raised_once_after_the_five_steps_and_after_every_exit() {
    let code = code_only(include_str!("zorder_group_maintain.rs"));

    assert_eq!(
        code.matches("tick_wake::mark(").count(),
        1,
        "旗を立てる呼出はちょうど 1 つ（要件 7.4 の生産者）"
    );
    assert!(
        code.contains("crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::ZORDER)"),
        "立てるのは重なりの旗（既存ペア機構と同じ書き方）"
    );
    assert!(
        squeeze(&code).contains(
            "if wants_wake(groups.pending, groups.has_verify()) { crate::ecs::world::tick_wake::mark(crate::ecs::world::tick_wake::ZORDER); }"
        ),
        "旗が⑥の判断に守られていない（無条件に立てると表示に変化の無い巡を省く仕組みが死ぬ）"
    );

    let pass_at = index_of(&code, "pub(crate) fn run_group_maintenance_pass<P:");
    let steps_call_at = index_of(
        &code,
        "run_group_maintenance_steps(groups, pair_fix_this_pass, probe);",
    );
    let mark_at = index_of(&code, "tick_wake::mark(");
    let steps_fn_at = index_of(&code, "fn run_group_maintenance_steps<P:");
    let verify_fn_at = index_of(&code, "fn verify_previous_issue<P:");
    assert!(
        pass_at < steps_call_at && steps_call_at < mark_at && mark_at < steps_fn_at,
        "旗が①〜⑤の後に置かれていない（巡={pass_at}・①〜⑤の呼出={steps_call_at}・旗={mark_at}・①〜⑤の本体={steps_fn_at}）"
    );

    let outer = &code[pass_at..steps_fn_at];
    assert!(
        !outer.contains("return"),
        "旗を立てる側に早期の脱出が在る（促さないまま抜ける巡ができる）"
    );
    let steps = &code[steps_fn_at..verify_fn_at];
    assert!(
        steps.matches("return;").count() >= 2,
        "②の門と③の調停の脱出が①〜⑤の側に見当たらない（走査の前提が崩れている）"
    );
    assert!(
        steps.contains("groups.pending = false;"),
        "⑤の印の解除が①〜⑤の側に見当たらない（旗が古い印を見て 1 巡余分に促す）"
    );

    // 迂回路——①〜⑤だけを回す側は「旗を立てない双子」である。本番がそちらを呼べば
    // 要件 7.4 は本番だけで死に、判断も位置も正しいままなので他の檻は 1 本も赤くならない。
    // よって呼出は⑥の内側の 1 つに限り、本番の system が旗の立つ入口を通ることを名指しで
    // 押さえる（この 2 本が無いと `:386` の 1 語の書き換えが全緑で通る）。
    assert_eq!(
        code.matches("run_group_maintenance_steps(").count(),
        1,
        "①〜⑤を旗の側を迂回して呼ぶ経路がある（本番が旗の立たない側を通れる）"
    );
    assert!(
        code.contains("run_group_maintenance_pass(&mut groups, !pair_fixes.is_empty(), &probe);"),
        "本番の system が旗の立つ入口を通っていない（要件 7.4 が本番で成立しない）"
    );
    assert!(
        code.contains("fn wants_wake("),
        "説明文を落とす処理が本文まで落としている"
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

/// 本文に 1 度だけ現れるはずの字面の位置（見つからなければその場で落とす）。
fn index_of(source: &str, needle: &str) -> usize {
    source
        .find(needle)
        .unwrap_or_else(|| panic!("本番の字面 `{needle}` が見つからない（檻の前提が崩れている）"))
}
