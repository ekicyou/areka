//! バリア状態機械の檻——バリアに到達すると再生が停止し、各解決経路で再開する。
//!
//! WaitForInput・WaitForChoice の停止と外部解決、`skip_barrier` の強制再開と非待機での no-op、
//! Timeout バリアの自動解除を収める。`logged_has_text` は本テーマからしか参照されないのでここに残した。

use super::{BarrierKind, CueCommand, CuePlayer, CuePlayerState, CueSheet, TalkCue};
use crate::test_support::{barrier, choice, logged_commands, recording_sink, text};
use std::cell::RefCell;
use std::rc::Rc;

/// 記録する受け手に指定テキストが配られたか。
fn logged_has_text(log: &Rc<RefCell<Vec<TalkCue>>>, s: &str) -> bool {
    logged_commands(log).contains(&CueCommand::Text(s.into()))
}

// ============================================================================
// 主 observable: WaitForInput バリアで停止し、外部解決（resolve_click）で再開する
// ============================================================================

/// **観測可能な完了条件（Task 4.2）**: WaitForInput バリアに到達すると再生が停止し、
/// バリア以降の cue は配送されない。外部からの解決通知（`resolve_click`）で再開し、以降の
/// cue が配送される。占有 horizon 到達で `Completed` へ遷移する。
#[test]
fn wait_for_input_barrier_stops_and_resolve_click_resumes() {
    // before@0.0 → Barrier(WaitForInput)@0.1 → after@0.2。
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::WaitForInput { timeout: None }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "初期状態は Playing"
    );

    // 全時刻を跨ぐ tick を送るが、バリアで停止するため before までしか配送されない。
    player.tick(10.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForInput,
        "バリア到達で WaitingForInput へ停止する"
    );
    assert!(
        logged_has_text(&log, "before"),
        "バリア手前の before は配送される"
    );
    assert!(
        !logged_has_text(&log, "after"),
        "バリア以降の after は解決前に配送されない（停止の証）"
    );
    assert!(
        player.current_barrier().is_some(),
        "現在バリアで停止中である"
    );
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("before".into())],
        "配られたのは before だけ（after はスケジュールに残置される）"
    );

    // 待機中に更に tick を送っても再生は進まない（after は依然配送されない）。
    player.tick(20.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForInput,
        "待機中の tick は進行しない（バリアが保持される）"
    );
    assert!(
        !logged_has_text(&log, "after"),
        "待機中の tick でも after は配送されない"
    );
    assert_eq!(
        logged_commands(&log),
        vec![CueCommand::Text("before".into())],
        "待機中の tick でも配られたのは before だけ（after は依然残置）"
    );

    // 外部解決（クリック）で再開する。
    player.resolve_click();
    // 解決直後、次の tick で after が配送され、占有 horizon 到達で完了する。
    player.tick(20.0);
    assert!(
        logged_has_text(&log, "after"),
        "解決通知の後、after が配送される（再開の証）"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "全 cue 配送かつ horizon 到達で Completed へ遷移する"
    );
}

// ============================================================================
// WaitForChoice バリア: 選択肢先積み ＋ resolve_choice で再開
// ============================================================================

/// **選択肢の配送列合流 ＋ 解決照合バッグ並存 ＋ 再開（案C・R1.8/R8.6）**: WaitForChoice バリアの
/// 手前で連続投入された Choice cue は配送列へ**順序を保って合流**し（配送列＝表示の
/// 単一真実源）、同時に `pending_choices()` へも積まれる（バッグ＝解決照合の単一真実源）。バリア
/// 到達で WaitingForChoice へ停止し、`resolve_choice` で再開する。
///
/// 注: 配送列合流の交互配置（`\q`/`\n`/`\_l` の順序保存）と冪等再 tick の bag 不変を厚く固定する
/// 檻は Task 2.2 の対置換（配送列檻＋バッグ並存檻）に委ねる。ここでは旧「先積み一択」檻を新挙動へ
/// 最小追従させる（Choice が配送列に現れることの確認へ反転）。
#[test]
fn wait_for_choice_barrier_preloads_choices_and_resolve_choice_resumes() {
    // Choice(a)@0.0, Choice(b)@0.0 → Barrier(WaitForChoice)@0.1 → picked@0.2。
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "選択A"),
        choice(0.0, "b", "選択B"),
        barrier(0.1, BarrierKind::WaitForChoice { timeout: None }),
        text(0.2, "picked", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(10.0);
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "WaitForChoice バリアで WaitingForChoice へ停止する"
    );

    // 案C（R1.8/R8.6）: Choice は配送列へ順序を保って合流し、かつ pending_choices へも積まれる。
    assert!(
        logged_commands(&log)
            .iter()
            .any(|c| matches!(c, CueCommand::Choice { .. })),
        "Choice は配送列へ合流する（配送列＝表示の単一真実源・案C＝R1.8/R8.6）"
    );
    let ids: Vec<&str> = player
        .pending_choices()
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(ids, vec!["a", "b"], "選択肢が記述順で先積みされる");
    let texts: Vec<&str> = player
        .pending_choices()
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert_eq!(texts, vec!["選択A", "選択B"], "選択肢テキストも保持される");
    assert!(
        !logged_has_text(&log, "picked"),
        "バリア以降の picked は未配送"
    );

    // 未知 id は再開しない（該当 id のみ解決可能）。
    assert_eq!(
        player.resolve_choice("zzz"),
        None,
        "未知 id では解決されない"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "未知 id 解決後も WaitingForChoice のまま"
    );

    // 該当 id で再開。
    assert_eq!(
        player.resolve_choice("b"),
        Some("b".to_string()),
        "該当 id は解決され id を返す"
    );
    assert!(
        player.pending_choices().is_empty(),
        "解決で先積み選択肢はクリアされる"
    );
    player.tick(20.0);
    assert!(
        logged_has_text(&log, "picked"),
        "選択肢解決の後、picked が配送される（再開の証）"
    );
    assert_eq!(
        player.state(),
        &CuePlayerState::Completed,
        "horizon 到達で完了"
    );
}

