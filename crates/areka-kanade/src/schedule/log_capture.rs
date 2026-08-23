//! テスト専用ログ捕捉ヘルパ（タスク 6.1・Req 6.1／6.3）。
//!
//! 純粋関数 [`crate::schedule::step`]（および各サブモジュール `step`）は、失敗・防御
//! アームごとに `tracing::{error,warn}!` を発行する（steering:
//! areka-log-first-no-silent-failure・「ログ無しの失敗経路を持たない」）。本モジュールは
//! その発行を**実行可能なテストで**捕捉し、各アームが規約どおりの `target="kanade"`・
//! `event=<語彙>`・レベル（ERROR/WARN）でログを出していることを検証可能にする。
//!
//! # 硬化機構は 1 箇所にしかない
//! 捕捉窓そのものは**共有 crate [`log_capture_kit`] へ委譲**する（spec:
//! areka-P0-test-cage-determinism・要件 1.5／2.2）。本モジュールが持つのは
//! 「この crate の檻が照合する形（[`CapturedEvent`]）への変換」と表明ヘルパだけで、
//! subscriber の設置も interest の管理も自前では行わない。以前ここにあった
//! interest-keeper（プロセス全体の既定 subscriber を常駐させる方式）は、同じ機構が
//! crate ごとに写し取られて写し損ねた側だけが静かに嘘をつく形だったため、共有 crate へ
//! 一本化して撤去した。
//!
//! 捕捉されるのは**呼出スレッド**で同期的に発火したイベントだけである。`step()` は
//! テストスレッド上で同期的に走る純粋関数ゆえ確実に捕えられる（spawn したアクター
//! スレッドのログは捕えない——それはタスク 6.2 の担当）。
//!
//! # 機序（「スレッドローカルゆえ安全」は誤り）
//! `tracing::subscriber::with_default` が差し替えるのはスレッドローカルの既定 dispatcher
//! だけで、そこは確かにスレッドごとに独立している。しかし「そのログを評価するか」を決める
//! callsite の **interest キャッシュはプロセス全体で 1 つ**であり、その発行点をプロセス内で
//! 最初に踏んだスレッドの判定が焼き付く。捕捉窓を持たないスレッドの既定は `NoSubscriber` で
//! 判定は「不要」なので、先に踏まれると `never` が大域へ焼き付き、自分のスレッドへ捕捉先を
//! 差していても以後そのイベントは早期 return で捨てられる。結果、不在の主張は捕捉 0 件のまま
//! 静かに緑になり（偽陰性）、存在の主張は捕捉 0 件で確率的に赤になる（偽陽性）。
//!
//! [`log_capture_kit::capture`] はこれを ⑴ プロセス寿命の probe 常駐 ⑵ 窓の内側での
//! interest 再計算 ⑶ 窓の内側で発火する対照イベント（番兵）による空振り検出、の 3 点で塞ぐ。
//! 番兵は返却前に取り除かれるので、呼出側の件数・主張は変わらない。機序の逐条解説と
//! `tracing-core` の実コード引用は `log_capture_kit` の crate doc および同 crate の
//! `src/probe.rs` にある。

use std::collections::BTreeMap;

use log_capture_kit::CapturedEvent as CanonicalEvent;
use tracing::Level;

/// 捕捉した 1 イベント（照合対象は target／event／outcome／message／level）。
#[derive(Debug, Clone)]
pub(crate) struct CapturedEvent {
    pub target: String,
    /// 構造化フィールド `event`（区別語彙）の値。未設定なら `None`。
    pub event: Option<String>,
    /// 構造化フィールド `outcome`（リソース照会 prefetch 完了固定ログの分類値・R9.3）。未設定なら `None`。
    pub outcome: Option<String>,
    /// イベントメッセージ本文（固定ログ `"shiori resource prefetch done"` の照合に使う）。未設定なら `None`。
    pub message: Option<String>,
    pub level: Level,
    /// 構造化フィールドの全記録（フィールド名 → 値の文字列表現）。
    ///
    /// 文字列フィールドは素の値、それ以外（数値・`?expr`／`%expr` の Debug 記録）は `Debug`
    /// 表現が入る。「ログに載っていること」自体が要求である値（例: 選択待ち帳簿確立の候補数・
    /// 期限）を檻から突合するために使う。
    pub fields: BTreeMap<String, String>,
}

