# 技術設計書: areka-P0-ukadoc-survey-assets

> 本書の事実主張はすべて 2026-09-03 にこの作業ツリー（`claude/areka-p0-ukadoc-survey-33754d`・origin/main へ rebase 済み）で `file:line` を開くか、正典スナップショットを機械で数え直して確かめた。要件・ギャップ分析と食い違った箇所は「§12 実測の訂正」に一覧で置き、本文はすべて訂正後の値で書いてある。

## 1. 概要

本 spec の成果物は**文書**である。ukadoc の「資産の定義と配布・更新」24 ページ 542 項目を全数仕訳し、台帳 1 本・ブリーフィング 1 本を書き、実装済みと判定した項目の定義箇所へ正典 URL 1 行を置く。areka の実行時の振る舞いは 1 行も変わらない。

したがって本書で言う「設計」は、動くものの構造ではなく**生産手順**である。⑴ 542 項目の骨組みをどう機械で起こすか、⑵ 状態をどの順でどの規則で埋めるか、⑶ ブリーフィングに何をどの形で載せるか、⑷ URL をどのファイルの何行目に置くか、⑸ 上流の道具が無い間に何を決定論の手順で検査するか——この 5 つを決める。

利用者は 3 人いる。統合担当（`areka-P0-ukadoc-coverage-roadmap`）は台帳を集計して段階を決める。将来の実装者はブリーフィングを読んで「読まれない記述がどう扱われるか」と「nar と更新に何が要るか」を知る。台帳を読む人はソースの URL で根拠が生きていることを確かめる。

### 1.1 目標

- 台帳 `doc/ukadoc-coverage/ledger/assets.toml` に 542 項目がちょうど収まり、未分類が 0 件で、id の集合がスナップショットと完全に一致すること。
- 台帳の正しさが、上流の道具が無くても決定論の手順（§11）で確かめられること。
- ブリーフィング `doc/ukadoc-coverage/briefing-assets.md` が、未知の記述の扱い 9 種・SERIKO/MAYUNA 世代表 137 行・nar と更新の導線 6 段を持つこと。
- 実装済みと判定した項目に、スナップショットと 1 文字も違わない正典 URL がソース側に 1 行ずつ置かれること。

### 1.2 非目標

- 実装・設計の決定（展開ライブラリの選定、更新機構の構造、プラグイン／ヘッドラインの実装可否）。
- 他 3 ドメインの台帳、全体報告 `report/summary.md`、束の解説 `linkage.md`、段階 A〜E の最終順序。
- 上流の道具（調査用クレート）の実装、カタログ・テーマ定義・README の作成。
- 隣接 spec の brief の書き換え。SSP 実機との挙動比較。

## 2. 境界の約束

### 2.1 本 spec が持つもの

- `doc/ukadoc-coverage/ledger/assets.toml`（新設）——資産ドメイン 542 項目の状態・世代・別名・担当・優先度・テーマ・関連・備考。
- `doc/ukadoc-coverage/briefing-assets.md`（新設）——人手で書く読み物。
- `crates/` 配下の `ukadoc: <URL>` 1 行コメント（項目の直上は `///`・式の途中は `//`・D11）——実装済みと判定した項目の定義箇所のみ。
- `doc/ukadoc-coverage/report/assets.md`（既存・道具で再生成するのみ・手編集なし）。

### 2.2 持たないもの

- 他 3 台帳（`shiori.toml`・`sakura-script.toml`・`property.toml`）と `catalog.toml`・`values.md`・`README.md`・`report/summary.md`・`linkage.md`。
- `doc/COMPAT_ARCHITECTURE.md`・`doc/emo2-conformance-scope.md`・`.kiro/steering/roadmap.md`・隣接 spec の文書（すべて読むだけ）。
- areka の実行時の振る舞い。ログを増やす変更・分類を足す変更も行わない（要件 3.7・10.1）。
- 担当が決まっていない項目の担当割り当て（空のまま統合担当へ渡す・要件 7.5）。

### 2.3 依存してよいもの

| 依存先 | 使い方 | 区分 |
|---|---|---|
| 上流契約 `.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/requirements.md`（完了・付録 A / 付録 B）と道具 `crates/ukadoc-survey` | 台帳の形式・7 状態語彙・6 関連種別・8 テーマ・ページ割り当て・URL の書き方・id 抽出手順 | P0・変更しない |
| ukadoc スナップショット `%APPDATA%\npm\node_modules\ukagaka-doc-mcp\data\index.json`（`version` 1・`generatedAt` 2026-08-24T04:08:57.881Z） | id・見出し・本文・URL の取得元 | P0・読むだけ |
| ライブの ukadoc（4 ページのみ・要件 1.12） | 綴りの実在確認と新しい見出しの検出 | P1 |
| repo 内の既存判定表（`doc/COMPAT_ARCHITECTURE.md:128-207` の 80 行、`doc/emo2-conformance-scope.md:82-88` の 7 行、完了 spec の設計表） | 縮退の転記元・担当の取り込み元 | P1・読むだけ |
| 隣接 spec の brief 群 | 担当の取り込み | P1・読むだけ |

### 2.4 再確認が要る変化

- **上流契約の付録 A・状態語彙・ページ割り当てが改訂されたとき**——台帳の全行が影響する。
- **スナップショットが更新されたとき**——id の集合が変わるので要件 1.4 の一致が崩れる。上流契約 要件 8 の差分で見直す範囲を絞る。
- **上流の道具がさらに改訂されたとき**——所見の種類や台帳の形式が変わると本 spec の検査計画（§11）が古びる。`doc/ukadoc-coverage/README.md` と `cargo test -p ukadoc-survey` の結果で見直す。
- **担当として書いた spec 名が消えた・改名されたとき**——`owner` 欄が宙に浮く。
- **本 spec が URL を置いたソース行が別 spec の実装で消えたとき**——上流契約 要件 6.6 の検査が赤になる。

## 3. 上流契約の確定範囲と未決範囲

上流 spec は **2026-09-05 に完了して main へ入った**（PR#136）。要件・設計は `.kiro/specs/completed/areka-P0-ukadoc-survey-toolkit/` にあり、道具 `crates/ukadoc-survey` と `doc/ukadoc-coverage/` 一式（`catalog.toml` 1,749 件・4 台帳の骨組み・報告 4 本・`values.md`・`README.md`）も同じコミットで着地している。本作業ツリーは 2026-09-05 に main をマージして取り込んだ（`cargo test -p ukadoc-survey` 43 本が緑）。

**この着地により本書の前提が 2 つ反転した**。⑴ D7 が置いた「道具は未着地」は成り立たない——要件 9.1／9.2 が発効し、報告の再生成とテストの通過が完了条件になる。⑵ `doc/ukadoc-coverage/ledger/assets.toml` は**既に 542 項目の骨組みが並んでいる**（全件 `unclassified`・CRLF）ので、本 spec が骨組みを起こす仕事は消え、**その場で値を書き換えるだけ**になる（`doc/ukadoc-coverage/README.md:492-500` の合流手順「道具が先に出来上がっている場合」）。以下の表と D1・D6・D7・§6.1・§7・§10・§11 はこの事実に合わせて書き直してある。

| 事項 | 状態 | 本 spec の扱い |
|---|---|---|
| 台帳 1 項目の形（`[entry."<id>"]`＋欄）・欄名 `owner`／`supersedes` | **凍結済み**（付録 A.1・A.2） | そのまま従う |
| 状態 7 語彙・関連 6 種別・テーマ 8 つ | **凍結済み**（要件 2.2・4.3・4.4） | そのまま従う |
| doc コメントの書式 `ukadoc: <正典 URL>`・1 項目 1 行・定義箇所のみ・語彙表は頭にページ URL 1 つ | **凍結済み**（要件 5.1〜5.4・付録 A.3）。行の形は着地した実装が「行頭の空白を除いて `///`・`//!`・`//` のいずれかで始まり、`ukadoc:` の後に空白区切りの 1 語（URL）だけが続く行」と定める（`crates/ukadoc-survey/src/evidence/extract.rs:37`・`doc/ukadoc-coverage/README.md:358`） | そのまま従う。項目の直上は `///`、式の途中（引数・構造体リテラルの欄）は `//`（D11） |
| id の抽出手順（コロンで分割・`_` で分割しない） | **凍結済み**（付録 B） | そのまま従う |
| 報告 `report/<ドメイン>.md` の構成 | **凍結済み**（要件 7.1）・実物が main にある | そのまま従い、道具で作り直すだけ |
| **本文からの SSP 版番号の抽出規則** | **凍結済み・結果が `catalog.toml` の `versions` に入っている**（項目ごとの版番号の配列） | D6。本 spec は規則を再実装せず、`versions` から選ぶ |
| 台帳と報告の一致検査の実装 | **着地済み**（`cargo test -p ukadoc-survey` / `cargo run -p ukadoc-survey -- check` が 15 種の所見を出す） | §11 で上流の検査に委ね、足りない分だけ自前で足す |

## 4. アーキテクチャ（生産の流れ）

