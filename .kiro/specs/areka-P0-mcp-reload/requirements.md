# Requirements Document

> 本文の実測は **2026-10-18・本ブランチ**（main `e2370df8`＝棚卸㉗の PR#334 を取り込んだ後）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない（brief の行番号は古いコミットのもの）。
> 「要件の段での暫定の裁定」の表は、要件を書く途中で答えが要った点に**推奨案で暫定の答えを置いたもの**で、要件ディスカッションで覆せる（覆したら該当する要件も改める）。SSP で測っていない点（成功の本文・shell／balloon／shiori の読み直しで SHIORI イベントが出るか）は、要件ディスカッションの前に SSP の MCP で測ってから確定する（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。本 spec の作業中は開発者の SSP に `reload` を打っていない。

## Project Description (Input)
**誰が困っているか**: ゴーストの辞書・シェル・バルーンを書き換えながら動きを確かめる作者と、同じことを MCP 越しに行う AI エージェント。

**今の状況**: SSP には再読み込み（MCP の `reload` と、さくらスクリプトの `\![reload,…]`）があり、ghost／shiori／shell／balloon／makoto／descript の 6 対象を読み直せる。areka には再読み込みという操作そのものが無く、MCP の `reload` は `NG:not implemented yet` を返すだけ、台本の `\![reload,…]` を拾う受け取り手も 0 件。いちばん近い既存の道は、同じゴーストのフォルダへの切替（`network-update` の再読み込み）と、シェル・バルーンの切替（`shell-balloon-switch`）である。

**何を変えるか**: 再読み込みを 1 つの操作として作り、MCP の `reload` と台本の `\![reload,ghost／shiori／shell／balloon／descript]`（`\![reload,descript,パラメータ]`・旧 `\![reloadsurface]` を含む）の両方から呼べるようにする。読み込みの失敗はエラーログ（`mcp-log-history` の error 種別）へ出す。makoto は `makoto-dll-host` が着地するまで `NG:` で答えてログを出す縮退の口だけを置き、同 spec へ申し送る。範囲外: makoto の本体・`\![reload,aigraph]`・別のゴースト／シェル／バルーンへの切替・ファイルの変更を見張っての自動再読み込み。

## Introduction

### 誰が困っているか

- **ぱすたさんを作る開発者**（棚卸㉗の問い合わせの答え）: 毎日の流れは「辞書を書き換える → 読み直す → 確かめる」。SSP では MCP の `reload` を ghost・shiori・shell で使っている。areka では読み直しが無いので、試すたびに areka を終了して起動し直している。pasta のデバッガの「SHIORIリロード」も、エンジンが `\![reload,shiori]` を出してベースウェアに読み直させる作り（pasta docs `debug/dev-actions`）。
- **AI エージェントでゴーストを作る人**: SSP と同じ手順（ファイルを直す → `reload` → `get_log` で失敗を見る）で作業したい。SSP の `reload` の説明文は、読み込み中のエラーはエラーログに残るので `get_log` で見よ、と書いている。
- **既存のゴーストの作者**: 台本の `\![reload,…]` を使うゴーストが、areka では何も起きずに黙って壊れる（網羅台帳の各行の「壊れ方: 黙って壊れる」）。

### 正典（ukadoc `list_sakura_script`）

| 綴り | ukadoc の意味（要約） |
|---|---|
| `\![reload,ghost]` | ゴースト全体を読み直す。記述例の説明は「自分自身へのゴースト切り替えと、SHIORIイベントが発生しないことを除けばほぼ同じ」 |
| `\![reload,shiori]` | shiori.dll を読み直す。結果として辞書ファイルを全部読み直す（起動中に書き換えた辞書を効かせるときに使う） |
| `\![reload,shell]` | シェルを読み直す |
| `\![reloadsurface]` | 旧仕様。SSP では `\![reload,shell]` と同じ |
| `\![reload,balloon]` | バルーンを読み直す |
| `\![reload,descript]` | すべての descript.txt を読み直す |
| `\![reload,descript,パラメータ]` | 指定した descript.txt を読み直す。半角スペースで区切って複数指定できる。語は `ghost` `shell` `balloon` `headline` `plugin` `language` `calendar.skin` `calendar.plugin`（`calendar skin`・`calendar plugin` の表記も受け付けるが、区切りが半角スペースなので点の表記を勧める） |
| `\![reload,makoto]` | makoto.dll を読み直す（本 spec は縮退の口だけ） |
| `\![reload,aigraph]` | AI グラフを読み直す（areka に AI グラフは無い＝範囲外） |

SSP の MCP の `reload`: 引数は `target`（ghost／shiori／shell／balloon／makoto／descript）と `ghost_name`（省略可）。知らない `target` への答えは `NG:Unknown target (ghost / shiori / shell / balloon / makoto / descript)`（`doc/ssp-mcp/survey.md` §3）。成功の本文は未実測（同 §5）。

