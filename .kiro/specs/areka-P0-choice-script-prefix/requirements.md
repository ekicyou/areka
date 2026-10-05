# Requirements Document

## Project Description (Input)

**誰の問題か**: 選択肢からその場で台本を走らせたいゴースト作者。正典では `\q[タイトル,script:さくらスクリプト]` と書けば、SHIORI を通さずに、選んだ瞬間に台本が動く（ukadoc `\q[タイトル,script:実行内容]`「選択後、script:以下の内容をさくらスクリプトとして実行する」・記述例 `\q[バルーンを閉じる,script:\e]`・入れ子 `\q[その１,"script:\q[その２,script:その３はない]"]`）。「メモ帳を開く」「公式サイトを開く」のような選択肢も、`script:` と開く系のタグの組み合わせで書くのが正典の作法である。

**現状**: areka では `script:` の選択肢を選んでも**何も実行されない**。選択肢は正典どおりに出て選べ、選んだときに選択肢の待ちが解けて会話も止まらないが、`script:` の後ろの台本だけが走らない。記録は「未対応の選択肢」の警告 1 行だけで、ゴースト作者は動かない理由に気付きにくい。

**何が変わるべきか**: `\q[…,script:…]` を選ぶと、`script:` の後ろが新しいトークとして実行される。`\e` で閉じる・`\q` の入れ子・開く系のタグ（入っていれば）が正典どおりに動く。実行した台本は、選択肢を含んでいた元の台本と同じ出どころとして扱う。実行したことを記録に残し、空の `script:` も記録を出して閉じる（黙って捨てない）。

## Introduction

本仕様は、選択肢の ID が `script:` で始まるときに、選んだ後の動き（`script:` の後ろの台本を新しいトークとして再生する）を正典どおりに埋める。選択肢の表示・選び方・選択肢の待ち・タイムアウト・通常の選択肢のイベント（`OnChoiceSelectEx`・`OnChoiceSelect`・`On` で始まる ID のイベント）は完了済みの仕様が持っており、本仕様はそれらを変えない。今は「未対応」として待ちを閉じるだけになっている 1 つの分かれ道を、「台本を実行する」に替えるのが本仕様の中身である。

### 要件の段で確かめた現状（2026-10-05）

- **`"…"` 括りの読み方は正典どおり**: さくらスクリプトの読み込みは、角括弧の引数の中で `"` から次の `"` までを 1 つの引数として取り込み、その間の `,` と `]` を区切りとして扱わない（`""` は `"` 1 文字）。したがって `\q[その１,"script:\q[その２,script:その３はない]"]` の第 2 引数は `script:\q[その２,script:その３はない]` になり、選んで再生した台本の中の `\q[その２,script:その３はない]` の第 2 引数は `script:その３はない` になる。読み方の変更は要らない。
- **括らずに書いた `,` は区切りになる**: `\q[メモ帳を開く,script:\![open,file,notepad.exe]]` のように括らずに書くと、`,` で引数が分かれ、最初の `]` で選択肢が閉じる（第 2 引数は `script:\![open`）。正典の入れ子の記述例が括りを使っているとおり、`,` を含む台本は `\q[メモ帳を開く,"script:\![open,file,notepad.exe]"]` と括って書くのが正しい書き方である。
- **網羅台帳の行**: `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\q[タイトル,script:実行内容]` の行は、状態が `degraded`（縮退）・持ち主は空・優先度 `A1`。注記は「選択肢は出て選べるが、`script:` に書いたさくらスクリプトだけが走らない」「未対応の選択肢の警告が出る」と書かれている。同じ縮退は `doc/choice-cascade-compat.md` の 7a の 2 行（`script:` 前置形の存在・M1 の縮退）と `doc/COMPAT_ARCHITECTURE.md` の選択肢の行にも記されている。

## Boundary Context

- **In scope**:
  - `\q[タイトル,script:台本]` を選んだときに、`script:` の後ろの台本を新しいトークとして再生すること。
  - 再生する台本の出どころの扱い（元の台本と同じ）。
  - 実行したこと・空の `script:`・使われない余分な引数の記録。
  - 正典の記述例 2 つ（閉じる・入れ子）と境目の場合の、実機なしで繰り返し同じ結果になるテスト。
  - 網羅台帳の行と、選択肢の正典の沈黙・縮退を記した文書の更新。
- **Out of scope**:
  - `\__q[script:…]…\__q`（範囲を選択肢にするタグ。同じ道を使うのは `range-choice-tag`）。
  - 開く系のタグそのもの（`\![open,…]`・`\j[…]` などの実行は `open-external-tags`）。本仕様は、台本に含まれていれば SHIORI の応答の台本と同じに流すだけ。
  - おすすめサイトなどの `script:`（SHIORI リソースのメニューの項目の側の話）。
  - 台本の出どころの印を運ぶ仕組み（`script-security-level`）と、影響の段・同意の窓（`script-impact-tiers`）。
  - 通常の選択肢の流れ（`OnChoiceSelectEx` → `OnChoiceSelect`・`On` で始まる ID のイベント・タイムアウト）。完了済み `choice-select-events` の決めごとのまま変えない。
  - 旧仕様の `\q[ID][タイトル]`（areka はこの形を選択肢にしないので、`script:` も起こらない）。
  - さくらスクリプトの引数の読み方の変更（上で確かめたとおり正典どおりなので変えない）。
