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

use crate::charset::{DefaultEncoding, decode};
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
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#sakura.name_2c_540d_524d:1
        sakura_name: map.get("sakura.name").cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#sakura.name2_2c_540d_524d:1
        sakura_name2: map.get("sakura.name2").cloned(),
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#kero.name_2c_540d_524d:1
        kero_name: map.get("kero.name").cloned(),
    };

    // SHIORI マウント: dir は起点の親（存在確定）、file は未指定なら None（推測禁止・Req 2.3）。
    let shiori = ShioriMount {
        dir: shiori_dir,
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#shiori_2c_30d5_30a1_30a4_30eb_540d:1
        file: map.get("shiori").cloned(),
    };

    // shell マウント: 指定名 or 既定 master（Req 3.1/3.2）→ 物理存在確認（Req 3.3）。
    let shell_name = map
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#seriko.defaultsurfacedirectoryname_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1
        .get("seriko.defaultsurfacedirectoryname")
        .map(String::as_str)
        .unwrap_or(DEFAULT_SHELL_DIR);
    let shell_dir = ghost_root.join(SHELL_ROOT).join(shell_name);
    if !shell_dir.is_dir() {
        return Err(MountError::ShellDirMissing {
            expected: shell_dir,
        });
    }
    // shell descript.txt から bindgroup default を転記（bindopt 1.1/1.2）。bindgroup default
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
/// bindoption 宣言キーの接頭辞（`sakura.bindoptionN.group` の先頭・task 10.1）。
const SAKURA_BINDOPTION_PREFIX: &str = "sakura.bindoption";
const KERO_BINDOPTION_PREFIX: &str = "kero.bindoption";
/// bindoption 宣言キーの接尾辞（`sakura.bindoptionN.group` の末尾・task 10.1）。
const BINDOPTION_GROUP_SUFFIX: &str = ".group";
/// ちょうど 1 個（解除不可）を宣言するオプション語（オプション欄の 1 語・bindopt 1.2）。
const MUSTSELECT_OPTION: &str = "mustselect";
/// 複数可を宣言するオプション語（オプション欄の 1 語・bindopt 1.1）。
///
/// ukadoc 正典の 3 値は `mustselect`（ちょうど 1 個・解除不可）／非宣言（既定＝高々 1 個・
/// 解除可）／`multiple`（複数可）。旧実装は `multiple` を破棄していたため下流に「明示
/// multiple」と「非宣言」を区別する情報が無かった。
const MULTIPLE_OPTION: &str = "multiple";
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
fn read_bindgroup_defaults(
    shell_dir: &Path,
    default_encoding: DefaultEncoding,
) -> BindGroupDefaults {
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
            if let Some(id) =
                // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.default_2c_6570_5024:1
                parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX)
            {
                defaults.sakura_default_on.push(id);
                continue;
            } else if let Some(id) =
                parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX, BINDGROUP_DEFAULT_SUFFIX)
            {
                defaults.kero_default_on.push(id);
                continue;
            }
        }
        // --- .name 経路（task 1.2・値はカテゴリ,パーツ[,サムネ] 文字列）---
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
        if let Some(id) = parse_bindgroup_id(key, SAKURA_BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX)
            && let Some(name) = parse_bindgroup_name(id, value)
        {
            defaults.sakura_names.push(name);
            continue;
        } else if let Some(id) =
            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1
            parse_bindgroup_id(key, KERO_BINDGROUP_PREFIX, BINDGROUP_NAME_SUFFIX)
            && let Some(name) = parse_bindgroup_name(id, value)
        {
            defaults.kero_names.push(name);
            continue;
        }
        // --- bindoption.group 経路（値はカテゴリ,オプション[+オプション...] 文字列）---
        // `sakura/kero.bindoptionN.group,カテゴリ名,オプション` のオプション欄を `+` で分解し、
        // 認識できた語（`mustselect`／`multiple`）の**所属**をスコープ別に忠実転記する。
        // 併記（`mustselect+multiple`）は両方の集合へ push し、併記の情報を落とさない
        // （優先則は下流 seriko の解釈・bindopt D4）。どちらの集合にも入らないカテゴリが
        // 正典の既定（高々 1 個・解除可）を表す——ここで既定を捏造しない（bindopt 1.1）。
        // 「排他か」の判定語彙は parsers に持ち込まない（転写層原則）。オプション
        // インデックス `N`（`parse_bindgroup_id` が検証する）は M1 不使用＝キー形の妥当性
        // 検査にのみ用いる（転記のみ・展開しない）。
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        if parse_bindgroup_id(key, SAKURA_BINDOPTION_PREFIX, BINDOPTION_GROUP_SUFFIX).is_some()
            && let Some(decl) = parse_bindoption_options(value)
        {
            if decl.mustselect {
                defaults.sakura_mustselect.push(decl.category.clone());
            }
            if decl.multiple {
                defaults.sakura_multiple.push(decl.category);
            }
        // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell.html#kero.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1
        } else if parse_bindgroup_id(key, KERO_BINDOPTION_PREFIX, BINDOPTION_GROUP_SUFFIX).is_some()
            && let Some(decl) = parse_bindoption_options(value)
        {
            if decl.mustselect {
                defaults.kero_mustselect.push(decl.category.clone());
            }
            if decl.multiple {
                defaults.kero_multiple.push(decl.category);
            }
        }
    }
    defaults
}