### いま何が起きているか（2026-10-18 実測）

- **ダミーの所在。** アプリ本体側 `crates/areka/src/mcp/reload.rs` の `handle` は `NG:not implemented yet` を返す 1 文だけ。`crates/areka/src/mcp/reload_tests.rs` は同じ文言を期待する（中身を入れたら書き換える）。プロトコル側 `crates/areka-mcp/src/tools/reload.rs` は定義の逐語（`DEFINITION`）と `Args { target, ghost_name }` まで完成。`target` の値の検査はアプリ本体側の受け持ち。
- **宛先の解決は手前で済む。** `crates/areka/src/mcp/mod.rs` の `dispatch` が `Reload` を「省略なら起動中の 1 体」で解決してから `handle` を呼ぶ。その場で答えられない処理は「後から答える置き場」（同ファイルの `later`）に返事を預けられ、置き場は毎フレーム覗かれる（UI スレッドは待たない）。MCP の橋は返事を最大 10 秒（`crates/areka-mcp/src/tools/mod.rs` の `REPLY_WAIT`）待ち、超えたら `NG:areka did not respond within 10 seconds` で答える（`crates/areka-mcp/src/tools/bridge.rs` の `timeout_text`）。
- **同じゴーストへ載せ替える既存の道。** `crates/areka/src/update/desk.rs` の `fn reload` が、`request_ghost_switch`（`crates/areka/src/emo2_boot/ghost_switch.rs`）へ「同じフォルダ・`raise_event: false`・起動の知らせつき」で切替を頼んでいる。`raise_event` が偽なら `OnGhostChanging` も `OnClose` も送らずに降ろす。降ろす側（`take_down`）は SHIORI を unload する＝辞書は起こし直しで読み直される。
- **起動の側の口が足りない。** 起動の由来 `BootOrigin`（`crates/areka-kanade/src/change.rs`）は `Plain`／`ChangedFrom`／`Halted`／`Updated` の 4 つで、起動の根を選ぶ所（`crates/areka-kanade/src/schedule/boot.rs` の `boot_root`）は `ChangedFrom` に `OnGhostChanged`、`Updated` に渡された知らせ、`Plain`・`Halted` に `OnBoot` を送る。**起動の知らせを 1 つも送らない由来が無い**。`switch_to` は起動の知らせが無ければ `ChangedFrom` を選ぶ＝今の道のまま同じゴーストへ切り替えると `OnGhostChanged` が出る。
- **切替の入口の判定。** `request_ghost_switch_with` は、切替の途中（`SwitchInFlight`）なら `Busy`、目録に無ければ `NotFound`、文脈が無ければ `NoContext` を返し、どれも `warn!` を 1 件残す。再生中のトークがあると、kanade は切替をトークの完了まで保留する（`crates/areka-kanade/src/schedule/change.rs`）。切替先が起きなければ、既定のゴーストへ戻す（`switch_to_default`・既定も起きなければ致命）。
- **切替の入口の判定の穴。** ゴースト切替の入口はシェル・バルーンの切替の進行中とネットワーク更新の実行中を見ていない（シェル・バルーンの切替の入口は両方を見る）。読み直しは自分の入口で全部を見る（要件 2.1）。
- **シェル・バルーンの切替。** `crates/areka/src/emo2_boot/shell_balloon_switch.rs` の `request_skin_switch` が台本とメニューの唯一の入口。進行中（`Busy`）・ゴースト切替中・ネットワーク更新中は `warn!` を残して断る。受理したら背景で資産を組み、kanade に台詞の切れ目を待ってもらって差し替える。差し替えの後に `OnShellChanged`／`OnBalloonChange` を送る（`crates/areka/src/emo2_boot/frame/switch.rs`）。今と同じ名前を指しても素通りせず、組み直して `Changed` の知らせを送る（読みからの推定・固定したテストは無い）。資産の作り置きは無く、受理のたびにディスクから読む。
- **`\!` の受け取り手の台帳。** `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical` に `reload`・`reloadsurface` の行は無い。登記の無い `\!` は「誰も拾わない」として何も起きず、areka 独自の MCP ツール `check_script` は「知らない命令」と答える。
- **エラーログ。** `crates/areka/src/log_history.rs` の `classify` は、`warn` 以上の記録をすべて error 種別に振り分ける＝`warn!`・`error!` で残した記録は MCP の `get_log`（`log_type=error`）で読める。ただし記録の名前は既定で `[SYSTEM]` になり、`get_log` の `ghost_name` の絞り込みに掛からない（ゴーストの名前を載せる取り決めは同ファイルの `TARGET_ERROR`＋`ghost` の欄・使い方の例は `crates/areka/src/readme/opener.rs`）。今の切替の失敗の記録はどれもこの取り決めを使っていない。
- **SHIORI だけを読み直す道は無い。** SHIORI の接続は 1 回きり（`crates/areka-kanade/src/shiori/real.rs` の `spawn_shiori_actor`）で、SHIORI のスレッドへの指示は `Request`／`Unload`／`Close` だけ（kanade の `msg.rs`）。作るなら ⒜ kanade に接続し直しの指示を足す（`msg.rs`・`actor.rs`＝`mcp-kanade-tools` の持ち物と重なる）か、⒝ 窓を残したまま実行系（`GhostRuntime`）を組み直す（UI の側は降ろす・窓を閉じる・窓を開き直すが既に分かれている）。
- **網羅台帳の行**（`doc/ukadoc-coverage/ledger/sakura-script.toml`）: `reload,ghost`・`reload,shell`・`reload,shiori`・`reload,descript,パラメータ` は `owner` が空、`reload,descript`・`reloadsurface` は別名の行、`reload,balloon` は `owner` が `areka-P0-balloon-canon-residue` のまま（棚卸㉑の裁定 5 で本 spec へ移したのに古い）、`reload,makoto` は `owner` が `areka-P0-makoto-dll-host`（命令は切り出し先の `makoto-reload-directives` が持つ）。
- **上限の近いファイル**: `consumer_ledger.rs` 960 行（うちファイルの中のテストが約 517 行）・`ghost_switch.rs` 948 行・`emo2_boot/mod.rs` 916 行（1 ファイル 1,000 行の目安）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | `\![reload,shiori]` を本 spec に含めるか | **✅ 確定（2026-10-18 開発者）: 含める。シェルと窓を残して SHIORI だけを読み直す本物を作る**（ghost の読み直しの別名にはしない・本 spec から外さない） | 開発者「シェルを維持して SHIORI だけリロードする需要は高い。省略すべきでない」。タスクが 20 を少しまたいでも本 spec から切り出さない | 4・12.5・13.2 |
| 2 | ゴーストの読み直しで起動の知らせを送るか | **送らない**（`OnBoot`・`OnGhostChanged`・`OnFirstBoot` も送らない）。起動の由来に「知らせを送らない」1 種を足す既定の方針 | ukadoc「SHIORIイベントが発生しない」。ぱすた開発の注意「ゴーストの読み直しで OnBoot は送られない」 | 3.2・3.3 |
| 3 | 起こし直しのときの情報の通知（`basewareversion` など、SHIORI へ環境を伝える NOTIFY） | **ふつうの起動と同じく送る** | 読み直した SHIORI は前の値を持っていない。台詞を生む GET の知らせとは別のもの。SSP で測って違えば改める | 3.4 |
| 4 | shell・balloon・shiori の読み直しで SHIORI イベント（`OnShellChanging`・`OnShellChanged`・`OnBalloonChange` など）を送るか | **送らない** | ukadoc は ghost の読み直しについて「SHIORIイベントが発生しない」と書く。読み直しは切替ではない。SSP で測って違えば改める | 4.3・5.3・6.3 |
| 5 | `\![reload,balloon]` はバルーンを選び直すか | **今のバルーンのフォルダを読み直すだけで、選び直さない**（ゴーストの descript の指定 → 記憶 → 同梱、の順で決め直さない） | 棚卸㉓の見込み。読み直しは「同じものを読み込み直す」操作 | 6.1 |
| 6 | MCP の返事をいつ返すか | **読み直しが終わってから返す**（後から答える置き場に預ける）。10 秒を超えたら橋の時間切れの答えになり、読み直しは続け、結果は記録に残す | 読み直しの直後に AI エージェントが次のツールを呼ぶと、終わる前なら「起動中でない」と答えられて紛らわしい。SSP の説明文は「失敗はエラーログで見よ」と書く。橋の上限 `REPLY_WAIT` は `crates/areka-mcp` にあり、本 spec は触らない | 8.1〜8.5 |
| 7 | 成功・失敗の本文 | 成功 `OK`（`isError: false`）。知らない `target` は SSP の実測の文。読み直しを始められない・読み込みに失敗した・makoto は `NG:` で始まる英文（下の要件に置いた暫定の文） | SSP の成功の本文は未実測。`sakurascript` の成功が `OK` なので型をそろえる。SSP で測って違えば SSP の文にそろえる | 8.2・8.3・9.1・10.1 |
| 8 | `target` の英字の大小 | **大小を区別しない**（ASCII の範囲だけ。前後の空白は削らない） | 同じ MCP の `get_log` の `log_type` の扱い（`Kind::from_log_type`）にそろえる。SSP で測って違えば改める | 1.2 |
| 9 | `\![reload,descript,…]` の中で areka に無い種類（`headline`・`plugin`・`language`・`calendar.*`）と知らない語 | **その語だけ `warn!` を 1 件残して飛ばし、残りの語は読み直す** | areka にヘッドライン・プラグイン・言語ファイル・カレンダーは無い。黙って捨てない（ログの無い失敗の禁止） | 7.4・7.5 |
| 10 | 台本の `\![reload,makoto]` | **本 spec では拾わない**（今と同じく「誰も拾わない `\!`」のまま）。拾うのは `makoto-reload-directives` | 同 spec の brief が `\![reload,makoto]` の消費者と台帳の行を持つ。本 spec は MCP の `target: "makoto"` の縮退の口だけ | 10.3 |

