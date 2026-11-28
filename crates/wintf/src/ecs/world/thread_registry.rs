//! スレッド名簿——役割名つきでスレッドを登記し、後から一覧で読み出す。
//!
//! # 何のためか
//!
//! 「どこが重いか」をスレッド単位で突き止めるには、CPU 時間を**役割名**（vblank 検出・
//! カーソル監視・UI・ティッカー・アクター…）に結び付けて読めなければならない。OS から
//! スレッドを列挙する道（ToolHelp）は使えない——その feature は `Cargo.toml` に無く、
//! 追加しない制約がある。そこで**列挙の代わりに登録**を採る。
//!
//! # 誰が登録するか
//!
//! **生成されたスレッド自身**が、走り始めに 1 度 [`register_current_thread`] を呼ぶ。
//! 役割名は呼び出し側の**宣言**であって、名前やスタックからの推測ではない。登録時に
//! 自分のハンドルを複製して名簿へ預ける。
//!
//! # 終わったスレッド（系統の畳んだ合計）
//!
//! 登録と読み出しのたびに、預かったハンドルで各項目のスレッドが終わったかを見分ける
//! （掃除）。終わったスレッドは最後の CPU 時間を**役割の系統**ごとの合計へ足し、項目と
//! ハンドルを手放す。系統の名は役割名の末尾の `-<数字>` を `-*` に畳んだもの
//! （`actor:sakura-talk-17` → `actor:sakura-talk-*`）で、番号の無い役割名はそのまま系統の
//! 名になる。合計と畳んだ本数は減らない。これで項目の数は「生きている登録済みスレッドの
//! 数」で頭打ちになり、トークのたびに起きるスレッドの分だけ積み上がらない。最後の CPU を
//! 読めなかったスレッドは本数にだけ入り、その CPU は `unregistered_rest` へ回る（`warn!`）。
//!
//! # 名簿外の扱い（`unregistered_rest`）
//!
//! 名簿に載らないスレッド（タスクプール等）は必ず残る。その分は「プロセス全体の CPU −
//! 生きた項目と畳んだ合計」の差として 1 行で報告する約束であり、**差の計算は報告器の
//! 仕事**で本モジュールは行わない。ここが持つのは語彙の定数 [`ROLE_UNREGISTERED_REST`]
//! だけである。
//!
//! # 費用
//!
//! 登録はスレッド 1 本につき 1 度きり（ハンドル複製 1 回と掃除 1 回）。掃除は項目ごとに
//! 待たない `WaitForSingleObject` 1 回と、終わった項目ごとの `GetThreadTimes` 1 回。
//! 読み出し [`snapshot`]・[`read_cpu`] は報告器が周期的に呼ぶだけで、tick の経路には
//! 一切入らない。

use std::collections::BTreeMap;
use std::os::windows::io::OwnedHandle;
use std::sync::{Arc, Mutex};

use tracing::{debug, error, warn};
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
/// 報告行に出る役割名は、ここに在るもの（アクターは `actor:<name>`）と、その系統の名
/// （末尾の `-<数字>` を `-*` に畳んだもの）だけである。語彙を増やすときは本配列を直し、
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
    /// 登録時に複製した所有権つきハンドル（名簿が手放すまで、終了後も CPU 時間を読める）。
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

/// 生きた項目の写し（読み出しの時点の CPU 時間つき）。
#[derive(Clone, Debug)]
pub struct LiveReading {
    /// 登録時に宣言された役割名。
    pub role: String,
    /// OS 側のスレッド名（付いていなければ `None`）。
    pub name: Option<String>,
    /// スレッド ID。
    pub tid: u32,
    /// 読み出しの時点の CPU 時間（読めなければ `Err`）。
    pub cpu: Result<CpuTimes>,
}

/// 役割の系統ごとの畳んだ合計の写し。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldedReading {
    /// 系統の名（例 `actor:sakura-talk-*`）。
    pub family: String,
    /// 畳んだスレッドの最後の CPU 時間の和（累積）。
    pub cpu: CpuTimes,
    /// 畳んだスレッドの本数（累積・最後の CPU を読めなかった分も入る）。
    pub exited: u64,
}

/// 名簿の 1 つの時点の読み出し（生きた項目の CPU と系統の畳んだ合計）。
#[derive(Clone, Debug)]
pub struct RegistryReading {
    /// 生きた項目（登録順）。
    pub live: Vec<LiveReading>,
    /// 系統の畳んだ合計（系統名の昇順）。
    pub folded: Vec<FoldedReading>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct FoldedTotal {
    kernel_100ns: u64,
    user_100ns: u64,
    exited: u64,
}

trait ThreadProbe {
    fn has_exited(&self, entry: &ThreadEntry) -> Result<bool>;
    fn cpu_times(&self, entry: &ThreadEntry) -> Result<CpuTimes>;
}

struct Win32Probe;

impl ThreadProbe for Win32Probe {
    fn has_exited(&self, entry: &ThreadEntry) -> Result<bool> {
        crate::api::thread_has_exited(&entry.handle)
    }

