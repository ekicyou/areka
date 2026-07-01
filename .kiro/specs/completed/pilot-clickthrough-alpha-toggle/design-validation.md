# 設計検証レポート: pilot-clickthrough-alpha-toggle（先進坑 / pilot）

> 本レポートは kiro-validate-design による技術設計の品質レビュー（非対話・GO/NO-GO 判定）。
> 検証対象: design.md（**視覚的透過機構を DirectComposition (DComp) visual tree に再確定した改訂版**）。
> 再検証理由: 設計ディスカッションの決定により視覚的透過機構が **DWM extend-frame glass → DirectComposition** に変更された。旧 design-validation.md（DWM glass 版）は陳腐化したため本レポートで全面置換する。
> 特別焦点: DComp 再確定が (a) ex_style 規約、(b) DComp 描画パイプライン、(c) クリック透過のトグル単独制御（汚染回避）、(d) 新規依存なし、(e) 核心 Unknown の温存、(f) R1〜R10 整合 を満たすか。

## レビューサマリ

本設計は既存先進坑 `wintf-winmsg-executor` の確立パターン（`Window::new_ex`・wndproc クロージャ・`block_on`/`spawn_local`・`event_listener` 起床・`AtomicBool` done）を忠実に踏襲した最小検証台であり、責務分離（ワーカ判定／UI 適用）・差替シーム・座標手順・状態変化最適化が具体的契約（Rust シグネチャ・state model・API 呼出列）まで落ちている。視覚的透過機構は **DirectComposition visual tree** として、その採用が「好み」ではなく `WS_EX_NOREDIRECTIONBITMAP` 窓における**唯一のピクセル供給経路（強制）**である理由（redirection surface 不在ゆえ GDI/DWM glass は inoperative）まで含めて具体 API・呼出列・描画規約が確定されている。当たり判定は `WS_EX_TRANSPARENT` トグル単独に分離され、核心 Unknown（プロセス越えクリック透過）は実機 go ゲートとして正しく温存されている。実装着手に十分な品質である。

## 特別焦点の検証（DComp 再確定）

| 観点 | 判定 | 根拠 |
|------|------|------|
| (a) ex_style = `WS_EX_NOREDIRECTIONBITMAP \| WS_EX_TOPMOST \| WS_EX_TRANSPARENT`、動的トグルは `WS_EX_TRANSPARENT` のみ加除し他 2 ビットを保存 | ✅ | design.md「起動時初期状態確定」(L203)・TransparentWindow Responsibilities(L330)・StateApplier(L308) に明記。`new_ex` 生成時付与＋「`new_ex` は `WS_EX_NOREDIRECTIONBITMAP` と `WS_EX_TOPMOST` を保存したまま `WS_EX_TRANSPARENT` ビットだけを反転」と具体化。 |
| (b) 視覚的透過 = DComp per-pixel α（D3D11→DXGI `CreateSwapChainForComposition`→`DCompositionDevice`/`CreateTargetForHwnd`/Visual/Commit、D2D `Clear(transparent)`＋`FillEllipse` 不透明、`WM_LBUTTONDOWN` で再描画）。GDI/WM_PAINT/DWM glass を prescriptive 設計から除去 | ✅ | 「視覚的透過方式（決定済み・DComp）」節(L103-129) にパイプライン 5 段＋D2D 描画＋再描画トリガを明記。`WM_PAINT`/GDI/`InvalidateRect` は「もはや描画経路ではない」(L120,L334)。残存する GDI/glass 言及はすべて否定・棄却・歴史注記のみ（L62,65,120,125,350 を grep 確認、prescriptive 記述なし）。 |
| (c) ヒットテストは全矩形維持 ⇒ クリック透過は `WS_EX_TRANSPARENT` トグル単独制御（DComp 透過は自動でクリックを透過しない＝汚染なし）。`WM_NCHITTEST` 不介入(R2.4) | ✅ | 「DComp 視覚αは純粋に見た目だけの効果でありヒットテストはスタイル基準で全矩形のまま」「クリック透過は `WS_EX_TRANSPARENT` 動的トグル単独でのみ制御」(L122)。`WM_NCHITTEST` は「分岐に書かない＝自前ハンドルしない(R2.4)」(L62,L333)。layered+colorkey/SetWindowRgn は「自動クリック透過で汚染」ゆえ棄却(L126-128)。 |
| (d) 新規依存/feature なし（Direct3D11/Dxgi(+Common)/DirectComposition/Direct2D(+Common)/Direct3D は workspace `windows` 0.62.2 で有効済み） | ✅ | workspace `Cargo.toml` L61-67 を実機確認: `Win32_Graphics_Direct2D_Common`・`Direct3D`・`Direct3D11`・`DirectComposition`・`Dxgi_Common` すべて有効。design.md(L46,129,144)・research.md Decision 5 とも「新規依存・新規 feature 不要」「`DwmExtendFrameIntoClientArea`/`MARGINS` 不使用」で整合。 |
| (e) 核心 Unknown（`WS_EX_TRANSPARENT` 単独のプロセス越えクリック透過・T2/T6）が、実物の DComp/`WS_EX_NOREDIRECTIONBITMAP`（production 等価）窓のもとで go ゲートとして温存され誤って「解決済み」とされていない | ✅ | Open Questions「核心 Unknown」(L431)・Implementation Notes Validation(L349) で「`WS_EX_LAYERED` 不付与での別プロセス・クリック透過の成否は T2/T6 実機検証（核心 Unknown）。今回は実物の DComp/`WS_EX_NOREDIRECTIONBITMAP` 窓（production 等価）で検証」と明記。「視覚的透過は DComp で決定済みだがクリック透過は実機 go ゲート本体」「DComp visual はヒットテストに無影響ゆえ核心 Unknown の純度を保つ」と分離が一貫。 |
| (f) R1〜R10 が DComp 再描画ベースでも整合（特に R2.1–2.5・R6.1–6.3 が GDI でなく DComp 再描画で充足） | ✅ | Traceability 表(L223 R2 行・L227 R6 行) が DComp 経路で記述。R6.1/6.2/6.3 は「`WM_LBUTTONDOWN` 受領（トグル OFF 時は全矩形ヒットテスト）→ログ→DComp 色再描画（D2D 再描画＋Present＋Commit）」(L120,L335)で充足。R2.2 描画円＝判定円一致(L332)、R2.3 layered 不付与(L331)、R2.4 NCHITTEST 不介入(L333)、R2.5 跨ぎ共有は生値＋読み取り専用に narrowing(L295) で維持。 |

