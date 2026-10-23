# ギャップ分析: shiori2-protocol

> 2026-10-23・ブランチ `claude/kiro-start-shiori2-protocol-22a93e`（HEAD `54077cca`）で調べた。要件は `requirements.md` のとおり（この文書では変えない）。ここに書くのは情報と案で、決めごとではない。
> 調べ方は「読むだけ」に限った。SSP にも areka にも MCP で話しかけていない。ゴーストの起動・読み直し・切り替えもしていない。検体の書庫は調べるために作業用の一時フォルダへ展開しただけで、リポジトリには何も足していない。

## 1. まとめ

- **要求の文を組むのは今も 1 か所だけ**（`crates/shiori-host32-host/src/shiori3.rs` の `build_request`）。32 ビットの経路（`client.rs`）と同じプロセスの経路（`crates/areka-ghost/src/shiori_inproc.rs`）は、どちらも共通の手 `request_of`（`request_headers.rs`）を通る。版ごとの組み分けをこの下に足せば、2 つの経路が同じ要求を送ることは形の上で保てる。
- **接続ごとに覚える先例がある**。文字コードの取り決め `CharsetNegotiator` は「接続（load から unload まで）につき 1 つ」で、kanade の `ShioriConnection`（`crates/areka-kanade/src/shiori/real.rs`）が持つ。決めた版も同じ所に置けば、読み直し（新しい接続が作られる）でやり直しになる。
- **SSP の版の確かめ方は、手元の SSP の記録でほぼ分かった**。SSP は SHIORI を読み込んだ直後に `GET Version SHIORI/2.6` を送り、3.0 のゴーストは状態 204 でも先頭の行を `SHIORI/3.0` で返す。SSP はその後 3.0 で話している（4.1 節）。つまり「答えの先頭の行の版の字」で決めている見込みが高い。
- **2.x の要求と応答の形の一部は、SSP の不具合報告の頁で読めた**。華和梨 7.3.1 のゴーストへ SSP 2.6.07 が送った `GET Sentence SHIORI/2.2`（`Event:` 見出し）と `TRANSLATE Sentence SHIORI/2.6`（`Sentence:` 見出し）、答えの `SHIORI/2.6 200 OK`＋`Sentence:` が載っている（4.2 節）。ただし版の古い SSP・違う版の華和梨の記録なので、今の SSP と「．さくら」で測り直すのが要件 1 の決め。通知だけのイベントと資源の問い合わせの 2.x の形は、手元に手がかりが無い。
- **ふさがりそうな所**: ⑴ 同じプロセスの経路は SHIORI4（`shiori_factory` を持つ x64 の DLL）しか載せないので、2.x の SHIORI がそこへ来ることは今の作りでは無い。要件 2.6・3.8・7.5 を「同じ組み立ての手を通る」で満たすか、確かめの要求も同じに送るかは決めが要る。⑵ 「400 が返ったら 2.x へ移る」案は当てにならない（華和梨は受け口の無いイベントにも 400 を返す・4.2 節）。⑶ 3.0 のゴーストへも `GET Version` が 1 本増えるので、要件 6.1 の「違いは確かめの要求だけ」をはっきり書く必要がある。

## 2. 今のコード（要件ごとの対応）

印: **在る**＝今のまま使える／**足す**＝この spec で書く／**不明**＝SSP で測って決まる／**制約**＝守ること

