#[macro_export]
macro_rules! current {
    ( $( $editor:ident ).+ ) => {{
        let view = $crate::view_mut!( $( $editor ).+ );
        let doc = &mut $( $editor ).+ .documents[view.doc];
        (view, doc)
    }};
}

#[macro_export]
macro_rules! doc_mut {
    ( $( $editor:ident ).+ ) => {{
        $crate::current!( $( $editor ).+ ).1
    }};
}

#[macro_export]
macro_rules! view_mut {
    ( $( $editor:ident ).+ ) => {{
        $( $editor ).+ .tree.get_mut($( $editor ).+ .tree.focus)
    }};
}

#[macro_export]
macro_rules! view {
    ( $( $editor:ident ).+ ) => {{
        $( $editor ).+ .tree.get($( $editor ).+ .tree.focus)
    }};
}
