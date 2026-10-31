//! 起動時のシェル（spec: areka-P0-shell-balloon-switch 要件 6.2〜6.4・6.6・11.7）。
//!
//! 偽の SHIORI を持つゴースト A（`ghost_switch_test_support` の土台）に 2 つ目のシェルを写し、
//! 記憶の名を置いてから、本番と同じ順（配置の準備 [`prepare_ghost_windows`] → 起動
//! [`super::boot_ghost`]）で起こす。シェルを決めるのは準備の 1 回だけで、その値が
//! 解決の 4 か所（配置の情報源・資産・実行系のマウント・`OnBoot` の Ref0）に届くことを判定する。
//! 資産のシェルは、シェル読み込みの権威が資産の組み立てで 1 回だけ残す `info!`（`shell_dir`）で、
//! 採寸のシェルは準備が置いたシェルの選択（[`BootShellChoice`]）で見る（採寸は記録を出さない入口を
//! 通る・spec: placement-measure-bake-once D5）。後半は起動と切り替えの通しで、読み込みの記録の行の
//! 数を固定する（同 要件 1.3・3.1〜3.3・3.5・3.7・4.4）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use areka_ghost::sylphya_wiring::profile_areka_root;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::resolve_with_shell;
use areka_sylphya::persist::FsPersistIo;
use areka_sylphya::{PersistKey, PersistScope, ScopeRoots, save_scope};
use log_capture_kit::{CapturedEvent, capture};
use wintf::ecs::widget::bitmap_source::WintfTaskPool;

use super::{BootShellChoice, GhostSlot, prepare_ghost_windows};
use crate::boot_resolve::read_last_shell;
use crate::emo2_boot::ghost_switch::SwitchInFlight;
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
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
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
    add_hidden_shell_from(&default_shell_dir(ghost_dir), ghost_dir, folder, name);
}

