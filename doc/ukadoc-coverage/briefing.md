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

**この節の順位は第一段の草案である**（要件 9.1）。第一段は台帳の状態・`linkage.md` の欄・
標準テンプレート辞書の語彙だけを材料にして数えたものである。M1 完成の実機の記録を材料に
この草案のどの行を動かすかを 1 行ずつ決めた第二段の記録は 7（第二段の改訂記録）にある。

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
`override` を持つ行を数えた。第二段の改訂（要件 9）の印は 0 行であり、これは 7-5 の判断で
確定した値である）。

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
implemented = 9
vocabulary_only = 1
degraded = 0
absent = 64
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
implemented = 5
vocabulary_only = 57
degraded = 4
absent = 67
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
**27 本**）。完了済み spec は `.kiro/specs/completed/` の直下にあるもの（同じ日に既定ブランチを
取り込んだ後で数え直して **176 本**。下の規則 ⑴〜⑷ を当てたときは 174 本だった）。

**規則 ⑴——進行中 spec 宛ての宛先は保つ。** 該当は **372 件・13 spec**。変えた件数は **0** で
ある（1 件も空にせず、1 件も書き換えていない）。

**規則 ⑵——完了済み spec 宛ての宛先は状態で分ける。** 該当は **75 件・16 spec**。状態が実装済み
か縮退の **41 件**（実装済み 33・縮退 8）は「その項目を実装した spec の記録」として保った。状態が
未対応か語彙のみの **34 件**（未対応 32・語彙のみ 2）は宛先を空にし、理由を備考に書いた。変えた
件数は **34** である。宛先だった 16 spec のうち 2 本は持っていた項目がすべて未対応だったので全件
が空になり、下の列挙から落ちる（`areka-P0-window-placement` 16 件・`areka-P0-emo-atlas` 4 件）。
残るのは **14 spec** である。

**整理を終えた後に完了した spec が 1 本ある。** 既定ブランチを取り込んだところ、
`areka-P0-charset-canon` が完了して封じる場所へ移っていた。この spec を宛先に持つ項目は **5 件**で、
状態は 5 件とも実装済みである。規則 ⑵ を当て直しても宛先を空にする件数は **0** なので、台帳は
1 行も変わらない。下の列挙に足さないのは、その列挙が上の整理で保った 41 件の記録だからである。
この spec の名前は `roadmap-draft.md` の spec 表が持っており、宛先の名前がその表か下の列挙の
どちらかに必ず在ることは機械が見張っている。

**規則 ⑶——brief がまだ無い候補 spec の名前を宛先に書かない。** 整理の前も後も、非空の宛先の
うち進行中 27 本にも完了済み spec にも当たらない名前は **0 件**であった。変えた件数は **0** で
ある。この 0 は書き落としではなく、整理の前に上の手順が出した非空の名前 29 通り（進行中 13・
完了済み 16）を 2 つの集まりに 1 つずつ当てて、どれも外れなかったことを数えた 0 である。整理の
後は 27 通りになり（規則 ⑵ で 2 本が全件空になった分）、同じく外れは無い。候補 spec への割り
当ては `roadmap-draft.md` の側に持つ。

**規則 ⑷——変えた件数を書く。** 合計 **34 件**（⑴ **0** 件・⑵ **34** 件・⑶ **0** 件）。2026-09-13 に
上の手順で数え直すと、非空の宛先は **416 件**、空が **1,333 件**で、和は 1,749 である。整理を終えた
直後は 413 件と 1,336 件だった。差の **3 件**の内訳は、6 の裁定で足した **2 件**と、既定ブランチを
取り込んだときに `areka-P0-charset-canon` の宛先が 1 件増えた分 **1 件**である。

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

**拾ったのは 37 件、37 件すべてに処分と理由が付いている。開発者の裁定候補は 12 件。**
処分の内訳は 採用 26・却下 9・開発者の裁定候補 2 である。裁定候補 12 件のうち 10 件は
2-5 に前置きごと書いてあるので、ここでは登記だけを行って同じ話を 2 度書かない。残る
2 件は 6-3 にある。

### 6-1. 何をどう拾ったか

拾い方は 2 段である。⑴ 調査 4 本のブリーフィングと、完了して封じた調査 5 本の作業記録
（`tasks.md`）を 4 つの語で引く。⑵ 当たった行を 1 行ずつ読み、宛先で仕分ける。

**引いた 4 つの語は「統合担当」「裁定案」「是正候補」「申し送り」である**（要件 8.2）。
文書ごとの当たり数は次のとおり（2026-09-13 にこの木で数え直した実測）。

| 引いた文書 | 統合担当 | 裁定案 | 是正候補 | 申し送り | 当たった行（重なりを畳んだ数） |
| --- | ---: | ---: | ---: | ---: | ---: |
| `doc/ukadoc-coverage/briefing-shiori.md` | 5 | 0 | 0 | 3 | 8 |
| `doc/ukadoc-coverage/briefing-assets.md` | 2 | 0 | 3 | 0 | 5 |
| `doc/ukadoc-coverage/briefing-sakura-script.md` | 12 | 3 | 11 | 2 | 27 |
| `doc/ukadoc-coverage/briefing-property.md` | 2 | 2 | 4 | 1 | 9 |
| `.kiro/specs/completed/areka-P0-ukadoc-survey-shiori/tasks.md` | 14 | 0 | 0 | 31 | 37 |
| `.kiro/specs/completed/areka-P0-ukadoc-survey-assets/tasks.md` | 6 | 0 | 19 | 14 | 36 |
| `.kiro/specs/completed/areka-P0-ukadoc-survey-sakura-script/tasks.md` | 5 | 3 | 16 | 13 | 31 |
| `.kiro/specs/completed/areka-P0-ukadoc-survey-property/tasks.md` | 4 | 3 | 6 | 2 | 13 |
| `.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/tasks.md` | 1 | 0 | 0 | 24 | 25 |
| **合計** | **51** | **11** | **59** | **90** | **191** |

**当たった 191 行と、拾った 37 件は別の数である。** 1 つの申し送りが、見出しと本文と作業記録の
言及に分かれて何行にも散るからで、逆に 1 行に 2 件が並ぶこともある。件数は行を読んで数えた。

**0 が出た 9 か所は、引き方が壊れていないかを確かめた。** 0 になったのは、`briefing-shiori.md`
の「裁定案」と「是正候補」、`briefing-assets.md` の「裁定案」と「申し送り」、shiori の作業記録の
「裁定案」と「是正候補」、assets の作業記録の「裁定案」、toolkit の作業記録の「裁定案」と
「是正候補」である。9 か所それぞれについて、その文書の写しへその語を 1 行だけ足すと当たりが
1 件に増えることを確かめた（9 か所とも 0 → 1）。**引き方そのものは他の文書で 1 以上を返して
いるので、この 0 は「その語をその文書が使っていない」という意味である。**

**宛先の仕分け**（1 件 ＝ 1 つの申し送り）。

| 宛先 | 件数 | 扱い |
| --- | ---: | --- |
| 統合担当（本 spec）宛て | 36 | 6-2 に 1 行ずつ |
| 4 つの語では当たらないが本 spec で処分するもの | 1 | 6-2 の 申-37 |
| 既存の brief と `doc/COMPAT_ARCHITECTURE.md` の持ち主宛て | 18 | 8 節が指す是正候補の表へ回す |
| 調査 spec 自身の要件・設計の直し | 12 | 群として却下（下の理由） |
| **合計** | **67** | |

**1 行ずつ処分するのは、宛先が本 spec である 37 件だけである。** 残る 30 件は行き先が別に
あるので、群として次のように扱う。

