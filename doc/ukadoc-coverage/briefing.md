# 段階と優先順の統合ブリーフィング

この文書は「どの束をどの段階に置き、段階の中でどの順に並べるか」の正本である。束がどの項目を
持つかは `linkage.md` が持ち、この文書はそれを前提に順位だけを決める。台帳の `priority` は
両者から機械で導いた写しである。

数はこの文書の囲みに 1 度だけ置き、常時の検査が数え直して突き合わせる。本文で同じ数に触れる
ときは、値を写さず欄の名前で指す。項目を指すときは必ず引用符か逆引用符で囲み、地の文に裸で
書かない。

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

段階の中の順位は、4 つの根拠を固定の序列で比べて決める（要件 6.1）。序列は **壊れ方 ＞
伺からしさのテーマ ＞ 影響する既存資産の広さ ＞ 依存基盤の共有度** で、入れ替えない。
テーマは集合なので、比べる値には要素の数を使う。

### 3-1. 4 つの根拠をどこで引くか

同じ値を 2 か所に持たない（設計 D-2）。順位の囲みが自分で持つのは、`linkage.md` に置き場の
無い 2 つ——資産の広さと基盤共有度——だけで、残りは帰属の文書の欄を指す。

| 根拠 | 引く場所 |
| --- | --- |
| ⑴ 壊れ方 | `linkage.md` の束の囲みの `breakage` 欄 |
| ⑵ 伺からしさのテーマ | 同じ囲みの `themes` 欄（比べる値はその要素の数） |
| ⑶ 影響する既存資産の広さ | 下の囲みの `assets` |
| ⑷ 依存基盤の共有度 | 下の囲みの `shared` |
| ⑴ ⑵ の値が由来する構成 id | 同じ囲みの `members` 欄 |

⑶ は、その束の `members` のうち標準テンプレート辞書の語彙に現れる項目の数である（語彙の
全列挙は 5-1 の `[[template]]` の `ids` 欄。2 本の和集合を取ってから数える）。⑷ は、同じ
`foundation` の綴りを持つ束の数で、自分自身を含む。`foundation` を持たない単独項目は 0 と
する——欄が空の者どうしを数え合わせると「基盤を書いていない」が「同じ基盤を共有している」に
化けるためである。

⑴ は 67 の束すべてで同じ値である（数え方と結果は 2-3 に置いた）。したがって順位を実際に
分けるのは ⑵ ⑶ ⑷ の 3 つで、同順位が多く出る（3-5）。

### 3-2. 順位の振り方

段階ごとに、4 つの根拠を上の序列で比べて大きい順に並べ、順位を 1 から振る。4 つとも同じ値の
束は同じ順位にし、その次の順位は 1 だけ増やす（1, 2, 2, 3 の密な順位。要件 6.7・7.1）。
単独項目も 1 つの束として順位を持ち、4 つの根拠が同じ単独項目は `singles` の 1 行にまとめる。

段階が仮のまま順位表に載せた行には、囲みの中に「段階は仮」の注記を付けた。**6 行**である
（数え方: 2-5 の裁定候補 5〜10 に挙がった束を数えた）。裁定で段階が動けば、動いた先の段階の
順位を組み直す。

### 3-3. 順位表

#### 段階 A

```toml
[[rank]]
stage = "A"
rank = 1
bundle = "会話"
assets = 28
shared = 1

[[rank]]
stage = "A"
rank = 2
bundle = "窓の配置と重なり"
assets = 9
shared = 1

[[rank]]
stage = "A"
rank = 3
bundle = "名前の記憶"
assets = 4
shared = 1

[[rank]]
stage = "A"
rank = 4
bundle = "起動と挨拶"
assets = 9
shared = 1

[[rank]]
stage = "A"
rank = 5
bundle = "バルーンの文字"
assets = 5
shared = 1

[[rank]]
stage = "A"
rank = 6
bundle = "サーフェスアニメーション"
assets = 4
shared = 1

[[rank]]
stage = "A"
rank = 7
bundle = "入力窓とダイアログ"
assets = 3
shared = 1

[[rank]]
stage = "A"
rank = 7
bundle = "自発発話"
assets = 3
shared = 1

[[rank]]
stage = "A"
rank = 8
bundle = "終了"
assets = 2
shared = 1

[[rank]]
stage = "A"
rank = 9
bundle = "キーとゲームパッド"
assets = 1
shared = 1
# 段階は仮（2-5 の裁定候補 5）。裁定で段階が動けば順位も組み直す。

[[rank]]
stage = "A"
rank = 10
bundle = "descript の転記"
assets = 0
shared = 1

[[rank]]
stage = "A"
rank = 10
bundle = "バルーンのリンク"
assets = 0
shared = 1

[[rank]]
stage = "A"
rank = 10
bundle = "マウスの矢印"
assets = 0
shared = 1

[[rank]]
stage = "A"
rank = 11
bundle = "メニュー"
assets = 18
shared = 1

[[rank]]
stage = "A"
rank = 12
bundle = "撫で"
assets = 9
shared = 1

[[rank]]
stage = "A"
rank = 13
bundle = "バルーンの付属画像"
assets = 4
shared = 1

[[rank]]
stage = "A"
rank = 14
bundle = "イベントの呼び起こし"
assets = 1
shared = 1

[[rank]]
stage = "A"
rank = 14
bundle = "選択肢の目印"
assets = 1
shared = 1

[[rank]]
stage = "A"
rank = 15
bundle = "絵の重ね方"
assets = 0
shared = 1

[[rank]]
stage = "A"
rank = 16
bundle = "動作モードの出入り"
assets = 2
shared = 1

[[rank]]
stage = "A"
rank = 17
bundle = "定義ファイルの文字コード"
assets = 1
shared = 1

[[rank]]
stage = "A"
rank = 17
bundle = "組み込みの置換語"
assets = 1
shared = 1

[[rank]]
stage = "A"
rank = 18
bundle = "SHIORI の要求と応答"
assets = 0
shared = 1

[[rank]]
stage = "A"
rank = 18
bundle = "シェル定義の転記"
assets = 0
shared = 1

[[rank]]
stage = "A"
rank = 18
bundle = "同期オブジェクト"
assets = 0
shared = 1
```

#### 段階 B

```toml
[[rank]]
stage = "B"
rank = 1
bundle = "インストール"
assets = 4
shared = 1

[[rank]]
stage = "B"
rank = 1
bundle = "更新"
assets = 10
shared = 1
override = { kind = "stage-rule", ref = "要件 5.3" }

[[rank]]
stage = "B"
rank = 2
bundle = "切替"
assets = 12
shared = 1

[[rank]]
stage = "B"
rank = 3
bundle = "消滅"
assets = 5
shared = 1

[[rank]]
stage = "B"
rank = 4
bundle = "投げ込み"
assets = 3
shared = 1

[[rank]]
stage = "B"
rank = 5
bundle = "休止と復帰"
assets = 0
shared = 1

[[rank]]
stage = "B"
rank = 5
bundle = "好感度の絵柄"
assets = 0
shared = 1
# 段階は仮（2-5 の裁定候補 6）。裁定で段階が動けば順位も組み直す。

[[rank]]
stage = "B"
rank = 5
bundle = "着せ替え"
assets = 0
shared = 1

[[rank]]
stage = "B"
rank = 6
singles = ["ukadoc:manual_balloon"]
assets = 0
shared = 0

[[rank]]
stage = "B"
rank = 7
bundle = "配布物の素性"
assets = 6
shared = 1

[[rank]]
stage = "B"
rank = 8
singles = ["ukadoc:manual_directory", "ukadoc:manual_ghost"]
assets = 0
shared = 0
```

#### 段階 C

```toml
[[rank]]
stage = "C"
rank = 1
bundle = "スクリーンセーバー"
assets = 2
shared = 5

[[rank]]
stage = "C"
rank = 1
bundle = "バッテリー"
assets = 2
shared = 5

[[rank]]
stage = "C"
rank = 2
bundle = "OS の変化の察知"
assets = 1
shared = 5

[[rank]]
stage = "C"
rank = 2
bundle = "ディスプレイ変化"
assets = 1
shared = 5

[[rank]]
stage = "C"
rank = 3
bundle = "最小化"
assets = 1
shared = 2

[[rank]]
stage = "C"
rank = 4
bundle = "壁紙"
assets = 1
shared = 1

[[rank]]
stage = "C"
rank = 4
bundle = "通知領域"
assets = 1
shared = 1
# 段階は仮（2-5 の裁定候補 9）。裁定で段階が動けば順位も組み直す。

[[rank]]
stage = "C"
rank = 5
bundle = "スリープ復帰"
assets = 0
shared = 5

[[rank]]
stage = "C"
rank = 6
bundle = "フルスクリーン退避"
assets = 0
shared = 2

[[rank]]
stage = "C"
rank = 7
bundle = "ごみ箱"
assets = 0
shared = 1

[[rank]]
stage = "C"
rank = 8
bundle = "一覧と汎用プロパティの照会"
assets = 1
shared = 2

[[rank]]
stage = "C"
rank = 9
bundle = "サウンド"
assets = 0
shared = 1

[[rank]]
stage = "C"
rank = 9
bundle = "予定表"
assets = 0
shared = 1

[[rank]]
stage = "C"
rank = 10
bundle = "環境の照会"
assets = 0
shared = 2
override = { kind = "stage-rule", ref = "要件 5.3" }
```

