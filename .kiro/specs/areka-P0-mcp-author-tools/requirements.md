# Requirements Document

> 本文の実測は **2026-10-05・本ブランチ**（main `ec072853` の上）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「裁定」の表のうち「開発者」と書いた行は、2026-10-05 の要件ディスカッションで開発者が決めたもの。「暫定」と書いた行は推奨案による暫定の確定で、覆したら該当要件も改める。

## Project Description (Input)

**誰の何が困っているか**: 伺かの作者が AI エージェント（Claude Code など）と一緒にゴーストを作ろうとしても、エージェントが自分の書いた台本を確かめる道が無い。

- 台本を確かめるには再生するしかない。再生すると、ゴーストが実際に喋ってしまう。`\![change,ghost]` や `\-` が入っていれば、本当に切り替わる・終了する。
- areka が対応しているタグと `\!` が分からないので、エージェントは ukadoc の全タグを前提に書いてしまう。

どちらもさくらスクリプトでは届かない（台本は実行することしかできない）。SSP の MCP にも無い（SSP の 10 本は、動いているゴーストを動かす・覗く道具で、動かさずに確かめる道具が無い）。

**今の状態**: areka の MCP は SSP 2.9.05 と同じ 10 本だけを持ち、その 10 本は SSP との逐語一致をテストで固定している（`crates/areka-mcp/src/tools/tools_tests.rs` と `doc/ssp-mcp/tools-list-ssp-2.9.05.json` の照合）。新しいツールはこの表に足せない。アプリ本体への橋（`crates/areka-mcp/src/tools/mod.rs` の `ToolCall`・`crates/areka/src/mcp/mod.rs` の `dispatch`）も 10 本に固定されている。

**何を変えるか**: SSP 互換の 10 本を変えずに、areka 独自のツールを足せる登録口と橋を作る。その上に、台本を再生せずに確かめるツール `check_script` を 1 本置く。タグ 1 つだけの台本を渡せば、そのタグが areka で効くかどうかの答えにもなる。サーバーの指示文と登録案内（help）にも、独自のツールの案内を足す。

> 起票: 2026-10-05 `/kiro-discovery`（開発者「エージェント目線でこんなツールがあったらいいな」「すそ野を広げる」）。起票時は 3 本（`check_script`・`list_capabilities`・`validate_ghost`）だったが、同日の要件ディスカッションで 1 本に絞った（下の裁定の表）。起票時の中身は同じフォルダの [brief.md](brief.md)。

## Introduction

### 誰が困っているか

- **AI エージェントとゴーストを作る作者**: エージェントが書いた台本の誤りを、ゴーストを喋らせずに知りたい。
- **後続の spec の実装者**（`mcp-user-response`・`mcp-shiori-query`）: areka 独自のツールを足したいが、今の表は SSP の 10 本で固定されていて足す場所が無い。

### いま何が起きているか（2026-10-05 実測）

- **10 本の表は固定。** `crates/areka-mcp/src/tools/mod.rs` の `TABLE` は要素数 10 の配列で、`ToolCall` も 10 変種。`crates/areka/src/mcp/mod.rs` の `dispatch` は 10 変種を振り分ける。
- **登録口そのものは表を選ばない。** `crates/areka-mcp/src/registry.rs` の `ToolRegistry::register` は定義と実装の対を受けるだけで、SSP の表かどうかを見ない。
- **指示文は SSP と同じツールだとだけ書いている。** `crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS` は「It exposes the same tools as the MCP server of SSP」と書く。独自のツールが入ると足りなくなる。
- **登録案内は接続の仕方だけ。** `crates/areka-mcp/src/help.rs` の `help_html` は URL・登録コマンド・ポートの説明だけで、ツールの案内は無い。
- **前提の spec の状態。** `mcp-tool-entrances` は着地済み。`mcp-strict-errors`（strict のエラーログ）と `script-impact-tiers`（影響の段）はどちらも未着手。
- **strict が記録する対象**（`doc/ssp-mcp/survey.md` の `strict` の説明）: 存在しないサーフェス（`\s`）・アニメーション（`\i`）・バルーン（`\b`）・未知のタグ・`\![コマンド]`・`\&[実体参照]`。

