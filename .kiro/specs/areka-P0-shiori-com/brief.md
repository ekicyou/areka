# Brief: areka-P0-shiori-com

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md` §5。

## Problem
脳（SHIORI）と areka の境界に、ネイティブ脳（pasta）と過去互換DLLの両方を同一視できる安定したABIが要る。呼び出し側に「ネイティブ/過去互換」の分岐を出したくない。

## Current State
areka は windows-rs 経由で COM を多用（DComp/D2D/DWrite）。だが SHIORI 抽象は未定義。pasta はnative脳の候補。

## Desired Outcome
areka の**内部唯一の SHIORI ABI = `IShiori`（COM, HSTRING/UTF-16）**が定義され、ネイティブ脳が in-proc COM で直結できる。push 用の `IShioriHost`(sink) も定義し、native脳の能動wakeupを可能にする。

## Approach
`IShiori`（hload/hunload/hrequest 相当のメソッド、HSTRING引数）と `IShioriHost`（Raise 等）を COM インターフェイスとして定義。ネイティブ経路は in-proc 直結（マーシャリングゼロ）。x64/CPUネイティブ前提（x86除外）。OOP自動マーシャリングは設計上回避（過去互換は別hostで自前IPC）。pasta が `IShiori` をnative実装する受け皿。

## Scope
- **In**: `IShiori`/`IShioriHost` のインターフェイス定義、in-proc ネイティブ・アクティベーション、HSTRING取り回し、push(sink)経路、ライフサイクル(load/unload)
- **Out**: 過去互換DLLホスティング（→ `areka-P0-shiori-host-32`）、さくらスクリプト解釈（→ sakura-script）、毎秒ポーリング駆動の上位ロジック

## Boundary Candidates
- インターフェイス面（メソッド・文字列・エラー）
- アクティベーション（in-proc native）
- push(sink) 機構

## Out of Boundary
- 32bit/レガシー、SAORI、IPC

## Upstream / Downstream
- **Upstream**: COM基盤（windows-rs・完了）
- **Downstream**: `areka-P0-shiori-host-32`（同 `IShiori` を実装する一実装）、`areka-P0-compat-ghost-integration`、ぱすたさんnative脳

## Existing Spec Touchpoints
- **Adjacent**: `com-resource-naming-unification`（完了・COM命名規約）

## Constraints
- HSTRING/UTF-16。在プロセスHSTRING取り回しはWinRTランタイム非依存（windows-rs純Rust実装）。OOP自動マーシャリングを要求しない設計に保つ。
