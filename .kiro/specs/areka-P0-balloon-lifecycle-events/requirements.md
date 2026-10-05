# Requirements Document

## Project Description (Input)
**誰が困っているか**: 既存のゴーストを areka で動かす利用者と、そのゴーストの作者。話の途中でバルーンをダブルクリックして止めても、読み終えたバルーンを閉じても、放っておいてバルーンが時間切れで消えても、ゴーストはそれに気付けない。SSP ではこの 3 つの場面で SHIORI イベント `OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout` が届くので、「途中で切られた」「読んでくれた」「読まれないまま消えた」に応じて喋る辞書が動く。areka ではどれも呼ばれない。

**今どうなっているか**: 完了 spec `areka-P0-balloon-visibility` が 3 つのイベントの語彙と Reference の割り当て、表示の側から会話の進行の側へ渡す情報の型（`BalloonLifecycleNotice`）だけを残し、送る仕組みは作らなかった。その型は「作る側も受け取る側も無い予約」の印を付けたまま残っている。利用者の中断（バルーンの左ダブルクリック）は完了 spec `areka-P0-balloon-break` が会話の進行の側まで届けるようにしたが、SHIORI へは何も送っていない（「どのバルーンで起きたか」の番号は記録に載るだけ）。時間切れでバルーンを隠す判断は表示の側にあり、それを会話の進行の側へ伝える口が無い。また、中断で終わった会話では、時間切れまでの計測の起点が「中断までに表示へ配られた演出の終わりのうち最も遅いもの」になる。中断の時点で進行中だった演出（待機など）の残りの長さだけ、バルーンが余分に残る（今ある中断の種類では目に見える差はほとんど出ないが、時刻を正確に扱う原則に合わせて直す）。網羅の台帳では 3 つのイベントは「語彙のみ」。

**何を変えるか**: 3 つのイベントを正典（ukadoc）の Reference で SHIORI へ送る。中断で終わった会話の時間切れの計測は、中断が起きた時刻から始める。予約の印を外し、互換対応表と網羅の台帳を実際の振る舞いに合わせる。`\![set,balloontimeout,時間]` の表示の側（項目 7）は、台本のコンパイルの側を持つ `areka-P0-sakura-time-directives` がまだ着手前なので、本 spec からは切り離す（詳細は brief.md）。

## Introduction
バルーンが「中断された」「閉じられた」「時間切れで消えた」ことを、正典の SHIORI イベントでゴーストへ知らせる。送る時機は他のイベントと同じ「定常の間だけ」の決まりに従い、送った応答の台本は他のイベントの応答と同じように新しいトークとして再生する。

正典の根拠（ukadoc `list_shiori_event.html`）:
- [OnBalloonBreak](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonBreak)（「SSTP以外でブレイクされた際に発生。」）: Reference0＝中断の操作が起きたスクリプト／Reference1＝中断の操作が起きたバルーンのスコープ番号（本体側 0、相方側 1、それ以降も）／Reference2＝中断位置（スクリプト先頭からの文字数。さくらスクリプトのタグも含めて数える）。
- [OnBalloonClose](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonClose)（「バルーンを閉じた際に発生。」）: Reference0＝閉じる際に表示されていたスクリプト。
- [OnBalloonTimeout](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonTimeout)（「選択肢以外でバルーンがタイムアウトした際に発生。」）: Reference0＝タイムアウトした際に表示されていたスクリプト／Reference1＝残り時間。
- 選択肢の時間切れは別のイベント `OnChoiceTimeout` で、既に送っている（本 spec は触らない）。