## Boundary Context

- **In scope**: 5 対象（ghost／shiori／shell／balloon／descript）の読み直し・台本の `\![reload,ghost／shiori／shell／balloon／descript]`・`\![reload,descript,パラメータ]`・`\![reloadsurface]` の受け取り・MCP の `reload` の中身（対象の振り分け・答え）・makoto の縮退の口（MCP だけ）・読み直しの失敗をエラーログへ残すこと・`\!` の受け取り手の台帳と網羅台帳の行・決定論テストと実機確認・行数の上限に近いファイルを先に分けること。
- **Out of scope**: makoto の本体と台本の `\![reload,makoto]`（`makoto-dll-host`・`makoto-reload-directives`）・`\![reload,aigraph]`・別のゴースト／シェル／バルーンへの切替（既存）・ファイルの変更を見張って自動で読み直すこと・ほかの 9 本の MCP ツールの中身・MCP の案内文（`crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS`）の「not implemented yet」の 1 文を消すこと（`mcp-strict-errors`）・橋の返事の上限（`REPLY_WAIT`）の変更・右クリックメニューに「読み直し」を足すこと・`doc/ssp-mcp/areka-tools.md` の受け取り手の数の 1 行（数を書かない形への直しごと `check-script-arg-checks`／`mcp-strict-errors` に預ける）。
- **Adjacent expectations**:
  - `mcp-tool-entrances` の振り分け（`dispatch`）が `ghost_name` を解決し、0 体・名前違いを `handle` の手前で `NG:` に終える。本 spec はこれに頼り、変えない。
  - 既存のゴースト切替（`request_ghost_switch`）とシェル・バルーン切替（`request_skin_switch`）の判定・差し替え・失敗時の扱いを、読み直しはそのまま使う。既存の切替・終了・ネットワーク更新の読み直しの振る舞いは 1 つも変えない（roadmap の保存義務）。
  - `mcp-log-history` が `warn` 以上の記録を error 種別に振り分ける。本 spec の失敗の記録はこの振り分けに乗る形で残す。
  - `mcp-author-tools` の `check_script` は台帳を引くので、台帳に行を足せば `\![reload,…]` を「知らない命令」と答えなくなる。
  - `makoto-reload-directives` が `\![reload,makoto]` を作るときに、本 spec の MCP の縮退の口を同じ操作へつなぐ（どちらが先に着地しても後着がつなぐ）。

