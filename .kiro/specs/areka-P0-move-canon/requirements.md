# Requirements Document

## Project Description (Input)
既定のゴースト emo2 の 2026-10-17 版（配布物に同梱）は、メニューの「エモの位置調整」で名前付きの形の `\![move,--X=0,--Y=0,--base=0,--base-offset=left.bottom,--move-offset=right.bottom]` を使い、エモ（`\1`）の窓の右下をむらさき（`\0`）の窓の左下にそろえようとする。ところが areka は名前付きの形を記録付きで飛ばし、座標の計算も参照点（`base-offset`／`move-offset`）を読まないので、利用者がメニューから位置調整を選んでも何も起きない（読めるようにしても、今の計算のままでは違う位置へ動く）。v0.0.3 の利用者がまず踏む不具合である。

この spec は、ukadoc の正典どおりに瞬間の `\![move]` を受けられるようにする。名前付きの形と旧い位置の形を同じ意味に解釈し、参照点 9 点＋`base` を座標の計算に入れ、基準 `screen`・`primaryscreen`・`global`・`me` を解決する。あわせて `\![set,position]`／`\![reset,position]`（窓の固定と解除）と `\![execute,resetwindowpos]`・`OnResetWindowPos`（窓を既定の位置へ戻す）を受ける。時間つきの移動（`--time` の補間・`\![moveasync]`）と窓どうしの連携（sticky-window・alignmenttodesktop）は範囲外。詳細は同じフォルダの `brief.md`。

## Introduction

ゴーストは台本の `\![move]` で自分のキャラクターの窓を動かす。ukadoc はこの指令に 2 つの書き方を定めている。

- **名前付きの形**（SSP 2.3.85 以降の「新仕様」）: `\![move,--X=,--Y=,--time=,--base=,--base-offset=,--move-offset=,--option=]`。どのキーも省略でき、順は自由、キー名の大文字・小文字は区別しない。
- **旧い位置の形**（「旧仕様」・廃止とは書かれていない）: `\![move,X座標,Y座標,移動時間,移動基準,基準とするキャラ基準位置,動かすキャラ基準位置]`。移動時間より後は省略できる。

どちらも「基準（`base`）の上の参照点（`base-offset`）から X・Y だけずらした所に、動かす窓の参照点（`move-offset`）を合わせる」という同じ意味を持つ。参照点は `横.縦` の形で、横は `left`・`right`・`base`・`center`、縦は `top`・`bottom`・`base`・`center` の組み合わせ（`base` は surfaces.txt の `point.basepos`）。基準は `screen`（窓が今いるモニタの左上・省略時）・`primaryscreen`（主モニタの左上）・`global`（システムの座標）・スコープの番号・他のゴーストの名前・`me`（動かす窓自身）から選ぶ。ukadoc は「screen 指定の場合は base、global 指定の場合は right,bottom,center が利用できない」と制限を書く。

今の areka は旧い位置の形だけを読み、基準はスコープの番号だけを解決し、参照点は読まずに常に「両窓の基準位置どうし」で計算する。旧い emo2 の `\1\![move,-353,,,0,base,base]` がたまたま `base.base` だったので表に出なかった。emo2 の 2026-10-17 版が名前付きの形と `left.bottom`／`right.bottom` を使い始めたため、この前提が崩れた。

本 spec は瞬間の移動の正典化に絞る。あわせて、`\![execute,resetwindowpos]`／`OnResetWindowPos`（メニューの「シェル位置初期化」と同じ働きで窓を既定の位置へ戻す）を受ける。`\![set,position]`／`\![reset,position]`（窓の固定と解除）は、SSP 2.9.18 も実装していない（「unknown subcommand」で何もしない・2026-10-17 実測）ので受けない。

ukadoc が黙っている点は、SSP 2.9.18 の実測（`research.md` §8）に合わせる。

### 用語

- **キャラクターの窓**: スコープ（本体側 0・相方側 1 …）ごとに 1 枚ある、シェルの絵を出す窓。バルーンの窓はこれに付いて動く。
- **動かす窓**: `\![move]` を実行したスコープのキャラクターの窓。
- **基準の矩形**: 基準（`base`）が指す矩形。キャラクターの窓、モニタ、または点（`global`）。
- **参照点**: 矩形の上の 1 点。`横.縦` の語で決まる。`base` はキャラクターの窓の基準位置（今の areka では既定の基準位置＝絵の幅の半分・下端）。
- **拡大率**: 窓に今かかっている、画面の DPI とシェルの倍率を合わせた倍率。
- **既定の位置**: 位置の記憶が無いときに起動で窓を並べる位置。
- **位置の記憶**: 次の起動でキャラクターの窓を戻すために areka が覚えておく位置。

