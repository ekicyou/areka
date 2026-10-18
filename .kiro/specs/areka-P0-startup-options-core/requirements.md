# Requirements Document

## Project Description (Input)
開発者と AI のエージェントが areka を自動の試験で起こすとき、今の areka で指定できるのは、ゴーストのフォルダとバルーンのフォルダを並べて渡す形（`areka.exe <ゴースト> <バルーン>`）と、`AREKA_` で始まる環境変数だけである。SSP なら `--ghost`・`--option nobootcheck,bootunlock,standalone`・`--sstp-listen 9900` のように、どのゴーストで・どの待受で・どんな約束で起こすかを 1 行で書ける。areka では同じことを環境変数の組み合わせで真似るしかなく、試験ごとに書き方がばらつく。また、SSP 向けに書かれた手順書や道具の `ssp.exe --ghost 名前` をそのまま areka に向けると、知らない引数がゴーストのフォルダの位置に読まれ、「ゴーストが見つからない」で止まる。

この spec は、areka が SSP と同じ書き方の起動オプションを読めるようにする。`--ghost`（名前か絶対パス）・`--random`・`--option`（`nobootcheck`・`dontloadmodule` と、受けて何もしない語）・`--sstp-listen`（MCP の待受）・`--help` を効かせ、知らないオプションは記録に残して起動を続ける。並べて渡す今の形はそのまま動かす。v0.1.0 の目標「SSP 並みの自動試験・開発の速さ」の入口で、後に続く `offline-surface-dump`・`readonly-mode`・`startup-install-option` はこの spec が作る起動オプションの読み取りの上に載る。詳細は同じフォルダの `brief.md`。

## Introduction

SSP のヘルプ「起動オプション一覧」（https://ssp.shillest.net/ukadoc/ssphelp/option.html ・2026-10-18 に ukadoc の MCP で読んだ）が、この spec の正典である。正典は書き方を次のように定める。

- 「各オプションは --オプション名 形式を推奨します。後方互換のため /x / -x 形式のショートオプションも引き続き使用できます。」
- 値を取るオプションは `--オプション名 値` のように空白で区切る。2.8.97 以降は `--オプション名=値` も受ける。「なお /x 形式のショートオプションでは = 形式は使えません。」
- `--option` は「カンマ区切りで複数指定できます（例：--option bootunlock,standalone）」。

正典の各オプションのうち、この spec が効かせるものと、その意味は次のとおり（引用は正典の「機能」の欄）。

- `--ghost フォルダ名/ゴースト名`（旧 `/g`）: 「指定したゴーストを起動する。空白区切りで複数起動する。」起動時は「指定されたゴーストのうち最初のものを1組目のゴーストとして起動する。」2.8.99 以降、ゴーストのフォルダの絶対パスを渡し、そのゴーストが読み込みフォルダに無ければ、親のフォルダを「その起動中だけ一時的に」読み込みフォルダへ足す。対象は `ghost\master\descript.txt` が在るフォルダだけ。足したフォルダは設定に保存されず、編集・削除はできず、ゴーストのインストール先の候補にもならない。
- `--random`（旧 `/r`）: 「設定にかかわらずゴーストをランダム起動」。
- `--option nobootcheck`: 「異常終了チェックを無効にする」。
- `--option dontloadmodule`: 「SHIORI/MAKOTO/PLUGIN/HEADLINE/CALENDAR系を読まない」。
- `--option bootunlock`（二重起動の確かめを飛ばす）・`standalone`（起動中の SSP へ処理を任せない）・`layered`／`region`（半透明の窓の有無）・`dpiaware`（DPI の追従を Per-Monitor V2 にする）。
- `--sstp-listen 待ち受け先`: 「保存されているSSTPサーバの待ち受け設定は読み込まず、指定した待ち受け先だけで待ち受ける。」待ち受け先はカンマ区切りで複数書け、書式は `ポート番号`（ループバック）・`localhost:ポート番号`・`IPv4アドレス:ポート番号`・`[IPv6アドレス]:ポート番号`・`none`（「一切待ち受けない。待ち受け先を省略した場合も同じ。」）。「書式が正しくない待ち受け先や待ち受けに失敗した待ち受け先は無視され、エラーログに記録される。この場合も保存されている設定には戻らない。」SSP の MCP は SSTP と同じ待受に載るので、areka ではこれが MCP の待受に当たる。
- `--help`（旧 `/?`）: 「起動オプションのヘルプページをブラウザで開く」。

