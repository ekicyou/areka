# Brief: areka-P0-offline-surface-dump

> 2026-10-18 棚卸㉗で起票（開発者の指示「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします」＝v0.1.0 のゴール）。根拠は棚卸㉗の調べ（SSP の起動オプションと areka の今の起動の比べ・main `2d86dbff`）。正典は SSP ヘルプ「起動オプション一覧」の「オフラインサーフェスダンプ」（ukadoc MCP の `ssphelp:option`・https://ssp.shillest.net/ukadoc/ssphelp/option.html ）。区分 **A**。版 **v0.1.0**。

## Problem

- **開発者・AI のエージェント**: シェルの絵（surfaces.txt・element・着せ替え）を確かめるには、今は areka を立てて窓を出し、ゴーストを起こし、MCP の `dump_surface` で今見えている面を 1 枚ずつ撮るしかない。窓・GPU・SHIORI・32bit の仲介がそろわないと撮れず、1 枚のために数秒の起動と終了の握手が要る。並んで走る試験とは窓や入力を取り合う。
- SSP には `--offline-dump` がある。ゴーストを起こさず、窓もメッセージループも作らず、指定した面を PNG に書き出して即終わる。surfaces.txt の読み込みの誤りだけを CSV に書き出す使い方（`--dump-error-log` だけを付ける）もあり、終了コードが最も重い誤りの段を返す。シェルの作者と自動の試験には、これがいちばん速い確かめになる。
- 同じ形が areka にあれば、areka と SSP の同じ面の絵を画素で比べる試験を、窓を出さずに CI で回せる（v0.1.0 のもう 1 つの目標「ぱすたが SSP と同じに動く」の確かめにも使える）。

## Current State

- 窓を作らずにシェルを組む道は既にある: シェルの読み込みの入口 `load_shell_target`（`crates/areka-emo-present/src/shell_target.rs` 267 行目・フォルダと画像の読み手を受けて `ShellTarget` を返す・窓も GPU も要らない）。画像の読み手は WIC（`crates/areka-emo-atlas/src/decode/wic_arm.rs` の `WicDecoderArm`）で、COM の初期化だけが要る。
- 面を組む `Composer::compose`（`crates/areka-emo-compose/src/lib.rs` 165 行目）は CPU だけの純粋な合成で、着せ替えの組（`BindSet`・`crates/areka-emo-compose/src/bind.rs`）と動きの状態を受ける。descript の `bindgroup*.default` から組を作る所は `areka-seriko` の `build_static_bindset`（`crates/areka-seriko/src/bind.rs`）。画面に出す絵も同じ合成から作っている（emo の「合成 → アトラス → 出す」の 3 段）＝オフラインで撮った絵と画面の絵の差は出す段だけ。
- 前もって乗算した色を戻して PNG にする所は MCP の `crates/areka/src/mcp/dump_surface_image.rs`（80 行・`png_base64`・中の `unpremultiply` と `png` は私的な関数）。
- 起動の誤りの記録は記録の履歴（`crates/areka/src/log_history.rs`）に WARN 以上が残る。シェルの読み込みの警告・誤りは `load_shell_target` がすべて自分で出す（同じ関数の注記「要件 6 の記録はすべて本関数が出す」）。
- 起動オプションを読む口は無い（`startup-options-core` が作る）。`crates/areka/src/main.rs` は 1,000 行の上限ちょうど。

### SSP の正典（2026-10-18 に ukadoc MCP で読んだ `ssphelp:option`）

- `--offline-dump フォルダパス`: ゴーストを起こさず、そのゴーストの surfaces.txt を読んで面を書き出し、即終わる。SHIORI・MAKOTO・PIRO を読まない。窓とメッセージループを作らない。相対パスは起動したときの今のフォルダから（2.8.98〜）。
- `--dump-shell フォルダ名か名前`（省略で descript の既定のシェル。見つからなければ既定で代えず、エラーログに Error を残して何も出さない・2.8.98〜）・`--dump-scope ID`（省略で 0）・`--dump-surface-list リスト`（カンマ区切り・`surface` の頭は各要素で省略可・surfaces.txt の面の番号の拡張の書き方も使える・省略すると絵は書かない・単独の番号の面が無ければ Warning）・`--dump-output-dir`（省略で今のフォルダ）・`--dump-output-prefix`（既定の `surface` を置き換える・`x_` なら `x_0.png`）。
- `--dump-error-log ファイルパス`: シェルの読み込みの誤りを BOM 付き UTF-8 の CSV（改行 CRLF・各列を `"` で囲む・`"時刻","ゴースト名","レベル","内容"`・時刻は `HH:MM:SS`・レベルは Info／Notice／Warning／Error／Critical／System）へ。1 件も無ければファイルを作らない（既存のファイルも消さない）。終了コードは最も重いレベルで 0（無し・Info・Notice）／1（Warning）／2（Error）／3（Critical）。ゴーストの読み込みそのものに失敗したときは記録なし・終了コード 0。SERIKO の行には定義元のファイルと行（`shell\master\surfaces.txt:Line=12:Surface=10`）が付く（2.8.94〜）。
- `--dump-bind "カテゴリ,パーツ,ON/OFF;…"`（2.9.05〜）: descript の `sakura.bindgroup*.default` を基準に、指定のパーツだけ変える。利用者が最後に着せ替えた保存の値は使わない。パーツ名が空か `__ALL__` でカテゴリの全部。ON/OFF の省略は ON。無いカテゴリ・パーツは Warning で無視して続ける。
- `--dump-surface-option backlog,collision`・`--dump-animation ID` もある（下の Out）。

