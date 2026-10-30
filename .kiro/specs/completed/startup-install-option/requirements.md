# Requirements Document

> 本文の実測は **2026-10-29・本ブランチ**（main `932d2c30`）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 要件 10 は要件討議（2026-10-29）で確定した 5 つの決めと、完了の段での申し送り。要件 1〜9 の「（議題 N）」はその決めを指す。

## Project Description (Input)

**誰の何が困っているか**: 作ったばかりの NAR（例: ぱすたの `pasta_check release` が作る `pasta-in-windows.nar`）を areka に入れて確かめたい開発者と AI のエージェント。今は areka を立ててから窓へ投げ込むか、右クリックメニューの「インストール…」から選ぶしかない。自動の試験では窓への投げ込みを真似る（`SendInput` や画面の操作）ことになり、遅く、ほかの試験と画面を取り合う。「入れる → 起こす → MCP で確かめる」を 1 本の試験で回せない。ゴーストを areka に同梱して配りたい人も、起動のときに黙って入れる置き方を持たない。

**今の状態**: インストールの手続きは揃っている（完了 `ghost-install`・`file-drop`・`nar-install` ほか）が、入口はメニュー・台本・窓への投げ込みの 3 つだけ。起動オプションの読み手（完了 `startup-options-core`）は `--install` と `--option` の語 `callghost`・`deletesource` を「受けて、まだ扱わないと記録して無視する」だけ。exe の隣の `initial_install.nar` を見る所も無い。

**何を変えるか**: SSP と同じく、`areka.exe --install a.nar b.nar` で起動の後に並べた順に入れ、`--option deletesource` で入った NAR を消し、`--option callghost` で入れた後にそのゴーストを呼ぶ。exe の隣に置いた `initial_install.nar` を起動のときに入れ、入ったら消す。インストールの手続きそのものは窓への投げ込みと同じ道を通し、変えない。読み取りだけの起動（`--option readonly`）では、どれも行わない。

> 起票: 2026-10-18 棚卸㉗（区分 A・版 v0.1.0）。測り直し: 棚卸㉚（2026-10-24）。ウェーブ C12 の ⑤。正典は SSP ヘルプ「起動オプション一覧」の `--install`・`--option callghost`・`--option deletesource`・`initial_install.nar`（https://ukatech.shillest.net/ukadoc-ssp/functions/option/ ・ukadoc MCP の `ssphelp:functions/option`・2026-10-29 に読み直し）。

## Introduction

### 誰が困っているか

- **自動の試験を回す開発者・AI のエージェント**: NAR を作るたびに、areka を立てて画面の操作で投げ込む必要がある。画面の操作は排他の机で順を取るので遅く、ほかの試験を止める。起動の 1 行で入れられれば、実機の試験の窓口（`real-run-mcp-kit`）から「入れる → MCP で確かめる」を画面に触らずに回せる（下流 `pasta-in-windows-verification` の使い道）。
- **ゴーストを areka に同梱して配りたい人**: SSP では `ssp.exe` の隣に `initial_install.nar` を置けば、利用者の最初の起動で入る。areka にはその置き方が無い。

### いま何が起きているか（2026-10-29 のコードの読み直し）

