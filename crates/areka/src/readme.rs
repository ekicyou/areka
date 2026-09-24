//! ゴーストの説明書（areka-P0-popup-menu-minimal）。
//!
//! 説明書を開く要求、開くファイルの決め方（ゴースト descript の `readme` キー）、
//! 既定のアプリで開く関数、World への結線と要求の取り出しを置く。
//! メニューにも入力配線にも依存しない葉の module である。
//!
//! 開くファイルは決めて渡すだけで、中身も `readme.charset` も読まない（要件 4.7）。

use std::cell::Cell;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

/// 台本（`\![open,readme]`）から届く「説明書を開いて」の要求（引数なし・要件 4.5）。
///
/// 中身を持たないのは、開くファイルが要求ごとではなく起動時に決まるからである
/// （引数付きの `\![open,readme,種類,名前]` は送出側が警告して捨てる・要件 4.6）。
pub(crate) struct ReadmeRequest;

/// `readme` キーが無いときの正典の既定名（要件 4.1 ⑵）。
///
/// 正典の URL は `readme` キーの定義箇所（`areka_parsers` の `MountModel.readme` の欄）に置いてある。
pub(crate) const DEFAULT_README: &str = "readme.txt";

/// 説明書のファイルを決める（要件 4.1）。
///
/// ゴースト定義の `readme` キーの値、無ければ正典の既定名を、ゴーストのフォルダの根
/// （起動引数で渡した根）の直下で解決する。実在するかはここでは見ない（[`is_available`]）。
pub(crate) fn resolve_path(ghost_root: &Path, key: Option<&str>) -> PathBuf {
    ghost_root.join(key.unwrap_or(DEFAULT_README))
}

/// 説明書の持ち物（NonSend・UI スレッド所有）。
///
/// `missing_logged` を [`Cell`] にしているのは、メニューの供給関数が `&World` しか持たない
/// まま [`is_available`] を呼ぶためである（共有借用のまま初回の記録を覚える）。
pub(crate) struct ReadmeWiring {
    /// 起動時に決めた説明書のファイル（[`resolve_path`] の結果）。
    path: PathBuf,
    /// 台本からの要求の受信端（送出端は `ReadmeCueSink` が持つ）。
    rx: Receiver<ReadmeRequest>,
    /// 「ファイルが無い」を 1 度でも記録したか（要件 4.3 の初回だけ）。
    missing_logged: Cell<bool>,
}

/// 説明書の持ち物を World へ入れ、要求の取り出しを毎 tick の入力の段へ登録する。
///
/// 並びは `dispatch_pointer_events` の後——同じ tick のうちに届いた要求をその tick で
/// 処理する（`input_events/choice_drain.rs` の `wire_choice_drain` と同型）。
///
/// 呼び手は boot 成功後の `wire_emo2_boot`（ゴーストの根と `readme` キーを読める場所・task 4.3）。
pub(crate) fn wire_readme(world: &mut World, path: PathBuf, rx: Receiver<ReadmeRequest>) {
    world.insert_non_send(ReadmeWiring {
        path,
        rx,
        missing_logged: Cell::new(false),
    });
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain_readme_requests.after(dispatch_pointer_events));
}

/// 説明書のファイルがあるか（要件 4.3・11.4）。
///
/// 無いときは「説明書」を灰色で出すための `false` を返し、**初めて無いと分かったときだけ**
/// `debug!` で記録する（毎 tick の照会で記録が溢れない）。持ち物が無いときも `false`。
pub(crate) fn is_available(world: &World) -> bool {
    let Some(wiring) = world.get_non_send::<ReadmeWiring>() else {
        tracing::trace!(
            event = "readme_available_no_wiring",
            "[readme] ReadmeWiring absent: the readme item stays disabled"
        );
        return false;
    };
    if wiring.path.exists() {
        return true;
    }
    // 直前の値が false のときだけ記録する（`replace` は古い値を返す）。
    if !wiring.missing_logged.replace(true) {
        tracing::debug!(
            event = "readme_missing",
            path = %wiring.path.display(),
            "[readme] readme file not found: the menu item stays disabled"
        );
    }
    false
}

/// 決めたファイルを既定のアプリで開く（要件 4.2）。
///
/// メニューの「説明書」と台本の `\![open,readme]` の両方がここへ届く。開けなかった失敗は
/// [`open`] が記録済みなので、ゴーストの動作はそのまま続ける（要件 4.4）。
///
/// 説明書のファイルが無ければ OS を呼ばず、要求 1 件につき 1 行を `warn!` で記録して戻る
/// （要件 5.1）。[`is_available`] は呼ばない——メニュー側の初回だけの `debug!` を奪うため（要件 5.4）。
pub(crate) fn open_from_world(world: &World) {
    let Some(wiring) = world.get_non_send::<ReadmeWiring>() else {
        tracing::warn!(
            event = "readme_open_no_wiring",
            "[readme] ReadmeWiring absent: nothing to open"
        );
        return;
    };
    if !wiring.path.exists() {
        tracing::warn!(
            event = "readme_open_skipped_missing",
            path = %wiring.path.display(),
            "[readme] readme file not found: nothing to open, the ghost keeps running"
        );
        return;
    }
    let _ = open(&wiring.path);
}

/// 溜まっている要求を全件取り出し、その件数を返す（開く処理は呼び手が行う）。
///
/// 受け口を空にするのと件数を数えるのを 1 か所に閉じ、「全件取り出して 1 件も残さない」を
/// OS に触れずに確かめられるようにする（開く処理そのものは [`drain_readme_requests`] 側）。
fn take_pending(world: &World) -> usize {
    let Some(wiring) = world.get_non_send::<ReadmeWiring>() else {
        tracing::trace!(
            event = "readme_drain_no_wiring",
            "[readme] ReadmeWiring absent: the drain degrades to a no-op"
        );
        return 0;
    };
    wiring.rx.try_iter().count()
}

/// 溜まった要求を全件取り出し、1 件ごとに説明書を開く（Input スケジュールの system）。
pub(crate) fn drain_readme_requests(world: &mut World) {
    for _ in 0..take_pending(world) {
        open_from_world(world);
    }
}

/// 既定のアプリでファイルを開く（要件 4.2・4.4）。
///
/// `ShellExecuteW` の戻り値は成功なら 32 より大きく、32 以下は失敗を表す符号である
/// （`SE_ERR_*`）。失敗はパスと符号を添えて `error!` で記録し、呼び手はゴーストの動作を
/// 続ける。この module の `unsafe` はこの関数だけに閉じる。
fn open(path: &Path) -> windows::core::Result<()> {
    // 終端 0 付きの UTF-16 へ写す（呼び出しの間だけ生きていればよい）。
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    // SAFETY: `wide` は終端 0 付きで呼び出しの間ずっと生存し、他の引数は定数と None。
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as usize;
    if code <= 32 {
        tracing::error!(
            event = "readme_open_failed",
            path = %path.display(),
            code,
            "[readme] ShellExecuteW failed: the ghost keeps running"
        );
        return Err(windows::core::Error::from_hresult(
            WIN32_ERROR(code as u32).to_hresult(),
        ));
    }
    tracing::info!(
        event = "readme_opened",
        path = %path.display(),
        "[readme] opened the readme with the default application"
    );
    Ok(())
}

#[cfg(test)]
#[path = "readme_tests.rs"]
mod readme_tests;
