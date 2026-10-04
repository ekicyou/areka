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

**⚠ 2026-09-20 追記**: 依頼元の `areka-P0-shell-implicit-surface` は同日に完了し `.kiro/specs/completed/areka-P0-shell-implicit-surface/` へ退避した。**完了した spec は先送りを吸収できない**ので、`owner` にその名前を書くと「もう誰も仕上げられない行」が増える（`.kiro/steering/roadmap.md` の所見 ⓑ と同じ形）。本 spec の要件段階で、⑴ 生きた引受先へ付け替えるか ⑵ 実装済みとして状態ごと畳むかを決めること。

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


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- **roadmap の覚え書き `ukadoc-coverage-custody`（仮称・brief なし）を本 spec へ合流した**（棚卸⑳の裁定）。どちらも「網羅台帳の手書きの数と持ち主が、spec が完了するたびに黙って偽になる」同じ仕組みを直す仕事で、触るファイルも同じ（`doc/ukadoc-coverage/{briefing,roadmap-draft,README}.md`・`ledger/*.toml`・`crates/ukadoc-survey/src/check/`）。2 本に分けると同じファイルを順に書き換えることになる。
- 前提（`nar-install`・`popup-menu-minimal`・`default-balloon-bundle`）はすべて完了＝着手できる。優先度 中（製品は何も壊れないが、spec が 1 本完了するたびに悪くなる。09-19 から約 4 倍）。規模 M（**14〜18 タスク**。下の ⑵⑶ を機械でまとめて書き換える前提。1 行ずつ人が裁くと 20 を超える）。
- **数え直し（main `03e8d7d6`）**。左が 09-19 の覚え書き、右が今。

| 項目 | 09-19 | 今 |
|---|---|---|
| `briefing.md` の `[[owner_completed]]`（手書きで凍結した一覧） | 14 | **14**（変わらず。検査は今も `completed/` を走査しない） |
| `completed/` の `areka-P0-*` のフォルダ | 82 | **110**（全フォルダ 213） |
| 台帳の持ち主が完了済みなのに一覧に居ない | 4 本・29 行 | **16 本・128 行**（`network-update` 30・`text-decoration-canon` 16・`baseware-root-layout` 15・`ghost-install` 13・`nar-install` 11・`popup-menu-minimal` 10・`shell-balloon-switch` 7・`balloon-font-descript-keys` 7・`charset-canon` 5・`ghost-shell-balloon-switch` 4 ほか） |
| 持ち主が完了済みで状態が `vocabulary-only`（誰も仕上げられない行） | 5 | **9**（`popup-menu-minimal` の 4 行が増えた）。ほかに `degraded` で持ち主が完了済みの行が 11 |
| `roadmap-draft.md` の `[[spec]]` が完了済みを未完として載せる | 8 | **39 行のうち 20** |
| 一度も起票されなかった仕様名（`areka-P0-seriko-runtime`・`areka-P0-balloon-loader`・`areka-P0-shiori-host-32`）を引く項目 | 107 | **107**（`assets.toml` 105・`shiori.toml` 2） |
| さくらスクリプトの台帳で持ち主が空 | 263／342 | **248／342** |
| `briefing.md` 5-3 の持ち主あり／空 | 416／1,333 | **517／1,232**（合計 1,749 は不変） |
| `roadmap-draft.md` の波の欄の旧番号（W13〜W17） | 全行 | **70 か所のまま** |

