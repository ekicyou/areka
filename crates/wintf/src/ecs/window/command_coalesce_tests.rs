//! 同一 tick・同一窓のジオメトリ指令の**合流**に対する決定論的テスト（要件 4.5・4.3・10.3）。
//!
//! 固定するのは 5 つである。
//!
//! - **窓ごとの書込回数**——実機の 1 遷移・1 スコープを再現した積み上げで、バルーン窓へ
//!   積まれる指令が 1 本になること（是正前は 2 本＝決定論の上限 1 に対する違反）。
//! - **後勝ちと札の合成**——位置・寸は後着の値が勝ち、「移動なし」「寸なし」の札は
//!   **双方が持つときだけ**残ること。合流後の最終ジオメトリが逐次適用と一致すること。
//! - **畳まないもの**——Z のみの指令・表示状態を変える指令・挿入位置を持つ指令・
//!   活性化を伴う指令・別の窓の指令は、合流の対象にも先にもならないこと（10.3）。
//! - **順序**——畳めない指令を**跨いで**畳まないこと。Z 指令の相対順と引数が不変であること。
//! - **積み上げの記録**——畳んだときは `merged_into_seq` に**先着の通し番号**が載り、
//!   畳まなかったときは番兵のままであること。
//!
//! # 「畳まない」の主張には必ず陽性の対を置く
//!
//! 「Z 専用は畳まれない」は、合流そのものが死んでいても緑になる。よって各否定の主張には
//! **同じ駆動口で畳まれる側**を隣に置き、駆動口が生きていることを同時に示す。
//!
//! # 純関数を直に呼ぶテストがある理由
//!
//! [`coalesce_geometry`](super::coalesce_geometry) は `&mut Vec` を受ける純関数なので、
//! `enqueue` 経由では作れない配置（同一窓の合流可能な指令が 2 本並ぶ状態）を手で組める。
//! 「**先着**の枠へ畳む」は、その配置でしか後着との差が出ない。

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_TOP, IsWindow, SET_WINDOW_POS_FLAGS, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
};

use super::super::transition_diag::{KIND_ENQUEUE, RECORD_PREFIX_TAG, TRANSITION_TARGET, WriteTag};
use super::{SetWindowPosCommand, coalesce_geometry, drain_window_pos_commands};
use crate::ecs::test_support::capture_under_filter;

/// 観測チャネルを点灯させる directive。
const SIGNOFF_DIRECTIVES: &str = "info,wintf::transition=debug";

/// 捕捉が生きている証拠として同じ窓の中で必ず出す対照行。
const CONTROL_LINE: &str = "[coalesce-probe] alive";

fn emit_control() {
    tracing::info!(target: TRANSITION_TARGET, "{CONTROL_LINE}");
}

/// 実在し得ない窓ハンドルを作る。
///
/// 64bit Windows の `HWND` は 32bit 値の**符号拡張**であり、上位 32bit が 0 のまま下位が
/// 負域にある値はハンドル表に存在し得ない。それでも他プロセスの実窓を掴んでいないことを
/// `IsWindow` で毎回確かめる。
///
/// 識別子は**左へ 1 桁ずらしてから**奇数化する——`usize::from(tag) | 1` だと 0x60 と 0x61 が
/// 同じ値へ潰れ、「別の窓」を意図した呼び分けが同一窓を指す。
fn fake_hwnd(tag: u8) -> HWND {
    let value = 0xFFFF_FE00_usize | (usize::from(tag) << 1) | 1;
    let hwnd = HWND(value as *mut core::ffi::c_void);
    // SAFETY: `IsWindow` は任意のハンドル値に対して安全に真偽を返す読み取り専用 API で
    // あり、無効値でも未定義動作を起こさない。
    assert!(
        !unsafe { IsWindow(Some(hwnd)) }.as_bool(),
        "偽ハンドル 0x{value:X} が実窓を掴んでいる——このまま書込を撃つと無関係の窓を動かす"
    );
    hwnd
}

/// 位置と寸の両方を書く指令（実機の `origin=DpiReproject`／`KeepPositionResize`＝`flags=0x14`）。
fn move_and_size(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) -> SetWindowPosCommand {
    SetWindowPosCommand::new(hwnd, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE, None)
}

