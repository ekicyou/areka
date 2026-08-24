use super::*;

fn config() -> KanadeConfig {
    KanadeConfig::new("master", "1.0.0")
}

/// GET variant を分解して (id の wire 形, references) を取り出す（Notify なら panic）。
fn expect_get(call: ShioriCall) -> (String, Vec<String>) {
    match call {
        ShioriCall::Get { id, references, .. } => (id.as_str().to_string(), references),
        ShioriCall::Notify { .. } => panic!("expected GET, got NOTIFY"),
    }
}

/// NOTIFY variant を分解して (id の wire 形, references) を取り出す（Get なら panic）。
fn expect_notify(call: ShioriCall) -> (String, Vec<String>) {
    match call {
        ShioriCall::Notify { id, references, .. } => (id.as_str().to_string(), references),
        ShioriCall::Get { .. } => panic!("expected NOTIFY, got GET"),
    }
}

/// 呼出の `id`（出所カテゴリ込み）を GET/NOTIFY 不問で取り出す（許可集合檻の被覆確認用）。
fn event_id(call: &ShioriCall) -> &EventId {
    match call {
        ShioriCall::Get { id, .. } | ShioriCall::Notify { id, .. } => id,
    }
}

/// 呼出の `status` を render した wire 値（`None` ⇔ ヘッダ行なし）を取り出す。
fn call_status(call: &ShioriCall) -> Option<String> {
    match call {
        ShioriCall::Get { status, .. } | ShioriCall::Notify { status, .. } => status.render(),
    }
}

