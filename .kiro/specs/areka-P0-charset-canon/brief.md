# Brief: areka-P0-charset-canon

> 起票: 2026-09-02（`/kiro-discovery`・Path C・**M 規模**）。開発者「現在の areka は任意の charset に対応してましたっけ？ emo2 は UTF-8 だけど、汎用サポートフェーズでは charset 対応が必要になります。現コードを確認し、未対応ならブリーフィングを立ち上げよ」。
> **調査の答え＝半分だけ対応**。ファイル層（descript.txt／balloon）は `charset,<name>` 宣言＋`encoding_rs` で任意ラベルに対応済み。**SHIORI/3 プロトコル層は UTF-8 固定**（Shift_JIS は「将来のシーム」として意図的に先送り済み）。**surfaces.txt は 2 箇所で decode を迂回**し UTF-8 決め打ち。本 spec はこの残り半分を ukadoc 正典どおりに閉じる。
> **配置**: M2（汎用サポートフェーズ・里々/YAYA 網羅の前提）。M1 の e2e とは共有ファイル 0。

## Problem

Shift_JIS のゴースト（里々の標準テンプレート・古い YAYA ゴーストの大半）を areka に入れると、利用者から見える結果は次の 3 つ。

1. **SHIORI の応答が読めない（明示エラー・全イベント）**: 里々は `Charset: Shift_JIS` で応答する。`parse_response` が `std::str::from_utf8` で fail-fast するため `ShioriError::Parse` になり、挨拶も何も出ない。
2. **areka の要求を SHIORI が誤読する（黙って壊れる）**: areka は `Charset: UTF-8` 固定で送る。`OnCommunicate` の利用者入力・`OnFileDrop` のパス等、日本語を含む `Reference` を Shift_JIS 想定の SHIORI が化けたまま処理する。
3. **シェルが起動しない（明示エラー）**: `surfaces.txt` が Shift_JIS だと `read_to_string` が `InvalidData` を返し、`ShellRead` で起動失敗。

emo2 は全て UTF-8 なので M1 適合には無害。toolkit 規則 6 の壊れ方は ⑴⑶＝明示エラー・⑵＝黙って壊れる。影響する既存資産の広さは**最広**（里々ゴーストほぼ全数）。

## Current State（2026-09-02 実測・着手時に再検証）

| 層 | 状態 | 証跡 |
|---|---|---|
| ファイル層（descript.txt／balloon descript／placement 保存） | ✅ 任意 charset | `areka-parsers/src/charset/{prescan,decode,model}.rs`: 冒頭 4096 バイトの ASCII 窓で `charset,<name>` を抽出（BOM 寛容）→ `encoding_rs::Encoding::for_label` で解決（Shift_JIS／UTF-8／EUC-JP 等 encoding_rs の全ラベル）→ 未宣言・未知ラベルは `DefaultEncoding::{Ansi→SHIFT_JIS, Utf8}` へ寛容フォールバック。本番の呼び出しは全て `Ansi`（`areka/src/boot_config.rs:130`・`emo2_boot/assets.rs:249,333`・`placement/source.rs:138`・`areka-emo-present/src/balloon.rs:418`）。消費者＝`package::resolve`（`resolve.rs:42`）・balloon・placement persist |
| **surfaces.txt** | ⚠ **decode 迂回** | `areka/src/emo2_boot/assets.rs:279`（起動）と `areka/src/placement/measure.rs:333`（配置採寸）が `std::fs::read_to_string` で UTF-8 決め打ち。ukadoc は surfaces\*\*\*.txt の `charset` を**ファイル毎**に指定させる。shell の lexer／decode は `charset` 行を素通し（`shell/decode.rs:11-12`）なので、読み取りを `fs::read`＋`charset::decode` に替えるだけで閉じる。テスト側の同種読取（`areka-emo-compose/src/world.rs:229`・`areka-seriko/src/resolve.rs:222`）は emo2 fixture＝UTF-8 で変更不要 |
| **SHIORI/3 プロトコル層（host-32 経路）** | ❌ **UTF-8 固定** | `shiori-host32-host/src/shiori3.rs:39-41` `enum Charset { Utf8 }`（`ShiftJis` はコメントのシームのみ）・`:100` `build_request` が `Charset: UTF-8` 固定・`:178-189` `parse_response` が `from_utf8` fail-fast・`:217` 応答の `Charset` ヘッダを読み飛ばし・`client.rs:132,140,169` が `Charset::Utf8` 固定。**意図的先送りの登記**: completed `host32-request` 要件 1.7「Shift_JIS 対応が将来含まれる場合…切替シームのみ」・completed `shiori-protocol` 要件 8.2「レガシー wire の charset 符号化は host-32 の責務」・`doc/COMPAT_ARCHITECTURE.md:85`「64bit areka 側で早期に HSTRING→Charset ヘッダ解析→charset 符号化バイト列」（置き場所は決定済み）・`:117`「Charset 交渉の具体」＝未決 TODO |
| ghost descript の `shiori.encoding`／`shiori.forceencoding` | ❌ 未解析 | `package/model.rs` の `ShioriMount` に無い。`kv::parse_kv` は `BTreeMap` で全キーを保持するため取り出しは容易 |
| SHIORI/4 in-proc（`IShiori`・HSTRING） | 対象外 | `areka-ghost/src/shiori_inproc.rs:340,380` は UTF-16 経路＝charset 概念なし |
| 32bit helper | 変更不要 | request バイト列は素通し（`shiori-host32-helper/src/shiori_proxy.rs:4,21-23`）・`load` のパスは CP_ACP 符号化済み |

