use log_capture_kit::count_levels;
use tracing::Level;

use super::test_support::{REVEAL_INTERVAL, cue, cue_dur, items_of, reveal_times_of};
use super::*;
use crate::actor::test_support::{capture_checked, level_count};

// ── R2.1: Text 追記 ──

#[test]
fn text_cue_appends_glyphs_in_order() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒル".into())));
    assert_eq!(
        items_of(&state, "0"),
        &[
            TextItem::glyph("ア"),
            TextItem::glyph("ヒ"),
            TextItem::glyph("ル"),
        ]
    );
}

#[test]
fn consecutive_text_cues_append() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒ".into())));
    state.apply_cue(&cue("0", 0.5, CueCommand::Text("ルや".into())));
    assert_eq!(
        items_of(&state, "0"),
        &[
            TextItem::glyph("ア"),
            TextItem::glyph("ヒ"),
            TextItem::glyph("ル"),
            TextItem::glyph("や"),
        ]
    );
}

// ── R2.2: NewLine 改行（ratio 転写） ──

#[test]
fn newline_cue_appends_line_break_marker_with_ratio() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())));
    state.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue("0", 0.2, CueCommand::NewLine { ratio: 0.5 }));
    state.apply_cue(&cue("0", 0.3, CueCommand::Text("B".into())));
    assert_eq!(
        items_of(&state, "0"),
        &[
            TextItem::glyph("A"),
            TextItem::LineBreak { ratio: 1.0 },
            TextItem::LineBreak { ratio: 0.5 },
            TextItem::glyph("B"),
        ]
    );
}

// ── R2.3: Clear 全消去（未リビール分含む・schedule ごと初期化） ──

#[test]
fn clear_resets_actor_state_to_initial() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("アヒルや".into())));
    state.apply_cue(&cue("0", 0.1, CueCommand::NewLine { ratio: 1.0 }));
    state.apply_cue(&cue("0", 0.2, CueCommand::Text("アヒル！".into())));
    state.apply_cue(&cue("0", 0.3, CueCommand::Clear));

    let actor = state
        .actor_state(&ActorKey::from("0"))
        .expect("actor state should exist");
    assert_eq!(actor, &ActorTextState::default());
}

#[test]
fn clear_only_affects_target_actor() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("さくら".into())));
    state.apply_cue(&cue("1", 0.0, CueCommand::Text("けろ".into())));
    state.apply_cue(&cue("0", 0.5, CueCommand::Clear));

    assert!(items_of(&state, "0").is_empty());
    assert_eq!(
        items_of(&state, "1"),
        &[TextItem::glyph("け"), TextItem::glyph("ろ")]
    );
}

/// `ClearAll` は保持する**全**スコープを消去し、対象スコープのみの `Clear` と
/// 峻別される。cue の actor（ここでは "0"）に関わらず、当該 talk が書き込んで
/// いないスコープ（"1"）も消える点が要点。
#[test]
fn clear_all_erases_every_actor_scope_unlike_clear() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("さくら".into())));
    state.apply_cue(&cue("1", 0.0, CueCommand::Text("けろ".into())));

    // 対象スコープのみの Clear では他スコープが残る（対比）。
    state.apply_cue(&cue("0", 0.5, CueCommand::Clear));
    assert!(!items_of(&state, "1").is_empty());

    // ClearAll は cue の actor に関わらず全スコープを消す。
    state.apply_cue(&cue("0", 1.0, CueCommand::ClearAll));
    assert!(items_of(&state, "0").is_empty());
    assert!(
        items_of(&state, "1").is_empty(),
        "ClearAll は当該 cue が名指ししていないスコープも消去する"
    );
}

