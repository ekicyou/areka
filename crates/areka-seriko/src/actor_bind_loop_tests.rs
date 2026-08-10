use super::test_support::*;
use super::*;
use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::{ActorKey, CueCommand, TalkCue};
use crate::output::{DisplayCommand, MockSurfaceOutput};
use std::collections::{BTreeMap, BTreeSet};

// ─────────────────────────────────────────────────────────────────────
// Task 6.1/6.3: bind 消費分岐（`cue_target_of == None` 枝内・Wait 判定前・D1）の網羅檻。
//
// `\![bind]` キャリアの名前自己選別（name=="bind"）→ 引数解釈（parse_bind_directive）→
// scope 写像（scope_namespace）→ 名前解決（BindResolver）→ 適用（apply_bind）→ 単一発行点
// （emit_display）の一本経路と、D8 severity split（①解決不能=error／②Toggle/CategoryWide=warn／
// ③Malformed=error／④宛名規律 non-canonical: bind=warn・他人=debug／⑤scope 写像なし=warn）を
// 同期 `handle_message`＋`capture_logs_flow`（テストスレッド発火）で決定論的に檻化する。
// 全 severity 枝は正カウント assert（優しい縮退の非空虚化・deterministic-test-coverage mandate）。
// ─────────────────────────────────────────────────────────────────────

/// `\![bind,tokens...]` 正準キャリア cue（`Custom` の String Array）を組む。
fn bind_carrier_cue(scope: &str, tokens: &[&str]) -> TalkCue {
    let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::command_carrier("bind", toks),
        duration: 0.0,
    }
}

/// 任意コマンド名の正準キャリア cue（名前ゲートの担当外検証用）。
fn named_carrier_cue(scope: &str, name: &str, tokens: &[&str]) -> TalkCue {
    let toks: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::command_carrier(name, toks),
        duration: 0.0,
    }
}

/// 非正準 params の `Custom` cue（`params` が非 Array＝`as_command_carrier()==None`・宛名規律検証用）。
fn noncanonical_custom_cue(scope: &str, command: &str) -> TalkCue {
    let cue = TalkCue {
        at: 0.0,
        actor: ActorKey::from(scope),
        command: CueCommand::Custom {
            command: command.to_string(),
            params: dola::DynamicValue::Null,
        },
        duration: 0.0,
    };
    // 前提: この構成は必ずキャリア開封に失敗する（宛名規律 D8④ の入口条件）。
    assert!(
        cue.command.as_command_carrier().is_none(),
        "非 Array params の Custom は as_command_carrier() が None（宛名規律の入口）"
    );
    cue
}

/// (腕,伸び)→1302 の sakura 名前表を持つ解決層（static={1100,1207} と交わらない新 id・mustselect なし）。
fn arm_bind_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("腕".to_string(), "伸び".to_string()), 1302);
    BindResolver::new(sakura, BTreeMap::new(), BTreeSet::new(), BTreeSet::new())
}

/// mustselect カテゴリ「目」を持つ sakura 解決層（D11・R4.5・actor 経路の排他検証用）。
/// (目,笑)→1301 / (目,普)→1303 / (目,閉)→1304。static={1100,1207} と交わらない新 id。
fn eye_mustselect_resolver() -> BindResolver {
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("目".to_string(), "笑".to_string()), 1301);
    sakura.insert(("目".to_string(), "普".to_string()), 1303);
    sakura.insert(("目".to_string(), "閉".to_string()), 1304);
    let mut sakura_ms: BTreeSet<String> = BTreeSet::new();
    sakura_ms.insert("目".to_string());
    BindResolver::new(sakura, BTreeMap::new(), sakura_ms, BTreeSet::new())
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
            SerikoMsg::Cue(bind_carrier_cue("0", &["腕", "伸び", "1"])),
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