```mermaid
graph TB
    Catalog[catalog toml 1749 件 上流の道具が生成]
    Skeleton[P1 骨組み 542 件 着地済み]
    Ledger[assets toml 542 項目]
    Report[report assets md 道具で再生成]
    MachineFill[P2 機械で決まる 178 件]
    ManualFill[P3 人手の仕訳]
    Themes[P4 テーマと優先度]
    Briefing[P5 briefing assets md]
    Urls[P6 ソースの URL 1 行]
    Checks[V 決定論の検査]
    Live[ライブ ukadoc 4 ページ]
    Existing[既存の判定表と隣接 brief]

    Catalog --> Skeleton
    Skeleton --> Ledger
    Ledger --> MachineFill
    MachineFill --> ManualFill
    Existing --> ManualFill
    ManualFill --> Themes
    Themes --> Ledger
    Ledger --> Briefing
    Ledger --> Urls
    Live --> Briefing
    Ledger --> Report
    Report --> Checks
    Ledger --> Checks
    Urls --> Checks
```

**依存の向き**は一方向である。カタログ → 台帳 → （ブリーフィング｜ソースの URL｜報告）。逆流させない。とくに次の 2 つを規律として置く。

- **ブリーフィングは台帳を写す側**である。世代表 137 行も 44 行の一覧も、台帳から機械で起こすか、台帳の値をそのまま引く（要件 4.2）。ブリーフィングを直して台帳を追随させる向きは採らない。
- **ソースの URL は台帳の従属物**である。`status = "implemented"` の項目だけが URL を持ち、URL を先に置いてから状態を決めることはしない。

### 4.1 使う道具

| 層 | 選択 | 役割 | 備考 |
|---|---|---|---|
| 台帳の検査・報告の再生成 | `crates/ukadoc-survey`（上流の道具・main 着地済み） | 15 種の所見・報告 4 本の再生成・証拠と置き場の手掛かり | `cargo test -p ukadoc-survey`／`-- check`／`-- report`／`-- evidence`／`-- candidates` |
| 世代表生成・上流に無い分の検査 | Python 3.13（この環境で実測・`tomllib` 同梱） | 台帳から世代表を起こす・未分類 0 件と非接触の確認 | **`crates/` の外**（作業用の一時ディレクトリ）に置く。成果物としてコミットしない |
| ライブ確認 | WebFetch（4 ページのみ） | 綴りの実在と新しい見出しの検出 | `mcp__ukadoc` は同じスナップショットを引くので**ライブ確認には使わない** |
| 版管理 | git（`core.autocrlf` = true・`.gitattributes` 無し） | 改行の正規化 | 新規ファイルは CRLF で書く（D8） |

## 5. 設計判断

### D1: 骨組みは作らない——既にあるものを書き換える（道具の着地により改訂）

542 項目の骨組みは上流の道具が既に `doc/ukadoc-coverage/ledger/assets.toml` へ並べてある（全件 `unclassified`・id は厳密な昇順・CRLF）。本 spec が骨組みを生成する仕事は無い。

- **その場で値を書き換えるだけ**にする。id・並び順・`[ledger]` の前置きには触らない（`doc/ukadoc-coverage/README.md:496-500`）。
- 骨組みが正しいことは自分で確かめない——上流の検査（`LedgerIdNotInCatalog`・`CatalogIdMissingFromLedgers`・`LedgerOutOfOrder`・`LedgerPagesMismatch`）が毎回見ている。初版で自前に計画していた検査 3 本はこれに置き換わる（§11）。
- ただし**要件 1.3 が求める前置きのコメント 3 行は既存のヘッダに無い**（実測: 現ヘッダはファイル名・形式の正本・`[ledger]` の 3 要素だけ）。⑴ id の読み替え規則、⑵ 段階の最終順序を決めない旨、⑶ 版番号は `catalog.toml` の `versions` から選ぶ旨——この 3 行をコメントとして足す。`[ledger]` テーブル自体は書き換えないので `LedgerDomainMismatch`・`LedgerPagesMismatch` には触れないが、足した直後に検査を 1 度回して確かめる。
- 世代表とブリーフィングの表を起こす小さなスクリプトは引き続き `crates/` の外に置く（要件 10.1・10.2）。

### D2: 台帳は 4 段に分けて書く（research 案 C）

| 段 | 中身 | 完了の目印 |
|---|---|---|
| 第 1 段 | 既にある骨組み 542 件の受け入れと前置き 3 行の補記 | 上流の検査が緑（件数・id 集合・並び順） |
| 第 2 段 | 状態・別名・担当・関連。埋める順は **balloon → shell → surfaces → install／update／plugin／headline → ページ全体項目 15 件** | `unclassified` 0 件（W1） |
| 第 3 段 | テーマと優先度を束ごとに一括 | alias と not-applicable 以外の全件に優先度（W2） |
| 第 4 段 | ソースの URL とブリーフィング・報告の再生成 | URL と台帳の一致（W4）・報告が最新（上流の `DomainReportStale`） |

第 2 段の順序は「既存の判定表がある面 → 実装ゼロが確定している面 → 粒度の粗い面」である。balloon と shell は `doc/COMPAT_ARCHITECTURE.md` の 80 行と完了 spec の判定表がそのまま写せるので、判断の基準を先に固めてから残りへ進む。

### D3: `charset` の URL は 3 ページ分・`prescan.rs` に 3 行（research 判断 3 = 案 ⑴）

`charset` は本ドメインの descript 系 8 ページすべてに項目がある。areka の実装は 1 か所（`crates/areka-parsers/src/charset/prescan.rs:54`・クレート内で唯一大小文字を無視する照合）で、ファイル種別を区別しない。**種別ごとに実際に通るかどうかで割れる**。

| ページの `charset` | 通る経路 | 状態 |
|---|---|---|
| `descript_ghost` | `package/resolve.rs:64` が `charset::decode` を呼ぶ | implemented |
| `descript_shell` | 同 `:156` | implemented |
| `descript_balloon` | `crates/areka-emo-present/src/balloon.rs:418`（`read_decoded` が `decode(&bytes, DefaultEncoding::Ansi)`） | implemented |
| `descript_shell_surfaces` | 通らない。`crates/areka/src/emo2_boot/assets.rs:279` と `crates/areka/src/placement/measure.rs:333` が `std::fs::read_to_string` で読み、`shell/decode.rs` は `charset` を照合しない（明文 `:90`・`:490`） | absent・担当は `areka-P0-charset-canon` |
| `descript_shell_surfacetable`・`descript_install`・`descript_plugin`・`descript_headline` | 読む経路そのものが無い | absent・担当は空（`areka-P0-charset-canon/brief.md:51` が対象外を宣言） |

**URL は `prescan.rs:54` の直上に 3 行**（ghost・shell・balloon の 3 id）。`resolve.rs:64`・`:156`・`balloon.rs:418` は呼び出し側なので何も置かない（要件 6.2）。

### D4: 記録の段は備考に書き、分類は 3 つのまま（research 判断 5 = 案 ⑴）

要件 3.1 の 3 分類（黙って捨てる／記録を残す／エラーにする）を増やさない。段（`warn!`／`debug!`）は備考の情報として書く。要件 3.5a の裁定に従い、**`debug!` だけの記録は分類を「記録を残す」のままにして、壊れ方の判定は「黙って壊れる」にする**（既定のログ水準は `info`・`crates/areka/src/main.rs:141`）。

備考の書き方を 1 行の型に固定する。

```
壊れ方: <黙って壊れる|明示的なエラー|見た目の差>。記録: <なし|debug!|warn!|error!> <ファイルパス>::<定義名>。
```

台帳の備考にはソースの**行番号を書かない**（ファイルパス＋関数名・定数名で指す）。行番号は整理で動き、備考が黙って古びるからである。同じ裁定が同日の `areka-P0-ukadoc-survey-shiori` の要件ディスカッションで出ており（議題 1・案 ⒜）、台帳 4 本の流儀を揃える。本書の表に書いた `file:line` は設計時点の所在を示すためのもので、台帳へは写さない。

現在の記録の全数（本ドメインに関わる範囲）。

| file:line | 段 | 何を記録するか |
|---|---|---|
| `crates/areka-parsers/src/package/resolve.rs:296` | `warn!` | bindgroup の名前宣言にパーツ名が無い（`areka-parsers` 唯一の `warn!`） |
| `crates/areka-parsers/src/charset/decode.rs:35` | `debug!` | 未対応の charset ラベルを既定へ落とす |
| `crates/areka-parsers/src/charset/decode.rs:52` | `debug!` | デコード中の不正バイト列を代替文字で吸収 |
| `crates/areka-seriko/src/table.rs:111-115` | `debug!` | 間隔語 `bind` は静的な着せ替えゆえ非駆動 |
| `crates/areka-seriko/src/table.rs:118-127` | `debug!` | 未知の間隔語を元の語を添えて非駆動（`vocab` は `:123`） |
| `crates/areka-seriko/src/table.rs:128-136` | `debug!` | 将来の値を非駆動 |
| `crates/areka-emo-compose/src/method.rs:160-161` | `warn!` | 解決できない合成メソッド名を `Unknown` へ吸収 |

`areka-parsers` に `error!` は 0 件。無言で落ちる経路は `crates/areka-parsers/src/kv/parse.rs:39`（同じキーは後勝ち）・`crates/areka-parsers/src/shell/decode.rs:197-199`（第 2 欄が `overlay` 以外の `element` 行）・同 `:234-236`（`collisionex`）の 3 つ。