#### 段階 D

```toml
[[rank]]
stage = "D"
rank = 1
bundle = "呼び出し"
assets = 3
shared = 2

[[rank]]
stage = "D"
rank = 2
bundle = "SSTP"
assets = 2
shared = 2

[[rank]]
stage = "D"
rank = 2
bundle = "コミュニケート"
assets = 2
shared = 2

[[rank]]
stage = "D"
rank = 3
bundle = "FMO"
assets = 0
shared = 1

[[rank]]
stage = "D"
rank = 4
bundle = "多重ゴースト"
assets = 5
shared = 2

[[rank]]
stage = "D"
rank = 5
bundle = "PLUGIN"
assets = 1
shared = 3

[[rank]]
stage = "D"
rank = 6
bundle = "リンク"
assets = 0
shared = 1
```

#### 段階 E

```toml
[[rank]]
stage = "E"
rank = 1
bundle = "外部アプリ"
assets = 12
shared = 1

[[rank]]
stage = "E"
rank = 2
bundle = "開発者機能"
assets = 3
shared = 1

[[rank]]
stage = "E"
rank = 3
bundle = "ヘッドライン"
assets = 4
shared = 3

[[rank]]
stage = "E"
rank = 4
bundle = "トランスレータ"
assets = 1
shared = 3

[[rank]]
stage = "E"
rank = 5
bundle = "薦める場所"
assets = 1
shared = 1
# 段階は仮（2-5 の裁定候補 8）。裁定で段階が動けば順位も組み直す。

[[rank]]
stage = "E"
rank = 6
bundle = "作り付けの窓"
assets = 0
shared = 1
# 段階は仮（2-5 の裁定候補 7）。裁定で段階が動けば順位も組み直す。

[[rank]]
stage = "E"
rank = 6
bundle = "読み上げと聞き取り"
assets = 0
shared = 1
# 段階は仮（2-5 の裁定候補 10）。裁定で段階が動けば順位も組み直す。

[[rank]]
stage = "E"
rank = 7
bundle = "書庫"
assets = 0
shared = 1

[[rank]]
stage = "E"
rank = 8
singles = ["ukadoc:memo"]
assets = 0
shared = 0
```

### 3-4. 順序の主張から外した行（型の付いた例外の印）

要件 5.3 は「更新」のテーマを持つ束を段階 B の先頭に、`system.*` の照会を段階 C の末尾に
置くと定める。これは人の決めであって 4 つの根拠から出る順序ではないので、両立しない 2 行に
`override` の印を付け、順序の主張（3-2）から外した。**2 行**である（数え方: 上の 5 つの囲みで
`override` を持つ行を数えた。第二段の改訂（要件 9）の印はまだ 0 行である）。

**「更新」を段階 B の先頭に置く**（`override = { kind = "stage-rule", ref = "要件 5.3" }`）

- 何が問題か: 4 つの根拠で並べると、この束は段階 B の 4 番目に落ちる。要件 5.3 は先頭に
  置くと定めている。「更新」のテーマを持つもう 1 つの束「インストール」は、根拠の順でも
  先頭に来るので印は要らない。
- 何を決めるか: 「更新」を順位 1 のまま置くか、根拠の順に戻して 4 番目にするか。
- 答えで変わること: 順位 1 のままなら、配布されたゴーストが網越しに自分を新しくできるように
  なるのが段階 B の最初になる。根拠の順に戻せば、シェルとバルーンの差し替えと消滅が
  先に動き、更新はその後になる。

**`system.*` の照会を段階 C の末尾に置く**（同じ印）

- 何が問題か: 4 つの根拠で並べると「環境の照会」は段階 C の 9 番目で、末尾より前に来る。
  要件 5.3 は末尾に置くと定めている。`system.` で始まる id を持つ束はこの 1 つだけで、
  ほかは **0 束**である（数え方: `linkage.md` の 67 の囲みの `members` を走査し、`system.`
  を含む id が 1 つでもある囲みを数えた。この束の中では 28 件中 25 件が該当する）。
- 何を決めるか: 末尾に置いたままにするか、根拠の順に戻して 9 番目にするか。
- 答えで変わること: 末尾のままなら、機械の様子や時刻を尋ねる語にゴーストが答えられるように
  なるのは段階 C の最後で、音の再生と予定の通知が先に動く。根拠の順に戻せば、その 2 つより
  先に照会が答えるようになる。

どちらも本タスクでは決めていない。先頭ウェーブの選定（要件 10）の段で開発者の裁定に上げる。

### 3-5. 同順位（要件 6.7）

4 つの根拠がすべて同じ値になった束は、同じ順位で並べた。**13 組・29 束**である（数え方:
上の 5 つの囲みのうち `override` も `insufficient` も持たない行を段階ごとに集め、同じ順位の
行が 2 つ以上ある組と、その組に属する束を数えた。`singles` の 1 行に 2 つの id が並ぶ行は
2 束として数えた）。

同順位のままでは「どちらを先に作るか」が決まらないので、解消は先頭ウェーブの選定（要件 10）の
段で開発者の裁定候補に上げる。本タスクでは順序を作らない——4 つの根拠のほかに順序の根拠を
足すと、要件 6.1 が凍結した序列に 5 つ目の根拠を足すことになる。

### 3-6. 根拠不足の一覧（要件 6.6）

**0 束である。** 数え方: 4 つの根拠のうち値が空欄になる束を数えた。⑴ ⑵ は `linkage.md` の
67 の囲みがすべて `breakage`・`themes` の欄を持ち、⑶ ⑷ は上の囲みの `assets`・`shared` に
数（0 を含む）が入る。要件 6.8 の退路は使っていない（5-1 の `[[template]]` の `fallback` は
2 本とも偽）ので、`insufficient = true` を付けた行も **0 行**である。したがって順位表から
外した束は無く、67 の束と単独項目はすべて上の 5 つの囲みにちょうど 1 度ずつ現れる。

### 3-7. 段階ごとの束数・単独項目数・項目数（要件 5.6）

5 段階すべてを、値が 0 の段階も省略せずに置く。数え方: `bundles` は上の囲みの `bundle` の
行の数、`singles` は `singles` の行に並んだ id の総数、`items` はその段階の束と単独項目の
`members` を重複を除いて数えた数である。`items` の 5 つの和は「合計」の `target`
（`linkage.md`）と一致する。段階 A・C・D の `singles` が 0 なのは、4 つの単独項目のうち
3 つが段階 B に、1 つが段階 E に落ちて、この 3 段階には 1 つも落ちなかったことを確かめた
0 である。

```toml
[stage.A]
bundles = 25
singles = 0
items = 874

[stage.B]
bundles = 9
singles = 3
items = 210

[stage.C]
bundles = 14
singles = 0
items = 169

[stage.D]
bundles = 7
singles = 0
items = 133

[stage.E]
bundles = 8
singles = 1
items = 166
```

## 4. 段階 A の主障壁

段階 A の節目は「当面 emo2 が動けばよい」である。その節目に届くのを妨げている物として、brief は
2 つを名指した——⑴ ベースウェアとゴーストの間でやり取りする口がほとんど無いこと、⑵ 定義ファイルに
作者が書いた記述を areka が引き当てなかったとき、利用者にも作者にも何も知らせずに捨てること。
この節は、その 2 つが台帳のどのページにどれだけ残っているかを、ページ単位の状態の分布として置く。

**数え方**（6 ページとも同じ手順。作業ツリーの根で走らせる）: 台帳 4 本の項目は、行頭が
`[entry."` で始まる見出しの行をちょうど 1 つと、その直後に行頭が `status = ` の行をちょうど
1 つ持つ。見出しの名前はコロンで 4 つに割れ、その 2 つ目がページ名である。見出しからページ名を
取り、直後の `status` の値でページごとに数え上げた。これは `cargo run -p ukadoc-survey -- report`
が作るドメイン別報告の「ページ別の状態の分布」の表と同じ数え方であり、下の 6 行はその表と
食い違わない（`report/shiori.md` と `report/assets.md` の同名の節の該当行を 1 行ずつ照合した）。

**下の 2 つの囲みは、ページの全項目を数えたものである。** 段階 A に落ちた項目だけを数えたもの
ではない。要件 8.1 ⑷ が数え直しを求めているのは「着手時の実測」9 の数で、その数がページ単位
だからである。段階 A に落ちた項目の総数は 3-7 の `[stage.A]` の `items` が持つ。

### 4-1. 障壁 ⑴ イベントと問い合わせの口

