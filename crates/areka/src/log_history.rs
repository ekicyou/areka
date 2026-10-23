//! ログの履歴（areka-P0-mcp-log-history）。
//!
//! ログの出来事を SSP の 5 種別（error・script・network・update・status）へ振り分ける。
//! 規則は [`RULES`] の 1 表だけに書き、[`classify`] が判定の順に当てる。
//! 振り分けは tracing の受け口にも World にも触れない純粋な関数で、テストは値を直に渡す。
//! 記録は [`History`] が全種別で 1 本の通し番号を振り、種別ごとに [`PER_KIND_CAP`] 件まで持つ。
//! 入れ物はプロセスに 1 つの置き場に据え、[`record`] で積み [`snapshot`]・[`last_id`] で読む。
//! 出口は [`init`] が標準出力の層と履歴の層（[`HistoryLayer`]＋[`HistoryFilter`]）と
//! 写しの層（[`CaptureLayer`]＋[`CaptureFilter`]・[`capture_begin`]／[`capture_finish`] の窓）を重ねて据える。

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread::ThreadId;

use tracing::field::{Field, Visit};
use tracing::subscriber::Interest;
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::{Context, Filter, Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use windows::Win32::System::SystemInformation::GetLocalTime;

/// 種別（SSP の 5 語）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Error,
    Script,
    Network,
    Update,
    Status,
}

impl Kind {
    const ALL: [Kind; 5] = [
        Kind::Error,
        Kind::Script,
        Kind::Network,
        Kind::Update,
        Kind::Status,
    ];

    /// `log_type` の語（小文字）。
    pub(crate) fn word(self) -> &'static str {
        match self {
            Kind::Error => "error",
            Kind::Script => "script",
            Kind::Network => "network",
            Kind::Update => "update",
            Kind::Status => "status",
        }
    }

    /// 5 語と大文字小文字の違いを除いて一致すれば Some（前後の空白は削らない・空は None）。
    pub(crate) fn from_log_type(word: &str) -> Option<Kind> {
        Kind::ALL
            .into_iter()
            .find(|k| k.word().eq_ignore_ascii_case(word))
    }

    /// `[<種別>]` の既定の語（Error / SSTP / Info / Info / STAT）。
    pub(crate) fn default_label(self) -> &'static str {
        match self {
            Kind::Error => "Error",
            Kind::Script => "SSTP",
            Kind::Network | Kind::Update => "Info",
            Kind::Status => "STAT",
        }
    }

    /// `<名>` の既定（status は STAT、ほかは [SYSTEM]）。
    pub(crate) fn default_name(self) -> &'static str {
        match self {
            Kind::Status => "STAT",
            _ => "[SYSTEM]",
        }
    }
}

/// 取り決めの target: 再生した台本を script 種別へ残す（info 以上で出す約束）。
pub(crate) const TARGET_SCRIPT: &str = "areka::log::script";
/// 取り決めの target: error 種別へ直接残す（info 以上で出す約束）。
pub(crate) const TARGET_ERROR: &str = "areka::log::error";

/// info の行を種別へ振る規則 1 行。
pub(crate) struct Rule {
    pub target: &'static str,
    pub kind: Kind,
    /// true は完全一致だけ。false は自身か `::` で区切った下のモジュール。
    pub exact: bool,
}

impl Rule {
    /// target がこの行に当たるか（`areka::update` は `areka::update::desk` に当たり、
    /// `areka::updater` には当たらない）。
    pub(crate) fn matches(&self, target: &str) -> bool {
        match target.strip_prefix(self.target) {
            Some("") => true,
            Some(rest) => !self.exact && rest.starts_with("::"),
            None => false,
        }
    }
}

const fn rule(target: &'static str, kind: Kind, exact: bool) -> Rule {
    Rule {
        target,
        kind,
        exact,
    }
}

