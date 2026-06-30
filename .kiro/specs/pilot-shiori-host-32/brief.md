# Brief: pilot-shiori-host-32

> **種別: 先進坑（pilot・使い捨て）。** 二坑モデル規律の正本は [.kiro/steering/two-tunnel.md](../../steering/two-tunnel.md)。
> 成果物はコードではなく**知見（go／違う／直す ＋ 学び）**。一次記録は `crates/pilot/examples/shiori-host-32/README.md`（3 幕構成）。

## Problem

areka（x64）が適合対象ゴースト emo2 を「そのまま」起動するには、emo2 の脳 `pasta.dll` を駆動せねばならない。だが `pasta.dll` は **PE Machine 0x14C = x86(32bit)**（`emo2-conformance-scope.md §0`）であり、**x64 プロセスへ in-proc ロード不可**。SHIORI を別プロセス（32bit）でホストし IPC で橋渡しする機構（host-32）が必須となる。

これは M1 の**唯一の耐力壁**ですわ。「x64 areka が emo2 の 32bit `pasta.dll` を駆動できるか」が確証されない限り、host-32 通信層トラック（本坑 `areka-P0-host32-*` 群）に着手するのは「見てから違う」の最悪パターン——コード肥大後に方向の誤りが発覚し、やり直し地獄に落ちる。ゆえに**本実装の前に**先進坑で実現可能性を潰し、開発者の go 判定を取る。

## Current State

- **存在する土台**:
  - `crates/shiori-abi`（内部唯一 ABI `IShiori`/`IShioriHost` COM・HSTRING/UTF-16）— 完成済み。host-32 が最終的に橋渡しする先の内部契約。
  - `crates/pilot`（先進坑の検疫所・空 lib ＋ examples-only ＝命綱の構造的担保）— 受け皿は既にある。`examples/_template/` 雛形あり。
  - emo2 実物（`pasta.dll`・32bit SHIORI/3.0・UTF-8・SAORI 不使用）— 検証ターゲットが実在。
- **存在しないもの**: x64↔32bit ブリッジの実コードはゼロ。`crates/pilot/examples/shiori-host-32/` フォルダ未作成。host-32 本坑トラックは全 BLOCKED（go 待ち）。

## Desired Outcome

開発者が「この方向で本坑を掘れる（go）」と人間判断できるだけの**実走知見**が得られている。具体的な **go 基準**（`roadmap.md` 唯一の耐力壁節）:

1. **1 往復成功**: x64 から 32bit `pasta.dll` を `load → OnBoot → Value 受領 → unload` の一周が成功する（emo2 の起動挨拶さくらスクリプトが `Value:` として x64 側へ返る）。
2. **メッセージループ生存**: 窓を持つ SHIORI に対応する自前メッセージループが、helper プロセス側で安定して回り続ける（N 秒運転して clean unload できる）。

両者を満たせば go。満たせない／要修正なら、その学びと共に README 検証結果へ記録する。

## Approach

`crates/pilot/examples/shiori-host-32/`（**1 仕様 = 1 フォルダ**・`main.rs` 必須）に**使い捨ての最小探索コード**を置く。`_template/` をコピーして起点とする。

- x64 親（areka 側相当）と 32bit helper（SHIORI ホスト）を**別プロセス**で立て、自前 IPC（フレーミング＋プロセス監視）で結ぶ。
- 32bit helper が `LoadLibrary(pasta.dll)` → SHIORI `load(ghostdir)` → `request(OnBoot)` → `unload` を実行し、`Value:` を IPC で x64 親へ返す。
- helper は窓を作る SHIORI に備え**自前メッセージループ**を回す。
- **品質は緩くてよい**（使い捨て前提）。整形・命名・テストの厳格さは求めない。**但し葉ノード隔離だけは厳守**（production クレートは pilot に依存しない）。

なぜこの方向か: 32bit/x64 境界はプロセス分離以外に解がなく（in-proc 不可は PE Machine が確定）、areka は既に「SHIORI 内部 ABI = COM」「過去互換 = 32bit Rust ホスト＋自前 IPC」という設計判断（`COMPAT_ARCHITECTURE.md`）を持つ。先進坑はその設計判断の**実現可能性の一点突破検証**に徹する。

## Scope

