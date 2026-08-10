use super::*;
use super::test_support::{cue, cue_dur, items_of, reveal_times_of};

// ── R3.1/R3.2/R3.3/R7.1: r_i 式（先頭 r_0 = at・以降 prev + interval）・注入時刻駆動 ──

/// reveal interval は配送 duration 由来（`interval = duration / N`）。Text("アヒル") へ
/// `cue` ヘルパが焼き込む duration = 3 × 0.25 = 0.75 ゆえ interval = 0.75/3 = 0.25。
#[test]
fn reveal_times_follow_duration_derived_interval_from_chunk_start() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())));
    // r_0 = at(chunk(0)) = 1.0・以降 prev + interval(=duration/N=0.25)。
    assert_eq!(reveal_times_of(&state, "0"), vec![1.0, 1.25, 1.5]);
}

#[test]
fn visible_glyphs_progress_one_by_one_with_injected_time() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 1.0, CueCommand::Text("アヒル".into())));

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 0.0), 0);
    assert_eq!(state.visible_glyphs(&actor, 1.0), 1); // r_0 <= t で可視
    assert_eq!(state.visible_glyphs(&actor, 1.24), 1);
    assert_eq!(state.visible_glyphs(&actor, 1.25), 2);
    assert_eq!(state.visible_glyphs(&actor, 1.5), 3);
    assert_eq!(state.visible_glyphs(&actor, 100.0), 3); // 末尾到達後は飽和
}

/// 非 2 冪の配送 duration でも進行する（丸め安全マージン付き時刻で観測）。
/// duration=0.15・N=3 → interval = 0.15/3 ≈ 0.05（f64 除算・厳密ビット等価は主張しない）。
/// リビール時刻 r ≈ [1.0, 1.05, 1.10] を、±0.01 マージン付き注入時刻で観測する
/// （旧 0.05 リテラル由来の期待値でなく `D/N` 由来の近似時刻＋マージンで固定・FP flaky 回避）。
#[test]
fn visible_glyphs_progress_with_duration_derived_interval() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue_dur("0", 1.0, 0.15, CueCommand::Text("アヒル".into())));

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 0.99), 0);
    assert_eq!(state.visible_glyphs(&actor, 1.0), 1);
    assert_eq!(state.visible_glyphs(&actor, 1.06), 2); // r_1 ≈ 1.05
    assert_eq!(state.visible_glyphs(&actor, 1.11), 3); // r_2 ≈ 1.10
}

// ── R3.4: at は下限（それより早く可視化しない）・後続 chunk が未来なら待つ ──

#[test]
fn glyphs_never_visible_before_chunk_start() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
    state.apply_cue(&cue("0", 10.0, CueCommand::Text("cd".into())));

    // chunk 2 の r は max(0.25+0.25, 10.0)=10.0 起点——前 chunk 完了済みでも 10.0 まで待つ。
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 10.0, 10.25]);

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 5.0), 2);
    assert_eq!(state.visible_glyphs(&actor, 9.99), 2);
    assert_eq!(state.visible_glyphs(&actor, 10.0), 3);
    assert_eq!(state.visible_glyphs(&actor, 10.25), 4);
}

/// R3.4 後段: 直前 chunk が未リビールでも、リビールカーソルは配送 duration が定める
/// ペース（interval）でバッファ末尾を追う（at が過去でも max が prev+interval を選ぶ）。
#[test]
fn reveal_cursor_chases_tail_when_next_chunk_start_is_earlier() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
    // 前 chunk の末尾 r_3=0.75 が未来のうちに次 chunk（at=0.1）が届く。
    state.apply_cue(&cue("0", 0.1, CueCommand::Text("ef".into())));

    assert_eq!(
        reveal_times_of(&state, "0"),
        vec![0.0, 0.25, 0.5, 0.75, 1.0, 1.25]
    );

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 0.9), 4); // chunk 境界で加速しない
    assert_eq!(state.visible_glyphs(&actor, 1.0), 5);
}

/// リビール時刻列は常に単調非減少（RevealSchedule の不変条件）。
#[test]
fn reveal_times_are_monotonic_non_decreasing() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 2.0, CueCommand::Text("abc".into())));
    state.apply_cue(&cue("0", 0.5, CueCommand::Text("de".into()))); // at が過去
    state.apply_cue(&cue("0", 9.0, CueCommand::Text("f".into()))); // at が未来

    let times = reveal_times_of(&state, "0");
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "times: {times:?}");
}

