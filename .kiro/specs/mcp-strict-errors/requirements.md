# Requirements Document

## Project Description (Input)
AI（Claude Code など）が MCP の `sakurascript`・`raise_event` で流す台本には、今のシェルに無い surface 番号や、綴りを誤ったタグがよく混じる。SSP では `strict` を付けると、そうした箇所がエラーログに残る。AI は `get_log(log_type=error, since_id=…)` でそれを読み、自分の誤りを知って直せる。areka は `strict` を受け取って運ぶところまでは作ってあるが、その値を誰も使っていない。そのため台本は黙って一部だけ動き、AI には何が悪いのか分からない。

変えたいこと:
- `strict` 付きの台本を再生する途中で、実際に起きた誤りをエラーログに 1 件ずつ残す。種類と文面は `check_script`（台本を再生せずに確かめる areka 独自のツール）と同じにする。
- `sakurascript` の答えで「どの番号より後を読めばよいか」を返す。
- MCP の案内文に残っている「まだのツールは `NG:not implemented yet` を返す」の 1 文を消す（当てはまるツールはもう無い）。
- `params` がオブジェクトでない要求に、JSON-RPC のエラーで答える（brief.md の 2026-10-22 の申し送り）。

## Introduction

本 spec は、SSP の MCP を areka へ移す列の最後の 1 本である。SSP の `strict` は、台本の解釈に失敗した箇所をエラーログに残す働きで、AI が自分で書いた台本の誤りを再生した後に知るために使う。失敗とみなす箇所は次の 6 つ。

- 無いサーフェス
- 無いアニメーション
- 無いバルーン
- 知らないタグ
- 知らない `\![コマンド]`
- 知らない `\&[実体参照]`

v0.1.0 のゴールは「SSP 並みの自動の試験」である。そのためには、AI が台本を流したあとエラーログに何も増えていなければ「台本に誤りが無い」と判断できなければならない。

本 spec は SSP と同じく、**再生の途中で実際に起きた誤りを記録する**。台本に `strict` の印を持たせ、その印を翻訳（`OnTranslate`）の後の台本にも残したまま、再生の各所（台本の解釈・表情・バルーン・`\!` の受け取り手）まで運ぶ。各所は、誤りに当たったときに印を見て記録する。

台本の中でシェルやバルーンを切り替えた後ろの `\s`・`\b` も、切り替えた後の実際の答えで記録する。誤りの種類と文面は、既にある `check_script` と同じものを使う。どちらにも誤りの見つけ方の物差しを 1 つしか持たせないためである。

検出点は「再生の中で誤りを見つける所」のことで、ほかの機能の分岐の中に散らばる。漏れを防ぐため、全部を spec 単位の 1 つの表に持ち、表と実装が一致していることを検査で判定する（brief.md の Constraints）。

2026-10-23 の要件の討議で、開発者が次のように裁定した。
- 「入口で台本の文字列だけを確かめる形」は取らない。
- `params` の誤りへの答えは、公式 SDK（rmcp）の中で直す。

範囲が広がったので、本 spec はウェーブ C11 から外す。触るファイルが重なる並走の相手の着地を待ってから設計と実装に進む（順と待ちは brief.md と roadmap.md に書く）。

用語:

- **strict**: `sakurascript`・`raise_event` の引数 `strict`（真・偽・省略）。真のときだけ本 spec の記録をする。
- **strict の台本**: `strict` 真で受け付けた台本。`raise_event` では、ゴーストが返した台本がこれに当たる。翻訳で書き換わった後も strict の台本のままである。
- **診断**: `check_script` が返す 1 件（種類・位置・綴り・文面）。種類の名前と文面の正本は `doc/ssp-mcp/areka-tools.md` の「⑶ 診断の種類と文面」。
- **strict の記録**: 本 spec がエラーログ（`get_log` の `log_type=error`）へ残す 1 件。再生の途中で誤りに 1 つ当たるごとに 1 件。
- **検出点**: 再生の側で、誤りに当たったことが分かる所（例: 台本の解釈が知らないタグを捨てる所、表情の層が今のシェルに無い surface を断る所）。
- **検出点の表**: すべての検出点を 1 行ずつ並べた、spec 単位の表。
- **通し番号**: エラーログを含む全種類の記録に振られる番号（`get_log` の `#<番号>`・`since_id` で使うもの。`doc/ssp-mcp/log-convention.md` の 5 節・8 節）。

