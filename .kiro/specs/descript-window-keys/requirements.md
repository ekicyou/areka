# Requirements Document

## Project Description (Input)
ゴーストの作者が descript に書いた窓の決まり 2 つを、areka は読まない。

- **最初に出す面**（ゴーストの descript の `sakura.seriko.defaultsurface`・`kero.seriko.defaultsurface`・`char*.seriko.defaultsurface`）: ゴースト「窓際のぱすたさん」は `sakura.seriko.defaultsurface,1000` を書き、着せ替えの面 1000 を既定にしている。areka は本体側を面 0・相方側を面 10 と決め打ちしている。シェルは最初の `\s` まで窓に出さない決まりなので、起動の直後に面 0 の絵が見えることは今は無い。それでも決め打ちの値は 2 か所で効いている。⑴ 起動の位置を決めるときの窓の大きさを、面 0 で測る。ぱすたの面 0 は 320×420 の小さい絵なので、200% の画面で窓が 640×840 と測られ、下端に揃えた位置 (0,864) から実際の面 1000（1108×1368）が画面の下端を越えた。バルーンも起動の関門で上へ寄せられ、ぱすたの × の板にかかった（完了 `areka-P0-pasta-in-windows-verification` の件 4・件 5）。⑵ ゴーストの読み直しで持ち越した面が読み直したシェルに無いときの戻り先になる。
- **バルーンを動かさせない**（ゴーストの descript の `balloon.dontmove,true`、シェルの descript の `sakura.`・`kero.`・`char*.balloon.dontmove`、台本の `\![lock,balloonmove]`・`\![unlock,balloonmove]`）: ぱすたの縦書きのバルーンは、窓の枠の絵にぴったり重ねてある。利用者がバルーンをドラッグできると、絵からずれる。areka はバルーンのドラッグを止めない。

この spec は、descript の既定の面を読んで窓の大きさの測りと戻り先に使うことと、descript と台本の指定の間は利用者のドラッグでバルーンが動かないことを入れる。v0.1.0 のゴール「ぱすたが SSP と同じレベルで動く」の一部。詳細は同じフォルダの `brief.md`。

## Introduction

ukadoc は、この spec が受け持つキーと指令を次のように定めている（2026-10-23 に ukadoc を読み直した）。

- **ゴーストの descript `sakura.seriko.defaultsurface,数値`**: 「本体側のゴーストのデフォルトのサーフェス番号」。既定 0。`kero.seriko.defaultsurface,数値` は相方側で既定 10。`char*.seriko.defaultsurface,数値` は「\p[*]のゴーストのデフォルトのサーフェス番号」で既定 10。ukadoc の「エラーメッセージの解消」は、0 番と 10 番の面を「存在しない番号を\sタグで指定した時などに代わりに表示する「デフォルトサーフェス」」と呼び、10 番を作りたくないときはこれらのキーを書くよう勧める。シェルの descript には同じ名前のキーは無い。
- **ゴーストの descript `balloon.dontmove,true`**: 「バルーン位置調整の標準設定をOFFにする」。既定 false。SSP ヘルプ「ゴーストごとの設定」は、この標準設定にあたる項目「バルーン位置補正有効」を「バルーンをドラッグして位置補正をするかどうかを設定します」と説明する。
- **シェルの descript `sakura.balloon.dontmove,数値`**（2.4.96）: 「本体側の吹き出しのユーザーによる移動操作を禁止する。1(もしくはtrue)で禁止」。既定は「ghostのdescript設定(balloon.dontmove)」。`kero.balloon.dontmove` は相方側、`char*.balloon.dontmove` は 2 人目以降で、同じ書き方。
- **`\![lock,balloonmove]`**（2.4.96）: 「\![unlock,balloonmove]が呼ばれるまで、バルーンのユーザーによるドラッグ移動を抑制する」。descript の `sakura.balloon.dontmove` のように恒久的にも指定できる。
- **`\![unlock,balloonmove]`**（2.4.96）: 「上記を解除する。descript.txtですでに指定されている場合は解除できない」。

