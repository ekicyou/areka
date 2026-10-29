# Requirements Document

## Project Description (Input)

- **困っている人**:
  - 利用者。キャラクターの触れる所にマウスを乗せても、カーソルの形が変わらない。ゴースト「窓際のぱすたさん」は身体の当たり判定の上で手の形になり、「触れられる」と分かる作り。areka では矢印のまま（`areka-P0-pasta-in-windows-verification` の申し送りの件 16）。
  - シェルとゴーストの作者。`surfaces.txt` のカーソルの定義（cursorブレス＝`sakura.cursor`・`kero.cursor`・`char*.cursor`）と、ゴーストの descript のカーソル（`cursor`・`mousecursor.*`）が効かない。
- **今の状況**:
  - `surfaces.txt` の読み手は、知らない見出しのブレスを記録なしに捨てる。cursorブレスもその 1 つで、中身ごと消えて何も残らない。
  - キャラクターの窓は、カーソルの形を決める問い合わせに、診断の十字（開発用の設定のときだけ）しか答えない。
  - 窓ごとに「今どの当たり判定の上に居るか」を覚える所は、完了 `mouse-click-wheel-events` が作った（`OnMouseEnter`／`OnMouseLeave` の送出に使う）。外から読む口はまだ無い。押下とホイールの見張りも同じ所に在る。
  - 網羅台帳では、cursorブレスの 5 つの行・`system:*` の 10 行・ゴーストの descript の `cursor`／`mousecursor.*` の 6 行が「無い」。
- **変えたいこと**:
  - キャラクターの当たり判定の上に乗ると、`surfaces.txt` の定義のカーソル（`.cur`・`.ani`・`system:*`）になる。押したとき・右を押したとき・止まったとき・ホイールのときも定義どおり。当たり判定の外では矢印に戻る。
  - ゴーストの descript の `mousecursor.*` が既定の形を差し替える。
  - 定義の行やファイルが読めないときは記録が残る（黙って捨てない）。
  - 網羅台帳の行が実物と合う。
- 詳細は同じフォルダの `brief.md`（末尾の「2026-10-24 棚卸㉚の再測定」と「本 spec からの申し送り」が最新）。

## Introduction

正典（ukadoc・ukadoc MCP と ukadoc の頁で 2026-10-29 に引き直した）は次のとおり書く。

- cursorブレス（ https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html ）: 見出しは「cursorブレス」。記述例は `sakura.cursor` で、「相方側の設定はkero.cursor、char*.cursor」と書く。
- `mouseup*,当たり判定ID,ファイル名`（ https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#mouseup*%2C%E5%BD%93%E3%81%9F%E3%82%8A%E5%88%A4%E5%AE%9AID%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D ）: 「当たり判定の上に乗った時表示するカーソルファイルを設定。」。既定は「system:finger（当たり判定名がbustの場合のみsystem:hand）」。
- `mousedown*`（同じ頁の `#mousedown*…`）: 「当たり判定の上でマウスボタンを押した時に表示するカーソルファイルを設定。」。既定は「system:finger（当たり判定名がbustの場合のみsystem:grip）」。
- `mouserightdown*`（同じ頁の `#mouserightdown*…`・SSP 2.6.14）: 「当たり判定の上でマウス右ボタンを押した時に表示するカーソルファイルを設定。」。既定は `system:finger`。
- `mousehover*`（同じ頁の `#mousehover*…`）: 「当たり判定の上でマウスをしばらく動かさない時に表示するカーソルファイルを設定。」。既定は `system:arrow`。何ミリ秒で「しばらく」かは書かない。
- `mousewheel*`（同じ頁の `#mousewheel*…`）: 「当たり判定の上でマウスホイールを回した時に表示するカーソルファイルを設定。」。既定は `system:arrow`。いつ元の形へ戻すかは書かない。
- 5 つの場面に共通の書き方: `*` は「同じスコープの」同じ場面の定義で重複しない番号・当たり判定名は collision定義の名前・ファイル名は「カーソルファイル名(cur/ani)」か、その代わりの「システムカーソル指定」。
- システムカーソル（同じ頁の `#system%3Aarrow` ほか）: `system:arrow`（矢印）・`system:cross`（十字）・`system:finger`（指差し）・`system:grip`（つかんだ手）・`system:hand`（手）・`system:help`（？）・`system:move`（移動）・`system:no`（禁止）・`system:text`（I）・`system:wait`（時計）の 10 種。
- ゴーストの descript（ https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#cursor%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D ほか）: `cursor`／`mousecursor`＝「ゴーストの操作できる部分に使用するカーソル（指先）」（既定 `system:finger`・`mousecursor` の綴りは 2.5.41 から）・`mousecursor.arrow`＝「ゴーストの操作できない部分を示すカーソル（矢印）」（2.6.11）・`mousecursor.hand`＝「ゴーストの掴める部分を示すカーソル（手を開いた形）。通常はBust当たり判定に自動で使用される。」（2.6.11）・`mousecursor.grip`＝「通常はBust当たり判定でマウスボタンを押した状態に自動で使用される。」（2.6.11）・`mousecursor.text`＝「ゴーストの入力ボックスに使用するカーソル(I-BEAM)」（2.5.41）・`mousecursor.wait`＝「ゴーストがなにか処理中で応答できないときに使用するカーソル(腕時計型)」（2.5.43）。
- ファイルの探し方（プロパティ `currentghost.seriko.cursor.scope(ID).mouse????list(当たり判定名).path`・ https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html ）: 「相対パス：シェル→ゴーストフォルダ内の順で探索」。

