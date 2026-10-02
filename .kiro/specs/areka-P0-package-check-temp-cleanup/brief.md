# Brief: areka-P0-package-check-temp-cleanup

> 2026-10-02 `/kiro-discovery` で起票（`alpha-release-signoff` の完了の手順の中・開発者指示「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」）。roadmap「alpha-release-signoff の持ち越し」節。出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6 と受入記録 `verification/acceptance-record.md` §8.6。**道具のバグ**（利用者には無関係・開発者の決まりに反する）。本文のソースの指し先は起票時（`c430480d`）の実測＝着手時に引き直すこと。

## Problem

- **開発者**: `tools/package-alpha.ps1 -Check` を回すたびに `%TEMP%` の下に `areka-alpha-check-<HHmmss>`（展開した zip）と `areka-alpha-check-<HHmmss>-logs`（起動の記録）が残り、誰も片付けない。2026-10-02 に開発者の求め（「あとさっきの、テンポラリにある余計なフォルダの削除もお願い。」）で、AI が 38 個を手で消した。
- 開発者の絶対ルール「実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ（`C:\` 直下も `C:\tmp` も不可）」に、道具の既定のふるまいが反している。しかも `-CheckDir` でワークツリーの `target\` を指すことができない（下）。

## Current State

- 展開先の親は `-CheckDir` が無ければ `[IO.Path]::GetTempPath()`、その下に `areka-alpha-check-<HHmmss>` を作る（「前提の確認」の段）。記録の置き場は展開先の名前に `-logs` を付けたもの（「短いパスへ展開」の段の `$script:LogDir`）。
- 展開先のフルパスの長さの上限 `EXPAND_DIR_MAX_CHARS = 160`（較正値の一覧）。短くする理由は完了 `alpha-package` の要件 3.1＝「リポジトリの外の新しい空の、パスの短い場所へ展開」——emo2 の SHIORI（pasta）が初回に `ghost/master/profile/` の奥へ書くファイルのパスが長すぎると、接続の失敗を出さずに黙る（同 spec の `research.md` §4.2）。ワークツリーのパス（例 `C:\home\maz\git\areka\.claude\worktrees\<名>\`）は深いので、上限に収まるかを着手時に実測する。
- **`-CheckDir` はリポジトリの中を断る**（終了コード 3「-CheckDir はリポジトリの外を指定する」）。理由はスクリプトのコメント「展開先はリポジトリの外（要件 3.1）。中だと target/ の下は git status でも捕まらない」＝完了 `alpha-package` の要件 3.1 と、最後の段「git status 不変の確認」の検査が `target\`（追跡外）の書き込みを捕まえられないこと。なお zip と途中物はもともと追跡外の `target/alpha/` に置いている。
- 後始末（`Invoke-Cleanup`）は `.zip.tmp` の削除と環境変数の復元だけで、展開先と記録は消さない。
- **記録は署名の根拠に引かれる**: `alpha-release-signoff` の受入記録 §1 は `-Check` の記録を生の記録の置き場へ `check-215146-logs\`・`check-221148-logs\` として写してから引いた。記録を残す手段は要る。

## Desired Outcome

- `-Check` の既定の展開先と記録の置き場が、ワークツリーの `target\` の下になる（160 字の上限を守る）。`%TEMP%` には何も残らない。
- 展開した木（起動した `areka.exe` が書いた記憶などを含む）は、判定の後に片付く。片付けを止めて残す手段（引数）がある。
- 起動の記録（`run.log`・`run.stderr.log`）は、署名や受入記録が引けるように残る（置き場を印字する）。
- 「git status 不変の確認」は今どおり効く（`target\` の下の書き込みは追跡外なので、検査の意味は変わらない）。
- 子のプロセスは自分が起こしたものだけを止める今の作り（番犬）を保つ。

## Approach

- 既定の親を `<リポジトリ>\target\alpha-check\` へ替え、`-CheckDir` のリポジトリの中を断る検査は `target\` の下に限って通す（要件 3.1 の理由＝「git status で捕まらない」は、片付けと記録の印字で埋める）。
- 判定の後に展開先を消し、記録だけを残す。消せなかったときは理由を印字して終了コードで知らせる（黙って残さない）。
- 完了 `alpha-package` の要件 3.1 を上書きすることになるので、本 spec の要件にその旨を書く。

## Scope

- **In**: `tools/package-alpha.ps1` の展開先・記録の置き場・後始末・引数の説明（`Get-Help` の欄）・較正値の一覧、`tools/` の自己検査があればその追随、`.kiro/steering/structure.md` などの道具の説明の追随。
- **Out**: zip の中身と判定の 8 項目・起動の記録の判定の 6 条件・`test-all.ps1`・性能改善ループの道具（登記だけの行「`tools/perf` の自己検査の赤」）。過去の記録（`alpha-release-signoff` の受入記録に書いた `%TEMP%` のパス）は書き換えない。

## Boundary Candidates

- 展開先と記録の置き場の決め方（「前提の確認」の段）
- 後始末（`Invoke-Cleanup` と `-Check` の各段）

## Out of Boundary

- 配布物の組み方・release のビルド・謝辞の生成。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。完了 `alpha-package` の上に建つ。
- **Downstream**: 次の配布（α の更新版）の `-Check` と、その署名の記録。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `alpha-package` の要件 3.1 を上書きする）。
- **Adjacent**: 登記だけの行「`tools/perf` の自己検査の赤」（別の道具）。

## Constraints

- **一時フォルダはワークツリーの `target\` の下だけ**（開発者の絶対ルール）。展開先のフルパスは 160 字以内（ワークツリーの深いパスでも収まることを確かめる）。
- 追跡しているファイルを 1 つも書き換えない（最後の段の検査を保つ）。プロセスは自分が起こしたと確かめたものだけを止める。
- `cargo`・`crates/` には触らない見込み。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **段はバグ（道具）・ウェーブ C1**。規模 XS〜S（3〜5 タスク）・Opus で足りる。
- brief の記述はすべて実物と一致した。**足りなかった事実**: スクリプトの `.EXAMPLE` が `-CheckDir C:\t` を勧めている（「一時フォルダはワークツリーの `target\` の下だけ」に反する）＝直す対象に足す。
- 長さの実測: ワークツリーの根が 64 字のとき `<根>\target\alpha-check\areka-alpha-check-HHmmss` は約 107 字＝emo2 の上限 160 字に収まる。
- **触るファイル**: `tools/package-alpha.ps1`（541 行）・`.kiro/steering/structure.md`（道具の説明）。`crates/` には触らない。
- **後ろに居る spec**: `mcp-stdio-bridge` が同じスクリプトへ中継 exe の同梱（`$ALLOWED_EXECUTABLES`・ビルドの段・配置と CPU 種別の検査）を足す＝本 spec が先。
- 小さな議題 2 つ: 判定が否のときに展開した木を証拠として残すか／消せなかったときの終了コードを新しく作るか。
