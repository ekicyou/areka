# 段階と優先順の統合ブリーフィング

この文書は「どの束をどの段階に置き、段階の中でどの順に並べるか」の正本である。束がどの項目を
持つかは `linkage.md` が持ち、この文書はそれを前提に順位だけを決める。台帳の `priority` は
両者から機械で導いた写しである。

数はこの文書の囲みに 1 度だけ置き、常時の検査が数え直して突き合わせる。本文で同じ数に触れる
ときは、値を写さず欄の名前で指す。項目を指すときは必ず引用符か逆引用符で囲み、地の文に裸で
書かない。

まだ 1 行も無い表（順位の行・主障壁・書き戻し後の分布・完了済み spec 宛ての宛先・テンプレート
辞書）は、囲みに見出しだけを置くことができないので書いていない。読み手は「その表が無い」を
「0 行」として扱う。

## 1. 着手条件の確認

**確認日: 2026-09-12**（この節は着手の当日に書き、以後は書き換えない）

上流の成果物 3 つがそろっていることを、下の手順でその場で確かめた。数はいずれもこの日に
数え直した値であり、既存の報告や上流の文書から写した値ではない。

### 1-1. 4 台帳の未分類が 0 件

台帳 4 本のそれぞれについて、次の 3 つを数えた（作業ディレクトリは `doc/ukadoc-coverage/ledger`）。

- 項目の数: `grep -c '^\[entry\.' <台帳>`
- 状態の行の数: `grep -c '^status = ' <台帳>`
- 未分類の行の数: `grep -c '^status = "unclassified"' <台帳>`

| 台帳 | 項目 | 状態の行 | 未分類 |
| --- | ---: | ---: | ---: |
| `assets.toml` | 542 | 542 | **0** |
| `property.toml` | 188 | 188 | **0** |
| `sakura-script.toml` | 342 | 342 | **0** |
| `shiori.toml` | 677 | 677 | **0** |
| 合計 | 1749 | 1749 | **0** |

未分類は 4 本とも 0 件である。4 本の合計も 0 件である。

項目の数と状態の行の数を並べて数えたのには理由がある。備考は三重引用符で囲んだ自由文なので、
備考の中の行が偶然 `status = ` で始まっていると、状態の行として数えられてしまう。両者が 4 本
とも同じ値になったので、そういう行は 1 本も無い。したがって未分類 0 件は項目の数え落としでは
ない。

数え方の注意: 未分類を数える行は 1 件も当たらないので、`grep -c` はそれ自体は「見つからな
かった」を表す終了コードを返す。出力の 0 が答えであり、終了コードを失敗と読まない。

### 1-2. 全体報告の実在

`doc/ukadoc-coverage/report/summary.md` が実在することを `ls -l` で確かめた。大きさと行数は
`ls -l` からは読めないので別に数えた——`wc -c` で 34,477 バイト、`wc -l` で 145 行、`grep -c ''`
でも 145 行。改行の数え方が異なる 2 つの道具が同じ値を返したので、末尾の改行の有無で 1 ずれて
いない。この 145 は確認日に上の 3 つの道具でその場で数え直した値であり、上流の要件文書に載る
同じ値を写したものではない（両者が一致したことは、着手の時点で報告がまだ作り直されていない
という事実の裏取りになる）。ドメイン別の報告 4 本（`report/assets.md`・`report/property.md`・
`report/sakura-script.md`・`report/shiori.md`）も同じディレクトリに実在する。

上流の brief が名指ししていた綴りは実在しない。本 spec の 3 文書ではその綴りを一切使わず、
上のパスだけを使う。

### 1-3. M1 完成の宣言と開発者の署名

- `.kiro/specs/completed/areka-P0-emo2-conformance-e2e` が実在することを `ls -d` で確かめた。
  すなわち完了済みの側にある。
- 同ディレクトリの `verification/m1-completion.md` が実在することを `ls` で確かめた。
- 同ファイルの §4「サインオフ（人間の記入欄）」に開発者の署名が入っていることを確かめた。
  署名は `ekicyou 2026-09-11` で、20 項目すべて合格・実機層の総合判定は合格と書かれている。
  同 §6 に持ち越しが 8 行あり、うち 7 件は持ち越し、1 件は 2026-09-11 の開発者の裁定で受容と
  なっている。第二段（段 4）はこの 20 項目と 8 行を材料に使う。

### 1-4. 欠けている物

上の 3 つはいずれもそろっており、欠けている物は無い（0 件）。したがって推量で代わりの値を
書いた箇所も無い（0 か所）。

### 1-5. テンプレート辞書の配布元（2026-09-12 に行った「実装時の確認」の記録）

順位の根拠の 3 つ目「資産の広さ」は、里々とヤヤの標準テンプレート辞書に現れる語彙を物差しに
する。この節は、その辞書の配布元をどう調達するかの記録である。

**もとの裁定は 2026-09-11 の議題 2 である。** 中身は「テンプレート辞書の実物を静的に読む。
配布元のアドレスは実装時に開発者へ確認してから取得する。配布物はリポジトリに入れない。
実走しない」で、出典は本 spec の要件 6.3 と設計 D-8 の 2 か所である。つまり 2026-09-11 に
決まったのは「実装のときに確認する」ところまでで、確認そのものは実装へ持ち越されていた。

**その持ち越された確認を、2026-09-12 に本 spec の実装の最初のタスク（1.1）で行った。**
作業の進行を受け持つ側が選択肢を示し、開発者はその中から「実装側が公式の配布元を探して
取得してよい」を選んだ。**この回答の一次資料はリポジトリの中に無い**——やり取りは会話の
中だけで行われた——**ので、本節がその記録である。** 後から読む人が「どこにも書かれていない
裁定」と読み違えないよう、出所をここに残す。

**この時点で記録した配布元のアドレスは 0 個である。** 0 になる理由は、開発者が「このアドレス
を使え」ではなく「実装側が探せ」を選んだからで、確認すべきアドレスがまだ存在しないためで
ある。推量で 1 つも書かない。アドレス・取得日・読んだ辞書ファイルの名前の 3 つは、取得を
実際に行う段 3（タスク 4.1）で、この文書のテンプレートの表の `url`・`fetched_on`・`files` に
書く。

タスク 4.1 は「1.1 で確認した配布元から取得する」と書かれている。**その「確認した配布元」の
中身は、本節の回答そのもの、すなわち「実装側が公式の配布元を探して取得してよい」である。**
4.1 が誰の承認も無い出所から取ってくるという意味ではない。

取得できなかった場合は要件 6.8 の退路へ落とす。ukadoc の里々・ヤヤの wiki が名指しする語彙
だけを参照値にし、テンプレートの表に `fallback = true` と取得できなかった配布元を書き、
順序の主張から外す束には `insufficient = true` を付ける。開発者の追加の指示は待たない。

この記録により、後続の段は答え待ちで止まらない。

## 2. 段階の定義と写像の規則

この節は 4 つを置く——⑴ 段階 A〜E の定義（5 つの節目）、⑵ 束の初期配置、⑶ 束から段階への
写像の規則 3 つ、⑷ 初期配置と台帳の根拠が食い違う束の裁定候補。段階ごとの束数と項目数は
この節に置かない。それは次の節の `[stage.A]`〜`[stage.E]` の持ち物である。

