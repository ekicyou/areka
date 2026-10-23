# Requirements Document

## Project Description (Input)
ゴースト作者が台本で書く、窓どうしをつなぐ指令 `\![set,sticky-window,スコープID,スコープID,...]`／`\![reset,sticky-window]` と、窓を画面の端へ吸着させる指令 `\![set,alignmenttodesktop,方向]` を、areka は受けない。どちらも運ばれるだけで誰も受け取らず、黙って何も起きない。シェルの descript の `seriko.sticky-window` も生のまま書き写すだけで効かず、`\![move]` の `--option=ignore-sticky-window` も、つなぐ仕組みが無いので意味を持たない。

v0.1.0 のゴール「ぱすたが SSP と同じレベルで動く」がこれを待っている。ゴースト「窓際のぱすたさん」は、シェルの descript に `seriko.sticky-window,0,1` を書いて、窓の枠（相方側 `\1`）とぱすた（本体側 `\0`）を同じ位置に重ねて一緒にドラッグさせ、辞書の「窓あわせ」で `\![move,…,--option=ignore-sticky-window]` を出して相方側だけを動かす。今の areka では 2 つの窓が別々に動き、ぱすたが窓の枠から離れてしまう。

この spec は、ukadoc の正典どおりに吸着の台本の指令・窓の組（sticky-window）・`ignore-sticky-window` の効き目を入れる。あわせて、完了 spec `areka-P0-move-canon` から申し送られた `\![execute,resetballoonpos]`・メニューの「バルーン位置初期化」・位置初期化の 2 つのキーボードの近道を受ける。時間つきの移動での連れ動き・ほかのアプリの窓への吸着・3 人目以降の窓は範囲外。詳細は同じフォルダの `brief.md`。

## Introduction

ukadoc は、キャラクターの窓の位置について次の指令を定めている（2026-10-22 に ukadoc を読み直した）。

- **`\![set,alignmenttodesktop,方向]`**: 「現スコープ側サーフェスをデスクトップの指定した方向に吸着」。方向は `top`・`bottom`・`left`・`right`・`free`（吸着なし）・`default`（ゴーストのデフォルト設定に戻す）。「上または下に吸着した場合、上下方向へのドラッグ移動ができなくなる。左または右に吸着した場合、左右方向へのドラッグ移動ができなくなる」「ゴースト終了まで有効」「alignmentondesktopと書いても有効」。兄弟の項目に `\![set,alignmenttodesktop,free]`（「サーフェスを自由移動可能にする」）と、古い綴りの `\![set,alignmentondesktop,bottomまたはtop]` がある。
- **`\![set,sticky-window,スコープID,スコープID,...]`**（2.3.77）: 「指定スコープのウインドウをドラッグ中やmoveタグを実行中にセットで動くようにする」。ID の違うタグを何度も打つと組が複数できる。「すでにペアにしたスコープIDを含むタグを実行するとエラー」。「\![set,alignmenttodesktop]などでウインドウ位置に制限がある場合は、その制限が優先され、X,Yのうち自由に動かせるものだけが追従する」。「\![move]タグでグループ化したウインドウ内の1つを動かした場合でも、他のウインドウも追従する。ただし、moveタグの --option=ignore-sticky-window で単独移動可能になる」。「ゴースト終了まで有効」。
- **`\![reset,sticky-window]`**（2.3.77）: 「上記指定を解除し、descriptであらかじめ指定された分にリセットする」。
- **シェルの descript の `seriko.sticky-window,スコープID,スコープID,...`**（2.4）: 「\![set,sticky-window,…]のdescript版。タグを実行しなくてもあらかじめ設定できる」。
- **`\![execute,resetballoonpos]`**（2.7.41）: 「各バルーンの位置を初期位置に戻す。メニューの「バルーン位置初期化」と同じ働きをする」。項目の名前は SHIORI リソース `resetballoonpositionbutton.caption`。SSP ヘルプ「機能」は「ドラッグして動かしたバルーンが、各キャラクターに対する標準の位置に戻ります」と書き、近道を「バルーン位置初期化」Ctrl＋Shift＋F10・「シェル位置初期化」Ctrl＋Shift＋F12 とする（「ショートカット一覧」も同じ）。

