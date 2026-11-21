use super::test_support::{TWO_BOX_SHELL, row};
use super::*;
use areka_sakura::ActorKey;

// -------------------------------------------------------------------------
// hit_choice_row 純関数檻
//
// 上流実型 `ChoiceHitRow`／`HitRectPx`（全 pub フィールド）で fixture を組み、
// 包含／境界（半開）／行外／空／複数行／病的重なりを GPU・実窓不要で決定的に判定する。
// -------------------------------------------------------------------------

use areka_emo_text::state::SpanKind;

/// 内側包含: 矩形の内部点はその行 index を返す。
#[test]
fn hit_inside_rect_returns_index() {
    let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];
    assert_eq!(
        hit_choice_row(&rows, 30.0, 30.0),
        Some(0),
        "矩形内部の点はヒット index を返す"
    );
}

/// 空 rows: 判定対象が無ければ常に None。
#[test]
fn hit_empty_rows_returns_none() {
    let rows: [ChoiceHitRow; 0] = [];
    assert_eq!(hit_choice_row(&rows, 30.0, 30.0), None, "空 rows は None");
}

/// 行外（非ヒット→None）: 矩形外の点は None。四方向すべて外側を確認。
#[test]
fn hit_outside_rect_returns_none() {
    let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];
    assert_eq!(hit_choice_row(&rows, 5.0, 30.0), None, "左外");
    assert_eq!(hit_choice_row(&rows, 60.0, 30.0), None, "右外");
    assert_eq!(hit_choice_row(&rows, 30.0, 10.0), None, "上外");
    assert_eq!(hit_choice_row(&rows, 30.0, 50.0), None, "下外");
}

/// 半開区間の境界: `left`/`top` 辺は包含・
/// `right`/`bottom` 辺は非包含（whole-pixel 行矩形と整合）。
#[test]
fn hit_half_open_boundary_edges() {
    let rows = [row(0, 10.0, 20.0, 50.0, 40.0)];

    // left/top 辺は包含。
    assert_eq!(hit_choice_row(&rows, 10.0, 30.0), Some(0), "left 辺は包含");
    assert_eq!(hit_choice_row(&rows, 30.0, 20.0), Some(0), "top 辺は包含");
    assert_eq!(hit_choice_row(&rows, 10.0, 20.0), Some(0), "左上角は包含");

    // right/bottom 辺は非包含。
    assert_eq!(hit_choice_row(&rows, 50.0, 30.0), None, "right 辺は非包含");
    assert_eq!(hit_choice_row(&rows, 30.0, 40.0), None, "bottom 辺は非包含");
    assert_eq!(hit_choice_row(&rows, 50.0, 40.0), None, "右下角は非包含");
}

/// 複数の非重複行のうち正しい行: 点を含む行の
/// index が返る。各行が独立に判定される。
#[test]
fn hit_multiple_non_overlapping_rows_returns_correct_index() {
    let rows = [
        row(0, 0.0, 0.0, 100.0, 20.0),  // index 0
        row(1, 0.0, 20.0, 100.0, 40.0), // index 1
        row(2, 0.0, 40.0, 100.0, 60.0), // index 2
    ];
    assert_eq!(hit_choice_row(&rows, 50.0, 10.0), Some(0), "1 行目内");
    assert_eq!(hit_choice_row(&rows, 50.0, 30.0), Some(1), "2 行目内");
    assert_eq!(hit_choice_row(&rows, 50.0, 50.0), Some(2), "3 行目内");
    assert_eq!(hit_choice_row(&rows, 50.0, 70.0), None, "全行外");
}

/// 病的重なり→最終一致: 点を含む行が複数あっても、逆順走査の最初の
/// 一致＝スライス**最終** index を決定的に返す（後定義が手前・画家のアルゴリズム）。
/// 期待 index を各行で一意にするため、重なり方を非対称にして「最初一致」なら別 index に
/// なる配置を用いる（最初一致=0／最終一致=2 を弁別）。
#[test]
fn hit_pathological_overlap_returns_last_match_deterministically() {
    // 3 行が (50, 30) を共通に含む。スライス順（index 昇順）で最後の index 2 が返るべき。
    let rows = [
        row(0, 0.0, 0.0, 100.0, 100.0), // index 0: 点を含む
        row(1, 40.0, 20.0, 60.0, 40.0), // index 1: 点を含む
        row(2, 45.0, 25.0, 80.0, 60.0), // index 2: 点を含む（最後定義＝手前）
    ];
    assert_eq!(
        hit_choice_row(&rows, 50.0, 30.0),
        Some(2),
        "重なり時はスライス最終一致 index（逆順走査の最初一致）"
    );

    // 2 行重なりでも最終一致（index 1）が返る——最初一致 index 0 と弁別。
    let two = [
        row(0, 0.0, 0.0, 100.0, 100.0), // index 0: 点を含む
        row(1, 10.0, 10.0, 90.0, 90.0), // index 1: 点を含む（最後定義）
    ];
    assert_eq!(
        hit_choice_row(&two, 50.0, 50.0),
        Some(1),
        "2 行重なりでも最終一致 index を返す（最初一致 0 ではない）"
    );
}

