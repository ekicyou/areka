# Brief: areka-P0-shiori-loadu

> 起票: 2026-09-20（`/kiro-discovery`・Path C・開発者指示「`load` と `loadu` の両方がある場合、`loadu` を優先して呼び出すこと」）。
> **種別**: 正典（ukadoc [DLL 共通仕様](https://ssp.shillest.net/ukadoc/manual/spec_dll.html) の 4 つ目の入口 `loadu`）。**規模 XS〜S**。
> **出自**: `areka-P0-makoto-dll-host` の brief が切れ端 ⓐ「helper の `loadu` 優先（XS・独立 PR 可・SHIORI DLL にも効く）」として抱えていたものを、**SHIORI を主語にした独立 spec** として切り出す。`loadu` は MAKOTO の話ではなく DLL 共通仕様＝SHIORI の入口そのものであり、makoto を待つ理由が無い。makoto 側は本 spec の成果（`ShioriByteProxy`）をそのまま流用するので、移管で失うものは無い。

## Problem

ukadoc `spec_dll`「loadu関数」（SSP 2.6.92・2025-01-16 で追加）:

- 署名は `BOOL __cdecl loadu(HGLOBAL h, long len)`。`load` との違いは **1 点だけ**＝第 1 引数のディレクトリパスが **UTF-8**（`load` は既定の OEM コードページ・日本語環境では CP932）。
- 「本関数はベースウェアによって優先的に使用される」。
- `load` の節:「loadu関数が実装されていない場合のフォールバック先として使用される」「loadとloaduの両方を実装し、loaduで初期化済の時にloadも呼ばれた場合は無視するのが望ましい」。

areka の 32bit 助け手は `load`・`unload`・`request` の 3 つしか名前で引かず、`loadu` を持つ DLL にも常に `load` を呼ぶ。

**利用者から見える結果（今日）**: ゴーストの置き場所に、Windows の既定コードページで表せない字（日本語環境なら韓国語・絵文字・一部の漢字など。利用者名がそういう字だと `C:\Users\<名>\…` の下は全滅）が含まれると、`load` に渡るパスが**黙って `?` に化ける**（`WideCharToMultiByte(CP_ACP, 0, …)` は表せない字を既定文字へ置き換え、失敗を返さない＝[shiori_proxy.rs の `ansi_encode`](../../../crates/shiori-host32-helper/src/shiori_proxy.rs)）。SHIORI は辞書を見つけられず、ログには原因が 1 行も出ない。`loadu` を持つ DLL ならこの事故は起こり得ないのに、areka は使っていない。

## Current State（2026-09-20 実測・着手時に再検証）

- **入口の解決は 1 か所だけ**: `ShioriByteProxy::load` の中の `resolve` クロージャ（`GetProcAddress` で `load`／`unload`／`request` の 3 つを引き、1 つでも欠ければ `ProxyError::EntryNotFound`）。`loadu` の語はソースに 0 件（`crates/` 全域の grep で 0・文書と台帳にのみ現れる）。flat-C の DLL を読む製品コードは他に無い（x64 の同居経路は COM `IShiori`＝`shiori_factory` であり `load`／`loadu` を持たない。`crates/pilot/examples/shiori-host-32/` は凍結した先進坑＝触らない）。
- **パスの符号化**: `ansi_encode`（CP_ACP・フラグ 0・`lpUsedDefaultChar` は `None`）＝**表せない字の検出をしていない**。helper に `activeCodePage` のマニフェストは無い（`crates/` に 0 件）＝CP_ACP は OS の既定のまま。
- **パスの出どころ**: helper の argv 第 2 引数（`load_dir_arg_env() -> Option<String>`）＝helper の中では常に正しい Unicode。UTF-8 への変換は `str::as_bytes` で足り、新しい Win32 呼び出しも依存も要らない。
- **戻り値の型**: `LoadFn`／`UnloadFn` は Rust の `bool`（1 バイト）で受けている（完了 spec `host32-shiori-load` の research §9 が pasta の実体に合わせて固定）。正典は `BOOL`（4 バイト）。cdecl の戻りは EAX で、今は下位 1 バイトだけを読む＝C 製の DLL が `TRUE`(1) を返す限り動くが、**下位バイトが 0／1 以外の値だと Rust の `bool` としては未定義動作**。
- **手元の検体が両方の枝を持つ**（`vendors/sample_ghost/*.nar` の DLL を展開して入口名を走査）:

  | 検体 | DLL | `loadu` | `load` |
  |---|---|---|---|
  | `konnoyayame.nar` | `yaya.dll`（909,312 B） | **あり** | あり |
  | `R_POST_and_KOMAINU.nar` | `satori.dll`（720,896 B） | なし | あり |
  | `emo2.nar` | `pasta.dll` | なし | あり |

  走査は入口名の NUL 区切り一致という粗い方法＝着手時に正規の道具（`dumpbin /exports` 等）で取り直す。
- **テスト資産**: 偽 32bit DLL `shiori-host32-testdll`（出力 `shiori.dll`・`load`／`unload`／`request` のみ・`HOST32_TESTDLL_LOAD_FAIL`・`HOST32_TESTDLL_UNLOAD_MARKER`）。helper 単体テストと `shiori-host32-host/tests/*_e2e.rs` がこれを読む（i686 先ビルド）。**`loadu` を持つ偽 DLL は無い**。
- **観測経路**: helper は `eprintln!("[helper] …")`。親は helper の標準出力系を付け替えていない（`shiori-host32-host/src` に `Stdio::` 0 件）＝親の stderr にそのまま混ざる。
- **台帳**: `doc/ukadoc-coverage/ledger/shiori.toml` の `[entry."ukadoc:spec_dll"]`（`status = "degraded"`）が「loadu は引かない」と 3 か所に書く。同じ文が台帳冒頭の注釈と、生成物 `briefing-shiori.md` にも写っている。

## Desired Outcome

1. **`loadu` を先に引き、在ればそれだけを呼ぶ**。パスは `load` に渡すのと同じ文字列を UTF-8 にしたもの（末尾の区切りの有無なども含めて同一）。`loadu` を呼んだら `load` は呼ばない。
2. **`loadu` が無ければ今日と同じく `load`**（CP_ACP）。既存の検体（里々・pasta）と既存のテストは挙動不変。
3. **`load` へ落ちたとき、パスが既定コードページで表せなければ `warn` を 1 行残す**（今日は黙って化ける＝ログ無し失敗経路の禁止に反している）。渡すこと自体はやめない。
4. **どちらの入口を使ったかを 1 行記録する**（helper の既存の `[helper]` 行）。実機の確認はこの行を grep する。
5. 戻り値は `u8` で受けて `!= 0` で判定する（`loadu`・`load`・`unload` とも）。pasta（下位 1 バイトだけを書く）と C 製 DLL（`BOOL`）の両方で正しく、未定義動作の余地が消える。同じ型定義の行を触るので本 spec で直す。
6. 決定論テスト（下記）・`cargo test --workspace` 緑（i686 先ビルド）・1,000 行未満。
7. 台帳 `ukadoc:spec_dll` の記述を実装に合わせる（`status` は `degraded` のまま＝SAORI・MAKOTO・PLUGIN が残る）。派生文書は `ukadoc-survey` の生成器で撮り直す（手で直さない）。

### 要件定義で裁定する点（推奨つき・いずれも正典が沈黙＝`doc/COMPAT_ARCHITECTURE.md` §8 に登記）

| # | 分かれ目 | 利用者から見える差 | 推奨 |
|---|---|---|---|
| 1 | `loadu` だけ在って `load` が無い DLL | 拒むと「新しい作法だけで書いた SHIORI」が起動しない | **受け入れる**（`loadu`／`load` のどちらか 1 つが在ればよい。両方無ければ今日と同じ `EntryNotFound`） |
| 2 | `loadu` が偽を返した | `load` へ落ちると同じ DLL を 2 度初期化する | **落ちない**＝`LoadReturnedFalse`。正典の落ちる条件は「実装されていない場合」だけ |
| 3 | `load` へ落ち、パスが表せない | 起動を止めるか、化けたパスを渡して DLL に任せるか | **`warn` して渡す**（DLL が自分で失敗を返せば既存の `LoadReturnedFalse` が出る） |

## Approach

**`resolve` に 1 本足して、呼ぶ入口を選ぶ純関数を 1 つ置く（推奨・採用）**: `GetProcAddress(module, "loadu")` を任意で引き、「`loadu` の有無 × `load` の有無」の 4 通りから呼ぶ入口を決める純関数（判断はここだけ）を経て、`encode_alloc_and_load` に「UTF-8 か CP_ACP か」を渡す。凍結 wire（`MsgTag`）・親側・ack 1 バイトの契約は無改変。

不採用:
- **親側から入口を指定する案**（argv や wire で「loadu を使え」と渡す）: 親は DLL の中身を知らない。凍結 wire を触る理由にならない。
- **helper に `activeCodePage=UTF-8` のマニフェストを付けて `load` だけで済ます案**: `load` を UTF-8 で呼ぶことになり、CP932 を期待する既存の SHIORI（里々ほか大半）を全部壊す。

新しい依存・新しい技術は無い（UTF-8 化は標準ライブラリ・表せない字の検出は既に使っている `WideCharToMultiByte` の引数 1 つ）＝実現性の別途調査は不要と判断。

## Scope

- **In**: `crates/shiori-host32-helper/src/shiori_proxy.rs`（`loadu` の解決・入口の選択・UTF-8 符号化・表せない字の検出と `warn`・戻り値 `u8`・冒頭の「確立シーケンス」の説明）・`main.rs` の記録 1 行・`loadu` を持つ偽 DLL（2 本目の fixture）・決定論テスト・実機確認・`doc/COMPAT_ARCHITECTURE.md` §8（裁定 3 件）・台帳 `ukadoc:spec_dll` と派生文書の撮り直し・i686 先ビルド手順に fixture を 1 つ足す（手順を書いている全ての場所）。
- **Out**: MAKOTO・SAORI・PLUGIN の読み込み（`makoto-dll-host` ほか）／`request` の文字コード（完了 `charset-canon`）／`unload`・`request` の振る舞い／凍結 wire と親側（`shiori-host32-host`・`areka-kanade`）／x64 同居経路（COM）／`crates/pilot/` の先進坑／pasta に `loadu` を足すこと（上流の話＝提案はできるが本 spec は持たない）／パスの末尾区切りの作法を SSP に合わせるかどうか（今日の `load` と同じ文字列を渡す＝変えない）。

## Boundary Candidates

- ⓐ **入口の選択**（純関数・4 通りの判断表）と **UTF-8 符号化**。
- ⓑ **`load` へ落ちたときの表せない字の検出**（今日から在る黙った失敗の是正・ⓐ と独立に検証できる）。
- ⓒ **2 本目の偽 DLL**（規模の大半はここ。`loadu`＋`load` の両方を持ち、どちらが呼ばれたかと受け取ったバイト列を env 指定のファイルへ書く。偽応答の注入は既存 fixture と同じ env の作法）。
- 1 PR で足りる。分けるなら ⓑ だけが単独で先行できる。

## Out of Boundary

- 完了 spec（`host32-shiori-load` ほか）の文書は改変しない。要件の上書きは本 spec の requirements と COMPAT §8 に書く。
- 既存 fixture `shiori-host32-testdll`（`load` のみ）は**無改変**で残す＝「`loadu` が無い DLL は今日のまま」の証拠が既存テスト全緑で得られる。

## テストの方針（決定論・到達する経路を踏む）

1. 入口の選択の判断表 4 行（両方在る→`loadu`／`loadu` のみ→`loadu`／`load` のみ→`load`／両方無い→エラー）。
2. UTF-8 符号化の固定バイト列（CP932 に在る字と無い字の両方を含むパス）。表せない字の検出（CP932 前提にせず「どのコードページにも無い字」で判定＝絵文字など）。
3. i686 の helper 単体テスト: 2 本目の fixture を実際に読み、**`loadu` が呼ばれ `load` は呼ばれない**こと・受け取ったバイト列が UTF-8 であること。優先順を逆にすると赤になること（記録が `load` に変わる）。
4. `loadu` が偽を返す注入→`LoadReturnedFalse` かつ `load` の記録が無い（裁定 2）。
5. 既存の helper 単体テスト・e2e は無改変で緑（`load` の枝の不変）。
6. **実機**（有界 auto-exit＋`[helper]` 行の grep）: `konnoyayame`（YAYA）で `loadu`、`R_POST_and_KOMAINU`（里々）と `emo2`（pasta）で `load`。加えて、既定コードページに無い字を含むフォルダの下へ `konnoyayame` を置いて喋ること（**今日どう壊れるかは未実測**＝要件定義の前に実機で赤を確かめる）、同じ場所の里々で `warn` が出ること。

## Upstream / Downstream

- **Upstream**: 完了 `host32-shiori-load`（`ShioriByteProxy`）・`host32-request`・`host32-window-thread-pump`・`nar-install`（検体は `.nar`＝共有ヘルパ経由）・`shell-implicit-surface`（里々／YAYA の検体が実機で動く）。未完の前提は無い。
- **Downstream**: `makoto-dll-host`（`ShioriByteProxy` を流用＝`loadu` 優先を無償で得る。brief の ⓐ は本 spec へ移管済み）・`ukadoc-coverage` の台帳。

## Existing Spec Touchpoints

- **Extends**: 完了 `host32-shiori-load`（入口 3 つ→`loadu` を加えた 4 つ・戻り値の受け方）。
- **Adjacent**: `makoto-dll-host`（`shiori_proxy.rs` を触る予定が本 spec へ移った＝残るのは流用だけ）／`property-ipc-transport`・`property-query-channels` ⑵（`shiori-host32-*` を触るが `shiori_proxy.rs` の入口解決には触らない見込み＝着手時に確認）／`coverage-roadmap-refresh`（台帳の手書きの数。本 spec は `spec_dll` の 1 項目の記述だけを直す）。

## Constraints

- 32bit 可搬性の適用範囲は host-32 系のみ・helper は i686。常時テストは純関数を x64 で、DLL を読むものは i686（既存の `#[cfg_attr(ignore)]` の作法）。
- unsafe は `shiori_proxy.rs` に集約し、各ブロックに Safety 根拠（steering）。
- ログ無し失敗経路の禁止。本番 env は `AREKA_` 冠・fixture の注入 env は `HOST32_TESTDLL_` の既存慣行。
- 1 ファイル 1,000 行未満（`shiori_proxy.rs` は今 500 行強＝テストが膨らむなら兄弟ファイルへ）。
- 正典の根拠: ukadoc [DLL 共通仕様](https://ssp.shillest.net/ukadoc/manual/spec_dll.html) の「loadu関数」「load関数」の節。SSP の実測は取らない（開発者方針）。
