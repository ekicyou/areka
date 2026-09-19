//! `.nar`（ゴースト／バルーンの配布アーカイブ）を読み、呼び出し側から受け取った
//! ベースウェアの根へ入れるエンジン。
//!
//! 公開面はコンテナ読取・エントリ名の復号と検証・`install.txt` の解釈・配置計画・
//! 原子的な展開と `refresh`・閉じた拒否語彙（[`RefuseReason`]）で構成する。
//! 根の場所は決めず、`&Path` として受け取る（決めるのは `baseware-root-layout`）。
//! 検体の在処も知らない（窓口は開発専用の `sample-ghost-kit` にある）。
//!
//! # 2 段で使う
//!
//! ```rust,no_run
//! use areka_nar::{InstallKind, InstallOutcome, InstallRequest, NarArchive, NarError};
//! use std::path::Path;
//!
//! fn install(nar: &Path, root: &Path) -> Result<InstallOutcome, NarError> {
//!     // ⑴ 読む。ここを通れば書庫は全て検証済みで、宛先には 1 バイトも触れていない。
//!     let archive = NarArchive::open(nar)?;
//!     if archive.manifest().kind == InstallKind::Ghost {
//!         println!("{} を入れます", archive.manifest().name);
//!     }
//!     // ⑵ 入れる。失敗したら宛先は呼ぶ前のまま（巻き戻せなかったぶんは理由に載る）。
//!     match archive.install(&InstallRequest { root, target_ghost: None }) {
//!         Err(NarError::Refused { reason, .. }) => {
//!             // 短い語をそのまま `OnInstallFailure` の理由に写せる。
//!             println!("拒否: {}", reason.kind());
//!             Err(NarError::Refused { archive: nar.to_path_buf(), reason })
//!         }
//!         other => other,
//!     }
//! }
//! # let _ = install;
//! ```
//!
//! # 記録はここだけが出す
//!
//! [`NarArchive::open`] と [`NarArchive::install`] が `Err` を返す直前に、失敗の記録を
//! **1 回だけ**出す（[`log_failure`]）。下の層は理由を値として返すだけで記録を出さない
//! ——出すと、ログを読む人には 1 回の失敗が 2 回に見える。記録の無い失敗経路も持たない
//! （要件 9.1）。

use crate::container::{inflate_entry, read_central_directory};
use crate::install::{WorkArea, commit_all, stage_placement};
use crate::names::{EntryName, validate_entry_names};
use crate::plan::build_plan;
use std::path::{Path, PathBuf};

mod container;
mod crc32;
mod error;
mod install;
mod manifest;
mod names;
mod plan;

pub use crc32::crc32;
pub use error::{
    ElementKind, ExistingState, InstalledElement, Integrity, IoPhase, ManifestWarning, NarError,
    RefuseReason, UnsafeWhy, Unsupported,
};
pub use install::{InstallOutcome, InstallRequest};
pub use manifest::{Companion, ExistingPolicy, InstallKind, InstallManifest};

/// 検証を終えた `.nar` 1 本。
///
/// [`NarArchive::open`] を通った時点で、コンテナの構造・全エントリの伸長と CRC の突合・
/// 名前の復号と安全性・`install.txt` の解釈まで全て済んでいる。伸長済みのバイト列は
/// この値が持つので、[`NarArchive::install`] は**保持したものを書くだけ**で、原本を
/// 読み直すことも伸長をやり直すこともない（設計 `container`）。
pub struct NarArchive {
    /// 読んだ `.nar` のパス。失敗の理由と記録に載せる。
    archive: PathBuf,
    manifest: InstallManifest,
    names: Vec<EntryName>,
    /// エントリ番号で引ける伸長済みの中身。並びは中央ディレクトリの順。
    contents: Vec<Vec<u8>>,
}

impl NarArchive {
    /// `.nar` を読み、全ての検証を済ませる。**1 バイトも書かない**（要件 4.1）。
    ///
    /// # Errors
    ///
    /// 読み取りに失敗したとき [`NarError::Io`]（`phase: Read`）。構造・整合性・名前・
    /// マニフェストのいずれかが通らなかったとき [`NarError::Refused`]。
    pub fn open(path: &Path) -> Result<NarArchive, NarError> {
        Self::read(path).map_err(|failure| log_failure(failure, None))
    }

    /// `install.txt` の解釈の結果。呼び出し側が `accept` の照合や宛先の選択に使う。
    pub fn manifest(&self) -> &InstallManifest {
        &self.manifest
    }

    /// 検証済みの中身をベースウェアの根へ入れる（要件 5.10・5.11）。
    ///
    /// 何度呼んでも同じ結果になる。2 度目以降は既存の宛先の扱い（[`ExistingState`]）が
    /// `New` から `Overlaid`／`Refreshed` に変わるだけで、置かれる木は同じ。
    ///
    /// # Errors
    ///
    /// 宛先ゴーストが無い・同梱バルーンの取り出し元が無いとき [`NarError::Refused`]
    /// （宛先には触れていない）。組み上げ・確定・巻き戻しの I/O が失敗したとき
    /// [`NarError::Io`]（どこまで確定したかと巻き戻せたかを持つ＝要件 6.4）。
    pub fn install(&self, request: &InstallRequest<'_>) -> Result<InstallOutcome, NarError> {
        let mut work = None;
        match self.place(request, &mut work) {
            Ok(outcome) => {
                // 拒否ではないが黙って通さないもの（要件 3.9）。1 件ずつ出す。
                for warning in &outcome.warnings {
                    tracing::warn!(
                        archive = %self.archive.display(),
                        warning = %warning,
                        "[areka_nar] manifest entry skipped"
                    );
                }
                // 人が消す所。放っておくと開発用の根では原本に写って全複製へ伝播する。
                for leftover in &outcome.leftovers {
                    tracing::warn!(
                        archive = %self.archive.display(),
                        path = %leftover.display(),
                        "[areka_nar] work folder left behind"
                    );
                }
                tracing::info!(
                    archive = %self.archive.display(),
                    installed = outcome.installed.len(),
                    "[areka_nar] installed"
                );
                Ok(outcome)
            }
            Err(failure) => Err(log_failure(failure, work)),
        }
    }

