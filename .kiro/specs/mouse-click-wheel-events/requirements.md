# Requirements Document

## Project Description (Input)

- **困っている人**:
  - ゴーストの作者と利用者。キャラクターを 1 回クリックしても、ホイールを回しても、当たり判定に乗っても離れても、areka は SHIORI へ何も知らせない。多くのゴーストが辞書に持つ「つつく」「ホイールでなでる」「乗ると色が変わる」の返事が 1 度も呼ばれず、「何も喋らない」形でだけ現れる。記録にも出ない。
  - 特にゴースト「窓際のぱすたさん」は、× の印の左クリックで終わり、ボールの左クリックでボールのトークを出し、初回の起動ではボールのクリックで身体が映り、× に乗ると × が赤くなる。areka ではどれも起きない（`areka-P0-pasta-in-windows-verification` の申し送りの件 10〜13）。
- **今の状況**:
  - areka がキャラクターの窓のマウスについて SHIORI へ送るのは、移動・ダブルクリック（左と、メニューを抑えたゴーストへの右）・ドラッグの開始と終了の 4 つだけ。単押しのクリック・ボタンを放したこと・ホイール・当たり判定の出入りは送らない。完了 `input-events` の要件が最初の範囲の線として「単発のクリックとホイールは送らない」と決め、完了 `popup-menu-minimal` が「右クリックで `OnMouseClick` を送らない」と決めた。
  - ダブルクリックの見分けは OS がしている。ホイールの回転量と、ボタンを放した知らせは、窓の層に既にある。
  - 網羅台帳では `OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel`・`OnMouseUp`・`OnMouseUpEx`・`OnMouseEnter`・`OnMouseLeave` が「無い」。
- **変えたいこと**:
  - 1 回のクリックで、正典どおりの Reference で `OnMouseUp`（`OnMouseUpEx`）が届き、それに返事が無ければ `OnMouseClick`（`OnMouseClickEx`）が届く。
  - ホイールを回すと `OnMouseWheel` が届き、回転の量と向きが Reference2 に入る。
  - 当たり判定に入ると `OnMouseEnter`、出ると `OnMouseLeave` が届く。
  - ダブルクリック・ドラッグ・右クリックのメニューの今の動きは変わらない。
  - 送った・送らなかったが記録で分かる。網羅台帳の行が実物と合う。
- 詳細は同じフォルダの `brief.md`（末尾の「2026-10-23 棚卸㉙の再測定」と「本 spec からの申し送り」が最新）。

## Introduction

正典（ukadoc「SHIORI Event」の一覧・ukadoc MCP で 2026-10-23 に引き直した）は次のとおり書く。

- `OnMouseUp`（ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseUp ）: 「左・右のマウスボタンが放された際に発生」。Reference0／1＝カーソルの x／y（ローカル座標）・2＝常に 0・3＝本体 0／相方 1・4＝当たり判定の識別子・5＝左 0／右 1／中 2・6＝入力の機器（`mouse` など）。
- `OnMouseClick`（ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseClick ）: 「マウスボタンを1回押して放された瞬間に、OnMouseUpに反応しない場合に発生」。Reference は `OnMouseUp` と同じ並び。中ボタンの 2 は「互換仕様：OnMouseClickExへの移行を推奨」。
- `OnMouseUpEx`・`OnMouseClickEx`（ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseUpEx ・ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseClickEx ）: 左・右以外のボタン。`OnMouseClickEx` は `OnMouseUpEx` に反応しない場合に発生。Reference5＝`middle`・`xbutton1`・`xbutton2`。
- `OnMouseWheel`（ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseWheel ）: ホイールが回転した際に発生。Reference2＝「マウスホイールの回転量および回転方向」。Reference5＝常に 0。タッチパネルの `OnMouseGesture` の代わりとして起きる場合は「upなら正数、downなら負数」に変換すると書く。
- `OnMouseEnter`・`OnMouseLeave`（ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseEnter ・ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseLeave ）: マウスが当たり判定に入った・出た際に発生。Reference2 と 5＝常に 0・ほかは上と同じ並び。
- `OnMouseMultipleClick`（ https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseMultipleClick ）: 応答が無い場合「通常通りOnMouseClickとOnMouseDoubleClickの流れに戻る」＝ダブルクリックの前の 1 回目のクリックも `OnMouseClick` になると読める。クリックを待って見分ける決まりは正典に無い。

