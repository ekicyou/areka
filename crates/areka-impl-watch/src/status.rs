//! `status.md` の組み立て・端末向け ASCII の要約・UTC の文字化。
//!
//! どちらの組み立ても状態を読むだけで、いまの時刻と生死の答えは引数で受ける（ファイルも時計も
//! 触らない）。日本語の値（名前・内容）が載るのは読み物（[`render_markdown`]）だけで、端末向けの
//! 要約（[`render_terminal`]）は識別・リポジトリ・spec・PR・sha・時刻と数だけを使う。
//! このファイルの文字列リテラルは、読み物の見出しも含めて全部 ASCII。

// 使い手（store・cli）が載るまで、本番のビルドではここが未使用になる。
// 「満たされない expect」の警告が出たら外す。理由の文もこのファイルの文字列リテラルなので ASCII。
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "the users of this module arrive in later tasks")
)]

use serde::Serialize;

use crate::error::escape_path;
use crate::plan::Presence;
use crate::state::{
    LastMerge, LoadRequest, MergeRequest, Participant, ParticipantStatus, State, WaitKind,
};

/// 「居ない・無い」の綴り（持ち主なし・待ち行列なし・直前のマージなし・項目なし）。
const NONE: &str = "none";

/// 状態の確認の読み物（UTF-8・`status.md` の中身）。`now` は書いた時刻。
pub fn render_markdown(state: &State, presence: &dyn Presence, now: u64) -> String {
    let mut out = vec![
        "# areka-impl-watch status".to_owned(),
        String::new(),
        format!("generated: {}", utc(now)),
    ];

    let participants = state.participants.iter().map(|(id, p)| {
        let reason = p
            .stop_reason
            .as_ref()
            .map(|r| format!("by {}: {}", r.by, r.purpose));
        let watch = p
            .watch
            .as_ref()
            .map(|w| format!("pid {} since {}", w.pid, utc(w.since)));
        row(&[
            id,
            &p.name,
            &p.repo,
            &spelling(p.status),
            &utc(p.since),
            mark(id, p, presence).unwrap_or("-"),
            reason.as_deref().unwrap_or("-"),
            watch.as_deref().unwrap_or("-"),
        ])
    });
    table(
        &mut out,
        "Participants",
        "| id | name | repo | status | since | mark | stop reason | watch |",
        participants.collect(),
    );

    section(&mut out, "Load desk");
    out.push(match &state.load.holder {
        Some(holder) => format!(
            "- holder: {} (requested {}, granted {}, running: {}, stopped: {}): {}",
            holder.id,
            utc(holder.requested),
            utc(holder.granted),
            yes_no(holder.running),
            if holder.stopped.is_empty() {
                NONE.to_owned()
            } else {
                holder.stopped.join(", ")
            },
            text(&holder.purpose),
        ),
        None => format!("- holder: {NONE}"),
    });
    let waiting = load_order(&state.load.queue).into_iter().map(|request| {
        format!(
            "{} (requested {}): {}",
            request.id,
            utc(request.requested),
            text(&request.purpose)
        )
    });
    queue(&mut out, waiting.collect());

    section(&mut out, "Merge desks");
    if state.merge.is_empty() {
        out.push(NONE.to_owned());
    }
    for (nth, (repo, desk)) in state.merge.iter().enumerate() {
        if nth > 0 {
            out.push(String::new());
        }
        out.extend([format!("### {repo}"), String::new()]);
        out.push(match &desk.holder {
            Some(holder) => format!(
                "- holder: {} (spec {}, bug: {}, requested {}, granted {})",
                holder.id,
                holder.spec,
                yes_no(holder.bug),
                utc(holder.requested),
                utc(holder.granted),
            ),
            None => format!("- holder: {NONE}"),
        });
        let waiting = merge_order(&desk.queue).into_iter().map(|request| {
            format!(
                "{} (spec {}, bug: {}, requested {})",
                request.id,
                request.spec,
                yes_no(request.bug),
                utc(request.requested),
            )
        });
        queue(&mut out, waiting.collect());
        out.push(format!("- last: {}", last_merge(desk.last.as_ref())));
    }

    let waits = state.waits.iter().map(|wait| {
        row(&[
            &spelling(wait.kind),
            &wait.id,
            wait.repo.as_deref().unwrap_or("-"),
            &wait.pid.to_string(),
            &utc(wait.since),
        ])
    });
    table(
        &mut out,
        "Waits and watches",
        "| kind | id | repo | pid | since |",
        waits.collect(),
    );

    let recent = state.recent.iter().map(|recent| {
        row(&[
            &utc(recent.at),
            &spelling(recent.kind),
            recent.id.as_deref().unwrap_or("-"),
            &recent.detail,
        ])
    });
    table(
        &mut out,
        "Recent reclaims, recoveries and clears",
        "| at | kind | id | detail |",
        recent.collect(),
    );

    out.join("\n") + "\n"
}

