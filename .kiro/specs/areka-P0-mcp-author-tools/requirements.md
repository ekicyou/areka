# Requirements Document

> 本文の実測は **2026-10-05・本ブランチ**（main `ec072853` の上）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、brief が「要件の議題」とした点と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: 伺かの作者が AI エージェント（Claude Code など）と一緒にゴーストを作ろうとしても、エージェントが自分の書いたものを確かめる道が無い。

- 台本を確かめられない。再生しないと、知らないタグや存在しない surface に気付けない。再生すると、ゴーストが実際に喋ってしまう。
- 使えるタグが分からない。areka が対応しているタグと `\!` が分からないので、エージェントは ukadoc の全タグを前提に書いてしまう。
- 設定ファイルを検査できない。descript.txt・surfaces.txt・バルーンの descript.txt は、読み込ませて壊れるまで誤りが見えない。

どれもさくらスクリプトでは届かない（台本は実行することしかできない）。SSP の MCP にも無い。

**今の状態**: areka の MCP は SSP 2.9.05 と同じ 10 本だけを持ち、その 10 本は SSP との逐語一致をテストで固定している（`crates/areka-mcp/src/tools/tools_tests.rs` と `doc/ssp-mcp/tools-list-ssp-2.9.05.json` の照合）。新しいツールはこの表に足せない。アプリ本体への橋（`crates/areka-mcp/src/tools/mod.rs` の `ToolCall`・`crates/areka/src/mcp/mod.rs` の `dispatch`）も 10 本に固定されている。台本の解釈器・設定の読み取り・`\!` の対応表・網羅台帳という部品はそろっている。

**何を変えるか**: SSP 互換の 10 本を変えずに、areka 独自のツールを足せる登録口と橋を作る。その上に、読むだけのツールを 3 本置く。

- 台本を再生せずに確かめるツール（brief の仮名 `check_script`）。
- areka が対応するタグ・`\!` コマンド・SHIORI イベントと状態を返すツール（仮名 `list_capabilities`）。
- ゴーストの設定ファイルを検査するツール（仮名 `validate_ghost`）。

サーバーの指示文と登録案内（help）にも、独自のツールの案内を足す。

> 起票: 2026-10-05 `/kiro-discovery`（開発者「エージェント目線でこんなツールがあったらいいな」「すそ野を広げる」）。開発者の方針（同日）: 「さくらスクリプトを知らないエージェントは無い・スキルでもカバーできる。さくらスクリプトでもできるツールは優先度を下げる」。詳しくは同じフォルダの [brief.md](brief.md)。

## Introduction

### 誰が困っているか

- **AI エージェントとゴーストを作る作者**: エージェントが書いた台本や設定の誤りを、ゴーストを喋らせたり壊したりせずに知りたい。
- **後続の spec の実装者**（`mcp-user-response`・`mcp-shiori-query`）: areka 独自のツールを足したいが、今の表は SSP の 10 本で固定されていて足す場所が無い。

### いま何が起きているか（2026-10-05 実測）