今の areka は、descript の `seriko.alignmenttodesktop` の 4 段の優先（シェルのスコープ個別＞シェルの全体＞ゴーストのスコープ個別＞ゴーストの全体）で吸着の向きを決め、その向きでドラッグを縛り、表情の切り替えで窓の大きさが変わったときと `\![move]` のときにその向きへ当て直す。台本から向きを変える口と、窓どうしをつなぐ仕組みが無い。バルーンの位置はドラッグで変えられて記憶されるが、元へ戻す口が無い。

ukadoc が黙っている点は、本 spec の「要件の段での裁定」に答えを置いた（2026-10-22 の要件ディスカッションで全行を確定）。SSP で測った結果は `research.md` 1.10 にある。

### 用語

- **キャラクターの窓**: スコープ（本体側 0・相方側 1 …）ごとに 1 枚ある、シェルの絵を出す窓。バルーンの窓はこれに付いて動く。
- **吸着**: キャラクターの窓を、その窓がいるモニタの作業領域（タスクバーを除いた矩形）の辺へ沿わせること。向きは上・下・左・右・なし（`free`）。
- **descript の吸着の向き**: descript の `seriko.alignmenttodesktop` 系の 4 段の優先で決まる向き（今の areka が起動で使う向き）。
- **窓の組**: `\![set,sticky-window]` または descript の `seriko.sticky-window` で、一緒に動くように結んだキャラクターの窓の集まり。
- **組の相手**: 窓の組の中で、動かした窓以外の窓。
- **位置の記憶**: 次の起動でキャラクターの窓とバルーンを戻すために areka が覚えておく位置。
- **バルーンの標準の位置**: バルーンの位置の記憶が無いときに、キャラクターの窓に対して areka がバルーンを置く位置（descript のバルーンのずらしの指定から決まる）。
- **拡大率**: 窓に今かかっている、画面の DPI とシェルの倍率を合わせた倍率。

## 要件の段での裁定

下の表は、要件を書く途中で答えが要った点に推奨案で答えを置いたもの。2026-10-22 の要件ディスカッションで、開発者が全行を推奨案どおりに承認して確定した（行 3 は SSP で測った結果を受けて推奨を改めたもの）。