## Boundary Context

- **In scope**:
  - `\![move]` の名前付きの形の解釈と、旧い位置の形との同じ意味での解釈（キーの大小・順・省略・`fix`・`--option` の繰り返し）
  - 参照点 9 点＋`base` による位置の計算（両方の形に効く）
  - 基準 `screen`・`primaryscreen`・`global`・`me`・スコープの番号の解決
  - 正典で使えない組み合わせ・他のゴーストの名前の基準・読めない値を、SSP と同じ置き換えで動かし、置き換えたことを記録すること
  - `--option` の語彙（`ignore-sticky-window` は受け取るが、今は動きを変えない）
  - `\![set,position]`／`\![reset,position]` を受けないこと（SSP と同じ）を確かめ、台帳に書くこと
  - `\![execute,resetwindowpos]`、右クリックのメニューの「シェル位置初期化」の項目、`OnResetWindowPos` の送出と返事が無いときの既定の並べ直し
  - 互換の対応表（`doc/COMPAT_ARCHITECTURE.md` §8）と網羅台帳の該当行の改め
  - emo2 の 2026-10-17 版での実機の確かめ
- **Out of scope**（どれも申し送り先がある。申し送りは 2026-10-17 に各 brief へ書いた）:
  - `--time` の補間（途中の位置を通って動くこと）と、移動の時間に合わせて台本の進みを待たせること・`\![moveasync]`・`\![moveasync,cancel]` → roadmap の覚え書き「`sakura-time-directives` から戻した C・D 群」（v0.2.0 の dola の改造の後に起票すると登記済み）
  - 窓の固定と解除の働き → 申し送り先なし（SSP 2.9.18 も実装していない＝受けないことを本 spec の要件 7 で決め、台帳に書く）
  - `\![set,alignmenttodesktop]`・`\![set,sticky-window]`／`\![reset,sticky-window]`、`--option=ignore-sticky-window` の効き目 → `areka-P0-window-link-directives`（v0.1.0）
  - `\![execute,resetballoonpos]`・メニューの「バルーン位置初期化」（`resetballoonpositionbutton.caption`）・位置初期化の 2 つのキーボードの近道（SSP ヘルプの「よくある質問」と「ショートカット一覧」） → `areka-P0-window-link-directives`（v0.1.0）
  - surfaces.txt に宣言された `point.basepos` を面ごとに使うこと → `areka-P0-surfaces-basepos`
  - `check_script` による `\![move]` の引数の診断 → `areka-P0-check-script-arg-checks`
  - 他のゴーストの名前の基準の解決（今はスコープ 0 へ置き換える・要件 3.8） → `areka-P0-multi-ghost-runtime`
  - 3 人目以降の窓の `\![move]` と並べ直し → `areka-P0-extra-character-windows`
- **Adjacent expectations**:
  - 台本の指令で動いた窓は位置の記憶に書かない、という今の決まり（完了 spec `areka-P0-char-position-save-on-exit`・COMPAT §8 の位置の記憶の行）を保つ。本 spec はそれに「シェル位置初期化」の後の扱いだけを足す（要件 8.8）。
  - 台本の X・Y に拡大率を掛ける今の決まり（COMPAT §8・SSP との意図した差）を保つ。
  - `areka-P0-check-script-arg-checks` は `\![move]` の読み方を共有するので、本 spec の後は名前付きの形も読めるものとして扱う。`areka-P0-window-link-directives` は本 spec が受け取る `--option` の語彙に意味を持たせる。

## Requirements

### Requirement 1: `\![move]` の 2 つの書き方の解釈

**Objective:** As a ゴースト作者, I want ukadoc の新旧どちらの書き方で `\![move]` を書いても同じに動いてほしい, so that 正典の記述例そのままの書き方で窓を動かせる

#### Acceptance Criteria

