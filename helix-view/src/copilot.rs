use helix_lsp::{copilot_types::DocCompletion, OffsetEncoding};
use parking_lot::Mutex;
use std::sync::Arc;

static GLOBAL_AUTO_RENDER: once_cell::sync::OnceCell<Arc<Mutex<bool>>> = once_cell::sync::OnceCell::new();

#[derive(Clone, Debug)]
pub struct Copilot {
    completion_response: Option<(Vec<DocCompletion>, OffsetEncoding)> ,
    render: Render
}

#[derive(Clone, Debug)]
struct Render {
    global_auto_render: Arc<Mutex<bool>>,
    should_render: Option<usize>,
}

impl Render {
    pub fn reset(& mut self) {
        let lock = self.global_auto_render.lock();
        self.should_render = if *lock {Some(0)} else {None};
    }
    pub fn should_not_render(&mut self) {
        self.should_render = None;
    }
 }

impl Copilot {
    pub fn new(editor_auto_render: bool) -> Copilot {
        let global_auto_render_completion = GLOBAL_AUTO_RENDER.get_or_init(|| Arc::new(Mutex::new(editor_auto_render))).clone();

        return Self {
            completion_response: None,
            render: Render { 
                global_auto_render: global_auto_render_completion,
                should_render: None
            }
        }
    }

    pub fn delete_state_and_reset_should_render(&mut self) {
        self.render.reset();
        self.completion_response = None;
    }

    pub fn delete_state_and_should_not_render(&mut self) {
        self.render.should_not_render();
        self.completion_response = None;
    }

    pub fn show_completion(&mut self) {
        self.render.should_render = Some(0);
    }

    pub fn fill_with_completions(&mut self, completions: Vec<DocCompletion>, offset_encoding: OffsetEncoding) {
       self.completion_response = Some((completions, offset_encoding));
    }

    pub fn get_completion_if_should_render(&self) -> Option<&DocCompletion> {
        let idx = self.render.should_render?;
        let completions = &self.completion_response.as_ref()?.0;
        completions.get(idx)
    }

    pub fn offset_encoding(&self) -> Option<OffsetEncoding> {
        Some(self.completion_response.as_ref()?.1)
    }

    pub fn toggle_auto_render(&self) -> bool {
       let mut lock = self.render.global_auto_render.lock(); 
       *lock = !(*lock);
       return *lock;
    }

}
