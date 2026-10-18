# Requirements Document

## Project Description (Input)
AI のクライアント（Claude Code・Claude Desktop など）から見た areka の MCP の受け口の答えに、事実と合わないものと返らなかったものが 1 件ずつある（brief.md の件 1・件 2）。

- **件 1**: 形の違う `params` の `tools/call`（`name` が文字列でなく数、など）を送ると、受け口は `-32602`（引数が正しくない）でなく `-32601`（そのメソッドは無い・文は `tools/call`）で答える。`tools/call` は在るので答えの番号が事実と合わず、クライアントは「このサーバーはツールを呼べない」と読みうる。この答えは中継（`areka-mcp-bridge`）ごしでも同じで、中継の比較テストの 1 行と差の記録（`doc/ssp-mcp/bridge-diff-areka.md`）の 1 行が今の答えを固定している。
- **件 2**: 2026-10-06、debug 版の areka（検体 claudia）で MCP の `dump_balloon` を呼ぶと、呼び出し側の時間切れ（"task cancelled"）で答えが返らなかった。当時の待ちの場所（UI スレッドで GPU の終わりを待つ呼び出し）は `mcp-dump-images-residue`（PR#254）で無くなったが、今の main の debug 版で返ることは確かめていない。

変えたいこと: 件 1 は、areka が受け付けるメソッドに形の違う `params` が来たら 5 つの版のどれでも・中継ごしでも `-32602` で答え、固定している 2 か所を新しい答えに替える。件 2 は、今の main の debug 版で `dump_balloon` が返ることを claudia と emo2 で 5 回ずつ確かめ、返らなければ原因を直して決定論のテストを 1 本付ける。件 1 の実機の確かめ（中継ごし）と件 2 の確かめは同じ実機の起動で済ませる。

## Introduction

本 spec は、areka の MCP の受け口が AI のクライアントへ返す答えのうち、2 つの穴を埋めるバグの直しと確かめである。

1. 受け付けるメソッドに形の合わない `params` が来たときの答えの番号を、`mcp-tool-entrances` が決めた「型の誤りは `-32602`」に揃える（件 1）。
2. debug 版で `dump_balloon` が答えを返すことを実機で確かめ、返らなければ根本を直す（件 2）。

どちらも、ツールの中身・中継の作り・待ちの上限は変えない。

## Boundary Context

- **In scope**:
  - 受け付けるメソッドに形の違う `params` の要求が届いたときの答え（`-32602`）と、それを 5 つの版で固定する受け口の決定論テスト。
  - 中継の比較テスト（本物の受け口と中継ごしの答えを比べるテスト）の `name` が数の `tools/call` の 1 行と、差の記録 `doc/ssp-mcp/bridge-diff-areka.md` の同じ件の 1 行を新しい答えに替えること。
  - 今の main の debug 版での `dump_balloon` の確かめ（claudia と emo2 で 5 回ずつ）とその記録。返らなかったときは、原因の特定と根本の直しと決定論テスト 1 本。
- **Out of scope**:
  - `params` の中の `arguments` がツールの定義（`inputSchema`）に合わないときの検査（今も `-32602` で、変えない）。未登録のツール名への答え（今も `-32602`）。
  - ツールの中身・中継の作り（中継のコードは直さない見込み）・SSP の受け口がこの形にどう答えるか（測らない）。
  - 通知（`id` の無い要求）の形の違い・`id` の形の違い（どちらも今の答え＝本文なしの受け付けのまま）。
  - MCP の待ちの上限（10 秒）や呼び出し側の時間切れの値の引き上げ（症状止めはしない）。
  - バルーンの描き方・装着の待ちの段数。`dump_surface` の変更（件 2 の原因が `dump_surface` と分け合う所にあると分かったときだけ範囲に入る＝要件 3.6）。
- **Adjacent expectations**:
  - 上流の `mcp-server-core`・`mcp-tool-entrances`・`mcp-stdio-bridge`・`mcp-dump-images`・`mcp-dump-images-residue` はどれも着地済みで、本 spec はその答えの形（エラーの形・`isError` の答え・10 秒の上限とその文）をそのまま使う。
  - 受け口の答えの入口のファイルを同じく触る spec は `mcp-strict-errors`・`mcp-user-response`・`mcp-shiori-query` の 3 本で、同時に走らせるなら触る行を照合する。`mcp-kanade-tools`・`mcp-reload` は受け口のクレートにも `dump_balloon` の処理にも後から答える仕組みにも触らない約束なので重ならない（brief の初めの Adjacent の記述は 2026-10-16 の再測定で改めた）。
  - 中継のテストのファイルを触る `mcp-stdio-bridge-residue` は着地済み（PR#320）で、今は重ならない。
  - 下流は、実機の確かめで `dump_balloon` を使うすべての spec（本 spec の確かめの記録を前提にできる）。

## Requirements

### Requirement 1: 形の違う `params` への答え（件 1）

**Objective:** As an AI のクライアント, I want 在るメソッドに形の違う `params` を送ったときに「引数が正しくない」と返ってほしい, so that 「このサーバーはツールを呼べない」と取り違えずに、送り方を直せる

#### Acceptance Criteria

