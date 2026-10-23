# Brief: shiori4-api

> 起票: 2026-10-10（`/kiro-discovery`・開発者「SHIORI4（COM の SHIORI）の API 整備の spec はあるか。いまは最低限の決まりだけ作っていて、どこかの段階で着手しないといけない」）。
> **2026-10-23 書き直し**（`/kiro-discovery`・開発者「『shiori4』セッションと議論を行い、SHIORI4 インターフェースについて意見を出し合ってインターフェース案を調整せよ。以前から COM で作ろうかと原案を出していたインターフェースだが、クロスプラットフォームを見据え、C ABI でアクセスできるように仕様調整したものである」）。SHIORI4 の正本は COM の `IShiori` から、別リポジトリ `ekicyou/shiori4`（`C:\home\maz\git\shiori4`）の **C ABI**（`shiori4_abi`・`shiori4`・`shiori4_host` の 3 クレート、crates.io 0.0.1）へ移った。areka は**ベースウェア（ホスト）側の実装者**としてそれを使う。10-10 版の COM 前提の記述（3 層の食い違い・JSON-RPC の紙の契約・重ねるか並べるか）は、この書き直しで置き換えた。
> **区分 C・台帳の段は「その他」**（開発者・10-10）。**版の段は v0.2.0**（2026-10-23 開発者「対応時期は、dolaなどの基幹部分大整備と同じタイミングで。すぐ必要ではないが、基幹部分となるので」＝「いつか」から移した）。**着手の条件**は「実装がある程度整備されてから」（同）に、10-23 から「shiori4 側の `host-protocol` が入った版が crates.io に出てから」を足す。

## Problem
areka の SHIORI の口は、32bit の SHIORI/3.0 の DLL を別プロセスの helper で読む道だけが本番で動いている。SHIORI からベースウェアへ問い合わせる口（プロパティの読み書き・台本の再生・イベントの依頼）は本番のどこにもつながっていない。

areka には COM の `IShioriFactory`／`IShiori`／`IShioriHost`（`crates/shiori-abi/`）と JSON-RPC の紙の契約（`doc/shiori/fragments/_shared.toml` の封筒）があるが、どちらも本番の経路に現れず、第三者が SHIORI を書く拠り所にもならない。開発者はこの COM の原案を、OS を問わず C ABI で呼べる形に作り直した（`ekicyou/shiori4`）。areka の側の古い原案は、それに置き換えないと二重の正本になる。

## Current State
2026-10-23 の調べ（main `ef96cb39`・コードは読んだだけ）。着手のときに調べ直すこと。

**shiori4（上流・別リポジトリ）の形**（main `72cff0b`・0.0.1）
- 公開シンボルは `shiori_get_api(版)` の 1 つ。`#[repr(C)]` の関数テーブル（`ShioriApi`＝load／unload／request／notify、`ShioriHostApi`＝request／post／log）。構造体は末尾にだけ足し、`struct_size` で有無を判定する。
- 版は v4（既定・ホストを呼べる）と v3（SHIORI/3.0 全文を運ぶ互換。他 OS の互換ベースウェア向け・host は NULL）。
- **v4 のペイロードは SHIORI/3.0 のヘッダ部分そのもの**（開始行なし・状態は戻り値・UTF-8 固定）。
- SHIORI → ホスト: 同期の `host.request`（SHIORI のスレッドで、load／request／notify の処理中だけ）と、任意のスレッドからの非同期の `host.post`（`Tag` で完了を `OnHostComplete` として返す）。
- 規約: ホストが決めた単一のスレッドからだけ呼ぶ・`host.request` の中から SHIORI を呼び返さない（`SHIORI_E_REENTRANT`）・unload で未配送を捨てる。
- ホスト側の高水準層 `shiori4_host`: `Host` trait（`property`・`request`・`accept_post`・`on_post`・`log`・`wake`）と `GhostRuntime`（`!Send`・作ったスレッドで request／notify／pump）。
- `host_protocol` はまだ「暫定」（読みの `ID: property` だけ。書き込みと post の語彙は無い）。

