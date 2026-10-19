//! 並びでインストールの宛先の根に答える部品（design.md「InstallDestination」・要件 5.1〜5.3・5.6）。
//!
//! 答えは `catalog_roots::install_root` から取る（新しいものは見つかるいちばん上の根・すでにあるものは
//! 見えている根）。どの答えも並びの写しの綴りのまま返す（`areka-nar` は作業フォルダを根の綴りの
//! 完全一致でまとめるため・tasks.md Implementation Notes 1.5）。行き先を尋ねる画面は出さない（5.6）。

use std::path::{Path, PathBuf};

use areka_ghost::catalog_roots::{
    StoreKind, find_balloon, find_ghost, install_root, same_folder_name, same_root,
};
use areka_ghost::{BasewareRoot, BasewareRoots};
use areka_nar::{InstallOutcome, InstallRequest, IoPhase, NarArchive, NarError};

/// 並びの写しで `InstallRoots` に答える。見つかる根が 1 つも無ければ作れない。
pub(crate) struct CatalogDestinations {
    /// 並びの写しから一時の置き場を除いたもの（一時の置き場は入れ先にしない・要件 3.5）。
    roots: BasewareRoots,
    /// 除いた一時の置き場（起動中のゴーストがそこにいるかを見る）。
    temporary: Vec<PathBuf>,
    /// 作ったときの見つかるいちばん上の根（答えが出ないときに答える）。
    first: PathBuf,
    /// 起動中のゴーストの場所（`<根>\ghost\<フォルダ>`・根の下のゴーストのときだけ）。
    running: Option<PathBuf>,
    /// 実行ファイルのフォルダ（写す判断に渡す・テストは [`Self::with_exe_dir`] で差し替える）。
    exe_dir: Option<PathBuf>,
}