### 要件討議で決めた点（2026-10-23・brief の議題 1〜5 と、ギャップ分析から足した 2 件）

番号は下の表のもの（brief の議題 1〜5 に 6〜8 を足した）。要件討議の場では表の 5 を「議題 6」として個別に議論して「入れる」に決め、ほかは推奨の案で承認された（開発者 2026-10-23「入れる、他は承認」）。SSP の測り（討議の場の「議題 1」）も承認された。本文の番号の参照はこの表の番号である。

| 議題 | 決めた案（本文の要件） | 採らなかった案 | 当たる要件 |
| --- | --- | --- | --- |
| 1 `OnMouseUp` をどうするか | **送る**。`OnMouseUp`（`OnMouseUpEx`）を先に送り、台本が返らなかったときだけ `OnMouseClick`（`OnMouseClickEx`）を続ける（正典どおり・選択肢とアンカーの 2 段と同じ形）。ぱすたの辞書に `OnMouseUp` の受け口は無い＝ぱすたには `OnMouseClick` が届く | 送らず、いつも `OnMouseClick` を送る（軽い・規模は 3 タスクほど小さい） | 1・2 |
| 2 クリック・ダブルクリック・ドラッグの見分け | 放した時点で決める（待たない）。ドラッグになった押下の解放と、ダブルクリックを生んだ押下（2 回目）の解放では、`OnMouseUp` も `OnMouseClick` も送らない | ダブルクリックの 2 回目の解放でも送る・ドラッグの後も `OnMouseUp` だけは送る | 3・4 |
| 3 右クリックとメニューの順 | 右クリックのメニューが有効な間（`popupmenu.visible` が 0 でない）は、右ボタンの `OnMouseUp`・`OnMouseClick` を送らない（完了 `popup-menu-minimal` の要件 1.5 のまま）。ゴーストがメニューを抑えた（`popupmenu.visible`＝0）ときだけ送る（右ダブルクリックの決まり〔同 1.10〕と同じ形） | メニューの前に送り、返事があればメニューを出さない・メニューと並べて送る | 5 |
| 4 ホイールの送る頻度 | 窓ごとに 100 ミリ秒（移動の間引きと同じ）に高々 1 回。間の回転は捨てずに足し合わせて次に送る。静かな所からの最初の回転は待たずに送り、残りは間隔が明けた時点で次の回転を待たずに送る。Reference2 は OS の回転量（1 目盛り＝120・上が正）の和 | 1 フレーム分の和を毎フレーム送る | 6 |
| 5 `OnMouseEnterAll`／`OnMouseLeaveAll` | **入れない**（ぱすたは使わない・規模を 16 タスクの内に保つ）。網羅台帳の 2 行は今のまま「無い」 | 入れる（窓の出入りの追跡は同じなので 2〜3 タスク増える） | 7 |
| 6 窓の層（wintf）を触るか（brief の Out of Boundary を改める） | **最小に触る**。ホイールの量を `PointerState.wheel` へ移す（今は溜めるだけで誰にも渡らず、ホイールが一切届かない）・拡張ボタンの押下を配る・ドラッグの終わりに「閾値を越えたか」を残す、の 3 つ。触るのは `pointer/buffers.rs`・`pointer/dispatch/mod.rs`・`drag/state/mod.rs` だけ（同じウェーブの `dpi-realign-remembered-chain` が触る `window_proc/mouse_move.rs`・`drag/dispatch.rs` には触らない） | ホイールだけ wintf で直し、残りは areka の側で回る（同じ瞬間の閾値越えと解放でクリックが先に届く隙が残る） | 2・3・6・Boundary |
| 7 送信の失敗 | **2 段とも、選択肢・アンカーの前例と同じに 204 と同じ扱い**。失敗は error で残し、ゴーストを終わらせない | 2 段目の失敗は今のマウスのイベントと同じに終了の系列へ倒す | 1・2 |
| 8 右のクリックの遅れ（議題 3 の続き） | **受け入れて §8 に書く**。右のクリックはメニューの照会の決着を待って送るので、放した後に届いた移動・出入りのイベントより後に届くことがある | メニューの前に送る（メニューが出るまで毎回待つ） | 5・10 |