/// 端末向けの要約（ASCII だけ）。1 行目は数、続けて参加者 1 人 1 行、マージの机 1 つ 1 行、
/// 負荷テストの机 1 行。名前・内容・停止要請の内容は使わない。
///
/// 1 行目の数: `participants`＝参加者／`load-queued`＝負荷テストの待ち行列の申し込み／
/// `merging`＝持ち主の居るマージの机（リポジトリ）／`merge-queued`＝全リポジトリのマージの
/// 待ち行列の申し込みの合計／`not-working`＝「停止要請中」と「止まった」の参加者／
/// `waits`＝走っている待ち・見張りの記録（見張りを含む）。
pub fn render_terminal(state: &State, presence: &dyn Presence) -> String {
    let load_holder = state
        .load
        .holder
        .as_ref()
        .map_or(NONE, |holder| holder.id.as_str());
    let mut lines = vec![format!(
        "participants={} load-holder={load_holder} load-queued={} merging={} merge-queued={} not-working={} waits={}",
        state.participants.len(),
        state.load.queue.len(),
        state
            .merge
            .values()
            .filter(|desk| desk.holder.is_some())
            .count(),
        state
            .merge
            .values()
            .map(|desk| desk.queue.len())
            .sum::<usize>(),
        state
            .participants
            .values()
            .filter(|p| p.status != ParticipantStatus::Working)
            .count(),
        state.waits.len(),
    )];
    for (id, participant) in &state.participants {
        let mark = mark(id, participant, presence).map_or(String::new(), |mark| format!(" {mark}"));
        lines.push(format!(
            "{id} {} {}{mark}",
            participant.repo,
            spelling(participant.status)
        ));
    }
    for (repo, desk) in &state.merge {
        lines.push(format!(
            "merge {repo}: holder={} queue={} last={}",
            desk.holder
                .as_ref()
                .map_or(NONE, |holder| holder.id.as_str()),
            desk.queue.len(),
            last_merge(desk.last.as_ref()),
        ));
    }
    let waiting: Vec<&str> = load_order(&state.load.queue)
        .into_iter()
        .map(|request| request.id.as_str())
        .collect();
    lines.push(format!(
        "load: holder={load_holder} queue={}",
        if waiting.is_empty() {
            NONE.to_owned()
        } else {
            waiting.join(",")
        },
    ));
    // 識別・リポジトリ・spec は引数の形の決まりで ASCII だが、手で直した状態ファイルからは
    // 何でも来うる。行ごとに ASCII の外の字を逃がして、出力が ASCII だけであることを値に頼らない。
    lines.iter().map(|line| escape_path(line) + "\n").collect()
}

