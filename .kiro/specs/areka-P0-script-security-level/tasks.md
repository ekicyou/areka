# 実装計画

- 決めごとの正本は `design.md` と `research.md` §8・§9。決定論のテストはすべて x64 の偽の境界で書き、テストは実装と同じディレクトリの兄弟ファイルへ置く。どのファイルも 1,000 行以内に収める。ログの無い失敗の経路を作らない。
- `crates/areka/src/emo2_boot/consumer_ledger.rs` は読むだけで、どのタスクでも編集しない（並走のウェーブとの約束）。
- 記録の種別の表示（`Ghost`・`SSTP(Local,Auth)` など）・`ActiveTalk::origin`・`StrictNote` は変えない（要件 5.1・research §8.2 の 9）。

- [ ] 1. 出どころの語彙と台本の起動の約束
- [x] 1.1 出どころの型を依存 0 のクレートに置く
  - `SenderType` の 9 語を「先頭の 2 語」と「残りの 7 語」に分けて持ち、先頭が必ず 1 つ在ることを型で保つ。Owned かどうかは SSTP 系の出どころだけが持つ 3 値、セキュリティレベルは `local`／`external` の 2 値で表す。
  - ゴースト自身・MCP の SEND・MCP の NOTIFY の 3 つの定数、語のつなぎ方（`internal`／`external` を先頭にカンマでつなぐ）、大文字小文字を区別しないレベルの読み、下げるだけの重ね方、「外部扱い」と「SSTP 系か」の判定を用意する。
  - 兄弟のテストで、つなぎ方（`internal`・`external,sstp`・`internal,raise`）・レベルの読み（`LOCAL`・`External`・前後の空白・読めない値で無し）・重ね方の 4 通り（`external` から上がらない）・3 定数と Owned でない SSTP の判定の真理表が緑になる。
  - 再生の層の窓口から同じ型が再輸出され、`areka` のクレートから直接の依存を足さずに使える。
  - _Requirements: 1.8, 2.7, 3.2, 3.3_

- [x] 1.2 台本の起動の約束に出どころの欄を足す
  - 台本 1 つの起動の約束に出どころの欄を足し、既定の作り方はゴースト自身を入れて製品の呼び手の振る舞いを据え置く（`epilogue` の既定と同じ先例）。
  - 再生の層は欄を運ぶだけで解釈しない（分解の追随のみ）。
  - 約束を字面で組むテスト（約 85 か所）を追随させ、ワークスペース全体がビルドでき、既存のテストが緑のままになる。
  - _Requirements: 1.4_

- [ ] 2. SHIORI の線の書き読み（32bit の仲介のクレート）
- [x] 2.1 追加のヘッダの入れ物と要求の文の並びを変える
  - 解釈しない文字列の 4 つの欄（`SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`）を持つ入れ物を足し、要求の構造の送り手の欄をこの入れ物に替える。今の既定（`local`・`null`・送り手の名前・`internal`）を作る手を用意する。
  - 要求の文を「要求行・`Charset`・`SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`・`Status`（在るとき）・`ID`・`Reference*`」の順に書き、末尾に固定で書いていた `SecurityLevel: local` を消す。`Charset`・`Status`・`ID`・`Reference*`（欠番の印を含む）の書き方は変えない。
  - 同じプロセスの経路の入力の組み立て（`areka-ghost` の `build_input`）も要求の構造を組むので、ここで暫定の `RequestHeaders::internal("areka")` へ直し、2.1 の後もワークスペース全体がビルドできるようにする（共通の組み立ての手へ移すのは 4.1）。
  - 要求の文の字面を固定したテスト（クレートの中・文字コードのテスト・kanade の翻訳のテスト）を新しい並びへ書き換え、`Status` の有無の 2 通りで `SecurityLevel` が `ID` より前に出ることを確かめるテストが緑になる。
  - _Requirements: 2.1, 2.9_

