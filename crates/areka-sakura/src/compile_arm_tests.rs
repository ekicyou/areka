use super::*;
use crate::duration::text_playback_duration;
use crate::sysvar::SystemVarSnapshot;
use areka_parsers::sakura::{NewLineRatio, SurfaceArg};
use super::test_support::{assert_clear_all_prefix_and_rest, command_of, compile, cue_eq};

/// サーフェス切替命令の引数を解釈・変換せず値のまま転写する（R3.1/3.2）。
/// `"0,1,foo"` はカンマ区切りを一切パースせず `Emote{key}` へバイト完全転写される。
#[test]
fn surface_arg_is_transcribed_opaquely() {
    let compiled = compile(&[Instruction::Surface(SurfaceArg::new("0,1,foo".into()))]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::Emote { key } => assert_eq!(key, "0,1,foo"),
        other => panic!("expected Emote, got {other:?}"),
    }
}

/// バルーン面切替命令の引数を解釈・変換せず値のまま `BalloonSurface{key}` へ
/// バイト完全一致で転写する（不透明写像・R3.1）。名前形（非 ASCII）・数値形・非表示
/// センチネル `-1` のいずれも数値化・alias 解決せず、`Surface`→`Emote` と完全対称に扱う。
/// `BalloonSurface` が catch-all（M-boot 外タグの無視ログ）へ落ちて破棄されないことの固定。
#[test]
fn balloon_surface_arg_is_transcribed_opaquely() {
    // 名前形（非 ASCII）: 整数化せず文字列のまま転写。
    let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new(
        "バルーン１".into(),
    ))]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        cues.len(),
        1,
        "BalloonSurface が cue を生成しない（破棄された）"
    );
    match command_of(&cues[0]) {
        CueCommand::BalloonSurface { key } => assert_eq!(key, "バルーン１"),
        other => panic!("expected BalloonSurface, got {other:?}"),
    }

    // 数値形: 数値化・展開せず文字列のまま転写。
    let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new("10".into()))]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::BalloonSurface { key } => assert_eq!(key, "10"),
        other => panic!("expected BalloonSurface, got {other:?}"),
    }

    // 非表示センチネル `-1`: パース段階同様に数値化せず不透明転写。
    let compiled = compile(&[Instruction::BalloonSurface(SurfaceArg::new("-1".into()))]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::BalloonSurface { key } => assert_eq!(key, "-1"),
        other => panic!("expected BalloonSurface, got {other:?}"),
    }
}

/// バルーン面切替命令の追加後も、既存のシェル面切替 `Surface`→`Emote` 写像は不変
/// （additive・R3.1 既存写像非変更）。scope/start_time 転写も従来通り。
#[test]
fn surface_to_emote_mapping_is_unchanged_by_balloon_arm() {
    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::Surface(SurfaceArg::new("0,1,foo".into())),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].actor.as_str(), "1");
    match command_of(&cues[0]) {
        CueCommand::Emote { key } => assert_eq!(key, "0,1,foo"),
        other => panic!("expected Emote, got {other:?}"),
    }
}

/// 話者スコープ切替で actor が転写され、未指定開始は既定 "0"（R5.1/5.2/5.3）。
#[test]
fn speaker_scope_is_transcribed_to_actor() {
    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::Text("a".into()),
        Instruction::SpeakerScope { n: 0 },
        Instruction::Surface(SurfaceArg::new("s".into())),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2);
    assert_eq!(cues[0].actor.as_str(), "1");
    assert_eq!(cues[1].actor.as_str(), "0");

    // SpeakerScope を先行させない talk は既定 "0"（内容先頭 Text の actor）。
    let compiled = compile(&[Instruction::Text("hi".into())]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues[0].actor.as_str(), "0");
}

