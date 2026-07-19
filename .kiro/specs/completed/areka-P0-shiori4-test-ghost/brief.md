# Brief: areka-P0-shiori4-test-ghost

> **種別**: 本坑（main）・① shiori トラック帰属のテスト基盤ユニット（＋M2 native SHIORI4 の正規シーム先行敷設）。
> **調査日**: 2026-07-18（/kiro-discovery・Explore 実査）。

## Problem

areka の実 boot 経路（package-mount → SHIORI DLL ロード → wire → talk）を貫くテストが、**実ゴースト emo2（脳＝32bit `pasta.dll`）にしか乗れない**：

- `pasta.dll` は Lua 駆動の**非決定論脳**（乱数選択・実時間依存）＝再現性のある回帰檻にならない。
- 32bit ゆえ **i686 成果物ビルド＋別プロセス helper が前提**（[[workspace-test-needs-i686-host32-artifacts]]）——テストの重量・前提が過大。
- 既存の決定論シーム `ScriptedShioriBackend`（`ShioriWiring::Custom`・[spine_e2e_test.rs](../../../crates/areka-ghost/tests/ghost/spine_e2e_test.rs)）は **mount 解決→DLL ロード→SHIORI 境界を全部バイパス**する＝その実境界は非決定論テスト（env-gate 実 pasta）でしか踏めていない。

つまり「決定論だが境界を踏まない（Custom）」か「境界を踏むが非決定論＋i686 前提（emo2）」の二極しかなく、**中間＝境界を踏む決定論テストゴースト**が不在。

## Current State（2026-07-18 実査）

- **IShiori COM ABI 完備**: `crates/shiori-abi`（`IShioriFactory::CreateInstance`＝create+load 融合・`IShiori::Get/Notify`・`IShioriHost::Raise/Complete/GetProperty/SetProperty`）。x64 内部唯一 ABI（completed `areka-P0-shiori-com` 系譜）。
- **ReferenceBrain 実在・ただし DLL でない**: `crates/areka/src/reference_brain.rs`＝決定論エコー脳＋`shiori_factory` エクスポート（`extern "system"`・[[windows-com-export-calling-convention]]）。だが areka は bin-only（[[areka-bin-crate-internal-tests-in-crate]]）＝ **cdylib が存在せず in-proc テスト到達のみ**。
- **boot の SHIORI 結線は2択のみ**: `ShioriWiring::Helper`（i686 別プロセス・本番唯一経路）／`ShioriWiring::Custom`（クロージャ注入）——**x64 in-proc IShiori DLL ロード経路は無い**（[shiori_wiring.rs](../../../crates/areka-ghost/src/shiori_wiring.rs)）。
- **マウント可能な完全ゴースト fixture は emo2 ただ一つ**（`crates/pilot/examples/shiori-host-32/fixtures/emo2/`）。`smoke_boot_loop_exit.rs`／`emo2_real_run.rs` はこれに依存。
- **決定論 32bit fixture はある**: `shiori-host32-testdll`（i686 flat-C・固定応答）——ただし i686 ビルド＋helper 経由が前提＝軽量化にならない。
- **純 x64 SHIORI/3.0 codec は完備**: `build_request`/`parse_response`（completed `areka-P0-host32-request`）——in-proc アダプタで再利用可能。

## Desired Outcome

**`cargo test --workspace`（常設ゲート・[[areka-no-ci-gpu-tests-in-cargo-test]]）内で、実 boot 経路（descript.txt 起点 mount → x64 SHIORI4 DLL 実ロード → SHIORI 交信 → talk 再生 → close 握手）が pasta 非依存・i686 非依存・sleep 不使用で決定論的に一周する。**

✔ 観測（単一 pass/fail）: テストゴースト（x64 SHIORI4 脳）を絶対パス指定で boot → 台本どおりの OnBoot talk が sink に観測される → clean close、の e2e が決定論 green（[[deterministic-test-coverage-mandate]]）。

## Approach

三点セット（1ユニット＝1かたまりの動く振る舞い）:

