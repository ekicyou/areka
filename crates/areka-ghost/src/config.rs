//! 運行設定（`KanadeConfig`）の値源解決。
//!
//! shell descript からシェル名を読み取り、読取不能・欠落時はディレクトリ名へ
//! 安全側にフォールバックする純関数と、areka 固有のベースウェア定数を提供する
//! （design.md「ghost::config」）。

use areka_kanade::KanadeConfig;
use areka_parsers::charset::{DefaultEncoding, decode};
use areka_parsers::kv::parse_kv;
use areka_parsers::package::MountModel;

/// shell descript のファイル名（`{shell.dir}/descript.txt`）。
const SHELL_DESCRIPT_FILE: &str = "descript.txt";
/// shell descript の name キー（OnBoot Reference0＝「起動時のシェル名」・ukadoc）。
const SHELL_NAME_KEY: &str = "name";

/// `MountModel` と shell descript から `KanadeConfig` を解決する（純関数・I/O は
/// shell descript 読取のみ）。
///
/// shell 名は `MountModel.shell.dir` 直下の `descript.txt` を読み、`decode` →
/// `parse_kv` の上で `name` キーの値を採る。読取不能・`name` 欠落時は `warn!` の
/// 上で shell ディレクトリ名（通常 `"master"`）へフォールバックする——shell
/// descript は補助情報であり、boot を落とさない（design.md「ghost::config」）。
///
/// baseware 情報は areka 固有の定数から供給する: `baseware_name` は
/// `KanadeConfig::new` の既定 `"areka"`、`baseware_version` は
/// `env!("CARGO_PKG_VERSION")`（workspace 統一 version）。
pub fn resolve_kanade_config(
    mount: &MountModel,
    default_encoding: DefaultEncoding,
) -> KanadeConfig {
    let shell_name = resolve_shell_name(mount, default_encoding);
    KanadeConfig::new(shell_name, env!("CARGO_PKG_VERSION"))
}

/// shell descript から `name` キーを読み取る。読取不能・欠落は shell ディレクトリ
/// 自身のディレクトリ名（basename）へフォールバックする（`warn!` 付き）。
fn resolve_shell_name(mount: &MountModel, default_encoding: DefaultEncoding) -> String {
    let shell_dir = &mount.shell.dir;
    let fallback = || shell_dir_basename(shell_dir);

    let descript_path = shell_dir.join(SHELL_DESCRIPT_FILE);
    let bytes = match std::fs::read(&descript_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(
                path = %descript_path.display(),
                error = %err,
                "shell descript が読取不能——shell ディレクトリ名へフォールバックする"
            );
            return fallback();
        }
    };

    let text = decode(&bytes, default_encoding);
    let map = parse_kv(&text);
    match map.get(SHELL_NAME_KEY) {
        Some(name) => name.clone(),
        None => {
            tracing::warn!(
                path = %descript_path.display(),
                "shell descript に name キーが無い——shell ディレクトリ名へフォールバックする"
            );
            fallback()
        }
    }
}