/// NewLine/Clear の写像（DD-9・R4.2/4.3）。
#[test]
fn newline_and_clear_map_to_commands() {
    let compiled = compile(&[
        Instruction::NewLine(NewLineRatio::new(1.5)),
        Instruction::Clear,
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2);
    match command_of(&cues[0]) {
        CueCommand::NewLine { ratio } => assert_eq!(*ratio, 1.5_f32),
        other => panic!("expected NewLine, got {other:?}"),
    }
    // 対象スコープのみ消去の `Clear`（冒頭前置の全消去 `ClearAll` とは別コマンド）。
    assert_eq!(command_of(&cues[1]), &CueCommand::Clear);
}

/// 除外集合の Raw-only 対置換（catch-all の意図的更新の檻・R8.2/8.3・Testing Strategy 項目 6）。
///
/// task 4.1（Choice/Cursor）・task 4.2（Move/GenericCommand/SystemVar）で 5 語彙が専用アームへ
/// **卒業**した結果、compile の catch-all が無視する除外集合は `Raw`＋`#[non_exhaustive]` の未知
/// variant のみへ縮小された（`Instruction` は別 crate の `#[non_exhaustive]` ゆえ catch-all は
/// 構造上必須・未知 variant は防御経路だが workspace 内に合成手段がなく直接は載せられない）。
/// 本檻はその縮小後の除外集合を **Raw-only** で固定し、4.1/4.2 の卒業と決して矛盾させない:
/// (1) `Raw` は cue を 0 個生成する（無視ログ・非 panic・従来挙動維持・R8.2）、
/// (2) 卒業した 5 語彙（Choice / Cursor / Move / GenericCommand / SystemVar）は除外集合に
///     **含まれない**＝各々 cue を生成する（本檻は決してこれらが「無視される」とは主張しない・
///     R8.3）。各語彙の写像詳細は個別 behavioral 檻が担い、ここでは除外集合の境界のみを固定する。
#[test]
fn catch_all_ignored_set_is_raw_only() {
    use areka_parsers::sakura::{Choice, MoveArgs};

    // (1) Raw は除外集合ゆえ 0 cue。中途に Raw を挟んでも内容は後続 Text のみ（ClearAll 前置）。
    let compiled = compile(&[
        Instruction::Raw("\\?".into()),
        Instruction::Text("hi".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "Raw は cue を生成しない（除外集合は Raw のみ）");
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "hi"),
        other => panic!("expected Text, got {other:?}"),
    }
    // Raw 単独（後続なし）は空 sheet（内容 cue ゼロゆえ ClearAll 前置もなし・R8.2）。
    assert!(
        compile(&[Instruction::Raw("\\0".into())]).sheet.is_empty(),
        "Raw のみの台本は空 sheet（除外集合の 0 cue を単独でも固定）"
    );

    // (2) 卒業した 5 語彙は除外集合の外＝各々 cue を生成する（sheet 非空）。ここでこれらが
    //     「無視される」と主張することは 4.1/4.2 と矛盾するため、本檻は生成側のみを固定する
    //     （R8.3 対置換）。詳細写像は個別 behavioral 檻（`*_maps_to_*` 等）が担う。
    let graduated: [Instruction; 5] = [
        Instruction::Choice(Choice {
            disp: "はい".into(),
            target: "OnYes".into(),
            references: vec![],
        }),
        Instruction::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        Instruction::Move(MoveArgs {
            args: vec!["100".into()],
        }),
        Instruction::GenericCommand {
            name: "raise".into(),
            raw_args: vec!["OnBoot".into()],
        },
        Instruction::SystemVar("username".into()),
    ];
    for instruction in graduated {
        assert!(
            !compile(std::slice::from_ref(&instruction)).sheet.is_empty(),
            "卒業語彙は除外集合に含まれず cue を生成する（4.1/4.2 の卒業を明示）: {instruction:?}"
        );
    }
}

/// task 4.2 で Move/SystemVar/GenericCommand は専用アームへ卒業し cue を発行する（R4.1/7.1/8.2）。
/// catch-all に残る除外は `Raw`（＋未知 variant）のみで、これは従来どおり 0 cue・非 panic
/// （R8.2/8.3/11.2）。卒業アームと Raw が交錯しても各々の写像・破棄が一貫することを固定する。
/// 各アームの詳細写像は個別の behavioral 檻（`move_maps_to_command_carrier_*` 他）が担う。
#[test]
fn graduated_arms_emit_while_raw_remains_ignored() {
    use areka_parsers::sakura::MoveArgs;

    let compiled = compile(&[
        Instruction::Move(MoveArgs {
            args: vec!["100".into(), "200".into()],
        }),
        Instruction::SystemVar("username".into()),
        Instruction::GenericCommand {
            name: "raise".into(),
            raw_args: vec!["OnBoot".into()],
        },
        Instruction::Raw("\\?".into()),
        Instruction::Text("tail".into()),
    ]);
    // 内容: carrier(move) / Text(username 既定値) / carrier(raise) / Text(tail) の 4 件。
    // Raw のみ破棄される（冒頭に ClearAll が前置される）。
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 4, "卒業アーム 3 種＋末尾 Text の 4 cue（Raw のみ破棄）");
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("move", vec!["100", "200"])),
        "Move はキャリア cue へ卒業（R4.1）"
    );
    match command_of(&cues[1]) {
        CueCommand::Text(s) => assert_eq!(s, "ユーザーさん", "SystemVar は Text cue へ卒業（R7.1/7.4）"),
        other => panic!("expected Text, got {other:?}"),
    }
    assert_eq!(
        command_of(&cues[2]).as_command_carrier(),
        Some(("raise", vec!["OnBoot"])),
        "GenericCommand はキャリア cue へ卒業（R4.1/8.2）"
    );
    match command_of(&cues[3]) {
        CueCommand::Text(s) => assert_eq!(s, "tail", "Raw は破棄され末尾 Text のみ残る"),
        other => panic!("expected Text(\"tail\"), got {other:?}"),
    }
    // 終端命令を含まないため末尾到達で Ended。
    assert_eq!(compiled.end, TalkEndReason::Ended);
}

