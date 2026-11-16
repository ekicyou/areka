# ギャップ分析: audit-tools-scripts

> 2026-11-16 `kiro-validate-gap`（`/kiro-start` の中のサブエージェント）。範囲 `tools/` のうち `tools/perf/` の外のもの全部（直下の `*.ps1` 14 本・`release-notes.strings.json`・`release-notes-fixture/` の 5 ファイル）だけに `/ponytail:ponytail-audit` の作法をかけた。作法のうち 2 つは brief のとおり外した＝「最大 20 件」を外して全件を出す・「読まなかった所（Not checked）」を残さない（20 ファイル 4,333 行を全部、頭から終わりまで読んだ）。番号は audit の通し番号で、以後の段はこの番号で指す。形は完了した `audit-tools-perf-run` の research.md に合わせた。
> 回さなかったもの（要件 8.1 と依頼のとおり）: 本物の xperf・机への設置・`load-flake.ps1`・`package.ps1`（`-Check` を含む）・`mouse-input.ps1`・リリースの操作・`perf-loop.ps1 selftest`。測りは全部コードを読んで行った。回したのは `python --version`・`py -3 --version`・`$PSVersionTable`・昇格の有無の確かめと、`git` の読むだけのコマンドだけ。

## 1. 要約

- 範囲は開発の道具の台本 14 本（4,213 行）と文言のファイル 1 本（11 行）・見本 5 本（109 行）。テストの行は 0（自分の検査は各台本の中の `-SelfTest`・較正）。指摘は **19 件**＝必ず直す 3・直したほうがよい 12・できれば 4。
- いちばん重いのは **1**＝リリースノートの道具が、spec 名から `areka-P0-` を外した 2026-10-23 の改名に追いついていない。製品の変更の見分けと見どころの「spec 名」の検査が `areka-P0-` だけを見るので、v0.1.0 の詳しいリリースノートでは `feat`・`fix`・`perf` の 53 件のうち 36 件が「開発の内部の作業など」へ落ちており、v0.1.0 の後の 22 件は 1 件も製品の変更に入らない。次に **2**＝机へ置く道具 2 本が、置き換えた後に確かめるので、確かめが落ちると壊れた exe が残り、前の exe はその前の段で消えている（共有の机の全部のセッションに響く）。
- 外への約束を変える直しは 10 件（1・2・3・9・10・11・12・14・15・19）、変えずに守るよう気を付ける直しは 4 件（4・13・14・17）。要件 8.1 の操作でしか効き目を確かめられないものは 1 件（6）、一部だけそうなものが 10 件。
- 件 1 の前提は変わっていた。`audit-tools-perf-run` の直しで `invoke-cpu-sample.ps1 -SelfTest` は採取を飛ばす形（`Get-ProbeResult -SkipCapture`）になり、その形を自分の検査が構文木で固定している＝**昇格した机でも本物の 5 秒の採取をしない**。ほかのセッションがカーネルの記録を握っていても影響しない。要件 5.3 と 4.4 の例外の書き方は古い（12 節の 1）。
- 規模は M・リスクは中。タスクは 15〜18 の見立てで 20 に収まる（10 節）。

## 2. 測り直し（要件 1.2）

- 取り込んだ `main`: `5b3a361c`（このブランチの `HEAD~1`・`origin/main` と同じ）。`tools/` の差分は無い（`git diff --stat origin/main HEAD -- tools/` が 0 行）。
- 範囲: 台本 14 ファイル＝本番のコード 4,213 行・テスト 0 行（brief と同じ数え方＝`_tests.rs`・`tests/`・`_test_support.rs` をテストに数えた。台本の中の `-SelfTest` の部分はコードに数えた）。ほかに文言のファイル 1（11 行）と見本 5（109 行）。計 20 ファイル 4,333 行。

| ファイル | 行数 | 自分の検査 | 全体テストが回すか |
|---|---:|---|---|
| `package.ps1` | 754 | 無い | 回さない |
| `encoding-check.ps1` | 539 | 較正（本体の中） | 回す（`encoding check`） |
| `sample-fetch.ps1` | 366 | `-SelfTest`（8 判定） | 回す（`sample fetch`・`sample fetch check`） |
| `load-flake.ps1` | 357 | 無い | 回さない |
| `cursor-art.ps1` | 339 | `-SelfTest`（2 判定） | 回さない |
| `app-icon.ps1` | 325 | `-SelfTest`（5 判定） | 回さない |
| `crates-io.ps1` | 325 | 較正（本体の中） | 回す（`crates.io pre-publish check (package only)`） |
| `release-notes.ps1` | 317 | `-SelfTest` | 回す（`release notes check`） |
| `mouse-input.ps1` | 243 | 無い | 回さない |
| `sync-i686-helper.ps1` | 174 | `-SelfTest`（6 判定） | 回す（`i686 helper sync check`・最後の段） |
| `mcp-bridge-install.ps1` | 156 | 無い | 回さない |
| `impl-watch-install.ps1` | 151 | 無い | 回さない |
| `test-all.ps1` | 97 | — | （全体テストそのもの） |
| `utf8-child.ps1` | 70 | — | `encoding check` が筋を確かめる |
| `release-notes.strings.json` | 11 | — | `release notes check` が鍵を確かめる |
| `release-notes-fixture/`（5） | 109 | — | `release notes check` が読む |

- 棚卸㊳の「15 の台本（4,213 行）」は、行数が `*.ps1` 14 本の和と一致する（要件の読みのとおり、15 本目は文言のファイルを数えたものと読む）。1,000 行の番人の近くは無い（最大は `package.ps1` の 754）。
- 物差しの版: brief は ponytail 5.3.2 と書くが、今入っているのは **5.3.3**（`~/.claude/plugins/installed_plugins.json`・2026-11-06 更新・キャッシュには 5.3.2 も残る）。audit の作法（見る順・3 つの組・1 件 4 部・`shortcut:` の扱い）は 5.3.3 の `skills/ponytail-audit/SKILL.md` に従った（先例の `audit-tools-perf-run` と同じ）。
- 範囲の中の `shortcut:`・`ponytail:` の注釈: 1 件（`crates-io.ps1:273`「索引の置き場は 4 文字以上の名前の規則だけ」）。公開の一覧の 3 本はどれも 4 文字以上なので、書いた限界は今の負荷を越えていない＝作法どおり指摘にしない（要件 2.2 ⑵ に当たる、わざと置いた印）。
- この机: 昇格していない・pwsh 7.7.0・Python 3.13.17（`C:\Python313\python`・`py -3` も同じ）。

## 3. この道具が何をするか・想定した負荷

**何をするか**: 開発者 1 人の Windows の机で回す開発の道具。全体テスト（`test-all.ps1`）が取り寄せ・ビルド・テスト・道具の自分の検査を段に並べ、リリースでは CI（`release.yml`・`crates-io.yml`）が版の読み・リリースノート・配布物の zip・crates.io の公開前の確認を呼ぶ。机へ置く道具 2 本は、並んで走る全部のセッションが使う共有の exe（`AREKA_IMPL_WATCH_HOME`・`AREKA_MCP_HOME`）を置き直す。絵の道具 2 本はコミットする資源（`crates/areka/icon/`・`crates/areka/cursor/`）を作り直す。`load-flake.ps1` は CPU を空回しで埋めてテストを回し、`mouse-input.ps1` は画面へ本物の入力を送る。

**想定した負荷**: 1 台の机で、並んで走るセッションが最大 8 本。全体テストは 1 ワークツリーに 1 本ずつ（排他テストの机で順を取る）。共有の机の exe はそのセッション全部が同時に使う。リリースは数週に 1 回で、`release.yml` と `crates-io.yml` はタグの push でしか走らない（手元で壊れても気付けない）。

## 4. 範囲の外の読み手と外への約束（いま確かめた事実）

