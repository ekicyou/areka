# Requirements Document

## Project Description (Input)

- **だれが困っているか**: シェルを作る人と、そのシェルを使う利用者。シェルの `surfaces.txt` の pattern定義に、別のアニメーションを呼ぶ語や `move` を書くと、そのアニメーションが動かない。開発者のゴースト「．さくら」（検体 `vendors/sample_ghost/dot_sakura.nar`）のシェル `master` で見えた。
- **今の状況**:
  - pattern定義の描画メソッドの欄にある、アニメーションを呼ぶ 7 語（`start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert`）と `move` は、重ね方の合成メソッドの表に無く、読み込みのときに「未知の合成メソッド名: Unknown シームへ吸収」の WARN を出して、何も描かない・何も呼ばない。「．さくら」のシェル `master` では `move` が 22 行（`surface12`・`surface22` の `animation0.pattern1,move,0,10,0,6` など）、`alternativestart` が 5 行（`animation2.pattern0,alternativestart,(0,1)`）、この WARN になった。
  - 行を欄に割る読み手は `[ ]` の中のカンマしか守らないので、`alternativestart,(0,1)` の候補の並びは `(0` と `1)` の 2 つの欄に割れ、番号として読めない。
  - SERIKO の再生には、別のアニメーションを開始・停止・差し込みする働きも、土台の層の表示位置をずらす働きも無い。
- **何を変えるか**: 7 語と `move` を重ね方の合成メソッドから切り離し、pattern の「働き」として SERIKO の再生が受ける。7 語は正典どおりに別のアニメーションを開始・停止・差し込みし、`move` はベースレイヤの表示位置をずらす。「．さくら」のシェル `master` のアニメーションが SSP と同じに動き、WARN は出ない。

出どころは `brief.md`（2026-10-23 起票・2026-10-24 の棚卸㉚の再測定を含む。後の節が前の節を上書きする）。区分は **A**（開発者の条件「次のリリース v0.1.0 までに治ること」）。

## Introduction

正典は ukadoc の `descript_shell_surfaces`（Shell設定 - surfaces.txt）の「描画メソッド」の節である。各語の意味は次のとおり（URL と短い引用。ほかは要約）。

| 語 | 正典の意味（要約） | 出どころ |
| ---- | ---- | ---- |
| `start,ID` | 指定した ID のアニメーションを再生させる。この pattern定義では「サーフェスID、ウェイト、XY座標は無視される」。着せ替え・elementでの使用不可 | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#start%2CID |
| `stop,ID` | 指定した ID のアニメーションを停止させる。ほかは `start` と同じ | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#stop%2CID |
| `alternativestart,(ID1,ID2...)` | 指定した複数の ID の内どれかのアニメーションを再生させる。SSP では括弧が `[]`、区切りが `.` でも読み込まれる。旧定義は `alternativestart,[ID1,ID2...]` | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#alternativestart%2C_ID1%2CID2..._ |
| `alternativestop,(ID1,ID2...)` | 指定した複数の ID の内どれかのアニメーションを停止させる | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#alternativestop%2C_ID1%2CID2..._ |
| `parallelstart,(ID1,ID2...)` | 指定した複数の ID のすべてのアニメーションを再生させる | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#parallelstart%2C_ID1%2CID2..._ |
| `parallelstop,(ID1,ID2...)` | 指定した複数の ID のすべてのアニメーションを停止させる | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#parallelstop%2C_ID1%2CID2..._ |
| `insert,ID` | 指定した ID の着せ替えグループ（interval が `bind` のアニメーション）をその位置に挿入する。着せ替え用のメソッドで、着せ替えでないアニメーション・elementでの使用は未定義。入れ子も可能。表示順を予約するだけで、実際に表示するかは着せ替えの有効/無効による | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#insert%2CID |
| `move` | 「ベースレイヤの表示位置を、元の表示位置から指定座標分ずらす」。瞬間的な変更で連続的な移動ではない。x は右が正、y は下が正。サーフェスIDは無視。着せ替え・elementでは使用不可 | https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#move |

pattern定義そのものの書式（`animation*.pattern*,描画メソッド,サーフェス番号,ウェイト,X座標,Y座標`・ウェイトは「そのコマを描画するまでの待ち時間」）は https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation*.pattern*%2C%E6%8F%8F%E7%94%BB%E3%83%A1%E3%82%BD%E3%83%83%E3%83%89%2C%E3%82%B5%E3%83%BC%E3%83%95%E3%82%A7%E3%82%B9%E7%95%AA%E5%8F%B7%2C%E3%82%A6%E3%82%A7%E3%82%A4%E3%83%88%2CX%E5%BA%A7%E6%A8%99%2CY%E5%BA%A7%E6%A8%99 による。