// ── R3.6: リビール中の後続 cue も後出し優先で即時反映 ──

/// 追記: リビール中の Text 追記は items へ即時反映され、schedule は末尾を追う。
#[test]
fn text_append_during_reveal_applies_immediately() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
    state.apply_cue(&cue("0", 0.3, CueCommand::Text("ef".into())));

    // items は即時に 6 グリフ（未リビール分も保持＝無損失）。
    assert_eq!(items_of(&state, "0").len(), 6);
    assert_eq!(reveal_times_of(&state, "0").len(), 6);
}

/// 改行: LineBreak はリビール枠（時刻）を消費しない——schedule はグリフのみ対象。
#[test]
fn line_break_takes_no_reveal_slot() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
    state.apply_cue(&cue("0", 0.25, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue("0", 0.25, CueCommand::Text("cd".into())));

    // items 5 件（グリフ 4＋改行 1）・times はグリフ 4 件分のみ。
    assert_eq!(items_of(&state, "0").len(), 5);
    // 改行マーカーは reveal 枠（interval）を消費しない: c は max(0.25+0.25, 0.25)=0.5。
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.5, 0.75]);
}

/// 全消去: リビール中の Clear は未リビール分を含め schedule ごと破棄し、
/// 以後の可視数は 0。次 chunk のリビールは旧 tail に影響されず at 起点で再開。
#[test]
fn clear_during_reveal_discards_unrevealed_and_resets_pacing() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
    state.apply_cue(&cue("0", 0.3, CueCommand::Clear)); // r_2/r_3 未リビールのまま消去

    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 0.3), 0);
    assert_eq!(state.visible_glyphs(&actor, 100.0), 0);

    // 新 chunk は旧 tail（0.75）でなく自身の at=0.5 から: r = [0.5, 0.75]。
    state.apply_cue(&cue("0", 0.5, CueCommand::Text("xy".into())));
    assert_eq!(reveal_times_of(&state, "0"), vec![0.5, 0.75]);
    assert_eq!(state.visible_glyphs(&actor, 0.4), 0);
    assert_eq!(state.visible_glyphs(&actor, 0.5), 1);
}

// ── R3.5/10.2: 決定論（同一 cue 列＋同一注入時刻列→各時刻の可視数が常に一致） ──

#[test]
fn same_cues_and_times_yield_identical_visible_counts() {
    let sequence = vec![
        cue("0", 0.0, CueCommand::Text("アヒルやアヒル！".into())),
        cue("0", 0.4, CueCommand::NewLine { ratio: 1.0 }),
        cue("1", 0.5, CueCommand::Text("なんやそれ".into())),
        cue("0", 0.8, CueCommand::Text("ガーガー".into())),
        cue("1", 0.9, CueCommand::Clear),
        cue("1", 1.1, CueCommand::Text("……".into())),
    ];
    // 注入時刻列（フレーム時刻のつもり・cue 境界を跨ぐサンプル点）。
    let probe_times: Vec<f64> = (0..40).map(|i| i as f64 * 0.05).collect();

    let mut a = TextLayerState::default();
    let mut b = TextLayerState::default();
    for c in &sequence {
        a.apply_cue(c);
        b.apply_cue(c);
    }

    for actor in [ActorKey::from("0"), ActorKey::from("1")] {
        for &t in &probe_times {
            assert_eq!(
                a.visible_glyphs(&actor, t),
                b.visible_glyphs(&actor, t),
                "actor {actor} at t={t}"
            );
        }
    }
    assert_eq!(a, b);
}

// ── 境界: 空テキスト・未知 actor・複数 chunk 連結 ──

#[test]
fn empty_text_cue_adds_no_reveal_times() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
    state.apply_cue(&cue("0", 0.5, CueCommand::Text("".into())));

    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25]);
    // 空 chunk は tail も動かさない: 次 chunk は通常式のまま。
    state.apply_cue(&cue("0", 0.5, CueCommand::Text("c".into())));
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25, 0.5]);
}

#[test]
fn visible_glyphs_of_unknown_actor_is_zero() {
    let state = TextLayerState::default();
    assert_eq!(state.visible_glyphs(&ActorKey::from("9"), 42.0), 0);
}

