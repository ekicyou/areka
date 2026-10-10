# SSP の BTS の要望の台帳

`areka-P0-ssp-bts-salvage` の調査の記録である（要件 1〜8）。SSP の課題管理（BTS・https://bts.shillest.net/ ）に溜まった要望を 1 回だけ通読し、areka の目線で 1 件 1 行にまとめた。

**これは 1 回きりの調査である。** 定期的に見直す仕組みは持たない。BTS の側は書いた後も動くので、古さは下の「見た範囲」で判断する。

## 見た範囲

| 回 | 通読した日付 | 番号の範囲 | 一覧の総数 | 深刻度「要望」の件数 | 一覧の取り方と絞り込みの条件 | areka の側を照らし合わせた日付と main のコミット |
|---|---|---|---|---|---|---|
| 1 | 2026-10-10 | 0000001〜0000808 | 808 | 187 | CSV を 1 回（匿名）。検索の入口 `search.php` で「ステータス非表示」を「なし」にした、その場かぎりの絞り込みで出した全件 | 2026-10-10・main `414d43eb` |

- 起票時の実測（全 808 件・要望 187 件・最新 0000808）との差: **差なし**。
- 通読を始めた時点の最新の番号は 0000808。これより後の番号は扱っていない。
- 既定の一覧は「完了」の課題を隠していて 20 件しか出ない。絞り込みの欄の送信（`view_all_set.php`）は匿名では拒まれる（401）。全件は上の検索の入口から出せた。

## 件数の内訳

| 項目 | 件数 |
|---|---|
| 深刻度「要望」から行にした件数 | 187 |
| それ以外の深刻度から行にした件数 | 54 |
| 読んだが外した件数 | 11 |
| 対象に入らなかった件数（頁を開いていない） | 556 |
| 合計（一覧の総数） | 808 |

- 読んだが外した番号: 0000068、0000081、0000082、0000109、0000150、0000279、0000374、0000409、0000430、0000441、0000471

## 読み方

### 欄の意味

| 欄 | 書いてあるもの |
|---|---|
| BTS | 課題の番号（`BTS 0000709` の形） |
| 要約 | 課題の題をそのまま |
| 深刻度 | BTS の深刻度（画面の日本語の語） |
| SSP 側の結末 | 下の 7 語のどれか 1 つ。「実装済み」には修正済バージョンを括弧で添える |
| 種別 | 下の 3 語のどれか 1 つ。深刻度ではなく中身で決めた |
| areka との関係 | 下の 5 語のどれか 1 つ |
| 根拠・理由 | 関係の根拠（spec の名前・網羅の台帳の項目・ぶつかる方針・対象外の理由）。「未着手」で書くことが無ければ `—` |
| ひと言 | 中身を自分の言葉で 1〜2 文にまとめたもの。説明やコメントの引用ではない |

- **課題の頁の URL**: `https://bts.shillest.net/view.php?id=` の後ろに、番号の先頭の 0 を除いた数を付ける（`BTS 0000709` なら `view.php?id=709`）。
- ひと言が「（未読）」の行は、まだ頁を読んでいない行である。「頁を読めず（理由）・一覧の情報だけ。」で始まる行は、頁を読めなかった行である。
- 設定や機能の名前は ukadoc の用語で書く。areka の spec は名前で指す。

### SSP 側の結末（7 語）

ステータスと解決状況から決める。頁を読んで、作者のコメントが欄と違う結末をはっきり述べているときは、コメントの側の語で書き、違いをひと言に書く。

| 台帳の語 | BTS の解決状況（CSV の語） | 補い |
|---|---|---|
| 実装済み（版） | fixed | 修正済バージョンを括弧で添える（一覧では `2.4`〜`2.9` の刻み）。取れなければ「版は不明」 |
| 未対応 | open・reopened | ステータスが新規・担当者決定などでも同じ語にする。閉じられているのに解決状況が open・reopened の課題は、頁のコメントで決める |
| 却下 | not fixable（won't fix は、この回の一覧には無かった） | |
| 仕様どおり | no change required | |
| 重複 | duplicate | 相手の番号が分かれば、ひと言に書く |
| 保留 | suspended（この回の一覧には無かった） | |
| 再現せず | unable to reproduce | |

### 種別（3 語）

上から順に当てはめ、最初に当たったものにする。

| 台帳の語 | いつ使うか |
|---|---|
| 要望 | 何かを足す・変えることを求めている（不具合として出されていても、中身がそうならこれ） |
| 挙動の説明 | 作者が「そう動く」と説明していて、その挙動を ukadoc が定めていない |
| 質問 | 上のどちらでもなく、やり方や仕様を尋ねている |

### areka との関係（5 語）と、決め方の順

上から順に当てはめ、最初に当たったものにする。

| 順 | 台帳の語 | いつ使うか | 根拠・理由の欄に書くもの |
|---|---|---|---|
| 1 | 対象外 | SHIORI（里々・YAYA など）かゴースト個別の話 | 「SHIORI の話」「ゴースト個別の話」 |
| 2 | 方針とぶつかる | 記録のある裁定・方針が、逆を決めている | ぶつかる方針と、その在りか |
| 3 | 対象外 | ベースウェアの機能の話でない・デスクトップマスコットの枠の外・areka の作りでは意味を持たない | その理由 |
| 4 | 実装済み | 求められていることが、今の areka で動く | spec の名前か、網羅の台帳の項目 |
| 5 | brief あり | まだ動かないが、進行中の spec（`.kiro/specs/` の直下にフォルダがある）が受け持っている | spec の名前 |
| 6 | 未着手 | 上のどれでもない。決めきれないときもこれ | 登記や一部の実装があれば、それ |

- 「挙動の説明」の行では、「実装済み」は areka が同じ挙動を決めて持っていること、「方針とぶつかる」は別の挙動を裁定済みであること、「未着手」はまだ決めていないことを表す。作者の説明は手がかりであって正典ではない。
- 「質問」の行の関係は、尋ねられている機能について決める。機能の話でない質問（配布元やサイトのこと・SSP の画面の使い方だけ）は、順 3 の「対象外」にする。

### 画面の語との対応

CSV は英語の語で出る。画面の日本語の語は、一覧の画面で確かめた分を書く。

| 欄 | CSV の語 → 画面の語 |
|---|---|
| 深刻度 | feature → 要望／minor → マイナー／major → メジャー／crash → クラッシュ／trivial → 些細／text → 表示／tweak → 微調整／block → ブロック（block だけは画面で未確認・MantisBT の標準の訳） |
| ステータス | new → 新規／feedback → フィードバック／confirmed → 再現済／assigned → 担当者決定／closed → 完了 |
| 解決状況 | fixed → 実装済／open → 不明／reopened → 差し戻し／no change required → 修正不要／not fixable → 修正不可／duplicate → 二重登録／unable to reproduce →（画面で未確認）。CSV にはこの 7 つが在った。一覧の画面にも課題の頁の本体にも解決状況の欄は出ないので、日本語の語は課題の頁の下の「課題の履歴」の変更内容で確かめた（open の「不明」は、履歴の変更前の値として出る） |
| カテゴリ | `SSPBT：本体(SSP)`・`SSPBT：ゴースト`・`SSPBT：その他`・`整備班：YAYA`・`整備班：里々`・`整備班：ゴースト`・`整備班：その他`・`SSPヘルプ（説明書）`・`UKADOC（仕様書）` の 9 つ |

### 照らし合わせの決まり

areka との関係は、ukadoc の網羅の台帳（`doc/ukadoc-coverage/`）・`.kiro/steering/roadmap.md`・`.kiro/specs/`（`completed/` を含む）・開発者の裁定（steering と spec の文書・`doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」・エージェントの記憶）に照らして決めた。網羅の台帳は書き替えていない。

**網羅の台帳の状態からの写し**

| 網羅の台帳の状態 | 持ち主（`owner`） | 関係 | ひと言に添えること |
|---|---|---|---|
| `implemented` | — | 実装済み | — |
| `degraded` | — | 求められている部分が動くなら 実装済み。動かない部分が要望の中心なら、下の `absent` と同じに決める | 「縮退」とその違い |
| `vocabulary-only`・`absent` | 進行中の spec | brief あり | — |
| `vocabulary-only`・`absent` | 空、または完了済みの spec | 未着手 | 「語彙のみ」／「持ち主は完了済み」 |
| `alias` | — | `alias_of` の先の項目で決める | — |
| `not-applicable` | — | 対象外 | 台帳の備考の理由 |
| `unclassified` | — | 未着手 | 「網羅の台帳で未分類」 |

**そのほかの相手からの写し**

| 見つけたもの | 関係 | 添えること |
|---|---|---|
| 完了した spec が求められていることを作った | 実装済み | spec の名前 |
| 完了した spec が一部だけ作り、残りが要望の中心 | 進行中の spec が残りを持てば brief あり、無ければ 未着手 | 一部を作った spec の名前 |
| roadmap の「覚え書き（brief なし…）」「予約（全て任意・brief なし）」に登記がある | 未着手（brief が無いので「brief あり」とは書かない） | 「roadmap の覚え書きに登記あり」／「予約に登記あり」 |
| roadmap の「生きている決まり」「テーマ別の決めごと」・`tech.md`・`doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」・spec の文書が逆を決めている | 方針とぶつかる | 文書の名前と、節の題か項目の名前 |
| 網羅の台帳のカタログに項目が無く、spec にも無い | 未着手 | 「カタログに項目なし」 |
| 資料から 1 つに決めきれない | 未着手 | 何が決めきれなかったか |

**根拠の指し方**

- spec は名前だけ（`areka-P0-…`）。書く前に実在を確かめる。
- 網羅の台帳は「台帳のファイル名＋ukadoc の用語」で指す（`catalog.toml` を `title` で引いて id を得て、その id で `ledger/*.toml` の `status` と `owner` を読む）。
- 裁定・方針は、文書と節の題（または項目の名前）で指す。行番号では指さない。リポジトリに記録の無い裁定は「開発者の裁定（日付・リポジトリに記録なし）: 中身」と言葉で書く。

## 採り方

### ふるい分けの決まり

一覧の情報（番号・要約・カテゴリ・深刻度・ステータス・解決状況）だけで決めた。この段では頁を開いていない。

1. 深刻度が「要望」→ 対象（全部）。
2. 深刻度が「要望」でないもののうち、次のどちらかに当たるもの → 対象。
   - **中身が要望・質問と読める**: 要約が、求める言い回し（〜したい・〜してほしい・〜できるように・〜の追加・対応の希望・提案）か、尋ねる言い回し（〜できますか・〜でしょうか・仕様ですか）になっている。英語の題も同じ読みで拾う。
   - **挙動の説明がありそう**: 解決状況が no change required・won't fix・not fixable のどれかで、要約がタグ・イベント・設定・表示の動きを名指ししている（落ちる・固まる・起動しないだけの題は拾わない）。
3. それ以外 → 対象に入らない。頁を開かず、行にもしない。

- 迷ったら拾う。深刻度が「要望」でない対象を読んで、要望でも質問でもなく、作者の説明も無い（または説明された挙動を ukadoc が定めている）と分かったら、行から外して「読んだが外した」に数える。深刻度「要望」の課題は読んだ後も外さない。

### 読まなかった課題の種類

- **SSP の実装の不具合**（上の 3）。areka の不具合ではないので、頁を開かず、行にもしていない。
- **SHIORI・ゴースト個別の課題**。対象に入ったものは、頁を開かず一覧の情報だけで行にして「対象外」とした（主題「SHIORI・ゴースト個別」）。

### SHIORI・ゴースト個別の見分け

カテゴリだけで決めず、カテゴリと要約の両方で決めた。

| カテゴリ | 要約 | 扱い |
|---|---|---|
| SHIORI のカテゴリ（`整備班：YAYA`・`整備班：里々`） | ベースウェアの語を含まない | 頁を開かず「対象外」 |
| ゴーストのカテゴリ（`SSPBT：ゴースト`・`整備班：ゴースト`） | 個別のゴーストの名前・台詞・辞書の話 | 頁を開かず「対象外」 |
| 上のどちらか | ベースウェアの語（本体・さくらスクリプトのタグ・SHIORI イベントの名前・設定ファイルの名前・バルーン・シェル・メニュー・インストール・更新）を含む、またはどちらとも読めない | 頁を開く |

開いた後で SHIORI・ゴースト個別と分かった行は「対象外」とし、ひと言に「頁を読んだ」と書く。

### 起票の候補の物差し

- 候補は、areka との関係が「未着手」の行の全部を、⑴ 作る話 ⑵ 確かめてから ⑶ 決める話 に分けて挙げる。
- デスクトップマスコットの枠に収まるのは、デスクトップに常駐するキャラクター（ゴースト・シェル・バルーン）の表示・会話・触れ合い・配布と更新と、作者と利用者がそれを作り動かすための支え。収まらないのは、キャラクターと関わらない汎用の道具と、利用者の様子を探るもの。
- リポジトリの中の先例は 2 つ。完了した `areka-P0-file-drop` の要件が範囲外に置いた「汎用の書庫の解凍機・ビューアとしての働き」と、roadmap の「MCP の areka 独自ツールと影響の段」が採らなかった案に挙げた「利用者がいるかの検知」。

## 続きを拾い直すときの決まり