**areka の今**
- 本番は `ShioriWiring::Helper` に固定（`crates/areka/src/boot_config.rs`・`ghost_session.rs`・`emo2_boot/mod.rs`）。進行役（kanade）の共通の口 `ShioriBackend`（`crates/areka-kanade/src/shiori/real.rs`）がイベント名と番号付きの Reference の並びを受け、`crates/shiori-host32-host/src/shiori3.rs` が SHIORI/3.0 のテキストを組む。
- COM の面 `crates/shiori-abi/`（依存元は `crates/areka`・`crates/areka-ghost`・`crates/shiori4-testdll`）。`IShioriHost` の実装は 3 つあり、どれも本番に届かない（`crates/areka/src/shiori_host.rs` の `ShioriHostSink`＝プロパティは sylphya 接続・env で有効にするデモだけ／`crates/areka-ghost/src/shiori_inproc.rs` の `InProcHost`＝孤立した表／Helper の経路＝口なし）。
- `ShioriWiring::InProc`（x64 の DLL を同じプロセスに読む）を通るのはテスト用の `crates/shiori4-testdll` だけ。descript.txt から選ぶ道は無い。
- `doc/shiori/fragments/` は封筒（`_shared.toml`・JSON-RPC 2.0）と、イベントと資源の目録（446 項目）の 2 つ。目録は `crates/ukadoc-survey/src/evidence/resolve.rs` が読む。封筒を読むコードは無い。
- `doc/COMPAT_ARCHITECTURE.md` §5 と `.kiro/steering/roadmap.md` の技術の要約は「SHIORI 内部唯一 ABI＝`IShiori`（COM, HSTRING/UTF-16）」のまま。
- `load` に渡すフォルダのパスは `…\ghost/master`（区切りが混ざり、末尾に区切りが無い）。出どころは `crates/areka-parsers/src/package/resolve_shell.rs` の `GHOST_MASTER` を `join` する所（`shiori_inproc.rs` も helper の `shiori_proxy.rs` の `init_bytes` も整えずに渡す）。

## 2026-10-23 の調整（shiori4・pasta との 3 者会談）
areka のセッション（SHIORI4:areka）が、ベースウェアの実装者として「shiori4」と「SHIORI4:pasta」の両セッションと詰めた。途中から shiori4 の開発者の指示で 3 者会談になった（shiori4 は記録係・割れたら shiori4 の開発者が裁定）。**割れた論点は無い**。

**正本（語を areka の文書へ写さず、ここを参照する）**: shiori4 リポジトリの作業ブランチ `claude/abi-host-protocol`（10-23 時点で main には未統合）。
- `.kiro/specs/host-protocol/brief.md` の「合意した論点」1〜9（`5cabbfa`）: `getproperty`／`setproperty`・post の `SEND`／`NOTIFY`（SSTP の語）・宛先は post したインスタンス・プロパティ名の決まり・必須のヘッダ・`ID: version` など・OnHostComplete の並び・拒否は 400＋`Reason: forbidden`・SEND は SSTP どおり既定で割り込み `Option: nobreak` で待つ。
- `.kiro/specs/abi-conventions/brief.md`: load の dir は「絶対パス・OS の区切り・末尾に区切りあり・UTF-8」・v4 は Charset を付けない・スレッドと unload の規約・論点 11「ホストは SHIORI を専用のプロセスで読むことがある。SHIORI はホストと同じプロセスにいることを前提にしない」（areka の決定 3 を受けて shiori4 が足した）。
- 要件の段に入ったら、それぞれの requirements.md を参照する。

**追加で合意した論点 A〜F**（areka が出し、shiori4・pasta が賛成。記録は `b69856d`＝C・D・E は host-protocol の論点 10〜12、A・B・F は abi-conventions の「追加の論点」8〜10。2 本の brief の論点はすべて合意済みで、この先は shiori4 の開発者が要件の段で承認する）:
- A: 同じ DLL のファイルを 2 体のゴーストが使うと、同じプロセスで load が 2 回呼ばれる。SHIORI は 2 つ目を断ってよい（load が NULL＋log）が、できれば受けるべき。pasta は受けられる見込み（Lua VM はアクタースレッドごと。プロセスに 1 つの mailbox などはインスタンスへ移し、デバッグの待受は 2 体目以降では開かない。入れる版は pasta の要件で決める）。
- B: 「ホストが決めた単一のスレッド」はインスタンスごと。`SHIORI_E_WRONG_THREAD` の判定もインスタンスごと。
- C: post の SEND は SSTP の Option の `nobreak`・`notranslate`・`strict` を ukadoc どおり読む。`nodescript` は読まない。知らない語は無視する。
- D: post の NOTIFY のイベント名に制限は掛けない。ホストは安全上の理由で 400＋`Reason: forbidden` で断れる。
- E: `Reason` の語は当面 `forbidden` だけ。未知の語は理由不明の拒否と読む。
- F: unload は長く止まらないこと。ホストは OS の終了の猶予の中で各インスタンスの unload を呼ぶことがある（時間の数字は書かない）。

