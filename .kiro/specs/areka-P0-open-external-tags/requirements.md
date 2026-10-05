# Requirements Document

## Project Description (Input)
バルーンから URL・ファイル・フォルダ・メールを開かせたいゴースト作者と、それを押す利用者のために、正典（ukadoc）の開く系のタグ `\j[ID]`（`http://～`・`file:///～`・`mailto:～`）、`\![open,file,…]`、`\![open,browser,…]`、`\![open,explorer,…]`、`\![open,editor,…]`、`\![open,mailer,…]` を、OS の既定のアプリ（関連付け）で開けるようにする。今の areka ではこれらは黙って無視され、バルーンから開けるのは `\![open,readme]` だけ。開く処理は 1 か所に集めて毎回記録を残し、開けなかったときは理由を記録する。台本から開く系の行き先を取り出す純粋な関数も持ち、後続の `link-context-copy`・`balloon-link-hover` が使う。外部アプリの設定画面・同意の窓（`script-impact-tiers`）・SSP 自身の窓を開く形・ネットワーク系は範囲外。

## Introduction

さくらスクリプトには「外のものを開く」タグがそろっている。`\j[ID]` は URL・ファイル・メールの宛先を開き、`\![open,file,…]`・`\![open,browser,…]`・`\![open,explorer,…]`・`\![open,editor,…]`・`\![open,mailer,…]` はファイル・ブラウザ・ファイル管理の窓・エディタ・メールソフトを開く（ukadoc さくらスクリプトリスト）。SSP ではこれらの一部が「本体設定 → 外部アプリ」で選んだアプリを使うが、areka はその設定を写さず、**OS の既定のアプリ（ファイルの関連付け・URL の既定のアプリ）で開く**（開発者裁定 2026-10-05「SSP は OS 既定のアプリが無い時代から続いてるアプリなので、いまは OS の受け口を最大限活用すべき」）。設定画面は作らない。

今の areka では、`\j` は読まれずに捨てられ、`\![open,○○]` は汎用の `\!` の運び手に載って最後まで届くものの受け取り手が `readme` しかなく、どれも**黙って**無視される（網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の 6 行はいずれも `absent`）。本 spec はこの 6 つの形を動かし、開く処理を 1 か所に集めて開くたびに記録を残し、開けなかったときは理由を記録する。あわせて「台本から開く系の行き先を取り出す」純粋な関数を持ち、後続の `link-context-copy`（右クリックで行き先をコピー）と `balloon-link-hover`（ホバーで行き先を出す）がそれを使う。

本書で使う語:

- **行き先**: タグが開こうとしているもの。URL・ファイルのパス・フォルダのパス・メールの宛先のいずれか。
- **出どころ**: その台本を出したゴースト（ゴースト名）。台本がどこから来たか（SHIORI の応答・選択肢の `script:`・MCP のツール など）は本 spec では記録しない。
- **開く処理**: 行き先を OS に渡して既定のアプリを起こす、areka の中の 1 か所。
- **`ghost/master`**: 実行中のゴーストのフォルダの中の `ghost/master`。ukadoc が相対パスの基準として定める場所。

## Boundary Context

- **In scope**:
  - `\j[ID]` の 3 つの形（`http://`／`https://`・`file:///`・`mailto:`）を読んで開く。
  - `\![open,file,ファイル名]`・`\![open,browser,パラメータ]`・`\![open,explorer,ファイル]`（`\![open,explorer,種類,名前]` の形を含む）・`\![open,editor,ファイル,表示行]`・`\![open,mailer,パラメータ]` を開く。
  - 開く処理を 1 か所に集め、開くたびに記録を残し、失敗の理由を記録する。
  - 台本から開く系の行き先を順に取り出す純粋な関数。
  - OS を偽の境界で差し替えた決定論テスト・網羅台帳の 6 行の更新・受け取り手の表の既存テストの書き替え。
