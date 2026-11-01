# Brief: script-impact-tiers

> 起票: 2026-10-05 `/kiro-discovery`（MCP の新しいツールの議論から）。開発者の考え方（同日）:
> - 「エージェントは通常、強大なアクセス権限を最初から持ってます。ベースウェア側で多少制約してもあまり関係ない気がしています」
> - 「セキュリティを区分けするなら、ベースウェアの外に環境影響を与えるものには特に留意が必要。ベースウェア内の環境への影響は中、その場のゴーストの挙動にのみ影響するものは低（＝気にしない）」
> - 「ゴースト切り替えも『低』です。アクター同士の演技の範疇であれば制約なしでよい」

## Problem

MCP のエージェントは `sakurascript` と `raise_event` で、正典のタグを何でも実行させられる（SSP は MCP の台本を Owned SSTP＝`local` として扱い、確認を取らない）。その中には、次のようにベースウェアの外へ影響するものがある。

- ファイルを実行する（`\![open,file]`）。
- ゴミ箱を空にする（`\![execute,emptyrecyclebin]`）。
- ネットワークへ送る（`\![execute,http-post]`）。

エージェントは元から強い権限を持つので、ベースウェアで門を閉ざしても守りにはならない。ただ、**影響の範囲で段を分け、段ごとに気を配る度合いを変える**ことで、デスクトップの利用者が「キャラクターが何をしたか」に気付ける。正典の `SecurityLevel`（`script-security-level`）とは別の軸で、areka の裁量。

## Current State

- 台本の出どころは運ばれていない（`script-security-level` が作る）。
- タグの影響の範囲を表す分類は無い。`\!` は汎用キャリア 1 本で、消費側が名前で選ぶ（`emo2_boot/consumer_ledger.rs` が `\!` の対応表）。
- 利用者の同意を取る窓は無い（失敗の知らせは既定ゴーストの台詞で伝える、という裁定がある＝メッセージボックスは使わない方針）。

## Desired Outcome

影響の段 3 つを定義し、タグごとの段の表を正本として持つ。段の判定はその表 1 つで行う。

| 段 | 範囲 | 扱い |
|---|---|---|
| **高** | ベースウェアの外（OS・ファイル・ネットワーク・他のアプリ） | 特に留意（要件で決める） |
| **中** | ベースウェアの中で、演技を超えるもの（入れる・消す・更新・設定） | 留意（要件で決める） |
| **低** | アクターの演技の範疇（その場の振る舞い・ゴースト同士のやり取り・ゴーストやシェルやバルーンの切替） | **気にしない（制約なし）**＝開発者確定 |

段の当てはめの候補（要件の段で ukadoc の全タグをたどって確定する）:

- **高**:
  - `\![open,file]`（ファイルの実行）・`\![open,browser]`・`\![open,mailer]`・`\![open,editor]`・`\![open,explorer]`・`\j[URL]`
  - `\![execute,http-get/post/put/patch/delete/head/options]`・`\![execute,rss-*]`・`\![execute,ical-*]`・`\![execute,websocket]`・`\![execute,ping]`・`\![execute,nslookup]`
  - `\![execute,extractarchive]`・`\![execute,compressarchive]`・`\![execute,filewatch]`
  - `\![execute,dumpsurface]`・`\![execute,dumpballoon]`（ゴーストの外のフォルダへも書ける）
  - `\![execute,emptyrecyclebin]`・`\m[…]`（他の窓へメッセージ）
- **中**:
  - `\![execute,install]`（`url` と `path`）・ネットワーク更新（`\![updatebymyself]` 系）・`\![vanishbymyself]`
  - `\![execute,createnar]`／`createupdatedata`
  - `\![execute,schedule-add]`／`schedule-delete`（ゴーストの間で共有の予定表）・本体の設定を変えるもの
  - 終了（`\-`）と `reload` の置き場所は要件で決める
- **低**:
  - 文字・`\s`・`\i`・`\b`・`\q`・`\w`・自分への `\![raise]`
  - `\![change,ghost/shell/balloon]`・`\![call,ghost]`・`\![raiseother]`
  - 窓の移動・`\![set,choicetimeout]` など

## Approach

