# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい。今までは、各 spec が自分の触る所だけを見てきた。全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（brief の起票のときは 5.3.2・利用者全体に入れてある）の `/ponytail:ponytail-audit`。この spec は **`tools/` 直下の台本（全体テスト・配布物・リリースノート・検体の取り寄せほか）** を受け持つ（受け持つファイル＝`tools/` のうち `tools/perf/` の外のもの全部。まだ audit をかけていない）。

着手のときの測り直し（2026-11-16・`main` `5b3a361c`・brief と同じく `_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた）: 台本 `*.ps1` が 14 ファイル＝コード 4,213 行・テスト 0 行（自分の検査は各台本の中の `-SelfTest`）。内訳は `package.ps1` 754 行・`encoding-check.ps1` 539 行・`sample-fetch.ps1` 366 行・`load-flake.ps1` 357 行・`cursor-art.ps1` 339 行・`app-icon.ps1` 325 行・`crates-io.ps1` 325 行・`release-notes.ps1` 317 行・`mouse-input.ps1` 243 行・`sync-i686-helper.ps1` 174 行・`mcp-bridge-install.ps1` 156 行・`impl-watch-install.ps1` 151 行・`test-all.ps1` 97 行・`utf8-child.ps1` 70 行。ほかに文言のファイル `release-notes.strings.json`（11 行）と見本 `release-notes-fixture/` の 5 ファイル（109 行）。1,000 行の近くは無い。棚卸㊳の「15 の台本（4,213 行）」は、行数が `*.ps1` 14 本の和と一致する（15 本目は文言のファイルを数えたと読む）。物差しの版は、いま入っている版をギャップ分析の報告に書く（先例の `audit-tools-perf-run` は 5.3.3・audit の作法は同じ）。

直した後の姿: 受け持つ範囲に audit をかけて出た指摘が、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直っている（開発者「とにかく指摘事項は全部直す方針」）。直した後に同じ範囲へもう一度 audit をかけて、新しい指摘が 0 件（直しが生んだ指摘も直す）。全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑で、直したバグ・危ない所には、それが壊れたら落ちるテストが 1 本ずつある。あわせて、brief に預かった 2 件（件 1＝全体テストに性能改善ループの道具の自分の検査の段を足す・件 2＝`encoding check` が子の思わぬ失敗の表示を見ない）を片づける。

## Introduction

この spec は、`tools/` 直下の開発の道具（全体テスト `test-all.ps1`、配布物 `package.ps1`、リリースノート `release-notes.ps1`、crates.io の公開前の確認 `crates-io.ps1`、検体の取り寄せ `sample-fetch.ps1`、文字コードの判定 `encoding-check.ps1`、子の出力の読み方 `utf8-child.ps1`、隣の helper を揃える `sync-i686-helper.ps1`、負荷の再現 `load-flake.ps1`、机へ置く道具 `impl-watch-install.ps1`・`mcp-bridge-install.ps1`、アイコンとカーソルの絵の道具 `app-icon.ps1`・`cursor-art.ps1`、入力の道具 `mouse-input.ps1`）を ponytail の audit で見直し、出た指摘を全部直すためのものである。要件は「受け持つ範囲の指摘を、3 つの組とも全部直す」という形だけを決める。指摘の中身はここには書かない。指摘はギャップ分析で audit をかけて洗い出し、要件の討議で直す対象として確定する（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。

これらの道具は開発の道具で、areka の実行時のコードからは参照されない。代わりに、範囲の外の多くの読み手がこの範囲の道具を呼ぶ。リリースの workflow（`.github/workflows/release.yml`・`crates-io.yml`）、スキル（`kiro-complete`・`kiro-impl`・`release`・`update`・`kiro-watch-clear` ほか）、性能改善ループの道具（`tools/perf/` が `utf8-child.ps1` と `encoding check` の段に頼る）、テストの道具のクレート（`sample-ghost-kit` が取り寄せの段の置き場と名前に頼る）、本体のビルド（`crates/areka/build.rs` がアイコンの道具の作ったファイルを読む）、steering と手順書（`tech.md`・`structure.md`・`workflow.md`・`doc/` の下）である。そのため、範囲の外から見た「公開の形」は、PowerShell の関数の中身ではなく、台本の名前・引数・終了コード・端末へ出す結果の行（全体テストの段の名前と結果の一覧の行を含む）・書き出すファイルの名前と置き場と形・読み込んで使う関数の名前（`Invoke-Utf8Child`）・環境変数の名前（`AREKA_IMPL_WATCH_HOME`・`AREKA_MCP_HOME` ほか）である（以下「外への約束」）。

この spec はウェーブ C21 で、ほかの 4 本（`balloon-flip-side-rules`・`wintf-surface-auto-promotion`・`pilot-windows-rs-next`・`runtime-crate-split`）と並んで走る。並走の約束（ロードマップの「ウェーブ編成」の C21 の行の約束 ⑴〜⑸）を守る（要件 7）。

要件の討議で開発者が決めたことは、本文の中に「確定 2026-11-16」と書き、一覧を末尾の「要件の討議の確定」に置いた。

## Boundary Context

- **In scope**:
  - `tools/` のうち `tools/perf/` の外のもの全部（今は直下の `*.ps1` 14 本・`release-notes.strings.json`・`release-notes-fixture/` の 5 ファイル）に audit をかけて出た指摘の直し。
  - 直しのために要るテストの追加と直し（全体テストの段の追加を含む）。
  - brief に預かった件 1（全体テストに `perf-loop.ps1 selftest` の段を足す）と件 2（`encoding check` が子の思わぬ失敗の表示を見るか）。
  - 直しのために要る、範囲の外の呼び出し元と読み手の直し（要件 3.2 と要件 7 の約束に従う）。
- **Out of scope**:
  - 範囲の外のファイルの見直し（ほかの見直しの spec の持ち物）。範囲の外で見つけた指摘は、その範囲の spec の持ち物として残す（例: `tools/perf/` は完了した `audit-tools-perf-run`・`audit-tools-perf-judge` の範囲、`crates/sample-ghost-kit` は `audit-test-kits` の範囲）。
  - 新しい機能（新しい台本・新しいサブコマンドや引数・新しい配布物）。ただし、確定した指摘の直しとして、走る前に止める確かめや止まる道を足すこと、および件 1・件 2 の段と検査は新しい機能に数えない。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物）。ただし audit が同じ所を指したら、その audit の指摘としては直してよい。
  - 開発者の裁定（ロードマップの「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする。
- **Adjacent expectations**:
  - リリースの workflow（`release.yml`・`crates-io.yml`）は、`package.ps1`・`release-notes.ps1`・`crates-io.ps1`・`utf8-child.ps1` の引数・終了コード・出す行・作るファイル（zip と `.sha256` の名前と形・リリースノートのファイル）が今のまま続くことを前提にしている。workflow はタグの push でしか走らないので、手元ではこの前提の崩れに気付けない。
  - スキル（`kiro-complete`・`kiro-impl`・`release`・`update`・`kiro-watch-clear` ほか）と steering（`tech.md` の Testing・`workflow.md`）は、全体テストの呼び方（`-Format`・`-License`）・段の名前・結果の一覧の行・終了コードと、机へ置く道具の呼び方と結果の行が今のまま続くことを前提にしている。
  - 性能改善ループの道具（`tools/perf/`）は `utf8-child.ps1` の `Invoke-Utf8Child` と、`encoding check` の段がその道具の子の呼び方を確かめることを前提にしている。
  - テストの道具のクレート `sample-ghost-kit` は、取り寄せの段が書庫とハッシュのファイルを `<target>\nar-samples\fetched\` に置くことを前提にしている。本体のビルド（`crates/areka/build.rs`）は、`app-icon.ps1` が作ってコミットした `crates/areka/icon/icons.res` を読む。
  - 同じウェーブの `runtime-crate-split` は `tools/` に触らない。`tools/load-flake.ps1` の `-Binary` の選び肢は、この spec と申し合わせる（要件 7.5・7.6）。

## Requirements

### Requirement 1: 受け持つ範囲の指摘をもれなく洗い出す

**Objective:** As a 開発者, I want 受け持つ範囲の指摘を、省かず、読み残しなく一覧にしたい, so that 直す対象をもれなく決められる

#### Acceptance Criteria

1. When ギャップ分析を行うとき, the ギャップ分析 shall `tools/` のうち `tools/perf/` の外のもの全部（`*.ps1`・`release-notes.strings.json`・`release-notes-fixture/`）だけを対象にして `/ponytail:ponytail-audit` をかける
2. When audit をかける前, the ギャップ分析 shall その時点の `main` を取り込んだうえで、受け持つ範囲のファイルの数と行数（本番のコードとテストを分けて）と、使った物差しの版を測り直し、報告に書く
3. The ギャップ分析 shall audit の「最大 20 件」の上限を外し、出た指摘を全件、省かずに報告へ残す
4. The ギャップ分析 shall 受け持つ範囲のファイルを全部読み、「読まなかった所（Not checked）」を残さない
5. The ギャップ分析 shall 各指摘に通し番号（audit の番号をそのまま使う。範囲を分けて読んだときは、まとめた後の 3 つの組を通した番号。以後の段はこの番号で指す）を付け、3 つの組（必ず直す・直したほうがよい・できれば）のどれに当たるかと、指している場所を報告に書く
6. When audit の指摘を直すのに外への約束を変える必要があるとき, the ギャップ分析 shall その指摘にその旨と、範囲の外のどの読み手（workflow・スキル・steering・手順書・`tools/perf/`・Rust のクレート）に響くかを書く（要件 7.7 の議題の材料）
7. When audit の指摘を確かめるのに要件 8.1 の操作（実際の配布・公開・机への設置・負荷の走行・画面の操作ほか）が要るとき, the ギャップ分析 shall その指摘にその旨を書く（要件 8.3 の議題の材料）
8. The ギャップ分析 shall 件 1 と件 2 について、要件 5 を満たすのに要る事実（件 1 の段が要る前提と所要時間・昇格した机での振る舞い、件 2 の今の表示）を測って報告に書く

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
6. If 直す対象のタスクが 20 を越えそうなとき, then the 要件の討議 shall 指摘の組ごとに、同じ範囲の 2 本目の spec へ切り分けることを議題にしてよい（確定 2026-11-16: 1 本のまま。ギャップ分析の見立ては 15〜18 タスク）
7. The 本 spec shall ギャップ分析 5 節の指摘 1〜19 の全件を直す対象とし、直さずに残す指摘を 0 件とする（確定 2026-11-16・末尾の「要件の討議の確定」の議題 1）

### Requirement 3: 確定した指摘を全部直し切る

**Objective:** As a 開発者, I want 確定した指摘が 3 つの組とも、半端なく直っていてほしい, so that 受け持つ範囲のバグ・危ない所・要らないコードが残らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直す対象として確定した指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直す
2. When 指摘を直すとき, the 本 spec の直し shall その直しで壊れる呼び出し元・読み手・テスト・見本・設定・文書を、範囲の外も含めてツリー全体で探し、全部直し切る
3. When コードやファイルを消すとき, the 本 spec の直し shall 消す前に、ツリー全体（テスト・見本・設定・workflow・スキル・エージェント・手順書・steering・文字列での参照を含む）でそれが使われていないことを確かめる
4. The 本 spec の直し shall 直す対象として確定していない新しい機能を足さない（確定した指摘の直しとして、走る前に止める確かめや止まる道を足すことと、要件 5 の段と検査は新しい機能に数えない）

### Requirement 4: 直したバグと危ない所をテストで縛る

**Objective:** As a 開発者, I want 直したバグや危ない所が、また壊れたらテストで分かってほしい, so that 直しが後の変更で黙って戻らない

#### Acceptance Criteria

1. The 本 spec の直し shall 直したバグと危ない所の 1 件ごとに、それが壊れたら落ちるテストを 1 本ずつ持たせる
2. The 本 spec のテスト shall 全体テストの段（既にある道具の自分の検査の段か、この spec が足す段）から回る形にする（置き場＝各道具の `-SelfTest` か新しい自分の検査の台本か、と足す段の数は設計で決める）
3. The 本 spec のテスト shall 決定論で動き、回すたびに同じ結果になる
4. The 本 spec のテスト shall ネット・昇格・画面の操作・静かな机・開発者の机の本物の置き場（`AREKA_IMPL_WATCH_HOME`・`AREKA_MCP_HOME` ほか）・コミット済みの資源の書き替えに頼らずに回る（既にある段 `sample fetch` の取り寄せを除く）
5. Where テストが実機の根や一時フォルダを使うところでは, the 本 spec のテスト shall それをワークツリーの `target` の下に置く
6. Where テストが GPU を使うところでは, the 本 spec のテスト shall 実 GPU で回す
7. When 既存の自分の検査が直しで古びたとき, the 本 spec の直し shall その検査を直しに合わせて改める（直しの的を外して緑にしない）

### Requirement 5: brief に預かった 2 件を片づける

**Objective:** As a 開発者, I want 前の見直しから預かった 2 件が、持ち主のこの spec で片づいてほしい, so that 持ち主の無い宿題が残らない

#### Acceptance Criteria

1. The 全体テスト shall 性能改善ループの道具の自分の検査（`pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest`）を 1 つの段として回し、その合否を結果の一覧に載せ、赤なら終了コード 1 で終わる（件 1）
2. The 件 1 の段 shall 昇格していない机で、xperf・画面の操作・静かな机・ネットに頼らずに緑で終わる
3. While 全体テストを昇格した机で回しているあいだ, the 件 1 の段 shall 本物の xperf の採取をせず、昇格していない机と同じ合否で終わる（ギャップ分析 7 節の測り: `invoke-cpu-sample.ps1 -SelfTest` は `Get-ProbeResult -SkipCapture` だけを呼び、その形を自分の検査 `probe-skip-capture` が構文木で固定している＝カーネルの記録に触らない。本 spec はこの形を崩さない）
4. If 件 1 の段が要る前提（Python 3.11 以上ほか）が机に無いとき, then the 件 1 の段 shall 足りない前提の名前を書いた行を出して赤で終わり、黙って飛ばさない（確定 2026-11-16: Python の版の名指しは、`tools/perf/perf-loop.ps1` の `selftest` に `preflight` が持つ版の確かめを使い回させて根本の 1 か所で行う＝要件 7.2 の「直しのために要る変更」として `tools/perf` に触る。前提は `tech.md` の Testing に書く）
5. If `encoding check` の段が確かめのために呼んだ子の pwsh が思わぬ形で失敗したとき（0 でない終了コード・行の欠け・例外）, then the `encoding check` の段 shall どの確かめが何を受け取ったかを書いた行を出して赤で終わる（件 2。ギャップ分析の指摘 4 として直す対象に入った＝方針「指摘事項は全部直す」で決まる。その表示が出ることを確かめるテストを 1 本持たせる）
6. The 本 spec shall 件 1・件 2 を片づけたことを、預かった brief の件の番号で記録に残す

### Requirement 6: 直しの中でも守る決まり

**Objective:** As a 開発者, I want 直しがプロジェクトの決まりを崩さないでほしい, so that 見直しで新しい問題が生まれない

#### Acceptance Criteria

1. The 本 spec の直し shall どのファイルも 1,000 行の番人を越えさせない（足すなら新しいファイルへ）
2. The 本 spec の直し shall 記録の無い失敗の道を作らない（失敗は、理由を書いた行と 0 でない終了コードの両方で表し、失敗を成功の終了コードで終わらせない）
3. The 本 spec の直し shall 終了の道（立てた子のプロセスの終わらせ方・途中で止めたときの後始末を含む）を正規の実装にする（終了を省く小細工をしない）
4. The 本 spec の直し shall 文字化けをコンソールの文字コードの切り替えで逃げず、読む側を直す
5. The 本 spec の直し shall 道具が端末へ出す文を ASCII だけに保ち、出す量を増やすときも呼ぶセッションのトークンの負荷を抑える形（数行）にする
6. The 本 spec の直し shall 依存を足さない（C21 の約束 ⑷＝`Cargo.lock` の席は `runtime-crate-split` だけ。PowerShell のモジュール・Python のパッケージ・外の道具を新しく要るようにすることも含む）
7. If 直しが新しい依存なしには成り立たないとき, then the 本 spec shall 依存を足さずに、`tech.md` への登記の案を添えて開発者の承認を仰ぐ議題にする

### Requirement 7: 範囲の外へ直しが及ぶとき・C21 の並走の約束

**Objective:** As a 開発者, I want 範囲の外への直しが、呼ぶ側や並んで走るほかの spec とぶつからないでほしい, so that 並走しても互いの作業とリリースの道を壊さない

#### Acceptance Criteria

1. While 範囲の外のファイルを受け持つ spec のセッションが走っているあいだ, when 直しがそのファイルへ及ぶとき, the 本 spec shall そのセッションと申し合わせてから触る
2. The 本 spec の直し shall 範囲の外のファイルについて、直しのために要る変更のほかは手を入れない
3. While C21 のあいだ, the 本 spec の直し shall C21 のほかの 4 本が触るファイル（ロードマップの C21 の行の「触るファイル」の ①〜④。`Cargo.lock`・根の `Cargo.toml`・`crates/areka/Cargo.toml`・`crates/areka/src/runtime/` の全部・`crates/areka-runtime/`・`doc/COMPAT_ARCHITECTURE.md` を含む）に手を入れない（`tools/load-flake.ps1` の選び肢は 7.5・7.6。例外: `.kiro/steering/tech.md`・`structure.md` は `runtime-crate-split` と行を申し合わせ、重ならない自分の行だけを書く＝2026-11-16 の設計の討議で開発者が承認・7.1 の申し合わせとして記録に残す）
4. If 直しが C21 のほかの 4 本の触るファイルに及ぶ（その中の説明の文が道具の名前や引数を書いている場合を含む）とき, then the 本 spec shall そのファイルを触らずに、相手のセッションへ申し送り、申し送った件と相手を記録に残す
5. When `runtime-crate-split` が本 spec の実装の終わりより先に着地したとき, the 本 spec の直し shall `tools/load-flake.ps1` の `-Binary` に新しいクレートのテストを回す選び肢を足し、古くなった使い方の例（`-Filter runtime::frame`）を直す（C21 の約束 ⑴・⑸）
6. If 本 spec が `runtime-crate-split` より先に着地するとき, then the 本 spec shall 選び肢を足さずに、`runtime-crate-split` の完了で拾う件として記録に残す（C21 の約束 ⑴・⑸）
7. If 確定した指摘が外への約束を変えずには直せないとき, then the 本 spec shall 約束を黙って破って直さず、要件の討議でその指摘を議題にし、扱い（約束を変えない直し方・読み手を同じ変更で全部直す・後の spec へ回す）を開発者の裁定で決める（確定 2026-11-16: ギャップ分析で約束を変えるとした 10 件〔1・2・3・9・10・11・12・14・15・19〕は、読み手を同じ変更で全部直す。リリースの workflow に響く分は、8.3 の記録に「次のリリースで確かめる」と指摘の番号ごとに書く）
8. When 裁定に従って外への約束を変えるとき, the 本 spec の直し shall 同じ変更の中で、それを前提にしている範囲の外の読み手（Boundary Context の Adjacent expectations に挙げたもの）を全部直し切る（7.3・7.4 の約束の中で）
9. If 直しが pasta のリポジトリと同じ文で持つスキルの文を変えるとき, then the 本 spec shall 変えた文と理由を記録に残し、pasta の側で同じ文にそろえる件として「ぱすた開発」のセッションへ申し送る（確定 2026-11-16: `kiro-complete` の全体テストの段の一覧のような、リポジトリごとに違う部分の変更は申し送らない。両方で同じ文に持つ手順の文そのものを変えたときだけ申し送る）
10. If 指摘の直しを後の spec へ回すと裁定されたとき, then the 本 spec shall 受け持つ spec が実在することを確かめ、その spec の brief に件として書き足し、回した指摘と回し先を記録に残す
11. The 本 spec の直し shall `vendors/` の下に手を入れない

### Requirement 8: 副作用の大きい操作をウェーブの中で回さない

**Objective:** As a 並んで走るほかのセッションと開発者, I want この spec の作業のあいだに、外へ出る操作・共有の机を変える操作・重い走行・画面の操作が走らないでほしい, so that 公開物も共有の机もほかの仕事の負荷も汚れない

#### Acceptance Criteria

1. The 本 spec の作業 shall 直しを確かめるために、次の操作を回さない: リリースの workflow の起動とタグの push・crates.io への公開・GitHub Release の作成・開発者の机の本物の置き場への設置（`impl-watch-install.ps1`・`mcp-bridge-install.ps1` の本番の走り）・負荷の再現の走行（`load-flake.ps1` の本番の走り）・窓を出す配布物の起動確認（`package.ps1 -Check`）・入力の道具（`mouse-input.ps1`）による画面の操作・コミット済みの資源（`crates/areka/icon/`・`crates/areka/cursor/` ほか）を書き替える本番の走り
2. Where 操作が全体テストの段として既に回っているもの（`sample fetch` の取り寄せ・`crates.io pre-publish check (package only)` ほか）であるところでは, the 本 spec の作業 shall それを全体テストの中で回し、8.1 の禁止に数えない
3. If 直しが 8.1 の操作でしか確かめられないとき, then the 本 spec shall その直しを決定論のテスト（偽の置き場・偽の子のプロセス・`target` の下の写し）で筋の正しさまで確かめる形を先に探し、それでも足りない分は要件の討議で議題にし、扱いを開発者の裁定で決める（確定 2026-11-16: 決定論のテストで筋まで縛り、効き目は次に本当にその操作をしたとき〔次のリリース・次の設置ほか〕に確かめるものとして、検証の記録に指摘の番号ごとに書き残す＝`audit-tools-perf-run` の裁定と同じ形。指摘 6 は、ボタンの離しが `finally` に在ることを構文木で縛る。指摘 7 の、すぐ終わる偽の親と負荷の子 1 本を 1 秒未満だけ起こす試しは、8.1 の「負荷の再現の走行」に数えない）

### Requirement 9: 仕上げの確かめ

**Objective:** As a 開発者, I want 直し終わったことを、測り直した結果で確かめたい, so that 「全部直した」を報告だけでなく事実で受け取れる

#### Acceptance Criteria

1. When 直しを全部終えたとき, the 検証 shall 同じ範囲へもう一度 `/ponytail:ponytail-audit` を、要件 1 と同じ作法（上限なし・全部読む）でかける
2. When もう一度かけた audit が指摘を出したとき, the 本 spec の直し shall 直しが生んだ指摘も含めて直し、新しい指摘が 0 件になるまで繰り返す（要件 2 で直さずに残すと裁定された指摘と、要件 7.7・7.10 で後へ回すと裁定された指摘は数えない。打ち切りは 9.6）
3. The 検証 shall 新しい指摘が 0 件であることを、かけ直した audit の結果として記録に残す
4. When 直しを全部終えたとき, the 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`） shall 件 1 の段を含めて緑で終わる
5. When 直しを全部終えたとき, the 検証 shall 要件 7.3 の約束が守られている（7.3 のファイルに差分が無い）ことと、外への約束の変わった所が要件 7.7 の裁定の分だけであることを、既定のブランチとの差分で確かめて記録に残す
6. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と道具の振る舞い（終了コード・出す行と語・書き出すファイル・作る配布物）に関わるものが 1 件も無いとき, then the 本 spec shall 開発者に一言知らせたうえで、5 回目の指摘を直したところで打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書いて全体テストへ進む（2026-11-06 開発者の決め・audit-seriko と同じ形）

