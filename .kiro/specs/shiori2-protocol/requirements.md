# Requirements Document

## Project Description (Input)
SHIORI/2.x の口だけを持つゴースト（検体は開発者のゴースト「．さくら」・SHIORI は華和梨 7.4）が、areka では一言も話さない。起動・時間の通知・`OnClose`・`OnTranslate` の要求が全部 `status 400` で断られ、台本が 1 本も返らず、キャラクターの窓に面も出なかった（根拠は `.kiro/specs/completed/areka-P0-dot-sakura-specimen/verification.md` の 5 節・8 節の件 1 と件 7）。areka は相手の SHIORI の版を確かめず、誰にでも SHIORI/3.0 で話している。ベースウェアの側で相手の版を確かめ、2.x の相手にはイベントを 2.x の要求へ組み替えて送り、応答を 3.0 と同じ形へ戻して、2.x のゴーストが SSP と同じように話すようにする。SHIORI/3.0 のゴーストの振る舞いは変えない。開発者の条件は「次のリリース v0.1.0 までに治ること」。

## Introduction

古いゴーストの中には、SHIORI（ゴーストの頭脳にあたる DLL）が SHIORI/2.x の書き方しか受け付けないものがある。areka は今、どの相手にも SHIORI/3.0 の書き方で話しかけるので、そうしたゴーストには全部の問いかけを「読めない」（400）と断られ、ゴーストは黙ったまま、キャラクターの姿も出ない。開発者のゴースト「．さくら」（`vendors/sample_ghost/dot_sakura.nar`・華和梨 7.4）がその例で、2026-10-22 の実機の 1 回で、記録の `shiori_error_response` 132 行が全部 `status 400` だった。

本 spec は、areka が相手の SHIORI の版を確かめ、2.x の相手には 2.x の書き方で話しかけ、返事を 3.0 のときと同じに扱えるようにする。到達点は「．さくら」が areka で SSP と同じように、起動の挨拶・時間のトーク・終了のトークを話し、面を出すこと。SHIORI/3.0 のゴースト（emo2・ぱすたさん・里々・YAYA の検体）の振る舞いは変えない。

### 正典の出どころと、正典で読めない点

正典は ukadoc。ただし **ukadoc には SHIORI/2.x の要求と応答の書き方の項目が無い**（2026-10-23 に ukadoc の MCP で「SHIORI/2」「Sentence」「GET Version」「2.x」を引いた結果、2.x に触れるのは次の 2 か所だけ）。

- ステータスコード（<https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#response>）: 「400 Bad Requestリクエスト文字列が解釈不能だった。」・「311 Not EnoughID: OnTeach (SHIORI/2系のTEACHメソッド)において、さらなる追加情報を必要とすることを示す。」
- Age（同ページ）: 「SSP拡張だが、実質はSHIORI/2.x互換処理。」

SHIORI/2.x の要求の行（`GET Sentence`・`GET String`・`NOTIFY OwnerGhostName`・`TRANSLATE Sentence`・`GET Version` など）、応答の見出し（`Sentence:` など）、版の確かめ方は ukadoc で読めない。参考に、里々の文書（<https://ukatech.shillest.net/satori-docs/shiori/protocol/>・旧 URL <https://ukatech.github.io/satori-docs/shiori/protocol/> から移っている）は SHIORI 側の受け方として「SHIORI/2.x | Event を ID に読み替え、応答は SHIORI/3.0 形式（Sentence に読み替え）」「GET Version には 200 OK を返します」と書くが、これは SHIORI の側の作りで、ベースウェアが何を送るかの正典ではない。

そこで、ukadoc で読めない点は **SSP が同じゴーストへ送る要求の記録（ゴーストの `ghost\master\ssp_shiori_log.txt`）で測ったものを正本とする**（開発者方針「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。測る前の手がかり: 完了 spec `areka-P0-mcp-reload` の調べ（`.kiro/specs/completed/areka-P0-mcp-reload/research.md` 10 節）は、SSP が SHIORI/3.0 のゴーストを読み直したとき、load の直後に `GET Version` を送り、続けて `version` を問うことを記録している（要求の行のバイトまでは控えていない）。

要件 1 の測りで決める点（本書を書いた時点で未測定・推測で埋めない。**2026-10-23 に SSP 2.9.23 で測った結果は `research.md` 10 節**。測 1＝華和梨 7.4 は 2.x・測 7＝SSP では面が出たので測らない・`OnClose` の形だけは測っていない）:

| 番号 | 測って決めること |
|---|---|
| 測 1 | SSP が「．さくら」へ SHIORI/3.0 で話しかけているか、2.x で話しかけているか。華和梨 7.4 が SHIORI/3.0 を本当に受け付けないか（検証記録は「2.x だけの見込み」で、3.0 の要求に 400 が返ったことからの推し） |
| 測 2 | SSP が相手の版をどう確かめるか（load の直後の `GET Version` か・3.0 の要求に 400 が返ったら 2.x へ移るか・その両方か）と、その要求と答えのバイト |
| 測 3 | areka が今送っている要求の種類（台本を求めるイベント・通知だけのイベント・資源の問い合わせ〔`username`・`homeurl`・メニューの項目名など〕・`OnTranslate`）ごとに、SSP が 2.x の相手へ送る要求の行・見出し・References の載せ方 |
| 測 4 | 2.x の相手が返す応答の見出し（台本・資源の値・翻訳の結果がどの見出しに入るか）と状態の番号 |
| 測 5 | SSP が 2.x の相手には送らないイベントがあるか |
| 測 6 | 2.x の要求に `Charset`・`Sender`・`SecurityLevel` などの見出しを SSP が付けるか |
| 測 7 | （要件 5.5 の枝に入ったときだけ）SHIORI が台本を返さないとき、SSP が既定の面を出すか |

## Boundary Context

- **In scope**:
  - 相手の SHIORI の版（3.0 か 2.x か）を、読み込むたびに確かめて覚えること。
  - 2.x の相手へ、areka が今送っている要求を 2.x の書き方に組み替えて送ること（台本を求めるイベント・通知だけのイベント・資源の問い合わせ・`OnTranslate`）。
  - 2.x の応答を、3.0 の応答と同じ扱いへ戻すこと。
  - 件 2（起動しても面が 1 度も出ない）を、件 1 が直った後に実機で確かめること。
  - 偽の 2.x の SHIORI での決定論のテストと、「．さくら」の実機の 1 回。
  - 組み替えの表と出どころの記録（`doc/COMPAT_ARCHITECTURE.md` §8・網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の該当する行・`README.md` の検体の表の「．さくら」の行）。
- **Out of scope**:
  - 32 ビットの中継（`crates/shiori-host32-helper/`）の中身。要求のバイトを解釈せず素通しする今の形のまま。
  - SHIORI の文字コードの扱い（`areka-P0-charset-canon` の持ち場）。2.x の要求も 3.0 と同じ文字コードの決め方に乗る。
  - MAKOTO（`makoto-dll-host`・`EXECUTE MAKOTO/2.0` は別の口）。
  - SHIORI のエラー応答（400・500）を致命としない決め（変えない）。
  - 起動で台本が来ないときに面を出さない決め（SSP に合わせた決め・件 2 は確かめるだけ）。
  - areka が今送っていない要求（ゴースト同士の会話〔コミュニケート〕・TEACH・SHIORI/1.x など）を新しく送ること。2.x の書き方を足すのは、areka が今 3.0 で送っている要求に限る。
  - ゴーストの側（華和梨の辞書）の直し。
- **Adjacent expectations**:
  - `shiori4-api`（v0.2.0）は「SHIORI/2.x の見分けはこの spec の外＝areka の `shiori2-protocol`」と決めている。本 spec の版の確かめは、後で helper が乗り換わっても、要求を組む側（64 ビットの側）に残る形でよい。`shiori4-api` の決定 3（SHIORI はどの版でも別プロセスで動かし、同じプロセスの道 `ShioriWiring::InProc` と今の SHIORI4 の口を消す・2026-10-23 開発者が本 spec の話し合いでも確認）により、本 spec は同じプロセスの道（`crates/areka-ghost/src/shiori_inproc.rs`）に触らない。
  - `audit-shiori-host32`・`makoto-dll-host`・`mcp-shiori-query`・`property-ipc-transport`・`property-query-channels` は同じファイル群を挙げる後の版の spec。`popup-menu-residue`（版に縛らない）は kanade の往復の失敗の文言で重なる。
  - 同じウェーブ C11 の約束（`.kiro/steering/roadmap.md`「ウェーブ編成」の C11 の ⑧ の行）: kanade の `msg.rs`・`schedule/` と `crates/shiori-host32-helper/` に触らない。版の確かめに起動の運行の変更が要ると分かったときは、回避の形へ逃げず、持ち主のセッション（`mouse-click-wheel-events`）と調整する（開発者方針「設計は本質的な案で」）。

## Requirements

