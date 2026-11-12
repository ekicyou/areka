# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **32bit SHIORI の中継と試験用の DLL** を受け持つ（受け持つファイル＝`crates/shiori-host32-host/`・`crates/shiori-host32-helper/`・`crates/shiori-host32-ipc/`・`crates/shiori-host32-testdll/`・`crates/shiori-host32-testdll-loadu/` 全部。まだ audit をかけていない）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件。全体テストが緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。

あわせて、`audit-ghost-catalog` から申し送られた 2 件（件 1: i686 の helper が SHIORI の DLL を、依存の探し先を絞らずに読む・件 2: helper が親の `PASTA_DEBUG` をそのまま受け継ぎ、pasta が 127.0.0.1:9276 を取り合う）を、この spec で片づける。

## Introduction

この spec は、x64／arm64 の areka から 32bit の SHIORI の DLL を動かす中継（host・helper・その間の取り決め）と、その試験用の DLL 2 つを、ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

申し送りの 2 件は中身が決まっているので、要件 7・8 に外から見た結果として書く。直し方は要件の討議と設計で決める。

起票のときの範囲にあった `crates/shiori-abi/`（COM の面）と `crates/shiori4-testdll/` は、受け持ちから外す。2026-10-23 の開発者の決定（「COM-SHIORI インターフェースの作りかけは排除し、shiori4 に完全移行」）で `shiori4-api` が消すファイルであり（brief の 2026-10-23 の申し送り「受け持ちから外してよい」）、2026-11-13 の棚卸㊱も外すことを推している（残すと、その使い手の本体 `crates/areka/src/{reference_brain,shiori_demo,shiori_host,shiori_session}.rs` に直しが及び、C19 の約束 ⑴ に触れる）。外すことは要件の討議で開発者に確かめる。

この spec はウェーブ C19 で、ほかの 7 本（`wintf-d2d-composite-rules`・`dola-yaml-backend`・`runtime-module-untangle`・`audit-mcp-update-crates`・`audit-emo-atlas`・`emo-text-reveal-incremental`・`audit-impl-watch`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C19 の行の ⑴〜⑸）を守る。

## Boundary Context

- **In scope**:
  - `crates/shiori-host32-host/`・`crates/shiori-host32-helper/`・`crates/shiori-host32-ipc/`・`crates/shiori-host32-testdll/`・`crates/shiori-host32-testdll-loadu/` の全ファイル（本番のコード・テスト・テストの道具・fixture・`Cargo.toml`）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 3.2 と要件 6 の約束に従う。C19 のあいだは要件 6.3 で触れない所がある）。
  - 申し送りの件 1・件 2（要件 7・8）。
- **Out of scope**:
  - `crates/shiori-abi/` と `crates/shiori4-testdll/`（`shiori4-api` が消す。上の Introduction）。
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。
  - 新しい機能。
  - `vendors/` の下（`vendors/pasta` を含む）。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - 受け持つクレートを呼ぶ範囲の外のコードは、ほかの spec の持ち物である。本体 `crates/areka/`（`emo2_boot/spine.rs`・`spine_hold_support.rs` とそのテストなど）は C19 の `runtime-module-untangle` が書き換える所なので、C19 のあいだは触らず、`shiori-host32-host` の公開の形を変えない（要件 6）。本体の外の使い手（`crates/areka-ghost/`・`crates/areka-kanade/`・`pilot`）は C19 のほかの 7 本と重ならず、要件 6.1・6.2 の約束で触ってよい。
  - C19 で並ぶほかの 7 本とは、受け持つソースファイルが重ならない（棚卸㊱の測り）。
  - `clippy-199-lints` は `shiori-host32-host/tests/` などの clippy の分を受け持ちに数えている。本 spec が audit の指摘として clippy の指摘と同じ所を直したときは、`clippy-199-lints` が直し直さなくてよいように記録を残す（要件 3.5）。
  - 件 1 の手本は、`audit-ghost-catalog` が直した同じプロセス内の読み込み（`crates/areka-ghost/src/shiori_inproc.rs`）である。件 2 の手本は、実機の試験の道具が子へ `PASTA_DEBUG=0` を渡している形（`crates/real-run-kit/src/launch.rs`）である。どちらも本 spec では書き換えない。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall 受け持つ 5 つのクレートだけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）を測り直す
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（範囲を分けて読んだ結果をまとめた後の、3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘が申し送りの件 1・件 2 と同じ所を指すとき, the ギャップ分析 shall その指摘と件の番号の対応を報告に書く（同じ直しを 2 回数えない）
7. When audit の指摘を直すのに `shiori-host32-host` の公開の形（範囲の外から呼べる型・関数・欄の名前と形）を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの呼び出し元に響くかを書く（要件 6.5 の議題の材料）