| 物 | 今 | 定義点 |
|---|---|---|
| `--install`（短い形 `-i`） | 表にあるが「まだ扱わない」（値を受けて記録して無視） | `crates/areka/src/cli/parse.rs` の `TABLE`（`Kind::Unsupported`） |
| `--option callghost`・`deletesource` | 「まだ扱わない語」（記録して無視） | 同じファイルの `--option` の語の分岐（`kiosk` と並ぶ） |
| インストールの依頼の口 | 唯一の口。書庫を何本も並べた 1 件の依頼を受け、窓口の待ち行列に積む | `crates/areka/src/install/mod.rs` の `submit`・`InstallOrder` |
| 依頼の出どころの語 | `Menu`・`Script`・`WindowDrop` の 3 つ（記録の語だけ・手続きは分かれない） | 同じファイルの `InstallOrigin` |
| 読み取りだけのときの断り | `submit` の先頭で積まずに断り、`info!(readonly_refused)`（`what=install`・`origin`）を 1 件残す。ゴーストへは何も送らない | 同じ `submit`（完了 `readonly-mode`） |
| 手続きの一周 | 書庫ごとに `OnInstallBegin` → 読み取りと検査 → `accept` の照合 → 利用条件 → 展開 → `OnInstallCompleteEx`（応えが無ければ `OnInstallComplete`）。失敗は `OnInstallFailure`、宛先違いは `OnInstallRefuse`。2 本以上が全部入れば最後に `OnInstallCompleteAll` | `crates/areka/src/install/procedure.rs` の `run_order`・`archive_steps` |
| 書庫ごとの終わり方 | 入った・断った（宛先違い）・利用条件を拒んだ・失敗・終了が始まって途中でやめた、の 5 つ | 同じファイルの `ArchiveEnd` |
| 利用者へ確かめる段 | 「入れますか」の確かめは**無い**。既に在るフォルダへは確かめずに上書きする（下敷きを敷いて上書き）。出る画面は、書庫が `terms.txt`／`terms.md` を持つときの利用条件の はい／いいえ と、同梱の一部を飛ばしたときの警告の 2 つだけ | `procedure.rs` の `terms_step`・`crates/areka/src/install/worker.rs` の `ask_terms`・`warn_skipped`・`crates/areka-nar/src/install.rs` |
| 手続きが要るもの | 起動中のゴースト（宛先の根・知らせの送り先）。ゴーストが居なければ「閉じた」として途中でやめる | `worker.rs` の `ghost_facts`・`procedure.rs` の `run_archive` |
| 入れた後の切り替え | areka は始めない。ゴースト自身か、利用者のメニューの選択だけ（下の裁定）。切り替えの口はメニューと台本が使う 1 つ | `crates/areka/src/emo2_boot/ghost_switch.rs` の `request_ghost_switch` |
| 窓口が定常到達を聞く所 | 定常到達の知らせの末尾から呼ばれる口がある | `crates/areka/src/install/desk.rs` の `on_steady` |
| exe の場所 | 起動した exe の本当の場所（リンクを辿る）を 1 回だけ解いて覚える | `crates/areka/src/boot_config.rs` の `exe_location` |
| ゴーストが 1 体も無いとき | 起動の解決が「ゴーストが見つからない」として既定のゴーストの台詞の道（`alert`）で止まる。SSP の「何とかして下さい」の画面に当たる「入れてから起動を続ける」道は無い | `crates/areka/src/boot_resolve.rs` |
| 説明書 | `--install`・`callghost`・`deletesource` を「まだ扱わない」の 1 文に並べている。readonly の「断る物」の文に `--install` と `initial_install.nar` は無い | `dist/README.txt` の「■ 起動オプション」 |

### 正典（SSP）の位置づけ

SSP ヘルプ「起動オプション一覧」（https://ukatech.shillest.net/ukadoc-ssp/functions/option/ ）から逐語で引く:

- `--install NARファイル名`: 「NARをインストールする。起動中のSSPがある場合はインストール処理を起動中のほうに任せ、ない場合は自身でインストール処理を行った後起動を続行する。」「空白区切りで複数指定すると、複数個のNARを順番にインストールする。」
- `--option callghost`: 「--install と同時に使うと、インストール完了後にゴーストが自動で呼び出される。」
- `--option deletesource`: 「--install と同時に使うと、NARのインストールに成功した時にNARを削除する。」
- `initial_install.nar`: 「ssp.exeと同じフォルダにinitial_install.narという名前のNARファイルを配置しておくと、SSP起動時にサイレント（確認ダイアログなし）で上書きインストールされる。インストール完了後、initial_install.narは自動的に削除される。」（2.7.52〜）
- `--option readonly`: 「手動でのネットワーク更新・NARのインストールも行わない」「initial_install.narのインストールを行わない」。

areka の読み方:

- 「起動中の SSP に任せる」は areka に無い（areka はもともと起動中の areka や SSP に起動を任せない＝`dist/README.txt` の readonly の節）。この spec でも任せない（範囲外）。
- 「確認ダイアログなし」「上書き」は、areka の手続きに「入れますか」の確かめが無く、既に在るフォルダへは確かめずに上書きするので、新しく飛ばす段は無い。残る画面は利用条件と警告の 2 つ（議題 3）。
- `callghost` の「呼び出される」は、SSP の言葉では 2 体目として呼ぶ意味に読めるが、areka は今 1 体のゴーストしか動かせないので、この spec では切り替えとして扱う（議題 1）。2 体目として呼ぶ形は `ghost-call` が着地したときにそちらへ移す（要件 10.1）。

### 先にある裁定（2026-09-28 `ghost-install` の要件ディスカッション 議題 2・開発者）

> 「インストールと切り替えは異なるイベントです。『ゴーストが自分から切り替えに行く』挙動はありですが、arekaは主導しません。ゴーストが何も言わない場合、ユーザーが明示的にメニューから新しいゴーストを呼び出さない限り切り替わりません。」

`--option callghost` は、起動の行を書いた利用者が「入れた後に呼べ」と明示する指示である。要件討議（2026-10-29・議題 1）で開発者はこれを裁定の「利用者が明示的に呼び出す」に当たると認めた。この spec は、`callghost` を渡されたときだけ、メニューから選んだのと同じ切り替えの道を通す（要件 5）。`callghost` が無いときは、この裁定のとおり切り替えない。

### 何を変えるか

`areka.exe --install <NAR> [<NAR> …]` を、最初のゴーストが起動して定常に着いた後に、並べた順の 1 件の依頼として、窓への投げ込みと同じ手続きへ渡す。`deletesource` は入った NAR だけを消す。`callghost` は入れたゴーストへメニューと同じ道で切り替える。exe の隣の `initial_install.nar` も同じ手続きへ渡し、入ったら消す。読み取りだけの起動では、今ある断り（`submit` の先頭）でどれも断られる。記録には、依頼が起動の引数か `initial_install.nar` から来たことが読める語が出る。

## Boundary Context

- **In scope**: `--install`（短い形 `-i`・旧 `/i`）の値の受け取り（複数・順）・`--option callghost`・`--option deletesource` を扱う語にすること・起動の後に並びを 1 件の依頼として今の手続きへ渡すこと・`deletesource` で入った NAR だけを消すこと・`callghost` で入れたゴーストへ切り替えること（議題 1）・exe の隣の `initial_install.nar` を入れて、入ったら消すこと・依頼の出どころの語（起動の引数・`initial_install.nar`）が記録に出ること・読み取りだけのとき断られることの確かめ・ゴーストが居ないときに行わなかったことの記録（議題 2）・説明書 `dist/README.txt` の文・決定論のテスト・実機の確かめ。
- **Out of scope**: 起動中の areka へインストールを任せる仕組み（SSTP の受け口・二重起動の確かめの後）・`--ukagaka-link`・`--ssf`・インストールの手続きそのもの（判定・置き場・上書き・利用条件・警告・ゴーストへの知らせの中身と順）の変更・切り替えの手続きそのものの変更・NAR を作る道具（SSP の `--offline-tool`・ぱすたの `pasta_check`）・`callghost` を「2 体目として呼ぶ」形（`multi-ghost-runtime` の後）・ゴーストが 1 体も無いときに起動の解決の前に入れる道（議題 2・完了の段で別の spec として起票する＝要件 10.2）・`--option kiosk`（今までどおり「まだ扱わない」）。完了 spec は直さず、この spec が新しい決まりを持つ。
- **Adjacent expectations**:
  - 完了 `startup-options-core` の起動オプションの読み手に、`--install` の値と 2 つの語を足す。ほかの名前と語の扱いは変えない。`--offline-dump` と一緒の起動オプションは効かないという今の決まりも変えない。
  - 完了 `readonly-mode` が、インストールの依頼の唯一の口の先頭で読み取りだけの断りと記録を済ませている。この spec は新しい断りを作らず、起動の引数と `initial_install.nar` の依頼がその口を通ることと、記録に出どころが出ることを確かめる。
  - 完了 `ghost-install`・`nar-install`・`nar-install-hardening`・`file-drop` の手続き・イベント・警告をそのまま使う。
  - 完了 `ghost-shell-balloon-switch` の切り替えの道（メニューの「ゴースト」から選んだときの道）を、`callghost` のときに使う（議題 1）。
  - `install-live-target-hazards`（版に縛らない）: 使っている最中のフォルダへの上書きは `initial_install.nar` でも起こりうるが、その守りはこの spec の持ち物ではない。
  - `multi-ghost-runtime`（余力の枠）: `callghost` を「2 体目として呼ぶ」形に変えるなら、その後に回す。
  - 同じウェーブ C12 の約束（`crates/areka/src/ghost_session.rs` に触らない・`dist/README.txt` は `default-balloon-selfmade`・`bridge-store-user-root` と別の行）は設計の段で守る。

