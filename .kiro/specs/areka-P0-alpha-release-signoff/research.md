# ギャップ分析: areka-P0-alpha-release-signoff

> 2026-10-01 実施（`kiro-validate-gap`）。対象は確定済みの `requirements.md`（要件 1〜7）と、ワークツリーの HEAD `07e717f3`（main `238db25d`＝`shell-balloon-switch` の着地の直後＋本 spec の初期化）。
> 本文の file:line はこの時点の実測。設計の段で引き直すこと。
> 並走: バグ `balloon-reappear-short-talk` が別のセッションで直されており、本 spec の実装より先に main へ入る見込み。その着地に依る条項は §6 にまとめた（main の取り込みで突き合わせやすくするため）。

---

## 1. 分析の要約

- **本 spec は「作る」より「確かめて書く」仕事が大半**で、使う道具はほぼ全部そろっている。配布 zip は `tools/package-alpha.ps1` が 1 コマンドで組んで中身と起動を判定し、全体テストは `tools/test-all.ps1`、検体は `vendors/sample_ghost/*.nar` と `nar-sample-path`、受入記録と完成判定の器は M1 の完了 `areka-P0-emo2-conformance-e2e/verification/` に写し元がある。`crates/` のソースに手を入れる必要は見つからなかった（要件の「変更 0」と一致）。
- **要件と実物の食い違いが 3 件ある**（どれも設計の前に答えが要る）: ⑴ `BUILD-INFO.txt` の `commit=` は 7 桁で、要件 1.2 の「完全なコミットの識別子」と合わない ⑵ 検証項目 12 の期待（初回だけの位置合わせが 2 回目に既定へ戻って**しまわない**）は、M1 の設計が書いた許容の形（2 回目は既定の配置へ**戻る**）と向きが逆で、どちらが今のコードの振る舞いかを確かめていない ⑶ 既定バルーンの出典文（§6.2）を言い回しそのままに写すと、「バルーンが 1 つ入っています」が zip の実物（バルーン 2 つ）と食い違う。
- **説明書 `dist/README.txt` の偽の記述は 7 か所**（日付・メニューの項目数・「シェル・バルーンの切り替えは出ません」2 か所・記憶の置き場が えも？？ 専用の書き方・2 回目の起動の記述にシェルが無い・「.nar の入れ方」が未記入）。「更新のしかた」と窓への投げ込みの記述は 0 行で、新しく書く。
- **開発者の手の作業（`emo2` の `halt` の台詞）と、検証項目 8・13 の順序が絡む。** 今の `emo2.nar` は配布サイトと 24 件ずれているので、項目 8 は手を加えなくても差分が出る。逆に `emo2.nar` を最新に差し替えると差分は 0 になり、項目 8 は手で差分を作る形になる。さらに項目 8 は一周の根の `emo2` を配布サイトの版で上書きするので、項目 13 が見る `emo2` は zip の版ではなくなりうる。
- **規模は S〜M・危険度は低〜中。** 危険の源はコードではなく、実機（拡大率の違う画面・本物の配布サイト）と、リポジトリの外の作業（`halt` の台詞）と、並走するバグの着地の時期である。

---

## 2. 今あるもの（Current State）

### 2.1 配布 zip（`tools/package-alpha.ps1`・541 行・版 1.0.0）

