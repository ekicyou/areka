# Requirements Document

## Project Description (Input)

開発者は、areka のコードに残っているバグ・危ない所・負荷に弱い所・テストの抜け・遅い所・要らないコードを、一度まとめて直したい（2026-10-19 開発者「とにかく指摘事項は全部直す方針です。」）。今までは各 spec が自分の触る所だけを見てきて、全体を同じ物差しで見直したことはない。物差しは ponytail のプラグイン（起票のときは 5.3.2・着手のときに入っていたのは 5.3.3＝`ponytail-audit` の SKILL.md は 5.3.2 と同じ・利用者全体に入れてある）の `/ponytail:ponytail-audit`。全 33 本の見直しのうち、この spec は **性能改善ループの判定と台帳（Python）**（`tools/perf/` のうち PowerShell でないもの全部）を受け持つ。まだ audit をかけていない。指摘はギャップ分析で洗い出し、要件の討議で直す対象を確定し、設計で直し方を決め、実装で直す（開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」）。直した後に同じ範囲へもう一度 audit をかけて新しい指摘が 0 件であること、全体テストが緑であること、直したバグ・危ない所の 1 件ずつに壊れたら落ちるテストがあることを目指す。

## Introduction

この文書は、性能改善ループの判定と台帳の道具（`tools/perf/` のうち `.ps1` でないファイル全部）に `/ponytail:ponytail-audit` をかけて出た指摘を、3 つの組（必ず直す・直したほうがよい・できれば）とも全部直すための要件を定める。要件の段では「全部直す」という形だけを書き、指摘の中身は書かない。指摘の一覧はギャップ分析の報告に残し、要件の討議でこの文書に落として直す対象を確定する。

受け持つ範囲の大きさ（着手のときの測り直し・`main` `30a343eb`・git が追うファイルだけを数えた）:

| まとまり | ファイル | 中身 |
|---|---|---|
| `tools/perf/` の直下の Python | 11 | 判定（`judge-perf.py`・`judge_perf_catchup.py`・`judge-followup.py`）・順位表（`perf-rank.py`・`perf_rank_dump.py`）・比べ（`perf-compare.py`）・台帳（`perf-ledger.py`・`perf_ledger_goal.py`）・自己較正の中身（`*_selftest.py` の 3 つ） |
| `tools/perf/fixtures/` | 80 | 判定の検体と、検体を作る `generate.py` |
| `tools/perf/fixtures-loop/` | 190 | 順位表・比べ・台帳の検体（`judge-stub.py` の 5 つを含む） |
| `tools/perf/goals/` | 2 | 目標の定義（`.toml` と `.goal.md`） |
| `tools/perf/README.md` | 1 | 道具の説明 |
| 合計 | 284 | Python は 17 ファイル・11,471 行（うち自己較正の中身 3 ファイルが 701 行） |

brief の「コード 9,523 行・テスト 1,945 行」は Rust のテストの数え方（`_tests.rs`・`tests/`・`_test_support.rs`）で数えた起票のときの値で、この範囲の Python には当たらない。1,000 行を越えるファイルは着手のときに 2 本（`judge-perf.py` 4,159 行・`fixtures/generate.py` 1,244 行）。1,000 行に近いのは `perf-ledger.py`（996 行）・`perf-compare.py`（969 行）・`perf-rank.py`（957 行）・`judge-followup.py`（931 行）。1 ファイル 1,000 行の番人（`crates/log-capture-kit/tests/file_length_guard_test.rs`）は `.rs` だけを数えるので、この範囲のファイルは番人に載らない。Python の道具が使うのは標準ライブラリだけ（着手のときに import を数えた）。

## Boundary Context

- **In scope**:
  - `tools/perf/` のうち、git が追う `.ps1` でないファイル全部（上の表の 284 ファイル）に audit をかけて出た指摘の直し。
  - 直しのために要るテスト（各道具の自己較正 `--selftest` の中身と検体を含む）の追加と直し。
  - 直しのために要る、範囲の外の呼び出し元の直し（要件 5 と要件 6 の約束に従う）。
- **Out of scope**:
  - `tools/perf/*.ps1`（8 ファイル・同じウェーブ C20 の `audit-tools-perf-run` の持ち物）。
  - 範囲の外のファイルの見直し（ほかの 32 本の見直しの持ち物）。
  - 新しい機能。
  - `vendors/` の下。
  - clippy の指摘の機械的な直し（`clippy-199-lints` の持ち物。audit が同じ所を指したときは、その指摘として直してよい）。
  - 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）を覆す直し。直さずに議題にする（要件 3）。