## Requirements

### Requirement 1: 2 つの入口と対象の振り分け

**Objective:** As a ゴーストの作者・AI エージェント, I want 台本と MCP のどちらからでも同じ読み直しが呼べること, so that SSP と同じ手順で、書き換えた結果を確かめられる

#### Acceptance Criteria

1. When MCP の `reload` が起動中のゴーストへ解決され、`target` が `ghost`・`shiori`・`shell`・`balloon`・`descript` のどれかである, the areka shall その対象の読み直し（要件 3〜7）を、台本の同じ綴りと同じ操作として行う（`descript` は `\![reload,descript]` と同じく、すべての descript.txt を対象にする）。
2. The areka shall `target` を英字の大小を区別せずに（ASCII の範囲で）照らし、前後の空白は削らない。
3. If `target` が 6 語（ghost／shiori／shell／balloon／makoto／descript）のどれでもない（空の文字列を含む）, the areka shall 本文 `NG:Unknown target (ghost / shiori / shell / balloon / makoto / descript)`（`isError: true`）で答え、何も読み直さない。
4. When ゴーストの台本の再生が `\![reload,ghost]`・`\![reload,shiori]`・`\![reload,shell]`・`\![reload,balloon]`・`\![reload,descript]`・`\![reload,descript,パラメータ]`・`\![reloadsurface]` のどれかに達した, the areka shall その対象の読み直しを、MCP の同じ `target` と同じ操作として行う（`\![reloadsurface]` は `\![reload,shell]` と同じ）。
5. If 台本の `\![reload,…]` の第 1 引数が上の綴りのどれでもない（`aigraph`・`makoto` を含む）, the areka shall それを拾わず、今と同じく「誰も拾わない `\!`」として扱う。
6. The areka shall 読み直しを始めるたびに、対象・出どころ（台本か MCP か）・ゴーストを添えた `info!` を 1 件残す。

### Requirement 2: 重なりと断るとき

**Objective:** As a AI エージェント, I want 読み直しを始められない場面ではそれが分かる答えと記録が返ること, so that 何も起きなかったことに気付ける