**SSP で測る点**（ukadoc で読めない・開発者の承認〔2026-10-23〕を受けて、排他テストの机で computer-use により SSP を操作し、`ssp_shiori_log.txt` の差分で測る〔記憶「意味論は ukadoc から輸入・読めない点は SSP で測る」〕）: ⑴ 当たり判定から当たり判定へ直に移ったときの `OnMouseLeave` と `OnMouseEnter` の順（brief の棚卸㉗）、⑵ ダブルクリックの 2 回目の解放で `OnMouseUp`／`OnMouseClick` が来るか、⑶ ドラッグの終わりの解放で `OnMouseUp` が来るか、⑷ 中ボタンで `OnMouseClick`（Reference5＝2）と `OnMouseClickEx` のどちらが来るか（`OnMouseUp` の中ボタンも）、⑸ `OnMouseWheel` の Reference2 の値（1 目盛りで 120 か 1 か）と、SSP がホイールの知らせを間引くか。測った答えが上の決めと違えば、該当する要件を直す。答えは `research.md` に記す。

## Boundary Context

- **In scope**:
  - キャラクターの窓の上の操作から、`OnMouseUp`・`OnMouseClick`・`OnMouseUpEx`・`OnMouseClickEx`・`OnMouseWheel`・`OnMouseEnter`・`OnMouseLeave` を正典の Reference で送ること。
  - `OnMouseUp` → `OnMouseClick`（`Ex` も同じ）の 2 段の送り方。
  - クリックにしない場合（ドラッグ・ダブルクリック・窓の外で放す・箱の選択肢）の決まりと、ダブルクリック・ドラッグ・右クリックのメニューとの順。
  - ホイールの間引きと足し合わせ。
  - 窓ごとに「今どの当たり判定の上か」を覚え、変わったときに出入りを送ること。
  - 窓の層（wintf）の最小の直し: ホイールの量を受け手へ渡すこと・拡張ボタンの押下を配ること・ドラッグの終わりに閾値を越えたかを残すこと（議題 6）。
  - 送った・送らなかったの記録。網羅台帳の 7 行。`doc/COMPAT_ARCHITECTURE.md` §8 への areka の決めの登記。決定論の自動テスト。ぱすたの手順書の段 3・段 7 での実機の確かめ。
- **Out of scope**:
  - `OnMouseDown`・`OnMouseDownEx`（押した瞬間）・`OnMouseDoubleClickEx`・`OnMouseMultipleClick`・`OnMouseHover`・`OnMouseGesture`（タッチパネルの代わりの `OnMouseWheel` も含む）・`OnMouseEnterAll`・`OnMouseLeaveAll`（議題 5 で入れると決めたときだけ In）。
  - バルーンの窓の上のクリック・ホイール・出入り（バルーンのスクロールは別の spec）。
  - 横のホイール（正典の `OnMouseWheel` に横の Reference は無い）。
  - 入力の機器の見分け（Reference6 は今と同じく `mouse`）。
  - 当たり判定の解決そのもの・ドラッグの判定そのもの（今の結果を使うだけ）。
  - `\t`（時間に厳しい台本）の間にマウスのイベントを止めること（`sakura-time-critical` の持ち物。止める対象に本 spec の 7 つが加わる）。