/// mustselect 排他（R4.5・D11・actor 経路）: mustselect カテゴリ「目」で 2 つの異なる
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
    handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(bind_carrier_cue("0", &["目", "笑", "1"])),
    );
    // 2 度目: 目=閉（1304）を着衣 → 排他置換で 1301 が外れ {1100,1207,1304}。
    handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(bind_carrier_cue("0", &["目", "閉", "1"])),
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
        "mustselect カテゴリの 2 度目着衣は旧パーツ(1301) を自動 off し新パーツ(1304) のみ有効（R4.5・D11）"
    );
    // 動的集合も同カテゴリで高々 1 パーツ（1301 は残らない）。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1304]),
        "排他置換後は同カテゴリ内高々 1 パーツ有効（1301 は残らない・R4.5）"
    );
}

/// 非 mustselect カテゴリは従来どおり加算される（R4.5 の対比・actor 経路）。
/// 非排他の resolver（arm_bind_resolver・mustselect なし）で 2 つ着衣すると両方が積算される。
#[test]
fn bind_non_mustselect_accumulates_via_actor() {
    // (腕,伸び)→1302 と (肩,上げ)→1500 を持つ非 mustselect 表。
    let resolver = tiny_resolver();
    let mut sakura: BTreeMap<(String, String), u32> = BTreeMap::new();
    sakura.insert(("腕".to_string(), "伸び".to_string()), 1302);
    sakura.insert(("肩".to_string(), "上げ".to_string()), 1500);
    let bind_resolver =
        BindResolver::new(sakura, BTreeMap::new(), BTreeSet::new(), BTreeSet::new());
    let mut states = fresh_states(); // static = {1100, 1207}
    let scope = ActorKey::from("0");
    states.apply(&scope, SurfaceTarget::Show(2100));

    let mut out = MockSurfaceOutput::new();
    let mut loop_runtime = inert_runtime();
    let records = out.records();

    handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(bind_carrier_cue("0", &["腕", "伸び", "1"])),
    );
    handle_message(
        &resolver,
        &bind_resolver,
        &mut states,
        &mut loop_runtime,
        &mut out,
        SerikoMsg::Cue(bind_carrier_cue("0", &["肩", "上げ", "1"])),
    );

    // 非 mustselect ゆえ両方積算（1302 も 1500 も残る）。
    assert_eq!(
        states.current_binds(&scope),
        &BindSet::from_ids([1100, 1207, 1302, 1500]),
        "非 mustselect カテゴリは従来どおり加算（両パーツ有効・R4.5 対比）"
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
        SerikoMsg::Cue(bind_carrier_cue("0", &["腕", "伸び", "1"])),
    );

    assert_eq!(flow, ControlFlow::Continue(()), "StateOnly でも処理継続");
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "非表示 scope の bind 適用は発行しない（StateOnly・D5）"
    );
}