#### Acceptance Criteria

1. If 読み直しを頼まれたとき、ゴーストの切替・シェルかバルーンの切替・別の読み直し・ネットワーク更新・インストールのどれかが進行中である, the areka shall 新しい読み直しを重ねずに断り、理由を添えた `warn!` を 1 件残す。
2. If 読み直しを頼まれたとき、areka が終了の途中である, the areka shall 読み直しを始めない（MCP の返事は既存の「終了の途中」の答えに任せる）。
3. When 断った読み直しが MCP から来たものである, the areka shall 本文 `NG:Cannot reload now (another switch, reload or update is in progress)`（`isError: true`）で答える。
4. While ゴーストが台詞を再生している, when ghost または shiori の読み直しを頼まれた, the areka shall 既存のゴースト切替と同じく、そのトークが終わってから読み直しを始める。
5. While ゴーストが台詞を再生している, when shell・balloon・descript の読み直しを頼まれた, the areka shall 既存のシェル・バルーン切替と同じく、台詞の切れ目で差し替える。

### Requirement 3: ゴーストの読み直し（`ghost`）

**Objective:** As a ぱすたさんを作る開発者, I want ゴースト一式を読み直しても、起動の挨拶などの知らせが出ないこと, so that 辞書・シェル・バルーンの書き換えを、終了と起動をやり直さずに確かめられる

#### Acceptance Criteria

1. When ghost の読み直しを始めた, the areka shall 今のゴーストを同じフォルダのゴーストとして降ろして起こし直し、ゴーストのフォルダの辞書・descript.txt・シェル・バルーンをディスクから読み直す（自分自身へのゴースト切替と同じ道を通る）。起動の引数で直に指したゴースト（ゴーストの一覧に載らないもの）も、今のゴーストのフォルダをそのまま指して読み直す。
2. The areka shall ghost の読み直しで、降ろす側に `OnGhostChanging`・`OnClose` を送らない。
3. The areka shall ghost の読み直しで、起こす側に `OnBoot`・`OnGhostChanged`・`OnFirstBoot` などの起動の知らせ（台詞を求める GET）を 1 つも送らない。
4. The areka shall ghost の読み直しで起こし直した SHIORI へ、ふつうの起動と同じ情報の通知（`OnInitialize`・`basewareversion` など環境を伝える NOTIFY と、`username` などのリソースの問い合わせ）を送る。
5. The areka shall ghost の読み直しの後、窓を、同じゴーストを起動し直したときと同じ位置の記憶に従って並べる。
6. The areka shall 既存のゴースト切替（`\![change,ghost]`・メニュー）とネットワーク更新の読み直しが送る知らせを、本 spec の前と同じに保つ（`OnGhostChanged`・更新の起動の知らせ・`OnBoot` の Ref6=halt）。
7. If ghost の読み直しで、ゴーストを起こせなかった, the areka shall 既存のゴースト切替の失敗と同じく既定のゴーストへ戻し（既定も起きなければ既存どおり致命）、起こせなかった理由を `error!` で残す。

### Requirement 4: SHIORI だけの読み直し（`shiori`）

> 2026-10-18 の要件ディスカッションで確定（裁定 1）: 本 spec に含め、ghost の読み直しの別名にはしない。シェルと窓を残したまま SHIORI だけを読み直す。

**Objective:** As a ぱすたさんを作る開発者, I want シェルと窓を残したまま SHIORI だけを読み直せること, so that 辞書の書き換えを、窓の閉じ開きなしに素早く効かせられる

#### Acceptance Criteria

1. When shiori の読み直しを始めた, the areka shall 今の SHIORI を unload して、同じゴーストの SHIORI を load し直す（結果として辞書ファイルをすべて読み直す）。
2. The areka shall shiori の読み直しの間、キャラクターとバルーンの窓を閉じず、表示中のシェルとバルーンを読み直さない。
3. The areka shall shiori の読み直しで、SHIORI イベント（`OnClose`・`OnBoot`・`OnGhostChanged` など台詞を求める GET）を送らず、load し直した SHIORI へは要件 3.4 と同じ情報の通知を送る。
4. If load し直しに失敗した, the areka shall 要件 3.7 と同じく既定のゴーストへ戻し、失敗の理由を `error!` で残す。

### Requirement 5: シェルの読み直し（`shell`・`\![reloadsurface]`）

**Objective:** As a ゴーストの作者, I want 今のシェルを同じシェルのまま読み直せること, so that 差し替えた画像と定義を、シェルを切り替え直さずに確かめられる

#### Acceptance Criteria

