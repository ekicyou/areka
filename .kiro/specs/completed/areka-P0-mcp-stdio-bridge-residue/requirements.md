# Requirements Document

## Project Description (Input)
- **困っている人**: 中継 `areka-mcp-bridge`（Claude Desktop と areka の MCP の受け口をつなぐ exe）と、配布スクリプト `tools/package.ps1` へ次に手を入れる開発者。
- **今の状況**: 中継は実機で動いている（Claude Desktop から areka のツールを呼べる）。ただ、完了した `areka-P0-mcp-stdio-bridge` の最終検証とタスク 4.1・4.6 の記録で、「今は困らないが、守る仕組みが無い・同じ形が 2 か所に在る・端で動きが止まりうる」と分かった所が 8 つ残っている（件 1〜8）。配布スクリプトは中継の「窓を出さない作り」を確かめていない。偽の受け口のテストが 2 回走る。上限つきの接続が 2 か所に在る。2 つの枝（`id` を持つ要求への 202・標準入力が閉じて 5 秒を越える要求）にテストが無い。番号の指定が読めないときの中継の記録が「待ち受ける」と書く。端の動き 2 つ（壊れた行の書きが詰まったとき・areka の前に黙る相手が 10 個以上並んだとき）がどこにも書かれていない。放っておくと、後で中継や配布スクリプトを触る人が、守られていない所を知らずに壊す。
- **変えたいこと**: 配布の検査が窓の作りを見て、違えば赤になる。重複はそれぞれ 1 つになる（動きは変えない）。テストの無い 2 つの枝にテストが 1 本ずつ入る。中継の記録の文が中継の事実に合う。端の動き 2 つは限界として差の記録に書いて閉じる。中継の答えの形（完了した spec の要件 1〜4 の約束）は変えない。

## Introduction

中継 `areka-mcp-bridge` は、Claude Desktop が標準入出力で送る 1 行 1 要求を、`127.0.0.1` で待ち受ける areka の MCP の受け口へ届ける exe で、`areka-P0-mcp-stdio-bridge`（2026-10-11 完了）で作られた。この spec は、その最終検証と実装の記録に残った拾い残しを 1 本にまとめて片づける（`impl-watch-residue`・`popup-menu-residue` と同じ型）。

拾い残しは次のとおり（番号は brief の件の番号）。

- **配布の検査の穴**: (1) 配布スクリプトが、zip の中の `areka.exe` と `areka-mcp-bridge.exe` が「窓を出さない作り」で建っているかを確かめていない。
- **同じ形の重複**: (2) 偽の受け口のテストが 2 つのテストの束で 2 回走る。(3) 上限つきの接続が探索と要求の届けの 2 か所に同じ形で在る。
- **テストの穴**: (4) `id` を持つ要求に受け口が 202 を返したときの枝。(5) 標準入力が閉じてから 5 秒を越えて返る要求の枝。
- **記録の文**: (6) `AREKA_MCP_PORT` が読めないときの記録が、中継でも「待ち受ける」と書く（中継は待ち受けない）。
- **端の動き**: (7) 壊れた行への答えの書きが詰まっていると、標準入力が閉じても 6 秒の見切りが効かない。(8) 候補の並びで areka より前に黙る相手が 10 個以上並ぶと、その回の探索で areka を飛ばす。

この文書で使う言葉:
- **中継**: `areka-mcp-bridge.exe`。Claude Desktop が起こす。
- **受け口**: areka 本体が `127.0.0.1` で待ち受ける HTTP の MCP の口。
- **配布の検査**: 配布スクリプト `tools/package.ps1` が、作った zip の中身を判定する段。今は、必須の項目・許可表・起動記録の混入・実行ファイルの許可表・PE の機種・取り込む DLL などを見る。
- **窓の作り**: exe の PE の頭にある Subsystem の欄。「GUI の作り」は、起こしてもコンソールの黒い窓が出ない作り（今の `areka.exe` と中継はこれ）。「コンソールの作り」は、起こすとコンソールの窓が付く作り。
- **差の記録**: `doc/ssp-mcp/bridge-diff-areka.md`。SSP の中継と areka の中継が同じ入力に何を返すかを行ごとに並べた文書。
- **黙る相手**: 候補の番号で接続は受けるが、名乗りの問いに答えない相手。
- **決定論テスト**: 全体テストが毎回走らせ、時間や机の状態に左右されずに同じ結果を出すテスト。
- **変異を当てる**: 守りたい所を一時的に壊し、新しい判定やテストが赤になることを確かめてから元へ戻すこと。

## Boundary Context