今の areka は、起動とシェルの切り替えで各スコープの「最初の面」を本体側 0・それ以外 10 と決め打ちし、窓の大きさの測りも同じ決め打ちで行う。バルーンの窓はいつでも利用者がドラッグでき、ドラッグの後の位置は記憶に書かれる。キャラクターの窓をドラッグしたときや台本の `\![move]` のときは、バルーンがキャラクターの窓に付いて動く。

ukadoc が黙っている点は、本 spec の「要件の段での裁定」に推奨案で答えを置いた。要件ディスカッションで確定する。

### 用語

- **スコープ**: 本体側（0）・相方側（1）・3 人目から（2 以降）のキャラクターの区別。
- **キャラクターの窓**: スコープごとに 1 枚ある、シェルの絵を出す窓。今の areka が窓を作るのはスコープ 0 と 1 だけ。
- **descript の既定の面**: ゴーストの descript の `*.seriko.defaultsurface` から決まる、スコープごとの面の番号。書かれていなければ組み込みの既定の面。
- **組み込みの既定の面**: 本体側 0・相方側と 3 人目から 10（ukadoc の既定の値・今の areka の決め打ちと同じ）。
- **窓の大きさの測り**: 起動の位置を決めるために、areka がキャラクターの窓の大きさを、ある面の絵から求めること。
- **戻り先の面**: ゴーストの読み直しやシェルの切り替えの後に、それまで出していた面が新しいシェルに無いとき、代わりに出す面。
- **バルーンの動かせない指定**: スコープごとに、利用者のドラッグでそのスコープのバルーンを動かさせないかどうか。descript の段（シェルのスコープ個別＞ゴーストの `balloon.dontmove`）と、台本の lock から決まる。
- **台本の lock**: `\![lock,balloonmove]` を受けてから `\![unlock,balloonmove]` を受けるまでの状態。

## 要件の段での裁定

下の表は、要件を書く途中で答えが要った点に推奨案で答えを置いたもの。2026-10-23 の要件ディスカッションで、行 4 のほかは推奨の答えのまま確定した（開発者「承認」）。ukadoc で読めない行 1・4・5 は SSP 2.9 で測って確かめた（`research.md` 10.3 節）。行 2 は測るのに SSP の環境を変える必要があるので測らない。

