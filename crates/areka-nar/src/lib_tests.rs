//! 公開面（`lib.rs`）の兄弟テスト。
//!
//! ここが判定するのは 6 つ。
//!
//! ⑴ `open` は 1 バイトも書かない——固定入力を全て通した前後で根の木が 1 つも
//!    変わらないことを、実際に歩いて突き合わせる（走査そのものの較正を 3 本持つ）。
//! ⑵ `install` は伸長をやり直さない——`open` の後に原本を潰しても、展開された
//!    バイト列が元のままであることで測る（潰れたことの対照も置く）。
//! ⑶ `install` は何度呼んでも同じ結果になる（設計 `install` の節）。
//! ⑷ 記録は公開面の 2 つの関数だけが出す——本番のソースの字面で見張る。
//! ⑸ 伸長器の書き込み側の名前を本番のソースが綴らない（要件 10.2）。
//! ⑹ 新設したファイルが 1,000 行未満（要件 10.8）。
//!
//! 語彙の全数対応と「失敗のたびに記録が 1 回」は [`vocabulary`] が持つ。
//!
//! 根は OS の一時フォルダではなく [`WorkDir`] が配る（要件 7.10）。

use super::*;
use sample_ghost_kit::{NarBuilder, WorkDir, install_txt};
use std::collections::BTreeMap;
use std::fs;

/// 語彙の全数対応と記録の判定。1 ファイル 1,000 行の上限に収めるために分けただけで、
/// 助手はここから借りる。
#[path = "lib_vocabulary_tests.rs"]
mod vocabulary;

// ---- 助手 ----

/// ゴースト 1 本ぶんの受理される固定入力。同梱バルーン `kaku` を 1 つ持つ。
fn ghost_with_balloon() -> NarBuilder {
    NarBuilder::new()
        .file(
            "install.txt",
            &install_txt(&[
                "type,ghost",
                "name,テスト",
                "directory,tester",
                "balloon.directory,kaku",
                "balloon.source.directory,kaku",
            ]),
        )
        .done()
        .file("ghost/master/descript.txt", b"charset,Shift_JIS\r\n")
        .done()
        .file("ghost/master/dic.txt", b"line one\r\nline two\r\n")
        .deflate()
        .done()
        .file("kaku/balloon.txt", b"kaku\r\n")
        .done()
}

/// 木の全内容。ファイルは相対パス → バイト列、フォルダは末尾 `/` の相対パス → 空。
///
/// フォルダが無ければ空の写像を返す。
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    if root.is_dir() {
        walk(root, "", &mut out);
    }
    out
}

fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for child in fs::read_dir(dir).expect("木を辿れる") {
        let child = child.expect("要素を読める");
        let name = child.file_name().to_string_lossy().into_owned();
        let relative = format!("{prefix}{name}");
        if child.file_type().expect("種別を読める").is_dir() {
            out.insert(format!("{relative}/"), Vec::new());
            walk(&child.path(), &format!("{relative}/"), out);
        } else {
            out.insert(relative, fs::read(child.path()).expect("中身を読める"));
        }
    }
}

/// 書庫をファイルに置いて、そのパスを返す。
fn put(root: &Path, name: &str, builder: &NarBuilder) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, builder.bytes()).expect("固定入力を置ける");
    path
}

/// 本番のソース（兄弟テストを除く）を (ファイル名, 中身) で返す。
fn production_sources() -> Vec<(String, String)> {
    source_files()
        .into_iter()
        .filter(|(name, _)| !name.ends_with("_tests.rs"))
        .collect()
}

/// `src/` 直下の Rust のソース全て（兄弟テストを含む）を (ファイル名, 中身) で返す。
///
/// 一覧を手で綴らずに毎回数え直すので、ファイルを足した人が一覧を直し忘れても
/// 見張りの網から漏れない。
fn source_files() -> Vec<(String, String)> {
    let dir = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut out: Vec<(String, String)> = fs::read_dir(dir)
        .expect("本クレートの src を読める")
        .map(|child| child.expect("要素を読める").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".rs"))
        .map(|name| {
            let text = fs::read_to_string(dir.join(&name)).expect("ソースを読める");
            (name, text)
        })
        .collect();
    out.sort();
    out
}