    /// 読み取りと全検証。記録は出さない（出口は [`NarArchive::open`] 1 つ）。
    fn read(path: &Path) -> Result<NarArchive, NarError> {
        let archive = path.to_path_buf();
        let refused = |reason| NarError::Refused {
            archive: path.to_path_buf(),
            reason,
        };
        let bytes = std::fs::read(path).map_err(|source| NarError::Io {
            archive: archive.clone(),
            phase: IoPhase::Read,
            path: archive.clone(),
            source,
            committed: Vec::new(),
            rolled_back: true,
        })?;

        let raw = read_central_directory(&bytes).map_err(refused)?;
        let names = validate_entry_names(&raw).map_err(refused)?;
        // 伸長と CRC の突合は**全エントリに対して**ここで済ませ、結果を保持する。
        // これで「壊れていれば 1 バイトも書かない」（要件 2.6）と「書き込みの前に
        // 検証を終える」（要件 4.1）が、展開の段の書き方によらず成り立つ。
        let contents = raw
            .iter()
            .map(|entry| inflate_entry(&bytes, entry))
            .collect::<Result<Vec<_>, _>>()
            .map_err(refused)?;
        let index = manifest::locate_install_txt(&names).map_err(refused)?.index;
        let manifest = manifest::parse_manifest(&contents[index]).map_err(refused)?;

        Ok(NarArchive {
            archive,
            manifest,
            names,
            contents,
        })
    }

    /// 計画 → 組み上げ → 確定。記録は出さない（出口は [`NarArchive::install`] 1 つ）。
    ///
    /// 作業フォルダを掘ったら、その場所を `work` へ置く。`rolled_back` が偽のとき、
    /// 利用者の元の木はその下の `old-<k>/` に残っている——[`NarError`] はその場所を持つ
    /// 形をしていない（設計の逐語）ので、呼び手が記録の欄として出す。失敗の型に足すと
    /// 公開面の形が設計から離れるので、戻り値ではなく預かり先で渡す。
    fn place(
        &self,
        request: &InstallRequest<'_>,
        work: &mut Option<PathBuf>,
    ) -> Result<InstallOutcome, NarError> {
        let refused = |reason| NarError::Refused {
            archive: self.archive.clone(),
            reason,
        };
        let plan = build_plan(&self.manifest, &self.names, request).map_err(refused)?;

        let area = WorkArea::create(request.root)
            .map_err(|failure| self.io(IoPhase::Stage, failure.path, failure.source))?;
        *work = Some(area.path().to_path_buf());

        let mut states = Vec::with_capacity(plan.len());
        for (k, placement) in plan.iter().enumerate() {
            let state = stage_placement(&area.stage(k), placement, &self.contents)
                .map_err(|failure| self.io(IoPhase::Stage, failure.path, failure.source))?;
            states.push(state);
        }

        commit_all(&area, &self.manifest, &plan, &states).map_err(|failure| NarError::Io {
            archive: self.archive.clone(),
            phase: failure.phase,
            path: failure.path,
            source: failure.source,
            committed: failure.committed,
            rolled_back: failure.rolled_back,
        })
    }

    /// 宛先に触れていない段階の I/O 失敗。確定は 0 件で、宛先は呼ぶ前のまま。
    fn io(&self, phase: IoPhase, path: PathBuf, source: std::io::Error) -> NarError {
        NarError::Io {
            archive: self.archive.clone(),
            phase,
            path,
            source,
            committed: Vec::new(),
            rolled_back: true,
        }
    }
}

/// 失敗の記録を 1 回だけ出し、受け取った失敗をそのまま返す（要件 9.1・6.4）。
///
/// `NarError` の表示は確定済みの件数も巻き戻せたかも持たない（設計の逐語）ので、
/// 要件 6.4 が見えることを求めている 2 つは欄として足す。`work` は巻き戻せなかった
/// ときに利用者の元の木が残っている作業フォルダ（`rolled_back` が真なら空）。
fn log_failure(failure: NarError, work: Option<PathBuf>) -> NarError {
    let (archive, committed, rolled_back) = match &failure {
        // 拒否は書く前に止まっているので、確定は 0 件・宛先は呼ぶ前のまま。
        NarError::Refused { archive, .. } => (archive.clone(), 0, true),
        NarError::Io {
            archive,
            committed,
            rolled_back,
            ..
        } => (archive.clone(), committed.len(), *rolled_back),
    };
    tracing::error!(
        archive = %archive.display(),
        reason = %failure,
        committed,
        rolled_back,
        work = %work.unwrap_or_default().display(),
        "[areka_nar] refused or failed"
    );
    failure
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