// ── task 4.1: Choice/Cursor アーム写像の behavioral 檻 ──

/// fixture メインメニュー script 断片（`menu.pasta:15` 相当）を直入力し、`\q`/`\_l` が
/// 期待どおりの Choice/Cursor cue へ写像されることを固定する（R1.1/1.2/1.5/1.6・R3.1/3.4）。
///
/// 断片（さくらスクリプト）:
/// `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]\n\q[エモの位置調整,Onエモの位置調整メニュー]\_l[5em,2lh]\q[閉じる,Onメニュー閉じる]`
/// を parse 済み `Instruction` 列として直入力する（scope="1"＝エモ）。
///
/// 検証: (1) Choice cue は `id=target`（第 2 引数）・`text=disp`（第 1 引数）を欠落なく持つ、
/// (2) Cursor cue が `\_l[5em,2lh]` から `x="5em"`/`y="2lh"` で発行される、(3) 記述順が
/// 台本内順序として保存される（Choice/NewLine/Choice/Cursor/Choice が交互のまま並ぶ）、
/// (4) 各 cue が現在 scope "1" へ帰属し瞬時（duration 0）。barrier（task 4.2）・完全 at 檻
/// （task 4.4）はここでは主張しない。
#[test]
fn choice_and_cursor_arms_map_menu_fragment_in_description_order() {
    use areka_parsers::sakura::Choice;

    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::Choice(Choice {
            disp: "おしゃべり頻度".into(),
            target: "Onおしゃべり頻度メニュー".into(),
            references: vec![],
        }),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Choice(Choice {
            disp: "エモの位置調整".into(),
            target: "Onエモの位置調整メニュー".into(),
            references: vec![],
        }),
        Instruction::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        Instruction::Choice(Choice {
            disp: "閉じる".into(),
            target: "Onメニュー閉じる".into(),
            references: vec![],
        }),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    // 内容は Choice / NewLine / Choice / Cursor / Choice の 5 件（記述順保存）＋末尾に選択待ち
    // barrier 1 件（`\q` を含む台本ゆえ task 4.2 で発行・R2.1/2.2）。
    assert_eq!(cues.len(), 6, "内容 cue 5 件＋末尾 barrier 1 件");

    // [0] Choice(頻度): id=target・text=disp。
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Choice {
            id: "Onおしゃべり頻度メニュー".into(),
            text: "おしゃべり頻度".into(),
            references: vec![],
        },
        "Choice は id=target・text=disp で写像される（R1.2）"
    );
    // [1] NewLine（既存写像・記述順の中間に保存）。
    match command_of(&cues[1]) {
        CueCommand::NewLine { ratio } => assert_eq!(*ratio, 1.0_f32),
        other => panic!("expected NewLine, got {other:?}"),
    }
    // [2] Choice(位置調整)。
    assert_eq!(
        command_of(&cues[2]),
        &CueCommand::Choice {
            id: "Onエモの位置調整メニュー".into(),
            text: "エモの位置調整".into(),
            references: vec![],
        }
    );
    // [3] Cursor(5em,2lh): 不透明転写（単位付きの区別を失わない・R3.1）。
    assert_eq!(
        command_of(&cues[3]),
        &CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        "Cursor は x/y を不透明転写する（R3.1）"
    );
    // [4] Choice(閉じる)。
    assert_eq!(
        command_of(&cues[4]),
        &CueCommand::Choice {
            id: "Onメニュー閉じる".into(),
            text: "閉じる".into(),
            references: vec![],
        }
    );
    // [5] 末尾に選択待ち barrier（全 choice cue より後・R2.2）。
    assert_eq!(
        barrier_of(&cues[5]),
        &BarrierKind::WaitForChoice { timeout: None },
        "`\\q` を含む台本の末尾に選択待ち barrier（R2.1/2.2）"
    );

    // 全 cue が現在 scope "1" へ帰属し瞬時（duration 0）。barrier も末尾 scope 1・duration 0。
    for (i, cue) in cues.iter().enumerate() {
        assert_eq!(cue.actor.as_str(), "1", "index {i} は scope 1 帰属（R3.4）");
        assert_eq!(cue.duration, 0.0, "index {i} は瞬時（duration 0）");
    }
}

