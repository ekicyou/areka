//! 切替先の名前の解決とインストールの控えの決定論テスト（spec: areka-P0-shell-balloon-switch
//! 要件 1.6〜1.11・1.13・12.3〜12.5）。
//!
//! 確かめること: `name` → フォルダ名 → 該当なしの順・大文字小文字の区別・隠しシェルの名指し・
//! 今のものへの切替・`random` の注入の乱数と候補 0・シェルの `lastinstalled` 4 通り（ゴースト切替の
//! 要求 0）・バルーンの `lastinstalled`。目録は一時フォルダに実物の `descript.txt` を置いて読む。

use std::fs;
use std::path::Path;

use areka_ghost::BasewareRoot;
use bevy_ecs::world::World;
use log_capture_kit::{CapturedEvent, capture};
use temp_path_kit::TempPath;
use tracing::Level;

use super::*;
use crate::emo2_boot::ghost_switch::SwitchInFlight;

fn cand(folder: &str, name: Option<&str>, hidden: bool) -> SkinCandidate {
    SkinCandidate {
        dir: PathBuf::from(folder),
        folder: folder.to_owned(),
        name: name.map(str::to_owned),
        hidden,
    }
}

fn by_name(name: &str) -> SkinSpec {
    SkinSpec::Name(name.to_owned())
}

fn never(_: usize) -> usize {
    panic!("乱数は引かない")
}

const NO_MEMO: Result<&str, NotFoundReason> = Err(NotFoundReason::LastInstalledNone);

/// 解いた先のフォルダ名（失敗は理由のまま）。
fn folder(result: Result<SkinCandidate, NotFoundReason>) -> Result<String, NotFoundReason> {
    result.map(|c| c.folder)
}

fn write_descript(dir: &Path, body: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("descript.txt"),
        format!("charset,UTF-8\r\n{body}\r\n"),
    )
    .unwrap();
}

/// 根と、シェル `master`（通常）・`summer`（通常）・`secret`（`menu,hidden`）を持つゴースト `A`。
fn plant_ghost() -> (TempPath, BasewareRoot) {
    let temp = TempPath::new("areka-skin-resolve");
    let root = BasewareRoot::new(temp.path().to_path_buf());
    let shell = root.ghost_dir("A").join("shell");
    write_descript(&shell.join("master"), "name,通常");
    write_descript(&shell.join("summer"), "name,夏服");
    write_descript(&shell.join("secret"), "name,ひみつ\r\nmenu,hidden");
    write_descript(&root.balloon_dir("kaku"), "name,かくかく");
    write_descript(&root.balloon_dir("fluffy"), "name,ふわふわ");
    (temp, root)
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

/// 比較は大文字小文字を区別する（要件 1.6）。
#[test]
fn matching_is_case_sensitive() {
    let cands = [cand("Plain", None, false), cand("x", Some("Beta"), false)];
    let resolve = |spec: &SkinSpec| folder(resolve_skin_target(&cands, spec, None, NO_MEMO, never));
    assert_eq!(resolve(&by_name("beta")), Err(NotFoundReason::NoMatch));
    assert_eq!(resolve(&by_name("plain")), Err(NotFoundReason::NoMatch));
    assert_eq!(
        resolve(&SkinSpec::Folder("PLAIN".to_owned())),
        Err(NotFoundReason::NoMatch)
    );
    assert_eq!(resolve(&by_name("Plain")).as_deref(), Ok("Plain"));
}

/// シェルの候補は隠しを含み（印つき）、隠しシェルも名指しなら切り替えられる。今のものも候補（要件 1.6・1.8）。
#[test]
fn hidden_shell_by_name_and_the_current_one_are_candidates() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");
    let cands = shell_candidates(&ghost);
    let mut listed: Vec<(&str, bool)> = cands
        .iter()
        .map(|c| (c.folder.as_str(), c.hidden))
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        vec![("master", false), ("secret", true), ("summer", false)]
    );

    let hit = resolve_skin_target(&cands, &by_name("ひみつ"), Some("master"), NO_MEMO, never)
        .expect("隠しシェルも名指しなら解ける");
    assert_eq!(hit.folder, "secret");
    assert_eq!(hit.dir, ghost.join("shell").join("secret"));
    assert_eq!(hit.name.as_deref(), Some("ひみつ"));

    let itself = resolve_skin_target(&cands, &by_name("通常"), Some("master"), NO_MEMO, never);
    assert_eq!(
        folder(itself).as_deref(),
        Ok("master"),
        "今のものへも切り替える"
    );

    let balloons = balloon_candidates(&root);
    let hit = resolve_skin_target(
        &balloons,
        &by_name("かくかく"),
        Some("kaku"),
        NO_MEMO,
        never,
    );
    assert_eq!(
        folder(hit).as_deref(),
        Ok("kaku"),
        "バルーンも今のものが候補"
    );
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

