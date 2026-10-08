# Brief: areka-P0-open-external-tags

> 2026-10-05 `/kiro-discovery` で起票（開発者「バルーンからアプリを開く用途は結構あると思う」）。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。
> 開発者裁定（同日・議題 5）:
> - 影響の段（`script-impact-tiers`）より**先に入れてよい**。開くたびに必ず記録を残す。
> - SSP の「本体設定 → 外部アプリ」は写さない。「SSP は OS 既定のアプリが無い時代から続いてるアプリなので、いまは OS の受け口を最大限活用すべき」＝OS の既定のアプリ（関連付け）で開き、設定画面は作らない。

## Problem

- **利用者**: バルーンから URL・ファイル・フォルダ・メールを開かせたいゴースト作者と、それを押す利用者。
- 正典には開く系のタグがそろっている（ukadoc）:
  - `\j[ID]`＝`http://～` で URL を開く・`file:///～` で関連付けのアプリでファイルを開く（相対パスは `ghost/master` から）・`mailto:～` で新しいメール（SSP のみ）。
  - `\![open,file,ファイル名]`＝ファイルを実行（`ghost/master` からの相対か絶対・Windows の環境変数・実行ファイルは自動でパスを探す）。
  - `\![open,browser,パラメータ]`・`\![open,explorer,ファイル]`（フォルダなら開く・ファイルなら選んだ状態・`\![open,explorer,種類,名前]` の形）・`\![open,editor,ファイル,表示行]`・`\![open,mailer,パラメータ]`。
- areka ではどれも**黙って無視される**。今バルーンから開けるのは `\![open,readme]` だけで、SHIORI を通しても開けない。

## Current State

- `\j`: `crates/areka-parsers/src/sakura/decode.rs` に読む腕が無く `Raw` になり、`crates/areka-sakura/src/compile.rs` が捨てる（`decode.rs` の注記に `\j` の名前がある）。`mailto:`・`file:///` の扱いはコードのどこにも無い。
- `\![open,…]`: 汎用の `\!` の運び手（`GenericCommand{name:"open",raw_args}`）までは届く。受け取る側の表 `crates/areka/src/emo2_boot/consumer_ledger.rs` に登録があるのは `("open","readme")`（→ `ReadmeSink`）だけ。同ファイルのテストが `("open","browser")` に受け取り手が無いことを固定している（このテストは本 spec で書き替える）。
- OS で開く処理は `crates/areka/src/readme.rs` の `ShellExecuteW(…,"open",…)` 1 か所だけ。`popup-menu-residue` が「World を借りている間の `ShellExecuteW`」を残件に挙げている。
- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`: `\j[ID]`・`\![open,browser,…]`・`\![open,file,…]` などは「無い」・引受先なし。
- MCP の `sakurascript` ツールは入口（`crates/areka/src/mcp/mod.rs` の `ToolCall::Sakurascript`）だけがあり、今も `NG:not implemented yet` を返す（`crates/areka/src/mcp/sakurascript.rs` の `handle`・本物にするのは `mcp-kanade-tools`）。そちらが着地すれば、エージェントの台本でも開ける（2026-10-05 棚卸㉒で訂正）。

## Desired Outcome

- 上の 6 つの形が、OS の既定のアプリ（`ShellExecuteW` の動詞・関連付け）で開く。相対パス・環境変数・実行ファイルのパス探索は ukadoc どおり。
- **開く処理は 1 か所に集める**（例: `open_external(kind, target, origin)`）。そこで毎回 `info` の記録を出す＝何を（URL・ファイル・フォルダ・メール・エディタ）・行き先・どのゴーストの台本か。MCP の `get_log` でも追える。開けなかったときは `error!` で理由を残す（黙らない）。
- `script-impact-tiers` が入ったら、この 1 か所に同意の窓を差し込めば済む形にしておく。
- **「台本から開く系の行き先を取り出す」純粋な関数**を持つ（例: `link_destinations(script) -> Vec<Destination>`）。`\j[X]` と `\![open,○○,X]` の X を順に返す。`link-context-copy`・`balloon-link-hover` が使う。

## Approach

- `\j` は `decode.rs` の短い腕で読み、`\!` の運び手と同じ受け取り手へ流す（`compile.rs` に新しい命令を足さずに済むかを設計で確かめる）。
- 受け取り手は `consumer_ledger.rs` に登録し、実体は `readme.rs` の開く処理を一般化して使い回す。UI スレッドで World を借りたまま `ShellExecuteW` を呼ばない形を設計で決める（`popup-menu-residue` の残件と同じ所）。
- `\![open,editor,ファイル,表示行]` は OS の `edit` 動詞で開き、表示行は ukadoc の「エディタの指定がない場合」と同じく無視する。
- `\j[ID]` の ID が URL でも `file:///` でも `mailto:` でもないとき（旧来の「ID にジャンプ」）の扱いは要件の議題。

