# Brief: areka-P0-script-impact-tiers

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

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: M（10〜14）。同意の窓を入れるなら 16〜20 に上振れ＝そのときは「段の表と判定と記録」と「同意の窓」に切る。
- 前提の状態: **待ち**。`script-security-level`（出どころ）と `mcp-kanade-tools`（MCP の台本が実際に再生される）がどちらも未着手。段の表（純粋な判定）だけなら上流なしで書けるが、扱い（出どころと段の組み合わせ）は出どころが無いと試せない。
- 崩れた前提／古くなった位置:
  - `\!` の対応表は `crates/areka/src/emo2_boot/consumer_ledger.rs`（859 行）。段を表へ足すと同ファイルが 1,000 行に近づく＝段の表は新しいファイルに置き、台帳からは名前で引く形が素直。
  - 「高」の候補の `\![execute,dumpsurface]`・`\![execute,dumpballoon]` の台本の命令は、areka では MCP のツール（`dump_surface`／`dump_balloon`・ファイルを書かない）と別物＝台本の命令の側は未実装。表には載せるが止める実物は無い。
  - 「中」の候補の `reload` は `mcp-reload` が作る（未着手）。
- 触るファイル（並走の照合用・見込み）: 新規の段の表（コード 1 か所＋`doc/` の表）・表とコードの一致を判定する検査（新規）・`consumer_ledger.rs`（引く口）・扱いの実行（記録・同意の窓）の新規ファイル。kanade は触らない見込み（出どころは `script-security-level` が運ぶ）。
- 議題（答えで作業が変わるものだけ）: 中・高の扱い（起票時のまま）。同意の窓を作るか（作るならメッセージボックスを避ける裁定との兼ね合い）。
- 見つけた穴: なし。すぐ直せる軽微な修正: この brief の末尾の前に道具の残りかす `</content>`・`</invoke>` の 2 行が紛れている（消してよい）。
