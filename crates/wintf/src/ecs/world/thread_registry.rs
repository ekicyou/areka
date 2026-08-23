//! スレッド名簿——役割名つきでスレッドを登記し、後から一覧で読み出す。
//!
//! # 何のためか
//!
//! 「どこが重いか」をスレッド単位で突き止めるには、CPU 時間を**役割名**（vblank 検出・
//! カーソル監視・UI・ティッカー・アクター…）に結び付けて読めなければならない。OS から
//! スレッドを列挙する道（ToolHelp）は使えない——その feature は `Cargo.toml` に無く、
//! 追加しない制約がある（要件 8.6）。そこで**列挙の代わりに登録**を採る。
//!
//! # 誰が登録するか
//!
//! **生成されたスレッド自身**が、走り始めに 1 度 [`register_current_thread`] を呼ぶ。
//! 役割名は呼び出し側の**宣言**であって、名前やスタックからの推測ではない。登録時に
//! 自分のハンドルを複製して名簿へ預けるので、そのスレッドが終了した後も CPU 時間を
//! 読み出せる。
//!
//! # 名簿外の扱い（`unregistered_rest`）
//!
//! 名簿に載らないスレッド（タスクプール等）は必ず残る。その分は「プロセス全体の CPU −
//! 名簿の合計」の差として 1 行で報告する約束であり、**差の計算は報告器の仕事**で本
//! モジュールは行わない。ここが持つのは語彙の定数 [`ROLE_UNREGISTERED_REST`] だけである。
//!
//! # 費用
//!
//! 登録はスレッド 1 本につき 1 度きり（ハンドル複製 1 回と `Vec` への push 1 回）。
//! 読み出し [`snapshot`] は報告器が周期的に呼ぶだけで、tick の経路には一切入らない。

use std::os::windows::io::OwnedHandle;
use std::sync::{Arc, Mutex};

use tracing::{error, warn};
use windows::core::Result;

pub use crate::api::{CpuTimes, get_current_thread_id, get_process_times};

/// vblank 検出スレッド（`wintf-vsync`）。
pub const ROLE_VBLANK: &str = "vblank";
/// カーソル監視スレッド（`wintf-cursor-monitor`）。
pub const ROLE_CURSOR_MONITOR: &str = "cursor_monitor";
/// UI スレッド（メッセージループを回す本体）。
pub const ROLE_UI: &str = "ui";
/// ティッカーのうち dispatcher／kanade 系統。
pub const ROLE_TICKER_DISPATCHER_KANADE: &str = "ticker_dispatcher_kanade";
/// ティッカーのうちループ系統（SERIKO ループ）。
pub const ROLE_TICKER_LOOP: &str = "ticker_loop";
/// アクターの役割名に付く接頭辞。実際の役割名は [`role_actor`] で組み立てる
/// （`actor:<name>`）——接尾辞の無い `actor:` 単体は役割名として成立しない。
pub const ROLE_ACTOR_PREFIX: &str = "actor:";
/// スレッド別 CPU の報告器自身。
pub const ROLE_PERF_REPORT: &str = "perf_report";
/// 名簿に載っていない残り（報告器がプロセス CPU との差として 1 行で出す）。
pub const ROLE_UNREGISTERED_REST: &str = "unregistered_rest";

/// 固定語彙の全件（アクターは接頭辞 [`ROLE_ACTOR_PREFIX`] の形で 1 件として並ぶ）。
///
/// 報告行に出る役割名はここに在るものだけである。語彙を増やすときは本配列を直し、
/// 兄弟のテストで綴りと並びを固定し直す。
pub const ALL_ROLES: &[&str] = &[
    ROLE_VBLANK,
    ROLE_CURSOR_MONITOR,
    ROLE_UI,
    ROLE_TICKER_DISPATCHER_KANADE,
    ROLE_TICKER_LOOP,
    ROLE_ACTOR_PREFIX,
    ROLE_PERF_REPORT,
    ROLE_UNREGISTERED_REST,
];

