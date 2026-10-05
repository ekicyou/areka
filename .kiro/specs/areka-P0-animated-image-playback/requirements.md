# Requirements Document

> 本文のコードの引用は「何の定義か（関数名・型名＋ファイル）」で指す。現状の記述は**本ブランチでの実測（2026-10-05・main `ec072853` の上）**であり、設計・実装の着手時に引き直すこと。
>
> **未決の議題が 4 件ある**（末尾「要件討議へ持ち込む議題」）。該当する要件には「仮」と書いた。要件討議で裁定を受けてから確定する。

## Introduction

### 誰が困っているか

シェルの作者と、そのシェルを areka で使う利用者。

### いま何が起きているか（本ブランチで実測・2026-10-05）

- 動く APNG・動く WebP をシェルの絵に置くと、全部のコマは読まれてアトラスに載る（完了 `areka-P0-animated-image-decode`）。しかし**コマを時間で切り替える仕組みが無い**ので、画面に出るのは 1 枚目のコマだけである。コマの一覧を引く口（`areka-emo-atlas` の `AtlasTable::animation`）を呼ぶ本番のコードは **0 か所**（呼び手はテストだけ）。
- interval `always` は書いても動かない。seriko の表（`crates/areka-seriko/src/table.rs` の `AnimationTable::from_world`）が駆動するのは `random`・`bind+random`・`sometimes`・`rarely` の 4 語だけで、`always` は元の綴りを添えた `debug!` を残して落とされる。網羅台帳（`doc/ukadoc-coverage/ledger/assets.toml`）の `always` の項は「語彙のみ」。
- pattern定義の描画メソッド `import` は名前を知られていない。`areka-emo-compose` の `method::from_name` が未知の名前として `warn!` を出し、何も描かれない。網羅台帳の `import` の項は「absent」。
- element定義で数字だけを書いてサーフェスを部品として置く入口と、親の切り替えで巻き戻らない子の時計は、完了 `areka-P0-surface-element-nesting` で入った。本 spec はその上に載る。

### 何を変えるか

「動く画像」は 2 本の spec に分かれている。本 spec は 2 本目＝**再生の側**である。

1. 動く絵を element定義・`surface*.png` に置けば、SERIKO 定義を書かなくても動く（自動アニメーション）。
2. interval `always` のアニメーションが、そのサーフェスである間ずっと繰り返す。
3. pattern定義の描画メソッド `import` で取り込んだ動く絵が、ほかのアニメーションと並んで動く。
4. バルーンの面に置いた動く絵も動く（議題 1）。
5. 動く絵も `always` も `import` も使わないシェル（`emo2` を含む）は、見た目も軽さも変わらない。

### 正典の引き直し（ukadoc MCP で 2026-10-05 に取得した逐語）