- [x] 2.2 応答の `SecurityLevel` の生の値と、2 つの経路が共に使う組み立ての手
  - 応答の読み取りが `SecurityLevel` の頭を名前の大文字小文字を無視して拾い、前後の空白を落とした生の値（無ければ無し）を返す。`Value`・`ErrorLevel`・`ErrorDescription` の読み方と判定の順は変えない。
  - Value つきの応答（値と生のレベル）の型と、要求の組み立ての共通の手を足す。32bit の仲介の利用者にヘッダつきの GET・NOTIFY を足し、今の GET・NOTIFY はゴースト自身の既定のヘッダで新しい手を呼んで値だけを返す。誤りの種類は増やさない。
  - 兄弟のテストで、生の値（大小無視の頭の名前・無いときの無し）と、`Value`／`ErrorLevel` の読みの不変と、共通の手から組んだ要求の文のバイト列が緑になる。
  - _Requirements: 2.1, 2.8, 3.5_

- [ ] 3. kanade の要求の出どころと応答のレベル
- [x] 3.1 SHIORI の受け手の約束に既定の実装つきのメソッド 2 つを足す
  - ヘッダつきの GET（既定はヘッダを使わず今の GET を呼び、応答のレベルは無し）と、ヘッダつきの NOTIFY（既定は今の NOTIFY を呼ぶ）を足す（`on_idle` と同じ既定の実装の先例）。
  - 32bit の仲介の本物の受け手が 2 つを上書きし、ヘッダを線へ渡して応答の生のレベルを返す。
  - テストの偽物と中身なしの受け手は既定のまま今と同じに動き、ワークスペース全体が緑のままになる。
  - _Requirements: 2.1, 2.8, 3.1_
  - _Depends: 2.2_

- [x] 3.2 要求の出どころの規則と、翻訳の要求のカテゴリ
  - 送り手の名前（`areka`／`MCP`）と出どころの組を持ち、送るイベントの出所カテゴリから要求の出どころ・応答の台本の基本の出どころ・翻訳の要求の出どころを導く規則と、4 つのヘッダの文字列への写し（`SecurityOrigin` は常に `null`）を置く。32bit の仲介の型は使わず、カテゴリの `match` に既定の腕を置かない。
  - 応答の生のレベルの解釈を置く: 無い → `local`／`local`・`external`（大小無視）→ その値／それ以外 → イベントの ID と値を warn（`response_security_level_unreadable`）に残して `local`。
  - イベントの出所カテゴリに「翻訳（元の要求の出どころを抱える）」を足し、名前は `OnTranslate`、受理の判定・欠番の印・記録の名札は今の `OnTranslate` と同じに扱う。翻訳の要求は 5.1 まで文字列のカテゴリ（`Static("OnTranslate")`）で作られるので、欠番の印を通す判定（`actor.rs` の `replace_absent_markers`）は `Static("OnTranslate")` と翻訳のカテゴリの両方を受け、記録の名札の `match` と受理の判定の `match` にも翻訳のカテゴリの腕を足す。
  - 兄弟のテストで、design の写しの表の 5 行（運行表・選択肢・アンカー／`raise_event`／`sakurascript` の翻訳／ゴースト自身の応答の翻訳／`raise_event` の応答の翻訳）の値と、4 つのカテゴリの基本の出どころと、レベルの解釈の 4 通り（読めない値で warn が 1 件）が緑になる。
  - _Requirements: 1.1, 1.3, 1.7, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 3.3, 3.4_
  - _Depends: 1.1_

- [x] 3.3 SHIORI の結果の台詞に応答のレベルを載せる型の変更
  - 結果の台詞の腕を「台詞と実効のセキュリティレベルの組」に変え、今の作り手はすべて `local` を入れる（この時点では振る舞いを変えない）。
  - 意味のある読み手（翻訳の預かりの `before`・台詞を照らす `read_reply`）では 2 つ目の欄を無視する形（`Value(s, _)`）にとどめる。レベルを実際に読むのは 5.2・5.3。
  - 字面の追随（約 133 か所・42 ファイル）をコンパイラが挙げる場所すべてで済ませる。大きいファイル（運行表の常駐の腕・957 行）は型の追随だけで行を増やさない。
  - ワークスペース全体がビルドでき、既存のテストが緑のままになる。
  - _Requirements: 3.1_

