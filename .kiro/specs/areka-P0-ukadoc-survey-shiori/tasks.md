# 実装計画

> **前提の変更（2026-09-05・上流 `areka-P0-ukadoc-survey-toolkit` が着地したため）**: 道具（`crates/ukadoc-survey`）と `doc/ukadoc-coverage/` 一式が既に main にある。担当分の台帳 `ledger/shiori.toml` は **677 項目すべてが `unclassified` の状態で建っており**、カタログ `catalog.toml`（全 1,749 件）・テーマの正本 `values.md`・手引き `README.md`・報告 `report/shiori.md` も揃っている。したがって設計 DD-1（骨組みを作る使い捨て台本）と DD-9（検証の使い捨て台本）は前提が消え、道具の CLI と常時検査に置き換わる。要件 10.1（道具が着地済みなら本 spec が報告を再生成する）の側が確定した。

- [ ] 1. 基盤: 着地した台帳と道具を受け取り、検査が赤を出せることを確かめる
- [x] 1.1 担当 12 ページ 677 項目の集合と版番号の候補をカタログで確かめる
  - `doc/ukadoc-coverage/catalog.toml` を読み、ページ部分が担当 12 ページの項目が 677 件であること、ページ別の件数が要件 1.1 の内訳（イベント 290・拡張 168・リソース 159・SHIORI/3.0 26・PLUGIN 19・外部連携 6 ページ 14・補足 1）と 1 件ずつ一致することを確かめる
  - 食い違ったページがあれば仕分けに進まず、ページ名と件数を書き出して原因を先に解消する
  - 同じ読み取りで、繋がりの相手の実在を照らす先（カタログの全 1,749 件）と、`introduced` の候補になる `versions` 欄（担当分で 1 つ以上を持つのが 98 件・相異なる 2 つ以上を持つのが 11 件・拡張ページは 0 件）を控える
  - 完了条件: 12 ページの内訳と合計 677 がカタログと完全一致し、版番号を持つ 98 件と複数持つ 11 件の一覧が作業用の一時置き場に残っている
  - _Requirements: 1.1, 1.5, 1.6_

- [x] 1.2 着地している台帳を土台として受け取り、前置きと並びを確かめる
  - `doc/ukadoc-coverage/ledger/shiori.toml` が 677 個の `[entry."<id>"]` を持ち、すべて `unclassified` であること、`[ledger]` の `domain` と `pages` が担当と一致することを確かめる
  - 以後は id・並び順・`[ledger]` の前置き・冒頭の `#` コメントに触らず、値だけをその場で書き換える。独自の欄・独自の状態語彙・独自の関連の種別を足さない
  - id は見た目で直さずカタログから写す（符号化済みのため）。正典の見出しや本文を台帳に取り込まない
  - 骨組みを生成する使い捨て台本は書かない（道具の `ledger-init` が既に済ませている）
  - 完了条件: `cargo test -p ukadoc-survey` が緑で、台帳の 677 項目が付録 A の欄をそのまま持っている
  - _Requirements: 1.1, 1.2, 1.3, 1.7, 9.8, 12.7_

- [x] 1.3 道具の検査が赤を出せることを、書き始める前に確かめる
  - 台帳を仮に壊した状態で `cargo run -p ukadoc-survey -- check` を走らせ、少なくとも次の所見が実際に出ることを 1 件ずつ確かめる: `LedgerIdNotInCatalog`（id を 1 文字変える）・`LedgerOutOfOrder`（隣り合う 2 項目を入れ替える）・`LedgerPagesMismatch`（前置きのページを 1 つ落とす）・`AliasChain`（別名の指す先を別の `alias` へ向ける）・`LinkEndpointMissing`（相手 id の末尾の連番を別の数字に変える）・`IntroducedNotInCatalogVersions`（版番号を 1 桁変える）・`ImplementedWithoutEvidence`（証拠の無い行を `implemented` にする）・`SourceUrlNotInCatalog`（URL の符号化部分を 1 文字崩す）・`DomainReportStale`（台帳を直して報告を作り直さない）
  - 確かめたら壊した箇所をすべて戻し、`git status` で台帳が元のままであることを確認する
  - 未分類が 0 件であることと状態語の綴りは、報告の「未分類」列と読み込み段の失敗で見る（15 所見には別立てで現れない）
  - **テーマ名の綴り**（`UnknownTheme`）も同じく読み込み段の失敗で見る。読み取りの `parse_theme` と検査の `CheckInput::themes` が同じ `THEMES` 定数なので、台帳ファイル経由でこの所見に到達する道は無い（道具の常時テスト `a_misspelled_theme_turns_red` が覆う）
  - 使い捨ての検証台本は書かない（道具の 15 所見と常時検査が同じ範囲を覆う）
  - 完了条件: 9 通りの壊し方それぞれについて「その所見が出た」記録が残り、`UnknownTheme` は読み込み段の失敗で赤になることの記録が残り、戻した後に `cargo test -p ukadoc-survey` が緑
  - _Requirements: 1.4, 1.6, 6.8, 8.4, 10.5_