/// `src/` の 1 ファイルの中身。
fn source_of(name: &str) -> String {
    fs::read_to_string(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src")).join(name))
        .expect("ソースを読める")
}

// ---- 公開面が外から届く（設計「公開面」） ----

/// 公開面の型が `areka_nar::X` として外から届くことは、`lib.rs` の doctest が
/// **別クレートとして**組まれることで測る。ここではその doctest が実在することだけを
/// 見張る——消えると「外から届く」の主張が黙って消える。
#[test]
fn the_public_surface_doctest_exists() {
    let lib = source_of("lib.rs");
    assert!(
        lib.contains("use areka_nar::{"),
        "公開面が外から届くことを測る doctest が lib.rs から消えている"
    );
}

// ---- `open` は 1 バイトも書かない（要件 4.1・9.1） ----

/// 受理される入力も拒否される入力も、`open` を通した前後で根の木が 1 つも変わらない。
///
/// 「書く呼び出しが無い」を字面で数えるのでは足りない——`lib.rs` は根のパスも
/// `std::fs` も正当に持つ。実際に歩いて突き合わせる。
#[test]
fn open_changes_nothing_under_the_root() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();

    let mut archives = vec![put(root, "good.nar", &ghost_with_balloon())];
    for case in vocabulary::cases() {
        archives.push(put(root, &format!("{}.nar", case.kind), &case.nar));
    }

    let before = tree(root);
    assert!(
        before.len() >= 14,
        "固定入力が置かれていない根で突き合わせても何も測れない: {}",
        before.len()
    );
    for archive in &archives {
        let _ = NarArchive::open(archive);
    }

    assert_eq!(tree(root), before, "open が根の下の何かを変えた");
}

/// 上の突き合わせの較正。木の写しが⑴増えた⑵中身が変わった⑶消えた のいずれでも
/// 違いとして出ることを確かめる。3 つとも出なければ、上のテストは恒真で緑になる。
#[test]
fn the_tree_snapshot_notices_every_kind_of_change() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();
    fs::write(root.join("a.txt"), b"one").expect("置ける");
    fs::create_dir(root.join("sub")).expect("掘れる");
    let before = tree(root);
    assert_eq!(before.len(), 2, "写しの母数が 0 では較正にならない");

    fs::write(root.join("b.txt"), b"two").expect("置ける");
    assert_ne!(tree(root), before, "増えたファイルを見落とした");
    fs::remove_file(root.join("b.txt")).expect("消せる");

    fs::write(root.join("a.txt"), b"ONE").expect("置ける");
    assert_ne!(tree(root), before, "同じ長さの書き換えを見落とした");
    fs::write(root.join("a.txt"), b"one").expect("置ける");

    fs::remove_dir(root.join("sub")).expect("消せる");
    assert_ne!(tree(root), before, "消えたフォルダを見落とした");
}

// ---- `install` は伸長をやり直さない（設計 `container`） ----

/// `open` の後に原本を潰しても、展開された中身は元のバイト列のまま。
///
/// 伸長済みのバイト列を `NarArchive` が保持しているからこそ通る。`install` が
/// 原本を読み直して伸長する形なら、潰した後の展開は失敗するか別の中身になる。
#[test]
fn install_does_not_inflate_a_second_time() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();
    let path = put(root, "ghost.nar", &ghost_with_balloon());

    let archive = NarArchive::open(&path).expect("無傷の書庫は開ける");
    fs::write(&path, "これはもう zip ではない").expect("原本を潰せる");
    // 対照——潰したことが本当に効いている（効いていなければ上の一手が無意味）。
    assert!(
        NarArchive::open(&path).is_err(),
        "潰した原本が開けてしまうなら、この判定は何も測っていない"
    );

    let outcome = archive
        .install(&InstallRequest {
            root,
            target_ghost: None,
        })
        .expect("保持した中身だけで展開できる");

    assert_eq!(outcome.installed.len(), 2, "本体と同梱バルーンの 2 つ");
    assert_eq!(
        fs::read(root.join("ghost/tester/ghost/master/dic.txt")).expect("読める"),
        b"line one\r\nline two\r\n",
        "deflate のエントリの中身が元のまま戻る"
    );
    assert_eq!(
        fs::read(root.join("balloon/kaku/balloon.txt")).expect("読める"),
        b"kaku\r\n"
    );
}

/// 伸長の綴りが本番のソースに 2 つしか無い（container の定義 1・lib.rs の呼び出し 1）。
///
/// 数えるのは括弧まで込みの綴りなので、`use` の行は入らない。上の実行時の判定と
/// 合わせて、「2 度目の伸長が無い」を構造の側からも言う。
#[test]
fn the_inflate_call_appears_once_outside_its_definition() {
    let needle = "inflate_entry(";
    let counted: Vec<(String, usize)> = production_sources()
        .into_iter()
        .map(|(name, text)| (name, text.matches(needle).count()))
        .filter(|(_, count)| *count > 0)
        .collect();
    assert_eq!(
        counted,
        vec![("container.rs".to_string(), 1), ("lib.rs".to_string(), 1)],
        "伸長は container の定義 1 つと lib.rs の呼び出し 1 つだけ"
    );
    // 較正——数え方が何も拾わない／何にでも当たるのではない。
    assert_eq!(
        "inflate_entry(a) inflate_entry(b)".matches(needle).count(),
        2
    );
    assert_eq!(
        "use crate::container::inflate_entry;"
            .matches(needle)
            .count(),
        0
    );
}

// ---- 何度呼んでも同じ結果（設計 `install`） ----