- **Out of scope**:
  - 外部アプリの設定画面・設定ファイル（開発者裁定＝作らない。OS の既定のアプリだけを使う）。
  - 同意の窓・影響の段による制限（`script-impact-tiers`）。本 spec は開く処理の 1 か所を用意するところまでで、同意を求めない。
  - `\![open,help]`・`\![open,configurationdialog]`・`\![open,ghostexplorer]` など SSP 自身の窓を開く形（今までどおり受け取り手なし）。
  - `\![execute,http-get,…]` などネットワーク系。
  - `\![open,readme]` のふるまい（実装済み・何を開くかは変えない）。ただし OS へ渡す通り道は要件 7.1 の 1 か所へ寄せ、要件 7.5 のとおり UI を止めない形に移す。
  - 選択肢の `script:` の実行（`choice-script-prefix`）・右クリックのコピー（`link-context-copy`）・ホバー（`balloon-link-hover`）・`\_a` の働き（`anchor-tag-canon`）。
- **Adjacent expectations**:
  - 台本がどこから来ても（SHIORI の応答・`choice-script-prefix` が着地した後の選択肢の `script:`・`mcp-kanade-tools` が着地した後の MCP の `sakurascript`）、開く系のタグは同じ開く処理の 1 か所を通る。本 spec は出どころを増やさず、どの出どころでも同じふるまいになることだけを約束する。
  - `script-impact-tiers` は、本 spec が用意する開く処理の 1 か所に同意の窓を差し込む。本 spec は差し込む場所が 1 か所であることを保証し、同意の窓そのものは持たない。
  - `link-context-copy`・`balloon-link-hover` は、本 spec の「行き先を取り出す関数」をそのまま使う（取り出し方の規則は本 spec が持ち、2 本は規則を持たない）。
  - 同じウェーブ C4 の約束: `main.rs`・`emo2_boot/mod.rs` に触らない（受け取り手の結線は `emo2_boot/mod.rs` にあるので、新しい受け取り手を足すのではなく、既に結線されている `\![open,readme]` の受け取り手を `open` の他の第 1 引数まで受け持つ形に広げるのが前提。`mod.rs` を触らないと済まないと分かったら止めて報告する）。開く処理の新しいファイルは `readme.rs` の子に置く。dola の `CueCommand` に新しい種類を足さない（`\j` は汎用の `\!` の運び手へ写す。先例は裸の `\+` → `\![change,ghost,random]` の転記）。`decode.rs`・`compile.rs` は次に `anchor-tag-canon` が触るので、本 spec の腕は既存の腕の並びに 1 本足す形に留める。足さないと済まないと分かったら止めて報告する。

## Requirements

### Requirement 1: `\j[ID]` を読んで開く

**Objective:** As a ゴースト作者, I want `\j[http://～]`・`\j[file:///～]`・`\j[mailto:～]` が ukadoc どおりに開く, so that SHIORI の応答に書くだけで URL・ファイル・メールを利用者に開かせられる

#### Acceptance Criteria

1. When 台本に `\j[ID]` があり ID が `http://` または `https://` で始まる, the areka shall その ID を URL として OS の既定のアプリで開く。
2. When 台本に `\j[ID]` があり ID が `file:///` で始まり、続きが絶対パス, the areka shall そのパスのファイルを OS の関連付けで開く。
3. When 台本に `\j[ID]` があり ID が `file:///` で始まり、続きが相対パス, the areka shall `ghost/master` を基準にパスを解決してから OS の関連付けで開く（ukadoc の例 `\j[file:///descript.txt]` は `ghost/master/descript.txt` を開く）。
4. When 台本に `\j[ID]` があり ID が `mailto:` で始まる, the areka shall その宛先の新しいメールを OS の既定のメールソフトで開く。
5. The areka shall `\j[ID]` を読んだ結果を、`\![open,…]` と同じ受け取り手が受け取れる形にする（dola の `CueCommand` に新しい種類は 0 個）。
6. While 台本の再生中に `\j[ID]` に達した, the areka shall 台本の文字の表示や待ち時間を変えない（`\j` は表示に何も出さず、時間も消費しない）。
7. If 台本に `\j[ID]` があり ID が上の 3 つの形のどれでもない（空を含む）, then the areka shall 何も開かず、`warn!` で ID と理由を 1 行記録する（ukadoc は「ID にジャンプする」の行き先を定めていないので、推測で動かさない。補った URL として開くことも、選択肢として扱うこともしない）。

