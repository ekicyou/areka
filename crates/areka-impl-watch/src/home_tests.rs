//! 置き場所の解決の決定論テスト。値は引数で渡し、プロセスの環境変数は触らない
//! （テストは並んで走る）。向け先はワークツリーの `target\` の下の一時フォルダだけ。

use std::ffi::OsString;
#[cfg(windows)]
use std::path::Component;
use std::path::{Path, PathBuf};

use temp_path_kit::TempPath;

use super::{Home, resolve};
use crate::error::WatchError;
use crate::state::WaitKind;

/// フォルダの直下に在るものの一覧（並べ替え済み）。
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("フォルダを読める")
        .map(|entry| entry.expect("項目を読める").path())
        .collect();
    found.sort();
    found
}

#[test]
fn missing_or_empty_value_is_unset_and_creates_nothing() {
    // 値に行き先が無いので、作るとすれば相対の道筋が落ちる先（カレント＝このクレートのフォルダ）。
    let cwd = std::env::current_dir().expect("カレントが分かる");
    let before = entries(&cwd);
    for value in [None, Some(""), Some(" "), Some(" \t\r\n")] {
        let got = resolve(value.map(OsString::from));
        assert!(
            matches!(got, Err(WatchError::HomeUnset)),
            "{value:?}: {got:?}"
        );
    }
    assert_eq!(entries(&cwd), before, "既定の場所へ倒れて何かを作った");
}

// ───────── 絶対パスでない値 ─────────

/// 絶対パスでない形の値たち（名前, 値）。どれも、断られずにフォルダが作られるとすれば
/// `dir` の下の `<名前>` に落ちる（`dir` の外には何も作られない）。
#[cfg(windows)]
fn not_absolute_into(dir: &Path) -> Vec<(&'static str, String)> {
    let cwd = std::env::current_dir().expect("カレントが分かる");
    // カレントから見た相対の道筋（`..\..\target\…`）。
    let shared = cwd.components().zip(dir.components());
    let shared = shared.take_while(|(here, there)| here == there).count();
    let up = cwd.components().skip(shared).map(|_| Component::ParentDir);
    let relative: PathBuf = up.chain(dir.components().skip(shared)).collect();
    let relative = relative.to_string_lossy();
    // ドライブ（`C:`）と、ドライブを落とした残り（`\home\…`）。
    let mut rest = dir.components();
    let drive = rest.next().expect("ドライブが在る");
    let drive = drive.as_os_str().to_string_lossy();
    let rooted = rest.as_path().to_string_lossy();
    vec![
        // `..\x` の形。
        ("relative", format!("{relative}\\relative")),
        // `.` 始まりの形。
        ("dotted", format!(".\\{relative}\\dotted")),
        // `C:foo` の形（ドライブ文字つきだが、そのドライブのカレントが基準）。
        ("on-drive", format!("{drive}{relative}\\on-drive")),
        // `\foo` の形（ドライブが無い）。
        ("rooted", format!("{rooted}\\rooted")),
        // `/c/…` の形（Git Bash の綴り。ドライブが無く、区切りが `/`）。
        ("slashed", format!("{}/slashed", rooted.replace('\\', "/"))),
    ]
}

/// 断られ、文に載る値が渡した値のままであること。
#[cfg(windows)]
fn assert_refused(value: &str) {
    let got = resolve(Some(OsString::from(value)));
    let Err(WatchError::HomeNotAbsolute { dir }) = &got else {
        panic!("{value}: 断られなかった: {got:?}");
    };
    assert_eq!(dir, value);
}

/// カレントが基準になる形（[`not_absolute_into`] の名前）。
#[cfg(windows)]
const RELATIVE: [&str; 3] = ["relative", "dotted", "on-drive"];
/// 今のドライブの根が基準になる形（[`not_absolute_into`] の名前）。
#[cfg(windows)]
const ROOTED: [&str; 2] = ["rooted", "slashed"];

