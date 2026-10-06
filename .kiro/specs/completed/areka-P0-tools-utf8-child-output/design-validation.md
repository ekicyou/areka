# 設計の検証: areka-P0-tools-utf8-child-output

> 2026-10-05 `kiro-validate-design`（非対話）。入力は `spec.json`（language: ja）・`requirements.md`・`design.md`・`research.md`・steering（`tech.md`・`structure.md`・`workflow.md`・`roadmap.md` の該当行）。設計の主張は実物（`tools/package.ps1`・`tools/crates-io.ps1`・`tools/test-all.ps1`・`tools/perf/perf-loop.common.ps1` の `Invoke-Child`・`.github/workflows/release.yml`・`.github/workflows/crates-io.yml`）と突き合わせた。手元では PowerShell の構文解析器で、3 本の道具の「注記でない・ASCII の外の字を含む字句」を数えただけ（`package.ps1` 121・`crates-io.ps1` 132・`test-all.ps1` 11。どれも文字列の字句で、構文の誤りは 0）。ビルドは回していない。

## 設計の検証の要約

読み方を `tools/utf8-child.ps1` の `Invoke-Utf8Child` 1 つにまとめ、直したことを「本文の判定（規則 A〜E）」と「窓の無い子の端末を 932 にした動的な判定（較正つき）」の 2 本で固定する形は、要件 1〜8 をすべて辿れる。研究の記録（`research.md` 2.4・11.2）の実験が、要の前提（窓の無い子は自分だけの端末を持つ・その中で素の読み方は壊れ、`ProcessStartInfo` の読み方は読める・親の端末は変わらない）を裏付けている。下の 3 点はどれも設計の骨組みを変えない明確化で、実装に進んでよい。

### 実物と突き合わせて確かめたこと

- 書き替えの行: `tools/crates-io.ps1` の 62 行目 1 か所・`release.yml` の段の先頭 8 か所（31・52・95・152・205・220・274・297 行）＋段「zip を作る」の子の中 1 か所（209 行）・`crates-io.yml` の 5 か所（43・68・119・139・167 行）。計 15 行で、設計の数と合う。
- 版を読む 3 か所（`package.ps1` 311 行・`crates-io.ps1` 244 行・`release.yml` 55 行）と `Read-SamplePaths`（`package.ps1` 432 行）は、どれも素の `cargo` 呼び出し。設計の 4 か所と合う。`tools/` の直下は 3 本だけ（`perf/` を除く）で、棚卸の対象の数と合う。
- `crates-io.ps1` の JSON の失敗は `$ErrorActionPreference = 'Stop'` の下の捕まえない例外で終了コード 1（要件 1.7 の「今のまま」と合う）。較正の `(?=^# 権限)` は見本 `$flow` にだけ掛かり、実物の `crates-io.yml` には掛からない（英語へ変えても実物の判定は変わらない）。
- workflow の `run:` の本文に `${{ … }}` は 0 件（入力は環境変数だけ）。本文を pwsh として構文解析する規則と矛盾しない。
- 手本の `Invoke-Child`（`perf-loop.common.ps1`）は `ReadToEndAsync` を 2 本先に始めてから `WaitForExit` する形で、設計の「先に非同期で読み始める」と合う。

## 重大な指摘（3 件）

