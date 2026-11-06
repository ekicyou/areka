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
//! use areka_nar::{InstallKind, InstallOutcome, InstallRequest, NarArchive, NarError, SingleRoot};
//! use std::path::Path;
//!
//! fn install(nar: &Path, root: &Path) -> Result<InstallOutcome, NarError> {
//!     // ⑴ 読む。ここを通れば書庫は全て検証済みで、宛先には 1 バイトも触れていない。
//!     let archive = NarArchive::open(nar)?;
//!     if archive.manifest().kind == InstallKind::Ghost {
//!         println!("{} を入れます", archive.manifest().name);
//!     }
//!     // ⑵ 入れる。失敗したら宛先は呼ぶ前のまま（巻き戻せなかったぶんは理由に載る）。
//!     match archive.install(&InstallRequest { roots: &SingleRoot(root), target_ghost: None }) {
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

use crate::container::{
    MAX_TOTAL_DECLARED_SIZE, TAIL_WINDOW, archive_too_large, inflate_entry,
    locate_central_directory, read_central_directory, read_central_headers,
};
use crate::install::{WorkArea, commit_all, stage_placement};
use crate::names::{EntryName, same_name, validate_entry_names};
use crate::plan::build_plan;
use std::io::{Read, Seek, SeekFrom};
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
    RefuseReason, SurvivingTree, UnsafeWhy, Unsupported,
};
pub use install::{InstallOutcome, InstallRequest, InstallRoots, SingleRoot, SkippedCompanion};
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
        Self::read(path).map_err(|failure| log_failure(failure, &[]))
    }

    /// `install.txt` の解釈の結果。呼び出し側が `accept` の照合や宛先の選択に使う。
    pub fn manifest(&self) -> &InstallManifest {
        &self.manifest
    }

    /// 書庫の中のファイル 1 つの伸長済みの中身。`path` は書庫の中の `/` 区切りのパスで、
    /// 大文字小文字を区別しない（ASCII に限らない）。フォルダと、無いパスは `None`。
    ///
    /// 保持している中身を引くだけで、原本の読み直しも伸長のやり直しもしない。
    /// 何も書かず、記録も出さない。
    pub fn entry_bytes(&self, path: &str) -> Option<&[u8]> {
        self.names
            .iter()
            .find(|entry| !entry.is_dir && same_name(&entry.path, path))
            .map(|entry| self.contents[entry.index].as_slice())
    }

    /// 検証済みの中身をベースウェアの根へ入れる（要件 5.10・5.11）。
    ///
    /// 何度呼んでも同じ結果になる。2 度目以降は既存の宛先の扱い（[`ExistingState`]）が
    /// `New` から `Overlaid`／`Refreshed` に変わるだけで、置かれる木は同じ。
    ///
    /// # Errors
    ///
    /// 宛先ゴーストが無いとき [`NarError::Refused`]（宛先には触れていない）。同梱バルーンの
    /// 取り出し元が無いときは断らず、その同梱を飛ばして [`InstallOutcome::skipped`] に載せる。組み上げ・確定・巻き戻しの I/O が失敗したとき
    /// [`NarError::Io`]（どこまで確定したかと巻き戻せたかを持つ＝要件 6.4）。
    pub fn install(&self, request: &InstallRequest<'_>) -> Result<InstallOutcome, NarError> {
        let mut work = Vec::new();
        let mut overridden = Vec::new();
        let placed = self.place(request, &mut work, &mut overridden);
        // 書庫の `balloon/<dir>/` を名指しされた同梱が本体から除いた（計画の事実なので成否に関わらず）。
        for directory in &overridden {
            tracing::warn!(
                event = "companion_overrides_archive_folder",
                archive = %self.archive.display(),
                directory = directory.as_str(),
                "[areka_nar] archive folder at a companion balloon home dropped from the ghost body"
            );
        }
        match placed {
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
                // 取り出し元の無い同梱は断らずに飛ばした（要件 1.4）。1 件ずつ出す。
                for skip in &outcome.skipped {
                    tracing::warn!(
                        archive = %self.archive.display(),
                        key = skip.key.as_str(),
                        source = skip.source.as_str(),
                        directory = skip.directory.as_str(),
                        "[areka_nar] companion balloon skipped: its source folder is not in the archive"
                    );
                }
                tracing::info!(
                    archive = %self.archive.display(),
                    installed = outcome.installed.len(),
                    skipped = outcome.skipped.len(),
                    "[areka_nar] installed"
                );
                Ok(outcome)
            }
            Err(failure) => Err(log_failure(failure, &work)),
        }
    }

    /// 読み取りと全検証。記録は出さない（出口は [`NarArchive::open`] 1 つ）。
    fn read(path: &Path) -> Result<NarArchive, NarError> {
        let archive = path.to_path_buf();
        let refused = |reason| NarError::Refused {
            archive: path.to_path_buf(),
            reason,
        };
        let bytes = read_archive_bytes(path)?;

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
        let install_txt = manifest::locate_install_txt(&names).map_err(refused)?;
        let index = install_txt.index;
        if contents[index].len() > MAX_INSTALL_TXT_LEN {
            return Err(refused(RefuseReason::UnsupportedEntry {
                index,
                name: install_txt.path.clone(),
                what: Unsupported::TooLarge,
            }));
        }
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
    /// 作業フォルダは本体の根に 1 つ掘り（同梱も本体の根に置く）、`work` へ積む
    /// （呼び手が記録の欄として出す。途中で失敗しても掘った分は載る）。元の木を持たない失敗
    /// （組み上げの失敗・巻き戻せた確定の失敗）では、作業フォルダはここで消しにかかる
    /// （消せなかった残りは次の走行の片付けが拾う）。
    /// `rolled_back` が偽のとき、利用者の元の木が残っている場所は失敗の値の
    /// `survivors` が持ち、いずれも `work` に積んだ本体の根の作業フォルダ（1 つ）の配下に在る。
    /// 書庫の `balloon/<dir>/` を本体から除いた同梱の名前は `overridden` へ積む（同じく記録の材料）。
    fn place(
        &self,
        request: &InstallRequest<'_>,
        work: &mut Vec<PathBuf>,
        overridden: &mut Vec<String>,
    ) -> Result<InstallOutcome, NarError> {
        let refused = |reason| NarError::Refused {
            archive: self.archive.clone(),
            reason,
        };
        let (plan, skipped, dropped) =
            build_plan(&self.manifest, &self.names, request).map_err(refused)?;
        *overridden = dropped;

        // 棚の掃除は掘るときに行うので、触るのはこの `.nar` の根の棚だけ。計画の先頭は
        // 必ず本体で、同梱も本体の根に置く（`build_plan`）。
        debug_assert!(
            plan.iter().all(|placement| placement.root == plan[0].root),
            "計画の配置は全て本体の根に置く"
        );
        let area = WorkArea::create(&plan[0].root)
            .map_err(|failure| self.io(IoPhase::Stage, failure.path, failure.source))?;
        work.push(area.path().to_path_buf());

        // 全部を組み終えてから確定へ進む（どの根にもまだ 1 バイトも書いていない）。
        let mut states = Vec::with_capacity(plan.len());
        for (k, placement) in plan.iter().enumerate() {
            let state =
                stage_placement(&area.stage(k), placement, &self.contents).map_err(|failure| {
                    // 宛先には触れていないので、作業フォルダに在るのは組み上げの途中だけ。
                    area.discard();
                    self.io(IoPhase::Stage, failure.path, failure.source)
                })?;
            states.push(state);
        }

        commit_all(&area, &self.manifest, &plan, skipped, &states).map_err(|failure| NarError::Io {
            archive: self.archive.clone(),
            phase: failure.phase,
            path: failure.path,
            source: failure.source,
            committed: failure.committed,
            rolled_back: failure.rolled_back,
            survivors: failure.survivors.into_boxed_slice(),
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
            survivors: Box::default(),
        }
    }
}

/// 最上位に `install.txt` が在るか。書庫の目次（中央ディレクトリ）と名前の検証までで止め、
/// 伸長も書き込みも記録も無い（要件 1.9・2.2・2.4）。
///
/// `Ok(true)`＝在る（ASCII の大文字小文字は問わない）／`Ok(false)`＝無い（包みフォルダの
/// 下にだけ在るものを含む）／`Err`＝目次が読めない。探し方は [`NarArchive::open`] と同じ
/// `manifest::locate_install_txt` を同じ順（名前の検証 → 探す）で使うので、同じ書庫に
/// 対して手続きと答えが食い違わない。
///
/// **名前の検証で撥ねられる書庫**（`..` を含む・絶対パス・大文字小文字の衝突など）は、
/// 最上位に `install.txt` が在っても `Err`（[`NarError::Refused`]）＝「読めない」になる。
/// 手続きも `install.txt` を探す前に同じ理由で断るので、「`install.txt` を持つ物」と
/// 扱われたことが無い。生の名前で探す道は、探し方が 2 つになるので取らない。
///
/// **読むのは書庫の末尾（終端記録の窓）と目次だけ**で、各エントリのローカルヘッダは
/// 見ない。そのため**ローカルヘッダだけが壊れた書庫**は、判定が `Ok(true)`（在る）と
/// 答え、`open` が [`RefuseReason::CorruptArchive`] で断る（インストール対象として扱い、
/// 入らなければインストールの失敗になる）。目次と名前で決まる答えは `open` と一致する。
/// 書庫が受け入れる大きさの上限を越えるときは、`open` と同じく読む前に断る。
///
/// **記録を出さないのは意図**。「記録はここだけが出す」は `open`／`install` の契約で、
/// これは問い合わせ——読めなかったことの記録は呼び手が 1 回出す。
///
/// # Errors
///
/// 読み取りに失敗したとき [`NarError::Io`]（`phase: Read`）。構造または名前が通らない
/// とき [`NarError::Refused`]。中身は `open` が同じ書庫で返すものと同じ語彙。
pub fn peek_install_txt(path: &Path) -> Result<bool, NarError> {
    let refused = |reason| NarError::Refused {
        archive: path.to_path_buf(),
        reason,
    };
    let failed = read_failed(path);
    let (mut file, len) = open_within(path, MAX_ARCHIVE_LEN)?;
    let tail_len = len.min(TAIL_WINDOW as u64);
    let mut tail = vec![0; tail_len as usize];
    file.seek(SeekFrom::End(-(tail_len as i64)))
        .and_then(|_| file.read_exact(&mut tail))
        .map_err(&failed)?;
    let span = locate_central_directory(&tail, len).map_err(refused)?;
    let mut cd = vec![0; span.size];
    file.seek(SeekFrom::Start(span.offset))
        .and_then(|_| file.read_exact(&mut cd))
        .map_err(&failed)?;
    let raw = read_central_headers(&cd, &span, None).map_err(refused)?;
    let names = validate_entry_names(&raw).map_err(refused)?;
    Ok(manifest::locate_install_txt(&names).is_ok())
}

/// 受け入れる `install.txt` の長さの上限（1 MiB）。実物は数百バイトで、警告の数を縛るため。
const MAX_INSTALL_TXT_LEN: usize = 1024 * 1024;

/// 受け入れる書庫の長さの上限（宣言の総和の上限の 2 倍）。
///
/// 受け入れる中身は宣言の総和で [`MAX_TOTAL_DECLARED_SIZE`] まで。格納の中身はその大きさ
/// のまま、圧縮の中身は縮まない時でもほぼ同じ大きさで、頭の分を入れても 2 倍は越えない。
/// 越える書庫は全部読んでも必ず断ることになるので、読む前に断る。
const MAX_ARCHIVE_LEN: u64 = 2 * MAX_TOTAL_DECLARED_SIZE;

/// 書庫を丸ごと読む。失敗は `phase: Read` の [`NarError::Io`]（確定 0 件）。記録は出さない。
fn read_archive_bytes(path: &Path) -> Result<Vec<u8>, NarError> {
    read_archive_bytes_within(path, MAX_ARCHIVE_LEN)
}

/// 長さが `limit` を越える書庫は読まずに「大きすぎる」で断り、越えなければ丸ごと読む。
fn read_archive_bytes_within(path: &Path, limit: u64) -> Result<Vec<u8>, NarError> {
    let (file, len) = open_within(path, limit)?;
    read_capped(file, len, limit, path)
}

/// 測った後に書庫が伸びても `limit` を越えては読まず、越えた分が在れば「大きすぎる」で断る。
fn read_capped(reader: impl Read, len: u64, limit: u64, path: &Path) -> Result<Vec<u8>, NarError> {
    let mut bytes = Vec::with_capacity(len as usize);
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(read_failed(path))?;
    if bytes.len() as u64 > limit {
        return Err(NarError::Refused {
            archive: path.to_path_buf(),
            reason: archive_too_large(),
        });
    }
    Ok(bytes)
}

/// 書庫を開いて長さを測る。長さが `limit` を越えれば読む前に断る。
fn open_within(path: &Path, limit: u64) -> Result<(std::fs::File, u64), NarError> {
    let failed = read_failed(path);
    let file = std::fs::File::open(path).map_err(&failed)?;
    let len = file.metadata().map_err(&failed)?.len();
    if len > limit {
        return Err(NarError::Refused {
            archive: path.to_path_buf(),
            reason: archive_too_large(),
        });
    }
    Ok((file, len))
}

/// 書庫の読み取りの失敗を `phase: Read` の [`NarError::Io`]（確定 0 件）にする。
fn read_failed(path: &Path) -> impl Fn(std::io::Error) -> NarError + '_ {
    move |source| NarError::Io {
        archive: path.to_path_buf(),
        phase: IoPhase::Read,
        path: path.to_path_buf(),
        source,
        committed: Vec::new(),
        rolled_back: true,
        survivors: Box::default(),
    }
}

/// 失敗の記録を 1 回だけ出し、受け取った失敗をそのまま返す（要件 9.1・6.4）。
///
/// `NarError` の表示は確定済みの件数も巻き戻せたかも持たない（設計の逐語）ので、
/// 要件 6.4 が見えることを求めている 2 つは欄として足す。`work` はこの走行が掘った
/// 本体の根の作業フォルダ（1 つ）の場所で、巻き戻せたかに関わらず載せる（掘る前の
/// 失敗と拒否では空）。元の木を持たない失敗（組み上げの失敗・巻き戻せた確定の失敗）では、
/// その場所はその場で消しにかかる（消せなかった残りは次の走行の片付けが拾う）。
fn log_failure(failure: NarError, work: &[PathBuf]) -> NarError {
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
        work = %work
            .iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join("; "),
        "[areka_nar] refused or failed"
    );
    failure
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "lib_entry_tests.rs"]
mod entry_tests;

#[cfg(test)]
#[path = "lib_peek_tests.rs"]
mod peek_tests;

#[cfg(test)]
#[path = "lib_skip_tests.rs"]
mod skip_tests;
