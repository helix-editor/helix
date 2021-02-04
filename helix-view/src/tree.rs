use crate::View;
use slotmap::{DefaultKey as Key, HopSlotMap};
use tui::layout::Rect;

// the dimensions are recomputed on windo resize/tree change.
//
pub struct Tree {
    root: Key,
    // (container, index inside the container)
    current: (Key, usize),
    pub focus: Key,
    fullscreen: bool,
    area: Rect,

    nodes: HopSlotMap<Key, Node>,

    // used for traversals
    stack: Vec<(Key, Rect)>,
}

pub enum Node {
    View(Box<View>),
    Container(Box<Container>),
}

impl Node {
    pub fn container(area: Rect) -> Self {
        Self::Container(Box::new(Container::new()))
    }

    pub fn view(view: View) -> Self {
        Self::View(Box::new(view))
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

impl Tree {
    pub fn new(area: Rect) -> Self {
        let root = Node::container(area);
        let mut nodes = HopSlotMap::new();
        let root = nodes.insert(root);

        Self {
            root,
            current: (root, 0),
            focus: Key::default(),
            fullscreen: false,
            area,
            nodes,
            stack: Vec::new(),
        }
    }

    pub fn insert(&mut self, view: View) -> Key {
        let node = self.nodes.insert(Node::view(view));
        let (id, pos) = self.current;
        let container = match &mut self.nodes[id] {
            Node::Container(container) => container,
            _ => unreachable!(),
        };

        // insert node after the current item if there is children already
        let pos = if container.children.is_empty() {
            pos
        } else {
            pos + 1
        };

        container.children.insert(pos, node);
        // focus the new node
        self.current = (id, pos);
        self.focus = node;

        // recalculate all the sizes
        self.traverse();

        node
    }

    pub fn views(&mut self) -> impl Iterator<Item = (&mut View, bool)> {
        let focus = self.focus;
        self.nodes
            .iter_mut()
            .filter_map(move |(key, node)| match node {
                Node::View(view) => Some((view.as_mut(), focus == key)),
                Node::Container(..) => None,
            })
    }

    pub fn get(&self, index: Key) -> &View {
        match &self.nodes[index] {
            Node::View(view) => view,
            _ => unreachable!(),
        }
    }

    pub fn get_mut(&mut self, index: Key) -> &mut View {
        match &mut self.nodes[index] {
            Node::View(view) => view,
            _ => unreachable!(),
        }
    }

    pub fn resize(&mut self, area: Rect) {
        self.area = area;
        self.traverse();
    }

    pub fn traverse(&mut self) {
        self.stack.push((self.root, self.area));

        // take the area
        // fetch the node
        // a) node is view, give it whole area
        // b) node is container, calculate areas for each child and push them on the stack

        while let Some((key, area)) = self.stack.pop() {
            let node = &mut self.nodes[key];

            match node {
                Node::View(view) => {
                    // debug!!("setting view area {:?}", area);
                    view.area = area;
                } // TODO: call f()
                Node::Container(container) => {
                    // debug!!("setting container area {:?}", area);
                    container.area = area;

                    match container.layout {
                        Layout::Vertical => unimplemented!(),
                        Layout::Horizontal => {
                            let len = container.children.len() as u16;

                            let width = area.width / len;

                            let mut child_x = area.x;

                            for (_i, child) in container.children.iter().enumerate() {
                                let area = Rect::new(child_x, area.y, width, area.height);
                                child_x += width;

                                self.stack.push((*child, area));
                            }
                        }
                    }
                }
            }
        }
    }
}