### D5: 世代表 137 行は台帳から機械で起こす（research 判断 7 = 案 ⑴）

要件 4.2 が「表の版番号を台帳から取り、食い違わせない」と命じている。上流の道具はブリーフィングを見ないので、この一致を保つ仕組みは本 spec が持つしかない。世代表は作業用スクリプトが台帳を読んで Markdown の表を吐き、ブリーフィングへ貼る。表の生成規則（列・並び順・注記）は §9.2 に置く。

### D6: 版番号は `catalog.toml` の `versions` から選ぶ（道具の着地により改訂）

抽出規則は上流が凍結し、**その結果が `doc/ukadoc-coverage/catalog.toml` の各項目の `versions` に入っている**。本 spec は規則を再実装せず、この配列から選ぶだけにする。

着地したカタログを担当 24 ページ 542 件で数え直した実測（2026-09-05）: 項目数 **542**、`versions` が空でない項目 **144**、2 つ以上持つ項目 **7**。設計時に自前の規則で数えた値と 3 つとも一致した。したがって仕訳の作業量に変化は無い。

`introduced` は 1 つの版だけを書く欄なので、2 つ以上ある 7 項目は**最も古い版**を書く（その項目が最初に現れた版という欄の意味に合う）。`versions` が空の 398 項目は `""`（要件 2.7）。書いた値が `versions` に無ければ上流の検査が `IntroducedNotInCatalogVersions` で赤にする。自前で版番号を検査する必要は無くなった。

> 設計の初版は「語境界付き」の規則で 7 件を空にする案を採っていたが誤りだった。語境界の定義しだいで数が動く（Python の `\b` なら 8・英数字と下線とピリオドだけを境界に取れば 2）。カタログの値を写す今の形では、数える必要そのものが無い。

### D7: 上流の道具は着地済み——要件 9.1／9.2 で進める（2026-09-05 に反転）

道具は main にある。**要件 9.3 の退避路は使わない**。したがって完了条件は次の 2 つを含む。

- `doc/ukadoc-coverage/report/assets.md` を `cargo run -p ukadoc-survey -- report` で作り直し、**台帳と同じコミットに入れる**（片方だけ入れると次の人の検査が落ちる・README:528）。要件 9.1。
- `cargo test -p ukadoc-survey` が通ること。要件 9.2。加えて要件 6.8 のために `cargo test --workspace` を URL 追加の前後で 1 度ずつ走らせる。

ブリーフィングの冒頭には、退避路ではなく**着地済みである事実と、報告を再生成した旨**を書く（要件 9.3 の「未着地なら」という条件が成り立たないので、その文は書かない）。作り直すのは自分の報告 1 本だけで、`summary.md`・`catalog.toml`・`values.md`・他 3 台帳には手を出さない（要件 9.4・9.5・10.3）。

### D8: 新規ファイルは CRLF で書く（research §6-5）

この作業ツリーのテキストファイルはすべて復帰文字付きである（`doc/COMPAT_ARCHITECTURE.md` は 216 行すべて CRLF）。`.gitattributes` は無く、`core.autocrlf` = true が変換を担う。既存の `assets.toml` は既に CRLF なので、書き換えるときに改行の流儀を崩さない。新設する `briefing-assets.md` と、世代表を吐くスクリプトの出力も CRLF にそろえる（Python なら `open(..., "w", newline="\r\n")`）。

### D9: 正典に居場所が無い areka の語は台帳に行を作らない（research §6-7・要件 4.3）

`surface.append`・`kero.surface.alias`・`bind+random` はスナップショット全 1,749 件で見出し 0・本文 0 である（2026-09-03 に再確認）。要件 1.4 が件数を 542 に、要件 1.6 がページを 24 に固定しているので、新しい行を作る余地は無い。

| areka の語 | 認識箇所 | 書く先 |
|---|---|---|
| `surface.append` | `crates/areka-parsers/src/shell/decode.rs:127` | ページ全体項目 `ukadoc:manual_shell` の備考 ＋ ブリーフィングの surfaces.txt 節 |
| `kero.surface.alias` | 同 `:122` | 同上（`manual_shell` は `alias.txt` が旧仕様で surfaces.txt に統合された旨を書くだけで、綴りが違う） |
| `bind+random` | 同 `:391`・駆動は `crates/areka-seriko/src/table.rs:107-109` | `ukadoc:descript_shell_surfaces:random_2c_6570_5024:1` の備考 ＋ 世代表の注記（要件 4.3 が明示） |

3 語ともライブ確認の対象（要件 1.12 ⑴ は前 2 語）。結果はブリーフィングに「実在する／しない・正しい綴り」で記す。**ライブで実在が確かめられない限り、この 3 語に対応する台帳の項目を `implemented` にはしない**（根拠が立たないため）。

### D10: URL を置くのは `implemented` の項目だけ

要件 6.1 は `implemented` に URL を義務づけ、要件 6.6 は未実装に何も書くなと言う。`degraded`・`vocabulary-only` はどちらの文にも直接当たらない。**最も狭い読み方**を採り、URL を置く集合を `status = "implemented"` の項目だけに限る。理由は 2 つ。

- 検査が 1 本の等式になる——「ソース中の URL の集合」＝「台帳で `implemented` の id の集合」。どちらの向きのずれも W4 で赤にできる（上流の `ImplementedWithoutEvidence` は片側しか見ない）。
- 上流契約 要件 6.6 は `implemented` 行に証拠があることしか求めておらず、狭く置いても赤にならない。

**1 つの id に対する URL は repo 全体で 1 行だけ**にする（要件 6.3）。2 つのファイルが同じ語を定義している場合は、意味を所有する側に置く（例: 描画メソッド `overlay` は `crates/areka-emo-compose/src/method.rs:148`。`crates/areka-parsers/src/shell/decode.rs:198` は行の受理側なので置かない）。

### D11: 式の途中に置く URL 行は `//` で書く（設計検証 重大 3 への対処・道具の着地で裏取り済み）

URL を置く先の多くは**式の途中**である——`crates/areka-parsers/src/balloon/parse.rs:71-101` の 28 か所は `WindowPosition::new(...)` などの引数、`crates/areka-parsers/src/package/resolve.rs:69-72` の 4 か所は `GhostNames { ... }` の欄。この位置に `///` を置くと `rustc` が `unused doc comment` の警告を出し（検証で同型の最小コードを実際にコンパイルして確認）、rustdoc にも出ない。ワークスペースに `deny(warnings)` は無いのでテストは赤にならないが、ビルドのたびに警告が並ぶ。

上流の設計（行 761）は URL を拾う行の形を「`///`・`//!`・`//` のいずれかで始まる」と定めているので、**項目（関数・定数・構造体の欄の定義）の直上は `///`、式の途中は `//`** と書き分ける。どちらも証拠として同じに拾われる。同じ書き分けを `areka-P0-ukadoc-survey-shiori`（配列要素は `//`）と `areka-P0-ukadoc-survey-property`（値の定義行に `//`）も採っており、4 本で流儀が揃う。着地した実装がこの 3 形を等しく拾うことは `crates/ukadoc-survey/src/evidence/extract.rs:37`（`MARKERS = ["///", "//!", "//"]`）で確かめた。

なお要件 6.4 の「語彙表の先頭にページ URL 1 つ」という書き方は本ドメインでは使わない。唯一の候補である合成メソッドの種別表（`crates/areka-emo-compose/src/method.rs:186-204` の 19 種）は、実導出を持つのが `Overlay` だけ（`:130-132`）で表の中身が `vocabulary-only` になるため、D10 の下では URL を置く対象にならない。

## 6. データの定義

### 6.1 既にある骨組みの受け入れと前置きの補記（第 1 段・要件 1.2〜1.10）

骨組みは道具が作った（D1）。実測（2026-09-05・マージ直後）で次を確かめてある。

- `[entry."..."]` の塊は**ちょうど 542 個**、全件 `status = "unclassified"`（要件 1.4・2.2 の出発点）。
- `[ledger]` の `domain = "assets"`・`pages` は担当 24 ページを名前順で持つ（要件 1.3 の前半・1.6）。
- 欄は `status`／`introduced`／`owner`／`priority`／`values`／`links`／`note` の 7 つで、上流契約 付録 A.2 の順（要件 1.2）。
- id は符号化されたまま写されており、改行は CRLF（要件 1.8・D8）。

本 spec がここで足すのは**前置きのコメント 3 行だけ**である（要件 1.3 の後半・8.7）。

1. id の読み替え規則——「ukadoc の見出しにある `*` は areka の照合では 1 個以上の数字にあたる。例: `sakura.bindgroup*.default` ↔ `sakura.bindgroup0.default`」
2. 「段階 A〜E の最終順序は決めない。決定は `areka-P0-ukadoc-coverage-roadmap` に委ねる」
3. 「登場した版は `doc/ukadoc-coverage/catalog.toml` の `versions` から選ぶ。2 つ以上あるときは最も古い版」