- **10 本の表は固定。** `crates/areka-mcp/src/tools/mod.rs` の `TABLE` は要素数 10 の配列で、`ToolCall` も 10 変種。`crates/areka/src/mcp/mod.rs` の `dispatch` は 10 変種を振り分ける。
- **登録口そのものは表を選ばない。** `crates/areka-mcp/src/registry.rs` の `ToolRegistry::register` は定義と実装の対を受けるだけで、SSP の表かどうかを見ない。
- **指示文は SSP と同じツールだとだけ書いている。** `crates/areka-mcp/src/handler.rs` の `INSTRUCTIONS` は「It exposes the same tools as the MCP server of SSP」と書く。独自のツールが入ると足りなくなる。
- **登録案内は接続の仕方だけ。** `crates/areka-mcp/src/help.rs` の `help_html` は URL・登録コマンド・ポートの説明だけで、ツールの案内は無い。
- **網羅台帳の状態は 7 語。** `doc/ukadoc-coverage/ledger/*.toml` の `status` は `implemented`・`vocabulary-only`・`degraded`・`absent`・`alias`・`not-applicable`・`unclassified`（`doc/ukadoc-coverage/README.md`「状態の 7 語」）。brief の「実装・縮退・未実装」より細かい。
- **前提の spec の状態。** `mcp-tool-entrances` は着地済み。`mcp-strict-errors`（strict のエラーログ）と `script-impact-tiers`（影響の段）はどちらも未着手。
- **strict が記録する対象**（`doc/ssp-mcp/survey.md` の `strict` の説明）: 存在しないサーフェス（`\s`）・アニメーション（`\i`）・バルーン（`\b`）・未知のタグ・`\![コマンド]`・`\&[実体参照]`。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | ツール名の接頭辞（brief の議題） | **付ける**。名前は `areka_check_script`・`areka_list_capabilities`・`areka_validate_ghost` | brief の起票時の推し。SSP が将来同じ名前を足しても衝突しない | 1.2 |
| 2 | `tools/list` の並び | SSP の 10 本を先に SSP の並びで返し、その後に独自のツールを続ける | SSP 向けの手順が先頭の 10 本をそのまま読める | 1.3 |
| 3 | 台本を確かめるツールが診る種類 | strict の 6 類（無い surface・無いアニメーション・無いバルーン・未知のタグ・未知の `\!` コマンド・未知の `\&` 実体参照）と、引数を読めない箇所 | brief の「知らないタグ・不正な引数・無い surface・アニメーション・バルーン」と「strict の文言と揃える」を合わせた | 3.2〜3.4 |
| 4 | strict の文言との揃え方 | `mcp-strict-errors` が未着手なので、**本 spec が種類の名前と文言の形を先に決めて文書に残し**、`mcp-strict-errors` がそれに揃える | 先に着地する側が正本を作る。後から 2 つの文言を突き合わせるより安い | 3.5・7.4 |
| 5 | 影響の段 | **本 spec では返さない**。`script-impact-tiers` が着地するときに同 spec が足す | brief の「着地していれば」。未着手 | 3.9 |
| 6 | 対応はしているが何もしないタグ・縮退のタグを、台本の検査で知らせるか | **知らせない**（状態は一覧のツールで調べる） | brief の検査の項目に無い。台本の 1 箇所と台帳の 1 項目を結ぶ表が新たに要る | 3.10 |
| 7 | 台本の検査の宛先 | SSP の 10 本と同じ `ghost_name` の解決（省略は起動中のゴースト・失敗は同じ `NG:` の文言） | 「今のシェルに無い surface」は宛先が決まらないと判定できない | 2.3・3.1 |
| 8 | 一覧のツールが返す状態の語 | **網羅台帳の語をそのまま返す**（言い換えない） | 言い換えの表を持つと台帳とずれる。`vocabulary-only`（受け取るが何もしない）は作者にとって意味のある区別 | 4.2 |
| 9 | 一覧のツールの絞り込み | 種類（タグ・`\!` コマンド・SHIORI イベント）と状態で絞れる。省略は全件 | 全件は数百項目になるので、エージェントが要る分だけ取れるようにする | 4.4・4.5 |
| 10 | 一覧のツールに台帳の備考を載せるか | **載せない**（綴り・種類・状態・正典の URL だけ） | 備考は開発者向けの文で、作者に読める文であることを保証できない | 4.3 |
| 11 | 設定の検査の宛先 | `ghost_name` が起動中のゴーストに当たればそのゴースト。当たらないフルパスは、そのフォルダ（ゴーストの根かバルーンのフォルダ）を検査する。起動していないゴーストを名前で指すことはできない | brief の「指定したゴースト（またはフォルダ）」。作りかけのゴーストは起動していない | 5.1・5.2 |
| 12 | 設定の検査の範囲（ゴーストの根） | そのゴーストの descript.txt・**全部のシェル**の descript.txt と surfaces.txt。起動中のゴーストなら、今使っているバルーンも | 作者は使っていないシェルの誤りも知りたい | 5.3 |
| 13 | 設定の検査の深さ | 設定ファイルの文面と、参照先のファイルが在るかまで。画像は開かない | 読むだけで、返事の上限 10 秒の内に終える | 5.6 |
| 14 | 診断の重さ | 2 段（誤り＝読めない・参照先が無い／注意＝areka が読まずに捨てる） | 作者が先に直すものを見分けられる | 5.4・5.5 |

