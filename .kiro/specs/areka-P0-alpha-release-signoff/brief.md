# Brief: areka-P0-alpha-release-signoff

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。M1 の `areka-P0-emo2-conformance-e2e`（完成宣言の器）と同じ役割を α で担う——**配布物を作り、第三者の手順で一周し、開発者が署名する**。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## 2026-09-26 棚卸⑰の再測定（main `13b72893`＝#55・#58・#59・PR#180・PR#181 の着地後）

**棚卸⑰で配布物づくりを前に切り出した＝台帳 #63 `areka-P0-alpha-package`**（配布スクリプト `tools/package-alpha.ps1`・zip の起動確認・第三者向け README の骨子・根の README のライセンス是正・4〜5 タスク）。触るのは `tools/`・根の `README.md`・新規の第三者向け README だけで、#13・#50・#15・#16 はどれもここに触らない＝**#13 の実装と並走できる**。本仕様に残るのは **検証項目表・第三者の手順 12 項目の実機一周・README の仕上げ（入れ方の手順は #15 の入口で決まる）・既定ゴーストの差し替え（議題 ⑶ の答えしだい）・署名と宣言**（5〜6 タスク）。議題 ⑴⑵ と新しい議題 ⑶（emo2 の再配布の条件）は #63 の要件で先に決める。

**崩れた／変わった前提**
1. `tools/` の直下に `tools/test-all.ps1`（66 行）が入った（PR#181）。配布スクリプトは `tools/package-alpha.ps1` に置くのが前例に沿う（#63）。`scripts/` は今も無い。
2. ライセンスの不一致は残っている: 根の `README.md` のバッジと本文は「MIT OR Apache-2.0」（バッジのリンク先 `LICENSE` は実在しない）、`Cargo.toml` は `license = "MIT"`（各クレートは `license.workspace = true`）、実物は `LICENSE-MIT` だけ。`crates/*/README.md` と `doc/` に Apache の記述は 0 件。根の README は他の spec が触らない＝#63 で直す。同じ README の「57件の仕様を完了」も古い（完了フォルダは 201）。
3. 検体の SHIORI は `emo2`（pasta.dll）・`claudia`（yaya.dll）・`R_POST_and_KOMAINU`（satori.dll）とも 32 ビット（PE の機種 0x14c）＝zip に 32 ビットの helper（名前は `boot_config.rs` で `shiori-host32-helper.exe` に固定）は必須。
4. 展開した木を zip にコピーしても、同梱バルーンの判定は効く見込み（読み手は `<ゴースト>/install.txt` の `balloon.directory`＝`catalog.rs`、`areka-nar` は同梱バルーンの取り出し元フォルダだけを本体の配置から外す＝`install.txt` はゴーストのフォルダに残る。コードから読んだ推定で実走は未）。
5. **新しい論点（再配布の条件）**: `emo2` のシェルの説明書（`shell/master/readme.txt`・コンフィズリー・ゆゆぴか氏）は「フリーシェルとしての再配布」と「商用利用」を禁じ、再配布するならこのテキストを必ず同梱せよと書いている。\1 側のシェル（City-Pop'n・大槻氏）の条件は書庫に無い。`emo2` は既定ゴースト（`boot_resolve.rs` の `DEFAULT_GHOST_FOLDER = "emo2"`）なので、areka の zip にゴーストごと入れてよいかを確かめる（#63 の議題）。→ **2026-09-26 開発者裁定＝同梱してよい**（シェルは MIT ではないこと、areka のファーストゴーストとして使えるがシェルを抜き出して利用することはできないことを明記する＝#63）。既定ゴーストの差し替えは不要。
6. **PR#180 の影響**: 検体は 7 体。`claudia` は全体が Unlicense（`vendors/sample_ghost/README.md`）で、配布物に入れても支障のない唯一のゴーストの検体・同梱バルーンが 2 つ＝検証項目 4（2 体目）と 7（バルーン切替）の候補。`konnoyayame` はシェルが改変・営利の禁止付きなので zip へは入れない（実機一周で手で置くのは可）。シェルを 2 つ持つ検体は今も 0 体（項目 6 は #50 の議題 ⑴ 待ち）。
7. 旧議題 ⑷（SHIORI が動かないと黙って消える）は #55（PR#183）で解消した（告知して終了コード 1）。既知の制限には載せない。
8. `THIRD-PARTY-NOTICES.md`（2,828 行）は完了のたびに `tools/test-all.ps1 -License` が作り直す。本仕様は最後に差分 0 を確かめるだけ。
9. **`Cargo.lock` は追跡外**（`.gitignore` の 2 行目）。このため謝辞の版が環境で上下する（kiro-complete は「戻す」手順で回避しているだけ）。配布 zip の再現性と謝辞の正確さに効く＝#63 の議題（追跡すると並走する枝が互いの依存の変更で `Cargo.lock` を衝突させる代償がある）。
10. **既知の制限の候補**: `derive_scopes()` が `[0, 1]` 固定（`emo2_boot/mod.rs`）＝キャラが 3 人以上のゴーストの `\p[2]` 以降の窓は出ない（#13 の再測定項目 11）。
11. 検証項目 8 の更新先の候補: `emo2` の `homeurl`（開発者の配布サイト・https）。#16 の実機確認と共用できる。
12. 変わっていない: `default_app_profile_dir`（`<exe>/profile/areka`）・バルーンの決まる順「記憶 → 同梱 → 唯一 → 既定 → 無作為」（`boot_resolve.rs`）・`AREKA_APP_SMOKE_EXIT_MS` は zip の起動確認にそのまま使える（#63 が使う）。