- **Adjacent expectations**:
  - 当たり判定の解決（完了 `collision-geometry`）が、今の `OnMouseMove` と同じ座標と当たり判定の名前を返すこと。本 spec は新しい解決を作らない。
  - ドラッグの判定（完了 `mouse-drag-events`）が、押下がドラッグになったかを今のまま知らせること。`OnMouseDragStart`／`OnMouseDragEnd` の動きは変えない。
  - 右クリックのメニュー（完了 `popup-menu-minimal`）が、右ボタンの解放でメニューを出すかを今のまま決めること。本 spec が足すのは「メニューを出さないと決まったときに右のクリックを送る」だけ。
  - 後の `shell-mouse-cursor`・`shell-tooltip`・`shell-hover-animation` は、本 spec が持つ「窓ごとに今どの当たり判定の上か」を使うだけにする（作り直さない）。
  - 下流の `mcp-user-response` は、本 spec の送るイベントを「利用者のしたこと」として使う。
  - 同じウェーブの約束（roadmap「ウェーブ編成」C11 の ②）: kanade の SHIORI との往復の層（`actor.rs`＝`shiori2-protocol` の持ち物）と SSTP の受け口（`schedule/sstp.rs`＝`mcp-strict-errors` の持ち物）には触らない。窓の層（wintf）で触るのは議題 6 の 3 ファイルだけで、`dpi-realign-remembered-chain` が触る `window_proc/mouse_move.rs`・`drag/dispatch.rs` には触らない。

## Requirements

### Requirement 1: 左・右ボタンの 1 回のクリック

**Objective:** ゴーストの作者として、キャラクターを 1 回クリックしたときに `OnMouseUp` と `OnMouseClick` を受け取りたい。そうすれば、辞書の「つつく」「× で閉じる」「ボールを押す」の返事が areka でも呼ばれる。

#### Acceptance Criteria

1. When 利用者がキャラクターの窓の上で左ボタンを押して、ドラッグにならずに同じ窓の上で放した, the areka shall 放した時点で `OnMouseUp` を SHIORI へ送る（Reference0／1＝放した位置の x／y・2＝`0`・3＝その窓のスコープ・4＝放した位置の当たり判定の名前〔無ければ空〕・5＝`0`・6＝`mouse`）。
2. When `OnMouseUp` に台本が返らなかった（204）, the areka shall 同じ Reference で `OnMouseClick` を続けて送る。
3. When `OnMouseUp` に台本が返った, the areka shall その台本を再生し、`OnMouseClick` を送らない。
4. The areka shall Reference0／1 の座標を、今の `OnMouseMove` と同じ座標の空間（ローカル座標）で入れる。
5. The areka shall クリックを見分けるために時間を待たない。`OnMouseUp` は放した時点の処理で送り、ダブルクリックになるかどうかを待ってから送る作りにしない。
6. If `OnMouseUp`（`OnMouseUpEx`）の送信が失敗した, the areka shall 失敗を記録に残し、台本が返らなかったときと同じに `OnMouseClick`（`OnMouseClickEx`）へ進む（選択肢・アンカーの 2 段と同じ扱い）。
7. The areka shall 右ボタンのクリックを、右クリックのメニューを出さないと決まったときにだけ、要件 5 のとおり Reference5＝`1` で送る。
8. If `OnMouseClick`（`OnMouseClickEx`）の送信が失敗した, the areka shall 失敗を記録に残し、台本が返らなかったときと同じに扱って、ゴーストを終わらせない（選択肢・アンカーの 2 段と同じ扱い）。

### Requirement 2: 中ボタンと拡張ボタンのクリック

**Objective:** ゴーストの作者として、中ボタンと拡張ボタン（戻る・進む）のクリックを `OnMouseUpEx` と `OnMouseClickEx` で受け取りたい。そうすれば、正典が勧める形でボタンを見分けられる。

#### Acceptance Criteria

1. When 利用者がキャラクターの窓の上で中ボタン・拡張ボタン 1・拡張ボタン 2 のどれかを押して、ドラッグにならずに同じ窓の上で放した, the areka shall 放した時点で `OnMouseUpEx` を送る（Reference5＝`middle`・`xbutton1`・`xbutton2`・ほかは要件 1.1 と同じ並び）。
2. When `OnMouseUpEx` に台本が返らなかった（204。送信の失敗は要件 1.6 と同じ扱い）, the areka shall 同じ Reference で `OnMouseClickEx` を続けて送る。
3. When `OnMouseUpEx` に台本が返った, the areka shall その台本を再生し、`OnMouseClickEx` を送らない。
4. The areka shall 中ボタンのクリックを `OnMouseUp`／`OnMouseClick`（Reference5＝`2`・正典の互換仕様）としては送らない。