### Requirement 2: `\![open,file,ファイル名]` でファイルを実行する

**Objective:** As a ゴースト作者, I want `\![open,file,…]` でゴーストの中のファイルや OS のプログラムを開ける, so that 説明書・画像・外部ツールをバルーンから渡せる

#### Acceptance Criteria

1. When 台本に `\![open,file,ファイル名]` があり、ファイル名が絶対パス, the areka shall そのファイルを OS の関連付けで開く（実行ファイルなら実行する）。
2. When 台本に `\![open,file,ファイル名]` があり、ファイル名が相対パス, the areka shall `ghost/master` を基準に解決してから開く（ukadoc の例 `shell/master/surface0.png` は `ghost/master/shell/master/surface0.png`）。
3. When ファイル名に Windows の環境変数（`%SYSTEMROOT%`・`%TEMP%`・`%ComSpec%` など）が含まれる, the areka shall OS の規則で展開してから開く。
4. When ファイル名が実行ファイルの名前だけ（パスの区切りを含まない・例 `TortoiseProc.exe`）で `ghost/master` の下に無い, the areka shall OS のパス探索（`PATH` と OS の既定の探索）に任せて実行する。
5. If 解決したファイルが無い、または OS が開けなかった, then the areka shall 何も開かず、`error!` で行き先と OS の返した理由を 1 行記録し、ゴーストの動作を続ける。
6. If `\![open,file]` の引数が無い・空, then the areka shall 何も開かず `warn!` で 1 行記録する。

### Requirement 3: `\![open,browser,パラメータ]` で既定のブラウザを開く

**Objective:** As a ゴースト作者, I want `\![open,browser,URL]` で利用者のブラウザが開く, so that 公式サイトや配布ページへ案内できる

#### Acceptance Criteria

1. When 台本に `\![open,browser,パラメータ]` がある, the areka shall パラメータを URL として OS の既定のブラウザで開く（SSP の「外部アプリ → ブラウザの設定」は写さない）。
2. If パラメータが無い・空, then the areka shall 何も開かず `warn!` で 1 行記録する。
3. If OS が開けなかった, then the areka shall `error!` で行き先と理由を 1 行記録し、ゴーストの動作を続ける。

### Requirement 4: `\![open,explorer,…]` でフォルダとファイルの置き場所を示す

**Objective:** As a ゴースト作者, I want `\![open,explorer,…]` でフォルダを開いたりファイルの場所を示したりできる, so that 利用者に設定ファイルや保存先を見せられる

#### Acceptance Criteria

1. When 台本に `\![open,explorer,ファイル]` があり、解決した先がフォルダ, the areka shall そのフォルダを OS のファイル管理の窓で開く。
2. When 台本に `\![open,explorer,ファイル]` があり、解決した先がファイル, the areka shall そのファイルを含むフォルダを、そのファイルを選んだ状態で OS のファイル管理の窓で開く（ukadoc「ファイラーの指定がない場合」のふるまい）。
3. When `ファイル` が相対パス, the areka shall `ghost/master` を基準に解決する（要件 2.2 と同じ規則）。
4. When 台本に `\![open,explorer,種類,名前]` があり、種類が `ghost`, the areka shall ベースウェアの目録でその名前のゴーストを引き、そのゴーストのフォルダを開く（名指しの引き方＝`descript` の `name` → フォルダ名の順は `\![change,ghost,名前]` と同じ。`random`・`sequential`・`lastinstalled` などの特別な名前は解かない）。
5. When 台本に `\![open,explorer,種類,名前]` があり、種類が `balloon`, the areka shall ベースウェアの目録でその名前のバルーンを引き、そのフォルダを開く。
6. When 台本に `\![open,explorer,種類,名前]` があり、種類が `shell`, the areka shall 実行中のゴーストの `shell` の中からその名前のシェルのフォルダを開く。
7. If 種類が `headline`・`plugin`（areka に無い仕組み）または上のどれでもない, then the areka shall 何も開かず `warn!` で種類と名前を 1 行記録する（台帳では縮退として書く）。
8. If 名前に当たるゴースト・バルーン・シェルが無い、または解決した先が存在しない, then the areka shall 何も開かず `error!` で種類・名前・理由を 1 行記録し、ゴーストの動作を続ける。