| # | 問い | 推奨の答え | 根拠 | 該当 |
|---|---|---|---|---|
| 1 | ゴーストの `balloon.dontmove,true` は、利用者のドラッグを禁止する値か | **禁止する値と読む**。シェルの `*.balloon.dontmove` が書かれていないスコープでは、ゴーストの値がそのスコープの動かせない指定になる。キャラクターの窓に付いて動くこと（追従）は止めない | ukadoc はシェルの `*.balloon.dontmove`（移動操作の禁止）の既定を「ghostのdescript設定(balloon.dontmove)」と書く。SSP ヘルプは、ゴーストの値が OFF にする標準設定「バルーン位置補正有効」を「バルーンをドラッグして位置補正をするかどうか」と説明する。SSP でぱすたを 2 回ドラッグした後もバルーンは付いてきた（完了 `areka-P0-pasta-in-windows-verification` の `ssp-measurement.md` 3 節）＝追従は止まらない。2026-10-23 に SSP 2.9 で測った: ぱすたのバルーンだけを (-200,+200) ドラッグしても `currentghost.balloon.scope(0).rect` は変わらなかった。対照として dontmove の無い Emily/Phase4.5 のバルーンを同じやり方でドラッグすると、(-200,-200) 動いた。網羅台帳のこの行と lock・unlock の行の注記は「追従を止める」と読んでいて ukadoc と合わない（要件 8.3 で直す） | 4.1〜4.3・8.3 |
| 2 | descript の既定の面がシェルに無いとき | **組み込みの既定の面へ落とす**（測りも戻り先も）。それも無ければ今の扱いのまま。どちらも記録する | ukadoc は 0 番と 10 番を、存在しない面の代わりに出す特別な面と書く。descript の値はその特別な面を置き換えるもので、無い番号を指したときは元の 0・10 が残るのが自然。空の窓（大きさ 0）で測ると起動の位置が決められない。SSP でまだ測っていない＝確定の前に測る | 2.3・3.3 |
| 3 | `char*.seriko.defaultsurface` を、3 人目からの窓が無い今どう扱うか | **スコープ 2 以降の値も読み、既定 10 で決まる形だけ作る**。窓は作らない（`extra-character-windows` が使う） | 先送りする正典は語彙を全部持ち、追跡する spec を明記する（開発者の方針）。読むだけなら窓の無い今も試験で確かめられる | 1.4・Boundary Context |
| 4 | `\![lock,balloonmove]` は、どのスコープのバルーンに効くか | **ゴーストのすべてのスコープのバルーンに効く** | ukadoc はスコープを書かず「バルーン」とだけ書く。スコープを選ぶ書き方（引数）が無い。SSP でまだ測っていない＝確定の前に測る（`\0` で lock した後に `\1` のバルーンをドラッグする） | 5.1 |
| 5 | 台本の lock の寿命 | `\![unlock,balloonmove]` を受けるまで続く。トークの終わりでは解けない。シェルの切り替え（とシェルの読み直し）の後も続く。ゴーストの終了・切り替え・読み直しで解ける | ukadoc は「\![unlock,balloonmove]が呼ばれるまで」と書き、トークの終わりで解ける `\![lock,repaint]` のような書き方をしない。兄弟の裁定（完了 `areka-P0-window-link-directives` の裁定 1・2）は、台本で決めた窓の決まりをシェルの切り替えでは保ち、ゴーストの読み直しでは捨てる。2026-10-23 に SSP 2.9 で、lock の後の別のトークでもバルーンが動かないことを確かめた | 5.3〜5.5 |
| 6 | シェルの切り替えで、それまで出していた面が新しいシェルに無いとき | **ゴーストの読み直しと同じ戻り先の規則にそろえる**（descript の既定の面 → 組み込みの既定の面 → どちらも無ければ出さずに記録） | 今は切り替えだけ戻り先が無く、窓が次の `\s` まで消えたままになる。読み直しの規則（完了 `mcp-reload` 要件 3.8）と同じ値を使えば、descript の既定の面の使い道が 1 つに揃う | 3.2・3.3 |
| 7 | 動かせない指定が効いている間に、前にドラッグして記憶したバルーンの位置をどうするか | **そのまま使う**（記憶を消さない・無視しない） | ukadoc は利用者の移動操作の禁止だけを書き、記憶した位置には黙る。利用者は「バルーン位置初期化」（完了 `areka-P0-window-link-directives`）で標準の位置へ戻せる | 6.5 |
| 8 | 利用者がバルーンをドラッグしている途中に台本の lock を受けたとき | そのドラッグは最後まで続け、次のドラッグから止める。unlock も同じく次のドラッグから | ドラッグの途中で窓を止めると、マウスと窓が離れて利用者が戸惑う。ukadoc は黙る | 5.6 |
| 9 | 動かせない指定の値が読めないとき（`0`・`1`・`true`・`false` 以外） | その段は書かれていないものとして次の段（ゴーストの値・既定 false）へ進み、記録する | ukadoc は「1(もしくはtrue)で禁止」とだけ書く。読めない値で禁止すると、作者の書き損じで利用者の操作が奪われる | 4.5 |
| 10 | descript の既定の面の値が読めないとき（0 以上の整数でない） | 組み込みの既定の面を使い、記録する | ukadoc は「数値」とだけ書く。負の番号の面は無い（-1 は消す指定） | 1.5 |
| 11 | ukadoc の「存在しない番号を\sタグで指定した時などに代わりに表示する」働き（台本の `\s` が無い面を指したときに既定の面を出す） | **本 spec の範囲外**。今の areka は無い面の `\s` を記録して表示を変えない。この働きは本 spec の完了のときに `/kiro-discovery` で起票する | `brief.md` の範囲は窓の最初の面と測りと戻り先。台本の面の切り替えの失敗の扱いは別の場所（面の表示）を触る。2026-10-23 に SSP 2.9（ぱすた・既定の面 1000）で `\0\s[9998]` を送ると、`currentghost.scope(0).surface.num` は `-1` になり、既定の面 1000 は出なかった（ukadoc の字面と違う）。起票の brief にこの測りを書く | 9.3 |

