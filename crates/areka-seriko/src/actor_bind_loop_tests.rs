//! `\![bind]` の合図を役者が捌く道の檻。
//!
//! 番号の出どころ: R・D・①〜⑤ は areka-P0-mayuna-compose（D8 は縮退の severity split）、「bindopt」は
//! areka-P0-bindoption-exclusivity、要件 14.2 は areka-P0-offline-surface-dump の要件。「ケース 15〜21」は
//! spec の番号ではなく、`actor_dispatch_tests.rs`（1〜14）から続く通し番号。

use super::test_support::*;
use super::*;
use crate::bind::tests::groups_of;
use crate::output::{DisplayCommand, MockSurfaceOutput};
use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::ActorKey;

// ─────────────────────────────────────────────────────────────────────
// bind 消費分岐（`cue_target_of == None` 枝内・Wait 判定前・D1）の網羅檻。
//
// `\![bind]` キャリアの名前自己選別（`name` が `bind` か `bind-noevent`）→ 引数解釈（parse_bind_directive）→
// scope 写像（scope_namespace）→ 名前解決（BindResolver）→ ポリシー導出（BindResolver::policy）
// → 適用（apply_bind_exclusive／apply_bind／mustselect 脱衣は無視・bindopt 2.1/3.1/3.2）→ 単一発行点
// （emit_display）の一本経路と、D8 severity split（①解決不能=error／②Toggle/CategoryWide は当てる（script-small-tags 3.8・3.9）／
// ③Malformed=error／④宛名規律 non-canonical: bind=warn・他人=debug（④ は `actor_bind_noevent_tests.rs`）／
// ⑤scope 写像なし（数でない scope）=warn）を
// 同期 `handle_message`＋`capture_logs_flow`（テストスレッド発火）で決定論的に檻化する。
// 全 severity 枝は正カウント assert（優しい縮退の非空虚化）。
// ─────────────────────────────────────────────────────────────────────

/// 複数可（Multiple）宣言カテゴリ「髪飾り」を持つ sakura 解決層（bindopt 3.3・actor 経路の
/// 加算検証用）。(髪飾り,花)→1600 / (髪飾り,リボン)→1601。static={1100,1207} と交わらない新 id。
///
/// 明示 `multiple` 宣言のカテゴリは既定（Default）と異なり、同一カテゴリ 2 パーツの共存を許す
/// ——これが Default×着衣（排他置換）との差を観測できる唯一の点（bindopt 3.3）。
fn hair_multiple_resolver() -> BindResolver {
    BindResolver::from_groups(&groups_of(
        "charset,UTF-8\r\n\
         sakura.bindgroup1600.name,髪飾り,花\r\n\
         sakura.bindgroup1601.name,髪飾り,リボン\r\n\
         sakura.bindoption0.group,髪飾り,multiple\r\n",
    ))
}

/// 既定（Default＝非宣言・高々 1 個・解除可）カテゴリ「まばたき」を持つ sakura 解決層
/// （bindopt 2.1/2.2・actor 経路の排他置換／除去検証用）。emo2 のまばたきカテゴリを模した
/// (まばたき,通常)→1400 / (まばたき,ジトー)→1402。static={1100,1207} と交わらない新 id。
fn blink_default_resolver() -> BindResolver {
    BindResolver::from_groups(&groups_of(
        "charset,UTF-8\r\n\
         sakura.bindgroup1400.name,まばたき,通常\r\n\
         sakura.bindgroup1402.name,まばたき,ジトー\r\n",
    ))
}

/// mustselect カテゴリ「目」を持つ sakura 解決層（bindopt 3.1・actor 経路の排他検証用）。
/// (目,笑)→1301 / (目,普)→1303 / (目,閉)→1304。static={1100,1207} と交わらない新 id。
fn eye_mustselect_resolver() -> BindResolver {
    BindResolver::from_groups(&groups_of(
        "charset,UTF-8\r\n\
         sakura.bindgroup1301.name,目,笑\r\n\
         sakura.bindgroup1303.name,目,普\r\n\
         sakura.bindgroup1304.name,目,閉\r\n\
         sakura.bindoption0.group,目,mustselect\r\n",
    ))
}

