# 伺からしさのテーマ

この文書は、ukadoc の項目に付ける「伺からしさ」のテーマ 8 つを定義する。台帳
（`doc/ukadoc-coverage/ledger/<ドメイン>.toml`）の `values` 欄に書けるのは、ここに見出しとして
並ぶ 8 つの名前だけで、それ以外の綴りは検査が赤にする。

読み手は調査 spec 4 本（shiori・assets・sakura-script・property）の担当者で、担当分の項目を
台帳に書くときに、その項目へどのテーマを付けるかをここで決める。テーマは、どの項目から先に
手を着けるかを決める根拠の 2 番目にあたる（根拠の並びそのものは
`doc/ukadoc-coverage/README.md` が正本）。

8 つの名前と、その並び順は凍結されている。増やす・減らす・言い換えるには要件の改訂が要る
（`.kiro/specs/areka-P0-ukadoc-survey-toolkit/requirements.md` 要件 4.4・2.6）。同じ 8 つは
`crates/ukadoc-survey/src/model.rs` の `THEMES` にも同じ順で置いてあり、両者が食い違うと
標準のテスト実行が赤になる。

この文書で `##` の見出しになっているのはテーマ名だけで、他の見出しは置かない。テーマを
書き換えるときは、`##` の行がテーマ名そのものであることを崩さないこと。

**付与規則（1 つだけ）**

> 「この項目が無いと利用者はゴーストの何を失うか」に答えられるテーマだけを付け、答えられ
> なければ何も付けない。

規則はこれだけで、他に条件は無い。使い方の補足を 3 つ添える。

- 答えが 2 つ以上のテーマにまたがるなら、そのすべてを付けてよい。並べる順はこの文書の
  見出しの順にする。
- 迷ったら付けない。空欄は「まだ答えていない」ことをそのまま表すので、無理に埋めるより
  正しい。
- 「あると便利だから」「仕様書に載っているから」は理由にならない。利用者から見て何が
  失われるかを一言で言えるかどうかだけで決める。

以下の各テーマは、1 行の定義・無いと失うもの・代表となる項目の 3 つを持つ。代表項目の
見出し文と URL は `doc/ukadoc-coverage/catalog.toml` の該当行から取ってある。

## 気配

- **定義**: 話しかけられていない間も、ゴーストがそこに居て生きて見えること。
- **無いと失うもの**: 画面のゴーストが止め絵になる。瞬きも身じろぎも独り言も起きず、置き物と区別が付かなくなる。利用者は「居る」と感じられなくなり、立ち上げたまま忘れる。
- **代表となる項目**:
  - `animation*.interval,インターバル` — 瞬きや身じろぎを、どの周期で始めるかの定義。
    - id: `ukadoc:descript_shell_surfaces:animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1`
    - <https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation_2a.interval_2c_30a4_30f3_30bf_30fc_30d0_30eb:1>
  - `OnSecondChange` — 1 秒ごとに届く。何も操作されずに放置されている時間も一緒に渡る。
    - id: `ukadoc:list_shiori_event:OnSecondChange:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnSecondChange:1>
  - `OnMinuteChange` — 1 分ごとに届く。ひとりごとを始める合図に使われる。
    - id: `ukadoc:list_shiori_event:OnMinuteChange:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMinuteChange:1>

## 触れ合い

- **定義**: 利用者が撫でる・つつくといった、ゴーストに直接触る操作に応えること。
- **無いと失うもの**: 撫でても何も返らない。頭と胸の区別も付かないので、どこを触っても同じか、まったく反応しない。手を出す気がなくなる。
- **代表となる項目**:
  - `OnMouseClick` — 触られたときに届く。どの当たり判定を触ったかが渡る。
    - id: `ukadoc:list_shiori_event:OnMouseClick:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseClick:1>
  - `OnMouseDoubleClick` — 二度続けて触られたときに届く。
    - id: `ukadoc:list_shiori_event:OnMouseDoubleClick:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnMouseDoubleClick:1>
  - `collision*,始点X,始点Y,終点X,終点Y,ID` — 絵のどこが頭でどこが胸かを決める、触れる場所の定義。
    - id: `ukadoc:descript_shell_surfaces:collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1`
    - <https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#collision_2a_2c_59cb_70b9X_2c_59cb_70b9Y_2c_7d42_70b9X_2c_7d42_70b9Y_2cID:1>

## 掛け合い

