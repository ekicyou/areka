//! 項目名の照会と表示可否（[`super`]）の決定論テスト（要件 9.2 の写し側・9.3・3.8）。
//!
//! 確かめること: 枠とリソース名と既定名の対応表・表示可否の名前をスコープで選ぶこと・
//! 問い合わせる名前の並びと重複の潰し方・返り値から項目名への 4 通りの写し・失敗の記録が
//! 1 回の表示につき 1 行であること・上限超過（`Timeout`）で全件が既定名になること・
//! 表示可否が `0` のときだけ抑止されること・問い合わせない 4 名が運行の許可表に無いこと。
//!
//! 期待値はすべて書き下した値で、対応表や [`Frame::ORDER`] から導かない（表を書き換えたら
//! 赤になるように）。ただし「問い合わせない 4 名」だけは運行（kanade）の本物の許可表と
//! 突き合わせる——写しを置くと、許可表が増えた日に気付けないからである。

use std::rc::Rc;
use std::sync::mpsc::{TryRecvError, channel};

use areka_kanade::KanadeMsg;
use areka_kanade::resources::{ResourceOutcome, is_allowed_resource_id};
use log_capture_kit::{LineFormat, capture_lines};

use super::*;
use crate::menu::{ItemBody, MenuItem};

/// 文言にリソースを使う葉の項目。
fn leaf(label: &str, resource: Option<&'static str>) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        caption_resource: resource,
        enabled: true,
        checked: None,
        body: ItemBody::Action(Rc::new(|_, _| {})),
    }
}

/// 子項目を持つ見出し。
fn submenu(label: &str, resource: Option<&'static str>, children: Vec<MenuItem>) -> MenuItem {
    MenuItem {
        label: label.to_string(),
        caption_resource: resource,
        enabled: true,
        checked: None,
        body: ItemBody::Submenu(children),
    }
}

/// 第 1 スライスの写し（⑥説明書・⑦終了だけが登記されている）。
fn readme_and_close_snapshot() -> Vec<(Frame, MenuItem)> {
    vec![
        (Frame::Readme, leaf("説明書", Some("readmebutton.caption"))),
        (Frame::Close, leaf("終了", Some("closebutton.caption"))),
    ]
}

/// 値が返った 1 件。
fn value(id: &'static str, body: &str) -> (&'static str, ResourceOutcome) {
    (id, ResourceOutcome::Value(body.to_string()))
}

/// 失敗した 1 件。
fn failed(id: &'static str, reason: &str) -> (&'static str, ResourceOutcome) {
    (id, ResourceOutcome::Failed(reason.to_string()))
}

/// 指定のレベルの行だけを拾う。
fn lines_at<'a>(lines: &'a [String], level: &str) -> Vec<&'a String> {
    let needle = format!("level={level}");
    lines.iter().filter(|l| l.contains(&needle)).collect()
}

/// 本体側（スコープ 0）の写しを取りつつ、記録された行も返す。
fn interpret_capturing(reply: Result<QueryReply, QueryFailure>) -> (Interpreted, Vec<String>) {
    capture_lines(LineFormat::LevelFields, || {
        interpret(reply, SAKURA_POPUPMENU_VISIBLE, 0)
    })
}

/// 表示可否だけを 1 件返す照会結果から、表示するかどうかと記録を得る。
fn visibility_of(outcome: ResourceOutcome) -> (Visibility, Vec<String>) {
    let (interpreted, lines) = interpret_capturing(Ok(vec![(SAKURA_POPUPMENU_VISIBLE, outcome)]));
    (interpreted.visibility, lines)
}

/// 対応表は枠の数と同じ長さで、[`Frame::ORDER`] の順に要件 3.1 の組を並べる。
#[test]
fn frame_captions_table_is_the_seven_rows_of_requirement_3_1() {
    assert_eq!(
        FRAME_CAPTIONS.len(),
        Frame::ORDER.len(),
        "枠を増やしたら対応表も同時に増やす"
    );
    assert_eq!(
        FRAME_CAPTIONS,
        [
            ("ghostrootbutton.caption", Frame::Ghost, "ゴースト"),
            ("shellrootbutton.caption", Frame::Shell, "シェル"),
            ("balloonrootbutton.caption", Frame::Balloon, "バルーン"),
            ("updatebutton.caption", Frame::Update, "ネットワーク更新"),
            (
                "ghostinstallbutton.caption",
                Frame::Install,
                "インストール…"
            ),
            ("readmebutton.caption", Frame::Readme, "説明書"),
            ("closebutton.caption", Frame::Close, "終了"),
        ]
        .as_slice()
    );
    for (i, frame) in Frame::ORDER.iter().enumerate() {
        assert_eq!(FRAME_CAPTIONS[i].1, *frame, "対応表は枠の並び順に並べる");
    }
}