## Requirements

### Requirement 1: 起動の引数で入れる NAR と語を受け取る

**Objective:** As a 開発者・AI のエージェント, I want `areka.exe --install a.nar b.nar --option deletesource,callghost` の 1 行で入れる物と扱い方を渡したい, so that 画面に触らずに NAR を入れる試験が書ける

#### Acceptance Criteria

1. When 起動オプションに `--install`（短い形 `-i`・旧 `/i` を含む）と 1 つ以上の値がある, the areka shall 値の 1 つずつを入れる NAR のパスとして、書いた順に受け取る。
2. When `--install` の値に相対パスがある, the areka shall そのパスを、areka.exe を起動したときの今のフォルダからの道として読む（`--offline-dump` の相対パスと同じ読み方）。
3. When `--install` が値なしで渡された, the areka shall 入れる物が無いことを記録（warn）に 1 件残し、インストールの依頼を出さずに起動を続ける。
4. When `--option` の語に `callghost` または `deletesource` がある（大小を問わない・ほかの語とカンマで並べてもよい・`--option` を何回に分けて渡してもよい）, the areka shall その語を扱う語として受け取り、今の「まだ扱わない語」の記録をその語について残さない。
5. If `callghost` または `deletesource` が `--install` の値なしで渡された, the areka shall その語が効かないことを記録（warn）に 1 件残し、ほかは何も変えない（`initial_install.nar` にも効かない・議題 5）。
6. When `--install`・`callghost`・`deletesource` が `--offline-dump` と一緒に渡された, the areka shall 今の決まり（`--offline-dump` と一緒の起動オプションは効かず、効かなかったことを記録に残す）のとおりに扱い、何も入れず何も消さない。
7. The areka shall `--install` の値の書庫が在るか・読めるかで起動を止めず、在らない・読めない書庫をインストールの手続きの失敗として扱う（要件 2.4）。

### Requirement 2: 起動の後に並べた順に、投げ込みと同じ手続きで入れる

**Objective:** As a 開発者・AI のエージェント, I want 起動の引数の NAR が窓へ投げ込んだのと同じに入ってほしい, so that 起動の 1 行の試験の結果が、利用者の投げ込みの振る舞いと同じだと言える

#### Acceptance Criteria

