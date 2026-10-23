# shiori2-protocol 検証の記録（下書き）

5.x の実機の結果はこのファイルへ足していく。

## 4.3 3.0 の回帰（要件 6.1・6.2・6.3・7.5）

### 決定論のテスト（2026-10-23・机なし）

前準備: `cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`・`cargo build -p shiori-host32-testdll --target i686-pc-windows-msvc`（通信の層の結合テストが要する i686 の成果物）。

| 命令 | 結果 |
|---|---|
| `cargo test -p shiori-host32-host` | 緑・7 実行ファイル・202 通過・0 失敗・1 除外（`console_window_probe_child`＝別のテストが子として起こす観測の本体で、既定で除外の印つき） |
| `cargo test -p areka-kanade` | 緑・3 実行ファイル・773 通過（単体 688・結合 85）・0 失敗・0 除外 |
| `cargo test -p areka-ghost` | 緑・4 実行ファイル・253 通過（単体 207・結合 40・`real_pasta_online` 6）・0 失敗・0 除外 |

この回では `HOST32_PASTA_DLL` を与えていないので、条件つきの `real_pasta_online::emo2_does_not_start_chatting_while_online`・`snapshot_capture_test::capture_real_pasta_snapshots`（と `real_pasta_test`）は門で抜けて「通過」に数えられている。実物の pasta で回した結果は次の節。

### 確かめの 1 本と記録する要求の並び（静的）

確かめの往復は `crates/areka-ghost/src/shiori_wiring.rs` の `real_connect` の閉包の中で、LOAD の ack の直後・`ShioriConnection` を返す前に 1 回だけ行う。条件つきの 2 本は交信の記録（`Recorder`）で `real_connect` の返した接続を包むので、確かめの 1 本は記録の外で済み、記録する要求の並びには入らない。

### 実物の pasta と中継で回る条件つきの 2 本（机が要る）

