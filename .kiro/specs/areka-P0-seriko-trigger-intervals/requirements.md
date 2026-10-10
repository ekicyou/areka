# Requirements Document

## Project Description (Input)

台詞を話している間、キャラクターの口が動かない。多くのゴーストは `surfaces.txt` の `animationN.interval,talk,数値` で口を動かしているが、areka はこの語を読んで記録するだけで再生しない。同じく、面に切り替わった瞬間に 1 回再生する `runonce` と、一定の秒ごとに再生する `periodic,数値` も記録だけで動かない。本 spec は、この 3 つの引き金（`talk,数値`・`runonce`・`periodic,数値`）を seriko の表に採って再生し、口パクを動かす。文字の到着は seriko に既に届いている文字の cue から数え、新しい時計も新しい知らせの口も作らない。`yen-e`・`never`・`\i[ID]` は `areka-P0-seriko-script-triggers`、`bind+always` などの `+` の組み合わせは `areka-P0-seriko-interval-combinations` が別に持つ（2026-10-10 棚卸㉓の分割）。

## Introduction

本書は、SERIKO の interval の 3 語 `talk,数値`・`runonce`・`periodic,数値` を areka で動かすための要件を定める。正典は ukadoc の `descript_shell_surfaces`（`talk,数値`＝「そのサーフェスでバルーン内にテキストが表示されていく時に実行。数値分の文字がくるごとにアニメーションする。」、`runonce`＝「サーフェスに切り替わった瞬間に1回のみ再生。」、`periodic,数値`＝「そのサーフェスである間数値秒間隔で定期的に再生。」）であり、正典が沈黙する細部は areka の裁量として本書に明記し、網羅台帳に記録する。

利用者に見える結果は 3 つ。⑴ 台詞の文字がバルーンに現れるのに合わせてキャラクターの口が動く。⑵ 面に切り替わった瞬間に 1 回だけ動くアニメーションが動く。⑶ その面でいる間、決めた秒ごとに動くアニメーションが動く。

## Boundary Context

- **In scope**:
  - interval の `talk,数値`・`runonce`・`periodic,数値` を読み、数値を落とさずに運び、再生する。
  - 一番上のサーフェスだけでなく、element定義の子の面・pattern定義の先の面（部品）に書かれた 3 語も同じ決まりで再生する。
  - シェルの面とバルーンの面のどちらに書かれていても同じ決まりで扱う（今の `random` と同じ）。
  - 決定論のテスト（偽の時計・偽の文字の到着）、網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の 3 行、口パクを持つ検体の新設と実機の確かめ。
- **Out of scope**（別の spec・今までどおり変えない）:
  - `yen-e`・`never`・台本のタグ `\i[ID番号]`・`animation*.name` → `areka-P0-seriko-script-triggers`。
  - `always` を含む `+` の組み合わせ（`bind+always` ほか）→ `areka-P0-seriko-interval-combinations`。`talk`・`runonce`・`periodic` を含む `+` の組み合わせ（`bind+runonce` など）も本 spec では今までどおり元の綴りを添えた記録だけにする。
  - `\i[ID,wait]`・`start`／`alternativestart`／`stop` などアニメーションから別のアニメーションを呼ぶメソッド・`\![anim,…]` 系のタグ。
  - `always` の単独（`animated-image-playback` で実装済み）・`bind`（実装済み）・`sometimes`／`rarely`（実装済み）・`random`／`bind+random`（実装済み）。これらの振る舞いは変えない。
  - ゴーストの `descript.txt` の `don't need seriko talk`（口パクの無効化）。本 spec では読まない。
  - `\_q`・`\![quicksection]`・`\![set,balloonwait]`（`areka-P0-sakura-time-directives`）と、クリックでの早送り（`areka-P0-talk-fast-forward`）の実装。
  - 台本の進行そのもの（kanade・dola の cue の形）、台本の読み手とコンパイル、合成と描画。知らせを足さず、既に届いている cue を読むだけにする。