## Boundary Context

- **In scope**:
  - `sakurascript`・`raise_event` の strict の台本に印を付け、翻訳の後も含めて再生の各所まで運ぶこと。
  - 再生の各所で、誤りに当たったときに strict の記録を残すこと。
  - `sakurascript` の答えの文言（`since_id` 付き）。
  - 検出点の表と、表と実装の一致を判定する検査。
  - 記録の種類と文面を `doc/ssp-mcp/areka-tools.md` に揃えること。
  - その文書に strict の記録の約束（記録すること・しないこと・SSP との違い）を書き足すこと。
  - MCP の案内文（`initialize` の `instructions`、中継が areka の居ないときに返す同じ文）から、最後の「`NG:not implemented yet`」の 1 文を消すこと。
  - `params` がオブジェクトでない要求への答えを、受け口でも中継ごしでも JSON-RPC のエラーにすること。受け口の答えは rmcp の中で直す。
  - 輸送の差の一覧（`doc/ssp-mcp/transport-diff-areka.md`）と中継の差の一覧（`doc/ssp-mcp/bridge-diff-areka.md`）の該当の行。
  - SSP で測っていない答えを、設計の段で SSP の MCP を使って測ること（打つ前に毎回、開発者の許しを取る）。
  - 上のすべての決定論テストと、実機の確かめ 1 本。
- **Out of scope**:
  - SSTP の `Option: strict`。SSTP の口は予約のままにする。運ぶ印は同じものを将来 SSTP からも立てられる形にする。
  - 誤った箇所の再生のしかたを変えること。無い surface を出さない、知らないタグを捨てる、などの今の振る舞いは据え置く。
  - `check_script` の答え・判定・種類・文面を変えること（本 spec は同じものを使うだけ）。
  - `\i`・`\&` への対応。今は `unknown_tag` として記録される。対応したときに予約の種類 `missing_animation`・`unknown_entity` を使い始めるのは、対応する spec の仕事。
  - 通知（`id` の無い要求）と、`params` が `null` の要求への答え（今のまま）。
- **Adjacent expectations**:
  - 上流は着地済み。本 spec はそれらの答えの形と取り決めをそのまま使う。
    - `mcp-kanade-tools`: `strict` を受けて運ぶ口と、`sakurascript`／`raise_event` の答え。
    - `mcp-log-history`: エラーログと通し番号、記録の取り決め `doc/ssp-mcp/log-convention.md`。
    - `mcp-author-tools`: `check_script` と、種類と文面の正本。
    - `mcp-reload`: `NG:not implemented yet` を返す最後のツールに中身を入れた。
    - `mcp-receiver-bugs`: `params` の形の違いへの答えの前例。
    - `script-security-level`: 台本の始めの知らせに出どころの欄を足し、再生の側は運ぶだけという前例。
  - 再生の側が今出している警告の記録は今のまま残る。strict の記録は、それらとは別に足される。
  - 並走の順: 触るファイルが重なる spec の着地を待ってから、設計と実装に進む。
    - `\!` の受け取り手の台帳は `descript-window-keys`。
    - kanade の知らせの型は `mouse-click-wheel-events`。
    - 依存を差し替える席（`Cargo.lock`）は 1 ウェーブに 1 本。
    - 順と待ちの正本は brief.md の 2026-10-23 の節と roadmap.md。

## Requirements

### Requirement 1: `sakurascript` の strict の答え

