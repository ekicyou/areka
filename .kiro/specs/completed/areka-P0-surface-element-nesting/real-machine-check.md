# 実機の確かめ（areka-P0-surface-element-nesting・タスク 8.4）

検体のシェル（`crates/areka-emo-compose/tests/fixtures/surface-nesting/`）を emo2 のシェルとして本物の areka に読ませ、`\s` の切り替えをまたいで子のサーフェスのアニメーション（部品）の記録が途切れないこと（要件 5.6・5.7）と、無い番号・循環・子の中の箱の警告が読み込み 1 回につき 1 度ずつ出ること（要件 3.1・3.2・6.1）を、有界の自動終了とログの検索で判定する。判定はすべてログで行い、目視の項目は無い。

## 1. 走らせ方

| 項目 | 値 |
|---|---|
| 道具 | 本フォルダの `real-machine-run.ps1`（`-Prepare` で根を作る・`-Run` で起こして判定まで・`-Check` は判定だけをやり直す） |
| 根 | ワークツリーの `target\nest`。`nar-sample-path emo2` が展開した `target\nar-samples\manual\emo2` の写しに、`areka.exe`（debug）と 32bit の補助プロセス（PE の機種 `014C` を確かめる）を置いたもの |
| シェル | 写しの `ghost\emo2\shell\master` の中身を**検体の写しだけ**に入れ替える（emo2 の絵と surfaces.txt は残さない）。検体には `descript.txt` が無いので、`type,shell`・`name,surface-nesting`・`seriko.use_self_alpha,1`・`seriko.alignmenttodesktop,bottom` だけの最小の形を足す（着せ替えの定義は書かない＝surface0 の `animation100`（bind）の先 40・41 は絵に出ない・実装メモ 1.2） |
| 台本 | 写しのゴーストの `ghost\master\scripts\pasta\shiori\event\second_change.lua` だけを差し替える（shell-balloon の実機と同じ取り方）。起動の台詞 1 本で `\0` の面を 8 秒ごとに **0 → 1 → 2 → 30 → 0 → 2 → 1 → 0** と切り替える。ゴースト自身の台詞（ランダムトーク・マウス・シェル切替・面の復帰など）はこの回のあいだ黙らせる |
| 環境変数 | `NO_COLOR=1`・`RUST_LOG=info,areka_seriko=info,areka_emo_present::shell_target=info`（部品の記録は `info!`、警告は `warn!`、読み込みの回数を数える一覧の行は `info!`、面の切り替えの `apply(ShowSurface)` も `info!`）・`AREKA_APP_SMOKE_EXIT_MS=85000`（有界の自動終了）・`AREKA_ROOT`／`AREKA_PROFILE_DIR`＝根・`TMP`／`TEMP`＝根の `tmp` |
| 起動 | `<根>\areka.exe "<根>\ghost\emo2"`（ゴーストは絶対パスで渡す・デバッグ版の空のコンソール窓は `-WindowStyle Hidden`） |
| 記録 | `target\nest\run.log`（標準出力）・`run.err.log`・`runs.txt`（起動と終わりの時刻）・`exit.txt`（終了コード） |

面の役（検体の surfaces.txt の見出しより）:

| 面 | 子 10 | 役 |
|---|---|---|
| 0 | (20,30) に置く | 親 A |
| 1 | (90,50) に置く | 親 B |
| 2 | `element0` が子 10 | 親 C |
| 30 | 置かない | 3 段の入れ子の頭（30 → 31 → 32・どれもアニメーションを持たない） |

子 10 は `animation0.interval,random,2`（1 秒の境界ごとに 1/2 で発火・閉じ目 12 を 50ms → `-1` で停止）を持つ。