## Boundary Context

- **In scope**:
  - ゴーストの descript の `sakura.`・`kero.`・`char*.seriko.defaultsurface` の読み取り（スコープ 2 以降は読むだけ）
  - 起動・ゴーストの切り替えでの窓の大きさの測りと、ゴーストの読み直し・シェルの切り替えでの戻り先の面に、descript の既定の面を使うこと
  - ゴーストの descript の `balloon.dontmove`、シェルの descript の `sakura.`・`kero.`・`char*.balloon.dontmove`、台本の `\![lock,balloonmove]`・`\![unlock,balloonmove]` による、利用者のバルーンのドラッグの禁止
  - 記録・決定論のテスト・網羅台帳（`assets.toml`・`sakura-script.toml`）と互換の対応表（`doc/COMPAT_ARCHITECTURE.md` §8）の改め
  - ぱすた（「窓際のぱすたさん」）での確かめ（完了 `areka-P0-pasta-in-windows-verification` の件 4・件 5・`balloon.dontmove` の測りの申し送り）
- **Out of scope**:
  - プロパティ `currentghost.scope(ID).seriko.defaultsurface` の読み書き（`currentghost-property-tree`）
  - 3 人目からの窓（`extra-character-windows`。本 spec はスコープ 2 以降の既定の面を読む形だけ渡す）
  - バルーンの既定の面 `*.balloon.defaultsurface`（`balloon-canon-residue`）
  - 右クリックのメニューの「バルーン位置補正有効」の項目（`enableballoonmovebutton.caption`）と、利用者がドラッグの禁止を設定で切り替える口
  - 台本で動かすバルーンの位置（`\![move]`・`balloonoffset`）と、キャラクターの窓への追従
  - 面の合成とアニメーション。台本の `\s` が無い面を指したときの扱い（裁定 11）
  - 吸着の無い窓が後で大きい面へ替わったときの位置の扱い（`window-work-area-clamp`。本 spec は測る面を descript の既定の面にする所まで）
- **Adjacent expectations**:
  - ゴーストの読み直しで前の面を持ち越す決まり（完了 `mcp-reload` 要件 3.8）はそのまま使う。本 spec は戻り先の面の値を descript から渡すだけ。
  - シェルを最初の `\s` まで窓に出さない決まりはそのまま。本 spec は起動の直後に面を出さない。
  - `extra-character-windows` は、本 spec が決めるスコープ 2 以降の既定の面を、3 人目からの窓の最初の面に使う。

## Requirements

### Requirement 1: descript の既定の面の読み取り
**Objective:** As a ゴーストの作者, I want descript に書いた各スコープの既定の面を areka に読んでほしい, so that 着せ替えの面を既定にしたゴーストの窓が、その面の大きさで置かれる

