# Brief: areka-P0-alpha-package

> 2026-09-26 `/kiro-discovery` 再入（棚卸⑰）で起票（台帳 #63）。`areka-P0-alpha-release-signoff`（台帳 #17）の再測定で、配布物づくり（スクリプト・zip の起動確認・第三者向け README の骨子・ライセンス表記の是正）が他の α の spec のソースと 1 本も重ならないと分かったので、α の最後の段まで待たせずに前へ切り出した。#17 には検証項目表・実機一周・README の仕上げ・署名と宣言が残る。
> 本文の file:line は**起票時の実測値**（2026-09-26・main `13b72893`）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: α を受け取る第三者と、α の完成宣言を出す開発者。今日は配布物を作る手段が無い——zip を組むスクリプトも、第三者が最初に読む説明書も無い。さらに根の `README.md` のライセンス表記が実物と食い違っている（バッジと本文が「MIT OR Apache-2.0」・バッジのリンク先 `LICENSE` は実在しない／`Cargo.toml` は `license = "MIT"`・実物は `LICENSE-MIT` だけ）。これを α の最後の段（#17）でまとめてやると、再配布の条件の確認（下の議題 ⑶）の答え次第で既定ゴーストの差し替えまで最後に押し寄せる。

## Current State

- `tools/` の直下は `tools/test-all.ps1`（66 行・PR#181）と `tools/perf/`。`scripts/` は無い。
- 根の形は完了 `areka-P0-baseware-root-layout` で確定（`<根>/ghost/<名>/`・`<根>/balloon/<名>/`・根は exe の隣）。記憶の置き場は `<exe>/profile/areka`（`boot_config.rs` の `default_app_profile_dir`）。
- 検体の SHIORI は `emo2`（pasta.dll）・`claudia`（yaya.dll）・`R_POST_and_KOMAINU`（satori.dll）とも 32 ビット＝zip に `shiori-host32-helper.exe`（名前は `boot_config.rs` で固定）が必須。
- 展開した木は `cargo run -p sample-ghost-kit --bin nar-sample-path -- <検体>` が作る（出力の形は `tests/nar_sample_path_test.rs` が固定）。展開した木を zip へコピーしても同梱バルーンの判定は効く見込み（`install.txt` はゴーストのフォルダに残る＝`catalog.rs` の `balloon.directory` の読み手・コードから読んだ推定で実走は未）。
- 既定ゴーストは `emo2`（`boot_resolve.rs` の `DEFAULT_GHOST_FOLDER`）、既定バルーンは CC0 の `StayseeBalloon`。
- `THIRD-PARTY-NOTICES.md` は完了のたびに `tools/test-all.ps1 -License` が作り直す生成物。`Cargo.lock` は追跡外（`.gitignore` の 2 行目）＝謝辞の版が環境で上下する。
- 同じ README の「57件の仕様を完了」も古い（完了フォルダは 201）。

## Desired Outcome

1. `tools/package-alpha.ps1` 1 本で、release ビルドの `areka.exe`・`shiori-host32-helper.exe`（i686）・既定ゴーストと既定バルーン（`.nar` を展開した木）・第三者向け README・ライセンスと謝辞を 1 つの zip に組める。
2. 組んだ zip を別の場所へ展開し、`AREKA_APP_SMOKE_EXIT_MS` の有界の自動終了で起動確認が通る（ゴーストが立ち、終了コード 0 で終わる）。
3. 第三者向け README の骨子がある（起動・終了・メニュー・記憶の置き場・既知の制限の欄）。**`.nar` の入れ方の手順は #15 の入口で決まるので欄だけ置き、#17 が仕上げる**。
4. 根の `README.md` のライセンス表記が実物と一致する（議題 ⑵ の答えどおり）。
5. 同梱する `emo2` について、第三者向け README と同梱の告知に「シェルは MIT ではなくシェル作者の条件に従う」「areka のファーストゴーストとして使えるが、シェルを抜き出して利用することはできない」を明記し、シェルの説明書（`shell/master/readme.txt`）を zip の中に残す（議題 ⑶ の裁定・2026-09-26）。