**検証項目の差し替え（推し・議題にしない）**: 項目 1＝`ghost/` を空にした根で「ゴーストが見つかりません」を見る。項目 4＝`claudia.nar`。

**タスク数**: 本仕様 5〜6（#63 を切り出した後）。**議題 ⑶ は 2026-09-26 に開発者が裁定＝emo2 は同梱してよい**（シェルは MIT でないこと・ファーストゴーストとしては使えるがシェルの抜き出し利用は不可であることを明記＝#63 が書く）。既定ゴーストの差し替えは不要＝+1〜2 は消えた。

**要件段階の議題**: ⑴ zip に `emo2-kakukaku` を入れるか／⑵ ライセンスは MIT 単独か MIT OR Apache-2.0 か／~~⑶ emo2 を zip に入れてよいか~~（**2026-09-26 開発者裁定＝同梱可**・シェルは MIT でないこと、ファーストゴーストとしては使えるがシェルの抜き出し利用は不可であることを明記）——**⑴⑵ は #63 の要件で決め、本仕様はその答えを受け取る**。

## 2026-09-24 棚卸⑯の再測定（main `0b01f654`）

想定 **8〜10 タスク**・分割不要。Rust のソースはほぼ 0（`nar-sample-path` で足りなければ展開用の小さな bin を 1 本）。前提は α の残り全部（#55・#58・#13・#59・#50・#15・#16）。

**崩れた／変わった前提**

1. **根の `README.md` は開発者向け**（ビルド手順・クレート構成・「57 件の仕様を完了」・「ぱすたさん」を目標に掲げるなど古い）＝第三者向け README は**別ファイルとして新規に作る**。さらに **README のライセンス表記「MIT OR Apache-2.0」とバッジのリンク先 `LICENSE`（存在しない）が実物と合っていない**——実物は `Cargo.toml` の `license = "MIT"` と `LICENSE-MIT` だけ（09-24 に較正済み）。直し方は議題 2 の答えで決まる。
2. **アプリの記憶の既定の置き場所は「exe の隣」ではなく `<exe のフォルダ>/profile/areka/sylphya.toml`**（`boot_config.rs` の `default_app_profile_dir`・完了 #12 要件 1.7・9.5）。下の 09-24 追記の「既定は exe の隣」はこの細部だけ違う。helper は exe の隣の `shiori-host32-helper.exe` 固定（`boot_config.rs` の `default_helper_exe_path`）。
3. **新しい論点: `emo2.nar` の同梱バルーンは `emo2-kakukaku`**（`SAMPLES` の `balloons`）。バルーンは「記憶 → 同梱（`install.txt` の `balloon.directory`）」の順で決まる（`boot_resolve.rs` の `BalloonRoute::Companion`）ので、zip に `emo2-kakukaku` を入れると**初回は StayseeBalloon ではなく emo2-kakukaku で立つ**。開発者は emo2-kakukaku を「癖が強く既定に向かない」と裁定している（議題 1）。
4. `LICENSE-MIT`・`about.toml`・`about.hbs`・`THIRD-PARTY-NOTICES.md` は在る。`scripts/` は無い。`tools/` の直下は `perf/` だけ（置き場所は判断）。