| 要件 | 使う物・今の様子 | 状態 |
|---|---|---|
| 1.1〜1.4 SSP で測る | SSP は `C:\wintools\ssp\ssp.exe`。記録は `C:\wintools\ssp\ghost\pasta-in-windows-dresstest\ghost\master\ssp_shiori_log.txt`（772,914 バイト・2026-10-21）だけが在る。「．さくら」は SSP に入っていない（`ghost\` の 26 体に無い） | 不明（測る前に、入れること・記録を出す設定のどちらも開発者の許しが要る・6 節） |
| 2.1 版を確かめる | areka には版を確かめる所が 0（`GET Version` を送る所も、応答の `SHIORI/x.y` の字を読む所も無い）。`parse_status_code`（`shiori3.rs` 337 行）は最初の数の字句だけを取る | 足す |
| 2.1 確かめを入れる場所 | 32 ビットの経路の接続は `crates/areka-ghost/src/shiori_wiring.rs` の `real_connect`（LOAD の返事 `[1]` を受けた後に `ShioriConnection` を作る）。kanade の起動の運行は `OnInitialize`（NOTIFY）→`username`（GET）→`OnFirstBoot`／`OnBoot`（GET）→`basewareversion`（NOTIFY）の順で、接続ができた後に始まる | 足す（起動の運行 `schedule/` を変えずに入れられる見込み・3 節） |
| 2.2〜2.3 読み込みのあいだ覚える・読み直しでやり直す | `CharsetNegotiator` と同じく `ShioriConnection` に置ける。読み直しは新しい接続を作る（`emo2_boot/reload.rs` → 切り替えの運行）ので、前の接続の値は残らない見込み | 足す（読み直しで接続が作り直されることはテストで固定する） |
| 2.4〜2.5 INFO・WARN の 1 行 | 先例は `charset_initial`（INFO・`shiori_wiring.rs`） | 足す |
| 2.6 2 つの経路で同じ | 本番は常に 32 ビットの経路（`boot_config.rs` 577 行・`emo2_boot/mod.rs` 759 行が `ShioriWiring::Helper`）。同じプロセスの経路 `ShioriWiring::InProc` は SHIORI4 の DLL 専用（`shiori_factory` が無ければ読み込めない） | 制約（決めが要る・8 節の 2） |
| 3.1〜3.4 2.x の要求を組む | 2.x の要求の行は 0。`build_request` は `GET SHIORI/3.0`・`NOTIFY SHIORI/3.0` の 2 つだけ。`shiori3.rs` は 852 行・`client.rs` は 858 行で、1 ファイル 1,000 行の目安に近い | 足す（新しいファイルへ・例 `shiori2.rs`＋兄弟のテスト） |
| 3.1〜3.4 イベントの種類を見分ける | 種類の情報は kanade 側にある。資源の名前の表 `schedule/resources.rs`（14 名）、翻訳は `EventId::Translate`（`msg.rs`）。下の層（`shiori-host32-host`）は今 `ID` の文字列しか受け取らない | 決めが要る（8 節の 4） |
| 3.5 送らないイベント | 送出の関門は `actor.rs` の `round_trip_raw`（許可表の検査）。版を知らない | 不明（測 5）・足すなら kanade の `shiori/real.rs` か下の層 |
| 3.6 `Charset`・`Sender`・`SecurityLevel` | `RequestHeaders`（4 つの値）と `Charset` は今のまま渡せる | 在る（どれを書くかは測 6） |
| 3.7・8.1 表を §8 に | `doc/COMPAT_ARCHITECTURE.md` §8（122 行〜）の表の形に行を足す | 足す |
| 4.1〜4.3 `Sentence:` などを `Value` と同じに | `parse_response` は `Value`・`ErrorLevel`・`ErrorDescription`・`SecurityLevel` だけを読み、ほかは読み飛ばす | 足す（読む見出しを版で分ける） |
| 4.4〜4.6 204・400・500・読めない | `map_get_result`（`client.rs`）と `round_trip_request`（`actor.rs` の `shiori_error_response`）がそのまま効く。状態の番号の読みは版に関わらない | 在る |
| 4.7 そのほかの見出し | 今読んでいるのは上の 4 つ | 不明（測 4） |
| 5.x 実機の 1 回 | 検体は `vendors/sample_ghost/dot_sakura.nar`（登記済み）。手順の先例は完了 spec `areka-P0-dot-sakura-specimen` の `verification.md` | 在る |
| 5.5 面が出ないとき | 面を出さない決めは `emo2_boot/frame/attach.rs` 304 行の注記 | 在る（確かめるだけ） |
| 6.1〜6.3 3.0 は変えない | 3.0 の要求の全文を固定したテストは `shiori3.rs`・`request_headers_tests.rs`・`shiori3_charset_tests.rs`・`shiori-host32-helper` の `main_loopback_tests.rs`・`areka-kanade/tests/kanade/translate_test.rs` に在る。偽の SHIORI で数や並びを数えるテストは kanade の層（`ShioriBackend` の偽物）で回るので、接続の中で送る確かめの要求は数に入らない見込み | 在る（確かめの要求を足す場所しだいで変わる・8 節の 1） |
| 7.x 決定論のテスト | 偽の SHIORI は `ShioriBackend` の偽物（kanade）と、要求のバイトを組んで読むだけの純粋な関数で足りる。i686 のテスト用 DLL（`shiori-host32-testdll`）は 3.0 の要求の行を照らして答える作り（189 行）で、常時のテストには使っていない | 足す |
| 8.2 網羅台帳 | `doc/ukadoc-coverage/ledger/shiori.toml` の `spec_shiori3:_30b9_30c6_30fc_30bf_30b9_30b3_30fc_30c9:1`（ステータスコード・16275 行）と `spec_shiori3:Age_20_5bSSP_62e1_5f35_5d:1`（15727 行） | 足す（行の状態と持ち主） |
| 8.3 README | `README.md` の検体の表の「．さくら」の行 | 足す |

## 3. 触るファイルと、ウェーブの約束

ウェーブ C11 の約束（`.kiro/steering/roadmap.md` 189 行・⑧ の行）: 触るのは `shiori-host32-host/src/{shiori3,client}.rs`（2.x の処理は新しい `shiori2.rs` と兄弟のテストへ）・`lib.rs`（公開の 1 行）・`areka-ghost/src/shiori_inproc.rs`・kanade の `actor.rs`（要るときだけ）・`shiori.toml` の行・§8。**触らない**のは kanade の `msg.rs`・`schedule/`（② `mouse-click-wheel-events` の持ち物）と `crates/shiori-host32-helper/`。

調べて分かった、この表に載っていないが要りそうなファイル:

| ファイル | 何のために | 約束との関係 |
|---|---|---|
| `crates/areka-kanade/src/shiori/real.rs` | `ShioriConnection` に決めた版を持たせる・`handle_call` でイベントの種類を下へ渡す | 表に無いが、禁止の `schedule/`・`msg.rs` ではない。C11 のほかの 7 本の持ち物にも無い |
| `crates/areka-ghost/src/shiori_wiring.rs` | 読み込みの直後（LOAD の返事の後）に `GET Version` を送り、答えから版を決める場合 | 表に無い。C11 ④ `mcp-scope-rect-property` が持つのは同じ crate の `sylphya_wiring.rs`・`runtime.rs` で、`shiori_wiring.rs` は持たない |
| `crates/shiori-host32-host/src/request_headers.rs` | 共通の手 `request_of` に版を渡す場合 | 表に無い（`shiori3.rs` と同じ crate） |

**約束にかかる案**: 版の確かめを kanade の起動の運行（`schedule/boot.rs`）の 1 段として足す案は、禁止の `schedule/` に触る。下の案 A〜C はどれも接続の中（起動の運行より前）で確かめるので、`schedule/` には触らない。`crates/areka-ghost/src/runtime.rs` に手を入れる案（例: 結線の種類を増やす）は ④ と重なるので取らない方がよい。

## 4. 読むだけで分かったこと（出どころつき）

### 4.1 今の SSP（2.9.22）が 3.0 のゴーストへ送る、読み込みの直後の要求

出どころ: `C:\wintools\ssp\ghost\pasta-in-windows-dresstest\ghost\master\ssp_shiori_log.txt` の先頭（2026/10/21 23:37:43.971〜44.256）。並び:

1. `(load library)` → `(function : loadu found)` ・`request found`・`unload found` → `(getversion/loadex call)` → `(loadu call)` の答え `1`
2. 版の確かめ（抜き書き・行の区切りは CRLF）:

```
-----send------------------------
GET Version SHIORI/2.6
Charset: UTF-8
Sender: SSP