/// 名簿の 1 項目（役割名・OS 名・TID・複製ハンドル）。
///
/// ハンドルは [`Arc`] で共有する——一覧を取り出すたびに `DuplicateHandle` を呼ぶと
/// 失敗しうる経路が増えるため、複製は登録時の 1 回だけにして、以後は参照を配る。
#[derive(Clone, Debug)]
pub struct ThreadEntry {
    /// 登録時に宣言された役割名（固定語彙、またはアクターの `actor:<name>`）。
    pub role: String,
    /// OS 側のスレッド名（付いていなければ `None`）。
    pub name: Option<String>,
    /// スレッド ID。
    pub tid: u32,
    /// 登録時に複製した所有権つきハンドル（スレッド終了後も CPU 時間を読める）。
    pub handle: Arc<OwnedHandle>,
}

impl ThreadEntry {
    /// この項目のスレッドが消費した CPU 時間を読む（`GetThreadTimes`）。
    ///
    /// 失敗は握り潰さず `Err` で返す（記録は呼び出し側＝報告器が行う）。
    pub fn cpu_times(&self) -> Result<CpuTimes> {
        crate::api::get_thread_times(&self.handle)
    }
}

/// プロセス共有の名簿。
///
/// 触るのは登録（スレッド 1 本につき 1 度）と一覧（報告器の周期）だけで、
/// フレーム駆動の経路からは呼ばれない。
static REGISTRY: Mutex<Vec<ThreadEntry>> = Mutex::new(Vec::new());

/// 名簿の錠を取って中身を触る。
///
/// 別スレッドの panic で錠が毒化していても名簿は捨てない——観測の道具が観測対象の
/// 事故で黙って消えるのを避けるため、中身をそのまま受け取って続ける。
fn with_registry<R>(f: impl FnOnce(&mut Vec<ThreadEntry>) -> R) -> R {
    let mut guard = REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

/// アクターの役割名を組み立てる（`actor:<name>`）。
pub fn role_actor(name: impl AsRef<str>) -> String {
    format!("{ROLE_ACTOR_PREFIX}{}", name.as_ref())
}

/// 固定語彙として認められる役割名かを返す。
///
/// アクターだけは接尾辞つき（`actor:<name>`）で、接尾辞が空のものは認めない。
pub fn is_known_role(role: &str) -> bool {
    match role.strip_prefix(ROLE_ACTOR_PREFIX) {
        Some(actor_name) => !actor_name.is_empty(),
        None => ALL_ROLES.contains(&role),
    }
}

/// 呼び出しスレッド自身を、宣言した役割名で名簿へ登録する。
///
/// 呼ぶのは**登録される当のスレッド**である（他スレッドを代理登録する口は持たない
/// ——擬似ハンドルの複製が「今走っているスレッド」を捕まえる性質に依るため）。
/// 同じ TID の項目が既に在れば置き換える（再登録・および OS が TID を再利用した場合に
/// 二重計上を残さないため）。
///
/// 固定語彙に無い役割名は `warn!` を残したうえで登録する——観測対象を黙って落とすより、
/// 語彙の綻びが見える形で残るほうが良い。ハンドル複製の失敗は `error!` を残して `Err`
/// を返す（panic しない）。
pub fn register_current_thread(role: impl Into<String>) -> Result<()> {
    let role = role.into();
    if !is_known_role(&role) {
        warn!(
            target: "wintf::thread_registry",
            role = %role,
            "固定語彙に無い役割名で登録された（報告行には出るが語彙の検査には掛からない）"
        );
    }

    let handle = match crate::api::duplicate_current_thread_handle() {
        Ok(handle) => handle,
        Err(err) => {
            error!(
                target: "wintf::thread_registry",
                role = %role,
                error = %err,
                "スレッドハンドルの複製に失敗した——このスレッドは名簿に載らず、CPU は unregistered_rest へ回る"
            );
            return Err(err);
        }
    };

    let tid = get_current_thread_id();
    let name = std::thread::current().name().map(str::to_owned);
    let entry = ThreadEntry {
        role,
        name,
        tid,
        handle: Arc::new(handle),
    };

    with_registry(|entries| {
        entries.retain(|existing| existing.tid != tid);
        entries.push(entry);
    });
    Ok(())
}

/// 名簿の一覧（スナップショット）を返す。
///
/// 返るのは複製ハンドルを共有する複写であり、以後の登録は反映されない。錠は返る前に
/// 手放すので、呼び出し側は一覧を舐めながら `GetThreadTimes` を呼んでよい。
pub fn snapshot() -> Vec<ThreadEntry> {
    with_registry(|entries| entries.clone())
}

#[cfg(test)]
#[path = "thread_registry_tests.rs"]
mod tests;