/// `Wait`（action を持たない純粋な待ち）は文字状態機械の担当外——受け取っても
/// テキスト状態を一切変えない（葉の否定的 no-op・二重待ちを生まない）。
#[test]
fn wait_cue_leaves_text_state_untouched() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
    let before = state
        .actor_state(&ActorKey::from("0"))
        .expect("actor state should exist")
        .clone();

    state.apply_cue(&cue("0", 0.5, CueCommand::Wait));

    let after = state
        .actor_state(&ActorKey::from("0"))
        .expect("actor state should exist");
    assert_eq!(&before, after, "Wait は状態を変えない（action なし）");
}

// ── R1.6: actor 別振り分け・独立状態・lazily 生成 ──

#[test]
fn cues_route_to_independent_actor_states() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())));
    state.apply_cue(&cue("1", 0.1, CueCommand::Text("B".into())));
    state.apply_cue(&cue("0", 0.2, CueCommand::Text("C".into())));

    assert_eq!(
        items_of(&state, "0"),
        &[TextItem::glyph("A"), TextItem::glyph("C")]
    );
    assert_eq!(items_of(&state, "1"), &[TextItem::glyph("B")]);
}

#[test]
fn unknown_actor_state_lazily_created_and_accumulates() {
    let mut state = TextLayerState::default();
    assert!(state.actor_state(&ActorKey::from("7")).is_none());

    state.apply_cue(&cue("7", 0.0, CueCommand::Text("x".into())));
    assert_eq!(items_of(&state, "7"), &[TextItem::glyph("x")]);
}

#[test]
fn actors_iterate_in_deterministic_key_order() {
    let mut state = TextLayerState::default();
    // 逆順に生成しても走査は ActorKey 昇順（決定論的順序）。
    state.apply_cue(&cue("1", 0.0, CueCommand::Text("b".into())));
    state.apply_cue(&cue("0", 0.1, CueCommand::Text("a".into())));

    let keys: Vec<&ActorKey> = state.actors().map(|(k, _)| k).collect();
    assert_eq!(keys, vec![&ActorKey::from("0"), &ActorKey::from("1")]);
}

// ── R10.5: 上書きガードなし・後出し優先の忠実適用 ──

#[test]
fn later_cues_apply_immediately_without_overwrite_guard() {
    let mut state = TextLayerState::default();
    // talk 1 の途中に talk 2 の cue 列（Clear→Text）が届いても、そのまま忠実に適用される。
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("talk1".into())));
    state.apply_cue(&cue("0", 0.1, CueCommand::Clear));
    state.apply_cue(&cue("0", 0.1, CueCommand::Text("talk2".into())));

    let expected: Vec<TextItem> = clusters("talk2").map(TextItem::glyph).collect();
    assert_eq!(items_of(&state, "0"), expected.as_slice());
}

// ── Choice/Cursor 実消費（areka-P0-choice-render タスク 1.2） ──

/// actor の choices を取得する（未生成なら panic ＝テスト失敗として扱う）。
fn choices_of<'a>(state: &'a TextLayerState, actor: &str) -> &'a [ChoiceSpan] {
    state
        .actor_state(&ActorKey::from(actor))
        .expect("actor state should exist")
        .choices()
}

/// Choice cue ヘルパ。reveal を Text と同じ時刻式で観測するため、配送 duration =
/// `N × REVEAL_INTERVAL` を焼き込む（interval=0.25・[`cue`] の Text 分岐と機能等価）。
fn choice_cue(actor: &str, at: f64, id: &str, text: &str, refs: &[&str]) -> TalkCue {
    let duration = areka_sakura::cluster::cluster_count(text) as f64 * REVEAL_INTERVAL;
    cue_dur(
        actor,
        at,
        duration,
        CueCommand::Choice {
            id: id.into(),
            text: text.into(),
            references: refs.iter().map(|s| s.to_string()).collect(),
        },
    )
}

// ── R1.1/R1.2: Choice 実消費——グリフ追記＋非空 ChoiceSpan 記録 ──