/// 絶対パスでない値は断られ、フォルダは作られない。断られなければ一時フォルダの中に落ちる
/// 値だけを渡すので、判定が壊れていても `target\` の外には何も作られない。
#[cfg(windows)]
fn values_landing_in_a_temp_folder_are_refused_and_create_nothing(shapes: &[&str]) {
    let root = TempPath::under_target("impl-watch-home");
    let mut values = not_absolute_into(root.path());
    values.retain(|(name, _)| shapes.contains(name));
    assert_eq!(values.len(), shapes.len(), "形の名前が表と合わない");
    for (name, value) in values {
        assert!(!Path::new(&value).is_absolute(), "前提: {value}");
        // 較正: 断られなければ、この値のフォルダは一時フォルダの中にできる。
        let lands = std::path::absolute(&value).expect("綴れる");
        assert_eq!(lands, root.child(name), "{value}");
        assert_refused(&value);
    }
    assert_eq!(
        entries(root.path()),
        Vec::<PathBuf>::new(),
        "断った値のフォルダを作った"
    );
}

#[cfg(windows)]
#[test]
fn a_relative_value_is_refused_before_anything_is_created() {
    values_landing_in_a_temp_folder_are_refused_and_create_nothing(&RELATIVE);
}

#[cfg(windows)]
#[test]
fn a_rooted_value_without_a_drive_is_refused_before_anything_is_created() {
    values_landing_in_a_temp_folder_are_refused_and_create_nothing(&ROOTED);
}

/// 設計が名指す綴り（`foo`・`.`・`..\x`・`\foo`・`C:foo`・`/c/x`）。断られなければカレントや
/// ドライブの根（`target\` の外）にフォルダができる値なので、先に「一時フォルダの中に落ちる値」で
/// 同じ判定を通し、断ると分かってから渡す（判定が壊れていれば、渡す前に赤で止まる）。
#[cfg(windows)]
#[test]
fn the_spellings_named_by_the_design_are_refused_and_create_nothing() {
    values_landing_in_a_temp_folder_are_refused_and_create_nothing(&RELATIVE);
    values_landing_in_a_temp_folder_are_refused_and_create_nothing(&ROOTED);

    // 標準ライブラリの判定そのもの（ファイルには触らない）。ドライブ文字つき・UNC・`\\?\` は絶対。
    for value in ["foo", ".", r"..\x", r"\foo", "C:foo", "/c/x"] {
        assert!(!Path::new(value).is_absolute(), "{value}");
    }
    for value in [r"C:\x", r"\\server\share\x", r"\\?\C:\x"] {
        assert!(Path::new(value).is_absolute(), "{value}");
    }

    let cwd = std::env::current_dir().expect("カレントが分かる");
    let before = entries(&cwd);
    let drive = cwd.components().next().expect("ドライブが在る");
    let drive = drive.as_os_str().to_string_lossy();
    // ほかの何とも重ならない名前。
    let name = format!("impl-watch-home-test-{}", std::process::id());
    for value in [
        name.clone(),
        ".".to_owned(),
        format!("..\\{name}"),
        format!("\\{name}"),
        format!("{drive}{name}"),
        format!("/c/{name}"),
    ] {
        // 断られなければ作られる道筋のうち、いま無いもののいちばん上（`.` には無い）。
        let lands = std::path::absolute(&value).expect("綴れる");
        let missing = lands.ancestors().take_while(|at| !at.exists());
        let new = missing.last().map(Path::to_path_buf);
        assert_refused(&value);
        assert!(
            new.is_none_or(|new| !new.exists()),
            "{value}: フォルダを作った"
        );
    }
    assert_eq!(entries(&cwd), before, "カレントに何かを作った");
}

/// `\\?\` で始まる形は絶対パスなので通る（UNC の形は、上の判定の表だけで確かめる）。
#[cfg(windows)]
#[test]
fn a_verbatim_absolute_value_is_accepted() {
    let root = TempPath::under_target("impl-watch-home");
    let plain = root.child("verbatim");
    let dir = PathBuf::from(format!(r"\\?\{}", plain.display()));

    let home = resolve(Some(dir.clone().into_os_string())).expect("絶対パスは通る");

    assert_eq!(home.dir, dir);
    assert!(plain.is_dir());
}