- [x] 3.4 SHIORI の呼び出しが入れ物を渡し、応答のレベルを結果へ載せる
  - GET・NOTIFY の両方で、要求の出どころの規則から入れ物を作ってヘッダつきのメソッドだけを呼ぶ。台詞の応答では生のレベルを解釈して結果に載せる。204・誤りの扱いと NOTIFY の応答を読まない扱いは今のまま。
  - 送る直前の trace の記録 `shiori_request` に `sender`・`sender_type`・`security_level` を足す（同じ規則から導く）。
  - ヘッダを記録する偽の受け手を SHIORI の回り役に挿す兄弟のテストで、運行表・選択肢・`raise_event`・翻訳の 4 つのカテゴリの要求で渡った入れ物が要件の値と一致し、偽の受け手が `external` を返すと結果のレベルが `external` になることが緑になる。
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 3.1, 3.4_

- [ ] 4. 同じプロセスの経路と、中身へ渡す包み
- [x] 4.1 同じプロセスの SHIORI の受け手と、中身へ渡す包み 4 つが新しいメソッドを通す
  - 同じプロセスの受け手が 2 つのメソッドを上書きし、要求を 32bit の経路と同じ共通の手（UTF-8）で組む。GET の結果の写しは応答の生のレベルも返す。HRESULT と誤りの写しは変えない。今の GET・NOTIFY は `areka` の既定のヘッダで新しい手を呼ぶ。
  - 中身へ渡す包み（記録の包み 1 つとテストの包み 3 つ）が、新しい 2 メソッドも中身へそのまま渡す（既定のままにすると本物の pasta を通すテストでヘッダと応答のレベルが黙って落ちるため）。
  - 兄弟のテストで、同じヘッダから同じプロセスの経路が組んだ入力と、共通の手から組んだ要求の文のバイト列が一致し、結果の写しが生のレベルを返すことが緑になる。本物の pasta を通す既存のテストも緑のままになる。
  - _Requirements: 2.8, 3.1_
  - _Depends: 2.2, 3.1_

- [ ] 5. 運行表のトークの出どころ
- [x] 5.1 翻訳の要求を元の要求の出どころから作る
  - 応答の台本の翻訳は「翻訳（元のイベントの要求の出どころ・送り手 `areka`）」、外からの台本の翻訳は「翻訳（MCP の SEND・送り手 `MCP`）」のカテゴリで送る。`Reference*` の並びは変えない（ゴースト自身と `raise_event` の応答の翻訳の `Reference1` は今どおり欠番、外からの台本は `sstp-send,owned`）。
  - 翻訳の要求を文字列のカテゴリで確かめていたテスト 4 か所を追随させる。3.2 で欠番の印の判定に残した `Static("OnTranslate")` の受け入れを外し、翻訳のカテゴリだけにする。
  - 翻訳の回り役の兄弟のテストに、ゴースト自身・`raise_event`・外からの台本の 3 つで翻訳の要求の出どころが要件 2.4〜2.6 のとおりになる確かめを足し、緑になる。
  - _Requirements: 1.5, 2.4, 2.5, 2.6_
  - _Depends: 3.2_

- [x] 5.2 翻訳の応答のレベルでトークを下げる
  - このタスクが持つのは翻訳の応答のレベルだけ（`read_reply`・翻訳の結果・`Translated`・翻訳の完了・`on_done`）。元の応答のレベルを運行表の預かりの応答（`translate::Replied`）に持たせるのは 5.3。
  - 翻訳の結果（台詞とレベルの組）が翻訳の応答のレベルを運ぶ。204・誤り・起きない結果は展開済みの元の台詞と `local`。MAKOTO の口は今どおり台詞だけを受ける。
  - 翻訳が終わって預けた台本の台詞を差し替える所で、出どころを翻訳の応答のレベルで下げる（上げない）。`external` に下がったら info（`script_security_lowered`・`talk_id`・`by=translate`）を 1 件残す。
  - 翻訳の完了の字面（約 30 か所）を追随させ、翻訳の応答の `external` で下がり `local` で上がらないことを確かめるテストが緑になる。
  - _Requirements: 1.5, 3.1, 3.2_
  - _Depends: 3.3_