-----response--------------------
SHIORI/3.0 204 No Content
Charset: UTF-8
Sender: Pasta
SecurityLevel: local
```

3. 続けて `GET SHIORI/3.0`＋`ID: version`（見出しの並びは `Charset`・`ID`・`SecurityLevel`・`Sender`）
4. 続けて `NOTIFY SHIORI/3.0` の `OnLanguageChange` → `OnInitialize` → `ownerghostname` → `basewareversion` → `capability` → `OnNotifyOSInfo` → …

読めること:

- `GET Version` は `SHIORI/2.6` の書き方で、見出しは `Charset` と `Sender` の 2 つだけ（`ID`・`SecurityLevel` は無い）。
- 3.0 の SHIORI は状態が 204 でも先頭の行を `SHIORI/3.0` で返し、SSP はその後 3.0 で話している。**SSP は答えの状態の番号ではなく、先頭の行の版の字で決めている見込みが高い**（2.x へ移る枝の記録はこのファイルに無いので、2.x の側は未確認）。
- 完了 spec `areka-P0-mcp-reload` の `research.md` 197 行の「load → `GET Version` → `version` → NOTIFY `OnLanguageChange`・`OnInitialize`」と一致する。

### 4.2 古い SSP（2.6.07）が華和梨 7.3.1 のゴーストへ送った 2.x の要求

出どころ: SSP の不具合報告 BTS 0000503「古い栞で400 Bad Requestが出る」（<https://bts.shillest.net/view.php?id=503>・2022-06）。完了 spec `areka-P0-ssp-bts-salvage` の `bts-ledger.md` 498 行が拾っている。頁の記録の抜き書き（頁を取り込んで読んだ形。行の区切りと空行は頁の表示に拠る）:

```
GET Sentence SHIORI/2.2
Charset: Shift_JIS
Sender: SSP
SecurityLevel: local
Event: OnChoiceSelect
Status: choosing,balloon(0=0)
Reference0: Manzai
```
→ `SHIORI/2.6 200 OK`・`Charset: Shift_JIS`・`Sentence: \0\s[0]…\e`

```
TRANSLATE Sentence SHIORI/2.6
Charset: Shift_JIS
Sender: SSP
SecurityLevel: local
Sentence: \0\s[0]…\e
```
→ 辞書に `system.OnTranslate` が無いとき `SHIORI/2.6 400 Bad Request`、足した後は `SHIORI/2.6 200 OK`＋`Sentence: …`

読めること:

- 台本を求めるイベントは `GET Sentence SHIORI/2.2`。イベント名は `ID:` でなく `Event:` に載る。`Reference0` は 3.0 と同じ番号つき。`Status` も載る。
- 翻訳は `TRANSLATE Sentence SHIORI/2.6` で、台本は `Sentence:` 見出しに載る。この記録には `Reference*` が無い（areka の 3.0 の `OnTranslate` は Reference0〜3 を送る）。
- 2.x の要求には `SecurityOrigin`・`SenderType` が無く、`Charset`・`Sender`・`SecurityLevel` は在る（SSP 2.6.07 の時点）。
- 華和梨 7.x は答えの先頭の行を `SHIORI/2.6` で返す。SSP は華和梨 7.3.1 に 2.x で話していた。
- **華和梨は、辞書に受け口の無いイベントにも 400 を返す**。だから「400 が返ったら 2.x の相手」という見分け方は、3.0 の要求に対しても当てにならない（3.0 の SHIORI でも受け口の無いイベントに 400 を返すものがあれば誤って 2.x と決めてしまう）。

注意: SSP の版（2.6.07 と 2.9.22）も華和梨の版（7.3.1 と 7.4）も違う。要件 1.1 のとおり、今の SSP と「．さくら」で測り直して正本にする。

### 4.3 里々の文書（SHIORI 側の受け方）

出どころ: <https://ukatech.shillest.net/satori-docs/shiori/protocol/>（旧 URL <https://ukatech.github.io/satori-docs/shiori/protocol/> から移っている）。要件の本文が引いたとおり、2.x の `Event` を `ID` と読み替える・応答は 3.0 の形（`Sentence` に読み替え）・`GET Version` に 200 OK を返し、2.x では `ID`・`Craftman`・`Version` も返す、とある。SHIORI の側の作りなので正典ではないが、`GET Version` の答えの見出しの手がかりになる。

### 4.4 検体「．さくら」の中身（書庫を展開して読んだ）

- `ghost/master/shiori.dll` は 140,800 バイト・i386・UPX で圧縮されていて中の文字列は読めない（`KAWARI.kdt/7.4.` の切れ端だけ見える）。版の字を DLL から読むことはできない。
- 辞書には「SHIORI2.2 によるイベント」の注記が何か所も在る（`dict-keeps.txt`「#以下SHIORI2.2イベント」・`dict-keeps-winlocal.txt`「SHIORI/2.2 Notify対応アプリ汎用イベント」）。
- 受け口: `event.OnBoot`・`event.OnFirstBoot`・`event.OnClose`・`event.OnSecondChange`・`event.OnMinuteChange`・`event.OnMouseDoubleClick`・`event.OnChoiceSelect` など。**`OnTranslate`（`system.OnTranslate`）と `OnInitialize` の受け口は無い**。だから SSP でも翻訳の要求には 400 が返るはず（4.2 節と同じ）で、実機の 1 回の 400 の数え方に効く（要件 5.6）。
- 資源: `resource.username`・`resource.homeurl` のほか、`resource.updatebuttoncaption`・`resource.vanishbuttoncaption`・`resource.sakura.recommendbuttoncaption` など古い名前。areka が問い合わせる名前は `updatebutton.caption` など（`schedule/resources.rs`）。ukadoc は `updatebuttoncaption` を別の項目として載せている（<https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebuttoncaption>「ネットワーク更新の名称。」）。**2.x の書き方が直っても、「．さくら」のメニューの更新の項目名は areka が古い名前を問わない限り出ない**。これは版の書き方の話ではなく資源の名前の話なので、実機の 1 回で見えたら要件 8.4 で持ち主へ渡す候補。
- `descript.txt` は `charset,Shift_JIS`。要求も Shift_JIS になる（今の文字コードの決め方のまま）。

### 4.5 ukadoc で読めたこと

要件の本文のとおり、2.x に触れるのはステータスコード（400・311）と `Age` だけ。資源の名前の表には古い名前（`updatebuttoncaption`・`vanishbuttoncaption`）が在る（4.4 節）。

## 5. 測 1〜測 7 ごとの見通し

| 番号 | 読むだけで分かったこと | まだ SSP で測ること（最小の手順） |
|---|---|---|
| 測 1 | 華和梨 7.3.1 に SSP は 2.x で話していた（4.2）。「．さくら」の辞書は 2.2 の注記つき（4.4）。areka の 3.0 の要求には全部 400（完了 spec の `verification.md` 5 節）。華和梨 7.4 が 3.0 を受けないことは強く推せる | 「．さくら」を SSP で起こし、記録の `GET Version SHIORI/2.6` の答えの先頭の行が `SHIORI/2.x` か、続く要求が 2.x の書き方かを見る（下の「1 回の測り」で同時に分かる） |
| 測 2 | SSP は読み込みの直後に `GET Version SHIORI/2.6`（`Charset`・`Sender` だけ）を送る。3.0 の SHIORI は 204 でも `SHIORI/3.0` で答え、SSP は 3.0 で続ける（4.1）。版は答えの先頭の行の字で決めている見込み | 2.x の SHIORI の答え（状態の番号・`ID`・`Craftman`・`Version` の見出し）と、その後 SSP が 2.x の書き方へ移ることを記録で見る。「3.0 で 400 なら 2.x へ移る」枝が無いことも、同じ記録の並びで確かめられる |
| 測 3 | 台本を求めるイベント＝`GET Sentence SHIORI/2.2`＋`Event:`＋`Reference*`＋`Status`。翻訳＝`TRANSLATE Sentence SHIORI/2.6`＋`Sentence:`（Reference 無し）（4.2・古い SSP） | 通知だけのイベント（`OnInitialize`・`basewareversion`・`ownerghostname`・台詞の最中の `OnSecondChange`）と資源の問い合わせ（`username`・`homeurl`・メニューの項目名）の 2.x の形は手がかりが無い。今の SSP で台本のイベントと翻訳の形が変わっていないかも見る |
| 測 4 | 台本・翻訳の結果は `Sentence:`、先頭の行は `SHIORI/2.6 200 OK`（4.2）。辞書に受け口が無ければ 400 | 資源の値がどの見出しで返るか（`String:` などか）。204 を返すか、見出しの無い 200 を返すか。`Value` に当たるもの以外の見出し（`Marker`・`ErrorLevel` など）の有無 |
| 測 5 | 3.0 のゴーストには `OnLanguageChange`・`capability`・`OnNotifyOSInfo` などを送っている（4.1）。2.x の側は記録が無い | 1 回の測りの記録に出てくるイベント名の一覧と、areka が送るもの（`ALLOWED_EVENT_IDS`・資源の 14 名）を突き合わせる |
| 測 6 | SSP 2.6.07 は 2.x の要求に `Charset`・`Sender`・`SecurityLevel` を付け、`SecurityOrigin`・`SenderType` は付けていない。`GET Version` には `Charset`・`Sender` だけ（2.9.22） | 今の SSP 2.9.22 が 2.x の要求に付ける見出しの組み（`SecurityOrigin`・`SenderType` が増えていないか） |
| 測 7 | 「．さくら」の起動の受け口は在る（`event.OnBoot`）ので、2.x で話せば起動の挨拶に `\s` が来る見込み。測 7 は要件 5.5 の枝に入ったときだけ | （枝に入ったときだけ）SSP で台本を返さない起動のとき既定の面が出るかを見る。手順は枝に入ってから決める |

**1 回の測りの最小の手順**（測 1〜測 6 をまとめて取る。どの段も開発者の許しを 1 回ずつ取ってから。窓の位置は前に控えて後で戻す）:

1. 「．さくら」を SSP に入れる（`vendors/sample_ghost/dot_sakura.nar` を SSP へ投げ込む）。**SSP の持ち物を増やす操作**なので許しが要る。
2. 「．さくら」の SHIORI の要求の記録を出す設定にする。手元では `ssp_shiori_log.txt` が `pasta-in-windows-dresstest` にしか無く、同じ SSP の `pasta-in-windows` には無いので、記録はゴーストごとの設定で出ていると見られる。**どの設定で出すかはまだ確かめていない**（設計の段の最初に確かめる・設定の変更なので許しが要る）。
3. 「．さくら」へ切り替えて起こす（ここで `GET Version`・起動の通知・資源の問い合わせ・起動の挨拶・翻訳が記録に出る）。記録の大きさ（バイト位置）を切り替えの前に控える。
4. 1〜2 分待つ（`OnSecondChange`・`OnMinuteChange` と、台詞の最中の `OnSecondChange` が出る）。
5. 元のゴーストへ切り替えて戻す（`OnClose` ではなく切り替えの要求が出る。`OnClose` の形まで要るなら、`OnClose` を送る操作をもう 1 回だけ許しを取って行う）。
6. 控えた位置から後ろの記録を読み、要求と応答のバイトのまま `research.md` に書く。

## 6. 実装の案

### 案 A: 今の部品を広げる（`shiori3.rs` に 2.x の枝を足す）

- `Method` に 2.x の要求の種類を足し、`build_request` と `parse_response` に版の枝を足す。版は `CharsetNegotiator` の隣に持たせる。
- 良い点: 新しいファイルが要らない。
- 悪い点: `shiori3.rs` は 852 行で、2.x の組み立て・読み・テストを足すと 1,000 行を超える。ウェーブの約束（2.x は新しい `shiori2.rs` へ）にも合わない。3.0 の要求のバイトを固定したテストの近くを触るので、要件 6 の「3.0 は 1 バイトも変えない」を守る手間が増える。

### 案 B: 2.x の組み立てと読みを新しいファイルに分ける（約束どおりの形）

- `shiori-host32-host/src/shiori2.rs`（＋兄弟のテスト）に、⑴ 3.0 の要求の材料（`ShioriRequest`）から 2.x の要求を組む関数、⑵ 2.x の応答の `Sentence:` などを `Value` と同じ欄へ戻す読み、⑶ 応答の先頭の行から `SHIORI/x.y` を読む関数、⑷ 組み替えの表（イベントの種類 → 要求の行と見出し）を置く。
- 版は接続の持ち物（`ShioriConnection`）に置き、`Shiori3Client` が版で `shiori3` と `shiori2` を選ぶ。同じプロセスの経路の `build_input` も同じ選び方を通す。
- 確かめの要求 `GET Version` を送る所は 2 通り:
  - B-1: `real_connect`（`shiori_wiring.rs`）が LOAD の返事の後に 1 回送り、決めた版を `ShioriConnection` へ渡す。SSP と同じ「読み込みの直後」。kanade の偽の SHIORI を使うテストの要求の数は変わらない。32 ビットの経路でしか送らないので、同じプロセスの経路の扱いは別に決める。
  - B-2: `ShioriConnection`（kanade の `shiori/real.rs`）が最初の要求の前に 1 回送る。接続の作り方に触らない。起動の運行はいつも `OnInitialize` から始まるので、送る順は SSP と同じ「`GET Version` が最初」になる。
- 良い点: 3.0 の部品はほぼ触らずに済む。2.x の読み書きを単独でテストできる。接続ごとに覚える形は文字コードの先例と同じ。
- 悪い点: 版で選ぶ所が `Shiori3Client` と同じプロセスの経路の 2 か所に出る（共通の手 `request_of` の近くへ寄せれば 1 か所にできる）。

### 案 C: 組み合わせ（版で選ぶ所を 1 か所に寄せる）

- 案 B の `shiori2.rs` に加え、`request_headers.rs` の共通の手（`request_of` の隣）に「版を受けてどちらで符号化するか」を 1 本だけ置き、2 つの経路はどちらもそれを呼ぶ。`shiori3.rs` からは先頭の行の版の字を読む 1 関数だけを外へ出す。
- 良い点: 要件 3.8・7.5（2 つの経路で同じ要求）を、今の `request_of`（要件 2.8 を形で満たした先例）と同じやり方で満たせる。
- 悪い点: 共通の手の役目が「並べる」から「版で組み分ける」へ広がる。

### 選ばない方がよい案

- **400 を見て 2.x へ移る（読み直しの代わりの切り替え）**: 4.2 節のとおり、受け口の無いイベントにも 400 が返るので誤って移る。SSP の手元の記録も `GET Version` で決めている。移る前の 1 本（多くは `OnInitialize` の通知）を送り直す仕組みも要り、往復の関門 `round_trip_request`（`actor.rs`）の決め「400 は返事なしとして続ける」と絡む。
- **起動の運行（`schedule/boot.rs`）に確かめの段を足す**: ウェーブの約束で触れない。運行の状態の数も増える。
- **32 ビットの中継（helper）で版を確かめる**: 範囲の外（helper はバイトの素通し）。`shiori4-api` の決め「版の見分けは 64 ビットの側」にも合わない。

## 7. 規模と危うさ

- **規模: M（3〜7 日）**。組み立てと読みは純粋な関数で、先例（文字コードの取り決め・共通の手）に乗る。ただし SSP で測る段が開発者の許し待ちになるのと、通知と資源の 2.x の形が測るまで決まらないのとで、日数が延びうる。
- **危うさ: 中**。組み立ての技術は易しいが、⑴ 通知だけのイベントと資源の問い合わせの 2.x の形が手元に無い ⑵ 3.0 の全部のゴーストへ `GET Version` が 1 本増える（SSP も送っているので、SSP で動くゴーストは受け流す見込みだが、areka の検体で確かめる要がある）⑶ 同じプロセスの経路の扱いが決まっていない、の 3 つが残る。

## 8. 要件の議論へ渡す決めの候補

1. **確かめの要求を送る場所**: 接続を作るとき（`shiori_wiring.rs` の `real_connect`・LOAD の返事の後）か、接続の最初の要求の前（kanade の `shiori/real.rs`）か。前者は SSP の順と同じで kanade の偽物を使うテストに影響しないが、ウェーブの約束の表に無いファイルを触る。後者は表に無い `shiori/real.rs` を触る。
2. **同じプロセスの経路（SHIORI4）の扱い**: そこは SHIORI4 の DLL しか載らず 2.x の相手は来ない。⑴ 同じ確かめ（`GET Version`）をそちらでも送る（SHIORI4 のテスト用 DLL と、その要求の並びを固定したテストが変わる）か、⑵ 「SHIORI4 は 3.0 として扱う」と決めて確かめを送らず、組み立ての手だけを共通にして要件 2.6・3.8・7.5 を満たす（`shiori4-api` で C の口へ移る予定とも当たらない）か。
   - **→ 確定（2026-10-23 要件の話し合い・議題 2）**: 開発者「SHIORI はすべて別プロセスで動かす・今の SHIORI4 の口は破棄する」（`shiori4-api` の決定 3 と同じ）。同じプロセスの道には確かめも 2.x の書き方も足さず、`shiori_inproc.rs` に触らない。要件 2.6 を書き換え、3.8・7.5 を消した。
3. **版を決める根拠**: 答えの先頭の行の版の字（`SHIORI/2.x`・`SHIORI/3.0`）で決めるか、状態の番号も見るか。4.1 節の記録では 3.0 の SHIORI が 204 で答えても SSP は 3.0 で続けている。決められないとき（答えが無い・読めない）は 3.0（要件 2.5）。2.x のうち細かい版（2.0〜2.6）で書き方を変えるかも、測りの後に決める。
4. **イベントの種類（台本・通知・資源・翻訳）を下の層へどう渡すか**: ⑴ kanade の `shiori/real.rs` の `handle_call` が、`EventId::Translate` と資源の表（`schedule/resources.rs` を読むだけ）から種類を決めて渡す（`ShioriBackend` の口に欄が 1 つ増え、偽物とテストに波及する）。⑵ 下の層が `ID` の文字列と GET／NOTIFY の別だけで決める（`OnTranslate`＝翻訳、GET で `On` で始まらない名前＝資源、など）。⑵ は口を変えないが、名前の形に頼る決めになる。
5. **3.0 のゴーストに増える 1 本の扱い**: 要件 6.1 の「違いは確かめの要求だけ」として、`GET Version SHIORI/2.6` の 1 本を表に書く。3.0 のゴーストの答え（204・400 など）は記録を残すだけにし、`shiori_error_response` の WARN に数えるか（3.0 の検体で毎回 1 行増える）を決める。
6. **SSP で測る段の進め方**: 「．さくら」を SSP に入れること、記録を出す設定、切り替え、`OnClose` を送る操作のそれぞれに許しが要る（要件 1.3）。記録を出す設定がどこにあるかはまだ確かめていない。
7. **見つかった別の崩れの行き先**: 「．さくら」は古い資源の名前（`updatebuttoncaption` など）しか持たず、areka は新しい名前（`updatebutton.caption`）だけを問う。2.x の書き方とは別の話なので、実機の 1 回で見えたら要件 8.4 のとおり持ち主の spec へ渡すか起票する（今は持ち主が見当たらない）。
8. **翻訳の 400**: 「．さくら」には翻訳の受け口が無いので、2.x の書き方に直しても `TRANSLATE` には 400 が返る見込み（SSP でも同じ）。要件 5.6 の「`shiori_error_response` の内訳」で、これを崩れに数えないことを先に決めておくと実機の判定がぶれない。

## 9. 設計の段へ持ち越す調べ

- 今の SSP 2.9.22 が 2.x の相手へ送る通知だけのイベント・資源の問い合わせの要求の行と見出し（測 3・測 6）と、資源の値の応答の見出し（測 4）。
- 2.x の SHIORI の `GET Version` の答えの形（状態の番号・`ID`・`Craftman`・`Version`）。
- SSP で SHIORI の要求の記録を出す設定の場所（ゴーストごとか・全体か）。
- 3.0 の検体（emo2・ぱすたさん・里々・YAYA）が `GET Version SHIORI/2.6` に何を返すか（areka の実機で確かめるか、SSP の記録で読めるか）。

## 10. SSP での測り（2026-10-23・要件 1.1）

> 測った SSP: `SSP/2.9.23 (20261021-5)`（`C:\wintools\ssp`）。検体: `vendors/sample_ghost/dot_sakura.nar`。開発者が SSP に入れた（`ghost\dot_sakura`）。記録の設定はゴーストの `ghost\master\profile\ghost.dat` の `shiori.log,1` の行で、測りの前に足した。要件 1.3 で許された 5 つの操作のうち ⑴〜⑶ と ⑸ を行った。⑷ `OnClose` は送っていない（下の「測れなかったこと」）。窓の位置は前後とも `1710,376,2818,1744`。
> 記録: `C:\wintools\ssp\ghost\dot_sakura\ghost\master\ssp_shiori_log.txt`（166,364 バイト・20:43:24〜20:45:17）。手順は ①「窓際のぱすたさん」から `\![change,ghost,．さくら]` で切り替える ② 約 2 分待つ ③ `\![change,ghost,窓際のぱすたさん]` で戻す。記録は OS・フォント・入っているゴーストの一覧など機械の情報を含むので、リポジトリには写していない。以下は抜き書き（行の区切りは CRLF）と、要求の形ごとのまとめ。台本と利用者の名前の値は省いた。

### 10.1 版の確かめ（測 1・測 2）

```
-----send------------------------
GET Version SHIORI/2.6
Charset: UTF-8
Sender: SSP