- **Adjacent expectations**:
  - 文字の到着の知らせは、台詞の再生から seriko へ既に届いている（文字の cue は 1 続きの文字列と、その再生に占める時間を持つ）。本 spec はこの知らせの形を変えない。
  - 文字がバルーンに現れる時刻は台詞の時計で決まる（今は 1 字あたりの名目の間隔）。口の動きはこの同じ時刻の列に合わせる。将来 `\_q`・`balloonwait`・早送りが文字の現れる時刻を変えたとき、口の動きも「文字が現れる時刻」の決まり（要件 4）にそのまま従う＝本 spec は「文字が一度に現れたら、その瞬間に起きる引き金は 1 回にまとめる」決まりを先に置く。
  - 繰り返しの仕組み（`always`）と部品の時計は `animated-image-playback`・`surface-element-nesting` が入れたものをそのまま使う。
  - `areka-P0-seriko-rebuild-hidden-lottery`（見えていない部品の抽選）は本 spec の後でも先でもよいが、本 spec は「見えていない部品で再生が始まらない」ことを自分の要件（要件 5.6）として保証する。

## Requirements

### 要件 1: 3 語の読み込み（数値を落とさない）

**Objective:** As a シェルの作者, I want `surfaces.txt` に書いた `talk,数値`・`runonce`・`periodic,数値` が数値ごと正しく読まれること, so that 書いたとおりの間隔で口や絵が動く。

#### Acceptance Criteria

1. When `animationN.interval,talk,数値` を読んだとき, the シェルの読み手 shall `talk` の語と数値の両方を保って運ぶ（数値を捨てない）。
2. When `animationN.interval,periodic,数値` を読んだとき, the シェルの読み手 shall `periodic` の語と数値の両方を保って運ぶ。
3. When `animationN.interval,runonce` を読んだとき, the シェルの読み手 shall `runonce` の語を保って運ぶ。
4. The シェルの読み手 shall 3 語を小文字の完全一致で見分け、`Talk`・`RunOnce` のような大文字混じりの綴りや `+` を含む綴りは今までどおり元の綴りのまま「その他の語」として運ぶ。
5. If `talk` または `periodic` の数値が無い・0 である・非負の整数として読めないとき, then the areka shall そのアニメーションを引き金として採らず、面の番号・アニメーションの番号・元の綴りを添えた `warn!` の記録を 1 回残す（読み込みは失敗させず、ほかのアニメーションは影響を受けない）。
6. The areka shall 3 語を足したことで、既に読めている語（`bind`・`random,数値`・`bind+random,数値`・`always`・`sometimes`・`rarely`）の読み方と振る舞いを変えない。

### 要件 2: `runonce`＝面に切り替わった瞬間に 1 回

**Objective:** As a ゴーストの作者, I want `runonce` のアニメーションが面に切り替わった瞬間に 1 回だけ再生されること, so that 表情の切り替えに合わせた一度きりの動き（まばたきの合図など）が作れる。

#### Acceptance Criteria

1. When あるスコープの面が別の番号の面からその面に切り替わったとき（起動後の最初の表示を含む）, the seriko shall その面の `runonce` のアニメーションを、切り替わった刻みに 1 回だけ頭から再生する。
2. While その面を表示し続けている間, the seriko shall `runonce` のアニメーションを 2 回目以降は自動では再生しない。
3. When 同じ番号の面を続けて指定したとき（表示中の面と同じ面への指定）, the seriko shall 切り替わりとみなさず `runonce` を再生しない。
4. When 着せ替えの切り替えだけが起きて面の番号が変わらないとき, the seriko shall `runonce` を再生しない。
5. When 別の面へ移った後に再びその面へ戻ったとき, the seriko shall `runonce` のアニメーションをもう一度 1 回再生する。
6. When スコープの面が非表示（`\s[-1]` など）から表示へ戻ったとき, the seriko shall それを面への切り替わりとして扱い `runonce` を 1 回再生する。

### 要件 3: `periodic,数値`＝その面でいる間、数値秒ごと

