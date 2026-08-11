use super::test_support::*;
use super::*;

// ---- apply_bind（動的 bind 適用結果に応じた表示発行の判定・要件 3.5/3.6/3.8・D5/D9） ----

/// 表示中シーンでの集合変化 → 現シェル surface id を載せた Show を新規発行（R3.5・D5）。
/// binds は直前集合∪{新 id}、surface_id は現在表示中の id と一致すること。
#[test]
fn apply_bind_shown_set_change_emits_changed_show() {
    let mut states = empty_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");

    // シェル面を Shown(2100) に確定させる。
    states.apply(&scope, SurfaceTarget::Show(2100));

    // 集合を変える bind（1302 を on） → Changed(Show{現 sid=2100, 新集合})。
    let outcome = states.apply_bind(&scope, 1302, true);
    assert_eq!(
        outcome,
        BindApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207, 1302]),
            pattern: PatternState::default(),
        }),
        "表示中シーンでの着せ替え変化は現 surface id を載せた新 Show を発行（R3.5・D5）"
    );
    // 動的集合として保持されていること。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1302])
    );
}

/// 非表示シーンでの集合変化 → 発行なし・状態のみ更新（D5：Hidden への強制 Show 禁止）。
#[test]
fn apply_bind_hidden_set_change_is_state_only() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");

    // シェル面を Hidden にする。
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Hidden));

    // 集合を変える bind → StateOnly（発行なし）。
    let outcome = states.apply_bind(&scope, 1302, true);
    assert_eq!(
        outcome,
        BindApplyOutcome::StateOnly,
        "非表示シーンでの着せ替え変化は発行せず状態のみ更新（D5）"
    );
    // 状態（動的集合）は更新されている＝次 Show へ保留。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1302]),
        "Hidden でも集合は積算保持（次 Show へ持ち越し・D5）"
    );
}

/// 未知（未表示）scope での集合変化 → 発行なし・状態のみ更新（D5）。
#[test]
fn apply_bind_unknown_scope_set_change_is_state_only() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");

    // 一度も Show していない scope。
    assert_eq!(states.scopes.get(&scope), None);

    let outcome = states.apply_bind(&scope, 1302, true);
    assert_eq!(
        outcome,
        BindApplyOutcome::StateOnly,
        "未知 scope への着せ替え変化は発行せず状態のみ更新（D5）"
    );
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1302]),
        "未知 scope でも集合は積算保持"
    );
    // シェル状態を作らない（Show を強制しない）。
    assert_eq!(states.scopes.get(&scope), None, "apply_bind はシェル状態を作らない");
}

/// 同一集合への変化 → Unchanged・状態書き込みなし・発行なし（冪等・R3.6・D9）。
#[test]
fn apply_bind_same_set_is_unchanged() {
    let mut states = empty_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    // 不在 id の off は結果集合が現状と同値 → Unchanged。
    let outcome = states.apply_bind(&scope, 9999, false);
    assert_eq!(
        outcome,
        BindApplyOutcome::Unchanged,
        "結果集合が直前と同値なら再発行不要（冪等・R3.6・D9）"
    );
    // 状態は書き込まれない（current_binds は依然 static 既定を返す）。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207]),
        "同値ゆえ dynamic_binds へ書き込まれず static 既定のまま"
    );

    // 既に含む id の on も同値 → Unchanged。
    let outcome2 = states.apply_bind(&scope, 1100, true);
    assert_eq!(outcome2, BindApplyOutcome::Unchanged);
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207])
    );
}

/// 冪等 on→on（R3.6・D9）: 表示中 scope で同一 (id, true) を 2 回。
/// 1 回目 Changed（集合変化）、2 回目 Unchanged（既に含む）。
#[test]
fn apply_bind_idempotent_on_on_shown() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let first = states.apply_bind(&scope, 1302, true);
    assert!(
        matches!(first, BindApplyOutcome::Changed(_)),
        "初回の集合変化は Changed"
    );

    let second = states.apply_bind(&scope, 1302, true);
    assert_eq!(
        second,
        BindApplyOutcome::Unchanged,
        "同一 id の再 on は集合不変ゆえ Unchanged（冪等・R3.6・D9）"
    );
}