正典が書いていない点のうち、この spec が areka の読みとして決める点（§8 に残す）:

- `alternativestart` の「どれか」は、並びの候補から等しい確からしさで 1 つを選ぶ。
- 呼び出しが巡回する（自分を呼ぶ・呼び合う）シェルでも固まらない。
- `insert` の行のウェイト・X座標・Y座標は使わず、差し込む着せ替えは自分のコマの座標で描く。
- 正典が「使用不可」「未定義」とする所（着せ替え・element定義に書かれた語、着せ替えでないアニメーションの `insert`）は何もせず、読み込みのときに記録を残す（記録なしに捨てる経路を作らない）。

正典で読めない点は、設計の前に SSP の MCP で測って決める（要件 8）。

## Boundary Context

- **In scope**:
  - SERIKO/2.0 の書式（`animation*.pattern*,…`）の pattern定義に書かれた 7 語と `move` の見分け・欄の読み・再生での働き。
  - 候補の並び `(ID1,ID2,...)`（`[ ]` の括弧・`.` の区切りを含む）を 1 つの欄として読むこと。
  - 7 語と `move` に由来する WARN を無くし、正典が使えないとする所や読めない欄は、読み込みのときに 1 度だけ記録すること。
  - 決定論のテストと、「．さくら」の実機の 1 回。網羅台帳の 8 行と互換の文書の更新。
- **Out of scope**:
  - 旧 SERIKO 1.x の書式（`*pattern*,サーフェスID,ウェイト,描画メソッド,X座標,Y座標` の並び・`*interval`・`version,0`）の読み取り。今の areka は旧書式の行をどの語も読んでいない。brief の棚卸㉚の分割の案に従い、別の spec（仮の名 `seriko-legacy-pattern-format`）が全部の語を対象に持つ。「．さくら」に要るのは新しい書式だけである。
  - 重ね方の合成メソッド（`overlay`・`overlayfast`・`base`・`replace`・`interpolate`・`asis`・`reduce`・`blend-*` など）の描画そのもの（`draw-methods-canon`・`draw-methods-blend`）。
  - 台本のタグ `\i[*]`・`\![anim,…]`（`seriko-script-triggers` とその後続）、interval の `never`・`yen-e` の読み取り、`animation*.name` の名前の読み取り（`seriko-script-triggers`）。
  - `auto`（element定義の `--source` が要る・`element-clipping-option` の後の別件）。
  - interval の抽選そのもの（`seriko-interval-combinations`・`seriko-trigger-lottery-gaps`）。
  - サーフェス番号 `-2`（ほかの全てのアニメーションの停止）と `animation*.option,exclusive`。どちらも今の areka は動かしておらず、この spec では入れない。
  - ukadoc の注記「ユーザー設定でmove動作を切ることが可能」にあたる利用者の設定。
- **Adjacent expectations**:
  - `seriko-script-triggers` が `never` のアニメーションを再生の対象に載せた後は、この spec の呼び出しでそのアニメーションが動く。この spec は interval の種類で呼び出しを断らない。それまでは `never` のアニメーションは「指した番号のアニメーションが無い」と同じ扱いになる。
  - `animation*.name` の名前を 7 語の ID の欄に書いたシェルは、この spec では「番号として読めない欄」として扱う（要件 2.5）。名前で指せるようにするのは名前を読む spec の側である。
  - `base` の描画は `draw-methods-canon` が持つ。「．さくら」の `surface12`・`surface22` は `base` の後に `move` が続くので、`move` の確かめは `base` の描画の出来に依らない形で行う。
  - 実機の確かめは、ゴーストの台詞（`shiori2-protocol` の着地）に頼らず、areka の MCP で面を出して行ってよい。

## Requirements

### Requirement 1: 重ね方と働きの見分け

**Objective:** シェルを作る人として、pattern定義に書いたアニメーションを呼ぶ語と `move` が重ね方として扱われないようにしたい。そうすれば、正しく書いたシェルで警告が出ず、語が働きとして効く。

#### Acceptance Criteria

