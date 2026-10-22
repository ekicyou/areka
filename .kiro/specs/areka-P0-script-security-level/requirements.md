# Requirements Document

## Project Description (Input)
伺かの正典では、台本とイベントに「どこから来たか」（出どころ）がついて回る。SHIORI へのリクエストは `SecurityLevel`（`local`／`external`）と `SenderType` を運び、SHIORI はそれを見て外から来た呼び出しを断れる。SHIORI の応答も `SecurityLevel` を返せ、`external` の台本では一部のさくらスクリプトが実行できない。ukadoc は `\![execute,filewatch]` などいくつかのタグに「SSTPなど外部から送られたスクリプトでは実行できない」と明記している。

今の areka はこの出どころを運ばない。SHIORI へは `SecurityLevel: local` を固定で書き、`SenderType` は送らず、応答の `SecurityLevel` は読み飛ばす。台本には出どころの印が無く、正典が外部に禁じるタグ（実装済みの `\![enter,nouserbreakmode]` など）も誰が出しても通る。そのため、ゴーストの作者は外から来た呼び出しと自分の呼び出しを区別できず、利用者は MCP などで外から流された台本から守られない。MCP の `sakurascript`・`raise_event` が本物になった今、守りの無い期間が始まっている。

この spec は、台本とイベントに出どころ（ゴースト自身・`\![raise]`・`\![embed]`・MCP・将来の SSTP など）を付けて再生の終わりまで運び、SHIORI へ正典どおりの `SecurityLevel`・`SenderType` を渡し、応答の `SecurityLevel` を読み、ukadoc が「外部からは不可」と明記したタグを出どころが外部のときに止めて記録に残す。MCP の出どころは SSP に合わせる。SSTP の受信そのもの・`x-ukagaka-link`・影響の段（`script-impact-tiers`）・未実装のタグの実装は範囲外。

## Introduction

伺かのゴーストの作者は、SHIORI へのリクエストの `SecurityLevel`・`SenderType` を見て、外から来た呼び出しを断る（里々の `mkdir`・`lsimg`、蒼空の `File` は `local` でないリクエストで失敗する）。ベースウェアは、出どころが外部の台本では、ukadoc が「外部からは不可」と書いたタグを実行しない。areka はこの出どころを持たないので、ゴーストは呼び出しの出どころを区別できず、正典が外部に禁じるタグも止まらない。

この spec で、areka は台本とイベントに出どころを付けて再生の終わりまで運ぶ。SHIORI へ渡す `SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType` は、ukadoc の定義と SSP の実測に合わせる。応答の `SecurityLevel` を読み、外部扱いの台本では ukadoc が出どころで制限するタグを止めて記録に残す。

### 用語

- **出どころ**: 台本やイベントがどこから来たか。この spec が扱うのは「ゴースト自身（areka の中で起きたイベントへの SHIORI の応答）」「MCP の `sakurascript`（Owned SSTP の SEND 相当）」「MCP の `raise_event`（Owned SSTP の NOTIFY 相当）」の 3 つ。下流の spec が `\![raise]`・`\![embed]`・プロパティシステム・SSTP の受信を足す。
- **実効のセキュリティレベル**: 台本 1 つに付く `local`／`external`。出どころから決まる値に、SHIORI の応答の `SecurityLevel` を重ねた結果（要件 3）。
- **外部扱いの台本**: 実効のセキュリティレベルが `external` の台本、または出どころが Owned でない SSTP の台本（後者は SSTP の受信を持つ将来の spec から生じる）。

### 正典と実測の出典

- ukadoc `spec_shiori3`（https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html）:
  - 要求の `SecurityLevel` の値は `local`／`external` の 2 つ。「少なくともIDヘッダより前に現れる」。
  - `SenderType` の値は 9 つ（`internal`・`external`・`sakuraapi`・`embed`・`raise`・`property`・`plugin`・`sstp`・`communicate`）。複数のときは「カンマでつなげたもの」。見本は `SenderType: internal,raise`。
  - OnTranslate の `SenderType`・`SecurityLevel` は「トランスレート元のイベントの属性を引き継ぐ」。
  - `SenderType` と `SecurityLevel` は別の概念で、「SenderTypeがexternalでもSecurityLevelがlocalとなる場合がある」。
  - `SecurityOrigin` は送信元を URL 風に示す。情報が無いときは `null`。
  - 応答の `SecurityLevel` について、ukadoc は「externalだと一部のスクリプトがセキュリティ機能により実行不能になる」と書く。
