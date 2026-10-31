# Requirements Document

> 本文の実測は **2026-10-31・本ブランチ**（main `da020c10`）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> brief が「要件の段の議題」とした 3 点と、要件を書く中で見つけた 1 点は、**推奨案で書き**、要件 10 に議題として並べた（要件の討議で決める）。要件 1〜9 の「（議題 N）」はその議題に当たる所。

## Project Description (Input)

**誰の何が困っているか**: exe の隣に `initial_install.nar` を置いてゴーストを同梱して配る人と、空の試験の根に `--install a.nar` を渡して「入れる → 起こす → MCP で確かめる」を回したい開発者・AI のエージェント。置き場にゴーストが 1 体も無いと、areka は入れる物を入れずに「ゴーストが見つかりません」で止まる。今は試験の根に既定のゴースト（emo2）を先に写しておく必要がある。

**今の状態**: 完了 `startup-install-option` は、`--install` と `initial_install.nar` を最初のゴーストが定常に着いた後に入れる。起動の解決（根 → ゴースト → バルーン）がゴーストを見つけられないと、既定のゴーストの台詞の道で伝えて終了コード 1 で終わり、入れる物は入れなかったと記録（warn `startup_install_not_submitted reason=boot_stopped`）に残るだけ。

**何を変えるか**: SSP の `--install`「インストール処理を行った後起動を続行する」と同じく、`initial_install.nar` と `--install` の書庫を、起動の解決がゴーストとバルーンを選ぶ前に順に入れてから、入れた後の置き場で起動の解決を続ける。置き場が空でも、入れたゴーストで起動できる。入れた結果は今と同じ語で記録に残り、`deletesource` は入った書庫だけを消し、読み取りだけの起動では今どおり断る。

> 起票: 2026-10-30 `startup-install-option` の完了の棚卸（同 spec の要件 10.2）。測り直し: 2026-10-31 棚卸㉛。区分 A・版 v0.1.0（版の門に数える）。ウェーブ C14 の ②。正典は SSP ヘルプ「起動オプション一覧」の `--install` と `initial_install.nar`（https://ukatech.shillest.net/ukadoc-ssp/functions/option/ ・ukadoc MCP の `ssphelp:functions/option`）。

## Introduction

### 誰が困っているか

- **ゴーストを areka に同梱して配る人・その利用者**: 置き場が空の areka の隣に `initial_install.nar` を置いても、最初の起動で「ゴーストが見つかりません」で止まり、同梱したゴーストが入らない。SSP では入れてから起動を続ける。
- **自動の試験を回す開発者・AI のエージェント**: 空の試験の根から「入れる → 起こす → MCP で確かめる」を 1 行で回せない。今は根に emo2 を写し、emo2 を起こしてから入れ、`callghost` で切り替える回り道をしている（`crates/areka/tests/startup_install_real_run.rs`）。ぱすたの手順書 `doc/pasta-real-run.md` も、新しい書庫は起こした areka の窓へ投げ込む手順のまま。

### いま何が起きているか（2026-10-31 のコードの読み直し）