1. When 受け付けるメソッド（その版の話し方で形の正しい `params` を送れば `-32601` 以外で答えるメソッド。`tools/call`・`tools/list` など）に、そのメソッドの `params` の形に合わない要求（`id` の有るもの）が届いたとき, the areka の MCP の受け口 shall エラーの番号 `-32602` で答える。
2. The areka の MCP の受け口 shall 1 の形の違いとして、少なくとも `tools/call` の `name` が文字列でない（`{"name": 7}`）・`name` が無い、の 2 つを `-32602` で答える。
3. When 受け付けないメソッド（知らない名前・その版の話し方に無いメソッド。例: `2026-07-28` の話し方の `ping`）の要求が届いたとき, the areka の MCP の受け口 shall `params` の形に関わらず、今のとおりエラーの番号 `-32601`（文はメソッドの名前）で答える。
4. When 1 の `-32602` で答えるとき, the areka の MCP の受け口 shall 同じ版の話し方で未登録のツール名の `tools/call` に答える `-32602` と同じ HTTP の状態と同じエラーの欄の組（`code`・`message`）で答える。
5. When 1 の `-32602` で答えるとき, the areka の MCP の受け口 shall `arguments` がツールの定義に合わないときと同じ段（`debug`）で、メソッドの名前を添えた記録を 1 件残す。
6. The areka の MCP の受け口 shall 形の正しい要求・`arguments` がツールの定義に合わない `tools/call`（`-32602`）・未登録のツール名の `tools/call`（`-32602`）・通知・`id` の形の違う要求への答えを、本 spec の前と同じにする。

### Requirement 2: 5 つの版と中継ごしで同じ答え（件 1）

**Objective:** As a 開発者, I want 新しい答えがどの版でも中継ごしでも同じで、それがテストと記録に固定されていてほしい, so that 後の変更で答えが戻ったり、記録が事実と食い違ったりしたら気付ける

#### Acceptance Criteria

1. The areka の MCP の受け口 shall 要件 1.1〜1.4 の答えを、5 つの版の話し方（`2024-11-05`・`2025-03-26`・`2025-06-18`・`2025-11-25`・`2026-07-28`）のどれでも同じにする。
2. When 形の違う `params` の要求が中継ごしに届いたとき, the 中継 shall 受け口へ直接送ったときと同じ答え（`-32602`）を返す。
3. The 受け口の決定論テスト shall 要件 1.1・1.2・1.3 の答えを 5 つの版それぞれで確かめる。
4. The 中継の比較テスト shall `name` が数の `tools/call` の行で直接の答えの番号を `-32602` と確かめ、その行の上の説明も新しい答えに合わせたうえで、中継ごしの答えが直接の答えと同じことを、ほかの行と同じく 5 つの版で確かめる。
5. The 差の記録 `doc/ssp-mcp/bridge-diff-areka.md` shall 「形の違う `params` の `tools/call`」の行の areka の欄を `-32602` の答えに替え、「本 spec で直す」の書きを外す。
6. The 受け口と中継の既存のテスト shall 4 の 1 行の期待と説明のほかは期待を変えずに緑のままとする。
7. When 実機の確かめで debug 版の areka を起こしたとき, the 開発の手順 shall 要件 3 と同じ起動の中で、中継ごしに形の違う `params` の `tools/call` を 1 回送り、答えが `-32602` であることを記録する。

### Requirement 3: debug 版の `dump_balloon` が答えを返すことの確かめ（件 2）

**Objective:** As a 実機の確かめをするエージェント, I want debug 版の areka でも `dump_balloon` が必ず答えを返してほしい, so that バルーンの見た目を絵で確かめられる

#### Acceptance Criteria

1. When 今の main から作った debug 版の areka で、ゴースト claudia と emo2 をそれぞれ起こしバルーンを出した状態で、中継ごしに `dump_balloon` を 5 回ずつ（計 10 回）呼んだとき, the areka shall どの回も、待ちの上限（10 秒）と呼び出し側の時間切れより前に、バルーンの絵か、`mcp-dump-images` が決めた断りの答え（`isError: true` と理由）を返す。
2. The 開発の手順 shall 1 の確かめの条件（areka の版と元のコミット・debug 版であること・ゴースト・バルーン・呼び方）と、各回の結果（絵か、断りの理由か、返らなかったか）と所要時間を本 spec の記録に残す。
3. If 1 の 10 回がどれも答えを返した, then the 開発の手順 shall 件 2 を「再現しない」として記録して閉じ、areka のコードを変えない。
4. If 1 のどれかの回で、areka が待ちの上限の答え（`NG:areka did not respond within 10 seconds`）を返した、または呼び出し側の時間切れで答えが返らなかった, then the 開発の手順 shall 待ちが止まった場所を areka の記録（その分岐が見える段まで開けたもの）で突き止め、根本の原因を直す。
5. If 4 で直した, then the areka shall 直す前には赤になり直した後は緑になる決定論テストを 1 本持ち、直した後に 1 と同じ条件の 10 回がどれも答えを返す。
6. If 4 の原因が `dump_surface` と分け合う待ちの所にある, then the areka shall `dump_surface` についても同じ直しを行い、`dump_surface` が 1 と同じ条件で答えを返すことを確かめる。
7. The 開発の手順 shall 4 の直しで、待ちの上限（10 秒）・呼び出し側の時間切れの値・バルーンの描き方・装着の待ちの段数を変えない。
8. The 開発の手順 shall 実機の確かめのための根・検体・一時のフォルダをワークツリーの `target\` の下だけに置き、32bit の SHIORI のゴースト（claudia）を debug 版で起こすときは `areka.exe` の隣に 32bit 用の補助の実行ファイル（i686 版）を置く。