**用意済みの部品**: `SAMPLES`（`crates/sample-ghost-kit/src/lib.rs`）の 7 体＝`emo2`・`R_POST_and_KOMAINU`・`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`・`konnoyayame`・`StayseeBalloon`・`claudia`。任意の宛先へ展開する公開関数は**無い**——使えるのは `pub fn manual_paths(name)` と、それを包む bin `nar-sample-path`（`target/nar-samples/manual/<名>/` へ展開して `root=`／`folder=`／`balloon.<dir>=` を出力。`tools/perf/invoke-followup-checks.ps1` が既にこの出力を読んでいる＝PowerShell から呼ぶ前例）、または `areka_nar` の `install(&InstallRequest{root, target_ghost})`（根へ入れる正規の経路・CLI の bin は無い）。スクリプトは `nar-sample-path` を呼んでコピーすればコードを 0 行で済ませられる。既知の制限で既に書かれたもの＝`completed/areka-P0-default-balloon-bundle/verification/signoff-record.md` §6.2・`doc/COMPAT_ARCHITECTURE.md` §8（charset の「既知の限界」など）・`completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md` の「持ち越した事項」の表。

**触るファイル**: 高＝`scripts/package-alpha.ps1`（新規・置き場所は判断）・配布用 README（新規）・`verification/acceptance-record.md`・`verification/alpha-completion.md`（新規）・`THIRD-PARTY-NOTICES.md`（再生成）。中＝根の `README.md`（ライセンス表記の是正）。

**要件段階の議題**: ⑴ zip に `emo2-kakukaku` を入れるか（入れると初回の既定が StayseeBalloon でなくなる）。⑵ **ライセンスは MIT 単独か MIT OR Apache-2.0 か**——後者なら `LICENSE-APACHE` の追加と `Cargo.toml` の修正、前者なら README の修正（どちらも小さいが**開発者の決めごと**なので棚卸⑯では触っていない）。⑶ 検証項目 1 と 4 の差し替え先（空の根で確かめる手順と 2 体目の検体）。⑷ #55 が α までに着地しない場合、「SHIORI が動かないとアプリが黙って消える」を既知の制限に載せるか（B1 で着地する予定なので通常は不要）。**未測定**: `nar-sample-path` で展開した木を zip にコピーしても同梱バルーンの判定（Companion）が効くか／`emo2.nar` の中身（i686 の pasta.dll を同梱しているか）と再配布ライセンス。

## 2026-09-20 棚卸⑮の再測定

**実測の追記（main `fe157df1`）**

- ライセンスのファイル名は `LICENSE` ではなく **`LICENSE-MIT`**。
- `scripts/` は実在しない＝新規。既存の道具の置き場は `tools/perf`。
- 本文の検証項目は 12 項目。roadmap の 3 か所が「11 項目」と書いていたのを、本日 12 へ直した。
- **zip へ入れる既定バルーン 29 ファイルの出どころが変わった。** `areka-P0-default-balloon-nar-fold`（台帳 #42）が着地し、既定バルーンは展開フォルダではなく `vendors/sample_ghost/StayseeBalloon.nar` として保管されている（登記表 `SAMPLES` に `StayseeBalloon` の名前で登記済み）。zip を作るスクリプトは `.nar` を展開して入れる（窓口 `sample_ghost_kit` か `areka-nar` を呼ぶ）。
- **片道だった申し送りを受け取る。** 完了 `areka-P0-shell-implicit-surface` は、開発者の目と手が要る 5 項目（⑴ 絵の外のクリックが背後の窓へ抜ける ⑵ 右クリックメニューの 1 項目目の表示 ⑶ `konnoyayame` の目の周りに四角い地色が出ない ⑷ 起動挨拶の字形が文字化けしない ⑸ `emo2` の撫で・メニュー・終了が適用前と同じに見える）を「本仕様の実機一周で見る」と書いたが、本文に該当の記述が 0 件だった。**第三者の手順の実機一周に、検体 3 体それぞれで ⑴ を、テンプレート 2 体で ⑶⑷ を含める。** ⑴ は `areka-P0-keycolor-clickthrough-coverage`（台帳 #53）が決定論テストで退行を止めるが、実機の確認は外さない。
- 前提の spec は分割で増えた: `baseware-root-layout`・`app-lifetime-separation`・`ghost-shell-balloon-switch`・`shell-balloon-switch`・`ghost-install`・`update-engine`・`network-update`・`default-balloon-nar-fold`。

## Problem

**誰の何が困っているか**: 開発者。「α 版として第三者に使い始めてもらえる」と言える根拠が、個々の spec の緑の寄せ集めでは作れない。M1 が e2e の 20 項目で完成を宣言したように、α も**第三者の手順そのものを検証項目にした実機サインオフ**が要る。