- ukadoc `spec_sstp`（https://ssp.shillest.net/ukadoc/manual/spec_sstp.html）: Owned SSTP は「SecurityLevel指定がlocalになり」、ゴースト内部の処理と同じ優先度で扱われる。
- SSP ヘルプ（https://ssp.shillest.net/ukadoc/ssphelp/other.html）: MCP で送った台本は「対象ゴースト自身の処理(Owned SSTP)と同様に扱われ」る。
- ukadoc `list_sakura_script`（https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html）を全件たどった結果、出どころで実行を制限するタグは要件 4.1 の 5 つ。`list_shiori_event` の OnExecuteFileWatchFailure／OnExecuteScheduleFailure は、外部の台本で断ったときの理由を `notauthorized` とする。
- SSP の実測（2026-10-21 23:38〜23:41、SSP の MCP で検体 `pasta-in-windows-dresstest` へ送った要求。SSP の記録 `ghost\master\ssp_shiori_log.txt` で確かめた）:
  - 頭の順は `Charset`・`SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`・`Status`・`ID`・`Reference*`。`SecurityOrigin: null` はどの要求にも付く。
  - SSP の中で起きたイベント: `Sender: SSP`・`SenderType: internal`・`SecurityLevel: local`。
  - MCP の `raise_event` のイベント: `Sender: MCP`・`SenderType: external,sstp`・`SecurityLevel: local`。
  - MCP の `sakurascript` の台本の OnTranslate: `Sender: MCP`・`SenderType: internal`・`SecurityLevel: local`・`Reference1: sstp-send,owned`。
  - MCP の `raise_event` の応答の台本の OnTranslate: `Sender: SSP`・`SenderType: external,sstp`・`SecurityLevel: local`・`Reference1` は空。

## Boundary Context

- **In scope**:
  - 台本とイベントの出どころの区別と、再生の終わりまでの運搬。
  - SHIORI への要求の `SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`（32bit の仲介と同じプロセスの両方の経路）。
  - SHIORI の応答の `SecurityLevel` の読み取りと、実効のセキュリティレベル。
  - ukadoc が出どころで制限するタグの判定の表と、実装済みのタグ（今は `\![enter,nouserbreakmode]`）の止め。
  - OnTranslate の `Reference1` を出どころから導くこと。
  - 網羅台帳の該当行の更新。
- **Out of scope**:
  - SSTP の受信そのものと、そこで使う `Sender`・`SecurityOrigin`・Owned かどうかの判定。
  - `x-ukagaka-link`。
  - areka の裁量の影響の段（`script-impact-tiers`）。
  - 未実装のタグの実装。`\![execute,filewatch]`・`\![execute,schedule-add]`・`\![execute,schedule-delete]`・`\m[…]` と、それらの失敗イベント（`notauthorized`）は、そのタグを実装する spec が作る。
  - `\![raise]`・`\![embed]` などタグが起こすイベントのセキュリティレベルを元の台本から引き継ぐかどうか。ukadoc に記述が無いので、そのタグを実装する spec が決める（この spec は運ぶ仕組みだけを用意する）。
  - Owned SSTP の「`\t` の割り込み禁止の中でも割り込む」扱い。
  - MCP の `check_script` に出どころの診断を出すこと。`check_script` が調べるのは MCP で送る台本で、MCP は Owned SSTP 扱い＝外部扱いにならず、止まるタグが無いため。
  - MCP のツールの振る舞い（`mcp-kanade-tools`）。
- **Adjacent expectations**:
  - `property-query-channels` は、この spec の運ぶ仕組みで `SenderType: internal,property` を送る。
  - `sakura-embed-directive` は、この spec の運ぶ仕組みで `embed` の出どころを付けて送る。
  - `mcp-shiori-query` は、同じ要求の道に `X-MCP-PassThru-*` を足す。
  - `script-impact-tiers` は、この spec の出どころ（MCP か、ゴースト自身か）を読んで影響の段を決める。

## Requirements

### Requirement 1: 出どころの区別と運搬
**Objective:** As a ゴーストの作者, I want 台本とイベントの一つ一つに出どころが付いて最後まで運ばれること, so that 自分の呼び出しと外から来た呼び出しを区別できる

