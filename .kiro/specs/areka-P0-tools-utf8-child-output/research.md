# ギャップ分析: areka-P0-tools-utf8-child-output

> 2026-10-05 `kiro-validate-gap`。入力は requirements.md・brief.md・steering（tech.md・structure.md）と、`tools/` の 3 本・`.github/workflows/` の 2 本・`tools/perf/perf-loop.common.ps1` の実物。cargo のビルドは回していない。手元の確かめは `cargo metadata` の読み取りと、小さな PowerShell の実験だけ（一時ファイルは作っていない・この端末の文字コードは書き替えていない）。

## 1. まとめ

- **直す所は 4 か所で、書き方はもう `tools/perf/` に手本がある**。版を読む 3 か所（`package.ps1`・`crates-io.ps1`・`release.yml` の段「版の検査」）と、`package.ps1` の `Read-SamplePaths`。手本は `perf-loop.common.ps1` の `Invoke-Child`（`ProcessStartInfo` の `StandardOutputEncoding` を UTF-8 に固定し、標準出力と標準エラーを非同期で読む）。
- **手元の実験で、直し方と判定の作り方の両方が成り立つと確かめた**。窓の無い子（`CreateNoWindow`）は自分だけの端末を持ち、この機械では最初からコードページ 932 になる。その子の中で、今の読み方は `packages[5].description` で JSON が壊れ、`ProcessStartInfo` の読み方は版 `0.0.1` を正しく読んだ。親の端末の文字コードは前後で変わらなかった（65001 → 65001）。
- **棚卸の結果、`tools/`（`perf/` を除く）で直す所は上の 4 か所の外に無い**。ただし「表示だけ」「ASCII だけ」の 2 つの分け方に入らない所が 3 種ある（空かどうかだけを見る・ファイルを UTF-8 で読む・数を数えるだけ）。要件 4.4 の分け方を足すかは議題。
- **撤去したときの影響で調べきれていない所が 1 つある**。CI の `crates-io.yml` の段「公開前の確認」は文字コードの書き替えを持たず、今は `crates-io.ps1` の 62 行目がログの日本語を UTF-8 にしている。撤去した後にこの段のログが化けるかは、ランナーの端末の作りしだい（下の「調べが要る」）。
- 規模 **S**・危うさ **低**。新しい依存は要らない。

## 2. 今の作り（調べた事実）

### 2.1 子の出力を素で受けている 4 か所

| 所 | 今の読み方 | 失敗の扱い（変えてはいけない） |
|---|---|---|
| `tools/package.ps1` 段「前提の確認」の版を読む所（注記「版（正本は Cargo.toml の [workspace.package] version…）」の直下） | `@(cargo metadata --no-deps --locked --format-version 1 2>&1)` → `ErrorRecord` を除いて `-join "`n"` → `ConvertFrom-Json` | 子の失敗＝「版を読めない（cargo metadata が終了コード {0}: {1}）」で `{1}` は標準エラーの最後の 1 行・終了コード 3。JSON の失敗＝「版を読めない（cargo metadata の出力が JSON として読めない: $_）」・終了コード 3 |
| `tools/crates-io.ps1`「2〜6. 実物の判定」の先頭 | 同じ形（`2>&1`） | 子の失敗＝`Fail "読み取り: cargo metadata が終了コード …（$($out \| Select-Object -Last 1)）"`・終了コード 1。`{…}` は標準出力と標準エラーを混ぜた並びの最後の行。**JSON の失敗は try で受けておらず、捕まえられない例外のまま止まる**（`-File` で回すと終了コード 1） |
| `.github/workflows/release.yml` 段「版の検査」 | `package.ps1` の写し（注記「版の読み方は tools/package.ps1 と同じ」） | 文は `package.ps1` と同じ・終了コード 1 |
| `tools/package.ps1` の `Read-SamplePaths` | `@(cargo run -q --locked -p sample-ghost-kit --bin nar-sample-path -- $Sample)`（標準エラーは受けない）→ `key=` で始まる行を探し、`Test-Path -LiteralPath` に掛ける | 「nar-sample-path {検体} が終了コード {n}」「… の出力に {key}= が無い」「… の {key}= が実在しない: {path}」を throw → 段の失敗（終了コード 1） |

- `nar-sample-path`（`crates/sample-ghost-kit/src/bin/nar-sample-path.rs`）は `writeln!(out, "{key}={}", path.display())` で書く＝UTF-8。
- 4 か所とも、PowerShell は子の標準出力を `[Console]::OutputEncoding`（＝端末のコードページ）で解く。これが穴の根。

### 2.2 端末の文字コードの書き替え

- `tools/crates-io.ps1` 62 行目 `[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)`（注記「cargo の出力（JSON・UTF-8）を文字化けさせずに読む」）。**これは .NET の中で端末そのもののコードページを書き替える**ため、同じ端末を分け合う親（`test-all.ps1` やその前の開発者のシェル）にも、スクリプトが終わった後まで残る。
  - 手元の実験の裏付け: 窓を分け合う子（`CreateNoWindow` なし）は親の端末の 65001 を受け継ぎ、窓の無い子は自分の端末の 932 だった＝コードページは端末ごとの物で、分け合う子は同じ物を見る。