**この合意で areka が引き受ける仕事**（正本の語のとおりに作る）:
- `Host` の実装で `getproperty`／`setproperty` に答える（許可は `property-query-channels` の表・拒否は 400＋`Reason: forbidden`）。
- post の `SEND`／`NOTIFY` を、MCP と同じ kanade の口へ通す。SEND は既定で割り込み、`Option: nobreak` のときだけ待つ。優先度は Owned SSTP と同じ。
- **待ちの列を作る**（`nobreak` の SEND と、OnHostComplete の応答の台本の 2 つ）。areka は今トークの枠が 1 つで、列を持たない。
- SEND の完了は再生が終わった時に出す（`completed`／`interrupted`・一度も再生せずに捨てたら 204）。
- v4 の要求は、今の SHIORI/3.0 の組み立て器（`crates/shiori-host32-host/src/request_headers.rs` の `request_of`）から開始行と Charset を外して作る。post から起きた要求には `SenderType: internal`・`SecurityLevel: local` を付ける。
- 論点 A・B は、areka では「1 つの SHIORI に 1 つのプロセス」（下の決定 3）なので、同じプロセスで load が 2 回呼ばれることは無い。`GhostRuntime` は helper のプロセスの中で、そのプロセスの専用スレッドに置く。

**areka の今**（10-23・コードで確かめた）: 再生の列は無く、トークの枠は 1 つ。SSTP の受け口は未実装（起票の行だけ）。SEND は MCP の `sakurascript`／`raise_event` だけで、必ず割り込む（`crates/areka-kanade/src/schedule/sstp.rs`・完了 spec `mcp-kanade-tools` の裁定 1〜3）。OnSecondChange 以外のイベントの応答も、再生中のトークを置き換える。`\t` は未実装（`sakura-time-critical`）。OnBalloonBreak は利用者の中断のときだけ出す（ukadoc「SSTP以外でブレイクされた際に発生」と一致）。

## Desired Outcome
- **SHIORI4 は shiori4 の C ABI**。areka は `shiori4_host` を crates.io から使い（helper のプロセスの中で）、自前の COM の面（`crates/shiori-abi/`）とテスト用の COM の DLL（`crates/shiori4-testdll/`）を持たない。
- **SHIORI の実体は、どの版でも別プロセスで動く**（決定 3）。areka の本体のプロセスは SHIORI の DLL を読まない。`ShioriWiring::InProc` の道は無くなる。
- **経路は「進行役（kanade）→ 共通の窓口 `ShioriBackend` → helper のプロセス → SHIORI/3.0 の DLL／SHIORI4 の DLL」**（10-10 の開発者の方針「重ねず並べる」を引き継ぐ）。v4 のペイロードは SHIORI/3.0 のヘッダ部分そのものなので、要求を組むのは 1 つの組み立て器で、SHIORI/3.0 の道だけが開始行と Charset を足す。
- **SHIORI からの問い合わせが本番で使える**（10-10 から「いま特に欲しいもの」）: `getproperty`／`setproperty` が sylphya に届き、post の `SEND`／`NOTIFY` が、MCP の `sakurascript`／`raise_event` と同じ kanade の SEND／NOTIFY の口（`crates/areka-kanade/src/schedule/sstp.rs`）と安全度の扱いを通る。
- **プロパティへの入口が整理されている**: 入口が台本の指令・SHIORI4 の `host.request`・MCP のどれでも、名前の解決と許可の表は 1 か所（`property-query-channels` が決める所）を通る。
- descript.txt の `shiori` が指す DLL が `shiori_get_api` を持てば v4、持たなければ SHIORI/3.0 として、どちらも helper のプロセスで読む。v4 の DLL の arch（x64・arm64・x86）に合う helper を起こす（別プロセスなので、areka 本体の arch に縛られない）。
- helper のプロセスの環境は SHIORI/3.0 の道と同じ（カレントディレクトリはゴーストの `ghost/master`・親が死んだら一緒に終わるジョブ・終わらせ方）。
- `load` に渡すパスは、v4 の道も helper の道も「絶対パス・OS の区切り・末尾に区切りあり」（abi-conventions）。
- `doc/COMPAT_ARCHITECTURE.md` §5 と roadmap の技術の要約が、C ABI の SHIORI4 と「並べる」経路を書いている。

