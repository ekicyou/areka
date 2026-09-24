# 設計検証レポート: areka-P0-baseware-root-layout

> 検証日 2026-09-24・ブランチ `claude/areka-p0-baseware-root-layout-6ae6de`（`baaa3a2f` の design.md を対象）。
> 入力: spec.json・requirements.md・design.md・research.md（§6.1 の議題処理表と §8〜§13）・brief.md・steering。
> 設計が「既存コードはこうなっている」と述べた箇所は、すべて実物のコード（関数名・型名で指す）と突き合わせた。要件 9 の裁定 1〜5 は確定事項として扱い、設計がそれを守っているかだけを見た。

## 検証の要約

設計は research.md の Option C（列挙と鍵は下層 crate・判断と告知は bin の新ファイル・永続は既存の族追加）をそのまま採り、責務の切り方（列挙は記憶を知らない／判断は I/O を持たない／配線は `main` だけ）が明快で、要件 1〜9 の全項目が Traceability 表で部品と関数に結び付いている。設計が根拠にした既存コードの主張は、以下の実物と一致した: `fn main` の順序と `insert_persist_wiring` が wired／fallback の両アームにあること、`GhostRuntime::shutdown` の手順 10 が `barrier()` → `close()` → join であること、`ScopeRoots.ghost` が `profile_areka_root(&mount.shiori.dir)` で据えられ `ShioriMount.dir` が `<ゴースト>/ghost/master` であること、`ShellMount.dir` が `<ゴースト>/shell/<名>` であること、`PersistKey` が `Copy` であること、`load_scope(scope, &ScopeRoots, &dyn PersistIo)` の形、`FakePersistIo`／`spawn_sylphya` が既存テストで使われていること、`emo2.nar` の `install.txt` が `directory,emo2`・`balloon.directory,emo2-kakukaku` であること、`temp-path-kit`／`sample-ghost-kit`／`log-capture-kit` が 3 crate の dev-dependencies に登記済みであること、`Win32_UI_WindowsAndMessaging` がワークスペース依存に含まれること、ダミー窓に依存する 5＋2 本のテストと既定パスの 4 本の所在。裁定 1〜5 はいずれも設計に反映されており、覆した箇所は無い。残る指摘は文書内の整合に関する 2 点で、構造の作り直しを要するものは無い。

## 重要な指摘（最大 3 件）

### 指摘 1: 台帳の更新範囲（R7）の前提がゴースト `name` について実物と違う

- **問題**: 設計判断 R7 は「既に実装済みの欄（ゴースト `name`・`readme`、シェル `name`）は status と owner を動かさず note に追記するだけ」と書く。しかし `doc/ukadoc-coverage/ledger/assets.toml` のゴースト `name` の項目（`entry."ukadoc:descript_ghost:name_…"`）は `status = "vocabulary-only"`・`owner = ""` で、note にも「この欄を読む本番の経路が 1 つも無い」とある。実装済みなのはゴースト `readme` とシェル `name` だけである。
- **影響**: 本仕様の `catalog::list_ghosts` はゴーストの `name` を本番で読む初めての経路になる。R7 の指示どおりに status を据え置くと、実装後に台帳が「語彙のみ」のまま古くなり、記憶 doc-claims-need-file-line-verification（doc の主張は実物で裏取り）に反する。R7 の「ゴーストで動かす欄」の列挙（`craftman`／`craftmanw`／`id`）から `name` が漏れているのも同じ原因。
- **提案**: R7 の表を「ゴースト `name` は `vocabulary-only` → `implemented`・owner を本仕様へ」と改め、据え置きの列挙をゴースト `readme`・シェル `name` の 2 欄に直す。実装タスクではこの行を根拠に台帳を更新する。
- **要件**: 2.4・2.9（設計判断 R7）。
- **根拠の場所**: design.md「設計判断（確定）」R7 の行／`assets.toml` のゴースト `name` の項目の status・owner・note。

### 指摘 2: `resolve_root_from` の戻り値の型が design.md の中で 2 通りに書かれている