`list_shiori_event` はベースウェアがゴーストへ渡すイベントの一覧、`list_shiori_resource` は
ベースウェアがゴーストへ問い合わせる項目の一覧である。下の囲みの `list_shiori_event` の行は、
`absent` が 6 行のどの数よりも多く、`implemented` より桁違いに大きい。ゴーストが「起動した」
「クリックされた」「更新が終わった」を知る手立てがこれだけ欠けていると、利用者から見える結果は
「話しかけても何も返らない」である。

`list_shiori_resource` の行は `absent` が **0** で、ほぼ全部が `vocabulary_only` に乗る。この 0 は
「未対応の項目が 1 件も無い」という意味であって、「実装が済んでいる」という意味ではない。語彙のみは
「綴りは台帳に載っているが areka はその値を一度も問い合わせない」状態を指す（状態の 7 語の定義は
`README.md`）。上と同じ手順で数え、`status` の値が `absent` の行がこのページに 1 つも無いことを
確かめた 0 である。

```toml
[[barrier]]
page = "list_shiori_event"
implemented = 11
vocabulary_only = 3
degraded = 0
absent = 273
alias = 3
not_applicable = 0

[[barrier]]
page = "list_shiori_resource"
implemented = 1
vocabulary_only = 158
degraded = 0
absent = 0
alias = 0
not_applicable = 0
```

### 4-2. 障壁 ⑵ 未知の記述が無言で捨てられる

`descript_ghost`・`descript_balloon`・`descript_shell`・`descript_shell_surfaces` は、ゴースト・
バルーン・シェル・サーフェスの定義ファイルに書けるキーの一覧である。作者が書いたキーを areka が
引き当てなかったとき何が起きるかは、`briefing-assets.md` の「未知の記述の扱い」節が転記層の
コードを読んで確かめている——読み取りの経路にエラー段の記録は 1 行も無く、既定の記録の水準では
利用者に何も見えない。下の囲みの 4 行の `absent` と `vocabulary_only` が、その「引き当てない」側の
量である。

4 行のうち `descript_balloon` と `descript_shell_surfaces` は `degraded` が 0 でない。縮退は
「読んではいるが正典どおりには効かない」状態で、無言で捨てる経路とは別の壊れ方である（状態の
7 語の定義は `README.md`）。残る `descript_ghost` と `descript_shell` の `degraded` は 0 で、
この 2 ページには縮退させた実装が 1 つも無い。

```toml
[[barrier]]
page = "descript_ghost"
implemented = 7
vocabulary_only = 1
degraded = 0
absent = 66
alias = 0
not_applicable = 0

[[barrier]]
page = "descript_balloon"
implemented = 20
vocabulary_only = 5
degraded = 4
absent = 133
alias = 0
not_applicable = 0

[[barrier]]
page = "descript_shell"
implemented = 11
vocabulary_only = 2
degraded = 0
absent = 89
alias = 0
not_applicable = 0

[[barrier]]
page = "descript_shell_surfaces"
implemented = 4
vocabulary_only = 57
degraded = 4
absent = 68
alias = 4
not_applicable = 0
```

### 4-3. 0 と書いた欄

上の 2 つの囲みには 0 の欄が 15 ある（`not_applicable` 6・`degraded` 4・`alias` 4・`absent` 1）。
0 は「調べていない」ではなく「数えて 1 件も無かった」の印なので、内訳と理由を書く。数え方は
いずれもこの節の冒頭と同じで、ページ別に数え直した結果である。

- `not_applicable` は 6 行とも 0 である。台帳全体の対象外の項目（5-2 の `[priority_blank]` の
  `not_applicable`）は `list_shiori_event_ex`・`memo_shiorievent`・`list_sakura_script` の 3 ページ
  だけに乗っており、この 6 ページには 1 件も無い。
- `degraded` は `list_shiori_event`・`list_shiori_resource`・`descript_ghost`・`descript_shell` の
  4 行が 0 である。この 4 ページには縮退させた実装が 1 つも無い。
- `alias` は `list_shiori_resource`・`descript_ghost`・`descript_balloon`・`descript_shell` の
  4 行が 0 である。別名の項目がこの 4 ページに 1 件も無い。
- `absent` は `list_shiori_resource` の 1 行が 0 である。理由は 4-1 に書いた。

## 5. 根拠表への参照

順位を付けるときに引いた分布の表は、いずれも機械が作る報告の中にある。**この節はその置き場を
指すだけで、表そのものを写さない**（要件 2.4・8.8）。写しを作れば、台帳を直して報告を作り直した
日に、この文書の中の写しだけが古いまま残る。

| 見たい分布 | 置き場 | 作り直す副手続き |
| --- | --- | --- |
| テーマ別の状態分布 | `report/summary.md` の「テーマ別の状態分布」節 | `cargo run -p ukadoc-survey -- report-summary` |
| 状態の分布（全体）・ドメイン別の状態の分布・ドメインを跨いで繋がった束 | `report/summary.md` の同名の 3 節 | 同上 |
| SSP 世代別の対応表 | `report/shiori.md`・`report/assets.md`・`report/sakura-script.md`・`report/property.md` の「SSP 世代別の対応表」節 | `cargo run -p ukadoc-survey -- report` |
| ページ別の状態の分布 | 同じドメイン別報告 4 本の「ページ別の状態の分布」節 | 同上 |

`report/summary.md` が全体報告の実在するパスである（1-2 で確かめた。brief が書いた綴りは
実在しないので、上の表はどこも実在するファイル名で指している）。

**世代別の表は全体報告に無い。** `report/summary.md` の見出し（行頭が `## ` の行）を数えると
6 つで、そのうち世代を表す語を含む見出しは **0 件**である。だから世代別の見方はドメイン別報告
4 本に委ねる（要件 2.3）。世代別の合計を述べたくなったときは、上の表が指す 4 本の「SSP 世代別の
対応表」から数え、数えた手順を添える。

**この文書は世代別の数もテーマ別の数も 1 つも書いていない。** どちらも **0 か所**である。
確かめ方: この文書を「世代」「テーマ別」の 2 語で検索し、当たった行を 1 行ずつ読んで、分布の
表でも合計でもないことを確かめた（当たったのは、報告がどんな分布を載せる文書かを述べる 5-2 と
5-3 の各 1 行と、この節の説明だけである）。順位の根拠のうちテーマを見るのは 3-1 だが、そこが
引くのは束ごとの `themes`（`linkage.md`）で、テーマ別の状態分布の表とは別物である。

**報告は手で編集しない。** 台帳 4 本のいずれかを触った回は、上の副手続きを走らせて報告を
作り直し、食い違いは作り直しで解消する。報告の本文へ説明を書き足さない
（`README.md`「4. 報告の扱い」）。

#### 全体報告の新しさは常時の検査が判定する（上流の除外を本 spec が覆した）

`ukadoc-survey-toolkit`（完了済み）は全体報告を常時の検査（`cargo test -p ukadoc-survey`）から
外していた。外した理由は「調査 4 本が同時に走っていて、同じ 1 つのファイルを取り合うから」で
ある——4 本のどれが台帳を触っても全体報告が古くなり、触っていない 3 本の作業まで赤で止まる。
その 4 本（shiori・assets・sakura-script・property）は 2026-09-06 までに全部完了したので、
取り合う相手はもう居ない。**本 spec はこの除外を覆した。** いま `report/summary.md` の本文は、
カタログと台帳 4 本から作り直した本文と 1 文字でも違えば常時の検査が赤になる（要件 11.2）。
台帳を触った回に作り直しを忘れると、その場で赤くなって気付ける。

**証拠の件数の表は全体報告から外し、`cargo run -p ukadoc-survey -- evidence` の出力に一本化した**
（開発者裁定 2026-09-12）。以前の全体報告は末尾に「ドメインごとの証拠あり件数」の 4 行を
載せていたが、この数はカタログや台帳ではなく**ソースの木を歩いて**数えたものなので、areka の
どこか別の場所で正典 URL のコメントが 1 行増えるだけで古くなる。全文一致の判定に入れると、
`doc/ukadoc-coverage/` に無関係な作業までこの検査で赤くなってしまう。かといって判定から
外したまま本文に残せば、誰も見張らない数が正典の報告に居座り、必ず古びる。だから本文から
外し、読みたいときは上の副手続きで読む。これは `ukadoc-survey-toolkit` が定めた「報告に証拠の
有無を載せる」（同 spec の要件 2.3・設計 D-11）を**本 spec が覆したもの**である。

以上 2 件の覆しは `README.md` の「誰が何を作り直すか」の表と「4. 報告の扱い」節にも書く。

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

### 5-2. 書き戻しの前後のドメイン別段階分布（要件 7.3）

台帳の `priority` 欄には統合の前から値が入っていたが、その作り方は 4 本の調査でばらばら
だった。assets は 5 段階にほぼ等しい件数で割り、shiori は群ごとに 1 つの段階を仮に置き、
sakura-script と property は対象の項目を段階 C だけに置いていた。書き戻しはこの 4 通りを
1 つの作り方（`linkage.md` の束の帰属と 3-3 の順位表から導いた「段階 1 文字＋段階内の束の
順位」）に揃える。揃う前と揃った後を、同じ 1 つの手順で数えて並べる。