## Approach
- **shiori4 を上流のクレートとして使う**（自前で ABI を持たない）。理由: ABI の規約・post のキュー・unload の順序は `shiori4_host` が持っていて、同じものを二重に書かない。
- **helper のプロセスの中で `shiori4_host` を使う**。helper の中の `Host` の実装は、受けたものを IPC で親へ運ぶだけの中継（`getproperty`／`setproperty` は親に聞いて答える・post と log は親へ送る・wake は helper のメッセージの輪へ「pump して」を積む）。`GhostRuntime`（`!Send`）は helper の専用スレッドに置く。
- **areka の本体の側**は、IPC で届いた問い合わせに答える受け側（プロパティ → sylphya・post → kanade の SEND／NOTIFY の口・log → tracing）と、`ShioriBackend` の v4 版（今の helper の道と同じ形の IPC の客）を書く。
- **IPC に札を足す**。今の札は Hello／Load／Request／Response／Unload の 5 つ（`crates/shiori-host32-ipc/src/lib.rs`）。v4 には、helper → 親の同期の問い合わせ（`host.request`。親は Request の応答を待つ間にこれに答える・答える間に親から SHIORI を呼び返さない＝ABI の再入禁止と同じ）・その答え・helper → 親の post（非同期）・log の札が要る。
- **helper のプロセスを起こす仕組み**（`crates/shiori-host32-host/src/process_host.rs` のカレントディレクトリの設定・`job.rs`・`lifecycle.rs`・`terminator.rs`）は SHIORI/3.0 の道のものを使う。
- **post は「同じプロセスからの SSTP」として通す**。`SEND`／`NOTIFY` は MCP が使う kanade の口（`schedule/sstp.rs` の `on_send` など。SSTP の受け口そのものは areka に未実装）へ渡し、安全度（`script-security-level` が運び始めた SecurityLevel）は新しい規則を作らずに今の扱いに乗せる。
- **プロパティの許可は `property-query-channels` の表を使う**。この spec は表を作らない。
- **COM の面と JSON-RPC の封筒をやめる**（下の「開発者の判断」1・2）。446 項目の目録は ukadoc-survey が読むので残す。
- 規模: ABI・封筒・第三者向けの文書と見本は shiori4 側へ移った。一方で決定 3（別プロセス）で、v4 の helper と IPC の札が加わった。**20 タスクを超える見込み**。要件の段で切る。切る候補は「v4 の helper と IPC の札」「本体の受け側と kanade の待ちの列」「撤去と文書」の 3 つ。

## 開発者の決定（2026-10-23）
開発者「areka の立場としては、COM-SHIORI インターフェースの作りかけのものはもう排除しちゃって、shiori4 に完全移行するつもりで考えてほしい。shiori4 の ABI なら、Windows 以外でもそのまま動きますので」。

1. **COM の面を消し、shiori4 へ完全に移る**。`crates/shiori-abi/` と `crates/shiori4-testdll/` を消し、`IShioriHost` の 3 つの実装（`crates/areka/src/shiori_host.rs` の `ShioriHostSink`・`crates/areka-ghost/src/shiori_inproc.rs` の `InProcHost`・env で有効にするデモ）も消す。テスト用の v4 DLL は `shiori4::Ghost` で作り直す。
2. **JSON-RPC の封筒もやめる**（COM の原案と同時に作った紙の契約なので、同じ決定に含めた）。`doc/shiori/fragments/_shared.toml` の封筒は「v4 のペイロード＝SHIORI/3.0 のヘッダ部分（正本は shiori4）」へ書き換える。目録（events・resources）は `ukadoc-survey` が読むので残す。
3. **SHIORI の実体は別プロセスで動かす**（開発者・同日「arekaにだけ影響する設計上の大きな要件を出します。SHIORIの実体は別プロセスで動かすこと。これは、SHIORI3がブリッジで動いているのと、同じような環境で動いて欲しいためです。プロセスが同じだと、カレントディレクトリなどのシステム環境の影響を受けたり、pythonなどのプロセスでグローバル環境を持つランタイムが使いにくいため。あとプロセス分離は最近のアーキテクチャのトレンドでもあります」）。v4 の DLL も同じプロセスに読まない。shiori4 の ABI は変わらない（ホストがどのプロセスで DLL を読むかは ABI の外）。
   - **製品は別プロセスだけ**（開発者・同日。「別プロセスの版と同じプロセスの版を 2 つ作るか…でも設計者が混乱しそう」への推奨を承認）。同じプロセスで SHIORI を動かす道は、設定の項目も選ぶ道も作らない。理由: 2 つあると「同じ環境で動く」という決定の狙いが崩れ（カレントディレクトリ・プロセス全体の状態が版で違う）、試験と不具合の切り分けも倍になる。速さの得（問い合わせの IPC の往復が要らない）は小さい見込み（未計測）。
   - **同じプロセスで回すのはテストの道具に限る**。helper の中身（中継の `Host` と `GhostRuntime` を回すところ）をライブラリに切り出し、テストのときだけテストのプロセスのスレッドで回す（プロセスを起こさない決定論の試験のため）。速さが要ると測って分かったら、その切り出しの上に同じプロセスの道を後から足せる（今は作らない）。

