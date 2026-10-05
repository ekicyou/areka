# Brief: areka-P0-script-security-level

> 起票: 2026-10-05 `/kiro-discovery`（MCP の新しいツールの議論で、開発者「SSTP にもあるんだけど、セキュリティレベルで実行できるさくらスクリプトに制約が出る。areka に実装されていないと思う。ukadoc で調べてみては」）。

## Problem

伺かの正典では、台本とイベントに「どこから来たか」がついて回る。

- **SHIORI へのリクエスト**: `SecurityLevel`（`local`／`external`）と `SenderType`（`internal`・`external`・`sstp`・`raise`・`embed`・`plugin` など）を渡す。SHIORI はこれを見て、外から来た呼び出しを断れる。実例として、里々の `mkdir`・`lsimg` と蒼空の `File` は `local` 以外を拒む。
- **SHIORI の応答**: `SecurityLevel` を返せる。「externalだと一部のスクリプトがセキュリティ機能により実行不能になる」。
- **タグごとの制約**（ukadoc に明記のもの）:
  - `\![execute,filewatch]`・`\![execute,schedule-add]`・`\![execute,schedule-delete]` は「SSTPなど外部から送られたスクリプトでは実行できない」。
  - `\![enter,nouserbreakmode]` は「通常のSSTPでは使用不可(Auth.SSTP(= Owned SSTP)は可)」。
  - `\m[…]` は SSTP 専用。
- **`OnTranslate` の Reference1**（SSP）: `communicate`・`sstp-send`・`owned`・`remote`・`notranslate` などで出どころを示す。

areka はこの「出どころ」を運ばない。そのため、ゴーストは外から来た呼び出しと自分の呼び出しを区別できず、正典が外部に禁じるタグも素通りする。MCP（SSP では Owned SSTP 相当）と、将来の SSTP の受信・`x-ukagaka-link`（SecurityLevel は必ず `external`）は、どれもこの土台を要る。

## Current State

- SHIORI へのヘッダ `SecurityLevel` は `local` に固定で書き出している（`crates/shiori-host32-host/src/shiori3.rs` の `build_request`）。網羅台帳では `degraded`（`doc/ukadoc-coverage/ledger/shiori.toml` の `ukadoc:spec_shiori3:SecurityLevel:1`）で、「外部由来の呼び出しと区別できない」と登記済み。
- `SenderType`・`SecurityOrigin` は送らない。応答側の `SecurityLevel` は読み飛ばす（同台帳の群 11・`absent`）。
- 台本（talk）に出どころの印が無い。`\![enter,nouserbreakmode]` は実装済み（`balloon-break`）で、誰が出しても通る。
- kanade には `ChangeOrigin`・`BootOrigin`（切替と起動の理由）があるが、台本とイベント一般の出どころではない。

## Desired Outcome

- 台本とイベントに「出どころ」（ゴースト自身・`\![raise]`・`\![embed]`・MCP・将来の SSTP など）が付き、kanade → talk → 各消費者まで運ばれる。
- SHIORI へ、`SecurityLevel` と `SenderType` を出どころから導いた正典どおりの値で渡す。応答の `SecurityLevel` を読み、`external` の台本に正典の制約を掛ける。
- ukadoc が「外部からは不可」と明記したタグを、出どころが外部のときに実行しない。実装済みのタグ（今は `nouserbreakmode`）は実際に止め、未実装のタグは判定の表にだけ載せる。止めたら記録に残す（ログの無い失敗は作らない）。
- MCP の出どころの値は SSP に合わせる（SSP は MCP の台本を Owned SSTP と同じ扱い＝`local`。値の実測は `doc/ssp-mcp/survey.md` で確かめ、無ければ要件で決める）。

## Approach

出どころを 1 つの値の型にして、台本の入口（SHIORI の応答・MCP・将来の SSTP）で付け、kanade の再生の依頼と talk に載せる。SHIORI へのヘッダは、その値から導く 1 か所の関数で作る。タグの制約は、消費側が出どころを見て判断する（`\!` は汎用キャリア 1 本・消費側で名前を選別する、という既存の決まりに沿う）。

## Scope

- **In**:
  - 出どころの型と運び方。
  - SHIORI へのヘッダ `SecurityLevel`・`SenderType`（`SecurityOrigin` も要件で判断）。
  - 応答の `SecurityLevel` の読み取り。
  - ukadoc に明記されたタグの制約。
  - `OnTranslate` の Reference1 への出どころ（`translate-pipeline` が持つ口へ値を渡す）。
  - 網羅台帳の該当行の更新。
- **Out**:
  - SSTP の受信そのもの（予約）。
  - `x-ukagaka-link`。
  - areka の裁量の「影響の段」（`script-impact-tiers`）。
  - 未実装のタグを実装すること。

## Boundary Candidates

- 出どころの型と運搬（kanade・talk）。
- SHIORI のヘッダの組み立て（host32-host の `build_request`・in-proc の経路）。
- タグの制約の判定（消費側）。

## Out of Boundary

- 影響の段の分類と扱い（`script-impact-tiers` が、この出どころの上に載せる）。
- MCP のツールの振る舞い（`mcp-kanade-tools`）。

## Upstream / Downstream