/// 位置だけを動かす指令（実機の `origin=BalloonFollow`＝`flags=0x15`）。
fn move_only(hwnd: HWND, x: i32, y: i32) -> SetWindowPosCommand {
    SetWindowPosCommand::new(
        hwnd,
        x,
        y,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    )
}

/// 寸だけを変える指令（`flags=0x16`）。
fn size_only(hwnd: HWND, w: i32, h: i32) -> SetWindowPosCommand {
    SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        w,
        h,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    )
}

/// Z 順だけを動かす指令（`zorder_pair_maintain::pair_fix_command` と同じ形）。
fn zorder_only(hwnd: HWND, insert_after: HWND) -> SetWindowPosCommand {
    SetWindowPosCommand::new(
        hwnd,
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        Some(insert_after),
    )
}

/// 窓ごとの書込回数（取り出した指令列を hwnd で数える）。
fn writes_per_window(commands: &[SetWindowPosCommand], hwnd: HWND) -> usize {
    commands.iter().filter(|cmd| cmd.hwnd == hwnd).count()
}

/// キューを空にしてから、与えた指令を順に積み、積み上がった列を返す。
fn enqueue_all(commands: Vec<SetWindowPosCommand>) -> Vec<SetWindowPosCommand> {
    let _residue = drain_window_pos_commands();
    for cmd in commands {
        SetWindowPosCommand::enqueue(cmd);
    }
    drain_window_pos_commands()
}

/// 逐次適用の模型（合流を通さずに 1 本ずつ当てたときの最終ジオメトリ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Geometry {
    fn apply(&mut self, cmd: &SetWindowPosCommand) {
        if (cmd.flags.0 & SWP_NOMOVE.0) == 0 {
            self.x = cmd.x;
            self.y = cmd.y;
        }
        if (cmd.flags.0 & SWP_NOSIZE.0) == 0 {
            self.w = cmd.width;
            self.h = cmd.height;
        }
    }
}

// ---------------------------------------------------------------------------
// 窓ごとの書込回数（要件 4.5・基準値 §3.2 ⑵ の再現）
// ---------------------------------------------------------------------------

/// 1 遷移でバルーン窓へ積まれる指令が **1 本**になる。
///
/// 実機（`atom-20260820-193457`）の 1 遷移・1 スコープぶんをそのまま再現する——キャラ窓へ
/// 再射影の 1 本、バルーン窓へ寸変更（`KeepPositionResize`・`flags=0x14`）と位置追従
/// （`BalloonFollow`・`flags=0x15`）の 2 本。決定論の上限は窓あたり 1 なので、バルーン窓の
/// 2 本が 12 遷移 × 2 スコープ＝24 件の違反になっていた。
#[test]
fn a_balloon_window_is_written_once_per_transition() {
    let shell = fake_hwnd(0x60);
    let balloon = fake_hwnd(0x61);

    let drained = enqueue_all(vec![
        move_and_size(shell, 100, 200, 382, 684).with_tag(WriteTag {
            origin: "DpiReproject",
            scope: Some(0),
            kind: "shell",
        }),
        move_and_size(balloon, 500, 200, 336, 240).with_tag(WriteTag {
            origin: "KeepPositionResize",
            scope: Some(0),
            kind: "balloon",
        }),
        move_only(balloon, 482, 210).with_tag(WriteTag {
            origin: "BalloonFollow",
            scope: Some(0),
            kind: "balloon",
        }),
    ]);

    assert_eq!(
        writes_per_window(&drained, shell),
        1,
        "キャラ窓は元から 1 本（対照）: {drained:?}"
    );
    assert_eq!(
        writes_per_window(&drained, balloon),
        1,
        "バルーン窓の書込は寸と位置を畳んで 1 本になる: {drained:?}"
    );

    let merged = drained
        .iter()
        .find(|cmd| cmd.hwnd == balloon)
        .expect("バルーン窓の指令が消えている");
    assert_eq!((merged.x, merged.y), (482, 210), "位置は後勝ち");
    assert_eq!((merged.width, merged.height), (336, 240), "寸は先着の値");
    assert_eq!(
        merged.flags,
        SWP_NOZORDER | SWP_NOACTIVATE,
        "移動なし／寸なしの札は双方が持つときだけ残る"
    );
    assert_eq!(
        merged.tag.origin, "KeepPositionResize",
        "タグは先着の値を保つ: {merged:?}"
    );
}