/// 枠からリソース名と既定名を引く。
#[test]
fn resource_for_and_default_label_read_the_table() {
    assert_eq!(resource_for(Frame::Readme), "readmebutton.caption");
    assert_eq!(default_label(Frame::Readme), "説明書");
    assert_eq!(resource_for(Frame::Close), "closebutton.caption");
    assert_eq!(default_label(Frame::Close), "終了");
    assert_eq!(resource_for(Frame::Ghost), "ghostrootbutton.caption");
    assert_eq!(default_label(Frame::Update), "ネットワーク更新");
}

/// 表示可否の問い合わせ先はスコープで選ぶ（要件 3.6）。
#[test]
fn visible_resource_is_chosen_by_scope() {
    assert_eq!(SAKURA_POPUPMENU_VISIBLE, "sakura.popupmenu.visible");
    assert_eq!(KERO_POPUPMENU_VISIBLE, "kero.popupmenu.visible");
    assert_eq!(visible_resource_for(0), "sakura.popupmenu.visible");
    assert_eq!(visible_resource_for(1), "kero.popupmenu.visible");
}

/// 問い合わせない 4 名は運行の許可表に 1 つも無く、引く 9 名はすべて許可表にある（要件 3.8・3.1・3.6）。
#[test]
fn unqueried_resources_are_absent_from_the_real_allow_table() {
    assert_eq!(
        UNQUERIED_POPUPMENU_RESOURCES,
        [
            "char*.popupmenu.visible",
            "sakura.popupmenu.type",
            "kero.popupmenu.type",
            "char*.popupmenu.type",
        ]
        .as_slice()
    );
    for id in UNQUERIED_POPUPMENU_RESOURCES {
        assert!(
            !is_allowed_resource_id(id),
            "{id} は問い合わせない名前なので運行の許可表に入れてはならない"
        );
    }
    for (id, _, _) in FRAME_CAPTIONS {
        assert!(is_allowed_resource_id(id), "{id} を引けない");
    }
    assert!(is_allowed_resource_id(SAKURA_POPUPMENU_VISIBLE));
    assert!(is_allowed_resource_id(KERO_POPUPMENU_VISIBLE));
}

/// 問い合わせる名前は「表示可否が先頭・続いて写しに現れた順」。第 1 スライスは 3 件。
#[test]
fn query_ids_puts_the_visible_name_first_then_the_snapshot_order() {
    let snapshot = readme_and_close_snapshot();
    assert_eq!(
        query_ids(&snapshot, 0),
        [
            "sakura.popupmenu.visible",
            "readmebutton.caption",
            "closebutton.caption",
        ]
    );
    assert_eq!(
        query_ids(&snapshot, 1),
        [
            "kero.popupmenu.visible",
            "readmebutton.caption",
            "closebutton.caption",
        ]
    );
}

/// サブメニューの子のリソース名も集め、重複は 1 つに潰す（並びは最初に現れた順）。
#[test]
fn query_ids_collects_submenu_children_and_collapses_duplicates() {
    let snapshot = vec![
        (
            Frame::Ghost,
            submenu(
                "ゴースト",
                Some("ghostrootbutton.caption"),
                vec![
                    leaf("子1", None),
                    leaf("子2", Some("closebutton.caption")),
                    submenu(
                        "孫の親",
                        None,
                        vec![leaf("孫", Some("updatebutton.caption"))],
                    ),
                ],
            ),
        ),
        (Frame::Close, leaf("終了", Some("closebutton.caption"))),
    ];
    assert_eq!(
        query_ids(&snapshot, 0),
        [
            "sakura.popupmenu.visible",
            "ghostrootbutton.caption",
            "closebutton.caption",
            "updatebutton.caption",
        ]
    );
}