- `release.yml` の各段の先頭 9 か所・`crates-io.yml` の 5 か所（ランナーの端末）。範囲の外（要件の Boundary・議題 2）。
- `release.yml` の段「zip を作る」は子の pwsh の `-Command` の中で書き替えてから `package.ps1` を呼ぶ。直した後は、読み方の正しさのためには要らなくなり、ログを化けさせないためだけの物になる（範囲の外・残す）。
- 参考（範囲の外）: `tools/perf/perf-loop.common.ps1` の `Invoke-PwshChild` も子の中で `[Console]::OutputEncoding` を書き替える。注記は「端末そのもののコードページは変えない（子の中だけ）」だが、`Invoke-Child` は `CreateNoWindow` を付けないので、子は親の端末を分け合う見込みが高い＝注記と実際がずれている恐れ。`tools/perf/` は要件で外しているので、直すなら別に起票（`/kiro-discovery`）。

### 2.3 手本（`tools/perf/perf-loop.common.ps1` の `Invoke-Child`）

- `ProcessStartInfo`・`UseShellExecute=$false`・標準出力と標準エラーを両方つなぎ、両方の文字コードを UTF-8 に固定・`ReadToEndAsync` を先に始めてから `WaitForExit`（片方のバッファが埋まって子が止まるのを避ける）。
- 戻り値は `Code`・`Out`・`Err`・`Lines`・`LastLine`。
- そのまま読み込んで使うことはできない: `perf-loop.common.ps1` は perf の変数（`$PERF_LOOP_CHILD_ENCODING`・`Write-Info` ほか）を前提にした 684 行の部品集で、`Exe` に実在のパスを求める（`cargo` の名前だけでは通らない）。`tools/perf/` は範囲の外なので、手本は「写す」扱いになる。

### 2.4 手元の実験（2026-10-05・この作業木・pwsh 7.6.6）

1. `cargo metadata --no-deps --locked --format-version 1` を `ProcessStartInfo`（UTF-8）で読む → 終了コード 0・135,706 字・30 クレート・`areka` の版 `0.0.1`。
2. 同じ文字列を UTF-8 のバイトに戻してコードページ 932 で解く → `packages[5].description` で JSON が壊れる（brief の赤と同じ形）。
3. 窓の無い子 pwsh（`CreateNoWindow=$true`）の中で: 端末のコードページ 932・今の読み方（`@(cargo metadata … 2>&1)`）は**壊れる**・`ProcessStartInfo` の読み方は**版 `0.0.1` を読めた**。親の端末は前後とも 65001。
4. 窓の無い子は、子の中で何も書き替えなくても最初から 932（この機械の OEM のコードページ＝932）。窓を分け合う子は親の 65001。
5. 日本語（ASCII の外の字）の `description` を持つクレートは今 **16 本**（brief の起票のときは 13 本。数の差は新しいクレートの分と見られる・要件の主張には響かない）。

### 2.5 人が読む行の字（撤去の後の表示に関わる）

- `tools/crates-io.ps1` の文字列で、Shift_JIS（932）に無い字は `〜`（U+301C）だけで、どれも注記の中＝印字されない。撤去の後に 932 の端末へ直に書いても、印字する行は化けない見込み（要件 3.4 の土台）。
- `tools/package.ps1` で印字に出る 932 に無い字: `—`（U+2014・「展開先が長すぎる…」と「展開した木を消せなかった…」の 2 か所）と `〜`（「判定 1〜8 すべて合」）。今も 932 の端末では「?」などに化けうる＝**表示だけの文字化け**。要件 4.3 に従い、直さずに記録する対象。
- `tools/test-all.ps1` には 932 に無い字は無い。

## 3. 要件と今の作りの対応（ギャップ）

