# Brief: areka-P0-makoto-dll-host

> 起票: 2026-09-02（`/kiro-discovery`・Path D の 2 本目・**L 規模**）。翻訳経路の **後半**＝MAKOTO/2.0 DLL のホスティング。前半は [areka-P0-translate-pipeline](../areka-P0-translate-pipeline/brief.md)（継ぎ目・展開順序・OnTranslate）で、本 spec は前半が用意する `Translator` フックへ DLL 鎖を差し込む。
> **種別**: 互換機能の新設（32bit MAKOTO DLL・任意 charset・付け外し命令）。emo2 は使わない＝**M2 ゲート扱い**。
> **ブリーフィング段階の裁定（2026-09-02・開発者）**: ⑷ 実機サインオフは**本物の MAKOTO DLL 1 本**（YAYA as MAKOTO の UTF-8 改良版）＋自前テスト DLL ⑸ **任意の charset 対応は常に欲しい**（当初は SHIORI 側 wire も本 spec で広げる裁定＝**rebase 時に同日起票の [areka-P0-charset-canon](../areka-P0-charset-canon/brief.md)（追記(91)・SHIORI/3 の Charset 交渉＋surfaces.txt）が SHIORI 側を所有していると判明**→本 spec は charset-canon を**上流**に据えてその符号化器と交渉規則を MAKOTO wire に再利用し、SHIORI 側の配線は持たない） ⑹ **シェル側 MAKOTO も含める**（ゴースト側→シェル側の鎖）⑺ spec 名は本名で確定。
> ⚠ **一次資料**: MAKOTO/2.0 の wire 規格ページは ukadoc に無い（`spec_makoto.html` は 404・MCP スナップショットにも無し）。正典は materia（偽春菜）の原典 `usada.sakura.vg/contents/makoto.html`（サイトは消滅・Wayback 2008-02-10 のスナップショット http://web.archive.org/web/20080210074700id_/http://usada.sakura.vg/contents/makoto.html）。**要求行は `EXECUTE MAKOTO/2.0`**。Mac 互換ベースウェア Ourin が使う `TRANSLATE Sentence MAKOTO/2.0` は Ourin 独自（GitHub 全検索で Ourin 以外に 0 件・自身のコメントも「ninix の挙動参照に留め」）＝採用しない。「§ 原典の追記」節に詳細。

## Problem

