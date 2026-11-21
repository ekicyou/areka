use super::test_support::{
    assert_clear_all_prefix_and_rest, barrier_of, command_of, compile, cue_eq,
};
use super::*;
use crate::duration::text_playback_duration;
use crate::sysvar::SystemVarSnapshot;
use areka_parsers::sakura::{NewLineRatio, SurfaceArg};

/// バルーン面切替命令の引数を解釈・変換せず値のまま `BalloonSurface{key}` へ
/// バイト完全一致で転写する（不透明写像）。名前形（非 ASCII）・数値形・非表示
/// センチネル `-1` のいずれも数値化・alias 解決せず、`Surface`→`Emote` と完全対称に扱う。
/// `BalloonSurface` が無視されず cue になることの固定。
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

/// シェル面切替 `Surface`→`Emote` の写像は、バルーン面切替と独立に保たれる。
/// scope/start_time の転写も同じ。
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

/// 話者スコープ切替で actor が転写され、未指定開始は既定 "0"。
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

/// NewLine/Clear の写像。
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

/// `Raw` だけが cue を作らず、`Choice`・`Cursor`・`Anchor`・`GenericCommand`（`move` も含む）・
/// `SystemVar` は各々 cue を作る。無視される側が `Raw` だけであることを、cue を作る側と対にして
/// 固定する。`Raw` は自分の腕で `raw_ignored` の警告を 1 行残して捨てる
/// （`raw_is_dropped_with_one_warning`）。各語彙の写像の細目は個別の檻が担う。
#[test]
fn catch_all_ignored_set_is_raw_only() {
    use areka_parsers::sakura::{Anchor, Choice};

    // (1) Raw は除外集合ゆえ 0 cue。中途に Raw を挟んでも内容は後続 Text のみ（ClearAll 前置）。
    let compiled = compile(&[
        Instruction::Raw("\\?".into()),
        Instruction::Text("hi".into()),
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        cues.len(),
        1,
        "Raw は cue を生成しない（除外集合は Raw のみ）"
    );
    match command_of(&cues[0]) {
        CueCommand::Text(s) => assert_eq!(s, "hi"),
        other => panic!("expected Text, got {other:?}"),
    }
    // Raw 単独（後続なし）は空 sheet（内容 cue ゼロゆえ ClearAll 前置もなし）。
    assert!(
        compile(&[Instruction::Raw("\\0".into())]).sheet.is_empty(),
        "Raw のみの台本は空 sheet（除外集合の 0 cue を単独でも固定）"
    );

    // (2) Raw 以外の語彙は各々 cue を生成する（sheet 非空）。詳細写像は個別の檻
    //     （`*_maps_to_*` 等）が担い、ここでは生成側のみを固定する。
    let graduated: [Instruction; 6] = [
        Instruction::Anchor(Anchor {
            id: "x".into(),
            references: vec![],
        }),
        Instruction::Choice(Choice {
            disp: "はい".into(),
            target: "OnYes".into(),
            references: vec![],
        }),
        Instruction::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        Instruction::GenericCommand {
            name: "move".into(),
            raw_args: vec!["100".into()],
        },
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

/// `GenericCommand`（`move` も含む）・`SystemVar` は専用アームで cue を発行し、`Raw` は 0 cue で
/// panic しない。両者が交錯しても各々の写像・破棄が一貫することを固定する。各アームの詳細写像は
/// 個別の檻（`scoped_move_carrier_*` 他）が担う。
#[test]
fn graduated_arms_emit_while_raw_remains_ignored() {
    let compiled = compile(&[
        Instruction::GenericCommand {
            name: "move".into(),
            raw_args: vec!["100".into(), "200".into()],
        },
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
    assert_eq!(
        cues.len(),
        4,
        "卒業アーム 3 種＋末尾 Text の 4 cue（Raw のみ破棄）"
    );
    assert_eq!(
        command_of(&cues[0]).as_command_carrier(),
        Some(("move", vec!["100", "200"])),
        "move はキャリア cue へ卒業（R4.1）"
    );
    match command_of(&cues[1]) {
        CueCommand::Text(s) => assert_eq!(
            s, "ユーザーさん",
            "SystemVar は Text cue へ卒業（R7.1/7.4）"
        ),
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
    // 終端命令を含まないため末尾に着いて Ended。
    assert_eq!(compiled.end, TalkEndReason::Ended);
}

// ── Cursor アーム写像の檻（Choice の写像は下の決定論檻が担う） ──

/// `\_l[,]` 相当（x・y 双方空）でも Cursor cue が無条件に発行される（記述の存在を台本から
/// 失わせない）。空は代表形と別物（区別保持）。
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

// ── barrier 発行の檻（move・GenericCommand・SystemVar の写像は下の決定論檻が担う） ──

/// `\q` を 1 個以上含む台本には選択待ち barrier をちょうど 1 個、全 choice cue より後（最終
/// offset）へ発行する。タイムアウトは指定しない（`timeout:None`＝未指定＝
/// 下流の既定値へ委譲）。
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
    // barrier はちょうど 1 個。
    let barrier_count = cues
        .iter()
        .filter(|c| matches!(c.payload, CuePayload::Barrier(_)))
        .count();
    assert_eq!(barrier_count, 1, "選択待ち barrier はちょうど 1 個（R2.1）");
    // barrier は全 choice cue より後。
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
/// 置かれる。`Text("ab") \q[..]` → barrier は text D の位置。
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

/// `\q` の第 3 引数以降（references）を記述順を保って欠落なく運ぶ。
#[test]
fn choice_references_are_preserved_in_order() {
    use areka_parsers::sakura::Choice;

    let compiled = compile(&[Instruction::Choice(Choice {
        disp: "はい".into(),
        target: "OnYes".into(),
        references: vec!["r0".into(), "r1".into(), "".into()],
    })]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    // choice cue 1 件＋末尾に選択待ち barrier 1 件（`\q` を含むため）。
    assert_eq!(cues.len(), 2, "choice cue 1 件＋末尾 barrier 1 件");
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Choice {
            id: "OnYes".into(),
            text: "はい".into(),
            // 記述順・空トークンを含め欠落なく運ぶ（不透明・ID 解釈なし）。
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

// ── compile 決定論檻（メニュー・キャリア・sysvar 展開の包括固定）──

/// fixture `menu.pasta:15` メインメニュー script を parse 済み `Instruction` 列として直入力し、
/// **完全な順序付き cue 列**（冒頭 ClearAll 前置＋末尾 barrier を含む全 sheet）を期待ベクタと
/// index ごとに突合する決定論檻。Choice・NewLine・Cursor の写像（id=target・
/// text=disp・x/y の不透明転写）も、記述順も、ここで 1 件ずつ固定する。「実 fixture 断片 → 全 sheet」の
/// end-to-end 固定であり、
/// - 順序（記述順＋冒頭 ClearAll＋末尾 barrier）、
/// - `at`（全命令が瞬時ゆえ全 start_time が 0.0＝完全整列）、
/// - `duration`（全 cue が瞬時 0.0）、
/// - `scope`（内容は エモ scope "1" 帰属・冒頭 ClearAll のみ scope "0"）、
/// - barrier の**唯一性**（ちょうど 1 個）と**最終位置**（末尾 index）
///
/// を一括で固定する。期待ベクタは production の `emit` で組み、actor/duration の
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
        emit(1, 0.0, 0.0, BarrierKind::WaitForChoice { timeout: None }),
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
        assert_eq!(
            cue.start_time, 0.0,
            "index {i} の start_time は 0.0（at 整列）"
        );
        assert_eq!(cue.duration, 0.0, "index {i} は瞬時（duration 0）");
    }

    // scope 帰属: 冒頭 ClearAll のみ "0"、内容＋barrier は エモ "1"。
    assert_eq!(cues[0].actor.as_str(), "0", "冒頭 ClearAll は scope 0");
    for (i, cue) in cues.iter().enumerate().skip(1) {
        assert_eq!(
            cue.actor.as_str(),
            "1",
            "index {i} は エモ scope 1 帰属（R3.4）"
        );
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
/// 一切発行しない（既存の完了挙動を変えない）。Cursor を含んでも「選択メニュー」でなければ
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
/// **6 トークン（空 2 個保持）**の `move` キャリアへ写像されることを一括固定する。
/// `\1` プレフィックスの scope 帰属を明示する（fixture 忠実な end-to-end）。
#[test]
fn scoped_move_carrier_transcribes_scope_and_preserves_empty_tokens() {
    let compiled = compile(&[
        Instruction::SpeakerScope { n: 1 },
        Instruction::GenericCommand {
            name: "move".into(),
            raw_args: vec![
                "-353".into(),
                "".into(),
                "".into(),
                "0".into(),
                "base".into(),
                "base".into(),
            ],
        },
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        cues.len(),
        1,
        "内容は move キャリア cue 1 件のみ（barrier なし）"
    );
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
    // move は `\q` でないため barrier を伴わない。
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
/// 発行され**無音落ちしない**ことを一括固定する。compile は `\!` 名前空間を
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
        // 単独形 `\![*]`（raw_args 空）も無音落ちしない。
        Instruction::GenericCommand {
            name: "vanish".into(),
            raw_args: vec![],
        },
    ]);
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(
        cues.len(),
        2,
        "未知名＋単独形の 2 キャリアが載る（無音落ちなし）"
    );
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
        assert_eq!(
            cue.duration, 0.0,
            "index {i} のキャリアは瞬時（duration 0）"
        );
    }
}

/// sysvar スナップショット展開の決定論檻。実 2 引数 `compile(&instr, &snapshot)`
/// を直呼びし、スナップショット値がテキスト再生層まで実際に流れることを検証する（値あり／値なし
/// 既定／未対応名の 3 経路を script 直入力から一括固定）。値源は凍結スナップショットで
/// 実時間に依らない（no I/O）。
#[test]
fn sysvar_expansion_from_snapshot_is_deterministic() {
    // (1) 値ありスナップショット → その値の Text cue（duration は展開文字列の再生時間）。
    let mut vars = SystemVarSnapshot::default();
    vars.insert("username", "アヒル");
    // 後ろに面の切り替えを置き、展開した文字の長さの分だけ後ろが遅れることも見る。
    let compiled = super::compile(
        &[
            Instruction::SystemVar("username".into()),
            Instruction::Surface(SurfaceArg::new("1".into())),
        ],
        &vars,
    );
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 2, "展開 Text cue と Emote の 2 件");
    assert_eq!(
        command_of(&cues[0]),
        &CueCommand::Text("アヒル".into()),
        "値ありスナップショットは当該値の Text へ展開（R7.1）"
    );
    let d = text_playback_duration("アヒル"); // 3 クラスタ × 50ms
    assert_eq!(cues[0].start_time, 0.0);
    assert_eq!(
        cues[0].duration, d,
        "duration = text_playback_duration(展開文字列)（R7.2）"
    );
    assert_eq!(command_of(&cues[1]), &CueCommand::Emote { key: "1".into() });
    assert_eq!(
        cues[1].start_time, d,
        "SystemVar 由来 D の分だけ後続が遅れる（R7.2）"
    );

    // (2) 値なしスナップショット（`username` 欠落）→ 既定値「ユーザーさん」の Text cue。
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
    assert_eq!(
        cues[0].duration,
        text_playback_duration("ユーザーさん"),
        "既定値も通常テキストと同一の再生時間規則（R7.2）"
    );

    // (3) 未対応名 `%foo` → 元の `%foo` を Text として素通し（情報を失わない縮退）。
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

/// 知らないタグの `Raw` は合図を作らず、`raw_ignored` の警告を 1 行だけ残して捨てる。
/// 欄は綴りの先頭 64 字（`head`）とバイト数（`bytes`）。後ろの台本は今までどおり。
#[test]
fn raw_is_dropped_with_one_warning() {
    // 閉じの無い `[` で残りが 1 つの Raw になった形（65 字・非 ASCII を含む）。
    let raw: String = format!("\\s[10{}", "あ".repeat(61));
    let (compiled, events) = log_capture_kit::capture(|| {
        compile(&[
            Instruction::Raw(raw.clone()),
            Instruction::Text("hi".into()),
        ])
    });
    let warned: Vec<_> = events
        .iter()
        .filter(|ev| ev.level == tracing::Level::WARN)
        .collect();
    assert_eq!(warned.len(), 1, "{events:?}");
    assert_eq!(warned[0].field_str("event"), Some("raw_ignored"));
    let head: String = raw.chars().take(64).collect();
    assert_eq!(warned[0].field("head"), Some(head.as_str()));
    assert_eq!(
        warned[0].field("bytes"),
        Some(raw.len().to_string().as_str())
    );
    let cues = assert_clear_all_prefix_and_rest(compiled.sheet.cues());
    assert_eq!(cues.len(), 1, "Raw は合図を作らない");
    assert_eq!(command_of(&cues[0]), &CueCommand::Text("hi".into()));
}