### 裁定（要件ディスカッション）

| # | 議題 | 裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | ツール名の接頭辞 | **開発者: 付けない**（「ツール命名に areka を入れないで」）。名前は `check_script` | SSP の 10 本と同じ名付けの流儀に揃える。SSP が将来同じ名前を足したときは、そのときに扱いを決める | 1.2 |
| 2 | 対応の一覧のツール（起票時の `list_capabilities`） | **開発者: 作らない**（「返すコマンドが多すぎる・扱いきれない。タグを渡して、そのタグが有効かどうかわかれば十分」） | 一覧は 1,000 項目を超える。タグ 1 つの台本を `check_script` に渡せば同じ答えが得られる。網羅台帳をバイナリへ埋め込む仕事も要らなくなる | 3.10・3.11 |
| 3 | 設定ファイルの検査のツール（起票時の `validate_ghost`） | **開発者: 本 spec から外す**（「別 spec で起票、あるいはロードマップのメモでよい・優先度は高くなさそう」）。roadmap の覚え書きへ載せる | 設定の読み取りが読めない行を黙って捨てる作りで、読み取りの手直しだけで 8〜10 タスクになる。足りないものの本体は「読み取りが何を捨てたかを言えること」で、ツールはその上に乗る | Boundary |
| 4 | 受け取るが何もしないタグ・`\!` を、台本の検査で知らせるか | **開発者: 知らせる**（一覧を作らないので、調べる道がここだけになる） | 作者が知りたいのは「書いたら動くか」 | 3.10 |
| 10 | 引数の誤りをどこまで診るか | **開発者: 台本を読む段で決まるものだけ**。各機能の受け口が読む引数は診ず、文書に明記して roadmap の覚え書きへ | 受け口の判定は 8 つほどのファイルに散らばり、取り出すと規模が膨らむ。基本のタグは読む段で拾える | 3.4 |
| 11 | `\i`・`\&` の扱い | **開発者（異論なし）**: areka が未対応の間は「知らないタグ」として返す（種類の名前だけ文書に予約する） | 再生したときの実際と同じ答えにする（要件 3.11） | 3.2・3.3 |
| 5 | `tools/list` の並び | 暫定: SSP の 10 本を先に SSP の並びで返し、その後に独自のツールを続ける | SSP 向けの手順が先頭の 10 本をそのまま読める | 1.3 |
| 6 | 台本を確かめるツールが診る種類 | 暫定: strict の 6 類（無い surface・無いアニメーション・無いバルーン・未知のタグ・未知の `\!` コマンド・未知の `\&` 実体参照）、引数を読めない箇所、受け取るが何もしないもの | brief の「知らないタグ・不正な引数・無い surface・アニメーション・バルーン」と「strict の文言と揃える」を合わせ、裁定 4 を足した | 3.2〜3.4・3.10 |
| 7 | strict の文言との揃え方 | 暫定: `mcp-strict-errors` が未着手なので、**本 spec が種類の名前と文言の形を先に決めて文書に残し**、`mcp-strict-errors` がそれに揃える | 先に着地する側が正本を作る。後から 2 つの文言を突き合わせるより安い | 3.5・5.4 |
| 8 | 影響の段 | 暫定: **本 spec では返さない**。`script-impact-tiers` が着地するときに同 spec が足す | brief の「着地していれば」。未着手 | 3.9 |
| 9 | 台本の検査の宛先 | 暫定: SSP の 10 本と同じ `ghost_name` の解決（省略は起動中のゴースト・失敗は同じ `NG:` の文言） | 「今のシェルに無い surface」は宛先が決まらないと判定できない | 2.3・3.1 |

## Boundary Context

- **In scope**:
  - SSP 互換の 10 本とは別の、areka 独自のツールの登録口とアプリ本体への橋。
  - 台本を再生せずに確かめるツール `check_script` 1 本。
  - サーバーの指示文と登録案内（help）への独自のツールの案内。
  - 診断の種類と文言の正本の文書。