/// UNIX 秒を人が読む UTC の文字にする（`2026-10-10T12:34:56Z`）。
pub fn utc(secs: u64) -> String {
    let [year, month, day, hour, minute, second] = civil(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// UNIX 秒をファイル名に使える UTC の文字にする（`20261010T123456Z`）。
pub fn utc_compact(secs: u64) -> String {
    let [year, month, day, hour, minute, second] = civil(secs);
    format!("{year:04}{month:02}{day:02}T{hour:02}{minute:02}{second:02}Z")
}

/// UNIX 秒 → UTC の `[年, 月, 日, 時, 分, 秒]`（グレゴリオ暦）。
/// 3 月 1 日を年の初めに置いて、うるう日を年の末尾に寄せる数え方（400 年＝146,097 日の周期）。
fn civil(secs: u64) -> [u64; 6] {
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // 0000-03-01 からの日数（1970-01-01 はその 719,468 日後）。
    let days = days + 719_468;
    let (era, day_of_era) = (days / 146_097, days % 146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    // 3 月 1 日を 0 とする年内の日。
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // 3 月を 0 とする月。
    let march_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * march_month + 2) / 5 + 1;
    let month = if march_month < 10 {
        march_month + 3
    } else {
        march_month - 9
    };
    // 1 月と 2 月は次の年に属する。
    let year = era * 400 + year_of_era + u64::from(month <= 2);
    [year, month, day, rest / 3_600, rest % 3_600 / 60, rest % 60]
}

/// 参加者の印。`awaiting-watch` は「作業中」のときだけ示す（停止要請の出し直しの後も印の時刻は
/// 残る）。`absent` は、生死の口が「見張りが居ない」と答えた残りの参加者全部（要件 7.7。
/// 「停止要請中」「止まった」は見張りが終わっているのが普通だが、自動では回収されないので、
/// 落ちたセッションを開発者が見分けて手で外せるように同じ印を付ける）。
/// 人が読む見張りの記録（`Participant::watch`）は、見張りが殺されても残るので見ない。
fn mark(id: &str, participant: &Participant, presence: &dyn Presence) -> Option<&'static str> {
    if participant.status == ParticipantStatus::Working
        && participant.awaiting_watch_since.is_some()
    {
        Some("awaiting-watch")
    } else if !presence.is_present(id, WaitKind::Watch) {
        Some("absent")
    } else {
        None
    }
}

/// マージの待ち行列を番の来る順に並べる: バグ優先 → 申し込みの早い順・同点は行列の前
/// （判断が番を選ぶ鍵と同じ。行列そのものは到着順で持たれている）。
fn merge_order(queue: &[MergeRequest]) -> Vec<&MergeRequest> {
    let mut order: Vec<&MergeRequest> = queue.iter().collect();
    order.sort_by_key(|request| (!request.bug, request.requested));
    order
}

/// 負荷テストの待ち行列を番の来る順に並べる: 申し込みの早い順・同点は行列の前。
fn load_order(queue: &[LoadRequest]) -> Vec<&LoadRequest> {
    let mut order: Vec<&LoadRequest> = queue.iter().collect();
    order.sort_by_key(|request| request.requested);
    order
}

/// 直前のマージの 1 行分（`PR#281 414d43eb impl-watch 2026-10-10T01:00:00Z`）。
pub(crate) fn last_merge(last: Option<&LastMerge>) -> String {
    last.map_or(NONE.to_owned(), |last| {
        format!("PR#{} {} {} {}", last.pr, last.sha, last.spec, utc(last.at))
    })
}

/// 列挙の ASCII の綴り。状態ファイルと同じ綴り（serde の `kebab-case`）を唯一の表にする。
fn spelling(value: impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "?".to_owned(),
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

/// 自由な文（名前・内容・道筋）を読み物の 1 行・表の 1 欄に収める。
fn text(value: &str) -> String {
    value.replace('|', "\\|").replace(['\r', '\n'], " ")
}

fn row(cells: &[&str]) -> String {
    let cells: Vec<String> = cells.iter().map(|cell| text(cell)).collect();
    format!("| {} |", cells.join(" | "))
}

fn section(out: &mut Vec<String>, title: &str) {
    out.extend([String::new(), format!("## {title}"), String::new()]);
}

/// 見出しつきの表。行が無ければ表の代わりに `none`。
fn table(out: &mut Vec<String>, title: &str, header: &str, rows: Vec<String>) {
    section(out, title);
    if rows.is_empty() {
        out.push(NONE.to_owned());
        return;
    }
    let columns = header.matches('|').count() - 1;
    out.extend([header.to_owned(), format!("|{}", "---|".repeat(columns))]);
    out.extend(rows);
}

/// 番の来る順の待ち行列（番号つき）。空なら `none`。
fn queue(out: &mut Vec<String>, waiting: Vec<String>) {
    if waiting.is_empty() {
        out.push(format!("- queue: {NONE}"));
        return;
    }
    out.push("- queue:".to_owned());
    out.extend(
        waiting
            .iter()
            .enumerate()
            .map(|(nth, item)| format!("  {}. {item}", nth + 1)),
    );
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