/// 非退行（R3.8）: 一連の apply_bind はシェル面・バルーン面の状態機械を変更しない。
#[test]
fn apply_bind_does_not_touch_shell_or_balloon_state() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");

    // シェル面 Shown(2100)・バルーン面 Shown(2) を確定。
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    assert_eq!(states.scopes.get(&scope), Some(&ScopeState::Shown(2100)));
    assert_eq!(states.balloon.get(&scope), Some(&ScopeState::Shown(2)));

    // 複数 apply_bind を通しても scopes / balloon は不変（R3.8）。
    states.apply_bind(&scope, 1302, true);
    states.apply_bind(&scope, 1500, true);
    states.apply_bind(&scope, 1100, false);
    assert_eq!(
        states.scopes.get(&scope),
        Some(&ScopeState::Shown(2100)),
        "apply_bind はシェル面状態を変更しない（R3.8）"
    );
    assert_eq!(
        states.balloon.get(&scope),
        Some(&ScopeState::Shown(2)),
        "apply_bind はバルーン面状態を変更しない（R3.8）"
    );
}

/// 積算（R3.4 隣接）: 表示中 scope で 2 つの bind を続けて on。
/// 2 回目 Changed は直前集合を保持したまま両 id を載せる。
#[test]
fn apply_bind_accumulates_across_two_binds_when_shown() {
    let mut states = empty_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    // idA を on。
    let a = states.apply_bind(&scope, 1302, true);
    assert_eq!(
        a,
        BindApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207, 1302]),
            pattern: PatternState::default(),
        })
    );

    // idB を on → 直前集合 {1100,1207,1302} を保持したまま 1500 を追加。
    let b = states.apply_bind(&scope, 1500, true);
    assert_eq!(
        b,
        BindApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207, 1302, 1500]),
            pattern: PatternState::default(),
        }),
        "2 つ目の bind は直前状態を保持したまま積算（R3.4 隣接）"
    );
}

// ---- apply_bind_exclusive（排他カテゴリ〔mustselect／非宣言既定〕の排他置換・bindopt 2.1/3.1） ----

/// 排他置換（bindopt 2.1/3.1）: 表示中 scope で同カテゴリ全 ID を外し target のみ有効化する。
/// 現集合 {1301,1303,1207}・category_ids=[1301,1303,1304]・target=1304 →
/// 目カテゴリ旧パーツ(1301,1303) を外し 1304 を付与、非カテゴリ 1207 は保持 → {1207,1304}。
#[test]
fn apply_bind_exclusive_replaces_same_category_on_shown() {
    let mut states = ScopeStates::new(BindSet::from_ids([1301, 1303, 1207]));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let outcome = states.apply_bind_exclusive(&scope, &[1301, 1303, 1304], 1304);
    assert_eq!(
        outcome,
        BindApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1207, 1304]),
            pattern: PatternState::default(),
        }),
        "同カテゴリ旧パーツを外し target のみ有効・非カテゴリは保持（高々 1 パーツ・bindopt 2.1/3.1）"
    );
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1207, 1304]),
        "排他置換後の集合が動的集合として保持される"
    );
}

/// 冪等（R3.6・D9）: 同一 target の排他置換を 2 回目は Unchanged（結果集合同値・状態書き込みなし）。
#[test]
fn apply_bind_exclusive_idempotent_second_is_unchanged() {
    let mut states = ScopeStates::new(BindSet::from_ids([1301, 1303, 1207]));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let first = states.apply_bind_exclusive(&scope, &[1301, 1303, 1304], 1304);
    assert!(matches!(first, BindApplyOutcome::Changed(_)), "初回は集合変化ゆえ Changed");

    let second = states.apply_bind_exclusive(&scope, &[1301, 1303, 1304], 1304);
    assert_eq!(
        second,
        BindApplyOutcome::Unchanged,
        "同一 target の再排他置換は結果集合同値ゆえ Unchanged（冪等・R3.6・D9）"
    );
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1207, 1304])
    );
}

