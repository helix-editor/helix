#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchKind {
    Local,
    Remote,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branch {
    name: String,
    kind: BranchKind,
    is_current: bool,
    head: String,
    upstream: Option<String>,
    tracking: Option<String>,
}

impl Branch {
    #[cfg(feature = "git")]
    pub(crate) fn new(
        name: String,
        kind: BranchKind,
        is_current: bool,
        head: String,
        upstream: Option<String>,
        tracking: Option<String>,
    ) -> Self {
        Self {
            name,
            kind,
            is_current,
            head,
            upstream,
            tracking,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn kind(&self) -> BranchKind {
        self.kind
    }

    pub fn is_current(&self) -> bool {
        self.is_current
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    pub fn upstream(&self) -> Option<&str> {
        self.upstream.as_deref()
    }

    pub fn tracking(&self) -> Option<&str> {
        self.tracking.as_deref()
    }
}