1. When 台本が名前付きの形の `\![move]` を実行したとき, the areka shall `--X`・`--Y`・`--time`・`--base`・`--base-offset`・`--move-offset`・`--option` の各キーを読み、旧い位置の形で同じ値を書いたときと同じ移動を行う。
2. The areka shall 名前付きの形のキー名を大文字・小文字の区別なく読み（`--x=` も `--X=` と同じ）、キーの並び順に関わらず同じ意味に解釈する。
3. The areka shall 名前付きの形のどのキーも省略できるものとして扱い、省略したキーに次の既定を当てる: X・Y＝今の座標を保つ、`time`＝0、`base`＝`screen`、`base-offset`＝`left.top`、`move-offset`＝`left.top`。
4. When 台本が旧い位置の形の `\![move]` を実行したとき, the areka shall 要件 1.3 と同じ既定を省略された欄に当て、名前付きの形と同じ規則で移動する。
5. When X または Y が省略されたか `fix` と書かれたとき, the areka shall 動かす窓のその軸の位置を今のまま保つ。
6. The areka shall X・Y に負の整数を受け付ける。
7. When 参照点に点（`.`）の無い 1 語が書かれたとき, the areka shall その語を横と縦の両方に当てる（たとえば `base` は `base.base`。今の決まりを保つ）。
8. When 名前付きの形で `--option` が繰り返し書かれたとき, the areka shall すべての値を受け取る。
9. When `--option=ignore-sticky-window` が書かれたとき, the areka shall 移動をそのまま行う（窓どうしの連携がまだ無いので、この値で動きは変わらない）。
10. If 名前付きの形に正典に無いキー、`--option` に正典に無い値、または `--キー=値` の形でない引数が書かれた場合, then the areka shall その部分だけを無視して残りで移動を行い、無視した部分を記録する。
11. If 名前付きの形で `--option` 以外の同じキーが 2 回以上書かれた場合, then the areka shall 後に書かれた値を使い、重複したことを記録する。
12. The areka shall 旧い位置の形の欄の並び（X・Y・時間・基準・基準の参照点・動かす窓の参照点）の読み方を、今の areka と同じに保つ。
13. The areka shall 引数のどれかが `--` で始まる `\![move]` を名前付きの形として読み、それ以外を旧い位置の形として読む。

### Requirement 2: 参照点による位置の決め方

**Objective:** As a ゴースト作者, I want 窓の端や中央どうしを合わせて窓を置きたい, so that シェルの大きさや拡大率が変わっても、意図した並びに窓がそろう

#### Acceptance Criteria

1. When `\![move]` を行うとき, the areka shall 動かす窓の参照点（`move-offset`）が、基準の矩形の参照点（`base-offset`）から右へ X・下へ Y だけずらした位置に来るように、動かす窓を置く。
2. The areka shall 横の語 `left`・`right`・`center`・`base` を、それぞれ矩形の左端・右端・横の中央・基準位置の横の位置として扱い、縦の語 `top`・`bottom`・`center`・`base` を、それぞれ上端・下端・縦の中央・基準位置の縦の位置として扱う。
3. The areka shall キャラクターの窓の参照点 `base` を、今の areka の既定の基準位置（絵の幅の半分・下端）として扱う。
4. The areka shall X・Y を作者の寸法として扱い、動かす窓に今かかっている拡大率を掛けた量だけずらす（今の決まりを保つ）。
5. The areka shall 参照点の位置を画面に出ている窓の実際の大きさから求め、拡大率がどの値でも参照点どうしがそろうようにする。
6. The areka shall 旧い emo2 の `\1\![move,-353,,,0,base,base]` で相方の窓が着く位置を、本機能の前と同じに保つ。
7. When 動かす窓が動いたとき, the areka shall そのスコープのバルーンの窓をこれまでどおり窓に付いて動かす。
8. The areka shall 移動先が画面の外にかかっても、本機能によって移動先を画面の内へ寄せない（今の振る舞いを保つ）。
9. The areka shall `\![move]` で動いた窓の位置を位置の記憶に書かない（今の決まりを保つ）。

### Requirement 3: 基準の解決

**Objective:** As a ゴースト作者, I want 画面や自分自身を基準にして窓を置きたい, so that 相方の窓に頼らず画面の決まった場所へ窓を動かせる

#### Acceptance Criteria

1. When 基準が `screen` のとき（基準を省略したときを含む）, the areka shall 動かす窓が今いるモニタの作業領域（タスクバーを除いた矩形）を基準の矩形とする（SSP 2.9.18 の実測と同じ）。
2. The areka shall 動かす窓がどのモニタにいるかを、今の areka が窓の属する作業領域を決めるのと同じ決まり（窓の中心で決め、どのモニタにもいなければ一番近いモニタ）で決める（SSP 2.9.18 の実測と同じ）。
3. When 基準が `primaryscreen` のとき, the areka shall 主モニタの作業領域を基準の矩形とする（SSP 2.9.18 の実測と同じ）。
4. When 基準が `global` のとき, the areka shall システムの座標の原点（主モニタの左上）を基準の点とする。
5. When 基準がスコープの番号のとき, the areka shall そのスコープのキャラクターの窓の矩形を基準の矩形とする。
6. When 基準が `me` のとき, the areka shall 動かす窓自身の、動かす前の矩形を基準の矩形とする。
7. If 基準がスコープの番号で、そのスコープのキャラクターの窓が無い場合, then the areka shall 基準を `screen` に置き換えて動かし、置き換えたことを記録する（SSP 2.9.18 の実測と同じ）。
8. If 基準が正典の語でもスコープの番号でもない語（他のゴーストの名前など）の場合, then the areka shall 基準をスコープ 0 に置き換えて動かし、他のゴーストの基準は今は解決先が無いため置き換えたことを記録する（SSP 2.9.18 の実測と同じ）。

