//! 選択肢の「済み」の境目の檻（areka-P0-balloon-text-bugs 件 1・要件 1.10〜1.13）。
//!
//! タスク 4.3 の部分は GPU なしで、場所の状態（`ActorTextState`）の境目と層の状態
//! （`TextLayerState`）の口だけを固定する: 境目の範囲・消去（`\c`・台詞の頭）で 0 に戻ること・
//! 済みにした後に足された選択肢は済んでいないこと・アンカーは済みにならないこと・
//! 場所が無ければ何もしないこと・空回しの写しが境目も写すこと。
//!
//! タスク 4.4 の部分も GPU なしで、ランタイム（`TextLayerRuntime`）の済みの口を固定する: 選択待ちの
//! 前の押下の口は何もしない・後ならその場所の選択肢の問いが偽でほかの場所は真・全部済みで偽・
//! アンカーは押せる範囲に残る・強調が外れる・当たり行の写しから選択肢の行がその場で消える。
//! 提示で写しを作り直すときの外し方は実 GPU の檻（タスク 4.7）が固定する。
//!
//! タスク 4.7 の部分は実 GPU で、普通のバルーンと箱の場所の両方について、済んだ行の座標の押下が
//! 当たらないこと・WARN 0 行・済んだ行の強調が読み戻しに出ないこと・済む前と後で強調なしの読み戻しが
//! 同じことを固定する（押下のハンドラからの通しは `areka` の `balloon_choice_settle_tests.rs`）。

use areka_sakura::contract::{ActorKey, CueCommand, TalkCue};

use crate::place::PlaceKey;
use crate::state::{ActorTextState, ChoiceSpan, SpanKind, TextLayerState};

fn cue(actor: &str, command: CueCommand) -> TalkCue {
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(actor),
        command,
        duration: 0.0,
    }
}

fn choice(actor: &str, id: &str) -> TalkCue {
    cue(
        actor,
        CueCommand::Choice {
            id: id.to_owned(),
            text: id.to_owned(),
            references: Vec::new(),
        },
    )
}

/// アンカーの範囲を 1 つ記録する cue の列（開き → 文字 → 閉じ）。
fn anchor(actor: &str, id: &str) -> [TalkCue; 3] {
    [
        cue(
            actor,
            CueCommand::AnchorBegin {
                id: id.to_owned(),
                references: Vec::new(),
            },
        ),
        cue(actor, CueCommand::Text(id.to_owned())),
        cue(actor, CueCommand::AnchorEnd),
    ]
}

fn balloon(actor: &str) -> PlaceKey {
    PlaceKey::balloon(&ActorKey::from(actor))
}

fn place<'a>(state: &'a TextLayerState, actor: &str) -> &'a ActorTextState {
    state
        .place_state(&balloon(actor))
        .expect("場所の状態がある")
}

/// 場所の範囲ごとに「済んだ選択肢か」を並べる（序数順）。
fn settled_flags(state: &TextLayerState, actor: &str) -> Vec<(SpanKind, bool)> {
    let s = place(state, actor);
    s.choices()
        .iter()
        .map(|span: &ChoiceSpan| (span.kind, s.is_settled_choice(span)))
        .collect()
}

fn applied(cues: &[TalkCue]) -> TextLayerState {
    let mut state = TextLayerState::default();
    for c in cues {
        state.apply_cue(c);
    }
    state
}

/// 境目の範囲: 済ませる前は全部が済んでおらず、済ませると今ある選択肢の全部が済む
/// （境目は範囲の数で止まる＝2 回済ませても同じ）。
#[test]
fn settle_moves_the_boundary_to_the_span_count_and_stops_there() {
    let mut state = applied(&[choice("0", "a"), choice("0", "b")]);
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, false), (SpanKind::Choice, false)]
    );
    assert!(place(&state, "0").has_live_choice());

    assert!(state.settle_choices_at(&balloon("0")));
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, true), (SpanKind::Choice, true)]
    );
    assert!(!place(&state, "0").has_live_choice());

    let once = state.clone();
    assert!(state.settle_choices_at(&balloon("0")));
    assert_eq!(state, once, "2 回目は何も変えない");
}