**Objective:** As a 台本を流す AI, I want `strict` 付きで台本を流したら、どの番号より後のエラーログを読めばよいかを答えで知りたい, so that その台本で起きた誤りだけを `get_log` で取り出せる

#### Acceptance Criteria

1. When `sakurascript` が `strict` 真の台本を受け付けたとき, the areka shall 答えの本文を `OK:strict errors will be recorded to the error log. Check them later with get_log tool (log_type=error, since_id=<n>)` とし、`isError` を偽にする。この文は SSP の実測と同じで、`<n>` は 10 進の数である。
2. The areka shall 1 の `<n>` を、その台本の strict の記録を 1 件も残す前の時点で最後に振られた通し番号とする（1 件も無ければ 0）。
3. When 1 の答えを受け取った AI が、その台本の再生が終わった後に `get_log` を `log_type=error`・`since_id=<n>` で呼んだとき, the areka shall その台本の strict の記録をすべて返す。
   - その間に他の出どころが残した記録が混ざることはある。
   - エラーログの上限（種別ごとに 1,000 件）を超えた分は、古い方から消える。
4. While `strict` が偽か省略のとき, the areka shall `sakurascript` の答えを今のまま（受け付けたら `OK`）にする。
5. If `sakurascript` が次のどれかに当たったとき, then the areka shall `strict` の値に関わらず今と同じ `NG:` の答えを返し、strict の記録を残さない。
   - 台本が空。
   - ゴーストが今は台本を受け付けない。
   - 宛先のゴーストが居ない。
   - 答えの前にゴーストが降りた。

### Requirement 2: strict の記録の中身

**Objective:** As a 台本を流す AI, I want エラーログの 1 件から、何がどこで誤っていて再生でどうなったかを読み取りたい, so that 台本のどこをどう直せばよいかが分かる

#### Acceptance Criteria

1. When strict の台本の再生が誤りに 1 つ当たったとき, the areka shall その誤り 1 件につき 1 件を、`get_log` の `log_type=error` で読める記録として残す。
2. The areka shall 各記録の名（`get_log` の `<名>`）を、台本を再生しているゴーストの名前にする。これは `get_active_ghost_list` が返す値と同じもので、`get_log` の `ghost_name` で絞り込むと当たる。
3. The areka shall 各記録の本文から、次の 4 つを読み取れるようにする。
   - 種類の名前。
   - 台本の中の位置。再生した台本（翻訳の後）の先頭を 0 とした文字の数。翻訳で書き換わらなかった台本では、渡した台本の位置と同じになる。
   - 該当箇所の綴りそのまま。
   - 文面。
4. The areka shall 種類の名前と文面を、`doc/ssp-mcp/areka-tools.md` の「⑶ 診断の種類と文面」の表と 1 字も違わないものにする。
5. The areka shall 1 つの台本の strict の記録を、再生が誤りに当たった順で残す。
6. The areka shall 次の 7 種類を strict の記録の対象とする。
   - 無い surface（`missing_surface`）
   - 無いバルーン（`missing_balloon`）
   - 知らないタグ（`unknown_tag`。`\i[…]`・`\&[…]` を含む）
   - 誰も拾わない `\!`（`unknown_command`）
   - 読めない引数（`unreadable_argument`）
   - 効き目の無いもの（`ignored`）
   - アンカーの対応の崩れ（`unpaired_tag`）
7. When 台本が次の 4 つをすべて満たすとき, the areka shall その再生で残る strict の記録を、同じ台本を受け付けた時点で `check_script` に渡したときに返る診断と同じ集まりにする。同じとは、種類・位置・綴り・文面・順が同じことをいう。
   - 中でシェル・バルーン・ゴーストを切り替えない。
   - `\e`・`\-` の後ろに誤りを持たない。
   - 翻訳で書き換わらない。
   - 最後まで再生される。
8. The areka shall 予約だけの種類（`missing_animation`・`unknown_entity`）を記録に出さない。
9. When strict の台本の再生が誤りに 1 つも当たらなかったとき, the areka shall strict の記録を 1 件も残さない。このとき `strict` 真の答えは要件 1.1 のままである。