### Requirement 4: 正典で使えない組み合わせと読めない値（SSP と同じ置き換え）

**Objective:** As a ゴースト作者, I want 書き間違えた `\![move]` でも SSP と同じに動き、何を置き換えたかを知りたい, so that SSP で確かめた台本が areka でも同じに動き、原因も追える

#### Acceptance Criteria

1. If 基準が `screen` または `primaryscreen` で、基準の参照点の軸に `base` が書かれた場合, then the areka shall その軸を `center` に置き換えて動かし、置き換えたことを記録する（SSP 2.9.18 の実測と同じ）。
2. If 基準が `global` で、基準の参照点が `left.top` 以外の場合, then the areka shall 基準の参照点を使わず原点そのものを基準の点として動かし、置き換えたことを記録する（SSP 2.9.18 の実測と同じ）。
3. The areka shall 要件 4.1・4.2 の置き換えを基準の参照点にだけ当て、動かす窓の参照点には当てない。
4. If 参照点に正典に無い語が書かれた場合, then the areka shall その軸を `left`（横）または `top`（縦）に置き換えて動かし、受け取った語と置き換えを記録する（SSP 2.9.18 の実測と同じ）。
5. If X または Y の値が整数として読めない場合, then the areka shall その軸を `fix`（今の座標を保つ）として動かし、受け取った値と置き換えを記録する（SSP 2.9.18 の実測と同じ）。
6. If `time` の値が整数として読めない場合, then the areka shall `time` を 0 として動かし、受け取った値と置き換えを記録する。
7. The areka shall 置き換えたどの場合も、トークを止めずに続け、何を受け取り何に置き換えたかを記録に残す（記録を残さないまま黙って置き換えない）。

### Requirement 5: 時間つきの移動の扱い

**Objective:** As a ゴースト作者, I want 時間つきの `\![move]` を書いても窓が目的の位置へ着いてほしい, so that 補間がまだ無くても配置の意図は果たされる

#### Acceptance Criteria

1. When `time` に 0 より大きい値が書かれたとき, the areka shall 窓を途中の位置を経ずに最終の位置へすぐ置き、時間を扱わなかったことを記録する（今の扱いを保つ）。
2. The areka shall `time` の値によって台本の進み（次の字や指令へ進む時刻）を変えない（今の扱いを保つ）。

### Requirement 6: emo2 の「エモの位置調整」

**Objective:** As a emo2 の利用者, I want メニューの「エモの位置調整」でエモの窓をむらさきの窓の横へそろえたい, so that 窓がずれたときにメニューから直せる

#### Acceptance Criteria

1. When 利用者が emo2 の 2026-10-17 版のメニューで「エモの位置調整」を選んだとき, the areka shall エモ（`\1`）の窓の右下の角を、むらさき（`\0`）の窓の左下の角にそろえる。
2. The areka shall 要件 6.1 を、表示の拡大率が 100% のときと 200% のときの両方で成り立たせる。

### Requirement 7: 窓の固定の指令を受けない（SSP と同じ）

**Objective:** As a ゴースト作者, I want `\![set,position]`／`\![reset,position]` を書いても SSP と同じ結果になってほしい, so that SSP で確かめた台本が areka で別の動きをしない

#### Acceptance Criteria

1. When 台本が `\![set,position]` または `\![reset,position]` を実行したとき, the areka shall 窓を動かさず固定もせず、トークを止めずに続ける（SSP 2.9.18 は「unknown subcommand」で何もしない・2026-10-17 実測）。
2. The 網羅台帳 shall `set,position`・`reset,position` の行に、SSP 2.9.18 も実装していないため受けないことを書く。

### Requirement 8: 窓を既定の位置へ戻す

**Objective:** As a 利用者, I want 画面の外へ行った窓や、ずれた窓を一度に元の位置へ戻したい, so that ゴーストを見失わずに使い続けられる

