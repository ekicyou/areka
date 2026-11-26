use super::test_support::{cue, cue_dur, items_of, reveal_times_of};
use super::*;
use dola::time::testing::at_ms;
use dola::time::testing::ms;

// ── r_i 式（先頭 r_0 = at・以降 prev + interval）・注入時刻駆動 ──

/// reveal interval は配送 duration 由来（`interval = duration / N`）。Text("アヒル") へ
/// `cue` ヘルパが焼き込む duration = 3 × 250ms = 750ms ゆえ interval = 750ms / 3 = 250ms。
#[test]
fn reveal_times_follow_duration_derived_interval_from_chunk_start() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", ms(1000), CueCommand::Text("アヒル".into())));
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![at_ms(1000), at_ms(1250), at_ms(1500)]
    );
}

#[test]
fn visible_glyphs_progress_one_by_one_with_injected_time() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", ms(1000), CueCommand::Text("アヒル".into())));

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, TimePoint::ZERO), 0);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1000)), 1); // r_0 <= t で可視
    assert_eq!(state.visible_glyphs(&actor, at_ms(1240)), 1);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1250)), 2);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1500)), 3);
    assert_eq!(state.visible_glyphs(&actor, TimePoint::MAX), 3); // 末尾到達後は飽和
}

/// 250ms の倍数でない配送 duration でも進行する。duration=150ms・N=3 → interval=50ms ちょうどで、
/// リビール時刻は [1000, 1050, 1100] ms。
#[test]
fn visible_glyphs_progress_with_duration_derived_interval() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue_dur(
        "0",
        ms(1000),
        ms(150),
        CueCommand::Text("アヒル".into()),
    ));

    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![at_ms(1000), at_ms(1050), at_ms(1100)]
    );
    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, at_ms(990)), 0);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1000)), 1);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1049)), 1);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1050)), 2); // r_1 = 1050ms
    assert_eq!(state.visible_glyphs(&actor, at_ms(1099)), 2);
    assert_eq!(state.visible_glyphs(&actor, at_ms(1100)), 3); // r_2 = 1100ms
}

// ── at は下限（それより早く可視化しない）・後続 chunk が未来なら待つ ──

#[test]
fn glyphs_never_visible_before_chunk_start() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("ab".into())));
    state.apply_cue(&cue("0", ms(10000), CueCommand::Text("cd".into())));

    // chunk 2 の r は max(250ms+250ms, 10000ms)=10000ms 起点——前 chunk 完了済みでも 10000ms まで待つ。
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![TimePoint::ZERO, at_ms(250), at_ms(10000), at_ms(10250)]
    );

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, at_ms(5000)), 2);
    assert_eq!(state.visible_glyphs(&actor, at_ms(9990)), 2);
    assert_eq!(state.visible_glyphs(&actor, at_ms(10000)), 3);
    assert_eq!(state.visible_glyphs(&actor, at_ms(10250)), 4);
}

/// 直前 chunk が未リビールでも、リビールカーソルは配送 duration が定める
/// ペース（interval）でバッファ末尾を追う（at が過去でも max が prev+interval を選ぶ）。
#[test]
fn reveal_cursor_chases_tail_when_next_chunk_start_is_earlier() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("abcd".into())));
    // 前 chunk の末尾 r_3=750ms が未来のうちに次 chunk（at=100ms）が届く。
    state.apply_cue(&cue("0", ms(100), CueCommand::Text("ef".into())));

    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![
            TimePoint::ZERO,
            at_ms(250),
            at_ms(500),
            at_ms(750),
            at_ms(1000),
            at_ms(1250)
        ]
    );
    // items はリビール中の追記も即時に 6 グリフ（未リビール分も保持＝無損失）。
    assert_eq!(items_of(&state, "0").len(), 6);

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, at_ms(900)), 4); // chunk 境界で加速しない
    assert_eq!(state.visible_glyphs(&actor, at_ms(1000)), 5);
}