- [ ] 5.3 SHIORI の応答で始まるトークと外からの台本に出どころを付ける
  - 運行表の一歩の出口（振り分けの後・翻訳へ預ける前）で 1 回、台詞の応答のときだけ、一括の中のすべての台本の起動に「元のイベントの基本の出どころを応答のレベルで下げた値」を付ける。腕（起動・切替・終了・常駐）は触らない。元のイベントが分からないときは腕の値のまま（記録は今の `translate_source_missing`）。`external` に下がったら info（`by=response`）を 1 件残す。
  - 運行表の預かりの応答（`translate::Replied`）が元の応答のレベルを持ち、台詞の応答のときだけ出どころを出す手（`reply_origin()`）を持つ。これは 5.3 の持ち物。
  - 台本の起動の手に出どころを足し、呼び手 3 つをすべてここで直す: 外からの台本は MCP の SEND、外から頼まれたイベントの台本は MCP の NOTIFY、選択肢の `script:`（`steady_choice_script::begin`）は 5.4 が引き継ぎの値に替えるまで仮にゴースト自身を渡す。
  - 新しい兄弟のテストで、運行表の一歩を通して ⑴ ゴースト自身のイベントの応答はゴースト自身 ⑵ `raise_event` の応答は MCP の NOTIFY ⑶ 外からの台本は MCP の SEND ⑷ 応答の `external` で下がり翻訳の応答の `local` で上がらない ⑸ 翻訳の応答の `external` で下がる、が緑になる。運行表のファイルは 1,000 行以内に収まる。
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 3.1, 3.2_

- [ ] 5.4 選択肢の `script:` の引き継ぎと、選択肢の ID・アンカーのイベント
  - 最後に再生を始めたトークの控えに出どころを足し、控えを写す所が台本の起動の最終の値（翻訳の後）を写す。控えの字面（7 か所）を追随させる。
  - 5.3 で仮にゴースト自身を渡していた選択肢の `script:` の呼び手を、引き継ぎの値に替える。選択肢の `script:` で始まる台本は、控えのトークの番号が元のトークと一致すればその出どころを写す。一致しない（構造上は起きない）ときは error（`choice_script_origin_missing`）を残して外部扱いへ倒す。
  - 兄弟のテストで、⑹ 選択肢の `script:` は元のトークの値（`external` を含む）を写す ⑺ 選択肢の ID のイベントの応答は元が外部扱いでもゴースト自身、が緑になる。
  - _Requirements: 1.6, 1.7_

- [ ] 6. 配送と受け口のタグの止め
- [ ] 6.1 配送が受け口へトークの出どころを渡す
  - 受け口の約束の「複製の直後に 1 回」の手に出どころの引数を足し（既定は何もしない）、配送はトークの起動の出どころをすべての複製へ渡す。
  - 上書きしているバルーンの寿命の受け口とテスト（計 7 か所）は引数の追随だけで、出どころを使わない。
  - ワークスペース全体がビルドでき、既存の配送と受け口のテストが緑のままになる。
  - _Boundary: 配送（areka-ghost の sink・dispatcher）・バルーンの寿命の受け口（areka の emo2_boot の talk_lifecycle）_
  - _Requirements: 1.4_
  - _Depends: 1.2_