/// 非空 `text` の Choice cue はグリフを items へ追記し（Text と同一）、非空
/// `glyph_range` を持つ `ChoiceSpan` を記録する（ordinal=0・id/label/references 忠実転写）。
#[test]
fn choice_cue_appends_glyphs_and_records_nonempty_span() {
    let mut state = TextLayerState::default();
    state.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &["r0", "r1"]));

    // グリフは items へ追記される（Text cue と同一経路）。
    assert_eq!(
        items_of(&state, "0"),
        &[TextItem::glyph("は"), TextItem::glyph("い")]
    );
    // 非空 glyph_range のスパンが記録される。
    assert_eq!(
        choices_of(&state, "0"),
        &[ChoiceSpan {
            kind: SpanKind::Choice,
            ordinal: 0,
            id: "OnYes".into(),
            label: "はい".into(),
            references: vec!["r0".into(), "r1".into()],
            glyph_range: 0..2,
        }]
    );
    // reveal も Text と同じ時刻式で拡張される（duration=2×0.25 → interval=0.25）。
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0, 0.25]);
}

/// 配送順スパン記録（R1.2）: 3 つ以上の Choice を、間に Text／NewLine を挟んで配送しても、
/// `ChoiceSpan` は**配送順**に並び（label が配送順に一致）・`ordinal` は **0,1,2 と厳密に
/// 単調増加**し（design.md「不変条件 1」の順序）・`glyph_range` は互いに素かつ追記順単調。
/// 序数空間はグリフのみ（挟んだ改行等の非グリフ item を数えない）。
#[test]
fn choice_cues_preserve_delivery_order_with_strictly_monotonic_ordinals() {
    let mut state = TextLayerState::default();
    // 選択肢の間に通常テキスト・改行を interleave して配送する。
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("どれ".into()))); // グリフ 0..2
    state.apply_cue(&choice_cue("0", 0.1, "q0", "はい", &["a"])); // グリフ 2..4
    state.apply_cue(&cue("0", 0.2, CueCommand::Text("か".into()))); // グリフ 4..5（間テキスト）
    state.apply_cue(&cue("0", 0.3, CueCommand::NewLine { ratio: 1.0 })); // 非グリフ
    state.apply_cue(&choice_cue("0", 0.4, "q1", "いいえ", &["b"])); // グリフ 5..8
    state.apply_cue(&choice_cue("0", 0.5, "q2", "たぶん", &["c"])); // グリフ 8..11

    let spans = choices_of(&state, "0");
    assert_eq!(spans.len(), 3);

    // ordinal は 0,1,2 と厳密単調増加。
    let ordinals: Vec<usize> = spans.iter().map(|s| s.ordinal).collect();
    assert_eq!(ordinals, vec![0, 1, 2]);

    // 配送順が保存される（label／id が配送順に一致）。
    assert_eq!(
        spans.iter().map(|s| s.label.as_str()).collect::<Vec<_>>(),
        vec!["はい", "いいえ", "たぶん"]
    );
    assert_eq!(
        spans.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
        vec!["q0", "q1", "q2"]
    );

    // glyph_range は間テキストを跨いで正しく（グリフ序数空間・非グリフを数えない）。
    assert_eq!(spans[0].glyph_range, 2..4);
    assert_eq!(spans[1].glyph_range, 5..8);
    assert_eq!(spans[2].glyph_range, 8..11);
}

/// R1.5 縮退（design.md 縮退表 Choice text 空 row）: 空 `text` は warn!＋空範囲スパン記録・
/// グリフ追記なし・reveal 不変。空 Choice は届くたびに warn する（1 回で黙らせない）。
#[test]
fn empty_choice_text_warns_and_records_empty_range_no_glyphs() {
    let (state, counts) = count_levels(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into()))); // グリフ 0
        state.apply_cue(&choice_cue("0", 0.1, "empty", "", &[]));
        state
    });

    // グリフは追記されない（既存 "あ" のみ）・reveal 不変。
    assert_eq!(items_of(&state, "0"), &[TextItem::glyph("あ")]);
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0]);
    // 空範囲スパンが記録される（start==end==1＝現グリフ末尾）。
    assert_eq!(
        choices_of(&state, "0"),
        &[ChoiceSpan {
            kind: SpanKind::Choice,
            ordinal: 0,
            id: "empty".into(),
            label: "".into(),
            references: vec![],
            glyph_range: 1..1,
        }]
    );
    // 空 text は warn する（R1.5）。
    assert_eq!(counts.warn, 1);
}