- 続きは新しい spec として起票する。
- 「見た範囲」の最新の番号の次から、この台帳の「採り方」と同じ決まりで拾う。
- 新しい台帳は作らない。この台帳（完了後は `.kiro/specs/completed/areka-P0-ssp-bts-salvage/` の中）の主題の表に行を書き足す。
- 「見た範囲」に回の行を 1 つ足し（前の回の行は書き替えない）、件数の内訳を合計に直す。
- 主題は同じ一覧を使う。どの主題にも入らない行が出たときだけ主題を足す。

## 主題の一覧

| 主題 | ひと言の定義 | 行の数 |
|---|---|---|
| さくらスクリプトと文字の現れ方 | さくらスクリプトのタグと、文字・音の出し方 | 17 |
| バルーン | バルーンの見た目・位置・入力ボックス | 16 |
| シェルとサーフェス | シェルの描き方・サーフェスとアニメーションの定義 | 13 |
| SHIORI イベント・リソース・プロパティシステム | ベースウェアが SHIORI へ知らせるイベントと、SHIORI から引ける情報 | 15 |
| 窓と配置 | キャラクターの窓の位置・重なり順・ディスプレイ | 14 |
| メニュー・設定・操作 | 右クリックメニュー・設定画面・エクスプローラ・ログなど利用者が触る画面 | 33 |
| インストール・配布・削除 | nar の作り方と入れ方・アンインストール | 21 |
| ネットワーク更新 | ゴースト・シェル・バルーン・本体の更新の仕組み | 21 |
| 外との連携 | SSTP・FMO・プラグイン・メール・音声・Web からの呼び出し | 26 |
| 開発者向けの機能 | 開発用パレット・サーフェスの書き出し・テストの機能 | 20 |
| 起動・終了・ゴーストの切り替え | 起動の引数・終了の待ち・初回起動 | 4 |
| その他 | 翻訳・配布元・文字コードなど、上のどれにも入らないもの | 21 |
| SHIORI・ゴースト個別 | SHIORI（YAYA・里々など）とゴースト個別の話。頁は開いていない | 20 |

## 主題ごとの表

### さくらスクリプトと文字の現れ方

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000073 | a little question of sakura script | 些細 | 実装済み（版は不明） | 挙動の説明 | 未着手 | 引数を二重引用符で囲む読みは `areka-P0-sakura-parse` が作った。台本の引数に入った改行の扱いは決めていない | `\q` の引数にカンマや閉じ括弧を含む文字列を渡す書き方の質問に、引数を二重引用符で囲む方法を作者が示して閉じた（直しは無い）。改行は台本に入れられず、SSP は内部でバイト値 1 に置き換えている（仕様の外の動き）とも説明した。 |
| BTS 0000153 | \j command doesn't support all characters | 要望 | 実装済み（2.4.98） | 要望 | 実装済み | `ledger/sakura-script.toml` の `\j[ID]`（`degraded`）・`areka-P0-open-external-tags` | `\j[ID]` に日本語などを含む URL を書くと文字が化けて別の頁が開くので直してほしいという要望で、SSP は 2.4.98 で直した。areka は URL を UTF-16 のまま OS へ渡して開く（縮退は、URL でない旧来の ID へのジャンプを開かない点）。 |
| BTS 0000156 | 'The operation cannot be performed because the pins are not connected' error when playing some songs with \![sound,play,...] | マイナー | 仕様どおり | 挙動の説明 | 未着手 | `ledger/sakura-script.toml` の `\![sound,play,ファイル名,オプション...]`（`absent`・持ち主なし） | 一部の曲が再生できずエラーになるのは、SSP が音の再生を OS の DirectShow に任せていて、その形式を解く部品が入っていないためだと作者が説明した。areka は音の再生をまだ持たず、どの仕組みで鳴らすかも決めていない。 |
| BTS 0000213 | About the implementation details of '\C' script | 微調整 | 仕様どおり | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![lock,balloonrepaint]`（`absent`・持ち主なし）。`\C` も `absent`・持ち主なし | `\C` で前のバルーンへ継ぎ足して描き直すたびに表示がちらつくので、一区切りまで描いてからまとめて出してほしいという要望。作者は `\![lock,balloonrepaint]` と `\![unlock,balloonrepaint]` で挟めばよいと案内して閉じた。 |
| BTS 0000264 | Sakurascript tag to temporarily adjust text speed | 要望 | 実装済み（2.5.23） | 要望 | brief あり | `ledger/sakura-script.toml` の `\![set,balloonwait,倍率]`（`vocabulary-only`）・`areka-P0-sakura-time-directives` | 利用者の決めた文字の表示の速さを基準に、台本の中でだけ速さを相対的に上げ下げできるタグがほしいという要望。SSP は 2.5.23 で `\![set,balloonwait,倍率]` を足した。 |
| BTS 0000267 | Options to center or right-align text | 要望 | 実装済み（2.5.31） | 要望 | brief あり | `ledger/sakura-script.toml` の `\f[align,寄せる側]`（`vocabulary-only`）・`areka-P0-text-align-shadow-canon` | どのバルーンでもメニューが整って見えるよう、文字を中央寄せや右寄せにできる指定がほしいという要望。SSP は 2.5.31 で `\f[align,寄せる側]` を足した。 |
| BTS 0000271 | absolute changes with balloonwait tag | 要望 | 実装済み（2.5.24） | 要望 | brief あり | `ledger/sakura-script.toml` の `\![set,balloonwait,倍率]`（`vocabulary-only`）・`areka-P0-sakura-time-directives`（brief は倍率とミリ秒の指定の両方を挙げている） | 歌詞を曲に合わせて出すような使い方のために、文字ごとの待ち時間を倍率でなく決まった時間で指定できるようにしてほしいという要望。SSP は 2.5.24 で `\![set,balloonwait,倍率]` にミリ秒の指定を足した。 |
| BTS 0000330 | allow \![executesntp,timesever] | 要望 | 実装済み（2.5.40） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![executesntp]`（`absent`・持ち主なし） | 時計合わせを始める `\![executesntp]` で、問い合わせる時刻サーバーを引数で指定できるようにしてほしいという要望。SSP は 2.5.40 で入れた。 |
| BTS 0000348 | Add new 'disable' text color to balloons, which can be used with the \f[color] tag | 要望 | 実装済み（2.5.51） | 要望 | 実装済み | `ledger/sakura-script.toml` の `\f[disable]`（`implemented`・`areka-P0-text-decoration-canon`）と、`ledger/assets.toml` の `disable.font.(フォント定義),(指定)`（`degraded`・`areka-P0-balloon-font-descript-keys`。縮退は縁取りと影の指定が効かない点で、色は効く）。指定が無いときの色は `doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」が背景の色と文字色の混色と決めている | 選べない選択肢を別の色で見せたいが、決め打ちの色ではバルーンによって読みにくくなるので、「無効」の文字色をバルーンの側で決められるようにし、指定が無ければ背景と文字色から本体が作ってほしいという要望。SSP は 2.5.51 で入れた。 |
| BTS 0000389 | Make it possible for disable text option to affect only color | 要望 | 仕様どおり | 要望 | 実装済み | `ledger/sakura-script.toml` の `\f[color,色指定]`（`implemented`・`areka-P0-text-decoration-canon`。色指定の `disable` を受ける） | `\f[disable]` は太字や縁取りまで既定に戻してしまうので、色だけを「無効」の色に替える書き方がほしいという要望。作者は、`\f[color,色指定]` に `disable` を書く形が既に在ると案内して閉じた。 |
| BTS 0000419 | Give \![open,readme] an optional argument to specify what readme to open | 要望 | 実装済み（2.5.76） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,readme]`（`implemented`・`areka-P0-popup-menu-minimal`）は引数なしの形だけ。種類と名前の引数付きは、同 spec の要件が「語彙だけ持ち、何もしない」と置いている | プラグインのメニューから自分の readme を開き直せるよう、`\![open,readme]` に開く相手（種類と名前）を引数で指定できるようにしてほしいという要望。SSP は 2.5.76 で入れた。 |
| BTS 0000501 | Add time option to \![set,alpha] tag | 要望 | 実装済み（2.7.32） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![set,alpha,数値,オプション]`（`absent`・持ち主の `areka-P0-sakura-time-directives` は brief で範囲から外している）。roadmap の覚え書きに登記あり（時間の指令の C・D 群） | 透明度を少しずつ変えるタグを何十も並べずに済むよう、`\![set,alpha,数値,オプション]` に変化にかける時間を指定できるようにしてほしいという要望。SSP は 2.7.32 で入れた。 |
| BTS 0000580 | More ghost-friendly url handling | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000611 | \q[]に\u[]とかを挟むと\q側の]が認識されないのって仕様ですか？ | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000692 | Add tag \![open,dressupexplorer] | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000740 | Extra "wait" parameter for \![set,alpha] and \![set,scaling] tags | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000745 | \![open,dialog,open]などのダイアログ系を実行後に大きな負荷が発生する | メジャー | 仕様どおり | — | — | — | （未読） |

### バルーン

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000014 | 左右非対称なシェルでのバルーン表示位置 | 要望 | 実装済み（2.4） | 要望 | 未着手 | `ledger/assets.toml` の `sakura.balloon.offsetx,座標`（`absent`・持ち主なし）。左右別の指定はカタログに項目なし | 左右非対称のシェルのために、バルーンの表示位置のずらし量を左側用と右側用で別々に書けるようにしてほしいという要望。SSP は 2.4 で入れた。 |
| BTS 0000016 | 吹き出しの表示をSNS風にする機能 | 要望 | 実装済み（2.8.51） | 要望 | 未着手 | カタログに項目なし。前提になる多重ゴーストは roadmap の予約に登記あり | 複数のゴーストを同時に出すとバルーンが入り乱れて読みにくいので、発言をひとまとめにしてチャットの画面のように並べる表示がほしいという要望。SSP は 2.8.51 で入れた。 |
| BTS 0000026 | シェル倍率でバルーン連携しない場合 | マイナー | 仕様どおり | 挙動の説明 | 未着手 | `ledger/sakura-script.toml` の `\![set,scaling,倍率]`（`absent`・持ち主の `areka-P0-sakura-time-directives` は brief で範囲から外している） | シェルだけ倍率を変えてバルーンを連動させないとき、バルーンの大きさは変わらないが出る位置はシェルの倍率に合わせて動く、と作者が説明した。areka はシェルの倍率の変更をまだ持たないので、この動きも決めていない。 |
| BTS 0000119 | バルーン倍率をシェルと連携せずに別々に調整する機能 | 要望 | 実装済み（2.5） | 要望 | 未着手 | `ledger/shiori.toml` の `OnBalloonScaling`（`absent`・持ち主なし） | バルーンの倍率を、シェルの倍率に連動させずにゴーストごとに決められるようにしてほしいという要望。SSP は 2.5 で入れた。 |
| BTS 0000255 | let Gif / apng can be embedded in balloon, like PNG with \\_b | 要望 | 却下 | 要望 | 未着手 | `ledger/sakura-script.toml` の `\_b[ファイルパス,x,y,オプション,オプション...]`（`absent`・持ち主なし。文中に置く形は roadmap の覚え書きに登記あり）。動く GIF は完了した `areka-P0-animated-image-decode` が非対応と決めている（APNG は読める） | バルーンに画像を置く `\_b` で、PNG と同じように動く GIF や APNG も置けるようにしてほしいという要望。作者は作るのが重すぎるとして断った。 |
| BTS 0000278 | Allow input boxes to pass additional reference values | 要望 | 実装済み（2.5.25） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,inputbox,ID,表示時間,テキスト,オプション,...]`（`absent`）。入力ボックスを受け持つ `areka-P0-inputbox-user-input` は、brief で `--reference=` を範囲から外している | 入力ボックスを開くときに追加の情報を添え、入力の結果と一緒に Reference で受け取れるようにしてほしいという要望。SSP は 2.5.25 で、入力ボックスを開くタグに `--reference=` を足した。 |
| BTS 0000306 | Let input boxes have semi-transparency | 要望 | 実装済み（2.5.37） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,inputbox,ID,表示時間,テキスト,オプション,...]`（`absent`）。入力ボックスを受け持つ `areka-P0-inputbox-user-input` は、brief でバルーンの側の入力ボックスの見た目の指定を範囲から外している | 入力ボックスの画像でも半透明を使えるようにし、縁を滑らかに描けるようにしてほしいという要望。作者は OS の入力欄の部品が透過の窓に乗らないためだと説明したうえで、2.5.37 で入力欄の周りを除く枠だけ半透明にできるようにした。 |
| BTS 0000307 | Let input boxes default to balloonc0 if no image is present for them | 要望 | 実装済み（2.5.34） | 要望 | 未着手 | 入力ボックスの見た目は、`areka-P0-inputbox-user-input` が brief で範囲から外している。添えられた「相方のバルーンの画像が無ければ本体側のものを使う」は完了した `areka-P0-kero-balloon` が作った（`doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」） | 入力ボックスの種類ごとの画像が無いときは 1 枚目の画像を使い回し、同じ絵を何枚も置かずに済むようにしてほしいという要望（相方のバルーンの画像が無ければ本体側のものを使う案も添えられた）。SSP は 2.5.34 で入れた。 |
| BTS 0000324 | Add an option to change the color of the font and ok/cancel buttons for input boxes | 要望 | 実装済み（2.5.36） | 要望 | 未着手 | `ledger/assets.toml` の `communicatebox.font.color.r,数値`（`absent`・持ち主なし）。入力ボックスを受け持つ `areka-P0-inputbox-user-input` は、brief で見た目の指定を範囲から外している | 入力ボックスの文字と OK・キャンセルのボタンの色がバルーンの文字色から決まって読みにくいことがあるので、入力ボックスだけの色を指定できるようにしてほしいという要望。SSP は 2.5.36 で `communicatebox.font.color.r,数値` などを足した。 |
| BTS 0000338 | About cursor customisation of text input boxes | 要望 | 実装済み（2.5.40） | 要望 | 未着手 | `ledger/assets.toml` の `mousecursor.text,ファイル名`（`absent`・持ち主なし） | マウスカーソルの差し替えを、文字の入力欄の上やドラッグの最中など場面ごとにも指定できるようにしてほしいという要望。SSP は 2.5.40 で `mousecursor.text,ファイル名` を足し、ドラッグの最中のカーソルは OS の制約で替えられないと答えた。 |
| BTS 0000366 | Allow balloon arrows/online marker/etc to extend off the base image | 要望 | 実装済み（2.5.54） | 要望 | 未着手 | `ledger/assets.toml` の `overlay_outside_balloon,数値`（`absent`・持ち主なし） | バルーンの矢印やオンラインマーカーを、バルーンの画像の透明な所へ浮かせて置いても見えるようにしてほしいという要望。SSP は 2.5.54 で `overlay_outside_balloon,数値` を足した。 |
| BTS 0000382 | Let alternate arrow/marker/online marker files be specified in balloon(s/k)*s.txt files | 要望 | 実装済み（2.5.51） | 要望 | brief あり | `ledger/assets.toml` の `arrow.filename,ファイル名`・`onlinemarker.filename,ファイル名`・`sstpmarker.filename,ファイル名`（どれも `absent`・持ち主は `areka-P0-balloon-canon-residue`）。印の画像の系列は進行中の `areka-P0-balloon-markers` の brief が引き取っている | キャラクターごとやバルーンの大きさごとに矢印やオンラインマーカーの絵を替えられるよう、バルーン別の設定ファイルで別の画像を指定できるようにしてほしいという要望。SSP は 2.5.51 で入れた。 |
| BTS 0000402 | Allow the developer to specify what balloon to use for input boxes | 要望 | 実装済み（2.6.23） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,inputbox,ID,表示時間,テキスト,オプション,...]`（`absent`）。入力ボックスを受け持つ `areka-P0-inputbox-user-input` は、brief でバルーンの側の入力ボックスの見た目の指定を範囲から外している | 入力ボックスを開くタグのオプションで、使う入力ボックスの画像を番号で選べるようにし、画像ごとの設定ファイルも持てるようにしてほしいという要望。SSP は 2.6.23 で入れた。 |
| BTS 0000446 | Add option to adjust the speed of the balloon's online marker animation | 要望 | 実装済み（2.5.77） | 要望 | brief あり | `ledger/assets.toml` の `onlinemarker.interval,待機時間`（`absent`・持ち主は `areka-P0-balloon-canon-residue`）。通信中の印は進行中の `areka-P0-balloon-markers` の brief が引き取っている | オンラインマーカーの動く絵のこま送りの間隔を、バルーンの descript で決められるようにしてほしいという要望。SSP は 2.5.77 で `onlinemarker.interval,待機時間` を足した。 |
| BTS 0000567 | "Sticky" balloon tags | 要望 | 実装済み（2.6） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\b[ID番号]`（`implemented`・`areka-P0-kero-balloon`）。無い番号のときの代わりを指すオプションはカタログに項目なし | `\b[ID番号]` で指した番号のバルーンが無いとき、既定の小さいバルーンでなく、台本で決めた別の番号へ落ちるようにしてほしいという要望。SSP は `\b[ID番号]` に代わりの番号を並べるオプションを足した。 |
| BTS 0000804 | バルーンのsstpmessage.ybの挙動について | マイナー | 実装済み（2.8） | — | — | — | （未読） |

