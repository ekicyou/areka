# SSP の BTS の要望の台帳

`areka-P0-ssp-bts-salvage` の調査の記録である（要件 1〜8）。SSP の課題管理（BTS・https://bts.shillest.net/ ）に溜まった要望を 1 回だけ通読し、areka の目線で 1 件 1 行にまとめた。

**これは 1 回きりの調査である。** 定期的に見直す仕組みは持たない。BTS の側は書いた後も動くので、古さは下の「見た範囲」で判断する。

## 見た範囲

| 回 | 通読した日付 | 番号の範囲 | 一覧の総数 | 深刻度「要望」の件数 | 一覧の取り方と絞り込みの条件 | areka の側を照らし合わせた日付と main のコミット |
|---|---|---|---|---|---|---|
| 1 | 2026-10-10 | 0000001〜0000808 | 808 | 187 | CSV を 1 回（匿名）。検索の入口 `search.php` で「ステータス非表示」を「なし」にした、その場かぎりの絞り込みで出した全件 | （頁を読み始めるときに書く） |

- 起票時の実測（全 808 件・要望 187 件・最新 0000808）との差: **差なし**。
- 通読を始めた時点の最新の番号は 0000808。これより後の番号は扱っていない。
- 既定の一覧は「完了」の課題を隠していて 20 件しか出ない。絞り込みの欄の送信（`view_all_set.php`）は匿名では拒まれる（401）。全件は上の検索の入口から出せた。

## 件数の内訳

| 項目 | 件数 |
|---|---|
| 深刻度「要望」から行にした件数 | 187 |
| それ以外の深刻度から行にした件数 | 65 |
| 読んだが外した件数 | 0 |
| 対象に入らなかった件数（頁を開いていない） | 556 |
| 合計（一覧の総数） | 808 |

- 読んだが外した番号: なし

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
| 解決状況 | fixed・open・reopened・no change required・not fixable・duplicate・unable to reproduce の 7 つが在った（一覧の画面に列が無いので、日本語の語は課題の頁で確かめて足す） |
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
| さくらスクリプトと文字の現れ方 | さくらスクリプトのタグと、文字・音の出し方 | 18 |
| バルーン | バルーンの見た目・位置・入力ボックス | 17 |
| シェルとサーフェス | シェルの描き方・サーフェスとアニメーションの定義 | 14 |
| SHIORI イベント・リソース・プロパティシステム | ベースウェアが SHIORI へ知らせるイベントと、SHIORI から引ける情報 | 21 |
| 窓と配置 | キャラクターの窓の位置・重なり順・ディスプレイ | 15 |
| メニュー・設定・操作 | 右クリックメニュー・設定画面・エクスプローラ・ログなど利用者が触る画面 | 35 |
| インストール・配布・削除 | nar の作り方と入れ方・アンインストール | 20 |
| ネットワーク更新 | ゴースト・シェル・バルーン・本体の更新の仕組み | 21 |
| 外との連携 | SSTP・FMO・プラグイン・メール・音声・Web からの呼び出し | 27 |
| 開発者向けの機能 | 開発用パレット・サーフェスの書き出し・テストの機能 | 22 |
| 起動・終了・ゴーストの切り替え | 起動の引数・終了の待ち・初回起動 | 4 |
| その他 | 翻訳・配布元・文字コードなど、上のどれにも入らないもの | 23 |
| SHIORI・ゴースト個別 | SHIORI（YAYA・里々など）とゴースト個別の話。頁は開いていない | 15 |

## 主題ごとの表

### さくらスクリプトと文字の現れ方

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000073 | a little question of sakura script | 些細 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000153 | \j command doesn't support all characters | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000156 | 'The operation cannot be performed because the pins are not connected' error when playing some songs with \![sound,play,...] | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000213 | About the implementation details of '\C' script | 微調整 | 仕様どおり | — | — | — | （未読） |
| BTS 0000264 | Sakurascript tag to temporarily adjust text speed | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000267 | Options to center or right-align text | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000271 | absolute changes with balloonwait tag | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000330 | allow \![executesntp,timesever] | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000348 | Add new 'disable' text color to balloons, which can be used with the \f[color] tag | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000389 | Make it possible for disable text option to affect only color | 要望 | 仕様どおり | — | — | — | （未読） |
| BTS 0000419 | Give \![open,readme] an optional argument to specify what readme to open | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000471 | Rate option of sound command can only go to 435 | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000501 | Add time option to \![set,alpha] tag | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000580 | More ghost-friendly url handling | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000611 | \q[]に\u[]とかを挟むと\q側の]が認識されないのって仕様ですか？ | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000692 | Add tag \![open,dressupexplorer] | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000740 | Extra "wait" parameter for \![set,alpha] and \![set,scaling] tags | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000745 | \![open,dialog,open]などのダイアログ系を実行後に大きな負荷が発生する | メジャー | 仕様どおり | — | — | — | （未読） |