1. When 起動の後に最初のゴーストが定常に着いた（起動の解決で既定のゴーストへ移ったときは、そのゴーストが着いたとき）, the areka shall `--install` の値の並び全部を、書いた順のまま 1 件のインストールの依頼として出す（1 回の走りで 1 回だけ・後のゴーストの切り替えや読み直しで出し直さない）。
2. The areka shall 起動の引数の依頼を、窓への投げ込み・メニュー・台本からの依頼と同じ手続き（`accept` の照合・利用条件・展開・上書き・ゴーストへの知らせ）で扱い、出どころで手続きを分けない。
3. When 起動の引数の依頼の書庫の手続きが進んだ, the areka shall 今の手続きと同じイベント（`OnInstallBegin`・`OnInstallCompleteEx`／`OnInstallComplete`・`OnInstallFailure`・`OnInstallRefuse`・利用条件の受諾と拒否・2 本以上が全部入ったときの `OnInstallCompleteAll`）を同じ順でゴーストへ送る。
4. If 並びの中に在らない・NAR として読めない・入れられない書庫がある, the areka shall その書庫を今の手続きの失敗として扱い（記録とゴーストへの `OnInstallFailure`）、並びの残りの書庫を続けて扱う。
5. The areka shall 起動の引数の依頼について、依頼を受けたこと・書庫ごとの結果（入った・失敗した・断った・利用条件を拒んだ・途中でやめた）が起動の引数の依頼のものだと、記録から読めるようにする（出どころの語はメニュー・台本・投げ込みの語と別。どの記録に語を載せるかは設計で決める）。
6. While 起動の引数の依頼を扱っている, the areka shall ゴーストのトークと利用者の操作を待たせず、投げ込みで入れているときと同じに動かす（議題 4）。
7. If 最初のゴーストが定常に着く前に終了が始まった, the areka shall 起動の引数の依頼を出さず、入れなかったことと並びを記録（warn）に 1 件残す。
8. When 手続きの途中で終了が始まった, the areka shall 今の手続きと同じに、まだ始めていない書庫を扱わずにやめる（記録は今の手続きのとおり）。

### Requirement 3: `--option deletesource` で入った NAR だけを消す

**Objective:** As a 開発者・AI のエージェント, I want 入った NAR だけが消え、入らなかった NAR が残ってほしい, so that 試験の後に失敗した書庫を調べられる

#### Acceptance Criteria

1. Where `--option deletesource` が `--install` と一緒に渡された, when 起動の引数の依頼が終わった（全部を扱った・または途中でやめた）, the areka shall 手続きの終わり方が「入った」だった書庫のファイルだけを消し、それ以外（失敗・宛先違いで断った・利用条件を拒んだ・まだ扱っていない）の書庫のファイルを残す。ただし、終了が始まった後に終わった依頼では、どの書庫も消さずに残し、消さなかったことを記録（warn）に 1 件残す（取り消せない消去を終了の後始末に持ち込まない・2026-10-29 設計の討議 議題 2）。
2. When 同じ書庫のパスが並びに 2 回以上ある, the areka shall そのうち 1 回でも「入った」なら、並びの全部を扱い終えるまで消さずに残し、その後に 1 回だけ消す。
3. When `deletesource` で書庫を消した, the areka shall 消したことと書庫のパスを記録（info）に 1 件残す。
4. If 入った書庫を消せない（ファイルが使われている・読み取り専用・既に無いなど）, the areka shall 消せなかったことと理由と書庫のパスを記録（warn）に残し、ほかの書庫の消去と起動を続ける（メッセージボックスを出さない・ゴーストへ何も送らない）。
5. Where `deletesource` が渡されていない, the areka shall 起動の引数の書庫を、入っても消さない。
6. The areka shall `deletesource` で、`--install` に書かれた書庫のファイルのほかは何も消さない（書庫を置いたフォルダ・入れた先のフォルダを消さない）。

### Requirement 4: exe の隣の `initial_install.nar` を入れて、入ったら消す

**Objective:** As a ゴーストを areka に同梱して配りたい人, I want areka.exe の隣に置いた `initial_install.nar` が最初の起動で入って消えてほしい, so that 利用者の手を借りずにゴーストを入れた状態で配れる

#### Acceptance Criteria