- [x] 1.4 群の索引を確定し、ブリーフィング文書の冒頭と台帳冒頭のコメントに置く
  - 設計「群の一覧」の 18 行（群の番号は 1〜15・うち 2 つが a／b／c に枝分かれする）それぞれについて、対象・件数・状態・テーマ・優先度・共通 `note` の全文・判断の根拠の場所（ファイル名と定義名。行番号は書かない）を 1 度だけ書く
  - 共通 `note` の文面には壊れ方の段（黙って壊れる／明示的なエラー／見た目の差）と、その根拠として「どのログが出るか・出ないか」を必ず含める
  - 正本はブリーフィング文書の冒頭に置く。台帳冒頭には道具が書いた `#` の前置きが既にあるので、それを消さずに群の索引の写しを書き足す（消えても正本は残る形にしておく）
  - ここで凍結するのは群ごとの共通 `note` の文面までとし、項目ごとのテーマ（タスク 5.1）・優先度の値（タスク 5.2）・仮置きである旨の明記（タスク 5.2）は後から索引に書き足してよい
  - 完了条件: 18 行すべての共通 `note` 文面が索引に確定して書かれ、以後の仕分けはこの文面を写すだけで済む状態になっている
  - _Requirements: 2.9, 7.6_

- [ ] 2. 送る側（SHIORI イベント 290）の仕分け
- [x] 2.1 実装済み 11・別名 3・意図的非発火 3 を確定する
  - 送出許可表（`areka-kanade` の `schedule/events.rs` の `ALLOWED_EVENT_IDS`）の 11 要素に対応する項目を `implemented` とする。`basewareversion` はこの 11 要素の 1 つであり、正典での所在に従ってイベント側の行に置き、送信の向き（ベースウェアからの通知）を `note` に書く
  - 正典本文が旧仕様と明記する `OnFileDrop`・`OnFileDropped`・`OnFileDropEx` を `OnFileDrop2` の別名として `alias` とし、`alias_of` に正典側の id を書く。`OnFileDropping` は別の機能なので含めない
  - `OnBalloonClose`・`OnBalloonTimeout`・`OnBalloonBreak` を `vocabulary-only` とし、`owner` に追跡先の spec 名を書き、`doc/COMPAT_ARCHITECTURE.md` の該当行を転記元として `note` に写す
  - 正典と別名の向きは上流 要件 4.1 の順序（正典本文の注記 → 版番号 → 人手の判断）で決め、判断の根拠となる areka 側の場所をファイルパスと定義名で書く（行番号は書かない）
  - 完了条件: 17 行の状態・`alias_of`・`owner`・`note` が埋まり、検査 ⑸（別名の連鎖が無いこと）が緑になる
  - _Requirements: 2.2, 2.4, 2.5, 2.9, 3.6, 6.1, 6.2, 6.3, 6.4, 12.3_

- [x] 2.2 送出していないイベント 248 を `absent` として埋める
  - 許可表に無い項目を `absent` とし、群の共通 `note`（「areka はこのイベントを送らない・例外にもログにも現れない」）を写す。`absent` の各行には根拠の場所を繰り返さない
  - areka の内部でだけ使う名前（`OnTalk`・`OnHour`・`OnMenuBack`）は行を作らず、最も近い正典項目の `note` に areka 側の扱いを書く
  - 正典 290 と既存カタログ `doc/shiori/fragments/events/` の 287 を id 単位で突き合わせ、差の 3 件（`OnArchiveViewerOpen`・`OnMediaPlayerOpen`・`OnPictureViewerOpen`）を該当する行の `note` に記録する
  - 完了条件: 自分の担当 248 行がすべて `absent` になり、差の 3 件が該当行の `note` から読み取れる（290 行全体で未分類 0 件になるのはタスク 7.1 の検査 ⑹ で確かめる）
  - _Requirements: 2.1, 2.3, 2.6, 2.11_

- [x] 2.3 `On` で始まらない同居項目 25 を送信の向きで書き分ける
  - `property.get`／`property.set`・`hwnd`・`uniqueid`・`capability`・`installed*`・`*pathlist`・`enable_log` ほかについて、ベースウェアからの通知なのか SHIORI から引く値なのかを `note` に書き分ける
  - `basewareversion` はこの 25 件に含めない（送出許可表にある実装済みの項目としてタスク 2.1 が書く。要件 2.4 の言う 26 件は 2.1 の 1 件と本タスクの 25 件で分担する）
  - `property.get`／`property.set` は `owner` に `areka-P0-property-query-channels` を書くだけとし、その spec の判断を上書きしない
  - 完了条件: 25 行すべてに状態と向きの記述が入り、テーマは原則空・優先度は配管の値になっている
  - _Requirements: 2.4, 12.3_

- [x] 2.4 `OnUpdate` 系 26 件を 1 つの群として揃える
  - 26 件に同じテーマ（更新）と同じ優先度を置き、小群（本体更新 11・ゴースト以外の更新 8・点検 2・結果 5）の別を `note` の 1 文で書き分ける
  - 小群ごとの内訳が実測（合計 26）と一致することを、台帳を確定させる前に数え直して確かめる
  - 完了条件: 26 行のテーマ・優先度・状態が揃い、小群の別が `note` から一意に読み取れる
  - _Requirements: 2.7_

- [ ] 3. 引く側（リソース 159）と外部が送る拡張（168）の仕分け
- [ ] 3.1 リソース 159 を実装済み 1・画面の材料 131・その他 27 に仕分ける
  - 照会許可表（`areka-kanade` の `schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`）に対応する 1 件を `implemented` とし、値が無いときに既定値へ縮退する旨と転記元を `note` に書く
  - 語彙表（`areka-sylphya` の `vocab/shiori_resource.rs` の `SHIORI_RESOURCE_IDS`）に載るが実際には引かれていない項目を `vocabulary-only` とする。名前の突き合わせでは空白の全角・半角の違いを同一とみなす（該当は 1 件）
  - 画面の材料になる群（ボタンの文言・`menu.*`・`popupmenu.*`・`*.recommendsites`・`*.portalsites` ほか）を `values` と `links` で束ねられる形にし、それが何の画面の材料かを `note` に書く
  - 1 項目に 2 つの名前が入っている 1 件と、`doc/shiori/fragments/` 側が名前を置き換えて `.defaulttop` を落としている事実を `note` に明記する
  - 完了条件: 159 行の内訳が 1＋131＋27 になり、未分類が 0 件になる
  - _Requirements: 3.1, 3.2, 3.3, 3.5_