加えて、今日の areka には**配布物が無い**。`cargo build` の成果物は `target/debug/areka.exe` と i686 helper（別ディレクトリ・`target/debug/` へ手でコピーしないと実機が壊れる＝記憶 workspace-test-needs-i686-host32-artifacts）。第三者が受け取れる zip は存在しない。

## Current State

- M1 のサインオフの器: `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/`（適合検証項目表 20 項目・`verification/acceptance-record.md`・`verification/m1-completion.md`）。実機運転の定石は roadmap「制約」（絶対パス起動・i686 helper 先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了・`RUST_LOG` grep）。
- 第三者告知（`THIRD-PARTY-NOTICES.md`）は `nar-install` が `zip` を足す時に再生成される慣行。`cargo about` の設定 `about.toml`。
- 既定バルーンの同梱は `baseware-root-layout` の裁定候補 ⑴。
- ビルド種別: debug と release で CPU 約 3 倍（記憶 present-gpu-transform-scale の知見）。配布は release。

## Desired Outcome

完了時に次が真になっている。

1. **配布 zip が 1 コマンドで作れる。** 中身＝`areka.exe`（release・x64）＋ i686 helper（`shiori-host32-helper`・exe の隣）＋`balloon/<既定バルーン>/`（裁定 ⑴ で同梱するなら）＋`README.md`（第三者向け・置き方と最初の手順）＋`THIRD-PARTY-NOTICES.md`＋`LICENSE`。`ghost/` は空（利用者が入れる）。arm64 版は同じ手順で作れるが α の必須ではない（記憶 arm64-windows-build）。
2. **第三者の手順で一周が通る（実機サインオフ）。** 検証項目表（案・要件段階で確定）:
   1. zip を展開して `areka.exe` を起動 → 「ゴーストが無い」告知で止まる
   2. `.nar` をキャラクター窓へ…はまだ窓が無いので、**告知の中に「ゴーストの `.nar` をここへ置いてください」とフォルダを開く手段**があり、置いて再起動すると起動する（要件段階で「告知からファイル選択でインストール」に格上げしてよい）
   3. 里々の標準テンプレート（`R_POST_and_KOMAINU.nar`）で起動 → 挨拶 → 絵が出る（`shell-implicit-surface` 着地の確認）
   4. 2 体目（`emo2.nar`）を窓へ落とす → インストールイベント → 切替
   5. 右クリックメニュー → ゴースト一覧に 2 体 → 切替 → 戻る
   6. シェル切替（2 シェル持ちの検体）
   7. バルーン切替（同梱バルーン ⇄ 既定バルーン）
   8. ネットワーク更新（開発者の配布サーバに置いた検体で差分 1 件）→ `OnUpdateComplete` → 読み直し
   9. 終了（メニュー）→ 終了挨拶 → プロセス終了
   10. 再起動 → 前回のゴースト・バルーン・窓位置が復元される
   11. 表示スケール ≠ 100% の画面で 3〜9 が崩れない（記憶 areka-placement-real-ghost-first）
   12. **初回起動でだけ効く位置合わせが、2 回目以降の起動で既定の配置へ戻らない**（M1 からの持ち越し・下の 2026-09-19 追記）
3. **既知の制限が README に書いてある。** 表現力は M1（emo2 が動く水準）・オーナードローなし・多重ゴーストなし・SSTP なし・SAORI は SHIORI 任せ等。
4. **開発者の署名**（M1 と同じ人間判断・自動判定にしない）。

## Approach

| 段 | 中身 | 検証 |
|---|---|---|
| ① 配布物 | `cargo xtask` は無い方針（`structure.md`・`build.rs` 0 本）なので、**PowerShell スクリプト 1 本**（`scripts/package-alpha.ps1`＝release ビルド → helper コピー → 既定バルーン → README → zip） | スクリプトが作った zip を展開し、`areka.exe` が helper を見つけて起動する 1 本（有界自動終了） |
| ② 検証項目表 | 上の 12 項目を要件で確定・`verification/acceptance-record.md` の形は M1 を写す | 各項目に「操作・期待・証跡（ログ grep か目視）」 |
| ③ 実機一周 | 開発者の機械で 1 周（記憶 areka-real-machine-signoff-bounded-auto-exit・real-machine-signoff-catches-what-cages-hide） | 署名 |
| ④ 宣言 | `verification/alpha-completion.md`（M1 の `m1-completion.md` を写す・持ち越しと引受先の表） | — |