/// ケース15（6.3/3.5/7.1・D5・D8 正常経路）: 表示中 scope で解決可能な Apply は現 surface を
/// 新集合で再発行し、実機 grep マーカー `info!("seriko: bind 適用")` を発火する。
#[test]
fn bind_apply_on_shown_emits_show_and_info_marker() {
    let resolver = tiny_resolver();
    let bind_resolver = arm_bind_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    // シェル面を Shown(2100) に確定させる。
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "伸び", "1"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()), "適用後も処理継続（6.3）");
    // 単一発行点から現 surface(2100) を新集合 {1100,1207,1302} で再発行（D5・R3.5）。
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207, 1302]),
            pattern: PatternState::default(),
        }],
        "表示中 scope の解決可能 Apply は現 surface を新集合で再発行（R3.5・D5）"
    );
    // 実機 grep マーカー（R7.1）。
    assert!(
        flow.0.contains("level=INFO"),
        "bind 適用は info! マーカーを発火する（R7.1）: {}",
        flow.0
    );
    assert!(
        flow.0.contains("seriko: bind 適用"),
        "実機 grep マーカー文言を含む（R7.1）: {}",
        flow.0
    );
}

/// mustselect 排他（bindopt 3.1・actor 経路）: mustselect カテゴリ「目」で 2 つの異なる
/// パーツを続けて着衣（on）すると、2 度目の Show は同カテゴリ旧パーツ(1301) を外し新パーツ
/// (1304) のみを載せる（高々 1 パーツ有効・排他置換が actor を貫通して効くことを実証）。
#[test]
fn bind_mustselect_second_on_replaces_prior_part_in_category() {
    let resolver = tiny_resolver();
    let bind_resolver = eye_mustselect_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    // 1 度目: 目=笑（1301）を着衣 → {1100,1207,1301}。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["目", "笑", "1"])),
    );
    // 2 度目: 目=閉（1304）を着衣 → 排他置換で 1301 が外れ {1100,1207,1304}。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["目", "閉", "1"])),
    );

    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        &*recorded,
        &[
            DisplayCommand::Show {
                scope: scope.clone(),
                surface_id: 2100,
                binds: BindSet::from_ids([1100, 1207, 1301]),
                pattern: PatternState::default(),
            },
            DisplayCommand::Show {
                scope: scope.clone(),
                surface_id: 2100,
                binds: BindSet::from_ids([1100, 1207, 1304]),
                pattern: PatternState::default(),
            },
        ],
        "mustselect カテゴリの 2 度目着衣は旧パーツ(1301) を自動 off し新パーツ(1304) のみ有効（bindopt 3.1）"
    );
    // 動的集合も同カテゴリで高々 1 パーツ（1301 は残らない）。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1304]),
        "排他置換後は同カテゴリ内高々 1 パーツ有効（1301 は残らない・bindopt 3.1）"
    );
}

/// 最小再現の反転檻（bindopt 4.1/2.1）: `bindoption` 非宣言＝正典の既定（Default＝高々 1 個）の
/// カテゴリで 2 パーツを続けて着衣（on）すると、後勝ちの 1 個だけが残る。
///
/// emo2 のまばたきカテゴリ（1400 通常／1402 ジトー）を模した構成。非宣言カテゴリを加算で扱うと
/// {1400,1402} が共存する。
#[test]
fn bind_default_category_second_on_replaces_prior_part() {
    let resolver = tiny_resolver();
    // (まばたき,通常)→1400 / (まばたき,ジトー)→1402 の非宣言（既定）カテゴリ。
    let bind_resolver = blink_default_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();

    for tokens in [["まばたき", "通常", "1"], ["まばたき", "ジトー", "1"]] {
        let _ = handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &tokens)),
        );
    }

    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1402]),
        "既定（非宣言）カテゴリの 2 度目着衣は排他置換で後勝ち 1 個（1400 は残らない・bindopt 2.1/4.1）"
    );
}

