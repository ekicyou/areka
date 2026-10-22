//! emo2 の 1 回の走りの共通の手続き（件 1 の試験とくり返しの試験が `#[path]` で引く）。
//!
//! 印の綴り・保険の自動終了と締切の値・`RUST_LOG`・LOAD の失敗の綴り・時間の 1 行・失敗の分けの規則は
//! このファイルだけが持つ。純粋な関数の試験は `common_tests.rs`。

use real_run_kit::{Ended, EndedBy, Launch, McpError, RealRun, Run, ToolReply};
use std::time::{Duration, Instant};

/// 保険の自動終了。印までの最大（混んだ時間 6.82 秒）＋別れの台詞（約 22 秒）＝約 29 秒に余りを足した値（1.5）。
pub const SAFETY_EXIT: Duration = Duration::from_secs(45);
/// 締切（待ちごと）。
pub const DEADLINE: Duration = Duration::from_secs(120);
// 保険の自動終了が締切より先に来ないと、締切がハングの判定で先に働く（1.5）。
const _: () = assert!(SAFETY_EXIT.as_secs() < DEADLINE.as_secs());

pub const WIRE: &str = "emo2-boot: 実 sink 結線が成立しました（wire 成立）";
pub const ATTACH: &str = "emo2 attach: 装着計画を実行";
pub const WRAP: &str = "wrap=BudouxWordWrap";
/// 時間の切り分けにだけ使う（待たない）。
pub const BOOT_TALK: &str = "event=\"boot_talk\"";
pub const LOAD_RETURNED_FALSE: &str = "[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse";
/// `info,kanade=trace` に、主のモニタの DPI の行の出所の debug を足したもの。
pub const RUST_LOG: &str = "info,kanade=trace,wintf::ecs::layout::systems::monitor_systems=debug";
/// 子の `__COMPAT_LAYER` にだけ渡す値を受ける環境変数（100% 相当の確かめ）。
pub const COMPAT_LAYER_ENV: &str = "AREKA_EMO2_COMPAT_LAYER";
/// 比べの条件の待つ長さ（固定）。
pub const DROP_WAIT: Duration = Duration::from_secs(10);

/// 主のモニタの DPI の行の綴り。
const MONITOR_LINE: &str = "[initialize_layout_root] Creating Monitor entity";

/// 起動から各印が見えるまで・閉じる知らせから終わるまでの時間。
#[derive(Debug, Clone)]
pub struct Times {
    pub wire: Duration,
    pub attach: Duration,
    pub wrap: Duration,
    pub close: Duration,
    /// 記録の時刻で読んだ `BOOT_TALK` の行から `WRAP` の行まで。どちらかの行が無ければ None。
    pub talk_to_wrap: Option<Duration>,
    /// 記録の時刻で読んだ `ATTACH` の行から `BOOT_TALK` の行までのミリ秒。正なら最初の台詞が後、負なら装着が後
    /// （SHIORI の読み込みは装着と並んで走るので、どちらが後かは回ごとに変わる）。どちらかの行が無ければ None。
    pub attach_to_talk_ms: Option<i64>,
    /// 主のモニタの DPI。行が無ければ None。
    pub primary_dpi: Option<u32>,
}

impl Times {
    /// ASCII の 1 行。時間はミリ秒の整数（秒へ丸めない）、無い値は `none`。例:
    /// `emo2: wire_ms=812 attach_ms=1540 wrap_ms=6260 close_ms=22104 talk_to_wrap_ms=310 attach_to_talk_ms=4410 primary_dpi=192`
    pub fn line(&self, who: &str) -> String {
        fn or_none(v: Option<impl ToString>) -> String {
            v.map_or_else(|| "none".to_string(), |v| v.to_string())
        }
        format!(
            "{who}: wire_ms={} attach_ms={} wrap_ms={} close_ms={} talk_to_wrap_ms={} attach_to_talk_ms={} primary_dpi={}",
            self.wire.as_millis(),
            self.attach.as_millis(),
            self.wrap.as_millis(),
            self.close.as_millis(),
            or_none(self.talk_to_wrap.map(|d| d.as_millis())),
            or_none(self.attach_to_talk_ms),
            or_none(self.primary_dpi),
        )
    }
}

/// 比べの条件（`AREKA_EMO2_REPEAT_DROP`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Basic,
    Gap,
    CopyWait,
    NoMcp,
}