## 開発者の判断（着手の前または要件の段で）
推奨つき。

1. **helper の exe を 1 つにするか、v4 用を分けるか**。推奨は 1 つ（1 種類の helper の exe が SHIORI/3.0 と SHIORI4 の両方を受け持つ。DLL を読んだあと `shiori_get_api` があれば v4、`load`／`request`／`unload` だけなら SHIORI/3.0・両方を持つ DLL は v4。exe を共有するだけで、プロセスは SHIORI 1 つにつき 1 つのまま）。起こし方・ジョブ・終わらせ方・IPC の札の読み手を 1 つで持てる。今は i686 だけを建てているので、v4 の DLL のために x64・arm64 の版も建てる。
2. **v4 の DLL の arch の決め方**。推奨は DLL の PE ヘッダの機械の種類を読んで helper を選ぶ（違う arch の helper で読み込みに失敗させてから試し直すより確実）。
3. abi-conventions の load の dir の形を helper の道（32bit の SHIORI/3.0）にも当てるか。当てると、すべてのゴーストへ渡す `load` のバイト列が変わる（10-10 からの議題の候補）。推奨は当てる（SSP と同じ形になる。えみりの YAYA は今の形でも動いたので、変えても壊す見込みは低い）。

## Scope
- **In**:
  - helper の v4 の読み方（`shiori4_host` への依存・helper の中の中継の `Host`・`GhostRuntime` の置き場）と、x64・arm64・i686 の helper の建て方。
  - IPC の札の追加（同期の問い合わせとその答え・post・log）と、親の側で Request の応答を待つ間に問い合わせに答える仕組み。
  - 本体の受け側（getproperty／setproperty → sylphya・post の SEND／NOTIFY → kanade・log → tracing）。
  - `ShioriBackend` の v4 版の実装と、descript.txt の `shiori` からの選び分け（`shiori_get_api` の有無・DLL の arch）。
  - 要求の組み立て器を v3（helper）と v4 で共有する整理。
  - post の SEND を待たせるための待ちの列（3 者会談の合意＝`Option: nobreak` のときと OnHostComplete の応答のとき。areka は今トークの枠が 1 つで列を持たない）と、再生の終わりで出す完了（`completed`／`interrupted`）。
  - `load` に渡すパスの形（abi-conventions・判断 3）。
  - COM の面とテスト用の COM の DLL と `IShioriHost` の実装と `ShioriWiring::InProc` の道の撤去、テスト用の v4 DLL（決定 1・3）。
  - 封筒の書き換え（決定 2）・`doc/COMPAT_ARCHITECTURE.md` §5・roadmap の技術の要約の書き直し。
  - 古くなった注釈の掃除（`crates/areka/src/main.rs`・`shiori_inproc.rs`・`crates/shiori-host32-host/src/client.rs` の「`IShiori` への写像点」）。
- **Out**: 下の「Out of Boundary」。

## Boundary Candidates
- helper の v4 の読み方と中継の `Host`（helper のプロセスの中）。
- IPC の札（親と helper の間の契約）。
- 本体の受け側（プロパティは sylphya・post は kanade の SEND／NOTIFY の口）。
- `ShioriBackend` の v4 版と、要求の組み立て器の共有。
- 撤去と文書（COM の面・封筒・COMPAT §5）。