| 物 | 今 | 定義点 |
|---|---|---|
| 起動の持ち物 | `--install` の並び（絶対パスにした物）・`callghost`・`deletesource`・exe のフォルダを持つ。起動の解決の前に作られ、窓のアプリの資源へ移される | `crates/areka/src/install/startup.rs` の `StartupInstall`・`crates/areka/src/main.rs` の `main` |
| 入れる時点 | 最初のゴーストが定常に着いたとき 1 回だけ。`initial_install.nar` を先に 1 件、`--install` の並びを後に 1 件の依頼として積む | `install/startup.rs` の `on_steady`（`install/desk.rs` の `on_steady` から呼ばれる） |
| 起動の解決 | 根を決め、ゴーストを選び（位置引数 → `--ghost` → `--random` → 記憶の前回のゴースト → 置き場に 1 体だけならそれ → 既定の `emo2` → 無作為）、バルーンを選ぶ。根からゴーストまでを 1 つの関数で決める | `crates/areka/src/boot_config.rs` の `resolve_boot`・`crates/areka/src/boot_resolve.rs` |
| ゴーストが居ないとき | 「ゴーストが見つかりません。」を既定のゴーストの台詞の道で伝え、終了コード 1 で終わる。起動の持ち物は落とされ、warn `startup_install_not_submitted reason=boot_stopped` を 1 件残す | `main.rs`・`crates/areka/src/alert.rs`・`install/startup.rs` の `Drop` |
| 手続きの本体 | 書庫ごとに `OnInstallBegin` → 読み取りと検査 → `accept` の照合 → 利用条件 → 展開 → 記憶 → `OnInstallCompleteEx`／`OnInstallComplete` → 同梱を飛ばしたときの警告。外とのやりとりは 8 つの口（イベントを送る・今のゴーストの素性・利用条件の画面・ほかの所へ入れる・起動中のゴーストへの上書き・入れた物の記憶・警告の画面ほか）を通す | `crates/areka/src/install/procedure.rs` の `run_order`・`InstallPorts` |
| 口の今の中身 | イベント・今のゴーストの素性・上書き・入れた物の記憶は、窓のアプリと起動中のゴーストに問い合わせる。利用条件の画面と警告の画面は、窓のアプリの要らない素のダイアログ | `crates/areka/src/install/worker.rs` の `DeskPorts`・`alert::ask_yes_no`・`alert::warn_ok` |
| `accept` の照合 | `install.txt` の `accept` を、起動中のゴーストの `sakura.name` と `install.accept` に照らす。`accept` の無い追加シェル・追加ファイルは受け取れない | `crates/areka/src/install/judge.rs` の `judge_accept` |
| `callghost` | 依頼を扱い終えた後、並びの中で最後に入ったゴーストへ、メニューの「ゴースト」から選んだときと同じ道で切り替えを頼む | `install/startup.rs` の `decide_call_ghost` |
| 書庫の消去 | `deletesource` のとき `--install` の入った書庫だけ、`initial_install.nar` は入ったら必ず消す。消せなければ warn | `install/startup.rs` の `on_order_done` |
| 読み取りだけ | 依頼の口の先頭で断り、info `readonly_refused`（`what=install`・出どころの語）を残す。読み取りだけの印は窓のアプリの前からも読める | `crates/areka/src/install/mod.rs` の `submit`・`crates/areka/src/readonly.rs` の `is_on` |
| `bootghost` | `install.txt` の `bootghost` は読まれて捨てられる。起動の解決には何も渡らない | `crates/areka-nar` の install.txt の読み手 |
| 説明書 | 「入れ始めるのは、最初のゴーストが起動した後」「ゴーストが 1 体も起動しなかったときは、何も入れず」と書く | `dist/README.txt` の「--install と initial_install.nar」 |

### 正典（SSP）の位置づけ

SSP ヘルプ「起動オプション一覧」（https://ukatech.shillest.net/ukadoc-ssp/functions/option/ ）から逐語で引く:

- `--install NARファイル名`: 「起動中のSSPがある場合はインストール処理を起動中のほうに任せ、ない場合は自身でインストール処理を行った後起動を続行する。」
- `--option callghost`: 「--install と同時に使うと、インストール完了後にゴーストが自動で呼び出される。」
- `initial_install.nar`: 「SSP起動時にサイレント（確認ダイアログなし）で上書きインストールされる。インストール完了後、initial_install.narは自動的に削除される。」（2.7.52〜）
- ukadoc「install.txt」の `bootghost,ディレクトリ名`: 「type,packageのみ有効。initial_install.narまたはnar関連付けからSSPを起動した際に、起動すべきゴーストのディレクトリ名を指定する。」

areka の読み方:

- 「インストール処理を行った後起動を続行する」は、入れる時点が起動の続き（ゴーストを起こす）より前であり、ゴーストが居るかどうかで分けていないと読める（議題 1）。
- 起動の前にはゴーストが起動していないので、インストールのイベント（`OnInstallBegin` ほか）を受け取るゴーストが居ない。SSP がこのとき何を送るかは書かれていない（議題 2・SSP の MCP で測れる）。
- 「起動中の SSP に任せる」形は areka に無く、この spec でも作らない（範囲外）。
- `bootghost` は束ねた書庫（`type,package`）の持ち物で、`install-package-type` が読む。この spec は、入れた後の置き場から起こすゴーストを選ぶ所を持つだけで、`bootghost` は読まない（議題 4）。

### 先にある決め

