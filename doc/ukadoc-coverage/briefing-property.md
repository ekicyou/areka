# プロパティのブリーフィング

対象は ukadoc のプロパティシステムのページ（`list_propertysystem`）の 188 項目。
台帳は `doc/ukadoc-coverage/ledger/property.toml`、そこから機械で作った報告は
`doc/ukadoc-coverage/report/property.md` にある。この文書はその 2 本を読むための手引きで、
台帳の 1 行ずつには収まらない突き合わせの結果をまとめる。

読む人は 2 種類いる。ひとつは 4 ドメインをまとめる担当（`areka-P0-ukadoc-coverage-roadmap`）で、
「正典の項目 1 つ 1 つが誰の持ち物か」を知りたい。もうひとつはこれから実装する 4 本の spec の担当者で、
「自分の brief に書いた枝と概数が、実際にはどの id の集まりなのか」を知りたい。

この文書は次の順で書く。

- 前置き（この節）
- ⑴ 所有の突合表 — 4 本の brief の記述と id 群の対応、および書き込みが有効な項目の突き合わせ
- ⑵ 同じ値へ到達する名前の一覧
- ⑶ 持ち主のいない項目・裁定待ちの項目の一覧
- ⑷ 二重所有の裁定案
- ⑸ 既存 brief への是正候補
- ⑹ カタログとの件数の差
- 末尾に検証の結果

## 前置き

### id の書き方の約束

このページの項目の id はすべて `ukadoc:list_propertysystem:` で始まる。
以下の表では、この共通の頭を省いて後ろの部分だけを書く。
たとえば `menu:1` と書いたら `ukadoc:list_propertysystem:menu:1` のことである。
別のページの id を指すときだけ、省略せずに全部書く。

id の中にある `_28_30ad_30fc_29` のような記号の並びは、見出しに含まれる日本語や記号を
正典側が符号化したものである。読みやすい形に直すと id が別物になるので、そのまま写している。

### 全項目に共通する読み取りの経路は 2 本

このページの 188 項目は、どれも次の 2 つの入口から読める。項目ごとに入口が違うわけではない。

| 入口 | 使う場所 | 正典の id（全部書く） |
|---|---|---|
| `%property[プロパティ名]` | 台詞の中に値をそのまま差し込む | `ukadoc:list_sakura_script:_25property_5b_30d7_30ed_30d1_30c6_30a3_540d_5d:1` |
| `\![get,property,イベント名,プロパティ名,プロパティ名,...]` | イベントを起こして値を受け取る | `ukadoc:list_sakura_script:_5c_21_5bget_2cproperty_2c_30a4_30d9_30f3_30c8_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_30d7_30ed_30d1_30c6_30a3_540d_2c:1` |

**この 2 本を台帳の各項目の「関連」に書かなかったのは意図的である。** 188 項目すべてに同じ 2 本を書くと
376 本の同じ関連が並び、報告が作る「関連でつながった塊」の区切りが全部つぶれて意味を失う。
そこで 2 本はここに 1 度だけ記録し、台帳には書かない。台帳の「関連」に照会経路が出てくるのは、
その項目の本文が名指ししている特定のタグやイベントに限られる。

書き込みの入口 `\![set,property,プロパティ名,値]`
（`ukadoc:list_sakura_script:_5c_21_5bset_2cproperty_2c_30d7_30ed_30d1_30c6_30a3_540d_2c_5024_5d:1`）は
全項目に効くわけではないので、扱いが違う。正典が書き込みを認めている 26 項目にだけ台帳から指している。
どの 26 項目かは ⑴ 節の後半にある。

### `????` は正典の書き方そのもの

見出しに `mouse????list` と書かれた項目が 5 件ある。id では `mouse_3f_3f_3f_3flist` の形になる。

この `????` は、写している途中で文字が落ちたものではない。**正典のページに最初からこう書いてある。**
`mouseuplist`・`mousedownlist`・`mousehoverlist`・`mousewheellist` の 4 つをまとめて指すための書き方で、
`????` の位置に `up`・`down`・`hover`・`wheel` のどれかが入る、という意味である。
台帳でもこの 5 件は正典の見出しどおりに `????` のまま持っている。

### 「縮退 8 件」と「語彙のみ 178 件」の実質的な差は登記の有無だけ

報告は 188 件を実装済み 2・縮退 8・語彙のみ 178 に分けている。この 3 つのうち、
**読んだときに値が返るのは実装済みの 2 件だけである。** 縮退の 8 件も語彙のみの 178 件も、
読めば同じように値なしが返り、辞書は空文字を前提に先へ進む。利用者にも作者にも何も見えない。

では 8 件と 178 件で何が違うのか。**違いは「正典との食い違いが既にどこかに書き留めてあるか」だけである。**
縮退の 8 件は `doc/COMPAT_ARCHITECTURE.md` に「正典はこう定めるが areka はこうする」と個別に登記済みで、
台帳の備考にもその行番号が入っている。178 件のほうは、同じように値が返らないのに、
その事実がまだどこにも登記されていない。

**「壊れているのは 8 件だけ」と読まないでほしい。** 利用者から見れば 186 件すべてが同じように無い。
8 という数は「調べて記録が済んでいる件数」であって「動かない件数」ではない。

### 報告の「関連が閉じている束」が空になる理由

台帳には関連が **73 本**書いてある（内訳は「設定元」＝台帳での種別名 `configures` が 24 本・
「同じ機能」＝同 `same-feature` が 49 本）。
それなのに `report/property.md` は「ドメイン内で閉じている束はありません。」と出す。矛盾ではない。

報告を作る側は、**両端ともこの台帳の中にある関連だけを塊として残す**作りになっている
（`crates/ukadoc-survey/src/report/domain.rs:203-213`）。
このドメインの 73 本は測ってみると **1 本残らず他のページを指している**——
さくらスクリプトのページへ 40 本・シェルの定義ファイルへ 13 本・ゴーストの定義ファイルへ 9 本・
SHIORI のイベントへ 4 本・プラグインのイベントへ 4 本・シェルのサーフェス定義へ 2 本・
SHIORI3 の仕様へ 1 本。両端が中にある関連は 0 本なので、塊は 1 つも作られない。