/// `\_l[,]` 相当（x・y 双方空）でも Cursor cue が無条件に発行される（記述の存在を台本から
/// 失わせない・R3.5）。空は代表形と別物（区別保持）。
#[test]
fn cursor_double_empty_still_emits() {
    let compiled = compile(&[Instruction::Cursor {
        x: "".into(),
        y: "".into(),
    }]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "双方空でも Cursor cue は発行される（R3.5）");
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Cursor {
            x: "".into(),
            y: "".into(),
        },
        "双方空の Cursor も欠落させず発行（区別保持・R3.5）"
    );
    assert_eq!(cues[0].duration, 0.0, "Cursor は瞬時（duration 0）");
}

// ── task 4.2: Move/GenericCommand/SystemVar アーム＋barrier 発行の behavioral 檻 ──

/// `Cue::payload` から `BarrierKind` を取り出すヘルパ（barrier 檻用）。
fn barrier_of(cue: &Cue) -> &BarrierKind {
    match &cue.payload {
        CuePayload::Barrier(kind) => kind,
        other => panic!("expected CuePayload::Barrier, got {other:?}"),
    }
}

/// `Move(MoveArgs)` は `command_carrier("move", args)` へ写像される（`\!` 全体が第一級で
/// 台本に載る・R4.1/4.2）。空トークン（省略スロット）も欠落なく保持される。瞬時（duration 0）。
#[test]
fn move_maps_to_command_carrier_preserving_empty_tokens() {
    use areka_parsers::sakura::MoveArgs;

    // fixture の `\![move,-353,,,0,base,base]` 相当（空トークン 2 個を保つ 6 トークン）。
    let compiled = compile(&[Instruction::Move(MoveArgs {
        args: vec![
            "-353".into(),
            "".into(),
            "".into(),
            "0".into(),
            "base".into(),
            "base".into(),
        ],
    })]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "move は単一の汎用キャリア cue を生成する");
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some((
            "move",
            vec!["-353", "", "", "0", "base", "base"]
        )),
        "Move → command_carrier(\"move\", args)（空トークン保持・R4.1/4.2）"
    );
    assert_eq!(cues[0].duration, 0.0, "汎用キャリアは瞬時（duration 0）");
}

/// `GenericCommand{name,raw_args}` は `command_carrier(name, raw_args)` へ写像される（R4.1/4.2）。
/// 引数を持つ形も、`\![*]` 単独形（raw_args 空）も台本に第一級で載る（R8.2 卒業）。
#[test]
fn generic_command_maps_to_command_carrier_including_bare_form() {
    // 引数付き `\![raise,OnBoot]`。
    let compiled = compile(&[Instruction::GenericCommand {
        name: "raise".into(),
        raw_args: vec!["OnBoot".into()],
    }]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("raise", vec!["OnBoot"])),
        "GenericCommand → command_carrier(name, raw_args)（R4.1）"
    );
    assert_eq!(cues[0].duration, 0.0, "瞬時（duration 0）");

    // 単独形 `\![vanish]`（raw_args 空）でもキャリア cue が発行される（R8.2 卒業）。
    let compiled = compile(&[Instruction::GenericCommand {
        name: "vanish".into(),
        raw_args: vec![],
    }]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "単独形 `\\![*]` もキャリア cue を生成する");
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("vanish", vec![])),
        "単独形は空トークンのキャリアとして載る（R8.2）"
    );
}

/// `SystemVar(name)` は値ありスナップショットの値を `Text` cue へ写像する（R7.1/7.2）。
/// `duration = text_playback_duration(展開文字列)`・直後 offset += D で後続 cue を整列する。
/// 展開結果は通常テキストと同格（独立 cue・隣接 Text と併合しない）。
#[test]
fn system_var_present_maps_to_text_cue_with_playback_duration() {
    let mut vars = SystemVarSnapshot::default();
    vars.insert("username", "アヒル");
    // `[SystemVar("username"), Surface("1")]`: 展開 Text 後に Emote が D 分遅れて発火する。
    let compiled = super::compile(
        &[
            Instruction::SystemVar("username".into()),
            Instruction::Surface(SurfaceArg::new("1".into())),
        ],
        &vars,
    );
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2, "展開 Text と Emote の 2 件");
    let d = text_playback_duration("アヒル"); // 3 char × 50ms
    // [0] 展開値の Text cue（通常テキスト同格・R7.2）。
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "アヒル", "スナップショット値へ展開（R7.1）"),
        other => panic!("expected Text, got {other:?}"),
    }
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(
        cues[0].duration, d,
        "duration = text_playback_duration(展開文字列)（R7.2）"
    );
    // [1] 後続 Emote は展開テキスト再生完了後（offset += D）へ整列する。
    assert_eq!(
        command_of(&cues[1]),
        &CueCommand::Emote { key: "1".into() }
    );
    assert_eq!(cues[1].start_time, d, "SystemVar 由来 D の分だけ後続が遅れる（R7.2）");
}