- **Adjacent expectations**:
  - **PowerShell の道具（`audit-tools-perf-run` の範囲）が読む口**: `tools/perf/perf-loop.ps1` などは、この範囲の Python を引数と終了コード（0〜3）で呼び、`key=value` の出力の語を読み、自己較正を `perf-loop.ps1 selftest` の一覧（`--selftest`）から回す。`perf-loop.common.ps1` は `judge-perf.py` の `SCRIPT_VERSION` を、道具を起動せずに字面で読む。`judge-followup.py` の `CHECK_ALL` は `invoke-followup-checks.ps1` の `$CHECK_ALL` と対になる。逆向きに、この範囲の Python も PowerShell の道具が書く出力の語（`check-quiet.ps1` の判定語など）を読む。どの口も C20 の約束 ⑷ で、申し合わせなしに変えない（要件 6）。
  - **Rust の側が書く記録の行**: 判定と順位表の Python は、範囲の外の Rust が書く記録の行（`areka-emo-present` の性能の要約の行・`areka-placement` と wintf の `ecs/window/` の遷移の記録・本体 `crates/areka/` のスレッドの報告の行）を読む。Rust の側は `judge-perf.py` の `parse_fields` と同じ切り方（空白の直後の `名前=値`・同じ名前は後勝ち）をテストの中に写し、コメントで `judge-perf.py` の関数名や行番号を指している。C20 では `wintf-visual-damage-redraw` が性能の要約の行の形を変えない約束 ⑴ を持ち、`areka-placement` は `audit-placement-core`・`audit-placement-follow`、wintf の `ecs/window*` は `audit-wintf-window` が見直している。
  - **ほかに範囲のファイルを名で指す所**: スキル `.claude/skills/perf-loop-iteration/SKILL.md`・`doc/COMPAT_ARCHITECTURE.md`。未着手の `tick-gate-adoption`（`perf-compare.py`）と `talk-thread-count-growth`（`perf-rank.py`）の brief も、この範囲のファイルに触る見込みを書いている。

## 確定した直す対象（要件 1.5・要件の討議 2026-11-14）

指摘の中身はギャップ分析（`research.md` 3 節）が正本。番号はその番号（全 81 件）。議題 1〜9 は推奨案で確定した（2026-11-04 開発者「audit系は推奨案でよい」の常の承認による）。残すと裁定した指摘は 0 件。直さずに残してよい 3 種類のうち、⑵（わざと置いた先送りの印）と ⑶（SSP・ukadoc の正典に関わる振る舞い）に当たる指摘は 0 件（範囲は開発の道具で、`shortcut:`・`ponytail:` の印も無い）。

| 番号 | 組 | 一言 | 扱い |
|---|---|---|---|
| 1〜4 | 必ず直す | 一時フォルダが `%TEMP%`・発話の区切りの語彙が今の kanade と合わない・段③が他のプロセスを数える・TID の使い回しで段②が膨らむ | 直す（#4 は議題 9＝perf-rank の中だけで） |
| 5〜27 | 直したほうがよい | バグと危ない所（入力の穴・終了コード・台帳の食い違い・端末の日本語・README の例の置き場ほか） | 直す（#6 は議題 1・#24〜#26 は議題 3） |
| 28〜35 | 直したほうがよい | テストの抜け（壊れても緑のままの分岐） | 直す |
| 36〜41 | 直したほうがよい | 要らないコードと写し | 直す（#40 は議題 4） |
| 42〜81 | できれば | 小さなバグ・検体の重複と作り方・古い説明と行番号 | 直す（#55・#67 は議題 1・#68 は議題 5・#70 は議題 3・#74 は議題 2） |

議題の結論:

1. **perf-run の口に触る 3 件（#6・#55・#67・要件 6.3）**: `audit-tools-perf-run` のセッションと申し合わせて、C20 のうちに直す。どれも `.ps1` の側の直しは 0 行（#6・#55 は終了コード 1→3 で、`.ps1` は 3 を計測失敗と読むか 0 かどうかだけを見る・#67 の `--sampling-available true` は `.ps1` が渡さない）。2026-11-14 に申し合わせが成った（あちらの返事＝3 件とも差し支えない・#6 は「むしろ直したい所」＝今は想定外の例外が `perf-loop.measure.ps1:339` で判定 FAIL と記録され、3 にすると計測失敗で止まる・#55 は `perf-loop.ps1:284-286` と `:524` が 0 かどうかだけを見る・#67 は auto と false だけを渡す）。最後の行の字面だけが変わる #5・#24・#25・#26 は、知らせるだけにした（`.ps1` は judge 系の py の最後の行を自分の行に埋めるだけで解析しない＝あちらも確かめた）。
2. **範囲の外の古い行番号（#74）**: 範囲の中は、行番号をやめてファイル名と関数名・定数名で指す形に直す。`perf-loop.common.ps1:188`・`perf-loop.ps1:510` の 2 か所は `audit-tools-perf-run` が直す（2026-11-14 に引き受けてもらった・こちらの直しで py の行が動くので、あちらは着地の順を見て最後に合わせる）。`crates/areka/src/placement_tests/transition_judge.rs:22` は同じ C20 の `audit-placement-follow` の持ち物なので、あちらと申し合わせてから、どちらかが同じ形に直す（要件 5.2・5.4・行の数は変えない）。
3. **端末へ出る日本語（#24・#25・#26・#70）**: 道具の最後の行と、標準エラーへ出す失敗の文を ASCII にする。報告の本文（`judge-followup.py` の報告ほか）は日本語のまま残す。README §11 の「道具が自分で端末へ出す文は ASCII だけ」と、完了した `areka-P0-perf-tools-console-encoding` の「写した子の中身は日本語でよい」の両方に合うので、種類 ⑴ に当たらない。
4. **行の読み方の写し（#40）**: 判定式も較正値も持たない共通の兄弟モジュールへ寄せ、各道具がそれを読む。`perf-compare.py`・`perf-rank.py` の「判定スクリプトを import しない」は完了した `areka-P0-draw-load-parity` の設計の決め（要件でも開発者の裁定でもない）で、その理由（判定スクリプトを触ると 2 つの道具が同時に倒れる）は判定式を持たない共通モジュールには当たらない＝種類 ⑴ に当たらない。判定スクリプトそのものは、今までどおり import しない。
5. **H1b の検体（#68）**: H1b を消す。`generate.py` の説明が引く「task 3.2 の裁定」の記録（指定の外の遅い系列は合格を止めない）は、同じ分岐を通る H11 の説明と README へ移す＝裁定の中身は H11 で同じく固定されるので、種類 ⑴ に当たらない。
6. **検体の置き場（要件 7.1）**: 新しい検体の組み立ては `fixtures/generate.py` の兄弟の生成モジュールへ置き、`generate.py` は呼ぶだけにする。作り直してバイトが一致することを確かめる今の約束を保つ。手書きの検体は足さない。
7. **`README.md` の 1,178 行**: 1 ファイル 1,000 行の決まり（要件 7.1）は文書に当てない（番人は `.rs` だけを数える・この文書の表どおり）。README は直しの分だけ増減してよい。
8. **spec の大きさ（要件 8.1）**: 1 本のまま進める。タスク生成で 20 本を越えたら、「できれば」の組（#42〜#81）を同じ範囲の 2 本目の spec へ切る。
9. **`talk-thread-count-growth` との重なり（#4・#22・#63）**: どれもこの spec で `perf-rank.py` の中だけで直し、Rust（`crates/areka/src/perf_thread_report.rs` とそのテスト）には触らない。あちらの brief に、ここで直したことを一言残す（この spec の完了まで）。

申し合わせで受けた直し（`audit-tools-perf-run` の直しがこの範囲の文書に及ぶもの・2026-11-14 に受けた・要件 5 と 6 の逆向き）:

- あちらの指摘 16（wpaexporter の道を消す）: `goals/draw-load-parity.toml` の `[sampling]` の注釈の wpaexporter の行と README の説明を、この spec で直す。`[sampling]` の段と `backend` の鍵は残す（`perf_ledger_goal.py` の `GOAL_SCHEMA` が必須にしている）。
- あちらの指摘 24（`sampling.txt` が無いときの理由の語 `no_record`）: README の段③の理由の語の一覧に `no_record` を足す。`perf-rank.py` は理由の語を選ばないので直さない。

## Requirements

### Requirement 1: 指摘の洗い出し

**Objective:** As a 開発者, I want 受け持つ範囲の指摘が 1 件も漏れずに一覧になっていること, so that 直す対象を要件の討議で確定できる

#### Acceptance Criteria

1. When ギャップ分析を行う, the 本見直し shall `tools/perf/` のうち git が追う `.ps1` でないファイル全部だけを対象にして `/ponytail:ponytail-audit` をかける
2. The 本見直し shall audit の作法のうち「最大 20 件」を外し、指摘を件数の上限なしで全件出して、省いた件を残さない
3. The 本見直し shall audit の作法のうち「読まなかった所（Not checked）」を外し、範囲のファイル（検体を含む）を全部読んで、読まなかった所を残さない
4. The 本見直し shall 指摘の一覧を、audit の番号と 3 つの組（必ず直す・直したほうがよい・できれば）を付けたまま、ギャップ分析の報告に残す
5. When 要件の討議で直す対象が確定した, the 本見直し shall 確定した指摘の一覧（audit の番号・組・直すか裁定で残すか）をこの文書に書き加える