### バルーン

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000014 | 左右非対称なシェルでのバルーン表示位置 | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000016 | 吹き出しの表示をSNS風にする機能 | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000026 | シェル倍率でバルーン連携しない場合 | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000068 | balloon_tooltipについて | マイナー | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000119 | バルーン倍率をシェルと連携せずに別々に調整する機能 | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000255 | let Gif / apng can be embedded in balloon, like PNG with \\_b | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000278 | Allow input boxes to pass additional reference values | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000306 | Let input boxes have semi-transparency | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000307 | Let input boxes default to balloonc0 if no image is present for them | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000324 | Add an option to change the color of the font and ok/cancel buttons for input boxes | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000338 | About cursor customisation of text input boxes | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000366 | Allow balloon arrows/online marker/etc to extend off the base image | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000382 | Let alternate arrow/marker/online marker files be specified in balloon(s/k)*s.txt files | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000402 | Allow the developer to specify what balloon to use for input boxes | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000446 | Add option to adjust the speed of the balloon's online marker animation | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000567 | "Sticky" balloon tags | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000804 | バルーンのsstpmessage.ybの挙動について | マイナー | 実装済み（2.8） | — | — | — | （未読） |

### シェルとサーフェス

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000020 | 【要望】2D/3DCGを扱う | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000035 | シェル倍率を変えるとジャギるようになった | 要望 | 仕様どおり | — | — | — | （未読） |
| BTS 0000038 | 拡大/縮小時の補完処理の「シェル個別設定」がほしい。 | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000109 | animation-sortの表示順と当たり判定について | マイナー | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000173 | Make Stop and Start methods able to handle multiple intervals at once (or add a new method for this) | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000183 | allow shells to draw windows with it's own DLL | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000301 | Allow balloon and shell to have custom dlls to draw the interface | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000372 | Allow surfaces to have non-numerical names | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000601 | kero.seriko.defaultsurfaceがOnWindowStateRestore を用意していない場合に機能していない。 | 些細 | 仕様どおり | — | — | — | （未読） |
| BTS 0000630 | シェルの拡大縮小アルゴリズムについて | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000762 | Prevent looping of non-repeating .GIF surfaces | 微調整 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000785 | Optional name attribute for animations | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000792 | SSP組み込みクロスフェード機能を強制OFFにするdescript.txt設定がほしい | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000795 | How to get surface width before the surface is displayed | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### SHIORI イベント・リソース・プロパティシステム

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000071 | Line breaking in OnTextDrop | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000081 | my ghost can't get OnShellChanged on ssp 2.4.68 | マイナー | 却下 | — | — | — | （未読） |
| BTS 0000082 | 0000081: my ghost can't get OnShellChanged on ssp 2.4.68 | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000166 | some events have no docs | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000234 | new event to notify ghost what language is the user using | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000262 | BaseIDが来ない | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000279 | ghost's hourly chime dosn't work after updated to latest ssp | メジャー | 却下 | — | — | — | （未読） |
| BTS 0000286 | Allow OnSoundStop to be called as GET while ghost is minimized | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000288 | New event: rate of use graph within one week | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000310 | Give OnMouseDragEnd a reference value for if the sakura and kero are overlapping | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000336 | Detect cursor/other ghost position | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000430 | Statusの読み取りでエラーが出る | メジャー | 仕様どおり | — | — | — | （未読） |
| BTS 0000492 | Add option to set cursor for entire ghost with property system | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000503 | 古い栞で400 Bad Requestが出る | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000540 | OnDisplayChange parameter has error | マイナー | 却下 | — | — | — | （未読） |
| BTS 0000551 | Notification of whether the current network will be charged in network information notification event | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000584 | RecycleBin related update | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000593 | OnNetworkStatusChangeに置いて有線ＬＡＮ接続なのにReference2が「other」となる。 | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000616 | ゴーストやプラグインにおけるOnOtherGhostTalkイベントでNoContentなイベントも補足できるようにしてほしい | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000728 | OnUserInput occurs when inputbox times out | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000791 | プロパティシステムでバルーンの現在位置座標を取得したい | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### 窓と配置

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000023 | 吹き出しとシェルを前面に表示 | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000150 | defaulty and defaulttop shell commands do nothing | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000162 | デスクトップ解像度変更時の挙動 | 表示 | 未対応 | — | — | — | （未読） |
| BTS 0000355 | Allow user to specify the distance at which ghosts should 'stick' to the taskbar/screen edge | 要望 | 実装済み（2.8） | — | — | — | （未読） |
| BTS 0000463 | バルーンをシェルより前面に表示する設定がほしい。（2.5.34での仕様変更による） | 要望 | 却下 | — | — | — | （未読） |
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
| BTS 0000036 | 【要望】SSPエクスプローラの挙動 | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000163 | about thumbnail.png big than screen | 些細 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000227 | asking for some basic settings when SSP runs for the first time | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000291 | Add a sakurascript and a button in the owner draw menu to view terms.txt again | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000315 | Make the readme dialog support ctrl+A select all | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000327 | Add an option to tile the owner draw menu image | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000331 | \! [set,trayballoon] seems to cause lag on some computers | マイナー | 却下 | — | — | — | （未読） |
| BTS 0000337 | Use the busy mouse icon when shiori doesn't return for a long time instead of just getting stuck | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000345 | mousecursor / mousecursor.text for ghost's descript.txt & terms.txt in ghost dir | 微調整 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000347 | About the change to the owner draw menu... | 些細 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000359 | Add a right click to "Add to computer boot-up" in ghost browser | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000362 | Give tooltips multiple language support | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000374 | エクスプローラのソート | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000390 | Icon animation support and runtime modification of the animation's cycle time | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000407 | さくらスクリプト \![set,trayballoon,～] でタスクトレイからバルーンが出ない | 些細 | 仕様どおり | — | — | — | （未読） |
| BTS 0000416 | Icon animation but only run once | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000418 | Some suggestions for icon animation | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000429 | スクリプトログの挙動について | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000438 | "同じ処理を次以降の項目にも適用"チェックを キャンセルボタンでも有効にして欲しい | 些細 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000526 | Add option to disable user skipping text | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000533 | Widen TimeMachine window | 些細 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000537 | There is no argument for opening the speech tab of the SSP preferences | マイナー | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000561 | About the trayballoon timeout | マイナー | 実装済み（2.6） | — | — | — | （未読） |
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
| BTS 0000115 | 削除処理のOnVanishSelected以降をファイルを消さずに実行する機能 | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000167 | some nar can't install now | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000179 | Some methods of compressing the size of install package | 些細 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000190 | let nar file support password and 7-zip | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000221 | Some improvements on NAR password | マイナー | 未対応 | — | — | — | （未読） |
| BTS 0000243 | \![execute,createnar,filename] | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000252 | Let the NAR file can have its own icon | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000253 | narpng | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000254 | x-ukagaka-link: install from homeurl | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000261 | user terms file support for NAR | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000354 | Modify or add a new uninstall mode to allow ghost to do something after it has been uninstalled | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000395 | x-ukagaka-link: install from nar url | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000455 | definition of deleteX.txt | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000457 | New sakura script to open the uninstall confirmation dialog | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000546 | New command line parameters for installing nar | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000557 | some final minor tweaks needed for ghost installer | 微調整 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000570 | Allow a ghost to install multiple balloons | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000581 | バルーンのフォルダ名を変更してほしい | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000582 | Whitelist mode for delete.txt | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000798 | アーカイブビューワの動作 | 要望 | 実装済み（2.8） | — | — | — | （未読） |

