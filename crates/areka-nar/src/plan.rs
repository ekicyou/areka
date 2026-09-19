//! マニフェストとエントリ列と要求から配置計画を組む（要件 4.8・5.1〜5.8・6.3）。
//!
//! 1 本の `.nar` は 1 つ以上の**配置**（[`Placement`]）になる。配置 1 つが
//! 「インストール済みフォルダ 1 つ」で、どのエントリをどの相対パスへ置くか・
//! 既存の宛先をどう扱うかまで決まっている。ここから先（タスク 4.2・4.3）は
//! 判断をせず、この計画のとおりに組み上げて入れ替えるだけになる。
//!
//! # 宛先は継ぎ足しでしか組まない（要件 4.8）
//!
//! 宛先は必ず `root.join("ghost"|"balloon").join(<1 階層の名前>)` の形で組む。
//! 継ぎ足す名前は 3 つの出どころしかなく、いずれも**先に検証されている**:
//!
//! - アーカイブ内の相対パス → [`crate::names::validate_entry_names`] が
//!   絶対・`..`・`\`・NUL・Windows で作れない名前を全て撥ねている。
//! - `install.txt` の `directory`／`*.directory` → [`crate::manifest`] が
//!   [`crate::names::is_valid_one_level_name`] で検証している。
//! - 呼び出し側が渡す宛先ゴーストの名前 → **本モジュールが**同じ規則で検証する
//!   （[`existing_target_ghost`]）。`..` は `<根>/ghost/..` として実在してしまうので、
//!   実在の検査だけでは根の外が宛先になる。
//!
//! 「外へ出ない」は 1 本の道筋を見ても証明にならないので、兄弟テストが全ての
//! 固定入力の全ての宛先・ファイル・フォルダを歩いて数える。
//!
//! # ここで触るファイルシステムは 1 つだけ
//!
//! 宛先ゴーストが実在するかの**読み取り**だけを行う（要件 5.7 が実在を問うため）。
//! 書き込みは 1 バイトもしない。兄弟テストが、全ての固定入力を通した前後で根の
//! 木が 1 つも変わらないことを突き合わせる。

use crate::error::{ElementKind, RefuseReason};
use crate::install::InstallRequest;
use crate::manifest::{Companion, ExistingPolicy, INSTALL_TXT, InstallKind, InstallManifest};
use crate::names::{EntryName, is_valid_one_level_name};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// 根の直下のゴースト格納先（ukadoc「全体の構成」）。
const GHOST_STORE: &str = "ghost";

/// 根の直下のバルーン格納先（ukadoc「全体の構成」）。
const BALLOON_STORE: &str = "balloon";

/// ゴーストの下のシェル格納先。
const SHELL_STORE: &str = "shell";

/// インストール済みフォルダ 1 つぶんの計画。
pub(crate) struct Placement {
    pub kind: ElementKind,
    /// ghost／shell／supplement／balloon 本体は `manifest.name`・同梱バルーンは `directory`。
    pub name: String,
    /// 置き先の絶対パス。
    pub destination: PathBuf,
    pub target_ghost: Option<String>,
    pub existing: ExistingPolicy,
    /// 最上位の `install.txt` を置かない（`supplement` だけ真＝要件 5.8）。
    ///
    /// [`files`](Placement::files) からは既に除いてあるので、組み上げ側が改めて
    /// 判断する必要はない。決めたのが計画の側であることを結果に残すための欄。
    ///
    /// 読むのは兄弟テストだけ（4 種の別ごとに値を測る）。本番の経路に読み手が要ると
    /// 「除く判断」が 2 か所に散るので、意図して持たせたまま読ませない。
    #[allow(dead_code)]
    pub skip_top_level_install_txt: bool,
    /// (エントリ番号, 宛先からの相対パス `/` 区切り)。アーカイブの順。
    pub files: Vec<(usize, String)>,
    /// 作るフォルダの相対パス。名前順で、親は必ず子より前に並ぶ（親は子の接頭辞
    /// なので名前順がそのまま作れる順になる）。ファイルの親も全段そろう（要件 4.9）。
    pub dirs: Vec<String>,
}