-----response--------------------
SHIORI/2.6 200 OK
Charset: Shift_JIS
Craftman: KawariDevelopmentTeam
ID: KAWARI.kdt
Version: 7.4.0
```

- 読み込みの直後、`load` の答え `1` の次に 1 本だけ送る（4.1 節の 3.0 のゴーストと同じ並び）。
- 華和梨 7.4 は先頭の行を `SHIORI/2.6` で答える。続く要求は全部 2.x の書き方になる。3.0 の要求は 1 本も送られていない。**測 1: 華和梨 7.4 は 2.x の相手**（要件 1.2 の枝には入らない）。
- 3.0 のゴーストは `SHIORI/3.0 204 No Content` で答え、SSP は 3.0 で続けていた（4.1 節）。2.x のゴーストは `SHIORI/2.6 200 OK` で答え、SSP は 2.x へ移った。**測 2: 版は「`GET Version` の答えの先頭の行の版の字」で決まる**（状態の番号は 204 と 200 で違うが、どちらでも先頭の行の版の字の側へ移っている）。「3.0 の要求に 400 が返ったら 2.x へ移る」枝は記録に無い（3.0 の要求を 1 本も送っていない）。
- `GET Version` の要求の `Charset` は `UTF-8`。華和梨の答えの `Charset` は `Shift_JIS` で、その後の要求は全部 `Charset: Shift_JIS`（`descript.txt` の `charset,Shift_JIS` のとおり）。

### 10.2 要求の形（測 3・測 5・測 6）

記録の要求を、最初の行とイベント名で数えた（`Ref*`＝`Reference0` 以降）。

| 3.0 での種類 | SSP が 2.x の相手へ送った形 | 見出し（並びのまま） | 例（回数） |
|---|---|---|---|
| 台本を求めるイベント | `GET Sentence SHIORI/2.2` | `Charset`・`SecurityLevel`・`Sender`・`Event`・（`BaseEvent`）・（`Status`）・`Ref*` | `OnFirstBoot`（1）・`OnSecondChange`（113）・`OnMinuteChange`（2）・`OnSurfaceChange`（32）・`OnDestroy`（1） |
| 通知だけのイベント（3.0 では NOTIFY） | **`GET Sentence SHIORI/2.2`**（GET で送る） | 上と同じ | `OnInitialize`・`OnLanguageChange`・`OnNotifyOSInfo` ほか `OnNotify*`・`installedghostname` ほか `installed*`・`hwnd`・`uniqueid`・`ghostpathlist` ほか・`rateofusegraph`（各 1〜3） |
| `ownerghostname` | `NOTIFY OwnerGhostName SHIORI/2.3` | `Charset`・`Sender`・`Ghost`（値はゴーストの名前） | 1 |
| `otherghostname` | `NOTIFY OtherGhostName SHIORI/2.3` | `Charset`・`Sender`（`Ghost` の見出しが無い） | 22 |
| 資源の問い合わせ | `GET String SHIORI/2.5` | `Charset`・`SecurityLevel`・`SecurityOrigin`（値 `null`）・`Sender`・`ID`・（`Status`） | `username`・`version`・`name`・`craftman`・`sakura.recommendsites`・`sakura.defaultx` ほか |
| 台本の翻訳（`OnTranslate`） | `TRANSLATE Sentence SHIORI/2.6` | `Charset`・`SecurityLevel`・`SecurityOrigin`（`null`）・`Sender`・`Status`・`Sentence`（**References は無い**） | 2 |

- `Event:` の値は 3.0 の `ID:` の値と同じイベント名。資源は `ID:` に資源の名前を載せる。
- `OnFirstBoot` には `BaseEvent: OnBoot` が付く（3.0 の `BaseID` に当たる見込み。4.2 節の古い記録には無かった）。
- `Status:` は、台詞の最中や選択肢の最中など状態があるときだけ付く（例 `Status: talking,timecritical,balloon(0=0)`）。
- **測 5: 3.0 のゴーストへ送っていて 2.x の相手へ送らなかったもの**: `basewareversion`・`capability`（4.1 節では 3.0 のゴーストへ送っている）。`ownerghostname`・`otherghostname` は名前を変えた NOTIFY として送る。
- **測 6: 見出しの組み**: `Charset`・`Sender`・`SecurityLevel` は 2.x の要求にも付く。`SecurityOrigin` は `GET String`・`TRANSLATE` にだけ付き（値 `null`）、`GET Sentence` には付かない。`SenderType` はどの要求にも無い。

抜き書き（それぞれの形の 1 本目）:

```
GET Sentence SHIORI/2.2
Charset: Shift_JIS
SecurityLevel: local
Sender: SSP
Event: OnInitialize
Reference0: 

