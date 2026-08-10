//! 選択肢の配送列合流とバッグ並存の檻（原本の区画 `:245`＝Task 2.2 の対置換）。
//!
//! `newline`・`cursor` は本テーマからしか参照されないのでここに残した。

use super::test_support::{barrier, choice, logged_commands, recording_sink};
use super::{ActorKey, BarrierKind, Cue, CueCommand, CuePlayer, CuePlayerState, CueSheet};

/// 改行 cue を作る補助（`\n` 相当・配送列で選択肢と交互配置される非選択肢 cue）。
fn newline(start_time: f64, ratio: f32) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::NewLine { ratio }.into(),
        duration: 0.0,
    }
}

/// カーソル cue を作る補助（`\_l` 相当・配送列で選択肢と交互配置される非選択肢 cue）。
fn cursor(start_time: f64, x: &str, y: &str) -> Cue {
    Cue {
        actor: ActorKey::from("0"),
        start_time,
        payload: CueCommand::Cursor {
            x: x.into(),
            y: y.into(),
        }
        .into(),
        duration: 0.0,
    }
}

// ============================================================================
// Task 2.2 対置換: 配送列の交互配置檻 ＋ バッグ並存檻（R8.6/R9.7・案C＝R1.8）
//
// 旧「先積み一択」檻（Choice を配送列から隠す）を、Choice が NewLine/Cursor と**交互のまま
// 配送列へ現れる**ことを固定する配送列檻へ対置換し（削除でなく対置換＝非退行の観測を残す）、
// 併せて「bag 内容は tick 列に不変」を固定するバッグ並存檻を新設する（責務二分＝配送列は
// 表示の単一真実源／バッグは解決照合の単一真実源）。
// ============================================================================

/// **配送列の交互配置檻（R1.8/R8.6/R9.7）**: `\q \n \q \_l \q` に相当する
/// Choice/NewLine/Choice/Cursor/Choice を台本記述順に並べた schedule を tick すると、配送列
/// （`ready()`＝表示の単一真実源）と broadcast 記録 sink の双方に、3 つの Choice が
/// NewLine/Cursor と**交互のまま同一相対順序で**現れる（Choice を配送列から隠さない）。
///
/// この檻は旧「先積み一択」挙動（Choice を配送列から分離し pending_choices のみへ積む）では
/// 3 つの Choice が配送列から欠落するため落ちる＝実挙動（案C の配送列合流）を厳密に固定する。
#[test]
fn choice_interleaves_with_newline_and_cursor_in_delivery_stream() {
    // \q \n \q \_l \q に相当（記述順で配送列へ順序保存されることを固定）。
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "選択A"),
        newline(0.1, 1.0),
        choice(0.2, "b", "選択B"),
        cursor(0.3, "5em", "2lh"),
        choice(0.4, "c", "選択C"),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // 全 cue の発火時刻を跨ぐ単一 tick で配送列を一括収集する（バリアなしゆえ一括配送）。
    player.tick(10.0);

    // 期待する配送順（Choice が NewLine/Cursor と交互のまま保存される）。
    let expected = vec![
        CueCommand::Choice {
            id: "a".into(),
            text: "選択A".into(),
            references: vec![],
        },
        CueCommand::NewLine { ratio: 1.0 },
        CueCommand::Choice {
            id: "b".into(),
            text: "選択B".into(),
            references: vec![],
        },
        CueCommand::Cursor {
            x: "5em".into(),
            y: "2lh".into(),
        },
        CueCommand::Choice {
            id: "c".into(),
            text: "選択C".into(),
            references: vec![],
        },
    ];

    // 配送列（ready＝表示の単一真実源）に交互配置がそのまま現れる（Choice を隠さない・案C）。
    assert_eq!(
        player
            .ready()
            .iter()
            .map(|c| c.command.clone())
            .collect::<Vec<_>>(),
        expected,
        "配送列に \\q \\n \\q \\_l \\q の交互配置が順序保存で現れる（配送列＝表示の単一真実源・R1.8/R8.6）"
    );
    // broadcast 記録 sink も同一順序で観測する（記録 sink での観測順一致・R9.7）。
    assert_eq!(
        logged_commands(&log),
        expected,
        "broadcast 記録 sink の観測順も交互配置を保存する（R9.7）"
    );
}

