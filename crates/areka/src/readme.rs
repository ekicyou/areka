//! ゴーストの説明書（areka-P0-popup-menu-minimal）。
//!
//! 説明書を開く要求、開くファイルの決め方（ゴースト descript の `readme` キー）、
//! World への結線と要求の取り出しを置く。開く系の要求（areka-P0-open-external-tags）も
//! 同じ取り出しを通り、説明書も含めて開く処理の 1 か所（[`opener::submit`]）へ渡す。
//! メニューにも入力配線にも依存しない葉の module である。
//!
//! 開くファイルは決めて渡すだけで、中身も `readme.charset` も読まない（要件 4.7）。

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use bevy_ecs::schedule::{IntoScheduleConfigs, Schedules};
use bevy_ecs::world::World;
use wintf::ecs::Input;
use wintf::ecs::pointer::dispatch_pointer_events;

/// 台本の受け口から届く要求（areka-P0-open-external-tags 要件 7.1）。
pub(crate) enum ReadmeRequest {
    /// 引数なしの `\![open,readme]`（要件 4.5）。
    ///
    /// 中身を持たないのは、開くファイルが要求ごとではなく起動時に決まるからである
    /// （引数付きの `\![open,readme,種類,名前]` は送出側が警告して捨てる・要件 4.6）。
    Readme,
    /// 開く系の行き先（受け口が規則で分類済み）。
    // 受け口が送るのは 4.2 から＝それまで本番のビルドに作り手が無い（4.2 で外す）。
    #[cfg_attr(not(test), allow(dead_code))]
    Open(destination::Destination),
}

/// 台本の説明書の要求の綴り（記録の `tag`）。
const SCRIPT_README_TAG: &str = "\\![open,readme]";
/// メニューの「説明書」の印（記録の `tag`）。
pub(crate) const MENU_TAG: &str = "menu:readme";

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

/// 説明書の持ち物を World へ入れる（取り出しの登録は [`register_readme_drain`]）。
///
/// 呼び手は boot 成功後の `wire_emo2_boot`（ゴーストの根と `readme` キーを読める場所・task 4.3）。
pub(crate) fn wire_readme(world: &mut World, path: PathBuf, rx: Receiver<ReadmeRequest>) {
    world.insert_non_send(ReadmeWiring {
        path,
        rx,
        missing_logged: Cell::new(false),
    });
}

/// 要求の取り出しを入力の段へ登録し、開く専用のスレッドの持ち物（[`opener::Opener`]）が
/// 無ければ 1 度だけ起こして入れる（説明書の持ち物は置かない）。
///
/// 起こせなければ `error!` を残し、以後は入口（[`opener::submit`]）が要求ごとに `error!` を残す。
///
/// 並びは `dispatch_pointer_events` の後——同じ tick のうちに届いた要求をその tick で
/// 処理する（`input_events/choice_drain.rs` の `register_choice_drain` と同型）。
///
/// 呼び手は `ghost_session::register_systems`（プロセスに 1 回）。
pub(crate) fn register_readme_drain(world: &mut World) {
    world
        .resource_mut::<Schedules>()
        .add_systems(Input, drain_readme_requests.after(dispatch_pointer_events));
    if world.contains_resource::<opener::Opener>() {
        return;
    }
    match opener::Opener::spawn() {
        Ok(opener) => world.insert_resource(opener),
        Err(error) => tracing::error!(
            event = "open_external_spawn_failed",
            %error,
            "[readme] could not start the open-external thread: open requests will be dropped"
        ),
    }
}

impl ReadmeWiring {
    /// 起動時に決めた説明書のファイル（テスト専用の読み口）。
    #[cfg(test)]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
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

/// 決めたファイルを開く処理の 1 か所へ渡す（要件 4.2・areka-P0-open-external-tags 要件 7.1）。
///
/// メニューの「説明書」（`tag`＝[`MENU_TAG`]）と台本の `\![open,readme]`（台本の綴り）の両方が
/// ここへ届く。OS は直接呼ばず、絶対パスにした行き先を [`opener::submit`] へ渡す。開けなかった
/// 失敗は開く専用のスレッドが記録するので、ゴーストの動作はそのまま続ける（要件 4.4）。
///
/// 説明書のファイルが無ければ渡さず、要求 1 件につき 1 行を `warn!` で記録して戻る
/// （要件 5.1）。[`is_available`] は呼ばない——メニュー側の初回だけの `debug!` を奪うため（要件 5.4）。
pub(crate) fn open_from_world(world: &World, tag: &str) {
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
    // ghost/master 基準の解決に吸われないよう絶対パスにして渡す。
    let path = std::path::absolute(&wiring.path).unwrap_or_else(|_| wiring.path.clone());
    let written = path.to_string_lossy().into_owned();
    opener::submit(
        world,
        destination::Destination {
            target: destination::Target::Path(written.clone()),
            written,
            tag: tag.to_owned(),
        },
    );
}

/// 溜まっている要求を届いた順に全件取り出す（開く処理は呼び手が行う）。
///
/// 受け口を空にするのを 1 か所に閉じ、「全件取り出して 1 件も残さない」を OS に触れずに
/// 確かめられるようにする（渡す処理そのものは [`drain_readme_requests`] 側）。
fn take_pending(world: &World) -> Vec<ReadmeRequest> {
    let Some(wiring) = world.get_non_send::<ReadmeWiring>() else {
        tracing::trace!(
            event = "readme_drain_no_wiring",
            "[readme] ReadmeWiring absent: the drain degrades to a no-op"
        );
        return Vec::new();
    };
    wiring.rx.try_iter().collect()
}

/// 溜まった要求を届いた順に全件取り出し、開く処理へ渡す（Input スケジュールの system）。
///
/// 説明書は台本の綴りで [`open_from_world`] へ、開く系は入口 [`opener::submit`] へ。
pub(crate) fn drain_readme_requests(world: &mut World) {
    for request in take_pending(world) {
        match request {
            ReadmeRequest::Readme => open_from_world(world, SCRIPT_README_TAG),
            ReadmeRequest::Open(dest) => opener::submit(world, dest),
        }
    }
}

// OS の境界（areka-P0-open-external-tags task 1.2）。
mod os_port;

// 行き先の規則（task 2.1）。受け口の結線（4.2）まで本番のビルドに呼び手が無い（4.2 で外す）。
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod destination;

// 開く処理の 1 か所（task 3.1〜3.4）。
mod opener;

#[cfg(test)]
mod opener_test_support;

#[cfg(test)]
#[path = "readme_tests.rs"]
mod readme_tests;
