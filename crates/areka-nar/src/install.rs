//! 計画のとおりに作業フォルダへ木を組み上げる（要件 4.9・5.9・6.1・6.2）。
//!
//! # なぜ宛先に直接書かないか
//!
//! 「既存を全部消してから展開する」（要件 6.2）を字義どおりに書くと、消した後で
//! 失敗したときに宛先が空のまま残る。そこで**先に作業フォルダで完成形を組み上げ、
//! 最後にフォルダ単位で入れ替える**（入れ替えはタスク 4.3）。利用者から見える結果は
//! 同じで、「全部入った」か「何も変わっていない」しか見えない（要件 5.11）。
//!
//! 本モジュールの組み上げ側は宛先を**読む**だけで、1 バイトも書かない。
//!
//! # 既存の宛先の取り込み
//!
//! 完成形は「既存の宛先の木」＋「書庫の木」の重ね合わせで、下敷きの取り方だけが
//! [`ExistingPolicy`] で変わる。`Overlay` は丸ごと、`Replace { keep }` は除外マスクに
//! 挙がったファイル名だけ（全階層・ASCII 大小無視）。
//!
//! サプリメントに `refresh,1` が書かれていても全消去にならないのは、
//! [`crate::manifest`] が `supplement` の `refresh` を読み飛ばして必ず `Overlay` を
//! 返すため（`body_existing`）。ここで種別をもう一度見ても決して効かない腕になるので
//! 見ない。兄弟テストは `refresh,1` を書いたサプリメントを**マニフェストから通して**
//! 重ね置きになることを確かめる。

use crate::error::{ExistingState, InstalledElement, IoPhase, ManifestWarning, SurvivingTree};
use crate::manifest::{ExistingPolicy, InstallManifest};
use crate::plan::Placement;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

/// 根の直下に掘る作業フォルダの棚。入れ替えが `rename` で済むよう根と同じボリュームに置く。
const WORK: &str = ".nar-work";

/// 同一プロセス内で単調増加する連番。プロセス間の一意性はプロセス識別子が担う。
static NEXT_SERIAL: AtomicU32 = AtomicU32::new(0);

/// 巻き戻せなかった元の木を含む作業フォルダを片付けから守る期間（要件 2.7）。
pub(crate) const SURVIVOR_RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// 展開の要求。根の場所は呼び出し側が決める（要件 5.1）。
pub struct InstallRequest<'a> {
    /// ベースウェアの根（絶対パス）。
    pub root: &'a Path,
    /// `shell`／`supplement` の宛先ゴーストのフォルダ名。
    ///
    /// `accept` から宛先を求めるのは呼び出し側の仕事（要件 5.6）。
    pub target_ghost: Option<&'a str>,
}

/// どのパスで I/O に失敗したか。[`crate::NarError::Io`] の `path` と `source` になる。
///
/// 記録は公開面（タスク 4.4）が `Err` を返す直前に 1 回だけ出す。ここでは出さない
/// （二重記録を避ける＝設計「Monitoring」）。黙って握り潰す経路は 1 つも持たない。
#[derive(Debug)]
pub(crate) struct StageError {
    pub path: PathBuf,
    pub source: io::Error,
}

/// 失敗したときだけ、起きた場所のパスを添える。
fn at<T>(path: &Path, result: io::Result<T>) -> Result<T, StageError> {
    result.map_err(|source| StageError {
        path: path.to_path_buf(),
        source,
    })
}

/// 1 回の展開が使う作業フォルダ `<根>/.nar-work/<プロセス識別子>-<連番>/`。
#[derive(Debug)]
pub(crate) struct WorkArea {
    dir: PathBuf,
    /// 棚に残っていて消せなかった物。[`InstallOutcome::leftovers`] に合流する。
    residue: Vec<PathBuf>,
}

impl WorkArea {
    /// 棚の残骸を片付けてから、この展開のための作業フォルダを 1 つ作る。
    ///
    /// # Errors
    ///
    /// 根が実在しないとき（根を勝手に作らない＝設計の事前条件）、この走行の番地を
    /// 空にできないとき、作業フォルダを作れないとき [`StageError`]。
    pub(crate) fn create(root: &Path) -> Result<WorkArea, StageError> {
        WorkArea::create_at(root, SystemTime::now())
    }