### Requirement 3: クリックにしない操作

**Objective:** 利用者として、ドラッグやダブルクリックをしたときに余計な「クリック」の返事が出ないでほしい。そうすれば、窓を動かしたりダブルクリックしたりしたときの今の動きが変わらない。

#### Acceptance Criteria

1. When 押下がドラッグになった（ドラッグの閾値を越えた。閾値を越えたのと放したのが同じ瞬間の処理に入った場合を含む）後にボタンが放された, the areka shall その解放で `OnMouseUp`・`OnMouseClick`・`OnMouseUpEx`・`OnMouseClickEx` を送らない。
2. When ダブルクリックとして届いた押下（2 回目の押下）のボタンが放された, the areka shall その解放で `OnMouseUp`・`OnMouseClick`・`OnMouseUpEx`・`OnMouseClickEx` を送らない。
3. When キャラクターの窓の上で押したボタンが、別の窓の上か窓の外で放された, the areka shall クリックとして送らない。
4. When シェルの中のバルーン（箱）の選択肢の確定や中断に使われた押下が放された, the areka shall クリックとして送らない。
5. The areka shall ドラッグ・ダブルクリック・強制退避（Ctrl＋Shift＋左ダブルクリックと、結線前の Ctrl＋左ダブルクリック）の今の動きと、それらが SHIORI へ送るイベントを変えない。
6. When 1.〜4. のどれかでクリックを送らなかった, the areka shall 送らなかったことと理由を記録に残す。

### Requirement 4: ダブルクリックと他のイベントとの順

**Objective:** ゴーストの作者として、SHIORI に届くイベントの順が利用者の操作の順と同じであってほしい。そうすれば、「クリックの後にダブルクリック」の流れを辞書で正しく扱える。

#### Acceptance Criteria

1. When 利用者が左ボタンでダブルクリックした, the areka shall 1 回目の解放の `OnMouseUp`（返事が無ければ続く `OnMouseClick`）を送り、その後に 2 回目の押下の `OnMouseDoubleClick` を送る。
2. The areka shall `OnMouseUp` → `OnMouseClick`（`Ex` も同じ）の 2 段を、後から来た操作のイベントより先に送り終える。2 段の間に別の操作のイベントを挟まない。
3. The areka shall 右ダブルクリックの今の決まり（メニューが有効な間は `OnMouseDoubleClick`〔Reference5＝`1`〕を送らず、`popupmenu.visible`＝0 のときだけ送る）を変えない。
4. The areka shall 中・拡張ボタンのダブルクリックで `OnMouseDoubleClick`・`OnMouseDoubleClickEx` を送らない今の動きを変えない（1 回目の解放の `OnMouseUpEx`／`OnMouseClickEx` は要件 2 のとおり送る）。

### Requirement 5: 右ボタンのクリックと右クリックのメニュー

**Objective:** 利用者として、右クリックでメニューが出るときに、ゴーストが「右クリック」の台詞を同時に喋らないでほしい。ゴーストの作者として、メニューを抑えたゴーストでは右クリックを受け取りたい。

#### Acceptance Criteria

1. While 右クリックのメニューが有効である（`popupmenu.visible` が `0` でない・返事が無いか期限が切れて既定の表示になる場合を含む）, the areka shall 右ボタンの解放で `OnMouseUp`・`OnMouseClick` を送らず、今のとおりメニューを出す。
2. When 利用者がキャラクターの窓の上で右ボタンを押してドラッグにならずに同じ窓の上で放し、ゴーストが `popupmenu.visible`＝`0` でメニューを抑えていたためメニューを出さないと決まった, the areka shall 要件 1.1〜1.3 と同じ 2 段で、Reference5＝`1` の `OnMouseUp`（台本が返らなければ続く `OnMouseClick`）を送る。
3. The areka shall 右ボタンのクリックの位置と当たり判定を、メニューを出すかどうかが決まった時点でなく、右ボタンを放した時点のものにする。
4. The areka shall 右クリックのメニューの出し方・項目・照会・期限の今の動きを変えない。
5. When 右ボタンの解放でクリックを送らなかった, the areka shall メニューを出したために送らなかったことを記録に残す。
6. When ゴーストがメニューを抑えたと決まった, the areka shall その時点で右のクリックの 2 段を送る。右ボタンを放してから決まるまでの間に届いた移動・出入りのイベントは、右のクリックより先に送ってよい（議題 8・§8 に登記する）。

