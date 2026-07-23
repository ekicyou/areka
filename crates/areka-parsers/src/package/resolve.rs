//! resolve — ツリー解決 + I/O（parser ファミリ内で唯一の I/O 保有点）。
//!
//! 展開済みゴーストパッケージのルートから `ghost/master/descript.txt` を起点に
//! SHIORI / shell の 2 点マウントを解決する。制御フローは design.md「System Flows」
//! の mermaid に忠実（パス合成 → 存在 → 読取 → decode → kv → names → shiori →
//! shelldir → shell 存在確認 → 構築）。charset 判定・KV 分割は foundation へ委譲し
//! 再実装しない（Req 1.5）。参照キーは name / sakura.name / kero.name / shiori /
//! seriko.defaultsurfacedirectoryname のみ（install.txt / balloon 系 / NAR には触れない）。

use std::io::ErrorKind;
use std::path::Path;

use crate::charset::{decode, DefaultEncoding};
use crate::kv::parse_kv;

use super::model::{
    BindGroupDefaults, BindGroupName, GhostNames, MountError, MountModel, ShellMount, ShioriMount,
};

/// SHIORI マウント先（= 起点 descript.txt の親・Req 2.1）。
const GHOST_MASTER: &str = "ghost/master";
/// 起点定義ファイル名（Req 1.1）。
const DESCRIPT_FILE: &str = "descript.txt";
/// shell ルート（Req 3.1/3.2）。
const SHELL_ROOT: &str = "shell";
/// shell 既定ディレクトリ名（ukadoc 正典・Req 3.1）。
const DEFAULT_SHELL_DIR: &str = "master";

/// 展開済みゴーストパッケージのルートから、descript.txt 起点で
/// SHIORI/shell の 2 点マウントを解決する。
///
/// - `ghost_root`: 展開済みゴーストパッケージのルート（`ghost/` `shell/` を含む階層）。
/// - `default_encoding`: descript.txt に `charset` 宣言が無い場合に用いる既定エンコード。
///   本 module は既定をハードコードせず（固定 Utf8 はレガシー ANSI ゴーストを誤読する）、
///   呼び出し側が指定する（SSP 準拠の既定は ANSI）。非 UTF-8 拒否のエンフォースは
///   下流の SHIORI 層（設計ディスカッション #1）。
///
/// 成功時 `MountModel`、致命的欠落（起点不在・起点読取不能・shell dir 不在）時
/// `MountError` を返す。
pub fn resolve(
    ghost_root: &Path,
    default_encoding: DefaultEncoding,
) -> Result<MountModel, MountError> {
    // ghost/master（SHIORI マウント先）と起点 descript.txt のパスを合成。
    let shiori_dir = ghost_root.join(GHOST_MASTER);
    let descript = shiori_dir.join(DESCRIPT_FILE);

    // 起点 descript.txt を読む。不在は StartPointMissing、その他 I/O 失敗は
    // StartPointUnreadable（黙って空を返さない・Req 1.6/5.1）。
    let bytes = match std::fs::read(&descript) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(MountError::StartPointMissing { expected: descript });
        }
        Err(err) => {
            return Err(MountError::StartPointUnreadable {
                path: descript,
                kind: err.kind(),
            });
        }
    };

    // charset 判定 + デコード + KV 分割は foundation へ委譲（Req 1.5）。
    let text = decode(&bytes, default_encoding);
    let map = parse_kv(&text);

    // 名前情報（欠落は None・推測しない・Req 1.4）。
    let names = GhostNames {
        name: map.get("name").cloned(),
        sakura_name: map.get("sakura.name").cloned(),
        kero_name: map.get("kero.name").cloned(),
    };

    // SHIORI マウント: dir は起点の親（存在確定）、file は未指定なら None（推測禁止・Req 2.3）。
    let shiori = ShioriMount {
        dir: shiori_dir,
        file: map.get("shiori").cloned(),
    };

    // shell マウント: 指定名 or 既定 master（Req 3.1/3.2）→ 物理存在確認（Req 3.3）。
    let shell_name = map
        .get("seriko.defaultsurfacedirectoryname")
        .map(String::as_str)
        .unwrap_or(DEFAULT_SHELL_DIR);
    let shell_dir = ghost_root.join(SHELL_ROOT).join(shell_name);
    if !shell_dir.is_dir() {
        return Err(MountError::ShellDirMissing { expected: shell_dir });
    }
    // shell descript.txt から bindgroup default を転記（Req 4.5）。bindgroup default
    // （`sakura.bindgroup*.default,数値`／`kero.*`）は ukadoc カテゴリ `descript_shell`
    // に属し、起点 ghost/master/descript.txt ではなく **shell の descript.txt** に定義
    // される。shell descript は存在確定していない（shell dir の存在のみ Req 3.3 で確定）
    // ため、読取不能・不在は致命ではなく空の bindgroup として扱う（既存 name 系経路や
    // マウント成立を壊さない・転記のみ・展開しない）。
    let bindgroups = read_bindgroup_defaults(&shell_dir, default_encoding);

    let shell = ShellMount { dir: shell_dir };

    Ok(MountModel {
        names,
        shiori,
        shell,
        bindgroups,
    })
}