- [ ] 3.2 外部が送る拡張イベント 168 を 1 つの群として `not-applicable` にする
  - 群に共通の `note` に「送信元は areka ではない」「areka が問われるのは受け口の有無だけ」を書き、受け口（任意名イベントの経路）が areka に無い事実を群として 1 度だけ書く
  - 他のゴーストが送る 7 件（要件が名指しする 3 件＋本文が `raiseother`／`notifyother` を使うと明記する 4 件）には、ゴースト間の伝達そのものが areka に無いことを理由として書き添える
  - 168 件の `values` をすべて空にし、版番号を持つものが 0 件であることを確かめて `introduced` をすべて空にする
  - 完了条件: 168 行すべてが同じ状態と共通 `note` を持ち、`values` と `introduced` が全件空になっている
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5_

- [ ] 4. ヘッダ・PLUGIN の受け口・外部連携の仕分け
- [ ] 4.1 SHIORI/3.0 の 26 項目をリクエスト 11・レスポンス 15 に分けて仕分ける
  - 送っているヘッダ 5 を `implemented`、固定値でしか送れない 2（`Charset`・`SecurityLevel`）を `degraded` とし、固定値である旨を `note` に書く。`Charset` の行の `owner` に `areka-P0-charset-canon` を書く
  - 送らない 4（`SenderType`・`SecurityOrigin`・`BaseID`・リクエスト側の `X-SSTP-PassThru-`）を `absent` とし、`build_request` の説明に `BaseID` が挙がっていないことを `note` に書く
  - 読み飛ばす応答ヘッダ 11 を `absent`、現に解釈する 4（ステータスコード・`Value`・`ErrorLevel`・`ErrorDescription`）を `implemented` とし、いずれも判断の根拠を `parse_response` の分岐としてファイル名と定義名で書く
  - 見出しが同じ 2 組（`Charset`・`Sender`）を項目 id で区別し、1 つの行にまとめない。廃止予定の旧名の注記はレスポンス側の項目に登記する
  - `Reference*`／`Reference0`／`Reference1〜` のように 1 項目が可変個のヘッダを表すものについて、その粗さを `note` に書く
  - 定義箇所が特定できない項目は `implemented` とせず、`vocabulary-only` または `degraded` として理由を `note` に書く
  - 完了条件: 26 行の内訳が 5＋2＋4＋11＋4 になり、リクエスト側とレスポンス側の別が `note` から読み取れる
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.4a, 5.5, 5.8, 6.6, 9.9, 12.3_

- [ ] 4.2 PLUGIN の受け口 19 項目を種別で書き分ける
  - イベント・PLUGIN 向けのリソース・プロパティの照会・任意名イベントの枠という種別の違いを `note` に書き分ける
  - 完了条件: 19 行すべてに状態と種別の記述が入り、未分類が 0 件になる
  - _Requirements: 5.6_

- [ ] 4.3 外部連携 14 項目と補足 1 項目を「受け口の有無」で判定する
  - SSTP 2・FMO 6 を交わりの群として、WEB 3・PLUGIN 1・HEADLINE 1 を周辺の群として `absent` にする
  - DLL 共通仕様 1 件を `degraded` とし、`note` に SHIORI 用の入口が host-32 に実装済みであること・SAORI／MAKOTO／PLUGIN の同居が無いこと・SAORI の成立条件（32bit の同じプロセスに同居・作業ディレクトリ・DLL の探索パス）・MAKOTO の担当 spec を書く。SAORI の独立した行は作らない
  - アンカーの無いページ全体で 1 項目である 4 件（DLL 共通仕様・PLUGIN・HEADLINE・イベント一覧の補足）について、他ページの 1 項目より粒度が粗い旨を `note` に書く
  - イベント一覧の補足 1 件にも状態を与え、それが一覧の補足であることを `note` に書く
  - 完了条件: 15 行すべてに状態が入り、受け口の有無以外の判断（実装の可否）を書いていない
  - _Requirements: 2.10, 5.7, 5.8, 5.9_

- [ ] 5. テーマ・優先度・繋がり・版番号の総仕上げ
- [ ] 5.1 テーマを 8 語彙の中から付け、付けた理由と付けない理由を書く
  - 名前の頭による既定（設計「テーマの付け方」の表）を出発点とし、最後は 1 項目ずつ正典本文を読んで上流の付与規則（「無いと利用者はゴーストの何を失うか」に答えられるテーマだけを付ける）に当てはめる
  - 既定が本文と合わない項目は個別に直し、直した理由を `note` に書く。テーマを付けた行には「無いと利用者が失うもの」を 1 文で書く
  - テーマを付けない既定の群（拡張 168・`OnBasewareUpdating`／`OnBasewareUpdated`・`property.get`／`property.set`・HEADLINE）はその理由を `note` に書く
  - 完了条件: `values` に上流の 8 語彙以外が 1 つも現れず（検査 ⑷ が緑）、テーマの付いた行すべてに失うものの 1 文がある
  - _Requirements: 7.1, 7.2, 7.3, 7.4_

