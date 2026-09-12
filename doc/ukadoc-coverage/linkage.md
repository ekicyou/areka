# 束の名付けと帰属

この文書は「どの項目がどの束に属するか」の正本である。台帳の `priority` は、この文書の帰属と
`briefing.md` の順位から機械で導いた写しであり、逆ではない。

## 書き方の規律

- 台帳の項目を指すときは、必ず引用符か逆引用符で囲む。地の文に裸で書かない。囲みの中では
  TOML の文字列として引用符で囲む。
- 束の 8 つの事柄（名前・由来する機械の束・構成 id の全列挙・跨ぐドメイン・成立に要る最小の
  基盤・欠けると壊れる振る舞い・壊れ方の最悪値・テーマ）は、束ごとに 1 か所へまとめて書く。
- 数はこの文書の囲みに 1 度だけ置き、常時の検査が数え直して突き合わせる。本文で同じ数に
  触れるときは、値を写さず欄の名前で指す。
- 束の解説はこの文書にだけ書く。報告 5 本へ説明を書き足さない。

## 補修した関連

段 1（2026-09-12）で台帳へ書き足した関連は 3 本である。数え方: この節を書いたコミットの
`doc/ukadoc-coverage/ledger/` の差分のうち、`{ kind = ..., to = ... }` の**追加行**を数えた
（既存の行の削除・書き換えは 0 行）。この数は履歴の数なので機械は数え直さない。

| 台帳 | 足した本数 | 種別 | 書いた行（起点） | 相手（終点） |
| --- | --- | --- | --- | --- |
| assets | 0 | — | — | — |
| property | 1 | `configures` | `ukadoc:list_propertysystem:currentghost.seriko.zorder:1` | `ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1` |
| sakura-script | 2 | `triggers` | `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1` / `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1` | いずれも `ukadoc:list_shiori_event:OnInstallComplete:1` |
| shiori | 0 | — | — | — |

3 つの連鎖の着地（機械の束の id は `report/summary.md` の束の一覧から引いた）:

- 時刻の刻み: 追加 0 本。`ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1` を束 id とする
  1 つの束に既に収まっていた。
- 重なり順: 追加 1 本。これで `ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1` を
  束 id とする 1 つの束に収まった（足す前、`ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`
  はどの束にも入っていなかった）。
- インストール: 追加 2 本。3 つの id はいずれも
  `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` を束 id とする 1 つの束に収まる
  （この束は段 2 で複数の名前付き束に分ける）。

書き方の約束を 3 つ守った——同じ繋がりを往復で 2 度書いていない（足した 3 本の相手側の行は
いずれも戻りの関連を持たない）、種別は README の 6 つのうち `configures` と `triggers` の 2 つ
だけを使った、名前付き束への帰属を表すためだけの関連は 1 本も書いていない（帰属の正本はこの
文書である）。

**既存の向きの流儀は正典の種別定義と逆である。** README「関連の種別は 6 つ」は `configures` を
「設定キー → 挙動・タグ・イベント」と定めるが、足す前の台帳で assets の設定キーを指していた
`configures` 既存 29 本（property の行から 24 本・shiori の行から 5 本。終点はすべて `descript_` で
始まるページの設定キー）は、いずれも設定キーを**始点**ではなく**終点**に置いていた。上に足した
1 本もその流儀に従った。束は向きを持たないので機械の束の判定には影響しない。

## 機械の束の分割

段 1 を終えた時点で最も大きい機械の束は、束 id を
`ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` とする 53 件である。この 53 件は
1 つの機能ではないので、名前付き束へ分ける。

数え直し（2026-09-12・すべてこの節を書いた時点で数えた）。⑴ 構成 id の数: `report/summary.md`
「ドメインを跨いで繋がった束」の表からこの束 id の行を取り、構成 id の欄を読点で切って数えて
**53**。⑵ そのうちページ単位の id: 同じ 53 件のうち、id を `:` で切って 2 節しか持たないものを数えて
**13**（残る 40 件は 4 節）。⑶ 関連: 4 台帳の `links` をすべて読んで向きの無いグラフに
直し、13 の id のどちらかを端に持つ辺を数えて **46 本**。46 本は全部 assets 台帳の行に書かれた
`same-feature` で、内訳はページ id どうしが **26 本**・ページ id と項目 id が **20 本**。

### ページ単位の id が機能の繋がりでないこと

⑴ **綴りの形が違う。** id を `:` で切ると、項目の id は 4 節（頭の `ukadoc`・ページ名・正典の
見出し・版）でできているが、この 13 件は 2 節（頭の `ukadoc`・ページ名）しか持たない。53 件は
この形で 40 件と 13 件にちょうど分かれる。

⑵ **カタログの題がページの題そのものである。** 例えば `ukadoc:manual_update` の題は
「ネットワーク」、`ukadoc:dev_shell` の題は「シェルの作成」で（いずれも正典の題から共通の接頭辞
「UKADOC Project 」を落とした形）、正典のページ 1 枚の題と同じである。

⑶ **台帳の備考が自ら粒度を書いている。** 13 件すべての備考の冒頭の段落に「この項目はページ
1 枚をまとめて指す粗い粒度で、ページの中の記述を 1 つずつ分けて持っていない」がある。位置は
どれも 3 文目で、前に置かれているのは台帳共通の「壊れ方: …」と「記録: …」の 2 文である
（確かめ方: `ledger/assets.toml` の当該 13 件の `note` を読み、冒頭の段落を「。」で切って 3 番目の
文が上の 1 文と逐語で一致することを見た。13 分の 13）。

⑷ **関連を張った基準も備考が書いている。** 13 件すべての備考に「関連の向き」の段落があり、
そこで「正典の本文が相手のページを名指しで指しているとき、指している側にだけ関連を置き、
指されている側には戻りの関連を置かない」と宣言している（13 分の 13）。つまりこの 46 本の
`same-feature` が記録しているのは正典の本文にある相互参照であって、片方が欠けるともう片方の
振る舞いが壊れるという areka の側の関係ではない。

⑸ **この 13 件を外すと 53 件はばらばらになる。** 13 件を取り除いて残り 40 件の連結成分を数え直すと
10 に割れる（27 件・5 件と、1 件が 8 つ）。53 件を 1 つに保っていたのはページの相互参照だけで
あって、ネットワーク更新の 27 件・インストールの 5 件・孤立した 8 件の間には、台帳が記録した
機能の繋がりが 1 本も無い。

13 件の状態はすべて `absent` で、テーマを持つのは 3 件だけである（`ukadoc:manual_update` が
「更新」、`ukadoc:manual_balloon` と `ukadoc:manual_shell` が「装い」。残り 10 件は空）。

### 13 のページ単位の id の行き先

| ページ単位の id | 正典の題 | 行き先 | 由来する機械の束 id |
| --- | --- | --- | --- |
| `ukadoc:dev_bind` | アニメーション/着せ替えの設定 | 切替 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:dev_nar` | 配布用ファイルの作成 | 開発者機能 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:dev_ownerdraw` | オーナードローメニューの設定 | メニュー | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:dev_shell` | シェルの作成 | 切替 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:dev_update` | ネットワーク更新への対応 | 更新 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_balloon` | バルーン | 単独項目 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_directory` | 全体の構成 | 単独項目 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_ghost` | ゴースト | 単独項目 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_install` | インストール | インストール | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_owner_draw_menu` | オーナードローメニュー | メニュー | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_shell` | シェル | 切替 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_translator` | トランスレータ | トランスレータ | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |
| `ukadoc:manual_update` | ネットワーク | 更新 | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` |

行き先の付いていないページ単位の id は **0 件**である（13 件すべてに行き先がある）。行き先は
機構で切った。理由を行き先ごとに書く。

- **切替**（3 件）: `ukadoc:dev_bind` は着せ替えの設定一式、`ukadoc:dev_shell` はシェルを 1 つ作る
  手順（立ち絵・surfaces.txt・当たり判定・着せ替え）、`ukadoc:manual_shell` は shell/master に置く
  ファイル一式。3 ページとも同じ shell/master の中身（descript.txt・surfaces.txt・着せ替えの定義）
  を書くための説明である。分割後も 3 件は関連で繋がったまま残る（`ukadoc:dev_bind` と
  `ukadoc:dev_shell` が互いを、`ukadoc:dev_shell` が `ukadoc:manual_shell` を名指ししている）。
- **メニュー**（2 件）: `ukadoc:dev_ownerdraw` はオーナードローメニューの見た目の設定、
  `ukadoc:manual_owner_draw_menu` はその画像 3 枚の置き場で、同じ 1 つの機構の表と裏である
  （2 件は互いを名指ししており、分割後もこの繋がりが束の中に残る）。**要件 5.2 の初期配置は
  「メニュー」を段階 A に・「オーナードローメニュー」を段階 B に分けて置いている。** ここで
  1 つの束にまとめたのは名前を機構で切ったためであり、段階の
  食い違いは黙って変えず `briefing.md` の裁定候補（要件 5.5）で扱う。
- **更新**（2 件）: `ukadoc:dev_update` はネットワーク更新のサーバ側とゴースト側の準備、
  `ukadoc:manual_update` は更新に関わるファイル（updates2.dau・updates.txt・delete.txt など）の
  置き場で、置く物と作る手順という同じ 1 つの仕組みの両側である（2 件は互いを名指ししており、
  分割後もこの繋がりが束の中に残る）。`ukadoc:manual_update` は 13 件のうち唯一「更新」の
  テーマを持つ。
- **インストール**（1 件）: `ukadoc:manual_install` は配布アーカイブの中身の並べ方と install.txt・
  developer_options.txt の役割を定めるページである。