| 外への約束 | 読み手 | 確かめた所 |
|---|---|---|
| `test-all.ps1` の引数（`-Format`・`-License`）・段の名前・結果の一覧の行（`OK  `／`FAIL`／`SKIP  `・`==== results (commit …)`・`all steps green`・`red: N step(s)`）・終了コード | スキル `kiro-complete`（`SKILL.md:420-437`＝段の一覧。今も先頭の `sample fetch` と `sample fetch check` が抜けている）・`kiro-impl`（:69・:263）・`release`（`procedure.md:254`「約 19 分」）・`update`（:31・:135）・`tech.md:76-92`・`structure.md:19`・`workflow.md` | grep・読んだ |
| `package.ps1` の引数・終了コード 0〜3・`zip:`／`sha256:`／`version:` の行・`target/package/areka-{版}-{arch}.zip(.sha256)` の名前と形 | `release.yml:255-300`（`-Arch all`・`.sha256` の形を読む）・`release/procedure.md:276`（`-Check` を裏で回す）・`tech.md:101`・`crates/areka/icon/README.md` | 読んだ |
| `release-notes.ps1` の `-Check`／`-Notes`／`-Body`・`FAIL {鍵} line {行}` の形・出すファイル・分け方の決まり | `release.yml:91-185`・`release/procedure.md:129-134`・`:287`・`:417-418`・`doc/release-notes/README.md:104`・`:144`（「spec 名は `areka-P0-` で始まる」） | 読んだ |
| `crates-io.ps1` の `-Verify`／`-Pending`／`-Version`・標準出力は名前だけ・人が読む行は標準エラー | `crates-io.yml:135-185`・`release/procedure.md:60`・`:282`・`:359`・`:386`・`doc/crates-io-publish.md:77` | 読んだ |
| `utf8-child.ps1` の `Invoke-Utf8Child`（`Code`・`Out`・`ErrLines`） | 範囲の 7 本・`release.yml:56-57` | grep |
| `encoding check` の段が `tools/perf/perf-loop.common.ps1` を単独で読み込み、`Invoke-Child`／`Invoke-PwshChild` の 3 行の届き方と端末が 932 のままかを確かめる | `tools/perf/perf-loop.common.ps1:14`（単独で読み込める形を約束）・完了した `audit-tools-perf-run` の設計 | 読んだ |
| 取り寄せの段の置き場 `<target>\nar-samples\fetched\` と `nar-sample-path --fetch-list` の 5 列 | `crates/sample-ghost-kit/src/{fetched.rs,bin/nar-sample-path.rs,lib.rs}`・`tests/nar_sample_path_test.rs`・`vendors/sample_ghost/README.md` | grep |
| `app-icon.ps1` が作ってコミットする `crates/areka/icon/icons.res` | `crates/areka/build.rs` | grep |
| 机へ置く道具の出る行（`OK placed …`・`OK check: --help exit 0`・`FAIL place: …`・`KEEP …`・`OK clear: …`）と環境変数の名前 | `doc/impl-watch.md:110-172`・スキル `kiro-impl`・`kiro-complete`・`kiro-watch-clear`・`release`・`update`・`tech.md:120`・`structure.md:389`・`:476`・`crates/areka-impl-watch/src/error.rs`（`state file version mismatch` の文） | 読んだ |
| `load-flake.ps1` の `-Binary` の選び肢・`AREKA-CRASH` の行を写すこと | `crates/{areka,wintf,areka-placement}` の `crash_report_test_support.rs`・`test_crash_report.rs`・`runtime-crate-split` の brief・`tech.md` | grep |
| `mouse-input.ps1` の出る行と終了コード 0〜2 | `doc/pasta-real-run.md`・`roadmap.md` | grep |

## 5. 指摘の一覧（19 件）

旗の読み方（先例と同じ）:
- **(a) 外への約束**: 直すと、4 節の読み手が前提にしている形が変わるか。「変える」＝形が変わる・「守る」＝形は変えないが、壊さないよう直し方に気を付ける・「なし」。
- **(b) 8.1 の操作でしか**: 要件 8.1 の操作（設置・配布物の起動確認・負荷の走行・画面の操作・リリース）でしか効き目を確かめられないか。「はい」「一部」（筋は決定論のテストで縛れるが、効き目は本物の操作で）「いいえ」。
- **(c) 残す候補**: 要件 2.2 の ⑴〜⑶ に当たりうるか。候補を挙げるだけで、決めない。
- **(d) テストの置き場**: T1＝その台本の既にある `-SelfTest`・較正へ足す／T2＝その台本に新しく `-SelfTest` を持たせる（新しい段を全体テストに足す）／T3＝新しい自分の検査の台本（名前は設計で決める）。

### 必ず直す（Must fix）

**1. リリースノートの「製品の変更」の見分けと、見どころの「spec 名」の検査が、2026-10-23 の spec 名の改名から効いていない** （`release-notes.ps1:122`・`:153`・`:180-185`・`doc/release-notes/README.md:104`・`:144`・`release-notes-fixture/subjects.txt`）
- **何か**: `-Notes` は題名の「種類(スコープ): 文」のスコープが `SPEC_MARK`＝`areka-P0-` で始まるときだけ製品の変更（新しい機能・不具合の修正・性能の改善・そのほか）に分け、ほかは「開発の内部の作業など」に入れる。`-Check`・`-Body` の `spec-name` の検査も `areka-P0-` だけを探す。
- **困る場面**: spec 名は 2026-10-23 から接頭辞なし（`{module}-{feature}`・`kiro-spec-schema.md` の命名規則）。v0.0.3..v0.1.0 の本線の `feat`／`fix`／`perf` 53 件のうち 36 件（`feat(dola-yaml-backend)`・`fix(audit-shiori-host32)` ほか）は、公開済みの v0.1.0 の詳しいリリースノートで「開発の内部の作業など」に入っている。v0.1.0..HEAD の 47 件には `areka-P0-` が 1 件も無いので、次の版の詳しいリリースノートは製品の変更の見出しが空になり、見どころを選ぶ材料（`release/procedure.md:129`）にもならない。見どころに `balloon-flip-side-rules` のような新しい spec 名を書いても `spec-name` で落ちない。
- **直し方**: spec 名の見分けを「`areka-P0-` で始まる」から「`.kiro/specs/` か `.kiro/specs/completed/` に同じ名前のフォルダが在る（旧名の `areka-P0-` も読む）」に替える（道具 `read-wave.py`・`roadmap-overview.ps1` が両方の名前を読むのと同じ考え）。見どころの検査も同じ名前の一覧で探す。`README.md` の 2 行と見本（`subjects.txt`・`expected-*.md`）を新しい名前の題名を含む形に直す。どの題名を製品に数えるか（`audit-*` の道具の直しを含めるか）は 12 節の 4。
- **直さないと**: 次のリリースの詳しいリリースノートが、使う人に効く変更を全部「内部の作業」として出す（タグを打った後は動かせない）。
- **旗**: (a) 変える（詳しいリリースノートの分け方・`spec-name` の検査の範囲・`README.md` の決まりの文。読み手は `release.yml`〔呼ぶだけ・形は同じ〕・`release/procedure.md`・`doc/release-notes/README.md`・見本） ／ (b) 一部（分け方は見本で決定論に縛れる。本物の題名の一覧での効き目は、次のリリースか `-Notes` を手で回したとき〔`-Notes` は読むだけで 8.1 に当たらない〕） ／ (c) なし（決まりの意図＝「スコープが spec 名なら製品」は変えず、改名に追いつくだけ。見分け方の選び方だけ討議） ／ (d) T1（`-SelfTest` の見本に新しい名前の題名と旧名の題名を足す・spec のフォルダの一覧は見本のフォルダを渡せる形にする）

**2. 机へ置く道具 2 本が、置き換えた後で確かめるので、確かめが落ちると壊れた exe が残り、前の exe は既に消えている** （`impl-watch-install.ps1:100-138`・`mcp-bridge-install.ps1:96-155`・`doc/impl-watch.md:128-129`）
- **何か**: 手 4 で今の exe を `.old-<UTC>.exe` へ改名して新しいものを置き、手 5 で `.old*.exe` を消し、手 6 で置いたものを起こして確かめる（`--help`・標準入力を閉じて 15 秒）。
- **困る場面**: 起動すると落ちる版（依存の版上げで初期化が panic する・置き場の DLL が足りないなど）を置くと、手 6 が `FAIL check: …` で終了コード 1 になる。そのとき置き場には壊れた新しい exe が残り、前の exe は手 5 で消えている（誰も走らせていなければ）。`AREKA_IMPL_WATCH_HOME` の exe は並んで走る全部のセッションの机の貸し借りに使うので、全部のセッションの机の操作が止まる。手順書は「置いたものが動かなかった」とだけ書く（`doc/impl-watch.md:129`）。戻すには、前の版のソースから作り直すしかない。
- **直し方**: 確かめ（`--help`・標準入力を閉じた起動）を `.new.exe` に対して手 4 の前に行い、落ちたら `.new.exe` を消して前のものに触らずに終わる。古いものの掃除（手 5）は、置いて確かめが通った後だけにする。13 と一緒に 1 か所で直す。
- **直さないと**: 壊れた版を 1 度置くと、共有の机が全部のセッションで使えなくなり、前の版も残っていない。
- **旗**: (a) 変える（`FAIL check` の意味が「置いていない・前のものは元のまま」に変わる。出る行の順も変わりうる。読み手は `doc/impl-watch.md:110-172`・スキル `kiro-impl`・`kiro-complete`・`kiro-watch-clear`・`release`・`update`・`doc/pasta-real-run.md`） ／ (b) 一部（偽の置き場〔`target` の下〕と偽の作った exe で順を決定論に縛れる。本物の置き場での効き目は次の設置で） ／ (c) ⑴候補（手の順は完了した spec〔`areka-P0-impl-watch` ほか〕の設計に書かれた形。直しが裁定を覆すかを確かめる） ／ (d) T3 か T2（ビルドを飛ばして作った exe を渡す口が要る＝12 節の 6）

**3. `impl-watch-install.ps1` が、`status` の「版の不一致」でない失敗を、何も出さずに終了コード 0 で通す** （`impl-watch-install.ps1:140-151`）
- **何か**: 手 7 は `status` を呼び、標準エラーが `state file version mismatch` で始まるときだけ `clear` する。
- **困る場面**: `status` がほかの訳で落ちる（`state.json` が壊れて読めない・`state.lock` を長く握られている・置き場に書けない）と、`if` の両方の条件のうち片方だけが真になり、何の行も出さずに `exit 0` に落ちる。道具は「置けて確かめが通った」と読まれ、次に各セッションが机の操作をしたときに初めて失敗に気付く。決まり「記録の無い失敗の道を作らない」（要件 6.2・`tech.md`）に反する。
- **直し方**: `status` が 0 でなく、版の不一致でもないときは、その標準エラーの最後の行と `FAIL status: exit <n>` を出して終了コード 1 で終える。
- **直さないと**: 机が使えない状態を「入れ終えた」として残す。
- **旗**: (a) 変える（新しい `FAIL status: …` の行が増える。読み手は `doc/impl-watch.md` の表） ／ (b) 一部（偽の exe で決定論に縛れる。本物の置き場では次の設置で） ／ (c) なし ／ (d) 2 と同じ

### 直したほうがよい（Should fix）

**4. `encoding check` が、子の pwsh が思わぬ形で落ちたときに、どの確かめが何を受け取ったかを出さない（件 2）** （`encoding-check.ps1:359-430`・`:514-531`）
- **何か**: 窓の無い子（`-Cp932Child`）が 7 つの確かめをし、FAIL の文を `$bad` に溜めて最後にまとめて出す。親は子の標準出力の `FAIL ` の行と標準エラーの行を写し、FAIL が無いのに子が 0 でなければ `FAIL cp932 child: exit code <n>` を足す。
- **困る場面**: ⑴ `tools/perf` の確かめ（402〜425 行の `& { … }`）は `try` に包まれていない。`perf-loop.common.ps1` の読み込みが投げる（`tools/perf` の誰かが構文を壊した・`perf-tool-base.ps1` が見つからない）か `Get-ConsoleCodePages` の `Add-Type` が投げると、子は捕まえない例外で終わり、それまでに溜めた `FAIL` の行（較正の失敗など）は 1 つも出ない。⑵ 親に見えるのは `cp932 child stderr: …` の行と `FAIL cp932 child: exit code 1` だけで、どの確かめで落ちたかの名前が無い。⑶ 子は自分の端末を 932 に替えている（360 行）ので、子の .NET の標準エラーの書き手は 932 で書き、親は UTF-8 で読む＝地域語の例外の文やパスの日本語は `\uFFFD` になる（`Show-Ascii` で ASCII にはなるが、読めない）。例外の表示は色の符号（ESC）付きのこともある（`audit-tools-perf-run` の research.md の 7 で、同じ机の `Invoke-PwshChild` で確かめた形）。⑷ `production` と `perf calibration` の確かめは、子の終了コードが 0 でないとき、子の標準エラーを出さない。⑸ この表示を確かめるテストが無い。
- **直し方**: 子の確かめを 1 つずつ `try`/`catch` で包み、捕まえたら「確かめの名前・受け取った終了コード・最後の行（ASCII にしたもの）」の `FAIL` の行を `$bad` に足して続ける（溜めた行は必ず出す）。子の終了コードが 0 でない確かめでは、その子の標準エラーの最後の行を添える。わざと落ちる道を作り、その表示が出ることを較正の 1 本として持つ（12 節の 3）。端末の文字コードは親でも子でも新しく替えない。
- **直さないと**: `tools/perf` の変更が `encoding check` を赤にしたとき、理由が読めず、ほかの確かめの赤も隠れる。
- **旗**: (a) 守る（段の名前 `encoding check`・`PASS …`／`FAIL …` の形・端末を替えない約束と、`tools/perf` の 3 行の確かめそのものは変えない） ／ (b) いいえ ／ (c) なし ／ (d) T1（較正に「わざと落ちる子」の見本を足す）

**5. `package.ps1` は、途中で止めると後始末が走らない** （`package.ps1:160-213`・`:396-404`・`:650-721`）
- **何か**: 後始末（仮の名前の物を消す・自分の子を止める・差し替えた環境変数と `PATH` を戻す）は `Exit-Script` と最後の `Invoke-Cleanup` の 2 か所からだけ呼ぶ。台本全体を包む `try`/`finally` が無い。説明は「& で呼ばれても親に残さないよう、後始末で元へ戻す」（160 行）と約束する。
- **困る場面**: 開発者のシェルから `& ./tools/package.ps1` で回して、ビルドの段（数分）で Ctrl+C すると、`finally` が無いので後始末が飛び、そのシェルに `RUSTFLAGS=-C target-feature=+crt-static` が残り、`CARGO_ENCODED_RUSTFLAGS`・`CARGO_BUILD_RUSTFLAGS` は消えたまま、`PATH` は絞られたままになる。そのシェルで次に `cargo build` を回すと、`target/` の全部が `+crt-static` で作り直される（数分〜十数分）。`-Check` の `watchdog` の段で止めたときは、展開した木の `areka.exe` が残る（10 秒の自動終了つきなので自分で閉じる）。Ctrl+C では PowerShell は `finally` を回すが、`catch` と後始末の関数は回さない。
- **直し方**: 段を並べる本体を `try { … } finally { Invoke-Cleanup … }` で包む（`Invoke-Cleanup` を 2 度呼んでも害が無い形にする）。
- **直さないと**: 約束「親に残さない」が、止めたときにだけ破れる。
- **旗**: (a) なし ／ (b) 一部（`finally` の筋は、偽の段を持つ小さな子の pwsh を止めて決定論に縛れる。本物の止め方の効き目は手で止めたとき） ／ (c) なし ／ (d) T2（9 と同じ置き場）

**6. `mouse-input.ps1` は、押したボタンの離しを `catch` でしか送らない** （`mouse-input.ps1:174-242`）
- **何か**: 説明は「押しは例外のときも必ず離す」と書く。離しは `catch` の中で、`$pressed` が残っているときに送る。
- **困る場面**: `drag`（既定で 20 歩×20 ミリ秒＝約 0.4 秒）の最中に呼んだ側が Ctrl+C を送ると、PowerShell は `catch` を回さず（止めは例外として捕まえられない）、`finally` だけを回す。左ボタンが押されたまま残り、開発者の次のマウスの操作がドラッグになる（窓が動く・文字が選ばれる）。
- **直し方**: 離しを `finally` へ移す（`$pressed` が 0 でなければ送る）。
- **直さないと**: 画面の操作を途中で止めると、机の入力が押されたままになる。
- **旗**: (a) なし ／ (b) はい（本物の入力は 8.1 で回せない。決定論に縛れるのは「離しが `finally` に在る」という形まで〔構文木で見る・`invoke-cpu-sample.ps1` の probe-skip-capture と同じやり方〕） ／ (c) なし ／ (d) T2 か T3（構文木の検査）

**7. `load-flake.ps1` の負荷の子は、台本が強く止められると止まらずに残る** （`load-flake.ps1:258-276`・`:303-313`）
- **何か**: 負荷の子（`pwsh -Command 'while($true){}'`・既定で論理 CPU の 2 倍＝この机で 44 本）は `Start-Process -WindowStyle Hidden` で起こし、`finally` で番号を控えた子だけを止める。
- **困る場面**: 回している端末の窓を閉じる・背景の仕事として強く止める（プロセスを殺す）と `finally` は回らない。`Start-Process` の子は別の隠れた端末に居るので、親と一緒には止まらない。44 本の空回しが机の CPU を 100% に埋めたまま残り、次の静かな机の測りと全体テストが全部揺れる（開発者が気付くまで続く）。
- **直し方**: 負荷の子に親の番号を渡し、親が消えたら自分で終わる形にする（`$p = Get-Process -Id <親>; while (-not $p.HasExited) {}`・空回しはそのまま）。
- **直さないと**: 1 度の止め方の誤りで、机が負荷のまま残る。
- **旗**: (a) なし ／ (b) 一部（「親が消えたら終わる」は、すぐ終わる偽の親と負荷の子 1 本を 1 秒未満だけ起こして決定論に試せる。それが 8.1 の「負荷の再現の走行」に当たるかは 12 節の 9） ／ (c) なし ／ (d) T2（11 と同じ置き場）

**8. `sample-fetch.ps1` の「https:// だけを取り寄せる」決まりが、転送の先に効かない** （`sample-fetch.ps1:75-81`・`:185`）
- **何か**: 行の URL と検査の値のファイルの URL が `https://` で始まることは確かめるが、`curl.exe` には `--location`（転送に従う）だけを渡し、転送の先の形を縛らない。
- **困る場面**: 置き場の側が `http://` への転送を返すと、書庫と検査の値のファイルの両方を平文で取り寄せる。途中で両方を書き替えられると、SHA-256 は互いに合うので置き場に入り、検体として全体テストに読まれる。決まりの「https だけ」が最初の 1 歩にしか効かない。
- **直し方**: 本番の行では `curl` に `--proto =https --proto-redir =https` を足す（`-AnyScheme` の `-SelfTest` の行では足さない）。
- **直さないと**: 決まりが書いたとおりに効かない。
- **旗**: (a) なし ／ (b) 一部（本物の転送はネットでしか起きない。決定論に縛れるのは「本番の行で 2 つの旗を渡す」まで） ／ (c) なし ／ (d) T1