1. When シェルの読み込みで pattern定義の描画メソッドの欄が `start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert`・`move` のどれかであるとき, the areka shall その語を重ね方の合成メソッドとして扱わず、「未知の合成メソッド名」の記録を出さない。
2. The areka shall 8 語の綴りを、今の合成メソッドの語と同じ比べ方（大文字と小文字・`-`・`_`・前後の空白を問わない）で見分ける。
3. The areka shall 8 語のコマで、サーフェス番号の欄を絵を描く面の番号として引かず、そのコマの位置に絵を重ねない。
4. When 「．さくら」のシェル `master` を読み込んだとき, the areka shall 7 語と `move` に由来する警告の記録を 0 件にする。
5. The areka shall 8 語以外の描画メソッドの語の扱い（重ね方の解決と、未知の語の記録）を今と同じに保つ。

### Requirement 2: 呼び出しの相手の欄の読み

**Objective:** シェルを作る人として、呼び出しの相手の番号や候補の並びを正典の書き方どおりに読んでほしい。そうすれば、書いたとおりのアニメーションが呼ばれる。

#### Acceptance Criteria

1. When `start`・`stop`・`insert` の pattern定義を読むとき, the areka shall 描画メソッドの次の欄を、相手のアニメーションの番号（`animation*` の `*`）として読む。
2. When `alternativestart`・`alternativestop`・`parallelstart`・`parallelstop` の pattern定義を読むとき, the areka shall 描画メソッドの次の欄の候補の並び `(ID1,ID2,...)` を、括弧の中のカンマで欄が割れない 1 つの並びとして読む。
3. Where 候補の並びが `[ ]` で囲まれているか、区切りが `.` であるとき, the areka shall `( )` と `,` で書いたときと同じ並びとして読む。
4. The areka shall `start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop` のコマのウェイト・X座標・Y座標を無視し、7 語のどれでも、相手の欄より後ろの欄が欠けた行（例: `animation2.pattern0,alternativestart,(0,1)`）を 1 つのコマとして受ける。
5. If 相手の番号の欄が空・数として読めない（名前を含む）・候補の並びが空のいずれかであるとき, the areka shall そのコマを何もしないコマとして扱い、読み込みのときにその面・アニメーション・コマを示す記録を 1 度だけ残す。
6. The areka shall 括弧の中のカンマを守る読み方を足しても、7 語の並び以外の定義の行（element定義・collision定義・ほかの描画メソッドの pattern定義など）の読みの結果を今と同じに保つ。
7. The areka shall サーフェス定義ファイルの書式の版の宣言（`descript` の `version`）に関わらず、`animation*.pattern*` の形の行に本要件を当てる（今の読み方と同じ）。

### Requirement 3: 別のアニメーションの開始（start・parallelstart・alternativestart）

**Objective:** 利用者として、シェルが別のアニメーションを呼んで動かす作りのとき、そのアニメーションが動いてほしい。そうすれば、「．さくら」のまばたきのようなアニメーションが SSP と同じに見える。

#### Acceptance Criteria

1. When 再生中のアニメーションが `start` のコマの番になったとき, the areka shall そのコマで待たずに、今の面の、指した番号のアニメーションを pattern0 から再生させる。
2. When 再生中のアニメーションが `parallelstart` のコマの番になったとき, the areka shall 並びのすべてのアニメーションを同じ時刻に再生させる。
3. When 再生中のアニメーションが `alternativestart` のコマの番になったとき, the areka shall 並びの候補から等しい確からしさで 1 つを選んで再生させる。
4. The areka shall 呼び出したアニメーションを、呼び出しのコマで止めずに次のコマへ進める。
5. The areka shall 呼ばれて始まったアニメーションのコマの進み・重なりの順・サーフェス番号 `-1` による停止・終わったあとのコマの残り方を、interval で自動に始まったときと同じ規則で扱う。
6. The areka shall `alternativestart` と `alternativestop` の選びに使う偶然で、interval の抽選（`sometimes`・`rarely`・`random` など）の結果を変えない（同じ乱数の種なら、この spec の前と同じ時刻に同じアニメーションが抽選で始まる）。
7. The areka shall `alternativestart` と `alternativestop` の選びを、決定論のテストから決め打ちできるようにする。

### Requirement 4: 別のアニメーションの停止（stop・parallelstop・alternativestop）

**Objective:** シェルを作る人として、あるアニメーションの途中で別のアニメーションを止められるようにしたい。そうすれば、正典どおりに組んだアニメーションの切り替えが効く。

#### Acceptance Criteria