- **既存の brief と対応表の持ち主宛て 18 件**（`briefing-assets.md`「隣接 spec の是正候補」⑴〜⑹、
  `briefing-sakura-script.md` の是正候補 1〜4、`briefing-property.md`「⑸ 既存 brief への是正候補」
  の是正 1〜7 と「⑵」節が brief の担当者へ回している名前の書き直し 1 件）。要件 12.3 が既存
  brief と調査 4 本のブリーフィングの編集を禁じているので、本 spec は 1 文字も直さない。
  行き先は 8 節が指す是正候補の 3 列表である。
- **調査 spec 自身の要件・設計の直し 12 件**（`briefing-assets.md`「上流の決まりと本 spec 自身の
  文書へ回すもの」⑺〜⒂ の 9 件、`briefing-sakura-script.md` の是正候補 5 と 7、
  `briefing-shiori.md`「是正の候補」4）。**群として却下する。** 相手はすべて完了して
  `completed/` へ封じた spec の要件・設計であり、直しても走る検査が無く、読む人も居ない。
  中身は各ブリーフィングと台帳の備考に残るので失われない。
- `briefing-sakura-script.md`「すでに解消していて、是正候補として出さないもの」の 6 件は、
  調査の側が取り下げ済みと明記しているので拾っていない（二重の起票を防ぐための記録である）。

### 6-2. 処分の一覧

処分の語は 採用・却下・裁定候補 の 3 つだけを使う。反映先は台帳の項目 id か、新規 3 文書
（`linkage.md`・本文書・`roadmap-draft.md`）の節名で示す（要件 8.4）。

#### 出典 `briefing-shiori.md` と shiori の作業記録

| # | 出典（文書と節） | 要約 | 処分 | 理由 | 反映先 |
| --- | --- | --- | --- | --- | --- |
| 申-01 | `briefing-shiori.md`「次に読む人への申し送り」⑴（同じ話が shiori 作業記録 5.3） | 繋がりはイベントの行に書き、相手としてタグや設定キーを指す向きでそろえてある。読むときはこの向きを前提にすること | 採用 | 段 1 で関連を足すときに実際にこの向きに従った。向きを取り違えると同じ 1 本を二重に数える | `linkage.md`「補修した関連」節 |
| 申-02 | shiori 作業記録 5.1 | 「掛け合い」と「交わり」の境界。利用者からの差し込みを受けるイベントの一族は交わり寄りにも読めるが、既定のまま掛け合いに置いた。境界の見直しは統合側で | 却下 | 要件 12.2 が台帳への接触を優先度と宛先の 2 欄に限り、要件 12.3 がテーマの定義文書の編集を禁じている。本 spec にはテーマを動かす手が無い | — |
| 申-03 | shiori 作業記録 5.2 ⑴ | `ukadoc:list_shiori_event:OnArchiveViewerOpen:1` は名前で見ると別の群に当たるが、テーマを見て触れ合いの側へ置いた。名前を優先する別の項目とは逆向きの決め方である | 採用 | テーマを優先する決め方を追認する。順位の 4 つの根拠にテーマの個数は入るが項目の名前は入らない（3-1）ので、名前で置き直しても順位は動かない。答えで作業が変わらないので自分で決めた | `ukadoc:list_shiori_event:OnArchiveViewerOpen:1`（`linkage.md`「撫で（段階 A 相当）」の構成 id） |
| 申-04 | shiori 作業記録 5.2 の追記 | 群 4 と群 8 でテーマの扱いが逆向きである。片方は群の既定値が無条件に効き、もう片方はテーマから決まる。どちらの規約に寄せるかを判断すること | 採用 | 本 spec は優先度を群の既定値からではなく `linkage.md` の束の帰属と 3-3 の順位表から導き直した。4 ドメイン共通の 1 つの作り方になったので、群ごとの規約の食い違いは残っていない。答えで作業が変わらないので自分で決めた | 3-2「順位の振り方」・5-2 の書き戻し前後の分布 |
| 申-05 | `briefing-shiori.md`「是正の候補」2 | ゴースト同士のやり取りを本文に書いた拡張イベントの数を、件数の根拠に使わないこと | 採用 | この数を根拠に使っていない。本文書にこの数を根拠として書いた箇所は 0 か所である（文書を通して確かめた） | 本文書（該当なしを確かめた） |
| 申-06 | `briefing-shiori.md`「次に読む人への申し送り」⑵ と「是正の候補」3 | 正典の本文が、対になるタグの名前を実在しない綴りで書いている。実在する綴りはさくらスクリプト一覧の側にある | 却下 | 誤っているのは正典の側で、直せるのは ukadoc の書き手だけである。areka の側は既に実在する綴りを繋がりの相手にしており、食い違いは台帳の該当 3 行の備考に記録済みなので失われない | — |
| 申-07 | `briefing-shiori.md`「次に読む人への申し送り」⑷ | 群の索引を直すときは正本を先に直し、台帳の冒頭の写しは節ごと貼り直すこと。写しと正本の食い違いは機械の検査に出ない | 採用 | 手順に従い、本 spec は写しを部分的に直していない。冒頭の写しが事実でなくなっている点は 申-37 で別に処分する | 申-37 |
| 申-08 | `briefing-shiori.md`「是正の候補」1 | 調査 spec の brief の記載 26 行が正典と食い違う（実在しないイベント名など） | 却下 | 相手は完了して封じた調査 spec 自身の brief で、引受先が無い。実測の全数は同 spec の要件の付録に残っている | — |
| 申-09 | shiori 作業記録 7.1 と 7.2 の引き継ぎ ⒜〜⒠ | 全体報告が古い。直すのは作り直しの副手続き 1 本で、手書きは禁止。走らせる時機は調査 4 本が既定ブランチへ合流した後。この古さは常時の検査では赤にならない | 採用 | 4 本とも 2026-09-06 までに合流したので時機は満ちている。段 1 で作り直し、常時の検査へ入れた。いまは台帳から作り直した本文と 1 文字でも違えば赤になる | 5 節「全体報告の新しさは常時の検査が判定する」 |

#### 出典 上流の道具宛てだったもの（宛先を本 spec に読み替えた 5 件）

shiori の作業記録の末尾が、上流の道具の spec は封じてあって引受先にできないので、宛先を
統合担当に読み替えるようにと書いている。5 件をそのまま受け取る。

| # | 出典（文書と節） | 要約 | 処分 | 理由 | 反映先 |
| --- | --- | --- | --- | --- | --- |
| 申-10 | shiori 作業記録 5.4 の「上流へ送る材料」 | カタログが版番号を取り出す規則が、区切りの入った 2 桁の版を落とす。ある 1 項目の登場の版が本文に書いてあるのに拾われない | 却下 | カタログは機械生成で、要件 12.3 が編集を禁じている。規則を直すとカタログの全項目を作り直すことになり、4 台帳の照合がすべてやり直しになる。効くのは 1 項目の登場の版だけで、順位の 4 つの根拠に版番号は入らない（3-1） | — |
| 申-11 | shiori 作業記録 6.5 の発掘と 7.2 の引き継ぎ | 証拠の無い実装済みを見つける検査の摂動テストが、錨にした項目へ証拠が付いたせいで効かなくなる。直し方は錨の選び直しではなく、摂動の中で錨の証拠を剥がすこと | 採用 | 直し方どおりに直っていることを確かめた。`crates/ukadoc-survey/tests/consistency/checks.rs` の `implemented_without_evidence_turns_red_and_evidence_clears_it` は、錨の状態が実装済みであることを先に確かめ、証拠を索引から剥がしてから所見を数える形になっている。常時の検査は緑である | `crates/ukadoc-survey/tests/consistency/checks.rs` の同名のテスト |
| 申-12 | shiori 作業記録 6.2 の「2 つの罠」⑴ | 台帳へ項目を差し込む処理は後ろの見出しの直前に入れるが、コメントは直前の塊に属する。だから見出しの真上に書いた説明が、差し込みの後は別項目の説明に見えるようになる。黙って起きて赤にならない | 採用 | 本 spec はこの処理を 1 度も走らせていない。書き戻しは塊の中の優先度の行 1 つだけを差し替える形で、前置きも他の欄も備考もバイト列のまま写す | 5-2 の書き戻しの記録 |
| 申-13 | `briefing-shiori.md`「次に読む人への申し送り」⑶ と shiori 作業記録 6.2 の罠 ⑵ | 同じ処理は読み書きで行末を書き換えるので、1 度走らせるだけで台帳の全行の行末が変わる。差分の既定の表示には出ないが、バイト単位で照らす検査は必ず割れる | 採用 | 同上。本 spec が走らせたのは優先度の書き戻しと検査だけである。走らせた副手続きは 5-2 に記録がある | 5-2 の書き戻しの記録 |
| 申-14 | shiori 作業記録 3.8 の「上流へ送る材料」⑴ | 同じページの正典 URL が 2 か所に現れる。重複した証拠をどう数えるかが決まっていない | 採用 | 数え方が効いていたのは全体報告の末尾に載っていた証拠の件数の表だけで、本 spec がその表を報告から外した（5 節）。証拠は専用の副手続きの出力で読む形になり、報告の本文が重複の数え方に左右されなくなった | 5 節「全体報告の新しさは常時の検査が判定する」 |