/// 反復 Choice cue（非空 text）は警告を一切発火しない。
#[test]
fn repeated_nonempty_choice_cues_do_not_warn() {
    let ((), events) = capture_checked(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&choice_cue("0", 0.0, "a", "はい", &[]));
        state.apply_cue(&choice_cue("0", 0.5, "b", "いいえ", &[]));
        state.apply_cue(&choice_cue("1", 1.0, "c", "はい", &[]));
    });

    assert_eq!(
        level_count(&events, Level::WARN),
        0,
        "非空 Choice は警告しない"
    );
}

// ── R2.1: Cursor 実消費——CursorMove 追記・グリフ/リビール不変 ──

/// Cursor cue は各軸を parse_cursor_coord で語彙化した `CursorMove` を items へ追記し、
/// グリフ／リビール状態は変えない（非グリフ item・reveal 対象外）。警告も出さない。
#[test]
fn cursor_cue_appends_cursor_move_and_leaves_glyph_reveal_unchanged() {
    let (state, events) = capture_checked(|| {
        let mut state = TextLayerState::default();
        state.apply_cue(&cue("0", 0.0, CueCommand::Text("あ".into())));
        state.apply_cue(&cue(
            "0",
            0.1,
            CueCommand::Cursor {
                x: "5em".into(),
                y: "2lh".into(),
            },
        ));
        // 反復しても warn 無し。
        state.apply_cue(&cue(
            "0",
            0.2,
            CueCommand::Cursor {
                x: "".into(),
                y: "@3".into(),
            },
        ));
        state
    });

    // items: グリフ 1＋CursorMove 2（追記順）。
    assert_eq!(
        items_of(&state, "0"),
        &[
            TextItem::glyph("あ"),
            TextItem::CursorMove {
                x: CursorCoord::Absolute {
                    value: 5.0,
                    unit: CursorUnit::Em
                },
                y: CursorCoord::Absolute {
                    value: 2.0,
                    unit: CursorUnit::Lh
                },
            },
            TextItem::CursorMove {
                x: CursorCoord::Omitted,
                y: CursorCoord::Relative {
                    value: 3.0,
                    unit: CursorUnit::Px
                },
            },
        ]
    );
    // グリフ／リビール状態は不変（CursorMove は reveal 枠を消費しない）。
    assert_eq!(reveal_times_of(&state, "0"), vec![0.0]);
    assert_eq!(state.visible_glyphs(&ActorKey::from("0"), 100.0), 1);
    // Cursor は choices を作らない。
    assert!(choices_of(&state, "0").is_empty());
    // 警告は発火しない。
    assert_eq!(level_count(&events, Level::WARN), 0);
}