impl CatalogDestinations {
    /// 見つかる根が無ければ `None`（呼び手が error `install_no_root` を残して今の失敗の経路へ倒す）。
    /// 一時の置き場は並びから除く（同じフォルダ名の相手・控えの根・記録の根に使わない・要件 3.5・3.9）。
    pub(crate) fn new(roots: BasewareRoots) -> Option<Self> {
        let (temporary, kept): (Vec<_>, Vec<_>) = roots
            .roots()
            .iter()
            .cloned()
            .partition(BasewareRoot::is_temporary);
        let roots = BasewareRoots::new(kept);
        let first = roots.present().next()?.dir().to_path_buf();
        Some(Self {
            roots,
            temporary: temporary.iter().map(|r| r.dir().to_path_buf()).collect(),
            first,
            running: None,
            exe_dir: crate::roots::user_root::exe_dir(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_exe_dir(mut self, exe_dir: Option<PathBuf>) -> Self {
        self.exe_dir = exe_dir;
        self
    }

    /// 起動中のゴーストの場所を渡す（要件 8.7: 起動中のゴーストへの上書きはその根へ書く）。
    /// 一時の置き場にいるゴーストは渡さなかったことにする（`<一時の置き場>\<フォルダ>` の 2 つ上は
    /// 置き場の外なので、入れ先は並びの答え＝覚える置き場に任せる・要件 3.5）。
    pub(crate) fn with_running(mut self, running: Option<PathBuf>) -> Self {
        self.running = running.filter(|dir| {
            !dir.parent()
                .is_some_and(|p| self.temporary.iter().any(|t| same_root(t, p)))
        });
        self
    }

    /// 尋ねたフォルダ名が起動中のゴーストと同じ（`same_folder_name`）なら、起動中のゴーストの根。
    /// 並びに在ればその綴り、無ければ場所から導いた綴り（並びから外れても切替と同じゴーストへ書く）。
    fn running_root(&self, folder: &str) -> Option<PathBuf> {
        let dir = self.running.as_deref()?;
        let running = dir.file_name()?.to_str()?;
        if !same_folder_name(running, folder) {
            return None;
        }
        let derived = dir.parent()?.parent()?;
        let root = self
            .roots
            .roots()
            .iter()
            .map(|r| r.dir())
            .find(|d| same_root(d, derived))
            .unwrap_or(derived)
            .to_path_buf();
        tracing::info!(
            event = "install_running_ghost_root_kept",
            folder = %running,
            root = %root.display(),
            "[install] 起動中のゴーストのフォルダ名なので、起動中のゴーストの根へ入れる"
        );
        Some(root)
    }

    /// 見えている物が実行ファイルの隣の根に在れば写し、写しになったら書ける根の綴り（要件 4.4・4.8）。
    /// 並びに印の付いた項目が無ければ何も見ない（要件 7.1）。1 配置につき 1 回尋ねられるので写すのも 1 回。
    fn copied_root(&self, kind: StoreKind, folder: &str) -> Option<PathBuf> {
        let user = self.roots.user_root()?.dir();
        let dir = match kind {
            StoreKind::Ghost => find_ghost(&self.roots, folder)?.dir,
            StoreKind::Balloon => find_balloon(&self.roots, folder)?.dir,
        };
        let used = crate::roots::user_root::copy_for_write(
            &self.roots,
            self.exe_dir.as_deref(),
            kind,
            &dir,
        );
        (used != dir).then(|| user.to_path_buf())
    }

    fn answer(&self, kind: StoreKind, folder: &str) -> PathBuf {
        if let Some(root) = self.copied_root(kind, folder) {
            return root;
        }
        // 答えが出ない（尋ねた間に全部の根が消えた）ときは作ったときの根を答える。
        // `areka-nar` が作業フォルダを掘れずに失敗として返し、記録は `areka-nar` の今の `error!`。
        let Some(dest) = install_root(&self.roots, kind, folder) else {
            return self.first.clone();
        };
        if let Some(head) = &dest.head_skipped {
            log_head_skipped(head, &dest.root);
        }
        dest.root
    }
}

impl areka_nar::InstallRoots for CatalogDestinations {
    fn ghost_root(&self, folder: &str) -> PathBuf {
        self.running_root(folder)
            .unwrap_or_else(|| self.answer(StoreKind::Ghost, folder))
    }

    fn balloon_root(&self, folder: &str) -> PathBuf {
        self.answer(StoreKind::Balloon, folder)
    }
}

/// 並びの写しを宛先にして書庫を入れる（背景のスレッドと起動中のゴーストへ入れる一周の共通の口）。
/// `running` は起動中のゴーストの場所（根の下のゴーストのときだけ・[`CatalogDestinations::with_running`]）。
/// 見つかる根が無ければ入れずに error `install_no_root` を残し、書く前の I/O の失敗として返す
/// （手続きの今の失敗の経路へ倒す）。入れ終えたら要素ごとに info `install_placed`（要件 5.7）。
pub(crate) fn install_into(
    roots: &BasewareRoots,
    archive: &NarArchive,
    target_ghost: Option<&str>,
    running: Option<&Path>,
) -> Result<InstallOutcome, NarError> {
    let Some(dest) = CatalogDestinations::new(roots.clone())
        .map(|d| d.with_running(running.map(Path::to_path_buf)))
    else {
        let dirs: Vec<_> = roots
            .roots()
            .iter()
            .map(|r| r.dir().display().to_string())
            .collect();
        tracing::error!(
            event = "install_no_root",
            roots = ?dirs,
            directory = %archive.manifest().directory,
            "[install] 見つかる根が 1 つも無いので、入れられません"
        );
        let path = roots
            .roots()
            .first()
            .map(|r| r.dir().to_path_buf())
            .unwrap_or_default();
        return Err(NarError::Io {
            archive: PathBuf::new(),
            phase: IoPhase::Stage,
            path,
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "見つかる根が無い"),
            committed: Vec::new(),
            rolled_back: true,
            survivors: Box::default(),
        });
    };
    let outcome = archive.install(&InstallRequest {
        roots: &dest,
        target_ghost,
    })?;
    for element in &outcome.installed {
        // 答えは並びの写しの綴りなので、置いたパスはどれかの根の綴りで始まる（並びから外れた
        // 起動中のゴーストの根へ入れたときだけ、その場所から導いた根）。
        let root = dest
            .roots
            .roots()
            .iter()
            .map(|r| r.dir())
            .chain(dest.running.as_deref().and_then(|d| d.parent()?.parent()))
            .find(|dir| element.path.starts_with(dir));
        tracing::info!(
            event = "install_placed",
            kind = ?element.kind,
            root = %root.map(|d| d.display().to_string()).unwrap_or_default(),
            folder = %element.path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(),
            "[install] 入れた先"
        );
    }
    Ok(outcome)
}

fn log_head_skipped(head: &Path, root: &Path) {
    tracing::warn!(
        event = "install_head_root_skipped",
        head_root = %head.display(),
        root = %root.display(),
        "先頭の根が見つからないので、見つかるいちばん上の根へ入れる"
    );
}

#[cfg(test)]
#[path = "destination_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "destination_temporary_tests.rs"]
mod temporary_tests;
