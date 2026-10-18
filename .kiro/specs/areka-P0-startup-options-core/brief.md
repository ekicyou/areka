# Brief: areka-P0-startup-options-core

> 2026-10-18 棚卸㉗で起票（開発者の指示「0.1.0のスコープを決定しました。ゴールは、SSP並みの自動試験が出来るようにすること、とします。SSPなみの起動オプション、MCPサポートの安定化、kiro-watchの機能追加など、開発がスピードアップすることを第一目標とします」＝v0.1.0 のゴール）。根拠は棚卸㉗の調べ（SSP の起動オプションと areka の今の起動の比べ・main `2d86dbff`）。正典は SSP ヘルプ「起動オプション一覧」（ukadoc MCP の `ssphelp:option`・https://ssp.shillest.net/ukadoc/ssphelp/option.html ）。区分 **A**。版 **v0.1.0**。

## Problem

- **開発者・AI のエージェント**: areka を自動の試験で起こすとき、指定できるのは「ゴーストのフォルダ」と「バルーンのフォルダ」を並べて渡す形（`areka.exe <ゴースト> <バルーン>`）と、`AREKA_` で始まる環境変数だけである。SSP なら `--ghost`・`--option nobootcheck,bootunlock,standalone`・`--sstp-listen 9900` のように、起動のたびに「どのゴーストで・どの待受で・どんな約束で」立てるかを 1 行で書ける。areka では同じことを環境変数の組み合わせで真似るしかなく、試験ごとに書き方がばらつく。
- **SSP 向けに書かれた手順書・道具**: `ssp.exe --ghost 名前` の形をそのまま areka に向けると、知らない引数としてゴーストのフォルダの位置に読まれ、「ゴーストが見つからない」で止まる。
- これは v0.1.0 の 2 つの目標（SSP 並みの自動試験・開発の速さ）の入口で、`offline-surface-dump`・`readonly-mode`・`startup-install-option` の 3 本が、この spec の作る「起動オプションを読む口」の上に載る。

## Current State

- 起動の引数は、起動前の解決 `resolve_boot_from`（`crates/areka/src/boot_config.rs` の起動の解決の関数）が `args.get(1)` をゴーストのフォルダ、`args.get(2)` をバルーンのフォルダとして読むだけ。ほかの引数は読まない。引数を読む部品（クレート）は使っていない。先例として、机の貸し出しのアプリ `crates/areka-impl-watch/src/cli.rs` が自前でコマンドラインを読んでいる。
- 引数でゴーストを渡した起動は、目録・起動中の印・最後に使ったゴーストの記憶・利用者ごとの置き場への写しを読まない（同じ関数の注記・`user-data-root` の要件 4.1・5.1）。この「絶対パス起動」は roadmap の保存義務（「実機サインオフの『絶対パス起動』（argv 上書き）を残す」）で、実機の試験（`crates/areka/tests/emo2_real_run.rs`・`mcp_get_log_real_run.rs`・`smoke_boot_loop_exit.rs`）がこの形で起こしている。
- 試験のつまみは環境変数だけ: `AREKA_ROOT`・`AREKA_USER_ROOT`・`AREKA_PROFILE_DIR`・`AREKA_MCP_PORT`（0 で待受なし）・`AREKA_NO_ALERT`・`AREKA_APP_SMOKE_EXIT_MS`・`AREKA_SHIORI_REQUEST_TIMEOUT_MS` ほか。
- MCP の待受の番号は `crates/areka-mcp/src/port.rs` の `read_port_candidates`（9801→9821 を早い者勝ち・両方塞がりで隣の +1〜+9・`AREKA_MCP_PORT=p` ならその番号だけ・つなぐ先は `127.0.0.1`）。
- 二重起動の確かめは無い（実機の試験は areka を並べて立てることに頼っている）。
- 配布物の説明書 `dist/README.txt` は環境変数 3 つ（`AREKA_ROOT`・`AREKA_USER_ROOT`・`AREKA_MCP_PORT`）だけを書き、「引数は要らない」と書く。
- release 版は窓だけのアプリ（`main.rs` の 1 行目 `windows_subsystem = "windows"`）で、端末へは何も出ない。結果は終了コード・記録・ファイルで返すことになる。
- `crates/areka/src/main.rs` はちょうど 1,000 行（1 ファイル 1,000 行の番人 `file_length_guard` の上限）。引数を読む所（171〜183 行目）を外へ出さないと 1 行も足せない。