`[ledger]` テーブルと項目の塊は書き換えない。ukadoc の本文は写さない（要件 1.9）。アンカーを持たないページ全体の項目 15 件は既に同じ形で入っているので、粒度が粗いことを備考に書くだけでよい（要件 1.10）。要件 1.7 の退避路（カタログが無い場合に付録 B で直接写す）は、カタログが着地したので**発動しない**。

### 6.2 1 項目の欄（上流契約 付録 A.2 を再掲・変更しない）

| 欄 | 型 | 本 spec での決まり |
|---|---|---|
| （表のキー） | `[entry."<id>"]` | スナップショットの id をそのまま。文字順・重複なし |
| `status` | 文字列 | 7 語彙のみ。完了時に `unclassified` は 0 件 |
| `introduced` | 文字列 | D6 の規則。無ければ `""` |
| `alias_of` | 文字列 | `status = "alias"` のときだけ。指す先の状態は `alias` でない（要件 2.4） |
| `supersedes` | 文字列の配列 | 任意 |
| `owner` | 文字列 | 担当 spec 名。未定は `""`（要件 7.5） |
| `priority` | 文字列 | `^[A-E][0-9]+$`。`alias`・`not-applicable` は `""`（要件 8.6） |
| `values` | 文字列の配列 | 8 テーマ名のみ（要件 8.1） |
| `links` | インラインテーブルの配列 | `{ kind = "...", to = "..." }`。`kind` は 6 種・`to` はスナップショットに実在する id（要件 9.7） |
| `note` | 複数行文字列 | D4 の 1 行型を含む。ukadoc の本文は写さない（要件 1.9） |

### 6.3 機械で決まる 178 件（第 2 段の下書き・要件 2.1）

| 規則 | 件数 | 決め方 |
|---|---|---|
| 規則 1: areka の受理キー表と見出しが文字列一致 | 62 | ゴースト 7・シェル 8・バルーン 29・surfaces 18 |
| 規則 2: 合成メソッドの写像表と突き合わせ | 55 | `crates/areka-emo-compose/src/method.rs:173-204` が解ける 38 名 → `vocabulary-only`、解けない 17 名 → `absent`（`:160-161` で `warn!`） |
| 規則 3: 読む経路が無いページ | 61 | `descript_plugin` 13 ＋ `descript_headline` 9 ＋ `descript_install` 15 ＋ `spec_update_file` 9 ＋ ページ全体項目 15 |
| 合計 | **178** | 残り 364 件は人手。ただし大半は素直に `absent` で、実作業は備考・テーマ・優先度の文章 |

**規則 1 の受理キー表は `areka-parsers` だけでは足りない**（設計検証 重大 2）。`crates/areka/src/placement/` が本ドメインの descript キーを別途読んでいる——`config.rs:138`（`seriko.zorder`）・`:139`（`seriko.sticky-window`）・`:140`（`seriko.dpi`）・`:227`（`seriko.alignmenttodesktop`／`seriko.alignmentondesktop` の定数・`sakura.`／`kero.`／`char*.` の各形と全体形のカスケードは `:232-236`）と、`source.rs:45`（`SHELL_DPI_KEY` = `seriko.dpi`）・`:48`（`BALLOON_DPI_KEY` = バルーン descript の `dpi`）。第 2 段の仕訳ではこれらの読取も受理キー表に含め、`absent` へ落とさない。ただし `build_placement_config` は `#[allow(dead_code)]` の足場（`config.rs:125`）で本番経路から呼ばれていない形もあるので、`implemented` か `vocabulary-only` かは項目ごとに本番経路（`placement/mod.rs` → `main.rs` → `emo2_boot/frame/wiring.rs`）まで追って決める。62 件という数はこの分だけ増えうる。

規則 1 の一致は実測で裏が取れている。バルーンは 30 キーのうち 28 が `descript_balloon` の見出しと 1 文字も違わずに一致し、一致しない 2 つ（`writing_mode`・`budoux_newline`）は areka 独自の拡張で正典に項目が無い。ゴーストは 7/7 一致。シェルだけは形が違い、ukadoc の `sakura.bindgroup*.default` に対して areka は接頭辞＋番号＋接尾辞で照合するので、§6.1 の読み替え規則 1 行で橋を架ける。なお `descript_shell` には `char*.bindgroup*.*`・`char*.bindoption*.group` という別系統の項目もあり、areka はこちらを照合しない（`crates/areka-parsers/src/package/resolve.rs:111-121` の接頭辞は `sakura.`／`kero.` の 2 つだけ）。

### 6.4 別名の向きと「対象外」の決め方（要件 2.5・2.6）

**別名**（`status = "alias"`）は、上流契約 要件 4.1 の順で決める。⑴ 正典本文の注記（廃止予定・旧・統合された旨）→ ⑵ SSP 版番号 → ⑶ 人手の判断。**どの手掛かりで決めたかを必ず備考に書く**（要件 2.5）。最も新しい書式を正典とし、それ以外を別名にする。

第 1 段でそのまま決まる好例が本ドメインに 2 系統ある。

- 合成メソッドの `ukadoc:descript_shell_surfaces:bind:2` の本文が「現在は `add` が互換。処理の内容は `overlay` と同義」と自分で向きを書いており、`crates/areka-emo-compose/src/method.rs:148` の `"overlay" | "add" | "bind" => Overlay` とぴたり一致する。
- 旧書式の `overlaymultiply`・`overlayscreen` は、同 `:176-177` が `blend-multiply-fast`・`blend-screen-fast` へ明示写像している。

別名の連鎖は作らない（指す先の状態が `alias` であってはならない・要件 2.4・上流の `AliasChain`）。`alias_of` の逆向きを書きたいときは `supersedes` を使う。

**対象外**（`not-applicable`）は、SSP 以外のベースウェア専用の記述に付ける候補とし、根拠を備考に書く（要件 2.6）。本ドメインでの該当は少数で、いずれもページ全体項目に集中する（MATERIA の注記 8・CROW 5）。ベースウェア名を挙げているだけで SSP でも有効な記述は対象外にしない。

**「正典どおりでないなら実装したことにしない」（開発者裁定 2026-09-05）。** 受理キーが本番の経路まで届いていても、値の水準で正典と違う振る舞いをするなら `implemented` とは書かない。分かれ道は 2 つだけである。

- 食い違いを引き受ける行が `doc/COMPAT_ARCHITECTURE.md` の沈黙ルール対応表または `doc/emo2-conformance-scope.md` の見直し表に**ある** → `degraded`。その行を第 1 列の項目名で備考に引く（要件 2.8 のまま・行番号は書かない）。
- 引き受ける行が**無い** → `absent`。備考に「areka が実際に受けるもの」「正典とどこがどう違うか」「両方の表を当たったが引き受ける行が無いこと」を平易な言葉で書く。

この裁定は要件を変えない——要件 2.8 は `degraded` の側の書き方をそのまま定めており、`absent` の側は元から根拠の指定を持たない。適用範囲は第 2 段の機械で決まる分だけでなく、**第 2 段以降の仕訳すべて**である（人手の仕訳 364 件を含む）。
この裁定で `implemented` から外れた項目にはソースの正典 URL を置かない（D10）ため、§7.2 の URL の見積りは仕訳の確定後に引き直す。

### 6.5 テーマと優先度の付け方（第 3 段・要件 8）

**テーマ**（8 つのみ・要件 8.1〜8.4）。「この項目が無いと利用者はゴーストの何を失うか」に答えられるものだけを付け、答えられなければ空にする。既定で付けない群は、配布者向けページ（`dev_bind`・`dev_nar`・`dev_ownerdraw`・`dev_shell`・`dev_shell_error`・`dev_update`・`memo`）、プラグインとヘッドラインの descript、トランスレータ。外す場合は理由を備考に書く。更新機構に属する 14 項目（`spec_update_file` 9 ＋ `descript_install` の `refresh`／`refreshundeletemask`／`*.refresh`／`*.refreshundeletemask` 4 ＋ `manual_update` 1）には必ず「更新」を含める（要件 8.3）。

**優先度**は 1 件ずつではなく**束ごとに一括**で付ける。手順を固定する。

1. 壊れ方の段を決める（黙って壊れる ＞ 明示的なエラー ＞ 見た目の差）。判定は D4 の記録の実測に基づき、根拠を備考に書く（要件 8.8）。
2. テーマの数で段階を動かす。2 つ以上なら 1 段繰り上げてよい。1 つなら同じ段階の先頭寄り。0 個かつ見た目の差以下なら E 候補。
3. 同じ段階の中の数値は、影響する既存資産の広さ → 依存する基盤の共有度 の順で決める（要件 8.5 の固定序列）。
4. 同じ束に属する項目は**同じ優先度**を書き、束の名前を備考に残す。数値の一意性は求めない。

段階の名前は `.kiro/steering/roadmap.md` の登記（A そこにいて触れて話す／B 迎えて育てて見送る／C 察してくれる／D 仲間がいる／E 周辺）を読み替えの目安に使うが、**最終順序は決めない**（要件 8.7）。

## 7. ファイル構成計画

### 7.1 書き換えるファイルと新設するファイル（成果物）