- **開発者機能**（1 件）: `ukadoc:dev_nar` は配布用アーカイブ（nar）の作り方で、ゴーストの作り手の
  手元でだけ使う手順である。設計 D-7 はこれを「nar の作成」という別の束にする案を書いたが、
  タスク 3.4 が書く「開発者機能」に寄せて名前を増やさないことにした。
- **トランスレータ**（1 件）: `ukadoc:manual_translator` は makoto.dll の置き場と働きを定める。
  13 件のうち台帳の担当欄が埋まっている唯一のページ id でもある（`areka-P0-translate-pipeline`）。
- **単独項目**（3 件）: `ukadoc:manual_balloon`・`ukadoc:manual_directory`・`ukadoc:manual_ghost` は、
  それぞれバルーン・全体・ゴーストのフォルダに置くファイルを並べるだけのページで、備考も
  「担当が決まるのは…個々の欄であって、ページ全体ではない」と書いている。ページ全体が欠けた
  ときに壊れる振る舞いを 1 つに書けないので、束を名付けず単独項目に落とす（要件 4.4）。理由は
  タスク 3.6 の単独項目の表にも同じ言葉で書く。

上の表の「由来する機械の束 id」は、タスク 3.2 以降で名前付き束の `machine` 欄へそのまま写す。
1 つの機械の束を複数の名前付き束が引用してよい（設計 D-7）。単独項目に落とす 3 件は束では
ないので `machine` 欄を持たず、`reason` に落とした理由を書く。

### 過剰だった関連

分けたあとに両端が別の行き先へ落ちる関連を「過剰」と呼ぶ。ページ id と項目 id を結ぶ 20 本は
両端が同じ行き先へ落ちるので過剰は **0 本**である（項目の側の行き先をページの側に合わせて
決めたので、この 0 は定義から必ずそうなる）。ページ id どうしの 26 本のうち **19 本**が過剰で、
残る 7 本は同じ行き先の中で閉じる。

| 種別 | 書いた行（起点） | 相手（終点） | 分割後の行き先（起点 → 終点） |
| --- | --- | --- | --- |
| `same-feature` | `ukadoc:dev_nar` | `ukadoc:manual_directory` | 開発者機能 → 単独項目 |
| `same-feature` | `ukadoc:dev_nar` | `ukadoc:manual_ghost` | 開発者機能 → 単独項目 |
| `same-feature` | `ukadoc:dev_nar` | `ukadoc:manual_install` | 開発者機能 → インストール |
| `same-feature` | `ukadoc:dev_nar` | `ukadoc:manual_shell` | 開発者機能 → 切替 |
| `same-feature` | `ukadoc:dev_ownerdraw` | `ukadoc:dev_bind` | メニュー → 切替 |
| `same-feature` | `ukadoc:dev_shell` | `ukadoc:dev_ownerdraw` | 切替 → メニュー |
| `same-feature` | `ukadoc:dev_shell` | `ukadoc:dev_update` | 切替 → 更新 |
| `same-feature` | `ukadoc:manual_directory` | `ukadoc:manual_balloon` | 単独項目 → 単独項目 |
| `same-feature` | `ukadoc:manual_directory` | `ukadoc:manual_ghost` | 単独項目 → 単独項目 |
| `same-feature` | `ukadoc:manual_directory` | `ukadoc:manual_owner_draw_menu` | 単独項目 → メニュー |
| `same-feature` | `ukadoc:manual_directory` | `ukadoc:manual_shell` | 単独項目 → 切替 |
| `same-feature` | `ukadoc:manual_directory` | `ukadoc:manual_translator` | 単独項目 → トランスレータ |
| `same-feature` | `ukadoc:manual_ghost` | `ukadoc:manual_shell` | 単独項目 → 切替 |
| `same-feature` | `ukadoc:manual_ghost` | `ukadoc:manual_translator` | 単独項目 → トランスレータ |
| `same-feature` | `ukadoc:manual_ghost` | `ukadoc:manual_update` | 単独項目 → 更新 |
| `same-feature` | `ukadoc:manual_install` | `ukadoc:dev_nar` | インストール → 開発者機能 |
| `same-feature` | `ukadoc:manual_shell` | `ukadoc:manual_ghost` | 切替 → 単独項目 |
| `same-feature` | `ukadoc:manual_shell` | `ukadoc:manual_owner_draw_menu` | 切替 → メニュー |
| `same-feature` | `ukadoc:manual_shell` | `ukadoc:manual_translator` | 切替 → トランスレータ |

19 行のうち 2 対——`ukadoc:dev_nar` と `ukadoc:manual_install`、`ukadoc:manual_ghost` と
`ukadoc:manual_shell`——は同じ対が両向きに書かれている。正典の本文で両方のページが相手を
名指ししており、備考の規則が「指している側に置く」なので 2 行になる。向きの無い繋がりとして
数えると **17 対**である。

**確かめ方**: この 19 本だけを外して 53 件の連結成分を数え直すと、9 つに割れて上の行き先と
ちょうど一致する——更新 30 件・インストール 9 件・開発者機能 4 件・切替 3 件・メニュー 2 件・
トランスレータ 2 件・単独項目 3 件（各 1 件）。合計 53 件。40 件の側の最終の帰属はタスク 3.2〜3.6 が
名前付き束の構成 id として書く。

台帳の `links` は 1 本も削っていない（要件 3.3）。分割はこの文書の上だけで行うので、機械の束 id は
段 1 の値のまま動かず、報告からの引用が作り直しで外れることもない。

**「インストール」の束の核は、段 1 で sakura-script 台帳へ足した `triggers` 2 本である。** 足す前、
`ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1` の関連は
1 本だけで、その相手はページ id `ukadoc:manual_install` だった（ページ id を全部外すと 1 件で孤立
した）。足した後は、ページ id を全部外しても `ukadoc:list_shiori_event:OnInstallComplete:1`・
`ukadoc:list_plugin_event:OnInstallComplete:1`・`ukadoc:list_shiori_event:OnURLQuery:1` と上記 2 つの
タグの 5 件が 1 つに繋がる。過剰な `same-feature` に頼らない機能の繋がりが、ここで置き換わった。

**この節の数の保ち方**: 53・13・46・26・20・19・17 はいずれも履歴の数ではなく、今の台帳と報告から
上の手順でいつでも数え直せる。13 件の行き先は id 単位でタスク 3.8 の判定（構成 id が互いに素・
和集合が対象の全項目と過不足なく一致・機械由来の id が引用した機械の束の構成 id に含まれる）が
見張るので、行き先が抜けても 2 つの束に入っても赤になる。過剰な関連の 19 行そのものを数え直す
判定は、本タスクの境界がこの文書だけなので置いていない（置くならタスク 3.8 の持ち場である）。

## 名前付き束

帰属を決めているのはこの節と次の節の `members` である（冒頭の「この文書は…正本である」と
「書き方の規律」がその約束で、id の囲み方もそこに書いてある）。

段 2 のこのタスク（3.2）が書くのは段階 A 相当の 7 束と段階 B 相当の 4 束である。段階 C・D・E 相当の
束はタスク 3.3・3.4 が、関連を 1 本も持たない項目の残りはタスク 3.5 が、単独項目と合計は
タスク 3.6 がこの下に続けて書く。

各束の囲みは 8 つの事柄のうち ⑴ 名前（表の鍵）・⑵ `machine`・⑶ `members` と人手の印 `hand`・
⑷ `domains`・⑸ `foundation` の見出し・⑺ `breakage`・⑻ `themes` を持ち、⑸ の中身と ⑹ は
囲みの直下の本文に書く。`machine` が空配列の束は「人手のみ」で、核になる機械の束を持たない。

このタスクで名付けた束は **11**、構成 id は延べ **295** 件（機械の束から来たもの **108** 件・人手で足したもの **187** 件）で、同じ id が 2 つの束に
現れることは **0 件**である（数え方: 下の 11 の囲みの `members` を全部集めて重複を数えた）。
状態が `alias` の id と `not-applicable` の id は **0 件**である（同じ集合を台帳の `status` で
引き直して数えた。除外の件数と理由はタスク 3.6 の合計の節に書く）。

### 起動と挨拶（段階 A 相当）

```toml
[bundle."起動と挨拶"]
machine = [
  "ukadoc:list_sakura_script:_5c_21_5bupdate_2cplatform_5d:1",
]
members = [
  "ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:craftmanurl_2cURL:1",
  "ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:id_2cID_540d:1",
  "ukadoc:descript_ghost:shiori.cache_2c_6570_5024:1",
  "ukadoc:descript_ghost:shiori.encoding_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:shiori.escape_unknown_2c0_2f1:1",
  "ukadoc:descript_ghost:shiori.forceencoding_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:shiori.version_2c_30d0_30fc_30b8_30e7_30f3:1",
  "ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:title_2c_8868_793a_540d:1",
  "ukadoc:descript_ghost:type_2c_7a2e_5225:1",
  "ukadoc:list_shiori_event:OnBoot:1",
  "ukadoc:list_shiori_event:OnFirstBoot:1",
  "ukadoc:list_shiori_event:OnInitialize:1",
]
hand = [
  "ukadoc:descript_ghost:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:craftmanurl_2cURL:1",
  "ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_ghost:id_2cID_540d:1",
  "ukadoc:descript_ghost:shiori.cache_2c_6570_5024:1",
  "ukadoc:descript_ghost:shiori.encoding_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:shiori.escape_unknown_2c0_2f1:1",
  "ukadoc:descript_ghost:shiori.forceencoding_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:shiori.version_2c_30d0_30fc_30b8_30e7_30f3:1",
  "ukadoc:descript_ghost:shiori_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:title_2c_8868_793a_540d:1",
  "ukadoc:descript_ghost:type_2c_7a2e_5225:1",
  "ukadoc:list_shiori_event:OnFirstBoot:1",
  "ukadoc:list_shiori_event:OnInitialize:1",
]
domains = ["assets", "shiori"]
foundation = "SHIORI の読み込みと起動時イベントの発火路"
breakage = "黙って壊れる"
themes = ["気配", "記憶"]
```