## Desired Outcome

- `areka.exe --offline-dump <ゴーストのフォルダ> --dump-surface-list "0,1,10" --dump-output-dir <フォルダ>` で、窓も GPU も SHIORI も使わずに、指定の面を PNG（α 付き）に書き出して終わる。名前の付け方（`surface0.png`・`--dump-output-prefix`）は SSP と同じ。
- `--dump-shell`・`--dump-scope`・`--dump-bind` が SSP と同じ意味で効く。
- `--dump-error-log <ファイル>` で、シェルの読み込みの警告と誤りが SSP と同じ形の CSV に残り、終了コードが SSP と同じ段で返る。`--dump-surface-list` を付けなければ誤りの確かめだけで終わる。
- 書き出した絵は、同じ面を areka を立てて MCP の `dump_surface` で撮った絵と、出す段の差を除いて一致する（決定論のテストで固定）。
- 走る間に areka の記憶（起動中の印・最後に使ったゴースト・窓の位置）と利用者の置き場に 1 つも書かない。MCP の待受も立てない。

## Approach

- `startup-options-core` が作る起動オプションの値に、オフラインの一族の項目を足す。`main.rs` は、起動オプションを読んだ直後（窓のアプリ・MCP の待受・記録の履歴の外の初期化より前）に「オフラインなら新しいモジュールへ渡して、その終了コードで終わる」1 行だけを足す。
- 新しいモジュール `crates/areka/src/offline_dump/`（`mod.rs`・`run.rs`・CSV を書く `csv.rs`・兄弟のテスト）が、ゴーストの descript からシェルを決め、`load_shell_target`＋WIC で組み、`--dump-bind` を descript の既定の組に重ねた `BindSet` で `Composer::compose` を呼び、PNG を書く。
- PNG を書く所は `mcp/dump_surface_image.rs` から共有の場所へ出して、MCP とオフラインで同じ関数を使う（色の戻し方が 2 つにならない）。
- CSV は、走る間の記録（WARN 以上）を受ける小さな層を足して作る（記録の履歴の型を使い回せるかは設計で決める）。

## Scope

- **In**: `--offline-dump`・`--dump-shell`・`--dump-scope`・`--dump-surface-list`（拡張の書き方を含む）・`--dump-output-dir`・`--dump-output-prefix`・`--dump-error-log`（CSV の形・終了コードの 4 段・1 件も無いときに作らない）・`--dump-bind`・相対パスの読み方・窓も MCP も記憶も使わない走り方・PNG を書く所の共有・決定論のテスト（面の絵・CSV・終了コード・書かないこと）・SSP の `--offline-dump` の絵と画素で比べる試験の足場（比べる元の絵は SSP で測る）。
- **Out**: `--dump-surface-option backlog,collision`（当たり判定の描き込みと顔まわりの切り抜き＝後の 1 本 `offline-dump-overlays`）・`--dump-animation`（v0.2.0 の dola の作り直しの後）・`--offline-tool`（NAR・更新定義・書庫を作る＝AES の依存が要る・NAR は `pasta_check` が作れる）・バルーンの絵の書き出し（SSP にも無い）。

## Boundary Candidates

- 起動オプションの値（`startup-options-core`）と、オフラインの走り（新しいモジュール）の境＝`main.rs` の 1 行。
- シェルの組み立て（`areka-emo-present`・`areka-emo-compose`・`areka-seriko` の既定の着せ替え）を「窓なしで」呼ぶ所。
- 記録 → CSV の層（記録の履歴とは別に、この走りだけで持つか）。

## Out of Boundary

- シェルの読み方・合成そのもの（今のまま使う）。
- 画面に出す段（GPU・アトラス）。

## Upstream / Downstream