- 段の表を 1 つの正本（`doc/` の下の表＋コードの 1 か所の表）として持ち、消費側が `\!` の名前から段を引く。網羅台帳と同じく、表とコードがずれたら検査で赤にする。
- 中・高の扱いは要件の段で開発者が決める。起票時の Claude の推しは次のとおり。
  - **中**: 通して、必ず記録する（`get_log` で後から追える）。
  - **高**: 出どころが MCP（将来は SSTP も）のときは、台本から触れない areka 自身の窓で利用者の同意を取る。ゴースト自身の台本からのときは、伺かの作法どおり素通しにして記録だけする。
  - 同意の窓をバルーンにしないのは、バルーンの中身をエージェントが台本で書けてしまうから。メッセージボックスを避ける裁定との兼ね合いは、要件で開発者が決める。

## Scope

- **In**:
  - 段の定義と正本の表。
  - 出どころと段の組み合わせごとの扱い。
  - 記録の形。
  - （決まれば）同意の窓。
  - MCP の `reload`・`raise_event`（ゴーストが返した台本は元の出どころを引き継ぐ）への適用。
- **Out**:
  - 正典の `SecurityLevel` の制約（`script-security-level`）。
  - 未実装のタグの実装。
  - エージェント側（Claude Code）の権限の設定。

## Boundary Candidates

- 段の表と判定（純粋・テストしやすい）。
- 扱いの実行（記録・同意の窓）。

## Out of Boundary

- 出どころの運搬（`script-security-level`）。
- MCP のツールの中身（`mcp-kanade-tools`・`mcp-reload`）。

## Upstream / Downstream

- **Upstream**:
  - `script-security-level`（出どころ）
  - `mcp-kanade-tools`（MCP の台本が実際に再生される）
- **Downstream**:
  - 将来の SSTP の受信
  - `x-ukagaka-link`

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `mcp-reload`（`reload` の段）。
  - `popup-menu-residue`（`ShellExecuteW` を使う箇所＝高の実例）。

## Constraints

- 段の当てはめは ukadoc の全タグをたどって作る（https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html ）。上の候補は検索で拾えたもの。
- 開発者の確定は 2 つ。
  - 低は制約なし。
  - ゴースト切替は低。
- 中・高の扱いは未確定で、要件の議題。
- 規模の見立て: M（10〜14 タスク・同意の窓を含めると上振れ）。

## 2026-10-05 `/kiro-discovery`「バルーンのリンクと OS の連携」で足したこと

- 高の段の `\j[URL]`・`\![open,file/browser/explorer/editor/mailer]` は、新しい spec `open-external-tags` が**本 spec より先に**作る（開発者裁定・議題 5＝先に入れてよい・開くたびに必ず記録）。開発者の考え方「エージェントは元から強い権限を持つ」に沿う。
- `open-external-tags` は開く処理を 1 か所（例: `open_external(kind, target, origin)`）に集め、毎回 `info` の記録を出す。本 spec の同意の窓は、その 1 か所へ差し込めば足りる。
- 本 spec が入るまでは、MCP の `sakurascript` から来た `\![open,file,…]` も記録だけで素通しになる。
- 外部アプリは OS の既定のアプリで開く（開発者「いまは OS の受け口を最大限活用すべき」・SSP の外部アプリの設定は写さない）。

## 2026-10-10 棚卸㉓の申し送り

- **議題に足す: SHIORI の台本が、areka の内部の運び名を直に書ける**。完了 `mcp-author-tools` が見つけ、「塞ぐかどうかは別に決める」と残した件（同 spec の `research.md` の 13 節「設計の範囲の外で気づいたこと」）。持ち主がいないので、本 spec の議題に入れる。
  - `\![areka.prop.set,鍵,値]` と書くと、名前がそのまま運ばれて、プロパティの書き込みの受け口 `PropSetCueSink`（`crates/areka-ghost/src/prop_sink.rs`）が拾う。受け付ける鍵は数える鍵 2 つ（起動の回数 `areka.boot.count`・消滅の回数 `areka.vanish.count`）だけで、位置の鍵と知らない鍵は警告して捨てる＝今できるのは、この 2 つの数を書き換えることまで。
  - `\![\f,…]` と書くと、`\f` を運ぶ名前（`crates/areka-sakura/src/contract.rs` の `FONT_TAG_CARRIER`）と同じ綴りなので、文字の層が拾う。できるのは `\f[…]` と同じことだけ。
  - どちらも `\!` の対応表（`ConsumerLedger::canonical`）に行があるので、`check_script` は「拾う者がいる」と答える。
  - 決めること: 内部の名前の `\!` を台本を読む段で落とすか、段の表で「台本からは書けない」と扱うか、今のまま残すか。外から来た台本（MCP）にも同じ道が開いている点を、段の決め方と合わせて見る。