-----response--------------------
SHIORI/2.6 204 No Content
Charset: Shift_JIS
```

```
NOTIFY OwnerGhostName SHIORI/2.3
Charset: Shift_JIS
Sender: SSP
Ghost: さくら

-----response--------------------
SHIORI/2.6 200 OK
Charset: Shift_JIS
```

```
GET String SHIORI/2.5
Charset: Shift_JIS
SecurityLevel: local
SecurityOrigin: null
Sender: SSP
ID: username

-----response--------------------
SHIORI/2.6 200 OK
Charset: Shift_JIS
String: （値は省いた）
```

```
GET Sentence SHIORI/2.2
Charset: Shift_JIS
SecurityLevel: local
Sender: SSP
Event: OnFirstBoot
BaseEvent: OnBoot
Reference0: 0
Reference1: 
（Reference2〜6 は空）
Reference7: master

-----response--------------------
SHIORI/2.6 200 OK
Charset: Shift_JIS
Sentence: \t\t\h\s[6]…（台本は省いた）
```

```
TRANSLATE Sentence SHIORI/2.6
Charset: Shift_JIS
SecurityLevel: local
SecurityOrigin: null
Sender: SSP
Status: talking
Sentence: \t\t\h\s[6]…（直前の OnFirstBoot の台本）

