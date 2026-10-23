# Brief: shiori4-api

> 起票: 2026-10-10（`/kiro-discovery`・開発者「SHIORI4（COM の SHIORI）の API 整備の spec はあるか。いまは最低限の決まりだけ作っていて、どこかの段階で着手しないといけない」）。
> **2026-10-23 書き直し**（`/kiro-discovery`・開発者「『shiori4』セッションと議論を行い、SHIORI4 インターフェースについて意見を出し合ってインターフェース案を調整せよ。以前から COM で作ろうかと原案を出していたインターフェースだが、クロスプラットフォームを見据え、C ABI でアクセスできるように仕様調整したものである」）。SHIORI4 の正本は COM の `IShiori` から、別リポジトリ `ekicyou/shiori4`（`C:\home\maz\git\shiori4`）の **C ABI**（`shiori4_abi`・`shiori4`・`shiori4_host` の 3 クレート、crates.io 0.0.1）へ移った。areka は**ベースウェア（ホスト）側の実装者**としてそれを使う。10-10 版の COM 前提の記述（3 層の食い違い・JSON-RPC の紙の契約・重ねるか並べるか）は、この書き直しで置き換えた。
> **区分 C・台帳の段は「その他」**（開発者・10-10）。**着手の条件**は「実装がある程度整備されてから」（同）に、10-23 から「shiori4 側の `host-protocol` が入った版が crates.io に出てから」を足す。

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

## 2026-10-23 の shiori4 セッションとの調整
areka のセッション（SHIORI4:areka）が、ベースウェアの実装者として 8 点を出し、shiori4 のセッションがすべてに同意した（3 だけ対案）。**どれも両セッションの推奨で、確定は開発者**。確定した語は **shiori4 リポジトリを正本**にする（pasta 側の「SHIORI4:pasta」セッションにも同じ表が渡っている）。

| # | 論点 | 合意した推奨 | shiori4 側の置き場 |
|---|---|---|---|
| 1 | プロパティの書き込みが無い | `ID: setproperty` ／ `Reference0: 名前` ／ `Reference1: 値` → 204 成功・400 不正。読みも `ID: getproperty` へ改名して get／set の対にする（0.0.1 の `ID: property` を壊すが、利用者がまだいない 0.0.x のうちに直す） | ② host-protocol |
| 2 | post の `Command` の語彙が空 | SSTP の語に寄せる。`Command: SEND` ＋ `Script`＝台本を再生する。`Command: NOTIFY` ＋ `Event`／`Reference*`＝ホストがそのイベントを SHIORI へ送る。post は「同じプロセスからの SSTP」として扱う（pasta の「今すぐ台本を出す」要望も SEND で満たす） | ② |
| 3 | 安全上の拒否を「存在しない（204）」と区別する | 400 ＋ `Reason: forbidden`（SHIORI/3.0 の形を崩さない・`X-` は付けない）。v4 だけに 403 を足す案と二択で、shiori4 側が開発者に出す | ② |
| 4 | プロパティの名前の決まり | ukadoc のプロパティシステムの名前をそのまま使う・ベースウェア固有の名前は接頭辞（例 `areka.`）で分ける・大文字小文字は区別しない・値の改行は取り除く（改行を含む値が要るプロパティが出たら別の名前で足す） | ② |
| 5 | `load` の dir の形 | v4 も v3 互換も「絶対パス・OS の区切り・末尾に区切りあり・UTF-8」 | ① abi-conventions |
| 6 | v4 の Charset | 要求に付けなくてよく、付いていても無視する。応答にも付けない | ① |
| 7 | v4 のヘッダの語彙 | 「v4 のヘッダの名前と意味は SHIORI/3.0（ukadoc）と同じ」と明記する。ホストが必ず付けるヘッダ（Sender・SecurityLevel・Status・BaseID）と、SHIORI の名前と版をホストが知る手段（`ID: version` の候補）も同じ所で決める（pasta の要望） | ② |
| 8 | `OnHostComplete` の届け方 | 既定は request（結果を台本で返しうるため）。SHIORI は request と notify の両方を受けられなければならない | ② |

shiori4 側はこれを 2 本の spec（① `abi-conventions`＝ヘッダの注釈・スレッドの意味・`SHIORI_E_WRONG_THREAD`・unload の後の約束など／② `host-protocol`＝上の表の 1〜4・7・8）で確定させる。2 本の brief は shiori4 の作業ブランチ `claude/abi-host-protocol` に起票済み（10-23 時点で main には未統合）。この spec は ② を上流として参照する。

