# Brief: areka-P0-alpha-release-signoff

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。M1 の `areka-P0-emo2-conformance-e2e`（完成宣言の器）と同じ役割を α で担う——**配布物を作り、第三者の手順で一周し、開発者が署名する**。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

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

## 2026-09-19 追記（M1 からの持ち越し 1 件を検証項目に入れた）

M1 の完成宣言（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md` の「持ち越した事項」の表・1 行目）は、**初回起動でだけ効く位置合わせが 2 回目以降の起動で既定の配置へ戻ってしまわないか**を**未観測**のまま閉じている。M1 の走行では、2 回の走行のあいだに決定論のテストが走って保存された位置を消してしまい、確かめられなかった。持ち越し先はその表で「**次の一周**または開発者の個別確認」と書かれており、次の一周とは本仕様の実機一周のことである。しかし本 brief にはこの項目が 1 文字も無かったので、上の検証項目表に **12 番**として足した。

観測のしかたで気をつけること——**同じ機械でこの項目を測る前に、保存された位置を消す決定論のテストを走らせない**（M1 が観測できなかった原因がそれである）。項目 10（再起動で前回のゴースト・バルーン・窓位置が復元される）と紛らわしいが、別のものを見ている: 項目 10 は「保存した位置が戻ってくるか」、項目 12 は「初回だけ効く位置合わせが 2 回目に**効いてしまわない**か」である。既存の裁定（許容仕様）はそのままなので、違和感が出た時点で個別の仕様を切る——本仕様はそこまでを見る。
