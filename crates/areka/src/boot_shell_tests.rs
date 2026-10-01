//! 起動時のシェル（spec: areka-P0-shell-balloon-switch 要件 6.2〜6.4・6.6・11.7）。
//!
//! 偽の SHIORI を持つゴースト A（`ghost_switch_test_support` の土台）に 2 つ目のシェルを写し、
//! 記憶の名を置いてから、本番と同じ順（配置の準備 [`prepare_ghost_windows`] → 起動
//! [`super::boot_ghost`]）で起こす。シェルを決めるのは準備の 1 回だけで、その値が
//! 解決の 4 か所（配置の情報源・資産・実行系のマウント・`OnBoot` の Ref0）に届くことを判定する。
//! 配置と資産のシェルは、シェル読み込みの権威が 1 回ごとに残す `info!`（`shell_dir`）で見る
//! （採寸も資産づくりも同じ入口を通る）。

use std::path::{Path, PathBuf};

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::resolve_with_shell;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, save_scope};
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{GhostSlot, prepare_ghost_windows};
use crate::boot_resolve::read_last_shell;
use crate::emo2_boot::ghost_switch_test_support::{FakeShiori, SwitchRig, standard_script};
use crate::emo2_boot::spine::RecordedCall;

/// 写した 2 つ目のシェルのフォルダ名。
const SECOND: &str = "second";
/// 写した 2 つ目のシェルの `name`（`OnBoot` の Ref0 に載る）。
const SECOND_NAME: &str = "SecondShell";

/// A だけを持つ土台（作業プールつき＝配置の準備が通る）。
fn rig_a() -> SwitchRig {
    let mut rig = SwitchRig::new(vec![(
        "A",
        FakeShiori::Scripted(Box::new(|| standard_script("\\0A\\e"))),
    )]);
    rig.world.insert_resource(WintfTaskPool::new());
    rig
}

/// 今日の規則のシェル（名前なしの解決）のフォルダ。
fn default_shell_dir(ghost_dir: &Path) -> PathBuf {
    resolve_with_shell(ghost_dir, DefaultEncoding::Ansi, None)
        .expect("検体の既定のシェルは解決できる")
        .shell
        .dir
}

/// 既定のシェルを `shell/<folder>/` へ写し、`name` を差し替えて `menu,hidden` を足す
/// （隠しシェルでも記憶の名なら起きる＝要件 6.2）。
fn add_hidden_shell(ghost_dir: &Path, folder: &str, name: &str) {
    let to = ghost_dir.join("shell").join(folder);
    let mut stack = vec![(default_shell_dir(ghost_dir), to.clone())];
    while let Some((src, dst)) = stack.pop() {
        std::fs::create_dir_all(&dst).expect("写す先のフォルダを作る");
        for entry in std::fs::read_dir(&src).expect("シェルを走査する") {
            let entry = entry.expect("シェルの要素");
            let target = dst.join(entry.file_name());
            if entry.file_type().expect("要素の種別").is_dir() {
                stack.push((entry.path(), target));
            } else {
                std::fs::copy(entry.path(), &target).expect("シェルのファイルを写す");
            }
        }
    }
    let descript = to.join("descript.txt");
    let bytes = std::fs::read(&descript).expect("写した descript.txt を読む");
    let mut out = bytes
        .split(|b| *b == b'\n')
        .filter(|line| !line.starts_with(b"name,"))
        .collect::<Vec<_>>()
        .join(&b'\n');
    out.extend_from_slice(format!("\r\nname,{name}\r\nmenu,hidden\r\n").as_bytes());
    std::fs::write(&descript, out).expect("写した descript.txt を書く");
}

/// ゴーストの Ghost スコープへ「最後のシェル」を直接書く（起動が読む置き場と同じ）。
fn remember_shell(ghost_dir: &Path, folder: &str) {
    save_scope(
        PersistScope::Ghost,
        &ScopeRoots {
            ghost: Some(profile_areka_root(&ghost_dir.join("ghost").join("master"))),
            ..ScopeRoots::default()
        },
        &FsPersistIo,
        vec![(PersistKey::LastShell, folder.to_owned())],
    );
}

/// シェル読み込みの権威が読んだシェルのフォルダの列（配置の採寸と資産づくりの各 1 回）。
fn loaded_shell_dirs(events: &[CapturedEvent]) -> Vec<PathBuf> {
    events
        .iter()
        .filter(|e| e.field("recognized").is_some())
        .filter_map(|e| e.field("shell_dir").map(PathBuf::from))
        .collect()
}

