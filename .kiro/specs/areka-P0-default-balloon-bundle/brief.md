# Brief: areka-P0-default-balloon-bundle

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭の続き）で起票。同日の起票 6 本で「裁定候補 ⑴＝既定バルーンの同梱」として `baseware-root-layout` と `alpha-release-signoff` の末尾に置いていた議題を、開発者の指示「同梱バルーンの件は spec を作ってそこで扱う方がよい。起票し、ここまでの議論をブリーフィングに残せ」により独立の spec へ切り出した。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## Problem

**誰の何が困っているか**: areka を初めて手にする第三者。

第三者のゴーストの多くはバルーンを同梱しない（ukadoc「インストール」`ukadoc:manual_install` は同梱を任意の形として挙げるのみ。ukadoc「全体の構成」`ukadoc:manual_directory` は「バルーンはゴーストごとでなく、一つのベースウェアの元に統括的に管理され、全ゴーストで共有される」と定める）。SSP は本体に「SSPデフォルト+」と「balloon for Emily/P4」の 2 つを同梱しているので、利用者はバルーンの存在を意識せずにゴーストを入れて動かせる。**areka には同梱バルーンが無い**ので、バルーンを持たないゴーストを入れた第三者は「バルーンが無い」で止まる（`baseware-root-layout` の Desired Outcome 5）。

## Current State

- **areka のバルーン資産**: 検体は作者自作の `emo2-kakukaku`（emo2 同梱・`crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`）と、その検証用派生 2 つ（`emo2-kakukaku-offsetdpi`・`emo2-kakukaku-wplimit`）。`nar-install`（A1）がこれらを `.nar` に畳んで `vendors/sample_ghost/` へ集約する予定。
- **αの描画前提（2026-09-18 開発者確認）**: **areka は常に `use_self_alpha,1` 相当で bake する**（`crates/areka-emo-present/src/balloon.rs:18-20`「PNG α 尊重（R5.2）: `use_self_alpha,1` 相当＝`UseSelfAlpha::On` で bake する」・`crates/areka/src/emo2_boot/assets.rs:310` も `UseSelfAlpha::On` 固定）。**`.pna` 対応は行わない**（同 `balloon.rs:19-20` は `.pna` を `probe_pna` の既存 seam に委ねて追加しないと明記。開発者「pna 対応は不要」）。バルーンの `descript.txt` の `use_self_alpha`／`use_input_alpha`／`paint_transparent_region_black` は読んでいない（`crates/` の本番コードで 0 件。読むキーは `crates/areka-parsers/src/balloon/parse.rs` の 51 個＝`font.*`・`disable.font.*`・`cursor.*`・`origin.*`・`validrect.*`・`wordwrappoint.*`・`windowposition.*`・`vertical`）。**読まなくても挙動は「常に 1」で正しい**（台帳 `doc/ukadoc-coverage/ledger/assets.toml:2364` の項目 `ukadoc:descript_balloon:use_self_alpha_2c_5024:1` は「切るか切らないかだけを言う欄」）。
- **`thumbnail.pnr`**（左上 1 ドットと同色を透過する png の改名）は読んでいない（`crates/` で `pnr` 0 件）。`baseware-root-layout` の列挙は `thumbnail.png` の有無だけを見る予定。

### 議論の記録（2026-09-18・開発者と Claudia）