/// shell ディレクトリ自身の basename（安全側フォールバック値・通常 `"master"`）。
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
    use std::path::PathBuf;

    /// このテスト専用の一意な一時ディレクトリを返す（関数名でユニーク化・衝突回避）。
    fn unique_temp_dir(tag: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_ghost_config_tests_{tag}"));
        dir
    }

    /// `root` 直下に最小限の正常なゴーストツリー（`ghost/master/descript.txt` ＋
    /// `shell/<shell_dir_name>/`）を構築し、`shell/<shell_dir_name>/descript.txt` を
    /// 任意内容で書く（`None` なら shell descript 自体を作らない＝不在ケース）。
    ///
    /// `shell_dir_name` はゴースト側 descript の `seriko.defaultsurfacedirectoryname`
    /// へそのまま渡され、`areka_parsers::package::resolve` がそれを shell dir 名として
    /// 解決する（`master` 固定にすると「フォールバック値がたまたま `master` と一致する」
    /// だけで検出力を持たない偽陽性テストになるため、呼び出し側が任意名を選べるように
    /// している）。
    ///
    /// `MountModel` は `#[non_exhaustive]` ゆえ他クレートから構造体リテラルで
    /// 直接構築できない（design 契約どおり）ため、`areka_parsers::package::resolve`
    /// を通して実物を得る（`resolve_tests.rs` の tempdir fixture 流儀を踏襲）。
    fn resolve_mount_with_shell_descript(
        root: &PathBuf,
        shell_dir_name: &str,
        shell_descript: Option<&[u8]>,
    ) -> MountModel {
        let _ = fs::remove_dir_all(root);

        let ghost_master = root.join("ghost").join("master");
        fs::create_dir_all(&ghost_master).expect("create ghost/master");
        fs::write(
            ghost_master.join("descript.txt"),
            format!("charset,UTF-8\nseriko.defaultsurfacedirectoryname,{shell_dir_name}\n")
                .as_bytes(),
        )
        .expect("write ghost descript.txt");

        let shell_dir = root.join("shell").join(shell_dir_name);
        fs::create_dir_all(&shell_dir).expect("create shell/<shell_dir_name>");
        if let Some(contents) = shell_descript {
            fs::write(shell_dir.join("descript.txt"), contents).expect("write shell descript.txt");
        }

        areka_parsers::package::resolve(root, DefaultEncoding::Utf8)
            .expect("fixture tree resolves to a MountModel")
    }

    #[test]
    fn resolve_kanade_config_reads_shell_name_from_descript() {
        // --- Arrange: shell/master/descript.txt に name,SomeShellName ---
        let root = unique_temp_dir("reads_shell_name_from_descript");
        let mount = resolve_mount_with_shell_descript(
            &root,
            "master",
            Some(b"charset,UTF-8\nname,SomeShellName\n"),
        );

        // --- Act ---
        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        // --- Assert ---
        assert_eq!(config.shell_name, "SomeShellName");
        assert_eq!(config.baseware_name, "areka");
        assert_eq!(config.baseware_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(config.close_talk_deadline_ms, 30_000);

        // --- Cleanup（best-effort）---
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_kanade_config_falls_back_to_dir_name_when_name_key_missing() {
        // --- Arrange: descript.txt は読めるが name キーが無い ---
        let root = unique_temp_dir("falls_back_when_name_key_missing");
        let mount = resolve_mount_with_shell_descript(
            &root,
            "master",
            Some(b"charset,UTF-8\nother,value\n"),
        );

        // --- Act ---
        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        // --- Assert: shell ディレクトリ自身の basename（"master"）へフォールバック ---
        assert_eq!(config.shell_name, "master");

        // --- Cleanup（best-effort）---
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_kanade_config_falls_back_to_dir_name_when_descript_missing() {
        // --- Arrange: shell/master/descript.txt が存在しない ---
        let root = unique_temp_dir("falls_back_when_descript_missing");
        let mount = resolve_mount_with_shell_descript(&root, "master", None);

        // --- Act ---
        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        // --- Assert: shell ディレクトリ自身の basename（"master"）へフォールバック ---
        assert_eq!(config.shell_name, "master");

        // --- Cleanup（best-effort）---
        let _ = fs::remove_dir_all(&root);
    }

    /// フォールバック値が「たまたま `master` と一致する」偽陽性を防ぐための決め手テスト:
    /// shell ディレクトリ名を非 `master`（`customshell`）にし、フォールバックが実際に
    /// そのディレクトリ basename を返す（ハードコードされた `"master"` ではない）ことを
    /// 証明する。ゴースト側 descript の `seriko.defaultsurfacedirectoryname` を
    /// `customshell` に指定し、`shell/customshell/` は作るが descript.txt は置かない
    /// （不在フォールバック経路を再利用）。
    #[test]
    fn resolve_kanade_config_falls_back_to_non_master_dir_name() {
        // --- Arrange: shell ディレクトリ名は customshell（master ではない）---
        let root = unique_temp_dir("falls_back_to_non_master_dir_name");
        let mount = resolve_mount_with_shell_descript(&root, "customshell", None);

        // --- Act ---
        let config = resolve_kanade_config(&mount, DefaultEncoding::Utf8);

        // --- Assert: フォールバック値は実ディレクトリ名 "customshell"（"master" 固定ではない）---
        assert_eq!(config.shell_name, "customshell");
        assert_ne!(config.shell_name, "master");

        // --- Cleanup（best-effort）---
        let _ = fs::remove_dir_all(&root);
    }
}