### 2-1. 5 つの節目（段階の定義）

段階は機構の名前ではなく、利用者が体験できる節目で名付ける。

| 段階 | 節目 | その節目に届いたと言えること |
| --- | --- | --- |
| A | そこにいて、触れて、話す | 入れたゴーストが画面に出て立ち、撫でれば反応し、台詞をバルーンで読める |
| B | 迎えて、育てて、見送る | 配布物を受け取って入れられ、新しい版に入れ替えられ、着るものを換えられ、要らなくなったら見送れる |
| C | 察してくれる | 機械と画面の様子（休止からの復帰・電池・画面の切り替わり・音）に気づいて、そのことに触れられる |
| D | 仲間がいる | 他のゴーストや外のプログラムと、名指しで呼び合い、掛け合いができる |
| E | 周辺 | 外のプログラムや網の向こうを呼び、作り手の道具を動かし、ベースウェアの作り付けの画面を開ける |

### 2-2. 束の初期配置

初期配置は要件 5.2 が定めた開発者裁定（2026-09-02 議題 7）である。本タスクはこれを初期値と
して受け取り、黙って変えない。下の表は要件 5.2 の綴りをそのまま写したものである。

| 段階 | 初期配置が名指しした束 |
| --- | --- |
| A | 起動と挨拶・会話・撫で・メニュー・終了・自分から喋る（ランダムトーク・時報・分）・名前を尋ねて覚える |
| B | nar インストール（D&D 含む）・ネットワーク更新・シェル／バルーン切替・オーナードローメニュー・消滅 |
| C | スリープ復帰・バッテリー・スクリーンセーバー・フルスクリーン退避・最小化・ディスプレイ変化・サウンド |
| D | 多重ゴースト・コミュニケート・呼び出し・SSTP・FMO・PLUGIN・`x-ukagaka-link` |
| E | 外部アプリ Ex・開発者機能・トランスレータ・ヘッドライン |

上の表が名指しした束の名前は **30** である（数え方: 表の 5 行の中黒で区切られた名前を数えた）。

#### 段階の中の置き場所を釘付けする 2 つの方針（要件 5.3）

- **「更新」のテーマを持つ束を段階 B の先頭に置く。** そのテーマを持つ束は「更新」と
  「インストール」の **2** つで、どちらも段階 B にある（数え方: `linkage.md` の 67 の囲みの
  `themes` を全部読み、「更新」を含むものを数えた）。
- **照会系を段階 C の末尾に置く。** 要件 5.3 が名指しするのは `system.*` の照会で、それを
  構成 id に持つ束は「環境の照会」**1** つである（数え方: 同じ 67 の囲みの `members` を読み、
  ページが `list_propertysystem` で綴りが `system.` から始まるプロパティの id を持つ束を
  数えた。`ukadoc:list_propertysystem:system.cpu._28_30ad_30fc_29:1` がその族の 1 つである）。同じ `foundation` を
  持つ「一覧と汎用プロパティの照会」も問い合わせに答えるだけの束なので、並べて段階 C の
  末尾に置く。
- どちらも段階の中の**並び**の話なので、この節では段階だけを決め、順位そのものは次の節の
  順位の行が表す。

### 2-3. 束から段階への写像の規則（要件 5.4）

規則は次の 3 つに固定する。以後この 3 つ以外の理由で段階を動かさない。

1. **テーマ 1 つ** ＝ 同じ段階の中で先頭群へ。
2. **テーマ 2 つ以上** ＝ 段階を 1 つ繰り上げてよい。
3. **テーマ 0 かつ壊れ方が見た目の差以下** ＝ 段階 E の候補。

規則は「基準の配置に対して何をしてよいか」を言うもので、基準そのものは作らない。基準は、
初期配置がある束は初期配置、初期配置に名前が無い束は 2-1 の節目である。

#### 規則 ⑶ を全 67 束に当てた結果

**掛かる束は 0 件である。** 数え方: `linkage.md` の名前付き束 63 と単独項目 4 の囲みから
`breakage` を読み、値が「見た目の差」か「該当なし」のものを数えた——**0 件**で、67 の囲みは
すべて「黙って壊れる」だった。したがって規則 ⑶ の 2 つ目の条件（壊れ方が見た目の差以下）を
満たす束が 1 つも無く、テーマの数を見るまでもなく規則 ⑶ は誰にも当たらない。

テーマが 0 の束と単独項目は **15**（内訳: 名前付き束 **12**・単独項目 **3**。数え方: 同じ 67 の
囲みの `themes` が空配列のものを数えた）あるが、上の理由でどれも段階 E の候補にならない。
**テーマ 0 だけでは段階を下げる根拠にならない**——これは 2-5 の裁定候補 2・3 で扱う。

#### 規則 ⑵ の行使

本タスクは規則 ⑵ の繰り上げを **0 件** 行使した。理由は 2 つある。

- 初期配置のある 30 束については、要件 5.5 が「初期配置を黙って変えない」と定める。加えて
  初期配置を作った側が既に繰り上げを済ませている——要件 5.2 の出どころである brief の段階表の
  「旧定義からの移動」の欄が、時報と分を旧 C から A へ、入力窓を旧 C から A へ、消滅を旧 D
  から B へ、ファイルの投げ込みを旧 C から B へ、ヘッドラインを旧 D から E へ動かしたと
  書いている。同じ規則を二度当てると同じ束が二段動く。
- 初期配置に名前が無い 37 束については、本タスクが節目を当てて基準そのものを置く。繰り上げる
  前の基準が無いので、繰り上げの判断は基準の判定の中で済んでいる。

テーマ 2 つ以上の束は、初期配置に名前が無い 33 束のうち **8** である（数え方: 同じ 67 の囲みの
`themes` の要素数を数えた）。段階の中でどこに並ぶかは、この 8 つを含めて次の節の順位が決める。

規則 ⑴ は段階の中の並びの規則なので、この節では段階を動かさない。

### 2-4. 67 の束と単独項目に段階を当てる

要件 5.2 の初期配置が名指しした名前は 30 で、`linkage.md` が名付けた束は 63、単独項目は 4 で
ある。差の 37 は「初期配置と食い違う束」ではなく「初期配置に名前が無い束」である。名前が
無いままにすると要件 5.6（すべての束がちょうど 1 つの段階を持つ）を満たせないので、本タスクで
段階を当てる。

当て方は 2-1 の節目である。束の「欠けると壊れる既存ゴーストの振る舞い」が 5 つの節目のどれを
損なうかを読み、その段階に置いた。そのうえで 2-3 の規則を当てたが、⑵ も ⑶ も行使 0 件なので、
結果は節目の判定そのものである。

下の表は 67 行ある。「出どころ」が「要件 5.2」または「要件 5.3」の行は初期配置をそのまま
受け取ったもので、「本タスク」の行は本タスクが節目を当てたものである。

