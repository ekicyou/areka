# Brief: areka-P0-coverage-roadmap-refresh

起票: 2026-09-19（`areka-P0-popup-menu-minimal` のタスク 9.4 と最終検証が「統合担当への申し送り」として残した件の受け皿・開発者指示「どこにも分類されていない問題は起票」）

## Problem

ukadoc 網羅調査の文書（`doc/ukadoc-coverage/roadmap-draft.md`・`briefing.md`）には、**日付付きの写真として書いた数**と、**常設の検査が見張っている数**が混ざっている。検査の外にある手書きの数は、spec が 1 本着地するたびに黙って偽になる。各 spec は自分の行を足すときに隣の節まで数え直しているが、「節全体の撮り直し」は 1 本の spec の範囲に収まらないので、毎回「統合担当への申し送り」として記録されるだけで**引受先の spec が無い**。申し送りは完了アーカイブへ入ると誰も読まなくなる。

## Current State

2026-09-19 の時点で確かめた事実（`areka-P0-popup-menu-minimal` の tasks.md 完了記録 9.4）:

1. `roadmap-draft.md` の「先頭ウェーブ」の節は 2026-09-13 の写真。今日数え直すと 324／84／31 は 324／62／55 になり、「バルーンの文字」の状態の分布と「63 件のうち 45 件」（今日は 22 件）も動く。各束の進行中の件数が 84 の内訳そのものなので、**部分的に直すと算術が壊れる**（該当行には「写真である」旨の注記を 1 行添えてある）。
2. `briefing.md` 5-3 の 416 件／1,333 件は日付付きの作業記録（今日は 433 件／1,316 件）。
3. 段階 B「更新」の候補 spec 名の案 `areka-P0-network-update` が、2026-09-18 に起票された実在の spec と同名（意図しない重なり・同じものを指すのか別物なのかの裁定が要る）。
4. `roadmap-draft.md` の波の欄は全行が旧編成（W13〜W17）の写し。steering `roadmap.md` は 2026-09-18 に α の段（A0〜A5）へ組み直した。
5. main が進むたびに、台帳（`ledger/*.toml`）・`briefing.md`・steering `roadmap.md` は**衝突なしで自動マージされ、手書きの数が両側の値で黙って重なる**（完了時に何度も踏んだ: PR #148／#154 の完了数の二重更新）。`areka-P0-popup-menu-minimal` の完了時にも PR #157 との間で同じ形になる見込み。

他の spec の記録に「引受先なし」として残っているもの（**本 brief では未検証**・要件段階で 1 件ずつ実在を確かめてから取り込む）:

- `linkage` の符牒 101 か所・`nar-install` の綴りの衝突・`shell-implicit-surface` からの `dev_shell` の粒度化の依頼（`ukadoc-coverage-roadmap` の完了時の残件）
- `report/summary.md` の陳腐化を見張る所見の種別が無い（`ukadoc-survey-shiori` の完了時の指摘）
- 台帳の備考の「語境界の欠陥」の記述の陳腐化（`sakura-tag-word-boundary` の完了時の指摘）
- 常設の検査は「縮退の根拠の実在」を見張っていない（`ukadoc-survey-sakura-script` の完了時の指摘）


### `areka-P0-shell-implicit-surface` からの申し送り（2026-09-20・実在を確かめたうえで）

上の「引受先なし」の一覧の 1 つ目の箇条にある 3 つ目（`shell-implicit-surface` からの `dev_shell` の粒度化の依頼）を、実測で裏取りしたうえで正式な依頼として置き直す。**依頼の中身**: 台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `ukadoc:dev_shell` と `ukadoc:manual_shell` の当該の文を項目の粒度へ割り、担当（`owner`）を `areka-P0-shell-implicit-surface` にしてほしい。