### Requirement 5: `\![open,editor,ファイル,表示行]` でファイルを編集用に開く

**Objective:** As a ゴースト作者, I want `\![open,editor,…]` でテキストファイルを編集できる形で開ける, so that 利用者に設定ファイルを直してもらえる

#### Acceptance Criteria

1. When 台本に `\![open,editor,ファイル,表示行]` がある, the areka shall そのファイルを、OS で「編集」を選んだときと同じアプリで開く（ukadoc「エディタの指定がない場合は、エクスプローラ上で右クリックして『編集』を選んだ場合と同じ挙動」）。
2. The areka shall `表示行` を無視する（ukadoc「この時は『表示行』は無視される」）。
3. When `ファイル` が相対パス, the areka shall `ghost/master` を基準に解決する（要件 2.2 と同じ規則）。
4. If OS に「編集」の関連付けが無い、またはファイルが無い, then the areka shall 何も開かず `error!` で行き先と理由を 1 行記録し、ゴーストの動作を続ける。

### Requirement 6: `\![open,mailer,パラメータ]` で新しいメールを開く

**Objective:** As a ゴースト作者, I want `\![open,mailer,宛先]` で利用者のメールソフトが開く, so that 感想や不具合の連絡先を渡せる

#### Acceptance Criteria

1. When 台本に `\![open,mailer,パラメータ]` がありパラメータがメールアドレス, the areka shall その宛先が入った新しいメールを OS の既定のメールソフトで開く（ukadoc の例 `test@example.com`）。
2. When パラメータが `mailto:` で始まる, the areka shall そのまま `mailto:` として開く（二重に付けない）。
3. If パラメータが無い・空, then the areka shall 何も開かず `warn!` で 1 行記録する。
4. If OS が開けなかった（メールソフトが無いなど）, then the areka shall `error!` で行き先と理由を 1 行記録し、ゴーストの動作を続ける。

### Requirement 7: 開く処理は 1 か所・開くたびに記録を残す

**Objective:** As a 運用者（開発者・利用者）, I want 外のものを開いた事実と失敗の理由が必ず記録に残る, so that ゴーストが何を開いたかを後から追え、`script-impact-tiers` の同意の窓を 1 か所に差し込める

#### Acceptance Criteria

1. The areka shall 6 つの形すべての「OS に渡して開く」を 1 か所の開く処理で行い、その 1 か所以外から OS のアプリを起こさない（既存の `\![open,readme]` も同じ 1 か所を通す）。
2. When 開く処理が行き先を OS に渡す, the areka shall `info` で 1 件につき 1 行を記録し、その行に 種類（URL・ファイル・フォルダ・メール・エディタ）・解決した行き先・出どころのゴースト名・元のタグの綴り を含める。
3. When 開く処理が成功した, the areka shall MCP の `get_log` でその記録を読める形で残す（`doc/ssp-mcp/log-convention.md` の取り決めに従う。どの種別に入れるかは設計で決め、取り決めの表に行を足すならテストも同時に直す）。
4. If 開く処理が失敗した（行き先が無い・OS が断った・関連付けが無い）, then the areka shall `error!` で 種類・行き先・OS の返した理由（符号）を 1 行記録し、`get_log` の `error` 種別で読めるようにする。失敗の記録が無い失敗経路は 0 本。
5. While 開く処理が OS を呼んでいる, the areka shall ゴーストの画面更新・入力・台本の再生を止めない（開くのに時間がかかっても窓が固まらない）。
6. If 開けなかった, then the areka shall ゴーストの動作をそのまま続け、メッセージボックスを出さない（失敗は記録で伝える）。
7. The areka shall 外部アプリの選択を OS の既定（関連付け・URL の既定のアプリ・「編集」の関連付け）だけに委ね、areka 独自の設定値・設定画面・設定ファイルを 0 個にする。
8. The areka shall 同じ台本に開く系のタグが複数あれば、台本の順に 1 つずつ開く（取りこぼし 0・順序の入れ替え 0）。

