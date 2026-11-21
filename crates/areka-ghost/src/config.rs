//! 運行設定（`KanadeConfig`）の値源解決。
//!
//! shell descript からシェル名を読み、読取不能・欠落時はディレクトリ名へフォールバックする。
//! areka 固有のベースウェア定数も供給する。

use areka_kanade::KanadeConfig;
use areka_parsers::charset::DefaultEncoding;
use areka_parsers::package::MountModel;
use areka_sakura::strict::{StrictHooks, StrictJudges};
use std::path::Path;
use std::sync::Arc;

/// shell descript のファイル名（`{shell.dir}/descript.txt`）。
const SHELL_DESCRIPT_FILE: &str = "descript.txt";

/// `MountModel` と shell descript から `KanadeConfig` を解決する（I/O は shell descript 読取のみ）。
///
/// shell 名は `MountModel.shell.dir` 直下の `descript.txt` を一覧と同じ表（鍵の大小を
/// 無視・空の値は無し）で読み、`name` の値を採る。読取不能・`name` 欠落時は `warn!` の
/// 上で shell ディレクトリ名（通常 `"master"`）へフォールバックする（boot を落とさない）。
///
/// `baseware_name` は `KanadeConfig::new` の既定 `"areka"`、`baseware_version` は
/// `env!("CARGO_PKG_VERSION")`。
pub fn resolve_kanade_config(
    mount: &MountModel,
    default_encoding: DefaultEncoding,
) -> KanadeConfig {
    let shell_name = resolve_shell_name(mount, default_encoding);
    KanadeConfig {
        ghost_name: listed_ghost_name(mount),
        ..KanadeConfig::new(shell_name, env!("CARGO_PKG_VERSION"))
    }
}

/// 一覧に出すゴーストの名（script の記録・MCP の `ghost_name` の両方がこれを使う）。`name` が
/// 空でなければそれ、無い・空なら根の絶対パスから末尾の区切りを除いた字。
pub fn listed_name(name: Option<&str>, ghost_root: &Path) -> String {
    if let Some(name) = name.filter(|n| !n.is_empty()) {
        return name.to_string();
    }
    std::path::absolute(ghost_root)
        .unwrap_or_else(|_| ghost_root.to_path_buf())
        .display()
        .to_string()
        .trim_end_matches(['\\', '/'])
        .to_string()
}

/// script 種別のログに出すゴーストの名前（根は `ghost/master` の 2 つ上）。
fn listed_ghost_name(mount: &MountModel) -> String {
    let master = &mount.shiori.dir;
    let root = master.ancestors().nth(2).unwrap_or(master);
    listed_name(mount.names.name.as_deref(), root)
}

/// strict の記録の束ね。名は `script` の記録と同じ
/// [`listed_ghost_name`] で作るので `get_log` の `ghost_name` の絞り込みに当たる。判定の差し込みが無ければ束ねない。
pub(crate) fn strict_hooks(
    mount: &MountModel,
    judges: Option<Arc<dyn StrictJudges>>,
) -> Option<StrictHooks> {
    judges.map(|judges| StrictHooks {
        ghost: listed_ghost_name(mount).into(),
        judges,
    })
}

/// shell descript から `name` キーを読む。読取不能・欠落は shell ディレクトリの basename へ
/// フォールバックする（`warn!` 付き）。
fn resolve_shell_name(mount: &MountModel, default_encoding: DefaultEncoding) -> String {
    let shell_dir = &mount.shell.dir;
    let descript_path = shell_dir.join(SHELL_DESCRIPT_FILE);

    // OnBoot Reference0 の「起動時のシェル名」。一覧（`catalog`）と同じ読み手を使う。
    // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#name_2c_30b7_30a7_30eb_540d:1
    match crate::catalog::descript_name_in(shell_dir, default_encoding) {
        Ok(Some(name)) => name,
        Err(err) => {
            tracing::warn!(
                event = "shell_descript_unreadable",
                path = %descript_path.display(),
                error = %err,
                "shell descript が読取不能——shell ディレクトリ名へフォールバックする"
            );
            shell_dir_basename(shell_dir)
        }
        Ok(None) => {
            tracing::warn!(
                event = "shell_name_missing",
                path = %descript_path.display(),
                "shell descript の name が無いか空——shell ディレクトリ名へフォールバックする"
            );
            shell_dir_basename(shell_dir)
        }
    }
}