```
doc/
└── ukadoc-coverage/
    ├── ledger/
    │   └── assets.toml          # 既存（骨組み 542 件）。値を書き換える。本 spec が唯一の書き手
    ├── briefing-assets.md       # 新設。人手で書く読み物（§9 の目次）
    └── report/
        └── assets.md            # 既存。道具で再生成するだけ・手編集しない（D7）
```

`doc/ukadoc-coverage/` は上流の着地で既に存在する。新設するのは `briefing-assets.md` の 1 本だけである。`catalog.toml`・`values.md`・`README.md`・`report/summary.md`・他 3 台帳には**手を出さない**（要件 9.4・9.5・10.3・README:526）。`linkage.md` は作らない。

### 7.2 変更するファイル（`crates/` の doc コメントのみ）

| ファイル | 置く URL の数（上限） | 置く場所 | 行の形（D11） |
|---|---|---|---|
| `crates/areka-parsers/src/charset/prescan.rs` | 3 | `:54` の照合の直上（D3） | `//`（`if` の腕の中） |
| `crates/areka-parsers/src/package/resolve.rs` | 12 | ゴースト 6 キーは各引き行（`:69`・`:70`・`:71`・`:72`・`:78`・`:83`）の直上、シェルの合成形 6 は定数群（`:111-121`）の直上にまとめて 6 行 | ゴースト 6 は `//`（構造体リテラルの欄）。シェル 6 も `//`（`///` だと `:110` の既存 doc へ合流して `SAKURA_BINDGROUP_PREFIX` の説明文になるため） |
| `crates/areka-parsers/src/balloon/parse.rs` | 28 | 各引き行の直上（`writing_mode`・`budoux_newline` は正典項目が無いので置かない） | `//`（すべて `X::new(...)` の引数） |
| `crates/areka-parsers/src/shell/decode.rs` | 8 | `element*`（`:197`）・`collision*`（`:234`）・`animation*.interval`（`:323`）・`animation*.pattern*`（`:334`）・`animation-sort`（`:501`）・`collision-sort`（`:502`）・`bind`（`:387`）・`random`（`:388`）。`descript`／`surface`／`surface.append`／`kero.surface.alias`／`ascend`／`descend` は正典項目が無いので置かない | `//`（`match` の腕・`if` の条件） |
| `crates/areka-emo-compose/src/method.rs` | 1 | `overlay`（`:148`）。ほかは `vocabulary-only` ゆえ置かない（D10） | `//`（`match` の腕） |
| `crates/areka/src/placement/config.rs` | 5 | `seriko.zorder`（`:138`）・`seriko.sticky-window`（`:139`）・`seriko.dpi`（`:140`）・`seriko.alignmenttodesktop`／`seriko.alignmentondesktop`（`:227` の定数の直上に 2 行）。**設計初版の `emo2_boot/frame/zorder_descript.rs:47` は誤り**（同行は doc コメントの一行で読取ではない・§12 訂正 11） | `:138-140` は `//`（構造体リテラルの欄）、`:227` は `///`（関数内の `const` 定義の直上） |
| `crates/areka/src/placement/source.rs` | 2 | `SHELL_DPI_KEY`（`:45`）・`BALLOON_DPI_KEY`（`:48`） | `///`（定数定義の直上・既存 doc の末尾に 1 行足す） |

**上限は 59 行**である。実際に置く数は状態が `implemented` に確定した項目数で決まる（`degraded`・`vocabulary-only` と判定した項目には置かない・D10。`placement/` の 7 か所は本番経路まで追って決める・§6.3）。URL を置く対象は本ドメインの項目に限り、他ドメインの項目の定義箇所には置かない（要件 6.9）。

行数の上限テストは `crates/**/*.rs` を走査する（`crates/log-capture-kit/tests/workspace_scan/mod.rs:38` が上限 1,000・`:82` が `root.join("crates")`・`:103` が `.rs` だけ）。追加後の行数は `resolve.rs` 949→961、`balloon/parse.rs` 168→196、`shell/decode.rs` 558→566、`method.rs` 395→396、`prescan.rs` 60→63、`placement/config.rs` 703→708、`placement/source.rs` 284→286 で、いずれも上限を超えない（W6 で確かめる）。

### 7.3 作業用（コミットしない）

`crates/` の外の一時ディレクトリに 2 本置く。`gen_seriko_table.py`（§9.2）・`check_ledger.py`（§11 のうち上流に無い分だけ）。骨組み生成の `gen_skeleton.py` は道具の着地で不要になった。生成規則と検査項目は本書に全部書いてあるので、失われても同じ結果を再現できる。

## 8. 担当 spec の取り込み

要件 7.2 が名指しする 15 本に加えて、要件確定後に main へ現れた 2 本を取り込む（要件 7.1「既存 spec が担当している項目は担当の欄に書き、新しい追跡先を作らない」）。担当 spec 名の実在は 2026-09-03 に確認した。

| 担当 spec | 取り込む項目 | 出所 |
|---|---|---|
| `areka-P0-balloon-canon-residue` | バルーンの残語彙 **12 項目すべて**（brief の Scope 行が書く「10 項目」は採らない・要件 7.3） | `brief.md:11-16`・`:24-27`・`:33`・`:35` |
| `areka-P0-surfaces-basepos` | `point.basepos.x`／`.y` | `brief.md:38` |
| `areka-P0-text-decoration-canon` | バルーン descript の `font.*` 系と `disable.font.*` | `brief.md:33` |
| `areka-P0-anchor-tag-canon` | `anchor.font.*`／`anchor.notselect.font.*`／`anchor.visited.font.*` | `brief.md:30`・`:16` |
| `areka-P0-choice-marker-styling` | バルーン descript の `cursor.*` | `brief.md:26`・`:8` |
| `areka-P0-charset-canon` | `shiori.encoding`／`shiori.forceencoding`／surfaces.txt の `charset` | `brief.md:50` |
| `areka-P0-scope-zorder-pinning` | シェル descript の `seriko.zorder` | 完了 spec |
| `areka-P0-windowposition-limit` | `descript_balloon` の `windowposition.x`／`.y`／`.limit`（要件 7.2 は「ゴースト側」と書くが、本ドメインで `windowposition` を見出しに持つ正典項目はバルーンの 3 件だけ・`descript_ghost` には無い——§12 訂正 12） | 完了 spec |
| `areka-P0-kero-balloon`／`-balloon-visibility`／`-balloon-vertical-canon`／`-balloon-offset-dpi` | バルーンの系列名・表示寿命・縦書き・単位空間 | 完了 spec |
| `areka-P0-bindoption-exclusivity` | `bindoption*.group` | 完了 spec |
| `areka-P0-package-mount` | ゴースト descript の起点と install.txt の対象外宣言 | 完了 spec |
| `areka-P0-shell-parse`／`-balloon-parse` | 転記層の範囲 | 完了 spec |
| **`areka-P0-makoto-dll-host`（追加）** | `ukadoc:descript_ghost:makoto_2c_30d5_30a1_30a4_30eb_540d:1`（本ドメインで `makoto` を見出しに持つ唯一の項目——2026-09-03 に全 542 件で確認） | `brief.md:10`・`:37`・`:57` |
| **`areka-P0-translate-pipeline`（追加）** | ページ全体項目 `ukadoc:manual_translator` | `brief.md:9`・`:47` |

`ukadoc:manual_translator` は 2 本が半分ずつ主張する（`translate-pipeline` が継ぎ目と順序、`makoto-dll-host` が DLL のホスティング）。`owner` は 1 つしか書けないので、ページが説明する仕組みの入口を持つ `areka-P0-translate-pipeline` を担当とし、備考に `areka-P0-makoto-dll-host` が後半を持つことを書き、`links` に `{ kind = "same-feature", to = "ukadoc:descript_ghost:makoto_..." }` を置く。

**担当が空のまま残るもの**（要件 7.5）: `descript_install`・`descript_shell_surfacetable`・`descript_plugin`・`descript_headline` の `charset`（`areka-P0-charset-canon/brief.md:51` が対象外を宣言）、install／update／nar の全項目、プラグインとヘッドラインの descript、配布者向けページ。

**名前が同じで id が違う項目**は同一視しない（要件 7.6）。備考に区別を書く。代表例は `homeurl`（本ドメイン 5 ページ＋`list_propertysystem`＋`list_shiori_resource`）と、`descript_ghost` と `descript_shell` の両方にある `name`・`sakura.name`・`sakura.name2`・`kero.name`（areka はゴースト側だけを読む——`crates/areka-parsers/src/package/resolve.rs:69-72`——ので状態が割れる）。

**隣接 brief の是正候補**（要件 7.4・brief は書き換えない）。ブリーフィングに 1 節として並べ、当該項目の備考にも「担当 spec の記述が古い」と 1 行書く。現時点で確定しているのは 4 件——⑴ `balloon-canon-residue/brief.md:58` の Scope 行が「10 項目」のまま（実際は 12）、⑵ `charset-canon/brief.md:51` の対象外宣言により 4 ページの `charset` が担当なしになる、⑶ `zorder-property/brief.md:52` の三重所有が未決、⑷ `text-decoration-canon/brief.md:33` の「13 キー」に名前の列挙が無い（`descript_balloon` の `font.*` は実測 14 種）。

## 9. ブリーフィングの構成

`doc/ukadoc-coverage/briefing-assets.md` は人手で書く（要件 5.1）。節を次の順で置く。