**Objective:** As a ゴーストの作者, I want `periodic,数値` のアニメーションがその面でいる間、決めた秒ごとに再生されること, so that 一定の間隔で起きる動き（時計の振り子・呼吸など）が作れる。

#### Acceptance Criteria

1. While あるスコープがその面を表示している間, the seriko shall 面に切り替わった時刻を起点に、数値秒・2×数値秒・3×数値秒…の時刻ごとに `periodic` のアニメーションを 1 回ずつ頭から再生する（切り替わった瞬間には再生しない）。
2. When その面から別の面へ切り替わった、または非表示になったとき, the seriko shall その面の `periodic` の定期の再生を止め、次にその面へ戻ったときは戻った時刻を新しい起点にする。
3. If 定期の時刻が来たときに同じアニメーションがまだ再生中であるとき, then the seriko shall その回の再生は始めず（再生中のコマを乱さず）、次の定期の時刻を待つ。
4. If 画面の更新が遅れて 1 回の刻みの間に定期の時刻を 2 回以上またいだとき, then the seriko shall その刻みで再生を 1 回だけ始め、またいだ回数分を積み上げない。
5. The seriko shall 定期の時刻を数値秒ちょうどで数え、刻みの間隔や更新の遅れで起点をずらさない（遅れた分は過ぎた時間として数える）。

### 要件 4: `talk,数値`＝文字が現れるごと（口パク）

**Objective:** As a 利用者, I want 台詞の文字がバルーンに現れるのに合わせてキャラクターの口が動くこと, so that キャラクターが話しているように見える。

#### Acceptance Criteria

1. While あるスコープの台詞の文字がバルーンに現れていく間, the seriko shall そのスコープの今の面の `talk,数値` のアニメーションを、数値分の文字が現れるごとに 1 回頭から再生する（数値が 3 なら 3 文字目・6 文字目・9 文字目…が現れる時刻）。
2. The seriko shall 文字を書記素クラスタで数える（台詞の再生時間を求める単位と同じ。結合文字や絵文字の列は 1 文字）。
3. The seriko shall 文字の数をスコープごとに数え、同じ面にいる間は台詞の途中の待ち（`\w` など）や文字の切れ目をまたいで積み上げる。
4. When スコープの面が別の番号の面へ切り替わったとき, the seriko shall そのスコープの文字の数を 0 から数え直す。
5. When 文字が現れる時刻が来たとき, the seriko shall 口の動きを文字が現れるのと同じフレームで始める（1 フレーム後に始めない）。
6. If 数値分の文字の区切りが同じ時刻に 2 つ以上来たとき（文字が一度に現れたとき）, then the seriko shall その時刻の再生を 1 回にまとめる（区切りの数だけ積み上げない）。
7. If 区切りの時刻が来たときに同じアニメーションがまだ再生中であるとき, then the seriko shall その区切りでは再生を始め直さず、再生中のコマを続ける。
8. The seriko shall 文字が現れるスコープの面だけを動かし、ほかのスコープの面は動かさない（`\0` の台詞で `\1` の口は動かない）。
9. If 文字が現れるスコープの面が非表示であるとき、またはその面に `talk` のアニメーションが無いとき, then the seriko shall 何も再生せず、文字ごと・刻みごとの記録も増やさない。
10. The seriko shall 改行・消去・選択肢・`\!` のコマンドなど文字でない知らせを文字として数えない。

### 要件 5: 3 つの引き金に共通の再生の決まり

**Objective:** As a ゴーストの作者, I want 3 つの引き金で始まった再生が、今の `random` と同じ決まりでコマを進めること, so that 既存のシェルのアニメーションの書き方がそのまま効く。

#### Acceptance Criteria

