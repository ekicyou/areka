//! 検体の展開済みツリーを `.nar` に畳む**使い捨ての手順**（spec: `areka-P0-nar-install`
//! タスク 5.1・要件 8.2〜8.4・8.7）。
//!
//! # 呼び方
//!
//! ```text
//! cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon
//! cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/StayseeBalloon --check
//! ```
//!
//! - `--from <展開形のフォルダ>`: そのフォルダ 1 本を畳む。登記表を見ないので、まだ
//!   登記していない検体でも畳める（登記の 1 行は畳んだ**後**に足す）。`.nar` の名前は
//!   フォルダ名から採る。
//! - `--check`: `.nar` を書き直さず、既にある `.nar` の往復だけを確かめる。
//!
//! `--from` は必須である。段 ③ の登記表は展開形の在処を持たない（保管は
//! `vendors/sample_ghost/<名>.nar` に統一され、展開形はタスク 5.6 で消える）ので、畳む元の
//! フォルダは呼び手が指すしかない。
//!
//! # なぜ使い捨てか
//!
//! 畳むのは検体を足すときの 1 回だけで、常設の判定は「登記の往復」（タスク 5.7）が
//! 持つ。ここに残すのは**同じ結果をもう一度作れる**ようにするためで、手順そのものは
//! `vendors/sample_ghost/README.md`（タスク 6.4）が案内する。
//!
//! # 何を入れるか——`git ls-files` が唯一の出どころ（要件 8.3）
//!
//! フォルダを走査して入れると、追跡外の永続化フォルダ（`ghost/master/profile/`）まで
//! 入る。emo2 の木は今日、ディスク上 157 ファイル・追跡 110 ファイルで、走査と追跡が
//! 47 件ずれている。そこで**追跡ファイルだけを作業フォルダへ写してから畳む**。写しは
//! バイトの複製なので、改行も文字コードも変わらない（要件 8.7）。
//!
//! # 何と突き合わせるか（要件 8.4）
//!
//! 畳んだ `.nar` を本番の展開器で空の根へ入れ、**インストール済み形の全ファイル**を
//! 元の追跡ファイルと 1 バイト単位で突き合わせる。同梱バルーンを持つ検体は
//! `<元>/<取り出し元>/…` → `<根>/balloon/<directory>/…`、それ以外は
//! `<元>/…` → `<根>/ghost/<directory>/…` の写像で、**和**が元のツリーに一致する
//! ことを見る（片側だけ数えても抜けは見えない）。写像の元は `install.txt` の解釈結果
//! だけで、登記表の綴りは使わない（`--from` でも同じ突き合わせが働く）。

use areka_nar::{InstallKind, InstallManifest, InstallRequest, NarArchive};
use sample_ghost_kit::fold_tree;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};
use std::process::Command;
use std::{fs, io};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check_only = args.iter().any(|arg| arg == "--check");
    let from = match args.iter().position(|arg| arg == "--from") {
        Some(at) => Some(
            args.get(at + 1)
                .ok_or("--from の後にフォルダを書くこと")?
                .clone(),
        ),
        None => None,
    };

    let root = workspace_root();
    let scratch = root.join("target").join("fold-samples");
    if scratch.exists() {
        fs::remove_dir_all(&scratch)?;
    }

    // 畳む対象。`--from` は登記表を見ない＝まだ登記していない検体でも畳める。
    let sources: Vec<PathBuf> = match &from {
        Some(spelled) => vec![resolve(&root, spelled)?],
        None => {
            return Err(
                "--from <展開形のフォルダ> を書くこと（登記表は展開形の在処を持たない）".into(),
            );
        }
    };

    let mut mismatches = 0usize;
    for source in &sources {
        mismatches += fold_one(&root, &scratch, source, check_only)?;
    }

    if mismatches != 0 {
        return Err(format!("不一致が {mismatches} 件ある").into());
    }
    println!("全ての検体で一致");
    Ok(())
}