## Approach

- PowerShell スクリプト 1 本（前例 `tools/test-all.ps1`）。検体の展開は `nar-sample-path` の出力をコピーする（展開の仕組みを 2 つ持たない）。
- 起動確認は既存の smoke の作法（`AREKA_APP_SMOKE_EXIT_MS`＋`RUST_LOG` の grep）をスクリプトから呼ぶ。常時テストには入れない（release ビルドを要する・開発者が手で回す）。

## Scope

- **In**: 配布スクリプト・zip の起動確認・第三者向け README の骨子・根の README のライセンス表記と古い数の是正・（議題 ⑵ が「MIT OR Apache-2.0」なら `LICENSE-APACHE` の追加と `Cargo.toml` の `license` の修正）
- **Out**: 検証項目表と第三者の手順 12 項目の実機一周・README の仕上げ・署名と宣言（#17）／**既定ゴーストの差し替え**（議題 ⑶ は「同梱してよい」で決着し差し替えは不要になった。万一将来要るときも `boot_resolve.rs` は #13・#50 と共有するので本仕様では行わない）／インストーラ（msi 等）・署名付きの exe・自動更新（本体の更新は α 後の予約）

## Boundary Candidates

- 組む（スクリプト）
- 確かめる（展開先での起動確認）
- 書く（第三者向け README の骨子・根の README の是正）

## Out of Boundary

- Rust のソース（`crates/` に触らない）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-baseware-root-layout`（根の形）・`areka-P0-default-balloon-nar-fold`（既定バルーンの `.nar`）・`areka-P0-nar-install`（`nar-sample-path`）・PR#181（`tools/test-all.ps1` の前例）
- **Downstream**: #17 `areka-P0-alpha-release-signoff`（この zip で実機一周をする）

## Existing Spec Touchpoints

- **Extends**: #17 から切り出し
- **Adjacent**: #13（同じウェーブで並走・共有 0＝#13・#50・#15・#16 はどれも `tools/` と根の `README.md` に触らない）

## Constraints

- `crates/` に触らない（触る必要が出たら #17 へ送る）。`THIRD-PARTY-NOTICES.md` は手で直さず生成器で作り直す。
- 第三者の資産の再配布条件を README に正しく載せる（`StayseeBalloon` は CC0、`claudia` は Unlicense、`konnoyayame` のシェルは CC BY-NC-ND＝zip に入れない、`emo2` のシェルは MIT ではない＝ファーストゴーストとしての同梱は可・シェルの抜き出し利用は不可と明記）。
- 規模 **S（4〜5 タスク）**。要件は Opus で足りる（議題は開発者の決めごとで、深掘りではない）。

## 要件段階の議題（開発者の決めごと）

- ⑴ zip に `emo2-kakukaku` を入れるか（入れると初回に立つバルーンが `StayseeBalloon` ではなく `emo2-kakukaku` になる）。
- ⑵ ライセンスは MIT 単独か MIT OR Apache-2.0 か（前者なら README の 2 か所、後者なら `LICENSE-APACHE` の追加と `Cargo.toml` の修正）。
- ⑶ ~~emo2 を zip に入れてよいか~~ → **2026-09-26 開発者裁定＝emo2 は同梱してよい。** ただし ⓐ シェルは MIT ではない（シェル作者の条件に従う）ことを第三者向け README と同梱の告知に明記する ⓑ areka のファーストゴースト（既定ゴースト）として使うことはできるが、**シェルを抜き出して利用することはできない**と明記する ⓒ シェル作者の条件どおりシェルの説明書（`shell/master/readme.txt`）を zip の中にそのまま残す。既定ゴーストの差し替えは不要になった。（旧文: シェルの説明書は「フリーシェルとしての再配布」と「商用利用」を禁じ、再配布するならこのテキストを必ず同梱せよと書いている。）
- ⑷ `Cargo.lock` を追跡するか（追跡すれば zip と謝辞が再現できるが、並走する枝が依存を変えるたびに `Cargo.lock` が衝突する）。