### Requirement 3: 記録する時と、しない時

**Objective:** As a 台本を流す AI, I want `strict` を付けた台本の、実際に再生された所で起きた誤りだけが記録されてほしい, so that 記録を SSP の strict と同じ意味で読め、strict を付けていない台本で記録が増えることもない

#### Acceptance Criteria

1. When `sakurascript` が `strict` 真の台本を受け付けたとき, the areka shall その台本を strict の台本として再生し、再生が誤りに当たった時点で strict の記録を残す。
2. When `raise_event` が `strict` 真で送ったイベントに、ゴーストが空白だけでない台本を返したとき, the areka shall 返った台本を strict の台本として再生し、1 と同じく記録する。
3. While strict の台本がゴーストの翻訳（`OnTranslate`）を通ったとき, the areka shall 翻訳の後の台本を strict の台本のまま再生する。
4. If `raise_event` が次のどれかに当たったとき, then the areka shall strict の記録を残さない。
   - ゴーストが台本を返さなかった。
   - 空白だけの台本を返した。
   - イベントを受け付けなかった。
   - 送れなかった。
5. While `strict` が偽か省略のとき, the areka shall strict の記録を 1 件も残さない。
6. The areka shall 再生されなかった所の誤りを記録しない。
   - `\e`・`\-` の後ろ。
   - 別の台本・利用者の操作・終了などで途中で打ち切られた後ろ。
7. The areka shall 打ち切られる前に残した strict の記録を取り消さない。
8. The areka shall strict の台本の中から呼ばれて始まった別の台本（`\![raise,…]` に返った台本など）に記録が及ぶかを、SSP の実測（要件 6.8）に合わせる。測れなかった場合は及ばない（strict で受け付けたその台本だけ）とする。
9. The areka shall `strict` の値に関わらず、次のものを変えない。
   - 台本の再生のしかた。
   - `script` の種類の記録。
   - 再生の側が今出している警告の記録。
10. The areka shall `raise_event` の答えの文言を、要件 6.8 の SSP の実測と同じにする。今の文（`OK:the ghost returned no script.`・`OK:the ghost returned a script and it is being played.` と `Script: ` の行）は、実測が違う形を示したときだけ変える。
11. The areka shall strict の印を運ぶことで、ゴーストの表示（アニメーション・窓の操作）と他の MCP のツールの答えを、台本の長さに応じて止めない。

### Requirement 4: 再生中の記録の届く範囲と、検出点の表

**Objective:** As a 台本を流す AI と開発者, I want strict の記録が再生のどこで何を拾うかが 1 つの表で決まっていて、表と実装が食い違わないでほしい, so that 記録が 0 件のときに「誤りが無い」と言える範囲を取り違えず、後の変更で検出点が黙って抜けても気付ける

#### Acceptance Criteria

1. The areka shall 台本の中でシェルやバルーンを切り替えた後ろの `\s`・`\b` を、切り替えた後のシェルやバルーンで確かめて記録する。
2. The areka shall `\b[…]` の名前の形や、表情の層が後から解決する指定も、再生が解決できなかった時点で記録する。
3. Where 宛先のゴーストが窓を持たない（表示の準備に失敗し、記録だけで動いている）とき, the areka shall 無い surface・無いバルーンの記録を出さず、ほかの種類だけを記録する。
4. The 本 spec の文書 shall 再生の側のすべての検出点を、1 行ずつ「種類・どの層のどの分岐か・記録する条件」で並べた検出点の表として持つ。
5. The areka の決定論テスト shall 検出点の表と実装の一致を判定する。
   - 表に在って記録しない検出点が在れば赤にする。
   - 実装に在って表に無い検出点が在れば赤にする。
   - 表の行数と確かめの数が食い違えば赤にする。