- **Out of scope**:
  - 対応しているタグ・`\!`・SHIORI イベントの一覧を返すツール（裁定 2。作らない）。
  - ゴーストの設定ファイル（descript.txt・surfaces.txt・バルーンの descript.txt）を検査するツール（裁定 3。roadmap の覚え書き）。
  - 台本を再生するツール（`sakurascript` で足りる）。
  - 喋らせる・表情を付ける・ゴーストを切り替える・インストールするツール（さくらスクリプトでできる）。
  - SSP 互換の 10 本の定義と振る舞い、`ghost_name` の照合の規則（`mcp-ghost-name-match`）。
  - 網羅台帳（`doc/ukadoc-coverage/`）。本 spec は台帳を読まず、書き換えもしない。
  - 影響の段（`script-impact-tiers`）。strict の台本の再生中の記録（`mcp-strict-errors`）。
  - 未実装のタグの実装。
  - 独自のツールを隠す設定（要望が出てから）。
- **Adjacent expectations**:
  - `mcp-strict-errors` は、本 spec が決めた診断の種類の名前と文言の形（要件 3.5）に揃える。本 spec の完了のときに、同 spec の brief へ申し送りを書く。
  - `script-impact-tiers` は、着地するときに台本の検査の結果へ影響の段を足す。
  - `mcp-user-response`・`mcp-shiori-query` は、本 spec の登録口へ自分のツールを足す。足すときに触るファイルは本 spec の設計で固定する（要件 1.6）。
  - MCP の共有ファイル（`crates/areka-mcp/src/` の `handler.rs`・`registry.rs`・`tools/mod.rs`・`tools/bridge.rs`、`crates/areka/src/mcp/mod.rs`）を触る。共有ファイルを触る他の MCP の spec とは同じウェーブに置かない。

## Requirements

### Requirement 1: areka 独自のツールの登録口

**Objective:** As a 後続の spec の実装者と SSP 向けの手順を持つエージェントの利用者, I want SSP と同じ 10 本を変えずに areka 独自のツールが足されること, so that SSP 向けの手順がそのまま通じ、新しいツールも同じ接続から呼べる

#### Acceptance Criteria

1. When `tools/list` が届く, the areka shall SSP と同じ 10 本に続けて、独自のツール `check_script` を返す（合計 11 本・重複する名前 0）。
2. The areka shall 独自のツールの名前に `areka` などの接頭辞を付けない（SSP の 10 本と同じく、小文字の英単語を `_` でつなぐ）。
3. The areka shall SSP と同じ 10 本を、今と同じ並びで一覧の先頭に返し、独自のツールをその後に置く。
4. The areka shall SSP と同じ 10 本の `name`・`title`・`description`・`inputSchema` と、呼び出したときの振る舞いを変えない（`doc/ssp-mcp/tools-list-ssp-2.9.05.json` との逐語一致を固定している既存のテストは、書き換えずに緑のまま）。
5. The areka shall 独自のツールに `name`・`title`・`description`・`inputSchema` を持たせ、`description` は英文で、SSP に無い areka 独自のツールであることと、ゴーストに何もさせないことを書く。
6. The areka shall 設計の段で、後続の spec が独自のツールを 1 本足すときに触るファイルを固定し、`.kiro/steering/roadmap.md` の干渉の記述に書く（SSP の 10 本の表と、その逐語一致のテストは触らずに足せる形）。同じ文書の「MCP の 3 段目の約束」（`mcp/mod.rs`・`handler.rs` は触らない）は、本 spec が共有ファイルを触るので、合わせて書き直す。
7. If `tools/call` の名前が 11 本のどれでもない, then the areka shall 今と同じく JSON-RPC のエラー `-32602` で答える。

### Requirement 2: 独自のツールに共通の振る舞い