/// `username` 欠落スナップショット → 既定値 `ユーザーさん` の Text cue（R7.4・生の `%username`
/// を露出しない）。既定 snapshot（値なし）を渡す薄いブリッジ経由で確認する。
#[test]
fn system_var_missing_username_expands_to_default_text() {
    let compiled = compile(&[Instruction::SystemVar("username".into())]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "ユーザーさん", "既定値へ展開（R7.4）"),
        other => panic!("expected Text, got {other:?}"),
    }
    assert_eq!(
        cues[0].duration,
        text_playback_duration("ユーザーさん"),
        "既定値も通常テキストと同一の再生時間規則（R7.2）"
    );
}

/// M1 未対応のシステム変数名 → 元の `%名前` を Text として素通し出力（R7.5・情報を失わない縮退）。
#[test]
fn system_var_unsupported_passes_through_as_text() {
    let compiled = compile(&[Instruction::SystemVar("selfname".into())]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1);
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "%selfname", "元の `%名前` を素通し（R7.5）"),
        other => panic!("expected Text, got {other:?}"),
    }
}

/// `\q` を 1 個以上含む台本には選択待ち barrier をちょうど 1 個、全 choice cue より後（最終
/// offset）へ発行する（R2.1/2.2/2.6）。タイムアウトは指定しない（`timeout:None`＝未指定＝
/// 下流の既定値へ委譲・DD-8）。
#[test]
fn talk_with_choice_appends_single_barrier_after_all_choices() {
    use areka_parsers::sakura::Choice;

    let compiled = compile(&[
        Instruction::Choice(Choice {
            disp: "はい".into(),
            target: "OnYes".into(),
            references: vec![],
        }),
        Instruction::Choice(Choice {
            disp: "いいえ".into(),
            target: "OnNo".into(),
            references: vec![],
        }),
    ]);
    // 内容: Choice / Choice / Barrier の 3 件（barrier は末尾）。
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 3, "choice 2 個＋barrier 1 個");
    // barrier はちょうど 1 個（R2.1）。
    let barrier_count = cues
        .iter()
        .filter(|c| matches!(c.payload, CuePayload::Barrier(_)))
        .count();
    assert_eq!(barrier_count, 1, "選択待ち barrier はちょうど 1 個（R2.1）");
    // barrier は全 choice cue より後（R2.2）。
    assert!(
        matches!(command_of(&cues[0]), CueCommand::Choice { .. }),
        "[0] は choice"
    );
    assert!(
        matches!(command_of(&cues[1]), CueCommand::Choice { .. }),
        "[1] は choice"
    );
    assert_eq!(
        barrier_of(&cues[2]),
        &BarrierKind::WaitForChoice { timeout: None },
        "barrier は末尾・WaitForChoice{{timeout:None}}（R2.2/2.6）"
    );
    // barrier は最終 offset（choice が瞬時なので 0.0）・duration 0.0。
    assert_eq!(cues[2].start_time, 0.0, "barrier は最終 offset（R2.2）");
    assert_eq!(cues[2].duration, 0.0, "barrier の duration は 0.0");
}