### 9.1 冒頭（D7 の差し替え点）

上流の道具が着地済みであることと、`report/assets.md` を道具で作り直した旨を 1 段落で書く（D7。要件 9.3 の退避路は条件が成り立たないので書かない）。基準時点（スナップショット `generatedAt` = 2026-08-24T04:08:57.881Z・ライブ確認日）もここに置く。

### 9.2 SERIKO/MAYUNA 世代別対応表（要件 4）

`descript_shell_surfaces` 137 項目を 1 行ずつ。列は「項目 id・見出し・登場した版・areka の状態」（要件 4.1）。台帳から機械で起こす（D5）。表に添える注記は 4 つ。

- 間隔語のうち実際にアニメーションを駆動するのは `random` と `bind+random` の 2 語だけで、`bind` は駆動しない。転記側 `crates/areka-parsers/src/shell/decode.rs:380-396`・駆動側 `crates/areka-seriko/src/table.rs:105-137`（`Random` は `:106`、`BindRandom` は `:107-109`、`Bind` の非駆動と記録は `:110-117`）（要件 4.3）。
- `bind+random` に対応する正典項目は無いので `random` の備考に書いた（D9）。
- 合成メソッドの実導出は `overlay` だけ（`crates/areka-emo-compose/src/method.rs:130-132`）（要件 4.4）。
- 当たり判定は矩形のみ。`collisionex`（円・楕円・多角形）は何も記録せずに読み飛ばす（`crates/areka-parsers/src/shell/decode.rs:234-236`）（要件 4.5）。縮小の出所は `doc/emo2-conformance-scope.md:82`（要件 4.6）。

見出しが `bind` で重なる 2 項目（`ukadoc:descript_shell_surfaces:bind:1` = 間隔語、`:2` = 合成メソッド）は id で区別して別々の行に載せる（要件 4.7）。

### 9.3 未知の記述の扱い（9 種・要件 3）

定義ファイル種別ごとに 1 節。各節に ⑴ 分類（3 つのいずれか 1 つ）、⑵ file:line の根拠、⑶ 記録の段、⑷ 壊れ方の判定、⑸ 「その記述を読むのは誰か」（転記層で止まるのか下流のどのエンジンまで届くのか）、⑹ 「成立に要る基盤は何か」（例: オーナードローメニュー・更新機構）を書く（要件 3.2・3.6）。

| 節 | 分類 | 根拠 |
|---|---|---|
| ゴーストの descript | 黙って捨てる | `crates/areka-parsers/src/kv/parse.rs:20`（KV 化）・`:26`（最初の読点で分割）・`:39`（後勝ち・記録なし）。受理は `package/resolve.rs:69-83` の 6 キー |
| シェルの descript | 黙って捨てる | 同上。受理は `package/resolve.rs:111-121` の定数と `:146-218` の照合（6 形）。唯一の `warn!` は `:296` で、これは未知キーではなく bindgroup の名前宣言の不備 |
| surfaces.txt | 黙って捨てる | `crates/areka-parsers/src/shell/decode.rs`。塊の見出しは `:118`・`:122`・`:127`・`:132` の 4 語だけ。`charset` は照合されず素通り（明文 `:90`・`:490`）。例外として未知の間隔語だけが下流で記録を残す（`crates/areka-seriko/src/table.rs:118-127`・段は `debug!`＝既定では見えない・要件 3.5a） |
| バルーンの descript | 黙って捨てる | `crates/areka-parsers/src/balloon/parse.rs` の完全一致引き 31 か所（30 キー）。明文は `:9`・`:39`・`:121-125` |
| install.txt | 黙って捨てる（ファイル全体） | 読む経路が無い。宣言は `crates/areka-parsers/src/package/resolve.rs:8`、影響しないことを固定するテストは `crates/areka-parsers/src/package/validation_tests.rs:113`・`:122-125`・`:127`（要件 3.8） |
| プラグインの descript | 黙って捨てる（ファイル全体） | 解析コードが無い。名前の予約のみ（`crates/areka-sylphya/src/vocab/dotted.rs:25`・`:104`） |
| ヘッドラインの descript | 黙って捨てる（ファイル全体） | 同 `:24`・`:104` |
| surfacetable.txt | 黙って捨てる（ファイル全体） | `crates/` に `surfacetable` を含む Rust の行が 0 件 |
| 更新ファイル（updates2.dau・updates.txt・delete.txt） | 黙って捨てる（ファイル全体） | Rust から参照する行が 0 件 |

`areka-parsers` の記録経路が 3 つしか無いこと（`warn!` 1・`debug!` 2・`error!` 0）と、それ以外がすべて無言であることを 1 節として明記する（要件 3.3）。`collisionex` が何も記録せずに読み飛ばされることは、対応する台帳の項目の備考にも file:line 付きで書く（要件 3.4）。扱いが「黙って捨てる」に当たる項目は、上流契約 要件 4.7 の壊れ方 ⑴ に当たるかを判定し、「どの記録が出るか・出ないか」を備考に書く（要件 3.5）。

### 9.4 nar インストールとネットワーク更新の導線（要件 5）

6 段（入手 → 展開 → 配置 → 起動 → 更新 → 削除）。各段に ⑴ 必要な正典項目の id、⑵ 最小成立要件（install.txt の解釈・zip の展開・配置の規則・updates2.dau の照合・delete.txt・更新イベントとの繋がり）、⑶ areka の現状（実装ゼロ）を並べる（要件 5.2・5.3）。

- **areka の現状は実測値で書く**（要件 5.6）——`crates/` 配下の Rust コードから `updates2.dau`・`updates.txt`・`.nar`・`OnUpdate` を参照する行はいずれも 0 件、zip 展開の依存なし、ネットワーク入出力なし。
- **正典書式の実ファイルが試験用ゴーストに 7 本ある**ので、各段の実例に使う。`crates/pilot/examples/shiori-host-32/fixtures/emo2/updates.txt:1-2`（`charset,UTF-8` ＋ `file,...` 行）・`.../fixtures/emo2/ghost/master/updates.txt`・`.../fixtures/emo2/delete.txt:1-2`・`.../fixtures/emo2/install.txt:1-6`・`.../fixtures/emo2/emo2-kakukaku/install.txt`・`.../fixtures/emo2-kakukaku-offsetdpi/install.txt`・`.../fixtures/emo2-kakukaku-wplimit/install.txt`（後 2 本は `emo2/` の中ではなく兄弟——§12 の訂正 4）。
- **既存の判断記録が 1 件も無い**こと（沈黙ルール対応表に該当行 0 行）と、既存の言及がいずれも「対象外」の宣言であることを書く（要件 5.7）。実在を確かめた箇所は `doc/emo2-conformance-scope.md:73`・`:75`・`:76`・`:92` と `crates/areka-parsers/src/package/resolve.rs:8`（要件が挙げる `:89` は空行——§12 の訂正 5）。
- 本ドメインの外へ伸びる先は台帳の `links` で指し、項目を複製しない（要件 5.4）。少なくともさくらスクリプト側の `\![execute,install,...]`・`\![execute,createnar]`・`\![execute,createupdatedata]`・`\![update,...]`・`\![updateother,...]` と、shiori 側の `OnInstallComplete`／`OnInstallCompleteAll`／`OnInstallRefuse`／`OnInstallReroute`／`OnNarCreating`／`OnNarCreated`／`OnUpdatedataCreating`／`OnUpdatedataCreated`／`OnUpdateProcessExec`／`OnUpdateBegin`／`OnUpdateReady`／`OnUpdateComplete`／`OnUpdateFailure` を持つ（要件 5.5）。
- 実装方式は決めない（要件 5.8）。nar の作成側は導線の対象外候補として区別し、台帳には優先度だけを付ける（要件 5.9）。
- 将来 spec の自然な境界を 3 つ挙げるだけにする——「定義ファイルの解釈（既存の転記層の拡張）」「配布と更新（新しい基盤）」「surfaces.txt の SERIKO/MAYUNA（単独で 1 本になる大きさ）」（要件 5.10）。

### 9.5 沈黙ルール対応表 44 行の一覧（要件 2.9）

`doc/COMPAT_ARCHITECTURE.md:128-207` の 80 行のうち本ドメインの項目に触れる行の**一覧**を 1 節として載せる。行は**行番号ではなく表の第 1 列（項目名）で指す**——行番号は追記で動くが項目名は動かない（D4 と同じ理由・shiori の裁定に揃える）。表にドメインを示す欄が無く 44 と 16 が機械で再現できないため、この一覧が数の根拠になる。縮退（`degraded`）と判定した項目の備考には転記元の行を項目名で必ず書く（要件 2.8）ので、逆引きでも数え直せる。

### 9.6 ライブ確認の結果（要件 1.12）

WebFetch で 4 ページを引く（`manual_shell`・`descript_shell_surfaces`・`descript_balloon`・`descript_shell`）。`mcp__ukadoc` は同じスナップショットを引くので使わない。