/// bindgroup 番号を含むキーが `default,1` のときに番号を取り出す接頭辞集合。
const SAKURA_BINDGROUP_PREFIX: &str = "sakura.bindgroup";
const KERO_BINDGROUP_PREFIX: &str = "kero.bindgroup";
/// bindgroup 番号キーの接尾辞（`sakura.bindgroupNNNN.default` の末尾）。
const BINDGROUP_DEFAULT_SUFFIX: &str = ".default";
/// bindgroup 名前宣言キーの接尾辞（`sakura.bindgroupNNNN.name` の末尾・task 1.2）。
const BINDGROUP_NAME_SUFFIX: &str = ".name";
/// shell 定義ファイル名（ghost/master と同名だが所在が異なる）。
const SHELL_DESCRIPT_FILE: &str = "descript.txt";

/// 解決済み shell ディレクトリの descript.txt から bindgroup の default と name を転記する。
///
/// 同一走査で 2 種のサフィックスを転記する（task 1.2）:
/// - `<prefix>NNNN.default`: **値が `1`**（= 起動時オン）のものについて `NNNN` を u32
///   として本体／相方スコープ別に収集する（従来挙動・無改変・R1.6）。
/// - `<prefix>NNNN.name`: 値 `カテゴリ名,パーツ名,サムネイル名` を `splitn(3, ',')` で
///   再分割・各フィールド trim し、`BindGroupName` として本体／相方スコープ別に転記する。
///   パーツ名（第 2 フィールド）欠落/空の行は `warn!` の上転記対象外（正典上不完全・R1.5）。
///
/// **転記のみ・展開しない**（範囲展開・surface 解決はしない・parsers 転写層原則）。
/// 走査は `parse_kv` の `BTreeMap` 反復順（キー昇順）で決定的。名前は転記順（キー昇順）に
/// push され、重複 (カテゴリ, パーツ) はアクセサ側の後勝ち（`resolve_name` の逆順走査・D2）
/// と整合する。shell descript が不在・読取不能なら空を返す（致命でない）。
fn read_bindgroup_defaults(shell_dir: &Path, default_encoding: DefaultEncoding) -> BindGroupDefaults {
    let descript = shell_dir.join(SHELL_DESCRIPT_FILE);
    // shell descript は存在確定していない。読めなければ空（bindgroup 定義なしと同義）。
    let Ok(bytes) = std::fs::read(&descript) else {
        return BindGroupDefaults::default();
    };

    let text = decode(&bytes, default_encoding);
    let map = parse_kv(&text);

    let mut defaults = BindGroupDefaults::default();
    for (key, value) in &map {
        // --- .default 経路（従来挙動・値 "1" のみ・無改変・R1.6）---
        if value == "1" {
            if let Some(id) = parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX) {
                defaults.sakura_default_on.push(id);
                continue;
            } else if let Some(id) = parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX) {
                defaults.kero_default_on.push(id);
                continue;
            }
        }
        // --- .name 経路（task 1.2・値はカテゴリ,パーツ[,サムネ] 文字列）---
        if let Some(id) = parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX)
            && let Some(name) = parse_bindgroup_name(id, value)
        {
            defaults.sakura_names.push(name);
        } else if let Some(id) = parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX)
            && let Some(name) = parse_bindgroup_name(id, value)
        {
            defaults.kero_names.push(name);
        }
    }
    defaults
}

