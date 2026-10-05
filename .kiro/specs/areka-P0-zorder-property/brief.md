# Brief: areka-P0-zorder-property

> **種別**: 追跡 spec（正典先送り 4 点セット＝完全語彙＋縮退シーム＋**追跡 spec＋roadmap 明記**）。sylphya（統一プロパティシステム）＋ zsp グループ台帳の帰属。
> **源**: `areka-P0-scope-zorder-pinning`（zsp）要件 13。2026-08-27 の zsp 要件ディスカッション議題 3 で「追跡先の実在検証＝拾い手ゼロ」と判定され、開発者裁定「今作っておくべき」により即日起票。
> **着手ゲート**: **M1 外（M2 解禁ゲート棚）**。`currentghost` 配下プロパティの実導出を解禁する時、または実ゴーストが本プロパティを使うことが実測された時に解禁。

## Problem

ukadoc プロパティ **`currentghost.seriko.zorder`**（SSP 2.8.78・[SET有効]）は、現在の zorder 設定状態のスクリプトからの読み書きを定める。zsp（M1）はタグ `\![set,zorder,...]`／`\![reset,zorder]`／descript `seriko.zorder` の 3 入口を実装するが、**プロパティの読み書きは実装しない**（zsp 要件 13.1）。放置＝手抜きにせず、後続が実装できる完全な仕様をここに台帳化する。

## 完全語彙（ukadoc 2.8.78・zsp brief:18 から転記）

- **読み**: 現在の設定状態を返す。グループ内は**カンマ区切り**・グループ間は**セミコロン区切り**・要素は**手前から順**。
- 明示モード `s0,b0,s1;s2,b2` と数値モード `0,1;2,3` は**排他**（混在不可）。
- **書き込み**: 現在の設定の**完全置換**（タグ `\![set,zorder]` がグループを追加式に足すのと異なる）。
- **空文字列**の書き込みで全解除。
- 要素 **2 個未満**のグループは無視。

## Current State（2026-08-27）

- 縮退シーム: sylphya の `currentghost` 配下は M1 では NOT_FOUND 応答＝**何も実装しないことで現行どおりの応答が成立**（zsp 要件 13.2）。
- sylphya の SET 有効プロパティ一覧（`crates/areka-sylphya/src/vocab/dotted.rs`・21 項）に `seriko.zorder` は**入れない**（zsp 議題 3 裁定＝動かない名前の先行登録はしない。本 brief が語彙の正本）。
- zsp が着地させる**グループ台帳**（areka 側・scope／窓種別のまま保持）が、本プロパティの読み書きの唯一の情報源になる想定。

## Desired Outcome

（解禁時）読み＝グループ台帳の現在状態を上記の正典書式へ直列化して返す。書き＝完全置換としてグループ台帳へ反映し、タグと同じ検証規則（モード混在拒否・重複拒否・2 個未満無視など＝zsp の解釈純関数）を通す。

## Approach

- zsp のトークン解釈（parse）と対になる直列化（serialize）を足し、往復（parse→serialize→parse が恒等）を決定論テストで固定する見込み。
- 「名前で引ける値は 1 機構」（sylphya）に従い、バッキングは zsp グループ台帳＝可視性状態の二重帳簿を作らない先例（balloon-visibility R7.5）と同型。

## Scope

- **In**: プロパティ読み書きの実導出・SET 有効一覧への追加・書式検証規則の zsp との共有・タグ入口との整合（プロパティ書込後の是正発火）。
- **Out**: タグ／descript 入口（zsp で完成・不変）・窓の是正機構そのもの・他の `currentghost.*` プロパティ。

## Upstream / Downstream

- **Upstream**: `areka-P0-scope-zorder-pinning`（グループ台帳・解釈純関数・COMPAT §8 登記）＝**2026-09-02 完了・`.kiro/specs/completed/areka-P0-scope-zorder-pinning/`**。⚠着手前に `areka-P0-currentghost-property-tree` との**二重所有裁定**〔`currentghost.seriko.zorder` を本 spec が単独で持つのか、プロパティ樹 spec が一括で持つのか〕を 1 度で決めること（roadmap M2 ゲート節に持ち越しを登記済み）。
- **Downstream**: M2 互換面拡大の各ゴースト適合。

## Constraints

- M1 では着手しない（`surfaces-basepos`・`balloon-canon-residue` と同じ M2 解禁ゲート棚）。
- zsp の COMPAT §8 登記と本 brief は相互参照で重複記載しない（語彙の正本は本 brief）。

---