/// 非表示／未知 scope の排他置換 → 発行なし・状態のみ更新（D5：Hidden への強制 Show 禁止）。
#[test]
fn apply_bind_exclusive_hidden_or_unknown_is_state_only() {
    // Hidden scope。
    let mut states = ScopeStates::new(BindSet::from_ids([1301, 1303, 1207]));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply(&scope, SurfaceTarget::Hide);

    let outcome = states.apply_bind_exclusive(&scope, &[1301, 1303, 1304], 1304);
    assert_eq!(
        outcome,
        BindApplyOutcome::StateOnly,
        "非表示 scope の排他置換は発行せず状態のみ更新（D5）"
    );
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1207, 1304]),
        "Hidden でも排他置換は積算保持（次 Show へ持ち越し・D5）"
    );

    // 未知 scope。
    let mut states2 = ScopeStates::new(BindSet::from_ids([1301, 1303, 1207]));
    let unknown = ActorKey::from("1");
    let outcome2 = states2.apply_bind_exclusive(&unknown, &[1301, 1303, 1304], 1304);
    assert_eq!(outcome2, BindApplyOutcome::StateOnly, "未知 scope も StateOnly（D5）");
    assert_eq!(
        states2.scopes.get(&unknown),
        None,
        "apply_bind_exclusive はシェル状態を作らない（R3.8）"
    );
}

/// 非退行（R3.8）: 排他置換はシェル面・バルーン面の状態機械を一切変更しない。
#[test]
fn apply_bind_exclusive_does_not_touch_shell_or_balloon_state() {
    let mut states = ScopeStates::new(BindSet::from_ids([1301, 1303, 1207]));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply_balloon(&scope, SurfaceTarget::Show(2));

    states.apply_bind_exclusive(&scope, &[1301, 1303, 1304], 1304);
    assert_eq!(
        states.scopes.get(&scope),
        Some(&ScopeState::Shown(2100)),
        "排他置換はシェル面状態を変更しない（R3.8）"
    );
    assert_eq!(
        states.balloon.get(&scope),
        Some(&ScopeState::Shown(2)),
        "排他置換はバルーン面状態を変更しない（R3.8）"
    );
}

/// target が category_ids に含まれても最終的に有効になる（filter→chain の順序保証・bindopt 2.1/3.1）。
#[test]
fn apply_bind_exclusive_target_in_category_ids_stays_present() {
    let mut states = ScopeStates::new(BindSet::from_ids([1301]));
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    // target=1301 は category_ids に含まれる → 一旦除去され chain で再付与 → 有効維持だが
    // 単独になる（他の目パーツが無いので集合は {1301} のまま = 同値 → Unchanged）。
    let outcome = states.apply_bind_exclusive(&scope, &[1301, 1303], 1301);
    assert_eq!(
        outcome,
        BindApplyOutcome::Unchanged,
        "現集合が既に {{1301}} のみで target=1301 の排他置換は同値ゆえ Unchanged"
    );
    assert_eq!(states.current_binds(&scope), &BindSet::from_ids([1301]));
}

// ---- commit_pattern / current_pattern（per-(scope, slot) PatternState・要件 5.1/6.1/6.2） ----

use areka_emo_compose::{ComposeMethod, PatternFrame};

/// テスト用の非空 PatternState（単一コマ・Overlay）。
fn pat(anim_id: u32, surface_id: u32) -> PatternState {
    let mut p = PatternState::default();
    p.set(
        anim_id,
        PatternFrame {
            surface_id,
            method: ComposeMethod::Overlay,
            x: 0,
            y: 0,
        },
    );
    p
}

/// current_pattern 既定（要件 5.1）: 未格納の (scope, slot) は空 PatternState を返す（両 slot）。
#[test]
fn current_pattern_absent_returns_empty() {
    let states = empty_states();
    let scope = ActorKey::from("0");
    assert!(states.current_pattern(&scope, Slot::Shell).is_empty());
    assert!(states.current_pattern(&scope, Slot::Balloon).is_empty());
}