    /// [`WorkArea::create`] の時刻を受け取る形。`now` は棚の保持の期限の判定にだけ使う
    /// （テストが実際の日数を待たずに期限の内外を渡す口＝要件 2.10）。
    ///
    /// # Errors
    ///
    /// [`WorkArea::create`] と同じ。
    pub(crate) fn create_at(root: &Path, now: SystemTime) -> Result<WorkArea, StageError> {
        if !root.is_dir() {
            return Err(StageError {
                path: root.to_path_buf(),
                source: io::Error::new(io::ErrorKind::NotFound, "ベースウェアの根が実在しない"),
            });
        }
        let shelf = root.join(WORK);
        let dir = next_address(&shelf, now, || NEXT_SERIAL.fetch_add(1, Ordering::Relaxed));
        let residue = prepare_shelf(&shelf, &dir, now)?;
        at(&dir, std::fs::create_dir_all(&dir))?;
        Ok(WorkArea { dir, residue })
    }

    /// 作業フォルダそのもの。退避先 `old-<k>` もこの下に作る。
    pub(crate) fn path(&self) -> &Path {
        &self.dir
    }

    /// `k` 番目の配置を組み上げる場所。
    pub(crate) fn stage(&self, k: usize) -> PathBuf {
        self.dir.join(k.to_string())
    }

    /// `k` 番目の配置で宛先を退避しておく場所。
    ///
    /// 番号つきの場所を [`WorkArea::stage`] と同じ作業フォルダの直下に取るので、
    /// 最後の後片付け 1 回で退避も組み上げも一緒に消える。
    fn retired(&self, k: usize) -> PathBuf {
        self.dir.join(format!("old-{k}"))
    }

    /// 棚に残っていて消せなかった物。
    pub(crate) fn residue(&self) -> &[PathBuf] {
        &self.residue
    }
}

/// この走行の番地 `<プロセス識別子>-<連番>` を、保持中の項目を避けて取る。
///
/// 片付けは保持中の項目を消さないので、そこを自分の番地にすると前回の木が混ざる。
/// `serial` は連番の供給源（本番はプロセス全体の連番、テストは 0 始まりの閉包）。
fn next_address(shelf: &Path, now: SystemTime, mut serial: impl FnMut() -> u32) -> PathBuf {
    loop {
        let dir = shelf.join(format!("{}-{}", std::process::id(), serial()));
        if !is_retained(&dir, now) {
            return dir;
        }
    }
}

/// 棚の項目が、`old-` で始まるフォルダ（巻き戻せなかった元の木の退避先）を直下に持ち、
/// 更新時刻から [`SURVIVOR_RETENTION`] 未満か（要件 2.7・2.8）。
///
/// 時刻の根拠は項目自身の更新時刻。`old-<k>` は `rename` で直下へ入るので、項目の直下が
/// 最後に変わった時刻＝失敗した走行が最後に触った時刻になる（`old-<k>` 自身の更新時刻は
/// 利用者のゴーストの最終更新時刻のまま）。読めない・時刻が取れない項目は偽で、今までどおり
/// 消しにいく。時計が戻って更新時刻が `now` より未来なら経過 0 として真に倒す。
fn is_retained(entry: &Path, now: SystemTime) -> bool {
    let Ok(children) = std::fs::read_dir(entry) else {
        return false;
    };
    let holds_survivor = children.flatten().any(|child| {
        child.file_type().is_ok_and(|kind| kind.is_dir())
            && child.file_name().to_string_lossy().starts_with("old-")
    });
    if !holds_survivor {
        return false;
    }
    let Ok(modified) = std::fs::metadata(entry).and_then(|meta| meta.modified()) else {
        return false;
    };
    now.duration_since(modified).unwrap_or(Duration::ZERO) < SURVIVOR_RETENTION
}

