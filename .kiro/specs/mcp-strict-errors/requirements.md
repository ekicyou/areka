# Requirements Document

## Project Description (Input)
AI（Claude Code など）が MCP の `sakurascript`・`raise_event` で流す台本は、今のシェルに無い surface 番号や、綴りを誤ったタグを平気で含む。SSP では `strict` を付けると、そうした箇所がエラーログに残り、AI は `get_log(log_type=error, since_id=…)` で自分の誤りを知って直せる。areka は `strict` を受け取って運ぶところまでは作ってあるが、その値を誰も使っていない＝台本は黙って一部だけ動き、AI は何が悪いのか分からない。

変えたいこと: `strict` 付きの台本の誤りを、`check_script`（台本を再生せずに確かめる areka 独自のツール）と同じ種類・同じ文面でエラーログに 1 件ずつ残し、`sakurascript` の答えで「どの番号より後を読めばよいか」を返す。あわせて、MCP の案内文に残っている「まだのツールは `NG:not implemented yet` を返す」の 1 文（当てはまるツールがもう無い）を消し、`params` がオブジェクトでない要求への答えを「引数の誤り」（`-32602`）に揃える（brief.md の 2026-10-22 の申し送り）。

## Introduction

本 spec は、SSP の MCP を areka へ移す列の最後の 1 本である。SSP の `strict` は「台本の解釈に失敗した箇所（無いサーフェス・無いアニメーション・無いバルーン・知らないタグ・知らない `\![コマンド]`・知らない `\&[実体参照]`）をエラーログに残す」働きで、AI が自分で書いた台本の誤りを、再生した後に知るために使う。v0.1.0 のゴール（SSP 並みの自動の試験）では、AI が台本を流し、エラーログに何も増えていないことで「台本に誤りが無い」と判断できることが要る。

areka には既に、同じ誤りを再生せずに見つける `check_script` がある。その判定は再生が実際に使うのと同じ判定を引いている。本 spec は、台本を受け付けた時点で、その台本の文字列を `check_script` と同じ判定に通し、見つかった誤りをエラーログに残す（**入口で確かめる形**）。この形は、再生の途中でしか分からないこと（台本の中でシェルやバルーンを切り替えた後の `\s`・`\b` など）を記録できない。記録できること・できないことは要件 4 に書き、利用者向けの文書にも載せる。再生の途中まで印を運んで、実際に起きた誤りを記録する形は、本 spec では作らない（要件の討議で選び直す場合は、その足し分を後のウェーブへ回す）。

用語:

- **strict**: `sakurascript`・`raise_event` の引数 `strict`（真・偽・省略）。真のときだけ本 spec の記録をする。
- **診断**: `check_script` が返す 1 件（種類・位置・綴り・文面）。種類の名前と文面の正本は `doc/ssp-mcp/areka-tools.md` の「⑶ 診断の種類と文面」。
- **strict の記録**: 本 spec がエラーログ（`get_log` の `log_type=error`）へ残す 1 件。診断 1 件につき 1 件。
- **通し番号**: エラーログを含む全種類の記録に振られる番号（`get_log` の `#<番号>`・`since_id` で使うもの。`doc/ssp-mcp/log-convention.md` の 5 節・8 節）。

## Boundary Context

- **In scope**:
  - `sakurascript` が `strict` 真で台本を受け付けたとき、その台本の診断を strict の記録としてエラーログに残すことと、答えの文言（`since_id` 付き）。
  - `raise_event` が `strict` 真でゴーストから台本を受け取ったとき、返った台本の診断を strict の記録として残すこと。
  - 記録の種類・文面を `doc/ssp-mcp/areka-tools.md` に揃えることと、その文書に strict の記録の約束（記録できること・できないこと・SSP との違い）を書き足すこと。
  - MCP の案内文（`initialize` の `instructions`・中継が areka の居ないときに返す同じ文）から、最後の「`NG:not implemented yet`」の 1 文を消すこと。
  - `params` がオブジェクトでない要求への答えを、受け口でも中継ごしでも `-32602` にすることと、輸送の差の一覧（`doc/ssp-mcp/transport-diff-areka.md`）・中継の差の一覧（`doc/ssp-mcp/bridge-diff-areka.md`）の該当の行。
  - 上のすべての決定論テスト。
