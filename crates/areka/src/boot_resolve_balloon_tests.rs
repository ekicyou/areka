//! バルーンを決める鎖の descript の段の決定論テスト（areka-P0-ghost-standard-balloon 要件 7.2〜7.5）。
//!
//! 確かめること: 段の並び（引数 → 記憶 → descript → 同梱 → 唯一 → 既定 → 無作為）と、
//! 当たらなかった段の記録の件数と欄、descript の 2 鍵の突き合わせの各場面、実行中のバルーンの
//! 切替の決め方との一致。鎖は I/O を持たないので、根は実在を問わず、
//! 一覧はフォルダ名と `name` の組をテストの中で組む。プロセス・fs・乱数には触れない。

use std::path::{Path, PathBuf};

use areka_ghost::catalog::{BalloonEntry, Identity};
use log_capture_kit::{CapturedEvent, capture};

use super::*;

// ---------------------------------------------------------------- 道具立て

/// 実在を問わない根（鎖は fs を見ない）。
fn root() -> BasewareRoot {
    BasewareRoot::new(PathBuf::from(r"C:\areka-boot-resolve-balloon-tests"))
}

/// `(フォルダ名, name)` の列を `list_balloons` の戻りの形へ。
fn entries(root: &BasewareRoot, v: &[(&str, Option<&str>)]) -> Vec<BalloonEntry> {
    v.iter()
        .map(|(folder, name)| BalloonEntry {
            dir: root.balloon_dir(folder),
            identity: Identity {
                folder: (*folder).to_owned(),
                name: name.map(str::to_owned),
                ..Default::default()
            },
        })
        .collect()
}

/// 鎖へ渡す欄（引数・記憶・descript の 2 鍵・同梱）。既定は全部無し。
#[derive(Default)]
struct Given<'a> {
    argv: Option<&'a Path>,
    memory: Option<&'a str>,
    path: Option<&'a str>,
    name: Option<&'a str>,
    companion: Option<&'a str>,
}

fn chain(
    root: &BasewareRoot,
    given: Given<'_>,
    listed: &[BalloonEntry],
    pick: impl FnOnce(usize) -> usize,
) -> (Result<BalloonDecision, NoBalloon>, Vec<CapturedEvent>) {
    let inputs = BalloonInputs {
        root,
        argv: given.argv,
        memory: given.memory,
        default_balloon_path: given.path,
        balloon_name: given.name,
        companion: given.companion,
        listed,
    };
    capture(|| resolve_balloon(&inputs, pick))
}

/// 呼ばれてはならない添字（無作為の段に届かない場面で使う）。
fn no_pick(n: usize) -> usize {
    panic!("無作為の段に届いてはならない（候補数 {n}）")
}

fn at(root: &BasewareRoot, route: BalloonRoute, folder: &str) -> BalloonDecision {
    BalloonDecision {
        route,
        dir: root.balloon_dir(folder),
        folder: Some(folder.to_owned()),
    }
}

fn hits<'e>(events: &'e [CapturedEvent], event: &str) -> Vec<&'e CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .collect()
}

/// `descript_balloon_not_found` の (key, value) の列（出た順）。どれも warn で置き場を載せる。
fn descript_misses(root: &BasewareRoot, events: &[CapturedEvent]) -> Vec<(String, String)> {
    let store = root.balloon_store().display().to_string();
    hits(events, "descript_balloon_not_found")
        .into_iter()
        .map(|e| {
            assert_eq!(e.level, tracing::Level::WARN, "{e:?}");
            assert_eq!(e.field("balloon_store"), Some(store.as_str()), "{e:?}");
            (
                e.field_str("key").expect("key").to_owned(),
                e.field_str("value").expect("value").to_owned(),
            )
        })
        .collect()
}

fn pair(key: &str, value: &str) -> (String, String) {
    (key.to_owned(), value.to_owned())
}

/// 記憶・descript・同梱のどれもが当たる候補の並び（既定のバルーンも在る）。
fn all_hit(root: &BasewareRoot) -> Vec<BalloonEntry> {
    entries(
        root,
        &[
            ("StayseeBalloon", Some("Staysee")),
            ("bundled", None),
            ("by_name", Some("名前で当たる")),
            ("by_path", None),
            ("mine", None),
        ],
    )
}