/// 棚の残骸を片付け、この走行の番地だけは必ず空にする。
///
/// 他の走行の置き土産（`<別のプロセス識別子>-<連番>/`）は、この走行の木に混ざりようが
/// ない——組み上げも退避もこの走行の番地の中だけで起こる——ので、消せなくても止めない。
/// ただし黙って捨てもせず、消せなかった物を 1 件 1 行で返す（呼び手が結果へ載せる）。
///
/// 一方、この走行の番地そのものが残っていて消せないのは（Windows のプロセス識別子の
/// 再利用で起こり得る）前回の木が完成形に混ざる状態なので、そこだけは失敗を返す。
///
/// 保持の期限の内側にある元の木入りの項目（[`is_retained`]）は 1 バイトも触らず、
/// 残っている物として返す（要件 2.7）。自分の番地は [`next_address`] が保持中の項目を
/// 避けて取るので、ここへは来ない。
fn prepare_shelf(shelf: &Path, dir: &Path, now: SystemTime) -> Result<Vec<PathBuf>, StageError> {
    let mut residue = Vec::new();
    let entries = match std::fs::read_dir(shelf) {
        Ok(entries) => entries,
        // 棚がそもそも無いのは片付けの目的が達成された状態。
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(residue),
        Err(source) => {
            return Err(StageError {
                path: shelf.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            // 何が居るのかすら読めなかった。棚ごと人の目に出す。
            residue.push(shelf.to_path_buf());
            continue;
        };
        let path = entry.path();
        if is_retained(&path, now) {
            residue.push(path);
            continue;
        }
        let removed = match entry.file_type() {
            Ok(kind) if kind.is_dir() => std::fs::remove_dir_all(&path),
            Ok(_) => std::fs::remove_file(&path),
            Err(source) => Err(source),
        };
        if let Err(source) = removed {
            if source.kind() == io::ErrorKind::NotFound {
                continue;
            }
            if path == dir {
                return Err(StageError { path, source });
            }
            residue.push(path);
        }
    }
    Ok(residue)
}

/// 配置 1 つぶんの完成形を `stage` に組み上げ、宛先が確定前にどうだったかを返す。
///
/// 下敷き（既存の宛先）を敷いてから書庫の内容を上書きする。`contents` はエントリ番号で
/// 引ける伸長済みの中身で、**そのまま**書く（要件 5.9）。宛先は読むだけで触らない。
///
/// # Errors
///
/// 既存の複写・フォルダの作成・書き出しのいずれかが失敗したとき [`StageError`]。
pub(crate) fn stage_placement(
    stage: &Path,
    placement: &Placement,
    contents: &[Vec<u8>],
) -> Result<ExistingState, StageError> {
    at(stage, std::fs::create_dir_all(stage))?;

    let existing = if placement.destination.is_dir() {
        match &placement.existing {
            ExistingPolicy::Overlay => {
                copy_existing(&placement.destination, stage, None)?;
                ExistingState::Overlaid
            }
            ExistingPolicy::Replace { keep } => {
                copy_existing(&placement.destination, stage, Some(keep))?;
                ExistingState::Refreshed
            }
        }
    } else {
        ExistingState::New
    };

    // 先にフォルダを全部作る。フォルダのエントリ（要件 4.9 前段）とファイルの親
    // （同後段）が `dirs` で 1 つに揃っているので、ここは区別せずに作れる。
    for relative in &placement.dirs {
        let folder = stage.join(relative);
        at(&folder, std::fs::create_dir_all(&folder))?;
    }
    for (index, relative) in &placement.files {
        // 計画のエントリ番号は読取層が数えた列の添字。範囲を出る経路は今日は無いが、
        // 出たら添字の取り違えなので、黙って別のエントリを書かずにその場で落ちる。
        debug_assert!(
            *index < contents.len(),
            "計画のエントリ番号 {index} が中身の件数 {} を超えている",
            contents.len()
        );
        let file = stage.join(relative);
        at(&file, std::fs::write(&file, &contents[*index]))?;
    }
    Ok(existing)
}

/// 既存の宛先の木を作業フォルダへ複写する。
///
/// `keep` が `None` なら空フォルダも含めて丸ごと（要件 6.1）。`Some` なら、ファイル名が
/// 挙がっているファイルだけを**同じ相対位置**へ（要件 6.2）。後者では残すファイルを
/// 1 つも含まないフォルダは作らない——「全消去してから mask の名前だけ戻す」と同じ形。
fn copy_existing(from: &Path, to: &Path, keep: Option<&[String]>) -> Result<(), StageError> {
    for child in at(from, std::fs::read_dir(from))? {
        let child = at(from, child)?;
        let source = child.path();
        let target = to.join(child.file_name());
        if at(&source, child.file_type())?.is_dir() {
            if keep.is_none() {
                at(&target, std::fs::create_dir_all(&target))?;
            }
            copy_existing(&source, &target, keep)?;
        } else if kept(keep, &child.file_name()) {
            // 残す物が出てきて初めて親を作る（`keep` の側で空フォルダが生えない）。
            at(to, std::fs::create_dir_all(to))?;
            at(&target, std::fs::copy(&source, &target))?;
            // 複写は読み取り専用の属性も運ぶ。作業フォルダの写しは必ず書ける形にする
            // ——さもないと、書庫が同名を持っていたときに続く書き出しが作業フォルダの
            // パスを名指して失敗し、そのパスは次の走行の片付けで消えるので、利用者は
            // 直す場所に辿り着けない。属性は宛先を守るためのもので、入れ替えで丸ごと
            // 新しい木に替わる以上、写しの側で持ち越す意味も無い。
            let mut mode = at(&target, std::fs::metadata(&target))?.permissions();
            // Unix なら誰でも書ける形になる綴りだが、本プロジェクトは Windows 専用で、
            // ここが落とすのは読み取り専用の属性 1 つだけ（設計「Windows 固有」）。
            #[allow(clippy::permissions_set_readonly_false)]
            mode.set_readonly(false);
            at(&target, std::fs::set_permissions(&target, mode))?;
        }
    }
    Ok(())
}

/// このファイル名を下敷きに残すか。
///
/// 除外マスクは**ファイル名**の一覧で、階層を問わず同名に当たる（要件 6.2・正典
/// `descript_install#refreshundeletemask`）。突き合わせは ASCII の大小を無視する
/// （Windows では同じファイルを指す綴り）。
fn kept(keep: Option<&[String]>, name: &OsStr) -> bool {
    match keep {
        None => true,
        Some(keep) => {
            let name = name.to_string_lossy();
            keep.iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(&name))
        }
    }
}

/// 展開が成功したときに返るもの（要件 5.10）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallOutcome {
    /// `install.txt` の `name`（`OnInstallComplete` の参照に写す）。
    pub name: String,
    /// `install.txt` の `accept`。
    pub accept: Option<String>,
    /// インストール済みフォルダ 1 つにつき 1 要素。計画と同じ順。
    pub installed: Vec<InstalledElement>,
    /// 読み飛ばしたキーなど、拒否ではないが黙って通さないもの。
    pub warnings: Vec<ManifestWarning>,
    /// 片付けられずに残った場所（人が消す所）。中身は⑴最後の後片付けが失敗した
    /// ときの作業フォルダ（退避した木も組み上げた木もこの下に居る）と、⑵開始時に
    /// 棚から消せなかった他の走行の置き土産。
    pub leftovers: Vec<PathBuf>,
}