**ukadoc 正典**（snapshot 2026-08-24）: `spec_shiori3` `Charset`（request／response 双方・「最初の行、または少なくとも非 ASCII 行の前」）／`descript_ghost` **`shiori.encoding`**「SHIORI との通信を指定した文字コードで行う。**SHIORI 側から Charset ヘッダが返された場合は SHIORI 側が優先される**」／**`shiori.forceencoding`**「Charset ヘッダを返したか否かに関係なく強制」／各 descript `charset` の既定＝「OS の標準設定または SSP→国際化→省略時の文字コード」。里々 wiki の実例は SSP が `Charset: Shift_JIS` で要求を送っている（`栞としての里々`・`イベントの正体`）。

## Desired Outcome

1. **交渉規則（ukadoc 準拠）**: 初期 charset ＝ `shiori.forceencoding` ＞ `shiori.encoding` ＞ 既定（`DefaultEncoding::Ansi` と同じ固定写像＝Shift_JIS・OS ロケールは読まない）。応答の `Charset` ヘッダを読み、`forceencoding` でなければ**次回以降の要求 charset に採用**する。応答で省略されたときは要求側を継承（既存要件 2.6 維持）。
2. **要求の符号化**: `String` → 選定 charset のバイト列（`encoding_rs` の `encode`）。変換不能文字の扱い（encoding_rs 既定＝数値文字参照）は要件段階で裁定。**応答の復号**: 宣言 charset で decode。不正並びを寛容（U+FFFD）にするか fail-fast を保つかは要件段階で裁定（ファイル層は寛容・現行プロトコル層は fail-fast）。
3. **surfaces.txt の 2 箇所**を `fs::read`＋`charset::decode(bytes, default)` へ（既定 Ansi・宣言優先・ファイル毎）。
4. **ログ**: 初期 charset と応答による切替を `info!`／`debug!` 各 1 行・未知ラベルは `warn!`＋既定継続（記憶 areka-log-first-no-silent-failure）。
5. **決定論テスト**: codec（Shift_JIS の要求バイト列を逐語・Shift_JIS 応答の復号・`Charset` 省略時の継承・`forceencoding` 下での応答 `Charset` 無視・未知ラベルのフォールバック）・descript キー解析（2 キー・優先順）・surfaces.txt の Shift_JIS fixture・**UTF-8 経路の不変**（emo2 e2e 全緑が檻）。
6. **実機 1 体**: Shift_JIS のゴースト（候補＝里々標準テンプレート・検体は開発者指定）で OnBoot の挨拶が化けずに出る。

## Approach

設計で 2 案から選ぶ（交渉規則は同じ・型の持ち方が違う）:

- ⒜ `Charset` enum を `Utf8 | ShiftJis | Other(..)` へ拡張。既存型を保つが「任意 charset」を列挙で表すことになり、ラベル解決を二重に持つ。
- ⒝ `Charset` を **`&'static encoding_rs::Encoding` の newtype** に置換。ヘッダ値は `Encoding::name()`（`"Shift_JIS"`／`"UTF-8"`＝ukadoc 表記と一致）、応答ヘッダの解決は `for_label`。ファイル層の `DefaultEncoding` と同じ基盤に載る。`shiori-host32-host` に `encoding_rs` 依存を追加（workspace に既存）。

推奨は ⒝（列挙しない＝任意 charset を素直に表す・記憶 canonical-not-minimal-lifecycle）。交渉状態（次に使う charset）は `Shiori3Client` のセッション状態として持ち、初期値は `areka-ghost` runtime が descript から渡す。codec（bytes ⇄ String の純関数）と交渉状態（どの charset を次に使うか）は分離して檻に入れる。

## Scope

- **In**: `shiori3.rs` codec（要求符号化・応答復号・応答 `Charset` 読取）・`client.rs` の交渉状態・`areka-ghost` runtime／`shiori_wiring` の初期 charset 配線・`package/model.rs`＋`resolve.rs` の `shiori.encoding`／`shiori.forceencoding` 解析・surfaces.txt 読取 2 箇所・決定論テスト・COMPAT §8 に 1 節（既定 Shift_JIS 固定写像＝OS ロケール不読の裁定）・COMPAT `:117` の TODO 消し込み。
- **Out**: SHIORI/4 in-proc（charset 概念なし）・SSTP／FMO／`updates2.dau`／`install.txt` の文字コード（各 M2 spec が同じ encoding_rs 基盤を再利用）・`readme.charset`（readme 表示機能が無い）・`surfacetable.txt`（未読）・x64 SHIORI/3 DLL の in-proc ロード（別件）・SAORI（SHIORI 内の事＝`not-applicable`）・OS ロケール読取（D6 固定写像を維持）。

## Boundary Candidates

- ⑴ **ファイル層の閉鎖**（surfaces.txt 2 箇所・XS・先行スライス可・M1 中でも無害）／⑵ **プロトコル交渉**（codec＋client＋配線・M）。
- ⑵ の内側: codec 純関数と交渉状態の分離。

## Out of Boundary

- 文字集合の描画（フォント・字形）＝emo-text。
- charset 未宣言の ANSI ゴーストを非日本語ロケールの OS で動かすときの挙動＝D6 固定写像を維持（変えるなら別裁定）。

## Upstream / Downstream

- **Upstream**: completed `parser-foundation`（charset module）・completed `host32-request`（codec とシーム）・completed `shiori-protocol` 8.2（host-32 への委譲境界）・`emo2-conformance-e2e`（M1 完成が M2 解禁条件・UTF-8 経路不変の檻）。
- **Downstream**: 里々/YAYA 網羅（本 spec 無しでは 1 体も動かない）・`ukadoc-survey-shiori`（`Charset`／`shiori.encoding`／`shiori.forceencoding` の台帳行＝本 spec 着地で `implemented` へ）・SSTP／NAR／更新（同じ encoding_rs 基盤を再利用）。

## Existing Spec Touchpoints

- **Extends**: なし（completed spec は不変・`host32-request` 要件 1.7 のシームを本 spec が実装する）。
- **Adjacent**: `ukadoc-survey-shiori`（同じ `shiori3.rs` に ukadoc URL コメントを置く＝後着が rebase）・`property-query-channels`（`areka-ghost/src/runtime.rs` の sink 列を共有＝別ウェーブ）・`emo2-conformance-e2e`（UTF-8 経路の檻）。

## Constraints

- **M2**（汎用サポートフェーズ）・e2e 完了後に着手。⑴ のみ開発者裁定で前倒し可。
- 32bit helper と IPC は非接触（raw bytes 素通し・凍結）。
- UTF-8 経路は 1 バイトも変えない（emo2 e2e 全緑・変異＝Shift_JIS 実装を外すと赤になる檻）。
- 1,000 行番人・`file_length_guard_test.rs` 例外表は非接触。
- 優先度 4 軸: 壊れ方＝混在（応答＝明示エラー・要求＝黙って壊れる）／伺からしさ＝テーマ 0（配管）／影響資産＝最広（里々全ゴースト）／基盤共有度＝高（SSTP／NAR が再利用）。