- **定義**: 本体側と相方側が交互に喋り、間を取ってやり取りして見せること。
- **無いと失うもの**: 二人の会話が一人の独白になる。誰の台詞なのか分からず、間も無くなって一息に流れる。掛け合いを見せる筋書きが成立しない。
- **代表となる項目**:
  - `\0もしくは\h` — 以降の台詞を本体側のものにする。
    - id: `ukadoc:list_sakura_script:_5c0_3082_3057_304f_306f_5ch:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c0_3082_3057_304f_306f_5ch:1>
  - `\1もしくは\u` — 以降の台詞を相方側のものにする。
    - id: `ukadoc:list_sakura_script:_5c1_3082_3057_304f_306f_5cu:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c1_3082_3057_304f_306f_5cu:1>
  - `\w時間` — 台詞の途中で間を取る。
    - id: `ukadoc:list_sakura_script:_5cw_6642_9593:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5cw_6642_9593:1>

## 装い

- **定義**: ゴーストの見た目（シェル・着せ替え・バルーン）を選び替えられること。
- **無いと失うもの**: 作者が何着も用意しても、利用者は最初の一着しか見られない。季節物も帽子も出せず、吹き出しの見た目も選べない。
- **代表となる項目**:
  - `\![bind,カテゴリ名,パーツ名,数値]` — 着せ替えの部品を着せる・脱がせる。
    - id: `ukadoc:list_sakura_script:_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bbind_2c_30ab_30c6_30b4_30ea_540d_2c_30d1_30fc_30c4_540d_2c_6570_5024_5d:1>
  - `\![change,shell,シェル名(,--option=raise-event)]` — 絵の一式そのものを別のものへ替える。
    - id: `ukadoc:list_sakura_script:_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1>
  - `\![change,balloon,バルーン名]` — 台詞を載せる吹き出しを別のものへ替える。
    - id: `ukadoc:list_sakura_script:_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1>

## 記憶

- **定義**: 前に会ったときのことを本体側が覚えていて、ゴーストに渡せること。
- **無いと失うもの**: 何度呼んでも毎回はじめまして。前に消したことも、何回起動したかも、ゴーストは知らないまま話す。付き合いが積み上がらない。
- **代表となる項目**:
  - `OnFirstBoot` — 初めて起動したときに届く。前に消された回数も一緒に渡る。
    - id: `ukadoc:list_shiori_event:OnFirstBoot:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFirstBoot:1>
  - `rateofuselist(名前).boottime` — そのゴーストを何回起動したか。
    - id: `ukadoc:list_propertysystem:rateofuselist_28_540d_524d_29.boottime:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#rateofuselist_28_540d_524d_29.boottime:1>
  - `history.ghost.count` — 最近呼んだゴーストがいくつあるか。
    - id: `ukadoc:list_propertysystem:history.ghost.count:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_propertysystem.html#history.ghost.count:1>

## 交わり

- **定義**: 同居している他のゴーストや、利用者からの差し込みと言葉を交わせること。
- **無いと失うもの**: 二人以上を並べても互いに黙ったまますれ違う。話しかけ箱から声を掛けても返らない。一人ずつ立ち上げるのと変わらなくなる。
- **代表となる項目**:
  - `OnCommunicate` — 他のゴーストや話しかけ箱から台詞を渡されたときに届く。
    - id: `ukadoc:list_shiori_event:OnCommunicate:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnCommunicate:1>
  - `\![raiseother,ゴースト名,イベント名,r0,r1,r2...]` — 名指しした他のゴーストに出来事を伝える。
    - id: `ukadoc:list_sakura_script:_5c_21_5braiseother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5braiseother_2c_30b4_30fc_30b9_30c8_540d_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1>
  - `OnOtherGhostBooted` — 別のゴーストが立ち上がったときに届く。
    - id: `ukadoc:list_shiori_event:OnOtherGhostBooted:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnOtherGhostBooted:1>

## 気配り

- **定義**: 利用者や機械の様子を見て、邪魔にならないように振る舞うこと。
- **無いと失うもの**: 全画面のゲームや動画の前に居座る。寝かせて起こしても何も言わない。読まれないまま消えた吹き出しに気付けず、同じ話を同じ調子で続ける。常駐させておくのが煩わしくなる。
- **代表となる項目**:
  - `OnFullScreenAppMinimize` — 全画面のアプリが出てきて場所を空けたときに届く。
    - id: `ukadoc:list_shiori_event:OnFullScreenAppMinimize:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnFullScreenAppMinimize:1>
  - `OnSysResume` — 機械が眠りから戻ったときに届く。
    - id: `ukadoc:list_shiori_event:OnSysResume:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnSysResume:1>
  - `OnBalloonTimeout` — 読まれないまま吹き出しが閉じたときに届く。
    - id: `ukadoc:list_shiori_event:OnBalloonTimeout:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonTimeout:1>

## 更新

- **定義**: ネットワーク越しに、ゴースト自身を新しいものへ差し替えられること。
- **無いと失うもの**: 作者が直しても手元は古いまま。入れ直す以外に新しくする道が無く、直った台詞も増えた絵も届かない。
- **代表となる項目**:
  - `\![updatebymyself(,オプション,オプション...)]` — ゴースト自身から更新の確認を始める。
    - id: `ukadoc:list_sakura_script:_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1>
  - `OnUpdateBegin` — 更新が始まったときに届く。
    - id: `ukadoc:list_shiori_event:OnUpdateBegin:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateBegin:1>
  - `OnUpdateComplete` — 更新が終わったときに届く。
    - id: `ukadoc:list_shiori_event:OnUpdateComplete:1`
    - <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateComplete:1>