/// `<prefix>NNNN<suffix>` 形のキーから bindgroup 番号 `NNNN`（u32）を取り出す。
/// 形が一致しない／番号が u32 でパースできないキーは `None`（転記対象外）。
fn parse_bindgroup_id(key: &str, prefix: &str, suffix: &str) -> Option<u32> {
    let rest = key.strip_prefix(prefix)?;
    let number = rest.strip_suffix(suffix)?;
    number.parse::<u32>().ok()
}

/// `.name` の値 `カテゴリ名,パーツ名,サムネイル名` を `BindGroupName` へ忠実転記する。
///
/// `splitn(3, ',')` で最大 3 分割し各フィールドを trim する（第 3 フィールドは残余全部＝
/// サムネイル名として不透明保持）。第 2 フィールド（パーツ名）が欠落／trim 後に空なら
/// **転記対象外**（`warn!` を残し `None` を返す・捏造しない・R1.5）。第 3 フィールドが
/// 欠落／空なら `thumbnail == None`。展開・alias 解決はしない（転写層原則）。
fn parse_bindgroup_name(id: u32, value: &str) -> Option<BindGroupName> {
    let mut fields = value.splitn(3, ',');
    let category = fields.next().unwrap_or("").trim();
    let part = fields.next().map(str::trim).unwrap_or("");
    let thumbnail_raw = fields.next().map(str::trim).unwrap_or("");

    // パーツ名なしの宣言は正典上不完全＝転記しない（着せ替え ID を捏造しない・R1.5）。
    if part.is_empty() {
        tracing::warn!(
            bindgroup_id = id,
            value = %value,
            "bindgroup .name 宣言にパーツ名が無い（正典上不完全）ため転記対象外"
        );
        return None;
    }

    let thumbnail = if thumbnail_raw.is_empty() {
        None
    } else {
        Some(thumbnail_raw.to_owned())
    };

    Some(BindGroupName {
        id,
        category: category.to_owned(),
        part: part.to_owned(),
        thumbnail,
    })
}

#[cfg(test)]
mod bindgroup_name_transcription_tests {
    //! `read_bindgroup_defaults` の `.name` 転記（task 1.2）の in-source 檻。
    //!
    //! shell descript.txt を合成 tempdir に書き出し、`read_bindgroup_defaults` を
    //! 直接呼んで `.name` サフィックスの忠実転記（2/3 フィールド・trim・sakura/kero
    //! 区別・重複後勝ち・パーツ欠落 skip）と、既存 `.default` 経路の無改変（R1.6）を
    //! 決定論で固定する。tempfile 等外部クレートには依存せず temp_dir 直下を使う。

    use std::fs;
    use std::path::PathBuf;

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindScope;

    use super::read_bindgroup_defaults;

    /// テスト専用の一意な shell ディレクトリを作り、descript.txt を書き込んで返す。
    fn shell_with_descript(tag: &str, descript_body: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("areka_bindgroup_name_tests_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create shell dir");
        fs::write(dir.join("descript.txt"), descript_body.as_bytes()).expect("write descript.txt");
        dir
    }

    /// 3 フィールド形（カテゴリ, パーツ, サムネ）を忠実転記し、名前解決できる。
    #[test]
    fn name_three_fields_transcribed_and_resolvable() {
        let shell = shell_with_descript(
            "three_fields",
            "charset,UTF-8\nsakura.bindgroup1100.name,腕,伸び,thumb\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_names.len(), 1);
        let n = &defaults.sakura_names[0];
        assert_eq!(n.id, 1100);
        assert_eq!(n.category, "腕");
        assert_eq!(n.part, "伸び");
        assert_eq!(n.thumbnail, Some("thumb".to_string()));
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), Some(1100));