/// マニフェストとエントリ列と要求から配置計画を組む。
///
/// 返る列の先頭は必ず本体の配置で、その後ろに同梱バルーンが接頭辞の名前順で並ぶ。
/// 組み上げも確定もこの順で行い、巻き戻しは逆順になる（タスク 4.3）。
///
/// # Errors
///
/// 宛先ゴーストが渡されない・1 階層の名前でない・根に実在しないとき
/// [`RefuseReason::TargetGhostMissing`]（要件 5.7）。同梱バルーンの取り出し元
/// フォルダの配下にエントリが 1 件も無いとき [`RefuseReason::CompanionSourceMissing`]
/// （要件 5.4）。
pub(crate) fn build_plan(
    manifest: &InstallManifest,
    names: &[EntryName],
    request: &InstallRequest<'_>,
) -> Result<Vec<Placement>, RefuseReason> {
    let mut placements = Vec::with_capacity(1 + manifest.companions.len());
    placements.push(body_placement(manifest, names, request)?);
    for companion in &manifest.companions {
        placements.push(companion_placement(companion, names, request)?);
    }
    Ok(placements)
}

/// 本体（ゴースト／バルーン／シェル／サプリメント）の配置を組む（要件 5.2・5.5・5.6・5.8）。
fn body_placement(
    manifest: &InstallManifest,
    names: &[EntryName],
    request: &InstallRequest<'_>,
) -> Result<Placement, RefuseReason> {
    let ghost_store = request.root.join(GHOST_STORE);
    let (kind, destination, target_ghost) = match manifest.kind {
        InstallKind::Ghost => (
            ElementKind::Ghost,
            ghost_store.join(&manifest.directory),
            None,
        ),
        InstallKind::Balloon => (
            ElementKind::Balloon,
            request.root.join(BALLOON_STORE).join(&manifest.directory),
            None,
        ),
        InstallKind::Shell => {
            let target = existing_target_ghost(request)?;
            let destination = ghost_store
                .join(&target)
                .join(SHELL_STORE)
                .join(&manifest.directory);
            (ElementKind::Shell, destination, Some(target))
        }
        // サプリメントの宛先はゴースト本体そのもの。`directory` は使わない（要件 5.8）。
        InstallKind::Supplement => {
            let target = existing_target_ghost(request)?;
            let destination = ghost_store.join(&target);
            (ElementKind::Supplement, destination, Some(target))
        }
    };

    // 同梱バルーンの取り出し元は本体の側に複製しない（要件 5.3）。フォルダの
    // エントリそのものも除く——残すと中身の無い抜け殻が本体側に生える。
    // ゴーストだけでなくシェルにも同じ規則を当てる。同梱を持てるのはこの 2 種で、
    // どちらも「取り出し元は本体の置き場ではない」ことに違いが無い。
    let skip_install_txt = manifest.kind == InstallKind::Supplement;
    let accept = |entry: &EntryName| {
        !manifest
            .companions
            .iter()
            .any(|companion| in_folder(entry, &companion.source_directory))
            && !(skip_install_txt && is_top_level_install_txt(entry))
    };

    let (files, dirs) = collect_tree(names, accept, 0);
    Ok(Placement {
        kind,
        name: manifest.name.clone(),
        destination,
        target_ghost,
        existing: manifest.existing.clone(),
        skip_top_level_install_txt: skip_install_txt,
        files,
        dirs,
    })
}

/// 同梱バルーン 1 件の配置を組む（要件 5.3・5.4・6.3）。
///
/// 取り出し元の 1 階層を剥がしてバルーン格納先へ写す。中の `install.txt` は解釈せず、
/// 普通のファイルとして置く（正典 `manual_install`「同梱される側には install.txt
/// 不要。あっても無視される」）。
fn companion_placement(
    companion: &Companion,
    names: &[EntryName],
    request: &InstallRequest<'_>,
) -> Result<Placement, RefuseReason> {
    let (files, dirs) = collect_tree(
        names,
        |entry| in_folder(entry, &companion.source_directory),
        1,
    );
    // フォルダのエントリだけがあって配下が空の書庫も「取り出せない」に含める。
    // 空のバルーンを作っても利用者には何も届かないので、拒否のほうが正直。
    if files.is_empty() && dirs.is_empty() {
        return Err(RefuseReason::CompanionSourceMissing {
            key: format!("{}.source.directory", companion.key),
            source_directory: companion.source_directory.clone(),
        });
    }
    Ok(Placement {
        kind: ElementKind::Balloon,
        name: companion.directory.clone(),
        destination: request.root.join(BALLOON_STORE).join(&companion.directory),
        target_ghost: None,
        existing: companion.existing.clone(),
        skip_top_level_install_txt: false,
        files,
        dirs,
    })
}