/// 範囲の無い場所: 済ませても何も起きず、済んでいない選択肢も無い。
#[test]
fn settle_on_a_place_without_spans_is_harmless() {
    let mut state = applied(&[cue("0", CueCommand::Text("字".into()))]);
    assert!(!place(&state, "0").has_live_choice());
    assert!(state.settle_choices_at(&balloon("0")));
    assert!(place(&state, "0").choices().is_empty());
    assert!(!place(&state, "0").has_live_choice());
}

/// 済みにした後に足された選択肢（序数が境目以上）は済んでいない。
#[test]
fn choices_added_after_settling_are_not_settled() {
    let mut state = applied(&[choice("0", "a")]);
    state.settle_choices_at(&balloon("0"));
    state.apply_cue(&choice("0", "b"));
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, true), (SpanKind::Choice, false)]
    );
    assert!(place(&state, "0").has_live_choice());
}

/// アンカーは境目より前でも済みにならない。アンカーしか無い場所には済んでいない選択肢が無い。
#[test]
fn anchors_are_never_settled() {
    let mut cues = vec![choice("0", "a")];
    cues.extend(anchor("0", "link"));
    let mut state = applied(&cues);
    state.settle_choices_at(&balloon("0"));
    assert_eq!(
        settled_flags(&state, "0"),
        vec![(SpanKind::Choice, true), (SpanKind::Anchor, false)]
    );

    let only_anchor = applied(&anchor("1", "link"));
    assert!(!place(&only_anchor, "1").has_live_choice());
}

/// `\c`（`Clear`）で境目が 0 に戻る: 消した後の新しい選択肢は済んでいない。
#[test]
fn clear_resets_the_boundary() {
    let mut state = applied(&[choice("0", "a"), choice("0", "b")]);
    state.settle_choices_at(&balloon("0"));
    state.apply_cue(&cue("0", CueCommand::Clear));
    state.apply_cue(&choice("0", "c"));
    assert_eq!(settled_flags(&state, "0"), vec![(SpanKind::Choice, false)]);
    assert!(place(&state, "0").has_live_choice());
}

/// 台詞の頭（`ClearAll`）でも全部の場所の境目が 0 に戻る。
#[test]
fn clear_all_resets_the_boundary_of_every_place() {
    let mut state = applied(&[choice("0", "a"), choice("1", "b")]);
    state.settle_all_choices();
    state.apply_cue(&cue("0", CueCommand::ClearAll));
    state.apply_cue(&choice("0", "c"));
    state.apply_cue(&choice("1", "d"));
    assert_eq!(settled_flags(&state, "0"), vec![(SpanKind::Choice, false)]);
    assert_eq!(settled_flags(&state, "1"), vec![(SpanKind::Choice, false)]);
}

/// 場所を指す口はその場所だけを済ませ、場所が無ければ何もしない（偽を返し、場所も作らない）。
#[test]
fn settle_at_touches_only_that_place_and_ignores_missing_places() {
    let mut state = applied(&[choice("0", "a"), choice("1", "b")]);
    assert!(state.settle_choices_at(&balloon("0")));
    assert!(!place(&state, "0").has_live_choice());
    assert!(place(&state, "1").has_live_choice(), "ほかの場所は済まない");

    let before = state.clone();
    assert!(!state.settle_choices_at(&balloon("9")));
    assert_eq!(state, before, "無い場所は作らず何も変えない");
}

/// 全部の場所を済ませる口: どの場所にも済んでいない選択肢が残らない（アンカーは残る）。
#[test]
fn settle_all_settles_every_place() {
    let mut cues = vec![choice("0", "a"), choice("1", "b")];
    cues.extend(anchor("1", "link"));
    let mut state = applied(&cues);
    state.settle_all_choices();
    assert!(state.places().all(|(_, s)| !s.has_live_choice()));
    assert_eq!(
        settled_flags(&state, "1"),
        vec![(SpanKind::Choice, true), (SpanKind::Anchor, false)]
    );
}

/// 空回しの写しは境目も写す。
#[test]
fn rehearsal_copy_carries_the_boundary() {
    let mut state = applied(&[choice("0", "a")]);
    state.settle_choices_at(&balloon("0"));
    let copy = state.rehearsal_copy();
    assert!(!place(&copy, "0").has_live_choice());
    assert_eq!(settled_flags(&copy, "0"), vec![(SpanKind::Choice, true)]);
}

// ══ タスク 4.4: ランタイムの済みの口（写し・強調・問いを同時に揃える・GPU なし） ══

