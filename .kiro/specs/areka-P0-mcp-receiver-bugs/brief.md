# Brief: areka-P0-mcp-receiver-bugs

> 2026-10-11 `/kiro-discovery` で起票（開発者の指示「バグの起票は粒度が小さい。修正範囲がかぶらない範囲でいくつかに集約」）。areka の MCP の受け口のバグの起票 2 本を 1 本にまとめた。元の brief は下の「件 1〜2」に全文を写してある（見出しを 1 段下げただけ・中身は変えていない）。場所は元の起票（10-07・10-11）と棚卸㉓（main `ee3af616`）のときのもの＝着手時に引き直す。

## Problem

AI のクライアントから見た areka の MCP の答えに、事実と合わない・返らないものが 2 件ある。どちらも小さく、単独では 1 本の spec の手続きに見合わない。

- **件 1（旧 `mcp-malformed-call-code`）**: 形の違う `params` の `tools/call`（`name` が数 など）に、`-32602`（引数が正しくない）でなく `-32601`（そのメソッドは無い）で答える。
- **件 2（旧 `dump-balloon-debug-timeout`）**: debug 版の areka で `dump_balloon` が呼び出し側の時間切れ（"task cancelled"）で答えを返さなかった（10-06・`mcp-dump-images-residue` の着地の前のコード）。

## Current State

- 件 1 は原因が分かっている（rmcp が型の合わない要求を型のない要求として読み、`ArekaHandler` が `on_custom_request` を上書きしていない）。今の答えを固定している所が 2 つ在る（`crates/areka-mcp-bridge/tests/real_receiver_test.rs` の 1 行・`doc/ssp-mcp/bridge-diff-areka.md` の 1 行）。
- 件 2 は、当時の待ちの場所（UI スレッドで GPU の終わりを待つ呼び出し）が今のコードに無い＝原因が消えている見込みは高いが、main の debug 版での確かめは済んでいない。
- 触るファイル: 件 1＝`crates/areka-mcp/src/handler.rs` と受け口のテスト・上の 2 か所。件 2＝再現しなければ 0、再現したら `crates/areka/src/mcp/dump_balloon.rs` とその兄弟か `crates/areka/src/mcp/mod.rs` の `later`。2 件のファイルは重ならない。
- 細部は各件の節。

## Desired Outcome

- 件 1: areka が知っているメソッドに形の違う `params` が来たら `-32602`。知らないメソッドは今のまま `-32601`。5 つの版のどれでも・中継ごしでも同じ。上の 2 か所の固定が新しい答えに替わっている。
- 件 2: main の debug 版で `dump_balloon` が返ることが確かめてある（claudia と emo2 で 5 回ずつ）。返らなければ原因を直し、決定論のテストが 1 本付いている。

## Approach

件 1 は `ArekaHandler` に `on_custom_request` を書く（知っている名前の一覧の出どころは設計で決める）。件 2 はまず再現を確かめる。**件 1 の実機の確かめ（中継ごしの比較）と、件 2 の確かめ（debug 版で `dump_balloon` を 5 回）を、同じ実機の起動で済ませる**。

## Scope

- **In**: `handler.rs` の 1 か所・受け口の決定論テスト（形の違う `params` を版ごとに）・中継の比較テストの 1 行・差の記録の 1 行／`dump_balloon` の再現の確かめと記録・（再現したとき）原因の特定と修正。
- **Out**: `arguments` の検査（今も `-32602`）・ツールの中身・中継の作り・SSP の `mcp.exe` がこの形にどう答えるか（測らない）・`dump_surface` の変更（同じ形で再現したときだけ範囲に入れる）・MCP の時間切れの値の引き上げ（症状止めはしない）。

## Boundary Candidates

- 型のない要求の読み分け（受け口）と、その答えを固定する場所。
- `dump_balloon` の要求を UI スレッドへ渡して答えを待つ所／バルーンの読み戻し。

## Out of Boundary

- 通知（`id` の無いもの）の形の違い・`id` の形の違い（今のままで要件に合っている）。
- バルーンの描き方・装着の待ちの段数の変更。

## Upstream / Downstream