**数え方**（前も後も同じ手順。作業ツリーの根で走らせる。台帳の項目は行頭の `priority = `
の行をちょうど 1 つ持つので、その行の値の 1 文字目を段階として数え、値が空の行を「(空)」
として数える）:

```sh
for f in shiori sakura-script assets property; do
  echo "== $f"
  grep '^priority = ' doc/ukadoc-coverage/ledger/$f.toml | sed 's/^priority = "\(.*\)"$/\1/' | cut -c1 | sed 's/^$/(空)/' | sort | uniq -c
done
```

**書き戻しの前**（2026-09-13 に上の手順で数えた出力を写したもの）:

| ドメイン | A | B | C | D | E | 空 | 合計 |
|---|---:|---:|---:|---:|---:|---:|---:|
| shiori | 58 | 61 | 180 | 160 | 25 | 193 | 677 |
| sakura-script | 0 | 0 | 301 | 0 | 0 | 41 | 342 |
| assets | 112 | 92 | 103 | 127 | 104 | 4 | 542 |
| property | 0 | 0 | 186 | 0 | 0 | 2 | 188 |
| 合計 | 170 | 153 | 770 | 287 | 129 | 240 | 1,749 |

表の 0 は書き落としではない。sakura-script と property の A・B・D・E が 0 なのは、上の手順の
出力にその 4 文字の行が 1 つも現れなかったこと、すなわち段階 C 以外の値を持つ項目が 1 件も
無いことを数えて確かめた 0 である。4 本の作り方の違いはこの表の形に出ている——2 本は段階が
1 つしか無く、assets は 5 段階にほぼ均等、shiori だけが 5 段階に不均等である。

空欄の 240 も 4 本で意味が違っていた。同じ手順に状態の欄を足して数えると（`priority = ""`
の行の直前にある `status` の行を数える）、内訳は実装済み 46（shiori 21・sakura-script 23・
property 2）・別名 24（shiori 3・sakura-script 17・assets 4）・対象外 170（shiori 169・
sakura-script 1）である。実装済みを空にするか値を入れるかが 4 本で割れており、別名 27 の
うち 3 件（sakura-script）には値が入っていた。

**書き戻しの後**（同じ手順で数えた出力を写したもの）:

| ドメイン | A | B | C | D | E | 空 | 合計 |
|---|---:|---:|---:|---:|---:|---:|---:|
| shiori | 224 | 83 | 58 | 64 | 76 | 172 | 677 |
| sakura-script | 193 | 23 | 20 | 16 | 69 | 21 | 342 |
| assets | 395 | 76 | 12 | 40 | 15 | 4 | 542 |
| property | 62 | 28 | 79 | 13 | 6 | 0 | 188 |
| 合計 | 874 | 210 | 169 | 133 | 166 | 197 | 1,749 |

4 本の作り方の違いは消えた。どのドメインも 5 段階すべてに項目を持ち、段階は束の帰属から
一意に決まる。property の空 0 は、このドメインに別名の項目も対象外の項目も 1 件も無いことを
状態の欄で数えて確かめた 0 である（property の 188 項目は語彙のみ 178・縮退 8・実装済み 2）。
段階ごとの合計 874・210・169・133・166 は 3-7 の `[stage.*]` の `items` と一致し、その和は
1,552 である。空欄の 197 は別名 27 と対象外 170 の和で、前の 240 とは中身が違う（前の 240 に
含まれていた実装済み 46 は段階を持つ側へ移り、値が入っていた別名 3 件は空へ移った）。

**走らせた副手続きと結果**: `cargo run -p ukadoc-survey -- priority-apply` を 2 回続けて
走らせた。1 回目は shiori 439・assets 494・sakura-script 324・property 188（計 1,445）項目を
変更し、2 回目は 4 本とも「変更 0 項目」だった（冪等）。台帳を触ったので
`cargo run -p ukadoc-survey -- report` と `cargo run -p ukadoc-survey -- report-summary` を
続けて走らせた。報告 5 本の本文に差分は出ていない（0 本）。報告は状態・テーマ・世代の分布を
載せる文書で `priority` 欄を載せないので、`priority` だけの変更では本文が変わらない。
改行の違い（副手続きは LF で書き、作業ツリーは CRLF）は手で直していない。

**赤になった所見**: **0 件**である。書き戻しと報告の作り直しの後に
`cargo test -p ukadoc-survey` を走らせ、単体 600・cli_streams 6・consistency 81・doctests 5 が
すべて緑だった（書き戻しの前と同じ本数）。判定 ⑷（段階と順位）と判定 ⑹（全体報告の
新しさ）はこの時点ではまだ実装されていないので、この 0 件は「今ある検査が赤にならなかった」
という意味であり、段階と順位の正しさを主張するものではない。その主張は 4.7 と 4.8 の判定が
入ってから成り立つ。

```toml
[[after]]
domain = "shiori"
A = 224
B = 83
C = 58
D = 64
E = 76
empty = 172

[[after]]
domain = "sakura-script"
A = 193
B = 23
C = 20
D = 16
E = 69
empty = 21

[[after]]
domain = "assets"
A = 395
B = 76
C = 12
D = 40
E = 15
empty = 4

[[after]]
domain = "property"
A = 62
B = 28
C = 79
D = 13
E = 6
empty = 0

[priority_blank]
alias = 27
not_applicable = 170
```

### 5-3. 宛先の欄の整理（要件 7.4・7.5）

台帳の `owner` 欄は「この項目を引き受ける spec の名前」を書く欄である。整理の前は 1,749 件の
うち 447 件に名前が入っており、その中に既に完了して封じられた spec の名前も混じっていた。
完了した spec はもう作業を受け取れないので、まだ実装されていない項目に完了済み spec の名前が
入っていると、読み手は「引受先が決まっている」と読み違える。次の 4 つの規則で整えた。

**数え方**（前も後も同じ手順。作業ツリーの根で走らせる。台帳の項目は行頭の `owner = ` の行を
ちょうど 1 つ持つ）:

```sh
grep -h '^owner = ' doc/ukadoc-coverage/ledger/*.toml | sed 's/^owner = //' | sort | uniq -c
```

宛先の名前を 2 つの集まりに突き合わせる。進行中 spec は `.kiro/specs/` の直下で、ディレクトリ名が
`completed` でも本文書を持つ spec 自身でもなく、`brief.md` を持つもの（2026-09-13 に数えて
**27 本**）。完了済み spec は `.kiro/specs/completed/` の直下にあるもの（同 **174 本**）。

**規則 ⑴——進行中 spec 宛ての宛先は保つ。** 該当は **372 件・13 spec**。変えた件数は **0** で
ある（1 件も空にせず、1 件も書き換えていない）。

**規則 ⑵——完了済み spec 宛ての宛先は状態で分ける。** 該当は **75 件・16 spec**。状態が実装済み
か縮退の **41 件**（実装済み 33・縮退 8）は「その項目を実装した spec の記録」として保った。状態が
未対応か語彙のみの **34 件**（未対応 32・語彙のみ 2）は宛先を空にし、理由を備考に書いた。変えた
件数は **34** である。宛先だった 16 spec のうち 2 本は持っていた項目がすべて未対応だったので全件
が空になり、下の列挙から落ちる（`areka-P0-window-placement` 16 件・`areka-P0-emo-atlas` 4 件）。
残るのは **14 spec** である。

**規則 ⑶——brief がまだ無い候補 spec の名前を宛先に書かない。** 整理の前も後も、非空の宛先の
うち進行中 27 本にも完了済み 174 本にも当たらない名前は **0 件**であった。変えた件数は **0** で
ある。この 0 は書き落としではなく、整理の前に上の手順が出した非空の名前 29 通り（進行中 13・
完了済み 16）を 2 つの集まりに 1 つずつ当てて、どれも外れなかったことを数えた 0 である。整理の
後は 27 通りになり（規則 ⑵ で 2 本が全件空になった分）、同じく外れは無い。候補 spec への割り
当ては `roadmap-draft.md` の側に持つ。

**規則 ⑷——変えた件数を書く。** 合計 **34 件**（⑴ **0** 件・⑵ **34** 件・⑶ **0** 件）。整理の
後は非空の宛先が **413 件**、空が **1,336 件**で、和は 1,749 である。

**備考への書き足し方**（要件 7.5）: 空にした 34 件には、備考の末尾に 1 行だけ足した。足した行は
「担当の欄を空にした・宛先だった spec 名・完了して封じられていること・状態・規則の在り処・上に
残る担当の記述は経緯として置くこと」を書く。既にある記述は 1 行も消していない。差分の削除行は
**34 行**で、そのすべてが `owner = ` の行である（備考の行も優先度の行も削除側に現れない）。
行番号はどの行にも書いていない。

台帳を触ったので報告 5 本を副手続きで作り直した（`cargo run -p ukadoc-survey -- report` と
`cargo run -p ukadoc-survey -- report-summary`）。本文に差分は出ていない（**0 本**）。報告は状態・
テーマ・世代の分布を載せる文書で `owner` 欄を載せないので、宛先だけの変更では本文が変わらない。
改行の違いは手で直していない。