use super::{ChoiceHitRow, TextLayerRuntime};
use crate::choice::HitRectPx;
use crate::state::TextLayerConfig;

/// 範囲の列から当たり行の写しを作る（提示の代わり・矩形は使わないので 0）。
fn seed_snapshot(rt: &mut TextLayerRuntime, actor: &str) {
    let place = balloon(actor);
    let rows: Vec<ChoiceHitRow> = rt
        .state
        .place_state(&place)
        .expect("場所の状態がある")
        .choices()
        .iter()
        .map(|span| ChoiceHitRow {
            kind: span.kind,
            ordinal: span.ordinal,
            id: span.id.clone(),
            label: span.label.clone(),
            references: span.references.clone(),
            rect: HitRectPx {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            },
        })
        .collect();
    rt.choice_snapshot.insert(place, rows);
}

/// スコープ 0 に選択肢 2 つとアンカー 1 つ、スコープ 1 に選択肢 1 つ。写しを入れ、
/// スコープ 0 の選択肢 0・スコープ 1 の選択肢 0 に強調を入れる。
fn runtime_with_choices() -> TextLayerRuntime {
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let mut cues = vec![choice("0", "a"), choice("0", "b")];
    cues.extend(anchor("0", "link"));
    cues.push(choice("1", "c"));
    for c in &cues {
        rt.apply_cue(c);
    }
    seed_snapshot(&mut rt, "0");
    seed_snapshot(&mut rt, "1");
    rt.inject_choice_hover_at(&balloon("0"), Some(0));
    rt.inject_choice_hover_at(&balloon("1"), Some(0));
    rt
}

fn row_kinds(rt: &TextLayerRuntime, actor: &str) -> Vec<(SpanKind, usize)> {
    rt.choice_hit_rows_at(&balloon(actor))
        .iter()
        .map(|row| (row.kind, row.ordinal))
        .collect()
}

fn hover(rt: &TextLayerRuntime, actor: &str) -> Option<usize> {
    rt.choice_hover.get(&balloon(actor)).copied().flatten()
}

/// 選択待ちに入る前の押下の口は何もしない（押し直せるまま残す）。
#[test]
fn press_before_the_choice_wait_does_nothing() {
    let mut rt = runtime_with_choices();
    let rows_before = row_kinds(&rt, "0");
    rt.settle_choices_at(&balloon("0"));
    assert!(
        rt.choice_active(&ActorKey::from("0")),
        "選択肢の問いは真のまま"
    );
    assert_eq!(row_kinds(&rt, "0"), rows_before, "当たり行は残る");
    assert_eq!(hover(&rt, "0"), Some(0), "強調も残る");
    assert!(
        !place(&rt.state, "0")
            .choices()
            .iter()
            .any(|s| place(&rt.state, "0").is_settled_choice(s))
    );
}

/// 選択待ちに入った後の押下の口: その場所の選択肢が済み、問いが偽・ほかの場所は真・
/// 写しから選択肢の行がその場で消え（アンカーの行は残る）・済んだ選択肢を指す強調が外れる。
#[test]
fn press_after_the_choice_wait_settles_that_place_at_once() {
    let mut rt = runtime_with_choices();
    rt.note_choice_wait_began();
    rt.settle_choices_at(&balloon("0"));

    assert!(
        !rt.choice_active(&ActorKey::from("0")),
        "その場所の選択肢の問いは偽"
    );
    assert!(
        rt.choice_active(&ActorKey::from("1")),
        "ほかの場所は真のまま"
    );
    assert!(
        rt.hit_active(&ActorKey::from("0")),
        "アンカーは押せる範囲に残る"
    );
    assert_eq!(
        row_kinds(&rt, "0"),
        vec![(SpanKind::Anchor, 2)],
        "写しから選択肢の行が消え、アンカーの行は残る（提示を待たない）"
    );
    assert_eq!(hover(&rt, "0"), None, "済んだ選択肢を指す強調は外れる");
    assert_eq!(row_kinds(&rt, "1"), vec![(SpanKind::Choice, 0)]);
    assert_eq!(hover(&rt, "1"), Some(0), "ほかの場所の強調は残る");
}

/// アンカーを指す強調は、場所が済んでも外れない。
#[test]
fn hover_on_an_anchor_survives_settling() {
    let mut rt = runtime_with_choices();
    rt.inject_choice_hover_at(&balloon("0"), Some(2));
    rt.note_choice_wait_began();
    rt.settle_choices_at(&balloon("0"));
    assert_eq!(hover(&rt, "0"), Some(2));
}