SSP の基準（`areka-P0-pasta-in-windows-verification` の `ssp-measurement.md`・SSP 2.9.23・2026-10-23）: ぱすたの身体（cursorブレスで `system:hand`）は SSP の手の形、ブレスに行の無い × とボールは Windows の標準の指さしの形、当たり判定の無い窓の白地は矢印だった。**定義の無い当たり判定にも既定の `system:finger` が効く**ことはこの測りで確かめてある（brief の議題 1 の答え）。

本 spec の要件は、brief の議題と、正典が黙る所について**推奨の案**で書き、2026-10-29 の要件討議で全議題を推奨の案のまま確定した（開発者「承認」）。決めと採らなかった案は末尾の「要件討議で決めた点」に並べる。SSP で測ってから決める議題（2・3・4・9）は、測りの答えが推奨とずれたときだけ当たる要件を書き直す。

## Boundary Context

- **In scope**:
  - `surfaces.txt` の cursorブレス（`sakura.cursor`・`kero.cursor`・`char*.cursor`）の 5 つの場面（`mouseup*`・`mousedown*`・`mouserightdown*`・`mousehover*`・`mousewheel*`）の読み取り。
  - システムカーソル 10 種・`.cur`・`.ani` の表示と、ファイルの探し方（シェルのフォルダ → ゴーストのフォルダ）。
  - ゴーストの descript の `cursor`／`mousecursor`・`mousecursor.arrow`・`mousecursor.hand`・`mousecursor.grip` による既定の形の差し替えと、`mousecursor.text`・`mousecursor.wait` の読み取り。
  - キャラクターの窓の上で、窓ごとの「今どの当たり判定の上か」と押下・ホイール・静止から形を選び、その場で出すこと。
  - 読めない行・ファイルの記録。決定論の自動テスト。ぱすたの手順書の段 3 での実機の確かめ。網羅台帳の行。`doc/COMPAT_ARCHITECTURE.md` §8 への areka の決めの登記。
- **Out of scope**:
  - ツールチップ（`shell-tooltip`）。
  - バルーンの窓の上のカーソル（バルーンの descript の `cursor`・`mousecursor.*`）。
  - プロパティ（`currentghost.seriko.cursor.*`・`currentghost.mousecursor.*`）での読み書きと差し替え（`currentghost-property-tree`）。
  - `OnMouseEnter`／`OnMouseLeave`・`OnMouseHover` の送出（`mouse-click-wheel-events`・`mouse-hover-event`）。
  - 当たり判定の形と解決（`collisionex-regions` ほか）・ドラッグの判定そのもの（今の結果を使うだけ）。
  - 入力ボックス（`inputbox-user-input`・`balloon-communicatebox`）の上のカーソル。