1. When shell の読み直しを始めた, the areka shall 今のシェルと同じフォルダのシェルを、ディスクから読み直して差し替える（画像・surfaces.txt・シェルの descript.txt を含む）。
2. The areka shall shell の読み直しで、SHIORI を読み直さず、ゴーストの記憶と会話の状態を保つ。
3. The areka shall shell の読み直しで、`OnShellChanging`・`OnShellChanged` を送らず、前回のシェルの記憶（`LastShell`）を書き換えない。
4. If shell の読み直しで、新しいシェルを組めなかった, the areka shall 今のシェルを表示したまま保ち、組めなかった理由を `error!` で残す。

### Requirement 6: バルーンの読み直し（`balloon`）

**Objective:** As a バルーンの作者, I want 今のバルーンを同じバルーンのまま読み直せること, so that 書き換えたバルーンの設定と画像を、会話を止めずに確かめられる

#### Acceptance Criteria

1. When balloon の読み直しを始めた, the areka shall 今のバルーンと同じフォルダのバルーンをディスクから読み直して差し替え、バルーンを選び直さない。今のバルーンがバルーンの一覧の外（起動の引数・ゴーストの descript の指定・同梱のもの）でも、今のバルーンのフォルダをそのまま指して読み直す。
2. The areka shall balloon の読み直しで、SHIORI を読み直さず、ゴーストの記憶と会話の状態を保つ。
3. The areka shall balloon の読み直しで、`OnBalloonChange` を送らず、前回のバルーンの記憶（`LastBalloon`）とバルーンの決め方（どこから選んだか）を書き換えない。
4. If balloon の読み直しで、新しいバルーンを組めなかった, the areka shall 今のバルーンを表示したまま保ち、組めなかった理由を `error!` で残す。

### Requirement 7: descript の読み直し（`descript`）

**Objective:** As a ゴーストの作者, I want descript.txt の書き換えを、SHIORI を読み直さずに効かせられること, so that 名前や配置の設定を手早く確かめられる

#### Acceptance Criteria

1. When `\![reload,descript]` または MCP の `target: "descript"` で読み直しを始めた, the areka shall 今のゴースト・今のシェル・今のバルーンの descript.txt をすべて読み直す。
2. When `\![reload,descript,パラメータ]` で読み直しを始めた, the areka shall パラメータを半角スペースで区切った語のうち `ghost`・`shell`・`balloon` に当たるものの descript.txt だけを読み直す（同じ語が重なっても 1 度だけ）。
3. The areka shall descript の読み直しの後、areka が動作中に使う descript.txt の値（ゴーストの名前など・シェルとバルーンの配置の値など）を、読み直したファイルの値に替え、SHIORI を読み直さず、SHIORI イベントを送らない（動作中に使い続ける値の一覧は設計で洗い出し、一覧の値は 1 つ残らず替える。起動のときにだけ読む値＝次の起動で読み直される値（kanade が `OnBoot` の Ref0 にだけ使うシェルの名前など）は一覧に入れない）。
4. If パラメータに areka に無い種類（`headline`・`plugin`・`language`・`calendar.skin`・`calendar.plugin`、および `calendar skin`・`calendar plugin` の表記）が含まれる, the areka shall その語ごとに「areka には無い種類」と分かる `warn!` を 1 件残して飛ばし、残りの語の読み直しを続ける。
5. If パラメータに知らない語が含まれる、またはパラメータに読み直せる語が 1 つも無い, the areka shall 知らない語ごとに `warn!` を 1 件残し（語が 1 つも無いときはその旨を 1 件）、読み直せる語だけを読み直す。
6. If descript.txt を読めなかった, the areka shall その対象の今の値を保ち、読めなかったファイルと理由を `error!` で残す。

### Requirement 8: MCP の答え

**Objective:** As a AI エージェント, I want `reload` の答えを受け取った時点で読み直しが終わっていること, so that 続けて呼ぶツールが読み直した後のゴーストに届く

#### Acceptance Criteria

1. When MCP の `reload` で始めた読み直しが終わった, the areka shall その時点で返事を返す（始めた時点では返さない）。終わった時点とは、ghost・shiori はゴーストが起動を終えて定常に入った時点（既定のゴーストへ戻ったときは戻った時点）、shell・balloon・descript は差し替え（または値の入れ替え）を終えた時点とする。
2. When 読み直しが成功した, the areka shall 本文 `OK`（`isError: false`）で答える。
3. If 読み直しが失敗した（要件 3.7・4.4・5.4・6.4・7.6）, the areka shall 本文 `NG:Failed to reload. Check the error log with get_log tool (log_type=error)`（`isError: true`）で答える。
4. While 読み直しの返事を待たせている, the areka shall ほかの MCP のツールの要求を止めずに受け付けて答える（宛先のゴーストが降りている間に届いた要求は、既存の振り分けどおり「起動中でない」の答えになってよい）。
5. If 読み直しが橋の返事の上限（10 秒）のうちに終わらなかった, the areka shall 読み直しを途中でやめずに最後まで行い、終わったときの結果を記録に残す（返事は橋の時間切れの答えになる）。
6. The areka shall `reload` のどの入力・どの場面でも panic しない。