/// 確定に失敗したときの形。`archive` を足すと [`crate::NarError::Io`] になる（タスク 4.4）。
///
/// `path` は**呼び手が手を打てる場所**を指す。巻き戻しまで成功したなら確定を止めた
/// 失敗そのもの（例: 使用中の宛先）、巻き戻しに失敗したなら元へ戻せなかった宛先。
#[derive(Debug)]
pub(crate) struct CommitError {
    pub phase: IoPhase,
    pub path: PathBuf,
    pub source: io::Error,
    /// 失敗までに確定した配置。`rolled_back` が真ならこれらは元へ戻っている。
    pub committed: Vec<InstalledElement>,
    pub rolled_back: bool,
    /// 元へ戻せなかった宛先ごとの、元の木が残っている退避先。`rolled_back` が真なら空。
    pub survivors: Vec<SurvivingTree>,
}

/// 確定済みの配置を元へ戻す 1 手。確定した順に積み、逆順に解く。
enum Undo {
    /// 新規に置いた木を消す。
    Remove(PathBuf),
    /// 退避した木を宛先へ戻す（新しく置いた木が在れば先に消す）。
    ///
    /// 先に消すのは、Windows の `rename` が既に在る宛先を上書きしないため。消えるのは
    /// 書庫から作り直せる新しい木だけで、利用者の元の木は `old` に居る。戻す手が失敗
    /// しても `old` はそのまま残る（失敗の経路では作業フォルダを片付けない）ので、
    /// 取り返しのつかない消え方はしない。
    Restore { old: PathBuf, dest: PathBuf },
}