/// 控えを置いた World で、シェルの `lastinstalled` を入口と同じ順（控え → 解決）で解く。
fn shell_lastinstalled(world: &World, ghost: &Path) -> Result<String, NotFoundReason> {
    let installed = installed_for(world, SkinKind::Shell, ghost);
    folder(resolve_skin_target(
        &shell_candidates(ghost),
        &by_name("lastinstalled"),
        Some("master"),
        installed.as_deref().map_err(|r| *r),
        never,
    ))
}

/// シェルの `lastinstalled` の 4 通り。控えのゴーストが今のゴースト（大文字小文字は問わない）で先が
/// 在るときだけ解け、どの場合もゴースト切替の要求は 0（要件 1.10・1.13・裁定 3）。
#[test]
fn shell_lastinstalled_resolves_only_within_the_current_ghost() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");

    let mut world = World::new();
    assert_eq!(
        shell_lastinstalled(&world, &ghost),
        Err(NotFoundReason::LastInstalledNone),
        "控えなし"
    );

    record_installed_shell(&mut world, Some("a".to_owned()), "summer".to_owned());
    assert_eq!(
        shell_lastinstalled(&world, &ghost).as_deref(),
        Ok("summer"),
        "控えあり"
    );

    record_installed_shell(&mut world, Some("B".to_owned()), "summer".to_owned());
    assert_eq!(
        shell_lastinstalled(&world, &ghost),
        Err(NotFoundReason::LastInstalledOtherGhost),
        "別のゴースト"
    );

    record_installed_shell(&mut world, Some("A".to_owned()), "winter".to_owned());
    assert_eq!(
        shell_lastinstalled(&world, &ghost),
        Err(NotFoundReason::LastInstalledMissing),
        "先が無い"
    );

    assert!(
        world.get_non_send::<SwitchInFlight>().is_none(),
        "ゴースト切替の要求は 0"
    );
    assert_eq!(
        world.get_resource::<LastInstalledShell>(),
        Some(&LastInstalledShell {
            ghost_folder: Some("A".to_owned()),
            folder: "winter".to_owned(),
        }),
        "控えは最後の 1 件"
    );
}

/// バルーンの `lastinstalled`: 控えが目録に在るときだけ解ける（要件 1.11）。
#[test]
fn balloon_lastinstalled_resolves_only_when_in_the_catalog() {
    let (_temp, root) = plant_ghost();
    let ghost = root.ghost_dir("A");
    let resolve = |world: &World| {
        let installed = installed_for(world, SkinKind::Balloon, &ghost);
        folder(resolve_skin_target(
            &balloon_candidates(&root),
            &by_name("lastinstalled"),
            Some("kaku"),
            installed.as_deref().map_err(|r| *r),
            never,
        ))
    };
    let mut world = World::new();
    assert_eq!(resolve(&world), Err(NotFoundReason::LastInstalledNone));

    record_installed_balloon(&mut world, "fluffy".to_owned());
    assert_eq!(resolve(&world).as_deref(), Ok("fluffy"));

    record_installed_balloon(&mut world, "gone".to_owned());
    assert_eq!(resolve(&world), Err(NotFoundReason::LastInstalledMissing));
}
