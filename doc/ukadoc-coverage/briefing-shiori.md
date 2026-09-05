# SHIORI ドメインの調査ブリーフィング

この文書は、SSP 公式仕様書（ukadoc）のうち「ベースウェアと SHIORI／外部との対話面」に属する
677 項目について、areka がどこまで実現しているかを人が読める形で示すものである。数字ではなく
「利用者に何が起きるか」で読めることを目指している。機械が読む側の正本は台帳
`doc/ukadoc-coverage/ledger/shiori.toml` にある。

**結論**: 677 項目のうち、areka が正典どおりに動かしているのは **21 件**（3.1%）である。一部だけ
できているものが 3 件あり、**未対応が 481 件**（送り先・受け口が無いもの 320 件と、名前だけ登記して
あって実際には引かないもの 161 件）、判定の対象にならないものが 172 件ある。**この文書が対応の
順序の対象として扱う群はすべて未対応であり、対応の予定を述べるものではない。** 既にできている
21 件と判定の対象にならない 172 件も、群を漏れなく並べるために章を持つ（前者は第 4 部、後者は
168 件が第 3 部・4 件が第 4 部）。

**段階と優先度は仮置きである。最終決定は統合担当の `ukadoc-coverage-roadmap` が行う。** 台帳の
`priority` 欄と、この文書に出てくる `A1`〜`E1` の値は、群の中と群どうしの並びを揃えるための
出発点であって、着手の順番の約束ではない。

**この文書の構成**: まず群の索引（この文書が正本・機械で読む側の写しが台帳の冒頭にある）を置き、
その後ろに群ごとの本文を置く。本文は「黙って壊れるもの」→「テーマは付いているが現れ方が見た目の
差にとどまるもの」→「受け口が無い外部との連携」→「判定の対象にならない群と、既にできている群」
の順に並ぶ。末尾に、次に読む人への申し送りと、是正の候補を置く。

**状態ごとの件数**（台帳の `status` 欄の実測・合計 677）

| 状態 | 件数 | 意味 |
|---|---|---|
| `implemented` | 21 | 実際に送っている・引いている・読み取っている |
| `degraded` | 3 | 一部だけできている |
| `absent` | 320 | 送る場所・引く場所・受ける口がコードに無い |
| `vocabulary-only` | 161 | 名前は登記してあるが実際には引かない |
| `alias` | 3 | 旧仕様の別名（判定は写像先が持つ） |
| `not-applicable` | 169 | areka が送る・引く・受ける対象ではない |

---

## 群の索引（この文書が正本）

677 項目は 18 の群で尽くされる。群の番号は 1 から 15 までで、そのうち 2 つ（群 2 と群 14）が
枝分かれするため行数は 18 になる。

この索引は**正本**である。台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の冒頭にも同じ索引の
写しを `#` のコメントとして置いてあるが、台帳のコメントは道具が項目を差し込む処理を通したときに
残る保証が無いので、写しが消えてもこちらが残る。

**貼り直しの手順**: 写しと正本が食い違っても機械の検査は赤にならない（`check` は `#` のコメントを
読まない）ので、この索引を直すときは必ず**正本 → 写しの順で、写しは節の全体を貼り直す**
（写しだけを部分的に直さない）。

**共通 `note` の使い方**: 下の各群にある「共通 `note`」は、台帳のその群に属する行の `note` へ
そのまま写す文面である。仕分けの作業は、この文面を写したうえで、その項目にだけ当てはまることを
必要な分だけ書き足す形で進める。

**テーマと優先度がどこに書いてあるか**: 群の欄に「項目ごと」と書いてあるもの（群 1・2・6・7・8）は、
テーマを群で 1 つに決めず 1 項目ずつ判断するという意味である。**決まった値の正本は台帳の各行の
`values` 欄**であり、この索引の欄はその出発点にすぎない。優先度も同じく台帳の `priority` 欄にあり、
**677 行のうち 484 行に記入済みで、193 行は意図的に空**である。空にしてあるのは、着手の順番を
考える対象にならない 3 つの状態の行——`implemented` 21・`alias` 3・`not-applicable` 169——で、
この 3 つを足すと 193 になる。下の群の欄が「**優先度**: `""`」と書いてある 7 つの群（群 1・3・5・
6・9・12・15）がこれに当たる。記入済みの 484 行についてこの索引に書いてあるのは群ごとの既定値で
あり、段階（A〜E）と数値はいずれも**仮置き**である。最終的な順序を決めるのは統合担当の
`ukadoc-coverage-roadmap` である。

**優先度はいずれも仮置きである**: 台帳の `priority` 欄には、下の各群が定める値を置いてある。
段階の 1 文字（A〜E）と、その後ろの数値の**どちらも仮置き**であり、最終的にどの順で
手を着けるかを決めるのは統合担当の `ukadoc-coverage-roadmap` である。ここに書いてある値は、
群の中と群どうしの並びを揃えるための出発点であって、着手の順番の約束ではない。値の決め方は
設計の「群→段階」の表に従い、表に当たらない項目は同じテーマの群の値を写した。優先度の
4 つの根拠の序列（壊れ方 ＞ 伺からしさ ＞ 影響する既存資産の広さ ＞ 基盤の共有度）は入れ替えていない。

**壊れ方の段**: 下の共通 `note` はすべて、次の 3 つのどれに当たるかを最初に述べる。

- **黙って壊れる** — 例外にもログにも現れず、「その場面で何も喋らない」という形でだけ現れる。
- **明示的なエラー** — 失敗が呼び出し元へ返り、error のログが残る。
- **見た目の差** — 動くが、見え方が正典と違う。

そして必ず「どのログが出るか・出ないか」を添える。「ログが出ない」と書いてある群は、grep して
見つからなかったという意味ではなく、**その経路に到達する場所がコードに無い**という構造上の理由で
出ない。ただしこの言い方が成り立つ範囲には境界があるので、先にそれを述べる。

areka の送出の口は 1 か所（`crates/areka-kanade/src/actor.rs` の `round_trip_request`）だが、
そこへ渡る呼び出しの**出所は 2 系統**ある。

- **⑴ スケジューラ起源** — `crates/areka-kanade/src/schedule/events.rs` と
  `crates/areka-kanade/src/schedule/resources.rs` の構築関数が組み立てる系統。名前は
  `crates/areka-kanade/src/msg.rs` の `EventId::Static` が運ぶ `&'static str` で、構築関数が
  書いた文字列しか入らない。ベースウェアが場面に応じて自分から発火するのは、この系統である。
- **⑵ 選択起源** — ゴースト作者が `\q[タイトル,ID,…]` の ID に書いた名前を、逐語のまま運ぶ系統。
  `crates/areka-kanade/src/schedule/steady.rs` の選択受理（`CascadePlan::Named` の枝）が
  `crates/areka-kanade/src/schedule/events.rs` の `on_choice_named` を通し、名前は
  `crates/areka-kanade/src/msg.rs` の `EventId::Choice` が運ぶ。受理規則は同じ `events.rs` の
  `is_allowed_choice_event` で、条件は「`On` で始まること」ただ 1 つであり、固定の表への登録を
  要求しない。つまり `On` で始まる名前であれば、正典のどの名前でも、ゴーストが選択肢の ID に
  書いた時点で areka は実際に送出する。この経路は `doc/emo2-conformance-scope.md` にも
  「ID が `"On"` で始まっている場合は、選択後、SHIORI イベント `OnID` が開始される」と記録がある。

**「ログが 1 行も出ない」と書けるのは⑴の系統に限る。** ⑴については、構築関数を持たない正典の
項目は「送ろうとして断られる」のではなく、**送ろうとする場所そのものが無い**（送出を止める判定
＝ error のログ `event_id_not_allowed` は組み立て済みの呼び出しにしか働かないので、⑴の側では
この判定に到達しない）。一方⑵については、`On` で始まる名前はゴーストの書きよう次第で送出され得る
ので、そのときは trace のログ `shiori_request` がその名前で 1 行出る。ただしそれはベースウェアが
正典の発火条件で発火したものではないため、**台帳の状態は⑴の系統だけで決める**（⑵で送出され得る
ことは状態を動かさない）。

---

### 群 1 — 送出しているイベント

- **対象**: areka が実際に SHIORI へ送っている固定名のイベント。
- **件数**: 11
- **状態**: `implemented`
- **テーマ**: 項目ごと（`values.md` の 8 つから選ぶ）
- **優先度**: `""`（作業が残っていない）
- **判断の根拠の場所**: `crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS` と
  同じファイルの各構築関数（`on_initialize`・`on_boot` ほか）。送出は
  `crates/areka-kanade/src/actor.rs` の `round_trip_request` の 1 か所。
- **共通 `note`**:

  ```
  壊れ方: 該当なし。areka はこのイベントを実際に SHIORI へ送っている。
  ログ: 送出の直前に trace のログ（shiori_request）が 1 行出て、Method・ID・参照値・実行状態が
  残る。送出そのものが失敗したときは error のログ（shiori_send_failed）が出て終了の系列へ倒れる
  ので、黙って消えることはない。
  根拠の場所: crates/areka-kanade/src/schedule/events.rs の ALLOWED_EVENT_IDS にこの名前が載って
  おり、同じファイルの構築関数がこの呼び出しを組み立てる。送出は
  crates/areka-kanade/src/actor.rs の round_trip_request。
  ```

---

### 群 2 — 送出していないイベント（`On` 始まり）

- **対象**: `list_shiori_event` のうち名前が `On` で始まり、areka が送っていないもの。
- **件数**: 248（290 − 実装済み 11 − `On` 以外 25 − 別名 3 − 意図的に発火させない 3）
- **状態**: `absent`
- **テーマ**: 項目ごと（8 つから選ぶ。付与規則に答えられないものは空）
- **優先度**: 項目のテーマから決まる（`A1`／`A2`／`A3`／`B1`〜`B5`／`C1`／`C2`／`D1`／`D2` の
  いずれか・仮置き）