6. The `doc/ssp-mcp/areka-tools.md` shall strict の記録の約束を書く。中身は次の 4 つ。
   - 1〜3 の範囲。
   - 要件 3 の記録しない場合。
   - SSP の strict との違い（要件 6.8 の実測で分かった分）。
   - 同じ誤りが再生の側の記録（名は `[SYSTEM]`）としてもエラーログに出ることがあり、strict の記録だけを読むには `get_log` の `ghost_name` で絞ればよいこと。
7. The `doc/ssp-mcp/areka-tools.md` shall `ignored` を「areka がまだ対応していないもので、台本の誤りではない」と書く。そのうえで、`ignored` を除いて読みたい AI は種類の名前で外せることを書く。

### Requirement 5: 案内文の最後の 1 文を消す

**Objective:** As a MCP につなぐ AI, I want 接続の時に読む案内文に、もう当てはまらない約束が書かれていないでほしい, so that 「まだのツールがある」と取り違えて、ツールの答えを読み誤らない

#### Acceptance Criteria

1. The areka の MCP の受け口 shall `initialize` の `instructions` と、同じ文を返す `server/discover` の `instructions` から、「A tool that is not implemented yet returns a result starting with "NG:not implemented yet".」の 1 文を消す。ほかの文（`check_script` の案内を含む）は 1 字も変えない。
2. When areka が居ないときに中継が組み込みの答えで `initialize` に答えるとき, the 中継 shall 1 と同じ案内文を返す。
3. The areka shall どのツールも `NG:not implemented yet` で始まる答えを返さない状態を保つ（本 spec の前と同じ）。

### Requirement 6: `params` がオブジェクトでない要求への答えと、SSP の実測

**Objective:** As a MCP につなぐ AI のクライアント, I want `params` の形を誤った要求に JSON-RPC の正典どおりのエラーが返ってほしい, so that 受け口へ直接でも中継ごしでも、同じ番号で誤りを見分けて送り方を直せる

#### Acceptance Criteria

1. When `params` の値が数・文字列・真偽値の要求（`id` の有るもの）が受け口へ直接届いたとき, the areka の MCP の受け口 shall その要求の `id` を付けた JSON-RPC のエラーで答える。
   - 番号は `-32600` とする。JSON-RPC 2.0 は `params` を「在るなら配列かオブジェクト」と定めているので、これは要求の形の誤りに当たる。
   - メソッドが何であっても（知らない名前のメソッドでも）同じとする。
2. When `params` が配列の要求（`id` の有るもの）が受け口へ直接届いたとき, the areka の MCP の受け口 shall その要求の `id` を付けた JSON-RPC のエラーで答える。
   - 番号は `-32602` とする。JSON-RPC としては正しいが、MCP は `params` をオブジェクトと定めているので、引数の誤りに当たる。
   - メソッドが何であっても同じとする。
3. The areka の MCP の受け口 shall 1・2 の答えを、公式 SDK（rmcp）が返す形で返す。
   - rmcp の手前で本文を読んで、areka が自分で答えることはしない。
   - 今使っている rmcp で返せないなら、次の順で直す。
     - ⑴ 新しい版で直っていれば、その版へ上げる。
     - ⑵ 直っていなければ、rmcp の上流へ直しを出す。取り込まれるまでは、同じ直しを依存の差し替え（`[patch.crates-io]`）で持つ。
   - 上流へ直しを出すのは外へ出す操作なので、出す前に開発者の許しを取る。
4. The areka の MCP の受け口 shall 1・2 の答えの HTTP の状態を、同じ版の話し方で、rmcp がその番号のエラーに付ける状態と同じにする。
5. When 1・2 と同じ要求が中継へ届いたとき, the 中継 shall 受け口へ直接送ったときと同じ答え（その要求の `id` を付けた同じ番号）を返す。
6. The 受け口と中継 shall 1・2 の答えを返した後に届く次の要求を、いつもどおり処理する。
7. The 受け口と中継 shall 1・2 に当たらないものへの答えを今のまま変えない。
   - オブジェクトでない行・要求の形でない行への中継の `-32600`（`id` は `null`）。
   - JSON でない本文。
   - 通知（`id` の無い要求）。
   - `params` が `null` の要求。
   - `params` の中の形の違い（`mcp-receiver-bugs` が決めた答え）。
