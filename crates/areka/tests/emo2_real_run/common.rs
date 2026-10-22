//! emo2 の 1 回の走りの共通の手続き（件 1 の試験とくり返しの試験が `#[path]` で引く）。
//!
//! 印の綴り・保険の自動終了と締切の値・`RUST_LOG`・LOAD の失敗の綴り・時間の 1 行・失敗の分けの規則は
//! このファイルだけが持つ。純粋な関数の試験は `common_tests.rs`。

use std::time::Duration;

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

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