/// 値が非空ならその文言をそのまま使う（`&` もそのまま・要件 3.2）。
#[test]
fn non_empty_value_becomes_the_caption_verbatim() {
    let (interpreted, lines) = interpret_capturing(Ok(vec![
        value(SAKURA_POPUPMENU_VISIBLE, "1"),
        value("readmebutton.caption", "取扱説明書(&R)"),
    ]));
    assert_eq!(
        interpreted.captions.get("readmebutton.caption"),
        Some("取扱説明書(&R)")
    );
    assert!(lines_at(&lines, "WARN").is_empty(), "{lines:?}");
    assert!(lines_at(&lines, "DEBUG").is_empty(), "{lines:?}");
}

/// 空文字列と値なしは既定名（表に入らない）＋ `debug!` で記録する（要件 3.3）。
#[test]
fn empty_value_and_no_content_fall_back_to_the_default_label_with_debug() {
    for outcome in [
        ResourceOutcome::Value(String::new()),
        ResourceOutcome::NoContent,
    ] {
        let (interpreted, lines) =
            interpret_capturing(Ok(vec![("readmebutton.caption", outcome.clone())]));
        assert_eq!(
            interpreted.captions.get("readmebutton.caption"),
            None,
            "{outcome:?} は既定名に落ちる（文言の表に入らない）"
        );
        let debug = lines_at(&lines, "DEBUG");
        assert_eq!(debug.len(), 1, "{outcome:?} の記録: {lines:?}");
        assert!(debug[0].contains("readmebutton.caption"), "{:?}", debug[0]);
        assert!(debug[0].contains("[menu]"), "{:?}", debug[0]);
        assert!(
            lines_at(&lines, "WARN").is_empty(),
            "空は警告ではない: {lines:?}"
        );
    }
}

/// 失敗した項目は既定名に落ちる（要件 3.4）。
#[test]
fn failed_resource_falls_back_to_the_default_label() {
    let (interpreted, _) =
        interpret_capturing(Ok(vec![failed("readmebutton.caption", "shiori timeout")]));
    assert_eq!(interpreted.captions.get("readmebutton.caption"), None);
}

/// 2 件失敗しても記録は 1 回の表示につき 1 行で、両方の名前を載せる（要件 3.4 の「1 回」）。
#[test]
fn two_failed_ids_are_reported_in_a_single_warn_line() {
    let (interpreted, lines) = interpret_capturing(Ok(vec![
        value(SAKURA_POPUPMENU_VISIBLE, "1"),
        failed("readmebutton.caption", "broken reply"),
        failed("closebutton.caption", "shiori down"),
    ]));
    assert_eq!(interpreted.captions.get("readmebutton.caption"), None);
    assert_eq!(interpreted.captions.get("closebutton.caption"), None);
    let warns = lines_at(&lines, "WARN");
    assert_eq!(warns.len(), 1, "失敗 2 件でも警告は 1 行: {lines:?}");
    assert!(warns[0].contains("readmebutton.caption"), "{:?}", warns[0]);
    assert!(warns[0].contains("closebutton.caption"), "{:?}", warns[0]);
    assert!(warns[0].contains("broken reply"), "{:?}", warns[0]);
    assert!(warns[0].contains("shiori down"), "{:?}", warns[0]);
    assert!(warns[0].contains("[menu]"), "{:?}", warns[0]);
    assert!(
        matches!(interpreted.visibility, Visibility::Show),
        "失敗してもメニューは出す"
    );
}

/// 上限超過は全件既定名で、警告は 1 行・理由に `timeout` を載せる（要件 3.4）。
#[test]
fn timeout_defaults_every_id_with_one_warn_carrying_the_reason() {
    let (interpreted, lines) = interpret_capturing(Err(QueryFailure::Timeout));
    for (id, _, _) in FRAME_CAPTIONS {
        assert_eq!(interpreted.captions.get(id), None, "{id} も既定名へ落ちる");
    }
    let warns = lines_at(&lines, "WARN");
    assert_eq!(warns.len(), 1, "{lines:?}");
    assert!(warns[0].contains("timeout"), "{:?}", warns[0]);
    assert!(warns[0].contains("[menu]"), "{:?}", warns[0]);
    assert!(
        matches!(interpreted.visibility, Visibility::Show),
        "返事が来なくてもメニューは出す（要件 3.7）"
    );
}