        let _ = fs::remove_dir_all(&shell);
    }

    /// 2 フィールド形（サムネ欠落）は `thumbnail == None` で転記する。
    #[test]
    fn name_two_fields_thumbnail_none() {
        let shell = shell_with_descript(
            "two_fields",
            "charset,UTF-8\nsakura.bindgroup1200.name,口,笑い\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_names.len(), 1);
        assert_eq!(defaults.sakura_names[0].thumbnail, None);
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "口", "笑い"), Some(1200));

        let _ = fs::remove_dir_all(&shell);
    }

    /// kero 側は本体側と区別して `kero_names` へ入る（R1.2）。
    #[test]
    fn kero_name_distinguished_from_sakura() {
        let shell = shell_with_descript(
            "kero_scope",
            "charset,UTF-8\nkero.bindgroup2100.name,腕,伸び\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.sakura_names.is_empty(), "sakura 側は空であるべき");
        assert_eq!(defaults.kero_names.len(), 1);
        assert_eq!(defaults.resolve_name(BindScope::Kero, "腕", "伸び"), Some(2100));
        // sakura スコープでは解決できない（別集合）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), None);

        let _ = fs::remove_dir_all(&shell);
    }

    /// パーツ欠落（第 2 フィールド無し）の宣言は転記対象外（捏造しない・R1.5）。
    #[test]
    fn name_missing_part_not_transcribed() {
        let shell = shell_with_descript(
            "missing_part",
            "charset,UTF-8\nsakura.bindgroup1300.name,カテゴリのみ\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(
            defaults.sakura_names.is_empty(),
            "パーツ欠落の宣言は転記されない: {:?}",
            defaults.sakura_names
        );

        let _ = fs::remove_dir_all(&shell);
    }

    /// 空パーツ（`カテゴリ,`）も転記対象外。
    #[test]
    fn name_empty_part_not_transcribed() {
        let shell = shell_with_descript(
            "empty_part",
            "charset,UTF-8\nsakura.bindgroup1301.name,カテゴリ,\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.sakura_names.is_empty());

        let _ = fs::remove_dir_all(&shell);
    }

    /// 重複 (カテゴリ, パーツ) はキー昇順走査の後勝ち（D2）＝最後の ID を採る。
    #[test]
    fn duplicate_category_part_last_wins() {
        // 1100 と 1400 が同じ (腕, 伸び) を宣言。キー昇順で 1100→1400 の順に転記され、
        // `resolve_name` は後勝ちで 1400 を返す。
        let shell = shell_with_descript(
            "duplicate",
            "charset,UTF-8\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             sakura.bindgroup1400.name,腕,伸び\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), Some(1400));

        let _ = fs::remove_dir_all(&shell);
    }

    /// R1.6: `.default` 経路は `.name` 増設後も無改変（value=="1" のみ・番号集合）。
    #[test]
    fn default_path_unchanged_alongside_names() {
        let shell = shell_with_descript(
            "default_coexist",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1101.default,0\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             kero.bindgroup2100.default,1\n",
        );
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(defaults.kero_default_on, vec![2100]);
        // 名前も併せて転記されている（両経路が同一走査で共存）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), Some(1100));

        let _ = fs::remove_dir_all(&shell);
    }

    /// Observable（emo2 実 fixture）: 宣言済みの全 (カテゴリ, パーツ) が ID へ解決する。
    #[test]
    fn emo2_declared_names_all_resolve() {
        let shell_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("pilot")
            .join("examples")
            .join("shiori-host-32")
            .join("fixtures")
            .join("emo2")
            .join("shell")
            .join("master");
        let defaults = read_bindgroup_defaults(&shell_dir, DefaultEncoding::Utf8);

        // 非空虚（emo2 は 30 本超の sakura .name を持つ）。
        assert!(
            defaults.sakura_names.len() >= 30,
            "emo2 の sakura .name 宣言が十分に転記されている（実測 30 本超）: {}",
            defaults.sakura_names.len()
        );
        // 代表的な宣言が (カテゴリ, パーツ) → ID へ引ける。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), Some(1100));
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "口", "にこっ"), Some(1207));
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "目", "通常"), Some(1302));
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "髪飾り", "ボンボン"), Some(1801));
        // 宣言済みの全組が漏れなく引ける（サムネ無し 2 フィールド形でも解決）。
        for n in &defaults.sakura_names {
            assert_eq!(
                defaults.resolve_name(BindScope::Sakura, &n.category, &n.part),
                Some(n.id),
                "宣言済み ({}, {}) が ID {} へ解決しない",
                n.category,
                n.part,
                n.id
            );
        }
    }
}
