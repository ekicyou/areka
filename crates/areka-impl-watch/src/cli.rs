//! 入口・コマンドの手順・終了コードへの写像。
//!
//! 引数の種類・値の形・コマンドの表・`--help` の本文・読みは子モジュール `args`
//! （`cli_args.rs`）が持ち、ここは読んだ呼び出しを手順へ渡す。
//!
//! 順は「引数を読む → `--help` ならここで終わる → 置き場所を決める → コマンドの手順」。
//! 使い方の誤りと `--help` は置き場所を決める前に終わるので、何も読み書きしない。
//! 直ちに終わるコマンドの手順は 3 つ: 状態を変える 1 回（[`change`]）・状態の確認
//! （[`status`]）・全部消す（[`clear`]）。どれも「口を開く → 1 回 → 出力 → 終了コード」。
//! 待つコマンド（見張り・`--wait`・再開の待ち）の手順は [`waiting`] の 1 つ。
//!
//! 標準出力には結果（終了コード 0 の文）だけ、断り・「消えた」・失敗は標準エラーへ出す。このファイルの文字列リテラルは全部
//! ASCII で、端末へ出す文に載せるのは識別・リポジトリ・spec・PR・sha・時刻と数だけ（名前と
//! 内容は載せない）。打たれた引数と置き場所の道筋を文に映すときは ASCII の外の字を逃がす
//! （[`escape_path`]）。

use std::ffi::OsString;
use std::io::{self, Write};

use crate::error::{WatchError, escape_path};
use crate::home::{self, Home};
use crate::plan::{self, Event, Verdict};
use crate::presence::hold;
use crate::state::{State, WaitKind};
use crate::status;
use crate::store;
use crate::wait::{self, WaitEnd, WaitSpec};

#[path = "cli_args.rs"]
mod args;

use args::{Command, EXE, UsageError, parse, usage};

/// 実行した結果。失敗は `Err`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// できた。
    Done,
    /// 当てはまらなかった。
    NotApplied,
    /// 使い方の誤り。
    Usage,
}

/// 結果から終了コードへの写像（ここ 1 か所。`--help` の `exit codes:` と同じ 4 つ）。
pub fn exit_code(result: &Result<Outcome, WatchError>) -> u8 {
    match result {
        Ok(Outcome::Done) => 0,
        Err(_) => 1,
        Ok(Outcome::Usage) => 2,
        Ok(Outcome::NotApplied) => 3,
    }
}

/// 入口。結果は標準出力、断りと失敗は標準エラーへ出す。
///
/// `home_value` は環境変数 `AREKA_IMPL_WATCH_HOME` の値（テストは値を直に渡す）。
pub fn run(args: &[String], home_value: Option<OsString>) -> Result<Outcome, WatchError> {
    run_with(args, home_value, &mut io::stdout(), &mut io::stderr())
}

