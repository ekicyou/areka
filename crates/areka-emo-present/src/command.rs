//! 提示段の指令 API 契約正本（`PresentCommand`）と構造化エラー（`PresentError`）。
//!
//! 本モジュールは emo-present が下流 seriko-engine へ公開する**指令 API の契約正本**である。
//! surface 切替・非表示（`\s[-1]` 相当）・キャッシュ無効化を運ぶ指令の形を定義し、それらを
//! メッセージ enum（将来の `EmoMsg` 級）の variant へそのまま転写できる `Send + 'static` 所有形
//! （`areka-actor` の envelope 規約準拠）として与える。合成の実体や表示リソースには触れない
//! 純粋な契約層であり、wintf にも依存しない（依存方向: `emo-compose → emo-present`）。
//!
//! # 責務分界（呼び手＝seriko 側の解決責務）
//!
//! - `\s[-1]` → [`PresentCommand::Hide`] への写像は**呼び手（seriko）の責務**である。本 API は
//!   `surface_id: u32` をそのまま受け、`-1` 番兵を導入しない（非表示は専用 variant で表す）。
//! - `\s[エイリアス]`（alias/name 文字列）→ `u32` の解決も呼び手側（surface 状態所有者）の責務。
//!   本 API は解決済み `u32` のみを受け取る。
//! - `binds` は解決済みの有効 bind 集合（[`areka_emo_compose::BindSet`]）であること。bind 状態の
//!   所有・切替は seriko の責務であり、本段はスナップショットを受け取って合成へ渡すだけである。

/// 表示ターゲット識別子（0=sakura シェル・1=バルーン等。結線側が採番する不透明 id）。
///
/// 値の意味づけ（どの番号がどの窓か）は結線側の採番規約に属し、本段はこれを不透明キーとして
/// target ごとの状態（キャッシュ・表示面）へ引き当てるためだけに用いる。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetId(pub u32);

/// 指令適用の結果（成功は `()`・失敗は [`PresentError`]）。
///
/// request/reply が要る指令では [`areka_actor::ReplySender<PresentOutcome>`] を variant に同梱し、
/// 受信側（提示段）がこれ 1 本へ高々 1 回応答する（envelope 規約・oneshot 相当）。
pub type PresentOutcome = Result<(), PresentError>;

/// 提示段への表示指令（契約正本・`Send + 'static` 所有データ）。
///
/// 借用を一切持たない所有形ゆえ、将来のメッセージ enum（`EmoMsg` 級）の variant へそのまま転写
/// できる（areka-actor envelope 規約準拠）。各 variant の `reply` は任意（`None` は撃ちっぱなし・
/// `Some` は結果を要求する）で、応答は同梱 [`areka_actor::ReplySender`] へ 1 回だけ返る。
///
/// `#[non_exhaustive]` ゆえ将来 variant を追加しても下流の `match` は網羅性エラーにならない。
#[non_exhaustive]
pub enum PresentCommand {
    /// surface 切替（キャッシュ or 合成 → 表示＋マスク対入替）。
    ///
    /// `surface_id` は解決済みの `u32`（alias/`-1` 解決は呼び手責務）。`binds` は解決済み有効集合。
    ShowSurface {
        /// 適用先ターゲット。
        target: TargetId,
        /// 表示する surface id（解決済み・`u32`）。
        surface_id: u32,
        /// 有効 bind 集合（解決済みスナップショット・bind 状態所有は seriko）。
        binds: areka_emo_compose::BindSet,
        /// pattern 進行状態（現在コマ集合・合成入力の第一級要素・R5.1/5.2）。
        ///
        /// `binds` と同格の合成入力キー要素であり、seriko の pattern タイムライン評価が生産した
        /// スナップショットを値で運ぶ（envelope 規約準拠の `Send + 'static` 所有）。空
        /// （[`areka_emo_compose::PatternState::default`]）は pattern 寄与なし＝拡張前と観測等価（R5.4）。
        pattern: areka_emo_compose::PatternState,
        /// 結果返信端（任意・`Some` のとき高々 1 回応答する）。
        reply: Option<areka_actor::ReplySender<PresentOutcome>>,
    },
    /// 非表示（`\s[-1]` 相当）。visual 非表示＋当たり判定停止。キャッシュは保持する。
    Hide {
        /// 適用先ターゲット。
        target: TargetId,
        /// 結果返信端（任意・`Some` のとき高々 1 回応答する）。
        reply: Option<areka_actor::ReplySender<PresentOutcome>>,
    },
    /// 合成キャッシュ全破棄（アトラス再構築・ghost 再読込用の口）。
    InvalidateCache {
        /// 適用先ターゲット。
        target: TargetId,
        /// 結果返信端（任意・`Some` のとき高々 1 回応答する）。
        reply: Option<areka_actor::ReplySender<PresentOutcome>>,
    },
}