- **In scope**:
  - `crates/areka-mcp-bridge/` の中のコードとテスト（ただし `src/config.rs` を除く）の上での件 2〜5 と、件 6 の中継の側。
  - 配布スクリプト `tools/package.ps1` の、zip の中身の判定のうち PE を読む所への窓の作りの判定の追加（件 1）。
  - 件 6 を共有の側で直すときの、`crates/areka-mcp/src/port.rs` の記録の 2 行と、その文を確かめる兄弟のテスト `crates/areka-mcp/src/port_tests.rs` の期待。
  - 差の記録 `doc/ssp-mcp/bridge-diff-areka.md` の、件 4・5・6・7・8 に当たる行と、行を足したときの文書の頭の行の数。
- **Out of scope**:
  - 中継の答えの形（完了した spec の要件 1〜4 の約束）・組み込みの答え（完了した spec の設計 D-18）・受け口の答え（`areka-P0-mcp-receiver-bugs` の件 1）・SSE や取り消しへの対応（完了した spec の設計「残る縛り」）。
  - 件 7・8 の動きを直すこと（この spec では限界として差の記録に書いて閉じる。Requirement 7。2026-10-16 要件の討議で開発者が承認＝どちらも実機では起きず、直すと輸送と探索の並べ方を作り直すことになるため）。
  - 件 9（arm64 の機械で中継が起きるかの確かめ）。次のリリース `v0.0.3` の `/release` の走りの記録で見る申し送りとし、この spec では扱わない（2026-10-16 棚卸㉖の約束）。
  - `crates/areka-mcp-bridge/src/config.rs`（控えの置き場の決め方）。同じウェーブの `areka-P0-user-data-root` が持つ（2026-10-16 棚卸㉖の約束）。
  - `shiori-host32-helper.exe` と `pasta.dll` の窓の作り（helper は areka が窓なしの作成フラグで起こすコンソールの作りで、それで正しい）。
  - 配布スクリプトの、PE を読む所と zip の中身の判定のほかの段（組み立て・静的リンクの検査・起動の検査・版の扱いなど）。
  - 説明書 `dist/README.txt`（件 7・8 は開発者向けの差の記録にだけ書く。2026-10-16 要件の討議で開発者が承認＝実機でまず起きない条件を利用者向けに書くと迷わせる・声が出てから足す）。
  - Claude Desktop の側の振る舞い（中継を 2 つ起こす・エラーの後に起こし直す、など）。
  - 新しい外部のクレートの追加（開発用を含む）。
- **Adjacent expectations**:
  - `tools/package.ps1` は `release-code-signing`・`install-live-target-hazards`・`winget-release-automation` も触る。この spec は PE を読む所と zip の中身の判定の 1 項目だけを触り、ほかの段の振る舞い・出力の形・終了コードを変えない。
  - `areka-P0-mcp-receiver-bugs` は `tests/real_receiver_test.rs` を触る。この spec はこのファイルを触らない。
  - 件 9 の結果は、`/release` の `v0.0.3` の走りの記録が受け持つ。そこが赤だったときの直しは、その時点で別に起票する。

## Requirements

### Requirement 1: 配布の検査が窓の作りを確かめる
**Objective:** As a 配布スクリプトや中継・本体の入口へ手を入れる開発者, I want 配布の検査が zip の中の `areka.exe` と中継の窓の作りを見て、GUI の作りでなければ赤になる, so that 窓を出さない宣言を誤って消しても、中継が黒い窓を出す zip を配ってしまう前に気付ける

#### Acceptance Criteria
1. When 配布の検査が作った zip の中身を判定する, the 配布スクリプト shall zip の最上位の `areka.exe` と `areka-mcp-bridge.exe` の窓の作りを読み、exe の名前と読んだ値を端末へ 1 行ずつ示す。
2. If `areka.exe` または `areka-mcp-bridge.exe` の窓の作りが GUI の作りでない, then the 配布スクリプト shall zip の中身の判定を赤にし、赤の理由の行に exe の名前・読んだ値・期待した値を載せる。
3. If `areka.exe` または `areka-mcp-bridge.exe` の窓の作りを読めない, then the 配布スクリプト shall 判定を赤にし、読めなかったことを赤の理由に載せる（黙って通さない）。
4. When 配布スクリプトが複数の CPU 種別の zip を作る, the 配布スクリプト shall 1〜3 をそれぞれの zip に当てる。
5. The 配布スクリプト shall `shiori-host32-helper.exe` と `pasta.dll` を窓の作りの判定にかけない。
6. The 配布スクリプト shall 窓の作りの判定のほかの、今ある中身の判定の項目・出力の形・終了コードを変えない。
7. The areka-P0-mcp-stdio-bridge-residue spec shall 窓の作りがコンソールの作りの `areka-mcp-bridge.exe` を入れた zip で、配布の検査が 2 の赤になることを 1 回確かめ、その出力をこの spec の文書に記録する。