| 要件 | 今の作り | ギャップ |
|---|---|---|
| 1.1・1.4 `package.ps1` の版 | 端末で解く | **Missing**: UTF-8 で読む形へ付け替え |
| 1.2 `crates-io.ps1 -Verify` | 62 行目の書き替えで隠れている | **Missing**: 読み方の付け替え（3.3 により撤去と同じ変更で） |
| 1.3 `release.yml` 段「版の検査」 | 段の先頭の書き替えで隠れている | **Missing**: 書き替えが有っても無くても読める形へ |
| 1.5 3 か所が同じ読み方 | 写し 3 本 | **Constraint**: 写しのままか 1 つへ寄せるか（議題 1） |
| 1.6・1.7 失敗の文と終了コード | 2.1 の表 | **Constraint**: 標準エラーの最後の行（`package.ps1`・`release.yml`）と「混ぜた並びの最後の行」（`crates-io.ps1`）を新しい読み方でも作れること。`crates-io.ps1` の JSON の失敗は今「捕まえない例外」なので、同じに保つなら try で包まない |
| 2.1〜2.3 検体のパス | 端末で解く | **Missing**: 付け替え。失敗の文 3 つは今のまま |
| 3.1・3.2 書き替えない | `crates-io.ps1` 62 行目だけが書き替える | **Missing**: 62 行目の撤去 |
| 3.3 同じ変更で | — | **Constraint**: タスクを分けても 1 つの PR に入れる |
| 3.4 `-Verify` の日本語の行 | 62 行目で UTF-8 | **Unknown**: 932 の端末に直に出すなら化けない見込み（2.5）。パイプで受けて UTF-8 で読む人（例: Git Bash 経由のエージェント）からは、撤去の後は化けて見える。**どの読み手で確かめるか**が要件に書かれていない（議題） |
| 3.5 `-Pending` の標準出力 | クレート名＝ASCII | ほぼ無し。名前は ASCII なので文字コードに依らず同じ |
| 3.6 `test-all.ps1` の段 | `pwsh -NoProfile -File tools/crates-io.ps1` の終了コードだけを見る | 終了コードは同じになる見込み。ただし撤去の後は、`test-all.ps1` の後ろの段の表示の文字コードが「62 行目に書き替えられた 65001」から「元の端末の値」に戻る（表示だけ） |
| 4.1〜4.4 棚卸 | — | **Missing**: 結果を文書に残す（下の 4 節が下書き）。分け方の不足（議題） |
| 5.1〜5.4 判定で固定 | 無い | **Missing**: 判定そのもの・置き場・`test-all.ps1` への組み込み。窓の無い子で 932 を作れることは実験で確かめた |
| 6.1〜6.4 手元の確かめ | — | **Constraint**: 窓が出るので開発者の手元。結果を spec の文書に残す |
| 7.1〜7.5 変えないもの | — | **Constraint**: 失敗の文・終了コード・4 ファイル・依存・workflow の段の順・環境変数を印字しない |

## 4. 棚卸の下書き（要件 4・`tools/` の中・`tools/perf/` を除く）

調べたスクリプトは `tools/package.ps1`・`tools/crates-io.ps1`・`tools/test-all.ps1` の 3 本（`tools/` の直下はこの 3 本だけ）。

| スクリプト | 所 | 子の出力の使い方 | 扱いの案 |
|---|---|---|---|
| package.ps1 | `git rev-parse --short=7 HEAD` | 値（BUILD-INFO と zip の判定） | ASCII しか出ないので対象外 |
| package.ps1 | `git status --porcelain`（2 か所） | 始めと終わりの比較 | 要件の範囲外（`core.quotepath` で ASCII）。なお両方を同じ解き方で読むので、日本語が出ても比較は崩れない |
| package.ps1 | `cargo about --version`・`cargo deny --version` | 捨てる（終了コードだけ） | 対象外（値を使わない） |
| package.ps1 | `vswhere … -property installationPath` | **空かどうかだけ**を見る | 字面に依らない（化けても空にはならない）。今の 3 つの分け方のどれにも入らない → 議題 |
| package.ps1 | `cargo metadata` | JSON | **直す**（要件 1） |
| package.ps1 | `nar-sample-path` | パス | **直す**（要件 2） |
| package.ps1 | 段 `Step` の `& $Command 2>&1 \| ForEach-Object { Write-Host }`（`rustup`・`cargo build`・`cargo deny check`・`cargo about generate` ほか） | 表示と、失敗のときの「出力の末尾」 | 表示だけで対象外 |
| package.ps1 | `Start-Process areka.exe -RedirectStandardOutput/-RedirectStandardError` → `Get-Content -Raw` | 記録の判定（日本語の目印を探す） | 端末を通らない（子はファイルへ直に書き、pwsh 7 の `Get-Content` は既定で UTF-8 で読む）。今の分け方に入らない → 議題。α の `-Check` が日本語の目印で緑だった事実とも合う |
| crates-io.ps1 | `cargo metadata` | JSON | **直す**（要件 1.2） |
| crates-io.ps1 | `cargo publish --dry-run`・`cargo package` | 受けない（端末へそのまま）・終了コードだけ | 表示だけで対象外 |
| crates-io.ps1 | `Invoke-WebRequest`・`Get-Content -Raw`（Cargo.toml・workflow） | 子のプロセスではない | 対象外 |
| test-all.ps1 | `git rev-parse --short HEAD` | 一覧への表示 | 表示だけ・ASCII |
| test-all.ps1 | `@(git status --porcelain).Count` | 数だけ | 字面に依らない（議題の分け方） |
| test-all.ps1 | 各段の子（`cargo`・`rustup`・`pwsh -File tools/crates-io.ps1`） | 受けない・終了コードだけ | 表示だけで対象外 |

- 表示だけの文字化けの記録（要件 4.3）: `package.ps1` の `—`・`〜` を含む 3 行（2.5）。
- 版を読む段より後で、文字コードのために落ちる所は**見つからなかった**（静的に見た限り）。要件 6 の手元の走りで確かめる。

## 5. 実装の方針の候補

### 案 A: 3 か所とも、その場に同じ数行を書く（今の「写し」の作りのまま）