**取らない形**: インストーラ（MSI・自己解凍）。zip で足りる（SSP と同じ）。

## Scope

- **In**: 配布スクリプト・README（第三者向け）・第三者告知の再生成・検証項目表・実機一周・宣言文書・既知の制限の一覧
- **Out**: 自動更新（`\![update,platform]`）・コード署名・配布サイトの用意・arm64 の必須化・αでの性能目標の引き直し

## Boundary Candidates

- **配布物の形**（zip の中身＝`baseware-root-layout` の根の形）
- **検証項目表**（第三者の手順＝各 spec の受入の総和ではなく体験の一周）

## Out of Boundary

- 各機能の実装（先行 spec）

## Upstream / Downstream

- **Upstream**: α の全 spec（`shell-implicit-surface`・`nar-install`・`baseware-root-layout`・`ghost-shell-balloon-switch`・`popup-menu-minimal`・`ghost-install`・`network-update`）。
- **Downstream**: α 後の組み直し（M3「伺かの冠」の起点＝本仕様の宣言）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-emo2-conformance-e2e`（完了・器の写し元）。

## Constraints

- 実機一周は**有界**（長時間試行禁止）。各項目は 1 分以内に観測できる形にする。
- release ビルドで測る（debug は CPU 3 倍）。
- サインオフの根拠になった実行体のコミットを記録する（M1 の教訓＝§6 R8.3）。
- 規模 **S〜M**（スクリプトと文書が主・コードはほぼ増えない）。

---

## 2026-09-18 追記（裁定候補 ⑴ の移管）

- 「既定バルーンの同梱」は独立の spec `areka-P0-default-balloon-bundle`（2026-09-18 起票・A2 並走）へ移した。開発者裁定: `emo2-kakukaku` は癖が強く既定に向かない。候補は CC0 の `Balloon for Staysee Syncfield`（作者は SSP 本家）。**areka は常に `use_self_alpha,1`・`.pna` 非対応**（開発者確認）。本 brief の裁定候補 ⑴ は同 spec の要件段階で決める。

## 2026-09-19 追記（`popup-menu-minimal` の実機確認 9.3 から引き受ける手順の教訓 3 件）

実機サインオフの手順を組むときに入れること。どれも `areka-P0-popup-menu-minimal` の tasks.md 完了記録 9.3 に経緯がある。

1. **右クリックメニューは「左クリックを 1 度した後で」開く**。1・2 回目の走行は右クリックしかしなかったので、「左クリック以降メニューが二度と出ない」欠陥（wintf のドラッグ状態が `JustEnded` で休む）を踏まなかった。操作の順序を 1 通りしか試さない手順は、状態を持つ欠陥を見落とす。
2. **「メニューを開いたまま別のアプリを操作する」手順は書けない**（タスクマネージャへフォーカスを移した瞬間にメニューが閉じる）。表示中に SHIORI を落とす確認は、ログの `menu_shown` を見張って helper を外から止める形で行った。
3. **台本の入口 `\![open,readme]` は実機で 1 度も踏んでいない**（メニューの「説明書」だけ確認済み）。サインオフの台本に 1 行足して 1 度見ること（`areka-P0-popup-menu-residue` の残件 3 と相乗り）。
## 2026-09-19 追記（M1 からの持ち越し 1 件を検証項目に入れた）

M1 の完成宣言（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md` の「持ち越した事項」の表・1 行目）は、**初回起動でだけ効く位置合わせが 2 回目以降の起動で既定の配置へ戻ってしまわないか**を**未観測**のまま閉じている。M1 の走行では、2 回の走行のあいだに決定論のテストが走って保存された位置を消してしまい、確かめられなかった。持ち越し先はその表で「**次の一周**または開発者の個別確認」と書かれており、次の一周とは本仕様の実機一周のことである。しかし本 brief にはこの項目が 1 文字も無かったので、上の検証項目表に **12 番**として足した。

観測のしかたで気をつけること——**同じ機械でこの項目を測る前に、保存された位置を消す決定論のテストを走らせない**（M1 が観測できなかった原因がそれである）。項目 10（再起動で前回のゴースト・バルーン・窓位置が復元される）と紛らわしいが、別のものを見ている: 項目 10 は「保存した位置が戻ってくるか」、項目 12 は「初回だけ効く位置合わせが 2 回目に**効いてしまわない**か」である。既存の裁定（許容仕様）はそのままなので、違和感が出た時点で個別の仕様を切る——本仕様はそこまでを見る。
---

