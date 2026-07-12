//! scope → 表示対象（TargetId）写像の正本（純関数・std のみ）。
//!
//! 本仕様が新たに正本として確立する唯一の契約（design.md「依存方向」の最左・R3.5）。
//! DD-3 の採番規約（シェル表示対象＝`2*scope`／バルーン表示対象＝`2*scope+1`）で
//! scope からシェル／バルーン両表示対象への互いに素な写像を与え、`ActorKey` の数値 parse
//! （非数値は `None`）を担う。
//!
//! 骨格のみ。`shell_target`／`balloon_target`／`scope_of` の実装は tasks.md task 2.1 が担う。