- **Out of scope**:
  - SSTP の `Option: strict`（SSTP の口は予約のまま。同じ記録を将来 SSTP からも立てられればよい）。
  - 誤った箇所の再生のしかたを変えること（無い surface を出さない・知らないタグを捨てる、などの今の振る舞いは据え置く）。
  - 再生の途中まで strict の印を運び、再生中に実際に起きた誤りを記録すること（要件 4 の限界を埋める形。選ぶなら後のウェーブ）。
  - `check_script` の答え・判定・種類・文面を変えること（本 spec は同じものを使うだけ）。
  - `\i`・`\&` への対応（今は `unknown_tag` として記録される。対応したときに予約の種類 `missing_animation`・`unknown_entity` を使い始めるのは、対応する spec の仕事）。
  - 通知（`id` の無い要求）と、`params` が `null` の要求への答え（今のまま）。
- **Adjacent expectations**:
  - 上流は着地済み: `mcp-kanade-tools`（`strict` を受けて運ぶ口・`sakurascript`／`raise_event` の答え）・`mcp-log-history`（エラーログと通し番号・記録の取り決め `doc/ssp-mcp/log-convention.md`）・`mcp-author-tools`（`check_script` と種類・文面の正本）・`mcp-reload`（`NG:not implemented yet` を返す最後のツールに中身を入れた）・`mcp-receiver-bugs`（`params` の形の違いへの答えの前例）。本 spec はそれらの答えの形と取り決めをそのまま使う。
  - 再生の側（台本の解釈・表情・バルーン・`\!` の受け取り手）は今のまま動き、今出している警告の記録も今のままである。strict の記録は、それらと別に足される。
  - 同じウェーブで並走する spec と重ならないよう、`\!` の受け取り手の台帳・表示の層の表示の処理・kanade の知らせの型・`get_property` の処理は変えない（案内文の 1 文はこのウェーブで本 spec が持つ）。

## Requirements

### Requirement 1: `sakurascript` の strict の答え

**Objective:** As a 台本を流す AI, I want `strict` 付きで台本を流したら、どの番号より後のエラーログを読めばよいかを答えで知りたい, so that その台本で起きた誤りだけを `get_log` で取り出せる

#### Acceptance Criteria

1. When `sakurascript` が `strict` 真の台本を受け付けたとき, the areka shall 答えの本文を `OK:strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=<n>)`（SSP の実測と同じ文・`<n>` は 10 進の数）とし、`isError` を偽にする。
2. The areka shall 1 の `<n>` を、その台本の strict の記録を 1 件も残す前の時点で最後に振られた通し番号（1 件も無ければ 0）とする。
3. When 1 の答えを受け取った AI が `get_log` を `log_type=error`・`since_id=<n>` で呼んだとき, the areka shall その台本の strict の記録をすべて返す（その間に他の出どころが残した記録が混ざることはある）。
4. While `strict` が偽か省略のとき, the areka shall `sakurascript` の答えを今のまま（受け付けたら `OK`）にする。
5. If `sakurascript` の台本が空・ゴーストが今は台本を受け付けない・宛先のゴーストが居ない・答えの前にゴーストが降りた, then the areka shall `strict` の値に関わらず今と同じ `NG:` の答えを返し、strict の記録を残さない。

### Requirement 2: strict の記録の中身

**Objective:** As a 台本を流す AI, I want エラーログの 1 件から、何がどこで誤っていて再生でどうなったかを読み取りたい, so that 台本のどこをどう直せばよいかが分かる

#### Acceptance Criteria

1. When strict の記録を残すとき, the areka shall 診断 1 件につき 1 件を、`get_log` の `log_type=error` で読める記録として残す。
2. The areka shall 各記録の名（`get_log` の `<名>`）を、台本を送った先のゴーストの名前（`get_active_ghost_list` が返す値と同じもの）にする＝`get_log` の `ghost_name` での絞り込みで当たる。
3. The areka shall 各記録の本文から、診断の種類の名前・台本の中の位置（`check_script` と同じく、台本の先頭を 0 とした文字の数）・該当箇所の綴りそのまま・文面の 4 つを読み取れるようにする。
4. The areka shall 種類の名前と文面を、`doc/ssp-mcp/areka-tools.md` の「⑶ 診断の種類と文面」の表と 1 字も違わないものにする。
5. The areka shall 1 つの台本の strict の記録を、台本の中の順（`check_script` の診断と同じ順）で残す。
6. The areka shall 記録する診断を、同じ台本を同じ時点で `check_script` に渡したときに返る診断と同じ集まり（種類・位置・綴り・文面が同じ）にする。今の areka では、無い surface（`missing_surface`）・無いバルーン（`missing_balloon`）・知らないタグ（`unknown_tag`。`\i[…]`・`\&[…]` を含む）・誰も拾わない `\!`（`unknown_command`）・読めない引数（`unreadable_argument`）・効き目の無いもの（`ignored`）・アンカーの対応の崩れ（`unpaired_tag`）の 7 種類がこれに当たる。
7. The areka shall 予約だけの種類（`missing_animation`・`unknown_entity`）を記録に出さない。
8. When 台本に誤りが 1 つも無いとき, the areka shall strict の記録を 1 件も残さない（`strict` 真の答えは要件 1.1 のまま）。