| 事実 | 出どころ |
|---|---|
| 版は `1.0.0`（`BUILD-INFO.txt` の `script=` に書く） | `tools/package-alpha.ps1:65` |
| コミットは **7 桁固定**で取る（`git rev-parse --short=7 HEAD`） | `tools/package-alpha.ps1:196` |
| `dirty=` は始めの `git status --porcelain` の行数（追跡外のファイルも数える） | `tools/package-alpha.ps1:198-199` |
| `BUILD-INFO.txt` は `commit=`・`dirty=`・`built=`（UTC）・`script=`・`rustflags=` の 5 行 | `tools/package-alpha.ps1:324-331` |
| zip の名前は `areka-alpha-x64-<yyyyMMdd>-<7 桁>[-dirty].zip`、置き場は `target/alpha/` | `tools/package-alpha.ps1:335-336` |
| 許可表: 最上位 8 項目・`ghost/` は `emo2` だけ・`balloon/` は `emo2-kakukaku` と `StayseeBalloon` | `tools/package-alpha.ps1:374-378` |
| 実行ファイルの許可表は `areka.exe`・`shiori-host32-helper.exe`・`ghost/emo2/ghost/master/pasta.dll` の 3 本 | `tools/package-alpha.ps1:70` |
| 判定 7: zip の `emo2` の説明書 2 本は `vendors/sample_ghost/emo2.nar` の中身とバイトが同じ | `tools/package-alpha.ps1:403-411` |
| 判定 8: `README.txt` は `dist/README.txt` と、謝辞は `target/alpha/THIRD-PARTY-NOTICES.md` とバイトが同じ／`BUILD-INFO.txt` の `commit=`・`dirty=` | `tools/package-alpha.ps1:412-422` |
| `-Check` は初回のバルーンが `route=Companion` で `\balloon\emo2-kakukaku` であることを判定する | `tools/package-alpha.ps1:76-78`・`:463-471` |
| `-Check` の展開先はリポジトリの外でなければ断る | `tools/package-alpha.ps1:189-193` |
| 実機の走行中に回すと、同じ検体の展開した木（`target/nar-samples/manual/<検体>/`）を消す | `tools/package-alpha.ps1:12-13` |
| 謝辞は x64 と i686 の 2 ターゲットに絞って `target/alpha/` に作る（リポジトリの `THIRD-PARTY-NOTICES.md` とは別の生成物） | `tools/package-alpha.ps1:282-287` |
| `Cargo.lock` は**追跡済み**（`git ls-files Cargo.lock` が 1 行・`.gitignore` に `Cargo.lock` の行は無い）。brief の 09-26 の項目 9「`Cargo.lock` は追跡外」は `alpha-package` で解消済み | `alpha-package` の design.md:39 |

### 2.2 第三者向け説明書（`dist/README.txt`・105 行・UTF-8 の BOM つき）

