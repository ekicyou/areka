# Requirements Document

> 本文の実測は **2026-09-28・本ブランチ**（main `f233f720`＝棚卸⑲のコミット。ソースは `10a8d724`〔`session-mark-residue` の完了〕から不変＝`git diff --stat 10a8d724 HEAD -- crates` は空）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 要件 12 の裁定は、brief の議題 4 件と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。開発者の確定はまだ 1 件も無い。

## Project Description (Input)

**誰の何が困っているか**: 配布サイトから `.nar` を落としてきた第三者（α の利用者）。正典は「install.txtが適切に用意されていれば、D&Dなどの手段でインストーラ機能が働き、自動的にインストールできる」と書くが、今日の areka には `.nar` を渡す入口が 1 つも無い。

**今の状態**: `.nar` を読んで根へ入れる部品（`areka-nar`・完了 `areka-P0-nar-install`／`areka-P0-nar-install-hardening`）は在るが、本体 `areka` からは辿れない（`crates/areka/Cargo.toml` の依存に無い）。窓への投げ込み・ファイル選択・台本 `\![execute,install,path,…]` の受け口は 0、インストール系のイベントを送る所も 0。右クリックメニューの「インストール…」の枠と項目名は在るが、登記が 0 なので項目は出ない。

**何を変えるか**: ゴーストの窓へ `.nar`／`.zip` を落とす・メニュー「インストール…」でファイルを選ぶ・台本 `\![execute,install,path,フルパス]` の 3 つの入口から、同じ 1 本の手続きでインストールできるようにする。手続きは正典のイベント（`OnInstallBegin` → `OnInstallCompleteEx`／`OnInstallComplete`／`OnInstallCompleteAll`、`OnInstallFailure`、`OnInstallRefuse`）を送り、`terms.txt`／`terms.md` があれば展開の前に受諾か拒否かを選ばせる。入れたゴーストは `lastinstalled` で引けるようにし、ゴーストが何も言わなければ areka が入れたゴーストへ切り替える。書庫でないファイルとフォルダの投げ込みは `OnFileDrop2`／`OnDirectoryDrop` としてゴーストへ渡す。失敗はメッセージボックスではなくゴーストの台詞（イベント）と記録で伝える。