ukadoc [トランスレータ](https://ssp.shillest.net/ukadoc/manual/manual_translator.html): ゴースト側（`ghost/master/makoto.dll`・descript [`makoto,ファイル名`](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#makoto_2c_30d5_30a1_30a4_30eb_540d:1)）とシェル側（`shell/<名>/makoto.dll`）の両方に置け、**ゴースト側の翻訳の後にシェル側の翻訳**が行われる。SHIORI の `OnTranslate` と併用時は SHIORI → MAKOTO。用途は今日「シェルによって口調を変える」仕掛け（YAYA as MAKOTO：季節で服装と口調が変わるシェル・ウィンドウにお座りするシェル）。付け外しは [`\![reload,makoto]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cmakoto_5d:1)・[`\![unload,makoto]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bunload_2cmakoto_5d:1)・[`\![load,makoto]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bload_2cmakoto_5d:1)（SSP 2.5.58）。

DLL の口は SHIORI と同じ [DLL 共通仕様](http://ssp.shillest.net/ukadoc/manual/spec_dll.html)（`load(HGLOBAL,long)`／`unload()`／`request(HGLOBAL,long*)`・要求 HGLOBAL は DLL が解放・応答 HGLOBAL はホストが解放・NUL 終端保証なし）。wire は SHIORI/2.x 風のヘッダ形式:

```
EXECUTE MAKOTO/2.0\r\n
Sender: <ベースウェア名>\r\n
String: <台詞>\r\n
Charset: <符号名>\r\n
\r\n
```

応答は `MAKOTO/2.0 200 OK\r\n`＋`String: <変換後>\r\n`＋`Charset: …\r\n`＋`\r\n`。状態は 200／**204 No Content（触らない＝素通し）**／400／500（原典）。

**利用者から見える結果（今日）**: `makoto,` を書いたゴースト／シェルは無言で翻訳されない（descript の未知キーは捨てられる）。加えて areka の wire は UTF-8 のみなので、Shift_JIS の MAKOTO DLL（hanasi 等・大半）は文字化けするか読めない。SHIORI 側の同じ問題（里々／YAYA テンプレートが会話できない）は `charset-canon` が所有する。

## Current State（2026-09-02 実測・着手時に再検証）

- **32bit helper は DLL 1 本専用**: プロキシ保持スロット 1 つ（[main.rs:155](../../../crates/shiori-host32-helper/src/main.rs:155)）・DLL 名は argv 1 枠（`main.rs:491`）・wire の `MsgTag` は Hello/Load/Request/Response/Unload の 5 種で DLL 選別子なし（[lib.rs:44](../../../crates/shiori-host32-ipc/src/lib.rs:44)・凍結）。**関数の型は MAKOTO と同一**（[shiori_proxy.rs:63-70](../../../crates/shiori-host32-helper/src/shiori_proxy.rs:63)）で `ShioriByteProxy` は SHIORI/3.0 の知識を持たない＝そのまま流用可。spawn 契約は `[parent_hwnd, load_dir, dll_name]`（[process_host.rs:233](../../../crates/shiori-host32-host/src/process_host.rs:233)・cwd＝load_dir・`load()` の引数はディレクトリを ANSI で）。
- **charset**: SHIORI/3.0 codec は `Charset::Utf8` のみ（[shiori3.rs:39](../../../crates/shiori-host32-host/src/shiori3.rs:39)・Shift_JIS は「切替シームのみ」と明記）。`client.rs:132/:169` が固定で渡す。`encoding_rs 0.8` はワークスペース依存に既にある（`areka-parsers` が descript の charset で使用）。
- **descript**: `makoto,` は読まれない（[resolve.rs:76](../../../crates/areka-parsers/src/package/resolve.rs:76)・認識キーは name 4 種・`shiori`・`seriko.defaultsurfacedirectoryname` のみ）。shell 側 descript は bindgroup 既定値だけ転記（`resolve.rs:146`）。`ShioriMount` は `#[non_exhaustive]`（[model.rs:198](../../../crates/areka-parsers/src/package/model.rs:198)）。
- **付け外し命令**: `reload`/`load`/`unload` の消費者は 0（reload という概念自体が未実装・`events.rs:109` に「M1 に reload なし」）。`\!` の消費者台帳は 4 行（[consumer_ledger.rs:221](../../../crates/areka/src/emo2_boot/consumer_ledger.rs:221)＝move／bind／set,zorder／reset,zorder・全て表示側）。**ゴースト側（アクター）へ届く cue 消費者は今日 1 つも無い**。
- **接続の所有**: `ShioriConnection`（窓＋helper ライフサイクル・[real.rs:36](../../../crates/areka-kanade/src/shiori/real.rs:36)）が SHIORI 1 本分。
- **テスト資産**: 偽 32bit DLL `shiori-host32-testdll`（cdylib・出力 `shiori.dll`・HGLOBAL 所有権を忠実に再現・`HOST32_TESTDLL_LOAD_FAIL` で load 失敗注入）と `shiori-host32-host/tests/*_e2e.rs`（i686 helper を先ビルド・`HOST32_HELPER_EXE`）。

## Desired Outcome

1. **descript の `makoto,` を読む**: ゴースト側（`ghost/master/descript.txt`）とシェル側（`shell/<名>/descript.txt`）の両方。未指定なら翻訳なし（推測しない）。ukadoc（最新）は単値＝正典。原典 materia の複数値 `makoto,[a.dll,b.dll]`（左から順に鎖・最後の出力が最終）は**旧書式のエイリアスとして受理**（toolkit 仕分け規則「新書式正典・旧書式は alias」）。原典の `ghost/master/alias.txt` の `makoto,別名.dll` 上書きは ukadoc に無い＝要件定義で採否を裁定（推奨: 読むが `info!` で記録）。
2. **MAKOTO/2.0 の wire codec**（純粋関数・要求組立と応答解析）: 上記形式。`Sender: areka`。応答は状態行 `MAKOTO/x.y 200` かつ `String:` ありのとき置換（空文字列も採用）、**204 は素通し**（正典どおり・`debug!`）、それ以外（400／500・`String:` なし・解析不能・タイムアウト）は**元の台詞**（`warn!`）。台詞中の CR/LF は要求に載せない（さくらスクリプトの改行は `\n` タグ＝実改行は持たない・持っていたら除去して `debug!`）。⚠ **HGLOBAL 所有権の食い違い**: DLL 共通仕様（ukadoc）は「要求 HGLOBAL は DLL が解放」だが、原典は「204 のときは**ハンドルを解放せずに**返せ」＝204 では要求ハンドルがホストに残る。要件定義で helper の解放規則を裁定（推奨: 応答 204 のときだけホストが要求ハンドルを解放・テスト DLL で両方の振る舞いを注入し二重解放・リークの決定論テスト）。
3. **ホスティング＝DLL 1 本につき 32bit helper プロセス 1 つ**（ゴースト側・シェル側で最大 2 つ追加）。wire（`MsgTag`）は無改変・helper の DLL 名 argv をそのまま使う・`ShioriByteProxy` 流用。親側に `MakotoConnection`（`ShioriConnection` と同形・窓＋ライフサイクル）。**load 失敗は致命ではない**: `error!` を残して翻訳なしで起動を続ける（SHIORI の load 失敗＝致命とは扱いが違う・COMPAT §8）。終了時は SHIORI と同じ正規の unload 経路。
4. **鎖の順序**: 〔前半 spec の OnTranslate〕→ ゴースト側 MAKOTO → シェル側 MAKOTO。鎖は起動時に組む（シェル切替は未実装＝切替時の付け替えは将来 spec）。
5. **任意 charset＝`charset-canon` の符号化器と交渉規則を再利用**（上流・本 spec は新しい符号化器を作らない）: 要求は選定 charset で符号化し `Charset:` ヘッダに宣言・応答は応答の `Charset:` を優先して復号（無ければ要求側を継承）・以後の要求は応答 charset に追随。初期 charset は charset-canon の規則（`shiori.forceencoding` ＞ `shiori.encoding` ＞ 既定 Shift_JIS）を MAKOTO にも当てる（MAKOTO 専用の descript キーは ukadoc に無い＝COMPAT §8 に登記）。**`loadu()` 優先**（SSP 2.6.92 以降の UTF-8 パス・ukadoc `spec_dll`）＝DLL が export していれば使い、無ければ `load()`（OEM コードページ）へ落とす。この規則は helper 側（`shiori_proxy.rs`）の変更であり SHIORI DLL にも効く＝charset-canon（helper 非接触と宣言）が持たない唯一の charset 項目として本 spec が所有する。charset-canon 未着手のまま本 spec が先行する場合は、符号化器の型（`&'static encoding_rs::Encoding` の newtype＝charset-canon 案 ⒝）を本 spec が先に置き charset-canon が rebase する（順序は棚卸で裁定）。
6. **付け外し命令**: `\![unload,makoto]`（鎖の全 DLL を unload・以後は素通し）／`\![load,makoto]`（unload 後に再ロード・ロード済みなら冪等）／`\![reload,makoto]`（unload→load）。消費者台帳に 3 行（選別子 `makoto`）。**cue 消費者からゴースト側アクターへ届く最初の配線**＝UI スレッドを塞がないメッセージ経路で設計する。命令は鎖の両側（ゴースト側・シェル側）に効く（SSP は明記なし＝COMPAT §8）。
7. **決定論テスト**: ⑴ codec の往復（要求 bytes golden・応答行列 200／200 空／非 200／`String:` なし／壊れた charset）⑵ MAKOTO wire の charset（Shift_JIS 要求の golden bytes・Shift_JIS 応答の復号・応答 `Charset` 省略時の継承・`loadu` 有無の分岐）⑶ 新設 `shiori-host32-makoto-testdll`（i686 cdylib・出力 `makoto.dll`・決定論の変換〔例: 末尾に固定語を付ける〕・応答 charset を Shift_JIS に切り替える env・load 失敗注入・非 200 注入）で helper 経由の e2e ⑷ descript 読取（ゴースト／シェル／不在／複数値）⑸ 鎖の順序（ゴースト側→シェル側の変異＝入れ替えると赤）⑹ 命令 3 種の状態遷移（Loaded／Unloaded・冪等）⑺ load 失敗時に起動が続く。
8. **実機サインオフ**（有界 auto-exit＋`RUST_LOG` grep）: ⑴ **YAYA as MAKOTO UTF-8 改良版**（nightwork 配布）＋語尾変換の小辞書を emo2 の複製ゴーストへ `makoto,` で装着し、台詞に変換が出ること・`\![reload,makoto]` の往復 ⑵ 同じ DLL をシェル側に置いて鎖の順序 ⑶ Shift_JIS 応答を返すテスト DLL で文字化けなし。Shift_JIS の**実 SHIORI**（里々テンプレート）は `charset-canon` の実機項目。
9. `cargo test --workspace` 緑（i686 先ビルド）・1,000 行未満・例外表非接触。

## Approach

**別プロセス案（推奨）**: MAKOTO DLL 1 本につき既存 helper をもう 1 プロセス起こす。wire・helper は無改変（DLL 名は既に汎用 argv）、親側の接続型を SHIORI と同形でもう 1 つ持つ。解決: 32bit 同居・所有権・ライフサイクル・死活監視が SHIORI と同じ機構で得られる。未解決: プロセスが最大 3 つになる（起動コストは helper 1 本あたり数十 ms・M1 の計測結果を再計測）。

**多重 DLL 案（不採用）**: helper に DLL 選別子を足して 1 プロセスで複数 DLL を飼う。解決: プロセス数 1。未解決: 凍結 wire（`MsgTag`）の改変・helper の単一スロット設計の全面改修・SHIORI と MAKOTO の障害が同居する（片方のクラッシュで会話が止まる）。SAORI の「同 32bit プロセス同居」（COMPAT §5:87）は SHIORI が自分で `LoadLibrary` する話であり、ベースウェアが呼ぶ MAKOTO には当てはまらない。

x64 in-proc（COM `IShiori`）の MAKOTO 版は作らない（そのような DLL は存在しない・MAKOTO は常に 32bit helper 経由）。arm64 ホストでも helper は i686（SHIORI と同じ）。

## Scope

- **In**: descript `makoto,`（ghost／shell）・`MakotoMount`・MAKOTO/2.0 codec（charset-canon の符号化器を呼ぶ）・helper の `loadu` 優先・`MakotoConnection`（第 2/3 helper）・鎖と順序・`Translator` フックへの差し込み・命令 3 種と消費者台帳 3 行・`shiori-host32-makoto-testdll` 新設・e2e・実機サインオフ・COMPAT §5（MAKOTO ホスティング）／§8（裁量 4 件＝複数値拒否・load 失敗は非致命・命令は鎖全体・charset 追随規則）・`boot_config.rs`（helper exe の再利用）。
- **Out**: `OnTranslate`・翻訳の継ぎ目・展開順序（前半 spec）・**SHIORI/3 wire の charset 交渉・`shiori.encoding`／`forceencoding` の解析・surfaces.txt の decode**（`charset-canon`）・シェル切替時の鎖の付け替え（切替自体が未実装）・SAORI／PLUGIN／HEADLINE・`\![reload,shiori]`（本 spec の reload 機構で書けるようになるが起票は別・追跡登記）・MAKOTO/1.0（`execute`・生台詞）・符号の自動判別。

## Boundary Candidates

- ⓐ **helper の `loadu` 優先**（XS・独立 PR 可・SHIORI DLL にも効く・挙動不変を e2e で担保）。
- ⓑ **第 2 helper のライフサイクル＋MAKOTO codec＋descript＋鎖**（L・本体・charset-canon の後）。
- ⓒ **付け外し命令 3 種**（M・ゴースト側へ届く cue 消費者の初出＝配線の型を決める）。
- 分割の裁定は開発者（推奨: ⓑⓒ を 1 PR・ⓐ は任意で先行）。

## Out of Boundary

- 翻訳の継ぎ目そのもの（前半 spec が所有・本 spec はフックを埋めるだけ）。
- helper の wire（`MsgTag`）改変・helper の多重 DLL 化。
- 里々／YAYA を areka がソースビルドすること（COMPAT §5:87・bitness 連鎖）。

## Upstream / Downstream

- **Upstream**: `translate-pipeline`（`Translator` フックと出所語彙）・**`charset-canon`**（符号化器の型・交渉規則・descript `shiori.encoding`／`forceencoding`）・完了 spec `host32-ipc`／`host32-lifecycle`／`host32-request`／`host32-shiori-load`（helper・wire・ライフサイクル）・`parser-foundation`（encoding_rs・descript charset）・`package-mount`（`MountModel`）。
- **Downstream**: `ukadoc-coverage-roadmap`（MAKOTO 項目 5 件＋charset の段階 A 反映）・将来のシェル切替 spec（鎖の付け替え）・`\![reload,shiori]`（reload 機構の再利用・追跡登記）・Shift_JIS 実ゴースト（里々／YAYA テンプレート）の起動検証 spec。

## Existing Spec Touchpoints

- **Extends**: `package-mount`（`MountModel` に `makoto`）・完了 spec `host32-shiori-load`（helper の `load` 解決に `loadu` を足す）。完了 spec の文書は改変せず、縮退表の更新は COMPAT §8 で行う。
- **Adjacent**: **`charset-canon`**（`shiori3.rs`／`client.rs`／`resolve.rs` を持つ＝本 spec は符号化器を呼ぶ側・**charset-canon を先に着地**・`resolve.rs` は両方が触る＝後着 rebase）／`property-query-channels` ⑵ IPC 片（`shiori-host32-*`・`areka/shiori_host.rs`＝**共有あり**）と ⑴⑶（`consumer_ledger.rs`＝**共有あり**）→ **直列**（先着が rebase 源）／`ukadoc-survey-shiori`（kanade/schedule の doc 1 行＝後着 rebase）／`ukadoc-survey-toolkit`（`Cargo.lock` の機械マージのみ）／`emo2-conformance-e2e`（tests のみ・`loadu` 分岐は SHIORI 経路に触る＝e2e 全緑で不変を担保）。
- **編集集合（2026-09-02 実測）**: `crates/shiori-host32-host/src/`＋新規 `makoto_client.rs`（MAKOTO codec・charset-canon の符号化器を呼ぶ）・`crates/shiori-host32-helper/src/shiori_proxy.rs`（`loadu` 優先のみ）・新規 `crates/shiori-host32-makoto-testdll/`・`crates/areka-parsers/src/package/{model,resolve}.rs`・`crates/areka-kanade/src/shiori/real.rs`＋新規 `crates/areka-kanade/src/makoto/{mod,chain}.rs`・`crates/areka-ghost/src/{runtime.rs, shiori_wiring.rs}`＋新規 `makoto_wiring.rs`・`crates/areka/src/{boot_config.rs, emo2_boot/consumer_ledger.rs}`・`Cargo.lock`・`doc/COMPAT_ARCHITECTURE.md` §5/§8。`shiori3.rs`／`client.rs`（charset-canon 所有）と `shiori-host32-ipc` の wire は非接触。

## Constraints

- 32bit 可搬性の適用範囲は host-32 系のみ（roadmap 制約）・helper は i686・テストは x64 の偽境界＋i686 e2e（記憶 prefer-x64-fake-boundary-tests-not-x86）。
- ログ無し失敗経路の禁止（load 失敗・非 200・charset 未知は全て `warn!`/`error!`）。
- 本番 env は `AREKA_` 冠（テスト DLL の注入 env は `HOST32_` 系の既存慣行に従う）。
- 1 ファイル 1,000 行未満・兄弟テスト・例外表非接触。
- 実機の定石: 絶対パス起動・i686 先ビルド・`AREKA_APP_SMOKE_EXIT_MS`・`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。
- 正典の根拠: ukadoc [トランスレータ](https://ssp.shillest.net/ukadoc/manual/manual_translator.html)・[descript makoto](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#makoto_2c_30d5_30a1_30a4_30eb_540d:1)・[DLL 共通仕様](http://ssp.shillest.net/ukadoc/manual/spec_dll.html)・`\![load|unload|reload,makoto]`・[ゴースト](https://ssp.shillest.net/ukadoc/manual/manual_ghost.html)／[シェル](https://ssp.shillest.net/ukadoc/manual/manual_shell.html)の配置図・YAYA wiki「YAYA as MAKOTO」。wire 形式の原典: materia `makoto.html`（Wayback・下記）。傍証: dnproxy（`IMakoto20 : ILoad, IUnload, IRequest`）・ooyashima 用語集。

## 原典の追記（2026-09-02・調査サブエージェントの裏取り結果）

- **要求行は `EXECUTE MAKOTO/2.0`**（原典の記述例: `EXECUTE MAKOTO/2.0` / `Sender: embryo` / `String: \0\s0これはペンです。\e` / `Charset: Shift_JIS`、応答 `MAKOTO/2.0 200 OK` / `String: …` / `Charset: Shift_JIS`）。export は `request(HGLOBAL, long*)`。framing（CR+LF・1 行目＝命令＋版・`名: 値`・空行で終端・NUL 終端保証なし）は ukadoc `spec_dll` と同じ。
- **MAKOTO/1.0 は別物**: export が `execute(HGLOBAL, long*)` で、ヘッダなしの生の台詞を受けて生の台詞を返す。本 spec は 2.0 のみ（DLL が `request` を持たず `execute` だけ持つときは `warn!`＋翻訳なし＝世代の記録のみ・1.0 の実装は起票しない）。
- **`load()` は DLL のディレクトリを受ける**（1.0／2.0 とも・戻り値は materia が見ていない）。SSP 2.6.92（2025-01-16）以降は `loadu()`（UTF-8 パス）を先に探す。
- **状態**: 200（`String` を使う）／204（触らない＝素通し・**要求ハンドルを解放せずに返す**＝所有権の食い違い・上記 2 項）／400／500。**非 200 や `String` 欠落時の SSP の振る舞いは文書化されていない**＝素通し（204 と同義）を areka の裁量として COMPAT §8 に登記。
- **鎖と別名**: 原典は `makoto,[a.dll,b.dll]` の順次連鎖と `alias.txt` の `makoto,` 上書きを定義（ukadoc には無い＝旧書式エイリアス扱い・上記 1 項）。`sstp.alwaystranslate`（SSTP でも翻訳）は SSTP 未実装ゆえ範囲外。
- **YAYA as MAKOTO UTF-8 改良版**: 配布中（`YAYA_as_MAKOTO_UTF-8(20250116).7z`・yaya.dll Tc571-9・辞書 UTF-8・`makoto_systemfunc.dic`）。**呼び出される YAYA 関数名は未確認**（同梱 readme／`.dic` にある＝要件定義で開発者が展開して確認・サブエージェントは配布物のダウンロードを行わない）。
- 到達不能: `navy.nm.land.to/post/makoto.html`（MAKOTO 総合解説・2026-03 にサービス終了・Wayback は 429）・`usada.sakura.vg/contents/specification{,2}.html`（Wayback 429・再試行の価値あり）。