## Boundary Context

- **In scope**:
  - SSP 互換の 10 本とは別の、areka 独自のツールの登録口とアプリ本体への橋。
  - 読むだけのツール 3 本（台本の検査・対応の一覧・設定の検査）。
  - サーバーの指示文と登録案内（help）への独自のツールの案内。
  - 診断の種類と文言の正本の文書。
- **Out of scope**:
  - 台本を再生するツール（`sakurascript` で足りる）。
  - 喋らせる・表情を付ける・ゴーストを切り替える・インストールするツール（さくらスクリプトでできる）。
  - 画像を出すツール（`mcp-dump-images` で着地済み）。
  - SSP 互換の 10 本の定義と振る舞い、`ghost_name` の照合の規則（`mcp-ghost-name-match`）。
  - 網羅台帳の中身の正しさ（`coverage-roadmap-refresh`）。本 spec は台帳を読むだけで、行を書き換えない。
  - 影響の段（`script-impact-tiers`）。strict の台本の再生中の記録（`mcp-strict-errors`）。
  - 未実装のタグ・設定キーの実装。
  - 独自のツールを隠す設定（要望が出てから）。
- **Adjacent expectations**:
  - `mcp-strict-errors` は、本 spec が決めた診断の種類の名前と文言の形（要件 3.5）に揃える。本 spec の完了のときに、同 spec の brief へ申し送りを書く。
  - `script-impact-tiers` は、着地するときに台本の検査の結果へ影響の段を足す。
  - `mcp-user-response`・`mcp-shiori-query` は、本 spec の登録口へ自分のツールを足す。足すときに触るファイルは本 spec の設計で固定する（要件 1.6）。
  - MCP の共有ファイル（`crates/areka-mcp/src/` の `handler.rs`・`registry.rs`・`tools/mod.rs`・`tools/bridge.rs`、`crates/areka/src/mcp/mod.rs`）を触る。共有ファイルを触る他の MCP の spec とは同じウェーブに置かない。
  - 規模が 20 タスクを超える見込みになったら、設定の検査（要件 5）を別の spec へ切り出す（brief の Constraints）。

## Requirements

### Requirement 1: areka 独自のツールの登録口

**Objective:** As a 後続の spec の実装者と SSP 向けの手順を持つエージェントの利用者, I want SSP と同じ 10 本を変えずに areka 独自のツールが足されること, so that SSP 向けの手順がそのまま通じ、新しいツールも同じ接続から呼べる

#### Acceptance Criteria

1. When `tools/list` が届く, the areka shall SSP と同じ 10 本に続けて、独自のツール 3 本を返す（合計 13 本・重複する名前 0）。
2. The areka shall 独自のツールの名前をすべて `areka_` で始める（`areka_check_script`・`areka_list_capabilities`・`areka_validate_ghost`）。
3. The areka shall SSP と同じ 10 本を、今と同じ並びで一覧の先頭に返し、独自のツールをその後に置く。
4. The areka shall SSP と同じ 10 本の `name`・`title`・`description`・`inputSchema` と、呼び出したときの振る舞いを変えない（`doc/ssp-mcp/tools-list-ssp-2.9.05.json` との逐語一致を固定している既存のテストは、書き換えずに緑のまま）。
5. The areka shall 独自のツールそれぞれに、`name`・`title`・`description`・`inputSchema` を持たせ、`description` は英文で、SSP に無い areka 独自のツールであることと、ゴーストに何もさせないことを書く。
6. The areka shall 設計の段で、後続の spec が独自のツールを 1 本足すときに触るファイルを固定し、`.kiro/steering/roadmap.md` の干渉の記述に書く（SSP の 10 本の表と、その逐語一致のテストは触らずに足せる形）。同じ文書の「MCP の 3 段目の約束」（`mcp/mod.rs`・`handler.rs` は触らない）は、本 spec が共有ファイルを触るので、合わせて書き直す。
7. If `tools/call` の名前が 13 本のどれでもない, then the areka shall 今と同じく JSON-RPC のエラー `-32602` で答える。