- **2026-09-28 の裁定（`ghost-install` の要件の討議 議題 2・開発者）**: 「インストールと切り替えは異なるイベントです。『ゴーストが自分から切り替えに行く』挙動はありですが、arekaは主導しません。ゴーストが何も言わない場合、ユーザーが明示的にメニューから新しいゴーストを呼び出さない限り切り替わりません。」
- **2026-10-29 `startup-install-option` の要件の討議**: 議題 1＝`callghost` は起動の行を書いた利用者の明示の指示（裁定の「利用者が明示的に呼び出す」に当たる）。議題 3＝`initial_install.nar` でも利用条件の画面と警告の画面を今の手続きと同じに出す。議題 5＝`initial_install.nar` を先に・`--install` を後に別々の依頼にし、`callghost`・`deletesource` は `--install` と組んだときだけ効かせて `initial_install.nar` には効かせない。議題 2＝ゴーストが落ち着いた後だけ入れ、起動の前に入れる形は別の spec（＝この spec）にする。
- この spec は、議題 2 の「落ち着いた後だけ入れる」を改め、議題 1・3・5 の決めはそのまま受け継ぐ。完了 spec の文書は直さず、この spec が新しい決まりを持つ。

### 何を変えるか

起動の解決が根（置き場）を決めた後、ゴーストとバルーンを選ぶ前に、`initial_install.nar` と `--install` の書庫を今の手続きで順に入れ、全部を扱い終えてから、入れた後の置き場でゴーストとバルーンを選ぶ。`callghost` は、入れた後に切り替えるのではなく、最後に入ったゴーストを起動のゴーストに選ぶ指示として扱う。起動の前には受け取るゴーストが居ないので、インストールのイベントは送らない。最初のゴーストが定常に着いた後に入れる今の道は、起動の引数と `initial_install.nar` については使わなくなる（議題 1）。

## Boundary Context

- **In scope**: 起動の解決がゴーストを選ぶ前に `initial_install.nar` と `--install` を入れる道・起動の前に入れるときの手続きの口の中身（窓のアプリとゴーストの無い形）・入れた後の置き場での起動の解決・`callghost` を起動のゴーストの選び方として扱うこと・`deletesource` と `initial_install.nar` の消去を起動の前の道へ移すこと・`boot_stopped` の記録の見直し・読み取りだけのときの断りの確かめ・説明書 `dist/README.txt` の「--install と initial_install.nar」の行・決定論のテスト・実機の確かめ（空の根に `--install` で入れて MCP の `get_active_ghost_list`）。
- **Out of scope**: 起動中の areka へインストールを任せる仕組み（SSTP の受け口）・インストールの手続きそのもの（読み取りと検査・`accept` の照合の決まり・置き場・上書き・利用条件・警告の中身）の変更・`--install` と `--option` の値の読み方（完了 `startup-options-core`・`startup-install-option` の `cli/`）・切り替えの手続きそのもの・`callghost` を 2 体目として呼ぶ形（`ghost-call`）・`install.txt` の `bootghost` を読むこと（`install-package-type`）・使っている最中のフォルダへの上書きの守り（`install-live-target-hazards`）・メニュー・台本・窓への投げ込みからのインストール（今のまま起動の後に入れる）・`--option kiosk`。
- **Adjacent expectations**:
  - 完了 `startup-install-option`: 起動の持ち物・出どころの語（起動の引数・`initial_install.nar`）・結果の語・消去の決まり・`callghost` の判断の多くを受け継ぐ。同 spec の要件 2.1（定常の後に出す）・2.3（ゴーストへイベントを送る）・2.6〜2.8・4.1（定常の後に見る）・5（切り替えを頼む）・7.1（ゴーストが居ないときは入れない）は、この spec の要件 1・3・4・6 が置き換える（議題 1 の推奨案のとき）。
  - 完了 `readonly-mode`: 依頼の口の先頭の断りと記録を、起動の前の依頼にもそのまま当てる。新しい断りを作らない。
  - 完了 `ghost-install`・`nar-install`・`nar-install-hardening`・`file-drop`: 手続きと画面とイベントの中身はそのまま使い、起動の前にゴーストが居ないことで変わる所（イベントを送らない・受け取るゴーストが要る書庫・バルーンの記憶）だけを要件 2・3 で決める。
  - `install-package-type`（いつか）: `bootghost` を読んだら、要件 4 の「起動のゴーストの選び方」へ名前を渡す。この spec はその受け口の形を決めない（設計で、渡せる形にしておくかを決める）。
  - `ghost-call`・`multi-ghost-runtime`: `callghost` を 2 体目として呼ぶ形はそちら。この spec の `callghost` は起動の 1 体目を選ぶだけ。
  - ウェーブ C14 の約束（roadmap「ウェーブ編成」）: `crates/areka/src/ghost_session.rs` に触らない（③ `placement-measure-bake-once`）。`dist/README.txt` を触るのはこの spec だけ。`install/desk_overwrite_tests.rs` は ⑥ `actor-thread-log-capture` が触らない。③ は `boot_config.rs` と `install/` に触らない。`main.rs` は 1,000 行の上限（今 986 行）＝足す分は `main.rs` の外に置く（設計へ渡す制約）。