今の areka は、並べて渡した 1 つ目をゴーストのフォルダ、2 つ目をバルーンのフォルダとして読むだけで、ほかの引数は読まない。並べて渡したゴーストで起こす形（以下「絶対パス起動」）は、ゴーストの一覧・起動中の印・最後に使ったゴーストの記憶・利用者ごとの置き場への写しのどれも読まない。この形は実機の試験が使っており、残す約束になっている。

release 版の areka は窓だけのアプリで、端末へは何も出さない。結果は記録・終了コード・開いた画面で伝える。

### 用語

- **起動オプション**: areka.exe を起こすときに付ける `--名前`・`--名前=値`・`/x`・`-x` の形の引数と、その値。`--`・`/`・`-` で始まる引数を、起動オプションの名前として読む。
- **位置の引数**: どの起動オプションの値にもならなかった引数。1 つ目をゴーストのフォルダ、2 つ目をバルーンのフォルダとして読む（今の形）。
- **絶対パス起動**: 位置の引数でゴーストのフォルダを渡した起動（今の形）。
- **いつもの選び方**: 位置の引数のゴーストも `--ghost` も無いときに、起動するゴーストを決める今の段（起動中の印を見て、最後に使ったゴーストの記憶 → 唯一 → 既定 → 無作為）。`--random`（要件 5）と `--option nobootcheck`（要件 6.2）はこの選び方を変える。
- **置き場**: ゴーストとバルーンを置くフォルダ。並びに順番がある（完了 spec `areka-P0-baseware-root-list`）。
- **一時の置き場**: `--ghost` の絶対パスの親のフォルダを、その起動のあいだだけ置き場の並びに足したもの。
- **起動中の印**: 起動するときに書き、きれいに終わったときにだけ消す印。在れば前回はきれいに終わらなかったとみなし、最後に使ったゴーストの記憶を読まずに起動するゴーストを決める（今の決まり）。
- **SHIORI**: ゴーストの台詞と判断を受け持つ部品（DLL）。MAKOTO は台詞を書き換える部品。
- **MCP の待受**: areka が MCP の要求を受ける番号（今は 9801 → 9821 を早い者勝ち、両方塞がっていれば隣の番号へ。`AREKA_MCP_PORT` で 1 つに決められ、0 なら待ち受けない）。
- **記録**: areka のログ。重さは info（知らせ）・warn（無視した入力など）・error（失敗）。

## Boundary Context

- **In scope**:
  - 起動オプションの読み方（4 つの書き方・短い形・`--option` のカンマ区切り・知らない名前・SSP にはあるが areka が扱わない名前・書式の誤り）
  - 位置の引数の今の形を保つこと
  - `--ghost`（絶対パス＝一時の置き場・名前＝置き場から探す・複数のときは最初の 1 体）
  - `--random`
  - `--option` の `nobootcheck`・`dontloadmodule`・受けて何もしない `bootunlock`・`standalone`・`layered`・`region`・`dpiaware`
  - `--sstp-listen`（MCP の待受の指定）
  - `--help`（areka の説明書を開いて終わる）
  - 配布物の説明書 `dist/README.txt` の起動オプションの節
  - 読み方と起動の解決の答えを確かめる決定論のテスト
- **Out of scope**（申し送り先は roadmap と各 brief に登記済み）:
  - `--offline-dump` とその仲間（`--dump-*`） → `areka-P0-offline-surface-dump`
  - `--option readonly` → `areka-P0-readonly-mode`
  - `--install`・`--option callghost`・`--option deletesource`・exe の隣の `initial_install.nar` → `areka-P0-startup-install-option`
  - `--option kiosk`（通知領域のアイコンの後）・`--fmo-name`・`--ukagaka-link`・`--ssf`（MCP で足りる）・`--windowmode`・`--guifont`・`--offline-tool` とその仲間 → 受けて記録だけ（要件 1.6）。働きは作らない
  - 起動中の areka へ処理を任せる仕組みと、二重起動の確かめ（SSTP の受け口の後）
  - `--ghost` で並べた 2 体目以降を同時に起こすこと → `areka-P0-multi-ghost-runtime`
  - `--help ファイル名`（指定のヘルプの頁を開くこと）。値は受けて記録だけ（要件 9.4）
  - 置き場の並びの決め方と保存そのもの（完了 `areka-P0-baseware-root-list`）・開発用のフォルダの別名の表（`areka-P0-dev-folder-alias`）
  - MCP の中継（`areka-mcp-bridge.exe`）の待受の読み方。中継は今までどおり `AREKA_MCP_PORT` を読み、起動オプションは読まない