/// 全部の場所を済みにする内部の口: どこにも済んでいない選択肢が残らず、写しはアンカーの行だけ・
/// 強調は外れる（選択待ちに入っていなくても効く＝トークの終わりの口）。
#[test]
fn settle_all_settles_every_place_and_aligns_rows_and_hover() {
    let mut rt = runtime_with_choices();
    rt.settle_all_choices();
    assert!(!rt.choice_active(&ActorKey::from("0")));
    assert!(!rt.choice_active(&ActorKey::from("1")));
    assert!(rt.hit_active(&ActorKey::from("0")), "アンカーは残る");
    assert_eq!(row_kinds(&rt, "0"), vec![(SpanKind::Anchor, 2)]);
    assert!(row_kinds(&rt, "1").is_empty());
    assert_eq!(hover(&rt, "0"), None);
    assert_eq!(hover(&rt, "1"), None);
}

/// 戻す口の後は、押下の口がまた何もしなくなる。
#[test]
fn closing_the_choice_wait_makes_the_press_a_no_op_again() {
    let mut rt = runtime_with_choices();
    rt.note_choice_wait_began();
    rt.close_choice_wait();
    rt.settle_choices_at(&balloon("0"));
    assert!(rt.choice_active(&ActorKey::from("0")));
    assert_eq!(row_kinds(&rt, "0").len(), 3);
}

// ══ タスク 4.7: 済んだ行の振る舞い（実 GPU・普通のバルーンと箱） ══

use bevy_ecs::prelude::World;
use log_capture_kit::capture;

use super::box_sync_tests::{SHELL_IMAGE, emote, layout, name, resolver, select};
use super::test_support::{choice_cue, com_world};
use super::{ResolvedBalloonText, TextSlotBinding, present_frame};
use crate::place::TextPlace;

/// 全部の文字が見えている時刻。
const LATE: f64 = 10.0;

/// 押下の口が読む値（押せる範囲があるか・当たり行）で、その座標に当たる行があるか。
/// 入力の層の `click_selection`／`judge_box_click` と同じ判定（押せる範囲が無ければ当たらない・
/// 矩形は左上を含み右下を含まない）。
fn press_hits(rt: &TextLayerRuntime, place: &PlaceKey, x: f32, y: f32) -> bool {
    rt.hit_active(&place.actor)
        && rt.choice_hit_rows_at(place).iter().any(|row| {
            let r = row.rect;
            x >= r.left && x < r.right && y >= r.top && y < r.bottom
        })
}

/// 済んだ行の通しの檻（要件 1.10〜1.12）。`place` に選択肢 2 つを載せて装着済みの `rt` を受ける。
///
/// 提示→当たり行あり→選択待ちの始まり→強調したまま場所を済み→その座標の押下は当たらない・
/// 済み・提示で WARN 0 行／済む前と後で強調なしの読み戻しが同じ／済んだ行に強調を入れても強調なしと同じ。
/// 済む前の強調が読み戻しを変えることを先に確かめ、読み戻しの比べが効くことを担保する。
fn settled_row_is_unpressable_and_unlit(
    rt: &mut TextLayerRuntime,
    world: &mut World,
    place: &PlaceKey,
) {
    let read = |rt: &TextLayerRuntime| {
        rt.surface_at(place)
            .expect("供給面")
            .read_back()
            .expect("read_back")
    };
    present_frame(rt, world, LATE).expect("提示");
    let rows = rt.choice_hit_rows_at(place).to_vec();
    let first = rows
        .iter()
        .find(|r| r.kind == SpanKind::Choice)
        .expect("前提: 選択肢の当たり行がある")
        .clone();
    let (x, y) = (
        (first.rect.left + first.rect.right) / 2.0,
        (first.rect.top + first.rect.bottom) / 2.0,
    );
    assert!(
        press_hits(rt, place, x, y),
        "前提: 済む前はその座標の押下が当たる"
    );
    let plain = read(rt);
    rt.inject_choice_hover_at(place, Some(first.ordinal));
    present_frame(rt, world, LATE).expect("強調の提示");
    assert_ne!(
        read(rt),
        plain,
        "前提: 済む前の強調は読み戻しを変える（比べが効く）"
    );

    // 押下を送れた後の口（選択待ちの始まりの後・強調したまま）。済みと次の提示の記録を檻に入れる。
    rt.note_choice_wait_began();
    let ((), events) = capture(|| {
        rt.settle_choices_at(place);
        present_frame(rt, world, LATE).expect("済んだ後の提示");
        let _ = press_hits(rt, place, x, y);
        tracing::warn!("対照の警告");
    });
    let warns: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .collect();
    assert_eq!(
        warns.len(),
        1,
        "数えられるのは対照の 1 行だけ（済み・提示・押下の判定で WARN 0 行）: {warns:?}"
    );
    assert!(
        !press_hits(rt, place, x, y),
        "済んだ行の座標の押下は選択にならない（要件 1.11）"
    );
    assert_eq!(
        read(rt),
        plain,
        "済む前と後で強調なしの読み戻しが同じ（強調は済みで外れる・要件 1.10・1.12）"
    );

    rt.inject_choice_hover_at(place, Some(first.ordinal));
    present_frame(rt, world, LATE).expect("済んだ行へ強調を入れた提示");
    assert_eq!(
        read(rt),
        plain,
        "済んだ行に強調を入れても読み戻しは強調なしと同じ（要件 1.10）"
    );
}