### SSP の正典（2026-10-18 に ukadoc MCP で読んだ `ssphelp:option`）

- 書き方は `--名前` を推す。古い `/x`・`-x` も使える。値は空白で区切る（`--名前 値`）。2.8.97 からは `--名前=値` も受ける（`/x` の形では `=` は使えない）。
- `--option` はカンマ区切りで複数（例 `--option bootunlock,standalone`）。語: `nobootcheck`（異常終了の確かめをしない）・`layered`／`region`（半透明の窓の有無）・`bootunlock`（二重起動の確かめを飛ばす）・`standalone`（起動中の SSP へ任せない）・`kiosk`・`readonly`・`dontloadmodule`（SHIORI／MAKOTO／PLUGIN などを読まない）・`dpiaware`・`callghost`・`deletesource`（後の 2 つは `--install` と組む）。
- `--ghost フォルダ名/ゴースト名`: 空白区切りで複数。起動時は最初のものを 1 組目にする。2.8.99 からは、ゴーストのフォルダの絶対パスを渡すと、読み込みの置き場に無ければ親のフォルダを「その起動中だけ」置き場に足す（設定に保存しない・インストール先の候補にしない）。
- `--random`（設定にかかわらず無作為に起動）・`--help`（起動オプションの説明の頁をブラウザで開く）・`--sstp-listen 待ち受け先`（保存された待受を読まず、指定だけで待ち受ける。`ポート番号`＝ループバック・`localhost:p`・`IPv4:p`・`[IPv6]:p`・`none`、カンマ区切りで複数、書式の誤りと待受の失敗は無視してエラーログへ）。
- SSP の MCP は SSTP と同じ待受（`127.0.0.1:9801/api/mcp`・SSP ヘルプ「その他の機能」）＝areka では `--sstp-listen` が MCP の待受の並びに当たる。

## Desired Outcome

- areka が SSP と同じ書き方の起動オプションを読む（`--名前 値`・`--名前=値`・`/x`・`--option a,b`）。知らない名前は記録（warn）に残して無視し、起動は続ける（メッセージボックスは出さない）。
- 次のオプションが効く:
  - `--ghost パス|名前`: 絶対パスなら、そのフォルダを今の起動だけの一時の置き場として起こす（保存しない・インストール先にしない）。名前（フォルダ名またはゴースト名）なら置き場の並びから探して起こす。複数並べたら最初の 1 体を起こし、残りは記録に残す（同時に何体も起こすのは `multi-ghost-runtime`）。
  - `--random`: 最後に使ったゴーストの記憶にかかわらず無作為に選ぶ。
  - `--option nobootcheck`: 起動中の印（前回きれいに終わらなかった印）を読まない。
  - `--option bootunlock`・`standalone`・`layered`・`region`・`dpiaware`: 受け取り、今の areka では何も変わらないことを info に 1 行残す（二重起動の確かめも任せる先も無いので、SSP の `bootunlock,standalone` は今の areka の常の振る舞い）。
  - `--option dontloadmodule`: SHIORI を読まずに起こす。
  - `--sstp-listen 待ち受け先`: MCP の待受の並びを指定だけにする（`none`／省略で待ち受けない）。環境変数 `AREKA_MCP_PORT` より強い。
  - `--help`: 起動オプションの説明（SSP ヘルプの頁か areka の説明書）を開いて終わる。
- 引数を並べて渡す今の形（`areka.exe <ゴースト> <バルーン>`）は今までどおり動く（roadmap の保存義務）。実機の試験 3 本は 1 行も変えずに緑。
- `dist/README.txt` に起動オプションの節が加わる。

## Approach

