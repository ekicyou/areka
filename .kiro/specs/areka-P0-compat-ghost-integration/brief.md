# Brief: areka-P0-compat-ghost-integration

> 上位設計の正本: `doc/COMPAT_ARCHITECTURE.md`。M1（互換ベースウェア）の縦スライス到達点。

## Problem
T1〜T3 の各部品（SERIKO/シェルローダ／さくらスクリプト/バルーンローダ／SHIORIホスト）が揃っても、それらを束ねて「実在の伺かゴースト1体が実際に表示・会話する」E2E統合が無ければ、互換ベースウェアの達成にならない。

## Current State
areka バイナリは試作済（シェル＋バルーン2ウィンドウ、ドラッグ、ダブルクリック終了）。各互換部品は個別 spec で実装される見込み。横断統合が未整備。

## Desired Outcome
**実在の里々ベースゴースト1体が、SAORI込みで areka 上に表示され会話する**（M1北極星の達成）。ゴーストディレクトリのロード→シェル表示→SHIORI起動→さくらスクリプト往復→バルーン会話、の一連が通る。

## Approach
ゴーストパッケージ（ghost/master の descript・shiori.dll、shell、balloon）を解決し、shell-loader→seriko-runtime で表示、shiori-host-32 で里々を起動、OnBoot/OnSecondChange/マウスイベントを `IShiori` 経由で往復、応答さくらスクリプトを sakura-script で実行、balloon-loader＋balloon-system で会話表示。

## Scope
- **In**: ゴースト解決・ライフサイクル統合、イベント配線（boot/clock/mouse/close）、各部品のオーケストレーション、実ゴースト1体での通し検証
- **Out**: 各部品の内部実装（個別 spec）、nar インストール、複数ゴースト同時/切替（将来）、ぱすたさんnative（→ M2）

## Boundary Candidates
- ゴースト・ライフサイクル/解決
- イベント・オーケストレーション
- E2E 検証シナリオ

## Out of Boundary
- 各部品の詳細実装、配布/インストーラ、native旗艦

## Upstream / Downstream
- **Upstream**: `areka-P0-shell-loader`, `areka-P0-seriko-runtime`, `areka-P0-sakura-script`, `areka-P0-balloon-loader`, `areka-P0-shiori-host-32`, `areka-P0-shiori-com`
- **Downstream**: M2（ぱすたさんnative）、アプリ統合（system-tray/persistence 等）

## Existing Spec Touchpoints
- **Extends**: areka バイナリ試作、`completed/areka-mock-shell`
- **Adjacent**: `multiwindow-event-validation`（完了）

## Constraints
- ukadoc 準拠。検証は「実在ゴースト1体が SAORI込みで喋る」を合格基準とする（fresh evidence）。