### Requirement 3: 記録する時と、しない時

**Objective:** As a 台本を流す AI, I want `strict` を付けた台本の分だけが、答えを受け取った時点で確実に記録されていてほしい, so that 答えの直後に `get_log` を呼んでも取りこぼさず、strict を付けていない台本で記録が増えることもない

#### Acceptance Criteria

1. When `sakurascript` が `strict` 真の台本を受け付けたとき, the areka shall その台本の strict の記録をすべて残し終えてから答えを返す。
2. When `raise_event` が `strict` 真で送ったイベントに、ゴーストが空白だけでない台本を返したとき, the areka shall 返った台本（答えの `Script: ` の行に載るもの）の strict の記録をすべて残し終えてから答えを返す。
3. If `raise_event` でゴーストが台本を返さなかった・空白だけの台本を返した・イベントを受け付けなかった・送れなかった, then the areka shall strict の記録を残さない。
4. While `strict` が偽か省略のとき, the areka shall strict の記録を 1 件も残さない。
5. The areka shall `strict` の値に関わらず、台本の再生のしかた・`script` の種類の記録・再生の側が今出している警告の記録を変えない。
6. The areka shall `raise_event` の答えの文言を `strict` の値で変えない（`OK:the ghost returned no script.`・`OK:the ghost returned a script and it is being played.` と `Script: ` の行は今のまま）。
7. The areka shall strict の記録を作る間も、ゴーストの表示（アニメーション・窓の操作）と他の MCP のツールの答えを、台本の長さに応じて止めない。

### Requirement 4: 入口で確かめる形の届く範囲（記録できること・できないこと）

**Objective:** As a 台本を流す AI, I want strict の記録が何を拾い何を拾わないかを前もって知りたい, so that 記録が 0 件のときに「誤りが無い」とどこまで言えるかを取り違えない

#### Acceptance Criteria

1. The areka shall strict の記録を、受け付けた時点の台本の文字列（`sakurascript` は渡された台本、`raise_event` はゴーストが返した台本）に対して作る。ゴーストの翻訳（`OnTranslate`）で書き換わった後の台本は確かめない。
2. The areka shall `\s`・`\b` の有無を、台本を受け付けた時点の宛先のゴーストのシェルとバルーンで確かめる（`check_script` の「答えは、呼んだ時点のシェルとバルーンに対するもの」と同じ）。
3. The areka shall `check_script` が診ないもの（`doc/ssp-mcp/areka-tools.md` の「⑷ 診ていないもの」）を strict の記録にも出さない。とくに、台本の中でシェル・バルーン・ゴーストを切り替えた後ろの `\s`・`\b`、名前の形の `\b[名前]`、窓を持たない話し手での `\s`・`\b`、`\![raise,…]` で呼んだ先の台本の中身は記録しない。
4. Where 宛先のゴーストが窓を持たない（表示の準備に失敗し、記録だけで動いている）とき, the areka shall 無い surface・無いバルーンの記録を出さず、ほかの種類だけを記録する。
5. The areka shall `\e`・`\-` の後ろの誤りも記録する（再生はそこで止まるので後ろは再生されないが、`check_script` と同じく書いてある誤りを知らせる。アンカーの対応の崩れだけは後ろに出さない）。
6. The areka shall 台本を受け付けた後に、再生が別の台本・利用者の操作・終了などで途中で打ち切られても、残した strict の記録を取り消さない。
7. The `doc/ssp-mcp/areka-tools.md` shall 1〜6 の範囲と、SSP の strict（再生中に実際に起きた誤りを記録する）との違いを、strict の記録の約束として書く。

### Requirement 5: 案内文の最後の 1 文を消す

**Objective:** As a MCP につなぐ AI, I want 接続の時に読む案内文に、もう当てはまらない約束が書かれていないでほしい, so that 「まだのツールがある」と取り違えて、ツールの答えを読み誤らない

#### Acceptance Criteria

