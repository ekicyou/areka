# Brief: areka-P0-shiori-host-32

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。

## Problem
既存の里々(satori)/YAYA 等の SHIORI DLL は大半が 32bit で、SAORI サブDLLも 32bit。64bit areka はこれらをプロセス内ロードできない。「既存SHIORIが動かないと面白くない」ため過去互換の起動手段が要る。

## Current State
`IShiori`（COM, `areka-P0-shiori-com`）が内部ABIとして定義される見込み。だが旧 flat-C DLL（load/unload/request＋HGLOBAL＋charset）をホストする経路が無い。

## Desired Outcome
**32bit Rust 随伴ホスト（`areka-shiori-host`, i686）**が本物の shiori.dll を実行時ロードし、SAORIサブDLLも同居させ、`IShiori` 相当として 64bit areka へ橋渡しする。窓を作るSHIORIも満たす自前メッセージループを持つ。

## Approach
`LoadLibraryW`＋`GetProcAddress` で `load/unload/request` を解決（`extern "C"` cdecl）。64bit側で HSTRING→Charset符号化バイト列へ早期変換し、自前IPC（名前付きパイプ/共有メモリ）で授受。ホスト側で `GlobalAlloc`→HGLOBAL 化し DLL を駆動、SHIORI規約の所有権はホスト内に閉じる。毎秒 `OnSecondChange` ポーリング。SAORIは同プロセス同居（実DLLを飼う）。

## Scope
- **In**: 32bit host バイナリ、DLL動的ロード、HGLOBAL/charset マーシャリング、自前IPCプロトコル(フレーミング/エラー/タイムアウト/プロセス監視)、自前メッセージループ、毎秒ポーリング、SAORI同居
- **Out**: ネイティブ in-proc 経路（→ shiori-com）、さくらスクリプト解釈、里々/YAYA の再実装（実DLLを飼うので不要）

## Boundary Candidates
- 32bit host プロセス／ライフサイクル
- IPC プロトコル
- HGLOBAL/charset マーシャリング層

## Out of Boundary
- ネイティブ脳、SERIKO、バルーン

## Upstream / Downstream
- **Upstream**: `areka-P0-shiori-com`（橋渡し先の `IShiori`）
- **Downstream**: `areka-P0-compat-ghost-integration`

## Existing Spec Touchpoints
- **Adjacent**: `process_singleton`（プロセス制御の既存知見）、`crate-name-reservation`（完了）

## Constraints
- i686 ターゲットの随伴バイナリをワークスペースに追加（クロスbitness）。里々はソースビルドせず実DLLをホスト（bitness連鎖回避）。クラッシュ分離を維持。