| 束 | 段階 | 出どころ | 段階を当てた理由 |
| --- | :---: | --- | --- |
| 起動と挨拶 | A | 要件 5.2 | 初期配置のまま |
| 会話 | A | 要件 5.2 | 初期配置のまま |
| 撫で | A | 要件 5.2 | 初期配置のまま |
| メニュー | A | 要件 5.2 | 初期配置のまま。ただしこの束は初期配置が段階 B に置いた「オーナードローメニュー」を含む（2-5 の裁定候補 1） |
| 終了 | A | 要件 5.2 | 初期配置のまま |
| 自発発話 | A | 要件 5.2 | 初期配置の「自分から喋る（ランダムトーク・時報・分）」 |
| 名前の記憶 | A | 要件 5.2 | 初期配置の「名前を尋ねて覚える」 |
| 更新 | B | 要件 5.2 | 初期配置の「ネットワーク更新」。「更新」のテーマを持つので段階 B の先頭（要件 5.3） |
| インストール | B | 要件 5.2 | 初期配置の「nar インストール（D&D 含む）」。「更新」のテーマを持つので段階 B の先頭（要件 5.3） |
| 切替 | B | 要件 5.2 | 初期配置の「シェル／バルーン切替」 |
| 消滅 | B | 要件 5.2 | 初期配置のまま |
| スリープ復帰 | C | 要件 5.2 | 初期配置のまま |
| バッテリー | C | 要件 5.2 | 初期配置のまま |
| スクリーンセーバー | C | 要件 5.2 | 初期配置のまま |
| フルスクリーン退避 | C | 要件 5.2 | 初期配置のまま |
| 最小化 | C | 要件 5.2 | 初期配置のまま |
| ディスプレイ変化 | C | 要件 5.2 | 初期配置のまま |
| サウンド | C | 要件 5.2 | 初期配置のまま。テーマ 0 だが規則 ⑶ が掛からない（2-5 の裁定候補 3） |
| 環境の照会 | C | 要件 5.3 | `system.*` の照会なので段階 C の末尾。テーマ 0 だが規則 ⑶ が掛からない（2-5 の裁定候補 3） |
| 多重ゴースト | D | 要件 5.2 | 初期配置のまま |
| コミュニケート | D | 要件 5.2 | 初期配置のまま |
| 呼び出し | D | 要件 5.2 | 初期配置のまま |
| SSTP | D | 要件 5.2 | 初期配置のまま |
| FMO | D | 要件 5.2 | 初期配置のまま |
| PLUGIN | D | 要件 5.2 | 初期配置のまま |
| リンク | D | 要件 5.2 | 初期配置の「`x-ukagaka-link`」 |
| 外部アプリ | E | 要件 5.2 | 初期配置の「外部アプリ Ex」 |
| 開発者機能 | E | 要件 5.2 | 初期配置のまま |
| トランスレータ | E | 要件 5.2 | 初期配置のまま |
| ヘッドライン | E | 要件 5.2 | 初期配置のまま。ただし初期配置が挙げた理由は台帳と食い違う（2-5 の裁定候補 2） |
| 絵の重ね方 | A | 本タスク | 重ね絵が出ず立ち絵が元の 1 枚のまま止まる＝「そこにいて」が成り立たない |
| サーフェスアニメーション | A | 本タスク | 目も口も動かない止め絵のまま立ち、瞬きもしない＝「そこにいて」 |
| マウスの矢印 | A | 本タスク | どこに触れられるのかが見て分からない＝「触れて」 |
| シェル定義の転記 | A | 本タスク | 書式の宣言を読まない定義ファイルでは読み取りそのものが途中で止まる＝「そこにいて」の前提 |
| descript の転記 | A | 本タスク | 作者が決めた最初のサーフェスと最初のバルーンが使われない＝「そこにいて」 |
| 定義ファイルの文字コード | A | 本タスク | 題や説明文が文字化けして日本語の名前が読めない＝「そこにいて」の前提 |
| バルーンの文字 | A | 本タスク | 作者が選んだ書体も文字色も効かず、どのゴーストも同じ見た目で喋る＝「話す」 |
| バルーンのリンク | A | 本タスク | 本文の中のどこを押せばよいか分からない＝「話す」の受け口 |
| 選択肢の目印 | A | 本タスク | 選択肢が並んでも今どれを選んでいるかが見えない＝「話す」の受け口 |
| バルーンの付属画像 | A | 本タスク | 続きがあるのか押すのを待たれているのかが示されず、利用者がバルーンの前で止まる＝「話す」 |
| 窓の配置と重なり | A | 本タスク | 起動のたびに画面の同じ隅へ出て、相方が本体の後ろに隠れたまま出てこない＝「そこにいて」 |
| 入力窓とダイアログ | A | 本タスク | 名前や答えを尋ねる場面で窓が出ず会話が止まる＝初期配置が段階 A に置く「名前を尋ねて覚える」の相手側 |
| 動作モードの出入り | A | 本タスク | 割り込みを止める型に入れず、長い台詞の途中で撫でられて話が飛ぶ＝「触れて、話す」 |
| キーとゲームパッド | A | 本タスク | キーを叩いてもゴーストに届かず手応えが返らない＝「触れて」（もう一方の候補は 2-5 の裁定候補 5） |
| SHIORI の要求と応答 | A | 本タスク | 話しかけの合図そのものが通じず、何を尋ねられているかも分からない＝「話す」の土台 |
| 組み込みの置換語 | A | 本タスク | 時刻や画面の大きさを指す語がそのままの綴りで台詞に出る＝「話す」 |
| イベントの呼び起こし | A | 本タスク | 台本が自分で次の場面を呼び出せず、話が一区切りで止まる＝「話す」 |
| 同期オブジェクト | A | 本タスク | 掛け合いの間が合わず、相手を待たずに一人だけ先に喋る＝「話す」 |
| 着せ替え | B | 本タスク | 作者が用意した衣装や小物へ着替えられない＝「育てて」。`ukadoc:dev_bind` は段階 B の「切替」と同じ shell/master の中身を書くページである |
| 配布物の素性 | B | 本タスク | ゴースト一覧に作者名も説明も出ず、どれを入れたのか見分けが付かない＝「迎えて」 |
| 投げ込み | B | 本タスク | 初期配置が段階 B に置く「nar インストール（D&D 含む）」の D&D の受け口（テーマの食い違いは 2-5 の裁定候補 4） |
| 休止と復帰 | B | 本タスク | 裏へ控えるときと戻ってきたときの一言が無い＝段階 B の「切替」の裏側（`ukadoc:list_shiori_event:OnCacheSuspend:1`・`ukadoc:list_shiori_event:OnCacheRestore:1`） |
| 好感度の絵柄 | B | 本タスク | どれだけ長く付き合ってきたかをゴーストが知らず、節目に触れる話ができない＝「育てて」（もう一方の候補は 2-5 の裁定候補 6） |
| `ukadoc:manual_balloon` | B | 本タスク | 配布されたバルーンのフォルダに何が入るかを並べるページ＝「迎えて」 |
| `ukadoc:manual_directory` | B | 本タスク | 配布物のフォルダ構成の全体像を示すページ＝「迎えて」 |
| `ukadoc:manual_ghost` | B | 本タスク | 配布されたゴーストのフォルダに何が入るかを並べるページ＝「迎えて」 |
| OS の変化の察知 | C | 本タスク | 機械が重いことも席を外して鍵が掛かったことにも気づかない＝「察してくれる」。`foundation` は段階 C の 5 束と同じである |
| ごみ箱 | C | 本タスク | ごみ箱が溜まっていることを教えてくれない＝「察してくれる」 |
| 壁紙 | C | 本タスク | 壁紙を変えても気づかない＝「察してくれる」 |
| 通知領域 | C | 本タスク | 隠しているあいだにそこにいる印が画面に残らない＝「察してくれる」（もう一方の候補は 2-5 の裁定候補 9） |
| 予定表 | C | 本タスク | 予定を教えてくれず、約束の前に声を掛けてもらえない＝「察してくれる」 |
| 一覧と汎用プロパティの照会 | C | 本タスク | 問い合わせに答えるだけの照会系で、`foundation` が「環境の照会」と同じ＝要件 5.3 の段階 C の末尾に並ぶ |
| 読み上げと聞き取り | E | 本タスク | 台詞が声にならず、話しかけても言葉として届かない＝外の装置との出入り＝「周辺」（もう一方の候補は 2-5 の裁定候補 10） |
| 書庫 | E | 本タスク | 台本から荷物をまとめたり開いたりできない＝作り手の道具＝「周辺」 |
| 作り付けの窓 | E | 本タスク | ベースウェアが自分で持っている説明書や設定の窓へ台詞から案内できない＝「周辺」（もう一方の候補は 2-5 の裁定候補 7） |
| 薦める場所 | E | 本タスク | 作者が案内したい外の行き先がメニューに並ばない＝「周辺」（もう一方の候補は 2-5 の裁定候補 8） |
| `ukadoc:memo` | E | 本タスク | ベースウェアの機能比較表と雑多な覚え書きのページ＝「周辺」 |