- **本文の古くなった点**: 候補名 `areka-P0-network-update` の重なり（「裁定待ち」）は、その spec が完了して自然に解けた＝`roadmap-draft.md` の記述を直すだけ。`README.md` は今も、完了済みの `ukadoc-coverage-roadmap` を「統合担当」として申し送り先に書いている。
- **合流した後の範囲**:
  1. 整合検査が `.kiro/specs/completed/` を走査する（手書きの `[[owner_completed]]` をやめるか、そこから導く）。一度も起票されなかった仕様名も検査で赤にする。
  2. 持ち主が完了済みの 128 行を、実装済みへ改めるか生きている持ち主へ付け替える（機械でまとめて）。誰も仕上げられない 9 行は裁定する（`popup-menu-minimal` の 4 行は `popup-menu-residue` へ）。
  3. 実在しない仕様名を引く 107 項目を書き換える（機械でまとめて・各名前を今の持ち主へ対応付ける）。
  4. 日付つきの数の撮り直しを道具に任せる（手で直さない）・波の欄の旧番号と `network-update` の名前の記述を直す。
  5. `README.md` の「統合担当」の指示を改める。「持ち主の居ない一覧」（`\_` の仲間の 7 件・さくらスクリプトの空の 248 行・段階が仮の 11 束）は**道具が出す一覧**にし、1 行ずつの裁定は本 spec に含めない。
  6. 取り下げ予定の `emo-text-canon-residue` を `roadmap-draft.md` の `[[spec]]` の表から外し、フォルダを片付ける（残り 1 件は `shell-balloon` が引き取り済み）。
  7. 完了 `ghost-install` が申し送った「`assets.toml` の `manual_install` の束の行の語の食い違い」を引き取る。
- **合流しない物**: 覚え書きの残り 3 つ（`present-write-coherence` の未達 40 件・正典語彙の孤児 2 件・配布物を束ねる／作る側の 3 件）は製品の穴で、台帳の番ではない＝roadmap の覚え書きに残す。
- **並べ方**: 触るのは `doc/ukadoc-coverage/` と `crates/ukadoc-survey/` だけで製品のコードと共有 0。ただし**台帳を書き換える spec とは同じウェーブに置かない**（`choice-timeout-directive`＝C1・`install-companion-reading`＝C2・`ghost-standard-balloon`＝C3・`shell-companion-balloon`＝C4 の候補は台帳の行を直す。10-03 に `install-companion-canon` を 3 本へ分けた）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（14〜18 タスク・10-02 の見立てのまま）。切る: なし。
- 前提の状態: 上流なし。単独でいつでも取れるが、台帳を書き換える spec と同じウェーブに置けない（下）。
- 数え直し（main `634032f6`・10-02 の表の続き）:

| 項目 | 10-02 | 今 |
|---|---|---|
| 台帳の持ち主が完了済みなのに `[[owner_completed]]` に居ない | 16 本・128 行 | **19 本・134 行**（`status-execution-states` 3・`translate-pipeline` 2・`choice-timeout-directive` 1 が増えた。上位は変わらず `network-update` 30・`text-decoration-canon` 16・`baseware-root-layout` 15・`ghost-install` 13・`nar-install` 11・`popup-menu-minimal` 10） |
| `briefing.md` の `[[owner_completed]]` | 14 | **14** |
| `roadmap-draft.md` の `[[spec]]` が完了済みを載せる | 39 行のうち 20 | **41 行のうち 23** |
| 一度も起票されなかった仕様名を引く項目 | 107 | **107**（`assets.toml` の `areka-P0-seriko-runtime` 78・`areka-P0-balloon-loader` 27、`shiori.toml` の `areka-P0-shiori-host-32` 2） |
| 持ち主が空（4 台帳の合計） | — | 1,231（`assets.toml` 364・`sakura-script.toml` 248・`shiori.toml` 619・`property.toml` 0） |
| `roadmap-draft.md` の波の欄の旧番号（W13〜W17）を含む行 | 70 | **70** |