| 行 | 今の記述 | 実物 | 要件 |
|---|---|---|---|
| 5 | 「2026-09-26 時点の内容です」 | 古い | 4.1 |
| 13 | 「2 回目からは、前回使ったゴーストと吹き出しで立ちます。」 | シェルも記憶から戻る（完了 `shell-balloon-switch`）。きれいに終わらなかった次の起動は えも？？ で立つ記述が無い | 4.5 |
| 26 | 項目は 5 つ | 7 つ（`crates/areka/src/menu/mod.rs:64-72` の `Frame::ORDER`、既定の名前は `crates/areka/src/menu/captions.rs:30-40`）。シェル枠は「1 つだけでも枠は出す」（`crates/areka/src/menu/shell_frame.rs:3-4`） | 4.2 |
| 33 | 「シェル・バルーンの切り替えの項目は、今の版ではメニューに出ません。」 | 偽 | 4.2 |
| 40-42 | 記憶の置き場を `ghost\emo2\...` で書く | ゴーストごとの記憶は `<ゴーストの SHIORI のフォルダ>\profile\areka\`、シェルの記憶は `<シェルのフォルダ>\profile\areka\`（`crates/areka-ghost/src/sylphya_wiring.rs:83-90`）。アプリの記憶は exe の隣の `profile\areka\`（`crates/areka/src/boot_config.rs:291-299`） | 4.7 |
| 52 | 「α 版の時点では、次のことはできません: シェル・バルーンの切り替え。」 | 偽 | 5.5 |
| 55-57 | 「■ .nar の入れ方」は未記入の目印だけ | メニューの「インストール…」と窓への投げ込み（`crates/areka/src/input_events/file_drop.rs:79` が受け取りの記録）の 2 経路が在る | 4.3 |
| — | 「■ 更新のしかた」の欄が無い | 完了 `network-update` | 4.4 |
| — | 既定バルーンの出典文（§6.2）が無い | `default-balloon-bundle` の `verification/signoff-record.md:922-956` | 4.6 |

正しいままの記述（確かめた範囲）: 29 行目の「インストール…」の説明（利用条件の箱は `crates/areka/src/install/terms.rs:10` の `terms.txt`／`terms.md`）、28 行目の「ネットワーク更新」の説明、60-105 行目の「同梱物とライセンス」（`emo2.nar` を差し替えるなら要件 6.3 で引き直す）。

### 2.3 アプリの振る舞い（説明書と検証項目の期待の裏付け）

| 事実 | 出どころ |
|---|---|
| 「ゴーストが見つかりません。」の告知は `MB_OK` の箱で、本文は 3 行（何が無いか・置く場所・置くもの） | `crates/areka/src/alert.rs:77`・`:100-112`・`:180`、置くものの文は `:60` |
| 既定ゴーストのフォルダ名は `emo2` | `crates/areka/src/boot_resolve.rs:26` |
| きれいな終わりの判定は `session_mark_verdict`。引数で始めたプロセスは印に触れない・有界の自動終了（`ExitOrigin::Smoke`）はきれいな終わりに数える | `crates/areka/src/main.rs:619-660`（`Smoke` は `:646`、発火は `:277`） |
| 前回きれいに終わらなかった次の起動の `OnBoot` には `halt` と落ちたゴーストの名前が載る | `crates/areka-kanade/src/schedule/events.rs:279`・`crates/areka/src/main.rs:429-430` |
| 台本の入口 `\![open,readme]` は登記済み | `crates/areka/src/emo2_boot/consumer_ledger.rs:325` |
| キャラの窓は 2 人分だけ（`derive_scopes()` が `[0, 1]` 固定） | `crates/areka/src/emo2_boot/mod.rs:274-276` |
| 更新のオプション 3 語は受けない | `crates/areka/src/emo2_boot/update_cue.rs:32` |
| 取得したファイルは 7 日で掃除 | `crates/areka/src/install/fetch_url.rs:25` |
| 入れる途中で戻せなかった元の中身は 7 日まで残す | `crates/areka-nar/src/install.rs:40`・`crates/areka/src/install/procedure.rs:400` |

### 2.4 検体（`vendors/sample_ghost/`）

| 検体 | 事実 |
|---|---|
| `emo2.nar` | 110 項目・最後の変更 `a7b7eb19`。`halt`・`Reference6` は UTF-8／CP932 のどちらで読んでも 0 件（本分析で再測定）。同梱バルーンの `emo2-kakukaku/descript.txt` の `homeurl` は `https://raw.githubusercontent.com/ekicyou/emo-gs/stable/ghost/emo-gs/emo-kakukaku/`（古い更新先のまま）。ゴーストの `homeurl` は `https://ekicyou.github.io/ghost_dev/emo2/emo2/`。`install.txt` は `balloon.directory,emo2-kakukaku` |
| `vendors/sample_ghost/README.md:7` | `emo2.nar` の行＝110 項目・4,560,408 バイト（要件 6.1 で直す行） |
| `R_POST_and_KOMAINU.nar` | 49 項目・`terms.txt` 無し・シェル 1 つ（`shell/master`） |
| `claudia.nar` | 135 項目・`terms.txt` 無し・同梱バルーン 2 つ（`balloon0.directory,claudia`・`balloon1.directory,claudia_vertical`）。入れると `balloon/` の一覧が 4 つに増える |
| `konnoyayame.nar` | 125 項目・`terms.txt` 無し |
| **利用条件のファイルを持つ検体は 0 体** | 項目 4 の「利用条件の画面が出たときは『はい』」は一周では踏まれない |
| `emo2` に依るテストのファイルは 28 本（`SampleRoot::acquire("emo2")`／`manual_paths("emo2")` の呼び手） | 例: `crates/areka-emo-text/tests/kero_menu_capacity_test.rs:19`（実物の `menu.pasta` の選択肢 3 本を読む）・`crates/areka/tests/emo2_real_run.rs`。展開済みの別の写し（旧 `crates/pilot/examples/shiori-host-32/fixtures/`）はもう無い＝`emo2.nar` を差し替えれば全部が新しい版を読む |

### 2.5 写し元の器（M1）

