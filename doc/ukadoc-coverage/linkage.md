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

このタスクで名付けた束は **11**、構成 id は延べ **409** 件（機械の束から来たもの **108** 件・人手で足したもの **301** 件）で、同じ id が 2 つの束に
現れることは **0 件**である（数え方: 下の 11 の囲みの `members` を全部集めて重複を数えた。この 3 つの数はタスク 3.5 が id を足した後の値で、
タスク 3.2 が書いた時点の値ではない）。
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
  "ukadoc:list_shiori_resource:craftman:1",
  "ukadoc:list_shiori_resource:craftmanw:1",
  "ukadoc:list_shiori_resource:name:1",
  "ukadoc:list_shiori_resource:version:1",
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
  "ukadoc:list_shiori_resource:craftman:1",
  "ukadoc:list_shiori_resource:craftmanw:1",
  "ukadoc:list_shiori_resource:name:1",
  "ukadoc:list_shiori_resource:version:1",
]
domains = ["assets", "shiori"]
foundation = "SHIORI の読み込みと起動時イベントの発火路"
breakage = "黙って壊れる"
themes = ["気配", "記憶"]
```

**成立に要る最小の基盤**: descript.txt を読んでゴーストを組み立て、`shiori` の欄が指す SHIORI を読み込んで `OnInitialize`・`OnFirstBoot`・`OnBoot` をこの順に送り、返ってきたさくらスクリプトを再生できること。

**欠けると壊れる既存ゴーストの振る舞い**: 初めて入れたゴーストが最初の挨拶をしない。2 回目以降の起動でも黙ったまま立っているだけになり、作者名や表示名を読む欄が無いので、ゴースト一覧にも正しい名前が出ない。

構成 id は 20 件で、うち機械の束から来たものが 1 件、人手で足したものが 19 件である（`hand` の行を数えた。タスク 3.5 が 4 件を足した後の値である）。

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
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_2a:1",
  "ukadoc:list_sakura_script:_5c__21:1",
  "ukadoc:list_sakura_script:_5c__3f:1",
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
  "ukadoc:list_shiori_event:OnBalloonBreak:1",
  "ukadoc:list_shiori_event:OnBalloonClose:1",
  "ukadoc:list_shiori_event:OnBalloonTimeout:1",
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
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonwait_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1",
  "ukadoc:list_sakura_script:_5c_2a:1",
  "ukadoc:list_sakura_script:_5c__21:1",
  "ukadoc:list_sakura_script:_5c__3f:1",
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
  "ukadoc:list_shiori_event:OnBalloonBreak:1",
  "ukadoc:list_shiori_event:OnBalloonClose:1",
  "ukadoc:list_shiori_event:OnBalloonTimeout:1",
  "ukadoc:list_shiori_event:OnChoiceEnter:1",
  "ukadoc:list_shiori_event:OnChoiceHover:1",
  "ukadoc:list_shiori_resource:balloon_tooltip:1",
]
domains = ["sakura-script", "shiori"]
foundation = "さくらスクリプトの解釈とバルーンへの文字送り"
breakage = "黙って壊れる"
themes = ["気配", "掛け合い", "交わり", "気配り"]
```

**成立に要る最小の基盤**: さくらスクリプトを字句に分け、スコープの切り替え・サーフェスの指定・改行・待ち・終端をバルーンへ順に反映し、選択肢とアンカーを押した結果を SHIORI へ返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 台詞が最後まで流れて止まらず、クリックで送る間が無くなる。選択肢が出ないので、分岐する会話はどの枝にも進まない。二人が交互に喋る掛け合いも、片方のバルーンにまとめて出る。

構成 id は 46 件で、うち機械の束から来たものが 11 件、人手で足したものが 35 件である（`hand` の行を数えた。タスク 3.5 が 7 件を足した後の値である）。

### 撫で（段階 A 相当）

```toml
[bundle."撫で"]
machine = [
  "ukadoc:list_shiori_event:OnMouseClick:1",
]
members = [
  "ukadoc:descript_shell_surfaces:_5f53_305f_308a_5224_5b9a_540d_2c_8868_793a_5185_5bb9:1",
  "ukadoc:descript_shell_surfaces:animation_2a.collision_2a_2c_5f53_305f_308a_5224_5b9a_5b9a_7fa9animation_2a.collisionex_2a_2c_5f53_305f_308a_5224_5b9a_5:1",
  "ukadoc:descript_shell_surfaces:collision-sort_2c_30bd_30fc_30c8_9806_5e8f:1",
  "ukadoc:descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1",
  "ukadoc:descript_shell_surfaces:collisionex_2a_2cID_2c_30bf_30a4_30d7_2c_5ea7_6a191_2c_5ea7_6a192...:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist.count:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.text:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.text:1",
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
  "ukadoc:descript_shell_surfaces:_5f53_305f_308a_5224_5b9a_540d_2c_8868_793a_5185_5bb9:1",
  "ukadoc:descript_shell_surfaces:animation_2a.collision_2a_2c_5f53_305f_308a_5224_5b9a_5b9a_7fa9animation_2a.collisionex_2a_2c_5f53_305f_308a_5224_5b9a_5:1",
  "ukadoc:descript_shell_surfaces:collision-sort_2c_30bd_30fc_30c8_9806_5e8f:1",
  "ukadoc:descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1",
  "ukadoc:descript_shell_surfaces:collisionex_2a_2cID_2c_30bf_30a4_30d7_2c_5ea7_6a191_2c_5ea7_6a192...:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist.count:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.text:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.text:1",
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
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "当たり判定の解決とマウス入力の配送"
breakage = "黙って壊れる"
themes = ["触れ合い"]
```

**成立に要る最小の基盤**: surfaces.txt の当たり判定を面ごとに解決し、マウスの座標をその名前へ写して `OnMouseMove`・`OnMouseClick` 系のイベントとして SHIORI へ送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 頭を撫でても顔を触っても何も起こらない。ゴーストは立っているだけで、触れ合いを入口にした反応がすべて出ない。

構成 id は 33 件で、うち機械の束から来たものが 2 件、人手で足したものが 31 件である（`hand` の行を数えた。タスク 3.5 が 6 件を足した後の値である）。

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
  "ukadoc:list_shiori_resource:-:1",
  "ukadoc:list_shiori_resource:activaterootbutton.caption:1",
  "ukadoc:list_shiori_resource:addressbarbutton.caption:1",
  "ukadoc:list_shiori_resource:aistatebutton.caption:1",
  "ukadoc:list_shiori_resource:alignrootbutton.caption:1",
  "ukadoc:list_shiori_resource:alwaysstayontopbutton.caption:1",
  "ukadoc:list_shiori_resource:alwaystrayiconvisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:balloonhistorybutton.caption:1",
  "ukadoc:list_shiori_resource:balloonrootbutton.caption:1",
  "ukadoc:list_shiori_resource:calendarbutton.caption:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.type:1",
  "ukadoc:list_shiori_resource:char_2a.popupmenu.visible:1",
  "ukadoc:list_shiori_resource:charsetbutton.caption:1",
  "ukadoc:list_shiori_resource:closeballoonbutton.caption:1",
  "ukadoc:list_shiori_resource:closebutton.caption:1",
  "ukadoc:list_shiori_resource:collisionvisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:configurationbutton.caption:1",
  "ukadoc:list_shiori_resource:configurationrootbutton.caption:1",
  "ukadoc:list_shiori_resource:definedsurfaceonlybutton.caption:1",
  "ukadoc:list_shiori_resource:dictationbutton.caption:1",
  "ukadoc:list_shiori_resource:dressuprootbutton.caption:1",
  "ukadoc:list_shiori_resource:duibutton.caption:1",
  "ukadoc:list_shiori_resource:enableballoonmovebutton.caption:1",
  "ukadoc:list_shiori_resource:firststaffbutton.caption:1",
  "ukadoc:list_shiori_resource:ghostexplorerbutton.caption:1",
  "ukadoc:list_shiori_resource:ghosthistorybutton.caption:1",
  "ukadoc:list_shiori_resource:ghostinstallbutton.caption:1",
  "ukadoc:list_shiori_resource:ghostrootbutton.caption:1",
  "ukadoc:list_shiori_resource:helpbutton.caption:1",
  "ukadoc:list_shiori_resource:hidebutton.caption:1",
  "ukadoc:list_shiori_resource:historyrootbutton.caption:1",
  "ukadoc:list_shiori_resource:inforootbutton.caption:1",
  "ukadoc:list_shiori_resource:kero.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:kero.popupmenu.type:1",
  "ukadoc:list_shiori_resource:kero.popupmenu.visible:1",
  "ukadoc:list_shiori_resource:leavepassivebutton.caption:1",
  "ukadoc:list_shiori_resource:legacyinterface:1",
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
  "ukadoc:list_shiori_resource:messengerbutton.caption:1",
  "ukadoc:list_shiori_resource:portalrootbutton.caption:1",
  "ukadoc:list_shiori_resource:purgeghostcachebutton.caption:1",
  "ukadoc:list_shiori_resource:quitbutton.caption:1",
  "ukadoc:list_shiori_resource:rateofuseballoonbutton.caption:1",
  "ukadoc:list_shiori_resource:rateofusebutton.caption:1",
  "ukadoc:list_shiori_resource:rateofuserootbutton.caption:1",
  "ukadoc:list_shiori_resource:rateofusetotalbutton.caption:1",
  "ukadoc:list_shiori_resource:readmebutton.caption:1",
  "ukadoc:list_shiori_resource:readmebuttoncaption:1",
  "ukadoc:list_shiori_resource:recommendrootbutton.caption:1",
  "ukadoc:list_shiori_resource:regionenabledbutton.caption:1",
  "ukadoc:list_shiori_resource:reloadinfobutton.caption:1",
  "ukadoc:list_shiori_resource:resetballoonpositionbutton.caption:1",
  "ukadoc:list_shiori_resource:resettodefaultbutton.caption:1",
  "ukadoc:list_shiori_resource:sakura.popupmenu.applybindtoself:1",
  "ukadoc:list_shiori_resource:sakura.popupmenu.type:1",
  "ukadoc:list_shiori_resource:sakura.popupmenu.visible:1",
  "ukadoc:list_shiori_resource:shellrootbutton.caption:1",
  "ukadoc:list_shiori_resource:shellscaleotherbutton.caption:1",
  "ukadoc:list_shiori_resource:shellscalerootbutton.caption:1",
  "ukadoc:list_shiori_resource:switchactivatewhentalkbutton.caption:1",
  "ukadoc:list_shiori_resource:switchactivatewhentalkexceptupdatebutton.caption:1",
  "ukadoc:list_shiori_resource:switchcompatiblemodebutton.caption:1",
  "ukadoc:list_shiori_resource:switchconsolealwaysvisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:switchconsolevisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:switchdeactivatebutton.caption:1",
  "ukadoc:list_shiori_resource:switchdontactivatebutton.caption:1",
  "ukadoc:list_shiori_resource:switchdontforcealignbutton.caption:1",
  "ukadoc:list_shiori_resource:switchduivisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:switchforcealignfreebutton.caption:1",
  "ukadoc:list_shiori_resource:switchforcealignlimitbutton.caption:1",
  "ukadoc:list_shiori_resource:switchignoreserikomovebutton.caption:1",
  "ukadoc:list_shiori_resource:switchmovetodefaultpositionbutton.caption:1",
  "ukadoc:list_shiori_resource:switchproxybutton.caption:1",
  "ukadoc:list_shiori_resource:switchquietbutton.caption:1",
  "ukadoc:list_shiori_resource:switchreloadbutton.caption:1",
  "ukadoc:list_shiori_resource:switchreloadtempghostbutton.caption:1",
  "ukadoc:list_shiori_resource:switchrootbutton.caption:1",
  "ukadoc:list_shiori_resource:switchtalkghostbutton.caption:1",
  "ukadoc:list_shiori_resource:systeminfobutton.caption:1",
  "ukadoc:list_shiori_resource:termsbutton.caption:1",
  "ukadoc:list_shiori_resource:texttospeechbutton.caption:1",
  "ukadoc:list_shiori_resource:updatebutton.caption:1",
  "ukadoc:list_shiori_resource:updatebuttoncaption:1",
  "ukadoc:list_shiori_resource:updateplatformbutton.caption:1",
  "ukadoc:list_shiori_resource:utilityrootbutton.caption:1",
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
  "ukadoc:list_shiori_resource:-:1",
  "ukadoc:list_shiori_resource:activaterootbutton.caption:1",
  "ukadoc:list_shiori_resource:addressbarbutton.caption:1",
  "ukadoc:list_shiori_resource:aistatebutton.caption:1",
  "ukadoc:list_shiori_resource:alignrootbutton.caption:1",
  "ukadoc:list_shiori_resource:alwaysstayontopbutton.caption:1",
  "ukadoc:list_shiori_resource:alwaystrayiconvisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:balloonhistorybutton.caption:1",
  "ukadoc:list_shiori_resource:balloonrootbutton.caption:1",
  "ukadoc:list_shiori_resource:calendarbutton.caption:1",
  "ukadoc:list_shiori_resource:charsetbutton.caption:1",
  "ukadoc:list_shiori_resource:closeballoonbutton.caption:1",
  "ukadoc:list_shiori_resource:closebutton.caption:1",
  "ukadoc:list_shiori_resource:collisionvisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:configurationbutton.caption:1",
  "ukadoc:list_shiori_resource:configurationrootbutton.caption:1",
  "ukadoc:list_shiori_resource:definedsurfaceonlybutton.caption:1",
  "ukadoc:list_shiori_resource:dictationbutton.caption:1",
  "ukadoc:list_shiori_resource:dressuprootbutton.caption:1",
  "ukadoc:list_shiori_resource:duibutton.caption:1",
  "ukadoc:list_shiori_resource:enableballoonmovebutton.caption:1",
  "ukadoc:list_shiori_resource:firststaffbutton.caption:1",
  "ukadoc:list_shiori_resource:ghostexplorerbutton.caption:1",
  "ukadoc:list_shiori_resource:ghosthistorybutton.caption:1",
  "ukadoc:list_shiori_resource:ghostinstallbutton.caption:1",
  "ukadoc:list_shiori_resource:ghostrootbutton.caption:1",
  "ukadoc:list_shiori_resource:helpbutton.caption:1",
  "ukadoc:list_shiori_resource:hidebutton.caption:1",
  "ukadoc:list_shiori_resource:historyrootbutton.caption:1",
  "ukadoc:list_shiori_resource:inforootbutton.caption:1",
  "ukadoc:list_shiori_resource:leavepassivebutton.caption:1",
  "ukadoc:list_shiori_resource:legacyinterface:1",
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
  "ukadoc:list_shiori_resource:messengerbutton.caption:1",
  "ukadoc:list_shiori_resource:portalrootbutton.caption:1",
  "ukadoc:list_shiori_resource:purgeghostcachebutton.caption:1",
  "ukadoc:list_shiori_resource:quitbutton.caption:1",
  "ukadoc:list_shiori_resource:rateofuseballoonbutton.caption:1",
  "ukadoc:list_shiori_resource:rateofusebutton.caption:1",
  "ukadoc:list_shiori_resource:rateofuserootbutton.caption:1",
  "ukadoc:list_shiori_resource:rateofusetotalbutton.caption:1",
  "ukadoc:list_shiori_resource:readmebutton.caption:1",
  "ukadoc:list_shiori_resource:readmebuttoncaption:1",
  "ukadoc:list_shiori_resource:recommendrootbutton.caption:1",
  "ukadoc:list_shiori_resource:regionenabledbutton.caption:1",
  "ukadoc:list_shiori_resource:reloadinfobutton.caption:1",
  "ukadoc:list_shiori_resource:resetballoonpositionbutton.caption:1",
  "ukadoc:list_shiori_resource:resettodefaultbutton.caption:1",
  "ukadoc:list_shiori_resource:shellrootbutton.caption:1",
  "ukadoc:list_shiori_resource:shellscaleotherbutton.caption:1",
  "ukadoc:list_shiori_resource:shellscalerootbutton.caption:1",
  "ukadoc:list_shiori_resource:switchactivatewhentalkbutton.caption:1",
  "ukadoc:list_shiori_resource:switchactivatewhentalkexceptupdatebutton.caption:1",
  "ukadoc:list_shiori_resource:switchcompatiblemodebutton.caption:1",
  "ukadoc:list_shiori_resource:switchconsolealwaysvisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:switchconsolevisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:switchdeactivatebutton.caption:1",
  "ukadoc:list_shiori_resource:switchdontactivatebutton.caption:1",
  "ukadoc:list_shiori_resource:switchdontforcealignbutton.caption:1",
  "ukadoc:list_shiori_resource:switchduivisiblebutton.caption:1",
  "ukadoc:list_shiori_resource:switchforcealignfreebutton.caption:1",
  "ukadoc:list_shiori_resource:switchforcealignlimitbutton.caption:1",
  "ukadoc:list_shiori_resource:switchignoreserikomovebutton.caption:1",
  "ukadoc:list_shiori_resource:switchmovetodefaultpositionbutton.caption:1",
  "ukadoc:list_shiori_resource:switchproxybutton.caption:1",
  "ukadoc:list_shiori_resource:switchquietbutton.caption:1",
  "ukadoc:list_shiori_resource:switchreloadbutton.caption:1",
  "ukadoc:list_shiori_resource:switchreloadtempghostbutton.caption:1",
  "ukadoc:list_shiori_resource:switchrootbutton.caption:1",
  "ukadoc:list_shiori_resource:switchtalkghostbutton.caption:1",
  "ukadoc:list_shiori_resource:systeminfobutton.caption:1",
  "ukadoc:list_shiori_resource:termsbutton.caption:1",
  "ukadoc:list_shiori_resource:texttospeechbutton.caption:1",
  "ukadoc:list_shiori_resource:updatebutton.caption:1",
  "ukadoc:list_shiori_resource:updatebuttoncaption:1",
  "ukadoc:list_shiori_resource:updateplatformbutton.caption:1",
  "ukadoc:list_shiori_resource:utilityrootbutton.caption:1",
]
domains = ["assets", "property", "shiori"]
foundation = "メニューの組み立てと自前描画"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: 右クリックでメニューを組み立て、descript.txt と SHIORI の資源が指定した項目・配色・背景画像で自前描画し、選んだ項目を実行できること。

**欠けると壊れる既存ゴーストの振る舞い**: 右クリックしても何も出ない。着せ替えの切り替え・シェルの選択・ゴーストの入れ替え・終了はすべてメニューが入口なので、利用者はゴーストを操作する手段を持たない。

