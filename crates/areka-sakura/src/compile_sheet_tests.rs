use super::test_support::{assert_clear_all_prefix_and_rest, command_of, compile, cue_eq};
use super::*;
use crate::duration::text_playback_duration;
use areka_parsers::sakura::{NewLineRatio, SurfaceArg};
use std::time::Duration;

/// 多様な variant を織り交ぜた代表的な命令列（決定性・不変条件の両テストで共用）。
/// 終端命令は含めず全命令を走査させる（末尾到達で end=Ended）。
fn representative_instructions() -> Vec<Instruction> {
    vec![
        Instruction::SpeakerScope { n: 1 },
        Instruction::Text("a".into()),
        Instruction::Wait(Duration::from_millis(50)),
        Instruction::Surface(SurfaceArg::new("10".into())),
        Instruction::Wait(Duration::from_millis(100)),
        Instruction::NewLine(NewLineRatio::new(1.5)),
        Instruction::Clear,
        Instruction::SpeakerScope { n: 0 },
        Instruction::Text("b".into()),
    ]
}

/// 先頭に待ち命令がある場合でもその待ち時間が 0 へ潰れず保存される（R2.4）。
/// min 正規化（先頭待ちを 0 へ食う旧実装）を使っていないことの固定。第一級化後は、先頭待ちが
/// `duration` 付き Wait cue（`start_time=0.0`）として台本に残り、後続 Text はその分だけ遅れる。
#[test]
fn leading_wait_is_preserved_not_collapsed() {
    let compiled = compile(&[
        Instruction::Wait(Duration::from_millis(450)),
        Instruction::Text("hi".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2, "内容は先頭 Wait と Text の 2 件");
    // 期待値は同一の as_secs_f64() で計算（10 進リテラル直書きの表現誤差を排除）。
    let w = Duration::from_millis(450).as_secs_f64();
    // 先頭 Wait cue は 0.0 で保存され（潰れず）、待ち時間を duration に持つ。
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Wait,
        "先頭内容は Wait cue"
    );
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(cues[0].duration, w);
    // 後続 Text は先頭待ちが潰れず 450ms 後に発火する（0 へ正規化されない）。
    match command_of(&cues[1]) {
        CueCommand::Text(s) => assert_eq!(s, "hi"),
        other => panic!("expected Text, got {other:?}"),
    }
    assert_eq!(cues[1].start_time, w);
    assert_ne!(cues[1].start_time, 0.0);
}

/// 暗黙 per-char D と明示ウェイトの累積で発火時刻が単調に進む（R2.2/2.4/4.4）。
/// テキスト D 焼き込み・Wait 第一級化の後も、明示ウェイト累積が退行しない（4.4 非退行）。
/// 期待値は SAME `text_playback_duration`/`as_secs_f64()` 累積で計算（IEEE-754 加算を一致させる）。
#[test]
fn wait_accumulation_is_monotonic() {
    let compiled = compile(&[
        Instruction::Text("a".into()),
        Instruction::Wait(Duration::from_millis(50)),
        Instruction::Text("b".into()),
        Instruction::Wait(Duration::from_millis(100)),
        Instruction::Surface(SurfaceArg::new("1".into())),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    // 内容: Text("a") / Wait / Text("b") / Wait / Emote の 5 件。
    assert_eq!(cues.len(), 5);

    let d_a = text_playback_duration("a");
    let w50 = Duration::from_millis(50).as_secs_f64();
    let d_b = text_playback_duration("b");
    let w100 = Duration::from_millis(100).as_secs_f64();

    let t_text_a = 0.0_f64;
    let t_wait1 = t_text_a + d_a; // Text("a") の再生完了後
    let t_text_b = t_wait1 + w50; // 明示 \w[50] の累積
    let t_wait2 = t_text_b + d_b; // Text("b") の再生完了後
    let t_emote = t_wait2 + w100; // 明示 \w[100] の累積

    // Text("a")
    assert_eq!(command_of(&cues[0]), &CueCommand::Text("a".into()));
    assert_eq!(cues[0].start_time, t_text_a);
    assert_eq!(cues[0].duration, d_a);
    // Wait（\w[50]）— 第一級・duration に待ち時間
    assert_eq!(command_of(&cues[1]), &CueCommand::Wait);
    assert_eq!(cues[1].start_time, t_wait1);
    assert_eq!(cues[1].duration, w50);
    // Text("b")
    assert_eq!(command_of(&cues[2]), &CueCommand::Text("b".into()));
    assert_eq!(cues[2].start_time, t_text_b);
    assert_eq!(cues[2].duration, d_b);
    // Wait（\w[100]）
    assert_eq!(command_of(&cues[3]), &CueCommand::Wait);
    assert_eq!(cues[3].start_time, t_wait2);
    assert_eq!(cues[3].duration, w100);
    // Emote（瞬時・テキスト D と待ちの累積後に発火）
    assert_eq!(command_of(&cues[4]), &CueCommand::Emote { key: "1".into() });
    assert_eq!(cues[4].start_time, t_emote);
    assert_eq!(cues[4].duration, 0.0);

    // 非減少（構成的保証の固定）。
    for pair in cues.windows(2) {
        assert!(pair[0].start_time <= pair[1].start_time);
    }
}

/// 末尾到達で end=Ended（R6.3・task 3.1 の既定）。
#[test]
fn end_defaults_to_ended() {
    let compiled = compile(&[Instruction::Text("hi".into())]);
    assert_eq!(compiled.end, TalkEndReason::Ended);

    let compiled = compile(&[]);
    assert_eq!(compiled.end, TalkEndReason::Ended);
    assert!(compiled.sheet.is_empty());
}

/// `End` 検出で終端理由 `Ended` を確定し、以降の命令を発火列へ含めず破棄する
/// （終端切詰め・R6.1/6.5）。ukadoc `\e` = この後のスクリプトは実行・表示されない。
#[test]
fn end_truncates_following_instructions() {
    let compiled = compile(&[
        Instruction::Text("a".into()),
        Instruction::End,
        Instruction::Text("b".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    // 内容は "a" のみ（"b" は切り詰められる）。冒頭に ClearAll が前置される。
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "a"),
        other => panic!("expected Text(\"a\"), got {other:?}"),
    }
    assert_eq!(compiled.end, TalkEndReason::Ended);
}

/// `Quit` 検出で終端理由 `Quit` を確定し、以降の命令を破棄する（終端切詰め・R6.2/6.5）。
#[test]
fn quit_truncates_following_instructions_and_sets_quit() {
    let compiled = compile(&[
        Instruction::Text("a".into()),
        Instruction::Quit,
        Instruction::Text("b".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "a"),
        other => panic!("expected Text(\"a\"), got {other:?}"),
    }
    assert_eq!(compiled.end, TalkEndReason::Quit);
}

/// 終端命令なしで末尾まで到達した列は全命令が発火し end=Ended（末尾到達・R6.3）。
#[test]
fn no_terminal_keeps_all_instructions_and_ends() {
    let compiled = compile(&[Instruction::Text("a".into()), Instruction::Text("b".into())]);
    // 全命令が cue になる（"a"/"b" の 2 件）。冒頭に ClearAll が前置される。
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2);
    assert_eq!(compiled.end, TalkEndReason::Ended);
    assert_ne!(compiled.end, TalkEndReason::Quit);
}

/// 先行 cue のない `[Quit]` でも Quit が検出され、空 sheet＋end=Quit となる
/// （空 sheet でも Ended と判別可能なことの固定・下流 R6.2 の区別根拠）。
#[test]
fn bare_quit_yields_empty_sheet_with_quit_end() {
    let compiled = compile(&[Instruction::Quit]);
    assert!(compiled.sheet.is_empty());
    assert_eq!(compiled.end, TalkEndReason::Quit);
}

/// 決定的コンパイル: 同一命令列を複数回コンパイルすると常に同一の発火列・終端理由を得る
/// （R2.5・R9.4）。cue 数・各 index の actor/start_time（ビット同一）/payload・end が一致する。
#[test]
fn compile_is_deterministic_for_identical_input() {
    let instructions = representative_instructions();

    let first = compile(&instructions);
    let second = compile(&instructions);

    let a = first.sheet.cues();
    let b = second.sheet.cues();
    assert_eq!(a.len(), b.len(), "cue 数が回によって異なる");
    for (i, (ca, cb)) in a.iter().zip(b.iter()).enumerate() {
        assert!(
            cue_eq(ca, cb),
            "index {i} の cue が回によって異なる: {ca:?} != {cb:?}"
        );
    }
    assert_eq!(first.end, second.end, "終端理由が回によって異なる");
}

/// 不変条件（純粋コンパイル層 Postconditions/Invariants・NaN 放電）:
/// 生成される全 `start_time` は有限・非負であり、`sheet.cues()` 順に非減少
/// （`Duration` 由来の構成的保証）。待ちの無い連続 cue は同一時刻を共有し（非減少・非狭義増加）、
/// 待ちを挟む cue で時刻が増加することで `<=` の固定が意味を持つ。
#[test]
fn compiled_start_times_are_finite_non_negative_non_decreasing() {
    // 冒頭 ClearAll と先頭 Text は待ち無し＝同一時刻 0.0 を共有（非減少）、以降は
    // テキスト D と待ちで増加する。
    let compiled = compile(&[
        Instruction::Text("a".into()),
        Instruction::Surface(SurfaceArg::new("1".into())),
        Instruction::Wait(Duration::from_millis(50)),
        Instruction::Text("b".into()),
        Instruction::Wait(Duration::from_millis(100)),
        Instruction::Text("c".into()),
    ]);
    // 全 sheet（冒頭 ClearAll 含む）: ClearAll / Text("a") / Emote / Wait / Text("b") / Wait / Text("c")。
    let cues = compiled.sheet.cues();
    assert_eq!(cues.len(), 7);

    // 有限・非負（NaN/∞/負の放電）。
    for (i, cue) in cues.iter().enumerate() {
        assert!(
            cue.start_time.is_finite(),
            "index {i} の start_time が非有限"
        );
        assert!(cue.start_time >= 0.0, "index {i} の start_time が負");
        assert!(cue.duration.is_finite(), "index {i} の duration が非有限");
        assert!(cue.duration >= 0.0, "index {i} の duration が負");
    }

    // 非減少（構成的保証の固定）。
    for pair in cues.windows(2) {
        assert!(
            pair[0].start_time <= pair[1].start_time,
            "start_time が減少した: {} > {}",
            pair[0].start_time,
            pair[1].start_time
        );
    }

    // 待ち無しの冒頭 2 cue（ClearAll@0.0 と Text("a")@0.0）は同一時刻
    // （非狭義増加＝非減少の `<=` が真に効くことの固定）。
    assert_eq!(cues[0].start_time, cues[1].start_time);
    assert_eq!(cues[0].start_time, 0.0);
    // テキスト D を挟んだ cue は狭義増加（Text("a")@0.0 < Emote@D_a）。
    assert!(cues[1].start_time < cues[2].start_time);
    // 同一時刻に並ぶ Emote と Wait（ともに D_a）は非狭義増加（`<=` の固定・FIFO で Emote が先）。
    assert_eq!(cues[2].start_time, cues[3].start_time);
}

// ── task 5.2: D 焼き込み・Wait 第一級化・ClearAll 前置の behavioral 檻 ──

/// テキスト cue は算出した再生時間 D を envelope duration として保持する（N>0 で D>0・R4.1）。
/// 期待値は同一の `text_playback_duration` で導出（10 進直書きの表現誤差を排除）。
#[test]
fn text_cue_carries_playback_duration() {
    let compiled = compile(&[Instruction::Text("こんにちは".into())]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "内容は Text cue 1 件のみ");
    let expected = text_playback_duration("こんにちは"); // 5 char × 50ms = 0.25s
    assert!(expected > 0.0, "N>0 の再生時間は正");
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "こんにちは"),
        other => panic!("expected Text, got {other:?}"),
    }
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(
        cues[0].duration, expected,
        "テキスト cue の duration は text_playback_duration の値（R4.1）"
    );
}

/// テキスト cue の直後に別の cue が続くと、後続 cue はテキスト再生完了後（`text_start + D`）へ
/// 焼き込まれる（R4.2）。`Text("ab") Surface("7")` → Emote は 2×50ms 後に発火する。
#[test]
fn cue_after_text_fires_after_text_playback_completes() {
    let compiled = compile(&[
        Instruction::Text("ab".into()),
        Instruction::Surface(SurfaceArg::new("7".into())),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2, "内容は Text と Emote の 2 件");
    let d = text_playback_duration("ab"); // 2 char × 50ms = 0.1s
    // [0] Text("ab")@0.0（duration=D）。
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "ab"),
        other => panic!("expected Text, got {other:?}"),
    }
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(cues[0].duration, d);
    // [1] Emote@D（テキスト再生完了後に発火・R4.2）。
    match command_of(&cues[1]) {
        CueCommand::Emote { key } => assert_eq!(key, "7"),
        other => panic!("expected Emote, got {other:?}"),
    }
    assert_eq!(
        cues[1].start_time, d,
        "後続 cue はテキスト再生完了後（text_start + D）へ焼き込まれる（R4.2）"
    );
    assert_eq!(cues[1].duration, 0.0, "Emote は瞬時（duration=0）");
}

/// 明示ウェイトは offset へ吸収して消すのでなく、action を持たず duration のみを持つ第一級
/// `CueCommand::Wait` cue として台本に残る（R5.1）。かつ後続 Text はその待ち時間分だけ遅れる
/// （`D_text1 + wait`・R4.4）。
#[test]
fn explicit_wait_between_texts_is_first_class_and_delays_following_text() {
    let compiled = compile(&[
        Instruction::Text("aa".into()),
        Instruction::Wait(Duration::from_millis(500)),
        Instruction::Text("bb".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 3, "内容は Text / Wait / Text の 3 件");
    let d1 = text_playback_duration("aa"); // 0.1s
    let w = Duration::from_millis(500).as_secs_f64(); // 0.5s
    // [1] 中間に第一級 Wait cue（吸収されず台本に残る）。
    assert_eq!(
        command_of(&cues[1]),
        &CueCommand::Wait,
        "明示ウェイトは第一級 Wait cue として残る（吸収しない・R5.1）"
    );
    assert_eq!(
        cues[1].start_time, d1,
        "Wait cue はテキスト再生完了後に置かれる"
    );
    assert_eq!(
        cues[1].duration, w,
        "待ち時間は envelope duration が担う（action なし）"
    );
    // [2] 2 つ目 Text は D_text1 + wait 分だけ遅れる（R4.4）。
    match command_of(&cues[2]) {
        CueCommand::Text(s) => assert_eq!(s, "bb"),
        other => panic!("expected Text, got {other:?}"),
    }
    let t_b = d1 + w;
    assert_eq!(
        cues[2].start_time, t_b,
        "後続 Text は暗黙 D と明示待ちの累積分だけ遅れる（R4.4）"
    );
}

/// 末尾（単独）の明示ウェイトも offset へ吸収されず Wait cue として台本に残り、台本のみから
/// talk の全時間範囲（`max(start_time + duration)`）が復元可能である（自己完結した楽譜・R5.3）。
#[test]
fn trailing_explicit_wait_remains_in_sheet_and_extends_extent() {
    let compiled = compile(&[
        Instruction::Text("abc".into()),
        Instruction::Wait(Duration::from_millis(800)),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2, "内容は Text と末尾 Wait の 2 件");
    let d = text_playback_duration("abc"); // 0.15s
    let w = Duration::from_millis(800).as_secs_f64(); // 0.8s
    // 末尾 Wait cue が吸収されず台本に残る（R5.3）。
    assert_eq!(
        command_of(&cues[1]),
        &CueCommand::Wait,
        "末尾の明示ウェイトも第一級 Wait cue として台本に残る（R5.3）"
    );
    assert_eq!(cues[1].start_time, d);
    assert_eq!(cues[1].duration, w);
    // 台本のみから talk 全時間範囲が復元可能（absolute_start_time 未刻印は 0.0 起点）。
    assert_eq!(
        compiled.sheet.absolute_end_time(),
        d + w,
        "末尾待ちを含めた全時間範囲が台本のみから復元可能（R5.3）"
    );
}

/// 内容を持つ台本の先頭へ ClearAll を単一前置する（#6・R6.1/6.2）。書き込むスコープ数に
/// 依らず ClearAll はちょうど 1 件（compile は残存スコープを列挙できないため全消し 1 件で表現）。
#[test]
fn clear_all_is_prepended_once_regardless_of_scope_count() {
    // 3 スコープへ書き込む talk。
    let compiled = compile(&[
        Instruction::SpeakerScope { n: 0 },
        Instruction::Text("a".into()),
        Instruction::SpeakerScope { n: 1 },
        Instruction::Text("b".into()),
        Instruction::SpeakerScope { n: 2 },
        Instruction::Text("c".into()),
    ]);
    let cues = compiled.sheet.cues();
    // 先頭に単一 ClearAll@0.0/duration0。
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::ClearAll,
        "先頭 cue は ClearAll"
    );
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(cues[0].duration, 0.0);
    // スコープ数（ここでは 3）に依らず ClearAll はちょうど 1 件。
    let clear_all_count = cues
        .iter()
        .filter(|c| command_of(c) == &CueCommand::ClearAll)
        .count();
    assert_eq!(
        clear_all_count, 1,
        "ClearAll はスコープ数に依らず単一前置（6.1/6.2）"
    );
}

/// 内容 cue を生成しない台本（リテラル空・裸の終端・無視タグのみ）は空 sheet のままで
/// ClearAll を前置しない——drive の `is_empty()` 即時 TalkDone 契約を保ち、ドライブ配線
/// （task 7.x）を本 task で先取りしない（消すべき前 talk のテキストも存在しない）。
#[test]
fn empty_content_talk_gets_no_clear_all_prepended() {
    assert!(
        compile(&[]).sheet.is_empty(),
        "リテラル空 script は空 sheet（ClearAll 前置なし）"
    );
    assert!(
        compile(&[Instruction::Quit]).sheet.is_empty(),
        "先行 cue のない裸 Quit は空 sheet（ClearAll 前置なし）"
    );
    assert!(
        compile(&[
            Instruction::Raw("\\0".into()),
            Instruction::Raw("\\1".into()),
        ])
        .sheet
        .is_empty(),
        "破棄される Raw のみの script は空 sheet（ClearAll 前置なし）"
    );
}

// ── task 4.2: append_epilogue（末尾 carrier cue 付加・純関数）の behavioral 檻 ──
// （design C12・Testing Strategy §5・R3.4）

use areka_talk::EpilogueCommand;

/// 台本末尾の占有 horizon（`max(start_time + duration)`）を素朴に算出するテスト用参照。
/// `append_epilogue` が返す carrier cue の `start_time` 期待値、および horizon 不延長の
/// 検証に使う（`CueSheet::absolute_end_time` はアンカー未刻印で同値だが、独立導出で固定する）。
fn relative_horizon(sheet: &CueSheet) -> f64 {
    sheet
        .cues()
        .iter()
        .map(|c| c.start_time + c.duration)
        .fold(0.0_f64, f64::max)
}

/// 空 epilogue は恒等: 台本を一切変えない（既存経路完全不変・R3.4）。
/// cue 数・各 cue のフィールド等価を固定し、`epilogue.is_empty()` が perfect no-op であることを保証する。
#[test]
fn append_epilogue_empty_is_identity() {
    let compiled = compile(&[
        Instruction::Text("hi".into()),
        Instruction::Surface(SurfaceArg::new("1".into())),
    ]);
    let before = compiled.sheet.cues().to_vec();
    let after = append_epilogue(compiled.sheet, &[]);
    let after_cues = after.cues();
    assert_eq!(
        after_cues.len(),
        before.len(),
        "空 epilogue は cue 数を変えない（恒等）"
    );
    for (i, (a, b)) in after_cues.iter().zip(before.iter()).enumerate() {
        assert!(
            cue_eq(a, b),
            "index {i} の cue が空 epilogue で変化した: {a:?} != {b:?}"
        );
    }
}

/// 空 sheet＋単一 epilogue: `start_time=0.0`・`duration=0.0`・`actor="0"` の carrier cue 1 個
/// （空 sheet の horizon は 0.0・design C12）。
#[test]
fn append_epilogue_on_empty_sheet_yields_single_cue_at_zero() {
    let empty = CueSheet::new(Vec::new());
    assert!(empty.is_empty(), "前提: 空 sheet");
    let epilogue = [EpilogueCommand {
        name: "areka.prop.set".into(),
        tokens: vec!["areka.boot.count".into(), "1".into()],
    }];
    let sheet = append_epilogue(empty, &epilogue);
    let cues = sheet.cues();
    assert_eq!(cues.len(), 1, "空 sheet＋1 epilogue → 1 cue");
    assert_eq!(cues[0].start_time, 0.0, "空 sheet の horizon は 0.0");
    assert_eq!(cues[0].duration, 0.0, "carrier cue は瞬時（duration 0）");
    assert_eq!(cues[0].actor.as_str(), "0", "carrier cue の actor は \"0\"");
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("areka.prop.set", vec!["areka.boot.count", "1"])),
        "payload は command_carrier(name, tokens) の正準形"
    );
}

/// 非空 epilogue: 末尾 offset（`max(start_time+duration)`）へ carrier cue が付加される。
/// `duration=0.0` かつ horizon が延長されない（zero-duration ゆえ・design C12）。
#[test]
fn append_epilogue_appends_carrier_at_tail_horizon_without_extending() {
    let compiled = compile(&[
        Instruction::Text("abc".into()),
        Instruction::Wait(Duration::from_millis(800)),
    ]);
    let horizon_before = relative_horizon(&compiled.sheet);
    assert!(horizon_before > 0.0, "前提: 非空台本の horizon は正");
    let len_before = compiled.sheet.cues().len();

    let epilogue = [EpilogueCommand {
        name: "areka.prop.set".into(),
        tokens: vec!["areka.boot.count".into(), "1".into()],
    }];
    let sheet = append_epilogue(compiled.sheet, &epilogue);
    let cues = sheet.cues();
    assert_eq!(cues.len(), len_before + 1, "carrier cue が 1 個付加される");

    // 末尾の carrier cue（同時刻の既存要素の後・安定ソート FIFO）。
    let last = cues.last().expect("非空");
    assert_eq!(
        last.start_time, horizon_before,
        "carrier cue の start_time は既存 cues の max(start+duration)"
    );
    assert_eq!(last.duration, 0.0, "carrier cue は duration 0");
    assert_eq!(last.actor.as_str(), "0");
    assert_eq!(
        command_of(last).as_command_carrier(),
        Some(("areka.prop.set", vec!["areka.boot.count", "1"])),
    );

    // horizon 不延長: zero-duration cue は max(start+duration) を増やさない。
    assert_eq!(
        relative_horizon(&sheet),
        horizon_before,
        "zero-duration carrier cue は占有 horizon を延長しない（TalkDone を遅らせない）"
    );
}

/// 同時刻（horizon と同じ `start_time`）の既存末尾要素（選択待ち barrier 等）に対し、
/// carrier cue は **安定ソート FIFO** で **後ろ** に並ぶ＝barrier 解決後・horizon 到達 tick で
/// 発火する（design C12・R3.4）。同一 `at` で既存要素が先・epilogue が後を固定する。
#[test]
fn append_epilogue_stable_sorts_after_same_time_barrier() {
    // horizon=1.0 の台本を直接構築する: Text@0.0(duration 1.0) と barrier@1.0(duration 0.0)。
    // horizon = max(0+1, 1+0) = 1.0。barrier の start_time が epilogue の付加時刻と同一になる。
    let text = Cue {
        actor: ActorKey::from("0".to_string()),
        start_time: 0.0,
        payload: CuePayload::Command(CueCommand::Text("hi".into())),
        duration: 1.0,
    };
    let barrier = Cue {
        actor: ActorKey::from("0".to_string()),
        start_time: 1.0,
        payload: CuePayload::Barrier(BarrierKind::WaitForChoice { timeout: None }),
        duration: 0.0,
    };
    let sheet = CueSheet::new(vec![text, barrier]);
    assert_eq!(relative_horizon(&sheet), 1.0, "前提: horizon=1.0");

    let epilogue = [EpilogueCommand {
        name: "areka.prop.set".into(),
        tokens: vec!["areka.boot.count".into(), "1".into()],
    }];
    let sheet = append_epilogue(sheet, &epilogue);
    let cues = sheet.cues();
    assert_eq!(cues.len(), 3, "Text / barrier / carrier の 3 件");

    // 同一 at=1.0 で barrier が先・carrier が後（安定ソート FIFO＝barrier 解決後に発火）。
    assert_eq!(cues[1].start_time, 1.0);
    assert!(
        matches!(
            &cues[1].payload,
            CuePayload::Barrier(BarrierKind::WaitForChoice { .. })
        ),
        "同時刻群の先頭は既存 barrier（安定ソートで epilogue より前）"
    );
    assert_eq!(cues[2].start_time, 1.0, "carrier cue も同一 at=1.0");
    assert_eq!(
        command_of(&cues[2]).as_command_carrier(),
        Some(("areka.prop.set", vec!["areka.boot.count", "1"])),
        "carrier cue は barrier の後ろへ安定ソートされる（barrier 解決後・horizon 到達 tick で発火）"
    );
    // horizon は延長されない。
    assert_eq!(relative_horizon(&sheet), 1.0);
}

/// 複数 epilogue コマンドは各々 1 carrier cue へ写像され、全て同一の末尾 horizon へ付加される
/// （記述順を保つ・design C12）。
#[test]
fn append_epilogue_maps_each_command_to_one_carrier_cue() {
    let compiled = compile(&[Instruction::Text("x".into())]);
    let horizon = relative_horizon(&compiled.sheet);
    let epilogue = [
        EpilogueCommand {
            name: "areka.prop.set".into(),
            tokens: vec!["areka.boot.count".into(), "1".into()],
        },
        EpilogueCommand {
            name: "areka.prop.set".into(),
            tokens: vec!["areka.vanish.count".into(), "0".into()],
        },
    ];
    let sheet = append_epilogue(compiled.sheet, &epilogue);
    let cues = sheet.cues();
    // 末尾 2 件が記述順の carrier cue。
    let tail = &cues[cues.len() - 2..];
    assert_eq!(
        command_of(&tail[0]).as_command_carrier(),
        Some(("areka.prop.set", vec!["areka.boot.count", "1"])),
    );
    assert_eq!(
        command_of(&tail[1]).as_command_carrier(),
        Some(("areka.prop.set", vec!["areka.vanish.count", "0"])),
    );
    for cue in tail {
        assert_eq!(
            cue.start_time, horizon,
            "全 carrier cue が同一末尾 horizon へ付加される"
        );
        assert_eq!(cue.duration, 0.0);
    }
}