- `completed/areka-P0-emo2-conformance-e2e/verification/` に `acceptance-record.md`・`lap-procedure.md`・`m1-completion.md`・`isolation-decision.md`。
- `m1-completion.md` は §0 前提・§2 全体テスト・§3 許諾と謝辞・§4 実機サインオフ（人間の記入欄）・§5 宣言・§6 未達と引受先・§7 次の段階の起点。持ち越しの表は `m1-completion.md:101-110`。
- 検証項目 12 の M1 での扱い: 設計の項目表は「初回起動限定の位置調整は 2 回目以降の起動では既定配置へ戻る（許容仕様の裁定を実機で最終確定する）」（`completed/areka-P0-emo2-conformance-e2e/design.md:277`）。受入記録は「未観測」で閉じた（`acceptance-record.md:822`・`:826`）。未観測の原因は、走行のあいだに `cargo test` が**リポジトリの中の**検体のフォルダの `profile\areka\` を消したこと（`acceptance-record.md:239`）。α の一周の根はリポジトリの外の zip の展開先で、今の決定論テストは `SampleRoot` の一時の写しを使うので、同じ形の事故は構造上起きにくい（要件 3.3 の順序は守るとしても安い）。

### 2.6 全体テストと謝辞

- `tools/test-all.ps1` は x64 全テスト（`:45`）・i686 テスト（`:46`）、`-License` のとき `cargo deny check` と `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（`:48-49`）を回し、謝辞に差分があれば黄色で知らせる（`:60-63`）。検査したコミットは 7 桁の短い形で出る（`:31`）。
- 完了の手順（`.kiro/steering/workflow.md:45`）は、アーカイブを**コミットした後**に `test-all.ps1 -Format -License` を 1 回回す。要件 7.1 ⑴ の「署名の根拠のコミットで全段成功」とは回す時点が別である。

---

## 3. 要件と資産の対応（ギャップの印: 欠け／不明／制約）

