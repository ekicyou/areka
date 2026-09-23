# Requirements Document

> 本文の実測は **2026-09-23・本ブランチ**のもの（brief の 2026-09-20 実測を着手時に再検証した）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> brief の「裁定済みの 3 点」（2026-09-20 開発者承認）は本文にそのまま要件として書き、要件ディスカッションで再び議題にしない。

## Project Description (Input)

`load` と `loadu` の両方を持つ SHIORI DLL に対して、areka の 32bit の助け手は `loadu` を優先して呼び出す（開発者指示・2026-09-20）。ukadoc「DLL 共通仕様」が定める 4 つ目の入口 `loadu`（置き場所のパスを UTF-8 で受け取る）を areka が使っていないため、既定コードページで表せない字を含むフォルダに置かれたゴーストは、`load` に渡るパスが黙って `?` に化け、ログ 0 行で辞書を見失う。本仕様は (1) `loadu` を先に引いて在ればそれだけを呼ぶ、(2) `loadu` が無ければ今日と同じ `load`、(3) `load` へ落ちてパスが表せないときに警告を 1 行残す、(4) どの入口を使ったかを 1 行記録する、(5) 戻り値を 1 バイト整数で受けて 0 か否かで判定する、を実装し、`loadu` を持つ 2 本目の偽 DLL と決定論テスト・実機確認・台帳の追随までを含む。

## Introduction

### 誰が困っているか

areka の利用者のうち、**ゴーストを Windows の既定コードページで表せない字を含むフォルダに置いている人**（日本語環境なら韓国語・絵文字・一部の漢字など。利用者名にそういう字が入っていると `C:\Users\<名>\…` の下は全滅）。加えて、`loadu` だけを実装した新しい SHIORI を使いたいゴースト作者。

### いま何が起きているか（2026-09-23 実測・brief の記述を再検証）

- **入口の解決は 1 か所だけ。** `crates/shiori-host32-helper/src/shiori_proxy.rs` の `ShioriByteProxy::load` の中の `resolve` クロージャが `GetProcAddress` で `load`・`unload`・`request` の 3 つを引き、1 つでも欠ければ `ProxyError::EntryNotFound`。`loadu` の語は `crates/` 以下のソースに **0 件**（文書と台帳にのみ現れる）。brief と一致。
- **パスの符号化は表せない字を検出していない。** 同ファイルの `fn ansi_encode` は `WideCharToMultiByte(CP_ACP, 0, …)` を長さ問い合わせと変換の 2 回呼び、`lpUsedDefaultChar` に当たる最後の引数は両方 `None`。表せない字は既定文字（`?`）へ置き換えられ、関数は成功を返す。helper に `activeCodePage` のマニフェストは無い（`crates/` に 0 件）。brief と一致。
- **パスの出どころは helper の引数。** `crates/shiori-host32-helper/src/main.rs` の `fn load_dir_arg_env`（argv 第 2 引数、無ければ env `HOST32_LOAD_DIR`）→ `PathBuf` → `HelperShared::load_dir`。`TriggerLoad` の枝で `ShioriByteProxy::load(&dll_path, &s.load_dir)` に **そのまま**渡る。末尾の区切りの有無などは呼び出し側が渡した文字列のまま。brief と一致。
- **戻り値は Rust の `bool` で受けている。** `type LoadFn = unsafe extern "C" fn(HGLOBAL, usize) -> bool`・`type UnloadFn = unsafe extern "C" fn() -> bool`。正典は `BOOL`（4 バイト）。cdecl の戻りは EAX で、今は下位 1 バイトだけを読む。下位バイトが 0／1 以外だと Rust の `bool` としては未定義動作。brief と一致。
- **確立の記録は失敗時だけ。** `main.rs` の `TriggerLoad` の枝は成功時に `[helper]` 行を出さず、失敗時のみ `eprintln!("[helper] LOAD 失敗（観測・ack[0]）: {e:?}")`。どの入口を呼んだかは今は書けない（入口が 1 つしか無い）。
- **観測経路は helper の標準エラー出力。** helper は `eprintln!("[helper] …")` で書き、親（`crates/shiori-host32-host/src`）は `Stdio::` を 1 件も使っていない＝親の stderr にそのまま混ざる。brief と一致。
- **テスト資産。** 偽 32bit DLL `crates/shiori-host32-testdll`（出力 `shiori.dll`）は `load`・`unload`・`request` の 3 つだけを公開し、`HOST32_TESTDLL_LOAD_FAIL=1` で `load` が偽を返し、`HOST32_TESTDLL_UNLOAD_MARKER` で `unload` の実呼出をファイルに残す。`loadu` を持つ偽 DLL は **無い**。brief と一致。
- **台帳。** `doc/ukadoc-coverage/ledger/shiori.toml` の `[entry."ukadoc:spec_dll"]`（`status = "degraded"`）の `note` が「loadu は引かない」と書き、同じ文が台帳冒頭の注釈（群 14c）と生成物 `doc/ukadoc-coverage/briefing-shiori.md` にも写っている。brief と一致。
- **brief からのずれ 1 件。** brief は裁定 3 件を「`doc/COMPAT_ARCHITECTURE.md` §8 に登記」と書くが、§8 の表に `loadu`・`spec_dll` を含む行は **0 件**（2026-09-23 grep）。登記は**まだ行われていない**＝本仕様の作業として要件 8 に含める。
- **未再検証の項目。** 検体 3 体（`konnoyayame`＝YAYA・`R_POST_and_KOMAINU`＝里々・`emo2`＝pasta）の入口の有無は brief の粗い走査（入口名の NUL 区切り一致）のままで、正規の道具（`dumpbin /exports` 等）では取り直していない（要件 7.4）。「今日どう壊れるか」の実機の赤も未実測（要件 7.3）。