/// mustselect 脱衣の無視檻（bindopt 3.2・bindopt D1）: mustselect カテゴリへの脱衣（off）指示は
/// bind 集合を一切変えず、固定文言の `warn!` を 1 回残して読み飛ばす（正典「解除不可」）。
#[test]
fn bind_mustselect_off_is_ignored_with_warn() {
    let resolver = tiny_resolver();
    let bind_resolver = eye_mustselect_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();

    // 前段: 目=笑（1301）を着衣 → {1100,1207,1301}。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["目", "笑", "1"])),
    );

    // 本題: 同じパーツへ脱衣指示 → 集合不変・warn! 1 回。
    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["目", "笑", "0"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()), "無視後も処理継続");
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1301]),
        "mustselect カテゴリの脱衣指示では bind 集合が変わらない（解除不可・bindopt 3.2）"
    );
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        1,
        "mustselect 脱衣の無視は warn! を 1 回残す（実機の既定ログ水準で可視・bindopt D1）: {}",
        flow.0
    );
    assert!(
        flow.0
            .contains("seriko: mustselect カテゴリの脱衣指示を無視（正典・解除不可・bindopt 3.2）"),
        "固定文言（grep マーカー）を含む（bindopt D1）: {}",
        flow.0
    );
}

// ─────────────────────────────────────────────────────────────────────
// bindopt 4.3: ポリシー×着脱の直積 6 セル（design.md §System Flows 直積表）の所在。
//
// | on    | policy     | 期待動作             | 檻                                                |
// |-------|------------|----------------------|---------------------------------------------------|
// | true  | MustSelect | 排他置換             | bind_mustselect_second_on_replaces_prior_part_in_category |
// | true  | Default    | 排他置換             | bind_default_category_second_on_replaces_prior_part（最小再現・bindopt 4.1） |
// | true  | Multiple   | 加算（2 パーツ共存） | bind_multiple_category_two_parts_coexist_via_actor |
// | false | MustSelect | 無視（集合不変+warn）| bind_mustselect_off_is_ignored_with_warn          |
// | false | Default    | 除去                 | bind_default_category_off_removes_part            |
// | false | Multiple   | 除去                 | bind_multiple_category_off_removes_only_that_part |
//
// 排他置換の経路でも守る決まり: 変更時のみ発行＋info マーカー（bindopt 2.4）＝
// bind_default_exclusive_replace_emits_show_and_info_marker／同値適用は非発行（bindopt 2.5）＝
// bind_default_same_part_re_on_is_unchanged_no_emit／非表示 scope は状態のみ更新（bindopt 2.6）＝
// bind_apply_on_hidden_scope_state_only_no_emit（状態の無い scope は
// state::bind_pattern_tests::apply_bind_exclusive_hidden_or_unknown_is_state_only）／
// 解決不能は読み飛ばし（bindopt 2.7）＝bind_unresolvable_errors_no_emit。
// ─────────────────────────────────────────────────────────────────────

/// 複数可×着衣（bindopt 3.3）: 明示 `multiple` 宣言カテゴリでは同一カテゴリの 2 パーツを
/// 続けて着衣すると**両方が共存**する（既定＝排他置換との差を検出する唯一の観測点）。
#[test]
fn bind_multiple_category_two_parts_coexist_via_actor() {
    let resolver = tiny_resolver();
    let bind_resolver = hair_multiple_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    for tokens in [["髪飾り", "花", "1"], ["髪飾り", "リボン", "1"]] {
        let _ = handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &tokens)),
        );
    }

    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1600, 1601]),
        "複数可カテゴリの着衣は加算＝同一カテゴリ 2 パーツが共存する（bindopt 3.3）"
    );
    assert_eq!(
        records.lock().expect("records mutex poisoned").len(),
        2,
        "2 回とも集合が変化＝表示中 scope で Changed 発行（bindopt 2.4）"
    );
}