上の表の行を「出どころ」で数えると、初期配置から受け取った行が **30**（要件 5.2 が **29**・
要件 5.3 が **1**）、本タスクが節目を当てた行が **37** である。本タスクが新しく段階 D に置いた
束は **0 件** である——初期配置に名前が無い 37 のいずれも「仲間がいる」を損なわないためで、
他のゴーストや外のプログラムを相手にする束はすべて初期配置が段階 D に名指ししていた。
段階ごとの束数と項目数は次の節の `[stage.A]`〜`[stage.E]` が持つ。

### 2-5. 裁定候補（要件 5.5）

台帳の根拠が初期配置と食い違う束と、初期配置に名前が無く節目の判定が割れた束を並べる。
**どれも本タスクでは配置を変えていない。** 2-4 の表には仮の段階を 1 つ付けてあるが、それは
次の段が段階ごとの数を数えられるようにするためで、裁定で動きうる。

#### 初期配置と台帳の根拠が食い違う束

| # | 束 | 初期配置 | 台帳が示す配置 | 差の理由 |
| --- | --- | --- | --- | --- |
| 1 | メニュー | 「メニュー」を段階 A に、「オーナードローメニュー」を段階 B に分けて置く | 1 つの束「メニュー」（段階 A）。`ukadoc:dev_ownerdraw` と `ukadoc:manual_owner_draw_menu` を含む | 束の名前を機構で切ったため。2 件は互いを名指ししており、片方が見た目の設定、もう片方がその画像 3 枚の置き場で、同じ 1 つの機構の表と裏である |
| 2 | ヘッドライン | 段階 E（挙げられた理由は「テーマ 0」） | 段階 E のまま。ただし挙げられた理由は成り立たない | `ukadoc:list_shiori_resource:headlinesenserootbutton.caption:1`・`ukadoc:list_shiori_resource:headlinesensehistorybutton.caption:1`・`ukadoc:list_shiori_resource:switchautoheadlinesensebutton.caption:1` の 3 件が「装い」を持つのでテーマは 0 でない。さらに壊れ方が「黙って壊れる」なので、規則 ⑶ の 2 つ目の条件も満たさない |
| 3 | サウンド・環境の照会 | サウンドは段階 C（要件 5.2）、環境の照会は段階 C の末尾（要件 5.3） | どちらも段階 C のまま | 2 つともテーマが 0 だが、壊れ方が「黙って壊れる」なので規則 ⑶ が掛からず、段階 E へ落とす根拠が無い。音が鳴らないことも `ukadoc:list_propertysystem:system.cpu._28_30ad_30fc_29:1` のような照会が答えないことも、利用者には黙って壊れたようにしか見えない |
| 4 | 投げ込み | 段階 B（「nar インストール（D&D 含む）」の一部として） | 独立した束「投げ込み」。段階は 2-4 で初期配置に合わせて B に置いた | 初期配置の出どころである brief は、段階 B へ動かす理由を「触れ合い＋更新の 2 テーマ」と書く。台帳を引き直すとこの束の構成 id 10 件はすべて「触れ合い」だけを持ち、`ukadoc:list_shiori_event:OnFileDrop2:1` を含めて「更新」を持つ id は **0 件**である。つまり規則 ⑵（テーマ 2 つ以上）を根拠にできない |

**1 の決めること**: オーナードローメニューを段階 A のメニューと一緒に作るか、段階 B へ切り
出すか。答えで変わること——段階 A で作れば、最初から作者が絵で作ったメニューが出る。段階 B へ
回せば、段階 A のあいだは OS の素のメニューだけになる。

**2 の決めること**: ヘッドラインを段階 E に置いたままにするか。答えで変わること——置いたまま
なら網の見出しを読む機能は当分できない。段階を上げれば早く読めるようになるが、そのぶん
段階 A〜D の束が後ろへ下がる。

**3 の決めること**: テーマ 0 という事実だけで束を段階 E へ落とす道を作るか、規則 ⑶ の 2 つ目の
条件（壊れ方が見た目の差以下）を守り続けるか。答えで変わること——道を作れば、音も環境の照会も
後回しになる。守り続ければ、この 2 つは段階 C のままで、里々やヤヤの雛形が使う音の再生が
段階 C までに動く。

**4 の決めること**: 投げ込みを独立した束のまま段階 B に置くか、テーマが 1 つしか無いことを
理由に段階 C へ戻すか。答えで変わること——段階 B に置けば、画像や文章をゴーストに落として
渡す遊びが早く動く。段階 C へ戻せば、それは後回しになる。

#### 初期配置に名前が無く、節目の判定が割れた束

要件 5.2 はこの 6 つを名指ししていない。2-4 の表では左の段階を仮に置いたが、右の段階も同じ
くらい筋が通る。