構成 id は 145 件で、うち機械の束から来たものが 19 件、人手で足したものが 126 件である（`hand` の行を数えた。タスク 3.5 が 76 件を足した後の値である）。

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
  "ukadoc:descript_balloon:homeurl_2cURL:1",
  "ukadoc:descript_ghost:homeurl_2cURL:1",
  "ukadoc:descript_shell:homeurl_2cURL:1",
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
  "ukadoc:spec_update_file:URL_30a8_30f3_30b3_30fc_30c9:1",
  "ukadoc:spec_update_file:_30bb_30ad_30e5_30ea_30c6_30a3_30c1_30a7_30c3_30af:1",
  "ukadoc:spec_update_file:_30d5_30a1_30a4_30eb_8d70_67fb:1",
  "ukadoc:spec_update_file:_5fc5_9808_30d5_30a3_30fc_30eb_30c9:1",
  "ukadoc:spec_update_file:_62e1_5f35_30d5_30a3_30fc_30eb_30c9_20_28_4f4d_7f6e_5b2_5d_4ee5_964d_29:1",
  "ukadoc:spec_update_file:_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:spec_update_file:_884c_30d5_30a9_30fc_30de_30c3_30c8:1",
  "ukadoc:spec_update_file:_884c_7a2e_5225:1",
  "ukadoc:spec_update_file:ghost_5cmaster_3078_306e_30b3_30d4_30fc:1",
]
hand = [
  "ukadoc:descript_balloon:homeurl_2cURL:1",
  "ukadoc:descript_ghost:homeurl_2cURL:1",
  "ukadoc:descript_shell:homeurl_2cURL:1",
  "ukadoc:list_sakura_script:_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1",
  "ukadoc:list_shiori_resource:homeurl:1",
  "ukadoc:list_shiori_resource:useorigin1:1",
  "ukadoc:spec_update_file:URL_30a8_30f3_30b3_30fc_30c9:1",
  "ukadoc:spec_update_file:_30bb_30ad_30e5_30ea_30c6_30a3_30c1_30a7_30c3_30af:1",
  "ukadoc:spec_update_file:_30d5_30a1_30a4_30eb_8d70_67fb:1",
  "ukadoc:spec_update_file:_5fc5_9808_30d5_30a3_30fc_30eb_30c9:1",
  "ukadoc:spec_update_file:_62e1_5f35_30d5_30a3_30fc_30eb_30c9_20_28_4f4d_7f6e_5b2_5d_4ee5_964d_29:1",
  "ukadoc:spec_update_file:_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:spec_update_file:_884c_30d5_30a9_30fc_30de_30c3_30c8:1",
  "ukadoc:spec_update_file:_884c_7a2e_5225:1",
  "ukadoc:spec_update_file:ghost_5cmaster_3078_306e_30b3_30d4_30fc:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "ネットワーク越しの差分取得とファイルの入れ替え"
breakage = "黙って壊れる"
themes = ["更新"]
```

**成立に要る最小の基盤**: homeurl が指すサーバから updates2.dau を取り、md5 を突き合わせて差分だけを取得し、入れ替えの前後で `OnUpdate*` 系のイベントを送れること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴーストが新しい版に上がらない。作者が配信した修正も追加の台詞も届かず、更新の進行を伝える台詞も 1 つも出ない。

構成 id は 50 件で、うち機械の束から来たものが 35 件、人手で足したものが 15 件である（`hand` の行を数えた。タスク 3.5 が 11 件を足した後の値である）。

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
  "ukadoc:list_sakura_script:_25lastghostname:1",
  "ukadoc:list_sakura_script:_25lastobjectname:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2cpath_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_28feed_7cnar_7chomeurl_306e_3044_305a_308c_304b_29_5d:1",
  "ukadoc:list_shiori_event:OnGhostTermsAccept:1",
  "ukadoc:list_shiori_event:OnGhostTermsDecline:1",
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
  "ukadoc:list_sakura_script:_25lastghostname:1",
  "ukadoc:list_sakura_script:_25lastobjectname:1",
  "ukadoc:list_shiori_event:OnGhostTermsAccept:1",
  "ukadoc:list_shiori_event:OnGhostTermsDecline:1",
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

構成 id は 40 件で、うち機械の束から来たものが 9 件、人手で足したものが 31 件である（`hand` の行を数えた。タスク 3.5 が 4 件を足した後の値である）。

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
  "ukadoc:list_propertysystem:currentghost.shelllist.count:1",
  "ukadoc:list_propertysystem:currentghost.shelllist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.shelllist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.shelllist_28_30b7_30a7_30eb_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
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
  "ukadoc:list_sakura_script:_5cb_5bID_756a_53f7_5d:1",
  "ukadoc:list_shiori_event:OnBalloonChange:1",
  "ukadoc:list_shiori_event:OnBalloonScaling:1",
  "ukadoc:list_shiori_event:OnDressupChanged:1",
  "ukadoc:list_shiori_event:OnGhostChanged:1",
  "ukadoc:list_shiori_event:OnGhostChanging:1",
  "ukadoc:list_shiori_event:OnNotifyBalloonInfo:1",
  "ukadoc:list_shiori_event:OnNotifyDressupInfo:1",
  "ukadoc:list_shiori_event:OnNotifySelfInfo:1",
  "ukadoc:list_shiori_event:OnNotifyShellInfo:1",
  "ukadoc:list_shiori_event:OnShellChanged:1",
  "ukadoc:list_shiori_event:OnShellChanging:1",
  "ukadoc:list_shiori_event:OnShellScaling:1",
  "ukadoc:manual_shell",
]
hand = [
  "ukadoc:list_propertysystem:currentghost.shelllist.count:1",
  "ukadoc:list_propertysystem:currentghost.shelllist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.shelllist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.shelllist_28_30b7_30a7_30eb_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bbind-noevent_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cballoon_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cghost_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cshell_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_500d_7387_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cscaling_2c_6a2a_500d_7387_2c_7e26_500d_7387_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5cb_5bID_756a_53f7_5d:1",
  "ukadoc:list_shiori_event:OnBalloonScaling:1",
  "ukadoc:list_shiori_event:OnGhostChanged:1",
  "ukadoc:list_shiori_event:OnNotifyBalloonInfo:1",
  "ukadoc:list_shiori_event:OnNotifySelfInfo:1",
  "ukadoc:list_shiori_event:OnNotifyShellInfo:1",
  "ukadoc:list_shiori_event:OnShellScaling:1",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "シェル・バルーン・ゴーストの読み直しと差し替え"
breakage = "黙って壊れる"
themes = ["装い", "記憶"]
```

**成立に要る最小の基盤**: shell/master と balloon の別のフォルダを読み直して立ち絵とバルーンを差し替え、着せ替えの重ね合わせを付け外しし、別のゴーストへ入れ替えられること。

**欠けると壊れる既存ゴーストの振る舞い**: シェルを選んでも見た目が変わらない。着せ替えの服も切り替わらず、同じゴーストに複数の姿を用意した作品はどれも 1 つの姿しか見せられない。

構成 id は 32 件で、うち機械の束から来たものが 15 件、人手で足したものが 17 件である（`hand` の行を数えた。タスク 3.5 が 6 件を足した後の値である）。

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

このタスクで名付けた束は **8**、構成 id は延べ **81** 件（機械の束から来たもの **18** 件・
人手で足したもの **63** 件。タスク 3.5 が id を足した後の値である）で、同じ id が 2 つの束に現れることは **0 件**である（数え方: 下の
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
  "ukadoc:list_sakura_script:_5c8_5b_30d5_30a1_30a4_30eb_540d_5d:1",
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
  "ukadoc:list_sakura_script:_5c8_5b_30d5_30a1_30a4_30eb_540d_5d:1",
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

構成 id は 34 件で、うち機械の束から来たものが 14 件、人手で足したものが 20 件である（`hand` の行を数えた。タスク 3.5 が 1 件を足した後の値である）。テーマは **0 件**である（構成 id 34 件の `values` をすべて読み、空でないものが 1 件も無かった）。

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

段 2 のこのタスク（3.4）が書くのは段階 D 相当の 7 束——多重ゴースト・コミュニケート・呼び出し・
SSTP・FMO・PLUGIN・リンク——と、段階 E 相当の 4 束——外部アプリ・開発者機能・トランスレータ・
ヘッドライン——である。書き方は上の 19 束と同じで、囲みが ⑴ 名前（表の鍵）・⑵ `machine`・
⑶ `members` と人手の印 `hand`・⑷ `domains`・⑸ `foundation` の見出し・⑺ `breakage`・⑻ `themes`
を持ち、⑸ の中身と ⑹ は囲みの直下の本文に書く。

このタスクで名付けた束は **11**、構成 id は延べ **261** 件（機械の束から来たもの **64** 件・
人手で足したもの **197** 件。タスク 3.5 が id を足した後の値である）で、同じ id が 2 つの束に現れることは **0 件**である（数え方: 下の
11 の囲みの `members` を全部集めて重複を数えた）。上の 19 束が使った id との重なりも **0 件**で
ある（数え方: 上の 19 束と下の 11 束の `members` を集めて共通部分を数えた）。状態が `alias` の id と
`not-applicable` の id は **0 件**である（同じ集合を台帳の `status` で引き直して数えた。除外の
件数と理由はタスク 3.6 の合計の節に書く）。

タスク 3.4 までに名付けた 30 束が使った id を合わせると **751** 件である（数え方: 30 の囲みの
`members` を全部集めて重複を除いて数えた。タスク 3.5 が 13 束へ id を足した後の値で、タスク 3.4 が
書いた時点の値ではない）。対象 4 状態の全数は **1,552** 件で、残りはタスク 3.5 が下で引き受ける
（数え方: 台帳 4 本から状態が `implemented`・`vocabulary-only`・`degraded`・`absent` の項目を数えた）。

### このタスクへ回された id の始末

タスク 3.1 と 3.3 がこのタスクへ回した id は 3 群ある。3 群とも下で処分した。

**⑴ makoto の機械の束から回ってきた 6 件。** `ukadoc:manual_translator` は「トランスレータ」へ
置いた（3.1 の表の行き先どおり）。`ukadoc:dev_nar`・
`ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreatenar_5d:1`・
`ukadoc:list_shiori_event:OnNarCreated:1`・`ukadoc:list_shiori_event:OnNarCreating:1` の 4 件は
「開発者機能」へ置いた（3.1 の表が `ukadoc:dev_nar` に付けた行き先どおりで、設計 D-7 が案として
書いた「nar の作成」という別の束は立てていない）。
`ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1` は「トランスレータ」へ置いた——
3.1 の表はページ単位の id 13 件だけを並べたもので、この id は 4 節の項目 id なので表に無い。
設計 D-7 が「トランスレータ（`descript_ghost:makoto`・`manual_translator`）」と書いた組み合わせに
従った。残る 3 件（`ukadoc:manual_balloon`・`ukadoc:manual_directory`・`ukadoc:manual_ghost`）は
3.1 の表が「単独項目」としており、タスク 3.6 の担当なのでここでは扱わない。

**⑵ タスク 3.3 が回した `property.get`・`property.set`。** 同じ題の行は 4 つある——プラグイン
向けの一覧に 2 つ（`ukadoc:list_plugin_event:property.get:1`・
`ukadoc:list_plugin_event:property.set:1`）、ゴースト向けの一覧に 2 つ
（`ukadoc:list_shiori_event:property.get:1`・`ukadoc:list_shiori_event:property.set:1`）である。
台帳の関連を読むと、プラグイン向けの 2 つは `pluginlist(…).ext.拡張プロパティ名` の 2 行を、
ゴースト向けの 2 つは `activeghostlist(…).ext.拡張プロパティ名` の 2 行を、いずれも `queries` で
指している。**問う相手で切った**——プラグイン向けの 2 件と `pluginlist` の `ext` 2 件を「PLUGIN」へ、
ゴースト向けの 2 件と `activeghostlist` の `ext` 2 件を「多重ゴースト」へ置いた。段階 C 相当の
「環境の照会」へ入れなかったのは、あちらの構成 id が `system.` で始まる 25 の名前と 3 つの口で
閉じていて、外の相手へ取り次ぐ往復を含まないためである。

**⑶ 重なり順の 3 件。** 引き受けない。**引受先はタスク 3.5 で、同じ機構の
`ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1` を加えた 4 件をまとめて 1 つの名前付き束へ
入れることになる。** 4 件が指しているのは 1 体のゴーストが持つ複数のキャラクター窓をどの順で
重ねて描くかであって、段階 D の 7 束が扱う「他のゴースト・外部のプログラム・プラグインとの
やりとり」でも、段階 E の 4 束が扱う「外の道具と作り手の道具」でもない。

**この文書で「関連 0 本」と言うときは、4 台帳の `links` を向き無しのグラフに直したときに端を
1 本も持たず、かつ状態が対象 4 状態のいずれかであることを指す**（タスク 3.5 の表題が挙げる
1,127 件はこの数え方である。要件 4.2 はもう 1 つの数え方——自分の行の `links` が空・全状態——で
1,494 件にも触れているが、こちらではない。そちらを同じく今の台帳で数え直すと 1,492 件で、段 1 が
2 つの行に `links` を書き足したので 2 件減っている）。この数え方で今の台帳を数え直すと **1,126 件**で、
タスク 3.5 の表題の 1,127 より 1 件少ない。減った 1 件は下の `seriko.zorder…` そのもので、段 1 の
「補修した関連」が端を 1 本足したからである（数え方: 4 台帳の `links` をすべて読んで向き無しの
グラフに直し、端を 1 本も持たない id のうち状態が `implemented`・`vocabulary-only`・`degraded`・
`absent` のものを数えた）。

4 件の端の本数を、同じ数え方で id ごとに数え直した結果は次のとおりである。

| id | 状態 | 向き無しの端 | 相手と種別（どちらの行に書かれているか） |
| --- | --- | ---: | --- |
| `ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1` | `implemented` | 1 | `configures` で `ukadoc:list_propertysystem:currentghost.seriko.zorder:1`（相手の行に書かれている。段 1 が足した 1 本＝上の「補修した関連」の表の property の行） |
| `ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1` | `implemented` | 1 | `same-feature` で `ukadoc:list_propertysystem:currentghost.seriko.zorder:1`（相手の行に書かれている） |
| `ukadoc:list_propertysystem:currentghost.seriko.zorder:1` | `degraded` | 3 | `same-feature` で `ukadoc:list_sakura_script:_5c_21_5bset_2cproperty_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_5024_5d:1` と上の `\![set,zorder,…]`、`configures` で上の `seriko.zorder…`（3 本とも自分の行に書かれている） |
| `ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1` | `implemented` | 0 | 端を 1 本も持たない |

**つまり「関連 0 本」なのは `\![reset,zorder]` の 1 件だけで、残る 3 件はタスク 3.5 の母集団に
入らない。** 3 件は機械の束 `ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1` の
構成 id として引き渡す（下の「例示の 3 連鎖」の表がこの束 id を挙げている。端を持つからこそ
機械の束に入っており、「関連 0 本」とは両立しない）。4 件を 1 つの束にまとめる必要があるのは、
要件 3.1 が重なり順を「1 つの束に収まる」連鎖として例示しているからである——3 件と 1 件を
別々に扱うと、この連鎖が 2 つ以上に割れる。

引受先をタスク 3.5 に寄せたのは、名前付き束へ id を足せる最後のタスクが 3.5 だからである
（3.6 が書くのは単独項目＝`members` が id 1 つの行で、そこへ落とすと連鎖が 4 つに割れる）。
設計 D-7 ⑵ は `\![reset,zorder]` について「関連 0 本のままで、名前付き束へは人手で入れる（3.6）」
と書いているが、この「3.6」は**要件 3.6**（`links` を足さずに帰属は `linkage.md` で決める）を指す
番号であってタスク番号ではない。要件 3.6 の指示どおり `links` を 1 本も足さずに人手で入れる形は
タスク 3.5 の持ち場なので、設計と食い違いは無い。

4 件とも状態は `implemented` か `degraded` で、`absent` は 0 件である（台帳の `status` を 4 件とも
引いて数えた）。

### 多重ゴースト（段階 D 相当）

```toml
[bundle."多重ゴースト"]
machine = [
  "ukadoc:descript_plugin:otherghosttalk_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:list_plugin_event:property.get:1",
  "ukadoc:list_sakura_script:_5c_21_5bnotifyother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cothersurfacechange_2ctrue_304bfalse_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5btimernotifyother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30:1",
]
members = [
  "ukadoc:list_propertysystem:activeghostlist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:activeghostlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:activeghostlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_5:1",
  "ukadoc:list_propertysystem:activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30:1",
  "ukadoc:list_sakura_script:_5c_21_5bnotifyother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5braiseother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cotherghosttalk_2ctrue_7cfalse_7cbefore_7cafter_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cothersurfacechange_2ctrue_304bfalse_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cserikotalk_2ctrue_2ffalse_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5btimernotifyother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30:1",
  "ukadoc:list_sakura_script:_5c_21_5btimerraiseother_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f:1",
  "ukadoc:list_shiori_event:OnEmbryoExist:1",
  "ukadoc:list_shiori_event:OnNekodorifExist:1",
  "ukadoc:list_shiori_event:OnNotifyOtherFailure:1",
  "ukadoc:list_shiori_event:OnOtherGhostBooted:1",
  "ukadoc:list_shiori_event:OnOtherGhostChanged:1",
  "ukadoc:list_shiori_event:OnOtherGhostClosed:1",
  "ukadoc:list_shiori_event:OnOtherGhostTalk:1",
  "ukadoc:list_shiori_event:OnOtherGhostVanished:1",
  "ukadoc:list_shiori_event:OnOtherOffscreen:1",
  "ukadoc:list_shiori_event:OnOtherOverlap:1",
  "ukadoc:list_shiori_event:OnOtherSurfaceChange:1",
  "ukadoc:list_shiori_event:OnRaiseOtherFailure:1",
  "ukadoc:list_shiori_event:otherghostname:1",
  "ukadoc:list_shiori_event:ownerghostname:1",
  "ukadoc:list_shiori_event:property.get:1",
  "ukadoc:list_shiori_event:property.set:1",
]
hand = [
  "ukadoc:list_propertysystem:activeghostlist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:activeghostlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_5:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cserikotalk_2ctrue_2ffalse_5d:1",
  "ukadoc:list_shiori_event:OnEmbryoExist:1",
  "ukadoc:list_shiori_event:OnNekodorifExist:1",
  "ukadoc:list_shiori_event:OnOtherGhostBooted:1",
  "ukadoc:list_shiori_event:OnOtherGhostChanged:1",
  "ukadoc:list_shiori_event:OnOtherGhostClosed:1",
  "ukadoc:list_shiori_event:OnOtherGhostVanished:1",
  "ukadoc:list_shiori_event:OnOtherOffscreen:1",
  "ukadoc:list_shiori_event:OnOtherOverlap:1",
  "ukadoc:list_shiori_event:otherghostname:1",
  "ukadoc:list_shiori_event:ownerghostname:1",
]
domains = ["property", "sakura-script", "shiori"]
foundation = "起動中のゴーストを数え上げて名指しで届ける経路"
breakage = "黙って壊れる"
themes = ["交わり"]
```

**成立に要る最小の基盤**: 同じパソコンで動いている他のゴーストの一覧を持ち、相手を本体側名で名指しできること。相手が起動した・切り替わった・閉じた・消えた場面を `OnOtherGhostBooted`・`OnOtherGhostChanged`・`OnOtherGhostClosed`・`OnOtherGhostVanished` で送り、相手の台詞を `OnOtherGhostTalk` で横に流し（流し方は `\![set,otherghosttalk,true|false|before|after]` で選ぶ）、相手の表情が変わったことを `OnOtherSurfaceChange` で送れること（負荷を避けるため既定では無効で、`\![set,othersurfacechange,trueかfalse]` で有効にしたときだけ届く）。全員の窓の重なりと画面外への はみ出しを `OnOtherOverlap`・`OnOtherOffscreen` で送れること。相手を名指ししてイベントを投げる `\![raiseother,ゴースト名,イベント名,r0,r1,r2...]`・`\![notifyother,ゴースト名,イベント名,r0,r1,r2...]` と時間差の 2 つを受け付け、届かなかったときに `OnRaiseOtherFailure`・`OnNotifyOtherFailure` を返せること。他に起動しているゴーストの名前を `otherghostname`、自分を呼んだゴーストの名前を `ownerghostname` でゴーストへ知らせられること。起動中のゴーストの一覧を `activeghostlist` として読めて、`activeghostlist(…).ext.拡張プロパティ名` を問われたら相手のゴーストへ `property.get`・`property.set` で取り次ぎ、返った値を答えとして渡せること。

**欠けると壊れる既存ゴーストの振る舞い**: 2 体目のゴーストを立ち上げても、互いに相手がいることに気づかない。隣に誰か来たときの挨拶も、相手の台詞への合いの手も、相手が帰るときの見送りも出ない。他のゴーストとの掛け合いを売りにした作品は、1 体だけを立ち上げたときと同じ台詞しか出さない。相手を名指しして話しかける演目は、投げたイベントが誰にも届かず失敗の知らせも返らないので、返事を待つ側が黙ったまま止まる。相手のプロパティを尋ねて反応を変える辞書も、いつも同じ既定の反応になる。

構成 id は 28 件で、うち機械の束から来たものが 14 件、人手で足したものが 14 件である（`hand` の行を数えた。タスク 3.5 が 3 件を足した後の値である）。この束の `machine` に挙げた 5 つの機械の束のうち、`ukadoc:descript_plugin:otherghosttalk_2c_30aa_30d7_30b7_30e7_30f3:1` と `ukadoc:list_plugin_event:property.get:1` の 2 つは下の「PLUGIN」も引用する（1 つの機械の束を複数の名前付き束が引用してよい・設計 D-7）。分け方は相手が誰かで切った——問う相手・知らせる相手がゴーストである 4 件をこちらに、プラグインである 4 件を PLUGIN に置いた。

### コミュニケート（段階 D 相当）

```toml
[bundle."コミュニケート"]
machine = [
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ccommunicatebox_5d:1",
  "ukadoc:list_shiori_event:OnCommunicate:1",
]
members = [
  "ukadoc:descript_balloon:communicatebox.background.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.background.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.background.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:communicatebox.height_2c_30b5_30a4_30ba:1",
  "ukadoc:descript_balloon:communicatebox.width_2c_30b5_30a4_30ba:1",
  "ukadoc:descript_balloon:communicatebox.x_2c_5ea7_6a19:1",
  "ukadoc:descript_balloon:communicatebox.y_2c_5ea7_6a19:1",
  "ukadoc:descript_ghost:sstp.allowcommunicate_2c_6570_5024:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2ccommunicatebox_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cteachbox_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ccommunicatebox_5d:1",
  "ukadoc:list_shiori_event:OnCommunicate:1",
  "ukadoc:list_shiori_event:OnCommunicateInputCancel:1",
]
hand = [
  "ukadoc:descript_balloon:communicatebox.background.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.background.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.background.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:communicatebox.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:communicatebox.height_2c_30b5_30a4_30ba:1",
  "ukadoc:descript_balloon:communicatebox.width_2c_30b5_30a4_30ba:1",
  "ukadoc:descript_balloon:communicatebox.x_2c_5ea7_6a19:1",
  "ukadoc:descript_balloon:communicatebox.y_2c_5ea7_6a19:1",
  "ukadoc:descript_ghost:sstp.allowcommunicate_2c_6570_5024:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2ccommunicatebox_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cteachbox_5d:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "外から渡された要求をスクリプトとイベントに変える受け口"
breakage = "黙って壊れる"
themes = ["装い", "交わり"]
```

**成立に要る最小の基盤**: 利用者が文字を打ち込む小さな窓（コミュニケートボックス）を `\![open,communicatebox]` で開き `\![close,communicatebox]` で閉じられること。打ち込まれた文字と、他のゴーストから渡されたスクリプトの両方を `OnCommunicate` として SHIORI へ送り、送り元を Reference0 に載せられること（窓からは `user`、ゴーストからは相手の本体側名）。入力を取り消した場面を `OnCommunicateInputCancel` で送れること。`sstp.allowcommunicate,0` を書いたゴーストへは他のゴーストからのコミュニケートを渡さないこと。窓の位置・大きさ・文字色・背景色・フォントを、バルーンの `communicatebox.x,座標` 以下 11 個の設定どおりに描けること。

**欠けると壊れる既存ゴーストの振る舞い**: 利用者がゴーストに文字で話しかける手段が無い。「何か話しかけて」と促す作品はその先が続かず、打ち込んだ言葉を受け取る辞書は 1 度も呼ばれない。ゴーストどうしがスクリプトを渡し合う道も閉じているので、相方を呼んで掛け合いをさせる演目は片方が黙ったままになる。バルーンの作者が入力窓の見た目を作り込んでいても、その窓は 1 度も画面に出ない。

構成 id は 17 件で、うち機械の束から来たものが 3 件、人手で足したものが 14 件である（`hand` の行を数えた。タスク 3.5 が 1 件を足した後の値である）。

### 呼び出し（段階 D 相当）

```toml
[bundle."呼び出し"]
machine = [
  "ukadoc:list_sakura_script:_5c_21_5bcall_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1",
]
members = [
  "ukadoc:list_sakura_script:_5c_21_5bcall_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1",
  "ukadoc:list_shiori_event:OnGhostCallComplete:1",
  "ukadoc:list_shiori_event:OnGhostCalled:1",
  "ukadoc:list_shiori_event:OnGhostCalling:1",
  "ukadoc:list_shiori_resource:callghosthistorybutton.caption:1",
  "ukadoc:list_shiori_resource:callghostrootbutton.caption:1",
]
hand = [
  "ukadoc:list_shiori_event:OnGhostCallComplete:1",
  "ukadoc:list_shiori_event:OnGhostCalled:1",
  "ukadoc:list_shiori_resource:callghosthistorybutton.caption:1",
  "ukadoc:list_shiori_resource:callghostrootbutton.caption:1",
]
domains = ["sakura-script", "shiori"]
foundation = "起動中のゴーストを数え上げて名指しで届ける経路"
breakage = "黙って壊れる"
themes = ["交わり", "装い"]
```

**成立に要る最小の基盤**: 台詞の中の `\![call,ghost,ゴースト名(,--option=raise-event)]` で別のゴーストを起動できること（`lastinstalled` は最後にインストールしたもの、`random` は無作為）。呼ぶ側には、`--option=raise-event` を付けたときとメニューから切り替えたときに `OnGhostCalling` を、相手が立ち上がった後に `OnGhostCallComplete` を送ること。呼ばれた側には `OnGhostCalled` を送り、呼んだゴーストの本体側名・呼び出し時のスクリプト・名前・パスと、呼ばれた側のシェル名を Reference に載せること。`OnGhostCalled` に台詞が返らなかった（204）ときは、続けて `OnBoot` を起こすこと。メニューの「呼び出し」と「呼び出し履歴」の名前を、ゴーストが返す `callghostrootbutton.caption`・`callghosthistorybutton.caption` から読めること。

**欠けると壊れる既存ゴーストの振る舞い**: 台詞から友達のゴーストを呼べない。「あの子を呼ぶね」と言って相手を連れてくる演目はそこで止まり、呼ばれた側の「呼ばれて来ました」という第一声も出ない。2 体を呼び合わせて話を進める作品は、利用者が自分でもう 1 体を起動するまで先へ進まない。メニューの「呼び出し」と「呼び出し履歴」の項目も、ゴーストが用意した言葉に置き換わらない。

構成 id は 6 件で、うち機械の束から来たものが 2 件、人手で足したものが 4 件である（`hand` の行を数えた）。

### SSTP（段階 D 相当）

```toml
[bundle."SSTP"]
machine = []
members = [
  "ukadoc:descript_balloon:onlinemarker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:onlinemarker.interval_2c_5f85_6a5f_6642_9593:1",
  "ukadoc:descript_balloon:onlinemarker.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:onlinemarker.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmarker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:sstpmarker.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmarker.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmessage.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:sstpmessage.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:sstpmessage.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:sstpmessage.font.height_2c_30b5_30a4_30ba:1",
  "ukadoc:descript_balloon:sstpmessage.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:sstpmessage.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmessage.xr_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmessage.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_ghost:sstp.allowunspecifiedsend_2c_6570_5024:1",
  "ukadoc:list_shiori_event:OnSSTPBlacklisting:1",
  "ukadoc:list_shiori_event:OnSSTPBreak:1",
  "ukadoc:list_shiori_event:uniqueid:1",
  "ukadoc:list_shiori_resource:callsstpsendboxbutton.caption:1",
  "ukadoc:list_shiori_resource:switchblacklistingbutton.caption:1",
  "ukadoc:list_shiori_resource:switchlocalsstpbutton.caption:1",
  "ukadoc:list_shiori_resource:switchremotesstpbutton.caption:1",
  "ukadoc:spec_shiori3:X-SSTP-PassThru-_28_4efb_610f_306e_6587_5b57_5217_29_20_5bSSP_202.5.03_7e_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:X-SSTP-PassThru-_28_4efb_610f_306e_6587_5b57_5217_29_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1",
  "ukadoc:spec_sstp:request:1",
  "ukadoc:spec_sstp:response:1",
]
hand = [
  "ukadoc:descript_balloon:onlinemarker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:onlinemarker.interval_2c_5f85_6a5f_6642_9593:1",
  "ukadoc:descript_balloon:onlinemarker.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:onlinemarker.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmarker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:sstpmarker.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmarker.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmessage.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:sstpmessage.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:sstpmessage.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:sstpmessage.font.height_2c_30b5_30a4_30ba:1",
  "ukadoc:descript_balloon:sstpmessage.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:sstpmessage.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmessage.xr_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:sstpmessage.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_ghost:sstp.allowunspecifiedsend_2c_6570_5024:1",
  "ukadoc:list_shiori_event:OnSSTPBlacklisting:1",
  "ukadoc:list_shiori_event:OnSSTPBreak:1",
  "ukadoc:list_shiori_event:uniqueid:1",
  "ukadoc:list_shiori_resource:callsstpsendboxbutton.caption:1",
  "ukadoc:list_shiori_resource:switchblacklistingbutton.caption:1",
  "ukadoc:list_shiori_resource:switchlocalsstpbutton.caption:1",
  "ukadoc:list_shiori_resource:switchremotesstpbutton.caption:1",
  "ukadoc:spec_shiori3:X-SSTP-PassThru-_28_4efb_610f_306e_6587_5b57_5217_29_20_5bSSP_202.5.03_7e_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:X-SSTP-PassThru-_28_4efb_610f_306e_6587_5b57_5217_29_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1",
  "ukadoc:spec_sstp:request:1",
  "ukadoc:spec_sstp:response:1",
]
domains = ["assets", "shiori"]
foundation = "外から渡された要求をスクリプトとイベントに変える受け口"
breakage = "黙って壊れる"
themes = ["装い", "交わり"]
```

**成立に要る最小の基盤**: 外部のプログラムから届いた要求を読み、指定されたさくらスクリプトを再生するか SHIORI へイベントとして渡し、決められた形の応答を返せること。要求の共通ヘッダ（`Charset`・`Sender`・`SecurityLevel`・`SecurityOrigin`・`Option`・`ID`・`HWnd`・`ReceiverGhostHWnd`・`ReceiverGhostName`）を解釈できること。ゴーストを名指ししない要求を受けるかどうかを `sstp.allowunspecifiedsend,数値` で選べること。`X-SSTP-PassThru-` で始まるヘッダを、名前も中身も変えずに SHIORI へ通すこと。ゴーストが `uniqueid` で返した識別子（または FMO の識別 ID）を `ID` に添えた要求を、ゴースト内部の処理と同じ優先度で扱うこと。割り込みを断ったときに `OnSSTPBreak`、送り元を締め出したときに `OnSSTPBlacklisting` を送れること。外から来たメッセージであることを示す印と送り元の文字列を、バルーンの `sstpmarker.filename,ファイル名` 以下 3 個と `sstpmessage.x,座標 *1` 以下 8 個の設定どおりに描けること。

**欠けると壊れる既存ゴーストの振る舞い**: 外部のプログラムからゴーストに喋らせることができない。Web ページのボタンや常駐ツールからゴーストへ言葉を送る仕掛けは何も起こさず、メールの着信・天気・音楽の再生などをゴーストの口から知らせる連携ツールを使っている利用者には、その通知が 1 つも届かない。バルーンの作者が用意した「外から来た言葉」の印と送り元の表示も、出番が無いまま残る。

構成 id は 27 件で、うち機械の束から来たものが 0 件、人手で足したものが 27 件である（`hand` の行を数えた。タスク 3.5 が 4 件を足した後の値である）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（SSTP の 23 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### FMO（段階 D 相当）

```toml
[bundle."FMO"]
machine = []
members = [
  "ukadoc:list_shiori_resource:updatefmobutton.caption:1",
  "ukadoc:spec_fmo_mutex:32_30d0_30a4_30c8_306e_8b58_5225ID:1",
  "ukadoc:spec_fmo_mutex:FMO_306e_30b5_30a4_30ba:1",
  "ukadoc:spec_fmo_mutex:FMO_306e_540d_524d_3068_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:spec_fmo_mutex:_30ad_30fc_540d_30fb_5024:1",
  "ukadoc:spec_fmo_mutex:_30c7_30fc_30bf_672c_4f53:1",
  "ukadoc:spec_fmo_mutex:_30c7_30fc_30bf_7d42_7aef:1",
]
hand = [
  "ukadoc:list_shiori_resource:updatefmobutton.caption:1",
  "ukadoc:spec_fmo_mutex:32_30d0_30a4_30c8_306e_8b58_5225ID:1",
  "ukadoc:spec_fmo_mutex:FMO_306e_30b5_30a4_30ba:1",
  "ukadoc:spec_fmo_mutex:FMO_306e_540d_524d_3068_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:spec_fmo_mutex:_30ad_30fc_540d_30fb_5024:1",
  "ukadoc:spec_fmo_mutex:_30c7_30fc_30bf_672c_4f53:1",
  "ukadoc:spec_fmo_mutex:_30c7_30fc_30bf_7d42_7aef:1",
]
domains = ["shiori"]
foundation = "起動中のゴーストの情報を他のプログラムから読める共有メモリに載せる器"
breakage = "黙って壊れる"
themes = ["交わり", "装い"]
```

**成立に要る最小の基盤**: 起動中のゴースト 1 組ごとに 32 バイトの識別 ID を決め、`Sakura`（OS 依存・日本語 OS では Shift_JIS）と `SakuraUnicode`（UTF-8 固定）の 2 つの名前で 64KB 固定の共有メモリを確保できること。先頭 4 バイトに確保サイズをリトルエンディアンで書き、続けて「(識別ID).(キー名) とバイト値 1 と値と CR+LF」の行を並べ、最後をバイト値 0 で終えること。キーとして `path`・`hwnd`・`name`・`keroname`・`sakura.surface`・`kero.surface`・`kerohwnd`・`hwndlist`・`ghostpath` などを載せること。データ本体に使える 65531 バイトを超えそうなときは、1 組分を丸ごと書かないこと。メニューから FMO の整理・更新を行い、その名前をゴーストが返す `updatefmobutton.caption` から読めること。

**欠けると壊れる既存ゴーストの振る舞い**: 他のプログラムから「今どのゴーストが動いていて、どの窓がそのゴーストのものか」を知る手段が無い。伺かの周辺で使われている外部プログラム——SSTP を送る道具、起動中ゴーストの一覧を出すツール、ゴーストの窓を狙って動く補助ツール——は areka のゴーストを 1 体も見つけられない。利用者から見ると、SSP では使えていた道具がこのベースウェアの上でだけ「ゴーストが起動していません」と言って止まる。

構成 id は 7 件で、うち機械の束から来たものが 0 件、人手で足したものが 7 件である（`hand` の行を数えた）。**この束は人手のみである**——`machine` が空配列で、核になる機械の束を持たない（FMO の 7 件はいずれも台帳の関連を 1 本も持たないので、機械の束に現れない）。

### PLUGIN（段階 D 相当）

```toml
[bundle."PLUGIN"]
machine = [
  "ukadoc:descript_plugin:otherghosttalk_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:list_plugin_event:_5c_21_5braiseplugin_5d_304a_3088_3073_5c_21_5bnotifyplugin_5d_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30:1",
  "ukadoc:list_plugin_event:balloonpathlist:1",
  "ukadoc:list_plugin_event:ghostpathlist:1",
  "ukadoc:list_plugin_event:headlinepathlist:1",
  "ukadoc:list_plugin_event:pluginpathlist:1",
  "ukadoc:list_plugin_event:property.get:1",
  "ukadoc:list_sakura_script:_5c_21_5bnotifyplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_:1",
  "ukadoc:list_sakura_script:_5c_21_5btimernotifyplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_30:1",
]
members = [
  "ukadoc:descript_plugin:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_plugin:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_plugin:craftmanurl_2cURL:1",
  "ukadoc:descript_plugin:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_plugin:filename_2cdll:1",
  "ukadoc:descript_plugin:homeurl_2cURL:1",
  "ukadoc:descript_plugin:id_2cID:1",
  "ukadoc:descript_plugin:name_2c_30d7_30e9_30b0_30a4_30f3_540d:1",
  "ukadoc:descript_plugin:otherghosttalk_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_plugin:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_plugin:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_plugin:type_2c_7a2e_5225:1",
  "ukadoc:list_plugin_event:OnChoiceSelect_28Ex_29_2fOnAnchorSelect_28Ex_29_2f_5cq_7b49_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30f3_:1",
  "ukadoc:list_plugin_event:OnGhostBoot:1",
  "ukadoc:list_plugin_event:OnGhostExit:1",
  "ukadoc:list_plugin_event:OnGhostInfoUpdate:1",
  "ukadoc:list_plugin_event:OnOtherGhostTalk:1",
  "ukadoc:list_plugin_event:_5c_21_5braiseplugin_5d_304a_3088_3073_5c_21_5bnotifyplugin_5d_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30:1",
  "ukadoc:list_plugin_event:balloonpathlist:1",
  "ukadoc:list_plugin_event:ghostpathlist:1",
  "ukadoc:list_plugin_event:headlinepathlist:1",
  "ukadoc:list_plugin_event:pluginpathlist:1",
  "ukadoc:list_plugin_event:property.get:1",
  "ukadoc:list_plugin_event:property.set:1",
  "ukadoc:list_plugin_event:version:1",
  "ukadoc:list_propertysystem:history.plugin.count:1",
  "ukadoc:list_propertysystem:history.plugin.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.plugin_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:pluginlist.count:1",
  "ukadoc:list_propertysystem:pluginlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:pluginlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bnotifyplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cpluginexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5braiseplugin_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305f_306f_540d_524d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2:1",
  "ukadoc:list_sakura_script:_5c_21_5btimernotifyplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_30:1",
  "ukadoc:list_sakura_script:_5c_21_5btimerraiseplugin_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30d7_30e9_30b0_30a4_30f3_306eID_307e_305:1",
  "ukadoc:list_shiori_event:OnNotifyPluginFailure:1",
  "ukadoc:list_shiori_event:OnRaisePluginFailure:1",
  "ukadoc:list_shiori_event:pluginpathlist:1",
  "ukadoc:list_shiori_resource:pluginhistorybutton.caption:1",
  "ukadoc:list_shiori_resource:pluginrootbutton.caption:1",
  "ukadoc:spec_plugin",
]
hand = [
  "ukadoc:descript_plugin:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_plugin:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_plugin:craftmanurl_2cURL:1",
  "ukadoc:descript_plugin:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_plugin:filename_2cdll:1",
  "ukadoc:descript_plugin:homeurl_2cURL:1",
  "ukadoc:descript_plugin:id_2cID:1",
  "ukadoc:descript_plugin:name_2c_30d7_30e9_30b0_30a4_30f3_540d:1",
  "ukadoc:descript_plugin:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_plugin:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_plugin:type_2c_7a2e_5225:1",
  "ukadoc:list_plugin_event:OnChoiceSelect_28Ex_29_2fOnAnchorSelect_28Ex_29_2f_5cq_7b49_306b_6307_5b9a_3055_308c_305f_4efb_610f_540d_30a4_30d9_30f3_:1",
  "ukadoc:list_plugin_event:OnGhostBoot:1",
  "ukadoc:list_plugin_event:OnGhostExit:1",
  "ukadoc:list_plugin_event:OnGhostInfoUpdate:1",
  "ukadoc:list_plugin_event:version:1",
  "ukadoc:list_propertysystem:history.plugin.count:1",
  "ukadoc:list_propertysystem:history.plugin.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.plugin_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:pluginlist.count:1",
  "ukadoc:list_propertysystem:pluginlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cpluginexplorer_5d:1",
  "ukadoc:list_shiori_resource:pluginhistorybutton.caption:1",
  "ukadoc:list_shiori_resource:pluginrootbutton.caption:1",
  "ukadoc:spec_plugin",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "外部 DLL を読み込んで要求と応答を往復させるホスト"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: `descript.txt` と DLL の 2 つでできたプラグインを読み込み、`GET`／`NOTIFY` の `PLUGIN/2.0` 要求（`ID`・`Charset`・`Sender`・`Reference*`）を送って応答（`Target`・`Event`・`EventOption`・`Reference*`・`Script`・`ScriptOption`）を受け取れること。応答の `Script` は、ゴーストがそのイベントに反応しなかったときの既定の台詞として再生すること。`Target` が指すゴーストへ届け、`__SYSTEM_ALL_GHOST__` のときは起動中の全ゴーストへ届けること。プラグインの設定を `descript.txt` の 12 個のキー（`id,ID`・`name,プラグイン名`・`filename,dll`・`type,種別`・`otherghosttalk,オプション` ほか）から読めること。ゴーストの起動・終了・情報の更新・他ゴーストの発話・選択肢とアンカーの選択・秒の刻みをプラグインへ知らせ、`version`・`balloonpathlist`・`ghostpathlist`・`headlinepathlist`・`pluginpathlist`・`property.get`・`property.set` の照会に答えられること。台詞の中の `\![raiseplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]`・`\![notifyplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` と時間差の 2 つでプラグインを名指しして呼べ、届かなければ `OnRaisePluginFailure`・`OnNotifyPluginFailure` を返すこと。入っているプラグインの一覧と履歴を `pluginlist`・`history.plugin` で読め、`pluginlist(…).ext.拡張プロパティ名` を問われたらプラグインへ取り次げること。プラグインの置き場を開く窓を `\![open,pluginexplorer]` で出し、メニューの「プラグイン」と「プラグイン履歴」の名前をゴーストから読めること。

**欠けると壊れる既存ゴーストの振る舞い**: プラグインを入れても何も起こらない。カレンダー・時計・付箋のような常駐の小道具は画面に出ず、プラグインからの合図に反応する台詞を書いた作品はその分岐に 1 度も入らない。プラグイン側が用意した既定の台詞も再生されないので、利用者から見るとプラグインを入れる前と後で画面が何も変わらない。プラグインの一覧を出すメニューも空のままになる。

構成 id は 44 件で、うち機械の束から来たものが 18 件、人手で足したものが 26 件である（`hand` の行を数えた）。さくらスクリプトの `\![effect,プラグイン名,速度倍率,パラメータ]`・`\![effect2,追加サーフェスID,プラグイン名,速度倍率,パラメータ]`・`\![filter,プラグイン名,起動時間,パラメータ]`・`\![filter]` はこの束に入れない。正典の本文は前者を「サーフェスをプラグインにより変化させる。」と述べており、指しているのは絵を加工する仕掛けであって、`PLUGIN/2.0` の要求と応答を往復させる相手ではない。段 2 のタスク 3.5 が名前付き束「サーフェスアニメーション」へ入れた。

### リンク（段階 D 相当）

```toml
[bundle."リンク"]
machine = [
  "ukadoc:descript_ghost:char_2a.name_2c_540d_524d:1",
]
members = [
  "ukadoc:list_shiori_event:OnXUkagakaLinkOpen:1",
  "ukadoc:spec_web:x-ukagaka-link_3atype_3devent_26ghost_3d_28_30b4_30fc_30b9_30c8_540d_29_26info_3d_28_8ffd_52a0_60c5_5831_29:1",
  "ukadoc:spec_web:x-ukagaka-link_3atype_3dhomeurl_26url_3d_28_30a8_30f3_30b3_30fc_30c9_6e08URL_29:1",
  "ukadoc:spec_web:x-ukagaka-link_3atype_3dinstall_26url_3d_28_30a8_30f3_30b3_30fc_30c9_6e08URL_29:1",
]
hand = [
  "ukadoc:spec_web:x-ukagaka-link_3atype_3devent_26ghost_3d_28_30b4_30fc_30b9_30c8_540d_29_26info_3d_28_8ffd_52a0_60c5_5831_29:1",
  "ukadoc:spec_web:x-ukagaka-link_3atype_3dhomeurl_26url_3d_28_30a8_30f3_30b3_30fc_30c9_6e08URL_29:1",
  "ukadoc:spec_web:x-ukagaka-link_3atype_3dinstall_26url_3d_28_30a8_30f3_30b3_30fc_30c9_6e08URL_29:1",
]
domains = ["shiori"]
foundation = "OS の URL 関連づけを受け取ってゴーストへ渡す経路"
breakage = "黙って壊れる"
themes = ["交わり"]
```

**成立に要る最小の基盤**: OS に `x-ukagaka-link:` で始まる URL の関連づけを登録し、3 つの形を処理できること。`type=install&url=(エンコード済URL)` は URL デコードした先の nar を取り込む。`type=homeurl&url=(エンコード済URL)` はその URL を更新先として扱い、ネットワーク更新と同じ手順でファイル群を取ってから取り込む。`type=event&ghost=(ゴースト名)&info=(追加情報)` は `descript.txt` の `name` か `sakura.name` で名指ししたゴーストへ `OnXUkagakaLinkOpen` を送り、`info=` の中身を URL デコードして Reference0 に載せる。文字コードは UTF-8 固定で URL エンコード済みとして読み、この経路で来たイベントの SecurityLevel は必ず `external` として扱うこと。

**欠けると壊れる既存ゴーストの振る舞い**: Web ページに置かれた「このゴーストを入れる」「この話を始める」のリンクを押しても何も起こらない。配布サイトの 1 クリック導入は使えず、利用者は nar を自分で保存してから窓へ落とす手順に戻る。ページから合図を送って特定の場面を始めさせる仕掛け（イベント名を URL に書く類い）も届かないので、Web と連動した演目を用意した作品はその演目に入れない。

構成 id は 4 件で、うち機械の束から来たものが 1 件、人手で足したものが 3 件である（`hand` の行を数えた）。

### 外部アプリ（段階 E 相当）

```toml
[bundle."外部アプリ"]
machine = [
  "ukadoc:list_sakura_script:_5c7:1",
  "ukadoc:list_sakura_script:_5c_21_5bcancel_2cwebsocket_2cURL_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cnslookup_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cping_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._:1",
  "ukadoc:list_shiori_event:OnMusicPlay:1",
]
members = [
  "ukadoc:list_sakura_script:_5c_21_5bbiff_28_2c_30a2_30ab_30a6_30f3_30c8_540d_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bcancel_2chttp_2cURL_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bcancel_2cwebsocket_2cURL_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cwebsocket_2cURL_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bcreate_2cshortcut_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-delete_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-head_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-options_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-patch_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3.:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-put_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cnslookup_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cping_2c_30d1_30e9_30e1_30fc_30bf1_2c_30d1_30e9_30e1_30fc_30bf2_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2crss-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cwebsocket_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecutesntp_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2carchiveviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ceditor_2c_30d5_30a1_30a4_30eb_2c_8868_793a_884c_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cexplorer_2c_30d5_30a1_30a4_30eb_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cfile_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cmailer_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cpictureviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket-binary_2cURL_2cbase64data_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket_2cURL_2cdata1_2cdata2_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c__c:1",
  "ukadoc:list_sakura_script:_5c__t:1",
  "ukadoc:list_sakura_script:_5cj_5bID_5d:1",
  "ukadoc:list_sakura_script:_5cm_5bumsg_2cwparam_2clparam_5d:1",
  "ukadoc:list_shiori_event:OnArchiveViewerOpen:1",
  "ukadoc:list_shiori_event:OnBIFF2Complete:1",
  "ukadoc:list_shiori_event:OnBIFFBegin:1",
  "ukadoc:list_shiori_event:OnBIFFComplete:1",
  "ukadoc:list_shiori_event:OnBIFFFailure:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPComplete:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPFailure:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPProgress:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPSSLInfo:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPStreaming:1",
  "ukadoc:list_shiori_event:OnExecuteRSSComplete:1",
  "ukadoc:list_shiori_event:OnExecuteRSSFailure:1",
  "ukadoc:list_shiori_event:OnExecuteRSS_SSLInfo:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketClose:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketFailure:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketOpen:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketReceive:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketReconnect:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocket_SSLInfo:1",
  "ukadoc:list_shiori_event:OnMediaPlayerOpen:1",
  "ukadoc:list_shiori_event:OnMusicPlay:1",
  "ukadoc:list_shiori_event:OnMusicPlayEx:1",
  "ukadoc:list_shiori_event:OnNSLookupComplete:1",
  "ukadoc:list_shiori_event:OnNSLookupFailure:1",
  "ukadoc:list_shiori_event:OnPictureViewerOpen:1",
  "ukadoc:list_shiori_event:OnPingComplete:1",
  "ukadoc:list_shiori_event:OnPingProgress:1",
  "ukadoc:list_shiori_event:OnSNTPBegin:1",
  "ukadoc:list_shiori_event:OnSNTPCompare:1",
  "ukadoc:list_shiori_event:OnSNTPCompareEx:1",
  "ukadoc:list_shiori_event:OnSNTPCorrect:1",
  "ukadoc:list_shiori_event:OnSNTPCorrectEx:1",
  "ukadoc:list_shiori_event:OnSNTPFailure:1",
  "ukadoc:list_shiori_event:OnVideoPlayEx:1",
  "ukadoc:list_shiori_event:configuredbiffname:1",
  "ukadoc:list_shiori_resource:biffallbutton.caption:1",
  "ukadoc:list_shiori_resource:biffbutton.caption:1",
  "ukadoc:list_shiori_resource:sntpbutton.caption:1",
  "ukadoc:list_shiori_resource:switchautobiffbutton.caption:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bbiff_28_2c_30a2_30ab_30a6_30f3_30c8_540d_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bcancel_2chttp_2cURL_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bcreate_2cshortcut_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-delete_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-get_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-head_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-options_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-patch_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3.:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-post_2cURL_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2chttp-put_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3...:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cwebsocket_2cURL_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2carchiveviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cbrowser_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ceditor_2c_30d5_30a1_30a4_30eb_2c_8868_793a_884c_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cexplorer_2c_30d5_30a1_30a4_30eb_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cfile_2c_30d5_30a1_30a4_30eb_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cmailer_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cpictureviewer_2c_28_30d5_30a1_30a4_30eb_540d_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket-binary_2cURL_2cbase64data_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsend_2cwebsocket_2cURL_2cdata1_2cdata2_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c__c:1",
  "ukadoc:list_sakura_script:_5c__t:1",
  "ukadoc:list_sakura_script:_5cj_5bID_5d:1",
  "ukadoc:list_sakura_script:_5cm_5bumsg_2cwparam_2clparam_5d:1",
  "ukadoc:list_shiori_event:OnArchiveViewerOpen:1",
  "ukadoc:list_shiori_event:OnBIFF2Complete:1",
  "ukadoc:list_shiori_event:OnBIFFBegin:1",
  "ukadoc:list_shiori_event:OnBIFFComplete:1",
  "ukadoc:list_shiori_event:OnBIFFFailure:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPComplete:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPFailure:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPProgress:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPSSLInfo:1",
  "ukadoc:list_shiori_event:OnExecuteHTTPStreaming:1",
  "ukadoc:list_shiori_event:OnExecuteRSS_SSLInfo:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketFailure:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketOpen:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketReceive:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocketReconnect:1",
  "ukadoc:list_shiori_event:OnExecuteWebSocket_SSLInfo:1",
  "ukadoc:list_shiori_event:OnMediaPlayerOpen:1",
  "ukadoc:list_shiori_event:OnMusicPlayEx:1",
  "ukadoc:list_shiori_event:OnPictureViewerOpen:1",
  "ukadoc:list_shiori_event:OnSNTPCompareEx:1",
  "ukadoc:list_shiori_event:OnSNTPCorrect:1",
  "ukadoc:list_shiori_event:OnSNTPCorrectEx:1",
  "ukadoc:list_shiori_event:OnSNTPFailure:1",
  "ukadoc:list_shiori_event:OnVideoPlayEx:1",
  "ukadoc:list_shiori_event:configuredbiffname:1",
  "ukadoc:list_shiori_resource:biffallbutton.caption:1",
  "ukadoc:list_shiori_resource:biffbutton.caption:1",
  "ukadoc:list_shiori_resource:sntpbutton.caption:1",
  "ukadoc:list_shiori_resource:switchautobiffbutton.caption:1",
]
domains = ["sakura-script", "shiori"]
foundation = "外のプログラムと網の相手を呼び出して結果を待ち受ける経路"
breakage = "黙って壊れる"
themes = ["触れ合い", "装い", "交わり", "気配り"]
```

**成立に要る最小の基盤**: 台詞の中から areka の外にある相手を呼び、その結果をイベントで受け取れること。本体設定の「外部アプリ」に登録したブラウザ・メーラ・エディタと、エクスプローラ・関連づけられたアプリ・ショートカットの作成を `\![open,browser,パラメータ]`・`\![open,mailer,パラメータ]`・`\![open,editor,ファイル,表示行]`・`\![open,explorer,ファイル]`・`\![open,file,ファイル名]`・`\![create,shortcut]` で起こせること。HTTP の 8 つのメソッドと RSS と WebSocket を `\![execute,http-get,URL,オプション,オプション,オプション...]` 以下 10 個のタグで呼び、`\![cancel,http,URL]`・`\![cancel,websocket,URL]`・`\![close,websocket,URL]` で止められ、完了・失敗・途中経過・切断・再接続・証明書の情報を `OnExecuteHTTPComplete` 以下 14 個のイベントで返せること。`\![execute,ping,パラメータ1,パラメータ2,...]`・`\![execute,nslookup,パラメータ1,パラメータ2,...]` と時計合わせの `\![executesntp]` を呼び、`OnPingComplete`・`OnPingProgress`・`OnNSLookupComplete`・`OnNSLookupFailure` と `OnSNTPBegin` 以下 6 個で結果を返せること。`\![biff(,アカウント名)]` でメールの着信を調べて `OnBIFFBegin`・`OnBIFFComplete`・`OnBIFF2Complete`・`OnBIFFFailure` を返し、調べる先の名前を `configuredbiffname` でゴーストから受け取れること。対応する再生ソフトで鳴っている曲や動画の情報を `OnMusicPlay`・`OnMusicPlayEx`・`OnVideoPlayEx`・`OnMediaPlayerOpen` で受け取り、画像と書庫の中身を見る窓を `\![open,pictureviewer,(ファイル名)]`・`\![open,archiveviewer,(ファイル名)]` で開いて `OnPictureViewerOpen`・`OnArchiveViewerOpen` を返せること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴーストがパソコンの外の世界に触れる演目が動かない。台詞に添えたリンクからブラウザが開かず、「調べてくるね」と言って Web から取ってきた内容を読み上げる作品は、取得の結果が返らないまま次へ進む。天気やニュースを取りに行く辞書、時計を合わせる演目、メールの着信を知らせる台詞、いま鳴っている曲名に反応する台詞は、どれもその分岐に 1 度も入らない。利用者から見ると、ゴーストはパソコンの中の出来事しか知らず、外の出来事には何も反応しない。

構成 id は 72 件で、うち機械の束から来たものが 18 件、人手で足したものが 54 件である（`hand` の行を数えた。タスク 3.5 が 4 件を足した後の値である）。

### 開発者機能（段階 E 相当）

```toml
[bundle."開発者機能"]
machine = [
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
]
members = [
  "ukadoc:dev_nar",
  "ukadoc:dev_shell_error",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2ccreatenar_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cdumpsurface_2c_30c7_30a3_30ec_30af_30c8_30ea_2c_30b9_30b3_30fc_30d7ID_2c_30b5_30fc_30d5_30a7_30b9_30e:1",
  "ukadoc:list_sakura_script:_5c_21_5bload_2cshiori_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdeveloper_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cerrorlog_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cshiorirequest_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2csurfacetest_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cdescript_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cshiori_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cshioridebugmode_2c_28true_2ffalse_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunload_2cshiori_5d:1",
  "ukadoc:list_shiori_event:OnNarCreated:1",
  "ukadoc:list_shiori_event:OnNarCreating:1",
  "ukadoc:list_shiori_event:enable_debug:1",
  "ukadoc:list_shiori_event:enable_log:1",
  "ukadoc:list_shiori_resource:debugballoonbutton.caption:1",
  "ukadoc:list_shiori_resource:log_path:1",
  "ukadoc:list_shiori_resource:scriptlogbutton.caption:1",
]
hand = [
  "ukadoc:dev_shell_error",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cdumpsurface_2c_30c7_30a3_30ec_30af_30c8_30ea_2c_30b9_30b3_30fc_30d7ID_2c_30b5_30fc_30d5_30a7_30b9_30e:1",
  "ukadoc:list_sakura_script:_5c_21_5bload_2cshiori_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdeveloper_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cerrorlog_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cshiorirequest_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2csurfacetest_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cshiori_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cshioridebugmode_2c_28true_2ffalse_29_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunload_2cshiori_5d:1",
  "ukadoc:list_shiori_event:enable_debug:1",
  "ukadoc:list_shiori_event:enable_log:1",
  "ukadoc:list_shiori_resource:debugballoonbutton.caption:1",
  "ukadoc:list_shiori_resource:log_path:1",
  "ukadoc:list_shiori_resource:scriptlogbutton.caption:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "動いているゴーストの中身を覗き、作り直しをその場で反映する経路"
breakage = "黙って壊れる"
themes = ["装い", "記憶"]
```

**成立に要る最小の基盤**: ゴーストの作り手が手元で使う道具を、台詞と設定から呼べること。`\![execute,createnar]` で配布用の nar を作り、その前と後に `OnNarCreating`・`OnNarCreated` を送れること。開発者向けの窓・エラーログ・サーフェスの確認・SHIORI とのやりとりの記録を `\![open,developer]`・`\![open,errorlog]`・`\![open,surfacetest]`・`\![open,shiorirequest]` で開けること。`\![set,shioridebugmode,(true/false)]` で SHIORI のデバッグ表示を切り替え、`\![execute,dumpsurface,ディレクトリ,スコープID,サーフェスリスト,prefix,イベントID,ゼロ位置切り出し]` でサーフェスを画像として書き出せること。`\![reload,descript,パラメータ]` で設定を読み直し、`\![load,shiori]`・`\![unload,shiori]`・`\![reload,shiori]` で SHIORI を入れ直せること。ゴーストが返す `enable_debug`・`enable_log`・`log_path` に従って記録を残し、デバッグバルーンとスクリプトログのメニュー名をゴーストから読めること。`ukadoc:dev_nar` が定める配布用ファイルの作り方と、`ukadoc:dev_shell_error` が並べるエラーメッセージの直し方に沿えること。

**欠けると壊れる既存ゴーストの振る舞い**: 里々や YAYA でゴーストを書く人が、areka の上では作りかけを試せない。辞書を直しても読み直しの指示が効かないので、ひと手直しごとにゴーストを起動し直すことになる。エラーが出ても中身を見る窓が開かないので、どこで失敗したのかを画面から知る手段が無い。出来上がったものを配布用の nar にまとめる手順も動かないので、areka だけで作って配るところまでが閉じない。

構成 id は 20 件で、うち機械の束から来たものが 5 件、人手で足したものが 15 件である（`hand` の行を数えた）。

### トランスレータ（段階 E 相当）

```toml
[bundle."トランスレータ"]
machine = [
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
]
members = [
  "ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:sstp.alwaystranslate_2c_6570_5024:1",
  "ukadoc:list_sakura_script:_5c_21_5bload_2cmakoto_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cmakoto_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunload_2cmakoto_5d:1",
  "ukadoc:list_shiori_event:OnTranslate:1",
  "ukadoc:manual_translator",
]
hand = [
  "ukadoc:descript_ghost:sstp.alwaystranslate_2c_6570_5024:1",
  "ukadoc:list_sakura_script:_5c_21_5bload_2cmakoto_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2cmakoto_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunload_2cmakoto_5d:1",
  "ukadoc:list_shiori_event:OnTranslate:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "外部 DLL を読み込んで要求と応答を往復させるホスト"
breakage = "黙って壊れる"
themes = ["掛け合い"]
```

**成立に要る最小の基盤**: `descript.txt` の `makoto,ファイル名` が指す DLL を、ゴースト側は `ghost/master` の直下、シェル側は各シェルのフォルダの直下から読み込めること。両方にあるときはゴースト側を先に、続けてシェル側を通すこと。変換を掛けるのはバルーンへ表示する直前で、SHIORI と MAKOTO の両方で変換するときは SHIORI が先であること。`\![load,makoto]`・`\![reload,makoto]`・`\![unload,makoto]` で読み込み直せること。SHIORI が MAKOTO を兼ねる口として、ベースウェアが受け取ったスクリプトを環境変数の展開の後にもう 1 度 `OnTranslate` として SHIORI へ渡し、返ったものを最終のスクリプトとして再生すること（`OnTranslate` 自身では再び起こさない）。`OnTranslate` の Reference1 に、そのスクリプトの出所（`communicate`・`sstp-send`・`owned`・`remote`・`notranslate`・`plugin-script`・`plugin-event`）を載せること。`sstp.alwaystranslate,1` を書いたゴーストでは、SSTP のオプションに関わらず常に変換を通すこと。

**欠けると壊れる既存ゴーストの振る舞い**: 台詞を後から書き換える仕掛けが働かない。標準語で書いた辞書に方言や口癖をかぶせて話し方を作っている作品は、素の文面のまま喋る。シェルを着せ替えると口調が変わる作りの作品では、絵だけが変わって言葉が変わらない。伏せ字や誤変換の演出をトランスレータで作っている作品も、その加工が全部抜け落ちた文面が出る。SHIORI 側で `OnTranslate` を使って台詞を整えている辞書は、その処理が 1 度も呼ばれない。

構成 id は 7 件で、うち機械の束から来たものが 2 件、人手で足したものが 5 件である（`hand` の行を数えた）。

### ヘッドライン（段階 E 相当）

```toml
[bundle."ヘッドライン"]
machine = [
  "ukadoc:list_plugin_event:headlinepathlist:1",
]
members = [
  "ukadoc:descript_headline:alwaysdisplay_2c_6570_5024:1",
  "ukadoc:descript_headline:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_headline:dllname_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_headline:homeurl_2cURL:1",
  "ukadoc:descript_headline:name_2c_30d8_30c3_30c9_30e9_30a4_30f3_540d:1",
  "ukadoc:descript_headline:openurl_2cURL:1",
  "ukadoc:descript_headline:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_headline:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_headline:url_2cURL:1",
  "ukadoc:list_propertysystem:headlinelist.count:1",
  "ukadoc:list_propertysystem:headlinelist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:headlinelist_28_30d8_30c3_30c9_30e9_30a4_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.headline.count:1",
  "ukadoc:list_propertysystem:history.headline.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.headline_28_30d8_30c3_30c9_30e9_30a4_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cheadline_2c_30d8_30c3_30c9_30e9_30a4_30f3_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cheadlinesensorexplorer_5d:1",
  "ukadoc:list_shiori_event:OnHeadlinesense.OnFind:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseBegin:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseComplete:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseFailure:1",
  "ukadoc:list_shiori_event:OnRSSBegin:1",
  "ukadoc:list_shiori_event:OnRSSComplete:1",
  "ukadoc:list_shiori_event:OnRSSFailure:1",
  "ukadoc:list_shiori_event:headlinepathlist:1",
  "ukadoc:list_shiori_resource:headlinesensehistorybutton.caption:1",
  "ukadoc:list_shiori_resource:headlinesenserootbutton.caption:1",
  "ukadoc:list_shiori_resource:switchautoheadlinesensebutton.caption:1",
  "ukadoc:spec_headline",
]
hand = [
  "ukadoc:descript_headline:alwaysdisplay_2c_6570_5024:1",
  "ukadoc:descript_headline:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_headline:dllname_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_headline:homeurl_2cURL:1",
  "ukadoc:descript_headline:name_2c_30d8_30c3_30c9_30e9_30a4_30f3_540d:1",
  "ukadoc:descript_headline:openurl_2cURL:1",
  "ukadoc:descript_headline:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_headline:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_headline:url_2cURL:1",
  "ukadoc:list_propertysystem:headlinelist.count:1",
  "ukadoc:list_propertysystem:headlinelist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:headlinelist_28_30d8_30c3_30c9_30e9_30a4_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.headline.count:1",
  "ukadoc:list_propertysystem:history.headline.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.headline_28_30d8_30c3_30c9_30e9_30a4_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cheadline_2c_30d8_30c3_30c9_30e9_30a4_30f3_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cheadlinesensorexplorer_5d:1",
  "ukadoc:list_shiori_event:OnHeadlinesense.OnFind:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseBegin:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseComplete:1",
  "ukadoc:list_shiori_event:OnHeadlinesenseFailure:1",
  "ukadoc:list_shiori_event:OnRSSBegin:1",
  "ukadoc:list_shiori_event:OnRSSComplete:1",
  "ukadoc:list_shiori_event:OnRSSFailure:1",
  "ukadoc:list_shiori_resource:headlinesensehistorybutton.caption:1",
  "ukadoc:list_shiori_resource:headlinesenserootbutton.caption:1",
  "ukadoc:list_shiori_resource:switchautoheadlinesensebutton.caption:1",
  "ukadoc:spec_headline",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "外部 DLL を読み込んで要求と応答を往復させるホスト"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: `descript.txt` と `execute` を出す DLL の 2 つでできたヘッドラインセンサを読み込めること。`url,URL` が指すページを取り込んで一時ファイルに落とし、`GET Headline HEADLINE/2.0`（`Charset`・`Sender`・`Option`・`Path`）で DLL に渡して `Headline:` の行を受け取れること。`GET Version HEADLINE/2.0` で版を尋ねられること。取り込みは古いファイルと新しいファイルに対して 2 回呼び、どちらが先かに頼らないこと。`Option: url` を付けたときは「内容とバイト値 1 と URL」の形の応答を受け取り、URL もゴーストへ渡すこと。開始・読み上げる内容が見つかった・正常終了・失敗を `OnHeadlinesenseBegin`・`OnHeadlinesense.OnFind`・`OnHeadlinesenseComplete`・`OnHeadlinesenseFailure` で送れること。`\![execute,headline,ヘッドライン名]` で名指しして走らせ、`\![open,headlinesensorexplorer]` で置き場を開けること。入っているヘッドラインの一覧と履歴を `headlinelist`・`history.headline` で読め、置き場を `headlinepathlist` でゴーストへ知らせられること。`alwaysdisplay,数値` に 1 を書いたときは更新が無くても読み上げること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴーストがニュースや更新情報を読み上げなくなる。ヘッドラインセンサを入れて「新着があったら教えて」という使い方をしている利用者には、何も知らされない。読み上げた内容に感想を付ける辞書を書いた作品は、その台詞に 1 度も到達しない。メニューの「ヘッドライン」の項目も空のままで、利用者は入れたセンサが働いているかどうかを画面から確かめられない。

構成 id は 29 件で、うち機械の束から来たものが 1 件、人手で足したものが 28 件である（`hand` の行を数えた。タスク 3.5 が 3 件を足した後の値である）。**brief の段階表（要件 5.2）はこの束を段階 E に置く理由を「テーマ 0」と書いているが、台帳を引き直すとテーマは 0 ではない。** 構成 id 29 件の `values` を全部読むと、`headlinesenserootbutton.caption`・`headlinesensehistorybutton.caption`・`switchautoheadlinesensebutton.caption` の 3 件が「装い」を持ち、残る 26 件が空である。ここでは配置を変えず、事実だけを書き留める（要件 5.5 に従い、段階の食い違いは段 3 の `briefing.md` が裁定候補として扱う）。

### 段 2 のこのタスク（3.5）が入れたもの

段 2 のこのタスク（3.5）が引き受けるのは、台帳の関連を 1 本も持たない項目である。**この文書で
「関連 0 本」と言うときは、4 台帳の `links` を向き無しのグラフに直したときに端を 1 本も持たず、
かつ状態が対象 4 状態のいずれかであることを指す**（定義は「このタスクへ回された id の始末」節の
末尾に置いた。要件 4.2 が並べるもう 1 つの数え方——自分の行の `links` が空・全状態——ではない）。
台帳の関連は 1 本も足していない（要件 3.6 のとおり、帰属はこの文書だけで決める）。

このタスクは、束へ入れる id を **927** 件足した。内訳は、名前付き束を新たに **33** 立てて
**797** 件を入れ、タスク 3.2〜3.4 が名付けた 30 束のうち **13** 束へ **130** 件を足した
（数え方: 下の 33 の囲みの `members` を数え、既存 30 束については囲みを書き換える前後の
`members` の差を数えた。合計はドメイン別に assets **396** 件・property **118** 件・sakura-script **192** 件・shiori **221** 件）。足した id はすべて `hand` に載せた——
ただし「窓の配置と重なり」の 5 件だけは、その束が `machine` に引いた機械の束の構成 id でもあるので
`hand` に入れていない。同じ id が 2 つの束に現れることは **0 件**である（数え方: 63 束すべての
`members` を集めて重複を数えた）。状態が `alias` の id と `not-applicable` の id は **0 件**である
（同じ集合を台帳の `status` で引き直して数えた）。

**関連 0 本の対象項目の始末**（完了の判定に使う 3 つの数）。関連 0 本の対象項目は今の台帳で
**1,126** 件である（数え方: 4 台帳の `links` をすべて読んで向き無しのグラフに直し、端を 1 本も
持たない id のうち状態が `implemented`・`vocabulary-only`・`degraded`・`absent` のものを数えた。
タスク 3.5 の表題が挙げる 1,127 は着手時の値で、段 1 が `links` を 1 本足したので 1 件少ない）。
その 1,126 件の行き先は次の 3 つに分かれ、足すと 1,126 件になる。

| 行き先 | 件数 | 数え方 |
| --- | ---: | --- |
| タスク 3.2〜3.4 が既に束へ入れていた | 415 | 30 束の `members` と関連 0 本の集合の共通部分を数えた |
| タスク 3.5 が束へ入れた | 710 | このタスクが足した 927 件のうち関連 0 本のものを数えた |
| タスク 3.5 が単独項目の候補にした | 1 | 下の「単独項目に落とす候補」節の箇条書きのうち関連 0 本のものを数えた |

このタスクが足した 927 件のうち、関連 0 本でないもの（向き無しの端を 1 本以上持つもの）は
**217** 件である。これらは上の表の母集団には入らないが、名前付き束へ id を足せるのはこのタスクが
最後なので（タスク 3.6 が書くのは `members` が id 1 つの単独項目である）、同じ機構の束へ一緒に入れた。

**束の切り方の根拠。** 新たに立てた 33 束の切れ目は、台帳の備考が項目ごとに書いている
「先に要る仕組み」（assets 台帳の `束:` の行）と「群」（sakura-script 台帳の `[群: …]` の行）を
そのまま境界に使った。どちらの台帳も、項目 1 件ごとに「これが動くには何が先に要るか」を逐語で
書いており、要件 4.1 ⑸ が求める「成立に要る最小の基盤」と同じ問いに答えている。shiori 台帳と
property 台帳にはその欄が無いので、備考の「無いと失うもの」「内容」「向き」の逐語と、正典の
見出しそのものを根拠にした。根拠を作れない項目は束へ入れず、下の候補の節へ落とした。

**このタスクへ回された id の着地。** タスク 3.1・3.2・3.3・3.4 がこのタスクへ回した id は、
すべて下の束のどれかに入っている。重なり順の 4 件（`ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1`・
`ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1`・
`ukadoc:list_propertysystem:currentghost.seriko.zorder:1`・`ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1`）は
4 件とも「窓の配置と重なり」に入れた。タスク 3.3 が名指しした察知の形は「OS の変化の察知」を
軸に、機構の違う `OnOffscreen`（画面からのはみ出し）を「窓の配置と重なり」へ、ごみ箱の 3 件を
「ごみ箱」へ、`OnWallpaperChange` を「壁紙」へ、通知領域の 2 件と `\![set,tasktrayicon,…]` を
「通知領域」へ分けた。タスク 3.2 が回した `OnCacheSuspend`・`OnCacheRestore` は「休止と復帰」、
`\4`・`\5` は「窓の配置と重なり」、`\6` は「組み込みの置換語」に入れた。タスク 3.4 が回した
`ukadoc:list_shiori_event:hwnd:1` は「窓の配置と重なり」、`%lastghostname` は「インストール」、
`\![effect,…]`・`\![effect2,…]`・`\![filter,…]`・`\![filter]` は「サーフェスアニメーション」、
書庫の 6 件は「書庫」、予定表の 9 件は「予定表」、ごみ箱と壁紙は上のとおりである。

各束の囲みは、上の 30 束と同じ 7 つの欄——⑴ 名前（表の鍵）・⑵ `machine`・⑶ `members` と人手の
印 `hand`・⑷ `domains`・⑸ `foundation` の見出し・⑺ `breakage`・⑻ `themes`——を持ち、⑸ の中身と
⑹ は囲みの直下の本文に書く。見出しに段階を書かないのは、要件 5.2 の初期配置がこの 33 束を
名指ししていないためで、段階は段 3 の `briefing.md` が付ける。

### 絵の重ね方（段階は段 3 で決める）

```toml
[bundle."絵の重ね方"]
machine = []
members = [
  "ukadoc:descript_balloon:overlay_outside_balloon_2c_6570_5024:1",
  "ukadoc:descript_balloon:paint_transparent_region_black_2c_6570_5024:1",
  "ukadoc:descript_balloon:use_input_alpha_2c_6570_5024:1",
  "ukadoc:descript_balloon:use_self_alpha_2c_5024:1",
  "ukadoc:descript_shell:seriko.paint_transparent_region_black_2c_6570_5024:1",
  "ukadoc:descript_shell:seriko.use_self_alpha_2c_5024:1",
  "ukadoc:descript_shell_surfaces:_2a_2c_5b_2a_5d:1",
  "ukadoc:descript_shell_surfaces:add:1",
  "ukadoc:descript_shell_surfaces:asis:1",
  "ukadoc:descript_shell_surfaces:auto:1",
  "ukadoc:descript_shell_surfaces:base:1",
  "ukadoc:descript_shell_surfaces:blend-add-fast:1",
  "ukadoc:descript_shell_surfaces:blend-add-glow-fast:1",
  "ukadoc:descript_shell_surfaces:blend-add-glow:1",
  "ukadoc:descript_shell_surfaces:blend-add:1",
  "ukadoc:descript_shell_surfaces:blend-color-burn-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color-burn:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge-glow-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge-glow:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge:1",
  "ukadoc:descript_shell_surfaces:blend-color-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color:1",
  "ukadoc:descript_shell_surfaces:blend-darken-fast:1",
  "ukadoc:descript_shell_surfaces:blend-darken:1",
  "ukadoc:descript_shell_surfaces:blend-darker-color-fast:1",
  "ukadoc:descript_shell_surfaces:blend-darker-color:1",
  "ukadoc:descript_shell_surfaces:blend-difference-fast:1",
  "ukadoc:descript_shell_surfaces:blend-difference:1",
  "ukadoc:descript_shell_surfaces:blend-dither:1",
  "ukadoc:descript_shell_surfaces:blend-divide-fast:1",
  "ukadoc:descript_shell_surfaces:blend-divide:1",
  "ukadoc:descript_shell_surfaces:blend-exclusion-fast:1",
  "ukadoc:descript_shell_surfaces:blend-exclusion:1",
  "ukadoc:descript_shell_surfaces:blend-hard-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-hard-light:1",
  "ukadoc:descript_shell_surfaces:blend-hard-mix-fast:1",
  "ukadoc:descript_shell_surfaces:blend-hard-mix:1",
  "ukadoc:descript_shell_surfaces:blend-hue-fast:1",
  "ukadoc:descript_shell_surfaces:blend-hue:1",
  "ukadoc:descript_shell_surfaces:blend-lighten-fast:1",
  "ukadoc:descript_shell_surfaces:blend-lighten:1",
  "ukadoc:descript_shell_surfaces:blend-lighter-color-fast:1",
  "ukadoc:descript_shell_surfaces:blend-lighter-color:1",
  "ukadoc:descript_shell_surfaces:blend-linear-burn-fast:1",
  "ukadoc:descript_shell_surfaces:blend-linear-burn:1",
  "ukadoc:descript_shell_surfaces:blend-linear-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-linear-light:1",
  "ukadoc:descript_shell_surfaces:blend-luminosity-fast:1",
  "ukadoc:descript_shell_surfaces:blend-luminosity:1",
  "ukadoc:descript_shell_surfaces:blend-multiply-fast:1",
  "ukadoc:descript_shell_surfaces:blend-multiply:1",
  "ukadoc:descript_shell_surfaces:blend-overlay-fast:1",
  "ukadoc:descript_shell_surfaces:blend-overlay:1",
  "ukadoc:descript_shell_surfaces:blend-pin-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-pin-light:1",
  "ukadoc:descript_shell_surfaces:blend-saturation-fast:1",
  "ukadoc:descript_shell_surfaces:blend-saturation:1",
  "ukadoc:descript_shell_surfaces:blend-screen-fast:1",
  "ukadoc:descript_shell_surfaces:blend-screen:1",
  "ukadoc:descript_shell_surfaces:blend-soft-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-soft-light:1",
  "ukadoc:descript_shell_surfaces:blend-subtract-fast:1",
  "ukadoc:descript_shell_surfaces:blend-subtract:1",
  "ukadoc:descript_shell_surfaces:blend-vivid-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-vivid-light:1",
  "ukadoc:descript_shell_surfaces:element_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30d5_30a1_30a4_30eb_540d_2cX_5ea7_6a19_2cY_5ea7_6a19_28_2c_30aa_30d7_30b7:1",
  "ukadoc:descript_shell_surfaces:import_2c_30d5_30a1_30a4_30eb_540d_2c_30a6_30a8_30a4_30c8msec_2cX_2cY:1",
  "ukadoc:descript_shell_surfaces:interpolate:1",
  "ukadoc:descript_shell_surfaces:move:1",
  "ukadoc:descript_shell_surfaces:overlay-fast:1",
  "ukadoc:descript_shell_surfaces:overlay:1",
  "ukadoc:descript_shell_surfaces:reduce:1",
  "ukadoc:descript_shell_surfaces:replace:1",
  "ukadoc:descript_shell_surfaces:scaling:1",
]
hand = [
  "ukadoc:descript_balloon:overlay_outside_balloon_2c_6570_5024:1",
  "ukadoc:descript_balloon:paint_transparent_region_black_2c_6570_5024:1",
  "ukadoc:descript_balloon:use_input_alpha_2c_6570_5024:1",
  "ukadoc:descript_balloon:use_self_alpha_2c_5024:1",
  "ukadoc:descript_shell:seriko.paint_transparent_region_black_2c_6570_5024:1",
  "ukadoc:descript_shell:seriko.use_self_alpha_2c_5024:1",
  "ukadoc:descript_shell_surfaces:_2a_2c_5b_2a_5d:1",
  "ukadoc:descript_shell_surfaces:add:1",
  "ukadoc:descript_shell_surfaces:asis:1",
  "ukadoc:descript_shell_surfaces:auto:1",
  "ukadoc:descript_shell_surfaces:base:1",
  "ukadoc:descript_shell_surfaces:blend-add-fast:1",
  "ukadoc:descript_shell_surfaces:blend-add-glow-fast:1",
  "ukadoc:descript_shell_surfaces:blend-add-glow:1",
  "ukadoc:descript_shell_surfaces:blend-add:1",
  "ukadoc:descript_shell_surfaces:blend-color-burn-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color-burn:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge-glow-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge-glow:1",
  "ukadoc:descript_shell_surfaces:blend-color-dodge:1",
  "ukadoc:descript_shell_surfaces:blend-color-fast:1",
  "ukadoc:descript_shell_surfaces:blend-color:1",
  "ukadoc:descript_shell_surfaces:blend-darken-fast:1",
  "ukadoc:descript_shell_surfaces:blend-darken:1",
  "ukadoc:descript_shell_surfaces:blend-darker-color-fast:1",
  "ukadoc:descript_shell_surfaces:blend-darker-color:1",
  "ukadoc:descript_shell_surfaces:blend-difference-fast:1",
  "ukadoc:descript_shell_surfaces:blend-difference:1",
  "ukadoc:descript_shell_surfaces:blend-dither:1",
  "ukadoc:descript_shell_surfaces:blend-divide-fast:1",
  "ukadoc:descript_shell_surfaces:blend-divide:1",
  "ukadoc:descript_shell_surfaces:blend-exclusion-fast:1",
  "ukadoc:descript_shell_surfaces:blend-exclusion:1",
  "ukadoc:descript_shell_surfaces:blend-hard-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-hard-light:1",
  "ukadoc:descript_shell_surfaces:blend-hard-mix-fast:1",
  "ukadoc:descript_shell_surfaces:blend-hard-mix:1",
  "ukadoc:descript_shell_surfaces:blend-hue-fast:1",
  "ukadoc:descript_shell_surfaces:blend-hue:1",
  "ukadoc:descript_shell_surfaces:blend-lighten-fast:1",
  "ukadoc:descript_shell_surfaces:blend-lighten:1",
  "ukadoc:descript_shell_surfaces:blend-lighter-color-fast:1",
  "ukadoc:descript_shell_surfaces:blend-lighter-color:1",
  "ukadoc:descript_shell_surfaces:blend-linear-burn-fast:1",
  "ukadoc:descript_shell_surfaces:blend-linear-burn:1",
  "ukadoc:descript_shell_surfaces:blend-linear-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-linear-light:1",
  "ukadoc:descript_shell_surfaces:blend-luminosity-fast:1",
  "ukadoc:descript_shell_surfaces:blend-luminosity:1",
  "ukadoc:descript_shell_surfaces:blend-multiply-fast:1",
  "ukadoc:descript_shell_surfaces:blend-multiply:1",
  "ukadoc:descript_shell_surfaces:blend-overlay-fast:1",
  "ukadoc:descript_shell_surfaces:blend-overlay:1",
  "ukadoc:descript_shell_surfaces:blend-pin-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-pin-light:1",
  "ukadoc:descript_shell_surfaces:blend-saturation-fast:1",
  "ukadoc:descript_shell_surfaces:blend-saturation:1",
  "ukadoc:descript_shell_surfaces:blend-screen-fast:1",
  "ukadoc:descript_shell_surfaces:blend-screen:1",
  "ukadoc:descript_shell_surfaces:blend-soft-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-soft-light:1",
  "ukadoc:descript_shell_surfaces:blend-subtract-fast:1",
  "ukadoc:descript_shell_surfaces:blend-subtract:1",
  "ukadoc:descript_shell_surfaces:blend-vivid-light-fast:1",
  "ukadoc:descript_shell_surfaces:blend-vivid-light:1",
  "ukadoc:descript_shell_surfaces:element_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30d5_30a1_30a4_30eb_540d_2cX_5ea7_6a19_2cY_5ea7_6a19_28_2c_30aa_30d7_30b7:1",
  "ukadoc:descript_shell_surfaces:import_2c_30d5_30a1_30a4_30eb_540d_2c_30a6_30a8_30a4_30c8msec_2cX_2cY:1",
  "ukadoc:descript_shell_surfaces:interpolate:1",
  "ukadoc:descript_shell_surfaces:move:1",
  "ukadoc:descript_shell_surfaces:overlay-fast:1",
  "ukadoc:descript_shell_surfaces:overlay:1",
  "ukadoc:descript_shell_surfaces:reduce:1",
  "ukadoc:descript_shell_surfaces:replace:1",
  "ukadoc:descript_shell_surfaces:scaling:1",
]
domains = ["assets"]
foundation = "絵の重ね合わせと透過の合成器"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: サーフェスの元絵の上に追加の絵を重ね、指定された重ね方と透過の扱いで 1 枚に合成して窓へ出せること。

**欠けると壊れる既存ゴーストの振る舞い**: 着せ替えや目の開閉の重ね絵が出ず、立ち絵が元の 1 枚のまま止まる。半透明で重ねる指定も効かないので、髪や小物の縁が四角く切れて見える。

構成 id は 75 件で、うち機械の束から来たものが 0 件、人手で足したものが 75 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 75 件はすべて `hand` に載る。

### サーフェスアニメーション（段階は段 3 で決める）

```toml
[bundle."サーフェスアニメーション"]
machine = []
members = [
  "ukadoc:descript_shell_surfaces:alternativestart_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:alternativestop_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:always:1",
  "ukadoc:descript_shell_surfaces:animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1",
  "ukadoc:descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1",
  "ukadoc:descript_shell_surfaces:animation_2a.name_2c_5b9a_7fa9_540d:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2cbackground:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2cexclusive:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2cshared-index:1",
  "ukadoc:descript_shell_surfaces:animation_2a.pattern_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30b5_30fc_30d5_30a7_30b9_756a_53f7_2c_30a6_30a7_30a4_30c8_2c:1",
  "ukadoc:descript_shell_surfaces:endtalk:1",
  "ukadoc:descript_shell_surfaces:insert_2cID:1",
  "ukadoc:descript_shell_surfaces:never:1",
  "ukadoc:descript_shell_surfaces:parallelstart_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:parallelstop_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:periodic_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:random_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:rarely:1",
  "ukadoc:descript_shell_surfaces:runonce:1",
  "ukadoc:descript_shell_surfaces:sometimes:1",
  "ukadoc:descript_shell_surfaces:start_2cID:1",
  "ukadoc:descript_shell_surfaces:starttalk:1",
  "ukadoc:descript_shell_surfaces:stop_2cID:1",
  "ukadoc:descript_shell_surfaces:talk_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:yen-e:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.animation.num:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.seriko.defaultsurface:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.num:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface_28ID_29.rect:1",
  "ukadoc:list_propertysystem:currentghost.seriko.surfacelist.all:1",
  "ukadoc:list_propertysystem:currentghost.seriko.surfacelist.defined:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2coverlay_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2ctext_2cx_2cy_2c_6a2a_5e45_2c_7e26_5e45_2c_6587_5b57_5217_2c_8868_793a_6642_9593_2cr_2cg_2cb_2c_658:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cclear_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2coffset_2cID_2cx_5ea7_6a19_2cy_5ea7_6a19_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cpause_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cresume_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cstop_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5beffect2_2c_8ffd_52a0_30b5_30fc_30d5_30a7_30b9ID_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1:1",
  "ukadoc:list_sakura_script:_5c_21_5beffect_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bfilter_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_8d77_52d5_6642_9593_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bfilter_5d:1",
  "ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1",
  "ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1",
  "ukadoc:list_shiori_event:OnSurfaceChange:1",
  "ukadoc:list_shiori_event:OnSurfaceRestore:1",
]
hand = [
  "ukadoc:descript_shell_surfaces:alternativestart_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:alternativestop_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:always:1",
  "ukadoc:descript_shell_surfaces:animation-sort_2c_30bd_30fc_30c8_9806_5e8f:1",
  "ukadoc:descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1",
  "ukadoc:descript_shell_surfaces:animation_2a.name_2c_5b9a_7fa9_540d:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2cbackground:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2cexclusive:1",
  "ukadoc:descript_shell_surfaces:animation_2a.option_2cshared-index:1",
  "ukadoc:descript_shell_surfaces:animation_2a.pattern_2a_2c_63cf_753b_30e1_30bd_30c3_30c9_2c_30b5_30fc_30d5_30a7_30b9_756a_53f7_2c_30a6_30a7_30a4_30c8_2c:1",
  "ukadoc:descript_shell_surfaces:endtalk:1",
  "ukadoc:descript_shell_surfaces:insert_2cID:1",
  "ukadoc:descript_shell_surfaces:never:1",
  "ukadoc:descript_shell_surfaces:parallelstart_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:parallelstop_2c_28ID1_2cID2..._29:1",
  "ukadoc:descript_shell_surfaces:periodic_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:random_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:rarely:1",
  "ukadoc:descript_shell_surfaces:runonce:1",
  "ukadoc:descript_shell_surfaces:sometimes:1",
  "ukadoc:descript_shell_surfaces:start_2cID:1",
  "ukadoc:descript_shell_surfaces:starttalk:1",
  "ukadoc:descript_shell_surfaces:stop_2cID:1",
  "ukadoc:descript_shell_surfaces:talk_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:yen-e:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.animation.num:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.seriko.defaultsurface:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.num:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface_28ID_29.rect:1",
  "ukadoc:list_propertysystem:currentghost.seriko.surfacelist.all:1",
  "ukadoc:list_propertysystem:currentghost.seriko.surfacelist.defined:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2coverlay_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cadd_2ctext_2cx_2cy_2c_6a2a_5e45_2c_7e26_5e45_2c_6587_5b57_5217_2c_8868_793a_6642_9593_2cr_2cg_2cb_2c_658:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cclear_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2coffset_2cID_2cx_5ea7_6a19_2cy_5ea7_6a19_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cpause_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cresume_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5banim_2cstop_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5beffect2_2c_8ffd_52a0_30b5_30fc_30d5_30a7_30b9ID_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1:1",
  "ukadoc:list_sakura_script:_5c_21_5beffect_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_901f_5ea6_500d_7387_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bfilter_2c_30d7_30e9_30b0_30a4_30f3_540d_2c_8d77_52d5_6642_9593_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bfilter_5d:1",
  "ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1",
  "ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1",
  "ukadoc:list_shiori_event:OnSurfaceChange:1",
  "ukadoc:list_shiori_event:OnSurfaceRestore:1",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "SERIKO/MAYUNA の再生"
breakage = "黙って壊れる"
themes = ["気配", "装い"]
```

**成立に要る最小の基盤**: surfaces.txt の定義どおりにコマを差し替え、間隔と繰り返しの指定に従って再生し、台本からの開始・停止・差し込みを受け付けられること。

**欠けると壊れる既存ゴーストの振る舞い**: 目も口も動かない止め絵のまま立ち、瞬きもしない。台本が呼ぶ画面効果や重ね絵の追加も出ないので、場面が切り替わったことが絵で伝わらない。

構成 id は 47 件で、うち機械の束から来たものが 0 件、人手で足したものが 47 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 47 件はすべて `hand` に載る。

### 着せ替え（段階は段 3 で決める）

```toml
[bundle."着せ替え"]
machine = []
members = [
  "ukadoc:descript_shell:char_2a.bindgroup_2a.addid_2cID:1",
  "ukadoc:descript_shell:char_2a.bindgroup_2a.default_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1",
  "ukadoc:descript_shell:char_2a.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell:kero.bindgroup_2a.addid_2cID:1",
  "ukadoc:descript_shell:kero.bindgroup_2a.default_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1",
  "ukadoc:descript_shell:kero.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell:sakura.bindgroup_2a.addid_2cID:1",
  "ukadoc:descript_shell:sakura.bindgroup_2a.default_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1",
  "ukadoc:descript_shell:sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell_surfaces:bind:1",
]
hand = [
  "ukadoc:descript_shell:char_2a.bindgroup_2a.addid_2cID:1",
  "ukadoc:descript_shell:char_2a.bindgroup_2a.default_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1",
  "ukadoc:descript_shell:char_2a.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell:kero.bindgroup_2a.addid_2cID:1",
  "ukadoc:descript_shell:kero.bindgroup_2a.default_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1",
  "ukadoc:descript_shell:kero.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell:sakura.bindgroup_2a.addid_2cID:1",
  "ukadoc:descript_shell:sakura.bindgroup_2a.default_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.bindgroup_2a.name_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_30b5_30e0_30cd_30a4_30eb_540d:1",
  "ukadoc:descript_shell:sakura.bindoption_2a.group_2c_30ab_30c6_30b4_30ea_540d_2c_30aa_30d7_30b7_30e7_30f3:1",
  "ukadoc:descript_shell_surfaces:bind:1",
]
domains = ["assets"]
foundation = "着せ替え（MAYUNA）の組み立て"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: 着せ替えの分類と部品の宣言を読み、選ばれた組み合わせを立ち絵へ重ねられること。

**欠けると壊れる既存ゴーストの振る舞い**: 着せ替えの一覧に何も並ばず、作者が用意した衣装や小物へ着替えられない。既定で着せる指定も効かないので、初回から作者の意図した姿にならない。

構成 id は 13 件で、うち機械の束から来たものが 0 件、人手で足したものが 13 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 13 件はすべて `hand` に載る。

### マウスの矢印（段階は段 3 で決める）

```toml
[bundle."マウスの矢印"]
machine = []
members = [
  "ukadoc:descript_balloon:cursor_2c_30d5_30a1_30a4_30eb_540d_20_2f_20mousecursor_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:mousecursor.arrow_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:mousecursor.text_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:mousecursor.wait_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:cursor_2c_30d5_30a1_30a4_30eb_540d_20_2f_20mousecursor_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.arrow_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.grip_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.hand_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.text_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.wait_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mousedown_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mousehover_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mouserightdown_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mouseup_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mousewheel_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:system_3aarrow:1",
  "ukadoc:descript_shell_surfaces:system_3across:1",
  "ukadoc:descript_shell_surfaces:system_3afinger:1",
  "ukadoc:descript_shell_surfaces:system_3agrip:1",
  "ukadoc:descript_shell_surfaces:system_3ahand:1",
  "ukadoc:descript_shell_surfaces:system_3ahelp:1",
  "ukadoc:descript_shell_surfaces:system_3amove:1",
  "ukadoc:descript_shell_surfaces:system_3ano:1",
  "ukadoc:descript_shell_surfaces:system_3atext:1",
  "ukadoc:descript_shell_surfaces:system_3await:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor.arrow:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor.text:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor.wait:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.arrow:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.grip:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.hand:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.text:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.wait:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.count:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.path:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.path:1",
]
hand = [
  "ukadoc:descript_balloon:cursor_2c_30d5_30a1_30a4_30eb_540d_20_2f_20mousecursor_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:mousecursor.arrow_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:mousecursor.text_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:mousecursor.wait_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:cursor_2c_30d5_30a1_30a4_30eb_540d_20_2f_20mousecursor_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.arrow_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.grip_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.hand_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.text_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:mousecursor.wait_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mousedown_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mousehover_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mouserightdown_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mouseup_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:mousewheel_2a_2c_5f53_305f_308a_5224_5b9aID_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell_surfaces:system_3aarrow:1",
  "ukadoc:descript_shell_surfaces:system_3across:1",
  "ukadoc:descript_shell_surfaces:system_3afinger:1",
  "ukadoc:descript_shell_surfaces:system_3agrip:1",
  "ukadoc:descript_shell_surfaces:system_3ahand:1",
  "ukadoc:descript_shell_surfaces:system_3ahelp:1",
  "ukadoc:descript_shell_surfaces:system_3amove:1",
  "ukadoc:descript_shell_surfaces:system_3ano:1",
  "ukadoc:descript_shell_surfaces:system_3atext:1",
  "ukadoc:descript_shell_surfaces:system_3await:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor.arrow:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor.text:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor.wait:1",
  "ukadoc:list_propertysystem:currentghost.balloon.mousecursor:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.arrow:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.grip:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.hand:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.text:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor.wait:1",
  "ukadoc:list_propertysystem:currentghost.mousecursor:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.count:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.path:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1",
  "ukadoc:list_propertysystem:currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.path:1",
]
domains = ["assets", "property"]
foundation = "マウスの矢印の差し替え"
breakage = "黙って壊れる"
themes = ["触れ合い", "装い"]
```

**成立に要る最小の基盤**: 触れている場所に応じてマウスの矢印の絵を差し替え、差し替えた絵の在り処を問い合わせに答えられること。

**欠けると壊れる既存ゴーストの振る舞い**: 撫でられる場所も掴んで動かせる場所も矢印が同じままで、どこに触れられるのかが見て分からない。

構成 id は 40 件で、うち機械の束から来たものが 0 件、人手で足したものが 40 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 40 件はすべて `hand` に載る。

### シェル定義の転記（段階は段 3 で決める）

```toml
[bundle."シェル定義の転記"]
machine = []
members = [
  "ukadoc:descript_shell_surfaces:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell_surfaces:maxwidth_2c_30d4_30af_30bb_30eb:1",
  "ukadoc:descript_shell_surfaces:name_2c_5b9a_7fa9_540d:1",
  "ukadoc:descript_shell_surfaces:version_2c_2a:1",
  "ukadoc:descript_shell_surfacetable:_30b5_30fc_30d5_30a7_30b9ID_2c_540d_524d:1",
  "ukadoc:descript_shell_surfacetable:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell_surfacetable:group_2c_30b0_30eb_30fc_30d7_540d:1",
  "ukadoc:descript_shell_surfacetable:option_2c_30aa_30d7_30b7_30e7_30f31_2c_30aa_30d7_30b7_30e7_30f32_2c...:1",
  "ukadoc:descript_shell_surfacetable:scope_2c_6570_5024:1",
  "ukadoc:descript_shell_surfacetable:version_2c_6570_5024:1",
]
hand = [
  "ukadoc:descript_shell_surfaces:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell_surfaces:maxwidth_2c_30d4_30af_30bb_30eb:1",
  "ukadoc:descript_shell_surfaces:name_2c_5b9a_7fa9_540d:1",
  "ukadoc:descript_shell_surfaces:version_2c_2a:1",
  "ukadoc:descript_shell_surfacetable:_30b5_30fc_30d5_30a7_30b9ID_2c_540d_524d:1",
  "ukadoc:descript_shell_surfacetable:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell_surfacetable:group_2c_30b0_30eb_30fc_30d7_540d:1",
  "ukadoc:descript_shell_surfacetable:option_2c_30aa_30d7_30b7_30e7_30f31_2c_30aa_30d7_30b7_30e7_30f32_2c...:1",
  "ukadoc:descript_shell_surfacetable:scope_2c_6570_5024:1",
  "ukadoc:descript_shell_surfacetable:version_2c_6570_5024:1",
]
domains = ["assets"]
foundation = "シェルの定義ファイルの転記層"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: シェルの定義ファイルに書かれた作者の手控えと書式の宣言を、読み飛ばさずに写せること。

**欠けると壊れる既存ゴーストの振る舞い**: 作者がサーフェスに付けた名前が読めず、着せ替えの画面に番号だけが並ぶ。書式の宣言を読まないまま進む定義ファイルでは、読み取りそのものが途中で止まる。

構成 id は 10 件で、うち機械の束から来たものが 0 件、人手で足したものが 10 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 10 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 10 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### descript の転記（段階は段 3 で決める）

```toml
[bundle."descript の転記"]
machine = []
members = [
  "ukadoc:descript_ghost:balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:balloon_2c_30d0_30eb_30fc_30f3_540d:1",
  "ukadoc:descript_ghost:char_2a.balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:char_2a.seriko.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:default.balloon.path_2c_30d1_30b9:1",
  "ukadoc:descript_ghost:don_27t_20need_20bind_2c_6570_5024:1",
  "ukadoc:descript_ghost:don_27t_20need_20onmousemove_2c_6570_5024:1",
  "ukadoc:descript_ghost:don_27t_20need_20seriko_20talk_2c_6570_5024:1",
  "ukadoc:descript_ghost:kero.balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:kero.seriko.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:sakura.balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:sakura.seriko.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:seriko.defaultsurfacedirectoryname_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_shell:sakura.name2_2c_540d_524d:1",
  "ukadoc:descript_shell:seriko.force_disable_crossfade_2c_5024:1",
]
hand = [
  "ukadoc:descript_ghost:balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:balloon_2c_30d0_30eb_30fc_30f3_540d:1",
  "ukadoc:descript_ghost:char_2a.balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:char_2a.seriko.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:default.balloon.path_2c_30d1_30b9:1",
  "ukadoc:descript_ghost:don_27t_20need_20bind_2c_6570_5024:1",
  "ukadoc:descript_ghost:don_27t_20need_20onmousemove_2c_6570_5024:1",
  "ukadoc:descript_ghost:don_27t_20need_20seriko_20talk_2c_6570_5024:1",
  "ukadoc:descript_ghost:kero.balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:kero.seriko.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:sakura.balloon.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:sakura.seriko.defaultsurface_2c_6570_5024:1",
  "ukadoc:descript_ghost:seriko.defaultsurfacedirectoryname_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1",
  "ukadoc:descript_shell:sakura.name2_2c_540d_524d:1",
  "ukadoc:descript_shell:seriko.force_disable_crossfade_2c_5024:1",
]
domains = ["assets"]
foundation = "ゴーストとシェルの descript の転記層"
breakage = "黙って壊れる"
themes = ["掛け合い", "装い"]
```

**成立に要る最小の基盤**: ゴーストとシェルの descript.txt に書かれた既定値を、欠かさず内部の形へ写せること。

**欠けると壊れる既存ゴーストの振る舞い**: 作者が決めた最初のサーフェスや最初に着るバルーンが使われず、起動のたびに既定の姿で出てくる。切っておきたい機能を切る宣言も届かない。

構成 id は 15 件で、うち機械の束から来たものが 0 件、人手で足したものが 15 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 15 件はすべて `hand` に載る。

### 定義ファイルの文字コード（段階は段 3 で決める）

```toml
[bundle."定義ファイルの文字コード"]
machine = []
members = [
  "ukadoc:descript_balloon:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell:charset_2c_6587_5b57_30b3_30fc_30c9:1",
]
hand = [
  "ukadoc:descript_balloon:charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell:charset_2c_6587_5b57_30b3_30fc_30c9:1",
]
domains = ["assets"]
foundation = "定義ファイルの文字コードの前走査"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 定義ファイルの先頭にある文字コードの宣言を本文より先に読み、その符号化で本文を解けること。

**欠けると壊れる既存ゴーストの振る舞い**: 宣言と違う符号化で読まれた題や説明文が文字化けし、日本語の名前が読めない形で一覧に並ぶ。

構成 id は 2 件で、うち機械の束から来たものが 0 件、人手で足したものが 2 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 2 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 2 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 配布物の素性（段階は段 3 で決める）

```toml
[bundle."配布物の素性"]
machine = []
members = [
  "ukadoc:descript_balloon:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_balloon:craftmanurl_2cURL:1",
  "ukadoc:descript_balloon:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_balloon:id_2cID_540d:1",
  "ukadoc:descript_balloon:name_2c_30d0_30eb_30fc_30f3_540d:1",
  "ukadoc:descript_balloon:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_balloon:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:recommended.ghost.path_2c_30d1_30b9:1",
  "ukadoc:descript_balloon:recommended.ghost_2c_30b4_30fc_30b9_30c8_540d:1",
  "ukadoc:descript_balloon:type_2c_7a2e_5225:1",
  "ukadoc:descript_ghost:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:recommended.balloon.path_2c_30d1_30b9:1",
  "ukadoc:descript_ghost:recommended.balloon_2c_30d0_30eb_30fc_30f3_540d:1",
  "ukadoc:descript_shell:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:craftmanurl_2cURL:1",
  "ukadoc:descript_shell:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:id_2cID_540d:1",
  "ukadoc:descript_shell:name_2c_30b7_30a7_30eb_540d:1",
  "ukadoc:descript_shell:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:type_2c_7a2e_5225:1",
]
hand = [
  "ukadoc:descript_balloon:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_balloon:craftmanurl_2cURL:1",
  "ukadoc:descript_balloon:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_balloon:id_2cID_540d:1",
  "ukadoc:descript_balloon:name_2c_30d0_30eb_30fc_30f3_540d:1",
  "ukadoc:descript_balloon:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_balloon:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:recommended.ghost.path_2c_30d1_30b9:1",
  "ukadoc:descript_balloon:recommended.ghost_2c_30b4_30fc_30b9_30c8_540d:1",
  "ukadoc:descript_balloon:type_2c_7a2e_5225:1",
  "ukadoc:descript_ghost:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_ghost:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:recommended.balloon.path_2c_30d1_30b9:1",
  "ukadoc:descript_ghost:recommended.balloon_2c_30d0_30eb_30fc_30f3_540d:1",
  "ukadoc:descript_shell:craftman_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:craftmanurl_2cURL:1",
  "ukadoc:descript_shell:craftmanw_2c_4f5c_8005_540d:1",
  "ukadoc:descript_shell:id_2cID_540d:1",
  "ukadoc:descript_shell:name_2c_30b7_30a7_30eb_540d:1",
  "ukadoc:descript_shell:readme.charset_2c_6587_5b57_30b3_30fc_30c9:1",
  "ukadoc:descript_shell:readme_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:type_2c_7a2e_5225:1",
]
domains = ["assets"]
foundation = "資産の素性を保持する場所"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 配布物に書かれた名前・作者・説明文・薦める組み合わせを読み、必要なときに取り出せること。

**欠けると壊れる既存ゴーストの振る舞い**: ゴースト一覧に作者名も説明も出ず、どれを入れたのか見分けが付かない。作者が薦めた相方やバルーンの組み合わせも案内されない。

構成 id は 22 件で、うち機械の束から来たものが 0 件、人手で足したものが 22 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 22 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 22 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### バルーンの文字（段階は段 3 で決める）

```toml
[bundle."バルーンの文字"]
machine = []
members = [
  "ukadoc:descript_balloon:disable.font._28_30d5_30a9_30f3_30c8_5b9a_7fa9_29_2c_28_6307_5b9a_29:1",
  "ukadoc:descript_balloon:font.bold_2c0_2f1:1",
  "ukadoc:descript_balloon:font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.height_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.italic_2c0_2f1:1",
  "ukadoc:descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:font.outline_2c0_2f1:1",
  "ukadoc:descript_balloon:font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:font.strike_2c0_2f1:1",
  "ukadoc:descript_balloon:font.underline_2c0_2f1:1",
  "ukadoc:descript_balloon:origin.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:origin.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.bottom_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.left_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.right_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.top_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:vertical_2c0_2f1:1",
  "ukadoc:descript_balloon:wordwrappoint.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:wordwrappoint.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.background.color:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.x:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.y:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.char_width:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.count:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines.initial:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.num:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight.initial:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth.initial:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.vertical:1",
  "ukadoc:list_sakura_script:_5c_26_5bID_5d:1",
  "ukadoc:list_sakura_script:_5c__w_5banimation_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1",
  "ukadoc:list_sakura_script:_5c_m_5b0x00_5d:1",
  "ukadoc:list_sakura_script:_5c_n:1",
  "ukadoc:list_sakura_script:_5c_u_5b0x0000_5d:1",
  "ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1",
  "ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1",
  "ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bdefault_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bdisable_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1",
  "ukadoc:list_sakura_script:_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1",
  "ukadoc:list_shiori_event:OnNotifyFontInfo:1",
]
hand = [
  "ukadoc:descript_balloon:disable.font._28_30d5_30a9_30f3_30c8_5b9a_7fa9_29_2c_28_6307_5b9a_29:1",
  "ukadoc:descript_balloon:font.bold_2c0_2f1:1",
  "ukadoc:descript_balloon:font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.height_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.italic_2c0_2f1:1",
  "ukadoc:descript_balloon:font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:font.outline_2c0_2f1:1",
  "ukadoc:descript_balloon:font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:font.strike_2c0_2f1:1",
  "ukadoc:descript_balloon:font.underline_2c0_2f1:1",
  "ukadoc:descript_balloon:origin.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:origin.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.bottom_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.left_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.right_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:validrect.top_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:vertical_2c0_2f1:1",
  "ukadoc:descript_balloon:wordwrappoint.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:wordwrappoint.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.background.color:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.x:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.basepos.y:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.char_width:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.count:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines.initial:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.lines:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.num:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight.initial:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validheight:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth.initial:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.validwidth:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.vertical:1",
  "ukadoc:list_sakura_script:_5c_26_5bID_5d:1",
  "ukadoc:list_sakura_script:_5c__w_5banimation_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_l_5bx_2cy_5d:1",
  "ukadoc:list_sakura_script:_5c_m_5b0x00_5d:1",
  "ukadoc:list_sakura_script:_5c_n:1",
  "ukadoc:list_sakura_script:_5c_u_5b0x0000_5d:1",
  "ukadoc:list_sakura_script:_5cc_5bchar_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1",
  "ukadoc:list_sakura_script:_5cc_5bline_2c_6570_5024_2c_958b_59cb_4f4d_7f6e_5d:1",
  "ukadoc:list_sakura_script:_5cf_5balign_2c_5bc4_305b_308b_5074_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bdefault_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bdisable_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bitalic_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bname_2c_30d5_30a9_30f3_30c8_540d_5d:1",
  "ukadoc:list_sakura_script:_5cf_5boutline_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bshadowcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bshadowcolor_2cnone_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bshadowstyle_2c_5f62_614b_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bstrike_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bsub_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bsup_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bunderline_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bvalign_2c_5bc4_305b_308b_5074_5d:1",
  "ukadoc:list_shiori_event:OnNotifyFontInfo:1",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "バルーンの文字描画"
breakage = "黙って壊れる"
themes = ["掛け合い", "装い"]
```

**成立に要る最小の基盤**: 指定された書体・大きさ・色・飾りで台詞を組み、指定の折返しと寄せでバルーンの中に置けること。

**欠けると壊れる既存ゴーストの振る舞い**: 作者が選んだ書体も文字色も効かず、どのゴーストも同じ見た目で喋る。強調や影の指定も無視されるので、台詞の抑揚が絵として伝わらない。

構成 id は 63 件で、うち機械の束から来たものが 0 件、人手で足したものが 63 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 63 件はすべて `hand` に載る。

### バルーンのリンク（段階は段 3 で決める）

```toml
[bundle."バルーンのリンク"]
machine = []
members = [
  "ukadoc:descript_balloon:anchor.blendmethod_2c_30b3_30de_30f3_30c9:1",
  "ukadoc:descript_balloon:anchor.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:anchor.notselect.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:anchor.notselect.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.style_2c_5f62_72b6:1",
  "ukadoc:descript_balloon:anchor.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.style_2c_5f62_72b6:1",
  "ukadoc:descript_balloon:anchor.visited.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:anchor.visited.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.style_2c_5f62_72b6:1",
  "ukadoc:list_sakura_script:_5c_a_5bOnID_2cr0_2cr1..._5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchor.font.color_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorbrushcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchormethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchornotselectbrushcolor_2c_8272_6307_5b9a_5:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectstyle_2c_5f62_72b6_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorstyle_2c_5f62_72b6_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorvisitedbrushcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedmethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedstyle_2c_5f62_72b6_5d:1",
]
hand = [
  "ukadoc:descript_balloon:anchor.blendmethod_2c_30b3_30de_30f3_30c9:1",
  "ukadoc:descript_balloon:anchor.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:anchor.notselect.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:anchor.notselect.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.notselect.style_2c_5f62_72b6:1",
  "ukadoc:descript_balloon:anchor.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.style_2c_5f62_72b6:1",
  "ukadoc:descript_balloon:anchor.visited.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:anchor.visited.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:anchor.visited.style_2c_5f62_72b6:1",
  "ukadoc:list_sakura_script:_5c_a_5bOnID_2cr0_2cr1..._5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchor.font.color_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorbrushcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchormethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchornotselectbrushcolor_2c_8272_6307_5b9a_5:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchornotselectstyle_2c_5f62_72b6_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorstyle_2c_5f62_72b6_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5banchorvisitedbrushcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedmethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5banchorvisitedstyle_2c_5f62_72b6_5d:1",
]
domains = ["assets", "sakura-script"]
foundation = "バルーンの中のリンク機能"
breakage = "黙って壊れる"
themes = ["装い", "交わり"]
```

**成立に要る最小の基盤**: バルーンの本文に埋め込まれたリンクを、選ぶ前・選んだ後・触れている間の 3 つの見た目で描き分け、押されたらイベントを起こせること。

**欠けると壊れる既存ゴーストの振る舞い**: 本文の中のリンクが地の文と同じ見た目になり、どこを押せばよいか分からない。押した後も色が変わらないので、もう読んだ場所かどうかを見分けられない。

構成 id は 60 件で、うち機械の束から来たものが 0 件、人手で足したものが 60 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 60 件はすべて `hand` に載る。

### 選択肢の目印（段階は段 3 で決める）

```toml
[bundle."選択肢の目印"]
machine = []
members = [
  "ukadoc:descript_balloon:cursor.blendmethod_2c_30b3_30de_30f3_30c9:1",
  "ukadoc:descript_balloon:cursor.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:cursor.notselect.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:cursor.notselect.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.style_2c_5f62_72b6:1",
  "ukadoc:descript_balloon:cursor.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.style_2c_5f62_72b6:1",
  "ukadoc:list_sakura_script:_5c_21_5b_2a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursorbrushcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursormethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursornotselectbrushcolor_2c_8272_6307_5b9a_5:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectstyle_2c_5f62_72b6_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorstyle_2c_5f62_72b6_5d:1",
]
hand = [
  "ukadoc:descript_balloon:cursor.blendmethod_2c_30b3_30de_30f3_30c9:1",
  "ukadoc:descript_balloon:cursor.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:cursor.notselect.brush.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.brush.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.brush.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowcolor.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowcolor.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowcolor.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.font.shadowstyle_2c_5f62_614b_6307_5b9a:1",
  "ukadoc:descript_balloon:cursor.notselect.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.notselect.style_2c_5f62_72b6:1",
  "ukadoc:descript_balloon:cursor.pen.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.pen.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.pen.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:cursor.style_2c_5f62_72b6:1",
  "ukadoc:list_sakura_script:_5c_21_5b_2a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursorbrushcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursormethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectcolor_2c_8272_6307_5b9a_5d_3082_3057_304f_306f_5cf_5bcursornotselectbrushcolor_2c_8272_6307_5b9a_5:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectfontcolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectmethod_2c_63cf_753b_65b9_6cd5_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursornotselectstyle_2c_5f62_72b6_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorpencolor_2c_8272_6307_5b9a_5d:1",
  "ukadoc:list_sakura_script:_5cf_5bcursorstyle_2c_5f62_72b6_5d:1",
]
domains = ["assets", "sakura-script"]
foundation = "選択肢の目印の描画"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: 選択肢の行に付く目印を、選んでいる行と選んでいない行で描き分けられること。

**欠けると壊れる既存ゴーストの振る舞い**: 選択肢が並んでも今どれを選んでいるかが見えず、当てずっぽうで押すことになる。

構成 id は 40 件で、うち機械の束から来たものが 0 件、人手で足したものが 40 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 40 件はすべて `hand` に載る。

### バルーンの付属画像（段階は段 3 で決める）

```toml
[bundle."バルーンの付属画像"]
machine = []
members = [
  "ukadoc:descript_balloon:arrow.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:arrow0.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:arrow0.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:arrow1.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:arrow1.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:clickwaitmarker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:clickwaitmarker.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:clickwaitmarker.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:marker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:number.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.height_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:number.xr_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:number.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonmarker_2c_30de_30fc_30ab_30fc_8868_793a_6587_5b57_5217_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonnum_2c_30d5_30a1_30a4_30eb_540d_2c_73fe_5728_306e_6570_2c_6700_5927_6570_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1",
]
hand = [
  "ukadoc:descript_balloon:arrow.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:arrow0.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:arrow0.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:arrow1.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:arrow1.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:clickwaitmarker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:clickwaitmarker.x_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:clickwaitmarker.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:marker.filename_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_balloon:number.font.color.b_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.color.g_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.color.r_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.height_2c_6570_5024:1",
  "ukadoc:descript_balloon:number.font.name_2c_30d5_30a9_30f3_30c8_540d:1",
  "ukadoc:descript_balloon:number.xr_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:descript_balloon:number.y_2c_5ea7_6a19_20_2a1:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonmarker_2c_30de_30fc_30ab_30fc_8868_793a_6587_5b57_5217_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonnum_2c_30d5_30a1_30a4_30eb_540d_2c_73fe_5728_306e_6570_2c_6700_5927_6570_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1",
]
domains = ["assets", "sakura-script"]
foundation = "バルーンに付属する画像の族"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: バルーンに添える送り読みの矢印・クリック待ちの目印・枚数のカウンタ・本文に貼る画像を、指定された位置と絵で出せること。

**欠けると壊れる既存ゴーストの振る舞い**: 続きがあるのか押すのを待たれているのかが目印で示されず、利用者はバルーンの前で止まる。台詞に貼る画像も出ない。

構成 id は 22 件で、うち機械の束から来たものが 0 件、人手で足したものが 22 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 22 件はすべて `hand` に載る。

### 窓の配置と重なり（段階は段 3 で決める）

```toml
[bundle."窓の配置と重なり"]
machine = [
  "ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1",
]
members = [
  "ukadoc:descript_balloon:dpi_2c_63a8_5968DPI:1",
  "ukadoc:descript_balloon:windowposition.limit_2c0_2f1:1",
  "ukadoc:descript_balloon:windowposition.x_2c_5ea7_6a19:1",
  "ukadoc:descript_balloon:windowposition.y_2c_5ea7_6a19:1",
  "ukadoc:descript_ghost:balloon.dontmove_2ctrue:1",
  "ukadoc:descript_ghost:balloon.syncscale_2ctrue:1",
  "ukadoc:descript_ghost:char_2a.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:kero.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:sakura.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:char_2a.balloon.dontmove_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.balloon.syncscale_2ctrue:1",
  "ukadoc:descript_shell:char_2a.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:kero.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:kero.balloon.dontmove_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.balloon.syncscale_2ctrue:1",
  "ukadoc:descript_shell:kero.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:sakura.balloon.dontmove_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.balloon.syncscale_2ctrue:1",
  "ukadoc:descript_shell:sakura.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:seriko.dpi_2c_63a8_5968DPI:1",
  "ukadoc:descript_shell:seriko.sticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1",
  "ukadoc:descript_shell:seriko.zorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1",
  "ukadoc:descript_shell_surfaces:balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:kero.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:kero.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.centerx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.centery_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.kinoko.centerx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.kinoko.centery_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:sakura.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:sakura.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.rect:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.scaling:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.x:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.y:1",
  "ukadoc:list_propertysystem:currentghost.scope.count:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.bpp:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.dpi:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.primary:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.rect:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.work:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.rect:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.scaling:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.x:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.y:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.x:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.y:1",
  "ukadoc:list_propertysystem:currentghost.seriko.sticky-window:1",
  "ukadoc:list_propertysystem:currentghost.seriko.zorder:1",
  "ukadoc:list_sakura_script:_5c4:1",
  "ukadoc:list_sakura_script:_5c5:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetballoonpos_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetwindowpos_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5block_2cballoonmove_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5block_2cballoonrepaint_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5block_2crepaint_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bmove_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bmoveasync_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breset_2cposition_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breset_2csticky-window_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calpha_2c_6570_5024_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonalign_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonoffset_2cx_2cy_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cposition_2cx_2cy_2c_30b9_30b3_30fc_30d7ID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2csticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cstayontop_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2czorder_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonmove_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonrepaint_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunlock_2crepaint_5d:1",
  "ukadoc:list_sakura_script:_5cv:1",
  "ukadoc:list_shiori_event:OnOffscreen:1",
  "ukadoc:list_shiori_event:OnOverlap:1",
  "ukadoc:list_shiori_event:OnResetWindowPos:1",
  "ukadoc:list_shiori_event:hwnd:1",
  "ukadoc:list_shiori_resource:char_2a.defaultleft:1",
  "ukadoc:list_shiori_resource:char_2a.defaulttop:1",
  "ukadoc:list_shiori_resource:char_2a.defaultx:1",
  "ukadoc:list_shiori_resource:char_2a.defaulty:1",
  "ukadoc:list_shiori_resource:kero.defaultleft:1",
  "ukadoc:list_shiori_resource:kero.defaulttop:1",
  "ukadoc:list_shiori_resource:kero.defaultx:1",
  "ukadoc:list_shiori_resource:kero.defaulty:1",
  "ukadoc:list_shiori_resource:sakura.defaultleft:1",
  "ukadoc:list_shiori_resource:sakura.defaulttop:1",
  "ukadoc:list_shiori_resource:sakura.defaultx:1",
  "ukadoc:list_shiori_resource:sakura.defaulty:1",
]
hand = [
  "ukadoc:descript_balloon:dpi_2c_63a8_5968DPI:1",
  "ukadoc:descript_balloon:windowposition.limit_2c0_2f1:1",
  "ukadoc:descript_balloon:windowposition.x_2c_5ea7_6a19:1",
  "ukadoc:descript_balloon:windowposition.y_2c_5ea7_6a19:1",
  "ukadoc:descript_ghost:balloon.dontmove_2ctrue:1",
  "ukadoc:descript_ghost:balloon.syncscale_2ctrue:1",
  "ukadoc:descript_ghost:char_2a.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:kero.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:sakura.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_ghost:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:char_2a.balloon.dontmove_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.balloon.syncscale_2ctrue:1",
  "ukadoc:descript_shell:char_2a.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:char_2a.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:kero.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:kero.balloon.dontmove_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.balloon.syncscale_2ctrue:1",
  "ukadoc:descript_shell:kero.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:sakura.balloon.dontmove_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.balloon.syncscale_2ctrue:1",
  "ukadoc:descript_shell:sakura.defaultleft_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.defaulttop_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.defaultx_2cX_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.defaulty_2cY_5ea7_6a19:1",
  "ukadoc:descript_shell:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_shell:seriko.dpi_2c_63a8_5968DPI:1",
  "ukadoc:descript_shell:seriko.sticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1",
  "ukadoc:descript_shell_surfaces:balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:kero.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:kero.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.basepos.x_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.basepos.y_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.centerx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.centery_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.kinoko.centerx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:point.kinoko.centery_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:sakura.balloon.offsetx_2c_5ea7_6a19:1",
  "ukadoc:descript_shell_surfaces:sakura.balloon.offsety_2c_5ea7_6a19:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.rect:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.scaling:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.x:1",
  "ukadoc:list_propertysystem:currentghost.balloon.scope_28ID_29.y:1",
  "ukadoc:list_propertysystem:currentghost.scope.count:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.bpp:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.dpi:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.primary:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.rect:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.currentmonitor.work:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.rect:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.scaling:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.x:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.surface.y:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.x:1",
  "ukadoc:list_propertysystem:currentghost.scope_28ID_29.y:1",
  "ukadoc:list_sakura_script:_5c4:1",
  "ukadoc:list_sakura_script:_5c5:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetballoonpos_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cresetwindowpos_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5block_2cballoonmove_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5block_2cballoonrepaint_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5block_2crepaint_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bmove_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bmoveasync_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breset_2cposition_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breset_2csticky-window_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calignmentondesktop_2cbottom_307e_305f_306ftop_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2c_65b9_5411_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calignmenttodesktop_2cfree_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2calpha_2c_6570_5024_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonalign_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cballoonoffset_2cx_2cy_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cposition_2cx_2cy_2c_30b9_30b3_30fc_30d7ID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2c_21stayontop_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwindowstate_2cstayontop_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonmove_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunlock_2cballoonrepaint_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bunlock_2crepaint_5d:1",
  "ukadoc:list_sakura_script:_5cv:1",
  "ukadoc:list_shiori_event:OnOffscreen:1",
  "ukadoc:list_shiori_event:OnOverlap:1",
  "ukadoc:list_shiori_event:OnResetWindowPos:1",
  "ukadoc:list_shiori_event:hwnd:1",
  "ukadoc:list_shiori_resource:char_2a.defaultleft:1",
  "ukadoc:list_shiori_resource:char_2a.defaulttop:1",
  "ukadoc:list_shiori_resource:char_2a.defaultx:1",
  "ukadoc:list_shiori_resource:char_2a.defaulty:1",
  "ukadoc:list_shiori_resource:kero.defaultleft:1",
  "ukadoc:list_shiori_resource:kero.defaulttop:1",
  "ukadoc:list_shiori_resource:kero.defaultx:1",
  "ukadoc:list_shiori_resource:kero.defaulty:1",
  "ukadoc:list_shiori_resource:sakura.defaultleft:1",
  "ukadoc:list_shiori_resource:sakura.defaulttop:1",
  "ukadoc:list_shiori_resource:sakura.defaultx:1",
  "ukadoc:list_shiori_resource:sakura.defaulty:1",
]
domains = ["assets", "property", "sakura-script", "shiori"]
foundation = "窓の配置と重なりの解決"
breakage = "黙って壊れる"
themes = ["気配", "掛け合い", "装い"]
```

**成立に要る最小の基盤**: キャラクタ窓とバルーン窓の置き場所を画面の座標に解き、複数のキャラクタ窓をどの順で重ねるかを決め、台本の指示で動かし、今の位置と大きさを問い合わせに答えられること。

**欠けると壊れる既存ゴーストの振る舞い**: 作者が決めた初期位置が読まれないので、起動のたびに画面の同じ隅へ出る。台本で位置を動かす指示も効かず、相方が本体の後ろに隠れたまま出てこない。画面の外へはみ出しても気づかない。

構成 id は 126 件で、うち機械の束から来たものが 5 件、人手で足したものが 121 件である（`hand` の行を数えた）。`machine` に引いた機械の束の構成 id と重なるのが 5 件で、残りは人手で足した。

### 入力窓とダイアログ（段階は段 3 で決める）

```toml
[bundle."入力窓とダイアログ"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cdialog_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cinputbox_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdateinput_2cID_2c_8868_793a_6642_9593_2c_5e74_2c_6708_2c_65e5_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2ccolor_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2cfolder_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2copen_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2csave_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cipinput_2cID_2c_8868_793a_6642_9593_2cIP1_6841_76ee_2cIP2_6841_76ee_2cIP3_6841_76ee_2cIP4_6841_76ee_2c_3:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cpasswordinput_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2csliderinput_2cID_2c_8868_793a_6642_9593_2c_73fe_5728_5024_2c_6700_5c0f_2c_6700_5927_2c_30aa_30d7_30b7_30:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ctimeinput_2cID_2c_8868_793a_6642_9593_2c_6642_2c_5206_2c_79d2_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnConfigurationDialogHelp:1",
  "ukadoc:list_shiori_event:OnSystemDialog:1",
  "ukadoc:list_shiori_event:OnSystemDialogCancel:1",
  "ukadoc:list_shiori_event:OnUserInput:1",
  "ukadoc:list_shiori_event:OnUserInputCancel:1",
  "ukadoc:list_shiori_event:inputbox.autocomplete:1",
  "ukadoc:list_shiori_resource:_28_5165_529b_30dc_30c3_30af_30b9_7a2e_985e_29.defaultleft_20_28_5165_529b_30dc_30c3_30af_30b9_7a2e_985e_29.defaulttop:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cdialog_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bclose_2cinputbox_2cID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdateinput_2cID_2c_8868_793a_6642_9593_2c_5e74_2c_6708_2c_65e5_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2ccolor_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2cfolder_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2copen_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdialog_2csave_2c_30d1_30e9_30e1_30fc_30bf_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cipinput_2cID_2c_8868_793a_6642_9593_2cIP1_6841_76ee_2cIP2_6841_76ee_2cIP3_6841_76ee_2cIP4_6841_76ee_2c_3:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cpasswordinput_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2csliderinput_2cID_2c_8868_793a_6642_9593_2c_73fe_5728_5024_2c_6700_5c0f_2c_6700_5927_2c_30aa_30d7_30b7_30:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ctimeinput_2cID_2c_8868_793a_6642_9593_2c_6642_2c_5206_2c_79d2_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnConfigurationDialogHelp:1",
  "ukadoc:list_shiori_event:OnSystemDialog:1",
  "ukadoc:list_shiori_event:OnSystemDialogCancel:1",
  "ukadoc:list_shiori_event:OnUserInput:1",
  "ukadoc:list_shiori_event:OnUserInputCancel:1",
  "ukadoc:list_shiori_event:inputbox.autocomplete:1",
  "ukadoc:list_shiori_resource:_28_5165_529b_30dc_30c3_30af_30b9_7a2e_985e_29.defaultleft_20_28_5165_529b_30dc_30c3_30af_30b9_7a2e_985e_29.defaulttop:1",
]
domains = ["sakura-script", "shiori"]
foundation = "入力窓と OS のダイアログを出して答えをイベントで返す経路"
breakage = "黙って壊れる"
themes = ["掛け合い", "交わり"]
```

**成立に要る最小の基盤**: 文字・数値・時刻・色・ファイルを尋ねる窓を出し、入力された値か取り消しをイベントとして返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 名前や答えを尋ねる場面で窓が出ず、会話がそこで止まる。ファイルや色を選ばせる流れも先へ進まない。

構成 id は 19 件で、うち機械の束から来たものが 0 件、人手で足したものが 19 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 19 件はすべて `hand` に載る。

### 動作モードの出入り（段階は段 3 で決める）

```toml
[bundle."動作モードの出入り"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5benter_2cinductionmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2conlinemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cpassivemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cselectmode_2c_30e2_30fc_30c9_28rect_29_2c_5de6_2c_4e0a_2c_53f3_2c_4e0b_5d_5c_21_5benter_2cselectmode_2c:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cinductionmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cnouserbreakmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2conlinemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cpassivemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cselectmode_5d:1",
  "ukadoc:list_shiori_event:OnSelectModeBegin:1",
  "ukadoc:list_shiori_event:OnSelectModeCancel:1",
  "ukadoc:list_shiori_event:OnSelectModeComplete:1",
  "ukadoc:list_shiori_event:OnSelectModeMouseDown:1",
  "ukadoc:list_shiori_event:OnSelectModeMouseUp:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5benter_2cinductionmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cnouserbreakmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2conlinemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cpassivemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5benter_2cselectmode_2c_30e2_30fc_30c9_28rect_29_2c_5de6_2c_4e0a_2c_53f3_2c_4e0b_5d_5c_21_5benter_2cselectmode_2c:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cinductionmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cnouserbreakmode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2conlinemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cpassivemode_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bleave_2cselectmode_5d:1",
  "ukadoc:list_shiori_event:OnSelectModeBegin:1",
  "ukadoc:list_shiori_event:OnSelectModeCancel:1",
  "ukadoc:list_shiori_event:OnSelectModeComplete:1",
  "ukadoc:list_shiori_event:OnSelectModeMouseDown:1",
  "ukadoc:list_shiori_event:OnSelectModeMouseUp:1",
]
domains = ["sakura-script", "shiori"]
foundation = "ふるまいの型を切り替えて出入りを通知する状態機械"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 台本が指示するふるまいの型に出入りし、出入りの節目をイベントで知らせられること。

**欠けると壊れる既存ゴーストの振る舞い**: 話しかけを断る型や割り込みを止める型に入れず、長い台詞の途中で撫でられて話が飛ぶ。画面の一部を四角く選ばせる流れも始まらない。

構成 id は 15 件で、うち機械の束から来たものが 0 件、人手で足したものが 15 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 15 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 15 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### キーとゲームパッド（段階は段 3 で決める）

```toml
[bundle."キーとゲームパッド"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnGamepadAxisMove:1",
  "ukadoc:list_shiori_event:OnGamepadButtonDown:1",
  "ukadoc:list_shiori_event:OnGamepadButtonUp:1",
  "ukadoc:list_shiori_event:OnGamepadConnected:1",
  "ukadoc:list_shiori_event:OnGamepadDisconnected:1",
  "ukadoc:list_shiori_event:OnKeyPress:1",
]
hand = [
  "ukadoc:list_shiori_event:OnGamepadAxisMove:1",
  "ukadoc:list_shiori_event:OnGamepadButtonDown:1",
  "ukadoc:list_shiori_event:OnGamepadButtonUp:1",
  "ukadoc:list_shiori_event:OnGamepadConnected:1",
  "ukadoc:list_shiori_event:OnGamepadDisconnected:1",
  "ukadoc:list_shiori_event:OnKeyPress:1",
]
domains = ["shiori"]
foundation = "マウス以外の入力装置の通知を受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["触れ合い", "気配り"]
```

**成立に要る最小の基盤**: キーボードとゲームパッドの押し下げと傾きを受け取り、SHIORI へ送れること。

**欠けると壊れる既存ゴーストの振る舞い**: キーを叩いてもゴーストに届かず、手を出した手応えが返らない。ゲームパッドを繋いでも外しても気づいてもらえない。

構成 id は 6 件で、うち機械の束から来たものが 0 件、人手で足したものが 6 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 6 件はすべて `hand` に載る。

### 投げ込み（段階は段 3 で決める）

```toml
[bundle."投げ込み"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnDirectoryDrop:1",
  "ukadoc:list_shiori_event:OnFileDrop2:1",
  "ukadoc:list_shiori_event:OnFileDropping:1",
  "ukadoc:list_shiori_event:OnOtherObjectDropped:1",
  "ukadoc:list_shiori_event:OnOtherObjectDropping:1",
  "ukadoc:list_shiori_event:OnTextDrop:1",
  "ukadoc:list_shiori_event:OnURLDragDropping:1",
  "ukadoc:list_shiori_event:OnURLDropFailure:1",
  "ukadoc:list_shiori_event:OnURLDropped:1",
  "ukadoc:list_shiori_event:OnURLDropping:1",
]
hand = [
  "ukadoc:list_shiori_event:OnDirectoryDrop:1",
  "ukadoc:list_shiori_event:OnFileDrop2:1",
  "ukadoc:list_shiori_event:OnFileDropping:1",
  "ukadoc:list_shiori_event:OnOtherObjectDropped:1",
  "ukadoc:list_shiori_event:OnOtherObjectDropping:1",
  "ukadoc:list_shiori_event:OnTextDrop:1",
  "ukadoc:list_shiori_event:OnURLDragDropping:1",
  "ukadoc:list_shiori_event:OnURLDropFailure:1",
  "ukadoc:list_shiori_event:OnURLDropped:1",
  "ukadoc:list_shiori_event:OnURLDropping:1",
]
domains = ["shiori"]
foundation = "窓へ落とされたものを受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["触れ合い"]
```

**成立に要る最小の基盤**: ゴーストの窓へ落とされたファイル・フォルダ・文字列・URL を受け取り、種類ごとに分けて SHIORI へ送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 画像や文章を渡しても受け取ってもらえず、落としたものが何も言われずに消える。

構成 id は 10 件で、うち機械の束から来たものが 0 件、人手で足したものが 10 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 10 件はすべて `hand` に載る。

### OS の変化の察知（段階は段 3 で決める）

```toml
[bundle."OS の変化の察知"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnCPULoadHigh:1",
  "ukadoc:list_shiori_event:OnCPULoadLow:1",
  "ukadoc:list_shiori_event:OnDarkTheme:1",
  "ukadoc:list_shiori_event:OnDeviceArrival:1",
  "ukadoc:list_shiori_event:OnDeviceRemove:1",
  "ukadoc:list_shiori_event:OnLanguageChange:1",
  "ukadoc:list_shiori_event:OnMemoryLoadHigh:1",
  "ukadoc:list_shiori_event:OnMemoryLoadLow:1",
  "ukadoc:list_shiori_event:OnNetworkHeavy:1",
  "ukadoc:list_shiori_event:OnNetworkStatusChange:1",
  "ukadoc:list_shiori_event:OnNotifyInternationalInfo:1",
  "ukadoc:list_shiori_event:OnNotifyOSInfo:1",
  "ukadoc:list_shiori_event:OnOSUpdateInfo:1",
  "ukadoc:list_shiori_event:OnSessionDisconnect:1",
  "ukadoc:list_shiori_event:OnSessionLock:1",
  "ukadoc:list_shiori_event:OnSessionReconnect:1",
  "ukadoc:list_shiori_event:OnSessionUnlock:1",
  "ukadoc:list_shiori_event:OnTabletMode:1",
  "ukadoc:list_shiori_event:OnVirtualDesktopChanged:1",
]
hand = [
  "ukadoc:list_shiori_event:OnCPULoadHigh:1",
  "ukadoc:list_shiori_event:OnCPULoadLow:1",
  "ukadoc:list_shiori_event:OnDarkTheme:1",
  "ukadoc:list_shiori_event:OnDeviceArrival:1",
  "ukadoc:list_shiori_event:OnDeviceRemove:1",
  "ukadoc:list_shiori_event:OnLanguageChange:1",
  "ukadoc:list_shiori_event:OnMemoryLoadHigh:1",
  "ukadoc:list_shiori_event:OnMemoryLoadLow:1",
  "ukadoc:list_shiori_event:OnNetworkHeavy:1",
  "ukadoc:list_shiori_event:OnNetworkStatusChange:1",
  "ukadoc:list_shiori_event:OnNotifyInternationalInfo:1",
  "ukadoc:list_shiori_event:OnNotifyOSInfo:1",
  "ukadoc:list_shiori_event:OnOSUpdateInfo:1",
  "ukadoc:list_shiori_event:OnSessionDisconnect:1",
  "ukadoc:list_shiori_event:OnSessionLock:1",
  "ukadoc:list_shiori_event:OnSessionReconnect:1",
  "ukadoc:list_shiori_event:OnSessionUnlock:1",
  "ukadoc:list_shiori_event:OnTabletMode:1",
  "ukadoc:list_shiori_event:OnVirtualDesktopChanged:1",
]
domains = ["shiori"]
foundation = "OS の状態変化の通知を受け取って SHIORI へ送る経路"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: OS が知らせる状態の変化を受け取り、対応するイベントとして SHIORI へ送れること（既存の段階 C 相当の 5 束と同じ基盤である）。

**欠けると壊れる既存ゴーストの振る舞い**: 機械が重いこと、画面の配色が暗い装いに変わったこと、席を外して鍵が掛かったことに気づかず、そのどれにも触れずに黙っている。

構成 id は 19 件で、うち機械の束から来たものが 0 件、人手で足したものが 19 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 19 件はすべて `hand` に載る。

### ごみ箱（段階は段 3 で決める）

```toml
[bundle."ごみ箱"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cemptyrecyclebin_5d:1",
  "ukadoc:list_shiori_event:OnRecycleBinEmpty:1",
  "ukadoc:list_shiori_event:OnRecycleBinEmptyFromOther:1",
  "ukadoc:list_shiori_event:OnRecycleBinStatusUpdate:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cemptyrecyclebin_5d:1",
  "ukadoc:list_shiori_event:OnRecycleBinEmpty:1",
  "ukadoc:list_shiori_event:OnRecycleBinEmptyFromOther:1",
  "ukadoc:list_shiori_event:OnRecycleBinStatusUpdate:1",
]
domains = ["sakura-script", "shiori"]
foundation = "ごみ箱の中身を読み、空にする OS の口"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: ごみ箱の中身の量を読み、空にする指示を OS へ渡し、空になったことを通知として受け取れること。

**欠けると壊れる既存ゴーストの振る舞い**: ごみ箱が溜まっていることを教えてくれず、片付けを促す台詞も出ない。台本からごみ箱を空にすることもできない。

構成 id は 4 件で、うち機械の束から来たものが 0 件、人手で足したものが 4 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 4 件はすべて `hand` に載る。

### 壁紙（段階は段 3 で決める）

```toml
[bundle."壁紙"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5brestore_2cwallpaper_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsave_2cwallpaper_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwallpaper_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnWallpaperChange:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5brestore_2cwallpaper_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bsave_2cwallpaper_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2cwallpaper_2c_30d5_30a1_30a4_30eb_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnWallpaperChange:1",
]
domains = ["sakura-script", "shiori"]
foundation = "壁紙の読み書きと変更の通知"
breakage = "黙って壊れる"
themes = ["触れ合い"]
```

**成立に要る最小の基盤**: 壁紙の絵を保存して元に戻し、差し替え、壁紙が変わったことを通知として受け取れること。

**欠けると壊れる既存ゴーストの振る舞い**: 壁紙を変えても気づかず、壁紙を差し替えて遊ぶ台本も動かない。

構成 id は 4 件で、うち機械の束から来たものが 0 件、人手で足したものが 4 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 4 件はすべて `hand` に載る。

### 通知領域（段階は段 3 で決める）

```toml
[bundle."通知領域"]
machine = []
members = [
  "ukadoc:descript_ghost:icon_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:char_2a.icon.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.icon.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.icon.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.icon.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.icon.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.icon.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.icon.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.icon.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.icon.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:icon.rect_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2ctasktrayicon_2c_30d5_30a1_30a4_30eb_540d.ico_2c_30c6_30ad_30b9_30c8_28_2c--duration_3d_5f85_6a5f_6642_959:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2ctrayballoon_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_shiori_event:OnTrayBalloonClick:1",
  "ukadoc:list_shiori_event:OnTrayBalloonTimeout:1",
]
hand = [
  "ukadoc:descript_ghost:icon_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_shell:char_2a.icon.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.icon.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:char_2a.icon.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.icon.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.icon.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:kero.icon.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.icon.frame.color.b_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.icon.frame.color.g_2c_6570_5024:1",
  "ukadoc:descript_shell:sakura.icon.frame.color.r_2c_6570_5024:1",
  "ukadoc:descript_shell_surfaces:icon.rect_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2ctasktrayicon_2c_30d5_30a1_30a4_30eb_540d.ico_2c_30c6_30ad_30b9_30c8_28_2c--duration_3d_5f85_6a5f_6642_959:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2ctrayballoon_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1",
  "ukadoc:list_shiori_event:OnTrayBalloonClick:1",
  "ukadoc:list_shiori_event:OnTrayBalloonTimeout:1",
]
domains = ["assets", "sakura-script", "shiori"]
foundation = "通知領域とゴースト一覧の画面"
breakage = "黙って壊れる"
themes = ["気配り"]
```

**成立に要る最小の基盤**: 画面の通知領域にゴーストの絵と吹き出しを出し、押されたことと時間切れをイベントで返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 隠しているあいだにゴーストがそこにいる印が画面に残らず、呼び戻す手がかりが無くなる。台本が出す通知も届かない。

構成 id は 15 件で、うち機械の束から来たものが 0 件、人手で足したものが 15 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 15 件はすべて `hand` に載る。

### 予定表（段階は段 3 で決める）

```toml
[bundle."予定表"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnSchedule5MinutesToGo:1",
  "ukadoc:list_shiori_event:OnScheduleRead:1",
  "ukadoc:list_shiori_event:OnSchedulepostBegin:1",
  "ukadoc:list_shiori_event:OnSchedulepostComplete:1",
  "ukadoc:list_shiori_event:OnSchedulesenseBegin:1",
  "ukadoc:list_shiori_event:OnSchedulesenseComplete:1",
  "ukadoc:list_shiori_event:OnSchedulesenseFailure:1",
  "ukadoc:list_shiori_event:calendarpluginpathlist:1",
  "ukadoc:list_shiori_event:calendarskinpathlist:1",
]
hand = [
  "ukadoc:list_shiori_event:OnSchedule5MinutesToGo:1",
  "ukadoc:list_shiori_event:OnScheduleRead:1",
  "ukadoc:list_shiori_event:OnSchedulepostBegin:1",
  "ukadoc:list_shiori_event:OnSchedulepostComplete:1",
  "ukadoc:list_shiori_event:OnSchedulesenseBegin:1",
  "ukadoc:list_shiori_event:OnSchedulesenseComplete:1",
  "ukadoc:list_shiori_event:OnSchedulesenseFailure:1",
  "ukadoc:list_shiori_event:calendarpluginpathlist:1",
  "ukadoc:list_shiori_event:calendarskinpathlist:1",
]
domains = ["shiori"]
foundation = "予定表の読み取りと予定の通知"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 予定表の中身を読み、予定の時刻が近づいたことと読み書きの結果をイベントで知らせられること。

**欠けると壊れる既存ゴーストの振る舞い**: 予定を教えてくれず、約束の前に声を掛けてもらえない。

構成 id は 9 件で、うち機械の束から来たものが 0 件、人手で足したものが 9 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 9 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 9 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 読み上げと聞き取り（段階は段 3 で決める）

```toml
[bundle."読み上げと聞き取り"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnSpeechSynthesisStatus:1",
  "ukadoc:list_shiori_event:OnVoiceRecognitionStatus:1",
  "ukadoc:list_shiori_event:OnVoiceRecognitionWord:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1",
  "ukadoc:list_shiori_event:OnSpeechSynthesisStatus:1",
  "ukadoc:list_shiori_event:OnVoiceRecognitionStatus:1",
  "ukadoc:list_shiori_event:OnVoiceRecognitionWord:1",
]
domains = ["sakura-script", "shiori"]
foundation = "音声合成と音声認識の口と状態の通知"
breakage = "黙って壊れる"
themes = ["交わり"]
```

**成立に要る最小の基盤**: 台詞を声にして読み上げ、話しかけられた言葉を文字にして SHIORI へ送れること。

**欠けると壊れる既存ゴーストの振る舞い**: 台詞が声にならず、話しかけても言葉として届かない。

構成 id は 4 件で、うち機械の束から来たものが 0 件、人手で足したものが 4 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 4 件はすべて `hand` に載る。

### 書庫（段階は段 3 で決める）

```toml
[bundle."書庫"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2ccompressarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_3:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cextractarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_30:1",
  "ukadoc:list_shiori_event:OnCompressArchiveComplete:1",
  "ukadoc:list_shiori_event:OnCompressArchiveFailure:1",
  "ukadoc:list_shiori_event:OnExtractArchiveComplete:1",
  "ukadoc:list_shiori_event:OnExtractArchiveFailure:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2ccompressarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_3:1",
  "ukadoc:list_sakura_script:_5c_21_5bexecute_2cextractarchive_2c_30d5_30a1_30a4_30eb_540d_2c_30c7_30a3_30ec_30af_30c8_30ea_540d_2c_30aa_30d7_30b7_30:1",
  "ukadoc:list_shiori_event:OnCompressArchiveComplete:1",
  "ukadoc:list_shiori_event:OnCompressArchiveFailure:1",
  "ukadoc:list_shiori_event:OnExtractArchiveComplete:1",
  "ukadoc:list_shiori_event:OnExtractArchiveFailure:1",
]
domains = ["sakura-script", "shiori"]
foundation = "書庫ファイルの読み書きと結果の通知"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 書庫ファイルを作り、また展開し、終わったか失敗したかをイベントで返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 台本から荷物をまとめたり開いたりできず、待っていても終わりの合図が来ない。

構成 id は 6 件で、うち機械の束から来たものが 0 件、人手で足したものが 6 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 6 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 6 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### SHIORI の要求と応答（段階は段 3 で決める）

```toml
[bundle."SHIORI の要求と応答"]
machine = []
members = [
  "ukadoc:spec_dll",
  "ukadoc:spec_shiori3:Age_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:BalloonOffset_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:BaseID_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Charset:1",
  "ukadoc:spec_shiori3:Charset:2",
  "ukadoc:spec_shiori3:ErrorDescription_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:ErrorLevel_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:ID:1",
  "ukadoc:spec_shiori3:MarkerSend_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Marker_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Reference0:1",
  "ukadoc:spec_shiori3:Reference1_7e:1",
  "ukadoc:spec_shiori3:Reference_2a:1",
  "ukadoc:spec_shiori3:SecurityLevel:1",
  "ukadoc:spec_shiori3:SecurityLevel_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:SecurityOrigin_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Sender:1",
  "ukadoc:spec_shiori3:Sender:2",
  "ukadoc:spec_shiori3:SenderType_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Value:1",
  "ukadoc:spec_shiori3:ValueNotify_20_5bSSP_62e1_5f35_202.5.35_5d:1",
  "ukadoc:spec_shiori3:_30b9_30c6_30fc_30bf_30b9_30b3_30fc_30c9:1",
  "ukadoc:spec_shiori3:_30e1_30bd_30c3_30c9:1",
]
hand = [
  "ukadoc:spec_dll",
  "ukadoc:spec_shiori3:Age_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:BalloonOffset_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:BaseID_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Charset:1",
  "ukadoc:spec_shiori3:Charset:2",
  "ukadoc:spec_shiori3:ErrorDescription_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:ErrorLevel_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:ID:1",
  "ukadoc:spec_shiori3:MarkerSend_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Marker_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Reference0:1",
  "ukadoc:spec_shiori3:Reference1_7e:1",
  "ukadoc:spec_shiori3:Reference_2a:1",
  "ukadoc:spec_shiori3:SecurityLevel:1",
  "ukadoc:spec_shiori3:SecurityLevel_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:SecurityOrigin_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Sender:1",
  "ukadoc:spec_shiori3:Sender:2",
  "ukadoc:spec_shiori3:SenderType_20_5bSSP_202.5.05_7e_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Status_20_5bSSP_62e1_5f35_5d:1",
  "ukadoc:spec_shiori3:Value:1",
  "ukadoc:spec_shiori3:ValueNotify_20_5bSSP_62e1_5f35_202.5.35_5d:1",
  "ukadoc:spec_shiori3:_30b9_30c6_30fc_30bf_30b9_30b3_30fc_30c9:1",
  "ukadoc:spec_shiori3:_30e1_30bd_30c3_30c9:1",
]
domains = ["shiori"]
foundation = "SHIORI/3.0 の要求と応答の組み立てと解釈"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: SHIORI/3.0 の要求行とヘッダを組み立てて送り、返ってきた応答の状態番号とヘッダを解けること。

**欠けると壊れる既存ゴーストの振る舞い**: 話しかけの合図そのものが通じず、ゴーストは何を尋ねられているかも分からないまま黙る。届いた答えの中の追加の指示も読み落とす。

構成 id は 25 件で、うち機械の束から来たものが 0 件、人手で足したものが 25 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 25 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 25 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 一覧と汎用プロパティの照会（段階は段 3 で決める）

```toml
[bundle."一覧と汎用プロパティの照会"]
machine = []
members = [
  "ukadoc:list_propertysystem:balloonlist.count:1",
  "ukadoc:list_propertysystem:balloonlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:balloonlist_28_30d0_30eb_30fc_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:baseware.name:1",
  "ukadoc:list_propertysystem:baseware.version:1",
  "ukadoc:list_propertysystem:craftmanurl:1",
  "ukadoc:list_propertysystem:craftmanw:1",
  "ukadoc:list_propertysystem:currentghost._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.balloon._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.balloon.count:1",
  "ukadoc:list_propertysystem:currentghost.status:1",
  "ukadoc:list_propertysystem:ghostlist.count:1",
  "ukadoc:list_propertysystem:ghostlist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:ghostlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.icon:1",
  "ukadoc:list_propertysystem:history.balloon.count:1",
  "ukadoc:list_propertysystem:history.balloon.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.balloon_28_30d0_30eb_30fc_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.ghost.count:1",
  "ukadoc:list_propertysystem:history.ghost.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.ghost_28_30b4_30fc_30b9_30c8_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:homeurl:1",
  "ukadoc:list_propertysystem:index:1",
  "ukadoc:list_propertysystem:keroname:1",
  "ukadoc:list_propertysystem:name:1",
  "ukadoc:list_propertysystem:path:1",
  "ukadoc:list_propertysystem:sakuraname:1",
  "ukadoc:list_propertysystem:shiori._5909_6570_540d:1",
  "ukadoc:list_propertysystem:thumbnail:1",
  "ukadoc:list_propertysystem:update_result:1",
  "ukadoc:list_propertysystem:update_time:1",
  "ukadoc:list_propertysystem:username:1",
  "ukadoc:list_shiori_event:balloonpathlist:1",
  "ukadoc:list_shiori_event:basewareversion:1",
  "ukadoc:list_shiori_event:capability:1",
  "ukadoc:list_shiori_event:ghostpathlist:1",
]
hand = [
  "ukadoc:list_propertysystem:balloonlist.count:1",
  "ukadoc:list_propertysystem:balloonlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:balloonlist_28_30d0_30eb_30fc_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:baseware.name:1",
  "ukadoc:list_propertysystem:baseware.version:1",
  "ukadoc:list_propertysystem:craftmanurl:1",
  "ukadoc:list_propertysystem:craftmanw:1",
  "ukadoc:list_propertysystem:currentghost._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.balloon._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:currentghost.balloon.count:1",
  "ukadoc:list_propertysystem:currentghost.status:1",
  "ukadoc:list_propertysystem:ghostlist.count:1",
  "ukadoc:list_propertysystem:ghostlist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:ghostlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.icon:1",
  "ukadoc:list_propertysystem:history.balloon.count:1",
  "ukadoc:list_propertysystem:history.balloon.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.balloon_28_30d0_30eb_30fc_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.ghost.count:1",
  "ukadoc:list_propertysystem:history.ghost.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:history.ghost_28_30b4_30fc_30b9_30c8_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1",
  "ukadoc:list_propertysystem:homeurl:1",
  "ukadoc:list_propertysystem:index:1",
  "ukadoc:list_propertysystem:keroname:1",
  "ukadoc:list_propertysystem:name:1",
  "ukadoc:list_propertysystem:path:1",
  "ukadoc:list_propertysystem:sakuraname:1",
  "ukadoc:list_propertysystem:shiori._5909_6570_540d:1",
  "ukadoc:list_propertysystem:thumbnail:1",
  "ukadoc:list_propertysystem:update_result:1",
  "ukadoc:list_propertysystem:update_time:1",
  "ukadoc:list_propertysystem:username:1",
  "ukadoc:list_shiori_event:balloonpathlist:1",
  "ukadoc:list_shiori_event:basewareversion:1",
  "ukadoc:list_shiori_event:capability:1",
  "ukadoc:list_shiori_event:ghostpathlist:1",
]
domains = ["property", "shiori"]
foundation = "プロパティの問い合わせ口と値の解決"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 入っているゴースト・バルーン・シェルの一覧と履歴を、族の頭に汎用の名前を継ぎ足した綴りで引けること（既存の「環境の照会」と同じ基盤である）。

**欠けると壊れる既存ゴーストの振る舞い**: 入っているゴーストやバルーンの一覧も、これまで着てきたものの履歴もゴーストが引けず、手持ちに触れた話ができない。

構成 id は 37 件で、うち機械の束から来たものが 0 件、人手で足したものが 37 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 37 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 37 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 組み込みの置換語（段階は段 3 で決める）

```toml
[bundle."組み込みの置換語"]
machine = []
members = [
  "ukadoc:list_sakura_script:_25_2a:1",
  "ukadoc:list_sakura_script:_25day:1",
  "ukadoc:list_sakura_script:_25dms:1",
  "ukadoc:list_sakura_script:_25et:1",
  "ukadoc:list_sakura_script:_25exh:1",
  "ukadoc:list_sakura_script:_25hour:1",
  "ukadoc:list_sakura_script:_25m_3f:1",
  "ukadoc:list_sakura_script:_25mc:1",
  "ukadoc:list_sakura_script:_25me:1",
  "ukadoc:list_sakura_script:_25mh:1",
  "ukadoc:list_sakura_script:_25minute:1",
  "ukadoc:list_sakura_script:_25ml:1",
  "ukadoc:list_sakura_script:_25month:1",
  "ukadoc:list_sakura_script:_25mp:1",
  "ukadoc:list_sakura_script:_25ms:1",
  "ukadoc:list_sakura_script:_25mt:1",
  "ukadoc:list_sakura_script:_25mz:1",
  "ukadoc:list_sakura_script:_25screenheight:1",
  "ukadoc:list_sakura_script:_25screenwidth:1",
  "ukadoc:list_sakura_script:_25second:1",
  "ukadoc:list_sakura_script:_25wronghour:1",
  "ukadoc:list_sakura_script:_5c6:1",
]
hand = [
  "ukadoc:list_sakura_script:_25_2a:1",
  "ukadoc:list_sakura_script:_25day:1",
  "ukadoc:list_sakura_script:_25dms:1",
  "ukadoc:list_sakura_script:_25et:1",
  "ukadoc:list_sakura_script:_25exh:1",
  "ukadoc:list_sakura_script:_25hour:1",
  "ukadoc:list_sakura_script:_25m_3f:1",
  "ukadoc:list_sakura_script:_25mc:1",
  "ukadoc:list_sakura_script:_25me:1",
  "ukadoc:list_sakura_script:_25mh:1",
  "ukadoc:list_sakura_script:_25minute:1",
  "ukadoc:list_sakura_script:_25ml:1",
  "ukadoc:list_sakura_script:_25month:1",
  "ukadoc:list_sakura_script:_25mp:1",
  "ukadoc:list_sakura_script:_25ms:1",
  "ukadoc:list_sakura_script:_25mt:1",
  "ukadoc:list_sakura_script:_25mz:1",
  "ukadoc:list_sakura_script:_25screenheight:1",
  "ukadoc:list_sakura_script:_25screenwidth:1",
  "ukadoc:list_sakura_script:_25second:1",
  "ukadoc:list_sakura_script:_25wronghour:1",
  "ukadoc:list_sakura_script:_5c6:1",
]
domains = ["sakura-script"]
foundation = "台本の中の組み込みの語を値へ置き換える経路"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 台本の中の組み込みの語を、再生の前にその場の値へ置き換えられること。

**欠けると壊れる既存ゴーストの振る舞い**: 時刻や画面の大きさを指す語がそのままの綴りで台詞に出てしまい、意味の通らない記号を読まされる。

構成 id は 22 件で、うち機械の束から来たものが 0 件、人手で足したものが 22 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 22 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 22 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### イベントの呼び起こし（段階は段 3 で決める）

```toml
[bundle."イベントの呼び起こし"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bnotify_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5braise_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5btimernotify_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5btimerraise_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5ca:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bnotify_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5braise_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5btimernotify_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5c_21_5btimerraise_2c_6642_9593_2c_7e70_308a_8fd4_3059_304b_5426_304b_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1",
  "ukadoc:list_sakura_script:_5ca:1",
]
domains = ["sakura-script"]
foundation = "台本から SHIORI イベントを起こす経路"
breakage = "黙って壊れる"
themes = ["交わり"]
```

**成立に要る最小の基盤**: 台本から SHIORI のイベントを、その場でも時間を置いてからでも起こせること。

**欠けると壊れる既存ゴーストの振る舞い**: 台本が自分で次の場面を呼び出せず、話が一区切りで止まる。時間を置いて続きを始める仕掛けも動かない。

構成 id は 6 件で、うち機械の束から来たものが 0 件、人手で足したものが 6 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 6 件はすべて `hand` に載る。

### 同期オブジェクト（段階は段 3 で決める）

```toml
[bundle."同期オブジェクト"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5breset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bwait_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5breset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bset_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bwait_2csyncobject_2c_540c_671f_30aa_30d6_30b8_30a7_30af_30c8_540d_2c_30aa_30d7_30b7_30e7_30f3_5d:1",
]
domains = ["sakura-script"]
foundation = "複数の台本を待ち合わせる同期の器"
breakage = "黙って壊れる"
themes = []
```

**成立に要る最小の基盤**: 名前を付けた合図を立てて消し、その合図が立つまで台本を待たせられること。

**欠けると壊れる既存ゴーストの振る舞い**: 掛け合いの間が合わず、相手を待たずに一人だけ先に喋る。

構成 id は 3 件で、うち機械の束から来たものが 0 件、人手で足したものが 3 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 3 件はすべて `hand` に載る。テーマは **0 件**である（構成 id 3 件の `values` をすべて読み、空でないものが 1 件も無かった）。

### 休止と復帰（段階は段 3 で決める）

```toml
[bundle."休止と復帰"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnCacheRestore:1",
  "ukadoc:list_shiori_event:OnCacheSuspend:1",
]
hand = [
  "ukadoc:list_shiori_event:OnCacheRestore:1",
  "ukadoc:list_shiori_event:OnCacheSuspend:1",
]
domains = ["shiori"]
foundation = "裏へ回ったゴーストの SHIORI を休ませて呼び戻す経路"
breakage = "黙って壊れる"
themes = ["気配"]
```

**成立に要る最小の基盤**: 裏へ回ったゴーストの SHIORI をいったん休ませ、表に戻すときに呼び戻して、その両方をイベントで知らせられること。

**欠けると壊れる既存ゴーストの振る舞い**: 裏へ控えるときと戻ってきたときの一言が無く、出入りが素っ気なくなる。

構成 id は 2 件で、うち機械の束から来たものが 0 件、人手で足したものが 2 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 2 件はすべて `hand` に載る。

### 作り付けの窓（段階は段 3 で決める）

```toml
[bundle."作り付けの窓"]
machine = []
members = [
  "ukadoc:list_sakura_script:_5c_21_5bopen_2caddressbar_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2caigraph_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cbacklogviewer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cballoonexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ccalendar_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cconfigurationdialog_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdressupexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cghostexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2chelp_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cmessenger_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraph_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphballoon_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphtotal_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2creadme_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cshellexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cterms_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2caigraph_5d:1",
]
hand = [
  "ukadoc:list_sakura_script:_5c_21_5bopen_2caddressbar_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2caigraph_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cbacklogviewer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cballoonexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2ccalendar_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cconfigurationdialog_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cdressupexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cghostexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2chelp_2c_30c0_30a4_30a2_30ed_30b0ID_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cmessenger_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraph_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphballoon_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2crateofusegraphtotal_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2creadme_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cshellexplorer_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5bopen_2cterms_5d:1",
  "ukadoc:list_sakura_script:_5c_21_5breload_2caigraph_5d:1",
]
domains = ["sakura-script"]
foundation = "ベースウェアが持つ作り付けの窓を開く経路"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: ベースウェアが自分で持っている説明書・設定・一覧などの窓を、台本の指示で開けること。

**欠けると壊れる既存ゴーストの振る舞い**: 説明書や設定の窓へ台詞から案内できず、利用者は自分でメニューを探すことになる。

構成 id は 17 件で、うち機械の束から来たものが 0 件、人手で足したものが 17 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 17 件はすべて `hand` に載る。

### 薦める場所（段階は段 3 で決める）

```toml
[bundle."薦める場所"]
machine = []
members = [
  "ukadoc:list_shiori_event:OnRecommendsiteChoice:1",
  "ukadoc:list_shiori_resource:char_2a.recommendbuttoncaption:1",
  "ukadoc:list_shiori_resource:char_2a.recommendsites.caption:1",
  "ukadoc:list_shiori_resource:char_2a.recommendsites:1",
  "ukadoc:list_shiori_resource:kero.recommendbuttoncaption:1",
  "ukadoc:list_shiori_resource:kero.recommendsites:1",
  "ukadoc:list_shiori_resource:sakura.portalbuttoncaption:1",
  "ukadoc:list_shiori_resource:sakura.portalsites:1",
  "ukadoc:list_shiori_resource:sakura.recommendbuttoncaption:1",
  "ukadoc:list_shiori_resource:sakura.recommendsites:1",
]
hand = [
  "ukadoc:list_shiori_event:OnRecommendsiteChoice:1",
  "ukadoc:list_shiori_resource:char_2a.recommendbuttoncaption:1",
  "ukadoc:list_shiori_resource:char_2a.recommendsites.caption:1",
  "ukadoc:list_shiori_resource:char_2a.recommendsites:1",
  "ukadoc:list_shiori_resource:kero.recommendbuttoncaption:1",
  "ukadoc:list_shiori_resource:kero.recommendsites:1",
  "ukadoc:list_shiori_resource:sakura.portalbuttoncaption:1",
  "ukadoc:list_shiori_resource:sakura.portalsites:1",
  "ukadoc:list_shiori_resource:sakura.recommendbuttoncaption:1",
  "ukadoc:list_shiori_resource:sakura.recommendsites:1",
]
domains = ["shiori"]
foundation = "薦める行き先の一覧をメニューに並べて選びを返す経路"
breakage = "黙って壊れる"
themes = ["装い"]
```

**成立に要る最小の基盤**: ゴーストが薦める行き先の一覧を受け取ってメニューに並べ、選ばれた行き先をイベントで返せること。

**欠けると壊れる既存ゴーストの振る舞い**: 作者が案内したい場所がメニューに並ばず、そこから先へ進む道が無くなる。

構成 id は 10 件で、うち機械の束から来たものが 0 件、人手で足したものが 10 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 10 件はすべて `hand` に載る。

### 好感度の絵柄（段階は段 3 で決める）

```toml
[bundle."好感度の絵柄"]
machine = []
members = [
  "ukadoc:descript_ghost:shiori.logo.align_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:shiori.logo.file_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:shiori.logo.x_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:shiori.logo.y_2cY_5ea7_6a19:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminute:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminutemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminuteweekly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.boottime:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.boottimemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.boottimeweekly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.keroname:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.name:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.percent:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.percentmonthly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.percentweekly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.sakuraname:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.bootminute:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.bootminutemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.bootminuteweekly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottime:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottimemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottimeweekly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.keroname:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.name:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.percent:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.percentmonthly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.percentweekly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.sakuraname:1",
  "ukadoc:list_shiori_event:rateofusegraph:1",
]
hand = [
  "ukadoc:descript_ghost:shiori.logo.align_2c_4f4d_7f6e_60c5_5831:1",
  "ukadoc:descript_ghost:shiori.logo.file_2c_30d5_30a1_30a4_30eb_540d:1",
  "ukadoc:descript_ghost:shiori.logo.x_2cX_5ea7_6a19:1",
  "ukadoc:descript_ghost:shiori.logo.y_2cY_5ea7_6a19:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminute:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminutemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.bootminuteweekly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.boottime:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.boottimemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.boottimeweekly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.keroname:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.name:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.percent:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.percentmonthly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.percentweekly:1",
  "ukadoc:list_propertysystem:rateofuselist.index_28_9806_4f4d_29.sakuraname:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.bootminute:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.bootminutemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.bootminuteweekly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottime:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottimemonthly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottimeweekly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.keroname:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.name:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.percent:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.percentmonthly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.percentweekly:1",
  "ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.sakuraname:1",
  "ukadoc:list_shiori_event:rateofusegraph:1",
]
domains = ["assets", "property", "shiori"]
foundation = "好感度と使用時間の記録と絵柄の画面"
breakage = "黙って壊れる"
themes = ["記憶"]
```

**成立に要る最小の基盤**: ゴーストごとの起動時間と呼ばれた回数を記録し、順位と割合として引けること。絵柄の画面に載せる図も配布物から読めること。

**欠けると壊れる既存ゴーストの振る舞い**: どれだけ長く付き合ってきたかをゴーストが知らず、節目に触れる話ができない。使用時間の画面も空のまま出る。

構成 id は 29 件で、うち機械の束から来たものが 0 件、人手で足したものが 29 件である（`hand` の行を数えた）。**この束は `machine` が空配列である**——機械の束を核にせず、構成 id を人手だけで選んだので、`members` の 29 件はすべて `hand` に載る。

### 単独項目に落とす候補

この節はタスク 3.6 への引き渡し口である。**タスク 3.6 はこの節の箇条書きから id を拾う**——
1 行 1 件、行頭は `- ` で始まり、id は逆引用符で囲み、その後に「— 束へ入れない理由:」を続ける。
この形以外の行はこの節に置かない。

候補は **1** 件で、ドメイン別の内訳は assets **1** 件・property **0** 件・sakura-script **0** 件・shiori **0** 件である（数え方: 下の箇条書きの行を数え、
id を台帳の `[ledger] domain` で引いた）。うち関連 0 本のものは **1** 件である。

- `ukadoc:memo` — 束へ入れない理由: 台帳の備考が「この項目はページ 1 枚をまとめて指す粗い粒度で、ページの中の記述を 1 つずつ分けて持っていない」と書いており、同じ備考が中身を「ベースウェアの機能比較表と、おすすめサイトのバナーなどの雑多な覚え書き」と述べている。1 つの機構を指していないので、要件 4.1 ⑹ の「欠けると壊れる既存ゴーストの振る舞い」を 1 つに絞って書けない（要件 4.4）。

タスク 3.1 が「単独項目」と決めた `ukadoc:manual_balloon`・`ukadoc:manual_directory`・
`ukadoc:manual_ghost` の 3 件は、この節に重ねて書かない（行き先は「13 のページ単位の id の
行き先」の表が既に定めており、タスク 3.6 の持ち場である）。

### 例示の 3 連鎖が 1 つの束に収まること

要件 3.1 が例示する 3 つの連鎖について、束 id と構成 id で着地を示す。件数はこの節を書いた
時点で数え直した（数え方: `report/summary.md`「ドメインを跨いで繋がった束」の表からその束 id の
行を取り、構成 id の欄を読点で切って数えた。3 つとも段 1 の値と同じである）。

| 連鎖 | 収まる束 id | その束の構成 id 数 | 名前付き束 |
| --- | --- | ---: | --- |
| 時刻の刻み | `ukadoc:descript_plugin:secondchangeinterval_2c_79d2_6570:1` | 3 | `自発発話` |
| 重なり順 | `ukadoc:descript_shell:char_2a.menu_2cauto_307e_305f_306fhidden:1` | 49 | `窓の配置と重なり` |
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
  3 つとも名前付き束「窓の配置と重なり」の `members` に入っている（入っていない id は 0 件）。同じ機構の `ukadoc:list_sakura_script:_5c_21_5breset_2czorder_5d:1` も同じ束に入っており、4 件は 1 つの束に収まっている（数え方: 「窓の配置と重なり」の囲みの `members` から 4 件を引いた。欠けているものは **0 件**である）。この 3 つは `machine` に引いた機械の束の構成 id でもあるので `hand` には入れていない。4 件それぞれの向き無しの端の本数（この 3 つは 1 本・1 本・3 本を持ち、端が 0 本なのは `\![reset,zorder]` だけ）は、「このタスクへ回された id の始末」節の ⑶ に表で書いた。

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