### Requirement 1: 着手前の確かめ（SSP の記録で測る）
**Objective:** As a 開発者, I want 2.x の書き方を組む前に SSP が同じゴーストへ何を送っているかを測って記録してほしい, so that 推測でなく SSP と同じ要求を areka が送れる

#### Acceptance Criteria
1. When 2.x の要求の組み立てに着手する前に, the 本 spec の調べ shall SSP で「．さくら」を動かし、`ghost\master\ssp_shiori_log.txt` の操作前のバイト位置から後ろを読んで、上の表の測 1〜測 6 の答えを要求と応答のバイトのまま調べの記録（`research.md`）に書く。
2. If 測 1 で、華和梨 7.4 が SSP からの SHIORI/3.0 の要求を受け付けていると分かったなら, the 本 spec の調べ shall 2.x の書き方の作業へ進まず、400 の原因の見立てと記録の抜き書きを開発者へ報告して扱いを仰ぐ。
3. The 本 spec の調べ shall SSP の読み直し（reload）・ゴーストの切り替え・設定の変更を伴う操作を、1 回ごとに開発者の許しを得てから行い、測る前に窓の位置を控えて、測った後に戻す。ただし次の 1 回の測りの 5 つの操作は、2026-10-23 の要件の話し合いで開発者がまとめて許した（議題 1）: ⑴ `vendors/sample_ghost/dot_sakura.nar` を SSP に入れる ⑵ 「．さくら」の SHIORI の要求の記録を出す設定を入れる ⑶ 記録の大きさを控えて「．さくら」へ切り替え、1〜2 分待つ ⑷ `OnClose` を 1 回送る ⑸ 元のゴーストと窓の位置へ戻す。「．さくら」は測った後も SSP に残す。この 5 つに無い操作は、これまでどおり 1 回ごとに許しを得る。
4. The 本 spec の調べ shall 組み替えの表の各行に、出どころ（ukadoc の URL と逐語の引用、または SSP の記録の抜き書き）を付ける。ukadoc に書かれていない行は「ukadoc に無い・SSP で測った」と明記する。
5. When 要件の話し合いが終わったとき, the 本 spec の調べ shall 設計に入る前に要件 1.1 の測りを済ませる。

### Requirement 2: 相手の SHIORI の版を確かめて覚える
**Objective:** As a ゴーストの利用者, I want areka が相手の SHIORI の版を確かめてから話しかけてほしい, so that 古いゴーストでも最初の挨拶から話してくれる

#### Acceptance Criteria
1. When SHIORI を読み込んだとき（起動・ゴーストの切り替え・SHIORI の読み直しのどれでも）, the areka shall 相手が SHIORI/3.0 か SHIORI/2.x かを要件 1 で測った SSP と同じ確かめ方で決め、起動の通知と台本を求めるイベント（`OnInitialize`・`OnBoot` など）が版の違いのために断られたまま失われないようにする。
2. While 同じ読み込みが続いている, the areka shall 決めた版をその読み込みのあいだ覚えて使い、要求のたびに確かめ直さない。
3. When SHIORI を降ろして読み込み直したとき, the areka shall 前の読み込みで決めた版を使わず、要件 2.1 の確かめをやり直す。
4. When 版を決めたとき, the areka shall ゴーストの名前・決めた版・決めた根拠（相手の答えた版の文字列、または 400 などの状態）を INFO の記録に 1 行残す。
5. If 確かめの答えから版を決められない（答えが無い・読めない）なら, the areka shall 今と同じ SHIORI/3.0 として話し、その旨を WARN の記録に 1 行残す。
6. The areka shall 版の確かめと 2.x の書き方を、SHIORI を別プロセス（中継）で動かす道にだけ持たせ、同じプロセスで DLL を読む道（`ShioriWiring::InProc`・SHIORI4 の DLL 専用）には足さない（2026-10-23 開発者の決め「SHIORI はすべて別プロセスで動かす・今の SHIORI4 の口は破棄する」＝`shiori4-api` の決定 3。その道は `shiori4-api` で消える）。

### Requirement 3: 2.x の相手へは 2.x の書き方で送る
**Objective:** As a ゴーストの利用者, I want SHIORI/2.x のゴーストへ、そのゴーストが読める書き方で話しかけてほしい, so that 起動・時間・終了・翻訳のどの場面でもゴーストが答えられる

