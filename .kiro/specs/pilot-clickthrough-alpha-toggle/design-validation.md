# 設計バリデーションレポート: pilot-clickthrough-alpha-toggle（先進坑 / pilot）

> 本書は kiro-validate-design による設計品質レビュー（非対話・実行 2026-06-30）。design.md / requirements.md / research.md / spec.json は確定済みのため変更しない。判定の成果物（go ゲートの本体＝`WS_EX_TRANSPARENT` 単独での別プロセス透過の成否）は実機 T2/T6 で検証する live-fire 項目であり、設計上の欠陥ではない（pilot の設計通り）。

## 設計レビューサマリ

本設計は既存先進坑 `wintf-winmsg-executor` の確立パターン（`Window::new_ex`／wndproc クロージャ／`block_on`＋`spawn_local`／`event_listener` 起床／`AtomicBool` done／GDI 描画）を最大流用し、新規結線を「αマスク純関数 1 個＋状態差分トグル」に絞った最小検証台として明快である。要件 R1〜R10 の全 numeric ID が Requirements Traceability 表とコンポーネントブロックに出現し、Boundary Commitments 4 節・File Structure・座標手順・REPORT/README 役割分担まで具体化されており、実装着手の準備は整っている。先進坑の品質基準は緩めてよい一方で命綱（葉ノード隔離）は厳守という線引きも明示されており、two-tunnel.md と完全整合する。GO。

## 重大な問題（最大 3 件）

本レビューでは GO を覆す重大な設計欠陥は検出しなかった。以下は設計ディスカッションに送る軽微な確認事項（いずれも NO-GO 要因ではなく、実装/検証時の明確化レベル）。

### 🟡 確認事項 1: 「変化時のみ notify」とポーリング 16ms 周期の境界往復チャタリング
- **Concern**: ワーカは 16ms 周期で判定し desired 変化時のみ notify する設計（System Flows）。カーソルが円境界線上を微小往復すると 16ms ごとに ON↔OFF が連発し、`SetWindowLongPtr`＋`SetWindowPos(SWP_FRAMECHANGED)` が高頻度で走る余地がある。
- **Impact**: T4（境界またぎログ）の観測がノイズになり、T5（非変化時非発火）の負の証跡を取りにくくする可能性。検証の読み取りやすさに影響する程度で、go 判定の本質（透過成否）には影響しない。
- **Suggestion**: 設計を変えず、検証時に「境界をゆっくり一度だけまたぐ」手順を REPORT 手順注記に加えるか、必要なら実装でヒステリシス（境界に薄い不感帯）を後付けする余地を残す旨を Open Questions に一文添える。
- **Traceability**: R5.3, R5.4, R9.2（T4/T5）
- **Evidence**: design.md「System Flows / 状態判定・適用フロー」、「Testing Strategy / T4・T5」

### 🟡 確認事項 2: R2.5（HWND `!Send` のスレッド跨ぎ共有許容）と採用方針 (a) の関係明示
- **Concern**: R2.5 は Win32 慣例に従い状態をスレッド跨ぎ共有してよいと「許容」するが、設計は Decision 1(a) で HWND を跨がず、ワーカは値コピーした HWND に対し読み取り専用 API（`GetCursorPos`/`GetWindowRect`）のみ呼ぶ方針を採る。
- **Impact**: 矛盾ではなく安全側への意図的な narrowing だが、「HWND 値コピーをワーカへ渡す」が `!Send` 制約・Win32 慣例とどう両立するか（読み取り専用 API はスレッド跨ぎ可という前提）の根拠が research.md/Decision 1 散在で、実装者が `unsafe impl Send` ラッパ要否で迷う余地がある。
- **Impact 補足**: go 判定には無関係。実装着手時の小さな曖昧さ。
- **Suggestion**: 設計ディスカッションで「ワーカへ渡すのは HWND の生値（`isize`/`HWND`）であり読み取り専用 API のみ・スタイル変更は UI 専有」を一文で確認し、必要なら Send ラッパの要否を明記。
- **Traceability**: R2.5, R3.2
- **Evidence**: design.md「CursorWorker / State Management（HWND はワーカ起動時に値コピー）」、research.md §3.1

### 🟡 確認事項 3: 初回フレーム notify と UI 側 listen 確立のレース（軽微）
- **Concern**: 起動時初期状態確定フロー（design.md）はワーカ初回判定で無条件 notify する。ワーカ起動が UI 側 `event.listen()` 確立より先行した場合、初回 notify を取りこぼし初回 OFF 適用が次の変化まで遅延する理論余地がある（`event_listener` は listen 前 notify を保持しないため）。
- **Impact**: 「起動時にカーソルが円内」の稀ケースで初回 OFF 適用が遅延し得る。T3（円内クリック受領）初回観測がぶれる可能性。go 判定本質には無影響。
- **Suggestion**: 設計ディスカッションで起動順序（UI の listen 確立後にワーカを spawn、または初回は UI 側でも起動直後に desired を 1 回ポーリングして適用）を一文確認。既存 example の done-notify パターンと同様の確実性を初回にも担保する。
- **Traceability**: R5.3, R5.4（起動時初期状態確定）
- **Evidence**: design.md「System Flows / 起動時初期状態確定（R5.3/R5.4）」

## 設計の強み

1. **流用と新規の切り分けが明快で可逆性が高い**: 全インフラを既存 example から adopt し、新規 build をαマスク純関数 1 個＋状態差分トグルに限定。単一 `main.rs` 集約・production/`pilot/lib.rs`/`pilot/Cargo.toml` 変更ゼロ・依存追加ゼロで、葉ノード隔離（two-tunnel.md 命綱）を構造的に厳守。いつでも安全に捨てられる先進坑の規律を完全に満たす。
2. **核心 Unknown の扱いが正しい**: `WS_EX_TRANSPARENT` 単独でのプロセス越え透過の成否を「設計で潰せない実機 go ゲート（T2/T6）」と明示し、推測で結論を出さず人間判定に委ねる（R9.6/R10.4）。座標系も PMv2 物理座標統一で DPI 変換不要と判断しつつ、ずれた場合の per-monitor 補助を Follow-up に残す堅実さ。REPORT（根拠台帳）と README 3 幕（結論正本）の役割分担も two-tunnel.md と整合。

## 最終判定

- **判定: GO**
- **根拠**: 全要件 ID をトレース済みで実装パスが明確、既存実証パターンへの整合性が高く、先進坑の唯一の硬性制約である葉ノード隔離を構造的に厳守。残る Unknown（透過成否・DPI 一致）は設計欠陥ではなく本 pilot が潰しに行く実機検証点そのものであり、設計フェーズで解消する種類の問題ではない。上記 3 件はいずれも軽微な明確化で、設計ディスカッションで一文ずつ確認すれば足りる。
- **次ステップ**: 設計ディスカッションで確認事項 1〜3 を解消 → `/kiro-spec-tasks pilot-clickthrough-alpha-toggle` でタスク生成 → 実装 → T1〜T8 を人間と手動検証（go 判定は人間が下す）。