- 引数を読む口を新しいモジュール `crates/areka/src/cli/`（`mod.rs`・`parse.rs`・兄弟のテスト `parse_tests.rs`）に作り、読んだ結果を 1 つの値（起動オプション）にする。`main.rs` は `std::env::args()` と `resolve_boot` の呼び出しを、この値を読む 1 行と、外へ出した失敗の扱いへ置き換える（行は減る）。
- 起動の解決（`boot_config.rs`・`boot_resolve.rs`）は、引数の配列でなく起動オプションの値を受ける。位置で渡した 1 つ目・2 つ目は、`--ghost`／バルーンの指定の別名として同じ値に入る（今の道をそのまま通る）。
- MCP の待受は `areka-mcp/src/port.rs` に「起動オプションから候補を作る」入口を足し、`main.rs` が環境変数より先にそれを見る。
- 引数を読む部品は足さない（`areka-impl-watch` の自前の読み方が先例。SSP の書き方は `--x v`・`--x=v`・`/x`・カンマ区切りの 4 つだけで、部品の流儀と合わない所もある）。

## Scope

- **In**: 引数を読む口（4 つの書き方・知らない名前の扱い）・`--ghost`（パス・名前・複数のときは最初の 1 体）・`--random`・`--option` の `nobootcheck`・`bootunlock`・`standalone`・`layered`・`region`・`dpiaware`・`dontloadmodule`・`--sstp-listen`・`--help`・位置で渡す形の別名・`main.rs` から引数の扱いを外へ出すこと・`dist/README.txt` の節・決定論のテスト（読み方の表・起動の解決の答え）。
- **Out**: `--offline-dump` の一族（`offline-surface-dump`）・`--option readonly`（`readonly-mode`）・`--install`・`--option callghost,deletesource`・`initial_install.nar`（`startup-install-option`）・`--option kiosk`（通知領域の後）・`--fmo-name`・`--ukagaka-link`・`--ssf`（MCP で足りる）・`--windowmode`・`--guifont`・`--offline-tool`・起動中の areka へ処理を任せる仕組みと二重起動の確かめ（SSTP の受け口の後）・複数のゴーストを同時に起こすこと（`multi-ghost-runtime`）。

## Boundary Candidates

- 引数を読む口（文字列の列 → 起動オプションの値・純粋）と、起動の解決（値 → ゴースト・バルーン・置き場）の境。
- 一時の置き場（`--ghost` の絶対パスの親）を、置き場の並び（`baseware-root-list` の並び）へ「保存しない 1 本」として足す所。
- MCP の待受の候補の決め方（起動オプション → 環境変数 → 既定）。

## Out of Boundary

- 置き場の並びそのものの決め方と保存（完了 `baseware-root-list`）。
- 開発用のフォルダの別名の表（`dev-folder-alias`）。

## Upstream / Downstream

- **Upstream**: なし（完了 `baseware-root-list`・`user-data-root`・`mcp-server-core` の上に立つ）。
- **Downstream**: `offline-surface-dump`・`readonly-mode`・`startup-install-option`（この spec の起動オプションの値に項目を足す）・`real-run-mcp-kit`（環境変数の代わりに `--sstp-listen` などを使える）・`pasta-in-windows-verification`（SSP と同じ書き方で起こせる）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい入口）。
- **Adjacent**: `dev-folder-alias`（別名の表＝`--ghost` の名前の引き当てと同じ所を読む）・`multi-ghost-runtime`（議題 2＝試験から 2 体目を起こす入口・`--ghost A B` の残りを引き取る）・`update-check-options`（`\![update…]` の `--option` は台本の語で、起動の `--option` とは別）・`emo2-real-run-stability`（起動の形は変えない）。

## Constraints