**Objective:** As a AI エージェントとゴーストを作る作者, I want 独自のツールが SSP と同じ 10 本と同じ作法で答え、ゴーストに何の影響も与えないこと, so that 呼び方を覚え直さずに済み、確かめるだけのつもりでゴーストを動かしてしまうことが無い

#### Acceptance Criteria

1. If 独自のツールの必須の引数が無い・引数の型が `inputSchema` と違う, then the areka shall SSP と同じ 10 本と同じ規則で JSON-RPC のエラー `-32602` で答え、ツールの処理を始めない。
2. When 独自のツールが処理に失敗する, the areka shall 本文が `NG:` で始まる英文・`isError: true` の結果で答える。
3. When 独自のツールが `ghost_name` を受ける, the areka shall SSP と同じ 10 本と同じ規則で起動中のゴーストへ解決し、解決できないときは同じ文言（`NG:Specified ghost is not active`・`NG:Cannot find active ghost from specified name`）で答える。
4. The areka shall `check_script` の処理で、ゴーストに何もさせない（台本の再生 0 回・SHIORI への要求 0 回・表示の変化 0・ファイルへの書き込み 0・ネットワークへの送信 0）。
5. While 独自のツールを処理している, the areka shall ゴーストの描画・台詞の再生・メニューを止めない。
6. If 独自のツールの返事が 10 秒のうちに出ない, then the areka shall SSP と同じ 10 本と同じ時間切れの結果（`NG:areka did not respond within 10 seconds`・`isError: true`）で答える。
7. When 独自のツールに答える, the areka shall SSP と同じ 10 本と同じ形の記録を 1 件残す（ツール名・解決の結果・`isError`）。

### Requirement 3: 台本を再生せずに確かめる（`check_script`）

**Objective:** As a AI エージェントとゴーストを作る作者, I want 台本を再生せずに、areka で効かない箇所を知ること, so that ゴーストを喋らせる前に誤りを直せ、使おうとしているタグが areka で効くかも同じ道具で確かめられる

#### Acceptance Criteria

1. When `check_script` が台本（必須）と `ghost_name`（任意）で呼ばれる, the areka shall 台本を再生せずに解釈し、宛先のゴーストが今使っているシェルとバルーンに照らして診断の一覧を返す。
2. If 台本に areka が知らないタグ・知らない `\!` のコマンド・知らない `\&` の実体参照がある, then the areka shall その箇所ごとに診断を 1 件返す。
3. If 台本が、宛先のゴーストの今のシェルに無い surface（`\s`）・無いアニメーション（`\i`）、今のバルーンに無いバルーン（`\b`）を指している, then the areka shall その箇所ごとに診断を 1 件返す。areka が `\i`・`\&` に対応していない間は、どちらも「知らないタグ」の診断として返し、「無いアニメーション」「知らない実体参照」は種類の名前だけを要件 3.5 の文書に予約する。
4. If 台本に、areka が台本を読む段（字句・意味・台本の組み立て）で引数を読めずにタグを捨てる・既定の値へ落とす箇所がある, then the areka shall その箇所ごとに診断を 1 件返す。各機能の受け口が自分で読む引数（`\![move]`・`\![set,zorder]`・`\f` の値・`\_l` など）の誤りは診断せず（`\f` の知らないキーは値の誤りではないので、要件 3.2 の「知らないタグ」として返す）、診ていないタグの一覧を要件 5.4 の文書に書き、`.kiro/steering/roadmap.md` の覚え書きに載せる。
5. The areka shall 診断 1 件ごとに、種類・台本の中の位置・該当する綴り・説明の文言を載せ、種類の名前と文言の形の正本を `doc/ssp-mcp/` の下の文書に置く（`mcp-strict-errors` が strict のエラーログで同じ名前と文言を使えるようにする）。
6. When 診断が 0 件である, the areka shall 誤りが無いと分かる結果（`isError: false`）で答え、診断が 1 件以上あるときと機械的に見分けられる形にする。
7. When 診断が 1 件以上ある, the areka shall 検査そのものは成功として答える（`isError: false`。`isError: true` は検査ができなかったときだけ）。
8. The areka shall 台本の字面から決まることだけを診断し、再生しないと決まらないこと（`\![raise]` の先の台本・`\![change,shell]` の後の surface・SHIORI が置き換える `%` の中身など）は診断しない（誤って「無い」と答える診断 0 件を優先する）。
9. The areka shall 影響の段を結果に載せない（`script-impact-tiers` が着地するときに足す）。
10. If 台本に、areka が受け取るが何もしないタグ・`\!` のコマンドがある, then the areka shall その箇所ごとに、「知らない」とは別の種類の診断を 1 件返す（作者が「書いても効かない」と分かる）。
11. The areka shall `check_script` の答えの出どころを、台本を再生する経路の実際の扱いにする（網羅台帳を出どころにしない）。`check_script` が「知らない」「何もしない」と答えるタグ・`\!` のコマンドと、同じ台本を再生したときに areka がそう扱うものを一致させる（検査だけが通して再生で落ちる・検査だけが落として再生で通る、のどちらも 0 件）。ここでの「再生」は、SHIORI から来た台本の再生の経路を指す（MCP の `sakurascript` は 2026-10-05 の時点で未実装のため、比べる相手にしない）。
12. The areka shall 診る種類（3.2〜3.4・3.10）の全数を spec 単位の 1 つの表で持ち、表の各行に対して「診断が出る台本」と「診断が出ない台本」の決定論テストを置く。
13. When タグ 1 つだけの台本が渡される, the areka shall そのタグが areka で効くかどうかを同じ規則で答える（`description` に、タグが使えるかを確かめる用途にも使えることを英文で書く）。