- 触る所: `package.ps1`（版・`Read-SamplePaths`）・`crates-io.ps1`（版・62 行目の撤去）・`release.yml`（段「版の検査」）。
- ✅ ファイルが増えない。`release.yml` の「入力は環境変数だけ・手元でも同じ本文を回せる」がそのまま。
- ❌ 同じ穴を 3 か所に写したのが今回の元。`ProcessStartInfo` の十数行が 4 回（`Read-SamplePaths` 含む）写される。要件 5.2（どれか 1 か所が戻ったら不合格）を判定するには、3 か所それぞれの本文を検査する必要がある。

### 案 B: `tools/` に小さな共通のスクリプトを 1 つ置き、3 か所が読み込む（brief の推し）

- 新しいファイル 1 つ（子を起こして標準出力と標準エラーを UTF-8 で持ち帰る関数・`Invoke-Child` の写しを小さくした物）。`package.ps1`・`crates-io.ps1` は `. (Join-Path $PSScriptRoot '…')`、`release.yml` は取り出しの後なので `. ./tools/…` で読める（段「版の検査」は「取り出し」の後）。
- 失敗の文は呼ぶ側が今の文で作る（関数は終了コード・標準出力・標準エラーの行を返すだけ）。
- ✅ 読み方が 1 か所。判定（要件 5）は「関数を 932 の子で試す」＋「`cargo metadata` を呼ぶ所が関数を通っていることを静的に確かめる」の 2 本で 5.2・5.3 を満たしやすい。`winget-manifest-submission` が倣う先にもなる。
- ❌ ファイルが 1 つ増える。`release.yml` の本文がリポジトリのファイルを読むようになる（タグのコミットの物を読むので食い違いは無いが、「本文だけで完結」の度合いは下がる）。`crates-io.yml` の「タグのコミットの tools/ を使う」とは同じ性質。

### 案 C: 版を読むのは `package.ps1` だけにし、他は `package.ps1` に版を答えさせる

- `package.ps1` に「版だけを答える」口を足す。`release.yml` の段「版の検査」はそれを呼ぶ。
- ❌ 段「版の検査」は段「道具の用意」（cargo about・deny）より前にある。`package.ps1` の前提の確認は道具の有無を先に見るので、版だけの口は道具の確かめを飛ばす別の道が要る。`package.ps1` の終了コードの約束（3）と `release.yml` の 1 の対応も増える。`crates-io.ps1` は「環境変数を読まない・単独で回る」作りで、`package.ps1` に頼らせると道具の間の依存が増える。`Read-SamplePaths` は結局別に直す。触る量が最も多い。

### 判定（要件 5）の候補

- **5-a 窓の無い子で 932 を作る動的な判定**: 判定のスクリプトが `pwsh` を `CreateNoWindow` で起こし、その子が本物の読み方で今のワークスペースを読む（実験 2.4-3 と同じ形）。子の端末は子だけの物で、開発者の端末は変わらない（5.4）。
  - 932 の作り方: ⒜ 子の中で `[Console]::OutputEncoding` を 932 にする（どの機械でも 932 になる・ただし `tools/` の中に書き替えの行が 1 つ入る＝要件 3.1 の字面との関係が議題）⒝ 書き替えず、窓の無い端末の既定（OEM のコードページ）に任せる（この機械では 932。英語の機械では 437 になり、穴を再現できない＝その機械では判定が空振りする）。
- **5-b 字の並びに頼らない判定（5.3）**: 今の `description` が将来壊れない並びになっても不合格を出せるよう、932 で解くと必ず壊れる UTF-8 のバイト列（例: 今の `areka-nar` の `description`）を子に書かせ、読み方の関数が元どおりの字を返すことを確かめる。案 B なら関数を直に試せる。
- **5-c 静的な判定（5.2 を 3 か所に及ぼす）**: `tools/*.ps1`（`perf/` を除く）と `release.yml` で、`cargo metadata` と `nar-sample-path` を素で呼ぶ行（`@(cargo …)` の形）が無いことを機械で確かめる。案 A では 3 か所の本文を個別に見ることになる。
- 置き場の候補: ⒜ `crates-io.ps1` の「1. 判定の較正」に足す（既に「正しい見本を通し、誤った見本を落とす」形があり、全体テストで毎回回る。ただし公開の道具の仕事が増える）⒝ 新しい判定のスクリプトを `tools/` に置き、`test-all.ps1` の段に足す（steering の tech.md の全体テストの説明に段が増える）⒞ 案 B の共通スクリプトに自己判定の口を持たせ、`test-all.ps1` の段から呼ぶ。

## 6. 規模と危うさ

- **規模 S（1〜3 日）**: 手本があり、触るのは `tools/` の 2〜4 ファイルと `release.yml` の 1 段。依存の追加なし。
- **危うさ 低**: よく知られた .NET の口（`ProcessStartInfo`）で、実験で効き目と端末を変えないことを確かめ済み。残る不確かさは CI のログの見た目（下）と、失敗の文の作り直し（標準エラーの最後の行）だけ。

## 7. 設計へ持ち越す「調べが要る」

