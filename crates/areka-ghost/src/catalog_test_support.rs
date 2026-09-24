//! `catalog` のテストが一時フォルダに根を組むヘルパ（要件 8.1・design「File Structure Plan」）。
//!
//! 組むのは `temp-path-kit` の一時フォルダの下だけで、検体や作業ツリーには書かない。

use super::*;
use std::fs;
use temp_path_kit::TempPath;

/// `path` に `body` を書く（親フォルダも作る）。
pub(super) fn put(path: &Path, body: &[u8]) {
    fs::create_dir_all(path.parent().expect("親")).expect("親フォルダ作成");
    fs::write(path, body).expect("書き込み");
}

/// `<根>/ghost/<folder>/ghost/master/descript.txt` を置き、ゴーストのフォルダを返す。
pub(super) fn put_ghost(root: &BasewareRoot, folder: &str, descript: &str) -> PathBuf {
    let dir = root.ghost_dir(folder);
    put(
        &dir.join("ghost").join("master").join("descript.txt"),
        descript.as_bytes(),
    );
    dir
}

/// `Shift_JIS` 宣言の descript（`name`＝「吹き出し」・`craftmanw`＝「作者」を生バイトで・要件 2.8）。
fn sjis_descript() -> Vec<u8> {
    let mut sjis = b"charset,Shift_JIS\nname,".to_vec();
    sjis.extend_from_slice(&[0x90, 0x81, 0x82, 0xAB, 0x8F, 0x6F, 0x82, 0xB5]);
    sjis.extend_from_slice(b"\ncraftman,stayse\ncraftmanw,");
    sjis.extend_from_slice(&[0x8D, 0xEC, 0x8E, 0xD2]);
    sjis.extend_from_slice(b"\nid,StayseeBalloon\n");
    sjis
}

/// 要件 8.1 の構成の一時の根。`_tmp` を束縛している間だけ実在する。
///
/// - `ghost/`: `alpha`（素性 7 項目が全部ある・install.txt で `kaku` を同梱・シェル 2 つの
///   うち `hidden` は `menu,hidden`）・`beta`（素性はほぼ無し）・`empty`（descript の無い
///   フォルダ）・`stray.txt`（フォルダ以外の項目）
/// - `balloon/`: `StayseeBalloon`（`type` 無し・`Shift_JIS` の偽の既定バルーン）・`kaku`
///   （`type,balloon`）・`plugin`（`type,plugin`）・`empty`（descript の無いフォルダ）
pub(super) struct TempRoot {
    pub _tmp: TempPath,
    pub root: BasewareRoot,
}

pub(super) fn temp_root(label: &str) -> TempRoot {
    let tmp = TempPath::new(label);
    let root = BasewareRoot::new(tmp.path().to_path_buf());

    let alpha = put_ghost(
        &root,
        "alpha",
        "charset,UTF-8\nname,Alpha\ncraftman,maker\ncraftmanw,作り手\nid,AlphaId\nreadme,manual.txt\n",
    );
    put(&alpha.join("manual.txt"), b"m");
    put(&alpha.join("thumbnail.png"), b"p");
    put(
        &alpha.join("install.txt"),
        b"charset,UTF-8\nballoon.directory,kaku\n",
    );
    put(
        &alpha.join("shell").join("master").join("descript.txt"),
        b"charset,UTF-8\nname,Shell\n",
    );
    put(&alpha.join("shell").join("master").join("readme.txt"), b"r");
    put(
        &alpha.join("shell").join("hidden").join("descript.txt"),
        b"charset,UTF-8\nname,Hidden\nmenu,hidden\n",
    );
    put_ghost(&root, "beta", "charset,UTF-8\nname,Beta\n");
    fs::create_dir_all(root.ghost_dir("empty").join("ghost").join("master")).expect("empty");
    put(&root.ghost_store().join("stray.txt"), b"x");

    let staysee = root.balloon_dir("StayseeBalloon");
    put(&staysee.join("descript.txt"), &sjis_descript());
    put(&staysee.join("readme.txt"), b"r");
    put(
        &root.balloon_dir("kaku").join("descript.txt"),
        b"charset,UTF-8\ntype,balloon\nname,Kaku\n",
    );
    put(
        &root.balloon_dir("plugin").join("descript.txt"),
        b"charset,UTF-8\ntype,plugin\nname,Plugin\n",
    );
    fs::create_dir_all(root.balloon_dir("empty")).expect("empty");

    TempRoot { _tmp: tmp, root }
}
