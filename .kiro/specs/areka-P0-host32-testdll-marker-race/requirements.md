# Requirements Document

## Project Description (Input)
全体テスト（`tools/test-all.ps1`）の i686 の段が、コードを変えていないのにまれに赤になる。2026-10-04 の `translate-pipeline` の完了の手順の中で、`cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc --no-fail-fast` の `shiori_proxy::tests::testdll_drop_invokes_courtesy_unload` が 1 度だけ「unload はまだ呼ばれていない」で落ち、その段だけの採り直し 3 回は緑だった。困るのは開発者で、完了の手順の途中で赤を見ると原因を調べて採り直す手間がかかり、本物の赤を見落とす元にもなる。このテストは、プロセス全体の環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` に自分の印のパスを差してから偽の SHIORI DLL（`shiori-host32-testdll`）を load し、drop の後片付けの unload でその DLL が印のファイルを作ることを確かめる。偽の DLL は unload のたびにこの環境変数を読む。ところが同じテストのプロセスで、印のテストと直列にならずに偽の DLL を load して unload するテスト（`main_loopback_tests.rs` の loopback のテスト）があり、その unload が印のテストの「drop の前」に走ると、印のテストの印を先に作ってしまう。偽の DLL を load するテストが何本あっても印のテストが他のテストの unload に左右されないようにし、時間待ちを使わずに決定論で直し、i686 の段を 10 回続けて回して緑を確かめる。製品の振る舞いは変えない。種別はバグ（テストの揺れ）。

## Introduction

host-32 の補助 exe（`shiori-host32-helper`）には、32 ビットでしか走らないテストがある。そのうち 3 本が偽の SHIORI DLL（`shiori-host32-testdll`・出力名 `shiori.dll`）を実際に load して unload する。3 本とも補助 exe のテストのバイナリに入り、同じプロセスで、既定では並行に走る。

- `shiori_proxy.rs` の `mod tests` の `testdll_drop_invokes_courtesy_unload`（以下「印のテスト」）: プロセス全体の環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` に自分の一時フォルダの印のパスを差し、load の直後・drop の前に「印のファイルはまだ無い」を確かめ、drop の後に「印のファイルが在り、中身が `unloaded`」を確かめる。
- 同じ `mod tests` の `testdll_request_roundtrip_get_and_notify`: 偽の DLL へ要求を往復させ、最後に drop で unload する。
- `main_loopback_tests.rs`（`main.rs` から別のファイルとして読み込む loopback のテスト）の `loopback_hello_request_proxy_driven_and_bounded_loop`: 補助 exe の窓を組み、LOAD の経路で偽の DLL を読ませ、途中の UNLOAD の知らせで unload し、最後に窓の後片付けでもう一度 unload しうる。