- **Adjacent expectations**:
  - 絶対パス起動は、実機の試験 3 本（emo2 の実走・MCP の記録の取り出しの実走・起動から終了までの煙の試験）が使っている。この spec はその試験のファイルを変えず、試験はそのまま通る（v0.1.0 のウェーブ C9 で、試験のファイルは `areka-P0-real-run-mcp-kit` の持ち物）。
  - 起動中の印・最後に使ったゴーストの記憶・利用者ごとの置き場への写し・ゴーストの選び方の段（記憶 → 唯一 → 既定 → 無作為）は、今の決まりのまま使う。この spec は段の前に起動オプションの指定を差し込むだけで、段そのものは変えない。
  - 名前でゴーストを探す決まりは、今ある「名前を指してゴーストを切り替える」決まり（`descript.txt` の `name` を先に、一致が無ければフォルダ名）と同じにする。
  - 起動できないとき（ゴーストもバルーンも見つからないなど）の知らせは今までどおり。起動オプションのためにメッセージボックスを足さない。
  - `--option dontloadmodule` に要る「何を聞かれても中身なしで答える」働きは、SHIORI をつなぐ所に足す。v0.1.0 のウェーブ C9 で並走する `areka-P0-mcp-kanade-tools`・`areka-P0-mcp-reload` は、その所に触らない（2026-10-18 に両方の brief で確かめた）ので、この spec に含める。
  - 後に続く `offline-surface-dump`・`readonly-mode`・`startup-install-option` は、この spec の起動オプションの読み取りに項目を足す。そのとき、この spec の読み方の決まり（要件 1）はそのまま効く。

## Requirements

### Requirement 1: 起動オプションの読み方

**Objective:** As a 試験を書く開発者・AI のエージェント, I want SSP と同じ書き方で areka に起動オプションを渡したい, so that SSP 向けの手順や道具の 1 行をそのまま areka に向けられる

#### Acceptance Criteria

1. When `--名前 値` の形で値を取る起動オプションが渡されたとき, the areka shall 次の引数をその値として読む。
2. When `--名前=値` の形の引数が渡されたとき, the areka shall 最初の `=` より前を名前、後ろを値として読み、`--名前 値` と同じ意味に扱う。
3. When 短い形 `/o`・`/r`・`/g`・`/?` またはそれぞれの `-` で始まる形（`-o` など）が渡されたとき, the areka shall それぞれを `--option`・`--random`・`--ghost`・`--help` と同じ意味に扱う。
4. If 短い形に `=` を付けた引数（`/g=名前` など）が渡された場合, then the areka shall その引数を知らない名前として扱う（正典「/x 形式のショートオプションでは = 形式は使えません」）。
5. The areka shall 起動オプションの名前と `--option` の語を、大文字と小文字を区別せずに読む。
6. When 正典に載っているが areka が扱わない起動オプション（`--install`・`--offline-dump`・`--dump-*`・`--offline-tool`・`--target-dir`・`--output`・`--password`・`--fmo-name`・`--ukagaka-link`・`--ssf`・`--windowmode`・`--guifont` と、その短い形）が渡されたとき, the areka shall 正典の値の取り方（値なし・値 1 つ・省略できる値 1 つ・空白区切りで複数）に従ってその値も読み飛ばし、扱わないことを warn で 1 件記録し、起動を続ける。
7. When 正典にも areka にも無い名前の起動オプションが渡されたとき, the areka shall その引数だけを無視して warn で 1 件記録し、次の引数を値として読み飛ばさずに読み続け、起動を続ける。
8. If 値を取る起動オプションの値が無い（最後の引数である、または次の引数が起動オプションである）場合, then the areka shall その起動オプションを値なしとして扱い、そのオプションの要件が値なしの意味を定めていなければ warn で 1 件記録して無視する。
9. When `--option` が 2 回以上渡されたとき, the areka shall すべての語を合わせて受け取る。
10. When `--ghost` が 2 回以上渡されたとき, the areka shall 書かれた順にすべての値を並べて受け取る。
11. When `--option`・`--ghost` 以外の起動オプションが 2 回以上渡されたとき, the areka shall 後に書かれた値を使い、重なったことを warn で 1 件記録する。
12. When 起動オプションを読み終えたとき, the areka shall 受け取った起動オプションの中身（効かせるもの・受けて何もしないもの・無視したもの）を info で 1 件にまとめて記録する。
13. The areka shall 起動オプションの書式の誤りや知らない名前を理由に起動を止めず、メッセージボックスを出さず、終了コードも変えない。
14. If UTF-8 として読めない文字を含む引数が渡された場合, then the areka shall 起動を止めずに、読めない文字を置き換えた形でその引数を読み、warn で 1 件記録する。