### 正典（ukadoc）の位置づけ

[DLL 共通仕様](https://ssp.shillest.net/ukadoc/manual/spec_dll.html)「ライフサイクル関数」の節（逐語引用）:

| 節 | 逐語引用 |
|---|---|
| loadu関数 | `extern "C" __declspec(dllexport) BOOL __cdecl loadu(HGLOBAL h, long len);` |
| loadu関数 | 「第1引数のグローバルメモリにはモジュールのディレクトリパスがUTF-8エンコーディングで格納される。」 |
| loadu関数 | 「SSP 2.6.92 (2025/1/16) より実装された。それより前はloadのみの実装である。」 |
| loadu関数 | 「文字列データが不要な場合でも、渡されたグローバルメモリは必ず解放する」「初期化に成功した場合は TRUE を、失敗した場合は FALSE を返却する」 |
| loadu関数 | 「本関数はベースウェアによって優先的に使用される」 |
| load関数 | 「loadu関数の従来版。第1引数のモジュールディレクトリパスがデフォルトのOEM codepage（日本語環境ではCP932）でエンコードされている以外は、loadu関数と同じ仕様。」 |
| load関数 | 「loadu関数が実装されていない場合のフォールバック先として使用される」 |
| load関数 | 「互換性確保のため、loadとloaduの両方を実装し、loaduで初期化済の時にloadも呼ばれた場合は無視するのが望ましい」 |
| unload関数 | 「戻り値は現在は使用されていないが、将来の拡張性を考慮して TRUE を返却することが望ましい。」 |

正典が **沈黙**している 3 点は開発者が 2026-09-20 に裁定済み（`loadu` だけの DLL を受け入れる／`loadu` が偽でも `load` へ落ちない／`load` へ落ちて表せない字があれば警告して渡す）。本仕様はその裁定をそのまま要件にする。**SSP の実測は取らない**（開発者方針）。

### 何を変えるか

助け手が SHIORI DLL を確立するとき、`loadu` を先に引き、在れば **`loadu` だけ**を UTF-8 のパスで呼ぶ。無ければ今日と同じ `load`（既定コードページ）を呼ぶが、パスに表せない字があれば **警告を 1 行**残す。どちらの入口を呼んだかを **1 行記録**する。戻り値は 1 バイト整数で受け、0 か否かで判定する。`unload`・`request` の振る舞い、親との受け渡し、x64 の同居経路は **変えない**。

## Boundary Context

- **In scope**（利用者・運用者・ゴースト作者から見える範囲）:
  - 確立時の入口の選択（`loadu` 優先・4 通りの判断）と、`loadu` へ渡すパスの UTF-8 化。
  - `load` へ落ちたときの、既定コードページで表せない字の検出と警告（今日から在る黙った失敗の是正）。
  - どちらの入口を呼んだかの記録 1 行（実機確認はこの行を grep する）。
  - `loadu`・`load`・`unload` の戻り値の受け方（1 バイト整数・0 か否か）。
  - `loadu` を持つ 2 本目の偽 32bit DLL と、それを読む決定論テスト。i686 先ビルド手順に fixture を 1 つ足すこと（手順を書いている全ての場所）。
  - 実機確認（検体 3 体＋既定コードページに無い字を含むフォルダ）。
  - `doc/COMPAT_ARCHITECTURE.md` §8 への裁定 3 件の登記、台帳 `ukadoc:spec_dll` の記述の追随と派生文書の生成器による撮り直し、`shiori_proxy.rs` 冒頭の「確立シーケンス」の説明の更新。
- **Out of scope**（**変更 0**。以下は本仕様で 1 行も変えない）:
  - MAKOTO・SAORI・PLUGIN の DLL の読み込み（`areka-P0-makoto-dll-host` ほか）。台帳 `ukadoc:spec_dll` の `status` は `degraded` の**まま**（SAORI・MAKOTO・PLUGIN が残るため）。
  - `request` の文字コード（完了 `areka-P0-charset-canon`）。
  - `unload`・`request` の振る舞い（呼ぶ時機・引数・所有権・Drop での courtesy `unload`・結果の無視）。変わるのは `unload` の戻り値を受ける型だけ。
  - 親側（`crates/shiori-host32-host`・`crates/areka-kanade`）と凍結した受け渡し（`MsgTag`・load-ack の 1 バイト・`LOAD` にペイロードを持たせないこと）。親は「どの入口で確立したか」を**知らない**まま。
  - 既確立の再 `LOAD` の冪等応答（ack `[1]`・入口の再呼出なし）。
  - x64 の同居経路（COM `IShiori`＝`shiori_factory`）。`load`／`loadu` を持たず、対象外。
  - `crates/pilot/` の先進坑（凍結・無改変）。
  - pasta に `loadu` を足すこと（上流の話）。
  - パスの末尾区切りの作法を SSP に合わせるかどうか（今日 `load` に渡している文字列を **そのまま**使う＝変えない）。
  - 既存 fixture `crates/shiori-host32-testdll`（`load` のみ）の改変（**無改変**で残す＝「`loadu` が無い DLL は今日のまま」の証拠は既存テスト全緑で得る）。
  - 完了 spec（`areka-P0-host32-shiori-load` ほか）の文書の改変。要件の上書きは本仕様の requirements と COMPAT §8 に書く。
- **Adjacent expectations**:
  - **前提（いずれも完了済み・先行必須の未完 spec は無い）**: `areka-P0-host32-shiori-load`（`ShioriByteProxy`）・`areka-P0-host32-request`・`areka-P0-host32-window-thread-pump`・`areka-P0-nar-install`（検体は `.nar`）・`areka-P0-shell-implicit-surface`（里々／YAYA の検体が実機で動く）。
  - **下流**: `areka-P0-makoto-dll-host` は `ShioriByteProxy` を流用して `loadu` 優先を無償で得る（brief の切れ端 ⓐ は本仕様へ移管済み）。`areka-P0-ukadoc-coverage` 系の台帳は本仕様が 1 項目の記述だけを直す。
  - **並走**: `property-ipc-transport`・`property-query-channels` は `shiori-host32-*` を触るが `shiori_proxy.rs` の入口解決には触らない見込み（着手時に rebase して確認）。
  - **本仕様が上書きする完了要件**: 完了 `areka-P0-host32-shiori-load` の「`load`／`unload`／`request` の 3 エクスポートすべてを解決し、いずれか欠落→`EntryNotFound`」（`shiori_proxy.rs` 冒頭の確立シーケンス手順 2）を、「`unload`・`request` は必須、初期化の入口は `loadu`／`load` のどちらか 1 つが在ればよい」に改める。同 spec の「戻り `bool` は Rust bool 1 byte」（`LoadFn`／`UnloadFn` の定義行）を「1 バイト整数・0 か否か」に改める。

## Requirements

### Requirement 1: 初期化の入口の選択（`loadu` 優先）

**Objective:** As a ゴーストの利用者, I want 助け手が SHIORI DLL の持つ入口のうち正典が優先するものを呼ぶこと, so that `loadu` を持つ SHIORI ではフォルダの字がどんな字でも辞書を見失わない。

#### Acceptance Criteria

1. When SHIORI DLL が `loadu` と `load` の両方を公開している, the 助け手 shall `loadu` だけを呼び、`load` は呼ばない。
2. When SHIORI DLL が `loadu` を公開し `load` を公開していない, the 助け手 shall その DLL を受け入れて `loadu` を呼ぶ（裁定 1・`loadu`／`load` のどちらか 1 つが在ればよい）。
3. When SHIORI DLL が `load` を公開し `loadu` を公開していない, the 助け手 shall 今日と同じく `load` を呼ぶ（既存の検体＝里々・pasta と既存のテストは挙動不変）。
4. If SHIORI DLL が `loadu` も `load` も公開していない, then the 助け手 shall 今日と同じ「入口が無い」失敗（`EntryNotFound` 相当）として確立を失敗させ、失敗を 1 行記録し、親へは今日と同じ ack `[0]` を返す。
5. The 助け手 shall 1 回の確立で `loadu` と `load` を **合わせて最大 1 回**しか呼ばない（両方を呼ぶ経路は存在しない）。
6. If `loadu` が偽（0）を返した, then the 助け手 shall 確立を「初期化が偽を返した」失敗（今日 `load` が偽を返したときと同じ種別）として扱い、`load` へは **落ちない**（裁定 2・正典の落ちる条件は「実装されていない場合」だけ）。
7. The 助け手 shall `unload` と `request` を今日と同じく必須の入口として引き、いずれかが欠ければ今日と同じ「入口が無い」失敗にする（`loadu` の追加で `unload`・`request` の扱いは **変わらない**）。
8. The 助け手 shall 入口の選択の判断（`loadu` の有無 × `load` の有無の 4 通り）を、DLL を読まずに検証できる形で 1 か所に持つ。

### Requirement 2: `loadu` へ渡すパス

**Objective:** As a ゴーストの利用者, I want `loadu` に渡るパスが置き場所を正確に表していること, so that 既定コードページに無い字を含むフォルダでも SHIORI が辞書を見つけられる。

#### Acceptance Criteria

1. When 助け手が `loadu` を呼ぶ, the 助け手 shall 今日 `load` に渡しているのと **同じ文字列**（argv 第 2 引数または env `HOST32_LOAD_DIR` に由来する置き場所のパス。末尾の区切りの有無・大文字小文字・区切り文字の種類を含めて同一）を UTF-8 に符号化したバイト列を渡す。
2. The 助け手 shall `loadu` に渡すバイト列の長さを正味のバイト数とし、NUL 終端を付けない（今日の `load` と同じ作法）。
3. The 助け手 shall `loadu` へ渡すメモリの確保と所有権の規約を今日の `load` と同一にする（正典「メモリ管理」の節＝ベースウェアが確保し、モジュールが解放する）。
4. When パスに ASCII 以外の字（例: 日本語・韓国語・絵文字）が含まれる, the 助け手 shall それらを置き換えず UTF-8 のまま `loadu` へ渡す。
5. The 助け手 shall UTF-8 化に新しい依存も新しい OS 呼び出しも使わない（パスは helper の中では常に正しい Unicode として保持されている）。

### Requirement 3: `load` へ落ちたときの表せない字の検出と警告

**Objective:** As a ゴーストの利用者・運用者, I want 既定コードページで表せない字がパスに含まれていたことが警告として残ること, so that SHIORI が辞書を見失ったときに原因を突き止められる（今日は黙って `?` に化け、ログ 0 行）。

#### Acceptance Criteria

1. When 助け手が `load` を呼び、かつ パスに既定コードページで表せない字が 1 つ以上含まれる, the 助け手 shall 警告を **1 行**（`warn` 相当・既存の `[helper]` 行と同じ経路）に、元のパスと「表せない字があり既定文字へ置き換えた」旨を書く。
2. When 上記の警告を出した, the 助け手 shall それでも置き換え済みのバイト列を `load` へ **渡し**、確立を止めない（裁定 3・DLL が自分で偽を返せば要件 1.6 と同じ「初期化が偽を返した」失敗になる）。
3. When 助け手が `load` を呼び、かつ パスの全ての字が既定コードページで表せる, the 助け手 shall 警告を **0 行**出す（今日と同じ）。
4. The 助け手 shall 表せない字の判定を「実行中の既定コードページで表せるか」で行い、CP932 を前提にしない（日本語以外のロケールでも正しく判定する）。
5. When 助け手が `loadu` を呼ぶ, the 助け手 shall 表せない字の判定を **行わない**（UTF-8 に表せない字は無い＝この警告は `load` の枝だけに存在する）。
6. If 既定コードページへの変換そのものが失敗した（長さ問い合わせまたは変換が 0 以下）, then the 助け手 shall 今日と同じ「符号化失敗」の失敗（`EncodingFailed` 相当）として確立を失敗させ、失敗を 1 行記録する。

### Requirement 4: どの入口を使ったかの記録

**Objective:** As a 運用者・開発者, I want 助け手がどの入口で SHIORI を確立したかがログから分かること, so that 実機確認と障害調査でログを grep するだけで判定できる。

#### Acceptance Criteria

1. When 助け手が `loadu` または `load` を呼んだ, the 助け手 shall 呼んだ入口の名前（`loadu`／`load`）を **1 行**、既存の `[helper]` 行と同じ経路（helper の標準エラー出力＝親の stderr にそのまま混ざる）に記録する。
2. The 助け手 shall 要件 4.1 の行を、成功・失敗のどちらの場合も **入口を呼ぶ前または呼んだ直後に 1 回だけ**出し、同じ確立で 2 回以上出さない。
3. The 助け手 shall 要件 4.1 の行の書式を固定し（入口名を含む決まった語句）、実機確認の grep がその語句で判定できるようにする。
4. If 確立が失敗した（入口が無い・符号化失敗・初期化が偽を返した・DLL が開けない）, then the 助け手 shall 失敗の種別を 1 行記録する（今日の `[helper] LOAD 失敗（観測・ack[0]）: …` と同じ経路。**ログ無しの失敗経路は 0 本**）。
5. The 助け手 shall 親への応答（load-ack の 1 バイト・`MsgTag`）を今日のまま変えず、どの入口を使ったかを親へ **伝えない**（親は知らないまま・凍結した受け渡しは無改変）。

### Requirement 5: 戻り値の受け方

**Objective:** As a 開発者, I want `loadu`・`load`・`unload` の戻り値を C 製 DLL（`BOOL`・4 バイト）と pasta（下位 1 バイトだけを書く）の両方で正しく判定すること, so that 下位バイトが 0／1 以外の値でも未定義動作にならない。

#### Acceptance Criteria

1. The 助け手 shall `loadu`・`load`・`unload` の戻り値を **1 バイトの整数**として受け、**0 を失敗・0 以外を成功**と判定する（Rust の `bool` としては受けない）。
2. When `loadu` または `load` が 0 以外の任意の値（1 に限らない）を返した, the 助け手 shall 確立を成功として扱う。
3. When `loadu` または `load` が 0 を返した, the 助け手 shall 確立を「初期化が偽を返した」失敗として扱う。
4. The 助け手 shall `unload` の戻り値を今日と同じく **無視**する（Drop での courtesy `unload`・結果は成功失敗を問わない）。変わるのは受ける型だけで、判定の追加は **0 件**。
5. The 助け手 shall `request` の署名（`HGLOBAL` を返す）を **変えない**。

### Requirement 6: 決定論テストと 2 本目の偽 DLL

**Objective:** As a 開発者, I want 入口の選択・UTF-8 化・表せない字の検出・`loadu` の実呼出が決定論テストで固定されること, so that 優先順を逆にする・`load` へ落ちる・両方呼ぶ、といった後退が赤で止まる。

#### Acceptance Criteria

1. The 本仕様 shall `loadu`・`load`・`unload`・`request` の 4 つを公開する **2 本目の偽 32bit DLL**（別クレート・別の出力名）を用意し、`loadu`／`load` の **どちらが呼ばれたか**と **受け取ったバイト列**を env で指定したファイルへ書き、env で `loadu` が偽を返すことを注入できるようにする（注入 env は既存 fixture と同じ `HOST32_TESTDLL_` の接頭辞）。
2. The 本仕様 shall 既存の偽 DLL `crates/shiori-host32-testdll` を **無改変**で残す。
3. The 本仕様 shall 入口の選択の判断表 4 行（両方在る→`loadu`／`loadu` のみ→`loadu`／`load` のみ→`load`／両方無い→失敗）を、DLL を読まない純関数のテストとして x64 で常時実行する。
4. The 本仕様 shall UTF-8 符号化の固定バイト列を、CP932 に在る字と無い字（絵文字など）の両方を含むパスで検証する（x64・常時実行）。
5. The 本仕様 shall 表せない字の検出を、機械の既定コードページに **依存せず**決定論的に検証する（検出の判断が、表せる字だけのパスでは「無し」、表せない字を含むパスでは「有り」になること。テストが CP932 環境にも UTF-8 が既定の環境にも縛られないこと）。
6. When i686 で 2 本目の偽 DLL を実際に読む, the テスト shall `loadu` が呼ばれ `load` は呼ばれないこと、受け取ったバイト列が渡したパスの UTF-8 と一致することを確認し、優先順を逆にすると赤になる（記録が `load` に変わる）。
7. When i686 で `loadu` が偽を返す注入を有効にする, the テスト shall 確立が「初期化が偽を返した」失敗になり、かつ `load` が呼ばれた記録が **無い**ことを確認する（裁定 2）。
8. The 本仕様 shall 既存の helper 単体テストと `crates/shiori-host32-host/tests/*_e2e.rs` を **無改変**で緑に保つ（`load` の枝の不変の証拠）。
9. The 本仕様 shall i686 の先ビルド手順（`crates/shiori-host32-host/README.md`・テストの panic 文言・e2e の doc コメントなど、手順を書いている **全ての場所**）に 2 本目の偽 DLL のビルドを足す。
10. The 本仕様 shall `cargo test --workspace`（i686 先ビルド済み）を緑にする。

### Requirement 7: 実機確認

**Objective:** As a 開発者, I want 手元の検体で `loadu` と `load` の両方の枝が実機で踏まれることを確認すること, so that テストが隠す欠陥を炙り出せる。

#### Acceptance Criteria

1. When `konnoyayame`（YAYA・`loadu` 持ち）を有界 auto-exit で起動する, the 助け手のログ shall 要件 4.1 の行に `loadu` を記録し、`load` の記録を含まない。
2. When `R_POST_and_KOMAINU`（里々）と `emo2`（pasta）を有界 auto-exit で起動する, the 助け手のログ shall 要件 4.1 の行に `load` を記録し、警告（要件 3.1）を含まない。
3. When 既定コードページに無い字を含むフォルダの下へ `konnoyayame` を置いて起動する, the ゴースト shall 喋る（辞書を見失わない）。同じ場所の里々では要件 3.1 の警告が **1 行**出る。実装の前に、**今日どう壊れるか**を同じ配置で赤として記録する（未実測）。
4. The 本仕様 shall 着手時に検体 3 体の入口の有無を正規の道具（`dumpbin /exports` 等）で取り直し、brief の表（YAYA＝`loadu` あり／里々・pasta＝`loadu` なし）と一致することを確認する。一致しなければ要件 7.1〜7.2 の期待値を実物に合わせて改める。
5. The 実機確認 shall 判定の分岐が出すログの水準まで開けて走らせる（要件 4.1 の行と要件 3.1 の警告が捨てられないこと）。

### Requirement 8: 文書・台帳の追随

**Objective:** As a 開発者・第三者, I want 正典との差の記録と裁定の記録が実装と一致すること, so that 台帳と設計文書を読めば areka が `loadu` をどう扱うか分かる。

#### Acceptance Criteria

1. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 の表に裁定 3 件（`loadu` だけの DLL を受け入れる／`loadu` が偽でも `load` へ落ちない／`load` へ落ちて表せない字があれば警告して渡す）を、根拠（正典の沈黙箇所）と出典 spec 付きで **3 行**追記する。
2. The 本仕様 shall 台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `[entry."ukadoc:spec_dll"]` の `note` と、台帳冒頭の注釈（群 14c）から「loadu は引かない」の記述を取り去り、実装（`loadu` 優先・`load` へのフォールバック・表せない字の警告）に合わせる。`status` は `degraded` の **まま**（SAORI・MAKOTO・PLUGIN が残る）。
3. The 本仕様 shall 派生文書（`doc/ukadoc-coverage/briefing-shiori.md` ほか）を `ukadoc-survey` の生成器で撮り直し、**手で直さない**。
4. The 本仕様 shall `shiori_proxy.rs` 冒頭の「確立シーケンス」の説明を 4 つの入口（`loadu`／`load`／`unload`／`request`）と入口の選択に合わせて書き直す。
5. The 本仕様 shall 完了 spec の文書（`.kiro/specs/completed/areka-P0-host32-shiori-load/` ほか）を **改変しない**（上書きは本 requirements と COMPAT §8 に書く）。

### Requirement 9: 制約（非機能）

**Objective:** As a 開発者, I want 本仕様の変更が既存の規律の中に収まること, so that 32bit の助け手の可搬性と安全性が保たれる。

#### Acceptance Criteria

1. The 本仕様 shall 新しい依存クレート・新しい技術を **0 件**とする。
2. The 本仕様 shall 本番の env 変数を **追加しない**（新設は fixture の注入 env のみで、接頭辞は既存の `HOST32_TESTDLL_`）。
3. The 本仕様 shall unsafe を `shiori_proxy.rs` に集約し、各ブロックに Safety 根拠を書く（steering）。
4. The 本仕様 shall 1 ファイルを 1,000 行未満に保つ（`shiori_proxy.rs` は現在 585 行＝テストが膨らむなら兄弟ファイルへ）。
5. The 本仕様 shall 32bit 可搬性の適用範囲を host-32 系（helper は i686）に限り、純関数のテストは x64 で常時、DLL を読むテストは i686 限定（既存の `#[cfg_attr(not(target_arch = "x86"), ignore)]` の作法）とする。
6. The 本仕様 shall 全ての失敗経路がログを残すこと（`error!`／`warn!`／helper の `eprintln!("[helper] …")`）を保ち、黙って失敗する経路を **0 本**にする。