## Scope

- **In**: 6 つの形・開く処理の 1 か所・記録・行き先を取り出す関数・決定論テスト（OS は偽の境界で差し替える）・台帳の行の更新・`consumer_ledger.rs` の既存テストの書き替え。
- **Out**:
  - 外部アプリの設定画面（開発者裁定＝作らない）。
  - 同意の窓・影響の段（`script-impact-tiers`）。
  - `\![open,help]`・`\![open,configurationdialog]` など SSP 自身の窓を開く形。
  - `\![execute,http-*]` などネットワーク系。

## Boundary Candidates

- 読み込み（`\j` の腕）と行き先を取り出す関数（純粋）。
- 開く処理の 1 か所（OS の境界）と受け取り手の登録。

## Out of Boundary

- 選択肢の `script:` の実行（`choice-script-prefix`）。
- 右クリックのコピー・ホバー（`link-context-copy`・`balloon-link-hover`）。

## Upstream / Downstream

- **Upstream**: 既存の `\!` の汎用の運び手（`decode_bang` → `GenericCommand`）と受け取る側の表 `consumer_ledger.rs`。
- **Downstream**: `link-context-copy`・`balloon-link-hover`（行き先を取り出す関数）・`script-impact-tiers`（開く処理の 1 か所へ同意の窓を差し込む）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `script-impact-tiers`（brief に本 spec が先に入ることを書き足した）・`popup-menu-residue`（`ShellExecuteW` の残件）・`sakura-time-critical`（`decode_bare` で内部の `\!` へ写す同じ手口）。

## Constraints

- 台本のコンパイルの列（`decode.rs`）と `emo2_boot` の結線の列（`consumer_ledger.rs`）の両方に掛かる。着手の前に両列の先頭と照合する。
- 常時テストは x64 の偽の境界で決定論に（実際に OS のアプリを開かない）。
- 段: 優先（バルーン関係）。規模の見込み M（10〜14）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- 開く処理の新しいファイルは `readme.rs` の子に置く（`main.rs` は `mcp-author-tools` が触る見込み）。`emo2_boot/mod.rs`（`balloon-lifecycle-events`）に触らない。dola の `CueCommand` に種類を足さない（`\j` は汎用の `\!` の運び手へ写す。足さないと済まないと分かったら止めて報告）。
- `decode.rs`・`compile.rs` は C5 の `anchor-tag-canon` が次に触る＝本 spec の腕は既存の腕の並びに 1 本足す形に留める。


---

> **📌 2026-10-05 相互登記（`areka-P0-balloon-lifecycle-events` の要件の討議）**——同じウェーブの `balloon-lifecycle-events` も、`\![` の受け取り手の宣言表 `crates/areka/src/emo2_boot/consumer_ledger.rs` に `("set","balloontimeout")` の受け取り手（種類 1 つ・`canonical()` の登録 1 行）を足す。中身は独立しているが、書く場所が隣り合う。**後からマージする側が、受け取り手の種類・登録の行・表の行数のテストを手で足し直し、数は取り込んだ後に数え直す**（両方が同じ数に書き換えると git が黙って誤った数にまとめる）。