### Requirement 4: 独自のツールの案内

**Objective:** As a areka を初めて MCP でつなぐエージェントの利用者, I want areka が SSP に無い独自のツールを持つことを接続の時点で知ること, so that エージェントが台本を再生する前に検査のツールを使える

#### Acceptance Criteria

1. The areka shall サーバーの指示文（`initialize`・`server/discover` の `instructions`）に、独自のツールを名前で挙げ、それが SSP に無い areka 独自のもので、ゴーストを動かさずに確かめるためのものであることを英文で書く（今ある案内＝先に `get_active_ghost_list` を呼ぶ・未実装の結果の形、は残す）。
2. The areka shall 登録案内（help の HTML）に、独自のツールの名前と、何をするかの 1 行を日本語で載せる。
3. The areka shall 登録案内の独自のツールの名前を、登録した定義から組む（名前を手で 2 か所に書かない）。

### Requirement 5: 決定論テスト・文書・実機確認

**Objective:** As a 後続の spec の実装者, I want 登録口と `check_script` の振る舞いがテストで固定され、約束の正本が文書に残ること, so that ツールを足したときに何が変わったかが赤で分かる

#### Acceptance Criteria

1. The areka shall `tools/list` が 11 本を要件 1.3 の並びで返すことと、先頭の 10 本が保存した SSP の定義と一致することを、実ソケットの決定論テストで固定する。
2. The areka shall `check_script` について、引数の検査（全部ある・必須が無い・型違い）と、要件 2.4 の「何もさせない」を決定論テストで固定する。
3. The areka shall 常時テストでネットへ出ず、固定のポート番号を束ねない。
4. The areka shall `doc/ssp-mcp/` の下に、独自のツールの約束（名前・引数・結果の形・診断の種類と文言・SSP との違い・後続がツールを足す手順）を書いた文書を置く。
5. The areka shall 実機で確かめ、結果を本 spec の `verification/signoff.md` に残す: ⑴ 配布形の `areka.exe` を既定ゴースト（emo2）で起動し、Claude Code から `tools/list` に 11 本が出る、⑵ 誤りを含む台本（知らないタグ・無い surface・何もしないタグ）を `check_script` に渡すと診断が返り、ゴーストは喋らない、⑶ 誤りの無い台本で診断 0 件が返る、⑷ 呼んでいる間もゴーストの描画と会話が止まらない。