### Requirement 9: 失敗をエラーログへ残す

**Objective:** As a AI エージェント, I want 読み直しの失敗が `get_log` のエラーログで読めること, so that SSP と同じく、何が壊れているかを後から確かめられる

#### Acceptance Criteria

1. The areka shall 読み直しの失敗と、読み直しを断ったこと（要件 2.1・3.7・4.4・5.4・6.4・7.4〜7.6・10.2）を、MCP の `get_log`（`log_type=error`）で読める記録（`warn` 以上）として残す。
2. The areka shall 失敗の記録に、対象・ゴースト・失敗したファイルか理由を添え、`get_log` の `ghost_name` で読み直したゴーストを指したときにもその記録が出るようにする（記録の名前を `[SYSTEM]` にしない）。
3. The areka shall 読み直しのどの失敗も、記録を残さずに黙って終えない。

### Requirement 10: makoto の縮退の口

**Objective:** As a `makoto-reload-directives` の実装者, I want MCP の `target: "makoto"` の口が決まった形で置かれていること, so that makoto を作るときにこの口を本物の操作へつなぐだけで済む

#### Acceptance Criteria

1. When MCP の `reload` の `target` が `makoto` である, the areka shall 本文 `NG:Reloading makoto is not supported yet`（`isError: true`）で答え、何も読み直さない。
2. When 要件 10.1 の答えを返した, the areka shall 「makoto の読み直しはまだ無い」と分かる `warn!` を 1 件残す。
3. The areka shall 台本の `\![reload,makoto]` を本 spec では拾わない（暫定の裁定 10）。
4. When 本 spec を完了するとき（`/kiro-complete`）, the areka shall `.kiro/specs/areka-P0-makoto-reload-directives/brief.md` の末尾に「`mcp-reload` からの申し送り（日付）」の節を足し、⑴ 縮退の口の場所（ファイルと関数）、⑵ 口をつなぐときに書き換えるテスト、⑶ 読み直しの操作の入口（makoto も同じ入口・同じ重なりの判定に載せられること）を書く。この追記は spec 文書の変更で、要件 13.1 のソースファイルに数えない。

### Requirement 11: 台帳

**Objective:** As a areka の開発者, I want 新しく拾う `\!` が台帳に載り、正典の網羅台帳も実態と合っていること, so that `check_script` と網羅の集計が正しく答える

#### Acceptance Criteria

1. The areka shall `\!` の受け取り手の台帳（`ConsumerLedger::canonical`）に、本 spec が拾う組（`reload` と第 1 引数 `ghost`・`shiori`・`shell`・`balloon`・`descript`、および `reloadsurface`）を登記し、台帳と実際に拾う側が一致することを固定するテスト（`consumer_ledger_agreement_tests.rs`）に見本を足す。
2. The areka shall 台帳の登記を、`makoto-reload-directives` が後から `reload` と第 1 引数 `makoto` の組を足せる形にする（本 spec の登記がその組を塞がない）。
3. When 台帳に要件 11.1 の組を登記した, the areka shall `check_script` が `\![reload,ghost]` などを「知らない命令」と答えなくなり、`\![reload,makoto]`・`\![reload,aigraph]` は今と同じく「知らない命令」と答えることを確かめる。
4. The areka shall 網羅台帳（`doc/ukadoc-coverage/ledger/sakura-script.toml`）の `reload,ghost`・`reload,shell`・`reload,balloon`・`reload,descript,パラメータ`・`reload,shiori` の行を、実装に合わせた状態と `owner = "areka-P0-mcp-reload"` に直す（`reload,balloon` の古い `owner` も直す）。別名の行（`reload,descript`・`reloadsurface`）は写像先と同じ扱いにする。
5. The areka shall 網羅台帳の `reload,makoto`・`reload,aigraph` の行を変えない。

### Requirement 12: 決定論テストと実機確認

**Objective:** As a areka の開発者, I want 読み直しの判断の分岐がすべてテストで固定され、実機でも確かめられていること, so that 既存の切替を崩さずに読み直しを足せる

#### Acceptance Criteria