### 3 者会談（10-23・shiori4 の開発者の指示で areka・pasta・shiori4）
shiori4 は記録係。areka と pasta が直接詰め、合意したものと割れたものを shiori4 へ送る。割れたものは shiori4 の開発者が裁定する。

| 議題 | 結果 |
|---|---|
| post の宛先 | 「post したインスタンス」に固定し、IfGhost などは読まない（pasta 案） |
| 拒否の返し方（表の 3） | areka と pasta が合意: 400 ＋ `Reason: forbidden` |
| 必ず付けるヘッダ（表の 7） | areka と pasta が合意: 必須は `ID`・`Sender`・`SenderType`・`SecurityLevel`・`SecurityOrigin`。`Status`・`BaseID`・`Reference*` は状況による。`SecurityLevel` の語は `local`／`external` だけ。post から起きた要求には `SenderType: internal`・`SecurityLevel: local` を付ける（areka は今もこの順で組んでいる＝`crates/shiori-host32-host/src/shiori3.rs`・`request_headers.rs`） |
| SHIORI の名前と版 | areka と pasta が合意: SHIORI/3.0 と同じ GET（`version`・`name`・`craftman`・`craftmanw`、200 ＋ `Value`）。ABI の版はヘッダに載せない |
| OnHostComplete の NOTIFY | areka と pasta が合意: Reference1 は無し。起こせたかどうかを 200／400 で返す |
| load の dir（表の 5） | pasta は問題なし（末尾の区切りの有無に関係なく読む） |
| SEND の割り込みと完了 | **詰めている途中**。完了は「再生が終わった時」に出し、Status は 200＋Reference1 `completed`／`interrupted`・204＝一度も再生せずに捨てた・400＋`Reason`＝拒否、までは areka と pasta が一致。**既定で割れる見込み**: shiori4・pasta は「既定は待つ・割り込みは明示（`Interrupt: true`）」、areka は「SSTP どおり既定で割り込み、`Option: nobreak` で待つ・優先度は Owned SSTP と同じ」（ukadoc spec_sstp の Option・ID と、完了 spec `mcp-kanade-tools` の裁定 1〜3「SEND は割り込む」に揃える）。割れたら shiori4 の開発者の裁定 |
| 割り込まれた側への知らせ | pasta 案は OnBalloonBreak。areka は反対: ukadoc は OnBalloonBreak を「SSTP以外でブレイクされた際に発生」と定めている。代わりに、Tag 付きの post の SEND で出したトークなら、その OnHostComplete が `interrupted` を運ぶ |
| OnHostComplete の応答の台本（areka が足した論点） | `interrupted` の知らせは割り込んだトークの再生中に届く。その応答を通常のイベントの応答と同じに扱うと、さらに置き換えてしまう。areka 案は `Option: nobreak` の SEND と同じに扱う（再生中なら待ち、待っている間に終了・切り替えが来たら捨てる） |

**areka の今**（10-23・コードで確かめた）: 再生の列は無く、トークの枠は 1 つ。SSTP の受け口は未実装（起票の行だけ）。SEND は MCP の `sakurascript`／`raise_event` だけで、必ず割り込む（`crates/areka-kanade/src/schedule/sstp.rs`）。OnSecondChange 以外のイベントの応答も再生中のトークを置き換える。`\t` は未実装（`sakura-time-critical`）。

## Desired Outcome
- **SHIORI4 は shiori4 の C ABI**。areka は `shiori4_host` を crates.io から使い、自前の COM の面（`crates/shiori-abi/`）とテスト用の COM の DLL（`crates/shiori4-testdll/`）を持たない。
- **経路は「進行役（kanade）→ 共通の窓口 `ShioriBackend` → SHIORI/3.0（32bit helper）／SHIORI4（同じプロセスの v4 DLL）」**（10-10 の開発者の方針「重ねず並べる」を引き継ぐ）。v4 のペイロードは SHIORI/3.0 のヘッダ部分そのものなので、要求を組むのは 1 つの組み立て器で、helper の道だけが開始行を足す。
- **SHIORI からの問い合わせが本番で使える**（10-10 から「いま特に欲しいもの」）: `getproperty`／`setproperty` が sylphya に届き、post の `SEND`／`NOTIFY` が、MCP の `sakurascript`／`raise_event` と同じ kanade の SEND／NOTIFY の口（`crates/areka-kanade/src/schedule/sstp.rs`）と安全度の扱いを通る。
- **プロパティへの入口が整理されている**: 入口が台本の指令・SHIORI4 の `host.request`・MCP のどれでも、名前の解決と許可の表は 1 か所（`property-query-channels` が決める所）を通る。
- descript.txt の `shiori` が指す DLL が `shiori_get_api` を持ち、areka と同じ arch なら、v4 で同じプロセスに読む。持たなければ今までどおり helper で読む。
- `load` に渡すパスは、v4 の道も helper の道も「絶対パス・OS の区切り・末尾に区切りあり」（上の表の 5）。
- `doc/COMPAT_ARCHITECTURE.md` §5 と roadmap の技術の要約が、C ABI の SHIORI4 と「並べる」経路を書いている。

