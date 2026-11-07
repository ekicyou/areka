//! 知らせの型（[`KanadeMsg`]・[`ShioriCall`] ほか）のテスト。

use super::*;

/// `Send + 'static` を型検査で強制する静的アサーション。
fn assert_send_static<T: Send + 'static>() {}

#[test]
fn kanade_and_shiori_msgs_are_send_static() {
    // 観測可能な完了条件: メッセージ型が Send + 'static な所有データのみで構成される。
    assert_send_static::<KanadeMsg>();
    assert_send_static::<ShioriMsg>();
    assert_send_static::<ShioriCall>();
    assert_send_static::<ShioriOutcome>();
    assert_send_static::<ShioriFailure>();
    assert_send_static::<KanadeConfig>();
    assert_send_static::<CloseReason>();
    assert_send_static::<MonotonicMs>();
    // マウス入力契約型（DD-IE 系）も Send + 'static な所有データのみ。
    assert_send_static::<MouseInput>();
    assert_send_static::<MouseEventKind>();
    assert_send_static::<MouseButton>();
    // 送出イベント ID の出所カテゴリ型（DD-1）も同様。
    assert_send_static::<EventId>();
    // 選択確定入力も Send + 'static な所有データのみ。
    assert_send_static::<ChoiceInput>();
}

/// wire 形（SHIORI へ渡る文字列）は出所カテゴリに依らず逐語であること（DD-1・Req2.6/3.6）。
///
/// `EventId` はチョークポイントが出所別の受理規則を適用するためだけの区別であり、
/// SHIORI へ出る文字列は `as_str()`（および `Display`）が与える逐語表現のみである。
#[test]
fn event_id_as_str_is_verbatim_wire_form_for_both_origins() {
    assert_eq!(EventId::Static("OnBoot").as_str(), "OnBoot");
    assert_eq!(
        EventId::Static("basewareversion").as_str(),
        "basewareversion"
    );
    // 選択起源は任意名を逐語で運ぶ（事前登録不要・Req2.6）。
    assert_eq!(
        EventId::Choice("OnMenuBack".to_string()).as_str(),
        "OnMenuBack"
    );
    // 同一綴りなら出所が違っても wire 形は同一（表現の一字一句同一性）。
    assert_eq!(
        EventId::Choice("OnBoot".to_string()).as_str(),
        EventId::Static("OnBoot").as_str()
    );
    // Display は as_str と一致する（`id.to_string()` の既存呼出面が wire 形を保つ）。
    assert_eq!(EventId::Static("OnClose").to_string(), "OnClose");
    assert_eq!(
        EventId::Choice("OnMenuBack".to_string()).to_string(),
        "OnMenuBack"
    );
    // 外から頼まれた任意の名前も逐語（On 接頭を要さない）。
    assert_eq!(
        EventId::Sstp("UserDefined".to_string()).as_str(),
        "UserDefined"
    );
    // `\![get,property]` の返しのイベントも逐語（`On` で始まらなくてよい）。
    assert_eq!(
        EventId::Property("PastaShellScale".to_string()).as_str(),
        "PastaShellScale"
    );
}

#[test]
fn close_reason_maps_to_onclose_ref0() {
    assert_eq!(CloseReason::User { scope: 0 }.as_ref_str(), "user");
    assert_eq!(CloseReason::System.as_ref_str(), "system");
}

/// `KanadeConfig::new`（既定構築経路）が入れる既定値をまとめて固定する。
///
/// 選択肢タイムアウトの既定 30_000 ms は裁定 5（Req 7.8・DD-8）。正典（ukadoc）は数値を
/// 規定していないため areka 裁量で単一の値を定め、対応表へ記録する。追加の欄の既定
/// （true / 0 / 空）は既存の起動の檻を意味論無改変で緑に保つ（design C8）。
/// 起動の由来はふつうの起動・シェルのフォルダ名はシェル名の写し
/// （areka-P0-ghost-shell-balloon-switch 要件 2.1・4.1・11.7）。
#[test]
fn kanade_config_new_supplies_documented_defaults() {
    let config = KanadeConfig::new("master", "1.0.0");
    assert_eq!(config.shell_name, "master");
    assert_eq!(config.baseware_version, "1.0.0");
    assert_eq!(config.baseware_name, "areka");
    assert_eq!(config.close_talk_deadline_ms, 30_000);
    assert_eq!(
        config.choice_timeout_default_ms, 30_000,
        "choice_timeout_default_ms default must be 30_000 (Req 7.8・DD-8)"
    );
    assert!(
        config.first_boot,
        "first_boot default must be true (現行挙動不変)"
    );
    assert_eq!(
        config.vanish_count, 0,
        "vanish_count default must be 0 (OnFirstBoot Ref0)"
    );
    assert!(
        config.first_boot_epilogue.is_empty(),
        "first_boot_epilogue default must be empty (何も添付しない)"
    );
    // script 種別の記録の名は結線側が入れる（既定は空文字）。
    assert_eq!(config.ghost_name, "");
    assert_eq!(config.boot_origin, crate::change::BootOrigin::Plain);
    assert_eq!(config.shell_folder, "master");
}
