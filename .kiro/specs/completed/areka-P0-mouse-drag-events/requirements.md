# Requirements Document

## Project Description (Input)
**誰が困っているか**: 既存のゴーストを areka で動かす利用者と、そのゴーストの作者。キャラクターをドラッグで動かしても、ゴーストが反応しない。SSP では、ゴースト「悪役令嬢クローディア」がドラッグの間だけ足の浮いた絵に替わり、離すと元の顔に戻ってひとこと言う。areka ではこれが一切起こらない。

**今どうなっているか**: クローディアは SHIORI イベント `OnMouseDragStart`・`OnMouseDragEnd` でこれをしている（辞書 `ghost/master/dic/normal/yaya_mouse.dic`。開始では Reference5 が `0` のときだけ、Reference3 が `0` なら `\0\s[29]\e`、`1` なら `\1\s[19]\e` を返す。終了では同じ判定のあと、元の顔へ戻して台詞を返す）。areka が SHIORI へ送るマウスのイベントは `OnMouseMove` と `OnMouseDoubleClick` の 2 つだけで、この 2 つは送っていない。網羅の台帳でも 2 つとも未対応（優先度 A12・価値「触れ合い」）。窓のドラッグそのものは動いており、閾値（5 px）を越えたら開始、離すか取り消すかで終了の知らせが出ている。動かさないクリックでは開始も終了も出ない（完了 spec `areka-P0-drag-click-without-move`）。

**何を変えるか**: キャラクター窓を閾値を越えてドラッグし始めたら `OnMouseDragStart` を 1 回、離したら（取り消しも含む）`OnMouseDragEnd` を 1 回、ukadoc の並びの 7 つの Reference で SHIORI へ送る。動かさないクリックではどちらも送らない。窓の位置の保存は今のまま続ける。届くこと・届かないことを決定論のテストで固定し、実機でクローディアの反応を確かめ、網羅の台帳の 2 行を実装済みへ直す（詳細は brief.md）。

## Introduction
キャラクターをドラッグで動かしたことを、正典（ukadoc）の SHIORI イベント `OnMouseDragStart`・`OnMouseDragEnd` でゴーストへ知らせる。既にある `OnMouseDoubleClick` と同じ並びの Reference を使い、他のマウスのイベントと同じ「いつ送ってよいか」の決まりに従う。

正典の根拠: ukadoc [OnMouseDragStart](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseDragStart:1)（「マウスをドラッグし始めた際に発生。ただしパッシブモードでは抑制される。」）・[OnMouseDragEnd](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseDragEnd:1)（「マウスをドラッグし終えた際に発生。」同じくパッシブモードでは抑制）。Reference は 2 つとも同じで、Reference0／1＝マウスカーソルの x／y 座標（ローカル座標）・Reference2＝マウスホイールの回転量および回転方向・Reference3＝本体 0／相方 1（SSP では 2 以降もある）・Reference4＝当たり判定の識別子・Reference5＝左クリック 0／右クリック 1・Reference6＝入力デバイス（タッチパネル `touch`・ペン `pen`・ペンの消しゴム側 `eraser`・マウスなど `mouse`）。

### 用語
- **キャラクター窓**: 本体・相方などのキャラクターの絵を出す窓。バルーン（吹き出し）の窓は含まない。
- **ドラッグ**: キャラクター窓を左ボタンで押したまま、閾値を越えて動かすこと。窓は指に付いて動く。
- **開始**: ドラッグが閾値を越えて始まったこと。位置は押した位置。
- **終了**: 開始したドラッグが、ボタンを離して終わったこと、または取り消されたこと。位置は離したときは離した位置、取り消しのときは押した位置（取り消しでは窓が開始の位置へ戻るため）。
- **動かさないクリック**: 押してから閾値を越えずに離した操作（閾値を越えずに離したダブルクリックの各押下も含む）。
- **定常**: ゴーストが起動を終え、終了や切替の途中でない状態。今の他のマウスのイベントはこの間だけ送られる。
- **終了の握手の待ち**: 終了の要求を受けて、SHIORI とのやり取りで終わる段取りを待っている間。今の他のマウスのイベントはこの間送られない。