### シェルとサーフェス

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000020 | 【要望】2D/3DCGを扱う | 要望 | 却下 | 要望 | 未着手 | カタログに項目なし | 画像の重ね合わせでなく、2D や 3D のモデルとキーフレームのデータでキャラクターを描けるようにし、描画を別のモジュールへ差し替えられるようにしてほしいという要望。SSP は作りの上で難しいとして却下した。 |
| BTS 0000035 | シェル倍率を変えるとジャギるようになった | 要望 | 仕様どおり | 挙動の説明 | 未着手 | `ledger/sakura-script.toml` の `\![set,scaling,倍率]`（`absent`・持ち主の `areka-P0-sakura-time-directives` は brief で範囲から外している） | 倍率を変えたシェルがぎざぎざに見えるという報告に、拡大縮小のときの補完処理の設定を入れれば滑らかになると作者が案内し、既定で入っていたとして閉じた。areka はシェルの倍率の変更をまだ持たない。 |
| BTS 0000038 | 拡大/縮小時の補完処理の「シェル個別設定」がほしい。 | 要望 | 実装済み（2.5） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![set,scaling,倍率]`（`absent`・持ち主の `areka-P0-sakura-time-directives` は brief で範囲から外している） | 拡大縮小のときの補完処理を入れるかどうかを、シェルごとに覚えてほしいという要望。SSP はもともとシェルごとの設定で、2.5 で画面にそう明記した。 |
| BTS 0000173 | Make Stop and Start methods able to handle multiple intervals at once (or add a new method for this) | 要望 | 実装済み（2.5.06） | 要望 | 未着手 | `ledger/assets.toml` の `parallelstop,(ID1,ID2...)`（`absent`・持ち主なし） | アニメーションの pattern から、複数のアニメーションをまとめて止めたり始めたりできるようにしてほしいという要望。SSP は 2.5.06 で `parallelstop,(ID1,ID2...)` を足した。 |
| BTS 0000183 | allow shells to draw windows with it's own DLL | 要望 | 却下 | 要望 | 未着手 | カタログに項目なし | ゴーストが SHIORI の DLL を持つのと同じように、シェルが自前の DLL で窓を描けるようにしてほしいという要望。SSP は作りの上で難しいとして却下した（BTS 0000020 と同じ話として結ばれている）。 |
| BTS 0000301 | Allow balloon and shell to have custom dlls to draw the interface | 要望 | 未対応 | 要望 | 未着手 | カタログに項目なし | Live2D や 3D のモデルを使えるよう、本体の描画を差し替えられる部品にして、シェルやバルーンが自前の DLL で描けるようにしてほしいという要望（BTS 0000183 の続き）。SSP は未対応のまま（作者の返事は無い）。 |
| BTS 0000372 | Allow surfaces to have non-numerical names | 要望 | 実装済み（2.8.24） | 要望 | 未着手 | `ledger/assets.toml` の `name,定義名`（`absent`・持ち主なし） | サーフェスを数字でなく名前で定義できるようにし、番号の範囲を割り振る手間を無くしたいという要望。SSP は互換の問題を避けて、2.8.24 でサーフェスの定義に `name,定義名` を足す形で入れた。 |
| BTS 0000601 | kero.seriko.defaultsurfaceがOnWindowStateRestore を用意していない場合に機能していない。 | 些細 | 仕様どおり | — | — | — | （未読） |
| BTS 0000630 | シェルの拡大縮小アルゴリズムについて | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000762 | Prevent looping of non-repeating .GIF surfaces | 微調整 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000785 | Optional name attribute for animations | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000792 | SSP組み込みクロスフェード機能を強制OFFにするdescript.txt設定がほしい | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000795 | How to get surface width before the surface is displayed | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### SHIORI イベント・リソース・プロパティシステム

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000166 | some events have no docs | マイナー | 実装済み（2.5.03） | 要望 | 対象外 | ukadoc の記載の話（名指しされた `OnHouchi` は SAORI のイベントで、ベースウェアが送るものではない） | ukadoc に載っていないイベントが届くので説明を足してほしいという要望。例に挙がった `OnHouchi` は SAORI のイベントが SSP の不具合で常に送られていたもので、2.5.03 で送らないように直された。 |
| BTS 0000234 | new event to notify ghost what language is the user using | 要望 | 実装済み（2.5.16） | 要望 | 未着手 | `ledger/shiori.toml` の `OnNotifyInternationalInfo`・`OnLanguageChange`（どちらも `absent`・持ち主なし）。SSP がどのイベントで応えたかは頁に書かれていない | 多言語に対応したゴーストのために、利用者が使っている言語を知らせるイベントがほしいという要望。SSP は 2.5.16 で入れた。 |
| BTS 0000262 | BaseIDが来ない | マイナー | 仕様どおり | 挙動の説明 | 未着手 | `ledger/shiori.toml` の `BaseID [SSP拡張]`（`absent`・持ち主なし。areka はこのヘッダを送っていない） | `OnGhostCalled` に何も返さなくても、BaseID に挙がったイベントが続けて届くわけではないと作者が説明した。BaseID は、SHIORI の側が返事を流用できそうなイベントの名前を知らせるだけのものである。 |
| BTS 0000286 | Allow OnSoundStop to be called as GET while ghost is minimized | 要望 | 実装済み（2.5.35） | 要望 | 未着手 | `ledger/shiori.toml` の `ValueNotify [SSP拡張 2.5.35]`・`OnSoundStop`（どちらも `absent`・持ち主なし） | 最小化の間は `OnSoundStop` が NOTIFY で届き、次の曲を台本で始められないので、GET で届けてほしいという要望。SSP は 2.5.35 で、NOTIFY の応答でも台本を返せる試験的なヘッダ `ValueNotify [SSP拡張 2.5.35]` を足して応えた。 |
| BTS 0000288 | New event: rate of use graph within one week | 要望 | 実装済み（2.8.54） | 要望 | brief あり | `ledger/property.toml` の `rateofuselist(名前).bootminuteweekly` ほか週ごと・月ごとの値（`vocabulary-only`）・`areka-P0-property-catalog-lists`。SSP が何で応えたかは頁に書かれていない | 使用頻度の総計だけでなく、直近の 1 週間や 1 か月の使われ方をゴーストが知れるようにしてほしいという要望。SSP は 2.8.54 で入れた。 |
| BTS 0000310 | Give OnMouseDragEnd a reference value for if the sakura and kero are overlapping | 要望 | 実装済み（2.5.36） | 要望 | 未着手 | `ledger/shiori.toml` の `OnOverlap`（`absent`・持ち主なし）。`OnMouseDragEnd` そのものは `areka-P0-mouse-drag-events` が作った | キャラクター同士を重ねるドラッグへの反応を作りやすいよう、`OnMouseDragEnd` に重なっているかの Reference を足してほしいという要望。SSP は Reference を足さず、2.5.36 で `OnOverlap` と `OnOffscreen` の届く時機を直した。 |
| BTS 0000336 | Detect cursor/other ghost position | 要望 | 実装済み（2.5.43） | 要望 | brief あり | `ledger/property.toml` の `system.cursor.pos`・`activeghostlist(ゴースト名/本体側名/パス).汎用プロパティ名`（どちらも `vocabulary-only`）・`areka-P0-property-catalog-lists` | 画面の上のマウスカーソルの位置と、ほかのゴーストの位置を、ゴーストが知れるようにしてほしいという要望。SSP は 2.5.43 でプロパティシステムに `system.cursor.pos` を足し、ほかのゴーストの位置は今あるプロパティで引けると案内した。 |
| BTS 0000492 | Add option to set cursor for entire ghost with property system | 要望 | 実装済み（2.6.11） | 要望 | 未着手 | `ledger/property.toml` の `currentghost.mousecursor`（`vocabulary-only`。値の保持と GET は進行中の `areka-P0-currentghost-property-others` の brief が受け持つ）。カーソルを実際に差し替える `ledger/assets.toml` の `cursor,ファイル名 / mousecursor,ファイル名` は `absent`・持ち主なし | 利用者が入り切りできる自作のマウスカーソルをバルーンの上にも効かせられるよう、ゴースト全体のカーソルをプロパティで替えたいという要望。SSP は 2.6.11 で `currentghost.mousecursor` を足した。 |
| BTS 0000540 | OnDisplayChange parameter has error | マイナー | 却下 | 挙動の説明 | 未着手 | `ledger/shiori.toml` の `OnDisplayChange`（`absent`・持ち主なし）。areka は画面の拡大率に追従する作り（完了した `areka-P0-emo-dpi-scaling`）だが、このイベントで送る寸法は決めていない | `OnDisplayChange` の画面の寸法が OS の設定と合わないのは、SSP が画面の拡大率に対応しておらず、OS が拡大率で割った寸法を返すためだと作者が説明した（直せないとして閉じた）。 |
| BTS 0000551 | Notification of whether the current network will be charged in network information notification event | 要望 | 実装済み（2.6.15） | 要望 | 未着手 | `ledger/shiori.toml` の `OnNetworkStatusChange`（`absent`・持ち主なし）。更新の確かめの結果を知らせる `OnUpdateCheckResult` などは `areka-P0-update-check-options` が受け持つ | 通信量を食う処理をゴーストが控えられるよう、今の回線が従量課金かどうかをネットワークの状態のイベントで知らせ、更新の確かめでは取得する大きさも知らせてほしいという要望。SSP は 2.6.15 で入れた。 |
| BTS 0000584 | RecycleBin related update | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000593 | OnNetworkStatusChangeに置いて有線ＬＡＮ接続なのにReference2が「other」となる。 | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000616 | ゴーストやプラグインにおけるOnOtherGhostTalkイベントでNoContentなイベントも補足できるようにしてほしい | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000728 | OnUserInput occurs when inputbox times out | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000791 | プロパティシステムでバルーンの現在位置座標を取得したい | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### 窓と配置

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000023 | 吹き出しとシェルを前面に表示 | 要望 | 未対応 | 要望 | 未着手 | 重なり順を台本と設定で固定する形は `areka-P0-scope-zorder-pinning` が作った。喋る側を自動で手前へ出す切り替えはカタログに項目なし | キャラクターが 3 体以上いるゴーストで、喋っているキャラクターとそのバルーンを他より手前に出す選択肢がほしいという要望。SSP は未対応のまま。 |
| BTS 0000162 | デスクトップ解像度変更時の挙動 | 表示 | 実装済み（2.7） | 要望 | 未着手 | 起動のときに保存した位置が画面の外なら画面の中へ出す形は `areka-P0-position-persist` が作った。動いている間に解像度が変わったときの位置の扱いは決めていない | 画面の解像度を下げてから元に戻すと、キャラクターの位置が元に戻らないという報告。作者は画面の外へ出て操作できなくなるのを避ける仕様だと答えたが、後に手を入れたとして 2.7 で閉じた（解決状況の欄は差し戻しのまま）。 |
| BTS 0000355 | Allow user to specify the distance at which ghosts should 'stick' to the taskbar/screen edge | 要望 | 実装済み（2.8.55） | 要望 | 未着手 | カタログに項目なし。ドラッグの最中に画面の端へ吸い付かせる決まりは、areka の資料に見つからない | キャラクターを動かすとき画面の端やタスクバーへ吸い付く距離を設定で決められ、0 にすれば吸い付きを止められるようにしてほしいという要望。SSP は 2.8.55 で入れた。 |
| BTS 0000463 | バルーンをシェルより前面に表示する設定がほしい。（2.5.34での仕様変更による） | 要望 | 却下 | 要望 | 未着手 | 相方のバルーンを本体のキャラクターより手前に置く並びは、台本と設定から `areka-P0-scope-zorder-pinning` で固定できる。喋っている側のバルーンを自動で手前へ出す設定はカタログに項目なし | 相方のバルーンが本体のキャラクターの下に隠れて読めないので、喋っているバルーンを他より手前へ出すか、バルーンを常にシェルより手前に置く設定がほしいという要望。作者は OS の窓の仕組みの上で無理だとして断った。 |
| BTS 0000598 | New z-order options | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000609 | 他ウィンドウが全て最小化されていると、SSPが最小化される場合がある。 | マイナー | 却下 | — | — | — | （未読） |
| BTS 0000626 | 全ての仮想デスクトップに表示される | マイナー | 却下 | — | — | — | （未読） |
| BTS 0000653 | タブレットPCで縦長表示にして画面外判定になったあと元の横長表示に戻しても画面外判定が消えない | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000677 | ゴーストの立ち位置をデスクトップの相対位置に対応してほしい | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000723 | DisplayPort接続時に画面の電源を切るとゴーストの位置が動かされる | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000724 | 以前のsspと \![move]における移動距離が大きく異なっている問題。 | メジャー | 仕様どおり | — | — | — | （未読） |
| BTS 0000730 | さくらスクリプトの\![set,zorder,ID,ID～]の効果のバルーン適用要望 | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000733 | Sakurascript tag to reset balloon position (like \![execute,resetwindowpos]) | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000776 | 起動時にゴーストを最背面で起動する。 | 要望 | 未対応 | — | — | — | （未読） |

### メニュー・設定・操作

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000036 | 【要望】SSPエクスプローラの挙動 | 要望 | 未対応 | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,ghostexplorer]`（`absent`・持ち主なし） | ゴーストエクスプローラで並び順を自分で決められるようにし、起動と終了のボタンも足してほしいという要望で、お気に入りの案も添えられている。SSP は未対応のまま。 |
| BTS 0000163 | about thumbnail.png big than screen | 些細 | 実装済み（2.7.86） | 要望 | 未着手 | カタログに項目なし（`thumbnail.png` の表示の大きさ） | `thumbnail.png` が画面より大きいと画面の外へはみ出して表示されるので、出す位置や大きさを設定で決められるようにしてほしいという要望。SSP は 2.7.86 で、大きな画像も許すが画面の広さの 4 分の 1 未満に収める形にした。 |
| BTS 0000227 | asking for some basic settings when SSP runs for the first time | 要望 | 実装済み（2.7.72） | 要望 | 未着手 | カタログに項目なし。設定画面とインストーラー版は roadmap の予約に登記あり | 初めて起動したときに、ファイルの関連付けやユーザーの名前など、たいていの人が変える設定だけを尋ねる初期設定がほしいという要望（言語パックや最初のゴーストを選んで取ってくる案も添えられた）。SSP は 2.7.72 でインストーラーと初期設定を作った。 |
| BTS 0000291 | Add a sakurascript and a button in the owner draw menu to view terms.txt again | 要望 | 実装済み（2.5.33） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,terms]`（`absent`・持ち主なし。完了した `areka-P0-ghost-install` は範囲から外している） | インストールの後からでも利用条件を開き直せるタグとメニューの項目がほしいという要望。SSP は 2.5.33 で `\![open,terms]` を足し、利用条件が変わるたびに自動で出す案は利用者の邪魔になるとして採らなかった。 |
| BTS 0000315 | Make the readme dialog support ctrl+A select all | 要望 | 実装済み（2.5.35） | 要望 | 対象外 | areka は readme を OS の既定のアプリで開き、自前の窓を持たない（完了した `areka-P0-popup-menu-minimal`）ので、意味を持たない | readme を翻訳ソフトにかけたいので、readme を出す窓で Ctrl+A の全選択を使えるようにしてほしいという要望。SSP は 2.5.35 で入れた。 |
| BTS 0000327 | Add an option to tile the owner draw menu image | 要望 | 実装済み（2.5.39） | 要望 | 未着手 | `ledger/assets.toml` の `menu.background.alignment,位置`（`absent`・持ち主なし）。オーナードローのメニューは roadmap の予約に登記あり（「生きている決まり」6 が、メニューは Win32 標準・オーナードローは予約のままと決めている） | オーナードローメニューは項目の数で大きさが変わるので、背景の画像を途中で切らずに並べて敷き詰められる指定がほしいという要望。SSP は 2.5.39 で、`menu.background.alignment,位置` などに縦と横の繰り返しの指定を足した。 |
| BTS 0000331 | \! [set,trayballoon] seems to cause lag on some computers | マイナー | 却下 | 挙動の説明 | 未着手 | `ledger/sakura-script.toml` の `\![set,trayballoon,オプション,オプション,オプション...]`（`absent`・持ち主なし）。トレイアイコンは roadmap の予約に登記あり | `\![set,trayballoon,オプション,オプション,オプション...]` の知らせが遅れて出るのは Windows の通知の仕様で、遅れを無くす指定は通知が捨てられることがあるので使わない、と作者が説明した。 |
| BTS 0000337 | Use the busy mouse icon when shiori doesn't return for a long time instead of just getting stuck | 要望 | 実装済み（2.5.43） | 要望 | 未着手 | `ledger/assets.toml` の `mousecursor.wait,ファイル名`（`absent`・持ち主なし） | SHIORI の返事に時間がかかる間、ただ固まって見えるのでなく、マウスカーソルを待ちの形に変えてほしいという要望。SSP は 2.5.43 で入れ、待ちのカーソルを差し替える `mousecursor.wait,ファイル名` も足した。 |
| BTS 0000345 | mousecursor / mousecursor.text for ghost's descript.txt & terms.txt in ghost dir | 微調整 | 実装済み（2.5.41） | 要望 | 未着手 | `ledger/assets.toml` の `cursor,ファイル名 / mousecursor,ファイル名`・`mousecursor.text,ファイル名`（ゴーストの descript の側・どちらも `absent`・持ち主なし） | マウスカーソルを差し替える指定をゴーストの descript にも書けるようにすることと、`terms.txt` をゴーストのフォルダにも置けるようにすることを、題だけで求めた要望（本文に細かい説明は無い）。SSP は 2.5.41 で入れた。 |
| BTS 0000347 | About the change to the owner draw menu... | 些細 | 実装済み（2.5.41） | 要望 | 未着手 | `ledger/assets.toml` の `menu.frame.color.r,数値`（`absent`・持ち主なし）。オーナードローのメニューは roadmap の予約に登記あり（「生きている決まり」6 が、メニューは Win32 標準・オーナードローは予約のままと決めている） | オーナードローメニューの枠線の色が版の更新で黒から白っぽい色に変わったので、元に戻すか、ゴーストの側で色を決められるようにしてほしいという要望。SSP は 2.5.41 で既定を黒にし、`menu.frame.color.r,数値` などで指定できるようにした。 |
| BTS 0000359 | Add a right click to "Add to computer boot-up" in ghost browser | 要望 | 実装済み（2.5.86） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![create,shortcut]`・`\![open,ghostexplorer]`（どちらも `absent`・持ち主なし）。OS の起動のときに立ち上げる登録はカタログに項目なし | ゴーストエクスプローラの右クリックから、そのゴーストを PC の起動のときに立ち上げる登録やデスクトップのショートカット作りをできるようにし、台本から勧めるタグも足してほしいという要望。SSP は 2.5.86 で入れた。 |
| BTS 0000362 | Give tooltips multiple language support | 要望 | 実装済み（2.5.46） | 要望 | brief あり | `ledger/property.toml` の `currentghost.seriko.tooltip.scope(ID).textlist(当たり判定名).text`（`vocabulary-only`・`areka-P0-currentghost-property-tree`）。ツールチップを出す側は `areka-P0-shell-tooltip` | プロパティで差し替えたツールチップの文字が、OS のロケールに無い言語だと「?」に化けるので、どの言語でも出せるようにしてほしいという要望。SSP は 2.5.46 で直した。 |
| BTS 0000390 | Icon animation support and runtime modification of the animation's cycle time | 要望 | 実装済み（2.5.58） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`（`absent`・持ち主なし）。トレイアイコンは roadmap の予約に登記あり | タスクトレイのアイコンを本体の働きで動く絵にでき、その速さ（とシェルのアニメーションの速さ）を台本から変えられるようにしてほしいという要望。SSP は 2.5.58 で、アイコンを替えるタグに動く絵の指定を足した。 |
| BTS 0000407 | さくらスクリプト \![set,trayballoon,～] でタスクトレイからバルーンが出ない | 些細 | 仕様どおり | 挙動の説明 | 未着手 | `ledger/sakura-script.toml` の `\![set,trayballoon,オプション,オプション,オプション...]`（`absent`・持ち主なし）。トレイアイコンは roadmap の予約に登記あり | `\![set,trayballoon,オプション,オプション,オプション...]` の知らせが出ないという報告に、本文のオプションは必須で、Windows の通知の設定が切られていると出ないと作者が説明した。 |
| BTS 0000416 | Icon animation but only run once | 要望 | 実装済み（2.5.59） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`（`absent`・持ち主なし）。トレイアイコンは roadmap の予約に登記あり | タスクトレイのアイコンの動く絵を、繰り返さずに 1 回だけ再生できる指定がほしいという要望。SSP は 2.5.59 で、アイコンを替えるタグに繰り返しの回数のオプションを足した。 |
| BTS 0000418 | Some suggestions for icon animation | 要望 | 未対応 | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![set,tasktrayicon,ファイル名.ico,テキスト(,--duration=待機時間(,--runcount=繰り返し回数))]`（`absent`・持ち主なし）。トレイアイコンは roadmap の予約に登記あり | 台本で毎秒アイコンの絵を動かすと `OnSurfaceRestore` が届かなくなるので、アイコンの絵にもシェルと同じ無作為の再生を持たせ、こま割りを文字で書けるようにしてほしいという要望。SSP は未対応のまま。 |
| BTS 0000429 | スクリプトログの挙動について | 要望 | 実装済み（2.5.70） | 要望 | 対象外 | SSP 固有の画面（スクリプトログ）の選択とスクロールの動きの話。areka はこの画面を持たず、台本の記録は MCP の `get_log`（`areka-P0-mcp-log-history`）で引く | 開発者向けのスクリプトログで、新しいトークが足されるたびに選択の位置がずれ、表示が先頭へ戻るので、以前の動きに戻してほしいという要望。SSP は 2.5.70 で直した。 |
| BTS 0000526 | Add option to disable user skipping text | 要望 | 未対応 | 要望 | 未着手 | カタログに項目なし。利用者の早送りそのものは進行中の `areka-P0-talk-fast-forward` が作るが、台本から早送りを止める指定は brief に無い | 間合いが大事な場面のために、利用者による台詞の早送りを、台本の指定した区間だけ止められるタグがほしいという要望。SSP は未対応のまま。 |
| BTS 0000533 | Widen TimeMachine window | 些細 | 実装済み（2.6.14） | 要望 | 対象外 | SSP 固有の画面（時刻を仮に変える開発用の窓）の見た目の話 | 時刻を仮に変える窓の欄が狭く、選んだ時刻の文字が切れて読み違えるので、窓と欄を広げてほしいという要望。SSP は 2.6.14 で直した。 |
| BTS 0000537 | There is no argument for opening the speech tab of the SSP preferences | マイナー | 実装済み（2.6.15） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,configurationdialog,ダイアログID]`（`absent`・持ち主なし）。設定画面は roadmap の予約に登記あり | `\![open,configurationdialog,ダイアログID]` に、設定画面の音声の頁を開く ID が無いので足してほしいという要望。SSP は入れ忘れだったとして 2.6.15 で足した。 |
| BTS 0000561 | About the trayballoon timeout | マイナー | 実装済み（2.6.19） | 挙動の説明 | 未着手 | `ledger/shiori.toml` の `OnTrayBalloonTimeout`（`absent`・持ち主なし）。トレイアイコンは roadmap の予約に登記あり | タスクトレイの知らせが消える前に時間切れの台詞が始まるのは、OS が時間切れを窓の消えるより早く知らせてくるためだと作者が説明した。SSP は 2.6.19 で話し始めを 1 秒遅らせて和らげた。 |
| BTS 0000603 | Change SSP's ordering to word-by-word alphabetization | 微調整 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000670 | Add confirmation dialog to "delete" option in Ghost Explorer | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000671 | kero.popupmenu.typeを0に指定した際に着せ替えを\1側に変更する手段が欲しい。 | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000678 | \0\s[-1]となっている状態で、sspのショートカット機能が機能しない問題。 | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000685 | 右ダブルクリックで強制表示されるオーナードローメニューの制御 | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000709 | ゴーストが自由にいじれるスケジュール領域 | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000748 | 着せ替えエクスプローラの並び順を変更したい | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000768 | Separate the toggle for turning off menu thumbnails | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000773 | マルチディスプレイ環境での本体設定 | 微調整 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000793 | バックログの非表示対象を追加して欲しい | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000794 | How to handle cipher fonts in the backlog display | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000803 | スクリプトログの挙動について | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### インストール・配布・削除

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000115 | 削除処理のOnVanishSelected以降をファイルを消さずに実行する機能 | 要望 | 実装済み（2.5） | 要望 | 未着手 | `ledger/shiori.toml` の `OnVanishSelected`（`absent`・持ち主なし）。roadmap の予約に登記あり（消滅） | 消滅の流れを確かめるために、`OnVanishSelected` より後の手順をファイルを消さずに最後まで動かせるようにしてほしいという要望。SSP は 2.5 で入れた。 |
| BTS 0000167 | some nar can't install now | マイナー | 仕様どおり | 挙動の説明 | 実装済み | `areka-P0-nar-install`（無圧縮と deflate だけを受け、ほかの圧縮方式は理由を付けて断る） | zip の入れ物に LZMA で圧縮した nar はインストールできず、圧縮の指定を外して作り直すようにと作者が説明した（SSP は後の 2.5.20 から LZMA も読める）。areka は対応しない圧縮方式の nar を、理由を付けて丸ごと断ると決めている。 |
| BTS 0000179 | Some methods of compressing the size of install package | 些細 | 実装済み（版は不明） | 要望 | 対象外 | SSP の配布物の作りの話 | SSP の配布物を小さくするために、同梱の画像を圧縮し直し、実行ファイルの圧縮をやめてはどうかという提案。作者が配布物を作り直して閉じた。 |
| BTS 0000190 | let nar file support password and 7-zip | 要望 | 実装済み（2.5.08） | 要望 | 方針とぶつかる | 完了した `areka-P0-nar-install` の要件「Requirement 2: NAR コンテナの読取とファイル名の文字コード」が、暗号化されたエントリと対応しない圧縮方式の nar を丸ごと断ると決めている | nar にパスワードを掛けられるようにし、7-Zip の形式も読めるようにしてほしいという要望。SSP は 2.5.08 で zip のパスワードだけを入れ、7-Zip は権利と手間の事情で見送った。 |
| BTS 0000221 | Some improvements on NAR password | マイナー | 実装済み（2.5.10） | 要望 | 方針とぶつかる | 完了した `areka-P0-nar-install` の要件「Requirement 2: NAR コンテナの読取とファイル名の文字コード」が、暗号化されたエントリと対応しない圧縮方式の nar を丸ごと断ると決めている | パスワード付きの nar の扱い（対応しない圧縮方式でもパスワードを尋ねる・入力をやめると誤りの警告になる・`OnInstallFailure` がパスワード違いを区別しない）を直してほしいという要望。SSP は 2.5.10 までに直した（解決状況の欄は差し戻しのまま）。 |
| BTS 0000243 | \![execute,createnar,filename] | 要望 | 却下 | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![execute,createnar]`（`absent`・持ち主なし）。roadmap の予約に登記あり（`\![execute,createnar]`／`createupdatedata`） | `\![execute,createnar]` に書き出し先のファイル名を引数で渡し、毎回手で選ばずに済むようにしてほしいという要望。作者は、ゴーストが利用者の設定なしに書き出し先を決められるのは安全の上で危ないとして断り、設定画面の書き出し先の指定を使うよう案内した。 |
| BTS 0000252 | Let the NAR file can have its own icon | 要望 | 却下 | 要望 | 未着手 | カタログに項目なし | nar のファイルごとに、エクスプローラで中身に応じたアイコンや縮小画像を見せられるようにしてほしいという要望。作者は、技術的にはできるが OS へ部品を登録することになるので避けたいとして断った。 |
| BTS 0000253 | narpng | 要望 | 未対応 | 要望 | 未着手 | カタログに項目なし | PNG の画像の後ろに nar（または配布元の URL）を埋め込み、絵のままゴーストを配ってインストールできるようにしてほしいという要望。SSP は未対応のまま（作者の返事は無い）。 |
| BTS 0000254 | x-ukagaka-link: install from homeurl | 要望 | 実装済み（2.5.76） | 要望 | 未着手 | `ledger/shiori.toml` の `x-ukagaka-link:type=homeurl&url=(エンコード済URL)`（`absent`・持ち主なし）。台本から同じことをする `\![execute,install,url,URL,(feed\|nar\|homeurlのいずれか)]` は `areka-P0-network-update` が作った | nar を配らなくても、Web の頁のリンクから配布元の URL を渡してゴーストをインストールできる形がほしいという要望。SSP は 2.5.76 で `x-ukagaka-link:type=homeurl&url=(エンコード済URL)` を入れた。 |
| BTS 0000261 | user terms file support for NAR | 要望 | 実装済み（2.5.27） | 要望 | 実装済み | `areka-P0-ghost-install`（展開の前に `terms.txt` を出して受諾か拒否かを選ばせる）。`ledger/shiori.toml` の `OnGhostTermsAccept`（`implemented`） | 利用者が注意書きを読んで承知してからでないとゴーストを使えないよう、nar に利用条件のファイルを持たせ、中身が変わったら更新のときにも知らせてほしいという要望。SSP は 2.5.27 で、ゴーストのルートに置く `terms.txt` を入れた。 |
| BTS 0000354 | Modify or add a new uninstall mode to allow ghost to do something after it has been uninstalled | 要望 | 却下 | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![vanishbymyself]`（`absent`・持ち主なし）。消滅は roadmap の予約に登記あり。消した後もゴーストを動かし続ける形はカタログに項目なし | 消滅させた後もゴーストをしばらく動かし続けられる消し方を足してほしいという要望（消えた後に PC を落とす演出をしたいという動機）。作者は、消した後に何かを残して動くのはマルウェアと同じ振る舞いで利用者との信頼を壊すとして、強く断った（解決状況の欄は差し戻しのまま）。 |
| BTS 0000395 | x-ukagaka-link: install from nar url | 要望 | 実装済み（2.5.58） | 要望 | 未着手 | `ledger/shiori.toml` の `x-ukagaka-link:type=install&url=(エンコード済URL)`（`absent`・持ち主なし）。台本から同じことをする `\![execute,install,url,URL,(feed\|nar\|homeurlのいずれか)]` は `areka-P0-network-update` が作った | nar を手で落として開く代わりに、Web の頁のリンクを押すだけで nar の URL からインストールできる形がほしいという要望。SSP は 2.5.58 で入れた。 |
| BTS 0000438 | "同じ処理を次以降の項目にも適用"チェックを キャンセルボタンでも有効にして欲しい | 些細 | 実装済み（2.5.73） | 要望 | 未着手 | 複数のゴーストを入れた書庫（`install.txt` の `type,package`）は roadmap の覚え書きに登記あり（今は理由を付けて断る） | 複数のゴーストを入れた書庫のインストールで、入れ先を尋ねる窓の「以降の項目にも同じ処理をする」チェックを、キャンセルにも効かせてほしいという要望。SSP は 2.5.73 で入れた。 |
| BTS 0000455 | definition of deleteX.txt | 要望 | 実装済み（2.5.80） | 要望 | 実装済み | `areka-P0-update-engine`（`delete.txt` と番号付きの `delete[数字].txt` を番号の順に読んで適用する） | 古い版のための削除の指定は変わらないので、`delete.txt` を番号付きの複数のファイルに分け、更新のたびに全部を取り直さずに済むようにしてほしいという要望。SSP は 2.5.80 で入れた。 |
| BTS 0000457 | New sakura script to open the uninstall confirmation dialog | 要望 | 実装済み（2.5.82） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![vanishbymyself]`（`absent`・持ち主なし）。消滅は roadmap の予約に登記あり（「生きている決まり」6 が、要望が出たら切ると決めている） | アンインストールの確かめの窓を台本から開けるタグがほしいという要望（`OnGhostTermsDecline` に返事が無いときの既定の動きにする案も添えられた）。SSP は 2.5.82 で入れた。 |
| BTS 0000546 | New command line parameters for installing nar | 要望 | 実装済み（2.6.15） | 要望 | 未着手 | 起動の引数から nar を入れる口はカタログに項目なし。入れた後の切替は、roadmap の「生きている決まり」7 が「areka は主導しない」と決めていて、引数で頼まれた切替がこれに当たるかは決めきれなかった | 外のプログラムから nar を入れるとき、終わったら入れたゴーストやシェルへ自動で切り替える起動の引数がほしいという要望。SSP は 2.6.15 で入れた。引数で頼まれた切替が areka の決まりに触れるかは決めきれなかった。 |
| BTS 0000557 | some final minor tweaks needed for ghost installer | 微調整 | 実装済み（2.6.17） | 要望 | 対象外 | SSP の翻訳パックの話 | 翻訳パックの後にゴーストを続けて入れると、利用条件の窓が入れたばかりの言語にならず日本語で出るので、直してほしいという要望。SSP は 2.6.17 で直した。 |
| BTS 0000570 | Allow a ghost to install multiple balloons | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000581 | バルーンのフォルダ名を変更してほしい | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000582 | Whitelist mode for delete.txt | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000798 | アーカイブビューワの動作 | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### ネットワーク更新

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000090 | ゴーストの更新ファイル作成時に、ゴーストのルートフォルダだけではなく、ghost/master 以下にも更新ファイルが作られる | マイナー | 却下 | 挙動の説明 | 未着手 | roadmap の予約に登記あり（`\![execute,createnar]`／`createupdatedata`） | 更新ファイルを作るとゴーストのルートと ghost/master の両方に同じものができるのは、他のベースウェアとの互換のためにわざと写しているからだと作者が説明した（SSP 自身は ghost/master の側を使わない）。areka は更新ファイルを作る機能をまだ持たないので、どこへ作るかも決めていない。 |
| BTS 0000114 | ネットワーク更新検証機能 | 要望 | 実装済み（2.4.91） | 要望 | 未着手 | 置き換えずに試す更新オプション `testonly` に応えるかどうかは `areka-P0-update-check-options` が決める。全部を取って結果を並べる検証の働きはカタログに項目なし | 配布元の全部のファイルを無条件に取ってハッシュを確かめ、誤りがあっても止めずに結果を全部見せる、作者向けのネットワーク更新の検証がほしいという要望。SSP は 2.4.91 で入れたが、それが更新オプションの `testonly` と同じ物かは頁からは決めきれなかった。 |
| BTS 0000120 | checkonly option for Balloon and Shell updates | 要望 | 実装済み（2.4.92） | 要望 | brief あり | `areka-P0-update-check-options` | 更新があるかを確かめるだけの更新オプション `checkonly` を、ゴーストだけでなくシェルやバルーンの更新でも使えるようにしてほしいという要望。SSP は 2.4.92 で `\![update,更新対象(,オプション,オプション...)]` に足した。 |
| BTS 0000178 | About big file update | 微調整 | 実装済み（2.5.18） | 要望 | 未着手 | 取り直しや続きからの取得は `areka-P0-update-engine` に無い。失敗した回に取れた分は作業場所ごと片付ける作り（同 spec の要件）で、次の回へ持ち越さない | ネットワーク更新で 1 つのファイルの取得に失敗すると全体が失敗になるので、取り直しや続きからの取得をし、取れた分は次の回に使えるよう残してほしいという要望。SSP は 2.5.18 で入れた。 |
| BTS 0000205 | Let ghost provide another update path to the shell | 要望 | 実装済み（2.5.33） | 要望 | brief あり | `areka-P0-update-check-options` | シェルを別配布にしても国ごとに速い配布元から更新できるよう、シェルの更新先をゴーストの側から指定できるようにしてほしいという要望。SSP はまず `\![updateother,更新対象/オプション群,...]` を足し、2.5.33 で更新先を置き換える SHIORI リソース `other_homeurl_override` を入れた。 |
| BTS 0000214 | Delete or update the update cache after ghost is repeatedly installed | マイナー | 実装済み（2.5.08） | 要望 | 対象外 | areka は更新の控えを持たず、取得が要るファイルを毎回ローカルのファイルの MD5 から導く（完了した `areka-P0-update-engine`）ので、意味を持たない | ゴーストを上書きでインストールし直しても更新の控えが古いまま残り、ネットワーク更新が働かなくなるので、控えを消すか作り直してほしいという報告。SSP は 2.5.08 で直した。 |
| BTS 0000215 | Speed up the construction of update.txt by caching the last modification time of files and folders | 要望 | 実装済み（2.5.17） | 要望 | 未着手 | roadmap の予約に登記あり（`\![execute,createnar]`／`createupdatedata`） | 更新ファイルを作るとき、前回から変わっていないファイルを更新日時で見分けて飛ばし、速く作れるようにしてほしいという要望。SSP は一度断った後、2.5.17 でハッシュの計算を省く形で一部だけ入れた（解決状況の欄は差し戻しのまま）。 |
| BTS 0000218 | add new sakurascript to build Update.dau | 要望 | 実装済み（2.5.08） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![execute,createupdatedata]`（`absent`・持ち主なし）。roadmap の予約に登記あり（`\![execute,createnar]`／`createupdatedata`） | ゴーストが自分で配布の作業を片付けられるよう、更新ファイルを作る操作を台本から始められるタグがほしいという要望。SSP は 2.5.08 で `\![execute,createupdatedata]` と `\![execute,createnar]` を足した。 |
| BTS 0000223 | Add a sakurascript tag for updating plugins | 要望 | 実装済み（2.5.22） | 要望 | 未着手 | `\![updateother,更新対象/オプション群,...]` は `areka-P0-network-update` が作ったが、プラグインを指す `--plugin=` は範囲から外している。プラグインは roadmap の予約に登記あり（Plugin／HEADLINE） | 自作のプラグインに更新があるときメニューから更新を始められるよう、プラグインのネットワーク更新を台本から始めるタグがほしいという要望。SSP は 2.5.22 で `\![updateother,更新対象/オプション群,...]` を足した。 |
| BTS 0000226 | new update modes | 要望 | 実装済み（2.5.23） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![leave,onlinemode]`（`absent`・持ち主なし）。更新の最中にゴーストがほかのやり取りに応えられるかは、資料から決めきれなかった | 何万ものファイルを更新する間ゴーストが進み具合を喋り続けて相手をできないので、黙って裏で進める更新の形がほしいという要望。SSP は 2.5.23 で、通信中にバルーンを出し続ける動きをやめて応えた。areka で更新の最中に相手ができるかは決めきれなかった。 |
| BTS 0000232 | About the code page of update.dau | マイナー | 実装済み（2.5） | 要望 | 実装済み | `areka-P0-update-engine`（更新ファイルに文字コードの指定が無ければ Shift_JIS で読み、OS のロケールを読まない） | 利用者の OS の文字コードが更新ファイルの文字コードと違うと、日本語のファイル名が化けて取得に失敗するという報告。SSP は言語の設定画面に、ファイルの読み書きに使う文字コードの設定を足した。 |
| BTS 0000241 | cache the downloaded files to avoid repeated downloads when the update fails | 要望 | 実装済み（2.5.18） | 要望 | 未着手 | 取り直しや続きからの取得は `areka-P0-update-engine` に無い。失敗した回に取れた分は作業場所ごと片付ける作り（同 spec の要件）で、次の回へ持ち越さない | 更新が途中で失敗しても、取れた分を控えておいて次の回に取り直さずに済むようにしてほしいという要望（BTS 0000178 と同じ話として結ばれている）。SSP は 2.5.18 で入れ、控えは取得から 7 日で消す形にした。 |
| BTS 0000282 | let ghost can start other update modes | 要望 | 実装済み（2.5.26） | 要望 | brief あり | `areka-P0-update-check-options`（更新オプションの `recovery` を受け持つ） | ゴーストが壊れて動かないとき、開発者でない利用者でも直せるよう、全部のファイルを確かめ直す修復の更新を台本から始められるようにしてほしいという要望。SSP は 2.5.26 で入れた。 |
| BTS 0000303 | How to trigger other_homeurl_override? | マイナー | 実装済み（2.5.34） | 要望 | brief あり | `ledger/shiori.toml` の `other_homeurl_override`（`vocabulary-only`・持ち主なし）。進行中の `areka-P0-update-check-options` の brief が受け持つ | descript に `homeurl,URL` を書いていないシェルでは `other_homeurl_override` が尋ねられず、更新先を置き換えられないという報告。SSP は 2.5.34 で、`homeurl,URL` の有無に関わらず尋ねるように直した。 |
| BTS 0000305 | Gives users the option to update ssp automatically and silently | 要望 | 実装済み（2.7.70） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![update,platform]`（`absent`・持ち主なし）。roadmap の予約に登記あり（本体の更新） | 更新の知らせを嫌う利用者のために、最小化の間など目に付かないときに本体を自動で黙って更新し、立ち上げ直す選択肢がほしいという要望。SSP は 2.7.70 で入れた。 |
| BTS 0000442 | Optimisation of network updates and homeurl override installation | 要望 | 未対応 | 要望 | 未着手 | カタログに項目なし（手元に在る同じ中身のファイルを写して取得を省く働き）。更新先を置き換える `other_homeurl_override` は `areka-P0-update-check-options` が受け持つ | ネットワーク更新で、同じ MD5 と大きさのファイルがほかのゴーストやシェルに在れば取得せずに写すことと、更新先を置き換えたインストールの手順を縮めることを求めた要望。SSP は未対応のまま（作者の返事は無い）。 |
| BTS 0000456 | Separate the time cache from the update file | 微調整 | 実装済み（2.5.82） | 要望 | 未着手 | roadmap の予約に登記あり（`\![execute,createnar]`／`createupdatedata`）。更新の側は日時を使わず MD5 で差分を導く（完了した `areka-P0-update-engine`） | 更新ファイルに入っているファイルごとの更新日時は利用者に要らないので、別のファイルへ分けてほしいという要望。作者は分ける案を採らず、2.5.82 で更新のときに日時を読み飛ばす形にした（解決状況の欄は差し戻しのまま）。 |
| BTS 0000458 | 実際の更新日時がわからなくなった | 要望 | 実装済み（2.5.83） | 要望 | 未着手 | `ledger/property.toml` の `update_time`（`vocabulary-only`・`areka-P0-property-catalog-lists`）。日時を見せる `\![open,ghostexplorer]` は `absent`・持ち主なし。更新が無かった回に日時を動かすかどうかは、資料から決めきれなかった | 更新を確かめただけで更新が無かったゴーストまで、ゴーストエクスプローラの更新日時が今日になるので、実際に更新できた日時に戻してほしいという要望。SSP は 2.5.83 で直した。areka が日時をいつ動かすかは決めきれなかった。 |
| BTS 0000513 | add a file verification to the ssp update process | メジャー | 実装済み（2.6.12） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![update,platform]`（`absent`・持ち主なし）。roadmap の予約に登記あり（本体の更新） | 本体の更新の後に実行ファイルが壊れて起動できなくなる利用者がいるので、更新の手順にファイルの検証を足してほしいという要望。SSP は 2.6.12 で、展開の前に書庫が壊れていないかを確かめるようにした。 |
| BTS 0000515 | New extension to update function | 要望 | 実装済み（2.6.23） | 要望 | 未着手 | カタログに項目なし（SSP が何で応えたかは頁に書かれていない）。更新を始めるタグと更新のイベントは `areka-P0-network-update` が作った | 決めた nar を落として入れるなど、ゴーストが更新の手順を自分で組めるよう、本体の更新を止められるタグがほしいという要望。SSP は 2.6.23 で入れた。 |
| BTS 0000683 | バルーン指定recommended.balloon.forceupdateによるネットワーク更新時の強制バルーン更新機能の提案 | マイナー | 実装済み（2.7） | — | — | — | （未読） |