### Requirement 2: 指摘を全部直す

**Objective:** As a 開発者, I want 3 つの組の指摘が全部直っていること, so that 性能改善ループの判定と台帳の道具からバグ・危ない所・要らないコードが一度に無くなる

#### Acceptance Criteria

1. The 本見直し shall 「必ず直す」「直したほうがよい」「できれば」の 3 つの組とも、要件の討議で確定した指摘を全部直す
2. The 本見直し shall 要件 3 の 3 種類のどれかに当たって開発者が残すと裁定した指摘と、要件 6.3 で開発者が裁定した扱いの指摘のほかは、直さずに残さない
3. When 指摘の直しで消す・まとめる・形を変える箇所が出た, the 本見直し shall その箇所を使うテスト・検体・呼び出し元まで直し切り、直しの途中の状態を残さない

### Requirement 3: 直さずに残す指摘は開発者の裁定で決める

**Objective:** As a 開発者, I want 直さずに残す指摘を自分で裁定できること, so that 今までの決まりや互換を壊す直しが勝手に入らない

#### Acceptance Criteria

1. If 指摘の直しが次の 3 種類のどれかに当たる, the 本見直し shall その指摘を直さずに要件の討議の議題にし、開発者の裁定を仰ぐ
   1. 開発者の裁定（roadmap の「生きている決まり」・記憶の決めごと・完了した spec の要件）に反する直し
   2. わざと置いた先送りの印を消す直し（`#[allow(dead_code)]` に実在の理由の説明を付けた語彙の口、または `shortcut:`・`ponytail:` の注釈のうち、書かれた限界が今の負荷を越えていないもの）
   3. 外から見た振る舞いが変わる直しのうち、SSP との互換や ukadoc の正典に関わるもの
2. When 3 種類目に当たるかを判断する, the 本見直し shall ukadoc の該当箇所（URL と原文の引用）で確かめてから議題にする
3. If 指摘の直しが 3 種類のどれにも当たらない, the 本見直し shall その指摘を残すことを議題にせず直す
4. When 開発者が指摘を残すと裁定した, the 本見直し shall 裁定とその理由をこの文書に残し、要件 4 の「新しい指摘 0 件」の数から外す

### Requirement 4: 直した後の確かめ

**Objective:** As a 開発者, I want 直しが新しい問題を生んでいないことを確かめられること, so that 見直しが終わったと安心して言える

#### Acceptance Criteria

1. When 確定した指摘を全部直し終えた, the 本見直し shall 同じ範囲に、要件 1 と同じ作法（件数の上限なし・全部読む）で `/ponytail:ponytail-audit` をもう一度かける
2. If もう一度かけた audit が新しい指摘（直しが生んだ指摘を含む）を出した, the 本見直し shall その指摘も直し、新しい指摘が 0 件になるまで audit をかけ直す
3. If かけ直しが 5 回目になっても 0 件に収まらず、5 回目の指摘に「必ず直す」と本番の振る舞いに関わるものが 1 件も無い, the 本見直し shall 開発者に一言知らせたうえで、5 回目の指摘を直したところでかけ直しを打ち切り、検証の記録に「5 回目で打ち切った・残りは直し済み」と書く（2026-11-06 `audit-seriko` で開発者が承認した形）
4. The 本見直し shall かけ直した audit の結果（回ごとの組ごとの件数と、0 件または打ち切りの判定）を検証の記録に残す
5. When 見直しを終える, the 本見直し shall 全体テスト（`pwsh -NoProfile -File tools/test-all.ps1`）が緑であることを確かめる
6. When 見直しを終える, the 本見直し shall 道具の自己較正（`pwsh -NoProfile -File tools/perf/perf-loop.ps1 selftest`）が緑であることも確かめる（全体テストはこの範囲の Python の自己較正を回さないため）
7. The 本見直し shall 直したバグと危ない所の 1 件ずつに、その直しが壊れたら落ちるテストを 1 本以上、全体テストか道具の自己較正のどちらかで毎回回る所に置く
8. The 本見直し shall 足したテストと直したテストを、毎回同じ結果になる（決定論の）テストにする

### Requirement 5: 範囲の外へ直しが及ぶとき

