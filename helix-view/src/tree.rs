use crate::View;
use slotmap::{DefaultKey as Key, HopSlotMap};
use tui::layout::Rect;

// the dimensions are recomputed on windo resize/tree change.
//
pub struct Tree {
    root: Key,
    // (container, index inside the container)
    pub focus: Key,
    // fullscreen: bool,
    area: Rect,

    nodes: HopSlotMap<Key, Node>,

    // used for traversals
    stack: Vec<(Key, Rect)>,
}

pub struct Node {
    parent: Key,
    content: Content,
}

pub enum Content {
    View(Box<View>),
    Container(Box<Container>),
}

impl Node {
    pub fn container() -> Self {
        Node {
            parent: Key::default(),
            content: Content::Container(Box::new(Container::new())),
        }
    }

    pub fn view(view: View) -> Self {
        Node {
            parent: Key::default(),
            content: Content::View(Box::new(view)),
        }
    }
}

// TODO: screen coord to container + container coordinate helpers

pub enum Layout {
    Horizontal,
    Vertical,
    // could explore stacked/tabbed
}

pub struct Container {
    layout: Layout,
    children: Vec<Key>,
    area: Rect,
}

impl Container {
    pub fn new() -> Self {
        Self {
            layout: Layout::Horizontal,
            children: Vec::new(),
            area: Rect::default(),
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    pub fn new(area: Rect) -> Self {
        let root = Node::container();

        let mut nodes = HopSlotMap::new();
        let root = nodes.insert(root);

        // root is it's own parent
        nodes[root].parent = root;

        Self {
            root,
            focus: root,
            // fullscreen: false,
            area,
            nodes,
            stack: Vec::new(),
        }
    }

    pub fn insert(&mut self, view: View) -> Key {
        let focus = self.focus;
        let parent = self.nodes[focus].parent;
        let mut node = Node::view(view);
        node.parent = parent;
        let node = self.nodes.insert(node);
        self.get_mut(node).id = node;

        let container = match &mut self.nodes[parent] {
            Node {
                content: Content::Container(container),
                ..
            } => container,
            _ => unreachable!(),
        };

        // insert node after the current item if there is children already
        let pos = if container.children.is_empty() {
            0
        } else {
            let pos = container
                .children
                .iter()
                .position(|&child| child == focus)
                .unwrap();
            pos + 1
        };

        container.children.insert(pos, node);
        // focus the new node
        self.focus = node;

        // recalculate all the sizes
        self.recalculate();

        node
    }

    pub fn remove(&mut self, index: Key) {
        let mut stack = Vec::new();

        if self.focus == index {
            // focus on something else
            self.focus_next();
        }

        stack.push(index);

        while let Some(index) = stack.pop() {
            let parent_id = self.nodes[index].parent;
            if let Node {
                content: Content::Container(container),
                ..
            } = &mut self.nodes[parent_id]
            {
                if let Some(pos) = container.children.iter().position(|&child| child == index) {
                    container.children.remove(pos);

                    // TODO: if container now only has one child, remove it and place child in parent
                    if container.children.is_empty() && parent_id != self.root {
                        // if container now empty, remove it
                        stack.push(parent_id);
                    }
                }
            }
            self.nodes.remove(index);
        }

        self.recalculate()
    }

    pub fn views(&mut self) -> impl Iterator<Item = (&mut View, bool)> {
        let focus = self.focus;
        self.nodes
            .iter_mut()
            .filter_map(move |(key, node)| match node {
                Node {
                    content: Content::View(view),
                    ..
                } => Some((view.as_mut(), focus == key)),
                _ => None,
            })
    }

    pub fn get(&self, index: Key) -> &View {
        match &self.nodes[index] {
            Node {
                content: Content::View(view),
                ..
            } => view,
            _ => unreachable!(),
        }
    }

    pub fn get_mut(&mut self, index: Key) -> &mut View {
        match &mut self.nodes[index] {
            Node {
                content: Content::View(view),
                ..
            } => view,
            _ => unreachable!(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.nodes[self.root] {
            Node {
                content: Content::Container(container),
                ..
            } => container.children.is_empty(),
            _ => unreachable!(),
        }
    }

    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        self.recalculate();
    }

    pub fn recalculate(&mut self) {
        if self.is_empty() {
            return;
        }

        self.stack.push((self.root, self.area));

        // take the area
        // fetch the node
        // a) node is view, give it whole area
        // b) node is container, calculate areas for each child and push them on the stack

        while let Some((key, area)) = self.stack.pop() {
            let node = &mut self.nodes[key];

            match &mut node.content {
                Content::View(view) => {
                    // debug!!("setting view area {:?}", area);
                    view.area = area;
                } // TODO: call f()
                Content::Container(container) => {
                    // debug!!("setting container area {:?}", area);
                    container.area = area;

                    match container.layout {
                        Layout::Vertical => {
                            let len = container.children.len();

                            let height = area.height / len as u16;

                            let mut child_y = area.y;

                            for (i, child) in container.children.iter().enumerate() {
                                let mut area = Rect::new(
                                    container.area.x,
                                    child_y,
                                    container.area.width,
                                    height,
                                );
                                child_y += height;

                                // last child takes the remaining width because we can get uneven
                                // space from rounding
                                if i == len - 1 {
                                    area.height = container.area.y + container.area.height - area.y;
                                }

                                self.stack.push((*child, area));
                            }
                        }
                        Layout::Horizontal => {
                            let len = container.children.len();

                            let width = area.width / len as u16;

                            let mut child_x = area.x;

                            for (i, child) in container.children.iter().enumerate() {
                                let mut area = Rect::new(
                                    child_x,
                                    container.area.y,
                                    width,
                                    container.area.height,
                                );
                                child_x += width;

                                // last child takes the remaining width because we can get uneven
                                // space from rounding
                                if i == len - 1 {
                                    area.width = container.area.x + container.area.width - area.x;
                                }

                                self.stack.push((*child, area));
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn traverse(&self) -> Traverse {
        Traverse::new(self)
    }

    pub fn focus_next(&mut self) {
        // This function is very dumb, but that's because we don't store any parent links.
        // (we'd be able to go parent.next_sibling() recursively until we find something)
        // For now that's okay though, since it's unlikely you'll be able to open a large enough
        // number of splits to notice.

        // current = focus
        // let found = loop do {
        //   node = focus.parent;
        //   let found = node.next_sibling_of(current)
        //   if some {
        //       break found;
        //   }
        //   // else
        //   if node == root {
        //       return first child of root;
        //   };
        //   current = parent;
        //  }
        // }
        //
        // use found next sibling
        // loop do {
        //   if found = view -> focus = found, return
        //   if found = container -> found = first child
        // }

        let iter = self.traverse();

        let mut iter = iter.skip_while(|&(key, _view)| key != self.focus);
        iter.next(); // take the focused value

        match iter.next() {
            Some((key, _)) => {
                self.focus = key;
            }
            None => {
                // extremely crude, take the first item again
                let (key, _) = self.traverse().next().unwrap();
                self.focus = key;
            }
        }
    }
}

pub struct Traverse<'a> {
    tree: &'a Tree,
    stack: Vec<Key>, // TODO: reuse the one we use on update
}

impl<'a> Traverse<'a> {
    fn new(tree: &'a Tree) -> Self {
        Self {
            tree,
            stack: vec![tree.root],
        }
    }
}

impl<'a> Iterator for Traverse<'a> {
    type Item = (Key, &'a View);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let key = self.stack.pop()?;

            let node = &self.tree.nodes[key];

            match &node.content {
                Content::View(view) => return Some((key, view)),
                Content::Container(container) => {
                    self.stack.extend(container.children.iter().rev());
                }
            }
        }
    }
}