### Requirement 2: 直す対象を要件の討議で確定する

**Objective:** As a 開発者, I want 指摘の一覧を見て、直す対象と直さずに残すものを自分で確定したい, so that 方針「指摘事項は全部直す」から外れるものが私の裁定なしに残らない

#### Acceptance Criteria

1. When 要件の討議を行うとき, the 要件の討議 shall ギャップ分析の指摘を全件、3 つの組とも直す対象として要件に落とす
2. The 本 spec shall 直さずに残す指摘を、次の 3 種類のどれかに当たり、かつ要件の討議で開発者が残すと裁定したものだけに限る
   - ⑴ 開発者の裁定に反する直し
   - ⑵ わざと置いた先送りの印（実在の理由を書いた説明つきで使われていないことを許している語彙の口、および `shortcut:`・`ponytail:` の注釈で、書いた限界が今の負荷を越えていないもの）
   - ⑶ 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
3. If 指摘が ⑶ に当たるかもしれないとき, then the 要件の討議 shall ukadoc で正典を確かめた結果（出どころと引用）を添えて議題にする
4. If 指摘の直しが開発者の裁定を覆すとき, then the 本 spec shall その指摘を直さず、議題にする
5. The 本 spec shall 直さずに残すと裁定された指摘ごとに、当たる種類と開発者の裁定を記録に残す
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごとに、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい
7. When 要件の討議を行うとき, the 要件の討議 shall `crates/shiori-abi/` と `crates/shiori4-testdll/` を受け持ちから外すこと（Introduction）を開発者に確かめる

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・テスト・fixture・設定を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・fixture・設定・台本・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない
5. When audit の指摘として clippy の指摘と同じ所を直したとき, the 本 spec shall その場所を記録に残す（`clippy-199-lints` が直し直さないため）

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
3. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
4. Where テストが i686 の helper や試験用の DLL を要するところでは, the 本 spec のテスト shall 全体テストが回す段（x64 の段か i686 の段）のどちらかで必ず回る形にし、成果物が無いときに黙って飛ばさない
5. When 既存のテストが直しで古びたとき, the 本 spec の直し shall そのテストを直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ。棚卸㊱の時点で `shiori-host32-host/src/client.rs` 957 行・`shiori3.rs` 879 行・`shiori-host32-helper/src/shiori_proxy.rs` 819 行が近い）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は `error!` の記録とエラーの戻り値の両方で表す）
3. The 本 spec の直し shall 終了の道（helper の終わらせ方・SHIORI の `unload`・中継の後始末）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 受け持つクレートを、今と同じターゲット（helper・試験用の DLL は i686、host は x64 と arm64、取り決めのクレートは両方）で組み上がる形に保つ
6. The 本 spec の直し shall 依存を足さず、`Cargo.lock` に手を入れない（C19 の約束 ⑷＝`Cargo.lock` の席は `dola-yaml-backend` だけ）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 6: 範囲の外へ直しが及ぶとき・C19 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）
3. While C19 のあいだ, the 本 spec の直し shall 本体 `crates/areka/` の全部と、C19 のほかの 7 本が触るファイル（ロードマップの C19 の行の「触るファイル」）に手を入れない（C19 の約束 ⑴）
4. While C19 のあいだ, the 本 spec の直し shall `shiori-host32-host` の公開の形（範囲の外から呼べる型・関数・欄の名前と形。`ExitKind`・`HelperStatus`・`RequestError`・`ShutdownError` を含む）を、足すことのほかは変えない（C19 の約束 ⑴）
5. If 確定した指摘が `shiori-host32-host` の公開の形を変えずには直せないとき, then the 要件の討議 shall その指摘を議題にし、`runtime-module-untangle` のセッションと申し合わせて C19 のうちに直すか、C19 のほかの 7 本が `main` に着地した後へ回すかを決める（C19 の約束 ⑴「③ のセッションと申し合わせるか C20 へ回す」）
6. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 7: helper が SHIORI の DLL を読むときの依存の探し先を絞る（件 1）