| 要件 | 使える資産 | ギャップ |
|---|---|---|
| 1.1 zip を全段成功＋`-Check` 合格で組む | `tools/package-alpha.ps1`（`-Check`） | なし |
| 1.2 `commit=`（完全な識別子）と `dirty=0` を写す | `BUILD-INFO.txt` | **欠け／制約**: `commit=` は 7 桁（`tools/package-alpha.ps1:196`・`:325`）。「完全な識別子」を満たすには、スクリプトを直すか、記録で 7 桁を `git rev-parse` で完全な形へ引き直して添えるか（議題 D1）。`dirty=` は追跡外のファイルも数えるので、記録の下書きをコミット前に置いたまま組むと 0 にならない |
| 1.3 一周の後に中身を変えたら組み直して採り直す | 同上 | なし（手順の問題）。§6 のバグの着地の時期と絡む |
| 1.4 `ghost/` は `emo2` だけ | 許可表 `tools/package-alpha.ps1:376` | なし（議題 3 が「入れる」なら許可表 2 か所と判定 1 の必須の項目を変える） |
| 1.5 x64 の 1 種 | スクリプトは x64 だけ | なし |
| 2.1 12 項目 | 各完了 spec の実機の記録（`network-update`・`shell-balloon-switch` の `signoff.md`・`session-mark-residue` の `signoff.md`） | **不明**: 項目 12 の期待の向き（議題 D2）。項目 6 の 2 つ目のシェルは手で作る（作り方は `shell-balloon-switch` の `signoff.md:14-18`＝`shell\master` を写して `descript.txt` の `name` を変える）。項目 8 の差分の出方は `emo2.nar` を差し替えるかで変わる（議題 D4） |
| 2.2 `\![open,readme]` と「左クリックの後の右クリック」 | 入口は登記済み（`consumer_ledger.rs:325`） | **不明**: どのゴーストの辞書の写しに 1 行足すか（`emo2` は pasta、`R_POST_and_KOMAINU` は里々＝書きやすい）。設計で決める |
| 2.3 項目 13 | `session_mark_verdict`（`main.rs:619-660`）・`events.rs:279` | **制約**: 項目 8 の後に回すと、根の `emo2` は配布サイトの版に替わっている（議題 D4） |
| 2.4・2.5 1 分以内・確かめた期待だけ | — | 項目 12 の期待は今のコードで未確認（議題 D2） |
| 3.1 ふつうの権限・引数なし・短いパス・`C:\` 直下でない | — | **制約**: 一周の根の置き場。開発者方針は「ワークツリーの `target\` が第一・次善 `C:\tmp`」。ワークツリーの `target\` に置くとワークツリーの片付けで zip も記録も消える（M1 は `C:\home\maz\lap-records\` に生の記録を残した＝`m1-completion.md:46`） |
| 3.2 環境変数を渡さない・記録の水準を開ける・自動終了は安全弁 | `-Check` は `AREKA_*` を外して 4 つだけ入れる（`package-alpha.ps1:489-495`） | 一周の起動は `-Check` とは別の手順で、`RUST_LOG` を判定の分岐の水準（trace まで）に開ける起動の書き方を設計で決める。`AREKA_NO_ALERT` は項目 1 で外す |
| 3.3 項目 12 の前に位置を消すテストを回さない | — | 構造上の危険は小さい（§2.5）。順序の規則として手順書に書くだけ |
| 3.4〜3.9 受入記録の形 | M1 の `acceptance-record.md` | なし（写す）。項目表と手順の置き場（M1 は `lap-procedure.md` を別に持った）は設計で決める |
| 4.1〜4.5・4.7〜4.9 説明書 | `dist/README.txt` | **欠け**: §2.2 の 7 か所＋新しい欄 1 つ。「吹き出し」とメニューの「バルーン」の呼び名が混ざっている（4.8 の読みやすさ） |
| 4.6 §6.2 をそのまま写す | `signoff-record.md:922-956` | **制約**: ⑴ 本文の「バルーン…が 1 つ入っています」は zip の実物（`emo2-kakukaku` と `StayseeBalloon` の 2 つ）と食い違う＝4.7 とぶつかる ⑵ 本文は Markdown（見出し `###`・表・`<br>`・太字 `**`・コードの印）で、説明書はプレーンテキスト。どこまでが「記法を合わせる」でどこからが「言い回しの書き換え」かの線引き（議題 D3） |
| 5.1〜5.3 既知の制限 | §2.3 の各行 | なし（各行の裏付けは見つかった） |
| 5.4 条件つきの制限 ⒜⒝⒞ | `emo2.nar` の中身・main の状態 | ⒞ は §6 |
| 5.6 確かめられない候補の登記 | — | なし（記録の書き方） |
| 6.1〜6.5 `emo2.nar` の差し替え | 前例＝2026-09-20 の差し替え | **不明／危険**: 新しい `emo2` の台本で、実物の台本を読むテスト（`kero_menu_capacity_test.rs` など 28 本）が赤になりうる。`install.txt` の `balloon.directory` が変わると `-Check` の判定（`package-alpha.ps1:76-78`）が否になる |
| 7.1〜7.8 完成判定の文書 | `m1-completion.md` | **制約**: ⑴ の全体テストを「署名の根拠のコミット」で回すことと、完了の手順のアーカイブ後の 1 回（`workflow.md:45`）の関係（議題 D6）。⑵ の謝辞は `test-all.ps1 -License` で作り直したリポジトリの `THIRD-PARTY-NOTICES.md` が差分 0 であること。zip の謝辞（`target/alpha/` で 2 ターゲットに絞って生成）とは別物で、両者がバイトで同じとは限らない |

---

## 4. 実装の進め方の候補

本 spec はコードを作らないので、選択肢は「どこまで既存の道具を直すか」と「一周をどう組むか」の 2 軸になる。

### 案 A: 既存の道具をそのまま使う（スクリプトは触らない）