/// Cursor アームがグリフ／reveal 状態を一切変えないことを、対照実行（Cursor cue を
/// 挟まない）との**バイト等価**で固定する（「グリフ/reveal 状態を変更しない」）。
/// CursorMove は非グリフ item ゆえ、グリフ列（items から Glyph のみ抽出）と reveal 時刻列は
/// Cursor cue の有無で完全に一致し、可視グリフ数も全時刻で一致する。
#[test]
fn cursor_cue_leaves_glyph_and_reveal_byte_identical_to_run_without_it() {
    // 対照: Cursor cue なし。
    let mut control = TextLayerState::default();
    control.apply_cue(&cue("0", 0.0, CueCommand::Text("あい".into())));
    control.apply_cue(&cue("0", 0.5, CueCommand::Text("うえ".into())));

    // 実験: 途中と末尾に Cursor cue を挟む（Absolute/Relative/Omitted/Invalid を混在）。
    let mut experiment = TextLayerState::default();
    experiment.apply_cue(&cue("0", 0.0, CueCommand::Text("あい".into())));
    experiment.apply_cue(&cue(
        "0",
        0.25,
        CueCommand::Cursor {
            x: "3em".into(),
            y: "@-1".into(),
        },
    ));
    experiment.apply_cue(&cue("0", 0.5, CueCommand::Text("うえ".into())));
    experiment.apply_cue(&cue(
        "0",
        0.75,
        CueCommand::Cursor {
            x: "".into(),
            y: "bogus".into(),
        },
    ));

    // グリフ列（Glyph のみ抽出）はバイト等価。
    let glyphs = |s: &TextLayerState| -> Vec<TextItem> {
        items_of(s, "0")
            .iter()
            .filter(|it| matches!(it, TextItem::Glyph { .. }))
            .cloned()
            .collect()
    };
    assert_eq!(
        glyphs(&experiment),
        glyphs(&control),
        "Cursor cue はグリフ列を変えない"
    );

    // reveal 時刻列もバイト等価（CursorMove は reveal 枠を消費しない）。
    assert_eq!(
        reveal_times_of(&experiment, "0"),
        reveal_times_of(&control, "0"),
        "Cursor cue は reveal 時刻列を変えない"
    );

    // 全時刻で可視グリフ数が一致（reveal 進行が Cursor に非依存）。
    let actor = ActorKey::from("0");
    for i in 0..30 {
        let t = i as f64 * 0.05;
        assert_eq!(
            experiment.visible_glyphs(&actor, t),
            control.visible_glyphs(&actor, t),
            "可視グリフ数は Cursor cue の有無で一致すべき（t={t}）"
        );
    }
}

// ── R5.1/R5.3/R9.5: choices は items と同一ライフサイクル（Clear/ClearAll で同時初期化） ──

/// `Clear` は対象 actor の choices を items と同時に初期化する（データ形で 5.1/5.3 を保証）。
#[test]
fn clear_resets_choices_alongside_items() {
    let mut state = TextLayerState::default();
    state.apply_cue(&choice_cue("0", 0.0, "q0", "はい", &[]));
    assert_eq!(choices_of(&state, "0").len(), 1);

    state.apply_cue(&cue("0", 0.5, CueCommand::Clear));

    let actor = state
        .actor_state(&ActorKey::from("0"))
        .expect("actor state should exist");
    assert_eq!(
        actor,
        &ActorTextState::default(),
        "Clear は choices も items と同時に初期化する"
    );
}

/// `ClearAll` は全 actor スコープの choices を items と同時に初期化する（R5.3）。
#[test]
fn clear_all_resets_choices_of_every_actor_scope() {
    let mut state = TextLayerState::default();
    state.apply_cue(&choice_cue("0", 0.0, "q0", "はい", &[]));
    state.apply_cue(&choice_cue("1", 0.1, "q1", "いいえ", &[]));

    state.apply_cue(&cue("0", 1.0, CueCommand::ClearAll));

    assert!(choices_of(&state, "0").is_empty());
    assert!(
        choices_of(&state, "1").is_empty(),
        "ClearAll は名指ししていないスコープの choices も消去する"
    );
}

// ── Balloon 向けでない command の防御的無視 ──

#[test]
fn non_balloon_commands_do_not_disturb_state() {
    let mut state = TextLayerState::default();
    state.apply_cue(&cue("0", 0.0, CueCommand::Text("A".into())));

    let before = state.clone();
    state.apply_cue(&cue(
        "0",
        0.1,
        CueCommand::Emote {
            key: "smile".into(),
        },
    ));
    state.apply_cue(&cue("0", 0.2, CueCommand::EntityRef(42)));
    // BalloonSurface（バルーン面切替）は表示系の消費対象＝文字状態機械へは配送しない（R3.2）。
    // 適用しても文字状態（items／reveal／visible_glyphs）は完全に不変。
    state.apply_cue(&cue(
        "0",
        0.3,
        CueCommand::BalloonSurface { key: "2".into() },
    ));
    assert_eq!(state, before);
}