/// 組み上げ済みの全配置を宛先と入れ替えて確定する（要件 5.10・5.11・6.4・6.6）。
///
/// 配置の番号順に確定し、途中で失敗したら確定済みを逆順に元へ戻す。利用者から見えるのは
/// 「全部入った」か「何も変わっていない」のどちらかだけになる（要件 5.11）。
///
/// 退避した木を消すのは**全配置が確定した後**の後片付け 1 回にまとめてある。配置ごとに
/// 消すと、後の配置が失敗したときに前の配置を戻す元が既に無い——要件 5.11 を満たせない
/// ——ため。退避先は作業フォルダの直下なので、後片付けは `remove_dir_all` 1 回で済む。
///
/// # Errors
///
/// どれかの配置の入れ替えに失敗したとき [`CommitError`]。`rolled_back` が真なら宛先は
/// 全て呼ぶ前の内容で、偽なら `committed` に挙げた配置が確定したまま残っている。
pub(crate) fn commit_all(
    area: &WorkArea,
    manifest: &InstallManifest,
    plan: &[Placement],
    states: &[ExistingState],
) -> Result<InstallOutcome, CommitError> {
    debug_assert_eq!(
        plan.len(),
        states.len(),
        "既存の別は配置ごとに 1 つ（組み上げが返した列をそのまま渡す）"
    );
    let mut installed = Vec::with_capacity(plan.len());
    let mut undo = Vec::with_capacity(plan.len());
    for (k, (placement, existing)) in plan.iter().zip(states).enumerate() {
        if let Err(failure) = commit_one(area, k, placement, *existing, &mut undo) {
            return Err(roll_back(undo, installed, failure));
        }
        installed.push(InstalledElement {
            kind: placement.kind.clone(),
            name: placement.name.clone(),
            path: placement.destination.clone(),
            target_ghost: placement.target_ghost.clone(),
            existing: *existing,
        });
    }

    // 空のまま残すと、開発用の根では原本に写って全複製に伝播する（設計）。失敗しても
    // 確定は取り消さないが、黙って忘れはしない。
    let mut leftovers = area.residue().to_vec();
    if remove_tree(area.path()).is_err() {
        leftovers.push(area.path().to_path_buf());
    }
    Ok(InstallOutcome {
        name: manifest.name.clone(),
        accept: manifest.accept.clone(),
        installed,
        warnings: manifest.warnings.clone(),
        leftovers,
    })
}

