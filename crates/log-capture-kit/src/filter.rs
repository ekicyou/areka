//! `RUST_LOG` 相当の directive を実濾過した出力の捕捉（feature `env-filter` のときのみ）。
//!
//! 診断手順書が指定する directive（例: `"info,wintf::ecs::window_proc=debug"`）を
//! [`tracing_subscriber::EnvFilter`] でそのまま適用し、濾過を通過した整形済み出力だけを
//! 文字列で返す。「観測点が手順で有効化される水準に置かれている」ことを、定数の目視ではなく
//! **実濾過**で機械化するための窓口である。通過しなかった観測点は 1 文字も現れない。
//!
//! # 番兵の扱い（[`crate::capture`] との違い）
//!
//! 窓が本当に生きていたことは、[`crate::capture`] と同じく窓の内側で対照イベント（番兵）を
//! 1 件発火して確かめる。ただしこちらは呼出側の directive が濾過を握っているため、
//! 素のままでは番兵自身が濾過で消えてしまい「窓が死んでいる」と区別できない。そこで
//!
//! 1. 番兵の宛先だけを通す指令（`log_capture_kit::sentinel=trace`）を**内部で足す**。
//!    足すのは文字列連結ではなく [`EnvFilter::add_directive`] で、これは空の directive
//!    （`EnvFilter` の既定＝ERROR 水準）を壊さないため。文字列で連結すると、呼出側が
//!    空文字列を渡した場合だけ「解釈できた指令が番兵 1 件」となって既定 ERROR が付かず、
//!    呼出側の `error!` が黙って消える。
//! 2. 返す文字列から番兵の宛先を含む行を取り除く。取り除けなければ panic する
//!    （空の結果を静かに返して縮退しない）。
//!
//! 結果として、呼出側から見える文字列は番兵を導入する前と 1 バイトも変わらない。
//!
//! # 決定性の土台
//!
//! 発行点の interest がプロセス大域に焼き付く機序と、その対策（常駐 probe ＋窓内での
//! interest 再計算）は [`crate::probe`] の module doc に書いてある。ここでは同じ土台を
//! 使うだけで、濾過の有無は決定性に影響しない。

use std::sync::{Arc, Mutex};

use tracing_subscriber::EnvFilter;

use crate::capture::SENTINEL_TARGET;
use crate::probe::ensure_interest_probes;

/// 共有バッファへ書き出す [`std::io::Write`]（`fmt` subscriber の writer）。
#[derive(Clone)]
struct VecWriter(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("捕捉バッファは毒化していない")
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// `RUST_LOG` 相当の `directives` を実際に適用し、`f` の実行中に**現在のスレッド**で
/// 濾過を通過した tracing 出力を文字列で返す。
///
/// - 行の形は `tracing_subscriber::fmt` の既定（ANSI 無し）。1 イベントが 1 行になり、
///   レベル・宛先・本文・フィールドが載る。
/// - 戻り値に番兵行は含まれない（module doc 参照）。捕捉が働いていなければ panic する。
/// - 窓の外・他スレッドで発火したイベントは混入しない。
///
/// # Examples
///
/// ```rust
/// use log_capture_kit::capture_under_filter;
///
/// let out = capture_under_filter("info,areka_demo::quiet=warn", || {
///     tracing::info!(target: "areka_demo::loud", stage = "begin", "見える本文");
///     tracing::info!(target: "areka_demo::quiet", "濾過で消える本文");
/// });
///
/// // 通った側（対照）。これが無いと「消えた」の主張が空振りでも緑になる。
/// assert!(out.contains("見える本文"));
/// assert!(out.contains("stage=\"begin\""));
/// // 濾過された側。
/// assert!(!out.contains("濾過で消える本文"));
/// ```
///
/// # Panics
///
/// 窓の内側で対照イベント（番兵）を捕捉できなかった場合。
pub fn capture_under_filter(directives: &str, f: impl FnOnce()) -> String {
    ensure_interest_probes();

    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink = Arc::clone(&buf);

    // 呼出側の directive をまず素のまま解釈させ（空文字列の既定はここで決まる）、
    // そのうえで番兵の宛先だけを足す。番兵の指令は宛先が固定なので、呼出側の
    // どの宛先の判定にも影響しない。
    let sentinel_directive = format!("{SENTINEL_TARGET}=trace")
        .parse()
        .expect("番兵の指令は定数から組むので常に解釈できる");
    let filter = EnvFilter::new(directives).add_directive(sentinel_directive);

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(move || VecWriter(Arc::clone(&sink)))
        .finish();

    // `with_default` は内部で `Dispatch::new`（＝register_dispatch ＋全 callsite 再計算）を
    // 行うが、probe 常駐前に焼かれた `never` の掃き残しを窓の内側でもう一度潰す。
    tracing::subscriber::with_default(subscriber, || {
        tracing::callsite::rebuild_interest_cache();
        // 対照イベント（番兵）。窓が本当に生きているかを、同じ窓の内側で示す。
        tracing::trace!(target: SENTINEL_TARGET, "capture window is live");
        f();
    });

    let captured = String::from_utf8(buf.lock().expect("捕捉バッファは毒化していない").clone())
        .expect("UTF-8");

    strip_sentinel_lines(&captured)
}

/// 番兵の宛先を含む行を取り除く。取り除く行が 1 つも無ければ panic する。
///
/// 行の切り出しは [`str::split_inclusive`] で行うので、残す行は改行も含めて元のバイト列の
/// ままであり、末尾に改行の無い行も落ちない。
fn strip_sentinel_lines(captured: &str) -> String {
    let mut kept = String::with_capacity(captured.len());
    let mut removed = 0usize;
    for line in captured.split_inclusive('\n') {
        if line.contains(SENTINEL_TARGET) {
            removed += 1;
            continue;
        }
        kept.push_str(line);
    }

    assert!(
        removed > 0,
        "捕捉窓の対照イベント（{SENTINEL_TARGET}）を捕捉できなかった。\
         濾過を通した出力にも番兵行が無いため、この窓の捕捉結果は \
         「出なかった」ことの証拠にならない"
    );

    kept
}

#[cfg(test)]
#[path = "filter_tests.rs"]
mod tests;