#### 出典 `briefing-assets.md` と assets の作業記録

| # | 出典（文書と節） | 要約 | 処分 | 理由 | 反映先 |
| --- | --- | --- | --- | --- | --- |
| 申-15 | assets 作業記録 4.3 の警告 | この調査が付けた段階の中身が、ロードマップに登記された A〜E の名前と合っていない。登記の名前を保つなら統合担当の付け直しが要る | 採用 | 付け直した。段階は 2-1 の定義と 2-3 の規則で `linkage.md` の束から決め直し、優先度を 4 台帳へ書き戻した。この調査が使っていた「順位を 5 つに等分する」作り方は残っていない | 2-1・2-3・3-3・5-2 の書き戻し前後の分布 |
| 申-16 | `briefing-assets.md`「上流の決まりと本 spec 自身の文書へ回すもの」⒃ | 優先度の作り方が成果物のどこにも書いていないので、統合担当が同じ順位を作り直せない。あわせて、この台帳の数値は段階をまたいだ通し番号で、上流の説明書の言う「同じ段階の中での並び」と違う | 採用 | 作り方を 3-1（4 つの根拠をどこで引くか）と 3-2（順位の振り方）に書いた。数値は要件 7.1 のとおり段階の中で 1 から通しの密な順位に統一したので、段階をまたぐ通し番号は消えた | 3-1・3-2 |
| 申-17 | `briefing-assets.md`「引き受け先が決まっていないもの」 | シェルの貼り付きの欄を、近くの spec がどれも自分のものだと書いていない | 採用 | 束へ入れた。`ukadoc:descript_shell:seriko.sticky-window_2c_30b9_30b3_30fc_30d7ID_2c_30b9_30b3_30fc_30d7ID_2c...:1` は `linkage.md`「窓の配置と重なり」の構成 id である。引受先の spec 名は要件 7.4 ⑶ により台帳の宛先へは書かず、候補 spec の側（`roadmap-draft.md`）に持つ | `linkage.md`「窓の配置と重なり」 |
| 申-18 | 同上 | バルーンのずらし量は横と縦の両方が書かれたときだけ効き、片方だけの宣言は何も言わずに落ちる。この落ち方を引き受ける行がどの表にも無い | 採用 | 同上。`ukadoc:descript_shell:sakura.balloon.offsetx_2c_5ea7_6a19:1` と `ukadoc:descript_shell:sakura.balloon.offsety_2c_5ea7_6a19:1`（相方側の 2 件も同じ）は `linkage.md`「窓の配置と重なり」の構成 id である | `linkage.md`「窓の配置と重なり」 |
| 申-19 | 同上 | バルーンの左右の寄せは実装済みだが宛先が空で、値を読んでいる spec が自分のものだと書いていない | 採用 | 同上。`ukadoc:descript_shell:sakura.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1` と `ukadoc:descript_shell:kero.balloon.alignment_2c_4f4d_7f6e_60c5_5831:1` は `linkage.md`「窓の配置と重なり」の構成 id である | `linkage.md`「窓の配置と重なり」 |
| 申-20 | 同上 | 書体の欄のうち、通信欄・カウンタ・SSTP メッセージの 14 件を自分のものと書いている spec が 1 本も無い | 採用 | 同上。14 件は `linkage.md` の名前付き束か単独項目のいずれかに入っている。対象 4 状態の全項目がちょうど 1 つに属することは判定 ⑶ が全数で見るので、この 14 件だけが漏れることはない | `linkage.md` の名前付き束と単独項目（判定 ⑶） |

#### 出典 `briefing-sakura-script.md` と sakura-script の作業記録