- **Adjacent expectations**:
  - 完了 `mouse-click-wheel-events` が作った窓ごとの「今どの当たり判定の上か」の追跡を、本 spec は読むだけにする。追跡の決まり（どの位置をどの当たり判定と見るか・窓の外へ出たときの扱い）と、`OnMouseEnter`／`OnMouseLeave` の送り方は変えない。
  - 後の `shell-tooltip`・`shell-hover-animation` は、本 spec が足す読む口を使う（作り直さない）。
  - `mouse-hover-event`（版は未定）は「しばらく動かさない」を測る。本 spec の止まったと見なす時間と同じ時間を使う見込みで、後に来る側が先の側の決めを使う。
  - 当たり判定の解決（完了 `collision-geometry` ほか）が、今の `OnMouseMove` と同じ当たり判定の名前を返すこと。
  - 同じ版の `balloon-scroll-arrows`（後のウェーブ）も入力の層の束ね（`crates/areka/src/input_events/mod.rs`）に行を足すので、本 spec はそこに触らない形にする（brief の棚卸㉚の見立て）。

## Requirements

### Requirement 1: cursorブレスの読み取り

**Objective:** シェルの作者として、`surfaces.txt` に書いたカーソルの定義を areka に読んでほしい。そうすれば、SSP 向けに作ったシェルの形の指定がそのまま効く。

#### Acceptance Criteria

1. When シェルを読み込んだ, the areka shall `sakura.cursor`（スコープ 0）・`kero.cursor`（スコープ 1）・`char*.cursor`（`*` の数のスコープ）のブレスの行を、スコープ・当たり判定名・場面（`mouseup*`＝乗る・`mousedown*`＝押す・`mouserightdown*`＝右を押す・`mousehover*`＝止まる・`mousewheel*`＝ホイール）・ファイル名の組として読む。
2. The areka shall ファイル名の欄に、システムカーソル 10 種（`system:arrow`・`cross`・`finger`・`grip`・`hand`・`help`・`move`・`no`・`text`・`wait`）と、拡張子が `.cur`・`.ani` のファイル名を受け付ける。
3. If cursorブレスの行が読めない（欄が足りない・場面の名前が 5 つのどれでもない・`*` が数でない・`system:` に続く名前が 10 種のどれでもない・拡張子が `.cur`／`.ani` でない）, the areka shall その行を使わずに、ブレスの見出し・行の原文・理由を記録に残し、同じブレスのほかの行は使う。
4. If 同じスコープ・同じ当たり判定名・同じ場面の行が 2 つ以上ある, the areka shall 後に書いた行を使い、使わなかった行を記録に残す。
5. The areka shall cursorブレスの有無と中身によって、今の面・当たり判定・element定義・アニメーションの読み取りの結果を変えない。
6. The areka shall 当たり判定名を、collision定義の名前と同じ綴りで突き合わせる（cursorブレスの行の当たり判定名が、そのスコープのどの面の当たり判定にも無くても、その行は記録せずに持っておく＝面を替えたときに効く）。

### Requirement 2: 当たり判定の上に乗ったときの形

**Objective:** 利用者として、キャラクターの触れる所にマウスを乗せたら形が変わってほしい。そうすれば、どこを触れるかが見て分かる。

#### Acceptance Criteria

1. When カーソルがキャラクターの窓の当たり判定の上に入った, the areka shall そのスコープ・当たり判定名の `mouseup*` の行の形を出す。
2. If その当たり判定名の `mouseup*` の行が無い, the areka shall 既定の形を出す（当たり判定名が `bust` なら `system:hand`、ほかは `system:finger`。`bust` は大文字と小文字を区別せずに見る＝`Bust` も同じ。ゴーストの descript による差し替えは要件 5）。
3. When カーソルが当たり判定から別の当たり判定へ直に移った, the areka shall 移ったその場で移り先の形を出し、前の当たり判定の形を描画の 1 回分も残さない。
4. When カーソルがキャラクターの窓の上で当たり判定の無い所へ出た, the areka shall その場で矢印（`system:arrow`。ゴーストの descript による差し替えは要件 5）を出す。
5. While カーソルがキャラクターの窓の、マウスを下の窓へ通す所（絵の透けた所）の上に居る, the areka shall 形を決めず、下に見える窓の形のままにする。
6. When カーソルを動かしていない間に面が替わって、カーソルの下の当たり判定が変わった, the areka shall 当たり判定の出入りを決めるのと同じ時点（完了 `mouse-click-wheel-events` の要件 7.9 の「次にカーソルが動いた時点」）で形を選び直す。
7. The areka shall 形の選び方を窓ごと・スコープごとに分け、ある窓の形が別の窓の形を変えないようにする。
8. The areka shall バルーンの窓の上のカーソルの形を今のまま変えない。