### 準備（済み・作り直すときだけ）

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\.kiro\specs\completed\areka-P0-surface-element-nesting\real-machine-run.ps1" -Prepare
```

`CARGO_INCREMENTAL=0` で `areka`（debug）と補助プロセスを組み、C: の空きが 3 GB 未満ならビルドせずに止まる。

### 起動と判定（開発者の GO の後）

```
pwsh -NoProfile -File "C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\.kiro\specs\completed\areka-P0-surface-element-nesting\real-machine-run.ps1" -Run
```

約 85 秒で自動で終わり、続けて判定の一覧と `RESULT: PASS`／`RESULT: FAIL` を出す（不合格があれば終了コード 1）。走行のあいだ検体の小さな絵（単色の四角）が出て、8 秒ごとに形が替わる。触らなくてよい。判定だけをやり直すときは `-Run` の代わりに `-Check`。

## 2. 判定（`-Check` が記録を検索して決める）

N＝読み込みの回数＝`shell: シェルの面の画像の一覧が終わった` の行のうち `shell_dir` が `nest\ghost\emo2\shell\master` のもの。読み込みは起動時の資産組立と配置の採寸（`placement/measure.rs`）の両方で行われるので、起動 1 回で N＝2 になりうる（実装メモ 6.1）。警告は「読み込み 1 回につき 1 度」で数える。

| # | 確かめること | 合格の条件 | 要件 |
|---|---|---|---|
| J1 | 有界の自動終了で終わった | `event="app_exit"` と `origin=Smoke` の行が 1 行以上・終了コード 0 | — |
| J2 | 検体のシェルを読んだ | N ≥ 1 | — |
| J3 | 警告 3 種が読み込み 1 回につき 1 度ずつ | 下の 6 件がそれぞれ**ちょうど N 行**（`WARN`）。3 種の文言の行の総数が **6N**（仕込んでいない件が 0） | 3.1・3.2・6.1 |
| J4 | 台本どおりに面が替わった | `apply(ShowSurface): 表示・マスクを更新` の `target_id=TargetId(0)` の `surface_id` を、続く同じ番号をまとめて並べると `0,1,2,30,0,2,1,0` | — |
| J5 | 子 10 を置く面のあいだ、部品の記録が切り替えをまたいで続く | 面 0・1・2 の 7 区間のそれぞれに、`seriko: part 抽選発火` で `scope="0"`・`part=10` の行が 1 行以上 | 5.6・5.7 |
| J6 | 子 10 を置かない面（30）のあいだは抽選しない | 区間 4（面 30）の始まりから次の区間の 500ms 前までに、上の抽選発火の行が 0 行（戻る刻みの記録は絵の差し替えの行より先に出うるので、末尾 500ms は数えない） | 5.7 |
| J7 | 発火した再生が停止まで進む | `seriko: part 停止`（`scope="0"`・`part=10`）の行数 ≥ 抽選発火の行数 − 1（自動終了で切れた最後の 1 回だけ欠けてよい） | 5.6 |
| J8 | 失敗の記録が無い | `ERROR` の行が 0・`seriko: part \`-1\` 以外の負 surface` が 0 | — |

J3 の 6 件（文言は `crates/areka-emo-present/src/shell_target.rs` の `log_nest_issue`・`log_box_issue`、件は検体の surfaces.txt の「仕込んだ読み飛ばし」）:

| 種類 | 文言 | 欄 |
|---|---|---|
| 無い番号 | `shell: element定義が指したサーフェスが無いので置かない` | `surface=50 element=1 target="9999"` |
| 無い番号（範囲超え） | 同上 | `surface=50 element=2 target="4294967296"` |
| 循環（自分自身） | `shell: element定義の参照が循環するので、先祖へ戻る参照を置かない` | `surface=60 element=1 target=60` |
| 循環（相互） | 同上 | `surface=61 element=1 target=62` |
| 循環（相互） | 同上 | `surface=62 element=1 target=61` |
| 子の中の箱 | `shell: 子として置かれたサーフェスの箱は親の中に置かない` | `parent=71 child=70 name="fuda"` |

部品の記録の文言は実装メモ 7.3（`crates/areka-seriko/src/parts.rs`）。`scope="0"` で絞るのは、`\1` の既定の面（10＝子そのものを一番上に出す）の記録を混ぜないため。

手で照らすときの 1 行（区間の境目と部品の記録を時刻順に並べる）:

```
Select-String -Path "C:\home\maz\git\areka\.claude\worktrees\areka-p0-translate-pipeline-2cf68b\target\nest\run.log" -SimpleMatch -Pattern 'apply(ShowSurface)','seriko: part','shell: element定義','shell: 子として','shell: シェルの面の画像の一覧'
```

### 偶然で落ちる見込み

J5 は 8 秒の区間に 1/2 の抽選が約 8 回あるので、1 区間で 1 度も発火しない見込みは約 1/256、7 区間のどれかで起きる見込みは約 3%。J5 だけが 1 区間で落ちたときは、`-Run` をもう 1 回走らせて判断する（実装が壊れていれば同じ区間の型で落ち続ける）。

## 3. 結果

2026-10-05 08:21〜08:22（開発者の GO の後に 1 回）: **RESULT: PASS**（J1〜J8 全項目合格・終了コード 0）。

- J1: 有界の自動終了（`app_exit` origin=Smoke 1 行・終了コード 0）。
- J2: 検体のシェルの読み込み N=2（配置の採寸でも読み込むため・実装メモ 6.1）。
- J3: 警告 6 件（無い番号 `9999`・`4294967296`／循環 60→60・61→62・62→61／子の中の箱 71→70 `fuda`）がそれぞれ 2 行＝読み込み 1 回につき 1 行。総数 12＝6N で、仕込んでいない件は 0。
- J4: 面の切り替えの並びは 0,1,2,30,0,2,1,0 で台本どおり。
- J5・J6: 子 10 を置く 7 区間の `part=10` の抽選発火は 4・3・5・6・6・4・15 行で、どの区間も途切れない（要件 5.6・5.7）。子を置かない面 30 の区間は 0 行（要件 5.11）。
- J7: 抽選発火 43・停止 43。
- J8: ERROR 0 行・部品の負の番号の warn 0 行。

記録は `target\nest\run.log`。