| # | 束 | 2-4 が置いた段階 | もう一方の候補 | 割れる理由 |
| --- | --- | :---: | :---: | --- |
| 5 | キーとゲームパッド | A | C | 「触れて」に当たる入力装置だが、テーマが「触れ合い」と「気配り」に割れている。`ukadoc:list_shiori_event:OnKeyPress:1` は撫でと同じ手出しで段階 A 寄り、`ukadoc:list_shiori_event:OnGamepadConnected:1` は機械の様子に気づく話で段階 C 寄りである |
| 6 | 好感度の絵柄 | B | E | `ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminutemonthly:1` のような記録はゴーストが付き合いの長さを知る口で「育てて」に当たるが、`ukadoc:descript_ghost:shiori.logo.file_2c_30d5_30a1_30a4_30eb_540d:1` が飾るのはベースウェアが持つ使用時間の画面で「周辺」に当たる |
| 7 | 作り付けの窓 | E | A | `ukadoc:list_sakura_script:_5c_21_5bopen_2creadme_5d:1` はベースウェアの画面を開く「周辺」の口だが、台詞から案内する動きなので段階 A のメニューと地続きでもある |
| 8 | 薦める場所 | E | A | `ukadoc:list_shiori_resource:sakura.recommendsites:1` は外の行き先の一覧で「周辺」だが、その一覧が並ぶ先は段階 A のメニューである |
| 9 | 通知領域 | C | E | `ukadoc:list_shiori_event:OnTrayBalloonClick:1` は隠しているあいだの「そこにいる印」で段階 C 寄りだが、通知領域そのものはベースウェアの画面なので「周辺」でもある |
| 10 | 読み上げと聞き取り | E | D | `ukadoc:list_shiori_event:OnVoiceRecognitionWord:1` の相手は利用者の声で、テーマは「交わり」＝段階 D の節目の語である。外の装置を借りる点を採れば「周辺」になる |

**5〜10 に共通の決めること**: 左右どちらの段階に置くか。答えで変わること——段階 A や B に
置いた束は M2 の早いウェーブに入って先に動くようになり、段階 E に置いた束は当分作られない。

## 3. 順序付き束一覧

<!-- 段 3（タスク 4.3）で書く: 束ごとに段階・順位・資産の広さ・基盤共有度を置いた囲みと、
     束名・順位・資産・共有度だけを持つ本文の表。壊れ方とテーマは `linkage.md` の欄名で指す。 -->

下の囲みは着手時（2026-09-12）に置いた器である。**段階ごとの 3 つの数はまだ数えていない
仮の 0 であり、段 3（タスク 4.3）で数え直した値に置き換える。** 5 段階すべてを、値が 0 の
段階も省略せずに置く。

```toml
[stage.A]
bundles = 0
singles = 0
items = 0

[stage.B]
bundles = 0
singles = 0
items = 0

[stage.C]
bundles = 0
singles = 0
items = 0

[stage.D]
bundles = 0
singles = 0
items = 0

[stage.E]
bundles = 0
singles = 0
items = 0
```

## 4. 段階 A の主障壁

<!-- 段 3（タスク 4.6）で書く: ページ別の状態分布を台帳から数え直して置く。 -->

## 5. 根拠表への参照

<!-- 段 3（タスク 4.6）で書く: テーマ別の状態分布は全体報告を、SSP 世代別はドメイン別報告
     4 本を参照で指す。写しを作らない。資産の広さの物差しであるテンプレート辞書の表も
     この節に置く（段 3・タスク 4.1）。 -->

### 5-1. 資産の広さの物差し——テンプレート辞書

順位の根拠の 3 つ目「影響する既存資産の広さ」は、里々とヤヤの標準テンプレートゴーストの
辞書に現れる語彙を物差しにする。この節は、その辞書をどこから取り、辞書の語彙をどうやって
カタログの項目に結び付けたかの記録である。2 本とも取得できたので、取得できないときの退路
（要件 6.8）は使っていない——下の表の `fallback` は 2 本とも偽である。

#### 取得（2026-09-12）

| テンプレート | 取ってきた物 | 大きさ |
| --- | --- | ---: |
| ポストと狛犬（里々） | `post124.zip` | 645,716 バイト |
| はろーYAYAわーるど（YAYA） | `yayame.nar` | 628,261 バイト |

配布元にたどり着いた道筋も残す。公式の側からたどれる置き場だけを採った。

- **ポストと狛犬**: 里々の解説 wiki のページから作者のサイト（電気で動くうにゅう・廃屋の
  夏）へたどり、そこが里々のサンプルゴーストとして置いている最終版を取った。同じ wiki が
  案内しているもう 1 つの置き場は、有志が本家に手を入れた改造版だと wiki 自身が書いて
  いるので採らなかった。
- **はろーYAYAわーるど**: 整備班の配布ページが GitHub の Releases から取るよう書いて
  いるので、そのページが名指す置き場から取った。

配布物はリポジトリに入れていない。作業用の一時ディレクトリで展開してテキストのファイルだけ
を読み、読み終えたあとに消した。ゴーストは 1 度も起動していない。辞書を動かしてもいない。
同梱されている実行される形のファイル（里々とヤヤの本体・付属の道具）には触れていない。

#### 写し方

辞書から 4 種類の語彙を取り出した。里々は見出しが `＊` で始まる形、ヤヤは関数の名前が
そのままイベントの名前になる形なので、どちらも本文の綴りをそのまま拾える。

1. イベントの名前（`On` で始まる綴り。`OnUpdate.OnDownloadBegin` のような点でつないだ
   形は 1 つの名前として扱い、点で切った断片は数えない）
2. さくらスクリプトのタグの綴り
3. プロパティの名前
4. descript のキー（ゴーストとシェルの説明ファイル）

取り出した綴りは、カタログの題と突き合わせて項目に写す。突き合わせの規則は 3 つある。

- **種別ごとに相手のページを限る。** イベントは SHIORI イベントの 2 ページ、タグはさくら
  スクリプトのページ、プロパティはプロパティシステムのページ、descript のキーは読んだ
  ファイルの持ち主（ゴーストの説明ファイルならゴーストのページ、シェルの説明ファイルなら
  シェルのページ）に限る。限らないと、同じ綴りの欄が別のページにもあるために、語彙 1 つが
  無関係な項目まで連れてくる。
- **値の欄は落とし、副命令は残す。** 小文字の英字で始まる欄を副命令、それ以外（大文字で
  始まるもの・数字・日本語）を値と見なす。`\![open,readme]` は綴りのまま、
  `\![raise,OnAiTalk]` は `\![raise` まで、`\s[10]` は `\s` まで縮める。
- **族はしかたのないときだけ。** 副命令まで含めて当たらないときにだけ、同じ頭を持つ項目を
  まとめて当てる。族で当てたタグは里々 0 件・ヤヤ 2 件（`\![change` と `\_l`）である。

#### 写した数（2026-09-12 にその場で数え直した実測）

| 語彙の種別 | 里々 取り出し | 里々 写せた | 里々 写せず | ヤヤ 取り出し | ヤヤ 写せた | ヤヤ 写せず |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| イベントの名前 | 67 | 67 | 0 | 147 | 84 | 63 |
| さくらスクリプトのタグ | 17 | 15 | 2 | 70 | 41 | 29 |
| プロパティの名前 | 6 | 0 | 6 | 5 | 3 | 2 |
| descript のキー | 39 | 36 | 3 | 43 | 43 | 0 |
| 合計 | 129 | 118 | 11 | 265 | 171 | 94 |