1. The areka shall 次の判断の分岐をそれぞれ決定論テストで固定する: ⑴ `target` の 6 語と知らない語・空・大小違いの振り分け、⑵ 台本の綴り（`\![reloadsurface]` を含む）から対象への写し、⑶ 重なりで断る各場面（ゴースト切替中・シェルかバルーンの切替中・読み直し中・更新中・インストール中）、⑷ makoto の縮退の答えと記録、⑸ `\![reload,descript,パラメータ]` の語の振り分け（`ghost`・`shell`・`balloon`・areka に無い種類・知らない語・重なり・語が無い）、⑹ MCP の答えが読み直しの終わりまで返らず、成功で `OK`・失敗で `NG:…` になること。
2. The areka shall ghost の読み直しで起動の知らせ（台詞を求める GET）が 1 つも送られず、情報の通知は送られることを、起動の根を選ぶ所の決定論テストで固定する。
3. The areka shall 既存の起動の由来（`Plain`・`ChangedFrom`・`Halted`・`Updated`）が選ぶ知らせが変わらないことを、既存のテストを 1 本も変えずに緑のまま保って確かめる。
4. The areka shall 既存の終了経路・ゴースト切替・シェルとバルーンの切替・ネットワーク更新の読み直しの決定論テストを 1 本も落とさない。
5. The areka shall shiori の読み直しで窓を閉じず、SHIORI を unload して load し直し、台詞を求める GET を送らないことを決定論テストで固定する。
6. The areka shall `NG:not implemented yet` を期待していた `reload_tests.rs` のテストを、新しい振る舞いの期待へ書き換える。
7. When 実装が済んだ, the areka shall 実機で emo2（または複製したぱすたさん）を起こし、⑴ 辞書を書き換えて MCP の `reload`（`target: "ghost"`）→ 書き換えが効き、起動の挨拶が出ないこと、⑵ シェルの画像を差し替えて `target: "shell"` → 差し替えが見えること、⑶ 壊した辞書で読み直したときに `get_log`（`log_type=error`）に失敗が載ること、⑷ `target: "makoto"` の縮退の答え、⑸ `target: "shiori"` で窓が閉じずに辞書が効くこと、を確かめ、結果を本 spec の `verification/` に残す。実機の根・検体はワークツリーの `target\` の下に置く。

### Requirement 13: 触る範囲

**Objective:** As a 同じウェーブで並走する spec の実装者, I want 本 spec が触るファイルが決まっていて、約束したファイルに触らないこと, so that 互いのファイルを奪い合わずに並走できる

#### Acceptance Criteria

1. The areka shall 本 spec で変更するソースファイルを、MCP の `reload` のファイル（`crates/areka/src/mcp/reload.rs`・`reload_tests.rs`）・新しい読み直しの受け口と入口のファイル（`crates/areka/src/emo2_boot/` の下の新しいファイルと兄弟のテスト）・`emo2_boot/consumer_ledger.rs` とそのテスト・`emo2_boot/mod.rs`・`emo2_boot/ghost_switch.rs`・`emo2_boot/shell_balloon_switch.rs`（とその兄弟）・差し替えの相 `emo2_boot/frame/switch.rs` とその兄弟のテスト（読み直しのときに `OnShellChanged`／`OnBalloonChange` と記憶の書き込みを止める分かれ目）・descript の読み直しでゴーストの名前を替える口のための `crates/areka-ghost/src/runtime.rs`・`crates/areka/src/ghost_session.rs`（要件 4 と無関係に要る）・要件 13.5 のための seriko の `crates/areka-seriko/src/actor.rs`（名前の定数 1 つ）・kanade の `crates/areka-kanade/src/change.rs`・`crates/areka-kanade/src/schedule/boot.rs`・網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` に限る（ほかを触る要が出たら止めて報告する）。
2. The areka shall 要件 4 を、変更するソースファイル `crates/areka-ghost/src/runtime.rs` と `crates/areka/src/ghost_session.rs`（13.1 に挙げたもの）の中で作り（`mcp-kanade-tools` が `runtime.rs` に 1 行足すので、設計で照合する）、要件 13.3 の kanade のファイルには触らない形（窓を残して実行系を組み直す形など）で作る。触らずに作れないと分かったら止めて報告する。
3. The areka shall 次のファイルを変えない（変える行 0）: kanade の `msg.rs`・`actor.rs`・`schedule/mod.rs`・`schedule/change.rs`（`mcp-kanade-tools` の持ち物）・`crates/areka-mcp/` の下の全ファイル（`handler.rs`・`REPLY_WAIT` を含む・`mcp-receiver-bugs` の持ち物）・`crates/areka/src/mcp/mod.rs`・`crates/areka/src/mcp/resolve.rs`・`doc/ssp-mcp/areka-tools.md`・すべての `Cargo.toml`（新しい依存 0）。
4. The areka shall 1 ファイル 1,000 行以下の目安を、変更するすべてのファイルで守る。行を足す前に、`consumer_ledger.rs` のファイルの中のテストを兄弟のテストファイルへ出し、`ghost_switch.rs` には同じゴーストへ載せ替える入口を足さずに新しい兄弟のファイルへ置く（`emo2_boot/mod.rs` へ足すのは `mod` の宣言・台本の受け口の結線・系の登録だけ）。
5. Where `consumer_ledger.rs` に触る, the areka shall 台帳の `bind` の行の名前を、拾う側（seriko）と共有する定数から引く形へ直す（完了 `mcp-author-tools` からの申し送り。seriko のファイルに触れないウェーブなら、次に台帳へ触る spec へ送り、その旨を記録する）。