### ネットワーク更新

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000090 | ゴーストの更新ファイル作成時に、ゴーストのルートフォルダだけではなく、ghost/master 以下にも更新ファイルが作られる | マイナー | 却下 | — | — | — | （未読） |
| BTS 0000114 | ネットワーク更新検証機能 | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000120 | checkonly option for Balloon and Shell updates | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000178 | About big file update | 微調整 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000205 | Let ghost provide another update path to the shell | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000214 | Delete or update the update cache after ghost is repeatedly installed | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000215 | Speed up the construction of update.txt by caching the last modification time of files and folders | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000218 | add new sakurascript to build Update.dau | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000223 | Add a sakurascript tag for updating plugins | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000226 | new update modes | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000232 | About the code page of update.dau | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000241 | cache the downloaded files to avoid repeated downloads when the update fails | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000282 | let ghost can start other update modes | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000303 | How to trigger other_homeurl_override? | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000305 | Gives users the option to update ssp automatically and silently | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000442 | Optimisation of network updates and homeurl override installation | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000456 | Separate the time cache from the update file | 微調整 | 未対応 | — | — | — | （未読） |
| BTS 0000458 | 実際の更新日時がわからなくなった | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000513 | add a file verification to the ssp update process | メジャー | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000515 | New extension to update function | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000683 | バルーン指定recommended.balloon.forceupdateによるネットワーク更新時の強制バルーン更新機能の提案 | マイナー | 実装済み（2.7） | — | — | — | （未読） |