**保った完了済み spec の列挙。** 判定はこの列挙だけを見る（生きた `completed/` の全走査をしないので、
他の spec が完了しても赤にならない）。`items` は台帳でその名前を宛先に持つ項目数であり、判定が
数え直す。14 本の和は 41 で、規則 ⑵ で保った件数と一致する。

```toml
[[owner_completed]]
spec = "areka-P0-balloon-offset-dpi"
items = 2

[[owner_completed]]
spec = "areka-P0-balloon-parse"
items = 5

[[owner_completed]]
spec = "areka-P0-balloon-vertical-canon"
items = 4

[[owner_completed]]
spec = "areka-P0-bindoption-exclusivity"
items = 2

[[owner_completed]]
spec = "areka-P0-cursor-tag-canon"
items = 2

[[owner_completed]]
spec = "areka-P0-ghost-setup"
items = 1

[[owner_completed]]
spec = "areka-P0-kero-balloon"
items = 3

[[owner_completed]]
spec = "areka-P0-mayuna-compose"
items = 3

[[owner_completed]]
spec = "areka-P0-package-mount"
items = 4

[[owner_completed]]
spec = "areka-P0-sakura-dialogue-tags"
items = 1

[[owner_completed]]
spec = "areka-P0-scope-zorder-pinning"
items = 3

[[owner_completed]]
spec = "areka-P0-shell-parse"
items = 4

[[owner_completed]]
spec = "areka-P0-sylphya"
items = 4

[[owner_completed]]
spec = "areka-P0-windowposition-limit"
items = 3
```

## 6. 申し送りの処分台帳

<!-- 段 5（タスク 6.3）で書く: 調査 4 本のブリーフィングと完了 spec 5 本から拾った申し送りを
     1 件ずつ処分する。拾った件数・処分済み件数・裁定候補の件数を冒頭に並べる。

     ⚠ タスク 4.4 の書き戻しで事実でなくなった記述を 1 件、先に書き留める。上の検索（4 つの
     語でブリーフィング 4 本と完了 spec 5 本の `tasks.md` を引く）は台帳の冒頭コメントを
     読まないので、この 1 件はその検索には掛からない。処分は 6.3 で付ける。

     出典: `doc/ukadoc-coverage/ledger/shiori.toml` の冒頭コメント（`briefing-shiori.md` の
     「群の索引」の写し）。2026-09-13 にタスク 4.4 の担当が同じファイルを読み直して数え直した
     実測は次の 4 点で、いずれも本文の記述と食い違う。

     ⑴ 「677 行のうち 484 行に記入済みで、193 行は意図的に空」とあるが、実測は記入済み 505・
        空 172 である（行頭の `priority = ` の行が 677、うち値が空の行が 172）。
     ⑵ 「空にしてあるのは `implemented` 21・`alias` 3・`not-applicable` 169」とあるが、いま
        空なのは `alias` 3 と `not-applicable` 169 の 172 だけである（`implemented` の 21 件は
        段階を持つ側へ移った）。空の行の直前の状態の欄を数えると、この 2 種しか現れない。
     ⑶ 「下の群の欄が『優先度: `""`』と書いてある 7 つの群（群 1・3・5・6・9・12・15）が
        これに当たる」とある。欄の行数は実測でも 7 行だが、そのうち群 1（送出しているイベント。
        欄に「作業が残っていない」と添えてある）の項目はもう空ではない。⑵ のとおり空は別名と
        対象外だけになったからである。
     ⑷ 「優先度はいずれも仮置きである」を含め、冒頭コメントで「仮置き」に触れる行は実測 14 行
        ある。うち 9 行は群ごとの既定値（`C1`・`C2`・`D1`・`D2`・`E1`）を名指しし、2 行は
        「テーマから決まる（仮置き）」と書く。書き戻し後の値は `linkage.md` の束の帰属と
        本文書 3-3 の順位表から導いたものなので、これら群ごとの既定値とは対応しない。

     処分の向き（確定は 6.3）: この記述の正本は `briefing-shiori.md` であり、要件 12.3 が
     その編集を禁じ、要件 12.2 が台帳への接触を `priority`・`owner` の欄に限る。よって本 spec
     では直さず、是正候補として扱う（直すのは `briefing-shiori.md` を所有する側）。

     `README.md` について: 「仮置き」に触れるのは 2 か所（欄の定義の表の `priority` の行と、
     「段階（A〜E）の最終決定はここでは行わない」の節）である。要件 9.1 が第一段の順位を
     「草案」と位置づけているので、この 2 か所は第二段（タスク 5.1〜5.3）が終わるまでは
     まだ正しい。見直すのは第二段の完了後である。要件 12.4 が `README.md` の編集を要件 11.2
     まわりに限っているので、タスク 4.4 では触っていない。

     引受先の確認: タスク 6.3（`_Boundary: briefing.md_`・要件 8.2〜8.5）が本 spec の
     `tasks.md` に実在することを 2026-09-13 に読んで確かめた。
     -->

## 7. 第二段の改訂記録

### 7-1. 第一段の順位は草案である

**3-3 の順位表は第一段の草案である**（要件 9.1）。第一段は台帳の状態・`linkage.md` の欄・
標準テンプレート辞書の語彙だけを材料にして 4 つの根拠を数えたもので、実物が動くところを
まだ見ていない。第二段は M1 完成の実機の記録を材料にして、この草案のどの行を動かすかを
1 行ずつ決める。本節がその記録であり、動かすと決めた行があれば 3-3 の囲みを書き換える。

### 7-2. 読んだもの

| 材料 | 出どころ | 行数 |
| --- | --- | --- |
| 適合検証の項目 | 完了 spec `areka-P0-emo2-conformance-e2e` の設計「適合検証項目表」。項目ごとの結果と根拠は同 spec の受入記録 §8.1 | 20 |
| 持ち越し | 同 spec の完成判定 §6「未達と引受先」の表 | 8 |

合わせて **28 行**である（数え方: 上の表の「行数」の欄を足した。20 ＋ 8 ＝ 28）。

持ち越し行の見出しは、§6 の表の 1 列目のうち「——」の手前までをそのまま写した綴りを使う。
順位の囲みに第二段の根拠を書くときに受け付けられる綴りがこの形だからである（要件 9.2）。
項目の側は「項目」の語と 1 から 20 までの番号で指す。

実機の結果は **20 項目すべて合格**で、縮退は **1 件**（項目 18 の後半。開発者が
2026-09-11 に許容と裁定した）である。§6 の 8 行のうち 7 行は持ち越しで、1 行
（R8.3 後半の未履行）は同じ日の裁定で確定した。

### 7-3. 20 項目の判断

「関わる束」は、その項目が見た振る舞いを成り立たせる基盤を `linkage.md` の `foundation` の
欄で引いて選んだ束の名前である。順位そのものはここに写さず 3-3 で引く（同じ数を 2 か所に
持たない）。「正典の語彙の欠けを示していない」は、その行が台帳の状態・テーマ・テンプレート
辞書の語彙・基盤の綴りのどれも動かさない、という意味である。

