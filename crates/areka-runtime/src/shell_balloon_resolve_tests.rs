//! 切替先の名前の解決とインストールの控えの決定論テスト（spec: areka-P0-shell-balloon-switch
//! 要件 1.6〜1.11・1.13・12.3〜12.5）。
//!
//! 確かめること: `name` → フォルダ名 → 該当なしの順・大文字小文字の区別・隠しシェルの名指し・
//! 今のものへの切替・`random` の注入の乱数と候補 0・シェルの `lastinstalled` 4 通り（ゴースト切替の
//! 要求 0）・バルーンの `lastinstalled`。目録は一時フォルダに実物の `descript.txt` を置いて読む。

use log_capture_kit::{CapturedEvent, capture};
use tracing::Level;

use super::test_support::{NO_MEMO, by_name, folder, never};
use super::*;

fn cand(folder: &str, name: Option<&str>, hidden: bool) -> SkinCandidate {
    SkinCandidate {
        dir: PathBuf::from(folder),
        folder: folder.to_owned(),
        name: name.map(str::to_owned),
        hidden,
    }
}

fn infos(events: &[CapturedEvent], name: &str) -> Vec<CapturedEvent> {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(name) && e.level == Level::INFO)
        .cloned()
        .collect()
}

/// 名前は `name` → フォルダ名の順。フォルダ名の指し方はフォルダ名とだけ突き合わせる（要件 1.6・1.7）。
#[test]
fn name_then_folder_then_not_found() {
    let cands = [
        cand("first", Some("Beta"), false),
        cand("Beta", Some("Gamma"), false),
        cand("plain", None, false),
    ];
    let resolve = |spec: &SkinSpec| folder(resolve_skin_target(&cands, spec, None, NO_MEMO, never));
    assert_eq!(
        resolve(&by_name("Beta")).as_deref(),
        Ok("first"),
        "別の候補のフォルダ名より name が先"
    );
    assert_eq!(resolve(&by_name("Gamma")).as_deref(), Ok("Beta"));
    assert_eq!(
        resolve(&by_name("plain")).as_deref(),
        Ok("plain"),
        "name が無ければフォルダ名"
    );
    assert_eq!(resolve(&by_name("none")), Err(NotFoundReason::NoMatch));
    assert_eq!(
        resolve(&SkinSpec::Folder("Beta".to_owned())).as_deref(),
        Ok("Beta"),
        "フォルダの名指しは name を見ない"
    );
    assert_eq!(
        resolve(&SkinSpec::Folder("Gamma".to_owned())),
        Err(NotFoundReason::NoMatch)
    );
}

/// `name` は大文字小文字を区別し、フォルダ名は 1 つの規則で区別しない（要件 1.6・2.8）。
#[test]
fn name_is_case_sensitive_folder_name_is_not() {
    let cands = [cand("Plain", None, false), cand("x", Some("Beta"), false)];
    let resolve = |spec: &SkinSpec| folder(resolve_skin_target(&cands, spec, None, NO_MEMO, never));
    assert_eq!(resolve(&by_name("beta")), Err(NotFoundReason::NoMatch));
    assert_eq!(resolve(&by_name("plain")).as_deref(), Ok("Plain"));
    assert_eq!(
        resolve(&SkinSpec::Folder("PLAIN".to_owned())).as_deref(),
        Ok("Plain")
    );
    assert_eq!(resolve(&by_name("Plain")).as_deref(), Ok("Plain"));
}

/// `random` は隠しと今のものを除いた候補から注入の乱数で 1 つ選び、`info!` を 1 件。
/// 同名の候補より先に解く（要件 1.9・裁定 4）。
#[test]
fn random_picks_among_visible_others_with_the_injected_pick() {
    let cands = [
        cand("cur", None, false),
        cand("a", None, false),
        cand("h", None, true),
        cand("random", None, false),
    ];
    let mut seen = None;
    let (result, events) = capture(|| {
        resolve_skin_target(&cands, &by_name("random"), Some("cur"), NO_MEMO, |n| {
            seen = Some(n);
            1
        })
    });
    assert_eq!(
        seen,
        Some(2),
        "候補は a と random の 2 つ（隠しと今のものは除く）"
    );
    assert_eq!(folder(result).as_deref(), Ok("random"));
    let picked = infos(&events, "skin_switch_random_pick");
    assert_eq!(picked.len(), 1, "{events:?}");
    assert_eq!(picked[0].field("to"), Some("random"));

    let (result, _) =
        capture(|| resolve_skin_target(&cands, &by_name("random"), Some("cur"), NO_MEMO, |_| 0));
    assert_eq!(folder(result).as_deref(), Ok("a"));
}

/// 候補 0 なら今のもの（`info!` 1 件・乱数は引かない）。今のものも目録に無ければ該当なし。
#[test]
fn random_with_no_other_candidate_is_the_current_one() {
    let cands = [cand("cur", None, false), cand("h", None, true)];
    let (result, events) =
        capture(|| resolve_skin_target(&cands, &by_name("random"), Some("cur"), NO_MEMO, never));
    assert_eq!(folder(result).as_deref(), Ok("cur"));
    let picked = infos(&events, "skin_switch_random_pick");
    assert_eq!(picked.len(), 1, "{events:?}");
    assert_eq!(picked[0].field("to"), Some("cur"));

    let only_hidden = [cand("h", None, true)];
    assert_eq!(
        resolve_skin_target(
            &only_hidden,
            &by_name("random"),
            Some("gone"),
            NO_MEMO,
            never
        ),
        Err(NotFoundReason::RandomEmpty)
    );
    assert_eq!(
        folder(resolve_skin_target(
            &[cand("random", None, false)],
            &SkinSpec::Folder("random".to_owned()),
            Some("cur"),
            NO_MEMO,
            never,
        ))
        .as_deref(),
        Ok("random"),
        "フォルダの名指しは特別な名前を解かない"
    );
}

/// 特別な名前を解かない「名前 → フォルダ名」の突き合わせ（`--dump-shell` が使う・要件 2.4）。
#[test]
fn find_by_name_or_folder_ignores_special_names() {
    let cands = [
        cand("alpha", Some("beta"), false),
        cand("beta", Some("ガンマ"), false),
        cand("random", None, false),
        cand("other", Some("lastinstalled"), false),
    ];
    let find = |value: &str| find_by_name_or_folder(&cands, value).map(|c| c.folder.as_str());
    assert_eq!(find("ガンマ"), Some("beta"), "名前で見つかる");
    assert_eq!(find("alpha"), Some("alpha"), "フォルダ名で見つかる");
    assert_eq!(
        find("beta"),
        Some("alpha"),
        "名前を先に見る（フォルダ名 beta より名前 beta）"
    );
    assert_eq!(
        find("random"),
        Some("random"),
        "random はただのフォルダ名として探す"
    );
    assert_eq!(
        find("lastinstalled"),
        Some("other"),
        "lastinstalled はただの名前として探す"
    );
    assert_eq!(find("none"), None, "見つからない");
}