impl Condition {
    /// 空・空白だけ＝Basic。知らない値は Err（理由の文）。
    pub fn parse(value: &str) -> Result<Condition, String> {
        match value.trim() {
            "" => Ok(Condition::Basic),
            "gap" => Ok(Condition::Gap),
            "copy-wait" => Ok(Condition::CopyWait),
            "no-mcp" => Ok(Condition::NoMcp),
            other => Err(format!(
                "AREKA_EMO2_REPEAT_DROP={other:?} は知らない値です（受ける値: 空・gap・copy-wait・no-mcp）"
            )),
        }
    }
}

/// 1 回の走りの結果の分け。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    LoadFailed,
    OtherFailed,
}

/// 出力の中の `[initialize_layout_root] Creating Monitor entity` の行のうち `is_primary=true` の `dpi=` の値。
pub fn primary_dpi(output: &str) -> Option<u32> {
    output
        .lines()
        .filter(|l| l.contains(MONITOR_LINE))
        .find(|l| l.split_whitespace().any(|t| t == "is_primary=true"))?
        .split_whitespace()
        .find_map(|t| t.strip_prefix("dpi=")?.parse().ok())
}

/// 2 つの綴りを含む最初の行どうしの、行頭の記録の時刻（`…THH:MM:SS.ffffff…`）の差のミリ秒（`to` − `from`。
/// `to` が先なら負）。読めなければ None。
pub fn log_gap_ms(output: &str, from: &str, to: &str) -> Option<i64> {
    let at = |needle: &str| {
        output
            .lines()
            .find(|l| l.contains(needle))
            .and_then(line_micros)
    };
    Some((at(to)? - at(from)?) / 1000)
}