/// 正準イベントから 1 フィールドの値を、移行前の visitor と同じ規則で取り出す。
///
/// 移行前の `EventFieldVisitor` は `record_str` 経路を `insert`（後勝ち）で、`record_debug`
/// 経路を `or_insert_with`（先勝ち・生値があれば上書きしない）で記録していた。すなわち
/// **生値が 1 つでもあれば最後の生値、無ければ最初の Debug 表現**である。ここではその規則を
/// そのまま再現する。
///
/// 戻り値の第 2 要素は「生値（`record_str` 経路）だったか」で、`event`／`outcome` の
/// 引用符剥がしを Debug 経路だけに限るために使う。
///
/// **`log_capture_kit::CapturedEvent::field_str` だけで書いてはならない**——`?expr`／`%expr`
/// で渡されたフィールドは `record_str` を通らないため `None` になり、その経路の値が黙って
/// 落ちて判定が空振りになる（`log_capture_kit` の crate doc「注意」節）。
fn field_of(ev: &CanonicalEvent, name: &str) -> Option<(String, bool)> {
    let mut last_str: Option<&str> = None;
    let mut first_debug: Option<&str> = None;
    for (field_name, value) in &ev.fields {
        if field_name != name {
            continue;
        }
        match &value.str_raw {
            Some(raw) => last_str = Some(raw.as_str()),
            None if first_debug.is_none() => first_debug = Some(value.debug.as_str()),
            None => {}
        }
    }
    match (last_str, first_debug) {
        (Some(raw), _) => Some((raw.to_string(), true)),
        (None, Some(debug)) => Some((debug.to_string(), false)),
        (None, None) => None,
    }
}

/// `event`／`outcome` の語彙値。生値はそのまま、Debug 表現は囲みの引用符を剥がして返す
/// （移行前の `format!("{value:?}").trim_matches('"')` と同じ保険）。
fn label_of(ev: &CanonicalEvent, name: &str) -> Option<String> {
    field_of(ev, name).map(|(value, from_str)| {
        if from_str {
            value
        } else {
            value.trim_matches('"').to_string()
        }
    })
}

impl CapturedEvent {
    /// 共有機構の正準イベントから、本 crate の檻が照合する形へ変換する。
    fn from_canonical(ev: &CanonicalEvent) -> Self {
        let mut fields: BTreeMap<String, String> = BTreeMap::new();
        for (name, _) in &ev.fields {
            if fields.contains_key(name.as_str()) {
                continue;
            }
            if let Some((value, _)) = field_of(ev, name) {
                fields.insert(name.clone(), value);
            }
        }
        Self {
            target: ev.target.clone(),
            event: label_of(ev, "event"),
            outcome: label_of(ev, "outcome"),
            // メッセージ本文は `fmt::Arguments` ゆえ Debug 経路で届き、その `{:?}` は整形済みの
            // 本文そのもの（引用符なし）になる——移行前も引用符は剥がしていない。
            message: field_of(ev, "message").map(|(value, _)| value),
            level: ev.level,
            fields,
        }
    }
}

/// `f` を実行し、その間にテストスレッドで発行された `tracing` イベントを捕捉して返す。
///
/// 捕捉窓は [`log_capture_kit::capture`]（硬化機構の唯一の定義元）。捕捉が働いていなければ
/// 空の結果を静かに返さず panic する。
pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    let ((), events) = log_capture_kit::capture(f);
    events.iter().map(CapturedEvent::from_canonical).collect()
}

/// 捕捉列に `target="kanade"`・`event=event_name`・`level` のイベントが存在することを表明する。
///
/// アームのログが削除・語彙変更・レベル変更されると本表明は失敗する（Req 6.1／6.3 の回帰檻）。
pub(crate) fn assert_logged(events: &[CapturedEvent], level: Level, event_name: &str) {
    let hit = events.iter().any(|e| {
        e.target == "kanade" && e.level == level && e.event.as_deref() == Some(event_name)
    });
    assert!(
        hit,
        "期待ログ未検出: target=\"kanade\" level={level} event=\"{event_name}\"。\n捕捉={events:#?}"
    );
}