### Requirement 3: ボタンを押したときの形

**Objective:** 利用者として、キャラクターを押したときに形が変わってほしい。そうすれば、「つかんだ」「押した」が手に伝わる。

#### Acceptance Criteria

1. When 当たり判定の上で右以外のマウスボタンを押した, the areka shall そのスコープ・当たり判定名の `mousedown*` の行の形を出す。行が無ければ既定の形（当たり判定名が `bust` なら `system:grip`、ほかは `system:finger`。差し替えは要件 5）を出す。
2. When 当たり判定の上で右ボタンを押した, the areka shall そのスコープ・当たり判定名の `mouserightdown*` の行の形を出す。行が無ければ既定の `system:finger`（差し替えは要件 5）を出す。
3. While ボタンを押している（押下がドラッグになった間と、押したまま当たり判定の外へ出た間を含む）, the areka shall 押したときの形を保つ。
4. When 押していたボタンを放した, the areka shall その場で、その位置の形（当たり判定の上なら要件 2.1〜2.2 の形・当たり判定の無い所なら矢印）へ戻す。
5. When 当たり判定の無い所でボタンを押した, the areka shall 形を変えない。
6. The areka shall 押したときの形を出すことで、クリック・ダブルクリック・ドラッグ・右クリックのメニューの今の動きと、それらが SHIORI へ送るイベントを変えない。

### Requirement 4: 止まったとき・ホイールのときの形

**Objective:** シェルの作者として、当たり判定の上でマウスを止めたときやホイールを回したときの形も指定どおりにしてほしい。

#### Acceptance Criteria

1. When カーソルが当たり判定の上で、止まったと見なす時間のあいだ動かなかった, the areka shall そのスコープ・当たり判定名の `mousehover*` の行の形を出す。行が無ければ既定の `system:arrow`（差し替えは要件 5）を出す。
2. The areka shall 止まったと見なす時間を、利用者の OS の設定の「マウスを止めたと見なす時間」とし、描画の回の区切りに丸めずに計る。
3. When カーソルが当たり判定の上に居る間にホイールが回された, the areka shall そのスコープ・当たり判定名の `mousewheel*` の行の形を出す。行が無ければ既定の `system:arrow`（差し替えは要件 5）を出す。
4. When 止まったときの形かホイールのときの形を出した後に、カーソルが動いた, the areka shall その場で、その位置の形（要件 2）へ戻す。
5. While ボタンを押している, the areka shall 止まったときの形とホイールのときの形を出さず、押したときの形（要件 3.3）を保つ。

### Requirement 5: ゴーストの descript による既定の形の差し替え

**Objective:** ゴーストの作者として、descript に書いたカーソルで areka の既定の形を差し替えたい。そうすれば、シェルに cursorブレスを書かなくても、ゴーストの作りに合った形を出せる。

#### Acceptance Criteria

1. Where ゴーストの descript に `mousecursor`（または旧い綴りの `cursor`）がある, the areka shall `system:finger` の既定（要件 2.2・3.1 の `bust` 以外のときと、要件 3.2）の代わりに、その指定の形を出す。
2. Where ゴーストの descript に `mousecursor.hand` がある, the areka shall 当たり判定名が `bust` のときの乗る形の既定（要件 2.2）の代わりに、その指定の形を出す。
3. Where ゴーストの descript に `mousecursor.grip` がある, the areka shall 当たり判定名が `bust` のときの押す形の既定（要件 3.1）の代わりに、その指定の形を出す。
4. Where ゴーストの descript に `mousecursor.arrow` がある, the areka shall キャラクターの窓の当たり判定の無い所の矢印（要件 2.4）と、止まったとき・ホイールのときの既定の矢印（要件 4.1・4.3）の代わりに、その指定の形を出す。
5. The areka shall cursorブレスに行がある場面では、その行の形を出し、ゴーストの descript の指定を使わない。
6. If ゴーストの descript に `mousecursor` と `cursor` の両方がある, the areka shall `mousecursor` を使い、そのことを記録に残す。
7. The areka shall `mousecursor.text`・`mousecursor.wait` を読み、キャラクターの窓にはこれらを使う場面（入力ボックス・処理中で応答できないとき）が無いので出さない。読んだことと使わないことを、ゴーストを読み込むたびに 1 回だけ記録に残す。
8. The areka shall descript の値には、要件 1.2 と同じく、システムカーソル 10 種と `.cur`・`.ani` のファイル名を受け付ける。読めない値は要件 1.3 と同じに記録して、その差し替えを使わない。