## Out of Boundary
- **SHIORI4 の ABI と host-protocol の中身を決めること**。正本は shiori4 リポジトリ（① `abi-conventions`・② `host-protocol`）。areka は意見を出すが、自分の文書に二重に書かない。
- shiori4 の v3 互換モード（他 OS の互換ベースウェア向け）。areka は v4 だけを使う。従来の DLL は helper の道のまま。
- SHIORI/3.0 の道の helper の動き（v4 のために札と読み分けを足すだけで、SHIORI/3.0 の要求と応答の運び方は変えない）。32bit の SHIORI/3.0 の DLL からの台本を通さないプロパティの読み取りは `property-ipc-transport`（この spec が足す問い合わせの札を使えるなら使う）。
- helper のプロセスと本体の間の通信を、Windows 以外でも動く形にすること（areka は Windows のアプリ。WM_COPYDATA のまま）。
- 台本の指令によるプロパティの読み書き（`\![get,property]` など）は `property-query-channels`。
- SHIORI/3.0 の要求へヘッダを足す仕事（`script-security-level`・`mcp-shiori-query`）。
- MAKOTO（`makoto-dll-host`。v4 の MAKOTO 版は作らない）。
- pasta を v4 に対応させること（別リポジトリ・「SHIORI4:pasta」セッション）。
- SSP を動かして挙動を確かめること。

## Upstream / Downstream
- **Upstream**: `ekicyou/shiori4` の `host-protocol`（未起票）が入った版の公開・`property-query-channels`（許可の表）・`script-security-level`（SecurityLevel の運び）・kanade の SEND／NOTIFY の口・sylphya。
- **Downstream**: v4 の pasta を本番で読むこと（roadmap の予約「pasta の native x64」。決定 3 で同じプロセスでなく helper で読む）・第三者が v4 で SHIORI を書くこと（Python などプロセス全体の状態を持つランタイムを抱えた SHIORI も含む）。

## Existing Spec Touchpoints
- **Extends**: なし（完了済みの `shiori-com`・`shiori-protocol`・`shiori4-test-ghost`・`host32-shiori-load` の宿題を引き受け、その COM と JSON-RPC の部分は C ABI に置き換える）。
- **Adjacent**（同じファイルを触る＝同時に走らせない）:
  - 直列の列「host32-host」（`script-security-level` → `mcp-shiori-query` → `makoto-dll-host` → `makoto-reload-directives` → `property-ipc-transport`）。触るのは `crates/shiori-host32-host/src/{shiori3.rs, client.rs, process_host.rs}`・`crates/shiori-host32-helper/src/`・`crates/shiori-host32-ipc/src/lib.rs`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs, shiori_wiring.rs}`・kanade の `shiori/real.rs`。この spec はこの列の後ろ。
  - `property-ipc-transport`（据え置き）: 32bit の SHIORI/3.0 の道のプロパティの輸送。どちらも IPC に札を足すので、札の番号と問い合わせの形は先に着手した方が決め、後の方はそれを使う。v4 の道の `getproperty`／`setproperty` はこの spec が持つ（10-23 に持ち分を分けた）。
  - `makoto-dll-host`: MAKOTO 1 本につき helper をもう 1 プロセス起こす案。helper の起こし方を共有する。
  - `property-query-channels`: 許可の表と、台本の指令の入口。
  - `audit-shiori-host32`・`clippy-199-lints`: `crates/shiori-abi/` を受け持ちに挙げている。決定 1 で消すので、その行は消える。

## Constraints
- **優先度**: 区分 C・段「その他」（開発者・10-10）。版は v0.2.0（wintf・dola の基幹の整備と同じ時期・開発者・10-23）。
- **着手の条件**: 実装がある程度整備されてから（開発者・10-10）、かつ shiori4 の `host-protocol` が入った版が crates.io に出てから。着手のときに「Current State」を調べ直す。
- **依存の版**: shiori4 の 3 クレートは同じ版で出る（0.0.x）。areka のワークスペースのピンで 1 か所に持つ。
- **1,000 行の上限に近いファイル**: 着手のときに測り直す（10-10 の時点で kanade の `msg.rs`・`actor.rs`・`schedule/mod.rs`・`schedule/steady.rs`、`crates/areka/src/main.rs`、`crates/areka-ghost/src/runtime_tests.rs` が 900 行を超え、`crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` は 1135 行で超えている）。
- **host-32 は後付け**。進行役（kanade）の変更は、切り離す向きでだけ行う。
- 挙動の意味は ukadoc から決める。
- 要件定義のモデル: Opus（10-10 の見立ては Fable だったが、重ねるか並べるか・中身の形式という判断の山が shiori4 側の合意で片づいた。決定 3 で増えた IPC の入れ子の往復は、今の helper の道の形を広げるもの）。
- **WM_COPYDATA は同期の送り**なので、helper → 親の問い合わせは、親が Request の応答を待っている間に届く。親の待ち方がそれを受けて答えられる形でないと、互いに待って止まる。設計の段で最初に確かめる。