| 記号 | 出どころ | 逐語 |
| ------ | ------ | ------ |
| C1 | [descript_shell_surfaces `element*`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#element*%2C%E6%8F%8F%E7%94%BB%E3%83%A1%E3%82%BD%E3%83%83%E3%83%89%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D%2CX%E5%BA%A7%E6%A8%99%2CY%E5%BA%A7%E6%A8%99) | 「surface\*.pngまたはelement定義にアニメGIF/APNG/WebPアニメを指定すると、SERIKO定義を書かなくても自動的にアニメーションする(SSP 2.7.38～)。」 |
| C2 | 同上（オプション `--clipping`） | 「左、上、右、下で指定された矩形部分のみを描画する。これを使用するとアニメーション読み込みは無効となる。」 |
| C3 | 同上 | 「サーフェスに対応するsurface\*.pngという画像がある場合、element0が定義されていると元のsurface\*.pngの内容が破棄されて、element0で置き換えられる」 |
| C4 | [descript_shell_surfaces `import`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#import) | 「指定したAPNG/GIFアニメ/WebPアニメを、冒頭待機時間=ウエイトmsec、表示位置=(X,Y)で再生できるようインポートする。インポートしたアニメーションはoverlayメソッドで合成される。このアニメーションは既存のアニメーション定義と並列で動作し共存可能である。なお、フレームと待機時間のみインポートされ、元のファイルで定義されている「繰り返し回数」は無視される。」（2.7.50） |
| C5 | [descript_shell_surfaces `always`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#always) | 「そのサーフェスである間ループ再生。」 |
| C6 | [descript_shell_surfaces `animation*.interval`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation*.interval%2C%E3%82%A4%E3%83%B3%E3%82%BF%E3%83%BC%E3%83%90%E3%83%AB) | 「SSPのみ+区切りで列挙する事で組み合わせ指定が可能。」 |

**brief と照らした結果**:

- brief が引いた C1 と `import` の書式・「合成は overlay」「繰り返し回数は無視」は逐語どおり。
- brief が「要件で正典を確かめる」とした**自動アニメーションでのファイルの繰り返し回数**: 正典は `import` について「無視される」と書くだけで、自動アニメーションについては**黙っている**（議題 2）。
- 読み込みの側が残した**待ち時間 0 のコマの再生のしかた**: 正典は黙っている（議題 3）。
- brief に無かった C6: `always` は `bind+always` のように組み合わせて書ける。brief は `bind` などの他の interval を範囲外としており、組み合わせの扱いは書いていない（議題 4）。
- 正典 C1・C4 は GIF も挙げるが、areka は動く GIF を動かさない（開発者裁定 2026-10-04・理由: 古い形式）。対応するのは APNG と動く WebP の 2 形式である。

## Boundary Context

- **In scope**（シェルの作者・利用者・運用者から見える範囲）:
  - element定義・`surface*.png` に置いた動く絵（APNG・動く WebP）の自動アニメーション。
  - コマの終わりで頭へ戻る繰り返しと、ファイルの繰り返し回数・待ち時間の扱い。
  - interval `always`。
  - pattern定義の描画メソッド `import`。
  - バルーンの面に置いた動く絵を動かすこと（議題 1）。
  - 動かないシェルの見た目・絵を合成し直す回数・1 コマの時間が変わらないこと（合否判定）。
  - 網羅台帳の `element*`・`import`・`always` の項の更新、正典が黙っている箇所の記録、決定論テスト、2 形式の検体での実機の確かめ。
- **Out of scope**:
  - **動く絵の読み込みとアトラス**（完了 `areka-P0-animated-image-decode`）。上限を超えた絵・読めなかった絵は 1 枚の静止画として届き、本 spec はそれを静止画として扱う（動かす絵は 0 枚）。上限の値・変え方も変えない（変更 0）。
  - **数字だけの element定義でサーフェスを置く入口と、子の時計の決まり**（完了 `areka-P0-surface-element-nesting`）。本 spec は使うだけで、決まりを変えない（変更 0）。
  - **動く GIF**。今までどおり 1 枚の絵（変更 0）。
  - **`always` 以外の interval の語**（`runonce`・`never`・`yen-e`・`talk`・`periodic`・`bind` 単独 ほか）。今までどおり駆動しない（変更 0）。引き受け先は `areka-P0-seriko-trigger-intervals`。
  - **`overlay` 以外の描画メソッド全般**（`import` を除く）。今の扱いを変えない（変更 0）。`overlay` 以外の描画メソッドの element定義は今までどおり読み飛ばされ、そこに置いた動く絵も動かない。
  - **element定義のオプション**（`--clipping`・`--alpha`・`--source`・`--scaling`）。`--clipping` を付けた element定義では動く絵として読まない決まり（C2）は `areka-P0-element-clipping-option` が持つ。本 spec はオプションを読まない（要件 10.4）。
  - **台本の再生**（さくらスクリプトの再生と、その土台の時刻つきの台本）。触らない。`\i[ID]` の意味も変えない。
  - **起動の採寸で全コマを読む時間の直し**（`areka-P0-placement-measure-bake-once`）。
- **Adjacent expectations**:
  - **`areka-P0-animated-image-decode`（前提・完了）**: 動く絵ごとに、重ね済みの全部のコマ・コマごとの待ち時間（ミリ秒・0 は 0 のまま）・繰り返し回数（終わりなし／合計 N 回）を渡す。各コマは絵の全体の寸法で揃っている。
  - **`areka-P0-surface-element-nesting`（前提・完了）**: 子の時計は「スコープ × 子のサーフェス番号」ごとに 1 つで、親の切り替えで巻き戻らず、見えない間も進んだものとして扱われ、シェルの切り替え・ゴーストが降りるときに捨てられる。同 spec は「今動かない間隔の語は、後続の spec が一番上のサーフェスで実装した時点で子でも動く」と定めている。本 spec が入れる `always` は、子のサーフェスでも同じ規則で動く。
  - **`areka-P0-element-clipping-option`（後続）**: `--clipping` 付きの element定義を動く絵として読まない判定を持つ。
  - **`areka-P0-seriko-trigger-intervals`（後続）**: 残りの interval の語を引き受ける。本 spec の繰り返しの仕組みの上に載る。
  - **`areka-P0-balloon-element-order`・`areka-P0-self-alpha-declaration`（隣接）**: 同じ合成の場所を触るので、同時には走らせない（steering `roadmap.md` の列の決まり）。
  - **`areka-P0-mcp-dump-images`（隣接・完了）**: 今の見た目を読み戻す。動いている絵では、読み戻した時点のコマが返る。本 spec は読み戻しの道具を変えない。
  - **`areka-P0-currentghost-property-tree`（下流）**: `seriko.*` のプロパティでアニメーションを読む。本 spec はプロパティを足さない（0 個）。
  - **アニメーションの仕組みの数**: 台本の再生と、サーフェスのアニメーションの 2 つのまま。動く絵のための 3 つ目の時計は作らない（開発者確定・brief）。

## Requirements

### Requirement 1: 自動アニメーション

**Objective:** シェルの作者として、動く APNG・動く WebP を element定義や `surface*.png` に置くだけで絵が動いてほしい。そうすれば、コマを 1 枚ずつ切り出して SERIKO 定義を書かずに済む。

#### Acceptance Criteria

1. While 描画メソッド `overlay` の element定義が指す画像が動く絵であり、その element定義を持つサーフェスが表示されている, the areka shall SERIKO 定義が無くても、その絵のコマをファイルの順に切り替えて表示する（C1）。
2. While `surface<数字>.png` が動く絵であり、そのサーフェスが `element0` を持たずに表示されている, the areka shall SERIKO 定義が無くても、その絵のコマをファイルの順に切り替えて表示する（C1）。
3. While `surface<数字>.png` が動く絵であり、そのサーフェスに `element0` が定義されている, the areka shall `surface<数字>.png` を描かず、動かしもしない（C3。今の静止画の扱いと同じ）。
4. The areka shall 動く絵の各コマを、その絵が静止画だったときと同じ位置（element定義の X,Y）・同じ重ね順（element定義の番号の順）で描く。
5. The areka shall 各コマを、ファイルが持つそのコマの待ち時間のあいだ表示してから次のコマへ進める。
6. The areka shall コマの切り替えの時刻を、再生を始めた時刻からの待ち時間の累積で決め、画面の更新の刻みによる遅れを次のコマ以降へ積み上げない。
7. While 動く絵のコマが進んでいる, the areka shall サーフェスの外形（窓の大きさのもとになる範囲）と窓の位置を変えない。
8. While 動く絵のコマが進んでいる, the areka shall サーフェスの当たり判定の領域を、絵が静止画だったときと同じに保つ。
9. When 動く絵を置いたサーフェスが、子のサーフェスとして、または pattern定義が指すサーフェスとして表示される, the areka shall その動く絵を、一番上のサーフェスに置いたときと同じ規則で動かす。
10. While 同じサーフェスに、動く絵と作者が書いたアニメーション（まばたき・着せ替えなど）が在る, the areka shall 両方を互いに止めずに並べて動かし、作者が書いたアニメーションの始まり方・進み方を本 spec の前から変えない。
11. When 絵が動く GIF である、または読み込みの側で 1 枚へ縮められた絵である, the areka shall その絵を静止画として 1 枚だけ表示する（今までどおり・変更 0）。
12. The areka shall 作者が surfaces.txt に書いたサーフェスの番号・アニメーションの番号の意味を、自動アニメーションのために変えない。自動アニメーションが、作者の書いた番号のサーフェス・アニメーションを置き換えたり隠したりすることは 0 件である。

### Requirement 2: 繰り返し回数と待ち時間

**Objective:** シェルの作者として、動く絵の繰り返しと速さが、ファイルに書いたとおりに再生されてほしい。そうすれば、絵を作った道具で見たとおりの動きになる。

#### Acceptance Criteria

1. When 繰り返し回数が「終わりなし」の動く絵が最後のコマの待ち時間を終える, the areka shall 最初のコマへ戻り、表示されている間ずっと繰り返す。
2. （仮・議題 2）When 繰り返し回数が「合計 N 回」の動く絵が N 回目の最後のコマに達する, the areka shall そのコマを表示したまま止め、それ以上コマを進めない。
3. （仮・議題 2）While 繰り返し回数を使い切って止まった動く絵が在る, the areka shall シェルが切り替わる・ゴーストが降りるまで、サーフェスを切り替えても再び動かさない（時計が巻き戻らない決まり＝要件 3 に従う）。
4. （仮・議題 3）When 待ち時間 0 のコマを持つ動く絵を再生する, the areka shall 待ち時間を別の値へ丸めずにそのまま使い、同じ画面の更新の中で通り過ぎたコマは表示せずに飛ばす。
5. If 動く絵の全部のコマの待ち時間の合計が 0 である, then the areka shall その絵を動かさずに 1 枚目のコマを表示し、絵の相対パスと理由を `warn!` で 1 回記録する（止まらない繰り返しで固まらない）。
6. The areka shall 同じ動く絵と同じ経過時間から、何度求めても同じコマを選ぶ（乱数を使わない）。

### Requirement 3: 動きはサーフェスを切り替えても途切れない

**Objective:** シェルの作者として、同じ動く絵をいくつもの表情のサーフェスに置いたとき、表情を替えても動きが続いてほしい。そうすれば、背景の光や揺れる小物が、表情が替わるたびに頭から始まり直さない。

#### Acceptance Criteria

1. When あるスコープで動く絵が初めて表示される, the areka shall その絵の再生を 1 枚目のコマから始める。
2. When 一番上のサーフェスが、同じ動く絵（同じ画像ファイル）を置いている別のサーフェスへ切り替わる, the areka shall その絵を巻き戻さず、切り替えの前の続きから動かす（開発者確定・brief）。
3. When 一番上のサーフェスが、その動く絵を置いていないサーフェスへ切り替わり、後でその絵を置いているサーフェスへ戻る, the areka shall 見えなかった間も止まらずに進んでいたものとして続きから動かし、最初から動かし直さない。
4. While 同じ動く絵が、表示中の絵の中の複数の位置に置かれている, the areka shall それらを同じコマで揃えて動かす。
5. The areka shall 動く絵の再生の進み具合を、スコープごとに別々に持つ（`\0` と `\1` が同じ画像ファイルを置いていても、互いの進み具合に影響しない）。
6. When シェルが切り替わる、またはゴーストが降りる, the areka shall そのスコープの動く絵の再生の進み具合をすべて捨てる。
7. While 動く絵がどこにも表示されていない, the areka shall その絵のコマが進むことを理由に画面を描き直さない。

### Requirement 4: interval `always`

**Objective:** シェルの作者として、`animation*.interval,always` と書いたアニメーションが、そのサーフェスである間ずっと繰り返してほしい。そうすれば、揺れ続ける・光り続けるといった動きを SERIKO 定義で書ける。

#### Acceptance Criteria

1. When interval が `always` のアニメーションを持つサーフェスが表示される, the areka shall 抽選を待たずに、そのアニメーションを最初のコマから始める（C5）。
2. When interval が `always` のアニメーションが最後のコマの待ち時間を終える, the areka shall 最初のコマへ戻って続け、そのサーフェスが表示されている間ずっと繰り返す（C5）。
3. When 一番上のサーフェスが別のサーフェスへ切り替わる, the areka shall 前のサーフェス自身の `always` のアニメーションを止め、新しいサーフェス自身の `always` のアニメーションを最初から始める（一番上のサーフェス自身のアニメーションについての今の決まりと同じ）。
4. While `always` のアニメーションを持つサーフェスが、子のサーフェスとして、または pattern定義が指すサーフェスとして表示されている, the areka shall そのアニメーションを子の時計の決まり（親の切り替えで巻き戻らない）で繰り返す。
5. When `always` のアニメーションの途中に、アニメーションを終わらせるコマ（サーフェス番号 `-1`）が在る, the areka shall そこで絵を消した後、最初のコマへ戻って繰り返しを続ける。
6. If `always` のアニメーションの全部のコマの待ち時間の合計が 0 である, then the areka shall そのアニメーションを繰り返さずに 1 周だけ評価した絵を表示し、サーフェスの番号・アニメーションの番号・理由を `warn!` で 1 回記録する（止まらない繰り返しで固まらない）。
7. The areka shall `always` のアニメーションを、同じサーフェスのほかのアニメーション（`random`・`bind+random`・`sometimes`・`rarely`）と並べて動かし、それらの抽選と進み方を本 spec の前から変えない。
8. The areka shall `always` 以外の、今駆動していない interval の語（`runonce`・`never`・`yen-e`・`talk`・`periodic`・`bind` 単独 ほか）の扱いを変えない（今までどおり元の綴りを添えた記録を残して駆動しない・変更 0）。
9. （仮・議題 4）When interval が `always` を含む組み合わせ（`bind+always` など）で書かれている, the areka shall 今までどおり元の綴りを添えた記録を残して駆動しない（変更 0）。

### Requirement 5: 描画メソッド `import`

**Objective:** シェルの作者として、`animation*.pattern*,import,ファイル名,ウエイトmsec,X,Y` と書いて、動く絵をアニメーションの 1 本として取り込みたい。そうすれば、動く絵を始めるきっかけ（interval）と位置を、ほかのアニメーションと同じ書き方で決められる。

#### Acceptance Criteria

1. When pattern定義の描画メソッドが `import` で、ファイル名が動く絵（APNG・動く WebP）を指す, the areka shall その絵の全部のコマとコマごとの待ち時間を、そのアニメーションのコマとして取り込む（C4）。
2. When `import` で取り込んだアニメーションが始まる, the areka shall pattern定義のウエイト（ミリ秒）だけ待ってから 1 枚目のコマを表示し、以後はファイルのコマごとの待ち時間で進める（C4「冒頭待機時間」）。
3. The areka shall 取り込んだコマを、コマの左上が pattern定義の X,Y に来る位置に、`overlay` と同じ重ね方で描く（C4）。
4. The areka shall 取り込んだアニメーションをいつ始め、繰り返すかどうかを、そのアニメーションの interval だけで決め、ファイルの繰り返し回数を使わない（C4「無視される」）。interval が `always` なら最後のコマの後に頭（冒頭の待ちを含む）へ戻って繰り返し、`random` などの抽選の語なら 1 回再生して次の抽選を待つ。
5. The areka shall `import` で取り込んだアニメーションを、同じサーフェスのほかのアニメーションと並べて動かし、互いに止めない（C4「並列で動作し共存可能」）。
6. If `import` のファイル名が指すファイルが無い、または読めない, then the areka shall その pattern定義を描かず、サーフェスの番号・アニメーションの番号・ファイル名・理由を `warn!` で記録し、同じサーフェスのほかの絵とアニメーションは今までどおり描く。
7. When `import` のファイル名が指す絵が静止画（動く GIF・1 枚へ縮められた絵を含む）である, the areka shall その 1 枚を、ウエイトと X,Y に従う 1 コマのアニメーションとして `overlay` で描き、絵が動く絵として扱われなかったことを `warn!` で 1 回記録する。
8. The areka shall `import` の pattern定義だけが名指しする画像ファイル（element定義にも `surface*.png` にも現れないファイル）も、取り込める。
9. The areka shall `import` のファイルに、読み込みの側の 3 つの上限（コマの枚数・絵 1 つの画素の量・シェルの合計の画素の量）を、element定義の動く絵と同じに効かせる。
10. The areka shall 1 つのアニメーションに `import` の pattern定義とほかの pattern定義が混ざっているときの振る舞いを設計で定め、正典が黙っている箇所の記録（要件 10.2）に記す。どう定めても、記録を残さずに pattern定義を捨てる経路は 0 本とする。

### Requirement 6: バルーンの面の動く絵

> **議題 1（未決）**: brief は「バルーンの面に置いた動く絵も動く」を範囲に入れている（Desired Outcome・Scope の In）。一方、完了 `areka-P0-animated-image-decode` の要件は「バルーンの絵のコマを時間で切り替える仕組みは本 spec でも後続でも作らない」と書いている。本要件は brief に従って仮に置いた。裁定が「動かさない」なら、本要件は「バルーンの面の動く絵は 1 枚目のコマだけを出す（変更 0）」の 1 行へ置き換わる。

**Objective:** バルーンの作者として、バルーンの面の絵に動く APNG・動く WebP を置いたら、それも動いてほしい。そうすれば、シェルとバルーンで絵の作り方を分けずに済む。

#### Acceptance Criteria

1. （仮・議題 1）While バルーンが表示されていて、その面の絵が動く絵である, the areka shall その絵のコマを、シェルの動く絵と同じ決まり（要件 1.5・1.6・要件 2）で切り替えて表示する。
2. （仮・議題 1）While バルーンの面の動く絵のコマが進んでいる, the areka shall バルーンの大きさ・位置・文字の配置・選択肢の当たり判定を変えない。
3. （仮・議題 1）The areka shall バルーンの面の動く絵のコマが進むことを理由に、隠れているバルーンを表示せず、表示中のバルーンを隠さない（バルーンの見える・見えないの決まりを変えない）。
4. （仮・議題 1）While バルーンが隠れている, the areka shall バルーンの面の動く絵のコマが進むことを理由に画面を描き直さない。
5. The areka shall 静止画だけのバルーン（`emo2` のバルーンを含む）の見た目・絵を合成し直す回数を、本 spec の前から変えない。

### Requirement 7: 動かないシェルは変わらない

**Objective:** 利用者として、動く絵も `always` も `import` も使っていない手元のゴーストが、今までと同じに見え、同じ軽さで動いてほしい。

#### Acceptance Criteria

1. While シェルが動く絵・interval `always`・描画メソッド `import` のどれも持たない, the areka shall すべてのサーフェスを本 spec の前と同じ絵・同じ外形・同じ当たり判定で表示する。
2. While シェルが動く絵・interval `always`・描画メソッド `import` のどれも持たない, the areka shall 絵を合成し直す回数を本 spec の前から増やさない。
3. The areka shall `emo2` のシェルで、1 コマの時間（画面の更新 1 回にかかる時間）を本 spec の前から目に見えて落とさない。前後の数字を同じ機械・同じ測り方で採り、記録に残す。
4. While 動く絵・`always`・`import` のアニメーションが表示されている, the areka shall 表示するコマが替わったときにだけ絵を合成し直し、コマが替わらない画面の更新では合成し直さない。
5. The areka shall 動く絵 1 つを表示しているシェルについて、絵を合成し直す回数と 1 コマの時間を測り、記録に残す。画面の更新の刻み（16 ミリ秒）に収まらない場合は、その数字と対処（直す・後続へ起票する）を開発者へ報告する。
6. The areka shall 合成の結果を覚えておく席の数を変える場合に、変える理由と測った数字を記録に残す（変えないなら変更 0 と記す）。
7. The areka shall `emo2` の既存の照合（焼いた結果・合成の結果・まばたきの決定論テスト）を、書き換えずに通す。

### Requirement 8: 失敗は記録して続ける

**Objective:** シェルの作者として、動く絵やアニメーションの書き間違いが、ログで分かる形で安全に扱われてほしい。そうすれば、1 つの間違いでゴーストが止まったり、原因の分からない欠けた絵になったりしない。

#### Acceptance Criteria

1. The areka shall 動く絵の再生・`always`・`import` に、記録を残さない失敗の経路を作らない（**0 本**）。
2. If 動く絵 1 つの再生の準備に失敗する, then the areka shall その絵を 1 枚目のコマの静止画として表示し、絵の相対パスと理由を `warn!` で記録して、ほかの絵とアニメーションの処理を続ける。
3. The areka shall 動く絵の再生・`always`・`import` の失敗を理由に、異常終了もシェルの読み込みの失敗もさせない。
4. The areka shall 同じ原因の記録を、画面の更新のたびに繰り返さない（シェルの読み込み 1 回につき、原因 1 つあたり 1 回）。

### Requirement 9: 検体と決定論テスト

**Objective:** 開発者として、再生の決まりが正しいことを、実際の時間の流れや OS の設定に依らず、いつでも同じ結果で確かめたい。

#### Acceptance Criteria

1. The areka shall 次の振る舞いを、外から与えた時刻で進める決定論的なテストで判定する: コマの切り替えの時刻（要件 1.5・1.6）／終わりなしの繰り返し（要件 2.1）／回数つきの絵の止まり方（要件 2.2・2.3）／待ち時間 0 のコマと合計 0 の絵（要件 2.4・2.5）／サーフェスを切り替えても途切れないこと（要件 3.2・3.3）／複数の位置で揃うこと（要件 3.4）／`always` の開始・繰り返し・切り替え・子での動き・合計 0（要件 4.1〜4.6）／`import` の冒頭の待ち・位置・interval による開始・繰り返し回数を使わないこと・失敗と静止画（要件 5.1〜5.7）。
2. The areka shall 動く絵・`always`・`import` を使わないシェルで、絵を合成し直す回数が増えないこと（要件 7.2）と、コマが替わらない画面の更新で合成し直さないこと（要件 7.4）を、回数を数えるテストで判定する。
3. The areka shall テストを、Windows の拡張機能の有無・実際の時計・ネットワークに結果が左右されない形にする。
4. The areka shall 動く絵を element定義・`surface*.png`・`import`・（議題 1 の裁定しだいで）バルーンの面に置いた試験用のシェルを、リポジトリ内の検体として置く。絵は自作するか、読み込みの側の検体（`crates/areka-emo-atlas/src/testdata/animated/`）を使い、第三者の著作物を検体にしない（**0 件**）。
5. The areka shall 実機で、APNG と動く WebP の 2 形式それぞれについて、自動アニメーション・`always`・`import` が動くこと、サーフェスを切り替えても動く絵が途切れないこと、`emo2` の見た目が変わらないことを確かめ、結果を記録に残す。

### Requirement 10: 記録と申し送り

**Objective:** 後続 spec の担当者とシェルの作者として、areka が動く絵と `always`・`import` をどこまで扱うかを、コードを読まずに知りたい。

#### Acceptance Criteria

1. The areka shall 網羅台帳の `element*`・`import`・`always` の項を、本 spec の着地後の実際の振る舞いに合わせて更新する。動く GIF が非対応であることの注記は残す。
2. The areka shall 正典が黙っている箇所について本 spec が定めたこと（自動アニメーションでの繰り返し回数＝要件 2.2・2.3／待ち時間 0＝要件 2.4・2.5／サーフェスを切り替えても途切れないこと＝要件 3／`always` の途中の終わりのコマと合計 0＝要件 4.5・4.6／`import` が静止画を指すとき＝要件 5.7／`import` の混在＝要件 5.10／バルーンの面＝要件 6）を、対応表（`doc/COMPAT_ARCHITECTURE.md` の §8）に 1 か所へまとめて記す。
3. The areka shall 動く絵をサーフェスの切り替えで巻き戻さないこと（要件 3）を、SSP と見え方が違いうる areka の決まりとして、同じ対応表に記す。
4. The areka shall `--clipping` を付けた element定義の動く絵について、`areka-P0-element-clipping-option` が着地するまで areka がオプションを見分けないこと（C2 を満たさない間の振る舞い）を、網羅台帳の `element*` の項の注記に記す。
5. The areka shall 後続 `areka-P0-seriko-trigger-intervals` の brief へ、本 spec が入れた繰り返しの仕組みと、残した interval の語を追記する。
6. The areka shall 実装の途中で見つけた範囲外の問題を、すべて `/kiro-discovery` で起票して steering `roadmap.md` に載せる。

## 要件討議へ持ち込む議題

答えで作業が変わるものだけを挙げる。番号は要件の本文から引いている。

1. **バルーンの面の動く絵を動かすか**（要件 6）。brief（2026-10-01 起票・開発者確定の範囲）は「動かす」。完了 `areka-P0-animated-image-decode` の要件（2026-10-04 討議の後の文面）は「後続でも作らない」。2 つの文書が食い違っている。正典 C1 が名指しするのはシェルの `surface*.png` と element定義だけで、バルーンには触れていない。仮の案は brief どおり「動かす」。
2. **自動アニメーションでファイルの繰り返し回数を守るか**（要件 2.2・2.3）。正典は `import` について「無視」と書くだけで、自動アニメーションについては黙っている。仮の案は「守る（N 回で最後のコマに止まる。サーフェスを切り替えても動かし直さない）」。ほかの案は ⑴「無視して常に終わりなく繰り返す」（brief の Approach「`always` でコマを順に指す」にいちばん近く、仕組みが 1 つで済む）／⑵「守るが、その絵を置くサーフェスが表示し直されるたびに頭から動かし直す」（要件 3 の「巻き戻さない」と当たる）。
3. **待ち時間 0・極端に短い待ち時間のコマ**（要件 2.4）。正典は黙っている。仮の案は「丸めずにそのまま使い、通り過ぎたコマは飛ばす」。ほかの案は「一定の値より短い待ち時間を、決めた値へ引き上げる」（絵を見る道具の多くがそうしている）。
4. **`bind+always` などの組み合わせ**（要件 4.9）。正典 C6 は組み合わせを認める。brief は `bind` などの他の interval を範囲外としている。仮の案は「本 spec では駆動しない（変更 0）」。ほかの案は「`bind+always` だけは入れる（`bind+random` は今も動いているので、着せ替えが有効な間だけ繰り返す形で足す）」。