/// 切断と送出失敗も同じく全件既定名＋警告 1 行で、理由の綴りが違う。
#[test]
fn dropped_and_send_failed_carry_their_own_reason() {
    for (failure, reason) in [
        (QueryFailure::Dropped, "dropped"),
        (QueryFailure::SendFailed, "send_failed"),
    ] {
        let (interpreted, lines) = interpret_capturing(Err(failure));
        let warns = lines_at(&lines, "WARN");
        assert_eq!(warns.len(), 1, "{reason}: {lines:?}");
        assert!(warns[0].contains(reason), "{:?}", warns[0]);
        assert!(matches!(interpreted.visibility, Visibility::Show));
    }
}

/// 表示可否が `0` のときだけ抑止し、`info!` を 1 回残す（要件 3.7）。
#[test]
fn visible_zero_suppresses_the_menu_and_records_once() {
    let (visibility, lines) = visibility_of(ResourceOutcome::Value("0".to_string()));
    assert!(matches!(visibility, Visibility::Suppress));
    let infos = lines_at(&lines, "INFO");
    assert_eq!(infos.len(), 1, "{lines:?}");
    assert!(
        infos[0].contains("sakura.popupmenu.visible"),
        "{:?}",
        infos[0]
    );
    assert!(
        infos[0].contains("[menu] suppressed by popupmenu.visible"),
        "{:?}",
        infos[0]
    );
}

/// `0` 以外・値なし・失敗・返事なしはすべて「出す」（要件 3.7）。
#[test]
fn everything_other_than_zero_shows_the_menu() {
    for outcome in [
        ResourceOutcome::Value("1".to_string()),
        ResourceOutcome::Value(String::new()),
        ResourceOutcome::Value(" 0 ".to_string()),
        ResourceOutcome::Value("00".to_string()),
        ResourceOutcome::NoContent,
        ResourceOutcome::Failed("shiori down".to_string()),
    ] {
        let (visibility, lines) = visibility_of(outcome.clone());
        assert!(
            matches!(visibility, Visibility::Show),
            "{outcome:?} は「出す」: {lines:?}"
        );
        assert!(
            lines_at(&lines, "INFO").is_empty(),
            "抑止していないので info! は残さない: {lines:?}"
        );
    }
    let (interpreted, _) = interpret_capturing(Err(QueryFailure::Dropped));
    assert!(matches!(interpreted.visibility, Visibility::Show));
}

/// 表示可否の名前は文言の表へ入れない（項目名に化けない）。
#[test]
fn the_visible_name_never_becomes_a_caption() {
    let (interpreted, _) = interpret_capturing(Ok(vec![value(SAKURA_POPUPMENU_VISIBLE, "1")]));
    assert_eq!(interpreted.captions.get(SAKURA_POPUPMENU_VISIBLE), None);
}

/// 照会は 1 通だけ送り、待たずに受信端を返す。返事は受信端から覗ける（要件 3.2）。
#[test]
fn send_query_posts_exactly_one_message_and_returns_a_receiver() {
    let (tx, kanade) = channel::<KanadeMsg>();
    let ids = vec!["sakura.popupmenu.visible", "readmebutton.caption"];
    let rx = send_query(&tx, ids.clone()).expect("受け手が生きていれば送れる");

    assert!(
        matches!(rx.try_recv(), Ok(None)),
        "送っただけでは返事はまだ無い（待たない）"
    );
    let msg = kanade.try_recv().expect("照会が 1 通届く");
    assert!(
        matches!(kanade.try_recv(), Err(TryRecvError::Empty)),
        "送るのは 1 通だけ"
    );
    let KanadeMsg::ResourceQuery { ids: sent, reply } = msg else {
        panic!("送るのは ResourceQuery");
    };
    assert_eq!(sent, ids);

    reply
        .send(vec![value("readmebutton.caption", "取扱説明書")])
        .expect("受信端は生きている");
    let got = rx.try_recv().expect("切れていない").expect("返事が届く");
    assert_eq!(got, vec![value("readmebutton.caption", "取扱説明書")]);
}

/// 運行が止まっていて送れなければ `SendFailed`（受け手は全件既定名で出す）。
#[test]
fn send_query_reports_send_failed_when_kanade_is_gone() {
    let (tx, kanade) = channel::<KanadeMsg>();
    drop(kanade);
    assert!(matches!(
        send_query(&tx, vec!["readmebutton.caption"]),
        Err(QueryFailure::SendFailed)
    ));
}