> 起票: 2026-09-18 `/kiro-discovery` 再入（棚卸⑭）。2026-09-20・09-24・09-26・09-27・09-28 の棚卸で再測定した。先進坑 `pilot-dropfiles-on-wuc-window` は 2026-09-26 に開発者判定 go（一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md`）。

## Introduction

### 誰が困っているか

- **`.nar` を手に入れた第三者**: areka を起動しても、その `.nar` を渡す方法が無い。フォルダを手で展開して根へ置くしかない。
- **ゴーストの作者**: 台本の `\![execute,install,path,…]` も、辞書に書いた `OnInstallComplete`・`OnFileDrop2` の返事も、areka では 1 度も動かない。

### いま何が起きているか（2026-09-28 実測）

- **入口が 0。** `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`GetOpenFileName`／`IDropTarget` は `crates/`（`crates/pilot/` を除く）の `.rs` に 0 件。wintf の振り分け表（`crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message`）に `WM_DROPFILES` の腕は 0 本。ゴースト窓の様式（`crates/areka/src/placement/spawn.rs` の `window_style`＝キャラクター窓とバルーン窓で共通）は `WS_EX_LAYERED | WS_EX_TOOLWINDOW` で、受け入れの宣言は無い。
- **イベントが 0。** `OnInstall`・`OnFileDrop`・`OnDirectoryDrop`・`OnGhostTerms` の綴りは `crates/` の `.rs` のうち `crates/areka-nar/src/` の 6 ファイル（説明と失敗の語彙・うち 1 つはテスト）にしか無く、送る側は 0 件。kanade の許可表 `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`）は 13 語で、上の 10 語（後述）は 0 語。数は `events_change_tests.rs` が `13` と直書きで判定している。
- **汎用の通知の入口は在るが、送り手が 0。** `KanadeMsg::RaiseEvent { id, references, method }`（`crates/areka-kanade/src/msg.rs`）を送る本番コードは 0 件。受け手 `on_raise_event`（`schedule/change.rs`）は、許可表に無い名前と定常以外での依頼を `warn!` の上で捨てる（積まない）。応答は定常の応答の腕へ流れ、再生中のトークを置き換える。**応答があったか無かったかを送り手へ返す道は無い**（運行の通知 `KanadeNotice` は定常到達・切替の中止・停止の 3 種）。
- **部品は 3 段で揃っている。** `areka_nar::NarArchive::open`（全部を検証し 1 バイトも書かない）→ `manifest()`（`kind`・`name`・`directory`・`accept`・同梱）→ `install(&InstallRequest { root, target_ghost })`。公開の口はこの 3 つだけで、**書庫の中の任意のファイル（`terms.txt` など）を読む口は無い**。失敗は `NarError::Refused { reason: RefuseReason }`（14 種）と `NarError::Io { phase, rolled_back, survivors, … }`。
- **展開は「作業フォルダで組み上げてから入れ替える」。** 宛先が在るとき、下敷きとして**宛先の今の中身を作業フォルダへ写してから**書庫の中身を重ねる（`crates/areka-nar/src/install.rs` の `stage_placement`）。確定は宛先ごとに「宛先 → 作業フォルダの `old-<k>`」「組み上げた木 → 宛先」の 2 手（同 `commit_one`）。2 手の間でプロセスが断たれると宛先のフォルダは無く、元の中身は `<根>/.nar-work/<プロセス識別子>-<連番>/old-<k>/` に残る（7 日の保持）。
- **`supplement` の宛先はゴーストのフォルダそのもの。** `crates/areka-nar/src/plan.rs` は `supplement` を `<根>/ghost/<宛先ゴースト>/` へ、`shell` を `<根>/ghost/<宛先ゴースト>/shell/<directory>/` へ置く。宛先ゴーストは呼び手が `accept` から決めて渡す。
- **切替の入口と受け皿。** `crates/areka/src/emo2_boot/ghost_switch.rs` の `request_ghost_switch`（唯一の入口）・`switch_to`（降ろす → 全窓を閉じる → 起こす）・`record_last_installed`（`#[allow(dead_code)]`・注釈「本番の呼び手は後続 areka-P0-ghost-install」・本番の呼び手 0）。
- **終了の後始末はインストールを知らない。** `crates/areka/src/session_end.rs` の `end_session_within`（OS の終了・SHIORI の待ちの上限 `SESSION_END_SHIORI_LIMIT`＝3 秒）も `fn main` の後始末も、背景で走る手続きを待つ口を持たない。
- **告知の部品は押されたボタンを返さない。** `crates/areka/src/alert.rs` の `raise` は `MB_OK | MB_ICONERROR` の箱を出して戻りを見ない。環境変数 `AREKA_NO_ALERT` で抑えられる。
- **メニュー。** `crates/areka/src/menu/mod.rs` に `Frame::Install`、`captions.rs` に `ghostinstallbutton.caption` が在る。登記の前例は `menu/ghost_frame.rs`。
- **配布物の文書。** `dist/README.txt` の 2 行が「インストール」を「できないこと」に数えている。
- **網羅台帳。** `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` のうち 11 行が `status = "absent"`・`owner = "areka-P0-nar-install"`。`crates/areka-nar/src/` に `// ukadoc:` の行は 0。`shiori.toml` の上記 10 イベント・`sakura-script.toml` の `\![execute,install,path,…]`・`assets.toml` の `install.accept` は `absent`・owner 空。`%lastghostname`／`%lastobjectname` は `vocabulary-only`。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [インストール](https://ssp.shillest.net/ukadoc/manual/manual_install.html) | 「narはzipの拡張子を変えただけのもので、実体はzipなので、サーバによってはzipで配布する場合もある。」「このファイルがなければD&Dなどによる自動インストール機能は動作しない。」 | 受ける拡張子は `.nar` と `.zip`。`install.txt` が無い書庫は入れられない。 |
| [`OnInstallBegin`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallBegin:1) | 「アーカイブのインストール開始の際に発生。」 | Reference は無い。 |
| [`OnInstallCompleteEx`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallCompleteEx:1) | 「インストールが正常終了した際に発生。このイベントが無かった場合OnInstallCompleteが発生。」Reference0「インストールした物の※識別子。複数の場合はバイト値1区切り。」Reference1「インストールした物の名前。複数の場合はバイト値1区切り。」Reference2「インストールした場所。複数の場合はバイト値1区切り。」 | 先に Ex を送り、応えが無ければ旧仕様へ続ける。 |
| [`OnInstallComplete`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallComplete:1) | 「このイベントは旧仕様のため、1回のインストールで同時に入った物のうち先頭の2件しか通知されない。」Reference0「インストールした物の※識別子。」Reference1「インストールした物の名前（install.txtのname指定）。」Reference2「インストールした物の名前2（ゴーストにバルーンを同梱した場合などのballoon側の名前）。」 | Ref2 が「名前 2」なのはこの旧仕様だけ。 |
| [`OnInstallCompleteAll`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallCompleteAll:1) | 「複数のnarをD&Dした時などに、インストールがすべて正常終了した際に、OnInstallComplete/同Exの後に、最後に発生。」 | Reference0〜2 は Ex と同じ形。 |
| [`OnInstallFailure`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallFailure:1) | 「インストールに失敗した際に発生。」Reference0「※失敗理由。」 | Reference は 0 番だけ。 |
| [`OnInstallRefuse`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallRefuse:1) | 「インストールするファイルが他のゴーストを指名していた際に発生。」Reference0「指定されたゴーストの本体側名（install.txtのaccept指定、つまり本来渡すべき相手）。」Reference1「インストールしようとした物の※識別子。」Reference2「インストールしようとした物の名前。」 | 宛先違いは失敗ではなく断り。 |
| [`OnInstallReroute`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnInstallReroute:1) | 「インストールするファイルが他のゴーストを指名していた際、かつ、指名対象ゴーストが一緒に起動していた時に発生。」 | areka は 1 度に 1 体なので起きない＝送らない。 |
| 同ページの注記「インストールの識別子」「インストールの失敗理由」（ukadoc MCP の索引には無く、2026-09-28 にページ本体から 2 度引いて一致を確かめた） | 識別子: 「shell」「ghost」「balloon」「plugin」「headline」「rss」「supplement」／「以下はOnInstallComplete / OnNarCreatedのみ」「SSPでは廃止されており、ゴーストとバルーンがそれぞれ別の識別子として通知される。」「ghost with balloon」「shell with balloon」／失敗理由: 「unlha32」「extraction」＝「解凍失敗（ファイル破損）」・「invalid type」＝「install.txtに不備がある」・「artificial」＝「ユーザがインストールを中断した（※SSPのみ）」・「unsupported」＝「その他サポートしていないアーカイブ」・「password」 | 識別子と失敗理由は正典の語で送る。 |
| [`accept,本体側名`](https://ssp.shillest.net/ukadoc/manual/descript_install.html#accept_2c_672c_4f53_5074_540d:1) | 「アーカイブを受け取ることができるゴーストの本体側の名前（＝\0名,sakura名）。もしくはゴースト側descript.txtのinstall.acceptに設定してある名前でも可。追加シェルや追加ファイル（supplement）など、特定ゴーストに渡す内容では必ず用いる。」 | 照合の相手は 2 つ。 |
| [`install.accept`](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#install.accept_2c_540d_524d1_2c_540d_524d2_2c_540d_524d3...:1) | 「カンマ区切りで列挙した名前のゴースト用narファイルをインストール可能にする。」 | 受け手のゴーストの側の名乗り。 |
| [全体の構成](https://ssp.shillest.net/ukadoc/manual/manual_directory.html)（`terms.txt`） | 「利用条件表示。NAR/ZIPファイルのインストール前にこの内容を表示するダイアログが出る。」「readmeと同じく、1行目にcharset,UTF-8と書いておくか、BOMを含んでおけばShift JIS以外も使える。」「readme.mdと同じ条件でterms.mdも利用可能。」バルーンの構成にも「terms.txt または terms.md (必要な時のみ)」 | ゴーストの書庫にもバルーンの書庫にも置ける。 |
| [`OnGhostTermsAccept`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnGhostTermsAccept:1)／[`OnGhostTermsDecline`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnGhostTermsDecline:1) | 「利用条件ダイアログで受諾が押された際に発生。」「利用条件ダイアログで拒否が押された際に発生。」「右上の閉じるボタンでは何もイベントは発生しない。」 | 閉じるボタンの読みは要件 12 裁定 3。 |
| [`\![execute,install,path,ファイル名]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1) | 「指定したnarファイルをインストールする。指定はフルパスで行うこと(相対パス指定不可)。」 | 相対パスは受けない。 |
| [`OnFileDrop2`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFileDrop2:1) | 「ファイルがDnDされた際に発生。このイベントが現時点での最新仕様となる。」Reference0「ドロップされたファイルパス。複数ファイル/ディレクトリがあればbyte値1で区切る。」Reference1「ドロップされたキャラクターのスコープ番号。本体側0、相方1、3人目以降は2以降。」Reference2「ドロップされたファイルのMIMEタイプ(SSP 2.7.98以降)。複数ファイル/ディレクトリがあればbyte値1で区切る。」 | 書庫でないファイルの投げ込み。 |
| [`OnDirectoryDrop`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnDirectoryDrop:1) | 「ディレクトリがDnD された際に発生。」Reference0「ドロップされたディレクトリのパス。」 | フォルダの投げ込み。 |
| [`%lastghostname`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_25lastghostname:1)／[`%lastobjectname`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_25lastobjectname:1) | 「インストール時用。最後にイベントを行ったゴースト名。」「インストール時用。最後にイベントを行ったオブジェクト名。」 | 何の名前かの読みは要件 12 裁定 11。 |
| [`\![change,ghost,…]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1) | 「ゴースト名をlastinstalledにすると最後にインストールしたゴーストに切り替え。ただしSSPを一度終了した場合は無効。」 | 記録はプロセスの中だけ（完了 `ghost-change-name-resolution` のまま）。 |

正典が沈黙している点（イベントと利用条件の画面の前後・利用条件を拒否した後に失敗の知らせを送るか・`accept` の無い `shell`／`supplement`・入れた後に誰が切り替えるか・起動中のゴーストを上書きするとき・途中でアプリが終わるとき・複数を一度に落としたときの順・`terms.txt` と `terms.md` の両方があるとき）は areka の裁量として要件 12 で決め、`doc/COMPAT_ARCHITECTURE.md` §8 に記す。SSP の挙動を測って合わせることはしない。

### brief の記述を正典と実物で引き直して改めた点

1. brief の Out の「理由 `unsupported type`」は正典の語に無い。正典の語は `unsupported` と `invalid type`（別の語）。
2. brief と `areka-nar` の説明は「`RefuseReason::kind()` の語をそのまま `OnInstallFailure` の理由に写す」と書くが、既存のゴーストの辞書が比べる相手は正典の語である。Reference0 は正典の語で送り、`kind()` の語は記録に残す（要件 5・裁定 10）。
3. brief の Desired Outcome 1 は `OnInstallCompleteEx` の Ref2 を「同梱バルーンの名前」と書くが、正典は「インストールした場所」。「名前 2」は旧仕様 `OnInstallComplete` だけ（brief の 09-26 節で訂正済みを確認）。
4. brief の Scope の「イベント 7 種」は、送るのが 6 種（`OnInstallBegin`・`OnInstallComplete`・`OnInstallCompleteEx`・`OnInstallCompleteAll`・`OnInstallFailure`・`OnInstallRefuse`）、送らないのが 1 種（`OnInstallReroute`）。
5. フォルダの投げ込みは `OnFileDrop2` ではなく `OnDirectoryDrop`（brief の 09-24 節で訂正済みを確認）。
6. 識別子 `ghost with balloon`／`shell with balloon` は正典自身が「廃止」と書く。送らない。
7. brief の Constraints の「固定 `.nar` 4 種は `nar-install` の fixture を再利用」は、ファイルとしての fixture が無いので成り立たない。テストの中で `sample_ghost_kit::nar_writer` が組む（brief の 09-24 節 11 のとおり）。

### 要件を書く途中で見つかった、brief に無い事実

1. **`supplement` は必ず起動中のゴーストのフォルダを入れ替える。** areka は 1 度に 1 体なので、`accept` が一致して受け取れる相手は起動中のゴーストだけ。その `supplement` の宛先はゴーストのフォルダそのものなので、議題 ⑴ を「断る」で答えると `supplement` は 1 本も入らなくなる（裁定 1）。
2. **降ろす前に組み上げると、ゴーストが降りるときに保存した内容を失う。** 展開は宛先の今の中身を下敷きに写すので、写した後で SHIORI が保存を書くと、その保存は入れ替えで古い写しに置き換わる。起動中のゴーストへ入れるときは、降ろし終えてから展開を始める必要がある（要件 7.3）。
3. **応えが無かったことを知る道が無い。** 「`OnInstallCompleteEx` に応えなければ `OnInstallComplete`」（正典）と「ゴーストが何も言わなければ areka が切り替える」（裁定 5）は、どちらもゴーストが台本を返したかどうかを知る必要がある。汎用の通知の入口は応答を送り手へ返さないので、brief が「触らない」に挙げた kanade のファイル（`msg.rs`・`schedule/change.rs` など）に手が入りうる。形は設計で決める。
4. **書庫の中の `terms.txt` を読む口が無い。** `areka-nar` に読む口を 1 つ足す必要がある（brief は `areka-nar` を `// ukadoc:` の行だけで触る見立て）。形は設計で決める。

### 何を変えるか

3 つの入口（窓への投げ込み・メニュー・台本）が同じ 1 本の手続きへ流れ、手続きは背景で走って、正典のイベントを起動中のゴーストへ送る。宛先違いは断り、利用条件は受諾か拒否かを選ばせ、失敗は正典の語の理由で知らせる。起動中のゴーストのフォルダへ入れるときは、そのゴーストを降ろしてから入れて起こし直す。入れたゴーストは `lastinstalled` で引け、ゴーストが何も言わなければ areka が切り替える。途中でアプリや Windows が終わるときは、展開の終わりを上限つきで待つ。書庫でないファイルとフォルダは `OnFileDrop2`／`OnDirectoryDrop` で渡す。

## Boundary Context

- **In scope**:
  - インストールの手続き 1 本（`areka-nar` の 3 段を呼ぶ・`accept` の照合・結果を Reference へ写す・背景で走る）と、依頼を順に扱う待ち行列。
  - 入口 3 つ: ゴーストの窓への投げ込み（`WM_DROPFILES`・wintf の受け口を含む）／メニュー「インストール…」とファイルを選ぶ画面／台本 `\![execute,install,path,フルパス]`。
  - 送るイベント 10 語: `OnInstallBegin`・`OnInstallComplete`・`OnInstallCompleteEx`・`OnInstallCompleteAll`・`OnInstallFailure`・`OnInstallRefuse`・`OnGhostTermsAccept`・`OnGhostTermsDecline`・`OnFileDrop2`・`OnDirectoryDrop`。
  - `terms.txt`／`terms.md` の表示（ゴーストの書庫・バルーンの書庫）と、告知の部品に足す「はい／いいえを返す」口 1 つ。
  - 起動中のゴーストのフォルダへ入れるときの「降ろす → 入れる → 起こし直す」。
  - 入れたゴーストの記録（`lastinstalled` の受け皿へ書く）と、入れた後の切替。置換語 `%lastghostname`／`%lastobjectname`。
  - インストールの途中でアプリや Windows が終わるときの、上限つきの待ち。
  - 網羅台帳・生成物・`dist/README.txt` の 2 行・`doc/COMPAT_ARCHITECTURE.md` §8・`crates/areka-nar/src/` の正典 URL の行。
  - 決定論テストと実機確認。
- **Out of scope**:
  - `.nar` の読み取り・検査・展開・パスの安全性・名前の長さの上限（完了 `nar-install`／`nar-install-hardening`）。
  - `\![execute,install,url,…]`・URL の投げ込み（`OnURLDropping`／`OnURLDropped`／`OnURLDropFailure`／`OnURLQuery`）＝`network-update`。
  - `\![open,terms]`（起動中のゴーストの利用条件を開き直す）・メニューの「利用条件」の項目（`termsbutton.caption`）。
  - インストール時に `readme.txt` を開くこと（メニュー「説明書」で足りる）。
  - `type` が `plugin`／`headline`／`language`／`calendar skin`／`calendar plugin`／`package` の書庫と `bootghost`（`areka-nar` が受けるのは `ghost`・`shell`・`supplement`・`balloon` の 4 つ。他は失敗の知らせで理由 `unsupported`）。
  - `OnInstallReroute`（多重ゴースト）・`OnFileDropping`（ドラッグ中・`WM_DROPFILES` では取れない）・旧仕様の `OnFileDrop`／`OnFileDropEx`／`OnFileDropped`・`OnTextDrop`／`OnOtherObjectDropping`／`OnOtherObjectDropped`・`OnNarCreating`／`OnNarCreated`（フォルダから `.nar` を作る）・`OnArchiveViewerOpen` などゴーストが応えなかったときのビューア。
  - シェルとバルーンを実行中に切り替えること・シェルを選ぶメニュー（`shell-balloon-switch`）。
  - `installedghostname` などの一覧の通知・PLUGIN 側の `OnInstallComplete`。
  - 巻き戻せなかった元の中身を別のゴーストとして救い出すこと（裁定 2）。
  - 管理者として起動した areka への、ふつうの権限のエクスプローラからの投げ込みの手当て。
  - emo2 の辞書にインストール系の台詞を足すこと（辞書はリポジトリの外・開発者の手）。
- **Adjacent expectations**:
  - 完了 `nar-install`／`nar-install-hardening` の保証（失敗したら宛先は呼ぶ前のまま・巻き戻せなかった元の中身の在りかを失敗の値が持つ・7 日の保持）に依る。areka 側の依頼と結果の型は、`areka_nar::InstallRequest`／`InstallOutcome` と別の名前にする（名前は設計で決める）。
  - 完了 `ghost-shell-balloon-switch` の切替の入口・汎用の通知の入口・起動中の印、完了 `ghost-change-name-resolution` の `lastinstalled` の受け皿、完了 `session-mark-residue` の上限 3 秒と印の判定、完了 `popup-menu-minimal` の「インストール」枠、完了 `shiori-fault-notice` の告知の部品、完了 `baseware-root-layout` の根と目録（目録の素性 `Identity` に項目を足さない）、完了 `charset-canon` の文字コードの規則に依る。完了 spec の文書は書き換えない。
  - 先進坑 `pilot-dropfiles-on-wuc-window`（go）の結果に依る: 受け入れの宣言は拡張スタイル `WS_EX_ACCEPTFILES` だけで足り、絵の外へ落とした物は OS が背後の窓へ渡す。
  - 後続 `shell-balloon-switch`（許可表の数を本仕様の 23 から動かす）・`network-update`（同じ台本の受け口に `url` を足す・終了で待つ口に背景の更新を乗せられる）は本仕様の後に建つ。
  - `alpha-release-signoff` へ申し送る: `dist/README.txt` の「■ .nar の入れ方」の本文、既知の制限の候補（絵の縁ぎりぎりに素早く落としたとき／巻き戻せなかった元の中身は `.nar-work` の下に 7 日残る／入れたシェルを選ぶ入口は `shell-balloon-switch` から）。
  - 並走する `frame-phases-after-exit` とは共有するソースが 0（同 spec が触るのは `crates/areka/src/emo2_boot/frame.rs` とその兄弟テストだけ）。

## Requirements

### Requirement 1: 3 つの入口は同じ 1 本の手続きへ流れる

**Objective:** As a `.nar` を手に入れた第三者, I want 窓へ落としてもメニューから選んでも同じようにインストールされること, so that 入れ方を覚え直さずに済む

#### Acceptance Criteria

1. The areka shall 窓への投げ込み・メニュー「インストール…」・台本 `\![execute,install,path,…]` から届いた書庫を、同じ 1 本のインストールの手続きで扱う（入口ごとに別の手順を持たない）。
2. When 利用者がゴーストの窓（キャラクター窓・バルーン窓）の絵の上へ、拡張子が `.nar` または `.zip`（大文字小文字を区別しない）のファイルを落とす, the areka shall そのファイルを手続きへ渡す。
3. When 利用者が右クリックメニューの「インストール…」を選ぶ, the areka shall ファイルを選ぶ画面を出し、選ばれたファイル 1 つを（拡張子を問わず）手続きへ渡す。
4. When 利用者がファイルを選ぶ画面を取り消す, the areka shall 何も入れず、イベントを 1 件も送らず、取り消されたことを記録に 1 件残す。
5. When 再生中の台本が `\![execute,install,path,パス]` の位置に達し、パスが絶対パスである, the areka shall そのファイルを（拡張子を問わず）手続きへ渡す。
6. If 台本のパスが相対パス・空である, then the areka shall 手続きを始めず、イベントを 1 件も送らず、`warn!` を 1 件残す。
7. If 台本の `\![execute,install,…]` の 2 番目の引数が `path` 以外（`url` を含む）である, then the areka shall 何もせず `warn!` を 1 件残す（`url` は `network-update` が引き受ける）。
8. When ゴーストを起こす（最初の起動でも切替の後でも）, the areka shall メニューの「インストール」枠へ項目を登記し直す（項目名は今日どおり `ghostinstallbutton.caption`、答えが無ければ既定名「インストール…」）。
9. The areka shall 手続きを 1 度に 1 本だけ走らせ、走っている間に届いた依頼は届いた順に待たせて続けて扱う（捨てない）。
10. While ゴーストが定常でない（起動の途中・切替の途中）, when 書庫の依頼が届く, the areka shall 依頼を待たせ、ゴーストが定常に入ってから手続きを始める。
11. While ゴーストが表示されている, the areka shall 書庫の読み取り・展開・ファイルを選ぶ画面・利用条件の画面のどれの間も、ゴーストの描画・台詞の再生・メニュー・Windows の終了の後始末を止めない（要件 7 でゴーストを降ろしている間は窓が 0 枚で、止める相手の描画・台詞・メニューが無い。その間の展開をどのスレッドで行うかは設計で決める）。
12. The areka shall 最初の起動が窓の無い形へ倒れた回（窓も台本の受け口も無い）では、3 つの入口のどれも受けない（今日どおり・本仕様で足す経路 0）。

### Requirement 2: 手続きは正典の順序でイベントを送る

**Objective:** As a ゴーストの作者, I want インストールの始まりと終わりが正典のイベントと Reference で届くこと, so that 辞書に書いた返事がそのまま動く

#### Acceptance Criteria

1. When 手続きが書庫 1 本を扱い始める, the areka shall 起動中のゴーストへ `OnInstallBegin`（Reference 0 個）を GET で送る。
2. The areka shall 書庫 1 本の手続きを「`OnInstallBegin` → 書庫の読み取りと検査（1 バイトも書かない）→ `accept` の照合 → 利用条件 → 展開 → 締めの知らせ」の順に進める。
3. The areka shall 書庫 1 本につき、締めの知らせを次の 4 つのうち 1 つだけ送る: 成功（`OnInstallCompleteEx`、応えが無ければ `OnInstallComplete`）／宛先違い（`OnInstallRefuse`）／利用条件の拒否（`OnGhostTermsDecline`）／失敗（`OnInstallFailure`）。途中でアプリが終わった場合だけは 0 件（要件 8.6）。
4. When 展開が成功する, the areka shall `OnInstallCompleteEx` を GET で送り、Reference0＝入れた物の識別子・Reference1＝入れた物の名前・Reference2＝入れた場所（フォルダの絶対パス）を、入れた物の数だけ byte 値 1 区切りで並べる。並びは 3 つの Reference で同じで、書庫の本体が先、同梱バルーンが後。
5. The areka shall 識別子を正典の語 `ghost`・`shell`・`balloon`・`supplement` の 4 つだけで送る（`ghost with balloon`・`shell with balloon` は 0 件）。
6. When `OnInstallCompleteEx` にゴーストが台本を返さない（返事なし・空の台本）, the areka shall 続けて `OnInstallComplete` を GET で送り、Reference0＝先頭の物の識別子・Reference1＝`install.txt` の `name`・Reference2＝同梱バルーンの名前（無ければ空）とする。
7. When `OnInstallCompleteEx` にゴーストが台本を返す, the areka shall `OnInstallComplete` を送らない。
8. When 1 回の依頼（1 回の投げ込み）が書庫を 2 本以上含み、その全部が成功で終わる, the areka shall 最後の書庫の完了の知らせの後に `OnInstallCompleteAll` を GET で 1 回送り、Reference0〜2 に全部の書庫ぶんを byte 値 1 区切りで並べる。
9. If 1 回の依頼の書庫が 1 本だけである、または 1 本でも成功しなかった, then the areka shall `OnInstallCompleteAll` を送らない。
10. While 締めの知らせを送る時点でゴーストが定常でない, the areka shall 定常に入るまで待ってから送る（捨てない）。
11. The areka shall `OnInstallReroute` を送らない（0 件）。
12. The areka shall 本仕様で新しいイベント名を作らない（0 個）。送るのは正典に在る 10 語だけで、kanade の許可表は 13 語から 23 語になる。
13. The areka shall イベントの送り先を、送る時点で起動中のゴーストとする（手続きの途中でゴーストが替わっていれば、替わった後のゴーストへ送る）。

### Requirement 3: `accept` を照合し、宛先違いは断る

**Objective:** As a ゴーストの作者, I want 自分のゴースト宛てのシェルや追加ファイルが、別のゴーストに入ってしまわないこと, so that 配布物が意図した相手にだけ届く

#### Acceptance Criteria

1. When 書庫の `install.txt` に `accept` が在る, the areka shall その値を、起動中のゴーストの `descript.txt` の `sakura.name` と、`install.accept` にカンマ区切りで並んだ各名前と突き合わせ、どれか 1 つに完全一致（大文字小文字を区別）すれば受け取る。
2. If `accept` がどれにも一致しない, then the areka shall 1 バイトも書かずに手続きを止め、`OnInstallRefuse` を GET で送る（Reference0＝`accept` の値・Reference1＝識別子・Reference2＝`install.txt` の `name`）。
3. The areka shall `accept` の照合を、展開の前に、`install.txt` の内容だけを材料にして行う。
4. When `type` が `ghost` または `balloon` で `accept` が無い, the areka shall 照合なしで受け取る。
5. When `type` が `shell` または `supplement` で `accept` が一致する, the areka shall 起動中のゴーストのフォルダを宛先にする。
6. If `type` が `shell` または `supplement` で `accept` が無い, then the areka shall 1 バイトも書かずに手続きを止め、`OnInstallFailure`（理由 `invalid type`）を送る。
7. The areka shall 宛先違いを失敗として数えない（`OnInstallFailure` 0 件・記録は `error!` ではなく `warn!` 以下）。

### Requirement 4: 利用条件があれば、展開の前に受諾か拒否かを選ばせる

**Objective:** As a 配布物の作者と利用者, I want 利用条件が入れる前に示され、拒否すれば何も入らないこと, so that 条件を読まずに入ってしまうことが無い

#### Acceptance Criteria

1. When 書庫の最上位（`install.txt` と同じ階層）に `terms.txt` または `terms.md` が在る, the areka shall 展開の前にその本文を画面に出し、利用者に受諾か拒否かを選ばせる。ゴーストの書庫でもバルーンの書庫でも同じ。
2. Where `terms.txt` と `terms.md` の両方が在る, the areka shall `terms.txt` を出す。
3. The areka shall 本文を、1 行目の `charset,` の指定か BOM があればそれに従い、無ければ Shift_JIS として読む。`terms.md` の Markdown の装飾は解釈せず、本文のまま出す。
4. The areka shall 利用条件の画面を「はい」（受諾）と「いいえ」（拒否）の 2 択にし、閉じるボタンで閉じる道を持たない。
5. When 利用者が受諾を選ぶ, the areka shall `OnGhostTermsAccept`（Reference 0 個）を GET で送り、展開へ進む。
6. When 利用者が拒否を選ぶ, the areka shall 1 バイトも書かずに手続きを止め、`OnGhostTermsDecline`（Reference 0 個）を GET で送る。`OnInstallFailure` は送らない（0 件）。
7. Where 環境変数 `AREKA_NO_ALERT` が設定されている, the areka shall 画面を出さずに拒否として扱い（要件 4.6 と同じ）、抑止で拒否に倒したことを `warn!` に 1 件残す。
8. If 利用条件の画面を出せなかった, then the areka shall 拒否として扱い、`error!` を 1 件残す。
9. If 本文が画面に収まらない長さである, then the areka shall 先頭から上限（設計で決める）までを出し、末尾に「続きは書庫の中の利用条件のファイルにある」ことを足す。
10. The areka shall 同梱バルーンのフォルダの中に置かれた利用条件のファイルを出さない（出すのは書庫の最上位の 1 つだけ・`info!` に読み飛ばしたことを残す）。
11. When 書庫に利用条件のファイルが無い, the areka shall 画面を出さず、`OnGhostTermsAccept`／`OnGhostTermsDecline` のどちらも送らずに展開へ進む。

### Requirement 5: 失敗はメッセージボックスではなく、ゴーストへの知らせと記録で伝える

**Objective:** As a 利用者, I want 入らなかったときに、入らなかったことと理由が残ること, so that 何も黙って消えない

#### Acceptance Criteria

1. If 書庫の読み取り・検査・展開のどれかが失敗する, then the areka shall `OnInstallFailure` を GET で送り、Reference0 に正典の失敗理由の語を 1 つ載せる。Reference1 以降は足さない（0 個）。
2. The areka shall 失敗理由の語を次の表で決め、`areka-nar` の拒否の 14 種と I/O の失敗のすべてが、3 語のどれか 1 つに必ず写るようにする（写らない種類 0・種類が増えたらビルドが止まる形）。

   | 正典の語 | 写す失敗 |
   |---|---|
   | `extraction` | 書庫が壊れている・中身の整合が合わない・名前を読めない（`CorruptArchive`・`IntegrityMismatch`・`NameUndecodable`）と、読み取り・組み上げ・確定・巻き戻しの I/O の失敗 |
   | `invalid type` | `install.txt` の不備（`MissingInstallTxt`・`type` の指定なし・`MissingRequiredKey`・`InvalidDirectoryName`・`CompanionSourceMissing`・`TargetGhostMissing`）と、`accept` の無い `shell`／`supplement` |
   | `unsupported` | areka が受けない書庫（`UnsupportedEntry`〔暗号化を含む〕・受けない `type` の指定・`SymlinkEntry`・`UnsafePath`・`PathTooLong`・`CaseCollision`） |

3. The areka shall 正典の語 `unlha32`・`artificial`・`password` を送らない（0 件）。
4. The areka shall 失敗・宛先違い・利用条件の拒否のどれでも、メッセージボックスを出さない（0 個）。本仕様が出す画面は、ファイルを選ぶ画面と利用条件の画面の 2 つだけ。
5. If 手続きが失敗する, then the areka shall `error!` を 1 件残し、書庫のパス・`areka-nar` の失敗の種類の語・どの段で失敗したか・宛先が元へ戻ったかを載せる。
6. If 確定に失敗して元へ戻せなかった, then the areka shall `error!` に、戻せなかった宛先・元の中身が残っているフォルダ・そのフォルダが 7 日で消えることを、宛先ごとに載せる。
7. The areka shall ゴーストが `OnInstallFailure`・`OnInstallRefuse` に応えなかったとき、代わりの画面や台詞を出さない（利用者から見える変化は 0・記録は残る）。
8. When 展開は成功したが片付けられずに残った作業フォルダが在る, the areka shall 成功として扱い、残った場所を `warn!` に残す。

### Requirement 6: 入れた物は覚えられ、ゴーストが何も言わなければ areka が切り替える

**Objective:** As a `.nar` を落とした第三者, I want 入れたゴーストがそのまま出てくること, so that 入れた後にメニューから探さずに済む

#### Acceptance Criteria

1. When ゴーストを入れ終える（`type` が `ghost`）, the areka shall そのゴーストのフォルダ名を `lastinstalled` の受け皿へ書く（プロセスの中だけ・ファイルへは書かない）。以後 `\![change,ghost,lastinstalled]` はそのゴーストへ切り替わる。
2. When ゴーストを入れ終え、完了の知らせ（`OnInstallCompleteEx`・`OnInstallComplete`・送った場合は `OnInstallCompleteAll`）のどれにも起動中のゴーストが台本を返さなかった, the areka shall 入れたゴーストへ、既存の切替の入口を通して切り替える（`OnGhostChanging` を送る切替・切替の理由は `automatic`）。
3. When 完了の知らせのどれかにゴーストが台本を返した, the areka shall 自分からは切り替えない（切り替えるかどうかはその台本に任せる）。
4. When 1 回の依頼で複数のゴーストを入れた, the areka shall 要件 6.1・6.2 の相手を最後に入れたゴーストにする。
5. When 入れたゴーストが起動中のゴースト自身である, the areka shall 要件 7 の起こし直しだけを行い、追加の切替をしない。
6. When バルーンだけを入れ終える, the areka shall 切り替えず、起動中のゴーストの「最後に使ったバルーン」の記憶を入れたバルーンへ書き換える（次にそのゴーストを起こしたときから使われる）。
7. When シェルだけを入れ終える, the areka shall 切り替えず、表示中のシェルも替えない。入れたシェルを選ぶ入口は本仕様では足さない（0 個・`shell-balloon-switch` が足す）。
8. When 書庫を 1 本入れ終える, the areka shall 置換語 `%lastobjectname` をその書庫の `install.txt` の `name` に、`%lastghostname` を関わったゴーストの名前（`ghost` なら入れたゴースト、`shell`／`supplement` なら宛先のゴースト、`balloon` なら前の値のまま）にする。
9. While このプロセスでまだ 1 本も入れていない, the areka shall `%lastghostname`／`%lastobjectname` を今日どおり置き換えない。

### Requirement 7: 起動中のゴーストのフォルダへ入れるときは、降ろしてから入れて起こし直す

**Objective:** As a 起動中のゴーストの新しい版や追加ファイルを落とした利用者, I want そのゴーストが一度引っ込んで、新しい中身で戻ってくること, so that 使っている最中のゴーストも、保存した内容を失わずに入れ替えられる

#### Acceptance Criteria

1. When 宛先が起動中のゴーストのフォルダそのものである（`ghost` の上書き・`supplement`）, the areka shall 展開へ進む時点で再生中の台詞（`OnInstallBegin`・`OnGhostTermsAccept` への返事）が終わってからそのゴーストを降ろし、展開し、同じゴーストを起こし直し、定常に入ってから締めの知らせを送る。
2. The areka shall 降ろし方と起こし方を、完了 `ghost-shell-balloon-switch` の「同じゴーストへの、知らせを送らない切替」と同じにする（`OnGhostChanging` も `OnClose` も送らない・SHIORI の解放を待つ・全窓が 0 になっても終了しない・起動中の印の扱いは切替の規則のまま）。
3. The areka shall 展開を、ゴーストを降ろし終えた後に始める（降ろす前の中身を下敷きにしない）。ゴーストが降りるときに保存した内容は、入れ替えの後も残る（書庫が同じ名前のファイルを持つ場合と、`refresh` で消すと書かれた場合を除く）。
4. If 展開に失敗し、宛先が元のまま残っている, then the areka shall 同じゴーストを元の中身で起こし直し、定常に入ってから `OnInstallFailure` を送る。
5. If 確定に失敗して元へ戻せず、ゴーストを起こせない, then the areka shall 既存の切替の失敗と同じく既定ゴーストへ戻す（`OnBoot` の Reference6＝`halt`・Reference7＝そのゴーストの名前）。戻した既定ゴーストが定常に入ってから `OnInstallFailure` を送る。
6. If 要件 7.5 で起こせないゴーストが既定ゴースト自身である, then the areka shall 今日の致命の経路（告知・終了コード 1）で終わる。
7. While 降ろして入れて起こし直している間, when 切替の要求が届く, the areka shall 完了 `ghost-shell-balloon-switch` 要件 1.9 と同じく無視して `warn!` を 1 件残す。
8. When 宛先が起動中のゴーストのフォルダの中のシェルのフォルダ、またはバルーンのフォルダである, the areka shall ゴーストを降ろさずに入れる。使用中で入れ替えられなければ要件 5 の失敗として扱う（宛先は元のまま）。
9. The areka shall きれいな終わりの判定（`session_mark_verdict`）の引数と、印を残す理由の語を変えない（足す理由 0 個）。

### Requirement 8: インストールの途中でアプリや Windows が終わるとき、展開の終わりを上限つきで待つ

**Objective:** As a 入れている最中にアプリを終えた利用者, I want 元のゴーストのフォルダが消えたまま終わらないこと, so that 終了の操作でゴーストを失わない

#### Acceptance Criteria

1. When 終了の後始末に入った時点で手続きが展開の最中である, the areka shall 展開の終わりを上限 T まで待ってからプロセスを終える。**T＝3 秒**（裁定 4）。
2. When OS のセッションの終了の後始末に入った時点で手続きが展開の最中である, the areka shall 展開の待ちと SHIORI の待ちを、後始末に入った同じ時点から数え、合わせて 3 秒を超えない（足し算にしない）。
3. If 上限 T に達した, then the areka shall 待つのをやめて後始末を続け、`warn!` に書庫のパス・宛先・元の中身が `<根>/.nar-work/` の下に残っているかもしれないことを 1 件残す。
4. While 手続きがまだ 1 バイトも書いていない段（ファイルを選ぶ画面・書庫の読み取りと検査・利用条件の画面）に居る, when 終了の後始末に入る, the areka shall 待たずに終え、途中でやめた書庫を記録に 1 件残す。
5. When 終了の後始末に入る, the areka shall 待っている依頼（まだ始めていない書庫）を始めずに捨て、捨てた件数とパスを `warn!` に残す。
6. The areka shall 終了の後始末に入った後は、インストール系のイベントを送らない（0 件）。
7. The areka shall インストールの途中で終えたこと・上限 T に達したことを、起動中の印を残す理由にしない（印の判定は今日どおり）。
8. When 途中で断たれて起動していたゴーストのフォルダが無くなった次の起動, the areka shall 今日どおり（前回のゴーストが見つからない `warn!` を残して次の候補へ進む）起き、本仕様の知らせや画面を足さない（0 個）。
9. The areka shall 切替・メニューの終了・OS の終了で SHIORI を待つ期限を、今日の値のまま変えない。
10. The areka shall 終了で背景の仕事を待つ口を、後続 `network-update` の背景の更新が同じ口で待てる形にする。

### Requirement 9: 書庫でないファイルとフォルダの投げ込みは、ゴーストへ渡す

**Objective:** As a ゴーストの作者, I want 落とされたファイルやフォルダのパスがイベントで届くこと, so that 渡された物に応える台詞や遊びが動く

#### Acceptance Criteria

1. When 利用者がゴーストの窓の絵の上へ、拡張子が `.nar`・`.zip` でないファイルを落とす, the areka shall `OnFileDrop2` を GET で 1 回送り、Reference0＝落とされたファイルの絶対パス（複数は byte 値 1 区切り）・Reference1＝落とされた窓のスコープ番号・Reference2＝各ファイルの MIME タイプ（Reference0 と同じ並び・byte 値 1 区切り）とする。
2. When 利用者がフォルダを落とす, the areka shall フォルダ 1 つにつき `OnDirectoryDrop` を GET で 1 回送り、Reference0＝フォルダの絶対パス・Reference1＝落とされた窓のスコープ番号とする。
3. When 1 回の投げ込みに書庫・書庫でないファイル・フォルダが混ざっている, the areka shall 書庫でないファイル（`OnFileDrop2` 1 回）→ フォルダ（`OnDirectoryDrop`）→ 書庫（手続き）の順に扱う。
4. The areka shall MIME タイプを拡張子から決め、決められない拡張子は空で送る。
5. If 書庫でないファイル・フォルダが落とされた時点でゴーストが定常でない, then the areka shall そのイベントを送らずに `warn!` を 1 件残す（書庫の依頼だけは要件 1.10 で待たせる）。
6. When 利用者が絵の外（透けている所）へ落とす, the areka shall 何もしない（落とし物は OS が背後の窓へ渡す）。
7. The areka shall `install.txt` を持つフォルダが落とされても、フォルダとして扱う（`.nar` を作らない）。
8. The areka shall `OnFileDropping`・`OnFileDrop`・`OnFileDropEx`・`OnFileDropped` を送らない（0 件）。
9. The areka shall 落とされた物の受け取りを、受け取った 1 件ごとに記録に残す（件数・窓のスコープ番号・種類の内訳）。

### Requirement 10: 記録・台帳・文書が実物と揃う

**Objective:** As a 後続の spec の開発者と利用者, I want 何が入り何が入らなかったかが記録から追え、台帳と説明書が実物と食い違わないこと, so that 次の作業と利用者の手順が誤った前提に立たない

#### Acceptance Criteria

1. The areka shall 手続きの各段（依頼を受けた・始めた・`accept` の照合の結果・利用条件の結果・展開の結果・締めの知らせ・受け皿への記録・切替の依頼）を記録に残し、記録の無い失敗の経路を作らない（0 本）。
2. The 本仕様 shall 本体から `areka-nar` を辿れるようにしたその変更と同じコミットで、`crates/areka-nar/src/` の定義の場所に正典の URL の行（`// ukadoc:`）を置き、網羅台帳 `assets.toml` の `descript_install` の 11 行（`type`・`name`・`directory`・`accept`・`charset`・`refresh`・`refreshundeletemask`・`*.directory`・`*.refresh`・`*.refreshundeletemask`・`*.source.directory`）を実装済みへ動かす。
3. The 本仕様 shall 網羅台帳の、本仕様が送る 10 イベント・`\![execute,install,path,…]`・`install.accept`・`%lastghostname`・`%lastobjectname`・`ghostinstallbutton.caption` の行を実物に合わせて更新し、生成物と `roadmap-draft.md` の数は生成器で作り直す（手で数を直さない）。
4. The 本仕様 shall `dist/README.txt` の「できないこと」を挙げる 2 行から「インストール」を消す。「■ .nar の入れ方」の本文は `alpha-release-signoff` が書く。
5. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 に、要件 12 の裁定のうち正典が沈黙している点を記す。
6. The 本仕様 shall 本番コードが読む環境変数を足さず（0 個）、外部クレートを足さない（0 個。ワークスペースの中の `areka-nar` を依存に足すことと、既に依存している `windows` クレートの機能を足すことは数えない）。
7. The 本仕様 shall 本番ファイルとテストファイルのどれも 1,000 行を超えさせない。

### Requirement 11: 決定論テストと実機確認

**Objective:** As a 開発者, I want 手続きの判断の分かれ目がテストで固定されていること, so that 後続の変更で黙って崩れない

#### Acceptance Criteria

1. The 本仕様 shall テストの中で組んだ書庫 5 種（ゴースト・バルーン同梱のゴースト・バルーン・`accept` 付きのシェル・`accept` 付きの追加ファイル）で、送られるイベントの名前・順・Reference を判定する。
2. The 本仕様 shall `accept` の一致（`sakura.name`・`install.accept` のそれぞれ）と不一致、`accept` の無い `shell`／`supplement` を判定する。
3. The 本仕様 shall 利用条件の 4 通り（ファイルなし・受諾・拒否・`AREKA_NO_ALERT` で抑止）を、画面を出さない形で判定する。
4. The 本仕様 shall 失敗理由の表（要件 5.2）を、`areka-nar` の拒否の 14 種の全部と I/O の失敗について判定する（値を印字するだけにしない）。
5. The 本仕様 shall 応えが無いときだけ `OnInstallComplete` へ続くこと・応えが無いときだけ areka が切り替えること・2 本以上が全部成功したときだけ `OnInstallCompleteAll` を送ることを判定する。
6. The 本仕様 shall 3 つの入口のそれぞれが同じ依頼を作ること（窓への投げ込みは偽のメッセージから・台本は台本から）と、相対パス・`path` 以外の引数・書庫でないファイル・フォルダ・混ざった投げ込みの振り分けを判定する。
7. The 本仕様 shall 起動中のゴーストへ入れる一周（降ろす → 入れる → 起こし直す → 締めの知らせ）と、展開が失敗して元の中身で起こし直す場合を、偽の SHIORI で判定する。あわせて、宛先が使用中で確定に失敗したとき、失敗の記録の作業フォルダの欄が実際の場所を持つことを判定する（完了 `nar-install` の申し送り）。
8. The 本仕様 shall 終了で待つ口を、実時間を待たない形で判定する（上限の内に終われば待つ・上限に達したら `warn!` を残して進む・まだ書いていない段では待たない・OS の終了では SHIORI の待ちと合わせて上限を超えない）。
9. The 本仕様 shall kanade の許可表が 23 語であることを判定する（既存の判定の数 13 を書き換える）。
10. The 本仕様 shall 既存のテストを置き換え無しに消さない。振る舞いが変わる行は新しい振る舞いを固定する形へ書き換える。
11. When 実機で確認する, the 開発者 shall 次を見て `signoff.md` に記録する: ⑴ ゴーストの `.nar` を絵の上へ落とす → 入る → 切り替わる ⑵ `\![change,ghost,lastinstalled]` で入れたゴーストへ切り替わる（記録に `ghost_switch_resolved name=lastinstalled` と `ghost_switch_done`）⑶ メニュー「インストール…」から入れる ⑷ 書庫でないファイルを落とす → `OnFileDrop2` が送られる ⑸ 絵の外へ落とす → areka の記録に受け取りが出ない ⑹ 起動中のゴーストの `.nar` を落とす → 引っ込んで戻る ⑺ 利用条件の画面で「はい」「いいえ」を手で押す。
12. When 実機で確認する, the 開発者 shall 手で落とす走行の前に、何が起きないのが正しいか（絵の外・透けた余白の位置）を先に決めて記録し、記録の水準をイベントの送出と判断の分かれ目が見える所まで開ける。

### Requirement 12: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want 答えで作業が変わる点が、推奨案と理由つきで並んでいること, so that 要件ディスカッションで覆すか決めるだけで済む

#### Acceptance Criteria

1. The 本仕様 shall **議題 ⑴（暫定）: 起動中のゴーストを上書きするとき**を「(a) 降ろしてから入れて起こし直す」とする（要件 7）。理由: (c)「断る」は `supplement` を 1 本も入れられなくする（`supplement` の宛先は必ず起動中のゴーストのフォルダ）。(b)「先に既定ゴーストへ切り替える」は、起動中のゴーストが既定ゴースト自身のときに切り替える先が無く、既定ゴーストの挨拶が一瞬挟まる。(a) は相手が誰でも同じ 1 本の道で済む。**代償**: 切替の道筋（`switch_to`）の「降ろした後・起こす前」に展開を挟む口が入る。テストファイル `ghost_switch_tests.rs`（987 行）の分割は要らない（`switch_to` に触れる行はそのファイルに 1 行だけで、新しいテストは新しい兄弟ファイルへ置ける＝ギャップ分析 §4 e）。完了 `ghost-shell-balloon-switch` の切替の入口の形を変えるなら、同 spec の見直しの引き金に当たる。展開を挟む形（間を空けて背景で展開するか、間を空けずに展開するか）は設計で決める。
2. The 本仕様 shall **議題 ⑵（暫定）: 巻き戻せなかったときの伝え方**を「`OnInstallFailure`（理由 `extraction`）＋`error!` に在りかと 7 日の期限」とする（要件 5.6）。正典に無い Reference は足さず（0 個）、元の中身を別のゴーストとして救い出すことはしない（部品が 7 日残すので後から足せる）。終了で断たれた場合（要件 8.3）も同じ扱いで、次の起動では記録だけが残る。在りかの説明は `alpha-release-signoff` の既知の制限へ申し送る。
3. The 本仕様 shall **議題 ⑶（暫定）: 利用条件の画面**を「はい／いいえの 2 択・閉じるボタンなし」とし、`AREKA_NO_ALERT` のときは拒否に倒す（要件 4.4・4.7）。理由: 閉じるボタンが無ければ、正典の「閉じるボタンでは何もイベントは発生しない」に反する場面が起きない。抑止のときに受諾へ倒すと、利用者が読んでいない条件を areka が代わりに受け入れることになる。
4. The 本仕様 shall **議題 ⑷（暫定）: 途中でアプリが終わるとき**を「(a) 展開の最中だけ上限つきで待つ」とし、上限は OS の終了でもそれ以外でも 3 秒とする（要件 8）。理由: (b)「待たない」は、終了の操作で元のゴーストのフォルダが消えうる。確定そのものは短い（フォルダの付け替え 2 回）ので、3 秒は進行中の確定を終えるのに足り、組み上げの途中で切れても宛先は無傷。終了の指示の後は確定の段へ入らない形にできれば待ちはさらに確実になる（`areka-nar` に手が入る・採るかは設計で決める）。
5. The 本仕様 shall **暫定の確定 5: 入れた後に誰が切り替えるか**を「ゴーストが完了の知らせに台本を返したらその台本に任せ、返さなければ areka が切り替える」とする（要件 6.2・6.3）。理由: 既存のゴーストの辞書には、完了の知らせの中で利用者に尋ねてから `\![change,ghost,lastinstalled]` を実行するものがある（里々 wiki「ゴースト切り替え」の作例）。areka が必ず切り替えると、その選択を上書きする。何も言わないゴースト（今日の emo2）では areka が切り替えるので、α の一周「落とす → 起動」は成り立つ。**採らなかった案**: 必ず切り替える／切り替えない（既定ゴーストの辞書に任せる）。
6. The 本仕様 shall **暫定の確定 6: 「応えが無い」の読み**を「返事なし（204）と空の台本」とする（要件 2.6）。
7. The 本仕様 shall **暫定の確定 7: `OnInstallComplete` の Reference0** を、バルーン同梱のゴーストでも `ghost` とする（正典が「廃止」と書く `ghost with balloon` は送らない）。
8. The 本仕様 shall **暫定の確定 8: イベントの前後**を「`OnInstallBegin` が先、`accept` の照合と利用条件の画面はその後」とし、利用条件を拒否した後は `OnGhostTermsDecline` だけを送る（`OnInstallFailure` の `artificial` は送らない）。理由: 正典は利用条件の画面を「インストールを中断して」出すと書く。拒否の知らせと失敗の知らせを続けて送ると、後の台詞が前の台詞を置き換える。
9. The 本仕様 shall **暫定の確定 9: `accept` の無い `shell`／`supplement`** を失敗（理由 `invalid type`）とする（要件 3.6）。理由: 正典は「必ず用いる」と書く。名指しの無い物を起動中のゴーストへ入れない。
10. The 本仕様 shall **暫定の確定 10: 失敗理由の語**を正典の 3 語に写す（要件 5.2）。`.zip` で `install.txt` が無い物も `OnInstallFailure`（`invalid type`）とし、`OnFileDrop2` へは回さない（brief のとおり）。
11. The 本仕様 shall **暫定の確定 11: 置換語の読み**を要件 6.8 のとおりとする（正典の「最後にイベントを行った」を「最後のインストールが関わった」と読む）。
12. The 本仕様 shall **暫定の確定 12: 複数を落としたとき**を「`OnInstallCompleteAll` は書庫が 2 本以上のときだけ」「混ざった投げ込みは、ファイル → フォルダ → 書庫の順」とする（要件 2.8・9.3）。理由: 書庫の手続きは時間がかかり、終わるとゴーストが替わりうるので、落とされた物の知らせは今のゴーストへ先に届ける。
13. The 本仕様 shall **暫定の確定 13: バルーンの窓への投げ込み**も受ける（要件 1.2。キャラクター窓とバルーン窓は同じ窓の様式を使う）。Reference1 はその窓のスコープ番号。
14. The 本仕様 shall **暫定の確定 14: バルーンだけを入れたときの「記憶」**を、起動中のゴーストの「最後に使ったバルーン」の記憶と読む（要件 6.6。brief の「記憶だけ更新」の読み）。
15. The 本仕様 shall **暫定の確定 15: 利用条件の 2 つのファイル**は `terms.txt` を先にし、長い本文は上限で切る（要件 4.2・4.9）。
16. The 本仕様 shall **暫定の確定 16: 投げ込みは切り出さない**。規模の見立ては 17〜20 タスク（brief の 15〜18 に、裁定 1 の (a) で +1、応えの有無を知る道で +1〜2）。設計で 20 を超える見立てになったら、書庫でない物の投げ込み（要件 9）を別の spec へ切り出す。
