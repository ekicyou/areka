# Brief: areka-P0-install-live-target-hazards

> 2026-10-02 `/kiro-discovery` で起票（`alpha-release-signoff` の完了の手順の中・開発者指示「あ、起票はあとでやってくれますよね。「実装完了を承認」スキルは最後に実施しますし。その前提で、今は実装に戻ってください。」）。roadmap「alpha-release-signoff の持ち越し」節。出どころは `alpha-release-signoff` の完成判定 `verification/alpha-completion.md` §6 と受入記録 `verification/acceptance-record.md` §8.1 の ⑴⑵（もとは完了 `ghost-install` の `design.md`「Open Questions / Risks」の 1・2）。本文のソースの指し先は起票時（`c430480d`）の実測＝着手時に引き直すこと。

## Problem

- **利用者**: いま表示しているシェル・いま使っているバルーンの新しい版（`.nar`）を入れたとき、入れ替わるのか、失敗するのか、半端に入れ替わるのかが分からない。
- **利用者**: 起動中のゴーストの新しい版を入れている最中に Windows を終えると、そのゴーストのフォルダが消えたまま終わるかもしれない（元の中身は作業フォルダに 7 日残るが、利用者はそれを知らない）。
- どちらも α の一周では確かめられず、`dist/README.txt` には書いていない（推測で書かない＝`alpha-release-signoff` 要件 5.6）。

## Current State

- **⑴ 使用中のフォルダへの上書き**: シェルとバルーンの宛先は、ゴーストを降ろさずに入れる道を通る（`crates/areka/src/install/judge.rs` の `destination_of`＝種類が `Shell`・`Balloon` なら `Destination::Elsewhere`）。完了 `ghost-install` 要件 7.8 は「使用中で入れ替えられなければ要件 5 の失敗として扱う（宛先は元のまま）」と決めているが、**areka 自身（絵の読み込み・フォントなど）か SHIORI がそのフォルダのファイルを開いたまま掴んでいるかは、ソースからは決められない**（同 spec の設計 Risk 2）。実機の記録は 0 件（同 spec の `signoff.md` 項目 3 で確かめたのは、ゴーストを降ろして窓が 0 枚になった後の上書きだけ）。確定は `areka-nar` の `rename` 2 回で、掴まれていたときに「元のまま」で止まるのか、片方だけ動くのかも実機では未確認。
- **⑵ 起動中のゴーストへ入れる途中の Windows の終了**: 一周は「降ろす → 展開 → 起こし直す」。完了 `ghost-install` 要件 8.1〜8.3 は「終了の後始末に入った時点で展開中なら上限 3 秒待つ」を持つが、**展開の間は窓が 0 枚で、Windows の終了の知らせ（`WM_ENDSESSION`）を受ける窓が無い**（受け手は `crates/areka/src/session_end.rs` の `on_os_session_end`）。後始末が呼ばれずにプロセスが終わらされうる（同 spec の設計 Risk 1）。確定の 2 手の間で断たれると宛先のフォルダは無く、元の中身は `<根>/.nar-work/` の下に 7 日残る（`crates/areka-nar/src/install.rs` の `SURVIVOR_RETENTION`）。実機で測れたのは所要（降ろしてから展開し終えるまで 277 ms）だけ。
- 次の起動は今日どおり（前回のゴーストが見つからない `warn!` を残して次の候補へ進む＝`ghost-install` 要件 8.8）。

## Desired Outcome

- ⑴⑵ のそれぞれについて、**実機で測った事実**が記録に残っている（何が掴むのか／掴まないのか、`rename` のどちらで止まるか、Windows の終了で後始末が呼ばれるか、宛先と作業フォルダと次の起動がどうなるか）。
- 測った結果に応じて、扱いが決まり実装されている。候補は要件の段で開発者が選ぶ:
  - ⑴ 掴んでいるなら、入れる前にその掴みを外す（シェル・バルーンの切替の「古い装着を片付ける口」〔完了 `shell-balloon-switch`〕と同じ形で外して入れて付け直す）か、今の「失敗して元のまま・`OnInstallFailure`」を確かなものにして既知の制限へ。掴んでいないなら、そのことを記録して終える。
  - ⑵ 展開の間も Windows の終了を受け取れるようにする（窓 0 枚の区間に受け手を残すなど）か、既知の制限として説明書へ書く。