/// 展開形のフォルダ 1 本を畳み、空の根へ戻して突き合わせる。返すのは不一致の件数。
///
/// `.nar` の名前はフォルダ名から採る（登記表を引かない）。
fn fold_one(
    root: &Path,
    scratch: &Path,
    source: &Path,
    check_only: bool,
) -> Result<usize, Box<dyn Error>> {
    let name = source
        .file_name()
        .and_then(|raw| raw.to_str())
        .ok_or_else(|| format!("{} からフォルダ名が採れない", source.display()))?
        .to_owned();

    let tracked = tracked_files(source)?;
    assert!(
        !tracked.is_empty(),
        "{} の追跡ファイルが 0 件（畳む対象が無い＝突き合わせが恒真になる。\
         展開形が消えているなら --from で在るフォルダを指すこと）",
        source.display()
    );

    // 追跡ファイルだけを作業フォルダへ写す。
    let stage = scratch.join("stage").join(&name);
    for relative in &tracked {
        let to = stage.join(relative.replace('/', MAIN_SEPARATOR_STR));
        fs::create_dir_all(to.parent().expect("写し先には必ず親がある"))?;
        fs::copy(source.join(relative), &to)?;
    }
    let staged = walk(&stage)?;
    assert_eq!(
        staged.len(),
        tracked.len(),
        "{name} の写しの数が追跡の数と違う"
    );

    let nar = root
        .join("vendors")
        .join("sample_ghost")
        .join(format!("{name}.nar"));
    if !check_only {
        fold_tree(&stage)?.write_to(&nar)?;
    }

    // 往復——空の根へ入れて、元のツリーと突き合わせる。
    let dest_root = scratch.join("roundtrip").join(&name);
    fs::create_dir_all(&dest_root)?;
    let archive = NarArchive::open(&nar)?;
    let manifest = archive.manifest();
    archive.install(&InstallRequest {
        root: &dest_root,
        target_ghost: None,
    })?;

    let expected: BTreeMap<PathBuf, PathBuf> = tracked
        .iter()
        .map(|relative| {
            (
                installed_path(&dest_root, manifest, relative),
                source.join(relative),
            )
        })
        .collect();
    assert_eq!(
        expected.len(),
        tracked.len(),
        "{name} の写像が 2 つの元を同じ宛先に潰した"
    );
    let actual = walk(&dest_root)?;

    let mut mismatches = Vec::new();
    for path in &actual {
        if !expected.contains_key(path) {
            mismatches.push(format!("元に無いものが置かれた: {}", path.display()));
        }
    }
    for (path, origin) in &expected {
        if !actual.contains(path) {
            mismatches.push(format!("置かれなかった: {}", path.display()));
            continue;
        }
        if fs::read(path)? != fs::read(origin)? {
            mismatches.push(format!("バイトが違う: {}", path.display()));
        }
    }

    println!(
        "{name}: 追跡 {tracked} ファイル / 展開 {actual} ファイル / {folders} / .nar {size} バイト / 不一致 {bad} 件",
        tracked = tracked.len(),
        actual = actual.len(),
        folders = folder_counts(&dest_root, &actual),
        size = fs::metadata(&nar)?.len(),
        bad = mismatches.len(),
    );
    for line in &mismatches {
        println!("  ! {line}");
    }
    Ok(mismatches.len())
}

/// `--from` に書かれた綴りをフォルダに解く。呼んだ場所からでもリポジトリ根からでも書ける。
fn resolve(root: &Path, spelled: &str) -> Result<PathBuf, Box<dyn Error>> {
    let as_written = PathBuf::from(spelled);
    if as_written.is_dir() {
        return Ok(as_written);
    }
    let from_root = root.join(spelled);
    if from_root.is_dir() {
        return Ok(from_root);
    }
    Err(format!("--from に書かれた {spelled} がフォルダとして見つからない").into())
}

/// 元のツリーの相対パスが、インストール済み形でどこへ置かれるか（要件 8.4 の写像）。
///
/// 同梱バルーンの取り出し元の配下は 1 階層剥がしてバルーン格納先へ、それ以外は本体の
/// 置き場へ。判断の元は `install.txt` の解釈結果だけで、登記の綴りは使わない。
fn installed_path(root: &Path, manifest: &InstallManifest, relative: &str) -> PathBuf {
    for companion in &manifest.companions {
        if let Some(tail) = strip_folder(relative, &companion.source_directory) {
            return root.join("balloon").join(&companion.directory).join(tail);
        }
    }
    let store = match manifest.kind {
        InstallKind::Ghost => "ghost",
        InstallKind::Balloon => "balloon",
        other => panic!("検体に {other:?} は居ない（居るなら写像を足すこと）"),
    };
    root.join(store)
        .join(&manifest.directory)
        .join(relative.replace('/', MAIN_SEPARATOR_STR))
}

/// `relative` が `folder` の配下なら、剥がした残りを OS の区切りで返す。
fn strip_folder(relative: &str, folder: &str) -> Option<String> {
    let (head, tail) = relative.split_once('/')?;
    head.eq_ignore_ascii_case(folder)
        .then(|| tail.replace('/', MAIN_SEPARATOR_STR))
}

/// `dir` の配下の追跡ファイルを、`dir` からの相対パス（`/` 区切り）で名前順に返す。
///
/// `git ls-files` を `dir` の中から呼ぶ。引数無しの `ls-files` は**その場所からの相対**で
/// 名前を出すので、リポジトリ根からの位置関係を綴らずに済む（`--from` で任意のフォルダを
/// 渡せるのはこのため）。
fn tracked_files(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(["ls-files", "-z"])
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files が失敗した（{}）: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    let mut files = Vec::new();
    for raw in output.stdout.split(|byte| *byte == 0) {
        if raw.is_empty() {
            continue;
        }
        files.push(std::str::from_utf8(raw)?.to_owned());
    }
    files.sort();
    Ok(files)
}

/// `dir` の配下の全ファイルの絶対パスを名前順に返す（フォルダは数えない）。
fn walk(dir: &Path) -> io::Result<BTreeSet<PathBuf>> {
    let mut found = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for child in fs::read_dir(&current)? {
            let child = child?;
            if child.file_type()?.is_dir() {
                stack.push(child.path());
            } else {
                found.insert(child.path());
            }
        }
    }
    Ok(found)
}

/// 根の直下の格納先ごとのファイル数を「ghost/emo2=90 balloon/emo2-kakukaku=20」の形で。
fn folder_counts(root: &Path, files: &BTreeSet<PathBuf>) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        let relative = file.strip_prefix(root).expect("走査の起点の配下");
        let mut parts = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned());
        let store = parts.next().unwrap_or_default();
        let folder = parts.next().unwrap_or_default();
        *counts.entry(format!("{store}/{folder}")).or_default() += 1;
    }
    counts
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// リポジトリ根。窓口と同じ綴り方（本例は使い捨てなので窓口の私有関数は借りない）。
fn workspace_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}