1. When 引き金で再生が始まったとき, the seriko shall pattern定義のコマを番号順に、各コマの待ち時間どおりに 1 回だけ流し、最後のコマの後に自動では繰り返さない。
2. When コマが面の番号 `-1` を指すとき, the seriko shall 今の `random` と同じく重ねた絵を消して再生を終える。
3. While 同じアニメーションが再生中である間, the seriko shall 同じアニメーションへの新しい引き金で再生を頭からやり直さない（要件 3.3・4.7 と同じ決まり）。
4. The seriko shall 3 つの引き金を、一番上の面に書かれたアニメーションでも、element定義の子の面・pattern定義の先の面（部品）に書かれたアニメーションでも、同じ決まりで受ける。
5. The seriko shall 3 つの引き金を、シェルの面に書かれていてもバルーンの面に書かれていても同じ決まりで受ける。
6. If 部品が今の外側のコマでは見えていないとき, then the seriko shall その部品の `runonce`・`periodic`・`talk` の再生を始めない（見えるようになった瞬間に途中のコマから始まることが無い）。
7. The seriko shall 3 つの引き金の再生で、乱数を消費しない（`random`・`sometimes`・`rarely`・`bind+random` の抽選の並びを変えない）。
8. While 同じ面に `random` など抽選の引き金と 3 つの引き金が混在している間, the seriko shall 互いに干渉させず、それぞれの決まりで再生する。
9. When 部品の面が見えるようになったとき（外側の面の切り替わりで最初から見えている場合と、外側のコマの変化で後から見えた場合の両方）, the seriko shall その瞬間を部品の「面に切り替わった瞬間」とし、`runonce` の 1 回と `periodic` の起点をそこから数える（部品が見えなくなったら `periodic` を止め、再び見えたときを新しい起点にする）。

### 要件 6: 時刻の決まり

**Objective:** As a 開発者, I want 3 つの引き金の時刻が正確に扱われること, so that 「待ち時間は丸めない・更新が遅れたら過ぎた時間の分だけ進める」という areka の大原則を破らない。

#### Acceptance Criteria

1. The seriko shall 面に切り替わった時刻・文字が現れる時刻を、その出来事が起きた時刻（刻みの境目に丸めない時刻）で受け取り、引き金の起点にする。
2. If 画面の更新が遅れたとき, then the seriko shall 再生中のコマを過ぎた時間の分だけ進める（遅れた分を引き延ばさない）。
3. The seriko shall 3 つの引き金のために新しい時計を持ち込まず、今の刻みと再生の時計の上で動かす（利用者から見て、`random` の再生と 3 つの引き金の再生で時間の進み方が違わない）。

### 要件 7: 記録（ログ無しの失敗の経路を作らない）

**Objective:** As a 開発者, I want 採ったこと・採らなかったこと・採れなかった理由がログから読めること, so that 「書いたのに動かない」を診断できる。

#### Acceptance Criteria

1. When 3 語のアニメーションを表に採ったとき, the areka shall 面の番号・アニメーションの番号・語・数値を添えた控えめな水準（`debug!`）の記録を、表を組むときに 1 回残す。
2. If 3 語のアニメーションを採れなかったとき（数値が無効・コマ列が空）, then the areka shall 理由と面の番号・アニメーションの番号・元の綴りを添えた `warn!` を、表を組むときに 1 回残す。
3. The areka shall `talk` の再生の開始を刻みごと・文字ごとに記録せず、記録を増やすとしても控えめな水準に留める（長い台詞で記録があふれない）。
4. The areka shall 3 語を含まない `+` の組み合わせや、本 spec の範囲外の語（`yen-e`・`never` ほか）について、今までどおり元の綴りを添えた記録を残して採らない（振る舞いを変えない）。
5. The areka shall 引き金を捨てる経路（面が非表示・アニメーション無し・再生中）を失敗として記録しない（正常な経路であり、`warn!` 以上を出さない）。

### 要件 8: 変わらないもの・負荷

**Objective:** As a 利用者, I want 3 語を使わないシェルの見た目と重さが今までと変わらないこと, so that 既存のゴーストに影響が出ない。

#### Acceptance Criteria