### Requirement 2: 偽の受け口のテストを 1 回だけ走らせる
**Objective:** As a 中継のテストを走らせる開発者, I want 偽の受け口の部品のテストが全体テストで 1 回だけ走る, so that 同じテストが 2 つの束で走って時間を食い、片方だけの赤に迷うことが無い

#### Acceptance Criteria
1. The areka-mcp-bridge shall 偽の受け口の部品のテスト（今の `tests/common/fake_server_tests.rs` の中身）を、全体テストの 1 回の走りの中で 1 回だけ走らせる。
2. The areka-mcp-bridge shall 偽の受け口を、中継の部品のテストと exe の通しのテストの両方から今のとおり使えるままにする。
3. The areka-mcp-bridge shall 1 の前後で、今走っているテストの名前（関数の名前。どの束のどの部品の下で走るかの道筋は変わってよい）がどれも少なくとも 1 回走ることを保つ（減るのは重複の 1 回分だけ）。
4. The areka-mcp-bridge shall 1〜3 で、偽の受け口の振る舞いとテストの判定を変えない。

### Requirement 3: 上限つきの接続を 1 か所にする
**Objective:** As a 中継の探索や届けを直す開発者, I want 受け口の番号へ上限つきで接続を開く手順が 1 か所にある, so that 片方だけを直して、探索と要求の届けで接続の待ち方が黙って食い違うことが無い

#### Acceptance Criteria
1. The areka-mcp-bridge shall `127.0.0.1` の番号へ上限つきで接続を開く手順を 1 か所で定め、探索（候補の番号を開く所）と要求の届け（つなぐ先へ要求を送る所）の両方がそれを使う。
2. The areka-mcp-bridge shall 接続の上限（300 ms）・開けなかったとき・上限を過ぎたときの扱い・記録・Claude Desktop への答えを今のまま保つ。
3. The areka-mcp-bridge shall 1 の後も、今ある中継のテストが判定を弱めずに通ることを保つ。

### Requirement 4: `id` を持つ要求に 202 が返ったときの枝を固定する
**Objective:** As a 中継の届けを直す開発者, I want 受け口が `id` を持つ要求に 202（受け取った・本文なし）で応えたときの今の振る舞いがテストで固定されている, so that この枝を誤って変えたとき（答えを書かない・空の結果を返す・二度答える など）に全体テストが赤になる

#### Acceptance Criteria
1. When 受け口が `id` を持つ要求に 202 で応えた, the areka-mcp-bridge shall その要求の答えを応答の上限まで待ち、上限を過ぎたら今の待ち時間切れのエラー（`No response from server: timed out after <上限>`）で、その `id` に 1 回だけ答える。
2. When 1 のエラーで答えた, the areka-mcp-bridge shall 同じつなぐ先で次の行に答え続ける。
3. The areka-mcp-bridge shall 1 と 2 を決定論テストで判定する（テストの中では応答の上限を短くして待ってよい）。
4. The areka-mcp-bridge shall 1 と 2 の振る舞いを、このテストのために変えない。

### Requirement 5: 標準入力が閉じて 5 秒を越える要求の枝を固定する
**Objective:** As a 中継の終わり方を直す開発者, I want 標準入力が閉じたときに途中の要求がどう扱われるかがテストで固定されている, so that 終わり方を誤って変えたとき（30 秒の応答の上限まで待つ・終わらない・終了コードが変わる など）に全体テストが赤になる

#### Acceptance Criteria
1. When 標準入力が閉じたとき、受け口へ届けた要求の応答がまだ返っておらず、閉じてから 5 秒を越えて返る, the areka-mcp-bridge shall その要求の応答を標準出力に書かずに、応答の上限（30 秒）を待たずに終了コード 0 で終わる。
2. When 標準入力が閉じたとき、受け口へ届けた要求の応答がまだ返っておらず、閉じてから 5 秒以内に返る, the areka-mcp-bridge shall その応答を標準出力に書いてから、終了コード 0 で終わる。
3. The areka-mcp-bridge shall 1 を決定論テストで判定する。2 が今あるテストで判定されていなければ、2 も判定する。
4. The areka-mcp-bridge shall 1 と 2 の振る舞い（5 秒は使っている MCP の部品の決まり）を、このテストのために変えない。

### Requirement 6: 番号の指定が読めないときの中継の記録の文
**Objective:** As a 中継の標準エラーの記録を読む開発者, I want `AREKA_MCP_PORT` が読めないときの中継の記録が、中継のしていることをそのまま書く, so that 待ち受けない中継の記録に「待ち受ける」とあって、何が起きたかを読み違えることが無い

