//! Unit tests for core data types: DolaDocument, AnimationVariableDef, DynamicValue, DolaError
//! Tasks 2.5, 3.4, 4.4, 5.7, 6.3
//!
//! Split into cohesion-based sub-files (research §R3 seam); test leaf names preserved.

use dola::*;
use std::collections::BTreeMap;

mod document_variable;
mod dynamic_value;
mod easing_transition;
mod storyboard_playback;