/// 冪等 commit（要件 6.2）: 同値の再 commit は Unchanged・書込副作用なし。
#[test]
fn commit_pattern_same_value_is_unchanged_no_write() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let p = pat(7, 1410);
    // 初回 nonempty commit は Changed（変化・表示中）。
    assert!(matches!(
        states.commit_pattern(&scope, Slot::Shell, p.clone()),
        PatternApplyOutcome::Changed(_)
    ));
    // 同値の再 commit は Unchanged（再発行なし・要件 6.2）。
    assert_eq!(
        states.commit_pattern(&scope, Slot::Shell, p.clone()),
        PatternApplyOutcome::Unchanged
    );
    assert_eq!(states.current_pattern(&scope, Slot::Shell), &p);
}

/// 空 commit の冪等（要件 6.2）: 未格納 (==空) へ空を commit は同値ゆえ Unchanged（書込なし）。
#[test]
fn commit_pattern_empty_on_shown_without_prior_is_unchanged() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    assert_eq!(
        states.commit_pattern(&scope, Slot::Shell, PatternState::default()),
        PatternApplyOutcome::Unchanged,
        "absent==空・new==空 は同値ゆえ Unchanged（発行なし）"
    );
}

/// 表示中シェル slot の pattern 変化（要件 6.1）: 現 surface id＋現 binds＋新 pattern の Show を発行。
#[test]
fn commit_pattern_different_on_shown_shell_emits_changed_show() {
    let mut states = empty_states(); // static binds {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let p = pat(7, 1410);
    let outcome = states.commit_pattern(&scope, Slot::Shell, p.clone());
    assert_eq!(
        outcome,
        PatternApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: binds_1100_1207(),
            pattern: p.clone(),
        }),
        "表示中シェル slot の pattern 変化は現 surface id＋現 binds＋新 pattern の Show を発行（6.1）"
    );
    assert_eq!(states.current_pattern(&scope, Slot::Shell), &p);
}

/// 表示中バルーン slot の pattern 変化（要件 6.1）: binds 非同梱の ShowBalloon を発行。
#[test]
fn commit_pattern_different_on_shown_balloon_emits_changed_showballoon() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply_balloon(&scope, SurfaceTarget::Show(2));

    let p = pat(3, 6);
    let outcome = states.commit_pattern(&scope, Slot::Balloon, p.clone());
    assert_eq!(
        outcome,
        PatternApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 2,
            pattern: p.clone(),
        }),
        "表示中バルーン slot の pattern 変化は binds 非同梱の ShowBalloon を発行（6.1・面種非依存同一経路）"
    );
    assert_eq!(states.current_pattern(&scope, Slot::Balloon), &p);
}

/// 非表示／未知 slot への commit は防御的非発行（要件 6.2）: 2.1 ゲートで実運用は非到達だが emit しない。
#[test]
fn commit_pattern_on_non_shown_slot_is_unchanged() {
    // Hidden shell slot。
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply(&scope, SurfaceTarget::Hide);
    assert_eq!(
        states.commit_pattern(&scope, Slot::Shell, pat(7, 1410)),
        PatternApplyOutcome::Unchanged,
        "非表示 slot への commit は発行しない（防御・6.2）"
    );

    // 未知 balloon slot。
    let unknown = ActorKey::from("1");
    assert_eq!(
        states.commit_pattern(&unknown, Slot::Balloon, pat(3, 6)),
        PatternApplyOutcome::Unchanged,
        "未知 slot への commit も発行しない（防御）"
    );
}

/// slot 独立（要件 5.1）: 同一 scope の Shell / Balloon pattern は別々に保持される。
#[test]
fn commit_pattern_slots_are_independent() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply_balloon(&scope, SurfaceTarget::Show(2));

    let ps = pat(7, 1410);
    let pb = pat(3, 6);
    states.commit_pattern(&scope, Slot::Shell, ps.clone());
    states.commit_pattern(&scope, Slot::Balloon, pb.clone());
    assert_eq!(states.current_pattern(&scope, Slot::Shell), &ps);
    assert_eq!(states.current_pattern(&scope, Slot::Balloon), &pb);
}