偽の DLL の unload の出口（`crates/shiori-host32-testdll/src/lib.rs` の `unload` の定義）は、呼ばれるたびに環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` を読み、値があればそのパスへ印を書く。写した DLL でも同じコードが走る。印のテストと他のテストの競合を避けるための直列化（`mod tests` の中の `TESTDLL_SERIAL`）は、同じ `mod tests` の 2 本だけが取っており、loopback のテストは取っていない。よって、印のテストが環境変数を差してから drop するまでの間に loopback のテストの unload が走ると、印のテストの印が先に作られ、「drop の前は印が無い」の確かめが落ちる。これは観測した症状（「unload はまだ呼ばれていない」で 1 度だけ落ちる）と一致する。起票時の見立てであり、実走での再現はまだしていない。

印のテストと直列化の説明は「testdll を load する唯一のテスト」「本テストのみが marker env を set/remove する」と書いており、loopback のテストがある今の事実と合っていない。

同じテストのバイナリのほかのテストは次のとおりで、この揺れには関わらない（起票時にコードの上で確認）。

- `shiori_proxy_loadu_tests.rs` の群 D は、別の偽の DLL（`shiori-host32-testdll-loadu`・出力名 `shiori_loadu.dll`）を読み、環境変数も `HOST32_TESTDLL_LOADU_*` で別。自前の直列化（`LOADU_SERIAL`）を持つ。`HOST32_TESTDLL_UNLOAD_MARKER` は読まない。
- `main_classify_tests.rs`・`main_load_ack_tests.rs`・`main_resolve_param_tests.rs`・`main_response_flavor_hung_cage_tests.rs` は偽の DLL を load しない。

直し方は 2 つの候補を設計で決める（偽の DLL を load するテストすべてを同じ直列化に入れる／印の置き場をテストごとに分ける）。本要件は、どちらを採っても満たすべき結果だけを定める。

## Boundary Context

- **In scope**:
  - 補助 exe のテストのうち偽の DLL（`shiori-host32-testdll`）を load するもの（起票時の見立てで 3 本）と、その間の印の競合を断つこと。
  - 印の置き場をテストごとに分ける案を設計で採る場合に限り、偽の DLL の印の書き方を変えること。
  - 直列化と印のテストの説明（コメント）を、修正後の事実に合わせること。
  - 競合の経路を示すこと（決定論で起こせれば赤のテスト、起こせなければコードの上の経路の記録）と、i686 の段を 10 回続けて回して緑を確かめること。
- **Out of scope**:
  - 製品の補助 exe の LOAD・unload の手順、`ShioriByteProxy` の振る舞い（テストでない部分のコード）。
  - x64 側（`shiori-host32-host`）の host-32 の通しのテスト。
  - 2 本目の偽の DLL（`shiori-host32-testdll-loadu`）と、それを読むテスト（群 D）とその直列化。
  - 全体テストの手順（`tools/test-all.ps1`）の変更。採り直しの仕組みを足して揺れを隠すことはしない。
  - 同じクレートのテストにある clippy の指摘（`areka-P0-clippy-199-lints` の範囲）。
- **Adjacent expectations**:
  - x64 側の host-32 の通しのテストは、補助 exe を別プロセスで起こして同じ偽の DLL を読ませる。本 spec が偽の DLL に触れる場合でも、それらのテストの判定と結果は変わらない前提で進める。
  - ワークスペース全体の見張り（1 ファイル 1,000 行の見張り、テスト用一時パスの窓口の迂回の見張り）は、本 spec の後も緑のまま保つ。一時パスの見張りの例外表には、補助 exe の `shiori_proxy.rs` と `main_loopback_tests.rs` が「プロセス識別子で一意化済み」として載っている。

## Requirements

### Requirement 1: 印のテストが他のテストの unload に左右されない

**Objective:** As a 開発者, I want 印のテストの合否が、そのテスト自身の drop の結果だけで決まること, so that 偽の DLL を load するテストが何本同じプロセスで走っても、i686 の段がコードを変えずに赤にならない

#### Acceptance Criteria

1. The i686 のテストの段 shall 印のテストの 2 つの確かめ（load の後・drop の前は印のファイルが無い／drop の後は印のファイルが在り中身が `unloaded`）を、印のテスト自身の drop による unload だけで決まるものにする（同じプロセスの他のテストの unload が、印のテストの印のファイルを作ることがない）。
2. The i686 のテストの段 shall 前項を、テストの実行の順、並行に走らせる数、同時に走るテストの組み合わせ（偽の DLL を load する他のテストが、印のテストの load から drop までの間に走る場合を含む）によらず保つ。
3. If 偽の DLL を load するテストの 1 本が途中で失敗したとき, then the i686 のテストの段 shall 他のテストを止めずに最後まで走らせ、各テストの合否をそれぞれ報告する（1 本の失敗が、別のテストの赤や停止に連鎖しない）。

### Requirement 2: 時間待ちに頼らず、既存の確かめを弱めない

**Objective:** As a 開発者, I want 修正が決定論で、既存のテストの確かめを減らさないこと, so that 揺れを「起きにくくした」のでなく「起きない」と言え、修正で検出力を失わない

#### Acceptance Criteria

1. The 修正 shall 要件 1 を、時間待ち（一定時間の sleep、時間切れまで待つ同期）に頼らずに満たす。
2. The 修正 shall 印のテストの「drop の前は印のファイルが無い」と「drop の後は印のファイルが在り中身が `unloaded`」の 2 つの確かめを、外さず、緩めずに残す。
3. The 修正 shall 偽の DLL を load する各テスト（起票時の見立てで 3 本）の本数と、それぞれの確かめの内容を減らさない。
4. The 修正 shall 偽の DLL を load するテストを、i686 のときに走る形のまま保つ（無視の印を足して走らせなくすることで揺れを消さない）。

### Requirement 3: 競合の経路を示し、塞いだことを確かめる

**Objective:** As a 開発者, I want 揺れの原因の経路が名指しで示され、修正がその経路を塞いだ証拠が残ること, so that 「採り直したら緑」でなく、原因を断ったことを根拠をもって言える

#### Acceptance Criteria

1. When 着手したとき, the 本 spec の作業 shall 補助 exe のテストのバイナリで偽の DLL（`shiori-host32-testdll`）を load するテストと、環境変数 `HOST32_TESTDLL_UNLOAD_MARKER` を読み書きするコードを全数で数え上げ、起票時の見立て（load するテストは 3 本・印を差すのは印のテストだけ）と合うかを記録する。
2. Where 揺れを時間待ちなしの決定論で起こせる形がある, the 本 spec の作業 shall 修正の前に赤になり、修正の後に緑になるテストを添える。
3. If 揺れを決定論で起こせる形が無いとき, then the 本 spec の作業 shall 競合の経路（どのテストのどの段の unload が、印のテストのどの確かめの前に印を書きうるか）を、コードの定義の名前で spec の記録に書き、修正の後はその経路が成り立たないことをコードの上で示す。
4. If 数え上げの結果が起票時の見立てと違うとき（偽の DLL を load するテスト、または印の環境変数に触れるコードが他にもある）, then the 本 spec の作業 shall 見つかったものすべてを要件 1 の対象に含める。

### Requirement 4: 走らせて緑を確かめる

**Objective:** As a 開発者, I want 修正の後に i686 の段を繰り返し回して緑であることを確かめられること, so that 決定論の主張を実走でも裏付けられる

#### Acceptance Criteria

1. When 修正の後に i686 の段（`cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc --no-fail-fast`・前提として i686 の成果物をビルド済み）を 10 回続けて回したとき, the i686 のテストの段 shall 10 回とも緑で終わる。
2. If 10 回のうち 1 回でも赤になったとき, then the 本 spec の作業 shall 修正を完了とせず、その赤の原因を調べる（採り直しの緑で置き換えない）。
3. When 修正の後に全体テスト（`tools/test-all.ps1`）を回したとき, the 全体テスト shall x64 側の host-32 の通しのテストとワークスペース全体の見張りを含めて緑で終わる。

### Requirement 5: 製品と隣のテストを変えない

**Objective:** As a 開発者, I want 修正がテストの側だけに閉じること, so that テストの揺れを直したことで、製品の振る舞いや他のテストの意味が変わらない

#### Acceptance Criteria

1. The 修正 shall 補助 exe の製品のコード（LOAD・unload の手順、`ShioriByteProxy` の振る舞い。テストのときだけ組み込まれる部分を除く）を変えない。
2. Where 印の置き場をテストごとに分ける案を採る, the 偽の DLL（`shiori-host32-testdll`）shall load・request・unload の各出口の返り値と、load の失敗を強いる環境変数 `HOST32_TESTDLL_LOAD_FAIL` の働きを変えない。
3. The 修正 shall 2 本目の偽の DLL（`shiori-host32-testdll-loadu`）と、それを読むテスト（群 D）とその直列化を変えない。
4. The 修正 shall 触ったファイルを 1 ファイル 1,000 行未満に保つ。

### Requirement 6: 説明を事実に合わせる

**Objective:** As a 開発者, I want 直列化と印のテストの説明が、どのテストが偽の DLL を load し、どう印の競合を避けているかを正しく書いていること, so that 後から偽の DLL を load するテストを足す人が、同じ揺れを作り直さない

#### Acceptance Criteria

1. The 修正 shall 印のテストと直列化の説明にある「testdll を load する唯一のテスト」「本テストのみが marker env を set/remove する」などの記述を、修正後の事実（偽の DLL を load するテストの一覧と、印の競合を避ける仕組み）に合わせる。
2. The 修正 shall 偽の DLL を load するテストを今後足すときに従う決まり（何をすれば印のテストと競合しないか）を、偽の DLL を load するテストのそばの説明に 1 か所で書く。
