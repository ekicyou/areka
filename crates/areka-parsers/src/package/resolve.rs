//! resolve — ツリー解決 + I/O（parser ファミリ内で唯一の I/O 保有点）。
//!
//! 展開済みゴーストパッケージのルートから `ghost/master/descript.txt` を起点に
//! SHIORI / shell の 2 点マウントを解決する。解決の本体はシェル名つきの解決
//! `resolve_shell::resolve_with_shell` にあり、`resolve` は名前なし（今日の規則＝
//! `seriko.defaultsurfacedirectoryname`、無ければ `master`）でそこへ委ねる。
//! shell descript.txt からの bindgroup 転記（`read_bindgroup_defaults`）は兄弟の
//! `resolve_bind` に置き、本体から呼ばれる。charset 判定・KV 分割は foundation へ委譲し再実装しない（Req 1.5）。

use std::path::Path;

use crate::charset::DefaultEncoding;

use super::model::{MountError, MountModel};
use super::resolve_shell::resolve_with_shell;

/// 展開済みゴーストパッケージのルートから、descript.txt 起点で
/// SHIORI/shell の 2 点マウントを解決する（名前なし＝今日の既定のシェル）。
///
/// - `ghost_root`: 展開済みゴーストパッケージのルート（`ghost/` `shell/` を含む階層）。
/// - `default_encoding`: descript.txt に `charset` 宣言が無い場合に用いる既定エンコード
///   （呼び出し側が指定する・SSP 準拠の既定は ANSI）。
///
/// 成功時 `MountModel`、致命的欠落（起点不在・起点読取不能・shell dir 不在）時
/// `MountError` を返す。
pub fn resolve(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
) -> Result<MountModel, MountError> {
    resolve_with_shell(ghost_root, default_encoding, None)
}

#[cfg(test)]
mod ghost_names_transcription_tests {
    //! `resolve` の `GhostNames` 転記（task 7・`sakura.name2` 追加）の in-source 檻。
    //!
    //! ghost/master/descript.txt を合成 tempdir に書き出し、`resolve` を直接呼んで
    //! `sakura.name2` の忠実転記（宣言あり→Some・宣言なし→None）を決定論で固定する。
    //! 縮退・フォールバックは行わない（それは ghost 層 task 8.1 の責務）。一時パスは
    //! 共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意）。

    use std::fs;

    use temp_path_kit::TempPath;

    use crate::charset::DefaultEncoding;

    use super::resolve;

    /// テスト専用の一意な ghost ルートを作り、descript.txt と shell/master を用意して返す。
    ///
    /// 一時ディレクトリは共通窓口 `temp-path-kit` 経由で組むので、名前にプロセス識別子と
    /// 連番が入り**プロセス間でも一意**になる。返り値が生きている間だけ実体が存在し、
    /// 破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
    fn ghost_root_with_descript(tag: &str, descript_body: &str) -> TempPath {
        // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
        let root = TempPath::new(&format!("parsers-ghost-names-{}", tag.replace('_', "-")));
        let master = root.path().join("ghost").join("master");
        fs::create_dir_all(&master).expect("create ghost/master");
        fs::write(master.join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        // shell/master は存在確認（Req 3.3）を通すために用意する。
        fs::create_dir_all(root.path().join("shell").join("master")).expect("create shell/master");
        root
    }

    /// `sakura.name2` 宣言あり → `GhostNames.sakura_name2 == Some(値)`（忠実転記・R4.4）。
    #[test]
    fn sakura_name2_declared_is_some() {
        let temp = ghost_root_with_descript(
            "name2_declared",
            "charset,UTF-8\nname,テスト\nsakura.name,本体\nsakura.name2,別名\nkero.name,相方\n",
        );
        let root = temp.path().to_path_buf();
        let model = resolve(&root, DefaultEncoding::Utf8).expect("resolve ok");

        assert_eq!(model.names.sakura_name2, Some("別名".to_string()));
        // 既存フィールドは無改変。
        assert_eq!(model.names.sakura_name, Some("本体".to_string()));
    }

    /// `sakura.name2` 宣言なし → `GhostNames.sakura_name2 == None`（推測しない・R4.4）。
    #[test]
    fn sakura_name2_absent_is_none() {
        let temp = ghost_root_with_descript(
            "name2_absent",
            "charset,UTF-8\nname,テスト\nsakura.name,本体\nkero.name,相方\n",
        );
        let root = temp.path().to_path_buf();
        let model = resolve(&root, DefaultEncoding::Utf8).expect("resolve ok");

        assert_eq!(model.names.sakura_name2, None);
        // 兄弟フィールドは通常どおり転記される（None は欠落を意味する）。
        assert_eq!(model.names.sakura_name, Some("本体".to_string()));
    }
}
