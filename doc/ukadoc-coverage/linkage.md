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

<!-- 段 2（タスク 3.2〜3.4）で書く: 束ごとに見出しを立て、直下に TOML の囲みを 1 つ置き、
     その下に本文で「成立に要る最小の基盤」と「欠けると壊れる既存ゴーストの振る舞い」を
     利用者から見える結果の差で書く。 -->

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
