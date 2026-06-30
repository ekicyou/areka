# 設計検証レポート: pilot-clickthrough-alpha-toggle（先進坑 / pilot）

> 本レポートは kiro-validate-design による技術設計の品質レビュー（非対話・GO/NO-GO 判定）。
> 検証対象: design.md（視覚的透過機構を DWM extend-frame glass に確定する再生成後の版）。
> 特別焦点: 直前に runtime 先送りだった「視覚的透過機構」が設計判断として確定されたか、かつトグル検証を汚染しないか。

## レビューサマリ

本設計は既存先進坑 `wintf-winmsg-executor` の確立パターンを忠実に踏襲した最小検証台であり、責務分離（ワーカ判定／UI 適用）・差替シーム・座標手順・状態変化最適化が具体的な契約（Rust シグネチャ・state model・API 呼出列）まで落ちている。最大の論点だった視覚的透過機構は **DWM extend-frame glass** として概念ではなく具体 API・塗り分け規約・呼出点まで確定され、当たり判定（クリック透過）を `WS_EX_TRANSPARENT` トグル単独に委ねる分離が明示されている。核心 Unknown（`WS_EX_TRANSPARENT` 単独でのプロセス越えクリック透過）は実機 go ゲートとして正しく温存されており、実装着手に十分な品質である。

## 特別焦点の検証（視覚的透過機構）

| 観点 | 判定 | 根拠 |
|------|------|------|
| (a) 機構が具体的に確定（runtime 先送りでない） | ✅ | design.md「視覚的透過方式（決定済み・R2.2/R2.3）」節＋ research.md Decision 5。`DwmExtendFrameIntoClientArea(hwnd, &MARGINS{-1,-1,-1,-1})` を窓セットアップ時に呼ぶと明記。塗り分け規約（黒→ガラス透過／非黒円→可視）と呼出点（TransparentWindow セットアップ）まで具体化。 |
| (b) トグル検証を汚染しない（視覚のみ・透明領域が自動でクリック透過しない） | ✅ | 「DWM ガラス透過は純粋に視覚効果であり窓は全矩形でヒットテストされ続ける」「クリック透過は `WS_EX_TRANSPARENT` トグル単独でのみ制御」と明示。layered+colorkey/SetWindowRgn は「自動クリック透過で汚染」ゆえ棄却と記載。R2.4（NCHITTEST 不介入）も維持。 |
| (c) `WS_EX_LAYERED` を導入しない（R2.3） | ✅ | 「`WS_EX_LAYERED` は付けない」と TransparentWindow Responsibilities・「視覚的透過方式」節の双方に明記。棄却代替の layered 案は R2.3 違反として明示排除。 |
| (d) 既有 feature 超の新規依存/feature なし（`Win32_Graphics_Dwm`＋`Win32_UI_Controls`） | ✅ | workspace `Cargo.toml` line 66（`Win32_Graphics_Dwm`）・line 77（`Win32_UI_Controls`）を実機確認、両者有効済み。design.md・research.md とも「新規依存・新規 feature 不要」と整合。 |
| (e) 核心 Unknown（T2/T6・プロセス越えクリック透過）が go ゲートとして温存され誤って「解決済み」とされていない | ✅ | Open Questions「核心 Unknown」＋ Implementation Notes Validation で「`WS_EX_LAYERED` 無しでの別プロセス・クリック透過の成否は T2/T6 実機検証（核心 Unknown）」と明記。「視覚的透過は決定済みだがクリック透過は実機 go ゲート本体」と分離が一貫。 |

特別焦点 5 項目すべて充足。視覚機構の確定が核心 Unknown の純度を保ったまま行われており、再生成の目的（runtime 先送り欠陥の解消）は達成されている。

## クリティカル issue（最大 3・設計ディスカッションへ送る）

クリティカル（実装可否を左右する）issue は検出されなかった。以下は GO を妨げない軽微な確認事項であり、参考として記す（issue ではない）:

- 起動順序の二択（listen-then-spawn ／ 初回ポーリング）が design.md「起動時初期状態確定」で代替併記のまま実装者裁量に委ねられている。先進坑の軽微判断として許容範囲だが、実装時にどちらを採るかログで明示すると T3 観測の再現性が上がる。
- 「黒＝透過」規約のため、不透明円のトグル色（R6.3）が偶然にも純黒へ遷移しないよう実装側で担保が要る（円が消えて見える事故防止）。design.md は「黒以外の単色」と制約済みゆえ設計上は手当て済み、実装注意に留まる。

## 設計の強み

- **検証台の純度設計が秀逸**: 視覚的透過（DWM ガラス・全矩形ヒットテスト維持）と当たり判定（`WS_EX_TRANSPARENT` トグル単独）を機構レベルで完全分離し、layered/colorkey/region を「自動クリック透過による汚染」という一貫した基準で棄却。本坑の DComp シナリオを忠実に写像しており、pilot の知見が本坑へそのまま移植可能。
- **既存パターンへの徹底した相乗り**: 窓生成・wndproc・block_on/spawn_local・event_listener 起床・AtomicBool done・GDI 描画をすべて既存 example から adopt し、新規 build をαマスク純関数 1 個＋状態差分トグルに限定。葉ノード隔離・依存追加ゼロ・32bit 可搬性を崩さず、要件 1/10 と完全整合。

## 最終判定

**判定: GO**

**根拠**: 全要件 R1〜R10 が traceability 表とコンポーネント契約に追跡可能で、再生成の目的だった視覚的透過機構が具体 API・塗り分け規約・呼出点まで確定され（特別焦点 a〜d 充足）、かつ核心 Unknown（T2/T6 のプロセス越えクリック透過）が誤解決されず実機 go ゲートとして温存されている（e 充足）。クリティカルな構造的不整合・要件欠落・過剰複雑性はなく、許容可能なリスクで実装に進める。

**次ステップ**: タスクは既に生成・承認済み（spec.json: tasks.generated/approved=true, ready_for_implementation=true）のため、設計ディスカッション（軽微確認事項の合意）を経て `/kiro-impl pilot-clickthrough-alpha-toggle` で実装に進む。NO-GO 時の再設計は不要。