#### Acceptance Criteria
1. When ゴーストの descript を読む, the areka shall 本体側の descript の既定の面を `sakura.seriko.defaultsurface` から決め、書かれていなければ 0 とする。
2. When ゴーストの descript を読む, the areka shall 相方側の descript の既定の面を `kero.seriko.defaultsurface` から決め、書かれていなければ 10 とする。
3. The areka shall スコープの綴りの別名（`char0.`・`char1.`）を、配置の設定のほかの descript のキーと同じ優先で扱う。
4. When ゴーストの descript を読む, the areka shall スコープ 2 以降の descript の既定の面を `char<スコープ番号>.seriko.defaultsurface` から決め、書かれていなければ 10 とし、その値で窓を作ることはしない。
5. If descript の既定の面の値が 0 以上の整数として読めない, the areka shall そのスコープは組み込みの既定の面を使い、スコープ・キー・書かれていた値を記録に残す。
6. The areka shall descript の既定の面を、ゴーストの descript からだけ読み、シェルの descript の同じ名前のキーを使わない。
7. When シェルを切り替える, the areka shall ゴーストの descript から決めた既定の面を変えずに使い続ける。

### Requirement 2: 窓の大きさの測りに既定の面を使う
**Objective:** As a ゴーストの利用者, I want 起動したゴーストの窓が実際に出る面の大きさで置かれてほしい, so that 窓が画面の下端を越えたり、バルーンが絵にかぶったりしない

#### Acceptance Criteria
1. When 起動の位置を決めるためにキャラクターの窓の大きさを測る（起動・ゴーストの切り替え・ゴーストの読み直し。シェルの切り替えは測り直さず、出した面の大きさで窓が決まる）, the areka shall 各スコープの descript の既定の面の絵で測る。
2. The areka shall 窓の大きさの測りと、戻り先の面（要件 3）とで、同じスコープに同じ descript の既定の面を使う。
3. If descript の既定の面がシェルに無い, the areka shall そのスコープは組み込みの既定の面で測り、スコープ・無かった面の番号・代わりに使った面の番号を記録に残す。
4. If 組み込みの既定の面もシェルに無い, the areka shall そのスコープの測りを今と同じに扱う（本体側は起動できない失敗として記録し、相方側は本体側の大きさで代える）。
5. The areka shall 最初から descript の既定の面の大きさで起動の位置を決め、いったん別の面の大きさで置いてから置き直すことをしない。
6. Where ゴーストが descript の既定の面を書いていない, the areka shall 今と同じ面（本体側 0・相方側 10）で測り、起動の位置を今と変えない。

### Requirement 3: 読み直しとシェルの切り替えでの戻り先の面
**Objective:** As a ゴーストの作者, I want 読み直しやシェルの切り替えで前の面が無くなったとき、descript の既定の面に戻ってほしい, so that 窓が消えたままになったり、決め打ちの面に戻ったりしない

#### Acceptance Criteria
1. When ゴーストを読み直し、それまで出していた面が読み直したシェルに無い, the areka shall そのスコープに descript の既定の面を出す。
2. When シェルを切り替え、それまで出していた面が新しいシェルに無い, the areka shall そのスコープに descript の既定の面を出す。
3. If 戻り先として descript の既定の面も新しいシェルに無い, the areka shall 組み込みの既定の面を出し、それも無ければ面を出さずに、どちらの場合もスコープと面の番号を記録に残す。
4. While それまで出していた面が新しいシェルに在る, the areka shall その面を持ち越し、descript の既定の面へ替えない。
5. The areka shall 戻り先の面を、読み直しや切り替えで窓を組むときに最初から出し、別の面を出してから 1 フレーム後に差し替えることをしない。

### Requirement 4: descript によるバルーンのドラッグの禁止
**Objective:** As a ゴーストの作者, I want descript でバルーンを利用者に動かさせない指定を書きたい, so that 絵に重ねて作ったバルーンが利用者の手で絵からずれない