### Requirement 6: 形の出し方とカーソルのファイル

**Objective:** 利用者として、指定の形がきちんと出てほしい。シェルの作者として、ファイルが壊れていても areka が止まらず、何が起きたか分かってほしい。

#### Acceptance Criteria

1. When `system:arrow`・`cross`・`finger`・`help`・`move`・`no`・`text`・`wait` のどれかを出す, the areka shall OS の標準の形のうち対応する形（矢印・十字・指差し・ヘルプ・四方向の移動・禁止・文字の I・待ち）を、利用者が OS で設定した絵と大きさで出す。
2. When `system:hand`・`system:grip` を出す, the areka shall areka が同梱する「手を開いた形」「手を握った形」の絵を出し、指差し（`system:finger`）と見分けられる形にする。絵は要件の段で開発者が選んだもの（fal で作った案から選ぶ・許諾の分かるもの）とする。
3. When `.cur` のファイルを指定どおりに出す, the areka shall そのファイルの絵を出す。`.ani` のファイルなら、ファイルどおりに動かして出す。
4. The areka shall cursorブレスのファイル名とゴーストの descript のファイル名を、どちらもシェルのフォルダ → ゴーストのフォルダ（`ghost\master`＝ゴーストの descript の置き場）の順で探す（ukadoc のプロパティ `currentghost.mousecursor` の「相対パス指定はシェル→ゴーストフォルダ内の順で探索する」に合わせる）。
5. If 指定のファイルが無い・読めない・カーソルのファイルとして壊れている, the areka shall その場面の既定の形（要件 2〜5 の順で決まる形）を出し、ファイル名・探した場所・理由を記録に残す。
6. If 指定のファイル名が、探す場所のフォルダの外を指す（絶対パス・上のフォルダへ出る書き方）, the areka shall そのファイルを読まずに既定の形を出し、記録に残す。
7. The areka shall 同じファイルの失敗を、形を出すたびには記録せず、シェル（またはゴースト）を 1 回読み込むごとに 1 回だけ記録する。
8. When シェルかゴーストを替えた, the areka shall 替えた後は新しいシェルとゴーストの定義だけで形を選び、前のシェルの形を出さない。
9. The areka shall カーソルのファイルをシェル（またはゴースト）の読み込みのときに 1 回だけ読み、形を出すたびには読み直さない。

### Requirement 7: 記録・確かめ・網羅台帳

**Objective:** 開発者として、形の選び方が SSP と同じであることをテストと実機で確かめ、網羅台帳を実物と合わせたい。

#### Acceptance Criteria

1. When 窓の形を選び直した, the areka shall 詳しい記録の段（既定では出ない段）に、窓のスコープ・当たり判定名・場面・選んだ形とその出どころ（cursorブレス・ゴーストの descript・既定）を残す。
2. The areka shall 次の点を決定論の自動テストで固定する: cursorブレスの読み取り（3 つの見出し・5 つの場面・読めない行の記録・重なった行）・既定の形（`bust` とほか）・出どころの優先順（cursorブレス ＞ ゴーストの descript ＞ 既定）・乗る／押す／右を押す／止まる／ホイールの切り替わりと戻り・当たり判定から当たり判定へ移ったその場での切り替わり・ファイルの探し方と失敗の記録。
3. When 実機で確かめる, the areka shall ぱすたの手順書（`doc/pasta-real-run.md` の段 3）の手順で、身体の上が手を開いた形・× とボールの上が指差し・当たり判定の無い窓の白地が矢印になることを、カーソルの形を読み取って示す。
4. The areka shall 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の、cursorブレスの 5 つの場面の行・システムカーソル 10 種の行・ゴーストの descript の `cursor`／`mousecursor`・`mousecursor.arrow`・`mousecursor.hand`・`mousecursor.grip`・`mousecursor.text`・`mousecursor.wait` の行を、実物に合わせて書き直し、持ち主を本 spec にする（`mousecursor.text`・`mousecursor.wait` は「読むが使う場面が無い」と書く）。バルーンの descript の行とプロパティの行は変えない。
5. The areka shall 正典が黙る所の areka の決め（止まったと見なす時間・ホイールの形から戻る時・押している間の形・重なった行・`mousecursor` と `cursor` の両方があるとき・`system:hand`／`system:grip` の絵）を `doc/COMPAT_ARCHITECTURE.md` §8 に書く。