| # | 問い | 答え | 根拠 | 該当 |
|---|---|---|---|---|
| 1 | ゴーストの読み直しの後に、台本で変えた吸着と台本で作った組を保つか | **保たない**（descript の値へ戻す）。SHIORI だけの読み直しでは保つ | SSP ヘルプ「ゴースト再読み込み」は「いま起動しているゴースト自身を選び直したのと同じ」＝ゴーストの終了を伴う。ぱすたも SSP で読み直しの後に「窓あわせ」をやり直している。areka の兄弟の指令 `\![set,zorder]` の組も、読み直しで descript の値へ戻る。SHIORI だけの読み直しは窓を残し、ゴーストを終えない。SSP の読み直しは測っていない（開発者の許可が要り、その時 SSP は別の作業に使われていた） | 6.2・6.3 |
| 2 | シェルの切り替え（とシェルの読み直し）の後に、台本で変えた吸着と台本で作った組を保つか | **保つ**。descript の値に従っていたスコープと descript の組だけを、新しいシェルの descript の値へ取り替える | 正典は「ゴースト終了まで有効」と書き、シェルの切り替えはゴーストの終了ではない。兄弟の `\![set,zorder]` も正典に同じ「ゴースト終了まで有効」とあるのに、areka はシェルの切り替えで台本の組を捨てている。この食い違いは本 spec の完了のときに起票する（要件 10.6） | 6.4・6.5・10.6 |
| 3 | 吸着の向きが読めない値・欠けているとき | 読めない語は**吸着を外す**（`free` と同じ・窓は今の位置に留める）。値が欠けたときは**何も変えない**。どちらも記録し、トークは止めない | SSP の測り（`research.md` 1.10）: 下に吸着させた後の `foo` で `\![move]` の縦の縛りが外れ、値なしでは縛りが残った。どちらもエラーの記録は 0 件。作者は SSP で試して書くので SSP に合わせる。外れた先が「なし」か「descript の向き」かは、測った検体の descript がなしだったので見分けられず、「なし」と読んだ | 1.7・1.9 |
| 4 | 組を作る指令の、正典の「エラー」と、正典が黙る書き方（ID が 2 つ未満・同じ ID の重ね書き・数でない ID）の扱い | どれも組を作らずに捨て、ほかの組はそのまま、何を捨てたかを記録する（トークは止めない・利用者への知らせは出さない） | 正典「すでにペアにしたスコープIDを含むタグを実行するとエラー」。プロパティ `currentghost.seriko.sticky-window` の説明「要素が2つ未満のグループ、および既に他のグループで使われているスコープIDを含むグループは無視される」。兄弟の `\![set,zorder]` も同じ捨て方 | 3.4・3.5 |
| 5 | ドラッグで連れ動いた組の相手の位置を、位置の記憶に書くか | **書く**（ドラッグした窓と同じ時機に） | 利用者のドラッグの結果だから。書かないと、次の起動でドラッグした窓だけが動いた位置に出て、組が離れる（ぱすたが窓の枠から外れる） | 4.5 |
| 6 | 吸着の当て直し・表情の切り替えによる大きさの変化・作業領域の変化・位置の初期化で窓が動いたとき、組の相手も動かすか | **動かさない**（連れ動くのはドラッグと `\![move]` だけ） | 正典は「ドラッグ中やmoveタグを実行中にセットで動く」とだけ書く | 4.7・5.6 |
| 7 | 拡大率が変わった後の詰め直しが、重ねた組の窓を横に並べ直すこと | 本 spec は、どの窓がどの組にいるかを他の機能が確かめられるようにする。詰め直しから組の窓を外すのは `areka-P0-dpi-realign-remembered-chain` が持つ | 同 spec の brief は、組の持ち主（本 spec）の目印を見て外すだけにすると書く。詰め直しのファイルを本 spec が触らずに済む | Boundary Context・9.2 |
| 8 | バルーン位置初期化の後の位置の記憶 | 初期化したバルーンの位置を記憶に書く（次の起動も標準の位置に出る） | 「シェル位置初期化」（完了 `move-canon` 要件 8.8）と同じ扱い | 7.3 |
| 9 | キーボードの近道が効く場面 | ゴーストのいずれかの窓（キャラクターの窓・バルーンの窓）がキーボードの入力先のときだけ | SSP の「ショートカット一覧」は、この 2 つに「フォーカス無し可」を付けていない | 8.1・8.2 |

## Boundary Context

- **In scope**:
  - `\![set,alignmenttodesktop,方向]` の受け取り（綴り `alignmentondesktop`・方向 `free`・`default` を含む）と、それによる吸着とドラッグの縛りの変更
  - `\![set,sticky-window,…]`・`\![reset,sticky-window]` の受け取りと、シェルの descript の `seriko.sticky-window` を効かせること
  - ドラッグと瞬間の `\![move]` での組の連れ動き、`--option=ignore-sticky-window` の効き目
  - 台本で変えた吸着と組の寿命（ゴーストの終了・切り替え・読み直し・シェルの切り替え）
  - `\![execute,resetballoonpos]`、右クリックのメニューの「バルーン位置初期化」の項目、位置初期化の 2 つのキーボードの近道（「バルーン位置初期化」・「シェル位置初期化」）
  - ぱすた（「窓際のぱすたさん」）での実機の確かめ
  - 互換の対応表（`doc/COMPAT_ARCHITECTURE.md` §8）と網羅台帳の該当行の改め