## Approach
- **shiori4 を上流のクレートとして使う**（自前で ABI を持たない）。areka が書くのは `shiori4_host::Host` の実装（プロパティ → sylphya、post → kanade の SEND／NOTIFY の口、log → tracing、wake → SHIORI のスレッドへの合図）と、`ShioriBackend` の v4 版の実装だけ。理由: ABI の規約・post のキュー・unload の順序は `shiori4_host` が持っていて、同じものを二重に書かない。
- **`GhostRuntime` は SHIORI の専用スレッドで作る**（`!Send`。UI スレッドで SHIORI を回さない＝ゴーストの台本は重い）。`Host::wake` はそのスレッドへ「pump して」を積む。
- **post は「同じプロセスからの SSTP」として通す**。`SEND`／`NOTIFY` は MCP が使う kanade の口（`schedule/sstp.rs` の `on_send` など。SSTP の受け口そのものは areka に未実装）へ渡し、安全度（`script-security-level` が運び始めた SecurityLevel）は新しい規則を作らずに今の扱いに乗せる。
- **プロパティの許可は `property-query-channels` の表を使う**。この spec は表を作らない。
- **COM の面と JSON-RPC の封筒をやめる**（下の「開発者の判断」1・2）。446 項目の目録は ukadoc-survey が読むので残す。
- 規模: 10-10 の見立て（L 超・要件で切る）は、ABI・封筒・第三者向けの文書と見本が shiori4 側へ移ったので小さくなった。**1 本（20 タスク以内）に収まる見込み**。要件の段で数え直し、超えたときだけ切る。

## 開発者の判断（着手の前または要件の段で）
推奨つき。

1. **自前の COM の面 `crates/shiori-abi/` と `crates/shiori4-testdll/` を消し、テスト用の v4 DLL を `shiori4::Ghost` で作り直す**（推奨）。残すと二重の正本になる。
2. **`doc/shiori/fragments/_shared.toml` の JSON-RPC の封筒をやめ、「v4 のペイロード＝SHIORI/3.0 のヘッダ部分（正本は shiori4）」と書き換える**（推奨）。目録（events・resources）は残す。
3. 上の表の 3（拒否の返し方）: 400 ＋ `Reason: forbidden` か、v4 だけの 403 か。shiori4 側の推奨は前者。areka 側も前者でよい（areka は v3 を使わないが、SHIORI/3.0 と同じ形のほうが組み立て器を共有できる）。
4. 上の表の 5 を helper の道（32bit の SHIORI/3.0）にも当てるか。当てると、すべてのゴーストへ渡す `load` のバイト列が変わる（10-10 からの議題の候補）。推奨は当てる（SSP と同じ形になる。えみりの YAYA は今の形でも動いたので、変えても壊す見込みは低い）。

## Scope
- **In**:
  - `shiori4_host` への依存と `Host` の実装（getproperty／setproperty・post の SEND／NOTIFY・log・wake）。
  - `ShioriBackend` の v4 版の実装と、descript.txt の `shiori` からの選び分け（`shiori_get_api` の有無・arch）。
  - 要求の組み立て器を v3（helper）と v4 で共有する整理。
  - post の SEND を「待つ」扱いで受けるための待ちの列（3 者会談の裁定しだい。SSTP どおりなら `Option: nobreak` のときと OnHostComplete の応答のとき）と、再生の終わりで出す完了（`completed`／`interrupted`）。
  - `load` に渡すパスの形（上の表の 5・判断 4）。
  - COM の面とテスト用の COM の DLL の撤去、テスト用の v4 DLL（判断 1）。
  - 封筒の書き換え（判断 2）・`doc/COMPAT_ARCHITECTURE.md` §5・roadmap の技術の要約の書き直し。
  - 古くなった注釈の掃除（`crates/areka/src/main.rs`・`shiori_inproc.rs`・`crates/shiori-host32-host/src/client.rs` の「`IShiori` への写像点」）。