- **判断の根拠の場所**: `crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS`
  （この名前が載っていない）と同じファイルの構築関数（この名前を組み立てるものが無い）、
  `crates/areka-kanade/src/actor.rs` の `round_trip_request`
  （送出の唯一の口と `event_id_not_allowed` の判定）、`crates/areka-kanade/src/msg.rs` の
  `EventId::Static`（固定名は構築関数の書いた文字列だけを運ぶ）。選択起源の但し書きの根拠は
  `crates/areka-kanade/src/schedule/steady.rs` の `CascadePlan::Named` の枝と、
  `crates/areka-kanade/src/schedule/events.rs` の `on_choice_named`・`is_allowed_choice_event`。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。ベースウェアが場面に応じてこのイベントを発火する経路が areka に無いので、
  既存ゴーストの辞書に用意された返事はその場面で 1 度も呼ばれず、「何も喋らない」という形でだけ
  現れる。
  ログ: ベースウェアが場面に応じて発火する系統では 1 行も出ない。ALLOWED_EVENT_IDS にこの名前が
  載っておらず、schedule/events.rs にこれを組み立てる構築関数も無いので、「送ろうとして断られる」
  のではなく、そもそも組み立てる場所が無い。例外も出ない。ただしゴーストが \q の選択肢 ID にこの
  名前を書いた場合だけは別で、選択起源の経路（schedule/steady.rs の CascadePlan::Named の枝が
  events.rs の on_choice_named を通す。受理規則 is_allowed_choice_event の条件は「On で始まること」
  ただ 1 つ）で名前が逐語のまま送出され、そのときは trace のログ shiori_request がこの名前で
  1 行出る。これはベースウェアが正典の発火条件で発火したものではないので、この行の状態は
  absent のまま変わらない。
  ```

---

### 群 2a — M1 で意図的に発火させていないバルーンのイベント

- **対象**: `OnBalloonClose`・`OnBalloonTimeout`・`OnBalloonBreak`。
- **件数**: 3
- **状態**: `vocabulary-only`
- **テーマ**: 気配り
- **優先度**: `C2`（仮置き）
- **判断の根拠の場所**: `crates/areka/src/emo2_boot/talk_lifecycle.rs` の
  `BalloonLifecycleNotice`（予約してある受け渡し口の型）。転記元は
  `doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表の行「`OnBalloonClose` ／ `OnBalloonTimeout` ／
  `OnBalloonBreak` の SHIORI 発火」。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。バルーンが閉じた・時間切れになった・中断されたことは SHIORI へ伝わらず、
  それに応じて喋る辞書はその場面で呼ばれない。
  ログ: ベースウェアが場面に応じて発火する系統では 1 行も出ない。理由は群 2 と同じ（この名前を
  組み立てる構築関数が無い。ゴーストが \q の選択肢 ID に書いた場合だけ選択起源の経路で送出され得る
  のも群 2 と同じで、そのときも状態は変わらない）。加えてこの群には第 2 の理由がある——表示側から
  会話進行側へ渡す受け皿の型は用意してあるが、それを作る側も受け取る側もまだ存在しないため、
  バルーンの開閉を起点とする経路そのものが動かない。
  縮退の転記元: doc/COMPAT_ARCHITECTURE.md の沈黙ルール対応表の行「OnBalloonClose ／
  OnBalloonTimeout ／ OnBalloonBreak の SHIORI 発火」。そこには M1 は発火させず、語彙・Reference の
  割り当て・受け渡し口の型だけを残すと記録されている。解禁の条件は M2 で互換の範囲を広げるときで、
  追跡先は areka-P0-balloon-canon-residue。
  根拠の場所: crates/areka/src/emo2_boot/talk_lifecycle.rs の BalloonLifecycleNotice。
  ```

---

### 群 3 — 旧仕様の別名

- **対象**: `OnFileDrop`・`OnFileDropped`・`OnFileDropEx`（いずれも正典本文が「旧仕様」と明記）。
  写像先は `OnFileDrop2`。
- **件数**: 3
- **状態**: `alias`
- **テーマ**: 空（`[]`）
- **優先度**: `""`
- **判断の根拠の場所**: 判定はこの行では行わない。実装しているかどうかは写像先
  `ukadoc:list_shiori_event:OnFileDrop2:1` の行が持つ（その行の根拠の場所は群 2 と同じ）。
- **共通 `note`**:

  ```
  壊れ方: この行では判定しない。別名の行が持つのは正典側の項目への写像だけで、実装しているか
  どうかは写像先（OnFileDrop2）の行に書く。実際の壊れ方は写像先の行に従う。
  ログ: この行に固有のログは無い。
  向き: 正典本文がこの 3 つを「旧仕様」で始めて説明しているため、OnFileDrop2 を正典側とする。
  OnFileDropping はドラッグ中の通知であって別の機能なので、この一群に含めない。
  ```

---

### 群 4 — `list_shiori_event` に同居する `On` 以外

- **対象**: イベント一覧のページに同居している、名前が `On` で始まらない項目
  （`property.get`／`property.set`・`hwnd`・`uniqueid`・`capability`・`installed*`・`*pathlist`・
  `enable_log` ほか）。同居する 26 件のうち `basewareversion` は送っているので群 1 に置き、
  残りがこの群。
- **件数**: 25
- **状態**: `absent`
- **テーマ**: 原則 空（`[]`）
- **優先度**: `D2`（仮置き）
- **判断の根拠の場所**: 知らせる側は `crates/areka-kanade/src/schedule/events.rs` の
  `ALLOWED_EVENT_IDS`、引く側は `crates/areka-kanade/src/schedule/resources.rs` の
  `ALLOWED_RESOURCE_IDS`（`username` 1 件だけ）。`property.get`／`property.set` の担当は
  `areka-P0-property-query-channels`。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。ベースウェアから知らせる側の項目はその知らせが届かず、ベースウェアが
  引く側の項目は値を引きに行かないので、ゴーストが用意した答えは使われない。どちらも例外にならず、
  「その情報が無いまま進む」という形でだけ現れる。
  ログ: 1 行も出ない。知らせる側は群 2 と同じで、この名前を組み立てる場所がコードのどこにも無く、
  送出を止める判定に到達しない。群 2 が但し書きにしている選択起源の経路も、この群には及ばない——
  受理規則が「On で始まること」であるのに対し、この群の名前はどれも On で始まらないためである
  （schedule/choice.rs の plan_cascade は On で始まらない選択肢 ID を正典形の OnChoiceSelectEx へ
  回すので、この群の名前が逐語のまま送出されることはない）。引く側も、引ける名前は username 1 つ
  だけを載せた固定の表で決まっており、表に無い名前を引きに行く場所が無い。
  向き: この行が「ベースウェアから知らせる側」か「ベースウェアが SHIORI から引く側」かは項目ごとに
  書き分ける。
  ```

---

### 群 5 — 外部が送る拡張イベント

- **対象**: `list_shiori_event_ex` の全項目。送信元は areka ではなく、外部のアプリ・プラグイン・
  他のゴーストである。
- **件数**: 168
- **状態**: `not-applicable`
- **テーマ**: 空（`[]`）
- **優先度**: `""`
- **判断の根拠の場所**: 任意の名前のイベントを外から受けて SHIORI へ渡す経路が areka に無いこと
  （`\![raiseplugin]`・`\![notifyplugin]` を受ける場所は `crates/` 以下に 1 件も無い）。ゴースト間の
  やり取りとの繋がりは `ukadoc:list_shiori_event:OnCommunicate:1` へ `same-feature` で結ぶ。
- **共通 `note`**:

  ```
  壊れ方: areka の側では起きない。送信元は areka ではなく外部のアプリ・プラグイン・他のゴースト
  であり、areka に問われるのは受け口があるかどうかだけである。その受け口（任意の名前のイベントを
  外から受けて SHIORI へ渡す経路）は areka に無い。
  ログ: 外から届く経路では 1 行も出ない。受け口が無いということは、外から届いた要求を読む場所が
  無いということなので、届かなかったことを書き留める場所も無い。なお、この群の名前のうち On で
  始まるものは、ゴーストが \q の選択肢 ID に書けば選択起源の経路（索引の前置きの⑵）で逐語のまま
  送出され得るが、それは外部からの依頼が届いたことを意味しないので、この行の状態は
  not-applicable のまま変わらない。
  テーマ: 空にする。送信元が areka でない以上、「これが無いと利用者はゴーストの何を失うか」に
  areka の側から答えられないため。
  版番号: このページの項目は本文に版番号を含まないので introduced は空にする。
  ```

  ゴースト同士のやり取りが本文に書いてある 7 件（`OnRequestValues`・`OnGetValues`・可変名の返信
  イベント・`Send60stair_GetStatus`・`OnKanadeTeaPartyInfomationRequest`・`OnPoker`・`OnMahjong`）
  には、上の文面に次の 1 文を足す。

  ```
  この項目の送信元は外部のアプリではなく他のゴーストで、ベースウェアがその伝達を運ぶ。areka には
  ゴースト間の伝達そのものが無いため、運ぶ側としても成立しない。
  ```

  さらに、この 7 件には次の「根拠の場所」の段落も足す。7 行とも同一の文面であり、実質この
  小さな群の共通の文面である。

  ```
  根拠の場所: crates/ 以下に raiseother・notifyother・raiseplugin・notifyplugin のいずれかを受ける
  場所も送る場所も 1 件も無く、他のゴーストを名指しで指す先を引く表（起動中のゴーストの登記）も
  無い。SHIORI へ渡す口は crates/areka-kanade/src/actor.rs の round_trip_request の 1 か所だけで、
  そこへ渡る呼び出しの出所は索引の前置きの⑴と⑵の 2 系統しかなく、どちらも他のゴーストからの
  依頼を入口としない。
  ```

  **上の 1 文についての補足（2026-09-05・実装時に正典で確認）**: 「送信元は外部のアプリではなく
  他のゴースト」という言い切りは正確ではない。`OnMahjong` の正典本文は、要求元がゴーストのときの
  返し方と並べて、**要求元が外部アプリで SSTP による通知であった場合の返し方**（`X-SSTP-PassThru-*`
  ヘッダを使う）も述べている。この 7 件は「ゴースト同士でやり取りすることが本文に書いてある」
  ものであって、送信元が他のゴーストに限られるという意味ではない。**7 という数も確定値として
  扱わないこと**——返信の相手側（`OnMahjongResponse`）まで同じ性質と見るなら 7 では足りない。
  台帳の 7 行は上の文面のまま凍結してあり、この補足はそれを読むときの注意である。

---

### 群 6 — 実際に引いているリソース

- **対象**: `username`。
- **件数**: 1
- **状態**: `implemented`
- **テーマ**: 項目ごと
- **優先度**: `""`
- **判断の根拠の場所**: `crates/areka-kanade/src/schedule/resources.rs` の
  `ALLOWED_RESOURCE_IDS` と `resource_username`、送出は `crates/areka-kanade/src/actor.rs` の
  `round_trip_request`。既定値の唯一の定義点は `areka_sakura::sysvar::DEFAULT_USERNAME`。
- **共通 `note`**:

  ```
  壊れ方: 該当なし。areka は起動の途中でこのリソースを実際に引いている。
  ログ: 引く直前に trace のログ（shiori_request）が 1 行出る。204 や空の応答・照会の失敗でも起動は
  止めず、値が無いものとして先へ進む。
  縮退の転記元: doc/COMPAT_ARCHITECTURE.md の沈黙ルール対応表の行「%username の SHIORI Resource
  username GET が 204 No Content／空値を応答した場合の値」。照会そのものは行われるが、値が無いときは
  既定値へ決定論的に縮退する。既定値の唯一の定義点は areka_sakura::sysvar::DEFAULT_USERNAME で、
  kanade は不在をそのまま渡すだけで既定値を書かない。
  根拠の場所: crates/areka-kanade/src/schedule/resources.rs の ALLOWED_RESOURCE_IDS と
  resource_username、および crates/areka-kanade/src/actor.rs の round_trip_request。
  ```

---

### 群 7 — 語彙だけあるリソース・画面の材料

- **対象**: メニューやゴースト管理の画面が材料として使うリソース（各ボタンの文言 99・`menu.*` 18・
  `popupmenu.*` 9・`*.recommendsites` と `*.portalsites` 4・`vanishbuttonvisible` 1）。
- **件数**: 131
- **状態**: `vocabulary-only`
- **テーマ**: 装い（台帳の実測では 131 行すべてこの 1 つ）
- **優先度**: `C1`（仮置き。テーマの上では B に当たるが、壊れ方が見た目の差なので 1 段下げる）
- **判断の根拠の場所**: `crates/areka-sylphya/src/vocab/shiori_resource.rs` の
  `SHIORI_RESOURCE_IDS`（名前は登記済み。この表を参照しているのはテストだけ）と
  `crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`（引く名前は
  `username` だけ）。
- **共通 `note`**:

  ```
  壊れ方: 見た目の差。名前は語彙として登記してあるが areka は引きに行かないので、ゴーストが用意した
  文言・画像・色は使われず、areka の既定の見た目のままになる。動かなくなるのではなく、そのゴースト
  らしい見え方が出ないという形で現れる。
  ログ: 1 行も出ない。語彙の表は名前を並べてあるだけで、実行時にそれを読んで照会を組み立てる場所が
  無い（この表を読んでいるのはテストだけである）。引ける名前を決める表は username 1 つしか載せて
  いないので、引かれなかったことを書き留める判定にも届かない。
  根拠の場所: crates/areka-sylphya/src/vocab/shiori_resource.rs の SHIORI_RESOURCE_IDS に名前は
  載っているが、crates/areka-kanade/src/schedule/resources.rs の ALLOWED_RESOURCE_IDS には無い。
  ```