#### Acceptance Criteria

1. The areka shall 右クリックのメニューに「シェル位置初期化」の項目を持つ。
2. When ゴーストが `switchmovetodefaultpositionbutton.caption` に名前を返したとき, the areka shall その名前で項目を出し、返さないときは既定の名前「シェル位置初期化」で出す。
3. When 利用者がその項目を選んだとき, the areka shall ゴーストへ `OnResetWindowPos` を送る。
4. When ゴーストが `OnResetWindowPos` に台本を返したとき, the areka shall その台本を再生し、自分からは並べ直さない。
5. If ゴーストが `OnResetWindowPos` に台本を返さなかった場合（空の返事・返事なし・エラーの返事を含む）, then the areka shall `\![execute,resetwindowpos]` と同じ並べ直しを行う。
6. When 台本が `\![execute,resetwindowpos]` を実行したとき, the areka shall すべてのキャラクターの窓を、位置の記憶が無いときに起動で並べるのと同じ既定の位置へ並べ直す。
7. When 台本が `\![execute,resetwindowpos]` を実行したとき, the areka shall `OnResetWindowPos` を送らずに並べ直す（`OnResetWindowPos` を送るのはメニューから選んだときだけ・SSP 2.9.18 の実測と同じ）。
8. When 並べ直しが行われた後に次の起動をしたとき, the areka shall 並べ直した窓を既定の位置に出す（並べ直しの後に利用者がドラッグした窓は、ドラッグした位置に出す）。
9. When 並べ直しで窓が動いたとき, the areka shall バルーンの窓をこれまでどおりキャラクターの窓に付いて動かす。

### Requirement 9: 記録と確かめ

**Objective:** As a 開発者, I want 窓の移動・並べ直しをログとテストで判定したい, so that 目視だけに頼らず後から確かめ直せる

#### Acceptance Criteria

1. When `\![move]` で窓を動かしたとき, the areka shall どのスコープの窓をどこからどこへ動かしたかと、使った基準・基準の参照点・動かす窓の参照点を記録する。
2. When 窓を並べ直したとき, the areka shall その対象の窓と位置を記録する。
3. The areka shall 二つの書き方の解釈・参照点の計算・基準の解決・使えない組み合わせと読めない値の置き換え・窓の固定の指令を受けないこと・並べ直しの判断の各分岐を、実際の画面を使わない決定論のテストで確かめられるようにする。
4. The areka shall emo2 の 2026-10-17 版を拡大率 100% と 200% で動かし、「エモの位置調整」を選んだ後の窓の位置と記録から要件 6.1 の成立を判定できる実機の証跡を残す。
5. The areka shall 実機の確かめに使う根と検体を、作業ツリーの `target\` の下だけに置く。

### Requirement 10: 互換の記録と網羅台帳

**Objective:** As a 開発者, I want 互換の記録が実装と食い違わないようにしたい, so that 次の spec が古い前提で設計しない

#### Acceptance Criteria

1. When 本機能が完成したとき, the 互換の対応表（`doc/COMPAT_ARCHITECTURE.md` §8） shall `\![move]` の名前付きの形・基準・参照点・時間の行を新しい振る舞いに合わせて改め、「emo2 は位置の形だけを使う」を根拠にした記述を残さない。
2. When 本機能が完成したとき, the 互換の対応表 shall 正典が黙っている点について本 spec で決めた扱いと SSP 2.9.18 の実測に合わせた扱い（`move-offset` の既定・`screen` と `primaryscreen` は作業領域・使えない組み合わせと読めない値の置き換え・無いスコープと他のゴーストの名前の基準の置き換え・X・Y に拡大率を掛ける SSP との意図した差・`\![set,position]`／`\![reset,position]` を受けないこと・`\![execute,resetwindowpos]` は `OnResetWindowPos` を送らないこと・並べ直しの後の位置の記憶・`OnResetWindowPos` の返事が無いときの扱い）を行として持つ。
3. When 本機能が完成したとき, the 網羅台帳 shall `move` の行の持ち主を本 spec へ付け替え、時間の部分（補間と台本の進み）が残りであることを書く。
4. When 本機能が完成したとき, the 網羅台帳 shall `set,position`・`reset,position`・`execute,resetwindowpos`・`OnResetWindowPos`・`switchmovetodefaultpositionbutton.caption` の行を、実装した状態と持ち主に合わせて改める（`set,position`・`reset,position` は要件 7.2 のとおり受けない理由を書く）。