### Requirement 6: ホイール

**Objective:** ゴーストの作者として、キャラクターの上でホイールを回されたときに `OnMouseWheel` を受け取りたい。利用者として、ホイールを速く回しても areka やゴーストが重くならないでほしい。

#### Acceptance Criteria

1. When カーソルがキャラクターの窓の上にある間に縦のホイールが回された, the areka shall `OnMouseWheel` を送る（Reference0／1＝カーソルの x／y〔要件 1.4 と同じ空間〕・2＝回転の量と向き・3＝その窓のスコープ・4＝カーソルの位置の当たり判定の名前〔無ければ空〕・5＝`0`・6＝`mouse`）。
2. The areka shall Reference2 に、送るまでに足し合わせた回転量を整数で入れる。1 目盛りは OS の単位で 120、ホイールを上（向こう側）へ回すと正、下（手前）へ回すと負とする。
3. The areka shall 1 つの窓について `OnMouseWheel` を 100 ミリ秒に高々 1 回だけ送る。
4. When その窓で前の `OnMouseWheel` から 100 ミリ秒以上経ってからホイールが回された, the areka shall 待たずにすぐ送る。
5. When 100 ミリ秒の間隔の中でホイールが回された, the areka shall その回転を捨てずに足し合わせ、前の送出から 100 ミリ秒が経った時点で、次の回転を待たずに送る。
6. If 足し合わせた回転量が 0 になった, the areka shall `OnMouseWheel` を送らず、送らなかったことを記録に残す。
7. The areka shall 横のホイールの回転を `OnMouseWheel` として送らない。
8. The areka shall ホイールの間引きを窓（スコープ）ごとに別々に行い、別の窓の回転を足し合わせない。
9. The areka shall Reference0／1／4 に、足し合わせた中で最後の回転の時点のカーソルの位置と当たり判定を入れる。
10. If ホイールの知らせが、カーソルの下に無いキャラクターの窓に届いた（入力の焦点を持つ窓に届く OS の設定のとき・カーソルが絵の透けた所の上のとき）, the areka shall その窓の `OnMouseWheel` として送らない。

### Requirement 7: 当たり判定の出入り

**Objective:** ゴーストの作者として、カーソルが当たり判定に入ったとき・出たときに `OnMouseEnter`・`OnMouseLeave` を受け取りたい。そうすれば、ぱすたの × の印のように、乗ると絵を変える作りが areka でも動く。

#### Acceptance Criteria

1. When キャラクターの窓の上でカーソルの下の当たり判定が「無し」から当たり判定 A に変わった, the areka shall `OnMouseEnter`（Reference4＝A）を送る。
2. When カーソルの下の当たり判定が A から「無し」に変わった, the areka shall `OnMouseLeave`（Reference4＝A）を送る。
3. When カーソルの下の当たり判定が A から別の当たり判定 B に直に変わった, the areka shall `OnMouseLeave`（A）を送り、その後に `OnMouseEnter`（B）を送る。
4. When カーソルが当たり判定 A の上にあるままキャラクターの窓の外へ出た, the areka shall `OnMouseLeave`（A）を送る。
5. The areka shall `OnMouseEnter`／`OnMouseLeave` の Reference を正典の並びで入れる（0／1＝カーソルの x／y〔要件 1.4 と同じ空間・窓の外へ出たときはその窓の中で最後に見た位置〕・2＝`0`・3＝その窓のスコープ・4＝入った／出た当たり判定の名前・5＝`0`・6＝`mouse`）。
6. The areka shall 出入りを `OnMouseMove` の間引きにかけず、当たり判定が変わるたびに送る。
7. When 1 回のカーソルの動きで出入りと `OnMouseMove` の両方を送る, the areka shall 出入りを先に送る。
8. The areka shall 「今どの当たり判定の上か」を窓（スコープ）ごとに別々に覚える。
9. The areka shall 出入りをカーソルの動きのときにだけ確かめる。カーソルが止まったまま表情が替わって当たり判定が変わっても、次にカーソルが動くまで出入りを送らない。
10. When ゴーストの切り替え・終了・窓の作り直しで窓が無くなった, the areka shall その窓について覚えていた当たり判定を捨て、`OnMouseLeave` を送らない。
11. The areka shall `OnMouseEnterAll`・`OnMouseLeaveAll` を送らない（議題 5 の推奨）。

