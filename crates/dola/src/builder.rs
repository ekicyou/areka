//! DolaDocument / Storyboard をコードから構築するビルダー API

use crate::document::DolaDocument;
use crate::error::DolaError;
use crate::storyboard::{InterruptionPolicy, LoopOffset, Storyboard, StoryboardEntry};
use crate::transition::TransitionDef;
use crate::validate::Validate;
use crate::variable::AnimationVariableDef;
use std::collections::BTreeMap;

/// DolaDocument ビルダー
pub struct DolaDocumentBuilder {
    schema_version: String,
    variable: BTreeMap<String, AnimationVariableDef>,
    transition: BTreeMap<String, TransitionDef>,
    storyboard: BTreeMap<String, Storyboard>,
}

impl DolaDocumentBuilder {
    /// 新しいビルダーを作成
    pub fn new(schema_version: impl Into<String>) -> Self {
        Self {
            schema_version: schema_version.into(),
            variable: BTreeMap::new(),
            transition: BTreeMap::new(),
            storyboard: BTreeMap::new(),
        }
    }

    /// アニメーション変数を追加
    pub fn variable(mut self, name: impl Into<String>, def: AnimationVariableDef) -> Self {
        self.variable.insert(name.into(), def);
        self
    }

    /// トランジションテンプレートを追加
    pub fn transition(mut self, name: impl Into<String>, def: TransitionDef) -> Self {
        self.transition.insert(name.into(), def);
        self
    }

    /// ストーリーボードを追加
    pub fn storyboard(mut self, name: impl Into<String>, sb: Storyboard) -> Self {
        self.storyboard.insert(name.into(), sb);
        self
    }

    /// ドキュメントを構築し、自動的にバリデーションを実行
    pub fn build(self) -> Result<DolaDocument, Vec<DolaError>> {
        let doc = DolaDocument {
            schema_version: self.schema_version,
            variable: self.variable,
            transition: self.transition,
            storyboard: self.storyboard,
        };
        doc.validate()?;
        Ok(doc)
    }
}

/// Storyboard ビルダー
pub struct StoryboardBuilder {
    time_scale: f64,
    loop_count: i32,
    interruption_policy: InterruptionPolicy,
    loop_offset: Option<LoopOffset>,
    entry: Vec<StoryboardEntry>,
}

impl StoryboardBuilder {
    /// 新しいビルダーを作成
    pub fn new() -> Self {
        Self {
            time_scale: 1.0,
            loop_count: 1,
            interruption_policy: InterruptionPolicy::Conclude,
            loop_offset: None,
            entry: Vec::new(),
        }
    }

    /// 再生速度倍率を設定
    pub fn time_scale(mut self, scale: f64) -> Self {
        self.time_scale = scale;
        self
    }

    /// ループ回数を設定（1 = 1回、n≥2 = n回、-1 = 無限ループ）
    pub fn loop_count(mut self, count: i32) -> Self {
        self.loop_count = count;
        self
    }

    /// 割り込み終了戦略を設定
    pub fn interruption_policy(mut self, policy: InterruptionPolicy) -> Self {
        self.interruption_policy = policy;
        self
    }

    /// ループオフセットを設定
    pub fn loop_offset(mut self, offset: LoopOffset) -> Self {
        self.loop_offset = Some(offset);
        self
    }

    /// エントリを追加
    pub fn entry(mut self, entry: StoryboardEntry) -> Self {
        self.entry.push(entry);
        self
    }

    /// ストーリーボードを構築
    ///
    /// NOTE(D2-V): StoryboardBuilder はバリデーションを行わない（任意の Storyboard を
    /// 構築できる）が、compile_storyboard は冒頭で必ず doc.validate() を実行するため、
    /// 本ビルダー経由でもバリデーションを迂回して不正文書をコンパイルさせることは
    /// できない（tests/compile/boundary_test.rs で特性化済み）。
    pub fn build(self) -> Storyboard {
        Storyboard {
            time_scale: self.time_scale,
            loop_count: self.loop_count,
            interruption_policy: self.interruption_policy,
            loop_offset: self.loop_offset,
            entry: self.entry,
        }
    }
}

impl Default for StoryboardBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storyboard::LoopOffset;

    // D2-T gap tests: tests/general/builder_test.rs が未カバーの Builder API 空白
    // （loop_offset 設定・Default 実装・同名上書き・スキーマ検証連動）

    #[test]
    fn storyboard_builder_loop_offset_set() {
        let sb = StoryboardBuilder::new()
            .loop_offset(LoopOffset::Scalar(1.5))
            .build();
        assert_eq!(sb.loop_offset, Some(LoopOffset::Scalar(1.5)));
    }

    #[test]
    fn storyboard_builder_loop_offset_default_none() {
        assert_eq!(StoryboardBuilder::new().build().loop_offset, None);
    }

    #[test]
    fn storyboard_builder_default_equals_new() {
        assert_eq!(
            StoryboardBuilder::default().build(),
            StoryboardBuilder::new().build()
        );
    }

    #[test]
    fn document_builder_duplicate_name_last_wins() {
        // BTreeMap::insert に基づく上書き挙動の固定（同名 variable は後勝ち）
        let doc = DolaDocumentBuilder::new("1.0")
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 1.0,
                    min: None,
                    max: None,
                },
            )
            .variable(
                "x",
                AnimationVariableDef::Float {
                    initial: 2.0,
                    min: None,
                    max: None,
                },
            )
            .build()
            .unwrap();
        assert_eq!(doc.variable.len(), 1);
        assert!(matches!(
            doc.variable.get("x"),
            Some(AnimationVariableDef::Float { initial, .. }) if *initial == 2.0
        ));
    }

    #[test]
    fn document_builder_schema_mismatch_fails_validation() {
        // build() はバリデーションを内包する: 不正スキーマバージョンで Err
        let errs = DolaDocumentBuilder::new("0.5").build().unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, DolaError::SchemaVersionMismatch { .. }))
        );
    }
}