- **Out**: 下の「Out of Boundary」。

## Boundary Candidates
- `Host` の実装（受け側: プロパティは sylphya・post は kanade の SEND／NOTIFY の口）。
- v4 の DLL の読み込みと `GhostRuntime` の置き場（SHIORI の専用スレッド）。
- `ShioriBackend` の v4 版と、要求の組み立て器の共有。
- 撤去と文書（COM の面・封筒・COMPAT §5）。

## Out of Boundary
- **SHIORI4 の ABI と host-protocol の中身を決めること**。正本は shiori4 リポジトリ（① `abi-conventions`・② `host-protocol`）。areka は意見を出すが、自分の文書に二重に書かない。
- shiori4 の v3 互換モード（他 OS の互換ベースウェア向け）。areka は v4 だけを使う。従来の DLL は helper の道のまま。
- x86 の v4 DLL を helper 越しに読むこと（`host.request` をプロセス間で運ぶことになる）。
- 32bit の helper と IPC の中身の作り替え（`shiori-host32-helper`・`shiori-host32-ipc`）。32bit の SHIORI/3.0 の DLL からの台本を通さないプロパティの読み取りは `property-ipc-transport`。
- 台本の指令によるプロパティの読み書き（`\![get,property]` など）は `property-query-channels`。
- SHIORI/3.0 の要求へヘッダを足す仕事（`script-security-level`・`mcp-shiori-query`）。
- MAKOTO（`makoto-dll-host`。v4 の MAKOTO 版は作らない）。
- pasta を v4 に対応させること（別リポジトリ・「SHIORI4:pasta」セッション）。
- SSP を動かして挙動を確かめること。

## Upstream / Downstream
- **Upstream**: `ekicyou/shiori4` の `host-protocol`（未起票）が入った版の公開・`property-query-channels`（許可の表）・`script-security-level`（SecurityLevel の運び）・kanade の SEND／NOTIFY の口・sylphya。
- **Downstream**: x64 の v4 の pasta を本番で読むこと（roadmap の予約「pasta の native x64」）・第三者が v4 で SHIORI を書くこと。

## Existing Spec Touchpoints
- **Extends**: なし（完了済みの `shiori-com`・`shiori-protocol`・`shiori4-test-ghost`・`host32-shiori-load` の宿題を引き受け、その COM と JSON-RPC の部分は C ABI に置き換える）。
- **Adjacent**（同じファイルを触る＝同時に走らせない）:
  - 直列の列「host32-host」（`script-security-level` → `mcp-shiori-query` → `makoto-dll-host` → `makoto-reload-directives` → `property-ipc-transport`）。触るのは `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/{runtime.rs, shiori_inproc.rs, shiori_wiring.rs}`・kanade の `shiori/real.rs`。この spec はこの列の後ろ。
  - `property-ipc-transport`（据え置き）: 32bit の道のプロパティの輸送。v4 の道の `getproperty`／`setproperty` はこの spec が持つ（10-23 に持ち分を分けた）。
  - `property-query-channels`: 許可の表と、台本の指令の入口。
  - `audit-shiori-host32`・`clippy-199-lints`: `crates/shiori-abi/` を受け持ちに挙げている。判断 1 で消すなら、その行は消える。

## Constraints
- **優先度**: 区分 C・段「その他」（開発者・10-10）。急がない。
- **着手の条件**: 実装がある程度整備されてから（開発者・10-10）、かつ shiori4 の `host-protocol` が入った版が crates.io に出てから。着手のときに「Current State」を調べ直す。
- **依存の版**: shiori4 の 3 クレートは同じ版で出る（0.0.x）。areka のワークスペースのピンで 1 か所に持つ。
- **1,000 行の上限に近いファイル**: 着手のときに測り直す（10-10 の時点で kanade の `msg.rs`・`actor.rs`・`schedule/mod.rs`・`schedule/steady.rs`、`crates/areka/src/main.rs`、`crates/areka-ghost/src/runtime_tests.rs` が 900 行を超え、`crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` は 1135 行で超えている）。
- **host-32 は後付け**。進行役（kanade）の変更は、切り離す向きでだけ行う。
- 挙動の意味は ukadoc から決める。
- 要件定義のモデル: Opus（10-10 の見立ては Fable だったが、重ねるか並べるか・中身の形式という判断の山が shiori4 側の合意で片づき、残りは配線と撤去）。