// ── TextLayerConfig 既定値（design.md 正準: 行間 line_gap=2） ──

#[test]
fn config_defaults_match_design_canon() {
    let config = TextLayerConfig::default();
    // reveal は配送 duration 由来——config は line_gap のみ。
    assert_eq!(config.line_gap, 2.0);
}

// ── 行送りの式（design.md §4.1 正典表・R1.2/R3.1/R3.5/R1.6） ──

/// 行送り＝フォント高さ＋行間（切り上げなし）。正典表の 3 例をそのまま固定する。
#[test]
fn line_pitch_adds_line_gap_without_ceiling() {
    let config = TextLayerConfig::default();
    assert_eq!(
        config.line_pitch(28.0),
        30.0,
        "emo2-kakukaku の font.height,28"
    );
    assert_eq!(config.line_pitch(12.0), 14.0, "既定フォント高さ 12");
    assert_eq!(config.line_pitch(10.0), 12.0, "構造テストの font 10");
    // 旧式（切り上げつきの係数倍）なら 35／15／13 になる——差が出る値を選んである。
}

/// 行間を変えると行送りがそのぶんだけ動く（行間は `TextLayerConfig` で可変・R1.6）。
#[test]
fn line_pitch_follows_non_default_line_gap() {
    let config = TextLayerConfig { line_gap: 5.0 };
    assert_eq!(config.line_pitch(10.0), 15.0);
    assert_eq!(TextLayerConfig { line_gap: 0.0 }.line_pitch(10.0), 10.0);
}

/// 決定論の代役（`FixedMetrics`）は自前の足し算を持たず config の式を呼ぶ（R3.5）。
#[test]
fn fixed_metrics_line_pitch_delegates_to_config() {
    use crate::layout::{FixedMetrics, GlyphMetrics};

    let config = TextLayerConfig::default();
    for height in [10.0_f32, 12.0, 28.0, 40.0] {
        assert_eq!(
            FixedMetrics.line_pitch(height),
            config.line_pitch(height),
            "font_height {height} で代役と config の式が食い違う"
        );
    }
}

/// 非有限の行間は警告 1 件つきで 0 へ縮退する（log-first・R1.6）。
#[test]
fn normalized_degrades_nonfinite_line_gap_to_zero_with_warning() {
    let (config, counts) = count_levels(|| TextLayerConfig { line_gap: f32::NAN }.normalized());

    assert_eq!(config.line_gap, 0.0);
    assert_eq!(
        config.line_pitch(28.0),
        28.0,
        "縮退後は行送り＝フォント高さ"
    );
    assert_eq!(counts.warn, 1, "縮退はログ無しで起きない（警告 1 件）");
}

/// 負の行間も同じく警告 1 件つきで 0 へ縮退する（`line_pitch >= font_height` の不変条件）。
#[test]
fn normalized_degrades_negative_line_gap_to_zero_with_warning() {
    let (config, counts) = count_levels(|| TextLayerConfig { line_gap: -3.0 }.normalized());

    assert_eq!(config.line_gap, 0.0);
    assert_eq!(counts.warn, 1, "縮退はログ無しで起きない（警告 1 件）");
}

/// 妥当な行間は値も警告も変えない（正常経路で警告を出さない）。
#[test]
fn normalized_keeps_valid_line_gap_without_warning() {
    // 出る側の対照——同じ窓・同じ発行点で、負の行間から 1 件は必ず出る。
    let ((config, _), counts) = count_levels(|| {
        (
            TextLayerConfig::default().normalized(),
            TextLayerConfig { line_gap: -1.0 }.normalized(),
        )
    });

    assert_eq!(config, TextLayerConfig::default());
    assert_eq!(counts.warn, 1, "対照の負の行間の 1 件だけ");
}