> **📌 2026-09-02 棚卸⑫**——上流 zsp は **完了**（PR#126・09-02）＝`.kiro/specs/completed/areka-P0-scope-zorder-pinning/`（本文の `zsp brief:18` 参照は移動後パスで再確認）。`dotted.rs` 21 項に `zorder` 不在（grep 0）・件数檻 `:191`＝一致。
> **⚠ 二重所有は三重所有だった**: `currentghost-property-tree`（`seriko.*` 14 項一括）に加え **`property-query-channels`** も「SET 台帳 21→26 の追随に `seriko.zorder`・`seriko.sticky-window` を含む（SET 経路の所有者として）」を In に掲げる。本 brief の「dotted.rs に入れない・本 brief が語彙の正本」と食い違う。**棚卸⑫の推奨＝切り出し**: 値の導出（parse の対＝serialize・往復恒等）は本 spec 単独／tree は `seriko.*` から `zorder` を除外／**`SET_EFFECTIVE` の 1 行は channels⑶（台帳スライス・S）が持ち本 spec は台帳に触れない**。完了 spec zsp 要件 13.3/13.4 の追跡先は本 spec 単独と記録済み。裁定は着手前に 1 度（roadmap 干渉台帳）。zsp 残件 B-4 のうち **13.3/13.4 の檻は本 spec の語彙記録行そのもの**＝実質の受け皿（12.1〜12.4 は `zorder-chain-residue`）。


---

> **📌 2026-09-11 棚卸⑬（三重所有の仮裁定）**——`currentghost.seriko.zorder` の所有は **値の導出（parse／serialize・往復恒等）＝本 spec 単独**・**SET 台帳行（`SET_EFFECTIVE` の `seriko.zorder`／`seriko.sticky-window`）＝`areka-P0-sylphya-set-ledger`**（`property-query-channels` ⑶ から独立）・**`currentghost-property-tree` は `seriko.*` から `zorder` を除外**。本 spec は `dotted.rs` に触れない（brief 本文の「dotted.rs に入れない」はそのまま・台帳行は set-ledger が置く）。編集集合の主戦場を明記: `crates/areka/src/placement/zorder_group_ledger.rs`（zsp グループ台帳・解釈純関数）＋`doc/COMPAT_ARCHITECTURE.md` §8。編成＝**W15**（`sylphya-set-ledger` W13・`property-query-channels` W14 の後）。規模 S・要件定義は Opus。

> **📌 2026-09-17 `areka-P0-sylphya-set-ledger` が **完了**（PR#151・`.kiro/specs/completed/areka-P0-sylphya-set-ledger/`）**——上の 09-11 の注記の「SET 台帳行（`SET_EFFECTIVE` の `seriko.zorder`）＝`sylphya-set-ledger`」は**開発者裁定 2026-09-13 で覆った**。`currentghost.seriko.zorder` は**語彙台帳の行も値の導出もともに本 spec が持つ**（先送り維持＝sylphya の語彙表へ名前だけの先行登記もしない・`SET_EFFECTIVE` は 21→**25** で `seriko.zorder` を含まない）。`seriko.sticky-window` の行だけが `sylphya-set-ledger` に登記された。本文 brief:22 の「21 項」は現在 25 項。調査台帳 `property.toml` の本項目の担当欄は本 spec 名で記入済み。記録の正本は `doc/COMPAT_ARCHITECTURE.md` §8 の【所有の相互参照】行。⚠ 先送りの見張り `crates/areka/src/placement/zorder_property_deferral_tests.rs` の t_zpd40 は `crates/areka-sylphya/src` 全ソースを `zorder` の小文字部分一致で走査する＝本 spec が語彙表へ登記するときはこのテストを同じ変更で改める。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 優先度 低。規模 S。読み書きの解析と台帳は `crates/areka/src/placement/zorder_group_ledger.rs` に在り、文字列へ戻す関数はまだ無い。
- **brief に無い問い 2 つ**: 台帳に「全部置き換える」口が無い（在るのは `try_add_tag_group`・`set_descript_base`・`reset_to_descript`）／空の文字列を書いたとき descript の基本の組も消すか。
- 書く側は `property-query-channels` の受け口を通る。読む側は単独で出して試験できる。`zorder-chain-residue` の B-4（`doc/COMPAT_ARCHITECTURE.md` の行）を引き取ってよい。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（5〜8 タスク）。ただし書き込みを届ける先を本 spec が作るなら S〜M（8〜11）。切る: なし。
- 前提の状態: `scope-zorder-pinning` は着地済み。読む道（`property-query-channels`・`mcp-get-property`）はまだ。
- 崩れた前提／古くなった位置:
  - 台帳 `crates/areka/src/placement/zorder_group_ledger.rs`（633 行）の公開の操作は今も `parse_zorder_tokens`・`try_add_tag_group`・`set_descript_base`・`reset_to_descript`・`groups`・`version`。文字列へ戻す関数と「全部置き換える」口は無い（前回どおり）。
  - **読む値は動く**: タグ `\![set,zorder]`・`\![reset,zorder]` で台帳は実行中に変わる。sylphya に動く値を出す口が無い（`SylphyaPublisher` は静的・SHIORI 照会・永続だけ）＝`currentghost-property-tree` が作る口に乗るか、本 spec が先に作るか。
  - **書く値の届け先が無い**: sylphya の SET で運行の値に分類されたものは `RuntimeCommandSink`（`crates/areka-sylphya/src/actor.rs`）へ渡す決まりだが、届け先は未登録（受けても警告を残して捨てる）。`currentghost.seriko.zorder` の書き込みを台帳まで届けるには、この届け先を登録する（UI のスレッドの台帳へ渡す）仕事が要る。届け先の登録は他の運行の値（`mousecursor.*` など）と共通の口になる。
  - 先送りの見張り `crates/areka/src/placement/zorder_property_deferral_tests.rs` の `t_zpd40` は `crates/areka-sylphya/src` の全ソースを `zorder` で走査する＝語彙表へ載せるときはこのテストを同じ変更で改める（前回どおり）。
