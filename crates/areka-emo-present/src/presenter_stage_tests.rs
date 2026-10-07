//! 閉じる → 出すの競り合いを、着く順の全部の並びで固定する（spec: areka-P0-animated-image-playback
//! task 4.4・要件 2.3・3.3・design Testing 7）。
//!
//! 面の表・コマ・指令の送り方は 4.2 の `presenter_film_tests.rs` の補助をそのまま使う（面 0 ＝
//! 回数つき `fin.png`（3 コマ）と終わりなし `end.png`（2 コマ）を並べたバルーン、面 1 ＝静止画）。
//! コマの番号は子の `frames` の添字（0 が経過 0 の絵）。回数つきの「コマ k」は 2、新しい指令の
//! 「コマ 1」は 1。終わりなしは全部の指令でコマ 1（経過 0 と違う絵）に置き、外されたら絵が変わる。

use super::*;

use super::film_tests::{T, ack, build_balloon, frames, golden, hide, send, shown};
use super::test_support::make_world_with_gpu;

/// 「隠す → `show_target`」の後に着くもの。
#[derive(Clone, Copy, Debug, PartialEq)]
enum Arrival {
    /// ① 世代を知る前に seriko が出した古い指令（回数つきはコマ k）。
    Old,
    /// ② seriko が閉じた知らせで出した戻しの指令（回数つきは経過 0）。
    Revert,
    /// ③ 合図 `StageAck`（今の出番の世代）。
    Ack,
    /// ④ 新しい指令（回数つきはコマ 1）。
    New,
}
use Arrival::{Ack, New, Old, Revert};

/// 制約の下で取りうる全部の並び（重複なし）: ③ と ④ はちょうど 1 件・①② は 0〜2 件・① は全部
/// ③ より前・③ は ④ より前。② は ③ の前後どこでもよいが、④ より後には着かない（④ は ② を出した
/// 後の刻みで seriko が出す指令）＝④ は常に最後。長さ 1〜5 の {①,②,③} の全部の列から制約を
/// 満たすものを拾い、最後に ④ を付ける（同じ種類どうしは区別しない）。
fn orderings() -> Vec<Vec<Arrival>> {
    let mut out = Vec::new();
    for len in 1..=5u32 {
        for code in 0..3usize.pow(len) {
            let seq: Vec<Arrival> = (0..len)
                .map(|i| [Old, Revert, Ack][code / 3usize.pow(i) % 3])
                .collect();
            let count = |k: Arrival| seq.iter().filter(|&&a| a == k).count();
            if count(Ack) != 1 || count(Old) > 2 || count(Revert) > 2 {
                continue;
            }
            let ack_at = seq.iter().position(|&a| a == Ack).expect("③ が 1 件");
            if seq
                .iter()
                .rposition(|&a| a == Old)
                .is_some_and(|o| o > ack_at)
            {
                continue;
            }
            let mut seq = seq;
            seq.push(New);
            out.push(seq);
        }
    }
    out
}

/// 全部の並びで: ④ が着くまでの全部の合成が経過 0 の絵（終わりなしは外れない）・④ の後はコマ 1。
/// 並びごとに、①が面の番号を替えない指令（コマ k）の回と、面の番号を替える指令（`\b[1]` 相当）の回
/// の 2 通りを流す。各到着の前に、古い世代の合図と出番より大きい合図を 1 件ずつ挟む（無視される）。
#[test]
fn every_arrival_order_after_reshow_shows_rest_until_the_new_command() {
    // 並びの数: ① を a 件・② を b 件とすると、①…① ③ の順が決まった a+1 件と ② b 件を並べる
    // 位置の選び方で C(a+b+1, b) 通り（④ は最後に固定）。a, b ∈ {0,1,2} で合計すると
    //   a=0: C(1,0)+C(2,1)+C(3,2) = 1+2+3  = 6
    //   a=1: C(2,0)+C(3,1)+C(4,2) = 1+3+6  = 10
    //   a=2: C(3,0)+C(4,1)+C(5,2) = 1+4+10 = 15
    // で 31 通り。
    let orders = orderings();
    assert_eq!(orders.len(), 31, "並びの数が手で数えた値と違う");

    let mut world = make_world_with_gpu();
    let (w, _) = build_balloon();
    let none = BindSet::default();
    let at_k = frames(&w, Some(2), Some(1));
    let rest = frames(&w, None, Some(1));
    let fresh = frames(&w, Some(1), Some(1));
    let rest_picture = golden(0, &none, &rest);
    let fresh_picture = golden(0, &none, &fresh);
    let still_picture = golden(1, &none, &PatternState::default());
    // 前提: 4 つの絵がどれも違う（外し・外し忘れ・終わりなしの外れが絵の違いで見える）。
    let k_picture = golden(0, &none, &at_k);
    let end_stripped = golden(0, &none, &frames(&w, None, None));
    for (a, b) in [
        (&rest_picture, &fresh_picture),
        (&rest_picture, &k_picture),
        (&rest_picture, &end_stripped),
        (&fresh_picture, &k_picture),
        (&rest_picture, &still_picture),
    ] {
        assert_ne!(a, b, "前提: 見分けたい絵が同じ");
    }

    for order in &orders {
        for old_changes_face in [false, true] {
            let case = format!("{order:?} 面を替える①={old_changes_face}");
            let mut presenter = shown(&mut world);
            send(&mut presenter, &mut world, 0, none.clone(), at_k.clone());
            assert_eq!(
                presenter.read_back(T).expect("read_back"),
                k_picture,
                "前提: 見えている間はコマ k: {case}"
            );

            hide(&mut presenter, &mut world);
            presenter.show_target(&mut world, T).expect("show_target");
            let stage = presenter.stage_generation(T).expect("装着済み");
            assert_eq!(stage, 2, "前提: 出し直しで世代が 1 つ進む: {case}");
            assert_eq!(
                presenter.read_back(T).expect("read_back"),
                rest_picture,
                "出し直しの絵が経過 0 でない: {case}"
            );

            for &arrival in order {
                // 古い世代の合図（追い付いた世代を戻さない）・出番より大きい合図（捨てる）。
                ack(&mut presenter, &mut world, T, stage - 1);
                ack(&mut presenter, &mut world, T, stage + 1);
                let expected = match arrival {
                    Old if old_changes_face => {
                        send(
                            &mut presenter,
                            &mut world,
                            1,
                            none.clone(),
                            PatternState::default(),
                        );
                        assert_eq!(
                            presenter.current_surface_id(T),
                            Some(1),
                            "面の番号を替える古い指令が捨てられた: {case}"
                        );
                        &still_picture
                    }
                    Old => {
                        send(&mut presenter, &mut world, 0, none.clone(), at_k.clone());
                        &rest_picture
                    }
                    Revert => {
                        send(&mut presenter, &mut world, 0, none.clone(), rest.clone());
                        &rest_picture
                    }
                    Ack => {
                        let before = presenter.read_back(T).expect("read_back");
                        ack(&mut presenter, &mut world, T, stage);
                        assert_eq!(
                            presenter.read_back(T).expect("read_back"),
                            before,
                            "合図で絵が変わった: {case}"
                        );
                        continue;
                    }
                    New => {
                        send(&mut presenter, &mut world, 0, none.clone(), fresh.clone());
                        &fresh_picture
                    }
                };
                assert_eq!(
                    &presenter.read_back(T).expect("read_back"),
                    expected,
                    "{arrival:?} の後の絵が違う: {case}"
                );
            }
            assert_eq!(presenter.stage_generation(T), Some(stage), "{case}");
            assert_eq!(presenter.target_visible(T), Some(true), "{case}");
        }
    }
}