| # | 出典（文書と節） | 要約 | 処分 | 理由 | 反映先 |
| --- | --- | --- | --- | --- | --- |
| 申-21 | `briefing-sakura-script.md`「裁定待ちの 2 件」 | 別のイベントが返す台詞をその場に埋め込むタグを、2 本の brief が主張している。名指しは一方向で分担が決まっていない。調査側の案は、受理の一覧と実行時の置き換えを別の spec に分ける | 採用 | 要件 8.6 が本 spec に統合担当としての裁定を課している。裁定の中身とその理由は、該当 id の宛先と備考へ書き戻すのと同じ回にこの表の下へ足す | `ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1` の宛先と備考 |
| 申-22 | 同上 | 窓を動かすタグを、2 本の brief が主張している。互いに相手を知らない。調査側の案は引数ごとに分ける | 採用 | 同上 | `ukadoc:list_sakura_script:_5c_21_5bmove_5d:1` の宛先と備考 |
| 申-23 | `briefing-sakura-script.md`「⑷ 担当なし・裁定待ちの一覧」 | 対応表が「所有先は未定で統合担当の一覧で裁定する」と明言している 8 件 | 採用 | 8 件とも `linkage.md` の束か単独項目へ入れた。宛先の欄は空のままにする——要件 7.4 ⑶ が brief のまだ無い候補 spec の名前を宛先に書くことを禁じており、引受先は `roadmap-draft.md` の候補 spec の側に持つ | `ukadoc:list_sakura_script:_5c__21:1`・`ukadoc:list_sakura_script:_5c__2b:1`・`ukadoc:list_sakura_script:_5c__3f:1`・`ukadoc:list_sakura_script:_5c__c:1`・`ukadoc:list_sakura_script:_5c__q_5bID_2c..._5d:1`・`ukadoc:list_sakura_script:_5c__t:1`・`ukadoc:list_sakura_script:_5c__v_5b_30aa_30d7_30b7_30e7_30f3_5d:1`・`ukadoc:list_sakura_script:_5c_n:1` |
| 申-24 | `briefing-sakura-script.md`「⑺ カタログとの件数の差」の注記 | 上流の仕様書が「編集しない」と名指しする文書のうち、束の帰属を書く文書がこの木に実在しない | 採用 | 本 spec が作った。`linkage.md` が束の帰属の正本であることは同文書の冒頭の規律に書いてある | `linkage.md` |
| 申-25 | `briefing-sakura-script.md`「テーマが 0 件の 2 つについて」（同じ話が sakura-script 作業記録 3.4 の実測） | 「触れ合い」と「記憶」がこのドメインに 1 件も付いていないのは取りこぼしではなく構造的である。撫でに応える定義は絵の側、覚えているのは SHIORI の側にあり、さくらスクリプトは結果を画面へ出す言語だから | 採用 | 取りこぼしとして扱っていない。本文書はドメイン別のテーマの件数を 1 つも書かず、テーマ別の分布は 5 節が置き場を指すだけである。順位の根拠に使うのは束ごとのテーマの集合（3-1）で、ドメイン別の 0 は根拠に入らない | 5 節「根拠表への参照」・3-1 |
| 申-26 | `briefing-sakura-script.md`「次に読む人への申し送り」⑴ | 調査 spec の完成条件の 1 つが、文字どおりには最初から満たせない。読み替えて確かめたが、文言をどう直すかは裁定事項 | 却下 | 相手は完了して封じた調査 spec 自身の完成条件で、引受先が無い。読み替えとその理由は同 spec のブリーフィングに残っている | — |
| 申-27 | `briefing-sakura-script.md`「次に読む人への申し送り」⑵ | 調査 spec の仕様書が挙げる件数は、3 つの条件をすべて課したときにだけ再現できる。数え方を書かないと誰も再現できない | 却下 | 同上。あわせて、本文書はこの数を 1 度も使っていない | — |
| 申-28 | `briefing-sakura-script.md`「次に読む人への申し送り」⑶ | 上流の道具の摂動テストが的にできる項目は、台帳が育つほど痩せる。並走する調査が関連を書き足すといちばん早く痩せ、0 件になったら試験の側だけでは直せない | 採用 | 痩せる原因だった並走はもう無い（調査 4 本とも完了済み）。本 spec が段 1 で足した関連 3 本が的に当たっていないことは、常時の検査が緑であることで分かる——当たっていれば所見が 2 件になって赤になる | `cargo test -p ukadoc-survey`（緑） |
| 申-29 | `briefing-sakura-script.md`「次に読む人への申し送り」⑷ | テストは最初の失敗で打ち切らない指定を付けて回すこと。付けないと走らなかった群が前後の比較の両側から同じように抜けて、数のつじつまが合ってしまう | 採用 | 本 spec の検証は crate を丸ごと 1 コマンドで走らせ、失敗しても残りが走る形で数を読む。前後を比べる場面（書き戻しの前後の分布）では、比べる相手を台帳から数え直している | 5-2 の書き戻し前後の分布 |
| 申-30 | `briefing-sakura-script.md`「次に読む人への申し送り」⑸ | 上流の道具が持つ「実データでは 0 件」の主張のうち 1 つは、対象が検査へ届く前に消えているので常に成り立つ。嘘ではないが検査が働いた証拠にはならない | 採用 | 同じ形にしないため、本 spec の判定にはすべて母数の下限を置いた（要件 11.4）。対象が 0 件になったら下限が赤になるので、恒真の緑は出ない | 判定の母数の下限（要件 11.4） |
| 申-31 | `briefing-sakura-script.md`「次に読む人への申し送り」⑹ | 並走する別の spec の着地が、一度確定した判定を黙って古びさせた。取り直すべきなのは参照の綴りではなく、参照から導いた結論のほうである | 採用 | 本文書と `linkage.md` の主張のうち台帳とカタログから決まるものは、すべて機械が数え直す形にした（判定 ⑴〜⑹）。綴りが生きたまま中身が変わっても、数え直しの側が赤になる。全体報告も常時の検査へ入れた（5 節） | 判定 ⑴〜⑹・5 節 |
| 申-32 | `briefing-sakura-script.md`「是正候補 6」（5 行） | 上流の道具と調査の材料への案。報告の行末・常設の検査が一方向しか見ていないこと・走査の台本が主題を途中で切ること・台帳の備考の札と言い回しの揺れ | 却下 | 5 行とも本 spec の境界の外にある。報告の行末の差は版管理の上では差分にならない（段 1〜4 で報告 5 本を作り直したときに実際に起きていない）。検査を足す案の相手は対応表で、要件 12.2 が道具への接触を本 spec の判定と書き戻しに限っている。走査の台本は封じた調査 spec の作業用の道具である。備考の札と言い回しは台帳の備考の話で、要件 12.2 が接触を優先度と宛先の 2 欄に限っている | — |

#### 裁定待ちだった 2 件の裁定（申-21・申-22）

**この 2 件は本 spec が統合担当として決めた**（要件 8.6）。どちらも 2 本の brief が同じタグを
主張していて、分担が決まっていなかったものである。決めた結果は台帳の該当 id の宛先と備考にも
書いた。

**申-21——別のイベントが返す台詞をその場に埋め込むタグ**
（`ukadoc:list_sakura_script:_5c_21_5bembed_2c_30a4_30d9_30f3_30c8_540d_2cr0_2cr1_2cr2..._5d:1`）

- **決めたこと**: 調査側の案をそのまま採る。台本を読む側でこのタグの綴りを受け付ける一覧は
  `areka-P0-sakura-time-directives` が持ち、返ってきた結果でタグを置き換える実行時の経路は
  `areka-P0-property-query-channels` が持つ。
- **この項目は 2 本の着地を要する。宛先の欄 1 つには収まらない。** 根拠は 2 つある。⑴
  `doc/COMPAT_ARCHITECTURE.md` には、台本を組み立てる側が時間の指令として受け付ける綴りの
  一覧を載せた行がある。その行はこのタグの綴りを逐語で並べたうえで、**M1 では綴りを受け取って
  覚えるだけで意味は実装しない**と定め、意味を実装する仕事を `areka-P0-sakura-time-directives`
  へ申し送っている。**受け付ける側の作業は残っている。** ⑵
  `areka-P0-sakura-time-directives` の brief は、自分が扱う綴りの一覧の中でこのタグを「タグ
  全体が SHIORI の返答で置き換わってそのまま続く＝台本を途中で分けて、進み方を組み直す必要が
  ある」と分類し、自分の担当範囲として「この一覧の綴りを台本を組み立てる側で解釈して実際の
  指令に落とすことと、決まったとおりの結果になるかを確かめる検査」を挙げている。一方
  `areka-P0-property-query-channels` が自分の担当範囲として挙げているのは照会の 4 経路の実装
  までで、台本を途中で分けることも、返事が返るまで待つ場所を台本に作ることも、進み方を組み
  直すことも引き受けていない。
- **台帳の宛先**: `areka-P0-property-query-channels`。宛先の欄は 1 本しか置けないので、同
  spec が担当範囲の 4 経路の 4 つ目として「`\![embed]` の対」を逐語で挙げていること、そして
  備考の「壊れ方」が述べる「別のイベントの返す台詞がその場に埋め込まれない」の、**返す側の
  道を作るのがこの 1 本である**ことを採った。
- **残りをどうするか**: 台本を組み立てる側でこのタグの意味を実装する仕事（台本を途中で分ける・
  返事が返るまで待つ場所を台本に作る・台本を進める側で進み方を組み直す）は
  `areka-P0-sakura-time-directives` に残る依存である。宛先の欄には現れないので、この行を依存の
  登記とする。
- **利用者から見える差**: 決める前は、別のイベントが返す本文がその場に現れないという壊れ方を
  誰も引き受けていなかった。決めた後は、引き受ける 2 本の名前が読めるようになった。**1 本
  だけでは直らない。**

**申-22——窓を動かすタグ**（`ukadoc:list_sakura_script:_5c_21_5bmove_5d:1`）

- **決めたこと**: 調査側の案「引数ごとに分ける」のうち、時間の側を
  `areka-P0-sakura-time-directives` が持つ点は採り、基準の側を `areka-P0-surfaces-basepos` が
  持つ点は採らない。
- **採らない理由**: `areka-P0-surfaces-basepos` の brief は、自分の範囲を宣言された基準点の
  意味を実装することに限り、このタグの他の意味を範囲外と明記している。そしてこの項目が「縮退」
  である中身は 3 つ（名前付きの書き方が解けない・基準の語のうち 4 つが解かれない・時間を指定
  しても即座に飛ぶ）で、宣言された基準点の実装はそのどれも消さない。既定の基準点の決め方は既に
  あるので、基準 `base` はそもそもこの 3 つに入っていない。