/// テキスト先行のメニューでも barrier は全 choice cue より後（テキスト再生完了後の最終 offset）へ
/// 置かれる（R2.2）。`Text("ab") \q[..]` → barrier は text D の位置。
#[test]
fn barrier_is_placed_at_final_offset_after_leading_text() {
    use areka_parsers::sakura::Choice;

    let compiled = compile(&[
        Instruction::Text("ab".into()),
        Instruction::Choice(Choice {
            disp: "はい".into(),
            target: "OnYes".into(),
            references: vec![],
        }),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    // Text / Choice / Barrier の 3 件。
    assert_eq!(cues.len(), 3);
    let d = text_playback_duration("ab"); // 0.1s
    // choice は text 再生完了後に発行され（start_time=D）、barrier はさらにその後（最終 offset=D）。
    assert!(matches!(command_of(&cues[0]), CueCommand::Text(_)));
    assert!(matches!(command_of(&cues[1]), CueCommand::Choice { .. }));
    assert_eq!(cues[1].start_time, d, "choice は text 再生完了後");
    assert_eq!(
        barrier_of(&cues[2]),
        &BarrierKind::WaitForChoice { timeout: None }
    );
    assert_eq!(
        cues[2].start_time, d,
        "barrier は最終 offset（choice と同一 at・FIFO で後）"
    );
}

/// `\q` を 1 つも含まない台本には barrier を発行しない（R2.5・既存完了挙動を変えない）。
#[test]
fn talk_without_choice_appends_no_barrier() {
    let compiled = compile(&[
        Instruction::Text("hi".into()),
        Instruction::Surface(SurfaceArg::new("1".into())),
    ]);
    let cues = compiled.sheet.cues();
    assert!(
        cues.iter()
            .all(|c| !matches!(c.payload, CuePayload::Barrier(_))),
        "`\\q` の無い台本は barrier を発行しない（R2.5）"
    );
}

/// `\q` の第 3 引数以降（references）を記述順を保って欠落なく運ぶ（R1.4）。
#[test]
fn choice_references_are_preserved_in_order() {
    use areka_parsers::sakura::Choice;

    let compiled = compile(&[Instruction::Choice(Choice {
        disp: "はい".into(),
        target: "OnYes".into(),
        references: vec!["r0".into(), "r1".into(), "".into()],
    })]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    // choice cue 1 件＋末尾に選択待ち barrier 1 件（`\q` を含むため・R2.1）。
    assert_eq!(cues.len(), 2, "choice cue 1 件＋末尾 barrier 1 件");
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Choice {
            id: "OnYes".into(),
            text: "はい".into(),
            // 記述順・空トークンを含め欠落なく運ぶ（不透明・ID 解釈なし・R1.4）。
            references: vec!["r0".into(), "r1".into(), "".into()],
        },
        "references は記述順を保って欠落なく運ばれる（R1.4）"
    );
    assert_eq!(
        barrier_of(&cues[1]),
        &BarrierKind::WaitForChoice { timeout: None },
        "`\\q` を含む台本の末尾に選択待ち barrier（R2.1）"
    );
}

// ── task 4.4: compile 決定論檻（メニュー・キャリア・sysvar 展開の包括固定・R9.1/9.2/9.3/9.4）──

/// fixture `menu.pasta:15` メインメニュー script を parse 済み `Instruction` 列として直入力し、
/// **完全な順序付き cue 列**（冒頭 ClearAll 前置＋末尾 barrier を含む全 sheet）を期待ベクタと
/// index ごとに突合する決定論檻（R9.1/9.2）。個別 behavioral 檻（task 4.1/4.2）が写像の各アームを
/// 担うのに対し、本檻は「実 fixture 断片 → 全 sheet」の end-to-end 固定であり、
/// - 順序（記述順＋冒頭 ClearAll＋末尾 barrier）、
/// - `at`（全命令が瞬時ゆえ全 start_time が 0.0＝完全整列）、
/// - `duration`（全 cue が瞬時 0.0）、
/// - `scope`（内容は エモ scope "1" 帰属・冒頭 ClearAll のみ scope "0"）、
/// - barrier の**唯一性**（ちょうど 1 個）と**最終位置**（末尾 index）、
/// を一括で固定する。期待ベクタは production の `emit`/`emit_barrier` で組み、actor/duration の
/// 転写規律を実装と同一に保つ（10 進直書きの表現差を排除）。
#[test]
fn menu_script_compiles_to_exact_ordered_cue_sheet() {
    use areka_parsers::sakura::Choice;

    // `\q[おしゃべり頻度,Onおしゃべり頻度メニュー]\n\q[エモの位置調整,Onエモの位置調整メニュー]`
    // `\_l[5em,2lh]\q[閉じる,Onメニュー閉じる]`（エモ発話＝scope "1"）を parse 済みとして直入力。
    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::Choice(Choice {
            disp: "おしゃべり頻度".into(),
            target: "Onおしゃべり頻度メニュー".into(),
            references: vec![],
        }),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Choice(Choice {
            disp: "エモの位置調整".into(),
            target: "Onエモの位置調整メニュー".into(),
            references: vec![],
        }),
        Instruction::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        Instruction::Choice(Choice {
            disp: "閉じる".into(),
            target: "Onメニュー閉じる".into(),
            references: vec![],
        }),
    ]);

    // 期待は全 sheet（冒頭 ClearAll＋内容 5 件＋末尾 barrier ＝ 7 件）。ClearAll のみ scope "0"
    // （production は `emit(0, ..)` で前置）、内容と barrier は現在 scope "1"。全 at=0.0/duration=0.0。
    let expected: Vec<Cue> = vec![
        emit(0, 0.0, 0.0, CueCommand::ClearAll),
        emit(
            1,
            0.0,
            0.0,
            CueCommand::Choice {
                id: "Onおしゃべり頻度メニュー".into(),
                text: "おしゃべり頻度".into(),
                references: vec![],
            },
        ),
        emit(1, 0.0, 0.0, CueCommand::NewLine { ratio: 1.0 }),
        emit(
            1,
            0.0,
            0.0,
            CueCommand::Choice {
                id: "Onエモの位置調整メニュー".into(),
                text: "エモの位置調整".into(),
                references: vec![],
            },
        ),
        emit(
            1,
            0.0,
            0.0,
            CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
        ),
        emit(
            1,
            0.0,
            0.0,
            CueCommand::Choice {
                id: "Onメニュー閉じる".into(),
                text: "閉じる".into(),
                references: vec![],
            },
        ),
        emit_barrier(1, 0.0, BarrierKind::WaitForChoice { timeout: None }),
    ];

    let cues = compiled.sheet.cues();
    assert_eq!(
        cues.len(),
        expected.len(),
        "全 sheet は ClearAll＋内容 5＋barrier ＝ 7 件（順序完全一致）"
    );
    for (i, (got, want)) in cues.iter().zip(expected.iter()).enumerate() {
        assert!(
            cue_eq(got, want),
            "index {i} の cue が期待と異なる: {got:?} != {want:?}"
        );
    }

    // at 整列: 全命令が瞬時ゆえ全 start_time は 0.0（完全整列・単調非減少の退化形）。
    for (i, cue) in cues.iter().enumerate() {
        assert_eq!(cue.start_time, 0.0, "index {i} の start_time は 0.0（at 整列）");
        assert_eq!(cue.duration, 0.0, "index {i} は瞬時（duration 0）");
    }

    // scope 帰属: 冒頭 ClearAll のみ "0"、内容＋barrier は エモ "1"。
    assert_eq!(cues[0].actor.as_str(), "0", "冒頭 ClearAll は scope 0");
    for (i, cue) in cues.iter().enumerate().skip(1) {
        assert_eq!(cue.actor.as_str(), "1", "index {i} は エモ scope 1 帰属（R3.4）");
    }

    // barrier の唯一性（ちょうど 1 個）と最終位置（末尾 index）。
    let barrier_positions: Vec<usize> = cues
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c.payload, CuePayload::Barrier(_)))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        barrier_positions,
        vec![cues.len() - 1],
        "選択待ち barrier はちょうど 1 個・末尾 index（唯一性＋最終位置・R2.1/2.2）"
    );
    assert_eq!(
        barrier_of(&cues[cues.len() - 1]),
        &BarrierKind::WaitForChoice { timeout: None },
        "末尾 barrier は WaitForChoice{{timeout:None}}（未指定＝下流の既定値へ委譲・DD-8）"
    );
}