## 要件討議で決めた点

2026-10-29 の要件討議で、議題 2〜9 を下の「推奨の案」のまま確定した（開発者「承認」）。議題 1 は SSP の測り（2026-10-23）で答えが出ていたので、討議に出さずに確定した。議題 2・3・4・9 は、承認をもらってから SSP で測って確かめる。測りの答えが推奨とずれたときだけ、当たる要件を書き直す。

| 議題 | 決めた案（本文） | 採らなかった案 | 当たる要件 |
| --- | --- | --- | --- |
| 1 既定の形を全部のゴーストに効かせるか（brief の議題 1） | **正典どおり効かせる**。SSP の測り（2026-10-23）で、定義の無い × とボールが指差しになることを確かめてある＝SSP と同じ。cursorブレスを書いていないゴーストでも、当たり判定の上は指差しになる | 定義のあるシェルだけで形を変える | 2.2・3.1・3.2 |
| 2 止まったと見なす時間（brief の議題 2） | **利用者の OS の設定の時間**（Windows の既定は 400 ミリ秒）。SSP で、止まってから形が替わるまでの時間を測って確かめる | SSP で測った固定の時間・`OnMouseHover`（`mouse-hover-event`）の決めを待つ | 4.1・4.2 |
| 3 `mousehover*`・`mousewheel*` の既定の矢印 | **正典どおり**（行が無ければ、止まったとき・ホイールのときに矢印になる）。ただし、これだと定義の無い全部のゴーストで、指差しが止まると矢印に替わる。SSP で、行の無い当たり判定の上で止まったときとホイールのときの形を測り、矢印にならなければ「行の無い場面は乗る形のまま」に改める | 行の無い場面は乗る形のまま | 4.1・4.3・5.4 |
| 4 ホイールの形から戻る時（brief の議題 2） | **カーソルが動いたとき**（止まったときの形と同じ）。SSP で測って確かめる | 一定の時間の後に戻す | 4.4 |
| 5 押している間の形（brief の議題 3） | **ボタンを放すまで押したときの形を保つ**（ドラッグ中も・当たり判定の外へ出ても） | 当たり判定を出たら乗る形・矢印へ戻す | 3.3 |
| 6 `system:hand`・`system:grip` の絵 | Windows の標準の形に「手を開いた形」「手を握った形」は無い（SSP は自前の絵を出した）。**areka が自前の絵を同梱する**。絵の出どころ（作る・既存の自由に使える絵）と許諾は討議で決める | 指差しで代える（見分けられない） | 6.2 |
| 7 `mousecursor.text`・`mousecursor.wait` | **読むが出さない**（キャラクターの窓に入力ボックスも「処理中で応答できない」状態も無い）。入力ボックスが来たら、その spec が使う | 処理中の状態を新しく決めて `wait` を出す | 5.7・7.4 |
| 8 `bust` の綴り | 正典は `bust` と `Bust` の両方で書く。**大文字と小文字を区別せずに `bust` と見る**（ほかの当たり判定名は綴りどおり） | 小文字の `bust` だけ | 2.2・3.1・5.2・5.3 |
| 9 ゴーストのフォルダ | 正典の「ゴーストフォルダ」を **`ghost\master`** と読む（descript の置き場所）。SSP で、シェルに無いカーソルのファイルを `ghost\master` に置いて効くかを測る | ゴーストの根 | 6.4 |
