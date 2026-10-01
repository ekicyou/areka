//! 新しいシェル・バルーンの資産を背景のスレッドで作る部品と、荷物の置き場
//! （spec: areka-P0-shell-balloon-switch 要件 4.4・5.5・5.6・design「SwitchAssets」）。
//!
//! 受理ごとに [`spawn_switch_build`] が `std::thread` を 1 本起こし、COM（MTA）の上で WIC を使って
//! 資産を作り、結果を線で UI へ返す。読み込みと復号はすべてこのスレッドで済ませ、UI は結果を
//! 受け取るだけにする（画面を進める処理の外・要件 4.4）。失敗は差し替えの前に決まり、元の表示は
//! そのまま残る（要件 5.5・5.6）。
//!
//! - 失敗はスレッドの中でも `error!`（`event = "switch_assets_failed"`）を 1 件残し、
//!   [`SwitchBuildError`] を線で返す。
//! - スレッドが倒れたら送り手が落ちるので、受け手は切断を見る（UI が `worker_gone` として記録する）。
//! - 作り直しは受理ごと（`EmoWorld` は複製できない）。
//!
//! 荷物 [`SwapPayload`] と置き場 [`SwapSlot`] は、UI が置いてから seriko へ差し替えを頼み、
//! seriko のスレッドの表示の橋渡しが合図の世代と突き合わせて取り出すための器である。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use areka_actor::ReplySender;
use areka_emo_atlas::{AtlasTable, WicDecoderArm};
use areka_emo_compose::EmoWorld;
use areka_emo_present::PresentOutcome;
use areka_parsers::charset::DefaultEncoding;
use areka_sylphya::PersistKey;
use tracing::{debug, error, info};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

use super::BootWiringError;
use super::assets::{BalloonAssets, ShellAssets, build_balloon_assets, build_shell_assets};
use crate::placement::PlacementError;
use crate::placement::persist::load_restored_state;
use crate::placement::source::{
    DescriptSource, load_balloon_author_dpi, load_descript_source_for_shell,
};

/// 背景の資産づくりの依頼（受理のとき 1 回）。
pub(crate) enum SwitchBuildRequest {
    /// シェル: ゴーストの根と、`shell/` の下のフォルダ名（目録に在る名前だけを渡す＝
    /// `resolve_with_shell` は名前を検査しない）。
    Shell { ghost_root: PathBuf, folder: String },
    /// バルーン: バルーンのフォルダ。
    Balloon { dir: PathBuf },
}

impl SwitchBuildRequest {
    /// 記録用の種別の語。
    fn kind(&self) -> &'static str {
        match self {
            Self::Shell { .. } => "shell",
            Self::Balloon { .. } => "balloon",
        }
    }

    /// 記録用の依頼の先。
    fn target(&self) -> String {
        match self {
            Self::Shell { ghost_root, folder } => {
                ghost_root.join("shell").join(folder).display().to_string()
            }
            Self::Balloon { dir } => dir.display().to_string(),
        }
    }
}

/// 背景の資産づくりの結果（線で UI へ返す）。
#[allow(dead_code)] // 欄を読むのは差し替えの相（9.3）
pub(crate) enum SwapBuilt {
    /// シェル: scope ごとの `EmoWorld`・アトラス・作者の DPI（新しいシェルの `seriko.dpi`）と
    /// 別名表・静的な着せ替え・着せ替えの名前表・アニメ表（[`ShellAssets`]）、配置の値
    /// （新しいシェルの `descript.txt`）、位置の記憶（読むだけ）。
    Shell {
        assets: ShellAssets,
        source: DescriptSource,
        restored: Vec<(PersistKey, String)>,
    },
    /// バルーン: scope ごとの `EmoWorld`・アトラス・文字の模型・背景色と、装着の全 scope ぶんの
    /// アニメ表、作者の DPI（新しいバルーンの `dpi`）（[`BalloonAssets`]）。
    Balloon { assets: BalloonAssets },
}