- **問題**: 「File Structure Plan → Modified Files（要点）」の `boot_config.rs` の項は `resolve_root_from(env, exe) -> Result<PathBuf, RootError>` と書き、「Components and Interfaces → `boot_config` の根の解決 → Service Interface」と `main` の Integration は `Result<(PathBuf, RootSource), RootError>` と書く。
- **影響**: 要件 1.4 の告知と `error!` には根の出所（環境変数か exe の隣か）を載せる設計になっており（`RootError::NotADirectory { source }`・`info!(event = "root_resolved", root, source)`）、`RootSource` を返す側が正しい。タスク生成が Modified Files の要約だけを読むと出所の無い口を作り、告知の文面と `main_config_input_tests.rs` の 5 通りが設計と食い違う。
- **提案**: Modified Files の 1 行を Service Interface の署名に揃える（`Result<(PathBuf, RootSource), RootError>`）。
- **要件**: 1.2〜1.4・8.2。
- **根拠の場所**: design.md「Modified Files（要点）」の `boot_config.rs` の項／同「`boot_config` の根の解決」の Service Interface・Implementation Notes。

（3 件目に相当する構造上の問題は見つからなかった。）

## 軽微な指摘（重要ではないが実装時に見ておく）

- `tests/emo2_real_run.rs`（`AREKA_EMO2_REAL_RUN` で点く開発者向けの実走テスト）は argv を 2 つ渡すので根・ゴースト・バルーンの告知は出ないが、モニタ 0 台では新しい「起動窓を開けない」の告知が出る。今日もその環境では目印の assert で落ちるので見え方は変わらないが、`AREKA_NO_ALERT=1` を渡しておくと見張り（60 秒）まで待たずに落ちる。設計の smoke 3 方向は既に全方向で `AREKA_NO_ALERT=1` を渡している。
- 「Allowed Dependencies」の依存方向の行は `areka-parsers → areka-sylphya → areka-ghost::catalog → …` と鎖で書かれているが、本文どおり `catalog` は `areka-sylphya` に依存しない。読み手が誤解しないよう「import は左から右へのみ」の注釈を「`catalog` は parsers だけを見る」に補うとよい。

## 設計の強み

1. **判断の純粋化が徹底している。** `resolve_ghost`／`resolve_balloon` は argv・記憶の値・列挙のフォルダ名・添字を返す関数だけから決まり、無作為は「候補数 → 添字」の注入で固定できる。ゴースト 6 分岐・バルーン 7 分岐＋「指す先が無い」3 通りを 1 ファイルの決定論テストで踏め、列挙（`catalog`）は記憶も解決順も知らない。steering の「判断分岐だけをテストし配線は再テストしない」に沿う。
2. **既存コードの前提が実物と一致している。** 記憶の書き込みを wired／fallback 両アームの `insert_persist_wiring` と同じ場所に置く判断、反映の保証を `shutdown` の手順 10 に任せる判断（R1）、起動前の Ghost スコープの直読みの根を `profile_areka_root(<ゴースト>/ghost/master)` にする判断（裁定 4）は、いずれも `main.rs`・`runtime.rs`・`resolve.rs` の定義と突き合わせて正しかった。既定の定数 `emo2`／`StayseeBalloon` も検体の `install.txt`・展開フォルダ名と一致する。

## 最終判定

**GO**

- **理由**: 既存の境界（`fn resolve` の 4 呼び手・`runtime.rs`・`menu/`・永続の 4 族）を侵さず、裁定 1〜5 を守り、要件の全項目に部品と検証（決定論テスト 5 群＋smoke 3 方向＋実機 4 点）が対応している。指摘 2 件はいずれも文書内の整合であり、設計を組み直す必要は無い。
- **次の段取り**: 設計ディスカッションで指摘 1・2 を design.md に反映（R7 の行と Modified Files の 1 行）してから `/kiro-spec-tasks areka-P0-baseware-root-layout` へ進む。実装の段取りは design.md「実装の段取り」のとおり 1（テスト移設）→ 2〜6（並走可）→ 7（合流）→ 8〜10 でよい。
