# Requirements Document

## Project Description (Input)

リリースの道具（`tools/package.ps1`・`tools/crates-io.ps1`・`.github/workflows/release.yml` の段「版の検査」）が、子のプロセス（`cargo metadata`）が UTF-8 で書いた出力を、**端末の文字コード**で解いている。開発者の端末（Shift_JIS＝コードページ 932）では、日本語の `description` を持つクレートの所で JSON が壊れ、`tools/package.ps1 -Check` が「版を読めない」の終了コード 3 で止まる。初回リリース（`release-cycle` の `v0.0.2`）の手順 3.3 がここで赤になり、初回リリースはこの直しを待っている。

困っているのはリリースを回す開発者。今は `tools/crates-io.ps1` だけが冒頭で端末の文字コードを UTF-8 に書き替えて穴を避けているが、これは共有物である端末の設定を道具が書き替える形で、スクリプトが終わった後も端末に残る。開発者の方針（2026-10-05「まずはコンソールをいじるべきじゃない」）は「文字化けは端末の文字コードを変えて逃げず、読む側を直す」。

変えたいこと: 子の出力を端末の文字コードに依らず UTF-8 として読む形に揃え、`crates-io.ps1` の書き替えを消す。端末の文字コードが何であっても道具が同じ結果を出し、道具を回した後の端末は回す前と同じであること。直したことは、コードページ 932 の条件で版が読めることを合否で判定して固定する。出どころの詳細は `brief.md`。

要件ディスカッション（2026-10-05）で開発者が示した方針: 開発者の PC と CI のランナーでは端末の文字コードが違う。道具はそれに影響を受けてはならず、ANSI・OEM のコードページが何であっても同じに動く。端末の設定を安易に書き替えると、開発者が同じ窓で続けて自分で PowerShell を打てなくなるので書き替えない。どの端末でも ASCII は読めることを前提にしてよく、道具が端末へ出す文に日本語は使わない（ASCII だけにする）。

## Introduction

本書は、リリースの道具が子のプロセスの出力を読むときに端末の文字コードへ頼らないようにするための要件を定める。対象は、版の正本を読む 3 か所（`tools/package.ps1`・`tools/crates-io.ps1`・`.github/workflows/release.yml` の段「版の検査」）、`tools/package.ps1` が検体のパスを読む所、`tools/crates-io.ps1` の冒頭の端末の書き替えの撤去、`tools/` の中の同じ読み方の棚卸、道具と workflow が端末へ出す文を ASCII だけにすること（あわせて workflow の各段の先頭の端末の書き替えも撤去する）、そして直したことを固定する判定である。段は**バグ**。

今の赤は `description` の字の並びしだいで出たり出なかったりする（Shift_JIS の 1 バイト目に見えるバイトが、文字列を閉じる `"` を飲み込んだときだけ壊れる）。日本語の `description` は 16 クレートにあり（2026-10-05 のギャップ分析の時点。起票のときは 13）、少なくとも `areka-nar`・`areka-ghost`・`areka-mcp` で壊れる。今まで通っていたのは偶然で、要件は「どの字の並びでも、どの端末の文字コードでも同じ結果」を求める。

## Boundary Context

- **In scope**:
  - 版の正本を読む 3 か所の読み方（`tools/package.ps1` の版を読む段・`tools/crates-io.ps1` の「2〜6. 実物の判定」の先頭・`release.yml` の段「版の検査」）
  - `tools/package.ps1` の検体のパスを読む所（`nar-sample-path` の出力の `key=value` からパスを読み、実在を確かめる所）
  - `tools/crates-io.ps1` の冒頭の端末の文字コードの書き替えの撤去と、撤去の後も出力が変わらないことの確かめ
  - `tools/` の中（`tools/perf/` を除く）で、子の出力を素で受けて字面・JSON・値として判定に使う所の棚卸と、見つかった所の直し
  - コードページ 932 の条件で版が読めることの合否の判定
  - `tools/` のスクリプト（`tools/perf/` を除く）と `release.yml`・`crates-io.yml` の段の本文が、自分で端末へ出す文（進みの表示・失敗の文・判定の結果の行）を ASCII だけにすること
  - workflow の各段の先頭にある端末の書き替え（`release.yml` の 9 か所＋段「zip を作る」の子の中の 1 か所・`crates-io.yml` の 5 か所）の撤去