1. 起票 6 本の時点の案は「作者自作の `emo2-kakukaku` を既定に」。開発者「**同梱バルーンは癖が無いものが必要。emo2-kakukaku では厳しい。SSP から借用でもよいと思うが…**」。
2. 調査結果:
   - **SSP 同梱の「SSPデフォルト+」「balloon for Emily/P4」**: 再配布条件はウェブ上のどこにも公開されていない（SSP 公式 `ssp.shillest.net`・SSP ヘルプ FAQ・ukadoc に記載なし）。SSP 本体のソースコードも GitHub には無い（`github.com/ponapalt/ssp` は 404）。条件を知るには SSP の配布 zip 内の readme を読むか作者に尋ねるほかない＝**借用は根拠が薄い**。
   - **`Balloon for Staysee Syncfield`**（作者: ぽな＝ばぐとら研究所／整備班＝SSP 本家の作者・[github.com/ponapalt/StayseeBalloon](https://github.com/ponapalt/StayseeBalloon)）: LICENSE は **CC0-1.0**。readme（Shift_JIS・2020/6/27 v1.00A）原文「■転載・再配布・同梱・改変等について　煮るなり焼くなり好きにしてください。License : CC0」「特定のゴーストを意識して作ったバルーンですが、専用指定はしていません。デザイン的には極端な特化はしていないので、なにか流用できるかもしれません」「■！！！注意事項！！！　半透明（アルファチャンネル）ONを前提に作っており、OFFではまともに見られません」。`descript.txt` は `charset,Shift_JIS`・`type,balloon`・`name,Balloon for Staysee Syncfield`・`id,StayseeBalloon`・`craftman,SSP BUGTRAQ`・`homeurl,http://ms.shillest.net/balloon/StayseeBalloon/`・`use_self_alpha,1`・`use_input_alpha,1`・`validrect.left,22`／`top,20`／`right,-26`／`bottom,-47`・`font.height,12`・`arrow0/1`・`onlinemarker`・`sstpmarker`・`sstpmessage.*`・`number.*`・`communicatebox.*`。ファイルは `balloons0〜3.png`・`balloonk0〜1.png`・`balloonc0〜4.png`・`arrow0/1.png`・`online0〜8.png`・`marker.png`・`sstp.png`・`thumbnail.pnr`・`readme.txt`・`descript.txt`・`install.txt`・`LICENSE`。`.pna` 無し。動作確認 SSP 2.4。
   - 整備班（`ms.shillest.net`）の他のバルーン 6 つは配布条件がページに無い。
   - 自作の無地バルーンという逃げ道もあるが、画像を描く手間と descript の調整が要り、CC0 の既製品より高くつく。
3. 開発者「**areka は常に `use_self_alpha,1`。pna 対応は不要**」→ StayseeBalloon の唯一の注意事項（半透明前提）は areka の設計と一致し、障害にならない。
4. 開発者「同梱バルーンの件は spec を作ってそこで扱う」→ 本 brief。

## Desired Outcome

完了時に次が真になっている。

1. **areka の配布物に既定バルーンが 1 つ入っている**（`<根>/balloon/<id>/`）。候補は **`Balloon for Staysee Syncfield`（CC0）**。最終の採否は本 spec の要件段階で開発者が見た目を確認して決める（`font.height,12` の小ささ・薄い青の色味が「癖が無い」に足るか）。
2. **リポジトリでの保管形は `.nar`**（`vendors/sample_ghost/` の慣行＝`nar-install`）。原作の `readme.txt`・`LICENSE`・`install.txt` を 1 バイトも変えずに含める。`thumbnail.pnr` もそのまま同梱する（読まなくても害は無い）。
3. **第三者告知に載る**（`THIRD-PARTY-NOTICES.md`・CC0 の記載と出典 URL）。CC0 は帰属義務が無いが、出典と作者名は README に書く（礼儀として・正典外）。
4. **areka で崩れずに表示される**——半角・全角・縦書き（`vertical` は Staysee の descript に無いので横書きのみ）・選択肢・`\_l`・DPI 追従 k≠1。現行の検証用バルーン（`emo2-kakukaku-*`）の決定論テストと同じ観測を StayseeBalloon で 1 周し、崩れが出れば**areka 側の欠陥として直す**（バルーンを改変して合わせない）。
5. **バルーンが無いゴーストを入れると既定バルーンで喋る**（`baseware-root-layout` の解決順「同梱 → 記憶 → 根に 1 つ → 既定」の最後が本 spec の資産）。

## Approach

**選んだ形**: CC0 の既製品を `.nar` で保管し、配布 zip に展開形で入れる。areka 側のコードは「既定バルーンの id を知る 1 定数」と「配布スクリプトが `.nar` を展開する 1 段」だけ。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 取り込み | GitHub の `master` を取得し `.nar`（zip）へ畳む。`install.txt` は原作のものをそのまま（`type,balloon`・`directory` は原作の指定） | `nar-install` の展開器で展開して `descript.txt` の `id` が `StayseeBalloon` になる 1 本 |
| ② 表示 | 既存の検証用バルーンの決定論テスト（`crates/areka/src/emo2_boot/balloon_background_tests.rs`・`areka-emo-present` の balloon テスト等）の fixture 差し替え版を StayseeBalloon で 1 周 | 崩れ 0。崩れれば areka 側の欠陥として本 spec 内で直すか、引受先を実在確認して先送り（記憶 deferral-requires-verified-owner） |
| ③ 配線 | `baseware-root-layout` の解決順の最後に既定バルーンの id を渡す（定数 1 つ）。配布スクリプト（`alpha-release-signoff`）が zip に `balloon/StayseeBalloon/` を入れる | 「ゴーストにバルーン無し・根に他のバルーン無し」で既定が選ばれる決定論 1 本 |
| ④ 告知 | `THIRD-PARTY-NOTICES.md` と第三者向け README に出典・CC0 を記す | 目視 |

**取らない形**:
- **「SSPデフォルト+」の借用**——条件が確認できない。
- **`emo2-kakukaku` の既定化**——開発者裁定「癖が強い」。検証用の検体としては残る。
- **自作の無地バルーン**——既製品より高くつく。CC0 の候補が見た目で不採用になった場合の**次善**として残す。
- **`.pna`／`use_self_alpha,0` の対応**——開発者裁定「areka は常に `use_self_alpha,1`・pna 不要」。台帳の `use_self_alpha` 項目は「読まない・常に 1」を areka の裁量として `doc/COMPAT_ARCHITECTURE.md` §8 に 1 行登記する（本 spec の仕事）。

## Scope

- **In**:
  - 既定バルーンの選定（候補 StayseeBalloon・要件段階で開発者が見た目を確認）
  - `.nar` 化と `vendors/sample_ghost/` への保管（原作ファイル無改変）
  - StayseeBalloon での表示検証（決定論 1 周＋実機で 1 度目視・k≠1）
  - 既定バルーン id の定数と `baseware-root-layout` の解決順への配線
  - 第三者告知（`THIRD-PARTY-NOTICES.md`・README の出典）
  - `COMPAT_ARCHITECTURE.md` §8 への「`use_self_alpha` は常に 1・`.pna` 非対応」の登記（台帳 `assets.toml` の該当項目の `status`／`note` を要件段階で合わせる＝`owner` を本 spec に）
- **Out**:
  - `.pna`・`use_self_alpha,0`・`use_input_alpha`・`paint_transparent_region_black` の実装（読まない・常に 1 の裁量）
  - `thumbnail.pnr` の透過解釈（列挙は `thumbnail.png` のみ・`.pnr` は将来の `baseware-root-layout` 拡張）
  - バルーンの `homeurl` によるネットワーク更新（`network-update` の仕事。既定バルーンも更新対象になるが本 spec は関知しない）
  - 複数の既定バルーン（SSP は 2 つ・α は 1 つ）
  - `recommended.balloon` による案内（α 後）
  - `sstpmessage.*`・`number.*`・`onlinemarker`・`communicatebox.*`（Staysee の descript に在るが areka が読まないキー＝表示に使わない。`balloon-canon-residue` 等の所有・α 後）

## Boundary Candidates

- **資産**（`.nar` と告知＝コード非接触）
- **表示検証**（既存 fixture テストの差し替え版）
- **配線**（id 定数 1 つ＝`baseware-root-layout` との境界）

## Out of Boundary

- 根の解決順そのもの（`baseware-root-layout`）
- 配布 zip の生成（`alpha-release-signoff`。本 spec は「入れるもの」を決めるだけ）
- バルーン切替（`ghost-shell-balloon-switch`）

## Upstream / Downstream

- **Upstream**: `areka-P0-nar-install`（`.nar` の保管慣行と展開器）／完了仕様 `areka-P0-balloon-parse`・`balloon-vertical-canon`・`balloon-font-descript-keys`（descript の読み手）／完了仕様 `areka-P0-kero-balloon`（`balloonk*` の相方側）。
- **Downstream**: `areka-P0-baseware-root-layout`（解決順の最後）・`areka-P0-alpha-release-signoff`（zip の中身・項目 7「バルーン切替」の相手）・`network-update`（既定バルーンも `homeurl` を持つので更新対象になる）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-baseware-root-layout` と `areka-P0-alpha-release-signoff` の末尾「裁定候補 ⑴」を本 spec へ移した（2026-09-18・両 brief に追記済み）。
- **Adjacent**: `areka-P0-nar-install`（保管形式・共有ヘルパの検体名に `StayseeBalloon` を足す）／`areka-P0-balloon-canon-residue`（α 後・descript の未読キー）。

## Constraints

- **原作ファイルは無改変**（CC0 でも改変せずに済むものは改変しない＝バイト保存の罠 2 件は `nar-install` brief の「バイト保存の罠」に従う: `.gitattributes` で binary／text を明示・`.gitignore` の `*_test.txt` に当たる名前が無いことを確認）。
- **areka は常に `use_self_alpha,1`・`.pna` 非対応**（開発者裁定 2026-09-18）。半透明前提のバルーンだけが正しく表示される設計であることを README の既知の制限に書く。
- 新規の外部依存 0。コードの増分は定数 1 つと fixture 差し替えテストのみ。
- 1 ファイル 1,000 行。決定論テスト網羅は必達（既存 fixture テストの差し替えで足りる）。
- **裁定（開発者・要件段階）**: StayseeBalloon の見た目を実機で 1 度確認して採否を決める。不採用なら次善は自作の無地バルーン。
- 規模 **S**。編成は A2 と並走可（`baseware-root-layout` と共有ファイル 0 の見込み＝本 spec は `vendors/`・テスト fixture・`THIRD-PARTY-NOTICES.md`・`COMPAT_ARCHITECTURE.md` §8 だけに触り、id 定数の配線は root-layout 側の 1 行を後着で足す）。