**成立に要る最小の基盤**: descript.txt を読んでゴーストを組み立て、`shiori` の欄が指す SHIORI を読み込んで `OnInitialize`・`OnFirstBoot`・`OnBoot` をこの順に送り、返ってきたさくらスクリプトを再生できること。

**欠けると壊れる既存ゴーストの振る舞い**: 初めて入れたゴーストが最初の挨拶をしない。2 回目以降の起動でも黙ったまま立っているだけになり、作者名や表示名を読む欄が無いので、ゴースト一覧にも正しい名前が出ない。

構成 id は 16 件で、うち機械の束から来たものが 1 件、人手で足したものが 15 件である（`hand` の行を数えた）。

### 会話（段階 A 相当）

```toml
[bundle."会話"]
machine = [
  "ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1",
]
members = [
  "ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1",
  "ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1",
  "ukadoc:list_sakura_script:_5cC:1",
  "ukadoc:list_sakura_script:_5c_21_5bquicksection_2cfalse_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bquicksection_2ctrue_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cdisable_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cenable_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c__w_5b_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_a_5bID_2cr2_2cr3..._5d:1",
  "ukadoc:list_sakura_script:_5c_q:1",
  "ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1",
  "ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5cc:1",
  "ukadoc:list_sakura_script:_5ce:1",
  "ukadoc:list_sakura_script:_5cn:1",
  "ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1",
  "ukadoc:list_sakura_script:_5cn_5bhalf_5d:1",
  "ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID1_2cID2_2cID3..._5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID_2cr2_2cr3..._5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cOnID_2cr0_2cr1_2c..._5d:1",
  "ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1",
  "ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5ct:1",
  "ukadoc:list_sakura_script:_5cw_6642_9593:1",
  "ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1",
  "ukadoc:list_shiori_event:OnAnchorEnter:1",
  "ukadoc:list_shiori_event:OnAnchorHover:1",
  "ukadoc:list_shiori_event:OnAnchorSelect:1",
  "ukadoc:list_shiori_event:OnAnchorSelectEx:1",
  "ukadoc:list_shiori_event:OnChoiceEnter:1",
  "ukadoc:list_shiori_event:OnChoiceHover:1",
  "ukadoc:list_shiori_event:OnChoiceSelect:1",
  "ukadoc:list_shiori_event:OnChoiceSelectEx:1",
  "ukadoc:list_shiori_event:OnChoiceTimeout:1",
  "ukadoc:list_shiori_resource:balloon_tooltip:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1",
  "ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1",
  "ukadoc:list_sakura_script:_5cC:1",
  "ukadoc:list_sakura_script:_5c_21_5bquicksection_2cfalse_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bquicksection_2ctrue_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cdisable_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cautoscroll_2cenable_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c__w_5b_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_q:1",
  "ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1",
  "ukadoc:list_sakura_script:_5c_w_5b_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5cc:1",
  "ukadoc:list_sakura_script:_5ce:1",
  "ukadoc:list_sakura_script:_5cn:1",
  "ukadoc:list_sakura_script:_5cn_5b_30d1_30fc_30bb_30f3_30c8_5d:1",
  "ukadoc:list_sakura_script:_5cn_5bhalf_5d:1",
  "ukadoc:list_sakura_script:_5cp_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5cs_5bID_756a_53f7_5d:1",
  "ukadoc:list_sakura_script:_5ct:1",
  "ukadoc:list_sakura_script:_5cw_6642_9593:1",
  "ukadoc:list_sakura_script:_5cx_5bnoclear_5d:1",
  "ukadoc:list_shiori_event:OnAnchorEnter:1",
  "ukadoc:list_shiori_event:OnAnchorHover:1",
  "ukadoc:list_shiori_event:OnChoiceEnter:1",
  "ukadoc:list_shiori_event:OnChoiceHover:1",
  "ukadoc:list_shiori_resource:balloon_tooltip:1",
]
domains = ["sakura-script", "shiori"]
foundation = "さくらスクリプトの解釈とバルーンへの文字送り"
breakage = "黙って壊れる"
themes = ["気配", "掛け合い", "交わり"]
```

**成立に要る最小の基盤**: さくらスクリプトを字句に分け、スコープの切り替え・サーフェスの指定・改行・待ち・終端をバルーンへ順に反映し、選択肢とアンカーを押した結果を SHIORI へ返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 台詞が最後まで流れて止まらず、クリックで送る間が無くなる。選択肢が出ないので、分岐する会話はどの枝にも進まない。二人が交互に喋る掛け合いも、片方のバルーンにまとめて出る。

構成 id は 39 件で、うち機械の束から来たものが 11 件、人手で足したものが 28 件である（`hand` の行を数えた）。

### 撫で（段階 A 相当）

```toml
[bundle."撫で"]
machine = [
  "ukadoc:list_shiori_event:OnMouseClick:1",
]
members = [
  "ukadoc:descript_shell_surfaces:animation_2a.collision_2a_2c_5f53_305f_308a_5224_5b9a_5b9a_7fa9animation_2a.collisionex_2a_2c_5f53_305f_308a_5224_5b9a_5:1",
  "ukadoc:descript_shell_surfaces:collision-sort_2c_30bd_30fc_30c8_9806_5e8f:1",
  "ukadoc:descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1",
  "ukadoc:descript_shell_surfaces:collisionex_2a_2cID_2c_30bf_30a4_30d7_2c_5ea7_6a191_2c_5ea7_6a192...:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2ccollisionmode_5d_5c_21_5benter_2ccollisionmode_2crect_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2ccollisionmode_5d:1",
  "ukadoc:list_shiori_event:OnMouseClick:1",
  "ukadoc:list_shiori_event:OnMouseClickEx:1",
  "ukadoc:list_shiori_event:OnMouseDoubleClick:1",
  "ukadoc:list_shiori_event:OnMouseDoubleClickEx:1",
  "ukadoc:list_shiori_event:OnMouseDown:1",
  "ukadoc:list_shiori_event:OnMouseDownEx:1",
  "ukadoc:list_shiori_event:OnMouseDragEnd:1",
  "ukadoc:list_shiori_event:OnMouseDragStart:1",
  "ukadoc:list_shiori_event:OnMouseEnter:1",
  "ukadoc:list_shiori_event:OnMouseEnterAll:1",
  "ukadoc:list_shiori_event:OnMouseGesture:1",
  "ukadoc:list_shiori_event:OnMouseHover:1",
  "ukadoc:list_shiori_event:OnMouseLeave:1",
  "ukadoc:list_shiori_event:OnMouseLeaveAll:1",
  "ukadoc:list_shiori_event:OnMouseMove:1",
  "ukadoc:list_shiori_event:OnMouseMultipleClick:1",
  "ukadoc:list_shiori_event:OnMouseMultipleClickEx:1",
  "ukadoc:list_shiori_event:OnMouseUp:1",
  "ukadoc:list_shiori_event:OnMouseUpEx:1",
  "ukadoc:list_shiori_event:OnMouseWheel:1",
  "ukadoc:list_shiori_resource:tooltip:1",
]
hand = [
  "ukadoc:descript_shell_surfaces:animation_2a.collision_2a_2c_5f53_305f_308a_5224_5b9a_5b9a_7fa9animation_2a.collisionex_2a_2c_5f53_305f_308a_5224_5b9a_5:1",
  "ukadoc:descript_shell_surfaces:collision-sort_2c_30bd_30fc_30c8_9806_5e8f:1",
  "ukadoc:descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1",
  "ukadoc:descript_shell_surfaces:collisionex_2a_2cID_2c_30bf_30a4_30d7_2c_5ea7_6a191_2c_5ea7_6a192...:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2ccollisionmode_5d_5c_21_5benter_2ccollisionmode_2crect_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2ccollisionmode_5d:1",
  "ukadoc:list_shiori_event:OnMouseDoubleClick:1",
  "ukadoc:list_shiori_event:OnMouseDoubleClickEx:1",
  "ukadoc:list_shiori_event:OnMouseDown:1",
  "ukadoc:list_shiori_event:OnMouseDownEx:1",
  "ukadoc:list_shiori_event:OnMouseDragEnd:1",
  "ukadoc:list_shiori_event:OnMouseDragStart:1",
  "ukadoc:list_shiori_event:OnMouseEnter:1",
  "ukadoc:list_shiori_event:OnMouseEnterAll:1",
  "ukadoc:list_shiori_event:OnMouseGesture:1",
  "ukadoc:list_shiori_event:OnMouseHover:1",
  "ukadoc:list_shiori_event:OnMouseLeave:1",
  "ukadoc:list_shiori_event:OnMouseLeaveAll:1",
  "ukadoc:list_shiori_event:OnMouseMove:1",
  "ukadoc:list_shiori_event:OnMouseMultipleClick:1",
  "ukadoc:list_shiori_event:OnMouseMultipleClickEx:1",
  "ukadoc:list_shiori_event:OnMouseUp:1",
  "ukadoc:list_shiori_event:OnMouseUpEx:1",
  "ukadoc:list_shiori_event:OnMouseWheel:1",
  "ukadoc:list_shiori_resource:tooltip:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "当たり判定の解決とマウス入力の配送"
breakage = "黙って壊れる"
themes = ["触れ合い"]
```

**成立に要る最小の基盤**: surfaces.txt の当たり判定を面ごとに解決し、マウスの座標をその名前へ写して `OnMouseMove`・`OnMouseClick` 系のイベントとして SHIORI へ送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 頭を撫でても顔を触っても何も起こらない。ゴーストは立っているだけで、触れ合いを入口にした反応がすべて出ない。