## Boundary Context
- **In scope**: `OnMouseDragStart`・`OnMouseDragEnd` の組み立てと送出・SHIORI へ送ってよいイベントの表への 2 行（ukadoc の URL の注記つき）・キャラクター窓での開始と終了の捕まえ方・窓の位置の保存と並べること・決定論のテスト・網羅の台帳の 2 行とその生成物・実機でのクローディアを使った確認。
- **Out of scope**:
  - パッシブモードでの抑え（areka にはパッシブモードへ入る経路がまだ無い。抑えを置く場所に印だけ残す＝要件 7.3）。
  - 右ボタンのドラッグ（今の窓のドラッグは左ボタンでだけ始まる。右ドラッグを有効にするかは別の話）。よって Reference5 は常に `0`。
  - バルーン窓のドラッグ（ukadoc のイベントはキャラクターのもの。今の「バルーンからマウスのイベントを送らない」決まりのまま）。
  - タッチ・ペン（Reference6 は今の他のマウスのイベントと同じく `mouse`）。
  - `OnMouseClick`・`OnMouseDown`／`OnMouseUp` など、まだ無い他のマウスのイベント。
  - ドラッグの間の `OnMouseMove` の送り方（今のまま変えない）。
- **Adjacent expectations**:
  - 窓のドラッグの仕組み（閾値・開始と終了の知らせ・取り消し・動かさないクリックでは知らせが出ないこと）は、完了 spec `event-drag-system`・`areka-P0-drag-click-without-move` の約束をそのまま使い、本 spec は変えない。開始の知らせを伴わない終了の知らせは来ないものとして扱う。
  - 窓の位置の保存の中身（何をいつ記憶へ書くか）は今の仕組みの持ち物で、本 spec は変えない（要件 6）。
  - 他のマウスのイベントの「いつ送ってよいか」の決まり（定常だけ・終了の握手の待ちは送らない・会話中は実行状態に talking を付けて送る）と、「SHIORI との往復は一度に 1 つまで」の決まりをそのまま使い、2 つのイベントのために例外を作らない。
  - SHIORI の失敗の扱い（エラー応答は致命でない・輸送路の失敗はそのゴーストの SHIORI の故障）は完了 spec `areka-P0-shiori-fault-notice` の決まりをそのまま使う。
  - `areka-P0-balloon-lifecycle-events`（送ってよいイベントの表とその個数のテストに行を足す）・`areka-P0-sakura-time-critical`（同じマウスの振り分けの場所を触る）とは同時に進めない。