1. **CI の段「公開前の確認」（`crates-io.yml`）のログ**: この段だけ書き替えの行を持たない。62 行目を消した後、ランナーで同じジョブの段どうしが端末を分け合うなら前の段の 65001 が効いたまま、段ごとに新しい端末ならランナーの既定（英語の機械なら 437 など）に戻って日本語の行が化ける。合否（終了コード）には響かない。同じく段「記録」は子の標準エラーをファイルにして UTF-8 で読むが、段の先頭で書き替えてから子を呼ぶので、分け合う子は 65001 を受け継ぐ見込み。→ 乾いた走り（`workflow_dispatch`）か、ランナーの端末の作りの資料で確かめる。
2. **`Read-SamplePaths` の標準エラーの行き先**: 今は受けていないので、段 `Step` の `2>&1` が拾って「出力の末尾」に出る見込み。新しい読み方で標準エラーをつなぐと、関数が持ち帰って呼ぶ側が出さない限り末尾から消える。つながずに端末へそのまま流す形もある。どちらで今の見え方を保つかを設計で決める。
3. **`crates-io.ps1` の子の失敗の文**: 今は「標準出力と標準エラーを混ぜた並びの最後の行」。分けて受けると並びの順は作れない。`cargo metadata` が失敗するときは標準出力が空なので「標準エラーの最後の行」と同じになる見込み、を設計で確かめて書く。
4. **`cargo` の起こし方**: `ProcessStartInfo` に名前 `cargo` を渡すと PATH から探す（実験で動いた）。`package.ps1` は PATH から MSVC 以外の `link.exe` の置き場を外した後に呼ぶので、その PATH で探されることを確かめる（rustup の代理の `cargo.exe` が見つかればよい）。

## 8. 要件ディスカッションへ渡す議題（答えで作業が変わるもの）

1. **読む関数の置き場**（brief の議題 1）: 案 A（3 か所に写す）／案 B（`tools/` に共通のスクリプト 1 つ・`release.yml` も読み込む）／案 C（`package.ps1` に版を答えさせる）。判定（要件 5.2）の作りやすさは案 B が最も良い。
2. **workflow の各段の先頭の書き替え**（brief の議題 2）: ⒜ 残す（段「版の検査」は頼らない読み方にするだけ）／⒝ 全部消す。加えて、`crates-io.yml` の段「公開前の確認」は今も書き替えを持たず、62 行目の撤去の後にログが化けうる（7-1）。⒜ を採るなら、この段に他の段と同じ 1 行を足すか（ランナーの端末の物で開発者の端末ではない・workflow の段の順や権限は変わらない）、化けを受け入れるか。
3. **判定のために子の端末を 932 にする行を許すか**: 要件 3.1 は「`tools/` のスクリプトは端末の文字コードを書き替えない」。判定の子は自分だけの窓の無い端末を持つので開発者の端末は変わらない（実験で確かめた）が、字面では書き替えの行が `tools/` に入る。許さないなら、窓の無い端末の既定（OEM のコードページ）に任せ、932 でない機械では判定が「再現できない」と言って止まる（黙って合格にしない）形になる。
4. **判定の置き場**: `crates-io.ps1` の較正に足す／新しい判定のスクリプトを `test-all.ps1` の段にする／共通のスクリプトの自己判定。段を足すなら steering の `tech.md` の全体テストの説明も合わせて直す。
5. **棚卸の分け方の不足（要件 4.4）**: 「直した・表示だけで対象外・ASCII しか出ないので対象外」の 3 つに入らない所がある（`vswhere` の空かどうか・`git status` の数・`areka.exe` の記録をファイルから UTF-8 で読む所）。分け方に「字面に依らない（空か数だけ）」「端末を通らない（ファイルを UTF-8 で読む）」を足すか、`vswhere` も念のため直すか。
6. **要件 3.4「化けずに出す」をどの読み手で確かめるか**: 932 の端末の窓に直に出すなら撤去の後も化けない見込み。パイプで受けて UTF-8 で読む読み手（エージェントの Bash など）からは、撤去の後は化けて見える（今は 62 行目のおかげで UTF-8）。確かめの条件を「開発者の端末の窓」に絞ってよいか。
7. **`crates-io.ps1` の JSON の失敗の扱い**: 今は捕まえない例外で止まる（`Fail` の文ではない）。要件 1.7「今と同じ」に従ってそのまま残すか、`package.ps1` と同じ形の文に揃えるか（揃えると文が変わる＝要件の改訂が要る）。
8. **範囲の外の気付きの起票**: `tools/perf/perf-loop.common.ps1` の `Invoke-PwshChild` は子の中で端末を書き替えるが、子は親の端末を分け合う見込みで、注記「子の中だけ」と実際がずれている恐れ。`/kiro-discovery` で起票するか。

## 9. 次の段

- 上の議題を要件ディスカッションで決めてから、`/kiro-spec-design areka-P0-tools-utf8-child-output`（または `/kiro-design`）へ。

## 10. 設計フェーズで決めること（要件ディスカッションの仕分け）

要件ディスカッション（2026-10-05）で「how の判断」として設計へ回したもの。要件は変えない。

