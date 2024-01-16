Semantic tokens are requested asynchronously in the background and will also try to update the document asynchronously. A document may be open in multiple views at once with arbitrary ranges. It's also possible that semantic highlighting from the LSP is invalid. When possible, semantic hihglighting should replace tree-sitter highlighting, but mapping is expensive. Having locals would not be a complete replacement for full semantic highlighting, and having it continuously change may be disorientating/jarring.

- The Handler will function on all views and update them consequently based on a predefined timeout. The corresponding LSP for each view's doc will be called for semantic tokens, and will be putted to another function to do it as a job.

