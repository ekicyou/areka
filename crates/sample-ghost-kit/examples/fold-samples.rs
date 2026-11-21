//! 検体の展開済みツリーを `.nar` に畳む**使い捨ての手順**。
//!
//! # 呼び方
//!
//! ```text
//! cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/<展開フォルダ>
//! cargo run -p sample-ghost-kit --example fold-samples -- --from vendors/sample_ghost/<展開フォルダ> --check
//! ```
//!
//! - `--from <展開形のフォルダ>`: そのフォルダ 1 本を畳む。登記表を見ないので、まだ
//!   登記していない検体でも畳める（登記の 1 行は畳んだ**後**に足す）。`.nar` の名前は
//!   フォルダ名から採る。
//! - `--check`: `.nar` を書き直さず、既にある `.nar` の往復だけを確かめる。
//!
//! `--from` は必須である。登記表は展開形の在処を持たない（保管は
//! `vendors/sample_ghost/<名>.nar` に統一されている）ので、畳む元のフォルダは呼び手が指す
//! しかない。
//!
//! # なぜ使い捨てか
//!
//! 畳むのは検体を足すときの 1 回だけで、常設の判定は「登記の往復」が
//! 持つ。ここに残すのは**同じ結果をもう一度作れる**ようにするためで、手順そのものは
//! `vendors/sample_ghost/README.md` が案内する。
//!
//! # 何を入れるか——`git ls-files` が唯一の出どころ
//!
//! フォルダを走査して入れると、追跡外の永続化フォルダ（`ghost/master/profile/`）まで
//! 入る（emo2 の木では走査と追跡がずれていた）。そこで**追跡ファイルだけを作業フォルダへ
//! 写してから畳む**。写しはバイトの複製なので、改行も文字コードも変わらない。
//!
//! # 何と突き合わせるか
//!
//! 畳んだ `.nar` を本番の展開器で空の根へ入れ、**インストール済み形の全ファイル**を
//! 元の追跡ファイルと 1 バイト単位で突き合わせる。同梱バルーンを持つ検体は
//! `<元>/<取り出し元>/…` → `<根>/<格納先>/<directory>/balloon/<バルーンの directory>/…`
//! （`areka-nar` の置き方と同じく、ゴーストのフォルダの中）、それ以外は
//! `<元>/…` → `<根>/<格納先>/<directory>/…`（格納先はゴーストが `ghost`、バルーンが `balloon`）の写像で、**和**が元のツリーに一致する
//! ことを見る（片側だけ数えても抜けは見えない）。写像の元は `install.txt` の解釈結果
//! だけで、登記表の綴りは使わない（`--from` でも同じ突き合わせが働く）。
//!
//! # どこへ書くか
//!
//! 畳んだ `.nar` はまず作業の置き場 `target/fold-samples/out/<名>.nar` に書いて往復を
//! 照らし、不一致が 0 件のときだけ `vendors/sample_ghost/<名>.nar` へ写す。確かめが
//! 落ちても、それまでの正しい `.nar` は消えない。

use areka_nar::{InstallKind, InstallManifest, InstallRequest, NarArchive, SingleRoot};
use sample_ghost_kit::{ascii, fold_tree};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{MAIN_SEPARATOR_STR, Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{fs, io};

/// 端末へ出す文は英語の ASCII だけ（OS の日本語の文も `ascii` に写す）。
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("fold-samples: {}", ascii(&error.to_string()));
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (from, check_only) = parse_args(&args)?;

    // 引数の読みは作業の置き場を消す前に（書き損じの呼び出しで置き場を消さない）。
    let root = workspace_root();
    let source = match &from {
        Some(spelled) => resolve(&root, spelled)?,
        None => {
            return Err(
                "--from <unfolded folder> is required (the registry does not hold where unfolded trees are)"
                    .into(),
            );
        }
    };

    let scratch = root.join("target").join("fold-samples");
    if scratch.exists() {
        fs::remove_dir_all(&scratch)?;
    }

    let tracked = tracked_files(&source)?;
    let mismatches = fold_one(&root, &scratch, &source, &tracked, check_only)?;
    if mismatches != 0 {
        return Err(format!("{mismatches} mismatches").into());
    }
    if !check_only {
        let name = folder_name(&source)?;
        let (from, to) = (folded_nar(&scratch, &name), vendored_nar(&root, &name));
        fs::copy(&from, &to).map_err(|err| copy_failed(&from, &to, &err))?;
    }
    println!("round trip matched");
    Ok(())
}