/// リビール時刻列は常に単調非減少（RevealSchedule の不変条件）。
#[test]
fn reveal_times_are_monotonic_non_decreasing() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", ms(2000), CueCommand::Text("abc".into())));
    state.apply_cue(&cue("0", ms(500), CueCommand::Text("de".into()))); // at が過去
    state.apply_cue(&cue("0", ms(9000), CueCommand::Text("f".into()))); // at が未来

    let times = reveal_times_of(&state, "0");
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "times: {times:?}");
}

// ── リビール中の後続 cue も後出し優先で即時反映 ──

/// 改行: LineBreak はリビール枠（時刻）を消費しない——schedule はグリフのみ対象。
#[test]
fn line_break_takes_no_reveal_slot() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("ab".into())));
    state.apply_cue(&cue("0", ms(250), CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue("0", ms(250), CueCommand::Text("cd".into())));

    // items 5 件（グリフ 4＋改行 1）・times はグリフ 4 件分のみ。
    assert_eq!(items_of(&state, "0").len(), 5);
    // 改行マーカーは reveal 枠（interval）を消費しない: c は max(250ms+250ms, 250ms)=500ms。
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![TimePoint::ZERO, at_ms(250), at_ms(500), at_ms(750)]
    );
}

/// 全消去: リビール中の Clear は未リビール分を含め schedule ごと破棄し、
/// 以後の可視数は 0。次 chunk のリビールは旧 tail に影響されず at 起点で再開。
#[test]
fn clear_during_reveal_discards_unrevealed_and_resets_pacing() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("abcd".into())));
    state.apply_cue(&cue("0", ms(300), CueCommand::Clear)); // r_2/r_3 未リビールのまま消去

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, at_ms(300)), 0);
    assert_eq!(state.visible_glyphs(&actor, TimePoint::MAX), 0);

    // 新 chunk は旧 tail（750ms）でなく自身の at=500ms から: r = [500ms, 750ms]。
    state.apply_cue(&cue("0", ms(500), CueCommand::Text("xy".into())));
    assert_eq!(reveal_times_of(&state, "0"), vec![at_ms(500), at_ms(750)]);
    assert_eq!(state.visible_glyphs(&actor, at_ms(400)), 0);
    assert_eq!(state.visible_glyphs(&actor, at_ms(500)), 1);
}

// ── 境界: 空テキスト・未知 actor・複数 chunk 連結 ──

#[test]
fn empty_text_cue_adds_no_reveal_times() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("ab".into())));
    state.apply_cue(&cue("0", ms(500), CueCommand::Text("".into())));

    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![TimePoint::ZERO, at_ms(250)]
    );
    // 空 chunk は tail も動かさない: 次 chunk は通常式のまま。
    state.apply_cue(&cue("0", ms(500), CueCommand::Text("c".into())));
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![TimePoint::ZERO, at_ms(250), at_ms(500)]
    );
}

#[test]
fn visible_glyphs_of_unknown_actor_is_zero() {
    let state = TextLayerState::default();
    assert_eq!(
        state.visible_glyphs(&ActorKey::from("9"), TimePoint::MAX),
        0
    );
}

/// 可視数は actor 独立（他 actor のリビール進行に影響されない）。
#[test]
fn visible_glyphs_are_independent_per_actor() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("abcd".into())));
    state.apply_cue(&cue("1", ms(1000), CueCommand::Text("xy".into())));

    assert_eq!(state.visible_glyphs(&ActorKey::from("0"), at_ms(500)), 3);
    assert_eq!(state.visible_glyphs(&ActorKey::from("1"), at_ms(500)), 0);
    assert_eq!(state.visible_glyphs(&ActorKey::from("1"), at_ms(1000)), 1);
}

// ══ 服従契約の縮退と honor no-op ══

