//! The canonical written form shared by every greenharvest
//! document.

use crate::jsonvalue::JsonValue;
use crate::render::canonical_text as render_document;

/// Render a document with sorted keys and two space indentation.
///
/// One trailing newline ends the text, and nothing in the
/// rendering carries a wall clock time or a local filesystem
/// path.
pub fn canonical_text(document: &JsonValue) -> String {
    render_document(document)
}