- **Out of scope**:
  - 版上げそのもの・タグ・Release（`release-cycle` の持ち物）
  - `tools/perf/`（既に端末に頼らない読み方になっている）
  - 子のプロセスの出力を人が読むためにそのまま写す所の文字化け（例: `package.ps1` が組み立ての出力を 1 行ずつ写す所）。中身は子の物で、合否に関わらないので直さない
  - スクリプトの注記・workflow の段の名前・手順書の地の文（端末へ出ないので日本語のままでよい）
  - `git status --porcelain` を読む所（git が日本語のパスを ASCII の逃がし字で出すため、文字コードの穴にならない）
  - workflow の中身の作り替え（段の順・権限・待ち方）
  - 端末・ランナーの設定を変える手順を足すこと、`Cargo.toml` の `description` の言い回しを変えること
- **Adjacent expectations**:
  - `release-cycle`: この直しが main に入った後、自分のワークツリーで main を取り込み、全体テスト → `-Check` → `-Verify` を頭から回し直す。`release-cycle` は `tools/`・`.github/` に触らない約束で、この spec は版上げの 4 ファイル（`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`dist/README.txt`）に触らない＝重ならない。
  - `release-cycle` の手順や記録が道具の日本語の出力（例「判定 1〜8 すべて合」）を目印にしていれば、取り込んだ後に ASCII の文面へ読み替える（この spec からは `release-cycle` の文書を書き替えない）。
  - `winget-manifest-submission`: これから足す `winget.yml` は、この spec の読み方と「端末へ出す文は ASCII だけ・端末を書き替えない」に倣う。
  - `tools/test-all.ps1` の「crates.io 公開前の確認」は `tools/crates-io.ps1` を通る。撤去の後もそこが緑のままであることを期待する。
  - `doc/crates-io-publish.md` ほかの手順書: 2026-10-05 時点で文字コードの記述は無い。道具の出す文を引いている所があれば、ASCII に変えた文面に合わせる。
  - 3 か所の読み方を 1 つの出どころへ寄せるか写しのままにするかは設計で決める（brief の議題 1）。要件は「3 か所が同じ読み方で、同じ結果を出すこと」だけを求める。

## Requirements

### Requirement 1: 版の読み取りが端末の文字コードに依らない

**Objective:** As a リリースを回す開発者, I want 端末の文字コードが何であっても道具が同じ版を読むこと, so that 開発者の端末（コードページ 932）でも初回リリースの前提の確認が通る

#### Acceptance Criteria

1. When `tools/package.ps1` が版を読むとき, the パッケージの道具 shall 端末の文字コードがコードページ 932 でも 65001 でも、同じ版を読み取る。
2. When `tools/crates-io.ps1 -Verify` がワークスペースのクレートを読むとき, the 公開の道具 shall 端末の文字コードがコードページ 932 でも 65001 でも、同じ判定（合否と失敗の文の一覧）を出す。
3. When `release.yml` の段「版の検査」が版を読むとき, the 版の検査 shall その段の端末の文字コードが何であっても（段の先頭の書き替えは撤去する＝要件 3.1）、同じ版を読み取る。
4. The 版を読む 3 か所 shall どのクレートの `description` にどんな日本語の字の並びが入っていても、版を読み取れる。
5. The 版を読む 3 か所 shall 同じ入力から同じ版を読み取る（1 か所だけが端末に頼る読み方で残らない）。
6. If 版を読む子のプロセスが失敗する、または出力が JSON として読めない, the パッケージの道具 shall 今と同じ終了コード 3 で、今の失敗の文と同じ意味を ASCII で書いた文（要件 8）を出して止まる。
7. If 版を読む子のプロセスが失敗する、または出力が JSON として読めない, the 公開の道具と版の検査 shall 今と同じ終了コードで、今の失敗の文と同じ意味を ASCII で書いた文（要件 8）を出して止まる（公開の道具の「JSON として読めない」は今、失敗の文を持たず捕まえない例外として終了コード 1 で止まる。この止まり方も今のまま残す）。

### Requirement 2: 検体のパスの読み取りが端末の文字コードに依らない

**Objective:** As a リリースを回す開発者, I want 検体のパスを読む所も端末の文字コードに頼らないこと, so that 作業ツリーのパスに日本語が入っても `-Check` が止まらない

#### Acceptance Criteria

1. When `tools/package.ps1` が検体の窓口（`nar-sample-path`）の出力からパスを読むとき, the パッケージの道具 shall 端末の文字コードに依らず、子のプロセスが書いたとおりのパスを読み取る。
2. Where 読み取ったパスに日本語などの ASCII の外の字が含まれる, the パッケージの道具 shall そのパスで実在の確かめを行い、実在すれば通す。
3. If 求めた鍵が出力に無い、またはパスが実在しない, the パッケージの道具 shall 今の失敗の文と同じ意味を ASCII で書いた文（要件 8）を出して止まる。

### Requirement 3: 道具は端末の文字コードを書き替えない

**Objective:** As a 開発者, I want 道具が共有物である端末の設定を書き替えないこと, so that 道具を回した後の端末が回す前と同じに保たれる

#### Acceptance Criteria

1. The `tools/` のスクリプトと `release.yml`・`crates-io.yml` の段の本文 shall 端末の文字コードの設定（出力の文字コード・入力の文字コード・コードページ・PowerShell が子へ渡す文字コード）を書き替えない（workflow の各段の先頭の書き替えと、段「zip を作る」の子の中の書き替えも撤去する。`release.yml` の本文は手元でも回せる作りなので、残すと開発者の端末を書き替えるため）。
2. When 開発者が `tools/` の道具を回し終えたとき, the 端末 shall 回す前と同じ文字コードの設定のままである。
3. When `tools/crates-io.ps1` の冒頭の書き替えを取り除くとき, the 変更 shall 同じ変更（同じ PR）の中で、公開の道具の版の読み方の直し（要件 1.2）を含める（先に取り除くと公開前の確かめが赤になるため）。
4. When 書き替えを取り除いた後に `tools/crates-io.ps1 -Verify` を回すとき, the 公開の道具 shall 取り除く前に日本語で出していた人が読む行を、同じ内容のまま ASCII の文で出す（どの端末の文字コードでも化けない）。
5. When 書き替えを取り除いた後に `tools/crates-io.ps1 -Pending` を回すとき, the 公開の道具 shall 標準出力へ出すクレート名の並び（`crates-io.yml` が受け取るもの）を、取り除く前と同じにする。
6. When 書き替えを取り除いた後に `tools/test-all.ps1` の「crates.io 公開前の確認」を回すとき, the 全体テストの道具 shall 取り除く前と同じ結果（合否と終了コード）を出す。
7. While 端末の文字コードが ANSI・OEM のどのコードページであっても（932・437・65001 など。開発者の PC と CI のランナーでは違いうる）, when 開発者が自分で PowerShell を打って `tools/` の道具を回すとき, the 道具 shall 同じ結果（合否・読み取った値・終了コード）を出し、その後も開発者が同じ端末で続けて PowerShell を打てるよう、端末の設定を回す前のまま残す。

### Requirement 4: 同じ穴の棚卸と直し

**Objective:** As a 開発者, I want `tools/` のほかの所に同じ穴が残っていないことを確かめたい, so that 次に日本語が入った所でまた赤にならない

#### Acceptance Criteria

1. The 棚卸 shall `tools/` の中（`tools/perf/` を除く）のすべてのスクリプトについて、子のプロセスの出力を素で受けて字面・JSON・値として判定に使う所を挙げる。
2. When 棚卸で判定に使う所が見つかったとき, the 該当のスクリプト shall その所を要件 1・2 と同じ、端末の文字コードに依らない読み方にする。
3. Where 子の出力を人が読むために表示するだけの所, the 棚卸 shall その所を直さず、文字化けが見つかった場合は棚卸の結果と同じ所（この spec の文書）に記録を残す。
4. The 棚卸の結果 shall 調べたスクリプトの名前と、所ごとの扱い（直した・表示だけで対象外・ASCII しか出ないので対象外・字面に依らない（空かどうかや行の数だけを使う）ので対象外・端末を通らない（子にファイルへ書かせて UTF-8 で読む）ので対象外）を、0 件の場合も 0 件と明示して、この spec の文書に残す。

### Requirement 5: 直したことを判定で固定する

**Objective:** As a 開発者, I want コードページ 932 の条件で版が読めることを合否で判定したい, so that 読み方が端末に頼る形へ戻ったら気付ける

#### Acceptance Criteria

1. The 判定 shall 日本語の `description` を持つ今のワークスペースを、コードページ 932 の端末（またはそれと同じ条件を作った子のプロセス）から読み、版が取れれば合格、取れなければ不合格とする（表示するだけでなく合否を付ける）。
2. If 版を読む所が端末の文字コードに頼る読み方へ戻される, the 判定 shall 不合格になる。
3. The 判定 shall ワークスペースの `description` の字が書き替えられて、コードページ 932 で解いても壊れない並びになった後でも、要件 5.2 の不合格を出せる（今の `description` が偶然壊れることに頼らない）。
4. The 判定 shall 判定を回す端末の文字コードの設定を書き替えない。
5. The 判定 shall 判定を回す機械の ANSI・OEM のコードページが何であっても（開発者の PC でも CI のランナーでも）、同じ合否を出す。コードページ 932 の条件は、判定のために起こす窓の無い子のプロセスだけが持つ端末に作る（その子の端末は子だけの物で、開発者の端末と CI の段の端末は変わらない）。

### Requirement 6: 開発者の手元での確かめ

**Objective:** As a リリースを回す開発者, I want コンソールを Shift_JIS のままにして `-Check` を最後まで回したい, so that 版を読む段より後にも同じ穴が無いことを実物で確かめられる

#### Acceptance Criteria

1. While 端末の文字コードが Shift_JIS（コードページ 932）のまま, when 開発者が `tools/package.ps1 -Check` を回すとき, the パッケージの道具 shall 「前提の確認」を越え、最後の段まで緑で終わる。
2. While 端末の文字コードが Shift_JIS（コードページ 932）のまま, when 開発者が `tools/crates-io.ps1 -Verify` を回すとき, the 公開の道具 shall 端末を書き替えずに緑で終わる。
3. If 版を読む段より後の段で文字コードのために落ちる所が見つかる, the 直し shall その所を要件 4 の棚卸に加え、同じ読み方で直す。
4. The 確かめ shall 窓が開くため開発者の手元で行い、その結果（端末の文字コード・終了コード・通った段）をこの spec の文書に残す。

### Requirement 7: 変えないもの

**Objective:** As a 開発者, I want 直しの範囲の外にあるものが変わらないこと, so that 並走している初回リリースや既存の手順と衝突しない

#### Acceptance Criteria

1. The 直し shall `tools/package.ps1`・`tools/crates-io.ps1`・`tools/test-all.ps1` の終了コードの約束（`package.ps1` の 3＝引数・前提の誤り ほか）を変えず、失敗の文は意味を変えずに ASCII へ書き替えるだけにする（要件 8）。
2. The 直し shall `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`dist/README.txt` に触れず、版を上げず、依存を足さない。
3. The `release.yml` shall 「入力は環境変数だけ・手元でも同じ本文を回せる」形を保つ。
4. The 直し shall workflow の段の順・権限・待ち方を変えない。
5. The 道具と判定 shall 子のプロセスの環境変数を表示しない（秘密を印字しない）。