1. When 起動の後に最初のゴーストが定常に着いた, the areka shall 起動した areka.exe の本当の場所（リンクを辿った先）と同じフォルダに `initial_install.nar`（大小を問わない）というファイルがあるかを見る（1 回の走りで 1 回だけ・後のゴーストの切り替えや読み直しで見直さない）。
2. When `initial_install.nar` がある, the areka shall それを 1 本の書庫の依頼として、投げ込みと同じ手続きで入れる（既に在るフォルダへは今の手続きのとおり確かめずに上書きする・「入れますか」の画面を新しく足さない）。
3. When `initial_install.nar` の書庫の手続きの終わり方が「入った」だった, the areka shall そのファイルを消し、消したことを記録（info）に 1 件残す。
4. If `initial_install.nar` が入らなかった（失敗・宛先違いで断った・利用条件を拒んだ・途中でやめた）, the areka shall ファイルを残し、今の手続きの記録とゴーストへの知らせのとおりに残す。
5. If 入った `initial_install.nar` を消せない, the areka shall 消せなかったことと理由を記録（warn）に残して起動を続ける（次の起動でまた入る＝上書きになることを説明書に書く・要件 8.3）。
6. When `initial_install.nar` が無い, the areka shall 何もせず、warn 以上の記録を残さない。
7. When `initial_install.nar` と `--install` の両方がある, the areka shall `initial_install.nar` の依頼を先に、`--install` の依頼をその後に、別々の依頼として出す（議題 5）。
8. The areka shall `initial_install.nar` の依頼の記録に、依頼の出どころが `initial_install.nar` であると読める語を、起動の引数・メニュー・台本・投げ込みの語と別に出す。
9. When `--offline-dump` で起動した, the areka shall `initial_install.nar` を見ず、入れず、消さない。
10. The areka shall `initial_install.nar` の手続きでも、利用条件の画面と同梱を飛ばしたときの警告の画面を今の手続きと同じに出す（議題 3）。

### Requirement 5: `--option callghost` で入れたゴーストへ切り替える（議題 1）

**Objective:** As a 開発者・AI のエージェント, I want 起動の行で「入れたら呼べ」と書いたときだけ、入れたゴーストが起きてほしい, so that 「入れる → 起こす → MCP で確かめる」を 1 本の試験で回せる

#### Acceptance Criteria

1. Where `--option callghost` が `--install` と一緒に渡された, when 起動の引数の依頼が全部の書庫を扱い終えて（途中でやめたときは頼まない）、その中でゴーストの書庫が 1 本以上「入った」, the areka shall 並びの中で最後に入ったゴーストへ、利用者が右クリックメニューの「ゴースト」から選んだときと同じ切り替えの道（送り出す側への知らせ・降ろし方・起こし方・記憶）で切り替えを頼む。送り出す側への `OnGhostChanging` の Reference1 は、メニューの選択と同じ `manual` にする。
2. If 最後に入ったゴーストが既に起動中のゴーストである（起動中のゴーストへ上書きした）, the areka shall 切り替えを頼まず、切り替えなかったことと理由を記録（info）に 1 件残す。
3. If 依頼の手続きの間に、ゴースト自身の台本や利用者のメニューの選択で切り替えが始まった・終わった, the areka shall `callghost` の切り替えを頼まず、頼まなかったことと理由を記録（info）に 1 件残す（ゴーストと利用者の選択を上書きしない）。
4. If ゴーストの書庫が 1 本も入らなかった（シェル・バルーン・追加ファイルだけ・全部が入らなかった）, the areka shall 切り替えを頼まず、頼まなかったことを記録（info）に 1 件残す。
5. If 切り替えを頼んだが受け付けられなかった, the areka shall 今の切り替えの道の記録と振る舞いのとおりに扱い、メッセージボックスを出さない。
6. When `callghost` で切り替えを頼んだ, the areka shall 頼んだことと切り替え先のゴーストを、出どころが起動の引数の `callghost` であると読める形で記録（info）に 1 件残す。
7. Where `callghost` が渡されていない, the areka shall 2026-09-28 の裁定のとおり、入れた後に自分から切り替えを始めない（ゴーストが自分で切り替えに行くのは止めない）。
8. The areka shall `callghost` でゴーストを 2 体目として並べて起こさない（今の areka は 1 体だけを動かす）。

### Requirement 6: 読み取りだけの起動では行わない（確かめ）