/// 上から順に当て、最初に当たった行の種別になる（network の行は update の行より先）。
pub(crate) const RULES: &[Rule] = &[
    rule("areka::install::fetch_url", Kind::Network, false),
    rule("areka_update::winhttp", Kind::Network, false),
    rule("areka_update::fetch", Kind::Network, false),
    rule("areka_update", Kind::Update, false),
    rule("areka::update", Kind::Update, false),
    rule("areka::install", Kind::Update, false),
    // 下のモジュールまで含めるとアプリ本体の info が全部 status になるので完全一致だけ。
    rule("areka", Kind::Status, true),
    rule("areka::boot_config", Kind::Status, false),
    rule("areka::boot_resolve", Kind::Status, false),
    rule("areka::ghost_session", Kind::Status, false),
    rule("areka::emo2_boot::ghost_switch", Kind::Status, false),
    rule("ghost-boot", Kind::Status, false),
    rule("ghost-shutdown", Kind::Status, false),
    // 外のもの（説明書・ファイル・URL・フォルダ・メール）を開いた記録。
    rule("areka::readme", Kind::Status, false),
];

/// 出来事のレベルと target から種別を決める。どれにも当たらなければ None（残さない）。
///
/// 判定の順: debug・trace は残さない → 取り決めの script → 取り決めの error
/// → warn 以上は error → info は [`RULES`] を上から。
pub(crate) fn classify(level: Level, target: &str) -> Option<Kind> {
    // tracing の `Level` は詳しいほど大きい（TRACE > DEBUG > INFO > WARN > ERROR）。
    if level > Level::INFO {
        return None;
    }
    if target == TARGET_SCRIPT {
        return Some(Kind::Script);
    }
    if target == TARGET_ERROR || level < Level::INFO {
        return Some(Kind::Error);
    }
    RULES.iter().find(|r| r.matches(target)).map(|r| r.kind)
}

/// 欄 1 つ（tracing の訪問で得る 2 つの形）。
pub(crate) struct FieldText<'a> {
    pub name: &'a str,
    /// `{:?}` の形（文字列で渡された欄は引用符つき）。
    pub debug: &'a str,
    /// 文字列で渡された欄の生の値（そうでなければ None）。
    pub raw: Option<&'a str>,
}

/// 置き場へ積む前の記録。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Draft {
    pub kind: Kind,
    pub label: String,
    pub name: String,
    pub body: String,
}

/// 本文の上限（`char` の数）。
pub(crate) const BODY_CAP_CHARS: usize = 4096;
/// 上限を超えた本文の末尾に付ける印。
pub(crate) const TRUNCATED_SUFFIX: &str = " ...(truncated)";

/// 出来事 1 件から記録の下書きを作る。種別に当たらなければ None。欄は出来事に書かれた順で渡す。
///
/// 本文は `message` の欄の後に残りの欄を ` 名前=値`（値は `{:?}` の形）で続ける。
/// 取り決めの target の行だけ `ghost`・`label` を名と表示の語として読み本文から除く
/// （同名が複数なら最後のもの）。`log.` で始まる欄（`log` クレートから橋渡しされた行）は除く。
pub(crate) fn draft<'a>(
    level: Level,
    target: &str,
    fields: impl IntoIterator<Item = FieldText<'a>>,
) -> Option<Draft> {
    let kind = classify(level, target)?;
    let convention = target == TARGET_SCRIPT || target == TARGET_ERROR;
    let (mut body, name, label) = body_text(convention, fields);
    if let Some((cut, _)) = body.char_indices().nth(BODY_CAP_CHARS) {
        body.truncate(cut);
        body.push_str(TRUNCATED_SUFFIX);
    }
    Some(Draft {
        kind,
        label: label.unwrap_or(kind.default_label()).to_string(),
        name: name.unwrap_or(kind.default_name()).to_string(),
        body,
    })
}