// ───────── 作る・使う ─────────

#[test]
fn present_value_creates_the_folder_and_nothing_inside() {
    let root = TempPath::under_target("impl-watch-home");
    // 途中のフォルダもまだ無い。
    let dir = root.child("a").join("b").join("home");
    assert!(!dir.exists());

    let home = resolve(Some(dir.clone().into_os_string())).expect("作れる");

    assert_eq!(home.dir, dir);
    assert!(dir.is_dir());
    assert_eq!(
        entries(&dir),
        Vec::<PathBuf>::new(),
        "解決はフォルダだけを作る"
    );
}

#[test]
fn existing_folder_is_used_as_it_is() {
    let root = TempPath::under_target("impl-watch-home");
    let kept = root.child("state.json");
    std::fs::write(&kept, b"keep").expect("書ける");

    let home = resolve(Some(root.path().as_os_str().to_owned())).expect("在るフォルダは通る");

    assert_eq!(home.dir, root.path());
    assert_eq!(std::fs::read(&kept).expect("読める"), b"keep");
    assert_eq!(entries(root.path()), vec![kept]);
}

#[test]
fn a_place_that_cannot_be_a_folder_is_not_creatable() {
    let root = TempPath::under_target("impl-watch-home");
    let file = root.child("file.txt");
    std::fs::write(&file, b"keep").expect("書ける");

    // 指す先そのものがファイル／親がファイル（道筋には ASCII の外の字も入れる）。
    for dir in [file.clone(), file.join("置き場")] {
        let err = resolve(Some(dir.clone().into_os_string())).expect_err("フォルダにできない");
        let WatchError::HomeNotCreatable { dir: spelled, .. } = &err else {
            panic!("別の失敗: {err:?}");
        };
        assert_eq!(spelled.as_str(), dir.to_string_lossy());
        let text = err.to_string();
        assert!(
            text.starts_with("AREKA_IMPL_WATCH_HOME cannot be created: "),
            "{text}"
        );
        assert!(text.is_ascii(), "ASCII の外の字: {text}");
        assert!(!text.contains(['\r', '\n']), "1 行でない: {text:?}");
    }

    assert_eq!(std::fs::read(&file).expect("読める"), b"keep");
    assert_eq!(entries(root.path()), vec![file]);
}

#[test]
fn paths_under_the_home_use_the_design_names() {
    let dir = Path::new("H");
    let home = Home {
        dir: dir.to_path_buf(),
    };
    assert_eq!(home.state_path(), dir.join("state.json"));
    assert_eq!(home.lock_path(), dir.join("state.lock"));
    assert_eq!(home.status_path(), dir.join("status.md"));
    assert_eq!(home.log_path(), dir.join("impl-watch.log"));
    assert_eq!(home.alive_dir(), dir.join("alive"));
}

#[test]
fn alive_path_is_id_dot_kind_dot_lock_for_every_kind() {
    let home = Home {
        dir: PathBuf::from("H"),
    };
    // 識別は点を含みうる（拡張子の付け替えで組むと欠ける）。
    let id = "areka-P0.impl_watch";
    for (kind, name) in [
        (WaitKind::Watch, "watch"),
        (WaitKind::Merge, "merge"),
        (WaitKind::Load, "load"),
        (WaitKind::Resume, "resume"),
    ] {
        assert_eq!(
            home.alive_path(id, kind),
            Path::new("H")
                .join("alive")
                .join(format!("areka-P0.impl_watch.{name}.lock"))
        );
        // ファイル名の綴りは状態ファイルの綴りと同じ。
        assert_eq!(
            serde_json::to_string(&kind).expect("綴れる"),
            format!("\"{name}\"")
        );
    }
}