---

### 群 8 — 語彙だけあるリソース・その他

- **対象**: `list_shiori_resource` のうち群 6・群 7 に入らないもの（`sakura.*`／`kero.*`／`char*.*`
  のスコープ違いの値、`homeurl`・`version` ほか）。
- **件数**: 27
- **状態**: `vocabulary-only`
- **テーマ**: 項目ごと
- **優先度**: 項目のテーマから決まる（テーマが付かないものは `D2`・仮置き）
- **判断の根拠の場所**: 群 7 と同じ 2 か所
  （`crates/areka-sylphya/src/vocab/shiori_resource.rs` の `SHIORI_RESOURCE_IDS` と
  `crates/areka-kanade/src/schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`）。
  `sakura.*`／`kero.*`／`char*.*` は新旧の関係ではなく、本体側・相方側・2 人目以降または `\p[*]` 側
  というスコープの違いなので、別名にせず `same-feature` で結ぶ。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。名前は語彙として登記してあるが areka は引きに行かないので、この値を前提に
  した動きはその場面で何も起きない。既定の見え方に置き換わるわけでもないため、利用者からは何も
  起きていないようにしか見えない。
  ログ: 1 行も出ない。語彙の表は名前を並べてあるだけで、実行時にそれを読んで照会を組み立てる場所が
  無い（この表を読んでいるのはテストだけである）。引ける名前を決める表は username 1 つしか載せて
  いないので、引かれなかったことを書き留める判定にも届かない。
  根拠の場所: crates/areka-sylphya/src/vocab/shiori_resource.rs の SHIORI_RESOURCE_IDS に名前は
  載っているが、crates/areka-kanade/src/schedule/resources.rs の ALLOWED_RESOURCE_IDS には無い。
  ```

---

### 群 9 — 送っているヘッダ

- **対象**: `spec_shiori3` のリクエスト側のうち areka が毎回組み立てて送るもの（要求行＝メソッド・
  `Sender`・`Status`・`ID`・`Reference*`）。
- **件数**: 5
- **状態**: `implemented`
- **テーマ**: 空（`[]`）
- **優先度**: `""`
- **判断の根拠の場所**: `crates/shiori-host32-host/src/shiori3.rs` の `build_request`
  （ヘッダを書き出す文が並ぶ）。`Status` の実行状態の語彙は
  `areka-P0-status-execution-states` が所有する。
- **共通 `note`**:

  ```
  壊れ方: 該当なし。areka はこのヘッダを毎回組み立てて送っている。
  ログ: 送出の直前に kanade が trace のログ（shiori_request）で送る内容を残す。組み立てが途中で
  失敗する経路は無い（値をそのまま文字列へ書き出すだけである）。
  根拠の場所: crates/shiori-host32-host/src/shiori3.rs の build_request。
  ```

---

### 群 10 — 固定値で送っているヘッダ

- **対象**: リクエスト側の `Charset`（UTF-8 のみ）と `SecurityLevel`（`local` のみ）。
- **件数**: 2
- **状態**: `degraded`
- **テーマ**: 空（`[]`）
- **優先度**: `D2`（仮置き）
- **判断の根拠の場所**: `crates/shiori-host32-host/src/shiori3.rs` の `build_request`。
  `Charset` の担当は `areka-P0-charset-canon`。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。ヘッダは送っているが値が 1 つに固定してあり、正典が許す他の値を選べない。
  Charset は UTF-8 だけなので、Shift_JIS で書かれた既存ゴーストとは噛み合わない。SecurityLevel は
  local だけなので、外部由来の呼び出しと区別できない。
  ログ: 固定値であること自体を知らせるログは 1 行も出ない。値は分岐せずそのまま書き出されるので、
  「選べなかった」という判断がコードの中に存在しない。文字の変換に失敗したときだけ、応答を読む側が
  読み取り失敗として扱い、そこで初めて明示的なエラーになる。
  根拠の場所: crates/shiori-host32-host/src/shiori3.rs の build_request（Charset は 1 値のみを持つ
  型から書き出し、SecurityLevel は local の文字列を直接書き出す）。
  ```

---

### 群 11 — 送らない／読み飛ばすヘッダ

- **対象**: リクエスト側で送らない 4 つ（`SenderType`・`SecurityOrigin`・`BaseID`・リクエスト側の
  `X-SSTP-PassThru-`）と、レスポンス側で読み飛ばす 11（レスポンス側の `Charset`・`Sender`・
  `SecurityLevel`・`X-SSTP-PassThru-`、`ValueNotify`・`Marker`・`BalloonOffset`・`Reference0`・
  `Reference1〜`・`Age`・`MarkerSend`）。
- **件数**: 15
- **状態**: `absent`
- **テーマ**: 空（`[]`）
- **優先度**: `D2`（仮置き）
- **判断の根拠の場所**: `crates/shiori-host32-host/src/shiori3.rs` の `build_request` と
  `parse_response`。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。リクエスト側の 4 つは組み立てに現れないので相手に届かず、レスポンス側の 11 は
  届いても読まずに捨てるので、ゴーストが返した指示は無かったことになる。
  ログ: 1 行も出ない。応答のヘッダを読む繰り返しは Value・ErrorLevel・ErrorDescription の 3 つに
  名前が一致したときだけ値を取り、一致しなかったときに何かをする枝を持たない（そのまま次の行へ進む）。
  リクエスト側も、書き出す文が並ぶだけで「書かなかった」を記録する場所が無い。
  根拠の場所: crates/shiori-host32-host/src/shiori3.rs の build_request と parse_response（ヘッダ名を
  大文字小文字を無視して比べる 3 つの分岐）。build_request の説明が挙げる「送らないもの」は
  SenderType・SecurityOrigin・X-SSTP-PassThru の 3 つで、BaseID は挙がっていない（BaseID は
  crates 以下のソースに 1 件も無い）。
  ```

  レスポンス側の `X-SSTP-PassThru-` の行には、上の文面に次の 1 文を足す。

  ```
  廃止予定と明記された旧名 X-SSTP-Return- の注記は、正典ではこのレスポンス側の項目に付いている。
  ```

---

### 群 12 — 解釈している応答

- **対象**: ステータスコード・`Value`・`ErrorLevel`・`ErrorDescription`。
- **件数**: 4
- **状態**: `implemented`
- **テーマ**: 空（`[]`）
- **優先度**: `""`
- **判断の根拠の場所**: `crates/shiori-host32-host/src/shiori3.rs` の `parse_response` と
  `parse_status_code`、および `crates/shiori-host32-host/src/client.rs` の `map_get_result`。
- **共通 `note`**:

  ```
  壊れ方: 該当なし。areka はステータスコードと Value・ErrorLevel・ErrorDescription を読み取って
  使っている。
  ログ: 応答が 400・500 か ErrorLevel を伴うときは黙って捨てず、SHIORI のエラーとして呼び出し元へ
  返り、kanade が error のログを残す（明示的なエラー）。応答が UTF-8 として読めないときや、
  ステータス行から数値が取れないときも同じく読み取り失敗として返る。
  根拠の場所: crates/shiori-host32-host/src/shiori3.rs の parse_response（ステータス行の取り出しと
  3 つのヘッダの分岐）と crates/shiori-host32-host/src/client.rs の map_get_result。
  ```

---

### 群 13 — PLUGIN の受け口

- **対象**: `list_plugin_event` の全項目（プラグイン向けのイベント・プラグイン向けのリソース・
  プロパティの照会・任意の名前のイベントの枠）。
- **件数**: 19
- **状態**: `absent`
- **テーマ**: 空（`[]`）
- **優先度**: `E1`（仮置き）
- **判断の根拠の場所**: プラグインを読み込む仕組みが `crates/` 以下に 1 件も無いこと
  （`\![raiseplugin]`・`\![notifyplugin]` を受ける場所も 0 件）。同じ名前が
  `list_shiori_event` にもある 12 組は `same-feature` で結ぶ。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。areka にはプラグインを読み込む仕組みが無く、プラグイン向けのイベントを送る
  先も、プラグインからの照会を受ける口も無い。プラグインを入れても何も起こらない。
  ログ: 1 行も出ない。受け口が無いということは要求を読む場所が無いということなので、断ったことを
  書き留める場所も無い。送る側についても、この名前を組み立てる構築関数がコードのどこにも無い。
  この群の名前のうち On で始まるものが選択起源の経路（索引の前置きの⑵）で逐語のまま送出され得る
  のは群 2 と同じだが、それはプラグインへ届けたことを意味しないので、この行の状態は absent のまま
  変わらない。
  種別: この行がイベントなのか・PLUGIN 向けのリソースなのか・プロパティの照会なのか・任意の名前の
  イベントの枠なのかは項目ごとに書き分ける。
  ```

---

### 群 14a — 外部連携のうちゴースト同士の交わり