/// [`run`] の中身。出力の行き先を受け取る（テストが読めるようにするため）。
pub(crate) fn run_with(
    args: &[String],
    home_value: Option<OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, WatchError> {
    let result = dispatch(args, home_value, out, err);
    if let Err(failure) = &result {
        // 失敗の本文（ASCII の 1 行）。ここで書けなければ、伝える先がもう無い（終了コードは 1 のまま）。
        let _ = writeln!(err, "{failure}");
    }
    result
}

fn dispatch(
    args: &[String],
    home_value: Option<OsString>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, WatchError> {
    let invocation = match parse(args) {
        Ok(invocation) => invocation,
        Err(UsageError(what)) => {
            let line = format!("usage error: {what}; see {EXE} --help");
            say(err, "stderr", &line)?;
            return Ok(Outcome::Usage);
        }
    };
    let name = invocation.name;
    // 置き場所を決めるのは `--help` 以外の枝だけ（呼ばれるのはどれか 1 つ）。
    let home = || home::resolve(home_value);
    match invocation.command {
        Command::Help => say(out, "stdout", usage()).map(|()| Outcome::Done),
        Command::Status => status(home()?, name, out),
        Command::Clear => clear(home()?, name, out),
        Command::Watch {
            id,
            repo,
            name: shown,
        } => {
            let spec = WaitSpec::Watch {
                id,
                repo,
                name: shown,
            };
            waiting(home()?, name, &spec, None, out, err)
        }
        Command::Resume { id } => waiting(home()?, name, &WaitSpec::Resume { id }, None, out, err),
        Command::Change(command) => match wait_after(&command).filter(|_| invocation.wait) {
            Some(spec) => waiting(home()?, name, &spec, Some(&command), out, err),
            None => change(home()?, name, &command, out, err),
        },
    }
}

/// `--wait` を付けたときに、そのコマンドの後に続ける待ち（`--wait` を取らないコマンドには無い）。
fn wait_after(command: &plan::Command) -> Option<WaitSpec> {
    use plan::Command as C;
    match command {
        C::Merge { id, repo, .. } => {
            let (id, repo) = (id.clone(), repo.clone());
            Some(WaitSpec::Merge { id, repo })
        }
        C::Request { id, .. } => Some(WaitSpec::Request { id: id.clone() }),
        // 「止まった」を記録してから、再開を待つ。
        C::Stopped { id } => Some(WaitSpec::Resume { id: id.clone() }),
        _ => None,
    }
}

/// 待つコマンド: 居る印を握る → 申し込み（`request`）→ 待ちのループ → 終わりの文 → 終了コード。
///
/// 居る印を握るのがいちばん先。同じ識別・同じ種類の待ちがすでに走っていれば、申し込みも
/// せず（状態を何も変えず）に失敗で返る。`request` は `--wait` の前半（申し込み・「止まった」の
/// 記録）。申し込み（`merge`・`request`）が当てはまらなければ待ちに入らず、残っていた自分の
/// 待ちの記録を同じ排他の中で消して、標準エラーへ `not applied: <訳>` を出して 3 で終わる
/// （始め直した待ちの同じ申し込みは当てはまるので、元の申し込みを引き継ぐ）。`stopped --wait`
/// だけは断らず、停止要請中でなければ再開の待ちだけになる。その時点でもう終わっていれば
/// （番が来ている・作業中に戻っている・停止要請が出し直されている・記録が無い）、待ちの記録を
/// 置かずに直ちに終わる。`request` の無い見張りと `resume` は、登録（見張りは参加を兼ねる）
/// からループに任せる（停止要請中の参加者の `resume` は、最初の判定で直ちに終わる）。
///
/// 待っている間は何も出さない。出すのは終わりの文だけ: 番・停止要請・再開・停止要請の出し直しは
/// 標準出力で 0、「消えた」は標準エラーへ `gone: <訳>` で 3。見張りの終わりには、停止要請の
/// 理由（内容は日本語になりうる）を読める読み物の道筋を 2 行目に添える。
fn waiting(
    home: Home,
    name: &'static str,
    spec: &WaitSpec,
    request: Option<&plan::Command>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, WatchError> {
    let (id, kind) = (spec.id(), spec.kind());
    let sign = home.alive_path(id, kind);
    let reading = home.status_path();
    let store = store::open(home, name)?;
    let held = match hold(&sign) {
        Ok(Some(held)) => Ok(held),
        Ok(None) => Err(WatchError::AlreadyRunning {
            id: id.to_owned(),
            kind: kind.as_str(),
        }),
        Err(failure) => Err(WatchError::io("hold the presence mark", &failure)),
    }
    .inspect_err(|failure| tracing::error!(error = %failure, "[cli] presence mark not held"))?;
    let early = match request {
        Some(command) => store.with_state(|state, now, alive| {
            let mut applied = plan::apply(state, command, Some(id), now, alive);
            // 当てはまらなかった申し込みは待たない（`stopped --wait` のほか）。
            let end = match applied.verdict {
                Verdict::NotApplied(why) if !matches!(command, plan::Command::Stopped { .. }) => {
                    Some(Err(why))
                }
                _ => wait::judge(spec, Some(state)).map(Ok),
            };
            if end.is_some() {
                // 殺された前の待ち（同じ識別・同じ種類）の記録を、答えを決めたのと同じ排他の中で
                // 消す。終わりの 1 行は 1 回目の判定のまま。
                let unregister = plan::Command::UnregisterWait {
                    id: id.to_owned(),
                    kind,
                };
                let cleanup = plan::apply(state, &unregister, Some(id), now, alive);
                applied.changed |= cleanup.changed;
                applied.events.extend(cleanup.events);
            }
            (applied, end)
        })?,
        None => None,
    };
    let end = match early {
        Some(Ok(end)) => end,
        Some(Err(why)) => {
            say(err, "stderr", &format!("not applied: {why}"))?;
            return Ok(Outcome::NotApplied);
        }
        None => wait::run(spec, &store, held)?,
    };
    match end {
        WaitEnd::Done(mut line) => {
            if kind == WaitKind::Watch {
                line += "\ndetails: ";
                line += &escape_path(&reading.to_string_lossy());
            }
            say(out, "stdout", &line)?;
            Ok(Outcome::Done)
        }
        WaitEnd::Gone(why) => {
            say(err, "stderr", &format!("gone: {why}"))?;
            Ok(Outcome::NotApplied)
        }
    }
}

/// 状態を変える 1 回: 口を開く → 排他の中で判断に通す → 結果の文 → 終了コード。
///
/// 呼んだ識別は、コマンドが名指す識別（その識別は同じ呼び出しでは回収されない）。
/// 「当てはまらなかった」は標準エラーへ 1 行出して 3（そのときも、回収の分は書かれている）。
fn change(
    home: Home,
    name: &'static str,
    command: &plan::Command,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<Outcome, WatchError> {
    let store = store::open(home, name)?;
    let (verdict, line) = store.with_state(|state, now, alive| {
        let applied = plan::apply(state, command, subject(command), now, alive);
        let line = result_line(name, command, state, &applied.events);
        let told = (applied.verdict, line);
        (applied, told)
    })?;
    match verdict {
        Verdict::Applied => {
            say(out, "stdout", &line)?;
            Ok(Outcome::Done)
        }
        Verdict::NotApplied(why) => {
            say(err, "stderr", &format!("not applied: {why}"))?;
            Ok(Outcome::NotApplied)
        }
    }
}

/// コマンドが名指す識別（`tick` と、全員宛ての `unstop` には無い）。
fn subject(command: &plan::Command) -> Option<&str> {
    use plan::Command as C;
    match command {
        C::Watch { id, .. }
        | C::Merge { id, .. }
        | C::Merged { id, .. }
        | C::Request { id, .. }
        | C::Return { id, .. }
        | C::LoadRunning { id, .. }
        | C::Stopped { id }
        | C::Cancel { id }
        | C::Leave { id }
        | C::UnregisterWait { id, .. } => Some(id),
        C::RegisterWait { record } => Some(&record.id),
        C::Unstop { id } => id.as_deref(),
        C::Tick | C::ClearRepo { .. } => None,
    }
}

/// 当てはまった呼び出しの結果の文（ASCII・末尾に改行なし）。`state` は変えた後の状態。
fn result_line(
    name: &'static str,
    command: &plan::Command,
    state: &State,
    events: &[Event],
) -> String {
    use plan::Command as C;
    match command {
        C::Merge { id, repo, .. } => {
            let wait = WaitSpec::Merge {
                id: id.clone(),
                repo: repo.clone(),
            };
            granted(&wait, state).unwrap_or_else(|| {
                let queue = state.merge.get(repo).map_or(&[][..], |desk| &desk.queue);
                let ahead = plan::merge_order(queue)
                    .into_iter()
                    .take_while(|&nth| queue[nth].id != *id)
                    .count();
                format!("queued merge repo={repo} pos={}", ahead + 1)
            })
        }
        C::Request { id, desks, .. } => granted(&WaitSpec::Request { id: id.clone() }, state)
            .unwrap_or_else(|| {
                let ahead = plan::request_order(&state.requests)
                    .into_iter()
                    .take_while(|&nth| state.requests[nth].id != *id)
                    .count();
                format!("queued {} pos={}", desks.as_str(), ahead + 1)
            }),
        // 空けた机 1 つに 1 行（1 つの識別が 2 つのリポジトリの机を持っていれば 2 行）。
        C::Merged { .. } => {
            let lines = events.iter().filter_map(|event| match event {
                Event::Merged { repo, .. } => Some(format!("merged repo={}", escape_path(repo))),
                _ => None,
            });
            lines.collect::<Vec<_>>().join("\n")
        }
        C::LoadRunning { .. } => "recorded running load".to_owned(),
        C::Return { .. } => {
            let returned = events.iter().find_map(|event| match event {
                Event::DeskReturned { desks, .. } => Some(desks.as_str()),
                _ => None,
            });
            format!("returned {}", returned.unwrap_or(status::NONE))
        }
        C::Unstop { .. } => {
            let unstopped = |event: &&Event| matches!(event, Event::Unstopped { .. });
            format!("unstopped n={}", events.iter().filter(unstopped).count())
        }
        C::Cancel { .. } => "cancelled".to_owned(),
        C::Leave { .. } => "left".to_owned(),
        // 外した者の出来事から組む（退避は行わない）。
        C::ClearRepo { repo } => {
            let left: Vec<String> = events
                .iter()
                .filter_map(|event| match event {
                    Event::Left { id, .. } => Some(id.clone()),
                    _ => None,
                })
                .collect();
            escape_path(&format!(
                "cleared repo={repo}; left: {}",
                status::id_list(&left)
            ))
        }
        // `stopped` と `tick` は、表の名前がそのまま結果の文（見張りの開始と待ちの記録の
        // 登録・抹消は、この手順へは来ない）。
        C::Stopped { .. }
        | C::Tick
        | C::Watch { .. }
        | C::RegisterWait { .. }
        | C::UnregisterWait { .. } => name.to_owned(),
    }
}

/// すでに番が来ていれば、待ちの終わりと同じ 1 行（綴りは [`wait::judge`] の 1 か所）。
fn granted(wait: &WaitSpec, state: &State) -> Option<String> {
    match wait::judge(wait, Some(state)) {
        Some(WaitEnd::Done(line)) => Some(line),
        _ => None,
    }
}

/// 状態の確認: 状態を変えずに読み、読み物を書き、ASCII の要約と読み物の道筋を出す。
/// 状態ファイルが無ければ、その旨だけを出す（状態ファイルも読み物も作らない）。
fn status(home: Home, name: &'static str, out: &mut dyn Write) -> Result<Outcome, WatchError> {
    let reading = home.status_path();
    let text = match store::open(home, name)?.summary()? {
        Some(summary) => {
            let reading = escape_path(&reading.to_string_lossy());
            format!("{summary}status: {reading}")
        }
        None => "no state file".to_owned(),
    };
    say(out, "stdout", &text)?;
    Ok(Outcome::Done)
}

/// 全部消す: 問い合わせをせずに直ちに行い、退避の道筋を 1 行で出す。
fn clear(home: Home, name: &'static str, out: &mut dyn Write) -> Result<Outcome, WatchError> {
    let backup = store::open(home, name)?.clear()?;
    let backup = backup.map_or(status::NONE.to_owned(), |path| {
        escape_path(&path.to_string_lossy())
    });
    say(out, "stdout", &format!("cleared; backup: {backup}"))?;
    Ok(Outcome::Done)
}

/// 1 行を書き出す。書けなかったことも黙って捨てない: 失敗にして、ログにも残す（ログの受け手が
/// 据わる前の、使い方の誤りと `--help` では、残す先がまだ無い）。
fn say(to: &mut dyn Write, op: &'static str, line: &str) -> Result<(), WatchError> {
    writeln!(to, "{line}").map_err(|err| {
        let failure = WatchError::io(op, &err);
        tracing::error!(error = %failure, "[cli] output failed");
        failure
    })
}

#[cfg(test)]
#[path = "cli_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "cli_commands_tests.rs"]
mod commands_tests;

#[cfg(test)]
#[path = "cli_wait_tests.rs"]
mod wait_tests;

#[cfg(test)]
#[path = "cli_desk_tests.rs"]
mod desk_tests;