1. The areka の MCP の受け口 shall `initialize` の `instructions`（と、同じ文を返す `server/discover` の `instructions`）から「A tool that is not implemented yet returns a result starting with "NG:not implemented yet".」の 1 文を消し、ほかの文（`check_script` の案内を含む）は 1 字も変えない。
2. When areka が居ないときに中継が組み込みの答えで `initialize` に答えるとき, the 中継 shall 1 と同じ案内文を返す。
3. The areka shall どのツールも `NG:not implemented yet` で始まる答えを返さない状態を保つ（本 spec の前と同じ）。

### Requirement 6: `params` がオブジェクトでない要求への答え

**Objective:** As a MCP につなぐ AI のクライアント, I want `params` の形を誤った要求に「引数の誤り」として JSON-RPC の答えが返ってほしい, so that 受け口へ直接でも中継ごしでも、同じ番号で誤りを見分けて送り方を直せる

#### Acceptance Criteria

1. When `params` がオブジェクトでない値（数・文字列・配列・真偽値）の要求（`id` の有るもの）が受け口へ直接届いたとき, the areka の MCP の受け口 shall その要求の `id` を付けた JSON-RPC のエラー（番号 `-32602`）で答える。メソッドが何であっても（知らない名前のメソッドでも）同じとする＝`params` の値そのものが誤っているので、メソッドの有無より先に答える。
2. The areka の MCP の受け口 shall 1 の答えの HTTP の状態とエラーの欄の組（`code`・`message`）を、同じ版の話し方で未登録のツール名の `tools/call` に答える `-32602` と同じ形にする（旧式の版の経路は 200、無状態版〔`2026-07-28`〕の経路は 400）。
3. When 1 と同じ要求が中継へ届いたとき, the 中継 shall 受け口へ直接送ったときと同じ答え（その要求の `id` を付けた `-32602`）を返す。
4. The 受け口と中継 shall 1 の答えを返した後に届く次の要求を、いつもどおり処理する。
5. The 受け口と中継 shall 1 に当たらないものへの答えを今のまま変えない: オブジェクトでない行・要求の形でない行への中継の `-32600`（`id` は `null`）、JSON でない本文、通知（`id` の無い要求）、`params` が `null` の要求、`params` の中の形の違い（`mcp-receiver-bugs` が決めた答え）。
6. The `doc/ssp-mcp/transport-diff-areka.md` shall `params` がオブジェクトでない要求の行を持ち、areka の答え（`-32602`・HTTP の状態）と、それを固定するテストの名前を書く（SSP の欄は測っていないと書く）。
7. The `doc/ssp-mcp/bridge-diff-areka.md` shall 中継の答えが 3 のとおりになったことに合わせ、該当の行の areka の欄を書き直す。

### Requirement 7: 決定論テストで固定すること

**Objective:** As a 開発者, I want 上の約束がすべて決定論テストに固定されていてほしい, so that 後の変更で記録や答えが崩れたら、実機を回さずに赤で気付ける

#### Acceptance Criteria

1. The areka の決定論テスト shall `doc/ssp-mcp/areka-tools.md` の「⑶」の表の 17 行それぞれについて、その行の「出る台本」を `strict` 真で流すとその行の種類と文面の strict の記録が残り（赤の側）、「出ない台本」では残らない（緑の側）ことを確かめる。確かめは表の行ごとに 1 つずつ持ち、表の行数と確かめの数が食い違えば赤になる。
2. The areka の決定論テスト shall 要件 2.6 の「`check_script` と同じ集まり」を、複数の誤りを含む台本で確かめる（種類・位置・綴り・文面・順）。
3. The areka の決定論テスト shall 要件 1.1・1.2・1.4 の答えの文言と `<n>` の値、要件 1.3 の `since_id` での読み出し、要件 3.1・3.2 の「答えの前に記録が済んでいる」ことを確かめる。
4. The areka の決定論テスト shall 要件 1.5・3.3・3.4 の「記録を残さない」場合それぞれで、エラーログに strict の記録が増えないことを確かめる。
5. The areka の決定論テスト shall 要件 4.3 の切替の後ろの `\s`・`\b`、要件 4.4 の窓を持たないゴースト、要件 4.5 の `\e` の後ろを、それぞれ 1 つ以上の台本で確かめる。
6. The 受け口と中継の決定論テスト shall 要件 5.1・5.2 の案内文と、要件 6.1〜6.4 の答えを、5 つの版の話し方（`2024-11-05`・`2025-03-26`・`2025-06-18`・`2025-11-25`・`2026-07-28`）で確かめる。
7. The 既存のテスト shall 本 spec が変える答え（要件 1.1・5.1・6.1・6.3）の期待のほかは、期待を変えずに緑のままとする。