/// 本文を組む（切らない）。返すのは（本文・`ghost` の値・`label` の値）。
/// `convention` が真のときだけ `ghost`・`label` を本文から除いて返す（組み方は [`draft`] の注記）。
pub(crate) fn body_text<'a>(
    convention: bool,
    fields: impl IntoIterator<Item = FieldText<'a>>,
) -> (String, Option<&'a str>, Option<&'a str>) {
    let mut name = None;
    let mut label = None;
    let mut message = None;
    let mut rest = String::new();
    for f in fields {
        match f.name {
            // 標準出力の層は文字列の message を引用符なしで書くので、生の値があればそれに揃える。
            "message" => message = Some(f.raw.unwrap_or(f.debug)),
            "ghost" if convention => name = Some(f.raw.unwrap_or(f.debug)),
            "label" if convention => label = Some(f.raw.unwrap_or(f.debug)),
            n if n.starts_with("log.") => {}
            n => {
                rest.push(' ');
                rest.push_str(n);
                rest.push('=');
                rest.push_str(f.debug);
            }
        }
    }
    let body = match message {
        Some(m) => format!("{m}{rest}"),
        // メッセージが無ければ欄だけ（先頭の空白は付けない）。
        None => rest.strip_prefix(' ').unwrap_or(&rest).to_string(),
    };
    (body, name, label)
}

/// 記録した時刻（現地時刻・分まで）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

/// 置き場に積んだ記録 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Record {
    pub id: u64,
    pub at: Stamp,
    pub kind: Kind,
    pub label: String,
    pub name: String,
    pub body: String,
}

/// 種別ごとに残す件数の上限。
pub(crate) const PER_KIND_CAP: usize = 1000;

/// 通し番号を振り、種別ごとの列へ古い順に積む入れ物。
///
/// 番号は全種別で 1 本（1 から）。`last_id` は減らず、捨てた番号は使い回さない。
/// ある種別の `push` は他の種別の列を変えない。
pub(crate) struct History {
    last_id: u64,
    /// 添え字は `Kind` の宣言順。
    rows: [VecDeque<Record>; 5],
}

impl History {
    pub(crate) const fn new() -> Self {
        History {
            last_id: 0,
            rows: [const { VecDeque::new() }; 5],
        }
    }

    /// 番号を振って積む。その種別が上限なら最古の 1 件を捨ててから積む。振った番号を返す。
    pub(crate) fn push(&mut self, at: Stamp, draft: Draft) -> u64 {
        self.last_id += 1;
        let row = &mut self.rows[draft.kind as usize];
        if row.len() >= PER_KIND_CAP {
            row.pop_front();
        }
        row.push_back(Record {
            id: self.last_id,
            at,
            kind: draft.kind,
            label: draft.label,
            name: draft.name,
            body: draft.body,
        });
        self.last_id
    }

    /// いま最後に振った番号（1 件も無ければ 0）。
    pub(crate) fn last_id(&self) -> u64 {
        self.last_id
    }

    /// その種別の記録（古い順）。
    pub(crate) fn rows(&self, kind: Kind) -> impl Iterator<Item = &Record> {
        self.rows[kind as usize].iter()
    }
}

/// プロセスに 1 つの置き場。
///
/// 排他を握るのは [`record`]・[`snapshot`]・[`last_id`] の 3 か所だけで、どれも排他の中で
/// tracing のマクロも呼び手の処理も走らせない（ログを出す側が自分の排他で固まる道が無い）。
static STORE: Mutex<History> = Mutex::new(History::new());

/// 排他を取る。毒されていたら中身を取り出して続ける（`push` は失敗の道を持たないので
/// パニックした側が途中で止まっても入れ物は壊れていない）。
fn lock(store: &Mutex<History>) -> MutexGuard<'_, History> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 置き場へ 1 件積み、振った番号を返す（履歴の層とテストが使う同じ口）。
pub(crate) fn record(at: Stamp, draft: Draft) -> u64 {
    lock(&STORE).push(at, draft)
}

/// いま最後に振った通し番号（1 件も無ければ 0）。MCP の読み直しの受理が返事に添える。
pub(crate) fn last_id() -> u64 {
    lock(&STORE).last_id()
}