写した項目の数は里々 122・ヤヤ 188 で、2 本を合わせると 216 である（両方が使う項目が
あるので、2 つの和より少ない）。語彙 1 つが項目 1 つに写るとは限らない——族で当てた場合と、
同じ題が 2 つのページにある場合は複数になる——ので、写せた語彙の数と項目の数は一致しない。

#### 写せなかった語彙（里々 11 件・ヤヤ 94 件）

里々の 11 件の内訳。

- タグ 2 件。`\![open,browzer`（辞書の側の綴り誤りで、正しい綴りは同じ辞書の別の行に
  ある）と、文字としての逆斜線 1 件。
- プロパティらしき綴り 6 件。`sakura.recommendsites`・`kero.recommendsites`・
  `sakura.portalsites`・`kero.recommendbuttoncaption` は SHIORI が返す資源の名前であって
  プロパティシステムの名前ではない。残る 2 件（`index.html`・`shiori.htm`）はファイル名を
  形が似ているために拾ったものである。
- descript のキー 3 件。`shiori.logo.filename`・`shiori.logo.x`・`shiori.logo.y` は
  シェルの説明ファイルに書かれているが、カタログのシェルのページに同じ題が無い。
- イベントの名前は 0 件。里々が使う 67 個の名前はすべて写せた。

ヤヤの 94 件の内訳。

- イベントの名前 63 件。テンプレートが内部で使う自前の名前で、正典の語彙ではない
  （`On_name`・`On_Get_Supported_Events`・`OnAYLXXWriteChangeList2` など）。
- タグの綴り 29 件。辞書の文字列の中の逃がし文字や、ファイルの通り道を拾ってしまったもの
  （`\[`・`\ghost`・`\master` など）である。
- プロパティらしき綴り 2 件。`sakura.name`・`kero.name` は説明ファイルのキーであって
  プロパティシステムの名前ではない（説明ファイルのキーとしては写してある）。
- descript のキーは 0 件。ゴーストとシェルの説明ファイルのキー 43 件はすべて写せた。

下の囲みが、写した結果の正本である。`files` は配布物を展開した中の位置、`ids` は写した
項目である。項目が実在することは常時の検査（判定 ⑴）が数え直す。