- **なぜ本仕様が自分で足せなかったか**: 台帳の id はカタログに実在しなければならない。ところがこの 2 ページは、台帳に**ページ 1 枚の粒度でしか無い**（2026-09-20 実測＝`[entry."ukadoc:dev_shell"]` と `[entry."ukadoc:manual_shell"]` が各 1 件。`ukadoc:dev_shell_error` は別のページ）。両方とも `status = "absent"`・`owner = ""` のままである。項目の行を足すことは、カタログの側を触らずにはできない。
- **割ってほしい文（本仕様が実装し終えたもの）**: `dev_shell` 側＝⑴ 「surface○○.png という名前の画像を用意するか、surfaces.txt で複数の画像を合成する」⑵ 「surface0000.png、surface0010.png 等の様に記述しても surface0.png、surface10.png と同様に認識されます」⑶ 「surface\*.png のような名前の png 画像は、element0 より下のパーツとみなされる」⑷ コマの相手の面の指し方（アニメーション編）⑸ 「サーフェス画像では左上端の 1 ドットが透過色として表示上透過されます」⑹ 「単色で塗り潰した画像（＝全て透明表示）などを surface10.png として用意してください」。`manual_shell` 側＝⑺ 「画像左上の 1 ドット（座標 0,0）と同色の領域は透過色（抜き色）とされ、表示上透過される」⑻ 対応画像形式の注記 ⑼ 「JPEG については圧縮によって色がずれやすいため透過色と相性が悪い」。
- **担当を本仕様にしてよい根拠**: ⑴〜⑶・⑸・⑺ は実装済み（ファイル名の慣習・先頭の 0・`element0` との関係・抜き色）。⑷ も実装済み（画像だけで存在する面をコマの相手にできる）。⑹ は未確認のままで（検体 3 体に「単色で塗り潰した surface10」が 0 件）、steering `roadmap.md` の「引き受け手の居ない残り」に登記してある。⑻ と ⑼ は正典側の注意書きで、areka には画像形式ごとの分岐が無い——面の画像として名前で認めるのは `.png` だけ（要件 1.3）で、`element` 行が名指しするファイルの復号は WIC 任せ、抜き色の腕は形式によらず同じ 1 本である。**粒度を割ったあとで状態を分けられる**ようにするのがこの依頼の目的で、今のページ 1 枚の粒度では「実装済みの文と未対応の文が同じ 1 行に同居していて `absent` と表示される」状態が続く。
- **本仕様が台帳へ新しく足した行は 0 件**である。担当を登記したのは既存の 2 行（`ukadoc:descript_shell_surfaces:sometimes:1`・`…:rarely:1`）だけで、どちらも 2026-09-20 に `implemented` へ改めてある。

## Desired Outcome

- 写真の節は「いつの写真か」が機械で読める形になっているか、道具が数え直して書く形になっている（手で引き算しない）。
- 検査の外にある手書きの数の一覧があり、それぞれが「検査に入れる」「写真と明記して凍結する」のどちらかに決まっている。
- 候補名の重なりと波の欄が今の steering `roadmap.md` と矛盾していない。
- main を取り込んだときに黙って重なる数を、取り込み直後に 1 コマンドで検出できる。

## Approach

方針は「**実装側だけを見る検査を足す**」（正典の増減に自動で気付く仕組みは買わない・開発規律）。まず手書きの数の全数を洗い出し、`ukadoc-survey` の既存の判定（owner と `owner_count` の突合・報告の鮮度）と同じ型で「文書の数 vs 台帳から数えた数」を 1 本ずつ判定に入れる。入れられない数（過去の写真）は日付を添えて凍結し、検査は「日付の無い手書きの数が無いこと」を見る。

## Scope

- **In**: `doc/ukadoc-coverage/roadmap-draft.md`・`briefing.md` の手書きの数の棚卸と撮り直し・`crates/ukadoc-survey` の判定の追加・候補名の重なりの裁定・波の欄の追随。
- **Out**: 台帳の項目の状態の変更（各 spec が自分で行う）・ukadoc のカタログの更新・steering `roadmap.md` の段の組み替え。

## Boundary Candidates

- 手書きの数の棚卸（文書）
- 判定の追加（`crates/ukadoc-survey`）
- 名前と波の整合（文書＋裁定）

## Out of Boundary

- 個別 spec の台帳登記（担当欄・状態）
- 正典（ukadoc）の増減の追跡

## Upstream / Downstream

- **Upstream**: `completed/ukadoc-coverage-roadmap`・`completed/ukadoc-survey-toolkit`（道具の持ち主）。α の A0 の 3 本（`nar-install`・`popup-menu-minimal`・`default-balloon-bundle`）が main へ入った後のほうが、撮り直しが 1 度で済む。
- **Downstream**: 台帳へ登記する全 spec（完了時の「隣の節の数え直し」が要らなくなる）。

## Existing Spec Touchpoints

- **Extends**: なし（元の 2 本は完了アーカイブ）
- **Adjacent**: `areka-P0-alpha-release-signoff`（α の宣言の時点で文書が正しいこと）

## Constraints

- 文書と検査だけの spec（製品コードに触れない）。α の必須ではないが、α の 6 本が着地するたびに同じ申し送りが増えるので、A0 の着地直後が最も安い。
- 1,000 行の番人は `log-capture-kit` に住む・`ukadoc-survey` のテストは報告の鮮度も見るので、文書を触ったら `report`／`report-summary` を作り直す。
- 規模の見立て: S〜M。