| 項目 | 見出し | 関わる束 | 順位を動かすか | 判断の理由 |
| --- | --- | --- | --- | --- |
| 項目 1 | 起動と既定位置 | 起動と挨拶・窓の配置と重なり | いいえ | 合格。起動しただけで二体が既定の位置に出て、96 dpi でない水準でも座標が崩れなかった。正典の語彙の欠けを示していない |
| 項目 2 | 起動挨拶の再生 | 会話・バルーンの文字 | いいえ | 合格。1 文字ずつの送り・待ち・改行・表情の切替が台本の順で起きた。正典の語彙の欠けを示していない |
| 項目 3 | 着せ替えの表情 | 着せ替え・サーフェスアニメーション | いいえ | 合格。挨拶の途中で本体の表情が変わり、変わった後に貼り付いたまま残らなかった。正典の語彙の欠けを示していない |
| 項目 4 | まばたき 2 系統 | サーフェスアニメーション | いいえ | 合格。放置しているあいだ本体と相方がそれぞれまばたいた。正典の語彙の欠けを示していない |
| 項目 5 | 放置で自発会話・会話中は割り込まない | 自発発話 | いいえ | 合格。何もしないでいると自発会話が始まり、その最中に別の会話が割り込まなかった。正典の語彙の欠けを示していない |
| 項目 6 | 撫で反応（本体側と相方側の両方） | 撫で | いいえ | 合格。本体と相方の両方で当たり領域の名が載った反応が返った。狙った 4 か所のうち領域名が載ったのは 2 か所だが、当たり領域の語彙そのものの状態は台帳で変わらない |
| 項目 7 | 二重クリックでメニュー | メニュー・バルーンの文字 | いいえ | 合格。字下げされた 3 つの選択肢がすべて見え、反転帯から下へはみ出した文字のインクは 0 画素だった。正典の語彙の欠けを示していない |
| 項目 8 | 選択によるシーン遷移の一周 | メニュー | いいえ | 合格。選択・サブメニュー・もどる・閉じるまで一周し、選択肢と同名のイベントが 1 段だけ出た。正典の語彙の欠けを示していない |
| 項目 9 | 位置調整 | 窓の配置と重なり | いいえ | 合格。メニューの位置調整で相方の窓が算出位置へ動いた。移動量に拡大率を掛けるのは登記済みの意図的な差である。正典の語彙の欠けを示していない |
| 項目 10 | 二人立ち総合 | 窓の配置と重なり・会話・サーフェスアニメーション | いいえ | 合格。相方の窓とバルーンが出て、話者が替わり、別名指定で相方の表情が変わり、両方のバルーンがそれぞれのキャラに付いて動いた。正典の語彙の欠けを示していない |
| 項目 11 | 利用者名の展開 | 組み込みの置換語 | いいえ | 合格。挨拶の文面に利用者名が入り、生の記法は漏れなかった。正典の語彙の欠けを示していない |
| 項目 12 | 位置の永続化 | 窓の配置と重なり | いいえ | 合格。1 回目の起動で保存した位置を 2 回目の起動が読み戻した。走行の途中で保存位置が消えたのは準備者の手順違反であって製品の側ではない。正典の語彙の欠けを示していない |
| 項目 13 | 終了 | 終了 | いいえ | 合格。終了挨拶が流れてから窓が自分で閉じ、解放がちょうど 1 度だけ起きた。挨拶の後の 15 秒の待ちは上流のゴーストの台本の事情で、areka の語彙の欠けではない |
| 項目 14 | 省略した機能の縮退 | 更新・切替・バルーンの付属画像・選択肢の目印・メニュー | いいえ | 合格。更新系とバルーン変更のイベントを送らなくても、矢印・目印・通信系の資産が無くても壊れず、選択待ちが 30.5 秒で時間切れになった。送らなくてよいと確かめられたことは、ここに挙がる 5 つの束の 4 つの根拠の値をどれも変えない |
| 項目 15 | バルーンの表示ライフサイクル | 会話・バルーンの文字 | いいえ | 合格。起動直後は見えず、会話が始まると現れ、終わってしばらくして消えた。正典の語彙の欠けを示していない |
| 項目 16 | 拡大率の切替 | 窓の配置と重なり | いいえ | 合格。接地点が新しい作業領域の下端に載り、バルーンが同じコマで付いて動き、途中の矩形は出なかった。見た目の跳ねは登記済みで合否に載せない。正典の語彙の欠けを示していない |
| 項目 17 | 掴んで動かしたときの追従 | 窓の配置と重なり | いいえ | 合格。掴んで動かすと窓がカーソルへ 1 対 1 で付いてきた。正典の語彙の欠けを示していない |
| 項目 18 | 二体の隣接 | 窓の配置と重なり | いいえ | 合格（縮退あり）。既定では隙間 0 で隣り合ったが、拡大率を 200% から 150% へ変えた後は間が開いたままだった。開発者が 2026-09-11 に許容と裁定している。縮退したのは既に実装済みの振る舞いの一部で、この束の 4 つの根拠の値はどれも変わらない |
| 項目 19 | 再表示直後の重なり順 | 窓の配置と重なり | いいえ | 合格。描き直された直後もバルーンがキャラの手前に居続けた。正典の語彙の欠けを示していない |
| 項目 20 | 子プロセスへの受け渡し | SHIORI の要求と応答・起動と挨拶 | いいえ | 合格。実 32bit の脳が立ち上がって応答し、接続失敗 0 行・正規の解放 1 行が揃った。正典の語彙の欠けを示していない |

### 7-4. 持ち越し 8 行の判断

| 持ち越し行の見出し | 関わる束 | 順位を動かすか | 判断の理由 |
| --- | --- | --- | --- |
| 記録 §13.1 行 3 | 窓の配置と重なり | いいえ | 初回起動限定の位置調整が 2 回目以降の起動で既定配置へ戻るかは**未観測**である。走行のあいだに決定論のテストが保存された位置を消したため確かめられなかった。観測が無いものは順位の材料にならない。引受先の spec は無く、次の実機一周での目視に据え置かれている |
| 記録 §13.1 行 1 | 窓の配置と重なり | いいえ | 絵が新しい寸法になってから窓がその寸へ動くまでの遅れ。開発者が 2026-09-10 に許容と裁定し、判定に載せないと決まっている。areka の内部の追従の速さの話で、正典の語彙の欠けではない。引受先は起票済み（7-6） |
| 症状 E | 絵の重ね方 | いいえ | 拡大率 200% で 1 コマの適用が重い。絵が変わるコマの費用そのものの問題で、示しているのは実装の速さであって正典の語彙の欠けではない。引受先は起票済み（7-6） |
| 記録 §13.2 行 4 | 起動と挨拶 | いいえ | 起動系列の途中に届いた再生完了の通知が捨てられる。構造的だが本走行では発現していない（該当のログが 0 行）。発現していないものは順位の材料にならない。引受先は起票済み（7-6） |
| 記録 §13.2 行 10 | SHIORI の要求と応答 | いいえ | 脳と話す窓のスレッドが待機中にメッセージを取り出さない。構造的だが本走行では発現しない。areka の橋渡しの作りの話で、正典の語彙の欠けではない。引受先は起票済み（7-6） |
| 記録 §13.2 行 13 | バルーンの文字 | いいえ | 話し始めに相方側バルーンの冒頭へ 1.5 行ぶんの空きが出る。出どころは上流のゴーストが空の相手側へ話者交替の改行を出すことで、areka は正典どおりに保っている。引受先は上流のリポジトリで、本リポジトリの束の順位を動かさない |
| 隔離裁定 `verification/isolation-decision.md` §4.5.1 の留保 | 無し（0 束） | いいえ | 壁時計の期限で待つ決定論テストの族が、高負荷で低確率に期限切れになりうるという留保。テストの作りの話で、正典の項目に写る先が無い（数え方: `linkage.md` の 67 の囲みの `foundation` を読み、テストの作りを扱う基盤が 1 つも無いことを確かめた）。引受先は既存の台帳 spec `areka-P0-zorder-chain-residue`（W14）の A 群である |
| R8.3 後半の未履行 | 無し（0 束） | いいえ | 2 つの修正の後に一周走行を採り直していないという手続きの未履行。開発者が 2026-09-11 に受容と裁定して確定しており、判定結果を書き換えない。走行の手続きの話で、正典の項目に写る先が無い（数え方は 1 つ上の行と同じ）。引受先は無い |

### 7-5. 結論（要件 9.3）

**20 項目と持ち越し 8 行のいずれも順位を動かす根拠にならなかった。**

確かめた項目番号（20 件）: 項目 1・項目 2・項目 3・項目 4・項目 5・項目 6・項目 7・項目 8・
項目 9・項目 10・項目 11・項目 12・項目 13・項目 14・項目 15・項目 16・項目 17・項目 18・
項目 19・項目 20。

確かめた持ち越し行の見出し（8 件）: 記録 §13.1 行 3・記録 §13.1 行 1・症状 E・
記録 §13.2 行 4・記録 §13.2 行 10・記録 §13.2 行 13・
隔離裁定 `verification/isolation-decision.md` §4.5.1 の留保・R8.3 後半の未履行。

順位を動かすと決めた行は **0 行**である（数え方: 7-3 と 7-4 の「順位を動かすか」の欄で
「動かす」と書いた行を数えた。28 行すべてが「いいえ」である）。よって「変更前・変更後・
理由」を書く行も **0 行**であり（要件 9.2）、3-3 の囲みに足す第二段の印——`override` の
`kind` が `second-stage` の行——も **0 行**である。3-4 が「第二段の改訂の印はまだ 0 行」と
書いているのは、この判断の後も変わらない。

**1 行も動かなかった理由。** 順位は 4 つの根拠——壊れ方・伺からしさのテーマ・影響する既存
資産の広さ・依存基盤の共有度——だけから決まり（要件 6.1）、その値の出どころは
`linkage.md` の `breakage`・`themes`・`foundation` と、5-1 のテンプレート辞書の語彙で
ある（3-1）。実機の一周が動かしうるのは `breakage` だけで、その値は束の構成項目がすべて
「実装済み」になったときにだけ変わる。走行は 20 項目すべてを合格にしたが、それは実装済みの
振る舞いが実装済みのまま確かめられたということで、台帳のどの項目の状態も変えていない。
持ち越しの側は 8 行のうち 5 行（症状 E・記録 §13.2 行 10・記録 §13.2 行 13・
隔離裁定 `verification/isolation-decision.md` §4.5.1 の留保・R8.3 後半の未履行）が
areka の内部の作りと上流のゴーストの事情で、正典の語彙の状態を動かさない。残る 3 行は、
1 行が未観測（記録 §13.1 行 3）、1 行が本走行で発現していない（記録 §13.2 行 4）、
1 行が判定に載せないと裁定済み（記録 §13.1 行 1）である。5 ＋ 3 ＝ 8 で、
持ち越し 8 行を余さず数えている。