/// 縮退: 配送 duration=0（瞬時／後方互換 cue）かつ N≥1 は interval=0 ゆえ
/// 全グリフが `cue.at` で**同時**可視になる（決まった刻みで 1 グリフずつ出す実装では
/// この同時可視は成立しない）。
#[test]
fn zero_duration_reveals_all_glyphs_simultaneously_at_cue_at() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue_dur(
        "0",
        ms(1000),
        TimeSpan::ZERO,
        CueCommand::Text("アヒル".into()),
    ));
    let actor = ActorKey::from("0");

    // reveal 時刻は全て cue.at（interval=0 ゆえ max(prev+0, at)=at）。
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![at_ms(1000), at_ms(1000), at_ms(1000)]
    );
    assert_eq!(state.visible_glyphs(&actor, at_ms(990)), 0);
    assert_eq!(
        state.visible_glyphs(&actor, at_ms(1000)),
        3,
        "D=0＋N=3 は全グリフが cue.at で同時可視"
    );
}

/// 縮退: N=0（空テキスト）は duration が非零でも追記せず、除算（duration/0）を
/// 行わない。`cue` ヘルパは空テキストへ duration=0 を焼くため、ここでは敵対的な
/// 「空テキスト＋非零 duration」を [`cue_dur`] で直接与え、0 割り・追記なしを固定する。
#[test]
fn empty_text_with_nonzero_duration_adds_nothing_and_never_divides() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("ab".into())));
    // 空テキスト＋非零 duration（dola ingress を経ずに来た敵対的 cue を想定）。
    state.apply_cue(&cue_dur(
        "0",
        ms(500),
        ms(5000),
        CueCommand::Text("".into()),
    ));

    // 追記なし・reveal 時刻も増えない（0 割りせず panic もしない）。
    assert_eq!(items_of(&state, "0").len(), 2, "空テキストは追記しない");
    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![TimePoint::ZERO, at_ms(250)]
    );
    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, TimePoint::MAX), 2);
}

/// honor no-op: 担当外の cue（Emote／Wait）は action を無視するのみで、
/// その duration から**新たなローカル遅延を生じさせない**——後続の担当 Text cue の
/// reveal は、担当外 cue を挟まない対照実行と**完全に一致**する（葉の否定的 no-op）。
#[test]
fn non_relevant_cue_adds_no_local_delay_to_following_text_reveal() {
    // 対照（担当外 cue なし）。
    let mut control = TextLayerState::default();
    control.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("あ".into())));
    control.apply_cue(&cue("0", ms(500), CueCommand::Text("い".into())));

    // 実験（間に Emote／Wait を巨大 duration で挿入）。担当外ゆえ reveal に効いてはならない。
    let mut experiment = TextLayerState::default();
    experiment.apply_cue(&cue("0", TimeSpan::ZERO, CueCommand::Text("あ".into())));
    experiment.apply_cue(&cue_dur(
        "0",
        ms(250),
        ms(100000),
        CueCommand::Emote {
            key: "smile".into(),
        },
    ));
    experiment.apply_cue(&cue_dur("0", ms(400), ms(100000), CueCommand::Wait));
    experiment.apply_cue(&cue("0", ms(500), CueCommand::Text("い".into())));

    assert_eq!(
        reveal_times_of(&experiment, "0"),
        reveal_times_of(&control, "0"),
        "担当外 cue（Emote/Wait）の duration は後続 Text の reveal を一切遅らせない"
    );

    // 直接値でも固定: 2 つ目 Text の r_0 は自身の at=500ms（担当外 duration 100 が乗らない）。
    // 1 つ目 "あ"(N=1,dur250ms) の r_0=0・tail=0 → "い"(N=1,dur250ms) r = max(0+250ms, 500ms)=500ms。
    assert_eq!(
        reveal_times_of(&experiment, "0"),
        vec![TimePoint::ZERO, at_ms(500)]
    );
    assert_eq!(
        experiment.visible_glyphs(&ActorKey::from("0"), at_ms(500)),
        2
    );
}