/// 複数可×脱衣（bindopt 3.3）: 明示 `multiple` 宣言カテゴリの脱衣は当該パーツのみを
/// 除去し、同カテゴリの他パーツは残る。
#[test]
fn bind_multiple_category_off_removes_only_that_part() {
    let resolver = tiny_resolver();
    let bind_resolver = hair_multiple_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();

    // 前段: 2 パーツを共存させる → {1100,1207,1600,1601}。
    for tokens in [["髪飾り", "花", "1"], ["髪飾り", "リボン", "1"]] {
        let _ = handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &tokens)),
        );
    }
    // 本題: 花（1600）だけ脱衣 → リボン（1601）は残る。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["髪飾り", "花", "0"])),
    );

    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1601]),
        "複数可カテゴリの脱衣は当該パーツのみ除去し他パーツを残す（bindopt 3.3）"
    );
}

/// 既定×脱衣（bindopt 2.2）: 非宣言（既定）カテゴリの脱衣は当該パーツを外し、カテゴリ内
/// ゼロ個の状態を許す（正典の既定＝「選択解除可能」）。
#[test]
fn bind_default_category_off_removes_part() {
    let resolver = tiny_resolver();
    let bind_resolver = blink_default_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();

    // 前段: まばたき=通常（1400）を着衣 → {1100,1207,1400}。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["まばたき", "通常", "1"])),
    );
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1400]),
        "前提: 既定カテゴリの着衣で 1 パーツが載る"
    );

    // 本題: 同じパーツへ脱衣 → カテゴリ内ゼロ個（解除可）。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["まばたき", "通常", "0"])),
    );

    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207]),
        "既定（非宣言）カテゴリの脱衣は当該パーツを外しゼロ個を許す（解除可・bindopt 2.2）"
    );
}

/// 排他置換でも変わったときだけ出す（bindopt 2.4）: 既定カテゴリの**排他置換**でも、集合が変わったときは
/// 単一発行点から Show を再発行し、実機 grep マーカー `seriko: bind 適用`（info）を発火する。
///
/// `bind_apply_on_shown_emits_show_and_info_marker` は 1 カテゴリ 1 パーツ構成ゆえ「置換が
/// 実際に起きる」経路を通らない。本檻は旧パーツを外して新パーツを載せる置換そのものが
/// Changed 扱いで発行され info が出ることを固定する。
#[test]
fn bind_default_exclusive_replace_emits_show_and_info_marker() {
    let resolver = tiny_resolver();
    let bind_resolver = blink_default_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    // 前段: まばたき=通常（1400）を着衣。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["まばたき", "通常", "1"])),
    );

    // 本題: まばたき=ジトー（1402）を着衣＝排他置換（1400 を外し 1402 を載せる）。
    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["まばたき", "ジトー", "1"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()), "適用後も処理継続");
    let recorded = records.lock().expect("records mutex poisoned");
    assert_eq!(
        recorded.len(),
        2,
        "排他置換でも集合が変わったときは発行する（bindopt 2.4）: {recorded:?}"
    );
    assert_eq!(
        &recorded[1],
        &DisplayCommand::Show {
            scope: scope.clone(),
            surface_id: 2100,
            binds: BindSet::from_ids([1100, 1207, 1402]),
            pattern: PatternState::default(),
        },
        "排他置換後の集合で現 surface を単一発行点から再発行（bindopt 2.4）"
    );
    assert!(
        flow.0.contains("level=INFO"),
        "排他置換の Changed も info! 水準（bindopt 2.4）: {}",
        flow.0
    );
    assert!(
        flow.0.contains("seriko: bind 適用"),
        "実機 grep マーカー文言を排他置換経路でも保つ（bindopt 2.4）: {}",
        flow.0
    );
}