- ⑴ `surface.append`・`kero.surface.alias` の綴りと正典上の実在を `manual_shell`／`descript_shell_surfaces` で確かめ、「実在する／しない・正しい綴り」を書く。
- ⑵ 版番号が新しい 3 ページのライブの見出し一覧とスナップショットの見出し一覧を突き合わせ、増えた見出しを §9.7 へ回す。
- 他の 21 ページはライブを見ない。ネットワークが使えない場合は「未確認」と理由を書き、D9 の判定（`implemented` にしない）を据え置く。

### 9.7 未収載の候補（要件 1.11）

スナップショットに無い正典項目（2.8.83 以降の追加など）を、ページ名と見出しで並べる。**台帳には書かない**（id はカタログと完全一致でなければならず、追加は上流契約 要件 8 のスナップショット更新で入る）。

### 9.8 隣接 spec の是正候補（要件 7.4）

§8 の 4 件と、調査中に見つかったものを並べる。brief は書き換えない。上流へ回すものも同じ節に置く——要件 7.2 の「ゴースト側 `windowposition.*`」は本 spec の要件側の記述誤り（§12 訂正 12）。設計初版が挙げていた「doc コメント `///` は `//` を含む読み替えが要る」は**取り下げる**——着地した実装が `///`・`//!`・`//` の 3 つを等しく拾い（`crates/ukadoc-survey/src/evidence/extract.rs:37`）、`doc/ukadoc-coverage/README.md:358` がその旨を明記しているため、是正の要る食い違いは無かった。

## 10. 進めかたと段取り

| 段 | 作業 | 出来上がるもの | 依存 |
|---|---|---|---|
| 0 | ライブ確認 4 ページ（§9.6） | 綴りの確定・新しい見出しの一覧 | なし。**先に済ませる**（D9 の判定と §9.7 の材料になるため） |
| 1 | 骨組みの受け入れと前置きの補記（§6.1） | 前置き 3 行が入った `assets.toml` | 0 は不要 |
| 2 | 機械で決まる 178 件（§6.3） | 状態が入った 178 件 | 1 |
| 3 | 人手の仕訳 364 件（balloon → shell → surfaces → install ほか → ページ全体） | `unclassified` 0 件 | 2・既存判定表・隣接 brief |
| 4 | テーマと優先度（§6.5） | 優先度が入った台帳 | 3 |
| 5 | ソースの URL（§7.2） | `crates/` の doc コメント | 4（`implemented` の集合が確定してから） |
| 6 | ブリーフィング（§9） | `briefing-assets.md` | 0・4・5 |
| 7 | 報告の再生成と検査（§11・D7） | `report/assets.md` が台帳と同じコミットに入り、全検査が緑 | 6 |

段 1〜4 は台帳 1 本しか触らないので、途中で止まっても `unclassified` の残り件数で進み具合が読める。段 5 を段 4 より前に出さない——URL は台帳の従属物である（§4）。

## 11. 検証計画

検査は**上流の道具に任せられる分は任せる**。`cargo test -p ukadoc-survey`（マージ直後に 43 本が緑であることを実測）と `cargo run -p ukadoc-survey -- check` が 15 種の所見を出し、その多くが本書の初版で自前に計画していた検査と同じものである。**`crates/` にテストを足さない**（要件 10.1）。

**上流に任せる検査**（自前で書かない）。

| 上流の所見 | 代わりに満たす要件 | 初版で自前に計画していた番号 |
|---|---|---|
| `LedgerIdNotInCatalog`・`CatalogIdMissingFromLedgers`・`CatalogIdInMultipleLedgers`・`LedgerIdPageMismatch` | 1.4・1.6 | V1・V2 |
| `LedgerOutOfOrder` | 1.5 | V3 |
| `LedgerDomainMismatch`・`LedgerPagesMismatch` | 1.3 の `[ledger]` の部分 | — |
| 状態語・テーマ名・関連種別の綴り違い（読み込みの段で落ちる）・`UnknownTheme` | 2.3・8.1 | V5・V8 |
| `AliasChain` | 2.4 | V6 |
| `LinkEndpointMissing` | 9.7 | V7 |
| `IntroducedNotInCatalogVersions` | 2.7 | V10 |
| `SourceUrlNotInCatalog`・`ImplementedWithoutEvidence` | 6.7 と、`implemented` に証拠がある側 | V11 の一部 |
| `DomainReportStale` | 9.1 | — |

**自前で要る検査**（上流が見ていない分だけ・作業用スクリプト `check_ledger.py` 1 本）。

| 番号 | 検査 | 要件 |
|---|---|---|
| W1 | `unclassified` が 0 件（文字列検索でも 0 件）。上流は未分類を誤りとしないので、これは本 spec の完了条件そのものである | 2.2 |
| W2 | `alias`・`not-applicable` 以外の全項目の `priority` が `^[A-E][0-9]+$`。この 2 状態は `""` | 8.6 |
| W3 | 担当が決まっている束の `owner` に空欄が無い（上流は `owner` の中身を一切見ない） | 7.1・7.2 |
| W4 | 台帳の `implemented` の id 集合と、ソースの URL 行から引いた id 集合が**両向きで**一致し、同じ id の URL が 2 行以上ない。上流の `ImplementedWithoutEvidence` は片側しか見ない | 6.3・6.5・6.9・D10 |
| W5 | 世代表 137 行の版番号が台帳の `introduced` と一致（表の再生成結果とブリーフィング中の表がバイト一致。比較は作業ツリーのファイル同士で行い、改行は CRLF に揃える・D8） | 4.1・4.2 |
| W6 | URL を足した各ファイルの行数が 1,000 未満（上限テストの前倒し・`crates/log-capture-kit/tests/workspace_scan/mod.rs:38`） | 6.8 |
| W7 | `git diff --name-only`（比較元は分岐点）の変更対象が `doc/ukadoc-coverage/ledger/assets.toml`・`report/assets.md`・`briefing-assets.md`・`crates/` の URL コメント・本 spec 自身の `.kiro/specs/areka-P0-ukadoc-survey-assets/` だけ。`catalog.toml`・`values.md`・`README.md`・`report/summary.md`・他 3 台帳・`doc/COMPAT_ARCHITECTURE.md`・`.kiro/steering/roadmap.md`・隣接 spec の文書に差分が無い（パスの実在を先に確かめてから差分を取る） | 10.1〜10.6 |
| W8 | 完了時に `cargo run -p ukadoc-survey -- report` を走らせ、`report/assets.md` を台帳と同じコミットに入れる。`cargo test -p ukadoc-survey` が緑 | 9.1・9.2 |

**道具そのものを較正する**（緑は道具が壊れていても出る）。W1〜W5 は、それぞれ「わざと 1 か所壊した写し」を作って赤になることを 1 度は確かめてから本番に当てる。上流の検査も同じく素通りでないことを 1 度確かめる——例えば `introduced` に `9.9.9` を入れて `IntroducedNotInCatalogVersions` が出るか、`status` を綴り違いにして落ちるか。確かめたら元に戻す。

**doc コメント追加の前後で既存テストの結果を変えない**（要件 6.8）。段 5 の前後で `cargo test --workspace` を 1 度ずつ走らせ、結果が同じであることを確かめる。差が出たら doc コメントを疑う。

## 12. 実測の訂正

要件・ギャップ分析と実測が食い違った箇所。**要件は書き換えない**。台帳とブリーフィングに書き写すときは、下の右列の値を使う。

