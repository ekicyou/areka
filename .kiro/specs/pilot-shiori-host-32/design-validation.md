# 設計検証レポート: pilot-shiori-host-32

> 種別: **先進坑（pilot・使い捨て）**。本レポートは確定済み design.md の実装準備性を非対話で品質レビューしたもの。
> レビュー観点正本: `.claude/skills/kiro-validate-design/rules/design-review.md`。
> 規律正本: `.kiro/steering/two-tunnel.md`／設計判断正本: `doc/COMPAT_ARCHITECTURE.md §5`／go 基準宿主: `.kiro/steering/roadmap.md`。
> 検証日: 2026-06-30。判定: **GO**。

## 1. レビューサマリ

design.md は WM_COPYDATA 一本化という IPC 中核判断を確定し、6 コンポーネント＋README3Act の最小構成へ責務を綺麗に分離した、**先進坑の使い捨て品質に過不足なく見合う実装準備済みの設計**である。命綱（葉ノード隔離）は `crates/pilot` の構造（空 lib ＋ examples-only・inbound ゼロをコードベースで実検証済）で構造的に担保され、go 基準 (1)(2) はともに観測可能なフロー（1 往復シーケンス／状態遷移図）として落ちている。research.md の確定決定（WM_COPYDATA・byte proxy・SHIORI4/3 層・HGLOBAL 32bit ローカル・跨ビットネス規約・i686 ビルド実証・wintf-winmsg-executor 流用・emo2 fixture）はすべて設計へ忠実に反映されている。残る未知（`request` の正確な ABI・HGLOBAL 所有権の実挙動）は **go-gating でない実走 discovery** として明示的に README へ送られており、設計を papering-over していない。

## 2. クリティカルイシュー（最大 3・設計ディスカッションへの入力）

> いずれも GO を覆さない軽微な明確化要望。先進坑の使い捨て品質ゆえ「直さなくても実走可能」だが、設計ディスカッションで一言確定しておくと実装時の手戻りを減らせる。

### 🟡 Issue 1: `request` flat-C シグネチャの「想定」と go 基準 (1) 達成可否の連動が未明示

- **Concern**: `ShioriByteProxy` の `RequestFn = unsafe extern "C" fn(h: HGLOBAL, len: *mut usize) -> HGLOBAL`（design.md「ShioriByteProxy / Service Interface」）は COMPAT §5 の像に基づく**想定**であり、design.md 自身が「実物 DLL で実走確認・実走で是正」と注記している。SHIORI/3.0 の歴史的な flat-C 規約は `request(long h, long *len)` で h は `HGLOBAL`（=`GlobalAlloc` ハンドル）を `long` へ載せる形が一般的だが、emo2 `pasta.dll` の実 export がこの並びか否かは未確認（research.md §6 は「装飾なし＝cdecl」までは確認済、引数 ABI は未確認）。
- **Impact**: このシグネチャが実 DLL と食い違うと go 基準 (1) の 1 往復そのものが成立しない（`Value:` 受領に到達しない）。go-gating な未知が「実走で是正」一語に畳まれている。
- **Suggestion**: 設計ディスカッションで「①このシグネチャ不一致は go 基準 (1) の達成可否そのものを左右する go-gating 未知である ②不一致時の是正手順（PE export の引数解析 or 既知の SHIORI3 flat-C 規約 `HGLOBAL load(HGLOBAL,long)` / `HGLOBAL request(HGLOBAL,long*)` への合わせ込み）を README『検証結果』幕で記録する」ことを一言確定する。設計変更は不要、位置づけの明示で足りる。
- **Traceability**: 要件 3.2, 3.3, 4.2, 4.5。
- **Evidence**: design.md「ShioriByteProxy ＞ Service Interface / Implementation Notes」（`type RequestFn ...`・「本シグネチャは COMPAT §5 の像に基づく想定で、実走で是正する」）。

### 🟡 Issue 2: 親側「再入受領」（2nd WM_COPYDATA を SendMessage 待機中に受ける）の実現方式が抽象のまま

- **Concern**: go 基準 (1) の往復は「親が `SendMessage(helper, REQUEST)` で待機中に、helper からの応答 2nd WM_COPYDATA を**再入受領**する」設計（design.md「フロー上の判断 ＞ 再入受領」「IpcChannel ＞ Invariants: SendMessage 専用」）。WM_COPYDATA は OS が sent message を WndProc へ同期配送するが、親が `SendMessageTimeout` でブロック中に**自分の** WndProc が再入呼び出しされる挙動（DispatchMessage を回さずとも sent message は配送される）に依存する。この再入の正否は実走で初確認の領域であり、`IpcChannel::send_request` が「応答 Vec<u8> を返す」までの内部機序（待機中に WndProc が応答をどこへ積み、send_request がそれをどう回収するか）が設計上ブラックボックス。
- **Impact**: 再入受領が想定どおり配送されない／応答の受け渡し（WndProc → send_request 呼び出し元）が噛み合わないと、go 基準 (1) がタイムアウトで失敗し得る。これは Revalidation Trigger にも挙がる急所（design.md「WM_COPYDATA 往復が i686↔x64 で成立しない」）。
- **Suggestion**: 設計ディスカッションで「応答の受け渡し（WndProc が 2nd WM_COPYDATA payload を thread-local／共有セルへ格納 → send_request がタイムアウトループで回収、等）の最小機序」を一行スケッチで確定するか、または「成立しない場合 named pipe へ後退」という既定の退路（既に Revalidation Trigger 化済）を README 一次記録の対象として明示確認する。先進坑ゆえ機序は実装裁量でよいが、go-gating な再入依存である点を README に残す前提を確定したい。
- **Traceability**: 要件 2.2, 2.3, 4.3。
- **Evidence**: design.md「System Flows ＞ go 基準 (1) ＞ フロー上の判断 ＞ 再入受領」「Components ＞ IpcChannel ＞ Invariants」。