1. 読む関数の置き場（8 節の議題 1）: 案 A・B・C。要件 1.5「3 か所が同じ読み方」と 5.2「どれか 1 か所が戻れば不合格」を満たす作りを選ぶ（brief とこの分析の推しは案 B）。
2. 判定の置き場（8 節の議題 4）: `crates-io.ps1` の較正・新しい判定のスクリプトを `test-all.ps1` の段に・共通のスクリプトの自己判定。段を足すなら steering の `tech.md` も同じ PR で直す。
3. 判定の組み立て（5 節の 5-a・5-b・5-c）: 5.2 と 5.3 を両方満たす組み合わせ。
4. 7 節の「調べが要る」4 つ（CI のログの見え方・`Read-SamplePaths` の標準エラーの行き先・`crates-io.ps1` の子の失敗の文の作り直し・絞った PATH での `cargo` の起こし方）。

仕分けで要件側を直したもの（自明な修正）: 8 節の議題 5（棚卸の分け方を 2 つ足した＝要件 4.4）・議題 6（要件 3.4 は開発者の端末の窓で確かめる）・議題 7（要件 1.7 に、公開の道具の JSON の失敗は今の止まり方のまま残すと明記）。議題 8（`tools/perf/` の `Invoke-PwshChild`）は範囲の外のため、別に `/kiro-discovery` で起票する候補として残す。

### 10.1 要件ディスカッションの裁定（2026-10-05・議題 1/1）

- 開発者の方針: 開発者の PC と CI では端末の文字コードが違う。道具はそれに影響を受けず、ANSI・OEM のコードページが何であっても同じに動く。端末の設定を安易に書き替えると、同じ窓で続けて自分で PowerShell を打てなくなる。ASCII はどの端末でも読める前提でよく、端末へ出す文に日本語は使わない。
- 要件への反映: 要件 3.7（どのコードページでも同じ結果・端末はそのまま）・要件 5.5（判定は機械のコードページに依らず同じ合否・932 は窓の無い子だけの端末に作る）・要件 8（端末へ出す文は ASCII だけ）。workflow の各段の先頭の書き替え（`release.yml` 9＋1 か所・`crates-io.yml` 5 か所）は撤去へ（要件 3.1）。8 節の議題 2・3 と 7 節の 1（CI のログの化け）はこれで解けた。
- 設計で決めること（追加）: ASCII へ書き替える文の一覧と英文の言い回し・ASCII の外の字を見つける判定の作り（どの行を「端末へ出す文」とみなすか）。

## 11. 設計フェーズの調べと決定（2026-10-05・`kiro-spec-design`）

### 11.1 まとめ

- 発見の種類: 既存の道具の手直し（light）。新しい依存なし。外部の資料は要らない（.NET と PowerShell に既に在る口だけ）。
- 決めたこと: 読み方は共通のスクリプト `tools/utf8-child.ps1` の `Invoke-Utf8Child` 1 つ（案 B）。判定は新しいスクリプト `tools/encoding-check.ps1` を `tools/test-all.ps1` の段にする。判定は「較正 → 本文の判定（構文解析器）→ 窓の無い子の端末を 932 にした動的な判定」の 3 段。
- 端末へ出す文の英語の文面と、棚卸の結果（要件 4）は `design.md` が正本。

### 11.2 手元の実験（2026-10-05・この作業木・pwsh 7.6.6・この端末の文字コードは書き替えていない）

- **1 字で壊れる見本**: `{"description":"版","version":"0.0.1"}`（`版`＝U+7248＝UTF-8 で `E7 89 88`）を UTF-8 のバイトにしてコードページ 932 で解くと JSON が壊れる。`検体`・`ゴースト`・`日本語の説明` も壊れた。`版` の最後のバイト `0x88` が 932 の 1 バイト目になり、続く `"` を飲む。判定の見本は今の `description` に頼らず、この 1 字に決めた（要件 5.3）。
- **判定の形の試作**: 親が `ProcessStartInfo`（`CreateNoWindow`）で子の pwsh を起こし、子が `[Console]::OutputEncoding` を 932 にした（子だけの端末）。子の中で、⒜ 見本を出す子を素の呼び出しで読むと **壊れた**（較正）・⒝ `ProcessStartInfo`（UTF-8）で読むと `description` が字のとおり・版 `0.0.1`・⒞ 本物の `cargo metadata` を同じ読み方で読むと終了コード 0・`areka` の版 `0.0.1`。親の端末のコードページは前後で同じ。
- **本文の判定の試作**: `Parser.ParseInput` の字句のうち注記でない物に ASCII の外の字を持つ物は、設計の時点で 319 個（`tools/package.ps1` 121・`tools/crates-io.ps1` 132・`tools/test-all.ps1` 11・`release.yml` の `run:` 38・`crates-io.yml` の `run:` 17）。`package.ps1` の 6 個は `$LOG_MARKER_*` の代入の右辺（areka の記録を探す日本語の目印）で、例外にする。`run:` の本文は字下げで取り出して pwsh として構文解析できた（構文の誤り 0）。
  - 余談の観察: この試作の出力を、端末の文字コード（932）のまま Git Bash で受けると化けた。UTF-8 のファイルへ書かせて読むと読めた。本件の穴と同じ形で、道具の出す文を ASCII にする理由の裏付け。