### Requirement 2: 位置の引数の今の形を保つ

**Objective:** As a 実機の試験を回す開発者, I want 今の `areka.exe <ゴースト> <バルーン>` の起こし方がそのまま動いてほしい, so that 今ある試験と手順を書き直さずに済む

#### Acceptance Criteria

1. The areka shall どの起動オプションの値にもならなかった引数を位置の引数とし、1 つ目をゴーストのフォルダ、2 つ目をバルーンのフォルダとして今までと同じに扱う。
2. While 起動オプションが 1 つも渡されていないあいだ, the areka shall 位置の引数による起動（絶対パス起動を含む）を、今の areka と同じ結果（選ぶゴーストとバルーン・読む記憶と読まない記憶・起動中の印・利用者ごとの置き場への写し・見つからないときの知らせと終了コード）で行う。
3. If 位置の引数が 3 つ以上渡された場合, then the areka shall 3 つ目以降を無視し、warn で 1 件記録する。
4. If 位置の引数のゴーストと `--ghost` の両方が渡された場合, then the areka shall `--ghost` を使い、位置の引数のゴーストを無視したことを warn で 1 件記録する（`--ghost` のゴーストが見つからなかったときも、位置の引数のゴーストは使わない）。
5. When 位置の引数でバルーンのフォルダが渡されたとき, the areka shall `--ghost` の有無に関わらず、そのバルーンを今までと同じに使う。
6. The areka shall 今の実機の試験 3 本（emo2 の実走・MCP の記録の取り出しの実走・起動から終了までの煙の試験）を、試験のファイルを変えずに通す。

### Requirement 3: `--ghost` に絶対パスを渡す（一時の置き場）

**Objective:** As a ゴーストを作っている開発者, I want 置き場の外にあるゴーストのフォルダを `--ghost` で直に指して起こしたい, so that 置き場を設定し直さずに、開発中のゴーストを普段どおりの決まりで試せる

#### Acceptance Criteria

1. When `--ghost` の最初の値が、`ghost\master\descript.txt` を持つフォルダの絶対パスであるとき, the areka shall 最後に使ったゴーストの記憶と起動中の印に関わらず、そのフォルダのゴーストを起こす。
2. When 要件 3.1 のフォルダの親がどの置き場とも同じでないとき, the areka shall その親のフォルダを、この起動のあいだだけ置き場の並びの末尾に一時の置き場として足す。
3. When 要件 3.1 のフォルダの親がすでに置き場の 1 つであるとき, the areka shall 一時の置き場を足さず、その置き場のゴーストとして起こす。
4. The areka shall 一時の置き場を、areka が覚えている置き場の並びに書き込まない（次の起動には残らない）。
5. The areka shall 一時の置き場を、新しく入れるもの（`.nar` で入れたものなど）の入れ先にしない。
6. The areka shall 一時の置き場を、メニューの「置き場を先頭へ」「置き場を外す」で動かせも外せもしない。
7. When 利用者がメニューの「置き場を追加…」で一時の置き場と同じフォルダを足したとき, the areka shall そのフォルダを普通の置き場として並びに足し、覚える。
8. When 一時の置き場のゴーストを起こすとき, the areka shall 同じフォルダ名のゴーストがほかの置き場にあっても、指されたフォルダそのもののゴーストを起こす。
9. While `--ghost` の絶対パスで起こしたゴーストが動いているあいだ, the areka shall そのゴーストを、置き場の並びにあるほかのゴーストと同じ決まり（シェルとバルーンの記憶の読み書き・最後に使ったゴーストの記憶の書き込み・起動中の印の読み書き）で扱う。
10. If `--ghost` の最初の値が絶対パスだが、`ghost\master\descript.txt` を持つフォルダでない場合, then the areka shall 一時の置き場を足さず、そのことを warn で 1 件記録し、いつもの選び方で起動するゴーストを決める。
11. When 次の起動で一時の置き場が並びに無く、最後に使ったゴーストの記憶が一時の置き場のゴーストを指しているとき, the areka shall 今の決まりどおり、記憶のゴーストが見つからないことを記録して、いつもの選び方の次の段へ進む。