/// 写しの失敗の文に両端のパスを添える（ASCII は `main` の出口が写す）。
fn copy_failed(from: &Path, to: &Path, err: &io::Error) -> String {
    format!("copy {} to {}: {err}", from.display(), to.display())
}

/// 引数を `--from <フォルダ>` と `--check` に読む。ほかの語は黙って捨てずに断る
/// （打ち間違えた `--check` を無視して `vendors/` を書き直さないため）。
fn parse_args(args: &[String]) -> Result<(Option<String>, bool), Box<dyn Error>> {
    let mut from = None;
    let mut check_only = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--from" => {
                from = Some(
                    rest.next()
                        .ok_or("--from must be followed by a folder")?
                        .clone(),
                );
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }
    Ok((from, check_only))
}

/// 展開形のフォルダ 1 本を畳み、空の根へ戻して突き合わせる。返すのは不一致の件数。
///
/// `.nar` の名前はフォルダ名から採る（登記表を引かない）。`tracked` は `source` からの
/// 相対パス（`/` 区切り）の追跡ファイルの一覧。畳んだ `.nar` は作業の置き場へ書き、
/// `vendors/` へは書かない（写すのは不一致 0 を見た `run`）。`check_only` なら畳まずに
/// `vendors/` の既にある `.nar` を照らす。
fn fold_one(
    root: &Path,
    scratch: &Path,
    source: &Path,
    tracked: &[String],
    check_only: bool,
) -> Result<usize, Box<dyn Error>> {
    let name = folder_name(source)?;
    // 追跡ファイルが 0 件だと畳む対象が無く、突き合わせが恒真になる。
    if tracked.is_empty() {
        return Err(format!(
            "no tracked files under {} (git add the unfolded tree first)",
            source.display()
        )
        .into());
    }

    let stage = scratch.join("stage").join(&name);
    for relative in tracked {
        let to = stage.join(relative.replace('/', MAIN_SEPARATOR_STR));
        fs::create_dir_all(to.parent().expect("写し先には必ず親がある"))?;
        let from = source.join(relative);
        fs::copy(&from, &to).map_err(|err| copy_failed(&from, &to, &err))?;
    }
    let staged = walk(&stage)?;
    assert_eq!(
        staged.len(),
        tracked.len(),
        "{name} の写しの数が追跡の数と違う"
    );

    let nar = if check_only {
        vendored_nar(root, &name)
    } else {
        let out = folded_nar(scratch, &name);
        fs::create_dir_all(out.parent().expect("書き先には必ず親がある"))?;
        fold_tree(&stage)?.write_to(&out)?;
        out
    };

    let dest_root = scratch.join("roundtrip").join(&name);
    fs::create_dir_all(&dest_root)?;
    let archive = NarArchive::open(&nar)?;
    let manifest = archive.manifest();
    archive.install(&InstallRequest {
        roots: &SingleRoot(&dest_root),
        target_ghost: None,
    })?;

    let expected: BTreeMap<PathBuf, PathBuf> = tracked
        .iter()
        .map(|relative| {
            Ok((
                installed_path(&dest_root, manifest, relative)?,
                source.join(relative),
            ))
        })
        .collect::<Result<_, String>>()?;
    assert_eq!(
        expected.len(),
        tracked.len(),
        "{name} の写像が 2 つの元を同じ宛先に潰した"
    );
    let actual = walk(&dest_root)?;

    let mut mismatches = Vec::new();
    for path in &actual {
        if !expected.contains_key(path) {
            mismatches.push(format!(
                "placed but not in the source: {}",
                ascii(&path.to_string_lossy())
            ));
        }
    }
    for (path, origin) in &expected {
        if !actual.contains(path) {
            mismatches.push(format!("not placed: {}", ascii(&path.to_string_lossy())));
            continue;
        }
        if fs::read(path)? != fs::read(origin)? {
            mismatches.push(format!("bytes differ: {}", ascii(&path.to_string_lossy())));
        }
    }

    println!(
        "{name}: tracked {tracked} files / installed {actual} files / {folders} / .nar {size} bytes / mismatches {bad}",
        name = ascii(&name),
        tracked = tracked.len(),
        actual = actual.len(),
        folders = ascii(&folder_counts(&dest_root, &actual)),
        size = fs::metadata(&nar)?.len(),
        bad = mismatches.len(),
    );
    for line in &mismatches {
        println!("  ! {line}");
    }
    Ok(mismatches.len())
}