**Objective:** As a 開発者, I want 範囲の外への直しが安全に、並んで走るほかの spec とぶつからずに入ること, so that ほかの道具やクレートが壊れず、並行作業が止まらない

#### Acceptance Criteria

1. When 直しで関数・引数・出力の形を変えた, the 本見直し shall ツリー全体（PowerShell の道具・スキル・文書・Rust のテストとコメントを含む）で呼び出し元と読み手を探して、全部直す
2. If 直しが及ぶ呼び出し元・読み手の範囲を受け持つ spec のセッションが走っている, the 本見直し shall そのセッションと申し合わせてから、その呼び出し元・読み手に触る
3. If 直しで、範囲の外の Rust が書く記録の行の読み方（読む行の形・読む名前・`parse_fields` と同じ切り方）が変わる, the 本見直し shall その直しを要件 5.1 と要件 5.2 に従う範囲の外への直しとして扱い、Rust の側の写しのテストと書き手を壊さない
4. When 範囲の外のコメントや文書が範囲のファイルを関数名や行番号で指していて、直しでそれがずれた, the 本見直し shall 要件 5.1 と要件 5.2 に従ってその指し先を直す
5. When コード・テスト・検体を消す, the 本見直し shall 消す前に、ツリー全体（テスト・検体・設定・文字列での参照を含む）で使われていないことを確かめる
6. The 本見直し shall 範囲の外のファイルの、直しに要らない箇所を直さない（範囲の外の指摘を見つけても、その範囲の spec の持ち物として残す）

### Requirement 6: C20 の並走の約束

**Objective:** As a 開発者, I want 見直しの直しが同じウェーブ C20 のほかの spec の邪魔をしないこと, so that 並走しても互いの作業を壊さない

#### Acceptance Criteria

1. While C20 のあいだ, the 本見直し shall `audit-tools-perf-run` と互いに読む口（引数・終了コード 0〜3・`key=value` の出力の語・`CHECK_ALL`・`SCRIPT_VERSION`）を、そのセッションとの申し合わせなしに変えない（C20 の約束 ⑷）
2. While C20 のあいだ, the 本見直し shall `tools/perf/*.ps1` に、`audit-tools-perf-run` のセッションとの申し合わせなしに手を入れない
3. If 確定した指摘が要件 6.1 の口を変えずには直せない, the 本見直し shall その指摘だけを直さずに議題にし、「`audit-tools-perf-run` のセッションと申し合わせて、両方の側を C20 のうちに直す」か「`audit-tools-perf-run` が着地した後に、この spec のうちで両方の側を直す」かを開発者の裁定で決める
4. The 本見直し shall 依存を足さず、`Cargo.lock` を変えず、Python の道具に標準ライブラリの外のパッケージを足さない（C20 の約束 ⑸＝`Cargo.lock` の席は `wintf-winmsg-executor-import`）
5. If 直しに新しい依存が要ると分かった, the 本見直し shall 依存を足さずに開発者へ議題として出す（足すなら `tech.md` への登記と開発者の承認が要る）
6. The 本見直し shall `vendors/` の下に手を入れない

### Requirement 7: 守る決まり

**Objective:** As a 開発者, I want 見直しの直しがプロジェクトの決まりを守ること, so that 直しが別の決まりを破らない

#### Acceptance Criteria

1. The 本見直し shall 1 ファイル 1,000 行の決まりを守り、直しで 1,000 行を越えるファイルを新しく生まず、着手のときにもう越えているファイル（`judge-perf.py`・`fixtures/generate.py`）の行を増やさず、足す行は新しいファイルへ置く
2. The 本見直し shall 記録の無い失敗の道を作らず、失敗はその理由を記録に残したうえで、道具ごとに決まった失敗の終了コードで呼び出し元へ返す
3. The 本見直し shall 終了の道を正規の実装にし、終了を早めるための小細工を入れない
4. The 本見直し shall 実機の根と一時フォルダを、ワークツリーの `target` の下に置く
5. Where GPU を使うテストを足すか直す, the 本見直し shall そのテストを実 GPU で回す
6. If 文字化けが起きた, the 本見直し shall コンソールの文字コードを変えて逃げず、読む側を直す

### Requirement 8: spec の大きさ

**Objective:** As a 開発者, I want 1 本の spec が大きくなりすぎないこと, so that 1 本ずつ確かめて着地できる

#### Acceptance Criteria

1. If 確定した指摘の直しが 20 タスクを越えそうになった, the 本見直し shall 要件の討議で、指摘の組ごとに同じ範囲の 2 本目の spec として切ることを議題にする