/// 呼び出し側が渡した宛先ゴーストの名前を検証し、根に実在することを確かめる（要件 5.7）。
///
/// 渡されない・1 階層の名前でない・実在しないの 3 つを同じ理由で返す。利用者から
/// 見ればどれも「その名前のゴーストが入っていない」で、`ghost-install` が採る手も
/// 同じ（宛先を選び直す）。名前の形を**実在の検査より先に**見るのは、`..` のような
/// 名前が `<根>/ghost/..` として実在してしまい、宛先が根の外へ出るため。
fn existing_target_ghost(request: &InstallRequest<'_>) -> Result<String, RefuseReason> {
    let missing = |target: Option<&str>| RefuseReason::TargetGhostMissing {
        target: target.map(str::to_owned),
    };
    let target = request.target_ghost.ok_or_else(|| missing(None))?;
    if !is_valid_one_level_name(target) || !request.root.join(GHOST_STORE).join(target).is_dir() {
        return Err(missing(Some(target)));
    }
    Ok(target.to_owned())
}

/// エントリが `folder` そのもの、またはその配下か。
///
/// 本体から除く側（要件 5.3）と同梱バルーンへ取り込む側（要件 5.4）が**同じ一言**を
/// 使う。二重に書くと、片方だけを直したときに同じエントリが両方へ入る（複製が残る）か
/// どちらにも入らない（消える）。取り出し元のフォルダのエントリそのものは、剥がすと
/// 何も残らないので [`collect_tree`] が落とす。
///
/// 突き合わせは ASCII の大小を無視する。`install.txt` に書かれた綴りと書庫の中の
/// 綴りが大小だけ違う配布物は普通にあり、Windows では同じフォルダを指す。
fn in_folder(entry: &EntryName, folder: &str) -> bool {
    entry.components[0].eq_ignore_ascii_case(folder)
}

/// 最上位の `install.txt`（マニフェストそのもの）か。
///
/// 下の階層の `install.txt` は普通のファイル。除くのは最上位の 1 件だけ（要件 5.8）。
/// [`EntryName::path`] は区切りを正規化してあるので、**丸ごと `install.txt` に等しい**
/// という一言が「最上位にある」と「名前が `install.txt` である」の両方を言っている
/// （`sub/install.txt` は等しくならない）。階層の数を別に数えても同じ判定にしか
/// ならないので、二重には書かない。
///
/// フォルダのエントリかどうかも見ない。ここへ来るのはマニフェストを読み終えた後で、
/// そのとき最上位に `install.txt` という**ファイル**が在ることが確定している。
/// [`crate::names::validate_entry_names`] は `a` と `a/` を衝突として撥ねるので、
/// 同じ名前のフォルダのエントリは同じ書庫に居られない。見ても決して効かない腕になる。
fn is_top_level_install_txt(entry: &EntryName) -> bool {
    entry.path.eq_ignore_ascii_case(INSTALL_TXT)
}

/// `accept` が通したエントリを、先頭 `strip` 要素を剥がした相対パスの木に写す。
///
/// フォルダの一覧には⑴フォルダのエントリそのものと⑵全てのファイルの親を全段入れる。
/// これで「フォルダのエントリを空フォルダとして作る」と「フォルダのエントリが無い
/// 書庫でもファイルの親を作る」（要件 4.9）が、組み上げ側から見て 1 つの手順になる。
fn collect_tree(
    names: &[EntryName],
    accept: impl Fn(&EntryName) -> bool,
    strip: usize,
) -> (Vec<(usize, String)>, Vec<String>) {
    let mut files = Vec::new();
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for entry in names.iter().filter(|entry| accept(entry)) {
        let tail = &entry.components[strip..];
        // 取り出し元のフォルダのエントリそのもの。剥がすと何も残らない。`split_last`
        // で受けるので、最後の要素（＝ファイル名かフォルダ名）が必ず在ることと、
        // その親の段数が同時に決まる（`tail.len() - 1` の引き算を書かずに済む）。
        let Some((_, parents)) = tail.split_last() else {
            continue;
        };
        let depth = if entry.is_dir {
            tail.len()
        } else {
            parents.len()
        };
        for ancestor in 1..=depth {
            dirs.insert(tail[..ancestor].join("/"));
        }
        if !entry.is_dir {
            files.push((entry.index, tail.join("/")));
        }
    }
    (files, dirs.into_iter().collect())
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