- **Upstream**: なし（`translate-pipeline`・`balloon-break` は着地済み）。
- **Downstream**:
  - `script-impact-tiers`
  - `mcp-kanade-tools`（MCP の台本に出どころを付ける）
  - 将来の SSTP の受信

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - host32-host の列（`property-query-channels`・`makoto-dll-host`）。`shiori3.rs` を触る。
  - kanade の列（`mcp-kanade-tools`・`mouse-drag-events`）。再生の依頼の型を触るので、同じウェーブに置かない。

## Constraints

- 正典の出典:
  - https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html （`SecurityLevel`・`SenderType`・`SecurityOrigin`・応答の `SecurityLevel`）
  - https://ssp.shillest.net/ukadoc/manual/spec_sstp.html （`SecurityLevel`・`Option`・`ID`＝Owned SSTP）
  - https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html （各タグの「外部から送られたスクリプトでは実行できない」の注記）
- 正典の制約の一覧は ukadoc の注記を全件たどって作る（この brief の列挙は検索で拾えたものだけ・要件の段で引き直す）。
- 規模の見立て: M〜L（12〜18 タスク）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模（タスク数）と切るかどうか: M〜L（12〜18）。運ぶ道（下）が長いので 18〜22 に上振れしうる。20 を超えたら、⒜ SHIORI へのヘッダ（`SecurityLevel`・`SenderType`）と応答の `SecurityLevel` の読み取り、⒝ 台本（talk）への出どころの印とタグの制約、に切る（⒜ → ⒝）。
- 前提の状態: 上流なし＝着手できる。ただし kanade の進行の列と host32-host の列の両方に触る。
- 崩れた前提／古くなった位置:
  - ヘッダの組み立ては `crates/shiori-host32-host/src/shiori3.rs` の `build_request`（`SecurityLevel: local` を直に書く）。x64 の組み立ては `crates/areka-ghost/src/shiori_inproc.rs` の共通の組み立て、32bit は `client.rs` の 2 か所。どちらも呼び出しの口 `ShioriBackend`（`crates/areka-kanade/src/shiori/real.rs` のトレイトの定義・`get`／`notify` は `id`・`references`・`status` だけ）の下にある。**`ShioriBackend` の実装は 19 か所**（本物 2・テストの偽物 17）＝引数を変えると全部に波及。既定の実装つきの新しいメソッドで足す形を設計で選ぶ。
  - kanade の呼び出しの型 `ShioriCall`（`msg.rs` の列挙の定義）に出どころの欄が要る。台本の起動 `StartTalk`（`crates/areka-talk/src/lib.rs` の構造体の定義・`talk_id`・`script`・`epilogue`）の構築点は kanade の `schedule/{boot.rs 3, change.rs 2, close.rs 2, steady.rs 4}` と `talk.rs` の 1 か所＝11 か所。
  - `\![enter,nouserbreakmode]` の消費者は `crates/areka/src/emo2_boot/user_break_cue.rs`（台帳の登記は `consumer_ledger.rs`）。
  - **`SenderType` を同じ道で送る spec が 2 本ある**: `property-query-channels`（`SenderType: property`・`shiori3.rs`・`client.rs`・`shiori_inproc.rs`・`msg.rs` の `ShioriCall` を触る）と本 spec。さらに `mcp-shiori-query` が同じ道に `X-MCP-PassThru-*` を足す。入れ物は 1 本が作る。
- 触るファイル（並走の照合用・見込み）: `crates/shiori-host32-host/src/{shiori3.rs, client.rs}`・`crates/areka-ghost/src/shiori_inproc.rs`・`crates/areka-kanade/src/{msg.rs, shiori/real.rs, actor.rs, talk.rs}`・`schedule/{steady,boot,change,close,translate}.rs`（`steady.rs` 947 行・新しい処理は別ファイルへ）・`crates/areka-talk/src/lib.rs`・`crates/areka/src/emo2_boot/user_break_cue.rs`・台帳 `shiori.toml` の `SecurityLevel`・`SenderType`・`SecurityOrigin` の行。
- 議題（答えで作業が変わるものだけ）:
  - `SenderType` の入れ物を本 spec と `property-query-channels` のどちらが作るか（＝2 本の順。本 spec は優先の段・`property-query-channels` はその他の段）。
  - `mcp-kanade-tools` より先に着地させるか（先なら MCP の台本は最初から出どころ付きで流れる・後なら `mcp-kanade-tools` の作った外からの台本の口へ値を足す）。どちらも kanade の進行の列で同時には走らせない。
- 見つけた穴: なし。すぐ直せる軽微な修正: この brief の末尾の前に道具の残りかす `</content>`・`</invoke>` の 2 行が紛れている（消してよい）。

### 棚卸㉒の裁定（2026-10-05）

- `SenderType`・`SecurityLevel` を SHIORI へ運ぶ仕組みの持ち主は本 spec（`property-query-channels` から寄せた）。網羅台帳 `shiori.toml` の `ukadoc:spec_shiori3:SenderType…` の行（持ち主が空）を要件の段で登記する。
- SHIORI への要求に追加のヘッダを運ぶ道は、本 spec・`property-query-channels`・`mcp-shiori-query` の 3 本が要る。入れ物は先に着手した 1 本が作り、`ShioriBackend` の引数を変えずに既定の実装を持つ新しいメソッドを足す形を推す（実装 19 か所への波及を止める）。