### 7-6. 既に spec が起票済みの 4 件（要件 9.4）

持ち越し 8 行のうち、M1 完成の走行を受けて 2026-09-11 に spec が起票された 4 件である。
いずれも**新たな束にせず**、次段の spec 表に載せる印を付ける。

| 持ち越し行の見出し | 引受先の spec | ウェーブ | 次段の spec 表に載せる |
| --- | --- | --- | --- |
| 症状 E | `areka-P0-present-gpu-transform-scale` | W13 | ○ |
| 記録 §13.2 行 4 | `areka-P0-kanade-boot-talkdone-drop` | W13 | ○ |
| 記録 §13.2 行 10 | `areka-P0-host32-window-thread-pump` | W13 | ○ |
| 記録 §13.1 行 1 | `areka-P0-dpi-transition-two-tick-bounce` | W14 | ○ |

「次段の spec 表」は `roadmap-draft.md`「既存 brief の位置づけ」の spec 表（段 5 が書く）で
ある。ウェーブの欄は正本のロードマップの spec 台帳から引いた。

**新たに作った束は 0 束である**（数え方: 上の 4 件と残る 4 行のどれについても
`linkage.md` に囲みを 1 つも足していない。段 4 は `linkage.md` を触らない）。理由: 4 件は
いずれも areka の内部の作りを直す仕事であって、正典の項目の集まりではない。束は台帳の項目を
束ねる器なので、束ねる項目を持たない仕事は束にならない。

引受先を持つ持ち越し行はほかに 2 行ある。⑴ 隔離裁定
`verification/isolation-decision.md` §4.5.1 の留保 の引受先は既存の台帳 spec
`areka-P0-zorder-chain-residue`（W14）の A 群で、M1 の走行で新しく起票されたものではない
ので要件 9.4 が名指す 4 件には入らない。⑵ 記録 §13.2 行 13 の引受先は上流のリポジトリで、
本リポジトリの spec ではない。どちらも新たな束にしない点は 4 件と同じである。

引受先を持たない持ち越し行は **2 行**である（記録 §13.1 行 3・R8.3 後半の未履行。数え方:
§6 の表の「引受先」の欄を 8 行とも読み、本リポジトリの spec 名も上流の宛先も書かれていない
行を数えた）。前者は次の実機一周での目視に据え置かれ、後者は開発者の受容で確定している。
4 ＋ 2 ＋ 2 ＝ 8 で、持ち越し 8 行を余さず数えている。

### 7-7. 段階 A の温度感と、一般化で壊れる項目（要件 9.5・9.6）

段階 A の温度感は **「当面 emo2 が動けばよい」** である。段階 A に置いた束は、M1 で通した
emo2 の一周（起動・発話・面の切り替え・終了）が成り立つ範囲を写したものであって、世に出て
いるゴーストが一般に動く範囲ではない。段階 A を終えても、里々製・ヤヤ製の標準テンプレート
ゴーストがそのまま動くとは言えない。この節は、その差を id で示す。

**何を「壊れる」と呼ぶか。** 5-1 の `[[template]]` の `ids`（2 本の和集合）と、段階 A に置
いた束の `members`（`linkage.md`）の和集合との積を取り、両方に現れる項目だけを見る。テンプ
レート辞書が実際に書いている語彙であって、しかも段階 A の仕事の射程に入っている項目である。
そのうち台帳の `status` が `implemented` のものは段階 A を終えた時点で動くので壊れない。残
る `absent`（未対応）・`vocabulary-only`（語彙のみ）・`degraded`（縮退）の 3 状態が、段階
A を終えた emo2 を里々製・ヤヤ製の代表 2 本へ一般化したときに壊れる項目である。3 つとも数
え方は同じで、上の積を台帳の `status` で引き直して数えた。

| 重なりの内訳 | 件数 |
| --- | ---: |
| 未対応（`absent`） | 67 |
| 語彙のみ（`vocabulary-only`） | 6 |
| 縮退（`degraded`） | 2 |
| 小計＝一般化で壊れる項目 | 75 |
| 実装済み（`implemented`）＝壊れない | 30 |
| 重なりの合計 | 105 |

残る 2 つの状態は、重なりに 1 件も現れない。テンプレートの語彙の側には別名と対象外がそれぞ
れ下の数だけあるが、いずれも段階 A の束の `members` に入っていないので積に残らない。数え方:
4 つの行とも、対象の集合を台帳の `status` で引き直して数えた。

| 重なりに残らなかった状態 | 件数 |
| --- | ---: |
| 重なりのうち別名（`alias`） | 0 |
| 重なりのうち対象外（`not-applicable`） | 0 |
| テンプレートの語彙のうち別名（段階 A に届かない） | 9 |
| テンプレートの語彙のうち対象外（段階 A に届かない） | 18 |

テンプレート 2 本のどちらが困るかも分けて数えた。1 つの項目を 2 本とも使っていることがある
ので、2 本の数の和は小計より大きい。数え方: 小計の 75 件を `[[template]]` の `ids` へ 1 本
ずつ照らし、どちらに現れるかで数えた。

| テンプレート | 壊れる項目 |
| --- | ---: |
| ポストと狛犬（里々） | 30 |
| はろーYAYAわーるど（ヤヤ） | 74 |
| うち 2 本とも使う項目 | 29 |

壊れる項目は段階 A の束すべてに散らばっているのではない。数え方: 下の 3 つの表の id を
`linkage.md` の帰属で束へ写し、3-3 の段階 A の順位表に並ぶ `bundle` の行から差し引いた。

| 束の分かれ方 | 件数 |
| --- | ---: |
| 壊れる項目を持つ段階 A の束 | 16 |
| 壊れる項目が 0 の段階 A の束 | 9 |

壊れる項目が 0 の束は「終了」「descript の転記」「バルーンのリンク」「マウスの矢印」「絵の
重ね方」「定義ファイルの文字コード」「SHIORI の要求と応答」「シェル定義の転記」「同期オブ
ジェクト」である。この 9 束は、段階 A の射程に入っていながらテンプレート 2 本が語彙を 1 つ
も書いていないか、書いている語彙がすべて実装済みかのどちらかである。

#### 未対応（67 件）