#[test]
fn on_initialize_is_notify_with_empty_references() {
    let (id, references) = expect_notify(on_initialize(&ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnInitialize");
    assert!(references.is_empty());
}

/// OnFirstBoot Ref0 は vanish 引数由来（Req 4.1／4.2）: `0` で従来の固定値 `"0"` と同値・
/// 非ゼロ（`7`）はそのまま Reference0 に載る（値源が呼び手へ移ったことの檻）。
#[test]
fn on_first_boot_ref0_is_vanish_count_argument() {
    // vanish_count=0 → 従来値 "0" と同値（既存全サイトはこの経路で挙動不変）。
    let (id, references) = expect_get(on_first_boot(&ExecutionSnapshot::INACTIVE, 0));
    assert_eq!(id, "OnFirstBoot");
    assert_eq!(references, vec!["0".to_string()]);

    // vanish_count=7 → Reference0 は "7"（Ref0 の値源が呼び手引数であることを固定）。
    let (id, references) = expect_get(on_first_boot(&ExecutionSnapshot::INACTIVE, 7));
    assert_eq!(id, "OnFirstBoot");
    assert_eq!(references, vec!["7".to_string()]);
}

#[test]
fn on_boot_is_get_with_shell_name_ref0() {
    let (id, references) = expect_get(on_boot(&config(), &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnBoot");
    assert_eq!(references, vec!["master".to_string()]);
}

#[test]
fn baseware_version_is_notify_with_version_and_name() {
    let (id, references) = expect_notify(baseware_version(&config(), &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "basewareversion");
    assert_eq!(references, vec!["1.0.0".to_string(), "areka".to_string()]);
}

#[test]
fn on_second_change_playable_is_get_ref3_one() {
    // 7_200_000 ms = 2 hours。talk_active=false（再生可能）→ GET・Ref3=1・status 空（DD-IT-3）。
    let call = on_second_change(
        MonotonicMs(7_200_000),
        &ExecutionSnapshot {
            talk_active: false,
            choice_active: false,
        },
    );
    assert_eq!(
        call_status(&call),
        None,
        "再生可能時（talk 非アクティブ）は Status ヘッダを出さない（DD-IT-3/DD-IT-5）"
    );
    let (id, references) = expect_get(call);
    assert_eq!(id, "OnSecondChange");
    assert_eq!(
        references,
        vec![
            "2".to_string(),
            "0".to_string(),
            "0".to_string(),
            "1".to_string(),
        ]
    );
}

#[test]
fn on_second_change_not_playable_is_notify_ref3_zero() {
    // 3_600_000 ms = 1 hour。talk_active=true（再生中）→ NOTIFY・Ref3=0・status talking（DD-IT-3）。
    let call = on_second_change(
        MonotonicMs(3_600_000),
        &ExecutionSnapshot {
            talk_active: true,
            choice_active: false,
        },
    );
    assert_eq!(
        call_status(&call),
        Some("talking".to_string()),
        "再生中は Ref3=0 と Status: talking が同一スナップショットから出る（DD-IT-3）"
    );
    let (id, references) = expect_notify(call);
    assert_eq!(id, "OnSecondChange");
    assert_eq!(
        references,
        vec![
            "1".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
        ]
    );
}

#[test]
fn on_second_change_ref0_truncates_toward_zero() {
    // 端数（1 時間未満）は切り捨てて "0"（3_599_999 ms < 1 hour）。
    let (_, references) = expect_get(on_second_change(
        MonotonicMs(3_599_999),
        &ExecutionSnapshot {
            talk_active: false,
            choice_active: false,
        },
    ));
    assert_eq!(references[0], "0");
}

#[test]
fn on_close_user_maps_to_user() {
    let (id, references) = expect_get(on_close(CloseReason::User, &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnClose");
    assert_eq!(references, vec!["user".to_string()]);
}

#[test]
fn on_close_system_maps_to_system() {
    let (id, references) = expect_get(on_close(CloseReason::System, &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnClose");
    assert_eq!(references, vec!["system".to_string()]);
}

/// `on_close_notify`（DD-IT-8）: NOTIFY・Ref0=reason・status は snapshot 由来（INACTIVE→None）。
#[test]
fn on_close_notify_is_notify_with_reason_and_derived_status() {
    let call = on_close_notify(CloseReason::System, &ExecutionSnapshot::INACTIVE);
    assert_eq!(
        call_status(&call),
        None,
        "INACTIVE スナップショット（Unloading 遷移後・DD-IT-4）は Status ヘッダを出さない"
    );
    let (id, references) = expect_notify(call);
    assert_eq!(id, "OnClose");
    assert_eq!(references, vec!["system".to_string()]);
}

/// 許可 ID 檻（Req3.1/3.2/7.1・DD-IT-8・DD-IE-11・DD-2）: 表が期待11集合と完全一致し
/// `OnTalk`/`OnHour` を含まない。マウス系2種（OnMouseMove/OnMouseDoubleClick）は
/// Task 2.1 で additive 追加され、選択関連の固定 3 ID（OnChoiceSelectEx/OnChoiceSelect/
/// OnChoiceTimeout）は choice-select-events 2.3 で同じ前例に倣い additive 追加された
/// （whitelist が意図的に 6→8→11 へ増えたための更新）。3 ID はいずれも正典（ukadoc）の
/// 固定イベント ID であり、「表＝正典固定 ID の部分集合」の性質は保たれる。
#[test]
fn allowed_event_ids_are_exactly_the_eleven_and_exclude_ontalk_onhour() {
    assert_eq!(
        ALLOWED_EVENT_IDS,
        &[
            "OnInitialize",
            "OnFirstBoot",
            "OnBoot",
            "basewareversion",
            "OnSecondChange",
            "OnClose",
            "OnMouseMove",
            "OnMouseDoubleClick",
            "OnChoiceSelectEx",
            "OnChoiceSelect",
            "OnChoiceTimeout",
        ]
    );
    assert!(
        is_allowed_event_id("OnMouseMove"),
        "OnMouseMove は許可集合に属する（Req7.1）"
    );
    assert!(
        is_allowed_event_id("OnMouseDoubleClick"),
        "OnMouseDoubleClick は許可集合に属する（Req7.1）"
    );
    for id in ["OnChoiceSelectEx", "OnChoiceSelect", "OnChoiceTimeout"] {
        assert!(
            is_allowed_event_id(id),
            "{id} は選択関連の正典固定 ID ゆえ許可集合に属する（DD-2）"
        );
    }
    assert!(
        !is_allowed_event_id("OnTalk"),
        "OnTalk は恒久的に許可しない（Req3.2）"
    );
    assert!(
        !is_allowed_event_id("OnHour"),
        "OnHour は恒久的に許可しない（Req3.2）"
    );
    // 表の全要素が許可判定を通ること。
    for id in ALLOWED_EVENT_IDS {
        assert!(is_allowed_event_id(id), "{id} は表にあるのに許可されない");
    }
}

/// 選択起源の受理規則（Req2.6・DD-2）: `On` 接頭のみを受理し、事前の固定登録を要さない。
///
/// 判定は接頭辞ただ 1 条件——ゴースト作者が `\q` の ID に書いた名前を逐語で受理するため、
/// 固定表（[`ALLOWED_EVENT_IDS`]）への登録有無・大小文字の揺れの補正はいずれも行わない。
#[test]
fn is_allowed_choice_event_accepts_only_on_prefixed_names_verbatim() {
    // 受理: 未登録の任意名・境界入力（"On" 単独）・正典固定 ID の逐語形。
    for id in [
        "On",
        "OnMenu",
        "Onおしゃべり頻度メニュー",
        "OnChoiceSelect",
        "On ",
    ] {
        assert!(
            is_allowed_choice_event(id),
            "{id} は On 接頭ゆえ選択起源として受理される（Req2.6）"
        );
    }
    // 拒否: On 接頭でない形（空文字・小文字・大文字・部分一致・接頭でない位置）。
    for id in ["", "foo", "on", "onMenu", "ONMENU", "MenuOn", " OnMenu"] {
        assert!(
            !is_allowed_choice_event(id),
            "{id:?} は On 接頭でないゆえ選択起源として受理されない（Req2.6）"
        );
    }
}

/// 裁定 8（Req2.9）: スケジューラ起源の恒久禁止と choice 起源の逐語発火が**交差しない**。
///
/// `OnTalk`／`OnHour` は「ベースウェアが自発的に周期発火すると消費側ゴーストの自発生成と
/// 二重駆動する」ことを根拠に固定表から恒久的に除外されるが、この根拠はゴースト作者が
/// 選択肢へ明示的に書いた 1 クリック = 1 回の発火には該当しない。ゆえに同じ ID が
/// スケジューラ起源では拒否・選択起源では受理される（両方向をこの 1 檻で固定する）。
#[test]
fn scheduler_forbidden_ids_are_still_fireable_from_choice_origin() {
    for id in ["OnTalk", "OnHour"] {
        assert!(
            !is_allowed_event_id(id),
            "{id} はスケジューラ起源では恒久的に禁止（Req3.2・自発生成との二重駆動）"
        );
        assert!(
            is_allowed_choice_event(id),
            "{id} は選択起源なら逐語で発火できる（Req2.9・恒久禁止を適用しない）"
        );
    }
}

/// OnMouseMove 正典 layout（Req2.1/2.2/2.5・DD-IE-6）: References が期待7並びと完全一致。
#[test]
fn on_mouse_move_builds_canonical_seven_reference_layout() {
    let call = on_mouse_move(10, 20, 0, Some("Head"), &ExecutionSnapshot::INACTIVE);
    assert_eq!(
        call_status(&call),
        None,
        "INACTIVE スナップショットは Status ヘッダを出さない"
    );
    let (id, references) = expect_get(call);
    assert_eq!(id, "OnMouseMove");
    assert_eq!(
        references,
        vec![
            "10".to_string(),    // Ref0=x
            "20".to_string(),    // Ref1=y
            "0".to_string(),     // Ref2=wheel（M1 固定・Req2.4）
            "0".to_string(),     // Ref3=scope（本体0）
            "Head".to_string(),  // Ref4=region（不透明転写）
            "0".to_string(),     // Ref5=移動は常に "0"（Req2.5）
            "mouse".to_string(), // Ref6=デバイス種（DD-IE-6）
        ]
    );
    assert_eq!(references.len(), 7, "Reference 数は常に 7");
}

/// Ref4 の None→空文字転写（Req2.3・DD-IE-6）: 位置は保持され Vec 長は 7 のまま。
#[test]
fn on_mouse_move_region_none_transcribes_to_empty_ref4() {
    let (_, references) = expect_get(on_mouse_move(1, 2, 1, None, &ExecutionSnapshot::INACTIVE));
    assert_eq!(references[4], "", "None は空文字へ転写（省略ではない）");
    assert_eq!(references[3], "1", "Ref3=scope 相方は 1");
    assert_eq!(references.len(), 7, "None でも Reference 数は 7 のまま");
}

/// OnMouseDoubleClick 正典 layout・左ボタン（Req3.1/3.2/3.3）: Ref5="0"・Ref2="0"・Ref6="mouse"。
#[test]
fn on_mouse_double_click_left_builds_ref5_zero() {
    let call = on_mouse_double_click(
        10,
        20,
        0,
        Some("Bust"),
        MouseButton::Left,
        &ExecutionSnapshot::INACTIVE,
    );
    let (id, references) = expect_get(call);
    assert_eq!(id, "OnMouseDoubleClick");
    assert_eq!(
        references,
        vec![
            "10".to_string(),
            "20".to_string(),
            "0".to_string(), // Ref2=常に "0"（Req3.2）
            "0".to_string(),
            "Bust".to_string(),
            "0".to_string(), // Ref5=左 "0"（Req3.3）
            "mouse".to_string(),
        ]
    );
}

/// OnMouseDoubleClick 右ボタン（Req3.3）: Ref5="1"。
#[test]
fn on_mouse_double_click_right_builds_ref5_one() {
    let (_, references) = expect_get(on_mouse_double_click(
        -5,
        0,
        1,
        None,
        MouseButton::Right,
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(references[5], "1", "右ボタンは Ref5 \"1\"（Req3.3）");
    assert_eq!(references[2], "0", "Ref2 は常に \"0\"（Req3.2）");
    assert_eq!(references[4], "", "Ref4 None→空文字（Req3.4）");
    assert_eq!(references[6], "mouse", "Ref6=デバイス種");
    assert_eq!(references.len(), 7);
}

/// talk_active=true では両構築子が `Status: talking` を snapshot から導出する（DD-IT-3）。
#[test]
fn mouse_constructors_carry_talking_status_when_active() {
    let active = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
    };
    let mv = on_mouse_move(0, 0, 0, Some("Head"), &active);
    assert_eq!(call_status(&mv), Some("talking".to_string()));
    let dbl = on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &active);
    assert_eq!(call_status(&dbl), Some("talking".to_string()));
}

/// 全構築関数の返す `id` が**スケジューラ起源**（[`EventId::Static`]）であること（DD-1）。
///
/// 選択起源（[`EventId::Choice`]）はカスケード planner のみが構成する不変条件を、構築関数側から
/// 固定する檻——events.rs の構築関数が任意名を作り得ないことを型の実値で観測する。
///
/// 対象は**スケジューラ起源**の構築関数のみ。選択起源の [`on_choice_named`] は設計どおり
/// [`EventId::Choice`] を返すため本檻の被覆対象ではなく、
/// `choice_constructors_split_event_id_category_by_origin` が別途カテゴリを固定する。
#[test]
fn every_construction_function_returns_static_event_id() {
    let cfg = config();
    let snap = ExecutionSnapshot::INACTIVE;
    let calls = [
        on_initialize(&snap),
        on_first_boot(&snap, 0),
        on_boot(&cfg, &snap),
        baseware_version(&cfg, &snap),
        on_second_change(MonotonicMs(0), &snap),
        on_second_change(
            MonotonicMs(0),
            &ExecutionSnapshot {
                talk_active: true,
                choice_active: false,
            },
        ),
        on_close(CloseReason::User, &snap),
        on_close_notify(CloseReason::System, &snap),
        on_mouse_move(0, 0, 0, Some("Head"), &snap),
        on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &snap),
    ];
    for call in &calls {
        let id = event_id(call);
        assert!(
            matches!(id, EventId::Static(_)),
            "構築関数がスケジューラ起源でない id={} を返した",
            id.as_str()
        );
    }
}

/// 全構築関数の返す `id` が許可集合の要素であること（Service Interface Postcondition）。
///
/// 対象は [`EventId::Static`] を返す構築関数——固定 3 ID を許可表へ載せた（DD-2）ことにより
/// `on_choice_select_ex`／`on_choice_select`／`on_choice_timeout` も本檻の被覆対象に入る。
/// 選択起源の任意名 `on_choice_named`（[`EventId::Choice`]）だけは固定表ではなく出所別の
/// 受理規則（`is_allowed_choice_event`）で検証されるため、本檻の被覆対象外である。
#[test]
fn every_construction_function_returns_an_allowed_id() {
    let cfg = config();
    let snap = ExecutionSnapshot::INACTIVE;
    let calls = [
        on_initialize(&snap),
        on_first_boot(&snap, 0),
        on_boot(&cfg, &snap),
        baseware_version(&cfg, &snap),
        on_second_change(MonotonicMs(0), &snap),
        on_second_change(
            MonotonicMs(0),
            &ExecutionSnapshot {
                talk_active: true,
                choice_active: false,
            },
        ),
        on_close(CloseReason::User, &snap),
        on_close_notify(CloseReason::System, &snap),
        on_mouse_move(0, 0, 0, Some("Head"), &snap),
        on_mouse_double_click(0, 0, 0, None, MouseButton::Left, &snap),
        on_choice_select_ex("ラベル", "ID", &[], &snap),
        on_choice_select("ID", &snap),
        on_choice_timeout("\\e", &snap),
    ];
    for call in &calls {
        let id = event_id(call).as_str();
        assert!(
            is_allowed_event_id(id),
            "構築関数が許可集合外の id={id} を返した"
        );
    }
}

/// テスト用の付随参照列（不透明転写の檻に使う「加工されたら壊れる」値の並び）。
///
/// 非 ASCII・前後空白・空文字要素・記号（カンマ／バックスラッシュ）を含み、トリム・
/// 正規化・空要素除去のいずれかが混入すれば必ず不一致になる。
fn opaque_references() -> Vec<String> {
    vec![
        " 頻度  ".to_string(),
        String::new(),
        "a,b".to_string(),
        "\\q[x,y]".to_string(),
    ]
}

/// `OnChoiceSelectEx` 正典 layout（Req3.1）:
/// Ref0=ラベル／Ref1=ID／Ref2 以降が付随参照列の記述順であること（位置と値の実値突合）。
#[test]
fn on_choice_select_ex_builds_label_id_then_references() {
    let references = opaque_references();
    let call = on_choice_select_ex(
        "おしゃべり頻度",
        "Choice頻度",
        &references,
        &ExecutionSnapshot::INACTIVE,
    );
    let (id, refs) = expect_get(call);
    assert_eq!(id, "OnChoiceSelectEx");
    assert_eq!(
        refs,
        vec![
            "おしゃべり頻度".to_string(), // Ref0=表示ラベル（Req3.1）
            "Choice頻度".to_string(),     // Ref1=選択肢 ID（Req3.1）
            " 頻度  ".to_string(),        // Ref2 以降＝付随参照列を記述順（不透明転写）
            String::new(),
            "a,b".to_string(),
            "\\q[x,y]".to_string(),
        ]
    );
    assert_eq!(
        refs.len(),
        2 + references.len(),
        "Reference 数は 2＋付随参照列長"
    );
}

/// 空参照列で Ref2 以降の位置が生えないこと（Req3.5）: Reference は Ref0/Ref1 の**2 個のみ**。
///
/// 既存マウス系の `None→""`（位置保持）とは**非対称**な規約であることを実値で固定する。
#[test]
fn on_choice_select_ex_with_empty_references_stops_at_ref1() {
    let (id, refs) = expect_get(on_choice_select_ex(
        "ラベル",
        "ID",
        &[],
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(id, "OnChoiceSelectEx");
    assert_eq!(refs, vec!["ラベル".to_string(), "ID".to_string()]);
    assert_eq!(
        refs.len(),
        2,
        "空参照列は Ref2 以降の位置を作らない（空文字で埋めない）"
    );
}

/// `OnChoiceSelect` 正典 layout（Req3.2）: Ref0=選択肢 ID の**1 個のみ**。
#[test]
fn on_choice_select_builds_id_only_ref0() {
    let (id, refs) = expect_get(on_choice_select("Choice頻度", &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnChoiceSelect");
    assert_eq!(refs, vec!["Choice頻度".to_string()]);
    assert_eq!(refs.len(), 1, "無印は常に Ref0=ID の 1 個のみ");
}

/// 任意名イベント正典 layout（Req3.3）: Ref0 以降が付随参照列のみで、
/// 表示ラベルと選択肢 ID を Reference に**含めない**こと。
#[test]
fn on_choice_named_builds_references_from_ref0_without_label_or_id() {
    let references = opaque_references();
    let call = on_choice_named(
        "Onおしゃべり頻度メニュー".to_string(),
        &references,
        &ExecutionSnapshot::INACTIVE,
    );
    let (id, refs) = expect_get(call);
    assert_eq!(id, "Onおしゃべり頻度メニュー", "任意名は逐語で wire へ載る");
    assert_eq!(
        refs, references,
        "Ref0 以降＝付随参照列そのもの（記述順・不透明転写）"
    );
    assert!(
        !refs.contains(&"Onおしゃべり頻度メニュー".to_string()),
        "任意名イベントの Reference に選択肢 ID を含めない（Req3.3）"
    );
}

/// 空参照列で Reference が 1 個も生えないこと（Req3.3/3.5）: References は空 Vec。
#[test]
fn on_choice_named_with_empty_references_builds_no_reference() {
    let (id, refs) = expect_get(on_choice_named(
        "OnMenu".to_string(),
        &[],
        &ExecutionSnapshot::INACTIVE,
    ));
    assert_eq!(id, "OnMenu");
    assert!(
        refs.is_empty(),
        "空参照列は Reference 位置を 1 個も作らない（空文字で埋めない・Req3.5）"
    );
}

/// `OnChoiceTimeout` 正典 layout（Req3.4）: Ref0=起動スクリプトの**1 個のみ**・不透明転写。
#[test]
fn on_choice_timeout_builds_script_ref0() {
    let script = "\\0\\s[0]選んで\\q[はい,Onはい]\\q[いいえ,Onいいえ]\\e";
    let (id, refs) = expect_get(on_choice_timeout(script, &ExecutionSnapshot::INACTIVE));
    assert_eq!(id, "OnChoiceTimeout");
    assert_eq!(refs, vec![script.to_string()]);
    assert_eq!(refs.len(), 1, "Timeout は常に Ref0=script の 1 個のみ");
}

/// 共通リクエストヘッダ（実行状態スナップショット）が 4 構築関数すべてに載ること（Req3.6）。
///
/// snapshot が必須引数であるため欠落は構造上起こらない。実値としても、
/// INACTIVE→ヘッダ行なし（`None`）／talk_active=true→`talking` の双方向を固定する。
#[test]
fn choice_constructors_carry_the_common_request_header() {
    let refs = opaque_references();
    let active = ExecutionSnapshot {
        talk_active: true,
        choice_active: false,
    };
    let idle = ExecutionSnapshot::INACTIVE;

    let active_calls = [
        on_choice_select_ex("ラベル", "ID", &refs, &active),
        on_choice_select("ID", &active),
        on_choice_named("OnMenu".to_string(), &refs, &active),
        on_choice_timeout("\\e", &active),
    ];
    for call in &active_calls {
        assert_eq!(
            call_status(call),
            Some("talking".to_string()),
            "選択関連イベントも共通ヘッダを snapshot から導出する（Req3.6）: id={}",
            event_id(call).as_str()
        );
    }

    let idle_calls = [
        on_choice_select_ex("ラベル", "ID", &refs, &idle),
        on_choice_select("ID", &idle),
        on_choice_named("OnMenu".to_string(), &refs, &idle),
        on_choice_timeout("\\e", &idle),
    ];
    for call in &idle_calls {
        assert_eq!(
            call_status(call),
            None,
            "非アクティブ snapshot は Status ヘッダ行を出さない: id={}",
            event_id(call).as_str()
        );
    }
}

/// 出所カテゴリの型分離（DD-1）: 任意名イベントのみ [`EventId::Choice`]、
/// 固定 ID 3 種は [`EventId::Static`]。
#[test]
fn choice_constructors_split_event_id_category_by_origin() {
    let snap = ExecutionSnapshot::INACTIVE;
    let named = on_choice_named("OnMenu".to_string(), &[], &snap);
    assert!(
        matches!(event_id(&named), EventId::Choice(_)),
        "任意名イベントは選択起源（EventId::Choice）"
    );

    let statics = [
        on_choice_select_ex("ラベル", "ID", &[], &snap),
        on_choice_select("ID", &snap),
        on_choice_timeout("\\e", &snap),
    ];
    for call in &statics {
        let id = event_id(call);
        assert!(
            matches!(id, EventId::Static(_)),
            "固定 ID の選択関連イベントはスケジューラ起源の型を保つ id={}",
            id.as_str()
        );
    }
}