/// 排他置換でも同値は出さない（bindopt 2.5）: 排他置換経路でも同値適用（同一パーツの再着衣）は結果集合が
/// 変わらず Unchanged＝再発行しない（冪等）。
#[test]
fn bind_default_same_part_re_on_is_unchanged_no_emit() {
    let resolver = tiny_resolver();
    let bind_resolver = blink_default_resolver();
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    // 1 度目の着衣で Changed 発行（1 件）。
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["まばたき", "通常", "1"])),
    );
    assert_eq!(
        records.lock().expect("records mutex poisoned").len(),
        1,
        "前提: 1 度目の着衣は Changed で発行される"
    );

    // 本題: 同一パーツを再着衣 → 排他置換の結果集合が同値ゆえ Unchanged・非発行。
    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["まばたき", "通常", "1"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()), "同値適用でも処理継続");
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1400]),
        "同値適用は集合を変えない（冪等・bindopt 2.5）"
    );
    assert_eq!(
        records.lock().expect("records mutex poisoned").len(),
        1,
        "同値適用は再発行しない（変更時のみ発行・bindopt 2.5）"
    );
    assert_eq!(
        flow.0.matches("seriko: bind 適用").count(),
        0,
        "同値適用では info マーカーを発火しない（bindopt 2.5）: {}",
        flow.0
    );
}

/// 異カテゴリ加算（bindopt 3.4）: 異なるカテゴリ間の bind は共存する。
///
/// 各カテゴリが既定（Default＝高々 1 個）でも、排他置換が外すのは**同一カテゴリ**の ID のみ
/// ゆえ、腕と肩は互いに干渉せず両方が載る（3 値化で変わらない不変量＝回帰の錨）。
#[test]
fn bind_cross_category_accumulates_via_actor() {
    // (腕,伸び)→1302 と (肩,上げ)→1500 を持つ 2 カテゴリ表（いずれも bindoption 非宣言＝既定）。
    let resolver = tiny_resolver();
    let bind_resolver = BindResolver::from_groups(&groups_of(
        "charset,UTF-8\r\n\
         sakura.bindgroup1302.name,腕,伸び\r\n\
         sakura.bindgroup1500.name,肩,上げ\r\n",
    ));
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "伸び", "1"])),
    );
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["肩", "上げ", "1"])),
    );

    // 異なるカテゴリ同士は干渉しない（1302 も 1500 も残る）。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1302, 1500]),
        "異なるカテゴリ間の bind は共存する（両パーツ有効・bindopt 3.4）"
    );
    assert_eq!(
        records.lock().expect("records mutex poisoned").len(),
        2,
        "2 回とも表示中 scope で Changed 発行"
    );
}

/// ケース16（D5）: 解決可能だがシェル面が Hidden の scope では発行しない（StateOnly）。
#[test]
fn bind_apply_on_hidden_scope_state_only_no_emit() {
    let resolver = tiny_resolver();
    let bind_resolver = arm_bind_resolver();
    let mut states = fresh_states();
    let scope = ActorKey::from("0");
    // Show→Hide で Hidden にする。
    states.apply(&scope, SurfaceTarget::Show(2100));
    states.apply(&scope, SurfaceTarget::Hide);

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "伸び", "1"])),
    );

    assert_eq!(flow, ControlFlow::Continue(()), "StateOnly でも処理継続");
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "非表示 scope の bind 適用は発行しない（StateOnly・D5）"
    );
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1302]),
        "非表示 scope でも状態は直す（StateOnly）"
    );
}

/// ケース17（script-small-tags 3.9）: トグル形（数値欄省略）は今の着脱の反対を当てる。脱いでいる
/// 「伸び」を着せ、表示中 scope へ新集合の Show を 1 件出し、適用の info! を残す。
#[test]
fn bind_toggle_form_flips_current_part() {
    let resolver = tiny_resolver();
    let bind_resolver = arm_bind_resolver();
    let mut states = fresh_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "伸び"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert!(
        flow.0.contains("seriko: bind 適用") && flow.0.contains("level=INFO"),
        "トグル形は当てて info! を残す（3.9）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=WARN").count() + flow.0.matches("level=ERROR").count(),
        0,
        "トグル形は warn!／error! を残さない: {}",
        flow.0
    );
    assert_eq!(
        states.current_binds(&scope).ids(),
        &[1100, 1207, 1302],
        "脱いでいたパーツを着る"
    );
    assert_eq!(
        records.lock().expect("records mutex poisoned").len(),
        1,
        "トグル形は新集合で 1 件出す"
    );
}