### Requirement 8: 道具が端末へ出す文は ASCII だけ

**Objective:** As a リリースを回す開発者, I want 道具と workflow が自分で端末へ出す文が ASCII だけであること, so that 開発者の PC でも CI のランナーでも、端末の文字コードを書き替えずに表示を読める

#### Acceptance Criteria

1. The `tools/` のスクリプト（`tools/perf/` を除く） shall 自分で端末へ出す文（進みの表示・失敗の文・判定の結果の行・投げる例外の文）を ASCII の字だけで書く。
2. The `release.yml`・`crates-io.yml` の段の本文 shall 自分で端末（ランナーのログ）へ出す文を ASCII の字だけで書く。
3. When 日本語の文を ASCII に書き替えるとき, the 書き替え shall 文の意味（どの段の・何が・どの値で失敗したか）を落とさず、終了コードを変えない。
4. Where 子のプロセスの出力をそのまま写す所, the 道具 shall その中身を書き替えない（中身は子の物で、この要件の外）。
5. If 要件 8.1・8.2 の文に ASCII の外の字が入る, the 判定 shall 不合格になる（表示するだけでなく合否を付ける）。
6. Where 手順書（`doc/` ほか）が道具の出す文を引いている, the 手順書 shall ASCII に変えた後の文面に合わせる。
7. The スクリプトの注記・workflow の段の名前・手順書の地の文 shall 端末へ出ないので、日本語のままでよい（書き替えの対象外）。
