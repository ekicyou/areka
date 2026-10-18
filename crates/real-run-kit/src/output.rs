//! 出力のファイルの読み直しと文字列の待ち・失敗の文の組み立て（要件 3.2〜3.5・3.7）。
//!
//! 子の出力はファイルへ直に流れる（D-9）。待ちはファイルごとに読んだ位置を持ち、増えた分だけを
//! 読んで探す（`kanade=trace` を開けた数 MB の走りでも、見回りのたびに全文を読み直さない）。
//! 眠るのは観測を読み直す見回りの間隔だけで、決まった時間の眠りは持たない（3.7）。

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

/// 締切切れの文に添える、各ファイルの末尾の長さ。
pub(crate) const TAIL_BYTES: usize = 64 * 1024;

/// 子が待ちの途中で終わった（3.5・4.8）。締切切れと見分けられる値。
///
/// 下の段の close・mcp も使うので、ここで定める。run とクレートの入口が再輸出する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildExited {
    /// 子の終わり方（終了コードは `status.code()`）。
    pub status: ExitStatus,
}

/// 文字列が現れる前に待ちが終わった訳。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaitEnd {
    /// 子が先に終わった（3.5）。
    ChildExited(ChildExited),
    /// 締切を過ぎた（呼び手が panic・`Unreachable` などへ写す＝3.4・4.7）。
    Deadline,
}

/// 1 つのファイルの読んだ位置と、境目をまたぐ一致のために重ねる前回の末尾。
pub(crate) struct Tail {
    path: PathBuf,
    offset: u64,
    carry: Vec<u8>,
}

impl Tail {
    pub(crate) fn new(path: PathBuf) -> Self {
        Tail {
            path,
            offset: 0,
            carry: Vec::new(),
        }
    }

    /// 増えた分だけを読み、`needles` のどれかがあればその番号を返す（バイト列で照合）。
    ///
    /// 前回の末尾の「最も長い文字列のバイト数 − 1」を重ねて探すので、追記の境目（多バイトの字の
    /// 途中を含む）をまたいだ文字列も拾う。ファイルがまだ無い・読めないは「増えていない」。
    pub(crate) fn scan(&mut self, needles: &[&str]) -> Option<usize> {
        let mut buf = std::mem::take(&mut self.carry);
        let before = buf.len();
        if let Ok(mut f) = File::open(&self.path)
            && f.seek(SeekFrom::Start(self.offset)).is_ok()
        {
            // 途中で読めなくなっても、読めた分は位置へ足す（同じ分を 2 度読まない）。
            let _ = f.read_to_end(&mut buf);
        }
        self.offset += (buf.len() - before) as u64;
        let found = needles.iter().position(|n| contains(&buf, n.as_bytes()));
        let keep = needles
            .iter()
            .map(|n| n.len())
            .max()
            .unwrap_or(0)
            .saturating_sub(1);
        self.carry = buf.split_off(buf.len().saturating_sub(keep));
        found
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || hay.windows(needle.len()).any(|w| w == needle)
}

/// 2 つの出力のファイルに `needles` のどれかが現れるまで待ち、その番号を返す（3.2・3.5・3.7）。
///
/// 1 回ごとに ⑴ 増えた分に文字列があればその番号 ⑵ 子が終わっていれば、もう 1 度だけ増えた分を
/// 読んでから `ChildExited` ⑶ 締切を過ぎていれば `Deadline` ⑷ `poll` だけ待って ⑴ へ。
pub(crate) fn wait_any(
    files: [&Path; 2],
    needles: &[&str],
    child: &mut Child,
    deadline: Instant,
    poll: Duration,
) -> Result<usize, WaitEnd> {
    let mut tails = files.map(|p| Tail::new(p.to_path_buf()));
    let scan = |tails: &mut [Tail; 2]| tails.iter_mut().find_map(|t| t.scan(needles));
    loop {
        if let Some(i) = scan(&mut tails) {
            return Ok(i);
        }
        // 終わりを見られないのは取っ手の故障。終わったとは言えないので見回りを続け、締切で終える。
        if let Ok(Some(status)) = child.try_wait() {
            // 終わる直前に書かれた分を取りこぼさない。
            return scan(&mut tails).ok_or(WaitEnd::ChildExited(ChildExited { status }));
        }
        if Instant::now() >= deadline {
            return Err(WaitEnd::Deadline);
        }
        std::thread::sleep(poll);
    }
}

/// 締切切れの文（3.4・5.4 で共用）: 待っていた文字列・待った時間・2 つのファイルの場所・
/// 各ファイルの末尾 64 KiB（全文はファイルにある旨を添える）。末尾は全文から取る。
pub(crate) fn failure_text(needles: &[&str], waited: Duration, files: [&Path; 2]) -> String {
    let mut text = format!("締切切れ: {needles:?} を {waited:?} 待ったが現れなかった\n");
    for path in files {
        let bytes = std::fs::read(path).unwrap_or_default();
        let tail = &bytes[bytes.len().saturating_sub(TAIL_BYTES)..];
        text.push_str(&format!(
            "--- {} の末尾 {} バイト（全 {} バイト・全文はファイルにある） ---\n{}\n",
            path.display(),
            tail.len(),
            bytes.len(),
            String::from_utf8_lossy(tail),
        ));
    }
    text
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