- **Adjacent expectations**:
  - **上流（完了済み `choice-select-events`・`choice-interact`・`choice-timeout-directive`）**: 選択肢の ID を書いたとおりの文字列で届け、選択肢の待ち・タイムアウト・選ばれた選択肢の受け付け（候補に無い ID や遅れたクリックを捨てる）を今までどおり行う。本仕様はそこを変えず、受け付けた後の `script:` の分かれ道だけを埋める。
  - **下流（`range-choice-tag`）**: `\__q` の `script:` も本仕様と同じ動きになる（同じ道を使う）。
  - **下流（`mcp-kanade-tools`）**: MCP の `sakurascript` が「台本を 1 つ渡してトークとして走らせる」ときに、本仕様が作る台本の走らせ方を使い回せるようにする（形は設計の段で決める）。
  - **下流（`link-context-copy`）**: 右クリックのコピーで `script:` の中の行き先を読む。本仕様は選択肢の ID を書き替えないので、ID はそのまま読める。
  - **下流（`script-security-level`）**: 台本に出どころの印が付くようになったら、本仕様の新しいトークへ元のトークの出どころを引き継ぐ。本仕様はそのために、`script:` の台本を他のトークと同じ 1 つの入口から始める。
  - **同じ時期に並行する作業**: さくらスクリプトの読み込み（`open-external-tags` が触る）と、選択肢の流れの共通の部分（`balloon-lifecycle-events`・`mcp-get-status` が触る）には手を入れない。どこに置くかの約束は設計の段で守る。

## Requirements

### Requirement 1: `script:` の選択肢を選ぶと台本を実行する

**Objective:** As a ゴースト作者, I want `\q[タイトル,script:台本]` を選んだら `script:` の後ろの台本がその場で動くこと, so that SHIORI を通さずに選択肢から台本を走らせる正典の書き方がそのまま使える

#### Acceptance Criteria

1. When 利用者が、第 2 引数が `script:` で始まる選択肢を選んだ, the areka shall `script:` の後ろの文字列を、新しいトークとして再生する。
2. When 利用者が `script:` の選択肢を選んだ, the areka shall SHIORI のイベント（`OnChoiceSelectEx`・`OnChoiceSelect`・`On` で始まる名前のイベント・`OnTranslate`）を 1 つも起こさない。
3. When `script:` の台本で新しいトークを始める, the areka shall 選択肢の待ち（タイムアウトの計測を含む）を、新しいトークを始めるのと同じときに閉じる。
4. When `script:` の台本で新しいトークを始める, the areka shall 選択肢を含んでいたトークを、選択肢の SHIORI イベントが台本を返して次のトークが始まるときと同じく、新しいトークへ置き換える（元のトークの残りは再生しない）。
5. The areka shall `script:` の台本を、SHIORI の応答の台本と同じ読み方・同じ再生の結果で扱う（崩れた書き方の扱い、選択肢・タイムアウトの指定・`\e` などのタグの効き方・再生時の環境変数（`%…`）の展開が、応答の台本と変わらない）。
6. The areka shall `script:` の判定を、第 2 引数がちょうど小文字の `script:` で始まるときだけに限る（`script` だけ・`Script:`・`xscript:` などは今までどおり通常の選択肢として扱う）。
7. The areka shall `script:` の台本を、翻訳（`OnTranslate`・MAKOTO）に通さずに再生する（`script:` の台本は、選択肢を含んでいた元の台本が翻訳を通るときに、その一部として 1 回通っている。選んだ後にもう一度通すと、台本全体を置き換える翻訳が二重に掛かる）。

### Requirement 2: 正典の記述例が正典どおりに動く

**Objective:** As a ゴースト作者, I want ukadoc の記述例をそのまま書いて正典どおりに動くこと, so that 他のベースウェア向けに書いた選択肢が areka でも同じに動く

#### Acceptance Criteria

1. When 利用者が `\q[バルーンを閉じる,script:\e]` を選んだ, the areka shall `\e` を新しいトークとして再生し、トークを終えてバルーンを閉じる。
2. When 台本に `\q[その１,"script:\q[その２,script:その３はない]"]` が含まれる, the areka shall 選択肢「その１」の第 2 引数を、括りの中の `,` と `]` で分けずに `script:\q[その２,script:その３はない]` の 1 つとして受け取る。
3. When 利用者が上の選択肢「その１」を選んだ, the areka shall `\q[その２,script:その３はない]` を新しいトークとして再生し、選択肢「その２」を出して選択肢の待ちに入る。
4. When 利用者が続けて選択肢「その２」を選んだ, the areka shall 「その３はない」を新しいトークとして表示する。
5. Where `script:` の台本に開く系のタグ（例: `\q[メモ帳を開く,"script:\![open,file,notepad.exe]"]`）が含まれる, the areka shall そのタグを、SHIORI の応答の台本に含まれていたときと同じに扱う（開く動きの有無と中身は開く系のタグの対応に従う）。