### 外との連携

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000003 | POPに複数のアカウントを追加した場合の挙動について | 要望 | 実装済み（2.5） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![biff(,アカウント名)]`（`absent`・持ち主なし） | メールチェックの POP アカウントが複数あるとき、アカウントごとの件数を分けて知らせ、開くメーラーもアカウントで選べるようにしてほしいという要望。SSP は 2.5 で入れた。 |
| BTS 0000084 | ヘッドラインについて | 要望 | 実装済み（2.5.03） | 要望 | 未着手 | roadmap の予約に登記あり（Plugin／HEADLINE） | ヘッドラインの台本に本体が `\![set,choicetimeout,時間]` の 0 を差し込むので、自動の巡回のときに選択肢が時間切れにならず止まってしまう、差し込むかどうかを選べるようにしてほしいという要望。SSP は 2.5 で設定を足し、効かないという続報を 2.5.03 で直して閉じた（解決状況の欄は差し戻しのまま）。 |
| BTS 0000096 | new type sstp | 要望 | 重複 | 要望 | 未着手 | roadmap の覚え書きに登記あり（SSTP の受信）。同じ用途を MCP で果たす話は `areka-P0-mcp-shiori-query` が受け持つ | 外のソフトからゴーストへ話しかけ、SHIORI が返した中身を受け取れる SSTP の口がほしいという要望。BTS 0000137 と重複として閉じられた。 |
| BTS 0000125 | Select different text to speech voices for Sakura and Kero | 要望 | 実装済み（2.5） | 要望 | 未着手 | カタログに項目なし | 台詞の読み上げ（音声合成）の声を、本体側と相方側で別々に選べる設定がほしいという要望。SSP は 2.5 で入れた。 |
| BTS 0000137 | Expand SSTP support | 要望 | 実装済み（2.5.03） | 要望 | 未着手 | `ledger/shiori.toml` の `X-SSTP-PassThru-(任意の文字列) [SSP 2.5.03～拡張]`（`absent`・持ち主なし）。SSTP の受信は roadmap の覚え書きに登記あり。同じヘッダを MCP の答えへ通す話は `areka-P0-mcp-shiori-query` が受け持つ | SHIORI が応答に足した独自のヘッダを、SSTP の応答として外のソフトが受け取れるようにしてほしいという要望。SSP は 2.5.03 で、決まった前置きの付いたヘッダを SSTP の応答へそのまま渡す形を入れた。 |
| BTS 0000160 | \![notifyplugin]でも\![raiseplugin]と同様にReferenceで情報を通知してほしい | マイナー | 実装済み（2.5.01） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![notifyplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]`（`absent`・持ち主なし）。roadmap の予約に登記あり（Plugin／HEADLINE） | `\![raiseplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` では Reference に付く窓のハンドルなどの情報を、`\![notifyplugin,プラグインのIDまたは名前,イベント名,r0,r1,r2...]` でも同じように付けてほしいという要望。SSP は 2.5.01 で揃えた。 |
| BTS 0000170 | Can't get X-SSTP-Returns when Value empty | 要望 | 実装済み（2.5.04） | 要望 | 未着手 | `ledger/shiori.toml` の `X-SSTP-PassThru-(任意の文字列) [SSP 2.5.03～拡張]`（`absent`・持ち主なし）。SSTP の受信は roadmap の覚え書きに登記あり。同じヘッダを MCP の答えへ通す話は `areka-P0-mcp-shiori-query` が受け持つ | SHIORI が台本を空にして独自のヘッダだけを返すと、SSTP の応答にそのヘッダが載らないので直してほしいという報告。SSP は 2.5.04 で直し、SSTP の接続は 1 回の要求ごとに切る作りだとも説明した。 |
| BTS 0000195 | Interact with ghost using web link | 要望 | 実装済み（2.5.16） | 要望 | 未着手 | `ledger/shiori.toml` の `x-ukagaka-link:type=event&ghost=(ゴースト名)&info=(追加情報)`（`absent`・持ち主なし） | Web の頁のリンクを押すとゴーストへイベントが届く、専用の URL の形がほしいという要望。SSP は 2.5.11 から試し、2.5.16 までに `x-ukagaka-link:type=event&ghost=(ゴースト名)&info=(追加情報)` と、それを受けるイベントの形に固めた。 |
| BTS 0000247 | Some suggestions for speech recognition | 要望 | 未対応 | 要望 | 未着手 | `ledger/shiori.toml` の `OnVoiceRecognitionWord`・`OnVoiceRecognitionStatus`（どちらも `absent`・持ち主なし） | 音声認識について、会話の始めと終わりを合図の音で区切る形・台本からの入り切り・合言葉での呼び出し・画面に出さない音声だけの選択肢など 6 つの案を出した要望（ゴーストごとの音声合成の指定も添えられた）。SSP は未対応のまま。 |
| BTS 0000257 | let Discord Rich Presence PLUGIN can show some info of ghost | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SSP 向けの個別のプラグイン（Discord Rich Presence）の話で、ベースウェアの機能ではない | Discord のプロフィールに出す表示へゴーストの今の様子をひと言載せられるよう、プラグインからゴーストへ尋ねるイベントを送ってほしいという要望。本体ではなくプラグインの側の更新（1.1.2.2）で入った。 |
| BTS 0000284 | improve the specification | 要望 | 仕様どおり | 要望 | 未着手 | roadmap の予約に登記あり（FMO・DirectSSTP）。SSTP の受信は覚え書きに登記あり。動いているゴーストの一覧は MCP の `get_active_ghost_list`（`areka-P0-mcp-tool-entrances`）で引ける | 外のソフトから、動いているゴーストの一覧を知り、SSTP の届け先のゴーストを選べるようにしてほしいという要望。作者は、FMO と DirectSSTP（または IfGhost）で今でもできると説明し、直さずに閉じた（解決状況の欄は差し戻しのまま）。 |
| BTS 0000319 | Allow ValueNotify to use \![notifyother] | 要望 | 実装済み（2.5.36） | 要望 | 未着手 | `ledger/shiori.toml` の `ValueNotify [SSP拡張 2.5.35]`（`absent`・持ち主なし）。`ledger/sakura-script.toml` の `\![notifyother,ゴースト名,イベント名,r0,r1,r2...]` も `absent`・持ち主なし | NOTIFY の応答で台本を返せる `ValueNotify [SSP拡張 2.5.35]` から、ほかのゴーストへイベントを知らせるタグも使えるようにしてほしいという要望。SSP は 2.5.36 で入れた。 |
| BTS 0000332 | Allow "\![raise]" and "\![open,file]" in ValueNotify | 要望 | 実装済み（2.5.40） | 要望 | 未着手 | `ledger/shiori.toml` の `ValueNotify [SSP拡張 2.5.35]`（`absent`・持ち主なし）。`ledger/sakura-script.toml` の `\![raise,イベント名,r0,r1,r2...]` も `absent`・持ち主なし。`\![open,file,ファイル名]` そのものは `areka-P0-open-external-tags` が作った | 最小化の間でも時刻どおりの仕事を動かせるよう、`ValueNotify [SSP拡張 2.5.35]` でイベントを起こすタグとファイルを開くタグを使えるようにしてほしいという要望。SSP は 2.5.40 で前者だけを通し、後者は見えている親の窓が要るとして見送った。 |
| BTS 0000361 | x-ukagaka-linkで起動するゴーストを指定したい | 要望 | 未対応 | 要望 | 未着手 | `ledger/shiori.toml` の `x-ukagaka-link:type=event&ghost=(ゴースト名)&info=(追加情報)`（`absent`・持ち主なし）。ゴーストを指して起動する形はカタログに項目なし | Web のリンクから本体が起動するとき、指したゴーストで立ち上げ、動いていればそのゴーストを呼び出せる選択肢がほしいという要望。SSP は未対応のまま（作者の返事は無い）。 |
| BTS 0000380 | Direct SSTP: send with ThreadID to get ThreadMessage | 要望 | 実装済み（2.5.57） | 要望 | 未着手 | roadmap の予約に登記あり（FMO・DirectSSTP）。SSTP の受信は覚え書きに登記あり | 窓を持たないコンソールのプログラムでも DirectSSTP の返事を受け取れる形がほしいという要望。SSP は窓の無い相手へは返せないとして、2.5.57 でソケットの SSTP に FMO を引く命令と届け先のゴーストを指すヘッダを足した。 |
| BTS 0000436 | メールチェックに失敗する | 要望 | 実装済み（2.5.73） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![biff(,アカウント名)]`（`absent`・持ち主なし） | メールチェックが、暗号化なしの設定でもサーバーへつながらず失敗するようになったという報告（記録には暗号化の接続の誤りが出ていた）。SSP は 2.5.73 で直した。 |
| BTS 0000443 | http-get but not write to file | 要望 | 実装済み（2.5.77） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![execute,http-get,URL,オプション,オプション,オプション...]`・`ledger/shiori.toml` の `OnExecuteHTTPComplete`（どちらも `absent`・持ち主なし） | `\![execute,http-get,URL,オプション,オプション,オプション...]` で取った中身を一時ファイルへ書かず、イベントで直に受け取れるオプションがほしいという要望。SSP は 2.5.77 で入れた。 |
| BTS 0000450 | FMO feature extension: support for shiori status view | 要望 | 実装済み（2.5.78） | 要望 | 未着手 | roadmap の予約に登記あり（FMO・DirectSSTP） | 外のデバッグの道具が、相手のゴーストの SHIORI の状態（降ろしてある・動いている・誤り）を FMO から知りたいという要望。SSP は誤りは分からないとして、2.5.78 で載っているかどうかだけを足した。 |
| BTS 0000453 | Let FMO modulestate reflect shiori's Critical errors | 要望 | 実装済み（2.5.79） | 要望 | 未着手 | roadmap の予約に登記あり（FMO・DirectSSTP） | SHIORI が致命的な誤りで止まっているとき、FMO の SHIORI の状態にもそれが表れるようにしてほしいという要望（BTS 0000450 の続き）。SSP は 2.5.79 で入れた。 |
| BTS 0000512 | FMO shiori status update when unloaded | 些細 | 実装済み（2.6.10） | 要望 | 未着手 | roadmap の予約に登記あり（FMO・DirectSSTP） | SHIORI を降ろした後も FMO の SHIORI の状態が変わらず、外の道具が表示を切り替えられないので、降ろしたことが FMO に表れるようにしてほしいという要望。SSP は 2.6.10 で直した。 |
| BTS 0000516 | Gamepad peripheral support | 要望 | 実装済み（2.7.54） | 要望 | 未着手 | `ledger/shiori.toml` の `OnGamepadButtonDown`・`OnGamepadAxisMove` ほか（どれも `absent`・持ち主なし） | ゲームパッドでゴーストと触れ合えるようにしてほしいという要望。SSP は 2.7.54 で、XInput の機器に限って入れた。 |
| BTS 0000583 | socket sstp don't work | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000604 | sstp over http: origin key in shiori/3.0 | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000618 | sstp expansion | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000652 | 実験的機能のValueNotifyで\![raise]系や\![notify]系が動作しない | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000708 | SBV2と直接連携 | 要望 | 実装済み（2.7） | — | — | — | （未読） |