- **Out of scope**:
  - 時間つきの移動（`--time` の補間）と `\![moveasync]` での組の連れ動き → dola の改造の後の時間の spec（時間つきの移動が入るとき、組の相手も同じ時間で動くことはその spec が引き取る）
  - 吸着の相手を作業領域の辺から、ほかのアプリの窓へ広げること → `areka-P0-window-snap`
  - 3 人目以降のキャラクターの窓（その窓を含む組の連れ動きの確かめを含む） → `areka-P0-extra-character-windows`
  - プロパティ `currentghost.seriko.sticky-window` の読み書き → `areka-P0-currentghost-property-tree`
  - 拡大率が変わった後の詰め直しから組の窓を外すこと → `areka-P0-dpi-realign-remembered-chain`（裁定 7）
  - 吸着の向きが上・左・右のときに、作業領域の変化で窓を辺へ当て直すこと（今の areka は下の吸着でだけ当て直す。descript で書いた向きと同じ扱いを保ち、本 spec では広げない）
- **Adjacent expectations**:
  - 台本の指令で動いた窓は位置の記憶に書かない、という今の決まり（完了 `areka-P0-char-position-save-on-exit`・COMPAT §8）を保ち、`\![move]` で連れ動いた組の相手にも当てる。ドラッグの結果は今どおり記憶に書く。
  - 吸着している窓への `\![move]` は吸着した軸に台本の値を使わない、という完了 `areka-P0-move-canon` の決まりを保つ。台本で向きを変えれば、`\![move]` もその向きに従う。
  - `areka-P0-dpi-realign-remembered-chain` は、本 spec が持つ「どの窓がどの組にいるか」を見て、詰め直しから組の窓を外す。`areka-P0-window-snap` は、本 spec の吸着（作業領域の辺）を土台にして、吸着の相手を広げる。
  - descript の `seriko.alignmenttodesktop` の 4 段の優先の読み方（完了 `areka-P0-window-placement`）は変えない。

## Requirements

### Requirement 1: 台本からの吸着の向きの変更

**Objective:** As a ゴースト作者, I want 台本でキャラクターの窓を画面の端へ吸着させたり外したりしたい, so that 場面に合わせて窓を画面の上や横に貼り付けたり、自由に置けるようにしたりできる

#### Acceptance Criteria

1. When 台本が `\![set,alignmenttodesktop,top]`・`bottom`・`left`・`right` のどれかを実行したとき, the areka shall その指令の位置の現スコープのキャラクターの窓を、その窓がいるモニタの作業領域の指定した辺へすぐ沿わせ、沿わせない方の軸の位置は保つ。
2. When 台本が `\![set,alignmenttodesktop,free]` を実行したとき, the areka shall 現スコープのキャラクターの窓の吸着を外し、窓を今の位置に留める。
3. When 台本が `\![set,alignmenttodesktop,default]` を実行したとき, the areka shall 現スコープの吸着の向きを descript の吸着の向きへ戻し、その向きが辺への吸着なら要件 1.1 と同じに窓を沿わせる。
4. The areka shall `\![set,alignmentondesktop,方向]` を `\![set,alignmenttodesktop,方向]` と同じに扱う。
5. The areka shall 台本で変えた吸着の向きを、以後の表情の切り替えによる窓の大きさの変化・作業領域の変化・`\![move]` で、descript に同じ向きを書いたときと同じに扱う。
6. When 吸着の指令で窓が動いたとき, the areka shall そのスコープのバルーンの窓をこれまでどおり窓に付いて動かし、窓の位置を位置の記憶に書かない。
7. If 方向が正典の 6 つの語のどれでもない場合, then the areka shall `free` を受けたときと同じに現スコープの窓の吸着を外して窓を今の位置に留め、受け取った値を記録し、トークを止めずに続ける（SSP の振る舞いに合わせる）。
8. If 現スコープのキャラクターの窓が無い場合, then the areka shall 何も変えず、そのことを記録し、トークを止めずに続ける。
9. If 方向が欠けている場合（`\![set,alignmenttodesktop]`）, then the areka shall 吸着の向きを変えず、そのことを記録し、トークを止めずに続ける。

