//! 道具と保管した書庫のずれの見張り（spec: default-balloon-selfmade 要件 8.2・設計で決めたこと 10）。
//!
//! 道具 `gen_default_balloon` の組み立て（`build`）と子の 3 つを `#[path]` の兄弟として読み、
//! 組んだ 23 ファイルと、窓口から取り出した `areka-balloon` の木の全ファイルを名前の集合と
//! バイト列で比べる。道具を直して書庫を作り直し忘れた・`image` の版が上がって出力が
//! 変わったときに赤になる。直し方は道具を回して書庫を作り直すこと。
//!
//! 道具の子のファイルに在るモジュール内のテストも、このターゲットで一緒に走る。

#[path = "../examples/gen_default_balloon/build.rs"]
mod build;
#[path = "../examples/gen_default_balloon/draw.rs"]
mod draw;
#[path = "../examples/gen_default_balloon/texts.rs"]
mod texts;
#[path = "../examples/gen_default_balloon/webp.rs"]
mod webp;

use sample_ghost_kit::SampleRoot;
use std::collections::BTreeMap;
use std::path::Path;

/// `dir` の下の全ファイルを（`/` 区切りの相対名, バイト列）で集める。
fn read_tree(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).expect("取り出した木を読めない") {
        let path = entry.expect("取り出した木の項目を読めない").path();
        if path.is_dir() {
            read_tree(base, &path, out);
        } else {
            let name = path
                .strip_prefix(base)
                .expect("木の下のパス")
                .to_string_lossy()
                .replace('\\', "/");
            out.insert(
                name,
                std::fs::read(&path).expect("取り出したファイルを読めない"),
            );
        }
    }
}

#[test]
fn stored_archive_matches_the_tool_output() {
    let built: BTreeMap<String, Vec<u8>> = build::build()
        .expect("道具の組み立てが失敗した")
        .into_iter()
        .collect();
    assert_eq!(built.len(), 23, "道具の組み立ては 23 ファイル");

    let sample = SampleRoot::acquire("areka-balloon").expect("areka-balloon は登記済みの検体");
    let mut stored = BTreeMap::new();
    read_tree(sample.folder(), sample.folder(), &mut stored);

    let only_built: Vec<&String> = built.keys().filter(|n| !stored.contains_key(*n)).collect();
    let only_stored: Vec<&String> = stored.keys().filter(|n| !built.contains_key(*n)).collect();
    assert!(
        only_built.is_empty() && only_stored.is_empty(),
        "ファイルの集合がずれている（道具を回して書庫を作り直す）: 道具だけ {only_built:?}・書庫だけ {only_stored:?}"
    );
    let differing: Vec<&String> = built
        .iter()
        .filter(|(name, bytes)| stored[*name] != **bytes)
        .map(|(name, _)| name)
        .collect();
    assert!(
        differing.is_empty(),
        "書庫の中身が道具の今の出力と違う（道具を回して書庫を作り直す）: {differing:?}"
    );
}