8. The 設計の段 shall 次の答えを SSP の MCP で測り、`doc/ssp-mcp/survey.md` または各差の一覧に書く。打つ前に毎回、開発者の許しを取る。
   - `raise_event` に `strict` 真で送り、台本が返ったときの答えの文。
   - strict の記録の表示の語と本文の形。
   - strict の台本の中から呼んだ台本に記録が及ぶか。
   - `params` がオブジェクトでない要求への答え。
   - 測れなかったものは「測っていない」と書く。
9. The `doc/ssp-mcp/transport-diff-areka.md` shall `params` がオブジェクトでない要求の行を持つ。その行には、areka の答え（番号・HTTP の状態）、それを固定するテストの名前、SSP の欄（8 の実測か「測っていない」）を書く。
10. The `doc/ssp-mcp/bridge-diff-areka.md` shall 中継の答えが 5 のとおりになったことに合わせ、該当の行の areka の欄を書き直す。

### Requirement 7: テストで固定すること

**Objective:** As a 開発者, I want 上の約束がすべてテストに固定されていてほしい, so that 後の変更で記録や答えが崩れたら、実機を回さずに赤で気付ける

#### Acceptance Criteria

1. The areka の決定論テスト shall `doc/ssp-mcp/areka-tools.md` の「⑶」の表の 17 行それぞれについて、次の 2 つを確かめる。
   - 赤の側: その行の「出る台本」を `strict` 真で再生すると、その行の種類と文面の strict の記録が残る。
   - 緑の側: 「出ない台本」では残らない。
   - 確かめは表の行ごとに 1 つずつ持ち、表の行数と確かめの数が食い違えば赤になる。
   - 種類の名前と文面は、テストが文書の表そのものを読んで比べる。要件 2.4 の「1 字も違わない」は、文面の定数の写しでなく文書と突き合わせて判定する。
2. The areka の決定論テスト shall 要件 2.7 の「`check_script` と同じ集まり」を、複数の誤りを含む台本で確かめる（種類・位置・綴り・文面・順）。
3. The areka の決定論テスト shall 次のことを、それぞれ 1 つ以上の台本で確かめる。
   - 要件 4.1: 切替の後ろの `\s`・`\b`。
   - 要件 4.2: 名前の形の `\b[…]`。
   - 要件 4.3: 窓を持たないゴースト。
   - 要件 3.3: 翻訳の後の台本。
   - 要件 3.6: `\e` の後ろと、打ち切られた後ろ。
4. The areka の決定論テスト shall 要件 1.1・1.2・1.4 の答えの文言と `<n>` の値、要件 1.3 の `since_id` での読み出しを確かめる。
5. The areka の決定論テスト shall 要件 1.5・3.4・3.5 の「記録を残さない」場合それぞれで、エラーログに strict の記録が増えないことを確かめる。
6. The areka の決定論テスト shall 要件 4.5 の検出点の表と実装の一致を判定する。
7. The 受け口と中継の決定論テスト shall 要件 5.1・5.2 の案内文と、要件 6.1〜6.6 の答えを、5 つの版の話し方（`2024-11-05`・`2025-03-26`・`2025-06-18`・`2025-11-25`・`2026-07-28`）で確かめる。
8. The 既存のテスト shall 本 spec が変える答え（要件 1.1・5.1・6.1・6.2・6.5）の期待のほかは、期待を変えずに緑のままとする。
9. The 実機の確かめ shall 実走の道具（`real-run-mcp-kit` の窓口）で 1 本行う。中身は、本物のゴーストに `strict` 真で誤りを含む台本を流し、答えの `since_id` で `get_log(log_type=error)` を読むと、その台本の strict の記録が読めることである。
