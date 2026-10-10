# 口パクの検体

spec `areka-P0-seriko-trigger-intervals`（要件 11.1）の試験用のサーフェス定義。`talk,数値`・`runonce`・`periodic,数値` を書いたサーフェスを持つ（同梱の検体にはどれも無い）。第三者の著作物は 0 件。

置いてあるのは `surfaces.txt` だけで、画像ファイルは無い。単独のシェルではなく、emo2 の `surfaces.txt` の末尾へ書き足して使う断片である。

## 絵の出どころ

同梱の検体 emo2（`vendors/sample_ghost/emo2.nar`）の `shell/master/` に既に在る絵を指す。指し方は 2 通り。

| 指し方 | 指しているもの | emo2 での定義 |
|---|---|---|
| element定義のファイル名 | `surface0.png` | `\0` の立ち絵 |
| element定義のファイル名 | `purple/2/mouthbase.png` | 口の土台 |
| pattern定義のサーフェス番号 | 1200・1202・1203 | 口（`purple/2/a.png`・`e.png`・`i.png`） |
| pattern定義のサーフェス番号 | 1600 | 紅（`purple/1/cheek.png`） |
| pattern定義のサーフェス番号 | 1700・1701 | キラリ（`purple/a/siitake.png`・`siitake_han.png`） |

番号で指す先のサーフェス（1200 ほか）は emo2 の `surfaces.txt` が定義している。このフォルダの `surfaces.txt` は定義し直さない。

## サーフェス（`surfaces.txt`）

番号は emo2（0・10・1000〜2210）と当たらないよう 9100 から。

| 番号 | 役 | animation |
|---|---|---|
| 9100 | 一番上に 3 語 | 0＝`talk,3`（1200 → 80 ms で 1202 → 80 ms で消す）・1＝`runonce`（1700 → 300 ms で消す）・2＝`periodic,2`（1600 → 500 ms で消す） |
| 9101 | 一番上は 3 語を持たず、element定義で部品 9102 を置く | なし |
| 9102 | 部品（9101 が置く） | 0＝`talk,2`（1203 → 100 ms で消す）・1＝`runonce`（1701 → 300 ms で消す） |
| 9103 | 採らない書き方。何も動かない | 0＝`talk,abc`・1＝`periodic,0`（どちらも数値が正の整数でない）・2＝`Talk,3`（大文字混じり） |
| 9104 | 採らない書き方。何も動かない | 0＝`runonce`（pattern定義が 1 つも無い） |

## 表を組むときの記録

`surfaces.txt` の行を足し引きしたら、下の件数と `surfaces.txt` の冒頭のコメントとテストの期待値を合わせて直す。

| 記録 | 件数 | 出どころ |
|---|---|---|
| `debug!`「引き金の語を採録」 | 5 | 9100 の animation 0・1・2、9102 の animation 0・1 |
| `warn!`「talk/periodic の数値が無効ゆえ非採録」 | 2 | 9103 の animation 0（元の綴り `talk,abc`）・animation 1（`periodic,0`） |
| `warn!`「コマ列が空のアニメは非採録」 | 1 | 9104 の animation 0 |

`warn!` はこの 3 件だけ。9103 の animation 2（`Talk,3`）は 3 語でない語として `debug!` だけを残す。

## 決定論のテスト

`crates/areka-seriko/src/actor_trigger_fixture_tests.rs` が、この `surfaces.txt` を本物の読み手で読み、表を組み、偽の時計のアクターへサーフェスの切り替え・文字・刻みを流して、出てくるコマと上の記録の件数を判定する。emo2 の `surfaces.txt` の末尾へ書き足した本文でも件数が同じこと、番号が emo2 と当たらないこと、指している絵が emo2 に全部在ることも同じファイルが判定する。

## 実機の確かめ

手順は spec の `design.md`「実機の確かめ」（`tasks.md` の 9.2）。要点だけ写す。

1. `cargo run -p sample-ghost-kit --bin nar-sample-path emo2` で emo2 を展開し、ワークツリーの `target\` の下に写しを作る（同梱の検体そのものと `target\` の外は書き替えない）。
2. 写しの `shell\master\surfaces.txt` の末尾へ、この `surfaces.txt` を丸ごと書き足す（1 行目の `charset,UTF-8` は残してよい。読み手は読み飛ばす）。
3. 有界の自動終了・走行ごとのプロファイル・`RUST_LOG=info,areka_seriko=debug,areka_emo_text=debug` で起こし、MCP の `sakurascript` で `\s[9100]` と台詞、`\s[9101]`、`\s[9103]`、`\s[9104]` ほかを送る。
4. 記録を検索して、上の表の件数と、`runonce`・`periodic`・`talk` の開始の時刻を判定する。