### Requirement 8: 台本から開く系の行き先を取り出す純粋な関数

**Objective:** As a 後続の spec（`link-context-copy`・`balloon-link-hover`）の実装者, I want 台本の文字列から開く系の行き先を順に取り出す関数, so that 右クリックのコピーやホバーの説明が、開く処理と同じ規則で行き先を知れる

#### Acceptance Criteria

1. When 台本の文字列を渡す, the 行き先を取り出す関数 shall `\j[X]` と `\![open,file|browser|explorer|editor|mailer,X]` の X を、台本に現れた順に一覧で返す。
2. The 行き先を取り出す関数 shall 各項目に 種類（URL・ファイル・フォルダ・メール・エディタ）と 元の綴り を添える。
3. The 行き先を取り出す関数 shall 開く系以外のタグ（`\![open,readme]`・`\![open,help]`・`\q`・`\_a` など）と本文の文字を一覧に含めない。
4. The 行き先を取り出す関数 shall OS にもファイルにも記録にも触れず、同じ入力に同じ一覧を返す（純粋）。相対パスの解決・環境変数の展開はこの関数では行わず、書かれた綴りのまま返す。
5. When 台本に開く系のタグが 1 つも無い, the 行き先を取り出す関数 shall 空の一覧を返す。
6. The 開く処理 shall 行き先の読み取り（どのタグのどの引数が行き先か）をこの関数と同じ規則で行い、2 つの規則が食い違わないことをテストで固定する。

### Requirement 9: 網羅台帳と受け取り手の表の更新

**Objective:** As a 互換性の記録を見る人, I want 台帳と受け取り手の表が実装と一致している, so that 「areka で何が開けるか」を台帳だけで判断できる

#### Acceptance Criteria

1. When 本 spec が完了する, the 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` shall `\j[ID]`・`\![open,browser,…]`・`\![open,file,…]`・`\![open,explorer,…]`・`\![open,editor,…]`・`\![open,mailer,…]` の 6 行の `status` を `absent` から実装に合う語（URL・`file:///`・`mailto:` 以外の ID を開かない `\j[ID]` と、`headline`・`plugin` を開かない `\![open,explorer,…]` は `degraded`、残りの 4 行は `implemented`）に改め、`owner` を本 spec とし、`note` に壊れ方とログの実態を書き直す。`implemented` にした行は、台帳の検査（`ukadoc-survey`）が求める `/// ukadoc: <URL>` の出典コメントを実装のソースに持つ。
2. The 受け取り手の表（`consumer_ledger.rs` の正準台帳） shall `("open","file")`・`("open","browser")`・`("open","explorer")`・`("open","editor")`・`("open","mailer")` の受け取り手を登記し、`("open","readme")` の登記はそのまま残す。
3. The 受け取り手の表の既存テスト（`("open","browser")` に受け取り手が無いことを固定しているものと、登記の総数 15 を固定しているもの） shall 新しい登記に合わせて書き替え、`("open","help")` など本 spec の範囲外の第 1 引数は受け取り手なしのままであることを固定する。総数は main を取り込んだ後の実物の行を数え直して書く（並走の `balloon-lifecycle-events` も同じ表に `("set","balloontimeout")` の 1 行を足す。後から main へ入る側が、相手の変種・登録の行を落とさず手で足し直し、数を数え直す。2026-10-05 相互の約束。並走の `mcp-author-tools` も同じ表に 2 行と一致のテスト `consumer_ledger_agreement_tests.rs` を足すので、同じ規則で、後から入る側が相手の行と見本を落とさず足し直し、総数とモジュールの doc の行数を数え直す）。
4. When 本 spec が正典の沈黙や SSP との差を裁量で決めた（外部アプリの設定を写さない・`表示行` を無視する・`headline`／`plugin` を開かない・`\j[ID]` の旧来のジャンプ）, the `doc/COMPAT_ARCHITECTURE.md` §8 shall その裁量を 1 行ずつ登記する。