### Requirement 4: `--ghost` に名前を渡す・複数を渡す

**Objective:** As a 試験を書く開発者・AI のエージェント, I want 置き場にあるゴーストを名前で指して起こしたい, so that パスを書かずに、どの PC でも同じ 1 行で同じゴーストを起こせる

#### Acceptance Criteria

1. When `--ghost` の最初の値が絶対パスでないとき, the areka shall その値を名前として、置き場の並びにあるゴーストから探す。
2. The areka shall 名前の探し方を、名前を指してゴーストを切り替えるときと同じ決まり（`descript.txt` の `name` と先に突き合わせ、一致が無ければフォルダ名と突き合わせる）にする。
3. When 名前に一致するゴーストが見つかったとき, the areka shall 最後に使ったゴーストの記憶と起動中の印に関わらず、そのゴーストを起こす。
4. If 名前に一致するゴーストが見つからない場合, then the areka shall そのことを warn で 1 件記録し、いつもの選び方で起動するゴーストを決める。
5. When `--ghost` に値が 2 つ以上渡されたとき, the areka shall 最初の値のゴーストだけを起こし、残りの値を起こさなかったことを warn で 1 件記録する。
6. When `--ghost` の値を読むとき, the areka shall 次の起動オプションの前までの空白区切りの引数をすべて `--ghost` の値として読む（正典「空白区切りで複数起動する」）。
7. If `--ghost` に値が 1 つも無い場合, then the areka shall そのことを warn で 1 件記録し、いつもの選び方で起動するゴーストを決める。

### Requirement 5: `--random`

**Objective:** As a 試験を書く開発者, I want 記憶に関わらず無作為にゴーストを選んで起こしたい, so that 置き場のどのゴーストでも起動が通るかを確かめられる

#### Acceptance Criteria

1. When `--random` が渡され、`--ghost` も位置の引数のゴーストも渡されていないとき, the areka shall 最後に使ったゴーストの記憶・起動中の印・既定のゴーストに関わらず、置き場の並びにあるゴーストの中から無作為に 1 体を選んで起こす。
2. When `--random` で選んだとき, the areka shall 選んだゴーストと候補の数を info で 1 件記録する。
3. When `--random` と、`--ghost` または位置の引数のゴーストが一緒に渡されたとき, the areka shall 指されたゴーストを起こし、`--random` を使わなかったことを info で 1 件記録する。
4. If `--random` が渡されたが置き場にゴーストが 1 体も無い場合, then the areka shall ゴーストが見つからないときの今の知らせと終了で終える。

### Requirement 6: `--option` の語

**Objective:** As a 試験を書く開発者・AI のエージェント, I want SSP の試験の 1 行に並ぶ `--option` の語を areka に渡しても困らないでほしい, so that SSP と同じ約束で試験の areka を立てられる

#### Acceptance Criteria

1. When `--option` の値がカンマで区切られているとき, the areka shall 区切られた語をそれぞれ 1 つの語として受け取る。
2. When `--option nobootcheck` が渡されたとき, the areka shall 起動中の印を読まずに（前回はきれいに終わったものとして）起動するゴーストを決める。
3. While `--option nobootcheck` で起動したあいだ, the areka shall 起動中の印の書き込みと、きれいに終わったときの消去を、今までどおり行う。
4. When `--option` の語 `bootunlock`・`standalone`・`layered`・`region`・`dpiaware` のどれかが渡されたとき, the areka shall その語を受け取り、今の areka では振る舞いが変わらないことを語ごとに info で 1 件記録する（二重起動の確かめも、任せる先の起動中の areka も無く、窓の作りと DPI の追従は今の決まりのまま）。
5. When `--option` の語 `readonly`・`kiosk`・`callghost`・`deletesource` のどれかが渡されたとき, the areka shall その語を受け取るが働きを変えず、まだ扱わないことを語ごとに warn で 1 件記録する。
6. If `--option` に正典に無い語が渡された場合, then the areka shall その語だけを無視して warn で 1 件記録し、ほかの語は受け取る。
7. If `--option` の値が空（`--option` だけ・語の無いカンマ区切り）の場合, then the areka shall 空の語を無視して warn で 1 件記録する。

