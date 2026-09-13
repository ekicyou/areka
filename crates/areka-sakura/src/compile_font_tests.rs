//! `Instruction::Font`（`\f[...]`）の台本写像の檻（要件 2.3/2.4/14.4・15.2）。
//!
//! 固定するのは 4 点:
//!
//! 1. 装飾の cue は**再生時間 0**（`duration == 0.0`）。
//! 2. 前後の文字と**時刻が整合**する（書き出し位置 `offset` の転写ゆえ、直前の Text の
//!    再生完了時刻＝直後の Text の開始時刻と同一）。
//! 3. `\f` の**有無で他の cue の時刻と長さが変わらない**（要件 14.4）。
//! 4. 台本の先頭の **`ClearAll` 前置は従来どおり**。
//!
//! 較正（要件 15.2）: `duration` を非 0 にする誤りは (1)、腕を catch-all より後ろに置いて
//! cue が生成されなくなる誤りは (1)(2)(4) の cue 本数と種別の断言が、それぞれ赤にする。

use super::test_support::{assert_clear_all_prefix_and_rest, compile, cue_eq};
use super::*;
use crate::contract::FONT_TAG_CARRIER;
use crate::duration::text_playback_duration;
use areka_parsers::sakura::NewLineRatio;

/// cue が `\f` の汎用キャリアなら引数列を返す（他の cue は `None`）。
fn font_tokens(cue: &Cue) -> Option<Vec<String>> {
    match &cue.payload {
        CuePayload::Command(cmd) => match cmd.as_command_carrier() {
            Some((name, tokens)) if name == FONT_TAG_CARRIER => {
                Some(tokens.into_iter().map(str::to_string).collect())
            }
            _ => None,
        },
        _ => None,
    }
}

/// `\f` は再生時間 0 の汎用キャリア cue として、前後の文字の間の書き出し位置へ載る
/// （要件 2.3/2.4/14.4）。catch-all（M-boot 外タグの無視）へ落ちて捨てられないことの固定。
#[test]
fn font_is_zero_duration_carrier_between_neighbours() {
    let compiled = compile(&[
        Instruction::Text("ab".into()),
        Instruction::Font {
            args: vec!["bold".into(), "1".into()],
        },
        Instruction::Text("cd".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        cues.len(),
        3,
        "Text・Font・Text の 3 cue（Font が捨てられた）"
    );

    // (2) 並び順は書き出し位置の転写で保たれる: 直前 Text の再生完了＝Font＝直後 Text の開始。
    let boundary = text_playback_duration("ab");
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(cues[0].duration, boundary);
    assert_eq!(
        cues[1].start_time, boundary,
        "Font の start_time は直前 Text の再生完了時刻"
    );
    assert_eq!(
        cues[2].start_time, boundary,
        "直後 Text の start_time は Font と同一（Font は時間を進めない）"
    );

    // (1) 再生時間 0（非 0 を与える誤りはここで赤になる）。
    assert_eq!(cues[1].duration, 0.0, "装飾の命令の再生時間は 0");

    // (2') 運搬形は typed variant でなく 1 本の汎用キャリア（要件 2.4）。
    assert_eq!(
        font_tokens(&cues[1]).as_deref(),
        Some(["bold".to_string(), "1".to_string()].as_slice()),
        "`\\f` は FONT_TAG_CARRIER の汎用キャリアで引数列を記述順のまま運ぶ"
    );
    // scope の転写は他の cue と同じ規律（既定 "0"）。
    assert_eq!(cues[1].actor, ActorKey::from("0".to_string()));
}

/// 引数の 4 つの形（要件 2.2 の受理形）が台本まで無変形で届く（空トークンを潰さない）。
#[test]
fn font_arg_shapes_are_carried_verbatim() {
    // 表の母数を先に固定する（表が空になるとループが恒真で緑になる）。
    let shapes: [Vec<String>; 4] = [
        vec![],                                                     // `\f[]` / 裸の `\f`
        vec![String::new()],                                        // `\f[""]`
        vec!["bold".into(), String::new()],                         // `\f[bold,]`
        vec!["color".into(), "255".into(), "0".into(), "0".into()], // `\f[color,255,0,0]`
    ];
    assert_eq!(shapes.len(), 4, "引数の形は 4 通り");

    for want in shapes {
        let compiled = compile(&[Instruction::Font { args: want.clone() }]);
        let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
        assert_eq!(cues.len(), 1, "Font 単独でも cue を 1 件生成する: {want:?}");
        assert_eq!(cues[0].duration, 0.0, "再生時間は 0: {want:?}");
        assert_eq!(
            font_tokens(&cues[0]),
            Some(want.clone()),
            "引数列は記述順のまま無変形（空トークンも潰さない）: {want:?}"
        );
    }
}

/// `\f` の有無で、装飾以外の cue の時刻・長さ・内容が 1 ビットも変わらない（要件 14.4）。
/// 先頭の `ClearAll` 前置も従来どおり（要件 2.3 の「先頭の全消去」）。
#[test]
fn font_does_not_change_other_cue_timings() {
    let without = compile(&[
        Instruction::Text("ab".into()),
        Instruction::Wait(std::time::Duration::from_millis(300)),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Text("cd".into()),
    ]);
    let with = compile(&[
        Instruction::Font {
            args: vec!["default".into()],
        },
        Instruction::Text("ab".into()),
        Instruction::Font {
            args: vec!["bold".into(), "1".into()],
        },
        Instruction::Wait(std::time::Duration::from_millis(300)),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Font {
            args: vec!["italic".into(), "0".into()],
        },
        Instruction::Text("cd".into()),
    ]);

    // 先頭の全消去は従来どおり（両方に単一前置）。
    let base = assert_clear_all_prefix_and_rest(without.sheet.cues());
    let rest = assert_clear_all_prefix_and_rest(with.sheet.cues());

    assert_eq!(
        rest.iter().filter(|c| font_tokens(c).is_some()).count(),
        3,
        "装飾の cue は 3 件（捨てられていない）"
    );
    let others: Vec<&Cue> = rest.iter().filter(|c| font_tokens(c).is_none()).collect();
    assert_eq!(
        others.len(),
        base.len(),
        "装飾以外の cue の本数が装飾の有無で変わらない"
    );
    for (i, (a, b)) in base.iter().zip(others.iter()).enumerate() {
        assert!(
            cue_eq(a, b),
            "装飾の有無で cue[{i}] が変わった: {:?} != {:?}",
            (a.start_time, a.duration, &a.payload),
            (b.start_time, b.duration, &b.payload)
        );
    }

    // 台本全体の占有時間も不変（`\f` は時間を進めない）。
    let horizon = |t: &CompiledTalk| {
        t.sheet
            .cues()
            .iter()
            .map(|c| c.start_time + c.duration)
            .fold(0.0_f64, f64::max)
    };
    assert_eq!(horizon(&without), horizon(&with), "台本の占有時間は不変");
}