**Objective:** As a 開発者・AI のエージェント, I want `--option readonly` と組んだとき何も入らず何も消えないと確かめたい, so that 読み取りだけの試しの起動で置き場と NAR を汚さない

#### Acceptance Criteria

1. While 読み取りだけで動いている, the areka shall 起動の引数の依頼と `initial_install.nar` の依頼を、今あるインストールの断り（完了 `readonly-mode`）で断り、何も入れず、ゴーストへインストールのイベントを何も送らない。
2. While 読み取りだけで動いている, the areka shall 断りの記録（`readonly_refused`）に、依頼の出どころ（起動の引数・`initial_install.nar`）が読める語を出す。
3. While 読み取りだけで動いている, the areka shall `deletesource` があっても `--install` の書庫を消さず、`initial_install.nar` も消さない。
4. While 読み取りだけで動いている, the areka shall `callghost` があっても切り替えを頼まない。
5. The areka shall 起動の引数と `initial_install.nar` の依頼を断るときの記録と振る舞いを、メニュー・台本・投げ込みの依頼を読み取りだけで断るときと同じにする（違うのは記録の出どころの語だけ）。

### Requirement 7: ゴーストが居ないとき（議題 2）と失敗の伝え方

**Objective:** As a 開発者・利用者, I want 入れられなかったときに理由が記録に残り、画面が無粋な箱で止まらないでほしい, so that 試験の失敗を記録から追える

#### Acceptance Criteria

1. If 起動の解決がゴーストを見つけられず、ゴーストが 1 体も定常に着かない, the areka shall 今の振る舞い（既定のゴーストの台詞の道で伝えて止まる）を変えず、`--install` と `initial_install.nar` を入れなかったことと並びを記録（warn）に 1 件残す（議題 2）。
2. The areka shall この spec で足す道の失敗（依頼を出せない・書庫を消せない・切り替えを頼めない）を、どれも理由の分かる記録に残す（error か warn・黙って捨てない）。
3. The areka shall この spec で足す道の失敗を、メッセージボックスで告げない（インストールの手続きが今出す利用条件と警告の画面はそのまま）。
4. If インストールの依頼が受け付けられなかった（インストールの受け付けがまだ無い・終了が始まっている・読み取りだけ）, the areka shall 今の受け付けの記録のとおりに残し、`deletesource` の消去と `callghost` の切り替えを行わない。

### Requirement 8: 説明書に書く

**Objective:** As a 利用者・開発者, I want `dist/README.txt` で `--install` と `initial_install.nar` の使い方と限りを知りたい, so that 起動の前に何が入り何が消えるか分かる

#### Acceptance Criteria

1. The areka shall 説明書の「■ 起動オプション」で、`--install`・`callghost`・`deletesource` を「まだ扱わない」の文から外し、扱う物として書く。
2. The areka shall 説明書に、`--install` の書き方（複数・順・相対パスの読み方）・入れ始める時（最初のゴーストが起動した後）・手続きが窓への投げ込みと同じこと・`deletesource` で消える物と残る物・`callghost` で切り替わる先（並びの最後に入ったゴースト）とその限り（2 体目としては呼ばない）・ゴーストが居ないときは入れないこと・起動中の areka や SSP には任せないことを書く。
3. The areka shall 説明書に、`initial_install.nar` の置き場所・確かめずに上書きで入ること・入ったら消えること・消せないときは次の起動でまた入ること・利用条件の画面が今の手続きのとおり出ることを書く。
4. The areka shall 説明書の readonly の「断る物」の文に、`--install` と `initial_install.nar` を足す（入れず、消さない）。
5. The areka shall 説明書に、NAR を入れて呼ぶ 1 行の例を 1 つ載せる（例: `areka.exe --install C:\work\my.nar --option callghost,deletesource`）。

### Requirement 9: テストで固定し、実機で確かめる

**Objective:** As a 開発者, I want 並びの順・消す／消さない・断り・切り替えの判断を決定論のテストで固定したい, so that 後の変更が黙って書庫を消したり切り替えたりしない

#### Acceptance Criteria