- [ ] 5.2 優先度を群ごとに 1 つの値で仮置きする
  - 設計「群→段階」の表に従い、段階 1 文字（A〜E）と数値を群ごとに置く。表に当たらない項目は同じテーマの群の値を写す
  - 各項目の壊れ方の段を `note` に書き、その根拠として「どのログが出るか・出ないか」を添える
  - 優先度の 4 つの根拠の序列を入れ替えず、段階と数値がいずれも仮置きであること（最終決定は統合担当）を台帳冒頭の索引に明記する
  - 完了条件: `priority` が空でない行すべてが群→段階の表のいずれかの値と一致し、**索引が具体値を定めている群の行に空欄が 1 つも残っておらず**（特に群 2a の 3 行が `B4`）、壊れ方の段が `note` から読み取れる
  - _Requirements: 7.5, 7.6, 7.7_

- [ ] 5.3 繋がりを 6 種別の中で登記する
  - 確定している繋がり（拡張から本体への `same-feature` 7・拡張と本体の同名 3・`OnUpdate` の掲載順の鎖 22・本体更新とゴースト以外の更新の対応 8・`sakura.`／`kero.`／`char*.` の総当たり 27・PLUGIN と本体の同名 12・マウスの分担 1）を、設計が定めた「書く側の決め方」に従って一方の側にだけ書く
  - `sakura.*`／`kero.*`／`char*.*` は新旧の関係として扱わず、正典本文が示すスコープの違いとして結ぶ
  - 発火条件の源（descript のキー・プロパティ・さくらスクリプトのタグ・OS の事象・利用者の操作）を `triggers`／`configures`／`queries` で登記する。相手が他ドメインの項目でもその id を書いてよく、その項目の行は作らない
  - 繋がりに人手の名付けや解説を持たせない
  - 完了条件: 検査 ⑺（相手 id が全 1,749 件の一覧に実在すること）が緑になり、確定している繋がりの本数が設計の表と一致する
  - _Requirements: 2.8, 3.4, 6.5, 8.1, 8.2, 8.3, 8.4, 8.5_

- [ ] 5.4 版番号を決め、複数ある項目の扱いを揃える
  - 版番号を持つ 98 件について、項目そのものの登場を示す版番号を `introduced` に書く。書く値は**カタログの `versions` 欄にある版番号のいずれか**にする（道具の検査 `IntroducedNotInCatalogVersions` が照らす先はカタログであり、正典本文を自分で読み直した値ではない）
  - 相異なる版番号を 2 つ以上持つ **11 件**（`OnDarkTheme`・`OnDisplayChangeEx`・`OnNetworkStatusChange`・`OnRecycleBinEmpty`・`OnRecycleBinStatusUpdate`・`sakura`／`kero`／`char*` の `popupmenu.applybindtoself` 3 件・FMO のキー名と値・`ValueNotify`・SSTP の `request`）は、残りの版番号とその意味（挙動の変更・引数の追加など）を `note` に書く。本文から登場の版を判別できなければ最も小さい版番号を書く
  - 設計と要件 6.10 が「12 件」としていたのはカタログ着地前の見立てで、カタログ実測は 11 件である。この訂正を `note` かブリーフィングに記録する
  - 版番号が無い項目は `introduced` を空にし、最も古いものとして扱わない（拡張ページはカタログでも 0 件）
  - 完了条件: `introduced` が入っている行の値がすべてカタログの `versions` に含まれ（検査が緑）、98 件以外は空になっている
  - _Requirements: 4.5, 6.9, 6.10_

- [ ] 5.5 縮退の転記元を読み直して漏れが無いことを確かめる
  - `doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表の全行と `doc/emo2-conformance-scope.md` の見直し表を 1 行ずつ読み直し、SHIORI ドメインの項目を名指ししている行が設計の表の 5 行のほかに無いことを確かめる
  - 増えていれば同じ形で対応する行の `note` に転記元を書き足す
  - 転記元を書くのは縮退の記録が名指ししている行（群 2a の 3 件・`Status`・`username`・DLL 共通仕様）に限る。語彙表に載るだけの `vocabulary-only` の行は、転記元ではなく判断の根拠の場所（ファイル名と定義名）を持つ
  - 完了条件: 名指しの行の一覧が設計の表と一致し（または増えた分が台帳に反映され）、名指しされた行すべてに転記元がある
  - _Requirements: 3.6_

- [ ] 6. 証拠のコメントとブリーフィング文書
- [ ] 6.1 (P) 送出・照会の許可表に正典 URL のコメント 12 行を置く
  - 送出許可表の 11 要素それぞれの直前に `//` で `ukadoc: <正典 URL>` を 1 行ずつ置く（要素には `///` を使わない）
  - 照会許可表の doc コメントの末尾に `///` で 1 行置く（要素が 1 件で 1 行に収まるため定義そのものに置く）
  - URL はスナップショットの値をそのまま写し、説明文を伴わない 1 行だけを追加する。実行時に評価される記述を 1 行も追加・変更・削除しない
  - 完了条件: 2 ファイルの差分がコメント 12 行だけで、`cargo test -p areka-kanade` が緑
  - _Requirements: 9.1, 9.2, 9.5_
  - _Boundary: areka-kanade schedule_
  - _Depends: 2.1, 3.1_