/// shell ディレクトリの basename（フォールバック値・通常 `"master"`）。
fn shell_dir_basename(shell_dir: &std::path::Path) -> String {
    shell_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use temp_path_kit::TempPath;

    /// `temp-path-kit` の一時ディレクトリ（プロセス間でも一意）。返り値が生きている間だけ実体が在り、
    /// 破棄で中身ごと消える。
    fn unique_temp_dir(tag: &str) -> TempPath {
        TempPath::new(&format!("ghost-config-{tag}"))
    }

    /// `root` 直下に最小限のゴーストツリー（`ghost/master/descript.txt` ＋ `shell/<shell_dir_name>/`）を
    /// 組み、`shell/<shell_dir_name>/descript.txt` を任意内容で書く（`None` なら作らない）。
    /// `ghost_line`（例 `"name,えみり"`）はゴースト側 descript に足す行。
    ///
    /// `shell_dir_name` はゴースト側 descript の `seriko.defaultsurfacedirectoryname` へ渡る。
    /// `master` 固定だとフォールバック値と偶然一致して検出力を失うので、呼び手が選べる。
    /// `MountModel` は `#[non_exhaustive]` で直接構築できないので `resolve_with_shell` を通す。
    fn resolve_mount(
        root: &Path,
        ghost_line: Option<&str>,
        shell_dir_name: &str,
        shell_descript: Option<&[u8]>,
    ) -> MountModel {
        let ghost_master = root.join("ghost").join("master");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        let ghost_line = ghost_line.map(|l| format!("{l}\n")).unwrap_or_default();
        fs::write(
            ghost_master.join("descript.txt"),
            format!(
                "charset,UTF-8\nseriko.defaultsurfacedirectoryname,{shell_dir_name}\n{ghost_line}"
            )
            .as_bytes(),
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join(shell_dir_name);
        fs::create_dir_all(&shell_dir).expect("create shell/<shell_dir_name>");
        if let Some(contents) = shell_descript {
            fs::write(shell_dir.join("descript.txt"), contents).expect("write shell descript.txt");
        }

        areka_parsers::package::resolve_with_shell(root, DefaultEncoding::Utf8, None)
            .expect("fixture tree resolves to a MountModel")
    }

    /// ゴースト側 descript に `name_line` を足し、shell descript は置かない（shell は `master`）。
    fn resolve_mount_with_ghost_name(root: &Path, name_line: Option<&str>) -> MountModel {
        resolve_mount(root, name_line, "master", None)
    }

    /// ゴースト側 descript には足さず、shell descript を `shell_descript` で置く。
    fn resolve_mount_with_shell_descript(
        root: &Path,
        shell_dir_name: &str,
        shell_descript: Option<&[u8]>,
    ) -> MountModel {
        resolve_mount(root, None, shell_dir_name, shell_descript)
    }

    #[test]
    fn resolve_kanade_config_sets_ghost_name_from_descript_name() {
        let temp = unique_temp_dir("ghost-name-from-descript");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_ghost_name(&root, Some("name,えみり"));

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.ghost_name, "えみり");
    }

    #[test]
    fn resolve_kanade_config_sets_ghost_name_to_root_path_when_name_missing() {
        let temp = unique_temp_dir("ghost-name-missing");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_ghost_name(&root, None);

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.ghost_name, listed_name(None, &root));
        assert!(std::path::Path::new(&config.ghost_name).is_absolute());
        assert!(!config.ghost_name.ends_with(['\\', '/']));
    }

    /// 判定を入れない（記録の名だけを見る）。
    struct NoJudges;

    impl StrictJudges for NoJudges {
        fn command_handled(&self, _name: &str, _first: Option<&str>) -> bool {
            true
        }
        fn font(&self, _args: &[String]) -> Option<areka_sakura::strict::DetectionPoint> {
            None
        }
    }

    /// strict の記録の名は `script` の記録（`KanadeConfig.ghost_name`）と同じ一覧の名になる
    /// （descript の名がある時も、無くて根のパスになる時も）。
    #[test]
    fn strict_hooks_name_the_ghost_as_listed() {
        for (tag, name_line) in [("strict-name", Some("name,えみり")), ("strict-path", None)] {
            let temp = unique_temp_dir(tag);
            let root = temp.path().to_path_buf();
            let mount = resolve_mount_with_ghost_name(&root, name_line);
            let listed = resolve_kanade_config(&mount, DefaultEncoding::Utf8).ghost_name;

            let hooks = strict_hooks(&mount, Some(Arc::new(NoJudges))).expect("judges given");

            assert_eq!(&*hooks.ghost, listed);
            let want = match name_line {
                Some(_) => "えみり".to_string(),
                None => listed_name(None, &root),
            };
            assert_eq!(&*hooks.ghost, want);
        }
    }

    #[test]
    fn strict_hooks_are_absent_without_judges() {
        let temp = unique_temp_dir("strict-none");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_ghost_name(&root, Some("name,えみり"));

        assert!(strict_hooks(&mount, None).is_none());
    }

    #[test]
    fn resolve_kanade_config_sets_ghost_name_to_root_path_when_name_empty() {
        let temp = unique_temp_dir("ghost-name-empty");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_ghost_name(&root, Some("name,"));

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.ghost_name, listed_name(None, &root));
        assert!(!config.ghost_name.ends_with(['\\', '/']));
    }

    #[test]
    fn resolve_kanade_config_reads_shell_name_from_descript() {
        let temp = unique_temp_dir("reads-shell-name-from-descript");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_shell_descript(
            &root,
            "master",
            Some(b"charset,UTF-8\nname,SomeShellName\n"),
        );

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.shell_name, "SomeShellName");
        assert_eq!(config.baseware_name, "areka");
        assert_eq!(config.baseware_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(config.close_talk_deadline_ms, 30_000);
    }

    /// シェルの descript の鍵は一覧と同じく大小を無視して読む。
    #[test]
    fn shell_name_key_is_read_case_insensitively() {
        let temp = unique_temp_dir("shell-name-key-case");
        let root = temp.path().to_path_buf();
        let mount =
            resolve_mount_with_shell_descript(&root, "master", Some(b"charset,UTF-8\nName,Foo\n"));

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.shell_name, "Foo");
    }

    /// 同じ鍵が大小の違う綴りで 2 回出たら、後の行が勝つ（parsers の表）。
    #[test]
    fn a_later_shell_name_row_wins_whatever_the_spelling() {
        let temp = unique_temp_dir("shell-name-later-row");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_shell_descript(
            &root,
            "master",
            Some(b"charset,UTF-8\nname,Early\nName,Later\n"),
        );

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.shell_name, "Later");
    }

    /// 空の `name,` は一覧と同じく「無い」なので、フォルダ名へ倒す。
    #[test]
    fn an_empty_shell_name_falls_back_to_the_folder_name() {
        let temp = unique_temp_dir("shell-name-empty");
        let root = temp.path().to_path_buf();
        let mount =
            resolve_mount_with_shell_descript(&root, "custom", Some(b"charset,UTF-8\nname,\n"));

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.shell_name, "custom");
    }

    /// 名が空でなければそれ、無い・空なら根の絶対パスから末尾の区切りを除いた字。
    #[test]
    fn listed_name_falls_back_to_the_absolute_root_without_a_trailing_separator() {
        assert_eq!(listed_name(Some("Emily"), Path::new(r"C:\x\")), "Emily");
        assert_eq!(listed_name(None, Path::new(r"C:\a\b\")), r"C:\a\b");
        assert_eq!(listed_name(Some(""), Path::new(r"C:\a\b/")), r"C:\a\b");
        let cwd = std::env::current_dir().expect("cwd");
        assert_eq!(
            listed_name(None, Path::new(r"rel\dir\")),
            cwd.join(r"rel\dir").display().to_string()
        );
    }

    #[test]
    fn resolve_kanade_config_falls_back_to_dir_name_when_name_key_missing() {
        let temp = unique_temp_dir("falls-back-when-name-key-missing");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_shell_descript(
            &root,
            "keyless",
            Some(b"charset,UTF-8\nother,value\n"),
        );

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.shell_name, "keyless");
    }

    /// shell ディレクトリ名を `master` 以外（`customshell`）にして、フォールバックが固定の
    /// `"master"` でなく実際の basename を返すことを固定する（descript.txt は置かない）。
    #[test]
    fn resolve_kanade_config_falls_back_to_non_master_dir_name() {
        let temp = unique_temp_dir("falls-back-to-non-master-dir-name");
        let root = temp.path().to_path_buf();
        let mount = resolve_mount_with_shell_descript(&root, "customshell", None);

        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        assert_eq!(config.shell_name, "customshell");
        assert_ne!(config.shell_name, "master");
    }
}