// ============================================================================
// バリア seam: skip_barrier による強制再開
// ============================================================================

/// `skip_barrier` は待機状態（入力/選択いずれも）を強制的に Playing へ戻す。
#[test]
fn skip_barrier_force_resumes_from_waiting() {
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::WaitForInput { timeout: None }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    player.tick(10.0);
    assert_eq!(player.state(), &CuePlayerState::WaitingForInput);

    player.skip_barrier();
    player.tick(20.0);
    assert!(
        logged_has_text(&log, "after"),
        "skip_barrier で強制再開し after が配送される"
    );
    assert_eq!(player.state(), &CuePlayerState::Completed);
}

/// 非待機状態での解決通知は no-op（Playing のまま何も壊さない）。
#[test]
fn resolve_on_non_waiting_state_is_noop() {
    let sheet = CueSheet::new(vec![text(0.0, "only", 0.0)]);
    let mut player = CuePlayer::from_sheet(&sheet);

    // まだ tick 前＝Playing。resolve_click / skip_barrier は no-op。
    player.resolve_click();
    player.skip_barrier();
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "非待機での解決は no-op"
    );
    assert_eq!(
        player.resolve_choice("x"),
        None,
        "非待機の resolve_choice は None"
    );
}

/// 選択待ちのあいだのクリックの解決は no-op（選択待ちは選択肢の id でしか解けない）。
/// 状態も先積みの選択肢も変えない。
#[test]
fn resolve_click_is_noop_while_waiting_for_choice() {
    let sheet = CueSheet::new(vec![
        choice(0.0, "a", "選択A"),
        barrier(0.1, BarrierKind::WaitForChoice { timeout: None }),
        text(0.2, "picked", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    player.tick(10.0);
    assert_eq!(player.state(), &CuePlayerState::WaitingForChoice);
    let before = player.pending_choices().to_vec();

    player.resolve_click();

    assert_eq!(
        player.state(),
        &CuePlayerState::WaitingForChoice,
        "クリックでは選択待ちが解けない"
    );
    assert_eq!(
        player.pending_choices(),
        before.as_slice(),
        "先積みの選択肢は変わらない"
    );
}

// ============================================================================
// Timeout バリア: Playing を維持しつつ継続 tick で自動解除される
// ============================================================================

/// Timeout バリアは待機状態にせず Playing を維持し（schedule が自動管理）、継続 tick で
/// duration 経過後に自動解除されて後続 cue が配送される。
#[test]
fn timeout_barrier_keeps_playing_and_auto_resolves_by_continued_ticking() {
    // before@0.0 → Barrier(Timeout{1.0})@0.1 → after@0.2。
    let sheet = CueSheet::new(vec![
        text(0.0, "before", 0.0),
        barrier(0.1, BarrierKind::Timeout { duration: 1.0 }),
        text(0.2, "after", 0.0),
    ]);
    let mut player = CuePlayer::from_sheet(&sheet);
    let (log, sink) = recording_sink();
    player.register_sink(sink);

    // Timeout バリアに到達しても Playing を維持する（WaitingFor* へは遷移しない）。
    player.tick(0.1);
    assert_eq!(
        player.state(),
        &CuePlayerState::Playing,
        "Timeout バリアは Playing を維持する（自動管理）"
    );
    assert!(
        !logged_has_text(&log, "after"),
        "duration 未経過ゆえ after は未配送"
    );

    // Timeout duration（0.1 + 1.0 = 1.1）経過後の tick で自動解除され after が配送される。
    player.tick(1.1);
    assert!(
        logged_has_text(&log, "after"),
        "Timeout duration 経過で自動解除され after が配送される"
    );
}