- 位置で渡す形（`areka.exe <ゴースト> <バルーン>`）を残す（roadmap の保存義務「絶対パス起動」）。実機の試験 3 本を変えずに通す。
- `main.rs` は 1,000 行の上限ちょうど＝新しい処理は新しいモジュールへ。テストは兄弟のファイル `*_tests.rs`。
- 失敗はメッセージボックスで告げない。知らない引数・書式の誤りは warn の記録。起動できないときは今までどおり既定のゴーストの台詞の道（`alert`）。
- 本番の環境変数は `AREKA_` の名前の中に置く（新しく足すなら）。オプションと環境変数が両方あれば、オプションが勝つ（SSP の `--sstp-listen` が保存された待受より勝つのと同じ）。
- 失敗の道は必ず記録（`error!`＋`Err`）。
- 試験の根・検体はワークツリーの `target\` の下だけ。

## 2026-10-18 棚卸㉗の見立て（main 2d86dbff）

- 触るファイル: 新 `crates/areka/src/cli/mod.rs`・新 `cli/parse.rs`・新 `cli/parse_tests.rs`・`crates/areka/src/main.rs`（引数の扱いを外へ出す＝減る）・`boot_config.rs`（467 行・`resolve_boot`／`resolve_boot_from` が値を受ける）・`boot_resolve.rs`（676 行・`--random`・`nobootcheck`）と兄弟のテスト（`boot_config_*_tests.rs`・`boot_resolve_tests.rs`）・`crates/areka-mcp/src/port.rs`（76 行）と `port_tests.rs`・`dontloadmodule` の道（今の areka に「SHIORI を読まずに起こす」道は無い。`shiori.file` が無いと接続の失敗として終了の系列へ倒れる＝`crates/areka-ghost/src/shiori_wiring.rs` の接続の口。何を聞かれても「中身なし」と答える SHIORI の代わりを差し込む形が要る。テストの偽物は 17 か所あるが本番の型は無い・調べの「`LogSink` の起動の道がある」は台詞の出口の代わりで、SHIORI の代わりではない）・`--help` の開き口（`crates/areka/src/readme/opener.rs` を使う）・`dist/README.txt`・網羅台帳に起動オプションの行が無ければ足すかは議題。
- 規模: M（10〜12 タスク・`dontloadmodule` を入れると 12〜15）。上限 20 の内。
- 先に要るもの: なし。ファイルの上では、`main.rs`・`boot_config.rs`・`boot_resolve.rs` を触る `multi-ghost-runtime`（v0.1.0 の後）と `dev-folder-alias` と同じウェーブに置かない。
- 優先度の区分: **A**（開発者の指示「SSPなみの起動オプション」）。
- 版: **v0.1.0**。
- 要件定義のモデル: **Opus**。引数を読む口は `areka-impl-watch` の自前の読み方が先例で、起動の解決は今ある道に値を渡すだけ。新しい境界・並行の設計・先例で決まらない骨格の分かれ目のどれにも当たらない。
- 要件の段の議題（答えで作業が変わるもの）:
  1. `--ghost` の名前の引き当ての順（フォルダ名が先か・ゴースト名〔descript の `name`〕が先か・`sakura.name` も見るか）。SSP で測る（同じ名前のフォルダとゴーストを並べたときに SSP がどちらを取るか）。
  2. `--ghost` の絶対パスの起動で、今の「絶対パス起動」（目録・記憶・起動中の印・写しを読まない）と、SSP の「親を一時の置き場に足す」をどう合わせるか。推し: 位置で渡す形は今のまま（何も読まない・書かない）、`--ghost パス` は SSP どおり一時の置き場として足し、記憶は読むが置き場の並びには保存しない。
  3. `--help` で開く先（SSP ヘルプの頁 `ssphelp/option.html`・配布物の説明書・areka 独自の頁）。
  4. `dontloadmodule` を本 spec に入れるか（入れると「何にも中身なしで答える SHIORI の代わり」を本番に足す＝`areka-ghost` の接続の口に触り、2〜3 タスク増える）。SHIORI が無いときの起動の知らせ・既定の台詞・終了の握手の振る舞いは SSP で測る。外すなら `readonly-mode` の後の 1 本へ回す。
  5. 結果を返す道（release 版は端末を持たない）: 書式の誤りを終了コードで返すか、記録だけで起動を続けるか。推し: SSP どおり無視して続ける。