## Requirements

### Requirement 1: 起動の解決がゴーストを選ぶ前に入れる

**Objective:** As a ゴーストを同梱して配る人・開発者・AI のエージェント, I want 置き場が空でも、入れる物を入れてから起動が続いてほしい, so that 空の置き場から最初のゴーストを入れて起こせる

#### Acceptance Criteria

1. When areka が起動し、起動の解決が根（置き場）を決めた, the areka shall ゴーストとバルーンを選ぶ前に、`initial_install.nar` の依頼を先に、`--install` の並びの依頼を後に、別々の依頼として順に入れ、両方を扱い終えてから起動の解決の続き（ゴーストとバルーンを選ぶ）へ進む（読み取りだけのときは要件 7）。
2. The areka shall 置き場にゴーストが居るときも居ないときも、起動の引数と `initial_install.nar` を同じ時点（ゴーストを選ぶ前）に入れる（議題 1・推奨案）。
3. The areka shall 起動の引数と `initial_install.nar` を、1 回の走りで起動の前の 1 回だけ扱い、最初のゴーストが定常に着いた後・後のゴーストの切り替えや読み直しで入れ直さない。
4. The areka shall 起動の前の依頼を、窓へ落としたときに入る置き場と同じ置き場へ入れる。
5. When `initial_install.nar` が無く `--install` の値も無い, the areka shall 何もせずに起動の解決を続け、warn 以上の記録を残さない。
6. When `--offline-dump` で起動した, the areka shall 今の決まりのとおり、`--install` も `initial_install.nar` も扱わず、何も入れず何も消さない。
7. If 起動の解決が根を決められずに止まる, the areka shall 何も入れず、今と同じ止まり方（既定のゴーストの台詞の道・終了コード 1）で止まり、入れなかったことと理由と並びを記録（warn）に 1 件残す。
8. While 起動の前に入れている, the areka shall ゴーストとバルーンの窓を出さず（利用条件と警告の画面は要件 2 のとおり出す）、入れ終わった後に今と同じ起動を続ける（起動の前には待たせるトークも操作も無い）。

### Requirement 2: 起動の前でも今の手続きで入れる

**Objective:** As a 開発者・AI のエージェント, I want 起動の前に入れた結果が、窓へ落としたときと同じ決まりで決まってほしい, so that 起動の 1 行の試験の結果を、利用者の投げ込みの振る舞いと比べられる

#### Acceptance Criteria

1. The areka shall 起動の前の依頼を、窓へ落としたときと同じ手続き（読み取りと検査・展開・既に在るフォルダへの確かめない上書き・利用条件・同梱を飛ばしたときの警告）で扱い、「入れますか」の画面を新しく足さない。
2. When 起動の前に入れる書庫が利用条件（`terms.txt`／`terms.md`）を持つ, the areka shall 窓とゴーストがまだ無くても、利用条件の はい／いいえ の画面を今の手続きと同じに出し、「いいえ」なら利用条件を拒んだものとして入れない（議題 3）。
3. When 起動の前に入れる書庫で同梱の一部を飛ばした, the areka shall 今の手続きと同じ警告の画面を出す（議題 3）。
4. While 利用条件の画面を出せない設定で動いている（今の決まりで画面を出さないとき）, the areka shall 今の手続きと同じく、利用条件を持つ書庫を利用条件を拒んだものとして扱う。
5. If 起動の前に入れる書庫が、受け取るゴーストを要する物（`install.txt` の `accept` を持つ書庫・追加シェル・追加ファイル）である, the areka shall 起動の前には受け取るゴーストが起動していないので、今の照合の決まりで受け取るゴーストが合わないときと同じに扱って入れず、受け取るゴーストが居なかったことを記録に残し、並びの残りを続ける。
6. If 並びの中に在らない・NAR として読めない・入れられない書庫がある, the areka shall その書庫を今の手続きの失敗として記録に残し、並びの残りの書庫を続けて扱い、起動を止めない。
7. When 起動の前にバルーンの書庫が入った, the areka shall 置き場へ入れ、どのゴーストのバルーンの記憶にも書かず、書かなかったことを記録（info）に 1 件残す（起動の前には選んでいるゴーストが居ない）。