**9. `package.ps1` の判定（記録の判定・zip の中身の判定・PE の読み・拒否表・引数の確かめ）にテストが無い** （`package.ps1:253-341`・`:351-386`・`:487-558`・`:616-647`）
- **何か**: `Test-RunLog`（7 条件・自動終了より後の挨拶を数えない・初回のバルーンの置き場）・`Test-ZipContent`（8 項目）・`Read-PeInfo`（PE32／PE32+ の読み）・`Get-DeniedImports`・前提の確かめ（終了コード 3 の道）は、本物の配布物の組み立てか `-Check` でしか回らない。全体テストはこの台本を回さない。
- **困る場面**: `Test-RunLog` の行の位置の比べ（632〜636 行）を直し間違えると、自動終了の後の挨拶を「話し始めた」と数える誤りが戻る（完了した `alpha-package` の要件 3.3 の空振り）。それが見えるのは `-Check`（窓を出す・8.1 で回せない）か、CI の `-Arch all`（タグの push）のときだけで、どちらもこの spec の作業の中では回らない。
- **直し方**: 判定の関数を、ビルドの前で止まる `-SelfTest` から回す（記録の判定は作った記録の文字列で・zip の判定は `target` の下に組んだ小さな zip で・PE は小さな偽の PE で）。全体テストに段を 1 つ足す。
- **直さないと**: 配布物の判定の後戻りに、次のリリースまで気付けない。
- **旗**: (a) 変える（新しい引数 `-SelfTest`・全体テストの段が 1 つ増える。読み手は `kiro-complete` の段の一覧・`tech.md:76-92`） ／ (b) 一部（判定の筋は決定論に縛れる。本物の zip と本物の記録での効き目は次の `-Check` とリリースで） ／ (c) なし ／ (d) T2