### Requirement 2: 独自のツールに共通の振る舞い

**Objective:** As a AI エージェントとゴーストを作る作者, I want 独自のツールが SSP と同じ 10 本と同じ作法で答え、ゴーストに何の影響も与えないこと, so that 呼び方を覚え直さずに済み、確かめるだけのつもりでゴーストを動かしてしまうことが無い

#### Acceptance Criteria

1. If 独自のツールの必須の引数が無い・引数の型が `inputSchema` と違う, then the areka shall SSP と同じ 10 本と同じ規則で JSON-RPC のエラー `-32602` で答え、ツールの処理を始めない。
2. When 独自のツールが処理に失敗する, the areka shall 本文が `NG:` で始まる英文・`isError: true` の結果で答える。
3. When 独自のツールが `ghost_name` を受ける, the areka shall SSP と同じ 10 本と同じ規則で起動中のゴーストへ解決し、解決できないときは同じ文言（`NG:Specified ghost is not active`・`NG:Cannot find active ghost from specified name`）で答える（要件 5.2 のフォルダの指定を除く）。
4. The areka shall 独自のツール 3 本の処理で、ゴーストに何もさせない（台本の再生 0 回・SHIORI への要求 0 回・表示の変化 0・ファイルへの書き込み 0・ネットワークへの送信 0）。
5. While 独自のツールを処理している, the areka shall ゴーストの描画・台詞の再生・メニューを止めない。
6. If 独自のツールの返事が 10 秒のうちに出ない, then the areka shall SSP と同じ 10 本と同じ時間切れの結果（`NG:areka did not respond within 10 seconds`・`isError: true`）で答える。
7. When 独自のツールに答える, the areka shall SSP と同じ 10 本と同じ形の記録を 1 件残す（ツール名・解決の結果・`isError`）。

### Requirement 3: 台本を再生せずに確かめる（`areka_check_script`）

**Objective:** As a AI エージェントとゴーストを作る作者, I want 台本を再生せずに、areka で動かない箇所を知ること, so that ゴーストを喋らせる前に誤りを直せる

#### Acceptance Criteria

1. When `areka_check_script` が台本（必須）と `ghost_name`（任意）で呼ばれる, the areka shall 台本を再生せずに解釈し、宛先のゴーストが今使っているシェルとバルーンに照らして診断の一覧を返す。
2. If 台本に areka が知らないタグ・知らない `\!` のコマンド・知らない `\&` の実体参照がある, then the areka shall その箇所ごとに診断を 1 件返す。
3. If 台本が、宛先のゴーストの今のシェルに無い surface（`\s`）・無いアニメーション（`\i`）、今のバルーンに無いバルーン（`\b`）を指している, then the areka shall その箇所ごとに診断を 1 件返す。
4. If 台本に、areka が引数を読めずにタグを捨てる・既定の値へ落とす箇所がある, then the areka shall その箇所ごとに診断を 1 件返す。
5. The areka shall 診断 1 件ごとに、種類・台本の中の位置・該当する綴り・説明の文言を載せ、種類の名前と文言の形の正本を `doc/ssp-mcp/` の下の文書に置く（`mcp-strict-errors` が strict のエラーログで同じ名前と文言を使えるようにする）。
6. When 診断が 0 件である, the areka shall 誤りが無いと分かる結果（`isError: false`）で答え、診断が 1 件以上あるときと機械的に見分けられる形にする。
7. When 診断が 1 件以上ある, the areka shall 検査そのものは成功として答える（`isError: false`。`isError: true` は検査ができなかったときだけ）。
8. The areka shall 台本の字面から決まることだけを診断し、再生しないと決まらないこと（`\![raise]` の先の台本・`\![change,shell]` の後の surface・SHIORI が置き換える `%` の中身など）は診断しない（誤って「無い」と答える診断 0 件を優先する）。
9. The areka shall 影響の段を結果に載せない（`script-impact-tiers` が着地するときに足す）。
10. The areka shall areka が知っているタグについて、対応の状態（何もしない・縮退）を理由にした診断を出さない（状態は `areka_list_capabilities` で調べる）。
11. The areka shall `areka_check_script` が「知らない」と答えるタグ・`\!` のコマンドと、同じ台本を再生したときに areka が「知らない」として扱うものを一致させる（検査だけが通して再生で落ちる・検査だけが落として再生で通る、のどちらも 0 件）。ここでの「再生」は、SHIORI から来た台本の再生の経路を指す（MCP の `sakurascript` は 2026-10-05 の時点で未実装のため、比べる相手にしない）。
12. The areka shall 診る種類（3.2〜3.4）の全数を spec 単位の 1 つの表で持ち、表の各行に対して「診断が出る台本」と「診断が出ない台本」の決定論テストを置く。