/// surface 切替で格納 pattern クリア（design ScopeStates 拡張）: 別 id への Changed 前に空へリセット。
#[test]
fn apply_surface_switch_clears_stored_pattern() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.commit_pattern(&scope, Slot::Shell, pat(7, 1410));
    assert!(!states.current_pattern(&scope, Slot::Shell).is_empty());

    // 別 surface へ切替 → 当該 slot の pattern を空へリセット（新面のコマは未生成＝空 pattern で発行）。
    let outcome = states.apply(&scope, SurfaceTarget::Show(2106));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2106,
            binds: binds_1100_1207(),
            pattern: PatternState::default(),
        })
    );
    assert!(
        states.current_pattern(&scope, Slot::Shell).is_empty(),
        "surface 切替で格納 pattern はクリアされる（次 commit の baseline が空）"
    );
}

/// バルーン面 surface 切替で格納 pattern クリア（design・シェル面と同一経路）。
#[test]
fn apply_balloon_surface_switch_clears_stored_pattern() {
    let mut states = empty_states();
    let scope = ActorKey::from("0");
    states.apply_balloon(&scope, SurfaceTarget::Show(2));
    states.commit_pattern(&scope, Slot::Balloon, pat(3, 6));
    assert!(!states.current_pattern(&scope, Slot::Balloon).is_empty());

    let outcome = states.apply_balloon(&scope, SurfaceTarget::Show(6));
    assert_eq!(
        outcome,
        ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
            scope: scope.clone(),
            surface_id: 6,
            pattern: PatternState::default(),
        })
    );
    assert!(
        states.current_pattern(&scope, Slot::Balloon).is_empty(),
        "バルーン面 surface 切替で格納 pattern はクリアされる"
    );
}

/// bind 変化の Show 再発行は current_pattern を同梱（design）: 現在コマを保ったまま再合成する。
#[test]
fn apply_bind_reemit_carries_current_pattern() {
    let mut states = empty_states(); // static {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    // シェル slot に現在コマを確定。
    let p = pat(7, 1410);
    states.commit_pattern(&scope, Slot::Shell, p.clone());

    // bind 変化の Show 再発行は空 default ではなく current_pattern を載せる。
    let outcome = states.apply_bind(&scope, 1302, true);
    assert_eq!(
        outcome,
        BindApplyOutcome::Changed(DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207, 1302]),
            pattern: p.clone(),
        }),
        "bind 変化の Show 再発行は current_pattern(Shell) を同梱（現在コマ保持で再合成）"
    );
}

/// `shown_slots` は `Shown` の (scope, slot, surface_id) のみ列挙する（`Hidden`／未知は除外・要件 2.1）。
#[test]
fn shown_slots_enumerates_only_shown_shell_and_balloon() {
    let mut states = empty_states();
    let s0 = ActorKey::from("0");
    let s1 = ActorKey::from("1");

    // scope0: シェル面 Shown(2100)・バルーン面 Shown(2)。
    states.apply(&s0, SurfaceTarget::Show(2100));
    states.apply_balloon(&s0, SurfaceTarget::Show(2));
    // scope1: シェル面を Show→Hide で Hidden にする（除外されるべき）。
    states.apply(&s1, SurfaceTarget::Show(3000));
    states.apply(&s1, SurfaceTarget::Hide);

    let mut shown = states.shown_slots();
    shown.sort_by(|a, b| a.0.cmp(&b.0).then(format!("{:?}", a.1).cmp(&format!("{:?}", b.1))));

    // scope0 の Shell/Balloon 2 件のみ（scope1 は Hidden ゆえ除外）。
    assert_eq!(shown.len(), 2, "Shown な slot のみ列挙（Hidden は除外）");
    assert!(shown.contains(&(s0.clone(), Slot::Shell, 2100)));
    assert!(shown.contains(&(s0.clone(), Slot::Balloon, 2)));
    assert!(
        !shown.iter().any(|(scope, _, _)| *scope == s1),
        "Hidden の scope1 は列挙されない"
    );
}