#### Acceptance Criteria
1. While 相手が SHIORI/2.x と決まっている, when areka が台本を求めるイベント（`OnBoot`・`OnFirstBoot`・`OnSecondChange`・`OnClose` など）を送るとき, the areka shall 要件 1 で測った SSP と同じ 2.x の要求の行・見出し・References の載せ方で送る。
2. While 相手が SHIORI/2.x と決まっている, when areka が通知だけのイベント（`OnInitialize`・`ownerghostname` など、ukadoc が [NOTIFY] と書くもの）を送るとき, the areka shall 要件 1 で測った SSP と同じ 2.x の通知の書き方で送る。
3. While 相手が SHIORI/2.x と決まっている, when areka が資源の問い合わせ（`username`・`homeurl`・メニューの項目名など）を送るとき, the areka shall 要件 1 で測った SSP と同じ 2.x の書き方で送る。
4. While 相手が SHIORI/2.x と決まっている, when areka が台本の翻訳（`OnTranslate`）を頼むとき, the areka shall 要件 1 で測った SSP と同じ 2.x の翻訳の書き方で送る。
5. If 要件 1 の測りで、SSP が 2.x の相手へ送らないイベントがあると分かったなら, the areka shall そのイベントを 2.x の相手へ送らず、送らなかったことを DEBUG の記録に残す。
6. Where SSP が 2.x の要求にも `Charset`・`Sender`・`SecurityLevel` などの見出しを付けると測れた, the areka shall 3.0 の要求と同じ値をその見出しに載せる（文字コードの決め方は今のまま）。
7. The areka shall 3.0 のイベントから 2.x の要求への組み替えを 1 つの表として持ち、その表を `doc/COMPAT_ARCHITECTURE.md` §8 に出どころ付きで記録する。

### Requirement 4: 2.x の応答を 3.0 と同じ扱いへ戻す
**Objective:** As a ゴーストの利用者, I want 2.x のゴーストの返事を、3.0 のゴーストの返事と同じに扱ってほしい, so that 台詞・メニューの項目名・翻訳の結果がいつもどおりに出る

#### Acceptance Criteria
1. When 2.x の相手が台本を求めるイベントに台本付きで答えたとき（要件 1 の測 4 で決めた見出し）, the areka shall その台本を、3.0 の応答の `Value` の台本と同じに再生する。
2. When 2.x の相手が資源の問い合わせに値付きで答えたとき, the areka shall その値を、3.0 の応答の `Value` と同じに使う（例: メニューの項目名・`username`）。
3. When 2.x の相手が翻訳の頼みに台本付きで答えたとき, the areka shall その台本を、3.0 の `OnTranslate` の応答の `Value` と同じに扱う。
4. When 2.x の相手が「返す物が無い」と答えたとき（204、または台本の見出しが無い 200）, the areka shall 3.0 の 204 と同じに扱う。
5. If 2.x の相手が 400 や 500 で答えたなら, the areka shall 今の 3.0 と同じく致命とせず、`shiori_error_response` の WARN を 1 行残して会話を続ける。
6. If 2.x の応答が読めない（状態の行が無い・壊れている）なら, the areka shall 今の 3.0 の読めない応答と同じに扱い、記録を残す。
7. Where 3.0 の応答から areka が今読んでいる見出し（台本・値のほか）に、測 4 で同じ意味の 2.x の見出しが見つかった, the areka shall その 2.x の見出しを 3.0 の見出しと同じに扱う。測 4 で見つからなかった見出しは扱わず、表に「2.x に無い」と書く。

### Requirement 5: 「．さくら」が話し、面が出る（実機の 1 回）
**Objective:** As a 開発者, I want 自分のゴースト「．さくら」が areka で SSP と同じように話して姿を見せることを、実機の 1 回で確かめてほしい, so that v0.1.0 までに治ったと言える