- 触るファイル（並走の照合用）:
  - `crates/areka/src/placement/zorder_group_ledger.rs`（文字列へ戻す・全部置き換える）と兄弟のテスト・`placement/zorder_property_deferral_tests.rs`
  - `crates/areka-sylphya/src/{actor.rs, vocab/dotted.rs}`（`SET_EFFECTIVE` に `seriko.zorder`・届け先の登録）
  - 届け先を UI へつなぐ所（`crates/areka-ghost/src/sylphya_wiring.rs`・`crates/areka/src/emo2_boot/mod.rs` の見込み）
  - `doc/COMPAT_ARCHITECTURE.md` §8・`doc/ukadoc-coverage/ledger/property.toml`（1 行）
- 議題（答えで作業が変わるものだけ）: 前回の 2 つ（「全部置き換える」口・空文字で descript の基本の組も消すか）に加えて、`RuntimeCommandSink` の届け先の登録を本 spec が作るか、`property-query-channels`（書く道）か `currentghost-property-tree` が作るか（最初に要る spec が作る、が素直）。
- 見つけた穴: なし。並走の照合: `currentghost-property-tree`・`property-catalog-lists` と sylphya の `actor.rs`・`dotted.rs` を分け合う＝直列。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（5〜8）・書き込みの届け先を作るなら S〜M（8〜11）のまま。切る: なし。
- 前提の状態: `currentghost-property-tree`（動く値の口）はまだ＝着手は待ち。読む道は `mcp-get-property`（✅ 10-04）でできた。書く道（`\![set,property]`）の `property-query-channels` はまだ。
- 崩れた前提／古くなった位置: なし。`crates/areka/src/placement/zorder_group_ledger.rs`（633）は C3 で変わっていない（文字列へ戻す関数と「全部置き換える」口は無いまま）。先回りの見張り `placement/zorder_property_deferral_tests.rs` の `t_zpd40_the_property_system_sources_never_mention_the_name` は在る。sylphya の `actor.rs` の `RuntimeCommandSink` は説明文の言い換えだけで、届け先は未登録のまま。
- 触るファイル: `crates/areka/src/placement/{zorder_group_ledger.rs, zorder_property_deferral_tests.rs}` と兄弟のテスト・`crates/areka-sylphya/src/{actor.rs, vocab/dotted.rs}`・届け先を UI へつなぐ所（`crates/areka-ghost/src/sylphya_wiring.rs`・`crates/areka/src/emo2_boot/mod.rs` の見込み）・`doc/COMPAT_ARCHITECTURE.md` §8・`doc/ukadoc-coverage/ledger/property.toml`（1 行）。
- 議題（答えで作業が変わるものだけ）: 前回の 3 つのまま（全部置き換える口・空文字で descript の基本の組も消すか・`RuntimeCommandSink` の届け先の登録をどの spec が作るか）。
- 見つけた穴: `property-name-case-fold` の調べで、SET の仕分け `classify_set` は `SET_EFFECTIVE` と大小まで一致する名前だけを運行の値に回す。本 spec が `seriko.zorder` を `SET_EFFECTIVE` に載せた後も、`CURRENTGHOST.SERIKO.ZORDER` は自由な名前として保存へ落ちる（向こうが着地すれば解ける）。
