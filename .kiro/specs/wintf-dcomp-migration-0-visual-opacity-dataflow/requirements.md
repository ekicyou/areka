# Requirements Document

## Project Description (Input)
DComp → Layered Window 移行における透明度データフローの設計ギャップを解決する Phase 0 子仕様。

現行パイプラインでは Widget が `Opacity` コンポーネント (metrics.rs) を設定し、`visual_property_sync_system` が `IDCompositionVisual3::SetOpacity()` を呼び出す。しかし `Visual.opacity` フィールドは存在するが、Widget から書き込まれておらず、プロダクションコードで未使用（0 usages）。

D2D1 合成方式（Phase 1 以降）では `composite_render_system` が `Visual.opacity` を読んで階層累積描画を行う設計だが、Widget → `Visual.opacity` への書き込みパスが存在しないため、opacity が常にデフォルト 1.0 になる。

本仕様は、Widget 層から `Visual.opacity` / `Visual.is_visible` へのデータフローを確立し、`Opacity` コンポーネント (metrics.rs) の廃止方針を策定する。

## Requirements
<!-- Will be generated in /kiro-spec-requirements phase -->