### 外との連携

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000003 | POPに複数のアカウントを追加した場合の挙動について | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000084 | ヘッドラインについて | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000096 | new type sstp | 要望 | 重複 | — | — | — | （未読） |
| BTS 0000125 | Select different text to speech voices for Sakura and Kero | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000137 | Expand SSTP support | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000160 | \![notifyplugin]でも\![raiseplugin]と同様にReferenceで情報を通知してほしい | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000170 | Can't get X-SSTP-Returns when Value empty | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000195 | Interact with ghost using web link | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000247 | Some suggestions for speech recognition | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000257 | let Discord Rich Presence PLUGIN can show some info of ghost | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000319 | Allow ValueNotify to use \![notifyother] | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000332 | Allow "\![raise]" and "\![open,file]" in ValueNotify | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000361 | x-ukagaka-linkで起動するゴーストを指定したい | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000380 | Direct SSTP: send with ThreadID to get ThreadMessage | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000409 | \![raiseother の __SYSTEM_ALL_GHOST__ | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000436 | メールチェックに失敗する | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000441 | Sometimes the plugin's selection event is sent to ghost | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000443 | http-get but not write to file | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000450 | FMO feature extension: support for shiori status view | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000453 | Let FMO modulestate reflect shiori's Critical errors | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000512 | FMO shiori status update when unloaded | 些細 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000516 | Gamepad peripheral support | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000583 | socket sstp don't work | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000604 | sstp over http: origin key in shiori/3.0 | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000618 | sstp expansion | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000652 | 実験的機能のValueNotifyで\![raise]系や\![notify]系が動作しない | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000708 | SBV2と直接連携 | 要望 | 実装済み（2.7） | — | — | — | （未読） |

### 開発者向けの機能

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000088 | 開発用パレットの「SHIORI内部ログ機能有効」が常に無効になっているように見える | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000176 | Give test update events in the dev palette the ability to test any number of files | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000180 | Make it possible to select multiple intervals at once in surface test | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000185 | Make it possible to export the base layer of a surface as a png | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000186 | Add optional warning message that ghost has booted into emergency mode | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000192 | add new sakurascript to open editer with file and linenum | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000225 | Let test update events be cancelled by double clicking the balloon | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000230 | add new sakurascript to turn debugmode of shiori on/off | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000242 | Extend the syntax of developerp_options.txt | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000244 | New option to show only certain events in shiori log | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000358 | New sakura script tag for unload and load shiori | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000379 | 要望：サーフェスリストを名前順でソートする機能 | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000392 | Expand Balloon Test Mode | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000451 | New development panel: request sending panel and the sakura script to open it | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000479 | Sometimes the surface dumper outputs a surface with empty space on the bottom | マイナー | 仕様どおり | — | — | — | （未読） |
| BTS 0000480 | Suggestions for surface dumper | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000520 | dumpsurface for shells that are not in loading | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000613 | Refreshing the AI graph | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000659 | Reload AI graph with sakurascript | 要望 | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000686 | developer_options.txt - exclude all shells but master shell | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000752 | "Open target directory" button in surface dumper | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000759 | 現在時刻の仮想的変更で秒を指定したい | 要望 | 実装済み（2.7） | — | — | — | （未読） |

