//! 検体の絶対パスを印字するコマンド（spec: `areka-P0-nar-install` 要件 1.9・
//! `areka-P0-sample-url-fetch` 要件 2.9・3.7）。
//!
//! # 呼び方
//!
//! ```text
//! cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2
//! cargo run -p sample-ghost-kit --bin nar-sample-path -- --fetch-list
//! ```
//!
//! # 出す形
//!
//! 標準出力に 1 行 1 組の `key=value`。値は Windows の絶対パスそのままで、末尾に区切り
//! 記号は付けない。
//!
//! ```text
//! root=C:\...\target\nar-samples\manual\emo2
//! folder=C:\...\target\nar-samples\manual\emo2\ghost\emo2
//! balloon.emo2-kakukaku=C:\...\target\nar-samples\manual\emo2\ghost\emo2\balloon\emo2-kakukaku
//! ```
//!
//! # 終了コード
//!
//! - 0: 成功。
//! - 3: URL の行の書庫かハッシュのファイルが置き場に無い・ハッシュのファイルの形が違う・
//!   書庫と合わない。理由は ASCII の英語で標準エラーへ出る（直し方は `tools/sample-fetch.ps1`）。
//! - 2: ほかの失敗（未登録の名前・展開や入出力の失敗・置き場が決まらない）。
//!
//! どの失敗でも標準出力へは 1 行も出さない。読み手（`tools/perf/`・`tools/package.ps1`）は
//! `folder=` の 1 行を読み、0 以外なら止まるので、これが「黙って壊れたパスを渡さない」の
//! 担保になる。
//!
//! # `--fetch-list`
//!
//! 取り寄せの段（`tools/sample-fetch.ps1`）へ登記を渡す。URL の行ごとに 1 行、`SAMPLES` の
//! 順で、タブ区切りの 5 列を出す。
//!
//! ```text
//! <名前>\t<URL>\t<ハッシュのファイルの URL>\t<置き場の書庫の絶対パス>\t<置き場のハッシュのファイルの絶対パス>
//! ```
//!
//! URL の行が 0 なら何も出さずに 0。置き場が決まらなければ標準出力は空で 2。置き場は
//! `CARGO_TARGET_DIR`（無ければこの実行体の祖先の `target`）から決まるので、同じ環境の
//! テストの窓口と同じ所を指す。
//!
//! # 配る先
//!
//! 実機は印字された絶対パスを、このコマンドが終わった**後で**使う。だから使い捨ての複製
//! （破棄で消える）ではなく、プロセスが終わっても残る `manual/<名>/` へ配る。呼ぶたびに
//! 作り直すので、2 周すれば 2 回とも起動記録の無い根で始まる（要件 9.7）。
//!
//! 作り直しは同じ `manual/<名>/` を**消してから**行う。だから 2 つの端末から同時にこの
//! コマンドを呼ぶと、後から呼んだ側が先の走行の使っている木を消してしまう。実機を 2 周
//! するときは、1 周が終わってから次を呼ぶこと。
//!
//! # 検査の置き場
//!
//! 判定は `tests/nar_sample_path_test.rs` が**この実行体を起こして**行う。中の関数を直に
//! 呼ぶ兄弟テストにしないのは、出力の形も終了コードも流れの選び分けも `main` の配線その
//! ものだからである（cargo が実行体を組んで場所を教えるのも結合テストのときだけ）。

use std::io::Write;

use sample_ghost_kit::{SAMPLES, SampleError, SampleSource};

/// 実機走行の絶対パスか、`--fetch-list` の登記を印字する。
///
/// 引数が無いときは空の検体名として扱う——[`sample_ghost_kit::manual_paths`] が既知の
/// 名前の一覧を添えて断るので、打ち間違いも指定忘れも同じ 1 本の道を通る。
fn main() {
    let name = std::env::args().nth(1).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = if name == "--fetch-list" {
        fetch_list(&mut out, &mut err)
    } else {
        run(&name, &mut out, &mut err)
    };
    drop(out);
    drop(err);
    std::process::exit(code);
}

/// 検体名 1 つを処理して終了コードを返す（0 が成功・3 が URL の行の失敗・2 がほかの失敗）。
///
/// 標準出力へ書けなかったときも黙って 0 で終わらない（失敗を握り潰さない）。
fn run(name: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match sample_ghost_kit::manual_paths(name) {
        Ok(printed) => {
            let lines: Vec<String> = printed
                .into_iter()
                .map(|(key, path)| format!("{key}={}", path.display()))
                .collect();
            write_all(&lines, out, err)
        }
        Err(refused) => refuse(&refused, err),
    }
}

/// URL の行ごとに 5 列の 1 行を `SAMPLES` の順に出す。置き場が決まらなければ何も出さずに 2。
///
/// 全部の行を組み終えてから書くので、途中で失敗しても標準出力は空のまま。
fn fetch_list(out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut lines = Vec::new();
    for sample in SAMPLES {
        let SampleSource::Fetched { url } = sample.source else {
            continue;
        };
        let (nar, checksum) = match (
            sample_ghost_kit::fetched_archive_path(sample),
            sample_ghost_kit::fetched_checksum_path(sample),
        ) {
            (Ok(Some(nar)), Ok(Some(checksum))) => (nar, checksum),
            (Err(refused), _) | (_, Err(refused)) => return refuse(&refused, err),
            _ => unreachable!("URL の行には置き場が在る"),
        };
        lines.push(format!(
            "{}\t{url}\t{}\t{}\t{}",
            sample.name,
            sample_ghost_kit::checksum_url(url),
            nar.display(),
            checksum.display()
        ));
    }
    write_all(&lines, out, err)
}

/// 失敗を標準エラーへ出し、終了コードを返す（URL の行の 3 種は 3・ほかは 2）。
fn refuse(refused: &SampleError, err: &mut dyn Write) -> i32 {
    let _ = writeln!(err, "{refused}");
    match refused {
        SampleError::FetchedNarMissing { .. }
        | SampleError::FetchedChecksumInvalid { .. }
        | SampleError::FetchedNarMismatch { .. } => 3,
        _ => 2,
    }
}

/// 行を標準出力へ書く。書けなければ標準エラーへ出して 2。
fn write_all(lines: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    for line in lines {
        if let Err(broken) = writeln!(out, "{line}") {
            let _ = writeln!(err, "nar-sample-path: 標準出力へ書けない: {broken}");
            return 2;
        }
    }
    0
}