### Requirement 2: 吸着とドラッグ

**Objective:** As a 利用者, I want 吸着した窓が吸着した向きにはドラッグで動かないでほしい, so that 画面の端に貼り付いた窓がうっかり離れない

#### Acceptance Criteria

1. While キャラクターの窓が上または下に吸着している間, when 利用者がその窓をドラッグしたとき, the areka shall 窓を左右にだけ動かし、上下には動かさない。
2. While キャラクターの窓が左または右に吸着している間, when 利用者がその窓をドラッグしたとき, the areka shall 窓を上下にだけ動かし、左右には動かさない。
3. While キャラクターの窓の吸着が無い（`free`）間, when 利用者がその窓をドラッグしたとき, the areka shall 窓を上下左右に自由に動かす。
4. When 台本で吸着の向きが変わったとき, the areka shall 次のドラッグから新しい向きの縛りを当てる。

### Requirement 3: 窓の組を作る・解く

**Objective:** As a ゴースト作者, I want 複数のキャラクターの窓を 1 つの組に結びたい, so that 重ねたり並べたりした窓が、離れずに一緒に動く

#### Acceptance Criteria

1. When 台本が `\![set,sticky-window,スコープID,スコープID,...]` を、異なる 2 つ以上のスコープ ID で、どれもまだ組に入っていない ID で実行したとき, the areka shall それらのスコープの窓を 1 つの新しい組に結び、既にある組はそのまま残す。
2. The areka shall 組の中の ID の並び順に意味を持たせず、組のどの窓を動かしても組の全員を連れ動かす（`1,0` と `0,1` は同じ組）。
3. The areka shall 組を複数持てるようにする（`\![set,sticky-window,1,0]` と `\![set,sticky-window,3,2]` を続けて実行すると、2 つの組ができる）。
4. If 指令のスコープ ID のどれかが既にほかの組（descript で作った組を含む）に入っている場合, then the areka shall その指令では組を作らず、既にある組を変えず、どの ID が重なったかを記録し、トークを止めずに続ける。
5. If 指令の異なるスコープ ID が 2 つ未満の場合、同じ ID が 2 回以上書かれた場合、または数として読めない ID がある場合, then the areka shall その指令では組を作らず、既にある組を変えず、受け取った引数と捨てた理由を記録し、トークを止めずに続ける。
6. If 組に含まれるスコープのキャラクターの窓が今は無い場合, then the areka shall 組をそのまま受け付け、無い窓は連れ動かす相手から外し、無いことを記録する。
7. When 台本が `\![reset,sticky-window]` を実行したとき, the areka shall 台本で作ったすべての組を解き、組を descript の `seriko.sticky-window` で作った組だけに戻す。
8. When ゴーストが起動したとき, the areka shall シェルの descript の `seriko.sticky-window,スコープID,スコープID,...` を、`\![set,sticky-window]` を実行したときと同じ規則で組にする（台本の指令を待たない）。
9. If descript の `seriko.sticky-window` の値が要件 3.5 の捨てる書き方に当たる場合, then the areka shall その組を作らず、受け取った値と理由を記録し、起動を続ける。

### Requirement 4: ドラッグでの組の連れ動き

**Objective:** As a 利用者, I want 組になった窓の 1 つをドラッグすると組の全員が一緒に動いてほしい, so that 重ねて表示されたキャラクターと窓の枠が離れない