- **開発上の制約**: 1 ファイル 1,000 行未満を保つ（kanade の定常の振り分けのファイル〔929 行〕・運行表の本体のファイル〔937 行〕が本 spec の変更で上限を超えるなら、振る舞いを変えずに先に分ける）。1 フレーム遅らせる解は取らない。決定論のテストは x64 の偽の境界で組む。実機の根と一時フォルダはワークツリーの `target\` の下だけに置く。areka 本体のクレートで触るのはマウスの入力の配線（`input_events`）だけとし、`emo2_boot/`・`frame/` には触れない。

## Requirements

### Requirement 1: ドラッグの開始を届ける
**Objective:** As a ゴースト作者, I want キャラクターを掴んで動かし始めたことを `OnMouseDragStart` で受け取りたい, so that ドラッグの間だけ絵を替えるなどの反応ができる

#### Acceptance Criteria
1. When 利用者がキャラクター窓を左ボタンで押して閾値を越えて動かし、ドラッグが始まったとき, the areka shall `OnMouseDragStart` を GET でその窓のゴーストの SHIORI へ 1 回送る。
2. The areka shall 1 回のドラッグにつき `OnMouseDragStart` を 1 回だけ送る（開始の知らせが複数の段を通って受け手に届いても、重ねて送らない）。
3. When キャラクター窓のドラッグが始まったとき, the areka shall 押した場所が当たり判定の内か外か、シェルの絵の中の箱（シェルの中のバルーン）の上かどうかに関わらず `OnMouseDragStart` を送る（窓のドラッグが始まったかどうかだけで決める）。
4. The areka shall `OnMouseDragStart` を、マウスの移動の送出の間引きの対象にしない（間引きで落とさない・遅らせない）。

### Requirement 2: ドラッグの終了を届ける
**Objective:** As a ゴースト作者, I want 掴んで動かしたキャラクターを離したことを `OnMouseDragEnd` で受け取りたい, so that 元の顔に戻してひとこと言える

#### Acceptance Criteria
1. When 開始したドラッグで利用者が左ボタンを離したとき, the areka shall `OnMouseDragEnd` を GET でその窓のゴーストの SHIORI へ 1 回送る。
2. When 開始したドラッグが離す前に取り消されたとき, the areka shall 離したときと同じく `OnMouseDragEnd` を 1 回送り、Reference の並びも離したときと同じにする（取り消しを区別する値を足さない）。
3. The areka shall 1 回のドラッグにつき `OnMouseDragEnd` を 1 回だけ送る（終了の知らせが複数の段を通って受け手に届いても、重ねて送らない）。
4. The areka shall 同じドラッグの `OnMouseDragEnd` を、その `OnMouseDragStart` より後に SHIORI へ届ける（順を入れ替えない）。
5. The areka shall `OnMouseDragEnd` を、マウスの移動の送出の間引きの対象にしない。

### Requirement 3: 動かさない操作では送らない
**Objective:** As a 利用者, I want 動かさずにクリックしただけでドラッグの反応が出ないでほしい, so that 撫でやダブルクリックのつもりの操作で、ゴーストが「浮いた」反応をしない

#### Acceptance Criteria
1. If 利用者がキャラクター窓を押して閾値を越えずに離したとき, the areka shall `OnMouseDragStart`・`OnMouseDragEnd` のどちらも送らない。
2. If 利用者がキャラクター窓をダブルクリックし、どの押下も閾値を越えずに離したとき, the areka shall `OnMouseDragStart`・`OnMouseDragEnd` を送らない（`OnMouseDoubleClick` の送り方は今のまま）。2 回目の押下のまま閾値を越えて動かしたときは、要件 1・2 のドラッグとして扱う（`OnMouseDoubleClick` の後に `OnMouseDragStart` が送られる。押下とドラッグを突き合わせる仕組みは作らない）。
3. If 利用者がバルーン窓をドラッグしたとき, the areka shall `OnMouseDragStart`・`OnMouseDragEnd` を送らない。
4. If 利用者がキャラクター窓を右ボタンで押したまま動かしたとき, the areka shall `OnMouseDragStart`・`OnMouseDragEnd` を送らない（今の窓のドラッグは左ボタンでだけ始まる）。

### Requirement 4: Reference の並び
**Objective:** As a 既存ゴーストの辞書, I want 正典どおりの 7 つの Reference で 2 つのイベントを受け取りたい, so that SSP 向けに書かれた判定（左ボタンか・本体か相方か・どの当たり判定か）がそのまま働く

#### Acceptance Criteria
1. The areka shall `OnMouseDragStart`・`OnMouseDragEnd` の Reference を、Reference0〜6 の 7 つとし、`OnMouseDoubleClick` と同じ並びにする。
2. The areka shall Reference0／1 を、他のマウスのイベントと同じ座標の空間（当たり判定を引くのと同じ、キャラクターの絵の座標）の x／y とし、`OnMouseDragStart` では押した位置、`OnMouseDragEnd` では終わった位置（離したときは離した位置、取り消しのときは押した位置）を入れる。取り消しでは窓が開始の位置へ戻るので、取り消しの `OnMouseDragEnd` の Reference0／1 は `OnMouseDragStart` と同じ値になる（正典は取り消しについて書いていない＝要件 7.4 の裁量の記録に載せる）。
3. The areka shall Reference2 を `0` とする。
4. The areka shall Reference3 を、ドラッグしたキャラクター窓のスコープの番号（本体 `0`・相方 `1`）とし、他のマウスのイベントと同じ引き方で得る（今の areka は 3 体目以降のキャラクター窓を作らない。作れるようになったときは他のマウスのイベントと一緒に直す）。
5. The areka shall Reference4 を、Reference0／1 の位置にある当たり判定の識別子とし、当たり判定が無い位置では空文字列とする。
6. The areka shall Reference5 を `0`（左ボタン）とする。
7. The areka shall Reference6 を `mouse` とする。
8. The areka shall 2 つのイベントに、他のマウスのイベントの GET と同じ要求の見出し（実行状態 `Status` など）を付ける。

### Requirement 5: いつ送ってよいか
**Objective:** As a areka の保守者, I want 2 つのイベントが他のマウスのイベントと同じ決まりで送られてほしい, so that 終了や切替の段取りの最中に余計な往復が割り込まない

#### Acceptance Criteria
1. While 定常の間, the areka shall 2 つのイベントを送り、会話の再生中は他のマウスのイベントと同じく実行状態に `talking` を付ける。
2. While 終了の握手の待ちの間, the areka shall 2 つのイベントを送らず、送らなかったことを記録に 1 件残す。
3. While 定常でない間（起動の途中・終了の途中・ゴーストの切替の途中など）, the areka shall 2 つのイベントを送らず、送らなかったことを記録に 1 件残す。
4. If `OnMouseDragStart` を送った後、終了までの間に定常でなくなったとき, the areka shall `OnMouseDragEnd` にも要件 5.2・5.3 の決まりをそのまま当てはめ、開始と対にするための例外を作らない。
5. The areka shall 2 つのイベントを他のマウスのイベントと同じ順番待ちに載せ、「SHIORI との往復は一度に 1 つまで」の決まりに例外を作らない。

### Requirement 6: 窓の位置の保存と並べる
**Objective:** As a 利用者, I want ドラッグの終了をゴーストへ知らせるようになっても、窓の位置の記憶が今どおりであってほしい, so that 次の起動でも動かした位置にキャラクターが立つ

#### Acceptance Criteria
1. When ドラッグが終わったとき, the areka shall 窓の位置の保存を、本 spec の前と同じ条件で行い、同じ値を記憶へ書く（`OnMouseDragEnd` の送出を足しても保存の有無と値を変えない）。
2. If `OnMouseDragEnd` を送らないとき（要件 5.2・5.3 に当たるとき、または送り先が無いとき）, the areka shall それでも窓の位置の保存を本 spec の前と同じに行う。
3. The areka shall 動かさないクリックで窓の位置を保存しない決まり（完了 spec `areka-P0-drag-click-without-move`）を変えない。

### Requirement 7: 正典の注記と網羅の台帳
**Objective:** As a 互換の網羅を追う開発者, I want 2 つのイベントが実装済みとして台帳に載ってほしい, so that 何が届き何が届かないかを台帳だけで判断できる

#### Acceptance Criteria
1. The areka shall `OnMouseDragStart`・`OnMouseDragEnd` を SHIORI へ送ってよいイベントの表に加え、他の行と同じ形で ukadoc の URL の注記を付ける。
2. The areka shall 網羅の台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）の `OnMouseDragStart:1`・`OnMouseDragEnd:1` の 2 行を実装済み（担当は本 spec）に改め、備考を実装済みの他の行（`OnMouseDoubleClick:1` など）と同じ形（壊れ方・ログ・根拠の場所・組み立ての場所・無いと失うもの）で書き直し、台帳から作る報告（`doc/ukadoc-coverage/report/` の該当の生成物）を作り直し、網羅の検査（`ukadoc-survey` のテスト）が台帳と突き合わせる文書（`briefing.md` の状態の数・`roadmap-draft.md` の担当 spec の行など）も検査が緑になるように直す。
3. The areka shall パッシブモードでの抑えを持たないことを、台帳の 2 行の備考と、送るかどうかを決める場所の印（パッシブモードへ入る経路ができたときに抑えを置く場所）の 2 か所に残す。
4. The areka shall 正典が書いていない点について本 spec で決めたこと（取り消しでも終了を送ること・取り消しの位置・Reference2 を `0` とすること など）を、正典の沈黙箇所の裁量の記録（`doc/COMPAT_ARCHITECTURE.md` §8 の表）に追記する。

### Requirement 8: 記録
**Objective:** As a 実機で確かめる開発者, I want 送ったことも送らなかったことも記録で追えてほしい, so that 反応が無いときに、どこで止まったかが分かる

#### Acceptance Criteria
1. When 2 つのイベントのどちらかを送るとき, the areka shall 他の SHIORI イベントと同じく、イベントの名前と Reference を含む記録を 1 件残す。
2. The areka shall 開始・終了の知らせを受けてから送らないと決めるすべての経路（定常でない・終了の握手の待ち・送り先が無い、および重ねて届いた知らせを捨てる経路を設けるならその経路）に記録を残し、記録の無いまま捨てる経路を 0 本とする。

### Requirement 9: 検証
**Objective:** As a 開発者, I want 届くこと・届かないことが決定論のテストと実機で確かめられていてほしい, so that 後から他のマウスのイベントを足しても壊れたことに気付ける

#### Acceptance Criteria
1. The areka shall 次を DLL を使わない決定論のテスト（x64 の偽の境界）で確かめる: ⑴ ドラッグ 1 回で開始 1 件 → 終了 1 件がこの順に送られること ⑵ 動かさないクリック・ダブルクリックで 0 件であること ⑶ 取り消しでも終了が 1 件送られること ⑷ 知らせが複数の段を通っても 1 回ずつであること ⑸ Reference0〜6 の中身（座標の空間・当たり判定の有無・スコープ・`0`・`mouse`） ⑹ 終了の握手の待ちの間と定常でない間に送られないこと ⑺ 窓の位置の保存が本 spec の前と同じであること ⑻ 移動の間引きを受けないこと ⑼ 送ってよいイベントの表に 2 つが載っていること。
2. The areka shall 要件 9.1 の各項目について、その決まりを外すと赤になるテストを置く（届く側のテストは、送出を外すと赤・届かない側のテストは、送らない判断を外すと赤）。
3. When 実機でクローディアを起動し、本体をドラッグしたとき, the areka shall ドラッグの間に本体の絵を `\s[29]` にし、離したときにクローディアの台詞を表示する（相方は `\s[19]` と相方の台詞）。この確認の根と一時フォルダはワークツリーの `target\` の下に置き、記録の照合（2 つのイベントの送出の記録が開始 → 終了の順に 1 件ずつあること）で裏付ける。
4. The areka shall ワークスペース全体のテスト（`tools/test-all.ps1` の手順）を緑で通す。