## 要件の討議の確定（2026-11-16・開発者「推奨どおり承認」）

要件の討議の議題 7 件を、全部推奨案で確定した（audit 系の議題は推奨案で承認してよい＝2026-11-04 開発者）。本文の各所の「確定 2026-11-16」はここを指す。ギャップ分析の前に仮置きにしていた問いのうち、件 1 の段の昇格した机での振る舞い（要件 5.3）と件 2 の検査を足すか（要件 5.5）は、ギャップ分析の測りと方針で解けた（要件 5.3・5.5 の本文）。

1. **直す対象と残す指摘（要件 2.2・2.7）**: ギャップ分析 5 節の指摘 1〜19（必ず直す 1〜3・直したほうがよい 4〜15・できれば 16〜19）を全部直す。直さずに残す指摘は 0 件。残す候補だった指摘 2（置き替えてから確かめる手の順）は、完了した `areka-P0-impl-watch-skills` の設計の役の説明（`design.md:108`・`:374`）に書かれた順で、開発者の発言を引いた裁定ではない＝要件 2.2 ⑴ に当たらないので直す。
2. **指摘 1 の「製品の変更」の見分け方**: 題名のスコープと同じ名前のフォルダが `.kiro/specs/` か `.kiro/specs/completed/` に在れば製品の変更とする（旧名の `areka-P0-` も読む）。見どころの `spec-name` の検査も同じ名前の一覧で行う。決まりの意図「スコープが spec 名なら製品」は変えず、2026-10-23 の改名に追いつくだけ。公開済みの v0.1.0 の詳しいリリースノートの作り直しは範囲の外（リリースの操作＝要件 8.1）。
3. **Python 3.11 の名指し（要件 5.4）**: `tools/perf/perf-loop.ps1` の `selftest` に、`preflight` が持つ版の確かめを使い回させる（根本の 1 か所。`tools/perf` には今ほかの spec が触っていない）。前提は `tech.md` の Testing に書く。
4. **外への約束を変える 10 件（要件 7.7）**: 1・2・3・9・10・11・12・14・15・19 は、読み手（スキル・手順書・`tech.md`・`release.yml` ほか＝ギャップ分析 4 節）を同じ変更の中で全部直す。リリースの workflow に響く分は、要件 8.3 の記録に「次のリリースで確かめる」と番号ごとに書く。
5. **要件 8.1 の操作でしか効き目を確かめられない直し（要件 8.3）**: 決定論のテストで筋まで縛り、効き目は次の本当の操作で確かめるものとして番号ごとに記録する。指摘 6 は離しが `finally` に在ることを構文木で縛る。指摘 7 の、すぐ終わる偽の親と負荷の子 1 本を 1 秒未満だけ起こす試しは、「負荷の再現の走行」に数えない。
6. **pasta と同じ文で持つスキル（要件 7.9）**: `kiro-complete` の全体テストの段の一覧のような、リポジトリごとに違う部分の変更は申し送らない。両方で同じ文に持つ手順の文そのものを変えたときだけ、「ぱすた開発」のセッションへ申し送る。
7. **分け方と `load-flake.ps1` の選び肢（要件 2.6・7.5・7.6）**: 1 本のまま（見立て 15〜18 タスク）。`-Binary` の選び肢はロードマップの C21 の約束 ⑴・⑸ のまま（`runtime-crate-split` が先に着地したらこの spec が足す・こちらが先なら申し送り）。着地の順が実装の途中で入れ替わり、どちらで拾うかが崩れたときだけ議題にする。