#### Acceptance Criteria

1. While キャラクターの窓が組に入っている間, when 利用者がその窓をドラッグしたとき, the areka shall ドラッグの途中から、組の相手の窓を、ドラッグした窓が動いたのと同じ量だけ同じ向きに動かし、組の窓どうしの位置の関係を保つ。
2. While 組の相手の窓が吸着している間, when 組の相手を連れ動かすとき, the areka shall その相手を吸着していない軸にだけ動かす（吸着の制限を優先する）。
3. The areka shall ドラッグした窓自身の動きを、要件 2 の吸着の縛りの後の量で組の相手へ伝える（ドラッグした窓が動かなかった軸では、組の相手も動かさない）。
4. When 組の相手の窓が連れ動いたとき, the areka shall その相手のバルーンの窓をこれまでどおり相手の窓に付いて動かす。
5. When 組の窓のドラッグが終わったとき, the areka shall ドラッグした窓と、連れ動いた組の相手の窓の位置を、位置の記憶に書く。
6. When 利用者がバルーンの窓をドラッグしたとき, the areka shall そのバルーンのずらしだけを変え、キャラクターの窓と組の相手を動かさない（今の振る舞いを保つ）。
7. The areka shall 吸着の当て直し・表情の切り替えによる窓の大きさの変化・作業領域の変化で窓が動いたとき、組の相手を動かさない。
8. When 台本で吸着の向きを変えた窓（組の相手を含む）をドラッグした後に次の起動をしたとき, the areka shall その窓を、ドラッグを終えた位置に出す（記憶に書いた位置を次の起動の descript の向きで読み戻しても、窓の幅や高さの半分だけずれたりしない）。

### Requirement 5: `\![move]` での組の連れ動きと `ignore-sticky-window`

**Objective:** As a ゴースト作者, I want `\![move]` で組の 1 つを動かしたとき組の全員が付いてくるか、1 つだけを動かすかを選びたい, so that 組を保ったまま全体を動かすことも、組の中の位置合わせをすることもできる

#### Acceptance Criteria

1. While 動かす窓が組に入っている間, when 台本が `--option=ignore-sticky-window` の無い `\![move]` を実行したとき, the areka shall 動かす窓を `\![move]` の規則で動かし、組の相手の窓を、動かす窓が実際に動いたのと同じ量だけ同じ向きに動かす。
2. While 組の相手の窓が吸着している間, when `\![move]` で組の相手を連れ動かすとき, the areka shall その相手を吸着していない軸にだけ動かす。
3. When 台本が `--option=ignore-sticky-window` の付いた `\![move]` を実行したとき, the areka shall 動かす窓だけを動かし、組の相手を動かさず、組は解かずに残す（以後のドラッグと `\![move]` では、新しい位置の関係のまま組で動く）。
4. When `\![move]` で組の相手が連れ動いたとき, the areka shall その相手のバルーンの窓を相手の窓に付いて動かし、相手の位置を位置の記憶に書かない。
5. When 時間つきの `\![move]`（`time` が 0 より大きい）で組の窓が動くとき, the areka shall 組の相手も途中の位置を経ずに最終の位置へすぐ置く（動かす窓の今の扱いと同じ）。
6. When `\![execute,resetwindowpos]` または「シェル位置初期化」で窓を並べ直したとき, the areka shall 組に関わらず各窓をそれぞれの既定の位置へ置き、組は解かずに残す。
7. The areka shall 組に入っていない窓への `\![move]` の振る舞いを、本機能の前と同じに保つ。

### Requirement 6: 台本で変えた吸着と組の寿命

**Objective:** As a ゴースト作者, I want 台本で変えた吸着と組がいつまで効くかを正典どおりにしてほしい, so that 次の起動や切り替えの後の窓の動きを予想できる

#### Acceptance Criteria