| 項目 | 題 | 使うテンプレート | 段階 A の束 |
| --- | --- | --- | --- |
| `ukadoc:descript_ghost:craftman_2c_4f5c_8005_540d:1` | `craftman,作者名` | 里々・ヤヤ | 起動と挨拶 |
| `ukadoc:descript_ghost:craftmanurl_2cURL:1` | `craftmanurl,URL` | 里々・ヤヤ | 起動と挨拶 |
| `ukadoc:descript_ghost:craftmanw_2c_4f5c_8005_540d:1` | `craftmanw,作者名` | 里々・ヤヤ | 起動と挨拶 |
| `ukadoc:descript_ghost:type_2c_7a2e_5225:1` | `type,種別` | 里々・ヤヤ | 起動と挨拶 |
| `ukadoc:descript_shell:kero.balloon.offsetx_2c_5ea7_6a19:1` | `kero.balloon.offsetx,座標` | 里々・ヤヤ | 窓の配置と重なり |
| `ukadoc:descript_shell:kero.balloon.offsety_2c_5ea7_6a19:1` | `kero.balloon.offsety,座標` | 里々・ヤヤ | 窓の配置と重なり |
| `ukadoc:descript_shell:kero.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1` | `kero.seriko.alignmenttodesktop,位置情報` | ヤヤ | 窓の配置と重なり |
| `ukadoc:descript_shell:menu.background.alignment_2c_4f4d_7f6e:1` | `menu.background.alignment,位置` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.background.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1` | `menu.background.bitmap.filename,ファイル名` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.background.font.color.b_2c_6570_5024:1` | `menu.background.font.color.b,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.background.font.color.g_2c_6570_5024:1` | `menu.background.font.color.g,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.background.font.color.r_2c_6570_5024:1` | `menu.background.font.color.r,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.disable.font.color.b:1` | `menu.disable.font.color.b` | ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.disable.font.color.g:1` | `menu.disable.font.color.g` | ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.disable.font.color.r:1` | `menu.disable.font.color.r` | ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.foreground.alignment_2c_4f4d_7f6e:1` | `menu.foreground.alignment,位置` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.foreground.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1` | `menu.foreground.bitmap.filename,ファイル名` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.foreground.font.color.b_2c_6570_5024:1` | `menu.foreground.font.color.b,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.foreground.font.color.g_2c_6570_5024:1` | `menu.foreground.font.color.g,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.foreground.font.color.r_2c_6570_5024:1` | `menu.foreground.font.color.r,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.separator.color.b_2c_6570_5024:1` | `menu.separator.color.b,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.separator.color.g_2c_6570_5024:1` | `menu.separator.color.g,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.separator.color.r_2c_6570_5024:1` | `menu.separator.color.r,数値` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.sidebar.alignment_2c_4f4d_7f6e:1` | `menu.sidebar.alignment,位置` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:menu.sidebar.bitmap.filename_2c_30d5_30a1_30a4_30eb_540d:1` | `menu.sidebar.bitmap.filename,ファイル名` | 里々・ヤヤ | メニュー |
| `ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1` | `sakura.balloon.offsetx,座標` | 里々・ヤヤ | 窓の配置と重なり |
| `ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1` | `sakura.balloon.offsety,座標` | 里々・ヤヤ | 窓の配置と重なり |
| `ukadoc:descript_shell:sakura.seriko.alignmenttodesktop_2c_4f4d_7f6e_60c5_5831:1` | `sakura.seriko.alignmenttodesktop,位置情報` | ヤヤ | 窓の配置と重なり |
| `ukadoc:list_sakura_script:_5c4:1` | `\4` | ヤヤ | 窓の配置と重なり |
| `ukadoc:list_sakura_script:_5c6:1` | `\6` | 里々・ヤヤ | 組み込みの置換語 |
| `ukadoc:list_sakura_script:_5cC:1` | `\C` | ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5c_21_5b_2a_5d:1` | `\![*]` | ヤヤ | 選択肢の目印 |
| `ukadoc:list_sakura_script:_5c_21_5bclose_2cinputbox_2cID_5d:1` | `\![close,inputbox,ID]` | ヤヤ | 入力窓とダイアログ |
| `ukadoc:list_sakura_script:_5c_21_5benter_2cpassivemode_5d:1` | `\![enter,passivemode]` | ヤヤ | 動作モードの出入り |
| `ukadoc:list_sakura_script:_5c_21_5bleave_2cpassivemode_5d:1` | `\![leave,passivemode]` | ヤヤ | 動作モードの出入り |
| `ukadoc:list_sakura_script:_5c_21_5bopen_2cinputbox_2cID_2c_8868_793a_6642_9593_2c_30c6_30ad_30b9_30c8_2c_30aa_30d7_30b7_30e7_30f3_2c..._5d:1` | `\![open,inputbox,ID,表示時間,テキスト,オプション,...]` | 里々・ヤヤ | 入力窓とダイアログ |
| `ukadoc:list_sakura_script:_5c_21_5braise_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` | `\![raise,イベント名,r0,r1,r2...]` | ヤヤ | イベントの呼び起こし |
| `ukadoc:list_sakura_script:_5c__21:1` | `\_!` | ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5c__3f:1` | `\_?` | ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` | `\_b[ファイルパス,inline,オプション,オプション...]` | ヤヤ | バルーンの付属画像 |
| `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cinline_2copaque_5d:1` | `\_b[ファイルパス,inline,opaque]` | ヤヤ | バルーンの付属画像 |
| `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._5d:1` | `\_b[ファイルパス,x,y,オプション,オプション...]` | ヤヤ | バルーンの付属画像 |
| `ukadoc:list_sakura_script:_5c_b_5b_30d5_30a1_30a4_30eb_30d1_30b9_2cx_2cy_2copaque_5d:1` | `\_b[ファイルパス,x,y,opaque]` | ヤヤ | バルーンの付属画像 |
| `ukadoc:list_sakura_script:_5c_n:1` | `\_n` | ヤヤ | バルーンの文字 |
| `ukadoc:list_sakura_script:_5c_q:1` | `\_q` | 里々・ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5c_s_5bID1_2cID2_2cID3..._5d:1` | `\_s[ID1,ID2,ID3...]` | ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5cf_5bbold_2c_30d1_30e9_30e1_30fc_30bf_5d:1` | `\f[bold,パラメータ]` | ヤヤ | バルーンの文字 |
| `ukadoc:list_sakura_script:_5cf_5bcolor_2c_8272_6307_5b9a_5d:1` | `\f[color,色指定]` | ヤヤ | バルーンの文字 |
| `ukadoc:list_sakura_script:_5cf_5bheight_2c_6570_5024_5d:1` | `\f[height,数値]` | ヤヤ | バルーンの文字 |
| `ukadoc:list_sakura_script:_5ci_5bID_2cwait_5d:1` | `\i[ID,wait]` | ヤヤ | サーフェスアニメーション |
| `ukadoc:list_sakura_script:_5ci_5bID_756a_53f7_5d:1` | `\i[ID番号]` | ヤヤ | サーフェスアニメーション |
| `ukadoc:list_sakura_script:_5ct:1` | `\t` | ヤヤ | 会話 |
| `ukadoc:list_shiori_event:OnAITalk:1` | `OnAITalk` | ヤヤ | 自発発話 |
| `ukadoc:list_shiori_event:OnAnchorSelect:1` | `OnAnchorSelect` | ヤヤ | 会話 |
| `ukadoc:list_shiori_event:OnKeyPress:1` | `OnKeyPress` | 里々・ヤヤ | キーとゲームパッド |
| `ukadoc:list_shiori_event:OnMinuteChange:1` | `OnMinuteChange` | 里々・ヤヤ | 自発発話 |
| `ukadoc:list_shiori_event:OnMouseClick:1` | `OnMouseClick` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnMouseDown:1` | `OnMouseDown` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnMouseDragEnd:1` | `OnMouseDragEnd` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnMouseDragStart:1` | `OnMouseDragStart` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnMouseGesture:1` | `OnMouseGesture` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnMouseUp:1` | `OnMouseUp` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnMouseWheel:1` | `OnMouseWheel` | ヤヤ | 撫で |
| `ukadoc:list_shiori_event:OnNotifyUserInfo:1` | `OnNotifyUserInfo` | ヤヤ | 名前の記憶 |
| `ukadoc:list_shiori_event:OnSurfaceChange:1` | `OnSurfaceChange` | ヤヤ | サーフェスアニメーション |
| `ukadoc:list_shiori_event:OnSurfaceRestore:1` | `OnSurfaceRestore` | ヤヤ | サーフェスアニメーション |
| `ukadoc:list_shiori_event:OnUserInput:1` | `OnUserInput` | 里々 | 入力窓とダイアログ |

#### 語彙のみ（6 件）

| 項目 | 題 | 使うテンプレート | 段階 A の束 |
| --- | --- | --- | --- |
| `ukadoc:descript_ghost:name_2c_30b4_30fc_30b9_30c8_540d:1` | `name,ゴースト名` | 里々・ヤヤ | 名前の記憶 |
| `ukadoc:list_sakura_script:_5c_21_5bset_2cballoontimeout_2c_6642_9593_5d:1` | `\![set,balloontimeout,時間]` | ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5c_21_5bset_2cchoicetimeout_2c_6642_9593_5d:1` | `\![set,choicetimeout,時間]` | ヤヤ | 会話 |
| `ukadoc:list_shiori_event:OnBalloonBreak:1` | `OnBalloonBreak` | ヤヤ | 会話 |
| `ukadoc:list_shiori_event:OnBalloonClose:1` | `OnBalloonClose` | ヤヤ | 会話 |
| `ukadoc:list_shiori_event:OnBalloonTimeout:1` | `OnBalloonTimeout` | ヤヤ | 会話 |

#### 縮退（2 件）

| 項目 | 題 | 使うテンプレート | 段階 A の束 |
| --- | --- | --- | --- |
| `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cID1_2cID2_2cID3..._5d:1` | `\q[タイトル,ID1,ID2,ID3...]` | ヤヤ | 会話 |
| `ukadoc:list_sakura_script:_5cq_5b_30bf_30a4_30c8_30eb_2cscript_3a_5b9f_884c_5185_5bb9_5d:1` | `\q[タイトル,script:実行内容]` | ヤヤ | 会話 |

#### 参照元をいつ切り替えるか（要件 9.6）

上の 2 本は代表として選んだテンプレートであって、動かす約束をした相手ではない。**外部から
「このゴーストを動かして」という要望が来た時点で、参照元と検証対象をそのゴーストに切り替え
る。** 切り替える先は 2 つある——順位の根拠 ⑶「影響する既存資産の広さ」を数えるときに読
む辞書（5-1 の `[[template]]` の `files`）と、段階 A の出口で「動いた」と言うために動かす
相手である。切り替えたときは 5-1 を書き直し、この節の重なりを数え直す。

**現時点で切り替え先は 0 件である。** 数え方: 本 spec の着手（2026-09-11）からこの節を書い
た 2026-09-13 までに開発者から名指しで受け取った「このゴーストを動かして」の要望を数えた。
候補があるとすれば自作の「どっとさくら」だが、これは外部から来た要望ではなく自分で選ぶ相手
なので、切り替え先には数えない。

| 数えたもの | 件数 |
| --- | ---: |
| 外部から名指しされたゴースト（＝切り替え先） | 0 |
| 自作の候補（「どっとさくら」） | 1 |

切り替え先が 0 である間は、上の代表 2 本を参照元のまま置く。

<!-- 段 4 の残り: タスク 5.3（順位表への反映）が続く。5.3 が書き換える順位の行は
     7-5 のとおり 0 行である。 -->

## 8. 是正候補への参照

<!-- 段 5（タスク 6.2）で書く: 台帳の id と brief の所有宣言が食い違うものの一覧への参照を
     置く。一覧の本体は `roadmap-draft.md` にある。 -->