### Requirement 4: 対応しているものの一覧（`areka_list_capabilities`）

**Objective:** As a AI エージェントとゴーストを作る作者, I want areka が対応しているタグ・`\!` コマンド・SHIORI イベントと、その状態を一覧で得ること, so that areka で動くものだけを使って書ける

#### Acceptance Criteria

1. When `areka_list_capabilities` が呼ばれる, the areka shall さくらスクリプトのタグ・`\!` のコマンド・SHIORI イベントの項目の一覧を返す。
2. The areka shall 項目ごとの状態を、網羅台帳の状態の語（`implemented`・`vocabulary-only`・`degraded`・`absent`・`alias`・`not-applicable`・`unclassified`）のまま返し、各語の意味をツールの `description` に英文で書く。
3. The areka shall 項目ごとに、ukadoc の見出しの綴り・種類・状態・ukadoc の URL を載せる（`alias` の項目には指す先の綴りも載せる。台帳の備考は載せない）。
4. Where 種類の指定（タグ・`\!` コマンド・SHIORI イベント）がある, the areka shall その種類の項目だけを返す。
5. Where 状態の指定がある, the areka shall その状態の項目だけを返す。
6. If 種類・状態の指定が決められた語でない, then the areka shall 使える語を並べた `NG:` の結果で答える。
7. The areka shall 一覧の中身を、同じビルドの網羅台帳（`doc/ukadoc-coverage/ledger/` のさくらスクリプトと SHIORI の台帳）と一致させ、食い違い（項目の過不足・状態の違い）が 1 件でもあれば赤になる決定論テストを置く。
8. The areka shall 配布形（zip を展開した `areka.exe`）で、リポジトリの `doc/` が無くても同じ一覧を返す。
9. The areka shall `ghost_name` を受けず、ゴーストが起動していなくても答える。
10. If `\!` の対応表（`crates/areka/src/emo2_boot/consumer_ledger.rs`）に載っているコマンドが、一覧で `absent` と返る, then the areka shall それを決定論テストで名指しして赤にする（台帳の側の誤りなら台帳を直さず、`coverage-roadmap-refresh` へ申し送る）。

### Requirement 5: ゴーストの設定ファイルの検査（`areka_validate_ghost`）

**Objective:** As a AI エージェントとゴーストを作る作者, I want ゴーストの descript.txt・surfaces.txt・バルーンの descript.txt の誤りを、読み込ませる前に知ること, so that 起動してから壊れた見た目で気付く手戻りが無くなる

#### Acceptance Criteria