### Requirement 3: 起動の前に入れたことをゴーストへ送らない（議題 2）

**Objective:** As a ゴーストの作者, I want 起動の前のインストールで、ゴーストが受けるイベントの意味が崩れないでほしい, so that `OnInstall*` を起動中のゴーストの出来事として書ける

#### Acceptance Criteria

1. While 起動の前に入れている, the areka shall インストールのイベント（`OnInstallBegin`・`OnInstallCompleteEx`・`OnInstallComplete`・`OnInstallCompleteAll`・`OnInstallFailure`・`OnInstallRefuse`）と利用条件のイベント（`OnGhostTermsAccept`・`OnGhostTermsDecline`）を、どのゴーストへも送らない。
2. When 起動の前に入れた後にゴーストが起動した, the areka shall 起動の前のインストールの結果を、起動したゴーストへ後からイベントで送らず、起動したゴーストには今と同じ起動のイベントだけを送る（議題 2・推奨案）。
3. The areka shall 起動の後にメニュー・台本・窓への投げ込みから頼まれたインストールでは、今と同じイベントを今と同じ順で送る。

### Requirement 4: 入れた後の置き場で起動のゴーストを選ぶ（`callghost`）

**Objective:** As a 開発者・AI のエージェント, I want 入れたゴーストが起動のゴーストとして起きてほしい, so that 「入れる → 起こす → MCP で確かめる」を空の根から 1 行で回せる

#### Acceptance Criteria

1. When 起動の前の依頼を扱い終えた, the areka shall 入れた後の置き場を見てゴーストとバルーンを選ぶ（起動の前に入ったゴーストとバルーンも選べる物に入る）。
2. Where `--option callghost` が渡されていない, the areka shall 入れた後の置き場で今と同じ選び方（位置引数 → `--ghost` → `--random` → 記憶の前回のゴースト → 置き場に 1 体だけならそれ → 既定のゴースト → 無作為）でゴーストを選ぶ（空の置き場に 1 体だけ入ったときは、そのゴーストが起きる）。
3. Where `--option callghost` が `--install` と一緒に渡された, when `--install` の依頼でゴーストの書庫が 1 本以上入り、起動するゴーストの指定（位置引数・`--ghost`・`--random`）が無い, the areka shall 並びの中で最後に入ったゴーストを、記憶の前回のゴーストより先に、起動のゴーストに選ぶ。
4. When `callghost` で起動のゴーストを選んだ, the areka shall 選んだことと選んだゴーストを、出どころが起動の引数の `callghost` だと読める形で記録（info）に 1 件残す。
5. If `callghost` が渡されたが、起動するゴーストの指定（位置引数・`--ghost`・`--random`）もある, the areka shall 指定のほうでゴーストを選び、`callghost` を使わなかったことと理由を記録（info）に 1 件残す。
6. If `callghost` が渡されたが、`--install` の依頼でゴーストの書庫が 1 本も入らなかった（シェル・バルーン・追加ファイルだけ・全部が入らなかった）, the areka shall `callghost` を使わなかったことを記録（info）に 1 件残し、要件 4.2 の選び方でゴーストを選ぶ。
7. While 前回の起動がきれいに終わらなかった（記憶の前回のゴーストを使わない起動）, when `callghost` の相手が決まった, the areka shall `callghost` の相手を起動のゴーストに選ぶ（起動の行を書いた利用者の明示の指示・前回のゴーストの記憶を使うのではない）。起動のイベントの中身（前回がきれいに終わらなかったことの知らせ）は今のまま。
8. The areka shall `callghost` のとき、起動した後にゴーストの切り替えを頼まず、ゴーストを 2 体目として並べて起こさない。
9. The areka shall `initial_install.nar` で入ったゴーストを `callghost` の相手にせず、`initial_install.nar` によって起動のゴーストの選び方を変えない（2026-10-29 議題 5・議題 4・推奨案）。
10. The areka shall 起動の前に入れたゴーストを起動のゴーストに選んだときも、ほかの起動と同じに扱う（起動の後の記憶・起動のイベントは今のまま）。
11. If 起動の前に入れた後も置き場にゴーストが 1 体も無い, the areka shall 今と同じく「ゴーストが見つかりません」を既定のゴーストの台詞の道で伝えて終了コード 1 で止まる（入れた結果の記録は要件 6.1 のとおり残す）。