/// `bindoption*.group` の値 1 行を分解した宣言（カテゴリ名と、認識できたオプション語の所属）。
///
/// 「排他か」の**解釈**は持たない——認識できた語の所属を写すだけの転写値（転写層原則）。
struct BindOptionDecl {
    /// 第 1 フィールド（カテゴリ名・trim 済み・非空）。
    category: String,
    /// オプション欄に `mustselect` が現れたか（bindopt 1.2）。
    mustselect: bool,
    /// オプション欄に `multiple` が現れたか（bindopt 1.1）。
    multiple: bool,
}

/// `bindoption*.group` の値 `カテゴリ名,オプション[+オプション...]` を分解し、認識できた
/// オプション語の所属を返す（ukadoc 正典「オプションは `+` 区切りで複数指定可」・bindopt 1.3）。
///
/// `splitn(2, ',')` でカテゴリ名とオプション欄に分け、オプション欄を `'+'` で split して各語
/// を trim し、`mustselect`／`multiple` との完全一致で認識する。認識できない語は捏造せず
/// 読み流す（寛容パース・bindopt 1.4）。次のいずれかなら `None`＝収録対象外（bindopt 1.5）:
/// カテゴリ名が空／オプション欄が欠落（`,` なし）／認識語ゼロ（空欄・未知語のみ）。
/// どちらの集合にも入らないカテゴリは正典の既定（高々 1 個・解除可）として下流が扱うため、
/// ここで既定を捏造しない。
///
/// 純関数（副作用なし・同一入力同一出力・bindopt 1.7）。
fn parse_bindoption_options(value: &str) -> Option<BindOptionDecl> {
    let mut fields = value.splitn(2, ',');
    let category = fields.next().unwrap_or("").trim();
    // オプション欄欠落（`,` なし）は空欄と同じく認識語ゼロへ落ちる。
    let options = fields.next().unwrap_or("");
    if category.is_empty() {
        return None;
    }

    let mut mustselect = false;
    let mut multiple = false;
    for option in options.split('+') {
        match option.trim() {
            MUSTSELECT_OPTION => mustselect = true,
            MULTIPLE_OPTION => multiple = true,
            // 未知オプション語は読み流す（捏造しない・bindopt 1.4）。
            _ => {}
        }
    }

    if mustselect || multiple {
        Some(BindOptionDecl {
            category: category.to_owned(),
            mustselect,
            multiple,
        })
    } else {
        None
    }
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

#[cfg(test)]
mod bindgroup_name_transcription_tests {
    //! `read_bindgroup_defaults` の `.name` 転記（task 1.2）の in-source 檻。
    //!
    //! shell descript.txt を合成 tempdir に書き出し、`read_bindgroup_defaults` を
    //! 直接呼んで `.name` サフィックスの忠実転記（2/3 フィールド・trim・sakura/kero
    //! 区別・重複後勝ち・パーツ欠落 skip）と、既存 `.default` 経路の無改変（R1.6）を
    //! 決定論で固定する。一時パスは共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意）。

    use std::fs;

    use temp_path_kit::TempPath;

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindScope;

    use super::read_bindgroup_defaults;

    /// テスト専用の一意な shell ディレクトリを作り、descript.txt を書き込んで返す。
    ///
    /// 一時ディレクトリは共通窓口 `temp-path-kit` 経由で組むので、名前にプロセス識別子と
    /// 連番が入り**プロセス間でも一意**になる。返り値が生きている間だけ実体が存在し、
    /// 破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
        let dir = TempPath::new(&format!("parsers-bindgroup-name-{}", tag.replace('_', "-")));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// 3 フィールド形（カテゴリ, パーツ, サムネ）を忠実転記し、名前解決できる。
    #[test]
    fn name_three_fields_transcribed_and_resolvable() {
        let temp = shell_with_descript(
            "three_fields",
            "charset,UTF-8\nsakura.bindgroup1100.name,腕,伸び,thumb\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_names.len(), 1);
        let n = &defaults.sakura_names[0];
        assert_eq!(n.id, 1100);
        assert_eq!(n.category, "腕");
        assert_eq!(n.part, "伸び");
        assert_eq!(n.thumbnail, Some("thumb".to_string()));
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
    }

    /// 2 フィールド形（サムネ欠落）は `thumbnail == None` で転記する。
    #[test]
    fn name_two_fields_thumbnail_none() {
        let temp = shell_with_descript(
            "two_fields",
            "charset,UTF-8\nsakura.bindgroup1200.name,口,笑い\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_names.len(), 1);
        assert_eq!(defaults.sakura_names[0].thumbnail, None);
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "口", "笑い"),
            Some(1200)
        );
    }

    /// kero 側は本体側と区別して `kero_names` へ入る（R1.2）。
    #[test]
    fn kero_name_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_scope",
            "charset,UTF-8\nkero.bindgroup2100.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.sakura_names.is_empty(), "sakura 側は空であるべき");
        assert_eq!(defaults.kero_names.len(), 1);
        assert_eq!(
            defaults.resolve_name(BindScope::Kero, "腕", "伸び"),
            Some(2100)
        );
        // sakura スコープでは解決できない（別集合）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), None);
    }

    /// パーツ欠落（第 2 フィールド無し）の宣言は転記対象外（捏造しない・R1.5）。
    #[test]
    fn name_missing_part_not_transcribed() {
        let temp = shell_with_descript(
            "missing_part",
            "charset,UTF-8\nsakura.bindgroup1300.name,カテゴリのみ\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(
            defaults.sakura_names.is_empty(),
            "パーツ欠落の宣言は転記されない: {:?}",
            defaults.sakura_names
        );
    }

    /// 空パーツ（`カテゴリ,`）も転記対象外。
    #[test]
    fn name_empty_part_not_transcribed() {
        let temp = shell_with_descript(
            "empty_part",
            "charset,UTF-8\nsakura.bindgroup1301.name,カテゴリ,\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.sakura_names.is_empty());
    }

    /// 重複 (カテゴリ, パーツ) はキー昇順走査の後勝ち（D2）＝最後の ID を採る。
    #[test]
    fn duplicate_category_part_last_wins() {
        // 1100 と 1400 が同じ (腕, 伸び) を宣言。キー昇順で 1100→1400 の順に転記され、
        // `resolve_name` は後勝ちで 1400 を返す。
        let temp = shell_with_descript(
            "duplicate",
            "charset,UTF-8\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             sakura.bindgroup1400.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1400)
        );
    }

    /// R1.6: `.default` 経路は `.name` 増設後も無改変（value=="1" のみ・番号集合）。
    #[test]
    fn default_path_unchanged_alongside_names() {
        let temp = shell_with_descript(
            "default_coexist",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1101.default,0\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             kero.bindgroup2100.default,1\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(defaults.kero_default_on, vec![2100]);
        // 名前も併せて転記されている（両経路が同一走査で共存）。
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
    }

    /// R1.6（純粋非退行）: `.default` on/off **のみ**（`.name` 皆無）の shell descript は、
    /// 従来どおり default 集合を転記しつつ名前表を一切生成しない（名前解決の追加が
    /// 既定 on/off マウント結果を変えない・捏造しない）。`default_path_unchanged_alongside_names`
    /// が `.name` 併存を扱うのに対し、本テストは名前宣言が皆無の既定 on/off 専用ケースを固定する。
    #[test]
    fn default_only_no_names_leaves_names_empty() {
        let temp = shell_with_descript(
            "default_only_no_names",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1101.default,0\n\
             kero.bindgroup2100.default,1\n\
             kero.bindgroup2101.default,0\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // 既定 on/off は従来どおり（value=="1" のみ収集・off は非収集）。
        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(defaults.kero_default_on, vec![2100]);

        // 名前宣言が皆無 → 名前表は空（default,1 が名前を捏造しない・R1.5/R1.6）。
        assert!(
            defaults.sakura_names.is_empty(),
            "名前宣言皆無なら sakura_names は空"
        );
        assert!(
            defaults.kero_names.is_empty(),
            "名前宣言皆無なら kero_names は空"
        );

        // 名前解決は全スコープで None（着せ替え ID を捏造しない）。
        assert_eq!(defaults.resolve_name(BindScope::Sakura, "腕", "伸び"), None);
        assert_eq!(defaults.resolve_name(BindScope::Kero, "腕", "伸び"), None);
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
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "口", "にこっ"),
            Some(1207)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "目", "通常"),
            Some(1302)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "髪飾り", "ボンボン"),
            Some(1801)
        );
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

#[cfg(test)]
mod bindoption_options_tests {
    //! `read_bindgroup_defaults` の `bindoption*.group` 取り込みの in-source 檻（bindopt 4.2）。
    //!
    //! shell descript.txt を合成 tempdir に書き出し、`read_bindgroup_defaults` を直接呼んで
    //! `sakura/kero.bindoption*.group,カテゴリ,オプション[+オプション...]` の取り込みを
    //! 決定論で固定する。網羅マトリクス: ukadoc 正典 3 値（`mustselect`／非宣言＝既定／
    //! `multiple`）・`+` 区切り併記・未知語・不完全値・宣言ゼロ・sakura/kero 隔離・再読の
    //! 同一結果。一時パスは共通窓口 `temp-path-kit` 経由で組む（プロセス間でも一意）。
    //!
    //! 語彙: 「排他置換（exclusive）」「既定（Default＝高々 1 個・解除可）」「複数可
    //! （Multiple）」。ここで検証するのは**宣言の所属の転記**だけで、「排他か」の解釈は
    //! 下流 seriko の責務（転写層原則）。

    use std::fs;

    use temp_path_kit::TempPath;

    use crate::charset::DefaultEncoding;
    use crate::package::model::BindScope;

    use super::read_bindgroup_defaults;

    /// テスト専用の一意な shell ディレクトリを作り、descript.txt を書き込んで返す。
    ///
    /// 一時ディレクトリは共通窓口 `temp-path-kit` 経由で組むので、名前にプロセス識別子と
    /// 連番が入り**プロセス間でも一意**になる。返り値が生きている間だけ実体が存在し、
    /// 破棄で中身ごと消える（後始末を呼び忘れる余地が無い）。
    fn shell_with_descript(tag: &str, descript_body: &str) -> TempPath {
        // 札は `-` と英数字だけ（窓口の約束）。呼出側の tag は関数名由来で `_` を含む。
        let dir = TempPath::new(&format!(
            "parsers-bindoption-options-{}",
            tag.replace('_', "-")
        ));
        fs::write(dir.path().join("descript.txt"), descript_body.as_bytes())
            .expect("write descript.txt");
        dir
    }

    /// `mustselect` 単独宣言はスコープ別に収録され、multiple 側へは漏れない（bindopt 1.2）。
    ///
    /// 既存挙動不変の錨。非宣言カテゴリ・別スコープはどちらの集合にも入らない＝既定。
    #[test]
    fn mustselect_declared_categories_ingested() {
        let temp = shell_with_descript(
            "declared",
            "charset,UTF-8\n\
             sakura.bindoption0.group,腕,mustselect\n\
             sakura.bindoption3.group,目,mustselect\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "目"));
        // mustselect 単独宣言は multiple 集合へ入らない（2 集合は独立）。
        assert!(defaults.sakura_multiple.is_empty());
        assert!(!defaults.is_multiple(BindScope::Sakura, "腕"));
        // 非宣言カテゴリはどちらの集合にも入らない＝既定（捏造しない）。
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        assert!(!defaults.is_multiple(BindScope::Sakura, "紅"));
        // 別スコープ（kero）には漏れない。
        assert!(!defaults.is_mustselect(BindScope::Kero, "腕"));
    }

    /// `multiple` 単独宣言は multiple としてスコープ別に**収録**される（bindopt 1.1）。
    ///
    /// 旧実装は `multiple` を破棄しており、下流に「複数可の明示宣言」と「非宣言（既定＝
    /// 高々 1 個・解除可）」を区別する情報が存在しなかった——情報欠落の根。本檻は収録
    /// されること（＝区別が成立すること）を固定する。
    #[test]
    fn multiple_option_ingested() {
        let temp = shell_with_descript(
            "multiple",
            "charset,UTF-8\nsakura.bindoption0.group,紅,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_multiple, vec!["紅".to_string()]);
        assert!(defaults.is_multiple(BindScope::Sakura, "紅"));
        // multiple 宣言は mustselect ではない（2 集合は独立）。
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        assert!(defaults.sakura_mustselect.is_empty());
        // 別スコープ（kero）には漏れない。
        assert!(!defaults.is_multiple(BindScope::Kero, "紅"));
        assert!(defaults.kero_multiple.is_empty());
    }

    /// kero 側の `multiple` は本体側と区別して収録する（bindopt 1.1・スコープ隔離）。
    #[test]
    fn kero_multiple_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_multiple",
            "charset,UTF-8\nkero.bindoption0.group,尻尾飾り,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.kero_multiple, vec!["尻尾飾り".to_string()]);
        assert!(defaults.is_multiple(BindScope::Kero, "尻尾飾り"));
        assert!(!defaults.is_multiple(BindScope::Sakura, "尻尾飾り"));
        assert!(defaults.sakura_multiple.is_empty());
    }

    /// kero 側は本体側と区別して取り込む（bindopt 1.2・スコープ隔離）。
    #[test]
    fn kero_mustselect_distinguished_from_sakura() {
        let temp = shell_with_descript(
            "kero_scope",
            "charset,UTF-8\nkero.bindoption0.group,腕,mustselect\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert!(defaults.is_mustselect(BindScope::Kero, "腕"));
        assert!(!defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.sakura_mustselect.is_empty());
    }

    /// `+` 区切り併記（`mustselect+multiple`）は両方の集合へ収録し情報を落とさない
    /// （ukadoc 正典「オプションは `+` 区切りで複数指定可」・bindopt 1.3）。
    ///
    /// 旧実装は値全体と `mustselect` の完全一致で判定していたため、`+` 結合値は
    /// mustselect 側も含めて全部落ちていた。どちらを優先するか（bindopt D4）は
    /// 下流 seriko の解釈であり、転写層は両方を写す。
    #[test]
    fn plus_separated_both_options_ingested_into_both_sets() {
        let temp = shell_with_descript(
            "plus_both",
            "charset,UTF-8\nsakura.bindoption0.group,腕,mustselect+multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_mustselect, vec!["腕".to_string()]);
        assert_eq!(defaults.sakura_multiple, vec!["腕".to_string()]);
        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_multiple(BindScope::Sakura, "腕"));
    }

    /// `+` 区切りの語順は結果に影響しない（`multiple+mustselect`・bindopt 1.3）。
    #[test]
    fn plus_separated_option_order_is_insensitive() {
        let temp = shell_with_descript(
            "plus_order",
            "charset,UTF-8\nsakura.bindoption0.group,腕,multiple+mustselect\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(defaults.sakura_mustselect, vec!["腕".to_string()]);
        assert_eq!(defaults.sakura_multiple, vec!["腕".to_string()]);
    }

    /// `+` 区切りに未知語が混ざっても、認識できる語のみ収録して未知語は読み流す
    /// （寛容パース・bindopt 1.3/1.4）。空白入りの語も trim して認識する。
    #[test]
    fn plus_separated_unknown_word_is_skipped() {
        let temp = shell_with_descript(
            "plus_unknown",
            "charset,UTF-8\n\
             sakura.bindoption0.group,紅,unknown+multiple\n\
             sakura.bindoption1.group,腕,mustselect + unknown\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // unknown+multiple → multiple のみ収録。
        assert_eq!(defaults.sakura_multiple, vec!["紅".to_string()]);
        assert!(!defaults.is_mustselect(BindScope::Sakura, "紅"));
        // mustselect + unknown → mustselect のみ収録（各語 trim）。
        assert_eq!(defaults.sakura_mustselect, vec!["腕".to_string()]);
        assert!(!defaults.is_multiple(BindScope::Sakura, "腕"));
    }

    /// 未知語のみ・オプション欄空・カテゴリ空・`,` なしはいずれも収録対象外
    /// （捏造しない・寛容パース維持・bindopt 1.4/1.5）。
    ///
    /// 収録対象外＝どちらの集合にも入らない＝当該カテゴリは既定（高々 1 個・解除可）。
    #[test]
    fn missing_or_empty_fields_not_ingested() {
        let temp = shell_with_descript(
            "missing",
            "charset,UTF-8\n\
             sakura.bindoption0.group,腕\n\
             sakura.bindoption1.group,,mustselect\n\
             sakura.bindoption2.group,口,\n\
             sakura.bindoption4.group,眉,unknown\n\
             sakura.bindoption5.group,頬,Mustselect\n\
             sakura.bindoption6.group, ,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // オプション欄欠落（腕・`,` なし）・カテゴリ空（,mustselect）・オプション欄空（口,）
        // ・未知語のみ（眉）・大小異なる語（頬・完全一致のみ認識）・空白のみカテゴリ（multiple）
        // はいずれも非収録。
        assert!(
            defaults.sakura_mustselect.is_empty(),
            "不完全値・未知語は mustselect へ収録されない: {:?}",
            defaults.sakura_mustselect
        );
        assert!(
            defaults.sakura_multiple.is_empty(),
            "不完全値・空カテゴリは multiple へ収録されない: {:?}",
            defaults.sakura_multiple
        );
        for category in ["腕", "口", "眉", "頬", ""] {
            assert!(!defaults.is_mustselect(BindScope::Sakura, category));
            assert!(!defaults.is_multiple(BindScope::Sakura, category));
        }
    }

    /// bindoption 宣言が 1 件も無い shell も読み取りは成立し、全カテゴリ既定になる
    /// （読み取り失敗にしない・bindopt 1.6）。
    #[test]
    fn no_bindoption_declarations_yields_all_default() {
        let temp = shell_with_descript(
            "no_bindoption",
            "charset,UTF-8\n\
             sakura.bindgroup1100.default,1\n\
             sakura.bindgroup1100.name,腕,伸び\n\
             kero.bindgroup2100.name,腕,伸び\n",
        );
        let shell = temp.path().to_path_buf();
        let defaults = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        // 4 集合すべて空＝全カテゴリ既定（高々 1 個・解除可）。
        assert!(defaults.sakura_mustselect.is_empty());
        assert!(defaults.kero_mustselect.is_empty());
        assert!(defaults.sakura_multiple.is_empty());
        assert!(defaults.kero_multiple.is_empty());
        // 併存する `.default`／`.name` 経路は通常どおり成立する（読み取り失敗にしない）。
        assert_eq!(defaults.sakura_default_on, vec![1100]);
        assert_eq!(
            defaults.resolve_name(BindScope::Sakura, "腕", "伸び"),
            Some(1100)
        );
        assert_eq!(
            defaults.resolve_name(BindScope::Kero, "腕", "伸び"),
            Some(2100)
        );
    }

    /// 同一 descript を再読すると同一の収録結果を返す（決定論・走査順維持・bindopt 1.7）。
    #[test]
    fn same_descript_reread_yields_same_result() {
        let temp = shell_with_descript(
            "determinism",
            "charset,UTF-8\n\
             sakura.bindoption0.group,腕,mustselect\n\
             sakura.bindoption1.group,口,mustselect+multiple\n\
             sakura.bindoption2.group,紅,multiple\n\
             sakura.bindoption3.group,眉,unknown\n\
             kero.bindoption0.group,尻尾飾り,multiple\n",
        );
        let shell = temp.path().to_path_buf();
        let first = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);
        let second = read_bindgroup_defaults(&shell, DefaultEncoding::Utf8);

        assert_eq!(first, second, "同一入力に同一結果（決定論）");
        // 収録順もキー昇順で固定（走査順維持）。
        assert_eq!(
            first.sakura_mustselect,
            vec!["腕".to_string(), "口".to_string()]
        );
        assert_eq!(
            first.sakura_multiple,
            vec!["口".to_string(), "紅".to_string()]
        );
        assert_eq!(first.kero_multiple, vec!["尻尾飾り".to_string()]);
        assert!(first.kero_mustselect.is_empty());
    }

    /// Observable（emo2 実 fixture）: 腕・口・眉・目 が `mustselect` 宣言と判別でき、
    /// 宣言のないカテゴリ（紅・髪飾り・まばたき）はどちらの集合にも属さない＝既定
    /// （高々 1 個・解除可）と判別できる（bindopt 1.1/1.2）。
    ///
    /// emo2 は `multiple` を 1 件も宣言していない——本件の表情固着はまさに「非宣言の
    /// まばたきカテゴリ（1400-1403）」で起きており、非宣言が `multiple` と同一視されて
    /// いたことが根であった。
    #[test]
    fn emo2_bindoption_categories_discriminated() {
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

        assert!(defaults.is_mustselect(BindScope::Sakura, "腕"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "口"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "眉"));
        assert!(defaults.is_mustselect(BindScope::Sakura, "目"));
        // emo2 は multiple を 1 件も宣言していない。
        assert!(
            defaults.sakura_multiple.is_empty(),
            "emo2 に multiple 宣言は無い: {:?}",
            defaults.sakura_multiple
        );
        assert!(defaults.kero_multiple.is_empty());
        // 宣言のないカテゴリ（紅・髪飾りは .name 宣言はあるが bindoption 宣言なし）＝既定。
        for category in ["紅", "髪飾り", "まばたき"] {
            assert!(!defaults.is_mustselect(BindScope::Sakura, category));
            assert!(!defaults.is_multiple(BindScope::Sakura, category));
        }
        // kero 側は宣言なし＝全偽（スコープ隔離）。
        assert!(!defaults.is_mustselect(BindScope::Kero, "腕"));
        assert!(defaults.kero_mustselect.is_empty());
    }
}