### Requirement 7: `--option dontloadmodule`（SHIORI を読まずに起こす）

**Objective:** As a シェルやバルーンを確かめる開発者, I want SHIORI を読まずにゴーストを起こしたい, so that 台詞や判断に左右されずに、見た目と起動の道だけを確かめられる

#### Acceptance Criteria

1. While `--option dontloadmodule` で起動したあいだ, the areka shall どのゴーストの SHIORI も MAKOTO も読み込まず、32bit の SHIORI のための補助のプロセスも起こさない。
2. While `--option dontloadmodule` で起動したあいだ, the areka shall ゴーストへのすべての問い合わせ（起動・終了・クリック・MCP からのイベントなど）に、SHIORI が中身なしで答えたときと同じ振る舞いで応じる。
3. When `--option dontloadmodule` でゴーストを起こしたとき, the areka shall SHIORI が起動の問い合わせに中身なしで答えたときと同じ見た目で、シェルを出して起動を終える。
4. When `--option dontloadmodule` で起こしたゴーストが `descript.txt` の SHIORI のファイルを持たない、または読めない SHIORI を指しているとき, the areka shall 起動を失敗にせず、要件 7.3 のとおり起こす。
5. When `--option dontloadmodule` で起動したあいだに利用者がメニューの「終了」を選んだとき, the areka shall SHIORI が終了の問い合わせに中身なしで答えたときと同じに areka を終える。
6. While `--option dontloadmodule` で起動したあいだ, the areka shall メニューや台本で別のゴーストへ切り替えたときも、切り替え先の SHIORI と MAKOTO を読み込まない。
7. When `--option dontloadmodule` で起動したとき, the areka shall SHIORI を読まずに起動したことを info で 1 件記録する。
8. The areka shall `dontloadmodule` のうち PLUGIN・HEADLINE・CALENDAR の部分を、areka にそれらを読む働きがまだ無いので、何もしないものとして扱う。

### Requirement 8: `--sstp-listen`（MCP の待受）

**Objective:** As a 複数の areka や SSP を並べて試験する開発者・AI のエージェント, I want 起動の 1 行で MCP の待受の番号を決めたい, so that 環境変数を設定せずに、ほかの areka や SSP とぶつからない試験の areka を立てられる

#### Acceptance Criteria

1. When `--sstp-listen` が渡されたとき, the areka shall `AREKA_MCP_PORT` と既定の候補（9801 → 9821 → 隣の番号）を使わず、`--sstp-listen` で指定した待ち受け先だけを MCP の待受の候補にする。
2. The areka shall 待ち受け先 `ポート番号`・`localhost:ポート番号`・`127.0.0.1:ポート番号` を、その番号でのループバック（127.0.0.1）の待受として扱う。
3. When 待ち受け先がカンマで区切られて複数あるとき, the areka shall 書かれた順に試し、最初に待ち受けられた 1 つで MCP を待ち受ける（areka の MCP の待受は 1 つなので、SSP と違って複数で同時には待ち受けない）。
4. When `--sstp-listen` の値が `none`（大文字と小文字を区別しない）であるか値が無いとき, the areka shall MCP を待ち受けず、指定によって待ち受けないことを info で 1 件記録する（要件 8.7 の error と区別できる文で）。
5. If 待ち受け先の書式が正しくない（番号が 1〜65535 でない・読めない）場合, then the areka shall その待ち受け先だけを無視し、warn で 1 件記録する。
6. If 待ち受け先が 127.0.0.1 以外のアドレス（`0.0.0.0`・ほかの IPv4 アドレス・`[::1]` を含む `[IPv6アドレス]:ポート番号`）を指している場合, then the areka shall その待ち受け先を使わず、areka の MCP はこの PC の中（127.0.0.1）でだけ待ち受けることを warn で 1 件記録する。
7. If 指定したどの待ち受け先でも待ち受けられなかった（すべて無視されたか、すべて使用中だった）場合, then the areka shall `AREKA_MCP_PORT` や既定の候補へ戻らずに MCP を待ち受けないで起動を続け、そのことを error で記録する（正典「この場合も保存されている設定には戻らない」）。
8. While `--sstp-listen` が渡されていないあいだ, the areka shall MCP の待受を今までどおり（`AREKA_MCP_PORT` か既定の候補で）決める。