🔴 **指摘 1: 本文の判定の規則 A と `run:` の取り出しに、実装で割れうる所が 2 つある**
**Concern**: ⒜ 規則 A の「字句の字」が、字句の原文（`Token.Text`／`Extent.Text`）か、解いた値（`StringToken.Value`）かが書かれていない。値で見ると、判定のスクリプト自身が較正の見本を組むための `` `u{…} `` が日本語に解けて、自分の規則 A で落ちる。⒝ `run:` の取り出しは「`run: |`」と「`run: <1 行>`」の 2 形だけを定めるが、実物の両方の workflow には `defaults:` の下の `run:`（値が写像 `shell: pwsh`）がある。どの形にも当たらないので、扱いが実装しだいになる。
**Impact**: ⒜ は判定が自分で赤になる（気付けるが手戻り）。⒝ は `shell: pwsh` を本文として解く・空の本文とする・取り出しを誤る、のどれにも転びうる。規則 E（`release.yml` の `run:` に `Invoke-Utf8Child … metadata` が在る）の数え方にも響く。
**Suggestion**: 設計の「2. 本文の判定」に 2 行足す。「規則 A は字句の原文（`Extent.Text`）を見る」「取り出すのは `steps` の要素の `run:` で、値が `|` か 1 行のスカラーのものだけ。`defaults.run` のような写像の `run:` は本文にしない」。較正の通すべき見本に `defaults:` → `run:` → `shell: pwsh` を持つ YAML を足すとなお良い。
**Traceability**: 8.5・8.7・5.2・1.5
**Evidence**: design.md「encoding-check.ps1」→「2. 本文の判定」（`run:` の本文の取り出し・規則 A）・「1. 判定の較正」

🔴 **指摘 2: `Read-SamplePaths` の標準エラーを `Write-Host` で写すと、段が失敗したときの「出力の末尾」から cargo の誤りが抜ける**
**Concern**: 今は `cargo run … nar-sample-path` の標準エラーが、段 `Step` の `& $Command 2>&1` に拾われて `$out` に入り、失敗のときに `---- 出力の末尾 ----` に出る。設計は `ErrLines` を `Write-Host` でそのまま写すとするが、`Write-Host` は情報の流れで `2>&1` に入らず、`$out` に残らない。失敗の文は「終了コード N」だけなので、末尾の節から誤りの中身が消える。あわせて、組み立ての間の逐次の表示は子が終わるまで出なくなる。
**Impact**: 検体の窓口の組み立てが落ちたとき、画面の上の方には出るが、失敗の要約（末尾の節）だけを見る読み手（エージェントの末尾の読み取りなど）からは原因が見えなくなる。要件 8.3「どの値で失敗したかを落とさない」の趣旨に触れる。
**Suggestion**: 版を読む所と同じ形にそろえ、終了コードの失敗の文に `ErrLines` の最後の行を添える（例 `nar-sample-path $Sample exited with code $($r.Code): $($r.ErrLines | Select-Object -Last 1)`）。逐次の表示が消えることは「`cargo run -q` で出る物は少ない」として設計に 1 行で認めておく。
**Traceability**: 2.3・8.3・8.4
**Evidence**: design.md「Read-SamplePaths（package.ps1）」・「tools/package.ps1 の文」の `nar-sample-path $Sample exited with code …` の行

🔴 **指摘 3: 要件の境界の数と、実物・設計の数がずれている（`release.yml` の書き替えの行）**
**Concern**: 要件の In scope は「`release.yml` の 9 か所＋段『zip を作る』の子の中の 1 か所」（＝10）と書くが、実物は段の先頭 8 か所＋子の中 1 か所（＝9 行）で、設計も「8 段の先頭＋子の中の 1 行・計 15 行」と実物に合わせている。設計の側にこのずれの説明が無い。
**Impact**: タスクの生成やレビューが要件の「9＋1」を数の正本として読むと、無い 10 個目を探すか、設計の 15 行を誤りと見なす。判定の規則 C が機械で 0 件を見るので結果は変わらないが、手戻りの種になる。
**Suggestion**: 設計の Boundary Commitments（または `research.md`）に「要件の『9 か所』は段の先頭 8 か所＋段『zip を作る』の 1 か所を合わせた数で、子の中の 1 行と合わせて `release.yml` は 9 行」と 1 行で読み替えを残す（要件は確定済みなので書き替えない）。
**Traceability**: 3.1（要件の Boundary Context の In scope）
**Evidence**: design.md「Overview」の Impact（計 15 行）・「Boundary Commitments」→「This Spec Owns」の 3 つ目

## 設計の強み

- **読み方の出どころを 1 つにし、戻りを 2 本の独立した網で塞いでいる**: 呼ぶ所の形（規則 D の素の呼び出しの禁止・規則 E の在処）と、関数の中身（932 の子で固定の見本 `版`＝U+7248 を読む・素の読み方で壊れることを先に較正）。どちらも今の `description` の字にも、判定を回す機械のコードページにも頼らない（5.2・5.3・5.5）。子の端末だけを 932 にし、親の 2 つのコードページを前後で比べるので 5.4 も機械で見ている。
- **要件 8 を「一覧＋機械の門」で閉じている**: 端末へ出す文を所ごとに英文と終了コードで並べた一覧（正本）と、構文解析器で注記以外の字句を見る規則 A の組み合わせで、取りこぼしは全体テストが赤にする。`gh` の出力を `--jq` で「最後の欄が ASCII」の形に絞る手当ては、2 バイトの文字コードで行が混ざる穴まで考えた、撤去の後にも結果を端末から切り離す丁寧な設計。

## 最終の判断

**Decision: GO**

**Rationale**: 設計は要件 1〜8 のすべての受け入れ基準を部品と流れに辿れ、要の技術的な前提は実験で裏付けられている。3 つの指摘はどれも 1〜2 行の明確化で済み、骨組み（共通の関数・判定の 3 段・撤去の範囲）を変えない。

**Next Steps**:
1. 設計の討議（`kiro-design-discussion`）で指摘 1〜3 を扱い、必要なら design.md に明確化の行を足す。
2. その後 `/kiro-spec-tasks areka-P0-tools-utf8-child-output` でタスクを生成する。要件 3.3（撤去と読み方の直しを同じ PR）と、設計の「戻しの確かめ」⒜〜⒟ をタスクに落とす。
3. 要件 6 の手元の確かめ（932 の端末で `-Check`・`-Verify`）は窓が開くので開発者の手で行い、`research.md` の節に記録する。