**10. 机へ置く道具 2 本にテストが無い** （`impl-watch-install.ps1` 全体・`mcp-bridge-install.ps1` 全体）
- **何か**: 改名・戻し・古いものの掃除・確かめ・版の不一致の `clear` の分かれ道が 6〜8 あるが、どれも本物の置き場でしか回らない（8.1 で回せない）。
- **困る場面**: 2 と 3 の直しも、13 の寄せも、確かめずに入れることになる。手 4 の「2 つ目の改名に失敗したら古いものを元へ戻す」（`:111-116`）は、1 度も通った記録が無い。
- **直し方**: 置き場を引数で渡せる自分の検査を持たせ、偽の置き場（`target` の下）と偽の作った exe（小さな pwsh の `.cmd` か、作り済みの exe の写し）で分かれ道を回す。ビルドを飛ばす口が要る（12 節の 6）。全体テストに段を足す。
- **直さないと**: 共有の机の exe を置き替える道を、試さずに変え続ける。
- **旗**: (a) 変える（新しい引数か内部の口・全体テストの段。出る行と環境変数の名前は守る） ／ (b) 一部（本物の置き場では次の設置で） ／ (c) なし ／ (d) T2 か T3

**11. `load-flake.ps1` にテストが無い** （`load-flake.ps1:105-200`・`:209-218`）
- **何か**: 回の記録の読み（`Read-Round`＝`test result:` の行・赤の名前・`failures:` の一覧・待ちの打ち切り・`sample-ghost-kit:`・`AREKA-CRASH`）・`Get-Median`・cargo の引数の組み立て・`Stop-Tree` は、本物の負荷の走行（8.1 で回せない）でしか回らない。
- **困る場面**: 要件 7.5 で `-Binary` に新しいクレートの選び肢を足すと、その変更を確かめる道が 1 つも無い。`Read-Round` の読みを直し間違えると、赤の数え（負荷の下で許す赤の判断の材料・`tech.md:93`）が黙って狂う。
- **直し方**: 作った記録の文字列で `Read-Round`・`Get-Median`・引数の組み立てを回す `-SelfTest` を持たせ（負荷の子を起こさず cargo も呼ばない）、全体テストの段から回す。7 の「親が消えたら終わる」もここで縛る。
- **直さないと**: 負荷の再現の道具が、変えるたびに本物の走行でしか確かめられない。
- **旗**: (a) 変える（新しい引数 `-SelfTest`・全体テストの段） ／ (b) 一部（本物の負荷の下での効き目は次の走行で） ／ (c) なし ／ (d) T2