#### Acceptance Criteria
1. Where ゴーストの descript が `balloon.dontmove` を `true` または `1` にしていて、シェルの descript にそのスコープの `*.balloon.dontmove` が無い, the areka shall そのスコープのバルーンを、利用者のドラッグで動かさない。
2. Where シェルの descript がスコープの `*.balloon.dontmove`（本体側 `sakura.`・相方側 `kero.`・スコープ 2 以降 `char<スコープ番号>.`）を書いている, the areka shall ゴーストの `balloon.dontmove` よりその値を優先し、`1` または `true` なら動かさず、`0` または `false` なら動かせるようにする。スコープの綴りの別名（`char0.`・`char1.`）は要件 1.3 と同じ優先で扱う。
3. Where ゴーストの descript とシェルの descript のどちらにも指定が無い, the areka shall 今と同じく、利用者のドラッグでバルーンを動かせるようにする。
4. When シェルを切り替える, the areka shall descript の段の指定を、ゴーストの descript と新しいシェルの descript から決め直す。
5. If `balloon.dontmove` または `*.balloon.dontmove` の値が `0`・`1`・`true`・`false` のどれでもない, the areka shall その段は書かれていないものとして次の段で決め、スコープ・キー・書かれていた値を記録に残す。

### Requirement 5: 台本の lock と unlock
**Objective:** As a ゴーストの作者, I want 台本でバルーンのドラッグを一時的に止めたり戻したりしたい, so that 演出の間だけバルーンを動かさせないようにできる

#### Acceptance Criteria
1. When 台本の `\![lock,balloonmove]` を受ける, the areka shall そのゴーストのすべてのスコープのバルーンを、利用者のドラッグで動かさない。
2. When 台本の `\![unlock,balloonmove]` を受ける, the areka shall 台本の lock を解き、descript の段で動かせるスコープのバルーンを、利用者のドラッグで動かせるようにする。
3. While descript の段がスコープのバルーンを動かさない指定にしている, when 台本の `\![unlock,balloonmove]` を受ける, the areka shall そのスコープのバルーンを動かさないままにする。
4. While 台本の lock が続いている, the areka shall トークが終わっても、シェルを切り替えても、lock を解かない。
5. When ゴーストが終わる・切り替わる・読み直される, the areka shall 台本の lock を解く。SHIORI だけの読み直し（`\![reload,shiori]`）では解かない（窓の組の先例＝完了 `areka-P0-window-link-directives` 要件 6.3 と同じ）。
6. If 利用者がバルーンをドラッグしている途中に lock または unlock を受ける, the areka shall そのドラッグは最後まで今の扱いで続け、次のドラッグから新しい指定を使う。
7. When MCP の `check_script` が `\![lock,balloonmove]` または `\![unlock,balloonmove]` を含む台本を調べる, the areka shall それを知らない指令として報告しない。
8. The areka shall 台本の lock を受けたことと解いたことを記録に残し、descript の段のために unlock で解けなかったスコープも記録に残す。

### Requirement 6: 動かせない間の振る舞い
**Objective:** As a ゴーストの利用者, I want バルーンが動かせない間も、バルーンのほかの操作とゴーストの動きは今どおりであってほしい, so that 禁止がドラッグ以外の操作を奪わない

#### Acceptance Criteria
1. While スコープのバルーンが動かせない指定になっている, when 利用者がそのバルーンをドラッグする, the areka shall バルーンの位置を変えず、バルーンの位置の記憶も書き換えない。
2. While スコープのバルーンが動かせない指定になっている, the areka shall バルーンのクリック・選択肢・リンク・スクロールを今どおり受ける。
3. While スコープのバルーンが動かせない指定になっている, when 利用者がキャラクターの窓をドラッグする, the areka shall バルーンを今どおりキャラクターの窓に付いて動かす。
4. While スコープのバルーンが動かせない指定になっている, the areka shall 台本の `\![move]`・窓の追従・「バルーン位置初期化」によるバルーンの移動を止めない。
5. While スコープのバルーンが動かせない指定になっている, the areka shall 前に記憶したバルーンの位置を消さず、今どおり使う。
6. When 動かせないバルーンへの利用者のドラッグを止める, the areka shall 1 回のドラッグにつき 1 件、スコープと止めた理由（descript の段か台本の lock か）を記録に残す。