### Requirement 5: 入った書庫を消す（今の決まりを起動の前へ）

**Objective:** As a 開発者・AI のエージェント・配る人, I want 入った書庫だけが消え、入らなかった書庫が残ってほしい, so that 試験の後に失敗した書庫を調べられ、`initial_install.nar` が次の起動で入り直さない

#### Acceptance Criteria

1. Where `--option deletesource` が `--install` と一緒に渡された, when `--install` の依頼を扱い終えた, the areka shall 起動の解決の続きへ進む前に、終わり方が「入った」だった書庫のファイルだけを消し、それ以外（失敗・断った・利用条件を拒んだ・まだ扱っていない）の書庫のファイルを残す。
2. When 同じ書庫のパスが並びに 2 回以上あり、そのうち 1 回でも「入った」, the areka shall 並びの全部を扱い終えた後に 1 回だけ消す。
3. When `initial_install.nar` の終わり方が「入った」だった, the areka shall `deletesource` の有無によらず、起動の解決の続きへ進む前にそのファイルを消す。入らなかったときはファイルを残す。
4. When 書庫を消した, the areka shall 消したことと書庫のパスを記録（info）に 1 件残す。
5. If 入った書庫を消せない（ファイルが使われている・読み取り専用・既に無いなど）, the areka shall 消せなかったことと理由と書庫のパスを記録（warn）に残し、ほかの書庫の消去と起動を続ける（`initial_install.nar` は次の起動でまた入る）。
6. The areka shall `deletesource` と `initial_install.nar` の消去で、書庫のファイルのほかは何も消さない（書庫を置いたフォルダ・入れた先のフォルダを消さない）。
7. Where `deletesource` が渡されていない, the areka shall `--install` の書庫を、入っても消さない。

### Requirement 6: 結果と止まり方の記録

**Objective:** As a 開発者・AI のエージェント, I want 起動の前に入れた結果と、入らなかった理由が記録から読めてほしい, so that 試験の失敗を記録から追える

#### Acceptance Criteria

1. The areka shall 起動の前の依頼ごとに、依頼を受けたこと・出どころ（起動の引数・`initial_install.nar`）・書庫ごとの終わり方（入った・断った・利用条件を拒んだ・失敗した・まだ扱っていない）を、今の記録と同じ語で記録に残す。
2. The areka shall 起動の前の依頼の記録から、起動の前（ゴーストを選ぶ前）に入れたことが読めるようにする。
3. When 起動の前に入れた後に、起動の解決がゴーストを見つけられずに止まった, the areka shall 「入れなかった」の warn（今の `boot_stopped`）を残さない（入れた結果は要件 6.1 の記録に既に在る）。
4. The areka shall 起動の前に入れる道の失敗（依頼を扱えない・書庫を消せない・`callghost` を使えない）を、どれも理由の分かる記録（error か warn か info）に残し、黙って捨てない。
5. The areka shall 起動の前に入れる道の失敗を、メッセージボックスで告げない（今の手続きが出す利用条件と警告の画面はそのまま・ゴーストが居なくて止まるときは今の既定のゴーストの台詞の道）。

### Requirement 7: 読み取りだけの起動では入れない（確かめ）

**Objective:** As a 開発者・AI のエージェント, I want `--option readonly` と組んだとき、起動の前でも何も入らず何も消えないと確かめたい, so that 読み取りだけの試しの起動で置き場と書庫を汚さない

#### Acceptance Criteria