1. While 表示中のシェルとバルーンに 3 語のアニメーションが 1 つも無い間, the areka shall 刻みごと・文字ごとの仕事を増やさず、合成の回数も変えない。
2. The areka shall 3 語を足した前後で、`random`・`bind+random`・`sometimes`・`rarely`・`always`・`bind` の決定論のテストの結果と乱数の消費の並びを変えない。
3. The areka shall 同梱の検体（emo2・claudia・konnoyayame・R_POST_and_KOMAINU）の見た目と記録を変えない（いずれも 3 語を書いていない）。

### 要件 9: 決定論のテスト

**Objective:** As a 開発者, I want 3 つの引き金の判断の分岐が偽の時計と偽の文字の到着で固定されること, so that 実機を立ち上げずに回帰を捕まえられる。

#### Acceptance Criteria

1. The areka shall 読み手の 3 語（数値あり・数値なし・0・非数値・大文字混じり・`+` 入り）を決定論のテストで固定する。
2. The areka shall `runonce` の切り替わり（最初の表示・同じ面の再指定・着せ替えだけの変化・戻ってきたとき・非表示からの復帰）を偽の時計で固定する。
3. The areka shall `periodic` の定期の時刻（起点・N 秒ごと・面を離れたときの停止・再生中の回の飛ばし・刻みを 2 周期以上またいだとき）を偽の時計で固定する。
4. The areka shall `talk` の数え方（書記素クラスタ・スコープごと・文字の切れ目をまたぐ積み上げ・面の切り替えでの数え直し・同じ時刻の区切りのまとめ・再生中の区切り・非表示のスコープ・文字でない知らせ）を偽の文字の到着で固定する。
5. The areka shall 部品に書かれた 3 語（見えている部品で始まる・見えていない部品で始まらない）を決定論のテストで固定する。
6. The areka shall 注入した偽の時刻が観測を追い越さない形でテストを書き、実時計に依らない。
7. The areka shall 新しいテストを本番ファイルの中でなく同じディレクトリの兄弟ファイルへ置き、本番ファイル・テストファイルとも 1 ファイル 1,000 行以下を守る。

### 要件 10: 網羅台帳

**Objective:** As a 開発者, I want 網羅台帳の判定が実装と一致していること, so that 正典の対応状況が台帳から読める。

#### Acceptance Criteria

1. When 3 語の実装が着地したとき, the areka shall `doc/ukadoc-coverage/ledger/assets.toml` の `talk,数値`・`runonce`・`periodic,数値` の 3 行を `implemented` にし、担当を本 spec にし、正典が沈黙していて areka の裁量で決めた点（`periodic` が切り替わった瞬間には再生しないこと・`talk` の数える単位と数え直しの時点・同じ時刻の区切りのまとめ）を note に書く。
2. The areka shall 台帳の整合を見張る常時テストを緑のまま保つ。
3. The areka shall `always`・`yen-e`・`never` の行には触れない（担当の付け替えはそれぞれの spec が行う）。

### 要件 11: 口パクの検体と実機の確かめ

**Objective:** As a 開発者, I want 口パクを持つ検体で実機の見た目を確かめられること, so that 檻が隠す欠陥（文字と口のずれなど）を炙り出せる。

#### Acceptance Criteria

1. The areka shall `talk,数値`・`runonce`・`periodic,数値` を書いた面を持つ検体のシェルを新しく用意する（同梱の検体にはいずれも無い）。検体の作り方と置き場は設計で決めるが、実機の根・検体・一時フォルダはワークツリーの `target\` の下だけに作る。
2. When 実機で検体を起動して台詞を再生したとき, the 開発者 shall 文字がバルーンに現れるのに合わせて口が動くこと・面の切り替えで `runonce` が 1 回動くこと・その面でいる間 `periodic` が決めた秒ごとに動くことを、有界の自動終了とログの検索で確かめる。
3. The 実機の確かめ shall 判定の分岐の記録が読める水準まで記録の水準を開けて行う。
4. If 実機で areka の未対応のために動かなかった件が見つかったとき, then the 開発者 shall 範囲外でもすべて起票する。