**この節が無いと、報告だけを読んだ人は「この台帳には関連が 1 本も書かれていない」と誤解する。**
実際は逆で、このドメインの項目は値を自分で持たず、ほぼ全部が外を向いている。
外へ伸びた関連の集計は 4 ドメインをまとめる側（`report/summary.md`）の担当である。

### テーマ「記憶」を付けなかったことは ⑷ 節にある

テーマ定義書 `doc/ukadoc-coverage/values.md` がこのドメインの id を「記憶」の代表として 2 件挙げているのに
本調査がテーマを付けなかった件は、未決の裁定の 1 つとして ⑷ 節にまとめてある。

## ⑴ 所有の突合表

### 表の見方

4 本の brief は、所有する範囲を「枝の名前」と「おおよその件数」で書いている。
**id を 1 つも列挙していない。** そこで、brief の記述 1 つを 1 行として並べ、
それが実際にはどの id の集まりなのかを「接頭辞＋件数」で示す。

- 「brief の書き方」の列は、brief に書いてある件数をそのまま写す。言い回しも brief のままに
  するので、areka 側の語彙表を `sylphya` と呼んでいる行がある。この文書のほかの場所では
  「areka 側の語彙表」と書いている。
- 「実測」の列は台帳とカタログを数えた結果で、**食い違ったときはカタログを正とする。**
- 食い違いのある行だけ、あとで id を全部並べる。一致している行は接頭辞と件数だけにする。
- brief の記述に対応する id が見つからないものは、表記の揺れとしてそのまま残す。
  近そうな id へ憶測で結び付けることはしない。

担当欄（台帳の `owner`）の分布は 188 件の内訳と一致している——
`areka-P0-property-catalog-lists` 120・`areka-P0-currentghost-property-tree` 64・
`areka-P0-sylphya`（完了済み）2・空文字 2。

### `areka-P0-property-catalog-lists`（担当 120 件）

| # | brief の場所 | brief の書き方 | 実測の id 群（接頭辞＋件数） | 突合 |
|---|---|---|---|---|
| L1 | brief:16 | `system.*` 25 | `system.` 25 | 一致 |
| L2 | brief:17 | `ghostlist` ×5・`activeghostlist` ×5＋`.ext` ＝ 12 | `ghostlist` 5 ＋ `activeghostlist` 5 ＝ **10** | **食い違い**（下に全列挙） |
| L3 | brief:18 | `balloonlist` ×3・`headlinelist` ×2・`pluginlist` ×4＋`.ext` ＝ 11 | `balloonlist` 3 ＋ `headlinelist` **3** ＋ `pluginlist` **5** ＝ 11 | **食い違い**（合計だけ一致・下に全列挙） |
| L4 | brief:19 | `history.*` 8 | `history.` **12** | **食い違い**（下に全列挙） |
| L5 | brief:20 | `rateofuselist.*` 24 | `rateofuselist` 24（`(名前)` 形 12・`.index(順位)` 形 12） | 一致 |
| L6 | brief:21 | 汎用プロパティ名（共有葉）17 | 見出しが葉だけの汎用名 **13** ＋ `menu` の群 **4** ＝ 17 | **食い違い**（合計だけ一致・下に全列挙） |
| L7 | brief:22 | `currentghost.sound.*` ×3＋サウンド語彙族 ≈21 | `currentghost.sound` 3 ＋ サウンドの要素葉 18 ＝ 21 | 一致 |
| L8 | brief:23 | `.ext.拡張プロパティ名`（逆方向・件数を書いていない） | `activeghostlist` 2 ＋ `pluginlist` 2 ＝ 4（L2・L3 に含まれる） | 件数の記載が無く突き合わせられない |

L8 の 4 件は L2・L3 の中に既に入っている。ここで数え直すと二重になるので、内訳の切り直しとして扱う。

**この spec の担当件数の検算**: 25 ＋ 10 ＋ 11 ＋ 12 ＋ 24 ＋ 17 ＋ 21 ＝ **120**。台帳の担当欄の件数と一致する。

### `areka-P0-currentghost-property-tree`（担当 64 件）

| # | brief の場所 | brief の書き方 | 実測の id 群（接頭辞＋件数） | 突合 |
|---|---|---|---|---|
| T1 | brief:23 | `currentghost.balloon.scope(ID).*` ×17 ＋ `balloon.汎用` ＋ `balloon.count` ＝ 19 | `currentghost.balloon.scope(ID).` 17 ＋ `currentghost.balloon.汎用プロパティ名` 1 ＋ `currentghost.balloon.count` 1 ＝ 19 | 一致（17 の名前も 1 つずつ合う） |
| T2 | brief:23 の括弧書き | 「＋`mousecursor` 系 4 は SET 有効側」（どの件数にも足していない） | `currentghost.balloon.mousecursor` 4 | **食い違い**（下に全列挙） |
| T3 | brief:25 | `currentghost.scope(ID).*` ＋ `.scope.count` ×17 | `currentghost.scope(ID).` 16 ＋ `currentghost.scope.count` 1 ＝ 17 | 一致 |
| T4 | brief:26 | `currentghost.mousecursor.*` ×6（全 SET 有効） | `currentghost.mousecursor` 1 ＋ `currentghost.mousecursor.` 5 ＝ 6 | 一致 |
| T5 | brief:26 | `currentghost.seriko.*` ×14 | `currentghost.seriko.` 14 | 件数は一致。ただし **2 件は担当欄が空文字**（`zorder`・`sticky-window`＝裁定待ち）なので、この spec が実際に持つのは 12 件 |
| T6 | brief:26 | `currentghost.shelllist.*` ×4 | `currentghost.shelllist` 4 | 一致 |
| T7 | brief:26 | `.status`・`.汎用` | `currentghost.status` 1 ＋ `currentghost.汎用プロパティ名` 1 ＝ 2 | 一致 |

**検算**: 19 ＋ 4 ＋ 17 ＋ 6 ＋ 14 ＋ 4 ＋ 2 ＝ **66**。このうち 2 件（`zorder`・`sticky-window`）は担当欄が空文字なので、
台帳の担当欄では **64** になる。66 ＋ `currentghost.sound` 3（L7 で lists が持つ）＝ **69** が
カタログの `currentghost` で始まる項目の総数である。