### Requirement 8: 話の最中と、受け付ける時期

**Objective:** 利用者として、ゴーストが話している最中でもクリックやホイールに反応してほしい。ゴーストの作者として、正典どおりの割り込みの決まりで動いてほしい。

#### Acceptance Criteria

1. While ゴーストが話している, when 本 spec の 7 つのイベントのどれかに台本が返った, the areka shall 今のマウスのイベントと同じく、今の話をその台本で置き換える（生きている決まり 10）。
2. While ゴーストが話している, the areka shall 本 spec の 7 つのイベントを、話していないときと同じ条件で送る（話の最中だからといって送らないことはしない）。
3. While 起動・終了の握手・ゴーストの切り替えの途中で、マウスのイベントを受け付けない時期である, the areka shall 今のマウスのイベントと同じく本 spec の 7 つのイベントを送らず、送らなかったことを記録に残す。
4. While 右クリックのメニューが表示されている, the areka shall メニューへの操作を本 spec のイベントとして送らない（今の決まりのまま）。

### Requirement 9: 記録

**Objective:** 開発者として、クリック・ホイール・出入りを送ったか、送らなかったならなぜかを記録で分かりたい。そうすれば、「何も喋らない」ときに areka が送っていないのかゴーストが返事をしていないのかを見分けられる。

#### Acceptance Criteria

1. When 本 spec の 7 つのイベントのどれかを SHIORI へ送った, the areka shall イベントの名前・スコープ・当たり判定の名前・ボタン（ホイールは回転量）を記録に残す。
2. When `OnMouseUp`（`OnMouseUpEx`）に台本が返って `OnMouseClick`（`OnMouseClickEx`）へ進まなかった, the areka shall 2 段目へ進まなかったことを記録に残す。
3. When クリック・ホイールを送らなかった, the areka shall 理由（ドラッグ・ダブルクリックの解放・窓の外で放した・箱の選択肢・メニューを出した・受け付けない時期・回転量が 0）を記録に残す。
4. The areka shall ホイールの 1 回ごとの回転と、カーソルの 1 回ごとの動きを、既定の記録の水準（info 以上）で 1 行ずつ残さない（ホイールは足し合わせた結果を送るか捨てるときにだけ残す）。

### Requirement 10: 網羅台帳と、areka が決めた点の登記

**Objective:** 開発者として、網羅台帳と互換の設計書が実物と合っていてほしい。そうすれば、正典のどこまでを areka が持っているかを台帳で引ける。

#### Acceptance Criteria