#### Acceptance Criteria
1. When areka の中で起きたイベント（時刻・マウス・起動・終了・切替など）への SHIORI の応答で台本の再生が始まる, the areka shall その台本の出どころを「ゴースト自身」とし、実効のセキュリティレベルの元の値を `local` とする。
2. When MCP の `sakurascript` で台本を受け取る, the areka shall その台本の出どころを「MCP の SEND（Owned SSTP 相当）」とし、実効のセキュリティレベルの元の値を `local` とする。
3. When MCP の `raise_event` のイベントへの SHIORI の応答で台本の再生が始まる, the areka shall その台本の出どころを「MCP の NOTIFY（Owned SSTP 相当）」とし、実効のセキュリティレベルの元の値を `local` とする。
4. When 台本の再生が始まる, the areka shall その台本の出どころと実効のセキュリティレベルを、台本の最後のタグを実行し終えるまで変えずに保つ。
5. When OnTranslate で翻訳された台本の再生が始まる, the areka shall 翻訳の元の台本の出どころを引き継ぐ。
6. When 利用者が選択肢の `script:` を選び、その台本の再生が始まる, the areka shall 選択肢を出した元の台本の出どころと実効のセキュリティレベルを引き継ぐ。
7. When 利用者が選択肢の ID やアンカーを選んでイベントが起きる, the areka shall そのイベントを areka の中で起きたイベントとして扱い、その応答の台本の出どころを「ゴースト自身」とする（利用者の操作で起きたイベントだから。元の台本が外部扱いでも同じ）。
8. Where 下流の機能（`\![raise]`・`\![embed]`・プロパティシステム・SSTP の受信）が台本やイベントを起こす, the areka shall その出どころを ukadoc の `SenderType` の 9 つの語と、Owned かどうか、`local`／`external` の組で表せるようにし、要件 2〜4 の決まりをそのまま当てる。

### Requirement 2: SHIORI への要求のヘッダ
**Objective:** As a ゴーストの作者, I want SHIORI へのリクエストに正典どおりの `SecurityLevel`・`SenderType` などが付くこと, so that SSP で動く判定（`local` 以外を断るなど）が areka でも同じに動く

#### Acceptance Criteria
1. The areka shall SHIORI への GET と NOTIFY のどの要求にも `SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType` を付け、`Charset`・`SecurityLevel`・`SecurityOrigin`・`Sender`・`SenderType`・`Status`・`ID`・`Reference*` の順に並べる（`SecurityLevel` は必ず `ID` より前）。
2. When areka の中で起きたイベントを SHIORI へ送る, the areka shall `SecurityLevel: local`・`SecurityOrigin: null`・`Sender: areka`・`SenderType: internal` を付ける。
3. When MCP の `raise_event` のイベントを SHIORI へ送る, the areka shall `SecurityLevel: local`・`SecurityOrigin: null`・`Sender: MCP`・`SenderType: external,sstp` を付ける。
4. When MCP の `sakurascript` の台本を OnTranslate へ送る, the areka shall `SecurityLevel: local`・`SecurityOrigin: null`・`Sender: MCP`・`SenderType: internal`・`Reference1: sstp-send,owned` を付ける。
5. When SHIORI の応答の台本を OnTranslate へ送る, the areka shall 翻訳の元のイベントの要求の `SenderType` と `SecurityLevel` を引き継ぎ（元の応答の `SecurityLevel` で下がる前の値。台本の実効のセキュリティレベルは要件 3 で決まる）、`Sender: areka`・`SecurityOrigin: null` を付け、`Reference1` を出どころから導く（ゴースト自身の台本では今と同じく空）。
6. When MCP の `raise_event` の応答の台本を OnTranslate へ送る, the areka shall `SenderType: external,sstp`・`SecurityLevel: local` を付け、`Reference1` を空のままにする（SSP の実測と同じ）。
7. Where 出どころが `SenderType` の語を 2 つ以上持つ, the areka shall それらを ukadoc の見本（`internal,raise`）の形で、`internal`／`external` を先頭にしてカンマでつなぐ。
8. The areka shall 32bit の SHIORI の仲介を通る経路と、同じプロセスで SHIORI を呼ぶ経路で、同じ要求に同じヘッダを同じ順で付ける。
9. The areka shall 要求の `Charset`・`Status`・`ID`・`Reference*` の値を、この spec の前と同じに保つ。

### Requirement 3: SHIORI の応答の `SecurityLevel`
**Objective:** As a ゴーストの作者, I want 応答に `SecurityLevel: external` を付けると、その台本が外部扱いで実行されること, so that 正典どおりに自分の台本の権限を下げられる