- [ ] 6.2 (P) リソース語彙表にページの URL 1 行を置く
  - 語彙表の doc コメントの末尾に `///` でページの URL を 1 つ置き、159 の要素ごとには置かない
  - 語彙表の中身（159 要素）を書き換えない
  - 上流へ送る材料を `research.md` に残す。⑴ 同じページの URL が 2 か所（語彙表の先頭と照会許可表の 1 件）に現れる＝重複した証拠の扱いの最初の実例／⑵ 台帳へ id を挿入する処理が、道具の書いた冒頭の `#` 前置きだけでなく人が書き足した `#` コメント行も保存するかどうか（`ledger-init` を試した結果を書く）。版番号の取り出し方は着地したカタログの `versions` が正本と決まったので材料から外す
  - 完了条件: ソース側の差分がコメント 1 行だけで、材料 2 件が `research.md` に書かれ、`cargo test -p areka-sylphya` が緑
  - _Requirements: 9.1, 9.3, 9.5, 12.6_
  - _Boundary: areka-sylphya vocab_
  - _Depends: 3.1_

- [ ] 6.3 (P) SHIORI/3.0 のやり取りに正典 URL のコメント 9 行を置く
  - 作業に入る時点で既定の枝の該当ファイルが既に任意の文字コードへ書き換わっているかを確かめ、書き換わっていれば書き換わった後の定義箇所に置き直す（隣接 spec が先着した場合の切替）
  - リクエストを組み立てる側は要求行・`Sender`・`Status`・`ID`・`Reference` を書き出す文の直前に 5 行、応答を読む側はステータスコードを取り出す文の直前と 3 つの分岐の腕の直前に 4 行を `//` で置く
  - `Charset` と `SecurityLevel` を書き出す文には置かない（この 2 項目は実装済みでないため）
  - 完了条件: 差分がコメント 9 行だけで、`cargo build -p shiori-host32-host` が警告なく通る
  - _Requirements: 9.1, 9.2, 9.4, 9.5_
  - _Boundary: shiori-host32-host_
  - _Depends: 4.1_

- [ ] 6.4 (P) ブリーフィング文書を書く
  - タスク 1.4 が冒頭に置いた群の索引は書き換えず、その後ろに章を足す形で書く
  - 台帳の群がそのまま章になり、「黙って壊れる」ものを先に、次にテーマの付いたもの、最後に受け口が無い外部連携という順で並べる
  - 各章に「利用者に何が起きるか（何を失うか）」「その群を成立させる最小の基盤（今ある物と足りない物を分けて）」「台帳の項目 id」を書く
  - 「段階と優先度は仮置きであり最終決定は統合担当が行う」を冒頭に明記する
  - 憶測の実装計画（設計・工程・見積り）と段階の最終順序を書かず、内輪でしか通じない言い回しを使わない。隣接 spec の brief との食い違いは brief を書き換えず末尾に是正の候補として列挙する
  - 完了条件: 群の一覧の 18 行すべてが章として現れ、未対応であることが結論として明記されている
  - _Requirements: 6.7, 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 12.4_
  - _Boundary: briefing-shiori.md_
  - _Depends: 1.4, 5.4_

- [ ] 6.5 コメントを足しても既存のテストが緑のままであることを確かめる
  - 逐語一致のテスト 4 本（送出許可 ID・照会許可リソース・語彙表の件数・台帳キーの決定性）が緑であることを、対象クレートのテストを走らせて確かめる
  - 触れた 4 ファイルが 1 ファイル 1,000 行の上限を超えないことを、上限の見張りのテストで確かめる
  - コメントを足した 4 ファイルの属するクレートをビルドし、`unused_doc_comments` の警告が 1 件も出ないことを確かめる（`///` と `//` の使い分けが正しいことの確認）
  - 4 ファイルの差分を読み直し、追加がコメント 22 行だけで実行時に評価される記述が 1 行も変わっていないことを確かめる
  - 完了条件: `areka-kanade`・`areka-sylphya`・`log-capture-kit`・`ukadoc-survey` のテストが緑、`shiori-host32-host` が警告なくビルドでき、ソースの差分がコメント 22 行のみ
  - _Requirements: 9.6, 9.7, 12.1_
  - _Depends: 6.1, 6.2, 6.3_

- [ ] 7. 検証と完了条件の確定
- [ ] 7.1 道具の検査を確定した台帳に対して完走させる
  - `cargo run -p ukadoc-survey -- check` と `cargo test -p ukadoc-survey` を走らせ、15 所見のいずれも出ないことを確かめる
  - 証拠の突き合わせ（ソースの正典 URL ⇄ `implemented` の行）は `SourceUrlNotInCatalog` と `ImplementedWithoutEvidence` が担う。手掛かりが要るときは `evidence`／`candidates` を使う
  - ページ別の件数が合わないときは台帳を確定させず、食い違ったページ名と件数を示して原因を先に解消する。数を合わせるために行を足したり消したりしない
  - 完了条件: 検査が所見 0 件で通り、報告の「未分類」列が全ページ 0 件
  - _Requirements: 1.4, 10.5_
  - _Depends: 6.1, 6.2, 6.3_

- [ ] 7.2 ドメイン別報告を台帳から作り直し、台帳と同じコミットに入れる
  - 道具は着地済みなので、本 spec が `report/shiori.md` の作り直しを所有する（要件 10.1 の側で確定。要件 10.2 の「未着地なら作らない」は起きない）
  - `cargo run -p ukadoc-survey -- report` で作り直す。報告を手で書き換えて辻褄を合わせない（食い違いは `DomainReportStale` で赤になる）
  - 触るのは自分の報告 1 本だけとし、`report/summary.md` は統合担当に残す。台帳と報告は同じコミットに入れる
  - 完了条件: `report/shiori.md` が台帳とバイト単位で一致し（`DomainReportStale` が出ない）、状態の分布の「未分類」が 0 件になっている
  - _Requirements: 10.1, 10.2, 10.3_
  - _Depends: 7.1_