/// **バッグ並存檻（R8.6/R9.7・「bag 内容は tick 列に不変」）**: Choice は配送列へ合流しつつ、
/// 解決照合用に `pending_choices`（バッグ）へも積まれる（責務二分＝バッグは照合の単一真実源）。
/// バッグ内容は tick 列に**不変**である——同一時刻の冪等再 tick でバッグは成長しない
/// （二重積みしない）。かつバッグの選択肢集合は配送列に現れた Choice 集合と一致する
/// （同一の選択肢が配送列とバッグの両真実源へ並存する）。
#[test]
fn choice_bag_coexists_with_delivery_stream_and_is_invariant_across_reticks() {
    // choices を NewLine/Cursor と交互に並べ、末尾に WaitForChoice バリアを置く
    // （choice ありの台本の正準形）。バリアは 0.5 で、0.4 の tick では未到達。
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "選択A"),
        newline(0.1, 1.0),
        choice(0.2, "b", "選択B"),
        cursor(0.3, "5em", "2lh"),
        choice(0.4, "c", "選択C"),
        barrier(0.5, BarrierKind::WaitForChoice { timeout: None }),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // バリア手前（0.4）まで一括配送する。バリア 0.5 未到達ゆえ Playing を維持する
    // （＝冪等再 tick が state ゲートでなく schedule 前進差分ゲートを通ることを保証する）。
    player.tick(0.4);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "バリア（0.5）未到達ゆえ Playing を維持する"
    );

    let bag_ids = |p: &CuePlayer| {
        p.pending_choices()
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    };
    let bag_texts = |p: &CuePlayer| {
        p.pending_choices()
            .iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
    };

    // バッグは 3 選択肢を記述順で各 1 回だけ保持する（配送列と並存＝責務二分）。
    assert_eq!(
        bag_ids(&player),
        vec!["a", "b", "c"],
        "バッグは 3 選択肢を記述順で保持する（照合の単一真実源）"
    );
    assert_eq!(
        bag_texts(&player),
        vec!["選択A", "選択B", "選択C"],
        "バッグはテキストも保持する"
    );

    // 冪等再 tick（同一時刻を 2 度）: schedule は前進せずバッグも成長しない
    // （bag 内容は tick 列に不変＝二重積みしない）。
    player.tick(0.4);
    player.tick(0.4);
    assert_eq!(
        bag_ids(&player),
        vec!["a", "b", "c"],
        "冪等再 tick でバッグは二重積みされない（bag 内容は tick 列に不変）"
    );
    assert_eq!(
        bag_texts(&player),
        vec!["選択A", "選択B", "選択C"],
        "再 tick 後もバッグのテキストは不変"
    );

    // 責務二分の並存: バッグの選択肢集合は配送列（broadcast 記録 sink）に現れた Choice 集合と
    // 一致する（同一の選択肢が配送列とバッグの両方に現れる）。
    let delivered_choice_ids: Vec<String> = log
        .borrow()
        .iter()
        .filter_map(|c| match &c.command {
            CueCommand::Choice { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        delivered_choice_ids,
        bag_ids(&player),
        "配送列に現れた Choice 集合とバッグが一致する（同一選択肢が両真実源へ並存・責務二分）"
    );
}

/// 選択肢は WaitForChoice バリアの手前で**複数 tick に跨って**累積し、
/// `pending_choices()` から取得できる（先積みの累積性）。
#[test]
fn choices_accumulate_across_ticks_before_choice_barrier() {
    // Choice(a)@0.0 → Choice(b)@0.1 → Barrier(WaitForChoice)@0.2。
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "A"),
        choice(0.1, "b", "B"),
        barrier(0.2, BarrierKind::WaitForChoice { timeout: None }),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);

    player.tick(0.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "0.0 ではまだ Playing"
    );
    assert_eq!(
        player
            .pending_choices()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a"],
        "0.0 で選択肢 a が先積みされる"
    );

    player.tick(0.1);
    assert_eq!(
        player
            .pending_choices()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"],
        "0.1 で選択肢 b が追加累積される（跨 tick 累積）"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "0.1 でもまだ Playing"
    );

    player.tick(0.2);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "バリア到達で停止"
    );
    assert_eq!(
        player
            .pending_choices()
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"],
        "バリア時点で累積した全選択肢が先積みされている"
    );
}