- **対象**: `spec_sstp` 2 と `spec_fmo_mutex` 6。
- **件数**: 8
- **状態**: `absent`
- **テーマ**: 交わり
- **優先度**: `D1`（仮置き）
- **判断の根拠の場所**: SSTP の待ち受けと FMO の読み書きが `crates/` 以下に 1 か所も無いこと
  （バルーンの `sstpmessage` で始まるキーは未知の設定として読み飛ばされるだけで、SSTP の実装では
  ない）。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。SSTP の受け付け口も、起動中のゴーストの一覧を共有する仕組みも areka に
  無いので、他のゴーストや外部のアプリから話しかけることができず、他のゴーストが居ることも分からない。
  利用者からは「うちのゴーストたちが互いに気づかない」という形でだけ現れる。
  ログ: 1 行も出ない。待ち受ける口が開いていないので接続そのものが成立せず、断ったことを書き留める
  場所が無い。共有の仕組みについても、それを読み書きする場所がコードのどこにも無い。
  ```

---

### 群 14b — 外部連携のその他

- **対象**: `spec_web` 3（ブラウザから渡す `x-ukagaka-link`）・`spec_plugin` 1・`spec_headline` 1。
- **件数**: 5
- **状態**: `absent`
- **テーマ**: 空（`[]`）
- **優先度**: `E1`（仮置き）
- **判断の根拠の場所**: ブラウザからの受け口・プラグインの決まりに沿った読み込み・ヘッドラインの
  取得がいずれも `crates/` 以下に無いこと。HEADLINE と PLUGIN について areka にあるのは sylphya の
  根の名前（`crates/areka-sylphya/src/vocab/dotted.rs`）とボタンの文言の語彙だけ。
- **共通 `note`**:

  ```
  壊れ方: 黙って壊れる。ブラウザから渡される導入やイベントの依頼を受け取る口も、プラグインの決まりに
  沿った読み込みも、ヘッドラインの取得も areka に無い。配布サイトの導入用のリンクを押しても areka には
  何も届かない。
  ログ: 1 行も出ない。受け口が無いということは要求を読む場所が無いということなので、断ったことを
  書き留める場所も無い。
  テーマ: HEADLINE には付けない。付与規則に照らして「これが無いと利用者はゴーストの何を失うか」に
  答えられる段階に無いため。
  ```

---

### 群 14c — DLL 共通仕様

- **対象**: `ukadoc:spec_dll`（DLL の入口の決まり。SHIORI・SAORI・MAKOTO・PLUGIN が共有する）。
  正典が定める入口は **4 つ**——初期化の `loadu`（SSP 2.6.92 で加わった側。置き場所のパスを UTF-8 で
  受け取り、ベースウェアはこちらを優先して使う）と `load`（従来版。同じパスを既定の各国語の
  コードページで受け取り、`loadu` が無いときのフォールバックになる）、終了処理の `unload`、
  要求の `request` である。
- **件数**: 1
- **状態**: `degraded`
- **テーマ**: 空（`[]`）
- **優先度**: `E1`（仮置き）
- **判断の根拠の場所**: `crates/shiori-host32-helper/src/shiori_proxy.rs` の `ShioriByteProxy::load`
  （絶対パスで読み込み、`load`・`unload`・`request` の 3 つを名前で引く。正典の 4 つのうち `loadu`
  は引かない）と `ShioriByteProxy::request`。SAORI・MAKOTO の語は
  `crates/` 以下のソースに 1 件も無い。MAKOTO の担当は `areka-P0-makoto-dll-host`。
- **共通 `note`**:

  ```
  壊れ方: できている範囲は明示的なエラーで守られており、できていない範囲は黙って壊れる。正典が定める
  DLL の入口は、生存期間の loadu（SSP 2.6.92 以降・置き場所のパスを UTF-8 で受け取る。ベースウェアは
  こちらを優先する）と load（従来版・同じパスを既定の各国語コードページで受け取る）と unload、そして
  要求の request である。SHIORI の DLL については areka の 32bit の助け手が絶対パスで読み込み、この
  うち load・unload・request の 3 つを名前で引いてから呼んでいる（loadu は引かない）。同じ決まりを
  共有する SAORI・MAKOTO・PLUGIN の DLL は、読み込む呼び出しそのものが無いので同居できない。
  ログ: DLL が開けない・3 つの入口のいずれかが引けない・load が偽を返したときは、いずれも失敗として
  呼び出し元へ返り、起動が黙って先へ進むことはない。一方 SAORI・MAKOTO・PLUGIN については、読み込もう
  とする場所が無いのでログは 1 行も出ない。
  SAORI: areka はプロトコルを実装せず、実装の主体は SHIORI 側である。成立の条件は 32bit の同じ
  プロセスに同居すること・作業ディレクトリ・DLL の探索パスの 3 つ。ukadoc に SAORI の独立したページは
  無いので、SAORI 用の行は作らない。MAKOTO は areka-P0-makoto-dll-host が担う。
  縮退の転記元: doc/emo2-conformance-scope.md の旧ロードマップ spec への影響の表の行
  「areka-P0-shiori-host-32 ＝ SAORI 同居を M1 から削除（emo2 未使用）。32bit SHIORI 往復に集中」。
  粒度: このページ全体で 1 項目であり、他のページの 1 項目より粗い。
  根拠の場所: crates/shiori-host32-helper/src/shiori_proxy.rs の ShioriByteProxy::load と
  ShioriByteProxy::request。
  ```

  **上の共通 `note` を書き直した（2026-09-06・タスク 7.3）**: 以前の文面は「DLL の入口の決まり
  （load・unload・request の 3 つ）」と書いており、**正典が定める入口の数を 3 つと述べる形**に
  なっていた。これは事実として誤りである。正典（`spec_dll` の「ライフサイクル関数」の節と
  「request関数」の節）が定める入口は、`loadu`・`load`・`unload` の 3 つの生存期間の関数と、
  要求の `request` である。「3 つ」は **areka の助け手が名前で引いている入口の数**でしかない。
  正典を引き直したうえで上の共通 `note` を書き直し、台帳の該当行 1 件へも同じ文面を写した。
  同じ行の別の段落（「内容:」で始まる段落）が以前から 4 つの入口と両者の違いを書いており、
  今回の書き直しで冒頭の段落もそれと揃った。

---

### 群 15 — イベント一覧の補足

- **対象**: `ukadoc:memo_shiorievent`（イベント一覧のページに添えられた読み方の補足）。
- **件数**: 1
- **状態**: `not-applicable`
- **テーマ**: 空（`[]`）
- **優先度**: `""`
- **判断の根拠の場所**: 対応する送出も照会も無い。areka の内部でだけ使う名前（`OnTalk`・`OnHour`・
  `OnMenuBack`）の扱いは、正典に対応する項目が無いので行を作らず、
  `crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS` の説明にある恒久的に含めない
  旨を最も近い項目の `note` に写す。
- **共通 `note`**:

  ```
  壊れ方: 該当なし。この項目はイベント一覧のページに添えられた読み方の補足であって、areka が送る・
  引く・受けるものではない。実装の対象にならないので、利用者から見える壊れ方も無い。
  ログ: 実行時の経路を持たないので、出るログも出ないログも無い。
  粒度: このページ全体で 1 項目であり、他のページの 1 項目より粗い。
  ```

---

### 索引の検算

11＋248＋3＋3＋25 ＝ 290（`list_shiori_event`）／168（`list_shiori_event_ex`）／
1＋131＋27 ＝ 159（`list_shiori_resource`）／5＋2＋15＋4 ＝ 26（`spec_shiori3`）／
19（`list_plugin_event`）／8＋5＋1 ＝ 14（外部連携の 6 ページ）／1（`memo_shiorievent`）
＝ **677**。

---

## 本文の読み方

ここから先が群ごとの本文である。章は台帳の群にそのまま対応し、18 の群がすべて 1 章ずつ現れる。
各章に書くのは次の 3 つで、順序は章ごとに同じである。

1. **利用者に何が起きるか（何を失うか）** — 画面の前の人から見える結果で書く。
2. **その群を成立させる最小の基盤** — 「今ある物」と「足りない物」を分けて書く。どう作るか
   （設計・工程・見積り）は書かない。
3. **台帳の項目 id** — その群に属する行の id。数が多い群は id の形と名前の並びで示す。

**章に共通する「今ある物」**（章ごとに繰り返さないため、ここに一度だけ置く）

- SHIORI との往復の口が 1 か所ある: `crates/areka-kanade/src/actor.rs` の `round_trip_request`。
- 送るイベントの名前を決める固定の表と、呼び出しを組み立てる構築関数が
  `crates/areka-kanade/src/schedule/events.rs` にある（表に載る名前は 11）。
- 引くリソースの名前を決める固定の表が `crates/areka-kanade/src/schedule/resources.rs` にある
  （載っている名前は `username` の 1 つ）。
- 正典のリソース 159 件の名前は `crates/areka-sylphya/src/vocab/shiori_resource.rs` に登記済み。
  ただしこの表を読んでいるのはテストだけである。
- リクエストのヘッダを組み立てる場所と、応答を読む場所が
  `crates/shiori-host32-host/src/shiori3.rs` にある（`build_request`・`parse_response`）。
- 32bit の SHIORI DLL を読み込んで呼ぶ助け手が
  `crates/shiori-host32-helper/src/shiori_proxy.rs` にある（`ShioriByteProxy`）。
- ゴーストが `\q` の選択肢 ID に `On` で始まる名前を書けば、その名前は逐語のまま送出される
  （索引の前置きの⑵）。ベースウェアが場面を見て自分から発火するのとは別の経路である。

---

## 第 1 部 — 黙って壊れるもの

例外にもログにも現れず、「その場面で何も起きない」という形でだけ現れる群を先に置く。利用者からは
不具合に見えず、開発者からも欠けていることが見えない。この部の 6 章で 320 項目を占める。

### 群 2 — 送出していないイベント 248 件

**利用者に何が起きるか**

既存ゴーストの辞書は「SSP がこの場面でこのイベントを送ってくる」ことを前提に書かれている。areka は
その 248 件を送らないので、辞書に用意された返事は 1 度も呼ばれない。画面の上では、次のような形で
現れる。

- **時刻と起動終了まわり（8 件・仮の値 `A1`・テーマ「気配」）** — 時報や分の変わり目で何も言わない。
  画面の外へ出た・他の窓に重なったといった状況にも反応しない。
- **触れ合い（35 件・`A2`・テーマ「触れ合い」）** — 押す・離す・掴んで動かす・撫でる向きを変える・
  ホイールを回す・ファイルや文字列や URL を落とす、といった働きかけのほとんどが伝わらない。
  areka が送っているのは動かしたときと 2 度押したときの 2 つだけである。
- **掛け合い（13 件・`A3`・テーマ「掛け合い」「記憶」）** — 選択肢に触れただけの反応・アンカーを
  押した反応・入力欄への記入・教え込みの一連が伝わらない。選択肢を「選んだ」ことは伝わるので、
  会話は成り立つが、その周りの間や仕込みが失われる。
- **更新（26 件・`B1`・テーマ「更新」）** — ネットワーク越しの更新の一部始終（始まり・照合・完了・
  失敗・ゴースト以外の更新）が伝わらない。ゴーストが更新の進み具合を喋る仕掛けは何も動かない。
- **ファイルの受け渡し（2 件・`B2`）** — ドラッグ中の通知と、現行仕様の受け取りの通知。
  旧仕様の別名 3 件（群 3）の写像先もここに含まれる。
- **消滅（6 件・`B3`・テーマ「記憶」）** — 消す前の確認・取り消し・見送りが成立しない。
- **見た目の変化（13 件・`B4`・テーマ「装い」）** — シェルやバルーンや着せ替えを替えたことを
  ゴースト自身が知らないので、着替えたときの台詞が出ない。
- **導入と配布（9 件・`B5`・テーマ「記憶」）** — 導入の始まり・完了・失敗・断りが伝わらない。
- **察し（46 件・`C2`・テーマ「気配り」）** — 電池・ネットワーク・画面の切り替え・全画面のアプリ・
  スリープと復帰・画面の鍵・音楽の再生といった周囲の様子を一切知らない。常駐させておく値打ちを
  作っている部分がまるごと欠ける。
- **交わり（23 件・`D1`・テーマ「交わり」「記憶」）** — 他のゴーストの起動・終了・着替え、呼び出し、
  伝達の受け渡しが成立しない。ゴーストを取り替えたことも伝わらない。
- **テーマの付かない配管（67 件・`D2`）** — 通信（HTTP・RSS・WebSocket・名前解決・時刻合わせ）、
  書庫の展開、予定表、音の再生、範囲選択の開始と終了、システムの問い合わせなど。単体では利用者に
  見えないが、これらを使う辞書は返事を受け取れないまま止まる。

**その群を成立させる最小の基盤**

- **今ある物**: 送る口・許可の表・構築関数を置く場所・参照値を積んで渡す仕組み（既に 11 件が
  同じ道を通っている）。イベントを 1 件足すのに新しい伝送路は要らない。
- **足りない物**: **発火の条件を観測する場所**である。許可の表に名前を書き足しても送出は 1 件も
  増えない——呼び出しを組み立てる場所が無いからで、送出を止める判定にすら到達しない。観測の元手は
  小群ごとに違い、入力の通知・電源や画面やセッションの通知・更新の進行・起動中のゴーストの一覧
  （群 14a）・プラグインの受け口（群 13）などがそれぞれ要る。つまりこの 248 件は 1 つの土台では
  埋まらず、小群ごとに別々の観測元を持つ。

**台帳の項目 id**

id は `ukadoc:list_shiori_event:<名前>:1` の形。名前は次の 248 件（優先度の仮置きの値とテーマで
まとめた。値は台帳の `priority`・`values` と一致する）。

- `A1`／気配（8）: OnAITalk, OnCacheRestore, OnCacheSuspend, OnCloseAll, OnHourTimeSignal,
  OnMinuteChange, OnOffscreen, OnOverlap
- `A2`／触れ合い（35）: OnArchiveViewerOpen, OnDirectoryDrop, OnGamepadAxisMove,
  OnGamepadButtonDown, OnGamepadButtonUp, OnKeyPress, OnMediaPlayerOpen, OnMouseClick,
  OnMouseClickEx, OnMouseDoubleClickEx, OnMouseDown, OnMouseDownEx, OnMouseDragEnd,
  OnMouseDragStart, OnMouseEnter, OnMouseEnterAll, OnMouseGesture, OnMouseHover, OnMouseLeave,
  OnMouseLeaveAll, OnMouseMultipleClick, OnMouseMultipleClickEx, OnMouseUp, OnMouseUpEx,
  OnMouseWheel, OnOtherObjectDropped, OnOtherObjectDropping, OnPictureViewerOpen, OnTextDrop,
  OnURLDragDropping, OnURLDropFailure, OnURLDropped, OnURLDropping, OnURLQuery, OnWallpaperChange
- `A3`／掛け合い（12）: OnAnchorEnter, OnAnchorHover, OnAnchorSelect, OnAnchorSelectEx,
  OnChoiceEnter, OnChoiceHover, OnTeach, OnTeachInputCancel, OnTeachStart, OnTranslate,
  OnUserInput, OnUserInputCancel
- `A3`／記憶（1）: OnNotifyUserInfo
- `B1`／更新（26）: OnUpdate.OnDownloadBegin, OnUpdate.OnMD5CompareBegin,
  OnUpdate.OnMD5CompareComplete, OnUpdate.OnMD5CompareFailure, OnUpdateBegin,
  OnUpdateCheckComplete, OnUpdateCheckFailure, OnUpdateCheckResult, OnUpdateCheckResultEx,
  OnUpdateComplete, OnUpdateFailure, OnUpdateOther.OnDownloadBegin,
  OnUpdateOther.OnMD5CompareBegin, OnUpdateOther.OnMD5CompareComplete,
  OnUpdateOther.OnMD5CompareFailure, OnUpdateOtherBegin, OnUpdateOtherComplete,
  OnUpdateOtherFailure, OnUpdateOtherReady, OnUpdateProcessExec, OnUpdateReady, OnUpdateResult,
  OnUpdateResultEx, OnUpdateResultExplorer, OnUpdatedataCreated, OnUpdatedataCreating
- `B2`／触れ合い（2）: OnFileDrop2, OnFileDropping
- `B3`／記憶（6）: OnDestroy, OnVanishButtonHold, OnVanishCancel, OnVanishSelected,
  OnVanishSelecting, OnVanished
- `B4`／装い（13）: OnBalloonChange, OnBalloonScaling, OnDressupChanged, OnNotifyBalloonInfo,
  OnNotifyDressupInfo, OnNotifyFontInfo, OnNotifySelfInfo, OnNotifyShellInfo, OnShellChanged,
  OnShellChanging, OnShellScaling, OnSurfaceChange, OnSurfaceRestore
- `B5`／記憶（9）: OnInstallBegin, OnInstallComplete, OnInstallCompleteAll, OnInstallCompleteEx,
  OnInstallFailure, OnInstallRefuse, OnInstallReroute, OnNarCreated, OnNarCreating
- `C2`／気配り（46）: OnBatteryChargingStart, OnBatteryChargingStop, OnBatteryCritical,
  OnBatteryLow, OnBatteryNotify, OnCPULoadHigh, OnCPULoadLow, OnDarkTheme, OnDeviceArrival,
  OnDeviceRemove, OnDisplayChange, OnDisplayChangeEx, OnDisplayHandover, OnDisplayPowerStatus,
  OnFullScreenAppMinimize, OnFullScreenAppRestore, OnGamepadConnected, OnGamepadDisconnected,
  OnLanguageChange, OnMemoryLoadHigh, OnMemoryLoadLow, OnMusicPlay, OnMusicPlayEx, OnNetworkHeavy,
  OnNetworkStatusChange, OnNotifyInternationalInfo, OnNotifyOSInfo, OnOSUpdateInfo,
  OnRecycleBinEmpty, OnRecycleBinEmptyFromOther, OnRecycleBinStatusUpdate, OnScreenSaverEnd,
  OnScreenSaverStart, OnSessionDisconnect, OnSessionLock, OnSessionReconnect, OnSessionUnlock,
  OnSysResume, OnSysSuspend, OnTabletMode, OnTrayBalloonClick, OnTrayBalloonTimeout, OnVideoPlayEx,
  OnVirtualDesktopChanged, OnWindowStateMinimize, OnWindowStateRestore
- `D1`／交わり（21）: OnCommunicate, OnCommunicateInputCancel, OnEmbryoExist, OnGhostCallComplete,
  OnGhostCalled, OnGhostCalling, OnNekodorifExist, OnNotifyOtherFailure, OnOtherGhostBooted,
  OnOtherGhostChanged, OnOtherGhostClosed, OnOtherGhostTalk, OnOtherGhostVanished,
  OnOtherOffscreen, OnOtherOverlap, OnOtherSurfaceChange, OnRaiseOtherFailure, OnSSTPBlacklisting,
  OnSSTPBreak, OnVoiceRecognitionWord, OnXUkagakaLinkOpen
- `D1`／記憶（2）: OnGhostChanged, OnGhostChanging
- `D2`／テーマなし（67）: OnBIFF2Complete, OnBIFFBegin, OnBIFFComplete, OnBIFFFailure,
  OnBasewareUpdated, OnBasewareUpdating, OnCompressArchiveComplete, OnCompressArchiveFailure,
  OnConfigurationDialogHelp, OnExecuteHTTPComplete, OnExecuteHTTPFailure, OnExecuteHTTPProgress,
  OnExecuteHTTPSSLInfo, OnExecuteHTTPStreaming, OnExecuteRSSComplete, OnExecuteRSSFailure,
  OnExecuteRSS_SSLInfo, OnExecuteWebSocketClose, OnExecuteWebSocketFailure, OnExecuteWebSocketOpen,
  OnExecuteWebSocketReceive, OnExecuteWebSocketReconnect, OnExecuteWebSocket_SSLInfo,
  OnExtractArchiveComplete, OnExtractArchiveFailure, OnGhostTermsAccept, OnGhostTermsDecline,
  OnHeadlinesense.OnFind, OnHeadlinesenseBegin, OnHeadlinesenseComplete, OnHeadlinesenseFailure,
  OnNSLookupComplete, OnNSLookupFailure, OnNotifyPluginFailure, OnPingComplete, OnPingProgress,
  OnRSSBegin, OnRSSComplete, OnRSSFailure, OnRaisePluginFailure, OnRecommendsiteChoice,
  OnResetWindowPos, OnSNTPBegin, OnSNTPCompare, OnSNTPCompareEx, OnSNTPCorrect, OnSNTPCorrectEx,
  OnSNTPFailure, OnSchedule5MinutesToGo, OnScheduleRead, OnSchedulepostBegin,
  OnSchedulepostComplete, OnSchedulesenseBegin, OnSchedulesenseComplete, OnSchedulesenseFailure,
  OnSelectModeBegin, OnSelectModeCancel, OnSelectModeComplete, OnSelectModeMouseDown,
  OnSelectModeMouseUp, OnSoundError, OnSoundLoop, OnSoundStop, OnSpeechSynthesisStatus,
  OnSystemDialog, OnSystemDialogCancel, OnVoiceRecognitionStatus

---

### 群 2a — バルーンの開閉を知らせない 3 件

**利用者に何が起きるか**

吹き出しが閉じた・読まれないまま時間切れになった・途中で中断された、という出来事が SHIORI に
伝わらない。読み飛ばされたことに気づいて話し方を変える、といった気配りがまるごと働かない。

**その群を成立させる最小の基盤**

- **今ある物**: 表示側から会話進行側へ渡すための受け皿の型が
  `crates/areka/src/emo2_boot/talk_lifecycle.rs` に用意してある（`BalloonLifecycleNotice`）。
  イベントの名前と参照値の割り当ても決まっている。
- **足りない物**: その受け皿を作る側（吹き出しの開閉を見て通知を起こす側）と、受け取って SHIORI へ
  渡す側の両方。どちらも存在しないので、経路そのものが動かない。
- この 3 件は **M1 では意図的に発火させない**という記録があり（転記元は `doc/COMPAT_ARCHITECTURE.md`
  の沈黙ルール対応表）、追跡先は `areka-P0-balloon-canon-residue` である。他の群と違い、
  「気づいていない欠落」ではなく「先送りと記録されている欠落」である。

**台帳の項目 id**

`ukadoc:list_shiori_event:OnBalloonBreak:1`・`ukadoc:list_shiori_event:OnBalloonClose:1`・
`ukadoc:list_shiori_event:OnBalloonTimeout:1`

---

### 群 4 — イベント一覧に同居する `On` 以外の 25 件

**利用者に何が起きるか**

この群は 2 種類の混在である。ベースウェアからゴーストへ**知らせる**もの（導入済みのゴースト・
バルーン・シェルの名前、置き場所の一覧、窓の取っ手、固有の識別子など）と、ベースウェアが
ゴーストから**引く**もの（ゴーストが対応している機能の申告、記録の可否、入力欄の補完の設定、
プロパティの照会）である。どちらも届かないので、「他のゴーストを呼ぶ一覧を作る」「導入済みかどうかで
台詞を変える」「自分の窓を指して何かをする」といった仕掛けが成立しない。利用者からは、その機能が
最初から無いようにしか見えない。

**その群を成立させる最小の基盤**

- **今ある物**: 知らせる側には送る口と構築関数の置き場所があり、引く側には照会の口と固定の表がある
  （表に載る名前は `username` の 1 つ）。
- **足りない物**: 知らせる中身そのもの——導入済みのものを登記した一覧、置き場所の一覧、窓の取っ手を
  外へ渡す経路。引く側は、引ける名前を決める表に名前が無いこと。`property.get`／`property.set` の
  受け口は別の spec（`areka-P0-property-query-channels`）が担当する。

**台帳の項目 id**

id は `ukadoc:list_shiori_event:<名前>:1` の形。名前は次の 25 件（すべて `absent`・仮の値 `D2`）。

balloonpathlist, calendarpluginpathlist, calendarskinpathlist, capability, configuredbiffname,
enable_debug, enable_log, ghostpathlist, headlinepathlist, hwnd, inputbox.autocomplete,
installedballoonname, installedghostname, installedheadlinename, installedkeroname,
installedplugin, installedsakuraname, installedshellname, otherghostname, ownerghostname,
pluginpathlist, property.get, property.set, rateofusegraph, uniqueid

---

### 群 8 — 語彙だけあるリソース・その他 27 件

**利用者に何が起きるか**

ゴーストが用意した値を areka が引きに行かないので、その値を前提にした動きが何も起きない。立ち位置の
既定値（本体側・相方側・2 人目以降）、ゴーストの名前・版・作者名、ホームページの所在、機嫌の値、
吹き出しの補助表示などである。群 7 と違って既定の見た目に置き換わるわけでもないため、利用者からは
何も起きていないようにしか見えない。

**その群を成立させる最小の基盤**

- **今ある物**: 正典 159 件の名前は `crates/areka-sylphya/src/vocab/shiori_resource.rs` に登記済みで、
  照会の往復そのものも `username` で実際に動いている。
- **足りない物**: 引ける名前を決める表に名前が無いこと（載っているのは `username` だけ）と、引いた値を
  受け取る先——立ち位置なら窓の配置、名前や版なら管理の画面、機嫌なら会話の進行——との繋がり。
  `sakura.*`／`kero.*`／`char*.*` は新旧の関係ではなく相手の違い（本体側・相方側・2 人目以降または
  `\p[*]` 側）なので、別名として片付けることはできない。

**台帳の項目 id**

id は `ukadoc:list_shiori_resource:<名前>:1` の形。名前は次の 27 件。

- `A2`／触れ合い（1）: tooltip
- `A3`／掛け合い（1）: balloon_tooltip
- `B1`／更新（3）: homeurl, other_homeurl_override, useorigin1
- `B3`／記憶（2）: getaistate, getaistateex
- `D2`／テーマなし（20）: -,
  `_28_5165_529b_30dc_30c3_30af_30b9_7a2e_985e_29.defaultleft_20_28_5165_529b_30dc_30c3_30af_30b9_7a2e_985e_29.defaulttop`,
  char_2a.defaultleft, char_2a.defaulttop, char_2a.defaultx, char_2a.defaulty, craftman, craftmanw,
  kero.defaultleft, kero.defaulttop, kero.defaultx, kero.defaulty, legacyinterface, log_path, name,
  sakura.defaultleft, sakura.defaulttop, sakura.defaultx, sakura.defaulty, version

（`-` と、長い符号化された名前の 1 件は、正典の見出しがそのまま id になったものである。`char_2a` の
`_2a` は `*` を符号化した綴りで、`char*.` を指す。）

---

### 群 10 — 固定値で送っているヘッダ 2 件

**利用者に何が起きるか**

Shift_JIS で書かれた既存ゴーストと噛み合わない。areka は文字コードの申告を UTF-8 の 1 値でしか
送らないので、従来の文字コードで書かれた辞書は文字化けするか、応答を読み取れずに終わる。既存資産を
そのまま動かしたい利用者にとっては、最初の 1 歩で止まる種類の欠けである。もう 1 つの
`SecurityLevel` は `local` に固定してあり、外から来た呼び出しかどうかをゴーストが区別できない。

**その群を成立させる最小の基盤**

- **今ある物**: ヘッダを組み立てる場所と応答を読む場所が 1 か所ずつあり、値は実際に送られている。
- **足りない物**: 値を選ぶ余地そのもの。文字コードは 1 つの値しか持てない型から書き出しており、
  `SecurityLevel` は文字列を直接書き出している。選んだ結果を実際の文字の変換へ渡す繋がりも要る。
  文字コードの担当は `areka-P0-charset-canon` である。

**台帳の項目 id**

`ukadoc:spec_shiori3:Charset:1`・`ukadoc:spec_shiori3:SecurityLevel:1`（いずれも `degraded`・
仮の値 `D2`。末尾が `:1` なのはリクエスト側の見出しであることを表し、レスポンス側の同名の見出しは
`:2` として別の行になっている）

---

### 群 11 — 送らない／読み飛ばすヘッダ 15 件

**利用者に何が起きるか**

ゴーストが返事に添えた指示が無かったことになる。読み終わりの通知、目印、吹き出しの位置ずらし、
追加の参照値、返事の寿命、外部への返信のための転送——いずれも届いても読まずに捨てるので、
ゴースト作者が意図した細かい制御が効かない。リクエスト側の 4 つは相手に届かないので、ゴースト側が
呼び出しの素性を見て振る舞いを変えることもできない。

**その群を成立させる最小の基盤**

- **今ある物**: 応答のヘッダを 1 行ずつ読む繰り返しがあり、そこで 3 つの名前だけを拾っている。
  リクエスト側も、ヘッダを書き足す場所そのものはある。
- **足りない物**: 残りのヘッダ名を拾う枝と、拾った値の行き先（位置ずらしなら吹き出しの配置、
  目印なら会話の進行、寿命なら再生の制御）。名前を読むだけでは何も変わらないので、受け取り先の側が
  同時に要る。

**台帳の項目 id**（`ukadoc:spec_shiori3:` に続く綴り。長い綴りは正典の見出しをそのまま符号化したもの）

`Age_20_5bSSP_62e1_5f35_5d:1`, `BalloonOffset_20_5bSSP_62e1_5f35_5d:1`,
`BaseID_20_5bSSP_62e1_5f35_5d:1`, `Charset:2`, `MarkerSend_20_5bSSP_62e1_5f35_5d:1`,
`Marker_20_5bSSP_62e1_5f35_5d:1`, `Reference0:1`, `Reference1_7e:1`,
`SecurityLevel_20_5bSSP_62e1_5f35_5d:1`, `SecurityOrigin_20_5bSSP_62e1_5f35_5d:1`, `Sender:2`,
`SenderType_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1`, `ValueNotify_20_5bSSP_62e1_5f35_202.5.35_5d:1`,
`X-SSTP-PassThru-_28_4efb_610f_306e_6587_5b57_5217_29_20_5bSSP_202.5.03_7e_62e1_5f35_5d:1`,
`X-SSTP-PassThru-_28_4efb_610f_306e_6587_5b57_5217_29_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1`

---

## 第 2 部 — テーマは付いているが、現れ方が見た目の差にとどまるもの

この部は 1 章である。動かなくなるのではなく、そのゴーストらしい見え方が出ない、という形で現れる。

### 群 7 — 画面の材料 131 件

**利用者に何が起きるか**

メニューやゴースト管理の画面が、ゴーストの用意した文言・色・背景ではなく areka の既定のままになる。
作者が付けた各ボタンの呼び名、メニューの前景・背景・枠・区切りの色や画像、推奨サイトとポータルの
並び、消去のボタンを出すかどうか——これらが反映されない。台帳ではこの 131 行すべてにテーマ「装い」が
付いている。壊れ方が見た目の差にとどまるため、仮の値は他の「装い」の群より 1 段低い `C1` である。

**その群を成立させる最小の基盤**

- **今ある物**: 131 件を含む正典 159 件の名前はすべて登記済みで、照会の往復も `username` で動く。
- **足りない物**: 引ける名前を決める表に名前が無いことと、これらの値を読んで画面を組み立てる側との
  繋がり。なお、その画面そのものが areka にどこまであるかは本調査の担当範囲の外である。

**台帳の項目 id**

id は `ukadoc:list_shiori_resource:<名前>:1` の形。名前は次の 131 件（すべて `vocabulary-only`・
テーマ「装い」・仮の値 `C1`）。

activaterootbutton.caption, addressbarbutton.caption, aistatebutton.caption,
alignrootbutton.caption, alwaysstayontopbutton.caption, alwaystrayiconvisiblebutton.caption,
balloonhistorybutton.caption, balloonrootbutton.caption, biffallbutton.caption, biffbutton.caption,
calendarbutton.caption, callghosthistorybutton.caption, callghostrootbutton.caption,
callsstpsendboxbutton.caption, char_2a.popupmenu.applybindtoself, char_2a.popupmenu.type,
char_2a.popupmenu.visible, char_2a.recommendbuttoncaption, char_2a.recommendsites.caption,
char_2a.recommendsites, charsetbutton.caption, closeballoonbutton.caption, closebutton.caption,
collisionvisiblebutton.caption, configurationbutton.caption, configurationrootbutton.caption,
debugballoonbutton.caption, definedsurfaceonlybutton.caption, dictationbutton.caption,
dressuprootbutton.caption, duibutton.caption, enableballoonmovebutton.caption,
firststaffbutton.caption, ghostexplorerbutton.caption, ghosthistorybutton.caption,
ghostinstallbutton.caption, ghostrootbutton.caption, headlinesensehistorybutton.caption,
headlinesenserootbutton.caption, helpbutton.caption, hidebutton.caption, historyrootbutton.caption,
inforootbutton.caption, kero.popupmenu.applybindtoself, kero.popupmenu.type,
kero.popupmenu.visible, kero.recommendbuttoncaption, kero.recommendsites,
leavepassivebutton.caption, menu.background.bitmap.filename, menu.background.font.color.b,
menu.background.font.color.g, menu.background.font.color.r, menu.disable.font.color.b,
menu.disable.font.color.g, menu.disable.font.color.r, menu.foreground.bitmap.filename,
menu.foreground.font.color.b, menu.foreground.font.color.g, menu.foreground.font.color.r,
menu.frame.color.b, menu.frame.color.g, menu.frame.color.r, menu.separator.color.b,
menu.separator.color.g, menu.separator.color.r, menu.sidebar.bitmap.filename,
messengerbutton.caption, pluginhistorybutton.caption, pluginrootbutton.caption,
portalrootbutton.caption, purgeghostcachebutton.caption, quitbutton.caption,
rateofuseballoonbutton.caption, rateofusebutton.caption, rateofuserootbutton.caption,
rateofusetotalbutton.caption, readmebutton.caption, readmebuttoncaption,
recommendrootbutton.caption, regionenabledbutton.caption, reloadinfobutton.caption,
resetballoonpositionbutton.caption, resettodefaultbutton.caption,
sakura.popupmenu.applybindtoself, sakura.popupmenu.type, sakura.popupmenu.visible,
sakura.portalbuttoncaption, sakura.portalsites, sakura.recommendbuttoncaption,
sakura.recommendsites, scriptlogbutton.caption, shellrootbutton.caption,
shellscaleotherbutton.caption, shellscalerootbutton.caption, sntpbutton.caption,
switchactivatewhentalkbutton.caption, switchactivatewhentalkexceptupdatebutton.caption,
switchautobiffbutton.caption, switchautoheadlinesensebutton.caption,
switchblacklistingbutton.caption, switchcompatiblemodebutton.caption,
switchconsolealwaysvisiblebutton.caption, switchconsolevisiblebutton.caption,
switchdeactivatebutton.caption, switchdontactivatebutton.caption,
switchdontforcealignbutton.caption, switchduivisiblebutton.caption,
switchforcealignfreebutton.caption, switchforcealignlimitbutton.caption,
switchignoreserikomovebutton.caption, switchlocalsstpbutton.caption,
switchmovetodefaultpositionbutton.caption, switchproxybutton.caption, switchquietbutton.caption,
switchreloadbutton.caption, switchreloadtempghostbutton.caption, switchremotesstpbutton.caption,
switchrootbutton.caption, switchtalkghostbutton.caption, systeminfobutton.caption,
termsbutton.caption, texttospeechbutton.caption, updatebutton.caption, updatebuttoncaption,
updatefmobutton.caption, updateplatformbutton.caption, utilityrootbutton.caption,
vanishbutton.caption, vanishbuttoncaption, vanishbuttonvisible

---

## 第 3 部 — 受け口が無い外部との連携

ここから先は、送る側の欠落ではなく**外から来るものを受け取る口が 1 つも無い**群である。前の 2 部と
違い、名前を足す・値を読む、という話ではなく、待ち受ける仕組みそのものが存在しない。この部の 5 章で
201 項目を占める。判定の対象にならない群 5（168 件）もこの部に置いてある。areka の側では起きないと
判定した理由が「外から受け取る口が無い」ことであり、この部の他の章とまったく同じだからである。

### 群 14a — ゴースト同士の交わり 8 件

**利用者に何が起きるか**

うちのゴーストたちが互いに気づかない。2 体以上を並べても黙ってすれ違い、話しかけ箱から声を掛けても
返らない。1 体ずつ立ち上げるのと変わらなくなる。

**その群を成立させる最小の基盤**

- **今ある物**: この群については無い。SSTP の待ち受けも、起動中のゴーストの一覧を共有する仕組みも
  `crates/` 以下に 1 か所も無い（バルーンの `sstpmessage` で始まる設定キーは、未知のキーとして
  読み飛ばされているだけで、SSTP の実装ではない）。
- **足りない物**: 待ち受ける口、一覧を共有する領域の読み書き、そこから受けた要求を SHIORI へ渡す
  経路の 3 つ。群 2 の「交わり」23 件と群 5 の一部は、この土台の上でしか成立しない。

**台帳の項目 id**

`ukadoc:spec_sstp:request:1`, `ukadoc:spec_sstp:response:1`,
`ukadoc:spec_fmo_mutex:32_30d0_30a4_30c8_306e_8b58_5225ID:1`,
`ukadoc:spec_fmo_mutex:FMO_306e_30b5_30a4_30ba:1`,
`ukadoc:spec_fmo_mutex:FMO_306e_540d_524d_3068_6587_5b57_30b3_30fc_30c9:1`,
`ukadoc:spec_fmo_mutex:_30ad_30fc_540d_30fb_5024:1`,
`ukadoc:spec_fmo_mutex:_30c7_30fc_30bf_672c_4f53:1`,
`ukadoc:spec_fmo_mutex:_30c7_30fc_30bf_7d42_7aef:1`

---

### 群 13 — PLUGIN の受け口 19 件

**利用者に何が起きるか**

プラグインを入れても何も起こらない。プラグイン向けのイベントを送る先も、プラグインからの照会を
受ける口も無いので、配布されているプラグインは areka の上では単に無視される。

**その群を成立させる最小の基盤**

- **今ある物**: この群については無い。プラグインを読み込む仕組みが `crates/` 以下に 1 件も無く、
  `\![raiseplugin]`・`\![notifyplugin]` を受ける場所も 0 件である。
- **足りない物**: 読み込む仕組み、プラグインへ送る側、プラグインからの照会を受ける側。19 件のうち
  12 件は SHIORI 側にも同じ名前があり、台帳では `same-feature` で結んである。

**台帳の項目 id**

id は `ukadoc:list_plugin_event:<名前>:1` の形。名前は次の 19 件。

`OnChoiceSelect_28Ex_29_2fOnAnchorSelect_28Ex_29_2f_5cq_7b49_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30f3_`,
OnGhostBoot, OnGhostExit, OnGhostInfoUpdate, OnInstallComplete, OnMenuExec, OnOtherGhostTalk,
OnSecondChange,
`_5c_21_5braiseplugin_5d_304a_3088_3073_5c_21_5bnotifyplugin_5d_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30`,
balloonpathlist, ghostpathlist, headlinepathlist, installedballoonname, installedghostname,
installedplugin, pluginpathlist, property.get, property.set, version

（長い 2 件は、正典の見出しがそのまま項目の名前になったものである。それぞれ「選択肢やアンカーに
書かれた任意名のイベント」と「`\![raiseplugin]`／`\![notifyplugin]` に書かれた任意名のイベント」の
枠を指す。）

---

### 群 14b — ブラウザ・プラグイン・ヘッドライン 5 件

**利用者に何が起きるか**

配布サイトの導入用のリンクを押しても areka には何も届かない。ゴーストの配布ページから 1 押しで
入れる、という導線がまるごと成立しない。ヘッドラインの取得も無いので、更新情報を読み上げる類の
仕掛けも動かない。

**その群を成立させる最小の基盤**

- **今ある物**: sylphya の設定の根の名前（`crates/areka-sylphya/src/vocab/dotted.rs`）と、
  ヘッドライン・プラグイン関連のボタンの文言の語彙だけ。どちらも名前があるだけである。
- **足りない物**: ブラウザからの受け口、プラグインの決まりに沿った読み込み、ヘッドラインの取得。
  いずれも `crates/` 以下に無い。

**台帳の項目 id**

`ukadoc:spec_headline`, `ukadoc:spec_plugin`,
`ukadoc:spec_web:x-ukagaka-link_3atype_3devent_26ghost_3d_28_30b4_30fc_30b9_30c8_540d_29_26info_3d_28_8ffd_52a0_60c5_5831_29:1`,
`ukadoc:spec_web:x-ukagaka-link_3atype_3dhomeurl_26url_3d_28_30a8_30f3_30b3_30fc_30c9_6e08URL_29:1`,
`ukadoc:spec_web:x-ukagaka-link_3atype_3dinstall_26url_3d_28_30a8_30f3_30b3_30fc_30c9_6e08URL_29:1`

（`ukadoc:spec_headline` と `ukadoc:spec_plugin` は、見出しの無いページ全体で 1 項目という粗い
粒度の id である。）

---

### 群 14c — DLL 共通仕様 1 件

**利用者に何が起きるか**

SHIORI の DLL は読み込めるので、ゴーストは動く。一方、同じ入口の決まりを共有する SAORI・MAKOTO・
PLUGIN の DLL は読み込む呼び出しそのものが無いので同居できない。SAORI を前提に書かれた既存ゴースト
（外部の部品に計算や読み上げを任せているもの）は、その部分だけが動かない。

**その群を成立させる最小の基盤**

- **今ある物**: 32bit の助け手が絶対パスで DLL を読み込み、`load`・`unload`・`request` の 3 つを
  名前で引いてから呼んでいる。読み込みに失敗したときは黙って先へ進まず、失敗として呼び出し元へ
  返る。
- **足りない物**: ⑴ 正典が定める 4 つ目の入口 `loadu` を名前で引く枝（正典はこちらを優先して使う
  ことにしており、`load` との違いは置き場所のパスの文字コードである——`loadu` は UTF-8、`load` は
  既定の各国語コードページ）、⑵ SAORI・MAKOTO・PLUGIN の DLL を読み込む側。MAKOTO の担当は
  `areka-P0-makoto-dll-host` である。SAORI については areka がプロトコルを実装するのではなく、
  32bit の同じプロセスに同居できること・作業ディレクトリ・DLL の探索パスの 3 つが条件になる。

**台帳の項目 id**

`ukadoc:spec_dll`（ページ全体で 1 項目・`degraded`・仮の値 `E1`）

---

### 群 5 — 外部が送る拡張イベント 168 件

**利用者に何が起きるか**

対応アプリやゲームとの連携が何も動かない。この 168 件は areka が送るものではなく、外部のアプリ・
プラグイン・他のゴーストが「このゲームでこういう出来事があった」とゴーストへ知らせるための名前で
あり、areka に問われるのは受け取る口があるかどうかだけである。その口が無いので、連携を前提に
書かれた辞書は 1 度も呼ばれない。台帳ではこの 168 件を `not-applicable`（areka が実装の主体では
ない）として扱っており、テーマも付けていない——送信元が areka でない以上、「これが無いと利用者は
ゴーストの何を失うか」に areka の側から答えられないからである。

**その群を成立させる最小の基盤**

- **今ある物**: 受け取った要求を SHIORI へ渡す口は 1 つある（往復の口）。
- **足りない物**: 外から任意の名前のイベントを受け取る経路。具体的には群 14a の SSTP の待ち受けと
  群 13 のプラグインの受け口が土台であり、この 168 件はその上に乗る。土台が無い間は、名前を 1 つ
  ずつ扱っても意味がない。

**台帳の項目 id**

id は `ukadoc:list_shiori_event_ex:<名前>:1` の形。168 件すべてが `not-applicable`・テーマ空・
優先度は空文字である。名前は次のとおり。

OnApplicationBoot, OnApplicationClose, OnApplicationExist, OnApplicationFileOpen,
OnApplicationOperationFinish, OnApplicationVersion, OnBatteryCritical, OnBatteryLow, OnBeerShower,
OnCrystalDiskInfoClear, OnCrystalDiskInfoEvent, OnDive, OnElinAllyCondition, OnElinAllyDead,
OnElinCatchFish, OnElinMapCharaGenerate, OnElinMapEnter, OnElinMapItemGenerate, OnElinPCCondition,
OnElinPCDead, OnElinTarget, OnElonaOmakeMMAEventAbandonPet, OnElonaOmakeMMAEventAddNewsTopic,
OnElonaOmakeMMAEventAdventGod, OnElonaOmakeMMAEventAreaChanged, OnElonaOmakeMMAEventAreaChanged_2a,
OnElonaOmakeMMAEventAtonement, OnElonaOmakeMMAEventAwake, OnElonaOmakeMMAEventBecomeCriminal,
OnElonaOmakeMMAEventBelieveGod, OnElonaOmakeMMAEventBuyNuke, OnElonaOmakeMMAEventClothOut,
OnElonaOmakeMMAEventConqueredLesimas, OnElonaOmakeMMAEventCooking, OnElonaOmakeMMAEventDead,
OnElonaOmakeMMAEventEtherDisease, OnElonaOmakeMMAEventEtherDiseaseCured,
OnElonaOmakeMMAEventGameLoad, OnElonaOmakeMMAEventGameQuit, OnElonaOmakeMMAEventGrandmapocalypse,
OnElonaOmakeMMAEventHour, OnElonaOmakeMMAEventHourPlayed, OnElonaOmakeMMAEventInvestNPC,
OnElonaOmakeMMAEventJoinGuild, OnElonaOmakeMMAEventJoinParty, OnElonaOmakeMMAEventLastword,
OnElonaOmakeMMAEventLevelUp, OnElonaOmakeMMAEventLomiasInTheParty, OnElonaOmakeMMAEventLomiasKilled,
OnElonaOmakeMMAEventMapChanged, OnElonaOmakeMMAEventMarriage, OnElonaOmakeMMAEventMewmewmew,
OnElonaOmakeMMAEventMutation, OnElonaOmakeMMAEventMutationCured, OnElonaOmakeMMAEventNewGame,
OnElonaOmakeMMAEventNukeExploded, OnElonaOmakeMMAEventOffer, OnElonaOmakeMMAEventPayTax,
OnElonaOmakeMMAEventPerformance, OnElonaOmakeMMAEventPetDead, OnElonaOmakeMMAEventPray,
OnElonaOmakeMMAEventPrayEyth, OnElonaOmakeMMAEventPrayFailed, OnElonaOmakeMMAEventRagnarok,
OnElonaOmakeMMAEventRandomEvent, OnElonaOmakeMMAEventReadTreasureMap,
OnElonaOmakeMMAEventRefuseMarriage, OnElonaOmakeMMAEventSaleSlave, OnElonaOmakeMMAEventSaleWife,
OnElonaOmakeMMAEventSetNuke, OnElonaOmakeMMAEventSisterRagnarok, OnElonaOmakeMMAEventSkillDown,
OnElonaOmakeMMAEventSkillLearned, OnElonaOmakeMMAEventSkillUp, OnElonaOmakeMMAEventSleep,
OnElonaOmakeMMAEventStealItem, OnElonaOmakeMMAEventTravelGuide,
OnElonaOmakeMMAEventTreasureDigging, OnElonaOmakeMMAEventWeatherChanged, OnElonaOmakeMMAEventWish,
OnElonaOmakeMMAEventWished, OnElonaOmakeMMAEventZeomeKilled, OnFleetClockComplete, OnGetValues,
OnHandActivate, OnHitThunder, OnHttpcNotify, OnHydrateStatsNotify, OnJitenBattle, OnJitenTagBattle,
OnKanadeTeaParty, OnKanadeTeaPartyEnd, OnKanadeTeaPartyInfomationRequest, OnKinokoObjectChanged,
OnKinokoObjectChanging, OnKinokoObjectCreate, OnKinokoObjectDestroy, OnKinokoObjectInstalled,
OnMahjong, OnMahjongResponse, OnMglBattle, OnMopClear, OnMusicPlay, OnMusicPlayer.SongInfo,
OnNeedlePoke, OnNekodorifObjectDodge, OnNekodorifObjectDrop, OnNekodorifObjectEmerge,
OnNekodorifObjectHit, OnNekodorifObjectVanish, OnNostr, OnPoker, OnPokerNotify, OnPotatoError,
OnPotatoFileNotFound, OnPotatoReturn, OnRequestValues, OnSatolistBoot, OnSatolistClosed,
OnSatolistDictionaryFolderChanged, OnSatolistEventAdded, OnSatolistGhostOpened, OnSatolistSaved,
OnSpectrePlugin.ConfirmCalibration, OnSpectrePlugin.Possession, OnSpectrePlugin.Surface,
OnStampAdd, OnStampInfo, OnStampInfoCall, OnSysResourceCritical, OnSysResourceLow, OnTalkRequest,
OnTourabuConquestEnd, OnTourabuConquestStart, OnTourabuDutyEnd, OnTourabuDutyStart,
OnUkadocScriptExample, OnWeatherStation.Alerts, OnWeatherStation.Astro, OnWeatherStation.Error,
OnWeatherStation.Forecast.Day, OnWeatherStation.Forecast.Hourly, OnWeatherStation.Weather,
OnWebsiteUpdateNotify, Send60stair_Call, Send60stair_DiceRoll, Send60stair_Dobon,
Send60stair_GameEnd, Send60stair_GetStatus, Send60stair_Goal, Send60stair_Marking,
Send60stair_Start, Send60stair_YourTurnEnd, Send60stair_YourTurnStart, ShioriEcho.Begin,
ShioriEcho.CommandComplete, ShioriEcho.CommandHistory.ForwardIndex, ShioriEcho.CommandHistory.Get,
ShioriEcho.CommandHistory.New, ShioriEcho.CommandHistory.Update, ShioriEcho.CommandPrompt,
ShioriEcho.CommandUpdate, ShioriEcho.End, ShioriEcho.GetName, ShioriEcho.GetResult,
ShioriEcho.TabPress, ShioriEcho, `_53ef_5909_540d_306e_8fd4_4fe1_30a4_30d9_30f3_30c8`

（最後の 1 件は「可変名の返信イベント」という見出しがそのまま項目になったもので、決まった 1 つの
名前ではなく、依頼の側が指定した名前で返す枠を指す。）

---

## 第 4 部 — 判定の対象にならない群と、既にできている群

未対応の話はここで終わりである。以下の 6 章は、対応の順序を考える対象ではないが、18 の群を
漏れなく並べるために置く。

### 群 3 — 旧仕様の別名 3 件

**利用者に何が起きるか**

この 3 行そのものでは何も起きない。正典本文がいずれも「旧仕様」と述べており、実際に何が起きるかは
写像先の `OnFileDrop2` の行に従う。その行は群 2 に属し、状態は `absent` である——つまり、
落としたファイルは新旧どちらの名前でも伝わらない。

**その群を成立させる最小の基盤**

- **今ある物**: 別名の向きが正典本文の記述から決まっており、台帳では `alias_of` で写像先を指して
  ある。別名の連鎖は作っていない。
- **足りない物**: この行としては無い。写像先の行が必要とするもの（群 2 と同じ）がそのまま当たる。
  なお `OnFileDropping` はドラッグ中の通知であって旧仕様ではないので、この 3 件には含めない。

**台帳の項目 id**

`ukadoc:list_shiori_event:OnFileDrop:1`・`ukadoc:list_shiori_event:OnFileDropEx:1`・
`ukadoc:list_shiori_event:OnFileDropped:1`

---

### 群 15 — イベント一覧の補足 1 件

**利用者に何が起きるか**

何も起きない。この項目は正典のイベント一覧のページに添えられた読み方の補足であって、areka が送る・
引く・受けるものではない。実装の対象にならないので、利用者から見える壊れ方も無い。

**その群を成立させる最小の基盤**

- **今ある物**: 該当なし（実行時の経路を持たない項目である）。
- **足りない物**: 該当なし。ページ全体で 1 項目という粗い粒度なので、他のページの 1 項目と同列に
  数えないこと。

**台帳の項目 id**

`ukadoc:memo_shiorievent`

---

### 群 1 — 送出しているイベント 11 件

**利用者に何が起きるか**

失うものは無い。areka はこの 11 件を実際に SHIORI へ送っており、起動から終了まで、押す・動かす・
選ぶ、という最小限の会話は成立する。

**その群を成立させる最小の基盤**

- **今ある物**: 許可の表と構築関数と送る口が揃っている。送出の直前には記録が 1 行残り、送出に
  失敗すれば失敗として扱われるので、黙って消えることはない。この 11 件の名前は逐語一致のテストで
  固定してあり、静かに増減しない。
- **足りない物**: この群としては無い。

**台帳の項目 id**

id は `ukadoc:list_shiori_event:<名前>:1` の形。OnBoot, OnChoiceSelect, OnChoiceSelectEx,
OnChoiceTimeout, OnClose, OnFirstBoot, OnInitialize, OnMouseDoubleClick, OnMouseMove,
OnSecondChange, basewareversion

（`basewareversion` は `On` で始まらないが、ベースウェアの版を知らせる通知として実際に送っている
ため、群 4 ではなくこの群に属する。）

---

### 群 6 — 実際に引いているリソース 1 件

**利用者に何が起きるか**

失うものは無い。areka は起動の途中で利用者の呼び名を実際に引いている。応答が空だったり照会に
失敗したりしても起動は止まらず、既定の呼び名で先へ進む。

**その群を成立させる最小の基盤**

- **今ある物**: 引く名前を決める表・照会を組み立てる関数・往復の口が揃っている。値が無いときの
  既定値は 1 か所だけで定義してある。
- **足りない物**: この群としては無い。ただし、同じ仕組みの上に載るはずの 158 件が群 7・群 8 で
  待っている。

**台帳の項目 id**

`ukadoc:list_shiori_resource:username:1`

---

### 群 9 — 送っているヘッダ 5 件

**利用者に何が起きるか**

失うものは無い。要求の 1 行目と、送り主・実行状態・イベント名・参照値は毎回組み立てて送っている。

**その群を成立させる最小の基盤**

- **今ある物**: ヘッダを組み立てる場所が 1 つあり、値をそのまま書き出す形なので途中で失敗する
  経路が無い。
- **足りない物**: この群としては無い。実行状態の語彙そのものは別の spec
  （`areka-P0-status-execution-states`）が持つ。

**台帳の項目 id**

`ukadoc:spec_shiori3:_30e1_30bd_30c3_30c9:1`（メソッド）・`ukadoc:spec_shiori3:Sender:1`・
`ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1`・`ukadoc:spec_shiori3:ID:1`・
`ukadoc:spec_shiori3:Reference_2a:1`

---

### 群 12 — 解釈している応答 4 件

**利用者に何が起きるか**

失うものは無い。応答の状態番号と、返ってきた本文・エラーの重さ・エラーの説明を読み取って使って
いる。相手が失敗を返したときは黙って捨てず、記録が残る形で呼び出し元へ返る。

**その群を成立させる最小の基盤**

- **今ある物**: 応答を読み解く場所と、その結果を呼び出し元の型へ写す場所が揃っている。
- **足りない物**: この群としては無い。読み飛ばしている残りのヘッダは群 11 が持つ。

**台帳の項目 id**

`ukadoc:spec_shiori3:_30b9_30c6_30fc_30bf_30b9_30b3_30fc_30c9:1`（ステータスコード）・
`ukadoc:spec_shiori3:Value:1`・`ukadoc:spec_shiori3:ErrorLevel_20_5bSSP_62e1_5f35_5d:1`・
`ukadoc:spec_shiori3:ErrorDescription_20_5bSSP_62e1_5f35_5d:1`

---

## 次に読む人への申し送り

**⑴ 繋がりを書く向きは「イベントの行に書く」で統一してある。** タグや設定キーとの関わりは、
**イベントの行に `triggers`／`configures` を書き、相手としてタグ・設定キーの id を指す**向きで
そろえてある。逆向き（タグの行にイベントを書く）にしていないのは、要件 8.3 が他ドメインの項目の
行を本台帳に作ることを禁じているためで、書ける側が片方しかない。統合担当が 4 つの台帳を突き合わせる
ときは、この向きを前提に読むこと。向きを取り違えると、同じ 1 本の繋がりを二重に数えるか、逆向きに
探して見つからないことになる。

**⑵ 正典の側に綴りの誤りが 1 件ある。** `OnSelectModeBegin`・`OnSelectModeCancel`・
`OnSelectModeComplete` の本文は、対になるタグの名前を `\![enter,selectrect]`／`\![leave,selectrect]`
と書いている。しかし `selectrect` という綴りの項目はカタログに 1 件も無く、さくらスクリプト一覧の
側の綴りは `selectmode` である。台帳ではさくらスクリプト一覧の側の綴りを繋がりの相手にし、
この食い違いを該当 3 行の `note` に記録した。ukadoc 本体へ知らせる価値のある誤りである。

**⑶ 台帳に対して `ledger-init` を走らせないこと。** 道具の読み書きは行末を LF にそろえる作りに
なっており（`read_normalized` が復帰文字を落とし、`write_lf` が LF で書く）、**1 度走らせるだけで
台帳の全行（この時点で 16,134 行）の行末が書き換わる**。この書き換えは `git diff` の既定の表示には出ないが、
バイト単位で照合する検査は必ず割れる。台帳を触るときは行末を保つ手で編集し、`check` だけを走らせる
こと。

**⑷ 索引を直したときの手順**を守ること。索引の正本はこの文書で、台帳の冒頭にあるのはその写しである。
直すときは正本を先に直し、写しは節の全体を貼り直す（部分的に直さない）。写しと正本の食い違いは
機械の検査に出ないので、貼り直した後に `#` を外した写しと正本が行単位で一致することを自分で
確かめること。