/// 行頭の記録の時刻を、その日の 0 時からのマイクロ秒で読む。
/// shortcut: 日付を見ないので、日付をまたいだ 2 行の差は狂う。1 回の走りは数十秒なので、真夜中に回すときだけ問題。
fn line_micros(line: &str) -> Option<i64> {
    let stamp = line.split_whitespace().next()?;
    let (_, time) = stamp.split_once('T')?;
    let mut hms = time.splitn(3, ':');
    let h: i64 = hms.next()?.parse().ok()?;
    let m: i64 = hms.next()?.parse().ok()?;
    let rest = hms.next()?;
    let digits = rest.trim_end_matches(|c: char| !c.is_ascii_digit());
    let (s, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let s: i64 = s.parse().ok()?;
    // 小数はマイクロ秒の 6 桁へ揃える（短ければ 0 を足し、長ければ切る）。
    let frac: String = frac.chars().chain(std::iter::repeat('0')).take(6).collect();
    let us: i64 = frac.parse().ok()?;
    Some(((h * 60 + m) * 60 + s) * 1_000_000 + us)
}

/// 落ちた回の出力が「LOAD の失敗」か（`LOAD_RETURNED_FALSE` を含むか）。
pub fn is_load_returned_false(output: &str) -> bool {
    output.contains(LOAD_RETURNED_FALSE)
}

/// 待つ間の MCP の呼び出しの間隔。
const MCP_GAP: Duration = Duration::from_millis(500);

/// 新しい写しで emo2 を起こす（`Launch::Sample { emo2, emo2-kakukaku }`・`SAFETY_EXIT`・`DEADLINE`・`RUST_LOG`）。
/// `COMPAT_LAYER_ENV` が空でなければ子にだけ `__COMPAT_LAYER` を渡す。`wait_after_copy` があれば
/// `before_spawn` の中でその時間だけ眠る。返す `Instant` は起こした直後。
pub fn spawn_emo2(wait_after_copy: Option<Duration>) -> (Run, Instant) {
    let mut kit = RealRun::new(env!("CARGO_BIN_EXE_areka"))
        .launch(Launch::Sample {
            name: "emo2".into(),
            balloon: Some("emo2-kakukaku".into()),
        })
        .rust_log(RUST_LOG)
        .safety_exit(SAFETY_EXIT)
        .deadline(DEADLINE);
    if let Ok(v) = std::env::var(COMPAT_LAYER_ENV)
        && !v.trim().is_empty()
    {
        kit = kit.env("__COMPAT_LAYER", v.trim());
    }
    if let Some(wait) = wait_after_copy {
        kit = kit.before_spawn(move |_| std::thread::sleep(wait));
    }
    let run = kit.spawn();
    (run, Instant::now())
}

/// 印を順に待ち、利用者と同じ閉じ方で閉じ、終わり方と終了コードを確かめる。`run` はこの中で捨てる
/// （panic の途中で捨てられれば、窓口が証拠を残す）。
/// `mcp_while_waiting` が真なら、`WRAP` が見えるまで `dump_surface`・`dump_balloon` を呼び続ける。
///
/// # Panics
/// 印の前に子が終わった（終了コードと終わり方）・締切（窓口の文）・終わり方が利用者と同じ閉じ方でない
/// （保険の自動終了ならその旨）・終了コードが 0 でない・MCP の呼び出しが `ListenFailed`／`Unreachable` で失敗した
/// （`NG:` の答えは失敗にしない）。
pub fn boot_and_close(mut run: Run, started: Instant, mcp_while_waiting: bool) -> Times {
    if mcp_while_waiting {
        call_mcp_until_wrap(&mut run, started);
    }
    let mut seen = [Duration::ZERO; 3];
    for (at, needle) in seen.iter_mut().zip([WIRE, ATTACH, WRAP]) {
        if run.wait_for(needle).is_err() {
            let ended = run.wait_exit();
            panic!(
                "印 {needle:?} の前に子が終わった（終了コード {:?}・終わり方 {:?}）\n--- child output ---\n{}",
                ended.status.code(),
                ended.by,
                run.output()
            );
        }
        *at = started.elapsed();
    }
    let close_at = Instant::now();
    let ended = run.close();
    let close = close_at.elapsed();
    let output = run.output();
    if let Some(why) = ending_failure(&ended) {
        panic!("{why}\n--- child output ---\n{output}");
    }
    Times {
        wire: seen[0],
        attach: seen[1],
        wrap: seen[2],
        close,
        talk_to_wrap: log_gap_ms(&output, BOOT_TALK, WRAP)
            .and_then(|ms| u64::try_from(ms).ok())
            .map(Duration::from_millis),
        attach_to_talk_ms: log_gap_ms(&output, ATTACH, BOOT_TALK),
        primary_dpi: primary_dpi(&output),
    }
}

/// `WRAP` が見える・子が終わる・締切を過ぎるまで、面とバルーンを引数なしで呼び続ける（取り合いの相手としての負荷）。
fn call_mcp_until_wrap(run: &mut Run, started: Instant) {
    while started.elapsed() < DEADLINE && !run.output().contains(WRAP) {
        for tool in ["dump_surface", "dump_balloon"] {
            match keep_calling(run.call(tool, "{}")) {
                Ok(true) => {}
                // 子が終わった: 後の待ちが終わり方を報告する。
                Ok(false) => return,
                Err(e) => panic!(
                    "{tool} を呼べませんでした: {e}\n--- child output ---\n{}",
                    run.output()
                ),
            }
        }
        std::thread::sleep(MCP_GAP);
    }
}

/// 待つ間の呼び出しの答えの扱い。答えが返れば（`NG:` の答えも）続ける＝`Ok(true)`、子が終わったら抜ける＝`Ok(false)`、
/// 届かないときだけ `Err`。
fn keep_calling(reply: Result<ToolReply, McpError>) -> Result<bool, McpError> {
    match reply {
        Ok(_) => Ok(true),
        Err(McpError::ChildExited(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// 終わり方が利用者と同じ閉じ方で、終了コードが 0 なら None。そうでなければ理由の文。
fn ending_failure(ended: &Ended) -> Option<String> {
    let code = ended.status.code();
    match &ended.by {
        EndedBy::UserClose if ended.status.success() => None,
        EndedBy::UserClose => Some(format!(
            "利用者と同じ閉じ方で終わったが終了コードが 0 でない（終了コード {code:?}）"
        )),
        EndedBy::SafetyExit => Some(format!(
            "閉じ方が働かず、保険の自動終了（{} 秒）で終わった（終了コード {code:?}）",
            SAFETY_EXIT.as_secs()
        )),
        other => Some(format!(
            "利用者と同じ閉じ方で終わらなかった（終わり方 {other:?}・終了コード {code:?}）"
        )),
    }
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