### Requirement 3: 空の `script:` と使われない引数

**Objective:** As a ゴースト作者, I want 書き損じた `script:` の選択肢でも会話が止まらず、理由が記録に残ること, so that 動かない選択肢の原因に記録から気付ける

#### Acceptance Criteria

1. If 利用者が選んだ選択肢の第 2 引数が `script:` だけで、その後ろに 1 文字も無い, then the areka shall 新しいトークを始めず、空だったことを警告として記録し、選択肢の待ちを閉じる。
2. If 空の `script:` で選択肢の待ちを閉じた, then the areka shall 選択肢を含んでいたトークを、選択肢の SHIORI イベントが何も返さなかったときと同じに続ける（会話を止めない）。
3. If `script:` の選択肢に第 3 引数以降がある, then the areka shall それらを台本に含めず、どこへも渡さずに `script:` の台本を実行し、使わなかった引数があったこととその数を記録に残す（`,` を含む台本の括り忘れに気付けるようにする）。

### Requirement 4: 実行する台本の出どころ

**Objective:** As a ゴーストの利用者, I want 選択肢から走る台本が、その選択肢を出した台本と同じ扱いを受けること, so that 選択肢を経由するだけで台本の扱いが変わらない

#### Acceptance Criteria

1. The areka shall `script:` の台本を、選択肢を含んでいたトークと同じ出どころの台本として扱い、`script:` の台本だけに制限を足したり緩めたりしない。
2. While 台本に出どころの区別が無い, the areka shall `script:` の台本を、ゴーストの SHIORI の応答の台本と同じ扱いで再生する。

### Requirement 5: 実行の記録

**Objective:** As a ゴースト作者・開発者, I want `script:` の台本を走らせたことが記録に残ること, so that 選択肢から何が走ったかを後から確かめられる

#### Acceptance Criteria

1. When `script:` の台本で新しいトークを始めた, the areka shall 選択肢の ID・新しいトークの番号・選択肢を含んでいたトークの番号を、選択肢を受け付けた記録と同じ段の記録に 1 件残す。
2. When 利用者が `script:` の選択肢を選んだ, the areka shall 今までの「未対応の選択肢」の警告を出さない。
3. The areka shall `script:` の選択肢を選んだときの結末（台本を始めた・空で閉じた）のどちらでも、記録を出さずに終わる道を持たない。

### Requirement 6: 網羅台帳と互換の記録の更新

**Objective:** As a areka の開発者, I want 正典の対応状況の記録が実際の動きと一致すること, so that 台帳から `script:` の選択肢が使えると分かる

#### Acceptance Criteria

1. When 本仕様の実装が終わった, the 網羅台帳（`doc/ukadoc-coverage/ledger/sakura-script.toml`）の `\q[タイトル,script:実行内容]` の行 shall 状態を縮退から実装済みに改め、注記を今の動き（選ぶと台本を新しいトークとして再生する・空の `script:` と余分な引数の扱い・記録の出方）に書き替える。
2. When 本仕様の実装が終わった, the 選択肢の正典の沈黙・縮退の記録（`doc/choice-cascade-compat.md` の 7a の行と、それを指す `doc/COMPAT_ARCHITECTURE.md` の選択肢の行） shall `script:` が縮退ではなくなったことを記し、正典が黙っている所の areka の決めごと（SHIORI のイベントを起こさない・翻訳（`OnTranslate`・MAKOTO）に通さない・第 3 引数以降を使わない・空なら待ちを閉じるだけ）を出どころ付きで記録する。翻訳に通さない決めごとには、台本全体を置き換える翻訳の二重掛けを避ける理由と、タグの中を避けて翻訳するゴーストでは `script:` の台本が翻訳されないまま再生されることを併記する。
3. The 網羅台帳の検査 shall 本仕様の更新の後も通る。

### Requirement 7: 実機なしで確かめるテスト

**Objective:** As a areka の開発者, I want `script:` の選択肢の動きを実機なしで繰り返し確かめられること, so that 後の変更で正典の動きが崩れたらすぐ分かる

#### Acceptance Criteria

1. The areka shall 正典の記述例 `\q[バルーンを閉じる,script:\e]` を選んだときに、SHIORI のイベントを起こさず、選択肢の待ちを閉じて `\e` の新しいトークを始めることを、実機なしで毎回同じ結果になるテストで確かめる。
2. The areka shall 入れ子の記述例 `\q[その１,"script:\q[その２,script:その３はない]"]` について、第 2 引数の読み方と、「その１」→「その２」と選んだときに順に始まるトークの台本を、実機なしで毎回同じ結果になるテストで確かめる。
3. The areka shall 空の `script:`・第 3 引数以降がある `script:`・`script:` に当たらない境目の ID（`script` だけ・`Script:`・`xscript:`）の扱いと、それぞれの記録を、実機なしで毎回同じ結果になるテストで確かめる。
4. The areka shall 今の「未対応の選択肢」を前提にしたテストを、本仕様の動きに合わせて書き替え、古い前提のテストを残さない。
