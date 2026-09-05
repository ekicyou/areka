# SHIORI ドメインの調査ブリーフィング

この文書は、SSP 公式仕様書（ukadoc）のうち「ベースウェアと SHIORI／外部との対話面」に属する
677 項目について、areka がどこまで実現しているかを人が読める形で示すものである。数字ではなく
「利用者に何が起きるか」で読めることを目指している。機械が読む側の正本は台帳
`doc/ukadoc-coverage/ledger/shiori.toml` にある。

本文（群ごとに、利用者に何が起きるか・その群を成立させる最小の基盤・台帳の項目 id）は、この
索引に続けて置く。

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

**まだ確定していないもの**: 項目ごとのテーマ（群 1・2・6・7・8 の「項目ごと」と書いてある欄）と、
優先度の具体的な値は、この索引に後から書き足す。段階（A〜E）と数値はいずれも**仮置き**であり、
最終的な順序を決めるのは統合担当の `ukadoc-coverage-roadmap` である。

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
- **テーマ**: 装い
- **優先度**: `B4`（仮置き）
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

  他のゴーストが送る 7 件（`OnRequestValues`・`OnGetValues`・可変名の返信イベント・
  `Send60stair_GetStatus`・`OnKanadeTeaPartyInfomationRequest`・`OnPoker`・`OnMahjong`）には、
  上の文面に次の 1 文を足す。

  ```
  この項目の送信元は外部のアプリではなく他のゴーストで、ベースウェアがその伝達を運ぶ。areka には
  ゴースト間の伝達そのものが無いため、運ぶ側としても成立しない。
  ```

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
- **テーマ**: 項目ごと（装い・記憶が中心）
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

- **対象**: `ukadoc:spec_dll`（`load`・`unload`・`request` という DLL の入口の決まり。SHIORI・
  SAORI・MAKOTO・PLUGIN が共有する）。
- **件数**: 1
- **状態**: `degraded`
- **テーマ**: 空（`[]`）
- **優先度**: `E1`（仮置き）
- **判断の根拠の場所**: `crates/shiori-host32-helper/src/shiori_proxy.rs` の `ShioriProxy::load`
  （絶対パスで読み込み、3 つの入口を名前で引く）と `ShioriProxy::request`。SAORI・MAKOTO の語は
  `crates/` 以下のソースに 1 件も無い。MAKOTO の担当は `areka-P0-makoto-dll-host`。
- **共通 `note`**:

  ```
  壊れ方: できている範囲は明示的なエラーで守られており、できていない範囲は黙って壊れる。DLL の入口の
  決まり（load・unload・request の 3 つ）のうち、SHIORI の DLL については areka の 32bit の助け手が
  絶対パスで読み込み、3 つの入口をすべて名前で引いてから呼んでいる。同じ決まりを共有する SAORI・
  MAKOTO・PLUGIN の DLL は、読み込む呼び出しそのものが無いので同居できない。
  ログ: DLL が開けない・3 つの入口のいずれかが引けない・load が偽を返したときは、いずれも失敗として
  呼び出し元へ返り、起動が黙って先へ進むことはない。一方 SAORI・MAKOTO・PLUGIN については、読み込もう
  とする場所が無いのでログは 1 行も出ない。
  SAORI: areka はプロトコルを実装せず、実装の主体は SHIORI 側である。成立の条件は 32bit の同じ
  プロセスに同居すること・作業ディレクトリ・DLL の探索パスの 3 つ。ukadoc に SAORI の独立したページは
  無いので、SAORI 用の行は作らない。MAKOTO は areka-P0-makoto-dll-host が担う。
  縮退の転記元: doc/emo2-conformance-scope.md の旧ロードマップ spec への影響の表の行
  「areka-P0-shiori-host-32 ＝ SAORI 同居を M1 から削除（emo2 未使用）。32bit SHIORI 往復に集中」。
  粒度: このページ全体で 1 項目であり、他のページの 1 項目より粗い。
  根拠の場所: crates/shiori-host32-helper/src/shiori_proxy.rs の ShioriProxy::load と
  ShioriProxy::request。
  ```

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