1. While 読み取りだけで動いている, the areka shall 起動の前の `initial_install.nar` と `--install` の依頼を、今あるインストールの断り（完了 `readonly-mode`）と同じに断り、何も入れない。
2. While 読み取りだけで動いている, the areka shall 断りの記録（`readonly_refused`）に、依頼の出どころ（起動の引数・`initial_install.nar`）が読める語を今と同じに出す。
3. While 読み取りだけで動いている, the areka shall `deletesource` があっても `--install` の書庫を消さず、`initial_install.nar` も消さない。
4. While 読み取りだけで動いている, the areka shall `callghost` を使わず、今ある置き場で今と同じ選び方でゴーストを選ぶ（置き場が空なら今と同じく「ゴーストが見つかりません」で止まる）。

### Requirement 8: 説明書に書く

**Objective:** As a 利用者・配る人・開発者, I want `dist/README.txt` で、いつ何が入り、どのゴーストが起きるかを知りたい, so that 空の置き場の areka に同梱するときと、試験の 1 行を書くときに迷わない

#### Acceptance Criteria

1. The areka shall 説明書の「--install と initial_install.nar」で、入れ始める時を「起動の途中、ゴーストを選ぶ前（ゴーストが 1 体も入っていなくても入れる）」に改め、「最初のゴーストが起動した後」「ゴーストが 1 体も起動しなかったときは、何も入れず」の文を外す。
2. The areka shall 説明書に、起動の前に入れたときはゴーストへインストールの知らせを送らないこと・受け取るゴーストを要する書庫（`accept` を持つ書庫・追加シェル・追加ファイル）は起動の前には入らないこと・入れ終わるまで窓が出ないことを書く。
3. The areka shall 説明書の `--option callghost` の文を、「最後に入ったゴーストを起動のゴーストに選ぶ（起動した後に切り替えない・2 体目としては起こさない）」「起動するゴーストの指定（位置引数・`--ghost`・`--random`）があればそちらが先」に改める。
4. The areka shall 説明書の `initial_install.nar` の文で、入れる時を起動の前に改め、置き場が空の areka に同梱したゴーストが最初の起動で起きること（置き場にそのゴーストだけのとき）を書く。
5. The areka shall 説明書の例を、空の置き場でも動く 1 行の例として残す（例: `areka.exe --install C:\work\my.nar --option callghost,deletesource`）。
6. The areka shall 説明書の readonly の「断る物」の文の意味を変えない（`--install` と `initial_install.nar` は入れず、消さない）。

### Requirement 9: テストで固定し、実機で確かめる

**Objective:** As a 開発者, I want 起動の前に入れる順・起動のゴーストの選び方・消す／消さない・断りを決定論のテストで固定したい, so that 後の変更が黙って入れる時点や起きるゴーストを変えない

#### Acceptance Criteria