- [ ] 7.3 非接触の境界を最終確認する
  - 変更したファイルの一覧を取り、台帳 1 本・報告 1 本・ブリーフィング 1 本・ソースのコメント 22 行・本 spec の `.kiro/specs/areka-P0-ukadoc-survey-shiori/` 配下の文書のほかに何も変わっていないことを確かめる
  - 他ドメインの台帳 3 本・全体の報告 `report/summary.md`・束の文書・語彙の文書 `values.md`・カタログ `catalog.toml`・手引き `README.md`・道具の crate・`.kiro/steering/roadmap.md`・`doc/shiori/fragments/`・語彙表の中身・隣接 spec の brief のいずれも変わっていないことを確かめる
  - 他ドメインの項目の行が台帳に 1 つも無いことを確かめる
  - 完了条件: 変更ファイルの一覧が上記の集合と完全一致し、`unclassified` 0 件と合わせて完了条件が満たされている
  - _Requirements: 10.4, 12.2, 12.5, 12.6_
  - _Depends: 7.1, 7.2_

## Implementation Notes

- 1.1: カタログ実測はページ別・合計 677・全 1,749・重複 id 無しまで完全一致。`versions` を持つ担当分 98 件・相異なる 2 つ以上 11 件・拡張 0 件も一致。作業ファイルは scratchpad の `shiori-page-counts.md` / `shiori-versions.md` / `shiori-catalog-ids.txt`。
- 1.1: 「相異なる版番号 2 つ以上」の 11 件は数え方に依存しない（重複潰しによる減少は 0 件）。設計・要件が「12 件」とするのは**正典本文の正規表現による計測**、実測 11 は**カタログ `versions` 欄による計測**で測る対象が違う。タスク 5.4 は「12 が誤り」と断ぜず「`introduced` の正本はカタログの `versions`」と揃える形で書くこと。
- 1.1: 「12 件」の記載は **4 か所**ある — `design.md:311`・`design.md:420`（要件対応表）・`requirements.md:164`（要件 6.10 本文）・`requirements.md:246`（付録の再検証表）。タスク 5.4 はこの 4 か所すべてを対象にする。
- 1.1: scratchpad の `shiori-catalog-ids.txt` は CRLF。LF 出力と `diff` で突き合わせる際は CR を落とすこと。
- 1.1: worktree では `git submodule update --init --recursive`（`vendors/pasta`）を先に済ませないと `cargo` が一切動かない。
- 1.2: 台帳は付録 A（`.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/requirements.md` A.1:184 / A.2:224 / A.3:241）どおり。欄の並びは全 677 項目で 1 通り（`status`→`introduced`→`owner`→`priority`→`values`→`links`→`note`）。`alias_of` 0 件・`supersedes` 0 件は**欠落ではなく正しい初期状態**（A.2 が「`alias` のとき必須・それ以外は書かない」「任意」と定め、A.2 末尾が初期値を逐語で列挙している）。
- 1.2: 台帳は CRLF・6,101 行・BOM 無し。**値を書き換える際も CRLF を保つこと。**
- 1.2: 触ってはいけない部分の SHA-256 を scratchpad `shiori-ledger-frozen.md` に控えた（冒頭コメント `a5ccc6a1…` / `[ledger]` 節 `4c9cc888…` / id 行 677 本 `b68abbbc…`）。別セッションでも台帳から数行で再計算できる。
- 1.2: 1.1 が残した `shiori-catalog-ids.txt` はカタログ全 1,749 件。担当 677 件に絞った `cat-shiori-ids.txt`（LF・677 行）を新設済み。繋がりの相手の実在照合には全件側を使うこと。
- 1.3: 較正で台本の字句の誤りが 2 件出た（タスク本文を訂正済み）。⑴ `LinkEndpointMissing` は「相手 id の末尾の連番を**消す**」では出ない — `links.to`・`alias_of`・`supersedes` はすべて `ledger/read.rs` の `reference_id` → `model.rs` の `EntryId::parse` を通り、コロン数が合わずに**読み取り段**で止まる。「別の数字に変える」なら出る。⑵ `UnknownTheme` は台帳ファイル経由では**構造上到達しない** — 読み取りの `parse_theme` と検査の `CheckInput::themes` が同一の `THEMES` 定数。所見は道具の常時テスト `a_misspelled_theme_turns_red` が覆う。
- 1.3: 巻き添えで出る所見の型 — id を書き換えると `CatalogIdMissingFromLedgers` が付く。状態・テーマ・版番号を書き換えると `DomainReportStale` が付く。並び順・前置き・関連の書き換えでは付かない（報告がそれらを載せないため）。
- 1.3: **`MSYS_NO_PATHCONV=1` を立てること。** MSYS のパス変換が `//` で始まる引数の `\r\n` を `/r/n` に化けさせ、CRLF のファイルへの置換が黙って壊れる（`SourceUrlNotInCatalog` の較正で 1 度空振りした）。
- 1.3: 戻しは `git checkout -- <明示パス>` で行うこと。`git checkout .` / `git reset --hard` / `git stash` は禁止（stash スタックは他セッションと共有）。
- 1.4: **areka の SHIORI 送出経路は 2 系統ある。**⑴ スケジューラ起源＝`schedule/events.rs`／`schedule/resources.rs` の構築関数が `EventId::Static` の固定名（`ALLOWED_EVENT_IDS` の 11 件）を組み立てる系統。⑵ **選択起源**＝`schedule/steady.rs` の `CascadePlan::Named` が `on_choice_named` を通してゴースト作者が `\q` の ID に書いた名前を `EventId::Choice` として**逐語で運ぶ**系統で、受理規則 `is_allowed_choice_event` は「`On` 接頭であること」の 1 条件のみ。**「areka はこのイベントを送らない・ログが 1 行も出ない」と書けるのは ⑴ の系統に限る。** `On` 始まりの項目はゴーストが `\q` に書けば送出され `actor.rs` の trace `shiori_request` が 1 行出る（ただし正典の発火条件で発火したものではないので状態は `absent` のまま）。
- 1.4: `On` 始まりを含むのは 3 ページだけ — `list_shiori_event` 290 中 264（非 `On` は 26＝群 4 の 25＋`basewareversion`）・`list_shiori_event_ex` 168 中 **144**・`list_plugin_event` 19 中 **8**。リソース 159 と `spec_*`／`memo_*` の 7 ページ 40 は `On` 始まり 0 件。
- 1.4: `event_id_not_allowed` の `Choice` 側の腕は**本番では到達しない防御用**（`schedule/choice.rs` の `plan_cascade` が非 `On` を `Canonical` へ回すので `Choice` は必ず `On` 始まり）。索引はこの腕が働くとは書いていない。
- 1.4: 群の索引の**正本は `doc/ukadoc-coverage/briefing-shiori.md` の冒頭**、写しが台帳冒頭の `#` コメント 559 行。`check` は `#` を読まないので写しの陳腐化は機械では赤にならない。**更新は正本 → 写しの順で節の全体を貼り直すこと。**
- 1.4 → **タスク 2.2／2.3 への申し送り**: 群 2・群 4 の共通 `note` は根拠の場所（ファイル名と定義名）を含むが、設計は「`absent` の各行には根拠を繰り返さない」と定めている。要件 2.9 は群の共通 `note` に根拠を書くことを許しているので違反ではないが、248 行・25 行へ写すときに緊張が表面化する。**正確さを削ってまで縮めないこと。**
- 1.4 → **タスク 2.1 への申し送り**: 群 3（別名 3 件）の共通 `note` は「この行に固有のログは無い」とするが、別名 3 件も `On` 始まりなので `\q` 経由なら**別名の名前で** `shiori_request` が 1 行出る（写像先 `OnFileDrop2` の名前では出ない）。判定をしない行なので実害は無いが、群 2 と同じ但し書き 1 文を足せば揃う。
- 2.1: **`cargo test -p ukadoc-survey` はタスク 2.1 以降 7.1／7.2 が着地するまで赤のまま。これは構造上避けられない中間状態であって欠陥ではない。** 台帳に `implemented` の行が立った瞬間に `ImplementedWithoutEvidence` が付き、それを消す正典 URL コメントを置くタスク 6.1 は `_Depends: 2.1, 3.1_` で後ろにあるため。台帳の値を変えれば `DomainReportStale` も付き、報告を作り直すタスク 7.2 も後ろにある。**以後の各タスクの検証は「テスト緑」ではなく「`check` の所見が予測どおりの集合とちょうど一致する」で行うこと。**
- 2.1: 赤の内訳の読み方 — 失敗するのは `tests/consistency/checks.rs` のテスト。うち大半は `perturb.rs` の `expect_exactly`（所見集合を「含む」でなく「ちょうど」で比べる）が基底の所見に巻き添えになったもの、2 本（`real_repo_data_produces_no_findings`・`every_kind_of_finding_is_absent_from_real_data`）は基底の主張そのもの。**新しい種別の所見が 1 件でも増えたらそれは本物の欠陥。**
- 2.1: 別名の向きは**上流 要件 4.1 の第 1 段（正典本文の注記）**で決着。`OnFileDrop`／`OnFileDropped`／`OnFileDropEx` は本文が逐語で `[旧仕様]` 始まり、`OnFileDrop2` は「現時点での最新仕様」。第 2 段（版番号）は 4 件とも `2.7.98` で決め手にならない。`OnFileDropping` はドラッグ中の別機能で `[旧仕様]` の注記が無い。
- 2.1: `basewareversion` はカタログ全体で 1 件のみ（`list_shiori_event` の `[NOTIFY]`）、リソース側に同名は無い。要件 2.4 の「26 件」＝群 4 の 25 件＋本件 1 件。
- 2.1 → **タスク 5.1／5.2 への申し送り（埋め忘れの隙）**: 索引は群 2a に具体値（テーマ「装い」・優先度 `B4`）を定めているが、5.1 の完了条件（「8 語彙以外が現れないこと」）も 5.2 の完了条件（「**空でない行**が表の値と一致すること」）も**空欄を通してしまう**。機械検査にもテーマ未設定を赤にする種別は無い。**5.1／5.2 では群 2a の 3 行を明示的に確認すること。**
- 2.2: 群 2 は 248 行ちょうど（`list_shiori_event` 290 の分布は `absent` 248／`unclassified` 25＝群 4／`implemented` 11／`vocabulary-only` 3／`alias` 3）。共通 `note` は索引から一字一句写した（短縮しない — 1.4 の裁定どおり）。個別追記はちょうど 6 行。
- 2.2: 正典 290 と `doc/shiori/fragments/events/` 287 の差は **3 件・向きは一方向**（正典にあって断片に無い＝`OnArchiveViewerOpen`・`OnMediaPlayerOpen`・`OnPictureViewerOpen`。逆向きは 0 件）。
- 2.2: 内部名 3 つはカタログ全 1,749 件に完全一致 0 なので行を作っていない。`OnTalk` → `OnAITalk`、`OnHour` → `OnHourTimeSignal` の `note` へ。**`OnMenuBack` は恒久禁止の名前ではない** — `ALLOWED_EVENT_IDS` の doc コメントが恒久禁止とするのは `OnTalk`／`OnHour` の 2 つだけで、`OnMenuBack` は `msg.rs` の単体テストに「任意名が `EventId::Choice` に逐語で載る」見本として現れるだけ。要件 2.6 がこれを内部名に数えているのは要件側の不正確さ。
- 2.2 → **タスク 4.2（PLUGIN 19）への申し送り**: `OnMenuBack` は暫定的に `OnChoiceEnter` の `note` に置いたが、**より近い正典項目が `list_plugin_event` にある** — 「`\q` 等に指定された任意名イベント」を表題にした唯一の項目（現在 `unclassified`）。その行に相互参照を置くか、明示的に置かないと決めること。
- 2.2 → **タスク 4.3（`memo_shiorievent`）への申し送り**: 索引の群 15 の「判断の根拠の場所」にも内部名 3 つの扱いを書く旨が残っている。**二重に書かないこと。** なお群 15 の文面「恒久的に含めない旨を写す」が当たるのは `OnTalk`／`OnHour` だけ。
- 2.2 → **タスク 7.1／7.3 への申し送り**: 群 2 の 248 行の `owner` は `""` のまま。付録 A は `""` を正当な値と定めており本タスクの要件にも `owner` は含まれないが、要件 12.3（既存 spec が所有する項目に `owner` を書く）を 248 行へ適用するタスクが tasks.md に無い。**248 行に `owner` が要らないことが意図的かを確認すること。**
- 2.2: 台帳は 9,869 行に膨らんだが、1,000 行の見張り（`crates/log-capture-kit/tests/file_length_guard_test.rs` ＋ `tests/workspace_scan/mod.rs`）は **`crates/` 配下の `.rs` だけ**を列挙するので対象外。
- 2.3: 群 4 は 25 行ちょうど（`list_shiori_event` の非 `On` 26 件から `basewareversion` を除いた分）。これで **`list_shiori_event` 290 の未分類が 0** になった（`absent` 273／`implemented` 11／`vocabulary-only` 3／`alias` 3）。
- 2.3: 送信の向きは正典本文で全件裏が取れた — `[NOTIFY]` の明示がある通知 22 件／引く側 2 件（`inputbox.autocomplete`・`property.get`）／値を渡して書かせる側 1 件（`property.set`）。
- 2.3: 索引の共通 `note` にある `向き:`／`種別:` の行は**書き手への指示文**であって写す本文ではない。具体の記述に置き換えるのが正しい（2.1 が `basewareversion` で採った形）。
- 2.3 → **タスク 5.2 への申し送り（完了条件を強化済み）**: 索引が具体値を定めているのに空欄のまま残りうるのは**群 2a の 3 行（`B4`）だけ**。群 1 の `priority = ""` は索引・設計とも意図的なので隙ではない。
- 2.3: **バックスラッシュを含む文字列はヒアドキュメントで書かないこと。** ヒアドキュメントが 1 段落として TOML が読めなくなる（`\!` → `\!`）。`chr(92)` で組み立てるか、台本をファイルに書いてから実行すること。
- 2.4: `OnUpdate` 系は 26 行ちょうど。テーマ `更新`・優先度 `B1`（設計 DD-11 の表）を全 26 行に置き、小群を示す 1 文を共通 `note` の末尾に足した。小群は正典本文で 1 件ずつ振り分けて **11＋8＋2＋5＝26** を実測。
- 2.4: **正典との呼称の食い違いを訂正した。** `OnUpdateOther*` 8 件の正典本文はすべて「**ゴースト以外の**…」で、「他のゴースト」の語は 1 件も無い（`OnUpdateReady` の `Reference3` が対象種別を `(shell ghost balloon headline plugin)` と列挙）。design.md 4 か所・tasks.md 2 か所の「他ゴースト更新」を「ゴースト以外の更新」へ改めた。requirements.md は「Other 系は 8」としか書いておらず汚染されていなかった。**関係の本数 8 本と書く側（本体側）は変わらない。**
- 2.4: 点検 2 と結果 5 が分かれる根拠 — 点検（`OnUpdateCheckComplete`／`Failure`）は 1 回の点検の成否を伝える単発通知、結果（`OnUpdateResult` 系 5 件）は `OnUpdateResult` と同じ一括結果リストの `Reference*` を共有する。
- 2.4 → **タスク 5.1 への申し送り**: 要件 7.3（テーマを付けた行に「無いと利用者が失うもの」を 1 文）はこの 26 行で未充足。5.1 の完了条件は総称形なので抜けはないが、機械検査に赤にする種別が無いので目視で確かめること。あわせて小群の 1 文の語を正典へ寄せること — ⑴ 本体更新 11 の文「ゴースト本体そのものを新しくする流れ」は `OnUpdatedataCreating`（ゴーストフォルダの DnD）と `OnUpdatedataCreated`（`updates2.dau` の作成）には厳密には当たらない（更新データを**作る側**の出来事）。⑵ 結果 5 の文「一度の更新で起きたことを…」は `OnUpdateCheckResult`／`Ex`（更新チェックのみ）には当たらない（更新は実行されていない）。