- **作業フォルダの落とし穴**: `ProcessStartInfo` は .NET のプロセスの作業フォルダで子を起こし、PowerShell の `Set-Location` はそれを動かさない。`package.ps1`・`crates-io.ps1` は根へ `Set-Location` するので、`Invoke-Utf8Child` は `WorkingDirectory` に今の場所を必ず入れる（`perf-loop.common.ps1` の `Invoke-Child` も同じ）。

### 11.3 決定

#### 決定: 読み方の置き場は共通のスクリプト 1 つ（案 B）

- **Context**: 要件 1.5（3 か所が同じ読み方）・5.2（どれか 1 か所が戻れば不合格）。同じ穴を 3 か所に写したのが今回の元。
- **Alternatives**: 案 A（3 か所に写す）・案 B（`tools/` に 1 本・3 か所が読み込む）・案 C（`package.ps1` に版を答えさせる）。
- **Selected**: 案 B。`tools/utf8-child.ps1` は `Invoke-Utf8Child` だけを定義する。失敗の文と終了コードは呼ぶ側が今のまま持つ。
- **Rationale**: 読み方の出どころが 1 か所で、判定は「関数の中身（932 の子）」と「呼ぶ所の形（本文の規則 D・E）」の 2 本で 5.2 を塞げる。案 C は段の順（「版の検査」は「道具の用意」の前）と道具の間の依存が増える。
- **Trade-offs**: ファイルが 1 つ増え、`release.yml` の段「版の検査」がリポジトリのファイルを読む（取り出したタグのコミットの物＝`crates-io.yml` が `tools/crates-io.ps1` を使うのと同じ性質。「入力は環境変数だけ」は保つ）。
- **Follow-up**: `winget-manifest-submission` は同じ関数と同じ判定に倣う。

#### 決定: 子は既定で端末を分け合い、判定の子だけが窓の無い自分の端末を持つ

- **Context**: 要件 5.5 は 932 を判定の子だけの端末に作ると定める。道具の子（cargo の長い組み立て）は Ctrl+C で道連れに止まってほしい。
- **Selected**: `Invoke-Utf8Child -OwnConsole` のときだけ `CreateNoWindow`。既定は今の素の呼び出しと同じく端末を分け合う（子は端末を書き替えないので分け合っても 3.2 は守られる）。
- **Rationale**: 判定の子の 932 が開発者の端末に漏れない形を、引数 1 つで作れる。

#### 決定: 判定は 3 段（較正 → 本文 → 932 の子）・置き場は新しいスクリプトを全体テストの段に

- **Context**: 要件 5.1〜5.5・8.5。ギャップ分析 5 節の 5-a・5-b・5-c と置き場の 3 案。
- **Selected**: `tools/encoding-check.ps1` を `tools/test-all.ps1` の段 `encoding check` にする。本文の判定は PowerShell の構文解析器で、注記でない字句の ASCII（規則 A）・目印の差し込みの禁止（B）・書き替えの禁止（C・構文木だけ）・素の `cargo metadata`／`nar-sample-path` の禁止（D）・3 か所の呼び出しの在処（E）。各規則は埋めた見本で先に較正する（`crates-io.ps1` の「判定の較正」と同じ形）。動的な判定は窓の無い子の端末を 932 にし、素の読み方で見本が壊れること（較正）と、`Invoke-Utf8Child` で読めること・本物の版が読めることを確かめる。
- **Rationale**: 公開の道具（`crates-io.ps1`）の仕事を増やさない。本文の判定は今の `description` の字に頼らない（5.3）。932 は子が明示して作るので、英語の機械（OEM 437）でも同じ合否（5.5）。子の端末を 932 にできない・較正で見本が壊れない機械では不合格にして、黙って合格にしない。
- **Trade-offs**: 全体テストに数秒（pwsh の起動 3 回と `cargo metadata` 2 回）。steering の `tech.md` の全体テストの説明を同じ PR で直す。
- **要件 3.1 との関係**: 判定の子が自分の端末を 932 にする 1 行は `tools/` の中に在る。要件 3.1 の書き替え禁止は共有の端末が対象で、要件 5.5 が子だけの端末の 932 を定めている（ギャップ分析の議題 3 は 10.1 の裁定で解けた）。規則 C の例外はその関数 1 つに絞り、子は標準出力と標準エラーがつながれていない（＝手で端末から回された）ときは何もしない。

#### 決定: ASCII の規則の例外は `$LOG_MARKER_*` の代入の右辺だけ