/// 寸だけ・位置だけの 2 本も 1 本になる（札の合成が両方向で効くことの対）。
#[test]
fn size_only_then_move_only_also_collapses_to_one_command() {
    let hwnd = fake_hwnd(0x62);
    let drained = enqueue_all(vec![size_only(hwnd, 300, 400), move_only(hwnd, 11, 22)]);

    assert_eq!(writes_per_window(&drained, hwnd), 1, "{drained:?}");
    assert_eq!((drained[0].x, drained[0].y), (11, 22));
    assert_eq!((drained[0].width, drained[0].height), (300, 400));
    assert_eq!(
        drained[0].flags,
        SWP_NOZORDER | SWP_NOACTIVATE,
        "先着の「移動なし」も後着の「寸なし」も落ちる: {:?}",
        drained[0]
    );
}

/// 3 本以上でも畳み続ける（合流結果がもう一度合流先になれる）。
#[test]
fn three_geometry_commands_for_one_window_collapse_to_one() {
    let hwnd = fake_hwnd(0x63);
    let drained = enqueue_all(vec![
        move_and_size(hwnd, 1, 2, 3, 4),
        size_only(hwnd, 30, 40),
        move_only(hwnd, 10, 20),
    ]);

    assert_eq!(drained.len(), 1, "{drained:?}");
    assert_eq!((drained[0].x, drained[0].y), (10, 20));
    assert_eq!((drained[0].width, drained[0].height), (30, 40));
}

// ---------------------------------------------------------------------------
// 後勝ちと札の合成
// ---------------------------------------------------------------------------

/// 「移動なし」は**双方が持つときだけ**残る（陽性と陰性の対）。
#[test]
fn no_move_survives_only_when_both_commands_carry_it() {
    let both = fake_hwnd(0x64);
    let drained = enqueue_all(vec![size_only(both, 10, 20), size_only(both, 30, 40)]);
    assert_eq!(drained.len(), 1);
    assert_eq!(
        drained[0].flags,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        "双方が「移動なし」なら残る: {:?}",
        drained[0]
    );
    assert_eq!(
        (drained[0].width, drained[0].height),
        (30, 40),
        "寸は後勝ち"
    );

    // 後着だけが「移動なし」——先着の位置がそのまま残り、札は落ちる。
    let one = fake_hwnd(0x65);
    let drained = enqueue_all(vec![
        move_and_size(one, 7, 8, 9, 10),
        size_only(one, 30, 40),
    ]);
    assert_eq!(drained.len(), 1);
    assert_eq!(
        (drained[0].x, drained[0].y),
        (7, 8),
        "後着が動かさない項目は先着の値のまま（0 で塗り潰さない）: {:?}",
        drained[0]
    );
    assert_eq!(
        drained[0].flags,
        SWP_NOZORDER | SWP_NOACTIVATE,
        "片方だけの「移動なし」は残らない: {:?}",
        drained[0]
    );
}

/// 「寸なし」は**双方が持つときだけ**残る（陽性と陰性の対）。
#[test]
fn no_size_survives_only_when_both_commands_carry_it() {
    let both = fake_hwnd(0x66);
    let drained = enqueue_all(vec![move_only(both, 1, 2), move_only(both, 3, 4)]);
    assert_eq!(drained.len(), 1);
    assert_eq!(
        drained[0].flags,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        "双方が「寸なし」なら残る: {:?}",
        drained[0]
    );
    assert_eq!((drained[0].x, drained[0].y), (3, 4), "位置は後勝ち");

    // 後着だけが「寸なし」——先着の寸がそのまま残り、札は落ちる。
    let one = fake_hwnd(0x67);
    let drained = enqueue_all(vec![move_and_size(one, 7, 8, 9, 10), move_only(one, 3, 4)]);
    assert_eq!(drained.len(), 1);
    assert_eq!(
        (drained[0].width, drained[0].height),
        (9, 10),
        "後着が動かさない項目は先着の値のまま: {:?}",
        drained[0]
    );
    assert_eq!(
        drained[0].flags,
        SWP_NOZORDER | SWP_NOACTIVATE,
        "片方だけの「寸なし」は残らない: {:?}",
        drained[0]
    );
}