**12. 作ってコミットした資源が、元の絵とずれても気付かない** （`app-icon.ps1:259-319`・`cursor-art.ps1:300-334`・`test-all.ps1:61-80`）
- **何か**: `app-icon.ps1 -SelfTest` と `cursor-art.ps1 -SelfTest` は「2 回作って同じバイト」を見るが、作ったものをコミット済みの `app.ico`・`tray.ico`・`icons.res`・`hand.cur`・`grip.cur` とは比べない。全体テストはどちらの `-SelfTest` も回さない。
- **困る場面**: 誰かが `app-32.png` や `hand.svg` を直してコミットし、台本を回し忘れると、`crates/areka/build.rs` が読む `icons.res`（と .cur）は古い絵のまま出荷され、何も赤にならない。
- **直し方**: 両方の `-SelfTest` に「コミット済みの入力を `target` の下の写しで組んだ結果が、コミット済みの出力とバイトで同じ」の判定を足し、全体テストの段から回す（コミット済みの資源は書き替えない）。
- **直さないと**: 絵と出荷物のずれに、使う人が見るまで気付けない。
- **旗**: (a) 変える（全体テストの段が増える） ／ (b) いいえ（写しで組むので 8.1 の「資源を書き替える本番の走り」に当たらない） ／ (c) なし ／ (d) T1。所要時間（`cursor-art.ps1` の画素の繰り返しは PowerShell で回る）は 11 節の 3

**13. 机へ置く道具 2 本が、ほとんど同じ写し** （`impl-watch-install.ps1:46-130`・`mcp-bridge-install.ps1:43-126`）
- **何か**: 手 1〜5（環境変数の確かめ・cargo の JSON からの exe の道筋・`.new.exe` への写し・改名と戻し・古いものの掃除）は、名前と環境変数の名前のほかは 1 字違わない約 85 行の写し。違うのは手 6〜7 の確かめだけ。
- **困る場面**: 2 の直しを片方にだけ入れると、もう片方に同じ穴が残る。実際に、既に片方にだけ手 7 がある。
- **直し方**: 手 1〜5 と確かめの順（2）を 1 つの関数にし、2 本はその関数に名前・環境変数の名前・確かめの scriptblock を渡すだけにする（読み込む小さなファイルを 1 つ足すか、どちらかへ寄せる＝設計で決める）。
- **直さないと**: 同じ直しを 2 度書き、片方を忘れる。
- **旗**: (a) 守る（台本の名前・引数なし・出る行・環境変数の名前・終了コードを変えない） ／ (b) 一部（10 と同じ） ／ (c) なし ／ (d) 10 と同じ

**14. 版の読み方が 3 か所、版の形の正規表現が 2 か所に写してある** （`package.ps1:314-330`・`encoding-check.ps1:302`・`:325-335`・`release.yml:56-72`）
- **何か**: 「`cargo metadata --no-deps --locked` → 名前が `areka` の包みがちょうど 1 つ → 版」が 3 か所に、`^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$` が 2 か所に在る。
- **困る場面**: 版の正本の置き場や形の決まり（`+` の付記を受けないなど）を変えるとき、3 か所を同じに直す必要があり、`release.yml` の写しはタグの push でしか走らない（手元で外れに気付けない）。
- **直し方**: 読みを `utf8-child.ps1` の隣の 1 関数（例: 版を返す関数）にまとめ、3 か所はそれを呼ぶ。`encoding check` の規則 E の表（`RequiredCalls`）も合わせて直す。
- **直さないと**: 版の決まりの直しが 1 か所だけに入る。
- **旗**: (a) 変える（`release.yml` の段の本文・規則 E の表） と 守る（`Invoke-Utf8Child` の名前・`release.yml` の振る舞い・出る文） ／ (b) 一部（`release.yml` は次のリリースで） ／ (c) なし ／ (d) T1（`encoding check` の本物の確かめ〔`version=` の突き合わせ〕がそのまま縛る）

**15. `package.ps1` と `crates-io.ps1` は、例外の文（地域語）をそのまま端末へ出す** （`package.ps1:233-247`・`:317`・`:322`・`:449-450`・`crates-io.ps1:248`・`:278`・`:282`）
- **何か**: ほかの台本（`sample-fetch`・`mouse-input`・`load-flake`・`sync-i686-helper`・`encoding-check`）は例外の文を `\uXXXX` か `?` にしてから出す。この 2 本は `"$_"`・`$_.Exception.Message`・子の標準エラーをそのまま出す。
- **困る場面**: `Copy-Item` の写し元が無いとき、`package.ps1` は `FAIL x64 assemble (… s, パス 'C:\…' が存在しないため…)` の形で日本語の文を出す。Claude のツールや背景の仕事が出力を UTF-8 で読むと、932 の端末を通った字は化け、`release` のスキルが失敗の訳を読めない（決まり「道具が端末へ出す文は ASCII だけ」）。
- **直し方**: 2 本とも、出す前に ASCII へ直す（18 で寄せる関数を使う）。
- **直さないと**: 失敗の訳ほど読めない。
- **旗**: (a) 変える（失敗の行の中の字が `\uXXXX` になる。成功の行・形は同じ） ／ (b) 一部（`package.ps1` の段の失敗は本物の組み立てで。引数の確かめの道は 9 の `-SelfTest` で） ／ (c) なし ／ (d) T1・T2

### できれば（Nice to have）

**16. `crates-io.ps1 -Pending` の索引の読みに時間の上限が無い** （`crates-io.ps1:277`・`crates-io.yml` 全体）
- **何か**: `Invoke-WebRequest -Uri $url -SkipHttpErrorCheck`。`-TimeoutSec` の既定 0 は「上限なし」。`crates-io.yml` には `timeout-minutes` が 1 つも無い。
- **困る場面**: 索引の配信が応答を止めると、公開の仕事がランナーの上限（6 時間）まで待ち、`concurrency: crates-io` の席を握り続ける。
- **直し方**: `-TimeoutSec 60` を足す（1 語）。
- **直さないと**: まれな詰まりで、公開と次の走りが数時間止まる。
- **旗**: (a) なし ／ (b) はい（詰まりは本物のネットでしか起きない） ／ (c) なし ／ (d) なし（1 語の直しで、分岐を足さない）

**17. MSVC 以外の `link.exe` を `PATH` から外す 3 行が、3 か所に写してある** （`test-all.ps1:35-37`・`package.ps1:163-165`・`crates-io.ps1:305-307`）
- **何か**: 同じ `Where-Object` の 3 行が 3 本に在り、コメントで「test-all.ps1 と同じ」と互いを指す。
- **困る場面**: Git Bash の `link.exe` の見分け方を直すとき（例: 別の道具の `link.exe` が増えた）、1 か所だけ直すと `-Verify` か配布物の組み立てだけが落ちる。
- **直し方**: 1 つの関数にして 3 本から呼ぶ（置き場は 14 と同じ所でよい）。
- **直さないと**: 小さな写しが互いにずれる。
- **旗**: (a) 守る ／ (b) 一部（`package.ps1`・`-Verify` の道は本物の組み立てで） ／ (c) なし ／ (d) なし

**18. 字を ASCII へ直す関数が 4 つある** （`encoding-check.ps1:85-90`・`sample-fetch.ps1:40-44`・`app-icon.ps1:65`・`cursor-art.ps1:48`・ほかに `mouse-input.ps1:52`・`load-flake.ps1:103`・`release-notes.ps1:255`・`:293` の行の中の写し）
- **何か**: `Show-Ascii`（`\uXXXX`・長さで切る）と `ConvertTo-AsciiText`（`\uXXXX`・コメントで「Show-Ascii の考え」と書く写し）、`ConvertTo-Ascii`（`?`・2 本で 1 字違わない）。
- **困る場面**: 15 の直しで新しい呼び手が増えると、どれを使うかで出る字の形がそろわない。
- **直し方**: `\uXXXX` の 1 つ（`Show-Ascii` の形）にまとめ、`utf8-child.ps1` の隣に置いて読み込む。`?` で十分な所はそのままでもよい（設計で決める）。
- **直さないと**: 同じ仕事の関数が増え続ける。
- **旗**: (a) なし（出る字の形が `?` から `\uXXXX` に変わる所があれば「変える」＝読み手は無い） ／ (b) いいえ ／ (c) なし ／ (d) 既にある各 `-SelfTest`