/// その種別の記録を写して返す（古い順・最大 [`PER_KIND_CAP`] 件）。
/// 排他を握るのは写す間だけで、呼び手は排他の外で絞って整形する。
pub(crate) fn snapshot(kind: Kind) -> Vec<Record> {
    lock(&STORE).rows(kind).cloned().collect()
}

/// 現地時刻（`GetLocalTime`・分まで）。
fn local_now() -> Stamp {
    // SAFETY: GetLocalTime は出力先を自前で用意して埋めるだけで、前提も失敗の道も無い。
    let t = unsafe { GetLocalTime() };
    // 月・日・時・分は Windows の定義で u8 に収まる範囲（1〜12・1〜31・0〜23・0〜59）。
    Stamp {
        year: t.wYear,
        month: t.wMonth as u8,
        day: t.wDay as u8,
        hour: t.wHour as u8,
        minute: t.wMinute as u8,
    }
}

/// 履歴の層がその呼び出し口を必ず欲しいか（出来事で、かつ振り分けが当たる）。
/// 偽なら「関心なし」で、その呼び出し口は履歴の層へ届かない。
pub(crate) fn wants(is_event: bool, level: Level, target: &str) -> bool {
    is_event && classify(level, target).is_some()
}

/// 履歴の層のフィルタが通すか。出来事でない問い合わせ（`tracing::enabled!`・スパン）は必ず通す。
///
/// 層ごとのフィルタは断るたびに「断った」印をスレッドに書き、直後の `on_event` で消す。
/// 出来事でない問い合わせを断ると印が残り、同じスレッドの次の出来事を履歴の層だけが
/// 1 件飛ばす（design-validation.md 指摘 1）。
pub(crate) fn passes(is_event: bool, level: Level, target: &str) -> bool {
    !is_event || classify(level, target).is_some()
}

/// 履歴の層の規則のフィルタ（答えは [`wants`]・[`passes`]。`filter_fn` は上の印の理由で使わない）。
pub(crate) struct HistoryFilter;

impl<S> Filter<S> for HistoryFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        passes(meta.is_event(), *meta.level(), meta.target())
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        if wants(meta.is_event(), *meta.level(), meta.target()) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        // 履歴が残すのは info 以上だけ（取り決めの行も info 以上で出す約束）。
        Some(LevelFilter::INFO)
    }
}

/// 出来事の欄を書かれた順に集める（文字列の欄は生の値と `{:?}` の両方）。
#[derive(Default)]
struct Fields(Vec<(&'static str, String, Option<String>)>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push((field.name(), format!("{value:?}"), None));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0
            .push((field.name(), format!("{value:?}"), Some(value.to_string())));
    }
}

impl Fields {
    fn of(event: &Event<'_>) -> Self {
        let mut fields = Fields::default();
        event.record(&mut fields);
        fields
    }

    fn texts(&self) -> impl Iterator<Item = FieldText<'_>> {
        self.0.iter().map(|(name, debug, raw)| FieldText {
            name,
            debug,
            raw: raw.as_deref(),
        })
    }
}

/// 当たる出来事を下書きにし、出したスレッドの上で同期に置き場へ積む層。
pub(crate) struct HistoryLayer;

impl<S: Subscriber> Layer<S> for HistoryLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let fields = Fields::of(event);
        let meta = event.metadata();
        // `log` から橋渡しされた行はここへ target `log` で届く。info 以下は下書きが None で捨てる。
        if let Some(d) = draft(*meta.level(), meta.target(), fields.texts()) {
            record(local_now(), d);
        }
    }
}

/// 写しの窓（areka-P0-offline-surface-dump）の時刻（現地時刻・秒まで）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hms {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// 窓に写した記録 1 件（本文は切らない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedRecord {
    pub at: Hms,
    pub level: Level,
    pub body: String,
}

/// プロセスに 1 つの窓（開けたスレッドの番号と写した記録）。閉じていれば None。
/// 排他の中で tracing のマクロも本文の組み立ても走らせない（置き場と同じ約束）。
static WINDOW: Mutex<Window> = Mutex::new(None);

type Window = Option<(ThreadId, Vec<CapturedRecord>)>;