- 崩れた前提／古くなった位置:
  - **`install-companion-canon` の分解は、台帳の持ち主を 1 行も動かしていない**。同 spec は要件の段で閉じ（PR#221）、台帳の `owner` に一度も登記していない（`[[spec]]` の表にも行が無い）。同 spec が直す予定だった `assets.toml` の `descript_install` の `*.directory`・`*.source.directory` の行は、今も持ち主が完了 `nar-install`・状態 `implemented` のまま（上の 134 行に含まれる）。この 2 行は後継の `install-companion-reading`（C2-⑥）が、同梱の行は `ghost-standard-balloon`（C3-⑥）・`shell-companion-balloon`（C4 の候補）が書き換える予定＝**本 spec の「持ち主の付け替え」（範囲 2）と同じ行を取り合う**。本 spec は、この 3 本が触る行を付け替えの対象から外すか、3 本の着地の後に回す。
  - **`emo-text-canon-residue` の片付け**: フォルダは `.kiro/specs/areka-P0-emo-text-canon-residue/` に残り、`roadmap-draft.md` の `[[spec]]` に `none = true`・`owner_count = 0` の行がある。整合検査（`crates/ukadoc-survey/tests/consistency/spec_checks.rs` の「`[[spec]]` の各名前が `.kiro/specs/` の直下か `completed/` に在ること」）が名前を見るので、表の行を消すのとフォルダを消す（または `_rejected/` へ移す）のを同じコミットで行う。台帳の `owner` には 0 行＝それ以外の片付けは無い。残り 1 件を引き取った `shell-balloon` は完了した。
  - **brief の「検査は今も `completed/` を走査しない」は半分だけ正しい**。整合検査（`crates/ukadoc-survey/tests/consistency/documents.rs`）は既に `.kiro/specs/completed/` の直下を集めており、`spec_checks.rs` の判定 2 つ（`[[spec]]` の名前の実在・`[[owner_completed]]` の行が本当に完了済みか）で使っている。無いのは「台帳の `owner` が完了済みなのに `[[owner_completed]]` に居ない」を赤にする判定。範囲 1 は新しい集め方を作るのでなく、この判定を 1 本足す形で済む。
  - `briefing.md` の数（5-3）は `status-execution-states`・`translate-pipeline`・`choice-timeout-directive` が手で動かした（`git log 03e8d7d6..` で `doc/ukadoc-coverage/` を触ったのはこの 3 本）。
- 触るファイル（並走の照合用）:
  - `doc/ukadoc-coverage/{briefing.md, briefing-*.md, roadmap-draft.md, README.md}`
  - `doc/ukadoc-coverage/ledger/{assets,shiori,sakura-script}.toml`（持ち主の付け替え・実在しない名前の書き換え）
  - `doc/ukadoc-coverage/report/*.md`（作り直し）
  - `crates/ukadoc-survey/tests/consistency/{spec_checks.rs, documents.rs, documents_non_vacuity.rs}`（判定の追加）・必要なら `crates/ukadoc-survey/src/documents/`
  - `.kiro/specs/areka-P0-emo-text-canon-residue/`（消すか移す）
- **同じウェーブに置けない相手**（台帳の行を直す未完了 spec・C2〜C3 の今の並び）: `install-companion-reading`（C2-⑥・`assets.toml`）・`mouse-drag-events`（C2-⑦・`shiori.toml` の `OnMouseDragStart`・`OnMouseDragEnd`）・`ghost-standard-balloon`（C3-⑥・`assets.toml` の `balloon`・`default.balloon.path` と同梱の行）。ほかに台帳の `owner` に名前を持つ未完了 spec（`property-catalog-lists` 120 行・`currentghost-property-tree` 64・`anchor-tag-canon` 61・`choice-marker-styling` 39・`balloon-canon-residue` 26 ほか）は、着手した時点で同じ扱いになる。C3 の残りでは `balloon-font-file`（C3-②・`assets.toml` の `font.name` と `sakura-script.toml` の `\f[name]` を着手時に確かめる＝直す見込み）も台帳を触りうる。`surface-element-nesting`・`balloon-lifecycle-events`・`mcp-*` の 3 本の brief は台帳に触れていない（`animated-image-decode` は状態を読むだけ）。並べるなら C3 の着地の後の、台帳を触る spec が 1 本も走らない席。
- 議題（答えで作業が変わるものだけ）: 
  1. 上の `descript_install` の行（後継 3 本が触る）を本 spec の付け替えから外してよいか（外せば後継 3 本と並べやすいが、134 行の一部が残る）。
- 見つけた穴: なし（製品の穴ではない）。