// ---------------------------------------------------------------- 段の並び（要件 3・7.2）

/// 記憶が根に在れば、descript の 2 鍵と同梱が当たっても記憶（記録 0 件）（要件 3.3）。
#[test]
fn memory_wins_over_descript_and_companion() {
    let root = root();
    let given = Given {
        memory: Some("mine"),
        path: Some("by_path"),
        name: Some("名前で当たる"),
        companion: Some("bundled"),
        ..Default::default()
    };
    let (got, events) = chain(&root, given, &all_hit(&root), no_pick);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Memory, "mine")));
    assert!(events.is_empty(), "{events:?}");
}

/// 記憶が無く descript が当たれば、同梱が当たっても descript（どちらの鍵でも・記録 0 件）（要件 3.4）。
#[test]
fn descript_wins_over_companion() {
    let root = root();
    let listed = all_hit(&root);
    for (path, name, want) in [
        (Some("by_path"), None, "by_path"),
        (None, Some("名前で当たる"), "by_name"),
    ] {
        let given = Given {
            path,
            name,
            companion: Some("bundled"),
            ..Default::default()
        };
        let (got, events) = chain(&root, given, &listed, no_pick);
        assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, want)));
        assert!(events.is_empty(), "{events:?}");
    }
}

/// 同梱は既定より先（候補 3 個・既定のバルーンも在る・記録 0 件）（要件 3.5）。
#[test]
fn companion_wins_over_default() {
    let root = root();
    let listed = entries(
        &root,
        &[("StayseeBalloon", None), ("a", None), ("bundled", None)],
    );
    let given = Given {
        companion: Some("bundled"),
        ..Default::default()
    };
    let (got, events) = chain(&root, given, &listed, no_pick);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert!(events.is_empty(), "{events:?}");
}

/// 同梱は唯一より先（候補 1 個がそのまま同梱の名前 → 段は同梱）（要件 7.2）。
#[test]
fn companion_wins_over_only() {
    let root = root();
    let listed = entries(&root, &[("bundled", None)]);
    let given = Given {
        companion: Some("bundled"),
        ..Default::default()
    };
    let (got, events) = chain(&root, given, &listed, no_pick);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert!(events.is_empty(), "{events:?}");
}

/// 同梱は無作為より先（候補 2 個・既定のバルーンなし。添字は呼ばれない）（要件 3.5）。
#[test]
fn companion_wins_over_random() {
    let root = root();
    let listed = entries(&root, &[("a", None), ("bundled", None)]);
    let given = Given {
        companion: Some("bundled"),
        ..Default::default()
    };
    let (got, events) = chain(&root, given, &listed, no_pick);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert!(events.is_empty(), "{events:?}");
}

/// 記憶の先が根に無い: 記憶の警告 1 件で descript へ（descript の警告は 0 件）（要件 3.4・7.2）。
#[test]
fn memory_missing_falls_to_descript() {
    let root = root();
    let given = Given {
        memory: Some("gone"),
        name: Some("名前で当たる"),
        companion: Some("bundled"),
        ..Default::default()
    };
    let (got, events) = chain(&root, given, &all_hit(&root), no_pick);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, "by_name")));
    let memory = hits(&events, "last_balloon_not_found");
    assert_eq!(memory.len(), 1, "{events:?}");
    assert_eq!(memory[0].level, tracing::Level::WARN);
    assert_eq!(memory[0].field_str("memory"), Some("gone"));
    assert!(descript_misses(&root, &events).is_empty(), "{events:?}");
    assert_eq!(events.len(), 1, "{events:?}");
}

/// descript の先が根に無い: 書かれた鍵の数だけ警告（鍵・値・置き場）を出して同梱へ（要件 2.12・5.2・5.5）。
#[test]
fn descript_missing_falls_to_companion_one_warning_per_key() {
    let root = root();
    let listed = all_hit(&root);
    let cases: [(Option<&str>, Option<&str>, Vec<(String, String)>); 3] = [
        (
            Some("gone_path"),
            None,
            vec![pair("default.balloon.path", "gone_path")],
        ),
        (None, Some("gone_name"), vec![pair("balloon", "gone_name")]),
        (
            Some("gone_path"),
            Some("gone_name"),
            vec![
                pair("default.balloon.path", "gone_path"),
                pair("balloon", "gone_name"),
            ],
        ),
    ];
    for (path, name, want) in cases {
        let given = Given {
            path,
            name,
            companion: Some("bundled"),
            ..Default::default()
        };
        let (got, events) = chain(&root, given, &listed, no_pick);
        assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
        assert_eq!(descript_misses(&root, &events), want, "{events:?}");
        assert_eq!(events.len(), want.len(), "{events:?}");
    }
}