- [ ] 6.2 出どころで制限されるタグの判定の表
  - ukadoc が出どころで実行を制限する 5 つのタグ（`\![execute,filewatch,…]`・`\![execute,schedule-add,…]`・`\![execute,schedule-delete,…]`・`\![enter,nouserbreakmode]`・`\m[umsg,wparam,lparam]`）を、区分（外部扱いでは実行しない／SSTP 専用）・ukadoc の注記の要旨・出典の URL・実装済みかの旗とともに 1 つの表に持つ。実装済みは `\![enter,nouserbreakmode]` だけ。`\![leave,nouserbreakmode]` は載せない。
  - `\!` の名前と選別子から行を引く手と、行と出どころから許すかを決める手を置く（外部扱いでは実行しない → 外部扱いでないとき許す／SSTP 専用 → SSTP 系のとき許す）。
  - 兄弟のテストで、表がちょうど 5 行で綴りが要件 4.1 の 5 つと一致すること（違えば「ukadoc を引き直して表を直す」の文で赤）と、許すかの真理表が緑になる。
  - _Requirements: 4.1, 4.5, 4.6, 4.8_

- [ ] 6.3 ユーザーブレイク禁止の受け口が外部扱いのトークで enter を止める
  - 一括の複製の実装をやめて受け口の約束を自分で実装し（複製は今と同じく状態を引き継がない）、複製の直後の手でトークの番号と出どころを控える。登録の原本はゴースト自身。
  - 自己選別で `\![enter,nouserbreakmode]` に当たったら判定の表を引き、許されなければ合図を送らずに warn（`script_tag_refused`・表の綴り・`talk_id`・`sender_type`・`sstp`・`level`）を 1 件残して戻る。台本の残りはそのまま流れる。トークの始まり・`leave`・トークの終わりは今どおり送る。
  - 既存の受け口のテストの複製の追随（5 か所）を済ませ、新しい兄弟のテスト（記録の捕捉は `log-capture-kit`）で、外部扱いを渡した複製は `enter` を送らず warn が 1 件・ほかの合図は今どおり、ゴースト自身・MCP の SEND・MCP の NOTIFY では今どおり `enter` が出る、が緑になる。受け口の名簿のファイルには触らない。
  - _Requirements: 4.2, 4.3, 4.4, 4.5_
  - _Depends: 6.1, 6.2_

- [ ] 6.4 判定の表と実装の食い違いをフルテストで赤にする照合
  - `\!` の行は、受け口の名簿の担当の有無と表の実装済みの旗が一致することを照らす。`\m[…]` の行は、台本の読みで `\m[umsg,0,0]` が未解釈の命令になること（未実装）と旗が偽であることを照らす。名簿は読むだけ（先例 `consumer_ledger_agreement_tests.rs`）。
  - 旗だけで済ませず、実装済みの行ごとに名簿が返す担当の組から受け口を作り、外部扱いの出どころを渡してその行のタグを流し、そのタグの合図が出ないことを確かめる。担当の組から受け口を作る `match` には既定の腕を置かない。
  - 照合のテストが緑になり、旗を偽へ倒すか止めを外すと赤になることを手元で一度確かめる。
  - _Requirements: 4.6, 4.7_

- [ ] 7. 通しの確かめと記録
- [ ] 7.1 偽の SHIORI で起動の道を通す通しのテストと、全体の回帰
  - 起動の通しのテストと同じ組み方（偽の受け手と、ユーザーブレイク禁止の受け口）で起こし、テストのファイルの中で台本つきの偽の受け手を包んでヘッダつきの GET だけを上書きし（共有の偽物のファイルは触らない）、`OnBoot` の台本 `\![enter,nouserbreakmode]…` を返す。
  - ⑴ 応答の `SecurityLevel: external` つきでは線に `enter` が出ない ⑵ `local` または無しでは今どおり `enter` が出る、が緑になる（kanade → 台本の起動 → 配送 → 受け口の運ぶ道が既定の値に落ちていない）。
  - ワークスペースの全体テスト（記録の種別の表示・`Status`・翻訳・MCP の 2 つの道・1 ファイル 1,000 行の番人を含む）が緑のままになる。
  - _Requirements: 1.4, 2.9, 3.1, 3.5, 4.2, 4.3, 5.1_
  - _Depends: 5.3, 6.3_