1. When `areka_validate_ghost` が `ghost_name` で呼ばれ、起動中のゴーストに当たる（省略を含む）, the areka shall そのゴーストの設定ファイルを検査して診断の一覧を返す。
2. When `ghost_name` が起動中のゴーストに当たらず、実在するフォルダのフルパスである, the areka shall そのフォルダをゴーストの根またはバルーンのフォルダとして検査する（ゴーストが 1 体も起動していなくてもよい）。どちらとも見分けられないフォルダには、理由を書いた `NG:` の結果で答える。
3. The areka shall ゴーストの根の検査で、ゴーストの descript.txt と、そのゴーストが持つ全部のシェルの descript.txt・surfaces.txt を診る。起動中のゴーストでは、今使っているバルーンの descript.txt も診る。
4. If 設定ファイルが読めない・文法として読めない行がある・設定が指すファイル（画像など）が無い, then the areka shall それぞれを「誤り」の診断として 1 件ずつ返す。
5. If 設定ファイルに、areka が読まずに捨てるキー・定義がある, then the areka shall それぞれを「注意」の診断として 1 件ずつ返す。
6. The areka shall 設定ファイルの文面と、参照先のファイルが在るかどうかだけを診る（画像の中身は開かない・SHIORI は読み込まない）。
7. The areka shall 診断 1 件ごとに、重さ（誤り・注意）・ファイルの場所（宛先の根からの相対パス）・行の位置（決まるとき）・該当する綴り・説明の文言を載せる。
8. When 診断が 0 件である, the areka shall 誤りが無いと分かる結果（`isError: false`）で答え、診断が 1 件以上あるときと機械的に見分けられる形にする。
9. The areka shall 同じ設定ファイルを実際に読み込んだときに areka が捨てる・読めないとするものと、検査の診断を一致させる（読み込みと検査で別々の読み取りを持たない）。
10. The areka shall 検体のゴースト（`emo2`・`R_POST_and_KOMAINU`・`konnoyayame`・`claudia`）と既定バルーン（`StayseeBalloon`）の検査で「誤り」の診断が 0 件であること、わざと壊した設定（読めない行・無い画像・読まれないキー）でそれぞれの診断が出ることを決定論テストで固定する。

### Requirement 6: 独自のツールの案内

**Objective:** As a areka を初めて MCP でつなぐエージェントの利用者, I want areka が SSP に無い独自のツールを持つことを接続の時点で知ること, so that エージェントが台本を再生する前に検査のツールを使える

#### Acceptance Criteria

1. The areka shall サーバーの指示文（`initialize`・`server/discover` の `instructions`）に、`areka_` で始まるツールが SSP に無い areka 独自のもので、ゴーストを動かさずに確かめるためのものであることを英文で書く（今ある案内＝先に `get_active_ghost_list` を呼ぶ・未実装の結果の形、は残す）。
2. The areka shall 登録案内（help の HTML）に、独自のツール 3 本の名前と、それぞれが何をするかの 1 行を日本語で載せる。
3. The areka shall 登録案内の独自のツールの名前を、登録した定義から組む（名前を手で 2 か所に書かない）。

### Requirement 7: 決定論テスト・文書・実機確認

**Objective:** As a 後続の spec の実装者, I want 登録口と 3 本の振る舞いがテストで固定され、約束の正本が文書に残ること, so that ツールを足したときや台帳を直したときに何が変わったかが赤で分かる

#### Acceptance Criteria

1. The areka shall `tools/list` が 13 本を要件 1.3 の並びで返すことと、先頭の 10 本が保存した SSP の定義と一致することを、実ソケットの決定論テストで固定する。
2. The areka shall 独自のツール 3 本それぞれについて、引数の検査（全部ある・必須が無い・型違い）と、要件 2.4 の「何もさせない」を決定論テストで固定する。
3. The areka shall 常時テストでネットへ出ず、固定のポート番号を束ねない。
4. The areka shall `doc/ssp-mcp/` の下に、独自のツール 3 本の約束（名前・引数・結果の形・診断の種類と文言・SSP との違い）を書いた文書を置く。
5. The areka shall 実機で確かめ、結果を本 spec の `verification/signoff.md` に残す: ⑴ 配布形の `areka.exe` を既定ゴースト（emo2）で起動し、Claude Code から `tools/list` に 13 本が出る、⑵ 誤りを含む台本（知らないタグ・無い surface）を `areka_check_script` に渡すと診断が返り、ゴーストは喋らない、⑶ `areka_list_capabilities` が一覧を返す、⑷ `areka_validate_ghost` を起動中のゴーストと、起動していないゴーストのフォルダのフルパスの両方で呼んで診断が返る、⑸ 呼んでいる間もゴーストの描画と会話が止まらない。