/// 同梱の最初の 1 個が根に無ければ、2 個目の名前のバルーンが在っても繰り下げず既定へ
/// （同梱の警告 1 件）（要件 1.9・1.10・5.3）。
#[test]
fn companion_first_missing_goes_to_default_not_second() {
    let root = root();
    let listed = entries(
        &root,
        &[("StayseeBalloon", None), ("claudia_vertical", None)],
    );
    let given = Given {
        companion: Some("claudia"),
        ..Default::default()
    };
    let (got, events) = chain(&root, given, &listed, no_pick);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Default, "StayseeBalloon")));
    let companion = hits(&events, "companion_balloon_not_found");
    assert_eq!(companion.len(), 1, "{events:?}");
    assert_eq!(companion[0].field_str("companion"), Some("claudia"));
    assert_eq!(events.len(), 1, "{events:?}");
}

/// 同梱の値が区切りを含む: 読み替えず（`_` への置き換えも区切りの取り除きもしない）当たらない
/// として同梱の警告 1 件・値はそのまま載る（要件 1.15・5.3）。
#[test]
fn companion_with_separator_is_not_rewritten() {
    let root = root();
    let listed = entries(
        &root,
        &[
            ("StayseeBalloon", None),
            ("mine", None),
            ("x_mine", None),
            ("xmine", None),
        ],
    );
    for value in [r"x\mine", "x/mine", "mine/"] {
        let given = Given {
            companion: Some(value),
            ..Default::default()
        };
        let (got, events) = chain(&root, given, &listed, no_pick);
        assert_eq!(got, Ok(at(&root, BalloonRoute::Default, "StayseeBalloon")));
        let companion = hits(&events, "companion_balloon_not_found");
        assert_eq!(companion.len(), 1, "{events:?}");
        assert_eq!(companion[0].field_str("companion"), Some(value));
        assert_eq!(events.len(), 1, "{events:?}");
    }
}

/// 引数が在れば、当たらない記憶・descript・同梱を渡しても見ない（記録 0 件で `Argv`）（要件 3.2）。
#[test]
fn argv_ignores_everything_else() {
    let root = root();
    let argv = PathBuf::from(r"C:\somewhere\outside\my_balloon");
    let given = Given {
        argv: Some(&argv),
        memory: Some("gone"),
        path: Some("gone_path"),
        name: Some("gone_name"),
        companion: Some("gone_companion"),
    };
    let (got, events) = chain(&root, given, &all_hit(&root), no_pick);
    assert_eq!(
        got,
        Ok(BalloonDecision {
            route: BalloonRoute::Argv,
            dir: argv.clone(),
            folder: None,
        })
    );
    assert!(events.is_empty(), "{events:?}");
}

// ---------------------------------------------------------------- descript の突き合わせ（要件 2・7.4）

/// 突き合わせの場面に使う候補の並び（フォルダ名の昇順＝`list_balloons` の並び）。
/// `shared` はフォルダ名、`zz_named` は `name` が同じ綴り（フォルダ名の側が並びで先）。
fn matching(root: &BasewareRoot) -> Vec<BalloonEntry> {
    entries(
        root,
        &[
            ("Claudia", Some("Claudia")),
            ("a_folder", Some("名前A")),
            ("bundled", None),
            ("dup1", Some("dup")),
            ("dup2", Some("dup")),
            ("folder_only", None),
            ("shared", None),
            ("x", None),
            ("zz_named", Some("shared")),
        ],
    )
}

/// descript の 2 鍵を渡し、同梱 `bundled` を後ろに置いて鎖を回す（無作為の添字は呼ばれない）。
fn descript(
    root: &BasewareRoot,
    path: Option<&str>,
    name: Option<&str>,
) -> (Result<BalloonDecision, NoBalloon>, Vec<CapturedEvent>) {
    let given = Given {
        path,
        name,
        companion: Some("bundled"),
        ..Default::default()
    };
    chain(root, given, &matching(root), no_pick)
}