### 🟡 Issue 3: helper を 2 本目バイナリにする Cargo 宣言と「親が helper exe パスを解決する」段取りが実装裁量に委ねられたまま

- **Concern**: design.md は helper を「`examples/` の 2 本目バイナリにする具体（別 `[[example]]` か `src/bin` 相当か）は実装時に確定」とし、`ProcessHost::spawn(helper_exe: &Path, ...)` の `helper_exe` パス解決も「親 cwd 相対 or 環境変数・先進坑ゆえ手動指定で可」と裁量化している。i686 helper は**別ターゲットビルド**（`cargo build --target i686-pc-windows-msvc`）の別成果物ゆえ、x64 親の `cargo run` 成果物ディレクトリとはツリーが分かれ、親からのパス解決が「手動指定」前提になる。
- **Impact**: 機能的リスクは低い（先進坑ゆえ手動 OK）が、README「実行法」幕（要件 6.1）に **2 段ビルドの具体コマンドと helper exe パスの渡し方**が書かれないと、go 検証の再現（開発者が実走して go 判定を下す）が滞る。再現性は二坑規律で fixture 取り込みまでして守った要件であり、ここで実行法が曖昧だと片手落ち。
- **Suggestion**: 設計変更は不要。設計ディスカッションで「README『実行法』幕に ①helper の i686 ビルドコマンド ②生成 exe の実パス ③親へ渡す手段（env `HELPER_EXE` or 第1引数）を必ず明記する」ことを確定する（design.md の File Structure Plan 注記を README 必須項目へ格上げ）。
- **Traceability**: 要件 1.5, 6.1, 7.5。
- **Evidence**: design.md「File Structure Plan ＞ モジュール物理配置の注」「ProcessHost ＞ Implementation Notes / Risks（helper exe パス解決）」「Modified Files ＞ Cargo.toml」。

## 3. 設計の強み（1–2）

### ✅ Strength 1: 命綱（葉ノード隔離）が構造で担保され、コードベースで実検証できた

design.md が依拠する「`crates/pilot` 空 lib ＋ examples-only ゆえ inbound 構造的にゼロ」（two-tunnel・要件 7.2）は机上論でなく実在する。検証時に全 `Cargo.toml` を走査した結果、`pilot` を依存に挙げる production クレートは皆無（`pilot/Cargo.toml` の `name = "pilot"` 以外ヒットなし）で、命綱が現に成立している。これは先進坑の唯一の非交渉不変条件であり、設計がこれを最優先で守り切っている点は決定的に良い。

### ✅ Strength 2: research.md の確定決定と go 基準が設計へ 1:1 で落ち、未知の扱いが誠実

WM_COPYDATA 一本化・helper=byte proxy／x64=Shiori3Codec・HGLOBAL は IPC を跨がない（32bit ローカル）・dwData タグは低 32bit のみ・HWND は u32 LE・emo2 fixture（実在を実検証: `pasta.dll` 3.46MB＋`descript.txt` を `ghost/master/` に確認）——research.md §3.1.1/§5.4/§6 の確定事項がすべて設計の Components/Contracts/Flows へ忠実に反映されている。さらに go-gating でない未知（`request` ABI 等）を「設計で詰め切らず実走で確定し README に記録」と**明示的に先送り**しており、先進坑の成果物（知見）の性質に整合した誠実な設計判断になっている。

## 4. 最終判定

- **判定: GO**
- **根拠**: 命綱（葉ノード隔離）が構造＋実検証で成立し、go 基準 (1)(2) がともに観測可能なフローとして定義され、research.md の確定決定が忠実に反映され、未知の扱いも誠実。先進坑の使い捨て品質に対し過剰な production 級複雑性（COM 配線・named pipe・共有メモリ・reader スレッド・全 SHIORI イベント）はすべて適切に削除されており、over-engineering なし。3 件のイシューはいずれも go を覆さない軽微な明確化要望で、設計ディスカッションでの一言確定で足りる。
- **次ステップ**: 設計ディスカッション（`/kiro-design` 内）で Issue 1–3 を確認 → `/kiro-spec-tasks pilot-shiori-host-32` でタスク生成へ。

---

> 本レポートは情報提供であり最終判定（go）の代替ではない。先進坑の go 判定は開発者の人間判断（要件 6.5・two-tunnel ハードゲート）。