- [ ] 7.2 本物の起動の道での実機の確かめ
  - 窓口 `real-run-kit`（手引き `doc/testing.md`・先例 `crates/areka/tests/mcp_get_log_real_run.rs`）で `crates/areka/tests/script_security_real_run.rs` を書き、検体 `emo2` を `RUST_LOG=warn,kanade=trace,areka::input_events::user_break=debug` で起こす。記録は子の標準出力から読む（`get_log` は debug の行を残さないことがあるため）。
  - 記録を読んで ⑴ MCP の `raise_event` の `shiori_request` が `sender=MCP`・`sender_type=external,sstp`・`security_level=local` ⑵ MCP の `sakurascript` の `OnTranslate` が `sender=MCP`・`sender_type=internal` ⑶ 起動の道のゴースト自身のイベントが `sender=areka`・`sender_type=internal` ⑷ MCP の `sakurascript` で流した `\![enter,nouserbreakmode]` が今どおり区間に入る（証拠は `crates/areka/src/input_events/user_break.rs` の debug の記録 `no_user_break_changed`・target `areka::input_events::user_break`）、を確かめて閉じる。
  - 本物の pasta.dll を同じプロセスの経路で通したテストが緑になる。決定論のテストの代わりにはしない。
  - _Requirements: 1.2, 1.3, 2.2, 2.3, 2.4, 2.8, 4.3_
  - _Depends: 4.1, 6.3_

- [ ] 7.3 (P) 網羅台帳と沈黙ルール対応表を実装の状態に合わせる
  - `doc/ukadoc-coverage/ledger/shiori.toml` の要求の `SecurityLevel`・`SecurityOrigin`・`SenderType` と応答の `SecurityLevel` の 4 行を、実装の状態と持ち主（本 spec）に合わせて書き換える（`SecurityOrigin` は「今は常に `null`・SSTP の受信の spec が値を持つ」と注記）。群 10・11 の説明の行も合わせる。
  - `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![enter,nouserbreakmode]` の行に「外部扱いの台本では合図を送らず warn（本 spec）」、`\m[umsg,wparam,lparam]` の行に「SSTP 専用・判定の表に載せた（未実装のため扱いは変えない）」を書く。
  - `doc/COMPAT_ARCHITECTURE.md` §8 の沈黙ルール対応表に、`Sender: areka`・選択肢の ID とアンカーのイベントをゴースト自身とする・応答の `SecurityLevel` でレベルを上げない・`SecurityOrigin` を常に `null` とする・読めない応答のレベルを無いものとする、の 5 つを登記する。
  - 台帳 2 つを書き換えた後に `cargo run -p ukadoc-survey -- report`・`cargo run -p ukadoc-survey -- report-summary`・`cargo run -p ukadoc-survey -- check` を回す。
  - 作り直した報告の差分がコミットされ、網羅台帳のテストが緑になり、3 つのファイルの差分に上の行がすべて現れる。
  - _Boundary: 網羅台帳・COMPAT_ARCHITECTURE.md_
  - _Requirements: 5.2, 5.3, 5.4_
  - _Depends: 6.4_

## Implementation Notes

- （範囲外・1.2 で発見）`crates/areka-kanade/src/actor_raise_reply_tests.rs:95` に clippy の `never_loop` の誤りが既にあり、kanade の lib test の clippy を落とす。
- （範囲外・1.2 のレビューで発見）`areka-sakura` の `drive::delivery_tests::same_sheet_started_at_different_times_delivers_cue_at_different_absolute_fire_times` が有界の `try_recv`／`yield_now` の空回しで時間に依存し、まれに赤になる。
- `cargo test -p shiori-host32-host` は先に `cargo build -p shiori-host32-testdll -p shiori-host32-helper --target i686-pc-windows-msvc` が要る（無いと統合テスト 1 本が前提不足で赤）。
- `crates/areka-kanade/src/actor_tests.rs` は 3.4 の後 983 行。これ以上足すなら兄弟のテストのファイルへ。
- （範囲外・4.1 のレビューで発見）`crates/areka-ghost/tests/ghost/inproc_e2e_test.rs` は main で既に 1,135 行（行数の番人の許可表に載っている）。同じ `BoxedBackend` の写しが 3 つのテストのファイルにある。
