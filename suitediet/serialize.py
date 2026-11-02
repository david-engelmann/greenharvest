"""The canonical written form shared by every suitediet document."""

from __future__ import annotations

import json
from typing import Any


def canonical_text(document: Any) -> str:
    """Render a document with sorted keys and two space indentation.

    One trailing newline ends the text, and nothing in the rendering
    carries a wall clock time or a local filesystem path.
    """
    return json.dumps(document, indent=2, sort_keys=True) + "\n"