1. The 本 spec shall 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `OnMouseUp`・`OnMouseUpEx`・`OnMouseClick`・`OnMouseClickEx`・`OnMouseWheel`・`OnMouseEnter`・`OnMouseLeave` の 7 行を、実装済み・持ち主＝本 spec・実物の在りかで書き直す。
2. The 本 spec shall `OnMouseDown`・`OnMouseDownEx`・`OnMouseDoubleClickEx`・`OnMouseMultipleClick`・`OnMouseHover`・`OnMouseGesture`・`OnMouseEnterAll`・`OnMouseLeaveAll` の行を変えない（議題 5 で入れると決めたときは `OnMouseEnterAll`・`OnMouseLeaveAll` も 1. に加える）。
3. The 本 spec shall 網羅台帳の報告（生成物）を手で直さず、生成の道具で作り直す。
4. The 本 spec shall `doc/COMPAT_ARCHITECTURE.md` §8 の表の末尾に、正典が黙る所で areka が決めた点を登記する: ドラッグの後とダブルクリックの 2 回目の解放でクリックを送らないこと・右ボタンのクリックとメニュー（右のクリックが放した後の移動・出入りより後に届くことを含む）・中ボタンを `OnMouseClick`（Reference5＝2）で送らないこと・2 段の送信の失敗を 204 と同じに扱うこと・ホイールの単位と間引きと横のホイール・出入りの順と窓の外へ出たときと表情が替わったとき。SSP で測った点は測った版と答えを添える。

### Requirement 11: 決定論の自動テスト

**Objective:** 開発者として、本 spec の決まりを、OS のマウスや実際の時間の経過に頼らない自動テストで確かめたい。そうすれば、後の変更で崩れたときにすぐ分かる。

#### Acceptance Criteria

1. The 本 spec shall 次の各場合を、本物のマウスの操作も実際の待ち時間も使わない決定論の自動テストで確かめる: 左・右・中・拡張 2 つのボタンごとの送るイベントと Reference／`OnMouseUp` に台本が返ったとき・返らなかったとき・送信に失敗したときの 2 段目／ドラッグの後は送らない／ダブルクリックの 2 回目の解放は送らない・1 回目のクリックがダブルクリックより先に届く／窓の外で放したとき／箱の選択肢の押下／右クリックでメニューが有効なとき・抑えたとき／話の最中に台本が返ったとき／受け付けない時期。
2. The 本 spec shall ホイールについて、静かな所からの最初の回転をすぐ送ること・間隔の中の回転を足し合わせること・回転が止まった後の残りを間隔が明けた時点で送ること・和が 0 のとき送らないこと・横のホイール・窓ごとに別々であることを、時刻を注入する決定論のテストで確かめる。
3. The 本 spec shall 出入りについて、無し→A・A→無し・A→B の順・窓の外へ出たとき・`OnMouseMove` が間引かれたときも出入りは送ること・出入りが `OnMouseMove` より先であること・窓ごとに別々であること・窓が無くなったときを確かめる。
4. The 本 spec shall 今のダブルクリック・ドラッグ・右クリックのメニュー・強制退避のテストが、変えずに通ることを確かめる。
5. The 本 spec shall 窓の層に足した 3 つ（ホイールの量を受け手へ渡す・拡張ボタンの押下を配る・ドラッグの終わりに閾値を越えたかを残す）を、窓の層の決定論のテストで確かめる。ホイールの量が 1 回の処理で渡された後に空になること、閾値を越えたのと放したのが同じ瞬間の処理に入った場合も含める。

### Requirement 12: 実機での確かめ

**Objective:** 開発者として、ゴースト「窓際のぱすたさん」で、SSP と同じにクリックと出入りが効くことを実機で確かめたい。そうすれば、申し送りの件 10〜13 が解けたと言える。

#### Acceptance Criteria

1. When ぱすたの手順書 `doc/pasta-real-run.md` の段 3 と段 7 を、画面のロックを外した在席の机で回した, the areka shall × に乗ると × が赤くなり、離れると元へ戻る（件 10）。
2. When 同じ手順でボールを左クリックした, the areka shall ボールのトークを出す。初回の起動ではボールのクリックで身体が映り、その後の撫でと自然なトークが回る（件 11・件 12）。
3. When 同じ手順で × を左クリックした, the areka shall 終わりの台詞の後にゴーストを終える（件 13）。
4. The 本 spec shall 実機の確かめの記録に、ぱすたの記録の `OnMouseEnter`・`OnMouseLeave`・`OnMouseClick` の行と、areka の記録の送った行を、撮った絵と並べて残す。
5. The 本 spec shall 同じ実機の走りで、右クリックのメニューが今のとおり出て、ダブルクリックとドラッグが今のとおり効くことを確かめる。