**Objective:** As a 開発者, I want 32bit の SHIORI の DLL を helper で読むときも、同じプロセス内で読むときと同じく依存の DLL の探し先が絞られていてほしい, so that ゴーストのフォルダの外に置かれた同じ名前の DLL を、意図せず読み込まない

#### Acceptance Criteria

1. When helper が SHIORI の DLL を読むとき, the helper shall その DLL が頼る DLL を、読む DLL の置かれたフォルダと OS の既定の安全な探し先からだけ探し、作業フォルダや `PATH` の並びからは探さない（同じプロセス内の読み込み `crates/areka-ghost/src/shiori_inproc.rs` と同じ探し方）
2. If SHIORI の DLL を読めなかったとき, then the helper shall 今と同じく、読めなかったことを記録とエラーの戻り値で返し、OS のエラーの窓を出さない
3. When 件 1 を直した後, the 全体テストが読む SHIORI の DLL（試験用の DLL 2 つを含む） shall 今と同じく読み込める
4. The 本 spec のテスト shall 7.1 の探し方を、探し先を絞らない形へ戻したら落ちるテストで縛る
5. If 探し先を絞ることで、今は読めている実物のゴースト（emo2 の `pasta.dll` など）が読めなくなるおそれが見つかったとき, then the 要件の討議 shall それを議題にする

### Requirement 8: helper の下の pasta がデバッグの待ち受けを取り合わない（件 2）

**Objective:** As a 開発者, I want 私の環境に `PASTA_DEBUG=1` が立っていても、areka が起こす helper の下の pasta が 127.0.0.1:9276 を取り合わないでほしい, so that SSP などで pasta のゴーストを動かしたままでも、areka の起動と実物の pasta のテストが LOAD の失敗で赤にならない

#### Acceptance Criteria

1. When areka（本番とテスト）が helper を起こすとき, the helper の下で動く pasta shall 親の環境に `PASTA_DEBUG=1` があっても、デバッグの待ち受け（127.0.0.1:9276）を開かない
2. The 本 spec の直し shall 件 2 を helper の起こし方の中で閉じて直し、`shiori-host32-host` の公開の形を変えない（要件 6.4。足す形の口を設けるときは要件 6.4 の「足すこと」に当たる）
3. When 要件の討議を行うとき, the 要件の討議 shall 開発者が helper の下の pasta のデバッグの待ち受けを使いたいときのために、それを開ける口を設けるかどうかを議題にする（brief「`PASTA_DEBUG=0` を渡すか、渡す口を設けるかを決める」）
4. When 要件の討議を行うとき, the 要件の討議 shall `audit-ghost-catalog` が当座しのぎに置いた、実物の pasta のテストを 1 本ずつにする錠（`crates/areka-ghost/tests/ghost/common.rs` の `real_pasta_turn`）を、件 2 の直しの後に残すか外すかを議題にする
5. The 本 spec のテスト shall 8.1 を、親に `PASTA_DEBUG=1` を立てても helper へ受け継がれないことで縛る（壊れたら落ちるテスト）

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘は数えない。打ち切りは 9.5）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`。x64 の段と i686 の段を含む） shall 緑で終わる
5. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（`audit-seriko`・`audit-dola` と同じ形）