- **Upstream**: `mcp-server-core`・`mcp-tool-entrances`・`mcp-stdio-bridge`（✅ 10-11）・`mcp-dump-images`・`mcp-dump-images-residue`（✅ 10-06）。どれも着地済み。
- **Downstream**: 実機の確かめで `dump_balloon` を使うすべての spec。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `mcp-tool-entrances` の決まり「型の誤りは `-32602`」の穴を埋める）。
- **Adjacent**: `crates/areka-mcp/src/handler.rs` を触る spec（`mcp-kanade-tools`・`mcp-reload`・`mcp-shiori-query`・`mcp-user-response`・`mcp-strict-errors`）＝同時に走らせるなら触る行を照合する。`mcp-stdio-bridge-residue`（中継のテストのファイルが重なる）。

## Constraints

- 段は**バグ**・区分 B。
- 規模の見立て: 4〜12 タスク（件 1＝3〜5・件 2＝確かめだけ 1〜2／再現すれば 4〜7）。
- rmcp は `=3.5.0` の固定。受け口の答えを変えるので、中継の `real_receiver_test.rs` を必ず一緒に通す。
- 実機の根と検体はワークツリーの `target\` の下だけ。32bit の SHIORI のゴーストを debug 版で起こすときは、`areka.exe` の隣に i686 の helper が要る（`dev-helper-x64-clobber` の罠）。
- 要件定義のモデル: Opus。

---

## 件 1: 旧 `areka-P0-mcp-malformed-call-code`（元の brief の全文）

> 2026-10-11 `/kiro-discovery` で起票（`areka-P0-mcp-stdio-bridge` の完了時の棚卸）。出どころは同 spec のタスク 5.1（本物の受け口と中継ごしの応答の比較）の気付き。場所の記述は起票時（main `95b34055` を取り込んだ `mcp-stdio-bridge` のブランチ）＝着手時に引き直す。

### Problem

AI のクライアントが、形の違う `params` の `tools/call`（`name` が文字列でなく数、など）を送ると、areka の MCP の受け口は `-32602`（引数が正しくない）でなく `-32601`（そのメソッドは無い・文は `tools/call`）で答える。`tools/call` は在るので、答えの番号が事実と合っていない。クライアントは「このサーバーはツールを呼べない」と読みうる。

### Current State

- `mcp-tool-entrances` は「名前・必須の欄・型の誤りは `-32602`」と決めて作った（`crates/areka-mcp/src/handler.rs` の頭の説明）。これは `params` の**中の `arguments`** が `inputSchema` に合わないときと、未登録のツール名のときに効いている。
- 効いていないのは、`params` そのものの形が MCP の型に合わないとき（`{"name": 7}` など）。rmcp 3.5.0 は型の合わない要求を `CustomRequest`（型のない要求）として読み、`ArekaHandler` は `ServerHandler::on_custom_request` を上書きしていないので、rmcp の既定の `-32601` が返る。
- 中継（`areka-mcp-bridge`）は型のない要求も中身のまま届けるので、中継ごしでも同じ答えになる。
- この答えを固定している所が 2 つある（直すときに一緒に直す）:
  - `crates/areka-mcp-bridge/tests/real_receiver_test.rs` の比較の表の行 `{"id": 8, "method": "tools/call", "params": {"name": 7}, "at": "/error/code", "is": -32601}` と、その上の説明の注釈。
  - `doc/ssp-mcp/bridge-diff-areka.md` の「形の違う `params` の `tools/call`」の行（areka の欄に `-32601` と本 spec の名前が書いてある）。

### Desired Outcome

- areka が知っているメソッド（`tools/call`・`tools/list`・`initialize` など）に形の違う `params` が来たら `-32602` で答える。知らないメソッドは今のまま `-32601`。
- 5 つの版（`2024-11-05`〜`2026-07-28`）のどれでも同じ。中継ごしでも同じ（中継のコードは直さない見込み）。
- 上の 2 か所の固定が新しい答えに替わっている。

### Approach

`ArekaHandler` に `on_custom_request` を書き、メソッド名が areka の知っている名前なら `-32602`、そうでなければ既定の `-32601` を返す。知っている名前の一覧をどこから取るか（rmcp の型の名前の定数か、自分の表か）は設計で決める。

### Scope

- **In**: `crates/areka-mcp/src/handler.rs` の 1 か所・受け口の決定論テスト（形の違う `params` を版ごとに）・中継の比較テストの 1 行・差の記録の 1 行。
- **Out**: `arguments` の検査（今も `-32602`）。ツールの中身。中継の作り。SSP の `mcp.exe` がこの形にどう答えるか（未実測のまま・測らない）。

### Boundary Candidates

- 「型のない要求をどう読み分けるか」（受け口）と「その答えを固定する場所」（受け口のテスト・中継の比較テスト・差の記録）。

### Out of Boundary

- 通知（`id` の無いもの）の形の違い（今は 202・本文なしで、要件に合っている）。
- `id` の形の違い（`"id":{}` など。rmcp が通知として読む＝`mcp-stdio-bridge` のタスク 3.1 の記録）。

### Upstream / Downstream

- **Upstream**: `mcp-server-core`・`mcp-tool-entrances`・`mcp-stdio-bridge`（どれも着地済み）。
- **Downstream**: なし。

### Existing Spec Touchpoints

- **Extends**: なし（完了した `mcp-tool-entrances` の決まり「型の誤りは `-32602`」の穴を埋める）。
- **Adjacent**: `crates/areka-mcp/src/handler.rs` を触る spec（`mcp-kanade-tools`・`mcp-reload`・`mcp-shiori-query`・`mcp-user-response` など MCP の段）。同時に走らせるなら触る行を照合する。`mcp-stdio-bridge-residue`（中継の拾い残し・テストのファイルが重なる）。

### Constraints

- rmcp は `=3.5.0` の固定。版を上げる話は別（`tech.md`）。
- 受け口の答えを変えるので、中継の `real_receiver_test.rs`（本物の受け口との一致）を必ず一緒に通す。
- 区分 B（バグ）。規模 S（3〜5）。

---

## 件 2: 旧 `areka-P0-dump-balloon-debug-timeout`（元の brief の全文）

> 2026-10-07 `ghost-standard-balloon` の完了時の棚卸で起票（`completed/areka-P0-ghost-standard-balloon/tasks.md` の Implementation Notes の 6.2・最終検証 ⑶）。

### Problem

2026-10-06、`ghost-standard-balloon` の実機の確かめ（debug 版の `target\debug\areka.exe`・claudia）で、MCP の `dump_balloon` を呼ぶと呼び出し側の時間切れ（"task cancelled"）になり、答えが返らなかった。バルーンの絵を撮れないと、エージェントがバルーンの見た目を確かめられない。

### Current State

- 起きたのは main `d3000dee` の時点のコード（`mcp-dump-images-residue` の着地の前）。原因は調べていない。
- その後 main に入った `mcp-dump-images-residue`（PR#254・squash `c37d9d61`）は、バルーンの文字の面を UI スレッドで待たずに読み戻すよう直した。この直しで解けている見込みが高いが、**確かめていない**。
- release 版で同じことが起きるかも分からない。

### Desired Outcome

- main の debug 版で `dump_balloon` が、呼び出し側の時間切れより前に答え（絵、または `isError: true` の理由）を返す。
- 再現しない場合は、再現を試みた条件（版・ゴースト・バルーン・呼び方）を記録して閉じる（rejected）。

### Approach

まず再現を確かめる。main の debug 版で claudia（`balloon\claudia`）と emo2 を起こし、MCP の `dump_balloon` を繰り返し呼ぶ。再現したら、待ちがどこで止まっているかを trace の記録で突き止めて根本を直し、決定論テストを 1 本添える。

### Scope
- **In**: `dump_balloon` の答えが返らない原因の特定と修正・再現の記録。
- **Out**: `dump_surface` の変更（同じ形で再現したときだけ範囲に入れる）・MCP の時間切れの値の引き上げ（症状止めはしない）。

### Boundary Candidates
- MCP の受け口（`dump_balloon` の要求を UI スレッドへ渡し、答えを待つ所）
- バルーンの読み戻し（`mcp-dump-images-residue` が直した所）

### Out of Boundary
- バルーンの描き方・装着の待ちの段数の変更

### Upstream / Downstream
- **Upstream**: `mcp-dump-images`（✅ PR#242）・`mcp-dump-images-residue`（✅ PR#254）
- **Downstream**: 実機の確かめで `dump_balloon` を使うすべての spec

### Existing Spec Touchpoints
- **Extends**: なし（完了 spec は直さない）
- **Adjacent**: `ghost-session-test-load-flake`（負荷で待ちが伸びる系の赤）

### Constraints
- 実機の根と検体はワークツリーの `target\` の下だけ。
- 32bit の SHIORI のゴーストを debug 版で起こすときは、`areka.exe` の隣に i686 の helper が要る（`dev-helper-x64-clobber` の罠）。
- 規模の見立て: 再現しなければ XS（記録だけ）、再現すれば S（4〜7）。

### 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - 起きたのは main `d3000dee`（10-06 の朝）のコードで、`mcp-dump-images-residue`（`c37d9d61`・10-06 の夜）より前だと git の履歴で確かめた。
  - そのときの `dump_balloon` は、バルーンの文字の面を GPU から読み戻す間、UI スレッドで待っていた（`mcp-dump-images-residue` の要件の件 1）。今の `crates/areka/src/mcp/dump_balloon.rs` は、読み戻しを後のフレームで待たずに覗き、絵の仕上げは別のスレッドで行う。UI スレッドで GPU の終わりを待つ呼び出しは無くなった＝当時の待ちの場所そのものが無い。原因が消えている見込みは高い。
  - ただし、その後の実機の記録（C4 の各 spec の実機確認）に「debug 版で `dump_balloon` が返った」と書いたものは見つからなかった＝確かめは今も済んでいない。
- 触るファイル: 再現しなければ 0（本 spec の記録だけ）。再現したら `crates/areka/src/mcp/dump_balloon.rs` とその兄弟（待ちの段）か、後から答える仕組み（`crates/areka/src/mcp/mod.rs` の `later`）。
- 規模: 再現しなければごく小さい（1〜2 タスク・main の debug 版で claudia と emo2 を起こして 5 回ずつ呼び、条件と結果を記録して閉じる）。再現したら S（4〜7 タスク）。
- 先に要るもの: なし。触るファイルが決まるまでは、どの spec とも重なり 0。
- 優先度の区分: B（バグ・実機で 1 度起きた）。ただし確かめることが本体の spec。
- 要件定義のモデル: Opus。
- 分割の案: なし。単独の席を取らない手もある: 次に実機で MCP を使う spec（`mcp-kanade-tools` は実機で `dump_surface` を呼ぶ申し送りを持つ）の実機確認に「debug 版で `dump_balloon` を 5 回」を相乗りさせ、返れば本 spec を閉じる。
- 見つけた穴・古くなった記述: なし（brief は 10-07 のまま正しい）。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）

- 前提の変化: 版の段で **v0.1.0**（MCP の列・`mcp-strict-errors` と並ぶ後ろ）。起票（10-11）から触るファイルは動いていない。件 1 の原因どおり `crates/areka-mcp/src/handler.rs`（144 行）に `on_custom_request` はまだ無い。今の答えを固定している 2 か所も在る（`crates/areka-mcp-bridge/tests/real_receiver_test.rs` の 208 行目＝`"params": {"name": 7}` の行と 196 行目の説明・`doc/ssp-mcp/bridge-diff-areka.md` の 93 行目）。件 2 の `crates/areka/src/mcp/dump_balloon.rs`（163 行）も変わらない。
- 触るファイル: 件 1＝`handler.rs`・受け口の決定論テスト（`crates/areka-mcp/src/server_protocol_tests.rs` か新しい兄弟）・上の 2 か所。件 2＝再現しなければ 0、すれば `dump_balloon.rs` か `crates/areka/src/mcp/mod.rs` の `later`。
- 規模: 4〜12（件 1＝3〜5・件 2＝確かめだけ 1〜2／再現すれば 4〜7）。
- 先に要るもの: なし（働きの前提は全部着地済み）。
- 同じウェーブに置けない相手（24 本の中）: `mcp-strict-errors`（`handler.rs`）だけ。列の外: `mcp-stdio-bridge-residue`（`real_receiver_test.rs`）・`mcp-user-response`・`mcp-shiori-query`（`handler.rs`・件 2 で `mcp/mod.rs` を触るときも）。
- 重なり 0: `mcp-kanade-tools`・`mcp-reload`（どちらも `crates/areka-mcp/**`・`dump_balloon.rs`・`mcp/mod.rs` に触らない約束）と、ほかの 24 本すべて。
- 優先度の区分: B（件 1＝答えの番号が事実と違う・件 2＝実機で 1 度返らなかった）。
- 要件定義のモデル: Opus（原因の分かった小さい直しと確かめ）。
- 分割の案: なし。件 2 は、`mcp-kanade-tools` の実機確認（debug 版で `dump_surface` を呼ぶ申し送り）に「`dump_balloon` を 5 回」を相乗りさせ、返れば記録して閉じる手もある（棚卸㉓のまま）。
- 見つけた穴・古くなった記述: 本 brief の Adjacent と件 1 の節は `mcp-kanade-tools`・`mcp-reload` を `handler.rs` を触る spec に数えているが、どちらも触らない（触るのは `mcp-strict-errors`・`mcp-user-response`・`mcp-shiori-query`）。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 版: **v0.1.0（棚卸㉗でゴールを組み直した後も残る）**。「MCP サポートの安定化」（開発者の指示）にそのまま当たる。件 1 は AI のクライアントに「ツールを呼べない」と誤って読ませうる答えの番号、件 2 は実機の確かめでバルーンの絵が撮れない恐れ。どちらも自動の試験の足元。
- 前提の変化: 10-16 から本 spec の触るファイルは動いていない。`crates/areka-mcp/src/handler.rs`（144 行）に `on_custom_request` はまだ無い。今の答えを固定している 2 か所も同じ所に在る（`crates/areka-mcp-bridge/tests/real_receiver_test.rs` 208 行目・`doc/ssp-mcp/bridge-diff-areka.md` 93 行目）。`mcp-stdio-bridge-residue`（10-17）は中継のテストのファイルを変えたが、この 2 行には触れていない。件 2 の `crates/areka/src/mcp/dump_balloon.rs` 163 行・`mcp/mod.rs` 152 行も同じ。
- **前の記述の誤りの直し（本文と件 1 の節の Adjacent）**: 本文の「Existing Spec Touchpoints」と件 1 の節は、`crates/areka-mcp/src/handler.rs` を触る spec として `mcp-kanade-tools`・`mcp-reload` を挙げているが、**どちらも `handler.rs` を触らない**（両方の brief の触るファイルに無い・roadmap の MCP の 3 段目の約束でも `handler.rs` は「3 段目が触らない共有ファイル」）。`handler.rs` を触るのは `mcp-strict-errors`（案内文の最後の 1 文）・`mcp-user-response`・`mcp-shiori-query`。古い文はそのまま残し、ここで正す。
- 触るファイル: 件 1＝`handler.rs`・受け口の決定論のテスト（`server_protocol_tests.rs` か新しい兄弟）・上の 2 か所。件 2＝再現しなければ 0、すれば `dump_balloon.rs` か `mcp/mod.rs` の `later`。
- 規模: 4〜12（件 1＝3〜5・件 2＝確かめだけ 1〜2／再現すれば 4〜7）。
- 先に要るもの: なし。件 2 の確かめは `real-run-mcp-kit` の窓口で書くと 1 本の試験で済む（debug 版の areka を立てて `dump_balloon` を 5 回）。窓口の後に回せば、その試験が残る。
- 同じウェーブに置けない相手（v0.1.0）: `mcp-strict-errors`（`handler.rs`）だけ。`mcp-kanade-tools`・`mcp-reload` とは 0。
- 優先度の区分: B のまま。要件定義のモデル: **Opus** のまま。分割の案: なし。