/// 同じ根へ同じ書庫を 2 度入れると、2 度目は `Overlaid` になり中身は同じ。
#[test]
fn install_is_idempotent() {
    let work = WorkDir::new().expect("根を借りられる");
    let root = work.path();
    let path = put(root, "ghost.nar", &ghost_with_balloon());
    let archive = NarArchive::open(&path).expect("開ける");
    let request = InstallRequest {
        root,
        target_ghost: None,
    };

    let first = archive.install(&request).expect("1 度目");
    let after_first = tree(&root.join("ghost").join("tester"));
    assert!(
        first
            .installed
            .iter()
            .all(|element| element.existing == ExistingState::New),
        "1 度目は新規: {:?}",
        first.installed
    );

    let second = archive.install(&request).expect("2 度目");
    assert!(
        second
            .installed
            .iter()
            .all(|element| element.existing == ExistingState::Overlaid),
        "2 度目は重ね置き: {:?}",
        second.installed
    );
    assert_eq!(second.name, first.name);
    assert_eq!(
        second.installed.iter().map(|e| &e.path).collect::<Vec<_>>(),
        first.installed.iter().map(|e| &e.path).collect::<Vec<_>>()
    );
    assert_eq!(
        tree(&root.join("ghost").join("tester")),
        after_first,
        "2 度目で木が変わった"
    );
}

// ---- 記録は公開面だけが出す（要件 9.1・設計「Monitoring」） ----

/// `tracing` を綴る本番のソースは `lib.rs` だけで、`error!` はその中に 1 つだけ。
///
/// 二重記録はログを読む人からは「2 回失敗した」に見える。出口が 1 つであることは
/// 実行時にも [`vocabulary`] が数えるが、「他所に無い」は字面でしか言えない。
#[test]
fn only_the_public_surface_writes_records() {
    let spelled: Vec<(String, usize)> = production_sources()
        .into_iter()
        .map(|(name, text)| (name, text.matches("tracing::").count()))
        .filter(|(_, count)| *count > 0)
        .collect();
    assert_eq!(
        spelled.len(),
        1,
        "記録を出す本番のソースは lib.rs だけ: {spelled:?}"
    );
    assert_eq!(spelled[0].0, "lib.rs");

    assert_eq!(
        source_of("lib.rs").matches("tracing::error!").count(),
        1,
        "失敗の記録の発火点は 1 つ（二重記録を避ける）"
    );
    // 較正——数え方が空振りしていない。
    assert_eq!("tracing::error!".matches("tracing::").count(), 1);
}

// ---- 伸長器の書き込み側を呼ばない（要件 10.2） ----

/// 圧縮側の綴り。このファイルに直に書くと自分自身が当たるので、連結して組む。
fn deflate_side_needle() -> String {
    format!("{}::{}", "miniz_oxide", "deflate")
}

/// 本番のソースが伸長器の書き込み側の名前を 1 つも綴らない（設計「Open Questions」）。
#[test]
fn no_deflate_side_is_called() {
    let needle = deflate_side_needle();
    let found: Vec<String> = production_sources()
        .into_iter()
        .filter(|(_, text)| text.contains(&needle))
        .map(|(name, _)| name)
        .collect();
    assert!(
        found.is_empty(),
        "本番のソースが {needle} を綴っている: {found:?}"
    );
}

/// 上の走査の較正。走査するファイルが実在すること、綴りを足せば拾えること、
/// 伸長側の綴りでは拾わないことを、この 1 本で確かめる。
#[test]
fn the_deflate_scan_looks_at_real_files_and_catches_the_spelling() {
    let sources = source_files();
    assert!(
        sources.len() >= 16,
        "src の走査が空なら no_deflate_side_is_called は恒真: {}",
        sources.len()
    );
    for expected in ["lib.rs", "container.rs", "install.rs", "lib_tests.rs"] {
        assert!(
            sources.iter().any(|(name, _)| name == expected),
            "{expected} が走査の対象に入っていない"
        );
    }
    let needle = deflate_side_needle();
    assert!(
        format!("let x = {needle}::compress_to_vec(d, 6);").contains(&needle),
        "綴りを足しても拾えないなら見張りになっていない"
    );
    assert!(
        !"miniz_oxide::inflate::decompress_to_vec_with_limit(data, limit)".contains(&needle),
        "伸長側の綴りまで拾うのでは意味が無い"
    );
}

// ---- 1,000 行（要件 10.8） ----

/// 本タスクが新設したファイルが 1,000 行未満である。
///
/// ワークスペース全体の番人は `log-capture-kit` が持つ（例外表には触れない）。
/// ここは新設分だけを自分で確かめる。
#[test]
fn the_new_files_stay_under_the_line_limit() {
    for name in ["lib.rs", "lib_tests.rs", "lib_vocabulary_tests.rs"] {
        let lines = source_of(name).lines().count();
        assert!(lines < 1000, "{name} が {lines} 行（上限 1,000）");
    }
}
