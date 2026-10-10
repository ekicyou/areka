# 口パクの検体

spec `areka-P0-seriko-trigger-intervals`（要件 11.1）の試験用のサーフェス定義。`talk,数値`・`runonce`・`periodic,数値` を書いたサーフェスを持つ（同梱の検体にはどれも無い）。第三者の著作物は 0 件。

置いてあるのは `surfaces.txt` だけで、画像ファイルは無い。単独のシェルではなく、emo2 の `surfaces.txt` の末尾へ書き足して使う断片である。

## 絵の出どころ

同梱の検体 emo2（`vendors/sample_ghost/emo2.nar`）の `shell/master/` に既に在る絵を指す。指し方は 2 通り。

| 指し方 | 指しているもの | emo2 での定義 |
|---|---|---|
| element定義のファイル名 | `purple/0/base1.png` | 体（顔の中身は無い）。surface1000 の腕「伸び」（1100）の絵 |
| element定義のファイル名 | `purple/4/normal.png` | 目「通常」（1302）の絵 |
| element定義のファイル名 | `purple/3/normal.png` | 眉「通常」（1500）の絵 |
| element定義のファイル名 | `purple/a/ribbon.png` | 髪飾り「リボン」（1800）の絵 |
| element定義のファイル名 | `purple/2/niko.png` | 閉じた口「にこっ」（1207）の絵 |
| pattern定義のサーフェス番号 | 1200・1202・1203 | 口（`purple/2/a.png`・`e.png`・`i.png`） |
| pattern定義のサーフェス番号 | 1600 | 紅（`purple/1/cheek.png`） |
| pattern定義のサーフェス番号 | 1700・1701 | キラリ（`purple/a/siitake.png`・`siitake_han.png`） |

番号で指す先のサーフェス（1200 ほか）は emo2 の `surfaces.txt` が定義している。このフォルダの `surfaces.txt` は定義し直さない。

立ち絵は、emo2 の surface1000 が `descript.txt` の既定の着せ替えで出す 5 枚（体・口・目・眉・髪飾り）を element定義で重ねたもの。動いていないときの絵は surface1000 の既定の姿と同じになる（まばたきはしない）。体を `surface0.png`（`\0` の 1 枚絵）にしないのは、口・紅・キラリの絵が surface1000 の立ち絵（382×547）の顔の位置に合わせて描かれていて、別の構図の `surface0.png`（434×687）に重ねると顔から外れるからである。

element定義の並びは 体 → 目 → 眉 → 髪飾り → 口（element4）。9101 は口の代わりに部品 9102 を element4 に置く。部品の animation が重ねる絵（キラリ 1701 は目の上に出る）は部品の element定義の位置に入るので、部品を目より後ろに置かないと目の絵に隠れる。

開いた口の絵は閉じた口（`niko.png`）の上に重なる。`a.png`・`e.png` は閉じた口を全部覆い、`i.png` の下からもほとんどはみ出さないので、口の animation が出ている間に口が 2 つには見えない（実機の絵で確かめた）。emo2 に在る `mouthbase.png`（口のまわりを肌の色で塗る絵）は使わない。

## サーフェス（`surfaces.txt`）

番号は emo2（0・10・1000〜2210）と当たらないよう 9100 から。

| 番号 | 役 | element定義 | animation |
|---|---|---|---|
| 9100 | 一番上に 3 語 | 立ち絵（体・目・眉・髪飾り・閉じた口） | 0＝`talk,3`（1200 → 80 ms で 1202 → 80 ms で消す）・1＝`runonce`（1700 → 300 ms で消す）・2＝`periodic,2`（1600 → 500 ms で消す） |
| 9101 | 一番上は 3 語を持たず、element定義で部品 9102 を置く | 体・目・眉・髪飾り・部品 9102（element4） | なし |
| 9102 | 部品（9101 が置く）＝口 | 閉じた口 | 0＝`talk,2`（1203 → 100 ms で消す）・1＝`runonce`（1701 → 300 ms で消す） |
| 9103 | 採らない書き方。何も動かない | 立ち絵（9100 と同じ） | 0＝`talk,abc`・1＝`periodic,0`（どちらも数値が正の整数でない）・2＝`Talk,3`（大文字混じり） |
| 9104 | 採らない書き方。何も動かない | 立ち絵（9100 と同じ） | 0＝`runonce`（pattern定義が 1 つも無い） |

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
3. 有界の自動終了・走行ごとのプロファイル・`RUST_LOG=info,areka_seriko=debug,areka_emo_text=debug` で起こし、`\s[9100]` と台詞、`\s[9101]`、`\s[9103]`、`\s[9104]` ほかを流す。areka の MCP の `sakurascript` はまだ中身が無いので、写しの `scripts\pasta\shiori\event\boot.lua` の `OnBoot`／`OnFirstBoot` が生の台本 1 本を返すようにして流す（面の切り替えの時刻は台本の `\_w[ms]` で決める）。面の写しは MCP の `dump_surface` で繰り返し撮る。
4. 記録を検索して、上の表の件数と、`runonce`・`periodic`・`talk` の開始の時刻を判定する。