#### Acceptance Criteria
1. If 中継が読んだ `AREKA_MCP_PORT` の値が UTF-8 でない, then the areka-mcp-bridge shall 既定の候補で areka を探すことが分かる記録を警告として 1 行残し、その行に「待ち受ける」と書かない。
2. If 中継が読んだ `AREKA_MCP_PORT` の値が番号として読めない（空・空白だけ・数でない・負・65536 以上）, then the areka-mcp-bridge shall 読めなかった値を添えて、1 と同じく既定の候補で areka を探すことが分かる記録を警告として 1 行残し、「待ち受ける」と書かない。
3. If areka 本体が読んだ `AREKA_MCP_PORT` の値が UTF-8 でない、または番号として読めない, then the areka shall 今と同じく警告を 1 行残し、その行の文が本体のしていること（既定の候補で待ち受けを試す）と食い違わないようにする。
4. The areka-mcp-bridge shall 1 と 2 で、記録の件数・段（警告）・環境変数の名前と値の項目・その後の振る舞い（既定の 20 個の候補を使う）を今のまま保つ。

### Requirement 7: 端の動き 2 つを限界として書き、差の記録を足したテストに合わせる
**Objective:** As a 中継の振る舞いを調べる開発者, I want 中継が端で止まりうる 2 つの場合が、起きる条件と避け方とともに差の記録に書いてある, so that 実機で出会ったときに、仕様どおりの限界だと分かり、調べ直さずに済む

#### Acceptance Criteria
1. The areka-P0-mcp-stdio-bridge-residue spec shall 件 7 を中継の限界として差の記録に書き、起きる条件（相手が壊れた行を送り、かつ標準出力を読まない）・起きたときの見え方（標準入力が閉じても 6 秒の見切りが効かず、中継が終わらない）・Claude Desktop では起きない理由（壊れた行を送らない）を添える。
2. The areka-P0-mcp-stdio-bridge-residue spec shall 件 8 を中継の限界として差の記録に書き、起きる条件（候補の並びで areka より前に黙る相手が 10 個以上並ぶ）・起きたときの見え方（その回の探索で areka を見つけられず、areka が居ないときと同じ答えになる）・SSP では起きない理由（すぐ名乗る）・避け方（`AREKA_MCP_PORT` で番号を 1 つに決める）を添える。
3. The areka-mcp-bridge shall 件 7 と件 8 の振る舞いを変えない。
4. The areka-P0-mcp-stdio-bridge-residue spec shall 差の記録の、件 4 と件 5 の枝を述べる所（4 節「応答の上限を越えたときに受け口へ届くもの」と 1 節「終わり」の「テスト」の欄、節「残る縛り」の「テストで固定していない」の並び）を、Requirement 4 と 5 で足したテストに合わせて直し、テストの名前を載せる。

### Requirement 8: テストと確かめの作法
**Objective:** As a この spec のテストを足し、確かめを記録する開発者, I want テストが机の本物の番号と控えに触れず、足した判定が本当に赤になることを確かめてある, so that テストが開発者の Claude Desktop や SSP の控えを壊さず、緑のテストが何も守っていないということが無い

#### Acceptance Criteria
1. When テストが中継の exe を起こす, the areka-mcp-bridge shall 番号（0 か、テストが自分で取った番号）と控えの置き場（ワークツリーの `target\` の下）を必ず渡し、机の 9801〜9830 の番号と `%LOCALAPPDATA%` に触れない。
2. When Requirement 4 と 5 のテストを足した, the areka-P0-mcp-stdio-bridge-residue spec shall そのテストが守る振る舞いを壊す変異を当てた実物で赤になり、戻した実物で緑になることを確かめる。守る振る舞いが使っている外の部品の決まりで、実物の側に当てられる変異が無いときは、その旨と理由を記録に残す。
3. When 変異を当てて戻す, the areka-P0-mcp-stdio-bridge-residue spec shall 元のファイルの控えをワークツリーの `target\` の下に取り、素のコピーで戻し、戻した後の中身が控えと同じ（sha256 が一致する）ことを確かめる（`git checkout` で戻さない）。
4. The areka-P0-mcp-stdio-bridge-residue spec shall Requirement 1 の 7 とこの要件の 2 の結果（当てた変異・赤になった判定やテスト）を、この spec の文書に記録する。
5. The areka-mcp-bridge shall 新しいテストを、テストする所と同じ場所の兄弟のテストファイルへ置き（建てた exe を起こす通しのテストは、今の終わり方のテストと同じ `tests/exe_test.rs` へ置く）、どのファイルも 1,000 行を超えない（869 行の `service_tests.rs` へは足さない）。
6. The areka-mcp-bridge shall この spec の全体で、新しい外部のクレート（開発用を含む）を足さない。