---

## 是正の候補（この文書では直さず、担当のところへ送るもの）

隣接する spec の brief に正典と食い違う記述を見つけても、この調査では相手の brief を書き換えない。
候補として次に挙げる。

1. **本 spec の brief の記載 26 行**は、着手時に正典と本ワークツリーで引き直した結果が
   `requirements.md` の付録の表に記録してある。実害の大きいものを 3 つだけ再掲する——
   ⑴ `OnTeachInput` という名前の項目は正典に存在しない（実在は `OnTeach`・`OnTeachStart`・
   `OnTeachInputCancel`）、⑵ `basewareversion` は引く側のリソースではなく知らせる側のイベントで
   ある、⑶ `OnMenuExec` は SHIORI のイベント一覧には無く、PLUGIN 向けの一覧にだけある。
2. **設計の「7 件」を確定値として扱わないこと。** 設計 DD-3 は、ゴースト同士のやり取りが本文に
   書いてある拡張イベントを 7 件と数えている。しかし同じ論法で見るなら、返信の相手側である
   `OnMahjongResponse` も同じ性質を持つ。台帳の登記（7 行に共通の文面を写す）は変えていないが、
   統合担当はこの 7 を件数の根拠に使わないこと。
3. **`\![enter,selectrect]`／`\![leave,selectrect]` の綴り**（申し送り⑵）は正典側の誤りなので、
   ukadoc へのフィードバックの候補である。