/// `default.balloon.path` だけ: フォルダ名に当たれば descript（記録 0 件）、当たらなければ警告 1 件で
/// 同梱へ。`name` とは突き合わせない（`名前A` は `name` に在ってもフォルダ名に無いので当たらない）（要件 2.3・2.12）。
#[test]
fn default_balloon_path_alone_hits_folder_or_warns_once() {
    let root = root();
    let (got, events) = descript(&root, Some("folder_only"), None);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, "folder_only")));
    assert!(events.is_empty(), "{events:?}");

    for value in ["gone", "名前A"] {
        let (got, events) = descript(&root, Some(value), None);
        assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
        assert_eq!(
            descript_misses(&root, &events),
            vec![pair("default.balloon.path", value)]
        );
        assert_eq!(events.len(), 1, "{events:?}");
    }
}

/// `default.balloon.path` の値が `..`・区切り・絶対パスを含む: 読み替えず（`x` を当てない）、
/// 値そのままの警告 1 件で同梱へ（要件 2.4・5.2）。
#[test]
fn default_balloon_path_with_parent_separator_or_absolute_is_not_rewritten() {
    let root = root();
    for value in ["../balloon/x", "balloon/x", "x/", r"C:\balloon\x"] {
        let (got, events) = descript(&root, Some(value), None);
        assert_eq!(
            got,
            Ok(at(&root, BalloonRoute::Companion, "bundled")),
            "{value}"
        );
        assert_eq!(
            descript_misses(&root, &events),
            vec![pair("default.balloon.path", value)]
        );
        assert_eq!(events.len(), 1, "{events:?}");
    }
}

/// `balloon` だけ: `name` で当たる・フォルダ名で当たる（記録 0 件）・どちらにも当たらない（警告 1 件で
/// 同梱へ）（要件 2.6・2.11・2.12）。
#[test]
fn balloon_name_hits_by_name_or_folder_or_warns_once() {
    let root = root();
    for (value, want) in [("名前A", "a_folder"), ("folder_only", "folder_only")] {
        let (got, events) = descript(&root, None, Some(value));
        assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, want)), "{value}");
        assert!(events.is_empty(), "{events:?}");
    }
    let (got, events) = descript(&root, None, Some("gone"));
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert_eq!(
        descript_misses(&root, &events),
        vec![pair("balloon", "gone")]
    );
    assert_eq!(events.len(), 1, "{events:?}");
}

/// `name` と別のバルーンのフォルダ名の両方に一致 → 並びで先のフォルダ名でなく `name` の側。
/// 同じ `name` が複数 → 列挙の並びで最初（要件 2.6・2.7）。
#[test]
fn balloon_name_prefers_name_then_first_in_listing() {
    let root = root();
    for (value, want) in [("shared", "zz_named"), ("dup", "dup1")] {
        let (got, events) = descript(&root, None, Some(value));
        assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, want)), "{value}");
        assert!(events.is_empty(), "{events:?}");
    }
}

/// 大文字と小文字だけ違う: どちらの鍵も当たらず、鍵ごとに警告 1 件（要件 2.3・2.6）。
#[test]
fn descript_match_is_case_sensitive() {
    let root = root();
    let (got, events) = descript(&root, Some("claudia"), Some("claudia"));
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert_eq!(
        descript_misses(&root, &events),
        vec![
            pair("default.balloon.path", "claudia"),
            pair("balloon", "claudia"),
        ]
    );
    assert_eq!(events.len(), 2, "{events:?}");
}

/// `random`・`lastinstalled` は特別に解かない: その `name` のバルーンが在れば当たり、無ければ警告 1 件。
/// どちらも無作為の添字は呼ばれない（`no_pick`）（要件 2.8）。
#[test]
fn balloon_name_random_and_lastinstalled_are_plain_names() {
    let root = root();
    for word in ["random", "lastinstalled"] {
        let named = entries(&root, &[("a", None), ("b", Some(word)), ("bundled", None)]);
        let given = Given {
            name: Some(word),
            companion: Some("bundled"),
            ..Default::default()
        };
        let (got, events) = chain(&root, given, &named, no_pick);
        assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, "b")), "{word}");
        assert!(events.is_empty(), "{events:?}");

        let (got, events) = descript(&root, None, Some(word));
        assert_eq!(
            got,
            Ok(at(&root, BalloonRoute::Companion, "bundled")),
            "{word}"
        );
        assert_eq!(descript_misses(&root, &events), vec![pair("balloon", word)]);
        assert_eq!(events.len(), 1, "{events:?}");
    }
}