/// `.nar` の名前に使うフォルダ名。
fn folder_name(source: &Path) -> Result<String, Box<dyn Error>> {
    Ok(source
        .file_name()
        .and_then(|raw| raw.to_str())
        .ok_or_else(|| format!("cannot take a folder name from {}", source.display()))?
        .to_owned())
}

/// 畳んだ `.nar` を往復の確かめの前に置く作業の置き場。
fn folded_nar(scratch: &Path, name: &str) -> PathBuf {
    scratch.join("out").join(format!("{name}.nar"))
}

/// 確かめを通った `.nar` の保管先。
fn vendored_nar(root: &Path, name: &str) -> PathBuf {
    root.join("vendors")
        .join("sample_ghost")
        .join(format!("{name}.nar"))
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
    Err(format!("--from {spelled} is not a folder").into())
}

/// 元のツリーの相対パスが、インストール済み形でどこへ置かれるか。
///
/// 同梱バルーンの取り出し元の配下は取り出し元の段を剥がして本体のフォルダの中のバルーン格納先
/// （`<根>/<格納先>/<directory>/balloon/<バルーンの directory>/`・`areka-nar` の置き方と
/// 同じ）へ、それ以外は本体の置き場へ。判断の元は `install.txt` の解釈結果だけで、登記の
/// 綴りは使わない。検体に居ない種別（シェル・追加ファイル）は写像を持たないので断る。
fn installed_path(
    root: &Path,
    manifest: &InstallManifest,
    relative: &str,
) -> Result<PathBuf, String> {
    let store = match manifest.kind {
        InstallKind::Ghost => "ghost",
        InstallKind::Balloon => "balloon",
        other => return Err(format!("unsupported kind {other:?} (add a mapping for it)")),
    };
    let body = root.join(store).join(&manifest.directory);
    for companion in &manifest.companions {
        if let Some(tail) = strip_folder(relative, &companion.source_directory) {
            return Ok(body.join("balloon").join(&companion.directory).join(tail));
        }
    }
    Ok(body.join(relative.replace('/', MAIN_SEPARATOR_STR)))
}

/// `relative` が `folder`（`/` 区切りで 1 段以上）の配下なら、剥がした残りを OS の区切りで返す。
///
/// 各段は `to_uppercase` で大小を畳んで照らす（展開器も段ごとに大小を畳む。ASCII だけを
/// 畳むと、大小の違う非 ASCII の名前を取り出し元の外と見誤る）。
fn strip_folder(relative: &str, folder: &str) -> Option<String> {
    let mut rest = relative;
    for part in folder.split('/') {
        let (head, tail) = rest.split_once('/')?;
        // shortcut: to_uppercase は ß→SS のように 2 字になる字で areka-nar の names::fold より甘い（不一致としては表に出る）。検体にそういう字が入ったら fold と同じ 1 字対 1 字の畳みにする
        if head.to_uppercase() != part.to_uppercase() {
            return None;
        }
        rest = tail;
    }
    Some(rest.replace('/', MAIN_SEPARATOR_STR))
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
            "git ls-files failed ({}): {}",
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

/// 根の直下の格納先ごとのファイル数を「ghost/<名>=<件数>」の形で（同梱バルーンは本体のフォルダの中に数える）。
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

#[cfg(test)]
#[path = "fold-samples_tests.rs"]
mod tests;