構成 id は 27 件で、うち機械の束から来たものが 2 件、人手で足したものが 25 件である（`hand` の行を数えた）。

### メニュー（段階 A 相当）

```toml
[bundle."メニュー"]
machine = [
  "ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1",
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.type:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.visible:1",
]
members = [
  "ukadoc:descript_ghost:menu.font.height_2c_30d5_30a9_30f3_30c8_30b5_30a4_30ba:1",
  "ukadoc:descript_ghost:menu.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1",
  "ukadoc:descript_shell:char_2a.menuitem_2a_2cID:1",
  "ukadoc:descript_shell:char_2a.menuitemex_2a_2c_30e1_30cb_30e5_30fc_540d_2cID:1",
  "ukadoc:descript_shell:kero.menu_2cauto_307e_305f_306fhidden:1",
  "ukadoc:descript_shell:kero.menuitem_2a_2cID:1",
  "ukadoc:descript_shell:kero.menuitemex_2a_2c_30e1_30cb_30e5_30fc_540d_2cID:1",
  "ukadoc:descript_shell:menu.background.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.background.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.background.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.disable.font.color.b:1",
  "ukadoc:descript_shell:menu.disable.font.color.g:1",
  "ukadoc:descript_shell:menu.disable.font.color.r:1",
  "ukadoc:descript_shell:menu.font.height_2c_30d5_30a9_30f3_30c8_30b5_30a4_30ba:1",
  "ukadoc:descript_shell:menu.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_shell:menu.foreground.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.foreground.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.foreground.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.sidebar.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.sidebar.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu_2chidden:1",
  "ukadoc:descript_shell:sakura.menu_2cauto_307e_305f_306fhidden:1",
  "ukadoc:descript_shell:sakura.menuitem_2a_2cID:1",
  "ukadoc:descript_shell:sakura.menuitemex_2a_2c_30e1_30cb_30e5_30fc_540d_2cID:1",
  "ukadoc:dev_ownerdraw",
  "ukadoc:list_plugin_event:OnMenuExec:1",
  "ukadoc:list_propertysystem:char_2a.bind.menu:1",
  "ukadoc:list_propertysystem:kero.bind.menu:1",
  "ukadoc:list_propertysystem:menu:1",
  "ukadoc:list_propertysystem:sakura.bind.menu:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.type:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.visible:1",
  "ukadoc:list_shiori_resource:kero.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:kero.popupmenu.type:1",
  "ukadoc:list_shiori_resource:kero.popupmenu.visible:1",
  "ukadoc:list_shiori_resource:menu.background.bitmap.filename:1",
  "ukadoc:list_shiori_resource:menu.background.font.color.b:1",
  "ukadoc:list_shiori_resource:menu.background.font.color.g:1",
  "ukadoc:list_shiori_resource:menu.background.font.color.r:1",
  "ukadoc:list_shiori_resource:menu.disable.font.color.b:1",
  "ukadoc:list_shiori_resource:menu.disable.font.color.g:1",
  "ukadoc:list_shiori_resource:menu.disable.font.color.r:1",
  "ukadoc:list_shiori_resource:menu.foreground.bitmap.filename:1",
  "ukadoc:list_shiori_resource:menu.foreground.font.color.b:1",
  "ukadoc:list_shiori_resource:menu.foreground.font.color.g:1",
  "ukadoc:list_shiori_resource:menu.foreground.font.color.r:1",
  "ukadoc:list_shiori_resource:menu.frame.color.b:1",
  "ukadoc:list_shiori_resource:menu.frame.color.g:1",
  "ukadoc:list_shiori_resource:menu.frame.color.r:1",
  "ukadoc:list_shiori_resource:menu.separator.color.b:1",
  "ukadoc:list_shiori_resource:menu.separator.color.g:1",
  "ukadoc:list_shiori_resource:menu.separator.color.r:1",
  "ukadoc:list_shiori_resource:menu.sidebar.bitmap.filename:1",
  "ukadoc:list_shiori_resource:sakura.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:sakura.popupmenu.type:1",
  "ukadoc:list_shiori_resource:sakura.popupmenu.visible:1",
  "ukadoc:manual_owner_draw_menu",
]
hand = [
  "ukadoc:descript_ghost:menu.font.height_2c_30d5_30a9_30f3_30c8_30b5_30a4_30ba:1",
  "ukadoc:descript_ghost:menu.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_shell:char_2a.menuitem_2a_2cID:1",
  "ukadoc:descript_shell:char_2a.menuitemex_2a_2c_30e1_30cb_30e5_30fc_540d_2cID:1",
  "ukadoc:descript_shell:kero.menuitem_2a_2cID:1",
  "ukadoc:descript_shell:kero.menuitemex_2a_2c_30e1_30cb_30e5_30fc_540d_2cID:1",
  "ukadoc:descript_shell:menu.background.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.background.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.background.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.background.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.disable.font.color.b:1",
  "ukadoc:descript_shell:menu.disable.font.color.g:1",
  "ukadoc:descript_shell:menu.disable.font.color.r:1",
  "ukadoc:descript_shell:menu.font.height_2c_30d5_30a9_30f3_30c8_30b5_30a4_30ba:1",
  "ukadoc:descript_shell:menu.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_shell:menu.foreground.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.foreground.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:menu.foreground.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.foreground.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.separator.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:menu.sidebar.alignment_2c_4f4d_7f6e:1",
  "ukadoc:descript_shell:menu.sidebar.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:sakura.menuitem_2a_2cID:1",
  "ukadoc:descript_shell:sakura.menuitemex_2a_2c_30e1_30cb_30e5_30fc_540d_2cID:1",
  "ukadoc:list_plugin_event:OnMenuExec:1",
  "ukadoc:list_shiori_resource:menu.background.bitmap.filename:1",
  "ukadoc:list_shiori_resource:menu.background.font.color.b:1",
  "ukadoc:list_shiori_resource:menu.background.font.color.g:1",
  "ukadoc:list_shiori_resource:menu.background.font.color.r:1",
  "ukadoc:list_shiori_resource:menu.disable.font.color.b:1",
  "ukadoc:list_shiori_resource:menu.disable.font.color.g:1",
  "ukadoc:list_shiori_resource:menu.disable.font.color.r:1",
  "ukadoc:list_shiori_resource:menu.foreground.bitmap.filename:1",
  "ukadoc:list_shiori_resource:menu.foreground.font.color.b:1",
  "ukadoc:list_shiori_resource:menu.foreground.font.color.g:1",
  "ukadoc:list_shiori_resource:menu.foreground.font.color.r:1",
  "ukadoc:list_shiori_resource:menu.frame.color.b:1",
  "ukadoc:list_shiori_resource:menu.frame.color.g:1",
  "ukadoc:list_shiori_resource:menu.frame.color.r:1",
  "ukadoc:list_shiori_resource:menu.separator.color.b:1",
  "ukadoc:list_shiori_resource:menu.separator.color.g:1",
  "ukadoc:list_shiori_resource:menu.separator.color.r:1",
  "ukadoc:list_shiori_resource:menu.sidebar.bitmap.filename:1",
]
domains = ["assets", "property", "shiori"]
foundation = "メニューの組み立てと自前描画"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: 右クリックでメニューを組み立て、descript.txt と SHIORI の資源が指定した項目・配色・背景画像で自前描画し、選んだ項目を実行できること。

**欠けると壊れる既存ゴーストの振る舞い**: 右クリックしても何も出ない。着せ替えの切り替え・シェルの選択・ゴーストの入れ替え・終了はすべてメニューが入口なので、利用者はゴーストを操作する手段を持たない。

構成 id は 69 件で、うち機械の束から来たものが 19 件、人手で足したものが 50 件である（`hand` の行を数えた）。

### 終了（段階 A 相当）

```toml
[bundle."終了"]
machine = [
  "ukadoc:list_sakura_script:_5c_21_5bupdate_2cplatform_5d:1",
]
members = [
  "ukadoc:list_sakura_script:_5c-:1",
  "ukadoc:list_shiori_event:OnClose:1",
  "ukadoc:list_shiori_event:OnCloseAll:1",
  "ukadoc:list_shiori_event:OnDestroy:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c-:1",
  "ukadoc:list_shiori_event:OnCloseAll:1",
  "ukadoc:list_shiori_event:OnDestroy:1",
]
domains = ["sakura-script", "shiori"]
foundation = "終了要求の受理と最後の台詞の再生"
breakage = "黙って壊れる"
themes = ["気配", "記憶"]
```

**成立に要る最小の基盤**: 終了要求を受けて `OnClose` を送り、返ってきた別れの台詞を再生し終えてからプロセスを畳めること。

**欠けると壊れる既存ゴーストの振る舞い**: 別れの挨拶をせずに窓が消える。複数のゴーストを立てているときも、全体の終了に合わせた台詞が 1 つも出ない。

構成 id は 4 件で、うち機械の束から来たものが 1 件、人手で足したものが 3 件である（`hand` の行を数えた）。

### 自発発話（段階 A 相当）

```toml
[bundle."自発発話"]
machine = [
  "ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1",
]
members = [
  "ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1",
  "ukadoc:list_plugin_event:OnSecondChange:1",
  "ukadoc:list_shiori_event:OnAITalk:1",
  "ukadoc:list_shiori_event:OnHourTimeSignal:1",
  "ukadoc:list_shiori_event:OnMinuteChange:1",
  "ukadoc:list_shiori_event:OnSecondChange:1",
  "ukadoc:list_shiori_resource:getaistate:1",
  "ukadoc:list_shiori_resource:getaistateex:1",
]
hand = [
  "ukadoc:list_shiori_event:OnAITalk:1",
  "ukadoc:list_shiori_event:OnHourTimeSignal:1",
  "ukadoc:list_shiori_event:OnMinuteChange:1",
  "ukadoc:list_shiori_resource:getaistate:1",
  "ukadoc:list_shiori_resource:getaistateex:1",
]
domains = ["assets", "shiori"]
foundation = "絶対時刻の刻みを台本の起点にする発火路"
breakage = "黙って壊れる"
themes = ["気配", "記憶"]
```

**成立に要る最小の基盤**: 起動からの絶対時刻を刻み、秒・分・正時の境界で `OnSecondChange`・`OnMinuteChange`・`OnHourTimeSignal` を送り、`OnAITalk` の間隔を資源の値で決められること。

**欠けると壊れる既存ゴーストの振る舞い**: 話しかけない限りゴーストが一言も喋らない。時報も鳴らず、机の隅で勝手に喋っているという伺かの基本の姿にならない。

構成 id は 8 件で、うち機械の束から来たものが 3 件、人手で足したものが 5 件である（`hand` の行を数えた）。

### 名前の記憶（段階 A 相当）

```toml
[bundle."名前の記憶"]
machine = [
  "ukadoc:descript_ghost:char_2a.name_2c_540d_524d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cteachbox_5d:1",
]
members = [
  "ukadoc:descript_ghost:char_2a.name_2c_540d_524d:1",
  "ukadoc:descript_ghost:kero.name_2c_540d_524d:1",
  "ukadoc:descript_ghost:name.allowoverride_2c_6570_5024:1",
  "ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1",
  "ukadoc:descript_ghost:sakura.name2_2c_540d_524d:1",
  "ukadoc:descript_ghost:sakura.name_2c_540d_524d:1",
  "ukadoc:descript_shell:char_2a.name_2c_540d_524d:1",
  "ukadoc:descript_shell:kero.name_2c_540d_524d:1",
  "ukadoc:descript_shell:sakura.name_2c_540d_524d:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.name:1",
  "ukadoc:list_sakura_script:_25keroname:1",
  "ukadoc:list_sakura_script:_25selfname2:1",
  "ukadoc:list_sakura_script:_25selfname:1",
  "ukadoc:list_sakura_script:_25username:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cteachbox_5d:1",
  "ukadoc:list_shiori_event:OnNotifyUserInfo:1",
  "ukadoc:list_shiori_event:OnTeach:1",
  "ukadoc:list_shiori_event:OnTeachInputCancel:1",
  "ukadoc:list_shiori_event:OnTeachStart:1",
  "ukadoc:list_shiori_event:installedkeroname:1",
  "ukadoc:list_shiori_event:installedsakuraname:1",
  "ukadoc:list_shiori_resource:username:1",
]
hand = [
  "ukadoc:descript_ghost:name.allowoverride_2c_6570_5024:1",
  "ukadoc:descript_ghost:sakura.name2_2c_540d_524d:1",
  "ukadoc:list_sakura_script:_25keroname:1",
  "ukadoc:list_sakura_script:_25selfname2:1",
  "ukadoc:list_sakura_script:_25selfname:1",
  "ukadoc:list_sakura_script:_25username:1",
  "ukadoc:list_shiori_event:OnNotifyUserInfo:1",
  "ukadoc:list_shiori_event:installedkeroname:1",
  "ukadoc:list_shiori_event:installedsakuraname:1",
  "ukadoc:list_shiori_resource:username:1",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "ゴーストと利用者の名前を保存して読み戻す口"
breakage = "黙って壊れる"
themes = ["掛け合い", "記憶", "交わり"]
```

**成立に要る最小の基盤**: 本体側と相方の名前を descript.txt と shell の descript.txt から読み、利用者の名前を SHIORI の資源として保存して読み戻せること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴースト自身の名前が出ないので、台詞の中の名前が空欄になる。名前を尋ねる会話をしても答えを覚えないため、次の起動で同じ質問を繰り返す。

構成 id は 22 件で、うち機械の束から来たものが 12 件、人手で足したものが 10 件である（`hand` の行を数えた）。

### 更新（段階 B 相当）

```toml
[bundle."更新"]
machine = [
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_shiori_event:OnUpdate.OnDownloadBegin:1",
  "ukadoc:list_shiori_event:OnUpdateCheckResult:1",
  "ukadoc:list_shiori_event:OnUpdateCheckComplete:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdate_2cplatform_5d:1",
]
members = [
  "ukadoc:descript_ghost:homeurl_2cURL:1",
  "ukadoc:dev_update",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreateupdatedata_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdate_2cplatform_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdateother_2c_66f4_65b0_5bfe_8c61_2f_30aa_30d7_30b7_30e7_30f3_7fa4_2c..._5d:1",
  "ukadoc:list_shiori_event:OnBasewareUpdated:1",
  "ukadoc:list_shiori_event:OnBasewareUpdating:1",
  "ukadoc:list_shiori_event:OnUpdate.OnDownloadBegin:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareBegin:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareComplete:1",
  "ukadoc:list_shiori_event:OnUpdate.OnMD5CompareFailure:1",
  "ukadoc:list_shiori_event:OnUpdateBegin:1",
  "ukadoc:list_shiori_event:OnUpdateCheckComplete:1",
  "ukadoc:list_shiori_event:OnUpdateCheckFailure:1",
  "ukadoc:list_shiori_event:OnUpdateCheckResult:1",
  "ukadoc:list_shiori_event:OnUpdateCheckResultEx:1",
  "ukadoc:list_shiori_event:OnUpdateComplete:1",
  "ukadoc:list_shiori_event:OnUpdateFailure:1",
  "ukadoc:list_shiori_event:OnUpdateOther.OnDownloadBegin:1",
  "ukadoc:list_shiori_event:OnUpdateOther.OnMD5CompareBegin:1",
  "ukadoc:list_shiori_event:OnUpdateOther.OnMD5CompareComplete:1",
  "ukadoc:list_shiori_event:OnUpdateOther.OnMD5CompareFailure:1",
  "ukadoc:list_shiori_event:OnUpdateOtherBegin:1",
  "ukadoc:list_shiori_event:OnUpdateOtherComplete:1",
  "ukadoc:list_shiori_event:OnUpdateOtherFailure:1",
  "ukadoc:list_shiori_event:OnUpdateOtherReady:1",
  "ukadoc:list_shiori_event:OnUpdateProcessExec:1",
  "ukadoc:list_shiori_event:OnUpdateReady:1",
  "ukadoc:list_shiori_event:OnUpdateResult:1",
  "ukadoc:list_shiori_event:OnUpdateResultEx:1",
  "ukadoc:list_shiori_event:OnUpdateResultExplorer:1",
  "ukadoc:list_shiori_event:OnUpdatedataCreated:1",
  "ukadoc:list_shiori_event:OnUpdatedataCreating:1",
  "ukadoc:list_shiori_resource:homeurl:1",
  "ukadoc:list_shiori_resource:other_homeurl_override:1",
  "ukadoc:list_shiori_resource:useorigin1:1",
  "ukadoc:manual_update",
]
hand = [
  "ukadoc:descript_ghost:homeurl_2cURL:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1",
  "ukadoc:list_shiori_resource:homeurl:1",
  "ukadoc:list_shiori_resource:useorigin1:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "ネットワーク越しの差分取得とファイルの入れ替え"
breakage = "黙って壊れる"
themes = ["更新"]
```

**成立に要る最小の基盤**: homeurl が指すサーバから updates2.dau を取り、md5 を突き合わせて差分だけを取得し、入れ替えの前後で `OnUpdate*` 系のイベントを送れること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴーストが新しい版に上がらない。作者が配信した修正も追加の台詞も届かず、更新の進行を伝える台詞も 1 つも出ない。

構成 id は 39 件で、うち機械の束から来たものが 35 件、人手で足したものが 4 件である（`hand` の行を数えた）。

### インストール（段階 B 相当）

```toml
[bundle."インストール"]
machine = [
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_plugin_event:OnInstallComplete:1",
]
members = [
  "ukadoc:descript_ghost:install.accept_2c_540d_524d1_2c_540d_524d2_2c_540d_524d3...:1",
  "ukadoc:descript_install:_2a.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:_2a.refresh_2c_6570_5024:1",
  "ukadoc:descript_install:_2a.refreshundeletemask_2c_30d5_30a1_30a4_30eb_540d1_3a_30d5_30a1_30a4_30eb_540d2...:1",
  "ukadoc:descript_install:_2a.source.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:_76f8_5bfe_30d1_30b9:1",
  "ukadoc:descript_install:_76f8_5bfe_30d1_30b9_2c_30aa_30d7_30b7_30e7_30f31_2c_30aa_30d7_30b7_30e7_30f32_2c...:1",
  "ukadoc:descript_install:_76f8_5bfe_30d1_30b9_2cignore:1",
  "ukadoc:descript_install:accept_2c_672c_4f53_5074_540d:1",
  "ukadoc:descript_install:bootghost_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_install:directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:name_2c_30aa_30d6_30b8_30a7_30af_30c8_540d:1",
  "ukadoc:descript_install:refresh_2c_6570_5024:1",
  "ukadoc:descript_install:refreshundeletemask_2c_30d5_30a1_30a4_30eb_540d1_3a_30d5_30a1_30a4_30eb_540d2...:1",
  "ukadoc:descript_install:type_2c_7a2e_5225:1",
  "ukadoc:list_plugin_event:OnInstallComplete:1",
  "ukadoc:list_plugin_event:installedballoonname:1",
  "ukadoc:list_plugin_event:installedghostname:1",
  "ukadoc:list_plugin_event:installedplugin:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1",
  "ukadoc:list_shiori_event:OnInstallBegin:1",
  "ukadoc:list_shiori_event:OnInstallComplete:1",
  "ukadoc:list_shiori_event:OnInstallCompleteAll:1",
  "ukadoc:list_shiori_event:OnInstallCompleteEx:1",
  "ukadoc:list_shiori_event:OnInstallFailure:1",
  "ukadoc:list_shiori_event:OnInstallRefuse:1",
  "ukadoc:list_shiori_event:OnInstallReroute:1",
  "ukadoc:list_shiori_event:OnURLQuery:1",
  "ukadoc:list_shiori_event:installedballoonname:1",
  "ukadoc:list_shiori_event:installedghostname:1",
  "ukadoc:list_shiori_event:installedheadlinename:1",
  "ukadoc:list_shiori_event:installedplugin:1",
  "ukadoc:list_shiori_event:installedshellname:1",
  "ukadoc:manual_install",
]
hand = [
  "ukadoc:descript_ghost:install.accept_2c_540d_524d1_2c_540d_524d2_2c_540d_524d3...:1",
  "ukadoc:descript_install:_2a.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:_2a.refresh_2c_6570_5024:1",
  "ukadoc:descript_install:_2a.refreshundeletemask_2c_30d5_30a1_30a4_30eb_540d1_3a_30d5_30a1_30a4_30eb_540d2...:1",
  "ukadoc:descript_install:_2a.source.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:_76f8_5bfe_30d1_30b9:1",
  "ukadoc:descript_install:_76f8_5bfe_30d1_30b9_2c_30aa_30d7_30b7_30e7_30f31_2c_30aa_30d7_30b7_30e7_30f32_2c...:1",
  "ukadoc:descript_install:_76f8_5bfe_30d1_30b9_2cignore:1",
  "ukadoc:descript_install:accept_2c_672c_4f53_5074_540d:1",
  "ukadoc:descript_install:bootghost_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_install:directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_install:name_2c_30aa_30d6_30b8_30a7_30af_30c8_540d:1",
  "ukadoc:descript_install:refresh_2c_6570_5024:1",
  "ukadoc:descript_install:refreshundeletemask_2c_30d5_30a1_30a4_30eb_540d1_3a_30d5_30a1_30a4_30eb_540d2...:1",
  "ukadoc:descript_install:type_2c_7a2e_5225:1",
  "ukadoc:list_plugin_event:installedballoonname:1",
  "ukadoc:list_plugin_event:installedghostname:1",
  "ukadoc:list_plugin_event:installedplugin:1",
  "ukadoc:list_shiori_event:OnInstallBegin:1",
  "ukadoc:list_shiori_event:OnInstallCompleteEx:1",
  "ukadoc:list_shiori_event:OnInstallFailure:1",
  "ukadoc:list_shiori_event:installedballoonname:1",
  "ukadoc:list_shiori_event:installedghostname:1",
  "ukadoc:list_shiori_event:installedheadlinename:1",
  "ukadoc:list_shiori_event:installedplugin:1",
  "ukadoc:list_shiori_event:installedshellname:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "配布アーカイブの受け取りと所定の場所への展開"
breakage = "黙って壊れる"
themes = ["触れ合い", "装い", "記憶", "更新"]
```

**成立に要る最小の基盤**: nar 書庫を受け取って install.txt の指定どおりに展開し、`OnInstallComplete` 系のイベントで受け入れの結果を伝えられること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴーストやバルーンやシェルを窓へ落としても何も入らない。配布された物を追加する手段が無くなるので、最初に入れた 1 体だけを使い続けることになる。

構成 id は 36 件で、うち機械の束から来たものが 9 件、人手で足したものが 27 件である（`hand` の行を数えた）。

### 切替（段階 B 相当）

```toml
[bundle."切替"]
machine = [
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1",
]
members = [
  "ukadoc:dev_bind",
  "ukadoc:dev_shell",
  "ukadoc:list_sakura_script:_5c_21_5bbind-noevent_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cballoon_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cghost_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cshell_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_2b:1",
  "ukadoc:list_sakura_script:_5c__2b:1",
  "ukadoc:list_shiori_event:OnBalloonChange:1",
  "ukadoc:list_shiori_event:OnBalloonScaling:1",
  "ukadoc:list_shiori_event:OnDressupChanged:1",
  "ukadoc:list_shiori_event:OnGhostChanged:1",
  "ukadoc:list_shiori_event:OnGhostChanging:1",
  "ukadoc:list_shiori_event:OnNotifyBalloonInfo:1",
  "ukadoc:list_shiori_event:OnNotifyDressupInfo:1",
  "ukadoc:list_shiori_event:OnNotifyShellInfo:1",
  "ukadoc:list_shiori_event:OnShellChanged:1",
  "ukadoc:list_shiori_event:OnShellChanging:1",
  "ukadoc:list_shiori_event:OnShellScaling:1",
  "ukadoc:manual_shell",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bbind-noevent_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cballoon_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cghost_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cshell_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnBalloonScaling:1",
  "ukadoc:list_shiori_event:OnGhostChanged:1",
  "ukadoc:list_shiori_event:OnNotifyBalloonInfo:1",
  "ukadoc:list_shiori_event:OnNotifyShellInfo:1",
  "ukadoc:list_shiori_event:OnShellScaling:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "シェル・バルーン・ゴーストの読み直しと差し替え"
breakage = "黙って壊れる"
themes = ["装い", "記憶"]
```

**成立に要る最小の基盤**: shell/master と balloon の別のフォルダを読み直して立ち絵とバルーンを差し替え、着せ替えの重ね合わせを付け外しし、別のゴーストへ入れ替えられること。

**欠けると壊れる既存ゴーストの振る舞い**: シェルを選んでも見た目が変わらない。着せ替えの服も切り替わらず、同じゴーストに複数の姿を用意した作品はどれも 1 つの姿しか見せられない。

構成 id は 26 件で、うち機械の束から来たものが 15 件、人手で足したものが 11 件である（`hand` の行を数えた）。

### 消滅（段階 B 相当）

```toml
[bundle."消滅"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5bvanishbymyself_5d:1",
  "ukadoc:list_shiori_event:OnVanishButtonHold:1",
  "ukadoc:list_shiori_event:OnVanishCancel:1",
  "ukadoc:list_shiori_event:OnVanishSelected:1",
  "ukadoc:list_shiori_event:OnVanishSelecting:1",
  "ukadoc:list_shiori_event:OnVanished:1",
  "ukadoc:list_shiori_resource:vanishbutton.caption:1",
  "ukadoc:list_shiori_resource:vanishbuttoncaption:1",
  "ukadoc:list_shiori_resource:vanishbuttonvisible:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bvanishbymyself_5d:1",
  "ukadoc:list_shiori_event:OnVanishButtonHold:1",
  "ukadoc:list_shiori_event:OnVanishCancel:1",
  "ukadoc:list_shiori_event:OnVanishSelected:1",
  "ukadoc:list_shiori_event:OnVanishSelecting:1",
  "ukadoc:list_shiori_event:OnVanished:1",
  "ukadoc:list_shiori_resource:vanishbutton.caption:1",
  "ukadoc:list_shiori_resource:vanishbuttoncaption:1",
  "ukadoc:list_shiori_resource:vanishbuttonvisible:1",
]
domains = ["sakura-script", "shiori"]
foundation = "ゴーストの削除要求の確認と自分自身の後始末"
breakage = "黙って壊れる"
themes = ["装い", "記憶"]
```

**成立に要る最小の基盤**: 消滅の要求を受けて確認のやりとりを行い、`OnVanishSelected` を送ってからそのゴーストのフォルダを削除して窓を閉じられること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴーストを消す手段が無い。別れの場面を用意した作品はその台詞に到達せず、入れたゴーストは手で削除するほかなくなる。

構成 id は 9 件で、うち機械の束から来たものが 0 件、人手で足したものが 9 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（消滅の 9 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

段 2 のこのタスク（3.3）が書くのは段階 C 相当の 8 束——環境の察知の 7 束と、環境の値を尋ねる
1 束——である。書き方は上の 11 束と同じで、囲みが ⑴ 名前（表の鍵）・⑵ `machine`・⑶ `members` と
人手の印 `hand`・⑷ `domains`・⑸ `foundation` の見出し・⑺ `breakage`・⑻ `themes` を持ち、⑸ の
中身と ⑹ は囲みの直下の本文に書く。

このタスクで名付けた束は **8**、構成 id は延べ **80** 件（機械の束から来たもの **18** 件・
人手で足したもの **62** 件）で、同じ id が 2 つの束に現れることは **0 件**である（数え方: 下の
8 つの囲みの `members` を全部集めて重複を数えた）。上の 11 束が使った id との重なりも **0 件**で
ある（数え方: 上の 11 束と下の 8 束の `members` を集めて共通部分を数えた）。状態が `alias` の id と
`not-applicable` の id は **0 件**である（同じ集合を台帳の `status` で引き直して数えた。除外の
件数と理由はタスク 3.6 の合計の節に書く）。

`OnCacheSuspend` と `OnCacheRestore` はこの 8 束のどれにも入れない。正典の本文はそれぞれ
「ゴーストキャッシュに入った際に発生。」「ゴーストキャッシュから出た際に発生。」だけを述べており、
指しているのはゴーストを入れ替えるときにいったん裏へ回して保つ仕組みであって、パソコンの電源
状態でも画面の状態でも音でもない。段 2 のタスク 3.5（関連を 1 本も持たない項目の残り）へ回す。

### スリープ復帰（段階 C 相当）

```toml
[bundle."スリープ復帰"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnSysResume:1",
  "ukadoc:list_shiori_event:OnSysSuspend:1",
]
hand = [
  "ukadoc:list_shiori_event:OnSysResume:1",
  "ukadoc:list_shiori_event:OnSysSuspend:1",
]
domains = ["shiori"]
foundation = "OS の状態変化の通知を受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: パソコンがサスペンド（スリープと休止状態の両方）に入ったことと解除されたことを OS から受け取り、`OnSysSuspend`・`OnSysResume` を送って、返ったさくらスクリプトを再生できること。解除のときは理由（`normal`・`auto`・`critical`）を Reference0 に添えること。

**欠けると壊れる既存ゴーストの振る舞い**: パソコンを眠らせるときと目覚めさせたときに、ゴーストが何も言わない。眠る前の見送りの一言も、戻ったときの「おかえり」も出ないので、席を外して戻ってきた利用者から見ると、留守の間に時間が流れたことがゴーストに伝わっていない。

構成 id は 2 件で、うち機械の束から来たものが 0 件、人手で足したものが 2 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（スリープ復帰の 2 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### バッテリー（段階 C 相当）

```toml
[bundle."バッテリー"]
machine = [
  "ukadoc:list_shiori_event:OnBatteryLow:1",
  "ukadoc:list_shiori_event:OnBatteryCritical:1",
]
members = [
  "ukadoc:list_shiori_event:OnBatteryChargingStart:1",
  "ukadoc:list_shiori_event:OnBatteryChargingStop:1",
  "ukadoc:list_shiori_event:OnBatteryCritical:1",
  "ukadoc:list_shiori_event:OnBatteryLow:1",
  "ukadoc:list_shiori_event:OnBatteryNotify:1",
]
hand = [
  "ukadoc:list_shiori_event:OnBatteryChargingStart:1",
  "ukadoc:list_shiori_event:OnBatteryChargingStop:1",
  "ukadoc:list_shiori_event:OnBatteryNotify:1",
]
domains = ["shiori"]
foundation = "OS の状態変化の通知を受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: 電源の供給状態（バッテリー駆動・電源供給中・補助電源）と残量と残り時間を OS から読み、起動のときは通知として、以後は問い合わせに答える形で `OnBatteryNotify` を送り、残量が 1/3 以下・5% 以下になった境目と、充電が始まった・止まった境目で `OnBatteryLow`・`OnBatteryCritical`・`OnBatteryChargingStart`・`OnBatteryChargingStop` を送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 電池が減っても、電源を挿しても抜いても、ゴーストは何も言わない。「そろそろ充電して」と促す台詞や、電源を抜いたときに心配する台詞を持つ作品では、その台詞に 1 度も到達しない。ノートパソコンを持ち歩く利用者は、電池切れを自分で見張ることになる。

構成 id は 5 件で、うち機械の束から来たものが 2 件、人手で足したものが 3 件である（`hand` の行を数えた）。

### スクリーンセーバー（段階 C 相当）

```toml
[bundle."スクリーンセーバー"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnDisplayPowerStatus:1",
  "ukadoc:list_shiori_event:OnScreenSaverEnd:1",
  "ukadoc:list_shiori_event:OnScreenSaverStart:1",
]
hand = [
  "ukadoc:list_shiori_event:OnDisplayPowerStatus:1",
  "ukadoc:list_shiori_event:OnScreenSaverEnd:1",
  "ukadoc:list_shiori_event:OnScreenSaverStart:1",
]
domains = ["shiori"]
foundation = "OS の状態変化の通知を受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: スクリーンセーバーが始まったこと・終わったことと、モニタの電源が入ったこと・切れたこと（ノートパソコンの蓋を閉じた場合を含む）を OS から受け取り、`OnScreenSaverStart`・`OnScreenSaverEnd`・`OnDisplayPowerStatus` を送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 画面が消えて誰も見ていない間も、ゴーストはそれまでと同じ調子で独り言を続ける。画面が戻ったときの「おかえり」も出ない。スクリーンセーバー中は静かにする作法を辞書に書いた作品でも、その分岐に入らない。

構成 id は 3 件で、うち機械の束から来たものが 0 件、人手で足したものが 3 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（スクリーンセーバーの 3 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### フルスクリーン退避（段階 C 相当）

```toml
[bundle."フルスクリーン退避"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnFullScreenAppMinimize:1",
  "ukadoc:list_shiori_event:OnFullScreenAppRestore:1",
]
hand = [
  "ukadoc:list_shiori_event:OnFullScreenAppMinimize:1",
  "ukadoc:list_shiori_event:OnFullScreenAppRestore:1",
]
domains = ["shiori"]
foundation = "ゴースト窓の最小化と復帰の状態遷移とその通知"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: 全画面のアプリが前に出たときにゴーストの窓を退避させ、その理由を `fullscreen` として `OnFullScreenAppMinimize` を送り、元に戻したときに `OnFullScreenAppRestore` を送れること。この 2 つは、同じ場面で通常の最小化として出る `OnWindowStateMinimize` を上書きする。

**欠けると壊れる既存ゴーストの振る舞い**: 動画やゲームを全画面にしてもゴーストが画面の前に残り、見たいものの上に立ち塞がる。退避と復帰の一言も無いので、利用者は全画面にするたびに自分でゴーストを引っ込め、終わったら自分で戻す。

構成 id は 2 件で、うち機械の束から来たものが 0 件、人手で足したものが 2 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（フルスクリーン退避の 2 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### 最小化（段階 C 相当）

```toml
[bundle."最小化"]
machine = []
members = [
  "ukadoc:descript_ghost:icon.minimize_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cminimize_5d:1",
  "ukadoc:list_shiori_event:OnWindowStateMinimize:1",
  "ukadoc:list_shiori_event:OnWindowStateRestore:1",
]
hand = [
  "ukadoc:descript_ghost:icon.minimize_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cminimize_5d:1",
  "ukadoc:list_shiori_event:OnWindowStateMinimize:1",
  "ukadoc:list_shiori_event:OnWindowStateRestore:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "ゴースト窓の最小化と復帰の状態遷移とその通知"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: ゴーストの窓を最小化して元に戻す操作を受け付け、最小化の理由（`system`・`script`・`sakuraapi`・`user`）を添えて `OnWindowStateMinimize`・`OnWindowStateRestore` を送り、台詞の中の `\![set,windowstate,minimize]` でも同じ最小化を起こせて、`icon.minimize,ファイル名` が指す絵を最小化中の姿として使えること。

**欠けると壊れる既存ゴーストの振る舞い**: 作業に集中したいときにゴーストを引っ込められない。台詞の中から自分で引っ込む演出を書いた作品でもタグが何も起こさず、引っ込んだ・戻ったの一言も出ない。最小化中の姿の絵を用意した作品でも、その絵は 1 度も画面に出ない。

構成 id は 4 件で、うち機械の束から来たものが 0 件、人手で足したものが 4 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（最小化の 4 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### ディスプレイ変化（段階 C 相当）

```toml
[bundle."ディスプレイ変化"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnDisplayChange:1",
  "ukadoc:list_shiori_event:OnDisplayChangeEx:1",
  "ukadoc:list_shiori_event:OnDisplayHandover:1",
]
hand = [
  "ukadoc:list_shiori_event:OnDisplayChange:1",
  "ukadoc:list_shiori_event:OnDisplayChangeEx:1",
  "ukadoc:list_shiori_event:OnDisplayHandover:1",
]
domains = ["shiori"]
foundation = "OS の状態変化の通知を受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: 主画面の解像度と色深度が変わったこと、画面ごとの設定が変わったこと、窓が別の画面へ移ったことを OS から受け取り、変化の前後の値を添えて `OnDisplayChange`・`OnDisplayChangeEx`・`OnDisplayHandover` を送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 画面の解像度を変えても、ノートパソコンを外部ディスプレイに繋いでも、ゴーストはそれに気づかない。画面が変わったときに立ち位置や台詞を直す辞書を書いた作品ではその分岐に入らず、ゴーストは前の画面のつもりのまま、画面の端や外に取り残される。

構成 id は 3 件で、うち機械の束から来たものが 0 件、人手で足したものが 3 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（ディスプレイ変化の 3 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### サウンド（段階 C 相当）

```toml
[bundle."サウンド"]
machine = [
  "ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cwait_5d:1",
]
members = [
  "ukadoc:list_propertysystem:currentghost.sound.count:1",
  "ukadoc:list_propertysystem:currentghost.sound.index_28ID_29._30b5_30a6_30f3_30c9_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.sound_28_8981_7d20_540d_29._30b5_30a6_30f3_30c9_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:duration:1",
  "ukadoc:list_propertysystem:error:1",
  "ukadoc:list_propertysystem:id:1",
  "ukadoc:list_propertysystem:loop:1",
  "ukadoc:list_propertysystem:meta.album:1",
  "ukadoc:list_propertysystem:meta.albumartist:1",
  "ukadoc:list_propertysystem:meta.artist:1",
  "ukadoc:list_propertysystem:meta.artwork:1",
  "ukadoc:list_propertysystem:meta.genre:1",
  "ukadoc:list_propertysystem:meta.title:1",
  "ukadoc:list_propertysystem:meta.track:1",
  "ukadoc:list_propertysystem:meta.year:1",
  "ukadoc:list_propertysystem:name:2",
  "ukadoc:list_propertysystem:path:2",
  "ukadoc:list_propertysystem:pause:1",
  "ukadoc:list_propertysystem:playing:1",
  "ukadoc:list_propertysystem:position:1",
  "ukadoc:list_propertysystem:preload:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2ccdplay_2c_30c8_30e9_30c3_30afNo._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cload_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cloop_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2coption_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cpause_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cplay_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cresume_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cstop_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cwait_5d:1",
  "ukadoc:list_shiori_event:OnSoundError:1",
  "ukadoc:list_shiori_event:OnSoundLoop:1",
  "ukadoc:list_shiori_event:OnSoundStop:1",
]
hand = [
  "ukadoc:list_propertysystem:currentghost.sound.index_28ID_29._30b5_30a6_30f3_30c9_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:duration:1",
  "ukadoc:list_propertysystem:error:1",
  "ukadoc:list_propertysystem:id:1",
  "ukadoc:list_propertysystem:meta.album:1",
  "ukadoc:list_propertysystem:meta.albumartist:1",
  "ukadoc:list_propertysystem:meta.artist:1",
  "ukadoc:list_propertysystem:meta.artwork:1",
  "ukadoc:list_propertysystem:meta.genre:1",
  "ukadoc:list_propertysystem:meta.title:1",
  "ukadoc:list_propertysystem:meta.track:1",
  "ukadoc:list_propertysystem:meta.year:1",
  "ukadoc:list_propertysystem:path:2",
  "ukadoc:list_propertysystem:preload:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2ccdplay_2c_30c8_30e9_30c3_30afNo._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsound_2cload_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_shiori_event:OnSoundError:1",
  "ukadoc:list_shiori_event:OnSoundLoop:1",
  "ukadoc:list_shiori_event:OnSoundStop:1",
]
domains = ["property", "sakura-script", "shiori"]
foundation = "音声ファイルの再生器と再生状態の通知"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 台詞の中から音声ファイルを読み込んで、再生・停止・一時停止・再開・繰り返し・音の終わりの待ち合わせを指示でき、再生が終わったこと・繰り返したこと・失敗したことを `OnSoundStop`・`OnSoundLoop`・`OnSoundError` で返し、鳴っている音の名前・場所・長さ・位置・曲の題や演者をプロパティで読み戻せること。

**欠けると壊れる既存ゴーストの振る舞い**: 効果音も音楽も鳴らない。足音や鐘の音を台詞に合わせて鳴らす作品では、音の部分だけが抜け落ちる。`\![sound,wait]` で音の終わりを待つ台本は待ち合わせが効かず、音が鳴らないまま台詞だけが先へ進んで間合いが崩れる。鳴っている曲の題を台詞に差し込む作品では、その場所が空のまま読み上げられる。

構成 id は 33 件で、うち機械の束から来たものが 14 件、人手で足したものが 19 件である（`hand` の行を数えた）。テーマは **0 件**である（構成 id 33 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 環境の照会（段階 C 相当）

```toml
[bundle."環境の照会"]
machine = [
  "ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight.initial:1",
]
members = [
  "ukadoc:list_propertysystem:system.cpu._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.cursor.pos:1",
  "ukadoc:list_propertysystem:system.day:1",
  "ukadoc:list_propertysystem:system.dayofweek:1",
  "ukadoc:list_propertysystem:system.disk.count:1",
  "ukadoc:list_propertysystem:system.disk.index_28ID_29._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.dnd.mode:1",
  "ukadoc:list_propertysystem:system.hour:1",
  "ukadoc:list_propertysystem:system.memory._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.millisecond:1",
  "ukadoc:list_propertysystem:system.minute:1",
  "ukadoc:list_propertysystem:system.monitor.count:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.bpp:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.dpi:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.primary:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.rect:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.work:1",
  "ukadoc:list_propertysystem:system.month:1",
  "ukadoc:list_propertysystem:system.network._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.os._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.power._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.second:1",
  "ukadoc:list_propertysystem:system.theme.app.mode:1",
  "ukadoc:list_propertysystem:system.theme.os.mode:1",
  "ukadoc:list_propertysystem:system.year:1",
  "ukadoc:list_sakura_script:_25property_5b_30d7_30ed_30d1_30c6_30a3_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cproperty_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_5024_5d:1",
]
hand = [
  "ukadoc:list_propertysystem:system.cpu._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.cursor.pos:1",
  "ukadoc:list_propertysystem:system.day:1",
  "ukadoc:list_propertysystem:system.dayofweek:1",
  "ukadoc:list_propertysystem:system.disk.count:1",
  "ukadoc:list_propertysystem:system.disk.index_28ID_29._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.dnd.mode:1",
  "ukadoc:list_propertysystem:system.hour:1",
  "ukadoc:list_propertysystem:system.memory._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.millisecond:1",
  "ukadoc:list_propertysystem:system.minute:1",
  "ukadoc:list_propertysystem:system.monitor.count:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.bpp:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.dpi:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.primary:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.rect:1",
  "ukadoc:list_propertysystem:system.monitor.index_28ID_29.work:1",
  "ukadoc:list_propertysystem:system.month:1",
  "ukadoc:list_propertysystem:system.network._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.os._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.power._28_30ad_30fc_29:1",
  "ukadoc:list_propertysystem:system.second:1",
  "ukadoc:list_propertysystem:system.theme.app.mode:1",
  "ukadoc:list_propertysystem:system.theme.os.mode:1",
  "ukadoc:list_propertysystem:system.year:1",
  "ukadoc:list_sakura_script:_25property_5b_30d7_30ed_30d1_30c6_30a3_540d_5d:1",
]
domains = ["property", "sakura-script"]
foundation = "プロパティの問い合わせ口と値の解決"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 台詞に埋め込む `%property[プロパティ名]`、名指ししたイベントへ値を返す `\![get,property,イベント名,プロパティ名,プロパティ名,...]`、値を書き込む `\![set,property,プロパティ名,値]` の 3 つの口を受け付け、`system.` で始まる 25 の名前——今の年月日・曜日・時分秒とミリ秒、カーソルの位置、CPU とメモリとディスクとネットワークと電源の状態、画面の枚数と 1 枚ごとの位置・作業領域・色深度・DPI・主画面かどうか、OS の種別、OS とアプリの配色、応答不可の設定——を今の環境から解決して返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 今が何時か、画面が何枚あってどれだけの広さか、電池がどれだけ残っているか、OS が明るい配色か暗い配色か——そうしたことをゴーストが尋ねても値が返らない。台詞に書いた `%property[プロパティ名]` はその綴りのまま画面に出るので、利用者はゴーストの台詞の中に生の記号を読まされる。時刻や曜日で挨拶を変える作品、画面の広さに合わせて立ち位置を変える作品は、いつも同じ挨拶といつも同じ場所になる。

構成 id は 28 件で、うち機械の束から来たものが 2 件、人手で足したものが 26 件である（`hand` の行を数えた）。テーマは **0 件**である（構成 id 28 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 例示の 3 連鎖が 1 つの束に収まること

要件 3.1 が例示する 3 つの連鎖について、束 id と構成 id で着地を示す。件数はこの節を書いた
時点で数え直した（数え方: `report/summary.md`「ドメインを跨いで繋がった束」の表からその束 id の
行を取り、構成 id の欄を読点で切って数えた。3 つとも段 1 の値と同じである）。

| 連鎖 | 収まる束 id | その束の構成 id 数 | 名前付き束 |
| --- | --- | ---: | --- |
| 時刻の刻み | `ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1` | 3 | `自発発話` |
| 重なり順 | `ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1` | 49 | 段 2 のこのタスクでは付けない |
| インストール | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` | 53 | `インストール` |

**時刻の刻み** — 連鎖の 3 つの id はいずれも `ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1` を束 id とする 1 つの機械の束の構成 id である。
- `ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1`（secondchangeinterval,秒数・absent・assets）
- `ukadoc:list_plugin_event:OnSecondChange:1`（OnSecondChange・absent・shiori）
- `ukadoc:list_shiori_event:OnSecondChange:1`（OnSecondChange・implemented・shiori）
  3 つとも名前付き束「自発発話」の `members` に入っている（入っていない id は 0 件）。

**重なり順** — 連鎖の 3 つの id はいずれも `ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1` を束 id とする 1 つの機械の束の構成 id である。
- `ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`（seriko.zorder,スコープID,スコープID,...・implemented・assets）
- `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`（\![set,zorder,スコープID,スコープID,...]・implemented・sakura-script）
- `ukadoc:list_propertysystem:currentghost.seriko.zorder:1`（currentghost.seriko.zorder・degraded・property）
  この 3 つを含む名前付き束は段階 D 以降の担当（タスク 3.4）なので、タスク 3.2・3.3 では名付けない。上の 19 束の `members` に入っているものは 0 件である——機械の束としては 1 つに収まっており、名前付き束はこの後のタスクが付ける。

**インストール** — 連鎖の 3 つの id はいずれも `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` を束 id とする 1 つの機械の束の構成 id である。
- `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1`（\![execute,install,path,ファイル名]・absent・sakura-script）
- `ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1`（\![execute,install,url,URL,(feed|nar|homeurlのいずれか)]・absent・sakura-script）
- `ukadoc:list_shiori_event:OnInstallComplete:1`（OnInstallComplete・absent・shiori）
  3 つとも名前付き束「インストール」の `members` に入っている（入っていない id は 0 件）。

## 単独項目

<!-- 段 2（タスク 3.6）で書く: 束に入らない項目を同じ表に `single = true` で置き、
     入れられない理由と、ドメインごとの件数（0 のドメインも 0 と）を書く。 -->

## 合計

下の囲みは着手時（2026-09-12）に置いた器である。**すべての数はまだ数えていない仮の 0 であり、
段 2（タスク 3.6）で数え直した値に置き換える。** 引き算や引用で導かず、その時点で数え直す。

```toml
[tally]
target = 0
from_machine = 0
by_hand = 0
singles = 0
alias_excluded = 0
not_applicable_excluded = 0

[tally.singles_by_domain]
assets = 0
property = 0
sakura-script = 0
shiori = 0
```

## SAORI

<!-- 段 2（タスク 3.6）で書く: 台帳に行が 0 件である SAORI を、成立条件 3 つとともにこの節へ
     置く。実装項目としてどの束にも入れない。 -->

## 別軸

<!-- 段 2（タスク 3.6）で書く: 技術選定は順位を付けず、この節へ置く。 -->