### Requirement 7: 決定論のテスト
**Objective:** As a areka の開発者, I want この spec の判断の分かれ目を実機なしで確かめたい, so that 後の変更で既定の面とドラッグの禁止が黙って壊れない

#### Acceptance Criteria
1. The areka shall descript の既定の面の決まり方（書かれている・書かれていない・別名・読めない値・スコープ 2 以降）を、実機なしの決定論のテストで確かめられるようにする。
2. The areka shall 窓の大きさの測りと戻り先の面が、descript の既定の面・組み込みの既定の面・どちらも無いの 3 通りで要件 2・3 のとおりになることを、実機なしの決定論のテストで確かめられるようにする。
3. The areka shall バルーンの動かせない指定の決まり方（シェルのスコープ個別・ゴーストの値・読めない値・台本の lock・unlock で解けない場合）を、実機なしの決定論のテストで確かめられるようにする。
4. The areka shall 動かせないバルーンのドラッグでバルーンの位置と記憶が変わらず、キャラクターの窓のドラッグではバルーンが付いて動くことを、決定論のテストで確かめられるようにする。

### Requirement 8: 網羅台帳と互換の対応表
**Objective:** As a areka の開発者, I want 網羅台帳と対応表がこの spec の着地の後の実物と合ってほしい, so that 何が効いて何が残っているかを台帳から読める

#### Acceptance Criteria
1. When この spec が着地する, the areka の網羅台帳 shall `assets.toml` の `sakura.`・`kero.`・`char*.seriko.defaultsurface`（ゴースト）・`balloon.dontmove`（ゴースト）・`sakura.`・`kero.`・`char*.balloon.dontmove`（シェル）の行の状態と持ち主を、実物に合わせて改める（`char*.seriko.defaultsurface` は窓が無く読むだけと分かる状態にする）。
2. When この spec が着地する, the areka の網羅台帳 shall `sakura-script.toml` の `\![lock,balloonmove]`・`\![unlock,balloonmove]` の行の状態と持ち主を、実物に合わせて改める。
3. When この spec が着地する, the areka の網羅台帳 shall ゴーストの `balloon.dontmove` と lock・unlock の行の注記を、ukadoc の「利用者の移動操作の禁止」に合わせて直す。
4. When この spec が着地する, the 互換の対応表（`doc/COMPAT_ARCHITECTURE.md` §8） shall 本 spec の「要件の段での裁定」のうち ukadoc が黙る点の areka の扱いを、根拠と一緒に載せる。

### Requirement 9: ぱすたでの確かめと申し送り
**Objective:** As a 開発者, I want 着地の後にぱすたで崩れが直ったことを確かめたい, so that 完了 `areka-P0-pasta-in-windows-verification` の申し送りが漏れなく片づく

#### Acceptance Criteria
1. When この spec の着地の後に、ぱすたの手順書 `doc/pasta-real-run.md` の段 1 を回す, the areka shall ぱすたの窓の大きさを面 1000 の大きさ（200% の画面で 1108×1368）で測り、ぱすたの窓の下端を作業領域の中に収める（件 4）。
2. When 同じ段 1 を回す, the areka shall 起動の関門でぱすたのバルーンを上へ寄せず、バルーンがぱすたの × の板にかからない（件 5・縦書きのバルーンを根の `balloon\` に写した置き方で見る）。
3. When この spec が完了する, the 完了の手順 shall 裁定 11 の働き（台本の `\s` が無い面を指したときに既定の面を出す）を `/kiro-discovery` で起票し、起票したことを完了の記録に残す。
4. When ぱすたの段 1 を回した後に、利用者の操作でぱすたのバルーンだけをドラッグする, the areka shall バルーンを動かさない（ぱすたの descript は `balloon.dontmove,true`）。