### 起動・終了・ゴーストの切り替え

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000194 | let /? or --help as an acceptable command line parameter for SSP | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000228 | Show warning when SSP is running in temporary directory | マイナー | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000588 | ゴースト終了時、execute,http-コマンドの完了を待機させたい | 要望 | 仕様どおり | — | — | — | （未読） |
| BTS 0000619 | Save information about firstboot soon after firstboot occurs, to avoid data loss | メジャー | 実装済み（2.6） | — | — | — | （未読） |

### その他

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000012 | SSP-I18N：翻訳環境の充実 | 要望 | 仕様どおり | — | — | — | （未読） |
| BTS 0000013 | 高速スタートアップによる不具合の説明 | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000015 | 2.4.38以下はWindows 2000では起動不可能 | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000025 | ゴースト毎にサブプロセス化 | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000076 | translate ssp to chinese,done. | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000112 | sounds stop working | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000126 | Add traditional Chinese language and update simplified Chinese | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000133 | When no ghosts/balloons are installed, the window that comes up is always in Japanese | 要望 | 実装済み（2.4） | — | — | — | （未読） |
| BTS 0000141 | why not try to put SSP on steam? | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000147 | how about put ssp on github? | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000161 | Chinese pakeage update | 要望 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000164 | language pack homeurl (automatic update) | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000209 | About the code page | 些細 | 仕様どおり | — | — | — | （未読） |
| BTS 0000217 | Inform the user of the download address of LAV filters when the audio file cannot be played | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000236 | cloud saves of ghost&ssp profile | 要望 | 実装済み（2.7） | — | — | — | （未読） |
| BTS 0000250 | a constant link points to the latest version of the SSP zip file | 些細 | 実装済み（版は不明） | — | — | — | （未読） |
| BTS 0000280 | allow charset to be set separately for each ghost | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000284 | improve the specification | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000293 | Auto update of language packages when the dll version in use is less than the ssp ver | 要望 | 実装済み（2.5） | — | — | — | （未読） |
| BTS 0000544 | Some minor adjustments required | マイナー | 実装済み（2.6） | — | — | — | （未読） |
| BTS 0000553 | New option to automatically switch languages after installing a language pack | 要望 | 未対応 | — | — | — | （未読） |
| BTS 0000569 | ゴースト「あかね＆ますたー」にてメニューを開いた後項目を選択しようとするとエラーメッセージ | 要望 | 却下 | — | — | — | （未読） |
| BTS 0000620 | New image for fixing pages in dark mode | 些細 | 実装済み（版は不明） | — | — | — | （未読） |

### SHIORI・ゴースト個別

| BTS | 要約 | 深刻度 | SSP 側の結末 | 種別 | areka との関係 | 根拠・理由 | ひと言 |
|---|---|---|---|---|---|---|---|
| BTS 0000175 | If the dic file wrong, return the wrong content through ErrorDescription & ErrorLevel in response | 要望 | 重複 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000177 | make Shiori work normally when dic file wrong | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000181 | run time dic file load | 要望 | 却下 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000182 | Let Yaya support automatically contain the entire folder‘s dic file | 要望 | 却下 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000189 | draft code:support of 'dicdir' in yaya.txt | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000193 | Increase number of errors output to error log | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000199 | Add a function to yaya to get the name of the current mode | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000200 | Multi mode support of yaya | 要望 | 仕様どおり | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000204 | Let yaya short-circuit evaluation like C | 要望 | 却下 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000265 | Anti cheating check => Hook function for variable monitoring | 要望 | 未対応 | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。閉じられているが解決状況は reopened のまま。 |
| BTS 0000274 | Make a yatama exe | 要望 | 仕様どおり | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000333 | New function modifier for dialogue pools | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000340 | About the behavior of : pool... | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000365 | Increasing the possible depth of yaya's stack | 些細 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |
| BTS 0000381 | Let parallel function be used for sets of brackets | 要望 | 実装済み（版は不明） | 要望 | 対象外 | SHIORI の話 | YAYA（SHIORI）への話。頁は開いておらず、一覧の情報だけで書いた。 |

## 起票の候補

（全部の行を読み終えた後に書く）