/// ケース17（D8②）: トグル形（数値欄省略）は warn! を 1 回残し発行しない。
#[test]
fn bind_toggle_form_warns_no_emit() {
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
            SerikoMsg::Cue(bind_carrier_cue("0", &["腕", "伸び"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        1,
        "トグル形は warn! を 1 回残す（M1 縮退・R4.2・D8②）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=ERROR").count(),
        0,
        "トグル形は error! を残さない（D8②）: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "トグル形は発行しない"
    );
}

/// ケース18（D8②）: カテゴリ単位形（パーツ欄省略）は warn! を 1 回残し発行しない。
#[test]
fn bind_category_wide_form_warns_no_emit() {
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
            SerikoMsg::Cue(bind_carrier_cue("0", &["腕"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        1,
        "カテゴリ単位形は warn! を 1 回残す（M1 縮退・R4.2・D8②）: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "カテゴリ単位形は発行しない"
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
                SerikoMsg::Cue(bind_carrier_cue("0", &tokens)),
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

/// ケース20（D7・D8⑤）: scope 写像なし（"2"）の Apply は warn! を 1 回残し発行しない。
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
            SerikoMsg::Cue(bind_carrier_cue("2", &["腕", "伸び", "1"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        1,
        "scope 写像なし（\"2\"）は warn! を 1 回残す（M-dual 拡張シーム・D7・D8⑤）: {}",
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
            SerikoMsg::Cue(bind_carrier_cue("0", &["腕", "伸び", "1"])),
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

/// ケース22（D1・R2.5）: 名前ゲート——正準キャリアだが name!="bind"（例 "move"）は
/// 良性 debug! で読み飛ばし、発行なし・WARN/ERROR なし（名前自己選別）。
#[test]
fn bind_name_gate_other_name_is_benign_debug_no_emit() {
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
            SerikoMsg::Cue(named_carrier_cue("0", "move", &["-353", "", "", "0"])),
        )
    });

    assert_eq!(flow.1, ControlFlow::Continue(()));
    assert_eq!(
        flow.0.matches("level=WARN").count(),
        0,
        "担当外コマンド名は warn! を出さない（名前自己選別・R2.5）: {}",
        flow.0
    );
    assert_eq!(
        flow.0.matches("level=ERROR").count(),
        0,
        "担当外コマンド名は error! を出さない: {}",
        flow.0
    );
    assert!(
        flow.0.contains("level=DEBUG"),
        "担当外コマンド名は良性 debug! として観測できる（R2.5）: {}",
        flow.0
    );
    assert!(
        records.lock().expect("records mutex poisoned").is_empty(),
        "担当外コマンド名では発行しない"
    );
}

/// ケース23（D8④・宛名規律）: 非正準 params の Custom で宛名が自分（"bind"）＝warn! ；
/// 宛名が他人（"noexist"）＝warn! を出さず debug! 素通し。いずれも発行なし。
#[test]
fn bind_noncanonical_addressee_severity_split() {
    // (a) 宛名 "bind"（自分宛の壊れ物）→ warn!。
    {
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
                SerikoMsg::Cue(noncanonical_custom_cue("0", "bind")),
            )
        });

        assert_eq!(flow.1, ControlFlow::Continue(()));
        assert_eq!(
            flow.0.matches("level=WARN").count(),
            1,
            "宛名 bind の非正準 params は warn! を 1 回残す（自分宛の壊れ物・D8④）: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "非正準 params では発行しない"
        );
    }
    // (b) 宛名 "noexist"（他人宛/未知名）→ warn! を出さず debug! 素通し。
    {
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
                SerikoMsg::Cue(noncanonical_custom_cue("0", "noexist")),
            )
        });

        assert_eq!(flow.1, ControlFlow::Continue(()));
        assert_eq!(
            flow.0.matches("level=WARN").count(),
            0,
            "他人宛/未知名の非正準 params は warn! を出さない（報告責任は宛名の担当者・D8④）: {}",
            flow.0
        );
        assert!(
            flow.0.contains("level=DEBUG"),
            "他人宛/未知名は良性 debug! として観測できる（D8④）: {}",
            flow.0
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "非正準 params では発行しない"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// Task 6.1: Tick 経路（SerikoMsg::Tick・send_tick・handle_message Tick 腕・on_surface_changed
// 連動）の網羅檻（R1.1/6.1/6.3/7.1/7.5/8.3）。
//
// 檻の要点:
// - (A) 表示中 slot が 1 つもない Tick は完全 no-op（無発行）——on_tick が空を返す 2.1 表示中
//       ゲートの自然帰結。
// - (B) live な loop_config（実表＋注入 rng）＋表示中 slot で境界跨ぎ Tick が pattern を載せた
//       Show を**既存 emit_display 単一発行点**から発行する（handle_message→on_tick→emit_display
//       の end-to-end 配線・5.3 の直接 on_tick 檻とは別の配線証明）。
// - (C) アクター停止後の send_tick は debug!（shutdown 期待事象）で panic しない（7.5/6.3）。
// - (D) Emote の Changed で on_surface_changed が呼ばれ、切替後の Tick が旧アニメの残再生を
//       復活させない（playback リセット・R2.3 表示従属）。
//
// すべて Tick 直投函＋注入 rng で決定論（sleep 不使用・7.1）。表構築は looper.rs 檻と同じ
// from_world 実構築パスを通す。
// ─────────────────────────────────────────────────────────────────────
mod tick_loop_tests {
    use super::*;
    use areka_emo_compose::EmoWorld;
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
    };
    use crate::table::AnimationTable;
    use crate::timeline::LoopRng;

    /// コマ 1 本（overlay 固定・x/y=0）。
    fn pat(index: u32, surface_id: i64, wait: u32) -> Pattern {
        Pattern {
            index,
            method: DrawMethod::new("overlay".to_string()),
            surface_id,
            wait,
            x: 0,
            y: 0,
        }
    }

    fn surface_with(id: u32, animations: Vec<Animation>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements: Vec::new(),
            collisions: Vec::new(),
            animations,
        }
    }

    fn shell_table_of(surfaces: Vec<Surface>) -> AnimationTable {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        let shell = Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        };
        AnimationTable::from_world(&EmoWorld::build(&shell))
    }

    /// 単一 anim（id/interval/frames）を持つ surface から shell 表を build する。
    fn table_single(
        surface_id: u32,
        anim_id: u32,
        interval: Interval,
        frames: &[(i64, u32)],
    ) -> AnimationTable {
        let patterns = frames
            .iter()
            .enumerate()
            .map(|(i, (sid, wait))| pat(i as u32, *sid, *wait))
            .collect();
        shell_table_of(vec![surface_with(
            surface_id,
            vec![Animation {
                id: anim_id,
                interval,
                patterns,
            }],
        )])
    }

    /// 実表＋注入 rng の live config（バルーン表の写像は空＝全 scope 不活性）。
    fn live_cfg(shell_table: AnimationTable, rng: LoopRng) -> SerikoLoopConfig {
        SerikoLoopConfig {
            shell_table,
            balloon_tables: BTreeMap::new(),
            rng,
        }
    }

    /// 常に発火する rng（`should_fire` は `rng(k)==0` で発火）。
    fn always_fire() -> LoopRng {
        Box::new(|_bound: u32| 0)
    }

    /// 1 度だけ発火し以後発火しない rng（境界 1 回だけ抽選を通す）。
    fn fire_once() -> LoopRng {
        let mut calls: u32 = 0;
        Box::new(move |_bound: u32| {
            calls += 1;
            if calls == 1 {
                0
            } else {
                1
            }
        })
    }

    /// 表示中シェル面 surface `sid`・静的 binds{1100,1207} の scope "0" 状態を組む。
    fn shown_shell_states(sid: u32) -> (ScopeStates, ActorKey) {
        let mut states = ScopeStates::new(BindSet::from_ids([1100, 1207]));
        let scope = ActorKey::from("0");
        states.apply(&scope, SurfaceTarget::Show(sid));
        (states, scope)
    }

    /// (A・R6.1/2.1) 表示中 slot が 1 つもない Tick は完全 no-op（無発行）。
    ///
    /// live な表（常時発火 rng）でも Show 前は表示中 slot ゼロ＝on_tick は評価対象なし＝空を返す。
    /// handle_message Tick 腕は空列を回すだけで emit_display を一度も呼ばない（単一発行点の
    /// 発火ゼロ＝MockSurfaceOutput 記録ゼロ）。
    #[test]
    fn tick_with_no_shown_slot_is_complete_no_op() {
        let resolver = tiny_resolver();
        // 表は live（surface 10 に常時発火アニメ）だが、Show していない＝表示中 slot 皆無。
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
        let mut loop_runtime = LoopRuntime::new(live_cfg(table, always_fire()));
        let mut states = ScopeStates::new(BindSet::from_ids([1100, 1207])); // Show なし
        let mut out = MockSurfaceOutput::new();
        let records = out.records();

        // 起動 tick と境界跨ぎ tick の双方が完全 no-op（表示中 slot が無いため）。
        for now in [0_u64, 1000, 5000] {
            let flow = handle_message(
                &resolver,
                &BindResolver::empty(),
                &mut states,
                &mut loop_runtime,
                &mut out,
                SerikoMsg::Tick { now_ms: now },
            );
            assert_eq!(flow, ControlFlow::Continue(()), "Tick は常に処理継続");
        }
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "表示中 slot が 1 つもない Tick は完全 no-op（emit_display 単一発行点の発火ゼロ・R6.1/2.1）"
        );
    }

    /// (B・R1.1/6.3/7.1) 表示中 slot＋live config で境界跨ぎ Tick が pattern を載せた Show を
    /// 既存 emit_display 単一発行点から発行する（handle_message→on_tick→emit_display の end-to-end）。
    ///
    /// 5.3 の直接 on_tick 檻と異なり、**Tick メッセージが handle_message を貫通**して発行に至る
    /// 配線を固定する。注入 rng（always_fire）＋注入 Tick 列で決定論（sleep 不使用）。
    #[test]
    fn tick_boundary_cross_emits_show_carrying_pattern() {
        let resolver = tiny_resolver();
        let table = table_single(10, 0, Interval::Random { k: 4 }, &[(2106, 0)]);
        let mut loop_runtime = LoopRuntime::new(live_cfg(table, always_fire()));
        let (mut states, scope) = shown_shell_states(10);
        let mut out = MockSurfaceOutput::new();
        let records = out.records();

        // 起動 tick（遅延初期化・非跨ぎ）→ 無発行。
        let _ = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Tick { now_ms: 0 },
        );
        assert!(
            records.lock().expect("records mutex poisoned").is_empty(),
            "起動 tick は境界を跨がず無発行"
        );

        // 境界跨ぎ tick（1000）→ 抽選発火＋先頭コマで Show を発行。
        let flow = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Tick { now_ms: 1000 },
        );
        assert_eq!(flow, ControlFlow::Continue(()), "Tick は処理継続");

        let recorded = records.lock().expect("records mutex poisoned");
        assert_eq!(recorded.len(), 1, "境界跨ぎ Tick で Show ちょうど 1 件");
        match &recorded[0] {
            DisplayCommand::Show {
                scope: s,
                surface_id,
                pattern,
                ..
            } => {
                assert_eq!(s, &scope, "表示中 scope の Show");
                assert_eq!(*surface_id, 10, "表示中 surface は 10");
                let f = pattern.get(0).expect("anim0 の現在コマが pattern に載る");
                assert_eq!(
                    f.surface_id, 2106,
                    "先頭コマ 2106 が Tick 経由で単一発行点から発行される（配線証明・R6.3）"
                );
            }
            other => panic!("Show を期待（Tick→on_tick→emit_display の end-to-end）: {other:?}"),
        }
    }

    /// (C・R7.5/6.3) アクター停止後の send_tick は debug! を残し panic しない（shutdown 期待事象）。
    ///
    /// spawn_seriko→close→join でアクター完全停止（inbox 受信端消失）→その後テストスレッドで
    /// send_tick を capture_logs 直下で呼ぶ。send 失敗が debug!（error! でない）として観測でき、
    /// send_tick が panic せず戻る（本行到達＝処理系が異常終了しない）ことを固定する。
    #[test]
    fn send_tick_after_actor_stopped_logs_debug_no_panic() {
        let out = MockSurfaceOutput::new();
        let (sink, handle) = spawn_seriko(
            tiny_resolver(),
            BindSet::from_ids([1100, 1207]),
            BindResolver::empty(),
            SerikoLoopConfig::disabled(),
            out,
        );
        sink.close().expect("Close を送れること");
        handle.join().expect("Close で正常終了する");

        let logs = capture_logs(|| {
            sink.send_tick(1234); // 停止後の tick 送出。
        });
        assert!(
            logs.contains("level=DEBUG"),
            "停止後の send_tick が送出失敗を debug! で残すこと（shutdown 期待事象・R7.5）: {logs}"
        );
        assert_eq!(
            logs.matches("level=ERROR").count(),
            0,
            "停止後の send_tick は error! を出さない（PresentBridge 先例・shutdown 期待事象）: {logs}"
        );
        assert!(
            logs.contains("target=areka_seriko"),
            "本クレート target で発火すること: {logs}"
        );
        // panic せず本行へ到達したこと自体が「処理系が異常終了しない」証跡（R6.3）。
    }

    /// (D・R2.3・8.3) Emote の Changed で on_surface_changed が呼ばれ、面切替後の Tick が旧アニメの
    /// 残再生を復活させない（playback リセット）。
    ///
    /// 同一 anim id 0 を surface10（コマ 500/501）と surface20（コマ 700/701）が持つ表で、
    /// surface10 で再生開始（started_at=1000）→ Emote で surface20 へ切替 → 境界を跨がない後続 Tick。
    /// on_surface_changed が playback を除去するため、後続 Tick は「継続再生」を surface20 上へ
    /// 復活させず無発行。**リセットが無ければ** elapsed 継続で surface20 のコマ 700 が発行されてしまう
    /// （本テストはその差分を檻に入れる）。
    #[test]
    fn emote_surface_change_resets_loop_playback() {
        let resolver = tiny_resolver(); // 数値 key は直接解決（"20"→Show(20)）。
        let table = shell_table_of(vec![
            surface_with(
                10,
                vec![Animation {
                    id: 0,
                    interval: Interval::Random { k: 4 },
                    patterns: vec![pat(0, 500, 0), pat(1, 501, 100)],
                }],
            ),
            surface_with(
                20,
                vec![Animation {
                    id: 0,
                    interval: Interval::Random { k: 4 },
                    patterns: vec![pat(0, 700, 0), pat(1, 701, 100)],
                }],
            ),
        ]);
        // 境界 1 回だけ発火（切替後の非跨ぎ Tick では再抽選しない）。
        let mut loop_runtime = LoopRuntime::new(live_cfg(table, fire_once()));
        let (mut states, _scope) = shown_shell_states(10);
        let mut out = MockSurfaceOutput::new();
        let records = out.records();

        // 起動 tick → 境界跨ぎ tick（1000）で surface10 の anim0 が発火・elapsed0→コマ 500。
        let _ = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Tick { now_ms: 0 },
        );
        let _ = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Tick { now_ms: 1000 },
        );

        // Emote で surface20 へ切替（apply Changed → emit Show{20,空} → on_surface_changed で playback 除去）。
        let _ = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Cue(emote_cue(1.0, "0", "20")),
        );

        // 境界を跨がない後続 tick（1050・次境界は 2000）。playback がリセット済みなら無発行。
        let _ = handle_message(
            &resolver,
            &BindResolver::empty(),
            &mut states,
            &mut loop_runtime,
            &mut out,
            SerikoMsg::Tick { now_ms: 1050 },
        );

        let recorded = records.lock().expect("records mutex poisoned");
        // 発行列: [Show{10,コマ500}, Show{20,空}]。切替後 Tick は無発行（旧再生を復活させない）。
        assert_eq!(
            recorded.len(),
            2,
            "切替後の非跨ぎ Tick は旧アニメの残再生を復活させない（playback リセット・R2.3）: {recorded:?}"
        );
        match &recorded[0] {
            DisplayCommand::Show { surface_id, pattern, .. } => {
                assert_eq!(*surface_id, 10, "1 件目は surface10 の発火");
                assert_eq!(pattern.get(0).expect("コマ").surface_id, 500, "surface10 の先頭コマ 500");
            }
            other => panic!("Show を期待: {other:?}"),
        }
        match &recorded[1] {
            DisplayCommand::Show { surface_id, pattern, .. } => {
                assert_eq!(*surface_id, 20, "2 件目は surface20 への切替");
                assert!(
                    pattern.get(0).is_none(),
                    "面切替の Show は空 pattern（旧コマも新残再生も載らない・R2.3/8.3）"
                );
            }
            other => panic!("Show を期待: {other:?}"),
        }
    }
}