- **台帳の宛先**: `areka-P0-sakura-time-directives`。3 つのうち時間の指定を正面から、名前付きの
  書き方をその中の時間の指定を通じて動かす唯一の brief である。
- **残りをどうするか**: 基準の語 4 つ（`screen`・`primaryscreen`・`me`・`global`）は、どちらの
  brief の範囲にも入っていない。引受先は束「窓の配置と重なり」の候補 spec の側に持つ。候補
  spec には brief がまだ無く、要件 7.4 ⑶ が brief の無い名前を宛先に書くことを禁じているので、
  宛先には書かない。この 1 点は裁定候補にしていない——答えで作業の中身が変わらず、どちらに
  転んでも同じ束の同じ順位のまま候補 spec が引き受けるからである。
- **利用者から見える差**: 正典の記述例そのままの書き方で窓が動かないという壊れ方に、引受先が
  付いた。

**この 2 件は 5-3 の整理の後に足したものである。** 5-3 の規則 ⑴〜⑶ で変えた件数は変わらない
——この 2 件は規則 ⑴〜⑶ ではなく要件 8.6 による書き足しだからである。非空の宛先の件数と空の
件数はこの 2 件を含めて 5-3 の規則 ⑷ が数え直しており、生の数はそちらの 1 か所だけが持つ
（数え方は 4 台帳の行頭の `owner = ` の行を全部拾い、値が空文字列のものとそうでないものに
分ける）。

**この裁定で `roadmap-draft.md` の 4 か所が古びた。直すのは本節ではなく後続のタスクである**
（`roadmap-draft.md` は本節の担当の境界の外にある）。数え直した実数を置くので、この 4 か所を
そのまま直してほしい。放っておくと、宛先の数を突き合わせる判定を入れた時点で原因の分からない
赤になる。

| 古びた場所 | いまの記載 | 数え直した値 | 数え方 |
| --- | --- | --- | --- |
| spec 表 `[[spec]]` の `areka-P0-property-query-channels` の `owner_count` | 5 | **6** | 4 台帳でこの名前を宛先に持つ項目を数えた |
| spec 表 `[[spec]]` の `areka-P0-sakura-time-directives` の `owner_count` | 10 | **11** | 同上 |
| 束 2「窓の配置と重なり」の「依存する既存 spec」の `areka-P0-sakura-time-directives`（W16・1 件） | 1 件 | **2 件** | この束の構成 id 126 件のうち、この名前を宛先に持つものを数えた。**同じ記載が段階の一覧の表と先頭ウェーブの節の 2 か所にある** |
| 束 14「イベントの呼び起こし」の「依存する既存 spec」 | **0 本** | **1 本**（`areka-P0-property-query-channels`・1 件） | この束の構成 id 6 件のうち、非空の宛先を持つものを数えた |

**最後の 1 行は、明示して書いた 0 が偽になった点で、件数のずれより重い。** 0 は「調べたうえで
無かった」ことを表す書き方なので、放置すると「調べた結果」として読まれ続ける。

#### 出典 `briefing-property.md` と property の作業記録

| # | 出典（文書と節） | 要約 | 処分 | 理由 | 反映先 |
| --- | --- | --- | --- | --- | --- |
| 申-33 | `briefing-property.md`「26 という数には未決の別の読みが 1 つある」と「⑷ 二重所有の裁定案」ⓒ | 拡張プロパティの 4 件は、書き込みを認める印が無いのに本文が値を入れることも述べている。狭い読みと広い読みで、正典が書き込みを認める総数が割れる | 裁定候補 | 答えで作業が変わる。この 4 件の優先度が変わり、別の spec の brief が書く書き込み一覧の件数との一致も変わる。3 行の前置きは 6-3 にある | 6-3 |
| 申-34 | `briefing-property.md`「ⓑ テーマ『記憶』を付けなかったことと、その見え方」（同じ話が property 作業記録 4.1／6.1 の申し送り） | テーマの定義文書はこのドメインの 2 件を「記憶」の代表に挙げているが、調査はテーマを付けなかった。付けるかどうかと、付けるなら 2 件だけか枝ごとで最大 36 件かが未決 | 裁定候補 | 答えで作業が変わる。テーマの個数は順位の 4 つの根拠の 2 番目なので、付ければ束の順位が動く。3 行の前置きは 6-3 にある | 6-3 |
| 申-35 | `briefing-property.md`「⑷ 二重所有の裁定案」ⓐ（案 甲） | 重なり順と貼り付きの 2 項目を 3 本の spec が主張している。調査が推す案は、値の導出は 1 本が単独で持ち、まとめて持つ側から重なり順を外し、書き込み一覧の 1 行は別の 1 本が持つというもの | 採用 | ロードマップの正本には同じ形の仮裁定が既に登記されており、案の 3 つの柱のうち 2 つは一致する。**ただし書き込み一覧の 1 行の持ち主だけが両者で違う**（調査の案が挙げる spec と、ロードマップの仮裁定が挙げる spec が別である）。要件 12.3 がロードマップの編集を禁じ、要件 10.9 が反映を棚卸セッションの一括裁定に委ねているので、本 spec は食い違いを記録するにとどめ、2 件の宛先は空のままにする。先に実名を書くと、同じ問いへの答えが 2 か所に並ぶ | `ukadoc:list_propertysystem:currentghost.seriko.zorder:1` と `ukadoc:list_propertysystem:currentghost.seriko.sticky-window:1` の宛先は空のまま。食い違いの記録はこの行 |
| 申-36 | `briefing-property.md`「上流の README への提案（1 件）」 | 優先度の後ろの数値を 10 刻みにし、段階の末尾を 90 と定める提案 | 却下 | 理由は 6-4 に書いた | 6-4 |

#### 4 つの語では当たらなかった 1 件

| # | 出典（文書と節） | 要約 | 処分 | 理由 | 反映先 |
| --- | --- | --- | --- | --- | --- |
| 申-37 | `doc/ukadoc-coverage/ledger/shiori.toml` の冒頭コメント（`briefing-shiori.md`「群の索引」の写し） | 書き戻しで事実でなくなった記述が 4 点ある（下に列挙） | 却下 | 正本は `briefing-shiori.md` の側にあり、要件 12.3 がその編集を禁じ、要件 12.2 が台帳への接触を優先度と宛先の 2 欄に限っている。写しだけを直すと 申-07 の手順に反する | 8 節が指す是正候補の表 |

この 1 件は台帳のコメントの中にあるので、4 つの語でブリーフィングと作業記録を引く手では
当たらない。書き戻しを行った回の担当が同じファイルを読み直して数え直したもので、
事実でなくなったのは次の 4 点である。

- 記入済みと空の件数。書き戻しの後は記入済み 505・空 172 である。
- 空にしてある行の内訳。いま空なのは別名と対象外の 2 種だけで、実装済みの分は段階を持つ側へ移った。
- 「優先度が空」と断ってある群のうち 1 つは、もう空ではない。
- 「仮置き」に触れる行が群ごとの既定値を名指ししているが、書き戻し後の値は束の帰属と 3-3 の
  順位表から導いたもので、群ごとの既定値とは対応しない。

**直せるのは正本を持つ側だけである。** だから 8 節が指す是正候補の表へ回す。

**`README.md` の「仮置き」も同じ理由で事実でなくなった。** 欄の定義の表と「段階の最終決定は
ここでは行わない」の節が、優先度を仮置きと述べている（実測で 3 行）。第一段が草案だったあいだは
正しかったが、7 節で第二段が着地して段階と順位が確定したので、いまは正しくない。**こちらは本
spec の持ち物なので直す**——要件 12.4 が許す `README.md` の限定編集の中で、確定した引受先の
綴りへ改める。

### 6-3. 開発者の裁定候補