/// [`add_hidden_shell`] の、写すシェルを `src` に決めた形。
fn add_hidden_shell_from(src: &Path, ghost_dir: &Path, folder: &str, name: &str) {
    let to = ghost_dir.join("shell").join(folder);
    let mut stack = vec![(src.to_path_buf(), to.clone())];
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

/// シェル読み込みの権威が記録を残して読んだシェルのフォルダの列（資産の組み立ての 1 回だけ・
/// 採寸は記録を出さない入口を通る）。
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

/// 準備が置いた採寸のシェルの選択（ゴーストの根・シェルのフォルダ名）。起こす処理が取り出すので、
/// 準備の直後に読む。
fn measuring_shell(rig: &SwitchRig) -> Option<(PathBuf, Option<String>)> {
    rig.world
        .get_resource::<BootShellChoice>()
        .map(|c| (c.ghost_root.clone(), c.shell.clone()))
}

/// 読み込みの記録の行の数（宛先と文言は `areka_emo_present` が出すとおり）。scope ごとの欄は
/// scope の値 → 行の数。
#[derive(Debug, PartialEq, Eq)]
struct LoadLines {
    /// シェルの面の画像の一覧の `info!`。
    shell_list: usize,
    /// シェルの透過の宣言の `info!`。
    shell_alpha: usize,
    /// バルーンの透過の宣言の `info!`。
    balloon_alpha: usize,
    /// バルーンの系列の解決が完了した `info!`。
    series: BTreeMap<String, usize>,
    /// バルーン定義の 2 層マージが確定した `info!`。
    merge: BTreeMap<String, usize>,
}

impl LoadLines {
    /// 起動（読み込み直し）`n` 回ぶんの期待（emo2 の scope 0・1）。
    fn per_boot(n: usize) -> Self {
        let scopes = |k: usize| BTreeMap::from([("0".to_owned(), k), ("1".to_owned(), k)]);
        Self {
            shell_list: n,
            shell_alpha: n,
            balloon_alpha: n,
            series: scopes(n),
            merge: scopes(2 * n),
        }
    }
}

/// `events` の読み込みの記録の行を数える。
fn load_lines(events: &[CapturedEvent]) -> LoadLines {
    let is = |e: &CapturedEvent, target: &str, message: &str| {
        e.level == tracing::Level::INFO && e.target == target && e.message() == message
    };
    let count = |target: &str, message: &str, kind: Option<&str>| {
        events
            .iter()
            .filter(|e| {
                is(e, target, message) && kind.is_none_or(|k| e.field_str("kind") == Some(k))
            })
            .count()
    };
    let per_scope = |target: &str, message: &str| {
        let mut out = BTreeMap::new();
        for e in events.iter().filter(|e| is(e, target, message)) {
            *out.entry(e.field("scope").unwrap_or_default().to_owned())
                .or_insert(0) += 1;
        }
        out
    };
    const SHELL: &str = "areka_emo_present::shell_target";
    const ALPHA: &str = "areka_emo_present::self_alpha";
    const BALLOON: &str = "areka_emo_present::balloon";
    const ALPHA_MSG: &str = "self_alpha: 透過の扱いを決めた";
    LoadLines {
        shell_list: count(
            SHELL,
            "shell: シェルの面の画像の一覧が終わった（R6.1）",
            None,
        ),
        shell_alpha: count(ALPHA, ALPHA_MSG, Some("shell")),
        balloon_alpha: count(ALPHA, ALPHA_MSG, Some("balloon")),
        series: per_scope(BALLOON, "balloon: scope 別バルーン系列の解決が完了"),
        merge: per_scope(BALLOON, "balloon: scope 別バルーン定義の 2 層マージが確定"),
    }
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

    let ((prepared, measured), events) = capture(|| {
        let prepared = prepare_ghost_windows(&mut rig.world, &cfg).is_ok();
        let measured = measuring_shell(&rig);
        remember_shell(&ghost_dir, "gone");
        rig.boot("A");
        (prepared, measured)
    });
    let mounted = mounted_shell(&rig);
    let ref0 = on_boot_ref0(&mut rig);
    let shutdown_ok = rig.shutdown();

    let second = ghost_dir.join("shell").join(SECOND);
    assert_eq!(
        (
            prepared,
            measured,
            loaded_shell_dirs(&events),
            mounted,
            ref0,
            count_event(&events, "boot_shell_missing"),
            shutdown_ok,
        ),
        (
            true,
            Some((ghost_dir.clone(), Some(SECOND.to_owned()))),
            vec![second.clone()],
            Some(second),
            Some(SECOND_NAME.to_owned()),
            0,
            true,
        ),
        "記憶のシェルが 4 か所にそろわない（準備が通った・採寸のシェルの選択・資産が読んだシェル・\
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

    let ((prepared, measured), events) = capture(|| {
        let prepared = prepare_ghost_windows(&mut rig.world, &cfg).is_ok();
        let measured = measuring_shell(&rig);
        rig.boot("A");
        (prepared, measured)
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
            measured,
            loaded_shell_dirs(&events),
            mounted,
            count_event(&events, "boot_shell_missing"),
            shutdown_ok,
            remembered,
        ),
        (
            true,
            Some((ghost_dir.clone(), None)),
            vec![default.clone()],
            Some(default),
            1,
            true,
            default_folder,
        ),
        "記憶の先が無い起動が既定へそろわない（準備が通った・採寸のシェルの選択・資産が読んだシェル・\
         マウントのシェル・前回のシェルが無い警告の件数・降ろせた・書き直った記憶）"
    );
}

/// 本番の順（配置の準備 → 起こす処理）で A を起こし、準備が通ったかと、その間の記録を返す。
fn boot_a_in_production_order(rig: &mut SwitchRig) -> (bool, Vec<CapturedEvent>) {
    let cfg = rig.cfg("A");
    capture(|| {
        let prepared = prepare_ghost_windows(&mut rig.world, &cfg).is_ok();
        rig.boot("A");
        prepared
    })
}

/// emo2 の写しを本番の順で起こすと、シェルの一覧・シェルとバルーンの透過の宣言が 1 行ずつ、
/// 系列の解決の完了が scope ごとに 1 行、2 層マージの確定が scope ごとに 2 行（今のまま）出る
/// （spec: placement-measure-bake-once 要件 3.1・3.2・3.7・4.4）。
///
/// # 非空虚性
/// 採寸か windowposition の読みを記録を出す入口へ戻すと、一覧か系列の解決が 2 行になって赤。
#[test]
fn production_boot_emits_each_load_record_once() {
    let mut rig = rig_a();
    let (prepared, events) = boot_a_in_production_order(&mut rig);
    let mounted = mounted_shell(&rig).is_some();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (prepared, load_lines(&events), mounted, shutdown_ok),
        (true, LoadLines::per_boot(1), true, true),
        "起動 1 回の読み込みの記録の行の数がずれる（準備が通った・記録の行の数・マウントがある・降ろせた）"
    );
}

/// 動く絵の検体のシェルを隠しシェルとして写し、記憶の名で起こしても、記録の行の数は
/// [`production_boot_emits_each_load_record_once`] と同じで、起動が通る（要件 1.3 の起動の側・4.4）。
#[test]
fn animated_hidden_shell_boot_emits_each_load_record_once() {
    const ANIMATED: &str = "animated";
    const ANIMATED_NAME: &str = "AnimatedShell";
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../areka-emo-compose/tests/fixtures/animated-playback/shell");
    let mut rig = rig_a();
    let ghost_dir = rig.root.ghost_dir("A");
    add_hidden_shell_from(&fixture, &ghost_dir, ANIMATED, ANIMATED_NAME);
    remember_shell(&ghost_dir, ANIMATED);

    let (prepared, events) = boot_a_in_production_order(&mut rig);
    let mounted = mounted_shell(&rig);
    let ref0 = on_boot_ref0(&mut rig);
    let shutdown_ok = rig.shutdown();

    let animated = ghost_dir.join("shell").join(ANIMATED);
    assert_eq!(
        (
            prepared,
            load_lines(&events),
            loaded_shell_dirs(&events),
            mounted,
            ref0,
            shutdown_ok,
        ),
        (
            true,
            LoadLines::per_boot(1),
            vec![animated.clone()],
            Some(animated),
            Some(ANIMATED_NAME.to_owned()),
            true,
        ),
        "動く絵のシェルの起動の記録の行の数がずれるか起動が通らない（準備が通った・記録の行の数・\
         資産が読んだシェル・マウントのシェル・OnBoot の Ref0・降ろせた）"
    );
}

/// A を本番の順で起こし、台本の `\![change,ghost,B,…]` で B へ切り替えると、切り替え 1 回につき
/// 起動と同じ数の記録がもう 1 組出る（要件 1.3・3.3）。
#[test]
fn each_switch_adds_one_more_set_of_load_records() {
    let mut rig = SwitchRig::new(vec![
        (
            "A",
            FakeShiori::Scripted(Box::new(|| {
                standard_script("\\0A\\![change,ghost,B,--option=raise-event]\\e")
                    .get("OnGhostChanging", Ok(Some("\\0Bへ\\e".to_owned())))
            })),
        ),
        (
            "B",
            FakeShiori::Scripted(Box::new(|| {
                standard_script("\\0B\\e").get("OnGhostChanged", Ok(None))
            })),
        ),
    ]);
    rig.world.insert_resource(WintfTaskPool::with_threads(1));
    rig.plant_boot_record("A");
    rig.plant_boot_record("B");

    let (prepared, booted) = boot_a_in_production_order(&mut rig);
    let steady = rig.wait_steady();
    let (welcomed, switched) = capture(|| {
        rig.pump_talking_until(|rig| {
            rig.exit_requested()
                || (!rig.calls("B").is_empty()
                    && rig.world.get_non_send::<SwitchInFlight>().is_none())
        })
    });
    let exit_requested = rig.exit_requested();
    let shutdown_ok = rig.shutdown();

    assert_eq!(
        (
            prepared,
            steady,
            welcomed,
            exit_requested,
            load_lines(&booted),
            load_lines(&switched),
            shutdown_ok,
        ),
        (
            true,
            true,
            true,
            false,
            LoadLines::per_boot(1),
            LoadLines::per_boot(1),
            true,
        ),
        "切り替えの記録の行の数がずれる（準備が通った・A の定常・B の定常・終了の指示・起動の記録・\
         切り替えの記録・降ろせた）"
    );
}