- `tools/package-alpha.ps1` は変えない。要件 1.2 の「完全な識別子」は、受入記録に `BUILD-INFO.txt` の 7 桁をそのまま写し、横に `git rev-parse <7 桁>` で引いた 40 桁を添えて満たす。
- 新しく作るのは文書だけ: `verification/acceptance-record.md`（項目表・手順・結果）・`verification/alpha-completion.md`、`dist/README.txt` の仕上げ、条件しだいで `emo2.nar` と `vendors/sample_ghost/README.md`。
- ✅ 変えるファイルが最少・`alpha-package` の判定を一切動かさない
- ❌ `commit=` の行そのものは 7 桁のままで、要件 1.2 の字面（「`commit=`（完全なコミットの識別子）」）とずれる。読み替えを記録に書く必要がある

### 案 B: スクリプトを小さく直す

- `tools/package-alpha.ps1` の `BUILD-INFO.txt` の `commit=` を 40 桁にし（zip の名前は 7 桁のまま）、版を `1.0.0` → `1.1.0` に上げる。判定 8 の `commit=` の照合も 40 桁に合わせる。
- 一周の起動を補う小さなスクリプト（展開・`RUST_LOG` を開けた起動・記録の置き場）を足す案もあるが、M1 は手順書（`lap-procedure.md`）と手の操作で回しており、足すほどの利得は薄い。
- ✅ 要件 1.2 を字面どおり満たす・後から zip だけを見ても完全な識別子が分かる
- ❌ `crates/` の外とはいえ、完了 spec `alpha-package` の設計（`design.md:328`「`commit=<7 桁>`」）と違う形になる。版の上げ方と判定の直しを 1 つの変更で行う必要がある

### 案 C: 混ぜる（推しの形の候補）

- スクリプトの直しは要件 1.2 のための 1 点（`commit=` の桁）だけにするか、案 A の読み替えにするかを議題 D1 で決める。それ以外はすべて案 A（文書と検体だけ）。
- 一周の手順は M1 と同じく手順書＋手の操作で、自動化は `-Check` の既存の判定に任せる。
- ✅ 変更の量を最小に保ちつつ、要件の字面との食い違いを残さない
- ❌ 議題 D1 の答えを待つ

---

## 5. 規模と危険度

- **規模: S〜M（5〜7 タスク）**。書く文書が 3 本（受入記録・完成判定・説明書）と、条件つきの検体の差し替え。実機の一周そのものは開発者の時間で、拡大率の違う画面（項目 11）と本物の配布サイト（項目 8）が要る。
- **危険度: 低〜中**。コードの変更は 0〜1 点で、既存の型に沿う。中に寄せる要因は、⑴ リポジトリの外の作業（`halt` の台詞）の到着時期、⑵ `emo2.nar` を差し替えたときに実物の台本を読むテストが赤になりうること、⑶ 並走するバグの着地の時期と zip を組む時期の前後（§6）、⑷ 項目 12 の期待がまだ確かめられていないこと。

---

## 6. バグ `balloon-reappear-short-talk` の着地に依る条項（main の取り込みでの突き合わせ用）

バグの中身（`.kiro/specs/areka-P0-balloon-reappear-short-talk/brief.md`）: 隠れたバルーン（時間切れ・利用者の中断・バルーンの切替の後）が、次の台詞が 1 文字だけだと現れない。直す場所は `crates/areka/src/emo2_boot/balloon_visibility.rs` の表示の判定（`decide_content`）。本 spec の前提ではないが、「バグ → α」の順で先に着地する見込み。

| 本 spec の条項 | 着地したとき | 未着地のとき |
|---|---|---|
| 要件 5.4 ⒞（説明書の条件つきの制限） | 書かない（5.5 により、直って偽になった制限は書かない） | 「隠れた吹き出しが、次の台詞が 1 文字だけのときに現れないことがある」を書く |
| 要件 7.4・7.7（持ち越しの表） | 行を作らない | 行を作る。引受先は `balloon-reappear-short-talk`（`.kiro/steering/roadmap.md:167` に台帳の行が実在＝7.7 を満たす） |
| 要件 1.1・1.3（署名の zip を組む時期） | バグの PR が main に入り、本ブランチへ取り込んだ**後**に組めば、組み直しは要らない | 一周の**後**にバグが着地して本ブランチへ取り込むと、本体のソースが変わる＝要件 1.3 で zip を組み直し、バルーンの見え方に関わる項目（少なくとも 7。台詞の前にバルーンが時間切れで隠れる場面を含む 3・5・8・9）を採り直す |
| 要件 2.1 項目 7（バルーンの切り替え）の観測 | 期待どおり | 切替の直後の台詞が 1 文字だと吹き出しが出ない。えも？？ の台詞はふつう 2 文字以上なので踏みにくいが、踏んだら不合格ではなく既知の症状として登記する（3.5・3.8 の扱い）か、開発者が決める |
| 要件 3.4（受入記録の「登記」） | — | 一周で見かけたら、判定に載せない既知の症状として登記し、引受先をバグの spec にする |