**この節に載るのは 2 件で、いずれも答えで作業が変わる。** 答えで作業が変わらないものは
6-2 でこちらが決め、理由を書いた。2-5 に書いた 10 件（束をどの段階に置くか）も裁定候補で、
前置きは 2-5 が持つ。**裁定候補は合わせて 12 件である。**

#### 裁定候補 11: 拡張プロパティの 4 件を、書き込みが効く側に数えるか（申-33）

- **何が問題か**: 正典は、この 4 件に書き込みを認める印を付けていないのに、本文では値を
  取り出すことと入れることの両方を述べている。狭く読むか広く読むかで、書き込みを認める
  項目の総数が変わる。
- **何を決めるか**: 狭い読み（調査が既定として採ったほう）を確定させるか、広い読みへ改めるか。
- **答えで利用者から見える結果がどう変わるか**: 広い読みを採ると、この 4 件は「書き込みが
  受け付けられるのに何も起きない」側へ移り、優先度が上がって早く直る対象になる。狭い読みの
  ままなら、拡張プロパティへの書き込みが黙って捨てられる状態が当分続く。

対象の 4 件: `ukadoc:list_propertysystem:activeghostlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1`・
`ukadoc:list_propertysystem:activeghostlist_28_30b4_30fc_30b9_30c8_540d_2f_672c_4f53_5074_540d_2f_30d1_30b9_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30:1`・
`ukadoc:list_propertysystem:pluginlist.index_28ID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1`・
`ukadoc:list_propertysystem:pluginlist_28_30d7_30e9_30b0_30a4_30f3_540d_2f_30d1_30b9_2fID_29.ext._62e1_5f35_30d7_30ed_30d1_30c6_30a3_540d:1`。

#### 裁定候補 12: テーマ「記憶」を付けるか、付けるなら何件に付けるか（申-34）

- **何が問題か**: テーマの定義文書は、ゴーストが付き合いの長さを知る 2 件を「記憶」の代表に
  挙げている。しかし調査はこの 2 件にテーマを付けなかったので、報告は「記憶 0 件」と出る。
  両方を読む人には食い違って見える。
- **何を決めるか**: 「記憶」を付けるかどうかと、付けるなら定義文書が名指しする 2 件だけか、
  同じ枝の下の全部（最大 36 件）か。
- **答えで利用者から見える結果がどう変わるか**: 付けるとテーマの個数が増え、この枝を含む束の
  順位が上がる。ゴーストが「前に会ったこと」を語れるようになる時期が早まる。付けなければ、
  付き合いの長さを使う挨拶は当分できない。

**この 2 件は 2-5 の 10 件と性質が違う。** 2-5 は「どの段階に置くか」の争いで、答えが順位表の
段階の列だけを動かす。この 2 件は台帳の中身（書き込みの可否・テーマ）の読み方の争いで、
答えが順位の根拠そのものを動かす。

### 6-4. 「10 刻み」の提案を採らない理由

`briefing-property.md` が上流の説明書へ 1 件だけ提案している——優先度の後ろの数値を 10 刻みに
し、段階の末尾を 90 と定める、というものである（申-36）。**この提案は採らない。**

理由は 3 つある。

1. **要件 7.1 が別の作り方を定めている。** 段階の中の順位は 1 から通しで振り、同じ鍵の束は
   同じ数値を持ち、その次は 1 増やす（1,2,2,3 の形）。10 刻みはこの形と両立しない。
2. **提案が挙げる利点が、本 spec の作り方では要らない。** 10 刻みが避けたいのは「1 件を
   前へ挿すたびに同じ段階の全部を振り直す」ことだが、本 spec の数値は人が手で振るのではなく、
   束の帰属と 4 つの根拠から機械が導いて 4 台帳へ書き戻す（3-2）。順位が動いたら書き戻しを
   もう 1 度走らせるだけで、手で直す行は 0 行である。
3. **数値の意味が 4 台帳で 1 つに揃った。** 数値は「その段階の中で何番目の束か」であって、
   束の数より大きい値は現れない。10 刻みにすると、値の大小と束の番号が別のものになる。

**この提案は誤っていたのではない。** 人が手で優先度を書き足していく台帳なら、10 刻みのほうが
書き換える行が少なくて済む。前提が変わった——本 spec が優先度を機械で導くようにしたので、
手で挿し込む場面が無くなった。

### 6-5. 上流の要件を 2 つ覆したこと

本 spec は、完了した上流の spec が定めた決まりを 2 か所で覆している。**どちらも本文は 5 節に
書いてあるので、ここでは覆した事実と場所だけを記す**（同じ話を 2 か所に書かない）。

| 覆したもの | 上流の定め | 本 spec の扱い | 本文 |
| --- | --- | --- | --- |
| 全体報告を常時の検査から外していたこと | 調査 4 本が同じ 1 つのファイルを取り合うので外す | 取り合う相手が居なくなったので検査へ戻した | 5 節「全体報告の新しさは常時の検査が判定する」 |
| 報告に証拠の有無を載せること | 上流の道具の spec の要件 2.3 と設計 D-11 | 全体報告の末尾から証拠の件数の表を外し、専用の副手続きの出力へ一本化した（開発者裁定 2026-09-12） | 同上 |

2 つ目は、上流が「載せる」と定めたものを外したので、**上流の要件をこちらが覆した形になる。**
外した理由は 5 節に書いたとおりで、この数がカタログにも台帳にも無い場所（ソースの木）から
来るため、本文に残すと `doc/ukadoc-coverage/` と関わりのない作業でも検査が赤くなるからである。

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
`kind` が `second-stage` の行——も **0 行**である。3-4 の「第二段の改訂（要件 9）の印は
0 行」は、この判断で確定した値である。

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

## 8. 是正候補への参照

既存の説明書（27 本の brief）と台帳の宛先が食い違うものの一覧は、**`roadmap-draft.md` の
「既存 brief への是正候補」の節**にある。spec 名・食い違う id・直し方の案の 3 列で並べてあり、
件数と数え方もその節が持つ。

6-2 の処分で「8 節が指す是正候補の表へ回す」とした申し送りの行き先も、この 3 列表である。

**この文書には表の写しを置かない。** 同じ表を 2 か所に持つと片方が必ず古びるためで（要件
8.8）、直すかどうかを決めるのは各 spec の持ち主である。説明書の本体は 1 文字も書き換えて
いない（要件 10.7）。

## 9. 完了時の報告

**この節を置く理由**: 要件 12.7 は完了時の報告に 6 つを書くことを求めているが、置き場を定めて
いない。本 spec の設計も置き場を挙げていない（どの工程で書くかだけを書いている）。6 つのうち 4 つ
がこの文書の中の節を指すので、段階と順位の正本であるこの文書の末尾に置いた。

数はすべて 2026-09-13 に数え直した値で、数え方を 1 つずつ添えてある。

### 9-1. 作った文書 3 本の絶対パスと行数（⑴）

| 文書 | 絶対パス | 行数 |
| --- | --- | ---: |
| 束の帰属の正本 | `C:/home/maz/git/areka/doc/ukadoc-coverage/linkage.md` | 4,902 |
| 段階と順位の正本（この文書） | `C:/home/maz/git/areka/doc/ukadoc-coverage/briefing.md` | 2,581 |
| 候補 spec とウェーブ案 | `C:/home/maz/git/areka/doc/ukadoc-coverage/roadmap-draft.md` | 1,198 |

数え方: 行数は `wc -l` で数えた。絶対パスは、この作業が本流へ合流した後に 3 本が置かれる場所
である。いま編集しているのは作業用ディレクトリ
`C:/home/maz/git/areka/.claude/worktrees/sakura-bare-tag-lexer-bbdf8c/` の下にある、
`doc/ukadoc-coverage/` から下が同じ綴りの 3 本である。

### 9-2. 台帳の編集件数（⑵）