1. The areka shall 空の置き場に起動の前にゴーストを 1 体入れると、起動の解決がそのゴーストを選ぶことを判定する決定論のテストを持つ。
2. The areka shall 起動の前の依頼が `initial_install.nar` → `--install` の順に、ゴーストを選ぶ前に扱われ、1 回の走りで 1 回だけであることを判定する決定論のテストを持つ。
3. The areka shall 並びの失敗（在らない・読めない・受け取るゴーストを要する書庫・利用条件を拒んだ）の後も並びの残りが扱われ、起動が止まらないことを判定する決定論のテストを持つ。
4. The areka shall 起動の前にインストールと利用条件のイベントをどのゴーストへも送らないことを判定する決定論のテストを持つ。
5. The areka shall `callghost` について、選ぶ／使わない（ゴーストが入った・入らない・起動するゴーストの指定がある・前回がきれいに終わらなかった・`callghost` なし・`initial_install.nar` だけ）の判断と、選ぶ相手（最後に入ったゴースト）を判定する決定論のテストを持つ。
6. The areka shall `deletesource` と `initial_install.nar` の消去について、終わり方ごとの消す／残す・同じパスが 2 回・消せないときを、起動の前の道で判定する決定論のテストを持つ。
7. The areka shall 根が決まらずに止まるとき・入れた後もゴーストが無くて止まるときの記録の形（要件 1.7・6.3）を判定する決定論のテストを持つ。
8. The areka shall 読み取りだけのとき、起動の前の依頼が断られ、記録に出どころが出て、書庫が消えず、`callghost` が使われないことを判定する決定論のテストを持つ。
9. The areka shall テストの根と書庫の写しをワークツリーの `target\` の下だけに置き、本物の書庫を消さない（写しを使う）。
10. The areka shall 実機で、`target\` の下の空の根（ゴーストが 1 体も無い置き場）と書庫の写しを使い、起動の 1 行（`--install`）で入れたゴーストが起動中になったこと（MCP の `get_active_ghost_list`）を、実機の試験の窓口（`real-run-mcp-kit`）の足場で確かめる。
11. The areka shall 今ある実機の試験（emo2 の居る根に `--install` と `callghost,deletesource` で入れる）の確かめ（入れたゴーストが置き場に入り、起動中になり、写しが消えた）が、起動の前に入れる形でも成り立つことを確かめる。

### Requirement 10: 要件の討議に回す議題（推奨案で書いた）

> brief の「要件の段の議題」の 3 点（議題 1〜3）と、要件を書く中で見つけた 1 点（議題 4）。どれも答えで作業が変わる。要件 1〜9 は推奨案で書いた。

**Objective:** As a 開発者, I want 推奨案で書いた所と、違う答えのときに変わる所が分かってほしい, so that 要件の討議で 1 件ずつ決められる

#### Acceptance Criteria

1. The areka shall 議題 1（ゴーストが居るときも起動の前に入れるか）を、要件の討議の決めのとおりに扱う。推奨案＝**案 A: 居ても居なくても起動の前に入れる**（SSP の「インストール処理を行った後起動を続行する」の読み・道が 1 本になる・起動するゴーストはまだ起きていないので、そのゴーストへの上書きが使っている最中のフォルダへの上書きにならない）。代わりに、ゴーストが居る起動でも、入れたことをゴーストへ送らなくなり（要件 3）、受け取るゴーストを要する書庫が起動の引数では入らなくなり（要件 2.5）、`callghost` が切り替えから起動のゴーストの選び方に変わる（要件 4）。案 B（ゴーストが居ないときだけ起動の前に入れ、居るときは今の定常の後の道を残す）なら、要件 1.2・1.3・3・4.3〜4.8・8 の文が変わり、2 つの道のどちらを通ったかの記録とテストが増える。
2. The areka shall 議題 2（起動の前に入れたことをゴーストへ送るか）を、要件の討議の決めのとおりに扱う。推奨案＝**どのゴーストへも送らない**（起動の前には受け取るゴーストが居ない・起動の後に送り直すと「起動中のゴーストの出来事」という `OnInstall*` の意味が崩れる・入ったゴーストは初めての起動のイベントで自分の初回を知れる）。SSP が `ssp.exe --install`（起動中の SSP が無いとき）でどのイベントを誰へ送るかは、SSP の MCP と `ghost\master\ssp_shiori_log.txt` で測れる（SSP を動かす＝打つ前に開発者の許しを取る）。測った答えが「起動したゴーストへ後から送る」なら、要件 3.2 とテスト 9.4 が変わる。
3. The areka shall 議題 3（利用条件の画面と警告の画面を、窓のアプリの前に出せるか）を、要件の討議の決めのとおりに扱う。コードの読み（2026-10-31）では、利用条件の画面と警告の画面は窓のアプリの要らない素のダイアログで出している（定義点は Introduction の表の「口の今の中身」）＝推奨案＝**今と同じに出す**（2026-10-29 議題 3 の決め「利用条件への同意は利用者の判断なので飛ばさない」を受け継ぐ・要件 2.2〜2.4）。出せないと設計で分かったときだけ、議題に戻す。
4. The areka shall 議題 4（`initial_install.nar` で入れたゴーストを起動のゴーストに選ぶか）を、要件の討議の決めのとおりに扱う。推奨案＝**選ばない（今の選び方のまま）**（2026-10-29 議題 5 の決め「`callghost` は `initial_install.nar` に効かせない」を受け継ぐ・空の置き場に同梱したゴーストだけなら、今の選び方の「置き場に 1 体だけならそれ」で起きる・どのゴーストを起こすかを書庫が決める形は、正典では束ねた書庫の `bootghost` の持ち物で `install-package-type` が持つ）。SSP が `type,ghost` の `initial_install.nar` の後にどのゴーストを起こすかは、議題 2 と同じ測りで確かめられる。答えが「選ぶ」なら、要件 4.9・8.4 とテスト 9.5 が変わる。