    fn cpu_times(&self, entry: &ThreadEntry) -> Result<CpuTimes> {
        entry.cpu_times()
    }
}

/// 掃除で見つけた失敗。錠を放してから記録へ出す。
#[derive(Debug)]
enum SweepNote {
    ExitUnknown {
        tid: u32,
        role: String,
        error: windows::core::Error,
    },
    FinalCpuUnreadable {
        tid: u32,
        role: String,
        family: String,
        error: windows::core::Error,
    },
}

impl SweepNote {
    fn emit(self) {
        match self {
            SweepNote::ExitUnknown { tid, role, error } => warn!(
                target: "wintf::thread_registry",
                tid,
                role = %role,
                error = %error,
                "スレッドの終わりを見分けられなかった——項目は名簿に残す"
            ),
            SweepNote::FinalCpuUnreadable {
                tid,
                role,
                family,
                error,
            } => warn!(
                target: "wintf::thread_registry",
                tid,
                role = %role,
                family = %family,
                error = %error,
                "終わったスレッドの最後の CPU 時間を読めなかった——その分は unregistered_rest へ回る"
            ),
        }
    }
}

fn emit_notes(notes: Vec<SweepNote>) {
    notes.into_iter().for_each(SweepNote::emit);
}

#[derive(Debug, Default)]
struct Registry {
    live: Vec<ThreadEntry>,
    folded: BTreeMap<String, FoldedTotal>,
}

impl Registry {
    const fn new() -> Self {
        Self {
            live: Vec::new(),
            folded: BTreeMap::new(),
        }
    }

    fn sweep(&mut self, probe: &impl ThreadProbe) -> Vec<SweepNote> {
        let mut notes = Vec::new();
        let folded = &mut self.folded;
        self.live.retain(|entry| match probe.has_exited(entry) {
            Ok(false) => true,
            Err(error) => {
                notes.push(SweepNote::ExitUnknown {
                    tid: entry.tid,
                    role: entry.role.clone(),
                    error,
                });
                true
            }
            Ok(true) => {
                let family = role_family(&entry.role);
                let cpu = probe.cpu_times(entry);
                let total = folded.entry(family.clone()).or_default();
                total.exited += 1;
                match cpu {
                    Ok(cpu) => {
                        total.kernel_100ns += cpu.kernel_100ns;
                        total.user_100ns += cpu.user_100ns;
                    }
                    Err(error) => notes.push(SweepNote::FinalCpuUnreadable {
                        tid: entry.tid,
                        role: entry.role.clone(),
                        family,
                        error,
                    }),
                }
                false
            }
        });
        notes
    }

    /// 掃除の後に同じ TID が残っていれば、生きている同じスレッドの登録し直しなので畳まずに外す。
    fn insert(&mut self, entry: ThreadEntry, probe: &impl ThreadProbe) -> Vec<SweepNote> {
        let notes = self.sweep(probe);
        self.live.retain(|existing| existing.tid != entry.tid);
        self.live.push(entry);
        notes
    }

    fn read(&mut self, probe: &impl ThreadProbe) -> (RegistryReading, Vec<SweepNote>) {
        let notes = self.sweep(probe);
        let live = self
            .live
            .iter()
            .map(|entry| LiveReading {
                role: entry.role.clone(),
                name: entry.name.clone(),
                tid: entry.tid,
                cpu: probe.cpu_times(entry),
            })
            .collect();
        let folded = self
            .folded
            .iter()
            .map(|(family, total)| FoldedReading {
                family: family.clone(),
                cpu: CpuTimes {
                    kernel_100ns: total.kernel_100ns,
                    user_100ns: total.user_100ns,
                },
                exited: total.exited,
            })
            .collect();
        (RegistryReading { live, folded }, notes)
    }
}

/// プロセス共有の名簿。
///
/// 触るのは登録（スレッド 1 本につき 1 度）と一覧（報告器の周期）だけで、
/// フレーム駆動の経路からは呼ばれない。
static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());

/// 名簿の錠を取って中身を触る。
///
/// 別スレッドの panic で錠が毒化していても名簿は捨てない——観測の道具が観測対象の
/// 事故で黙って消えるのを避けるため、中身をそのまま受け取って続ける。
fn with_registry<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    let mut guard = REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

fn role_family(role: &str) -> String {
    match role.rsplit_once('-') {
        Some((stem, digits))
            if !stem.is_empty()
                && !digits.is_empty()
                && digits.bytes().all(|b| b.is_ascii_digit()) =>
        {
            format!("{stem}-*")
        }
        _ => role.to_owned(),
    }
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
/// 登録の前に終わったスレッドを系統の合計へ畳んで外す。その後も同じ TID の項目が在れば
/// 生きている同じスレッドの登録し直しなので、合計へ足さずに置き換える（二重計上しない）。
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

    let role = entry.role.clone();
    emit_notes(with_registry(|registry| {
        registry.insert(entry, &Win32Probe)
    }));
    debug!(
        target: "wintf::thread_registry",
        role = %role,
        tid,
        "スレッドを名簿へ登録した"
    );
    Ok(())
}

/// 掃除の後の生きた項目の一覧（スナップショット）を返す。
///
/// 返るのは複製ハンドルを共有する複写であり、以後の登録は反映されない。錠は返る前に
/// 手放すので、呼び出し側は一覧を舐めながら `GetThreadTimes` を呼んでよい。
pub fn snapshot() -> Vec<ThreadEntry> {
    let (live, notes) = with_registry(|registry| {
        let notes = registry.sweep(&Win32Probe);
        (registry.live.clone(), notes)
    });
    emit_notes(notes);
    live
}

/// 掃除の後の生きた項目の CPU と系統の畳んだ合計を、錠の中の 1 つの時点の組で返す。
///
/// 報告器向けの口。生きた項目の CPU も錠の中で読むので、1 本のスレッドの CPU は
/// 生きた項目か畳んだ合計のどちらか一方にだけ入る。
pub fn read_cpu() -> RegistryReading {
    let (reading, notes) = with_registry(|registry| registry.read(&Win32Probe));
    emit_notes(notes);
    reading
}

#[cfg(test)]
#[path = "thread_registry_tests.rs"]
mod tests;
