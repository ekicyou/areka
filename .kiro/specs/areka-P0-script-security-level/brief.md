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
</content>
</invoke>