/// `\q` を 1 つも含まないメニュー系 script（NewLine/Cursor/Text の混在）には barrier を
/// 一切発行しない（R2.5・既存完了挙動を変えない）。Cursor を含んでも「選択メニュー」でなければ
/// barrier は出ないことを決定論的に固定する（Cursor 単独では barrier 条件を満たさない）。
#[test]
fn no_choice_menu_script_emits_no_barrier() {
    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::Text("メニューやで".into()),
        Instruction::NewLine(NewLineRatio::new(1.0)),
        Instruction::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        Instruction::Text("また来てな".into()),
    ]);
    let cues = compiled.sheet.cues();
    assert!(
        cues.iter()
            .all(|c| !matches!(c.payload, CuePayload::Barrier(_))),
        "`\\q` の無い台本は Cursor を含んでも barrier を発行しない（R2.5）"
    );
}

/// fixture `menu.pasta:65` の `\1\![move,-353,,,0,base,base]` を parse 済みとして直入力し、
/// `\1` の話者スコープが汎用キャリア cue へ actor "1" として転写されること、`\![move,..]` が
/// **6 トークン（空 2 個保持）**の `move` キャリアへ写像されることを一括固定する（R9.3・R4.1/4.2/5）。
/// 既存の `move_maps_to_command_carrier_preserving_empty_tokens` は既定 scope 0 での写像を固定する
/// のに対し、本檻は `\1` プレフィックスの scope 帰属を明示する（fixture 忠実な end-to-end）。
#[test]
fn scoped_move_carrier_transcribes_scope_and_preserves_empty_tokens() {
    use areka_parsers::sakura::MoveArgs;

    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::Move(MoveArgs {
            args: vec![
                "-353".into(),
                "".into(),
                "".into(),
                "0".into(),
                "base".into(),
                "base".into(),
            ],
        }),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "内容は move キャリア cue 1 件のみ（barrier なし）");
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("move", vec!["-353", "", "", "0", "base", "base"])),
        "`\\![move,..]` → command_carrier(\"move\", 6 トークン・空 2 個保持・R9.3/4.1/4.2)"
    );
    assert_eq!(
        cues[0].actor.as_str(),
        "1",
        "`\\1` の話者スコープがキャリア cue へ actor \"1\" として転写される（R5）"
    );
    assert_eq!(cues[0].duration, 0.0, "汎用キャリアは瞬時（duration 0）");
    // move は `\q` でないため barrier を伴わない（R2.5）。
    assert!(
        compiled
            .sheet
            .cues()
            .iter()
            .all(|c| !matches!(c.payload, CuePayload::Barrier(_))),
        "キャリアのみの台本は barrier を発行しない（R2.5）"
    );
}