## 2026-09-19 追記（`default-balloon-bundle` からの申し送り＝zip の中身と README の出典文）

- **zip に入れるもの**: 根の下の `balloon/StayseeBalloon/`。中身は `vendors/sample_ghost/StayseeBalloon.nar` を窓口 `sample_ghost_kit` で展開した `balloon/StayseeBalloon/` の **29 ファイルを無改変で**（原作の `readme.txt` と `LICENSE` を含める・告知ファイルを中に足さない）。取り出したものが上流と同一であることは `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/provenance.md` の「ハッシュ一覧」の節（29 本の sha256）で確かめられる。
- **README の出典文は書き起こさず、そのまま写す**: 写す対象は `.kiro/specs/completed/areka-P0-default-balloon-bundle/verification/signoff-record.md` の **§6.2「README へそのまま写す本文」の引用ブロック**である。行頭の引用記号（本文の行は `> `、ブロック内の空行は素の `>`）を落として貼ればよく、中身を書き直す必要はない。**§6.1 と §6.3 は写す対象ではない**（§6.1 は使い方と裏取りの対応表、§6.3 は確かめ方）。ブロックには作者名・CC0・出典 URL・配布サイト・同梱した版・取得コミットと日付に加え、既知の制限（半透明を前提に作られたバルーンだけが正しく表示されること・`.pna` は置いてあるかを見るだけで中身を表示に使わないこと・相方用の枠が 3 種類目と 4 種類目では本体用の絵を借りること）が入っている。
- **「そのまま写せる」ことは §6.3 の判定 2 本で確かめられる**: ⑴ §6.2 の中に引用ブロックでも空行でもない行（＝写せない地の文）が 0 行であること、⑵ 引用ブロックの中に第三者が知らない内部の言葉が 0 件であること。どちらも打つ命令と、その 0 が探し方の壊れでないことを示す較正（同じ採り方を §6.1 に当てると 0 でない値が出る）が §6.3 に並べてある。写す前と写した後にそのまま打てる。
- **`THIRD-PARTY-NOTICES.md` は手で編集しない。** 同ファイルは cargo の依存から自動生成されるもので、cargo 依存でない同梱資産は載らない（`default-balloon-bundle` は同ファイルを 1 行も変えていない）。第三者向けの出典表示は、上の本文を README へ写すことで果たす。

## 2026-09-24 追記（`baseware-root-layout` の完了で変わった前提 3 件）

`areka-P0-baseware-root-layout` の完了（2026-09-24）で、Desired Outcome の前提が次のとおり変わった。本仕様の要件段階で揃えること（正本は `.kiro/specs/completed/areka-P0-baseware-root-layout/requirements.md` 要件 9 の裁定 3〜5）。

- **zip の `ghost/` は空ではない**（Desired Outcome 1 の「`ghost/` は空」と検証項目 1・4 を改める）。裁定 3＝既定ゴースト `emo2` は areka の配布物に**必ず同梱**する。記憶が無くゴーストが複数あるときは `emo2` を選び、無ければ無作為に 1 体。したがって検証項目 1「『ゴーストが無い』告知で止まる」は同梱 zip では起きない（空の根で確かめる手順に置き換える）。検証項目 4「2 体目（`emo2.nar`）を窓へ落とす」は別の検体（例 `R_POST_and_KOMAINU.nar`・`konnoyayame.nar`）へ差し替える。
- **告知にフォルダを開く手段は無い**（検証項目 2）。`baseware-root-layout` の告知（`crates/areka/src/alert.rs` の `alert_text`）は `MessageBoxW` の `MB_OK` で、本文 3 行に「何が無いか」「置く場所の絶対パス」「置くものの形」を書くだけ。フォルダを開く手段を足すなら本仕様か別 spec の範囲で、`alert` の 4 場面の文面（smoke の目印「ゴーストが見つかりません」を含む）を壊さないこと。
- **記憶の置き場所**（検証項目 10）: 最後のゴーストはアプリの記憶（`AREKA_PROFILE_DIR`／既定は exe の隣）の `sylphya.toml` の `[last] ghost`、最後のバルーンとシェルは**ゴーストごとの記憶**（`<ゴースト>/ghost/master/profile/areka/sylphya.toml` の `[last] balloon`／`shell`）。argv で起動したときは記憶を書かない（裁定 5）ので、実機一周は argv なし（根の下に置いて起動）で回すこと。