brief の冒頭（brief:4）と brief:22 は「≈65 項目」と書くが、brief 自身が件数として足しているのは
19 ＋ 17 ＋ 6 ＋ 14 ＋ 4 ＋ 2 ＝ 62 で、括弧書きの 4 件（T2）が抜けている。
65 という数と 69 との差の全体は ⑹ 節で扱う。

### `areka-P0-property-query-channels`（担当 0 件）

この spec は照会の経路そのものを持ち、値の木は持たない。したがって台帳の担当欄には 1 件も現れない。
それでも突合表に載せるのは、**この spec の主張が他 3 本の担当範囲と重なっているから**である
（重なっている側は「語彙表の 1 行だけを触る」ので、担当欄は値を導出する側が取る）。

| # | brief の場所 | brief の書き方 | 実測の id 群 | 突合 |
|---|---|---|---|---|
| C1 | brief:42 | 経路 1〜4 の実装（`\![get,property,…]`・`\![set,property,…]`・`%property[…]`・`\![embed,…]`） | このページに id なし。相手は `list_sakura_script` の項目（別ドメインの台帳の持ち物） | 対応 id 0 件（枝が違うのであって数え違いではない） |
| C2 | brief:44 | sylphya の書き込み有効一覧を 21 → 26 へ追随 | 正典が書き込みを認める **26 id**（下の突合表） | 一致（26）。26 件の担当欄は tree 17・lists 7・空文字 2 に散る |
| C3 | brief:46 | 経路 5（`.ext.*` の逆方向）は語彙の登記だけ | `activeghostlist` 2 ＋ `pluginlist` 2 ＝ 4 | 一致（4）。担当は lists。4 件の備考にこの spec の名前が入っている |
| C4 | brief:29 | サウンド語彙 ≈18 葉は族ごと不在 | サウンドの要素葉 **18** | 一致（18）。`currentghost.sound.*` の頭 3 件は含まない（brief が指しているのは葉） |
| C5 | brief:29 | 21 が先取りしていない 5 件（`seriko.zorder`・`seriko.sticky-window`・サウンドの 3 葉） | 5 id（下の突合表の区分 ⑵ と同じ） | 一致（5・id 単位で一致） |

### `areka-P0-zorder-property`（担当 0 件）

| # | brief の場所 | brief の書き方 | 実測の id 群 | 突合 |
|---|---|---|---|---|
| Z1 | brief:9・brief:13-17 | `currentghost.seriko.zorder` の読み書きの実導出と完全な語彙 | `currentghost.seriko.zorder:1` 1 件 | 件数は一致。ただし担当欄は**空文字**（裁定待ちが先に当たる）。⑷ 節で扱う |
| Z2 | brief:22 | sylphya の書き込み有効一覧に `seriko.zorder` は入れない・この brief が語彙の正本 | 同じ 1 件 | **食い違い**。C2（21 → 26 に含める）と正面からぶつかる。語彙表の 1 行の持ち主が 2 本いる |

### `areka-P0-sylphya`（完了済み・担当 2 件）

突合の対象である 4 本には入らないが、188 件の内訳としては必要なので併記する。

| # | 根拠 | 実測の id 群 | 突合 |
|---|---|---|---|
| S1 | 完了済み spec の要件が `baseware.name`／`baseware.version` の実導出を明記 | `baseware.name:1`・`baseware.version:1` の 2 件 | 一致。この 2 件だけが読んで値の返る項目 |

### 188 件の検算

| 担当 | 件数 | 出どころ |
|---|---:|---|
| `areka-P0-property-catalog-lists` | 120 | L1〜L7 の合計 |
| `areka-P0-currentghost-property-tree` | 64 | T1〜T7 の 66 から裁定待ち 2 を引いた数 |
| `areka-P0-sylphya`（完了済み） | 2 | S1 |
| 空文字（裁定待ち） | 2 | `currentghost.seriko.zorder`・`currentghost.seriko.sticky-window` |
| **合計** | **188** | 台帳の項目数と一致 |

`areka-P0-property-query-channels` と `areka-P0-zorder-property` の行はどれも
上のどれかに既に数えられている id を別の切り口で見たものなので、合計には足さない。
前者の主張はすべて語彙表の側（値を導出しない）で、後者の唯一の主張は裁定待ちに先取りされている。

### 食い違いのある行の id 全列挙

#### L2 — `ghostlist` ＋ `activeghostlist`（brief 12・実測 10）

brief は `.ext` の 2 件を `activeghostlist` の 5 件と別に数えているが、
実際には 5 件の中に既に入っている。二重に数えた分だけ 2 件多い。

| 見出し | id |
|---|---|
| `ghostlist.count` | `ghostlist.count:1` |
| `ghostlist.current.汎用プロパティ名` | `ghostlist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `ghostlist.index(ID).汎用プロパティ名` | `ghostlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `ghostlist(ゴースト名/本体側名/パス).汎用プロパティ名` | `ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `ghostlist(ゴースト名/本体側名/パス).icon` | `ghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.icon:1` |
| `activeghostlist.current.汎用プロパティ名` | `activeghostlist.current._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `activeghostlist.index(ID).汎用プロパティ名` | `activeghostlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `activeghostlist.index(ID).ext.拡張プロパティ名` | `activeghostlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `activeghostlist(ゴースト名/本体側名/パス).汎用プロパティ名` | `activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_5:1` |
| `activeghostlist(ゴースト名/本体側名/パス).ext.拡張プロパティ名` | `activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30:1` |

#### L3 — `balloonlist` ＋ `headlinelist` ＋ `pluginlist`（brief 11・実測 11・内訳は不一致）

合計が一致しているのは偶然で、**3 つのずれが打ち消し合っている**。
`headlinelist` が 1 件少なく（`.count` の数え落とし）、`pluginlist` も 1 件少なく（同じく `.count`）、
そのぶん `.ext` の 2 件を別に足して二重に数えている。9 ＋ 2 ＝ 11 と 3 ＋ 3 ＋ 5 ＝ 11 が
たまたま同じ数になっただけである。