## 2026-11-01 棚卸㉝の再測定（main 13b80ab1）
- **前提の変化**: 先に要った 2 本は着地した（✅ `mcp-kanade-tools` 10-19・✅ `script-security-level` 10-23）＝働きの上で待つものは無い（先頭）。ただし外を開く入口 `crates/areka/src/readme/opener.rs` の `submit(world, destination)`（174 行目）は今も出どころを受けない＝同意の窓や段ごとの記録を差し込むなら、引数か `Destination` に出どころを足す。✅ `mcp-reload`（10-22）で `reload` 系が受け手の表に入った＝「中」の候補の `reload` は実物ができた。✅ `shell-scale-linkage`（11-01）が `\![get,property]`（`(get,property)`）を足した＝段の表に 1 行増える（ベースウェアの中を読むだけ＝低の候補）。
- **受け手の表**: `consumer_ledger.rs` は 560 行・登記 43 行（棚卸㉓の 943 行は古い）。この spec は表に行を足さない。段の表は新しいファイルに置き、受け手の表の全行に段があることを新しいテストで確かめる形なら `consumer_ledger.rs` にも触らない（棚卸㉒の推しのまま）。
- **触るファイル**（行数）: 新規＝段の表（コード 1 か所＋`doc/` の表）・受け手の表との一致の検査・扱いの実行（記録・決まれば同意の窓）／`crates/areka/src/readme/opener.rs`（406・差し込み口）／`crates/areka/src/mcp/check_script_judge.rs`（165）・`check_script.rs`（77）と兄弟のテスト（`check_script_judge_tests.rs` 550・`check_script_tests.rs` 281）／答えの形が変わるなら `crates/areka-mcp/src/tools/check_script.rs`（120）・`doc/ssp-mcp/areka-tools.md`（306）／内部の運び名を台本から書けなくするなら `crates/areka-sakura/src/contract.rs`・`crates/areka-ghost/src/prop_sink.rs`（550）か台本の読み手。kanade・emo-text・`Cargo.lock` には触らない見込み。
- **規模**: 10〜14 タスク。同意の窓を入れるなら 16〜20（`check_script` に段を足す 1〜2 は幅の中）。
- **先に要るもの**: 無し。同じウェーブに置けない相手: `check_script_judge.rs` を触る `range-choice-tag`・`font-size-keywords`・`check-script-arg-checks`・`talk-fast-forward`／`areka-tools.md` を触る `mcp-shiori-query`・`mcp-user-response`／開く処理の `link-context-copy`・`balloon-link-hover`／受け手の表に触る形を選んだときだけ `\!` の名前を足す列（`balloon-communicatebox` を含む）。
- **人の手・机**: 同意の窓を作るなら、窓が出て押せるかの実機の確かめ（画面の操作・排他テストの机）。人の待ち・静かな机・前後の全体テストの比べは要らない。
- **優先度の区分**: A のまま（開発者が段の分け方を示した: 「ベースウェアの外に環境影響を与えるものには特に留意が必要」「ゴースト切り替えも『低』です」）。
- **要件定義のモデル**: **Opus に改める**。中・高の扱いと同意の窓は開発者が要件の議題で決めることで、決まりのどれにも当たらない（出どころの運搬は ✅ `script-security-level` が先例・開く入口は 1 か所に集まっている）。
- **分割の案**: 同意の窓を入れると決まったときだけ「段の表と判定と記録」→「同意の窓」に切る（棚卸㉒から変わらない）。
- **見つけた穴・古くなった記述**: 10-05 の節の例の名前 `open_external(kind, target, origin)` は実物と違う（実物は `opener.rs` の `submit`・出どころの引数は無い）。「高」の候補の `\![execute,dumpsurface]`・`\![execute,dumpballoon]` は台本の命令の側が未実装で受け手の表にも無い＝表には載せるが止める実物は無い。roadmap の台帳の行（399 行目）は依存の `mcp-kanade-tools` に着地の印が無い。

## 2026-11-01 棚卸㉝の後の開発者の決め

2026-11-01 開発者の承認（棚卸㉝の後・ツールチップは「境界領域です。リアクター描画にした方が良い可能性がある。WinUIで描画できる要素ですからね」）＝同意の窓（中・高の段のタグを出どころごとに確かめる窓）の部分だけ windows-reactor 待ち。影響の段の表・判定・記録はこの spec のまま版に縛らない列に残す。同意の窓を外すと規模は 10〜14（棚卸㉝の測り）。