1. The areka shall 起動オプションの読み手について、`--install` の値の並びと順・短い形と旧い形・値なし・`callghost` と `deletesource` が扱う語になったこと（大小・カンマの並び）・`--install` なしの語・`--offline-dump` との組みを判定する決定論のテストを持つ。
2. The areka shall 起動の引数の並びが 1 件の依頼として、書いた順のまま、起動の引数の出どころの語で口へ渡ることを判定する決定論のテストを持つ。
3. The areka shall `deletesource` について、書庫の終わり方の 5 つ（入った・断った・利用条件を拒んだ・失敗・途中でやめた）の 1 つずつで消す／残すが決まること・同じパスが 2 回あるとき・消せないときを判定する決定論のテストを持つ。
4. The areka shall `initial_install.nar` について、在る／無い・入った／入らない・大小の違う名前・`--install` との順・`--offline-dump` のときを判定する決定論のテストを持つ。
5. The areka shall `callghost` について、切り替えを頼む／頼まない（ゴーストが入った・入らない・起動中のゴーストだった・途中で切り替えが始まった・`callghost` なし）の判断と切り替え先（最後に入ったゴースト）を判定する決定論のテストを持つ。
6. The areka shall 読み取りだけのとき、起動の引数と `initial_install.nar` の依頼が断られ、記録に出どころが出て、書庫が消えないことを判定する決定論のテストを持つ。
7. The areka shall テストの根と書庫の写しをワークツリーの `target\` の下だけに置き、`deletesource` と `initial_install.nar` の試験で本物の書庫を消さない（写しを使う）。
8. The areka shall 実機で、`target\` の下の根と書庫の写しを使い、起動の 1 行で入れたゴーストが置き場に入ったことと、`callghost` のときにそのゴーストが起動中になったこと（MCP の `get_active_ghost_list`）を、実機の試験の窓口（`real-run-mcp-kit`）の足場で確かめる。

### Requirement 10: 要件討議の決めと完了の段の申し送り

> 2026-10-29 の要件討議で、開発者が 5 つの議題をすべて推奨案で承認した（「承認」）。

**Objective:** As a 開発者, I want 要件討議で決めたことと、この spec の外へ渡すことを残したい, so that 設計が推し量りで進まず、先送りした働きが黙って落ちない

#### Acceptance Criteria

1. The areka shall 議題 1 の決めのとおり、`callghost` を起動の行を書いた利用者の明示の指示（2026-09-28 の裁定の「利用者が明示的に呼び出す」）として扱い、メニューの「ゴースト」と同じ切り替えの道・Reference1 `manual`・切り替え先は並びの最後に入ったゴースト、で切り替える（要件 5）。SSP での測りは行わない（測った答えがどちらでも、今の作業は同じ）。正典の「呼び出される」を 2 体目として呼ぶ形は `ghost-call` が持つ。この spec は完了の段で、`.kiro/specs/ghost-call/brief.md` へ「`callghost` を、2 体目として呼ぶ働きが着地したらその形へ移す」申し送りを書く。
2. The areka shall 議題 2 の決めのとおり、ゴーストが落ち着いた後だけ入れ、ゴーストが 1 体も無いときは入れなかったことを記録に残す（要件 7.1）。SSP のように起動の解決の前に入れてから起動を続ける形は、この spec の完了の段で `/kiro-discovery` で別の spec として起票する。
3. The areka shall 議題 3 の決めのとおり、`initial_install.nar` でも利用条件の画面と警告の画面を今の手続きと同じに出す（利用条件への同意は利用者の判断なので飛ばさない・要件 4.10）。
4. The areka shall 議題 4 の決めのとおり、起動の引数のインストールが終わるまでゴーストのトークを待たせない（要件 2.1・2.6）。
5. The areka shall 議題 5 の決めのとおり、`initial_install.nar` を先に・`--install` を後に別々の依頼として出し（要件 4.7）、`callghost`・`deletesource` は `--install` と組んだときだけ効かせて `initial_install.nar` には効かせない（要件 1.5）。