```toml
[[template]]
name = "ポストと狛犬"
shiori = "里々"
url = "http://ukgk.s34.xrea.com/poskoma/post124.zip"
fetched_on = "2026-09-12"
files = [
  "ghost/master/descript.txt",
  "shell/master/descript.txt",
  "ghost/master/dic01_Base.txt",
  "ghost/master/dic02_Event.txt",
  "ghost/master/dic03_Menu.txt",
  "ghost/master/dic04_Change.txt",
  "ghost/master/dic05_Communicate.txt",
  "ghost/master/dic06_String.txt",
  "ghost/master/dic07_Time.txt",
  "ghost/master/dic08_Labo.txt",
  "ghost/master/dic09_ExEvent.txt",
  "ghost/master/dic10_SAORI_test.txt",
  "ghost/master/replace.txt",
  "ghost/master/replace_after.txt",
  "ghost/master/satori_conf.txt",
]
mapping = "語彙の種別ごとに突き合わせる相手のページを限り（イベントは SHIORI イベントの 2 ページ、タグはさくらスクリプトのページ、プロパティはプロパティシステムのページ、descript のキーは読んだファイルの持ち主のページ）、カタログの題から値の欄を落とした綴りとの一致で写す。小文字の英字で始まる欄は副命令として残し、大文字始まり・数字・日本語の欄は値として落とす。副命令まで含めて当たらないときだけ同じ頭の族をまとめて当てる。"
fallback = false
ids = [
  "ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:craftmanurl_2cURL:1",
  "ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:icon_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:kero.name_2c_540d_524d:1",
  "ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1",
  "ukadoc:descript_ghost:sakura.name_2c_540d_524d:1",
  "ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:type_2c_7a2e_5225:1",
  "ukadoc:descript_shell:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:craftmanurl_2cURL:1",
  "ukadoc:descript_shell:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:id_2cID_540d:1",
  "ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:menu.background.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.background.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.background.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.foreground.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.foreground.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.sidebar.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.sidebar.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:name_2c_30b7_30a7_30eb_540d:1",
  "ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:type_2c_7a2e_5225:1",
  "ukadoc:list_sakura_script:_5c-:1",
  "ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1",
  "ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1",
  "ukadoc:list_sakura_script:_5c6:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ccommunicatebox_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cmailer_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c__2b:1",
  "ukadoc:list_sakura_script:_5c_q:1",
  "ukadoc:list_sakura_script:_5cbID_756a_53f7:1",
  "ukadoc:list_sakura_script:_5cb_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5cc:1",
  "ukadoc:list_sakura_script:_5cn_5bhalf_5d:1",
  "ukadoc:list_sakura_script:_5cw_6642_9593:1",
  "ukadoc:list_sakura_script:_5cx:1",
  "ukadoc:list_shiori_event:OnBIFF2Complete:1",
  "ukadoc:list_shiori_event:OnBIFFBegin:1",
  "ukadoc:list_shiori_event:OnBIFFComplete:1",
  "ukadoc:list_shiori_event:OnBIFFFailure:1",
  "ukadoc:list_shiori_event:OnBatteryCritical:1",
  "ukadoc:list_shiori_event:OnBatteryLow:1",
  "ukadoc:list_shiori_event:OnBoot:1",
  "ukadoc:list_shiori_event:OnChoiceSelect:1",
  "ukadoc:list_shiori_event:OnChoiceTimeout:1",
  "ukadoc:list_shiori_event:OnClose:1",
  "ukadoc:list_shiori_event:OnFirstBoot:1",
  "ukadoc:list_shiori_event:OnGhostChanged:1",
  "ukadoc:list_shiori_event:OnGhostChanging:1",
  "ukadoc:list_shiori_event:OnHeadlinesense.OnFind:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseBegin:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseComplete:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseFailure:1",
  "ukadoc:list_shiori_event:OnInstallBegin:1",
  "ukadoc:list_shiori_event:OnInstallComplete:1",
  "ukadoc:list_shiori_event:OnInstallFailure:1",
  "ukadoc:list_shiori_event:OnInstallRefuse:1",
  "ukadoc:list_shiori_event:OnKeyPress:1",
  "ukadoc:list_shiori_event:OnMinuteChange:1",
  "ukadoc:list_shiori_event:OnMouseDoubleClick:1",
  "ukadoc:list_shiori_event:OnMusicPlay:1",
  "ukadoc:list_shiori_event:OnNarCreated:1",
  "ukadoc:list_shiori_event:OnNarCreating:1",
  "ukadoc:list_shiori_event:OnNetworkHeavy:1",
  "ukadoc:list_shiori_event:OnSNTPBegin:1",
  "ukadoc:list_shiori_event:OnSNTPCompare:1",
  "ukadoc:list_shiori_event:OnSNTPCorrect:1",
  "ukadoc:list_shiori_event:OnSNTPFailure:1",
  "ukadoc:list_shiori_event:OnSSTPBreak:1",
  "ukadoc:list_shiori_event:OnShellChanged:1",
  "ukadoc:list_shiori_event:OnShellChanging:1",
  "ukadoc:list_shiori_event:OnURLDropped:1",
  "ukadoc:list_shiori_event:OnURLDropping:1",
  "ukadoc:list_shiori_event:OnUpdate.OnDownloadBegin:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareBegin:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareComplete:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareFailure:1",
  "ukadoc:list_shiori_event:OnUpdateBegin:1",
  "ukadoc:list_shiori_event:OnUpdateComplete:1",
  "ukadoc:list_shiori_event:OnUpdateFailure:1",
  "ukadoc:list_shiori_event:OnUpdateReady:1",
  "ukadoc:list_shiori_event:OnUpdatedataCreated:1",
  "ukadoc:list_shiori_event:OnUpdatedataCreating:1",
  "ukadoc:list_shiori_event:OnUserInput:1",
  "ukadoc:list_shiori_event:OnVanishButtonHold:1",
  "ukadoc:list_shiori_event:OnVanishCancel:1",
  "ukadoc:list_shiori_event:OnVanishSelected:1",
  "ukadoc:list_shiori_event:OnVanishSelecting:1",
  "ukadoc:list_shiori_event:OnVanished:1",
  "ukadoc:list_shiori_event:OnWallpaperChange:1",
  "ukadoc:list_shiori_event:OnWindowStateRestore:1",
  "ukadoc:list_shiori_event_ex:OnApplicationOperationFinish:1",
  "ukadoc:list_shiori_event_ex:OnBatteryCritical:1",
  "ukadoc:list_shiori_event_ex:OnBatteryLow:1",
  "ukadoc:list_shiori_event_ex:OnKinokoObjectChanged:1",
  "ukadoc:list_shiori_event_ex:OnKinokoObjectCreate:1",
  "ukadoc:list_shiori_event_ex:OnKinokoObjectDestroy:1",
  "ukadoc:list_shiori_event_ex:OnMusicPlay:1",
  "ukadoc:list_shiori_event_ex:OnNekodorifObjectDodge:1",
  "ukadoc:list_shiori_event_ex:OnNekodorifObjectDrop:1",
  "ukadoc:list_shiori_event_ex:OnNekodorifObjectEmerge:1",
  "ukadoc:list_shiori_event_ex:OnNekodorifObjectHit:1",
  "ukadoc:list_shiori_event_ex:OnNekodorifObjectVanish:1",
  "ukadoc:list_shiori_event_ex:OnSysResourceCritical:1",
  "ukadoc:list_shiori_event_ex:OnSysResourceLow:1",
  "ukadoc:list_shiori_event_ex:OnWebsiteUpdateNotify:1",
]

[[template]]
name = "はろーYAYAわーるど"
shiori = "YAYA"
url = "https://github.com/YAYA-shiori/konnoyayame/releases/download/23584055353/yayame.nar"
fetched_on = "2026-09-12"
files = [
  "ghost/master/descript.txt",
  "shell/master/descript.txt",
  "ghost/master/dic/emerg/yaya_emerg_dic.dic",
  "ghost/master/dic/emerg/yaya_homeurl.dic",
  "ghost/master/dic/normal/yaya_aitalk.dic",
  "ghost/master/dic/normal/yaya_bootend.dic",
  "ghost/master/dic/normal/yaya_change.dic",
  "ghost/master/dic/normal/yaya_communicate.dic",
  "ghost/master/dic/normal/yaya_etc.dic",
  "ghost/master/dic/normal/yaya_homeurl.dic",
  "ghost/master/dic/normal/yaya_menu.dic",
  "ghost/master/dic/normal/yaya_mouse.dic",
  "ghost/master/dic/normal/yaya_string.dic",
  "ghost/master/dic/normal/yaya_tmpl_util.dic",
  "ghost/master/dic/normal/yaya_word.dic",
  "ghost/master/dic/system/aya_lilith/aya_lilith.dic",
  "ghost/master/dic/system/aya_lilith/aya_lilith_ex.dic",
  "ghost/master/dic/system/yaya_base/compatible.dic",
  "ghost/master/dic/system/yaya_base/config.dic",
  "ghost/master/dic/system/yaya_base/optional.dic",
  "ghost/master/dic/system/yaya_base/shiori3.dic",
]
mapping = "語彙の種別ごとに突き合わせる相手のページを限り（イベントは SHIORI イベントの 2 ページ、タグはさくらスクリプトのページ、プロパティはプロパティシステムのページ、descript のキーは読んだファイルの持ち主のページ）、カタログの題から値の欄を落とした綴りとの一致で写す。小文字の英字で始まる欄は副命令として残し、大文字始まり・数字・日本語の欄は値として落とす。副命令まで含めて当たらないときだけ同じ頭の族をまとめて当てる。"
fallback = false
ids = [
  "ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:craftmanurl_2cURL:1",
  "ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:icon_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:kero.name_2c_540d_524d:1",
  "ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1",
  "ukadoc:descript_ghost:sakura.name_2c_540d_524d:1",
  "ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:sstp.allowunspecifiedsend_2c_6570_5024:1",
  "ukadoc:descript_ghost:type_2c_7a2e_5225:1",
  "ukadoc:descript_shell:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:craftmanurl_2cURL:1",
  "ukadoc:descript_shell:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:kero.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:menu.background.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.background.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.background.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.disable.font.color.b:1",
  "ukadoc:descript_shell:menu.disable.font.color.g:1",
  "ukadoc:descript_shell:menu.disable.font.color.r:1",
  "ukadoc:descript_shell:menu.foreground.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.foreground.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.foreground.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.sidebar.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.sidebar.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:name_2c_30b7_30a7_30eb_540d:1",
  "ukadoc:descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:type_2c_7a2e_5225:1",
  "ukadoc:list_propertysystem:ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_sakura_script:_5c-:1",
  "ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1",
  "ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1",
  "ukadoc:list_sakura_script:_5c4:1",
  "ukadoc:list_sakura_script:_5c6:1",
  "ukadoc:list_sakura_script:_5cC:1",
  "ukadoc:list_sakura_script:_5c_21_5b_2a_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cinputbox_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cpassivemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cpassivemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ccommunicatebox_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5braise_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5braiseplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cshiori_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c__21:1",
  "ukadoc:list_sakura_script:_5c__3f:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_5d:1",
  "ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1",
  "ukadoc:list_sakura_script:_5c_n:1",
  "ukadoc:list_sakura_script:_5c_q:1",
  "ukadoc:list_sakura_script:_5c_s:1",
  "ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1",
  "ukadoc:list_sakura_script:_5cbID_756a_53f7:1",
  "ukadoc:list_sakura_script:_5cb_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5cc:1",
  "ukadoc:list_sakura_script:_5ce:1",
  "ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1",
  "ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5cj_5bID_5d:1",
  "ukadoc:list_sakura_script:_5cn:1",
  "ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1",
  "ukadoc:list_sakura_script:_5cn_5bhalf_5d:1",
  "ukadoc:list_sakura_script:_5cpID_756a_53f7:1",
  "ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5cq_5bID_5d_5b_30bf_30a4_30c8_30eb_5d_307e_305f_306f_5cq_2a_5bID_5d_5b_30bf_30a4_30c8_30eb_5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID1_2cID2_2cID3..._5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1",
  "ukadoc:list_sakura_script:_5csID_756a_53f7:1",
  "ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5ct:1",
  "ukadoc:list_sakura_script:_5cw_6642_9593:1",
  "ukadoc:list_sakura_script:_5cx:1",
  "ukadoc:list_shiori_event:OnAITalk:1",
  "ukadoc:list_shiori_event:OnAnchorSelect:1",
  "ukadoc:list_shiori_event:OnBIFFBegin:1",
  "ukadoc:list_shiori_event:OnBIFFComplete:1",
  "ukadoc:list_shiori_event:OnBIFFFailure:1",
  "ukadoc:list_shiori_event:OnBalloonBreak:1",
  "ukadoc:list_shiori_event:OnBalloonChange:1",
  "ukadoc:list_shiori_event:OnBalloonClose:1",
  "ukadoc:list_shiori_event:OnBalloonTimeout:1",
  "ukadoc:list_shiori_event:OnBoot:1",
  "ukadoc:list_shiori_event:OnChoiceSelect:1",
  "ukadoc:list_shiori_event:OnChoiceTimeout:1",
  "ukadoc:list_shiori_event:OnClose:1",
  "ukadoc:list_shiori_event:OnCommunicate:1",
  "ukadoc:list_shiori_event:OnDisplayChange:1",
  "ukadoc:list_shiori_event:OnEmbryoExist:1",
  "ukadoc:list_shiori_event:OnFirstBoot:1",
  "ukadoc:list_shiori_event:OnGhostCallComplete:1",
  "ukadoc:list_shiori_event:OnGhostCalled:1",
  "ukadoc:list_shiori_event:OnGhostCalling:1",
  "ukadoc:list_shiori_event:OnGhostChanged:1",
  "ukadoc:list_shiori_event:OnGhostChanging:1",
  "ukadoc:list_shiori_event:OnHeadlinesense.OnFind:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseBegin:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseComplete:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseFailure:1",
  "ukadoc:list_shiori_event:OnInitialize:1",
  "ukadoc:list_shiori_event:OnInstallBegin:1",
  "ukadoc:list_shiori_event:OnInstallComplete:1",
  "ukadoc:list_shiori_event:OnInstallFailure:1",
  "ukadoc:list_shiori_event:OnInstallRefuse:1",
  "ukadoc:list_shiori_event:OnKeyPress:1",
  "ukadoc:list_shiori_event:OnMinuteChange:1",
  "ukadoc:list_shiori_event:OnMouseClick:1",
  "ukadoc:list_shiori_event:OnMouseDoubleClick:1",
  "ukadoc:list_shiori_event:OnMouseDown:1",
  "ukadoc:list_shiori_event:OnMouseDragEnd:1",
  "ukadoc:list_shiori_event:OnMouseDragStart:1",
  "ukadoc:list_shiori_event:OnMouseGesture:1",
  "ukadoc:list_shiori_event:OnMouseMove:1",
  "ukadoc:list_shiori_event:OnMouseUp:1",
  "ukadoc:list_shiori_event:OnMouseWheel:1",
  "ukadoc:list_shiori_event:OnNarCreated:1",
  "ukadoc:list_shiori_event:OnNarCreating:1",
  "ukadoc:list_shiori_event:OnNekodorifExist:1",
  "ukadoc:list_shiori_event:OnNotifyDressupInfo:1",
  "ukadoc:list_shiori_event:OnNotifySelfInfo:1",
  "ukadoc:list_shiori_event:OnNotifyUserInfo:1",
  "ukadoc:list_shiori_event:OnOtherGhostBooted:1",
  "ukadoc:list_shiori_event:OnOtherGhostChanged:1",
  "ukadoc:list_shiori_event:OnOtherGhostClosed:1",
  "ukadoc:list_shiori_event:OnRecommendsiteChoice:1",
  "ukadoc:list_shiori_event:OnSNTPBegin:1",
  "ukadoc:list_shiori_event:OnSNTPCompare:1",
  "ukadoc:list_shiori_event:OnSNTPFailure:1",
  "ukadoc:list_shiori_event:OnSSTPBreak:1",
  "ukadoc:list_shiori_event:OnScreenSaverEnd:1",
  "ukadoc:list_shiori_event:OnScreenSaverStart:1",
  "ukadoc:list_shiori_event:OnSecondChange:1",
  "ukadoc:list_shiori_event:OnShellChanged:1",
  "ukadoc:list_shiori_event:OnShellChanging:1",
  "ukadoc:list_shiori_event:OnSurfaceChange:1",
  "ukadoc:list_shiori_event:OnSurfaceRestore:1",
  "ukadoc:list_shiori_event:OnTextDrop:1",
  "ukadoc:list_shiori_event:OnTranslate:1",
  "ukadoc:list_shiori_event:OnURLDropping:1",
  "ukadoc:list_shiori_event:OnUpdate.OnDownloadBegin:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareBegin:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareComplete:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareFailure:1",
  "ukadoc:list_shiori_event:OnUpdateBegin:1",
  "ukadoc:list_shiori_event:OnUpdateComplete:1",
  "ukadoc:list_shiori_event:OnUpdateFailure:1",
  "ukadoc:list_shiori_event:OnUpdateReady:1",
  "ukadoc:list_shiori_event:OnUpdatedataCreated:1",
  "ukadoc:list_shiori_event:OnUpdatedataCreating:1",
  "ukadoc:list_shiori_event:OnVanishCancel:1",
  "ukadoc:list_shiori_event:OnVanishSelected:1",
  "ukadoc:list_shiori_event:OnVanishSelecting:1",
  "ukadoc:list_shiori_event:OnVanished:1",
  "ukadoc:list_shiori_event:OnWindowStateRestore:1",
  "ukadoc:list_shiori_event_ex:OnStampAdd:1",
  "ukadoc:list_shiori_event_ex:OnStampInfo:1",
  "ukadoc:list_shiori_event_ex:OnStampInfoCall:1",
]
```

書き戻しの前後を記録する器を下に置く。**いずれもまだ数えていない仮の 0 であり、段 3
（タスク 4.4）で数え直した値に置き換える。**

```toml
[priority_blank]
alias = 0
not_applicable = 0
```

## 6. 申し送りの処分台帳

<!-- 段 5（タスク 6.3）で書く: 調査 4 本のブリーフィングと完了 spec 5 本から拾った申し送りを
     1 件ずつ処分する。拾った件数・処分済み件数・裁定候補の件数を冒頭に並べる。 -->

## 7. 第二段の改訂記録

<!-- 段 4（タスク 5.1〜5.3）で書く: 適合検証 20 項目と持ち越し 8 行の 1 行ずつに判断を付け、
     順位を動かす行は変更前・変更後・理由を書く。 -->

## 8. 是正候補への参照

<!-- 段 5（タスク 6.2）で書く: 台帳の id と brief の所有宣言が食い違うものの一覧への参照を
     置く。一覧の本体は `roadmap-draft.md` にある。 -->