### 用語
- **トーク**: SHIORI の応答の台本を 1 本再生すること。起動の挨拶・定常のトーク・別れの台詞を含む。
- **台本**: そのトークとして再生した さくらスクリプト。`OnTranslate` で書き換えたときは書き換えた後のもの（既に送っている `OnChoiceTimeout` の Reference0 と同じもの）。
- **利用者の中断**: 完了 spec `areka-P0-balloon-break` が定めた操作。出ているバルーン（シェルの中のバルーンを含む）を左ダブルクリックすると、再生中のトークがその場で止まり、出ている全スコープのバルーンが即座に隠れる。再生中のトークが無いときは、バルーンを隠すだけ。
- **時間切れ**: 完了 spec `areka-P0-balloon-visibility` が定めた、会話の表示が終わってから既定の待ち時間（30 秒）が過ぎたら出ている全バルーンを隠す働き。ドラッグ・ポインタの滞在・選択肢の表示の間は抑止される。
- **占有区間の終端**: 待機（`\w` など）を含めて、表示へ配られた演出の終わりのうち最も遅い時刻。最後まで流れたトークでは台本の終わりの時刻になる。中断で終わったトークでは、中断の時点で進行中だった演出の終わりまで含む（配られなかった残りは含まない）。今の時間切れの計測はここから始まる。
- **定常**: ゴーストが起動を終え、終了や切替の途中でない状態。トークを再生中の定常と、再生中のトークが無い定常がある。
- **中断で終わったトーク**: 最後まで流れずに止められて終わったトーク。利用者の中断のほか、選択肢の時間切れでの解除などがある。