- **Upstream**: `startup-options-core`（起動オプションの値と `main.rs` の入口）。
- **Downstream**: `pasta-in-windows-verification`（SSP と areka の同じ面を画素で比べる）・`config-parse-diagnostics`（設定の読み込みの誤りを詳しくすると CSV の中身が増える）・後の `offline-dump-overlays`。

## Existing Spec Touchpoints

- **Extends**: なし（新しい入口）。完了 `mcp-dump-images`（PNG にする所）を共有の場所へ出す。
- **Adjacent**: `config-parse-diagnostics`（surfaces.txt の誤りの記録に行と場所を付ける＝CSV の「内容」が SSP の `Line=` の形に近づく）・`placement-measure-bake-once`（同じ `load_shell_target` を呼ぶ）・`animated-image-import`（v0.1.0 の後・動く絵の読み手）。

## Constraints

- 失敗はメッセージボックスで告げない（窓を作らない走りなので、記録・CSV・終了コードで返す）。失敗の道は必ず記録（`error!`＋`Err`）。
- `main.rs` は 1,000 行の上限＝足すのは入口の 1 行だけ。テストは兄弟のファイル `*_tests.rs`。
- 試験の根・書き出し先はワークツリーの `target\` の下だけ。
- 依存は足さない見込み（PNG と CRC は今の手書きの関数・BOM 付き UTF-8 の CSV は標準の文字列で書ける）。
- 時刻の扱い: CSV の時刻は `HH:MM:SS`（SSP と同じ）。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: 新 `crates/areka/src/offline_dump/mod.rs`・新 `offline_dump/run.rs`・新 `offline_dump/csv.rs`・新 `offline_dump/*_tests.rs`・`crates/areka/src/main.rs`（入口の 1 行）・`crates/areka/src/cli/`（`startup-options-core` が作る・オプションの項目を足す）・`crates/areka/src/mcp/dump_surface_image.rs`（PNG の関数を共有の場所へ出す）と `dump_surface_image_tests.rs`・共有の置き場（新・例 `crates/areka/src/png_image.rs`）・読むだけ: `crates/areka-emo-present/src/shell_target.rs`・`crates/areka-emo-compose/src/lib.rs`・`crates/areka-seriko/src/bind.rs`・`crates/areka-parsers/src/shell/decode.rs`（面の番号の拡張の書き方を使い回せるか）・`dist/README.txt`。
- 規模: M〜L（14〜18 タスク）。上限 20 の内。20 を超えそうなら `--dump-bind` と SSP との画素の比べの足場を後の 1 本へ。
- 先に要るもの: `startup-options-core`（起動オプションの値と入口）。ファイルの上では `dump_surface_image.rs` を触る spec は他に無い。`main.rs` の 1 行は `startup-options-core` の後。
- 優先度の区分: **A**（開発者の指示「SSP並みの自動試験」「開発がスピードアップすること」）。調べでは開発の速さへの効きがいちばん大きい。
- 版: **v0.1.0**。
- 要件定義のモデル: **Opus**。シェルの組み立てと合成は今ある関数を窓なしで呼ぶだけで、CSV の形と終了コードは SSP の正典が決めている。新しい境界・並行の設計・先例で決まらない骨格の分かれ目のどれにも当たらない。
- 要件の段の議題（答えで作業が変わるもの）:
  1. 書き出す絵の「時刻」: SSP は「runonce のアニメーションを 1 周させた後の状態」を書き出す（`--dump-animation` の説明）。areka は SERIKO の動きを進めずに既定の面だけを組むか、runonce を最後まで進めて組むか（進めるなら `areka-seriko` の時計を回す足場が要る＝2〜3 タスク増）。SSP で測る（runonce を持つ面を `--offline-dump` で撮って比べる）。
  2. element に埋め込んだ動く絵（APNG・動く WebP）は何コマ目で撮るか（SSP は 1 コマ目か・最後か）。SSP で測る。
  3. CSV の「レベル」への当てはめ: areka の記録は WARN／ERROR の 2 段。SSP の Notice・Critical・System をどう当てるか（推し: WARN→Warning・ERROR→Error・Critical と System は使わない）。終了コード 3 は出ないことになる。
  4. 「ゴーストの読み込みそのものに失敗した」（descript が読めない）ときの終了コード: SSP は 0（記録なし）。areka も 0 にするか（試験の道具としては非 0 が便利だが SSP と違う）。
  5. CSV の「内容」の形: SSP 2.8.94 の `[SERIKO] shell\master\surfaces.txt:Line=12:Surface=10 …` に合わせるのは `config-parse-diagnostics` の後か、本 spec で行番号を付けるか。