-----response--------------------
SHIORI/2.6 400 Bad Request
Charset: Shift_JIS
```

```
GET Sentence SHIORI/2.2
Charset: Shift_JIS
SecurityLevel: local
Sender: SSP
Event: OnSecondChange
Status: talking,timecritical,balloon(0=0)
Reference0: 193
Reference1: 0
Reference2: 0
Reference3: 0
Reference4: 64

-----response--------------------
SHIORI/2.6 204 No Content
Charset: Shift_JIS
```

### 10.3 応答の形（測 4）

- 台本は `Sentence:`、資源の値は `String:` に入る。どちらも状態は `200 OK`。
- 「返す物が無い」は `SHIORI/2.6 204 No Content`（`Charset` だけ）。見出しの無い 200 は、台本や値を求める要求への答えには出ていない。NOTIFY（`OwnerGhostName`・`OtherGhostName`）への答えは `200 OK`＋`Charset` だけ。
- 受け口の無い要求は `400 Bad Request`（翻訳の 2 本）。
- 答えに出た見出しは `Charset`・`Sentence`・`String`、`GET Version` への `Craftman`・`ID`・`Version` だけ。`Value`・`Marker`・`ErrorLevel`・`ErrorDescription`・`SecurityLevel` に当たる見出しは出ていない（要件 4.7 の「2.x に無い」の行になる）。
- 記録の中で、応答の前に `***CallTime:15msec.` の行が付くものが 2 本ある。SSP が呼び出しにかかった時間を書いた行で、SHIORI の答えの一部ではない。

### 10.4 面（測 7）

- `OnFirstBoot` の台本に `\s[6]` などがあり、`OnSurfaceChange` が 32 回送られている（例 `Reference0: 6`・`Reference1: -1`）。SSP では 2.x の書き方で話せば面が出た。要件 5.5 の枝（台本が来ても面が出ない）は、SSP の側では起きていない。測 7 は測らない。

### 10.5 測れなかったこと

- **`OnClose`**: 送らなかった。MCP の `raise_event` は返ってきた台本を再生する。「．さくら」の終了のトークは `\-` で終わる見込みで、再生すれば SSP ごと閉じる。同じ SSP をこの後 ② `mouse-click-wheel-events` が使う予定だった。記録の 70 種を超えるイベントは、ownerghostname・otherghostname を除いて全部 `GET Sentence SHIORI/2.2`＋`Event:` の同じ形だったので、`OnClose` も同じ形と見ている。**これは測った値ではなく、同じ形の並びからの見込み**。areka の実機の 1 回（要件 5.3）で、areka が送る形と「．さくら」の答えを記録して確かめる。
- **ゴーストを切り替えて出すときのイベント**: `\![change,ghost,…]` で戻したときに「．さくら」へ届いたのは `TRANSLATE`（切り替えの台本そのもの）→ `rateofusegraph` → `OnDestroy` → `unload` だった。`OnGhostChanging` は来ていない。台本を外から（SSTP で）送って切り替えたからの見込みで、利用者がメニューから切り替えたときの並びは測っていない。
- **時間のトーク**: 約 2 分のあいだ、`OnSecondChange` 113 本・`OnMinuteChange` 2 本はどれも 204 だった。この間に時間のトークは出ていない（要件 5.2 の「来なかったときは待った時間を書く」の先例）。

### 10.6 9 節の持ち越しへの答え

- 2.x の相手へ送る通知・資源の問い合わせの形 → 10.2 節。資源の値の見出し → `String:`（10.3 節）。
- 2.x の SHIORI の `GET Version` の答え → 10.1 節（`200 OK`・`Craftman`・`ID`・`Version`）。
- SSP の記録を出す設定 → ゴーストごと。`ghost\master\profile\ghost.dat` の `shiori.log,1`。
- 3.0 の検体が `GET Version SHIORI/2.6` に何を返すか → ぱすたさんは `SHIORI/3.0 204 No Content`（4.1 節）。ほかの 3.0 の検体（emo2・里々・YAYA）は測っていない。設計の段で、areka の決定論のテストか実機で確かめる。

## 11. 設計の段の決め（2026-10-23・`/kiro-spec-design -y`）

> 調べの種類: 軽い調べ（既存の仕組みへの足し）。新しい外部の依存は無い。読んだ物: `shiori-host32-host` の `shiori3.rs`・`client.rs`・`request_headers.rs`・`charset.rs`・`lib.rs`、kanade の `shiori/real.rs`・`schedule/events.rs`（許可の表）・`schedule/resources.rs`（資源の 14 名）・`schedule/boot.rs`、`areka-ghost` の `shiori_wiring.rs` と `shiori_wiring_charset_tests.rs`、`runtime.rs` の `real_connect` の呼び出し、steering の `logging.md`・`roadmap.md`（C11 の ⑧ の行）。参考に里々の文書（<https://ukatech.shillest.net/satori-docs/shiori/protocol/>）を引き直した: 「`GET Version` には 200 OK を返します」「SHIORI/2.x では `ID` `Craftman` `Version` を付けます」・2.x の要求への応答は「SHIORI/3.0 形式」。里々は確かめに 3.0 の先頭の行で答える見込みだが逐語では書かれていない（実機の 1 回で 3.0 の検体の版の行を確かめる）。
> 使ったスキル: `kiro-spec-design`（決めの規則・調べの規則・まとめの規則・見直しの関門）。

### 11.1 8 節の決めの候補への答え

| 候補 | 決め | 理由 |
|---|---|---|
| 1 確かめを送る場所 | `areka-ghost` の `shiori_wiring.rs` の `real_connect`、LOAD の ack の直後（案 B-1） | SSP の並び（load の答えの次に 1 本）と同じ。kanade の要求より前に済むので `OnInitialize` が失われない。kanade の偽物を使うテストの要求の数が変わらない。`schedule/`・`runtime.rs` に触らない。 |
| 3 版を決める根拠 | `GET Version` の答えの先頭の行の最初の字句だけ（`SHIORI/2.`→2.x・`SHIORI/3.`→3.0・ほか→決められない＝3.0＋WARN）。状態の番号は見ない | SSP は 3.0 のゴーストの 204 でも、2.x のゴーストの 200 でも、先頭の行の版の字の側へ移った（4.1・10.1 節）。細かい版（2.0〜2.6）で書き分けない（SSP も種類ごとに決まった行を送っていた）。 |
| 4 イベントの種類を下へ渡す形 | 下の層が GET／NOTIFY の別とイベント名で決める（案 ⑵）。`GET`＋`OnTranslate`＝翻訳・`GET`＋`On…`＝台本・`GET`＋その他＝資源・`NOTIFY`＋`basewareversion`＝送らない・`NOTIFY`＋その他＝`GET Sentence` | 種類を運ぶ案 ⑴ は `ShioriCall`（kanade の `msg.rs`＝ウェーブの約束で触れない）と `ShioriBackend` の口・偽物の全部へ波及する。ukadoc のイベント名は全部 `On` で始まり、資源の名前は 1 つも `On` で始まらない。SSP の記録も同じ線で分かれていた。 |
| 5 3.0 のゴーストの確かめへの 400 | `shiori_error_response` の WARN に数えない。INFO の `evidence` に状態の行を残す | 確かめは kanade の往復の関門を通らない。数えると `Version` の受け口の無い 3.0 のゴーストで起動のたびに WARN が増え、要件 5.6 の内訳を濁す。SSP も状態の番号で振る舞いを変えていない。 |
| ファイルの分け方 | 新 `shiori2.rs`（純粋な 2.x の書き方）＋兄弟の `shiori2_tests.rs`。版で選ぶ分岐は `client.rs` の 2 つの手の中。確かめの記録は `shiori_wiring.rs` | `shiori3.rs` は 852 行。`client.rs` は 858 行で足すのは 50 行ほど。テストは兄弟のファイルへ（`client_shiori2_tests.rs`・`shiori_wiring_version_tests.rs`）。 |

### 11.2 そのほかの決め

- **確かめの答えを文字コードの採用に使わない**。使うと、宣言の無い UTF-8 の 3.0 ゴースト（emo2 の pasta）で、今は `Shift_JIS` を名乗る `OnInitialize`・`username` の要求が `UTF-8` を名乗るように変わり、要件 6.1 に反する。確かめの要求の `Charset` は今の文字コード（SSP は `UTF-8` 固定）。本文は ASCII だけなので、違いはこのラベルだけ。
- **版は `ShioriConnection` の欄**（文字コードの交渉と同じ「接続の持ち物」）。`Shiori3Client` は `with_version` で写しを受ける。kanade の結合テストの 3 ファイルは構造体の書き下しに欄を 1 行足す（コンパイルのためだけ・要件 6.3 で名前を挙げる）。包む型を新しく作る案は、6 つの手を全部写すので採らない。
- **答えの読みは `shiori3::parse_response_with(bytes, policy, value_header)`** に本体を移し、3.0 は `"Value"`、2.x は種類に応じて `Sentence`／`String` を渡す。2.x では `ErrorLevel`・`ErrorDescription`・`SecurityLevel` を読まない（測 4 で 2.x に無い・要件 4.7）。
- **`Sender` は `areka`** のまま（SSP を名乗らない今の決め）。`BaseEvent` は書かない（areka は 3.0 でも `BaseID` を送っていない）。`NOTIFY OwnerGhostName`・`NOTIFY OtherGhostName`・`capability` は areka が今送っていないので組まない。
- **Reference の付く資源の問い合わせ（`homeurl` など）**は SSP の記録に無かった。3.0 と同じ値を同じ番号で `Status` の後に書く（`GET Sentence` と同じ並べ方）。
- **INFO の `ghost` はゴーストのフォルダ名**（`ghost\master` の 2 つ上）。`real_connect` は descript の `name` を受け取らず、受け取らせるには `runtime.rs`（C11 ④ の持ち物）の呼び方を変える要がある。
- **確かめは `real_connect` の閉包の中で 1 回**。閉包は接続ごとに 1 回走るので、読み直し・切り替えで確かめ直す（要件 2.3）。この受け渡しは中継が無いと挙動で確かめられないので、先例（`the_connection_receives_the_already_decided_negotiator`）と同じく本文の構造で検査する。

### 11.3 まとめの規則の当てはめ

- **一般化**: 要件 3.1〜3.4 は「種類で要求の行と見出しを選ぶ」1 つの問題。表を 1 か所（`shiori2` の私の手 `route`）に持ち、組み立てと読みの両方がそれを引く。口は `build_request`・`parse_response` の 2 つだけ。
- **作るか借りるか**: SHIORI/2.x の符号化を持つ既製のクレートは無い（ukadoc にも項目が無い）。符号化は既存の `Charset`（`encoding_rs`）を借り、状態の行・`Charset` の拾い方・読めない応答の判定は 3.0 の読みを借りる（`parse_response_with`）。
- **削ったもの**: 種類を kanade から運ぶ口・2.x の細かい版での書き分け・版を設定で固定する手・確かめのための新しい待ちの仕組み・`OwnerGhostName` などの送っていない要求。

### 11.4 残る危うさ

- 3.0 の検体のうち確かめへの答えを測ったのはぱすただけ（`SHIORI/3.0 204`）。emo2・里々・YAYA は実機の 1 回で版の INFO の行を見る。2.x に決まる 3.0 の検体があれば止めて報告する（SSP と同じ規則なので SSP でも同じに振る舞うはずだが、要件 6 と当たる）。
- `OnClose` の 2.x の形は測っていない（10.5 節）。実機の 1 回で areka が送った形と答えを記録する。

### 11.x 設計の話し合いでの直し（2026-10-23）

- 資源の問い合わせかどうかを「名前が `On` で始まるか」で分ける案をやめ、kanade の正本の一覧（`schedule::resources::is_allowed_resource_id`）を kanade の `shiori/real.rs` が引いて `Shiori3Client::for_resource` で渡す形にした（設計の検証の問題 1）。SSP は `installedghostname`・`hwnd` など `On` で始まらないイベントも `GET Sentence` で送っており（10.2 節）、名前の形では分けられない。SSTP や MCP の `raise_event` から頼まれた任意の名前のイベントも `GET Sentence` になる。