1. **x64 SHIORI4 テスト DLL（新規 cdylib crate・命名候補 `shiori4-testdll`）**: `shiori_factory` → `IShioriFactory`/`IShiori` 実装の決定論脳。`shiori-host32-testdll` の x64/COM 版に相当。応答は台本駆動（M1 イベント最小集合: OnFirstBoot/OnBoot/OnClose/OnSecondChange/マウス系/選択系の固定 sakura script・未知 ID は 204 相当）。既存 `ReferenceBrain` を種として活用（echo でなく固定台本へ）。SHIORI4 content は SHIORI/3.0 テキストを不透明搬送（[[areka-shiori-layer-naming-and-memory]]・codec 再利用）。
2. **正規 in-proc ロード経路 `ShioriWiring::InProc`**: `LoadLibraryW`（x64 DLL）→ `GetProcAddress("shiori_factory")` → `CreateInstance(load_dir,…)` → `IShiori` を `ShioriBackend` へアダプト。**テスト専用ハックではなく正規実装**（[[canonical-not-minimal-lifecycle]]）——M2 native pasta x64 が同じシームに乗る。失敗経路は log-first（[[areka-log-first-no-silent-failure]]）。
3. **マウント可能テストゴースト fixture**: 完全なゴーストフォルダ（`descript.txt`＝charset UTF-8＋`shiori,<testdll>`・最小 shell（surfaces.txt＋数枚 PNG）・最小 balloon）。emo2 を置き換える軽量決定論 fixture として smoke/e2e が消費。

置換の規律: 既存 env-gate 実 pasta 追験（`HOST32_PASTA_DLL`／`AREKA_EMO2_REAL_RUN`）は**廃止しない**（実機サインオフの正本・[[obsolete-vs-broken-test-policy]]）。本 spec は常設決定論檻を「emo2 依存の smoke」から「テストゴースト駆動」へ差し替え／追加する。

## Scope

- **In**: x64 SHIORI4 テスト DLL（cdylib）／`ShioriWiring::InProc` 正規ロード経路＋`IShiori`→`ShioriBackend` アダプタ／テストゴースト fixture（descript＋最小 shell/balloon）／boot→talk→close の決定論 e2e（常設）／既存 smoke の乗り換え判断（陳腐化仕分け）。
- **Out**: 本番 main の結線変更（M1 の本番ゴーストは emo2＝`Helper` 経路のまま。descript 駆動の bitness/種別自動判別は M2 シーム予約・[[areka-descript-encoding-ishiori-utf8]] の charset 規則が将来の判別鍵）／`IShioriHost::Raise` 起点の自発イベントや deferred (`SHIORI_S_PENDING`) の網羅（ReferenceBrain の既存檻が持つ分を超えない・必要最小のみ）／emo2-conformance-e2e の spine 改稿（下記）／SAORI・里々/YAYA。

## Boundary Candidates

- テスト DLL（脳・台本）／ロード経路＋アダプタ（areka-ghost 側シーム）／fixture（データ）——の三層。実装は同一 spec 内で直列。

## Out of Boundary

- **`areka-P0-emo2-conformance-e2e` の決定論 spine は `ScriptedShioriBackend` 拡張のまま**（同 brief:51 の裁定を侵さない）。将来「conformance spine をテストゴーストに乗せ換えるか」は**別セッションの合流判断**（[[portfolio-convergence-decided-in-separate-session]]）。
- 各エンジン内部品質（各 spec の檻が正本）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-shiori-com`〜`-shiori-reference`（IShiori ABI＋ReferenceBrain）／`completed/areka-P0-ghost-setup`（`ShioriWiring`・boot 背骨）／`completed/areka-P0-host32-request`（純 x64 codec）／`completed/areka-P0-package-mount`（descript 解決）。
- **Downstream**: **M2 native pasta x64**（`InProc` シームの本番消費者・vendored `vendors/pasta` の行き先）／`areka-P0-emo2-conformance-e2e`（乗り換え候補・合流判断は別途）／以降の全 e2e 系テストの軽量決定論土台。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-shiori-reference`（ReferenceBrain を DLL 化・台本化へ発展）・`completed/areka-P0-ghost-setup`（`ShioriWiring` へ第3変種追加）。
- **Adjacent**: `areka-P0-emo2-conformance-e2e`（spine 裁定は不侵・置換せず併存）／`shiori-host32-testdll`（32bit 側の兄弟 fixture・そのまま残置）。

## Constraints

- Rust 2024・新規依存なし（windows/windows-core は既存承認済み）・tokio 不使用。
- x64（＋arm64 ビルド可能性維持・[[areka-multiarch-x64-arm64-i686-helper]]）。i686 は本 spec 非関与。
- COM エクスポートは `extern "system"`（[[windows-com-export-calling-convention]]）。
- 決定論 e2e は sleep 不使用・注入入力のみ（[[deterministic-test-coverage-mandate]]・[[prefer-x64-fake-boundary-tests-not-x86]] と同思想の「偽装境界」を実 DLL 境界まで前進させるもの）。
- 正典は ukadoc（[[ukadoc-mcp-preferred-source]]）・SHIORI4/IShiori の内部規約は `crates/shiori-abi` rustdoc＋completed shiori 系 spec が正本。