/// 1 配置を入れ替えて確定する。
///
/// 宛先が無ければ「作業フォルダ → 宛先」の 1 手。在れば「宛先 → 退避」「作業フォルダ →
/// 宛先」の 2 手で、2 手目が失敗したら退避を戻す（その戻しは [`roll_back`] が積み上がった
/// [`Undo`] を解く形で行う——1 配置目の戻しと同じ一言を通す）。
///
/// 元へ戻す手は**成功した後**に積む。先に積むと、宛先が既に在るのに `New` として来た
/// （組み上げから確定までの間に誰かが作った）ときに、自分が作っていない木を消してしまう。
fn commit_one(
    area: &WorkArea,
    k: usize,
    placement: &Placement,
    existing: ExistingState,
    undo: &mut Vec<Undo>,
) -> Result<(), StageError> {
    let dest = &placement.destination;
    // 格納先（`<根>/ghost` など）は根に無いことがある。`rename` は親を作らない。
    let parent = dest
        .parent()
        .expect("宛先は必ず格納先の下にあるので親がある");
    at(parent, std::fs::create_dir_all(parent))?;

    if existing == ExistingState::New {
        at(dest, std::fs::rename(area.stage(k), dest))?;
        undo.push(Undo::Remove(dest.clone()));
        return Ok(());
    }

    // 宛先の中のファイルが他のプロセスに掴まれていると、Windows はここを拒む
    // （要件 6.6）。解放は試みない。宛先は 1 バイトも動いていない。
    let old = area.retired(k);
    at(dest, std::fs::rename(dest, &old))?;
    undo.push(Undo::Restore {
        old,
        dest: dest.clone(),
    });
    at(dest, std::fs::rename(area.stage(k), dest))
}

/// 確定済みを逆順に元へ戻し、呼び手へ返す失敗を組む。
fn roll_back(
    undo: Vec<Undo>,
    committed: Vec<InstalledElement>,
    failure: StageError,
) -> CommitError {
    let Unwound { stuck, survivors } = unwind(undo);
    match stuck {
        // 全て元へ戻った。報告するのは確定を止めた失敗そのもの。躓きが無いので
        // 生き残りも集まっていない（空）。
        None => CommitError {
            phase: IoPhase::Commit,
            path: failure.path,
            source: failure.source,
            committed,
            rolled_back: true,
            survivors,
        },
        // 戻せなかった。呼び手が手を打てるのは戻せなかった宛先なので、そちらを名指す。
        Some(stuck) => CommitError {
            phase: IoPhase::Rollback,
            path: stuck.path,
            source: stuck.source,
            committed,
            rolled_back: false,
            survivors,
        },
    }
}

/// [`unwind`] の結果。
struct Unwound {
    /// 最初に戻せなかった宛先と理由（戻せていれば `None`）。
    stuck: Option<StageError>,
    /// 戻せなかった「元へ戻す」手ごとの、元の木が残っている退避先。
    survivors: Vec<SurvivingTree>,
}

/// 積んだ手を逆順に解く。最初に戻せなかった宛先と、元の木の生き残りを全て返す。
///
/// 1 つ戻せなくても残りは戻す。戻せる宛先を巻き添えで壊さないため。
///
/// 生き残りに載せるのは、躓いた「元へ戻す」手のうち退避先がフォルダとして実在するもの
/// だけ（要件 2.1〜2.3）。どちらの手で躓いても退避先には触れていないので、元の木は
/// そのまま残っている。新規の宛先を消す手の躓きは、宛先がもともと無かった＝元の木が
/// 無いので載せない。
fn unwind(undo: Vec<Undo>) -> Unwound {
    let mut stuck = None;
    let mut survivors = Vec::new();
    for step in undo.into_iter().rev() {
        let (dest, result) = match step {
            Undo::Remove(dest) => {
                let result = remove_tree(&dest);
                (dest, result)
            }
            Undo::Restore { old, dest } => {
                // 新しく置いた木が在れば先に退ける。2 手目の `rename` が失敗した直後は
                // 宛先が空いているので、その場合は何もせずに戻しへ進む。
                let result = remove_tree(&dest).and_then(|()| std::fs::rename(&old, &dest));
                if result.is_err() && old.is_dir() {
                    survivors.push(SurvivingTree {
                        destination: dest.clone(),
                        path: old,
                    });
                }
                (dest, result)
            }
        };
        if let Err(source) = result {
            stuck.get_or_insert(StageError { path: dest, source });
        }
    }
    Unwound { stuck, survivors }
}

/// 木ごと消す。既に無いのは消えている状態として通す。
fn remove_tree(path: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(path) {
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
#[path = "install_tests.rs"]
mod tests;