/// 普通のバルーンの場所（要件 1.10〜1.12）。
#[test]
fn settled_balloon_row_is_unpressable_and_unlit_on_the_gpu() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &[]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &[]));
    let image = (120u32, 60u32);
    rt.register_actor(
        actor.clone(),
        TextSlotBinding::new(slot, window, 1.0, image, image),
        ResolvedBalloonText::resolve(&super::test_support::geo_model(), image),
    );

    settled_row_is_unpressable_and_unlit(&mut rt, &mut world, &balloon("0"));
    assert!(!rt.choice_active(&actor), "場所の選択肢が全部済んだ");
}

/// 箱の場所（要件 1.13）。サーフェス 0 の箱 b に選択肢を出す。
#[test]
fn settled_box_row_is_unpressable_and_unlit_on_the_gpu() {
    let (mut world, window, slot) = com_world();
    let actor = ActorKey::from("0");
    let mut rt = TextLayerRuntime::new(TextLayerConfig::default());
    let layout = layout();
    rt.set_box_layout(&mut world, layout.clone(), resolver(), vec![]);
    rt.apply_cue(&emote("0", "0"));
    rt.apply_cue(&select("0", "b"));
    rt.apply_cue(&choice_cue("0", 0.0, "OnYes", "はい", &[]));
    rt.apply_cue(&choice_cue("0", 0.2, "OnNo", "いいえ", &[]));
    let shell = TextSlotBinding::new(slot, window, 1.0, SHELL_IMAGE, SHELL_IMAGE);
    rt.sync_box_bindings(&mut world, &[(actor.clone(), Some((shell, 0)))]);

    let box_b = PlaceKey {
        actor: actor.clone(),
        place: TextPlace::Box(name(&layout, 0, "b")),
    };
    settled_row_is_unpressable_and_unlit(&mut rt, &mut world, &box_b);
    assert!(!rt.choice_active(&actor), "箱の選択肢が全部済んだ");
}

/// 同じスコープの別の箱に済んでいない選択肢が残るあいだは、選択肢の問いは真のまま
/// （バルーンの時間切れを止めたまま・要件 1.13・GPU なし）。
#[test]
fn a_live_choice_in_another_box_of_the_scope_keeps_choice_active() {
    let mut f = super::box_sync_tests::fixture();
    let actor = ActorKey::from("0");
    f.show("0");
    f.rt.apply_cue(&select("0", "a"));
    f.rt.apply_cue(&choice("0", "x"));
    f.rt.apply_cue(&select("0", "b"));
    f.rt.apply_cue(&choice("0", "y"));
    f.rt.note_choice_wait_began();
    let (box_a, box_b) = (f.key(0, "a"), f.key(0, "b"));

    f.rt.settle_choices_at(&box_b);
    assert!(
        f.rt.choice_active(&actor),
        "箱 a の選択肢は済んでいない＝問いは真のまま"
    );
    f.rt.settle_choices_at(&box_a);
    assert!(!f.rt.choice_active(&actor), "両方の箱が済むと偽");
}