| # | 出所の記載 | 実測（2026-09-03） |
|---|---|---|
| 1 | ギャップ分析 §1.2「シェルの照合の本体は `resolve.rs:171-215`」 | 要件本文の **`:146-218`** が正しい（`read_bindgroup_defaults` 全体）。`:171-215` は腕の途中で切れている |
| 2 | ギャップ分析 §1.2「未知キーの明文は `balloon/parse.rs:122-126`」 | **`:121-125`**（`:126` は `BalloonCursor::new(` のコード行） |
| 3 | ギャップ分析 §3-4・§6-2「語境界の有無で動くのは 5 件」 | **数は語境界の定義しだいで動く**（Python の `\b` なら 8・英数字と下線とピリオドだけを境界に取れば 2・上流の規則なら **0**）。設計初版は 7 件と書いたがこれも `\b` による数え方で、しかも `ukadoc:dev_update` を落としていた。上流の規則を採る D6 の下では境界事例そのものが無い |
| 4 | ギャップ分析 §1.2 の試験用ファイル 7 本の場所 | `emo2-kakukaku-offsetdpi/install.txt` と `emo2-kakukaku-wplimit/install.txt` は `fixtures/emo2/` の中ではなく **`fixtures/` 直下**（`emo2/` の兄弟）。7 本という数は正しい |
| 5 | 要件 5.7「`doc/emo2-conformance-scope.md:75`・`:76`・`:89` の 3 か所」 | **`:89` は空行**。実在する言及は `:73`（install.txt の type）・`:75`（delete.txt は M1 マウントでは無視可）・`:76`（NAR インストーラは M1 範囲外）・`:92`（生態系拡張として NAR インストール等が M1 後）の 4 か所。主張の中身（いずれも「対象外」の宣言）は正しい |
| 6 | ギャップ分析 §1.2「`prescan.rs:52` が最初の読点で分割」 | **`:51`**（`:52` は `continue`）。`:54` の照合は一致 |
| 7 | ギャップ分析 §1.2 の `areka-seriko/src/table.rs` の行 | 駆動の判断は **`:105-137`**、`Random` は **`:106`**、`BindRandom` は **`:107-109`**、`Bind` の非駆動は **`:110-117`**（`debug!` 自体は `:111-115`）、`Other` は `:118-127`（一致）、将来の値は **`:128-136`** |
| 8 | ギャップ分析 §1.2 の `areka-emo-compose/src/method.rs` の行 | 実導出の判定は **`:130-132`**（`:129` は説明文）、名前の解決は `:142` が関数の宣言で小文字化が **`:143`**・記号の除去が **`:145`**、種別 19 種は **`:186-204`**、語彙 10 種を数えるテストは **`:236-248`**。`:148`・`:153`・`:160-161`・`:173`・`:176-177` は一致 |
| 9 | ギャップ分析 §1.2「`validation_tests.rs:123-126` の doc コメント・`:128` のテスト本体」 | doc は **`:122-125`**、テスト関数は **`:127`**。`:113` は一致 |
| 10 | ギャップ分析 §6-6「上限テストの起点は `workspace_scan/mod.rs:81`」 | **`:82`**。上限値 `:38`・`.rs` だけを拾う `:103` は一致 |
| 11 | ギャップ分析 §1.2・設計初版 §7.2「`seriko.zorder` はアプリ側 `emo2_boot/frame/zorder_descript.rs:47` が読む」 | **`:47` は doc コメントの一行**。実際の読取は `crates/areka/src/placement/config.rs:138`。同じファイルが `seriko.sticky-window`（`:139`）・`seriko.dpi`（`:140`）・`seriko.alignmenttodesktop`／`alignmentondesktop`（`:227`）も読み、`placement/source.rs:45`・`:48` が `seriko.dpi`・バルーンの `dpi` を読む（設計検証 重大 2） |
| 12 | 要件 7.2「`areka-P0-windowposition-limit`（ゴースト側 `windowposition.*`）」 | 本ドメインで `windowposition` を見出しに持つ正典項目は **`descript_balloon` の 3 件**（`.x`・`.y`・`.limit`）だけ。`descript_ghost` には無い。担当の取り込み先はバルーンの 3 件 |
| 13 | 要件 5.7「`doc/emo2-conformance-scope.md:75`・`:76`・`:89`」 | 要件側を `:73`・`:75`・`:76`・`:92` へ訂正済み（設計ディスカッション 2026-09-03） |

**rebase による移動は 1 件も無い**。この作業ツリーが取り込んだ上流の 2 コミットは `crates/wintf/src/ecs/visual/draw/builder.rs` だけを変えており、`areka-parsers`・`areka-seriko`・`areka-emo-compose`・`areka` のいずれにも差分が無い（`git diff --name-only` で確認）。上の 10 件はすべて、もとの引用の取り方の粗さである。

## 13. 危険と対処

| 危険 | 影響 | 対処 |
|---|---|---|
| スナップショットが 2.8.82 で止まっている | 2.8.83 以降に追加された項目が台帳から丸ごと抜ける。既存項目の本文が改訂されていれば別名の向きの判断が古い本文に基づく | 要件 1.12 のライブ確認を 3 ページに限って行い、増えた見出しを §9.7 に列挙する。台帳には入れない |
| 版番号の書き値がカタログと食い違う | 上流の `IntroducedNotInCatalogVersions` が赤になる | D6 が `catalog.toml` の `versions` から選ぶだけにする。自前で規則を再実装しない |
| `///` を式の途中に置くと doc コメントとして成立しない | `unused doc comment` の警告がビルドのたびに並ぶ（テストは赤にならない） | D11（式の途中は `//`）。上流の走査は `//` も拾う |
| 名前の重なりで別ページの項目を取り違える | 同じ名前で状態が割れる（`charset` は 8 ページ・`homeurl` は 7 ページ・`name` 系は ghost と shell の両方） | 名前で引かず、必ず「ページを決めてから」引く。区別を備考に書く（要件 7.6）。URL も id ごとに書く（要件 6.5） |
| ブリーフィングと台帳が二重管理になる | 世代表 137 行と台帳の版番号がずれる。上流の道具はブリーフィングを見ないので気づけない | D5（表は台帳から機械で起こす）＋ W5 |
| 台帳だけを入れて報告を入れ忘れる | 次の人の検査が `DomainReportStale` で落ちる（README:528） | D7。報告の再生成を完了条件に固定し、台帳と同じコミットに入れる（W8） |
| doc コメントの追加でファイルが 1,000 行を越える | ワークスペースのテストが赤になる | W6。現状の最大は `resolve.rs` 949 行＋12 行＝961 行で余裕がある |
| 台帳が数千行になる | 影響なし | 行数の上限テストは `crates/**/*.rs` だけを走査する（`crates/log-capture-kit/tests/workspace_scan/mod.rs:82`・`:103`） |

## 14. 要件対応表

| 要件 | 受入基準 | 実現する設計要素 |
|---|---|---|
| 1 台帳の受け入れと全数収容 | 1.1・1.2・1.3・1.4・1.5・1.6・1.7・1.8・1.9・1.10 | §6.1 前置きの補記、§7.1 ファイル構成、D1、上流の id 検査 |
| | 1.11 | §9.7 未収載の候補 |
| | 1.12 | §9.6 ライブ確認、§10 段 0 |
| 2 仕訳と状態語彙 | 2.1・2.2・2.3 | §6.2 欄の定義、§6.3 機械で決まる 178 件、D2、W1・上流の語彙検査 |
| | 2.4・2.5・2.6 | §6.4 別名の向きと対象外の決め方、§6.2、D9、上流の `AliasChain` |
| | 2.7 | D6、上流の `IntroducedNotInCatalogVersions` |
| | 2.8・2.9 | §9.5 沈黙ルール表 44 行の一覧、D4 の備考の型 |
| 3 未知の記述の扱い | 3.1・3.2・3.6・3.8 | §9.3 の 9 節の表 |
| | 3.3・3.4・3.5・3.5a | D4 記録の段の全数表と壊れ方の判定、§9.3 の末尾 |
| | 3.7 | §2.2 持たないもの、W7 |
| 4 SERIKO/MAYUNA 世代表 | 4.1・4.2 | §9.2、D5、W5 |
| | 4.3 | D9、§9.2 の注記 |
| | 4.4・4.5・4.6・4.7 | §9.2 の注記 4 つ |
| 5 nar と更新の導線 | 5.1・5.2・5.3・5.6・5.7 | §9.4 |
| | 5.4・5.5 | §9.4 の `links`、§6.2 の `links` 欄、上流の `LinkEndpointMissing` |
| | 5.8・5.9・5.10 | §9.4 の末尾 3 項、§1.2 非目標 |
| 6 実装済みの証拠 | 6.1・6.2・6.3・6.4・6.6・6.9 | D10、§7.2 URL を置く場所の表 |
| | 6.5・6.7 | W4、上流の `SourceUrlNotInCatalog` |
| | 6.8 | §11 の末尾、W6 |
| 7 担当の取り込み | 7.1・7.2・7.3・7.5 | §8 取り込み表 |
| | 7.4 | §8 末尾、§9.8 |
| | 7.6 | §8 の「名前が同じで id が違う項目」、§13 |
| 8 テーマと優先度 | 8.1・8.2・8.3・8.4 | §6.5 テーマ、上流の `UnknownTheme` |
| | 8.5・8.6・8.8 | §6.5 優先度の 4 手順、W2 |
| | 8.7 | §6.1 の前置き 2 行目 |
| 9 報告と整合検査 | 9.1・9.2・9.3 | D7、§9.1、W8 |
| | 9.4・9.5・9.6 | §2.2、§7.1、W7 |
| | 9.7 | §6.2 の `links`、上流の `LinkEndpointMissing` |
| 10 非接触と非重複 | 10.1・10.2・10.3・10.4・10.5・10.6 | §2.2、§7 ファイル構成、W7 |
| | 10.7 | 本書と成果物の書き方の規律（平易な語だけを使い、プロジェクト内でしか通じない言い回しを持ち込まない） |
| | 10.8 | §3 上流契約の確定範囲、§12（変更せず訂正として記録する） |

## 15. 未決のまま残すもの

いずれも本 spec の作業を止めない。設計としての扱いを決めてある。

1. **上流の道具の版番号抽出規則と URL 行の形**——上流の設計は別ブランチに存在し未 main（§3）。D6・D11 は同じ規則を写しているが、main に載るまでに変わる余地がある。着地後に V10・V11 で照合する。
1. **`surface.append`・`kero.surface.alias` の正典上の実在**——ライブ確認（§9.6）で決める。確かめられない間は当該の areka の語を根拠にした `implemented` 判定をしない（D9）。
2. **`areka-P0-text-decoration-canon` が言う「13 キー」の実体**——当該 brief に名前の列挙が無く、`descript_balloon` の `font.*` は実測 14 種。台帳側で名前を確定し、是正候補として §9.8 に記録する（brief は書き換えない）。
3. **`ukadoc:manual_translator` の担当**——2 本が半分ずつ主張する。§8 の規則（`owner` は `translate-pipeline`・備考と `links` に `makoto-dll-host`）で扱い、最終判断は統合担当へ渡す。