fn window() -> MutexGuard<'static, Window> {
    WINDOW.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 窓を開ける（開いていれば中身を空にして開け直す）。呼んだスレッドの番号を覚える。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.7）
pub(crate) fn capture_begin() {
    *window() = Some((std::thread::current().id(), Vec::new()));
}

/// 窓を閉じて、写した記録を残した順に返す（開いていなければ空）。
#[allow(dead_code)] // 呼び手はオフラインの書き出しの走り（タスク 7.7）
pub(crate) fn capture_finish() -> Vec<CapturedRecord> {
    window().take().map(|(_, rows)| rows).unwrap_or_default()
}

/// 窓が開いていて、呼んだスレッドが窓を開けたスレッドか。
fn capture_open_here() -> bool {
    matches!(&*window(), Some((id, _)) if *id == std::thread::current().id())
}

/// 窓へ 1 件積む（写しの層とテストが使う同じ口）。窓が閉じているか、窓を開けたスレッドで
/// なければ積まない。
pub(crate) fn capture_push(at: Hms, level: Level, body: String) {
    let here = std::thread::current().id();
    if let Some((id, rows)) = &mut *window()
        && *id == here
    {
        rows.push(CapturedRecord { at, level, body });
    }
}

/// 写しの層がその呼び出し口を必ず欲しいか（出来事で warn 以上）。
pub(crate) fn capture_wants(is_event: bool, level: Level) -> bool {
    is_event && level <= Level::WARN
}

/// 写しの層のフィルタが通すか。出来事でない問い合わせは必ず通す（[`passes`] と同じ理由）。
pub(crate) fn capture_passes(is_event: bool, level: Level) -> bool {
    !is_event || level <= Level::WARN
}

/// 写しの層の規則のフィルタ。warn 以上は履歴の層が既に `always` なので受け口の関心は変わらない。
pub(crate) struct CaptureFilter;

impl<S> Filter<S> for CaptureFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        capture_passes(meta.is_event(), *meta.level())
    }

    fn callsite_enabled(&self, meta: &'static Metadata<'static>) -> Interest {
        if capture_wants(meta.is_event(), *meta.level()) {
            Interest::always()
        } else {
            Interest::never()
        }
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::WARN)
    }
}

/// 窓が開いているあいだ、窓を開けたスレッドの warn 以上を切らない本文で積む層。
pub(crate) struct CaptureLayer;

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // 窓が閉じているあいだの費用はロック 1 つの確かめだけ。
        if !capture_open_here() {
            return;
        }
        let meta = event.metadata();
        let convention = meta.target() == TARGET_SCRIPT || meta.target() == TARGET_ERROR;
        let fields = Fields::of(event);
        let (body, _, _) = body_text(convention, fields.texts());
        capture_push(local_hms(), *meta.level(), body);
    }
}

/// 現地時刻（`GetLocalTime`・秒まで）。
fn local_hms() -> Hms {
    // SAFETY: GetLocalTime は出力先を自前で用意して埋めるだけで、前提も失敗の道も無い。
    let t = unsafe { GetLocalTime() };
    // 時・分・秒は Windows の定義で u8 に収まる範囲（0〜23・0〜59・0〜59）。
    Hms {
        hour: t.wHour as u8,
        minute: t.wMinute as u8,
        second: t.wSecond as u8,
    }
}

/// ログの出口を据える（`fn main()` の先頭で 1 度だけ。2 度目はパニックする）。
/// 標準出力の層（`RUST_LOG` のフィルタ・未設定や不正なら info）と履歴の層と写しの層を重ねる。
/// `init()` は `log` クレートの受け口も据える。
pub(crate) fn init() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        ))
        .with(HistoryLayer.with_filter(HistoryFilter))
        .with(CaptureLayer.with_filter(CaptureFilter))
        .init();
}

#[cfg(test)]
#[path = "log_history_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "log_history_convention_tests.rs"]
mod convention_tests;

#[cfg(test)]
#[path = "log_history_capture_tests.rs"]
mod capture_tests;
