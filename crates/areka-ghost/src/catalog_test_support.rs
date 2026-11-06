//! `catalog` のテストが一時フォルダに根を組むヘルパ（要件 8.1・design「File Structure Plan」）。
//!
//! 組むのは `temp-path-kit` の一時フォルダの下だけで、検体や作業ツリーには書かない。
//! `catalog` と `catalog_roots` のテストが共に使う fixture の道具はここに 1 つずつ置く。

use super::*;
use crate::catalog_roots::BasewareRoots;
use crate::test_log_capture::CapturedEvent;
use std::fs;
use temp_path_kit::TempPath;
use tracing::Level;

/// `catalog` の記録の target。
pub(crate) const TARGET: &str = "areka_ghost::catalog";

/// `path` に `body` を書く（親フォルダも作る）。
pub(crate) fn put(path: &Path, body: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("親")).expect("親フォルダ作成");
    fs::write(path, body).expect("書き込み");
}

/// `<store>/<folder>/ghost/master/descript.txt` に `descript` を置き、ゴーストのフォルダを返す。
pub(crate) fn put_ghost_in(store: &Path, folder: &str, descript: impl AsRef<[u8]>) -> PathBuf {
    let dir = store.join(folder);
    put(
        &dir.join("ghost").join("master").join("descript.txt"),
        descript,
    );
    dir
}

/// `<base>/balloon/<folder>/descript.txt` に `name,<name>` を置き、バルーンのフォルダを返す。
pub(crate) fn put_balloon(base: &Path, folder: &str, name: &str) -> PathBuf {
    let dir = base.join("balloon").join(folder);
    put(&dir.join("descript.txt"), format!("name,{name}\r\n"));
    dir
}

/// 見つかる 2 つの根 `a`（上）・`b`（下）の並び。
pub(crate) fn two_roots(label: &str) -> (TempPath, PathBuf, PathBuf, BasewareRoots) {
    let tmp = TempPath::under_target(label);
    let a = tmp.child("a");
    let b = tmp.child("b");
    fs::create_dir_all(&a).unwrap();
    fs::create_dir_all(&b).unwrap();
    let roots = BasewareRoots::new(vec![
        BasewareRoot::new(a.clone()),
        BasewareRoot::new(b.clone()),
    ]);
    (tmp, a, b, roots)
}

/// 捕捉列のうち `catalog` の `warn!` の `event` を並びのまま返す。
pub(crate) fn catalog_warns(events: &[CapturedEvent]) -> Vec<&str> {
    events
        .iter()
        .filter(|e| e.target == TARGET && e.level == Level::WARN)
        .map(|e| e.event.as_deref().unwrap_or("<event 無し>"))
        .collect()
}

/// 他者が読めないよう共有なしで開いたまま持つ（`fs::read` を I/O 失敗させる・有無の検査は通る）。
pub(super) fn hold_exclusive(path: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .expect("共有なしで開く")
}

/// `Shift_JIS` 宣言の descript（`name`＝「吹き出し」・`craftmanw`＝「作者」を生バイトで・要件 2.8）。
pub(super) fn sjis_descript() -> Vec<u8> {
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

    let alpha = put_ghost_in(
        &root.ghost_store(),
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
    put_ghost_in(&root.ghost_store(), "beta", "charset,UTF-8\nname,Beta\n");
    fs::create_dir_all(root.ghost_dir("empty").join("ghost").join("master")).expect("empty");
    put(&root.ghost_store().join("stray.txt"), b"x");

    let staysee = root.balloon_dir("StayseeBalloon");
    put(&staysee.join("descript.txt"), sjis_descript());
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