**19. `package.ps1` は、`-Check` の無い `-CheckDir`・`-KeepExpanded`・`-SmokeExitMs` を黙って受ける** （`package.ps1:253-290`）
- **何か**: `-Check -Arch arm64` は終了コード 3 で断るが、`-Check` を付けずに `-KeepExpanded`・`-SmokeExitMs 5000`・`-CheckDir D:\x` を渡すと、何も言わずに起動確認をしない組み立てだけを行い、終了コード 0 で終わる。
- **困る場面**: `-Check` を打ち忘れた `-KeepExpanded` で十数分の組み立ての後に緑が出て、起動を確かめたつもりになる。
- **直し方**: 3 つのどれかが `-Check` なしで来たら、組み立ての前に終了コード 3 で 1 行を出して止める。
- **直さないと**: 打ち間違いが「確かめた」に見える。
- **旗**: (a) 変える（終了コード 3 の場合が増える。読み手の `release/procedure.md:276` は `-Check` を付けて呼ぶので響かない） ／ (b) いいえ（前提の確かめは組み立ての前なので、9 の `-SelfTest` か子の pwsh で決定論に試せる） ／ (c) なし ／ (d) T2

**読まなかった所（Not checked）**: なし（範囲の 20 ファイル 4,333 行を全部読んだ）。

**Verdict**: 1（リリースノートの分け方）と 2・3（共有の机の exe）を先に。
**Lean**: 13・14・17・18 で約 -110 行（写しの寄せ）・-0 依存。

## 6. 指摘の数と旗の集計

| 組 | 件数 | 番号 |
|---|---:|---|
| 必ず直す | 3 | 1〜3 |
| 直したほうがよい | 12 | 4〜15 |
| できれば | 4 | 16〜19 |
| 計 | 19 | |

| 旗 | 件数 | 番号 |
|---|---:|---|
| (a) 外への約束を変える | 10 | 1・2・3・9・10・11・12・14・15・19 |
| (a) 変えないよう守る | 4 | 4・13・14・17 |
| (b) 8.1 の操作でしか確かめられない | 1 | 6（16 は本物のネットでしか起きない詰まり＝8.1 の操作ではないが、決定論には縛れない） |
| (b) 一部だけ 8.1 の操作が要る | 10 | 1・2・3・5・7・8・9・10・11・14（ほかに 13・15・17 は 10・9 と同じ道） |
| (c) 残す候補（⑴ 完了した spec の設計・開発者の裁定） | 1 | 2（手の順） |
| (c) ⑵ わざと置いた先送りの印 | 0 | （範囲の唯一の `ponytail:` 注釈〔`crates-io.ps1:273`〕は限界を越えていないので指摘にしていない） |
| (c) ⑶ SSP・ukadoc の互換 | 0 | （開発の道具で、ゴーストやバルーンの振る舞いに関わらない） |

## 7. 件 1・件 2 の事実（要件 1.8）

### 件 1（全体テストに `perf-loop.ps1 selftest` の段を足す）