### Requirement 10: 決定論テスト（OS は偽の境界）

**Objective:** As a 開発者, I want 常時テストが実際のアプリを 1 つも起こさずに 6 つの形の判断を固定する, so that 全体テストが環境に依らず緑のまま、判断の分岐の後退に気付ける

#### Acceptance Criteria

1. The テスト shall OS に渡す境界を偽物に差し替え、「何を・どの動詞で・どの引数で渡したか」を記録して判定する。常時テストで実際に OS のアプリを起こす回数は 0。
2. The テスト shall 6 つの形それぞれについて、行き先の解決（絶対・相対・環境変数・名前だけの実行ファイル・`種類,名前`）と、成功・失敗の記録の有無・レベル・欄を判定する。
3. The テスト shall `\j[ID]` が 3 つの形のどれでもないとき（要件 1.7）と、引数が無いとき（要件 2.6・3.2・6.3）に OS を呼ばず記録だけ残すことを判定する。
4. The テスト shall 行き先を取り出す関数について、6 つの形の混在・開く系以外のタグの除外・空の一覧・順序を判定する（要件 8）。
5. The テスト shall 失敗の記録が無い失敗経路が 0 本であることを、偽の境界に失敗を返させて判定する（要件 7.4）。
6. The テスト shall x64 で決定論に動き、32bit の helper・実物の SHIORI・ネットワークを要しない。

## 零の明示

- dola の `CueCommand` に足す新しい種類: **0**。
- 新しい外部クレート・`Cargo.toml`／`Cargo.lock` の変更: **0**（OS の呼び出しは既存の `windows` クレートの範囲で行う）。
- 外部アプリの設定値・設定画面・設定ファイル: **0**。
- 同意の窓・確認のダイアログ・メッセージボックス: **0**。
- 記録を残さない失敗経路: **0**。
- 常時テストで実際に起こす OS のアプリ: **0**。
- `main.rs`・`emo2_boot/mod.rs` への変更: **0**（同じウェーブ C4 の約束）。

## 要件ディスカッションで片付いた項目

- **`\j[ID]` の ID が URL でも `file:///` でも `mailto:` でもないとき**（要件 1.7・2026-10-05 開発者裁定「http とかが無ければ処理できない方がよい」）: 何も開かず `warn!` 1 行。ukadoc は「IDにジャンプする」とだけ書き、何へ跳ぶかを定めていない（URL・`file:///`・`mailto:` 以外の意味は正典に無い）。選択肢を選んだときと同じ扱いにする案・`http://` を補う案は取らない。台帳の `\j[ID]` は `degraded`、kanade には触れない。

- **`\![open,explorer,種類,名前]` の名前解決の範囲**（要件 4.4〜4.7）: `ghost`・`balloon`・`shell` の名前解決は本 spec で持つ（要件 4.4〜4.6 のまま）。名前から 1 つを引く純粋な関数は 3 つとも既にあり（ギャップ分析 §2.5）、足すのは結線だけ。`headline`・`plugin` は areka に仕組みが無いので `warn!` 1 行の縮退（要件 4.7）。
- **成功の記録を `get_log` のどの種別に入れるか**（要件 7.3）: 要件は「`get_log` で読める」ことだけを約束し、種別は設計で決める（ギャップ分析 §6.3）。
