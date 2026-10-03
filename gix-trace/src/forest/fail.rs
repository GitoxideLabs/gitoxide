pub(crate) const SPAN_NOT_IN_CONTEXT: &str = "the registry retains a span during its layer callbacks";
pub(crate) const OPENED_SPAN_NOT_IN_EXTENSIONS: &str =
    "the forest layer stores an OpenedSpan when the registry creates a span";
pub(crate) const PROCESSING_ERROR: &str = "processing a completed trace tree failed";