#### Acceptance Criteria
1. When 台本を返す SHIORI の応答（OnTranslate の応答を含む）が `SecurityLevel: external` を持つ, the areka shall その台本の実効のセキュリティレベルを `external` にする。
2. When 台本を返す SHIORI の応答が `SecurityLevel: local` を持つか、`SecurityLevel` を持たない, the areka shall その台本の実効のセキュリティレベルを、要件 1 で決まった元の値のままにする（応答で `external` から `local` へは上げない）。
3. The areka shall 応答の `SecurityLevel` の値を、英字の大文字と小文字を区別せずに読む。
4. If 応答の `SecurityLevel` の値が `local`・`external` のどちらでもない, the areka shall その頭を無いものとして扱い、イベントの ID と読めなかった値を warn の記録に残す。
5. The areka shall 応答の `Value`・`ErrorLevel`・`ErrorDescription` の読み方を、この spec の前と同じに保つ。

### Requirement 4: 出どころで制限されるタグ
**Objective:** As a 利用者, I want 外から来た台本では、正典が外部に禁じるタグが実行されないこと, so that 外から流された台本にゴーストの操作を奪われない

#### Acceptance Criteria
1. The areka shall ukadoc が出どころで実行を制限する次の 5 つのタグを、制限の区分と ukadoc の注記とともに 1 つの判定の表に持つ。
   - `\![execute,filewatch,…]`: 外部扱いの台本では実行しない。
   - `\![execute,schedule-add,…]`: 外部扱いの台本では実行しない。
   - `\![execute,schedule-delete,…]`: 外部扱いの台本では実行しない。
   - `\![enter,nouserbreakmode]`: 外部扱いの台本では実行しない（Owned SSTP は可）。
   - `\m[umsg,wparam,lparam]`: SSTP 以外の出どころの台本では実行しない（SSTP 専用）。
2. While 再生中の台本が外部扱いである, when その台本が `\![enter,nouserbreakmode]` に届く, the areka shall ユーザーブレイクの禁止へ入らず、台本の残りの再生を続ける。
3. While 再生中の台本の出どころが「ゴースト自身」「MCP の SEND」「MCP の NOTIFY」で、実効のセキュリティレベルが `local` である, the areka shall `\![enter,nouserbreakmode]` を含むすべてのタグを、この spec の前と同じに実行する。
4. When 判定の表によってタグを止める, the areka shall タグの名前・台本の出どころ・実効のセキュリティレベルを warn の記録に残す。
5. The areka shall `\![leave,nouserbreakmode]` を出どころで止めない（ukadoc は `enter` だけを制限する）。
6. Where 判定の表のタグが areka でまだ実装されていない, the areka shall そのタグの今の扱いを変えない。
7. If 判定の表のタグのうち実装済みのものが、判定の表の制限を受けずに実行される状態になる, the areka shall その不一致をフルテストで赤にする。
8. If 判定の表の 5 つのタグのどれかが欠けるか、表に 6 つ目のタグが加わる, the areka shall その差をフルテストで赤にし、ukadoc を引き直して表を直すよう促す。

### Requirement 5: 記録と網羅台帳
**Objective:** As a areka の開発者, I want 出どころの扱いが記録と台帳に正しく出ること, so that 実装の状態と正典との差を追える

#### Acceptance Criteria
1. The areka shall 再生した台本の記録の種別の表示（ゴースト自身の台本は `Ghost`、MCP の SEND の台本は `SSTP(Local,Auth)` など）を、この spec の前と同じ文字で出す。
2. When この spec が完了する, the areka shall 網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の要求の `SecurityLevel`・`SenderType`・`SecurityOrigin` と応答の `SecurityLevel` の行を、実装の状態と持ち主（この spec）に合わせて書き換える。
3. When この spec が完了する, the areka shall 網羅台帳のさくらスクリプトの `\![enter,nouserbreakmode]` と `\m[…]` の行の注記に、出どころによる制限の扱いを書く。
4. When この spec が完了する, the areka shall 正典が沈黙していて areka が決めた扱い（`Sender: areka`・選択肢の ID とアンカーのイベントを「ゴースト自身」とすること・応答の `SecurityLevel` でレベルを上げないこと）を、`doc/COMPAT_ARCHITECTURE.md` §8 の沈黙ルール対応表に登記する。