取り込みのときに触れ合うファイル: バグの spec が触るのは `crates/areka/src/emo2_boot/balloon_visibility.rs` とそのテスト、場合によって `doc/COMPAT_ARCHITECTURE.md` §8 と `.kiro/steering/roadmap.md`。本 spec が触るのは `dist/README.txt`・`vendors/sample_ghost/`（条件つき）・`tools/package-alpha.ps1`（議題 D1 しだい）・本 spec の `verification/`・完了時の `.kiro/steering/roadmap.md`。**重なるのは `roadmap.md` だけ**。`-Check` の記録の目印（`package-alpha.ps1:71-78`）はバグの直しで変わらない見込み（判定の文言はバルーンの可視性に無い）。

説明書の下書きでは、⒞ の行を「main の状態で書くか消すかを決める行」と分かる形で置いておくと、取り込みの後の判断が 1 か所で済む。

---

## 7. 設計に持ち越す調べもの（Research Needed）

1. **項目 12 の「初回だけ効く位置合わせ」が何か、2 回目の起動でどうなるのが今のコードの振る舞いか。** M1 の項目 9 の位置調整は `\![move,-353,,,0,base,base]`（`completed/areka-P0-emo2-conformance-e2e/design.md:274`）で、M1 の設計は 2 回目に「既定の配置へ戻る」を許容とした（同 `:277`）。位置の保存は掴んで離したとき（M1 の記録 `char DragEnd 保存`）に見え、`\![move]` の結果が保存されるかは本分析では確かめていない（`crates/areka/src/placement/persist.rs` と結線側の保存の投函口を読む）。要件 2.5（確かめた期待だけを書く）のために設計で確定する。
2. **`emo2.nar` を差し替えたとき、実物の台本を読むテストが通るか。** 28 本の呼び手のうち、台本や絵の中身に依るもの（`kero_menu_capacity_test.rs`・`shipped_fixture_region_test.rs` など）を先に洗い出す。
3. **§6.2 の Markdown をプレーンテキストへ移すときの線引き**（表・`<br>`・太字・見出し）。§6.3 の判定 2 本（`signoff-record.md:960` 以降）を写した後の説明書に当てて「写せた」ことを確かめられるか。
4. **項目 8 の更新先と差分。** 今の `emo2.nar` は配布サイトと 24 件ずれている（`completed/areka-P0-network-update/signoff.md:98`）。差し替えたあとは差分 0 になるので、手で差分を作る手順（要件 2.1 の 8 の括弧書き）を手順書に書く。
5. **一周の根と記録の置き場。** ワークツリーの片付けで消えない置き場に生の記録を残すか（M1 の前例）。zip そのものの指紋（sha256）を受入記録に残すかは任意。

---

## 8. 要件討議・設計に渡す決めごとの候補

（要件はすでに確定しているので、ここに挙げるのは「設計の前に答えが要るもの」と「確定した要件の読み方に関わるもの」である。答えで作業が変わるものだけを挙げた。）

