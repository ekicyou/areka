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
- **Out**: `areka-nar` の確定の作り（`rename` 2 回）の作り直し・ネットワーク更新の背景の書き込み（同じ待ちの口を使うが、更新の側の危険は別に扱う）・同梱インストールの読み方（`install-companion-reading`）。

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
- **Adjacent**: `install-companion-reading`（同じ `areka-nar` の読み方の側＝触るファイルを着手時に照合）・完了 `network-update`（終了で背景の仕事を待つ口を共有＝`ghost-install` 要件 8.10）・完了 `shell-balloon-switch`（装着の片付けの口）。

## Constraints

- **実機の根・検体・一時フォルダはワークツリーの `target\` の下だけ**（開発者の絶対ルール・`C:\` 直下も `C:\tmp` も不可）。emo2 を使うときは絶対パスかつ 160 字の上限内。
- 確定の証跡は実機の記録と静的な構造（file と何の定義か）の二本立て。判定の分岐の記録の level まで `RUST_LOG` を開ける。
- 意味論は ukadoc から（SSP の実測には合わせない）。1 フレーム遅らせる解は取らない。決定論のテスト網羅は必達。ログの無い失敗の経路を作らない。
- 説明書は確かめた事実だけで書く。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **C4 の候補**（10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」）で段は「その他」。`dist/README.txt` を C2 の `crates-io-publish`・C3 の `winget-manifest-submission` と分け合うので、その後）**・Fable 推奨**。規模 S〜M（8〜14 タスク。測るだけで終われば 4〜5）。
- brief の記述はすべて実物と一致した。
- **静的に分かったこと（⑴ 掴み・仮説）**: 絵は `crates/areka-emo-atlas/src/decode/wic_arm.rs` が `CreateDecoderFromFilename` → `CopyPixels` で自前の領域へ写し、関数の終わりで手放す。DirectWrite へフォントファイルを登録する API は使っていない。areka がシェル・バルーンのファイルを開いたまま持ち続ける道は静的には見当たらない（読み込みの最中の一瞬は在りうる）。**実機で確かめる価値は残る**（SHIORI の側が掴む道は別）。
- **静的に分かったこと（⑵ 窓 0 枚・仮説）**: host-32 の結線が作る `ParentMessageWindow`（`crates/areka-ghost/src/shiori_wiring.rs`）は message-only の窓で、`WM_ENDSESSION` の一斉配信は届かない見込み＝窓 0 枚の区間に受け手は居ない。
- **触るファイル（測った結果で変わる）**: `crates/areka/src/install/{judge.rs, procedure.rs, overwrite.rs, desk.rs}`・`crates/areka/src/session_end.rs`・`crates/areka/src/app_exit.rs`・テスト（`install/desk_overwrite_tests.rs`・`session_end_tests.rs`・`session_end_deadline_tests.rs`）・`dist/README.txt`・`doc/COMPAT_ARCHITECTURE.md`。
- **同じウェーブの他の spec との約束（必ず守る）**: `crates/areka-nar/`（`install-companion-reading`）・`crates/areka/src/install/terms.rs`（同）・`crates/areka/src/emo2_boot/frame/drain_resnap.rs`（`restart-chain-finalize-stall`）・`crates/areka/src/main.rs` と `ghost_session.rs`（`mcp-server-core` と、その次の `mcp-tool-entrances`）には触らない。扱いがこれらに及ぶと分かったら、そこで止めて報告する。
- **議題**: ⑴ 掴みを外して入れるか既知の制限にするか（測った後）／⑵ 窓 0 枚の区間に受け手（隠れたトップレベルの窓など）を残すか説明書に書くか／Windows を本当に終える実機の 1 回を開発者が行うか。
- 関連: 起こし直しの後の `deferrals=600` の WARN は別 spec `restart-chain-finalize-stall` が直す（同じ場面の実機でこの WARN を見ても本 spec では追わない）。利用条件の文の切り詰めが絵文字を割る件は棚卸⑳で直した。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S〜M（8〜14 タスク。測るだけで終われば 4〜5）。変わらず。
- 前提の状態: 前提の spec は無い（α の完成宣言のみ）＝満たす。ただし下の「`balloon-font-file` の後に測る」を守るなら、C3 の `balloon-font-file` の着地を待つ。
- 崩れた前提／古くなった位置:
  - **`dist/README.txt` の共有**: `release-ci-workflow`・`crates-io-publish` は着地済み（PR#224・PR#225）。`crates-io-publish` は「入手のしかた」の節を足した（`release-ci-workflow` は `dist/README.txt` に触らなかった・「深いフォルダに展開しないで」の行を「既知の制限」から外したのは README の書き直し PR#222）＝この 2 本との共有は解けた。**残る相手は 4 本**: `winget-manifest-submission`（C3-①・「既知の制限」に Smart App Control とスタートメニューの 2 行を足す＝本 spec と同じ節）・`release-cycle`（「時点」の行）・`release-code-signing`（「既知の制限」の「署名なし」の行）・`mcp-stdio-bridge`（C4 の候補・要れば）。同じ節の行の足し引きなので、並べるなら本 spec の `dist/README.txt` の変更は最後のタスクに寄せ、着地の前に rebase する。
  - 「既知の制限」には `.nar-work` に 7 日残る旨の行がすでに在る（`dist/README.txt`）。⑵ で説明書へ書く場合は、この行への追記で足りるかを先に見る。
  - `crates/areka/src/install/` で棚卸⑳の後に変わったのは `fetch_url.rs`（と兄弟のテスト）・`mod.rs` 1 行・`terms.rs`・`worker_tests.rs`（`OnTranslate` が続くようになった・PR#226）。本 spec の触る `judge.rs`（`destination_of` は `Shell`・`Balloon` を `Destination::Elsewhere`）・`procedure.rs`・`overwrite.rs`・`desk.rs`・`session_end.rs`（`on_os_session_end`）・`app_exit.rs`・`crates/areka-nar/src/install.rs`（`SURVIVOR_RETENTION` 7 日）は変わっていない。
  - `install-companion-reading` とは同じ `crates/areka/src/install/` を使うが、向こうは `terms.rs` だけ＝ファイルの重なり 0（向こうが `procedure.rs`・`judge.rs` を無改変で済ませる限り）。`crates/areka-nar/` は本 spec では触らない約束のまま。
  - **⑴ の仮説が C3 で古くなる**: 棚卸⑳の「areka がシェル・バルーンのファイルを開いたまま持つ道は静的に見当たらない」（WIC は `decode/wic_arm.rs` で読み終えて手放す・DirectWrite へのフォントファイルの登録は無い）は今も正しい。ところが C3 の `balloon-font-file` は、バルーン・シェル・ゴーストのフォルダのフォントファイルを DirectWrite のフォントセット（`AddFontFile` → `CreateFontCollectionFromFontSet`）へ載せる計画で、フォント集が生きている間はそのファイルを areka が掴みうる。＝⑴ は `balloon-font-file` の着地の後に測らないと、測った事実がすぐ古くなる。
- 触るファイル（測った結果で変わる・並走の照合用）:
  - `crates/areka/src/install/{judge.rs, procedure.rs, overwrite.rs, desk.rs}` と兄弟のテスト（`desk_overwrite_tests.rs` ほか）
  - `crates/areka/src/session_end.rs`・`app_exit.rs` と `session_end_tests.rs`・`session_end_deadline_tests.rs`
  - 掴みを外すなら `balloon-font-file` が作るフォント集の片付けの口（`crates/areka-emo-text/src/draw_catalog.rs` の周り）
  - `dist/README.txt`・`doc/COMPAT_ARCHITECTURE.md`
- 議題（答えで作業が変わるものだけ）: 棚卸⑳の 3 つ（⑴ 掴みを外して入れるか既知の制限にするか／⑵ 窓 0 枚の区間に受け手を残すか説明書に書くか／Windows を本当に終える実機の 1 回を開発者が行うか）に加え、**⑴ を `balloon-font-file` の着地の後に測るか**（推し: 後。C4 の候補の席はもともと C3 の後なので、並びは変えずに済む）。
- 見つけた穴: 無し（新しいバグの候補は見当たらない。フォントファイルの掴みは `balloon-font-file` の設計で片付けの口を持たせれば済む＝その spec の着手時に申し送る）。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S〜M（8〜14 タスク。測るだけで終われば 4〜5）。切る: なし。
- 前提の状態: 前提の spec は無い。棚卸㉑の推し「⑴ は `balloon-font-file` の着地の後に測る」を守るなら**待ち**（`balloon-font-file` は C4 の予定で未着手）。⑵ だけなら今すぐ取れる。
- 崩れた前提／古くなった位置:
  - C3 で本 spec の触るファイルに入った変更は `crates/areka/src/install/terms_tests.rs` だけ（`install-companion-reading`）。`judge.rs`（`destination_of` は今も `Shell`・`Balloon` を降ろさずに入れる道へ送る）・`procedure.rs`・`overwrite.rs`・`desk.rs`・`session_end.rs`（`on_os_session_end`）・`app_exit.rs` は 0 行。`crates/areka-nar/src/install.rs` の `SURVIVOR_RETENTION`（7 日）も変わらない。
  - `install-companion-reading` が `crates/areka-nar/src/{manifest,plan,names}.rs` を変えた（同梱の探索の順と、階層つきの取り出し元）。確定の `rename` 2 回の作り（`install.rs`）には触れていない＝⑴ の「どちらの `rename` で止まるか」の問いはそのまま。
  - ⑴ の仮説に 1 か所の読み口が増えた: `animated-image-decode` が動く絵を `crates/areka-emo-atlas/src/decode/image_arm.rs` で読む（`std::fs::File` を `BufReader` で開き、コマを読み終えると関数の終わりで手放す）。WIC の `wic_arm.rs` と同じく読む間だけ掴む形で、持ち続ける道は増えていない。
  - `dist/README.txt` には「■ 動く絵の上限」の節が足されたが、「■ 既知の制限」の `.nar-work` に 7 日残る旨の行はそのまま。
- 触るファイル（並走の照合用・測った結果で変わる）: 棚卸㉑のまま＝`crates/areka/src/install/{judge.rs, procedure.rs, overwrite.rs, desk.rs}` と兄弟のテスト・`crates/areka/src/session_end.rs`・`app_exit.rs` とそのテスト・（掴みを外すなら）`balloon-font-file` が作るフォント集の片付けの口・`dist/README.txt`（「■ 既知の制限」）・`doc/COMPAT_ARCHITECTURE.md`。
- 共有しうる相手: 「■ 既知の制限」を触る `winget-manifest-submission`・`release-code-signing`・`update-check-options`（同じ節の別の行）。`crates/areka-nar/` は今も触らない約束。
- 議題（答えで作業が変わるものだけ）: 棚卸㉑のまま（⑴ 掴みを外すか既知の制限か／⑵ 窓 0 枚の区間に受け手を残すか説明書か／Windows を本当に終える実機の 1 回を開発者が行うか／⑴ を `balloon-font-file` の後に測るか）。加えて、⑵ だけを先に（`balloon-font-file` を待たずに）回すか。
- 見つけた穴: なし。