| 見出し | id |
|---|---|
| `balloonlist.count` | `balloonlist.count:1` |
| `balloonlist.index(ID).汎用プロパティ名` | `balloonlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `balloonlist(バルーン名/パス).汎用プロパティ名` | `balloonlist_28_30d0_30eb_30fc_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `headlinelist.count` | `headlinelist.count:1` |
| `headlinelist.index(ID).汎用プロパティ名` | `headlinelist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `headlinelist(ヘッドライン名/パス).汎用プロパティ名` | `headlinelist_28_30d8_30c3_30c9_30e9_30a4_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `pluginlist.count` | `pluginlist.count:1` |
| `pluginlist.index(ID).汎用プロパティ名` | `pluginlist.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `pluginlist.index(ID).ext.拡張プロパティ名` | `pluginlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `pluginlist(プラグイン名/パス/ID).汎用プロパティ名` | `pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `pluginlist(プラグイン名/パス/ID).ext.拡張プロパティ名` | `pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1` |

#### L4 — `history`（brief 8・実測 12）

brief は 4 つの枝（バルーン・ゴースト・ヘッドライン・プラグイン）× 2 つの指し方（名前・番号）で 8 と数えている。
実際には枝ごとに `.count` の葉がもう 1 つあり、4 × 3 ＝ 12 になる。

| 見出し | id |
|---|---|
| `history.balloon.count` | `history.balloon.count:1` |
| `history.balloon.index(ID).汎用プロパティ名` | `history.balloon.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.balloon(バルーン名/パス).汎用プロパティ名` | `history.balloon_28_30d0_30eb_30fc_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.ghost.count` | `history.ghost.count:1` |
| `history.ghost.index(ID).汎用プロパティ名` | `history.ghost.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.ghost(ゴースト名/パス).汎用プロパティ名` | `history.ghost_28_30b4_30fc_30b9_30c8_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.headline.count` | `history.headline.count:1` |
| `history.headline.index(ID).汎用プロパティ名` | `history.headline.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.headline(ヘッドライン名/パス).汎用プロパティ名` | `history.headline_28_30d8_30c3_30c9_30e9_30a4_30f3_540d_2f_30d1_30b9_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.plugin.count` | `history.plugin.count:1` |
| `history.plugin.index(ID).汎用プロパティ名` | `history.plugin.index_28ID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `history.plugin(プラグイン名/パス/ID).汎用プロパティ名` | `history.plugin_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |

#### L6 — 汎用プロパティ名の葉（brief 17・実測 13 ＋ 4）

brief の「17」は areka 側の語彙表（`crates/areka-sylphya/src/vocab/dotted.rs:37` の 17 名）の件数である。
正典の側では、このうち 13 件が「どのルート枝の下でも同じように使える葉」として置かれ、
残る 4 件（`menu` の群）は使える場所が限られた別の見出しになっている。
合計は 17 で一致し、名前も 1 対 1 で対応するが、**正典側の性格が 2 つに分かれる**点が違う。
語彙表の側を直すべきかどうかは ⑸ 節で扱う。

正典で「どのルート枝の下でも使える葉」として置かれている 13 件:

| 見出し | id |
|---|---|
| `craftmanurl` | `craftmanurl:1` |
| `craftmanw` | `craftmanw:1` |
| `homeurl` | `homeurl:1` |
| `index` | `index:1` |
| `keroname` | `keroname:1` |
| `name` | `name:1` |
| `path` | `path:1` |
| `sakuraname` | `sakuraname:1` |
| `shiori.変数名` | `shiori._5909_6570_540d:1` |
| `thumbnail` | `thumbnail:1` |
| `update_result` | `update_result:1` |
| `update_time` | `update_time:1` |
| `username` | `username:1` |

`menu` の群 4 件（正典が使える場所を絞っている・いずれも書き込みが有効）:

| 見出し | id |
|---|---|
| `menu` | `menu:1` |
| `sakura.bind.menu` | `sakura.bind.menu:1` |
| `kero.bind.menu` | `kero.bind.menu:1` |
| `char*.bind.menu` | `char_2a.bind.menu:1` |

`name` と `path` は見出しが 2 つずつあり、**見出しの名前だけでは id が決まらない。**
上の表に載っているのは汎用の葉の側（`:1`）で、もう一方（`:2`）は 2.8.72 で加わったサウンドの要素葉である。
4 行の書き分けの根拠は ⑹ 節に置く。

#### T2 — `currentghost.balloon.mousecursor` の 4 件

tree の brief はこの 4 件を括弧書きで名前だけ挙げ、**どの件数の束にも足していない。**
台帳では tree が唯一の主張者なので担当を tree に置いたが、brief の側は件数を直す必要がある。

| 見出し | id |
|---|---|
| `currentghost.balloon.mousecursor` | `currentghost.balloon.mousecursor:1` |
| `currentghost.balloon.mousecursor.arrow` | `currentghost.balloon.mousecursor.arrow:1` |
| `currentghost.balloon.mousecursor.text` | `currentghost.balloon.mousecursor.text:1` |
| `currentghost.balloon.mousecursor.wait` | `currentghost.balloon.mousecursor.wait:1` |

### 対応する id が無い記述

次の 3 つは brief に書かれているが、このページのどの id にも対応しない。
**近そうな id へ憶測で結び付けず、表記の揺れとしてこのまま残す。**

| brief の場所 | 記述 | なぜ対応が付かないか |
|---|---|---|
| `areka-P0-property-query-channels` brief:22 | 経路 6（スクリプトを通さない同期読み・里々の関数など） | このページに相当する見出しが無い。brief 自身も「輸送路がスナップショットに未記載」と書いており、実測でも 0 件 |
| `areka-P0-property-query-channels` brief:42-43 | 経路 1〜4 の実装と、イベント発生の届け先の型の新設 | 相手はさくらスクリプトのページの項目と areka のコードで、このページの項目ではない |
| `areka-P0-currentghost-property-tree` brief:28 | `vertical` などを汎用プロパティ名の一覧に登録するかの裁定 | 正典に単独の見出し `vertical` は無い。`vertical` を含む見出しはこのページに `currentghost.balloon.scope(ID).vertical` の 1 件だけで、これは T1 に既に入っている。汎用名への登録は areka 側の語彙表の話であって正典の項目ではない |

### 書き込みが有効な項目の突き合わせ

台帳の状態は「読んだときに値が返るか」だけで決めている。書き込みが効くかどうかは別の軸なので、
ここで正典と areka の両方向から突き合わせる。

**正典が書き込みを認めているのは 26 件。** 本文の読み方は 3 通りあり、印だけを数えると 14 件で
半分ほどしか拾えない。

| 読み方 | 件数 |
|---|---:|
| ⒜ 本文に `[SET有効]` の印がある | 14 |
| ⒝ 印は無いが本文が設定できると述べている | 3 |
| ⒞ 族の頭が「これ以降この名前で始まる項目に共通」と断った内容が及ぶ | 9 |
| **合計** | **26** |

⒜ の 14 件:

`char_2a.bind.menu:1`／`currentghost.mousecursor:1`／
`currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.path:1`／
`currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.path:1`／
`currentghost.seriko.sticky-window:1`／
`currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.text:1`／
`currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.text:1`／
`currentghost.seriko.zorder:1`／`kero.bind.menu:1`／`menu:1`／`pause:1`／`playing:1`／`position:1`／
`sakura.bind.menu:1`

⒝ の 3 件（本文がサーフェス番号・アニメーション番号・既定のサーフェスについて、設定もできると述べている）:

`currentghost.scope_28ID_29.animation.num:1`／`currentghost.scope_28ID_29.seriko.defaultsurface:1`／
`currentghost.scope_28ID_29.surface.num:1`

⒞ の 9 件（族の頭は `currentghost.mousecursor:1` の 1 つだけ。頭は⒜で数えているので増える分は 9）:

`currentghost.balloon.mousecursor:1`／`currentghost.balloon.mousecursor.arrow:1`／
`currentghost.balloon.mousecursor.text:1`／`currentghost.balloon.mousecursor.wait:1`／
`currentghost.mousecursor.arrow:1`／`currentghost.mousecursor.grip:1`／`currentghost.mousecursor.hand:1`／
`currentghost.mousecursor.text:1`／`currentghost.mousecursor.wait:1`

**areka 側の一覧は 21 名。** 出どころは `crates/areka-sylphya/src/vocab/dotted.rs:72` の `SET_EFFECTIVE` で、
件数を 21 に固定しているテストが同 `:191` にある。
21 名はどれも相対的な短い名前なので、正典の完全な名前へ広げると **25 の id** に当たる。
うち 21 の id は正典でも書き込みが有効で、残る 4 つは有効としていない。

### 突き合わせの 4 区分

#### ⑴ areka の 21 名のうち、正典の id に当たらないもの — **0 件**

該当なし。21 名はすべて 1 つ以上の id に当たる。調べる対象が無くて 0 件になったのではなく、
21 名すべてを正典の完全な名前へ広げ、1 名ずつ当たる id を確かめたうえでの 0 件である。

#### ⑵ 正典は書き込みを認めているのに、areka の 21 名に入っていないもの — **5 件**

| 見出し | id | 正典で入った版 |
|---|---|---|
| `currentghost.seriko.zorder` | `currentghost.seriko.zorder:1` | 2.8.78 |
| `currentghost.seriko.sticky-window` | `currentghost.seriko.sticky-window:1` | 2.8.78 |
| `position` | `position:1` | 2.8.72 |
| `playing` | `playing:1` | 2.8.72 |
| `pause` | `pause:1` | 2.8.72 |

この 5 件は `areka-P0-property-query-channels` brief:29・brief:44 が「21 → 26」として挙げている中身と
**id 単位でぴったり一致する。**

#### ⑶ areka の 21 名にあるが、正典は書き込みを認めていないもの — **2 名・4 id**

| areka の名前 | 当たる id |
|---|---|
| `seriko.cursor.name` | `currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.name:1` |
| | `currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1` |
| `seriko.tooltip.name` | `currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.name:1` |
| | `currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1` |

4 件とも印が無く、族の頭からの継承も届かない。正典の本文は当たり判定の呼び名を返すことしか述べておらず、
書き込みについては何も言っていない。**areka が正典より 1 歩先に登録している状態**である。

#### ⑷ 1 つの名前が 2 つの id に当たるもの — **4 名・8 id**

正典は「当たり判定の名前で指す形」と「並びの番号で指す形」を別々の見出しにしているため、
areka の短い名前 1 つがどちらにも当たる。

| areka の名前 | 当たる id |
|---|---|
| `seriko.cursor.path` | `currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.path:1` |
| | `currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.path:1` |
| `seriko.cursor.name` | `currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1` |
| | `currentghost.seriko.cursor.scope_28ID_29.mouse_3f_3f_3f_3flist.index_28ID2_29.name:1` |
| `seriko.tooltip.text` | `currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.text:1` |
| | `currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.text:1` |
| `seriko.tooltip.name` | `currentghost.seriko.tooltip.scope_28ID_29.textlist_28_5f53_305f_308a_5224_5b9a_540d_29.name:1` |
| | `currentghost.seriko.tooltip.scope_28ID_29.textlist.index_28ID2_29.name:1` |

#### 検算

- ⒜ 14 ＋ ⒝ 3 ＋ ⒞ 9 ＝ **26**（正典が書き込みを認める総数）。
- areka の 21 名 → **25 id**。うち正典でも有効なのが **21**、有効としないのが **4**（区分 ⑶）。21 ＋ 4 ＝ 25。
- 26 −（21 名が押さえている 21）＝ **5**（区分 ⑵）。

### 26 という数には未決の別の読みが 1 つある

**これは案であって決定ではない。** 裁定の一覧は ⑷ 節にまとめるが、数の根拠に直に関わるので要点だけここに置く。

`.ext.拡張プロパティ名` の 4 件（L8 の 4 件）は、印が無いのに、
本文がイベントを起こして値を取り出すことと値を入れることの両方を述べている。
文字どおりに読めば読み方 ⒝ に当たりうる。本調査は **含めない側を既定として採り、26 のままにした。**
理由は 2 つ——設定される先はゴーストやプラグインの内部であってベースウェア自身の状態ではないこと、
areka 側の受け皿も書き込み有効一覧ではなく `property.get`／`property.set` というイベント名の予約であること。

含める読みを採った場合に動くもの:

| 動くもの | 既定（26） | 広い読み（30） |
|---|---|---|
| 正典が書き込みを認める総数 | 26 | 30 |
| 読み方 ⒝ の件数 | 3 | 7 |
| 区分 ⑵ の件数 | 5 | 9 |
| この 4 件の優先度 | `C90` | `C10`（書き込みが受理されて効かない側に入るため） |
| 台帳の関連 | 動かない | 動かない（4 件は既に `property.get`／`property.set` を指している） |
| channels brief の「21 → 26」との一致 | 一致する | 崩れる |

対象の 4 id: `activeghostlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1`／
`activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30:1`／
`pluginlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1`／
`pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1`

### バルーンの被覆について 1 つ注意

`currentghost.balloon.scope(ID).scaling` の関連は、シェルのサーフェス定義の `scaling` を指している
（`ukadoc:descript_shell_surfaces:scaling:1`）。ただし**その項目の本文はキャラクターの窓とサーフェスの話**で、
バルーン専用の相手はカタログに存在しない。指せる相手が 1 つしか無いのでそれを選んだだけであって、
「バルーンの設定元まで押さえられている」と読まないでほしい。

## ⑵ 同じ値へ到達する名前の一覧

### 群は 0 件

このページには、同じ値へ 2 通り以上の名前で行き着く組み合わせが **1 つも無い**。
どの項目も、ほかの項目の古い呼び名ではない。

**これは「調べたが無かった」であって「調べていない」ではない。** 確かめ方は 2 つある。

1. **見出しの頭を数えた** — 188 件の見出しのうち `balloon.` で始まるものは **0 件**である。
   バルーンに関わる見出しの根は `balloonlist` 3 件・`currentghost.balloon` 23 件・`history.balloon` 3 件の
   3 つ（合わせて 29 件）で、どれも `balloon.` から始まる形ではない。
2. **188 件すべての本文を言い回しで走査した** — 古い呼び名・廃止・非推奨・改名・統合・置き換え・移行などを
   表す 27 語で全項目の本文を通した。当たったのは 11 件で、中身はすべて
   「設定するとこのタグを打ったのと同じ働きになる」「書き込みは追加ではなく丸ごとの入れ替えになる」
   といった**動きの説明**だった。**ある名前を別の名前へ読み替えよ、と述べたものは 1 件も無い。**

見出しが重複する `name` と `path`（各 2 件）も、同じ値への別の名前ではない。
汎用の葉の側とサウンドの要素葉の側で**指す値そのものが違う**別々の項目である。書き分けの根拠は ⑹ 節に置く。

### brief が例に挙げた古い `balloon.*` はカタログに無い

`areka-P0-currentghost-property-tree` の brief:23 は、自分の持ち分を
「`currentghost.balloon.scope(ID).*` ×17 ＋ `balloon.汎用` ＋ `balloon.count` ＝ 19 項目」と書いている。
このうち後ろの 2 つ、`balloon.汎用` と `balloon.count` という名前は**カタログに存在しない**。
実在するのは次の 2 件で、どちらも頭に `currentghost.` が付く。

| 見出し | id |
|---|---|
| `currentghost.balloon.汎用プロパティ名` | `currentghost.balloon._6c4e_7528_30d7_30ed_30d1_30c6_30a3_540d:1` |
| `currentghost.balloon.count` | `currentghost.balloon.count:1` |

つまり brief は同じ 2 件を短く書いただけで、**古い名前と新しい名前が両方あるわけではない。**
件数の突き合わせは既に取れている（⑴ 節の T1 の行で 19 ＝ 17 ＋ 1 ＋ 1 が一致する）。

**brief の担当者への申し送り**: brief:23 のこの 2 つを完全な名前へ書き直してほしい。
今の書き方だと「`balloon.*` という古い一族が別にあって、`currentghost.balloon.*` はその新しい形だ」と
読めてしまう。そう読んだ人はカタログに無い名前を探すことになる。

### 台帳と、上流の道具の検査への帰結

- 台帳の 188 件に「この名前の正しい行き先」を書いた欄（`alias_of`）は **1 つも無い**。
  「新しい名前が古い名前を置き換えた」ことを書く欄（`supersedes`）も 0 件、状態が「別名」の行も 0 件である。
- したがって上流の道具が持つ「別名の行き先がまた別名になっていないか」の検査は、
  **調べる対象が 0 件のまま緑になる。** 働いて緑になったのではない。
- 上流の空振り検査の国勢調査も、別名の行を数える面を「対象 0 件」と主張したままでよい
  （`crates/ukadoc-survey/tests/consistency/checks.rs:239`）。4 つのドメインの台帳を通しても別名の行は 0 件である。
- この文書の末尾の検証の記録では、この検査を「対象 0 件で緑」と書き分ける。

## ⑶ 持ち主のいない項目・裁定待ちの項目の一覧

### 一覧は 2 行、台帳で担当欄が空の項目も 2 件

台帳の担当欄（`owner`）が空文字になっている項目は **2 件**である。この節の一覧も **2 行**で、両者は一致する。

| 見出し | id | 優先度 | 空文字の理由 |
|---|---|---|---|
| `currentghost.seriko.zorder` | `currentghost.seriko.zorder:1` | `C10` | 裁定待ち |
| `currentghost.seriko.sticky-window` | `currentghost.seriko.sticky-window:1` | `C10` | 裁定待ち |

### 引受先がまったくいない項目は 0 件

担当欄が空になる理由は 2 通りある——「誰も名乗っていない」か「2 本以上が争っていて決まらない」かである。
**このドメインで起きているのは後者だけで、前者は 1 件も無い。** 188 件のうち 186 件には担当欄に
spec 名が入っている——4 本の brief のいずれかが自分の範囲だと書いているものが 184 件
（`areka-P0-property-catalog-lists` 120・`areka-P0-currentghost-property-tree` 64）、
既に実装が済んでいて完了済み spec の名前が入っているものが 2 件（`areka-P0-sylphya`）。
残る 2 件が上の裁定待ちである。

したがって「まだ起票されていない spec を引受先として提案する」形——備考に `候補: ...` の 1 行を足す書き方——は
**このドメインでは 1 件も使っていない。** 台帳を `候補:` で検索しても 0 件である。
ここでも「調べたが対象が無かった」であって「書き忘れた」のではない。

### 2 件が裁定待ちである理由（台帳が書いていること）

台帳はこの 2 件の備考に、争っている spec の名前と争点を 1 行ずつ持っている。写すと次のとおり。

- **`currentghost.seriko.zorder`** — `areka-P0-zorder-property` が値の導出を単独で持つと主張し、
  `areka-P0-currentghost-property-tree` は `seriko.*` 14 項の一括所有に含め、
  `areka-P0-property-query-channels` は areka 側の書き込み有効一覧の追随（21 → 26）に含める。
  争点は値の導出の担い手と、語彙表の 1 行の持ち主。主張しているのは **3 本**。
- **`currentghost.seriko.sticky-window`** — `areka-P0-currentghost-property-tree` が `seriko.*` 14 項の
  一括所有に含め、`areka-P0-property-query-channels` は同じ書き込み有効一覧の追随に含める。
  争点は同じ 2 つ。主張しているのは **2 本**（`areka-P0-zorder-property` はこちらを主張していない）。

### 優先度がどちらも `C10` である理由

優先度の規則を上から順に当てる。まず「実装済み」の行は当たらない（2 件とも読んでも値は返らない）。
次の行が「正典が書き込みを認めていて、しかも未実装」で、**この行が両方に当たるのでそこで `C10` に決まる。**
2 件とも本文に書き込みが有効である印が付いており（⑴ 節の⒜ 14 件に入っている）、
areka 側は書き込みを受け取って警告を出し、値は変えない。

`currentghost.seriko.zorder` は状態が縮退（`doc/COMPAT_ARCHITECTURE.md:207` に登記済み）なので
「縮退なら `C30`」の行にも当たる形をしているが、**先に当たる行が上にあるので `C30` へは落ちない。**
`currentghost.seriko.sticky-window` は状態が語彙のみなので、そもそもその行には当たらない。

この 2 件が前に出る理由は、読んで値が返らないことよりも**書き込みの側**にある。
作者が値を書き込むと呼び出しは成功として返るのに、実際には何も起きない。
失敗したことが作者に見えないぶん、値が返らないだけの項目より始末が悪い。
書き込みが有効な 26 件が一様に `C10` なのはこのためで、この 2 件も同じ扱いになる。

### 裁定はこの節では行わない

この節が受け持つのは「担当が決まっていない項目を 1 件残らず並べること」だけである。
どちらに決めるべきかの案と、その案を採ったとき／採らなかったときに 3 本の spec の作業がどう変わるかは、
次の ⑷ 節にある。

## ⑷ 二重所有の裁定案

**この節に書くことはすべて案であって決定ではない。** 決めるのは開発者と、4 ドメインをまとめる担当
（`areka-P0-ukadoc-coverage-roadmap`）である。本調査がしたのは、誰が何を主張しているかを並べ、
推す案を 1 つ挙げ、その案を採った場合と採らなかった場合に何が動くかを測ることまでで、
どれも決めていない。

### この節が抱える未決の問いは 3 つ

| # | 未決の問い | 何と何が食い違っているか | 決まると動くもの | 書いてある場所 |
|---|---|---|---|---|
| ⓐ | `currentghost.seriko.zorder` と `currentghost.seriko.sticky-window` を誰が持つか | spec 3 本の主張（`sticky-window` は 2 本） | 台帳の担当欄 2 件と、3 本の brief に書かれた範囲 | この節（すぐ下） |
| ⓑ | テーマ「記憶」を付けるか | 本調査の判断とテーマ定義書の記述 | 対象の項目の優先度とテーマ欄。何件に付けるかも未決（2 件か、枝ごとで最大 36 件か） | この節の後半 |
| ⓒ | `.ext.拡張プロパティ名` の 4 件を書き込みが有効な側に数えるか | 正典の本文の 2 通りの読み方 | 「書き込みが有効なのは 26 件」という数そのもの | ⑴ 節の末尾 |

ⓒ だけをこの節に持ってこなかったのは、あの数が ⑴ 節の突き合わせの土台そのものだからである。
数の根拠は数の隣に置いた（⑴ 節「26 という数には未決の別の読みが 1 つある」）。
3 つとも扱いは同じで、どれも決めずに渡す。

### ⓐ `seriko.zorder` と `seriko.sticky-window` を誰が持つか

#### 3 本が何を主張しているか

| spec | brief の場所 | 主張していること |
|---|---|---|
| `areka-P0-zorder-property` | brief:9・brief:13-17 | `currentghost.seriko.zorder` の読み書きの実導出を単独で持つ。読み書きの完全な書式（群の中はカンマ区切り・群と群の間はセミコロン区切り・名前で書く形と番号で書く形は混ぜられない・書き込みは今の設定の丸ごとの入れ替え・空文字で全解除・要素が 2 個未満の群は無視）は、この brief が正本である |
| 〃 | brief:22 | areka 側の書き込み有効一覧（`crates/areka-sylphya/src/vocab/dotted.rs` の 21 名）に `seriko.zorder` を**入れない**。動かない名前を先に登録することはしない |
| `areka-P0-currentghost-property-tree` | brief:26 | `currentghost.seriko.*` の 14 項目をまとめて持つ。その列挙の中に `zorder` と `sticky-window` が名指しで入っている |
| 〃 | brief:53 | `areka-P0-zorder-property` の範囲は自分の範囲の一部だと認めたうえで、「吸収する」か「切り出す」かを合流の会で決めるよう自分から求めている |
| `areka-P0-property-query-channels` | brief:29・brief:44 | areka 側の書き込み有効一覧を 21 から 26 へ追随させる仕事を、書き込み経路の持ち主として持つ。増える 5 件の中に `seriko.zorder` と `seriko.sticky-window` が入っている |

食い違いは 2 つある。

1. **値を導き出す仕事を誰がやるか** — `areka-P0-zorder-property` と `areka-P0-currentghost-property-tree` が
   どちらも自分だと書いている。
2. **areka 側の語彙表の 1 行を誰が触るか** — `areka-P0-zorder-property` は「入れない・この brief が正本」と書き、
   `areka-P0-property-query-channels` は「21 から 26 へ増やす中に入っている」と書いている。
   同じ 1 行について正反対のことが書いてある。

#### 推す案は 1 つ（案 甲）

- 値を導き出す仕事は `areka-P0-zorder-property` が単独で持つ。
- `areka-P0-currentghost-property-tree` は `seriko.*` の一括所有から `zorder` を外す（14 項目 → 13 項目）。
- areka 側の語彙表の 1 行は `areka-P0-property-query-channels` が持つ。`areka-P0-zorder-property` は語彙表に触れない。

**この案を選んだ理由**: この 3 つの組み合わせは、既に `.kiro/steering/roadmap.md:111` に推奨として
書き留められているものと同じである。そこには「推奨＝切り出し: 値の導出は `zorder-property` 単独・
tree は `seriko.*` から `zorder` を除外・台帳行 1 本は channels⑶ が持つ」と書かれている。
本調査が別の案を新しく立てて会に軸をもう 1 本持ち込むより、既にある推奨をそのまま案として出し、
採る／採らないの影響だけを測って渡すほうが、裁定を 1 度で終えられる。

#### 案 甲を採ったとき／採らなかったときに動くもの

| 誰の | 採ったとき | 採らなかったとき |
|---|---|---|
| `areka-P0-zorder-property` | 範囲は今のまま（値の導出だけ）。ただし brief:22 の「語彙表に入れない・この brief が語彙の正本」は書き直しが要る——語彙表の 1 行は `areka-P0-property-query-channels` が入れることになるため。⑸ 節の是正候補に載せる | 主張が宙に浮いたままになる。完了済み spec `areka-P0-scope-zorder-pinning` は、先送りにした語彙の追跡先をこの spec 単独と記録している（`.kiro/specs/completed/areka-P0-scope-zorder-pinning/requirements.md:230-231`）ので、その追跡先を誰も受け取らない状態が続く |
| `areka-P0-currentghost-property-tree` | `seriko.*` の一括所有が 14 項目から 13 項目へ減り、brief:26 の列挙から `zorder` を外すことになる。`sticky-window` は範囲に残る | brief:53 が自分から求めている合流の裁定が未了のまま実装に入る。`zorder` の値の導出を 2 本が並行して書く危険が残る |
| `areka-P0-property-query-channels` | 範囲は今のまま。21 → 26 の追随に `seriko.zorder`・`seriko.sticky-window` を含めてよいことが確定する | 「21 → 26」で増える 5 件のうち 2 件が誰の持ち物か決まらないまま、語彙表を触ることになる |
| この台帳 | 担当欄が空文字の 2 件が埋まる——`zorder` は `areka-P0-zorder-property`、`sticky-window` は `areka-P0-currentghost-property-tree`。備考の「裁定待ち」の行は要らなくなる | 空文字の 2 件がそのまま残り、4 ドメインをまとめる会で同じ議論をもう一度開くことになる |

#### 退けた代案

- **案 乙（`tree` が `seriko.*` を一括で持ち、`zorder-property` を畳む）** — `areka-P0-zorder-property` の brief にだけ書かれている完全な書式（brief:13-17）を丸ごと移し替える手間が要り、`.kiro/steering/roadmap.md:111` の推奨とも逆向きになる。完了済み spec が記録した追跡先も書き換えることになる。
- **案 丙（決めずに保留する）** — これは案ではなく、何も決めなかったときの今の状態そのものである（担当欄は空文字のまま）。比較の材料として並べるだけで、選ぶ対象にはならない。

#### `currentghost.seriko.sticky-window` も同じ案の対象

`currentghost.seriko.sticky-window` にも同じ形の食い違いがあるので、別扱いにせず同じ案で片付ける。
ただし**争っているのは 2 本だけ**である——`areka-P0-zorder-property` はこの項目を主張していない
（同 brief が扱うのは `zorder` の 1 項目だけ）。残る 2 本の争いは上の ⑵（語彙表の 1 行）だけで、
⑴（値を導き出す仕事）のほうは初めから争いになっていない。

案 甲を採ると、この項目はこう片付く。

- 値を導き出す仕事は `areka-P0-currentghost-property-tree` が持つ（`seriko.*` の一括所有に残る。外れるのは `zorder` だけ）。
- areka 側の語彙表の 1 行は `areka-P0-property-query-channels` が持つ（`zorder` と同じ扱い）。
- 台帳の担当欄には `areka-P0-currentghost-property-tree` が入る。

つまり案 甲は、`zorder` については「切り出す」、`sticky-window` については「切り出さない」と言っている。
2 件で答えが分かれるのは、主張している spec の数が違うからである。

#### 担当欄は裁定が下りるまで空のまま

2 件とも台帳の担当欄は空文字にしてあり、裁定が下りた時点で実名に変わる。
2 件の一覧（優先度と、空文字になっている理由つき）は ⑶ 節にあるので、ここでは繰り返さない。

### ⓑ テーマ「記憶」を付けなかったことと、その見え方

テーマの定義書 `doc/ukadoc-coverage/values.md` の「記憶」の節は、代表となる項目として
このドメインの id を 2 件挙げている——`rateofuselist(名前).boottime`（`rateofuselist_28_540d_524d_29.boottime:1`）と
`history.ghost.count`（`history.ghost.count:1`）。
しかし本調査はこの 2 件にテーマを付けなかった。付けたのは着せ替えに直接関わる 3 件だけで、
使った語も「装い」1 つである。結果として `report/property.md` のテーマ別の表は「記憶 0 件」と出る。

**両方を読む人には食い違って見えるので、先に断っておく。** テーマを付けるかどうかは
段階の割り振りと直結し、その最終決定は 4 ドメインをまとめる担当の仕事である。
本調査の側で先に付けてしまうと、どの範囲まで（2 件だけか、`rateofuselist` 24 件と `history` 12 件の枝ごとで
最大 36 件か）が決まっていないまま数字が固まる。決めずに記録して渡すほうを選んだ。

付ける裁定が下りた場合に動くもの——対象の id の優先度が `C90` から `C20` へ上がり、
備考に「無いと利用者が何を失うか」の行が要り、
本調査が書いた「テーマとして使ったのは『装い』1 語だけ」という記述を書き直すことになる。
何件に付けるか（2 件だけか、枝ごとで最大 36 件か）はこの裁定と一緒に決める必要がある。