1. When 再生中のアニメーションが `stop` のコマの番になったとき, the areka shall そのコマで待たずに、指した番号のアニメーションを止め、そのアニメーションがサーフェス番号 `-1` のコマに来たときと同じ見え方（そのアニメーションが描いたコマを消す）にする。
2. When 再生中のアニメーションが `parallelstop` のコマの番になったとき, the areka shall 並びのすべてのアニメーションを、1 と同じ見え方で同じ時刻に止める。
3. When 再生中のアニメーションが `alternativestop` のコマの番になったとき, the areka shall 並びの候補から 1 つを選んで、1 と同じ見え方で止める（選ぶ範囲は要件 8 の M2 で決める）。
4. If 止める相手のアニメーションが再生中でないとき, the areka shall 何もしない。
5. The areka shall 止めた後のアニメーションのその後の扱い（interval による次の開始）を、サーフェス番号 `-1` で止まったときと同じにする。

### Requirement 5: 呼び出しの境目

**Objective:** シェルを作る人として、呼び出しの相手が無いときや呼び出しが巡回するときにも、areka が固まらず、何が起きたかが記録に残ってほしい。そうすれば、シェルの誤りを直せる。

#### Acceptance Criteria

1. If 7 語のコマが指した番号のアニメーションが今の面に無いとき, the areka shall そのコマを何もしないコマとして扱い、読み込みのときにその面・アニメーション・相手の番号を示す記録を 1 度だけ残す。
2. If 呼び出しが自分自身を指すか、複数のアニメーションの呼び出しが巡回するとき, the areka shall 1 つの時刻の中で同じアニメーションを 2 度以上始めず、ほかのアニメーションの進みと台本の進みを止めない。
3. If 着せ替え（interval が `bind`）のアニメーションか element定義に `start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`move` が書かれているとき, the areka shall その語に何もさせず（描かず・呼ばず・ずらさず）、読み込みのときに 1 度だけ記録を残す。
4. The areka shall 本要件と要件 2.5・6.4〜6.6 の記録を、描くたびや刻みのたびに繰り返して出さない（同じシェルの同じ読み込みにつき 1 度）。

### Requirement 6: 着せ替えの差し込み（insert）

**Objective:** シェルを作る人として、着せ替えの前後関係を `insert` で組めるようにしたい。そうすれば、部分的に前・部分的に後ろになる着せ替えが自然な順で重なる。

#### Acceptance Criteria

1. When 着せ替えのアニメーション A のコマに `insert,B`（B は interval が `bind` のアニメーション）があるとき, the areka shall B の絵を A のそのコマの位置の重なりの順で描き、B 自身の本来の位置では重ねて描かない。
2. While B の着せ替えが無効であるとき, the areka shall `insert` の位置に何も描かず、A のほかのコマの重なりの順を保つ。
3. When 差し込んだ B のコマにさらに `insert` があるとき, the areka shall 同じ規則で入れ子に差し込む。
4. If `insert` が巡回するとき（A が B を、B が A を差し込むなど）, the areka shall 固まらずに各着せ替えを 1 度だけ描き、読み込みのときに 1 度だけ記録を残す。
5. If `insert` の相手が今の面に無いか、interval が `bind` でないとき, the areka shall その位置に何も描かず、読み込みのときに 1 度だけ記録を残す。
6. If `insert` が着せ替えでないアニメーションか element定義に書かれているとき（正典は未定義）, the areka shall 何も描かず、読み込みのときに 1 度だけ記録を残す。
7. The areka shall `insert` の行のウェイト・X座標・Y座標を使わず、差し込んだ B のコマを B 自身のコマに書かれた座標で描く（正典は黙る・areka の読み）。
8. The areka shall 着せ替えの何コマ目までを描き、どのコマの `insert` を差し込むかを、要件 8 の M6 の実測どおりにする。

### Requirement 7: ベースレイヤの表示位置をずらす（move）

**Objective:** 利用者として、シェルが `move` で絵をずらす作りのとき、絵がそのとおりにずれてほしい。そうすれば、「．さくら」の面 12・22 の、絵が下へ落ちていく動きが SSP と同じに見える。

#### Acceptance Criteria

1. When 再生中のアニメーションが `move` のコマの番になったとき, the areka shall そのコマのウェイトの後に、ベースレイヤの表示位置を元の表示位置から X座標・Y座標の分（x は右が正・y は下が正・単位はほかのコマの X・Y 座標と同じ）ずらす。
2. When 同じアニメーションの次の `move` のコマの番になったとき, the areka shall 元の表示位置からの新しいずれを当て、前のずれに足さない。
3. The areka shall `move` によるずらしを瞬間的に行い、コマとコマの間を補って動かさない。
4. The areka shall `move` のコマのサーフェス番号の欄を無視する。
5. The areka shall `move` でずらしても、キャラクターの窓の位置と大きさを変えず、窓の外へ出た部分を見せない（例: 「．さくら」の `move,0,10,0,9999` で絵が窓の下へ出て見えなくなる）。
6. The areka shall `move` のずれを元に戻す時（そのアニメーションの停止・終わり・面の切り替え）と、ずれる範囲（ほかのアニメーションの層・当たり判定がついて動くか・`base` で置き換えた土台もずれるか）を、要件 8 の M3・M4・M7 の実測どおりにする。

### Requirement 8: SSP で測って決める点

**Objective:** 開発者として、正典で読めない振る舞いを推測で決めずに SSP と同じにしたい。そうすれば、「SSP と同じに動く」を根拠つきで言える。

#### Acceptance Criteria

1. The areka shall 次の M1〜M7 を、SSP の MCP で測った振る舞いと同じにする。
   - M1: 再生中のアニメーションを `start`・`parallelstart`・`alternativestart` で呼んだとき（頭からやり直すか・そのまま続けるか・呼び出しを捨てるか）。
   - M2: `alternativestop` が選ぶ範囲（並びの全部から選ぶか・再生中のものから選ぶか）。
   - M3: `move` のずれが元に戻る時（そのアニメーションの停止・`runonce` などの終わり・面の切り替え）。
   - M4: `move` でずれる範囲（ベースレイヤだけか、ほかのアニメーションの層や当たり判定もずれるか）。
   - M5: 着せ替え（interval が `bind`）のアニメーションを `start` 系・`stop` 系の相手に指したとき。
   - M6: interval が `bind` だけの着せ替えの pattern1 以降を重ねて描くか、pattern1 以降に書いた `insert` が差し込まれるか（要件 6 の「A のそのコマの位置」が pattern0 の外にもあるか）。
   - M7: 同じアニメーションの `base` のコマの後の `move` が、`base` で置き換えた土台をずらすか（「．さくら」の `surface22` の形）。
2. When 測りを終えたとき, the 開発者 shall 測りの手順・検体・結果を設計の前に spec の調べの記録へ残し、要件 3〜7 の該当する所を測りの結果で確定する。
3. If SSP が記録を出さず黙って捨てる振る舞いであったとき, the areka shall 見え方は SSP に合わせ、記録は要件 5 の決まりで残す（記録は areka の足しとして §8 に書く）。

### Requirement 9: 確かめ

**Objective:** 開発者として、働きが正しいことを毎回の試験で固定し、実機でも 1 回確かめたい。そうすれば、v0.1.0 で「．さくら」が SSP と同じに動くと言える。

#### Acceptance Criteria

1. The areka shall 7 語と `move` の働き（要件 2〜7）を、時刻と選びを固定した決定論のテストで確かめる。
2. The areka shall 候補の並びの読み（`( )`・`[ ]`・`,`・`.` の組み合わせと、欠けた欄）と、要件 5 の境目（相手が無い・巡回・使えない所）を決定論のテストで確かめる。
3. The areka shall 「．さくら」のシェル `master` を読み込んで、7 語と `move` に由来する警告の記録が 0 件であることを決定論のテストで確かめる。
4. When 「．さくら」を実機で 1 回動かしたとき（根・検体・一時フォルダはワークツリーの `target\` の下だけ）, the 開発者 shall 7 語と `move` に由来する WARN が 0 件であること、`surface0` などの `alternativestart` で animation0 か 1 が動くこと、`surface12`・`surface22` で絵が `move` のとおりに下へずれて窓の外へ出ることを、記録と面の写しで確かめる。
5. The areka shall 既存の SERIKO の再生と合成のテストを、この spec の変更の後も通す。

### Requirement 10: 記録と文書

**Objective:** 開発者として、正典のどの行をこの spec が持ち、どこを areka の読みで決めたかを文書に残したい。そうすれば、後の spec が同じ所で迷わない。

#### Acceptance Criteria

1. The 開発者 shall 網羅台帳（`doc/ukadoc-coverage/ledger/assets.toml`）の `start,ID`・`stop,ID`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert,ID`・`move` の 8 行を、実装した状態と担当の spec に更新する。
2. The 開発者 shall 正典が書いていない点の areka の読み（`alternativestart` の等しい確からしさ・巡回の扱い・`insert` の行の座標を使わないこと・使えない所と未定義の所を何もせず記録すること・要件 8 の測りの結果）を、`doc/COMPAT_ARCHITECTURE.md` の §8 に残す。
