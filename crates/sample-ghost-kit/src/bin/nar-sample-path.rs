//! 検体の絶対パスを印字するコマンド（spec: `areka-P0-nar-install` 要件 1.9）。
//!
//! # 呼び方
//!
//! ```text
//! cargo run -p sample-ghost-kit --bin nar-sample-path -- emo2
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
//! balloon.emo2-kakukaku=C:\...\target\nar-samples\manual\emo2\balloon\emo2-kakukaku
//! ```
//!
//! 未登録の名前は理由を標準エラーに出して**終了コード 2**で終わる。読み手（`tools/perf/`
//! の 2 本）は `folder=` の 1 行を読むので、失敗のときに標準出力へ 1 行も出さないことが
//! 「黙って壊れたパスを渡さない」の担保になる。
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

/// 実機走行の絶対パスを印字する。
///
/// 引数が無いときは空の検体名として扱う——[`sample_ghost_kit::manual_paths`] が既知の
/// 名前の一覧を添えて断るので、打ち間違いも指定忘れも同じ 1 本の道を通る。
fn main() {
    let name = std::env::args().nth(1).unwrap_or_default();
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = run(&name, &mut out, &mut err);
    drop(out);
    drop(err);
    std::process::exit(code);
}

/// 検体名 1 つを処理して終了コードを返す（0 が成功・2 が失敗）。
///
/// 書き出し先を引数で受けるので、兄弟テストはプロセスを起こさずに両方の流れを読める。
/// 標準出力へ書けなかったときも黙って 0 で終わらない（失敗を握り潰さない）。
fn run(name: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match sample_ghost_kit::manual_paths(name) {
        Ok(printed) => {
            for (key, path) in printed {
                if let Err(broken) = writeln!(out, "{key}={}", path.display()) {
                    let _ = writeln!(err, "nar-sample-path: 標準出力へ書けない: {broken}");
                    return 2;
                }
            }
            0
        }
        Err(refused) => {
            let _ = writeln!(err, "{refused}");
            2
        }
    }
}