| 欄 | 値が変わった項目の数 |
| --- | ---: |
| 関連（`links`） | 3 |
| テーマ（`values`） | 0 |
| 優先度（`priority`） | 1,445 |
| 宛先（`owner`） | 36 |
| 備考（`note`） | 36 |

数え方: この作業が分かれる前（`origin/main`）の 4 台帳と、いまの 4 台帳を項目 id で突き合わせ、
欄ごとに値が違う項目を数えた。項目の増減は 0 で、4 台帳の 1,749 項目すべてがこの 5 欄を持つ。

**テーマの欄が 0 である理由**: テーマは 4 本の調査が確定させた値で、本 spec に書き換える根拠が
無い。要件 12.2 は書き戻しの道具が触れる欄を優先度と宛先の 2 つに限っており、テーマはそこに
入っていないので 1 件も触っていない。残る 3 欄を触った根拠は、優先度が要件 7.1、宛先が要件
7.4、関連が要件 3.2（跨ぐ繋がりの不足分を書き足す）である。

**宛先と備考が同じ 36 である理由**: 宛先を変えた 36 項目と備考を変えた 36 項目は同じ 36 項目
である。ただし直し方は 2 通りある。**34 件**は、完了して封じられた spec を指す宛先を空にし、
空にした理由を備考へ書き足したものである（要件 7.4 ⑵・7.5）。**残る 2 件**は、空だった宛先へ
6-2 の裁定で引き受け先を書き入れ、裁定の理由を備考へ書いたものである（要件 8.6。対象は
`\![embed,イベント名,r0,r1,r2...]` と `\![move]` の 2 項目）。ドメイン別では、関連は資産 0・
プロパティ 1・さくらスクリプト 2・SHIORI 0、宛先と備考は資産 34・さくらスクリプト 2・
プロパティ 0・SHIORI 0 で、書き入れた 2 件がさくらスクリプトの 2 である。

### 9-3. 判定の種別数と摂動の本数（⑶）

**判定は 6 種**である（要件 11.1 の ⑴ 引用した項目 id の実在・⑵ 引用した機械の束 id の実在・
⑶ 束の帰属が重ならず対象の項目が漏れなくどこかに属すること・⑷ 段階ごとの件数と優先度の
一致・⑸ spec の表・⑹ 全体報告の作り直し）。**摂動は 31 本**である。

| 判定 | 摂動の本数 |
| --- | ---: |
| ⑴ ⑵ ⑹（引用と全体報告） | 5 |
| ⑶（束の帰属） | 8 |
| ⑷（段階と優先度） | 10 |
| ⑸（spec の表） | 8 |

数え方: 常時の検査のうち、実データの写しを 1 か所だけ壊して赤になることを主張する 31 本を
数えた（壊す道具そのものが狙いどおりに働くかを見る確認と、対象が 0 件でないことの確認は
別勘定で、ここには数えていない）。**31 本のうち 28 本は「赤はちょうど 1 件」を主張する。**
残る 3 本は、1 か所壊すと判定の中の複数の主張が同時に破れるので、より弱い形で「赤が出る
こと」と「その本文が狙った名前を含むこと」を主張する。

### 9-4. 裁定候補の一覧（⑷）

**合わせて 34 件**である。内訳は、段階の割り当て 10 件（2-5）・台帳の読み方 2 件（6-3）・本
spec の作業から出た 22 件（下の表）である。

#### 段階の割り当て（10 件・前置きは 2-5 にある）

メニュー／ヘッドライン／サウンドと環境の照会／投げ込み／キーとゲームパッド／好感度の絵柄／
作り付けの窓／薦める場所／通知領域／読み上げと聞き取り。**この 10 件が名指す束は 11 ある**
（3 つ目が「サウンド」と「環境の照会」の 2 束を 1 行にまとめているため）。

#### 台帳の読み方（2 件・前置きは 6-3 にある）

拡張プロパティ 4 件を書き込みが効く側に数えるか／テーマ「記憶」を付けるか、付けるなら何件に
付けるか。

#### 本 spec の作業から出たもの（22 件）