- `dist/README.txt` は、確かめた事実だけで直す（確かめる前は変えない）。
- 失敗はゴーストの台詞（`OnInstallFailure` など正典のイベント）で伝わり、メッセージボックスは出ない。

## Approach

- **まず測る**。実機の手順をワークツリーの `target\` の下の根（`AREKA_ROOT`）で組み、表示中のシェル・使用中のバルーンと同じフォルダへ入れる `.nar` を検体から作って落とす。判定の分岐（`rename` の成否・掴んでいるプロセス）は記録の level まで開けて読む。掴んでいるプロセスの特定は追加のダウンロードなしで行う（OS の再起動マネージャの API など・要件で決める）。
- ⑵ は、窓が 0 枚の区間を決定論のテストで固定し、Windows の終了の入口（`on_os_session_end` に至る経路）を同じ区間で当てる。実機で Windows を本当に終えるのは開発者の手で 1 回だけ（要るかは要件で決める）。
- 測った結果を見てから扱いを決める（議題は「答えで作業が変わる」ものだけ）。

## Scope

- **In**: ⑴⑵ の実機の手順と記録・決定論のテスト・決めた扱いの実装・`dist/README.txt` の既知の制限の追随・`doc/COMPAT_ARCHITECTURE.md` §8 への登記（扱いが正典の沈黙に当たるなら）。
- **Out**: `areka-nar` の確定の作り（`rename` 2 回）の作り直し・ネットワーク更新の背景の書き込み（同じ待ちの口を使うが、更新の側の危険は別に扱う）・同梱インストールの読み方（`install-companion-canon`）。

## Boundary Candidates

- 宛先の判定と「降ろさずに入れる」道（`crates/areka/src/install/judge.rs`・手続き）
- 掴みを外して付け直す口（present の装着の片付け）
- 窓 0 枚の区間の Windows の終了の受け手（`session_end.rs` と降ろして起こし直す一周）

## Out of Boundary

- 失敗の理由の語（正典の 3 値）と `OnInstall*` の送り方は変えない。
- 起動中の印の判定（`session_mark_verdict`）と、印を残す理由の語は変えない（完了 `ghost-install` 要件 7.9・8.7）。

## Upstream / Downstream

- **Upstream**: α の完成宣言（`alpha-release-signoff`）。完了 `ghost-install`・`file-drop`・`shell-balloon-switch`・`session-mark-residue` の上に建つ。
- **Downstream**: 第三者向け説明書 `dist/README.txt` の既知の制限。

## Existing Spec Touchpoints

- **Extends**: なし（完了 `ghost-install` の Risk 1・2 を引き受ける。完了 spec の要件を上書きするなら `doc/COMPAT_ARCHITECTURE.md` §8 に記す）。
- **Adjacent**: `install-companion-canon`（同じ `areka-nar` の読み方の側＝触るファイルを着手時に照合）・完了 `network-update`（終了で背景の仕事を待つ口を共有＝`ghost-install` 要件 8.10）・完了 `shell-balloon-switch`（装着の片付けの口）。

## Constraints

- **実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ**（開発者の絶対ルール・`C:\` 直下も `C:\tmp` も不可）。emo2 を使うときは絶対パスかつ 160 字の上限内。
- 確定の証跡は実機の記録と静的な構造（file と何の定義か）の二本立て。判定の分岐の記録の level まで `RUST_LOG` を開ける。
- 意味論は ukadoc から（SSP の実測には合わせない）。1 フレーム遅らせる解は取らない。決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。
- 説明書は確かめた事実だけで書く。