- **Context**: `package.ps1` は areka の記録の日本語の行（`本物のゴースト窓を開きました` など）を目印に探す。目印は areka の文言なので英語にできない。
- **Selected**: 左辺が `$LOG_MARKER_*` の代入の右辺を規則 A の例外にし、その変数を差し込みのある文字列に入れることを規則 B で禁じる。`Test-RunLog` の `Detail` は目印の文言でなく変数名を出す。初回のバルーンの行は、記録の行の抜き書きをやめ、`route=Companion|other dir=<記録の値>` を出す。
- **Rationale**: 例外が名前で機械的に決まり、印字への漏れを塞げる。`` `u{…} `` で目印を書くと読めなくなるので取らない。

#### 決定: 文に入れる値は書き替えない

- **Context**: 要件 8.1 は「自分で端末へ出す文」を ASCII にと定める。文にはパス・子の出力の行・例外の文・URL が差し込まれる。
- **Selected**: 道具の作者が書く字だけを ASCII にし、差し込む値はそのまま（パスにはワークツリーの日本語が入りうる。8.4 と同じ考え）。

#### 決定: workflow の `gh` の読み方は `--jq` で「比べる欄も行の最後の欄も ASCII」の形にする

- **Context**: 段の先頭の書き替えを消すと、`gh` の出力はランナーの端末の文字コードで解かれる。2 バイトの文字コードでは ASCII の外のバイトが続きのバイト（区切りのタブや改行）を飲みうる。
- **Selected**: `crates-io.yml` 段「release を待つ」は回の一覧の JSON（回の題・コミットの文を含む）を丸ごと読むのをやめ、`--jq '.workflow_runs[] | [.head_branch, .status, (.conclusion | tostring)] | @tsv'` の 3 つの欄だけを読む。`release.yml` のタグの一覧は `[.name, .commit.sha] | @tsv`。Release の一覧（`[.tag_name, .id, .html_url, .draft] | @tsv`）は既にこの形なので変えない。
- **Rationale**: 行の最後の欄が ASCII なら行は混ざらず、比べる欄（タグの名前＝ASCII）は字のとおりに解ける。段「release を待つ」は「取り出し」の前で `tools/` を読めないので、`Invoke-Utf8Child` は使えない。
- **Follow-up**: 段「release を待つ」はタグの push でしか回らない。実装のときに手元の `gh api`（`event=workflow_dispatch` の回の一覧）で `--jq` の出力の形を確かめる。

#### 決定: 段「zip を作る」は `-File` で呼ぶ

- **Context**: `-Command` は子の中で書き替えるためだけの形だった（注記「-File では呼ぶ前に文字コードを設定できないので -Command」）。
- **Selected**: `pwsh -NoProfile -NonInteractive -File ./tools/package.ps1 -Arch all`。`$host.SetShouldExit($code)` と `exit $code` は今のまま（`-File` は子の終了コードをそのまま返す）。

### 11.4 ギャップ分析 7 節の「調べが要る」の答え

1. CI のログの化け: 10.1 の裁定（端末へ出す文は ASCII）で解けた。段の先頭の書き替えを全部消しても、道具と本文の文は ASCII なのでどの文字コードでも化けない。子の出力（cargo・gh）の中身の化けは表示だけで範囲の外。
2. `Read-SamplePaths` の標準エラーの行き先: `ErrLines` を `Write-Host` でそのまま写す（今は段の `2>&1` が拾って画面に出ている。見え方を保つ）。失敗の文は意味を変えない。
3. `crates-io.ps1` の子の失敗の文: `cargo metadata` は失敗のとき標準出力に何も書かないので、「混ぜた並びの最後の行」と「標準エラーの最後の行」は同じ値になる。`ErrLines` の最後の行を使う。
4. 絞った PATH での `cargo` の起こし方: `Invoke-Utf8Child` は `Get-Command -CommandType Application` で名前を解く＝今の素の `cargo` 呼び出しと同じ探し方で、`package.ps1` が MSVC 以外の `link.exe` を外した後の PATH で探す。

### 11.5 数の食い違いの記録

- 要件の Boundary は `release.yml` の書き替えを「9 か所＋段「zip を作る」の子の中の 1 か所」と書くが、実物は段の先頭 8 か所（段「改行の設定」「版の検査」「既存の Release の検査」「環境の記録」「zip を作る」「4 つの確かめ」「Release を公開」「後始末」）＋子の中の 1 か所＝計 9 行。要件の意図（全部消す）は変わらないので、設計は実物の 9 行を消す。判定の規則 C が 0 件を機械で確かめる。設計の討議（自明な修正）で要件の Boundary の数を「段の先頭 8 か所＋子の中の 1 か所」へ直した（数え違いの訂正で、範囲は変わらない）。

### 11.6 リスクと手当て

- Windows に 932 のコードページが無い機械 — 判定は不合格になる（黙って合格にしない）。Windows の標準の構成には入っている。
- 段「release を待つ」の `--jq` の式の誤りは初回のタグの push まで表に出ない — 実装のときに手元の `gh api` で出力の形を確かめる（11.3）。
- 道具の文が英語になり、`release-cycle` の手順や記録が日本語の文を目印にしていれば読み替えが要る — 要件の Adjacent expectations のとおり `release-cycle` の側で読み替える。

## 12. 手元の確かめの記録（要件 6）

（実装の後に開発者が記入する: 端末の文字コード・`tools/package.ps1 -Check` の終了コードと通った段・`tools/crates-io.ps1 -Verify` の終了コード・回す前後の `chcp` と `[Console]::OutputEncoding.CodePage`・見つかった文字化け）