## Boundary Context
- **In scope**:
  - `OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout` の組み立てと送出、SHIORI へ送ってよいイベントの表への 3 行（ukadoc の URL の注記つき）。
  - 時間切れでバルーンを隠したことを、表示の側から会話の進行の側へ伝える口。
  - 中断で終わったトークの時間切れの計測の起点を、中断が起きた時刻にすること（brief の項目 10）。
  - 予約の印（`BalloonLifecycleNotice` の「消費者ゼロ」の注記と警告の抑止）を外すこと。
  - 互換対応表（`doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表）と網羅の台帳（`doc/ukadoc-coverage/ledger/shiori.toml` の 3 行・`sakura-script.toml` の `balloontimeout` の行の持ち主）とその生成物の更新。
  - 決定論のテストと、実機での確認。
- **Out of scope**:
  - **中断位置（`OnBalloonBreak` の Reference2）の源を作ること**。台本の中の位置を、字句から台本のコンパイル・再生・完了の知らせまで通す工事が要る。本 spec では空で送り、縮退として登記する。位置を通す工事は `areka-P0-balloon-break-position` が持つ（要件 6.2・討議で確定する点 A）。
  - **`\![set,balloontimeout,時間]` の表示の側（brief の項目 7）**。台本のコンパイルの側を持つ `areka-P0-sakura-time-directives` が着手前なので、本 spec から切り離す（要件 6.5・討議で確定する点 C）。台本にこのタグがあっても、今と同じく既定の待ち時間のまま。
  - `\x`／`\x[noclear]`（クリック待ち）。`areka-P0-talk-fast-forward` が持つ。
  - 利用者の中断そのものの規則（ダブルクリックで止まること・即座に隠れること・中断を禁じる区間・終了の予約の扱い）。完了 spec `areka-P0-balloon-break` の約束をそのまま使い、変えない。
  - 時間切れの判断の規則（既定 30 秒・抑止の条件・抑止が解けた後の計り直し・バルーンが現れる契機）。完了 spec `areka-P0-balloon-visibility` の約束をそのまま使い、変えない（起点だけを要件 5 で変える）。
  - 選択肢の時間切れ（`OnChoiceTimeout`）と、その時間の指定（`\![set,choicetimeout]`）。
  - SSTP による中断（areka には SSTP が無い）・バルーンを閉じるためのショートカットキーなど、areka にまだ無い操作。
  - パッシブモードでの抑え（areka にはパッシブモードがまだ無い）。
- **Adjacent expectations**:
  - 利用者の中断が「どのスコープのバルーンで起きたか」の番号とともに会話の進行の側まで届くこと、再生中のトークが無いときの中断の合図も会話の進行の側まで届くことは、完了 spec `areka-P0-balloon-break` の仕組みをそのまま使う。
  - SHIORI へ送ってよいときの決まり（定常の間だけ・終了の握手の待ちや切替の途中は送らない・SHIORI との往復は一度に 1 つ）と、応答の台本の扱い（`OnTranslate` を通してから新しいトークとして再生する）は、既にある決まりをそのまま使い、3 つのイベントのために例外を作らない。
  - SHIORI の失敗の扱い（エラー応答は致命でない・輸送路の失敗はそのゴーストの SHIORI の故障）は完了 spec `areka-P0-shiori-fault-notice` の決まりをそのまま使う。
  - `areka-P0-sakura-time-directives`（項目 7 の引き取り先の候補）・`areka-P0-talk-fast-forward`（会話の進行の運行表を触る見込み）・`areka-P0-anchor-tag-canon`（同じイベントの表と運行表を触る）とは同時に進めない。
- **開発上の制約**（同じウェーブ C4 の約束を含む。破るなら止めて報告）:
  - 会話の進行（kanade）の `msg.rs`・`actor.rs` に触らない（`areka-P0-mcp-get-status` の持ち物）。定常の振り分けのファイル（`schedule/steady.rs`・947 行）にも触らない見込みで、触る必要が出たら止めて報告する（`areka-P0-choice-script-prefix` が同じウェーブで触る）。
  - 運行表の本体のファイル（`schedule/mod.rs`・938 行）が本 spec の変更で 1,000 行を超えるなら、先頭のタスクで振る舞いを変えずに分ける。60 行を超えて足すなら、足す分を新しいファイルへ置く。
  - `crates/areka/src/emo2_boot/spine.rs` に触らない（`areka-P0-ghost-session-test-load-flake` の持ち物）。
  - 1 ファイル 1,000 行未満を保つ。1 フレーム遅らせる解は取らない。決定論のテストは x64 の偽の境界で組み、時刻は注入して駆動する。実機の根と一時フォルダはワークツリーの `target\` の下だけに置く。

## 討議で確定する点
要件の討議で開発者と決める。下の要件は、括弧内の既定（brief の推しか、軽い側）で書いてある。答えが既定と違えば、該当する要件を書き直す。

- **A. 中断位置（Reference2）を今作るか**（要件 1.4・6.2）——**確定（2026-10-05 開発者裁定「今は作らない。起票を行え」）**: 空で送り、互換対応表と網羅の台帳に縮退を登記する。位置を通す工事は `/kiro-discovery` で起票した `areka-P0-balloon-break-position`（その他・台本のコンパイルの列の最後）が持つ。本 spec の規模は M のまま。
- **B. areka で `OnBalloonClose` が起きる場面**（要件 3）——**確定（2026-10-05 開発者裁定・案 1）**: **再生中のトークが無いときに、出ている普通のバルーンを利用者が左ダブルクリックして隠したとき**だけ送る。シェルの中のバルーン（箱）では起きない（話していないときの箱のダブルクリックは、完了 spec `areka-P0-shell-balloon` の決まりどおりシェルへの `OnMouseDoubleClick` のまま）。台本の末尾の待機（`\w` など）が残っている間は再生中なので、そのダブルクリックは `OnBalloonBreak` になる。areka のバルーンには閉じるボタンが無い。同じダブルクリックでも、再生中なら中断（`OnBalloonBreak`）、再生が終わって表示だけが残っているなら「読み終えたバルーンを閉じた」（`OnBalloonClose`）と読む。SSP の本体設定の説明（「スクリプトブレーク」は「スクリプトの表示を中断し、バルーンを閉じます」）と、完了 spec `areka-P0-balloon-visibility` の確定判断「クリックによる閉鎖は `OnBalloonClose` へ集約される」に沿う。台本の `\b[-1]`・次のトークの開始での消去・時間切れでは送らない。退けた案: 「areka には閉じる操作が無いので送らず、縮退として登記する」・「箱でも閉じるようにする」（完了 spec の決まりを覆す理由が無い）。
- **C. 項目 7（`balloontimeout` の表示の側）の扱い**（要件 6.5）——既定: **本 spec から切り離し**、`areka-P0-sakura-time-directives` に表示の側も引き取ってもらう（台本のコンパイルの側と揃って初めて成り立つため。brief「着手するときは 8 と 10 を先に、7 は切り離す」）。引き取り先を別の新しい spec にするなら `/kiro-discovery` で起票する。どちらでも、台帳と互換対応表の持ち主を引き取り先の名前へ直す。
- **D. `OnBalloonTimeout` の Reference1（残り時間）の値**（要件 2.3）——**討議の前に確定: `0`**（時間切れは満了の時刻に達した巡で決まるので残りは常に 0 以下で、ほかの値を作る材料が無く、単位も決めずに済む）。正典は「残り時間」とだけ書き、何の残りか・単位を示さない。時間切れの時点では待ち時間の残りは 0 なので `0` を送り、areka 裁量として互換対応表に記録する。
- **E. 切替の送り出しの台詞を中断したとき**（要件 1.1・1.6）——**討議の前に確定（規則を 1 つに保つ）**。ゴーストの切替の `OnGhostChanging` の台詞と切替の `OnClose` の別れの台詞、シェルの切替の `OnShellChanging` の台詞のどれも、中断すると切替が中止されて定常へ戻る。そのときも「中断の後に定常へ戻った」ので `OnBalloonBreak` を送る（Reference0 は止めた台詞の台本）。終了の握手の別れの台詞は終了へ進むので送らない（要件 1.6）。

## Requirements

### Requirement 1: 利用者の中断を `OnBalloonBreak` で届ける
**Objective:** As a ゴースト作者, I want 話の途中でバルーンをダブルクリックして止められたことを `OnBalloonBreak` で受け取りたい, so that 途中で切られたことに気付いて、続きを同じ調子で話し続けずに済む

#### Acceptance Criteria
1. When 利用者の中断で再生中のトークが止まり、その後 areka が定常へ戻ったとき, the areka shall `OnBalloonBreak` をそのゴーストの SHIORI へ 1 回送る（起動の挨拶・定常のトーク・切替の送り出しの台詞のどれを止めた場合も、定常へ戻るなら送る）。
2. The areka shall `OnBalloonBreak` の Reference0 に、止めたトークの台本を入れる。
3. The areka shall `OnBalloonBreak` の Reference1 に、ダブルクリックされたバルーンのスコープ番号（本体 0・相方 1・以降も。シェルの中のバルーンなら、その箱が属するキャラクターのスコープ番号）を入れる。
4. The areka shall `OnBalloonBreak` の Reference2（中断位置）を空の文字列で送る（討議で確定する点 A。縮退の記録は要件 6.2）。
5. The areka shall `OnBalloonBreak` を、止めたトークが止まり終えて定常へ戻った後に送り、トークを止める前や止めている途中には送らない（応答の台本が、止めたトークと重なって再生されない）。
6. If 中断の後に定常へ戻らないとき（止めた台本が終了を予約していて終了へ進む・終了の握手の別れの台詞を止めた・保留の終了の要求や保留の切替へ進む）, then the areka shall `OnBalloonBreak` を送らない。ゴーストの切替の別れの台詞（切替のときに送る `OnClose` の応答）を止めた場合は、切替が中止されて定常へ戻るので、送る側（1.1）に入る。
7. If 中断の合図が中断として受け入れられなかったとき（再生中のトークが無い・同じトークへの 2 回目の合図・作者が中断を禁じた区間）, then the areka shall `OnBalloonBreak` を送らない。
8. The areka shall 1 回の中断につき `OnBalloonBreak` を 1 回だけ送る。
9. The areka shall 利用者の中断の振る舞い（トークがその場で止まる・出ている全バルーンが即座に隠れる・終了の予約があれば終了へ進む）を、`OnBalloonBreak` を送ることで変えない。

### Requirement 2: 時間切れを `OnBalloonTimeout` で届ける
**Objective:** As a ゴースト作者, I want バルーンが読まれないまま時間切れで消えたことを `OnBalloonTimeout` で受け取りたい, so that 読まれなかった話を言い直すなど、読み手の様子に合わせられる

#### Acceptance Criteria
1. When 会話の後の時間切れで、出ていたバルーンが隠れたとき, the areka shall `OnBalloonTimeout` をそのゴーストの SHIORI へ 1 回送る（隠れたスコープの数によらず 1 回）。
2. The areka shall `OnBalloonTimeout` の Reference0 に、時間切れで隠れたバルーンに出ていた台本（直前に再生を終えたトークの台本）を入れる。
3. The areka shall `OnBalloonTimeout` の Reference1 に `0` を入れる（時間切れの時点で待ち時間の残りは無い。討議で確定する点 D）。
4. While 抑止（バルーンのドラッグ・バルーンの上のポインタ・選択肢の表示）で時間切れによる非表示が見送られている間, the areka shall `OnBalloonTimeout` を送らず、抑止が解けた後に実際にバルーンが隠れたときに送る。
5. If バルーンが時間切れ以外の理由で隠れたとき（利用者の中断・利用者が閉じた・台本の `\b[-1]`・次のトークの開始での消去・ゴーストやバルーンの切替・終了）, then the areka shall `OnBalloonTimeout` を送らない。
6. If 時間切れでバルーンが隠れた時点で、既に次のトークが始まっていた、または定常でなかったとき, then the areka shall `OnBalloonTimeout` を送らず、送らなかったことを記録する。
7. The areka shall 選択肢の時間切れでは `OnBalloonTimeout` を送らない（正典「選択肢以外で」。選択肢の時間切れは既にある `OnChoiceTimeout` のまま）。

### Requirement 3: 読み終えたバルーンを閉じたことを `OnBalloonClose` で届ける
**Objective:** As a ゴースト作者, I want 利用者が読み終えたバルーンを閉じたことを `OnBalloonClose` で受け取りたい, so that 読んでくれたことに気付き、読み手の間合いに合わせて次を話せる

#### Acceptance Criteria
1. While 再生中のトークが無い定常の間, when 利用者が出ているバルーン（シェルの中のバルーンを除く）を左ダブルクリックしてバルーンが隠れたとき, the areka shall `OnBalloonClose` をそのゴーストの SHIORI へ 1 回送る（討議で確定する点 B）。台本の末尾の待機が残っていて再生中のときは、要件 1 の中断として扱う。
   - 補足: シェルの中のバルーン（箱）を話していないときにダブルクリックした場合は、完了 spec `areka-P0-shell-balloon` の決まりどおりシェルへの `OnMouseDoubleClick` になり、`OnBalloonClose` は送らない。
2. The areka shall `OnBalloonClose` の Reference0 に、閉じたバルーンに出ていた台本（直前に再生を終えたトークの台本）を入れる。
3. If バルーンが閉じる操作以外の理由で隠れたとき（時間切れ・再生中の中断・台本の `\b[-1]`・次のトークの開始での消去・ゴーストやバルーンの切替・終了）, then the areka shall `OnBalloonClose` を送らない。
4. If ダブルクリックが定常でない間（起動の途中・終了の握手の待ち・切替の途中）、または終了の要求を保留している間（マウスのイベントを送らない間と同じ）に起きたとき, then the areka shall `OnBalloonClose` を送らない。
5. The areka shall クリックでバルーンを閉じたことを知らせる独自のイベント（正典に無い `OnBalloonClick` など）を作らず、`OnBalloonClose` だけで知らせる。

### Requirement 4: 3 つのイベントに共通する送り方
**Objective:** As a ゴースト作者, I want 3 つのイベントが他のイベントと同じ決まりで届いてほしい, so that 応答の書き方を変えずに済み、思わぬ時機に呼ばれない

#### Acceptance Criteria
1. The areka shall `OnBalloonBreak`・`OnBalloonClose`・`OnBalloonTimeout` を、マウスのイベントと同じく応答の台本を受け取る形（GET）で送る。
2. When 3 つのイベントのどれかに SHIORI が台本を返したとき, the areka shall その台本を他のイベントの応答と同じく `OnTranslate` を通してから新しいトークとして再生する。
3. When 3 つのイベントのどれかに SHIORI が台本を返さなかったとき（204 など）, the areka shall 何も再生せず、定常のまま続ける。
4. The areka shall 3 つのイベントを定常の間だけ送り、起動の途中・終了の握手の待ち・切替の途中には送らない（後で送るために積んでおくこともしない）。
5. The areka shall 同じ出来事（1 回の中断・1 回の時間切れ・1 回の閉じる操作）について同じイベントを 2 回以上送らない。
6. If 3 つのイベントの送出で SHIORI がエラーを返した、または輸送路が失敗したとき, then the areka shall 他のイベントと同じ扱い（エラー応答は致命でない・輸送路の失敗はそのゴーストの SHIORI の故障）に従い、3 つのイベントのための例外を作らない。

### Requirement 5: 中断で終わったトークの時間切れの起点
**Objective:** As a 利用者, I want 途中で終わった話のバルーンが、終わった時点から数えて既定の時間で消えてほしい, so that 中断の後に、流れなかった続きの分までバルーンが余分に残らない

#### Acceptance Criteria
1. When トークが中断で終わり、バルーンが出たまま残ったとき（利用者の中断以外。選択肢の時間切れでの解除など）, the areka shall 時間切れの計測の起点を、占有区間の終端と中断が起きた時刻のうち早い方にする。
2. The areka shall 中断が起きた時刻を、フレームの観測の時刻に丸めず、トークが止まった時刻そのもので扱う。
3. When トークが最後まで流れて終わったとき, the areka shall 今と同じく占有区間の終端を起点にする。
4. The areka shall 利用者の中断では今と同じく時間切れを待たずに即座にバルーンを隠す（起点の規則の対象にしない）。
5. The areka shall 抑止の規則（抑止の間は消さない・抑止が解けたらその時点から既定の時間を計り直す）と、次のトークが始まったら計測を捨てる規則を変えない。
6. If 中断が起きた時刻が表示の側に届かないとき, then the areka shall 今と同じく占有区間の終端を起点にし（表示を保持する側へ倒す）、時刻が届かなかったことを記録する。

### Requirement 6: 縮退・切り離しの記録と予約の印
**Objective:** As a 開発チーム, I want 送れるようになったことと、まだ正典どおりでない点を、互換対応表と網羅の台帳とコードで一致させたい, so that 語彙だけが残って追跡が失われる失敗を繰り返さない

#### Acceptance Criteria
1. The 開発チーム shall 互換対応表の行「`OnBalloonClose` ／ `OnBalloonTimeout` ／ `OnBalloonBreak` の SHIORI 発火」を、送る場面（要件 1〜3）と Reference の入れ方に書き直し、正典が沈黙する点の areka の決定（`OnBalloonClose` を送る場面・`OnBalloonTimeout` の Reference1 を `0` とすること・中断の後に定常へ戻ったときだけ `OnBalloonBreak` を送ること・3 つとも GET で送ること）を根拠の区分（正典整合／areka 裁量）とともに記録する。
2. The 開発チーム shall `OnBalloonBreak` の Reference2 を空で送ることを縮退として互換対応表に記録し、追跡先 `areka-P0-balloon-break-position`（起票済み）の名前を互換対応表と台帳に書く（追跡先の無い先送りを作らない）。
3. The 開発チーム shall 互換対応表の行「会話が中断で終わったときのタイムアウト起点」を、要件 5 の規則に書き直す。
4. The 開発チーム shall 網羅の台帳の 3 行の状態を実際に合わせ（`OnBalloonClose`・`OnBalloonTimeout` は実装済み、`OnBalloonBreak` は Reference2 を空で送るので縮退）、持ち主を本 spec の名前へ直し、`note` を書き直し、台帳から作る生成物を作り直す。実装済みとする行には、ソースの側に正典の URL の証拠を置く。
5. The 開発チーム shall `\![set,balloontimeout,時間]` の表示の側を本 spec では実装せず、台帳の `balloontimeout` の行と互換対応表の該当行の持ち主・追跡先を、引き取る spec の名前へ直し、引き取る spec の brief に相互登記する（討議で確定する点 C）。
6. The 開発チーム shall 予約の型に付いている「作る側も受け取る側も無い」の注記と警告の抑止を外す（台本と scope を持つのは会話の進行の側なので、型そのものが要らなくなれば型ごと消す）。外せない部分（討議の結果、送らないと決めた分岐が残る場合）は、注記に追跡先の spec の名前を書く。
7. The 開発チーム shall 完了 spec `areka-P0-balloon-break` の約束「中断を理由とするイベントを SHIORI へ 1 件も送らない」が `OnBalloonBreak` について本 spec で改められたことを、互換対応表の「バルーンの中断の操作」の行に追記する。

### Requirement 7: 観測できること
**Objective:** As a 開発チーム, I want 3 つのイベントを送ったこと・送らなかったこと・時間切れの起点を記録から確かめたい, so that 実機で「空振りした」ことにも気付ける

#### Acceptance Criteria
1. When 3 つのイベントのどれかを送ったとき, the areka shall イベント名・きっかけ（中断・閉じる操作・時間切れ）・スコープ番号（`OnBalloonBreak` のとき）を 1 行で記録する。
2. When 3 つのイベントを送る場面に当たりながら送らなかったとき（定常でない・次のトークが既に始まっていた・中断の後に終了へ進んだ など）, the areka shall 送らなかった理由を記録する。
3. When 中断で終わったトークの時間切れの計測を始めたとき, the areka shall 起点に採った時刻と、それが中断の時刻か占有区間の終端かを記録する。
4. The areka shall 毎フレームの判定そのものは記録せず、送った・送らなかった・起点を決めた、の出来事が起きたときだけ記録する。

### Requirement 8: 検証
**Objective:** As a 開発チーム, I want 送る・送らないの分かれ目をすべて決定論のテストで固定し、実機でも確かめたい, so that 後から壊れても気付ける

#### Acceptance Criteria
1. The 開発チーム shall 次の分かれ目を決定論のテストで固定する——中断の後に定常へ戻ると `OnBalloonBreak` が Reference0〜2 の正しい値で 1 回だけ送られること／中断の後に終了・別れの台詞の終わり・保留の終了・保留の切替へ進むときは送られないこと／中断が受け入れられないときは送られないこと／時間切れで `OnBalloonTimeout` が 1 回だけ送られること／抑止の間は送られず、抑止が解けて隠れたときに送られること／次のトークが既に始まっていたときは送られないこと／再生中のトークが無い定常でのダブルクリックで `OnBalloonClose` が送られ、定常でない間や他の理由で隠れたときは送られないこと／中断で終わったトークの時間切れの起点が、中断の時刻と占有区間の終端の早い方になること（境界の直前・直後）／送ってよいイベントの表の数。
2. The 開発チーム shall テストの時刻を注入して駆動し、実時間の待機に頼るテストを作らない。注入した時刻が観測すべき時点を追い越さない形にする。
3. When 実機で確かめるとき, the 開発チーム shall 適合ゴースト emo2 を絶対パスで起動し、判定の分かれ目の記録が出る詳しさまでログを開けて、有界の自動終了のもとで次を記録の照合で確かめる——⑴ 長い台詞の途中でバルーンを左ダブルクリックすると `OnBalloonBreak` が正しいスコープ番号で送られる ⑵ 会話の後に放っておくと（待ち時間は環境変数で短縮してよい）バルーンが消えて `OnBalloonTimeout` が送られる ⑶ 台詞が終わって出ているバルーンを左ダブルクリックすると `OnBalloonClose` が送られる。実機の根と一時フォルダはワークツリーの `target\` の下に置く。