/// 可視数は actor 独立（他 actor のリビール進行に影響されない）。
#[test]
fn visible_glyphs_are_independent_per_actor() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("abcd".into())));
    state.apply_cue(&cue("1", 1.0, CueCommand::Text("xy".into())));

    assert_eq!(state.visible_glyphs(&ActorKey::from("0"), 0.5), 3);
    assert_eq!(state.visible_glyphs(&ActorKey::from("1"), 0.5), 0);
    assert_eq!(state.visible_glyphs(&ActorKey::from("1"), 1.0), 1);
}

// ══ 服従契約の縮退（1.2/7.3）と honor no-op（2.2/7.5）══

/// 縮退（1.2/7.3）: 配送 duration=0（瞬時／後方互換 cue）かつ N≥1 は interval=0 ゆえ
/// 全グリフが `cue.at` で**同時**可視になる（旧 char_wait 実装は 0.05 刻みで 1 グリフずつ
/// 出すため、この同時可視は duration 服従後にのみ成立する）。
#[test]
fn zero_duration_reveals_all_glyphs_simultaneously_at_cue_at() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue_dur("0", 1.0, 0.0, CueCommand::Text("アヒル".into())));
    let actor = ActorKey::from("0");

    // reveal 時刻は全て cue.at（interval=0 ゆえ max(prev+0, at)=at）。
    assert_eq!(reveal_times_of(&state, "0"), vec![1.0, 1.0, 1.0]);
    assert_eq!(state.visible_glyphs(&actor, 0.99), 0);
    assert_eq!(
        state.visible_glyphs(&actor, 1.0),
        3,
        "D=0＋N=3 は全グリフが cue.at で同時可視"
    );
}

/// 縮退（1.8/7.3）: N=0（空テキスト）は duration が非零でも追記せず、除算（duration/0）を
/// 行わない。`cue` ヘルパは空テキストへ duration=0 を焼くため、ここでは敵対的な
/// 「空テキスト＋非零 duration」を [`cue_dur`] で直接与え、0 割り・追記なしを固定する。
#[test]
fn empty_text_with_nonzero_duration_adds_nothing_and_never_divides() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("ab".into())));
    // 空テキスト＋非零 duration（dola ingress を経ずに来た敵対的 cue を想定）。
    state.apply_cue(&cue_dur("0", 0.5, 5.0, CueCommand::Text("".into())));

    // 追記なし・reveal 時刻も増えない（0 割りせず panic もしない）。
    assert_eq!(items_of(&state, "0").len(), 2, "空テキストは追記しない");
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25]);
    let actor = ActorKey::from("0");
    assert_eq!(state.visible_glyphs(&actor, 100.0), 2);
}

/// honor no-op（2.2/2.3/7.5）: 担当外の cue（Emote／Wait）は action を無視するのみで、
/// その duration から**新たなローカル遅延を生じさせない**——後続の担当 Text cue の
/// reveal は、担当外 cue を挟まない対照実行と**完全に一致**する（葉の否定的 no-op）。
#[test]
fn non_relevant_cue_adds_no_local_delay_to_following_text_reveal() {
    // 対照（担当外 cue なし）。
    let mut control = TextLayerState::default();
    control.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    control.apply_cue(&cue("0", 0.5, CueCommand::Text("い".into())));

    // 実験（間に Emote／Wait を巨大 duration で挿入）。担当外ゆえ reveal に効いてはならない。
    let mut experiment = TextLayerState::default();
    experiment.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    experiment.apply_cue(&cue_dur(
        "0",
        0.25,
        100.0,
        CueCommand::Emote {
            key: "smile".into(),
        },
    ));
    experiment.apply_cue(&cue_dur("0", 0.4, 100.0, CueCommand::Wait));
    experiment.apply_cue(&cue("0", 0.5, CueCommand::Text("い".into())));

    assert_eq!(
        reveal_times_of(&experiment, "0"),
        reveal_times_of(&control, "0"),
        "担当外 cue（Emote/Wait）の duration は後続 Text の reveal を一切遅らせない"
    );

    // 直接値でも固定: 2 つ目 Text の r_0 は自身の at=0.5（担当外 duration 100 が乗らない）。
    // 1 つ目 "あ"(N=1,dur0.25) の r_0=0.0・tail=0.0 → "い"(N=1,dur0.25) r = max(0.0+0.25, 0.5)=0.5。
    assert_eq!(reveal_times_of(&experiment, "0"), vec![0.0, 0.5]);
    assert_eq!(experiment.visible_glyphs(&ActorKey::from("0"), 0.5), 2);
}