// -------------------------------------------------------------------------
// hover_action 純関数檻（全分岐）
//
// active・hit_ordinal・last_injected の 3 入力から hover 遷移を決める純関数の
// 全分岐（非表示時無処理／消滅時自前整合／同値維持／新規注入／解除注入）を
// World・runtime 不要で決定的に網羅する。この檻と下の click_selection の檻の「表示中」「非表示」は
// `active`（押せる範囲＝選択肢かアンカーがあるか）の真偽を指す。
// -------------------------------------------------------------------------

/// 非表示時無処理: `active == false` かつ `last_injected == None` は
/// 何もしない。hit_ordinal（Some/None いずれも）は無視される。
#[test]
fn hover_inactive_no_prior_injection_is_noop() {
    assert_eq!(
        hover_action(false, None, None),
        HoverAction::NoopInactive,
        "非表示・未注入・hit なしは NoopInactive"
    );
    assert_eq!(
        hover_action(false, Some(3), None),
        HoverAction::NoopInactive,
        "非表示・未注入は hit があっても NoopInactive（hit_ordinal 無視）"
    );
}

/// 消滅時自前整合: `active == false` かつ `last_injected == Some(k)` は
/// 自前状態を None 整合する `ResetOwnState`（inject はしない＝上流原子性が正本）。
/// hit_ordinal（Some/None いずれも）は無視される。
#[test]
fn hover_inactive_with_prior_injection_resets_own_state() {
    assert_eq!(
        hover_action(false, None, Some(2)),
        HoverAction::ResetOwnState,
        "非表示・注入済・hit なしは ResetOwnState"
    );
    assert_eq!(
        hover_action(false, Some(5), Some(2)),
        HoverAction::ResetOwnState,
        "非表示・注入済は hit があっても ResetOwnState（hit_ordinal 無視）"
    );
}

/// 同値維持（遷移なし）: 表示中で hit_ordinal == last_injected は再注入しない Keep。
/// Some==Some と None==None の双方を確認する。
#[test]
fn hover_active_same_value_keeps() {
    assert_eq!(
        hover_action(true, Some(2), Some(2)),
        HoverAction::Keep,
        "表示中・同一 Some は Keep（再注入しない）"
    );
    assert_eq!(
        hover_action(true, None, None),
        HoverAction::Keep,
        "表示中・None 同値は Keep（ハイライト無しのまま遷移なし）"
    );
}

/// 新規注入: 表示中で hover 対象が変化したら新値を Inject する。
/// None→Some の初回注入と Some→Some の遷移の双方を確認する。
#[test]
fn hover_active_new_row_injects() {
    assert_eq!(
        hover_action(true, Some(1), None),
        HoverAction::Inject(Some(1)),
        "表示中・None→Some(1) は Inject(Some(1))（初回ハイライト）"
    );
    assert_eq!(
        hover_action(true, Some(3), Some(1)),
        HoverAction::Inject(Some(3)),
        "表示中・Some(1)→Some(3) は Inject(Some(3))（行遷移）"
    );
}

/// 解除注入: 表示中で hover が行外へ出たら None を Inject する
/// （Some→None＝ハイライト解除の遷移）。
#[test]
fn hover_active_leaves_row_injects_none() {
    assert_eq!(
        hover_action(true, None, Some(2)),
        HoverAction::Inject(None),
        "表示中・Some(2)→None は Inject(None)（ハイライト解除）"
    );
}

// -------------------------------------------------------------------------
// apply_hover（場所への注入・借用の失敗）
// -------------------------------------------------------------------------