置き場はすべてこのワークツリーの `target\` の下。

#### 1 回目（2026-10-23）: 2 本とも赤

`HOST32_PASTA_DLL` には、`vendors/sample_ghost/emo2.nar` の `ghost/master/pasta.dll` を `target\s2p-4-3\pasta.dll` へ単体で取り出したものを渡した。`TMP`・`TEMP` は `target\s2p-4-3\tmp` に向けた。
- `real_pasta_online`: 失敗 1・通過 5。helper の LOAD が `LoadReturnedFalse { last_error: 203 }` で失敗し、記録は空のまま挨拶を 120 秒待って panic した（`real_pasta_online.rs:192`）。記録が空なので、要求の並びは比べられていない。
- `capture_real_pasta_snapshots`: テストのスレッドがスタックを使い切った（`STATUS_STACK_OVERFLOW` `0xc00000fd`）。書き出しは 0 件。
- ログ: `target\s2p-4-3\online.log`・`target\s2p-4-3\snapshot.log`

調べた結果（机なし）:
- スタックの溢れは、こちらの命令の誤り。`write_real_pasta_ghost_fixture` は DLL の親のフォルダを丸ごと再帰で写す。そのフォルダ（`target\s2p-4-3`）の中に一時フォルダ（`TMP`）を置いたので、写し先が写し元の中にあり、`copy_dir_contents_recursive` が終わらずに再帰を重ねた（`snapshot_capture_test.rs` の 230〜263 行目）。本 spec の差分は関わらない。また、このテストは DLL の親が本物の emo2 の `ghost/master`（`dic/`・`scripts/`・`pasta.toml` 込み）であることを前提にしている。
- LOAD の失敗は、確かめの往復より前の段で起きている（`real_connect` の手順 5。確かめは手順 6）。本 spec の差分（`git diff main...HEAD`）は 32 ビットの仲介（`shiori-host32-helper`）にも、LOAD を送る手順 1〜5 にも触れていない。完了 spec `areka-P0-emo2-real-run-stability` が扱った「新しい写しからの最初の LOAD が `LoadReturnedFalse` で断られる」件と同じ形で、あの spec は 32 回くり返して再現せずに閉じている。今回の `last_error: 203`（ERROR_ENVVAR_NOT_FOUND）は、あの spec が「次に出たときの手がかり」として仲介に足した値の、初めての観測になる。失敗した回の `profile\pasta\logs\pasta.log` は、写しを消すので残っていない。

#### 2 回目に回す命令

DLL は emo2 の `ghost/master` を丸ごと `target\s2p-4-3\emo2\ghost\master` へ取り出した中のものを渡す。一時フォルダはその外（`target\s2p-4-3\tmp`）に置き、snapshot の 1 本のときだけ向ける。

```powershell
$wt = 'C:\home\maz\git\areka\.claude\worktrees\kiro-start-shiori2-protocol-22a93e'
$env:HOST32_PASTA_DLL = "$wt\target\s2p-4-3\emo2\ghost\master\pasta.dll"
cargo test --manifest-path "$wt\Cargo.toml" -p areka-ghost --test real_pasta_online -- --nocapture
$env:AREKA_SNAPSHOT_OUT = "$wt\target\s2p-4-3\snapshots"
New-Item -ItemType Directory -Force "$wt\target\s2p-4-3\tmp" | Out-Null
$env:TMP = "$wt\target\s2p-4-3\tmp"; $env:TEMP = $env:TMP
cargo test --manifest-path "$wt\Cargo.toml" -p areka-ghost --test ghost snapshot_capture_test::capture_real_pasta_snapshots -- --nocapture
```

結果（2026-10-23・机を持って 1 回）: 2 本とも緑。
- `real_pasta_online`: 通過 6・失敗 0（79.16 秒）。LOAD は通り、`emo2_does_not_start_chatting_while_online` は記録の並びに置いた主張（起動の挨拶・`online` の間の雑談 0 件・守り手を落とした後の応答）をすべて満たした。
- `capture_real_pasta_snapshots`: 通過 1・失敗 0（1.37 秒）。書き出しは GET の初出の 3 件（`OnFirstBoot`・`OnTranslate`・`username`）で、確かめの 1 本（`GET Version`）は記録に入っていない。
- ログ: `target\s2p-4-3\online2.log`・`target\s2p-4-3\snapshot2.log`

確かめの 1 本が先に届いても、記録する要求の並びは変わらなかった（要件 6.1・6.3）。1 回目の LOAD の失敗は 2 回目では出なかった。

### 4.1 で変えた kanade のテスト（`ShioriConnection` の書き下しに `version: ShioriVersion::Shiori3` の 1 行を足しただけ）

- `crates/areka-kanade/tests/kanade/common/common_window_actor.rs`
- `crates/areka-kanade/tests/kanade/idle_pump_test.rs`
- `crates/areka-kanade/tests/kanade/real_helper_test.rs`

4.3 で変えたテスト: なし。

## 5.1 「．さくら」を areka で動かす実機の 1 回（要件 1.1・5.1〜5.7）

### 置き場と命令

根・記憶・一時フォルダ・記録は、すべてこのワークツリーの `target\s2p-5-1\` の下（`root\ghost\dot_sakura`・`root\ghost\emo2`・`root\balloon\bottle`・`root\balloon\emo2-kakukaku`・空の `profile\`・`tmp\`・`logs\`）。根の組み方とビルドは `doc/pasta-real-run.md` の 3 節と同じ（`nar-sample-path` で `emo2` と `dot_sakura` を開く・`cargo build -p areka`・`cargo build -p shiori-host32-helper --target i686-pc-windows-msvc`・`tools/sync-i686-helper.ps1`）。起動は 4 節の形で、替えたのは次の所だけ。

- `--ghost "<w>\target\s2p-5-1\root\ghost\dot_sakura"`・`AREKA_ROOT`／`AREKA_PROFILE_DIR` を `target\s2p-5-1` の下・`TMP`／`TEMP` を `target\s2p-5-1\tmp`・`AREKA_APP_SMOKE_EXIT_MS=900000`
- `RUST_LOG=info,ghost-boot=info,shiori2=debug,kanade=trace,areka_kanade::resource=debug,areka_sylphya::actor=debug`（`kanade=trace` は要求ごとの `shiori_request` を数えるため）
- 観測は areka の MCP（`get_active_ghost_list` が `．さくら` を答えることで、道具が SSP でなくこの areka を指していることを確かめた・待受 9801）。画面の操作（SendInput・computer-use）は使っていない

### 1 回目（run1・2026-10-23 23:29〜23:30）: 要件 5.5 で停止

版は 2.x に決まった（下の run2 と同じ INFO の行）が、`OnFirstBoot`・`OnBoot`・`username` を含む GET が全部「返す物が無い」になり、台本 0 件・面 0（`dump_surface` は scope 0・1 とも `NG:No surface has been shown in this scope yet`）。`shiori_error_response` は 0。原因は `load` に渡すフォルダのパスの形（`…\dot_sakura\ghost/master`・区切りが混ざり末尾に区切りが無い）で、華和梨がそこへファイル名を継いで辞書を 1 つも読めなかった。`shiori4-api` の決定 6 を前倒しした 4.4（絶対パス・`\` の区切り・末尾に `\` が 1 つ）で直し、run2 を回した。外から送った `OnClose` に台本が返らなかったので areka は終わらず、起こした pid だけを止めた（終了コード -1）。記録: `target\s2p-5-1\logs\run1.*`。

### 2 回目（run2・2026-10-24 00:32:02〜00:36:07・4.4 の後）: 通った

記録: `target\s2p-5-1\logs\run2.stdout.log`（69,147 行）・`run2.pid.txt`・`run2.exitcode.txt`（`areka exit=0`）。時刻は日本時間。

**版の INFO の行**（要件 5.6）:

```
INFO actor{actor=shiori}: ghost-boot: SHIORI の版を確かめた event="shiori_version_decided" ghost="dot_sakura" version="SHIORI/2.x" evidence="SHIORI/2.6 200 OK"
```

**資源の問い合わせ `GET String` の 1 件**（要件 3.3・4.2）: `username`。kanade の TRACE `shiori_request method=GET id=username` の直後に `areka_kanade::resource: shiori resource prefetch done id="username" outcome="value"`（値あり）。sylphya の「不在の記録」（`shiori resource absent`）は出ていない。値そのものは記録に出さない（run1 では同じ行が `outcome="no_content"` だった）。

**起動の挨拶**（要件 5.1）: 00:32:05 に `OnFirstBoot` の台本の再生を始め（`kanade: 起動グリーティングを再生起動`）、00:33:44 に `steady_talk_done`。台本の頭は `\t\t\h\s[6]初期起動、…`、終わりは `…ヨロシク頼むわ。\e`。挨拶の最中に `talking,balloon(0=0)`、終わった後に `balloon(0=0/1=0)`（`get_status`）。

**MCP の答えの抜き書き**（要件 5.6・`dump_surface` は要件 5.4 も）:

| 道具 | 答え |
|---|---|
| `get_active_ghost_list` | `．さくら` |
| `get_status`（挨拶の最中・後） | `talking,balloon(0=0)` → `balloon(0=0/1=0)` |
| `dump_surface` scope 0（挨拶の最中・後） | `OK:scope 0, surface 0 as currently shown …`（さくらの絵。後の 1 枚は目を閉じた瞬き） |
| `dump_surface` scope 1 | `OK:scope 1, surface 10 as currently shown …`（うにゅうの絵） |
| `get_log`（error） | `(no log entries)` |
| `raise_event OnClose` | `OK:the ghost returned a script and it is being played.` `Script: \t\t\u\s[10]お。\w8終わりやね。\w8\w8\h\s[7]よーし寝るぞ%keroname！\w8\u\n\n[half]\s[11]落ち着けっ！\w9\w9\-` |

面は出た（要件 5.4。要件 5.5 の枝には入らない）。

**時間のトーク**（要件 5.2）: 挨拶の終わり（00:33:44）から待ち、00:34:57 に `OnSecondChange` が台本を返して時間のトークが出た（待った時間 約 73 秒・`\u\s[10]ここだけの話、…`）。その後 00:35:54 まで待って、ほかの時間のトークは出ていない。

**`OnClose`**（要件 5.3・研究記録 10.5 節の測れなかった点）:
- areka が送った要求（kanade の TRACE）: `method=GET id=OnClose references=[] status=None sender="MCP" sender_type=external,sstp security_level="local"`。2.x の組み立て（`shiori2::build_request`）では `GET Sentence SHIORI/2.2` / `Charset: Shift_JIS` / `SecurityLevel: local` / `Sender: MCP` / `Event: OnClose`（References の行なし）の形になる。バイト列そのものは記録に出ないので、形は論理の要求の記録と組み立ての規則から言っている。
- 答え: 台本（上の表）。`\-` で終わる。
- 終了: 00:36:02 に再生を始め、00:36:06 に `talk_done_quit`（`reason=Quit`）→ `unload_clean`（unload が正常終了を返した）→ `ghost_quit cause=Quit` → `app_exit`。areka の終了コードは 0（00:36:07）。外から止めていない。
- 台本の `%keroname` は areka が展開して `うにゅう` にした（`OnTranslate` に渡した台本で確かめた）。

**台本を返したイベントの数**: 3（`OnFirstBoot`・`OnSecondChange` 1 回・`OnClose`。`areka::log::script` の行の `label=Ghost:<イベント>` で数えた）。

**要求の数**（kanade の TRACE `shiori_request`）: `GET OnSecondChange` 129・`NOTIFY OnSecondChange` 113・`GET OnTranslate` 3・`GET OnBalloonTimeout` 2・`NOTIFY OnInitialize` 1・`NOTIFY basewareversion` 1・`GET username` 1・`GET homeurl` 1・`GET OnFirstBoot` 1・`GET OnClose` 1。2.x の相手へ送らなかったのは `basewareversion` の 1 件（`shiori2` の DEBUG `shiori2_event_not_sent`）。

**エラー応答の数と内訳**（要件 5.6・Error Handling > Monitoring）:
- `shiori_error_response` の行: **0**。
- 受け口の無いイベントの 400（別に数える）: **3**。どれも `OnTranslate`（2.x では `TRANSLATE Sentence SHIORI/2.6`）への `SHIORI error status 400` で、`kanade: OnTranslate がエラー応答——展開済みの元の台詞で進む event="translate_reply" kind="error_response"` の WARN として出た（翻訳の道は `shiori_error_response` を出さない）。華和梨に `OnTranslate` の受け口が無いため（研究記録 4 節・SSP の 10.2 節でも翻訳の 2 本は 400）。台本はそのまま再生された。

**load のパスの形（4.4）**: 整えたパスそのものは記録に出ない（4.4 は記録の行を足していない・helper の標準エラーは `[helper] SHIORI 初期化の入口: load` だけ）。整えた形が効いたことは、同じ根と同じ検体で run1 が辞書を読めず run2 が台本と資源の値を返したことで見ている。

## 5.2 3.0 の検体で版の確かめを見る（要件 6.1・6.3・6.4）

2026-10-24・机を持って 1 回ずつ（00:43〜00:47・4.4 の後の `HEAD`）。置き場はすべて `target\s2p-5-2\` の下で、根（`root\ghost\emo2`・`root\ghost\R_POST_and_KOMAINU`・`root\ghost\konnoyayame`・`root\balloon\emo2-kakukaku`・`root\balloon\StayseeBalloon`）は `nar-sample-path` で開いた検体の写し。記憶は検体ごとに空のフォルダ（`profile\<名>`）で、起動はどれも `OnFirstBoot`。起動は 5.1 と同じ形（`--ghost` に絶対パス・`AREKA_APP_SMOKE_EXIT_MS=300000`・`RUST_LOG=info,ghost-boot=info,shiori2=debug`）。終わりは MCP の `raise_event OnClose` で、どれも外から止めずに終了コード 0 で終わった。記録: `target\s2p-5-2\logs\<名>.*`。

| 検体（SHIORI） | 版の INFO の行（抜き書き） | 起動 | 面（`dump_surface` scope 0） | `OnClose` と終了 |
|---|---|---|---|---|
| emo2（`pasta.dll`） | `ghost="emo2" version="SHIORI/3.0" evidence="SHIORI/3.0 204 No Content"` | 挨拶を再生（00:43:50〜00:44:08・`はじめましてや！！…`） | `OK:scope 0, surface 1000 …` | 台本（`…ほな、\_w[450]またな！…\-\e`）→ 終了コード 0 |
| 里々（`satori.dll`・Ｒポストと狛犬） | `ghost="R_POST_and_KOMAINU" version="SHIORI/3.0" evidence="SHIORI/3.0 200 OK"` | 挨拶を再生（00:45:35〜00:45:41・`はじめまして。…里々のサンプルゴースト「ポスト」と申します。…`） | `OK:scope 0, surface 0 …` | 台本（`…\1\_w[36]またな。\-`）→ 終了コード 0 |
| YAYA（`yaya.dll`・紺野ややめ） | `ghost="konnoyayame" version="SHIORI/3.0" evidence="SHIORI/3.0 204 No Content"` | 挨拶を再生（00:46:18〜00:46:38・`初めまして。…紺野ややめだよ。…`） | `OK:scope 0, surface 6 …` | 台本（`…ぷー。\w9\w9\-\e`）→ 終了コード 0 |

- 3 本とも版は 3.0 に決まった。2.x に決まった 3.0 の検体は無い（要件 6.1・6.3）。どの記録にも `shiori2` の行・`shiori_version_undecided`・`LoadReturnedFalse`／`LOAD 失敗` は 0。
- 4.4 で整えた `load` のパスの形（絶対パス・`\` の区切り・末尾に `\` が 1 つ）で、3 本とも今までどおり読み込み・挨拶・面・終了まで通った（要件 6.4）。各記録に `unload_clean`（unload が正常終了を返した）が 1 件。
- MCP の `get_status` は 3 本とも挨拶の最中に `talking,balloon(0=0/1=0)`。emo2 は挨拶の最中に送った最初の `raise_event OnClose` が `NG:the ghost cannot accept a script now` で断られ、挨拶の後（`balloon(0=0/1=0)`）に送り直して通った（断りは台詞の最中の正しい振る舞い）。
- `evidence` の状態の番号は検体で違う（ぱすたと YAYA は 204、里々は 200）が、どれも先頭の行の版の字 `SHIORI/3.0` で 3.0 に決まっている（研究記録 10.1 節の測 2 のとおり）。

### 4.1 と 4.3 で変えたテストの名前

- 4.1（`ShioriConnection` の書き下しに `version: ShioriVersion::Shiori3` の 1 行を足しただけ）: `crates/areka-kanade/tests/kanade/common/common_window_actor.rs`・`crates/areka-kanade/tests/kanade/idle_pump_test.rs`・`crates/areka-kanade/tests/kanade/real_helper_test.rs`
- 4.3: なし
