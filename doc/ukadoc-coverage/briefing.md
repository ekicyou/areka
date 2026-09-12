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

<!-- 段 3（タスク 4.2）で書く: 5 つの節目の定義表・束の初期配置・写像の規則 3 つ・
     台帳の根拠が初期配置と食い違う束の裁定候補。 -->

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