fn emo_text_debugs(
    events: &[log_capture_kit::CapturedEvent],
) -> Vec<&log_capture_kit::CapturedEvent> {
    events
        .iter()
        .filter(|e| e.target == "areka_emo_text::actor" && e.level == tracing::Level::DEBUG)
        .collect()
}

/// 普通のバルーンにだけ ordinal 0 の選択肢があるので、文字の層が「無い ordinal」を記録するのは
/// 箱の場所へ注入したときだけ。注入の到達と場所の取り違えの両方がここで分かる。
#[test]
fn apply_hover_injects_at_the_given_place() {
    let rt = super::test_support::runtime_with_active_choice("0");
    let balloon = PlaceKey::balloon(&ActorKey::from("0"));
    let boxed = PlaceKey {
        actor: ActorKey::from("0"),
        place: TextPlace::Box(super::test_support::box_name(TWO_BOX_SHELL, "b")),
    };

    let (written, events) =
        log_capture_kit::capture(|| apply_hover(&rt, &balloon, 0, HoverAction::Inject(Some(0))));
    assert_eq!(written, Some(Some(0)));
    assert!(emo_text_debugs(&events).is_empty(), "{events:?}");

    let (written, events) =
        log_capture_kit::capture(|| apply_hover(&rt, &boxed, 0, HoverAction::Inject(Some(0))));
    assert_eq!(written, Some(Some(0)));
    let debugs = emo_text_debugs(&events);
    assert_eq!(debugs.len(), 1, "{events:?}");
    assert_eq!(debugs[0].field("ordinal"), Some("0"));
}