/// ケース18（script-small-tags 3.8）: カテゴリ単位形（パーツ欄省略・数値 0）は、カテゴリのパーツを
/// 定義の順に全部脱がせ、変わった分を 1 件だけ出す。
#[test]
fn bind_category_wide_off_removes_all_parts() {
    let resolver = tiny_resolver();
    let groups = groups_of(
        "charset,UTF-8\r\nsakura.bindgroup1302.name,腕,伸び\r\nsakura.bindgroup1303.name,腕,曲げ\r\n",
    );
    let bind_resolver = BindResolver::from_groups(&groups);
    let mut states = fresh_states();
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();
    let _ = handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "伸び", "1"])),
    );
    records.lock().expect("records mutex poisoned").clear();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "", "0"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("seriko: bind 適用").count(),
        1,
        "着ていた 1 つを脱がせて info! を残す（3.8）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=WARN").count() + flow.0.matches("level=ERROR").count(),
        0,
        "カテゴリ単位形は warn!／error! を残さない: {}",
        flow.0
    );
    assert_eq!(states.current_binds(&scope).ids(), &[1100, 1207]);
    assert_eq!(
        records.lock().expect("records mutex poisoned").len(),
        1,
        "カテゴリ単位形は 1 件だけ出す"
    );
}

/// ケース19（D8③）: 破損入力（カテゴリ欠落・on/off 値破損）は error! を 1 回残し発行しない。
#[test]
fn bind_malformed_errors_no_emit() {
    // (a) カテゴリ欠落（トークン 0 個）、(b) on/off 値破損（"2"）。
    for tokens in [vec![], vec!["腕", "伸び", "2"]] {
        let resolver = tiny_resolver();
        let bind_resolver = arm_bind_resolver();
        let mut states = fresh_states();
        states.apply(&ActorKey::from("0"), SurfaceTarget::Show(2100));
        let mut out = MockSurfaceOutput::new();
        let mut loop_runtime = inert_runtime();
        let records = out.records();

        let flow = capture_logs_flow(|| {
            handle_message(
                &resolver,
                &bind_resolver,
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Cue(carrier_cue("0", "bind", &tokens)),
            )
        });

        assert_eq!(flow.1, ControlFlow::Continue(()));
        assert_eq!(
            flow.0.matches("level=ERROR").count(),
            1,
            "破損入力 {tokens:?} は error! を 1 回残す（D8③）: {}",
            flow.0
        );
        assert_eq!(
            flow.0.matches("level=WARN").count(),
            0,
            "破損入力 {tokens:?} は warn! を残さない（D8③）: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "破損入力 {tokens:?} は発行しない"
        );
    }
}

/// ケース20（D7・D8⑤）: scope 写像なし（数でない "x"）の Apply は warn! を 1 回残し発行しない。
/// scope "2" 以降は char{N} の名前表を引く（要件 14.2・`scope_namespace` の試験）ので写像なしにならない。
#[test]
fn bind_scope_unmapped_warns_no_emit() {
    let resolver = tiny_resolver();
    let bind_resolver = arm_bind_resolver();
    let mut states = fresh_states();
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("x", "bind", &["腕", "伸び", "1"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        1,
        "scope 写像なし（\"x\"）は warn! を 1 回残す（D7・D8⑤）: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "写像なし scope は発行しない"
    );
}

/// ケース21（D8①・R3.7）: 解決不能（resolver 空）の Apply は error! を 1 回残し発行しない。
#[test]
fn bind_unresolvable_errors_no_emit() {
    let resolver = tiny_resolver();
    let bind_resolver = BindResolver::empty(); // (腕,伸び) を解決できない
    let mut states = fresh_states();
    states.apply(&ActorKey::from("0"), SurfaceTarget::Show(2100));
    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    let flow = capture_logs_flow(|| {
        handle_message(
            &resolver,
            &bind_resolver,
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(carrier_cue("0", "bind", &["腕", "伸び", "1"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("level=ERROR").count(),
        1,
        "解決不能は error! を 1 回残す（R3.7・D8①）: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "解決不能では発行しない（状態不変・R3.7）"
    );
}