/// 背景の資産づくりの失敗（`BootWiringError`／`PlacementError` の写し）。
#[derive(Debug, thiserror::Error)]
pub(crate) enum SwitchBuildError {
    /// 資産づくり（読めない・解釈できない・復号できない）の失敗。
    #[error(transparent)]
    Assets(#[from] BootWiringError),
    /// 配置の値（シェルの `descript.txt`）の読みの失敗。
    #[error(transparent)]
    Placement(#[from] PlacementError),
    /// 新しいシェルの絵が復号できない（起動は読み飛ばして続けるが、切替では元の表示のまま
    /// 失敗として返す・要件 5.5・5.6。`BootWiringError` には足さない）。
    #[error("シェルの絵が復号できない: {shell_dir} {failures:?}")]
    ShellUndecodable {
        /// 新しいシェルのフォルダ。
        shell_dir: PathBuf,
        /// 焼く段で落ちた絵の理由（`ShellAssets::bake_failures`）。
        failures: Vec<String>,
    },
}

/// 差し替えの荷物（UI が置き場へ置き、表示の橋渡しが合図の世代と突き合わせて取り出す）。
#[allow(dead_code)] // 作るのは差し替えの相（9.3）・取り出すのは表示の橋渡し（9.1）
pub(crate) struct SwapPayload {
    /// 差し替えの世代（合図 `Rebased` の `epoch` と結ぶ）。
    pub epoch: u64,
    /// scope ごとの新しい `EmoWorld`・アトラス・作者の DPI。
    pub targets: Vec<(u32, EmoWorld, AtlasTable, u16)>,
    /// scope ごとの置き換えの返信の送り手。
    pub replies: Vec<(u32, ReplySender<PresentOutcome>)>,
}

/// 荷物を 1 つ置く共有の置き場（鍵を持つのは置くときと取り出すときの一瞬だけ）。
#[allow(dead_code)] // 結線（9.1）が作る
pub(crate) type SwapSlot = Arc<Mutex<Option<SwapPayload>>>;

/// 受理ごとに背景のスレッドを 1 本起こして資産を作り、結果を 1 件だけ送る線の受け手を返す。
///
/// 受け手が切断を見たら、スレッドが倒れた（送り手が落ちた）ことを意味する。
pub(crate) fn spawn_switch_build(
    request: SwitchBuildRequest,
) -> Receiver<Result<SwapBuilt, SwitchBuildError>> {
    spawn_build_worker(move || build_swap(&request))
}

/// `build` を COM（MTA）を初期化した新しいスレッドで走らせ、結果を線で返す。
///
/// テストが倒れるスレッドを注入できるように、本体を引数で受ける。
fn spawn_build_worker<F>(build: F) -> Receiver<Result<SwapBuilt, SwitchBuildError>>
where
    F: FnOnce() -> Result<SwapBuilt, SwitchBuildError> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("areka-switch-assets".to_string())
        .spawn(move || {
            // SAFETY: このスレッドで最初の COM 初期化（MTA・WIC のデコーダの前提）。失敗すれば
            // デコーダが作れず、資産づくりの失敗として記録と `Err` が線で返る。
            let com = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let result = build();
            if tx.send(result).is_err() {
                debug!(
                    event = "switch_assets_discarded",
                    "switch_assets: 受け手が先に消えたので作った資産を捨てる（切替の取りやめ）"
                );
            }
            if com.is_ok() {
                // SAFETY: 上の初期化が成功したときだけ対にして解く（デコーダは `build` の中で落ち済み）。
                unsafe { CoUninitialize() };
            }
        });
    if let Err(err) = spawned {
        // 送り手はクロージャと一緒に落ちているので、受け手は切断を見る。
        error!(
            event = "switch_assets_failed",
            reason = "spawn",
            error = %err,
            "switch_assets: 資産づくりのスレッドを起こせなかった"
        );
    }
    rx
}

/// 背景のスレッドが走らせる本体（COM 初期化済みのスレッドで呼ぶ）。
///
/// 失敗は `error!`（`event = "switch_assets_failed"`）を 1 件残して返す。
pub(crate) fn build_swap(request: &SwitchBuildRequest) -> Result<SwapBuilt, SwitchBuildError> {
    let started = Instant::now();
    // scope の集合は起動の結線と同じ導出（バルーンのアニメ表は装着の全 scope ぶん作る）。
    let scopes = super::derive_scopes();
    let result = match request {
        SwitchBuildRequest::Shell { ghost_root, folder } => {
            build_shell(ghost_root, folder, &scopes)
        }
        SwitchBuildRequest::Balloon { dir } => build_balloon(dir, &scopes),
    };
    match &result {
        Ok(_) => info!(
            event = "switch_assets_built",
            kind = request.kind(),
            target = %request.target(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "switch_assets: 差し替えの資産を作った"
        ),
        Err(err) => error!(
            event = "switch_assets_failed",
            kind = request.kind(),
            target = %request.target(),
            error = %err,
            "switch_assets: 差し替えの資産づくりに失敗（元の表示のまま）"
        ),
    }
    result
}

/// シェル: 配置の値 → 資産（作者の DPI は新しいシェルの `seriko.dpi`）→ 位置の記憶（読むだけ）。
fn build_shell(
    ghost_root: &Path,
    folder: &str,
    scopes: &[u32],
) -> Result<SwapBuilt, SwitchBuildError> {
    let source = load_descript_source_for_shell(ghost_root, Some(folder))?;
    let decoder = WicDecoderArm::new().map_err(BootWiringError::Decoder)?;
    let assets = build_shell_assets(
        ghost_root,
        Some(folder),
        scopes,
        source.shell_author_dpi(),
        &decoder,
    )?;
    // 起動は落ちた絵を読み飛ばして続けるが、切替では空の窓へ差し替えず元の表示のまま失敗にする。
    if !assets.bake_failures.is_empty() {
        return Err(SwitchBuildError::ShellUndecodable {
            shell_dir: source.shell_dir.clone(),
            failures: assets.bake_failures,
        });
    }
    let restored = load_restored_state(ghost_root, DefaultEncoding::Ansi);
    Ok(SwapBuilt::Shell {
        assets,
        source,
        restored,
    })
}

/// バルーン: 資産（作者の DPI は新しいバルーンの `dpi`）。
fn build_balloon(dir: &Path, scopes: &[u32]) -> Result<SwapBuilt, SwitchBuildError> {
    let decoder = WicDecoderArm::new().map_err(BootWiringError::Decoder)?;
    let assets = build_balloon_assets(dir, scopes, load_balloon_author_dpi(dir), &decoder)?;
    Ok(SwapBuilt::Balloon { assets })
}

#[cfg(test)]
#[path = "switch_assets_tests.rs"]
mod tests;