- **In**:
  - x64↔32bit の最小プロセス間ブリッジ（自前 IPC・フレーミング・プロセス生存監視）
  - 32bit helper での `pasta.dll` 動的ロード（`load`/`unload`/`request` 解決）
  - SHIORI/3.0 リクエスト 1 種（`OnBoot`）の組み立て・marshal・`Value` 受領
  - 窓持ち SHIORI 対応の自前メッセージループ生存確認
  - README 3 幕（動機 → 概要・実行法 → 検証結果 go/違う/直す＋学び＋日付）
- **Out**:
  - **SAORI 同居**（emo2 は DLL が `pasta.dll` 1 個のみ・`saori` 系 grep 全ゼロ → M1 不要）
  - production 品質のマーシャリング堅牢性・全 SHIORI イベント網羅（`OnSecondChange` 等は本坑 `host32-*` の領分）
  - charset 多様性（emo2 は UTF-8 のみ。Shift_JIS は里々/YAYA 生態系拡張で後続）
  - 本坑 host-32 の実装そのもの（先進坑は方向確認に徹し、本坑は知見を見て**一から綺麗に掘り直す**＝コピペ donor 流用禁止）

## Boundary Candidates

- 32bit helper プロセスの起動・生存監視・clean shutdown
- 自前 IPC レイヤ（メッセージフレーミング／タイムアウト／プロセス監視）
- 32bit SHIORI DLL 動的ロードと `load/unload/request` 解決
- SHIORI/3.0 リクエスト/レスポンス marshal（`OnBoot` → `Value`）
- 窓持ち SHIORI のための自前メッセージループ

## Out of Boundary

- 本坑 host-32 トラック（`areka-P0-host32-ipc` / `-shiori-load` / `-request` / `-lifecycle`）の実装＝この先進坑の go 判定**後**に着手する別物
- SAORI ブリッジ（M1 範囲外・emo2 未使用）
- emo2 の脳の中身（`.pasta`/`.lua`/`pasta.toml`/budoux/縦書き）の解釈＝すべて `pasta.dll` の腹の中で areka は一切触らない
- SERIKO 描画・さくらスクリプト解釈・バルーン描画（別エンジントラック）

## Upstream / Downstream

- **Upstream**: `crates/shiori-abi`（最終橋渡し先の内部 ABI `IShiori`）／ `crates/pilot`（検疫所・命綱構造）／ emo2 実物 `pasta.dll`（検証ターゲット）／ 設計判断正本 `doc/COMPAT_ARCHITECTURE.md`。
- **Downstream**: 本先進坑の **go 判定が以下の本坑トラックを gate** する（`two-tunnel.md` ハードゲート・`_Depends(confirmed): pilot/shiori-host-32`）:
  - `areka-P0-host32-ipc`（x64↔32bit helper＋pipe＋handshake/lifecycle）
  - `areka-P0-host32-shiori-load`（LoadLibrary＋load/unload/request 解決）
  - `areka-P0-host32-request`（SHIORI/3.0 build＋marshal＋Value＋charset）
  - `areka-P0-host32-lifecycle`（msg loop＋OnSecondChange poll＋unload＋crash 監視）

## Existing Spec Touchpoints

- **Extends**: なし（新規先進坑）。
- **Adjacent**: 上記 `areka-P0-host32-*` 本坑群（この先進坑が gate する被依存先・実装は重複させず知見参照）。命綱上、production クレート（wintf/dola/areka/shiori-abi）は本先進坑コードに**依存してはならない**。

## Constraints

- **二坑規律**: 使い捨て前提・葉ノード隔離厳守（`crates/pilot` の空 lib ＋ examples-only 構造で担保）・知見の一次記録は README 3 幕・go 判定は**開発者の人間判断**。
- **可搬性**: 32bit/x64 境界を崩さない。Rust 2024。helper は 32bit ターゲット、親は x64。
- **既知制約**: worktree で example を実ビルド/実行する際は前段で `git submodule update --init --recursive`（`vendors/pasta` 未 populate 回避）。ビルドは PowerShell（Git Bash の GNU `link.exe` が MSVC link を遮蔽する既知トラップ）。
- **設計判断**: SHIORI 内部唯一 ABI = `IShiori`(COM, HSTRING/UTF-16)。過去互換 = 32bit Rust ホスト（flat-C/HGLOBAL/charset/自前 IPC）。変更は `doc/COMPAT_ARCHITECTURE.md` を正本として更新。