#[test]
fn apply_hover_reset_own_state_does_not_inject() {
    let rt = super::test_support::runtime_with_active_choice("0");
    let boxed = PlaceKey {
        actor: ActorKey::from("0"),
        place: TextPlace::Box(super::test_support::box_name(TWO_BOX_SHELL, "b")),
    };
    let (written, events) =
        log_capture_kit::capture(|| apply_hover(&rt, &boxed, 0, HoverAction::ResetOwnState));
    assert_eq!(written, Some(None));
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn apply_hover_borrow_failure_writes_nothing_and_logs_one_error() {
    let rt = super::test_support::runtime_with_active_choice("0");
    let balloon = PlaceKey::balloon(&ActorKey::from("0"));
    let held = rt.borrow();
    let (written, events) =
        log_capture_kit::capture(|| apply_hover(&rt, &balloon, 0, HoverAction::Inject(Some(0))));
    drop(held);
    assert_eq!(written, None);
    let errors: Vec<_> = events
        .iter()
        .filter(|e| e.level == tracing::Level::ERROR)
        .collect();
    assert_eq!(errors.len(), 1, "{events:?}");
    assert_eq!(
        errors[0].field_str("event"),
        Some("balloon_runtime_borrow_failed")
    );
    assert!(
        events.iter().all(|e| e.level == tracing::Level::ERROR),
        "{events:?}"
    );
}

// -------------------------------------------------------------------------
// click_selection 純関数檻
//
// active・現行 rows・click 座標・scope の入力から確定 ChoiceSelection の構成を
// 決める純関数を、World・runtime・send 不要で決定的に判定する。
// ヒット時のフィールド一致（scope は arg 由来・ordinal 非含有）、非 hit／非表示での
// 非構成（None）を網羅する。
// -------------------------------------------------------------------------

/// ヒット確定: 表示中・ヒット座標では現行ヒット行の全フィールドを
/// clone 転写した `ChoiceSelection` を返す。scope は arg 由来・ordinal は非含有。
#[test]
fn click_hit_builds_selection_from_current_row() {
    let rows = [
        ChoiceHitRow {
            references: vec!["a".to_string()],
            ..row(0, 0.0, 0.0, 100.0, 20.0)
        },
        ChoiceHitRow {
            references: vec!["r0".to_string(), "r1".to_string()],
            ..row(1, 0.0, 20.0, 100.0, 40.0)
        },
    ];
    // (50, 30) は index 1 の行内。
    let sel = click_selection(true, &rows, 50.0, 30.0, 7)
        .expect("表示中・ヒット座標では ChoiceSelection を構成する");
    assert_eq!(sel.id, "q1", "id は現行ヒット行から転写");
    assert_eq!(sel.label, "label1", "label は現行ヒット行から転写");
    assert_eq!(
        sel.scope, 7,
        "scope は引数由来（BalloonWindowMarker.scope）"
    );
    assert_eq!(
        sel.references,
        vec!["r0".to_string(), "r1".to_string()],
        "references は現行ヒット行から忠実転写"
    );
}

// -------------------------------------------------------------------------
// 選択肢とアンカーが混ざった当たりの行
//
// 行の列は通し番号の昇順で、選択肢とアンカーが台本の順に混ざる。判定は種類の 2 段——選択肢を
// 後ろから見て、当たらなければアンカーを後ろから見る。
// -------------------------------------------------------------------------

use super::test_support::anchor_row;

/// 重なる矩形では選択肢が勝つ。台本での定義の順がどちらでも同じ。
#[test]
fn overlapping_choice_wins_over_anchor_in_either_order() {
    // どちらの並びでも (50, 30) を選択肢とアンカーの両方が含む。
    let choice_first = [
        row(0, 0.0, 0.0, 100.0, 100.0),
        anchor_row(1, 40.0, 20.0, 60.0, 40.0),
    ];
    assert_eq!(
        hit_choice_row(&choice_first, 50.0, 30.0),
        Some(0),
        "アンカーが後ろ（手前）にあっても選択肢"
    );
    let anchor_first = [
        anchor_row(0, 0.0, 0.0, 100.0, 100.0),
        row(1, 40.0, 20.0, 60.0, 40.0),
    ];
    assert_eq!(
        hit_choice_row(&anchor_first, 50.0, 30.0),
        Some(1),
        "選択肢が後ろにあれば、もちろん選択肢"
    );
    // 選択肢が 2 つ重なる所にアンカーが挟まっても、後ろの選択肢。
    let sandwiched = [
        row(0, 0.0, 0.0, 100.0, 100.0),
        anchor_row(1, 0.0, 0.0, 100.0, 100.0),
        row(2, 40.0, 20.0, 60.0, 40.0),
        anchor_row(3, 0.0, 0.0, 100.0, 100.0),
    ];
    assert_eq!(hit_choice_row(&sandwiched, 50.0, 30.0), Some(2));
    assert_eq!(
        hit_choice_row(&sandwiched, 10.0, 10.0),
        Some(0),
        "後ろの選択肢の外では、手前のアンカーより奥の選択肢"
    );
}

/// 選択肢に当たらずアンカーに当たればアンカー。アンカー同士が重なれば後ろの行。
/// どの範囲にも入らなければ無し。
#[test]
fn anchor_is_hit_only_where_no_choice_covers_the_point() {
    let rows = [
        row(0, 0.0, 0.0, 100.0, 20.0),
        anchor_row(1, 0.0, 20.0, 100.0, 40.0),
        anchor_row(2, 50.0, 20.0, 100.0, 40.0),
    ];
    assert_eq!(hit_choice_row(&rows, 10.0, 30.0), Some(1), "アンカーだけ");
    assert_eq!(
        hit_choice_row(&rows, 60.0, 30.0),
        Some(2),
        "アンカー同士は後ろの行"
    );
    assert_eq!(hit_choice_row(&rows, 10.0, 10.0), Some(0), "選択肢だけ");
    assert_eq!(hit_choice_row(&rows, 10.0, 50.0), None, "範囲の外");
}

/// 選択の知らせは当たった行の種類を写す。重なりでは選択肢の知らせになり、範囲の外では作らない。
#[test]
fn click_selection_copies_the_kind_of_the_hit_row() {
    let rows = [
        row(0, 0.0, 0.0, 100.0, 20.0),
        anchor_row(1, 0.0, 20.0, 100.0, 40.0),
        row(2, 0.0, 30.0, 100.0, 40.0),
    ];
    let kind_and_id = |y: f32| click_selection(true, &rows, 50.0, y, 0).map(|s| (s.kind, s.id));
    assert_eq!(kind_and_id(10.0), Some((SpanKind::Choice, "q0".into())));
    assert_eq!(kind_and_id(25.0), Some((SpanKind::Anchor, "q1".into())));
    assert_eq!(
        kind_and_id(35.0),
        Some((SpanKind::Choice, "q2".into())),
        "アンカーと選択肢が重なる所は選択肢の知らせ"
    );
    assert_eq!(kind_and_id(50.0), None, "範囲の外");
    assert_eq!(
        click_selection(false, &rows, 50.0, 25.0, 0),
        None,
        "押せる範囲が無い（消えた後）ならアンカーの上でも作らない"
    );
}