1. **D1 `commit=` の桁**（要件 1.2）: 今の `BUILD-INFO.txt` は 7 桁。⒜ スクリプトを直して 40 桁にする（版を上げる・判定 8 も直す）／⒝ スクリプトは変えず、受入記録で 7 桁を 40 桁へ引き直して添える。⒝ は要件 1.2 の字面の読み替えになる。
2. **D2 項目 12 の期待の向き**（要件 2.1 の 12・2.5）: 要件は「既定の配置へ戻ってしまわない」、M1 の設計は「既定の配置へ戻る（許容）」。今のコードがどちらかを設計で確かめたうえで、食い違うなら、要件の読みを開発者に確かめる（「戻らない」が正しいのに戻るなら一周で不合格＝要件 3.8 の欠陥の扱いに入る）。
3. **D3 §6.2 の写し方**（要件 4.6 と 4.7 のぶつかり）: 本文の「バルーン…が 1 つ入っています」は zip の実物（2 つ）と食い違う。⒜ 写した本文の前に 1 文の前置き（「既定の吹き出し StayseeBalloon について」など）を地の文で足し、本文はそのまま／⒝ 上流の `default-balloon-bundle` の記録は書き換えない前提で、食い違いを受入記録に登記して説明書は字面どおり／⒞ 4.7 を優先して該当の 1 文だけ変える（4.6 の例外になる）。あわせて Markdown の記法をどこまでプレーンテキストへ移してよいか。
4. **D4 項目 8・13 の順序と、どの `emo2` を見るか**（要件 2.1 の 8・2.3・6.4）: 項目 8 は一周の根の `emo2` を配布サイトの版で上書きする。項目 13 を項目 8 の後に回すと、見ているのは zip の `emo2` ではない（配布サイトに `halt` の台詞が先に載れば、zip に無くても台詞が聞こえてしまう）。⒜ 項目 13 を項目 8 より前に回す／⒝ 項目 13 を別の新しい根で回す／⒞ どの版を見たかを記録に書けば順序は問わない。
5. **D5 `emo2.nar` の差し替えの範囲**（議題 2・4 と要件 6.1〜6.3）: 開発者の最新の配布物で差し替えると、`halt` の台詞（⒜）と古い更新先（⒝）が一度に片付き、項目 8 の自然な差分は消える。差し替えたときに確かめることは、`install.txt` の `balloon.directory,emo2-kakukaku` が残っていること（`-Check` の判定）と、実物の台本を読むテストが通ること。台詞が届かず更新先だけ直った版が手に入る場合に、それだけで差し替えるか。
6. **D6 全体テストを回す時点**（要件 7.1 ⑴・7.2・3.3）: 署名の根拠のコミットで `test-all.ps1` を回す 1 回と、完了の手順がアーカイブの後に回す 1 回の関係。後者で前者を兼ねられるか（アーカイブは文書だけの変更）、それとも署名の前に別に回すか。いずれも項目 12 の 1 回目と 2 回目の起動のあいだには回さない。
7. **D7 バグ `balloon-reappear-short-talk` の着地と zip を組む時期**（§6）: 署名の zip は、バグの PR が main に入って本ブランチへ取り込んだ後に組むか。それより前に一周を回すなら、着地の後に要件 1.3 の組み直しと採り直しが要る。
8. **D8 一周の根と記録の置き場**（要件 3.1・3.4）: ワークツリーの `target\`（片付けで消える）か、`C:\tmp` などの短いパスか、生の記録を残す別の置き場か。
9. **D9 付随の確認 `\![open,readme]` に使うゴースト**（要件 2.2）: 1 行足す辞書の写しを `emo2`（pasta）にするか `R_POST_and_KOMAINU`（里々）にするか。
10. **D10 利用条件の箱を一周で見るか**（要件 2.1 の 4・4.3）: 検体に `terms.txt` を持つものが 0 体なので、一周では「はい／いいえ」の箱は出ない。説明書の記述はソースと `ghost-install` のテストで裏付けるだけでよいか、一周の根の中で手で作った `.nar` で 1 度見るか。

---

## 9. 次の段

- 上の決めごとのうち D2・D3 は要件の読みに関わるので、要件討議（`/kiro-requirements-discussion areka-P0-alpha-release-signoff`）で扱うのがよい。残りは設計で決められる。
- 設計は `/kiro-design areka-P0-alpha-release-signoff`（または `/kiro-spec-design areka-P0-alpha-release-signoff`）。設計の前に、バグ `balloon-reappear-short-talk` が main に入っていれば本ブランチへ取り込み、§2 の file:line を引き直す。