/// 捕捉列に `target="kanade"`・`event=event_name` のイベントが**存在しない**ことを表明する。
///
/// [`assert_logged`] の否定側。「この経路では当該語彙が発火してはならない」ことが要求である檻
/// （例: 1 世代 stale 防御が効いているとき `unknown_talk_done` は発火しない・C4 規則 9）で使う。
pub(crate) fn assert_not_logged(events: &[CapturedEvent], event_name: &str) {
    assert!(
        !events
            .iter()
            .any(|e| e.target == "kanade" && e.event.as_deref() == Some(event_name)),
        "発火してはならないログを検出: target=\"kanade\" event=\"{event_name}\"。\n捕捉={events:#?}"
    );
}

/// 捕捉列に ERROR レベルのイベントが 1 件も無いことを表明する。
///
/// 「正常系のユーザー操作で error レベルのログが出ない」ことが完了状態そのものである檻
/// （選択の happy path・タスク 4.6）で使う。target は限定しない——正常系で error を出す層は
/// 存在してはならないためである。
pub(crate) fn assert_no_error_logs(events: &[CapturedEvent]) {
    let errors: Vec<_> = events.iter().filter(|e| e.level == Level::ERROR).collect();
    assert!(
        errors.is_empty(),
        "正常系の操作で error レベルのログが出てはならない。\n検出={errors:#?}"
    );
}

/// 捕捉列から `target="kanade"`・`event=event_name`・`level` のイベントを 1 件取り出す。
///
/// [`assert_logged`] が「発火したこと」だけを固定するのに対し、本関数は**フィールド値まで
/// 突合する檻**（ログに載っていること自体が要求である値の検証）のために本体を返す。
/// 該当が 0 件、または複数件のときは panic する（1 発火であることも同時に固定する）。
pub(crate) fn logged_once<'a>(
    events: &'a [CapturedEvent],
    level: Level,
    event_name: &str,
) -> &'a CapturedEvent {
    let mut hits = events.iter().filter(|e| {
        e.target == "kanade" && e.level == level && e.event.as_deref() == Some(event_name)
    });
    let first = hits.next().unwrap_or_else(|| {
        panic!(
            "期待ログ未検出: target=\"kanade\" level={level} event=\"{event_name}\"。\n捕捉={events:#?}"
        )
    });
    assert!(
        hits.next().is_none(),
        "期待ログは 1 回だけ発火すべき: target=\"kanade\" level={level} event=\"{event_name}\"。\n捕捉={events:#?}"
    );
    first
}

/// リソース照会 prefetch の完了固定ログ（R9.3 grep 証跡）が**ちょうど 1 回**発火したことを表明する。
///
/// 固定ログは `info!(target: "areka_kanade::resource", id = "username", outcome = <outcome_label>,
/// "shiori resource prefetch done")`（design Postconditions・研究 §12-10）。target・level(INFO)・
/// `outcome` フィールド値・message 本文の全一致で照合し、発火回数が 1 であることまで固定する
/// （target が `kanade` でなく `areka_kanade::resource` ゆえ [`assert_logged`] は使えない・専用檻）。
pub(crate) fn assert_resource_prefetch_logged_once(events: &[CapturedEvent], outcome_label: &str) {
    let hits = events
        .iter()
        .filter(|e| {
            e.target == "areka_kanade::resource"
                && e.level == Level::INFO
                && e.outcome.as_deref() == Some(outcome_label)
                && e.message.as_deref() == Some("shiori resource prefetch done")
        })
        .count();
    assert_eq!(
        hits, 1,
        "prefetch 完了固定ログ（target=\"areka_kanade::resource\" level=INFO \
         outcome=\"{outcome_label}\" message=\"shiori resource prefetch done\"）は\
         ちょうど 1 回発火すべき（実際={hits}）。\n捕捉={events:#?}"
    );
}