/// 両方が書かれた 3 通り: `default.balloon.path` が当たる → `balloon` が別に当たっても記録 0 件／
/// 先が外れ `balloon` が当たる → 警告 1 件／どちらも外れる → 警告 2 件で同梱へ（要件 2.9〜2.12・5.5）。
#[test]
fn both_keys_path_first_then_name() {
    let root = root();
    let (got, events) = descript(&root, Some("folder_only"), Some("名前A"));
    assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, "folder_only")));
    assert!(events.is_empty(), "{events:?}");

    let (got, events) = descript(&root, Some("gone_path"), Some("名前A"));
    assert_eq!(got, Ok(at(&root, BalloonRoute::Descript, "a_folder")));
    assert_eq!(
        descript_misses(&root, &events),
        vec![pair("default.balloon.path", "gone_path")]
    );
    assert_eq!(events.len(), 1, "{events:?}");

    let (got, events) = descript(&root, Some("gone_path"), Some("gone_name"));
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert_eq!(
        descript_misses(&root, &events),
        vec![
            pair("default.balloon.path", "gone_path"),
            pair("balloon", "gone_name"),
        ]
    );
    assert_eq!(events.len(), 2, "{events:?}");
}

/// どちらも書かれていない: descript の段は記録 0 件で同梱へ（要件 2.13・5.6）。
#[test]
fn no_descript_keys_is_silent() {
    let root = root();
    let (got, events) = descript(&root, None, None);
    assert_eq!(got, Ok(at(&root, BalloonRoute::Companion, "bundled")));
    assert!(events.is_empty(), "{events:?}");
}

// ---------------------------------------------------------------- 実行中の決め方との一致（要件 2.6・4.2）

/// 同じ一覧と同じ名前で、鎖の descript の段（`balloon`）と、実行中のバルーンの切替の決め方
/// （`resolve_skin_target` に `SkinSpec::Name`）が同じフォルダを選ぶ（当たらないときはどちらも外れ）。
/// 候補の列は、鎖へ渡す一覧 1 つから本番の `balloon_candidates` と同じ写し方（1 件 → 1 件）で作る
/// （本番の候補づくりは根を列挙する I/O を持つので呼ばない）。
#[test]
fn descript_balloon_agrees_with_runtime_switch() {
    use crate::emo2_boot::shell_balloon_resolve::{
        NotFoundReason, SkinCandidate, resolve_skin_target,
    };
    use crate::emo2_boot::shell_balloon_switch::SkinSpec;

    let root = root();
    let listed = matching(&root);
    let candidates: Vec<SkinCandidate> = listed
        .iter()
        .map(|e| SkinCandidate {
            dir: e.dir.clone(),
            folder: e.identity.folder.clone(),
            name: e.identity.name.clone(),
            hidden: false,
        })
        .collect();
    for (value, want) in [
        ("名前A", Some("a_folder")),
        ("folder_only", Some("folder_only")),
        ("shared", Some("zz_named")),
        ("dup", Some("dup1")),
        ("gone", None),
    ] {
        let given = Given {
            name: Some(value),
            companion: Some("bundled"),
            ..Default::default()
        };
        let (chained, _) = chain(&root, given, &listed, no_pick);
        let chained = chained
            .ok()
            .filter(|d| d.route == BalloonRoute::Descript)
            .and_then(|d| d.folder);
        let runtime = match resolve_skin_target(
            &candidates,
            &SkinSpec::Name(value.to_owned()),
            None,
            Err(NotFoundReason::LastInstalledNone),
            no_pick,
        ) {
            Ok(c) => Some(c.folder),
            Err(NotFoundReason::NoMatch) => None,
            Err(other) => panic!("{value}: 予期しない外れ {other:?}"),
        };
        assert_eq!(chained.as_deref(), want, "鎖: {value}");
        assert_eq!(runtime, chained, "実行中と鎖が食い違う: {value}");
    }
}