特別焦点 (a)〜(f) すべて充足。DWM glass からの機構変更が、(c) 検証台の汚染回避と (e) 核心 Unknown の純度を保ったまま反映されており、再検証の目的（陳腐化レポートの是正）は達成されている。design.md に prescriptive な DWM-glass 内容の残置はなく（棄却理由・歴史注記のみ許容範囲で残存）、検証対象窓は production 等価（DComp/NOREDIRECTIONBITMAP）へ正しく格上げされている。

## クリティカル issue（最大 3）

実装可否を左右するクリティカル issue は検出されなかった。以下は GO を妨げない軽微な確認事項であり、参考として記す（issue ではない）:

- **起動順序の二択**: 初回 notify 取りこぼし防止として design.md「起動時初期状態確定」(L205) が listen-then-spawn と「UI 起動直後の初回ポーリング」を代替併記し実装者裁量に委ねている。先進坑の軽微判断として許容範囲だが、採用方式をログ出力すると「起動時カーソルが円内」の稀ケースで T3/T4 観測の再現性が上がる。
- **トグル色と α の整合**: R6.3 の色トグルは DComp では `FillEllipse(α=1)` で不透明描画されるため、旧 DWM glass 版にあった「純黒へ遷移すると円が消える」事故は機構上発生しない（α=0 の `Clear` 領域のみが透過）。設計は手当て済みで、実装時は再描画後の Present/Commit 漏れがないことだけ留意すればよい。

## 設計の強み

- **検証台の純度設計が一貫**: 視覚的透過（DComp visual α・全矩形ヒットテスト維持）と当たり判定（`WS_EX_TRANSPARENT` トグル単独）を機構レベルで完全分離し、layered+colorkey/SetWindowRgn を「自動クリック透過による汚染」、DWM glass を「`WS_EX_NOREDIRECTIONBITMAP` 窓では inoperative」という一貫した基準で棄却。これにより検証台が本坑 `wintf-clickthrough-alpha-toggle` の production DirectComposition 経路を忠実に写像し、pilot の go 知見がそのまま移植可能になっている。
- **DComp 強制性の論証が明快**: 「`WS_EX_NOREDIRECTIONBITMAP` 窓は redirection surface を持たず GDI/DWM glass は画面に出ない（inoperative）ゆえ DComp が唯一の描画手段」という前提を `wintf-winmsg-executor` example の自己文書まで遡って論証(L105)。視覚機構を「runtime 観測に先送り」していた旧設計の punt 欠陥を解消し、機構決定と核心 Unknown を明確に切り分けている。

## 最終判定

**判定: GO**

**根拠**: 視覚的透過機構が DWM glass から DirectComposition visual tree へ正しく再確定され、ex_style 規約・DComp パイプライン・トグル単独のクリック制御・新規依存ゼロ・核心 Unknown の温存（実物 DComp/NOREDIRECTIONBITMAP 窓）・R1〜R10 整合（特別焦点 a〜f）がすべて充足。prescriptive な DWM-glass/GDI 内容の残置はなく、クリティカルな構造的不整合・要件欠落・過剰複雑性も認められない。許容可能なリスクで実装に進める。

**次ステップ**: タスクは既に生成・承認済み（spec.json: tasks.generated=true / tasks.approved=true / ready_for_implementation=true）。設計ディスカッションで軽微確認事項（起動順序の明示）を合意のうえ、`/kiro-impl pilot-clickthrough-alpha-toggle` で実装に進む。NO-GO 時の再設計は不要。
