use std::fmt::Display;

#[derive(Debug, Copy, Clone)]
pub enum FinalStateVia {
    AzureAsyncOp,
    Location,
    OriginalUri,
    OperationLocation,
}

impl Display for FinalStateVia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FinalStateVia::AzureAsyncOp => "azure-async-operation",
            FinalStateVia::Location => "location",
            FinalStateVia::OriginalUri => "original-uri",
            FinalStateVia::OperationLocation => "operation-location",
        };
        f.write_str(s)
    }
}