/// 未知名 `\![raise,OnBoot]`・単独形 `\![*]`（bare・raw_args 空）のいずれも汎用キャリア cue として
/// 発行され**無音落ちしない**ことを一括固定する（R9.3・R8.2 卒業）。compile は `\!` 名前空間を
/// typed variant 化せず単一の不透明キャリアへ載せるため、未知名でも既知名でも扱いは同型
/// （name 選別は消費側の責務）。
#[test]
fn unknown_and_bare_carrier_forms_are_emitted_not_dropped() {
    let compiled = compile(&[
        // 未知名＋引数（消費側に既知の消費者が無くてもキャリアとして載る）。
        Instruction::GenericCommand {
            name: "raise".into(),
            raw_args: vec!["OnBoot".into()],
        },
        // 単独形 `\![*]`（raw_args 空）＝R8.2 卒業（無音落ちしない）。
        Instruction::GenericCommand {
            name: "vanish".into(),
            raw_args: vec![],
        },
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2, "未知名＋単独形の 2 キャリアが載る（無音落ちなし）");
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("raise", vec!["OnBoot"])),
        "未知名 `\\![raise,OnBoot]` もキャリア cue を発行（R9.3/8.2）"
    );
    assert_eq!(
        command_of(&cues[1]).as_command_carrier(),
        Some(("vanish", vec![])),
        "単独形 `\\![*]`（raw_args 空）もキャリア cue を発行（無音落ちしない・R8.2 卒業）"
    );
    for (i, cue) in cues.iter().enumerate() {
        assert_eq!(cue.duration, 0.0, "index {i} のキャリアは瞬時（duration 0）");
    }
}

/// sysvar スナップショット展開の決定論檻（R9.4・R7.1/7.4/7.5）。実 2 引数 `compile(&instr, &snapshot)`
/// を直呼びし、スナップショット値がテキスト再生層まで実際に流れることを検証する（値あり／値なし
/// 既定／未対応名の 3 経路を script 直入力から一括固定）。task 4.2 の実装が本檻で初めて
/// **実時間非依存・script 直入力から**検証可能になる（値源は凍結スナップショット・no I/O）。
#[test]
fn sysvar_expansion_from_snapshot_is_deterministic() {
    // (1) 値ありスナップショット → その値の Text cue（duration は展開文字列の再生時間）。
    let mut vars = SystemVarSnapshot::default();
    vars.insert("username", "アヒル");
    let compiled = super::compile(&[Instruction::SystemVar("username".into())], &vars);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "展開 Text cue 1 件");
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Text("アヒル".into()),
        "値ありスナップショットは当該値の Text へ展開（R7.1）"
    );
    assert_eq!(
        cues[0].duration,
        text_playback_duration("アヒル"),
        "duration = text_playback_duration(展開文字列)（R7.2）"
    );

    // (2) 値なしスナップショット（`username` 欠落）→ 既定値「ユーザーさん」の Text cue（R7.4）。
    let compiled = super::compile(
        &[Instruction::SystemVar("username".into())],
        &SystemVarSnapshot::default(),
    );
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Text("ユーザーさん".into()),
        "値なしスナップショットは既定値へ展開（生の `%username` を露出しない・R7.4）"
    );

    // (3) M1 未対応名 `%foo` → 元の `%foo` を Text として素通し（情報を失わない縮退・R7.5）。
    let compiled = super::compile(
        &[Instruction::SystemVar("foo".into())],
        &SystemVarSnapshot::default(),
    );
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Text("%foo".into()),
        "未対応名は元の `%名前` を素通し出力（R7.5）"
    );
}