1. The areka shall 台本で変えた吸着の向きと台本で作った組を、ゴーストが終わるまで保ち、次の起動では descript の値から始める（記憶に持ち越さない）。
2. When ゴーストを切り替えたとき（ゴーストの読み直しで同じゴーストを起こし直すときを含む）, the areka shall 新しく起こしたゴーストの吸着の向きと組を、そのゴーストの descript の値から始める。
3. When SHIORI だけを読み直したとき（`\![reload,shiori]` など）, the areka shall 台本で変えた吸着の向きと台本で作った組を保つ。
4. When シェルを切り替えたとき（シェルの読み直しを含む）, the areka shall 台本で吸着の向きを変えたスコープはその向きを保ち、descript の吸着の向きに従っていたスコープは新しいシェルの descript の吸着の向きへ取り替える。
5. When シェルを切り替えたとき（シェルの読み直しを含む）, the areka shall 台本で作った組を保ち、descript で作った組を新しいシェルの `seriko.sticky-window` の組へ取り替え、台本で作った組と ID が重なる descript の組は作らずに記録する。

### Requirement 7: バルーン位置初期化

**Objective:** As a 利用者, I want ドラッグで動かしたバルーンを一度に標準の位置へ戻したい, so that 画面の外へ行ったバルーンや、ずれたバルーンを元に戻せる

#### Acceptance Criteria

1. When 台本が `\![execute,resetballoonpos]` を実行したとき, the areka shall そのゴーストのすべてのバルーンの窓を、各キャラクターの窓に対するバルーンの標準の位置（今の拡大率で求めたもの）へ戻す。
2. The areka shall バルーン位置初期化でキャラクターの窓を動かさず、吸着の向きと組を変えない。
3. When バルーン位置初期化を行った後に次の起動をしたとき, the areka shall バルーンを標準の位置に出す（初期化の後に利用者がドラッグしたバルーンは、ドラッグした位置に出す）。
4. The areka shall バルーン位置初期化の結果を、起動の直後に行ったときとシェルを切り替えた後に行ったときで同じにする（同じシェル・同じ拡大率なら同じ位置）。
5. The areka shall 右クリックのメニューに「バルーン位置初期化」の項目を、「シェル位置初期化」と同じ場面で選べるように持つ。
6. When ゴーストが `resetballoonpositionbutton.caption` に名前を返したとき, the areka shall その名前で項目を出し、返さないときは既定の名前「バルーン位置初期化」で出す。
7. When 利用者が「バルーン位置初期化」の項目を選んだとき, the areka shall `\![execute,resetballoonpos]` と同じ初期化を行う（正典に対のイベントは無いので、ゴーストへ知らせを送らない）。

### Requirement 8: 位置初期化のキーボードの近道

**Objective:** As a 利用者, I want SSP と同じキーボードの近道で窓とバルーンの位置を戻したい, so that メニューを開きにくいときでも、見失った窓やバルーンを戻せる

#### Acceptance Criteria

1. While ゴーストのいずれかの窓（キャラクターの窓・バルーンの窓）がキーボードの入力先である間, when 利用者が Ctrl＋Shift＋F12 を押したとき, the areka shall 右クリックのメニューで「シェル位置初期化」を選んだときと同じ働きをする。
2. While ゴーストのいずれかの窓がキーボードの入力先である間, when 利用者が Ctrl＋Shift＋F10 を押したとき, the areka shall 右クリックのメニューで「バルーン位置初期化」を選んだときと同じ働きをする。
3. While その項目がメニューで選べない場面の間, when 利用者が対応する近道を押したとき, the areka shall 何もせず、そのことを記録する。
4. While ゴーストの窓がキーボードの入力先でない間, the areka shall これらの近道を受けない（ほかのアプリでの同じキーの操作を横取りしない）。

### Requirement 9: 記録と確かめ

**Objective:** As a 開発者, I want 吸着・組・連れ動き・初期化をログとテストで判定したい, so that 目視だけに頼らず後から確かめ直せる