/// 提示段の指令適用で観測し得る構造化エラー（失敗経路はログ＋`Err` で表現する）。
///
/// - [`Compose`](PresentError::Compose): 上流合成の失敗を [`areka_emo_compose::ComposeError`] から
///   透過（`#[from]`）する。
/// - [`TargetNotAttached`](PresentError::TargetNotAttached): 指定 [`TargetId`] が未装着。
/// - [`Device`](PresentError::Device): グラフィックスデバイス層の失敗（`HRESULT` ＋文脈）。
///
/// `#[non_exhaustive]` ゆえ将来 variant を追加しても下流の `match` は網羅性エラーにならない。
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PresentError {
    /// 上流合成の失敗（[`areka_emo_compose::ComposeError`] を透過）。
    #[error("compose failed: {0}")]
    Compose(#[from] areka_emo_compose::ComposeError),
    /// 指定ターゲットが未装着（表示面・キャッシュが結線されていない）。
    #[error("target {0:?} is not attached")]
    TargetNotAttached(TargetId),
    /// グラフィックスデバイス層の失敗（`HRESULT` ＋発生文脈の静的説明）。
    #[error("graphics device error: {hresult:#x} {context}")]
    Device {
        /// 失敗した DXGI/D2D 呼び出しの `HRESULT`。
        hresult: i32,
        /// 発生箇所を示す静的文脈文字列。
        context: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_compose::{BindSet, ComposeError, PatternState};

    /// 型 `T` が `Send + 'static` であることをコンパイル時に要求するヘルパ。
    fn _assert_send_static<T: Send + 'static>() {}

    /// Req 3.1/7.2 不変条件: `PresentCommand` が `Send + 'static` 所有であることを静的に固定する。
    ///
    /// 借用フィールドの混入や `!Send` な内部型の導入でこのテストのコンパイルが失敗し、メッセージ
    /// enum の variant へ転写可能である envelope 規約の逸脱を檻に入れる。
    #[test]
    fn present_command_is_send_static() {
        _assert_send_static::<PresentCommand>();
        _assert_send_static::<PresentError>();
        _assert_send_static::<TargetId>();
    }

    /// Req 3.3/3.5/3.6: 各 variant を実データで構築できる（reply=None の撃ちっぱなし形）。
    ///
    /// `ShowSurface` は実 [`BindSet`] を同梱し、`Hide`（`\s[-1]` 相当）・`InvalidateCache` と併せて
    /// 3 経路すべてが所有データのみで組み立てられることを確認する。
    #[test]
    fn variants_construct_with_owned_data() {
        let target = TargetId(0);

        let show = PresentCommand::ShowSurface {
            target,
            surface_id: 1000,
            binds: BindSet::from_ids([0u32, 5, 5, 3]),
            pattern: PatternState::default(),
            reply: None,
        };
        let hide = PresentCommand::Hide {
            target: TargetId(1),
            reply: None,
        };
        let invalidate = PresentCommand::InvalidateCache {
            target,
            reply: None,
        };

        // variant の識別を match で確認（構築が成立し所有されていること）。
        assert!(matches!(
            show,
            PresentCommand::ShowSurface {
                surface_id: 1000,
                ..
            }
        ));
        assert!(matches!(
            hide,
            PresentCommand::Hide {
                target: TargetId(1),
                ..
            }
        ));
        assert!(matches!(
            invalidate,
            PresentCommand::InvalidateCache {
                target: TargetId(0),
                ..
            }
        ));
    }

    /// TargetId は `Copy` かつ `Hash` キーとして扱える（`Debug` 整形も `Device`/`TargetNotAttached`
    /// の表示経路で用いる）。
    #[test]
    fn target_id_is_copy_and_hashable_key() {
        use std::collections::HashSet;
        let a = TargetId(7);
        let b = a; // Copy
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert_eq!(format!("{a:?}"), "TargetId(7)");
    }

    /// Req 4.3/7.2: `PresentError` は `ComposeError` を `#[from]` で透過取り込みできる。
    ///
    /// 上流合成失敗をそのまま `?` で持ち上げられることを、`.into()` 変換で静的・実行時に確認する。
    #[test]
    fn present_error_maps_compose_error_via_from() {
        let ce = ComposeError::SurfaceNotFound(1000);
        let pe: PresentError = ce.into();
        assert!(matches!(
            pe,
            PresentError::Compose(ComposeError::SurfaceNotFound(1000))
        ));

        // EmptyComposition も同経路で写像される（Error Categories 双方）。
        let pe2: PresentError = ComposeError::EmptyComposition(42).into();
        assert!(matches!(
            pe2,
            PresentError::Compose(ComposeError::EmptyComposition(42))
        ));
    }

    /// `Device` variant の `Display` は HRESULT を 16 進で整形する（`{hresult:#x}`）。
    #[test]
    fn device_error_displays_hex_hresult() {
        let err = PresentError::Device {
            hresult: 0x8899_AABBu32 as i32,
            context: "CreateSwapChain",
        };
        let text = err.to_string();
        assert!(text.contains("0x8899aabb"), "unexpected: {text}");
        assert!(text.contains("CreateSwapChain"));
    }
}