/// 合流後の最終ジオメトリが**逐次適用**と一致する（事後条件・design.md C2）。
///
/// 4 種の指令の全並び（256 通り）を、合流あり／なしの両方で当てて突き合わせる。
#[test]
fn the_coalesced_command_matches_sequential_application() {
    let hwnd = fake_hwnd(0x68);
    let choices: [fn(HWND) -> SetWindowPosCommand; 4] = [
        |h| move_and_size(h, 11, 12, 13, 14),
        |h| move_only(h, 21, 22),
        |h| size_only(h, 33, 34),
        |h| move_and_size(h, 41, 42, 43, 44),
    ];

    let start = Geometry {
        x: -1,
        y: -2,
        w: -3,
        h: -4,
    };
    for a in 0..choices.len() {
        for b in 0..choices.len() {
            for c in 0..choices.len() {
                for d in 0..choices.len() {
                    let sequence: Vec<_> = [a, b, c, d]
                        .iter()
                        .map(|&index| choices[index](hwnd))
                        .collect();

                    let mut expected = start;
                    for cmd in &sequence {
                        expected.apply(cmd);
                    }

                    let drained = enqueue_all(sequence.clone());
                    assert_eq!(drained.len(), 1, "{sequence:?} が 1 本へ畳まれていない");
                    let mut actual = start;
                    actual.apply(&drained[0]);
                    assert_eq!(
                        actual, expected,
                        "合流後の最終ジオメトリが逐次適用と食い違う: {sequence:?} → {:?}",
                        drained[0]
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 畳まないもの（否定の主張には必ず陽性の対を置く）
// ---------------------------------------------------------------------------

/// Z のみの指令は合流の**対象にも先にもならない**。
///
/// 陽性の対: 同じ駆動口・同じ窓で、ジオメトリ指令どうしなら畳まれる。
#[test]
fn a_zorder_only_command_is_neither_merged_nor_merged_into() {
    let target = fake_hwnd(0x69);
    let anchor = fake_hwnd(0x6A);

    // 先にならない: Z 指令のあとにジオメトリ指令が来ても畳まれない。
    let drained = enqueue_all(vec![
        zorder_only(target, anchor),
        move_and_size(target, 1, 2, 3, 4),
    ]);
    assert_eq!(drained.len(), 2, "Z 指令へ畳んではならない: {drained:?}");
    assert_eq!(drained[0].hwnd_insert_after, Some(anchor));

    // 対象にもならない: ジオメトリ指令のあとに Z 指令が来ても畳まれない。
    let drained = enqueue_all(vec![
        move_and_size(target, 1, 2, 3, 4),
        zorder_only(target, anchor),
    ]);
    assert_eq!(drained.len(), 2, "Z 指令を畳んではならない: {drained:?}");
    assert_eq!(drained[1].hwnd_insert_after, Some(anchor));

    // 陽性の対——同じ駆動口・同じ窓で、ジオメトリ指令どうしは畳まれる。
    let drained = enqueue_all(vec![
        move_and_size(target, 1, 2, 3, 4),
        move_only(target, 5, 6),
    ]);
    assert_eq!(
        drained.len(),
        1,
        "駆動口が死んでいる（否定の主張が空虚になる）: {drained:?}"
    );
}

/// 表示状態を変える指令（`SWP_SHOWWINDOW`）は畳まない。陽性の対を隣に置く。
#[test]
fn a_command_that_changes_visibility_is_not_coalesced() {
    let hwnd = fake_hwnd(0x6B);
    let show = SetWindowPosCommand::new(
        hwnd,
        1,
        2,
        3,
        4,
        SWP_SHOWWINDOW | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    );

    let drained = enqueue_all(vec![show.clone(), move_only(hwnd, 5, 6)]);
    assert_eq!(
        drained.len(),
        2,
        "表示状態を変える指令へ畳んだ: {drained:?}"
    );

    let drained = enqueue_all(vec![move_and_size(hwnd, 1, 2, 3, 4), show.clone()]);
    assert_eq!(
        drained.len(),
        2,
        "表示状態を変える指令を畳んだ: {drained:?}"
    );

    // 陽性の対——`SWP_SHOWWINDOW` を落とすだけで畳まれる。
    let drained = enqueue_all(vec![
        move_and_size(hwnd, 1, 2, 3, 4),
        SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOZORDER | SWP_NOACTIVATE, None),
    ]);
    assert_eq!(drained.len(), 1, "{drained:?}");
}

/// 非クライアント領域を作り直す指令（`SWP_FRAMECHANGED`）も畳まない。
#[test]
fn a_command_that_rebuilds_the_frame_is_not_coalesced() {
    let hwnd = fake_hwnd(0x6C);
    let framechanged = SetWindowPosCommand::new(
        hwnd,
        1,
        2,
        3,
        4,
        SWP_FRAMECHANGED | SWP_NOZORDER | SWP_NOACTIVATE,
        None,
    );

    let drained = enqueue_all(vec![framechanged.clone(), move_only(hwnd, 5, 6)]);
    assert_eq!(drained.len(), 2, "{drained:?}");

    // 陽性の対——同じ矩形・同じ窓でもフラグを落とせば畳まれる。
    let drained = enqueue_all(vec![
        SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOZORDER | SWP_NOACTIVATE, None),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(drained.len(), 1, "{drained:?}");
}

/// 挿入位置を持つ指令は、Z を動かさないフラグでも畳まない。
///
/// `SWP_NOZORDER` が付いていれば挿入位置は無視される——それでも「挿入位置つき」を
/// 合流の枠にすると、後で Z の意味を変えたときに黙って壊れる。持っていたら畳まない。
#[test]
fn a_command_carrying_an_insert_position_is_not_coalesced() {
    let hwnd = fake_hwnd(0x6D);
    let with_insert = SetWindowPosCommand::new(
        hwnd,
        1,
        2,
        3,
        4,
        SWP_NOZORDER | SWP_NOACTIVATE,
        Some(HWND_TOP),
    );

    let drained = enqueue_all(vec![with_insert.clone(), move_only(hwnd, 5, 6)]);
    assert_eq!(drained.len(), 2, "挿入位置つきの枠へ畳んだ: {drained:?}");
    assert_eq!(drained[0].hwnd_insert_after, Some(HWND_TOP));

    let drained = enqueue_all(vec![move_and_size(hwnd, 1, 2, 3, 4), with_insert.clone()]);
    assert_eq!(drained.len(), 2, "挿入位置つきの指令を畳んだ: {drained:?}");

    // 陽性の対——挿入位置を `None` にするだけで畳まれる（フラグは同一）。
    let drained = enqueue_all(vec![
        SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOZORDER | SWP_NOACTIVATE, None),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(drained.len(), 1, "{drained:?}");
}

/// 活性化を伴う指令（`SWP_NOACTIVATE` を欠く）は畳まない。
#[test]
fn a_command_that_activates_the_window_is_not_coalesced() {
    let hwnd = fake_hwnd(0x6E);
    let activating = SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOZORDER, None);

    let drained = enqueue_all(vec![activating.clone(), move_only(hwnd, 5, 6)]);
    assert_eq!(drained.len(), 2, "{drained:?}");

    let drained = enqueue_all(vec![move_and_size(hwnd, 9, 9, 9, 9), activating.clone()]);
    assert_eq!(drained.len(), 2, "{drained:?}");

    // 陽性の対——`SWP_NOACTIVATE` を足すだけで畳まれる。
    let drained = enqueue_all(vec![
        SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOZORDER | SWP_NOACTIVATE, None),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(drained.len(), 1, "{drained:?}");
}

/// Z を動かす指令（`SWP_NOZORDER` を欠く）は、挿入位置を持たなくても畳まない。
///
/// 挿入位置が `None` のまま `SWP_NOZORDER` を欠くと、`SetWindowPos` は `HWND_TOP` を渡された
/// のと同じに扱って窓を最前面へ動かす。挿入位置の有無だけでは Z を動かす指令を選り分け
/// られない——ゆえに `SWP_NOZORDER` を独立の必要条件として持つ。
#[test]
fn a_command_without_no_zorder_is_not_coalesced_even_without_an_insert_position() {
    let hwnd = fake_hwnd(0x80);
    let to_top = SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOACTIVATE, None);

    let drained = enqueue_all(vec![to_top.clone(), move_only(hwnd, 5, 6)]);
    assert_eq!(drained.len(), 2, "Z を動かす指令へ畳んだ: {drained:?}");
    assert_eq!(drained[0].flags, SWP_NOACTIVATE);

    let drained = enqueue_all(vec![move_and_size(hwnd, 9, 9, 9, 9), to_top.clone()]);
    assert_eq!(drained.len(), 2, "Z を動かす指令を畳んだ: {drained:?}");

    // 陽性の対——`SWP_NOZORDER` を足すだけで畳まれる（挿入位置は両方とも `None` のまま）。
    let drained = enqueue_all(vec![
        SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SWP_NOZORDER | SWP_NOACTIVATE, None),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(drained.len(), 1, "{drained:?}");
}

/// フラグ 0 の指令（Z も活性化も動かす）は畳まない。
#[test]
fn a_zero_flags_command_is_not_coalesced() {
    let hwnd = fake_hwnd(0x6F);
    let bare = SetWindowPosCommand::new(hwnd, 1, 2, 3, 4, SET_WINDOW_POS_FLAGS(0), None);

    let drained = enqueue_all(vec![bare.clone(), move_only(hwnd, 5, 6)]);
    assert_eq!(drained.len(), 2, "{drained:?}");
    assert_eq!(drained[0].flags, SET_WINDOW_POS_FLAGS(0));

    let drained = enqueue_all(vec![move_and_size(hwnd, 1, 2, 3, 4), bare.clone()]);
    assert_eq!(drained.len(), 2, "{drained:?}");
}

/// 別の窓の指令は畳まない。陽性の対は同一窓の同じ 2 本。
#[test]
fn commands_for_different_windows_are_not_coalesced() {
    let first = fake_hwnd(0x70);
    let second = fake_hwnd(0x71);
    assert_ne!(first, second, "前提: 2 つの偽ハンドルが別値である");

    let drained = enqueue_all(vec![
        move_and_size(first, 1, 2, 3, 4),
        move_only(second, 5, 6),
    ]);
    assert_eq!(drained.len(), 2, "別窓を畳んだ: {drained:?}");
    assert_eq!(drained[0].hwnd, first);
    assert_eq!(drained[1].hwnd, second);

    // 陽性の対——同じ 2 本を同一窓へ向ければ畳まれる。
    let drained = enqueue_all(vec![
        move_and_size(first, 1, 2, 3, 4),
        move_only(first, 5, 6),
    ]);
    assert_eq!(drained.len(), 1, "{drained:?}");
}

// ---------------------------------------------------------------------------
// 順序（10.3）
// ---------------------------------------------------------------------------

/// 同一窓の畳めない指令を**跨いで**畳まない。
///
/// 跨ぐと当該窓のジオメトリ書込が Z 指令より前へ移り、相対順が変わる。最終状態が同じでも
/// 順序は変わるので、要件 10.3 の「適用順を変えない」に当たらない。
#[test]
fn a_geometry_command_is_not_merged_across_a_blocking_command_for_the_same_window() {
    let hwnd = fake_hwnd(0x72);
    let anchor = fake_hwnd(0x73);

    let drained = enqueue_all(vec![
        move_and_size(hwnd, 1, 2, 3, 4),
        zorder_only(hwnd, anchor),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(drained.len(), 3, "Z 指令を跨いで畳んだ: {drained:?}");
    assert_eq!(
        drained[1].hwnd_insert_after,
        Some(anchor),
        "Z 指令は 2 番目のまま: {drained:?}"
    );
    assert_eq!((drained[0].x, drained[0].y), (1, 2));
    assert_eq!((drained[2].x, drained[2].y), (5, 6));

    // 仕切りの**後**どうしは畳まれる（仕切りが以後を丸ごと止めるわけではない）。
    let drained = enqueue_all(vec![
        zorder_only(hwnd, anchor),
        move_and_size(hwnd, 1, 2, 3, 4),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(drained.len(), 2, "{drained:?}");
    assert_eq!((drained[1].x, drained[1].y), (5, 6));
    assert_eq!((drained[1].width, drained[1].height), (3, 4));
}

/// **別の窓**の畳めない指令は仕切りにならない。
///
/// `SWP_NOZORDER` 付きの書込は他窓の状態に触れないため、跨いでも他窓の適用順は変わらない。
#[test]
fn a_blocking_command_for_another_window_does_not_block_the_merge() {
    let hwnd = fake_hwnd(0x74);
    let other = fake_hwnd(0x75);
    let anchor = fake_hwnd(0x76);

    let drained = enqueue_all(vec![
        move_and_size(hwnd, 1, 2, 3, 4),
        zorder_only(other, anchor),
        move_only(hwnd, 5, 6),
    ]);
    assert_eq!(
        drained.len(),
        2,
        "別窓の Z 指令が仕切りになった: {drained:?}"
    );
    assert_eq!((drained[0].x, drained[0].y), (5, 6), "先着の枠へ畳まれる");
    assert_eq!(drained[1].hwnd, other, "Z 指令はそのまま残る");
    assert_eq!(drained[1].hwnd_insert_after, Some(anchor));
}

/// Z 指令どうしの相対順と引数は合流の導入で変わらない（要件 10.3）。
#[test]
fn zorder_commands_keep_their_relative_order_and_arguments() {
    let balloon = fake_hwnd(0x77);
    let character = fake_hwnd(0x78);

    let drained = enqueue_all(vec![
        zorder_only(balloon, character),
        move_and_size(balloon, 1, 2, 3, 4),
        zorder_only(character, balloon),
        move_only(character, 5, 6),
        zorder_only(balloon, HWND_TOP),
    ]);

    let zorders: Vec<_> = drained
        .iter()
        .filter(|cmd| cmd.hwnd_insert_after.is_some())
        .map(|cmd| (cmd.hwnd, cmd.hwnd_insert_after, cmd.flags))
        .collect();
    assert_eq!(
        zorders,
        vec![
            (
                balloon,
                Some(character),
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            ),
            (
                character,
                Some(balloon),
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            ),
            (
                balloon,
                Some(HWND_TOP),
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE
            ),
        ],
        "Z 指令の相対順と引数が変わった: {drained:?}"
    );

    // 非空虚性——ジオメトリ側は仕切りに挟まれて畳まれないので 5 本のまま。
    assert_eq!(drained.len(), 5, "{drained:?}");
}

// ---------------------------------------------------------------------------
// 純関数を直に呼ぶ（`enqueue` では作れない配置）
// ---------------------------------------------------------------------------

/// 畳み先は同一窓の**先着**である（後着ではない）。
///
/// `enqueue` は畳み続けるので同一窓の合流可能な指令が 2 本並ぶ状態を作れない。純関数を
/// 直に呼んで手で並べ、先着が選ばれることを固定する。
#[test]
fn the_merge_target_is_the_first_command_of_that_window() {
    let hwnd = fake_hwnd(0x79);
    let other = fake_hwnd(0x7A);
    let mut queue = vec![
        move_and_size(other, 0, 0, 1, 1),
        move_and_size(hwnd, 1, 2, 3, 4).with_tag(WriteTag {
            origin: "first",
            scope: Some(0),
            kind: "shell",
        }),
        move_and_size(hwnd, 5, 6, 7, 8).with_tag(WriteTag {
            origin: "second",
            scope: Some(0),
            kind: "shell",
        }),
    ];

    let merged = coalesce_geometry(&mut queue, move_only(hwnd, 9, 10));

    assert_eq!(merged, Some(1), "先着（添字 1）の枠へ畳む: {queue:?}");
    assert_eq!(queue.len(), 3, "件数は増えない: {queue:?}");
    assert_eq!(
        (queue[1].x, queue[1].y),
        (9, 10),
        "先着の枠が更新される: {queue:?}"
    );
    assert_eq!(
        queue[1].tag.origin, "first",
        "タグは先着のまま: {:?}",
        queue[1]
    );
    assert_eq!(
        (queue[2].x, queue[2].y),
        (5, 6),
        "後着の枠は触らない: {queue:?}"
    );
}

/// 畳めない指令はそのまま末尾へ積まれ、`None` が返る。
#[test]
fn an_incoming_command_that_cannot_coalesce_is_pushed_verbatim() {
    let hwnd = fake_hwnd(0x7B);
    let anchor = fake_hwnd(0x7C);
    let mut queue = vec![move_and_size(hwnd, 1, 2, 3, 4)];

    let merged = coalesce_geometry(&mut queue, zorder_only(hwnd, anchor));

    assert_eq!(merged, None, "合流先は無い: {queue:?}");
    assert_eq!(queue.len(), 2, "末尾へ積まれる: {queue:?}");
    assert_eq!(queue[1].hwnd_insert_after, Some(anchor));

    // 仕切りの手前へは戻らない。
    let merged = coalesce_geometry(&mut queue, move_only(hwnd, 5, 6));
    assert_eq!(merged, None, "仕切りの手前へ畳んだ: {queue:?}");

    // 陽性の対——仕切りの無いキューなら同じ指令が畳まれて先着の番号が返る。
    let mut queue = vec![move_and_size(hwnd, 1, 2, 3, 4)];
    let merged = coalesce_geometry(&mut queue, move_only(hwnd, 5, 6));
    assert_eq!(merged, Some(0), "{queue:?}");
    assert_eq!(queue.len(), 1);
}

/// 空のキューへの最初の 1 件は畳まれない（合流先が無い）。
#[test]
fn the_first_command_of_a_tick_is_never_merged() {
    let hwnd = fake_hwnd(0x7D);
    let mut queue = Vec::new();
    let merged = coalesce_geometry(&mut queue, move_and_size(hwnd, 1, 2, 3, 4));
    assert_eq!(merged, None);
    assert_eq!(queue.len(), 1);
}

// ---------------------------------------------------------------------------
// 積み上げの記録
// ---------------------------------------------------------------------------

/// 畳んだ積み上げの記録に**先着の通し番号**が載り、畳まなかった記録は番兵のまま。
#[test]
fn the_enqueue_record_carries_the_merge_target_seq() {
    let first = fake_hwnd(0x7E);
    let second = fake_hwnd(0x7F);
    let _residue = drain_window_pos_commands();

    let captured = capture_under_filter(SIGNOFF_DIRECTIVES, || {
        emit_control();
        SetWindowPosCommand::enqueue(move_and_size(first, 1, 2, 3, 4));
        SetWindowPosCommand::enqueue(move_and_size(second, 1, 2, 3, 4));
        SetWindowPosCommand::enqueue(move_only(second, 5, 6));
        let _drained = drain_window_pos_commands();
    });

    assert!(
        captured.contains(CONTROL_LINE),
        "対照が拾えていない＝捕捉が死んでいる: {captured}"
    );
    let needle = format!("kind={KIND_ENQUEUE} ");
    let enqueues: Vec<&str> = captured
        .lines()
        .filter(|line| line.contains(RECORD_PREFIX_TAG) && line.contains(&needle))
        .collect();
    assert_eq!(enqueues.len(), 3, "積み上げ 3 件ぶんの行: {captured}");
    assert!(
        enqueues[0].contains("merged_into_seq=-"),
        "最初の 1 件は畳まれない: {}",
        enqueues[0]
    );
    assert!(
        enqueues[1].contains("merged_into_seq=-"),
        "別窓は畳まれない: {}",
        enqueues[1]
    );
    assert!(
        enqueues[2].contains("merged_into_seq=1"),
        "畳んだ先の通し番号（別窓の先着＝添字 1）が載る: {}",
        enqueues[2]
    );
}