### 開発者向けの機能

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000088 | 開発用パレットの「SHIORI内部ログ機能有効」が常に無効になっているように見える | マイナー | 仕様どおり | 挙動の説明 | 対象外 | SSP 固有の画面（開発用パレット）の使い方の話 | 開発用パレットの SHIORI の内部ログのチェックが選べないのは、先に SHIORI のデバッグ機能のチェックを入れる決まりだからだと作者が説明し、説明書に書き足した。 |
| BTS 0000176 | Give test update events in the dev palette the ability to test any number of files | 要望 | 実装済み（2.5.11） | 要望 | 未着手 | カタログに項目なし。作者がイベントを試しに送る口は MCP の `raise_event`（`areka-P0-mcp-tool-entrances`）が在るが、更新の一連のイベントをまとめて試す働きは無い | 開発用パレットでネットワーク更新のイベントを試すとき、ファイルの数を 10 に固定せず好きな数で試せるようにしてほしいという要望。SSP は 2.5.11 で入れた。 |
| BTS 0000180 | Make it possible to select multiple intervals at once in surface test | 要望 | 実装済み（2.5.13） | 要望 | 未着手 | カタログに項目なし。サーフェスの絵を作者が確かめる口は MCP の `dump_surface`（`areka-P0-mcp-dump-images`）が在るが、アニメーションを選んで重ねて見る働きは無い | サーフェステストで、複数のアニメーションを同時に選んで重ねた姿を確かめられるようにしてほしいという要望。SSP は 2.5.13 で入れた。 |
| BTS 0000185 | Make it possible to export the base layer of a surface as a png | 要望 | 実装済み（2.5.45） | 要望 | 実装済み | `areka-P0-mcp-dump-images`（MCP の `dump_surface`） | element を重ねて組んだサーフェスの絵を、1 枚の PNG として取り出せるようにしてほしいという要望で、SSP は 2.5.45 で開発用パレットにサーフェスの書き出しを足した。areka は MCP の `dump_surface` が合成済みの絵を透過 PNG で返す（台本からファイルへ書き出す `\![execute,dumpsurface,ディレクトリ,スコープID,サーフェスリスト,prefix,イベントID,ゼロ位置切り出し]` はまだ無い）。 |
| BTS 0000192 | add new sakurascript to open editer with file and linenum | 要望 | 実装済み（2.5.16） | 要望 | 実装済み | `ledger/sakura-script.toml` の `\![open,editor,ファイル,表示行]`（`implemented`・`areka-P0-open-external-tags`） | 誤りの出た辞書のファイルを、行を指定してエディタで開けるタグがほしいという要望で、SSP は 2.5.12 でタグを足し 2.5.16 で表示行にも対応した。areka は同じタグでファイルを OS の「編集」の関連付けで開くが、表示行は使わない（外部アプリの設定を写さない決まり＝`tech.md` の「Key Technical Decisions」）。 |
| BTS 0000225 | Let test update events be cancelled by double clicking the balloon | 要望 | 実装済み（2.5.13） | 要望 | 未着手 | カタログに項目なし。作者がイベントを試しに送る口は MCP の `raise_event`（`areka-P0-mcp-tool-entrances`）が在るが、更新の一連のイベントをまとめて試す働きは無い | 開発用パレットでネットワーク更新のイベントを試している最中に、本物の更新と同じくバルーンのダブルクリックで中断できるようにしてほしいという要望。SSP は 2.5.13 で入れた。 |
| BTS 0000230 | add new sakurascript to turn debugmode of shiori on/off | 要望 | 実装済み（2.5.78） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![set,shioridebugmode,(true/false)]`（`absent`・持ち主なし） | SHIORI のデバッグ機能の入り切りを、台本から切り替えられるタグがほしいという要望。SSP は 2.5.78 で `\![set,shioridebugmode,(true/false)]` を足した。 |
| BTS 0000242 | Extend the syntax of developerp_options.txt | 要望 | 実装済み（2.5.17） | 要望 | 未着手 | roadmap の予約に登記あり（`\![execute,createnar]`／`createupdatedata`。`developer_options.txt` もここ） | 配布物から外すファイルを書く `developer_options.txt` で、`*` や `?` を使った指定と、「このフォルダだけは残す」という打ち消しを書けるようにしてほしいという要望。SSP は 2.5.17 で `*` と `?` の指定だけを入れた。 |
| BTS 0000358 | New sakura script tag for unload and load shiori | 要望 | 実装済み（2.5.58） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![unload,shiori]`・`\![load,shiori]`（どちらも `absent`・持ち主なし）。続けて載せ直す `\![reload,shiori]` は進行中の `areka-P0-mcp-reload` の brief が受け持つ | SHIORI を作り直して試すたびに本体を立ち上げ直さずに済むよう、SHIORI を降ろすタグと載せるタグがほしいという要望。SSP は 2.5.58 で入れた。 |
| BTS 0000379 | 要望：サーフェスリストを名前順でソートする機能 | 要望 | 実装済み（2.5.51） | 要望 | 未着手 | カタログに項目なし。サーフェスの絵を作者が確かめる口は MCP の `dump_surface`（`areka-P0-mcp-dump-images`）が在るが、一覧を並べ替えて見る画面は無い | サーフェステストのサーフェスの一覧を、名前の順に並べ替えられるようにしてほしいという要望。SSP は 2.5.51 で入れた。 |
| BTS 0000392 | Expand Balloon Test Mode | 要望 | 未対応 | 要望 | 未着手 | カタログに項目なし。バルーンの絵を作者が確かめる口は MCP の `dump_balloon`（`areka-P0-mcp-dump-images`）が在るが、見本の文字を流して試す働きは無い | バルーンのテストの表示で、試すキャラクターとバルーンの番号を選べ、選択肢・アンカー・無効の色やマーカーの見本も出るようにしてほしいという要望。SSP は未対応のまま（作者の返事は無い）。 |
| BTS 0000451 | New development panel: request sending panel and the sakura script to open it | 要望 | 実装済み（2.5.78） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![open,shiorirequest]`（`absent`・持ち主なし）。SHIORI へ問い合わせを送って答えを受け取る口を MCP に足す話は `areka-P0-mcp-shiori-query` が受け持つ | 動いているゴーストの SHIORI へ好きな要求を送って試せる開発用の画面と、それを開くタグがほしいという要望。SSP は 2.5.78 で `\![open,shiorirequest]` を足した。 |
| BTS 0000479 | Sometimes the surface dumper outputs a surface with empty space on the bottom | マイナー | 仕様どおり | 挙動の説明 | 方針とぶつかる | 完了した `areka-P0-mcp-dump-images` の要件「Requirement 5: 画像の形」が、MCP の `dump_surface` の画像の幅と高さを読み戻した絵に一致させ、余白を足さないと決めている | サーフェスの書き出しで絵の下に余白が付くのは、全部のサーフェスのうち最も大きい寸法に揃えて出す作りだからだと作者が説明した。 |
| BTS 0000480 | Suggestions for surface dumper | 要望 | 実装済み（2.6.00） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![execute,dumpsurface,ディレクトリ,スコープID,サーフェスリスト,prefix,イベントID,ゼロ位置切り出し]`（`absent`・持ち主なし）。今の見た目を 1 枚返す口は MCP の `dump_surface`（`areka-P0-mcp-dump-images`）が在る | サーフェスの書き出しに、ファイル名の頭の指定・書き出し先の記憶・今のサーフェスの書き出し・終わりを知らせるイベントを足してほしいという要望。SSP は 2.6.00 で入れ、1 枚のときに番号を省く案だけは採らなかった。 |
| BTS 0000520 | dumpsurface for shells that are not in loading | 要望 | 実装済み（2.7.74） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![execute,dumpsurface,ディレクトリ,スコープID,サーフェスリスト,prefix,イベントID,ゼロ位置切り出し]`（`absent`・持ち主なし）。切り替えていないシェルの絵を出す口はカタログに項目なし | まだ切り替えていないシェルの見本を出せるよう、読み込んでいないシェルのサーフェスも書き出せるようにしてほしいという要望。SSP は 2.7.74 で入れた。 |
| BTS 0000613 | Refreshing the AI graph | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000659 | Reload AI graph with sakurascript | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000686 | developer_options.txt - exclude all shells but master shell | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000752 | "Open target directory" button in surface dumper | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000759 | 現在時刻の仮想的変更で秒を指定したい | 要望 | 実装済み（2.7） | — | — | — | （未読） |

### 起動・終了・ゴーストの切り替え

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000194 | let /? or --help as an acceptable command line parameter for SSP | 要望 | 実装済み（2.5.11） | 要望 | 未着手 | カタログに項目なし。起動の引数でゴーストのフォルダを指す形は `areka-P0-baseware-root-layout` が作ったが、引数の一覧を見せる口は無い | 起動の引数にどんなものがあるかを、`/?` や `--help` を付けて起動すれば見られるようにしてほしいという要望。SSP は 2.5.11 で入れた。 |
| BTS 0000228 | Show warning when SSP is running in temporary directory | マイナー | 実装済み（2.5.15） | 要望 | 未着手 | カタログに項目なし | 書庫の中から直に起動されて一時フォルダで動いているときは、ゴーストの記憶が消える恐れを利用者へ警告してほしいという要望。SSP は 2.5.15 で入れた。 |
| BTS 0000588 | ゴースト終了時、execute,http-コマンドの完了を待機させたい | 要望 | 仕様どおり | — | — | — | （未読） |
| BTS 0000619 | Save information about firstboot soon after firstboot occurs, to avoid data loss | メジャー | 実装済み（2.6） | — | — | — | （未読） |

### その他

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000012 | SSP-I18N：翻訳環境の充実 | 要望 | 仕様どおり | 要望 | 対象外 | SSP 固有の翻訳の仕組みの話 | SSP の画面の翻訳を、共同作業の道具やテキストファイルで進められる形に改めたいという作者自身の提案。翻訳の環境は変えないと決めて閉じた。 |
| BTS 0000013 | 高速スタートアップによる不具合の説明 | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SSP の配布サイトの説明文の話 | Windows の高速スタートアップが原因で更新やインストールに失敗する件と対処を、公式サイトの目立つ所に載せ直してほしいという要望。作者がサイトを直して閉じた。 |
| BTS 0000015 | 2.4.38以下はWindows 2000では起動不可能 | 要望 | 実装済み（2.4.40） | 要望 | 対象外 | areka は Windows 10／11 が前提（`tech.md` の「Required Tools」）で、古い Windows での起動は意味を持たない | Windows 2000 では起動に要る関数が見つからず動かないという報告。SSP は 2.4.40 で起動できるように直した。 |
| BTS 0000025 | ゴースト毎にサブプロセス化 | 要望 | 未対応 | 要望 | 未着手 | 前提になる多重ゴーストは roadmap の予約に登記あり | ゴーストごとに別のプロセスで動かし、1 体の異常で全部のゴーストが落ちないようにしてほしいという要望。SSP は未対応のまま。 |
| BTS 0000076 | translate ssp to chinese,done. | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SSP の翻訳パックの話 | SSP の画面の中国語（簡体字）の翻訳を作ったので使ってほしいという申し出。作者が言語ファイルを取り出して公式サイトに置いた。 |
| BTS 0000112 | sounds stop working | 要望 | 実装済み（2.4.93） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![sound,play,ファイル名,オプション...]`（`absent`・持ち主なし） | `\![sound,play,ファイル名,オプション...]` を繰り返し使うと音が一切鳴らなくなり、一部の曲は再生できないという不具合の報告。SSP は 2.4.89 と 2.4.93 で直した。 |
| BTS 0000126 | Add traditional Chinese language and update simplified Chinese | 要望 | 実装済み（2.4） | 要望 | 対象外 | SSP の翻訳パックの話 | SSP の画面の中国語の翻訳（繁体字の追加と簡体字の更新）を使ってほしいという申し出。作者が受け取り、2.4.91 から翻訳パックをインストールできる形にしたと知らせた。 |
| BTS 0000133 | When no ghosts/balloons are installed, the window that comes up is always in Japanese | 要望 | 実装済み（2.4.92） | 要望 | 対象外 | areka はこの窓を持たない（既定のゴーストを同梱し、メッセージボックスを出さない＝roadmap の「生きている決まり」7） | ゴーストもバルーンも入っていないときに出る案内の窓が日本語だけなので、他の言語でも出してほしいという要望。SSP は 2.4.92 で英語の窓を足した。 |
| BTS 0000141 | why not try to put SSP on steam? | 要望 | 却下 | 要望 | 対象外 | SSP の配布先の話 | 利用者を増やすために SSP を Steam で配ってはどうかという提案。伺かの界隈は無償でない商用の配布の場を受け入れられないとして却下された。 |
| BTS 0000147 | how about put ssp on github? | 要望 | 却下 | 要望 | 対象外 | SSP のソースの公開の話 | 要望を出すより直接直せるように SSP のソースを GitHub で公開してはどうかという提案。昔からのコードの権利の事情で公開できないとして却下された。 |
| BTS 0000161 | Chinese pakeage update | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SSP の翻訳パックの話 | SSP の画面の中国語の翻訳パックを新しくしたので差し替えてほしいという申し出。作者が公式サイトのリンクを替えて閉じ、翻訳パックに配布元の URL を持たせる話は BTS 0000164 へ分けた。 |
| BTS 0000164 | language pack homeurl (automatic update) | 要望 | 実装済み（2.5.16） | 要望 | 対象外 | SSP の翻訳パックの話 | SSP の翻訳パックにも配布元の URL を持たせて、ネットワーク更新できるようにしてほしいという要望。SSP は 2.5.16 で入れた。 |
| BTS 0000209 | About the code page | 些細 | 仕様どおり | 要望 | 方針とぶつかる | `doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」と完了した `areka-P0-charset-canon` の要件が、宣言が無いときの文字コードを固定の写像（Shift_JIS）で決め、OS の設定も読まないと決めている | 表示できない文字が出たら文字コードの取り違えとみなし、別の文字コードで読み直す自動の判定を入れてはどうかという提案。SSP は自動の判定を見送り、2.5.10 で既定の文字コードの設定と `readme.charset,文字コード` を足した。 |
| BTS 0000217 | Inform the user of the download address of LAV filters when the audio file cannot be played | 要望 | 実装済み（2.5.08） | 要望 | 未着手 | `ledger/sakura-script.toml` の `\![sound,play,ファイル名,オプション...]`（`absent`・持ち主なし）。知らせを窓で出さない決まり（roadmap の「生きている決まり」7）が先に在る | 音のファイルを再生できなかったときに、形式を解く部品（LAV Filters）を入れれば直ると利用者へ知らせてほしいという要望。SSP は 2.5.08 で入れた。 |
| BTS 0000236 | cloud saves of ghost&ssp profile | 要望 | 実装済み（2.7.76） | 要望 | 未着手 | カタログに項目なし | ゴーストの記憶と本体の設定を OneDrive などの同期フォルダへ置き、入れ直しや別の PC でも引き継げるようにしてほしいという要望。SSP は 2.7.76 で対応したとして閉じた（何を入れたかは頁に書かれていない）。 |
| BTS 0000250 | a constant link points to the latest version of the SSP zip file | 些細 | 実装済み（版は不明） | 要望 | 対象外 | SSP の配布サイトの話 | 配布物を自動で組み立てるために、いつも最新版の SSP の書庫を指す決まった URL がほしいという要望。作者が配布の頁を示して閉じた。 |
| BTS 0000280 | allow charset to be set separately for each ghost | 要望 | 実装済み（2.5.74） | 要望 | 未着手 | 宣言の無いファイルは OS の設定に依らず Shift_JIS で読む（完了した `areka-P0-charset-canon`）ので、頁の例（日本語のゴーストが他の言語の OS で化ける）は起きない。利用者がゴーストごとに選び直す設定は無く、設定画面は roadmap の予約に登記あり | 文字コードの自動の判定が外れてゴーストの名前や更新のファイル名が化けることがあるので、ゴーストごとに文字コードを手で決められる設定がほしいという要望。SSP は設定を足さず、2.5.74 で自動の判定を強めて閉じた。 |
| BTS 0000293 | Auto update of language packages when the dll version in use is less than the ssp ver | 要望 | 実装済み（2.5.49） | 要望 | 対象外 | SSP の翻訳パックの話 | 使っている翻訳パックが本体より古いときは自動で見つけて更新し、手で更新しなくて済むようにしてほしいという要望。SSP は 2.5.49 で、版の比較ではなく日ごとの更新の確認として入れた。 |
| BTS 0000544 | Some minor adjustments required | マイナー | 実装済み（2.6.15） | 要望 | 対象外 | areka は既定のゴーストを同梱していて、ゴーストが 1 体も無いときの案内の窓を持たない（roadmap の「生きている決まり」7）ので、意味を持たない | ゴーストが 1 体も無くて案内の窓が出ている間も、起動の引数から始めた nar のインストールを受け付けて、普通の起動へ進んでほしいという要望。SSP は 2.6.15 で入れた。 |
| BTS 0000553 | New option to automatically switch languages after installing a language pack | 要望 | 実装済み（2.6.16） | 要望 | 対象外 | SSP の翻訳パックの話 | 翻訳パックを入れても画面の言語が日本語のままで利用者が戸惑うので、入れた後に自動でその言語へ切り替えてほしいという要望。SSP は 2.6.16 で入れた（解決状況の欄は不明のまま）。 |
| BTS 0000620 | New image for fixing pages in dark mode | 些細 | 実装済み（版は不明） | — | — | — | （未読） |

### SHIORI・ゴースト個別

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000071 | Line breaking in OnTextDrop | マイナー | 仕様どおり | 挙動の説明 | 対象外 | SHIORI の話 | 頁を読んだ。`OnTextDrop` で受けた文字列の改行を判定できないという質問で、YAYA の標準の辞書がバイト値 1 をカンマへ自動で置き換えるためだと作者が説明し、YAYA の設定で止める方法を示した。 |
| BTS 0000175 | If the dic file wrong, return the wrong content through ErrorDescription & ErrorLevel in response | 要望 | 重複 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000177 | make Shiori work normally when dic file wrong | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000181 | run time dic file load | 要望 | 却下 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000182 | Let Yaya support automatically contain the entire folder‘s dic file | 要望 | 却下 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000186 | Add optional warning message that ghost has booted into emergency mode | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話（受ける側の `ErrorDescription [SSP拡張]` は `ledger/shiori.toml` で `implemented`） | 頁を読んだ。YAYA が辞書の誤りで緊急モードで立ったことを、はっきり警告で知らせる選択肢がほしいという要望で、YAYA の標準の辞書が `ErrorDescription [SSP拡張]` を返して SSP の誤りの表示に載せる形で片付いた。 |
| BTS 0000189 | draft code:support of 'dicdir' in yaya.txt | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000193 | Increase number of errors output to error log | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000199 | Add a function to yaya to get the name of the current mode | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000200 | Multi mode support of yaya | 要望 | 仕様どおり | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000204 | Let yaya short-circuit evaluation like C | 要望 | 却下 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000244 | New option to show only certain events in shiori log | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | 頁を読んだ。SHIORI の入出力のログに、決めたイベントだけを残す指定（今ある「除く」指定の逆）がほしいという要望で、YAYA の Tc560-1 にログの絞り込みの設定が入って片付いた。 |
| BTS 0000265 | Anti cheating check => Hook function for variable monitoring | 要望 | 未対応 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。閉じられているが解決状況は reopened のまま。 |
| BTS 0000274 | Make a yatama exe | 要望 | 仕様どおり | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000333 | New function modifier for dialogue pools | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000340 | About the behavior of : pool... | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000365 | Increasing the possible depth of yaya's stack | 些細 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000381 | Let parallel function be used for sets of brackets | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000503 | 古い栞で400 Bad Requestが出る | マイナー | 仕様どおり | 質問 | 対象外 | SHIORI の話 | 頁を読んだ。SHIORI/2.x の華和梨のゴーストで、台本の翻訳の要求に毎回 400 が返るのは古い栞だからかという質問で、ゴーストの辞書に翻訳の受け口が無いためだと作者が説明した。 |
| BTS 0000569 | ゴースト「あかね＆ますたー」にてメニューを開いた後項目を選択しようとするとエラーメッセージ | 要望 | 却下 | 要望 | 対象外 | ゴースト個別の話 | 頁を読んだ。あるゴーストのメニューで項目を選ぶと誤りが出るので調べてほしいという報告で、ゴーストの辞書の書き方が原因と分かり、作者は本体では直せないとして閉じた。 |

## 起票の候補

（全部の行を読み終えた後に書く）