4. **台帳の `ukadoc:spec_dll` の行の 1 段落は、2026-09-06 に直した（送り先は無い）。** 以前は
   DLL の入口を「load・unload・request の 3 つ」と書いており、areka が名前で引いている数ではなく
   **正典が定める入口の決まりそのものを 3 つと述べる形**になっていて、事実として誤りだった。
   正典が定める入口は `loadu`・`load`・`unload` の 3 つの生存期間の関数と、要求の `request` で
   ある。この台帳と索引は本 spec の持ち物であって送り先の担当が居ないので、候補として残さず、
   群 14c の共通 `note` と台帳の該当行 1 件をその場で書き直した（正典を引き直して確認済み）。

---

## この文書と台帳の対応

| 群 | 章のある部 | 件数 | 状態 |
|---|---|---|---|
| 2 | 第 1 部 | 248 | `absent` |
| 2a | 第 1 部 | 3 | `vocabulary-only` |
| 4 | 第 1 部 | 25 | `absent` |
| 8 | 第 1 部 | 27 | `vocabulary-only` |
| 10 | 第 1 部 | 2 | `degraded` |
| 11 | 第 1 部 | 15 | `absent` |
| 7 | 第 2 部 | 131 | `vocabulary-only` |
| 14a | 第 3 部 | 8 | `absent` |
| 13 | 第 3 部 | 19 | `absent` |
| 14b | 第 3 部 | 5 | `absent` |
| 14c | 第 3 部 | 1 | `degraded` |
| 5 | 第 3 部 | 168 | `not-applicable` |
| 3 | 第 4 部 | 3 | `alias` |
| 15 | 第 4 部 | 1 | `not-applicable` |
| 1 | 第 4 部 | 11 | `implemented` |
| 6 | 第 4 部 | 1 | `implemented` |
| 9 | 第 4 部 | 5 | `implemented` |
| 12 | 第 4 部 | 4 | `implemented` |

合計 677。第 1 部 320・第 2 部 131・第 3 部 201・第 4 部 25。