### Requirement 9: `--help`

**Objective:** As a areka を初めて試験に使う開発者, I want `--help` で起動オプションの説明を開きたい, so that どのオプションが効くかを、その場で確かめられる

#### Acceptance Criteria

1. When `--help` が渡されたとき, the areka shall areka.exe の隣にある配布物の説明書（起動オプションの節を含む `README.txt`）を OS の既定のアプリで開き、ゴーストを起こさずに終了コード 0 で終える。
2. When `--help` が渡されたとき, the areka shall ほかの起動オプションと位置の引数を効かせず、MCP も待ち受けない。
3. If 説明書のファイルが無い、または開けなかった場合, then the areka shall そのことを error で記録し、ゴーストを起こさずに 0 でない終了コードで終える。
4. When `--help` に値（正典の `--help ファイル名`）が渡されたとき, the areka shall その値を使わずに要件 9.1 の説明書を開き、値を使わなかったことを warn で 1 件記録する。

### Requirement 10: 配布物の説明書の起動オプションの節

**Objective:** As a 配布物を受け取った利用者・開発者, I want 説明書で起動オプションの使い方を読みたい, so that 環境変数を探さずに、試験の起こし方を知れる

#### Acceptance Criteria

1. The areka の配布物の説明書（`dist/README.txt`） shall 起動オプションの節を持ち、書き方（`--名前 値`・`--名前=値`・短い形・`--option` のカンマ区切り）と、この spec が効かせるオプション（`--ghost`・`--random`・`--option nobootcheck`・`--option dontloadmodule`・`--sstp-listen`・`--help`）の働きを書く。`--ghost` は次の起動オプションの前までを全部その値として読むので、位置の引数は `--ghost` より前に書くことも書く。
2. The 配布物の説明書 shall 受けて何もしない `--option` の語と、まだ扱わない起動オプション・語が記録に残って無視されることを書く。
3. The 配布物の説明書 shall 起動オプションが無くても今までどおり起動できること、並べて渡す今の形（`areka.exe <ゴースト> <バルーン>`）が使えること、`--sstp-listen` が `AREKA_MCP_PORT` より強く、MCP の中継は `--sstp-listen` を読まないことを書く。

### Requirement 11: 決定論のテスト

**Objective:** As a areka の開発者, I want 起動オプションの読み方と起動の解決の答えを、実機を使わないテストで固めたい, so that 後に続く spec がオプションを足しても、今の読み方と答えが崩れないと分かる

#### Acceptance Criteria

1. The areka のテスト shall 引数の列から起動オプションの中身への読み方を、要件 1（1.14 を含む）・2.1・2.3・4.6・6.1・6.6・6.7・8.2〜8.6・9.4 の各分岐について、実機・ファイル・環境変数を使わずに確かめる。
2. The areka のテスト shall 起動オプションの中身から起動するゴースト・バルーン・置き場の並びへの答えを、要件 2.2・2.4・3.1〜3.5・3.8・3.10・3.11・4.2〜4.5・4.7・5.1・5.3・5.4・6.2 の各分岐について、作業用のフォルダの検体で確かめる。
3. The areka のテスト shall MCP の待受の候補の決め方を、要件 8.1・8.3・8.4・8.7・8.8 の各分岐について確かめる。
4. The areka のテスト shall SHIORI を読まない起動で、ゴーストへの問い合わせに中身なしで応じ、SHIORI のファイルが無いゴーストでも起動が失敗にならないことを、要件 7.2・7.4・7.6 について実機を使わずに確かめる。
5. The areka のテスト shall 作業用のフォルダと検体を、ワークツリーの `target\` の下にだけ置く。