#### Acceptance Criteria

1. When 吸着の向きが変わったとき, the areka shall スコープ・前の向き・新しい向き・変えた出どころ（台本・descript・切り替え）を記録する。
2. When 組ができたとき・解いたとき・捨てたとき, the areka shall その時点のすべての組の中身（スコープ ID の集まり）と、出どころ（台本か descript か）を記録する。
3. When `\![move]` で組の相手が連れ動いたとき、または組の窓のドラッグが終わったとき, the areka shall 動かした窓・連れ動いた相手・動いた量・きっかけ（ドラッグか `\![move]` か）を記録する（ドラッグの途中の 1 回ごとの動きは記録しない）。
4. When `\![move]` が `ignore-sticky-window` で組の相手を動かさなかったとき, the areka shall そのことを記録する。
5. When バルーン位置初期化を行ったとき, the areka shall 対象のバルーンと戻した位置、きっかけ（台本・メニュー・近道）を記録する。
6. The areka shall 吸着の指令の読み取り（6 つの語・綴り・読めない値）・吸着の縛り・組を作る規則と捨てる規則・連れ動きの量と吸着の制限・`ignore-sticky-window`・寿命（切り替え・読み直し）・バルーン位置初期化の位置と記憶の各分岐を、実際の画面を使わない決定論のテストで確かめられるようにする。
7. The areka shall ぱすた（「窓際のぱすたさん」）を動かし、本体側の窓をドラッグしたときに相方側の窓とバルーンが一緒に動くこと、「窓あわせ」の `\![move,…,--option=ignore-sticky-window]` で相方側だけが動くこと、次の起動で 2 つの窓が重なったまま出ることを、窓の位置と記録から判定できる実機の証跡を残す。
8. The areka shall 実機の確かめに使う根と検体を、作業ツリーの `target\` の下だけに置く。

### Requirement 10: 互換の記録と網羅台帳

**Objective:** As a 開発者, I want 互換の記録が実装と食い違わないようにしたい, so that 次の spec が古い前提で設計しない

#### Acceptance Criteria

1. When 本機能が完成したとき, the 互換の対応表（`doc/COMPAT_ARCHITECTURE.md` §8） shall 正典が黙っている点について本 spec で決めた扱い（「要件の段での裁定」の各行・吸着の向きの読めない値・組の捨て方・連れ動きのきっかけ・寿命・バルーン位置初期化の記憶と対のイベントが無いこと・近道が効く場面）を行として持つ。
2. When 本機能が完成したとき, the 網羅台帳 shall `\![set,alignmenttodesktop,方向]`・`\![set,alignmenttodesktop,free]`・`\![set,alignmentondesktop,bottomまたはtop]`・`\![set,sticky-window,…]`・`\![reset,sticky-window]`・`\![execute,resetballoonpos]` の行を、実装した状態と持ち主に合わせて改める。
3. When 本機能が完成したとき, the 網羅台帳 shall シェルの descript の `seriko.sticky-window` の行と、SHIORI リソース `resetballoonpositionbutton.caption` の行を、実装した状態と持ち主に合わせて改める。
4. When 本機能が完成したとき, the 網羅台帳 shall ゴーストとシェルの descript の `seriko.alignmenttodesktop` 系の行（今は未実装と書かれているが、完了 `areka-P0-window-placement` が既に読む）を、今の実装の状態に合わせて改める。
5. The 網羅台帳 shall プロパティ `currentghost.seriko.sticky-window` の行を本 spec で実装済みにしない（持ち主は `areka-P0-currentghost-property-tree` のまま）。
6. When 本機能が完成したとき, the 本 spec の完了の作業 shall `\![set,zorder]` の台本の組がシェルの切り替えで捨てられる食い違い（正典は「ゴースト終了まで有効」・本 spec の窓の組はシェルの切り替えで保つ）を `/kiro-discovery` で起票する。