| # | 何を決めるか | 出どころ |
| ---: | --- | --- |
| 1 | 要件 12.4 とタスク 6.5 の完了状態を「3 か所」から**実際に直した 6 か所**へ改める | 4〜6 か所目は作業中に許可を得て直した |
| 2 | 要件 12.4 の括弧書きの節名を実態へ直す | 直した文のうち 2 つは「報告の扱い」節ではなく 2 章の別の節にある。設計の変更ファイルの節も同じ取り違えをしている |
| 3 | 要件 11.1 に判定 ⑺ を足すか | 7-7 の 5 つの数（未対応 67・語彙のみ 6・縮退 2・小計 75・実装済み 30）を見張る判定が **0 件**である。足すなら要件 12.2 の改訂も要る |
| 4 | 宛先の移動が絡む是正候補 **42 件**（うち 3 本を割って生まれた説明書へ移すもの **24**・どちらが持つかを決めるもの **18**）を、いま移すか、是正候補のまま各 spec の実装時に直すか | 作業中の判断は「移さない」。要件 10.7 は挙げるだけを求めており、移すと検証済みの表を作り直すことになる。**内訳**: 分割由来は書体の欄 14 ＋ 寄せと影 5 ＋ 表示寿命 5 ＝ 24 で、直し方は「宛先を移す」。残る 18 はサウンドの語彙で、分割由来ではなく「説明書と台帳のどちらが持つかを決める」別件である（数え方: `roadmap-draft.md`「既存 brief への是正候補」の表の 4 行を、その節の全列挙で数え直した）。本 spec のタスクの記録は 4 行をまとめて「分割 3 本由来の 42 件」と書いており、その綴りはここで改めた |
| 5 | 2-5 の書き方を直すか（候補 **10 件**に対し名指す束は **11**） | 3 つ目が 2 束を 1 行にまとめている |
| 6 | 裁定待ちの束 **11** のうち、順位表に「段階は仮」の注記が付くのは **6 束**だけでよいか | 残る 5 束は注記が無いのに裁定は済んでいない。注記の有無は段階が確定した印ではない |
| 7 | spec の表の段階の欄の**値そのもの**を見る判定を作るか | いまの判定 ⑸ の 6 つの見方は、この欄の値をどれも読まない。段階の欄を持つのは束に属する **13 行**だけで（値は A が 9・C が 2・E が 2。束に属さない 14 行は欄ごと無い）、**その 13 行の段階が、順位表でその束が置かれている段階と一致するかを確かめる見方が無い**。食い違っても赤にならない |
| 8 | 摂動 3 本が「ちょうど 1 件」を主張できないままでよいか | 1 か所壊すと判定の中の複数の主張が同時に破れる構造による |
| 9 | 設計の入口の署名の記述を実装へ合わせる | 設計は引数を取る形で書いているが、実装は引数を取らない |
| 10 | 設計の判定 ⑵ の走査の記述を実装へ合わせる | 設計は「項目 id で始まる行の 1 列目」と書くが、実装は束の見出しを走査する。設計どおりだと別名 27 件が紛れて偽の緑になるので、実装の側が正しい |
| 11 | 設計の「同じ機能の関連 46 本」の内訳を書き直す | 46 は「ページどうし 26 ＋ ページと項目 20」の和で、ページどうしだけでは 26 である |
| 12 | 設計の丸括弧の数字が**要件番号**でありタスク番号でないことを明記する | 読み違えると当たらない番号を探すことになる |
| 13 | 設計のファイル構成の節と「1,000 行を超えさせない」の節を実装へ合わせる | 判定 **⑴⑵⑸⑹** を `documents_checks.rs` 1 本に置くと書いてあるが、実装は `documents_checks.rs`（⑴⑵⑹）・`spec_checks.rs`（⑸）・`documents_tools.rs`・`documents_non_vacuity.rs` の 4 本に割れている（1 ファイル 1,000 行の上限を守るため）。⑶⑷ は設計自身が当初の同居案を取り下げており、設計と実装が一致している |
| 14 | 設計の変更ファイルの節が挙げる節名を実態へ直す | 2 と同じ取り違え |
| 15 | 設計が `README.md` へ求めている指示文を実態へ直す | 「証拠の表だけを外す理由に書き換えよ」と書いてあるが、表を丸ごと外す裁定がその後に出ている |
| 16 | 本 spec の外の文書に残る**旧見出しの参照 8 か所**を直すか | 要件・設計・タスク・設計の検証報告に残る。境界の外なので触っていない |
| 17 | `roadmap-draft.md` の先頭ウェーブの節に並ぶ状態の数を判定に載せるか | いまは 2026-09-13 に台帳を引いて数えた写しで、判定が数え直さない |
| 18 | 候補 spec 名 **63 個**をいつ実在させるか | ほとんどはまだ案で、この綴りの spec は無い。同じ綴りの spec が既に起票されていて説明書を持つものが **1 個** `areka-P0-nar-install` あり、これだけは意図しない重なりである。母数 63 の決まり方と、⑴ の規則で既存 spec 名をそのまま使う行（63 には入らない・こちらは意図した重なり）との切り分けは `roadmap-draft.md`「読み方」と「既存 brief の位置づけ」にある。ここには写さない |
| 19 | 要件 6.2 の文言を「順位の行に書く」から「帰属の文書の欄を指す」形へ改めるか | 順位の囲みが自分で持つのは ⑶ 影響する既存資産の広さと ⑷ 依存基盤の共有度の **2 つ**だけで、⑴ 壊れ方・⑵ 伺からしさのテーマ・⑴ ⑵ の値が由来する構成 id の **3 つ**は `linkage.md` の欄を指している（引き方の表は 3-1 にある）。同じ値を 2 か所に持たないための形（要件 8.8）で、値はどれも 1 手で辿れる。ただし「順位の行に書く」という要件 6.2 の文言は文字どおりには満たしていない |
| 20 | 台帳の備考に残る「ファイル名とその中の行の番号」の綴りを誰が引き受けるか | `ledger/property.toml` に **417 か所**ある（**188 項目**にまたがり、綴りの種類は **7**。ほかの 3 台帳は **0 か所**）。数え方: 4 台帳の本文から「拡張子付きのファイル名＋コロン＋数字」の形をすべて拾って数えた。全部が完了済み `areka-P0-ukadoc-survey-property` の残したもので、本 spec が足したのは **0 か所**である（数え方: 本流から分かれた地点からいまの差分の追加行を、同じ形で拾って数えた）。足していないので要件 12.6 は満たすが、要件 7.5「既存の記述を消さず」が消すことを塞いでおり、**いまの規律のままでは直せず引受先も無い** |
| 21 | 設計のファイル構成に挙がっていない実ファイル **4 件**を書き足すか（13 と同じ処分でまとめられる） | `src/documents/fields.rs`（**294 行**。`parse.rs` が 1 ファイル 1,000 行の上限に迫ったための切り出し）・`src/error.rs` に足した所見の種別 1 つ（`DeriveMismatch`）・`src/io/paths_tests.rs`・`tests/cli_streams.rs` の 4 つが、設計のファイル構成の節にも変更ファイルの節にも無い（`tests/cli_streams.rs` は設計の別の節が 1 行だけ触れている） |
| 22 | 判定 ⑸ の「完了済み spec の宛先」を見る見方が、spec の表の側を見ないままでよいか | この見方はこの文書の完了済み spec の列挙に載る名前だけを見るので、`roadmap-draft.md` の spec の表に載ったまま封じる場所へ移った spec は素通りする。いまの対象は `areka-P0-charset-canon`（宛先 **5 件**）と `areka-P0-present-gpu-transform-scale`（宛先 **0 件**）の 2 本で、5 件はすべて実装済みなので、**いま見落としている項目は 0 件**である（数え方: 表の 27 行のうち封じる場所にあるものを拾い、その名前を宛先に持つ項目の状態を台帳 4 本から引いた） |

### 9-5. 先頭ウェーブの束名と候補 spec 名（⑸）

`/kiro-discovery` へ渡すのは次の 6 束である（`roadmap-draft.md`「先頭ウェーブ」に、束ごとの
3 行の要約・依存する既存 spec・構成 id の全列挙がある）。

| 順位 | 束 | 候補 spec 名の案 |
| ---: | --- | --- |
| 1 | 会話 | `areka-P0-talk-script-canon` |
| 2 | 窓の配置と重なり | `areka-P0-window-placement-canon` |
| 3 | 名前の記憶 | `areka-P0-user-name-memory` |
| 4 | 起動と挨拶 | `areka-P0-boot-greeting-canon` |
| 5 | バルーンの文字 | `areka-P0-balloon-font-canon`（残余） |
| 6 | サーフェスアニメーション | `areka-P0-seriko-animation-canon` |

6 束の構成 id の総数と、そのうち進行中の spec・封じた spec が宛先に持つ件数は、
`roadmap-draft.md`「先頭ウェーブ」の冒頭に 1 度だけ置いた。**ここには写さない**（同じ数を
2 か所に持たない。以前はこの節にも写しを置いていて、宛先が動いた後も古い値のまま残った）。
その 3 つの数は判定が数え直さないので、台帳を触った人が向こうの節を直す。
**この 6 束に掛かる同順位の組は 0 組・段階の裁定候補は 0 束である**（数え
方は `roadmap-draft.md` の「先頭ウェーブ」と「裁定候補」の各節にある）。段階 A の順位 7 で
初めて同順位が現れるので、7 位から先を先頭ウェーブに入れるには 9-4 の裁定が要る。

### 9-6. ロードマップの改訂候補（⑹）

棚卸セッションへ渡す候補は **10 件**である。**`.kiro/steering/roadmap.md` は 1 文字も編集して
いない**（要件 12.3）。本体はすべて `roadmap-draft.md` にあり、ここは見出しだけを並べる。

| # | 改訂候補 | 出どころの節 |
| ---: | --- | --- |
| 1 | M2 の最初のウェーブを W17 の後に置く | 「裁定候補」ウェーブの並べ替え |
| 2 | `areka-P0-anchor-tag-canon` を繰り上げるか、宛先 1 件を先頭ウェーブの候補 spec へ移すか | 同上 |
| 3 | `areka-P0-surfaces-basepos` の「任意」を外して W13 で確定させるか | 同上 |
| 4 | 同順位 **13 組・29 束**の解消（先頭ウェーブより後の着手順を決める） | 「裁定候補」同順位の解消 |
| 5 | 先頭ウェーブ 6 束を spec として起票し、正本の spec 台帳へ登記するか | 「先頭ウェーブ」 |
| 6 | 候補 spec 名 **63 個**（うち先頭ウェーブが 6）を案のまま置くか、順に起票するか | 「読み方」「段階 A」〜「段階 E」 |
| 7 | M2 予約群 **17 項目**のうち、束に写った **10 項目**と写らなかった **7 項目**を正本の列挙へ反映するか | 「M2 予約群の対応表」 |
| 8 | M3「伺かの冠」の受入基準を「テーマ 8 つすべてで代表束が実装済み」とするか。代表の決め方は 2 案ある | 「M3 の受入基準の候補」 |
| 9 | 技術選定 4 つ（pasta の native x64・`IShiori` の in-proc 化・ベクトル描画・AI）を段階にもウェーブにも並べない扱いのままにするか。台帳の項目はいずれも **0 件**である | 「別軸」 |
| 10 | 既存 27 本の説明書への是正候補を、各 spec の持ち主へ回すか棚卸で一括裁定するか | 「既存 brief への是正候補」（8 節が指す表） |