#### Acceptance Criteria
1. When 「．さくら」を areka で起動したとき, the areka shall 起動の挨拶の台本を受け取って再生し、バルーンに台詞を出す。
2. When 「．さくら」が時間の通知（`OnSecondChange`・`OnMinuteChange` など）に台本を返したとき, the areka shall その台本（時間のトーク）を再生する。実機の 1 回で時間のトークが 1 度も来なかったときは、その旨と待った時間を記録に書く。
3. When 「．さくら」に終了のイベント（`OnClose`）を送ったとき, the areka shall 返ってきた終了のトークを再生し、終了まで通す（`OnClose` のトークは必ず終了で終わるという既存の決めのとおり）。
4. When 起動の挨拶の台本が再生されたとき, the areka shall 1 人目・2 人目のキャラクターの窓に面を出し、MCP の `dump_surface` が両方とも面の絵を返す（`NG:No surface has been shown in this scope yet` を返さない）。
5. If 件 1 が直った後も 1 人目か 2 人目の窓に面が 1 度も出ないなら, the 本 spec の調べ shall 面を出さない今の決めを独断で変えず、SHIORI が台本を返さないときに既定の面を出すかを ukadoc で引き、ukadoc で読めなければ SSP で測り（測 7）、結果と扱いの案を開発者へ報告して決めを仰ぐ。
6. The 実機の 1 回の記録 shall 版の確かめの INFO の行・台本を返したイベントの数・`shiori_error_response` の行の数とその内訳・MCP の答え（`get_status`・`dump_surface`・`raise_event OnClose`）を、検証の記録に抜き書きで残す。辞書に受け口の無いイベント（「．さくら」では翻訳の `OnTranslate`・`OnInitialize` など）への 400 は SSP でも同じく返るので、内訳では別に数え、崩れに数えない。
7. The 実機の 1 回 shall 根・検体・一時フォルダをこのワークツリーの `target\` の下だけに置き、自分が起こした areka のプロセスだけを扱う。

### Requirement 6: SHIORI/3.0 のゴーストの振る舞いを変えない
**Objective:** As a 3.0 のゴーストの利用者, I want 2.x への対応のために今のゴーストの動きが変わらないでほしい, so that emo2・ぱすたさん・里々・YAYA の検体がいつもどおり動く

#### Acceptance Criteria
1. While 相手が SHIORI/3.0 と決まっている, the areka shall 各イベントの要求のバイトを今と同じに送る。要件 2.1 の確かめのために SSP と同じく足す要求があるなら、それだけを違いとして表に書く。
2. While 相手が SHIORI/3.0 と決まっている, the areka shall 応答の読み方を今と同じに保つ。
3. The areka shall SHIORI/3.0 のゴーストを相手にした今の決定論のテストを、変えずに緑のまま通す。要件 2.1 の確かめで要求の数や並びを固定したテストが変わるなら、その変更を確かめの要求の分だけに限り、変えたテストの名前を記録に挙げる。

### Requirement 7: 決定論のテスト
**Objective:** As a 開発者, I want 2.x の書き方と読み方を、実物の SHIORI や SSP なしで毎回固定して確かめたい, so that 後の変更で 2.x のゴーストがまた黙ることを防げる

#### Acceptance Criteria
1. The 決定論のテスト shall 偽の 2.x の SHIORI を相手に、組み替えの表の各行について、areka が送る要求のバイトを固定する。
2. The 決定論のテスト shall 偽の 2.x の SHIORI の応答（台本付き・値付き・返す物が無い・400・500・読めない）ごとに、areka の扱い（要件 4 の各項）を固定する。
3. The 決定論のテスト shall 版の確かめを、3.0 と答える相手・2.x と答える相手・版を決められない相手の 3 通りで固定する（要件 2.1・2.5）。
4. The 決定論のテスト shall 読み込み直したときに版を確かめ直すこと（要件 2.3）を固定する。
5. The 決定論のテスト shall SSP・実物の華和梨・ネットへの接続を要さず、全体テストの常時のテストとして回る。

### Requirement 8: 記録と文書
**Objective:** As a 開発者, I want 2.x への対応の決めと出どころが文書に残ってほしい, so that 後の spec（`shiori4-api`・`audit-shiori-host32` など）が同じ決めの上に乗れる

#### Acceptance Criteria
1. The 本 spec shall 組み替えの表（要件 3.7）と版の確かめ方（要件 2.1）を、出どころ（ukadoc の引用、または SSP の記録の抜き書き）付きで `doc/COMPAT_ARCHITECTURE.md` §8 に記録する。
2. The 本 spec shall 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` のうち、本 spec が受け持つ ukadoc の項目の行（2.x に触れる項目を含む）の状態と持ち主を、実装の結果に合わせて直す。ukadoc に項目の無い 2.x の要求は台帳に作らず、§8 の記録だけに置く。
3. When 実機の 1 回で「．さくら」が話すと確かめられたとき, the 本 spec shall `README.md` の検体の表の「．さくら」の行（今は「話さず、面も出ません」）を実際の結果に合わせて直す。
4. The 本 spec shall 本 spec で直さなかった「．さくら」の崩れ（実機の 1 回で新しく見えたもの、およびギャップ分析で既に見えた古い資源の名前〔`updatebuttoncaption`・`vanishbuttoncaption` など・台帳では持ち主の無い `vocabulary-only`〕を areka が問わない件）を、持ち主の spec の brief へ足すか `/kiro-discovery` で起票し、記録だけで済ませない。