- **今の全体テスト**: `tools/perf` の自分の検査を回していない。触るのは段 `encoding check` だけ（`test-all.ps1:71`）。
- **`selftest` が回すもの**（`tools/perf/perf-loop.ps1:204-216`・`:353-385`）: Python の 5 本（`judge-perf.py`・`perf-rank.py`・`perf-compare.py`・`perf-ledger.py`・`judge-followup.py` の `--selftest`）、PowerShell の 6 本（`invoke-cpu-sample.ps1 -SelfTest`・`check-quiet.ps1 -SelfTest`・`invoke-followup-checks.ps1 -SelfTest`・`perf-loop.selftest.ps1`・`perf-loop.selftest2.ps1`・`invoke-perf-run.ps1 -SelfTest`）、最後に条件文の写しの一致（`goal-text-vs-goal-md`）。緑は `PERF-LOOP RESULT selftest code=0 dir=-`、1 つでも赤なら終了コード 4（`$EXIT_MEASURE_FAILED`）。全体テストの `Step` は 0 でない終了コードを FAIL に数えるので、段の合否はそのまま載り、全体テストは 1 で終わる（要件 5.1 は `test-all.ps1` に 1 行足すだけで満たせる）。
- **OPEN Q1（昇格した机・ほかのセッションがカーネルの記録を握っているとき）**: 前提が変わっている。`invoke-cpu-sample.ps1 -SelfTest` は `Get-ProbeResult -SkipCapture` だけを呼び（`invoke-cpu-sample.ps1:768-770`）、昇格と xperf の在り処を見て `available=… reason=…` を書くだけで、**xperf を起こさない**。しかも自分の検査の ⒜-7 `probe-skip-capture`（`:758-766`）が「`Invoke-SelfTestMode` の中の `Get-ProbeResult` の呼び出しは全部 `-SkipCapture` 付き」を構文木で固定している。`-Stop`／`-Cancel` の試し（`Test-FakeXperfModes`）は、子の `PATH` の先頭に置いた偽の `xperf.cmd`（`target\perf-loop-tmp\` の下）だけを呼び、`-Stop`／`-Cancel` は昇格を確かめない（`Test-Elevated` の呼び手は `Get-ProbeResult` だけ）。よって **昇格した机でも本物の採取は 0 秒・カーネルの記録には触らない**。ほかのセッションが記録を握っていても `selftest` の合否は変わらない（赤にならない）。昇格した机とそうでない机の違いは、`invoke-cpu-sample.ps1` の結果の行の `available=` の語だけで、合否は同じ（決定論）。注意: 偽の `xperf.cmd` が見つからないと `Get-XperfPath` は既定の位置の本物の xperf へ落ち、`-Cancel` の試しが本物の `xperf -stop` を呼びうる。今は子の `PATH` の先頭に偽物の部屋を置くので起きない（`tools/perf` の持ち物・13 節）。
- **OPEN Q2（要る前提）**: ⑴ pwsh 7（全体テストと同じ）。⑵ Python 3.11 以上（`perf-compare.py`・`perf-ledger.py` が `tomllib` を読む）。標準のライブラリだけで、外のパッケージは要らない。探し方は `python`・`python3`（WindowsApps の下でないもの）→ `py -3` → WindowsApps の別名の順（`perf-tool-base.ps1:150-164`）。⑶ 目標定義 `tools/perf/goals/draw-load-parity.toml`・`.goal.md`（リポジトリに在る）。⑷ 書くのはワークツリーの `target\perf-loop-tmp\` の下だけ（Python は `perf_selftest_common.py` の `WORK_ROOT`・PowerShell は `selftest-<GUID>`）で、終わったら消す。`%LOCALAPPDATA%` には書かない（出力先の根を決めるだけ）。⑸ ネット・昇格・画面の操作・静かな机・xperf は要らない。
- **足りないときの表示（要件 5.4）**: Python が無ければ `selftest python NG code=-1 | python not found (the 5 tools run on python)` の 1 行で赤（`perf-loop.ps1:346-350`）。**Python が 3.11 未満のときは、版を確かめない**（版の確かめは `preflight` だけ＝`:430-436`）。`perf-compare.py`・`perf-ledger.py`・条件文の写しの一致が `ModuleNotFoundError: No module named 'tomllib'` の行で NG になり、赤で終わる＝黙っては飛ばさないが、「Python 3.11 以上が要る」とは言わない。WindowsApps の別名しか無い机では、別名の案内の文か終了コードで NG になる。
- **所要時間**: 台本の頭書は「約 1 分」（`perf-loop.ps1:19`）。この分析では回していない（依頼の「コードを読んで測る」に従った）＝設計の段で 1 度回して測る（11 節の 1）。
- **全体テストのほかの読み手**: 段を足すと、`test-all.ps1` の頭書の一覧（:5-22）・`tech.md:76-92`・`kiro-complete/SKILL.md:437` の段の一覧（今も `sample fetch`・`sample fetch check` が抜けている）・`release/procedure.md:254` と `update/SKILL.md:135` の「約 19 分」が古くなる。

### 件 2（`encoding check` が子の思わぬ失敗の表示を見ない）

- 今の表示は 5 の 4 に書いたとおり。子が捕まえない例外で終わったときに親に見えるのは `cp932 child stderr: <\uXXXX にした行>` の並びと `FAIL cp932 child: exit code <n>` だけで、どの確かめで落ちたか・それまでの `FAIL` の行は出ない。子の標準エラーは 932 で書かれて UTF-8 で読まれる（地域語の字は `\uFFFD`）。子の確かめのうち `production`・`perf calibration` は、子の 0 でない終了コードだけを出し、子の標準エラーを添えない。`perf real` は終了コードと受け取った字（`\uXXXX`）を出す。親の `Invoke-Utf8Child` が起こせないときは `FAIL cp932 child: cannot start: …` を出す。行の欠け（`version=` が無い）は、ほかの FAIL が無いときだけ `expected one version= line, got 0` を出す。
- この表示を確かめるテストは無い。足すには、子の中で 1 つの確かめをわざと落とす道（内部の引数か、読み込む先を差し替える口）が要る（12 節の 3）。

## 8. 要件 8.1 の 2 つの読みの確かめ（説明からだけ取った前提）

- **机へ置く道具は共有の机の exe を置き替えるか**: **置き替える**。`impl-watch-install.ps1`・`mcp-bridge-install.ps1` は、開発者の環境変数 `AREKA_IMPL_WATCH_HOME`・`AREKA_MCP_HOME` の置き場（ワークツリーの外・並んで走る全部のセッションが使う）の `<名前>.exe` を、走っていても改名で入れ替え、古い写しを消す（`:100-130`）。さらに `impl-watch-install.ps1` は、置いた exe の `status` が版の不一致を返すと `clear` を呼び、**全部のセッションの机の記録（参加者・机・待ち）を退避して消す**（`:140-150`）。どちらも先に `cargo build --release` を回す（重い）。8.1 の禁止は正しい。
- **`package.ps1 -Check` は窓を出すか**: **出す**。`-Check` は x64 の zip を `target\package\check-<HHmmss>` へ展開し、`areka.exe`（GUI の作り）を `Start-Process` で起こす（`:664-678`）。記録の判定は「本物のゴースト窓を開きました」の行を数える（`:625-626`）＝本物のゴーストの窓が机に出る。10 秒の自動終了と 60 秒の番犬つき。前に x64 の release の組み立てと `cargo deny`・`cargo about` が回る。`tech.md:101` も「窓を出すので CI では回さない」と書く。8.1 の禁止は正しい。
- **補足**: `-Check` を付けない `package.ps1` は窓を出さないが、release の組み立て（数分〜十数分）と `cargo deny`・`cargo about` を回し、`target/package/` にだけ書く。8.1 の一覧には無いが重い走りで、全体テストの段にもしていない。9 の `-SelfTest` は組み立ての前で止まる形にすれば、この重さを持ち込まない。

## 9. 要件と今の資産の対応

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1 洗い出し | この文書（19 件・全部読んだ） | 済み |
| 2 対象の確定 | 6 節の (c) の候補 1 件（2） | 要件の討議で裁定（Constraint） |
| 3 直し切る | 範囲の 20 ファイル＋4 節の読み手 | 1・2・3・9〜12・14 は範囲の外の文書・スキル・`release.yml` まで直しが及ぶ（Constraint） |
| 4 テストで縛る | `-SelfTest`・較正を持つ 7 本（うち全体テストが回すのは 5 本） | `package`・`load-flake`・`mouse-input`・机へ置く 2 本にテストの置き場が無い（Missing）。`app-icon`・`cursor-art` の `-SelfTest` は全体テストに無い（Missing） |
| 5 件 1・件 2 | `perf-loop.ps1 selftest`・`encoding-check.ps1` の較正 | 件 1 は 1 行で足せる。5.3 の前提が古い（Constraint：要件の書き直し）。5.4 の版の名指しは無い（Missing）。件 2 は 4 の直し（Missing） |
| 6 守る決まり | 各台本の ASCII の出し方・`Invoke-Utf8Child` | 15 は 6.5 の形に寄せる直し。依存は足さない（どの直しも PowerShell と .NET の標準の機能で足りる） |
| 7 範囲の外・C21 | 4 節の読み手・C21 の約束 ⑴〜⑸ | 7.3 のファイル（`Cargo.lock`・`crates/areka/src/runtime/` ほか）に触る指摘は 0。`load-flake.ps1` の選び肢（7.5・7.6）は `runtime-crate-split` の着地の順しだい（今は未着地＝`.kiro/specs/runtime-crate-split` が completed に無い）。`kiro-complete` は pasta と同じ文で持つスキル（7.9 の申し送りの候補） |
| 8 回さない操作 | — | 6 は 8.1 の操作でしか効き目を確かめられない。一部だけのものが 10 件（扱いは裁定） |
| 9 仕上げ | `test-all.ps1` | 件 1 の段を足せば 9.4 を回せる（昇格は要らない） |

## 10. 直し方の選択肢・規模とタスクの見立て

### 案 A: 今のファイルの中で直す（テストも各台本の `-SelfTest` へ）
- 1・3〜8・12・15・16・19 は、今の関数を数行ずつ直すだけで済む。9・11 は各台本に `-SelfTest` を足す。
- ✅ ファイルが増えない。1,000 行の番人は余裕がある（最大の `package.ps1` 754 行に `-SelfTest` を足して 900 前後の見込み）。❌ 13・14・17・18 の写しは寄せ先が要る（どれかの台本の中に置くと、ほかの台本がそれを読み込むことになり、読み込みの向きがねじれる）。

### 案 B: 共通の部品を 1 つ足し、写しをそこへ寄せる
- `utf8-child.ps1` の隣に、版の読み（14）・`PATH` の手当て（17）・ASCII へ直す関数（18）・机へ置く手 1〜5（13）を置く小さなファイルを足す（1 本か 2 本・名前は設計で決める）。テストは各台本の `-SelfTest` と、部品の自分の検査。
- ✅ 写しが 1 か所になり、2 の直しが 1 度で済む。❌ ファイルが 1〜2 本増え、`encoding check` の規則 A〜E と規則 E の表の対象になる。`release.yml` が新しいファイルを読み込む形になる（次のリリースでしか確かめられない）。

### 案 C: 組み合わせ（推しではなく、比べる材料）
- 必ず直す 3 件と 4〜8・15・16・19 は今のファイルの中で直す。写しの寄せは、机へ置く 2 本（13・2・3・10）だけを新しい 1 ファイルで受け、14・17・18 は `utf8-child.ps1` へ足す（`release.yml` は既に `utf8-child.ps1` を読み込んでいるので、新しい読み込みを足さずに済む）。
- ✅ `release.yml` の変わりが小さい。❌ `utf8-child.ps1` が「子の読み方」より広い仕事を持つ（名前と仕事がずれる）。

### 規模とリスク
- **規模: M（3〜7 日）**。1 件ずつは数行〜数十行。テストの置き場を作る 9・10・11 と、写しの寄せ（13・14）が手間の中心。
- **リスク: 中**。`release.yml`・`crates-io.yml`・机への設置・`-Check` は手元で回せず、効き目の確かめが次のリリースと次の設置に残る。1 の分け方の決まりは、使う人が読む文書の中身を変える。

| まとまり | 指摘 | タスクの見立て |
|---|---|---:|
| リリースノートの分け方と spec 名の検査（見本・README を含む） | 1 | 2 |
| 机へ置く道具（寄せ・順・status・テスト） | 2・3・10・13 | 3 |
| `encoding check` の子の失敗の表示（件 2） | 4 | 1 |
| `package.ps1`（後始末・判定のテスト・引数・ASCII） | 5・9・15（半分）・19 | 2 |
| `load-flake.ps1`（負荷の子・テスト・7.5 の選び肢） | 7・11 | 1〜2 |
| 小さな直し（入力の離し・転送の決まり・索引の上限・`crates-io` の ASCII） | 6・8・15（半分）・16 | 1 |
| 写しの寄せ（版・`PATH`・ASCII） | 14・17・18 | 1〜2 |
| 絵の資源のずれの検査と全体テストの段 | 12 | 1 |
| 件 1（全体テストの段・文書の追随） | — | 1 |
| 検証（audit のかけ直し・全体テスト・既定のブランチとの差分の確かめ・8.3 の記録） | — | 2〜3 |
| 計 | 19 件 | **15〜18** |

- **分け方の案**（要件 2.6）: 1 本のまま（20 に収まる見込み）。分けるなら「できれば」の 4 件と写しの寄せ（14・17・18）を 2 本目にできるが、15 と 9 が同じ関数を使うので、2 本目は 1 本目の着地を待つことになる。

## 11. 設計へ持ち越す調べもの（Research Needed）

1. 件 1: `perf-loop.ps1 selftest` の実際の所要時間（頭書は約 1 分）と、全体テストの中での順の置き場（自分の検査の段の並びのどこか）。この机で 1 度回して測る（昇格は要らない）。
2. 4（件 2）: 子の標準エラーが 932 で書かれる道を、端末を替えずに読めるようにできるか（子の中で例外の文を `Show-Ascii` にして標準出力の `FAIL` の行へ書けば、読む側の字の解き方に依らない）。わざと落ちる子の作り方（内部の引数・読み込む先の差し替え）。
3. 12: `cursor-art.ps1` を全体テストで回すときの所要時間（10 寸法×2 枚の画素の繰り返しが PowerShell で回る）。長ければ、比べを 1 寸法に絞るか。
4. 1: spec 名の見分けを `.kiro/specs/` のフォルダで行うとき、`release.yml` の取り出し（タグのコミットの木）に `.kiro/specs/` が在るか（在る＝`actions/checkout` は木全体を取る）。`-Notes` を古い範囲に回したとき、後で改名された spec を落とさないか（`completed/` は改名しない決まり）。
5. 2・10: ビルドを飛ばして作った exe を渡す口を、新しい引数にせず内部の口（`encoding-check.ps1` の `-Cp932Child` と同じ形）にできるか。偽の exe（`--help` で 0・標準入力の終わりで 0 を返す小さなもの）を何で作るか（`pwsh` の `.cmd` か、`target` の下の作り済みの exe の写し）。
6. 7: 「親が消えたら終わる」負荷の子を、`load-flake.ps1` の止め方（`finally` での番号の止め）と並べても二重に止めないか。

## 12. 要件の討議で決めること（設計の判断の項目）

1. **要件 5.3 の前提が古い**: `invoke-cpu-sample.ps1 -SelfTest` は昇格した机でも本物の採取をしない（`-SkipCapture` を構文木で固定済み）。5.3 を「昇格した机でも本物の採取をしない」に書き直し、4.4 の例外から「件 1 の段の昇格した机での振る舞い」を外すか。ほかのセッションの採取中の扱いは、決める必要が無くなる。
2. **要件 5.4 の「足りない前提の名前」**: Python 3.11 未満では、今は `ModuleNotFoundError: No module named 'tomllib'` で赤になり、版を名指ししない。⑴ 全体テストの側（範囲の中）で、段の前に Python の版を確かめて名指しの 1 行を出す ⑵ `perf-loop.ps1 selftest` に版の確かめを足す（`tools/perf` は完了した spec の持ち物＝範囲の外の直し・要件 7.2）⑶ 今の赤（tomllib の名前は出る）で足りるとする、のどれか。前提を `tech.md` の Testing に書くことは仮置きのまま。
3. **件 2 の検査を足すか（要件 5.5）**: 仮置きは足す。足すなら、わざと落ちる子を作るための内部の口が要る（新しい機能に数えないことを確かめる）。
4. **1 の見分け方**: 製品の変更を「スコープが `.kiro/specs/`（`completed/` を含む）に在る spec 名」とするか、「スコープが付いた `feat`／`fix`／`perf` 全部」とするか。前者なら `audit-*`（道具の見直しも含む）の `fix` が「不具合の修正」に入る。見どころの `spec-name` の検査を同じ名前の一覧で行うか。公開済みの v0.1.0 の詳しいリリースノート（36 件が内部の作業の見出しにある）を作り直すかは、この spec の範囲の外（リリースの操作＝8.1）。
5. **2 の手の順の入れ替え**: 確かめを置き替えの前に行う直しが、完了した spec（`areka-P0-impl-watch` ほか）の設計の手の順に反するか（要件 2.2 ⑴ の候補）。
   - 要件の討議での追記（2026-11-16）: 手の順の出どころは `completed/areka-P0-impl-watch-skills/design.md:108`・`:374` の役の説明（ビルド → 置く・入れ替える → 後始末 → 動くかの確かめ）で、開発者の発言を引いた裁定ではない。同じ設計の `:24` は「走っている実行ファイルの入れ替えは偽の実行ファイルで再現できず、本物のビルドを全体テストへ入れると重い」として `-SelfTest` を範囲外にした＝10 のテストは、走っている exe の入れ替えそのものではなく、分かれ道の筋（確かめが落ちたら置かない・掃除は通った後だけ・`status` の失敗の行）を偽の置き場で縛る形にする（設計で決める）。
6. **テストの置き場（要件 4.2）**: 9・10・11 を各台本の `-SelfTest`（新しい引数）にするか、新しい自分の検査の台本（T3）にするか。全体テストに段を足す数（9・10・11・12 と件 1 で 4〜6 段・全体テストの一覧と `kiro-complete` の段の一覧が伸びる）。
7. **写しの寄せ先（10 節の案 A・B・C）**: 新しい共通のファイルを足すか、`utf8-child.ps1` へ足すか。`release.yml` の版の読みを寄せる（次のリリースでしか確かめられない）か、`release.yml` は残して `tools` の 2 か所だけ寄せるか。
8. **外への約束を変える直し 10 件の扱い（要件 7.7）**: 仮置きは「同じ変更の中で読み手を全部直す」。`release.yml`（14）と使う人が読む文書（1）は、8.3 の「次のリリースで確かめる」の記録に載せる。
9. **8.1 の操作でしか効き目を確かめられない直しの扱い（要件 8.3）**: 6（入力）と、一部の 10 件。仮置きは「決定論のテストで筋まで縛り、効き目は次の本当の操作で確かめる・番号ごとに記録」。7 の「偽の親で負荷の子を 1 本だけ 1 秒未満起こす」試しが 8.1 の「負荷の再現の走行」に当たるか。
10. **pasta と同じ文で持つスキル（要件 7.9）**: 件 1 の段と 9〜12 の段を足すと、`kiro-complete/SKILL.md:437` の段の一覧を直す（今も `sample fetch`・`sample fetch check` が抜けている）。pasta 側へ同じ文にそろえる件として申し送るか。
11. **直さずに残す指摘（要件 2.2）**: 候補は 2 の手の順（⑴）だけ。⑵・⑶ の候補は無い。仮置きは 0 件。
12. **分け方（要件 2.6）**: 10 節の見立て 15〜18 で 1 本のまま。

## 13. 範囲の外で気付いたこと（その範囲の持ち物として残す）

- スキル `kiro-complete`（`SKILL.md:437`）: 全体テストの段の一覧から、先頭の `sample fetch` と `sample fetch check` が抜けている（2026-10-19 に足された段）。件 1 で段を足すときに一緒に直す読み手（要件 3.2）。
- `tools/perf`（完了した `audit-tools-perf-run`・`audit-tools-perf-judge` の範囲）: `invoke-cpu-sample.ps1` の偽の xperf の試しは、偽物が見つからないと既定の位置の本物の xperf へ落ちる（`Get-XperfPath`）。今は子の `PATH` の先頭に偽物を置くので起きない。`perf-loop.ps1 selftest` は Python の版を確かめない（12 節の 2）。
- `tools/perf/invoke-followup-checks.ps1`: `mouse-input.ps1` の `INPUT`／`MOUSEINPUT` の型はそこからの写し（コメントで明示）。範囲をまたぐ写しなので、この spec では寄せない。
- `.github/workflows/crates-io.yml`: 段にも仕事にも `timeout-minutes` が無い（16 は道具の側の 1 語で直せる。workflow の側に足すかはこの spec の外の判断）。