/// `event` の記録の件数。
fn count_event(events: &[CapturedEvent], event: &str) -> usize {
    events
        .iter()
        .filter(|e| e.field_str("event") == Some(event))
        .count()
}

/// 置き場のゴーストのマウントのシェル。
fn mounted_shell(rig: &SwitchRig) -> Option<PathBuf> {
    rig.world
        .non_send::<GhostSlot>()
        .0
        .as_ref()
        .and_then(|session| session.runtime())
        .map(|runtime| runtime.mount().shell.dir.clone())
}

/// A の最後の起動の `OnBoot` の Ref0（`OnBoot` が届くまで有界に回す）。
fn on_boot_ref0(rig: &mut SwitchRig) -> Option<String> {
    let find = |rig: &SwitchRig| {
        rig.calls("A").last().and_then(|calls| {
            calls.iter().find_map(|c| match c {
                RecordedCall::Get { id, references } if id == "OnBoot" => {
                    Some(references.first().cloned().unwrap_or_default())
                }
                _ => None,
            })
        })
    };
    rig.pump_until(|rig| find(rig).is_some());
    find(rig)
}

/// 記憶の名の隠しシェルで起きると、配置の情報源・資産・実行系のマウント・`OnBoot` の Ref0 が
/// どれもそのシェルを指す。決めるのは配置の準備の 1 回だけ: 準備の後に記憶を壊しても、
/// 起こす処理は準備が置いた値を使う（決め直すと既定へ倒れて `warn!` が出る）。
#[test]
fn remembered_hidden_shell_reaches_all_resolutions_decided_once() {
    let mut rig = rig_a();
    let ghost_dir = rig.root.ghost_dir("A");
    add_hidden_shell(&ghost_dir, SECOND, SECOND_NAME);
    remember_shell(&ghost_dir, SECOND);
    let cfg = rig.cfg("A");

    let (prepared, events) = capture(|| {
        let prepared = prepare_ghost_windows(&mut rig.world, &cfg).is_ok();
        remember_shell(&ghost_dir, "gone");
        rig.boot("A");
        prepared
    });
    let mounted = mounted_shell(&rig);
    let ref0 = on_boot_ref0(&mut rig);
    let shutdown_ok = rig.shutdown();

    let second = ghost_dir.join("shell").join(SECOND);
    assert_eq!(
        (
            prepared,
            loaded_shell_dirs(&events),
            mounted,
            ref0,
            count_event(&events, "boot_shell_missing"),
            shutdown_ok,
        ),
        (
            true,
            vec![second.clone(), second.clone()],
            Some(second),
            Some(SECOND_NAME.to_owned()),
            0,
            true,
        ),
        "記憶のシェルが 4 か所にそろわない（準備が通った・配置の採寸と資産が読んだシェル・\
         マウントのシェル・OnBoot の Ref0・前回のシェルが無い警告の件数・降ろせた）"
    );
}

/// 記憶の先が無ければ `warn!` 1 件（決めるのは 1 回）で既定のシェルで起き、起動の成功で
/// 記憶が既定のシェルへ書き直る（次の起動から警告が出ない）。
#[test]
fn missing_remembered_shell_boots_default_and_rewrites_memory() {
    let mut rig = rig_a();
    let ghost_dir = rig.root.ghost_dir("A");
    remember_shell(&ghost_dir, "gone");
    let cfg = rig.cfg("A");
    let default = default_shell_dir(&ghost_dir);

    let (prepared, events) = capture(|| {
        let prepared = prepare_ghost_windows(&mut rig.world, &cfg).is_ok();
        rig.boot("A");
        prepared
    });
    let mounted = mounted_shell(&rig);
    let shutdown_ok = rig.shutdown();
    let remembered = read_last_shell(&ghost_dir);

    let default_folder = default
        .file_name()
        .map(|n| n.to_string_lossy().into_owned());
    assert_eq!(
        (
            prepared,
            loaded_shell_dirs(&events),
            mounted,
            count_event(&events, "boot_shell_missing"),
            shutdown_ok,
            remembered,
        ),
        (
            true,
            vec![default.clone(), default.clone()],
            Some(default),
            1,
            true,
            default_folder,
        ),
        "記憶の先が無い起動が既定へそろわない（準備が通った・配置の採寸と資産が読んだシェル・\
         マウントのシェル・前回のシェルが無い警告の件数・降ろせた・書き直った記憶）"
    );
}
